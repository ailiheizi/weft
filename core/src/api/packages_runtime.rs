use crate::api::openai_compat::AppState;
use crate::package::bridge::PackageLoadInfo;
use crate::package::config::{load_manifest, PackagePermissions};
use crate::package::permissions::escalated_permissions;
use crate::package::{
    discover_runtime_package, installed_packages_dir, merged_package_aliases,
    resolve_runtime_package, resolve_wasm_startup_mode, PackageInfo,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use base64::Engine;
use sha2::{Digest, Sha512};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
pub struct InstallPackageRequest {
    name: String,
    #[serde(default)]
    version: String,
    store_base_url: String,
    /// B4: must be explicitly set true to install an upgrade that requests more
    /// permissions than the currently-installed version.
    #[serde(default)]
    approve_escalation: bool,
}

#[derive(Debug, serde::Deserialize)]
struct StoreEnvelope<T> {
    success: bool,
    data: T,
}

#[derive(Debug, serde::Deserialize)]
struct StorePackageDetail {
    latest: Option<StoreVersionDetail>,
    #[serde(default)]
    versions: Vec<StoreVersionDetail>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct StoreVersionDetail {
    version: String,
    #[serde(default)]
    manifest: serde_json::Value,
    #[serde(default)]
    artifact_digest: String,
}

fn managed_packages_dir(state: &AppState) -> PathBuf {
    installed_packages_dir(&state.repo_root)
}

/// Resolve a package's on-disk directory by runtime name, honoring index.toml
/// `current_source` (so packages whose source lives under `official/` are found,
/// not just `installed/`). Falls back to `installed/<name>` for compatibility.
fn resolve_managed_package_dir(state: &AppState, name: &str) -> PathBuf {
    resolve_runtime_package(&state.repo_root, &state.package_index, name)
        .map(|pkg| pkg.dir)
        .unwrap_or_else(|| managed_packages_dir(state).join(name))
}

/// Parses just the `[permissions]` section from raw manifest bytes. Tolerant of
/// the multiple package.toml dialects (other sections are ignored), so it works
/// whether the manifest uses `[package_info]` or `[identity]+[package]`.
fn parse_manifest_permissions(manifest_bytes: &[u8]) -> PackagePermissions {
    #[derive(serde::Deserialize, Default)]
    struct PermsOnly {
        #[serde(default)]
        permissions: PackagePermissions,
    }
    std::str::from_utf8(manifest_bytes)
        .ok()
        .and_then(|s| toml::from_str::<PermsOnly>(s).ok())
        .map(|p| p.permissions)
        .unwrap_or_default()
}

fn json_error(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": message.into() })))
}

fn sha512_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn decode_manifest_bytes(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    match value {
        serde_json::Value::String(text) => base64::engine::general_purpose::STANDARD
            .decode(text)
            .or_else(|_| Ok(text.as_bytes().to_vec()))
            .map_err(|error: base64::DecodeError| format!("Failed to decode manifest: {error}")),
        serde_json::Value::Array(items) => {
            let mut bytes = Vec::with_capacity(items.len());
            for item in items {
                let Some(value) = item.as_u64() else {
                    return Err("Manifest byte array contains non-numeric values".into());
                };
                let Ok(byte) = u8::try_from(value) else {
                    return Err("Manifest byte array contains values outside u8 range".into());
                };
                bytes.push(byte);
            }
            Ok(bytes)
        }
        serde_json::Value::Object(_) => serde_json::to_vec_pretty(value)
            .map_err(|error| format!("Failed to serialize manifest object: {error}")),
        serde_json::Value::Null => Err("Manifest payload is missing".into()),
        _ => Err("Manifest payload has unsupported format".into()),
    }
}

fn select_store_version<'a>(
    detail: &'a StorePackageDetail,
    requested_version: &str,
) -> Option<&'a StoreVersionDetail> {
    if requested_version.is_empty() {
        return detail.latest.as_ref().or_else(|| detail.versions.first());
    }

    detail
        .latest
        .as_ref()
        .filter(|latest| latest.version == requested_version)
        .or_else(|| {
            detail
                .versions
                .iter()
                .find(|version| version.version == requested_version)
        })
}

fn trim_store_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

