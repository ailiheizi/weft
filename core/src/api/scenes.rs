use crate::api::openai_compat::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateSceneRequest {
    pub name: String,
    #[serde(flatten)]
    pub scene: crate::app::AppSceneConfig,
}

#[derive(Debug, Deserialize)]
pub struct BindSceneRequest {
    #[serde(default)]
    pub scene: String,
}

fn app_not_found(name: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": format!("App '{}' not found", name),
            "reason": "app_not_found",
            "app": name,
        })),
    )
}

fn scene_not_found(app_name: &str, scene_name: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": format!("Scene '{}' not found for app '{}'", scene_name, app_name),
            "reason": "scene_not_found",
            "app": app_name,
            "scene": scene_name,
        })),
    )
}

fn bad_scene_request(reason: &str, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": message,
            "reason": reason,
        })),
    )
}

fn scene_conflict(app_name: &str, scene_name: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "error": format!("Scene '{}' already exists for app '{}'", scene_name, app_name),
            "reason": "scene_exists",
            "app": app_name,
            "scene": scene_name,
        })),
    )
}

fn scene_write_failed(error: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": error.to_string(),
            "reason": "scene_write_failed",
        })),
    )
}

fn load_instance_config(app: &crate::app::ResolvedApp) -> crate::app::InstanceConfig {
    app.sources
        .config_path
        .as_deref()
        .map(std::path::Path::new)
        .and_then(|path| crate::app::load_instance_config_from_path(path).ok())
        .unwrap_or_default()
}

fn instance_config_path(
    app: &crate::app::ResolvedApp,
) -> Result<std::path::PathBuf, (StatusCode, Json<serde_json::Value>)> {
    app.sources
        .config_path
        .as_deref()
        .or(app.config_path.as_deref())
        .filter(|path| !path.trim().is_empty())
        .map(std::path::PathBuf::from)
        .ok_or_else(|| {
            bad_scene_request(
                "config_path_missing",
                "App has no writable instance config path",
            )
        })
}

fn scene_file_name(scene_name: &str) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let trimmed = scene_name.trim();
    if trimmed.is_empty() {
        return Err(bad_scene_request(
            "scene_name_empty",
            "Scene name is required",
        ));
    }

    if trimmed.contains('/') || trimmed.contains('\\') || trimmed == "." || trimmed == ".." {
        return Err(bad_scene_request(
            "scene_name_invalid",
            "Scene name must be a simple file name",
        ));
    }

    Ok(trimmed.to_string())
}

fn scene_response(
    app_name: String,
    active_scene: String,
    scene: crate::app::AppSceneConfig,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "app": app_name,
        "active_scene": active_scene,
        "scene": scene,
    }))
}

pub async fn list_scenes(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let apps = state.resolved_apps.read().await;
    let app = apps.get(&name).ok_or_else(|| app_not_found(&name))?;
    let config = load_instance_config(app);

    let mut scenes = config
        .scenes
        .into_values()
        .collect::<Vec<crate::app::AppSceneConfig>>();
    scenes.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(Json(serde_json::json!({
        "app": name,
        "active_scene": config.active_scene,
        "scenes": scenes,
    })))
}

