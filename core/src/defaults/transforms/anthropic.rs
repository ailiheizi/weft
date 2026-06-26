use crate::config::ProviderConfig;
use crate::layers::transform::{ProviderRequest, TransformLayer};
use crate::types::{
    ChatMessage, ChatRequest, ChatResponse, Choice, Delta, StreamChoice, StreamChunk, Usage,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Anthropic request types ──

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u64,
    /// Either a single string (legacy) or a structured `[{type:text,text:...,
    /// cache_control:...}]` array. Stored as `Value` so cache_control breakpoints
    /// emitted by upstream agents (e.g. agent-core's `cache_control: ephemeral`)
    /// reach Anthropic verbatim — they are silently dropped if the system field
    /// is stringified.
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Value>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    /// Same rationale as `AnthropicRequest::system` — preserve content blocks
    /// (including cache_control) instead of collapsing to a string.
    content: Value,
}

// ── Anthropic response types ──

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    _type: String,
    model: String,
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    _type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
}

// ── Anthropic streaming types ──

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    index: Option<u32>,
    #[serde(default)]
    delta: Option<AnthropicDelta>,
    #[serde(default)]
    message: Option<AnthropicStreamMessage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    _type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamMessage {
    id: String,
    model: String,
}

const DEFAULT_MAX_TOKENS: u64 = 4096;
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Transforms for Anthropic's native Messages API.
pub struct AnthropicTransform;

#[async_trait]
impl TransformLayer for AnthropicTransform {
    async fn transform_request(
        &self,
        request: &ChatRequest,
        api_key: &str,
        provider: &ProviderConfig,
    ) -> Result<ProviderRequest> {
        let url = format!("{}/messages", provider.base_url.trim_end_matches('/'));

        // Pull out system messages (top-level Anthropic field), keep everything
        // else as a regular message. Two reasons we don't collapse to a string:
        //
        //   1. agent-core emits the stable system block as
        //      `{role:system, content:[{type:text, text, cache_control:ephemeral}]}`
        //      and Anthropic only honors cache_control when system is sent as a
        //      structured array. Stringifying drops the breakpoint silently.
        //   2. Multiple system messages (e.g. stable + dynamic) must be merged
        //      while preserving any cache_control from the first block.
        let mut system_blocks: Vec<Value> = Vec::new();
        let mut messages: Vec<AnthropicMessage> = Vec::new();

        for msg in &request.messages {
            if msg.role == "system" {
                match &msg.content {
                    Value::Array(items) => {
                        for item in items {
                            system_blocks.push(item.clone());
                        }
                    }
                    Value::String(text) if !text.is_empty() => {
                        system_blocks.push(serde_json::json!({
                            "type": "text",
                            "text": text,
                        }));
                    }
                    Value::Null => {}
                    other if !other.is_string() => {
                        system_blocks.push(serde_json::json!({
                            "type": "text",
                            "text": other.to_string(),
                        }));
                    }
                    _ => {}
                }
            } else {
                messages.push(AnthropicMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                });
            }
        }

        // Encode `system` in the simplest form Anthropic accepts: empty -> None,
        // single plain-text block with no cache_control -> string, anything else
        // -> array (which lets cache_control survive).
        let system: Option<Value> = if system_blocks.is_empty() {
            None
        } else if system_blocks.len() == 1
            && system_blocks[0].get("type").and_then(Value::as_str) == Some("text")
            && system_blocks[0].get("cache_control").is_none()
        {
            system_blocks
                .into_iter()
                .next()
                .and_then(|block| {
                    block
                        .get("text")
                        .and_then(Value::as_str)
                        .map(|s| Value::String(s.to_string()))
                })
        } else {
            Some(Value::Array(system_blocks))
        };

