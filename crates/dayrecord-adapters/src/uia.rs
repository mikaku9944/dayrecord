//! Screenshot-free on-screen text capture via UI Automation.
//!
//! This is DayRecord's core differentiator vs. screenshot-based trackers: we read
//! the *text* the user actually sees (document bodies, web page content, chat
//! panels) through the accessibility tree, never a pixel. Capture is bounded
//! (node count + char budget + 2s wall clock) so it can never stall the sampler,
//! and password controls are always skipped.
//!
//! Known blind spots (documented honestly, not papered over): some Electron /
//! Chromium-embedded / Canvas / game UIs expose little or no UIA text. In those
//! cases we fall back to whatever focus/URL we can read; the window title from the
//! activity timeline remains the backstop signal.

const MAX_UIA_CHARS: usize = 4000;
/// Per-window collected text budget before we stop walking the tree.
const MAX_WINDOW_TEXT_CHARS: usize = 3500;
/// Hard cap on tree nodes visited per sample (protects the 2s budget).
const MAX_NODES: usize = 800;
/// Avoid pathological sibling lists (e.g. huge virtualized grids).
const MAX_CHILDREN_PER_NODE: usize = 200;
const UIA_POLL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[cfg(windows)]
mod platform {
    use super::{MAX_CHILDREN_PER_NODE, MAX_NODES, MAX_WINDOW_TEXT_CHARS};
    use windows::core::Interface;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
        IUIAutomationTreeWalker, IUIAutomationValuePattern, UIA_CONTROLTYPE_ID,
        UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_GroupControlTypeId,
        UIA_HyperlinkControlTypeId, UIA_PaneControlTypeId, UIA_TextControlTypeId, UIA_TextPatternId,
        UIA_ValuePatternId,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    pub fn read_focused_text_once() -> Option<String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        let automation: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()? };

        unsafe {
            let focus = automation.GetFocusedElement().ok().and_then(|el| {
                if el.CurrentIsPassword().map(|b| b.as_bool()).unwrap_or(false) {
                    return None;
                }
                read_text_pattern(&el).or_else(|| read_value_pattern(&el))
            });

            let (window_text, url) = read_window_context(&automation);

            super::compose_snapshot(focus, window_text, url)
        }
    }

    /// Bounded depth-first walk of the foreground window collecting readable text
    /// from Document/Text controls and a best-effort browser URL from the address bar.
    unsafe fn read_window_context(automation: &IUIAutomation) -> (Option<String>, Option<String>) {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return (None, None);
        }

        let window = match automation.ElementFromHandle(hwnd) {
            Ok(w) => w,
            Err(_) => return (None, None),
        };
        // RawView exposes more of Chromium's renderer tree than ControlView.
        let walker: IUIAutomationTreeWalker = match automation.RawViewWalker() {
            Ok(w) => w,
            Err(_) => match automation.ControlViewWalker() {
                Ok(w) => w,
                Err(_) => return (None, None),
            },
        };

        let mut texts: Vec<String> = Vec::new();
        let mut url: Option<String> = None;
        let mut total_chars = 0usize;
        let mut visited = 0usize;
        let mut stack: Vec<IUIAutomationElement> = vec![window];

        while let Some(node) = stack.pop() {
            if visited >= MAX_NODES || total_chars >= MAX_WINDOW_TEXT_CHARS {
                break;
            }
            visited += 1;

            let ct = node
                .CurrentControlType()
                .unwrap_or(UIA_CONTROLTYPE_ID(0));
            let is_password = node.CurrentIsPassword().map(|b| b.as_bool()).unwrap_or(false);

            if !is_password {
                if let Some(raw) = collect_readable_text(&node, ct) {
                    push_unique_text(&mut texts, &mut total_chars, raw);
                    if ct == UIA_DocumentControlTypeId || ct == UIA_TextControlTypeId {
                        // Full document captured; skip subtree to avoid duplicate runs.
                        continue;
                    }
                }

                if url.is_none() && ct == UIA_EditControlTypeId {
                    if let Some(value) = read_value_pattern(&node) {
                        if super::looks_like_url(&value) {
                            url = Some(value.trim().to_string());
                        }
                    }
                }
            }

            if let Ok(first) = walker.GetFirstChildElement(&node) {
                let mut child = Some(first);
                let mut count = 0usize;
                while let Some(c) = child {
                    if count >= MAX_CHILDREN_PER_NODE {
                        break;
                    }
                    let next = walker.GetNextSiblingElement(&c).ok();
                    stack.push(c);
                    count += 1;
                    child = next;
                }
            }
        }

        let window_text = if texts.is_empty() {
            None
        } else {
            Some(texts.join("\n"))
        };
        (window_text, url)
    }

    fn push_unique_text(texts: &mut Vec<String>, total_chars: &mut usize, raw: String) {
        let trimmed = raw.trim();
        if trimmed.is_empty() || is_chrome_noise(trimmed) {
            return;
        }
        let clipped: String = trimmed.chars().take(MAX_WINDOW_TEXT_CHARS).collect();
        if texts.iter().any(|t| t == &clipped || t.contains(&clipped) || clipped.contains(t)) {
            return;
        }
        *total_chars += clipped.chars().count();
        texts.push(clipped);
    }

    /// TextPattern first, then Name (common for Chromium headings/links), then Value on edits.
    unsafe fn collect_readable_text(
        element: &IUIAutomationElement,
        ct: UIA_CONTROLTYPE_ID,
    ) -> Option<String> {
        if let Some(text) = read_text_pattern(element) {
            return Some(text);
        }

        let name_types = [
            UIA_DocumentControlTypeId,
            UIA_TextControlTypeId,
            UIA_HyperlinkControlTypeId,
            UIA_GroupControlTypeId,
            UIA_PaneControlTypeId,
        ];
        if name_types.contains(&ct) {
            if let Ok(name) = element.CurrentName() {
                if let Some(s) = bstr_to_string(name) {
                    if s.chars().count() >= 3 {
                        return Some(s);
                    }
                }
            }
        }

        if ct == UIA_EditControlTypeId {
            return read_value_pattern(element);
        }

        None
    }

    fn is_chrome_noise(s: &str) -> bool {
        matches!(
            s,
            "Back"
                | "Forward"
                | "Reload"
                | "Home"
                | "Chrome"
                | "Microsoft Edge"
                | "Address and search bar"
                | "Search tabs"
                | "Minimize"
                | "Maximize"
                | "Close"
                | "Restore"
        )
    }

    unsafe fn read_text_pattern(element: &IUIAutomationElement) -> Option<String> {
        let pattern = element.GetCurrentPattern(UIA_TextPatternId).ok()?;
        let text_pattern: IUIAutomationTextPattern = pattern.cast().ok()?;
        let range = text_pattern.DocumentRange().ok()?;
        let bstr = range.GetText(-1).ok()?;
        bstr_to_string(bstr)
    }

    unsafe fn read_value_pattern(element: &IUIAutomationElement) -> Option<String> {
        let pattern = element.GetCurrentPattern(UIA_ValuePatternId).ok()?;
        let value_pattern: IUIAutomationValuePattern = pattern.cast().ok()?;
        let bstr = value_pattern.CurrentValue().ok()?;
        bstr_to_string(bstr)
    }

    fn bstr_to_string(bstr: windows::core::BSTR) -> Option<String> {
        let s = bstr.to_string();
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

/// Heuristic: does this edit-box value look like a browser address / URL?
fn looks_like_url(value: &str) -> bool {
    let t = value.trim();
    if t.is_empty() || t.len() > 1024 || t.chars().any(char::is_whitespace) {
        return false;
    }
    if t.starts_with("http://") || t.starts_with("https://") || t.contains("://") {
        return true;
    }
    let has_alpha = t.chars().any(|c| c.is_ascii_alphabetic());
    if !has_alpha || !t.contains('.') {
        return false;
    }
    if t.contains('/') {
        return true;
    }
    // bare domain like example.com — last label looks like a TLD
    t.rsplit('.')
        .next()
        .map(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()))
        .unwrap_or(false)
}

