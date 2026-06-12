//! Headless background capture loop (no GUI).

use crate::runtime::AppRuntime;
use anyhow::Result;
use dayrecord_adapters::{
    platform_clipboard, platform_window_sampler, start_keyboard_capture, SqliteRepository,
    SystemClock,
};
use dayrecord_core::paths;
use dayrecord_core::ports::LlmClient;
use dayrecord_runtime::Orchestrator;
use std::sync::Arc;
use std::time::Duration;

struct MockLlm;

impl LlmClient for MockLlm {
    fn complete(
        &self,
        _: &str,
        _: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(String::new())
    }
}

pub async fn run(_rt: AppRuntime) -> Result<()> {
    paths::ensure_data_dir()?;
    let repo = Arc::new(SqliteRepository::open(&paths::db_path())?);

    let orch: Arc<DaemonOrch> = Arc::new(Orchestrator::new(
        Arc::new(SystemClock),
        repo,
        Arc::new(MockLlm),
        Arc::new(platform_window_sampler()),
        Arc::new(platform_clipboard()),
        Arc::new(daemon_context_sampler()),
    ));

    orch.set_recording(true);
    tracing::info!("dayrecord daemon started");

    let (tx, rx) = std::sync::mpsc::channel();
    let _ = start_keyboard_capture(tx);
    let orch_kb = orch.clone();
    std::thread::spawn(move || {
        loop {
            for event in rx.try_iter() {
                let _ = orch_kb.handle_key_event(event);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    loop {
        let _ = orch.tick_window_sample();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(windows)]
fn daemon_context_sampler() -> dayrecord_adapters::WinContextSampler {
    dayrecord_adapters::WinContextSampler::default()
}
#[cfg(target_os = "macos")]
fn daemon_context_sampler() -> dayrecord_adapters::context_macos::MacContextSampler {
    dayrecord_adapters::context_macos::MacContextSampler::default()
}
#[cfg(target_os = "linux")]
fn daemon_context_sampler() -> dayrecord_adapters::context_linux::LinuxContextSampler {
    dayrecord_adapters::context_linux::LinuxContextSampler::default()
}
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn daemon_context_sampler() -> dayrecord_core::ports::NullContextSampler {
    dayrecord_core::ports::NullContextSampler
}

#[cfg(windows)]
type DaemonOrch = Orchestrator<
    SystemClock,
    SqliteRepository,
    MockLlm,
    dayrecord_adapters::ActiveWindowSampler,
    dayrecord_adapters::clipboard::ArboardClipboard,
    dayrecord_adapters::WinContextSampler,
>;
#[cfg(target_os = "macos")]
type DaemonOrch = Orchestrator<
    SystemClock,
    SqliteRepository,
    MockLlm,
    dayrecord_adapters::ActiveWindowSampler,
    dayrecord_adapters::clipboard::ArboardClipboard,
    dayrecord_adapters::context_macos::MacContextSampler,
>;
#[cfg(target_os = "linux")]
type DaemonOrch = Orchestrator<
    SystemClock,
    SqliteRepository,
    MockLlm,
    dayrecord_adapters::ActiveWindowSampler,
    dayrecord_adapters::clipboard::ArboardClipboard,
    dayrecord_adapters::context_linux::LinuxContextSampler,
>;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
type DaemonOrch = Orchestrator<
    SystemClock,
    SqliteRepository,
    MockLlm,
    dayrecord_adapters::ActiveWindowSampler,
    dayrecord_adapters::clipboard::ArboardClipboard,
    dayrecord_core::ports::NullContextSampler,
>;
