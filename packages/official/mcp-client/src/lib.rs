//! MCP-client package — MCP Server management and tool discovery.

use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const JSONRPC_VERSION: &str = "2.0";
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
static MCP_EPHEMERAL_PROCESS_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct McpToolDef {
    #[serde(default)]
    server: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "inputSchema", default = "default_input_schema")]
    input_schema: serde_json::Value,
}

#[derive(Serialize)]
struct ToolDiscoveryDiagnostic {
    server: String,
    transport: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    live_error: Option<String>,
    cached_tools: usize,
}

fn default_input_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": true
    })
}

#[derive(Serialize, Deserialize, Clone)]
struct McpServer {
    name: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: serde_json::Value,
    #[serde(default = "default_transport")]
    transport: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
}

fn default_transport() -> String { "stdio".into() }

fn servers_key(agent: &str) -> String {
    format!("mcp:servers:{}", agent)
}

fn get_servers(agent: &str) -> Vec<McpServer> {
    match kv_get(&servers_key(agent)) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    }
}

fn save_servers(agent: &str, servers: &[McpServer]) {
    let json = serde_json::to_string(servers).unwrap_or_else(|_| "[]".into());
    kv_set(&servers_key(agent), &json);
}

fn tools_cache_key(agent: &str, server: &str) -> String {
    format!("mcp:tools:{}:{}", agent, server)
}

fn next_mcp_ephemeral_process_name(kind: &str, agent: &str, server: &str) -> String {
    let suffix = MCP_EPHEMERAL_PROCESS_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("mcp-{}-{}-{}-{}", kind, agent, server, suffix)
}

fn build_jsonrpc_request(method: &str, params: serde_json::Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id: 1,
        method: method.to_string(),
        params,
    }
}

fn build_jsonrpc_notification(method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": method,
        "params": params,
    })
}

fn encode_stdio_message(value: &serde_json::Value) -> Result<String, String> {
    let mut body = serde_json::to_string(value)
        .map_err(|error| format!("failed to serialize MCP stdio payload: {}", error))?;
    body.push('\n');
    Ok(body)
}

fn try_parse_stdio_content_length_frame(
    buffer: &mut Vec<u8>,
) -> Result<Option<serde_json::Value>, String> {
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
    let response = serde_json::from_slice::<serde_json::Value>(&body)
        .map_err(|error| format!("invalid MCP stdio JSON body: {}", error))?;
    Ok(Some(response))
}

fn try_parse_stdio_line(buffer: &mut Vec<u8>) -> Result<Option<serde_json::Value>, String> {
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

    let response = serde_json::from_str::<serde_json::Value>(&trimmed)
        .map_err(|error| format!("invalid MCP stdio JSON line: {}", error))?;
    Ok(Some(response))
}

fn try_parse_stdio_message(buffer: &mut Vec<u8>) -> Result<Option<serde_json::Value>, String> {
    if buffer.starts_with(b"Content-Length:") || buffer.starts_with(b"content-length:") {
        return try_parse_stdio_content_length_frame(buffer);
    }

    try_parse_stdio_line(buffer)
}

fn post_jsonrpc_http(server: &McpServer, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = server
        .url
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("MCP server '{}' is missing URL for {} transport", server.name, server.transport))?;

    let request_body = serde_json::to_string(&build_jsonrpc_request(method, params))
        .map_err(|error| format!("failed to serialize MCP request: {}", error))?;

    let mut request = extism_pdk::HttpRequest::new(&url)
        .with_method("POST")
        .with_header("Content-Type", "application/json")
        .with_header("Accept", "application/json, text/event-stream");

    for (key, value) in &server.headers {
        request = request.with_header(key, value);
    }

    let response = extism_pdk::http::request::<String>(&request, Some(request_body))
        .map_err(|error| format!("MCP HTTP request failed: {}", error))?;

    let status = response.status_code();
    let body = String::from_utf8_lossy(&response.body()).to_string();
    if !(200..300).contains(&status) {
        return Err(format!("MCP server '{}' returned HTTP {}: {}", server.name, status, body));
    }

    let parsed: JsonRpcResponse = serde_json::from_str(&body)
        .map_err(|error| format!("failed to parse MCP JSON-RPC response: {}", error))?;
    if let Some(error) = parsed.error {
        return Err(format!("MCP server '{}' error: {}", server.name, error.message));
    }

    parsed
        .result
        .ok_or_else(|| format!("MCP server '{}' returned no result", server.name))
}

