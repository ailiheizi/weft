use crate::api::openai_compat::AppState;
use crate::app::{
    binding_set_id_from_lock_bindings, closure_digest_from_lock_packages,
    closure_id_from_lock_packages, scene_digest, AppGenerationSummaryMetadata,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use sha2::{Digest, Sha512};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{ErrorKind, Write};
use std::path::{Path as FsPath, PathBuf};

const WEFT_CLAW_REQUIRED_REAL_PACKAGES: &[&str] = &[
    "prompt-system",
    "workflow-orchestrator",
    "tool-runtime-core",
    "tool-shell",
    "tool-files",
    "tool-web",
    "tool-git",
];

#[derive(Debug, Clone, Serialize)]
struct GenerationPointerSnapshot {
    path: String,
    generation_id: Option<u64>,
    read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct GenerationIndexSnapshot {
    path: String,
    present: bool,
    index: Option<crate::app::AppGenerationIndex>,
    read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct InstanceGenerationDiagnostics {
    instance_dir: String,
    active_pointer: GenerationPointerSnapshot,
    previous_pointer: GenerationPointerSnapshot,
    generation_index: GenerationIndexSnapshot,
    consistency_report: Option<crate::app::GenerationIndexConsistencyReport>,
}

#[derive(Debug, Clone, Serialize)]
struct GenerationDiagnosticsResponse {
    app: String,
    store_present: bool,
    store_summary: crate::app::AppGenerationIndex,
    instance: Option<InstanceGenerationDiagnostics>,
}

#[derive(Debug, Clone, Serialize)]
struct ScalarDiff<T>
where
    T: Serialize,
{
    from: T,
    to: T,
    changed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CollectionDiff<T>
where
    T: Serialize,
{
    from: Vec<T>,
    to: Vec<T>,
    added: Vec<T>,
    removed: Vec<T>,
    changed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct BindingChange<T>
where
    T: Serialize,
{
    capability: String,
    from: T,
    to: T,
}

#[derive(Debug, Clone, Serialize)]
struct BindingCollectionDiff<T>
where
    T: Serialize,
{
    from: Vec<T>,
    to: Vec<T>,
    added: Vec<T>,
    removed: Vec<T>,
    changed_entries: Vec<BindingChange<T>>,
    changed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct GenerationSummarySnapshot {
    id: u64,
    status: crate::app::GenerationStatus,
    scene: String,
    profile: String,
    binding_set_id: String,
    closure_id: String,
    enabled_features: Vec<String>,
    capabilities: Vec<String>,
    bindings: Vec<crate::app::AppBindingResolution>,
}

#[derive(Debug, Clone, Serialize)]
struct GenerationSummaryDiff {
    id: ScalarDiff<u64>,
    status: ScalarDiff<crate::app::GenerationStatus>,
    scene: ScalarDiff<String>,
    profile: ScalarDiff<String>,
    binding_set_id: ScalarDiff<String>,
    closure_id: ScalarDiff<String>,
    enabled_features: CollectionDiff<String>,
    capabilities: CollectionDiff<String>,
    bindings: BindingCollectionDiff<crate::app::AppBindingResolution>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct LockBindingSnapshot {
    capability: String,
    provider: String,
}

#[derive(Debug, Clone, Serialize)]
struct ImmutableLockDiff {
    scene: ScalarDiff<String>,
    profile: ScalarDiff<String>,
    closure_id: ScalarDiff<String>,
    packages: CollectionDiff<String>,
    bindings: BindingCollectionDiff<LockBindingSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct ImmutableLockComparison {
    available: bool,
    from_path: Option<String>,
    to_path: Option<String>,
    diff: Option<ImmutableLockDiff>,
}

#[derive(Debug, Clone, Serialize)]
struct GenerationDiffWarning {
    warning_type: String,
    generation: u64,
    path: Option<String>,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct GenerationDiffResponse {
    app: String,
    from: GenerationSummarySnapshot,
    to: GenerationSummarySnapshot,
    summary_diff: GenerationSummaryDiff,
    immutable_lock: ImmutableLockComparison,
    warnings: Vec<GenerationDiffWarning>,
}

fn resolved_app_instance_dir(app: &crate::app::ResolvedApp) -> Option<PathBuf> {
    app.sources
        .lock_path
        .as_deref()
        .and_then(|path| FsPath::new(path).parent().map(FsPath::to_path_buf))
        .or_else(|| {
            app.sources
                .config_path
                .as_deref()
                .and_then(|path| FsPath::new(path).parent().map(FsPath::to_path_buf))
        })
}

fn scalar_diff<T>(from: T, to: T) -> ScalarDiff<T>
where
    T: Serialize + PartialEq,
{
    let changed = from != to;
    ScalarDiff { from, to, changed }
}

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}

fn collection_diff(values_from: &[String], values_to: &[String]) -> CollectionDiff<String> {
    let from = sorted_strings(values_from);
    let to = sorted_strings(values_to);
    let from_set = from.iter().cloned().collect::<BTreeSet<_>>();
    let to_set = to.iter().cloned().collect::<BTreeSet<_>>();
    let added = to_set.difference(&from_set).cloned().collect::<Vec<_>>();
    let removed = from_set.difference(&to_set).cloned().collect::<Vec<_>>();
    let changed = !added.is_empty() || !removed.is_empty();

    CollectionDiff {
        from,
        to,
        added,
        removed,
        changed,
    }
}

fn normalized_bindings(
    bindings: &[crate::app::AppBindingResolution],
) -> Vec<crate::app::AppBindingResolution> {
    let mut bindings = bindings.to_vec();
    bindings.sort_by(|left, right| {
        left.capability
            .cmp(&right.capability)
            .then(left.provider.cmp(&right.provider))
            .then(left.mutable.cmp(&right.mutable))
            .then(left.source.cmp(&right.source))
    });
    bindings
}

fn binding_collection_diff(
    from_bindings: &[crate::app::AppBindingResolution],
    to_bindings: &[crate::app::AppBindingResolution],
) -> BindingCollectionDiff<crate::app::AppBindingResolution> {
    let from = normalized_bindings(from_bindings);
    let to = normalized_bindings(to_bindings);

    let from_by_capability = from
        .iter()
        .cloned()
        .map(|binding| (binding.capability.clone(), binding))
        .collect::<BTreeMap<_, _>>();
    let to_by_capability = to
        .iter()
        .cloned()
        .map(|binding| (binding.capability.clone(), binding))
        .collect::<BTreeMap<_, _>>();

    let added = to_by_capability
        .iter()
        .filter(|(capability, _)| !from_by_capability.contains_key(*capability))
        .map(|(_, binding)| binding.clone())
        .collect::<Vec<_>>();
    let removed = from_by_capability
        .iter()
        .filter(|(capability, _)| !to_by_capability.contains_key(*capability))
        .map(|(_, binding)| binding.clone())
        .collect::<Vec<_>>();
    let changed_entries = from_by_capability
        .iter()
        .filter_map(|(capability, from_binding)| {
            let to_binding = to_by_capability.get(capability)?;
            if from_binding == to_binding {
                None
            } else {
                Some(BindingChange {
                    capability: capability.clone(),
                    from: from_binding.clone(),
                    to: to_binding.clone(),
                })
            }
        })
        .collect::<Vec<_>>();
    let changed = !added.is_empty() || !removed.is_empty() || !changed_entries.is_empty();

    BindingCollectionDiff {
        from,
        to,
        added,
        removed,
        changed_entries,
        changed,
    }
}

fn generation_summary_snapshot(
    generation: &crate::app::AppGeneration,
) -> GenerationSummarySnapshot {
    GenerationSummarySnapshot {
        id: generation.id,
        status: generation.status,
        scene: generation.scene.clone(),
        profile: generation.profile.clone(),
        binding_set_id: generation.binding_set_id.clone(),
        closure_id: generation.closure_id.clone(),
        enabled_features: sorted_strings(&generation.enabled_features),
        capabilities: sorted_strings(&generation.capabilities),
        bindings: normalized_bindings(&generation.bindings),
    }
}

fn generation_summary_diff(
    from: &crate::app::AppGeneration,
    to: &crate::app::AppGeneration,
) -> GenerationSummaryDiff {
    GenerationSummaryDiff {
        id: scalar_diff(from.id, to.id),
        status: scalar_diff(from.status, to.status),
        scene: scalar_diff(from.scene.clone(), to.scene.clone()),
        profile: scalar_diff(from.profile.clone(), to.profile.clone()),
        binding_set_id: scalar_diff(from.binding_set_id.clone(), to.binding_set_id.clone()),
        closure_id: scalar_diff(from.closure_id.clone(), to.closure_id.clone()),
        enabled_features: collection_diff(&from.enabled_features, &to.enabled_features),
        capabilities: collection_diff(&from.capabilities, &to.capabilities),
        bindings: binding_collection_diff(&from.bindings, &to.bindings),
    }
}

fn normalized_lock_bindings(bindings: &[crate::app::AppLockBinding]) -> Vec<LockBindingSnapshot> {
    let mut bindings = bindings
        .iter()
        .map(|binding| LockBindingSnapshot {
            capability: binding.capability.clone(),
            provider: binding.provider.clone(),
        })
        .collect::<Vec<_>>();
    bindings.sort_by(|left, right| {
        left.capability
            .cmp(&right.capability)
            .then(left.provider.cmp(&right.provider))
    });
    bindings
}

fn lock_binding_collection_diff(
    from_bindings: &[crate::app::AppLockBinding],
    to_bindings: &[crate::app::AppLockBinding],
) -> BindingCollectionDiff<LockBindingSnapshot> {
    let from = normalized_lock_bindings(from_bindings);
    let to = normalized_lock_bindings(to_bindings);

    let from_by_capability = from
        .iter()
        .cloned()
        .map(|binding| (binding.capability.clone(), binding))
        .collect::<BTreeMap<_, _>>();
    let to_by_capability = to
        .iter()
        .cloned()
        .map(|binding| (binding.capability.clone(), binding))
        .collect::<BTreeMap<_, _>>();

    let added = to_by_capability
        .iter()
        .filter(|(capability, _)| !from_by_capability.contains_key(*capability))
        .map(|(_, binding)| binding.clone())
        .collect::<Vec<_>>();
    let removed = from_by_capability
        .iter()
        .filter(|(capability, _)| !to_by_capability.contains_key(*capability))
        .map(|(_, binding)| binding.clone())
        .collect::<Vec<_>>();
    let changed_entries = from_by_capability
        .iter()
        .filter_map(|(capability, from_binding)| {
            let to_binding = to_by_capability.get(capability)?;
            if from_binding == to_binding {
                None
            } else {
                Some(BindingChange {
                    capability: capability.clone(),
                    from: from_binding.clone(),
                    to: to_binding.clone(),
                })
            }
        })
        .collect::<Vec<_>>();
    let changed = !added.is_empty() || !removed.is_empty() || !changed_entries.is_empty();

    BindingCollectionDiff {
        from,
        to,
        added,
        removed,
        changed_entries,
        changed,
    }
}

fn immutable_lock_diff(
    from: &crate::app::InstanceLock,
    to: &crate::app::InstanceLock,
) -> ImmutableLockDiff {
    let from_packages = from
        .packages
        .iter()
        .map(crate::app::AppLockPackage::identity)
        .collect::<Vec<_>>();
    let to_packages = to
        .packages
        .iter()
        .map(crate::app::AppLockPackage::identity)
        .collect::<Vec<_>>();

    ImmutableLockDiff {
        scene: scalar_diff(from.scene.clone(), to.scene.clone()),
        profile: scalar_diff(from.profile.clone(), to.profile.clone()),
        closure_id: scalar_diff(from.closure_id.clone(), to.closure_id.clone()),
        packages: collection_diff(&from_packages, &to_packages),
        bindings: lock_binding_collection_diff(&from.bindings, &to.bindings),
    }
}

fn generation_diff_not_found(
    app_name: &str,
    generation_id: u64,
    role: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": format!(
                "Generation {} not found for app '{}'",
                generation_id, app_name
            ),
            "reason": "generation_not_found",
            "app": app_name,
            "generation": generation_id,
            "role": role,
        })),
    )
}

fn load_generation_lock_for_diff(
    generation: &crate::app::AppGeneration,
    instance_dir: Option<&FsPath>,
) -> Result<GenerationLockDiffData, anyhow::Error> {
    let Some(instance_dir) = instance_dir else {
        return Ok((
            None,
            None,
            Some(GenerationDiffWarning {
                warning_type: "instance_dir_unavailable".into(),
                generation: generation.id,
                path: None,
                message: format!(
                    "Resolved instance directory is unavailable for generation {}; immutable lock diff was skipped",
                    generation.id
                ),
            }),
        ));
    };

    if generation.lock_path.trim().is_empty() {
        return Ok((
            None,
            None,
            Some(GenerationDiffWarning {
                warning_type: "missing_lock_path_metadata".into(),
                generation: generation.id,
                path: None,
                message: format!(
                    "Generation {} is missing immutable lock_path metadata; immutable lock diff was skipped",
                    generation.id
                ),
            }),
        ));
    }

    let lock_path = resolved_generation_lock_path(instance_dir, &generation.lock_path);
    let lock_path_display = lock_path.display().to_string();
    match crate::app::load_instance_lock_from_path(&lock_path) {
        Ok(lock) => Ok((Some(lock), Some(lock_path_display), None)),
        Err(error) if is_not_found_error(&error) => Ok((
            None,
            Some(lock_path_display.clone()),
            Some(GenerationDiffWarning {
                warning_type: "immutable_lock_missing".into(),
                generation: generation.id,
                path: Some(lock_path_display),
                message: format!(
                    "Immutable generation lock for generation {} is missing; returning summary diff only",
                    generation.id
                ),
            }),
        )),
        Err(error) => Err(error),
    }
}

type GenerationLockDiffData = (
    Option<crate::app::InstanceLock>,
    Option<String>,
    Option<GenerationDiffWarning>,
);

fn persist_generation_index_best_effort(
    app_name: &str,
    app_store: &crate::app::AppGenerationStore,
    instance_dir: &FsPath,
) -> (bool, Option<String>) {
    let index_path = crate::app::generation_index_path(instance_dir);
    let index = crate::app::AppGenerationIndex::from_store(app_store);
    match crate::app::save_generation_index(instance_dir, &index) {
        Ok(()) => (true, None),
        Err(error) => {
            let message = format!(
                "Failed to persist repairable generation index for '{}' at '{}': {error:#}",
                app_name,
                index_path.display()
            );
            tracing::warn!("{}", message);
            (false, Some(message))
        }
    }
}

fn write_generation_pointers(
    app_name: &str,
    instance_dir: &FsPath,
    previous_generation_id: Option<u64>,
    active_generation_id: u64,
    generation_lock_written: bool,
    failed_status: &str,
    operation_label: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if let Err(error) =
        crate::app::write_previous_generation_pointer(instance_dir, previous_generation_id)
    {
        let message = format!(
            "Failed to write previous generation pointer for '{}' at '{}': {error:#}",
            app_name,
            crate::app::previous_generation_pointer_path(instance_dir).display()
        );
        tracing::warn!("{}", message);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": message,
                "reason": "pointer_write_failed",
                "status": failed_status,
                "generation_lock_written": generation_lock_written,
                "operation": operation_label,
                "lock_written": true,
                "pointer_written": false,
                "pointer_error": {
                    "stage": "previous",
                    "repair_needed": false,
                    "message": format!(
                        "Previous generation pointer write failed before active pointer update for '{}' during {}",
                        app_name,
                        operation_label
                    ),
                    "details": format!("{error:#}"),
                },
                "index_written": false,
                "index_error": serde_json::Value::Null,
            })),
        ));
    }

    if let Err(error) =
        crate::app::write_active_generation_pointer(instance_dir, Some(active_generation_id))
    {
        let message = format!(
            "Failed to write active generation pointer for '{}' at '{}': {error:#}",
            app_name,
            crate::app::active_generation_pointer_path(instance_dir).display()
        );
        tracing::warn!("{}", message);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": message,
                "reason": "pointer_write_failed",
                "status": failed_status,
                "generation_lock_written": generation_lock_written,
                "operation": operation_label,
                "lock_written": true,
                "pointer_written": false,
                "pointer_error": {
                    "stage": "active",
                    "repair_needed": true,
                    "message": format!(
                        "Active generation pointer write failed after previous pointer update for '{}' during {}; repair is required before relying on pointer files",
                        app_name,
                        operation_label
                    ),
                    "details": format!("{error:#}"),
                },
                "index_written": false,
                "index_error": serde_json::Value::Null,
            })),
        ));
    }

    Ok(())
}

fn generation_pointer_snapshot(path: PathBuf) -> GenerationPointerSnapshot {
    match crate::app::read_generation_pointer(&path) {
        Ok(generation_id) => GenerationPointerSnapshot {
            path: path.display().to_string(),
            generation_id,
            read_error: None,
        },
        Err(error) => GenerationPointerSnapshot {
            path: path.display().to_string(),
            generation_id: None,
            read_error: Some(format!("{error:#}")),
        },
    }
}

fn generation_index_snapshot(instance_dir: &FsPath) -> GenerationIndexSnapshot {
    let path = crate::app::generation_index_path(instance_dir);
    match crate::app::load_generation_index_from_path(&path) {
        Ok(index) => GenerationIndexSnapshot {
            path: path.display().to_string(),
            present: path.exists(),
            index,
            read_error: None,
        },
        Err(error) => GenerationIndexSnapshot {
            path: path.display().to_string(),
            present: path.exists(),
            index: None,
            read_error: Some(format!("{error:#}")),
        },
    }
}

fn instance_generation_diagnostics(instance_dir: PathBuf) -> InstanceGenerationDiagnostics {
    let active_pointer =
        generation_pointer_snapshot(crate::app::active_generation_pointer_path(&instance_dir));
    let previous_pointer =
        generation_pointer_snapshot(crate::app::previous_generation_pointer_path(&instance_dir));
    let generation_index = generation_index_snapshot(&instance_dir);
    let consistency_report =
        generation_index.index.as_ref().and_then(|index| {
            if active_pointer.read_error.is_none() && previous_pointer.read_error.is_none() {
                Some(index.consistency_report(
                    active_pointer.generation_id,
                    previous_pointer.generation_id,
                ))
            } else {
                None
            }
        });

    InstanceGenerationDiagnostics {
        instance_dir: instance_dir.display().to_string(),
        active_pointer,
        previous_pointer,
        generation_index,
        consistency_report,
    }
}

async fn generation_diagnostics_snapshot(
    state: &AppState,
    name: &str,
) -> Result<GenerationDiagnosticsResponse, (StatusCode, Json<serde_json::Value>)> {
    let app = {
        let apps = state.resolved_apps.read().await;
        apps.get(name).cloned().ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("App '{}' not found", name)})),
            )
        })?
    };

    let (store_present, store_summary) = {
        let store = state.generation_store.read().await;
        match store.get(name) {
            Some(app_store) => (true, crate::app::AppGenerationIndex::from_store(app_store)),
            None => (false, crate::app::AppGenerationIndex::default()),
        }
    };

    Ok(GenerationDiagnosticsResponse {
        app: name.to_string(),
        store_present,
        store_summary,
        instance: resolved_app_instance_dir(&app).map(instance_generation_diagnostics),
    })
}

fn requires_real_package_source(package_name: &str) -> bool {
    WEFT_CLAW_REQUIRED_REAL_PACKAGES.contains(&package_name)
}

fn package_matches_required_real_source(pkg: &crate::app::PackageSource) -> bool {
    !requires_real_package_source(&pkg.name) || pkg.current_source.starts_with("packages/official/")
}

fn generation_declares_real_package(
    state: &AppState,
    generation: &crate::app::AppGeneration,
    provider: &str,
) -> bool {
    let Some(pkg) = state.package_index.get(provider) else {
        return provider == "core";
    };

    if !requires_real_package_source(&pkg.name) {
        return true;
    }

    if !package_matches_required_real_source(pkg) {
        return false;
    }

    generation.bindings.iter().any(|binding| {
        binding.provider == provider
            || state
                .package_index
                .get(&binding.provider)
                .map(|binding_pkg| binding_pkg.name == pkg.name)
                .unwrap_or(false)
    })
}

fn candidate_wasm_package_dirs(state: &AppState, provider: &str) -> Vec<std::path::PathBuf> {
    crate::package::discover_runtime_package(&state.repo_root, &state.package_index, provider)
        .map(|package| vec![package.dir])
        .unwrap_or_default()
}

fn build_wasm_package_artifact(package_dir: &FsPath) -> Result<(), String> {
    let manifest_path = package_dir.join("Cargo.toml");
    if !manifest_path.exists() {
        return Err(format!(
            "No Cargo.toml found for wasm provider source at '{}'",
            package_dir.display()
        ));
    }

    let output = std::process::Command::new("cargo")
        .arg("build")
        .arg("--target")
        .arg("wasm32-wasip1")
        .current_dir(package_dir)
        .output();

    match output {
        Ok(result) if result.status.success() => Ok(()),
        Ok(result) => Err(format!(
            "cargo build failed for '{}': {}",
            package_dir.display(),
            String::from_utf8_lossy(&result.stderr)
        )),
        Err(error) => Err(format!(
            "Failed to spawn cargo build for '{}': {}",
            package_dir.display(),
            error
        )),
    }
}

fn locate_built_wasm(package_dir: &FsPath) -> Option<std::path::PathBuf> {
    let target_dir = package_dir
        .join("target")
        .join("wasm32-wasip1")
        .join("debug");
    if !target_dir.exists() {
        return None;
    }
    let entries = std::fs::read_dir(target_dir).ok()?;
    let mut wasm_files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("wasm"))
        .collect();
    wasm_files.sort();
    wasm_files.into_iter().next()
}

async fn probe_wasm_provider_via_temp_load(
    state: &AppState,
    provider: &str,
    capability: &str,
    app_name: &str,
) -> crate::app::ValidationResult {
    let package_dirs = candidate_wasm_package_dirs(state, provider);
    if package_dirs.is_empty() {
        return crate::app::ValidationResult {
            check: format!("probe:{}", capability),
            passed: true,
            message: "Live probe skipped for wasm runtime because provider directory could not be located".into(),
        };
    }

    let handle_guard = state.wasm_handle.write().await;
    let Some(handle) = handle_guard.as_ref().cloned() else {
        return crate::app::ValidationResult {
            check: format!("probe:{}", capability),
            passed: true,
            message: "Live probe skipped for wasm runtime because no wasm runtime is active".into(),
        };
    };

    let already_loaded = handle.has_package(provider);
    let mut loaded_by_probe = false;

    if !already_loaded {
        let mut loaded = false;
        let mut build_failures = Vec::new();
        for package_dir in package_dirs {
            let Ok(manifest) = crate::package::config::load_manifest(&package_dir) else {
                continue;
            };
            let entry = manifest
                .resolved_entry()
                .map(|entry| package_dir.join(entry))
                .filter(|path| path.exists())
                .or_else(|| match build_wasm_package_artifact(&package_dir) {
                    Ok(()) => locate_built_wasm(&package_dir),
                    Err(error) => {
                        build_failures.push(error);
                        None
                    }
                });
            let Some(wasm_path) = entry else {
                continue;
            };
            let load_info = crate::package::bridge::PackageLoadInfo {
                name: provider.to_string(),
                dir: package_dir.clone(),
                wasm_path,
                startup_mode: crate::package::resolve_wasm_startup_mode(&manifest),
                permissions: manifest.permissions.clone(),
            };
            if handle.load_package(&load_info).is_ok() {
                loaded = true;
                loaded_by_probe = true;
                break;
            }
        }

        if !loaded {
            return crate::app::ValidationResult {
                check: format!("probe:{}", capability),
                passed: build_failures.is_empty(),
                message: if build_failures.is_empty() {
                    "Live probe skipped for wasm runtime because provider could not be temporarily loaded".into()
                } else {
                    format!(
                        "Live probe failed because wasm provider could not be built or loaded: {}",
                        build_failures.join(" | ")
                    )
                },
            };
        }
    }

    drop(handle_guard);

    let probe_payload = serde_json::json!({
        "action": "describe",
        "data": {},
        "app": app_name,
        "provider": provider,
    });
    let result =
        crate::api::capabilities::execute_capability_call(state, capability, probe_payload).await;

    if loaded_by_probe {
        let handle_guard = state.wasm_handle.read().await;
        if let Some(handle) = handle_guard.as_ref() {
            let _ = handle.unload_package(provider);
        }
    }

    match result {
        Ok(_) => crate::app::ValidationResult {
            check: format!("probe:{}", capability),
            passed: true,
            message: "Provider responded to live wasm probe".into(),
        },
        Err((status, value)) => {
            let body = value.to_string();
            let skipped_unknown_action = status == StatusCode::BAD_GATEWAY
                && body.to_lowercase().contains("unknown action")
                && body.contains("describe");
            crate::app::ValidationResult {
                check: format!("probe:{}", capability),
                passed: skipped_unknown_action,
                message: if skipped_unknown_action {
                    "Live probe skipped because provider does not implement generic describe action"
                        .into()
                } else {
                    format!("Live probe failed with {}: {}", status, value)
                },
            }
        }
    }
}

