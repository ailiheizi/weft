use serde_json::Value;
use std::path::{Component, Path, PathBuf};

pub async fn handle_core_capability(
    capability: &str,
    action: &str,
    data: &Value,
    workspace_root: Option<&Path>,
) -> Result<Value, String> {
    match capability {
        "core.files" => handle_files(action, data, workspace_root).await,
        "core.execution" => handle_execution(action, data).await,
        _ => Err(format!("Unknown core capability: {}", capability)),
    }
}

fn normalize_under_root(root: &Path, path_str: &str) -> Result<PathBuf, String> {
    if path_str.contains("..") {
        return Err("Path traversal is not allowed outside the configured workspace".into());
    }

    let candidate = if Path::new(path_str).is_absolute() {
        PathBuf::from(path_str)
    } else {
        root.join(path_str)
    };

    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(
                        "Path traversal is not allowed outside the configured workspace".into(),
                    );
                }
            }
        }
    }

    let root_canon = std::fs::canonicalize(root).map_err(|e| {
        format!(
            "Failed to resolve workspace root '{}': {}",
            root.display(),
            e
        )
    })?;

    if normalized.exists() {
        let candidate_canon = std::fs::canonicalize(&normalized)
            .map_err(|e| format!("Failed to resolve path '{}': {}", normalized.display(), e))?;
        if !candidate_canon.starts_with(&root_canon) {
            return Err("Path is outside the configured workspace".into());
        }
        Ok(candidate_canon)
    } else {
        let parent = normalized.parent().unwrap_or(root);
        let parent_canon = std::fs::canonicalize(parent).map_err(|e| {
            format!(
                "Failed to resolve parent path '{}': {}",
                parent.display(),
                e
            )
        })?;
        if !parent_canon.starts_with(&root_canon) {
            return Err("Path is outside the configured workspace".into());
        }
        Ok(normalized)
    }
}

fn resolve_file_path(workspace_root: Option<&Path>, path_str: &str) -> Result<PathBuf, String> {
    match workspace_root {
        Some(root) => normalize_under_root(root, path_str),
        None => Ok(PathBuf::from(path_str)),
    }
}

async fn handle_files(
    action: &str,
    data: &Value,
    workspace_root: Option<&Path>,
) -> Result<Value, String> {
    match action {
        "list" => {
            let path_str = data["path"].as_str().unwrap_or(".");
            let path_buf = resolve_file_path(workspace_root, path_str)?;
            let path = path_buf.as_path();
            if !path.exists() {
                return Err(format!("Path does not exist: {}", path_str));
            }
            let entries: Vec<Value> = match std::fs::read_dir(path) {
                Ok(reader) => reader
                    .filter_map(|e| e.ok())
                    .map(|entry| {
                        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                        serde_json::json!({
                            "name": entry.file_name().to_string_lossy(),
                            "is_dir": is_dir,
                        })
                    })
                    .collect(),
                Err(e) => return Err(format!("Failed to list directory: {}", e)),
            };
            Ok(serde_json::json!({ "entries": entries }))
        }
        "read" => {
            let path_str = data["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' in data".to_string())?;
            let path_buf = resolve_file_path(workspace_root, path_str)?;
            match std::fs::read_to_string(&path_buf) {
                Ok(content) => Ok(serde_json::json!({ "content": content })),
                Err(e) => Err(format!("Failed to read file: {}", e)),
            }
        }
        "write" => {
            let path_str = data["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' in data".to_string())?;
            let content = data["content"]
                .as_str()
                .ok_or_else(|| "Missing 'content' in data".to_string())?;
            let path_buf = resolve_file_path(workspace_root, path_str)?;
            if let Some(parent) = path_buf.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
            match std::fs::write(&path_buf, content) {
                Ok(()) => Ok(serde_json::json!({ "written": true, "path": path_str })),
                Err(e) => Err(format!("Failed to write file: {}", e)),
            }
        }
        "delete" => {
            let path_str = data["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' in data".to_string())?;
            let path_buf = resolve_file_path(workspace_root, path_str)?;
            let path = path_buf.as_path();
            let result = if path.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };
            match result {
                Ok(()) => Ok(serde_json::json!({ "deleted": true, "path": path_str })),
                Err(e) => Err(format!("Failed to delete: {}", e)),
            }
        }
        "metadata" => {
            let path_str = data["path"]
                .as_str()
                .ok_or_else(|| "Missing 'path' in data".to_string())?;
            let path_buf = resolve_file_path(workspace_root, path_str)?;
            match std::fs::metadata(&path_buf) {
                Ok(meta) => Ok(serde_json::json!({
                    "path": path_str,
                    "is_file": meta.is_file(),
                    "is_dir": meta.is_dir(),
                    "len": meta.len(),
                    "readonly": meta.permissions().readonly(),
                })),
                Err(e) => Err(format!("Failed to read metadata: {}", e)),
            }
        }
        _ => Err(format!("Unknown files action: {}", action)),
    }
}

