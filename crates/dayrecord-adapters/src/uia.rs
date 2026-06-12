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
//!
//! Platform-independent helpers (constants, snapshot composition, URL heuristics)
//! live in `uia_common.rs` and are shared with the macOS AX adapter.

use crate::uia_common::{
    looks_like_url, compose_snapshot, push_unique_text,
    MAX_CHILDREN_PER_NODE, MAX_NODES, MAX_WINDOW_TEXT_CHARS, UIA_POLL_TIMEOUT,
};

#[cfg(windows)]
mod platform {
    use super::{MAX_CHILDREN_PER_NODE, MAX_NODES, MAX_WINDOW_TEXT_CHARS};
    use crate::uia_common::{looks_like_url, push_unique_text};
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

            compose_snapshot(focus, window_text, url)
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
                        if looks_like_url(&value) {
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
    use crate::uia_common::*;

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