async fn runtime_package_aliases_for_host(
    state: &AppState,
) -> std::collections::HashMap<String, String> {
    let package_aliases = {
        let config = state.config.read().await;
        config.package_aliases.clone()
    };
    merged_package_aliases(&state.package_index, &package_aliases)
}

fn runtime_package_or_404(
    state: &AppState,
    name: &str,
) -> Result<crate::package::DiscoveredPackage, (StatusCode, Json<serde_json::Value>)> {
    resolve_runtime_package(&state.repo_root, &state.package_index, name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Package '{}' not found in runtime sources", name)
            })),
        )
    })
}

pub async fn list_packages(State(state): State<AppState>) -> Json<serde_json::Value> {
    let pm = state.package_manager.read().await;
    let packages: Vec<serde_json::Value> = pm
        .list()
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "version": p.version,
                "overrides": p.overrides,
                "enabled": p.enabled,
                "has_ui": p.has_ui,
                "description": p.description,
            })
        })
        .collect();
    Json(serde_json::json!({ "packages": packages }))
}

/// POST /api/packages/install — install a package from a remote store.
pub async fn install_package(
    State(state): State<AppState>,
    Json(body): Json<InstallPackageRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let name = body.name.trim();
    let requested_version = body.version.trim();
    let store_base_url = trim_store_base_url(&body.store_base_url);

    if name.is_empty() {
        return Err(json_error(StatusCode::BAD_REQUEST, "Package name is required"));
    }
    if store_base_url.is_empty() {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "store_base_url is required",
        ));
    }

    let detail_url = format!("{store_base_url}/api/packages/{name}");
    let download_url = format!("{store_base_url}/api/packages/{name}/download");
    let client = reqwest::Client::new();

    let detail_response = client
        .get(&detail_url)
        .query(&[("version", requested_version)])
        .send()
        .await
        .map_err(|error| {
            json_error(
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch package detail: {error}"),
            )
        })?
        .error_for_status()
        .map_err(|error| {
            json_error(
                StatusCode::BAD_GATEWAY,
                format!("Store detail request failed: {error}"),
            )
        })?;

    let envelope: StoreEnvelope<StorePackageDetail> = detail_response.json().await.map_err(|error| {
        json_error(
            StatusCode::BAD_GATEWAY,
            format!("Failed to decode store detail response: {error}"),
        )
    })?;
    if !envelope.success {
        return Err(json_error(
            StatusCode::BAD_GATEWAY,
            "Store detail response indicated failure",
        ));
    }

    let selected = select_store_version(&envelope.data, requested_version).ok_or_else(|| {
        json_error(
            StatusCode::NOT_FOUND,
            if requested_version.is_empty() {
                format!("No published version found for package '{name}'")
            } else {
                format!("Package '{name}' version '{requested_version}' not found")
            },
        )
    })?;

    let expected_digest = selected.artifact_digest.trim();
    if expected_digest.is_empty() {
        return Err(json_error(
            StatusCode::BAD_GATEWAY,
            format!("Store detail for package '{name}' is missing artifact_digest"),
        ));
    }

    let manifest_bytes = decode_manifest_bytes(&selected.manifest)
        .map_err(|error| json_error(StatusCode::BAD_GATEWAY, error))?;

    let download_response = client
        .get(&download_url)
        .query(&[("version", selected.version.as_str())])
        .send()
        .await
        .map_err(|error| {
            json_error(
                StatusCode::BAD_GATEWAY,
                format!("Failed to download package artifact: {error}"),
            )
        })?
        .error_for_status()
        .map_err(|error| {
            json_error(
                StatusCode::BAD_GATEWAY,
                format!("Store artifact request failed: {error}"),
            )
        })?;

    let header_digest = download_response
        .headers()
        .get("X-Artifact-Digest")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let expected_digest = header_digest.as_deref().unwrap_or(expected_digest);

    let artifact_bytes = download_response.bytes().await.map_err(|error| {
        json_error(
            StatusCode::BAD_GATEWAY,
            format!("Failed to read downloaded package artifact: {error}"),
        )
    })?;

    let actual_digest = sha512_hex(artifact_bytes.as_ref());
    if !actual_digest.eq_ignore_ascii_case(expected_digest) {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            format!(
                "Artifact digest mismatch for package '{name}': expected {expected_digest}, got {actual_digest}"
            ),
        ));
    }

    let package_dir = installed_packages_dir(&state.repo_root).join(name);

    // B4: detect permission escalation before overwriting the installed package.
    // Old permissions come from the currently-installed manifest (if any); new
    // permissions are parsed from the manifest about to be written.
    let old_permissions = if package_dir.join("package.toml").exists() {
        load_manifest(&package_dir)
            .map(|m| m.permissions)
            .unwrap_or_default()
    } else {
        PackagePermissions::default()
    };
    let new_permissions = parse_manifest_permissions(&manifest_bytes);
    let escalated = escalated_permissions(&old_permissions, &new_permissions);
    if !escalated.is_empty() && !body.approve_escalation {
        let names: Vec<String> = escalated.iter().map(|p| p.to_string()).collect();
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "Upgrade of '{name}' requests new permissions not held by the \
                     installed version: [{}]. Re-install with approve_escalation=true \
                     to grant them.",
                    names.join(", ")
                ),
                "escalation": names,
                "requires_approval": true,
            })),
        ));
    }

    fs::create_dir_all(&package_dir).map_err(|error| {
        json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create package directory: {error}"),
        )
    })?;
    fs::write(package_dir.join("package.wasm"), artifact_bytes.as_ref()).map_err(|error| {
        json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write package.wasm: {error}"),
        )
    })?;
    fs::write(package_dir.join("package.toml"), &manifest_bytes).map_err(|error| {
        json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write package.toml: {error}"),
        )
    })?;

    // TODO: enforce signature verification once platform key infrastructure is available.
    let discovered = discover_runtime_package(&state.repo_root, &state.package_index, name)
        .ok_or_else(|| {
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Installed package '{name}' could not be rediscovered after writing files"
                ),
            )
        })?;

    let mut pm = state.package_manager.write().await;
    pm.register(PackageInfo {
        name: discovered.manifest.package_info.name.clone(),
        version: Some(discovered.manifest.package_info.version.clone()),
        overrides: vec![],
        enabled: true,
        has_ui: false,
        description: Some(discovered.manifest.package_info.description.clone()),
    });

    Ok(Json(serde_json::json!({
        "installed": discovered.manifest.package_info.name,
        "version": discovered.manifest.package_info.version,
        "digest_verified": true,
    })))
}

