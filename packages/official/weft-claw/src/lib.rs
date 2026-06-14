use base64::Engine;
use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const WEFT_CLAW_TURN_CAPABILITY: &str = "weft_claw.turn";
const AGENT_RUNTIME_CAPABILITY: &str = "agent.runtime";
const SESSION_EVENTS_CAPABILITY: &str = "session.events";

fn cancel_key(session_id: &str) -> String {
    format!("weft-claw:cancel:{}", session_id)
}

#[derive(Serialize, Deserialize, Default)]
struct SendMessageInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    content_b64: String,
    #[serde(default)]
    attachments: Vec<Value>,
    #[serde(default)]
    workspace_id: String,
    #[serde(default)]
    workspace_root: String,
    #[serde(default)]
    mode: String,
    #[serde(default)]
    agent: Value,
    #[serde(default)]
    delegate_request: Value,
    #[serde(default)]
    runtime_context: Value,
}

#[derive(Serialize, Deserialize, Default)]
struct SessionIdInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    after_seq: Option<u64>,
    #[serde(default)]
    limit: Option<u64>,
}

fn decode_transport_text(primary: &str, encoded: &str) -> String {
    if !encoded.trim().is_empty() {
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded.trim()) {
            if let Ok(decoded) = String::from_utf8(bytes) {
                let trimmed = decoded.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    primary.trim().to_string()
}

fn call_agent_runtime(action: &str, data: &Value) -> Result<Value, String> {
    let raw = call_package_ws_action("agent-runtime", action, data)?;
    let envelope: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("capability call returned invalid json: {error}"))?;

    let response = envelope.get("response").cloned().unwrap_or(envelope);

    if response.get("status").and_then(Value::as_str) == Some("error") {
        return Err(response
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("agent runtime action failed")
            .to_string());
    }

    Ok(response.get("data").cloned().unwrap_or(response))
}

fn call_session_events(action: &str, data: &Value) -> Result<Value, String> {
    let raw = call_capability_action(SESSION_EVENTS_CAPABILITY, action, data)?;
    let envelope: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("session.events returned invalid json: {error}"))?;
    let response = envelope.get("response").cloned().unwrap_or(envelope);
    if response.get("status").and_then(Value::as_str) == Some("error") {
        return Err(response
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("session.events action failed")
            .to_string());
    }
    Ok(response.get("data").cloned().unwrap_or(response))
}

fn append_session_event(session_id: &str, event_type: &str, payload: Value) -> Option<Value> {
    if session_id.trim().is_empty() {
        return None;
    }
    match call_session_events(
        "append_event",
        &serde_json::json!({
            "session_id": session_id,
            "type": event_type,
            "payload": payload,
        }),
    ) {
        Ok(value) => value.get("event").cloned(),
        Err(error) => {
            log_warn(&format!("session.events append failed: {error}"));
            None
        }
    }
}

fn list_session_events(
    session_id: &str,
    after_seq: Option<u64>,
    limit: Option<u64>,
) -> Result<Value, String> {
    let mut data = serde_json::json!({ "session_id": session_id });
    if let Some(after_seq) = after_seq {
        data["after_seq"] = Value::from(after_seq);
    }
    if let Some(limit) = limit {
        data["limit"] = Value::from(limit);
    }
    call_session_events("list_events", &data)
}

fn delete_session_events(session_id: &str) -> Result<Value, String> {
    call_session_events(
        "delete_session_events",
        &serde_json::json!({ "session_id": session_id }),
    )
}

fn delete_agent_session(session_id: &str) -> Result<Value, String> {
    call_agent_runtime("delete_session", &serde_json::json!({ "id": session_id }))
}

