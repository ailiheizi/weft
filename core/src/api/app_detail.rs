use crate::api::openai_compat::AppState;
use crate::app::InstanceLock;
use axum::extract::{Path, State};
use axum::http::StatusCode;
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

fn active_lock_for_app(app: &crate::app::ResolvedApp) -> Option<InstanceLock> {
    let lock_path = app.sources.lock_path.as_ref()?;
    crate::app::load_instance_lock_from_path(std::path::Path::new(lock_path)).ok()
}

pub async fn get_app(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let apps = state.resolved_apps.read().await;
    if let Some(app) = apps.get(&name) {
        Ok(Json(
            serde_json::json!({ "app": app, "source_index": derived_source_index(app) }),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("App '{}' not found", name) })),
        ))
    }
}

pub async fn app_health(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let apps = state.resolved_apps.read().await;
    let app = apps.get(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("App '{}' not found", name)})),
        )
    })?;

    let resolved = app.status == crate::app::ResolvedAppStatus::Resolved;
    let has_bindings = !app.bindings.is_empty();
    let no_errors = app.errors.is_empty();

    let store = state.generation_store.read().await;
    let has_active_generation = store.get(&name).and_then(|s| s.active.as_ref()).is_some();

    let healthy = resolved && has_bindings && no_errors;

    Ok(Json(serde_json::json!({
        "app": name,
        "healthy": healthy,
        "resolved": resolved,
        "has_bindings": has_bindings,
        "no_errors": no_errors,
        "has_active_generation": has_active_generation,
        "profile": state.active_profile.read().await.as_str(),
    })))
}

pub async fn app_run(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let apps = state.resolved_apps.read().await;
    let app = apps.get(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("App '{}' not found", name)})),
        )
    })?;

    if app.status != crate::app::ResolvedAppStatus::Resolved {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("App '{}' is not resolved", name),
                "status": app.status,
            })),
        ));
    }

    let capability = body["capability"]
        .as_str()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'capability' field"})),
            )
        })?
        .to_string();

    let active_generation = {
        let store = state.generation_store.read().await;
        store
            .get(&name)
            .and_then(|app_store| app_store.active.clone())
    }
    .ok_or_else(|| {
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("App '{}' has no active generation", name),
                "reason": "active_generation_required",
            })),
        )
    })?;

    let active_binding = active_generation
        .bindings
        .iter()
        .find(|binding| binding.capability == capability)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!(
                        "Active generation for app '{}' has no binding for capability '{}'",
                        name, capability
                    ),
                    "reason": "capability_not_in_active_generation",
                })),
            )
        })?;

    let app_binding = app.bindings.iter().find(|b| b.capability == capability).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("App '{}' has no binding for capability '{}'", name, capability),
            })),
        )
    })?;

    if app_binding.provider != active_binding.provider {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "App '{}' binding for capability '{}' does not match active generation provider '{}'",
                    name, capability, active_binding.provider
                ),
                "resolved_provider": app_binding.provider,
                "active_provider": active_binding.provider,
                "reason": "active_generation_provider_mismatch",
            })),
        ));
    }

    if !active_generation
        .capabilities
        .iter()
        .any(|cap| cap == &capability)
    {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "Active generation for app '{}' does not include capability '{}'",
                    name, capability
                ),
                "reason": "capability_not_active",
            })),
        ));
    }

    let active_lock = active_lock_for_app(app).ok_or_else(|| {
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("App '{}' has no active lock assembly", name),
                "reason": "active_lock_required",
            })),
        )
    })?;

    if !active_lock.bindings.is_empty() {
        let lock_binding = active_lock
            .bindings
            .iter()
            .find(|binding| binding.capability == capability)
            .ok_or_else(|| {
                (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": format!(
                            "Active lock for app '{}' does not include capability '{}'",
                            name, capability
                        ),
                        "reason": "capability_not_locked",
                    })),
                )
            })?;

        if lock_binding.provider != active_binding.provider {
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": format!(
                        "Active lock for app '{}' does not match active generation provider '{}' for capability '{}'",
                        name, active_binding.provider, capability
                    ),
                    "locked_provider": lock_binding.provider,
                    "active_provider": active_binding.provider,
                    "reason": "active_lock_provider_mismatch",
                })),
            ));
        }
    }

    let provider = active_binding.provider.clone();
    drop(apps);

    let action = body["action"].as_str().unwrap_or("call").to_string();
    let data = body["data"].clone();

    let call_payload = serde_json::json!({
        "action": action,
        "data": data,
        "provider": provider,
        "app": name,
    });

    crate::api::capabilities::capability_call(
        Path(capability.clone()),
        State(state),
        Json(call_payload),
    )
    .await
    .map(|Json(result)| {
        Json(serde_json::json!({
            "app": name,
            "capability": capability,
            "result": result,
        }))
    })
}

