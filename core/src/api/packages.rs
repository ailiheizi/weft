use crate::api::openai_compat::AppState;
use axum::extract::State;
use axum::Json;

pub async fn list_packages(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": state.package_index.version,
        "packages": state.package_index.package_sources,
    }))
}
