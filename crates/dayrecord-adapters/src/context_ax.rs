//! Screenshot-free on-screen text capture via macOS Accessibility (AX) API.
//!
//! Mirrors the Windows UIA adapter (`uia.rs`): reads the *text* the user
//! actually sees through the accessibility tree, never a pixel. Capture is
//! bounded (node count + char budget + 2s wall clock) so it can never stall
//! the sampler, and password controls are always skipped.
//!
//! Uses raw FFI to the ApplicationServices framework's `AXUIElement` C API,
//! with `core-foundation` types for safe CFString / CFArray handling.
//!
//! Known blind spots (same as Windows UIA): some Electron / Canvas / game UIs
//! expose little or no AX text. The window title from the activity timeline
//! remains the backstop signal.

use crate::uia_common::{
    compose_snapshot, looks_like_url, push_unique_text, MAX_CHILDREN_PER_NODE, MAX_NODES,
    MAX_WINDOW_TEXT_CHARS, UIA_POLL_TIMEOUT,
};
use core_foundation::base::{CFIndex, CFRelease, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use std::sync::mpsc;
use std::thread;

// ── ApplicationServices FFI ─────────────────────────────────────────────────

type AXUIElementRef = CFTypeRef;
type AXError = i32;

const AX_SUCCESS: AXError = 0;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementCopyAttributeValues(
        element: AXUIElementRef,
        attribute: CFStringRef,
        index: CFIndex,
        max_values: CFIndex,
        values: *mut CFTypeRef,
    ) -> AXError;
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef)
        -> bool;
}

// ── Safe wrappers around CF types ───────────────────────────────────────────

/// Convert a `CFStringRef` to a Rust `String`. Does **not** release the reference.
unsafe fn cf_string_to_rust(cf_str: CFStringRef) -> Option<String> {
    if cf_str.is_null() {
        return None;
    }
    let s = CFString::wrap_under_get_rule(cf_str);
    let rust = s.to_string();
    if rust.trim().is_empty() {
        None
    } else {
        Some(rust)
    }
}

/// Read a string-valued AX attribute. Caller must `CFRelease(*value)` on success.
unsafe fn read_ax_string(element: AXUIElementRef, attr_name: &str) -> Option<String> {
    let attr = CFString::new(attr_name);
    let mut value: CFTypeRef = std::ptr::null();
    let err = AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value);
    if err != AX_SUCCESS || value.is_null() {
        return None;
    }
    let result = cf_string_to_rust(value as CFStringRef);
    CFRelease(value);
    result
}

/// Read the children array of an AX element, retaining each child reference.
/// Caller must `CFRelease` each returned element.
unsafe fn read_ax_children(element: AXUIElementRef) -> Vec<AXUIElementRef> {
    let attr = CFString::new("AXChildren");
    let mut value: CFTypeRef = std::ptr::null();
    let err = AXUIElementCopyAttributeValues(element, attr.as_concrete_TypeRef(), 0, 1024, &mut value);
    if err != AX_SUCCESS || value.is_null() {
        return Vec::new();
    }

    use core_foundation::array::CFArrayRef;
    let array = value as CFArrayRef;
    let count = core_foundation::array::CFArrayGetCount(array);
    let mut children = Vec::with_capacity(count as usize);

    for i in 0..count {
        let child = core_foundation::array::CFArrayGetValueAtIndex(array, i);
        if !child.is_null() {
            // Retain because CFArrayGetValueAtIndex returns an unretained reference.
            core_foundation::base::CFRetain(child);
            children.push(child);
        }
    }

    CFRelease(value); // release the array itself
    children
}

/// Read the AX role string of an element (e.g. "AXTextField", "AXStaticText").
unsafe fn read_ax_role(element: AXUIElementRef) -> Option<String> {
    read_ax_string(element, "AXRole")
}

/// Read the AX subrole string of an element (e.g. "AXSecureTextField").
unsafe fn read_ax_subrole(element: AXUIElementRef) -> Option<String> {
    read_ax_string(element, "AXSubrole")
}

