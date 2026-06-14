use weft_package_sdk::*;
use serde_json::Value;

const PACKAGE_NAME: &str = "tool-web";
const CAPABILITY_NAME: &str = "tool.web";
const JS_RUNTIME_PACKAGE: &str = "js-extension-runtime";

/// Unwrap the js-extension-runtime response envelope, mirroring agent-core logic.
fn unwrap_js_response(raw: &str) -> Result<Value, String> {
    let payload: Value =
        serde_json::from_str(raw).map_err(|e| format!("invalid json from js-runtime: {}", e))?;

    // Direct ok
    if payload.get("status").and_then(Value::as_str) == Some("ok") {
        return Ok(payload.get("data").cloned().unwrap_or(payload));
    }
    // {ok: true, result: {status: "ok", ...}}
    if payload.get("ok").and_then(Value::as_bool) == Some(true) {
        if let Some(result) = payload.get("result") {
            if result.get("status").and_then(Value::as_str) == Some("ok") {
                return Ok(result.get("data").cloned().unwrap_or(result.clone()));
            }
        }
    }
    // {status: "executed", response: {status: "ok", ...}}
    if payload.get("status").and_then(Value::as_str) == Some("executed") {
        if let Some(response) = payload.get("response") {
            if response.get("status").and_then(Value::as_str) == Some("ok") {
                return Ok(response.get("data").cloned().unwrap_or(response.clone()));
            }
            if response.get("ok").and_then(Value::as_bool) == Some(true) {
                if let Some(result) = response.get("result") {
                    if result.get("status").and_then(Value::as_str) == Some("ok") {
                        return Ok(result.get("data").cloned().unwrap_or(result.clone()));
                    }
                }
            }
        }
    }

    let err = payload
        .get("error")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("response")
                .and_then(|r| r.get("error"))
                .and_then(Value::as_str)
        })
        .unwrap_or("js-runtime returned non-ok response");
    Err(err.to_string())
}

fn call_js_runtime(action: &str, payload: &Value) -> PackageResult {
    match call_package(
        JS_RUNTIME_PACKAGE,
        "handle_ws_message",
        &payload.to_string(),
    ) {
        Ok(raw) => match unwrap_js_response(&raw) {
            Ok(data) => PackageResult::ok(data),
            Err(e) => PackageResult::err(e),
        },
        Err(e) => PackageResult::err(format!("js-runtime unavailable ({}): {}", action, e)),
    }
}

fn do_web_fetch(data: &Value) -> PackageResult {
    let url = data.get("url").and_then(Value::as_str).unwrap_or("").trim();
    if url.is_empty() {
        return PackageResult::err("web_fetch requires a url argument");
    }
    let method = data
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET")
        .trim()
        .to_uppercase();
    let body = data.get("body").and_then(Value::as_str);
    let agent = data
        .get("agent")
        .and_then(Value::as_str)
        .unwrap_or("tool-web");

    let mut args = serde_json::json!({ "url": url, "method": method });
    if let Some(body_str) = body {
        args["body"] = Value::String(body_str.to_string());
    }

    let payload = serde_json::json!({
        "action": "fetch_url",
        "data": {
            "agent": agent,
            "tool": "fetch_url",
            "args": args,
            "query_b64": "",
        }
    });
    call_js_runtime("fetch_url", &payload)
}

fn do_web_search(data: &Value) -> PackageResult {
    let query = data
        .get("query")
        .or_else(|| data.get("q"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if query.is_empty() {
        return PackageResult::err("web_search requires a query argument");
    }
    let agent = data
        .get("agent")
        .and_then(Value::as_str)
        .unwrap_or("tool-web");

    let args = serde_json::json!({
        "query": query,
        "provider": "exa",
        "use_exa": true,
    });

    let payload = serde_json::json!({
        "action": "web_search",
        "data": {
            "agent": agent,
            "tool": "web_search",
            "args": args,
            "query_b64": "",
        }
    });
    call_js_runtime("web_search", &payload)
}

fn dispatch(action: &str, data: Value) -> PackageResult {
    match action {
        "describe" => PackageResult::ok(serde_json::json!({
            "package": PACKAGE_NAME,
            "capability": CAPABILITY_NAME,
            "runtime": "wasm",
            "actions": ["describe", "health", "web_fetch", "web_search"],
            "delegate": JS_RUNTIME_PACKAGE,
        })),
        "health" => PackageResult::ok(serde_json::json!({
            "healthy": true,
            "package": PACKAGE_NAME,
        })),
        "web_fetch" | "fetch" | "fetch_url" => do_web_fetch(&data),
        "web_search" | "search" => do_web_search(&data),
        other => PackageResult::err(format!("unknown action: {}", other)),
    }
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("tool-web initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });
    Ok(dispatch(&req.action, req.data).to_json())
}

#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });
    Ok(dispatch(&req.action, req.data).to_json())
}

#[plugin_fn]
pub fn describe(_input: String) -> FnResult<String> {
    Ok(dispatch("describe", serde_json::Value::Null).to_json())
}

#[plugin_fn]
pub fn health(_input: String) -> FnResult<String> {
    Ok(dispatch("health", serde_json::Value::Null).to_json())
}
