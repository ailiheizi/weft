use crate::api::openai_compat::AppState;
use crate::package::bridge::{resolve_loaded_package_name_from_aliases, WasmHandle};
use crate::package::resolve_runtime_package;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;

/// GET /ws/plugins/{package_name} -> WebSocket upgrade
pub async fn package_websocket(
    ws: WebSocketUpgrade,
    Path(package_name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let token = state.runtime_token.as_deref().unwrap_or_default();
    let authorized = params
        .get("token")
        .map(|value| value == token)
        .unwrap_or(false);
    if !authorized {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "missing or invalid loopback bearer token"
            })),
        )
            .into_response();
    }

    ws.on_upgrade(move |socket| handle_plugin_ws(socket, package_name, state))
}

pub async fn package_call(
    Path(package_name): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    Json(dispatch_package_payload(&package_name, payload, &state).await)
}

/// Reusable WASM payload dispatcher for plugins.
/// This wrapper allows other modules to reuse the existing dispatch logic
/// without duplicating behavior.
pub async fn dispatch_to_wasm_package_payload(
    package_name: &str,
    payload: serde_json::Value,
    state: &AppState,
) -> serde_json::Value {
    let wasm_handle = state.wasm_handle.read().await.clone();
    let canonical_package_name =
        resolve_runtime_package(&state.repo_root, &state.package_index, package_name)
            .map(|package| package.manifest.package_info.name)
            .filter(|resolved| !resolved.trim().is_empty());

    let target_package_name = if let Some(handle) = wasm_handle.as_ref() {
        let package_aliases = {
            let config = state.config.read().await;
            config.package_aliases.clone()
        };
        let mut merged_aliases =
            crate::package::merged_package_aliases(&state.package_index, &package_aliases);
        if let Some(canonical) = canonical_package_name.as_ref() {
            if canonical != package_name {
                merged_aliases.insert(package_name.trim().to_string(), canonical.clone());
            }
        }
        let loaded_package_names = handle.package_names();
        Some(resolve_loaded_package_name_from_aliases(
            &merged_aliases,
            &loaded_package_names,
            canonical_package_name.as_deref().unwrap_or(package_name),
        ))
    } else {
        canonical_package_name.clone()
    };

    dispatch_to_plugin(
        package_name,
        target_package_name.as_deref(),
        &payload,
        &wasm_handle,
    )
    .await
}

async fn handle_plugin_ws(mut socket: WebSocket, package_name: String, state: AppState) {
    tracing::info!("[ws] Package '{}' UI connected", package_name);

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                let ws_msg: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(m) => m,
                    Err(e) => {
                        let err = serde_json::json!({
                            "id": "error",
                            "type": "error",
                            "payload": {"message": format!("Invalid message: {}", e)}
                        });
                        let _ = socket.send(Message::Text(err.to_string().into())).await;
                        continue;
                    }
                };

                let message_id = ws_msg
                    .get("id")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!(""));
                let response_payload = dispatch_package_payload(
                    &package_name,
                    ws_msg
                        .get("payload")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({})),
                    &state,
                )
                .await;

                let response = serde_json::json!({
                    "id": message_id,
                    "type": "response",
                    "payload": response_payload
                });
                let _ = socket
                    .send(Message::Text(response.to_string().into()))
                    .await;
            }
            Message::Close(_) => {
                tracing::info!("[ws] Package '{}' UI disconnected", package_name);
                break;
            }
            _ => {}
        }
    }
}

pub(crate) async fn dispatch_package_payload(
    package_name: &str,
    payload: serde_json::Value,
    state: &AppState,
) -> serde_json::Value {
    if let Some(service_payload) = dispatch_to_service_package(package_name, &payload, state).await
    {
        return service_payload;
    }

    dispatch_to_wasm_package_payload(package_name, payload.clone(), state).await
}

