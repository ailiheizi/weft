use crate::api::openai_compat::{AppState, ChatProviderInfo};
use crate::package::config::load_manifest;
use crate::package::{
    build_service_config, native_library_candidates, DiscoveredPackage, PackageInfo, PackageRuntime,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, serde::Deserialize)]
pub struct ActivationPlanRequest {
    #[serde(default)]
    materialized_path: Option<String>,
    #[serde(default)]
    package_path: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    apply: bool,
    #[serde(default)]
    confirm: bool,
    #[serde(default)]
    start_service: bool,
}

fn requested_package_path(request: &ActivationPlanRequest) -> Option<&str> {
    request
        .materialized_path
        .as_deref()
        .or(request.package_path.as_deref())
        .or(request.path.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn has_parent_dir_component(path: &std::path::Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn resolve_package_path(repo_root: &std::path::Path, requested: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(requested);
    if has_parent_dir_component(&path) {
        return Err("package path must not contain parent-directory components".to_string());
    }

    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(repo_root.join(path))
    }
}

fn normalize_path_for_guard(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    })
}

fn materialized_managed_root(repo_root: &Path) -> PathBuf {
    repo_root.join(".weft").join("materialized")
}

fn is_under_explicit_temp_or_managed_root(repo_root: &Path, package_path: &Path) -> bool {
    let package_path = normalize_path_for_guard(package_path);
    let temp_root = normalize_path_for_guard(&std::env::temp_dir());
    let managed_root = normalize_path_for_guard(&materialized_managed_root(repo_root));

    package_path.starts_with(&temp_root) || package_path.starts_with(&managed_root)
}

fn blocked_response(
    package_path: &Path,
    manifest_found: bool,
    checks: Vec<serde_json::Value>,
    validation_issues: Vec<String>,
) -> serde_json::Value {
    serde_json::json!({
        "status": "activation_plan_blocked",
        "plan_only": true,
        "metadata_only": true,
        "activation_performed": false,
        "mutation_performed": false,
        "lock_mutation_performed": false,
        "package_path": package_path.display().to_string(),
        "manifest_found": manifest_found,
        "activation_required": false,
        "ready_for_activation": false,
        "checks": checks,
        "validation_issues": validation_issues,
    })
}

