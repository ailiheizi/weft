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
    Json(serde_json::json!({ "providers": providers }))
}

pub async fn get_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    match config.providers.iter().find(|p| p.name == name) {
        Some(p) => {
            let resp = serde_json::json!({
                "name": p.name,
                "base_url": p.base_url,
                "format": p.format,
                "models": p.models,
                "key_count": p.keys.len(),
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
