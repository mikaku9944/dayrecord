//! macOS keyboard capture via CGEventTap (requires Input Monitoring permission).

use dayrecord_core::models::KeyEvent;

pub fn start(_tx: std::sync::mpsc::Sender<KeyEvent>) -> Result<(), String> {
    #[cfg(feature = "macos-keyboard")]
    {
        return crate::keyboard_macos_impl::start(_tx);
    }
    #[cfg(not(feature = "macos-keyboard"))]
    {
        tracing::warn!(
            "macOS keyboard capture disabled; build with --features macos-keyboard and grant Input Monitoring"
        );
        Ok(())
    }
}
