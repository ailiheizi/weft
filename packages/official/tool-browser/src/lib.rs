//! tool-browser package — browser automation via chrome-devtools-mcp.
//!
//! Bridges the `chrome-devtools-mcp` MCP server (launched through `npx`) so the
//! platform gains browser automation (navigate / snapshot / click / fill /
//! screenshot / evaluate / ...). Mirrors Kimi Work's WebBridge.
//!
//! KEY DIFFERENCE vs mcp-client: mcp-client is *ephemeral* (spawns a fresh
//! process per tool call, then stops it). tool-browser keeps a **long-lived**
//! process per agent, because chrome-devtools-mcp page references (uid) and the
//! browser session must persist across calls. We never stop the process between
//! tool calls — only `stop_session` tears it down.

use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::thread;
use std::time::{Duration, Instant};

const PACKAGE_NAME: &str = "tool-browser";
const CAPABILITY_NAME: &str = "tool.browser";
const JSONRPC_VERSION: &str = "2.0";
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const DEFAULT_BROWSER_URL: &str = "http://127.0.0.1:9222";

/// initialize handshake can be slow (npx cold start + Chrome connect).
const INIT_TIMEOUT: Duration = Duration::from_secs(60);
/// tool calls (navigation / screenshot) can also be slow.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

// ── JSON-RPC framing types ──

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Value,
}

// ── KV key helpers ──

fn process_name(agent: &str) -> String {
    format!("browser-mcp-{}", agent)
}

fn offset_key(agent: &str) -> String {
    format!("browser:offset:{}", agent)
}

fn rpcid_key(agent: &str) -> String {
    format!("browser:rpcid:{}", agent)
}

fn get_offset(agent: &str) -> usize {
    kv_get(&offset_key(agent))
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

fn set_offset(agent: &str, offset: usize) {
    kv_set(&offset_key(agent), &offset.to_string());
}

fn clear_offset(agent: &str) {
    let _ = kv_delete(&offset_key(agent));
}

/// Incrementing JSON-RPC id, persisted in KV so it keeps growing across separate
/// WASM instance invocations against the same long-lived process. Fixes the
/// mcp-client limitation of a hard-coded `id: 1`, which would be ambiguous for a
/// process that receives many sequential requests.
fn next_rpc_id(agent: &str) -> u64 {
    let key = rpcid_key(agent);
    let current = kv_get(&key).and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
    let next = current.saturating_add(1);
    kv_set(&key, &next.to_string());
    next
}

fn clear_rpc_id(agent: &str) {
    let _ = kv_delete(&rpcid_key(agent));
}

// ── stdio frame parsing (ported verbatim from mcp-client) ──

fn build_jsonrpc_request(id: u64, method: &str, params: Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id,
        method: method.to_string(),
        params,
    }
}

fn build_jsonrpc_notification(method: &str, params: Value) -> Value {
    serde_json::json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": method,
        "params": params,
    })
}

fn encode_stdio_message(value: &Value) -> Result<String, String> {
    let mut body = serde_json::to_string(value)
        .map_err(|error| format!("failed to serialize MCP stdio payload: {}", error))?;
    body.push('\n');
    Ok(body)
}

fn try_parse_stdio_content_length_frame(buffer: &mut Vec<u8>) -> Result<Option<Value>, String> {
    let Some(header_end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Ok(None);
    };

    let header_bytes = &buffer[..header_end];
    let header_text = std::str::from_utf8(header_bytes)
        .map_err(|error| format!("invalid MCP stdio header encoding: {}", error))?;
    let mut content_length: Option<usize> = None;
    for line in header_text.split("\r\n") {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, ':');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.eq_ignore_ascii_case("Content-Length") {
            content_length = Some(
                value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid MCP Content-Length '{}': {}", value, error))?,
            );
        }
    }

    let content_length = content_length.ok_or_else(|| "missing MCP Content-Length header".to_string())?;
    let body_start = header_end + 4;
    if buffer.len() < body_start + content_length {
        return Ok(None);
    }

    let body = buffer[body_start..body_start + content_length].to_vec();
    buffer.drain(0..body_start + content_length);
    let response = serde_json::from_slice::<Value>(&body)
        .map_err(|error| format!("invalid MCP stdio JSON body: {}", error))?;
    Ok(Some(response))
}

fn try_parse_stdio_line(buffer: &mut Vec<u8>) -> Result<Option<Value>, String> {
    let Some(line_end) = buffer.iter().position(|byte| *byte == b'\n') else {
        return Ok(None);
    };

    let line = buffer[..line_end].to_vec();
    buffer.drain(0..=line_end);
    let trimmed = String::from_utf8(line)
        .map_err(|error| format!("invalid MCP stdio line encoding: {}", error))?
        .trim()
        .to_string();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let response = serde_json::from_str::<Value>(&trimmed)
        .map_err(|error| format!("invalid MCP stdio JSON line: {}", error))?;
    Ok(Some(response))
}

