// MIT License
// Copyright (c) 2024 Graham King

const SUMMARIZE_SYSTEM_PROMPT: &str =
    "Respond in the first-person as if you are the author. Never refer to the blog post directly.";

//const SUMMARIZE_PROMPT: &str = "Read this blog post and then teach me the content in one short, concise paragraph. Use an active voice. Be direct. Only cover the key points.";
const SUMMARIZE_PROMPT: &str = "Re-write this as a single short concise paragraph, using an active voice. Be direct. Only cover the key points.";

pub const CHAT_MODEL_BIG: &str = "gpt-4o";
pub const CHAT_MODEL_SMALL: &str = "gpt-4o-mini";

#[derive(Debug, serde::Serialize)]
struct EmbedRequest<'a> {
    model: &'static str,
    input: &'a str,
}

#[derive(Debug, serde::Deserialize)]
struct EmbedResponse {
    data: Vec<Embedding>,
}

#[derive(Debug, serde::Deserialize)]
struct Embedding {
    embedding: Vec<f64>,
}

/// Use model text-embedding-3-small to calculate an embedding for this string
pub fn embed(body: &str) -> anyhow::Result<Vec<f64>> {
    let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
        return Err(anyhow::anyhow!("Set variable OPENAI_API_KEY to your key"));
    };
    let req = EmbedRequest {
        model: "text-embedding-3-small",
        input: body,
    };
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://api.openai.com/v1/embeddings")
        .bearer_auth(api_key)
        .json(&req)
        .send()?;
    if res.status() != http::StatusCode::OK {
        return Err(anyhow::anyhow!("HTTP error {}", res.status()));
    }
    let mut out: EmbedResponse = res.json()?;
    Ok(out.data.remove(0).embedding)

    /* Example response
    {
      "object": "list",
      "data": [
        {
          "object": "embedding",
          "index": 0,
          "embedding": [
            -0.006929283495992422,
            -0.005336422007530928,
            ... (omitted for spacing)
            -4.547132266452536e-05,
            -0.024047505110502243
          ],
        }
      ],
      "model": "text-embedding-3-small",
      "usage": {
        "prompt_tokens": 5,
        "total_tokens": 5
      }
    }
    */
}

#[derive(Debug, serde::Serialize)]
struct ChatRequest {
    model: &'static str,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, serde::Deserialize)]
struct ChatResponse {
    choices: Vec<ChatResponseChoice>,
}

#[derive(Debug, serde::Deserialize)]
struct ChatResponseChoice {
    message: ChatMessage,
}

/// Use 4o to summarize the given string
pub fn summarize(model: &'static str, s: &str) -> anyhow::Result<String> {
    let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
        return Err(anyhow::anyhow!("Set variable OPENAI_API_KEY to your key"));
    };

    let req = ChatRequest {
        model,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: SUMMARIZE_SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("{SUMMARIZE_PROMPT}\n\n{s}"),
            },
        ],
    };
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&req)
        .send()?;
    if res.status() != http::StatusCode::OK {
        return Err(anyhow::anyhow!("HTTP error {}", res.status()));
    }
    let mut out: ChatResponse = res.json()?;
    let Some(c0) = out.choices.pop() else {
        return Err(anyhow::anyhow!("No choices in response: {out:?}"));
    };
    Ok(c0.message.content)

    /* REQUEST
    curl "https://api.openai.com/v1/chat/completions" \
        -d '{
            "model": "gpt-4o-mini",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful assistant."
                },
                {
                    "role": "user",
                    "content": "Write a haiku that explains the concept of recursion."
                }
            ]
        }'
    */

    /* RESPONSE
    {
      "id": "chatcmpl-A35WeN4yONhlhuGncWbMZYmGMQPuU",
      "object": "chat.completion",
      "created": 1725299588,
      "model": "gpt-4o-mini-2024-07-18",
      "choices": [
        {
          "index": 0,
          "message": {
            "role": "assistant",
            "content": "A call within calls,  \nNestled in self-similarity,  \nLimits echo back.",
            "refusal": null
          },
          "logprobs": null,
          "finish_reason": "stop"
        }
      ],
      "usage": {
        "prompt_tokens": 28,
        "completion_tokens": 19,
        "total_tokens": 47
      },
      "system_fingerprint": "fp_f905cf32a9"
    }
    */
}