fn ensure_agent_session(input: &SendMessageInput, content: &str) -> Result<Value, String> {
    let title = content.chars().take(32).collect::<String>();
    let title = if title.trim().is_empty() {
        input.session_id.clone()
    } else {
        title
    };

    let now = now_ms();
    let agent = serde_json::json!({
        "label": "WEFT Claw",
        "role": "product_agent",
        "system_prompt": "You are WEFT Claw. Answer naturally and act through tools when the user asks for real work. Continue using tools until all explicit user requirements are complete.\n\nTool selection rules:\n- When the request is ambiguous or has several reasonable approaches (e.g. unclear scope, multiple tech/style options, missing key info), call ask_user with a concise question and 2-4 concrete options BEFORE building. Do this once for genuinely open choices — not for trivial or fully-specified tasks. The turn ends after asking; the user's pick arrives as the next message.\n- For LARGE or MULTI-MODULE build tasks (a full app/system spanning several distinct features, modules, or pages — e.g. an admin dashboard with product/order/user/analytics modules), call delegate_to_team with a clear goal INSTEAD of building everything yourself. An autonomous agent team (planner→implementer→reviewer→integrator) handles it and progress shows in the workspace. Reserve direct fs_write for small, single-file, or trivial outputs.\n- Use web_search when the user asks for current or online information.\n- Use fs_list for listing directory contents; use fs_read only for known regular files.\n- To find code in a large or unfamiliar codebase, do NOT read whole directory trees (it wastes context). First locate relevant files with shell_exec running a search: on Windows use PowerShell `Select-String -Path <glob> -Pattern <regex>` (or `findstr /s /n /c:\"text\" *.ext`); then fs_read only the matched files. This keeps long coding tasks within budget.\n- Use fs_write to create or update files.\n- Use the git tool (not shell_exec) for all git operations: init, add, commit, log, status, diff, etc. Pass args as [\"-C\", path, \"subcommand\", ...args].\n- Use shell_exec only when explicitly running a program, script, or non-git command. On Windows, to run a Python script use command=\"python\" with args=[script_path, arg1, ...] and cwd=script_dir. For other commands use command=\"pwsh\" with args=[\"-NoProfile\",\"-Command\",\"...\"].\n- If a tool fails, inspect the error, repair the arguments or switch tools, and retry before replying.\n- When the user asks for a git commit, run git init/add/commit/log and report the commit hash. Do not stop after only creating files.\n- When creating an HTML page / web page / report / dashboard, do NOT write plain bare HTML. A 'frontend-page' skill template will be injected into your context for such requests — base the page on that template's <style> and class structure (.container/.grid/.card/table/.badge), fill <body> with real content in a SINGLE fs_write step, and do not stop to say 'please wait'.\n- Choose the best PRESENTATION format for the user's result and write a self-contained .html accordingly (all render live in the workspace): for charts/trends/statistics/data-analysis use the data-report skill (Plotly); for motion/loading/flow/celebration effects use the lottie-animation skill; for pages/reports/dashboards use frontend-page; for plain answers just reply as text. Pick the format that best communicates the result, not always plain text.",
        "skills": ["web_search", "web_fetch", "fs_list", "fs_read", "shell_exec", "fs_write", "git"],
        "skills_plugin": "skills-runtime",
        "memory_plugin": "memory-store",
        "channels_plugin": "channel-core",
    });

    call_agent_runtime(
        "create_session",
        &serde_json::json!({
            "id": input.session_id,
            "title": title,
            "workspace_id": input.workspace_id,
            "workspace_root": input.workspace_root,
            "persistent": 1,
            "created_at": now,
            "updated_at": now,
            "agent": agent,
        }),
    )
}

fn ok_turn(data: Value) -> PackageResult {
    PackageResult::ok(data)
}

fn do_describe() -> PackageResult {
    PackageResult::ok(serde_json::json!({
        "package": "weft-claw",
        "runtime": "wasm",
        "capabilities": [WEFT_CLAW_TURN_CAPABILITY, "ui.surface"],
        "actions": {
            WEFT_CLAW_TURN_CAPABILITY: [
                "describe",
                "health",
                "send_message",
                "list_sessions",
                "get_session_messages",
                "list_events",
                "get_session_events",
                "delete_session_events",
                "reset_session",
                "cancel_turn",
                "submit_user_input"
            ]
        },
        "delegates": {
            "agent_runtime": AGENT_RUNTIME_CAPABILITY,
            "skills": "skills-runtime",
            "mcp": "ext.mcp",
            "memory": "memory-store",
            "channels": "channel-core"
        }
    }))
}

