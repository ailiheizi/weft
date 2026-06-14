use crate::api::openai_compat::SharedPipeline;
use crate::config::ProviderConfig;
use crate::layers::{
    key_selector::ApiKeyState, ErrorAction, ErrorHandlerLayer, KeySelectorLayer, RequestError,
    RouterLayer,
};
use crate::package::config::PackagePermissions;
use crate::package::circuit_breaker::CircuitBreaker;
use crate::package::permissions::{Permission, PermissionChecker};
use crate::process::ProcessManager;
use crate::types::ChatRequest;
use crate::vkeys::VirtualKeyStore;
use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use extism::*;
use rusqlite::{
    types::{ToSql, ToSqlOutput, ValueRef},
    Connection, OpenFlags,
};
use std::collections::HashMap;
use std::io::{Read as IoRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::Handle;

// ── Shared host state (UserData for host functions) ──

/// Shared package map — allows host functions to call other plugins
/// without going through the outer WasmPackageHost lock.
pub type PackageMap = Arc<StdMutex<HashMap<String, Arc<StdMutex<Plugin>>>>>;

/// State shared across all WASM packages via Extism host functions.
/// Each package instance gets its own clone with a unique `caller_package_name`.
/// Shared resources (kv_store, package_map, etc.) are behind Arc so clones share them.
#[derive(Clone)]
pub struct WasmHostState {
    pub config: Arc<tokio::sync::RwLock<crate::config::AppConfig>>,
    pub pipeline: SharedPipeline,
    pub runtime_handle: Handle,
    pub process_manager: Arc<ProcessManager>,
    pub vkey_store: Arc<VirtualKeyStore>,
    pub kv_store: Arc<StdMutex<HashMap<String, String>>>,
    /// Name of the package that owns this UserData instance.
    pub caller_package_name: String,
    /// Filesystem directory the owning package was loaded from (official/ or
    /// installed/ — whichever the index resolved). Exposed to the guest as the
    /// `WEFT_PACKAGE_DIR` env var so package resources (skills/, mcp/, assets/)
    /// can be located relative to the package rather than a hardcoded repo path.
    pub package_dir: String,
    /// Permissions declared by the owning package's manifest ([permissions] in package.toml).
    pub permissions: PackagePermissions,
    /// Direct access to all loaded plugins for cross-package calls.
    pub package_map: PackageMap,
    pub package_aliases: Arc<StdMutex<HashMap<String, String>>>,
    /// Call depth counter to prevent infinite recursion in cross-package calls.
    pub call_depth: Arc<StdMutex<u32>>,
    pub app_state: Arc<StdMutex<Option<crate::api::openai_compat::AppState>>>,
}

/// Checks whether the calling package holds `perm`. On denial, returns the JSON
/// error string `{"error":"permission denied: ..."}` so the WASM guest sees a
/// structured error consistent with other host_fn failures.
/// Usage inside a `host_fn!` body: `gate_permission!(user_data, Permission::Storage);`
macro_rules! gate_permission {
    ($user_data:expr, $perm:expr) => {{
        let __ud = $user_data.get()?;
        let __ud = __ud.lock().unwrap();
        let __checker =
            PermissionChecker::new(&__ud.caller_package_name, __ud.permissions.clone());
        if let Err(__e) = __checker.check($perm) {
            tracing::warn!(
                "permission denied: package '{}' lacks '{}' for host call",
                __ud.caller_package_name,
                $perm
            );
            return Ok(serde_json::json!({
                "error": format!("permission denied: {}", __e)
            })
            .to_string());
        }
    }};
}

/// Unit-returning variant of `gate_permission!` for host_fn whose body returns `Ok(())`.
macro_rules! gate_permission_unit {
    ($user_data:expr, $perm:expr) => {{
        let __ud = $user_data.get()?;
        let __ud = __ud.lock().unwrap();
        let __checker =
            PermissionChecker::new(&__ud.caller_package_name, __ud.permissions.clone());
        if let Err(__e) = __checker.check($perm) {
            tracing::warn!(
                "permission denied: package '{}' lacks '{}' for host call (ignored)",
                __ud.caller_package_name,
                $perm
            );
            let _ = __e;
            return Ok(());
        }
    }};
}

/// Normalizes a package directory for injection as `WEFT_PACKAGE_DIR`:
/// strips the Windows `\\?\` verbatim prefix and converts backslashes to
/// forward slashes, so the guest can join `/skills` etc. cleanly.
fn normalize_package_dir(dir: &Path) -> String {
    let s = dir.display().to_string();
    let s = if let Some(r) = s.strip_prefix(r#"\\?\UNC\"#) {
        format!(r#"\\{}"#, r)
    } else if let Some(r) = s.strip_prefix(r#"\\?\"#) {
        r.to_string()
    } else {
        s
    };
    s.replace('\\', "/")
}

fn resolve_loaded_package_name(ud: &WasmHostState, requested: &str) -> String {
    let requested_trimmed = requested.trim();
    if requested_trimmed.is_empty() {
        return String::new();
    }

    let alias_target = ud
        .package_aliases
        .lock()
        .ok()
        .and_then(|aliases| aliases.get(requested_trimmed).cloned());

    if let Some(target) = alias_target {
        let target_trimmed = target.trim();
        if !target_trimmed.is_empty() {
            let has_target = ud
                .package_map
                .lock()
                .map(|map| map.contains_key(target_trimmed))
                .unwrap_or(false);
            if has_target {
                return target_trimmed.to_string();
            }
        }
    }

    requested_trimmed.to_string()
}

pub fn resolve_loaded_package_name_from_aliases(
    package_aliases: &HashMap<String, String>,
    loaded_package_names: &[String],
    requested: &str,
) -> String {
    let requested_trimmed = requested.trim();
    if requested_trimmed.is_empty() {
        return String::new();
    }

    if let Some(target) = package_aliases.get(requested_trimmed) {
        let target_trimmed = target.trim();
        if !target_trimmed.is_empty()
            && loaded_package_names
                .iter()
                .any(|loaded| loaded == target_trimmed)
        {
            return target_trimmed.to_string();
        }
    }

    requested_trimmed.to_string()
}

// ── Host functions ──

#[derive(Debug, Default, serde::Deserialize)]
struct HostExecInput {
    #[serde(default)]
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    stdin: Option<String>,
    #[serde(default)]
    stdin_base64: Option<String>,
    #[serde(default)]
    workdir: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    env: HashMap<String, String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct HostChatCompletionInput {
    #[serde(default)]
    request_label: String,
    #[serde(default)]
    endpoint: String,
    #[serde(default)]
    body: String,
}

fn exec_result_json(status: i32, stdout: &[u8], stderr: &[u8]) -> String {
    serde_json::json!({
        "status": status,
        "stdout": String::from_utf8_lossy(stdout),
        "stderr": String::from_utf8_lossy(stderr),
        "stdout_base64": base64::engine::general_purpose::STANDARD.encode(stdout),
        "stderr_base64": base64::engine::general_purpose::STANDARD.encode(stderr),
    })
    .to_string()
}

fn run_host_exec_command(parsed: HostExecInput) -> String {
    if parsed.command.trim().is_empty() {
        return r#"{"error":"missing command"}"#.to_string();
    }

    let mut command = std::process::Command::new(&parsed.command);
    command.args(&parsed.args);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    if parsed.stdin.is_some() || parsed.stdin_base64.is_some() {
        command.stdin(std::process::Stdio::piped());
    }

    if let Some(workdir) = parsed
        .workdir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        command.current_dir(workdir);
    }

    if !parsed.env.is_empty() {
        command.envs(parsed.env.iter());
    }

    let timeout_ms = parsed.timeout_ms;
    let stdin_text = parsed.stdin;
    let stdin_base64 = parsed.stdin_base64;

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return format!(r#"{{"error":"{}"}}"#, error),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let write_result = if let Some(encoded) = stdin_base64 {
            match base64::engine::general_purpose::STANDARD.decode(encoded) {
                Ok(bytes) => stdin.write_all(&bytes),
                Err(error) => {
                    return format!(r#"{{"error":"invalid stdin_base64: {}"}}"#, error);
                }
            }
        } else if let Some(text) = stdin_text {
            stdin.write_all(text.as_bytes())
        } else {
            Ok(())
        };

        if let Err(error) = write_result {
            return format!(r#"{{"error":"failed to write stdin: {}"}}"#, error);
        }
    }

    if let Some(timeout_ms) = timeout_ms {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    return match child.wait_with_output() {
                        Ok(output) => exec_result_json(
                            output.status.code().unwrap_or(-1),
                            &output.stdout,
                            &output.stderr,
                        ),
                        Err(error) => format!(r#"{{"error":"{}"}}"#, error),
                    };
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        return match child.wait_with_output() {
                            Ok(output) => {
                                let timeout_stderr = format!(
                                    "{}{}command timed out after {}ms",
                                    String::from_utf8_lossy(&output.stderr),
                                    if output.stderr.is_empty() { "" } else { "\n" },
                                    timeout_ms
                                );
                                exec_result_json(-1, &output.stdout, timeout_stderr.as_bytes())
                            }
                            Err(error) => format!(r#"{{"error":"{}"}}"#, error),
                        };
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return format!(r#"{{"error":"{}"}}"#, error),
            }
        }
    }

    match child.wait_with_output() {
        Ok(output) => exec_result_json(
            output.status.code().unwrap_or(-1),
            &output.stdout,
            &output.stderr,
        ),
        Err(error) => format!(r#"{{"error":"{}"}}"#, error),
    }
}

host_fn!(pub host_log(user_data: WasmHostState; input: String) -> String {
    // input is JSON: ["level", "msg"]
    let parsed: Vec<String> = serde_json::from_str(&input).unwrap_or_default();
    let level = parsed.first().map(|s| s.as_str()).unwrap_or("info");
    let msg = parsed.get(1).map(|s| s.as_str()).unwrap_or("");
    match level {
        "error" => tracing::error!("[package] {}", msg),
        "warn"  => tracing::warn!("[package] {}", msg),
        "debug" => tracing::debug!("[package] {}", msg),
        _       => tracing::info!("[package] {}", msg),
    }
    Ok("ok".to_string())
});

host_fn!(pub host_kv_get(user_data: WasmHostState; key: String) -> String {
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let store = ud.kv_store.lock().unwrap();
    Ok(store.get(&key).cloned().unwrap_or_default())
});

host_fn!(pub host_kv_set(user_data: WasmHostState; input: String) {
    // input is JSON: ["key", "value"]
    let parsed: Vec<String> = serde_json::from_str(&input).unwrap_or_default();
    let key = parsed.first().cloned().unwrap_or_default();
    let value = parsed.get(1).cloned().unwrap_or_default();
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    ud.kv_store.lock().unwrap().insert(key, value);
    Ok(())
});

host_fn!(pub host_kv_list(user_data: WasmHostState; prefix: String) -> String {
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let store = ud.kv_store.lock().unwrap();
    let keys: Vec<String> = store
        .keys()
        .filter(|key| key.starts_with(&prefix))
        .cloned()
        .collect();
    Ok(serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string()))
});

host_fn!(pub host_kv_delete(user_data: WasmHostState; key: String) {
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    ud.kv_store.lock().unwrap().remove(&key);
    Ok(())
});

host_fn!(pub host_env_get(user_data: WasmHostState; key: String) -> String {
    // WEFT_PACKAGE_DIR: 返回该包的实际加载目录（per-package），供包定位自身资源。
    if key == "WEFT_PACKAGE_DIR" {
        let ud = user_data.get()?;
        let ud = ud.lock().unwrap();
        return Ok(ud.package_dir.clone());
    }
    Ok(std::env::var(&key).unwrap_or_default())
});

host_fn!(pub host_home_dir(user_data: WasmHostState; input: String) -> String {
    let _ = user_data;
    let _ = input;
    Ok(std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default())
});

host_fn!(pub host_read_file(user_data: WasmHostState; path: String) -> String {
    gate_permission!(user_data, Permission::Storage);
    if path.contains("..") {
        return Ok(r#"{"error":"path traversal not allowed"}"#.to_string());
    }
    let path_ref = std::path::Path::new(&path);
    match std::fs::metadata(path_ref) {
        Ok(metadata) if metadata.is_dir() => {
            return Ok(r#"{"error":"path is a directory; use fs_list instead","suggested_tool":"fs_list"}"#.to_string());
        }
        Ok(_) => {}
        Err(e) => return Ok(format!(r#"{{"error":"{}"}}"#, e)),
    }
    match std::fs::read_to_string(path_ref) {
        Ok(content) => Ok(content),
        Err(e) => Ok(format!(r#"{{"error":"{}"}}"#, e)),
    }
});

host_fn!(pub host_write_file(user_data: WasmHostState; input: String) {
    gate_permission_unit!(user_data, Permission::Storage);
    // input is JSON: ["path", "content"]
    let parsed: Vec<String> = serde_json::from_str(&input).unwrap_or_default();
    let path = parsed.first().cloned().unwrap_or_default();
    let content = parsed.get(1).cloned().unwrap_or_default();
    if path.contains("..") {
        return Ok(());
    }
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &content);
    Ok(())
});

// Write binary content (decoded from base64) to a file. For large media
// (images/audio/video) that can't go through host_write_file's String content.
// input JSON: ["path", "<base64>"]. Output JSON: {ok:true,path} or {error}.
host_fn!(pub host_write_file_base64(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Storage);
    let parsed: Vec<String> = serde_json::from_str(&input).unwrap_or_default();
    let path = parsed.first().cloned().unwrap_or_default();
    let b64 = parsed.get(1).cloned().unwrap_or_default();
    if path.is_empty() {
        return Ok(r#"{"error":"host_write_file_base64 requires path"}"#.to_string());
    }
    if path.contains("..") {
        return Ok(r#"{"error":"path traversal not allowed"}"#.to_string());
    }
    let bytes = match base64::engine::general_purpose::STANDARD.decode(b64.trim()) {
        Ok(b) => b,
        Err(e) => return Ok(serde_json::json!({ "error": format!("invalid base64: {e}") }).to_string()),
    };
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&path, &bytes) {
        Ok(()) => Ok(serde_json::json!({ "ok": true, "path": path, "bytes": bytes.len() }).to_string()),
        Err(e) => Ok(serde_json::json!({ "error": format!("write failed: {e}") }).to_string()),
    }
});

host_fn!(pub host_list_dir(user_data: WasmHostState; path: String) -> String {
    gate_permission!(user_data, Permission::Storage);
    if path.contains("..") {
        return Ok(r#"{"error":"path traversal not allowed"}"#.to_string());
    }
    match std::fs::read_dir(&path) {
        Ok(entries) => {
            let items: Vec<serde_json::Value> = entries
                .flatten()
                .map(|e| {
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    serde_json::json!({
                        "name": e.file_name().to_string_lossy(),
                        "is_dir": is_dir,
                    })
                })
                .collect();
            Ok(serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string()))
        }
        Err(e) => Ok(format!(r#"{{"error":"{}"}}"#, e)),
    }
});

host_fn!(pub host_exec(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Process);
    let parsed: HostExecInput = serde_json::from_str(&input).unwrap_or_default();
    Ok(run_host_exec_command(parsed))
});

host_fn!(pub host_exec_advanced(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Process);
    let parsed: HostExecInput = serde_json::from_str(&input).unwrap_or_default();
    Ok(run_host_exec_command(parsed))
});

host_fn!(pub host_chat_completion(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Network);
    let parsed: HostChatCompletionInput = serde_json::from_str(&input).unwrap_or_default();
    if parsed.body.trim().is_empty() {
        return Ok(r#"{"error":"missing body"}"#.to_string());
    }

    let request: ChatRequest = match serde_json::from_str(&parsed.body) {
        Ok(request) => request,
        Err(error) => {
            return Ok(serde_json::json!({
                "error": format!("invalid chat completion body: {}", error),
            })
            .to_string())
        }
    };

    if request.stream {
        return Ok(r#"{"error":"streaming chat completions are not supported via host_chat_completion"}"#.to_string());
    }

    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let Some(app_state) = ud.app_state.lock().ok().and_then(|state| state.clone()) else {
        return Ok(r#"{"error":"app state unavailable for host_chat_completion"}"#.to_string());
    };

    let request_label = parsed.request_label.trim().to_string();
    let endpoint = parsed.endpoint.trim().to_string();



    let result = ud.runtime_handle.block_on(async move {
        let config = app_state.config.read().await;
        app_state.pipeline.execute(&request, &config).await
    });

    match result {
        Ok(response) => serde_json::to_string(&response).map_err(|error| {
            extism::Error::msg(format!(
                "host_chat_completion failed to serialize response (label='{}', endpoint='{}'): {}",
                request_label, endpoint, error
            ))
        }),
        Err(error) => Ok(serde_json::json!({
            "error": format!(
                "host_chat_completion failed (label='{}', endpoint='{}'): {}",
                request_label,
                endpoint,
                error
            ),
        })
        .to_string()),
    }
});

// Generic HTTP egress for WASM packages. Mirrors host_chat_completion's
// Network permission gate, but issues an arbitrary HTTP request via reqwest
// (supports custom headers, e.g. Authorization for external model APIs).
// Input JSON: {method, url, headers:{k:v}, body}. Output JSON: {status, body}.
host_fn!(pub host_http_request(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Network);

    #[derive(serde::Deserialize, Default)]
    struct HttpReqInput {
        #[serde(default)]
        method: String,
        #[serde(default)]
        url: String,
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
        #[serde(default)]
        body: String,
    }

    let parsed: HttpReqInput = serde_json::from_str(&input).unwrap_or_default();
    if parsed.url.trim().is_empty() {
        return Ok(r#"{"error":"host_http_request requires url"}"#.to_string());
    }
    let method = if parsed.method.trim().is_empty() {
        "GET".to_string()
    } else {
        parsed.method.trim().to_uppercase()
    };

    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let runtime_handle = ud.runtime_handle.clone();
    drop(ud);

    let result: Result<(u16, String), String> = runtime_handle.block_on(async move {
        let client = reqwest::Client::new();
        let m = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|e| format!("invalid method: {e}"))?;
        let mut req = client.request(m, &parsed.url);
        for (k, v) in &parsed.headers {
            req = req.header(k.as_str(), v.as_str());
        }
        if !parsed.body.is_empty() {
            req = req.body(parsed.body.clone());
        }
        let resp = req.send().await.map_err(|e| format!("request failed: {e}"))?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| format!("read body failed: {e}"))?;
        Ok((status, text))
    });

    match result {
        Ok((status, body)) => Ok(serde_json::json!({ "status": status, "body": body }).to_string()),
        Err(error) => Ok(serde_json::json!({ "error": error }).to_string()),
    }
});


/// Given the raw content after `"assistant":"` in a streaming JSON response,
/// return the decoded string up to (but not including) the first unescaped closing quote.
fn extract_json_string_prefix(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some(other) => { out.push('\\'); out.push(other); }
                    None => {}
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Returns true if the raw content after `"assistant":"` contains a complete JSON string.
fn is_json_string_complete(raw: &str) -> bool {
    let mut escaped = false;
    for c in raw.chars() {
        if escaped { escaped = false; continue; }
        if c == '\\' { escaped = true; continue; }
        if c == '"' { return true; }
    }
    false
}

host_fn!(pub host_chat_completion_stream(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Network);
    #[derive(serde::Deserialize, Default)]
    struct StreamInput {
        #[serde(default)] request_label: String,
        #[serde(default)] body: String,
        #[serde(default)] session_id: String,
    }

    let parsed: StreamInput = serde_json::from_str(&input).unwrap_or_default();
    if parsed.body.trim().is_empty() {
        return Ok(r#"{"error":"missing body"}"#.to_string());
    }
    if parsed.session_id.trim().is_empty() {
        return Ok(r#"{"error":"missing session_id"}"#.to_string());
    }

    let mut request: crate::types::ChatRequest = match serde_json::from_str(&parsed.body) {
        Ok(r) => r,
        Err(e) => return Ok(serde_json::json!({"error": format!("invalid body: {}", e)}).to_string()),
    };
    request.stream = true;

    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let Some(app_state) = ud.app_state.lock().ok().and_then(|s| s.clone()) else {
        return Ok(r#"{"error":"app state unavailable"}"#.to_string());
    };

    let session_id = parsed.session_id.clone();
    let request_label = parsed.request_label.clone();
    let runtime_handle = ud.runtime_handle.clone();

    // Use std::sync::mpsc to bridge the async stream back to this sync WASM thread.
    // The stream task runs on the EXISTING tokio runtime (via spawn), so reqwest's
    // connection pool and wakers stay on the correct runtime — no cross-runtime deadlock.
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();

    runtime_handle.spawn(async move {
        use futures_util::StreamExt;

        let config = app_state.config.read().await.clone();
        // 对瞬时错误（5xx/网关 502/网络抖动）做有限重试：LLM 网关偶发 502
        // 不应让整个对话回合失败。execute_stream 对 status>=400 会 bail，
        // 错误信息含状态码，据此判断是否可重试（4xx 不重试）。
        let mut attempt = 0u32;
        const MAX_STREAM_ATTEMPTS: u32 = 3;
        let (provider_name, resp) = loop {
            attempt += 1;
            // 给 execute_stream（含 .send().await 建连+首字节）加 30s 超时：
            // 卡在首字节前不会无限挂起，超时按可重试错误处理。
            let stream_result = match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                app_state.pipeline.execute_stream(&request, &config),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => Err(anyhow::anyhow!("execute_stream timed out after 30s")),
            };
            match stream_result {
                Ok(r) => break r,
                Err(e) => {
                    let msg = e.to_string();
                    // 4xx 客户端错误（鉴权/参数）不可重试。
                    let is_client_error = msg.contains("status 4");
                    if attempt >= MAX_STREAM_ATTEMPTS || is_client_error {
                        let _ = tx.send(Err(format!("stream request failed: {}", msg)));
                        return;
                    }
                    tracing::warn!(
                        "stream completion attempt {}/{} failed, retrying: {}",
                        attempt, MAX_STREAM_ATTEMPTS, msg
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
                }
            }
        };

        let provider = config.providers.iter().find(|p| p.name == provider_name).cloned();
        let transforms = app_state.pipeline.transforms.clone();

        let mut byte_stream = resp.bytes_stream();
        // 字节级缓冲：避免多字节 UTF-8 字符（中文等）跨 chunk 边界被
        // from_utf8_lossy 截成 �。在字节层按 \n 切行，只对完整行解码。
        let mut buffer: Vec<u8> = Vec::new();
        let mut full_reply = String::new();
        // Track position inside the "assistant" JSON string value for real-time extraction.
        // States: 0=before assistant key, 1=inside assistant value, 2=after assistant value
        let mut assistant_state: u8 = 0;
        let mut assistant_buf = String::new(); // accumulated raw chars inside assistant value
        let mut last_emitted_len: usize = 0;   // how many chars of assistant_buf already emitted

        loop {
            // 流式 60s 无新 chunk → 判定卡死，中断而非无限阻塞 WASM 实例锁。
            // （reqwest client.timeout 只管到首字节，body 逐块读取不受其约束。）
            let next = tokio::time::timeout(
                std::time::Duration::from_secs(60),
                byte_stream.next(),
            )
            .await;
            let chunk_result = match next {
                Ok(Some(item)) => item,
                Ok(None) => break, // 流正常结束
                Err(_) => {
                    tracing::warn!("stream idle timeout (60s without chunk), aborting");
                    break;
                }
            };
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => { tracing::warn!("stream chunk error: {}", e); break; }
            };
            buffer.extend_from_slice(&chunk);

            while let Some(nl) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=nl).collect();
                let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
                if line.is_empty() { continue; }

                if let Some(ref prov) = provider {
                    match transforms.for_format(&prov.format).transform_stream_chunk(&line, prov).await {
                        Ok(Some(chunk_obj)) => {
                            let delta = chunk_obj.choices.first()
                                .and_then(|c| c.delta.content.as_deref())
                                .unwrap_or("");
                            if !delta.is_empty() {
                                full_reply.push_str(delta);

                                // Real-time extraction of the assistant field value.
                                // We scan the accumulated full_reply for the assistant key,
                                // then emit new characters as they arrive.
                                if assistant_state < 2 {
                                    if assistant_state == 0 {
                                        // Look for `"assistant":"` in full_reply
                                        if let Some(pos) = full_reply.find(r#""assistant":""#) {
                                            let start = pos + r#""assistant":""#.len();
                                            assistant_buf = full_reply[start..].to_string();
                                            assistant_state = 1;
                                        }
                                    } else {
                                        // Append new delta to assistant_buf
                                        assistant_buf.push_str(delta);
                                    }

                                    if assistant_state == 1 {
                                        // Find the closing unescaped quote
                                        let visible = extract_json_string_prefix(&assistant_buf);
                                        if visible.len() > last_emitted_len {
                                            let new_text = &visible[last_emitted_len..];
                                            if !new_text.is_empty() {
                                                if let Ok(mut buf) = app_state.stream_buffer.lock() {
                                                    buf.entry(session_id.clone()).or_default().push(new_text.to_string());
                                                }
                                            }
                                            last_emitted_len = visible.len();
                                        }
                                        // Check if assistant value is complete (closing quote found)
                                        if is_json_string_complete(&assistant_buf) {
                                            assistant_state = 2;
                                        }
                                    }
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(e) => { tracing::warn!("stream transform error (label={}): {}", request_label, e); }
                    }
                }
            }
        }

        // Extract the user-visible assistant text from the agent JSON plan.
        // agent-core requires LLM to output {"mode":"reply","assistant":"...","tool_calls":[]},
        // so raw stream deltas are JSON fragments. After the stream completes we parse the
        // full reply, extract the assistant field, clear the raw tokens, and write the
        // readable text in small chunks so the Flutter client sees a typewriter effect.
        let visible_text = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&full_reply) {
            v.get("assistant")
                .and_then(|a| a.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| full_reply.clone())
        } else {
            full_reply.clone()
        };

        // Replace raw JSON tokens with readable text chunks (~20 chars each).
        if let Ok(mut buf) = app_state.stream_buffer.lock() {
            buf.remove(&session_id);
            let chunk_size = 20usize;
            let chars: Vec<char> = visible_text.chars().collect();
            for chunk in chars.chunks(chunk_size) {
                buf.entry(session_id.clone()).or_default().push(chunk.iter().collect());
            }
        }

        let _ = tx.send(Ok(full_reply));
    });

    // Block this WASM thread until the stream completes —但有上界。
    // 即使 spawn 的异步任务永不发送（卡在 execute_stream 的 .send() 前、panic 等），
    // recv_timeout 也会在 180s 后返回，避免无限阻塞 WASM 实例锁、冻结整个 core。
    let result = match rx.recv_timeout(std::time::Duration::from_secs(180)) {
        Ok(r) => r,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            tracing::warn!(
                "host_chat_completion_stream timed out after 180s (no response from stream task)"
            );
            Err("stream completion timed out after 180s".to_string())
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err("stream channel closed unexpectedly".to_string())
        }
    };

    match result {
        Ok(reply) => Ok(serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": reply}, "finish_reason": "stop"}],
            "object": "chat.completion",
        }).to_string()),
        Err(e) => Ok(serde_json::json!({"error": e}).to_string()),
    }
});

host_fn!(pub host_now_ms(user_data: WasmHostState; input: String) -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    Ok(now_ms.to_string())
});

host_fn!(pub host_process_spawn(user_data: WasmHostState; config_json: String) -> String {
    gate_permission!(user_data, Permission::Process);
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&config_json).unwrap_or_default();
    let name = parsed["name"].as_str().unwrap_or("").to_string();
    if name.is_empty() {
        return Ok(r#"{"error":"missing name"}"#.to_string());
    }

    let svc_config = crate::config::ServiceConfig {
        name: name.clone(),
        command: parsed["command"].as_str().unwrap_or("").to_string(),
        args: parsed["args"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        workdir: parsed["workdir"].as_str().map(String::from),
        env: parsed["env"]
            .as_object()
            .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
            .unwrap_or_default(),
        health_url: parsed["health_url"].as_str().map(String::from),
        health_interval: 10,
        auto_start: false,
        restart_on_crash: parsed["restart_on_crash"].as_bool().unwrap_or(false),
    };

    if ud.process_manager.status_sync(&name).is_none() {
        ud.process_manager.register_sync(svc_config);
    }
    match ud.process_manager.start_sync(&name) {
        Ok(()) => Ok(format!(r#"{{"status":"ok","name":"{}"}}"#, name)),
        Err(e) => Ok(format!(r#"{{"error":"{}"}}"#, e)),
    }
});

host_fn!(pub host_process_stop(user_data: WasmHostState; name: String) -> String {
    gate_permission!(user_data, Permission::Process);
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    match ud.process_manager.stop_sync(&name) {
        Ok(()) => Ok(format!(r#"{{"status":"ok","name":"{}"}}"#, name)),
        Err(e) => Ok(format!(r#"{{"error":"{}"}}"#, e)),
    }
});

host_fn!(pub host_process_status(user_data: WasmHostState; name: String) -> String {
    gate_permission!(user_data, Permission::Process);
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    match ud.process_manager.status_sync(&name) {
        Some(s) => Ok(format!(r#"{{"status":"{}"}}"#, s)),
        None => Ok(r#"{"status":"not_registered"}"#.to_string()),
    }
});

host_fn!(pub host_process_write_stdin(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Process);
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let name = parsed["name"].as_str().unwrap_or("");
    let data = parsed["input"].as_str().unwrap_or("");
    if name.is_empty() {
        return Ok(r#"{"error":"missing name"}"#.to_string());
    }
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    match ud.process_manager.write_stdin_sync(name, data) {
        Ok(()) => Ok(serde_json::json!({"status":"ok","name":name}).to_string()),
        Err(e) => Ok(serde_json::json!({"error":e.to_string()}).to_string()),
    }
});

host_fn!(pub host_process_read_stdout(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Process);
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let name = parsed["name"].as_str().unwrap_or("");
    let offset = parsed["offset"].as_u64().unwrap_or(0) as usize;
    if name.is_empty() {
        return Ok(r#"{"error":"missing name"}"#.to_string());
    }
    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    match ud.process_manager.read_stdout_since_sync(name, offset) {
        Ok((next_offset, chunk)) => Ok(serde_json::json!({
            "status":"ok",
            "name": name,
            "next_offset": next_offset,
            "chunk": chunk,
        }).to_string()),
        Err(e) => Ok(serde_json::json!({"error":e.to_string()}).to_string()),
    }
});

fn execute_js_extension_runtime_service(
    envelope: &serde_json::Value,
    ud: &WasmHostState,
) -> serde_json::Value {
    let Some(service_config) = ud
        .process_manager
        .service_config_sync("js-extension-runtime")
    else {
        return serde_json::json!({"error":"service package 'js-extension-runtime' is not registered"});
    };
    let Some(health_url) = service_config.health_url.as_deref() else {
        return serde_json::json!({"error":"service package 'js-extension-runtime' has no health url"});
    };
    let base_url = health_url.trim_end_matches("/health");
    let action = envelope
        .get("action")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let data = envelope
        .get("data")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let request_body = serde_json::json!({
        "id": "weft-tool-executor",
        "action": action,
        "payload": data,
    });
    let request_body = match serde_json::to_vec(&request_body) {
        Ok(body) => body,
        Err(error) => {
            return serde_json::json!({"error": format!("failed to serialize service request: {}", error)})
        }
    };
    let request_url = format!("{}/execute", base_url);
    let result = std::thread::spawn(move || -> serde_json::Value {
        let parsed_url = match reqwest::Url::parse(&request_url) {
            Ok(url) => url,
            Err(error) => return serde_json::json!({"error": format!("invalid service url: {}", error)}),
        };
        let host = match parsed_url.host_str() {
            Some(host) => host.to_string(),
            None => return serde_json::json!({"error":"service url has no host"}),
        };
        let port = parsed_url.port_or_known_default().unwrap_or(80);
        let path = if parsed_url.path().is_empty() {
            "/"
        } else {
            parsed_url.path()
        };
        let mut stream = match std::net::TcpStream::connect((host.as_str(), port)) {
            Ok(stream) => stream,
            Err(error) => return serde_json::json!({"error": format!("service connect failed: {}", error)}),
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nAccept: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            path,
            host,
            port,
            request_body.len()
        );
        if let Err(error) = stream.write_all(request.as_bytes()).and_then(|_| stream.write_all(&request_body)) {
            return serde_json::json!({"error": format!("service write failed: {}", error)});
        }
        let mut response = Vec::new();
        if let Err(error) = stream.read_to_end(&mut response) {
            return serde_json::json!({"error": format!("service read failed: {}", error)});
        }
        let raw = String::from_utf8_lossy(&response);
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").trim();
        serde_json::from_str(body).unwrap_or_else(|error| {
            serde_json::json!({"error": format!("service returned invalid JSON: {}", error)})
        })
    })
    .join()
    .unwrap_or_else(|_| serde_json::json!({"error":"service request thread panicked"}));
    result
}

/// Max cross-package call depth to prevent infinite recursion.
const MAX_CALL_DEPTH: u32 = 8;

host_fn!(pub host_call_package(user_data: WasmHostState; input: String) -> String {
    // input: {"package":"name","func":"fn_name","args":"json_string"}
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let target = parsed["package"].as_str().or_else(|| parsed["plugin"].as_str()).unwrap_or("");
    let func = parsed["func"].as_str().unwrap_or("");
    let args = parsed["args"].as_str().unwrap_or("");

    if target.is_empty() || func.is_empty() {
        return Ok(r#"{"error":"missing package or func"}"#.to_string());
    }

    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let resolved_target = resolve_loaded_package_name(&ud, target);
    let display_target = if resolved_target.is_empty() { target } else { &resolved_target };

    // Check call depth
    {
        let mut depth = ud.call_depth.lock().unwrap();
        if *depth >= MAX_CALL_DEPTH {
            return Ok(format!(
                r#"{{"error":"max cross-package call depth ({}) exceeded: {} -> {}::{}"}}"#,
                MAX_CALL_DEPTH, ud.caller_package_name, display_target, func
            ));
        }
        *depth += 1;
    }

    // Get the target plugin's lock from the shared package map
    let plugin_arc = {
        let map = ud.package_map.lock().unwrap();
        match map.get(display_target) {
            Some(p) => p.clone(),
            None => {
                if func == "handle_ws_message" {
                    if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(args) {
                        if display_target == "js-extension-runtime" {
                            let result = execute_js_extension_runtime_service(&envelope, &ud);
                            *ud.call_depth.lock().unwrap() -= 1;
                            return Ok(result.to_string());
                        }
                        if envelope.get("action").and_then(|value| value.as_str()).is_some() {
                            if let Some(app_state) = ud.app_state.lock().ok().and_then(|state| state.clone()) {
                                let package_name = display_target.to_string();
                                let result = ud.runtime_handle.block_on(async move {
                                    crate::api::package_ws::dispatch_package_payload(&package_name, envelope, &app_state).await
                                });
                                *ud.call_depth.lock().unwrap() -= 1;
                                return Ok(serde_json::to_string(&result).unwrap_or_else(|error| {
                                    format!(r#"{{\"error\":\"{}\"}}"#, error)
                                }));
                            }
                        }
                    }
                }
                // Decrement depth before returning
                *ud.call_depth.lock().unwrap() -= 1;
                return Ok(format!(
                    r#"{{"error":"host_call_package missing target '{}' for func '{}'"}}"#,
                    display_target, func
                ));
            }
        }
    };

    // Lock only the target package (per-package lock, no outer lock needed)
    let result = {
        let mut package = plugin_arc.lock().map_err(|e| {
            *ud.call_depth.lock().unwrap() -= 1;
            extism::Error::msg(format!("package '{}' lock poisoned: {}", display_target, e))
        })?;
        package.call::<&str, &str>(func, args)
            .map(|s| s.to_string())
            .map_err(|e| format!("{}", e))
    };

    // Decrement depth
    *ud.call_depth.lock().unwrap() -= 1;

    match result {
        Ok(s) => Ok(s),
        Err(e) => Ok(format!(r#"{{"error":"{}"}}"#, e.replace('"', "\\\""))),
    }
});

host_fn!(pub host_call_package_ws(user_data: WasmHostState; input: String) -> String {
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let target = parsed["package"].as_str().or_else(|| parsed["plugin"].as_str()).unwrap_or("");
    let action = parsed["action"].as_str().unwrap_or("");
    let data = parsed["data"].clone();

    if target.is_empty() || action.is_empty() {
        return Ok(r#"{"error":"missing package or action"}"#.to_string());
    }

    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let resolved_target = resolve_loaded_package_name(&ud, target);
    let display_target = if resolved_target.is_empty() { target } else { &resolved_target };

    {
        let mut depth = ud.call_depth.lock().unwrap();
        if *depth >= MAX_CALL_DEPTH {
            return Ok(format!(
                r#"{{"error":"max cross-package call depth ({}) exceeded: {} -> {}::handle_ws_message"}}"#,
                MAX_CALL_DEPTH, ud.caller_package_name, display_target
            ));
        }
        *depth += 1;
    }

    let plugin_arc = {
        let map = ud.package_map.lock().unwrap();
        match map.get(display_target) {
            Some(package) => package.clone(),
            None => {
                let has_app_state = ud
                    .app_state
                    .lock()
                    .map(|state| state.is_some())
                    .unwrap_or(false);
                tracing::warn!(
                    "host_call_package_ws target '{}' is not a loaded WASM package; app_state_available={}",
                    display_target,
                    has_app_state
                );
                if let Some(app_state) = ud.app_state.lock().ok().and_then(|state| state.clone()) {
                    let package_name = display_target.to_string();
                    let envelope = serde_json::json!({
                        "action": action,
                        "data": data,
                    });
                    let result = ud.runtime_handle.block_on(async move {
                        crate::api::package_ws::dispatch_package_payload(&package_name, envelope, &app_state).await
                    });
                    *ud.call_depth.lock().unwrap() -= 1;
                    return Ok(serde_json::to_string(&result).unwrap_or_else(|error| {
                        format!(r#"{{"error":"{}"}}"#, error)
                    }));
                }
                *ud.call_depth.lock().unwrap() -= 1;
                return Ok(format!(r#"{{"error":"package '{}' not loaded"}}"#, display_target));
            }
        }
    };

    let envelope = serde_json::json!({
        "action": action,
        "data": data,
    })
    .to_string();

    let result = {
        let mut package = plugin_arc.lock().map_err(|e| {
            *ud.call_depth.lock().unwrap() -= 1;
            extism::Error::msg(format!("package '{}' lock poisoned: {}", display_target, e))
        })?;
        package
            .call::<&str, &str>("handle_ws_message", &envelope)
            .map(|s| s.to_string())
            .map_err(|e| format!("{}", e))
    };

    *ud.call_depth.lock().unwrap() -= 1;

    match result {
        Ok(s) => Ok(s),
        Err(e) => Ok(format!(r#"{{"error":"{}"}}"#, e.replace('"', "\\\""))),
    }
});

host_fn!(pub host_capability_call(user_data: WasmHostState; input: String) -> String {
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let capability = parsed["capability"].as_str().unwrap_or("");
    let action = parsed["action"].as_str().unwrap_or("call");
    let data = parsed["data"].clone();
    let app = parsed["app"].as_str().map(|s| s.to_string());
    let provider = parsed["provider"].as_str().map(|s| s.to_string());

    if capability.is_empty() {
        return Ok(r#"{"error":"missing capability"}"#.to_string());
    }

    let ud = user_data.get()?;
    let ud = ud.lock().unwrap();
    let Some(app_state) = ud.app_state.lock().ok().and_then(|state| state.clone()) else {
        return Ok(r#"{"error":"app state unavailable for capability routing"}"#.to_string());
    };

    let capability_name = capability.to_string();
    let payload = serde_json::json!({
        "action": action,
        "data": data,
        "app": app,
        "provider": provider,
    });

    let result = ud.runtime_handle.block_on(async move {
        crate::api::capabilities::execute_capability_call(&app_state, &capability_name, payload).await
    });

    match result {
        Ok(value) => Ok(value.to_string()),
        Err((status, value)) => Ok(serde_json::json!({
            "error": value,
            "status": status.as_u16(),
        }).to_string()),
    }
});

#[derive(Debug, Clone)]
struct JsonSqlParam(serde_json::Value);

impl ToSql for JsonSqlParam {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let output = match &self.0 {
            serde_json::Value::Null => ToSqlOutput::Owned(rusqlite::types::Value::Null),
            serde_json::Value::Bool(value) => {
                ToSqlOutput::Owned(rusqlite::types::Value::Integer(if *value { 1 } else { 0 }))
            }
            serde_json::Value::Number(number) => {
                if let Some(value) = number.as_i64() {
                    ToSqlOutput::Owned(rusqlite::types::Value::Integer(value))
                } else if let Some(value) = number.as_u64() {
                    match i64::try_from(value) {
                        Ok(integer) => ToSqlOutput::Owned(rusqlite::types::Value::Integer(integer)),
                        Err(_) => {
                            ToSqlOutput::Owned(rusqlite::types::Value::Text(value.to_string()))
                        }
                    }
                } else if let Some(value) = number.as_f64() {
                    ToSqlOutput::Owned(rusqlite::types::Value::Real(value))
                } else {
                    ToSqlOutput::Owned(rusqlite::types::Value::Text(number.to_string()))
                }
            }
            serde_json::Value::String(value) => {
                ToSqlOutput::Owned(rusqlite::types::Value::Text(value.clone()))
            }
            other => ToSqlOutput::Owned(rusqlite::types::Value::Text(other.to_string())),
        };
        Ok(output)
    }
}

fn open_sqlite_connection(path: &str) -> rusqlite::Result<Connection> {
    if let Some(parent) = Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
}

fn json_from_sql_value(value: ValueRef<'_>) -> serde_json::Value {
    match value {
        ValueRef::Null => serde_json::Value::Null,
        ValueRef::Integer(v) => serde_json::json!(v),
        ValueRef::Real(v) => serde_json::json!(v),
        ValueRef::Text(v) => serde_json::Value::String(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(v) => {
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(v))
        }
    }
}

host_fn!(pub host_sqlite_query(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Storage);
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let path = parsed["path"].as_str().unwrap_or("");
    let sql = parsed["sql"].as_str().unwrap_or("");
    let params: Vec<JsonSqlParam> = parsed["params"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(JsonSqlParam)
        .collect();
    let refs: Vec<&dyn ToSql> = params.iter().map(|item| item as &dyn ToSql).collect();

    let result = (|| -> rusqlite::Result<String> {
        let connection = open_sqlite_connection(path)?;
        let mut statement = connection.prepare(sql)?;
        let column_count = statement.column_count();
        let columns = statement
            .column_names()
            .into_iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let mut rows = statement.query(refs.as_slice())?;
        let mut result_rows = Vec::new();
        while let Some(row) = rows.next()? {
            let mut values = Vec::with_capacity(column_count);
            for index in 0..column_count {
                values.push(json_from_sql_value(row.get_ref(index)?));
            }
            result_rows.push(values);
        }
        Ok(serde_json::json!({ "columns": columns, "rows": result_rows }).to_string())
    })();

    match result {
        Ok(output) => Ok(output),
        Err(error) => Ok(format!(r#"{{"error":"{}"}}"#, error.to_string().replace('"', "\\\""))),
    }
});

host_fn!(pub host_sqlite_execute(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Storage);
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let path = parsed["path"].as_str().unwrap_or("");
    let sql = parsed["sql"].as_str().unwrap_or("");
    let params: Vec<JsonSqlParam> = parsed["params"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(JsonSqlParam)
        .collect();
    let refs: Vec<&dyn ToSql> = params.iter().map(|item| item as &dyn ToSql).collect();

    let result = (|| -> rusqlite::Result<String> {
        let connection = open_sqlite_connection(path)?;
        let affected = connection.execute(sql, refs.as_slice())?;
        Ok(serde_json::json!({ "rows_affected": affected }).to_string())
    })();

    match result {
        Ok(output) => Ok(output),
        Err(error) => Ok(format!(r#"{{"error":"{}"}}"#, error.to_string().replace('"', "\\\""))),
    }
});

host_fn!(pub host_sqlite_batch(user_data: WasmHostState; input: String) -> String {
    gate_permission!(user_data, Permission::Storage);
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let path = parsed["path"].as_str().unwrap_or("");
    let statements = parsed["statements"].as_array().cloned().unwrap_or_default();

    let result = (|| -> rusqlite::Result<String> {
        let mut connection = open_sqlite_connection(path)?;
        let transaction = connection.transaction()?;
        let mut total_rows = 0_u64;
        for statement in statements {
            let sql = statement["sql"].as_str().unwrap_or("");
            let params: Vec<JsonSqlParam> = statement["params"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(JsonSqlParam)
                .collect();
            let refs: Vec<&dyn ToSql> = params.iter().map(|item| item as &dyn ToSql).collect();
            total_rows += transaction.execute(sql, refs.as_slice())? as u64;
        }
        transaction.commit()?;
        Ok(serde_json::json!({ "rows_affected": total_rows }).to_string())
    })();

    match result {
        Ok(output) => Ok(output),
        Err(error) => Ok(format!(r#"{{"error":"{}"}}"#, error.to_string().replace('"', "\\\""))),
    }
});

// ── WasmPackageHost ──

/// Info needed to load a WASM package.
#[derive(Debug, Clone)]
pub struct PackageLoadInfo {
    pub name: String,
    pub dir: PathBuf,
    pub wasm_path: PathBuf,
    pub startup_mode: WasmStartupMode,
    pub permissions: PackagePermissions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmStartupMode {
    Persistent,
    OnDemand,
}

/// Manages loaded Extism WASM packages.
pub struct WasmPackageHost {
    /// Shared package map — same Arc is held by WasmHostState for cross-package calls.
    package_map: PackageMap,
    load_infos: Arc<StdMutex<HashMap<String, PackageLoadInfo>>>,
    /// Base host state (without caller_package_name set).
    base_state: WasmHostState,
    /// Per-package circuit breaker for fault isolation (A2). Shared (Arc inside)
    /// so both WasmPackageHost::call and WasmHandle::call enforce the same state.
    breaker: CircuitBreaker,
}

impl WasmPackageHost {
    /// Load all WASM packages with shared host state.
    pub fn new(load_infos: &[PackageLoadInfo], host_state: WasmHostState) -> Self {
        let package_map: PackageMap = Arc::new(StdMutex::new(HashMap::new()));
        let load_infos_map = Arc::new(StdMutex::new(
            load_infos
                .iter()
                .cloned()
                .map(|info| (info.name.clone(), info))
                .collect::<HashMap<_, _>>(),
        ));

        // Wire the package_map into the base state so host functions can access it
        let base_state = WasmHostState {
            package_map: package_map.clone(),
            ..host_state
        };

        for info in load_infos {
            match Self::build_plugin(info, &base_state) {
                Ok(package) => {
                    tracing::info!(
                        "Loaded WASM package '{}' from {}",
                        info.name,
                        info.wasm_path.display()
                    );
                    package_map
                        .lock()
                        .unwrap()
                        .insert(info.name.clone(), Arc::new(StdMutex::new(package)));
                }
                Err(e) => {
                    tracing::error!("Failed to load WASM package '{}': {}", info.name, e);
                }
            }
        }

        Self {
            package_map,
            load_infos: load_infos_map,
            base_state,
            breaker: CircuitBreaker::default(),
        }
    }

    fn build_user_data(info: &PackageLoadInfo, base_state: &WasmHostState) -> UserData<WasmHostState> {
        let per_plugin_state = WasmHostState {
            caller_package_name: info.name.clone(),
            package_dir: normalize_package_dir(&info.dir),
            permissions: info.permissions.clone(),
            ..base_state.clone()
        };
        UserData::new(per_plugin_state)
    }

    fn build_plugin(info: &PackageLoadInfo, base_state: &WasmHostState) -> Result<Plugin> {
        let user_data = Self::build_user_data(info, base_state);
        Self::load_one(&info.wasm_path, &info.name, user_data)
    }

    fn isolated_load_info(&self, package_name: &str) -> Result<PackageLoadInfo> {
        self.load_infos
            .lock()
            .map_err(|e| anyhow::anyhow!("Package load info lock poisoned: {}", e))?
            .get(package_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not loaded", package_name))
    }

    pub fn call_isolated(&self, package_name: &str, func: &str, input: &str) -> Result<String> {
        if let Err(reason) = self.breaker.check(package_name) {
            anyhow::bail!("{reason}");
        }

        let info = self.isolated_load_info(package_name)?;
        let mut package = Self::build_plugin(&info, &self.base_state)?;
        let outcome = package.call::<&str, &str>(func, input);
        self.breaker.record(package_name, outcome.is_ok());
        let result = outcome.map_err(|e| {
            anyhow::anyhow!("Package '{}' isolated call '{}' failed: {}", package_name, func, e)
        })?;
        Ok(result.to_string())
    }

    fn load_one(
        wasm_path: &Path,
        name: &str,
        user_data: UserData<WasmHostState>,
    ) -> Result<Plugin> {
        let wasm = Wasm::file(wasm_path);
        let manifest = Manifest::new([wasm]).with_allowed_hosts(["*".to_string()].into_iter());

        let package = PluginBuilder::new(manifest)
            .with_wasi(true)
            .with_function("host_log", [PTR], [PTR], user_data.clone(), host_log)
            .with_function("host_kv_get", [PTR], [PTR], user_data.clone(), host_kv_get)
            .with_function("host_kv_set", [PTR], [], user_data.clone(), host_kv_set)
            .with_function(
                "host_kv_list",
                [PTR],
                [PTR],
                user_data.clone(),
                host_kv_list,
            )
            .with_function(
                "host_kv_delete",
                [PTR],
                [],
                user_data.clone(),
                host_kv_delete,
            )
            .with_function(
                "host_env_get",
                [PTR],
                [PTR],
                user_data.clone(),
                host_env_get,
            )
            .with_function(
                "host_home_dir",
                [PTR],
                [PTR],
                user_data.clone(),
                host_home_dir,
            )
            .with_function(
                "host_read_file",
                [PTR],
                [PTR],
                user_data.clone(),
                host_read_file,
            )
            .with_function(
                "host_write_file",
                [PTR],
                [],
                user_data.clone(),
                host_write_file,
            )
            .with_function(
                "host_list_dir",
                [PTR],
                [PTR],
                user_data.clone(),
                host_list_dir,
            )
            .with_function("host_exec", [PTR], [PTR], user_data.clone(), host_exec)
            .with_function(
                "host_exec_advanced",
                [PTR],
                [PTR],
                user_data.clone(),
                host_exec_advanced,
            )
            .with_function(
                "host_chat_completion",
                [PTR],
                [PTR],
                user_data.clone(),
                host_chat_completion,
            )
            .with_function(
                "host_http_request",
                [PTR],
                [PTR],
                user_data.clone(),
                host_http_request,
            )
            .with_function(
                "host_chat_completion_stream",
                [PTR],
                [PTR],
                user_data.clone(),
                host_chat_completion_stream,
            )
            .with_function("host_now_ms", [PTR], [PTR], user_data.clone(), host_now_ms)
            .with_function(
                "host_call_package",
                [PTR],
                [PTR],
                user_data.clone(),
                host_call_package,
            )
            .with_function(
                "host_call_package_ws",
                [PTR],
                [PTR],
                user_data.clone(),
                host_call_package_ws,
            )
            .with_function(
                "host_call_plugin",
                [PTR],
                [PTR],
                user_data.clone(),
                host_call_package,
            )
            .with_function(
                "host_capability_call",
                [PTR],
                [PTR],
                user_data.clone(),
                host_capability_call,
            )
            .with_function(
                "host_process_spawn",
                [PTR],
                [PTR],
                user_data.clone(),
                host_process_spawn,
            )
            .with_function(
                "host_process_stop",
                [PTR],
                [PTR],
                user_data.clone(),
                host_process_stop,
            )
            .with_function(
                "host_process_status",
                [PTR],
                [PTR],
                user_data.clone(),
                host_process_status,
            )
            .with_function(
                "host_process_write_stdin",
                [PTR],
                [PTR],
                user_data.clone(),
                host_process_write_stdin,
            )
            .with_function(
                "host_process_read_stdout",
                [PTR],
                [PTR],
                user_data.clone(),
                host_process_read_stdout,
            )
            .with_function(
                "host_sqlite_query",
                [PTR],
                [PTR],
                user_data.clone(),
                host_sqlite_query,
            )
            .with_function(
                "host_sqlite_execute",
                [PTR],
                [PTR],
                user_data.clone(),
                host_sqlite_execute,
            )
            .with_function(
                "host_sqlite_batch",
                [PTR],
                [PTR],
                user_data.clone(),
                host_sqlite_batch,
            )
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build package '{}': {}", name, e))?;

        Ok(package)
    }

    /// Dynamically load a single plugin. Calls init() after loading.
    pub fn load_package(&mut self, info: &PackageLoadInfo) -> Result<()> {
        {
            let map = self.package_map.lock().unwrap();
            if map.contains_key(&info.name) {
                anyhow::bail!("Package '{}' is already loaded", info.name);
            }
        }

        let user_data = Self::build_user_data(info, &self.base_state);
        let package = Self::load_one(&info.wasm_path, &info.name, user_data)?;
        self.package_map
            .lock()
            .unwrap()
            .insert(info.name.clone(), Arc::new(StdMutex::new(package)));
        tracing::info!("Hot-loaded WASM package '{}'", info.name);

        // Best-effort init
        if let Err(e) = self.call(&info.name, "init", "") {
            tracing::warn!(
                "Package '{}' init() failed (may not export it): {}",
                info.name,
                e
            );
        }
        Ok(())
    }

    /// Unload a package by name. The WASM resources are freed on drop.
    pub fn unload_package(&mut self, name: &str) -> Result<()> {
        self.package_map
            .lock()
            .unwrap()
            .remove(name)
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not loaded", name))?;
        // A2: a reloaded/replaced package deserves a fresh breaker.
        self.breaker.reset(name);
        tracing::info!("Unloaded WASM package '{}'", name);
        Ok(())
    }

    /// Reload a plugin: unload then load from the same path.
    pub fn reload_package(&mut self, info: &PackageLoadInfo) -> Result<()> {
        // Unload if present (ignore error if not loaded)
        let _ = self.unload_package(&info.name);
        // Load fresh
        let user_data = Self::build_user_data(info, &self.base_state);
        let package = Self::load_one(&info.wasm_path, &info.name, user_data)?;
        self.package_map
            .lock()
            .unwrap()
            .insert(info.name.clone(), Arc::new(StdMutex::new(package)));
        tracing::info!("Reloaded WASM package '{}'", info.name);

        if let Err(e) = self.call(&info.name, "init", "") {
            tracing::warn!(
                "Package '{}' init() failed (may not export it): {}",
                info.name,
                e
            );
        }
        Ok(())
    }

    /// Call an exported function on a named plugin.
    pub fn call(&self, package_name: &str, func: &str, input: &str) -> Result<String> {
        // A2: reject calls to a tripped package without touching the plugin.
        if let Err(reason) = self.breaker.check(package_name) {
            anyhow::bail!("{reason}");
        }
        let plugin_arc = {
            let map = self.package_map.lock().unwrap();
            map.get(package_name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Package '{}' not loaded", package_name))?
        };
        let mut package = plugin_arc
            .lock()
            .map_err(|e| anyhow::anyhow!("Package '{}' lock poisoned: {}", package_name, e))?;
        let outcome = package.call::<&str, &str>(func, input);
        // Record host-side failure (trap/panic/instantiation error), not guest
        // business errors carried inside an Ok JSON payload.
        self.breaker.record(package_name, outcome.is_ok());
        let result = outcome.map_err(|e| {
            anyhow::anyhow!("Package '{}' call '{}' failed: {}", package_name, func, e)
        })?;
        Ok(result.to_string())
    }

    /// Check if a package is loaded.
    pub fn has_package(&self, name: &str) -> bool {
        self.package_map.lock().unwrap().contains_key(name)
    }

    /// List loaded package names.
    pub fn package_names(&self) -> Vec<String> {
        self.package_map.lock().unwrap().keys().cloned().collect()
    }

    pub fn set_app_state(&mut self, app_state: crate::api::openai_compat::AppState) {
        if let Ok(mut state) = self.base_state.app_state.lock() {
            *state = Some(app_state);
        }
    }
}

// ── Handle wrapper for async context ──

/// Thread-safe handle to WasmPackageHost for use in async Axum handlers.
#[derive(Clone)]
pub struct WasmHandle {
    host: Arc<StdMutex<WasmPackageHost>>,
}

impl WasmHandle {
    pub fn new(host: WasmPackageHost) -> Self {
        Self {
            host: Arc::new(StdMutex::new(host)),
        }
    }

    /// Call a package function. Safe to call from async context.
    pub fn call(&self, package_name: &str, func: &str, input: &str) -> Result<String> {
        // Grab plugin_arc and a breaker handle under one short host lock (A2).
        let (plugin_arc, breaker) = {
            let host = self
                .host
                .lock()
                .map_err(|e| anyhow::anyhow!("WasmHandle lock poisoned: {}", e))?;
            let breaker = host.breaker.clone();
            let map = host.package_map.lock().unwrap();
            let arc = map
                .get(package_name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Package '{}' not loaded", package_name))?;
            (arc, breaker)
        };

        // Reject calls to a tripped package without touching the plugin.
        if let Err(reason) = breaker.check(package_name) {
            anyhow::bail!("{reason}");
        }

        let mut package = plugin_arc
            .lock()
            .map_err(|e| anyhow::anyhow!("Package '{}' lock poisoned: {}", package_name, e))?;
        let outcome = package.call::<&str, &str>(func, input);
        breaker.record(package_name, outcome.is_ok());
        let result = outcome.map_err(|e| {
            anyhow::anyhow!("Package '{}' call '{}' failed: {}", package_name, func, e)
        })?;
        Ok(result.to_string())
    }

    pub fn call_isolated(&self, package_name: &str, func: &str, input: &str) -> Result<String> {
        // 关键:只在极短临界区内从 host 克隆出隔离调用所需的字段(全是 Arc/可廉价克隆),
        // 然后释放 host 总锁,再创建临时实例执行。绝不能在整个 isolated 调用期间持有
        // host 锁——否则被调 agent 经 host_capability_call → dispatch_package_payload →
        // handle.call 需要同一把 host 锁时会自锁,且并发 fan-out 全被串行阻塞。
        let (load_infos, base_state, breaker) = {
            let host = self
                .host
                .lock()
                .map_err(|e| anyhow::anyhow!("WasmHandle lock poisoned: {}", e))?;
            (
                host.load_infos.clone(),
                host.base_state.clone(),
                host.breaker.clone(),
            )
        };

        if let Err(reason) = breaker.check(package_name) {
            anyhow::bail!("{reason}");
        }
        let info = load_infos
            .lock()
            .map_err(|e| anyhow::anyhow!("Package load info lock poisoned: {}", e))?
            .get(package_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not loaded", package_name))?;
        let mut package = WasmPackageHost::build_plugin(&info, &base_state)?;
        let outcome = package.call::<&str, &str>(func, input);
        breaker.record(package_name, outcome.is_ok());
        let result = outcome.map_err(|e| {
            anyhow::anyhow!("Package '{}' isolated call '{}' failed: {}", package_name, func, e)
        })?;
        Ok(result.to_string())
    }

    pub fn has_package(&self, name: &str) -> bool {
        self.host
            .lock()
            .map(|h| h.has_package(name))
            .unwrap_or(false)
    }

    pub fn package_names(&self) -> Vec<String> {
        self.host
            .lock()
            .map(|h| h.package_names())
            .unwrap_or_default()
    }

    pub fn set_app_state(&self, app_state: crate::api::openai_compat::AppState) -> Result<()> {
        let mut host = self
            .host
            .lock()
            .map_err(|e| anyhow::anyhow!("WasmHandle lock poisoned: {}", e))?;
        host.set_app_state(app_state);
        Ok(())
    }

    /// Dynamically load a new plugin.
    pub fn load_package(&self, info: &PackageLoadInfo) -> Result<()> {
        let mut host = self
            .host
            .lock()
            .map_err(|e| anyhow::anyhow!("WasmHandle lock poisoned: {}", e))?;
        host.load_package(info)
    }

    /// Unload a package by name.
    pub fn unload_package(&self, name: &str) -> Result<()> {
        let mut host = self
            .host
            .lock()
            .map_err(|e| anyhow::anyhow!("WasmHandle lock poisoned: {}", e))?;
        host.unload_package(name)
    }

    /// Reload a package (unload + load).
    pub fn reload_package(&self, info: &PackageLoadInfo) -> Result<()> {
        let mut host = self
            .host
            .lock()
            .map_err(|e| anyhow::anyhow!("WasmHandle lock poisoned: {}", e))?;
        host.reload_package(info)
    }
}

// ── Pipeline bridge adapters ──

/// Calls package-exported route() via WASM.
pub struct WasmRouterBridge {
    package_name: String,
    handle: WasmHandle,
}

impl WasmRouterBridge {
    pub fn new(package_name: &str, handle: WasmHandle) -> Self {
        Self {
            package_name: package_name.to_string(),
            handle,
        }
    }
}

#[async_trait]
impl RouterLayer for WasmRouterBridge {
    async fn route(&self, request: &ChatRequest, providers: &[ProviderConfig]) -> Result<String> {
        let input = serde_json::json!({
            "request": request,
            "providers": providers.iter().map(|p| &p.name).collect::<Vec<_>>(),
        });
        let input_str = serde_json::to_string(&input)?;
        let result_str = self.handle.call(&self.package_name, "route", &input_str)?;
        let parsed: serde_json::Value = serde_json::from_str(&result_str)?;
        parsed["provider"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("WASM route() missing provider field"))
    }
}

/// Calls package-exported select_key() via WASM.
pub struct WasmKeySelectorBridge {
    package_name: String,
    handle: WasmHandle,
}

impl WasmKeySelectorBridge {
    pub fn new(package_name: &str, handle: WasmHandle) -> Self {
        Self {
            package_name: package_name.to_string(),
            handle,
        }
    }
}

#[async_trait]
impl KeySelectorLayer for WasmKeySelectorBridge {
    async fn select(&self, provider: &str, keys: &[ApiKeyState]) -> Result<usize> {
        let input = serde_json::json!({
            "provider": provider,
            "key_count": keys.len(),
            "failed": keys.iter().map(|k| k.failed).collect::<Vec<_>>(),
        });
        let input_str = serde_json::to_string(&input)?;
        let result_str = self
            .handle
            .call(&self.package_name, "select_key", &input_str)?;
        let parsed: serde_json::Value = serde_json::from_str(&result_str)?;
        parsed["key_index"]
            .as_u64()
            .map(|i| i as usize)
            .ok_or_else(|| anyhow::anyhow!("WASM select_key() missing key_index field"))
    }

    fn mark_failed(&self, _provider: &str, _index: usize) {}
    fn mark_success(&self, _provider: &str, _index: usize) {}
}

/// Calls package-exported handle_error() via WASM.
pub struct WasmErrorHandlerBridge {
    package_name: String,
    handle: WasmHandle,
}

impl WasmErrorHandlerBridge {
    pub fn new(package_name: &str, handle: WasmHandle) -> Self {
        Self {
            package_name: package_name.to_string(),
            handle,
        }
    }
}

#[async_trait]
impl ErrorHandlerLayer for WasmErrorHandlerBridge {
    async fn handle(&self, error: &RequestError) -> ErrorAction {
        let input = serde_json::json!({
            "status": error.status,
            "message": error.message,
            "provider": error.provider,
            "retry_count": error.retry_count,
        });
        let input_str = match serde_json::to_string(&input) {
            Ok(s) => s,
            Err(_) => {
                return ErrorAction::Fail {
                    message: error.message.clone(),
                }
            }
        };

        match self
            .handle
            .call(&self.package_name, "handle_error", &input_str)
        {
            Ok(result_str) => {
                let parsed: serde_json::Value =
                    serde_json::from_str(&result_str).unwrap_or_default();
                match parsed["action"].as_str() {
                    Some("retry") => ErrorAction::Retry {
                        delay_ms: parsed["delay_ms"].as_u64().unwrap_or(1000),
                    },
                    Some("switch_key") => ErrorAction::SwitchKey,
                    Some("switch_provider") => ErrorAction::SwitchProvider,
                    _ => ErrorAction::Fail {
                        message: parsed["message"]
                            .as_str()
                            .unwrap_or(&error.message)
                            .to_string(),
                    },
                }
            }
            Err(e) => {
                tracing::error!("WASM handle_error() failed: {}", e);
                ErrorAction::Fail {
                    message: error.message.clone(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_loaded_package_name_from_aliases;
    use std::collections::HashMap;

    #[test]
    fn resolve_loaded_package_name_from_aliases_prefers_loaded_alias_target() {
        let aliases = HashMap::from([
            ("agent-core".to_string(), "agent-runtime".to_string()),
            ("skills".to_string(), "skills-runtime".to_string()),
        ]);
        let loaded_plugins = vec!["agent-runtime".to_string(), "memory-runtime".to_string()];

        assert_eq!(
            resolve_loaded_package_name_from_aliases(&aliases, &loaded_plugins, "agent-core"),
            "agent-runtime"
        );
        assert_eq!(
            resolve_loaded_package_name_from_aliases(&aliases, &loaded_plugins, "skills"),
            "skills"
        );
        assert_eq!(
            resolve_loaded_package_name_from_aliases(&aliases, &loaded_plugins, "memory-runtime"),
            "memory-runtime"
        );
    }
}
