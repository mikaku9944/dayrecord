use dayrecord_adapters::DeepSeekClient;
use dayrecord_core::ports::LlmClient;

#[test]
#[ignore = "requires DEEPSEEK_API_KEY and network"]
fn live_deepseek_smoke() {
    let key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY");
    let client = DeepSeekClient::new(key);
    let out = client
        .complete("You are a test assistant.", "Reply with exactly: OK")
        .expect("api call");
    assert!(!out.is_empty());
}
