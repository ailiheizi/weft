use crate::api::openai_compat::AppState;
use crate::app::{AppProfile, CorePolicy, GenerationStoreMap, PackageIndex, ResolvedAppMap};
use crate::config::store::load_config;
use crate::defaults::{DefaultErrorHandler, DefaultRouter, FailoverSelector};
use crate::package::{
    build_service_config, discover_runtime_packages, DiscoveredPackage, PackageInfo,
    PackageManager, PackageRuntime,
};
use crate::pipeline::Pipeline;
use crate::process::ProcessManager;
use crate::types::{ChatRequest, ChatResponse};
use crate::vkeys::VirtualKeyStore;
use anyhow::Result;
use axum::Router;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::RwLock;

/// Controls how much package metadata the SDK bootstrap loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdkLoadMode {
    /// Embedded mode: avoid package index resolution and external package-index network calls.
    Lightweight,
    /// Full mode: resolve the package index using the same package-index loader used by the main runtime.
    WithPackageIndex,
}

/// Minimal options for embedding or constructing WEFT core services.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeftCoreOptions {
    pub config_path: PathBuf,
    pub repo_root: PathBuf,
    pub load_mode: SdkLoadMode,
    pub start_services: bool,
}

impl WeftCoreOptions {
    pub fn new(config_path: impl Into<PathBuf>, repo_root: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
            repo_root: repo_root.into(),
            load_mode: SdkLoadMode::Lightweight,
            start_services: false,
        }
    }

    pub fn with_load_mode(mut self, load_mode: SdkLoadMode) -> Self {
        self.load_mode = load_mode;
        self
    }

    pub fn with_package_index(mut self) -> Self {
        self.load_mode = SdkLoadMode::WithPackageIndex;
        self
    }

    pub fn with_start_services(mut self, start_services: bool) -> Self {
        self.start_services = start_services;
        self
    }
}

impl Default for WeftCoreOptions {
    fn default() -> Self {
        Self {
            config_path: PathBuf::from("config/config.toml"),
            repo_root: PathBuf::from("."),
            load_mode: SdkLoadMode::Lightweight,
            start_services: false,
        }
    }
}

fn default_provider_name(config: &crate::config::AppConfig) -> String {
    config
        .routing
        .default_provider
        .clone()
        .or_else(|| {
            config
                .providers
                .first()
                .map(|provider| provider.name.clone())
        })
        .unwrap_or_default()
}

fn build_pipeline(config: &crate::config::AppConfig) -> Result<Pipeline> {
    Ok(Pipeline {
        router: Arc::new(DefaultRouter {
            default_provider: default_provider_name(config),
        }),
        key_selector: Arc::new(FailoverSelector),
        transforms: Arc::new(crate::defaults::transforms::TransformRegistry::with_defaults()),
        error_handler: Arc::new(DefaultErrorHandler {
            max_retries: config.fallback.retry_count,
        }),
        http_client: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(90))
            .http1_only()
            .build()?,
    })
}

async fn load_package_index_for_mode(
    options: &WeftCoreOptions,
    data_dir: &str,
    configured_source_url: Option<&str>,
) -> PackageIndex {
    match options.load_mode {
        SdkLoadMode::Lightweight => PackageIndex::default(),
        SdkLoadMode::WithPackageIndex => {
            crate::app::resolve_package_index(&options.repo_root, data_dir, configured_source_url)
                .await
        }
    }
}

async fn register_discovered_runtime_plugins(
    options: &WeftCoreOptions,
    package_index: &PackageIndex,
    process_manager: &Arc<ProcessManager>,
) -> (
    PackageManager,
    Vec<crate::api::openai_compat::ChatProviderInfo>,
    Vec<DiscoveredPackage>,
) {
    let mut package_manager = PackageManager::new();
    let discovered_packages = match options.load_mode {
        SdkLoadMode::Lightweight => Vec::new(),
        SdkLoadMode::WithPackageIndex => {
            discover_runtime_packages(&options.repo_root, package_index)
        }
    };

    for package in &discovered_packages {
        let manifest = &package.manifest;
        package_manager.register(PackageInfo {
            name: manifest.package_info.name.clone(),
            version: Some(manifest.package_info.version.clone()),
            overrides: vec![],
            enabled: true,
            has_ui: false,
            description: Some(manifest.package_info.description.clone()),
        });

        if package.runtime == PackageRuntime::Service {
            if let Ok(service_config) = build_service_config(package) {
                process_manager.register(service_config).await;
            }
        }
    }

    let mut chat_providers: Vec<crate::api::openai_compat::ChatProviderInfo> = discovered_packages
        .iter()
        .filter(|package| {
            package
                .manifest
                .resolved_provides()
                .contains(&"chat_channel".to_string())
        })
        .map(|package| crate::api::openai_compat::ChatProviderInfo {
            name: package.manifest.package_info.name.clone(),
            endpoint: package
                .manifest
                .resolved_chat_endpoint()
                .unwrap_or_else(|| "/chat".to_string()),
            description: package.manifest.package_info.description.clone(),
        })
        .collect();
    chat_providers.sort_by(|left, right| left.name.cmp(&right.name));
    chat_providers
        .dedup_by(|left, right| left.name == right.name && left.endpoint == right.endpoint);

    (package_manager, chat_providers, discovered_packages)
}

