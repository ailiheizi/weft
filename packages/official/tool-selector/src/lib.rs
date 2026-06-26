//! tool-selector package — semantic selection engine via ONNX inference.
//!
//! Bridges a Python inference service (OnnxEncoder + UniversalSelector from
//! universal-selector) so the platform gains fast semantic matching from
//! candidate libraries. Use for tool routing, asset selection, model routing,
//! animation choreography, etc.
//!
//! Architecture: long-lived Python process per-agent (like tool-browser), JSON
//! line protocol over stdin/stdout. Process is spawned on first `select` call
//! and kept alive across calls. Explicit `stop_service` tears it down.

use weft_package_sdk::*;
use serde_json::Value;
use std::thread;
use std::time::{Duration, Instant};

const PACKAGE_NAME: &str = "tool-selector";
const CAPABILITY_NAME: &str = "tool.selector";

/// Python process can take a moment on first startup (loading ONNX model).
const INIT_TIMEOUT: Duration = Duration::from_secs(30);
/// Individual select calls should be fast (5-50ms) but allow some headroom.
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

// ── KV key helpers ──

fn process_name(agent: &str) -> String {
    format!("selector-svc-{}", agent)
}

fn offset_key(agent: &str) -> String {
    format!("selector:offset:{}", agent)
}

