use dayrecord_core::models::{KeyEvent, KeyEventKind};
use dayrecord_core::ports::KeyboardSource;
use std::collections::VecDeque;

/// Test double that replays scripted events.
pub struct ScriptedKeyboard {
    events: VecDeque<KeyEvent>,
}

impl ScriptedKeyboard {
    pub fn new(events: Vec<KeyEvent>) -> Self {
        Self {
            events: events.into(),
        }
    }
}

impl KeyboardSource for ScriptedKeyboard {
    fn poll_events(&mut self) -> Vec<KeyEvent> {
        let mut out = Vec::new();
        while let Some(e) = self.events.pop_front() {
            out.push(e);
        }
        out
    }
}

#[cfg(windows)]
pub mod win {
    use super::*;
    use std::sync::{Arc, Mutex};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_BACK, VK_RETURN, VK_SPACE, VK_TAB,
    };

    pub const VK_PROCESSKEY_U16: u16 = 0xE5;

    pub fn vk_to_kind(vk: u16, is_ime: bool) -> Option<KeyEventKind> {
        if is_ime || vk == VK_PROCESSKEY_U16 {
            return Some(KeyEventKind::ImeComposition);
        }
        match vk {
            x if x == VK_SPACE.0 as u16 => Some(KeyEventKind::Space),
            x if x == VK_RETURN.0 as u16 => Some(KeyEventKind::Enter),
            x if x == VK_BACK.0 as u16 => Some(KeyEventKind::Backspace),
            x if x == VK_TAB.0 as u16 => Some(KeyEventKind::Tab),
            0x43 if ctrl_down() => Some(KeyEventKind::Copy),
            0x56 if ctrl_down() => Some(KeyEventKind::Paste),
            0x41..=0x5A => Some(KeyEventKind::Char((vk as u8 + 32) as char)),
            0x30..=0x39 => Some(KeyEventKind::Char(vk as u8 as char)),
            _ => None,
        }
    }

    fn ctrl_down() -> bool {
        unsafe { GetAsyncKeyState(0x11) as u16 & 0x8000 != 0 }
    }

    pub struct WinKeyboardBuffer {
        pending: Arc<Mutex<Vec<KeyEvent>>>,
    }

    impl WinKeyboardBuffer {
        pub fn new() -> Self {
            Self {
                pending: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn push_vk(&self, vk: u16, at: chrono::DateTime<chrono::Utc>) {
            let kind = vk_to_kind(vk, vk == VK_PROCESSKEY_U16);
            if let Some(kind) = kind {
                if !matches!(kind, KeyEventKind::ImeComposition) || vk == VK_PROCESSKEY_U16 {
                    if vk != VK_PROCESSKEY_U16 {
                        self.pending.lock().unwrap().push(KeyEvent { at, kind });
                    }
                }
            }
        }
    }

    impl KeyboardSource for WinKeyboardBuffer {
        fn poll_events(&mut self) -> Vec<KeyEvent> {
            let mut guard = self.pending.lock().unwrap();
            std::mem::take(&mut *guard)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn scripted_replays_all() {
        let mut kb = ScriptedKeyboard::new(vec![KeyEvent {
            at: Utc::now(),
            kind: KeyEventKind::Char('a'),
        }]);
        assert_eq!(kb.poll_events().len(), 1);
        assert!(kb.poll_events().is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn ime_key_classified() {
        use win::vk_to_kind;
        assert!(matches!(
            vk_to_kind(0xE5, true),
            Some(KeyEventKind::ImeComposition)
        ));
    }
}
