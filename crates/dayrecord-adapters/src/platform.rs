//! Platform-selected adapter factories.

use dayrecord_core::ports::ContextSampler;

use crate::clipboard::ArboardClipboard;
use crate::secret::KeyringSecretStore;
use crate::window_unified::ActiveWindowSampler;

#[cfg(windows)]
use crate::WinContextSampler;
#[cfg(not(windows))]
use dayrecord_core::ports::NullContextSampler;

pub fn platform_window_sampler() -> ActiveWindowSampler {
    ActiveWindowSampler::default()
}

pub fn platform_clipboard() -> ArboardClipboard {
    ArboardClipboard::default()
}

pub fn platform_context_sampler() -> impl ContextSampler + Send + Sync + 'static {
    #[cfg(windows)]
    {
        WinContextSampler::default()
    }
    #[cfg(target_os = "macos")]
    {
        crate::context_macos::MacContextSampler::default()
    }
    #[cfg(target_os = "linux")]
    {
        crate::context_linux::LinuxContextSampler::default()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        NullContextSampler
    }
}

pub fn platform_secret_store() -> KeyringSecretStore {
    KeyringSecretStore::new()
}

pub fn start_keyboard_capture(
    tx: std::sync::mpsc::Sender<dayrecord_core::models::KeyEvent>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        crate::keyboard_hook::win::start(tx)
    }
    #[cfg(target_os = "macos")]
    {
        crate::keyboard_macos::start(tx)
    }
    #[cfg(target_os = "linux")]
    {
        crate::keyboard_linux::start(tx)
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = tx;
        Err("keyboard capture not supported on this platform".into())
    }
}
