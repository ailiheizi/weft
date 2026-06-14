use crate::api::openai_compat::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::json;

fn derived_source_index(app: &crate::app::ResolvedApp) -> serde_json::Value {
    json!({
        "name": app.name,
        "app_version": app.version,
        "manifest_path": app.sources.manifest_path,
        "config_path": app.sources.config_path,
        "lock_path": app.sources.lock_path,
        "trusted": true,
        "signature": "builtin:app-source",
        "source_authority": "product-package-instance",
        "source_public_keys": [],
    })
}

pub async fn list_apps(State(state): State<AppState>) -> Json<serde_json::Value> {
    let apps = state.resolved_apps.read().await;
    let values: Vec<_> = apps
        .values()
        .map(|app| {
            serde_json::json!({
                "name": app.name,
                "version": app.version,
                "display_name": app.display_name,
                "description": app.description,
                "capabilities": app.capabilities,
                "enabled_features": app.enabled_features,
                "bindings": app.bindings,
                "validation_checks": app.validation_checks,
                "config_path": app.config_path,
                "status": app.status,
                "errors": app.errors,
                "sources": app.sources,
                "source_index": derived_source_index(app),
            })
        })
        .collect();
    Json(serde_json::json!({ "apps": values }))
}