fn try_parse_stdio_message(buffer: &mut Vec<u8>) -> Result<Option<Value>, String> {
    if buffer.starts_with(b"Content-Length:") || buffer.starts_with(b"content-length:") {
        return try_parse_stdio_content_length_frame(buffer);
    }
    try_parse_stdio_line(buffer)
}

// ── process lifecycle helpers ──

/// Parse the `{"status":"..."}` JSON returned by `process_status`.
fn process_status_str(agent: &str) -> String {
    let raw = process_status(&process_name(agent)).unwrap_or_else(|_| r#"{"status":"unknown"}"#.into());
    serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|v| v.get("status").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| "unknown".into())
}

/// Whether the long-lived process currently exists and is (re)starting/running.
fn is_process_alive(agent: &str) -> bool {
    matches!(process_status_str(agent).as_str(), "running" | "starting")
}

/// Detect a spawn-host error returned as `{"error":"..."}` in the response body.
fn spawn_error(raw: &str) -> Option<String> {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|v| v.get("error").and_then(Value::as_str).map(String::from))
        .filter(|e| !e.trim().is_empty())
}

/// Resolve the chrome-devtools-mcp launch args from request data.
///
/// - `data.auto_connect == true` → `--autoConnect` (let chrome-devtools-mcp launch
///   / discover Chrome itself, no browser-url).
/// - otherwise `--browser-url=<data.browser_url || DEFAULT_BROWSER_URL>` to attach
///   to a user's already-running Chrome with remote debugging (reuses login state).
fn build_mcp_args(data: &Value) -> Vec<String> {
    let auto_connect = data.get("auto_connect").and_then(Value::as_bool).unwrap_or(false);
    if auto_connect {
        vec!["chrome-devtools-mcp@latest".into(), "--autoConnect".into()]
    } else {
        let browser_url = data
            .get("browser_url")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_BROWSER_URL);
        vec![
            "chrome-devtools-mcp@latest".into(),
            format!("--browser-url={}", browser_url),
        ]
    }
}

/// Start the long-lived session: spawn the process (if not already alive) with
/// `restart_on_crash: true`, run the initialize + notifications/initialized
/// handshake, and persist the post-handshake stdout offset for follow-up calls.
fn do_start_session(agent: &str, data: &Value) -> PackageResult {
    if agent.trim().is_empty() {
        return PackageResult::err("start_session requires an 'agent'");
    }

    // Already alive → idempotent no-op. Do NOT re-spawn or re-handshake; the
    // existing session (and its page uids) must be preserved.
    if is_process_alive(agent) {
        return PackageResult::ok(serde_json::json!({
            "agent": agent,
            "process": process_name(agent),
            "status": "already_running",
        }));
    }

    let name = process_name(agent);
    let env = data
        .get("env")
        .cloned()
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    let config = serde_json::json!({
        "name": name,
        "command": "npx",
        "args": build_mcp_args(data),
        "env": env,
        "restart_on_crash": true,
    });

    match process_spawn(&config.to_string()) {
        Ok(raw) => {
            if let Some(err) = spawn_error(&raw) {
                return PackageResult::err(format!("failed to spawn chrome-devtools-mcp: {}", err));
            }
        }
        Err(err) => return PackageResult::err(format!("failed to spawn chrome-devtools-mcp: {}", err)),
    }

    // Fresh spawn clears the host stdout buffer and resets its base offset to 0,
    // so our read offset starts at 0 too.
    let mut offset = 0usize;
    set_offset(agent, 0);
    clear_rpc_id(agent);

    let init_id = next_rpc_id(agent);
    let init = stdio_jsonrpc_exchange(
        agent,
        &build_jsonrpc_request(
            init_id,
            "initialize",
            serde_json::json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "weft-tool-browser", "version": "0.1.0"}
            }),
        ),
        &mut offset,
        INIT_TIMEOUT,
    );
    if let Err(error) = init {
        // Handshake failed — tear down so a later call can retry cleanly.
        let _ = process_stop(&name);
        clear_offset(agent);
        clear_rpc_id(agent);
        return PackageResult::err(format!("MCP initialize failed: {}", error));
    }

    if let Err(error) = stdio_write_only(
        agent,
        &build_jsonrpc_notification("notifications/initialized", serde_json::json!({})),
    ) {
        let _ = process_stop(&name);
        clear_offset(agent);
        clear_rpc_id(agent);
        return PackageResult::err(format!("MCP initialized notification failed: {}", error));
    }

    // Persist the offset reached after the handshake so subsequent tool calls
    // continue reading the same long-lived stdout stream.
    set_offset(agent, offset);

    PackageResult::ok(serde_json::json!({
        "agent": agent,
        "process": name,
        "status": "started",
    }))
}

