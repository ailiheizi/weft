use crate::api::openai_compat::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

fn service_request_url(name: &str, health_url: &str) -> String {
    let base_url = health_url.trim_end_matches("/health");
    let _ = name;
    format!("{}/webhook", base_url)
}

fn service_request_body(name: &str, body: &serde_json::Value) -> serde_json::Value {
    let _ = name;
    body.clone()
}

pub async fn list_services(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.process_manager.run_health_checks().await;
    let statuses = state.process_manager.all_statuses().await;
    let service_names: Vec<String> = statuses.keys().cloned().collect();

    let services: Vec<serde_json::Value> = service_names
        .iter()
        .filter_map(|name| {
            let config = state.process_manager.service_config_sync(name)?;
            let status = statuses
                .get(name)
                .map(|st| st.to_string())
                .unwrap_or_else(|| "unknown".into());
            Some(serde_json::json!({
                "name": config.name,
                "command": config.command,
                "auto_start": config.auto_start,
                "status": status,
            }))
        })
        .collect();

    Json(serde_json::json!({ "services": services }))
}

/// Proxy a webhook message to a managed weft-claw service.
/// POST /api/services/{name}/webhook  body: {"message": "..."}
pub async fn proxy_webhook(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let health_url = match state
        .process_manager
        .service_config_sync(&name)
        .and_then(|svc| svc.health_url)
    {
        Some(u) => u.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("Service '{}' not found or has no health_url", name) })),
            )
                .into_response();
        }
    };

    let request_url = service_request_url(&name, &health_url);
    let request_body = service_request_body(&name, &body);

    // Forward the request
    let client = reqwest::Client::new();
    match client
        .post(&request_url)
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(300))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(text) => {
                    let axum_status = StatusCode::from_u16(status.as_u16())
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                    (axum_status, text).into_response()
                }
                Err(e) => (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({ "error": format!("Failed to read response: {}", e) })),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("Failed to proxy webhook: {}", e) })),
        )
            .into_response(),
    }
}

pub async fn start_service(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.process_manager.start(&name).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn stop_service(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.process_manager.stop(&name).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn restart_service(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.process_manager.restart(&name).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
