use serde::{Deserialize, Serialize};
use serde_json::Value;

fn deserialize_message_content<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Anthropic-style structured content (`[{type, text, cache_control}]`) and
    // legacy plain-string content must both round-trip. `null`/missing degrades
    // to an empty string so older clients keep working without a content field.
    let raw = Option::<Value>::deserialize(deserializer)?;
    Ok(match raw {
        Some(Value::Null) | None => Value::String(String::new()),
        Some(value) => value,
    })
}

// ── OpenAI-compatible Request ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    /// Extension: force a specific provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    /// Either a plain string or a structured content array (Anthropic-style with
    /// `cache_control`). Stored as `Value` so transforms can either pass it
    /// through verbatim (preserving cache_control) or flatten to a string via
    /// `content_text()` for providers that only accept plain strings.
    #[serde(default, deserialize_with = "deserialize_message_content")]
    pub content: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// Flatten `content` into a plain text string. Used by transforms targeting
    /// providers that don't accept structured content blocks.
    pub fn content_text(&self) -> String {
        match &self.content {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            Value::Array(items) => items
                .iter()
                .filter_map(|item| {
                    item.get("text")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .or_else(|| item.as_str().map(str::to_string))
                })
                .collect::<Vec<_>>()
                .join(""),
            other => other.to_string(),
        }
    }
}

// ── OpenAI-compatible Response ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_hit_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_miss_tokens: Option<u64>,
    /// Anthropic-only: tokens served from upstream prompt cache. Distinct from
    /// `prompt_cache_hit_tokens` (DeepSeek) so consumers can attribute cache
    /// behavior per provider without aliasing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u64>,
    /// Anthropic-only: one-off tokens billed for writing a new cache entry the
    /// first time a stable prefix is seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u64>,
}

// ── Streaming ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Supports both "thinking" (Anthropic) and "reasoning_content" (DeepSeek-R1)
    #[serde(
        alias = "reasoning_content",
        skip_serializing_if = "Option::is_none"
    )]
    pub thinking: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<Value>>,
}

// ── Error ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: Option<String>,
}
