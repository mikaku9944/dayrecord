use crate::models::KeyEventKind;

/// Windows IME composition keys (`VK_PROCESSKEY` / 0xE5) must be dropped.
pub fn should_drop_ime_key(kind: &KeyEventKind) -> bool {
    matches!(kind, KeyEventKind::ImeComposition)
}

pub fn apply_key_event(buffer: &mut String, kind: &KeyEventKind) -> Option<String> {
    if should_drop_ime_key(kind) {
        return None;
    }

    match kind {
        KeyEventKind::Char(c) => {
            buffer.push(*c);
            None
        }
        KeyEventKind::Space => {
            buffer.push(' ');
            None
        }
        KeyEventKind::Enter => {
            buffer.push('\n');
            None
        }
        KeyEventKind::Backspace => {
            buffer.pop();
            None
        }
        KeyEventKind::Tab => {
            buffer.push('\t');
            None
        }
        KeyEventKind::Paste | KeyEventKind::Copy => None,
        KeyEventKind::ImeComposition => None,
    }
}

pub fn append_paste(buffer: &mut String, text: &str, max_len: usize) -> String {
    let mut clipped = text.chars().take(max_len).collect::<String>();
    if text.chars().count() > max_len {
        clipped.push_str("…");
    }
    let marked = format!("[PASTE]{clipped}");
    buffer.push_str(&marked);
    marked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KeyEvent;
    use chrono::Utc;
    use rstest::rstest;

    #[rstest]
    #[case(KeyEventKind::ImeComposition, true)]
    #[case(KeyEventKind::Char('a'), false)]
    fn ime_drop(#[case] kind: KeyEventKind, #[case] drop_it: bool) {
        assert_eq!(should_drop_ime_key(&kind), drop_it);
    }

    #[test]
    fn paste_truncates_at_limit() {
        let mut buf = String::new();
        let long = "x".repeat(2500);
        append_paste(&mut buf, &long, 2000);
        assert!(buf.contains("[PASTE]"));
        assert!(buf.chars().count() <= 2000 + "[PASTE]".len() + 1);
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut buf = String::from("ab");
        apply_key_event(&mut buf, &KeyEventKind::Backspace);
        assert_eq!(buf, "a");
    }

    #[test]
    fn char_appends() {
        let mut buf = String::new();
        apply_key_event(&mut buf, &KeyEventKind::Char('h'));
        apply_key_event(&mut buf, &KeyEventKind::Char('i'));
        assert_eq!(buf, "hi");
    }

    #[test]
    fn ime_ignored() {
        let mut buf = String::from("a");
        let event = KeyEvent {
            at: Utc::now(),
            kind: KeyEventKind::ImeComposition,
        };
        assert!(should_drop_ime_key(&event.kind));
        assert!(apply_key_event(&mut buf, &event.kind).is_none());
        assert_eq!(buf, "a");
    }
}
