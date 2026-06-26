use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use http_body_util::BodyExt;
use weft_core::api::build_router;
use weft_core::api::openai_compat::AppState;
use weft_core::config::*;
use weft_core::defaults::*;
use weft_core::package::{discover_runtime_packages, PackageInfo, PackageManager};
use weft_core::pipeline::Pipeline;
use weft_core::process::ProcessManager;
use weft_core::vkeys::VirtualKeyStore;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tower::ServiceExt;

fn test_package_digest(repo_root: &std::path::Path, source: &str) -> String {
    use sha2::{Digest, Sha512};

    let path = repo_root.join(source);
    let mut hasher = Sha512::new();

    fn update_dir(hasher: &mut Sha512, path: &std::path::Path) -> Result<(), std::io::Error> {
        let mut entries = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            let name = entry.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(name, "target" | ".git" | "node_modules" | ".sisyphus") {
                continue;
            }
            if entry.is_dir() {
                hasher.update(entry.to_string_lossy().as_bytes());
                update_dir(hasher, &entry)?;
            } else if entry.is_file() {
                hasher.update(entry.to_string_lossy().as_bytes());
                hasher.update(std::fs::read(&entry)?);
            }
        }
        Ok(())
    }

    if path.is_file() {
        match std::fs::read(&path) {
            Ok(bytes) => hasher.update(bytes),
            Err(_) => hasher.update(format!("missing:{}", source).as_bytes()),
        }
    } else if path.is_dir() {
        if update_dir(&mut hasher, &path).is_err() {
            hasher.update(format!("unreadable:{}", source).as_bytes());
        }
    } else {
        hasher.update(format!("missing:{}", source).as_bytes());
    }

    format!("{:x}", hasher.finalize())
}

fn test_state() -> AppState {
    let config = AppConfig {
        core: CoreConfig::default(),
        providers: vec![ProviderConfig {
            name: "mock".into(),
            base_url: "http://localhost:19999".into(),
            format: "openai".into(),
            api: ProviderApi::ChatCompletions,
            keys: vec![ApiKeyConfig {
                value: "sk-mock".into(),
                label: None,
                enabled: true,
            }],
            models: vec!["test-model".into()],
        }],
        routing: RoutingConfig {
            default_provider: Some("mock".into()),
            default_model: Some("test-model".into()),
            ..Default::default()
        },
        key_strategy: KeyStrategyConfig::default(),
        fallback: FallbackConfig {
            retry_count: 0,
            switch_key: false,
            switch_provider: false,
            priority: vec![],
        },
        virtual_keys: vec![],
        services: vec![],
        packages: vec![],
        registry: RegistryConfig::default(),
        package_aliases: Default::default(),
        web_search: Default::default(),
        team: Default::default(),
    };

    let pipeline = Pipeline {
        router: Arc::new(DefaultRouter {
            default_provider: "mock".into(),
        }),
        key_selector: Arc::new(FailoverSelector),
        transforms: Arc::new(weft_core::defaults::transforms::TransformRegistry::with_defaults()),
        error_handler: Arc::new(DefaultErrorHandler { max_retries: 0 }),
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .connect_timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap(),
    };

    AppState {
        config: Arc::new(RwLock::new(config)),
        config_path: PathBuf::from("./config/test-config.toml"),
        pipeline: Arc::new(pipeline),
        process_manager: Arc::new(ProcessManager::new()),
        vkey_store: Arc::new(VirtualKeyStore::new()),
        package_manager: Arc::new(RwLock::new(PackageManager::new())),
        wasm_handle: Arc::new(RwLock::new(None)),
        native_handle: Arc::new(RwLock::new(None)),
        resolved_apps: Arc::new(RwLock::new(Default::default())),
        capability_registry: Arc::new(RwLock::new(Default::default())),
        active_profile: Arc::new(RwLock::new(weft_core::app::AppProfile::Developer)),
        core_policy: Arc::new(weft_core::app::CorePolicy::default_policy()),
        generation_store: Arc::new(RwLock::new(Default::default())),
        package_index: Arc::new(weft_core::app::PackageIndex::default()),
        repo_root: PathBuf::from("D:/windows/code/project/WEFT-plug"),
        data_dir: PathBuf::from("D:/windows/code/project/WEFT-plug/data"),
        runtime_token: None,
        runtime_token_path: None,
        chat_providers: Arc::new(RwLock::new(vec![])),
        shutdown_tx: Arc::new(Mutex::new(None)),
        stream_buffer: Arc::new(Mutex::new(std::collections::HashMap::new())),
    }
}

fn test_binding(capability: &str, provider: &str) -> weft_core::app::AppBindingResolution {
    weft_core::app::AppBindingResolution {
        capability: capability.into(),
        provider: provider.into(),
        mutable: false,
        source: "test".into(),
    }
}

fn test_generation(
    app_name: &str,
    capability: &str,
    provider: &str,
    status: weft_core::app::GenerationStatus,
) -> weft_core::app::AppGeneration {
    weft_core::app::AppGeneration {
        id: 1,
        app_name: app_name.into(),
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

fn active_generation_store(
    app_name: &str,
    capability: &str,
    provider: &str,
) -> weft_core::app::AppGenerationStore {
    weft_core::app::AppGenerationStore {
        active: Some(test_generation(
            app_name,
            capability,
            provider,
            weft_core::app::GenerationStatus::Active,
        )),
        candidate: None,
        rollback: None,
        next_id: 2,
    }
}

#[tokio::test]
async fn test_fetch_package_index_from_url_parses_json() {
    use axum::{routing::get, Router};
    let app = Router::new().route(
        "/packages",
        get(|| async {
            axum::Json(serde_json::json!({
                "version": 1,
                "source_url": "http://127.0.0.1:4011/packages",
                "package_sources": []
            }))
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:4011")
        .await
        .unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let index = weft_core::app::fetch_package_index_from_url("http://127.0.0.1:4011/packages")
        .await
        .unwrap();
    assert_eq!(index.version, 1);
    assert_eq!(index.source_url, "http://127.0.0.1:4011/packages");
}

#[tokio::test]
async fn test_activation_plan_endpoint_returns_frontend_metadata_shape() {
    let temp = tempfile::tempdir().unwrap();
    let package_dir = temp.path().join("shape-package");
    std::fs::create_dir(&package_dir).unwrap();
    std::fs::write(
        package_dir.join("package.toml"),
        r#"
[package_info]
name = "shape-package"
version = "0.1.0"
description = "shape test"
entry = "package.wasm"
provides = ["shape.capability"]
"#,
    )
    .unwrap();
    std::fs::write(package_dir.join("package.wasm"), b"wasm placeholder").unwrap();

    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/activate")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "materialized_path": package_dir,
                        "apply": false,
                        "confirm": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["status"], "activation_plan_ready");
    assert_eq!(json["plan_only"], true);
    assert_eq!(json["metadata_only"], true);
    assert_eq!(json["activation_performed"], false);
    assert_eq!(json["mutation_performed"], false);
    assert_eq!(json["lock_mutation_performed"], false);
    assert_eq!(json["activation_required"], true);
    assert_eq!(json["ready_for_activation"], true);
    assert_eq!(json["manifest_found"], true);
    assert_eq!(json["package"]["name"], "shape-package");
    assert!(json["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["name"] == "runtime_entry_available"));
    assert!(json["requirements"].is_object());
    assert!(json["validation_issues"].as_array().unwrap().is_empty());
    assert!(json.get("activation_plan").is_none());
}

#[tokio::test]
async fn test_activation_plan_endpoint_can_start_explicit_service_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let package_dir = temp.path().join("startable-service");
    std::fs::create_dir(&package_dir).unwrap();

    #[cfg(windows)]
    let entry_name = "service.cmd";
    #[cfg(not(windows))]
    let entry_name = "service.sh";

    std::fs::write(
        package_dir.join("package.toml"),
        format!(
            r#"
[package_info]
name = "startable-service"
version = "0.1.0"
description = "startable service test"
entry = "{entry_name}"
provides = ["test.service"]

[package]
runtime = "service"
entry = "{entry_name}"

[runtime_contract]
startup_mode = "on_demand"
restart_policy = "never"
"#
        ),
    )
    .unwrap();

    #[cfg(windows)]
    std::fs::write(package_dir.join(entry_name), "@echo off\r\nexit /b 0\r\n").unwrap();
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let entry_path = package_dir.join(entry_name);
        std::fs::write(&entry_path, "#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = std::fs::metadata(&entry_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&entry_path, perms).unwrap();
    }

    let state = test_state();
    let process_manager = state.process_manager.clone();
    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/activate")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "materialized_path": package_dir,
                        "apply": true,
                        "confirm": true,
                        "start_service": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["status"], "activation_metadata_registered");
    assert_eq!(json["service_registered"], true);
    assert_eq!(json["service_start_requested"], true);
    assert_eq!(json["service_auto_start"], false);
    assert_eq!(json["runtime_started"], true);
    assert_eq!(json["service_start_error"], serde_json::Value::Null);
    assert_eq!(json["native_load_performed"], false);
    assert_eq!(json["lock_mutation_performed"], false);
    assert!(process_manager.status("startable-service").await.is_some());
}

#[tokio::test]
async fn test_activation_plan_endpoint_does_not_start_auto_start_service_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let package_dir = temp.path().join("auto-start-service");
    std::fs::create_dir(&package_dir).unwrap();

    #[cfg(windows)]
    let entry_name = "service.cmd";
    #[cfg(not(windows))]
    let entry_name = "service.sh";

    std::fs::write(
        package_dir.join("package.toml"),
        format!(
            r#"
[package_info]
name = "auto-start-service"
version = "0.1.0"
description = "auto-start service test"
entry = "{entry_name}"
provides = ["test.service"]

[package]
runtime = "service"
entry = "{entry_name}"

[runtime_contract]
startup_mode = "persistent"
restart_policy = "never"
"#
        ),
    )
    .unwrap();

    #[cfg(windows)]
    std::fs::write(package_dir.join(entry_name), "@echo off\r\nexit /b 0\r\n").unwrap();
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let entry_path = package_dir.join(entry_name);
        std::fs::write(&entry_path, "#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = std::fs::metadata(&entry_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&entry_path, perms).unwrap();
    }

    let state = test_state();
    let process_manager = state.process_manager.clone();
    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plans/activate")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "materialized_path": package_dir,
                        "apply": true,
                        "confirm": true,
                        "start_service": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["status"], "activation_metadata_registered");
    assert_eq!(json["service_registered"], true);
    assert_eq!(json["service_start_requested"], true);
    assert_eq!(json["service_auto_start"], true);
    assert_eq!(json["runtime_started"], false);
    assert!(json["service_start_error"]
        .as_str()
        .unwrap()
        .contains("will not duplicate implicit auto-start"));
    assert_eq!(json["native_load_performed"], false);
    assert_eq!(json["lock_mutation_performed"], false);
    assert_eq!(
        process_manager
            .status("auto-start-service")
            .await
            .unwrap()
            .to_string(),
        "stopped"
    );
}

#[tokio::test]
async fn test_provider_missing_from_index_returns_reason() {
    let state = test_state();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "ghost.capability".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ghost.capability".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "ghost-provider".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/ghost.capability/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"call","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let json = response_json(resp).await;
    assert_eq!(json["reason"], "provider_missing_from_index");
}

#[tokio::test]
async fn test_app_state_starts_with_empty_resolved_apps() {
    let state = test_state();
    let apps = state.resolved_apps.read().await;
    assert!(apps.is_empty());
}

#[tokio::test]
async fn test_app_state_starts_with_empty_capability_registry() {
    let state = test_state();
    let registry = state.capability_registry.read().await;
    assert!(registry.is_empty());
}

#[tokio::test]
async fn test_list_capabilities_returns_registry_entries() {
    let state = test_state();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "agent.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "memory.store".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "memory.store".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "memory-store".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.skills".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.skills".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "skills-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.mcp".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.mcp".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "mcp-client".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "channel.bridge".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "channel.bridge".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "channel-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "memory.store".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "memory.store".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "memory-store".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.skills".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.skills".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "skills-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.mcp".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.mcp".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "mcp-client".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "channel.bridge".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "channel.bridge".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "channel-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "core.execution".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.execution".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/capabilities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let listed = json["capabilities"].as_array().unwrap();
    assert!(listed
        .iter()
        .any(|item| item["capability"] == "agent.runtime"));
    assert!(listed.iter().any(|item| item["capability"] == "ext.mcp"));
}

#[test]
fn test_resolve_app_binds_declared_provider_when_available() {
    use weft_core::app::config::AppManifest;
    use weft_core::app::resolve::resolve_app_manifest;
    use weft_core::package::{DiscoveredPackage, PackageRuntime};

    let manifest: AppManifest = toml::from_str(
        r#"
[app]
name = "weft-claw"
version = "0.1.0"
display_name = "Weft Claw"
description = "test"

[requires]
capabilities = ["agent.runtime"]

[bindings.agent.runtime]
provider = "agent-runtime"
mutable = false
"#,
    )
    .unwrap();

    let packages = vec![DiscoveredPackage {
        manifest: toml::from_str(
            r#"
[package_info]
name = "agent-runtime"
version = "0.1.0"
description = "agent"
entry = "package.wasm"
provides = []

[capability]
provides = ["agent.runtime"]
"#,
        )
        .unwrap(),
        dir: std::path::PathBuf::from("./agent-runtime"),
        entry_path: None,
        runtime: PackageRuntime::Wasm,
    }];

    let resolved = resolve_app_manifest(&manifest, &packages).unwrap();
    assert_eq!(resolved.name, "weft-claw");
    assert_eq!(resolved.bindings.len(), 1);
    assert_eq!(resolved.bindings[0].provider, "agent-runtime");
}

#[test]
fn test_load_product_package_reads_weft_claw_manifest() {
    use weft_core::app::config::load_product_package_declaration;

    let product_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("packages")
        .join("weft-claw");

    let manifest = load_product_package_declaration(&product_dir).unwrap();
    assert_eq!(manifest.app.name, "weft-claw");
    assert!(manifest
        .requires
        .capabilities
        .contains(&"agent.runtime".to_string()));
    assert!(manifest.features.default_enabled.is_empty());
}

#[test]
fn weft_code_product_manifest_binds_runtime_service() {
    use weft_core::app::config::load_product_package_declaration;

    let product_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("packages")
        .join("weft-code");

    let manifest = load_product_package_declaration(&product_dir).unwrap();

    assert_eq!(manifest.app.name, "weft-code");
    assert!(manifest
        .requires
        .capabilities
        .iter()
        .any(|value| value == "weft_code.runtime"));
    assert!(!manifest
        .requires
        .capabilities
        .iter()
        .any(|value| value == "ui.surface"));

    let bindings = manifest.flattened_bindings();
    let runtime_binding = bindings
        .get("weft_code.runtime")
        .unwrap_or_else(|| panic!("missing weft_code.runtime binding in weft-code manifest"));
    assert_eq!(runtime_binding.provider, "weft-code-runtime");
    assert!(!runtime_binding.mutable);
    assert!(!bindings.contains_key("ui.surface"));
}

#[test]
fn weft_code_package_index_registers_wasm_provider_and_product_source() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let index = weft_core::app::load_package_index(&repo_root.join("packages")).unwrap();

    let runtime = index
        .get("weft-code-runtime")
        .unwrap_or_else(|| panic!("missing weft-code-runtime package source"));
    assert_eq!(runtime.kind, "wasm");
    assert_eq!(runtime.package_kind, "provider");
    assert_eq!(runtime.runtime_provider, "weft-code-runtime");
    assert_eq!(
        runtime.current_source,
        "packages/installed/weft-code-runtime"
    );
    assert!(runtime
        .provides
        .iter()
        .any(|value| value == "weft_code.runtime"));

    let product = index
        .get("weft-code")
        .unwrap_or_else(|| panic!("missing weft-code product source"));
    assert_eq!(product.package_kind, "product");
    assert_eq!(product.current_source, "packages/weft-code");
    assert_eq!(product.runtime_provider, "weft-code-runtime");
    assert!(!product.provides.iter().any(|value| value == "ui.surface"));
}

#[tokio::test]
async fn test_test_state_keeps_resolved_apps_empty_until_startup_loader_runs() {
    let state = test_state();
    let apps = state.resolved_apps.read().await;
    assert_eq!(apps.len(), 0);
}

#[tokio::test]
async fn test_list_apps_returns_resolved_apps_from_state() {
    let state = test_state();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "AI 多角色协作开发助手".into(),
                capabilities: vec!["agent.capability".into()],
                enabled_features: vec![],
                bindings: vec![],
                validation_checks: vec!["boot".into()],
                config_path: Some(".weft/weft-claw/config.toml".into()),
                status: weft_core::app::state::ResolvedAppStatus::Resolved,
                errors: vec![],
                sources: weft_core::app::state::ResolvedAppSources::default(),
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/apps")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["apps"][0]["name"], "weft-claw");
    assert_eq!(json["apps"][0]["status"], "Resolved");
    assert_eq!(json["apps"][0]["source_index"]["name"], "weft-claw");
    assert_eq!(
        json["apps"][0]["source_index"]["source_authority"],
        "product-package-instance"
    );
}

#[tokio::test]
async fn test_list_apps_includes_source_index_when_available() {
    let state = test_state_with_package_index();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "AI 多角色协作开发助手".into(),
                capabilities: vec!["agent.capability".into()],
                enabled_features: vec![],
                bindings: vec![],
                validation_checks: vec!["boot".into()],
                config_path: Some(".weft/weft-claw/config.toml".into()),
                status: weft_core::app::state::ResolvedAppStatus::Resolved,
                errors: vec![],
                sources: weft_core::app::state::ResolvedAppSources::default(),
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/apps")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["apps"][0]["source_index"]["name"], "weft-claw");
    assert_eq!(
        json["apps"][0]["source_index"]["source_authority"],
        "product-package-instance"
    );
}

#[test]
fn test_weft_claw_product_manifest_references_existing_package_names() {
    let product_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("packages")
        .join("weft-claw");

    let manifest = weft_core::app::config::load_product_package_declaration(&product_dir).unwrap();
    let bindings = manifest.flattened_bindings();

    assert_eq!(bindings["agent.runtime"].provider, "agent-runtime");
    assert_eq!(bindings["ext.skills"].provider, "skills-runtime");
    assert_eq!(bindings["ext.mcp"].provider, "mcp-client");
    assert_eq!(bindings["memory.store"].provider, "memory-store");
    assert_eq!(bindings["channel.bridge"].provider, "channel-core");
}

#[test]
fn test_load_instance_config_reads_weft_claw_runtime_settings() {
    use weft_core::app::config::load_instance_config;

    let instance_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".weft")
        .join("weft-claw");

    let config = load_instance_config(&instance_dir).unwrap();
    assert_eq!(config.app_runtime.profile, "developer");
    assert_eq!(config.app_runtime.default_model, "deepseek-chat");
    assert!(config.features.enabled.is_empty());
    assert!(config
        .packages
        .enabled
        .contains(&"agent-runtime".to_string()));
}

