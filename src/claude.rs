// MIT License
// Copyright (c) 2024 Graham King

const SUMMARIZE_SYSTEM_PROMPT: &str =
    "Respond in the first-person as if you are the author. Never refer to the blog post directly.";

const SUMMARIZE_PROMPT: &str = "Re-write this as a single short concise paragraph, using an active voice. Be direct. Only cover the key points.";

pub const CHAT_MODEL_BIG: &str = "claude-3-5-sonnet-20240620";
pub const CHAT_MODEL_SMALL: &str = "claude-3-haiku-20240307";

#[derive(Debug, serde::Serialize)]
struct ChatRequest {
    model: &'static str,
    max_tokens: usize,
    system: &'static str,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, serde::Deserialize)]
struct ChatResponse {
    content: Vec<ChatResponseText>,
}
#[derive(Debug, serde::Deserialize)]
struct ChatResponseText {
    text: String,
}

pub fn summarize(model: &'static str, s: &str) -> anyhow::Result<String> {
    let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") else {
        return Err(anyhow::anyhow!(
            "Set variable ANTHROPIC_API_KEY to your key"
        ));
    };
    let req = ChatRequest {
        model,
        max_tokens: 1024,
        system: SUMMARIZE_SYSTEM_PROMPT,
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: format!("{SUMMARIZE_PROMPT}\n\n{s}"),
        }],
    };
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()?;
    if res.status() != http::StatusCode::OK {
        return Err(anyhow::anyhow!(
            "HTTP error {} {:?}",
            res.status(),
            res.text()
        ));
    }
    let mut out: ChatResponse = res.json()?;
    let Some(c0) = out.content.pop() else {
        return Err(anyhow::anyhow!("No content in response: {out:?}"));
    };
    Ok(c0.text)
}