fn do_health() -> PackageResult {
    PackageResult::ok(serde_json::json!({
        "healthy": true,
        "package": "weft-claw",
        "capability": WEFT_CLAW_TURN_CAPABILITY,
    }))
}

fn do_send_message(input: SendMessageInput) -> PackageResult {
    if input.session_id.trim().is_empty() {
        return PackageResult::err("send_message requires session_id");
    }

    let content = decode_transport_text(&input.content, &input.content_b64);
    if content.trim().is_empty() {
        return PackageResult::err("send_message requires content");
    }

    // Clear any pending cancel flag from a previous cancel_turn call.
    let _ = kv_delete(&cancel_key(&input.session_id));

    let mut events = Vec::new();
    if let Some(event) = append_session_event(
        &input.session_id,
        "user_message",
        serde_json::json!({
            "role": "user",
            "content": content,
            "workspace_id": input.workspace_id,
            "workspace_root": input.workspace_root,
            "mode": input.mode,
        }),
    ) {
        events.push(event);
    }
    if let Some(event) = append_session_event(
        &input.session_id,
        "task_status",
        serde_json::json!({ "status": "running" }),
    ) {
        events.push(event);
    }

    if let Err(error) = ensure_agent_session(&input, &content) {
        return PackageResult::err(format!(
            "failed to prepare weft-claw agent session: {error}"
        ));
    }

    let mut payload = serde_json::json!({
        "session_id": input.session_id,
        "content": content,
        "content_b64": input.content_b64,
        "delegate_request": input.delegate_request,
    });

    if !input.agent.is_null() {
        payload["agent"] = input.agent;
    }

    let mut runtime_context = serde_json::json!({
        "product": "weft-claw",
        "mode": input.mode,
        "workspace_id": input.workspace_id,
        "workspace_root": input.workspace_root,
        "attachments": input.attachments,
    });
    if let Some(object) = input.runtime_context.as_object() {
        if let Some(target) = runtime_context.as_object_mut() {
            for (key, value) in object {
                target.insert(key.clone(), value.clone());
            }
        }
    }
    payload["runtime_context"] = runtime_context;

    match call_agent_runtime("send_session_message", &payload) {
        Ok(agent_result) => {
            let reply = agent_result
                .get("reply")
                .cloned()
                .unwrap_or(Value::String(String::new()));
            if let Some(event) = append_session_event(
                &input.session_id,
                "assistant_message",
                serde_json::json!({ "role": "assistant", "content": reply, "agent": agent_result }),
            ) {
                events.push(event);
            }
            if let Some(event) = append_session_event(
                &input.session_id,
                "done",
                serde_json::json!({ "status": "completed" }),
            ) {
                events.push(event);
            }
            ok_turn(serde_json::json!({
                "session_id": payload.get("session_id").cloned().unwrap_or(Value::Null),
                "status": "completed",
                "reply": reply,
                "agent": agent_result,
                "events": events,
            }))
        }
        Err(error) => {
            if let Some(event) = append_session_event(
                &input.session_id,
                "error",
                serde_json::json!({ "message": error }),
            ) {
                events.push(event);
            }
            PackageResult::err(error)
        }
    }
}

fn do_list_sessions() -> PackageResult {
    match call_agent_runtime("list_sessions", &serde_json::json!({})) {
        Ok(value) => ok_turn(value),
        Err(error) => PackageResult::err(error),
    }
}

fn do_get_session_messages(input: SessionIdInput) -> PackageResult {
    if input.session_id.trim().is_empty() {
        return PackageResult::err("get_session_messages requires session_id");
    }

    match call_agent_runtime(
        "get_session_messages",
        &serde_json::json!({ "session_id": input.session_id }),
    ) {
        Ok(value) => ok_turn(value),
        Err(error) => PackageResult::err(error),
    }
}