fn fetch_tools_from_server(agent: &str, server: &McpServer) -> Result<Vec<McpToolDef>, String> {
    match server.transport.as_str() {
        "stdio" => {
            let process_name = next_mcp_ephemeral_process_name("probe", agent, &server.name);
            let config = serde_json::json!({
                "name": process_name,
                "command": server.command,
                "args": server.args,
                "env": server.env,
                "restart_on_crash": false,
            });
            process_spawn(&config.to_string())?;
            let mut offset = 0usize;
            let init_result = stdio_jsonrpc_exchange(
                &process_name,
                &build_jsonrpc_request(
                    "initialize",
                    serde_json::json!({
                        "protocolVersion": MCP_PROTOCOL_VERSION,
                        "capabilities": {},
                        "clientInfo": {"name": "weft-mcp-client", "version": "0.1.0"}
                    }),
                ),
                &mut offset,
            )?;
            let _ = init_result;
            let _ = stdio_write_only(
                &process_name,
                &build_jsonrpc_notification(
                    "notifications/initialized",
                    serde_json::json!({}),
                ),
            );
            let result = stdio_jsonrpc_exchange(
                &process_name,
                &build_jsonrpc_request("tools/list", serde_json::json!({})),
                &mut offset,
            );
            let _ = process_stop(&process_name);
            let result = result?;
            let tools = result
                .get("tools")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            tools
                .into_iter()
                .map(|tool| {
                    let mut parsed: McpToolDef = serde_json::from_value(tool)
                        .map_err(|error| format!("invalid MCP tool definition: {}", error))?;
                    parsed.server = server.name.clone();
                    Ok(parsed)
                })
                .collect()
        }
        "http" | "sse" => {
            let result = post_jsonrpc_http(
                server,
                "tools/list",
                serde_json::json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                }),
            )?;
            let tools = result
                .get("tools")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            tools
                .into_iter()
                .map(|tool| {
                    let mut parsed: McpToolDef = serde_json::from_value(tool)
                        .map_err(|error| format!("invalid MCP tool definition: {}", error))?;
                    parsed.server = server.name.clone();
                    Ok(parsed)
                })
                .collect()
        }
        other => Err(format!(
            "MCP transport '{}' is not yet supported for live tool discovery in this runtime",
            other
        )),
    }
}

fn tool_defs_from_cache(agent: &str, server: &str) -> Vec<McpToolDef> {
    kv_get(&tools_cache_key(agent, server))
        .and_then(|json| serde_json::from_str::<Vec<McpToolDef>>(&json).ok())
        .unwrap_or_default()
}

fn save_tool_defs(agent: &str, server: &str, tools: &[McpToolDef]) {
    let json = serde_json::to_string(tools).unwrap_or_else(|_| "[]".into());
    kv_set(&tools_cache_key(agent, server), &json);
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("mcp-client package initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(), data: serde_json::Value::Null,
    });

    let result = match req.action.as_str() {
        "list_servers" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            do_list_servers(agent)
        }
        "add_server" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let server: McpServer = serde_json::from_value(req.data.clone()).unwrap_or(McpServer {
                name: String::new(), command: String::new(), args: vec![],
                env: serde_json::Value::Null, transport: default_transport(),
                url: None, headers: HashMap::new(),
            });
            do_add_server(agent, &server)
        }
        "remove_server" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let name = req.data["name"].as_str().unwrap_or("");
            do_remove_server(agent, name)
        }
        "start_server" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let name = req.data["name"].as_str().unwrap_or("");
            do_start_server(agent, name)
        }
        "stop_server" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let name = req.data["name"].as_str().unwrap_or("");
            do_stop_server(agent, name)
        }
        "get_tools" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let include_diagnostics = req.data["include_diagnostics"].as_bool().unwrap_or(false);
            do_get_tools(agent, include_diagnostics)
        }
        "call_tool" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let server = req.data["server"].as_str().unwrap_or("");
            let tool = req.data["tool"].as_str().unwrap_or("");
            let args = req.data.get("args").cloned().unwrap_or(serde_json::Value::Null);
            do_call_tool(agent, server, tool, &args)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

#[plugin_fn]
pub fn list_servers(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_list_servers(&p.agent).to_json())
}