pub fn package_digest(repo_root: &FsPath, source: &str) -> String {
    let path = repo_root.join(source);
    let mut hasher = Sha512::new();

    fn update_dir(hasher: &mut Sha512, root: &FsPath, path: &FsPath) -> Result<(), std::io::Error> {
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
            let relative = entry
                .strip_prefix(root)
                .unwrap_or(&entry)
                .to_string_lossy()
                .replace('\\', "/");
            if entry.is_dir() {
                hasher.update(relative.as_bytes());
                update_dir(hasher, root, &entry)?;
            } else if entry.is_file() {
                hasher.update(relative.as_bytes());
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
        if update_dir(&mut hasher, &path, &path).is_err() {
            hasher.update(format!("unreadable:{}", source).as_bytes());
        }
    } else {
        hasher.update(format!("missing:{}", source).as_bytes());
    }

    format!("{:x}", hasher.finalize())
}

fn packages_for_generation(
    state: &AppState,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::AppLockPackage> {
    generation
        .bindings
        .iter()
        .filter_map(|binding| {
            state.package_index.get(&binding.provider).map(|pkg| {
                let digest = package_digest(&state.repo_root, &pkg.current_source);
                crate::app::AppLockPackage {
                    name: pkg.name.clone(),
                    version: "current".into(),
                    runtime: pkg.kind.clone(),
                    sha512: digest.clone(),
                    source: pkg.current_source.clone(),
                    trusted: pkg.trusted,
                    signature: pkg.signature.clone(),
                    source_authority: pkg.source_authority.clone(),
                    source_public_keys: pkg.source_public_keys.clone(),
                    package_kind: pkg.package_kind.clone(),
                    entry_kind: pkg.kind.clone(),
                    runtime_provider: pkg.runtime_provider_name(),
                    provides: pkg.provides.clone(),
                    requires: pkg.requires.clone(),
                    capabilities: pkg.provides.clone(),
                    manifest_digest: digest.clone(),
                    artifact_digest: digest.clone(),
                    artifact_set_id: String::new(),
                    store_object_id: format!("sha512-{}-{}-current", digest, pkg.name),
                    store_path: String::new(),
                    closure_id: String::new(),
                    features: vec![],
                    default_enabled_features: vec![],
                    roles: pkg
                        .package_kind
                        .split(',')
                        .map(|role| role.trim().to_string())
                        .filter(|role| !role.is_empty())
                        .collect(),
                    evidence: crate::app::AppLockEvidence {
                        digest,
                        signature: pkg.signature.clone(),
                        source_authority: pkg.source_authority.clone(),
                        source_public_keys: pkg.source_public_keys.clone(),
                    },
                    reasons: vec![],
                }
            })
        })
        .fold(Vec::<crate::app::AppLockPackage>::new(), |mut acc, pkg| {
            if !acc
                .iter()
                .any(|existing| existing.identity() == pkg.identity())
            {
                acc.push(pkg);
            }
            acc
        })
}

fn generation_lock_path(generation_id: u64) -> String {
    format!("generations/{generation_id}.lock.toml")
}

fn resolved_generation_lock_path(instance_dir: &FsPath, lock_path: &str) -> PathBuf {
    let lock_path = FsPath::new(lock_path);
    if lock_path.is_absolute() {
        lock_path.to_path_buf()
    } else {
        instance_dir.join(lock_path)
    }
}

#[derive(Debug)]
enum ImmutableGenerationLockError {
    MissingLockPath,
    Serialize(String),
    ReadExisting { path: String, details: String },
    CreateParent { path: String, details: String },
    WriteNew { path: String, details: String },
    Conflict { path: String },
}

fn write_immutable_generation_lock(
    instance_dir: &FsPath,
    generation: &crate::app::AppGeneration,
    lock_file: &crate::app::AppLockFile,
) -> Result<(), ImmutableGenerationLockError> {
    if generation.lock_path.trim().is_empty() {
        return Err(ImmutableGenerationLockError::MissingLockPath);
    }

    let path = resolved_generation_lock_path(instance_dir, &generation.lock_path);
    let path_display = path.display().to_string();
    let content = toml::to_string_pretty(lock_file).map_err(|error| {
        ImmutableGenerationLockError::Serialize(format!(
            "Failed to serialize immutable generation lock for generation {}: {error}",
            generation.id
        ))
    })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            ImmutableGenerationLockError::CreateParent {
                path: parent.display().to_string(),
                details: error.to_string(),
            }
        })?;
    }

    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => file.write_all(content.as_bytes()).map_err(|error| {
            ImmutableGenerationLockError::WriteNew {
                path: path_display,
                details: error.to_string(),
            }
        }),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let existing = std::fs::read_to_string(&path).map_err(|read_error| {
                ImmutableGenerationLockError::ReadExisting {
                    path: path.display().to_string(),
                    details: read_error.to_string(),
                }
            })?;

            if existing == content {
                Ok(())
            } else {
                Err(ImmutableGenerationLockError::Conflict {
                    path: path.display().to_string(),
                })
            }
        }
        Err(error) => Err(ImmutableGenerationLockError::WriteNew {
            path: path.display().to_string(),
            details: error.to_string(),
        }),
    }
}

fn immutable_generation_lock_failure_response(
    app_name: &str,
    generation_id: u64,
    error: ImmutableGenerationLockError,
) -> (StatusCode, Json<serde_json::Value>) {
    match error {
        ImmutableGenerationLockError::MissingLockPath => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!(
                    "Generation {} is missing lock_path metadata required to persist immutable activation lock",
                    generation_id
                ),
                "reason": "generation_lock_write_failed",
                "status": "activation_failed",
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
                "generation_lock_error": {
                    "type": "missing_lock_path",
                    "message": format!(
                        "Generation {} for '{}' is missing lock_path metadata",
                        generation_id, app_name
                    ),
                }
            })),
        ),
        ImmutableGenerationLockError::Serialize(details) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!(
                    "Failed to serialize immutable generation lock for '{}' generation {}",
                    app_name, generation_id
                ),
                "reason": "generation_lock_write_failed",
                "status": "activation_failed",
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
                "generation_lock_error": {
                    "type": "serialize_failed",
                    "message": format!(
                        "Immutable generation lock serialization failed for '{}' generation {}",
                        app_name, generation_id
                    ),
                    "details": details,
                }
            })),
        ),
        ImmutableGenerationLockError::ReadExisting { path, details } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!(
                    "Failed to read existing immutable generation lock for '{}' at '{}'",
                    app_name, path
                ),
                "reason": "generation_lock_write_failed",
                "status": "activation_failed",
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
                "generation_lock_error": {
                    "type": "read_existing_failed",
                    "path": path,
                    "message": format!(
                        "Existing immutable generation lock for '{}' could not be read before activation",
                        app_name
                    ),
                    "details": details,
                }
            })),
        ),
        ImmutableGenerationLockError::CreateParent { path, details } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!(
                    "Failed to prepare immutable generation lock directory for '{}' at '{}'",
                    app_name, path
                ),
                "reason": "generation_lock_write_failed",
                "status": "activation_failed",
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
                "generation_lock_error": {
                    "type": "prepare_parent_failed",
                    "path": path,
                    "message": format!(
                        "Immutable generation lock directory setup failed for '{}'",
                        app_name
                    ),
                    "details": details,
                }
            })),
        ),
        ImmutableGenerationLockError::WriteNew { path, details } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!(
                    "Failed to write immutable generation lock for '{}' at '{}'",
                    app_name, path
                ),
                "reason": "generation_lock_write_failed",
                "status": "activation_failed",
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
                "generation_lock_error": {
                    "type": "write_failed",
                    "path": path,
                    "message": format!(
                        "Immutable generation lock write failed for '{}'",
                        app_name
                    ),
                    "details": details,
                }
            })),
        ),
        ImmutableGenerationLockError::Conflict { path } => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "Immutable generation lock for '{}' generation {} already exists with different content at '{}'",
                    app_name, generation_id, path
                ),
                "reason": "generation_lock_conflict",
                "status": "activation_failed",
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
                "generation_lock_error": {
                    "type": "content_conflict",
                    "path": path,
                    "message": format!(
                        "Immutable generation lock content for '{}' generation {} differs from the activation lock payload",
                        app_name, generation_id
                    ),
                }
            })),
        ),
    }
}

fn rollback_fallback_lock(
    existing_lock: crate::app::AppLockFile,
    generation: &crate::app::AppGeneration,
) -> crate::app::AppLockFile {
    crate::app::AppLockFile {
        generation: generation.id as u32,
        status: "active".into(),
        profile: generation.profile.clone(),
        scene: existing_lock.scene.clone(),
        scene_digest: existing_lock.scene_digest.clone(),
        binding_set_id: existing_lock.binding_set_id.clone(),
        closure_id: existing_lock.closure_id.clone(),
        closure_digest: existing_lock.closure_digest.clone(),
        assembly: crate::app::AppLockAssembly {
            enabled_features: generation.enabled_features.clone(),
            selected_packages: existing_lock.assembly.selected_packages.clone(),
            scene: existing_lock.assembly.scene.clone(),
            scene_digest: existing_lock.assembly.scene_digest.clone(),
            binding_set_id: existing_lock.assembly.binding_set_id.clone(),
            closure_id: existing_lock.assembly.closure_id.clone(),
            closure_digest: existing_lock.assembly.closure_digest.clone(),
        },
        bindings: generation
            .bindings
            .iter()
            .map(|binding| crate::app::AppLockBinding {
                capability: binding.capability.clone(),
                provider: binding.provider.clone(),
                package: binding.provider.clone(),
                mutable: binding.mutable,
                package_version: String::new(),
                package_sha512: String::new(),
                binding_source: binding.source.clone(),
            })
            .collect(),
        binding_sources: generation
            .bindings
            .iter()
            .map(|binding| crate::app::AppLockBindingSource {
                capability: binding.capability.clone(),
                source: binding.source.clone(),
                package: binding.provider.clone(),
            })
            .collect(),
        notes: crate::app::AppLockNotes {
            message: format!("Rolled back to generation {} via API", generation.id),
        },
        ..existing_lock
    }
}

fn is_not_found_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io_error| io_error.kind() == ErrorKind::NotFound)
    })
}

struct RollbackRootLockResolution {
    lock_file: crate::app::AppLockFile,
    generation_lock_replayed: bool,
    generation_lock_warning: Option<serde_json::Value>,
}

fn rollback_generation_lock_failure_response(
    app_name: &str,
    generation: &crate::app::AppGeneration,
    error_type: &str,
    message: String,
    path: Option<String>,
    details: Option<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "error": message,
            "reason": "generation_lock_replay_failed",
            "status": "rollback_failed",
            "lock_written": false,
            "pointer_written": false,
            "pointer_error": serde_json::Value::Null,
            "index_written": false,
            "index_error": serde_json::Value::Null,
            "generation_lock_replayed": false,
            "generation_lock_error": {
                "type": error_type,
                "app": app_name,
                "generation": generation.id,
                "path": path,
                "details": details,
            }
        })),
    )
}

fn activation_generation_lock_failure_response(
    app_name: &str,
    generation: &crate::app::AppGeneration,
    error_type: &str,
    message: String,
    path: Option<String>,
    details: Option<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "error": message,
            "reason": "generation_lock_replay_failed",
            "status": "activation_failed",
            "lock_written": false,
            "pointer_written": false,
            "pointer_error": serde_json::Value::Null,
            "index_written": false,
            "index_error": serde_json::Value::Null,
            "generation_lock_replayed": false,
            "generation_lock_error": {
                "type": error_type,
                "app": app_name,
                "generation": generation.id,
                "path": path,
                "details": details,
            }
        })),
    )
}

fn resolve_rollback_root_lock(
    app_name: &str,
    generation: &crate::app::AppGeneration,
    instance_dir: Option<&FsPath>,
    existing_lock: crate::app::AppLockFile,
) -> Result<RollbackRootLockResolution, (StatusCode, Json<serde_json::Value>)> {
    let fallback_lock = rollback_fallback_lock(existing_lock, generation);
    let Some(instance_dir) = instance_dir else {
        return Ok(RollbackRootLockResolution {
            lock_file: fallback_lock,
            generation_lock_replayed: false,
            generation_lock_warning: None,
        });
    };

    if generation.lock_path.trim().is_empty() {
        return Ok(RollbackRootLockResolution {
            lock_file: fallback_lock,
            generation_lock_replayed: false,
            generation_lock_warning: Some(serde_json::json!({
                "type": "missing_lock_path_metadata",
                "message": format!(
                    "Rollback generation {} for '{}' is missing immutable lock_path metadata; using compatibility fallback root lock rebuild",
                    generation.id, app_name
                ),
            })),
        });
    }

    let immutable_lock_path = resolved_generation_lock_path(instance_dir, &generation.lock_path);
    let immutable_lock_path_display = immutable_lock_path.display().to_string();
    match crate::app::load_instance_lock_from_path(&immutable_lock_path) {
        Ok(mut immutable_lock) => {
            let expected_app = if generation.app_name.trim().is_empty() {
                app_name
            } else {
                generation.app_name.as_str()
            };

            if immutable_lock.generation != generation.id as u32 {
                return Err(rollback_generation_lock_failure_response(
                    app_name,
                    generation,
                    "generation_mismatch",
                    format!(
                        "Immutable rollback generation lock at '{}' belongs to generation {} instead of {}",
                        immutable_lock_path_display, immutable_lock.generation, generation.id
                    ),
                    Some(immutable_lock_path_display),
                    None,
                ));
            }

            if immutable_lock.app != expected_app {
                return Err(rollback_generation_lock_failure_response(
                    app_name,
                    generation,
                    "app_mismatch",
                    format!(
                        "Immutable rollback generation lock at '{}' belongs to app '{}' instead of '{}'",
                        immutable_lock_path_display, immutable_lock.app, expected_app
                    ),
                    Some(immutable_lock_path_display),
                    None,
                ));
            }

            immutable_lock.status = "active".into();
            Ok(RollbackRootLockResolution {
                lock_file: immutable_lock,
                generation_lock_replayed: true,
                generation_lock_warning: None,
            })
        }
        Err(error) if is_not_found_error(&error) => Ok(RollbackRootLockResolution {
            lock_file: fallback_lock,
            generation_lock_replayed: false,
            generation_lock_warning: Some(serde_json::json!({
                "type": "immutable_lock_missing",
                "path": immutable_lock_path_display,
                "message": format!(
                    "Immutable rollback generation lock for '{}' generation {} is missing; using compatibility fallback root lock rebuild",
                    app_name, generation.id
                ),
            })),
        }),
        Err(error) => Err(rollback_generation_lock_failure_response(
            app_name,
            generation,
            "load_failed",
            format!(
                "Failed to load immutable rollback generation lock for '{}' generation {} from '{}': {error:#}",
                app_name, generation.id, immutable_lock_path_display
            ),
            Some(immutable_lock_path_display),
            Some(format!("{error:#}")),
        )),
    }
}

fn resolve_activation_root_lock(
    app_name: &str,
    generation: &crate::app::AppGeneration,
    instance_dir: Option<&FsPath>,
    existing_lock: crate::app::AppLockFile,
) -> Result<RollbackRootLockResolution, (StatusCode, Json<serde_json::Value>)> {
    let fallback_lock = rollback_fallback_lock(existing_lock, generation);
    let Some(instance_dir) = instance_dir else {
        return Ok(RollbackRootLockResolution {
            lock_file: fallback_lock,
            generation_lock_replayed: false,
            generation_lock_warning: None,
        });
    };

    if generation.lock_path.trim().is_empty() {
        return Ok(RollbackRootLockResolution {
            lock_file: fallback_lock,
            generation_lock_replayed: false,
            generation_lock_warning: Some(serde_json::json!({
                "type": "missing_lock_path_metadata",
                "message": format!(
                    "Activation target generation {} for '{}' is missing immutable lock_path metadata; using compatibility fallback root lock rebuild",
                    generation.id, app_name
                ),
            })),
        });
    }

    let immutable_lock_path = resolved_generation_lock_path(instance_dir, &generation.lock_path);
    let immutable_lock_path_display = immutable_lock_path.display().to_string();
    match crate::app::load_instance_lock_from_path(&immutable_lock_path) {
        Ok(mut immutable_lock) => {
            let expected_app = if generation.app_name.trim().is_empty() {
                app_name
            } else {
                generation.app_name.as_str()
            };

            if immutable_lock.generation != generation.id as u32 {
                return Err(activation_generation_lock_failure_response(
                    app_name,
                    generation,
                    "generation_mismatch",
                    format!(
                        "Immutable activation generation lock at '{}' belongs to generation {} instead of {}",
                        immutable_lock_path_display, immutable_lock.generation, generation.id
                    ),
                    Some(immutable_lock_path_display),
                    None,
                ));
            }

            if immutable_lock.app != expected_app {
                return Err(activation_generation_lock_failure_response(
                    app_name,
                    generation,
                    "app_mismatch",
                    format!(
                        "Immutable activation generation lock at '{}' belongs to app '{}' instead of '{}'",
                        immutable_lock_path_display, immutable_lock.app, expected_app
                    ),
                    Some(immutable_lock_path_display),
                    None,
                ));
            }

            immutable_lock.status = "active".into();
            Ok(RollbackRootLockResolution {
                lock_file: immutable_lock,
                generation_lock_replayed: true,
                generation_lock_warning: None,
            })
        }
        Err(error) if is_not_found_error(&error) => Ok(RollbackRootLockResolution {
            lock_file: fallback_lock,
            generation_lock_replayed: false,
            generation_lock_warning: Some(serde_json::json!({
                "type": "immutable_lock_missing",
                "path": immutable_lock_path_display,
                "message": format!(
                    "Immutable activation generation lock for '{}' generation {} is missing; using compatibility fallback root lock rebuild",
                    app_name, generation.id
                ),
            })),
        }),
        Err(error) => Err(activation_generation_lock_failure_response(
            app_name,
            generation,
            "load_failed",
            format!(
                "Failed to load immutable activation generation lock for '{}' generation {} from '{}': {error:#}",
                app_name, generation.id, immutable_lock_path_display
            ),
            Some(immutable_lock_path_display),
            Some(format!("{error:#}")),
        )),
    }
}

fn active_scene_config(
    app_config: &crate::app::AppConfigFile,
) -> Option<&crate::app::AppSceneConfig> {
    let active_scene = app_config.active_scene.trim();
    if active_scene.is_empty() {
        return None;
    }

    app_config.scenes.get(active_scene)
}

fn overlay_scene_enabled_features(
    enabled_features: &[String],
    scene: Option<&crate::app::AppSceneConfig>,
) -> Vec<String> {
    let Some(scene) = scene else {
        return enabled_features.to_vec();
    };

    let mut merged = enabled_features.iter().cloned().collect::<BTreeSet<_>>();
    for feature in &scene.enabled_features {
        merged.insert(feature.clone());
    }
    for feature in &scene.disabled_features {
        merged.remove(feature);
    }

    merged.into_iter().collect()
}

fn generation_scene_digest(app_config: &crate::app::AppConfigFile, scene_name: &str) -> String {
    if scene_name.trim().is_empty() {
        return String::new();
    }

    app_config
        .scenes
        .values()
        .find(|scene| scene.name == scene_name)
        .or_else(|| app_config.scenes.get(scene_name))
        .map(scene_digest)
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
struct ScenePinConflictDetail {
    conflict_type: &'static str,
    scene: String,
    capability: Option<String>,
    package: Option<String>,
    indexes: Vec<usize>,
    message: String,
}

fn normalized_pin_field(value: &str) -> String {
    value.trim().to_string()
}

fn non_empty_pin_field(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn scene_pin_conflict_response(
    detail: ScenePinConflictDetail,
) -> (StatusCode, Json<serde_json::Value>) {
    let ScenePinConflictDetail {
        conflict_type,
        scene,
        capability,
        package,
        indexes,
        message,
    } = detail;

    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(serde_json::json!({
            "error": message,
            "reason": "scene_pin_conflict",
            "conflict": {
                "type": conflict_type,
                "scene": scene,
                "capability": capability,
                "package": package,
                "indexes": indexes,
            }
        })),
    )
}

fn scene_package_pin_unresolved_response(
    scene: &str,
    package: &str,
    index: usize,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(serde_json::json!({
            "error": format!(
                "Scene package pin for '{}' cannot be resolved from the local package index",
                package
            ),
            "reason": "scene_package_pin_unresolved",
            "scene": scene,
            "package": package,
            "index": index,
        })),
    )
}

fn validate_scene_pin_conflicts(
    scene_name: &str,
    scene: &crate::app::AppSceneConfig,
) -> Result<(), Box<ScenePinConflictDetail>> {
    let resolved_scene_name = if scene.name.trim().is_empty() {
        scene_name.trim().to_string()
    } else {
        scene.name.trim().to_string()
    };

    let mut binding_by_capability =
        BTreeMap::<String, (usize, (String, String, String, String))>::new();
    for (index, binding_pin) in scene.binding_pins.iter().enumerate() {
        let capability = normalized_pin_field(&binding_pin.capability);
        if capability.is_empty() {
            continue;
        }

        let identity = (
            normalized_pin_field(&binding_pin.package),
            normalized_pin_field(&binding_pin.provider),
            normalized_pin_field(&binding_pin.version),
            normalized_pin_field(&binding_pin.sha512),
        );

        if let Some((existing_index, existing_identity)) = binding_by_capability.get(&capability) {
            if existing_identity != &identity {
                return Err(Box::new(ScenePinConflictDetail {
                    conflict_type: "duplicate_binding_pin",
                    scene: resolved_scene_name,
                    capability: Some(capability),
                    package: None,
                    indexes: vec![*existing_index, index],
                    message: format!(
                        "Scene binding pins for capability '{}' declare conflicting package/provider/version/hash identity",
                        binding_pin.capability.trim()
                    ),
                }));
            }
        } else {
            binding_by_capability.insert(capability, (index, identity));
        }
    }

    let mut package_by_name = BTreeMap::<String, (usize, (String, String, String))>::new();
    for (index, package_pin) in scene.package_pins.iter().enumerate() {
        let package = normalized_pin_field(&package_pin.package);
        if package.is_empty() {
            continue;
        }

        let identity = (
            normalized_pin_field(&package_pin.version),
            normalized_pin_field(&package_pin.sha512),
            normalized_pin_field(&package_pin.source),
        );

        if let Some((existing_index, existing_identity)) = package_by_name.get(&package) {
            if existing_identity != &identity {
                return Err(Box::new(ScenePinConflictDetail {
                    conflict_type: "duplicate_package_pin",
                    scene: resolved_scene_name,
                    capability: None,
                    package: Some(package),
                    indexes: vec![*existing_index, index],
                    message: format!(
                        "Scene package pins for package '{}' declare conflicting version/hash/source identity",
                        package_pin.package.trim()
                    ),
                }));
            }
        } else {
            package_by_name.insert(package, (index, identity));
        }
    }

    let package_pin_indexes = scene.package_pins.iter().enumerate().fold(
        BTreeMap::<String, Vec<usize>>::new(),
        |mut acc, (index, package_pin)| {
            let package = normalized_pin_field(&package_pin.package);
            if !package.is_empty() {
                acc.entry(package).or_default().push(index);
            }
            acc
        },
    );

    for (binding_index, binding_pin) in scene.binding_pins.iter().enumerate() {
        let package = normalized_pin_field(&binding_pin.package);
        if package.is_empty() {
            continue;
        }

        let Some(package_indexes) = package_pin_indexes.get(&package) else {
            continue;
        };

        for package_index in package_indexes {
            let package_pin = &scene.package_pins[*package_index];
            let field_conflicts = [
                (
                    "version",
                    non_empty_pin_field(&binding_pin.version),
                    non_empty_pin_field(&package_pin.version),
                ),
                (
                    "sha512",
                    non_empty_pin_field(&binding_pin.sha512),
                    non_empty_pin_field(&package_pin.sha512),
                ),
                (
                    "source",
                    non_empty_pin_field(&binding_pin.source),
                    non_empty_pin_field(&package_pin.source),
                ),
            ];

            if let Some((field_name, _, _)) = field_conflicts.into_iter().find(
                |(_, binding_value, package_value)| {
                    matches!((binding_value, package_value), (Some(left), Some(right)) if left != right)
                },
            ) {
                return Err(Box::new(ScenePinConflictDetail {
                    conflict_type: "binding_package_mismatch",
                    scene: resolved_scene_name.clone(),
                    capability: Some(binding_pin.capability.trim().to_string()),
                    package: Some(package.clone()),
                    indexes: vec![binding_index, *package_index],
                    message: format!(
                        "Scene binding pin for capability '{}' conflicts with package pin for '{}' on field '{}'",
                        binding_pin.capability.trim(),
                        package_pin.package.trim(),
                        field_name,
                    ),
                }));
            }
        }
    }

    Ok(())
}