/// POST /api/packages/{name}/reload — reload a package's WASM without restarting
pub async fn reload_package(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let package = runtime_package_or_404(&state, &name)?;
    let package_dir = package.dir.clone();
    let manifest = package.manifest.clone();
    let runtime_package_name = manifest.package_info.name.clone();
    let wasm_path = package.entry_path.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Package '{}' has no runtime entry to reload", name)
            })),
        )
    })?;

    let load_info = PackageLoadInfo {
        name: runtime_package_name.clone(),
        dir: package_dir.clone(),
        wasm_path,
        startup_mode: resolve_wasm_startup_mode(&manifest),
        permissions: manifest.permissions.clone(),
    };

    // Reload WASM
    let mut wh = state.wasm_handle.write().await;
    if let Some(ref handle) = *wh {
        handle.reload_package(&load_info).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Reload failed: {}", e)})),
            )
        })?;
    } else {
        let package_aliases = runtime_package_aliases_for_host(&state).await;
        // No WasmHandle yet — create one
        let host_state = crate::package::bridge::WasmHostState {
            config: state.config.clone(),
            pipeline: state.pipeline.clone(),
            runtime_handle: tokio::runtime::Handle::current(),
            process_manager: state.process_manager.clone(),
            vkey_store: state.vkey_store.clone(),
            kv_store: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            caller_package_name: String::new(),
            package_dir: String::new(),
            permissions: Default::default(),
            package_map: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            package_aliases: std::sync::Arc::new(std::sync::Mutex::new(package_aliases)),
            call_depth: std::sync::Arc::new(std::sync::Mutex::new(0)),
            app_state: std::sync::Arc::new(std::sync::Mutex::new(Some(state.clone()))),
        };
        let host = crate::package::bridge::WasmPackageHost::new(&[load_info], host_state);
        *wh = Some(crate::package::bridge::WasmHandle::new(host));
    }

    // Update PackageManager metadata
    let mut pm = state.package_manager.write().await;
    pm.register(PackageInfo {
        name: runtime_package_name,
        version: Some(manifest.package_info.version.clone()),
        overrides: vec![],
        enabled: true,
        has_ui: false,
        description: Some(manifest.package_info.description.clone()),
    });

    Ok(Json(
        serde_json::json!({"status": "ok", "message": format!("Package '{}' reloaded", name)}),
    ))
}