fn truncate_text(text: String) -> String {
    if text.chars().count() <= MAX_UIA_CHARS {
        return text;
    }
    let truncated: String = text.chars().take(MAX_UIA_CHARS.saturating_sub(1)).collect();
    format!("{truncated}…")
}

/// Combine focus text, surrounding window text, and URL into one labeled snapshot.
/// Drops the focus block when it's already contained in the window text (dedup).
fn compose_snapshot(
    focus: Option<String>,
    window_text: Option<String>,
    url: Option<String>,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    if let Some(u) = url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("[网址] {u}"));
    }

    let focus_t = focus.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let window_t = window_text.as_deref().map(str::trim).filter(|s| !s.is_empty());

    match (focus_t, window_t) {
        (Some(f), Some(w)) => {
            if !w.contains(f) {
                parts.push(format!("[焦点] {f}"));
            }
            parts.push(format!("[可见内容] {w}"));
        }
        (Some(f), None) => parts.push(format!("[焦点] {f}")),
        (None, Some(w)) => parts.push(format!("[可见内容] {w}")),
        (None, None) => {}
    }

    if parts.is_empty() {
        return None;
    }
    Some(truncate_text(parts.join("\n")))
}

/// Poll focused/visible on-screen text on a short-lived thread with a hard timeout
/// (avoids COM / WebView2 main-thread deadlocks blocking the activity sampler).
pub fn poll_focused_text() -> Option<String> {
    #[cfg(windows)]
    {
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = platform::read_focused_text_once();
            let _ = tx.send(result);
        });
        rx.recv_timeout(UIA_POLL_TIMEOUT).ok().flatten()
    }

    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_heuristic_accepts_common_forms() {
        assert!(looks_like_url("https://github.com/HKUDS/CatchMe"));
        assert!(looks_like_url("github.com/HKUDS/CatchMe"));
        assert!(looks_like_url("example.com"));
        assert!(looks_like_url("http://localhost:8765"));
    }

    #[test]
    fn url_heuristic_rejects_prose_and_numbers() {
        assert!(!looks_like_url("hello world"));
        assert!(!looks_like_url("3.14"));
        assert!(!looks_like_url(""));
        assert!(!looks_like_url("just some note with spaces.com here"));
    }

    #[test]
    fn compose_dedups_focus_inside_window() {
        let s = compose_snapshot(
            Some("查询订单状态".to_string()),
            Some("用户：查询订单状态\n客服：好的，请稍候".to_string()),
            None,
        )
        .unwrap();
        assert!(!s.contains("[焦点]"));
        assert!(s.contains("[可见内容]"));
    }

    #[test]
    fn compose_keeps_distinct_focus_and_url() {
        let s = compose_snapshot(
            Some("draft reply".to_string()),
            Some("inbox conversation body".to_string()),
            Some("mail.example.com/inbox".to_string()),
        )
        .unwrap();
        assert!(s.contains("[网址]"));
        assert!(s.contains("[焦点] draft reply"));
        assert!(s.contains("[可见内容]"));
    }

    #[test]
    fn compose_returns_none_when_empty() {
        assert!(compose_snapshot(None, None, None).is_none());
        assert!(compose_snapshot(Some("   ".to_string()), Some("".to_string()), None).is_none());
    }

    #[test]
    fn compose_truncates_to_cap() {
        let big = "字".repeat(MAX_UIA_CHARS + 500);
        let s = compose_snapshot(None, Some(big), None).unwrap();
        assert!(s.chars().count() <= MAX_UIA_CHARS);
    }
}