fn resolved_scene_name(scene_name: &str, scene: &crate::app::AppSceneConfig) -> String {
    if scene.name.trim().is_empty() {
        scene_name.trim().to_string()
    } else {
        scene.name.trim().to_string()
    }
}

fn apply_scene_binding_pins_to_candidate_bindings(
    scene_name: &str,
    scene: Option<&crate::app::AppSceneConfig>,
    capabilities: &[String],
    proposed_bindings: Vec<crate::app::AppBindingResolution>,
) -> Vec<crate::app::AppBindingResolution> {
    let Some(scene) = scene else {
        return proposed_bindings;
    };

    let capability_set = capabilities
        .iter()
        .map(|capability| capability.trim())
        .filter(|capability| !capability.is_empty())
        .collect::<BTreeSet<_>>();
    let scene_name = resolved_scene_name(scene_name, scene);
    let mut bindings = proposed_bindings;

    for binding_pin in &scene.binding_pins {
        let capability = binding_pin.capability.trim();
        if capability.is_empty() {
            continue;
        }

        let pinned_provider = if binding_pin.provider.trim().is_empty() {
            binding_pin.package.trim()
        } else {
            binding_pin.provider.trim()
        };
        if pinned_provider.is_empty() {
            continue;
        }

        let binding_source = format!("scene:{scene_name}");
        if let Some(existing_binding) = bindings
            .iter_mut()
            .find(|binding| binding.capability == capability)
        {
            existing_binding.provider = pinned_provider.to_string();
            existing_binding.source = binding_source;
            continue;
        }

        if capability_set.contains(capability) {
            bindings.push(crate::app::AppBindingResolution {
                capability: capability.to_string(),
                provider: pinned_provider.to_string(),
                mutable: false,
                source: binding_source,
            });
        }
    }

    bindings
}

fn summary_lock_bindings(
    bindings: &[crate::app::AppBindingResolution],
    packages: &[crate::app::AppLockPackage],
) -> Vec<crate::app::AppLockBinding> {
    bindings
        .iter()
        .map(|binding| {
            let package = packages.iter().find(|pkg| {
                pkg.name == binding.provider || pkg.runtime_provider == binding.provider
            });
            crate::app::AppLockBinding {
                capability: binding.capability.clone(),
                provider: binding.provider.clone(),
                package: package
                    .map(|pkg| pkg.name.clone())
                    .unwrap_or_else(|| binding.provider.clone()),
                mutable: binding.mutable,
                package_version: package.map(|pkg| pkg.version.clone()).unwrap_or_default(),
                package_sha512: package.map(|pkg| pkg.sha512.clone()).unwrap_or_default(),
                binding_source: binding.source.clone(),
            }
        })
        .collect()
}

fn binding_package_names(
    bindings: &[crate::app::AppBindingResolution],
    packages: &[crate::app::AppLockPackage],
) -> std::collections::HashMap<String, String> {
    bindings
        .iter()
        .map(|binding| {
            let package_name = packages
                .iter()
                .find(|pkg| {
                    pkg.name == binding.provider || pkg.runtime_provider == binding.provider
                })
                .map(|pkg| pkg.name.clone())
                .unwrap_or_else(|| binding.provider.clone());
            (binding.capability.clone(), package_name)
        })
        .collect()
}

fn lock_package_from_source(
    state: &AppState,
    pkg: &crate::app::PackageSource,
    selected_names: &BTreeSet<String>,
) -> crate::app::AppLockPackage {
    let digest = package_digest(&state.repo_root, &pkg.current_source);
    crate::app::AppLockPackage {
        name: pkg.name.clone(),
        version: "current".into(),
        runtime: pkg.kind.clone(),
        sha512: digest.clone(),
        source: pkg.current_source.clone(),
        trusted: pkg.trusted,
        signature: pkg.signature.clone(),
        source_authority: pkg.source_authority.clone(),
        source_public_keys: pkg.source_public_keys.clone(),
        package_kind: pkg.package_kind.clone(),
        manifest_digest: digest.clone(),
        artifact_digest: digest.clone(),
        artifact_set_id: String::new(),
        store_object_id: format!("sha512-{}-{}-current", digest, pkg.name),
        store_path: String::new(),
        closure_id: String::new(),
        entry_kind: pkg.kind.clone(),
        runtime_provider: pkg.runtime_provider_name(),
        provides: pkg.provides.clone(),
        requires: pkg.requires.clone(),
        roles: pkg
            .package_kind
            .split(',')
            .map(|role| role.trim().to_string())
            .filter(|role| !role.is_empty())
            .collect(),
        capabilities: pkg.provides.clone(),
        features: if selected_names.contains(&pkg.name) && pkg.package_kind.contains("feature") {
            vec![pkg.name.clone()]
        } else {
            vec![]
        },
        default_enabled_features: vec![],
        evidence: crate::app::AppLockEvidence {
            digest,
            signature: pkg.signature.clone(),
            source_authority: pkg.source_authority.clone(),
            source_public_keys: pkg.source_public_keys.clone(),
        },
        reasons: vec![],
    }
}

fn selected_summary_packages(
    state: &AppState,
    app_config: &crate::app::AppConfigFile,
    bindings: &[crate::app::AppBindingResolution],
) -> Result<Vec<crate::app::AppLockPackage>, (StatusCode, Json<serde_json::Value>)> {
    let mut selected_names = app_config.packages.enabled.clone();
    for binding in bindings {
        if !selected_names.contains(&binding.provider) {
            selected_names.push(binding.provider.clone());
        }
    }

    let active_scene = active_scene_config(app_config);
    let resolved_scene = active_scene
        .map(|scene| resolved_scene_name(app_config.active_scene.as_str(), scene))
        .unwrap_or_else(|| app_config.active_scene.clone());

    if let Some(scene) = active_scene {
        for (index, package_pin) in scene.package_pins.iter().enumerate() {
            let package = package_pin.package.trim();
            if package.is_empty() {
                continue;
            }

            if state.package_index.get(package).is_none() {
                return Err(scene_package_pin_unresolved_response(
                    &resolved_scene,
                    package,
                    index,
                ));
            }

            if !selected_names.iter().any(|name| name == package) {
                selected_names.push(package.to_string());
            }
        }
    }

    let mut packages = packages_for_selected_names(state, &selected_names);
    if let Some(scene) = active_scene {
        for package_pin in &scene.package_pins {
            let package = package_pin.package.trim();
            if package.is_empty() {
                continue;
            }

            if let Some(summary_package) = packages.iter_mut().find(|pkg| pkg.name == package) {
                let mut source_overridden = false;
                let mut sha512_overridden = false;
                if let Some(version) = non_empty_pin_field(&package_pin.version) {
                    summary_package.version = version;
                }
                if let Some(sha512) = non_empty_pin_field(&package_pin.sha512) {
                    summary_package.sha512 = sha512.clone();
                    summary_package.artifact_digest = sha512.clone();
                    summary_package.manifest_digest = sha512.clone();
                    summary_package.evidence.digest = sha512;
                    sha512_overridden = true;
                }
                if let Some(source) = non_empty_pin_field(&package_pin.source) {
                    summary_package.source = source;
                    source_overridden = true;
                }

                if source_overridden && !sha512_overridden {
                    // Once a scene pin changes package identity to a different source, the helper
                    // can no longer treat the original local digest as valid summary metadata.
                    summary_package.sha512.clear();
                    summary_package.manifest_digest.clear();
                    summary_package.artifact_digest.clear();
                    summary_package.evidence.digest.clear();
                    summary_package.store_object_id.clear();
                }
            }
        }
    }

    Ok(packages)
}

async fn verification_packages_for_candidate(
    state: &AppState,
    app_name: &str,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::AppLockPackage> {
    let app = {
        let apps = state.resolved_apps.read().await;
        apps.get(app_name).cloned()
    };

    let Some(app) = app else {
        return packages_for_generation(state, generation);
    };

    let Some(config_path) = app.sources.config_path.as_deref() else {
        return packages_for_generation(state, generation);
    };

    let app_config = crate::app::load_instance_config_from_path(std::path::Path::new(config_path))
        .unwrap_or_default();

    selected_summary_packages(state, &app_config, &generation.bindings)
        .unwrap_or_else(|_| packages_for_generation(state, generation))
}

async fn identity_consistency_results(
    state: &AppState,
    app_name: &str,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::ValidationResult> {
    let packages = verification_packages_for_candidate(state, app_name, generation).await;
    let lock_bindings = summary_lock_bindings(&generation.bindings, &packages);
    let recomputed_binding_set_id = binding_set_id_from_lock_bindings(&lock_bindings);
    let recomputed_closure_id = closure_id_from_lock_packages(&packages);

    let mut results = Vec::with_capacity(2);

    let recorded_binding_set_id = generation.binding_set_id.trim();
    results.push(if recorded_binding_set_id.is_empty() {
        crate::app::ValidationResult {
            check: "identity-consistency:binding_set_id".into(),
            passed: true,
            message: format!(
                "Warning: candidate generation summary is missing binding_set_id metadata; recomputed identity '{}' was used for verification compatibility",
                recomputed_binding_set_id
            ),
        }
    } else {
        let matches = recorded_binding_set_id == recomputed_binding_set_id;
        crate::app::ValidationResult {
            check: "identity-consistency:binding_set_id".into(),
            passed: matches,
            message: if matches {
                format!(
                    "Candidate binding_set_id matches the recomputed binding identity '{}' from current bindings and selected package summary",
                    recomputed_binding_set_id
                )
            } else {
                format!(
                    "Candidate binding_set_id '{}' does not match recomputed binding identity '{}' from current bindings and selected package summary",
                    recorded_binding_set_id, recomputed_binding_set_id
                )
            },
        }
    });

    let recorded_closure_id = generation.closure_id.trim();
    results.push(if recorded_closure_id.is_empty() {
        crate::app::ValidationResult {
            check: "identity-consistency:closure_id".into(),
            passed: true,
            message: format!(
                "Warning: candidate generation summary is missing closure_id metadata; recomputed identity '{}' was used for verification compatibility",
                recomputed_closure_id
            ),
        }
    } else {
        let matches = recorded_closure_id == recomputed_closure_id;
        crate::app::ValidationResult {
            check: "identity-consistency:closure_id".into(),
            passed: matches,
            message: if matches {
                format!(
                    "Candidate closure_id matches the recomputed closure identity '{}' from the selected package summary",
                    recomputed_closure_id
                )
            } else {
                format!(
                    "Candidate closure_id '{}' does not match recomputed closure identity '{}' from the selected package summary",
                    recorded_closure_id, recomputed_closure_id
                )
            },
        }
    });

    results
}

async fn binding_coverage_results(
    state: &AppState,
    app_name: &str,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::ValidationResult> {
    let packages = verification_packages_for_candidate(state, app_name, generation).await;
    let included_packages = packages
        .iter()
        .flat_map(|pkg| [pkg.name.as_str(), pkg.runtime_provider.as_str()])
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<_>>();
    let bindings_by_capability = generation
        .bindings
        .iter()
        .map(|binding| (binding.capability.as_str(), binding))
        .collect::<BTreeMap<_, _>>();
    let mut results = Vec::new();

    for capability in &generation.capabilities {
        let check_name = format!("binding-coverage:{capability}");
        let Some(binding) = bindings_by_capability.get(capability.as_str()) else {
            results.push(crate::app::ValidationResult {
                check: check_name,
                passed: false,
                message: format!(
                    "Required capability '{}' has no provider binding in candidate generation",
                    capability
                ),
            });
            continue;
        };

        if binding.provider == "core" {
            results.push(crate::app::ValidationResult {
                check: check_name,
                passed: true,
                message: format!(
                    "Required capability '{}' is bound to core; selected package coverage is not required",
                    capability
                ),
            });
            continue;
        }

        let included = included_packages.contains(binding.provider.as_str());
        results.push(crate::app::ValidationResult {
            check: check_name,
            passed: included,
            message: if included {
                format!(
                    "Required capability '{}' is bound to provider '{}' with an included selected package summary entry",
                    capability, binding.provider
                )
            } else {
                format!(
                    "Required capability '{}' is bound to provider '{}' but no included selected package summary entry matches that binding",
                    capability, binding.provider
                )
            },
        });
    }

    results
}

fn is_valid_sha512_hex(value: &str) -> bool {
    value.len() == 128 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn has_usable_digest_metadata(pkg: &crate::app::AppLockPackage) -> bool {
    is_valid_sha512_hex(pkg.sha512.trim())
        || !pkg.artifact_digest.trim().is_empty()
        || !pkg.evidence.digest.trim().is_empty()
}

async fn digest_policy_results(
    state: &AppState,
    app_name: &str,
    generation: &crate::app::AppGeneration,
) -> (Vec<crate::app::ValidationResult>, bool) {
    let profile = crate::app::AppProfile::from_str_loose(&generation.profile);
    let packages = verification_packages_for_candidate(state, app_name, generation).await;
    let mut dev_unsealed = false;

    let results = packages
        .into_iter()
        .map(|pkg| {
            let digest_ok = has_usable_digest_metadata(&pkg);
            match profile {
                crate::app::AppProfile::Developer if !digest_ok => {
                    dev_unsealed = true;
                    crate::app::ValidationResult {
                        check: format!("digest-policy:{}", pkg.name),
                        passed: true,
                        message: format!(
                            "Warning: package '{}' is missing usable digest metadata for source '{}' under profile 'developer'; candidate remains verified as dev-unsealed",
                            pkg.name, pkg.source
                        ),
                    }
                }
                _ => crate::app::ValidationResult {
                    check: format!("digest-policy:{}", pkg.name),
                    passed: digest_ok,
                    message: if digest_ok {
                        format!(
                            "Package '{}' has usable digest metadata for source '{}' under profile '{}'",
                            pkg.name, pkg.source, generation.profile
                        )
                    } else {
                        format!(
                            "Package '{}' is missing usable sha512/artifact/content digest metadata for source '{}' under profile '{}'",
                            pkg.name, pkg.source, generation.profile
                        )
                    },
                },
            }
        })
        .collect();

    (results, dev_unsealed)
}

fn generation_is_dev_unsealed(generation: &crate::app::AppGeneration) -> bool {
    crate::app::AppProfile::from_str_loose(&generation.profile) == crate::app::AppProfile::Developer
        && generation_has_dev_unsealed_digest_policy_validation(generation)
}

async fn store_policy_results(
    state: &AppState,
    app_name: &str,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::ValidationResult> {
    let packages = verification_packages_for_candidate(state, app_name, generation).await;
    let profile = crate::app::AppProfile::from_str_loose(&generation.profile);
    let instance_dir = {
        let apps = state.resolved_apps.read().await;
        apps.get(app_name).and_then(resolved_app_instance_dir)
    };

    crate::app::verify_local_store_packages(instance_dir.as_deref(), &packages, profile)
}

fn activation_policy_error(
    app_name: &str,
    generation: &crate::app::AppGeneration,
    instance_dir: Option<&FsPath>,
) -> Option<(StatusCode, Json<serde_json::Value>)> {
    if generation_has_dev_unsealed_digest_policy_validation(generation)
        && crate::app::AppProfile::from_str_loose(&generation.profile)
            != crate::app::AppProfile::Developer
    {
        return Some((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "Generation {} uses developer-only dev-unsealed validation but profile '{}' is not allowed to activate it",
                    generation.id, generation.profile
                ),
                "reason": "dev_unsealed_profile_mismatch",
                "status": "activation_failed",
                "generation": generation,
                "lock_written": false,
                "pointer_written": false,
                "pointer_error": serde_json::Value::Null,
                "index_written": false,
                "index_error": serde_json::Value::Null,
            })),
        ));
    }

    let needs_store_verification = generation
        .validation_results
        .iter()
        .any(crate::app::is_store_check);
    if needs_store_verification {
        let profile = crate::app::AppProfile::from_str_loose(&generation.profile);
        let packages = if let Some(instance_dir) = instance_dir {
            let immutable_lock_path =
                resolved_generation_lock_path(instance_dir, &generation.lock_path);
            crate::app::load_instance_lock_from_path(&immutable_lock_path)
                .map(|lock| lock.packages)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let store_results =
            crate::app::verify_local_store_packages(instance_dir, &packages, profile);
        if let Some(result) = store_results.into_iter().find(|result| !result.passed) {
            return Some((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": format!(
                        "Generation {} failed offline store verification required for activation: {}",
                        generation.id, result.message
                    ),
                    "reason": "store_object_missing",
                    "status": "activation_failed",
                    "generation": generation,
                    "app": app_name,
                    "lock_written": false,
                    "pointer_written": false,
                    "pointer_error": serde_json::Value::Null,
                    "index_written": false,
                    "index_error": serde_json::Value::Null,
                    "store_validation": result,
                })),
            ));
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoredGenerationSlot {
    Active,
    Candidate,
    Rollback,
}

fn stored_generation_slot(
    app_store: &crate::app::AppGenerationStore,
    generation_id: u64,
) -> Option<StoredGenerationSlot> {
    if app_store
        .active
        .as_ref()
        .is_some_and(|generation| generation.id == generation_id)
    {
        Some(StoredGenerationSlot::Active)
    } else if app_store
        .candidate
        .as_ref()
        .is_some_and(|generation| generation.id == generation_id)
    {
        Some(StoredGenerationSlot::Candidate)
    } else if app_store
        .rollback
        .as_ref()
        .is_some_and(|generation| generation.id == generation_id)
    {
        Some(StoredGenerationSlot::Rollback)
    } else {
        None
    }
}

fn activation_target_error(
    app_name: &str,
    generation_id: u64,
    status: Option<crate::app::GenerationStatus>,
) -> (StatusCode, Json<serde_json::Value>) {
    match status {
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!(
                    "Generation {} not found for app '{}'",
                    generation_id, app_name
                ),
                "reason": "generation_not_found",
                "status": "activation_failed",
                "generation": generation_id,
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
            })),
        ),
        Some(crate::app::GenerationStatus::Candidate) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "Generation {} must be verified before activation",
                    generation_id
                ),
                "reason": "generation_not_verified",
                "status": "activation_failed",
                "generation": generation_id,
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
            })),
        ),
        Some(crate::app::GenerationStatus::Failed) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "Generation {} failed verification and cannot be activated",
                    generation_id
                ),
                "reason": "generation_failed",
                "status": "activation_failed",
                "generation": generation_id,
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
            })),
        ),
        Some(crate::app::GenerationStatus::Archived) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "Generation {} is archived and cannot be activated",
                    generation_id
                ),
                "reason": "generation_archived",
                "status": "activation_failed",
                "generation": generation_id,
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
            })),
        ),
        Some(_) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "Generation {} cannot be activated from its current state",
                    generation_id
                ),
                "reason": "activation_failed",
                "status": "activation_failed",
                "generation": generation_id,
                "lock_written": false,
                "pointer_written": false,
                "index_written": false,
            })),
        ),
    }
}

fn generation_has_dev_unsealed_digest_policy_validation(
    generation: &crate::app::AppGeneration,
) -> bool {
    generation.validation_results.iter().any(|result| {
        result.check == "digest-policy:dev-unsealed"
            && result.passed
            && result
                .message
                .contains("developer-only dev-unsealed digest policy exceptions")
    })
}

fn packages_for_selected_names(
    state: &AppState,
    names: &[String],
) -> Vec<crate::app::AppLockPackage> {
    let selected = names.iter().cloned().collect::<BTreeSet<_>>();
    state
        .package_index
        .package_sources
        .iter()
        .filter(|pkg| selected.contains(&pkg.name) && package_matches_required_real_source(pkg))
        .map(|pkg| lock_package_from_source(state, pkg, &selected))
        .collect()
}

struct ActivationTargetSelection {
    generation: crate::app::AppGeneration,
    previous_generation_id: Option<u64>,
}

fn select_activation_target(
    app_name: &str,
    app_store: &mut crate::app::AppGenerationStore,
    target_generation_id: Option<u64>,
) -> Result<ActivationTargetSelection, (StatusCode, Json<serde_json::Value>)> {
    match target_generation_id {
        None => {
            if let Some(error) = app_store
                .candidate
                .as_ref()
                .and_then(|generation| activation_policy_error(app_name, generation, None))
            {
                return Err(error);
            }

            let previous_generation_id = app_store.active.as_ref().map(|generation| generation.id);
            let generation = app_store.activate().cloned().map_err(|err| {
                (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({"error": err})),
                )
            })?;

            Ok(ActivationTargetSelection {
                generation,
                previous_generation_id,
            })
        }
        Some(target_generation_id) => {
            let Some(slot) = stored_generation_slot(app_store, target_generation_id) else {
                return Err(activation_target_error(
                    app_name,
                    target_generation_id,
                    None,
                ));
            };

            let target_snapshot = app_store
                .generation(target_generation_id)
                .cloned()
                .ok_or_else(|| activation_target_error(app_name, target_generation_id, None))?;

            match target_snapshot.status {
                crate::app::GenerationStatus::Verified | crate::app::GenerationStatus::Rollback => {
                }
                status => {
                    return Err(activation_target_error(
                        app_name,
                        target_generation_id,
                        Some(status),
                    ));
                }
            }

            if let Some(error) = activation_policy_error(app_name, &target_snapshot, None) {
                return Err(error);
            }

            let previous_generation_id = match slot {
                StoredGenerationSlot::Active => {
                    app_store.rollback.as_ref().map(|generation| generation.id)
                }
                StoredGenerationSlot::Candidate | StoredGenerationSlot::Rollback => {
                    app_store.active.as_ref().map(|generation| generation.id)
                }
            };

            let generation = app_store
                .switch_to_existing(target_generation_id)
                .cloned()
                .map_err(|_| {
                    activation_target_error(
                        app_name,
                        target_generation_id,
                        Some(target_snapshot.status),
                    )
                })?;

            Ok(ActivationTargetSelection {
                generation,
                previous_generation_id,
            })
        }
    }
}

