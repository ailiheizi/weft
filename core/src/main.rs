use anyhow::{bail, Context};
use rand::RngCore;
use weft_core::api::build_router;
use weft_core::api::openai_compat::AppState;
use weft_core::app::{
    build_capability_registry, instance_config_path, instance_lock_path, load_instance_config,
    load_instance_config_from_path, load_instance_lock_from_path, load_product_package_declaration,
    merge_core_capabilities, product_package_declaration_path,
    resolve_product_package_declaration_with_policy_and_candidate_context, ResolvedApp,
    ResolvedAppStatus, ResolvedInstanceMap, ResolvedInstanceSources, ServiceOriginCandidatePayload,
};
use weft_core::config::store::load_config_or_default;
use weft_core::defaults::*;
use weft_core::package::bridge::{PackageLoadInfo, WasmHandle, WasmHostState, WasmPackageHost};
use weft_core::package::{
    build_service_config, discover_runtime_packages, is_managed_runtime_root,
    merged_package_aliases, native_library_candidates, resolve_wasm_startup_mode, NativeHandle,
    NativePackageHost, NativePackageLoadInfo, PackageInfo, PackageManager, PackageRuntime,
};
use weft_core::pipeline::Pipeline;
use weft_core::process::ProcessManager;
use weft_core::vkeys::VirtualKeyStore;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};
use tracing_subscriber::EnvFilter;

const ORCHESTRATOR_DISPATCH_CONCURRENCY: usize = 4;

fn resolved_app_instance_dir(app: &ResolvedApp) -> Option<PathBuf> {
    app.sources
        .lock_path
        .as_deref()
        .and_then(|path| Path::new(path).parent().map(Path::to_path_buf))
        .or_else(|| {
            app.sources
                .config_path
                .as_deref()
                .and_then(|path| Path::new(path).parent().map(Path::to_path_buf))
        })
}

fn log_startup_generation_store_diagnostics(
    app: &ResolvedApp,
    store: &weft_core::app::AppGenerationStore,
) {
    let Some(instance_dir) = resolved_app_instance_dir(app) else {
        return;
    };

    let diagnostics = weft_core::app::inspect_startup_generation_store(&instance_dir, store);
    if diagnostics.is_clean() {
        tracing::debug!(
            app = %app.name,
            instance_dir = %instance_dir.display(),
            "Startup generation store diagnostics are clean"
        );
        return;
    }

    for diagnostic in diagnostics.diagnostics {
        tracing::warn!(
            app = %app.name,
            instance_dir = %instance_dir.display(),
            code = %diagnostic.code,
            pointer = diagnostic.pointer.as_deref().unwrap_or("-"),
            generation_id = diagnostic.generation_id.unwrap_or_default(),
            "{}",
            diagnostic.message
        );
    }
}

fn lock_down_runtime_token_permissions(_path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(_path, std::fs::Permissions::from_mode(0o600));
    }
}

fn ensure_runtime_token(data_dir: &Path) -> anyhow::Result<(String, PathBuf)> {
    let token_path = data_dir.join("runtime-token");
    if token_path.exists() {
        let token = std::fs::read_to_string(&token_path)
            .with_context(|| format!("failed to read runtime token from {}", token_path.display()))?
            .trim()
            .to_string();
        if token.is_empty() {
            bail!("runtime token file {} is empty", token_path.display());
        }
        lock_down_runtime_token_permissions(&token_path);
        return Ok((token, token_path));
    }

    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("failed to create data dir {}", data_dir.display()))?;

    let mut token_bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    let token = token_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    std::fs::write(&token_path, format!("{token}\n"))
        .with_context(|| format!("failed to write runtime token to {}", token_path.display()))?;
    lock_down_runtime_token_permissions(&token_path);

    Ok((token, token_path))
}

fn resolve_data_dir(repo_root: &Path, configured_data_dir: &str) -> PathBuf {
    let candidate = PathBuf::from(configured_data_dir);
    if candidate.is_absolute() {
        candidate
    } else {
        repo_root.join(candidate)
    }
}

fn prebuilt_core_capability_registry() -> weft_core::app::CapabilityRegistry {
    let mut registry = weft_core::app::CapabilityRegistry::default();
    merge_core_capabilities(&mut registry);
    registry
}

/// Resolves the highest-priority provider package name for a capability from
/// the registry. Returns `None` if no provider is registered. Used by the
/// background tick loops so they call packages by capability (e.g.
/// `scheduler.cron`) rather than hardcoded names (A3).
fn resolve_capability_provider(
    registry: &weft_core::app::CapabilityRegistry,
    capability: &str,
) -> Option<String> {
    registry
        .get(capability)
        .and_then(|entry| entry.providers.iter().max_by_key(|p| p.priority))
        .map(|p| p.provider.clone())
}

fn discover_product_roots(repo_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut seen = HashSet::new();
    let active_instance_names = discover_instance_names(repo_root);

    let packages_dir = repo_root.join("packages");
    if let Ok(entries) = std::fs::read_dir(&packages_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() || !path.join("package.toml").exists() {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if name.is_empty() || !seen.insert(name.to_string()) {
                continue;
            }
            if !active_instance_names.is_empty() && !active_instance_names.contains(name) {
                continue;
            }
            roots.push(path);
        }
    }

    roots
}