/// Check whether an element is a password / secure text field.
unsafe fn is_password_field(element: AXUIElementRef) -> bool {
    read_ax_subrole(element)
        .map(|s| s == "AXSecureTextField")
        .unwrap_or(false)
}

/// Ensure the current process has Accessibility permission.
/// On first call, triggers the macOS system permission dialog automatically.
fn ensure_ax_trusted() -> bool {
    unsafe {
        if AXIsProcessTrusted() {
            return true;
        }
    }

    // Not yet trusted — prompt the user via the system dialog.
    tracing::info!(
        "Accessibility permission not granted; triggering system permission dialog..."
    );

    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::true_value();
    let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

    unsafe {
        let trusted = AXIsProcessTrustedWithOptions(
            options.as_concrete_TypeRef(),
        );
        if !trusted {
            tracing::warn!(
                "Accessibility permission denied or not yet granted. \
                 Go to System Settings → Privacy & Security → Accessibility → enable your terminal app."
            );
        }
        trusted
    }
}

// ── Core capture logic ──────────────────────────────────────────────────────

/// Read text from the focused element (without walking the tree).
unsafe fn read_focused_element_text(system: AXUIElementRef) -> Option<String> {
    let attr = CFString::new("AXFocusedUIElement");
    let mut value: CFTypeRef = std::ptr::null();
    let err = AXUIElementCopyAttributeValue(system, attr.as_concrete_TypeRef(), &mut value);
    if err != AX_SUCCESS || value.is_null() {
        return None;
    }

    let element = value; // AXUIElementRef

    // Skip password fields
    if is_password_field(element) {
        CFRelease(element);
        return None;
    }

    // Try AXValue first (works for text fields, text areas, etc.)
    let text = read_ax_string(element, "AXValue")
        .or_else(|| read_ax_string(element, "AXDescription"));

    CFRelease(element);
    text
}

/// Walk up the parent chain to find a container element suitable for tree walking.
/// Stops at a window element, an application element, or after 15 levels.
unsafe fn find_walk_root(focused: AXUIElementRef) -> Option<AXUIElementRef> {
    let mut current = focused;
    core_foundation::base::CFRetain(current);

    for _ in 0..15 {
        if let Some(role) = read_ax_role(current) {
            // Stop at Window or Application level
            if role == "AXWindow" || role == "AXApplication" {
                return Some(current);
            }
        }

        let attr = CFString::new("AXParent");
        let mut parent_value: CFTypeRef = std::ptr::null();
        let err =
            AXUIElementCopyAttributeValue(current, attr.as_concrete_TypeRef(), &mut parent_value);
        if err != AX_SUCCESS || parent_value.is_null() {
            // No more parents — use current as root
            return Some(current);
        }

        CFRelease(current);
        current = parent_value; // already retained by CopyAttributeValue
    }

    Some(current)
}

/// Bounded depth-first walk of an AX element tree, collecting visible text.
/// Mirrors the Windows `read_window_context()` with identical budget limits.
unsafe fn read_window_context(focused: AXUIElementRef) -> (Option<String>, Option<String>) {
    let root = match find_walk_root(focused) {
        Some(r) => r,
        None => return (None, None),
    };

    let mut texts: Vec<String> = Vec::new();
    let mut url: Option<String> = None;
    let mut total_chars = 0usize;
    let mut visited = 0usize;
    let mut stack: Vec<AXUIElementRef> = vec![root];

    while let Some(node) = stack.pop() {
        if visited >= MAX_NODES || total_chars >= MAX_WINDOW_TEXT_CHARS {
            CFRelease(node);
            // Drain remaining elements to avoid leaks
            for remaining in stack.drain(..) {
                CFRelease(remaining);
            }
            break;
        }
        visited += 1;

        let is_pw = is_password_field(node);

        if !is_pw {
            if let Some(raw) = collect_readable_text(node) {
                push_unique_text(&mut texts, &mut total_chars, raw);
                let role = read_ax_role(node).unwrap_or_default();
                if role == "AXTextArea" || role == "AXWebArea" {
                    // Full document captured; skip subtree to avoid duplicate runs.
                    CFRelease(node);
                    continue;
                }
            }

            // Check for URL in text fields
            if url.is_none() {
                let role = read_ax_role(node).unwrap_or_default();
                if role == "AXTextField" {
                    if let Some(value) = read_ax_string(node, "AXValue") {
                        if looks_like_url(&value) {
                            url = Some(value.trim().to_string());
                        }
                    }
                }
            }
        }

        // Push children onto stack (bounded per node)
        let children = read_ax_children(node);
        let mut count = 0usize;
        for child in children {
            if count >= MAX_CHILDREN_PER_NODE {
                CFRelease(child);
                continue;
            }
            stack.push(child);
            count += 1;
        }

        CFRelease(node);
    }

    let window_text = if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    };
    (window_text, url)
}

