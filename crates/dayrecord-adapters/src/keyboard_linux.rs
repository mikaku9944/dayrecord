//! Linux keyboard capture — X11 only (Wayland not supported).

use dayrecord_core::models::KeyEvent;

pub fn start(_tx: std::sync::mpsc::Sender<KeyEvent>) -> Result<(), String> {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        tracing::warn!("Linux keyboard capture is not supported on Wayland");
        return Ok(());
    }
    #[cfg(feature = "linux-x11-keyboard")]
    {
        return crate::keyboard_linux_x11::start(_tx);
    }
    #[cfg(not(feature = "linux-x11-keyboard"))]
    {
        tracing::warn!(
            "Linux X11 keyboard capture disabled; build with --features linux-x11-keyboard"
        );
        Ok(())
    }
}
