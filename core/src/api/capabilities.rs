use crate::api::core_capabilities::handle_core_capability;
use crate::api::openai_compat::AppState;
use crate::api::package_ws::dispatch_package_payload;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::path::{Path as FsPath, PathBuf};

async fn workspace_root_for_app(state: &AppState, app_name: Option<&str>) -> Option<PathBuf> {
    let app_name = app_name?;
    let apps = state.resolved_apps.read().await;
    let app = apps.get(app_name)?;
    if !app.sources.manifest_path.is_empty() {
        let manifest_dir = FsPath::new(&app.sources.manifest_path).parent()?;
        let candidate = manifest_dir.join("workspace");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    let config_path = app.config_path.as_ref()?;
    let app_dir = FsPath::new(config_path).parent()?;
    let config = crate::app::load_app_config(app_dir).ok()?;
    let workspace = config.app_runtime.workspace;
    if workspace.is_empty() {
        return None;
    }

    Some(if FsPath::new(&workspace).is_absolute() {
        PathBuf::from(workspace)
    } else {
        app_dir.join(workspace)
    })
}

fn package_for_provider<'a>(
    state: &'a AppState,
    provider: &str,
) -> Option<&'a crate::app::PackageSource> {
    state.package_index.get(provider)
}

async fn enforce_provider_security(
    state: &AppState,
    provider: &str,
    provider_runtime: &str,
) -> Result<(), (StatusCode, serde_json::Value)> {
    let profile = *state.active_profile.read().await;
    if provider_runtime == "core" {
        return Ok(());
    }

    let Some(pkg) = package_for_provider(state, provider) else {
        return Err((
            StatusCode::FORBIDDEN,
            serde_json::json!({
                "error": format!("Provider '{}' is not present in package index", provider),
                "provider": provider,
                "reason": "provider_missing_from_index",
            }),
        ));
    };

    let signature_ok = match profile {
        crate::app::AppProfile::Safe => {
            pkg.signature.starts_with("builtin:")
                || (pkg.signature.starts_with("ed25519:")
                    && crate::app::verify_package_signature_for_source(
                        &pkg.signature,
                        &crate::app::signature_message(
                            &pkg.name,
                            "current",
                            &crate::api::generations::package_digest(
                                &state.repo_root,
                                &pkg.current_source,
                            ),
                            &pkg.current_source,
                        ),
                        &pkg.source_authority,
                        &pkg.source_public_keys,
                    )
                    .is_ok())
        }
        crate::app::AppProfile::Developer => {
            (!pkg.signature.is_empty() && pkg.signature != "unsigned")
                || (pkg.signature.starts_with("ed25519:")
                    && crate::app::verify_package_signature_for_source(
                        &pkg.signature,
                        &crate::app::signature_message(
                            &pkg.name,
                            "current",
                            &crate::api::generations::package_digest(
                                &state.repo_root,
                                &pkg.current_source,
                            ),
                            &pkg.current_source,
                        ),
                        &pkg.source_authority,
                        &pkg.source_public_keys,
                    )
                    .is_ok())
        }
        crate::app::AppProfile::Trusted => true,
    };
    if !pkg.trusted && profile == crate::app::AppProfile::Safe {
        return Err((
            StatusCode::FORBIDDEN,
            serde_json::json!({
                "error": format!("Provider '{}' is not trusted under safe profile", provider),
                "provider": provider,
                "signature": pkg.signature,
                "reason": "provider_not_trusted",
            }),
        ));
    }
    if !signature_ok {
        return Err((
            StatusCode::FORBIDDEN,
            serde_json::json!({
                "error": format!("Provider '{}' signature '{}' is not accepted under profile '{}'", provider, pkg.signature, profile.as_str()),
                "provider": provider,
                "signature": pkg.signature,
                "profile": profile.as_str(),
                "reason": "signature_rejected",
            }),
        ));
    }

    Ok(())
}