fn discover_instance_names(repo_root: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    let instances_dir = repo_root.join(".weft");
    if let Ok(entries) = std::fs::read_dir(&instances_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() || !path.join("config.toml").exists() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                names.insert(name.to_string());
            }
        }
    }
    names
}

fn lock_bindings_to_app_bindings(
    lock: &weft_core::app::InstanceLock,
) -> Vec<weft_core::app::AppBindingResolution> {
    lock.bindings
        .iter()
        .filter(|binding| !binding.capability.trim().is_empty())
        .map(|binding| weft_core::app::AppBindingResolution {
            capability: binding.capability.clone(),
            provider: binding.provider.clone(),
            mutable: binding.mutable,
            source: if binding.binding_source.trim().is_empty() {
                "lock".into()
            } else {
                binding.binding_source.clone()
            },
        })
        .collect()
}

fn build_generation_store_map(
    resolved_apps: &ResolvedInstanceMap,
) -> weft_core::app::GenerationStoreMap {
    let mut generation_store_map = weft_core::app::GenerationStoreMap::default();
    for app in resolved_apps.values() {
        if let Some(ref lock_path) = app.sources.lock_path {
            if let Ok(lock) = load_instance_lock_from_path(std::path::Path::new(lock_path)) {
                let lock_bindings = lock_bindings_to_app_bindings(&lock);
                let active_generation = weft_core::app::AppGeneration {
                    id: lock.generation as u64,
                    app_name: app.name.clone(),
                    version: app.version.clone(),
                    bindings: if lock_bindings.is_empty() {
                        app.bindings.clone()
                    } else {
                        lock_bindings
                    },
                    capabilities: app.capabilities.clone(),
                    enabled_features: lock.assembly.enabled_features.clone(),
                    scene: lock.scene.clone(),
                    profile: lock.profile.clone(),
                    binding_set_id: lock.binding_set_id.clone(),
                    closure_id: lock.closure_id.clone(),
                    lock_digest: String::new(),
                    lock_path: lock_path.clone(),
                    parent_generation: None,
                    created_by: String::new(),
                    status: weft_core::app::GenerationStatus::Active,
                    validation_results: vec![],
                    created_at: 0,
                };
                let store = weft_core::app::AppGenerationStore {
                    active: Some(active_generation),
                    candidate: None,
                    rollback: None,
                    next_id: lock.generation as u64 + 1,
                };
                log_startup_generation_store_diagnostics(app, &store);
                generation_store_map.insert(app.name.clone(), store);
            }
        }
    }
    generation_store_map
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("weft_core=info".parse()?))
        .init();

    // Load config. In managed desktop mode, the runtime root is the current
    // working directory and config lives under ./config/config.toml.
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_source_root = crate_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate_root.clone());
    let current_dir = std::env::current_dir().unwrap_or_else(|_| repo_source_root.clone());
    let managed_runtime = is_managed_runtime_root(&current_dir);
    let repo_root = if managed_runtime {
        current_dir.clone()
    } else if current_dir.join("packages").is_dir() && current_dir.join("config").is_dir() {
        current_dir.clone()
    } else {
        repo_source_root.clone()
    };
    let config_path = if managed_runtime {
        repo_root.join("config").join("config.toml")
    } else {
        [
            repo_root.join("config").join("config.toml"),
            crate_root.join("config").join("config.toml"),
        ]
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| repo_root.join("config").join("config.toml"))
    };
    let mut config = load_config_or_default(&config_path);

    // Allow CLI overrides: --config-dir, --data-dir, --port
    // These are used when weft-core is launched as a system service with explicit paths.
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--config-dir" => {
                    if let Some(val) = args.get(i + 1) {
                        // Reload config from the specified directory.
                        let override_path = PathBuf::from(val).join("config.toml");
                        if override_path.exists() {
                            config = load_config_or_default(&override_path);
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--data-dir" => {
                    if let Some(val) = args.get(i + 1) {
                        config.core.data_dir = val.clone();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--port" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(port) = val.parse::<u16>() {
                            config.core.port = port;
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                _ => { i += 1; }
            }
        }
    }

    // 把 [web_search] 配置的 API key 注入进程环境，供 js-extension-runtime
    // 等 service 子进程继承（它们读 process.env.EXA_API_KEY 等）。
    config.web_search.apply_to_env();

    let default_provider = config
        .routing
        .default_provider
        .clone()
        .unwrap_or_else(|| "openrouter".into());
    let data_dir = resolve_data_dir(&repo_root, &config.core.data_dir);
    let (runtime_token, runtime_token_path) = ensure_runtime_token(&data_dir)?;
    tracing::info!(
        data_dir = %data_dir.display(),
        runtime_token_path = %runtime_token_path.display(),
        "Runtime loopback token ready"
    );

    let addr = format!("{}:{}", config.core.host, config.core.port);
    tracing::info!("weft-core starting on {}", addr);
    let shared_config = Arc::new(RwLock::new(config.clone()));
    let shared_pipeline = Arc::new(Pipeline {
        router: Arc::new(DefaultRouter {
            default_provider: default_provider.clone(),
        }),
        key_selector: Arc::new(FailoverSelector),
        transforms: Arc::new(weft_core::defaults::transforms::TransformRegistry::with_defaults()),
        error_handler: Arc::new(DefaultErrorHandler {
            max_retries: config.fallback.retry_count,
        }),
        http_client: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(90))
            .http1_only()
            .build()?,
    });

    // Process manager (created early so plugins can use it)
    let process_manager = Arc::new(ProcessManager::new());
    for svc_config in &config.services {
        process_manager.register(svc_config.clone()).await;
    }

    // Virtual key store (created early so plugins can use it)
    let vkey_store = Arc::new(VirtualKeyStore::new());
    vkey_store.load_from_config(&config.virtual_keys);

    let prebuilt_core_registry = prebuilt_core_capability_registry();
    let package_service_client = weft_core::app::ReqwestPackageServiceClient::new();
    let package_index = weft_core::app::resolve_package_index_with_client(
        &repo_root,
        &config.core.data_dir,
        config.registry.package_source_url.as_deref(),
        &package_service_client,
    )
    .await;
    let service_origin_payload = ServiceOriginCandidatePayload::from_package_index(&package_index);
    let resolver_input =
        weft_core::app::ResolveInputCoordinator::from_package_index(&package_index)
            .with_service_origin_candidates(
                &weft_core::app::SynthesizedServiceOriginCandidateAdapter,
                &service_origin_payload,
            )
            .build();
    let runtime_package_aliases = merged_package_aliases(&package_index, &config.package_aliases);

    // Discover runtime package packages once with stable source precedence.
    let mut package_manager = PackageManager::new();
    let discovered_packages = discover_runtime_packages(&repo_root, &package_index);

    // B3: capability requirement integrity check. Warn (don't block) about any
    // `requires` capability that no registered source provides — e.g. a package
    // requiring `ext.mcp` when mcp-client is absent, which would otherwise fail
    // silently at runtime.
    for unmet in package_index.unmet_requirements() {
        tracing::warn!(
            "Unmet requirement: package '{}' requires capability '{}' but no registered source provides it",
            unmet.package,
            unmet.capability
        );
    }
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
    }
    tracing::info!("Discovered {} plugins", discovered_packages.len());

    // Collect chat providers from installed packages
    let chat_providers: Vec<weft_core::api::openai_compat::ChatProviderInfo> = discovered_packages
        .iter()
        .filter(|package| {
            package
                .manifest
                .resolved_provides()
                .contains(&"chat_channel".to_string())
        })
        .map(|package| weft_core::api::openai_compat::ChatProviderInfo {
            name: package.manifest.package_info.name.clone(),
            endpoint: package
                .manifest
                .resolved_chat_endpoint()
                .unwrap_or_else(|| "/chat".to_string()),
            description: package.manifest.package_info.description.clone(),
        })
        .collect();
    let mut chat_providers = chat_providers;
    chat_providers.sort_by(|left, right| left.name.cmp(&right.name));
    chat_providers
        .dedup_by(|left, right| left.name == right.name && left.endpoint == right.endpoint);
    tracing::info!("Registered {} chat providers", chat_providers.len());

    for package in &discovered_packages {
        match package.runtime {
            PackageRuntime::Service => match build_service_config(package) {
                Ok(service_config) => {
                    tracing::info!(
                        "Registering service runtime package '{}' with process manager",
                        package.manifest.package_info.name
                    );
                    process_manager.register(service_config).await;
                }
                Err(error) => {
                    tracing::warn!(
                        "Failed to register service runtime package '{}': {}",
                        package.manifest.package_info.name,
                        error
                    );
                }
            },
            PackageRuntime::Remote | PackageRuntime::Unknown(_) => {
                tracing::warn!(
                    "Package '{}' declares unsupported runtime '{}' - manifest is loaded but runtime will not start",
                    package.manifest.package_info.name,
                    package.runtime.as_str()
                );
            }
            PackageRuntime::Wasm => {}
            PackageRuntime::Native => {
                tracing::info!(
                    "Package '{}' declares native runtime - registered for future native loading",
                    package.manifest.package_info.name
                );
            }
        }
    }
    let mut resolved_apps: ResolvedInstanceMap = HashMap::new();

    for product_root in discover_product_roots(&repo_root) {
        match load_product_package_declaration(&product_root) {
            Ok(manifest) => {
                let app_config = load_instance_config(&product_root).unwrap_or_default();
                let (enabled_features, _, config_overrides) =
                    weft_core::api::generations::preview_runtime_inputs(
                        &manifest,
                        &app_config,
                        &manifest
                            .flattened_bindings()
                            .iter()
                            .map(
                                |(capability, binding)| weft_core::app::AppBindingResolution {
                                    capability: capability.clone(),
                                    provider: binding.provider.clone(),
                                    mutable: binding.mutable,
                                    source: "declaration-default".into(),
                                },
                            )
                            .collect::<Vec<_>>(),
                    );
                match resolve_product_package_declaration_with_policy_and_candidate_context(
                    &manifest,
                    &discovered_packages,
                    None,
                    None,
                    Some(&prebuilt_core_registry),
                    Some(&package_index),
                    resolver_input.resolve_candidate_context(),
                ) {
                    Ok(mut resolved) => {
                        let allowed_capabilities =
                            weft_core::api::generations::preview_runtime_capabilities(
                                &manifest,
                                &enabled_features,
                            );
                        let runtime_bindings =
                            weft_core::api::generations::preview_runtime_bindings_from_manifest(
                                &manifest,
                                &allowed_capabilities,
                                &package_index,
                                &config_overrides,
                            );
                        resolved.capabilities = allowed_capabilities;
                        resolved.enabled_features = enabled_features;
                        resolved.bindings = runtime_bindings;
                        let declaration_path = product_package_declaration_path(&product_root);
                        let instance_config = instance_config_path(&product_root);
                        let instance_lock = instance_lock_path(&product_root);
                        resolved.sources.manifest_path = declaration_path.display().to_string();
                        resolved.sources.config_path = Some(instance_config.display().to_string());
                        resolved.sources.lock_path = Some(instance_lock.display().to_string());
                        resolved.config_path = Some(instance_config.display().to_string());
                        resolved_apps.insert(resolved.name.clone(), resolved);
                    }
                    Err(error) => {
                        let declaration_path = product_package_declaration_path(&product_root);
                        let instance_config = instance_config_path(&product_root);
                        let instance_lock = instance_lock_path(&product_root);
                        let unresolved = ResolvedApp {
                            name: manifest.app.name.clone(),
                            version: manifest.app.version.clone(),
                            display_name: manifest.app.display_name.clone(),
                            description: manifest.app.description.clone(),
                            capabilities:
                                weft_core::api::generations::preview_runtime_capabilities(
                                    &manifest,
                                    &enabled_features,
                                ),
                            enabled_features,
                            bindings: vec![],
                            validation_checks: manifest.validation.checks.clone(),
                            config_path: Some(instance_config.display().to_string()),
                            status: ResolvedAppStatus::Unresolved,
                            errors: vec![error.to_string()],
                            sources: ResolvedInstanceSources {
                                manifest_path: declaration_path.display().to_string(),
                                config_path: Some(instance_config.display().to_string()),
                                lock_path: Some(instance_lock.display().to_string()),
                            },
                        };
                        resolved_apps.insert(unresolved.name.clone(), unresolved);
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    "Failed to load product package declaration at {}: {}",
                    product_root.display(),
                    error
                );
            }
        }
    }

    let mut capability_registry = build_capability_registry(&discovered_packages, &resolved_apps);
    merge_core_capabilities(&mut capability_registry);

    let active_profile = {
        let mut profile = weft_core::app::AppProfile::Safe;
        for app in resolved_apps.values() {
            if app.status == ResolvedAppStatus::Resolved {
                if let Some(ref config_path_str) = app.config_path {
                    if let Ok(app_config) =
                        load_instance_config_from_path(std::path::Path::new(config_path_str))
                    {
                        profile = weft_core::app::AppProfile::from_str_loose(
                            &app_config.app_runtime.profile,
                        );
                        break;
                    }
                }
            }
        }
        profile
    };
    tracing::info!("Active profile: {}", active_profile.as_str());
    let core_policy = weft_core::app::CorePolicy::default_policy();
    // Persistent KV store for plugins
    let kv_path = std::path::PathBuf::from("./data/plugin_kv.json");
    let kv_data: HashMap<String, String> = if kv_path.exists() {
        match std::fs::read_to_string(&kv_path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(e) => {
                tracing::warn!("Failed to load KV store from {}: {}", kv_path.display(), e);
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };
    let kv_count = kv_data.len();
    let kv_store: Arc<StdMutex<HashMap<String, String>>> = Arc::new(StdMutex::new(kv_data));
    if kv_count > 0 {
        tracing::info!("Loaded {} KV entries from {}", kv_count, kv_path.display());
    }
    // 多 agent 编排:把 config 的 [team.roleRouting] 写入 KV(key=`team:role_routing`),
    // 供 team-runtime 按角色查 model/provider。config 是 source of truth,每次启动覆盖。
    {
        let role_routing_json = serde_json::to_string(&config.team.role_routing)
            .unwrap_or_else(|_| "".to_string());
        let n = config.team.role_routing.len();
        kv_store
            .lock()
            .unwrap()
            .insert("team:role_routing".to_string(), role_routing_json);
        tracing::info!("team role_routing loaded into KV: {} role(s)", n);
    }
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Load WASM packages
    let wasm_packages: Vec<_> = discovered_packages
        .iter()
        .filter(|package| package.runtime == PackageRuntime::Wasm)
        .collect();
    let wasm_handle = if !wasm_packages.is_empty() {
        let load_infos: Vec<PackageLoadInfo> = wasm_packages
            .iter()
            .map(|package| PackageLoadInfo {
                name: package.manifest.package_info.name.clone(),
                dir: package.dir.clone(),
                wasm_path: package.entry_path.clone().expect("wasm package entry"),
                startup_mode: resolve_wasm_startup_mode(&package.manifest),
                permissions: package.manifest.permissions.clone(),
            })
            .collect();

        let host_state = WasmHostState {
            config: shared_config.clone(),
            pipeline: shared_pipeline.clone(),
            runtime_handle: tokio::runtime::Handle::current(),
            process_manager: process_manager.clone(),
            vkey_store: vkey_store.clone(),
            kv_store: kv_store.clone(),
            caller_package_name: String::new(),
            package_dir: String::new(),
            permissions: Default::default(),
            package_map: Arc::new(StdMutex::new(HashMap::new())),
            package_aliases: Arc::new(StdMutex::new(runtime_package_aliases.clone())),
            call_depth: Arc::new(StdMutex::new(0)),
            app_state: Arc::new(StdMutex::new(None)),
        };

        let host = WasmPackageHost::new(&load_infos, host_state);

        // Call init() on each loaded plugin
        for info in &load_infos {
            if host.has_package(&info.name) {
                match host.call(&info.name, "init", "") {
                    Ok(_) => tracing::info!("Package '{}' initialized", info.name),
                    Err(e) => tracing::warn!(
                        "Package '{}' init() failed (may not export it): {}",
                        info.name,
                        e
                    ),
                }
            }
        }

        Some(WasmHandle::new(host))
    } else {
        None
    };

    let native_packages: Vec<_> = discovered_packages
        .iter()
        .filter(|package| package.runtime == PackageRuntime::Native)
        .collect();
    let native_handle = if !native_packages.is_empty() {
        let mut host = NativePackageHost::new();
        for package in native_packages {
            if let Some(entry_path) = package.entry_path.as_ref() {
                let candidates = native_library_candidates(entry_path);
                if let Some(library_path) =
                    candidates.into_iter().find(|candidate| candidate.exists())
                {
                    let load_info = NativePackageLoadInfo {
                        name: package.manifest.package_info.name.clone(),
                        dir: package.dir.clone(),
                        library_path,
                    };
                    if let Err(error) = host.load_package(&load_info) {
                        tracing::warn!(
                            "Failed to load native package '{}': {}",
                            package.manifest.package_info.name,
                            error
                        );
                    }
                } else {
                    tracing::warn!(
                        "Native package '{}' discovered but no loadable library was found",
                        package.manifest.package_info.name
                    );
                }
            }
        }
        Some(NativeHandle::new(host))
    } else {
        None
    };

    let generation_store_map = build_generation_store_map(&resolved_apps);

    // Build app state
    let state = AppState {
        config: shared_config,
        config_path: config_path.clone(),
        pipeline: shared_pipeline,
        process_manager: process_manager.clone(),
        vkey_store,
        package_manager: Arc::new(RwLock::new(package_manager)),
        wasm_handle: Arc::new(RwLock::new(wasm_handle)),
        native_handle: Arc::new(RwLock::new(native_handle)),
        resolved_apps: Arc::new(RwLock::new(resolved_apps)),
        capability_registry: Arc::new(RwLock::new(capability_registry)),
        active_profile: Arc::new(RwLock::new(active_profile)),
        core_policy: Arc::new(core_policy),
        generation_store: Arc::new(RwLock::new(generation_store_map)),
        package_index: Arc::new(package_index),
        repo_root: repo_root.clone(),
        data_dir: data_dir.clone(),
        runtime_token: Some(runtime_token.clone()),
        runtime_token_path: Some(runtime_token_path.clone()),
        chat_providers: Arc::new(RwLock::new(chat_providers)),
        shutdown_tx: Arc::new(StdMutex::new(Some(shutdown_tx))),
        stream_buffer: Arc::new(StdMutex::new(std::collections::HashMap::new())),
    };

    if let Some(handle) = state.wasm_handle.read().await.as_ref() {
        if let Err(error) = handle.set_app_state(state.clone()) {
            tracing::warn!("Failed to wire app state into WASM host: {}", error);
        }
    }

    if let Some(ref handle) = *state.wasm_handle.read().await {
        let loaded = handle.package_names();
        for (alias, target) in &runtime_package_aliases {
            if loaded.iter().any(|name| name == target) {
                tracing::info!(
                    "Runtime package alias '{}' resolved to loaded implementation '{}'",
                    alias,
                    target
                );
            } else {
                tracing::warn!(
                    "Runtime package alias '{}' points to '{}' but that implementation is not loaded",
                    alias,
                    target
                );
            }
        }
    }

    // Auto-start services
    process_manager.start_auto().await;

    // Cron tick timer — drive the scheduler.cron provider every 10s; also runs
    // periodic memory cleanup. Providers are resolved by capability (A3) rather
    // than hardcoded package names.
    {
        let cron_handle = state.wasm_handle.clone();
        let cron_registry = state.capability_registry.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            let mut cleanup_counter: u32 = 0;
            loop {
                interval.tick().await;
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let tick_input = format!(r#"{{"now_ms":{}}}"#, now_ms);

                // Resolve providers by capability (releases the registry lock
                // before touching the wasm handle).
                let (cron_provider, memory_provider) = {
                    let reg = cron_registry.read().await;
                    (
                        resolve_capability_provider(&reg, "scheduler.cron"),
                        resolve_capability_provider(&reg, "memory.store"),
                    )
                };

                let guard = cron_handle.read().await;
                if let Some(ref wh) = *guard {
                    // Cron tick
                    if let Some(ref provider) = cron_provider {
                        if wh.has_package(provider) {
                            if let Err(e) = wh.call(provider, "tick", &tick_input) {
                                tracing::debug!("cron tick error: {}", e);
                            }
                        }
                    }
                    // Memory cleanup every ~5 minutes (30 ticks * 10s)
                    cleanup_counter += 1;
                    if cleanup_counter >= 30 {
                        cleanup_counter = 0;
                        if let Some(ref provider) = memory_provider {
                            if wh.has_package(provider) {
                                let cleanup_input =
                                    format!(r#"{{"agent":"*","now_ms":{}}}"#, now_ms);
                                let _ = wh.call(provider, "cleanup_expired", &cleanup_input);
                            }
                        }
                    }
                }
            }
        });
    }

    {
        let orch_handle = state.wasm_handle.clone();
        let orch_registry = state.capability_registry.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
            loop {
                interval.tick().await;
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let tick_input = format!(r#"{{"now_ms":{}}}"#, now_ms);

                // Resolve the workflow.orchestration provider by capability (A3).
                let orch_provider = {
                    let reg = orch_registry.read().await;
                    resolve_capability_provider(&reg, "workflow.orchestration")
                };
                let Some(orch_provider) = orch_provider else {
                    continue;
                };

                // Clone the handle out, then release the RwLock read guard
                // BEFORE the (potentially slow, LLM-bearing) dispatch call so a
                // long dispatch never blocks writers (package load / generation swap).
                let wh = orch_handle.read().await.clone();
                if let Some(wh) = wh {
                    if wh.has_package(&orch_provider) {
                        {
                            let wh_tick = wh.clone();
                            let prov_tick = orch_provider.clone();
                            let inp_tick = tick_input.clone();
                            let res_tick = tokio::task::spawn_blocking(move || {
                                wh_tick.call(&prov_tick, "tick", &inp_tick)
                            })
                            .await;
                            match res_tick {
                                Ok(Err(e)) => tracing::debug!("orchestrator tick error: {}", e),
                                Err(e) => tracing::debug!("orchestrator tick join error: {}", e),
                                Ok(Ok(_)) => {}
                            }
                        }

                        let wh_list = wh.clone();
                        let prov_list = orch_provider.clone();
                        let inp_list = tick_input.clone();
                        let pending = match tokio::task::spawn_blocking(move || {
                            wh_list.call(&prov_list, "list_pending_handoffs", &inp_list)
                        })
                        .await
                        {
                            Ok(Ok(result)) => result,
                            Ok(Err(e)) => {
                                tracing::debug!("orchestrator list_pending_handoffs error: {}", e);
                                continue;
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "orchestrator list_pending_handoffs join error: {}",
                                    e
                                );
                                continue;
                            }
                        };

                        let handoffs = match serde_json::from_str::<serde_json::Value>(&pending)
                            .ok()
                            .and_then(|value| {
                                value
                                    .get("data")
                                    .and_then(|data| data.get("handoffs"))
                                    .and_then(|handoffs| handoffs.as_array().cloned())
                            }) {
                            Some(handoffs) => handoffs,
                            None => continue,
                        };

                        let semaphore = Arc::new(Semaphore::new(ORCHESTRATOR_DISPATCH_CONCURRENCY));
                        let mut tasks = Vec::with_capacity(handoffs.len());

                        for handoff in handoffs {
                            let board_id = handoff
                                .get("board_id")
                                .and_then(|value| value.as_str())
                                .map(str::to_owned);
                            let handoff_id = handoff
                                .get("handoff_id")
                                .and_then(|value| value.as_str())
                                .map(str::to_owned);
                            let (Some(board_id), Some(handoff_id)) = (board_id, handoff_id) else {
                                continue;
                            };

                            let permit = match semaphore.clone().acquire_owned().await {
                                Ok(permit) => permit,
                                Err(e) => {
                                    tracing::debug!("orchestrator semaphore closed: {}", e);
                                    break;
                                }
                            };
                            let wh_dispatch = wh.clone();
                            let prov_dispatch = orch_provider.clone();
                            let input = format!(
                                r#"{{"board_id":"{}","handoff_id":"{}"}}"#,
                                serde_json::to_string(&board_id).unwrap_or_default().trim_matches('"'),
                                serde_json::to_string(&handoff_id).unwrap_or_default().trim_matches('"')
                            );
                            tasks.push(tokio::spawn(async move {
                                let _permit = permit;
                                let res = tokio::task::spawn_blocking(move || {
                                    wh_dispatch.call_isolated(&prov_dispatch, "dispatch_one", &input)
                                })
                                .await;
                                match res {
                                    Ok(Err(e)) => tracing::debug!(
                                        board_id = %board_id,
                                        handoff_id = %handoff_id,
                                        "orchestrator dispatch_one error: {}",
                                        e
                                    ),
                                    Err(e) => tracing::debug!(
                                        board_id = %board_id,
                                        handoff_id = %handoff_id,
                                        "orchestrator dispatch_one join error: {}",
                                        e
                                    ),
                                    Ok(Ok(_)) => {}
                                }
                            }));
                        }

                        for task in tasks {
                            if let Err(e) = task.await {
                                tracing::debug!("orchestrator dispatch task join error: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }

    // KV persistence helper
    fn save_kv(kv: &Arc<StdMutex<HashMap<String, String>>>, path: &std::path::Path) {
        let data = kv.lock().unwrap();
        if data.is_empty() {
            return;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string(&*data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    tracing::warn!("Failed to save KV store: {}", e);
                }
            }
            Err(e) => tracing::warn!("Failed to serialize KV store: {}", e),
        }
    }

    // Periodic KV save — every 5 minutes
    {
        let kv_save = kv_store.clone();
        let kv_save_path = kv_path.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                save_kv(&kv_save, &kv_save_path);
            }
        });
    }

    let pm_shutdown = process_manager.clone();
    let kv_shutdown = kv_store.clone();
    let kv_shutdown_path = kv_path.clone();
    let graceful_shutdown = async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = async {
                let _ = shutdown_rx.await;
            } => {}
        }
        tracing::info!("Shutting down, saving KV store and stopping services...");
        save_kv(&kv_shutdown, &kv_shutdown_path);
        pm_shutdown.stop_all().await;
    };

    // Build router and serve
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("weft-core listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(graceful_shutdown)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_generation_store_map, discover_product_roots};
    use weft_core::app::{
        inspect_startup_generation_store, save_generation_index, write_active_generation_pointer,
        write_previous_generation_pointer, AppGenerationIndex, GenerationStatus, ResolvedApp,
        ResolvedAppMap, ResolvedAppSources,
    };

    fn temp_root(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "weft-main-{}-{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after epoch")
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn discover_product_roots_returns_package_roots_only() {
        let root = temp_root("discover-roots");
        let package_dir = root.join("packages").join("weft-claw");
        let secondary_dir = root.join("packages").join("secondary-product");
        let legacy_app_dir = root.join("apps").join("legacy-product");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&package_dir).expect("package dir created");
        std::fs::create_dir_all(&secondary_dir).expect("secondary dir created");
        std::fs::create_dir_all(&legacy_app_dir).expect("legacy app dir created");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            package_dir.join("package.toml"),
            "[identity]\nname='weft-claw'\n",
        )
        .expect("package declaration written");
        std::fs::write(
            secondary_dir.join("package.toml"),
            "[identity]\nname='secondary-product'\n",
        )
        .expect("secondary package declaration written");
        std::fs::write(
            legacy_app_dir.join("app.toml"),
            "[app]\nname='legacy-product'\n",
        )
        .expect("legacy app declaration written");
        std::fs::write(instance_dir.join("config.toml"), "schema_version = 2\n")
            .expect("instance config written");

        let roots = discover_product_roots(&root);

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], package_dir);
        assert!(!roots.iter().any(|path| path == &legacy_app_dir));
        assert!(!roots.iter().any(|path| path == &secondary_dir));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn generation_store_restores_from_explicit_instance_lock_path() {
        let root = temp_root("generation-store");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "lock".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let generation = store
            .get("weft-claw")
            .and_then(|entry| entry.active.as_ref())
            .expect("active generation restored from explicit lock path");
        assert_eq!(generation.id, 7);
        assert_eq!(generation.bindings[0].capability, "core.execution");
        assert_eq!(generation.bindings[0].provider, "core");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn generation_store_prefers_lock_bindings_over_resolved_app_bindings() {
        let root = temp_root("generation-store-lock-bindings");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='weft_claw.turn'\nprovider='weft-claw'\nmutable=false\nbinding_source='declaration-default'\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["weft_claw.turn".into(), "core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "declaration-default".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let generation = store
            .get("weft-claw")
            .and_then(|entry| entry.active.as_ref())
            .expect("active generation restored from explicit lock path");
        assert!(generation
            .bindings
            .iter()
            .any(|binding| binding.capability == "weft_claw.turn"));
        assert_eq!(generation.bindings.len(), 2);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn generation_store_restores_runtime_only_instance_lock_using_resolved_app_bindings() {
        let root = temp_root("generation-store-runtime-only");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=10\nstatus='active'\nprofile='developer'\n[assembly]\nenabled_features=[]\nselected_packages=['agent-runtime']\n",
        )
        .expect("runtime-only instance lock written");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["agent.runtime".into(), "core.execution".into()],
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
                ],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let generation = store
            .get("weft-claw")
            .and_then(|entry| entry.active.as_ref())
            .expect("active generation restored from runtime-only instance lock");
        assert_eq!(generation.id, 10);
        assert_eq!(generation.version, "0.1.0");
        assert_eq!(generation.bindings.len(), 2);
        assert_eq!(generation.capabilities.len(), 2);
        assert!(generation.enabled_features.is_empty());
        assert_eq!(generation.profile, "developer");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn startup_generation_store_ignores_missing_pointer_and_index_files() {
        let root = temp_root("startup-generation-missing-index");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\nscene='team'\nbinding_set_id='binding-set:sha256:7'\nclosure_id='closure:sha256:7'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "lock".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let app_store = store.get("weft-claw").expect("store entry present");
        let diagnostics = inspect_startup_generation_store(&instance_dir, app_store);

        assert_eq!(app_store.active.as_ref().expect("active generation").id, 7);
        assert_eq!(app_store.next_id, 8);
        assert!(diagnostics.is_clean());
        assert!(diagnostics.diagnostics.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn startup_generation_store_reports_pointer_and_index_mismatch_without_changing_store() {
        let root = temp_root("startup-generation-mismatch");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\nscene='team'\nbinding_set_id='binding-set:sha256:7'\nclosure_id='closure:sha256:7'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");
        write_active_generation_pointer(&instance_dir, Some(9)).expect("active pointer written");
        write_previous_generation_pointer(&instance_dir, Some(6))
            .expect("previous pointer written");
        save_generation_index(
            &instance_dir,
            &AppGenerationIndex {
                schema_version: weft_core::app::GENERATION_INDEX_SCHEMA_VERSION,
                active: Some(9),
                previous: Some(6),
                candidate: None,
                next_id: 10,
                generations: vec![
                    weft_core::app::AppGeneration {
                        id: 6,
                        app_name: "weft-claw".into(),
                        version: "0.1.0".into(),
                        bindings: vec![],
                        capabilities: vec!["core.execution".into()],
                        enabled_features: vec![],
                        scene: "team".into(),
                        profile: "developer".into(),
                        binding_set_id: "binding-set:sha256:6".into(),
                        closure_id: "closure:sha256:6".into(),
                        lock_digest: "sha256:lock-6".into(),
                        lock_path: "generations/6.lock.toml".into(),
                        parent_generation: Some(5),
                        created_by: "cli".into(),
                        status: GenerationStatus::Rollback,
                        validation_results: vec![],
                        created_at: 6,
                    },
                    weft_core::app::AppGeneration {
                        id: 9,
                        app_name: "weft-claw".into(),
                        version: "0.1.0".into(),
                        bindings: vec![],
                        capabilities: vec!["core.execution".into()],
                        enabled_features: vec![],
                        scene: "stale".into(),
                        profile: "developer".into(),
                        binding_set_id: "binding-set:sha256:9".into(),
                        closure_id: "closure:sha256:9".into(),
                        lock_digest: "sha256:lock-9".into(),
                        lock_path: "generations/9.lock.toml".into(),
                        parent_generation: Some(6),
                        created_by: "cli".into(),
                        status: GenerationStatus::Active,
                        validation_results: vec![],
                        created_at: 9,
                    },
                ],
            },
        )
        .expect("index saved");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "lock".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let app_store = store.get("weft-claw").expect("store entry present");
        let diagnostics = inspect_startup_generation_store(&instance_dir, app_store);

        assert_eq!(app_store.active.as_ref().expect("active generation").id, 7);
        assert_eq!(
            app_store.active.as_ref().expect("active generation").scene,
            "team"
        );
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "startup_active_pointer_mismatch"));
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "startup_previous_pointer_mismatch"));
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "startup_generation_index_active_mismatch"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn startup_generation_store_accepts_matching_pointer_and_index_diagnostics() {
        let root = temp_root("startup-generation-clean");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\nscene='team'\nbinding_set_id='binding-set:sha256:7'\nclosure_id='closure:sha256:7'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");
        write_active_generation_pointer(&instance_dir, Some(7)).expect("active pointer written");
        save_generation_index(
            &instance_dir,
            &AppGenerationIndex {
                schema_version: weft_core::app::GENERATION_INDEX_SCHEMA_VERSION,
                active: Some(7),
                previous: None,
                candidate: None,
                next_id: 8,
                generations: vec![weft_core::app::AppGeneration {
                    id: 7,
                    app_name: "weft-claw".into(),
                    version: "0.1.0".into(),
                    bindings: vec![weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "lock".into(),
                    }],
                    capabilities: vec!["core.execution".into()],
                    enabled_features: vec![],
                    scene: "team".into(),
                    profile: "developer".into(),
                    binding_set_id: "binding-set:sha256:7".into(),
                    closure_id: "closure:sha256:7".into(),
                    lock_digest: String::new(),
                    lock_path: instance_dir.join("lock.toml").display().to_string(),
                    parent_generation: None,
                    created_by: String::new(),
                    status: GenerationStatus::Active,
                    validation_results: vec![],
                    created_at: 0,
                }],
            },
        )
        .expect("index saved");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "lock".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let app_store = store.get("weft-claw").expect("store entry present");
        let diagnostics = inspect_startup_generation_store(&instance_dir, app_store);

        assert_eq!(app_store.active.as_ref().expect("active generation").id, 7);
        assert!(diagnostics.is_clean());
        assert!(diagnostics.diagnostics.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }
}