pub fn preview_runtime_inputs(
    manifest: &crate::app::AppManifest,
    app_config: &crate::app::AppConfigFile,
    default_bindings: &[crate::app::AppBindingResolution],
) -> (
    Vec<String>,
    Vec<String>,
    std::collections::HashMap<String, crate::app::AppBindingOverride>,
) {
    let mut enabled_features = if app_config.features.enabled.is_empty() {
        manifest.features.default_enabled.clone()
    } else {
        app_config.features.enabled.clone()
    };
    let disabled_features = app_config
        .features
        .disabled
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    enabled_features.retain(|feature| !disabled_features.contains(feature));
    enabled_features.sort();
    enabled_features.dedup();

    let mut selected_packages = app_config.packages.enabled.clone();
    for feature in &enabled_features {
        if let Some(binding) = manifest.feature_bindings.get(feature) {
            for package in &binding.packages {
                if !selected_packages.contains(package) {
                    selected_packages.push(package.clone());
                }
            }
        }
    }
    for binding in default_bindings {
        if !selected_packages.contains(&binding.provider) {
            selected_packages.push(binding.provider.clone());
        }
    }
    selected_packages.retain(|package| !app_config.packages.disabled.contains(package));
    selected_packages.sort();
    selected_packages.dedup();

    let mut overrides = std::collections::HashMap::new();
    for (capability, binding) in &app_config.binding_overrides {
        if !binding.provider.is_empty() {
            overrides.insert(capability.clone(), binding.clone());
        }
    }

    (enabled_features, selected_packages, overrides)
}

pub fn preview_runtime_capabilities(
    manifest: &crate::app::AppManifest,
    enabled_features: &[String],
) -> Vec<String> {
    let mut capabilities = manifest.requires.capabilities.clone();
    let mut seen = capabilities.iter().cloned().collect::<BTreeSet<_>>();

    for feature in enabled_features {
        if let Some(binding) = manifest.feature_bindings.get(feature) {
            for capability in &binding.requires {
                if seen.insert(capability.clone()) {
                    capabilities.push(capability.clone());
                }
            }
        }
    }

    capabilities
}

pub fn preview_runtime_bindings(
    app: &crate::app::ResolvedApp,
    allowed_capabilities: &[String],
    package_index: &crate::app::PackageIndex,
    config_overrides: &std::collections::HashMap<String, crate::app::AppBindingOverride>,
) -> Vec<crate::app::AppBindingResolution> {
    let allowed = allowed_capabilities
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    app.bindings
        .iter()
        .filter_map(|binding| {
            if !allowed.contains(&binding.capability) {
                return None;
            }

            let provider = config_overrides
                .get(&binding.capability)
                .map(|override_binding| override_binding.provider.clone())
                .unwrap_or_else(|| binding.provider.clone());

            if provider != "core" && package_index.get(&provider).is_none() {
                return None;
            }

            Some(crate::app::AppBindingResolution {
                capability: binding.capability.clone(),
                provider,
                mutable: binding.mutable,
                source: if config_overrides.contains_key(&binding.capability) {
                    "config-override".into()
                } else {
                    binding.source.clone()
                },
            })
        })
        .collect()
}

pub fn preview_runtime_bindings_from_manifest(
    manifest: &crate::app::AppManifest,
    allowed_capabilities: &[String],
    package_index: &crate::app::PackageIndex,
    config_overrides: &std::collections::HashMap<String, crate::app::AppBindingOverride>,
) -> Vec<crate::app::AppBindingResolution> {
    let allowed = allowed_capabilities
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let flattened = manifest.flattened_bindings();

    allowed_capabilities
        .iter()
        .filter_map(|capability| {
            let binding = flattened.get(capability)?;
            let provider = config_overrides
                .get(capability)
                .map(|override_binding| override_binding.provider.clone())
                .unwrap_or_else(|| binding.provider.clone());

            if provider != "core" && package_index.get(&provider).is_none() {
                return None;
            }

            Some(crate::app::AppBindingResolution {
                capability: capability.clone(),
                provider,
                mutable: binding.mutable,
                source: if config_overrides.contains_key(capability) {
                    "config-override".into()
                } else {
                    "declaration-default".into()
                },
            })
        })
        .filter(|binding| allowed.contains(&binding.capability))
        .collect()
}

fn default_runtime_bindings(
    app: &crate::app::ResolvedApp,
) -> Vec<crate::app::AppBindingResolution> {
    app.bindings
        .iter()
        .filter(|binding| {
            binding.provider == "core" || app.capabilities.contains(&binding.capability)
        })
        .cloned()
        .collect()
}

fn lock_features_for_generation(
    manifest: &crate::app::AppManifest,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::AppLockFeature> {
    let enabled = generation.capabilities.iter().collect::<BTreeSet<_>>();

    manifest
        .features
        .default_enabled
        .iter()
        .chain(manifest.features.optional.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|feature_name| {
            let binding = manifest.feature_bindings.get(&feature_name);
            let capabilities = binding
                .map(|binding| binding.requires.clone())
                .unwrap_or_default();
            let resolved = capabilities
                .iter()
                .all(|capability| enabled.contains(capability));
            crate::app::AppLockFeature {
                name: feature_name,
                resolved,
                packages: binding
                    .map(|binding| binding.packages.clone())
                    .unwrap_or_default(),
                capabilities,
            }
        })
        .collect()
}

fn trust_results(
    state: &AppState,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::ValidationResult> {
    let profile = crate::app::AppProfile::from_str_loose(&generation.profile);
    packages_for_generation(state, generation)
        .into_iter()
        .map(|pkg| {
            let allowed = match profile {
                crate::app::AppProfile::Safe => pkg.trusted && pkg.runtime != "native",
                crate::app::AppProfile::Developer => pkg.runtime != "native" || pkg.trusted,
                crate::app::AppProfile::Trusted => true,
            };
            crate::app::ValidationResult {
                check: format!("trust:{}", pkg.name),
                passed: allowed,
                message: if allowed {
                    format!(
                        "Package '{}' is allowed under profile '{}' (runtime={}, trusted={})",
                        pkg.name, generation.profile, pkg.runtime, pkg.trusted
                    )
                } else {
                    format!(
                        "Package '{}' is not allowed under profile '{}' (runtime={}, trusted={})",
                        pkg.name, generation.profile, pkg.runtime, pkg.trusted
                    )
                },
            }
        })
        .collect()
}

fn signature_results(
    state: &AppState,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::ValidationResult> {
    let profile = crate::app::AppProfile::from_str_loose(&generation.profile);
    packages_for_generation(state, generation)
        .into_iter()
        .map(|pkg| {
            let crypto_ok = if pkg.signature.starts_with("ed25519:") {
                crate::app::verify_package_signature_for_source(
                    &pkg.signature,
                    &crate::app::signature_message(
                        &pkg.name,
                        &pkg.version,
                        &pkg.sha512,
                        &pkg.source,
                    ),
                    &pkg.source_authority,
                    &pkg.source_public_keys,
                )
                .is_ok()
            } else {
                false
            };
            let signature_ok = match profile {
                crate::app::AppProfile::Safe => pkg.signature.starts_with("builtin:") || crypto_ok,
                crate::app::AppProfile::Developer => {
                    (!pkg.signature.is_empty() && pkg.signature != "unsigned") || crypto_ok
                }
                crate::app::AppProfile::Trusted => true,
            };
            crate::app::ValidationResult {
                check: format!("signature:{}", pkg.name),
                passed: signature_ok,
                message: if signature_ok {
                    format!(
                        "Package '{}' signature '{}' accepted under profile '{}'{}",
                        pkg.name,
                        pkg.signature,
                        generation.profile,
                        if crypto_ok {
                            " (cryptographically verified)"
                        } else {
                            ""
                        }
                    )
                } else {
                    format!(
                        "Package '{}' signature '{}' rejected under profile '{}'",
                        pkg.name, pkg.signature, generation.profile
                    )
                },
            }
        })
        .collect()
}

async fn live_probe_generation(
    state: &AppState,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::ValidationResult> {
    let mut results = Vec::new();
    let registry = state.capability_registry.read().await;

    for capability in &generation.capabilities {
        let binding = generation
            .bindings
            .iter()
            .find(|binding| binding.capability == *capability);
        let Some(binding) = binding else {
            results.push(crate::app::ValidationResult {
                check: format!("probe:{}", capability),
                passed: false,
                message: "No provider binding found for capability during live probe".into(),
            });
            continue;
        };

        let registry_provider = registry.get(capability).and_then(|entry| {
            entry
                .providers
                .iter()
                .find(|provider| provider.provider == binding.provider)
        });
        let Some(registry_provider) = registry_provider else {
            results.push(crate::app::ValidationResult {
                check: format!("probe:{}", capability),
                passed: false,
                message: format!(
                    "Live probe failed because registry has no runtime entry for capability '{}' bound provider '{}'",
                    capability, binding.provider
                ),
            });
            continue;
        };

        if !generation_declares_real_package(state, generation, &binding.provider) {
            results.push(crate::app::ValidationResult {
                check: format!("probe:{}", capability),
                passed: false,
                message: format!(
                    "Live probe failed because capability '{}' is bound to '{}' without an assembled real package source",
                    capability, binding.provider
                ),
            });
            continue;
        }

        let runtime = registry_provider.runtime.as_str();

        if runtime == "native" {
            results.push(crate::app::ValidationResult {
                check: format!("probe:{}", capability),
                passed: true,
                message: format!(
                    "Live probe skipped for {} runtime until direct probe support is available",
                    runtime
                ),
            });
            continue;
        }

        if runtime == "wasm" {
            results.push(
                probe_wasm_provider_via_temp_load(
                    state,
                    &binding.provider,
                    capability,
                    &generation.app_name,
                )
                .await,
            );
            continue;
        }

        if runtime == "metadata" {
            results.push(crate::app::ValidationResult {
                check: format!("probe:{}", capability),
                passed: !requires_real_package_source(&binding.provider),
                message: if requires_real_package_source(&binding.provider) {
                    format!(
                        "Live probe failed because capability '{}' requires a real package-backed runtime, not metadata provider '{}'",
                        capability, binding.provider
                    )
                } else {
                    "Metadata provider validated structurally; live probe skipped".into()
                },
            });
            continue;
        }

        let probe_payload = serde_json::json!({
            "action": "describe",
            "data": {},
            "app": generation.app_name,
        });
        let result =
            crate::api::capabilities::execute_capability_call(state, capability, probe_payload)
                .await;
        match result {
            Ok(_) => results.push(crate::app::ValidationResult {
                check: format!("probe:{}", capability),
                passed: true,
                message: format!("Provider responded to live {} probe", runtime),
            }),
            Err((status, value)) => {
                let body = value.to_string();
                let skipped_unknown_action = status == StatusCode::BAD_GATEWAY
                    && body.to_lowercase().contains("unknown action")
                    && body.contains("describe");
                results.push(crate::app::ValidationResult {
                    check: format!("probe:{}", capability),
                    passed: skipped_unknown_action,
                    message: if skipped_unknown_action {
                        "Live probe skipped because provider does not implement generic describe action"
                            .into()
                    } else {
                        format!("Live probe failed with {}: {}", status, value)
                    },
                });
            }
        }
    }

    results
}

fn integrity_results(
    state: &AppState,
    generation: &crate::app::AppGeneration,
) -> Vec<crate::app::ValidationResult> {
    packages_for_generation(state, generation)
        .into_iter()
        .map(|pkg| {
            let digest_ok = is_valid_sha512_hex(&pkg.sha512);
            crate::app::ValidationResult {
                check: format!("integrity:{}", pkg.name),
                passed: digest_ok,
                message: if digest_ok {
                    format!(
                        "Package '{}' has valid SHA-512 digest from {}",
                        pkg.name, pkg.source
                    )
                } else {
                    format!("Package '{}' has invalid digest", pkg.name)
                },
            }
        })
        .collect()
}

pub async fn propose_generation(
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

    let profile = state.active_profile.read().await;
    let manifest = crate::app::load_product_package_declaration_from_path(std::path::Path::new(
        &app.sources.manifest_path,
    ))
    .unwrap_or_default();
    let app_config = app
        .sources
        .config_path
        .as_deref()
        .map(std::path::Path::new)
        .map(|path| crate::app::load_instance_config_from_path(path).unwrap_or_default())
        .unwrap_or_default();
    let manifest_is_available =
        !app.sources.manifest_path.is_empty() && !manifest.app.name.is_empty();
    let manifest_bindings = manifest.flattened_bindings();
    let (config_enabled_features, _selected_packages, config_overrides) = if manifest_is_available {
        preview_runtime_inputs(&manifest, &app_config, &app.bindings)
    } else {
        (
            app.enabled_features.clone(),
            vec![],
            std::collections::HashMap::new(),
        )
    };
    let default_runtime_capabilities = if manifest_is_available {
        preview_runtime_capabilities(&manifest, &config_enabled_features)
    } else {
        app.capabilities.clone()
    };
    let default_runtime_bindings = if manifest_is_available {
        preview_runtime_bindings_from_manifest(
            &manifest,
            &default_runtime_capabilities,
            &state.package_index,
            &config_overrides,
        )
    } else {
        default_runtime_bindings(app)
    };

    let proposed_bindings: Vec<crate::app::AppBindingResolution> = body["bindings"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|b| {
                    Some(crate::app::AppBindingResolution {
                        capability: b["capability"].as_str()?.into(),
                        provider: b["provider"].as_str()?.into(),
                        mutable: b["mutable"].as_bool().unwrap_or(false),
                        source: b["source"].as_str().unwrap_or("request").into(),
                    })
                })
                .collect()
        })
        .unwrap_or_else(|| default_runtime_bindings.clone());

    for binding in &proposed_bindings {
        if manifest_is_available {
            if let Some(decl) = manifest_bindings.get(&binding.capability) {
                if !decl.mutable && binding.provider != decl.provider {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(serde_json::json!({
                            "error": format!(
                                "Binding '{}' is immutable — cannot change provider from '{}' to '{}'",
                                binding.capability, decl.provider, binding.provider
                            ),
                        })),
                    ));
                }
                if decl.mutable
                    && !decl.allowed.is_empty()
                    && !decl.allowed.contains(&binding.provider)
                {
                    return Err((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(serde_json::json!({
                            "error": format!(
                                "Provider '{}' is not in the allowed list for capability '{}'. Allowed: {:?}",
                                binding.provider, binding.capability, decl.allowed
                            ),
                        })),
                    ));
                }
            }
        }
    }

    let capabilities = body["capabilities"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| default_runtime_capabilities.clone());

    let version = body["version"].as_str().unwrap_or(&app.version).to_string();
    let parent_generation = {
        let store = state.generation_store.read().await;
        store
            .get(&name)
            .and_then(|entry| entry.active.as_ref())
            .map(|generation| generation.id)
    };
    let created_by = body["created_by"].as_str().unwrap_or("api").to_string();
    let active_scene = active_scene_config(&app_config);
    if let Some(scene_config) = active_scene {
        validate_scene_pin_conflicts(app_config.active_scene.as_str(), scene_config)
            .map_err(|detail| scene_pin_conflict_response(*detail))?;
    }
    let scene = active_scene
        .map(|scene| resolved_scene_name(app_config.active_scene.as_str(), scene))
        .unwrap_or_else(|| app_config.active_scene.clone());
    let effective_profile = active_scene
        .and_then(|scene| {
            if scene.profile.trim().is_empty() {
                None
            } else {
                Some(scene.profile.as_str())
            }
        })
        .unwrap_or(profile.as_str());
    let parent_generation = active_scene
        .and_then(|scene| scene.base_generation)
        .or(parent_generation);
    let enabled_features = overlay_scene_enabled_features(&config_enabled_features, active_scene);

    let proposed_bindings = apply_scene_binding_pins_to_candidate_bindings(
        app_config.active_scene.as_str(),
        active_scene,
        &capabilities,
        proposed_bindings,
    );

    let selected_packages = selected_summary_packages(&state, &app_config, &proposed_bindings)?;
    let lock_bindings = summary_lock_bindings(&proposed_bindings, &selected_packages);
    let binding_set_id = binding_set_id_from_lock_bindings(&lock_bindings);
    let closure_id = closure_id_from_lock_packages(&selected_packages);

    let mut store = state.generation_store.write().await;
    let app_store = store.entry(name.clone()).or_default();
    let candidate_id = app_store.next_generation_id();
    let gen = app_store.propose(crate::app::generation::AppGenerationProposal {
        app_name: name.clone(),
        version: version.clone(),
        bindings: proposed_bindings,
        capabilities,
        enabled_features,
        profile: effective_profile.to_string(),
        metadata: AppGenerationSummaryMetadata {
            scene,
            binding_set_id,
            closure_id,
            lock_digest: String::new(),
            lock_path: generation_lock_path(candidate_id),
            parent_generation,
            created_by,
        },
    });

    Ok(Json(serde_json::json!({
        "generation": gen,
        "status": "proposed",
    })))
}

pub async fn verify_generation(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut store = state.generation_store.write().await;
    let app_store = store.get_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("No generation store for app '{}'", name)})),
        )
    })?;

    let registry = state.capability_registry.read().await;
    let store_verification_error = app_store.verify_candidate(Some(&registry)).err();
    let generation_snapshot = match app_store.candidate.clone() {
        Some(candidate) => candidate,
        None => {
            let err = store_verification_error
                .unwrap_or_else(|| "No candidate generation to verify".to_string());
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": err,
                    "candidate": serde_json::Value::Null,
                })),
            ));
        }
    };
    let store_verification_ok = store_verification_error.is_none();
    drop(registry);
    drop(store);

    let binding_coverage = binding_coverage_results(&state, &name, &generation_snapshot).await;
    let (digest_policy, dev_unsealed) =
        digest_policy_results(&state, &name, &generation_snapshot).await;
    let store_policy = store_policy_results(&state, &name, &generation_snapshot).await;
    let identity_consistency =
        identity_consistency_results(&state, &name, &generation_snapshot).await;
    let integrity = integrity_results(&state, &generation_snapshot);
    let trust = trust_results(&state, &generation_snapshot);
    let signatures = signature_results(&state, &generation_snapshot);
    let probe_results = live_probe_generation(&state, &generation_snapshot).await;
    let binding_coverage_ok = binding_coverage.iter().all(|r| r.passed);
    let all_probes_passed = probe_results.iter().all(|r| r.passed);
    let digest_policy_ok = digest_policy.iter().all(|r| r.passed);
    let store_policy_ok = store_policy.iter().all(|r| r.passed);
    let identity_consistency_ok = identity_consistency.iter().all(|r| r.passed);
    let integrity_ok = integrity.iter().all(|r| r.passed);
    let trust_ok = trust.iter().all(|r| r.passed);
    let signatures_ok = signatures.iter().all(|r| r.passed);

    let mut store = state.generation_store.write().await;
    let app_store = store.get_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("No generation store for app '{}'", name)})),
        )
    })?;
    let candidate = app_store.candidate.as_mut().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Candidate generation disappeared during verification"})),
        )
    })?;
    candidate.validation_results.extend(binding_coverage);
    candidate.validation_results.extend(digest_policy);
    candidate.validation_results.extend(store_policy);
    candidate.validation_results.extend(identity_consistency);
    candidate.validation_results.extend(integrity);
    candidate.validation_results.extend(trust);
    candidate.validation_results.extend(signatures);
    candidate.validation_results.extend(probe_results);
    if dev_unsealed {
        candidate.validation_results.push(crate::app::ValidationResult {
            check: "digest-policy:dev-unsealed".into(),
            passed: true,
            message: "Warning: candidate verification succeeded with developer-only dev-unsealed digest policy exceptions".into(),
        });
    }
    if !store_verification_ok
        || !binding_coverage_ok
        || !all_probes_passed
        || !digest_policy_ok
        || !store_policy_ok
        || !identity_consistency_ok
        || !integrity_ok
        || !trust_ok
        || !signatures_ok
    {
        candidate.status = crate::app::GenerationStatus::Failed;
        let candidate_snapshot = candidate.clone();
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": "Verification failed",
                "reason": "generation_verify_failed",
                "candidate": candidate_snapshot,
            })),
        ));
    }

    let candidate_snapshot = candidate.clone();
    Ok(Json(serde_json::json!({
        "generation": candidate_snapshot,
        "status": "verified",
        "dev_unsealed": generation_is_dev_unsealed(&candidate_snapshot),
    })))
}