fn do_list_events(input: SessionIdInput) -> PackageResult {
    if input.session_id.trim().is_empty() {
        return PackageResult::err("list_events requires session_id");
    }

    match list_session_events(&input.session_id, input.after_seq, input.limit) {
        Ok(value) => ok_turn(value),
        Err(error) => PackageResult::err(error),
    }
}

fn do_delete_session_events(input: SessionIdInput) -> PackageResult {
    if input.session_id.trim().is_empty() {
        return PackageResult::err("delete_session_events requires session_id");
    }

    match delete_session_events(&input.session_id) {
        Ok(value) => ok_turn(value),
        Err(error) => PackageResult::err(error),
    }
}

fn do_reset_session(input: SessionIdInput) -> PackageResult {
    if input.session_id.trim().is_empty() {
        return PackageResult::err("reset_session requires session_id");
    }

    let agent_result = delete_agent_session(&input.session_id);
    let events_result = delete_session_events(&input.session_id);

    match (agent_result, events_result) {
        (Ok(agent), Ok(events)) => ok_turn(serde_json::json!({
            "session_id": input.session_id,
            "reset": true,
            "agent": agent,
            "events": events,
        })),
        (Err(error), _) | (_, Err(error)) => PackageResult::err(error),
    }
}

fn ask_user_pending_key(session_id: &str) -> String {
    format!("ask_user:{}:pending", session_id)
}

fn ask_user_response_key(session_id: &str) -> String {
    format!("ask_user:{}:response", session_id)
}

fn do_submit_user_input(data: Value) -> PackageResult {
    let session_id = data.get("session_id").and_then(Value::as_str).unwrap_or("").trim().to_string();
    let answer = data.get("answer").and_then(Value::as_str).unwrap_or("").trim().to_string();

    if session_id.is_empty() {
        return PackageResult::ok(serde_json::json!({ "submitted": false, "reason": "session_id required" }));
    }
    if answer.is_empty() {
        return PackageResult::ok(serde_json::json!({ "submitted": false, "reason": "answer required" }));
    }

    let response = serde_json::json!({ "answer": answer, "submitted_at": now_ms() });
    kv_set(&ask_user_response_key(&session_id), &response.to_string());
    let _ = kv_delete(&ask_user_pending_key(&session_id));

    PackageResult::ok(serde_json::json!({ "submitted": true, "session_id": session_id }))
}

fn do_cancel_turn(data: Value) -> PackageResult {
    let session_id = data
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if session_id.is_empty() {
        return PackageResult::ok(serde_json::json!({
            "cancelled": false,
            "reason": "cancel_turn requires session_id",
        }));
    }
    kv_set(&cancel_key(&session_id), "1");
    if let Some(event) = append_session_event(
        &session_id,
        "task_status",
        serde_json::json!({ "status": "cancelled" }),
    ) {
        return PackageResult::ok(serde_json::json!({
            "cancelled": true,
            "session_id": session_id,
            "event": event,
        }));
    }
    PackageResult::ok(serde_json::json!({
        "cancelled": true,
        "session_id": session_id,
    }))
}

fn dispatch(action: &str, data: Value) -> PackageResult {
    match action {
        "describe" => do_describe(),
        "health" => do_health(),
        "send_message" => do_send_message(serde_json::from_value(data).unwrap_or_default()),
        "list_sessions" => do_list_sessions(),
        "get_session_messages" => {
            do_get_session_messages(serde_json::from_value(data).unwrap_or_default())
        }
        "list_events" | "get_session_events" => {
            do_list_events(serde_json::from_value(data).unwrap_or_default())
        }
        "delete_session_events" => {
            do_delete_session_events(serde_json::from_value(data).unwrap_or_default())
        }
        "reset_session" | "delete_session" => do_reset_session(serde_json::from_value(data).unwrap_or_default()),
        "cancel_turn" => do_cancel_turn(data),
        "submit_user_input" => do_submit_user_input(data),
        other => PackageResult::err(format!("unknown action: {other}")),
    }
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("weft-claw product runtime initialized");
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
    Ok(do_describe().to_json())
}

#[plugin_fn]
pub fn health(_input: String) -> FnResult<String> {
    Ok(do_health().to_json())
}
