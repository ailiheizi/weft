use crate::config::{ProviderApi, ProviderConfig};
use crate::layers::transform::{ProviderRequest, TransformLayer};
use crate::types::{ChatMessage, ChatRequest, ChatResponse, Choice, StreamChunk, Usage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

fn normalize_openai_usage_fields(response: &mut serde_json::Value) {
    let Some(usage) = response.get_mut("usage") else {
        return;
    };
    let Some(usage_object) = usage.as_object_mut() else {
        return;
    };
    let prompt_tokens = usage_object
        .get("prompt_tokens")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let completion_tokens = usage_object
        .get("completion_tokens")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let total_tokens = usage_object
        .get("total_tokens")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(prompt_tokens.saturating_add(completion_tokens));
    let prompt_cache_hit_tokens = usage_object
        .get("prompt_cache_hit_tokens")
        .and_then(serde_json::Value::as_u64);
    let prompt_cache_miss_tokens = usage_object
        .get("prompt_cache_miss_tokens")
        .and_then(serde_json::Value::as_u64);
    // Some OpenAI-compatible upstreams (Anthropic via OpenAI-compat shim, xAI)
    // expose Anthropic-style cache fields too. Capture them so per-provider
    // cache attribution survives the normalize step.
    let cache_read_input_tokens = usage_object
        .get("cache_read_input_tokens")
        .and_then(serde_json::Value::as_u64);
    let cache_creation_input_tokens = usage_object
        .get("cache_creation_input_tokens")
        .and_then(serde_json::Value::as_u64);

    *usage = serde_json::to_value(Usage {
        prompt_tokens,
        completion_tokens,
        total_tokens,
        prompt_cache_hit_tokens,
        prompt_cache_miss_tokens,
        cache_read_input_tokens,
        cache_creation_input_tokens,
    })
    .unwrap_or_else(|_| usage.clone());
}

/// Transforms for OpenAI-compatible APIs (OpenRouter, OpenAI, etc.)
/// These APIs accept the same JSON format we use internally,
/// so the transform is mostly pass-through + auth header.
pub struct OpenAITransform;

fn normalize_openai_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    let lower = trimmed.to_lowercase();

    if lower.ends_with("/chat/completions") || lower.ends_with("/responses") {
        return trimmed.to_string();
    }

    if lower.ends_with("/v1") || lower.ends_with("/openai/v1") {
        return trimmed.to_string();
    }

    if lower.contains("api.openai.com") || lower.contains("api.deepseek.com") {
        return format!("{trimmed}/v1");
    }

    trimmed.to_string()
}

fn openai_endpoint(provider: &ProviderConfig) -> &'static str {
    match provider.api {
        ProviderApi::ChatCompletions => "chat/completions",
        ProviderApi::Responses => "responses",
    }
}