/// Lightweight reusable SDK handle around the existing core application state.
#[derive(Clone)]
pub struct WeftCore {
    state: AppState,
}

impl WeftCore {
    pub async fn load(options: WeftCoreOptions) -> Result<Self> {
        let config = load_config(&options.config_path)?;

        let shared_config = Arc::new(RwLock::new(config.clone()));
        let shared_pipeline = Arc::new(build_pipeline(&config)?);

        let process_manager = Arc::new(ProcessManager::new());
        for service in &config.services {
            process_manager.register(service.clone()).await;
        }
        let vkey_store = Arc::new(VirtualKeyStore::new());
        vkey_store.load_from_config(&config.virtual_keys);

        let package_index = load_package_index_for_mode(
            &options,
            &config.core.data_dir,
            config.registry.package_source_url.as_deref(),
        )
        .await;
        let (package_manager, chat_providers, discovered_packages) =
            register_discovered_runtime_plugins(&options, &package_index, &process_manager).await;
        if options.start_services {
            process_manager.start_auto().await;
        }
        let mut capability_registry =
            crate::app::build_capability_registry(&discovered_packages, &ResolvedAppMap::new());
        crate::app::merge_core_capabilities(&mut capability_registry);
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        drop(shutdown_rx);

        Ok(Self {
            state: AppState {
                config: shared_config,
                config_path: options.config_path,
                pipeline: shared_pipeline,
                process_manager,
                vkey_store,
                package_manager: Arc::new(RwLock::new(package_manager)),
                wasm_handle: Arc::new(RwLock::new(None)),
                native_handle: Arc::new(RwLock::new(None)),
                resolved_apps: Arc::new(RwLock::new(ResolvedAppMap::new())),
                capability_registry: Arc::new(RwLock::new(capability_registry)),
                active_profile: Arc::new(RwLock::new(AppProfile::Safe)),
                core_policy: Arc::new(CorePolicy::default_policy()),
                generation_store: Arc::new(RwLock::new(GenerationStoreMap::new())),
                package_index: Arc::new(package_index),
                repo_root: options.repo_root.clone(),
                data_dir: options.repo_root.join("data"),
                runtime_token: None,
                runtime_token_path: None,
                chat_providers: Arc::new(RwLock::new(chat_providers)),
                shutdown_tx: Arc::new(StdMutex::new(Some(_shutdown_tx))),
                stream_buffer: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            },
        })
    }