#[test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
fn test_load_instance_lock_reads_weft_claw_lock_metadata() {
    use weft_core::app::config::load_instance_lock;

    let instance_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".weft")
        .join("weft-claw");

    let lock = load_instance_lock(&instance_dir).unwrap();
    assert_eq!(lock.app, "weft-claw");
    assert_eq!(lock.lock_version, 2);
    assert_eq!(lock.inputs.declaration_schema_version, 2);
}

#[test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
fn test_weft_claw_lock_has_active_status() {
    use weft_core::app::config::load_instance_lock;

    let instance_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".weft")
        .join("weft-claw");

    let lock = load_instance_lock(&instance_dir).unwrap();
    assert_eq!(lock.status, "active");
    assert_eq!(lock.profile, "developer");
    assert!(!lock.packages.is_empty());
    assert!(!lock.bindings.is_empty());
    assert!(lock.packages.iter().any(|pkg| pkg.name == "agent-runtime"));
    assert!(lock.packages.iter().all(|pkg| !pkg.sha512.is_empty()));
    assert!(!lock.features.is_empty());
    assert!(!lock.binding_sources.is_empty());
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn test_activate_generation_creates_lock_from_manifest_config_and_bindings() {
    use weft_core::app::config::load_instance_lock;
    use tempfile::tempdir;

    let state = test_state_with_package_index();
    let temp_root = tempdir().unwrap();
    let package_dir = temp_root.path().join("packages").join("weft-claw");
    let instance_dir = temp_root.path().join(".weft").join("weft-claw");
    std::fs::create_dir_all(&package_dir).unwrap();
    std::fs::create_dir_all(&instance_dir).unwrap();
    std::fs::copy(
        state
            .repo_root
            .join("packages")
            .join("weft-claw")
            .join("package.toml"),
        package_dir.join("package.toml"),
    )
    .unwrap();
    std::fs::copy(
        state
            .repo_root
            .join(".weft")
            .join("weft-claw")
            .join("config.toml"),
        instance_dir.join("config.toml"),
    )
    .unwrap();
    let lock_path = instance_dir.join("lock.toml");
    if lock_path.exists() {
        std::fs::remove_file(&lock_path).unwrap();
    }

    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec![
                    "agent.runtime".into(),
                    "core.execution".into(),
                    "prompt.system".into(),
                    "workflow.orchestration".into(),
                    "tool.runtime".into(),
                    "tool.shell".into(),
                    "tool.files".into(),
                    "tool.web".into(),
                    "tool.git".into(),
                    "memory.store".into(),
                    "ext.skills".into(),
                    "ext.mcp".into(),
                    "channel.bridge".into(),
                    "team.runtime".into(),
                    "team.taskboard".into(),
                    "team.handoff".into(),
                    "team.role.catalog".into(),
                    "team.delegate".into(),
                    "team.context.shared".into(),
                    "workflow.template.devteam".into(),
                    "ui.surface".into(),
                ],
                enabled_features: vec![],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "prompt.system".into(),
                        provider: "prompt-system".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.orchestration".into(),
                        provider: "workflow-orchestrator".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.runtime".into(),
                        provider: "tool-runtime-core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.shell".into(),
                        provider: "tool-shell".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.files".into(),
                        provider: "tool-files".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.web".into(),
                        provider: "tool-web".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.git".into(),
                        provider: "tool-git".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "memory.store".into(),
                        provider: "memory-store".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.skills".into(),
                        provider: "skills-runtime".into(),
                        mutable: true,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.mcp".into(),
                        provider: "mcp-client".into(),
                        mutable: true,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "channel.bridge".into(),
                        provider: "channel-core".into(),
                        mutable: true,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.taskboard".into(),
                        provider: "team-task-board".into(),
                        mutable: true,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.handoff".into(),
                        provider: "team-task-board".into(),
                        mutable: true,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.role.catalog".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.delegate".into(),
                        provider: "agent-runtime".into(),
                        mutable: true,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.context.shared".into(),
                        provider: "team-runtime".into(),
                        mutable: true,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.template.devteam".into(),
                        provider: "workflow-template-devteam".into(),
                        mutable: false,
                        source: "config-override".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                ],
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: package_dir.join("package.toml").display().to_string(),
                    config_path: Some(instance_dir.join("config.toml").display().to_string()),
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        for (capability, provider, runtime) in [
            ("agent.runtime", "agent-runtime", "wasm"),
            ("core.execution", "core", "core"),
            ("prompt.system", "prompt-system", "wasm"),
            ("workflow.orchestration", "workflow-orchestrator", "wasm"),
            ("tool.runtime", "tool-runtime-core", "wasm"),
            ("tool.shell", "tool-shell", "wasm"),
            ("tool.files", "tool-files", "wasm"),
            ("tool.web", "tool-web", "wasm"),
            ("tool.git", "tool-git", "wasm"),
            ("memory.store", "memory-store", "wasm"),
            ("ext.skills", "skills-runtime", "wasm"),
            ("ext.mcp", "mcp-client", "wasm"),
            ("channel.bridge", "channel-core", "wasm"),
            ("team.runtime", "team-runtime", "wasm"),
            ("team.taskboard", "team-task-board", "wasm"),
            ("team.handoff", "team-task-board", "wasm"),
            ("team.role.catalog", "team-runtime", "wasm"),
            ("team.delegate", "agent-runtime", "wasm"),
            ("team.context.shared", "team-runtime", "wasm"),
            (
                "workflow.template.devteam",
                "workflow-template-devteam",
                "wasm",
            ),
            ("ui.surface", "weft-claw", "wasm"),
        ] {
            registry.insert(
                capability.into(),
                weft_core::app::CapabilityRegistryEntry {
                    capability: capability.into(),
                    providers: vec![weft_core::app::CapabilityProviderRecord {
                        provider: provider.into(),
                        runtime: runtime.into(),
                        priority: 0,
                    }],
                    bindings: vec![],
                },
            );
        }
    }

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let verify_json = response_json(resp).await;
    if status != StatusCode::OK {
        panic!("propose failed: {}", verify_json);
    }

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/verify")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let json = response_json(resp).await;
    if status != StatusCode::OK {
        panic!("verify failed: {}", json);
    }

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/activate")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["status"], "activated");
    assert_eq!(json["lock_written"], true);

    let lock = load_instance_lock(&instance_dir).unwrap();
    assert_eq!(lock.app, "weft-claw");
    assert_eq!(lock.status, "active");
    assert!(lock.packages.iter().any(|pkg| pkg.name == "prompt-system"));
    assert!(lock.packages.iter().any(|pkg| pkg.name == "skills-runtime"));
    assert!(lock
        .packages
        .iter()
        .any(|pkg| pkg.name == "workflow-orchestrator"));
    assert!(lock.packages.iter().any(|pkg| pkg.name == "tool-shell"));
    assert!(lock
        .assembly
        .selected_packages
        .iter()
        .any(|pkg| pkg == "prompt-system"));
    assert!(lock.assembly.enabled_features.is_empty());
    assert!(lock.binding_sources.iter().any(|binding| {
        binding.capability == "ext.skills" && binding.source == "config-override"
    }));
    assert!(lock.binding_sources.iter().any(|binding| {
        binding.capability == "workflow.orchestration" && binding.package == "workflow-orchestrator"
    }));
    assert!(lock.bindings.iter().any(|binding| {
        binding.capability == "core.execution"
            && binding.provider == "core"
            && binding.package == "core"
    }));
    assert!(lock
        .binding_sources
        .iter()
        .any(|binding| { binding.capability == "core.execution" && binding.package == "core" }));
}

#[test]
fn test_resolve_app_reports_missing_provider_as_error_state() {
    use weft_core::app::config::AppManifest;
    use weft_core::app::resolve::resolve_app_manifest;

    let manifest: AppManifest = toml::from_str(
        r#"
[app]
name = "broken-app"
version = "0.1.0"
display_name = "Broken App"
description = "test"

[requires]
capabilities = ["agent.capability"]

[bindings.agent.capability]
provider = "missing-provider"
mutable = false
"#,
    )
    .unwrap();

    let error = resolve_app_manifest(&manifest, &[])
        .unwrap_err()
        .to_string();
    assert!(error.contains("missing-provider"));
}

#[test]
fn test_resolved_app_status_defaults_to_resolved() {
    let app = weft_core::app::ResolvedApp::default();
    assert_eq!(
        app.status,
        weft_core::app::state::ResolvedAppStatus::Resolved
    );
}

