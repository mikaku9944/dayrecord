//! Linux AT-SPI context sampler (degraded: returns None when unavailable).

use dayrecord_core::ports::ContextSampler;

#[derive(Debug, Default, Clone)]
pub struct LinuxContextSampler;

impl ContextSampler for LinuxContextSampler {
    fn sample_context(&self) -> Option<String> {
        #[cfg(feature = "linux-atspi")]
        {
            return crate::context_atspi::poll_focused_text();
        }
        #[cfg(not(feature = "linux-atspi"))]
        {
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                tracing::debug!("AT-SPI context unavailable on Wayland without linux-atspi feature");
            }
            None
        }
    }
}