#[cfg(test)]
mod tests {
    use crate::api::build_router;
    use crate::api::openai_compat::AppState;
    use crate::app::{
        AppBindingResolution, AppGeneration, AppGenerationStore, AppProfile,
        CapabilityProviderRecord, CapabilityRegistry, CapabilityRegistryEntry, CorePolicy,
        GenerationStatus, GenerationStoreMap, PackageIndex, ResolvedApp, ResolvedAppMap,
        ResolvedAppSources, ResolvedAppStatus,
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
    use axum::body::Body;
    use axum::http::{Request, StatusCode as HttpStatusCode};
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::sync::RwLock;
    use tower::util::ServiceExt;

    fn test_state(
        repo_root: std::path::PathBuf,
        apps: ResolvedAppMap,
        capability_registry: CapabilityRegistry,
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
            resolved_apps: Arc::new(RwLock::new(apps)),
            capability_registry: Arc::new(RwLock::new(capability_registry)),
            active_profile: Arc::new(RwLock::new(AppProfile::Developer)),
            core_policy: Arc::new(CorePolicy::default_policy()),
            generation_store: Arc::new(RwLock::new(GenerationStoreMap::new())),
            package_index: Arc::new(PackageIndex::default()),
            repo_root,
            data_dir: std::path::PathBuf::from("data"),
            runtime_token: None,
            runtime_token_path: None,
            chat_providers: Arc::new(RwLock::new(vec![])),
            shutdown_tx: Arc::new(StdMutex::new(None)),
            stream_buffer: Arc::new(StdMutex::new(std::collections::HashMap::new())),
        }
    }

    fn weft_claw_app() -> ResolvedApp {
        let test_lock_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(".tmp-test-locks")
            .join("weft-claw");
        let _ = std::fs::create_dir_all(&test_lock_dir);
        let test_lock_path = test_lock_dir.join("lock.toml");
        let _ = std::fs::write(
            &test_lock_path,
            "lock_version = 2\napp='weft-claw'\ngeneration=1\nstatus='active'\nprofile='developer'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        );

        ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            display_name: "Weft Claw".into(),
            description: "AI 多角色协作开发助手".into(),
            capabilities: vec!["ui.surface".into()],
            enabled_features: vec![],
            bindings: vec![],
            validation_checks: vec![],
            config_path: None,
            status: ResolvedAppStatus::Resolved,
            errors: vec![],
            sources: ResolvedAppSources {
                manifest_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("packages")
                    .join("weft-claw")
                    .join("package.toml")
                    .display()
                    .to_string(),
                config_path: Some(
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("..")
                        .join(".weft")
                        .join("weft-claw")
                        .join("config.toml")
                        .display()
                        .to_string(),
                ),
                lock_path: Some(test_lock_path.display().to_string()),
            },
        }
    }