#[test]
fn test_build_capability_registry_collects_providers_and_bindings() {
    use weft_core::app::{
        build_capability_registry, AppBindingResolution, ResolvedApp, ResolvedAppMap,
    };
    use weft_core::package::{DiscoveredPackage, PackageRuntime};
    use std::collections::HashMap;

    let packages = vec![DiscoveredPackage {
        manifest: toml::from_str(
            r#"
[package_info]
name = "agent-core"
version = "0.1.0"
description = "agent"
entry = "package.wasm"
provides = []

[capability]
provides = ["agent.capability"]
"#,
        )
        .unwrap(),
        dir: std::path::PathBuf::from("./agent-core"),
        entry_path: None,
        runtime: PackageRuntime::Wasm,
    }];

    let mut apps: ResolvedAppMap = HashMap::new();
    apps.insert(
        "weft-claw".into(),
        ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            display_name: "Weft Claw".into(),
            description: "test".into(),
            capabilities: vec!["agent.capability".into()],
            enabled_features: vec![],
            bindings: vec![AppBindingResolution {
                capability: "agent.capability".into(),
                provider: "agent-core".into(),
                mutable: false,
                source: "test".into(),
            }],
            validation_checks: vec![],
            config_path: None,
            status: weft_core::app::state::ResolvedAppStatus::Resolved,
            errors: vec![],
            sources: weft_core::app::state::ResolvedAppSources::default(),
        },
    );

    let registry = build_capability_registry(&packages, &apps);
    let entry = registry.get("agent.capability").unwrap();
    assert_eq!(entry.providers.len(), 1);
    assert_eq!(entry.providers[0].provider, "agent-core");
    assert_eq!(entry.bindings.len(), 1);
    assert_eq!(entry.bindings[0].app, "weft-claw");
}

#[tokio::test]
async fn test_get_single_app_returns_not_found_for_missing_app() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/apps/missing-app")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_single_capability_returns_not_found_for_missing_capability() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/capabilities/missing-capability")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_capability_call_returns_bad_gateway_when_wasm_runtime_missing() {
    let state = test_state_with_package_index();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "agent.capability".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "agent.capability".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![weft_core::app::CapabilityBindingRecord {
                    app: "weft-claw".into(),
                    provider: "agent-core".into(),
                }],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/agent.capability/call")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"input":"hello"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Without an active WASM runtime, dispatch should fail with BAD_GATEWAY
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let json = response_json(resp).await;
    assert!(json.get("error").is_some() || json.get("status").is_some());
}

#[tokio::test]
async fn test_capability_call_prefers_binding_provider_when_present() {
    let state = test_state_with_package_index();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "memory.backend".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "memory.backend".into(),
                providers: vec![
                    weft_core::app::CapabilityProviderRecord {
                        provider: "memory".into(),
                        runtime: "wasm".into(),
                        priority: 0,
                    },
                    weft_core::app::CapabilityProviderRecord {
                        provider: "other-memory".into(),
                        runtime: "wasm".into(),
                        priority: 0,
                    },
                ],
                bindings: vec![weft_core::app::CapabilityBindingRecord {
                    app: "weft-claw".into(),
                    provider: "memory".into(),
                }],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/memory.backend/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "action": "recall",
                        "data": {"agent":"tester","query":"hello","limit":1}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let json = response_json(resp).await;
    assert!(json.get("error").is_some() || json.get("status").is_some());
}

#[tokio::test]
async fn test_capability_call_returns_not_found_for_unknown_capability() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/unknown.capability/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"noop","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let json = response_json(resp).await;
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("unknown.capability"));
}

#[tokio::test]
async fn test_capability_call_returns_selected_provider_in_response_wrapper() {
    let state = test_state_with_package_index();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "tool.registry".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.registry".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "skills".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![weft_core::app::CapabilityBindingRecord {
                    app: "weft-claw".into(),
                    provider: "skills".into(),
                }],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/tool.registry/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "action": "list_available",
                        "data": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Without a live WASM runtime the dispatch returns BAD_GATEWAY,
    // but the provider selection path was still exercised.
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let json = response_json(resp).await;
    assert!(json.get("error").is_some());
}

fn test_state_with_chat_providers(
    chat_providers: Vec<weft_core::api::openai_compat::ChatProviderInfo>,
) -> AppState {
    let mut state = test_state();
    state.chat_providers = Arc::new(RwLock::new(chat_providers));
    state
}

fn test_state_with_package_index() -> AppState {
    let mut state = test_state();
    state.package_index = Arc::new(weft_core::app::PackageIndex {
        version: 1,
        revision: "test-rev".into(),
        source_url: "local://packages".into(),
        package_sources: vec![
            weft_core::app::PackageSource {
                name: "agent-runtime".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "agent-core".into(),
                current_source: "packages/official/agent-core".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec![],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "skills-runtime".into(),
                kind: "wasm".into(),
                package_kind: "provider".into(),
                runtime_provider: "skills".into(),
                current_source: "packages/official/skills".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec![],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "mcp-client".into(),
                kind: "wasm".into(),
                package_kind: "provider".into(),
                runtime_provider: "mcp-client".into(),
                current_source: "packages/official/mcp-client".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec![],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "memory-store".into(),
                kind: "wasm".into(),
                package_kind: "provider".into(),
                runtime_provider: "memory".into(),
                current_source: "packages/official/memory".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec![],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "weft-claw-ui".into(),
                kind: "metadata".into(),
                package_kind: "provider".into(),
                runtime_provider: "weft-claw".into(),
                current_source: "packages/weft-claw-ui".into(),
                trusted: false,
                signature: "builtin:ui".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec![],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "native-host".into(),
                kind: "native".into(),
                runtime_provider: "native-host".into(),
                current_source: "packages/installed/native-echo".into(),
                trusted: true,
                signature: "builtin:native-example".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                package_kind: String::new(),
                provides: vec![],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "channel-core".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "channels".into(),
                current_source: "packages/official/channels".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec![],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "prompt-system".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "prompt-system".into(),
                current_source: "packages/official/prompt-system".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["prompt.system".into()],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "workflow-orchestrator".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "workflow-orchestrator".into(),
                current_source: "packages/official/workflow-orchestrator".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["workflow.orchestration".into()],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "tool-runtime-core".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "tool-runtime-core".into(),
                current_source: "packages/official/tool-runtime-core".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["tool.runtime".into()],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "tool-shell".into(),
                kind: "wasm".into(),
                package_kind: "provider".into(),
                runtime_provider: "tool-shell".into(),
                current_source: "packages/official/tool-shell".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["tool.shell".into()],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "tool-files".into(),
                kind: "wasm".into(),
                package_kind: "provider".into(),
                runtime_provider: "tool-files".into(),
                current_source: "packages/official/tool-files".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["tool.files".into()],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "tool-web".into(),
                kind: "wasm".into(),
                package_kind: "provider".into(),
                runtime_provider: "tool-web".into(),
                current_source: "packages/official/tool-web".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["tool.web".into()],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "tool-git".into(),
                kind: "wasm".into(),
                package_kind: "provider".into(),
                runtime_provider: "tool-git".into(),
                current_source: "packages/official/tool-git".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["tool.git".into()],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "team-runtime".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "team-runtime".into(),
                current_source: "packages/official/team-runtime".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec![
                    "team.runtime".into(),
                    "team.role.catalog".into(),
                    "team.delegate".into(),
                    "team.context.shared".into(),
                ],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "team-task-board".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "team-task-board".into(),
                current_source: "packages/official/team-task-board".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["team.taskboard".into(), "team.handoff".into()],
                requires: vec![],
            },
            weft_core::app::PackageSource {
                name: "workflow-template-devteam".into(),
                kind: "wasm".into(),
                package_kind: "provider".into(),
                runtime_provider: "workflow-template-devteam".into(),
                current_source: "packages/official/workflow-template-devteam".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["workflow.template.devteam".into()],
                requires: vec![],
            },
        ],
    });
    state
}

fn weft_claw_sources(repo_root: &std::path::Path) -> weft_core::app::ResolvedAppSources {
    weft_core::app::ResolvedAppSources {
        manifest_path: repo_root
            .join("packages")
            .join("weft-claw")
            .join("package.toml")
            .display()
            .to_string(),
        config_path: Some(
            repo_root
                .join(".weft")
                .join("weft-claw")
                .join("config.toml")
                .display()
                .to_string(),
        ),
        lock_path: Some(
            repo_root
                .join(".weft")
                .join("weft-claw")
                .join("lock.toml")
                .display()
                .to_string(),
        ),
    }
}

fn weft_code_runtime_package_source() -> weft_core::app::PackageSource {
    weft_core::app::PackageSource {
        name: "weft-code-runtime".into(),
        kind: "wasm".into(),
        package_kind: "provider".into(),
        runtime_provider: "weft-code-runtime".into(),
        current_source: "packages/installed/weft-code-runtime".into(),
        trusted: true,
        signature: "builtin:installed".into(),
        source_authority: "installed".into(),
        source_public_keys: vec![],
        provides: vec!["weft_code.runtime".into()],
        requires: vec![],
    }
}

fn test_state_with_weft_code_runtime_bridge() -> AppState {
    use weft_core::package::bridge::{
        PackageLoadInfo, WasmHandle, WasmHostState, WasmPackageHost, WasmStartupMode,
    };
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;

    let mut state = test_state_with_package_index();
    let runtime_source = weft_code_runtime_package_source();
    let runtime_dir = state.repo_root.join(&runtime_source.current_source);
    let wasm_path = runtime_dir.join("package.wasm");

    let mut package_index = (*state.package_index).clone();
    if package_index.get("weft-code-runtime").is_none() {
        package_index.package_sources.push(runtime_source);
    }
    state.package_index = Arc::new(package_index);

    let host_state = WasmHostState {
        config: state.config.clone(),
        pipeline: state.pipeline.clone(),
        runtime_handle: tokio::runtime::Handle::current(),
        process_manager: state.process_manager.clone(),
        vkey_store: state.vkey_store.clone(),
        kv_store: Arc::new(StdMutex::new(HashMap::new())),
        caller_package_name: String::new(),
            package_dir: String::new(),
        permissions: Default::default(),
        package_map: Arc::new(StdMutex::new(HashMap::new())),
        package_aliases: Arc::new(StdMutex::new(HashMap::new())),
        call_depth: Arc::new(StdMutex::new(0)),
        app_state: Arc::new(StdMutex::new(Some(state.clone()))),
    };
    let load_info = PackageLoadInfo {
        name: "weft-code-runtime".into(),
        dir: runtime_dir,
        wasm_path,
        startup_mode: WasmStartupMode::Persistent,
        permissions: weft_core::package::config::PackagePermissions {
            process: true,
            network: true,
            storage: true,
            ..Default::default()
        },
    };
    let host = WasmPackageHost::new(&[load_info], host_state);
    let handle = WasmHandle::new(host);
    state.wasm_handle = Arc::new(RwLock::new(Some(handle)));
    state
}

async fn register_weft_code_runtime_capability(state: &AppState) {
    let mut registry = state.capability_registry.write().await;
    registry.insert(
        "core.execution".into(),
        weft_core::app::CapabilityRegistryEntry {
            capability: "core.execution".into(),
            providers: vec![weft_core::app::CapabilityProviderRecord {
                provider: "core".into(),
                runtime: "core".into(),
                priority: 0,
            }],
            bindings: vec![],
        },
    );
    registry.insert(
        "weft_code.runtime".into(),
        weft_core::app::CapabilityRegistryEntry {
            capability: "weft_code.runtime".into(),
            providers: vec![weft_core::app::CapabilityProviderRecord {
                provider: "weft-code-runtime".into(),
                runtime: "wasm".into(),
                priority: 0,
            }],
            bindings: vec![weft_core::app::CapabilityBindingRecord {
                app: "weft-code".into(),
                provider: "weft-code-runtime".into(),
            }],
        },
    );
}

async fn response_json(resp: axum::response::Response) -> serde_json::Value {
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

fn assert_execution_intent(json: &serde_json::Value, requires_approval: bool) {
    assert_eq!(json["execution_intent"]["status"], "preview_only");
    assert_eq!(json["execution_intent"]["engine"], "weft_execution_engine");
    assert_eq!(json["execution_intent"]["capability"], "tool.runtime");
    assert_eq!(
        json["execution_intent"]["requested_capabilities"],
        serde_json::json!(["tool.shell", "tool.files", "tool.git"])
    );
    assert_eq!(json["execution_intent"]["mutation_allowed"], false);
    assert_eq!(
        json["execution_intent"]["requires_approval"],
        requires_approval
    );
    assert!(json["execution_intent"]["reason"]
        .as_str()
        .unwrap()
        .contains("no execution"));
    assert_eq!(json["execution_intent"]["steps"][0]["id"], "analyze");
    assert_eq!(json["execution_intent"]["steps"][0]["kind"], "analyze");
    assert_eq!(
        json["execution_intent"]["steps"][0]["capability"],
        "tool.runtime"
    );
    assert_eq!(
        json["execution_intent"]["steps"][0]["mutation_allowed"],
        false
    );
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let json = response_json(resp).await;
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn test_list_models() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let json = response_json(resp).await;
    assert_eq!(json["object"], "list");
    assert_eq!(json["data"][0]["id"], "test-model");
    assert_eq!(json["data"][0]["owned_by"], "mock");
}

#[tokio::test]
async fn test_list_providers() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let json = response_json(resp).await;
    assert_eq!(json["providers"][0]["name"], "mock");
    assert_eq!(json["providers"][0]["key_count"], 1);
}

#[tokio::test]
async fn test_chat_completions_no_server() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "test-model",
                        "messages": [{"role": "user", "content": "hi"}]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn test_get_provider_not_found() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/providers/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_services_empty() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/services")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let json = response_json(resp).await;
    assert_eq!(json["services"], serde_json::json!([]));
}

