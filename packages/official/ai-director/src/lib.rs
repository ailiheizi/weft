use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use weft_package_sdk::*;

const PACKAGE_NAME: &str = "ai-director";
const PLAN_CAPABILITY_NAME: &str = "director.plan";
const TURN_CAPABILITY_NAME: &str = "director.turn";
const AGENT_RUNTIME_CAPABILITY: &str = "agent.runtime";
const SESSION_EVENTS_CAPABILITY: &str = "session.events";
const DIRECTOR_PLAN_SYSTEM_PROMPT: &str = "你是一位资深视频剪辑导演。根据用户提供的素材清单，输出一个剪辑方案。要求：1)选用哪些片段(给时间区间) 2)排序逻辑 3)节奏与情绪基调 4)最关键——解释每个决策的理由(为什么这么剪)。方案要体现导演思维，不要罗列功能。";
const DIRECTOR_TURN_SYSTEM_PROMPT: &str = "你是 AI 导演，一支创作 Agent 队伍的总导演。用户给你创意或素材后，你先准确理解意图，再决定如何推进。若创意仍模糊、关键信息缺失，或存在多种合理方向（如品牌风格、叙事角度、节奏基调、受众取向不明），优先调用 ask_user，提出一个简洁问题，并给出 2 到 4 个具体可选方向，让用户快速做选择；问完后等待用户回复，不要擅自展开。若任务是大型、多阶段或多模态创作，需要文案、分镜、配音、剪辑、整合等多个环节协作，优先调用 delegate_to_team，把目标清晰委派给创作团队推进，而不是自己包办全部产出。若任务范围单一且目标清晰，例如只写一句文案、只出一个剪辑方案、只整理一个创意方向，可直接完成；其中涉及剪辑方案时可调用 director.plan。你具备实际的创作执行能力：需要生成画面时调用 generate_image（传入英文提示词 prompt 与输出路径 output_path 即可直接产出图片文件）；需要把多张图片合成视频成片时调用 render_video（传入 images 路径列表、durations 每张时长、output 输出路径）。当用户明确要求出图或成片、且方向已清晰时，直接调用这些工具完成，不要声称自己无法生成图像或推脱给外部工具。所有输出都要体现导演视角、创作判断与可解释理由，不要机械罗列功能或流程。";

fn cancel_key(session_id: &str) -> String {
    format!("ai-director:cancel:{}", session_id)
}

#[derive(Debug, Deserialize)]
struct GeneratePlanInput {
    #[serde(default)]
    assets: Vec<AssetInput>,
    #[serde(default)]
    goal: String,
}

#[derive(Debug, Deserialize)]
struct AssetInput {
    #[serde(default)]
    name: String,
    #[serde(default)]
    duration: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    #[serde(default)]
    content: String,
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
        "label": "AI Director",
        "role": "creative_director",
        "system_prompt": DIRECTOR_TURN_SYSTEM_PROMPT,
        "skills": ["ask_user", "delegate_to_team", "director.plan", "generate_image", "render_video"],
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
        "package": PACKAGE_NAME,
        "runtime": "wasm",
        "capabilities": [PLAN_CAPABILITY_NAME, TURN_CAPABILITY_NAME],
        "actions": {
            PLAN_CAPABILITY_NAME: ["describe", "health", "generate_plan"],
            TURN_CAPABILITY_NAME: [
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
            "memory": "memory-store",
            "channels": "channel-core",
            "session_events": SESSION_EVENTS_CAPABILITY
        }
    }))
}

fn do_health() -> PackageResult {
    PackageResult::ok(serde_json::json!({
        "healthy": true,
        "package": PACKAGE_NAME,
        "capabilities": [PLAN_CAPABILITY_NAME, TURN_CAPABILITY_NAME],
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
            "failed to prepare ai-director agent session: {error}"
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
        "product": PACKAGE_NAME,
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
    let session_id = data
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let answer = data
        .get("answer")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();

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
        "generate_plan" => generate_plan(&data),
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
        "reset_session" | "delete_session" => {
            do_reset_session(serde_json::from_value(data).unwrap_or_default())
        }
        "cancel_turn" => do_cancel_turn(data),
        "submit_user_input" => do_submit_user_input(data),
        other => PackageResult::err(format!("unknown action: {other}")),
    }
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("ai-director initialized");
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

fn generate_plan(data: &Value) -> PackageResult {
    let input: GeneratePlanInput = match serde_json::from_value(data.clone()) {
        Ok(value) => value,
        Err(error) => return PackageResult::err(format!("invalid generate_plan payload: {}", error)),
    };

    let user_prompt = build_user_prompt(&input.assets, &input.goal);
    let body = match serde_json::to_string(&ChatCompletionRequest {
        model: "deepseek-chat",
        messages: vec![
            ChatMessage {
                role: "system",
                content: DIRECTOR_PLAN_SYSTEM_PROMPT,
            },
            ChatMessage {
                role: "user",
                content: &user_prompt,
            },
        ],
        temperature: 0.7,
    }) {
        Ok(value) => value,
        Err(error) => return PackageResult::err(format!("failed to build chat request: {}", error)),
    };

    let response_text = match chat_completion("ai-director:plan", "", &body) {
        Ok(value) => value,
        Err(error) => return PackageResult::err(error),
    };

    let response: ChatCompletionResponse = match serde_json::from_str(&response_text) {
        Ok(value) => value,
        Err(error) => return PackageResult::err(format!("failed to parse chat response: {}", error)),
    };

    let plan = match response.choices.first() {
        Some(choice) if !choice.message.content.trim().is_empty() => choice.message.content.clone(),
        _ => return PackageResult::err("chat response missing choices[0].message.content"),
    };

    PackageResult::ok(serde_json::json!({
        "plan": plan,
        "raw_usage": response.usage,
    }))
}

fn build_user_prompt(assets: &[AssetInput], goal: &str) -> String {
    let assets_text = if assets.is_empty() {
        "- 无素材".to_string()
    } else {
        assets
            .iter()
            .enumerate()
            .map(|(index, asset)| {
                format!(
                    "{}. 名称：{}\n   时长：{}\n   内容：{}",
                    index + 1,
                    empty_as_placeholder(&asset.name),
                    empty_as_placeholder(&asset.duration),
                    empty_as_placeholder(&asset.content)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "素材清单：\n{}\n\n目标：{}\n请给出你的导演剪辑方案。",
        assets_text,
        empty_as_placeholder(goal)
    )
}

fn empty_as_placeholder(value: &str) -> &str {
    if value.trim().is_empty() {
        "未提供"
    } else {
        value
    }
}
