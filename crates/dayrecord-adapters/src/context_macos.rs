//! macOS Accessibility (AX) context sampler.

use dayrecord_core::ports::ContextSampler;

#[derive(Debug, Default, Clone)]
pub struct MacContextSampler;

impl ContextSampler for MacContextSampler {
    fn sample_context(&self) -> Option<String> {
        #[cfg(feature = "macos-ax")]
        {
            return crate::context_ax::poll_focused_text();
        }
        #[cfg(not(feature = "macos-ax"))]
        {
            tracing::debug!("macOS AX sampler requires macos-ax feature; grant Accessibility permission in System Settings");
            None
        }
    }
}