fn reqid_key(agent: &str) -> String {
    format!("selector:reqid:{}", agent)
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

fn next_req_id(agent: &str) -> u64 {
    let key = reqid_key(agent);
    let current = kv_get(&key).and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
    let next = current.saturating_add(1);
    kv_set(&key, &next.to_string());
    next
}

fn clear_req_id(agent: &str) {
    let _ = kv_delete(&reqid_key(agent));
}

// ── Process lifecycle ──

fn process_status_str(agent: &str) -> String {
    let raw = process_status(&process_name(agent)).unwrap_or_else(|_| r#"{"status":"unknown"}"#.into());
    serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|v| v.get("status").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| "unknown".into())
}

fn is_process_alive(agent: &str) -> bool {
    matches!(process_status_str(agent).as_str(), "running" | "starting")
}

/// Read all new stdout from the process since our last offset.
fn read_new_output(agent: &str) -> String {
    let offset = get_offset(agent);
    let read = match process_read_stdout(&process_name(agent), offset) {
        Ok(r) => r,
        Err(_) => return String::new(),
    };
    if read.chunk.is_empty() {
        return String::new();
    }
    set_offset(agent, read.next_offset);
    String::from_utf8(read.chunk).unwrap_or_default()
}

/// Send a JSON line request to the process.
fn send_request(agent: &str, id: u64, method: &str, params: Value) -> Result<(), String> {
    let request = serde_json::json!({
        "id": id,
        "method": method,
        "params": params,
    });
    let mut line = serde_json::to_string(&request)
        .map_err(|e| format!("failed to serialize request: {}", e))?;
    line.push('\n');
    process_write_stdin(&process_name(agent), &line)
        .map(|_| ())
        .map_err(|e| format!("failed to write to selector process: {}", e))
}

/// Poll stdout for a response with the given id, within timeout.
fn wait_for_response(agent: &str, id: u64, timeout: Duration) -> Result<Value, String> {
    let start = Instant::now();
    let mut buffer = String::new();

    loop {
        if start.elapsed() > timeout {
            return Err(format!(
                "timeout waiting for selector response (id={}, elapsed={:?})",
                id, start.elapsed()
            ));
        }

        let new_output = read_new_output(agent);
        if !new_output.is_empty() {
            buffer.push_str(&new_output);
            // Try to find our response line
            for line in buffer.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                    if parsed.get("id").and_then(Value::as_u64) == Some(id) {
                        if let Some(error) = parsed.get("error") {
                            return Err(format!("selector error: {}", error));
                        }
                        return Ok(parsed.get("result").cloned().unwrap_or(Value::Null));
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(20));
    }
}

/// Ensure the Python selector service is running for this agent.
fn ensure_service(agent: &str, libraries_dir: &str) -> Result<(), String> {
    if is_process_alive(agent) {
        return Ok(());
    }

    // Resolve service script path
    let package_dir = env_get("WEFT_PACKAGE_DIR").unwrap_or_else(|| ".".to_string());
    let service_script = format!("{}/service/server.py", package_dir);

    let spawn_args = serde_json::json!({
        "name": process_name(agent),
        "command": "python",
        "args": [&service_script, "--libraries-dir", libraries_dir],
        "env": {}
    });

    let result = process_spawn(
        &spawn_args.to_string(),
    ).map_err(|e| format!("failed to spawn selector service: {}", e))?;

    // Check for spawn error
    if let Ok(parsed) = serde_json::from_str::<Value>(&result) {
        if let Some(err) = parsed.get("error").and_then(Value::as_str) {
            if !err.is_empty() {
                return Err(format!("spawn error: {}", err));
            }
        }
    }

    // Clear offsets for fresh process
    clear_offset(agent);
    clear_req_id(agent);

    // Wait for ready signal
    let start = Instant::now();
    loop {
        if start.elapsed() > INIT_TIMEOUT {
            let _ = process_stop(&process_name(agent));
            return Err("selector service startup timeout".to_string());
        }
        let output = read_new_output(agent);
        if output.contains("\"ready\":true") || output.contains("READY") {
            return Ok(());
        }
        if !is_process_alive(agent) {
            return Err(format!("selector service died during startup. Output: {}", output));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn default_libraries_dir() -> String {
    let package_dir = env_get("WEFT_PACKAGE_DIR").unwrap_or_else(|| ".".to_string());
    format!("{}/libraries", package_dir)
}

// ── Actions ──

fn handle_select(agent: &str, args: &Value) -> String {
    let query = args.get("query").and_then(Value::as_str).unwrap_or("");
    let library = args.get("library").and_then(Value::as_str).unwrap_or("tools");
    let top_k = args.get("top_k").and_then(Value::as_u64).unwrap_or(3);

    if query.is_empty() {
        return PackageResult::err("'query' is required").to_json();
    }

    let libraries_dir = args.get("libraries_dir")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(default_libraries_dir);

    if let Err(e) = ensure_service(agent, &libraries_dir) {
        return PackageResult::err(&format!("failed to start selector service: {}", e)).to_json();
    }

    let id = next_req_id(agent);
    let params = serde_json::json!({
        "query": query,
        "library": library,
        "top_k": top_k,
    });

    if let Err(e) = send_request(agent, id, "select", params) {
        return PackageResult::err(&e).to_json();
    }

    match wait_for_response(agent, id, CALL_TIMEOUT) {
        Ok(result) => PackageResult::ok(result).to_json(),
        Err(e) => PackageResult::err(&e).to_json(),
    }
}

fn handle_select_multi(agent: &str, args: &Value) -> String {
    let queries = args.get("queries").cloned().unwrap_or(Value::Null);
    let top_k = args.get("top_k").and_then(Value::as_u64).unwrap_or(1);

    if !queries.is_object() {
        return PackageResult::err("'queries' must be an object {library_name: query_string}").to_json();
    }

    let libraries_dir = args.get("libraries_dir")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(default_libraries_dir);

    if let Err(e) = ensure_service(agent, &libraries_dir) {
        return PackageResult::err(&format!("failed to start selector service: {}", e)).to_json();
    }

    let id = next_req_id(agent);
    let params = serde_json::json!({
        "queries": queries,
        "top_k": top_k,
    });

    if let Err(e) = send_request(agent, id, "select_multi", params) {
        return PackageResult::err(&e).to_json();
    }

    match wait_for_response(agent, id, CALL_TIMEOUT) {
        Ok(result) => PackageResult::ok(result).to_json(),
        Err(e) => PackageResult::err(&e).to_json(),
    }
}

fn handle_list_libraries(agent: &str, args: &Value) -> String {
    let libraries_dir = args.get("libraries_dir")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(default_libraries_dir);

    if let Err(e) = ensure_service(agent, &libraries_dir) {
        return PackageResult::err(&format!("failed to start selector service: {}", e)).to_json();
    }

    let id = next_req_id(agent);
    if let Err(e) = send_request(agent, id, "list_libraries", serde_json::json!({})) {
        return PackageResult::err(&e).to_json();
    }

    match wait_for_response(agent, id, CALL_TIMEOUT) {
        Ok(result) => PackageResult::ok(result).to_json(),
        Err(e) => PackageResult::err(&e).to_json(),
    }
}

fn handle_build_library(agent: &str, args: &Value) -> String {
    let library_path = args.get("library_path").and_then(Value::as_str).unwrap_or("");
    if library_path.is_empty() {
        return PackageResult::err("'library_path' is required (path to directory with descriptions.jsonl)").to_json();
    }

    let libraries_dir = args.get("libraries_dir")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(default_libraries_dir);

    if let Err(e) = ensure_service(agent, &libraries_dir) {
        return PackageResult::err(&format!("failed to start selector service: {}", e)).to_json();
    }

    let id = next_req_id(agent);
    let params = serde_json::json!({
        "library_path": library_path,
    });

    if let Err(e) = send_request(agent, id, "build_library", params) {
        return PackageResult::err(&e).to_json();
    }

    // Building can take longer
    match wait_for_response(agent, id, INIT_TIMEOUT) {
        Ok(result) => PackageResult::ok(result).to_json(),
        Err(e) => PackageResult::err(&e).to_json(),
    }
}

fn handle_stop_service(agent: &str) -> String {
    if !is_process_alive(agent) {
        return PackageResult::ok(serde_json::json!({"status": "not_running"})).to_json();
    }
    let _ = process_stop(&process_name(agent));
    clear_offset(agent);
    clear_req_id(agent);
    PackageResult::ok(serde_json::json!({"status": "stopped"})).to_json()
}

// ── Entry point ──

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let envelope: Value = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("invalid JSON: {}", e)))?;

    let action = envelope.get("action").and_then(Value::as_str).unwrap_or("");
    let data = envelope.get("data").cloned().unwrap_or(Value::Object(Default::default()));
    let agent = data.get("agent").and_then(Value::as_str).unwrap_or("default");

    let result = match action {
        "select" => handle_select(agent, &data),
        "select_multi" => handle_select_multi(agent, &data),
        "list_libraries" => handle_list_libraries(agent, &data),
        "build_library" => handle_build_library(agent, &data),
        "stop_service" => handle_stop_service(agent),
        _ => PackageResult::err(&format!(
            "unknown action '{}'. Available: select, select_multi, list_libraries, build_library, stop_service",
            action
        )).to_json(),
    };

    Ok(result)
}