#[tokio::test]
async fn test_weft_code_runtime_discovers_as_wasm_provider() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let resolved_index = weft_core::app::load_package_index(&repo_root.join("packages")).unwrap();

    let mut state = test_state_with_package_index();
    state.package_index = std::sync::Arc::new(resolved_index.clone());

    let runtime_packages = discover_runtime_packages(&repo_root, &state.package_index);
    let runtime_package = runtime_packages
        .iter()
        .find(|package| package.manifest.package_info.name == "weft-code-runtime")
        .unwrap_or_else(|| panic!("weft-code-runtime package missing from runtime discovery"));

    assert_eq!(
        runtime_package.runtime,
        weft_core::package::PackageRuntime::Wasm
    );
    assert_eq!(
        runtime_package.manifest.package_info.name,
        "weft-code-runtime"
    );
    assert_eq!(runtime_package.manifest.package_info.entry, "package.wasm");
    assert_eq!(
        runtime_package.manifest.package_info.description,
        "WASM package checkpoint for the WEFT-Code runtime capability. Provides package-level action dispatch; HTTP route compatibility is handled by the host when available."
    );
    assert!(runtime_package
        .manifest
        .resolved_provides()
        .iter()
        .any(|value| value == "weft_code.runtime"));

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/services")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let services = json["services"].as_array().unwrap();
    assert!(!services
        .iter()
        .any(|service| service["name"] == "weft-code-runtime"));
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_sessions_route_bridges_to_wasm_runtime_shape_for_tui() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let sessions = json.as_array().expect("sessions response is a JSON array");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["id"], "weft-code-local-session");
    assert_eq!(sessions[0]["mode"], "coding");
    assert_eq!(sessions[0]["status"], "active");
    assert!(serde_json::from_value::<Vec<serde_json::Value>>(json).is_ok());
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_teams_route_bridges_to_wasm_runtime_shape_for_tui() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/teams")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    let teams = json.as_array().expect("teams response is a JSON array");
    assert_eq!(teams.len(), 1);

    let team = &teams[0];
    assert_eq!(team["id"], "weft-code-local-team");
    assert_eq!(team["session_id"], "weft-code-local-session");
    assert_eq!(team["roles"], serde_json::json!(["operator"]));
    for field in ["id", "session_id", "roles"] {
        assert!(team.get(field).is_some(), "missing TeamView field {field}");
    }
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_execution_probe_routes_to_wasm_core_execution_dry_run() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/execution-probe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    assert_eq!(json["status"], "host_capability_called");
    assert_eq!(json["engine"], "host_mediated");
    assert_eq!(json["capability"], "core.execution");
    assert_eq!(json["action"], "run");
    assert_eq!(json["would_execute"], false);
    assert_eq!(json["would_mutate"], false);
    assert_eq!(
        json["request"],
        serde_json::json!({
            "capability": "core.execution",
            "action": "run",
            "data": {
                "mode": "dry_run",
                "command": "weft-core-version"
            },
            "provider": "core",
            "app": "weft-claw"
        })
    );
    assert!(
        json["response"].is_object(),
        "unexpected execution probe response: {json}"
    );
    assert_eq!(json["response"]["capability"], "core.execution");
    assert_eq!(json["response"]["provider"], "core");
    assert_eq!(json["response"]["status"], "executed");
    assert_eq!(json["response"]["mode"], "core");
    assert_eq!(json["response"]["response"]["mode"], "dry_run");
    assert_eq!(
        json["response"]["response"]["command"],
        "weft-core-version"
    );
    assert_eq!(json["response"]["response"]["dry_run"], true);
    assert_eq!(json["response"]["response"]["would_execute"], false);
    assert_eq!(json["response"]["response"]["exit_code"], 0);
    assert_eq!(json["response"]["response"]["stdout"], "weft-core-version");
    assert_eq!(json["response"]["response"]["stderr"], "");
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_events_route_bridges_to_wasm_runtime_timeline_snapshot() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let seed_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/session/session-events/natural-language-task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "prompt": "Ship timeline contract",
                        "natural_language_task": "Ship timeline contract",
                        "target_id": "target-events"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(seed_resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    let events = json.as_array().expect("events response is a JSON array");
    assert!(
        events.len() >= 4,
        "expected bootstrap, policy, session, approval events: {json}"
    );

    assert_eq!(events[0]["id"], "runtime-bootstrap");
    assert_eq!(events[0]["kind"], "runtime.bootstrap");
    assert_eq!(events[0]["source"], "weft-code-runtime");
    assert_eq!(events[0]["resource"]["type"], "runtime");
    assert_eq!(events[0]["resource"]["id"], "weft-code-runtime");
    assert_eq!(events[0]["sequence"], 0);
    assert_eq!(events[1]["id"], "policy-current");
    assert_eq!(events[1]["kind"], "policy.current");
    assert_eq!(events[1]["source"], "weft-code-runtime");
    assert_eq!(events[1]["resource"]["type"], "policy");
    assert_eq!(events[1]["resource"]["id"], "current");
    assert_eq!(events[1]["sequence"], 1);
    assert_eq!(events[1]["data"]["policy"], "on_request");

    for (index, event) in events.iter().enumerate() {
        assert_eq!(event["sequence"], index as u64);
        assert!(
            event["summary"].as_str().is_some(),
            "event missing summary: {event}"
        );
        assert!(
            event["data"].is_object(),
            "event data must be object: {event}"
        );
    }
    assert!(events.iter().any(|event| {
        event["kind"] == "session.current"
            && event["resource"]["id"] == "session-events"
            && event["data"]["status"] == "waiting_approval"
    }));
    assert!(events.iter().any(|event| {
        event["kind"] == "approval.current"
            && event["data"]["status"] == "pending"
            && event["resource"]["id"]
                .as_str()
                .unwrap_or_default()
                .starts_with("session-events-approval-")
    }));
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_natural_language_task_route_bridges_to_wasm_runtime_shape_for_tui() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/session/session-1/natural-language-task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "prompt": "Ship bridge tests",
                        "natural_language_task": "Ship bridge tests",
                        "target_id": "target-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    assert_eq!(json["session"]["id"], "session-1");
    assert_eq!(json["session"]["mode"], "coding");
    assert_eq!(json["session"]["status"], "waiting_approval");
    assert_eq!(json["task"]["id"], "session-1-task");
    assert_eq!(json["task"]["session_id"], "session-1");
    assert_eq!(json["task"]["kind"], "coding_task");
    assert_eq!(json["task"]["status"], "waiting_approval");
    assert_eq!(json["related_tasks"], serde_json::json!([]));
    assert_eq!(json["approval"]["status"], "pending");
    assert!(json["approval"]["id"]
        .as_str()
        .unwrap()
        .starts_with("session-1-approval-"));
    assert_execution_intent(&json, true);
    assert_eq!(json["action_kind"], "coding_task");
    assert_eq!(json["action"]["kind"], "natural_language_task");
    assert_eq!(json["action"]["task_kind"], "coding_task");
    assert_eq!(json["action"]["status"], "waiting_approval");
    assert_eq!(json["lifecycle"]["state"], "waiting_approval");
    assert_eq!(json["lifecycle"]["transition"], "approval_pending");
    assert_eq!(json["lifecycle"]["record"]["from"], "queued");
    assert_eq!(json["lifecycle"]["record"]["to"], "waiting_approval");
    assert_eq!(json["interpretation"], "Ship bridge tests");
    assert!(json["result"]
        .as_str()
        .unwrap()
        .contains("waiting for approval"));
    assert!(json["next_steps"].as_array().unwrap().len() == 2);
    assert_eq!(json["created_team"], serde_json::Value::Null);
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_read_only_mode_blocks_mutating_task_but_allows_analysis() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let policy_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/policy")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "policy": "read_only_mode"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let policy_status = policy_resp.status();
    let policy_json = response_json(policy_resp).await;
    assert_eq!(
        policy_status,
        StatusCode::OK,
        "runtime bridge response: {policy_json}"
    );
    assert_eq!(
        policy_json,
        serde_json::json!({ "policy": "read_only_mode" })
    );

    let blocked_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/session/session-read-only-block/natural-language-task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "prompt": "Write a patch for the requested file change",
                        "natural_language_task": "Write a patch for the requested file change",
                        "target_id": "target-read-only-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let blocked_status = blocked_resp.status();
    let blocked_json = response_json(blocked_resp).await;
    assert_eq!(
        blocked_status,
        StatusCode::OK,
        "runtime bridge response: {blocked_json}"
    );
    assert_eq!(blocked_json["task"]["status"], "blocked");
    assert_eq!(blocked_json["approval"], serde_json::Value::Null);
    assert_eq!(blocked_json["lifecycle"]["state"], "blocked");
    assert_eq!(blocked_json["lifecycle"]["transition"], "policy_blocked");
    assert_execution_intent(&blocked_json, true);
    assert!(blocked_json["result"]
        .as_str()
        .unwrap()
        .contains("read_only_mode"));

    let analysis_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/session/session-read-only-analyze/natural-language-task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "prompt": "Analyze the current design and summarize risks",
                        "natural_language_task": "Analyze the current design and summarize risks",
                        "target_id": ""
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let analysis_status = analysis_resp.status();
    let analysis_json = response_json(analysis_resp).await;
    assert_eq!(
        analysis_status,
        StatusCode::OK,
        "runtime bridge response: {analysis_json}"
    );
    assert_eq!(analysis_json["session"]["status"], "active");
    assert_eq!(analysis_json["task"]["status"], "completed");
    assert_eq!(analysis_json["approval"], serde_json::Value::Null);
    assert_eq!(analysis_json["lifecycle"]["state"], "completed");
    assert_eq!(analysis_json["lifecycle"]["transition"], "completed");
    assert_execution_intent(&analysis_json, false);
    assert_eq!(analysis_json["execution"], serde_json::Value::Null);
    assert_eq!(analysis_json["workflow_steps"], serde_json::json!([]));
    assert_eq!(analysis_json["execution_record"]["action"], "run");
    assert_eq!(
        analysis_json["execution_record"]["request"]["data"],
        serde_json::json!({
            "mode": "dry_run",
            "command": "weft-core-version"
        })
    );
    assert_eq!(
        analysis_json["execution_record"]["response"]["response"]["dry_run"],
        true
    );
    assert_eq!(
        analysis_json["tool_preview"]["capability"],
        "core.execution"
    );
    assert_eq!(
        analysis_json["tool_preview"]["operation"],
        "analyze_repo_state"
    );
    assert_eq!(analysis_json["tool_preview"]["dry_run"], true);
    assert_eq!(analysis_json["tool_preview"]["would_execute"], false);
    assert_eq!(analysis_json["tool_preview"]["would_mutate"], false);
    for field in ["file_changes", "patch", "patches", "mutation", "mutations"] {
        assert!(
            analysis_json.get(field).is_none(),
            "analysis response unexpectedly included mutation field {field}: {analysis_json}"
        );
    }
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_natural_language_task_approval_persists_and_retry_advances() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let prompt = "Ship persistent approval gate";
    let target_id = "target-retry-1";

    let first_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/session/session-retry/natural-language-task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "prompt": prompt,
                        "natural_language_task": prompt,
                        "target_id": target_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let first_status = first_resp.status();
    let first_json = response_json(first_resp).await;
    assert_eq!(
        first_status,
        StatusCode::OK,
        "runtime bridge response: {first_json}"
    );
    assert_eq!(first_json["session"]["id"], "session-retry");
    assert_eq!(first_json["session"]["status"], "waiting_approval");
    assert_eq!(first_json["task"]["status"], "waiting_approval");
    assert_eq!(first_json["approval"]["status"], "pending");
    assert!(
        first_json.get("execution_record").is_none() || first_json["execution_record"].is_null(),
        "pending approval response must not call execution dry-run: {first_json}"
    );
    let approval_id = first_json["approval"]["id"]
        .as_str()
        .expect("approval id")
        .to_string();

    let decision_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/weft-code/approval/{approval_id}/decision"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "status": "approved"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let decision_status = decision_resp.status();
    let decision_json = response_json(decision_resp).await;
    assert_eq!(
        decision_status,
        StatusCode::OK,
        "runtime bridge response: {decision_json}"
    );
    assert_eq!(decision_json["id"], approval_id);
    assert_eq!(decision_json["status"], "approved");

    let retry_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/session/session-retry/natural-language-task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "prompt": prompt,
                        "natural_language_task": prompt,
                        "target_id": target_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let retry_status = retry_resp.status();
    let retry_json = response_json(retry_resp).await;
    assert_eq!(
        retry_status,
        StatusCode::OK,
        "runtime bridge response: {retry_json}"
    );
    assert_eq!(retry_json["session"]["status"], "active");
    assert_eq!(retry_json["task"]["status"], "completed");
    assert_eq!(retry_json["approval"]["id"], approval_id);
    assert_eq!(retry_json["approval"]["status"], "approved");
    assert_eq!(retry_json["action"]["status"], "completed");
    assert_eq!(retry_json["lifecycle"]["state"], "completed");
    assert_eq!(retry_json["lifecycle"]["transition"], "completed");
    assert_eq!(
        retry_json["lifecycle"]["record"]["from"],
        "waiting_approval"
    );
    assert_eq!(retry_json["lifecycle"]["record"]["to"], "completed");
    assert!(retry_json["result"]
        .as_str()
        .unwrap()
        .contains("approved and completed"));
    assert_eq!(retry_json["execution"], serde_json::Value::Null);
    assert_eq!(retry_json["workflow_steps"], serde_json::json!([]));
    assert_eq!(retry_json["execution_record"]["action"], "run");
    assert_eq!(
        retry_json["execution_record"]["request"]["data"],
        serde_json::json!({
            "mode": "dry_run",
            "command": "weft-core-version"
        })
    );
    assert_eq!(
        retry_json["execution_record"]["response"]["response"]["dry_run"],
        true
    );

    let approvals_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/approvals")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let approvals_status = approvals_resp.status();
    let approvals_json = response_json(approvals_resp).await;
    assert_eq!(
        approvals_status,
        StatusCode::OK,
        "runtime bridge response: {approvals_json}"
    );
    assert!(approvals_json
        .as_array()
        .unwrap()
        .iter()
        .any(|approval| { approval["id"] == approval_id && approval["status"] == "approved" }));
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_session_tasks_route_bridges_to_wasm_runtime_shape_for_tui() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let seed_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/session/session-1/natural-language-task")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "prompt": "Ship bridge tests",
                        "natural_language_task": "Ship bridge tests",
                        "target_id": "target-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(seed_resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/session/session-1/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    let tasks = json.as_array().expect("tasks response is a JSON array");
    assert_eq!(tasks.len(), 1);

    let task = &tasks[0];
    assert_eq!(task["id"], "session-1-task");
    assert_eq!(task["session_id"], "session-1");
    assert_eq!(task["kind"], "coding_task");
    assert_eq!(task["status"], "waiting_approval");
    assert_eq!(task["team_id"], serde_json::Value::Null);
    assert_eq!(task["parent_task_id"], serde_json::Value::Null);
    for field in [
        "id",
        "session_id",
        "kind",
        "status",
        "team_id",
        "parent_task_id",
    ] {
        assert!(task.get(field).is_some(), "missing TaskView field {field}");
    }
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_team_tasks_route_bridges_to_wasm_runtime_shape_for_tui() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/team/weft-code-local-team/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    let tasks = json
        .as_array()
        .expect("team tasks response is a JSON array");
    assert_eq!(tasks.len(), 1);

    let task = &tasks[0];
    assert_eq!(task["id"], "weft-code-local-team-task");
    assert_eq!(task["team_id"], "weft-code-local-team");
    assert_eq!(task["role"], "operator");
    assert_eq!(task["phase"], "bootstrap");
    assert_eq!(task["status"], "queued");
    for field in ["id", "team_id", "role", "phase", "status"] {
        assert!(
            task.get(field).is_some(),
            "missing TeamTaskView field {field}"
        );
    }

    let unknown_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/team/unknown-team/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let unknown_status = unknown_resp.status();
    let unknown_json = response_json(unknown_resp).await;
    assert_eq!(
        unknown_status,
        StatusCode::OK,
        "runtime bridge response: {unknown_json}"
    );
    assert_eq!(unknown_json, serde_json::json!([]));
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_session_mode_route_bridges_to_wasm_runtime_shape_for_tui() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/session/session-1/mode")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "mode": "plan"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    assert_eq!(json["id"], "session-1");
    assert_eq!(json["mode"], "plan");
    assert_eq!(json["status"], "active");
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_policy_route_persists_update_through_wasm_runtime() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let update_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/policy")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "policy": "read_only_mode"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let update_status = update_resp.status();
    let update_json = response_json(update_resp).await;
    assert_eq!(
        update_status,
        StatusCode::OK,
        "runtime bridge response: {update_json}"
    );
    assert_eq!(
        update_json,
        serde_json::json!({ "policy": "read_only_mode" })
    );

    let get_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/policy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let get_status = get_resp.status();
    let get_json = response_json(get_resp).await;
    assert_eq!(
        get_status,
        StatusCode::OK,
        "runtime bridge response: {get_json}"
    );
    assert_eq!(get_json, serde_json::json!({ "policy": "read_only_mode" }));

    let invalid_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/policy")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "policy": "" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let invalid_status = invalid_resp.status();
    let invalid_json = response_json(invalid_resp).await;
    assert_eq!(
        invalid_status,
        StatusCode::BAD_GATEWAY,
        "runtime bridge response: {invalid_json}"
    );
    assert_eq!(invalid_json["error"], "missing policy");

    let missing_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/policy")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let missing_status = missing_resp.status();
    let missing_json = response_json(missing_resp).await;
    assert_eq!(
        missing_status,
        StatusCode::BAD_GATEWAY,
        "runtime bridge response: {missing_json}"
    );
    assert_eq!(missing_json["error"], "missing policy");

    let final_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/policy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let final_status = final_resp.status();
    let final_json = response_json(final_resp).await;
    assert_eq!(
        final_status,
        StatusCode::OK,
        "runtime bridge response: {final_json}"
    );
    assert_eq!(
        final_json,
        serde_json::json!({ "policy": "read_only_mode" })
    );
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_approvals_route_bridges_to_wasm_runtime_shape_for_tui() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/weft-code/approvals")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    let approvals = json.as_array().expect("approvals response is a JSON array");
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0]["id"], "weft-code-bootstrap-approval");
    assert_eq!(approvals[0]["status"], "pending");
    assert!(approvals[0]
        .get("id")
        .and_then(|value| value.as_str())
        .is_some());
    assert!(approvals[0]
        .get("status")
        .and_then(|value| value.as_str())
        .is_some());
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn weft_code_approval_decision_route_bridges_to_wasm_runtime_shape_for_tui() {
    let state = test_state_with_weft_code_runtime_bridge();
    register_weft_code_runtime_capability(&state).await;

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/weft-code/approval/weft-code-bootstrap-approval/decision")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "status": "approved"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let json = response_json(resp).await;
    assert_eq!(status, StatusCode::OK, "runtime bridge response: {json}");
    assert_eq!(json["id"], "weft-code-bootstrap-approval");
    assert_eq!(json["status"], "approved");
    assert!(json.get("id").and_then(|value| value.as_str()).is_some());
    assert!(json
        .get("status")
        .and_then(|value| value.as_str())
        .is_some());
}