pub async fn execute_capability_call(
    state: &AppState,
    name: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, (StatusCode, serde_json::Value)> {
    let registry = state.capability_registry.read().await;
    let capability = if let Some(capability) = registry.get(name) {
        capability.clone()
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            serde_json::json!({
                "error": format!("Capability '{}' not found", name)
            }),
        ));
    };
    drop(registry);

    {
        let profile = *state.active_profile.read().await;
        let decision = state.core_policy.check(name, profile);
        if !decision.allowed {
            return Err((
                StatusCode::FORBIDDEN,
                serde_json::json!({
                    "error": format!("Policy denied: {}", decision.reason),
                    "capability": name,
                    "profile": profile.as_str(),
                }),
            ));
        }
    }

    let selected_provider = payload
        .get("provider")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| {
            let app_name = payload.get("app").and_then(|value| value.as_str())?;
            capability
                .bindings
                .iter()
                .find(|binding| binding.app == app_name)
                .map(|binding| binding.provider.clone())
        })
        .or_else(|| {
            capability
                .providers
                .first()
                .map(|provider| provider.provider.clone())
        });

    let provider = if let Some(p) = selected_provider {
        p
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": format!("Capability '{}' has no available provider", name)
            }),
        ));
    };

    let action = payload
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("call");
    let data = payload
        .get("data")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    let provider_runtime = capability
        .providers
        .iter()
        .find(|p| p.provider == provider)
        .map(|p| p.runtime.as_str())
        .unwrap_or("wasm");

    enforce_provider_security(state, &provider, provider_runtime).await?;

    if provider_runtime == "core" {
        let app_name = payload.get("app").and_then(|value| value.as_str());
        let workspace_root = workspace_root_for_app(state, app_name).await;
        match handle_core_capability(name, action, &data, workspace_root.as_deref()).await {
            Ok(response) => Ok(serde_json::json!({
                "capability": name,
                "provider": provider,
                "response": response,
                "status": "executed",
                "mode": "core"
            })),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "error": err,
                    "capability": name,
                    "provider": provider,
                }),
            )),
        }
    } else if provider_runtime == "native" {
        let native_handle = state.native_handle.read().await;
        let Some(handle) = native_handle.as_ref() else {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                serde_json::json!({
                    "error": format!(
                        "Native provider '{}' for capability '{}' is recognized but no native runtime is active.",
                        provider, name
                    ),
                    "capability": name,
                    "provider": provider,
                    "mode": "native-stub",
                }),
            ));
        };

        let native_payload = serde_json::json!({
            "capability": name,
            "action": action,
            "data": data,
            "app": payload.get("app").cloned().unwrap_or(serde_json::Value::Null),
        });

        match handle.call_json(&provider, &native_payload) {
            Ok(response) => Ok(serde_json::json!({
                "capability": name,
                "provider": provider,
                "response": response,
                "status": "executed",
                "mode": "native"
            })),
            Err(err) => Err((
                StatusCode::BAD_GATEWAY,
                serde_json::json!({
                    "error": err.to_string(),
                    "capability": name,
                    "provider": provider,
                    "mode": "native",
                }),
            )),
        }
    } else {
        let envelope = serde_json::json!({
            "action": action,
            "data": data,
        });

        let response = dispatch_package_payload(&provider, envelope, state).await;

        if response.get("error").is_some() {
            Err((StatusCode::BAD_GATEWAY, response))
        } else {
            Ok(serde_json::json!({
                "capability": name,
                "provider": provider,
                "response": response,
                "status": "executed",
                "mode": if provider_runtime == "service" { "service" } else { "wasm-phase" }
            }))
        }
    }
}

pub async fn list_capabilities(State(state): State<AppState>) -> Json<serde_json::Value> {
    let registry = state.capability_registry.read().await;
    let values: Vec<_> = registry.values().cloned().collect();
    Json(serde_json::json!({ "capabilities": values }))
}

pub async fn get_capability(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let registry = state.capability_registry.read().await;
    if let Some(capability) = registry.get(&name) {
        Ok(Json(serde_json::json!({ "capability": capability })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Capability '{}' not found", name)
            })),
        ))
    }
}

pub async fn capability_call(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    execute_capability_call(&state, &name, payload)
        .await
        .map(Json)
        .map_err(|(status, value)| (status, Json(value)))
}