/// Stop the long-lived session and clean up its KV bookkeeping.
fn do_stop_session(agent: &str) -> PackageResult {
    if agent.trim().is_empty() {
        return PackageResult::err("stop_session requires an 'agent'");
    }
    let name = process_name(agent);
    let stop_result = process_stop(&name);
    clear_offset(agent);
    clear_rpc_id(agent);
    match stop_result {
        Ok(_) => PackageResult::ok(serde_json::json!({
            "agent": agent,
            "process": name,
            "status": "stopped",
        })),
        Err(e) => PackageResult::err(format!("failed to stop session: {}", e)),
    }
}

/// Ensure a live session exists; auto-start one if the process isn't running.
fn ensure_session(agent: &str, data: &Value) -> Result<(), String> {
    if is_process_alive(agent) {
        return Ok(());
    }
    let started = do_start_session(agent, data);
    if started.status == "ok" {
        Ok(())
    } else {
        Err(started.error.unwrap_or_else(|| "failed to start browser session".into()))
    }
}

// ── generic MCP tool invocation over the long-lived process ──

/// Build the `arguments` object for a tools/call by picking the named keys that
/// are present (and non-null) in `data`, then merging any caller-supplied
/// `data.args` object on top (caller override / escape hatch for extra params).
fn collect_args(data: &Value, keys: &[&str]) -> Value {
    let mut obj = serde_json::Map::new();
    for key in keys {
        if let Some(value) = data.get(*key) {
            if !value.is_null() {
                obj.insert((*key).to_string(), value.clone());
            }
        }
    }
    if let Some(extra) = data.get("args").and_then(Value::as_object) {
        for (k, v) in extra {
            obj.insert(k.clone(), v.clone());
        }
    }
    Value::Object(obj)
}

/// Core helper: ensure session, then run a tools/call against the long-lived
/// process. Never stops the process. Reads the stored offset, advances it across
/// the exchange, and writes it back.
fn call_mcp_tool(agent: &str, data: &Value, tool_name: &str, arguments: Value) -> PackageResult {
    if agent.trim().is_empty() {
        return PackageResult::err("missing 'agent'");
    }
    if let Err(error) = ensure_session(agent, data) {
        return PackageResult::err(error);
    }

    let id = next_rpc_id(agent);
    let mut offset = get_offset(agent);
    let result = stdio_jsonrpc_exchange(
        agent,
        &build_jsonrpc_request(
            id,
            "tools/call",
            serde_json::json!({
                "name": tool_name,
                "arguments": arguments,
            }),
        ),
        &mut offset,
        CALL_TIMEOUT,
    );
    // Persist the advanced offset regardless of outcome so we never re-read stale
    // frames on the next call.
    set_offset(agent, offset);

    match result {
        Ok(output) => PackageResult::ok(serde_json::json!({
            "agent": agent,
            "tool": tool_name,
            "output": output,
        })),
        Err(error) => PackageResult::err(error),
    }
}

fn stdio_write_only(agent: &str, request: &Value) -> Result<(), String> {
    let payload = encode_stdio_message(request)?;
    process_write_stdin(&process_name(agent), &payload)?;
    Ok(())
}