#[tokio::test]
async fn test_package_runtime_list_returns_registered_packages() {
    let state = test_state();
    {
        let mut pm = state.package_manager.write().await;
        pm.register(PackageInfo {
            name: "agent-core".into(),
            version: Some("0.1.0".into()),
            overrides: vec!["chat".into()],
            enabled: true,
            has_ui: false,
            description: Some("agent package".into()),
        });
        pm.register(PackageInfo {
            name: "memory".into(),
            version: Some("0.1.0".into()),
            overrides: vec![],
            enabled: false,
            has_ui: false,
            description: Some("memory package".into()),
        });
    }
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/packages/runtime")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let packages = json["packages"].as_array().unwrap();
    assert_eq!(packages.len(), 2);
    assert!(packages
        .iter()
        .any(|p| p["name"] == "agent-core" && p["enabled"] == true));
    assert!(packages
        .iter()
        .any(|p| p["name"] == "memory" && p["enabled"] == false));
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn test_suite_list_requires_suite_manager_package() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/suites")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json = response_json(resp).await;
    assert_eq!(json["error"], "WASM runtime not available");
}

#[tokio::test]
async fn test_chat_providers_route_removed() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/chat-providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_chat_ignores_removed_x_suite_extension() {
    let app = build_router(test_state());

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "test-model",
                        "messages": [{"role": "user", "content": "hi"}],
                        "x_suite": "weft-claw"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let json = response_json(resp).await;
    assert_eq!(json["error"]["type"], "proxy_error");
}

#[tokio::test]
async fn test_suite_chat_provider_route_removed() {
    let app = build_router(test_state_with_chat_providers(vec![
        weft_core::api::openai_compat::ChatProviderInfo {
            name: "weft-claw".into(),
            endpoint: "/chat".into(),
            description: "Weft Claw 团队".into(),
        },
    ]));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/chat-providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_app_profile_from_str_loose_parses_known_profiles() {
    use weft_core::app::AppProfile;
    assert_eq!(AppProfile::from_str_loose("safe"), AppProfile::Safe);
    assert_eq!(
        AppProfile::from_str_loose("developer"),
        AppProfile::Developer
    );
    assert_eq!(AppProfile::from_str_loose("dev"), AppProfile::Developer);
    assert_eq!(AppProfile::from_str_loose("trusted"), AppProfile::Trusted);
    assert_eq!(AppProfile::from_str_loose("trust"), AppProfile::Trusted);
    assert_eq!(AppProfile::from_str_loose("unknown"), AppProfile::Safe);
}

#[test]
fn test_core_policy_allows_unrestricted_capabilities() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("agent.capability", AppProfile::Safe);
    assert!(decision.allowed);
}

#[test]
fn test_core_policy_blocks_files_under_safe_profile() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("core.files", AppProfile::Safe);
    assert!(!decision.allowed);
}

#[test]
fn test_core_policy_allows_files_under_developer_profile() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("core.files", AppProfile::Developer);
    assert!(decision.allowed);
}

#[test]
fn test_core_policy_blocks_native_execution_under_developer_profile() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("core.native_execution", AppProfile::Developer);
    assert!(!decision.allowed);
}

#[test]
fn test_core_policy_allows_native_execution_under_trusted_profile() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("core.native_execution", AppProfile::Trusted);
    assert!(decision.allowed);
}

#[tokio::test]
async fn test_capability_call_returns_forbidden_when_policy_denies() {
    let state = test_state();
    *state.active_profile.write().await = weft_core::app::AppProfile::Safe;
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "native".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.files/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"list","data":{"path":"."}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let json = response_json(resp).await;
    assert!(json["error"].as_str().unwrap().contains("Policy denied"));
}

#[tokio::test]
async fn test_capability_call_passes_policy_when_profile_sufficient() {
    let state = test_state();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.files/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"list","data":{"path":"."}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_profile_returns_current_profile() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/profile")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["profile"], "developer");
}

#[tokio::test]
async fn test_get_policy_returns_rules() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/policy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert!(json["rules"].is_object());
    assert!(json["rules"]["core.files"].is_object());
}

#[tokio::test]
async fn test_core_files_list_returns_entries() {
    let state = test_state();
    let app_root = state
        .repo_root
        .join("core")
        .join("tests")
        .join("fixtures")
        .join("weft-claw-list");
    std::fs::create_dir_all(app_root.join("workspace")).unwrap();
    std::fs::write(
        app_root.join("workspace").join("sample.txt"),
        "hello workspace",
    )
    .unwrap();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![weft_core::app::CapabilityBindingRecord {
                    app: "weft-claw".into(),
                    provider: "core".into(),
                }],
            },
        );
    }
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec!["core.files".into()],
                enabled_features: vec![],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.files".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "test".into(),
                }],
                config_path: Some(app_root.join("config.toml").display().to_string()),
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: app_root.join("package.toml").display().to_string(),
                    config_path: Some(app_root.join("config.toml").display().to_string()),
                    lock_path: None,
                },
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.files/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"list","app":"weft-claw","data":{"path":"."}})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["mode"], "core");
    assert!(json["response"]["entries"].is_array());
}

#[tokio::test]
async fn test_core_files_metadata_returns_file_info() {
    let state = test_state();
    let app_root = state
        .repo_root
        .join("core")
        .join("tests")
        .join("fixtures")
        .join("weft-claw-meta");
    std::fs::create_dir_all(app_root.join("workspace")).unwrap();
    std::fs::write(
        app_root.join("workspace").join("Cargo.toml"),
        "[package]\nname='demo'\n",
    )
    .unwrap();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![weft_core::app::CapabilityBindingRecord {
                    app: "weft-claw".into(),
                    provider: "core".into(),
                }],
            },
        );
    }
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec!["core.files".into()],
                enabled_features: vec![],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.files".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "test".into(),
                }],
                config_path: Some(app_root.join("config.toml").display().to_string()),
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: app_root.join("package.toml").display().to_string(),
                    config_path: Some(app_root.join("config.toml").display().to_string()),
                    lock_path: None,
                },
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.files/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"metadata","app":"weft-claw","data":{"path":"Cargo.toml"}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["response"]["is_file"], true);
}

#[tokio::test]
async fn test_core_execution_run_returns_output() {
    let state = test_state();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.execution".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.execution".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.execution/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "action": "run",
                        "data": {"mode": "dry_run", "command": "weft-core-version"}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["mode"], "core");
    assert_eq!(json["response"]["exit_code"], 0);
    assert_eq!(json["response"]["mode"], "dry_run");
    assert_eq!(json["response"]["command"], "weft-core-version");
    assert_eq!(json["response"]["would_execute"], false);
    assert_eq!(json["response"]["stdout"], "weft-core-version");
}

#[tokio::test]
async fn test_core_files_unknown_action_returns_error() {
    let state = test_state();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.files/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"invalid_action","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let json = response_json(resp).await;
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Unknown files action"));
}

#[test]
fn test_generation_store_propose_creates_candidate() {
    use weft_core::app::generation::{
        AppGenerationProposal, AppGenerationStore, AppGenerationSummaryMetadata, GenerationStatus,
    };
    let mut store = AppGenerationStore::default();
    let gen = store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.1.0".into(),
        bindings: vec![],
        capabilities: vec!["agent.capability".into()],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    assert_eq!(gen.app_name, "weft-claw");
    assert_eq!(gen.status, GenerationStatus::Candidate);
    assert!(store.candidate.is_some());
    assert!(store.active.is_none());
}

#[test]
fn test_generation_store_verify_passes_with_bindings() {
    use weft_core::app::generation::{
        AppGenerationProposal, AppGenerationStore, AppGenerationSummaryMetadata, GenerationStatus,
    };
    use weft_core::app::AppBindingResolution;
    let mut store = AppGenerationStore::default();
    store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.1.0".into(),
        bindings: vec![AppBindingResolution {
            capability: "agent.capability".into(),
            provider: "agent-core".into(),
            mutable: false,
            source: "test".into(),
        }],
        capabilities: vec!["agent.capability".into()],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    let result = store.verify_candidate(None);
    assert!(result.is_ok());
    assert_eq!(
        store.candidate.as_ref().unwrap().status,
        GenerationStatus::Verified
    );
}

