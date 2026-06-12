use crate::orchestrator::Orchestrator;
use dayrecord_adapters::{
    platform_clipboard, platform_secret_store, platform_window_sampler, DeepSeekClient,
    KeyringSecretStore, SqliteRepository, SystemClock,
};
use dayrecord_core::paths;
use dayrecord_core::ports::{LlmClient, SecretStore};
use std::sync::Arc;

pub type AppWindowSampler = dayrecord_adapters::ActiveWindowSampler;
pub type AppClipboard = dayrecord_adapters::clipboard::ArboardClipboard;

pub type AppOrchestrator = Orchestrator<
    SystemClock,
    SqliteRepository,
    DeepSeekLlm,
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

pub enum DeepSeekLlm {
    Client(DeepSeekClient),
    Mock(MockLlm),
}

pub struct MockLlm;

impl LlmClient for MockLlm {
    fn complete(&self, _: &str, _: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok("## 今日概览（含大致时间分配）\nplaceholder\n## 主要工作内容（按应用/项目/场景分组，结合时长说明投入）\nplaceholder\n## 重要粘贴片段摘要\nplaceholder\n## 明日待办（能推断则列出，否则写「暂无」）\n暂无".into())
    }
}

impl LlmClient for DeepSeekLlm {
    fn complete(&self, system: &str, user: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Self::Client(c) => c.complete(system, user),
            Self::Mock(m) => m.complete(system, user),
        }
    }
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
    let llm = match api_key.filter(|k| !k.is_empty()) {
        Some(key) => DeepSeekLlm::Client(DeepSeekClient::new(key)),
        None => DeepSeekLlm::Mock(MockLlm),
    };
    Ok(Arc::new(Orchestrator::new(
        Arc::new(SystemClock),
        repo,
        Arc::new(llm),
        Arc::new(platform_window_sampler()),
        Arc::new(platform_clipboard()),
        Arc::new(app_context_sampler()),
    )))
}

pub fn secret_store() -> KeyringSecretStore {
    platform_secret_store()
}

pub fn load_api_key<S: SecretStore>(store: &S) -> Option<String> {
    store.get("deepseek_api_key").ok().flatten()
}

pub fn save_api_key<S: SecretStore>(store: &S, key: &str) -> Result<(), String> {
    store.set("deepseek_api_key", key).map_err(|e| e.to_string())
}