/// Send a JSON-RPC request to the long-lived process and read until the response
/// matching this request `id` arrives. Notifications and frames with a different
/// id are skipped (important for a long-lived process that may emit log
/// notifications between request/response pairs).
fn stdio_jsonrpc_exchange(
    agent: &str,
    request: &JsonRpcRequest,
    offset: &mut usize,
    timeout: Duration,
) -> Result<Value, String> {
    let name = process_name(agent);
    let want_id = request.id;
    let request_value = serde_json::to_value(request)
        .map_err(|error| format!("failed to convert MCP request to JSON value: {}", error))?;
    let payload = encode_stdio_message(&request_value)?;
    process_write_stdin(&name, &payload)?;

    let deadline = Instant::now() + timeout;
    let mut pending = Vec::<u8>::new();
    while Instant::now() < deadline {
        let read = process_read_stdout(&name, *offset)?;
        *offset = read.next_offset;
        pending.extend_from_slice(&read.chunk);
        loop {
            let Some(frame) = try_parse_stdio_message(&mut pending)? else {
                break;
            };
            // Skip notifications (no id) and responses to other requests.
            let frame_id = frame.get("id").and_then(Value::as_u64);
            if frame_id != Some(want_id) {
                continue;
            }
            if let Some(error) = frame.get("error") {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown MCP error");
                return Err(message.to_string());
            }
            if let Some(result) = frame.get("result") {
                return Ok(result.clone());
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    Err(format!("timeout waiting for MCP response (id={}) from {}", want_id, name))
}

// ── dispatch ──

fn agent_of(data: &Value) -> String {
    data.get("agent").and_then(Value::as_str).unwrap_or("default").to_string()
}

fn dispatch(action: &str, data: Value) -> PackageResult {
    let agent = agent_of(&data);
    match action {
        "describe" => PackageResult::ok(serde_json::json!({
            "package": PACKAGE_NAME,
            "capability": CAPABILITY_NAME,
            "runtime": "wasm",
            "model": "chrome-devtools-mcp (snapshot -> uid -> interact)",
            "actions": [
                "describe", "health", "start_session", "stop_session",
                "navigate", "new_page", "snapshot", "click", "fill",
                "type_text", "hover", "screenshot", "evaluate", "wait_for",
                "list_pages", "handle_dialog"
            ],
            "notes": "Call 'snapshot' before interacting to obtain element uids.",
        })),
        "health" => PackageResult::ok(serde_json::json!({
            "healthy": true,
            "package": PACKAGE_NAME,
            "agent": agent,
            "session": process_status_str(&agent),
        })),

        // ── session lifecycle ──
        "start_session" => do_start_session(&agent, &data),
        "stop_session" => do_stop_session(&agent),

        // ── page navigation ──
        "navigate" => call_mcp_tool(&agent, &data, "navigate_page", collect_args(&data, &["url"])),
        "new_page" => call_mcp_tool(&agent, &data, "new_page", collect_args(&data, &["url"])),
        "list_pages" => call_mcp_tool(&agent, &data, "list_pages", collect_args(&data, &[])),

        // ── inspection ──
        // take_snapshot returns the a11y tree WITH uids; must be called before
        // any uid-based interaction.
        "snapshot" => call_mcp_tool(&agent, &data, "take_snapshot", collect_args(&data, &[])),
        "screenshot" => call_mcp_tool(&agent, &data, "take_screenshot", collect_args(&data, &["uid", "fullPage", "format"])),

        // ── interaction (uid from a prior snapshot) ──
        "click" => call_mcp_tool(&agent, &data, "click", collect_args(&data, &["uid"])),
        "fill" => call_mcp_tool(&agent, &data, "fill", collect_args(&data, &["uid", "value"])),
        "type_text" => call_mcp_tool(&agent, &data, "type_text", collect_args(&data, &["uid", "text", "value"])),
        "hover" => call_mcp_tool(&agent, &data, "hover", collect_args(&data, &["uid"])),

        // ── scripting / waiting / dialogs ──
        "evaluate" => call_mcp_tool(&agent, &data, "evaluate_script", build_evaluate_args(&data)),
        "wait_for" => call_mcp_tool(&agent, &data, "wait_for", collect_args(&data, &["text"])),
        "handle_dialog" => call_mcp_tool(&agent, &data, "handle_dialog", collect_args(&data, &["action", "promptText"])),

        other => PackageResult::err(format!("unknown action: {}", other)),
    }
}

/// evaluate_script expects a `function` body. Accept either `data.function` or
/// `data.script` as the source, then merge any extra caller args.
fn build_evaluate_args(data: &Value) -> Value {
    let mut obj = serde_json::Map::new();
    let source = data
        .get("function")
        .or_else(|| data.get("script"))
        .and_then(Value::as_str);
    if let Some(src) = source {
        obj.insert("function".to_string(), Value::String(src.to_string()));
    }
    if let Some(extra) = data.get("args").and_then(Value::as_object) {
        for (k, v) in extra {
            obj.insert(k.clone(), v.clone());
        }
    }
    Value::Object(obj)
}

// ── plugin entrypoints (tool-web provider conventions) ──

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("tool-browser initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });
    Ok(dispatch(&req.action, req.data).to_json())
}

#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });
    Ok(dispatch(&req.action, req.data).to_json())
}

#[plugin_fn]
pub fn describe(_input: String) -> FnResult<String> {
    Ok(dispatch("describe", Value::Null).to_json())
}

#[plugin_fn]
pub fn health(input: String) -> FnResult<String> {
    // Allow {"agent": "..."} so health can report the per-agent session status.
    let data: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
    Ok(dispatch("health", data).to_json())
}