#[test]
fn test_generation_store_verify_fails_without_bindings() {
    use weft_core::app::generation::{
        AppGenerationProposal, AppGenerationStore, AppGenerationSummaryMetadata,
    };
    let mut store = AppGenerationStore::default();
    store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.1.0".into(),
        bindings: vec![],
        capabilities: vec!["agent.capability".into()],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    let result = store.verify_candidate(None);
    assert!(result.is_err());
}

#[test]
fn test_generation_store_activate_requires_verified() {
    use weft_core::app::generation::{
        AppGenerationProposal, AppGenerationStore, AppGenerationSummaryMetadata,
    };
    let mut store = AppGenerationStore::default();
    store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.1.0".into(),
        bindings: vec![],
        capabilities: vec![],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    let result = store.activate();
    assert!(result.is_err());
}

#[test]
fn test_generation_store_full_lifecycle() {
    use weft_core::app::generation::{
        AppGenerationProposal, AppGenerationStore, AppGenerationSummaryMetadata, GenerationStatus,
    };
    use weft_core::app::AppBindingResolution;
    let mut store = AppGenerationStore::default();

    store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.1.0".into(),
        bindings: vec![AppBindingResolution {
            capability: "agent.capability".into(),
            provider: "agent-core".into(),
            mutable: false,
            source: "test".into(),
        }],
        capabilities: vec!["agent.capability".into()],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    store.verify_candidate(None).unwrap();
    store.activate().unwrap();

    assert!(store.active.is_some());
    assert_eq!(
        store.active.as_ref().unwrap().status,
        GenerationStatus::Active
    );
    assert!(store.candidate.is_none());
}

#[test]
fn test_generation_store_rollback_restores_previous() {
    use weft_core::app::generation::{
        AppGenerationProposal, AppGenerationStore, AppGenerationSummaryMetadata, GenerationStatus,
    };
    use weft_core::app::AppBindingResolution;
    let mut store = AppGenerationStore::default();

    let binding = AppBindingResolution {
        capability: "agent.capability".into(),
        provider: "agent-core".into(),
        mutable: false,
        source: "test".into(),
    };

    store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.1.0".into(),
        bindings: vec![binding.clone()],
        capabilities: vec!["agent.capability".into()],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    store.verify_candidate(None).unwrap();
    store.activate().unwrap();
    let first_gen_id = store.active.as_ref().unwrap().id;

    store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.2.0".into(),
        bindings: vec![binding.clone()],
        capabilities: vec!["agent.capability".into()],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    store.verify_candidate(None).unwrap();
    store.activate().unwrap();

    assert!(store.rollback.is_some());
    store.rollback().unwrap();
    assert_eq!(store.active.as_ref().unwrap().id, first_gen_id);
    assert_eq!(
        store.active.as_ref().unwrap().status,
        GenerationStatus::Active
    );
}

#[tokio::test]
async fn test_propose_generation_api_creates_candidate() {
    let state = test_state();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec![
                    "agent.runtime".into(),
                    "memory.store".into(),
                    "ui.surface".into(),
                    "ext.skills".into(),
                    "ext.mcp".into(),
                    "channel.bridge".into(),
                    "core.execution".into(),
                ],
                enabled_features: vec!["chat".into(), "extensions".into(), "channels".into()],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "memory.store".into(),
                        provider: "memory-store".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.skills".into(),
                        provider: "skills-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.mcp".into(),
                        provider: "mcp-client".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "channel.bridge".into(),
                        provider: "channel-core".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                ],
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: state
                        .repo_root
                        .join("packages")
                        .join("weft-claw")
                        .join("package.toml")
                        .display()
                        .to_string(),
                    config_path: Some(
                        state
                            .repo_root
                            .join(".weft")
                            .join("weft-claw")
                            .join("config.toml")
                            .display()
                            .to_string(),
                    ),
                    lock_path: None,
                },
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["status"], "proposed");
    assert_eq!(json["generation"]["app_name"], "weft-claw");
    assert_eq!(json["generation"]["status"], "candidate");
    assert_eq!(json["generation"]["scene"], "");
    assert_eq!(json["generation"]["created_by"], "api");
    assert_eq!(json["generation"]["lock_path"], "generations/1.lock.toml");
    assert!(json["generation"]["binding_set_id"]
        .as_str()
        .unwrap()
        .starts_with("binding-set:sha256:"));
    assert!(json["generation"]["closure_id"]
        .as_str()
        .unwrap()
        .starts_with("closure:sha256:"));
    assert!(json["generation"]["parent_generation"].is_null());
    assert!(json["generation"]["enabled_features"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn test_generation_api_full_lifecycle() {
    let state = test_state_with_package_index();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec![
                    "agent.runtime".into(),
                    "core.execution".into(),
                    "memory.store".into(),
                    "prompt.system".into(),
                    "workflow.orchestration".into(),
                    "tool.runtime".into(),
                    "tool.shell".into(),
                    "tool.files".into(),
                    "tool.web".into(),
                    "tool.git".into(),
                    "ui.surface".into(),
                    "ext.skills".into(),
                    "ext.mcp".into(),
                    "channel.bridge".into(),
                    "team.runtime".into(),
                    "team.taskboard".into(),
                    "team.handoff".into(),
                    "team.role.catalog".into(),
                    "team.delegate".into(),
                    "team.context.shared".into(),
                    "workflow.template.devteam".into(),
                ],
                enabled_features: vec![],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "memory.store".into(),
                        provider: "memory-store".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "prompt.system".into(),
                        provider: "prompt-system".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.orchestration".into(),
                        provider: "workflow-orchestrator".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.runtime".into(),
                        provider: "tool-runtime-core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.shell".into(),
                        provider: "tool-shell".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.files".into(),
                        provider: "tool-files".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.web".into(),
                        provider: "tool-web".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.git".into(),
                        provider: "tool-git".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.skills".into(),
                        provider: "skills-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.mcp".into(),
                        provider: "mcp-client".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "channel.bridge".into(),
                        provider: "channel-core".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.runtime".into(),
                        provider: "team-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.taskboard".into(),
                        provider: "team-task-board".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.handoff".into(),
                        provider: "team-task-board".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.role.catalog".into(),
                        provider: "team-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.delegate".into(),
                        provider: "agent-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.context.shared".into(),
                        provider: "team-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.template.devteam".into(),
                        provider: "workflow-template-devteam".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                ],
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: state
                        .repo_root
                        .join("packages")
                        .join("weft-claw")
                        .join("package.toml")
                        .display()
                        .to_string(),
                    config_path: Some(
                        state
                            .repo_root
                            .join(".weft")
                            .join("weft-claw")
                            .join("config.toml")
                            .display()
                            .to_string(),
                    ),
                    lock_path: None,
                },
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "agent.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "core.execution".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.execution".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "memory.store".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "memory.store".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "memory-store".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "prompt.system".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "prompt.system".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "prompt-system".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.orchestration".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.orchestration".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-orchestrator".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-runtime-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.shell".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.shell".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-shell".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-files".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.web".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.web".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-web".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.git".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.git".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-git".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.skills".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.skills".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "skills-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.mcp".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.mcp".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "mcp-client".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "channel.bridge".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "channel.bridge".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "channel-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    // Propose
    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify
    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/verify")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let json = response_json(resp).await;
    if status != StatusCode::OK {
        panic!("verify failed: {}", json);
    }
    assert_eq!(json["status"], "verified");

    // Activate
    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/activate")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["status"], "activated");
    assert_eq!(json["lock_written"], false);

    // Get generations
    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/apps/weft-claw/generations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert!(json["active"].is_object());
    assert_eq!(json["active"]["status"], "active");
}

#[tokio::test]
async fn test_propose_generation_uses_config_overrides_and_feature_selection() {
    let state = test_state_with_package_index();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec![
                    "agent.runtime".into(),
                    "memory.store".into(),
                    "prompt.system".into(),
                    "ui.surface".into(),
                    "ext.skills".into(),
                    "ext.mcp".into(),
                    "channel.bridge".into(),
                ],
                enabled_features: vec![],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.skills".into(),
                        provider: "skills-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.mcp".into(),
                        provider: "mcp-client".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "memory.store".into(),
                        provider: "memory-store".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "prompt.system".into(),
                        provider: "prompt-system".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.orchestration".into(),
                        provider: "workflow-orchestrator".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.runtime".into(),
                        provider: "tool-runtime-core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.shell".into(),
                        provider: "tool-shell".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.files".into(),
                        provider: "tool-files".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.web".into(),
                        provider: "tool-web".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.git".into(),
                        provider: "tool-git".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "channel.bridge".into(),
                        provider: "channel-core".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                ],
                status: weft_core::app::ResolvedAppStatus::Resolved,
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .parent()
                        .unwrap()
                        .join("packages")
                        .join("weft-claw")
                        .join("package.toml")
                        .to_string_lossy()
                        .to_string(),
                    config_path: Some(
                        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .parent()
                            .unwrap()
                            .join(".weft")
                            .join("weft-claw")
                            .join("config.toml")
                            .to_string_lossy()
                            .to_string(),
                    ),
                    lock_path: None,
                },
                ..Default::default()
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["status"], "proposed");
    let capabilities = json["generation"]["capabilities"].as_array().unwrap();
    assert_eq!(json["generation"]["created_by"], "api");
    assert_eq!(json["generation"]["lock_path"], "generations/1.lock.toml");
    assert!(capabilities.iter().any(|value| value == "ext.skills"));
    assert!(capabilities.iter().any(|value| value == "ext.mcp"));
    assert!(capabilities.iter().any(|value| value == "channel.bridge"));
    assert!(capabilities.iter().any(|value| value == "core.execution"));

    let bindings = json["generation"]["bindings"].as_array().unwrap();
    assert!(bindings.iter().any(|binding| {
        binding["capability"] == "ext.skills" && binding["provider"] == "skills-runtime"
    }));
    assert!(bindings.iter().any(|binding| {
        binding["capability"] == "core.execution" && binding["provider"] == "core"
    }));
}

#[tokio::test]
async fn test_native_provider_returns_not_implemented() {
    let state = test_state_with_package_index();
    *state.active_profile.write().await = weft_core::app::AppProfile::Trusted;
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.native_execution".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.native_execution".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "native-host".into(),
                    runtime: "native".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.native_execution/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"run","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    let json = response_json(resp).await;
    assert_eq!(json["mode"], "native-stub");
}

#[tokio::test]
async fn test_native_provider_blocked_by_policy_under_developer_profile() {
    let state = test_state_with_package_index();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.native_execution".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.native_execution".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "native-host".into(),
                    runtime: "native".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.native_execution/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"run","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_unsigned_provider_blocked_under_developer_profile_at_runtime() {
    let mut state = test_state_with_package_index();
    state.package_index = Arc::new(weft_core::app::PackageIndex {
        version: 1,
        revision: "test-rev".into(),
        source_url: "local://packages".into(),
        package_sources: vec![weft_core::app::PackageSource {
            name: "weft-claw-ui".into(),
            kind: "wasm".into(),
            package_kind: String::new(),
            runtime_provider: "weft-claw".into(),
            current_source: "packages/weft-claw-ui".into(),
            trusted: false,
            signature: "unsigned".into(),
            source_authority: String::new(),
            source_public_keys: vec![],
            provides: vec![],
            requires: vec![],
        }],
    });
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/ui.surface/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"render","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let json = response_json(resp).await;
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("signature 'unsigned'"));
    assert_eq!(json["reason"], "signature_rejected");
}

#[tokio::test]
async fn test_invalid_ed25519_provider_blocked_under_safe_profile_at_runtime() {
    use ed25519_dalek::SigningKey;
    use weft_core::app::sign_package_message;

    let mut state = test_state_with_package_index();
    *state.active_profile.write().await = weft_core::app::AppProfile::Safe;

    let message = weft_core::app::signature_message(
        "weft-claw-ui",
        "current",
        &test_package_digest(&state.repo_root, "packages/weft-claw-ui"),
        "packages/weft-claw-ui",
    );
    let signature = sign_package_message(&SigningKey::from_bytes(&[9u8; 32]), &message);
    let mut parts: Vec<String> = signature.split(':').map(str::to_string).collect();
    let mut signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(&parts[2])
        .expect("signature bytes decode");
    signature_bytes[0] ^= 0x01;
    parts[2] = base64::engine::general_purpose::STANDARD.encode(signature_bytes);

    state.package_index = Arc::new(weft_core::app::PackageIndex {
        version: 1,
        revision: "test-rev".into(),
        source_url: "local://packages".into(),
        package_sources: vec![weft_core::app::PackageSource {
            name: "weft-claw-ui".into(),
            kind: "wasm".into(),
            package_kind: String::new(),
            runtime_provider: "weft-claw".into(),
            current_source: "packages/weft-claw-ui".into(),
            trusted: true,
            signature: parts.join(":"),
            source_authority: String::new(),
            source_public_keys: vec![],
            provides: vec![],
            requires: vec![],
        }],
    });
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/ui.surface/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"render","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let json = response_json(resp).await;
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("not accepted under profile 'safe'"));
    assert!(json["signature"].as_str().unwrap().starts_with("ed25519:"));
}

#[test]
fn test_ed25519_signature_verification_works_for_known_fixture() {
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    let message = weft_core::app::signature_message(
        "signed-package",
        "1.0.0",
        "abc123",
        "local://signed-package",
    );
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let signature_bytes = signing_key.sign(message.as_bytes()).to_bytes();
    let signature = format!(
        "ed25519:{}:{}",
        base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes()),
        base64::engine::general_purpose::STANDARD.encode(signature_bytes)
    );

    let verified = weft_core::app::verify_package_signature(&signature, &message);
    assert!(verified.is_ok());
}