pub async fn get_scene(
    Path((name, scene_name)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let apps = state.resolved_apps.read().await;
    let app = apps.get(&name).ok_or_else(|| app_not_found(&name))?;
    let config = load_instance_config(app);
    let scene = config
        .scenes
        .get(&scene_name)
        .cloned()
        .ok_or_else(|| scene_not_found(&name, &scene_name))?;

    Ok(Json(serde_json::json!({
        "app": name,
        "active_scene": config.active_scene,
        "scene": scene,
    })))
}

pub async fn create_scene(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(mut request): Json<CreateSceneRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let apps = state.resolved_apps.read().await;
    let app = apps.get(&name).ok_or_else(|| app_not_found(&name))?;
    let config_path = instance_config_path(app)?;
    let config = load_instance_config(app);
    let scene_name = scene_file_name(&request.name)?;

    if config.scenes.contains_key(&scene_name) {
        return Err(scene_conflict(&name, &scene_name));
    }

    request.scene.name = scene_name.clone();
    if request.scene.schema_version == 0 {
        request.scene.schema_version = 1;
    }

    let scene_path = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("scenes")
        .join(format!("{}.toml", scene_name));
    crate::app::save_scene_config_to_path(&scene_path, &request.scene)
        .map_err(scene_write_failed)?;

    Ok(scene_response(name, config.active_scene, request.scene))
}

pub async fn bind_scene(
    Path((name, scene_name)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(request): Json<BindSceneRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let apps = state.resolved_apps.read().await;
    let app = apps.get(&name).ok_or_else(|| app_not_found(&name))?;
    let config_path = instance_config_path(app)?;
    let mut config = load_instance_config(app);
    let requested_scene = if request.scene.trim().is_empty() {
        scene_name
    } else {
        request.scene
    };
    let scene_name = scene_file_name(&requested_scene)?;
    let scene = config
        .scenes
        .get(&scene_name)
        .cloned()
        .ok_or_else(|| scene_not_found(&name, &scene_name))?;

    config.active_scene = scene_name.clone();
    crate::app::save_instance_config_to_path(&config_path, &config).map_err(scene_write_failed)?;

    Ok(scene_response(name, scene_name, scene))
}

#[cfg(test)]
mod tests {
    use crate::api::build_router;
    use crate::api::openai_compat::AppState;
    use crate::app::{
        AppProfile, CapabilityRegistry, CorePolicy, GenerationStoreMap, PackageIndex,
        PackageSource, ResolvedApp, ResolvedAppMap, ResolvedAppSources, ResolvedAppStatus,
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
    use tempfile::tempdir;
    use tokio::sync::RwLock;
    use tower::util::ServiceExt;

    fn test_state(repo_root: std::path::PathBuf, apps: ResolvedAppMap) -> AppState {
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
            capability_registry: Arc::new(RwLock::new(CapabilityRegistry::new())),
            active_profile: Arc::new(RwLock::new(AppProfile::Developer)),
            core_policy: Arc::new(CorePolicy::default_policy()),
            generation_store: Arc::new(RwLock::new(GenerationStoreMap::new())),
            package_index: Arc::new(PackageIndex {
                version: 1,
                revision: "test-rev".into(),
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
                    provides: vec![],
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

    fn resolved_app(name: &str, config_path: Option<&std::path::Path>) -> ResolvedApp {
        ResolvedApp {
            name: name.into(),
            version: "0.1.0".into(),
            display_name: name.into(),
            description: "test app".into(),
            capabilities: vec![],
            enabled_features: vec![],
            bindings: vec![],
            validation_checks: vec![],
            config_path: config_path.map(|path| path.display().to_string()),
            status: ResolvedAppStatus::Resolved,
            errors: vec![],
            sources: ResolvedAppSources {
                manifest_path: String::new(),
                config_path: config_path.map(|path| path.display().to_string()),
                lock_path: None,
            },
        }
    }

    #[tokio::test]
    async fn list_scenes_returns_merged_scene_files() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        let scenes_dir = instance_dir.join("scenes");
        std::fs::create_dir_all(&scenes_dir).expect("scene dir created");
        std::fs::write(
            instance_dir.join("config.toml"),
            r#"schema_version = 1
active_scene = "team"

[scenes.stable]
name = "stable"
description = "Stable embedded scene"
"#,
        )
        .expect("config written");
        std::fs::write(
            scenes_dir.join("team.toml"),
            r#"schema_version = 1
description = "Team scene from file"
profile = "developer"

[features]
enabled = ["team-mode"]
disabled = ["legacy-mode"]
"#,
        )
        .expect("scene file written");

        let mut apps = ResolvedAppMap::new();
        apps.insert(
            "weft-claw".into(),
            resolved_app("weft-claw", Some(&instance_dir.join("config.toml"))),
        );

        let response = build_router(test_state(repo_root, apps))
            .oneshot(
                Request::builder()
                    .uri("/api/apps/weft-claw/scenes")
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
        assert_eq!(payload["app"], "weft-claw");
        assert_eq!(payload["active_scene"], "team");
        let scenes = payload["scenes"].as_array().expect("scenes array");
        assert_eq!(scenes.len(), 2);
        assert_eq!(scenes[0]["name"], "stable");
        assert_eq!(scenes[1]["name"], "team");
        assert_eq!(scenes[1]["description"], "Team scene from file");
        assert_eq!(scenes[1]["profile"], "developer");
        assert_eq!(
            scenes[1]["features"]["enabled"],
            serde_json::json!(["team-mode"])
        );
        assert_eq!(
            scenes[1]["features"]["disabled"],
            serde_json::json!(["legacy-mode"])
        );
    }

    #[tokio::test]
    async fn get_scene_returns_scene_from_config() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("config.toml"),
            r#"schema_version = 1
active_scene = "team"

[scenes.team]
name = "team"
description = "Team scene"
profile = "developer"
base_generation = 7
"#,
        )
        .expect("config written");

        let mut apps = ResolvedAppMap::new();
        apps.insert(
            "weft-claw".into(),
            resolved_app("weft-claw", Some(&instance_dir.join("config.toml"))),
        );

        let response = build_router(test_state(repo_root, apps))
            .oneshot(
                Request::builder()
                    .uri("/api/apps/weft-claw/scenes/team")
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
        assert_eq!(payload["app"], "weft-claw");
        assert_eq!(payload["active_scene"], "team");
        assert_eq!(payload["scene"]["name"], "team");
        assert_eq!(payload["scene"]["description"], "Team scene");
        assert_eq!(payload["scene"]["base_generation"], 7);
    }

    #[tokio::test]
    async fn get_scene_returns_structured_not_found_for_missing_scene() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(instance_dir.join("config.toml"), "schema_version = 1\n")
            .expect("config written");

        let mut apps = ResolvedAppMap::new();
        apps.insert(
            "weft-claw".into(),
            resolved_app("weft-claw", Some(&instance_dir.join("config.toml"))),
        );

        let response = build_router(test_state(repo_root, apps))
            .oneshot(
                Request::builder()
                    .uri("/api/apps/weft-claw/scenes/missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["reason"], "scene_not_found");
        assert_eq!(payload["app"], "weft-claw");
        assert_eq!(payload["scene"], "missing");
    }

    #[tokio::test]
    async fn create_scene_writes_scene_file() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(instance_dir.join("config.toml"), "schema_version = 1\n")
            .expect("config written");

        let mut apps = ResolvedAppMap::new();
        apps.insert(
            "weft-claw".into(),
            resolved_app("weft-claw", Some(&instance_dir.join("config.toml"))),
        );

        let response = build_router(test_state(repo_root, apps))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/apps/weft-claw/scenes")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"team","description":"Team scene","profile":"developer","features":{"enabled":["team-mode"]}}"#,
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
        assert_eq!(payload["scene"]["name"], "team");
        assert_eq!(payload["scene"]["schema_version"], 1);
        assert_eq!(
            payload["scene"]["features"]["enabled"],
            serde_json::json!(["team-mode"])
        );
        let scene_file = std::fs::read_to_string(instance_dir.join("scenes").join("team.toml"))
            .expect("scene file written");
        assert!(scene_file.contains("name = \"team\""));
        assert!(scene_file.contains("profile = \"developer\""));
    }

    #[tokio::test]
    async fn bind_scene_updates_active_scene_in_config() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        let scenes_dir = instance_dir.join("scenes");
        std::fs::create_dir_all(&scenes_dir).expect("scene dir created");
        std::fs::write(
            instance_dir.join("config.toml"),
            "schema_version = 1\nactive_scene = \"stable\"\n",
        )
        .expect("config written");
        std::fs::write(
            scenes_dir.join("team.toml"),
            "schema_version = 1\nname = \"team\"\ndescription = \"Team scene\"\n",
        )
        .expect("scene file written");

        let mut apps = ResolvedAppMap::new();
        apps.insert(
            "weft-claw".into(),
            resolved_app("weft-claw", Some(&instance_dir.join("config.toml"))),
        );

        let response = build_router(test_state(repo_root, apps))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/apps/weft-claw/scenes/team/bind")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["active_scene"], "team");
        assert_eq!(payload["scene"]["name"], "team");
        let config_file =
            std::fs::read_to_string(instance_dir.join("config.toml")).expect("config file updated");
        assert!(config_file.contains("active_scene = \"team\""));
    }

    #[tokio::test]
    async fn list_scenes_returns_empty_when_config_is_unavailable() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let missing_config = repo_root
            .join(".weft")
            .join("weft-claw")
            .join("config.toml");

        let mut apps = ResolvedAppMap::new();
        apps.insert(
            "weft-claw".into(),
            resolved_app("weft-claw", Some(&missing_config)),
        );

        let response = build_router(test_state(repo_root, apps))
            .oneshot(
                Request::builder()
                    .uri("/api/apps/weft-claw/scenes")
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
        assert_eq!(payload["app"], "weft-claw");
        assert_eq!(payload["scenes"], serde_json::json!([]));
        assert_eq!(payload["active_scene"], "");
    }
}
