use crate::api::openai_compat::AppState;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

/// POST /api/packages/{package_name}/webhook
pub async fn package_webhook_no_channel(
    Path(package_name): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    do_webhook(&package_name, "", state, headers, body).await
}

/// POST /api/packages/{package_name}/webhook/{channel_type}
pub async fn package_webhook_with_channel(
    Path((package_name, channel_type)): Path<(String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    do_webhook(&package_name, &channel_type, state, headers, body).await
}

async fn do_webhook(
    package_name: &str,
    channel_type: &str,
    state: AppState,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if package_name.is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing package name").into_response();
    }

    // Collect headers into a JSON object
    let headers_map: serde_json::Map<String, serde_json::Value> = headers
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                serde_json::Value::String(v.to_str().unwrap_or("").to_string()),
            )
        })
        .collect();

    // Build the payload for the WASM package
    let raw_body = String::from_utf8_lossy(&body).to_string();
    let payload = serde_json::json!({
        "channel": channel_type,
        "headers": headers_map,
        "body": raw_body,
    });

    let payload_str = payload.to_string();

    // Call the WASM package's handle_webhook export
    let wasm_handle = state.wasm_handle.read().await.clone();
    let Some(handle) = wasm_handle else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "No WASM runtime available",
        )
            .into_response();
    };

    match handle.call(package_name, "handle_webhook", &payload_str) {
        Ok(result_str) => build_response_from_plugin(&result_str),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not loaded") || msg.contains("not found") {
                (
                    StatusCode::NOT_FOUND,
                    format!("Package '{}' not found: {}", package_name, msg),
                )
                    .into_response()
            } else {
                tracing::error!(
                    "[webhook] Package '{}' handle_webhook error: {}",
                    package_name,
                    e
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Package error: {}", msg),
                )
                    .into_response()
            }
        }
    }
}

/// Parse the plugin's JSON response and build an HTTP response.
/// Expected format: {"status_code": 200, "body": "..."}
fn build_response_from_plugin(result_str: &str) -> Response {
    let parsed: serde_json::Value = match serde_json::from_str(result_str) {
        Ok(v) => v,
        // If the package returned non-JSON, wrap it as a plain 200 response
        Err(_) => {
            return (StatusCode::OK, result_str.to_string()).into_response();
        }
    };

    let status_code = parsed["status_code"].as_u64().unwrap_or(200) as u16;

    let body = match &parsed["body"] {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    };

    let status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK);
    (status, body).into_response()
}