fn do_list_servers(agent: &str) -> PackageResult {
    let servers = get_servers(agent);
    let list: Vec<serde_json::Value> = servers.iter()
        .map(|s| {
            let status = process_status(&format!("mcp-{}-{}", agent, s.name))
                .unwrap_or_else(|_| r#"{"status":"unknown"}"#.into());
            serde_json::json!({
                "name": s.name,
                "command": s.command,
                "transport": s.transport,
                "status": serde_json::from_str::<serde_json::Value>(&status)
                    .unwrap_or(serde_json::json!({"status":"unknown"})),
            })
        })
        .collect();
    PackageResult::ok(serde_json::json!({"servers": list}))
}

#[plugin_fn]
pub fn add_server(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String, #[serde(flatten)] server: McpServer }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_add_server(&p.agent, &p.server).to_json())
}

fn do_add_server(agent: &str, server: &McpServer) -> PackageResult {
    if server.name.is_empty() || server.command.is_empty() {
        return PackageResult::err("missing server name or command");
    }

    let mut servers = get_servers(agent);
    servers.retain(|s| s.name != server.name);
    servers.push(server.clone());
    save_servers(agent, &servers);

    log_info(&format!("mcp: added server '{}' for agent '{}'", server.name, agent));
    PackageResult::ok_empty()
}

#[plugin_fn]
pub fn remove_server(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String, name: String }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_remove_server(&p.agent, &p.name).to_json())
}

fn do_remove_server(agent: &str, name: &str) -> PackageResult {
    let mut servers = get_servers(agent);
    servers.retain(|s| s.name != name);
    save_servers(agent, &servers);

    // Stop if running
    let _ = process_stop(&format!("mcp-{}-{}", agent, name));

    PackageResult::ok_empty()
}

#[plugin_fn]
pub fn start_server(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String, name: String }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_start_server(&p.agent, &p.name).to_json())
}

fn do_start_server(agent: &str, name: &str) -> PackageResult {
    let servers = get_servers(agent);
    let server = match servers.iter().find(|s| s.name == name) {
        Some(s) => s,
        None => return PackageResult::err(format!("server '{}' not found", name)),
    };

    let process_name = format!("mcp-{}-{}", agent, name);
    let config = serde_json::json!({
        "name": process_name,
        "command": server.command,
        "args": server.args,
        "env": server.env,
        "restart_on_crash": false,
    });

    match process_spawn(&config.to_string()) {
        Ok(r) => PackageResult::ok(serde_json::from_str(&r).unwrap_or(serde_json::json!({"status":"ok"}))),
        Err(e) => PackageResult::err(e),
    }
}

#[plugin_fn]
pub fn stop_server(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String, name: String }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_stop_server(&p.agent, &p.name).to_json())
}

fn do_stop_server(agent: &str, name: &str) -> PackageResult {
    let process_name = format!("mcp-{}-{}", agent, name);
    match process_stop(&process_name) {
        Ok(_) => PackageResult::ok_empty(),
        Err(e) => PackageResult::err(e),
    }
}

#[plugin_fn]
pub fn get_tools(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        agent: String,
        #[serde(default)]
        include_diagnostics: bool,
    }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_get_tools(&p.agent, p.include_diagnostics).to_json())
}

fn do_get_tools(agent: &str, include_diagnostics: bool) -> PackageResult {
    let servers = get_servers(agent);
    let mut all_tools = Vec::new();
    let mut diagnostics = Vec::new();

    for server in &servers {
        match fetch_tools_from_server(agent, server) {
            Ok(tools) => {
                save_tool_defs(agent, &server.name, &tools);
                all_tools.extend(
                    tools
                        .into_iter()
                        .map(|tool| serde_json::to_value(tool).unwrap_or(serde_json::Value::Null)),
                );
            }
            Err(error) => {
                let cached_tools = tool_defs_from_cache(agent, &server.name);
                let cached_count = cached_tools.len();
                all_tools.extend(
                    cached_tools
                        .into_iter()
                        .map(|tool| serde_json::to_value(tool).unwrap_or(serde_json::Value::Null)),
                );

                if include_diagnostics {
                    diagnostics.push(ToolDiscoveryDiagnostic {
                        server: server.name.clone(),
                        transport: server.transport.clone(),
                        source: if cached_count > 0 { "cache".into() } else { "none".into() },
                        live_error: Some(error),
                        cached_tools: cached_count,
                    });
                }
            }
        }
    }

    let mut response = serde_json::json!({"tools": all_tools});
    if include_diagnostics {
        response["diagnostics"] = serde_json::to_value(diagnostics).unwrap_or(serde_json::Value::Array(vec![]));
    }

    PackageResult::ok(response)
}