async fn handle_execution(action: &str, data: &Value) -> Result<Value, String> {
    match action {
        "describe" => Ok(serde_json::json!({
            "capability": "core.execution",
            "actions": ["describe", "health", "run"],
            "runtime": "core",
        })),
        "health" => Ok(serde_json::json!({
            "capability": "core.execution",
            "healthy": true,
            "runtime": "core",
        })),
        "run" => {
            let mode = data["mode"]
                .as_str()
                .ok_or_else(|| "core.execution/run only supports mode=dry_run".to_string())?;
            if mode != "dry_run" {
                return Err("core.execution/run only supports mode=dry_run".into());
            }

            let command = data["command"]
                .as_str()
                .ok_or_else(|| "Missing 'command' in data".to_string())?;
            if command != "weft-core-version" {
                return Err(format!(
                    "core.execution/run dry_run does not support command: {}",
                    command
                ));
            }

            Ok(serde_json::json!({
                "mode": "dry_run",
                "command": command,
                "dry_run": true,
                "would_execute": false,
                "exit_code": 0,
                "stdout": "weft-core-version",
                "stderr": "",
            }))
        }
        _ => Err(format!("Unknown execution action: {}", action)),
    }
}

#[cfg(test)]
mod tests {
    use super::handle_core_capability;
    use serde_json::json;

    #[tokio::test]
    async fn execution_describe_and_health_are_available() {
        let describe = handle_core_capability("core.execution", "describe", &json!({}), None)
            .await
            .expect("describe should succeed");
        assert_eq!(describe["capability"], "core.execution");
        assert_eq!(describe["runtime"], "core");

        let health = handle_core_capability("core.execution", "health", &json!({}), None)
            .await
            .expect("health should succeed");
        assert_eq!(health["healthy"], true);
    }

    #[tokio::test]
    async fn execution_run_dry_run_weft_core_version_is_deterministic() {
        let result = handle_core_capability(
            "core.execution",
            "run",
            &json!({"mode":"dry_run","command":"weft-core-version"}),
            None,
        )
        .await
        .expect("dry-run weft-core-version should succeed");

        assert_eq!(result["mode"], "dry_run");
        assert_eq!(result["command"], "weft-core-version");
        assert_eq!(result["dry_run"], true);
        assert_eq!(result["would_execute"], false);
        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stdout"], "weft-core-version");
        assert_eq!(result["stderr"], "");
    }

    #[tokio::test]
    async fn execution_run_rejects_arbitrary_command_without_dry_run_mode() {
        let error =
            handle_core_capability("core.execution", "run", &json!({"command":"echo hi"}), None)
                .await
                .expect_err("run without dry_run mode should be rejected");

        assert!(error.contains("core.execution/run only supports mode=dry_run"));
    }

    #[tokio::test]
    async fn execution_run_rejects_dry_run_unknown_command() {
        let error = handle_core_capability(
            "core.execution",
            "run",
            &json!({"mode":"dry_run","command":"echo hi"}),
            None,
        )
        .await
        .expect_err("unknown dry-run command should be rejected");

        assert!(error.contains("dry_run does not support command: echo hi"));
    }

    #[tokio::test]
    async fn execution_run_real_process_path_is_not_reachable_by_default() {
        let error = handle_core_capability(
            "core.execution",
            "run",
            &json!({"mode":"execute","command":"weft-core-version"}),
            None,
        )
        .await
        .expect_err("non-dry-run execution should be rejected before process spawn");

        assert!(error.contains("core.execution/run only supports mode=dry_run"));
    }
}
