//! WEFT Package SDK — shared types and host function wrappers for WASM packages.
//!
//! All WASM packages built for WEFT should depend on this crate.
//! It provides ergonomic wrappers around Extism host functions registered by weft-core.

pub use extism_pdk::{self, plugin_fn, FnResult, FromBytes, ToBytes};
pub use serde;
pub use serde_json;

use base64::Engine;
use extism_pdk::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Common types ──

/// WebSocket message envelope (matches weft-core's PackageWsMessage.payload).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsRequest {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Standard package response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PackageResult {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            status: "ok".into(),
            data: Some(data),
            error: None,
        }
    }

    pub fn ok_empty() -> Self {
        Self {
            status: "ok".into(),
            data: None,
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            status: "error".into(),
            data: None,
            error: Some(msg.into()),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| r#"{"status":"error","error":"serialize failed"}"#.into())
    }
}

// ── Host function imports ──
// These match the host functions registered in weft-core bridge.rs.

#[host_fn]
extern "ExtismHost" {
    pub fn host_log(input: String) -> String;
    pub fn host_kv_get(key: String) -> String;
    pub fn host_kv_set(input: String);
    pub fn host_kv_list(prefix: String) -> String;
    pub fn host_kv_delete(key: String);
    pub fn host_env_get(key: String) -> String;
    pub fn host_read_file(path: String) -> String;
    pub fn host_write_file(input: String);
    pub fn host_list_dir(path: String) -> String;
    pub fn host_exec(input: String) -> String;
    pub fn host_exec_advanced(input: String) -> String;
    pub fn host_chat_completion(input: String) -> String;
    pub fn host_chat_completion_stream(input: String) -> String;
    pub fn host_http_request(input: String) -> String;
    pub fn host_call_package(input: String) -> String;
    pub fn host_call_package_ws(input: String) -> String;
    pub fn host_capability_call(input: String) -> String;
    pub fn host_process_spawn(config: String) -> String;
    pub fn host_process_stop(name: String) -> String;
    pub fn host_process_status(name: String) -> String;
    pub fn host_process_write_stdin(input: String) -> String;
    pub fn host_process_read_stdout(input: String) -> String;
    pub fn host_sqlite_query(input: String) -> String;
    pub fn host_sqlite_execute(input: String) -> String;
    pub fn host_sqlite_batch(input: String) -> String;
    pub fn host_now_ms(input: String) -> String;
}

// ── Ergonomic wrappers ──

/// Log a message at the given level.
pub fn log(level: &str, msg: &str) {
    let input = serde_json::json!([level, msg]).to_string();
    let _ = unsafe { host_log(input) };
}

pub fn log_info(msg: &str) {
    log("info", msg);
}
pub fn log_warn(msg: &str) {
    log("warn", msg);
}
pub fn log_error(msg: &str) {
    log("error", msg);
}
pub fn log_debug(msg: &str) {
    log("debug", msg);
}