#[plugin_fn]
pub fn call_tool(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String, server: String, tool: String, args: serde_json::Value }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_call_tool(&p.agent, &p.server, &p.tool, &p.args).to_json())
}

fn do_call_tool(agent: &str, server_name: &str, tool: &str, args: &serde_json::Value) -> PackageResult {
    let servers = get_servers(agent);
    let Some(server) = servers.iter().find(|entry| entry.name == server_name) else {
        return PackageResult::err(format!("server '{}' not found", server_name));
    };

    if tool.trim().is_empty() {
        return PackageResult::err("missing MCP tool name");
    }

    match server.transport.as_str() {
        "stdio" => {
            let process_name = next_mcp_ephemeral_process_name("call", agent, &server.name);
            let config = serde_json::json!({
                "name": process_name,
                "command": server.command,
                "args": server.args,
                "env": server.env,
                "restart_on_crash": false,
            });
            if let Err(error) = process_spawn(&config.to_string()) {
                return PackageResult::err(error);
            }
            let mut offset = 0usize;
            let init = stdio_jsonrpc_exchange(
                &process_name,
                &build_jsonrpc_request(
                    "initialize",
                    serde_json::json!({
                        "protocolVersion": MCP_PROTOCOL_VERSION,
                        "capabilities": {},
                        "clientInfo": {"name": "weft-mcp-client", "version": "0.1.0"}
                    }),
                ),
                &mut offset,
            );
            if let Err(error) = init {
                let _ = process_stop(&process_name);
                return PackageResult::err(error);
            }
            let _ = stdio_write_only(
                &process_name,
                &build_jsonrpc_notification(
                    "notifications/initialized",
                    serde_json::json!({}),
                ),
            );
            let result = stdio_jsonrpc_exchange(
                &process_name,
                &build_jsonrpc_request(
                    "tools/call",
                    serde_json::json!({
                        "name": tool,
                        "arguments": args,
                    }),
                ),
                &mut offset,
            );
            let _ = process_stop(&process_name);
            match result {
                Ok(output) => PackageResult::ok(serde_json::json!({
                    "server": server_name,
                    "tool": tool,
                    "output": output,
                })),
                Err(error) => PackageResult::err(error),
            }
        }
        "http" | "sse" => match post_jsonrpc_http(
            server,
            "tools/call",
            serde_json::json!({
                "name": tool,
                "arguments": args,
                "protocolVersion": MCP_PROTOCOL_VERSION,
            }),
        ) {
            Ok(output) => PackageResult::ok(serde_json::json!({
                "server": server_name,
                "tool": tool,
                "output": output,
            })),
            Err(error) => PackageResult::err(error),
        },
        other => PackageResult::err(format!(
            "MCP transport '{}' is not yet supported for live tool execution in this runtime",
            other
        )),
    }
}

fn stdio_write_only(process_name: &str, request: &serde_json::Value) -> Result<(), String> {
    let payload = encode_stdio_message(request)?;
    process_write_stdin(process_name, &payload)?;
    Ok(())
}

fn stdio_jsonrpc_exchange(
    process_name: &str,
    request: &JsonRpcRequest,
    offset: &mut usize,
) -> Result<serde_json::Value, String> {
    let request_value = serde_json::to_value(request)
        .map_err(|error| format!("failed to convert MCP request to JSON value: {}", error))?;
    let payload = encode_stdio_message(&request_value)?;
    process_write_stdin(process_name, &payload)?;

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut pending = Vec::<u8>::new();
    while Instant::now() < deadline {
        let read = process_read_stdout(process_name, *offset)?;
        *offset = read.next_offset;
        pending.extend_from_slice(&read.chunk);
        loop {
            let Some(frame) = try_parse_stdio_message(&mut pending)? else {
                break;
            };
            let response: JsonRpcResponse = serde_json::from_value(frame)
                .map_err(|error| format!("invalid MCP stdio response: {}", error))?;
            if let Some(error) = response.error {
                return Err(error.message);
            }
            if let Some(result) = response.result {
                return Ok(result);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    Err(format!("timeout waiting for MCP stdio response from {}", process_name))
}

