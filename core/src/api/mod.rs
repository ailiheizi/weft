pub mod app_detail;
pub mod apps;
pub mod capabilities;
pub mod core_capabilities;
pub mod generations;
pub mod health;
pub mod openai_compat;
pub mod package_webhook;
pub mod package_ws;
pub mod packages;
pub mod packages_runtime;
pub mod plans;
pub mod profile;
pub mod providers;
pub mod scenes;
pub mod services;

use axum::extract::{Request, State};
use axum::http::{header::AUTHORIZATION, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use openai_compat::AppState;
use tower_http::cors::{Any, CorsLayer};

fn route_is_unprotected(path: &str) -> bool {
    path == "/health"
        || path == "/api/health"
        || path == "/v1/models"
}

fn matches_bearer_token(value: &HeaderValue, token: &str) -> bool {
    value
        .to_str()
        .ok()
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(|candidate| candidate == token)
        .unwrap_or(false)
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "missing or invalid loopback bearer token"
        })),
    )
        .into_response()
}

async fn require_loopback_token(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if route_is_unprotected(path) {
        return next.run(request).await;
    }

    if path.starts_with("/ws/packages/") {
        let token = state.runtime_token.as_deref().unwrap_or_default();
        let authorized = request
            .uri()
            .query()
            .and_then(|query| {
                query.split('&').find_map(|entry| {
                    let (key, value) = entry.split_once('=')?;
                    if key == "token" { Some(value) } else { None }
                })
            })
            .map(|value| value == token)
            .unwrap_or(false);
        return if authorized {
            next.run(request).await
        } else {
            unauthorized_response()
        };
    }

    // No runtime token configured → loopback auth is not armed; pass through.
    // The token file's presence is what enables auth, so dev/test and
    // not-yet-provisioned deployments keep working.
    let Some(token) = state.runtime_token.as_deref() else {
        return next.run(request).await;
    };

    if let Some(header) = request.headers().get(AUTHORIZATION) {
        if matches_bearer_token(header, token) {
            return next.run(request).await;
        }
    }

    unauthorized_response()
}

/// GET /api/config/registry — get registry config
async fn get_registry_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    let config = state.config.read().await;
    Json(serde_json::json!({
        "gitea_url": config.registry.gitea_url,
        "gitea_token": config.registry.gitea_token.as_ref().map(|t| {
            if t.len() > 4 {
                format!("****{}", &t[t.len()-4..])
            } else {
                "****".to_string()
            }
        })
    }))
}

/// PUT /api/config/registry — update registry config
async fn update_registry_config(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let gitea_url = body["gitea_url"].as_str().map(|s| s.to_string());
    let gitea_token = body["gitea_token"].as_str().map(|s| s.to_string());

    // Update in-memory config
    {
        let mut config = state.config.write().await;
        if let Some(url) = gitea_url {
            config.registry.gitea_url = url;
        }
        if let Some(token) = gitea_token {
            config.registry.gitea_token = if token.is_empty() { None } else { Some(token) };
        }
    }

    // Persist to disk
    {
        let config = state.config.read().await;
        if let Err(e) = crate::config::store::save_config(&state.config_path, &config) {
            tracing::error!("Failed to save config: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to save config: {}", e)})),
            ));
        }
    }

    Ok(Json(
        serde_json::json!({"status": "ok", "message": "Registry config updated"}),
    ))
}

async fn shutdown_core(State(state): State<AppState>) -> Json<serde_json::Value> {
    if let Some(tx) = state.shutdown_tx.lock().unwrap().take() {
        let _ = tx.send(());
    }

    Json(serde_json::json!({
        "status": "ok",
        "message": "shutdown requested",
    }))
}

/// GET /api/stream/tokens?session_id=xxx
/// Returns pending stream tokens for a session and clears them from the buffer.
/// This endpoint is lock-free relative to WASM execution.
async fn stream_tokens(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let session_id = params.get("session_id").cloned().unwrap_or_default();
    if session_id.is_empty() {
        return Json(serde_json::json!({"tokens": [], "error": "missing session_id"}));
    }
    let tokens = state
        .stream_buffer
        .lock()
        .map(|mut buf| buf.remove(&session_id).unwrap_or_default())
        .unwrap_or_default();
    Json(serde_json::json!({"tokens": tokens}))
}

