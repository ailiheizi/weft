use crate::api::openai_compat::AppState;
use crate::config::store::save_config;
use crate::config::{ApiKeyConfig, ProviderApi, ProviderConfig};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

pub async fn list_providers(State(state): State<AppState>) -> Json<serde_json::Value> {
    let config = state.config.read().await;
    let providers: Vec<serde_json::Value> = config
        .providers
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "base_url": p.base_url,
                "format": p.format,
                "models": p.models,
                "key_count": p.keys.len(),
            })
        })
        .collect();
    // 同时返回路由信息，让前端区分文本LLM(default_provider)与图像(image_provider)用途。
    Json(serde_json::json!({
        "providers": providers,
        "routing": {
            "default_provider": config.routing.default_provider,
            "default_model": config.routing.default_model,
            "image_provider": config.routing.image_provider,
        }
    }))
}

pub async fn get_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    match config.providers.iter().find(|p| p.name == name) {
        Some(p) => {
            // get_provider 用于编辑对话框,需返回完整 keys(本地单用户应用,
            // key 归用户所有,通过 runtime-token 鉴权后可见)。列表接口仍只给 key_count。
            let keys: Vec<serde_json::Value> = p
                .keys
                .iter()
                .map(|k| {
                    serde_json::json!({
                        "value": k.value,
                        "label": k.label,
                        "enabled": k.enabled,
                    })
                })
                .collect();
            let resp = serde_json::json!({
                "name": p.name,
                "base_url": p.base_url,
                "format": p.format,
                "models": p.models,
                "key_count": p.keys.len(),
                "keys": keys,
            });
            Json(resp).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub base_url: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub keys: Vec<ApiKeyConfig>,
    #[serde(default)]
    pub models: Vec<String>,
}

fn default_format() -> String {
    "openai".into()
}

/// 请求体:手动从 provider 拉取可用模型列表(配置对话框里的「获取模型」按钮)。
#[derive(Debug, Deserialize)]
pub struct FetchModelsRequest {
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_format")]
    pub format: String,
}

/// 调用 provider 的 /models 接口拉取可用模型 id 列表。
/// OpenAI 格式:GET {base_url}/models;Anthropic 没有标准 models 接口,返回提示。
pub async fn fetch_models(
    State(_state): State<AppState>,
    Json(req): Json<FetchModelsRequest>,
) -> impl IntoResponse {
    let base = req.base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "base_url required" })),
        )
            .into_response();
    }
    // 两种常见布局:base 已含 /v1 → 直接 /models;否则补 /v1/models;都试。
    let url = if base.ends_with("/v1") {
        format!("{base}/models")
    } else {
        format!("{base}/v1/models")
    };

    let client = reqwest::Client::new();
    let mut rb = client.get(&url);
    if !req.api_key.trim().is_empty() {
        rb = rb.bearer_auth(req.api_key.trim());
    }
    match rb.send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            // OpenAI: { "data": [ { "id": "..." }, ... ] }
            let models: Vec<String> = body
                .get("data")
                .and_then(|d| d.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            Json(serde_json::json!({ "models": models })).into_response()
        }
        Ok(resp) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": format!("provider returned {}", resp.status()),
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("fetch failed: {e}") })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProviderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<ApiKeyConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
}

/// Create a new provider
pub async fn create_provider(
    State(state): State<AppState>,
    Json(req): Json<CreateProviderRequest>,
) -> impl IntoResponse {
    // Validate format
    if req.format != "openai" && req.format != "anthropic" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid format. Must be 'openai' or 'anthropic'"
            })),
        )
            .into_response();
    }

    // Validate base_url
    if req.base_url.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "base_url cannot be empty"
            })),
        )
            .into_response();
    }

    let mut config = state.config.write().await;

    // Check if provider already exists
    if config.providers.iter().any(|p| p.name == req.name) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("Provider '{}' already exists", req.name)
            })),
        )
            .into_response();
    }

    // Add provider
    let provider = ProviderConfig {
        name: req.name.clone(),
        base_url: req.base_url,
        format: req.format,
        api: ProviderApi::ChatCompletions,
        keys: req.keys,
        models: req.models,
    };
    config.providers.push(provider);

    // Persist to disk
    if let Err(e) = save_config(&state.config_path, &config) {
        tracing::error!("Failed to save config: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to persist configuration"
            })),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": format!("Provider '{}' created successfully", req.name)
        })),
    )
        .into_response()
}

/// Update an existing provider
pub async fn update_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> impl IntoResponse {
    let mut config = state.config.write().await;

    // Find provider
    let provider = match config.providers.iter_mut().find(|p| p.name == name) {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Provider '{}' not found", name)
                })),
            )
                .into_response()
        }
    };

    // Apply updates
    if let Some(base_url) = req.base_url {
        if base_url.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "base_url cannot be empty"
                })),
            )
                .into_response();
        }
        provider.base_url = base_url;
    }

    if let Some(format) = req.format {
        if format != "openai" && format != "anthropic" {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid format. Must be 'openai' or 'anthropic'"
                })),
            )
                .into_response();
        }
        provider.format = format;
    }

    if let Some(keys) = req.keys {
        provider.keys = keys;
    }

    if let Some(models) = req.models {
        provider.models = models;
    }

    // Persist to disk
    if let Err(e) = save_config(&state.config_path, &config) {
        tracing::error!("Failed to save config: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to persist configuration"
            })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Provider '{}' updated successfully", name)
        })),
    )
        .into_response()
}

/// Delete a provider
pub async fn delete_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut config = state.config.write().await;

    // Check if provider exists
    let index = match config.providers.iter().position(|p| p.name == name) {
        Some(i) => i,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Provider '{}' not found", name)
                })),
            )
                .into_response()
        }
    };

    // Check dependencies
    let mut warnings = Vec::new();
    if config.routing.default_provider.as_ref() == Some(&name) {
        warnings.push("This is the default provider in routing config");
    }
    if config.fallback.priority.contains(&name) {
        warnings.push("This provider is in the fallback priority list");
    }

    // Remove provider
    config.providers.remove(index);

    // Persist to disk
    if let Err(e) = save_config(&state.config_path, &config) {
        tracing::error!("Failed to save config: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to persist configuration"
            })),
        )
            .into_response();
    }

    let mut response = serde_json::json!({
        "message": format!("Provider '{}' deleted successfully", name)
    });

    if !warnings.is_empty() {
        response["warnings"] = serde_json::json!(warnings);
    }

    (StatusCode::OK, Json(response)).into_response()
}

/// 从上游 provider 的 /v1/models 端点拉取实际可用模型列表。
/// 让 web 前端能展示上游中转站的全部模型，而不只是 config 手写的几个。
pub async fn list_upstream_models(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    let Some(provider) = config.providers.iter().find(|p| p.name == name) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "provider not found"}))).into_response();
    };
    let api_key = provider.keys.iter().find(|k| k.enabled && !k.value.trim().is_empty()).map(|k| k.value.trim().to_string());
    let base_url = provider.base_url.trim_end_matches('/').to_string();
    drop(config); // 释放读锁

    let url = format!("{base_url}/v1/models");
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(key) = &api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    match req.timeout(std::time::Duration::from_secs(15)).send().await {
        Ok(resp) => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let models: Vec<String> = body
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|arr| arr.iter().filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                Json(serde_json::json!({"models": models, "count": models.len()})).into_response()
            } else {
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "invalid upstream response"}))).into_response()
            }
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": format!("upstream request failed: {e}")}))).into_response()
        }
    }
}