/// Get a value from the KV store.
pub fn kv_get(key: &str) -> Option<String> {
    match unsafe { host_kv_get(key.to_string()) } {
        Ok(v) if v.is_empty() => None,
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

/// Set a value in the KV store.
pub fn kv_set(key: &str, value: &str) {
    let input = serde_json::json!([key, value]).to_string();
    let _ = unsafe { host_kv_set(input) };
}

pub fn kv_list(prefix: &str) -> Result<Vec<String>, String> {
    match unsafe { host_kv_list(prefix.to_string()) } {
        Ok(json) => parse_host_json(&json),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn kv_delete(key: &str) -> Result<(), String> {
    match unsafe { host_kv_delete(key.to_string()) } {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn env_get(key: &str) -> Option<String> {
    match unsafe { host_env_get(key.to_string()) } {
        Ok(value) if value.is_empty() => None,
        Ok(value) => Some(value),
        Err(_) => None,
    }
}

/// Read a file. Returns content or error JSON.
pub fn read_file(path: &str) -> Result<String, String> {
    match unsafe { host_read_file(path.to_string()) } {
        Ok(content) => {
            if content.starts_with(r#"{"error":"#) {
                Err(content)
            } else {
                Ok(content)
            }
        }
        Err(e) => Err(format!("{}", e)),
    }
}

/// Write a file.
pub fn write_file(path: &str, content: &str) {
    let input = serde_json::json!([path, content]).to_string();
    let _ = unsafe { host_write_file(input) };
}

/// List directory entries. Returns JSON array of {name, is_dir}.
pub fn list_dir(path: &str) -> Result<Vec<DirEntry>, String> {
    match unsafe { host_list_dir(path.to_string()) } {
        Ok(json) => parse_host_json(&json),
        Err(e) => Err(format!("{}", e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

/// Execute a shell command.
pub fn exec_command(cmd: &str, args: &[&str]) -> Result<ExecResult, String> {
    let input = serde_json::json!({
        "command": cmd,
        "args": args,
    })
    .to_string();
    match unsafe { host_exec(input) } {
        Ok(json) => parse_host_json(&json),
        Err(e) => Err(format!("{}", e)),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecAdvancedOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

pub fn exec_command_advanced_with_options(
    cmd: &str,
    args: &[&str],
    options: &ExecAdvancedOptions,
) -> Result<ExecResult, String> {
    let input = serde_json::json!({
        "command": cmd,
        "args": args,
        "stdin": options.stdin,
        "stdin_base64": options.stdin_base64,
        "workdir": options.workdir,
        "timeout_ms": options.timeout_ms,
        "env": options.env,
    })
    .to_string();

    match unsafe { host_exec_advanced(input) } {
        Ok(json) => parse_host_json(&json),
        Err(e) => Err(format!("{}", e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
    #[serde(default)]
    pub stdout_base64: Option<String>,
    #[serde(default)]
    pub stderr_base64: Option<String>,
}

impl ExecResult {
    pub fn stdout_text(&self) -> String {
        decode_exec_base64_text(&self.stdout_base64).unwrap_or_else(|| self.stdout.clone())
    }

    pub fn stderr_text(&self) -> String {
        decode_exec_base64_text(&self.stderr_base64).unwrap_or_else(|| self.stderr.clone())
    }
}

#[derive(Debug, Clone, Serialize)]
struct HostChatCompletionInput<'a> {
    request_label: &'a str,
    endpoint: &'a str,
    body: &'a str,
}

pub fn chat_completion(request_label: &str, endpoint: &str, body: &str) -> Result<String, String> {
    let input = serde_json::to_string(&HostChatCompletionInput {
        request_label,
        endpoint,
        body,
    })
    .map_err(|error| format!("failed to serialize host chat completion input: {}", error))?;

    match unsafe { host_chat_completion(input) } {
        Ok(result) => {
            if let Ok(error) = serde_json::from_str::<HostErrorResult>(&result) {
                if !error.error.trim().is_empty() {
                    return Err(error.error);
                }
            }
            Ok(result)
        }
        Err(e) => Err(format!("{}", e)),
    }
}

#[derive(Debug, Clone, Serialize)]
struct HostChatCompletionStreamInput<'a> {
    request_label: &'a str,
    body: &'a str,
    session_id: &'a str,
}

pub fn chat_completion_stream(
    request_label: &str,
    _endpoint: &str,
    body: &str,
    session_id: &str,
) -> Result<String, String> {
    let input = serde_json::to_string(&HostChatCompletionStreamInput {
        request_label,
        body,
        session_id,
    })
    .map_err(|e| format!("failed to serialize stream input: {}", e))?;

    match unsafe { host_chat_completion_stream(input) } {
        Ok(result) => {
            if let Ok(error) = serde_json::from_str::<HostErrorResult>(&result) {
                if !error.error.trim().is_empty() {
                    return Err(error.error);
                }
            }
            Ok(result)
        }
        Err(e) => Err(format!("{}", e)),
    }
}

/// Call another package exported function.
pub fn call_package(package: &str, func: &str, args: &str) -> Result<String, String> {
    let input = serde_json::json!({
        "package": package,
        "func": func,
        "args": args,
    })
    .to_string();
    match unsafe { host_call_package(input) } {
        Ok(result) => {
            if result.contains(r#""error""#) {
                Err(result)
            } else {
                Ok(result)
            }
        }
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn call_package_ws_action(
    package: &str,
    action: &str,
    data: &serde_json::Value,
) -> Result<String, String> {
    let input = serde_json::json!({
        "package": package,
        "action": action,
        "data": data,
    })
    .to_string();
    match unsafe { host_call_package_ws(input) } {
        Ok(result) => Ok(result),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn call_capability_action(
    capability: &str,
    action: &str,
    data: &serde_json::Value,
) -> Result<String, String> {
    let input = serde_json::json!({
        "capability": capability,
        "action": action,
        "data": data,
    })
    .to_string();
    match unsafe { host_capability_call(input) } {
        Ok(result) => Ok(result),
        Err(e) => Err(format!("{}", e)),
    }
}

/// Spawn a managed process.
pub fn process_spawn(config_json: &str) -> Result<String, String> {
    match unsafe { host_process_spawn(config_json.to_string()) } {
        Ok(r) => Ok(r),
        Err(e) => Err(format!("{}", e)),
    }
}

/// Stop a managed process.
pub fn process_stop(name: &str) -> Result<String, String> {
    match unsafe { host_process_stop(name.to_string()) } {
        Ok(r) => Ok(r),
        Err(e) => Err(format!("{}", e)),
    }
}

/// Get status of a managed process.
pub fn process_status(name: &str) -> Result<String, String> {
    match unsafe { host_process_status(name.to_string()) } {
        Ok(r) => Ok(r),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn process_write_stdin(name: &str, input: &str) -> Result<String, String> {
    let payload = serde_json::json!({
        "name": name,
        "input": input,
    })
    .to_string();
    match unsafe { host_process_write_stdin(payload) } {
        Ok(r) => Ok(r),
        Err(e) => Err(format!("{}", e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStdoutRead {
    pub status: String,
    pub name: String,
    pub next_offset: usize,
    pub chunk: Vec<u8>,
}

pub fn process_read_stdout(name: &str, offset: usize) -> Result<ProcessStdoutRead, String> {
    let payload = serde_json::json!({
        "name": name,
        "offset": offset,
    })
    .to_string();
    match unsafe { host_process_read_stdout(payload) } {
        Ok(json) => parse_host_json(&json),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn now_ms() -> u64 {
    unsafe { host_now_ms(String::new()) }
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteExecResult {
    pub rows_affected: u64,
}

pub fn sqlite_query(
    path: &str,
    sql: &str,
    params: &[serde_json::Value],
) -> Result<SqliteQueryResult, String> {
    let input = serde_json::json!({
        "path": path,
        "sql": sql,
        "params": params,
    })
    .to_string();
    match unsafe { host_sqlite_query(input) } {
        Ok(json) => parse_host_json(&json),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn sqlite_execute(
    path: &str,
    sql: &str,
    params: &[serde_json::Value],
) -> Result<SqliteExecResult, String> {
    let input = serde_json::json!({
        "path": path,
        "sql": sql,
        "params": params,
    })
    .to_string();
    match unsafe { host_sqlite_execute(input) } {
        Ok(json) => parse_host_json(&json),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn sqlite_batch(
    path: &str,
    statements: &[(String, Vec<serde_json::Value>)],
) -> Result<SqliteExecResult, String> {
    let payload: Vec<serde_json::Value> = statements
        .iter()
        .map(|(sql, params)| serde_json::json!({ "sql": sql, "params": params }))
        .collect();
    let input = serde_json::json!({
        "path": path,
        "statements": payload,
    })
    .to_string();
    match unsafe { host_sqlite_batch(input) } {
        Ok(json) => parse_host_json(&json),
        Err(e) => Err(format!("{}", e)),
    }
}

#[derive(Debug, Deserialize)]
struct HostErrorResult {
    error: String,
}

fn parse_host_json<T>(json: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    match serde_json::from_str::<T>(json) {
        Ok(value) => Ok(value),
        Err(parse_error) => {
            if let Ok(error) = serde_json::from_str::<HostErrorResult>(json) {
                if !error.error.is_empty() {
                    return Err(error.error);
                }
            }
            Err(format!("parse error: {}", parse_error))
        }
    }
}

fn decode_exec_base64_text(encoded: &Option<String>) -> Option<String> {
    encoded
        .as_deref()
        .and_then(|value| base64::engine::general_purpose::STANDARD.decode(value).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_host_json_returns_structured_value() {
        let parsed = parse_host_json::<ExecResult>(r#"{"status":0,"stdout":"ok","stderr":""}"#)
            .expect("expected exec result");

        assert_eq!(parsed.status, 0);
        assert_eq!(parsed.stdout, "ok");
        assert_eq!(parsed.stderr, "");
    }

    #[test]
    fn parse_host_json_surfaces_host_error_without_wrapper_parse_failure() {
        let err = parse_host_json::<ExecResult>(
            r#"{"error":"The system cannot find the path specified."}"#,
        )
        .expect_err("expected host error");

        assert_eq!(err, "The system cannot find the path specified.");
    }

    #[test]
    fn parse_host_json_accepts_optional_base64_exec_fields() {
        let parsed = parse_host_json::<ExecResult>(
            r#"{"status":0,"stdout":"fallback","stderr":"","stdout_base64":"aGVsbG8=","stderr_base64":null}"#,
        )
        .expect("expected exec result with base64 fields");

        assert_eq!(parsed.status, 0);
        assert_eq!(parsed.stdout, "fallback");
        assert_eq!(parsed.stdout_base64.as_deref(), Some("aGVsbG8="));
        assert_eq!(parsed.stdout_text(), "hello");
        assert_eq!(parsed.stderr_text(), "");
    }

    #[test]
    fn exec_text_helpers_fall_back_to_plain_text() {
        let parsed = ExecResult {
            status: 0,
            stdout: "plain stdout".into(),
            stderr: "plain stderr".into(),
            stdout_base64: None,
            stderr_base64: Some("%%%invalid%%%".into()),
        };

        assert_eq!(parsed.stdout_text(), "plain stdout");
        assert_eq!(parsed.stderr_text(), "plain stderr");
    }
}
