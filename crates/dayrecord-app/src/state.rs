use crate::orchestrator::Orchestrator;
use dayrecord_adapters::{
    platform_clipboard, platform_secret_store, platform_window_sampler, ConfiguredLlm,
    KeyringSecretStore, SqliteRepository, SystemClock,
};
use dayrecord_core::paths;
use dayrecord_core::ports::SecretStore;
use std::sync::Arc;

pub type AppWindowSampler = dayrecord_adapters::ActiveWindowSampler;
pub type AppClipboard = dayrecord_adapters::clipboard::ArboardClipboard;
pub type AppLlm = ConfiguredLlm;

pub type AppOrchestrator = Orchestrator<
    SystemClock,
    SqliteRepository,
    AppLlm,
    AppWindowSampler,
    AppClipboard,
    AppContextSampler,
>;

#[cfg(windows)]
pub type AppContextSampler = dayrecord_adapters::WinContextSampler;
#[cfg(windows)]
fn app_context_sampler() -> AppContextSampler {
    dayrecord_adapters::WinContextSampler::default()
}
#[cfg(target_os = "macos")]
pub type AppContextSampler = dayrecord_adapters::context_macos::MacContextSampler;
#[cfg(target_os = "macos")]
fn app_context_sampler() -> AppContextSampler {
    dayrecord_adapters::context_macos::MacContextSampler::default()
}
#[cfg(target_os = "linux")]
pub type AppContextSampler = dayrecord_adapters::context_linux::LinuxContextSampler;
#[cfg(target_os = "linux")]
fn app_context_sampler() -> AppContextSampler {
    dayrecord_adapters::context_linux::LinuxContextSampler::default()
}
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub type AppContextSampler = dayrecord_core::ports::NullContextSampler;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn app_context_sampler() -> AppContextSampler {
    dayrecord_core::ports::NullContextSampler
}

pub fn data_dir() -> std::path::PathBuf {
    paths::ensure_data_dir().unwrap_or_else(|_| paths::data_dir())
}

pub fn db_path() -> std::path::PathBuf {
    paths::db_path()
}

pub fn build_orchestrator(api_key: Option<String>) -> Result<Arc<AppOrchestrator>, String> {
    paths::ensure_data_dir().map_err(|e| e.to_string())?;
    let repo = Arc::new(SqliteRepository::open(&db_path()).map_err(|e| e.to_string())?);
    let llm = Arc::new(ConfiguredLlm::from_key_and_settings(repo.clone(), api_key));
    Ok(Arc::new(Orchestrator::new(
        Arc::new(SystemClock),
        repo,
        llm,
        Arc::new(platform_window_sampler()),
        Arc::new(platform_clipboard()),
        Arc::new(app_context_sampler()),
    )))
}

pub fn secret_store() -> KeyringSecretStore {
    platform_secret_store()
}

pub fn load_api_key<S: SecretStore>(store: &S) -> Option<String> {
    dayrecord_adapters::load_api_key(store)
}

pub fn save_api_key<S: SecretStore>(store: &S, key: &str) -> Result<(), String> {
    store.set("deepseek_api_key", key).map_err(|e| e.to_string())
}