pub async fn activate_generation(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut store = state.generation_store.write().await;
    let app_store = store.get_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("No generation store for app '{}'", name)})),
        )
    })?;

    match select_activation_target(&name, app_store, None) {
        Ok(ActivationTargetSelection {
            generation,
            previous_generation_id,
        }) => {
            let active_generation_id = generation.id;
            let mut packages = packages_for_generation(&state, &generation);
            let apps = state.resolved_apps.read().await;
            let app = apps.get(&name).ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("App '{}' not found during activation", name)})),
                )
            })?;
            let instance_dir = resolved_app_instance_dir(app);
            if let Some(error) =
                activation_policy_error(&name, &generation, instance_dir.as_deref())
            {
                return Err(error);
            }
            let manifest_path = std::path::Path::new(&app.sources.manifest_path);
            let manifest = crate::app::load_product_package_declaration_from_path(manifest_path)
                .unwrap_or_default();
            let app_config = app
                .sources
                .config_path
                .as_deref()
                .map(std::path::Path::new)
                .map(|path| crate::app::load_instance_config_from_path(path).unwrap_or_default())
                .unwrap_or_default();
            let manifest_is_available =
                !app.sources.manifest_path.is_empty() && !manifest.app.name.is_empty();
            let declaration_path = manifest_path.to_path_buf();
            let manifest_rel = declaration_path
                .strip_prefix(&state.repo_root)
                .ok()
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| declaration_path.display().to_string().replace('\\', "/"));
            let config_path = app
                .sources
                .config_path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("config.toml"));
            let config_rel = config_path
                .strip_prefix(&state.repo_root)
                .ok()
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| config_path.display().to_string().replace('\\', "/"));
            let scene_digest = generation_scene_digest(&app_config, &generation.scene);
            let features = if manifest_is_available {
                lock_features_for_generation(&manifest, &generation)
            } else {
                generation
                    .enabled_features
                    .iter()
                    .cloned()
                    .map(|name| crate::app::AppLockFeature {
                        name,
                        resolved: true,
                        packages: vec![],
                        capabilities: vec![],
                    })
                    .collect()
            };
            let (_, selected_packages, _) = if manifest_is_available {
                preview_runtime_inputs(&manifest, &app_config, &generation.bindings)
            } else {
                (
                    generation.enabled_features.clone(),
                    vec![],
                    std::collections::HashMap::new(),
                )
            };
            let selected_packages = selected_packages
                .into_iter()
                .filter(|package_name| {
                    state
                        .package_index
                        .get(package_name)
                        .map(package_matches_required_real_source)
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>();
            let mut selected_only_packages =
                packages_for_selected_names(&state, &selected_packages);
            for package in selected_only_packages.drain(..) {
                if !packages
                    .iter()
                    .any(|existing| existing.name == package.name)
                {
                    packages.push(package);
                }
            }
            let feature_only_packages = packages_for_selected_names(
                &state,
                &features
                    .iter()
                    .flat_map(|feature| feature.packages.clone())
                    .collect::<Vec<_>>(),
            );
            for package in feature_only_packages {
                if !packages
                    .iter()
                    .any(|existing| existing.name == package.name)
                {
                    packages.push(package);
                }
            }
            let binding_package_names = binding_package_names(&generation.bindings, &packages);
            let lock_bindings = summary_lock_bindings(&generation.bindings, &packages);
            let closure_digest = closure_digest_from_lock_packages(&packages);
            let dev_unsealed = generation_is_dev_unsealed(&generation);

            let lock_file = crate::app::AppLockFile {
                lock_version: 2,
                app: generation.app_name.clone(),
                generation: generation.id as u32,
                status: "active".into(),
                profile: generation.profile.clone(),
                scene: generation.scene.clone(),
                scene_digest: scene_digest.clone(),
                binding_set_id: if generation.binding_set_id.is_empty() {
                    binding_set_id_from_lock_bindings(&lock_bindings)
                } else {
                    generation.binding_set_id.clone()
                },
                closure_id: if generation.closure_id.is_empty() {
                    closure_id_from_lock_packages(&packages)
                } else {
                    generation.closure_id.clone()
                },
                closure_digest: closure_digest.clone(),
                store_generation_id: String::new(),
                trust_level: if dev_unsealed {
                    "dev-unsealed".into()
                } else {
                    String::new()
                },
                dev_unsealed,
                features,
                inputs: crate::app::AppLockInputSnapshot {
                    declaration_schema_version: manifest.schema_version,
                    config_schema_version: app_config.schema_version,
                    declaration_digest: if manifest_is_available {
                        package_digest(&state.repo_root, &manifest_rel)
                    } else {
                        String::new()
                    },
                    config_digest: if manifest_is_available {
                        package_digest(&state.repo_root, &config_rel)
                    } else {
                        String::new()
                    },
                    scene_digest: scene_digest.clone(),
                    package_index_digest: String::new(),
                    package_server_snapshot_digest: String::new(),
                },
                assembly: crate::app::AppLockAssembly {
                    enabled_features: generation.enabled_features.clone(),
                    selected_packages,
                    scene: generation.scene.clone(),
                    scene_digest,
                    binding_set_id: if generation.binding_set_id.is_empty() {
                        binding_set_id_from_lock_bindings(&lock_bindings)
                    } else {
                        generation.binding_set_id.clone()
                    },
                    closure_id: if generation.closure_id.is_empty() {
                        closure_id_from_lock_packages(&packages)
                    } else {
                        generation.closure_id.clone()
                    },
                    closure_digest,
                },
                packages,
                bindings: lock_bindings,
                binding_sources: generation
                    .bindings
                    .iter()
                    .map(|binding| crate::app::AppLockBindingSource {
                        capability: binding.capability.clone(),
                        source: binding.source.clone(),
                        package: binding_package_names
                            .get(&binding.capability)
                            .cloned()
                            .unwrap_or_else(|| binding.provider.clone()),
                    })
                    .collect(),
                notes: crate::app::AppLockNotes {
                    message: format!("Activated generation {} via API", generation.id),
                },
            };

            let generation_lock_written = if let Some(ref instance_dir) = instance_dir {
                write_immutable_generation_lock(instance_dir, &generation, &lock_file).map_err(
                    |error| {
                        tracing::warn!(
                        "Failed to write immutable generation lock for '{}' generation {}: {:?}",
                        name,
                        generation.id,
                        error
                    );
                        immutable_generation_lock_failure_response(&name, generation.id, error)
                    },
                )?;
                true
            } else {
                false
            };

            let mut lock_written = false;
            if let Some(ref lock_path) = app.sources.lock_path {
                if let Err(e) = crate::app::save_instance_lock_to_path(
                    std::path::Path::new(lock_path),
                    &lock_file,
                ) {
                    tracing::warn!("Failed to write lock file for '{}': {}", name, e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("Failed to write lock file for '{}': {}", name, e),
                            "reason": "lock_write_failed",
                            "generation": generation,
                            "status": "activation_failed",
                            "lock_written": false,
                            "generation_lock_written": generation_lock_written,
                        })),
                    ));
                } else {
                    lock_written = true;
                }
            }

            let (pointer_written, pointer_error, index_written, index_error) = if let Some(
                instance_dir,
            ) = instance_dir
            {
                write_generation_pointers(
                    &name,
                    &instance_dir,
                    previous_generation_id,
                    active_generation_id,
                    generation_lock_written,
                    "activation_failed",
                    "activation",
                )?;

                let (index_written, index_error) =
                    persist_generation_index_best_effort(&name, app_store, &instance_dir);
                (true, serde_json::Value::Null, index_written, index_error)
            } else {
                (
                        false,
                        serde_json::json!({
                            "stage": "instance_dir",
                            "repair_needed": false,
                            "message": format!(
                                "No resolved instance directory available for '{}' to write activation pointers",
                                name
                            ),
                            "details": "resolved_app_instance_dir returned None",
                        }),
                        false,
                        Some(format!(
                            "No resolved instance directory available for '{}' to persist repairable generation index",
                            name
                        )),
                    )
            };
            Ok(Json(serde_json::json!({
                "generation": generation,
                "status": "activated",
                "generation_lock_written": generation_lock_written,
                "lock_written": lock_written,
                "pointer_written": pointer_written,
                "pointer_error": pointer_error,
                "index_written": index_written,
                "index_error": index_error,
            })))
        }
        Err(error) => Err(error),
    }
}

pub async fn activate_existing_generation(
    Path((name, generation_id)): Path<(String, u64)>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut store = state.generation_store.write().await;
    let app_store = store.get_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("No generation store for app '{}'", name)})),
        )
    })?;

    match select_activation_target(&name, app_store, Some(generation_id)) {
        Ok(ActivationTargetSelection {
            generation,
            previous_generation_id,
        }) => {
            let active_generation_id = generation.id;
            let apps = state.resolved_apps.read().await;
            let app = apps.get(&name).ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("App '{}' not found during activation", name)})),
                )
            })?;
            let instance_dir = resolved_app_instance_dir(app);
            if let Some(error) =
                activation_policy_error(&name, &generation, instance_dir.as_deref())
            {
                return Err(error);
            }
            let mut generation_lock_replayed = false;
            let mut generation_lock_warning = serde_json::Value::Null;
            let mut lock_written = false;

            if let Some(ref lock_path) = app.sources.lock_path {
                let existing_lock =
                    crate::app::load_instance_lock_from_path(std::path::Path::new(lock_path))
                        .unwrap_or_default();
                let rollback_root_lock = resolve_activation_root_lock(
                    &name,
                    &generation,
                    instance_dir.as_deref(),
                    existing_lock,
                )?;
                generation_lock_replayed = rollback_root_lock.generation_lock_replayed;
                if let Some(warning) = rollback_root_lock.generation_lock_warning {
                    generation_lock_warning = warning;
                }
                if let Err(e) = crate::app::save_instance_lock_to_path(
                    std::path::Path::new(lock_path),
                    &rollback_root_lock.lock_file,
                ) {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("Failed to write lock file for '{}' during activation: {}", name, e),
                            "reason": "lock_write_failed",
                            "status": "activation_failed",
                            "lock_written": false,
                            "pointer_written": false,
                            "pointer_error": serde_json::Value::Null,
                            "index_written": false,
                            "index_error": serde_json::Value::Null,
                            "generation_lock_replayed": generation_lock_replayed,
                            "generation_lock_warning": generation_lock_warning,
                        })),
                    ));
                } else {
                    lock_written = true;
                }
            }

            let (pointer_written, pointer_error, index_written, index_error) = if let Some(
                instance_dir,
            ) = instance_dir
            {
                write_generation_pointers(
                    &name,
                    &instance_dir,
                    previous_generation_id,
                    active_generation_id,
                    lock_written,
                    "activation_failed",
                    "activation",
                )?;

                let (index_written, index_error) =
                    persist_generation_index_best_effort(&name, app_store, &instance_dir);
                (true, serde_json::Value::Null, index_written, index_error)
            } else {
                (
                        false,
                        serde_json::json!({
                            "stage": "instance_dir",
                            "repair_needed": false,
                            "message": format!(
                                "No resolved instance directory available for '{}' to write activation pointers",
                                name
                            ),
                            "details": "resolved_app_instance_dir returned None",
                        }),
                        false,
                        Some(format!(
                            "No resolved instance directory available for '{}' to persist repairable generation index",
                            name
                        )),
                    )
            };

            Ok(Json(serde_json::json!({
                "generation": generation,
                "status": "activated",
                "lock_written": lock_written,
                "generation_lock_replayed": generation_lock_replayed,
                "generation_lock_warning": generation_lock_warning,
                "pointer_written": pointer_written,
                "pointer_error": pointer_error,
                "index_written": index_written,
                "index_error": index_error,
            })))
        }
        Err(error) => Err(error),
    }
}

pub async fn rollback_generation(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut store = state.generation_store.write().await;
    let app_store = store.get_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("No generation store for app '{}'", name)})),
        )
    })?;

    match app_store.rollback() {
        Ok(gen) => {
            let generation = gen.clone();
            let apps = state.resolved_apps.read().await;
            let app = apps.get(&name).ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("App '{}' not found during rollback", name)})),
                )
            })?;
            let instance_dir = resolved_app_instance_dir(app);
            let mut lock_written = false;
            let mut generation_lock_replayed = false;
            let mut generation_lock_warning = serde_json::Value::Null;
            if let Some(ref lock_path) = app.sources.lock_path {
                let existing_lock =
                    crate::app::load_instance_lock_from_path(std::path::Path::new(lock_path))
                        .unwrap_or_default();
                let rollback_root_lock = resolve_rollback_root_lock(
                    &name,
                    &generation,
                    instance_dir.as_deref(),
                    existing_lock,
                )?;
                generation_lock_replayed = rollback_root_lock.generation_lock_replayed;
                if let Some(warning) = rollback_root_lock.generation_lock_warning {
                    generation_lock_warning = warning;
                }
                if let Err(e) = crate::app::save_instance_lock_to_path(
                    std::path::Path::new(lock_path),
                    &rollback_root_lock.lock_file,
                ) {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("Failed to write lock file for '{}' during rollback: {}", name, e),
                            "reason": "lock_write_failed",
                            "status": "rollback_failed",
                            "lock_written": false,
                            "pointer_written": false,
                            "pointer_error": serde_json::Value::Null,
                            "index_written": false,
                            "index_error": serde_json::Value::Null,
                            "generation_lock_replayed": generation_lock_replayed,
                            "generation_lock_warning": generation_lock_warning,
                        })),
                    ));
                } else {
                    lock_written = true;
                }
            }

            let active_generation_id = generation.id;
            let previous_generation_id = app_store.candidate.as_ref().map(|candidate| candidate.id);
            let (pointer_written, pointer_error, index_written, index_error) = if let Some(
                instance_dir,
            ) = instance_dir
            {
                write_generation_pointers(
                    &name,
                    &instance_dir,
                    previous_generation_id,
                    active_generation_id,
                    lock_written,
                    "rollback_failed",
                    "rollback",
                )?;

                let (index_written, index_error) =
                    persist_generation_index_best_effort(&name, app_store, &instance_dir);
                (true, serde_json::Value::Null, index_written, index_error)
            } else {
                (
                        false,
                        serde_json::json!({
                            "stage": "instance_dir",
                            "repair_needed": false,
                            "message": format!(
                                "No resolved instance directory available for '{}' to write rollback pointers",
                                name
                            ),
                            "details": "resolved_app_instance_dir returned None",
                        }),
                        false,
                        Some(format!(
                            "No resolved instance directory available for '{}' to persist repairable generation index",
                            name
                        )),
                    )
            };
            Ok(Json(serde_json::json!({
                "generation": generation,
                "status": "rolled_back",
                "lock_written": lock_written,
                "generation_lock_replayed": generation_lock_replayed,
                "generation_lock_warning": generation_lock_warning,
                "pointer_written": pointer_written,
                "pointer_error": pointer_error,
                "index_written": index_written,
                "index_error": index_error,
            })))
        }
        Err(err) => Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": err})),
        )),
    }
}

pub async fn get_generation(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let store = state.generation_store.read().await;
    let app_store = store.get(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("No generation store for app '{}'", name)})),
        )
    })?;

    Ok(Json(serde_json::json!({
        "active": app_store.active,
        "candidate": app_store.candidate,
        "rollback": app_store.rollback,
    })))
}

pub async fn get_generation_diff(
    Path((name, from_id, to_id)): Path<(String, u64, u64)>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let resolved_app = {
        let apps = state.resolved_apps.read().await;
        apps.get(&name).cloned()
    };

    let (from_generation, to_generation) = {
        let store = state.generation_store.read().await;
        let app_store = store.get(&name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::json!({"error": format!("No generation store for app '{}'", name)}),
                ),
            )
        })?;

        let from_generation = app_store
            .generation(from_id)
            .cloned()
            .ok_or_else(|| generation_diff_not_found(&name, from_id, "from"))?;
        let to_generation = app_store
            .generation(to_id)
            .cloned()
            .ok_or_else(|| generation_diff_not_found(&name, to_id, "to"))?;
        (from_generation, to_generation)
    };

    let instance_dir = resolved_app.as_ref().and_then(resolved_app_instance_dir);
    let (from_lock, from_path, from_warning) = load_generation_lock_for_diff(&from_generation, instance_dir.as_deref())
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!(
                        "Failed to load immutable generation lock for app '{}' generation {}: {error:#}",
                        name, from_generation.id
                    ),
                    "reason": "generation_lock_load_failed",
                    "app": name,
                    "generation": from_generation.id,
                    "role": "from",
                })),
            )
        })?;
    let (to_lock, to_path, to_warning) = load_generation_lock_for_diff(&to_generation, instance_dir.as_deref())
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!(
                        "Failed to load immutable generation lock for app '{}' generation {}: {error:#}",
                        name, to_generation.id
                    ),
                    "reason": "generation_lock_load_failed",
                    "app": name,
                    "generation": to_generation.id,
                    "role": "to",
                })),
            )
        })?;

    let mut warnings = Vec::new();
    if let Some(warning) = from_warning {
        warnings.push(warning);
    }
    if let Some(warning) = to_warning {
        warnings.push(warning);
    }

    let immutable_lock = match (from_lock.as_ref(), to_lock.as_ref()) {
        (Some(from_lock), Some(to_lock)) => ImmutableLockComparison {
            available: true,
            from_path,
            to_path,
            diff: Some(immutable_lock_diff(from_lock, to_lock)),
        },
        _ => ImmutableLockComparison {
            available: false,
            from_path,
            to_path,
            diff: None,
        },
    };

    let response = GenerationDiffResponse {
        app: name,
        from: generation_summary_snapshot(&from_generation),
        to: generation_summary_snapshot(&to_generation),
        summary_diff: generation_summary_diff(&from_generation, &to_generation),
        immutable_lock,
        warnings,
    };

    Ok(Json(serde_json::json!(response)))
}

pub async fn get_generation_diagnostics(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let diagnostics = generation_diagnostics_snapshot(&state, &name).await?;
    Ok(Json(serde_json::json!(diagnostics)))
}

#[cfg(test)]
mod tests {
    use super::{
        activate_existing_generation, activate_generation, generation_diagnostics_snapshot,
        generation_lock_path, get_generation_diff, live_probe_generation,
        packages_for_selected_names, propose_generation, rollback_generation,
        selected_summary_packages, summary_lock_bindings,
    };
    use crate::api::build_router;
    use crate::api::openai_compat::AppState;
    use crate::app::{
        active_generation_pointer_path, binding_set_id_from_lock_bindings,
        closure_id_from_lock_packages, previous_generation_pointer_path, save_generation_index,
        sign_package_message, signature_message, write_active_generation_pointer,
        write_previous_generation_pointer, AppBindingResolution, AppGeneration, AppGenerationIndex,
        AppGenerationStore, AppProfile, CapabilityProviderRecord, CapabilityRegistry,
        CapabilityRegistryEntry, CorePolicy, GenerationStatus, GenerationStoreMap, PackageIndex,
        PackageSource, ResolvedApp, ResolvedAppMap, ResolvedAppSources,
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
    use axum::extract::{Path, State};
    use axum::http::{Request, StatusCode as HttpStatusCode};
    use axum::Json;
    use ed25519_dalek::SigningKey;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::{Arc, Mutex as StdMutex};
    use tempfile::tempdir;
    use tokio::sync::RwLock;
    use tower::util::ServiceExt;

    fn generation_with_diff_fields(
        id: u64,
        status: GenerationStatus,
        scene: &str,
        profile: &str,
        features: Vec<&str>,
        capabilities: Vec<&str>,
        bindings: Vec<AppBindingResolution>,
    ) -> AppGeneration {
        AppGeneration {
            id,
            app_name: "weft-claw".into(),
            version: "0.1.0".into(),
            bindings,
            capabilities: capabilities.into_iter().map(String::from).collect(),
            enabled_features: features.into_iter().map(String::from).collect(),
            scene: scene.into(),
            profile: profile.into(),
            binding_set_id: format!("binding-set:sha256:{id}"),
            closure_id: format!("closure:sha256:{id}"),
            lock_digest: format!("sha256:lock-{id}"),
            lock_path: format!("generations/{id}.lock.toml"),
            parent_generation: id.checked_sub(1),
            created_by: "api".into(),
            status,
            validation_results: vec![],
            created_at: 1710000000 + id,
        }
    }

    fn write_generation_lock_for_diff(
        instance_dir: &std::path::Path,
        generation_id: u64,
        scene: &str,
        profile: &str,
        closure_id: &str,
        packages: &[&str],
        bindings: &[(&str, &str)],
    ) {
        let package_entries = packages
            .iter()
            .map(|name| {
                format!(
                    "[[packages]]\nname = '{name}'\nversion = '0.1.0'\nruntime_provider = '{name}'\n"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let binding_entries = bindings
            .iter()
            .map(|(capability, provider)| {
                format!(
                    "[[bindings]]\ncapability = '{capability}'\nprovider = '{provider}'\npackage = '{provider}'\n"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        write_text_file(
            &instance_dir
                .join("generations")
                .join(format!("{generation_id}.lock.toml")),
            &format!(
                "lock_version = 2\napp = 'weft-claw'\ngeneration = {generation_id}\nstatus = 'verified'\nprofile = '{profile}'\nscene = '{scene}'\nclosure_id = '{closure_id}'\n\n{package_entries}\n{binding_entries}\n"
            ),
        );
    }

    fn test_state(
        repo_root: std::path::PathBuf,
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
            resolved_apps: Arc::new(RwLock::new(Default::default())),
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
        profile: &str,
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
            profile: profile.into(),
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

    fn registry_with_provider(capability: &str, provider: &str) -> CapabilityRegistry {
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            capability.into(),
            CapabilityRegistryEntry {
                capability: capability.into(),
                providers: vec![CapabilityProviderRecord {
                    provider: provider.into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        registry
    }

    fn candidate_with_verified_identities(
        state: &AppState,
        config_path: &std::path::Path,
        mut candidate: AppGeneration,
    ) -> AppGeneration {
        let app_config =
            crate::app::load_instance_config_from_path(config_path).expect("config loads");
        let selected_packages = selected_summary_packages(state, &app_config, &candidate.bindings)
            .expect("selected packages resolve");
        let lock_bindings = summary_lock_bindings(&candidate.bindings, &selected_packages);
        candidate.binding_set_id = binding_set_id_from_lock_bindings(&lock_bindings);
        candidate.closure_id = closure_id_from_lock_packages(&selected_packages);
        candidate
    }

    fn test_resolved_app(name: &str, instance_dir: Option<&std::path::Path>) -> ResolvedApp {
        ResolvedApp {
            name: name.into(),
            version: "0.1.0".into(),
            capabilities: vec!["core.execution".into()],
            bindings: vec![test_binding("core.execution", "core")],
            sources: ResolvedAppSources {
                manifest_path: String::new(),
                config_path: instance_dir.map(|dir| dir.join("config.toml").display().to_string()),
                lock_path: instance_dir.map(|dir| dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        }
    }

    fn write_text_file(path: &std::path::Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir created");
        }
        fs::write(path, contents).expect("file written");
    }

    fn test_package_source(
        name: &str,
        package_kind: &str,
        provides: Vec<&str>,
        requires: Vec<&str>,
    ) -> PackageSource {
        PackageSource {
            name: name.into(),
            kind: if package_kind.contains("feature") {
                "embedded".into()
            } else {
                "wasm".into()
            },
            package_kind: package_kind.into(),
            runtime_provider: name.into(),
            current_source: format!("packages/official/{name}"),
            trusted: true,
            signature: "builtin:test".into(),
            source_authority: "test".into(),
            source_public_keys: vec![],
            provides: provides.into_iter().map(String::from).collect(),
            requires: requires.into_iter().map(String::from).collect(),
        }
    }

    fn state_for_propose_generation(
        repo_root: std::path::PathBuf,
        app: ResolvedApp,
        package_sources: Vec<PackageSource>,
    ) -> AppState {
        let mut apps = ResolvedAppMap::new();
        apps.insert(app.name.clone(), app);

        AppState {
            resolved_apps: Arc::new(RwLock::new(apps)),
            package_index: Arc::new(PackageIndex {
                version: 1,
                revision: "test-rev".into(),
                source_url: "local://packages".into(),
                package_sources,
            }),
            ..test_state(repo_root, CapabilityRegistry::new())
        }
    }

    fn state_for_verify_generation(
        repo_root: std::path::PathBuf,
        app: ResolvedApp,
        package_sources: Vec<PackageSource>,
        capability_registry: CapabilityRegistry,
        candidate: AppGeneration,
    ) -> AppState {
        let mut stores = GenerationStoreMap::new();
        stores.insert(
            app.name.clone(),
            AppGenerationStore {
                active: None,
                candidate: Some(candidate),
                rollback: None,
                next_id: 2,
            },
        );

        AppState {
            resolved_apps: Arc::new(RwLock::new({
                let mut apps = ResolvedAppMap::new();
                apps.insert(app.name.clone(), app);
                apps
            })),
            generation_store: Arc::new(RwLock::new(stores)),
            package_index: Arc::new(PackageIndex {
                version: 1,
                revision: "test-rev".into(),
                source_url: "local://packages".into(),
                package_sources,
            }),
            ..test_state(repo_root, capability_registry)
        }
    }

    fn write_test_manifest_and_config(
        repo_root: &std::path::Path,
        active_scene: Option<&str>,
        scene_block: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let package_dir = repo_root.join("packages").join("weft-claw");
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        let manifest_path = package_dir.join("package.toml");
        let config_path = instance_dir.join("config.toml");

        write_text_file(
            &manifest_path,
            r#"schema_version = 1
[identity]
name = 'weft-claw'
version = '0.1.0'
display_name = 'WEFT Claw'
description = 'test product'
[package]
kind = 'product'
[requires]
capabilities = ['agent.runtime']
[bindings."agent.runtime"]
provider = 'agent-runtime'
mutable = false
[features]
default_enabled = ['feature-default']
optional = ['feature-scene-on', 'feature-off', 'feature-extra']
[feature_bindings.feature-default]
requires = []
packages = ['feature-default']
[feature_bindings.feature-scene-on]
requires = []
packages = ['feature-scene-on']
[feature_bindings.feature-off]
requires = []
packages = ['feature-off']
[feature_bindings.feature-extra]
requires = []
packages = ['feature-extra']
"#,
        );

        let active_scene_line = active_scene
            .map(|value| format!("active_scene = '{value}'\n"))
            .unwrap_or_default();
        write_text_file(
            &config_path,
            &format!(
                "schema_version = 1\n{active_scene_line}\n[app_runtime]\nprofile = 'developer'\n\n[features]\nenabled = ['feature-default', 'feature-off']\ndisabled = []\n\n[packages]\nenabled = ['feature-default', 'feature-off']\ndisabled = []\n\n{scene_block}"
            ),
        );

        (package_dir, instance_dir, manifest_path)
    }

    fn generation_with_id(id: u64, status: GenerationStatus) -> AppGeneration {
        AppGeneration {
            id,
            app_name: "weft-claw".into(),
            version: "0.1.0".into(),
            bindings: vec![test_binding("core.execution", "core")],
            capabilities: vec!["core.execution".into()],
            enabled_features: vec![],
            scene: "stable".into(),
            profile: "developer".into(),
            binding_set_id: format!("binding-set:sha256:{id}"),
            closure_id: format!("closure:sha256:{id}"),
            lock_digest: format!("sha256:lock-{id}"),
            lock_path: format!("generations/{id}.lock.toml"),
            parent_generation: id.checked_sub(1),
            created_by: "api".into(),
            status,
            validation_results: vec![],
            created_at: 1710000000 + id,
        }
    }

    fn generation_with_validation_results(
        id: u64,
        status: GenerationStatus,
        profile: &str,
        validation_results: Vec<crate::app::ValidationResult>,
    ) -> AppGeneration {
        AppGeneration {
            profile: profile.into(),
            validation_results,
            ..generation_with_id(id, status)
        }
    }

    #[tokio::test]
    async fn propose_generation_uses_active_scene_summary_metadata() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team-scene'
profile = 'safe'
base_generation = 41
enabled_features = ['feature-extra']
disabled_features = []
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
                test_package_source("feature-extra", "feature", vec![], vec![]),
            ],
        );

        let response = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect("proposal succeeds");

        let generation = &response.0["generation"];
        assert_eq!(generation["scene"], "team-scene");
        assert_eq!(generation["profile"], "safe");
        assert_eq!(generation["parent_generation"], 41);
        assert_eq!(generation["status"], "candidate");
    }

    #[tokio::test]
    async fn propose_generation_overlays_scene_feature_selection_on_candidate() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'
enabled_features = ['feature-scene-on']
disabled_features = ['feature-off']

[[scenes.team.binding_pins]]
capability = 'agent.runtime'
provider = 'scene-provider'
package = 'scene-provider'

[[scenes.team.package_pins]]
package = 'scene-package'
version = '0.1.0'
source = 'local-index'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
                test_package_source("feature-scene-on", "feature", vec![], vec![]),
                test_package_source("feature-extra", "feature", vec![], vec![]),
                test_package_source("scene-package", "feature", vec![], vec![]),
            ],
        );

        let response = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect("proposal succeeds");

        let generation = &response.0["generation"];
        let enabled_features = generation["enabled_features"]
            .as_array()
            .expect("enabled feature array")
            .iter()
            .map(|value| value.as_str().expect("feature string"))
            .collect::<Vec<_>>();
        assert_eq!(
            enabled_features,
            vec!["feature-default", "feature-scene-on"]
        );

        let bindings = generation["bindings"].as_array().expect("bindings array");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0]["provider"], "scene-provider");
        assert_eq!(bindings[0]["source"], "scene:team");
    }

    #[tokio::test]
    async fn propose_generation_scene_binding_pin_adds_binding_for_requested_capability() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'

[[scenes.team.binding_pins]]
capability = 'team.delegate'
package = 'team-runtime'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("team-runtime", "provider", vec!["team.delegate"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
            ],
        );

        let response = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({
                "capabilities": ["agent.runtime", "team.delegate"]
            })),
        )
        .await
        .expect("proposal succeeds");

        let bindings = response.0["generation"]["bindings"]
            .as_array()
            .expect("bindings array");
        assert_eq!(bindings.len(), 2);
        assert!(bindings.iter().any(|binding| {
            binding["capability"] == "team.delegate"
                && binding["provider"] == "team-runtime"
                && binding["mutable"] == false
                && binding["source"] == "scene:team"
        }));
    }

    #[tokio::test]
    async fn propose_generation_scene_binding_pin_ignores_absent_capability() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'

[[scenes.team.binding_pins]]
capability = 'team.delegate'
provider = 'team-runtime'
package = 'team-runtime'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("team-runtime", "provider", vec!["team.delegate"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
            ],
        );

        let response = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect("proposal succeeds");

        let bindings = response.0["generation"]["bindings"]
            .as_array()
            .expect("bindings array");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0]["capability"], "agent.runtime");
        assert_eq!(bindings[0]["provider"], "agent-runtime");
        assert!(!bindings
            .iter()
            .any(|binding| binding["capability"] == "team.delegate"));
    }

    #[tokio::test]
    async fn propose_generation_keeps_legacy_behavior_without_matching_active_scene() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) =
            write_test_manifest_and_config(&repo_root, Some("missing-scene"), "");

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
            ],
        );

        let response = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect("proposal succeeds");

        let generation = &response.0["generation"];
        assert_eq!(generation["scene"], "missing-scene");
        assert_eq!(generation["profile"], "developer");
        assert!(generation["parent_generation"].is_null());
        let enabled_features = generation["enabled_features"]
            .as_array()
            .expect("enabled feature array")
            .iter()
            .map(|value| value.as_str().expect("feature string"))
            .collect::<Vec<_>>();
        assert_eq!(enabled_features, vec!["feature-default", "feature-off"]);
    }

    #[tokio::test]
    async fn propose_generation_rejects_duplicate_binding_pin_conflicts() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'

