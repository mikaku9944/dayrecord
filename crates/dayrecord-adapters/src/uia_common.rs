//! Platform-independent helpers for accessibility-tree text capture.
//!
//! Shared between the Windows UIA adapter (`uia.rs`) and the macOS AX adapter
//! (`context_ax.rs`). Constants, snapshot composition, URL heuristics, and
//! text deduplication all live here so both platforms produce identical output
//! formats.

/// Final output character cap after composition.
pub const MAX_UIA_CHARS: usize = 4000;
/// Per-window collected text budget before we stop walking the tree.
pub const MAX_WINDOW_TEXT_CHARS: usize = 3500;
/// Hard cap on tree nodes visited per sample (protects the 2s budget).
pub const MAX_NODES: usize = 800;
/// Avoid pathological sibling lists (e.g. huge virtualized grids).
pub const MAX_CHILDREN_PER_NODE: usize = 200;
/// Wall-clock budget for a single accessibility-tree read.
pub const UIA_POLL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Heuristic: does this edit-box value look like a browser address / URL?
pub fn looks_like_url(value: &str) -> bool {
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

/// Character-aware truncation (safe for CJK text).
pub fn truncate_text(text: String) -> String {
    if text.chars().count() <= MAX_UIA_CHARS {
        return text;
    }
    let truncated: String = text.chars().take(MAX_UIA_CHARS.saturating_sub(1)).collect();
    format!("{truncated}…")
}

/// Combine focus text, surrounding window text, and URL into one labeled snapshot.
/// Drops the focus block when it's already contained in the window text (dedup).
pub fn compose_snapshot(
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

/// Push a text fragment into the collection buffer with deduplication and budget tracking.
/// Filters out browser chrome noise and substring duplicates.
pub fn push_unique_text(texts: &mut Vec<String>, total_chars: &mut usize, raw: String) {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_browser_noise(trimmed) {
        return;
    }
    let clipped: String = trimmed.chars().take(MAX_WINDOW_TEXT_CHARS).collect();
    if texts
        .iter()
        .any(|t| t == &clipped || t.contains(&clipped) || clipped.contains(t))
    {
        return;
    }
    *total_chars += clipped.chars().count();
    texts.push(clipped);
}

/// Browser toolbar / chrome labels that are not useful user context.
/// Covers Chrome, Edge, Safari, and generic window controls.
pub fn is_browser_noise(s: &str) -> bool {
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
            // Safari-specific
            | "Favorites"
            | "Bookmarks"
            | "Tab Overview"
            | "Share"
            | "Reader"
            | "Safari"
    )
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

    #[test]
    fn browser_noise_covers_safari() {
        assert!(is_browser_noise("Safari"));
        assert!(is_browser_noise("Favorites"));
        assert!(is_browser_noise("Back"));
        assert!(!is_browser_noise("Hello World"));
    }
}
