//! Headless background capture loop (no GUI).

use crate::runtime::AppRuntime;
use anyhow::Result;
use dayrecord_adapters::{
    load_api_key, platform_clipboard, platform_secret_store, platform_window_sampler,
    start_keyboard_capture, ConfiguredLlm, SqliteRepository, SystemClock,
};
use dayrecord_core::paths;
use dayrecord_core::ports::Repository;
use dayrecord_runtime::{
    spawn_control_server, try_acquire_instance_lock, OrchestratorControlHandler,
};
use std::sync::Arc;
use std::time::Duration;

pub async fn run(_rt: AppRuntime) -> Result<()> {
    paths::ensure_data_dir()?;
    let _lock = try_acquire_instance_lock().map_err(|e| anyhow::anyhow!(e))?;

    let store = platform_secret_store();
    let api_key = load_api_key(&store);
    let repo = Arc::new(SqliteRepository::open(&paths::db_path())?);
    let llm = Arc::new(ConfiguredLlm::from_key_and_settings(repo.clone(), api_key));

    let orch: Arc<DaemonOrch> = Arc::new(Orchestrator::new(
        Arc::new(SystemClock),
        repo,
        llm,
        Arc::new(platform_window_sampler()),
        Arc::new(platform_clipboard()),
        Arc::new(daemon_context_sampler()),
    ));

    orch.set_recording(true);
    let _ = orch.repo.set_setting("recording", "true");

    let control = Arc::new(OrchestratorControlHandler {
        orchestrator: orch.clone(),
    });
    let _control_thread = spawn_control_server(control);

    tracing::info!("dayrecord daemon started");

    let (tx, rx) = std::sync::mpsc::channel();
    if let Err(e) = start_keyboard_capture(tx) {
        tracing::warn!("keyboard capture unavailable: {e}");
    }
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

use dayrecord_runtime::Orchestrator;

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
    ConfiguredLlm,
    dayrecord_adapters::ActiveWindowSampler,
    dayrecord_adapters::clipboard::ArboardClipboard,
    dayrecord_adapters::WinContextSampler,
>;
#[cfg(target_os = "macos")]
type DaemonOrch = Orchestrator<
    SystemClock,
    SqliteRepository,
    ConfiguredLlm,
    dayrecord_adapters::ActiveWindowSampler,
    dayrecord_adapters::clipboard::ArboardClipboard,
    dayrecord_adapters::context_macos::MacContextSampler,
>;
#[cfg(target_os = "linux")]
type DaemonOrch = Orchestrator<
    SystemClock,
    SqliteRepository,
    ConfiguredLlm,
    dayrecord_adapters::ActiveWindowSampler,
    dayrecord_adapters::clipboard::ArboardClipboard,
    dayrecord_adapters::context_linux::LinuxContextSampler,
>;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
type DaemonOrch = Orchestrator<
    SystemClock,
    SqliteRepository,
    ConfiguredLlm,
    dayrecord_adapters::ActiveWindowSampler,
    dayrecord_adapters::clipboard::ArboardClipboard,
    dayrecord_core::ports::NullContextSampler,
>;
