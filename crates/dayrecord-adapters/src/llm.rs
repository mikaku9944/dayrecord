use dayrecord_core::ports::LlmClient;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;

pub const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";
pub const DEEPSEEK_MODEL: &str = "deepseek-chat";

#[derive(Clone)]
pub struct DeepSeekClient {
    api_key: String,
    base_url: String,
    client: Client,
}

impl DeepSeekClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEEPSEEK_URL.to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("http client"),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

impl LlmClient for DeepSeekClient {
    fn complete(&self, system: &str, user: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let body = ChatRequest {
            model: DEEPSEEK_MODEL,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system,
                },
                ChatMessage {
                    role: "user",
                    content: user,
                },
            ],
            temperature: 0.2,
        };

        let resp = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(format!("deepseek error {status}: {text}").into());
        }

        let parsed: ChatResponse = resp.json()?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| "empty choices".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn sends_expected_payload() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let server = rt.block_on(async { MockServer::start().await });
        rt.block_on(async {
            Mock::given(method("POST"))
                .and(path("/chat/completions"))
                .and(header("authorization", "Bearer test-key"))
                .and(body_json(serde_json::json!({
                    "model": "deepseek-chat",
                    "messages": [
                        {"role": "system", "content": "sys"},
                        {"role": "user", "content": "user"}
                    ],
                    "temperature": 0.2
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "choices": [{"message": {"content": "## 今日概览\nok"}}]
                })))
                .mount(&server)
                .await;
        });

        let client = DeepSeekClient::new("test-key")
            .with_base_url(format!("{}/chat/completions", server.uri()));
        let out = client.complete("sys", "user").expect("complete");
        assert!(out.contains("今日概览"));
    }
}