[[scenes.team.binding_pins]]
capability = 'agent.runtime'
provider = 'agent-runtime'
package = 'agent-runtime'
version = '0.1.0'
sha512 = 'sha512:111'

[[scenes.team.binding_pins]]
capability = 'agent.runtime'
provider = 'agent-runtime-alt'
package = 'agent-runtime-alt'
version = '0.2.0'
sha512 = 'sha512:222'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
        );

        let (status, body) = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect_err("proposal should fail for conflicting binding pins");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        let payload = body.0;
        assert_eq!(payload["reason"], "scene_pin_conflict");
        assert_eq!(payload["conflict"]["type"], "duplicate_binding_pin");
        assert_eq!(payload["conflict"]["scene"], "team");
        assert_eq!(payload["conflict"]["capability"], "agent.runtime");
        assert_eq!(payload["conflict"]["indexes"], serde_json::json!([0, 1]));

        let store = state.generation_store.read().await;
        assert!(store.get("weft-claw").is_none());
    }

    #[tokio::test]
    async fn propose_generation_rejects_duplicate_package_pin_conflicts() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'

[[scenes.team.package_pins]]
package = 'agent-runtime'
version = '0.1.0'
sha512 = 'sha512:111'
source = 'local-index'

[[scenes.team.package_pins]]
package = 'agent-runtime'
version = '0.2.0'
sha512 = 'sha512:222'
source = 'local-index'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
        );

        let (status, body) = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect_err("proposal should fail for conflicting package pins");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        let payload = body.0;
        assert_eq!(payload["reason"], "scene_pin_conflict");
        assert_eq!(payload["conflict"]["type"], "duplicate_package_pin");
        assert_eq!(payload["conflict"]["scene"], "team");
        assert_eq!(payload["conflict"]["package"], "agent-runtime");
        assert_eq!(payload["conflict"]["indexes"], serde_json::json!([0, 1]));

        let store = state.generation_store.read().await;
        assert!(store.get("weft-claw").is_none());
    }

    #[tokio::test]
    async fn propose_generation_rejects_binding_package_pin_mismatch() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'

[[scenes.team.binding_pins]]
capability = 'agent.runtime'
provider = 'agent-runtime'
package = 'agent-runtime'
version = '0.1.0'
sha512 = 'sha512:111'
source = 'local-index'

[[scenes.team.package_pins]]
package = 'agent-runtime'
version = '0.2.0'
sha512 = 'sha512:111'
source = 'local-index'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
        );

        let (status, body) = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect_err("proposal should fail for binding/package mismatch");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        let payload = body.0;
        assert_eq!(payload["reason"], "scene_pin_conflict");
        assert_eq!(payload["conflict"]["type"], "binding_package_mismatch");
        assert_eq!(payload["conflict"]["scene"], "team");
        assert_eq!(payload["conflict"]["capability"], "agent.runtime");
        assert_eq!(payload["conflict"]["package"], "agent-runtime");
        assert_eq!(payload["conflict"]["indexes"], serde_json::json!([0, 0]));

        let store = state.generation_store.read().await;
        assert!(store.get("weft-claw").is_none());
    }

    #[tokio::test]
    async fn propose_generation_accepts_non_conflicting_scene_pins_metadata_only() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'
profile = 'developer'
enabled_features = ['feature-extra']

[[scenes.team.binding_pins]]
capability = 'agent.runtime'
provider = 'agent-runtime'
package = 'agent-runtime'
version = '0.1.0'
sha512 = 'sha512:111'
source = 'local-index'

[[scenes.team.package_pins]]
package = 'agent-runtime'
version = '0.1.0'
sha512 = 'sha512:111'
source = 'local-index'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
                test_package_source("feature-extra", "feature", vec![], vec![]),
            ],
        );

        let response = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect("proposal succeeds for non-conflicting scene pins");

        let generation = &response.0["generation"];
        assert_eq!(response.0["status"], "proposed");
        assert_eq!(generation["scene"], "team");
        assert_eq!(generation["profile"], "developer");
        assert_eq!(generation["status"], "candidate");
        let bindings = generation["bindings"].as_array().expect("bindings array");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0]["provider"], "agent-runtime");
        assert_eq!(bindings[0]["source"], "scene:team");

        let store = state.generation_store.read().await;
        let app_store = store.get("weft-claw").expect("store entry present");
        assert!(app_store.candidate.is_some());
        assert_eq!(app_store.next_generation_id(), 2);
    }

    #[tokio::test]
    async fn propose_generation_scene_package_pin_adds_package_to_candidate_closure_summary() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'

[[scenes.team.package_pins]]
package = 'scene-package'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
                test_package_source("scene-package", "provider", vec!["scene.extra"], vec![]),
            ],
        );

        let _ = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect("proposal succeeds");

        let candidate = {
            let store = state.generation_store.read().await;
            store
                .get("weft-claw")
                .and_then(|app_store| app_store.candidate.clone())
                .expect("candidate stored")
        };

        let app_config =
            crate::app::load_instance_config_from_path(&instance_dir.join("config.toml"))
                .expect("config loads");
        let selected_packages = selected_summary_packages(&state, &app_config, &candidate.bindings)
            .expect("selected packages resolve");

        assert!(selected_packages
            .iter()
            .any(|package| package.name == "scene-package"));
        assert_eq!(
            candidate.closure_id,
            closure_id_from_lock_packages(&selected_packages)
        );
    }

    #[tokio::test]
    async fn propose_generation_scene_package_pin_copies_exact_metadata_into_summary() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'

[[scenes.team.package_pins]]
package = 'scene-package'
version = '1.2.3'
sha512 = 'sha512:scene-package'
source = 'local-index'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let state = state_for_propose_generation(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
                test_package_source("scene-package", "provider", vec!["scene.extra"], vec![]),
            ],
        );

        let response = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect("proposal succeeds");

        let candidate = {
            let store = state.generation_store.read().await;
            store
                .get("weft-claw")
                .and_then(|app_store| app_store.candidate.clone())
                .expect("candidate stored")
        };

        let app_config =
            crate::app::load_instance_config_from_path(&instance_dir.join("config.toml"))
                .expect("config loads");
        let selected_packages = selected_summary_packages(&state, &app_config, &candidate.bindings)
            .expect("selected packages resolve");
        let scene_package = selected_packages
            .iter()
            .find(|package| package.name == "scene-package")
            .expect("scene package present");

        assert_eq!(scene_package.version, "1.2.3");
        assert_eq!(scene_package.sha512, "sha512:scene-package");
        assert_eq!(scene_package.source, "local-index");
        assert_eq!(
            response.0["generation"]["closure_id"],
            candidate.closure_id.as_str()
        );
    }

    #[tokio::test]
    async fn propose_generation_rejects_unresolved_scene_package_pin_without_replacing_candidate() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'

