use crate::config::AppConfig;
use crate::pipeline::Pipeline;
use crate::process::ProcessManager;
use crate::types::{ApiError, ApiErrorBody, ChatRequest};
use crate::vkeys::VirtualKeyStore;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::StreamExt;
use std::convert::Infallible;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::RwLock;

pub type SharedConfig = Arc<RwLock<AppConfig>>;
pub type SharedPipeline = Arc<Pipeline>;

#[derive(Clone)]
pub struct AppState {
    pub config: SharedConfig,
    pub config_path: std::path::PathBuf,
    pub pipeline: SharedPipeline,
    pub process_manager: Arc<ProcessManager>,
    pub vkey_store: Arc<VirtualKeyStore>,
    pub package_manager: Arc<RwLock<crate::package::PackageManager>>,
    pub wasm_handle: Arc<RwLock<Option<crate::package::bridge::WasmHandle>>>,
    pub native_handle: Arc<RwLock<Option<crate::package::NativeHandle>>>,
    pub resolved_apps: Arc<RwLock<crate::app::ResolvedAppMap>>,
    pub capability_registry: Arc<RwLock<crate::app::CapabilityRegistry>>,
    pub active_profile: Arc<RwLock<crate::app::AppProfile>>,
    pub core_policy: Arc<crate::app::CorePolicy>,
    pub generation_store: Arc<RwLock<crate::app::GenerationStoreMap>>,
    pub package_index: Arc<crate::app::PackageIndex>,
    pub repo_root: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
    pub runtime_token: Option<String>,
    pub runtime_token_path: Option<std::path::PathBuf>,
    pub chat_providers: Arc<RwLock<Vec<ChatProviderInfo>>>,
    pub shutdown_tx: Arc<StdMutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    /// Native stream buffer: session_id -> pending token chunks.
    /// Written by host_chat_completion_stream, read by /api/stream/tokens.
    pub stream_buffer: Arc<StdMutex<std::collections::HashMap<String, Vec<String>>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatProviderInfo {
    pub name: String,
    pub endpoint: String,
    pub description: String,
}

pub async fn chat_completions(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Response {
    if request.stream {
        return chat_completions_stream(state, request).await;
    }

    let config = state.config.read().await;
    match state.pipeline.execute(&request, &config).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => {
            let body = ApiError {
                error: ApiErrorBody {
                    message: e.to_string(),
                    error_type: "proxy_error".into(),
                    code: Some("bad_gateway".into()),
                },
            };
            (StatusCode::BAD_GATEWAY, Json(body)).into_response()
        }
    }
}

async fn chat_completions_stream(state: AppState, request: ChatRequest) -> Response {
    let config = state.config.read().await.clone();

    let (provider_name, resp) = match state.pipeline.execute_stream(&request, &config).await {
        Ok(r) => r,
        Err(e) => {
            let body = ApiError {
                error: ApiErrorBody {
                    message: e.to_string(),
                    error_type: "proxy_error".into(),
                    code: Some("bad_gateway".into()),
                },
            };
            return (StatusCode::BAD_GATEWAY, Json(body)).into_response();
        }
    };

    let provider = config
        .providers
        .iter()
        .find(|p| p.name == provider_name)
        .cloned();

    let transforms = state.pipeline.transforms.clone();

    let stream = async_stream::stream! {
        let mut byte_stream = resp.bytes_stream();
        // 字节级缓冲：SSE chunk 按字节到达，一个多字节 UTF-8 字符（如中文 3 字节）
        // 可能跨两个 chunk。必须在字节层累积、按 \n 切行，只对完整行解码，
        // 否则每个 chunk 单独 from_utf8_lossy 会把跨边界的半个字符变成 �（乱码）。
        let mut buffer: Vec<u8> = Vec::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Stream read error: {}", e);
                    break;
                }
            };

            buffer.extend_from_slice(&chunk);

            while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                // 去掉行尾的 \n（及可能的 \r）后按完整字节序列解码。
                let line = String::from_utf8_lossy(&line_bytes).to_string();

                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                if let Some(ref prov) = provider {
                    match transforms.for_format(&prov.format).transform_stream_chunk(&line, prov).await {
                        Ok(Some(chunk)) => {
                            let data = serde_json::to_string(&chunk).unwrap_or_default();
                            yield Ok::<_, Infallible>(Event::default().data(data));
                        }
                        Ok(None) => {
                            if line.contains("[DONE]") {
                                yield Ok(Event::default().data("[DONE]"));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Stream chunk transform error: {}", e);
                        }
                    }
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

pub async fn list_models(State(state): State<AppState>) -> Json<serde_json::Value> {
    let config = state.config.read().await;
    let models: Vec<serde_json::Value> = config
        .providers
        .iter()
        .flat_map(|p| {
            p.models.iter().map(move |m| {
                serde_json::json!({
                    "id": m,
                    "object": "model",
                    "owned_by": p.name,
                })
            })
        })
        .collect();

    Json(serde_json::json!({
        "object": "list",
        "data": models,
    }))
}
