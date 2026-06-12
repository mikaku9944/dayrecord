//! Keyboard capture port — platform entry points delegate to OS-specific modules.

use dayrecord_core::models::KeyEvent;
use std::sync::mpsc::Sender;

pub trait KeyboardCapture: Send {
    fn start(&self, tx: Sender<KeyEvent>) -> Result<(), String>;
}

pub struct PlatformKeyboardCapture;

impl KeyboardCapture for PlatformKeyboardCapture {
    fn start(&self, tx: Sender<KeyEvent>) -> Result<(), String> {
        crate::platform::start_keyboard_capture(tx)
    }
}