    pub fn from_state(state: AppState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn into_state(self) -> AppState {
        self.state
    }

    pub fn router(&self) -> Router {
        crate::api::build_router(self.state.clone())
    }

    pub async fn start_services(&self) {
        self.state.process_manager.start_auto().await;
    }

    pub async fn shutdown(&self) -> Result<()> {
        if let Some(shutdown_tx) = self.state.shutdown_tx.lock().unwrap().take() {
            let _ = shutdown_tx.send(());
        }
        self.state.process_manager.stop_all().await;
        Ok(())
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let config = self.state.config.read().await;
        self.state.pipeline.execute(&request, &config).await
    }

    pub async fn models(&self) -> serde_json::Value {
        let config = self.state.config.read().await;
        let models: Vec<serde_json::Value> = config
            .providers
            .iter()
            .flat_map(|provider| {
                provider.models.iter().map(move |model| {
                    serde_json::json!({
                        "id": model,
                        "object": "model",
                        "owned_by": provider.name,
                    })
                })
            })
            .collect();

        serde_json::json!({
            "object": "list",
            "data": models,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{WeftCore, WeftCoreOptions, SdkLoadMode};
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tempfile::TempDir;
    use tower::util::ServiceExt;

    fn write_package_index(dir: &TempDir) {
        let packages_dir = dir.path().join("packages");
        std::fs::create_dir_all(&packages_dir).expect("packages dir created");
        std::fs::write(
            packages_dir.join("index.toml"),
            r#"
version = 1
revision = "test-sdk"
source_url = "local://sdk-test"

[[package_sources]]
name = "sdk-test-runtime"
kind = "service"
package_kind = "provider"
runtime_provider = "sdk-test-runtime"
current_source = "packages/official/sdk-test-runtime"
trusted = true
provides = ["chat_channel"]
"#,
        )
        .expect("package index written");
    }

    fn write_runtime_plugin(dir: &TempDir) {
        let package_dir = dir
            .path()
            .join("packages")
            .join("official")
            .join("sdk-test-runtime");
        std::fs::create_dir_all(&package_dir).expect("package dir created");
        std::fs::write(package_dir.join("run.ps1"), "Write-Output sdk-test-runtime")
            .expect("package entry written");
        std::fs::write(
            package_dir.join("package.toml"),
            r#"
[package_info]
name = "sdk-test-runtime"
version = "0.1.0"
description = "SDK test runtime"
entry = "run.ps1"
provides = ["chat_channel"]
chat_endpoint = "/sdk-chat"

[package]
runtime = "service"
entry = "run.ps1"
provides = ["chat_channel"]
chat_endpoint = "/sdk-chat"

[runtime_contract]
startup_mode = "persistent"
restart_policy = "manual"
"#,
        )
        .expect("package manifest written");
    }

    fn write_minimal_config(dir: &TempDir) -> std::path::PathBuf {
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).expect("config dir created");
        let config_path = config_dir.join("config.toml");
        std::fs::write(
            &config_path,
            r#"
[core]
host = "127.0.0.1"
port = 0
data_dir = "data"

[routing]
default_provider = "mock"

[fallback]
retry_count = 0

[[providers]]
name = "mock"
base_url = "http://127.0.0.1:9/v1"
format = "openai"
models = ["mock-chat"]

[[providers.keys]]
value = "sk-test"

[[virtual_keys]]
key = "vk-test"
provider = "mock"
model = "mock-chat"
"#,
        )
        .expect("config written");
        config_path
    }

    #[tokio::test]
    async fn load_constructs_minimal_embedded_core_without_network() {
        let dir = TempDir::new().expect("temp dir");
        let config_path = write_minimal_config(&dir);

        let options = WeftCoreOptions::new(&config_path, dir.path());
        assert_eq!(options.load_mode, SdkLoadMode::Lightweight);
        assert!(!options.start_services);

        let core = WeftCore::load(options)
            .await
            .expect("core loads from minimal config");

        assert_eq!(core.state().config_path, config_path);
        assert!(core.state().resolved_apps.read().await.is_empty());
        assert!(core.state().chat_providers.read().await.is_empty());
        assert!(core.state().package_manager.read().await.list().is_empty());
        assert!(core.state().process_manager.all_statuses().await.is_empty());
        assert!(core.state().wasm_handle.read().await.is_none());
        assert!(core.state().native_handle.read().await.is_none());
        assert!(core.state().package_index.package_sources.is_empty());

        let models = core.models().await;
        assert_eq!(models["object"], "list");
        assert_eq!(models["data"][0]["id"], "mock-chat");
        assert_eq!(models["data"][0]["owned_by"], "mock");
    }

    #[tokio::test]
    async fn router_serves_models_from_loaded_core() {
        let dir = TempDir::new().expect("temp dir");
        let config_path = write_minimal_config(&dir);
        let core = WeftCore::load(WeftCoreOptions::new(&config_path, dir.path()))
            .await
            .expect("core loads from minimal config");

        let response = core
            .router()
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(payload["object"], "list");
        assert_eq!(payload["data"][0]["id"], "mock-chat");
        assert_eq!(payload["data"][0]["owned_by"], "mock");
    }

    #[tokio::test]
    async fn chat_returns_error_without_real_network() {
        let dir = TempDir::new().expect("temp dir");
        let config_path = write_minimal_config(&dir);
        let core = WeftCore::load(WeftCoreOptions::new(&config_path, dir.path()))
            .await
            .expect("core loads from minimal config");

        let response = core
            .router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": "mock-chat",
                            "messages": [{"role": "user", "content": "hello"}]
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(payload["error"]["type"], "proxy_error");
        assert_eq!(payload["error"]["code"], "bad_gateway");
        assert!(!payload["error"]["message"]
            .as_str()
            .expect("error message")
            .is_empty());
    }

    #[tokio::test]
    async fn with_package_index_loads_runtime_plugin_metadata_without_starting_services() {
        let dir = TempDir::new().expect("temp dir");
        let config_path = write_minimal_config(&dir);
        write_package_index(&dir);
        write_runtime_plugin(&dir);

        let core =
            WeftCore::load(WeftCoreOptions::new(&config_path, dir.path()).with_package_index())
                .await
                .expect("core loads with package index");

        assert_eq!(core.state().package_index.package_sources.len(), 1);
        assert!(core
            .state()
            .package_manager
            .read()
            .await
            .get("sdk-test-runtime")
            .is_some());
        assert_eq!(core.state().chat_providers.read().await.len(), 1);
        assert_eq!(
            core.state()
                .process_manager
                .status("sdk-test-runtime")
                .await
                .expect("service registered"),
            crate::process::ServiceStatus::Stopped
        );
        assert!(core.state().wasm_handle.read().await.is_none());
        assert!(core.state().native_handle.read().await.is_none());
    }

    #[tokio::test]
    async fn shutdown_is_noop_for_unstarted_sdk_core() {
        let dir = TempDir::new().expect("temp dir");
        let config_path = write_minimal_config(&dir);
        let core = WeftCore::load(WeftCoreOptions::new(&config_path, dir.path()))
            .await
            .expect("core loads from minimal config");

        core.shutdown().await.expect("first shutdown succeeds");
        core.shutdown().await.expect("second shutdown succeeds");
        assert!(core.state().process_manager.all_statuses().await.is_empty());
    }
}