        let anthropic_req = AnthropicRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            system,
            messages,
            temperature: request.temperature,
            top_p: request.top_p,
            stream: request.stream,
        };

        let body =
            serde_json::to_vec(&anthropic_req).context("Failed to serialize Anthropic request")?;

        Ok(ProviderRequest {
            url,
            method: "POST".into(),
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
                ("x-api-key".into(), api_key.to_string()),
                ("anthropic-version".into(), ANTHROPIC_VERSION.into()),
            ],
            body: Bytes::from(body),
        })
    }

    async fn transform_response(
        &self,
        status: u16,
        body: Bytes,
        _provider: &ProviderConfig,
    ) -> Result<ChatResponse> {
        if status != 200 {
            let text = String::from_utf8_lossy(&body);
            anyhow::bail!("Anthropic returned status {}: {}", status, text);
        }

        let resp: AnthropicResponse =
            serde_json::from_slice(&body).context("Failed to parse Anthropic response")?;

        let text = resp
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        let total = resp.usage.input_tokens + resp.usage.output_tokens;

        Ok(ChatResponse {
            id: resp.id,
            object: "chat.completion".into(),
            created: 0, // Anthropic doesn't return a unix timestamp
            model: resp.model,
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".into(),
                    content: Value::String(text),
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".into()),
            }],
            usage: Some(Usage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: total,
                prompt_cache_hit_tokens: None,
                prompt_cache_miss_tokens: None,
                cache_read_input_tokens: resp.usage.cache_read_input_tokens,
                cache_creation_input_tokens: resp.usage.cache_creation_input_tokens,
            }),
        })
    }

    async fn transform_stream_chunk(
        &self,
        chunk: &str,
        _provider: &ProviderConfig,
    ) -> Result<Option<StreamChunk>> {
        let line = chunk.trim();

        // Skip empty lines, SSE comments, and event-type lines
        if line.is_empty() || line.starts_with(':') || line.starts_with("event:") {
            return Ok(None);
        }

        let data = line.strip_prefix("data: ").unwrap_or(line);

        let event: AnthropicStreamEvent =
            serde_json::from_str(data).context("Failed to parse Anthropic stream event")?;

        match event.event_type.as_str() {
            "content_block_delta" => {
                let text = event.delta.and_then(|d| d.text).unwrap_or_default();

                Ok(Some(StreamChunk {
                    id: String::new(),
                    object: "chat.completion.chunk".into(),
                    created: 0,
                    model: String::new(),
                    choices: vec![StreamChoice {
                        index: event.index.unwrap_or(0),
                        delta: Delta {
                            content: Some(text),
                            ..Default::default()
                        },
                        finish_reason: None,
                    }],
                }))
            }
            "message_start" => {
                // Extract id/model from the message_start for downstream consumers
                let (id, model) = match &event.message {
                    Some(m) => (m.id.clone(), m.model.clone()),
                    None => (String::new(), String::new()),
                };
                Ok(Some(StreamChunk {
                    id,
                    object: "chat.completion.chunk".into(),
                    created: 0,
                    model,
                    choices: vec![StreamChoice {
                        index: 0,
                        delta: Delta {
                            role: Some("assistant".into()),
                            ..Default::default()
                        },
                        finish_reason: None,
                    }],
                }))
            }
            "message_stop" => Ok(Some(StreamChunk {
                id: String::new(),
                object: "chat.completion.chunk".into(),
                created: 0,
                model: String::new(),
                choices: vec![StreamChoice {
                    index: 0,
                    delta: Delta::default(),
                    finish_reason: Some("stop".into()),
                }],
            })),
            // message_delta, content_block_start, content_block_stop, ping — skip
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApiKeyConfig, ProviderApi, ProviderConfig};
    use crate::types::{ChatMessage, ChatRequest};

    fn provider() -> ProviderConfig {
        ProviderConfig {
            name: "anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            format: "anthropic".into(),
            api: ProviderApi::ChatCompletions,
            keys: vec![ApiKeyConfig {
                value: "sk-ant-test".into(),
                label: None,
                enabled: true,
            }],
            models: vec!["claude-sonnet-4-20250514".into()],
        }
    }

    fn request_with_system() -> ChatRequest {
        ChatRequest {
            model: "claude-sonnet-4-20250514".into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "You are helpful.".into(),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".into(),
                    content: "hello".into(),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            stream: false,
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            x_provider: None,
        }
    }

    fn request_no_system() -> ChatRequest {
        ChatRequest {
            model: "claude-sonnet-4-20250514".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hello".into(),
                tool_calls: None,
                tool_call_id: None,
            }],
            stream: false,
            temperature: Some(0.7),
            max_tokens: None,
            top_p: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            x_provider: None,
        }
    }

    #[tokio::test]
    async fn test_request_url_and_headers() {
        let t = AnthropicTransform;
        let req = t
            .transform_request(&request_with_system(), "sk-ant-test", &provider())
            .await
            .unwrap();

        assert_eq!(req.url, "https://api.anthropic.com/v1/messages");
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "x-api-key" && v == "sk-ant-test"));
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "anthropic-version" && v == ANTHROPIC_VERSION));
        // Must NOT have Bearer auth
        assert!(!req.headers.iter().any(|(k, _)| k == "Authorization"));
    }

    #[tokio::test]
    async fn test_request_system_extracted() {
        let t = AnthropicTransform;
        let req = t
            .transform_request(&request_with_system(), "sk-ant-test", &provider())
            .await
            .unwrap();

        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        assert_eq!(body["system"], "You are helpful.");
        // Messages should only contain the user message
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
    }

    #[tokio::test]
    async fn test_request_no_system() {
        let t = AnthropicTransform;
        let req = t
            .transform_request(&request_no_system(), "sk-ant-test", &provider())
            .await
            .unwrap();

        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        assert!(body.get("system").is_none());
    }

    #[tokio::test]
    async fn test_request_default_max_tokens() {
        let t = AnthropicTransform;
        let req = t
            .transform_request(&request_no_system(), "sk-ant-test", &provider())
            .await
            .unwrap();

        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
    }

    #[tokio::test]
    async fn test_request_explicit_max_tokens() {
        let t = AnthropicTransform;
        let req = t
            .transform_request(&request_with_system(), "sk-ant-test", &provider())
            .await
            .unwrap();

        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        assert_eq!(body["max_tokens"], 1024);
    }

    #[tokio::test]
    async fn test_transform_response() {
        let t = AnthropicTransform;
        let anthropic_body = serde_json::json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": "Hello!"}],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });
        let body = Bytes::from(serde_json::to_vec(&anthropic_body).unwrap());

        let resp = t.transform_response(200, body, &provider()).await.unwrap();

        assert_eq!(resp.id, "msg_123");
        assert_eq!(resp.object, "chat.completion");
        assert_eq!(resp.model, "claude-sonnet-4-20250514");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.role, "assistant");
        assert_eq!(resp.choices[0].message.content, "Hello!");
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[tokio::test]
    async fn test_transform_response_error_status() {
        let t = AnthropicTransform;
        let body = Bytes::from(r#"{"error":{"message":"invalid key"}}"#);
        let result = t.transform_response(401, body, &provider()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("401"));
    }

    #[tokio::test]
    async fn cache_control_survives_when_system_is_structured_array() {
        // Reasonix Pillar 1: agent-core emits system content as a structured
        // array with cache_control:ephemeral. Anthropic only honors that when
        // it's sent through verbatim in the request body. This test pins the
        // contract — if it ever regresses, prefix-cache hit rate silently drops
        // to 0 for every Anthropic-routed turn.
        let t = AnthropicTransform;
        let req = ChatRequest {
            model: "claude-sonnet-4-20250514".into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: serde_json::json!([{
                        "type": "text",
                        "text": "You are helpful.",
                        "cache_control": {"type": "ephemeral"}
                    }]),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".into(),
                    content: serde_json::Value::String("hello".into()),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            stream: false,
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            x_provider: None,
        };
        let outbound = t
            .transform_request(&req, "sk-ant-test", &provider())
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&outbound.body).unwrap();
        let system_field = &body["system"];
        assert!(
            system_field.is_array(),
            "structured system must stay an array, got {}",
            system_field
        );
        let first = &system_field[0];
        assert_eq!(first["type"], "text");
        assert_eq!(first["text"], "You are helpful.");
        assert_eq!(first["cache_control"]["type"], "ephemeral");
    }

    #[tokio::test]
    async fn anthropic_response_usage_carries_cache_read_and_creation_tokens() {
        let t = AnthropicTransform;
        let body = Bytes::from(serde_json::to_vec(&serde_json::json!({
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": "ok"}],
            "usage": {
                "input_tokens": 12,
                "output_tokens": 3,
                "cache_read_input_tokens": 9000,
                "cache_creation_input_tokens": 1200
            }
        })).unwrap());
        let resp = t.transform_response(200, body, &provider()).await.unwrap();
        let usage = resp.usage.expect("usage missing");
        assert_eq!(usage.cache_read_input_tokens, Some(9000));
        assert_eq!(usage.cache_creation_input_tokens, Some(1200));
    }

    #[tokio::test]
    async fn test_stream_content_block_delta() {
        let t = AnthropicTransform;
        let data = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#;
        let result = t.transform_stream_chunk(data, &provider()).await.unwrap();
        let chunk = result.unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hi"));
        assert!(chunk.choices[0].finish_reason.is_none());
    }

    #[tokio::test]
    async fn test_stream_message_start() {
        let t = AnthropicTransform;
        let data = r#"data: {"type":"message_start","message":{"id":"msg_1","model":"claude-sonnet-4-20250514","role":"assistant","content":[],"usage":{"input_tokens":10,"output_tokens":0}}}"#;
        let result = t.transform_stream_chunk(data, &provider()).await.unwrap();
        let chunk = result.unwrap();
        assert_eq!(chunk.id, "msg_1");
        assert_eq!(chunk.choices[0].delta.role.as_deref(), Some("assistant"));
    }

    #[tokio::test]
    async fn test_stream_message_stop() {
        let t = AnthropicTransform;
        let data = r#"data: {"type":"message_stop"}"#;
        let result = t.transform_stream_chunk(data, &provider()).await.unwrap();
        let chunk = result.unwrap();
        assert_eq!(chunk.choices[0].finish_reason.as_deref(), Some("stop"));
    }

    #[tokio::test]
    async fn test_stream_skip_event_line() {
        let t = AnthropicTransform;
        let result = t
            .transform_stream_chunk("event: content_block_delta", &provider())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stream_skip_empty() {
        let t = AnthropicTransform;
        let result = t.transform_stream_chunk("", &provider()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stream_skip_ping() {
        let t = AnthropicTransform;
        let data = r#"data: {"type":"ping"}"#;
        let result = t.transform_stream_chunk(data, &provider()).await.unwrap();
        assert!(result.is_none());
    }
}
