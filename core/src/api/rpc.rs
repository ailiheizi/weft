//! 统一 RPC 端点(/rpc) + 内部 dispatch(供 FFI 直接调用不走网络)。

use axum::body::Body;
use axum::extract::State;
use axum::http::{Method, Request};
use axum::response::IntoResponse;
use axum::routing::Router;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tower::ServiceExt;

use crate::api::openai_compat::AppState;

/// 全局 Router 引用(build_router 后设一次,/rpc 和 FFI rpc_call 共用)。
static ROUTER: OnceLock<Router> = OnceLock::new();

/// 在 core 启动时调用(build_router 后),注册全局 router 供内部 dispatch 用。
pub fn set_global_router(router: Router) {
    let _ = ROUTER.set(router);
}

/// 进程内就绪检查:ROUTER 已注册即表示 FFI dispatch 可用(与 HTTP listener 无关)。
/// FFI start_core 用它判断就绪,取代依赖 HTTP health 的脆弱探测。
pub fn router_ready() -> bool {
    ROUTER.get().is_some()
}

#[derive(Debug, Deserialize)]
pub struct RequestEnvelope {
    pub id: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub body: serde_json::Value,
}

fn default_method() -> String {
    "CALL".into()
}

#[derive(Debug, Serialize)]
pub struct ResponseEnvelope {
    pub id: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: serde_json::Value,
}

/// POST /rpc — 统一 RPC 入口(内部 dispatch,不走网络回环)。
pub async fn rpc_endpoint(
    State(state): State<AppState>,
    Json(envelope): Json<RequestEnvelope>,
) -> impl IntoResponse {
    let token = state.runtime_token.clone().unwrap_or_default();
    Json(dispatch_internal(envelope, &token).await)
}

/// 内部 dispatch:直接调 router.oneshot(不走网络)。
/// 供 /rpc endpoint 和 FFI rpc_call 共用。
pub async fn dispatch_internal(envelope: RequestEnvelope, token: &str) -> ResponseEnvelope {
    let router = match ROUTER.get() {
        Some(r) => r.clone(),
        None => {
            return ResponseEnvelope {
                id: envelope.id,
                status: 503,
                headers: None,
                body: serde_json::json!({ "error": "router not initialized" }),
            };
        }
    };

    let method = match envelope.method.to_uppercase().as_str() {
        "QUERY" | "GET" => Method::GET,
        "DELETE" => Method::DELETE,
        "PUT" => Method::PUT,
        _ => Method::POST,
    };

    let path = if envelope.path.starts_with('/') {
        envelope.path.clone()
    } else {
        format!("/{}", envelope.path)
    };

    let body_bytes = serde_json::to_vec(&envelope.body).unwrap_or_default();

    let mut req_builder = Request::builder()
        .method(method)
        .uri(&path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token));

    for (k, v) in &envelope.headers {
        if k.to_lowercase() != "authorization" {
            req_builder = req_builder.header(k.as_str(), v.as_str());
        }
    }

    let internal_req = match req_builder.body(Body::from(body_bytes)) {
        Ok(r) => r,
        Err(e) => {
            return ResponseEnvelope {
                id: envelope.id,
                status: 400,
                headers: None,
                body: serde_json::json!({ "error": format!("bad request: {e}") }),
            };
        }
    };

    match router.oneshot(internal_req).await {
        Ok(response) => {
            let status = response.status().as_u16();
            let body_bytes = axum::body::to_bytes(response.into_body(), 10 * 1024 * 1024)
                .await
                .unwrap_or_default();
            let body: serde_json::Value =
                serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
            ResponseEnvelope {
                id: envelope.id,
                status,
                headers: None,
                body,
            }
        }
        Err(e) => ResponseEnvelope {
            id: envelope.id,
            status: 500,
            headers: None,
            body: serde_json::json!({ "error": format!("dispatch error: {e}") }),
        },
    }
}