/// POST /api/packages/{name}/toggle — enable/disable a package
pub async fn toggle_package(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let enabled = body["enabled"].as_bool().unwrap_or(true);

    let runtime_package_name =
        resolve_runtime_package(&state.repo_root, &state.package_index, &name)
            .map(|package| package.manifest.package_info.name)
            .unwrap_or_else(|| name.clone());

    let mut pm = state.package_manager.write().await;
    let info = pm.get_mut(&runtime_package_name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Package '{}' not found", name)})),
        )
    })?;
    info.enabled = enabled;

    // If disabling, unload WASM; if enabling, reload WASM
    let wh = state.wasm_handle.read().await;
    if let Some(ref handle) = *wh {
        if !enabled {
            let _ = handle.unload_package(&runtime_package_name);
        } else {
            // Need to drop read lock and acquire write lock for reload
            drop(wh);
            if let Ok(package) = runtime_package_or_404(&state, &name) {
                if let Some(wasm_path) = package.entry_path.clone() {
                    let load_info = PackageLoadInfo {
                        name: package.manifest.package_info.name.clone(),
                        dir: package.dir.clone(),
                        wasm_path,
                        startup_mode: resolve_wasm_startup_mode(&package.manifest),
                        permissions: package.manifest.permissions.clone(),
                    };
                    let wh = state.wasm_handle.read().await;
                    if let Some(ref handle) = *wh {
                        let _ = handle.load_package(&load_info);
                    }
                }
            }
        }
    }

    Ok(Json(
        serde_json::json!({"status": "ok", "name": name, "runtime_package": runtime_package_name, "enabled": enabled}),
    ))
}

/// DELETE /api/packages/{name} — uninstall a package (unload WASM + delete files)
pub async fn uninstall_package(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let packages_dir = managed_packages_dir(&state);

    // Check if other packages depend on this one
    let dependents = crate::package::check_uninstall_safety(&name, &packages_dir);
    if !dependents.is_empty() {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("Cannot uninstall '{}': depended on by {:?}", name, dependents),
                "dependents": dependents,
            })),
        ));
    }

    // Unload WASM first
    {
        let wh = state.wasm_handle.read().await;
        if let Some(ref handle) = *wh {
            let _ = handle.unload_package(&name);
        }
    }

    // Remove from PackageManager
    {
        let mut pm = state.package_manager.write().await;
        pm.unregister(&name);
    }

    // Delete package directory
    let package_dir = packages_dir.join(&name);
    if package_dir.exists() {
        tokio::fs::remove_dir_all(&package_dir).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::json!({"error": format!("Failed to delete package files: {}", e)}),
                ),
            )
        })?;
    }

    Ok(Json(
        serde_json::json!({"status": "ok", "message": format!("Package '{}' uninstalled", name)}),
    ))
}

/// GET /api/packages/{name}/dependencies — view package dependency info
pub async fn get_package_dependencies(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let packages_dir = managed_packages_dir(&state);
    let package_dir = packages_dir.join(&name);
    let manifest = crate::package::config::load_manifest(&package_dir).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Package not found: {}", e)})),
        )
    })?;

    let pm = state.package_manager.read().await;
    let installed: Vec<_> = pm.list().into_iter().cloned().collect();
    let missing = crate::package::check_dependencies(&manifest, &installed);
    let dependents = crate::package::check_uninstall_safety(&name, &packages_dir);

    Ok(Json(serde_json::json!({
        "name": name,
        "dependencies": manifest.dependencies,
        "missing": missing,
        "depended_by": dependents,
    })))
}

