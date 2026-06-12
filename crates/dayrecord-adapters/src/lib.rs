pub mod clock;
pub mod clipboard;
pub mod keyboard;
#[cfg(windows)]
pub mod keyboard_hook;
pub mod keyboard_capture;
#[cfg(target_os = "macos")]
pub mod keyboard_macos;
#[cfg(all(target_os = "macos", feature = "macos-keyboard"))]
pub mod keyboard_macos_impl;
#[cfg(target_os = "linux")]
pub mod keyboard_linux;
pub mod llm;
pub mod platform;
pub mod repository;
pub mod secret;
pub mod secret_keyring;
pub mod uia_common;
pub mod window_unified;
#[cfg(windows)]
pub mod uia;
#[cfg(windows)]
pub mod window;
#[cfg(target_os = "macos")]
pub mod context_macos;
#[cfg(all(target_os = "macos", feature = "macos-ax"))]
pub mod context_ax;
#[cfg(target_os = "linux")]
pub mod context_linux;

pub use clock::SystemClock;
pub use llm::DeepSeekClient;
pub use platform::{
    platform_clipboard, platform_context_sampler, platform_secret_store, platform_window_sampler,
    start_keyboard_capture,
};
pub use repository::SqliteRepository;
pub use secret::KeyringSecretStore;
pub use window_unified::ActiveWindowSampler;

use dayrecord_core::ports::ContextSampler;

#[cfg(windows)]
pub struct WinContextSampler;

#[cfg(windows)]
impl Default for WinContextSampler {
    fn default() -> Self {
        Self
    }
}

#[cfg(windows)]
impl ContextSampler for WinContextSampler {
    fn sample_context(&self) -> Option<String> {
        uia::poll_focused_text()
    }
}