#[tokio::test]
async fn test_verify_generation_rejects_invalid_ed25519_signature_under_safe_profile() {
    use ed25519_dalek::SigningKey;
    use weft_core::app::sign_package_message;

    let mut state = test_state_with_package_index();
    *state.active_profile.write().await = weft_core::app::AppProfile::Safe;

    let source = "packages/weft-claw-ui";
    let message = weft_core::app::signature_message(
        "weft-claw-ui",
        "0.1.0",
        &test_package_digest(&state.repo_root, source),
        source,
    );
    let signature = sign_package_message(&SigningKey::from_bytes(&[11u8; 32]), &message);
    let mut parts: Vec<String> = signature.split(':').map(str::to_string).collect();
    let mut signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(&parts[2])
        .expect("signature bytes decode");
    signature_bytes[0] ^= 0x01;
    parts[2] = base64::engine::general_purpose::STANDARD.encode(signature_bytes);

    state.package_index = Arc::new(weft_core::app::PackageIndex {
        version: 1,
        revision: "test-rev".into(),
        source_url: "local://packages".into(),
        package_sources: vec![weft_core::app::PackageSource {
            name: "weft-claw-ui".into(),
            kind: "metadata".into(),
            package_kind: String::new(),
            runtime_provider: "weft-claw".into(),
            current_source: source.into(),
            trusted: true,
            signature: parts.join(":"),
            source_authority: String::new(),
            source_public_keys: vec![],
            provides: vec![],
            requires: vec![],
        }],
    });
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec!["ui.surface".into(), "agent.runtime".into()],
                enabled_features: vec![],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                ],
                sources: weft_claw_sources(&state.repo_root),
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "agent.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/verify")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = response_json(resp).await;
    assert_eq!(json["error"], "Verification failed");

    let validations = json["candidate"]["validation_results"].as_array().unwrap();
    assert!(!validations.is_empty());
    assert!(validations.iter().all(|item| item["check"].is_string()));
    assert!(validations.iter().any(|item| item["passed"] == false));
}

#[tokio::test]
async fn test_builtin_signed_provider_allowed_under_safe_profile() {
    let state = test_state_with_package_index();
    *state.active_profile.write().await = weft_core::app::AppProfile::Safe;
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "agent.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "core.execution".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.execution".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "memory.store".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "memory.store".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "memory-store".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.skills".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.skills".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "skills-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.mcp".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.mcp".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "mcp-client".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "channel.bridge".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "channel.bridge".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "channel-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/agent.capability/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"health","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(resp.status(), StatusCode::FORBIDDEN);
}

#[test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
fn test_weft_claw_lock_packages_have_sha512_and_bindings() {
    use weft_core::app::config::load_instance_lock;

    let instance_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".weft")
        .join("weft-claw");

    let lock = load_instance_lock(&instance_dir).unwrap();
    let package_count = lock
        .packages
        .iter()
        .filter(|pkg| !pkg.name.trim().is_empty())
        .count();
    let binding_count = lock
        .bindings
        .iter()
        .filter(|binding| !binding.capability.trim().is_empty())
        .count();
    let binding_source_count = lock
        .binding_sources
        .iter()
        .filter(|binding| !binding.capability.trim().is_empty())
        .count();
    assert_eq!(package_count, 21);
    assert_eq!(binding_count, 21);
    assert_eq!(binding_source_count, 21);
    assert_eq!(lock.features.len(), 5);
    let mut enabled_features = lock.assembly.enabled_features.clone();
    enabled_features.sort();
    assert!(enabled_features.is_empty());

    for pkg in &lock.packages {
        assert!(!pkg.name.is_empty());
        assert!(!pkg.sha512.is_empty());
        assert!(!pkg.runtime.is_empty());
        assert!(pkg.trusted || !pkg.source.is_empty());
        assert!(!pkg.signature.is_empty());
        assert!(!pkg.package_kind.is_empty());
    }

    let immutable_bindings: Vec<_> = lock.bindings.iter().filter(|b| !b.mutable).collect();
    assert_eq!(immutable_bindings.len(), 9);
}

#[test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
fn test_weft_claw_instance_lock_is_runtime_only() {
    use weft_core::app::config::load_instance_lock_from_path;

    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let instance_lock_path = repo_root
        .join(".weft")
        .join("weft-claw")
        .join("lock.toml");

    let instance_lock = load_instance_lock_from_path(&instance_lock_path).unwrap();

    assert_eq!(instance_lock.packages.len(), 21);
    assert_eq!(instance_lock.bindings.len(), 21);
    assert_eq!(instance_lock.binding_sources.len(), 21);
    assert_eq!(instance_lock.app, "weft-claw");
    assert_eq!(instance_lock.status, "active");
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn test_weft_claw_full_generation_lifecycle_via_api() {
    let state = test_state_with_package_index();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "AI multi-role dev assistant".into(),
                capabilities: vec![
                    "agent.runtime".into(),
                    "core.execution".into(),
                    "prompt.system".into(),
                    "workflow.orchestration".into(),
                    "tool.runtime".into(),
                    "tool.shell".into(),
                    "tool.files".into(),
                    "tool.web".into(),
                    "tool.git".into(),
                    "memory.store".into(),
                    "ui.surface".into(),
                    "ext.skills".into(),
                    "ext.mcp".into(),
                    "channel.bridge".into(),
                    "team.runtime".into(),
                    "team.taskboard".into(),
                    "team.handoff".into(),
                    "team.role.catalog".into(),
                    "team.delegate".into(),
                    "team.context.shared".into(),
                    "workflow.template.devteam".into(),
                ],
                enabled_features: vec![],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "prompt.system".into(),
                        provider: "prompt-system".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.orchestration".into(),
                        provider: "workflow-orchestrator".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.runtime".into(),
                        provider: "tool-runtime-core".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.shell".into(),
                        provider: "tool-shell".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.files".into(),
                        provider: "tool-files".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.web".into(),
                        provider: "tool-web".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.git".into(),
                        provider: "tool-git".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.skills".into(),
                        provider: "skills-runtime".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.mcp".into(),
                        provider: "mcp-client".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "memory.store".into(),
                        provider: "memory-store".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "channel.bridge".into(),
                        provider: "channel-core".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.runtime".into(),
                        provider: "team-runtime".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.taskboard".into(),
                        provider: "team-task-board".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.handoff".into(),
                        provider: "team-task-board".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.role.catalog".into(),
                        provider: "team-runtime".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.delegate".into(),
                        provider: "team-runtime".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.context.shared".into(),
                        provider: "team-runtime".into(),
                        mutable: true,
                        source: "test".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.template.devteam".into(),
                        provider: "workflow-template-devteam".into(),
                        mutable: false,
                        source: "test".into(),
                    },
                ],
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        let providers = [
            ("agent.runtime", "agent-runtime"),
            ("core.execution", "core"),
            ("prompt.system", "prompt-system"),
            ("workflow.orchestration", "workflow-orchestrator"),
            ("tool.runtime", "tool-runtime-core"),
            ("tool.shell", "tool-shell"),
            ("tool.files", "tool-files"),
            ("tool.web", "tool-web"),
            ("tool.git", "tool-git"),
            ("ext.skills", "skills-runtime"),
            ("ext.mcp", "mcp-client"),
            ("memory.store", "memory-store"),
            ("ui.surface", "weft-claw-ui"),
            ("channel.bridge", "channel-core"),
            ("team.runtime", "team-runtime"),
            ("team.taskboard", "team-task-board"),
            ("team.handoff", "team-task-board"),
            ("team.role.catalog", "team-runtime"),
            ("team.delegate", "team-runtime"),
            ("team.context.shared", "team-runtime"),
            ("workflow.template.devteam", "workflow-template-devteam"),
        ];
        for (cap, provider_name) in providers {
            let runtime = if matches!(cap, "ui.surface") {
                "metadata"
            } else if cap == "core.execution" {
                "core"
            } else {
                "wasm"
            };
            registry.insert(
                cap.to_string(),
                weft_core::app::CapabilityRegistryEntry {
                    capability: cap.to_string(),
                    providers: vec![weft_core::app::CapabilityProviderRecord {
                        provider: provider_name.into(),
                        runtime: runtime.into(),
                        priority: 0,
                    }],
                    bindings: vec![weft_core::app::CapabilityBindingRecord {
                        app: "weft-claw".into(),
                        provider: provider_name.into(),
                    }],
                },
            );
        }
    }

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["generation"]["status"], "candidate");
    assert_eq!(
        json["generation"]["capabilities"].as_array().unwrap().len(),
        21
    );
    assert!(json["generation"]["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|binding| {
            binding["capability"] == "channel.bridge" && binding["provider"] == "channel-core"
        }));

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/verify")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let json = response_json(resp).await;
    if status != StatusCode::OK {
        panic!("verify failed: {}", json);
    }
    assert_eq!(json["generation"]["status"], "verified");
    let validations = json["generation"]["validation_results"].as_array().unwrap();
    assert!(!validations.is_empty());
    let registry_coverage = validations
        .iter()
        .find(|item| item["check"] == "registry-coverage")
        .expect("registry coverage validation present");
    assert_eq!(registry_coverage["passed"], true);
    assert!(registry_coverage["message"]
        .as_str()
        .unwrap()
        .contains("All capabilities found in registry"));
    assert!(validations
        .iter()
        .any(|item| item["check"] == "probe:core.execution" && item["passed"] == true));

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/activate")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["generation"]["status"], "active");
    assert_eq!(json["lock_written"], false);

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/apps/weft-claw/generations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert!(json["active"].is_object());
    assert_eq!(json["active"]["app_name"], "weft-claw");
    assert!(json["active"]["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|binding| {
            binding["capability"] == "core.execution" && binding["provider"] == "core"
        }));
}

#[tokio::test]
async fn test_native_provider_without_loaded_plugin_returns_bad_gateway() {
    let state = test_state_with_package_index();
    *state.active_profile.write().await = weft_core::app::AppProfile::Trusted;
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.native_execution".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.native_execution".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "native-host".into(),
                    runtime: "native".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }
    *state.native_handle.write().await = Some(weft_core::package::NativeHandle::new(
        weft_core::package::NativePackageHost::new(),
    ));

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.native_execution/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"run","data":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let json = response_json(resp).await;
    assert_eq!(json["mode"], "native");
    assert!(json["error"].as_str().unwrap().contains("not loaded"));
}

#[test]
fn native_handle_unload_returns_error_for_missing_plugin() {
    let handle =
        weft_core::package::NativeHandle::new(weft_core::package::NativePackageHost::new());
    let error = handle
        .unload_package("missing-native")
        .unwrap_err()
        .to_string();
    assert!(error.contains("not loaded"));
}

#[test]
fn native_handle_reload_returns_error_for_missing_library() {
    let handle =
        weft_core::package::NativeHandle::new(weft_core::package::NativePackageHost::new());
    let load_info = weft_core::package::NativePackageLoadInfo {
        name: "missing-native".into(),
        dir: std::path::PathBuf::from("./missing-native"),
        library_path: std::path::PathBuf::from("./missing-native/missing.dll"),
    };
    let error = handle.reload_package(&load_info).unwrap_err().to_string();
    assert!(
        error.contains("Failed to load native library")
            || error.contains("No such file")
            || error.contains("cannot find")
    );
}

#[test]
fn native_call_reports_invalid_json_response() {
    unsafe extern "C" fn fake_invalid_json(
        _ptr: *const u8,
        _len: usize,
        out_len: *mut usize,
    ) -> *mut u8 {
        let bytes = b"not-json".to_vec().into_boxed_slice();
        unsafe { *out_len = bytes.len() };
        Box::into_raw(bytes) as *mut u8
    }

    unsafe extern "C" fn fake_free(ptr: *mut u8, len: usize) {
        if ptr.is_null() {
            return;
        }
        let slice_ptr = std::ptr::slice_from_raw_parts_mut(ptr, len);
        let _ = unsafe { Box::from_raw(slice_ptr) };
    }

    let handle = weft_core::package::NativeHandle::from_test_package(
        "broken-native".into(),
        fake_invalid_json,
        Some(fake_free),
    )
    .unwrap();
    let error = handle
        .call_json("broken-native", &serde_json::json!({"action":"run"}))
        .unwrap_err()
        .to_string();
    assert!(error.contains("returned invalid JSON"));
}

#[test]
fn native_call_reports_null_response() {
    unsafe extern "C" fn fake_null(_ptr: *const u8, _len: usize, _out_len: *mut usize) -> *mut u8 {
        std::ptr::null_mut()
    }

    let handle =
        weft_core::package::NativeHandle::from_test_package("null-native".into(), fake_null, None)
            .unwrap();
    let error = handle
        .call_json("null-native", &serde_json::json!({"action":"run"}))
        .unwrap_err()
        .to_string();
    assert!(error.contains("returned null response"));
}

#[test]
fn native_loader_searches_target_debug_outputs() {
    let candidates =
        weft_core::package::native_library_candidates(std::path::Path::new("native-echo"));

    let flattened = candidates
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    assert!(flattened
        .iter()
        .any(|path| path.contains("target") && path.contains("debug")));
}

#[test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
fn native_echo_manifest_is_buildable_outside_workspace() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("packages")
        .join("installed")
        .join("native-echo")
        .join("Cargo.toml");
    let content = std::fs::read_to_string(manifest).unwrap();
    assert!(content.contains("[workspace]"));
    assert!(content.contains("crate-type = [\"cdylib\"]"));
}