/// GET /api/packages/{name}/config/schema — get package config schema
pub async fn get_package_config_schema(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let package_dir = resolve_managed_package_dir(&state, &name);
    let manifest = crate::package::config::load_manifest(&package_dir).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Package not found: {}", e)})),
        )
    })?;

    Ok(Json(serde_json::json!({
        "name": name,
        "schema": manifest.config,
    })))
}

/// GET /api/packages/{name}/config — get current config values
pub async fn get_package_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let package_dir = resolve_managed_package_dir(&state, &name);
    let config_path = package_dir.join("config.json");

    let config: serde_json::Value = if config_path.exists() {
        let content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to read config: {}", e)})),
            )
        })?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        serde_json::json!({})
    };

    // Load schema to mask secret fields
    let manifest = crate::package::config::load_manifest(&package_dir).ok();

    let masked = if let Some(manifest) = manifest {
        mask_secret_fields(&config, &manifest.config)
    } else {
        config
    };

    Ok(Json(serde_json::json!({
        "name": name,
        "config": masked,
    })))
}

/// PUT /api/packages/{name}/config — save config values
pub async fn save_package_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let package_dir = resolve_managed_package_dir(&state, &name);
    let config_path = package_dir.join("config.json");

    // Validate package exists
    if !package_dir.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Package '{}' not found", name)})),
        ));
    }

    // Write config
    let content = serde_json::to_string_pretty(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid JSON: {}", e)})),
        )
    })?;
    tokio::fs::write(&config_path, &content)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to write config: {}", e)})),
            )
        })?;

    // Notify package of config change (best-effort)
    {
        let wh = state.wasm_handle.read().await;
        if let Some(ref handle) = *wh {
            if handle.has_package(&name) {
                let _ = handle.call(&name, "on_config_change", &content);
            }
        }
    }

    Ok(Json(
        serde_json::json!({"status": "ok", "message": format!("Config saved for '{}'", name)}),
    ))
}

/// Mask secret fields in config values for GET responses.
fn mask_secret_fields(
    config: &serde_json::Value,
    schema: &std::collections::HashMap<String, crate::package::config::PackageConfigField>,
) -> serde_json::Value {
    let Some(obj) = config.as_object() else {
        return config.clone();
    };

    let mut masked = obj.clone();
    for (key, field) in schema {
        if field.secret {
            if let Some(serde_json::Value::String(val)) = masked.get(key) {
                if val.len() > 4 {
                    let suffix = &val[val.len() - 4..];
                    masked.insert(key.clone(), serde_json::json!(format!("****{}", suffix)));
                } else {
                    masked.insert(key.clone(), serde_json::json!("****"));
                }
            }
        }
    }
    serde_json::Value::Object(masked)
}

#[cfg(test)]
mod b4_tests {
    use super::parse_manifest_permissions;
    use crate::package::permissions::{escalated_permissions, Permission};

    #[test]
    fn parses_permissions_from_package_info_dialect() {
        let toml = br#"
[package_info]
name = "x"
version = "1.0"
description = "d"
entry = "package.wasm"

[permissions]
storage = true
network = true
"#;
        let p = parse_manifest_permissions(toml);
        assert!(p.storage && p.network && !p.process);
    }

    #[test]
    fn parses_permissions_from_identity_dialect() {
        let toml = br#"
[identity]
name = "x"
version = "1.0"

[package]
runtime = "wasm"

[permissions]
process = true
"#;
        let p = parse_manifest_permissions(toml);
        assert!(p.process && !p.storage);
    }

    #[test]
    fn missing_permissions_section_yields_no_grants() {
        let toml = br#"
[package_info]
name = "x"
version = "1.0"
description = "d"
"#;
        let p = parse_manifest_permissions(toml);
        assert!(escalated_permissions(&Default::default(), &p).is_empty());
    }

    #[test]
    fn upgrade_adding_process_is_flagged_as_escalation() {
        let old = parse_manifest_permissions(
            b"[package_info]\nname='x'\nversion='1'\ndescription='d'\n[permissions]\nstorage=true\n",
        );
        let new = parse_manifest_permissions(
            b"[package_info]\nname='x'\nversion='2'\ndescription='d'\n[permissions]\nstorage=true\nprocess=true\n",
        );
        let esc = escalated_permissions(&old, &new);
        assert_eq!(esc, vec![Permission::Process]);
    }
}