[[scenes.team.package_pins]]
package = 'missing-package'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };

        let existing_candidate = generation_with_id(9, GenerationStatus::Candidate);
        let state = state_for_propose_generation_with_store(
            repo_root,
            app,
            vec![
                test_package_source("agent-runtime", "provider", vec!["agent.runtime"], vec![]),
                test_package_source("feature-default", "feature", vec![], vec![]),
                test_package_source("feature-off", "feature", vec![], vec![]),
            ],
            AppGenerationStore {
                active: None,
                candidate: Some(existing_candidate.clone()),
                rollback: None,
                next_id: 10,
            },
        );

        let (status, body) = propose_generation(
            Path("weft-claw".into()),
            State(state.clone()),
            Json(serde_json::json!({})),
        )
        .await
        .expect_err("proposal should fail for unresolved package pin");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        let payload = body.0;
        assert_eq!(payload["reason"], "scene_package_pin_unresolved");
        assert_eq!(payload["scene"], "team");
        assert_eq!(payload["package"], "missing-package");
        assert_eq!(payload["index"], 0);

        let store = state.generation_store.read().await;
        let candidate = store
            .get("weft-claw")
            .and_then(|app_store| app_store.candidate.as_ref())
            .expect("existing candidate preserved");
        assert_eq!(candidate.id, existing_candidate.id);
        assert_eq!(candidate.closure_id, existing_candidate.closure_id);
    }

    fn state_with_generation_store(
        repo_root: std::path::PathBuf,
        app: ResolvedApp,
        app_store: AppGenerationStore,
    ) -> AppState {
        let mut apps = ResolvedAppMap::new();
        apps.insert(app.name.clone(), app);

        let mut stores = GenerationStoreMap::new();
        stores.insert("weft-claw".into(), app_store);

        AppState {
            resolved_apps: Arc::new(RwLock::new(apps)),
            generation_store: Arc::new(RwLock::new(stores)),
            ..test_state(repo_root, CapabilityRegistry::new())
        }
    }

    fn state_for_propose_generation_with_store(
        repo_root: std::path::PathBuf,
        app: ResolvedApp,
        package_sources: Vec<PackageSource>,
        app_store: AppGenerationStore,
    ) -> AppState {
        let mut state = state_for_propose_generation(repo_root, app, package_sources);
        state.generation_store = Arc::new(RwLock::new({
            let mut stores = GenerationStoreMap::new();
            stores.insert("weft-claw".into(), app_store);
            stores
        }));
        state
    }

    #[tokio::test]
    async fn activate_generation_persists_repairable_generation_index_after_success() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_id(2, GenerationStatus::Verified)),
                rollback: None,
                next_id: 3,
            },
        );

        let response = activate_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("activation succeeds");

        let payload = response.0;
        assert_eq!(payload["status"], "activated");
        assert_eq!(payload["generation_lock_written"], true);
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["pointer_written"], true);
        assert!(payload["pointer_error"].is_null());
        assert_eq!(payload["index_written"], true);
        assert!(payload["index_error"].is_null());

        let generation_lock_path = instance_dir.join("generations").join("2.lock.toml");
        assert!(generation_lock_path.exists());
        let generation_lock = crate::app::load_instance_lock_from_path(&generation_lock_path)
            .expect("generation lock loads");
        assert_eq!(generation_lock.generation, 2);
        assert_eq!(generation_lock.status, "active");

        let root_lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("root lock loads");
        assert_eq!(root_lock.generation, 2);
        assert_eq!(root_lock.status, "active");

        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            Some(1)
        );
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            Some(2)
        );

        let index = crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .expect("index present");
        assert_eq!(index.active, Some(2));
        assert_eq!(index.previous, Some(1));
        assert_eq!(index.candidate, None);
        assert_eq!(index.next_id, 3);
        assert_eq!(index.generations.len(), 2);
    }

    #[tokio::test]
    async fn activate_generation_succeeds_when_generation_index_persist_fails() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");
        fs::write(
            instance_dir.join("generation-store.toml"),
            "schema_version = 1\nnext_id = 1\ngenerations = []\n",
        )
        .expect("existing generation index seeded");
        fs::create_dir_all(instance_dir.join("generation-store.toml.bak"))
            .expect("backup dir created to force index write failure");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_id(2, GenerationStatus::Verified)),
                rollback: None,
                next_id: 3,
            },
        );

        let response = activate_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("activation still succeeds");

        let payload = response.0;
        assert_eq!(payload["status"], "activated");
        assert_eq!(payload["generation_lock_written"], true);
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["pointer_written"], true);
        assert!(payload["pointer_error"].is_null());
        assert_eq!(payload["index_written"], false);
        assert!(payload["index_error"]
            .as_str()
            .expect("index error string")
            .contains("Failed to persist repairable generation index"));
    }

    #[tokio::test]
    async fn activate_generation_fails_when_previous_pointer_write_fails() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");
        fs::write(previous_generation_pointer_path(&instance_dir), "0\n")
            .expect("previous pointer seeded");
        fs::create_dir_all(instance_dir.join("previous.bak"))
            .expect("backup dir created to force previous pointer write failure");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_id(2, GenerationStatus::Verified)),
                rollback: None,
                next_id: 3,
            },
        );

        let (status, body) = activate_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("activation should fail when previous pointer write fails");

        assert_eq!(status, HttpStatusCode::INTERNAL_SERVER_ERROR);
        let payload = body.0;
        assert_eq!(payload["status"], "activation_failed");
        assert_eq!(payload["reason"], "pointer_write_failed");
        assert_eq!(payload["generation_lock_written"], true);
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["pointer_error"]["stage"], "previous");
        assert_eq!(payload["pointer_error"]["repair_needed"], false);
        assert_eq!(payload["index_written"], false);
        assert!(payload["index_error"].is_null());
        assert!(payload["error"]
            .as_str()
            .expect("error string")
            .contains("Failed to write previous generation pointer"));

        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            None
        );
        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            Some(0)
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn activate_generation_fails_when_active_pointer_write_fails_and_skips_index() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");
        fs::write(active_generation_pointer_path(&instance_dir), "1\n")
            .expect("active pointer seeded");
        fs::create_dir_all(instance_dir.join("active.bak"))
            .expect("backup dir created to force active pointer write failure");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_id(2, GenerationStatus::Verified)),
                rollback: None,
                next_id: 3,
            },
        );

        let (status, body) = activate_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("activation should fail when active pointer write fails");

        assert_eq!(status, HttpStatusCode::INTERNAL_SERVER_ERROR);
        let payload = body.0;
        assert_eq!(payload["status"], "activation_failed");
        assert_eq!(payload["reason"], "pointer_write_failed");
        assert_eq!(payload["generation_lock_written"], true);
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["pointer_error"]["stage"], "active");
        assert_eq!(payload["pointer_error"]["repair_needed"], true);
        assert_eq!(payload["index_written"], false);
        assert!(payload["index_error"].is_null());
        assert!(payload["pointer_error"]["message"]
            .as_str()
            .expect("pointer error message")
            .contains("repair is required"));

        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            Some(1)
        );
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            Some(1)
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn activate_generation_allows_existing_same_generation_lock() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("root lock seeded");

        let first_state = state_with_generation_store(
            root.path().to_path_buf(),
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_id(2, GenerationStatus::Verified)),
                rollback: None,
                next_id: 3,
            },
        );

        let first_response = activate_generation(Path("weft-claw".into()), State(first_state))
            .await
            .expect("first activation succeeds");
        assert_eq!(first_response.0["generation_lock_written"], true);

        let second_state = state_with_generation_store(
            root.path().to_path_buf(),
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_id(2, GenerationStatus::Verified)),
                rollback: None,
                next_id: 3,
            },
        );

        let second_response = activate_generation(Path("weft-claw".into()), State(second_state))
            .await
            .expect("second activation succeeds with same immutable lock");
        let payload = second_response.0;
        assert_eq!(payload["status"], "activated");
        assert_eq!(payload["generation_lock_written"], true);
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["pointer_written"], true);
        assert_eq!(payload["index_written"], true);

        let generation_lock = crate::app::load_instance_lock_from_path(
            &instance_dir.join("generations").join("2.lock.toml"),
        )
        .expect("generation lock still loads");
        assert_eq!(generation_lock.generation, 2);

        let root_lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("root lock loads after idempotent activation");
        assert_eq!(root_lock.generation, 2);
    }

    #[tokio::test]
    async fn activate_generation_fails_when_existing_generation_lock_differs_and_skips_root_pointers_and_index(
    ) {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("generations dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"stale-root\"\n")
            .expect("root lock seeded");
        fs::write(
            instance_dir.join("generations").join("2.lock.toml"),
            "lock_version = 2\napp = 'weft-claw'\ngeneration = 2\nstatus = 'verified'\nprofile = 'developer'\nscene = 'conflict'\n",
        )
        .expect("conflicting generation lock seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_id(2, GenerationStatus::Verified)),
                rollback: None,
                next_id: 3,
            },
        );

        let (status, body) = activate_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("activation should fail on immutable generation lock conflict");

        assert_eq!(status, HttpStatusCode::CONFLICT);
        let payload = body.0;
        assert_eq!(payload["status"], "activation_failed");
        assert_eq!(payload["reason"], "generation_lock_conflict");
        assert_eq!(payload["lock_written"], false);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["index_written"], false);
        assert_eq!(payload["generation_lock_error"]["type"], "content_conflict");

        let root_lock_contents =
            fs::read_to_string(instance_dir.join("lock.toml")).expect("root lock still readable");
        assert_eq!(root_lock_contents, "app = \"stale-root\"\n");
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            None
        );
        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            None
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn rollback_generation_persists_repairable_generation_index_after_success() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let response = rollback_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("rollback succeeds");

        let payload = response.0;
        assert_eq!(payload["status"], "rolled_back");
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["pointer_written"], true);
        assert!(payload["pointer_error"].is_null());
        assert_eq!(payload["index_written"], true);
        assert!(payload["index_error"].is_null());

        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            Some(2)
        );
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            Some(1)
        );

        let index = crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .expect("index present");
        assert_eq!(index.active, Some(1));
        assert_eq!(index.previous, None);
        assert_eq!(index.candidate, Some(2));
        assert_eq!(index.next_id, 4);
        assert_eq!(index.generations.len(), 2);
    }

    #[tokio::test]
    async fn rollback_generation_replays_immutable_lock_into_root_mirror() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");
        write_text_file(
            &instance_dir.join("generations").join("1.lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 1
status = 'verified'
profile = 'developer'
scene = 'rollback-scene'
scene_digest = 'sha256:scene-1'
binding_set_id = 'binding-set:sha256:1'
closure_id = 'closure:sha256:1'
closure_digest = 'sha256:closure-1'
trust_level = 'verified'

[assembly]
enabled_features = ['immutable-feature']
selected_packages = ['immutable-package']
scene = 'rollback-scene'
scene_digest = 'sha256:scene-1'
binding_set_id = 'binding-set:sha256:1'
closure_id = 'closure:sha256:1'
closure_digest = 'sha256:closure-1'

[notes]
message = 'immutable lock payload'
"#,
        );

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let response = rollback_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("rollback succeeds");

        let payload = response.0;
        assert_eq!(payload["status"], "rolled_back");
        assert_eq!(payload["generation_lock_replayed"], true);
        assert!(payload["generation_lock_warning"].is_null());

        let root_lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("root lock loads");
        assert_eq!(root_lock.generation, 1);
        assert_eq!(root_lock.status, "active");
        assert_eq!(root_lock.scene, "rollback-scene");
        assert_eq!(root_lock.scene_digest, "sha256:scene-1");
        assert_eq!(root_lock.binding_set_id, "binding-set:sha256:1");
        assert_eq!(root_lock.closure_id, "closure:sha256:1");
        assert_eq!(root_lock.closure_digest, "sha256:closure-1");
        assert_eq!(
            root_lock.assembly.enabled_features,
            vec!["immutable-feature"]
        );
        assert_eq!(
            root_lock.assembly.selected_packages,
            vec!["immutable-package".to_string()]
        );
        assert_eq!(root_lock.notes.message, "immutable lock payload");
    }

    #[tokio::test]
    async fn rollback_generation_falls_back_when_immutable_lock_is_missing() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        write_text_file(
            &instance_dir.join("lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 2
status = 'active'
profile = 'developer'
scene = 'existing-scene'
scene_digest = 'sha256:existing-scene'
binding_set_id = 'binding-set:sha256:existing'
closure_id = 'closure:sha256:existing'
closure_digest = 'sha256:closure-existing'

[assembly]
enabled_features = ['existing-feature']
selected_packages = ['existing-package']
scene = 'existing-scene'
scene_digest = 'sha256:existing-scene'
binding_set_id = 'binding-set:sha256:existing'
closure_id = 'closure:sha256:existing'
closure_digest = 'sha256:closure-existing'

[notes]
message = 'existing root lock'
"#,
        );

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let response = rollback_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("rollback succeeds with fallback");

        let payload = response.0;
        assert_eq!(payload["status"], "rolled_back");
        assert_eq!(payload["generation_lock_replayed"], false);
        assert_eq!(
            payload["generation_lock_warning"]["type"],
            "immutable_lock_missing"
        );

        let root_lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("root lock loads");
        assert_eq!(root_lock.generation, 1);
        assert_eq!(root_lock.status, "active");
        assert_eq!(root_lock.scene, "existing-scene");
        assert_eq!(root_lock.scene_digest, "sha256:existing-scene");
        assert_eq!(root_lock.binding_set_id, "binding-set:sha256:existing");
        assert_eq!(root_lock.closure_id, "closure:sha256:existing");
        assert_eq!(root_lock.closure_digest, "sha256:closure-existing");
        assert_eq!(root_lock.assembly.enabled_features, Vec::<String>::new());
        assert_eq!(
            root_lock.assembly.selected_packages,
            vec!["existing-package".to_string()]
        );
        assert_eq!(
            root_lock.notes.message,
            "Rolled back to generation 1 via API"
        );
    }

    #[tokio::test]
    async fn rollback_generation_fails_before_writes_when_immutable_lock_generation_mismatches() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"stale-root\"\n")
            .expect("lock file seeded");
        write_text_file(
            &instance_dir.join("generations").join("1.lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 99
status = 'verified'
profile = 'developer'
scene = 'wrong-generation'
"#,
        );

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let (status, body) = rollback_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("rollback should fail on mismatched immutable lock");

        assert_eq!(status, HttpStatusCode::CONFLICT);
        let payload = body.0;
        assert_eq!(payload["reason"], "generation_lock_replay_failed");
        assert_eq!(payload["status"], "rollback_failed");
        assert_eq!(payload["generation_lock_replayed"], false);
        assert_eq!(
            payload["generation_lock_error"]["type"],
            "generation_mismatch"
        );
        assert_eq!(payload["lock_written"], false);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["index_written"], false);

        let root_lock_contents =
            fs::read_to_string(instance_dir.join("lock.toml")).expect("root lock still readable");
        assert_eq!(root_lock_contents, "app = \"stale-root\"\n");
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            None
        );
        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            None
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn rollback_generation_fails_when_previous_pointer_write_fails() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");
        fs::write(previous_generation_pointer_path(&instance_dir), "0\n")
            .expect("previous pointer seeded");
        fs::create_dir_all(instance_dir.join("previous.bak"))
            .expect("backup dir created to force previous pointer write failure");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let (status, body) = rollback_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("rollback should fail when previous pointer write fails");

        assert_eq!(status, HttpStatusCode::INTERNAL_SERVER_ERROR);
        let payload = body.0;
        assert_eq!(payload["status"], "rollback_failed");
        assert_eq!(payload["reason"], "pointer_write_failed");
        assert_eq!(payload["operation"], "rollback");
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["pointer_error"]["stage"], "previous");
        assert_eq!(payload["pointer_error"]["repair_needed"], false);
        assert_eq!(payload["index_written"], false);
        assert!(payload["index_error"].is_null());

        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            Some(0)
        );
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            None
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn rollback_generation_fails_when_active_pointer_write_fails_and_skips_index() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");
        fs::write(active_generation_pointer_path(&instance_dir), "2\n")
            .expect("active pointer seeded");
        fs::create_dir_all(instance_dir.join("active.bak"))
            .expect("backup dir created to force active pointer write failure");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let (status, body) = rollback_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("rollback should fail when active pointer write fails");

        assert_eq!(status, HttpStatusCode::INTERNAL_SERVER_ERROR);
        let payload = body.0;
        assert_eq!(payload["status"], "rollback_failed");
        assert_eq!(payload["reason"], "pointer_write_failed");
        assert_eq!(payload["operation"], "rollback");
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["pointer_error"]["stage"], "active");
        assert_eq!(payload["pointer_error"]["repair_needed"], true);
        assert_eq!(payload["index_written"], false);
        assert!(payload["index_error"].is_null());

        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            Some(2)
        );
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            Some(2)
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn activate_existing_generation_switches_to_verified_rollback_generation_and_replays_lock(
    ) {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("instance dir created");
        write_text_file(
            &instance_dir.join("lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 2
status = 'active'
profile = 'developer'
scene = 'current-scene'
scene_digest = 'sha256:current-scene'
binding_set_id = 'binding-set:sha256:current'
closure_id = 'closure:sha256:current'
closure_digest = 'sha256:closure-current'

[assembly]
enabled_features = ['current-feature']
selected_packages = ['current-package']
scene = 'current-scene'
scene_digest = 'sha256:current-scene'
binding_set_id = 'binding-set:sha256:current'
closure_id = 'closure:sha256:current'
closure_digest = 'sha256:closure-current'

[notes]
message = 'current active root lock'
"#,
        );
        write_text_file(
            &instance_dir.join("generations").join("1.lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 1
status = 'verified'
profile = 'developer'
scene = 'rollback-scene'
scene_digest = 'sha256:scene-1'
binding_set_id = 'binding-set:sha256:1'
closure_id = 'closure:sha256:1'
closure_digest = 'sha256:closure-1'
trust_level = 'verified'

[assembly]
enabled_features = ['immutable-feature']
selected_packages = ['immutable-package']
scene = 'rollback-scene'
scene_digest = 'sha256:scene-1'
binding_set_id = 'binding-set:sha256:1'
closure_id = 'closure:sha256:1'
closure_digest = 'sha256:closure-1'

[notes]
message = 'immutable rollback payload'
"#,
        );

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let response = activate_existing_generation(Path(("weft-claw".into(), 1)), State(state))
            .await
            .expect("switch succeeds");

        let payload = response.0;
        assert_eq!(payload["status"], "activated");
        assert_eq!(payload["generation"]["id"], 1);
        assert_eq!(payload["lock_written"], true);
        assert_eq!(payload["generation_lock_replayed"], true);
        assert!(payload["generation_lock_warning"].is_null());
        assert_eq!(payload["pointer_written"], true);
        assert_eq!(payload["index_written"], true);

        let root_lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("root lock loads");
        assert_eq!(root_lock.generation, 1);
        assert_eq!(root_lock.status, "active");
        assert_eq!(root_lock.scene, "rollback-scene");
        assert_eq!(root_lock.notes.message, "immutable rollback payload");
        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            Some(2)
        );
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            Some(1)
        );

        let index = crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .expect("index present");
        assert_eq!(index.active, Some(1));
        assert_eq!(index.previous, Some(2));
        assert_eq!(index.candidate, Some(3));
    }

    #[tokio::test]
    async fn activate_existing_generation_fails_when_target_is_missing() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let (status, body) =
            activate_existing_generation(Path(("weft-claw".into(), 99)), State(state))
                .await
                .expect_err("missing generation should fail");

        assert_eq!(status, HttpStatusCode::NOT_FOUND);
        let payload = body.0;
        assert_eq!(payload["reason"], "generation_not_found");
        assert_eq!(payload["status"], "activation_failed");
        assert_eq!(payload["generation"], 99);
        assert_eq!(payload["lock_written"], false);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["index_written"], false);
    }

    #[tokio::test]
    async fn get_generation_diff_returns_summary_diff_for_active_and_rollback() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");

        let rollback = generation_with_diff_fields(
            1,
            GenerationStatus::Rollback,
            "stable",
            "safe",
            vec!["feature-a"],
            vec!["cap.a"],
            vec![test_binding("cap.a", "provider-a")],
        );
        let active = generation_with_diff_fields(
            2,
            GenerationStatus::Active,
            "team",
            "developer",
            vec!["feature-a", "feature-b"],
            vec!["cap.a", "cap.b"],
            vec![
                test_binding("cap.a", "provider-b"),
                test_binding("cap.b", "provider-c"),
            ],
        );

        let response = get_generation_diff(
            Path(("weft-claw".into(), 2, 1)),
            State(state_with_generation_store(
                repo_root,
                test_resolved_app("weft-claw", Some(&instance_dir)),
                AppGenerationStore {
                    active: Some(active),
                    candidate: None,
                    rollback: Some(rollback),
                    next_id: 3,
                },
            )),
        )
        .await
        .expect("diff succeeds")
        .0;

        assert_eq!(response["app"], "weft-claw");
        assert_eq!(response["from"]["id"], 2);
        assert_eq!(response["to"]["id"], 1);
        assert_eq!(response["summary_diff"]["status"]["from"], "active");
        assert_eq!(response["summary_diff"]["status"]["to"], "rollback");
        assert_eq!(response["summary_diff"]["status"]["changed"], true);
        assert_eq!(response["summary_diff"]["scene"]["from"], "team");
        assert_eq!(response["summary_diff"]["scene"]["to"], "stable");
        assert_eq!(
            response["summary_diff"]["enabled_features"]["removed"],
            serde_json::json!(["feature-b"])
        );
        assert_eq!(
            response["summary_diff"]["capabilities"]["removed"],
            serde_json::json!(["cap.b"])
        );
        assert_eq!(response["summary_diff"]["bindings"]["changed"], true);
        assert_eq!(
            response["summary_diff"]["bindings"]["removed"][0]["capability"],
            "cap.b"
        );
        assert_eq!(
            response["summary_diff"]["bindings"]["changed_entries"][0]["capability"],
            "cap.a"
        );
        assert_eq!(response["immutable_lock"]["available"], false);
        assert!(!response["warnings"]
            .as_array()
            .expect("warnings array")
            .is_empty());
    }

    #[tokio::test]
    async fn get_generation_diff_returns_404_for_missing_generation() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");

        let (status, body) = get_generation_diff(
            Path(("weft-claw".into(), 2, 99)),
            State(state_with_generation_store(
                repo_root,
                test_resolved_app("weft-claw", Some(&instance_dir)),
                AppGenerationStore {
                    active: Some(generation_with_id(2, GenerationStatus::Active)),
                    candidate: None,
                    rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                    next_id: 3,
                },
            )),
        )
        .await
        .expect_err("missing generation should fail");

        assert_eq!(status, HttpStatusCode::NOT_FOUND);
        assert_eq!(body.0["reason"], "generation_not_found");
        assert_eq!(body.0["generation"], 99);
        assert_eq!(body.0["role"], "to");
    }

    #[tokio::test]
    async fn get_generation_diff_includes_immutable_lock_package_and_binding_diff() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("instance dir created");

        write_generation_lock_for_diff(
            &instance_dir,
            1,
            "stable",
            "safe",
            "closure:sha256:1",
            &["package-a"],
            &[("cap.a", "provider-a")],
        );
        write_generation_lock_for_diff(
            &instance_dir,
            2,
            "team",
            "developer",
            "closure:sha256:2",
            &["package-a", "package-b"],
            &[("cap.a", "provider-b"), ("cap.b", "provider-c")],
        );

        let response = get_generation_diff(
            Path(("weft-claw".into(), 1, 2)),
            State(state_with_generation_store(
                repo_root,
                test_resolved_app("weft-claw", Some(&instance_dir)),
                AppGenerationStore {
                    active: Some(generation_with_id(2, GenerationStatus::Active)),
                    candidate: None,
                    rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                    next_id: 3,
                },
            )),
        )
        .await
        .expect("diff succeeds")
        .0;

        assert_eq!(response["immutable_lock"]["available"], true);
        assert_eq!(
            response["immutable_lock"]["diff"]["packages"]["added"],
            serde_json::json!(["package-b"])
        );
        assert_eq!(
            response["immutable_lock"]["diff"]["bindings"]["added"][0]["capability"],
            "cap.b"
        );
        assert_eq!(
            response["immutable_lock"]["diff"]["bindings"]["changed_entries"][0]["capability"],
            "cap.a"
        );
        assert_eq!(response["immutable_lock"]["diff"]["scene"]["changed"], true);
        assert_eq!(
            response["immutable_lock"]["diff"]["profile"]["changed"],
            true
        );
        assert!(response["warnings"]
            .as_array()
            .expect("warnings array")
            .is_empty());
    }

    #[tokio::test]
    async fn get_generation_diff_returns_summary_diff_with_warning_when_immutable_lock_missing() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("instance dir created");

        write_generation_lock_for_diff(
            &instance_dir,
            1,
            "stable",
            "safe",
            "closure:sha256:1",
            &["package-a"],
            &[("cap.a", "provider-a")],
        );

        let response = get_generation_diff(
            Path(("weft-claw".into(), 1, 2)),
            State(state_with_generation_store(
                repo_root,
                test_resolved_app("weft-claw", Some(&instance_dir)),
                AppGenerationStore {
                    active: Some(generation_with_id(2, GenerationStatus::Active)),
                    candidate: None,
                    rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                    next_id: 3,
                },
            )),
        )
        .await
        .expect("diff succeeds")
        .0;

        assert_eq!(response["summary_diff"]["id"]["changed"], true);
        assert_eq!(response["immutable_lock"]["available"], false);
        assert!(response["immutable_lock"]["diff"].is_null());
        let warnings = response["warnings"].as_array().expect("warnings array");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0]["warning_type"], "immutable_lock_missing");
        assert_eq!(warnings[0]["generation"], 2);
    }

    #[tokio::test]
    async fn get_generation_diff_route_is_wired() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");

        let router = build_router(state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: None,
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 3,
            },
        ));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/apps/weft-claw/generations/2/diff/1")
                    .method("GET")
                    .body(Body::empty())
                    .expect("request built"),
            )
            .await
            .expect("router response");

        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");
        assert_eq!(payload["from"]["id"], 2);
        assert_eq!(payload["to"]["id"], 1);
    }

    #[tokio::test]
    async fn activate_existing_generation_fails_when_target_is_not_verified() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let (status, body) =
            activate_existing_generation(Path(("weft-claw".into(), 3)), State(state))
                .await
                .expect_err("candidate generation should fail");

        assert_eq!(status, HttpStatusCode::CONFLICT);
        let payload = body.0;
        assert_eq!(payload["reason"], "generation_not_verified");
        assert_eq!(payload["status"], "activation_failed");
        assert_eq!(payload["generation"], 3);
        assert_eq!(payload["lock_written"], false);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["index_written"], false);
    }

    #[tokio::test]
    async fn activate_existing_generation_fails_before_pointer_and_index_writes_when_immutable_lock_mismatches(
    ) {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"stale-root\"\n")
            .expect("lock file seeded");
        write_text_file(
            &instance_dir.join("generations").join("1.lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 99
status = 'verified'
profile = 'developer'
scene = 'wrong-generation'
"#,
        );

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );

        let (status, body) =
            activate_existing_generation(Path(("weft-claw".into(), 1)), State(state))
                .await
                .expect_err("mismatched immutable lock should fail");

        assert_eq!(status, HttpStatusCode::CONFLICT);
        let payload = body.0;
        assert_eq!(payload["reason"], "generation_lock_replay_failed");
        assert_eq!(payload["status"], "activation_failed");
        assert_eq!(payload["generation_lock_replayed"], false);
        assert_eq!(
            payload["generation_lock_error"]["type"],
            "generation_mismatch"
        );
        assert_eq!(payload["lock_written"], false);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["index_written"], false);

        let root_lock_contents =
            fs::read_to_string(instance_dir.join("lock.toml")).expect("root lock still readable");
        assert_eq!(root_lock_contents, "app = \"stale-root\"\n");
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            None
        );
        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            None
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn activate_existing_generation_blocks_safe_profile_when_verified_store_object_is_missing(
    ) {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");
        write_text_file(
            &instance_dir.join("generations").join("1.lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 1
status = 'verified'
profile = 'safe'
scene = 'safe-scene'

[[packages]]
name = 'agent-runtime'
version = '0.1.0'
store_object_id = 'store:sha512:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
store_path = '.weft/store/missing-agent-runtime-0.1.0'
"#,
        );

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: None,
                rollback: Some(generation_with_validation_results(
                    1,
                    GenerationStatus::Rollback,
                    "safe",
                    vec![crate::app::ValidationResult {
                        check: "store-policy:agent-runtime".into(),
                        passed: true,
                        message: "Store metadata captured during prior verification".into(),
                    }],
                )),
                next_id: 3,
            },
        );

        let (status, body) =
            activate_existing_generation(Path(("weft-claw".into(), 1)), State(state))
                .await
                .expect_err("missing verified store object should block safe activation");

        assert_eq!(status, HttpStatusCode::CONFLICT);
        let payload = body.0;
        assert_eq!(payload["reason"], "store_object_missing");
        assert_eq!(payload["status"], "activation_failed");
        assert_eq!(payload["lock_written"], false);
        assert_eq!(payload["pointer_written"], false);
        assert_eq!(payload["index_written"], false);
        assert_eq!(
            payload["store_validation"]["check"],
            "store-policy:agent-runtime"
        );
        assert!(payload["store_validation"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("path is missing"));
    }

    #[tokio::test]
    async fn activate_existing_generation_route_is_wired() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(instance_dir.join("generations")).expect("instance dir created");
        write_text_file(
            &instance_dir.join("lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 2
status = 'active'
profile = 'developer'
scene = 'current-scene'
scene_digest = 'sha256:current-scene'
binding_set_id = 'binding-set:sha256:current'
closure_id = 'closure:sha256:current'
closure_digest = 'sha256:closure-current'

[assembly]
enabled_features = ['current-feature']
selected_packages = ['current-package']
scene = 'current-scene'
scene_digest = 'sha256:current-scene'
binding_set_id = 'binding-set:sha256:current'
closure_id = 'closure:sha256:current'
closure_digest = 'sha256:closure-current'

[notes]
message = 'current active root lock'
"#,
        );
        write_text_file(
            &instance_dir.join("generations").join("1.lock.toml"),
            r#"lock_version = 2
app = 'weft-claw'
generation = 1
status = 'verified'
profile = 'developer'
scene = 'rollback-scene'
scene_digest = 'sha256:scene-1'
binding_set_id = 'binding-set:sha256:1'
closure_id = 'closure:sha256:1'
closure_digest = 'sha256:closure-1'

[assembly]
enabled_features = ['immutable-feature']
selected_packages = ['immutable-package']
scene = 'rollback-scene'
scene_digest = 'sha256:scene-1'
binding_set_id = 'binding-set:sha256:1'
closure_id = 'closure:sha256:1'
closure_digest = 'sha256:closure-1'

[notes]
message = 'immutable rollback payload'
"#,
        );

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(2, GenerationStatus::Active)),
                candidate: Some(generation_with_id(3, GenerationStatus::Candidate)),
                rollback: Some(generation_with_id(1, GenerationStatus::Rollback)),
                next_id: 4,
            },
        );
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/apps/weft-claw/generations/1/activate")
                    .body(Body::empty())
                    .expect("request built"),
            )
            .await
            .expect("router response");

        assert_eq!(response.status(), HttpStatusCode::OK);
    }

    #[tokio::test]
    async fn generation_diagnostics_returns_store_summary_without_instance_sources() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut apps = ResolvedAppMap::new();
        apps.insert("weft-claw".into(), test_resolved_app("weft-claw", None));

        let state = AppState {
            resolved_apps: Arc::new(RwLock::new(apps)),
            ..test_state(repo_root, CapabilityRegistry::new())
        };

        let diagnostics = generation_diagnostics_snapshot(&state, "weft-claw")
            .await
            .expect("diagnostics snapshot");

        assert_eq!(diagnostics.app, "weft-claw");
        assert!(!diagnostics.store_present);
        assert_eq!(diagnostics.store_summary.next_id, 1);
        assert!(diagnostics.store_summary.generations.is_empty());
        assert!(diagnostics.instance.is_none());
    }

    #[tokio::test]
    async fn generation_diagnostics_reports_pointer_mismatch_from_instance_files() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");

        write_active_generation_pointer(&instance_dir, Some(20)).expect("active pointer written");
        write_previous_generation_pointer(&instance_dir, Some(19))
            .expect("previous pointer written");
        save_generation_index(
            &instance_dir,
            &AppGenerationIndex {
                schema_version: crate::app::GENERATION_INDEX_SCHEMA_VERSION,
                active: Some(21),
                previous: Some(19),
                candidate: None,
                next_id: 22,
                generations: vec![
                    AppGeneration {
                        id: 19,
                        ..test_generation(
                            "core.execution",
                            "core",
                            "developer",
                            GenerationStatus::Rollback,
                        )
                    },
                    AppGeneration {
                        id: 20,
                        ..test_generation(
                            "core.execution",
                            "core",
                            "developer",
                            GenerationStatus::Active,
                        )
                    },
                    AppGeneration {
                        id: 21,
                        ..test_generation(
                            "core.execution",
                            "core",
                            "developer",
                            GenerationStatus::Verified,
                        )
                    },
                ],
            },
        )
        .expect("index saved");

        let mut apps = ResolvedAppMap::new();
        apps.insert(
            "weft-claw".into(),
            test_resolved_app("weft-claw", Some(&instance_dir)),
        );

        let mut stores = GenerationStoreMap::new();
        stores.insert(
            "weft-claw".into(),
            AppGenerationStore {
                active: Some(AppGeneration {
                    id: 20,
                    ..test_generation(
                        "core.execution",
                        "core",
                        "developer",
                        GenerationStatus::Active,
                    )
                }),
                candidate: None,
                rollback: Some(AppGeneration {
                    id: 19,
                    ..test_generation(
                        "core.execution",
                        "core",
                        "developer",
                        GenerationStatus::Rollback,
                    )
                }),
                next_id: 21,
            },
        );

        let state = AppState {
            generation_store: Arc::new(RwLock::new(stores)),
            resolved_apps: Arc::new(RwLock::new(apps)),
            ..test_state(repo_root, CapabilityRegistry::new())
        };

        let diagnostics = generation_diagnostics_snapshot(&state, "weft-claw")
            .await
            .expect("diagnostics snapshot");
        let instance = diagnostics.instance.expect("instance diagnostics present");
        let report = instance
            .consistency_report
            .expect("consistency report present");

        assert!(diagnostics.store_present);
        assert_eq!(diagnostics.store_summary.active, Some(20));
        assert_eq!(instance.active_pointer.generation_id, Some(20));
        assert_eq!(instance.previous_pointer.generation_id, Some(19));
        assert!(instance.generation_index.present);
        assert!(report.repair_recommended);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "pointer_mismatch"
                && diagnostic.pointer.as_deref() == Some("active")));
    }

    #[tokio::test]
    async fn generation_diagnostics_endpoint_returns_read_only_instance_health() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");

        write_active_generation_pointer(&instance_dir, Some(7)).expect("active pointer written");
        write_previous_generation_pointer(&instance_dir, Some(6))
            .expect("previous pointer written");
        save_generation_index(
            &instance_dir,
            &AppGenerationIndex {
                schema_version: crate::app::GENERATION_INDEX_SCHEMA_VERSION,
                active: Some(7),
                previous: Some(6),
                candidate: None,
                next_id: 8,
                generations: vec![
                    AppGeneration {
                        id: 6,
                        ..test_generation(
                            "core.execution",
                            "core",
                            "developer",
                            GenerationStatus::Rollback,
                        )
                    },
                    AppGeneration {
                        id: 7,
                        lock_path: "generations/7.lock.toml".into(),
                        scene: "stable".into(),
                        binding_set_id: "binding-set:sha256:7".into(),
                        closure_id: "closure:sha256:7".into(),
                        created_by: "api".into(),
                        lock_digest: "sha256:lock-7".into(),
                        ..test_generation(
                            "core.execution",
                            "core",
                            "developer",
                            GenerationStatus::Active,
                        )
                    },
                ],
            },
        )
        .expect("index saved");

        let mut apps = ResolvedAppMap::new();
        apps.insert(
            "weft-claw".into(),
            test_resolved_app("weft-claw", Some(&instance_dir)),
        );

        let state = AppState {
            resolved_apps: Arc::new(RwLock::new(apps)),
            ..test_state(repo_root, CapabilityRegistry::new())
        };
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/apps/weft-claw/generations/diagnostics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request handled");

        assert_eq!(response.status(), HttpStatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json payload");

        assert_eq!(payload["app"], "weft-claw");
        assert_eq!(payload["store_present"], false);
        assert_eq!(payload["store_summary"]["next_id"], 1);
        assert_eq!(payload["instance"]["active_pointer"]["generation_id"], 7);
        assert_eq!(payload["instance"]["previous_pointer"]["generation_id"], 6);
        assert_eq!(payload["instance"]["generation_index"]["present"], true);
        assert_eq!(
            payload["instance"]["consistency_report"]["is_consistent"],
            true
        );
        assert_eq!(
            payload["instance"]["generation_index"]["index"]["active"],
            7
        );
    }

    #[tokio::test]
    async fn live_probe_reports_core_execution_success() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
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
        let state = test_state(repo_root, registry);

        let generation = test_generation(
            "core.execution",
            "core",
            "developer",
            GenerationStatus::Candidate,
        );

        let results = live_probe_generation(&state, &generation).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].check, "probe:core.execution");
        assert!(results[0].passed);
        assert!(results[0]
            .message
            .contains("Provider responded to live core probe"));
    }

    #[tokio::test]
    async fn live_probe_reports_provider_rejection_for_mismatched_digest_signature() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join("test-live-probe-signature-rejection");
        let provider = "cap-provider";
        let source = "fixtures/cap-provider";
        let source_dir = repo_root.join(source);
        std::fs::create_dir_all(&source_dir).expect("source directory created");
        std::fs::write(source_dir.join("package.toml"), "name = 'cap-provider'\n")
            .expect("marker file written");

        let signing_key = SigningKey::from_bytes(&[11; 32]);
        let bad_message = signature_message(provider, "current", "deadbeef", source);
        let signature = sign_package_message(&signing_key, &bad_message);
        let source_public_key = signature
            .split(':')
            .nth(1)
            .expect("public key segment exists")
            .to_string();

        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "cap.test".into(),
            CapabilityRegistryEntry {
                capability: "cap.test".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: provider.into(),
                    runtime: "service".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );

        let state = AppState {
            package_index: Arc::new(PackageIndex {
                version: 1,
                revision: "test-rev".into(),
                source_url: "local://packages".into(),
                package_sources: vec![PackageSource {
                    name: provider.into(),
                    kind: "service".into(),
                    package_kind: String::new(),
                    runtime_provider: provider.into(),
                    current_source: source.into(),
                    trusted: true,
                    signature,
                    source_authority: "test-authority".into(),
                    source_public_keys: vec![source_public_key],
                    provides: vec![],
                    requires: vec![],
                }],
            }),
            ..test_state(repo_root, registry)
        };

        let generation = test_generation("cap.test", provider, "safe", GenerationStatus::Candidate);

        *state.active_profile.write().await = AppProfile::Safe;

        let results = live_probe_generation(&state, &generation).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].check, "probe:cap.test");
        assert!(!results[0].passed);
        assert!(results[0]
            .message
            .contains("Live probe failed with 403 Forbidden"));
        assert!(results[0]
            .message
            .contains("is not accepted under profile 'safe'"));
    }

    #[tokio::test]
    async fn live_probe_rejects_required_real_package_when_registry_marks_it_metadata() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "prompt.system".into(),
            CapabilityRegistryEntry {
                capability: "prompt.system".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: "prompt-system".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        let state = AppState {
            package_index: Arc::new(PackageIndex {
                version: 1,
                revision: "test-rev".into(),
                source_url: "local://packages".into(),
                package_sources: vec![PackageSource {
                    name: "prompt-system".into(),
                    kind: "metadata".into(),
                    package_kind: "foundation".into(),
                    runtime_provider: "prompt-system".into(),
                    current_source: "packages/weft-claw".into(),
                    trusted: true,
                    signature: "builtin:product-package".into(),
                    source_authority: "product-package-instance".into(),
                    source_public_keys: vec![],
                    provides: vec!["prompt.system".into()],
                    requires: vec![],
                }],
            }),
            ..test_state(repo_root, registry)
        };

        let generation = test_generation(
            "prompt.system",
            "prompt-system",
            "developer",
            GenerationStatus::Candidate,
        );

        let results = live_probe_generation(&state, &generation).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].check, "probe:prompt.system");
        assert!(!results[0].passed);
        assert!(results[0]
            .message
            .contains("without an assembled real package source"));
    }

    #[tokio::test]
    async fn verify_generation_rejects_safe_candidate_when_selected_package_digest_metadata_missing(
    ) {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'
profile = 'safe'

[[scenes.team.package_pins]]
package = 'agent-runtime'
source = 'remote://candidate-without-digest'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let candidate = test_generation(
            "agent.runtime",
            "agent-runtime",
            "safe",
            GenerationStatus::Candidate,
        );
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "agent.runtime".into(),
            CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
            registry,
            candidate,
        );

        let (status, body) =
            super::verify_generation(Path("weft-claw".into()), State(state.clone()))
                .await
                .expect_err("safe verification should fail");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.0["reason"], "generation_verify_failed");
        let validations = body.0["candidate"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "digest-policy:agent-runtime"
                && result["passed"] == false
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("missing usable sha512/artifact/content digest metadata")
        }));
    }

    #[tokio::test]
    async fn verify_generation_rejects_trusted_candidate_when_selected_package_digest_metadata_missing(
    ) {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'
profile = 'trusted'

[[scenes.team.package_pins]]
package = 'agent-runtime'
source = 'remote://candidate-without-digest'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let candidate = test_generation(
            "agent.runtime",
            "agent-runtime",
            "trusted",
            GenerationStatus::Candidate,
        );
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "agent.runtime".into(),
            CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
            registry,
            candidate,
        );

        let (status, body) =
            super::verify_generation(Path("weft-claw".into()), State(state.clone()))
                .await
                .expect_err("trusted verification should fail");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.0["reason"], "generation_verify_failed");
        let validations = body.0["candidate"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "digest-policy:agent-runtime"
                && result["passed"] == false
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("under profile 'trusted'")
        }));
    }

    #[tokio::test]
    async fn verify_generation_allows_developer_candidate_with_dev_unsealed_warning() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'