#[derive(Debug, Serialize)]
struct ResponsesRequest {
    model: String,
    input: Vec<ResponsesInputMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ResponsesInputMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ResponsesResponse {
    id: String,
    model: String,
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    output: Vec<ResponsesOutputItem>,
    #[serde(default)]
    usage: Option<ResponsesUsage>,
}

#[derive(Debug, Deserialize)]
struct ResponsesOutputItem {
    #[serde(default)]
    content: Vec<ResponsesContentItem>,
}

#[derive(Debug, Deserialize)]
struct ResponsesContentItem {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsesUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

fn responses_body(request: &ChatRequest) -> ResponsesRequest {
    ResponsesRequest {
        model: request.model.clone(),
        input: request
            .messages
            .iter()
            .map(|message| ResponsesInputMessage {
                role: message.role.clone(),
                // Responses API only accepts string content. Structured content
                // (e.g. Anthropic-style `cache_control` blocks) gets flattened
                // to plain text here — cache_control is meaningless on this
                // endpoint anyway.
                content: message.content_text(),
            })
            .collect(),
        temperature: request.temperature,
        max_output_tokens: request.max_tokens,
        top_p: request.top_p,
        tools: request.tools.clone(),
        tool_choice: request.tool_choice.clone(),
        stream: request.stream,
    }
}

fn response_text(resp: &ResponsesResponse) -> String {
    if let Some(text) = &resp.output_text {
        if !text.is_empty() {
            return text.clone();
        }
    }

    resp.output
        .iter()
        .flat_map(|item| item.content.iter())
        .filter_map(|content| content.text.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

#[async_trait]
impl TransformLayer for OpenAITransform {
    async fn transform_request(
        &self,
        request: &ChatRequest,
        api_key: &str,
        provider: &ProviderConfig,
    ) -> Result<ProviderRequest> {
        let normalized_base_url = normalize_openai_base_url(&provider.base_url);
        let url = format!(
            "{}/{}",
            normalized_base_url.trim_end_matches('/'),
            openai_endpoint(provider)
        );
        let body = match provider.api {
            ProviderApi::ChatCompletions => {
                serde_json::to_vec(request).context("Failed to serialize request")?
            }
            ProviderApi::Responses => serde_json::to_vec(&responses_body(request))
                .context("Failed to serialize Responses request")?,
        };

        Ok(ProviderRequest {
            url,
            method: "POST".into(),
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
                ("Authorization".into(), format!("Bearer {}", api_key)),
            ],
            body: Bytes::from(body),
        })
    }

    async fn transform_response(
        &self,
        status: u16,
        body: Bytes,
        provider: &ProviderConfig,
    ) -> Result<ChatResponse> {
        if status != 200 {
            let text = String::from_utf8_lossy(&body);
            anyhow::bail!("Provider returned status {}: {}", status, text);
        }
        match provider.api {
            ProviderApi::ChatCompletions => {
                let mut response_value: serde_json::Value =
                    serde_json::from_slice(&body).context("Failed to parse provider response")?;
                normalize_openai_usage_fields(&mut response_value);
                let resp: ChatResponse = serde_json::from_value(response_value)
                    .context("Failed to normalize provider response")?;
                Ok(resp)
            }
            ProviderApi::Responses => {
                let resp: ResponsesResponse =
                    serde_json::from_slice(&body).context("Failed to parse Responses response")?;
                let usage = resp.usage.as_ref().map(|usage| Usage {
                    prompt_tokens: usage.input_tokens,
                    completion_tokens: usage.output_tokens,
                    total_tokens: if usage.total_tokens == 0 {
                        usage.input_tokens + usage.output_tokens
                    } else {
                        usage.total_tokens
                    },
                    prompt_cache_hit_tokens: None,
                    prompt_cache_miss_tokens: None,
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                });
                let content = response_text(&resp);
                Ok(ChatResponse {
                    id: resp.id,
                    object: "chat.completion".into(),
                    created: 0,
                    model: resp.model,
                    choices: vec![Choice {
                        index: 0,
                        message: ChatMessage {
                            role: "assistant".into(),
                            content: serde_json::Value::String(content),
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        finish_reason: Some("stop".into()),
                    }],
                    usage,
                })
            }
        }
    }

    async fn transform_stream_chunk(
        &self,
        chunk: &str,
        _provider: &ProviderConfig,
    ) -> Result<Option<StreamChunk>> {
        let line = chunk.trim();

        // SSE format: "data: {...}" or "data: [DONE]"
        if line.is_empty() || line.starts_with(':') {
            return Ok(None); // comment or keep-alive
        }

        let data = line.strip_prefix("data: ").unwrap_or(line);

        if data == "[DONE]" {
            return Ok(None);
        }

        let chunk: StreamChunk =
            serde_json::from_str(data).context("Failed to parse stream chunk")?;
        Ok(Some(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApiKeyConfig, ProviderApi, ProviderConfig};
    use crate::types::{ChatMessage, ChatRequest};

    fn provider() -> ProviderConfig {
        ProviderConfig {
            name: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            format: "openai".into(),
            api: ProviderApi::ChatCompletions,
            keys: vec![ApiKeyConfig {
                value: "sk-test".into(),
                label: None,
            }],
            models: vec!["gpt-4o".into()],
        }
    }

    fn provider_with_base_url(base_url: &str) -> ProviderConfig {
        ProviderConfig {
            base_url: base_url.into(),
            ..provider()
        }
    }

    fn request() -> ChatRequest {
        ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hello".into(),
                tool_calls: None,
                tool_call_id: None,
            }],
            stream: false,
            temperature: None,
            max_tokens: None,
            top_p: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            x_provider: None,
        }
    }

    #[tokio::test]
    async fn test_transform_request_url() {
        let t = OpenAITransform;
        let req = t
            .transform_request(&request(), "sk-test", &provider())
            .await
            .unwrap();
        assert_eq!(req.url, "https://openrouter.ai/api/v1/chat/completions");
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer sk-test"));
    }

    #[tokio::test]
    async fn test_transform_request_url_normalizes_deepseek_v1() {
        let t = OpenAITransform;
        let provider = provider_with_base_url("https://api.deepseek.com");
        let req = t
            .transform_request(&request(), "sk-test", &provider)
            .await
            .unwrap();

        assert_eq!(req.url, "https://api.deepseek.com/v1/chat/completions");
    }

    #[tokio::test]
    async fn test_transform_request_url_normalizes_deepseek_v1_with_trailing_slash() {
        let t = OpenAITransform;
        let provider = provider_with_base_url(" https://api.deepseek.com/ ");
        let req = t
            .transform_request(&request(), "sk-test", &provider)
            .await
            .unwrap();

        assert_eq!(req.url, "https://api.deepseek.com/v1/chat/completions");
    }

    #[tokio::test]
    async fn test_transform_request_url_preserves_deepseek_explicit_v1() {
        let t = OpenAITransform;
        let provider = provider_with_base_url("https://api.deepseek.com/v1");
        let req = t
            .transform_request(&request(), "sk-test", &provider)
            .await
            .unwrap();

        assert_eq!(req.url, "https://api.deepseek.com/v1/chat/completions");
    }

    #[tokio::test]
    async fn test_transform_request_url_preserves_explicit_v1() {
        let t = OpenAITransform;
        let provider = provider_with_base_url("https://api.openai.com/v1");
        let req = t
            .transform_request(&request(), "sk-test", &provider)
            .await
            .unwrap();

        assert_eq!(req.url, "https://api.openai.com/v1/chat/completions");
    }

    #[tokio::test]
    async fn test_transform_stream_done() {
        let t = OpenAITransform;
        let result = t
            .transform_stream_chunk("data: [DONE]", &provider())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_transform_stream_keepalive() {
        let t = OpenAITransform;
        let result = t.transform_stream_chunk("", &provider()).await.unwrap();
        assert!(result.is_none());
        let result = t
            .transform_stream_chunk(": ping", &provider())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_transform_response_preserves_prompt_cache_usage_fields() {
        let t = OpenAITransform;
        let body = Bytes::from_static(
            br#"{
                "id": "chatcmpl_1",
                "object": "chat.completion",
                "created": 1,
                "model": "deepseek-v4-flash",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 100,
                    "completion_tokens": 20,
                    "total_tokens": 120,
                    "prompt_cache_hit_tokens": 80,
                    "prompt_cache_miss_tokens": 20
                }
            }"#,
        );

        let response = t.transform_response(200, body, &provider()).await.unwrap();
        let usage = response.usage.expect("usage should be present");

        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 120);
        assert_eq!(usage.prompt_cache_hit_tokens, Some(80));
        assert_eq!(usage.prompt_cache_miss_tokens, Some(20));
    }
}