#[cfg(test)]
mod tests {
    use super::execute_capability_call;
    use crate::api::openai_compat::AppState;
    use crate::app::{
        sign_package_message, signature_message, AppProfile, CapabilityProviderRecord,
        CapabilityRegistry, CapabilityRegistryEntry, CorePolicy, GenerationStoreMap, PackageIndex,
        PackageSource,
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
    use axum::http::StatusCode;
    use ed25519_dalek::SigningKey;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::sync::RwLock;

    fn test_state(
        repo_root: std::path::PathBuf,
        capability_registry: CapabilityRegistry,
        package_index: PackageIndex,
        profile: AppProfile,
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
            resolved_apps: Arc::new(RwLock::new(Default::default())),
            capability_registry: Arc::new(RwLock::new(capability_registry)),
            active_profile: Arc::new(RwLock::new(profile)),
            core_policy: Arc::new(CorePolicy::default_policy()),
            generation_store: Arc::new(RwLock::new(GenerationStoreMap::new())),
            package_index: Arc::new(package_index),
            data_dir: repo_root.join("data"),
            repo_root,
            runtime_token: None,
            runtime_token_path: None,
            chat_providers: Arc::new(RwLock::new(vec![])),
            shutdown_tx: Arc::new(StdMutex::new(None)),
            stream_buffer: Arc::new(StdMutex::new(std::collections::HashMap::new())),
        }
    }

    fn capability_registry_for(provider: &str, runtime: &str) -> CapabilityRegistry {
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "cap.test".into(),
            CapabilityRegistryEntry {
                capability: "cap.test".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: provider.into(),
                    runtime: runtime.into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry
    }

    fn signed_package_source(
        repo_root: &std::path::Path,
        name: &str,
        current_source: &str,
        trusted: bool,
        signature_digest: &str,
    ) -> PackageSource {
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        let message = signature_message(name, "current", signature_digest, current_source);
        let signature = sign_package_message(&signing_key, &message);
        let source_public_key = signature
            .split(':')
            .nth(1)
            .expect("public key segment exists")
            .to_string();

        let source_dir = repo_root.join(current_source);
        std::fs::create_dir_all(&source_dir).expect("source directory created");
        std::fs::write(source_dir.join("package.toml"), "name = 'cap-provider'\n")
            .expect("marker file written");

        PackageSource {
            name: name.into(),
            kind: "wasm".into(),
            package_kind: String::new(),
            runtime_provider: name.into(),
            current_source: current_source.into(),
            trusted,
            signature,
            source_authority: "test-authority".into(),
            source_public_keys: vec![source_public_key],
            provides: vec![],
            requires: vec![],
        }
    }

    #[tokio::test]
    async fn execute_capability_call_rejects_provider_with_mismatched_digest_signature() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join("test-security-mismatched-digest");
        let provider = "cap-provider";
        let source = "fixtures/cap-provider";
        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![signed_package_source(
                &repo_root, provider, source, true, "deadbeef",
            )],
        };
        let state = test_state(
            repo_root.clone(),
            capability_registry_for(provider, "service"),
            package_index,
            AppProfile::Safe,
        );

        let error = execute_capability_call(
            &state,
            "cap.test",
            serde_json::json!({
                "action": "health",
                "data": {},
                "provider": provider,
            }),
        )
        .await
        .expect_err(
            "provider should be rejected when signed digest does not match current source digest",
        );

        assert_eq!(error.0, StatusCode::FORBIDDEN);
        assert!(error.1["error"]
            .as_str()
            .expect("error string")
            .contains("is not accepted under profile 'safe'"));
    }

    #[tokio::test]
    async fn execute_capability_call_rejects_untrusted_provider_under_safe_profile() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join("test-security-untrusted-provider");
        let provider = "cap-provider";
        let source = "fixtures/cap-provider";
        let source_dir = repo_root.join(source);
        std::fs::create_dir_all(&source_dir).expect("source directory created");
        std::fs::write(source_dir.join("package.toml"), "name = 'cap-provider'\n")
            .expect("marker file written");
        let digest = crate::api::generations::package_digest(&repo_root, source);
        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![signed_package_source(
                &repo_root, provider, source, false, &digest,
            )],
        };
        let state = test_state(
            repo_root,
            capability_registry_for(provider, "service"),
            package_index,
            AppProfile::Safe,
        );

        let error = execute_capability_call(
            &state,
            "cap.test",
            serde_json::json!({
                "action": "health",
                "data": {},
                "provider": provider,
            }),
        )
        .await
        .expect_err("untrusted provider should be rejected under safe profile");

        assert_eq!(error.0, StatusCode::FORBIDDEN);
        assert!(error.1["error"]
            .as_str()
            .expect("error string")
            .contains("is not trusted under safe profile"));
    }
}
