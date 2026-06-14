use crate::api::openai_compat::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

pub async fn get_profile(State(state): State<AppState>) -> Json<serde_json::Value> {
    let profile = state.active_profile.read().await;
    Json(serde_json::json!({
        "profile": profile.as_str(),
    }))
}

pub async fn set_profile(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let profile_str = body["profile"].as_str().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Missing 'profile' field"})),
        )
    })?;

    let new_profile = crate::app::AppProfile::from_str_loose(profile_str);
    *state.active_profile.write().await = new_profile;

    Ok(Json(serde_json::json!({
        "profile": new_profile.as_str(),
        "status": "switched",
    })))
}

pub async fn get_policy(State(state): State<AppState>) -> Json<serde_json::Value> {
    let profile = state.active_profile.read().await;
    Json(serde_json::json!({
        "profile": profile.as_str(),
        "rules": state.core_policy.rules,
    }))
}
