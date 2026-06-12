#![cfg(windows)]

use chrono::Utc;
use dayrecord_adapters::keyboard::ScriptedKeyboard;
use dayrecord_core::models::{KeyEvent, KeyEventKind};
use dayrecord_core::ports::KeyboardSource;

/// Local interactive desktop test — skipped in CI.
#[test]
#[ignore = "requires interactive Windows desktop; run: cargo test --test keyboard_sendinput -- --ignored"]
fn sendinput_style_scripted_keyboard() {
    let mut kb = ScriptedKeyboard::new(vec![
        KeyEvent {
            at: Utc::now(),
            kind: KeyEventKind::Char('h'),
        },
        KeyEvent {
            at: Utc::now(),
            kind: KeyEventKind::Char('i'),
        },
    ]);
    let events = kb.poll_events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0].kind, KeyEventKind::Char('h')));
}