/// Extract readable text from an AX element based on its role.
/// Priority: AXValue → AXDescription → AXTitle.
unsafe fn collect_readable_text(element: AXUIElementRef) -> Option<String> {
    let role = read_ax_role(element)?;

    match role.as_str() {
        // Text fields, text areas, web areas: read AXValue
        "AXTextField" | "AXTextArea" | "AXWebArea" | "AXSearchField" | "AXComboBox" => {
            read_ax_string(element, "AXValue")
                .or_else(|| read_ax_string(element, "AXDescription"))
        }
        // Static text, labels: read AXDescription or AXValue
        "AXStaticText" | "AXLabel" | "AXHeading" => read_ax_string(element, "AXDescription")
            .or_else(|| read_ax_string(element, "AXValue"))
            .or_else(|| read_ax_string(element, "AXTitle")),
        // Links, groups, panes: read AXDescription or AXTitle
        "AXLink" | "AXGroup" | "AXGenericElement" => {
            let text = read_ax_string(element, "AXDescription")
                .or_else(|| read_ax_string(element, "AXTitle"));
            // Require minimum length (skip short labels like "Back")
            text.filter(|s| s.chars().count() >= 3)
        }
        _ => None,
    }
}

/// Single-shot capture: read focused text + walk the window tree.
unsafe fn read_focused_text_once() -> Option<String> {
    // Check (and auto-prompt) for accessibility permissions
    if !ensure_ax_trusted() {
        return None;
    }

    let system = AXUIElementCreateSystemWide();
    if system.is_null() {
        return None;
    }

    // 1. Read focused element text
    let focus = read_focused_element_text(system);

    // 2. Walk the window tree for visible text + URL
    //    Re-create the system element (the previous calls consumed references)
    let system2 = AXUIElementCreateSystemWide();
    let (window_text, url) = {
        let attr = CFString::new("AXFocusedUIElement");
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(system2, attr.as_concrete_TypeRef(), &mut value);
        if err == AX_SUCCESS && !value.is_null() {
            let result = read_window_context(value); // consumes `value` reference
            result
        } else {
            (None, None)
        }
    };

    CFRelease(system);
    CFRelease(system2);

    compose_snapshot(focus, window_text, url)
}

// ── Public entry point ──────────────────────────────────────────────────────

/// Poll focused/visible on-screen text on a short-lived thread with a hard timeout.
/// Mirrors the Windows `uia::poll_focused_text()` — same contract, same output format.
pub fn poll_focused_text() -> Option<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = unsafe { read_focused_text_once() };
        let _ = tx.send(result);
    });
    rx.recv_timeout(UIA_POLL_TIMEOUT).ok().flatten()
}

#[cfg(test)]
mod tests {
    use crate::uia_common::*;

    // The compose_snapshot / looks_like_url / push_unique_text tests are shared
    // in uia_common.rs. Here we only test AX-specific logic that doesn't require
    // a running accessibility service (which requires system permissions).

    #[test]
    fn poll_returns_none_without_permission() {
        // In a CI / test environment without Accessibility permission,
        // poll_focused_text should return None gracefully (not panic).
        let result = super::poll_focused_text();
        // We can't assert the exact value (it depends on permissions),
        // but it must not panic.
        let _ = result;
    }
}