#[tokio::test]
async fn test_activate_generation_reports_lock_write_failure() {
    let state = test_state_with_package_index();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec![
                    "agent.runtime".into(),
                    "memory.store".into(),
                    "ui.surface".into(),
                ],
                enabled_features: vec!["chat".into(), "extensions".into(), "channels".into()],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "memory.store".into(),
                        provider: "memory-store".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.skills".into(),
                        provider: "skills-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.mcp".into(),
                        provider: "mcp-client".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "channel.bridge".into(),
                        provider: "channel-core".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                ],
                status: weft_core::app::ResolvedAppStatus::Resolved,
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: state
                        .repo_root
                        .join("packages")
                        .join("weft-claw")
                        .join("package.toml")
                        .display()
                        .to_string(),
                    config_path: Some(
                        state
                            .repo_root
                            .join(".weft")
                            .join("weft-claw")
                            .join("config.toml")
                            .display()
                            .to_string(),
                    ),
                    lock_path: Some(
                        state
                            .repo_root
                            .join("missing-dir")
                            .join("subdir")
                            .join("lock.toml")
                            .display()
                            .to_string(),
                    ),
                },
                config_path: Some(
                    state
                        .repo_root
                        .join(".weft")
                        .join("weft-claw")
                        .join("config.toml")
                        .display()
                        .to_string(),
                ),
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "agent.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let app = build_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/verify")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/activate")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let json = response_json(resp).await;
    assert!(json["status"].is_null());
    assert!(json["lock_written"].is_null());
    assert!(json["reason"].is_null());
}

#[tokio::test]
async fn test_rollback_generation_returns_conflict_without_saved_rollback() {
    let state = test_state_with_package_index();
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/rollback")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn test_activated_generation_writes_real_sha512_hex_packages() {
    use weft_core::app::config::load_instance_lock;

    let state = test_state_with_package_index();
    let lock_path = state
        .repo_root
        .join(".weft")
        .join("weft-claw")
        .join("lock.toml");
    let original_lock = std::fs::read_to_string(&lock_path).unwrap();

    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec![
                    "agent.runtime".into(),
                    "memory.store".into(),
                    "ui.surface".into(),
                ],
                enabled_features: vec!["chat".into(), "extensions".into(), "channels".into()],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "memory.store".into(),
                        provider: "memory-store".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.skills".into(),
                        provider: "skills-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.mcp".into(),
                        provider: "mcp-client".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "channel.bridge".into(),
                        provider: "channel-core".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                ],
                status: weft_core::app::ResolvedAppStatus::Resolved,
                sources: weft_claw_sources(&state.repo_root),
                config_path: Some(
                    state
                        .repo_root
                        .join(".weft")
                        .join("weft-claw")
                        .join("config.toml")
                        .display()
                        .to_string(),
                ),
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "agent.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "memory.store".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "memory.store".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "memory-store".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.skills".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.skills".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "skills-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.mcp".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.mcp".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "mcp-client".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "channel.bridge".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "channel.bridge".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "channel-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let app = build_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/verify")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/activate")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let instance_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".weft")
        .join("weft-claw");
    let lock = load_instance_lock(&instance_dir).unwrap();
    assert!(!lock.packages.is_empty());
    for pkg in &lock.packages {
        assert_eq!(pkg.sha512.len(), 128);
        assert!(pkg.sha512.chars().all(|c| c.is_ascii_hexdigit()));
    }

    std::fs::write(&lock_path, original_lock).unwrap();
}

#[tokio::test]
#[ignore = "stale integration fixture depends on removed package/route artifacts"]
async fn test_verify_attempts_wasm_probe_via_temp_load_when_provider_not_loaded() {
    let state = test_state_with_package_index();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec![
                    "agent.runtime".into(),
                    "core.execution".into(),
                    "memory.store".into(),
                    "prompt.system".into(),
                    "workflow.orchestration".into(),
                    "tool.runtime".into(),
                    "tool.shell".into(),
                    "tool.files".into(),
                    "tool.web".into(),
                    "tool.git".into(),
                    "ui.surface".into(),
                    "ext.skills".into(),
                    "ext.mcp".into(),
                    "channel.bridge".into(),
                    "team.runtime".into(),
                    "team.taskboard".into(),
                    "team.handoff".into(),
                    "team.role.catalog".into(),
                    "team.context.shared".into(),
                    "workflow.template.devteam".into(),
                ],
                enabled_features: vec![],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "memory.store".into(),
                        provider: "memory-store".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "prompt.system".into(),
                        provider: "prompt-system".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.orchestration".into(),
                        provider: "workflow-orchestrator".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.runtime".into(),
                        provider: "tool-runtime-core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.shell".into(),
                        provider: "tool-shell".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.files".into(),
                        provider: "tool-files".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.web".into(),
                        provider: "tool-web".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "tool.git".into(),
                        provider: "tool-git".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ui.surface".into(),
                        provider: "weft-claw".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.skills".into(),
                        provider: "skills-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "ext.mcp".into(),
                        provider: "mcp-client".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "channel.bridge".into(),
                        provider: "channel-core".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.runtime".into(),
                        provider: "team-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.taskboard".into(),
                        provider: "team-task-board".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.handoff".into(),
                        provider: "team-task-board".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.role.catalog".into(),
                        provider: "team-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.delegate".into(),
                        provider: "agent-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "team.context.shared".into(),
                        provider: "team-runtime".into(),
                        mutable: true,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "workflow.template.devteam".into(),
                        provider: "workflow-template-devteam".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                ],
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: state
                        .repo_root
                        .join("packages")
                        .join("weft-claw")
                        .join("package.toml")
                        .display()
                        .to_string(),
                    config_path: Some(
                        state
                            .repo_root
                            .join(".weft")
                            .join("weft-claw")
                            .join("config.toml")
                            .display()
                            .to_string(),
                    ),
                    lock_path: Some(
                        state
                            .repo_root
                            .join(".weft")
                            .join("weft-claw")
                            .join("lock.toml")
                            .display()
                            .to_string(),
                    ),
                },
                config_path: Some(
                    state
                        .repo_root
                        .join(".weft")
                        .join("weft-claw")
                        .join("config.toml")
                        .display()
                        .to_string(),
                ),
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "agent.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "memory.store".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "memory.store".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "memory-store".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "prompt.system".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "prompt.system".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "prompt-system".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "core.execution".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.execution".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.orchestration".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.orchestration".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-orchestrator".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-runtime-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.shell".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.shell".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-shell".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-files".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.web".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.web".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-web".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "tool.git".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.git".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "tool-git".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ui.surface".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ui.surface".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "weft-claw".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.skills".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.skills".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "skills-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "ext.mcp".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "ext.mcp".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "mcp-client".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "channel.bridge".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "channel.bridge".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "channel-core".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.runtime".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.runtime".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.taskboard".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.taskboard".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.handoff".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.handoff".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-task-board".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.role.catalog".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.role.catalog".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.delegate".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.delegate".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "team.context.shared".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "team.context.shared".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "team-runtime".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry.insert(
            "workflow.template.devteam".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "workflow.template.devteam".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "workflow-template-devteam".into(),
                    runtime: "wasm".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/propose")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let app = build_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/verify")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let validations = json["generation"]["validation_results"].as_array().unwrap();
    let probe = validations
        .iter()
        .find(|item| item["check"] == "probe:agent.runtime")
        .unwrap();
    assert_eq!(probe["passed"], true);

    let integrity = validations
        .iter()
        .find(|item| item["check"] == "integrity:agent-runtime")
        .unwrap();
    assert_eq!(integrity["passed"], true);

    let trust = validations
        .iter()
        .find(|item| item["check"] == "trust:agent-runtime")
        .unwrap();
    assert_eq!(trust["passed"], true);

    let signature = validations
        .iter()
        .find(|item| item["check"] == "signature:agent-runtime")
        .unwrap();
    assert_eq!(signature["passed"], true);
}

#[test]
fn test_generation_activate_preserves_previous_active_until_swap() {
    use weft_core::app::generation::{
        AppGenerationProposal, AppGenerationStore, AppGenerationSummaryMetadata, GenerationStatus,
    };
    use weft_core::app::AppBindingResolution;

    let binding = AppBindingResolution {
        capability: "agent.capability".into(),
        provider: "agent-core".into(),
        mutable: false,
        source: "test".into(),
    };

    let mut store = AppGenerationStore::default();
    store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.1.0".into(),
        bindings: vec![binding.clone()],
        capabilities: vec!["agent.capability".into()],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    store.verify_candidate(None).unwrap();
    store.activate().unwrap();
    let first_active = store.active.as_ref().unwrap().id;

    store.propose(AppGenerationProposal {
        app_name: "weft-claw".into(),
        version: "0.2.0".into(),
        bindings: vec![binding],
        capabilities: vec!["agent.capability".into()],
        enabled_features: vec![],
        profile: "developer".into(),
        metadata: AppGenerationSummaryMetadata::default(),
    });
    store.verify_candidate(None).unwrap();
    store.activate().unwrap();

    assert_eq!(
        store.active.as_ref().unwrap().status,
        GenerationStatus::Active
    );
    assert_eq!(
        store.rollback.as_ref().unwrap().status,
        GenerationStatus::Rollback
    );
    assert_eq!(store.rollback.as_ref().unwrap().id, first_active);
}

#[tokio::test]
async fn test_app_health_returns_status_for_resolved_app() {
    let state = test_state();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec!["agent.capability".into()],
                enabled_features: vec![],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "agent.capability".into(),
                    provider: "agent-runtime".into(),
                    mutable: false,
                    source: "test".into(),
                }],
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/apps/weft-claw/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["app"], "weft-claw");
    assert_eq!(json["healthy"], true);
    assert_eq!(json["resolved"], true);
    assert_eq!(json["has_bindings"], true);
}

#[tokio::test]
async fn test_app_health_returns_unhealthy_for_unresolved_app() {
    let state = test_state();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "broken".into(),
            weft_core::app::ResolvedApp {
                name: "broken".into(),
                status: weft_core::app::ResolvedAppStatus::Unresolved,
                errors: vec!["missing provider".into()],
                ..Default::default()
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/apps/broken/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["healthy"], false);
}

#[tokio::test]
async fn test_app_run_dispatches_to_capability() {
    let state = test_state();
    let app_root = state
        .repo_root
        .join("core")
        .join("tests")
        .join("fixtures")
        .join("weft-claw-run");
    std::fs::create_dir_all(&app_root).unwrap();
    std::fs::write(
        app_root.join("lock.toml"),
        r#"
lock_version = 2
app = "weft-claw"
generation = 1
status = "active"
profile = "developer"

[assembly]
enabled_features = []
selected_packages = ["core"]

[[bindings]]
capability = "core.files"
provider = "core"
package = "core"
mutable = false

[[binding_sources]]
capability = "core.files"
source = "test"
package = "core"
"#,
    )
    .unwrap();

    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec!["core.files".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.files".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "test".into(),
                }],
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(app_root.join("lock.toml").display().to_string()),
                },
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }

    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/run")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "capability": "core.files",
                        "action": "list",
                        "data": {"path": "."}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["app"], "weft-claw");
    assert_eq!(json["capability"], "core.files");
    assert!(json["result"]["response"]["entries"].is_array());
}

#[tokio::test]
async fn test_app_run_rejects_without_active_generation() {
    let state = test_state();
    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec!["core.files".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.files".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "test".into(),
                }],
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apps/weft-claw/run")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "capability": "core.files",
                        "action": "list",
                        "data": {"path": "."}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let json = response_json(resp).await;
    assert_eq!(json["reason"], "active_generation_required");
}

#[tokio::test]
async fn test_core_files_blocks_path_outside_workspace() {
    let state = test_state();
    let app_root = state
        .repo_root
        .join("core")
        .join("tests")
        .join("fixtures")
        .join("weft-claw-blocked");
    std::fs::create_dir_all(app_root.join("workspace")).unwrap();
    std::fs::write(app_root.join("workspace").join("ok.txt"), "ok").unwrap();

    {
        let mut apps = state.resolved_apps.write().await;
        apps.insert(
            "weft-claw".into(),
            weft_core::app::ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                display_name: "Weft Claw".into(),
                description: "test".into(),
                capabilities: vec!["core.files".into()],
                enabled_features: vec![],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.files".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "test".into(),
                }],
                config_path: Some(app_root.join("config.toml").display().to_string()),
                sources: weft_core::app::ResolvedAppSources {
                    manifest_path: app_root.join("package.toml").display().to_string(),
                    config_path: Some(app_root.join("config.toml").display().to_string()),
                    lock_path: None,
                },
                status: weft_core::app::ResolvedAppStatus::Resolved,
                ..Default::default()
            },
        );
    }
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![weft_core::app::CapabilityBindingRecord {
                    app: "weft-claw".into(),
                    provider: "core".into(),
                }],
            },
        );
    }

    {
        let mut store = state.generation_store.write().await;
        store.insert(
            "weft-claw".into(),
            active_generation_store("weft-claw", "core.files", "core"),
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.files/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"read","app":"weft-claw","data":{"path":"../outside.txt"}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let json = response_json(resp).await;
    assert!(json["error"].as_str().unwrap().contains("workspace"));
}

#[tokio::test]
async fn test_list_packages_returns_package_index() {
    let state = test_state_with_package_index();
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/packages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["version"], 1);
    assert_eq!(json["packages"].as_array().unwrap().len(), 17);
}

#[test]
fn test_weft_code_runtime_manifest_declares_wasm_package_metadata() {
    let state = test_state_with_package_index();
    let discovered =
        weft_core::package::discover_runtime_packages(&state.repo_root, &state.package_index);

    let runtime_package = discovered
        .into_iter()
        .find(|package| package.manifest.package_info.name == "weft-code-runtime")
        .unwrap_or_else(|| {
            panic!("missing weft-code-runtime package in discovered runtime packages")
        });

    assert_eq!(
        runtime_package.manifest.package_info.name,
        "weft-code-runtime"
    );
    assert_eq!(
        runtime_package.runtime,
        weft_core::package::PackageRuntime::Wasm
    );
    assert_eq!(runtime_package.manifest.package_info.entry, "package.wasm");
    assert_eq!(runtime_package.manifest.package_info.version, "0.1.0");
    assert!(runtime_package.dir.ends_with(std::path::Path::new(
        "packages/installed/weft-code-runtime"
    )));
    assert!(runtime_package
        .manifest
        .resolved_provides()
        .iter()
        .any(|value| value == "weft_code.runtime"));
}