    fn capability_registry_with_core_execution() -> CapabilityRegistry {
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "core.execution".into(),
            CapabilityRegistryEntry {
                capability: "core.execution".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry
    }

    fn test_binding(capability: &str, provider: &str) -> AppBindingResolution {
        AppBindingResolution {
            capability: capability.into(),
            provider: provider.into(),
            mutable: false,
            source: "test".into(),
        }
    }

    fn test_generation(
        capability: &str,
        provider: &str,
        status: GenerationStatus,
    ) -> AppGeneration {
        AppGeneration {
            id: 1,
            app_name: "weft-claw".into(),
            version: "0.1.0".into(),
            bindings: vec![test_binding(capability, provider)],
            capabilities: vec![capability.into()],
            enabled_features: vec![],
            scene: String::new(),
            profile: "developer".into(),
            binding_set_id: String::new(),
            closure_id: String::new(),
            lock_digest: String::new(),
            lock_path: String::new(),
            parent_generation: None,
            created_by: String::new(),
            status,
            validation_results: vec![],
            created_at: 0,
        }
    }

    fn active_generation_store(capability: &str, provider: &str) -> AppGenerationStore {
        AppGenerationStore {
            active: Some(test_generation(
                capability,
                provider,
                GenerationStatus::Active,
            )),
            candidate: None,
            rollback: None,
            next_id: 2,
        }
    }

    #[tokio::test]
    async fn get_app_returns_ui_metadata() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut apps = ResolvedAppMap::new();
        apps.insert("weft-claw".into(), weft_claw_app());
        let app = build_router(test_state(repo_root, apps, CapabilityRegistry::new()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/apps/weft-claw")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["source_index"]["name"], "weft-claw");
    }

    #[tokio::test]
    async fn app_run_routes_weft_claw_execution_to_core_execution() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut apps = ResolvedAppMap::new();
        let mut weft_claw = weft_claw_app();
        weft_claw.capabilities.push("core.execution".into());
        weft_claw
            .bindings
            .push(test_binding("core.execution", "core"));
        apps.insert("weft-claw".into(), weft_claw);
        let state = test_state(repo_root, apps, capability_registry_with_core_execution());
        {
            let mut store = state.generation_store.write().await;
            store.insert(
                "weft-claw".into(),
                active_generation_store("core.execution", "core"),
            );
        }
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/apps/weft-claw/run")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "capability": "core.execution",
                            "action": "describe",
                            "data": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["app"], "weft-claw");
        assert_eq!(payload["capability"], "core.execution");
        assert_eq!(payload["result"]["provider"], "core");
        assert_eq!(payload["result"]["mode"], "core");
        assert_eq!(payload["result"]["status"], "executed");
        assert_eq!(
            payload["result"]["response"]["capability"],
            "core.execution"
        );
        assert_eq!(payload["result"]["response"]["runtime"], "core");
    }

    #[tokio::test]
    async fn app_run_rejects_when_active_generation_missing() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut apps = ResolvedAppMap::new();
        let mut weft_claw = weft_claw_app();
        weft_claw.capabilities.push("core.execution".into());
        weft_claw
            .bindings
            .push(test_binding("core.execution", "core"));
        apps.insert("weft-claw".into(), weft_claw);
        let app = build_router(test_state(
            repo_root,
            apps,
            capability_registry_with_core_execution(),
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/apps/weft-claw/run")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "capability": "core.execution",
                            "action": "describe",
                            "data": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::CONFLICT);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["reason"], "active_generation_required");
    }

    #[tokio::test]
    async fn app_run_rejects_when_active_generation_binding_differs() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut apps = ResolvedAppMap::new();
        let mut weft_claw = weft_claw_app();
        weft_claw.capabilities.push("core.execution".into());
        weft_claw
            .bindings
            .push(test_binding("core.execution", "core"));
        apps.insert("weft-claw".into(), weft_claw);
        let state = test_state(repo_root, apps, capability_registry_with_core_execution());
        {
            let mut store = state.generation_store.write().await;
            store.insert(
                "weft-claw".into(),
                active_generation_store("core.execution", "other-provider"),
            );
        }
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/apps/weft-claw/run")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "capability": "core.execution",
                            "action": "describe",
                            "data": {}
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::CONFLICT);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["reason"], "active_generation_provider_mismatch");
    }
}