profile = 'developer'

[[scenes.team.package_pins]]
package = 'agent-runtime'
source = 'remote://candidate-without-digest'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let candidate = test_generation(
            "agent.runtime",
            "agent-runtime",
            "developer",
            GenerationStatus::Candidate,
        );
        let candidate = AppGeneration {
            lock_path: generation_lock_path(1),
            ..candidate
        };
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "agent.runtime".into(),
            CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
            registry,
            candidate,
        );

        let response = super::verify_generation(Path("weft-claw".into()), State(state.clone()))
            .await
            .expect("developer verification should succeed");
        assert_eq!(response.0["status"], "verified");
        assert_eq!(response.0["dev_unsealed"], true);
        let generation = &response.0["generation"];
        let validations = generation["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "digest-policy:agent-runtime"
                && result["passed"] == true
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("dev-unsealed")
        }));
        assert!(validations.iter().any(|result| {
            result["check"] == "digest-policy:dev-unsealed" && result["passed"] == true
        }));

        let activated = activate_generation(Path("weft-claw".into()), State(state.clone()))
            .await
            .expect("activation should succeed for verified developer candidate");
        assert_eq!(activated.0["status"], "activated");

        let lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("lock file written");
        assert!(lock.dev_unsealed);
        assert_eq!(lock.trust_level, "dev-unsealed");
    }

    #[tokio::test]
    async fn verify_generation_warns_when_store_metadata_is_absent_for_backward_compatibility() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            "[scenes.team]\nname = 'team'\nprofile = 'safe'\n",
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let app_config_path = instance_dir.join("config.toml");
        let state = state_for_propose_generation(
            repo_root.clone(),
            app.clone(),
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
        );
        let candidate = candidate_with_verified_identities(
            &state,
            &app_config_path,
            test_generation(
                "agent.runtime",
                "agent-runtime",
                "safe",
                GenerationStatus::Candidate,
            ),
        );
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
            registry_with_provider("agent.runtime", "agent-runtime"),
            candidate,
        );

        let response = super::verify_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("verification should succeed when store metadata is absent");
        let validations = response.0["generation"]["validation_results"]
            .as_array()
            .expect("validation array");

        assert!(validations.iter().any(|result| {
            result["check"] == "store-policy:agent-runtime"
                && result["passed"] == true
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("backward compatibility")
        }));
    }

    #[tokio::test]
    async fn verify_generation_passes_when_summary_identities_match_recomputed_values() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            "[scenes.team]\nname = 'team'\nprofile = 'developer'\n",
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let app_config_path = instance_dir.join("config.toml");
        let state = state_for_propose_generation(
            repo_root.clone(),
            app.clone(),
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
        );
        let candidate = candidate_with_verified_identities(
            &state,
            &app_config_path,
            test_generation(
                "agent.runtime",
                "agent-runtime",
                "developer",
                GenerationStatus::Candidate,
            ),
        );
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
            registry_with_provider("agent.runtime", "agent-runtime"),
            candidate,
        );

        let response = super::verify_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("verification should succeed");

        assert_eq!(response.0["status"], "verified");
        let validations = response.0["generation"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "identity-consistency:binding_set_id"
                && result["passed"] == true
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("matches the recomputed binding identity")
        }));
        assert!(validations.iter().any(|result| {
            result["check"] == "identity-consistency:closure_id"
                && result["passed"] == true
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("matches the recomputed closure identity")
        }));
    }

    #[tokio::test]
    async fn verify_generation_rejects_stale_binding_set_id_metadata() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            "[scenes.team]\nname = 'team'\nprofile = 'developer'\n",
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let app_config_path = instance_dir.join("config.toml");
        let proposal_state = state_for_propose_generation(
            repo_root.clone(),
            app.clone(),
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
        );
        let mut candidate = candidate_with_verified_identities(
            &proposal_state,
            &app_config_path,
            test_generation(
                "agent.runtime",
                "agent-runtime",
                "developer",
                GenerationStatus::Candidate,
            ),
        );
        candidate.binding_set_id = "binding-set:sha256:stale".into();
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
            registry_with_provider("agent.runtime", "agent-runtime"),
            candidate,
        );

        let (status, body) = super::verify_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("verification should fail for stale binding_set_id");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.0["reason"], "generation_verify_failed");
        let validations = body.0["candidate"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "identity-consistency:binding_set_id"
                && result["passed"] == false
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("does not match recomputed binding identity")
        }));
    }

    #[tokio::test]
    async fn verify_generation_rejects_stale_closure_id_metadata() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            "[scenes.team]\nname = 'team'\nprofile = 'developer'\n",
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let app_config_path = instance_dir.join("config.toml");
        let proposal_state = state_for_propose_generation(
            repo_root.clone(),
            app.clone(),
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
        );
        let mut candidate = candidate_with_verified_identities(
            &proposal_state,
            &app_config_path,
            test_generation(
                "agent.runtime",
                "agent-runtime",
                "developer",
                GenerationStatus::Candidate,
            ),
        );
        candidate.closure_id = "closure:sha256:stale".into();
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
            registry_with_provider("agent.runtime", "agent-runtime"),
            candidate,
        );

        let (status, body) = super::verify_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("verification should fail for stale closure_id");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.0["reason"], "generation_verify_failed");
        let validations = body.0["candidate"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "identity-consistency:closure_id"
                && result["passed"] == false
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("does not match recomputed closure identity")
        }));
    }

    #[tokio::test]
    async fn verify_generation_allows_empty_identity_metadata_with_warnings() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            "[scenes.team]\nname = 'team'\nprofile = 'developer'\n",
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let candidate = test_generation(
            "agent.runtime",
            "agent-runtime",
            "developer",
            GenerationStatus::Candidate,
        );
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![test_package_source(
                "agent-runtime",
                "provider",
                vec!["agent.runtime"],
                vec![],
            )],
            registry_with_provider("agent.runtime", "agent-runtime"),
            candidate,
        );

        let response = super::verify_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("verification should allow empty legacy identities");

        assert_eq!(response.0["status"], "verified");
        let validations = response.0["generation"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "identity-consistency:binding_set_id"
                && result["passed"] == true
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("missing binding_set_id metadata")
        }));
        assert!(validations.iter().any(|result| {
            result["check"] == "identity-consistency:closure_id"
                && result["passed"] == true
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("missing closure_id metadata")
        }));
    }

    #[tokio::test]
    async fn verify_generation_rejects_candidate_when_required_capability_binding_missing() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(&repo_root, None, "");

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let candidate = AppGeneration {
            bindings: vec![],
            capabilities: vec!["agent.runtime".into()],
            profile: "developer".into(),
            status: GenerationStatus::Candidate,
            ..test_generation(
                "agent.runtime",
                "agent-runtime",
                "developer",
                GenerationStatus::Candidate,
            )
        };
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "agent.runtime".into(),
            CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![],
                bindings: vec![],
            },
        );
        let state = state_for_verify_generation(repo_root, app, vec![], registry, candidate);

        let (status, body) = super::verify_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("verification should fail without required binding");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.0["reason"], "generation_verify_failed");
        let validations = body.0["candidate"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "binding-coverage:agent.runtime"
                && result["passed"] == false
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("has no provider binding")
        }));
    }

    #[tokio::test]
    async fn verify_generation_rejects_candidate_when_binding_provider_not_in_selected_package_summary(
    ) {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(
            &repo_root,
            Some("team"),
            r#"[scenes.team]
name = 'team'
profile = 'developer'
"#,
        );

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["agent.runtime".into()],
            bindings: vec![test_binding("agent.runtime", "agent-runtime")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let candidate = test_generation(
            "agent.runtime",
            "agent-runtime",
            "developer",
            GenerationStatus::Candidate,
        );
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "agent.runtime".into(),
            CapabilityRegistryEntry {
                capability: "agent.runtime".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: "agent-runtime".into(),
                    runtime: "metadata".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        let state = state_for_verify_generation(
            repo_root,
            app,
            vec![PackageSource {
                name: "prompt-system".into(),
                kind: "metadata".into(),
                package_kind: "provider".into(),
                runtime_provider: "prompt-system".into(),
                current_source: "packages/weft-claw".into(),
                trusted: true,
                signature: "builtin:test".into(),
                source_authority: "test".into(),
                source_public_keys: vec![],
                provides: vec!["agent.runtime".into()],
                requires: vec![],
            }],
            registry,
            candidate,
        );

        let (status, body) = super::verify_generation(Path("weft-claw".into()), State(state))
            .await
            .expect_err("verification should fail when binding provider is excluded");

        assert_eq!(status, HttpStatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.0["reason"], "generation_verify_failed");
        let validations = body.0["candidate"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "binding-coverage:agent.runtime"
                && result["passed"] == false
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("no included selected package summary entry matches")
        }));
    }

    #[tokio::test]
    async fn verify_generation_allows_core_provider_binding_for_coverage() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let (_, instance_dir, manifest_path) = write_test_manifest_and_config(&repo_root, None, "");

        let app = ResolvedApp {
            name: "weft-claw".into(),
            version: "0.1.0".into(),
            capabilities: vec!["core.execution".into()],
            bindings: vec![test_binding("core.execution", "core")],
            sources: ResolvedAppSources {
                manifest_path: manifest_path.display().to_string(),
                config_path: Some(instance_dir.join("config.toml").display().to_string()),
                lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
            },
            ..ResolvedApp::default()
        };
        let candidate = AppGeneration {
            bindings: vec![test_binding("core.execution", "core")],
            capabilities: vec!["core.execution".into()],
            profile: "developer".into(),
            status: GenerationStatus::Candidate,
            ..test_generation(
                "core.execution",
                "core",
                "developer",
                GenerationStatus::Candidate,
            )
        };
        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "core.execution".into(),
            CapabilityRegistryEntry {
                capability: "core.execution".into(),
                providers: vec![CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "native".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );
        let state = state_for_verify_generation(repo_root, app, vec![], registry, candidate);

        let response = super::verify_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("core provider binding should pass coverage verification");

        assert_eq!(response.0["status"], "verified");
        let validations = response.0["generation"]["validation_results"]
            .as_array()
            .expect("validation result array");
        assert!(validations.iter().any(|result| {
            result["check"] == "binding-coverage:core.execution"
                && result["passed"] == true
                && result["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("bound to core")
        }));
    }

    #[tokio::test]
    async fn activate_generation_sets_dev_unsealed_from_structured_validation_check() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_validation_results(
                    2,
                    GenerationStatus::Verified,
                    "developer",
                    vec![crate::app::ValidationResult {
                        check: "digest-policy:dev-unsealed".into(),
                        passed: true,
                        message: "Warning: candidate verification succeeded with developer-only dev-unsealed digest policy exceptions".into(),
                    }],
                )),
                rollback: None,
                next_id: 3,
            },
        );

        let response = activate_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("activation succeeds");

        assert_eq!(response.0["status"], "activated");
        let lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("lock file written");
        assert!(lock.dev_unsealed);
        assert_eq!(lock.trust_level, "dev-unsealed");
    }

    #[tokio::test]
    async fn activate_generation_rejects_failed_candidate_before_writes() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_id(2, GenerationStatus::Failed)),
                rollback: None,
                next_id: 3,
            },
        );

        let (status, body) = activate_generation(Path("weft-claw".into()), State(state.clone()))
            .await
            .expect_err("failed candidate activation should be rejected");

        assert_eq!(status, HttpStatusCode::CONFLICT);
        let payload = body.0;
        assert_eq!(
            payload["error"],
            "Candidate must be verified before activation"
        );
        assert!(payload["reason"].is_null());
        assert!(payload["status"].is_null());
        assert!(payload["lock_written"].is_null());
        assert!(payload["pointer_written"].is_null());
        assert!(payload["pointer_error"].is_null());
        assert!(payload["index_written"].is_null());
        assert!(payload["index_error"].is_null());

        let store = state.generation_store.read().await;
        let app_store = store.get("weft-claw").expect("store present");
        assert_eq!(
            app_store.active.as_ref().map(|generation| generation.id),
            Some(1)
        );
        assert_eq!(
            app_store
                .candidate
                .as_ref()
                .map(|generation| generation.status),
            Some(GenerationStatus::Failed)
        );
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            None
        );
        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            None
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
        let lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("lock remains readable");
        assert_eq!(lock.generation, 0);
        assert_eq!(lock.status, "");
    }

    #[tokio::test]
    async fn activate_generation_rejects_dev_unsealed_marker_under_safe_profile_before_writes() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_validation_results(
                    2,
                    GenerationStatus::Verified,
                    "safe",
                    vec![crate::app::ValidationResult {
                        check: "digest-policy:dev-unsealed".into(),
                        passed: true,
                        message: "Warning: candidate verification succeeded with developer-only dev-unsealed digest policy exceptions".into(),
                    }],
                )),
                rollback: None,
                next_id: 3,
            },
        );

        let (status, body) = activate_generation(Path("weft-claw".into()), State(state.clone()))
            .await
            .expect_err("safe-profile dev-unsealed activation should be rejected");

        assert_eq!(status, HttpStatusCode::CONFLICT);
        let payload = body.0;
        assert_eq!(payload["reason"], "dev_unsealed_profile_mismatch");
        assert_eq!(payload["status"], "activation_failed");
        assert_eq!(payload["lock_written"], false);
        assert_eq!(payload["pointer_written"], false);
        assert!(payload["pointer_error"].is_null());
        assert_eq!(payload["index_written"], false);
        assert!(payload["index_error"].is_null());
        assert!(payload["error"]
            .as_str()
            .expect("error string")
            .contains("developer-only dev-unsealed validation"));

        let store = state.generation_store.read().await;
        let app_store = store.get("weft-claw").expect("store present");
        assert_eq!(
            app_store.active.as_ref().map(|generation| generation.id),
            Some(1)
        );
        assert_eq!(
            app_store.candidate.as_ref().map(|generation| generation.id),
            Some(2)
        );
        assert_eq!(
            crate::app::read_active_generation_pointer(&instance_dir)
                .expect("active pointer read succeeds"),
            None
        );
        assert_eq!(
            crate::app::read_previous_generation_pointer(&instance_dir)
                .expect("previous pointer read succeeds"),
            None
        );
        assert!(crate::app::load_generation_index(&instance_dir)
            .expect("index load succeeds")
            .is_none());
        let lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("lock remains readable");
        assert_eq!(lock.generation, 0);
        assert_eq!(lock.status, "");
    }

    #[tokio::test]
    async fn activate_generation_does_not_set_dev_unsealed_without_structured_validation_check() {
        let root = tempdir().expect("temp dir");
        let repo_root = root.path().to_path_buf();
        let instance_dir = repo_root.join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");
        fs::write(instance_dir.join("lock.toml"), "app = \"weft-claw\"\n")
            .expect("lock file seeded");

        let state = state_with_generation_store(
            repo_root,
            test_resolved_app("weft-claw", Some(&instance_dir)),
            AppGenerationStore {
                active: Some(generation_with_id(1, GenerationStatus::Active)),
                candidate: Some(generation_with_validation_results(
                    2,
                    GenerationStatus::Verified,
                    "developer",
                    vec![crate::app::ValidationResult {
                        check: "digest-policy:agent-runtime".into(),
                        passed: true,
                        message: "Warning: package 'agent-runtime' is missing usable digest metadata for source 'packages/official/agent-runtime' under profile 'developer'; candidate remains verified as dev-unsealed".into(),
                    }],
                )),
                rollback: None,
                next_id: 3,
            },
        );

        let response = activate_generation(Path("weft-claw".into()), State(state))
            .await
            .expect("activation succeeds");

        assert_eq!(response.0["status"], "activated");
        let lock = crate::app::load_instance_lock_from_path(&instance_dir.join("lock.toml"))
            .expect("lock file written");
        assert!(!lock.dev_unsealed);
        assert_eq!(lock.trust_level, "");
    }

    #[test]
    fn selected_packages_skip_metadata_only_weft_claw_required_sources() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let state = AppState {
            package_index: Arc::new(PackageIndex {
                version: 1,
                revision: "test-rev".into(),
                source_url: "local://packages".into(),
                package_sources: vec![
                    PackageSource {
                        name: "prompt-system".into(),
                        kind: "metadata".into(),
                        package_kind: "foundation".into(),
                        runtime_provider: "agent-core".into(),
                        current_source: "packages/weft-claw".into(),
                        trusted: true,
                        signature: "builtin:product-package".into(),
                        source_authority: String::new(),
                        source_public_keys: vec![],
                        provides: vec!["prompt.system".into()],
                        requires: vec![],
                    },
                    PackageSource {
                        name: "agent-runtime".into(),
                        kind: "embedded".into(),
                        package_kind: "feature".into(),
                        runtime_provider: "agent-runtime".into(),
                        current_source: "packages/official/agent-core".into(),
                        trusted: true,
                        signature: "builtin:official".into(),
                        source_authority: "official".into(),
                        source_public_keys: vec![],
                        provides: vec![
                            "agent.runtime".into(),
                            "prompt.system".into(),
                            "memory.store".into(),
                        ],
                        requires: vec![
                            "agent-runtime".into(),
                            "prompt-system".into(),
                            "memory-store".into(),
                        ],
                    },
                    PackageSource {
                        name: "memory-store".into(),
                        kind: "wasm".into(),
                        package_kind: "provider".into(),
                        runtime_provider: "memory-store".into(),
                        current_source: "packages/official/memory".into(),
                        trusted: true,
                        signature: "builtin:official".into(),
                        source_authority: "official".into(),
                        source_public_keys: vec![],
                        provides: vec!["memory.store".into()],
                        requires: vec![],
                    },
                ],
            }),
            ..test_state(repo_root, CapabilityRegistry::new())
        };

        let packages = packages_for_selected_names(
            &state,
            &[
                "prompt-system".to_string(),
                "agent-runtime".to_string(),
                "memory-store".to_string(),
            ],
        );

        assert!(packages.iter().all(|pkg| pkg.name != "prompt-system"));
        assert!(packages.iter().any(|pkg| pkg.name == "agent-runtime"));
        assert!(packages.iter().any(|pkg| pkg.name == "memory-store"));
    }
}
