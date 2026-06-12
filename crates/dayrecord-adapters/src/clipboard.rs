use dayrecord_core::ports::Clipboard;
use std::error::Error;
use std::sync::Mutex;

pub struct MockClipboard {
    text: Mutex<Option<String>>,
}

impl MockClipboard {
    pub fn new(text: Option<String>) -> Self {
        Self {
            text: Mutex::new(text),
        }
    }
}

impl Default for MockClipboard {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Clipboard for MockClipboard {
    fn read_text(&self) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        Ok(self.text.lock().unwrap().clone())
    }
}

pub struct ArboardClipboard;

impl Default for ArboardClipboard {
    fn default() -> Self {
        Self
    }
}

impl Clipboard for ArboardClipboard {
    fn read_text(&self) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        match arboard::Clipboard::new()?.get_text() {
            Ok(t) => Ok(Some(t)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}