/// GET /api/stream/events?session_id=xxx&after_seq=N
/// Reads session events directly from SQLite, bypassing the WASM lock.
/// This allows real-time polling while send_message is executing.
async fn stream_events(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let session_id = params.get("session_id").cloned().unwrap_or_default();
    if session_id.is_empty() {
        return Json(serde_json::json!({"events": [], "latest_seq": 0, "error": "missing session_id"}));
    }
    let after_seq: i64 = params.get("after_seq").and_then(|v| v.parse().ok()).unwrap_or(0);

    let db_path = std::env::var("WEFT_SESSION_EVENTS_DB")
        .unwrap_or_else(|_| "./data/session-events/session-events.sqlite".to_string());

    let result = tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path)?;
        let mut stmt = conn.prepare(
            "SELECT seq, event_id, event_type, payload_json, created_at \
             FROM session_events WHERE session_id = ?1 AND seq > ?2 \
             ORDER BY seq ASC LIMIT 200",
        )?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map(rusqlite::params![session_id, after_seq], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(seq, event_id, event_type, payload_json, created_at)| {
                let payload = serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null);
                serde_json::json!({
                    "seq": seq,
                    "event_id": event_id,
                    "type": event_type,
                    "payload": payload,
                    "created_at": created_at,
                })
            })
            .collect();

        let latest_seq: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM session_events WHERE session_id = ?1",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .unwrap_or(after_seq);

        Ok::<_, rusqlite::Error>((rows, latest_seq))
    })
    .await;

    match result {
        Ok(Ok((events, latest_seq))) => Json(serde_json::json!({
            "events": events,
            "latest_seq": latest_seq,
        })),
        _ => Json(serde_json::json!({"events": [], "latest_seq": after_seq})),
    }
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // OpenAI-compatible API
        .route(
            "/v1/chat/completions",
            post(openai_compat::chat_completions),
        )
        .route("/v1/models", get(openai_compat::list_models))
        // Management API
        .route("/health", get(health::health))
        .route("/api/health", get(health::health))
        .route("/api/apps", get(apps::list_apps))
        .route("/api/apps/{name}", get(app_detail::get_app))
        .route("/api/apps/{name}/scenes", get(scenes::list_scenes))
        .route("/api/apps/{name}/scenes", post(scenes::create_scene))
        .route("/api/apps/{name}/scenes/{scene}", get(scenes::get_scene))
        .route(
            "/api/apps/{name}/scenes/{scene}/bind",
            post(scenes::bind_scene),
        )
        .route("/api/apps/{name}/health", get(app_detail::app_health))
        .route("/api/apps/{name}/run", post(app_detail::app_run))
        .route("/api/capabilities", get(capabilities::list_capabilities))
        .route(
            "/api/capabilities/{name}",
            get(capabilities::get_capability),
        )
        .route(
            "/api/capabilities/{name}/call",
            post(capabilities::capability_call),
        )
        .route("/api/packages", get(packages::list_packages))
        .route("/api/plans/activate", post(plans::activation_plan))
        .route("/api/profile", get(profile::get_profile))
        .route("/api/profile", axum::routing::put(profile::set_profile))
        .route("/api/policy", get(profile::get_policy))
        .route(
            "/api/apps/{name}/generations",
            get(generations::get_generation),
        )
        .route(
            "/api/apps/{name}/generations/diagnostics",
            get(generations::get_generation_diagnostics),
        )
        .route(
            "/api/apps/{name}/generations/{from}/diff/{to}",
            get(generations::get_generation_diff),
        )
        .route(
            "/api/apps/{name}/propose",
            post(generations::propose_generation),
        )
        .route(
            "/api/apps/{name}/verify",
            post(generations::verify_generation),
        )
        .route(
            "/api/apps/{name}/activate",
            post(generations::activate_generation),
        )
        .route(
            "/api/apps/{name}/generations/{id}/activate",
            post(generations::activate_existing_generation),
        )
        .route(
            "/api/apps/{name}/rollback",
            post(generations::rollback_generation),
        )
        .route("/api/shutdown", post(shutdown_core))
        .route("/api/providers", get(providers::list_providers))
        .route("/api/providers", post(providers::create_provider))
        .route("/api/providers/{name}", get(providers::get_provider))
        .route(
            "/api/providers/{name}",
            axum::routing::put(providers::update_provider),
        )
        .route("/api/providers/{name}", delete(providers::delete_provider))
        .route("/api/services", get(services::list_services))
        .route("/api/services/{name}/start", post(services::start_service))
        .route("/api/services/{name}/stop", post(services::stop_service))
        .route(
            "/api/services/{name}/restart",
            post(services::restart_service),
        )
        .route(
            "/api/services/{name}/webhook",
            post(services::proxy_webhook),
        )
        // Package runtime management
        .route(
            "/api/packages/runtime",
            get(packages_runtime::list_packages),
        )
        .route(
            "/api/packages/install",
            post(packages_runtime::install_package),
        )
        .route(
            "/api/packages/{name}/reload",
            post(packages_runtime::reload_package),
        )
        .route(
            "/api/packages/{name}/toggle",
            post(packages_runtime::toggle_package),
        )
        .route(
            "/api/packages/{name}",
            delete(packages_runtime::uninstall_package),
        )
        .route(
            "/api/packages/{name}/dependencies",
            get(packages_runtime::get_package_dependencies),
        )
        .route(
            "/api/packages/{name}/config/schema",
            get(packages_runtime::get_package_config_schema),
        )
        .route(
            "/api/packages/{name}/config",
            get(packages_runtime::get_package_config),
        )
        .route(
            "/api/packages/{name}/config",
            axum::routing::put(packages_runtime::save_package_config),
        )
        // Package Webhook
        .route(
            "/api/packages/{package_name}/webhook",
            post(package_webhook::package_webhook_no_channel),
        )
        .route(
            "/api/packages/{package_name}/webhook/{channel_type}",
            post(package_webhook::package_webhook_with_channel),
        )
        .route(
            "/api/packages/{package_name}/call",
            post(package_ws::package_call),
        )
        // Package WebSocket
        .route(
            "/ws/packages/{package_name}",
            get(package_ws::package_websocket),
        )
        // Config management
        .route("/api/config/registry", get(get_registry_config))
        .route(
            "/api/config/registry",
            axum::routing::put(update_registry_config),
        )
        .route("/api/stream/tokens", get(stream_tokens))
        .route("/api/stream/events", get(stream_events))
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_loopback_token,
        ))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::build_router;
    use crate::api::openai_compat::AppState;
    use crate::app::{
        AppProfile, CapabilityRegistry, CorePolicy, GenerationStoreMap, PackageIndex,
        PackageSource, ResolvedAppMap,
    };
    use crate::config::{
        AppConfig, CoreConfig, FallbackConfig, KeyStrategyConfig, RegistryConfig, RoutingConfig,
    };
    use crate::defaults::{
        error_handler::DefaultErrorHandler, key_selectors::FailoverSelector, router::DefaultRouter,
    };
    use crate::pipeline::Pipeline;
    use crate::process::ProcessManager;
    use crate::vkeys::VirtualKeyStore;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode as HttpStatusCode};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex as StdMutex};
    use tempfile::TempDir;
    use tokio::sync::RwLock;
    use tower::util::ServiceExt;

    fn test_state(repo_root: std::path::PathBuf) -> AppState {
        test_state_with_profile(repo_root, AppProfile::Developer)
    }

    fn test_state_with_profile(
        repo_root: std::path::PathBuf,
        active_profile: AppProfile,
    ) -> AppState {
        AppState {
            config: Arc::new(RwLock::new(AppConfig {
                core: CoreConfig::default(),
                providers: vec![],
                routing: RoutingConfig::default(),
                key_strategy: KeyStrategyConfig::default(),
                fallback: FallbackConfig::default(),
                virtual_keys: vec![],
                services: vec![],
                packages: vec![],
                registry: RegistryConfig::default(),
                package_aliases: HashMap::new(),
                web_search: Default::default(),
                team: Default::default(),
            })),
            config_path: repo_root.join("config").join("config.toml"),
            pipeline: Arc::new(Pipeline {
                router: Arc::new(DefaultRouter {
                    default_provider: "".into(),
                }),
                key_selector: Arc::new(FailoverSelector),
                transforms: Arc::new(crate::defaults::transforms::TransformRegistry::with_defaults()),
                error_handler: Arc::new(DefaultErrorHandler { max_retries: 0 }),
                http_client: reqwest::Client::new(),
            }),
            process_manager: Arc::new(ProcessManager::new()),
            vkey_store: Arc::new(VirtualKeyStore::new()),
            package_manager: Arc::new(RwLock::new(crate::package::PackageManager::new())),
            wasm_handle: Arc::new(RwLock::new(None)),
            native_handle: Arc::new(RwLock::new(None)),
            resolved_apps: Arc::new(RwLock::new(ResolvedAppMap::new())),
            capability_registry: Arc::new(RwLock::new(CapabilityRegistry::new())),
            active_profile: Arc::new(RwLock::new(active_profile)),
            core_policy: Arc::new(CorePolicy::default_policy()),
            generation_store: Arc::new(RwLock::new(GenerationStoreMap::new())),
            package_index: Arc::new(PackageIndex {
                version: 1,
                revision: "test".into(),
                source_url: "local://packages".into(),
                package_sources: vec![PackageSource {
                    name: "weft-claw-ui".into(),
                    kind: "embedded".into(),
                    package_kind: "provider".into(),
                    runtime_provider: "weft-claw-ui".into(),
                    current_source: "packages/installed/weft-claw".into(),
                    trusted: true,
                    signature: "builtin:test".into(),
                    source_authority: "test".into(),
                    source_public_keys: vec![],
                    provides: vec!["ui.surface".into()],
                    requires: vec![],
                }],
            }),
            repo_root,
            data_dir: std::path::PathBuf::from("data"),
            runtime_token: None,
            runtime_token_path: None,
            chat_providers: Arc::new(RwLock::new(vec![])),
            shutdown_tx: Arc::new(StdMutex::new(None)),
            stream_buffer: Arc::new(StdMutex::new(std::collections::HashMap::new())),
        }
    }

    fn create_package_ui_fixture() -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        let package_dir = dir
            .path()
            .join("packages")
            .join("installed")
            .join("weft-claw")
            .join("ui");
        std::fs::create_dir_all(&package_dir).expect("create package ui dir");
        std::fs::write(
            package_dir.join("index.html"),
            "<html><body>weft claw ui</body></html>",
        )
        .expect("write package ui html");
        std::fs::write(
            dir.path()
                .join("packages")
                .join("installed")
                .join("weft-claw")
                .join("package.toml"),
            "[identity]\nname = \"weft-claw-ui\"\nversion = \"0.1.0\"\ndescription = \"test\"\n\n[package]\nentry = \"ui/index.html\"\nruntime = \"embedded\"\napi_version = \"v1\"\n",
        )
        .expect("write package manifest");
        dir
    }

    #[tokio::test]
    async fn package_ui_route_serves_installed_package_assets_from_repo_root() {
        let fixture = create_package_ui_fixture();
        let app = build_router(test_state(fixture.path().to_path_buf()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/packages/weft-claw-ui/ui/index.html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);
    }

    #[tokio::test]
    async fn activation_plan_route_reports_metadata_without_mutating_runtime_state() {
        let fixture = TempDir::new().expect("temp dir");
        let package_dir = fixture.path().join("materialized-package");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("package.toml"),
            r#"[package_info]
name = "plan-only-package"
version = "0.1.0"
description = "plan only"
entry = "package.wasm"
"#,
        )
        .expect("manifest");
        std::fs::write(package_dir.join("package.wasm"), b"\0asm").expect("entry");

        let state = test_state(fixture.path().to_path_buf());
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "materialized_path": package_dir.display().to_string()
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload["status"],
            serde_json::json!("activation_plan_ready")
        );
        assert_eq!(payload["plan_only"], serde_json::json!(true));
        assert_eq!(payload["metadata_only"], serde_json::json!(true));
        assert_eq!(payload["activation_performed"], serde_json::json!(false));
        assert_eq!(payload["mutation_performed"], serde_json::json!(false));
        assert_eq!(payload["lock_mutation_performed"], serde_json::json!(false));
        assert_eq!(payload["activation_required"], serde_json::json!(true));
        assert_eq!(payload["ready_for_activation"], serde_json::json!(true));
        assert_eq!(
            payload["package"]["name"],
            serde_json::json!("plan-only-package")
        );
        assert!(payload["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .all(|check| { check["ok"].as_bool().unwrap_or(false) }));
        assert!(state.package_manager.read().await.list().is_empty());
    }

    #[tokio::test]
    async fn activation_plan_route_blocks_missing_manifest_without_mutating_runtime_state() {
        let fixture = TempDir::new().expect("temp dir");
        let package_dir = fixture.path().join("materialized-package");
        std::fs::create_dir_all(&package_dir).expect("package dir");

        let state = test_state(fixture.path().to_path_buf());
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "materialized_path": package_dir.display().to_string()
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload["status"],
            serde_json::json!("activation_plan_blocked")
        );
        assert_eq!(payload["plan_only"], serde_json::json!(true));
        assert_eq!(payload["activation_performed"], serde_json::json!(false));
        assert_eq!(payload["mutation_performed"], serde_json::json!(false));
        assert_eq!(payload["manifest_found"], serde_json::json!(false));
        assert_eq!(payload["activation_required"], serde_json::json!(false));
        assert_eq!(payload["ready_for_activation"], serde_json::json!(false));
        assert!(state.package_manager.read().await.list().is_empty());
    }

    #[tokio::test]
    async fn activation_plan_route_apply_true_registers_service_metadata_from_temp_root() {
        let fixture = TempDir::new().expect("temp dir");
        let package_dir = fixture.path().join("materialized-service-package");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("package.toml"),
            r#"[package_info]
name = "controlled-service-package"
version = "0.1.0"
description = "controlled service"
entry = "server.js"
provides = ["chat_channel"]
chat_endpoint = "/chat"

[package]
runtime = "service"
entry = "server.js"
"#,
        )
        .expect("manifest");
        std::fs::write(package_dir.join("server.js"), "console.log('service');\n").expect("entry");

        let state = test_state(fixture.path().to_path_buf());
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "materialized_path": package_dir.display().to_string(),
                            "apply": true,
                            "confirm": true
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload["status"],
            serde_json::json!("activation_metadata_registered")
        );
        assert_eq!(payload["plan_only"], serde_json::json!(false));
        assert_eq!(payload["activation_performed"], serde_json::json!(true));
        assert_eq!(payload["mutation_performed"], serde_json::json!(true));
        assert_eq!(payload["lock_mutation_performed"], serde_json::json!(false));
        assert_eq!(payload["service_registered"], serde_json::json!(true));
        assert_eq!(payload["runtime_started"], serde_json::json!(false));
        assert!(state
            .package_manager
            .read()
            .await
            .get("controlled-service-package")
            .is_some());
        assert_eq!(state.chat_providers.read().await.len(), 1);
    }

    #[tokio::test]
    async fn activation_plan_route_apply_true_requires_confirm_and_does_not_mutate() {
        let fixture = TempDir::new().expect("temp dir");
        let package_dir = fixture.path().join("materialized-service-package");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("package.toml"),
            r#"[package_info]
name = "unconfirmed-plugin"
version = "0.1.0"
description = "unconfirmed"
entry = "server.js"

[package]
runtime = "service"
entry = "server.js"
"#,
        )
        .expect("manifest");
        std::fs::write(package_dir.join("server.js"), "console.log('service');\n").expect("entry");

        let state = test_state(fixture.path().to_path_buf());
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "materialized_path": package_dir.display().to_string(),
                            "apply": true
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload["status"],
            serde_json::json!("activation_apply_blocked")
        );
        assert_eq!(payload["activation_performed"], serde_json::json!(false));
        assert_eq!(payload["mutation_performed"], serde_json::json!(false));
        assert!(payload["validation_issues"]
            .as_array()
            .expect("validation issues")
            .iter()
            .any(|issue| issue
                .as_str()
                .unwrap_or_default()
                .contains("explicit_confirmation_required_for_apply")));
        assert!(state.package_manager.read().await.list().is_empty());
    }

    #[tokio::test]
    async fn activation_plan_route_apply_true_blocks_native_runtime_without_mutating() {
        let fixture = TempDir::new().expect("temp dir");
        let package_dir = fixture.path().join("materialized-native-package");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("package.toml"),
            r#"[package_info]
name = "native-plugin"
version = "0.1.0"
description = "native"
entry = "native.dll"

[package]
runtime = "native"
entry = "native.dll"
"#,
        )
        .expect("manifest");

        let state = test_state(fixture.path().to_path_buf());
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "materialized_path": package_dir.display().to_string(),
                            "apply": true,
                            "confirm": true
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload["status"],
            serde_json::json!("activation_apply_blocked")
        );
        assert_eq!(payload["activation_performed"], serde_json::json!(false));
        assert!(payload["validation_issues"]
            .as_array()
            .expect("validation issues")
            .iter()
            .any(|issue| issue
                .as_str()
                .unwrap_or_default()
                .contains("runtime_safe_for_controlled_activation")));
        assert!(state.package_manager.read().await.list().is_empty());
    }

    #[tokio::test]
    async fn activation_plan_route_developer_blocks_trusted_native_plan() {
        let fixture = TempDir::new().expect("temp dir");
        let package_dir = fixture.path().join("developer-native-package");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("package.toml"),
            r#"[package_info]
name = "developer-native-plugin"
version = "0.1.0"
description = "native"
entry = "native.dll"

[package]
runtime = "native"
entry = "native.dll"
native_allowed = true
expected_digest = "sha256:0123456789abcdef"
"#,
        )
        .expect("manifest");
        std::fs::write(package_dir.join("native.dll"), b"fake native library")
            .expect("native file");

        let state = test_state_with_profile(fixture.path().to_path_buf(), AppProfile::Developer);
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "materialized_path": package_dir.display().to_string(),
                            "confirm": true
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload["status"],
            serde_json::json!("activation_plan_blocked")
        );
        assert_eq!(
            payload["ready_for_trusted_native_load"],
            serde_json::json!(false)
        );
        assert_eq!(payload["native_load_performed"], serde_json::json!(false));
        assert!(payload["validation_issues"]
            .as_array()
            .expect("validation issues")
            .iter()
            .any(|issue| issue
                .as_str()
                .unwrap_or_default()
                .contains("trusted_native_profile_required")));
        assert!(state.package_manager.read().await.list().is_empty());
        assert!(state.native_handle.read().await.is_none());
    }

    #[tokio::test]
    async fn activation_plan_route_trusted_native_missing_digest_blocks_plan() {
        let fixture = TempDir::new().expect("temp dir");
        let package_dir = fixture.path().join("trusted-native-missing-digest-package");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("package.toml"),
            r#"[package_info]
name = "trusted-native-missing-digest-plugin"
version = "0.1.0"
description = "native"
entry = "native.dll"

[package]
runtime = "native"
entry = "native.dll"
native_allowed = true
"#,
        )
        .expect("manifest");
        std::fs::write(package_dir.join("native.dll"), b"fake native library")
            .expect("native file");

        let state = test_state_with_profile(fixture.path().to_path_buf(), AppProfile::Trusted);
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "materialized_path": package_dir.display().to_string(),
                            "confirm": true
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload["status"],
            serde_json::json!("activation_plan_blocked")
        );
        assert_eq!(
            payload["ready_for_trusted_native_load"],
            serde_json::json!(false)
        );
        assert!(payload["validation_issues"]
            .as_array()
            .expect("validation issues")
            .iter()
            .any(|issue| issue
                .as_str()
                .unwrap_or_default()
                .contains("trusted_native_expected_digest_present")));
        assert!(state.native_handle.read().await.is_none());
    }

    #[tokio::test]
    async fn activation_plan_route_trusted_native_ready_returns_plan_only() {
        let fixture = TempDir::new().expect("temp dir");
        let package_dir = fixture.path().join("trusted-native-ready-package");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("package.toml"),
            r#"[package_info]
name = "trusted-native-ready-plugin"
version = "0.1.0"
description = "native"
entry = "native.dll"

[package]
runtime = "native"
entry = "native.dll"
native_allowed = true
expected_digest = "sha256:0123456789abcdef"
"#,
        )
        .expect("manifest");
        std::fs::write(package_dir.join("native.dll"), b"fake native library")
            .expect("native file");

        let state = test_state_with_profile(fixture.path().to_path_buf(), AppProfile::Trusted);
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/plans/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "materialized_path": package_dir.display().to_string(),
                            "confirm": true
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(
            payload["status"],
            serde_json::json!("ready_for_trusted_native_load")
        );
        assert_eq!(payload["plan_only"], serde_json::json!(true));
        assert_eq!(payload["activation_performed"], serde_json::json!(false));
        assert_eq!(payload["native_load_performed"], serde_json::json!(false));
        assert_eq!(
            payload["ready_for_trusted_native_load"],
            serde_json::json!(true)
        );
        assert!(payload["validation_issues"].as_array().unwrap().is_empty());
        assert!(payload["trusted_native"]["library_candidate"]
            .as_str()
            .unwrap_or_default()
            .ends_with("native.dll"));
        assert!(state.package_manager.read().await.list().is_empty());
        assert!(state.native_handle.read().await.is_none());
    }
}