async fn dispatch_to_service_package(
    package_name: &str,
    payload: &serde_json::Value,
    state: &AppState,
) -> Option<serde_json::Value> {
    let resolved_package_name =
        resolve_runtime_package(&state.repo_root, &state.package_index, package_name)
            .map(|package| package.manifest.package_info.name)
            .unwrap_or_else(|| package_name.to_string());

    let service_config = state
        .process_manager
        .service_config(&resolved_package_name)
        .await?;
    let Some(health_url) = service_config.health_url.clone() else {
        return Some(
            serde_json::json!({"error": format!("Service package '{}' has no health url", resolved_package_name)}),
        );
    };

    let base_url = health_url.trim_end_matches("/health");
    let request_url = service_plugin_request_url(&resolved_package_name, base_url);
    let client = reqwest::Client::new();

    let request_body = service_plugin_request_body(&resolved_package_name, payload);

    match client.post(&request_url).json(&request_body).send().await {
        Ok(response) => {
            let status = response.status();
            match response.json::<serde_json::Value>().await {
                Ok(body) => {
                    if status.is_success() {
                        Some(body)
                    } else {
                        Some(serde_json::json!({
                            "error": format!("service package '{}' returned HTTP {}", resolved_package_name, status),
                            "details": body,
                        }))
                    }
                }
                Err(error) => Some(serde_json::json!({
                    "error": format!("service package '{}' returned invalid JSON: {}", resolved_package_name, error),
                })),
            }
        }
        Err(error) => Some(serde_json::json!({
            "error": format!("service package '{}' request failed: {}", resolved_package_name, error),
        })),
    }
}

fn service_plugin_request_url(package_name: &str, base_url: &str) -> String {
    match package_name {
        "js-extension-runtime" => format!("{}/execute", base_url),
        _ => format!("{}/webhook", base_url),
    }
}

fn service_plugin_request_body(
    package_name: &str,
    payload: &serde_json::Value,
) -> serde_json::Value {
    if package_name == "js-extension-runtime" {
        let action = payload
            .get("action")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let data = payload
            .get("data")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let tool = match action {
            "web_search" | "search_web" | "websearch" => "web_search",
            "fetch_url" | "web_fetch" => "fetch_url",
            _ => action,
        };
        let js_payload = if data.get("tool").is_some() || data.get("args").is_some() {
            data
        } else {
            serde_json::json!({
                "tool": tool,
                "args": data,
            })
        };
        return serde_json::json!({
            "id": "weft-tool-executor",
            "action": "execute_tool",
            "payload": js_payload,
        });
    }
    payload.clone()
}
async fn dispatch_to_plugin(
    requested_package_name: &str,
    resolved_package_name: Option<&str>,
    payload: &serde_json::Value,
    wasm_handle: &Option<WasmHandle>,
) -> serde_json::Value {
    let Some(handle) = wasm_handle else {
        return serde_json::json!({"error": "No WASM runtime available"});
    };

    let target_package_name = resolved_package_name
        .filter(|name| handle.has_package(name))
        .unwrap_or(requested_package_name);

    if !handle.has_package(target_package_name) {
        return serde_json::json!({"error": format!("Package '{}' not loaded", target_package_name)});
    }

    let payload_str = serde_json::to_string(payload).unwrap_or_default();

    let package_name = target_package_name.to_string();
    let package_name_for_call = package_name.clone();
    let handle = handle.clone();
    let call_result = tokio::task::spawn_blocking(move || {
        handle.call(&package_name_for_call, "handle_ws_message", &payload_str)
    })
    .await;

    match call_result {
        Ok(Ok(result_str)) => serde_json::from_str(&result_str)
            .unwrap_or_else(|_| serde_json::json!({"result": result_str})),
        Ok(Err(e)) => {
            tracing::error!(
                "[ws] Package '{}' handle_ws_message error: {}",
                package_name,
                e
            );
            serde_json::json!({"error": format!("{}", e)})
        }
        Err(e) => {
            tracing::error!(
                "[ws] Package '{}' join error while handling package call: {}",
                package_name,
                e
            );
            serde_json::json!({"error": format!("package worker join error: {}", e)})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{service_plugin_request_body, service_plugin_request_url};

    #[test]
    fn service_package_mapping_keeps_memory_runtime_on_webhook_passthrough() {
        assert_eq!(
            service_plugin_request_url("memory-runtime", "http://127.0.0.1:17830"),
            "http://127.0.0.1:17830/webhook"
        );

        let payload = serde_json::json!({
            "action": "write",
            "data": {
                "key": "session",
                "value": "hello"
            }
        });

        assert_eq!(
            service_plugin_request_body("memory-runtime", &payload),
            payload
        );
    }

    #[test]
    fn generic_service_http_mapping_uses_webhook_and_passthrough_body() {
        let payload = serde_json::json!({"hello": "world"});

        assert_eq!(
            service_plugin_request_url("companion-core", "http://127.0.0.1:17830"),
            "http://127.0.0.1:17830/webhook"
        );
        assert_eq!(
            service_plugin_request_body("companion-core", &payload),
            payload
        );
    }
}