/// POST /api/plans/activate — inspect a materialized local package, or explicitly apply a controlled metadata activation.
pub async fn activation_plan(
    State(state): State<AppState>,
    Json(request): Json<ActivationPlanRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let requested_path = requested_package_path(&request).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "missing materialized_path/package_path/path",
                "activation_performed": false,
                "mutation_performed": false,
            })),
        )
    })?;

    let package_path = resolve_package_path(&state.repo_root, requested_path).map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": error,
                "activation_performed": false,
                "mutation_performed": false,
            })),
        )
    })?;

    let manifest_path = package_path.join("package.toml");
    let path_exists = package_path.is_dir();
    let manifest_found = manifest_path.is_file();

    let mut checks = vec![
        serde_json::json!({
            "name": "package_directory_exists",
            "ok": path_exists,
            "path": package_path.display().to_string(),
        }),
        serde_json::json!({
            "name": "plugin_manifest_found",
            "ok": manifest_found,
            "path": manifest_path.display().to_string(),
        }),
    ];

    if !path_exists || !manifest_found {
        return Ok(Json(blocked_response(
            &package_path,
            manifest_found,
            checks,
            vec!["materialized package directory and package.toml are required before activation can be planned".to_string()],
        )));
    }

    let manifest = load_manifest(&package_path).map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("failed to parse package.toml: {error}"),
                "activation_performed": false,
                "mutation_performed": false,
            })),
        )
    })?;

    let runtime = manifest.runtime_kind();
    let package_runtime = PackageRuntime::from_manifest(&manifest);
    let entry = manifest.resolved_entry();
    let entry_path = entry.as_ref().map(|entry| package_path.join(entry));
    let requires_entry = matches!(runtime.as_str(), "wasm" | "service" | "embedded");
    let entry_exists = entry_path
        .as_ref()
        .map(|path| path.is_file())
        .unwrap_or(false);
    let dependencies = manifest.dependencies.clone();
    let installed_packages = {
        let pm = state.package_manager.read().await;
        pm.list().into_iter().cloned().collect::<Vec<_>>()
    };
    let missing_dependencies = crate::package::check_dependencies(&manifest, &installed_packages);
    let dependency_check_ok = missing_dependencies.is_empty();
    let entry_check_ok = !requires_entry || entry_exists;
    let manifest_identity_ok = !manifest.package_info.name.trim().is_empty();
    let native_runtime_blocked = package_runtime == PackageRuntime::Native;
    let native_allowed = manifest
        .package
        .as_ref()
        .map(|package| package.native_allowed)
        .unwrap_or(false);
    let expected_digest = manifest
        .package
        .as_ref()
        .and_then(|package| package.expected_digest.as_deref())
        .map(str::trim)
        .filter(|digest| !digest.is_empty());
    let native_library_candidate = if package_runtime == PackageRuntime::Native {
        entry_path.as_ref().and_then(|entry_path| {
            native_library_candidates(entry_path)
                .into_iter()
                .find(|path| path.is_file())
        })
    } else {
        None
    };
    let root_check_ok =
        !request.apply || is_under_explicit_temp_or_managed_root(&state.repo_root, &package_path);
    let profile = *state.active_profile.read().await;
    let profile_allowed_for_apply = matches!(
        profile,
        crate::app::AppProfile::Developer | crate::app::AppProfile::Trusted
    );
    let profile_check_ok = !request.apply || profile_allowed_for_apply;
    let trusted_native_policy = state.core_policy.check("core.native_execution", profile);
    let trusted_native_profile_ok = profile == crate::app::AppProfile::Trusted;
    let confirmation_check_ok = !request.apply || request.confirm;
    let trusted_native_confirmation_ok = request.confirm;
    let expected_digest_present = expected_digest.is_some();
    let native_library_candidate_exists = native_library_candidate.is_some();
    let ready_for_trusted_native_load = native_runtime_blocked
        && manifest_identity_ok
        && dependency_check_ok
        && trusted_native_profile_ok
        && trusted_native_policy.allowed
        && trusted_native_confirmation_ok
        && native_allowed
        && expected_digest_present
        && native_library_candidate_exists;
    let ready_for_activation = manifest_identity_ok && entry_check_ok && dependency_check_ok;
    let apply_allowed = ready_for_activation
        && !native_runtime_blocked
        && root_check_ok
        && profile_check_ok
        && confirmation_check_ok;
    let runtime_safe_for_controlled_activation_ok = if request.apply {
        !native_runtime_blocked
    } else {
        !native_runtime_blocked || ready_for_trusted_native_load
    };

    checks.push(serde_json::json!({
        "name": "manifest_identity_present",
        "ok": manifest_identity_ok,
        "package_name": manifest.package_info.name,
        "version": manifest.package_info.version,
    }));
    checks.push(serde_json::json!({
        "name": "runtime_entry_available",
        "ok": entry_check_ok,
        "required": requires_entry,
        "entry": entry,
        "path": entry_path.as_ref().map(|path| path.display().to_string()),
        "exists": entry_exists,
    }));
    checks.push(serde_json::json!({
        "name": "dependencies_satisfied",
        "ok": dependency_check_ok,
        "missing": missing_dependencies,
    }));
    checks.push(serde_json::json!({
        "name": "runtime_safe_for_controlled_activation",
        "ok": runtime_safe_for_controlled_activation_ok,
        "runtime": runtime,
        "blocked_reason": if !runtime_safe_for_controlled_activation_ok { Some("native runtime requires trusted native execution flow and is not hot-loaded by this controlled endpoint") } else { None },
    }));
    if native_runtime_blocked {
        checks.push(serde_json::json!({
            "name": "trusted_native_profile_required",
            "ok": trusted_native_profile_ok && trusted_native_policy.allowed,
            "profile": profile.as_str(),
            "policy_reason": trusted_native_policy.reason,
        }));
        checks.push(serde_json::json!({
            "name": "trusted_native_explicit_confirmation_required",
            "ok": trusted_native_confirmation_ok,
            "confirmed": request.confirm,
        }));
        checks.push(serde_json::json!({
            "name": "trusted_native_manifest_allows_native",
            "ok": native_allowed,
            "native_allowed": native_allowed,
        }));
        checks.push(serde_json::json!({
            "name": "trusted_native_expected_digest_present",
            "ok": expected_digest_present,
            "expected_digest": expected_digest,
        }));
        checks.push(serde_json::json!({
            "name": "trusted_native_library_candidate_exists",
            "ok": native_library_candidate_exists,
            "candidate": native_library_candidate.as_ref().map(|path| path.display().to_string()),
            "entry_path": entry_path.as_ref().map(|path| path.display().to_string()),
        }));
    }
    checks.push(serde_json::json!({
        "name": "materialized_path_under_explicit_temp_or_managed_root",
        "ok": root_check_ok,
        "required_for_apply": true,
        "temp_root": normalize_path_for_guard(&std::env::temp_dir()).display().to_string(),
        "managed_root": materialized_managed_root(&state.repo_root).display().to_string(),
    }));
    checks.push(serde_json::json!({
        "name": "developer_or_trusted_profile_required_for_apply",
        "ok": profile_check_ok,
        "profile": profile.as_str(),
    }));
    checks.push(serde_json::json!({
        "name": "explicit_confirmation_required_for_apply",
        "ok": confirmation_check_ok,
        "confirmed": request.confirm,
    }));

    let validation_issues = checks
        .iter()
        .filter(|check| {
            if request.apply {
                return !check
                    .get("ok")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
            }

            match check.get("name").and_then(serde_json::Value::as_str) {
                Some("materialized_path_under_explicit_temp_or_managed_root")
                | Some("developer_or_trusted_profile_required_for_apply")
                | Some("explicit_confirmation_required_for_apply") => false,
                _ => !check
                    .get("ok")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            }
        })
        .filter_map(|check| check.get("name").and_then(serde_json::Value::as_str))
        .map(|name| format!("activation check failed: {name}"))
        .collect::<Vec<_>>();

    if native_runtime_blocked && !request.apply {
        return Ok(Json(serde_json::json!({
            "status": if ready_for_trusted_native_load { "ready_for_trusted_native_load" } else { "activation_plan_blocked" },
            "plan_only": true,
            "metadata_only": true,
            "activation_performed": false,
            "mutation_performed": false,
            "lock_mutation_performed": false,
            "native_load_performed": false,
            "package_path": package_path.display().to_string(),
            "manifest_found": true,
            "package": {
                "name": manifest.package_info.name,
                "version": manifest.package_info.version,
                "description": manifest.package_info.description,
                "runtime": runtime,
                "provides": manifest.resolved_provides(),
                "has_ui": false,
            },
            "activation_required": true,
            "ready_for_activation": false,
            "ready_for_trusted_native_load": ready_for_trusted_native_load,
            "trusted_native": {
                "native_allowed": native_allowed,
                "expected_digest_present": expected_digest_present,
                "expected_digest": expected_digest,
                "library_candidate": native_library_candidate.as_ref().map(|path| path.display().to_string()),
                "profile": profile.as_str(),
                "confirmed": request.confirm,
            },
            "requirements": {
                "profile": "trusted",
                "confirm": true,
                "native_allowed": true,
                "expected_digest": "present",
                "library_candidate": "exists",
                "native_load": "not performed by this plan endpoint"
            },
            "checks": checks,
            "validation_issues": validation_issues,
            "notes": [
                "This endpoint only plans trusted native activation and never loads native libraries.",
                "apply=true remains blocked for native runtimes unless a separate trusted native load flow is added."
            ],
        })));
    }

    if request.apply && !apply_allowed {
        return Ok(Json(serde_json::json!({
            "status": "activation_apply_blocked",
            "plan_only": false,
            "metadata_only": true,
            "activation_performed": false,
            "mutation_performed": false,
            "lock_mutation_performed": false,
            "package_path": package_path.display().to_string(),
            "manifest_found": true,
            "activation_required": true,
            "ready_for_activation": ready_for_activation,
            "checks": checks,
            "validation_issues": validation_issues,
            "requirements": {
                "apply_requires_confirm_true": true,
                "apply_requires_profile": "developer_or_trusted",
                "apply_requires_materialized_path_under": [
                    normalize_path_for_guard(&std::env::temp_dir()).display().to_string(),
                    materialized_managed_root(&state.repo_root).display().to_string()
                ],
                "native_runtime": "blocked; use trusted native execution flow",
                "missing_entry": "provide the runtime entry declared by package.toml before activation"
            }
        })));
    }

    if request.apply {
        let package = DiscoveredPackage {
            manifest: manifest.clone(),
            dir: normalize_path_for_guard(&package_path),
            entry_path: entry_path.clone(),
            runtime: package_runtime.clone(),
        };

        state.package_manager.write().await.register(PackageInfo {
            name: manifest.package_info.name.clone(),
            version: Some(manifest.package_info.version.clone()),
            overrides: vec![],
            enabled: true,
            has_ui: false,
            description: Some(manifest.package_info.description.clone()),
        });

        let mut service_registered = false;
        let mut runtime_started = false;
        let mut service_auto_start = false;
        let mut service_start_error: Option<String> = None;
        if package_runtime == PackageRuntime::Service {
            let service_config = build_service_config(&package).map_err(|error| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("failed to build service metadata: {error}"),
                        "activation_performed": false,
                        "mutation_performed": false,
                    })),
                )
            })?;
            service_auto_start = service_config.auto_start;
            state.process_manager.register(service_config).await;
            service_registered = true;

            if request.start_service {
                if service_auto_start {
                    service_start_error = Some(
                        "service declares persistent auto-start; controlled activation will not duplicate implicit auto-start".to_string(),
                    );
                } else {
                    match state
                        .process_manager
                        .start(&manifest.package_info.name)
                        .await
                    {
                        Ok(()) => {
                            runtime_started = true;
                        }
                        Err(error) => {
                            service_start_error = Some(error.to_string());
                        }
                    }
                }
            }
        }

        if manifest
            .resolved_provides()
            .contains(&"chat_channel".to_string())
        {
            let mut providers = state.chat_providers.write().await;
            providers.retain(|provider| provider.name != manifest.package_info.name);
            providers.push(ChatProviderInfo {
                name: manifest.package_info.name.clone(),
                endpoint: manifest
                    .resolved_chat_endpoint()
                    .unwrap_or_else(|| "/chat".to_string()),
                description: manifest.package_info.description.clone(),
            });
            providers.sort_by(|left, right| left.name.cmp(&right.name));
        }

        return Ok(Json(serde_json::json!({
            "status": "activation_metadata_registered",
            "plan_only": false,
            "metadata_only": true,
            "activation_performed": true,
            "mutation_performed": true,
            "lock_mutation_performed": false,
            "native_load_performed": false,
            "package_path": package_path.display().to_string(),
            "manifest_found": true,
            "package": {
                "name": manifest.package_info.name,
                "version": manifest.package_info.version,
                "description": manifest.package_info.description,
                "runtime": runtime,
                "provides": manifest.resolved_provides(),
                "has_ui": false,
            },
            "service_registered": service_registered,
            "service_start_requested": request.start_service,
            "service_auto_start": service_auto_start,
            "runtime_started": runtime_started,
            "service_start_error": service_start_error,
            "checks": checks,
            "validation_issues": validation_issues,
            "notes": [
                "Controlled activation registered package metadata only; it did not write lock files, copy packages, load native code, or start services unless start_service=true was explicitly requested for a non-auto-start service runtime.",
                "Native runtime remains blocked pending the trusted native execution flow."
            ],
        })));
    }

    Ok(Json(serde_json::json!({
        "status": if ready_for_activation { "activation_plan_ready" } else { "activation_plan_blocked" },
        "plan_only": true,
        "metadata_only": true,
        "activation_performed": false,
        "mutation_performed": false,
        "lock_mutation_performed": false,
        "package_path": package_path.display().to_string(),
        "manifest_found": true,
        "package": {
            "name": manifest.package_info.name,
            "version": manifest.package_info.version,
            "description": manifest.package_info.description,
            "runtime": runtime,
            "provides": manifest.resolved_provides(),
            "has_ui": false,
        },
        "activation_required": true,
        "ready_for_activation": ready_for_activation,
        "requirements": {
            "core_verification_required": true,
            "lock_mutation_required": true,
            "reload_or_runtime_start_required": true,
            "rollback_snapshot_required": true,
            "apply_requires_confirm_true": true,
            "apply_requires_profile": "developer_or_trusted",
            "apply_requires_materialized_path_under": [
                normalize_path_for_guard(&std::env::temp_dir()).display().to_string(),
                materialized_managed_root(&state.repo_root).display().to_string()
            ],
            "dependencies": dependencies,
        },
        "checks": checks,
        "validation_issues": validation_issues,
        "notes": [
            "apply defaults to false; without apply=true this endpoint is metadata-only and never loads, reloads, installs, activates, deletes, or writes package state.",
            "apply=true requires confirm=true, developer profile, a materialized path under the explicit temp/managed root, and a non-native runtime."
        ],
    })))
}
