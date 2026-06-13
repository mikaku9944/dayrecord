//! Build configured LLM client from secret store + repository settings.

use crate::llm::{DeepSeekClient, DEEPSEEK_MODEL, DEEPSEEK_URL};
use dayrecord_core::ports::{LlmClient, Repository, SecretStore};
use std::error::Error;
use std::sync::Mutex;

pub struct MockLlm;

impl LlmClient for MockLlm {
    fn complete(&self, _: &str, _: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok("## 今日概览（含大致时间分配）\nplaceholder\n## 主要工作内容（按应用/项目/场景分组，结合时长说明投入）\nplaceholder\n## 重要粘贴片段摘要\nplaceholder\n## 明日待办（能推断则列出，否则写「暂无」）\n暂无".into())
    }
}

enum LlmBackend {
    Client(DeepSeekClient),
    Mock(MockLlm),
}

/// Reloadable LLM client honoring `llm_base_url` / `llm_model` settings.
pub struct ConfiguredLlm {
    backend: Mutex<LlmBackend>,
    repo_settings: std::sync::Arc<dyn Fn() -> (Option<String>, Option<String>) + Send + Sync>,
}

impl ConfiguredLlm {
    pub fn from_key_and_settings<R: Repository + 'static>(
        repo: std::sync::Arc<R>,
        api_key: Option<String>,
    ) -> Self {
        let repo_settings: std::sync::Arc<dyn Fn() -> (Option<String>, Option<String>) + Send + Sync> =
            std::sync::Arc::new(move || {
                let base = repo.get_setting("llm_base_url").ok().flatten();
                let model = repo.get_setting("llm_model").ok().flatten();
                (base, model)
            });
        Self {
            backend: Mutex::new(build_backend(api_key, &repo_settings())),
            repo_settings,
        }
    }

    pub fn reload(&self, api_key: Option<String>) {
        let mut backend = self.backend.lock().unwrap();
        *backend = build_backend(api_key, &(self.repo_settings)());
    }
}

fn build_backend(
    api_key: Option<String>,
    settings: &(Option<String>, Option<String>),
) -> LlmBackend {
    let (base_url, model) = settings;
    match api_key.filter(|k| !k.is_empty()) {
        Some(key) => {
            let mut client = DeepSeekClient::new(key);
            if let Some(url) = base_url.as_ref().filter(|u| !u.is_empty()) {
                client = client.with_base_url(url.clone());
            } else {
                client = client.with_base_url(DEEPSEEK_URL);
            }
            if let Some(m) = model.as_ref().filter(|v| !v.is_empty()) {
                client = client.with_model(m.clone());
            } else {
                client = client.with_model(DEEPSEEK_MODEL);
            }
            LlmBackend::Client(client)
        }
        None => LlmBackend::Mock(MockLlm),
    }
}

impl LlmClient for ConfiguredLlm {
    fn complete(&self, system: &str, user: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let backend = self.backend.lock().unwrap();
        match &*backend {
            LlmBackend::Client(c) => c.complete(system, user),
            LlmBackend::Mock(m) => m.complete(system, user),
        }
    }
}

pub fn load_api_key<S: SecretStore>(store: &S) -> Option<String> {
    store
        .get("deepseek_api_key")
        .ok()
        .flatten()
        .or_else(|| store.get("llm_api_key").ok().flatten())
        .filter(|k| !k.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dayrecord_core::ports::InMemoryRepository;

    #[test]
    fn mock_when_no_api_key() {
        let repo = std::sync::Arc::new(InMemoryRepository::default());
        let llm = ConfiguredLlm::from_key_and_settings(repo, None);
        let out = llm.complete("sys", "user").expect("complete");
        assert!(out.contains("今日概览"));
    }
}
