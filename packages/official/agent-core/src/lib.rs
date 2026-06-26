//! Agent-core package - AI agent instance management and session-aware LLM dialog engine.

use std::cell::RefCell;

use base64::Engine;
use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(improper_ctypes_definitions)]
mod test_extism_host_stubs {
    use std::cell::RefCell;

    thread_local! {
        static MEMORY: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    fn write_memory(bytes: &[u8]) -> u64 {
        MEMORY.with(|memory| {
            let mut memory = memory.borrow_mut();
            let offset = memory.len().max(8) as u64;
            let length = bytes.len() as u64;
            memory.resize(offset as usize + 8 + bytes.len(), 0);
            memory[offset as usize..offset as usize + 8].copy_from_slice(&length.to_le_bytes());
            memory[offset as usize + 8..offset as usize + 8 + bytes.len()].copy_from_slice(bytes);
            offset
        })
    }

    fn read_memory(offset: u64, length: u64) -> Vec<u8> {
        MEMORY.with(|memory| {
            let memory = memory.borrow();
            let start = offset as usize;
            let end = start.saturating_add(length as usize).min(memory.len());
            if start >= memory.len() {
                Vec::new()
            } else {
                memory[start..end].to_vec()
            }
        })
    }

    fn read_string(offset: u64) -> String {
        String::from_utf8(read_memory(offset, length(offset))).unwrap_or_default()
    }

    fn write_string(value: &str) -> u64 {
        write_memory(value.as_bytes()) + 8
    }

    macro_rules! extism_stub {
        ($name:ident () -> $ret:ty, $value:expr) => {
            #[no_mangle]
            pub extern "C" fn $name() -> $ret {
                $value
            }
        };
        ($name:ident ($($arg:ident: $ty:ty),*) -> $ret:ty, $value:expr) => {
            #[no_mangle]
            pub extern "C" fn $name($($arg: $ty),*) -> $ret {
                $(let _ = $arg;)*
                $value
            }
        };
        ($name:ident ($($arg:ident: $ty:ty),*)) => {
            #[no_mangle]
            pub extern "C" fn $name($($arg: $ty),*) {
                $(let _ = $arg;)*
            }
        };
    }

    macro_rules! host_string_stub {
        ($name:ident) => {
            #[no_mangle]
            pub extern "C" fn $name(_input: u64) -> u64 {
                write_string("")
            }
        };
    }

    extism_stub!(error_set(_offset: u64));
    extism_stub!(input_length() -> u64, 0);
    extism_stub!(input_load_u64(_offset: u64) -> u64, 0);
    extism_stub!(input_load_u8(_offset: u64) -> u8, 0);

    #[no_mangle]
    pub extern "C" fn load_u8(offset: u64) -> u8 {
        MEMORY.with(|memory| memory.borrow().get(offset as usize).copied().unwrap_or(0))
    }

    #[no_mangle]
    pub extern "C" fn load_u64(offset: u64) -> u64 {
        let bytes = read_memory(offset, 8);
        if bytes.len() == 8 {
            u64::from_le_bytes(bytes.try_into().unwrap_or_default())
        } else {
            0
        }
    }

    #[no_mangle]
    pub extern "C" fn store_u8(offset: u64, data: u8) {
        MEMORY.with(|memory| {
            let mut memory = memory.borrow_mut();
            if offset as usize >= memory.len() {
                memory.resize(offset as usize + 1, 0);
            }
            memory[offset as usize] = data;
        });
    }

    #[no_mangle]
    pub extern "C" fn store_u64(offset: u64, data: u64) {
        MEMORY.with(|memory| {
            let mut memory = memory.borrow_mut();
            if offset as usize + 8 > memory.len() {
                memory.resize(offset as usize + 8, 0);
            }
            memory[offset as usize..offset as usize + 8].copy_from_slice(&data.to_le_bytes());
        });
    }

    extism_stub!(output_set(_offset: u64, _length: u64));
    extism_stub!(log_trace(_offset: u64));
    extism_stub!(log_debug(_offset: u64));
    extism_stub!(log_info(_offset: u64));
    extism_stub!(log_warn(_offset: u64));
    extism_stub!(log_error(_offset: u64));

    #[no_mangle]
    pub extern "C" fn alloc(length: u64) -> u64 {
        write_memory(&vec![0; length as usize]) + 8
    }

    extism_stub!(free(_offset: u64));

    #[no_mangle]
    pub extern "C" fn length(offset: u64) -> u64 {
        if offset < 8 {
            return 0;
        }
        load_u64(offset - 8)
    }

    #[no_mangle]
    pub extern "C" fn length_unsafe(offset: u64) -> u64 {
        length(offset)
    }

    extism_stub!(config_get(_offset: u64) -> u64, 0);
    extism_stub!(var_get(_offset: u64) -> u64, 0);
    extism_stub!(var_set(_offset: u64, _offset1: u64));
    extism_stub!(http_request(_request: u64, _body: u64) -> u64, 0);
    extism_stub!(http_status_code() -> i32, 0);
    extism_stub!(http_headers() -> u64, 0);
    extism_stub!(get_log_level() -> i32, 0);

    host_string_stub!(host_log);
    host_string_stub!(host_kv_get);
    host_string_stub!(host_kv_list);
    host_string_stub!(host_env_get);
    host_string_stub!(host_read_file);
    host_string_stub!(host_list_dir);
    host_string_stub!(host_exec);
    host_string_stub!(host_exec_advanced);
    host_string_stub!(host_chat_completion);
    host_string_stub!(host_chat_completion_stream);
    host_string_stub!(host_call_package_ws);

    // host_now_ms must return a plausible millisecond timestamp so that
    // `now_ms()` (which parses the result) yields a value >= any test fixture's
    // `started_at`. Using a large constant avoids assertion failures in
    // trajectory completion tests.
    #[no_mangle]
    pub extern "C" fn host_now_ms(_input: u64) -> u64 {
        write_string("9999999999")
    }
    host_string_stub!(host_capability_call);
    host_string_stub!(host_process_spawn);
    host_string_stub!(host_process_stop);
    host_string_stub!(host_process_status);
    host_string_stub!(host_process_write_stdin);
    host_string_stub!(host_process_read_stdout);
    host_string_stub!(host_sqlite_query);
    host_string_stub!(host_sqlite_execute);
    host_string_stub!(host_sqlite_batch);

    #[no_mangle]
    pub extern "C" fn host_call_package(input: u64) -> u64 {
        let request: serde_json::Value =
            serde_json::from_str(&read_string(input)).unwrap_or_default();
        let func = request
            .get("func")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let response = match func {
            "get_tool_specs" => r#"{"status":"ok","data":{"tools":[]}}"#,
            "get_inbox" => r#"{"status":"ok","data":{"messages":[]}}"#,
            _ => "",
        };
        write_string(response)
    }

    #[no_mangle]
    pub extern "C" fn host_kv_set(_input: u64) {}

    #[no_mangle]
    pub extern "C" fn host_kv_delete(_input: u64) {}

    #[no_mangle]
    pub extern "C" fn host_write_file(_input: u64) {}
}

const AGENT_RUNTIME_CAPABILITY: &str = "agent.runtime";
const TEAM_DELEGATE_CAPABILITY: &str = "team.delegate";
const TEAM_RUNTIME_PLUGIN: &str = "team-runtime";
const SESSION_EVENTS_CAPABILITY: &str = "session.events";

#[derive(Serialize, Deserialize, Clone)]
struct AgentConfig {
    name: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    role: String,
    #[serde(default = "default_model")]
    model: String,
    #[serde(default = "default_temp")]
    temperature: f64,
    #[serde(default)]
    system_prompt: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    channels: Vec<Value>,
    #[serde(default)]
    provider: String,
    #[serde(default = "default_memory_package")]
    memory_package: String,
    #[serde(default = "default_skills_package")]
    skills_package: String,
    #[serde(default = "default_channels_package")]
    channels_package: String,
    #[serde(default = "default_completion_endpoint")]
    completion_endpoint: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct SessionRecord {
    id: String,
    title: String,
    #[serde(default)]
    workspace_id: String,
    #[serde(default)]
    workspace_root: String,
    #[serde(default)]
    persistent: u8,
    created_at: u64,
    updated_at: u64,
    agent_name: String,
    #[serde(default = "default_session_agent")]
    agent: AgentConfig,
}

#[derive(Serialize, Deserialize, Clone)]
struct SessionMessageRecord {
    id: String,
    session_id: String,
    role: String,
    content: String,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_args: Option<String>,
    #[serde(default)]
    tool_status: Option<String>,
    #[serde(default)]
    streaming: bool,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct SessionAgentInput {
    #[serde(default)]
    label: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    temperature: Option<f64>,
    #[serde(default)]
    system_prompt: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    channels: Vec<Value>,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    memory_package: String,
    #[serde(default)]
    skills_package: String,
    #[serde(default)]
    channels_package: String,
    #[serde(default)]
    completion_endpoint: String,
}

#[derive(Serialize, Deserialize)]
struct CreateSessionInput {
    id: String,
    title: String,
    #[serde(default)]
    title_b64: String,
    #[serde(default)]
    workspace_id: String,
    #[serde(default)]
    workspace_root: String,
    #[serde(default)]
    persistent: u8,
    created_at: u64,
    updated_at: u64,
    #[serde(default)]
    agent: Option<SessionAgentInput>,
}

#[derive(Serialize, Deserialize)]
struct SessionIdInput {
    id: String,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Serialize, Deserialize)]
struct SessionTitleInput {
    id: String,
    title: String,
    #[serde(default)]
    title_b64: String,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Serialize, Deserialize)]
struct SessionPersistentInput {
    id: String,
    persistent: u8,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Serialize, Deserialize)]
struct SessionMessagesInput {
    session_id: String,
}

#[derive(Serialize, Deserialize)]
struct SessionContextInput {
    session_id: String,
    #[serde(default = "default_context_limit")]
    limit: usize,
}

#[derive(Serialize, Deserialize)]
struct SessionSkillsInput {
    session_id: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct WorkspaceRecord {
    id: String,
    name: String,
    #[serde(default)]
    root_path: String,
    created_at: u64,
    updated_at: u64,
}

#[derive(Serialize, Deserialize)]
struct CreateWorkspaceInput {
    id: String,
    name: String,
    #[serde(default)]
    name_b64: String,
    #[serde(default)]
    root_path: String,
    created_at: u64,
    updated_at: u64,
}

#[derive(Serialize, Deserialize)]
struct WorkspaceIdInput {
    id: String,
}

#[derive(Serialize, Deserialize)]
struct SaveSessionMessageInput {
    id: String,
    session_id: String,
    role: String,
    content: String,
    #[serde(default)]
    content_b64: String,
    #[serde(default)]
    tool_name: String,
    #[serde(default)]
    tool_args: String,
    #[serde(default)]
    tool_status: String,
    #[serde(default)]
    streaming: bool,
    timestamp: u64,
}

#[derive(Serialize, Deserialize)]
struct UpdateSessionMessageInput {
    id: String,
    session_id: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    content_b64: String,
    #[serde(default)]
    streaming: bool,
    #[serde(default)]
    tool_status: String,
}

#[derive(Serialize, Deserialize)]
struct SendSessionMessageInput {
    session_id: String,
    content: String,
    #[serde(default)]
    content_b64: String,
    #[serde(default)]
    agent: Option<SessionAgentInput>,
    #[serde(default)]
    delegate_request: Option<DelegateRequestInput>,
    #[serde(default)]
    selected_tools: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct DelegateRequestInput {
    #[serde(default)]
    must_act: bool,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    latest_user_query: String,
    #[serde(default)]
    visible_history: Vec<Value>,
    #[serde(default)]
    session_context: Vec<Value>,
    #[serde(default)]
    runtime_context: Value,
    #[serde(default)]
    skill_refs: Vec<String>,
    #[serde(default)]
    action_refs: Vec<String>,
    #[serde(default)]
    event: Value,
    /// 纯规划模式:不注入任何工具,强制 agent 只输出文本(如 planner 的子任务 JSON)。
    /// 防止带工具的 LLM 直接动手干活而非克制地做分解。
    #[serde(default)]
    planning_only: bool,
    /// 模型分层:本次 turn 覆盖默认 model(team-runtime 按角色从 config [team.roleRouting] 注入)。
    /// None 则用 agent 自身 config.model。
    #[serde(default)]
    model_override: Option<String>,
    /// 模型分层:本次 turn 覆盖 provider(对应 core 的 x_provider 路由)。None 则按 model 自动路由。
    #[serde(default)]
    provider_override: Option<String>,
}

/// 模型分层:把 delegate 携带的 model_override/provider_override 应用到 chat body,
/// 覆盖 agent 默认的 model / x_provider。None 或空串则保持原值。
fn apply_delegate_model_override(
    body: &mut serde_json::Value,
    delegate_request: Option<&DelegateRequestInput>,
) {
    if let Some(dr) = delegate_request {
        if let Some(m) = dr
            .model_override
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            body["model"] = serde_json::json!(m);
        }
        if let Some(p) = dr
            .provider_override
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            body["x_provider"] = serde_json::json!(p);
        }
    }
}

#[derive(Deserialize, Clone, Default)]
struct TeamDelegateRouteInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    workflow_id: String,
    #[serde(default)]
    from_role_id: String,
    #[serde(default)]
    to_role_id: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    must_act: bool,
    #[serde(default)]
    execute: bool,
    #[serde(default)]
    context_refs: Vec<String>,
    #[serde(default)]
    metadata: Value,
}

fn default_model() -> String {
    "deepseek-chat".into()
}
fn default_temp() -> f64 {
    0.7
}
fn default_memory_package() -> String {
    "memory".into()
}
fn default_skills_package() -> String {
    "skills".into()
}
fn default_channels_package() -> String {
    "channels".into()
}
fn default_completion_endpoint() -> String {
    "http://127.0.0.1:42617/v1/chat/completions".into()
}
fn default_context_limit() -> usize {
    18
}

const AGENTS_INDEX_KEY: &str = "agent-core:agents:__index";
const SESSIONS_INDEX_KEY: &str = "agent-core:sessions:__index";
const WORKSPACES_INDEX_KEY: &str = "agent-core:workspaces:__index";
const TRAJECTORIES_INDEX_KEY: &str = "agent-core:trajectories:__index";

#[derive(Serialize, Deserialize, Clone, Default)]
struct AgentTrajectory {
    id: String,
    agent: String,
    task: String,
    started_at: u64,
    completed_at: u64,
    status: String,
    steps: Vec<String>,
    tool_names: Vec<String>,
    final_result: String,
    failure: String,
    #[serde(default)]
    injected_skill_ids: Vec<String>,
    #[serde(default)]
    promotion_result: Value,
    /// Sum of upstream `prompt_tokens` (or `input_tokens`) across all rounds in this
    /// trajectory. Used together with `cache_read_tokens` to compute prefix-cache
    /// hit ratio — the single number that tells us whether Reasonix-style prefix
    /// stability is paying off in production.
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    /// Tokens served from upstream prompt cache. Anthropic reports this as
    /// `cache_read_input_tokens`; DeepSeek as `prompt_cache_hit_tokens`. Both are
    /// summed into this counter.
    #[serde(default)]
    cache_read_tokens: u64,
    /// Anthropic-only: tokens spent writing a new cache entry (one-off cost paid
    /// the first time a stable prefix is seen).
    #[serde(default)]
    cache_creation_tokens: u64,
    /// DeepSeek-only: tokens that missed the cache. `cache_read + cache_miss`
    /// should approximate `prompt_tokens` for a single round.
    #[serde(default)]
    cache_miss_tokens: u64,
    /// Hex digest of the *stable prefix* (first system message) the most recent
    /// round sent upstream. Reasonix Pillar 1 holds that this string must be
    /// byte-identical across turns for upstream prompt-cache to hit.
    #[serde(default)]
    last_prefix_fingerprint: String,
    /// Count of rounds whose stable-prefix fingerprint differed from the prior
    /// round's. Should be 0 in a healthy session; > 0 means something invalidated
    /// the cache (tool list churn, system prompt mutation, MCP reconnect, etc.).
    #[serde(default)]
    prefix_invalidation_count: u64,
}

#[derive(Clone, Default)]
struct RetrievedEvolvedSkills {
    context_block: String,
    skill_ids: Vec<String>,
}

fn extract_evolved_skill_ids(data: &Value) -> Vec<String> {
    data.get("skills")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("id").and_then(Value::as_str).or_else(|| {
                        item.get("skill")
                            .and_then(|skill| skill.get("id"))
                            .and_then(Value::as_str)
                    })
                })
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn default_agent_config() -> AgentConfig {
    AgentConfig {
        name: String::new(),
        label: String::new(),
        role: String::new(),
        model: default_model(),
        temperature: default_temp(),
        system_prompt: String::new(),
        skills: vec![],
        channels: vec![],
        provider: String::new(),
        memory_package: default_memory_package(),
        skills_package: default_skills_package(),
        channels_package: default_channels_package(),
        completion_endpoint: default_completion_endpoint(),
    }
}

fn default_session_agent() -> AgentConfig {
    default_agent_config()
}

fn get_agents_index() -> Vec<String> {
    match kv_get(AGENTS_INDEX_KEY) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    }
}

fn save_agents_index(names: &[String]) {
    let json = serde_json::to_string(names).unwrap_or_else(|_| "[]".into());
    kv_set(AGENTS_INDEX_KEY, &json);
}

fn agent_config_key(name: &str) -> String {
    format!("agent-core:config:{}", name)
}

fn load_agent(name: &str) -> Option<AgentConfig> {
    kv_get(&agent_config_key(name)).and_then(|json| serde_json::from_str(&json).ok())
}

fn save_agent(config: &AgentConfig) {
    let json = serde_json::to_string(config).unwrap_or_default();
    kv_set(&agent_config_key(&config.name), &json);
}

fn history_key(agent: &str) -> String {
    format!("agent-core:history:{}", agent)
}

const HISTORY_DB_PATH: &str = "./data/agent-core-history.db";

/// Ensure the SQLite table for history persistence exists.
fn ensure_history_table() {
    let _ = sqlite_execute(
        HISTORY_DB_PATH,
        "CREATE TABLE IF NOT EXISTS history (agent TEXT PRIMARY KEY, messages TEXT NOT NULL, updated_at INTEGER NOT NULL DEFAULT 0)",
        &[],
    );
}

fn get_history(agent: &str) -> Vec<Value> {
    // Try KV first (hot cache).
    if let Some(json) = kv_get(&history_key(agent)) {
        let msgs: Vec<Value> = serde_json::from_str(&json).unwrap_or_default();
        if !msgs.is_empty() {
            return msgs;
        }
    }
    // Fallback: restore from SQLite.
    ensure_history_table();
    if let Ok(result) = sqlite_query(
        HISTORY_DB_PATH,
        "SELECT messages FROM history WHERE agent = ?1",
        &[Value::String(agent.to_string())],
    ) {
        if let Some(row) = result.rows.first() {
            if let Some(json_val) = row.first().and_then(Value::as_str) {
                let msgs: Vec<Value> = serde_json::from_str(json_val).unwrap_or_default();
                if !msgs.is_empty() {
                    // Re-populate KV cache.
                    kv_set(&history_key(agent), json_val);
                    return msgs;
                }
            }
        }
    }
    vec![]
}

fn save_history(agent: &str, history: &[Value]) {
    let json = serde_json::to_string(history).unwrap_or_else(|_| "[]".into());
    // Write to KV (hot cache).
    kv_set(&history_key(agent), &json);
    // Persist to SQLite.
    ensure_history_table();
    let _ = sqlite_execute(
        HISTORY_DB_PATH,
        "INSERT OR REPLACE INTO history (agent, messages, updated_at) VALUES (?1, ?2, ?3)",
        &[
            Value::String(agent.to_string()),
            Value::String(json),
            Value::Number(serde_json::Number::from(now_ms())),
        ],
    );
}

/// Undo the last N conversation rounds (a round = one user message + all subsequent
/// assistant/tool messages until the next user message or end).
fn do_undo_round(agent: &str, rounds: usize) -> PackageResult {
    if agent.is_empty() {
        return PackageResult::err("agent name is required");
    }
    if rounds == 0 {
        return PackageResult::ok(serde_json::json!({"undone_rounds": 0, "remaining": 0}));
    }

    let mut history = get_history(agent);
    if history.is_empty() {
        return PackageResult::ok(serde_json::json!({"undone_rounds": 0, "remaining": 0}));
    }

    // Walk backwards, counting user messages as round boundaries.
    let original_len = history.len();
    let mut user_count = 0usize;
    let mut cut_index = history.len();

    for i in (0..history.len()).rev() {
        let role = history[i].get("role").and_then(Value::as_str).unwrap_or("");
        if role == "user" {
            user_count += 1;
            cut_index = i;
            if user_count >= rounds {
                break;
            }
        }
    }

    history.truncate(cut_index);
    save_history(agent, &history);

    let removed = original_len - history.len();
    PackageResult::ok(serde_json::json!({
        "undone_rounds": user_count.min(rounds),
        "removed_messages": removed,
        "remaining": history.len(),
    }))
}

/// Build a structured display payload for tool outputs so the frontend can render
/// rich visualization (cards, code blocks, images, etc.) instead of plain text.
/// Returns `Value::Null` when no special visualization applies.
fn build_tool_display(tool_name: &str, output: &str, is_error: bool) -> Value {
    if is_error {
        return serde_json::json!({"type": "error", "message": truncate_event_text(output, 1000)});
    }
    match tool_name {
        "web_search" | "tavily_search" => {
            // Try to parse as JSON array of search results
            if let Ok(results) = serde_json::from_str::<Value>(output) {
                return serde_json::json!({"type": "search_results", "data": results});
            }
            Value::Null
        }
        "fs_read" | "file_read" => {
            serde_json::json!({
                "type": "code",
                "content": truncate_event_text(output, 8000),
                "language": detect_language_hint(output),
            })
        }
        "shell_exec" | "run_command" => {
            serde_json::json!({
                "type": "terminal",
                "output": truncate_event_text(output, 8000),
            })
        }
        "browser_screenshot" => {
            // Output typically contains a file path or base64 image
            serde_json::json!({"type": "image", "data": truncate_event_text(output, 4000)})
        }
        "browser_snapshot" => {
            serde_json::json!({"type": "a11y_tree", "content": truncate_event_text(output, 8000)})
        }
        "browser_navigate" => {
            serde_json::json!({"type": "navigation", "data": truncate_event_text(output, 500)})
        }
        "semantic_select" => {
            if let Ok(results) = serde_json::from_str::<Value>(output) {
                return serde_json::json!({"type": "selector_results", "data": results});
            }
            Value::Null
        }
        "image_generate" | "image_gen" => {
            serde_json::json!({"type": "image", "data": truncate_event_text(output, 4000)})
        }
        _ => Value::Null,
    }
}

/// Simple heuristic to guess the language from file content for syntax highlighting.
fn detect_language_hint(content: &str) -> &'static str {
    let trimmed = content.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return "json";
    }
    if trimmed.starts_with("<!DOCTYPE") || trimmed.starts_with("<html") {
        return "html";
    }
    if trimmed.starts_with("fn ") || trimmed.starts_with("use ") || trimmed.contains("pub fn") {
        return "rust";
    }
    if trimmed.starts_with("import ") || trimmed.starts_with("from ") || trimmed.contains("def ") {
        return "python";
    }
    if trimmed.starts_with("const ") || trimmed.starts_with("function ") || trimmed.contains("=> {") {
        return "javascript";
    }
    "text"
}

fn get_sessions_index() -> Vec<String> {
    match kv_get(SESSIONS_INDEX_KEY) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    }
}

fn save_sessions_index(ids: &[String]) {
    let json = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    kv_set(SESSIONS_INDEX_KEY, &json);
}

fn get_workspaces_index() -> Vec<String> {
    match kv_get(WORKSPACES_INDEX_KEY) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    }
}

fn get_trajectories_index() -> Vec<String> {
    match kv_get(TRAJECTORIES_INDEX_KEY) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    }
}

fn save_trajectories_index(ids: &[String]) {
    let json = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    kv_set(TRAJECTORIES_INDEX_KEY, &json);
}

fn trajectory_key(id: &str) -> String {
    format!("agent-core:trajectory:{}", id)
}

fn save_agent_trajectory(trajectory: &AgentTrajectory) {
    let json = serde_json::to_string(trajectory).unwrap_or_default();
    kv_set(&trajectory_key(&trajectory.id), &json);
    let mut index = get_trajectories_index();
    if !index.iter().any(|entry| entry == &trajectory.id) {
        index.push(trajectory.id.clone());
        if index.len() > 200 {
            index = index[index.len() - 200..].to_vec();
        }
        save_trajectories_index(&index);
    }
}

fn new_agent_trajectory(agent: &str, task: &str) -> AgentTrajectory {
    let started_at = now_ms();
    AgentTrajectory {
        id: format!(
            "agent-trajectory-{}-{}",
            started_at,
            agent.replace(':', "-")
        ),
        agent: agent.to_string(),
        task: task.to_string(),
        started_at,
        completed_at: 0,
        status: "running".into(),
        steps: Vec::new(),
        tool_names: Vec::new(),
        final_result: String::new(),
        failure: String::new(),
        injected_skill_ids: Vec::new(),
        promotion_result: Value::Null,
        ..AgentTrajectory::default()
    }
}

fn skills_package_candidates(configured: &str) -> Vec<String> {
    let trimmed = configured.trim();
    let mut candidates = Vec::new();
    if trimmed.is_empty() || trimmed == "skills" || trimmed == "skills-runtime" {
        candidates.push("skills".to_string());
        candidates.push("skills-runtime".to_string());
    } else {
        candidates.push(trimmed.to_string());
        candidates.push("skills".to_string());
        candidates.push("skills-runtime".to_string());
    }
    candidates.dedup();
    candidates
}

fn call_skills_ws_action(
    configured: &str,
    action: &str,
    payload: &Value,
) -> Result<String, String> {
    let mut last_error = String::new();
    for candidate in skills_package_candidates(configured) {
        match call_package_ws_action(&candidate, action, payload) {
            Ok(response) => return Ok(response),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

/// artifact 回填:把 implementer 写过的文件路径累加到 KV(key=`artifacts:<session_id>`),
/// 去重。orchestrator 在 execute 推进时按重建的 session_id 读取,存入 task.artifact_refs。
fn record_artifact(session_id: &str, path: &str) {
    let path = path.trim();
    if session_id.trim().is_empty() || path.is_empty() {
        return;
    }
    let key = format!("artifacts:{}", session_id);
    let mut paths: Vec<String> = kv_get(&key)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    if !paths.iter().any(|p| p == path) {
        paths.push(path.to_string());
        kv_set(&key, &serde_json::to_string(&paths).unwrap_or_default());
    }
}

fn append_session_event(session_id: &str, event_type: &str, payload: Value) {
    if session_id.trim().is_empty() {
        return;
    }

    let data = serde_json::json!({
        "session_id": session_id,
        "type": event_type,
        "payload": payload,
    });

    if let Err(error) = call_capability_action(SESSION_EVENTS_CAPABILITY, "append_event", &data) {
        log_warn(&format!(
            "agent-core session event append failed type={} session={} error={}",
            event_type, session_id, error
        ));
    }
}

fn truncate_event_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn tool_args_preview(args: &Value) -> Value {
    match args {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let preview = match value {
                        Value::String(text) if text.chars().count() > 1000 => Value::String(format!(
                            "{}…",
                            truncate_event_text(text, 1000)
                        )),
                        other => other.clone(),
                    };
                    (key.clone(), preview)
                })
                .collect(),
        ),
        other => other.clone(),
    }
}
/// Short-lived cache entry for `retrieve_applicable_evolved_skills`. The
/// `[Evolved skills]` block we splice into the prompt is volatile by design,
/// but if a user fires two messages within seconds the underlying skill DB
/// almost never changes — caching the lookup keeps the dynamic system block
/// byte-stable across closely-spaced turns and avoids a needless RPC per round.
#[derive(Serialize, Deserialize, Default)]
struct EvolvedSkillsCacheEntry {
    context_block: String,
    skill_ids: Vec<String>,
    expires_at_ms: u64,
}

const EVOLVED_SKILLS_CACHE_TTL_MS: u64 = 30_000;

fn evolved_skills_cache_key(agent: &str, query: &str) -> String {
    format!(
        "agent-core:evolved_skills_cache:{}:{}",
        agent,
        fingerprint_str(query)
    )
}

fn retrieve_applicable_evolved_skills(
    skills_package: &str,
    agent: &str,
    query: &str,
) -> RetrievedEvolvedSkills {
    let cache_key = evolved_skills_cache_key(agent, query);
    let now = now_ms();
    if let Some(raw) = kv_get(&cache_key) {
        if let Ok(entry) = serde_json::from_str::<EvolvedSkillsCacheEntry>(&raw) {
            if entry.expires_at_ms > now {
                return RetrievedEvolvedSkills {
                    context_block: entry.context_block,
                    skill_ids: entry.skill_ids,
                };
            }
        }
    }

    let payload = serde_json::json!({
        "agent": agent,
        "query": query,
        "limit": 3,
    });
    let Ok(response) = call_skills_ws_action(skills_package, "retrieve_applicable", &payload)
    else {
        return RetrievedEvolvedSkills::default();
    };
    let Some(data) = parse_plugin_ok_data(&response) else {
        return RetrievedEvolvedSkills::default();
    };
    let context_block = data
        .get("context_block")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let skill_ids = extract_evolved_skill_ids(&data);
    let result = RetrievedEvolvedSkills {
        context_block: context_block.clone(),
        skill_ids: skill_ids.clone(),
    };

    let entry = EvolvedSkillsCacheEntry {
        context_block,
        skill_ids,
        expires_at_ms: now.saturating_add(EVOLVED_SKILLS_CACHE_TTL_MS),
    };
    if let Ok(raw) = serde_json::to_string(&entry) {
        kv_set(&cache_key, &raw);
    }
    result
}

fn has_trajectory_verification_evidence(trajectory: &AgentTrajectory) -> bool {
    trajectory.status == "success" && !trajectory.final_result.trim().is_empty()
}

fn build_trajectory_verification_evidence(trajectory: &AgentTrajectory) -> String {
    if !has_trajectory_verification_evidence(trajectory) {
        return String::new();
    }
    format!(
        "Trajectory completed successfully with final result recorded; steps={}, tools={}.",
        trajectory.steps.len(),
        trajectory.tool_names.len()
    )
}

fn infer_trajectory_promotion_risk(trajectory: &AgentTrajectory) -> String {
    if trajectory.tool_names.is_empty() {
        "low".into()
    } else {
        "medium".into()
    }
}

fn build_trajectory_promotion_payload(trajectory: &AgentTrajectory) -> Value {
    serde_json::json!({
        "agent": trajectory.agent,
        "trajectory_id": trajectory.id,
        "task": trajectory.task,
        "summary": format!("Successful run for {}", trajectory.task.trim()),
        "steps": trajectory.steps,
        "tools": trajectory.tool_names,
        "final_result": trajectory.final_result,
        "success": trajectory.status == "success",
        "verification_passed": has_trajectory_verification_evidence(trajectory),
        "verification_evidence": build_trajectory_verification_evidence(trajectory),
        "auto_activate": false,
        "risk_level": infer_trajectory_promotion_risk(trajectory),
    })
}

fn promote_trajectory_skill(skills_package: &str, trajectory: &mut AgentTrajectory) {
    if trajectory.status != "success" {
        return;
    }
    let payload = build_trajectory_promotion_payload(trajectory);
    trajectory.promotion_result =
        match call_skills_ws_action(skills_package, "crystallize_from_trajectory", &payload) {
            Ok(response) => serde_json::from_str::<Value>(&response).unwrap_or_else(|_| {
                serde_json::json!({
                    "status": "ok",
                    "raw_response": response,
                })
            }),
            Err(error) => {
                log_warn(&format!(
                    "agent-core submit skill promotion failed: {}",
                    error
                ));
                serde_json::json!({ "status": "error", "error": error })
            }
        };
}

fn record_evolved_skill_usage(skills_package: &str, trajectory: &AgentTrajectory, success: bool) {
    for skill_id in &trajectory.injected_skill_ids {
        let payload = serde_json::json!({
            "agent": trajectory.agent,
            "skill_id": skill_id,
            "success": success,
            "note": if success {
                format!("trajectory {} completed successfully", trajectory.id)
            } else {
                format!("trajectory {} failed: {}", trajectory.id, trajectory.failure)
            },
        });
        if let Err(error) = call_skills_ws_action(skills_package, "record_skill_usage", &payload) {
            log_warn(&format!("agent-core record skill usage failed: {}", error));
        }
    }
}

fn record_failure_learning(config: &AgentConfig, trajectory: &AgentTrajectory) {
    let skills_package = non_empty_or(&config.skills_package, "skills").to_string();
    if let Some(skill_id) = trajectory.injected_skill_ids.first() {
        let payload = serde_json::json!({
            "agent": trajectory.agent,
            "skill_id": skill_id,
            "trajectory_id": trajectory.id,
            "reason": trajectory.failure,
            "patch": format!("Review failed trajectory steps and adjust this evolved skill when handling similar task: {}", trajectory.task),
        });
        if let Err(error) =
            call_skills_ws_action(&skills_package, "record_patch_suggestion", &payload)
        {
            log_warn(&format!(
                "agent-core record patch suggestion failed: {}",
                error
            ));
        }
        return;
    }

    let memory_package = non_empty_or(&config.memory_package, "memory").to_string();
    // Skip memory storage if session is in private mode.
    let session_for_privacy = session_id_from_agent_name(&trajectory.agent).unwrap_or_default();
    if kv_get(&format!("private:{}", session_for_privacy)).as_deref() == Some("1") {
        return;
    }
    let payload = serde_json::json!({
        "agent": trajectory.agent,
        "key": format!("experience:{}", trajectory.id),
        "content": format!("Agent run failed for task: {}\nFailure: {}\nSteps: {}", trajectory.task, trajectory.failure, trajectory.steps.join("; ")),
        "category": "experience",
        "tags": ["agent-core", "failure", "trajectory"],
        "metadata": {
            "trajectory_id": trajectory.id,
            "status": trajectory.status,
            "started_at": trajectory.started_at,
            "completed_at": trajectory.completed_at,
        },
        "timestamp": trajectory.completed_at,
    });
    if let Err(error) = call_package_ws_action(&memory_package, "store", &payload) {
        log_warn(&format!(
            "agent-core store failure memory failed: {}",
            error
        ));
    }
}

fn mark_agent_trajectory_success(trajectory: &mut AgentTrajectory, final_result: &str) {
    trajectory.completed_at = now_ms();
    trajectory.status = "success".into();
    trajectory.final_result = final_result.to_string();
    trajectory.failure.clear();
}

fn mark_agent_trajectory_failure(trajectory: &mut AgentTrajectory, failure: &str) {
    trajectory.completed_at = now_ms();
    trajectory.status = "failure".into();
    trajectory.failure = failure.to_string();
}

fn finish_agent_trajectory_success(
    config: &AgentConfig,
    trajectory: &mut AgentTrajectory,
    final_result: &str,
) {
    mark_agent_trajectory_success(trajectory, final_result);
    save_agent_trajectory(trajectory);
    let skills_package = non_empty_or(&config.skills_package, "skills").to_string();
    record_evolved_skill_usage(&skills_package, trajectory, true);
    promote_trajectory_skill(&skills_package, trajectory);
    save_agent_trajectory(trajectory);
}

fn finish_agent_trajectory_failure(
    config: &AgentConfig,
    trajectory: &mut AgentTrajectory,
    failure: &str,
) {
    mark_agent_trajectory_failure(trajectory, failure);
    save_agent_trajectory(trajectory);
    let skills_package = non_empty_or(&config.skills_package, "skills").to_string();
    record_evolved_skill_usage(&skills_package, trajectory, false);
    record_failure_learning(config, trajectory);
}
fn save_workspaces_index(ids: &[String]) {
    let json = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    kv_set(WORKSPACES_INDEX_KEY, &json);
}

fn workspace_key(id: &str) -> String {
    format!("agent-core:workspace:{}", id)
}

fn load_workspace(id: &str) -> Option<WorkspaceRecord> {
    kv_get(&workspace_key(id)).and_then(|json| serde_json::from_str(&json).ok())
}

fn save_workspace(workspace: &WorkspaceRecord) {
    let json = serde_json::to_string(workspace).unwrap_or_default();
    kv_set(&workspace_key(&workspace.id), &json);
}

fn session_key(id: &str) -> String {
    format!("agent-core:session:{}", id)
}

fn session_messages_key(id: &str) -> String {
    format!("agent-core:session_messages:{}", id)
}

fn load_session(id: &str) -> Option<SessionRecord> {
    kv_get(&session_key(id)).and_then(|json| serde_json::from_str(&json).ok())
}

/// 会话的工作区根:优先用 session.workspace_root,为空则回退默认
/// `data/workspaces/<session_id>`(相对 core 进程 cwd,即项目根下 data/)。
/// 这样未显式设工作区的会话也不再把文件散落到项目根,而是集中到 data/workspaces。
fn session_workspace_root(session_id: &str) -> String {
    if let Some(session) = load_session(session_id) {
        let ws = session.workspace_root.trim();
        if !ws.is_empty() {
            return ws.to_string();
        }
    }
    default_workspace_root(session_id)
}

fn default_workspace_root(session_id: &str) -> String {
    let mut sanitized = String::with_capacity(session_id.len());
    for ch in session_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }
    format!("data\\workspaces\\{}", sanitized)
}

fn save_session(session: &SessionRecord) {
    let json = serde_json::to_string(session).unwrap_or_default();
    kv_set(&session_key(&session.id), &json);
}

fn get_session_messages(session_id: &str) -> Vec<SessionMessageRecord> {
    match kv_get(&session_messages_key(session_id)) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    }
}

fn save_session_messages(session_id: &str, messages: &[SessionMessageRecord]) {
    let json = serde_json::to_string(messages).unwrap_or_else(|_| "[]".into());
    kv_set(&session_messages_key(session_id), &json);
}

fn non_empty_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    }
}

fn parse_plugin_data(raw: &str, key: &str) -> Option<Value> {
    if raw.trim().is_empty() {
        return None;
    }

    let parsed = serde_json::from_str::<Value>(raw).ok()?;
    parsed.get("data").and_then(|data| data.get(key)).cloned()
}

fn session_agent_name(session_id: &str) -> String {
    let mut sanitized = String::with_capacity(session_id.len());
    for ch in session_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            sanitized.push(ch.to_ascii_lowercase());
        } else {
            sanitized.push('-');
        }
    }
    format!("weft-session-{}", sanitized)
}

fn now_ms() -> u64 {
    weft_package_sdk::now_ms()
}

fn js_runtime_tool_names() -> Vec<&'static str> {
    vec![
        "web_search",
        "websearch",
        "search_web",
        "fetch_url",
        "web_fetch",
    ]
}

fn unwrap_js_runtime_tool_response(raw: &str) -> Option<String> {
    let mut payload: Value = serde_json::from_str(raw).ok()?;
    if payload.get("status").and_then(|value| value.as_str()) == Some("ok") {
        return serde_json::to_string(&payload).ok();
    }
    if payload.get("ok").and_then(|value| value.as_bool()) == Some(true) {
        let result = payload.get("result")?;
        if result.get("status").and_then(|value| value.as_str()) == Some("ok") {
            return serde_json::to_string(result).ok();
        }
    }
    if payload.get("status").and_then(|value| value.as_str()) == Some("executed") {
        if let Some(response) = payload.get_mut("response") {
            if response.get("status").and_then(|value| value.as_str()) == Some("ok") {
                return serde_json::to_string(response).ok();
            }
            if response.get("ok").and_then(|value| value.as_bool()) == Some(true) {
                if let Some(result) = response.get("result") {
                    if result.get("status").and_then(|value| value.as_str()) == Some("ok") {
                        return serde_json::to_string(result).ok();
                    }
                }
            }
        }
    }
    None
}

fn normalize_js_runtime_tool_name(tool_name: &str) -> String {
    match tool_name.trim() {
        "websearch" | "search_web" => "web_search".to_string(),
        "web_fetch" => "fetch_url".to_string(),
        other => other.to_string(),
    }
}

fn execute_js_runtime_tool(agent_name: &str, tool_name: &str, args: &Value) -> Option<String> {
    let normalized_tool = normalize_js_runtime_tool_name(tool_name);
    if !js_runtime_tool_names()
        .iter()
        .any(|entry| *entry == normalized_tool.as_str())
    {
        return None;
    }
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .or_else(|| args.get("q").and_then(Value::as_str))
        .unwrap_or("")
        .trim();
    let mut forwarded_args = args.clone();
    if normalized_tool == "web_search" {
        if let Some(args_object) = forwarded_args.as_object_mut() {
            if !args_object.contains_key("provider") {
                args_object.insert("provider".to_string(), Value::String("exa".to_string()));
            }
            if !args_object.contains_key("use_exa") {
                args_object.insert("use_exa".to_string(), Value::Bool(true));
            }
        }
    }
    let payload = serde_json::json!({
        "agent": agent_name,
        "tool": normalized_tool,
        "args": forwarded_args,
        "query_b64": base64::engine::general_purpose::STANDARD.encode(query.as_bytes()),
    });
    let action = if normalized_tool == "fetch_url" {
        "fetch_url"
    } else {
        "web_search"
    };
    let service_payload = serde_json::json!({
        "action": action,
        "data": payload,
    })
    .to_string();
    let result = match call_package(
        "js-extension-runtime",
        "handle_ws_message",
        &service_payload,
    ) {
        Ok(result) => result,
        Err(error) => {
            log_warn(&format!(
                "agent-core js-runtime tool dispatch failed tool={}: {}",
                normalized_tool, error
            ));
            return Some(
                PackageResult::ok(serde_json::json!({
                    "heading": "",
                    "query": args.get("query").and_then(Value::as_str).unwrap_or("").trim(),
                    "results": [],
                    "links": [],
                    "provider": "js-runtime-unavailable",
                    "source": "JS runtime unavailable",
                    "summary": format!(
                        "JS runtime web search is unavailable: {}",
                        error
                    ),
                }))
                .to_json(),
            );
        }
    };
    match unwrap_js_runtime_tool_response(&result) {
        Some(result) => Some(result),
        None => {
            log_warn(&format!(
                "agent-core js-runtime tool response rejected tool={} response={}",
                normalized_tool,
                result.chars().take(500).collect::<String>()
            ));
            None
        }
    }
}

fn execute_skill_action(
    skills_package: &str,
    agent_name: &str,
    tool_name: &str,
    args: Value,
    workspace_root: &str,
) -> String {
    if let Some(result) = execute_js_runtime_tool(agent_name, tool_name, &args) {
        return result;
    }

    // 注入会话工作区根:tool-runtime-core 是隔离 wasm,读不到 agent-core 的 session,
    // 只能经 args 透传。fs_write/read/list、shell_exec 据此把相对路径落到工作区,
    // 不再散落进程 cwd(项目根)。
    let mut forwarded = args;
    if !workspace_root.trim().is_empty() {
        if let Some(obj) = forwarded.as_object_mut() {
            obj.insert(
                "__workspace_root".to_string(),
                Value::String(workspace_root.trim().to_string()),
            );
        }
    }

    call_skills_ws_action(
        skills_package,
        "execute_tool",
        &serde_json::json!({
            "agent": agent_name,
            "tool": tool_name,
            "args": forwarded,
        }),
    )
    .unwrap_or_else(|error| format!(r#"{{"error":"{}"}}"#, error))
}

fn append_debug_log(message: &str) {
    let prefix = format!("[{}] ", now_ms());
    let next_line = format!("{}{}", prefix, message);
    let existing = kv_get("agent-core:debug:trace").unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(|line| line.to_string()).collect();
    lines.push(next_line);
    if lines.len() > 48 {
        lines = lines[lines.len() - 48..].to_vec();
    }
    let trace = lines.join("\n");
    kv_set("agent-core:debug:trace", &trace);
    kv_set("agent-core:debug:last", message);

    let log_path = "./data/agent-core-debug.log";
    let mut file_trace = read_file(log_path).unwrap_or_default();
    if !file_trace.is_empty() && !file_trace.ends_with('\n') {
        file_trace.push('\n');
    }
    file_trace.push_str(&lines.last().cloned().unwrap_or_default());
    file_trace.push('\n');
    write_file(log_path, &file_trace);
}

fn decode_transport_text(raw: &str, raw_b64: &str) -> String {
    let encoded = raw_b64.trim();
    if encoded.is_empty() {
        return raw.to_string();
    }

    let bytes = match base64::engine::general_purpose::STANDARD.decode(encoded) {
        Ok(value) => value,
        Err(_) => return raw.to_string(),
    };

    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => raw.to_string(),
    }
}

fn session_id_from_agent_name(agent_name: &str) -> Option<String> {
    agent_name
        .strip_prefix("weft-session-")
        .map(|value| value.to_string())
}

fn derive_session_title(history: &[Value], fallback_label: &str) -> String {
    let first_user = history
        .iter()
        .find(|message| message.get("role").and_then(|value| value.as_str()) == Some("user"))
        .and_then(|message| message.get("content").and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(content) = first_user {
        let mut title = content.replace('\n', " ");
        if title.chars().count() > 32 {
            title = title.chars().take(32).collect::<String>();
            title.push_str("...");
        }
        return title;
    }

    let label = fallback_label.trim();
    if !label.is_empty() {
        return label.to_string();
    }

    "新对话".into()
}

fn legacy_history_to_session_messages(
    session_id: &str,
    history: &[Value],
    created_at: u64,
) -> Vec<SessionMessageRecord> {
    history
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
            let role = message.get("role").and_then(|value| value.as_str())?.trim();
            let content = message
                .get("content")
                .and_then(|value| value.as_str())?
                .trim();
            if role.is_empty() || content.is_empty() {
                return None;
            }

            Some(SessionMessageRecord {
                id: format!("legacy-{}-{}", session_id, index),
                session_id: session_id.to_string(),
                role: role.to_string(),
                content: content.to_string(),
                tool_name: None,
                tool_args: None,
                tool_status: None,
                streaming: false,
                timestamp: created_at.saturating_add(index as u64),
            })
        })
        .collect()
}

fn ensure_session_records_from_legacy_agents() {
    let mut sessions_index = get_sessions_index();
    let agent_names = get_agents_index();
    let mut index_changed = false;

    for agent_name in agent_names {
        let Some(session_id) = session_id_from_agent_name(&agent_name) else {
            continue;
        };

        if load_session(&session_id).is_some() {
            continue;
        }

        let mut agent = load_agent(&agent_name).unwrap_or_else(|| {
            let mut config = default_session_agent_config(&session_id);
            config.name = agent_name.clone();
            config
        });
        agent.name = agent_name.clone();

        let history = get_history(&agent_name);
        let created_at = now_ms();
        let messages = legacy_history_to_session_messages(&session_id, &history, created_at);
        let updated_at = messages
            .last()
            .map(|message| message.timestamp)
            .unwrap_or(created_at);

        let session = SessionRecord {
            id: session_id.clone(),
            title: derive_session_title(&history, &agent.label),
            workspace_id: String::new(),
            workspace_root: String::new(),
            persistent: 0,
            created_at,
            updated_at,
            agent_name: agent_name.clone(),
            agent,
        };

        save_session(&session);
        if !messages.is_empty() {
            save_session_messages(&session_id, &messages);
        }

        if !sessions_index.iter().any(|entry| entry == &session_id) {
            sessions_index.push(session_id);
            index_changed = true;
        }
    }

    if index_changed {
        save_sessions_index(&sessions_index);
    }
}

fn default_session_agent_config(session_id: &str) -> AgentConfig {
    let agent_name = session_agent_name(session_id);
    let mut config = default_agent_config();
    config.name = agent_name;
    config.label = format!("Session {}", session_id.chars().take(8).collect::<String>());
    if config.role.trim().is_empty() {
        config.role = "session_agent".into();
    }
    config
}

fn merge_session_agent_config(base: &mut AgentConfig, input: &SessionAgentInput) {
    if !input.label.trim().is_empty() {
        base.label = input.label.trim().to_string();
    }
    if !input.role.trim().is_empty() {
        base.role = input.role.trim().to_string();
    }
    if !input.model.trim().is_empty() {
        base.model = input.model.trim().to_string();
    }
    if let Some(temperature) = input.temperature {
        base.temperature = temperature;
    }
    if !input.system_prompt.trim().is_empty() {
        base.system_prompt = input.system_prompt.clone();
    }
    if !input.skills.is_empty() {
        base.skills = input.skills.clone();
    }
    if !input.channels.is_empty() {
        base.channels = input.channels.clone();
    }
    if !input.provider.trim().is_empty() {
        base.provider = input.provider.trim().to_string();
    }
    if !input.memory_package.trim().is_empty() {
        base.memory_package = input.memory_package.trim().to_string();
    }
    if !input.skills_package.trim().is_empty() {
        base.skills_package = input.skills_package.trim().to_string();
    }
    if !input.channels_package.trim().is_empty() {
        base.channels_package = input.channels_package.trim().to_string();
    }
    if !input.completion_endpoint.trim().is_empty() {
        base.completion_endpoint = input.completion_endpoint.trim().to_string();
    }
}

fn ensure_agent_index_contains(name: &str) {
    let mut index = get_agents_index();
    if !index.iter().any(|entry| entry == name) {
        index.push(name.to_string());
        save_agents_index(&index);
    }
}

fn sync_session_agent(session: &SessionRecord) {
    let mut config = session.agent.clone();
    config.name = session.agent_name.clone();
    if config.label.trim().is_empty() {
        config.label = format!("Session {}", session.id.chars().take(8).collect::<String>());
    }
    if config.role.trim().is_empty() {
        config.role = "session_agent".into();
    }

    save_agent(&config);
    ensure_agent_index_contains(&config.name);

    let skills_package = non_empty_or(&config.skills_package, "skills");
    let input = serde_json::json!({
        "agent": config.name,
        "skills": config.skills,
    })
    .to_string();
    let _ = call_package(skills_package, "set_for_agent", &input);
}

fn upsert_session_message_record(input: SaveSessionMessageInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let mut session = match load_session(&input.session_id) {
        Some(session) => session,
        None => return PackageResult::err(format!("session '{}' not found", input.session_id)),
    };

    let mut messages = get_session_messages(&input.session_id);
    let content = decode_transport_text(&input.content, &input.content_b64);
    let next = SessionMessageRecord {
        id: input.id,
        session_id: input.session_id.clone(),
        role: input.role,
        content,
        tool_name: if input.tool_name.trim().is_empty() {
            None
        } else {
            Some(input.tool_name)
        },
        tool_args: if input.tool_args.trim().is_empty() {
            None
        } else {
            Some(input.tool_args)
        },
        tool_status: if input.tool_status.trim().is_empty() {
            None
        } else {
            Some(input.tool_status)
        },
        streaming: input.streaming,
        timestamp: input.timestamp,
    };

    if let Some(existing) = messages.iter_mut().find(|message| message.id == next.id) {
        *existing = next;
    } else {
        messages.push(next);
    }

    messages.sort_by_key(|message| message.timestamp);
    save_session_messages(&input.session_id, &messages);

    session.updated_at = session.updated_at.max(input.timestamp);
    save_session(&session);

    PackageResult::ok_empty()
}

fn build_session_message_id(session_id: &str, role: &str, timestamp: u64) -> String {
    format!("session-{}-{}-{}", session_id, role, timestamp)
}

fn find_reusable_user_message(messages: &[SessionMessageRecord], content: &str) -> Option<usize> {
    for index in (0..messages.len()).rev() {
        let message = &messages[index];
        match message.role.as_str() {
            "assistant" | "companion"
                if !message.streaming || !message.content.trim().is_empty() =>
            {
                return None
            }
            "user" if message.content == content => return Some(index),
            "user" => return None,
            _ => {}
        }
    }

    None
}

fn find_reusable_assistant_placeholder(
    messages: &[SessionMessageRecord],
    user_index: usize,
) -> Option<usize> {
    messages
        .iter()
        .enumerate()
        .skip(user_index.saturating_add(1))
        .find(|(_, message)| {
            message.role == "assistant" && message.streaming && message.content.trim().is_empty()
        })
        .map(|(index, _)| index)
}

// Rough token estimate: 4 chars ≈ 1 token (good enough for budget gating).
fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

/// Stable 64-bit hash of a string, returned as 16-char hex. Used to fingerprint
/// the immutable prefix (system prompt + tool catalog) so we can detect when
/// something mutates it across rounds — which would destroy upstream prompt-cache.
fn fingerprint_str(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Serialize `value` to JSON with a canonical byte representation: object keys
/// are emitted in sorted order. Relies on `serde_json::Map` being `BTreeMap`-
/// backed (i.e. the `preserve_order` feature is NOT enabled across the workspace,
/// see `cargo tree -i serde_json -e features`). If that ever flips, the fingerprint
/// test below would break — that's the failure signal.
fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Total token budget the session-history block is allowed to occupy. Beyond this
/// `fold_session_if_needed` rewrites the head section into a single summary message.
const SESSION_HISTORY_TOKEN_BUDGET: usize = 16_000;
/// Fold trigger ratio (Reasonix HISTORY_FOLD_THRESHOLD). When tokens / budget exceeds
/// this we synthesise a summary; below it the log is left untouched (append-only).
const SESSION_FOLD_TRIGGER_RATIO: f64 = 0.75;
/// Tail kept verbatim after a fold, as a fraction of the total budget. Mirrors
/// `HISTORY_FOLD_TAIL_FRACTION` in Reasonix `context-manager.ts`.
const SESSION_FOLD_TAIL_RATIO: f64 = 0.20;
/// Marker prepended to the synthesised summary so future folds can recognise it
/// and skip re-summarising an already-folded head.
const SESSION_FOLD_MARKER: &str = "[FOLD SUMMARY]\n";
/// Minimum number of new chat messages that must appear after the last fold
/// before another fold is permitted. Prevents fold churn — without this, two
/// rounds in a row could each trip the 75% trigger and rewrite the summary
/// (with subtly different bytes), invalidating the cached prefix every turn.
const SESSION_FOLD_COOLDOWN_MESSAGES: usize = 8;

fn fold_meta_key(session_id: &str) -> String {
    format!("agent-core:fold_meta:{}", session_id)
}

/// KV key holding the prefix fingerprint of the most recent turn for a session.
/// Lets us detect prefix drift *across* user turns (e.g. someone edited the
/// agent system prompt overnight) — the per-turn RefCell in `run_agent_completion`
/// only catches drift *within* one turn's tool-call rounds.
fn session_prefix_fp_key(session_id: &str) -> String {
    format!("agent-core:session_prefix_fp:{}", session_id)
}

fn load_session_prefix_fp(session_id: &str) -> String {
    kv_get(&session_prefix_fp_key(session_id))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn save_session_prefix_fp(session_id: &str, fingerprint: &str) {
    if fingerprint.is_empty() {
        return;
    }
    kv_set(&session_prefix_fp_key(session_id), fingerprint);
}

/// Persisted fold history for a session. `last_folded_total` is the message
/// count at the moment we wrote the summary; subsequent fold calls require
/// `current_count - last_folded_total >= SESSION_FOLD_COOLDOWN_MESSAGES`.
#[derive(Serialize, Deserialize, Default)]
struct SessionFoldMeta {
    #[serde(default)]
    last_folded_total: usize,
}

fn load_fold_meta(session_id: &str) -> SessionFoldMeta {
    match kv_get(&fold_meta_key(session_id)) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => SessionFoldMeta::default(),
    }
}

fn save_fold_meta(session_id: &str, meta: &SessionFoldMeta) {
    let json = serde_json::to_string(meta).unwrap_or_else(|_| "{}".into());
    kv_set(&fold_meta_key(session_id), &json);
}

/// Read-only history view used to feed `build_completion_request`. Returns the
/// session messages as-is (in append order) — fold mutation happens in
/// `fold_session_if_needed` before this is called, so the prefix bytes stay
/// stable across turns and the upstream prompt cache can hit.
fn build_session_context(session_id: &str, limit: usize) -> Vec<Value> {
    let limit = limit.max(1);

    let messages: Vec<Value> = get_session_messages(session_id)
        .into_iter()
        .filter(|message| {
            let role = message.role.trim();
            if role != "user" && role != "assistant" && role != "companion" {
                return false;
            }
            let content = if role == "user" {
                message.content.trim().to_string()
            } else {
                visible_assistant_reply(&message.content)
            };
            !content.trim().is_empty()
        })
        .map(|message| {
            serde_json::json!({
                "role": if message.role == "user" { "user" } else { "assistant" },
                "content": if message.role == "user" {
                    message.content
                } else {
                    visible_assistant_reply(&message.content)
                },
            })
        })
        .collect();

    // Hard cap on message count as a safety net (the real budget enforcement is
    // `fold_session_if_needed`). Cap is intentionally generous so a normal session
    // never trips it before fold runs.
    if messages.len() > limit {
        messages[messages.len() - limit..].to_vec()
    } else {
        messages
    }
}

fn session_message_to_context_value(message: &SessionMessageRecord) -> Option<Value> {
    let role = message.role.trim();
    if role != "user" && role != "assistant" && role != "companion" {
        return None;
    }
    let normalized_role = if role == "user" { "user" } else { "assistant" };
    let content = if role == "user" {
        message.content.clone()
    } else {
        visible_assistant_reply(&message.content)
    };
    if content.trim().is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "role": normalized_role,
        "content": content,
    }))
}

/// If the session message history exceeds `SESSION_FOLD_TRIGGER_RATIO * SESSION_HISTORY_TOKEN_BUDGET`,
/// summarise the head into a single synthetic assistant message and rewrite the
/// session log so subsequent turns send byte-identical history (append-only after the fold).
///
/// This mirrors Reasonix `ContextManager.fold` (Pillar 1 of cache-first loop):
/// - boundary always lands on a user message, never mid tool turn;
/// - head is replaced by a single `[FOLD SUMMARY]` assistant message;
/// - the summarizer call uses the same agent system prompt so it costs one cache miss
///   then the new prefix is reusable for every turn that follows.
fn fold_session_if_needed(session_id: &str, agent: &AgentConfig) {
    if session_id.trim().is_empty() {
        return;
    }
    let raw_messages = get_session_messages(session_id);
    if raw_messages.len() < 6 {
        return;
    }

    // Build (role, content) pairs in append order. Skip empty / non-chat rows.
    let pairs: Vec<(String, String, usize)> = raw_messages
        .iter()
        .enumerate()
        .filter_map(|(idx, message)| {
            let value = session_message_to_context_value(message)?;
            let role = value
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let content = value
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Some((role, content, idx))
        })
        .collect();
    if pairs.len() < 4 {
        return;
    }

    // Cooldown: refuse to fold again unless N new messages have appeared since
    // the previous fold. Otherwise two adjacent rounds that both flirt with the
    // 75% threshold would each rewrite the summary, shifting prefix bytes turn
    // after turn — the exact failure mode P0 was meant to avoid.
    let total_tokens: usize = pairs
        .iter()
        .map(|(_, content, _)| estimate_tokens(content))
        .sum();
    let budget = SESSION_HISTORY_TOKEN_BUDGET;
    let trigger = (budget as f64 * SESSION_FOLD_TRIGGER_RATIO) as usize;
    if total_tokens < trigger {
        return;
    }
    // 紧急折叠:已超 95% 预算时,即便在冷却期也必须折叠,否则下一轮会溢出
    // 上下文窗口。正常情况遵守冷却,避免反复改写摘要破坏 prefix-cache。
    let emergency = total_tokens >= (budget as f64 * 0.95) as usize;
    let fold_meta = load_fold_meta(session_id);
    if !emergency
        && fold_meta.last_folded_total > 0
        && raw_messages.len() < fold_meta.last_folded_total + SESSION_FOLD_COOLDOWN_MESSAGES
    {
        return;
    }

    // Walk from newest to oldest collecting tail tokens; boundary is the first
    // user message whose inclusion still keeps tail under the tail budget.
    let tail_budget = (budget as f64 * SESSION_FOLD_TAIL_RATIO) as usize;
    let mut tail_tokens: usize = 0;
    let mut boundary_pair_index: Option<usize> = None;
    for i in (0..pairs.len()).rev() {
        let (role, content, _) = &pairs[i];
        let tokens = estimate_tokens(content);
        if tail_tokens + tokens > tail_budget && role == "user" && boundary_pair_index.is_some() {
            break;
        }
        tail_tokens += tokens;
        if role == "user" {
            boundary_pair_index = Some(i);
        }
    }
    let Some(boundary) = boundary_pair_index else {
        return;
    };
    if boundary == 0 {
        return;
    }
    // Already folded? The first kept-head message will be a fold summary; do not re-fold.
    if pairs[0].1.starts_with(SESSION_FOLD_MARKER) {
        return;
    }

    // Head = pairs[0..boundary], tail = pairs[boundary..]. Synthesize summary of head.
    let head_messages: Vec<Value> = pairs[..boundary]
        .iter()
        .map(|(role, content, _)| {
            serde_json::json!({
                "role": role,
                "content": content,
            })
        })
        .collect();

    let summary = match summarize_for_fold(agent, &head_messages) {
        Some(text) if !text.trim().is_empty() => text,
        _ => {
            log_warn("agent-core fold skipped: summarizer produced empty result");
            return;
        }
    };

    let head_raw_indices: std::collections::HashSet<usize> =
        pairs[..boundary].iter().map(|(_, _, idx)| *idx).collect();
    let earliest_head_timestamp = raw_messages
        .iter()
        .enumerate()
        .filter(|(idx, _)| head_raw_indices.contains(idx))
        .map(|(_, message)| message.timestamp)
        .min()
        .unwrap_or_else(now_ms);

    let summary_record = SessionMessageRecord {
        id: build_session_message_id(session_id, "fold-summary", earliest_head_timestamp),
        session_id: session_id.to_string(),
        role: "assistant".into(),
        content: format!("{}{}", SESSION_FOLD_MARKER, summary.trim()),
        tool_name: None,
        tool_args: None,
        tool_status: None,
        streaming: false,
        timestamp: earliest_head_timestamp,
    };

    let mut next_messages: Vec<SessionMessageRecord> = Vec::with_capacity(raw_messages.len());
    next_messages.push(summary_record);
    for (idx, message) in raw_messages.into_iter().enumerate() {
        if head_raw_indices.contains(&idx) {
            continue;
        }
        next_messages.push(message);
    }
    next_messages.sort_by_key(|message| message.timestamp);
    save_session_messages(session_id, &next_messages);
    save_fold_meta(
        session_id,
        &SessionFoldMeta {
            last_folded_total: next_messages.len(),
        },
    );

    log_info(&format!(
        "agent-core fold applied session={} head_msgs={} tail_msgs={} approx_tokens_before={}",
        session_id,
        boundary,
        pairs.len() - boundary,
        total_tokens
    ));
}

fn summarize_for_fold(agent: &AgentConfig, head_messages: &[Value]) -> Option<String> {
    if head_messages.is_empty() {
        return None;
    }

    let mut messages: Vec<Value> = Vec::with_capacity(head_messages.len() + 2);
    let system_prompt = if agent.system_prompt.trim().is_empty() {
        "You compress prior conversation into a self-contained recap.".to_string()
    } else {
        agent.system_prompt.clone()
    };
    messages.push(serde_json::json!({"role": "system", "content": system_prompt}));
    for message in head_messages {
        messages.push(message.clone());
    }
    messages.push(serde_json::json!({
        "role": "user",
        "content":
            "Summarize the conversation above as one self-contained prose recap. \
             Preserve the original objective (never paraphrase away negative constraints \
             like 'do NOT do X'), all 'do not' / 'never' / 'avoid' instructions, decisions \
             reached, files inspected or modified, tool results still relevant, and any \
             open todos. Skip turn-by-turn play-by-play. Output plain prose only — no tool \
             calls, no markdown headings, no SEARCH/REPLACE blocks."
    }));

    let body = serde_json::json!({
        "model": agent.model,
        "messages": messages,
        "temperature": 0.0,
    });
    if !agent.provider.trim().is_empty() {
        // Best-effort: keep summarizer on the same provider as the agent so cache
        // pricing stays predictable. Not a correctness issue if it falls back.
        let _ = agent.provider.trim();
    }
    let body_text = serde_json::to_string(&body).ok()?;
    let resolved = resolve_completion_endpoint(&agent.completion_endpoint);
    if resolved.trim().is_empty() {
        return None;
    }
    let response_text =
        weft_package_sdk::chat_completion("agent-core/fold-summary", &resolved, &body_text)
            .ok()?;
    let parsed: Value = serde_json::from_str(&response_text).ok()?;
    let text = parsed
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .unwrap_or_default();
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn do_get_agents() -> PackageResult {
    write_file("./data/agent-core-last-ws.txt", "stage=get_agents:start");
    let names = get_agents_index();
    write_file(
        "./data/agent-core-last-ws.txt",
        &format!("stage=get_agents:index_loaded count={}", names.len()),
    );
    let agents: Vec<Value> = names
        .iter()
        .filter_map(|name| load_agent(name))
        .map(|agent| {
            serde_json::json!({
                "name": agent.name,
                "label": agent.label,
                "role": agent.role,
                "model": agent.model,
                "provider": agent.provider,
                "memory_package": agent.memory_package,
                "skills_package": agent.skills_package,
                "channels_package": agent.channels_package,
                "completion_endpoint": agent.completion_endpoint,
            })
        })
        .collect();
    write_file(
        "./data/agent-core-last-ws.txt",
        &format!("stage=get_agents:built count={}", agents.len()),
    );

    PackageResult::ok(serde_json::json!({"agents": agents}))
}

fn do_create_agent(config: &AgentConfig) -> PackageResult {
    if config.name.trim().is_empty() {
        return PackageResult::err("missing agent name");
    }

    let mut normalized = config.clone();
    let endpoint = normalized.completion_endpoint.trim();
    if endpoint.is_empty()
        || endpoint.starts_with("host://weft-core/")
        || endpoint.contains("/v1/chat/completions")
    {
        normalized.completion_endpoint.clear();
    }

    save_agent(&normalized);
    ensure_agent_index_contains(&normalized.name);

    let skills_package = non_empty_or(&normalized.skills_package, "skills");
    let input = serde_json::json!({
        "agent": normalized.name,
        "skills": normalized.skills,
    })
    .to_string();
    let _ = call_package(skills_package, "set_for_agent", &input);

    PackageResult::ok(serde_json::json!({"name": normalized.name}))
}

fn do_delete_agent(name: &str) -> PackageResult {
    if name.trim().is_empty() {
        return PackageResult::err("missing agent name");
    }

    kv_set(&agent_config_key(name), "");
    kv_set(&history_key(name), "");

    let mut index = get_agents_index();
    index.retain(|entry| entry != name);
    save_agents_index(&index);

    PackageResult::ok_empty()
}

fn delegate_request_requires_real_action(delegate_request: Option<&DelegateRequestInput>) -> bool {
    delegate_request
        .map(|request| request.must_act)
        .unwrap_or(false)
}

fn build_delegate_contract_block(
    delegate_request: Option<&DelegateRequestInput>,
) -> Option<String> {
    let request = delegate_request?;
    if !request.must_act {
        return None;
    }

    let payload = serde_json::json!({
        "reason": request.reason,
        "latest_user_query": request.latest_user_query,
        "visible_history": request.visible_history,
        "session_context": request.session_context,
        "runtime_context": request.runtime_context,
        "skill_refs": request.skill_refs,
        "action_refs": request.action_refs,
        "event": request.event,
    });

    Some(format!(
        "[Delegated action contract]\nThis request was already delegated by the companion layer because it requires real action.\nYou must perform real tool calls and must not answer from memory or refuse.\nThe companion reason is already a resolved action brief; treat it as authoritative task intent.\nDo not ask the user to repeat omitted context; use the provided delegated context.\nDelegated context:\n{}",
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    ))
}

fn build_mode_enum(
    tool_names: &[String],
    delegate_request: Option<&DelegateRequestInput>,
) -> Value {
    if tool_names.is_empty() {
        serde_json::json!(["reply"])
    } else if delegate_request_requires_real_action(delegate_request) {
        serde_json::json!(["tool"])
    } else {
        serde_json::json!(["reply", "tool"])
    }
}

fn build_tool_call_min_items(delegate_request: Option<&DelegateRequestInput>) -> u64 {
    if delegate_request_requires_real_action(delegate_request) {
        1
    } else {
        0
    }
}

fn include_auxiliary_planning_context(delegate_request: Option<&DelegateRequestInput>) -> bool {
    !delegate_request_requires_real_action(delegate_request)
}

fn trim_tool_description(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<&str>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    let mut shortened = normalized.chars().take(max_chars).collect::<String>();
    shortened.push_str("...");
    shortened
}

fn summarize_tool_parameters(parameters: &Value) -> String {
    let mut required = parameters
        .get("required")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|entry| entry.trim().to_string())
                .filter(|entry| !entry.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    // Canonicalize: JSON Schema treats `required` as a set, so its array order
    // is not semantic — but if we echo it verbatim the prefix bytes shift when
    // upstream tool specs re-order it. Sort here so the catalog stays byte-stable.
    required.sort();
    let properties = parameters
        .get("properties")
        .and_then(Value::as_object)
        .map(|items| {
            items
                .keys()
                .map(|entry| entry.trim().to_string())
                .filter(|entry| !entry.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let optional = properties
        .into_iter()
        .filter(|entry| {
            !required
                .iter()
                .any(|required_entry| required_entry == entry)
        })
        .take(6)
        .collect::<Vec<String>>();

    let mut parts = Vec::new();
    if !required.is_empty() {
        parts.push(format!("required: {}", required.join(", ")));
    }
    if !optional.is_empty() {
        parts.push(format!("optional: {}", optional.join(", ")));
    }

    parts.join(" | ")
}

fn build_tool_catalog_prompt(
    tools: &[Value],
    delegate_request: Option<&DelegateRequestInput>,
) -> String {
    if delegate_request_requires_real_action(delegate_request) {
        return tools
            .iter()
            .filter_map(|entry| entry.get("function"))
            .filter_map(|tool| {
                let name = tool.get("name").and_then(Value::as_str)?.trim().to_string();
                if name.is_empty() {
                    return None;
                }

                let description = trim_tool_description(
                    tool.get("description")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    220,
                );
                let parameter_summary = summarize_tool_parameters(
                    &tool.get("parameters").cloned().unwrap_or(Value::Null),
                );

                let mut line = format!("- {}", name);
                if !description.is_empty() {
                    line.push_str(": ");
                    line.push_str(&description);
                }
                if !parameter_summary.is_empty() {
                    line.push_str(" (");
                    line.push_str(&parameter_summary);
                    line.push(')');
                }
                Some(line)
            })
            .collect::<Vec<String>>()
            .join("\n");
    }

    let tool_catalog = tools
        .iter()
        .filter_map(|entry| entry.get("function"))
        .map(|tool| serde_json::json!({
            "name": tool.get("name").and_then(|value| value.as_str()).unwrap_or(""),
            "description": tool.get("description").and_then(|value| value.as_str()).unwrap_or(""),
            "parameters": tool.get("parameters").cloned().unwrap_or(Value::Null),
        }))
        .collect::<Vec<Value>>();

    // canonical_json_string instead of to_string_pretty: pretty-printing adds
    // whitespace that depends on the serializer's spacing rules; canonical form
    // (compact, sorted keys via BTreeMap-backed serde_json::Map) is byte-stable
    // and feeds the prefix-cache fingerprint cleanly.
    canonical_json_string(&Value::Array(tool_catalog))
}

fn build_agent_turn_response_format(
    tool_names: &[String],
    delegate_request: Option<&DelegateRequestInput>,
) -> Value {
    // 纯规划委托(planner 分解):用专用 schema 强制输出 subtasks 数组。
    // deepseek 对 json_schema 的遵守度远高于 prompt 指令,避免它回散文而非 JSON。
    if delegate_request.map(|r| r.planning_only).unwrap_or(false) {
        return serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "planner_decomposition",
                "strict": true,
                "schema": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "mode": { "type": "string", "enum": ["reply"] },
                        "assistant": { "type": "string" },
                        "tool_calls": { "type": "array", "maxItems": 0, "items": {} },
                        "subtasks": {
                            "type": "array",
                            "minItems": 1,
                            "maxItems": 4,
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "properties": {
                                    "title": { "type": "string" },
                                    "description": { "type": "string" }
                                },
                                "required": ["title", "description"]
                            }
                        }
                    },
                    "required": ["mode", "assistant", "tool_calls", "subtasks"]
                }
            }
        });
    }

    let mode_enum = build_mode_enum(tool_names, delegate_request);
    let tool_name_schema = if tool_names.is_empty() {
        serde_json::json!({ "type": "string" })
    } else {
        serde_json::json!({
            "type": "string",
            "enum": tool_names,
        })
    };
    let tool_call_min_items = if tool_names.is_empty() {
        0
    } else {
        build_tool_call_min_items(delegate_request)
    };

    serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "agent_turn_plan",
            "strict": true,
            "schema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": mode_enum,
                    },
                    "assistant": {
                        "type": "string",
                    },
                    "tool_calls": {
                        "type": "array",
                        "minItems": tool_call_min_items,
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "name": tool_name_schema,
                                "arguments_json": {
                                    "type": "string",
                                }
                            },
                            "required": ["name", "arguments_json"]
                        }
                    }
                },
                "required": ["mode", "assistant", "tool_calls"]
            }
        }
    })
}

fn build_agent_turn_plan_schema(
    tool_names: &[String],
    delegate_request: Option<&DelegateRequestInput>,
) -> Value {
    build_agent_turn_response_format(tool_names, delegate_request)["json_schema"]["schema"].clone()
}

fn build_completion_request(
    config: &AgentConfig,
    content: &str,
    history: &[Value],
    delegate_request: Option<&DelegateRequestInput>,
    follow_up_prompt: Option<&str>,
    evolved_skill_context: Option<&str>,
) -> (
    String,
    String,
    String,
    String,
    serde_json::Map<String, Value>,
    Vec<String>,
) {
    let planning_content = if let Some(prompt) = follow_up_prompt {
        prompt
    } else if delegate_request_requires_real_action(delegate_request) {
        delegate_request
            .and_then(|request| {
                let trimmed = if request.latest_user_query.trim().is_empty() {
                    request.reason.trim()
                } else {
                    request.latest_user_query.trim()
                };
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .unwrap_or(content)
    } else {
        content
    };
    let skills_package = non_empty_or(&config.skills_package, "skills").to_string();
    let channels_package = non_empty_or(&config.channels_package, "channels").to_string();

    // 纯规划委托(planner 做分解):不给任何工具,强制 LLM 只输出文本(子任务 JSON),
    // 否则带 fs_write 等工具的 LLM 会直接动手干活而非克制分解。
    let planning_only = delegate_request.map(|r| r.planning_only).unwrap_or(false);

    let tools_json = if planning_only {
        String::new()
    } else {
        let tools_input = serde_json::json!({"agent": config.name}).to_string();
        log_info(&format!(
            "agent-core build_completion_request get_tool_specs start agent={}",
            config.name
        ));
        let json = call_package(&skills_package, "get_tool_specs", &tools_input).unwrap_or_default();
        log_info(&format!(
            "agent-core build_completion_request get_tool_specs done agent={}",
            config.name
        ));
        json
    };

    let inbox_input = serde_json::json!({"agent": config.name}).to_string();
    log_info(&format!(
        "agent-core build_completion_request get_inbox start agent={}",
        config.name
    ));
    let inbox_json = call_package(&channels_package, "get_inbox", &inbox_input).unwrap_or_default();
    log_info(&format!(
        "agent-core build_completion_request get_inbox done agent={}",
        config.name
    ));

    let mut messages = Vec::new();
    let mut system = config.system_prompt.clone();

    // Sort tools by name for stable serialization — DeepSeek prefix cache requires
    // byte-identical prefixes across requests; random tool ordering breaks cache hits.
    let mut tools: Vec<Value> = parse_plugin_data(&tools_json, "tools")
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    if !planning_only {
        // Tool injection strategy: always-on tools are always injected;
        // selectable tools are only injected when selected_tools KV is set.
        let all_virtual = virtual_capability_tools(config);
        let selected_tools_json = session_id_from_agent_name(&config.name)
            .and_then(|sid| kv_get(&format!("selected_tools:{}", sid)));
        let selected_tool_names: Option<Vec<String>> = selected_tools_json
            .and_then(|j| serde_json::from_str(&j).ok());

        let filtered_virtual: Vec<_> = all_virtual
            .into_iter()
            .filter(|tool| {
                // Always-on: meta-capabilities that aren't "work" tools.
                const ALWAYS_ON: &[&str] = &["ask_user", "delegate_to_team", "semantic_select"];
                if ALWAYS_ON.contains(&tool.name) {
                    return true;
                }
                // If no selection specified, inject all (backward compatible).
                match &selected_tool_names {
                    None => true,
                    Some(names) => names.iter().any(|n| n == tool.name),
                }
            })
            .collect();

        let virtual_specs: Vec<Value> = filtered_virtual
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters,
                    }
                })
            })
            .collect();
        tools.extend(virtual_specs);
    }
    tools.sort_by(|a, b| {
        let name_a = a.get("function").and_then(|f| f.get("name")).and_then(Value::as_str).unwrap_or("");
        let name_b = b.get("function").and_then(|f| f.get("name")).and_then(Value::as_str).unwrap_or("");
        name_a.cmp(name_b)
    });
    let tool_names: Vec<String> = tools
        .iter()
        .filter_map(|entry| entry.get("function"))
        .filter_map(|value| value.get("name").and_then(|item| item.as_str()))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<String>>();
    let tool_catalog_prompt = build_tool_catalog_prompt(&tools, delegate_request);
    let tool_schemas = tools
        .iter()
        .filter_map(|entry| entry.get("function"))
        .filter_map(|tool| {
            let name = tool
                .get("name")
                .and_then(|value| value.as_str())?
                .trim()
                .to_string();
            if name.is_empty() {
                return None;
            }
            Some((name, tool.get("parameters").cloned().unwrap_or(Value::Null)))
        })
        .collect::<serde_json::Map<String, Value>>();

    // Stable prefix first (tools + response contract), then per-request dynamic content.
    // This maximises DeepSeek automatic prefix cache hits: the stable block is written to
    // disk cache on the first request and reused on subsequent ones.
    if !tool_catalog_prompt.is_empty() {
        system.push_str("\n\n[Available tools]\n");
        system.push_str(&tool_catalog_prompt);
    }
    if let Some(delegate_contract) = build_delegate_contract_block(delegate_request) {
        system.push_str("\n\n");
        system.push_str(&delegate_contract);
    }
    system.push_str("\n\n[Response contract]\nReturn JSON only.\n");
    system.push_str("Output exactly one JSON object with this shape:\n");
    system.push_str("{\"mode\":\"reply|tool\",\"assistant\":\"string\",\"tool_calls\":[{\"name\":\"tool_name\",\"arguments_json\":\"{...}\"}]}\n");
    if tool_names.is_empty() {
        system.push_str(
            "No tools are currently available, so use mode=\"reply\" and keep tool_calls empty.\n",
        );
    } else if delegate_request_requires_real_action(delegate_request) {
        system.push_str("This delegated request already requires real action. You must use mode=\"tool\" and include at least one real tool call.\n");
        system.push_str("Do not answer with mode=\"reply\".\n");
        system.push_str("Use latest_user_query as the primary source for the actual target, object, term, path, or task details.\n");
        system.push_str("If latest_user_query is a generic follow-up like 'search it for me', 'look it up', or 'read it', you must resolve the concrete target from visible_history or session_context before planning tool arguments.\n");
        system.push_str("Use delegated reason only as a short action summary, not as a replacement for the user's concrete target.\n");
        system.push_str("Use visible_history and session_context to recover omitted nouns, entities, file paths, URLs, or quoted targets from the prior user request when the delegated latest_user_query is underspecified.\n");
        system.push_str("Do not leave arguments_json empty or {} when the delegated context identifies the target.\n");
    } else {
        system.push_str("Use mode=\"tool\" when tools materially help, or when the user explicitly asks for web search, online lookup, current information, file inspection, local path reading, desktop inspection, code inspection, debugging, testing, or another real action.\n");
        system.push_str("Do not answer from memory when the user explicitly asked for online lookup or file inspection and a matching tool is available.\n");
        system.push_str("Use mode=\"reply\" only when you can fully answer now without tools.\n");
    }
    system.push_str("When mode=\"reply\", put the full user-facing answer in assistant and keep tool_calls empty.\n");
    system.push_str("When mode=\"tool\", put the required tool invocations in tool_calls and keep assistant as a short note or empty string.\n");
    system.push_str("Each tool call must use arguments_json as a compact JSON string matching that tool's parameter schema.\n");
    system.push_str("Tool rules: use fs_list for directories and fs_read only for known regular files. Prefer fs_write for file creation/updates instead of shell redirection. The host OS is Windows; when shell_exec is necessary, use PowerShell/cmd-compatible commands and avoid bash-only syntax such as here-documents, cat > file, rm -rf, or python3 unless confirmed available. If a tool fails, read its error, repair the tool choice/arguments, and retry before replying.\n");

    // Per-request dynamic content (evolved skills, inbox) is emitted as its OWN
    // message rather than concatenated into `system`. Reasonix Pillar 1 requires
    // the system block to be byte-identical across turns so the upstream provider
    // can serve it from prompt cache; mixing in volatile per-turn data would
    // invalidate the cached prefix on every request.
    let mut dynamic_context = String::new();
    if let Some(context_block) = evolved_skill_context
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        dynamic_context.push_str("[Evolved skills]\n");
        dynamic_context.push_str(context_block);
        dynamic_context.push_str("\nUse these reusable procedures when they apply to the current task.");
    }
    if include_auxiliary_planning_context(delegate_request) {
        if let Some(inbox) = parse_plugin_data(&inbox_json, "messages") {
            if inbox.as_array().map(|items| !items.is_empty()).unwrap_or(false) {
                if !dynamic_context.is_empty() {
                    dynamic_context.push_str("\n\n");
                }
                dynamic_context.push_str("[Inbox messages]\n");
                dynamic_context.push_str(&serde_json::to_string_pretty(&inbox).unwrap_or_default());
            }
        }
    }
    // Inject selected MCP tool descriptions into dynamic context.
    // When selected_tools contains "mcp:server:tool" entries, we fetch the tool
    // descriptions from mcp-client and append a hint so AI knows to use ext_mcp_call_tool.
    {
        let mcp_selected: Option<Vec<String>> = session_id_from_agent_name(&config.name)
            .and_then(|sid| kv_get(&format!("selected_tools:{}", sid)))
            .and_then(|j| serde_json::from_str(&j).ok());
        if let Some(ref names) = mcp_selected {
            let mcp_entries: Vec<&String> = names.iter().filter(|n| n.starts_with("mcp:")).collect();
            if !mcp_entries.is_empty() {
                if !dynamic_context.is_empty() {
                    dynamic_context.push_str("\n\n");
                }
                dynamic_context.push_str("[Available MCP tools — call via ext_mcp_call_tool]\n");
                for entry in &mcp_entries {
                    // Format: "mcp:server_name:tool_name"
                    let parts: Vec<&str> = entry.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let server = parts[1];
                        let tool = parts[2];
                        dynamic_context.push_str(&format!(
                            "- server=\"{}\" tool=\"{}\"\n", server, tool
                        ));
                    }
                }
                dynamic_context.push_str("To call these tools, use ext_mcp_call_tool with the server and tool names above.");
            }
        }
    }

    let is_anthropic = config.provider.to_lowercase().contains("anthropic")
        || config.model.to_lowercase().starts_with("claude-");
    if is_anthropic {
        // Stable system block gets `cache_control: ephemeral` so Anthropic caches
        // it. The dynamic block is appended as a second system message WITHOUT
        // cache_control — it's allowed to change turn-to-turn without invalidating
        // the cached stable block.
        messages.push(serde_json::json!({
            "role": "system",
            "content": [{"type": "text", "text": system, "cache_control": {"type": "ephemeral"}}]
        }));
        if !dynamic_context.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": [{"type": "text", "text": dynamic_context}]
            }));
        }
    } else {
        messages.push(serde_json::json!({"role": "system", "content": system}));
        if !dynamic_context.is_empty() {
            messages.push(
                serde_json::json!({"role": "system", "content": dynamic_context}),
            );
        }
    }
    if delegate_request_requires_real_action(delegate_request) {
        if history.len() > 1 {
            for message in &history[..history.len() - 1] {
                messages.push(message.clone());
            }
        }
        messages.push(serde_json::json!({"role": "user", "content": planning_content}));
    } else {
        let replace_last_user = follow_up_prompt.is_some()
            && history
                .last()
                .and_then(|message| message.get("role"))
                .and_then(Value::as_str)
                == Some("user");
        let preserved_len = if replace_last_user {
            history.len().saturating_sub(1)
        } else {
            history.len()
        };
        for message in &history[..preserved_len] {
            messages.push(message.clone());
        }
        if let Some(prompt) = follow_up_prompt {
            messages.push(serde_json::json!({"role": "user", "content": prompt}));
        } else {
            for message in &history[preserved_len..] {
                messages.push(message.clone());
            }
        }
    }

    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "temperature": config.temperature,
    });
    let response_format_strategy = resolve_request_structured_output_strategy(
        &config.provider,
        &config.model,
        &config.completion_endpoint,
    );
    maybe_attach_schema_response_format(
        &mut body,
        response_format_strategy,
        "agent_turn_plan",
        build_agent_turn_plan_schema(&tool_names, delegate_request),
    );
    // fan-out 修复: planner 纯规划 turn,若 provider 不支持 native json_schema(如 deepseek 走
    // PromptValidatedJson),改用 json_object 强制输出合法 JSON 对象。否则 deepseek 常忽略
    // prompt 里的"只输出 JSON"指令、回散文,导致 subtasks 解析失败、不 fan-out。
    // deepseek 已验证支持 response_format={type:"json_object"}。
    if delegate_request.map(|r| r.planning_only).unwrap_or(false)
        && response_format_strategy != StructuredOutputStrategy::NativeJsonSchema
        && body.get("response_format").is_none()
    {
        body["response_format"] = serde_json::json!({ "type": "json_object" });
    }
    if !config.provider.trim().is_empty() {
        body["x_provider"] = serde_json::json!(config.provider.trim());
    }
    apply_delegate_model_override(&mut body, delegate_request);

    (
        serde_json::to_string(&body).unwrap_or_default(),
        String::new(),
        skills_package,
        channels_package,
        tool_schemas,
        tool_names,
    )
}

#[derive(Clone)]
struct ToolExecution {
    name: String,
    args: Value,
    output: String,
    is_error: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct PlannedToolCall {
    name: String,
    arguments_json: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct AgentTurnPlan {
    mode: String,
    assistant: String,
    #[serde(default)]
    tool_calls: Vec<PlannedToolCall>,
}

fn build_tool_markers(tool_outputs: &[ToolExecution]) -> String {
    // 单个工具输出进 follow-up prompt 前限制大小:一个超大 shell/fs 输出
    // (如打印整个文件/大量日志)会瞬间撑爆 token 预算,导致 fold 来不及触发、
    // prefix-cache 失效。保留首尾、中间截断,既控预算又保留关键信息(错误通常在尾部)。
    const MAX_TOOL_OUTPUT_CHARS: usize = 6000;
    tool_outputs
        .iter()
        .map(|tool| {
            let status = if tool.is_error { "error" } else { "ok" };
            let out = &tool.output;
            let bounded = if out.chars().count() > MAX_TOOL_OUTPUT_CHARS {
                let head: String = out.chars().take(4000).collect();
                let tail: String = out
                    .chars()
                    .rev()
                    .take(1500)
                    .collect::<Vec<char>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!(
                    "{}\n…[truncated {} chars]…\n{}",
                    head,
                    out.chars().count() - 5500,
                    tail
                )
            } else {
                out.clone()
            };
            format!("\n\n[Tool: {} status={}]\n{}", tool.name, status, bounded)
        })
        .collect::<Vec<String>>()
        .join("")
}

const MAX_AGENT_TOOL_ROUNDS: usize = 50;

#[derive(Clone, Default)]
struct RequiredActionState {
    needs_shell: bool,
    needs_git_commit: bool,
    needs_git_query: bool,
    needs_fs_read: bool,
    needs_fs_list: bool,
    needs_fs_write: bool,
    needs_web_fetch: bool,
    needs_web_search: bool,
    saw_shell_success: bool,
    saw_git_commit_success: bool,
    saw_git_query_success: bool,
    saw_fs_read_success: bool,
    saw_fs_list_success: bool,
    saw_fs_write_success: bool,
    saw_web_fetch_success: bool,
    saw_web_search_success: bool,
}

fn detect_git_commit_intent(content: &str, lower: &str) -> bool {
    let positive_english = [
        "git commit",
        "commit -m",
        "commit the change",
        "commit changes",
        "commit my change",
        "commit these change",
        "make a commit",
        "create a commit",
        "do a commit",
        "add and commit",
        "stage and commit",
    ];
    if positive_english.iter().any(|phrase| lower.contains(phrase)) {
        return true;
    }

    let positive_chinese = [
        "git 提交",
        "git提交",
        "创建提交",
        "建立提交",
        "进行提交",
        "提交更改",
        "提交修改",
        "提交代码",
        "实际提交",
        "必须提交",
        "完成提交",
        "并提交",
        "然后提交",
    ];
    if positive_chinese
        .iter()
        .any(|phrase| content.contains(phrase))
    {
        return true;
    }

    false
}

fn detect_git_query_intent(content: &str, lower: &str) -> bool {
    let english = [
        "git log",
        "git status",
        "git diff",
        "git show",
        "git branch",
        "commit history",
        "recent commit",
        "last commit",
        "latest commit",
        "show commit",
        "view commit",
        "check commit",
    ];
    if english.iter().any(|phrase| lower.contains(phrase)) {
        return true;
    }
    let chinese = [
        "查看提交",
        "最近提交",
        "最近一次提交",
        "上一次提交",
        "提交记录",
        "提交历史",
        "git 日志",
        "git日志",
        "查看日志",
        "查看状态",
        "git 状态",
        "查看一下提交",
        "看一下提交",
    ];
    if chinese.iter().any(|phrase| content.contains(phrase)) {
        return true;
    }
    false
}

impl RequiredActionState {
    fn from_request(content: &str) -> Self {
        let lower = content.to_lowercase();
        let needs_git_commit = detect_git_commit_intent(content, &lower);
        let needs_git_query = !needs_git_commit && detect_git_query_intent(content, &lower);
        let needs_shell = needs_git_commit
            || lower.contains("run the")
            || lower.contains("run this")
            || lower.contains("run it")
            || (lower.contains("run") && lower.contains("script"))
            || (lower.contains("run") && lower.contains("program"))
            || (lower.contains("run") && lower.contains("command"))
            || (lower.contains("run") && lower.contains("python"))
            || (lower.contains("run") && lower.contains(".py"))
            || (lower.contains("run") && lower.contains("test"))
            || lower.contains("execute the")
            || lower.contains("execute this")
            || (lower.contains("execute") && lower.contains("script"))
            || (lower.contains("execute") && lower.contains("command"))
            || lower.contains("python")
            || content.contains("运行")
            || content.contains("执行脚本")
            || content.contains("执行命令")
            || content.contains("执行程序")
            || content.contains("执行代码")
            || content.contains("实际执行");

        let needs_fs_read = lower.contains("read file")
            || lower.contains("read the file")
            || lower.contains("read from file")
            || lower.contains("read the content")
            || lower.contains("cat ")
            || lower.contains("show file")
            || lower.contains("show the file")
            || lower.contains("file content")
            || content.contains("读取")
            || content.contains("读文件")
            || content.contains("查看文件")
            || content.contains("文件内容");

        let needs_fs_list = lower.contains("list dir")
            || lower.contains("list folder")
            || lower.contains("ls ")
            || lower.contains("dir ")
            || lower.contains("directory listing")
            || lower.contains("list files")
            || (lower.contains("list") && lower.contains("director"))
            || (lower.contains("list") && lower.contains("folder"))
            || (lower.contains("show") && lower.contains("director"))
            || content.contains("列出目录")
            || content.contains("列出文件")
            || content.contains("目录下的文件")
            || content.contains("文件列表");

        let needs_fs_write = lower.contains("write file")
            || lower.contains("create file")
            || lower.contains("save file")
            || lower.contains("write to")
            || content.contains("写入")
            || content.contains("创建文件")
            || content.contains("保存文件");

        let needs_web_fetch = (lower.contains("fetch")
            && (lower.contains("http")
                || lower.contains("url")
                || lower.contains("page")
                || lower.contains("site")))
            || (lower.contains("download") && (lower.contains("http") || lower.contains("url")))
            || lower.contains("get url")
            || lower.contains("get http")
            || lower.contains("web page")
            || lower.contains("webpage")
            || content.contains("抓取")
            || content.contains("下载网页")
            || content.contains("获取网页")
            || content.contains("访问网址");

        let needs_web_search = lower.contains("web search")
            || lower.contains("search the web")
            || lower.contains("search online")
            || lower.contains("search internet")
            || lower.contains("use web_search")
            || lower.contains("look up")
            || lower.contains("google")
            || content.contains("搜索")
            || content.contains("网络搜索")
            || content.contains("在线搜索");

        Self {
            needs_shell,
            needs_git_commit,
            needs_git_query,
            needs_fs_read,
            needs_fs_list,
            needs_fs_write,
            needs_web_fetch,
            needs_web_search,
            saw_shell_success: false,
            saw_git_commit_success: false,
            saw_git_query_success: false,
            saw_fs_read_success: false,
            saw_fs_list_success: false,
            saw_fs_write_success: false,
            saw_web_fetch_success: false,
            saw_web_search_success: false,
        }
    }

    fn observe(&mut self, tool: &ToolExecution) {
        if tool.is_error {
            return;
        }
        match tool.name.as_str() {
            "shell_exec" => {
                // 只有退出码为 0 才算 shell 动作成功。否则(编译/测试失败等)
                // 不标记成功,让 agent 循环继续规划修复轮——这是 iterate-until-green
                // 的命门:之前无条件标记成功,导致 cargo build 退出1也被当完成,
                // agent 停止自我修正。
                let exit_status = serde_json::from_str::<Value>(&tool.output)
                    .ok()
                    .and_then(|v| v.get("status").and_then(Value::as_i64))
                    .unwrap_or(0);
                if exit_status != 0 {
                    return;
                }
                self.saw_shell_success = true;
                let command_text = serde_json::to_string(&tool.args)
                    .unwrap_or_default()
                    .to_lowercase();
                let output_text = tool.output.to_lowercase();
                if command_text.contains("git commit")
                    || output_text.contains("root-commit")
                    || output_text.contains("files changed")
                {
                    self.saw_git_commit_success = true;
                }
                if command_text.contains("git log")
                    || command_text.contains("git status")
                    || command_text.contains("git diff")
                    || command_text.contains("git show")
                {
                    self.saw_git_query_success = true;
                }
            }
            "git" => {
                let command_text = serde_json::to_string(&tool.args)
                    .unwrap_or_default()
                    .to_lowercase();
                let output_text = tool.output.to_lowercase();
                if command_text.contains("commit")
                    && (output_text.contains("root-commit")
                        || output_text.contains("files changed"))
                {
                    self.saw_git_commit_success = true;
                }
                if command_text.contains("log")
                    || command_text.contains("status")
                    || command_text.contains("diff")
                    || command_text.contains("show")
                    || command_text.contains("branch")
                {
                    self.saw_git_query_success = true;
                }
            }
            "fs_read" => self.saw_fs_read_success = true,
            "fs_list" => self.saw_fs_list_success = true,
            "fs_write" => self.saw_fs_write_success = true,
            "web_fetch" | "fetch_url" => self.saw_web_fetch_success = true,
            "web_search" => self.saw_web_search_success = true,
            _ => {}
        }
    }

    fn missing_actions(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        // git tool success counts as shell-level execution for needs_shell
        let shell_satisfied = self.saw_shell_success
            || (self.needs_git_commit && self.saw_git_commit_success)
            || (self.needs_git_query && self.saw_git_query_success);
        if self.needs_shell && !shell_satisfied {
            missing.push("run the requested program or command with shell_exec successfully");
        }
        if self.needs_git_commit && !self.saw_git_commit_success {
            missing.push(
                "complete git commit with the git tool or shell_exec and capture commit hash",
            );
        }
        if self.needs_git_query && !self.saw_git_query_success && !self.saw_shell_success {
            missing.push("query git status/log/diff with the git tool or shell_exec");
        }
        if self.needs_fs_read && !self.saw_fs_read_success {
            missing.push("read the requested file with fs_read");
        }
        if self.needs_fs_list && !self.saw_fs_list_success {
            missing.push("list the requested directory with fs_list");
        }
        if self.needs_fs_write && !self.saw_fs_write_success {
            missing.push("write the requested file with fs_write");
        }
        if self.needs_web_fetch && !self.saw_web_fetch_success {
            missing.push("fetch the requested URL with web_fetch");
        }
        if self.needs_web_search && !self.saw_web_search_success {
            missing.push("search the web with web_search");
        }
        missing
    }
}

fn build_required_action_prompt(
    content: &str,
    last_tool_reply: Option<&str>,
    missing: &[&str],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("Original user request:\n");
    prompt.push_str(content.trim());
    prompt.push_str("\n\nYou tried to finish, but required actions are still missing:\n");
    for item in missing {
        prompt.push_str("- ");
        prompt.push_str(item);
        prompt.push('\n');
    }
    if let Some(reply) = last_tool_reply.filter(|value| !value.trim().is_empty()) {
        prompt.push_str("\nPrevious tool context:\n");
        prompt.push_str(reply);
        prompt.push('\n');
    }
    prompt.push_str("\nReturn mode=\"tool\" with the next required tool calls only. Do not return mode=\"reply\" until all missing actions are complete. Use the git tool for git commands with args like [\"-C\", path, \"init\"] or [\"-C\", path, \"commit\", \"-m\", message]. For running a Python script on Windows, use shell_exec with command=\"python\", args=[script_path, ...script_args], cwd=script_dir (do not invoke .py files directly through PowerShell). For other Windows commands, use shell_exec with command=\"pwsh\" and args [\"-NoProfile\", \"-Command\", \"...\"]. Treat any tool output with status=\"error\" or a non-empty \"error\" field as failure even when nested inside a JSON string.");
    prompt
}

fn build_follow_up_planning_prompt(content: &str, tool_outputs: &[ToolExecution]) -> String {
    let mut prompt = String::new();
    prompt.push_str("Original user request:\n");
    prompt.push_str(content.trim());
    prompt.push_str("\n\nUse the following real tool outputs from the previous step to continue the same task:\n");
    prompt.push_str(&build_tool_markers(tool_outputs));
    prompt.push_str("\n\nRules for the next round:");
    prompt.push_str("\n- If a tool output has status=\"error\", OR contains a non-empty \"error\" field even at status=\"ok\" (including JSON strings nested inside the error field), treat that call as a FAILURE and repair the arguments or pick another tool in the next round; do not pretend it succeeded.");
    prompt.push_str("\n- For Python scripts on Windows, prefer shell_exec with command=\"python\", args=[script_path, arg1, arg2, ...], cwd=script_dir. Do NOT invoke a .py file directly via PowerShell or cmd; only python/py executables can run Python source.");
    prompt.push_str("\n- For other commands on Windows, use shell_exec with command=\"pwsh\" or command=\"powershell\" and explicit args, not bash-only syntax.");
    prompt.push_str("\n- For git operations, prefer the dedicated git tool with args=[\"-C\", path, ...subcommand]. For queries (log/status/diff) you only need one successful read; do not run init/add/commit just to read.");
    prompt.push_str("\n- After a successful fs_read / fs_list / fs_write / web_fetch / web_search / git-query that already satisfies the user's request, return mode=\"reply\" with the user-facing answer immediately; do not perform unrelated tools.");
    prompt.push_str("\n- If another tool step is still required, return mode=\"tool\" with only the next needed tool calls. If the task is complete, return mode=\"reply\" with the final user-facing answer.");
    prompt
}

fn finalize_tool_round_reply(plan_assistant: &str, tool_outputs: &[ToolExecution]) -> String {
    let local_tool_answer = build_local_tool_answer(tool_outputs);
    let mut final_reply = if !local_tool_answer.trim().is_empty() {
        local_tool_answer
    } else {
        plan_assistant.trim().to_string()
    };

    if final_reply.trim().is_empty() {
        final_reply = "Done.".to_string();
    }

    final_reply.push_str(&build_tool_markers(tool_outputs));
    final_reply
}

fn execute_agent_turn_plan_loop<PlanFn, ToolFn>(
    content: &str,
    planning_only: bool,
    mut plan_round: PlanFn,
    mut execute_tool_call: ToolFn,
) -> Result<String, String>
where
    PlanFn: FnMut(usize, Option<&str>) -> Result<AgentTurnPlan, String>,
    ToolFn: FnMut(usize, PlannedToolCall) -> Result<ToolExecution, String>,
{
    let mut last_tool_reply: Option<String> = None;
    let mut follow_up_prompt: Option<String> = None;
    // 纯规划委托(planner 分解)无工具、只需一轮 reply,不应触发"必需动作"门控
    // （否则会因任务文案含"build"等被判定有未完成动作而无限重试)。
    let mut required_actions = if planning_only {
        RequiredActionState::default()
    } else {
        RequiredActionState::from_request(content)
    };
    // Loop detection: track fingerprint of previous round's tool calls.
    let mut prev_tool_fingerprint: Option<String> = None;

    for round in 0..MAX_AGENT_TOOL_ROUNDS {
        let plan = plan_round(round, follow_up_prompt.as_deref())?;

        if plan.mode != "tool" {
            let missing = required_actions.missing_actions();
            if !missing.is_empty() && round + 1 < MAX_AGENT_TOOL_ROUNDS {
                follow_up_prompt = Some(build_required_action_prompt(
                    content,
                    last_tool_reply.as_deref(),
                    &missing,
                ));
                continue;
            }
            let final_reply = plan.assistant.trim().to_string();
            if !final_reply.is_empty() {
                return Ok(final_reply);
            }
            if let Some(reply) = last_tool_reply {
                return Ok(reply);
            }
            return Ok("Done.".to_string());
        }

        if plan.tool_calls.is_empty() {
            let missing = required_actions.missing_actions();
            if !missing.is_empty() && round + 1 < MAX_AGENT_TOOL_ROUNDS {
                follow_up_prompt = Some(build_required_action_prompt(
                    content,
                    last_tool_reply.as_deref(),
                    &missing,
                ));
                continue;
            }
            let final_reply = plan.assistant.trim().to_string();
            if !final_reply.is_empty() {
                return Ok(final_reply);
            }
            if let Some(reply) = last_tool_reply {
                return Ok(reply);
            }
            return Ok("Done.".to_string());
        }

        let mut tool_outputs = Vec::new();
        // Loop detection: build fingerprint of this round's tool calls.
        let this_fingerprint = plan
            .tool_calls
            .iter()
            .map(|tc| format!("{}:{}", tc.name.trim(), tc.arguments_json.trim()))
            .collect::<Vec<_>>()
            .join("|");
        if let Some(ref prev) = prev_tool_fingerprint {
            if *prev == this_fingerprint && !this_fingerprint.is_empty() {
                let reply = last_tool_reply.unwrap_or_else(|| "Done.".to_string());
                return Ok(format!(
                    "{}\n\n[Stopped: repeated identical tool calls detected]",
                    reply.trim()
                ));
            }
        }
        prev_tool_fingerprint = Some(this_fingerprint);
        for tool_call in plan.tool_calls {
            match execute_tool_call(round, tool_call.clone()) {
                Ok(output) => tool_outputs.push(output),
                Err(error) => tool_outputs.push(ToolExecution {
                    name: tool_call.name.trim().to_string(),
                    args: serde_json::json!({
                        "arguments_json": tool_call.arguments_json,
                    }),
                    output: serde_json::json!({
                        "status": "error",
                        "error": error,
                        "repair_hint": "Inspect this error, correct the tool name or arguments, and retry if the user task still requires action."
                    })
                    .to_string(),
                    is_error: true,
                }),
            }
        }

        let tool_reply = finalize_tool_round_reply(&plan.assistant, &tool_outputs);
        for output in &tool_outputs {
            required_actions.observe(output);
        }
        let missing = required_actions.missing_actions();
        if !missing.is_empty() {
            follow_up_prompt = Some(build_required_action_prompt(
                content,
                Some(&tool_reply),
                &missing,
            ));
        } else {
            follow_up_prompt = Some(build_follow_up_planning_prompt(content, &tool_outputs));
        }
        last_tool_reply = Some(tool_reply.clone());

        if round + 1 >= MAX_AGENT_TOOL_ROUNDS {
            let missing = required_actions.missing_actions();
            if !missing.is_empty() {
                // 撞工具轮数上限但还有未完成动作:给出结构化的"待续"摘要,
                // 让用户或 orchestrator 发一条"继续"消息即可从 session 历史无缝接续
                // (会话历史 + fold 摘要已保留全部上下文)。这是低风险的跨-turn 续接,
                // 优于高风险的 turn 内无人值守自动循环。
                return Ok(format!(
                    "⏸ 已达单轮工具预算上限({}轮),任务尚未完成。\n\n\
                     **待完成**:{}\n\n\
                     **最近进展**:{}\n\n\
                     回复\"继续\"我会从当前进度接着做。",
                    MAX_AGENT_TOOL_ROUNDS,
                    missing.join("、"),
                    last_tool_reply
                        .as_deref()
                        .map(|reply| reply.chars().take(500).collect::<String>())
                        .unwrap_or_else(|| "(无)".to_string())
                ));
            }
            return Ok(tool_reply);
        }
    }

    Ok(last_tool_reply.unwrap_or_else(|| "Done.".to_string()))
}

fn tool_result_is_error(raw: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    value_indicates_error(&value, 0)
}

fn value_indicates_error(value: &Value, depth: usize) -> bool {
    if depth > 6 {
        return false;
    }
    match value {
        Value::Object(map) => {
            if map.get("status").and_then(Value::as_str) == Some("error") {
                return true;
            }
            if let Some(error_field) = map.get("error") {
                if error_field_indicates_error(error_field, depth) {
                    return true;
                }
            }
            for key in ["response", "data", "result", "output", "body", "payload"] {
                if let Some(child) = map.get(key) {
                    if value_indicates_error(child, depth + 1) {
                        return true;
                    }
                }
            }
            false
        }
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return false;
            }
            if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                return value_indicates_error(&parsed, depth + 1);
            }
            false
        }
        _ => false,
    }
}

fn error_field_indicates_error(error_field: &Value, depth: usize) -> bool {
    match error_field {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return false;
            }
            if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                return value_indicates_error(&parsed, depth + 1) || true;
            }
            true
        }
        Value::Object(map) => {
            if map.is_empty() {
                return false;
            }
            if value_indicates_error(error_field, depth + 1) {
                return true;
            }
            true
        }
        Value::Array(items) => !items.is_empty(),
        Value::Number(_) => true,
    }
}

fn parse_completion_error(response_value: &Value) -> Option<String> {
    response_value
        .get("error")
        .and_then(|value| {
            value
                .get("message")
                .or_else(|| value.get("error"))
                .or(Some(value))
        })
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn visible_assistant_reply(raw: &str) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    // 防御:若整段是协议 JSON({"mode":...,"assistant":...,"tool_calls":...}),
    // 提取 assistant 字段而非原样返回——否则原始协议 JSON 会泄漏给用户(尤其
    // 历史消息存储后切回会话时)。
    let trimmed = normalized.trim();
    if trimmed.starts_with('{') && trimmed.contains("\"mode\"") {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) {
            if let Some(assistant) = map.get("assistant").and_then(Value::as_str) {
                return visible_assistant_reply(assistant);
            }
            // 是协议 JSON 但无 assistant 文本(纯工具轮):不泄漏 JSON,返回空。
            if map.contains_key("tool_calls") || map.get("mode").is_some() {
                return String::new();
            }
        }
    }
    if let Some(index) = normalized.find("\n[Tool: ") {
        return normalized[..index].trim().to_string();
    }
    if normalized.starts_with("[Tool: ") {
        return String::new();
    }
    normalized.trim().to_string()
}

fn parse_plugin_ok_data(raw: &str) -> Option<Value> {
    let payload: Value = serde_json::from_str(raw).ok()?;
    if payload.get("status").and_then(|value| value.as_str()) != Some("ok") {
        return None;
    }
    payload.get("data").cloned()
}

fn call_package_ws_action(
    package_name: &str,
    action: &str,
    payload: &Value,
) -> Result<String, String> {
    weft_package_sdk::call_package_ws_action(package_name.trim(), action.trim(), payload)
}

fn summarize_web_search_output(tool: &ToolExecution) -> Option<String> {
    let data = parse_plugin_ok_data(&tool.output)?;
    let mut parts = Vec::new();

    if let Some(summary) = data.get("summary").and_then(|value| value.as_str()) {
        let trimmed = summary.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }

    let mut related_lines = Vec::new();
    if let Some(results) = data.get("results").and_then(|value| value.as_array()) {
        for entry in results.iter().take(3) {
            let text = entry
                .get("text")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim();
            let url = entry
                .get("url")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim();
            if text.is_empty() {
                continue;
            }
            if url.is_empty() {
                related_lines.push(format!("- {}", text));
            } else {
                related_lines.push(format!("- {} ({})", text, url));
            }
        }
    }
    if !related_lines.is_empty() {
        parts.push(format!("Related results:\n{}", related_lines.join("\n")));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn summarize_package_manifest(content: &str) -> Option<String> {
    let payload: Value = serde_json::from_str(content).ok()?;
    let object = payload.as_object()?;
    if !object.contains_key("scripts")
        || !object.contains_key("dependencies") && !object.contains_key("devDependencies")
    {
        return None;
    }

    let name = object
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or("this project");
    let version = object
        .get("version")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let private = object
        .get("private")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let main_entry = object
        .get("main")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    let scripts = object
        .get("scripts")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let dependencies = object
        .get("dependencies")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let dev_dependencies = object
        .get("devDependencies")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();

    let has_dep =
        |name: &str| dependencies.contains_key(name) || dev_dependencies.contains_key(name);
    let uses_electron = has_dep("electron")
        || scripts
            .values()
            .any(|value| value.as_str().unwrap_or("").contains("electron"));
    let uses_vue = has_dep("vue");
    let uses_pinia = has_dep("pinia");
    let uses_electron_vite = has_dep("electron-vite")
        || scripts
            .values()
            .any(|value| value.as_str().unwrap_or("").contains("electron-vite"));
    let uses_electron_builder = has_dep("electron-builder")
        || scripts
            .values()
            .any(|value| value.as_str().unwrap_or("").contains("electron-builder"));

    let mut runtime_parts = Vec::new();
    if uses_electron {
        runtime_parts.push("Electron");
    }
    if uses_vue {
        runtime_parts.push("Vue 3");
    }
    if uses_pinia {
        runtime_parts.push("Pinia");
    }
    if uses_electron_vite {
        runtime_parts.push("electron-vite");
    }

    let mut lines = Vec::new();
    let mut intro = format!(
        "This is a {} project named {}",
        if private { "private" } else { "Node" },
        name
    );
    if !version.is_empty() {
        intro.push_str(&format!(" (v{}).", version));
    } else {
        intro.push('.');
    }
    lines.push(intro);

    if !runtime_parts.is_empty() {
        let mut runtime_line = format!("It is built around {}.", runtime_parts.join(", "));
        if !main_entry.trim().is_empty() {
            runtime_line.push_str(&format!(
                " The main desktop entry is {}.",
                main_entry.trim()
            ));
        }
        lines.push(runtime_line);
    }

    let mut script_notes = Vec::new();
    if let Some(dev) = scripts.get("dev").and_then(|value| value.as_str()) {
        script_notes.push(format!("dev uses {}", dev.trim()));
    }
    if let Some(build) = scripts.get("build").and_then(|value| value.as_str()) {
        script_notes.push(format!("build uses {}", build.trim()));
    }
    if uses_electron_builder {
        if let Some(build_win) = scripts.get("build:win").and_then(|value| value.as_str()) {
            script_notes.push(format!("Windows packaging uses {}", build_win.trim()));
        }
    }
    if !script_notes.is_empty() {
        lines.push(format!(
            "The package scripts show that {}.",
            script_notes.join("; ")
        ));
    }

    let mut notable_deps = Vec::new();
    for dependency in [
        "better-sqlite3",
        "ws",
        "mermaid",
        "marked",
        "katex",
        "highlight.js",
        "chart.js",
        "vue-chartjs",
    ] {
        if has_dep(dependency) {
            notable_deps.push(dependency);
        }
    }
    if !notable_deps.is_empty() {
        lines.push(format!(
            "Notable dependencies include {}, which suggests a desktop chat/UI app with local storage, markdown/rendering, and visualization features.",
            notable_deps.join(", ")
        ));
    }

    Some(lines.join("\n\n"))
}

fn summarize_fs_read_output(tool: &ToolExecution) -> Option<String> {
    let data = parse_plugin_ok_data(&tool.output)?;
    let content = data.get("content").and_then(|value| value.as_str())?.trim();
    if content.is_empty() {
        return None;
    }

    let path = tool
        .args
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim();
    if path.ends_with("package.json") {
        if let Some(summary) = summarize_package_manifest(content) {
            return Some(summary);
        }
    }

    if path.ends_with(".json") {
        if let Ok(payload) = serde_json::from_str::<Value>(content) {
            if let Some(object) = payload.as_object() {
                let keys = object.keys().take(12).cloned().collect::<Vec<String>>();
                if !keys.is_empty() {
                    return Some(format!(
                        "I inspected {}. It is a JSON document with top-level keys: {}.",
                        if path.is_empty() { "the file" } else { path },
                        keys.join(", ")
                    ));
                }
            }
        }
    }

    let preview = content
        .lines()
        .take(16)
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_string();
    if preview.is_empty() {
        None
    } else {
        Some(format!(
            "I inspected {}. Here is the relevant content preview:\n{}",
            if path.is_empty() { "the file" } else { path },
            preview
        ))
    }
}

fn build_local_tool_answer(tool_outputs: &[ToolExecution]) -> String {
    let mut parts = Vec::new();

    for tool in tool_outputs {
        let summary = summarize_known_tool_output(tool);
        if let Some(summary) = summary {
            let trimmed = summary.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_string());
            }
        }
    }

    parts.join("\n\n")
}

fn resolve_completion_endpoint(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with("host://") {
        return default_completion_endpoint();
    }
    trimmed.to_string()
}

fn summarize_web_fetch_output(tool: &ToolExecution) -> Option<String> {
    let parsed: Value = serde_json::from_str(tool.output.trim()).ok()?;
    if parsed.get("status").and_then(Value::as_str) != Some("ok") {
        return None;
    }

    let data = parsed.get("data")?;
    let status = data.get("status")?.as_u64()?;
    let body = data
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let preview = body
        .chars()
        .take(600)
        .collect::<String>()
        .trim()
        .to_string();
    let url = tool
        .args
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    let mut summary = if url.is_empty() {
        format!("Fetched the web page successfully (HTTP {}).", status)
    } else {
        format!("Fetched {} successfully (HTTP {}).", url, status)
    };

    if !preview.is_empty() {
        summary.push_str(" Preview:\n");
        summary.push_str(&preview);
    }

    Some(summary)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VirtualCapabilitySummaryKind {
    PromptSystemRender,
    WorkflowOrchestrationPlan,
    McpListServers,
    McpCallTool,
    ChannelRegister,
    ChannelList,
    ChannelSend,
    DelegateToTeam,
    AskUser,
    BrowserTool,
    SelectorTool,
}

#[derive(Clone, Debug)]
struct VirtualCapabilityToolSpec {
    name: &'static str,
    capability: &'static str,
    action: &'static str,
    description: &'static str,
    parameters: Value,
    provider_candidates: Vec<String>,
    summary_kind: VirtualCapabilitySummaryKind,
}

fn push_unique_provider_candidate(target: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }
    if !target.iter().any(|entry| entry == trimmed) {
        target.push(trimmed.to_string());
    }
}

fn provider_candidates_for_capability(config: &AgentConfig, capability: &str) -> Vec<String> {
    let mut providers = Vec::new();
    match capability {
        "prompt.system" => {
            push_unique_provider_candidate(&mut providers, "prompt-system");
        }
        "workflow.orchestration" => {
            push_unique_provider_candidate(&mut providers, "workflow-orchestrator");
        }
        "ext.mcp" => {
            push_unique_provider_candidate(&mut providers, "mcp-client");
        }
        "channel.bridge" => {
            push_unique_provider_candidate(
                &mut providers,
                non_empty_or(&config.channels_package, "channels"),
            );
            push_unique_provider_candidate(&mut providers, "channel-core");
            push_unique_provider_candidate(&mut providers, "channels");
        }
        "team.taskboard" => {
            push_unique_provider_candidate(&mut providers, "team-task-board");
        }
        "tool.browser" => {
            push_unique_provider_candidate(&mut providers, "tool-browser");
        }
        "tool.selector" => {
            push_unique_provider_candidate(&mut providers, "tool-selector");
        }
        _ => {}
    }
    providers
}

fn virtual_capability_tools(config: &AgentConfig) -> Vec<VirtualCapabilityToolSpec> {
    // 是否为团队委托的子 agent(planner/implementer/reviewer/integrator 的 delegate session)。
    // 子 agent 不应再持有 delegate_to_team,否则会嵌套委托、建出新 board 导致 dispatch 循环爆炸。
    let is_delegate_subagent = config.name.contains("team-delegate");
    let tools = vec![
        VirtualCapabilityToolSpec {
            name: "prompt_system_render",
            capability: "prompt.system",
            action: "render",
            description: "Render the prompt.system capability through the bound prompt provider. Use this for system prompt composition, prompt rendering, or higher-level prompt policy generation.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "system_prompt": {
                        "type": "string",
                        "description": "System prompt text to render through prompt.system"
                    }
                },
                "required": ["system_prompt"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "prompt.system"),
            summary_kind: VirtualCapabilitySummaryKind::PromptSystemRender,
        },
        VirtualCapabilityToolSpec {
            name: "workflow_orchestration_plan",
            capability: "workflow.orchestration",
            action: "plan",
            description: "Plan or validate a workflow through the workflow.orchestration capability. Use this for step planning, orchestration proposals, and execution workflows.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "goal": {
                        "type": "string",
                        "description": "High-level workflow goal"
                    },
                    "steps": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional ordered workflow steps"
                    },
                    "context": {
                        "type": "object",
                        "description": "Optional structured workflow context"
                    }
                },
                "required": ["goal"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "workflow.orchestration"),
            summary_kind: VirtualCapabilitySummaryKind::WorkflowOrchestrationPlan,
        },
        VirtualCapabilityToolSpec {
            name: "delegate_to_team",
            capability: "team.taskboard",
            action: "create_board",
            description: "Delegate a complex, multi-step build task to an autonomous agent team (planner → implementer → reviewer → integrator). Use this when the user asks to build, implement, or create something substantial that benefits from multi-role collaboration rather than a single direct answer. The team runs automatically in the background; this returns a board_id to track progress.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "goal": {
                        "type": "string",
                        "description": "The concrete goal/task for the team to accomplish"
                    },
                    "title": {
                        "type": "string",
                        "description": "Optional short title for the board (defaults to goal)"
                    },
                    "workspace_dir": {
                        "type": "string",
                        "description": "Optional working directory for produced files"
                    }
                },
                "required": ["goal"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "team.taskboard"),
            summary_kind: VirtualCapabilitySummaryKind::DelegateToTeam,
        },
        VirtualCapabilityToolSpec {
            name: "ask_user",
            capability: "session.events",
            action: "append_event",
            description: "Ask the user a question with optional choices when you need a decision before proceeding (e.g. which approach, which format, missing info). The turn ends after asking; the user's answer arrives as the next message.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Question to present to the user"
                    },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional clickable choices for the user"
                    }
                },
                "required": ["question"]
            }),
            provider_candidates: Vec::new(),
            summary_kind: VirtualCapabilitySummaryKind::AskUser,
        },
        VirtualCapabilityToolSpec {
            name: "ext_mcp_list_servers",
            capability: "ext.mcp",
            action: "list_servers",
            description: "List MCP servers exposed through the ext.mcp capability for the current agent.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent": {
                        "type": "string",
                        "description": "Agent name. Defaults to the current session agent when omitted."
                    }
                },
                "required": []
            }),
            provider_candidates: provider_candidates_for_capability(config, "ext.mcp"),
            summary_kind: VirtualCapabilitySummaryKind::McpListServers,
        },
        VirtualCapabilityToolSpec {
            name: "ext_mcp_call_tool",
            capability: "ext.mcp",
            action: "call_tool",
            description: "Call a concrete MCP tool through the ext.mcp capability. Use this when the user explicitly asks to run an MCP server tool.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent": {
                        "type": "string",
                        "description": "Agent name. Defaults to the current session agent when omitted."
                    },
                    "server": {
                        "type": "string",
                        "description": "MCP server name"
                    },
                    "tool": {
                        "type": "string",
                        "description": "Tool name exposed by the MCP server"
                    },
                    "args": {
                        "type": "object",
                        "description": "JSON arguments for the MCP tool call"
                    }
                },
                "required": ["server", "tool"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "ext.mcp"),
            summary_kind: VirtualCapabilitySummaryKind::McpCallTool,
        },
        VirtualCapabilityToolSpec {
            name: "channel_bridge_register",
            capability: "channel.bridge",
            action: "register_channel",
            description: "Register a channel configuration through the channel.bridge capability for the current agent.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent": {
                        "type": "string",
                        "description": "Agent name. Defaults to the current session agent when omitted."
                    },
                    "config": {
                        "type": "object",
                        "description": "Channel configuration payload to register"
                    }
                },
                "required": ["config"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "channel.bridge"),
            summary_kind: VirtualCapabilitySummaryKind::ChannelRegister,
        },
        VirtualCapabilityToolSpec {
            name: "channel_bridge_list",
            capability: "channel.bridge",
            action: "list_channels",
            description: "List registered channel configurations through the channel.bridge capability for the current agent.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent": {
                        "type": "string",
                        "description": "Agent name. Defaults to the current session agent when omitted."
                    }
                },
                "required": []
            }),
            provider_candidates: provider_candidates_for_capability(config, "channel.bridge"),
            summary_kind: VirtualCapabilitySummaryKind::ChannelList,
        },
        VirtualCapabilityToolSpec {
            name: "channel_bridge_send",
            capability: "channel.bridge",
            action: "send",
            description: "Send a message through the channel.bridge capability to another agent or registered channel target.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent": {
                        "type": "string",
                        "description": "Sender agent name. Defaults to the current session agent when omitted."
                    },
                    "to": {
                        "type": "string",
                        "description": "Target agent or channel identifier"
                    },
                    "content": {
                        "type": "string",
                        "description": "Message content to send"
                    }
                },
                "required": ["to", "content"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "channel.bridge"),
            summary_kind: VirtualCapabilitySummaryKind::ChannelSend,
        },
        VirtualCapabilityToolSpec {
            name: "browser_navigate",
            capability: "tool.browser",
            action: "navigate",
            description: "Navigate the automated browser to a URL. Starts a browser session automatically if needed. Use this to open a web page before reading or interacting with it.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to open in the automated browser"
                    }
                },
                "required": ["url"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "tool.browser"),
            summary_kind: VirtualCapabilitySummaryKind::BrowserTool,
        },
        VirtualCapabilityToolSpec {
            name: "browser_snapshot",
            capability: "tool.browser",
            action: "snapshot",
            description: "Take an accessibility-tree snapshot of the current page. Returns page elements each with a unique uid. You MUST call this before browser_click/browser_fill to obtain the uid of the element you want to interact with.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            provider_candidates: provider_candidates_for_capability(config, "tool.browser"),
            summary_kind: VirtualCapabilitySummaryKind::BrowserTool,
        },
        VirtualCapabilityToolSpec {
            name: "browser_click",
            capability: "tool.browser",
            action: "click",
            description: "Click an element on the page identified by its uid (obtained from browser_snapshot).",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "string",
                        "description": "The element uid from a prior browser_snapshot"
                    }
                },
                "required": ["uid"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "tool.browser"),
            summary_kind: VirtualCapabilitySummaryKind::BrowserTool,
        },
        VirtualCapabilityToolSpec {
            name: "browser_fill",
            capability: "tool.browser",
            action: "fill",
            description: "Fill a text input or select field identified by its uid (from browser_snapshot) with a value.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "uid": {
                        "type": "string",
                        "description": "The element uid from a prior browser_snapshot"
                    },
                    "value": {
                        "type": "string",
                        "description": "The text/value to enter into the element"
                    }
                },
                "required": ["uid", "value"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "tool.browser"),
            summary_kind: VirtualCapabilitySummaryKind::BrowserTool,
        },
        VirtualCapabilityToolSpec {
            name: "browser_screenshot",
            capability: "tool.browser",
            action: "screenshot",
            description: "Take a screenshot of the current browser page. Returns image data.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            provider_candidates: provider_candidates_for_capability(config, "tool.browser"),
            summary_kind: VirtualCapabilitySummaryKind::BrowserTool,
        },
        VirtualCapabilityToolSpec {
            name: "semantic_select",
            capability: "tool.selector",
            action: "select",
            description: "Semantically match a natural-language query against a candidate library and return the top-k best matches. Use for fast tool routing, asset/material selection, or any scenario where you need to pick the most relevant item from a known set. Latency: 5-50ms, no LLM call needed.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query describing what you're looking for"
                    },
                    "library": {
                        "type": "string",
                        "description": "Name of the candidate library to search in (e.g. 'tools', 'actions', 'expressions', 'backgrounds')"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Number of top matches to return (default: 3)"
                    }
                },
                "required": ["query", "library"]
            }),
            provider_candidates: provider_candidates_for_capability(config, "tool.selector"),
            summary_kind: VirtualCapabilitySummaryKind::SelectorTool,
        },
    ];
    // 子 agent 过滤掉 delegate_to_team(防嵌套委托)。
    tools
        .into_iter()
        .filter(|t| !(is_delegate_subagent && t.name == "delegate_to_team"))
        .collect()
}

fn virtual_capability_tool_specs(config: &AgentConfig) -> Vec<Value> {
    virtual_capability_tools(config)
        .into_iter()
        .map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }
            })
        })
        .collect()
}

fn summarize_shell_exec_output(tool: &ToolExecution) -> Option<String> {
    let data = parse_plugin_ok_data(&tool.output)?;
    let stdout = data
        .get("stdout")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let stderr = data
        .get("stderr")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let exit_status = data.get("status").and_then(Value::as_i64).unwrap_or(0);

    let command = tool
        .args
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    let output = if !stdout.is_empty() { stdout } else { stderr };
    if output.is_empty() && exit_status == 0 {
        return Some(format!(
            "Command `{}` completed successfully with no output.",
            if command.is_empty() { "shell" } else { command }
        ));
    }

    let preview = output
        .lines()
        .take(24)
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_string();

    if preview.is_empty() {
        return None;
    }

    let status_note = if exit_status != 0 {
        format!(" (exit {})", exit_status)
    } else {
        String::new()
    };

    Some(format!(
        "Command `{}`{} output:\n{}",
        if command.is_empty() { "shell" } else { command },
        status_note,
        preview
    ))
}

fn summarize_git_output(tool: &ToolExecution) -> Option<String> {
    let data = parse_plugin_ok_data(&tool.output)?;
    let stdout = data
        .get("stdout")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let stderr = data
        .get("stderr")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let exit_status = data.get("status").and_then(Value::as_i64).unwrap_or(0);

    let args_text = serde_json::to_string(&tool.args)
        .unwrap_or_default()
        .to_lowercase();
    let output = if !stdout.is_empty() { stdout } else { stderr };

    if output.is_empty() {
        if exit_status == 0 {
            return Some("Git command completed successfully.".to_string());
        }
        return None;
    }

    let preview = output
        .lines()
        .take(20)
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_string();

    let verb = if args_text.contains("\"log\"") || args_text.contains("log") {
        "log"
    } else if args_text.contains("\"status\"") || args_text.contains("status") {
        "status"
    } else if args_text.contains("\"commit\"") {
        "commit"
    } else {
        "git"
    };

    Some(format!("Git {} output:\n{}", verb, preview))
}

fn summarize_fs_list_output(tool: &ToolExecution) -> Option<String> {
    let data = parse_plugin_ok_data(&tool.output)?;
    let path = tool
        .args
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    let entries = data.get("entries").and_then(Value::as_array)?;
    if entries.is_empty() {
        return Some(format!(
            "Directory `{}` is empty.",
            if path.is_empty() { "." } else { path }
        ));
    }

    let mut lines = Vec::new();
    for entry in entries.iter().take(40) {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let kind = entry
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("file")
            .trim();
        if !name.is_empty() {
            let marker = if kind == "dir" { "/" } else { "" };
            lines.push(format!("{}{}", name, marker));
        }
    }

    if lines.is_empty() {
        return None;
    }

    Some(format!(
        "Directory `{}` contains: {}",
        if path.is_empty() { "." } else { path },
        lines.join(", ")
    ))
}

fn summarize_fs_write_output(tool: &ToolExecution) -> Option<String> {
    let data = parse_plugin_ok_data(&tool.output)?;
    let path = tool
        .args
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let written = data.get("written").and_then(Value::as_bool).unwrap_or(true);
    if written {
        Some(format!(
            "Successfully wrote to `{}`.",
            if path.is_empty() { "file" } else { path }
        ))
    } else {
        None
    }
}

fn summarize_fetch_url_output(tool: &ToolExecution) -> Option<String> {
    // fetch_url uses the same data shape as web_fetch
    summarize_web_fetch_output(tool)
}

fn summarize_known_tool_output(tool: &ToolExecution) -> Option<String> {
    match tool.name.as_str() {
        "web_search" => summarize_web_search_output(tool),
        "fs_read" => summarize_fs_read_output(tool),
        "web_fetch" | "fetch_url" => {
            summarize_web_fetch_output(tool).or_else(|| summarize_fetch_url_output(tool))
        }
        "shell_exec" => summarize_shell_exec_output(tool),
        "git" => summarize_git_output(tool),
        "fs_list" => summarize_fs_list_output(tool),
        "fs_write" => summarize_fs_write_output(tool),
        _ => None,
    }
    .or_else(|| {
        find_virtual_capability_tool(&default_agent_config(), &tool.name)
            .and_then(|route| summarize_virtual_capability_output(tool, &route))
    })
}

fn find_virtual_capability_tool(
    config: &AgentConfig,
    tool_name: &str,
) -> Option<VirtualCapabilityToolSpec> {
    virtual_capability_tools(config)
        .into_iter()
        .find(|tool| tool.name == tool_name)
}

fn with_default_agent_arg(args: &Value, agent_name: &str) -> Value {
    let mut object = args.as_object().cloned().unwrap_or_default();
    let should_set_agent = object
        .get("agent")
        .and_then(Value::as_str)
        .map(|value| value.trim().is_empty())
        .unwrap_or(true);
    if should_set_agent {
        object.insert("agent".to_string(), serde_json::json!(agent_name));
    }
    Value::Object(object)
}

fn build_virtual_capability_payload(
    route: &VirtualCapabilityToolSpec,
    agent_name: &str,
    args: &Value,
) -> Value {
    match route.capability {
        "ext.mcp" | "channel.bridge" | "tool.browser" | "tool.selector" => {
            with_default_agent_arg(args, agent_name)
        }
        _ => args.clone(),
    }
}

fn execute_virtual_capability_tool(
    session_id: &str,
    config: &AgentConfig,
    route: &VirtualCapabilityToolSpec,
    args: &Value,
) -> String {
    // delegate_to_team 需要两步(create_board + save_task),不是单 action 直传，
    // 单独处理：建一个 board + 一个 intake 任务，后台 orchestrator tick/dispatch
    // 会自动跑完 plan→execute→review→integrate→done。返回 board_id 供追踪。
    if route.name == "delegate_to_team" {
        return execute_delegate_to_team(route, args, session_id);
    }
    if route.name == "ask_user" {
        return execute_ask_user(session_id, args);
    }

    let payload = build_virtual_capability_payload(route, &config.name, args);
    let mut last_error = String::new();

    for provider in &route.provider_candidates {
        match call_package_ws_action(provider, route.action, &payload) {
            Ok(result) => return result,
            Err(error) => last_error = error,
        }
    }

    PackageResult::err(format!(
        "no available provider for capability '{}' (action '{}'): {}",
        route.capability,
        route.action,
        if last_error.trim().is_empty() {
            "provider call failed"
        } else {
            last_error.trim()
        }
    ))
    .to_json()
}

/// 启动一次团队编排：create_board + save_task(intake)。
/// 后台 orchestrator 的 tick/dispatch 循环会自动推进所有阶段。
fn execute_delegate_to_team(
    route: &VirtualCapabilityToolSpec,
    args: &Value,
    origin_session_id: &str,
) -> String {
    let goal = args
        .get("goal")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if goal.is_empty() {
        return PackageResult::err("delegate_to_team requires a non-empty goal").to_json();
    }
    let title = args
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(goal);

    let ts = now_ms();
    let board_id = format!("board-{}", ts);
    let session_id = format!("orch-{}", ts);
    let task_id = format!("task-{}", ts);

    let mut board_meta = serde_json::json!({ "origin": "weft-claw-chat" });
    if let Some(ws) = args.get("workspace_dir").and_then(Value::as_str) {
        if !ws.trim().is_empty() {
            board_meta["workspace_dir"] = Value::String(ws.trim().to_string());
        }
    }
    // 未显式指定 workspace_dir 时,继承发起会话的工作区,使团队产物与用户其他文件同处一地,
    // 而不是落到各自的 orch- 临时会话目录。
    if board_meta.get("workspace_dir").is_none() && !origin_session_id.trim().is_empty() {
        let ws = session_workspace_root(origin_session_id);
        if !ws.trim().is_empty() {
            board_meta["workspace_dir"] = Value::String(ws);
        }
    }

    // 团队看板与任务都由 team.taskboard 提供（create_board / save_task）。
    let provider = &route.provider_candidates;
    let try_call = |action: &str, payload: &Value| -> Result<String, String> {
        let mut last = String::new();
        for p in provider {
            match call_package_ws_action(p, action, payload) {
                Ok(r) => return Ok(r),
                Err(e) => last = e,
            }
        }
        Err(last)
    };

    let create_payload = serde_json::json!({
        "board_id": board_id,
        "session_id": session_id,
        "title": title,
        "status": "active",
        "metadata": board_meta,
    });
    if let Err(e) = try_call("create_board", &create_payload) {
        return PackageResult::err(format!("delegate_to_team create_board failed: {}", e))
            .to_json();
    }

    let task_payload = serde_json::json!({
        "board_id": board_id,
        "task_id": task_id,
        "title": title,
        "description": goal,
        "kind": "feature",
        "status": "ready_for_plan",
        "owner_member_id": "planner",
        "metadata": { "phase": "intake" },
    });
    if let Err(e) = try_call("save_task", &task_payload) {
        return PackageResult::err(format!("delegate_to_team save_task failed: {}", e)).to_json();
    }

    // Emit delegate_started event so the frontend can switch to child session view.
    append_session_event(
        origin_session_id,
        "session.delegate_started",
        serde_json::json!({
            "board_id": board_id,
            "child_session_id": session_id,
            "goal": goal,
            "title": title,
            "started_at": ts,
        }),
    );

    PackageResult::ok(serde_json::json!({
        "board_id": board_id,
        "session_id": session_id,
        "task_id": task_id,
        "goal": goal,
        "title": title,
        "status": "delegated",
    }))
    .to_json()
}

fn execute_ask_user(session_id: &str, args: &Value) -> String {
    let question = args
        .get("question")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if question.is_empty() {
        return PackageResult::err("ask_user requires a non-empty question").to_json();
    }

    let options = args
        .get("options")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| Value::String(value.to_string()))
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default();

    append_session_event(
        session_id,
        "ask_user",
        serde_json::json!({
            "question": question,
            "options": options,
        }),
    );

    PackageResult::ok(serde_json::json!({
        "question": question,
        "options": options,
        "status": "waiting_for_user",
    }))
    .to_json()
}


fn summarize_virtual_capability_output(
    tool: &ToolExecution,
    route: &VirtualCapabilityToolSpec,
) -> Option<String> {
    let data = parse_plugin_ok_data(&tool.output)?;
    match route.summary_kind {
        VirtualCapabilitySummaryKind::PromptSystemRender => {
            let system_prompt = data
                .get("system_prompt")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if system_prompt.is_empty() {
                return Some("Rendered prompt.system successfully.".to_string());
            }
            Some(format!(
                "Rendered prompt.system successfully.\n\nSystem prompt:\n{}",
                system_prompt
            ))
        }
        VirtualCapabilitySummaryKind::WorkflowOrchestrationPlan => {
            let accepted = data
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let workflow = data.get("workflow").cloned().unwrap_or(Value::Null);
            Some(format!(
                "Workflow planning completed through workflow.orchestration. accepted={}\n\nWorkflow payload:\n{}",
                accepted,
                serde_json::to_string_pretty(&workflow).unwrap_or_default()
            ))
        }
        VirtualCapabilitySummaryKind::DelegateToTeam => {
            let board_id = data.get("board_id").and_then(Value::as_str).unwrap_or("");
            let title = data.get("title").and_then(Value::as_str).unwrap_or("");
            Some(format!(
                "已把任务「{}」委托给自治 agent 团队(规划→执行→评审→集成),正在后台运行。board_id={}。团队会自动推进各阶段;进度会在工作区实时显示。",
                title, board_id
            ))
        }
        VirtualCapabilitySummaryKind::AskUser => {
            let question = data.get("question").and_then(Value::as_str).unwrap_or("").trim();
            let options = data
                .get("options")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if options.is_empty() {
                Some(format!("{}", question))
            } else {
                let rendered = options
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<&str>>()
                    .join(" / ");
                Some(format!("{}\n可选项: {}", question, rendered))
            }
        }
        VirtualCapabilitySummaryKind::McpListServers => {
            let servers = data
                .get("servers")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if servers.is_empty() {
                return Some("No MCP servers are currently registered for this agent.".to_string());
            }

            let lines = servers
                .iter()
                .map(|server| {
                    let name = server
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("unnamed")
                        .trim();
                    let command = server
                        .get("command")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim();
                    let transport = server
                        .get("transport")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim();
                    let status = server
                        .get("status")
                        .and_then(|value| value.get("status"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .trim();
                    format!(
                        "- {} (transport: {}, status: {}, command: {})",
                        name,
                        if transport.is_empty() {
                            "unknown"
                        } else {
                            transport
                        },
                        status,
                        if command.is_empty() { "n/a" } else { command }
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");
            Some(format!("Registered MCP servers:\n{}", lines))
        }
        VirtualCapabilitySummaryKind::McpCallTool => {
            let server = data
                .get("server")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .trim();
            let tool_name = data
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .trim();
            let output = data.get("output").cloned().unwrap_or(Value::Null);
            Some(format!(
                "MCP tool call completed via ext.mcp. server={} tool={}\n\nOutput:\n{}",
                server,
                tool_name,
                serde_json::to_string_pretty(&output).unwrap_or_default()
            ))
        }
        VirtualCapabilitySummaryKind::ChannelRegister => Some(
            "Registered the channel configuration through channel.bridge successfully.".to_string(),
        ),
        VirtualCapabilitySummaryKind::ChannelList => {
            let channels = data
                .get("channels")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if channels.is_empty() {
                return Some("No channels are currently registered for this agent.".to_string());
            }

            let lines = channels
                .iter()
                .take(8)
                .map(|channel| format!("- {}", serde_json::to_string(channel).unwrap_or_default()))
                .collect::<Vec<String>>()
                .join("\n");
            Some(format!("Registered channels:\n{}", lines))
        }
        VirtualCapabilitySummaryKind::ChannelSend => {
            let to = data.get("to").and_then(Value::as_str).unwrap_or("unknown");
            Some(format!(
                "Sent a message through channel.bridge successfully to {}.",
                to
            ))
        }
        VirtualCapabilitySummaryKind::BrowserTool => None,
        VirtualCapabilitySummaryKind::SelectorTool => None,
    }
}

#[derive(Deserialize)]
struct RuntimeRoutingConfig {
    #[serde(default)]
    default_provider: String,
}

#[derive(Deserialize)]
struct RuntimeProviderKeyConfig {
    #[serde(default)]
    value: String,
}

#[derive(Deserialize)]
struct RuntimeProviderConfig {
    #[serde(default)]
    name: String,
    #[serde(default)]
    format: String,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    keys: Vec<RuntimeProviderKeyConfig>,
    #[serde(default)]
    response_format_json_schema: Option<bool>,
}

#[derive(Deserialize)]
struct RuntimeCompletionConfig {
    #[serde(default)]
    routing: Option<RuntimeRoutingConfig>,
    #[serde(default)]
    providers: Vec<RuntimeProviderConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StructuredOutputStrategy {
    NativeJsonSchema,
    PromptValidatedJson,
}

fn load_runtime_completion_config() -> Result<RuntimeCompletionConfig, String> {
    let raw = read_file("./config/config.toml")
        .map_err(|error| format!("failed to read runtime config.toml: {}", error))?;
    toml::from_str(&raw).map_err(|error| format!("failed to parse runtime config.toml: {}", error))
}

fn find_runtime_provider_config<'a>(
    config: &'a RuntimeCompletionConfig,
    provider_hint: &str,
) -> Result<&'a RuntimeProviderConfig, String> {
    let preferred_name = if provider_hint.trim().is_empty() {
        config
            .routing
            .as_ref()
            .map(|routing| routing.default_provider.trim().to_string())
            .unwrap_or_default()
    } else {
        provider_hint.trim().to_string()
    };

    config
        .providers
        .iter()
        .find(|entry| !preferred_name.is_empty() && entry.name.trim() == preferred_name)
        .or_else(|| config.providers.first())
        .ok_or_else(|| "runtime config.toml does not contain any providers".to_string())
}

fn provider_family(provider: &RuntimeProviderConfig) -> String {
    let name = provider.name.trim().to_lowercase();
    let format = provider.format.trim().to_lowercase();
    let base_url = provider.base_url.trim().to_lowercase();

    if name.contains("deepseek") || base_url.contains("deepseek") {
        return "deepseek".to_string();
    }
    if name.contains("openai") || base_url.contains("api.openai.com") {
        return "openai".to_string();
    }
    if format == "generic-chat-completion-api" {
        return "generic-chat-completion-api".to_string();
    }
    if !format.is_empty() {
        return format;
    }
    name
}

fn model_family(model: &str) -> String {
    let normalized = model.trim().to_lowercase();
    if normalized.is_empty() {
        return String::new();
    }
    if normalized.starts_with("gpt-5")
        || normalized.starts_with("gpt-4.1")
        || normalized.starts_with("gpt-4o")
        || normalized.starts_with("codex")
    {
        return "openai".to_string();
    }
    if normalized.starts_with('o')
        && normalized
            .chars()
            .nth(1)
            .map(|ch| ch.is_ascii_digit())
            .unwrap_or(false)
    {
        return "openai".to_string();
    }
    String::new()
}

fn structured_output_strategy_for_family_hint(
    family_hint: &str,
) -> Option<StructuredOutputStrategy> {
    let normalized = family_hint.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }

    if normalized.contains("deepseek") {
        return Some(StructuredOutputStrategy::PromptValidatedJson);
    }
    if normalized.contains("openai") || normalized.contains("api.openai.com") {
        return Some(StructuredOutputStrategy::NativeJsonSchema);
    }

    None
}

fn structured_output_strategy_for_provider(
    provider: &RuntimeProviderConfig,
    model_hint: &str,
) -> StructuredOutputStrategy {
    if let Some(enabled) = provider.response_format_json_schema {
        return if enabled {
            StructuredOutputStrategy::NativeJsonSchema
        } else {
            StructuredOutputStrategy::PromptValidatedJson
        };
    }

    let provider_family = provider_family(provider);
    let model_family = model_family(model_hint);

    if provider_family == "generic-chat-completion-api" && model_family == "openai" {
        return StructuredOutputStrategy::PromptValidatedJson;
    }

    match provider_family.as_str() {
        "deepseek" | "generic-chat-completion-api" => StructuredOutputStrategy::PromptValidatedJson,
        _ => StructuredOutputStrategy::NativeJsonSchema,
    }
}

fn resolve_structured_output_strategy(
    provider_hint: &str,
    model_hint: &str,
) -> Result<StructuredOutputStrategy, String> {
    let config = load_runtime_completion_config()?;
    let provider = find_runtime_provider_config(&config, provider_hint)?;
    Ok(structured_output_strategy_for_provider(
        provider, model_hint,
    ))
}

fn resolve_request_structured_output_strategy(
    provider_hint: &str,
    model_hint: &str,
    endpoint_hint: &str,
) -> StructuredOutputStrategy {
    if let Some(strategy) = structured_output_strategy_for_family_hint(provider_hint) {
        return strategy;
    }

    if let Some(strategy) = structured_output_strategy_for_family_hint(endpoint_hint) {
        return strategy;
    }

    if let Ok(strategy) = resolve_structured_output_strategy(provider_hint, model_hint) {
        return strategy;
    }

    StructuredOutputStrategy::PromptValidatedJson
}

fn maybe_attach_schema_response_format(
    body: &mut Value,
    strategy: StructuredOutputStrategy,
    schema_name: &str,
    schema: Value,
) {
    if strategy != StructuredOutputStrategy::NativeJsonSchema {
        return;
    }

    body["response_format"] = serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": schema_name,
            "strict": true,
            "schema": schema,
        }
    });
}

fn load_runtime_completion_target(provider_hint: &str) -> Result<(String, String), String> {
    let parsed = load_runtime_completion_config()?;

    let preferred_name = if provider_hint.trim().is_empty() {
        parsed
            .routing
            .as_ref()
            .map(|routing| routing.default_provider.trim().to_string())
            .unwrap_or_default()
    } else {
        provider_hint.trim().to_string()
    };

    let provider = parsed
        .providers
        .iter()
        .find(|entry| !preferred_name.is_empty() && entry.name.trim() == preferred_name)
        .or_else(|| parsed.providers.first())
        .ok_or_else(|| "runtime config.toml does not contain any providers".to_string())?;

    let strategy = structured_output_strategy_for_provider(provider, "");

    let api_key = provider
        .keys
        .iter()
        .find_map(|entry| {
            let trimmed = entry.value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .ok_or_else(|| {
            format!(
                "provider '{}' does not have an api key",
                provider.name.trim()
            )
        })?;

    let base_url = provider.base_url.trim();
    if base_url.is_empty() {
        return Err(format!(
            "provider '{}' does not have a base_url",
            provider.name.trim()
        ));
    }

    let normalized_base = normalize_openai_compatible_base_url(base_url);
    let completion_path = if normalized_base.ends_with("/responses") {
        normalized_base
    } else if strategy == StructuredOutputStrategy::NativeJsonSchema {
        format!("{}/responses", normalized_base.trim_end_matches('/'))
    } else {
        format!("{}/chat/completions", normalized_base.trim_end_matches('/'))
    };

    Ok((completion_path, api_key))
}

fn normalize_openai_compatible_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    let lowered = trimmed.to_lowercase();

    if lowered.ends_with("/chat/completions") || lowered.ends_with("/responses") {
        return trimmed.to_string();
    }

    if lowered.ends_with("/v1") || lowered.ends_with("/openai/v1") {
        return trimmed.to_string();
    }

    if lowered.contains("code.pumpkinai.vip")
        || lowered.contains("api.openai.com")
        || lowered.contains("api.deepseek.com")
    {
        return format!("{}/v1", trimmed);
    }

    trimmed.to_string()
}

#[cfg(test)]
fn extract_response_text(response_value: &Value) -> String {
    if let Some(text) = response_value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
    {
        return text.to_string();
    }

    if let Some(output) = response_value.get("output").and_then(Value::as_array) {
        let mut parts = Vec::new();
        for item in output {
            if item.get("type").and_then(Value::as_str) != Some("message") {
                continue;
            }
            if let Some(contents) = item.get("content").and_then(Value::as_array) {
                for content in contents {
                    let item_type = content.get("type").and_then(Value::as_str).unwrap_or("");
                    if matches!(item_type, "output_text" | "text") {
                        let text = content
                            .get("text")
                            .or_else(|| content.get("output_text"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .trim();
                        if !text.is_empty() {
                            parts.push(text.to_string());
                        }
                    }
                }
            }
        }
        if !parts.is_empty() {
            return parts.join("\n");
        }
    }

    String::new()
}

fn build_outbound_request_debug_dump(
    request_label: &str,
    endpoint_hint: &str,
    resolved_endpoint: &str,
    target_url: &str,
    provider_hint: &str,
    body_text: &str,
) -> String {
    let body_value = serde_json::from_str::<Value>(body_text)
        .unwrap_or_else(|_| serde_json::json!({ "raw_body": body_text }));
    serde_json::to_string_pretty(&serde_json::json!({
        "request_label": request_label,
        "endpoint_hint": endpoint_hint,
        "resolved_endpoint": resolved_endpoint,
        "target_url": target_url,
        "provider_hint": provider_hint,
        "body": body_value,
        "raw_body": body_text,
    }))
    .unwrap_or_else(|_| body_text.to_string())
}

fn sanitize_completion_debug_text(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace("Bearer ", "[REDACTED_BEARER] ")
        .replace("sk-", "[REDACTED_API_KEY]-")
}

fn responses_request_supports_metadata(target_url: &str, provider_hint: &str) -> bool {
    let provider_text = provider_hint.trim().to_lowercase();
    let target_text = target_url.trim().to_lowercase();
    provider_text == "openai"
        || target_text.contains("api.openai.com")
        || target_text.contains("/openai/")
}

/// 发送 completion HTTP 请求，对瞬时错误（5xx / 传输失败）做有限重试。
/// LLM 网关偶发 502/503/超时很常见，一次抖动不应让整个 agent 回合失败。
/// 4xx（客户端错误，如鉴权/参数）不重试，直接返回。
fn send_completion_with_retry(
    request: &extism_pdk::HttpRequest,
    payload: &str,
) -> Result<String, String> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_err = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        match extism_pdk::http::request::<String>(request, Some(payload.to_string())) {
            Ok(response) => {
                let status = response.status_code();
                let text = String::from_utf8_lossy(&response.body()).to_string();
                if (200..300).contains(&status) {
                    return Ok(text);
                }
                if (400..500).contains(&status) {
                    return Err(format!("completion error: http {} {}", status, text));
                }
                last_err = format!("completion error: http {} {}", status, text);
            }
            Err(error) => {
                last_err = format!("completion transport failed: {}", error);
            }
        }
        if attempt < MAX_ATTEMPTS {
            log_warn(&format!(
                "agent-core completion attempt {}/{} failed, retrying: {}",
                attempt, MAX_ATTEMPTS, last_err
            ));
        }
    }
    Err(last_err)
}

fn request_completion(
    request_label: &str,
    endpoint: &str,
    body_text: &str,
    session_id: &str,
) -> Result<String, String> {
    let request_body = serde_json::from_str::<Value>(body_text).unwrap_or(Value::Null);
    let provider_hint = request_body
        .get("x_provider")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    let model_hint = request_body
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    let structured_strategy =
        resolve_request_structured_output_strategy(&provider_hint, &model_hint, endpoint);
    let resolved = resolve_completion_endpoint(endpoint);
    if !resolved.trim().is_empty() {
        let completion_result = if !session_id.trim().is_empty() {
            chat_completion_stream(request_label, &resolved, body_text, session_id)
        } else {
            chat_completion(request_label, &resolved, body_text)
        };
        match completion_result {
            Ok(response_text) => return Ok(response_text),
            Err(error) => {
                append_debug_log(&format!(
                    "host_chat_completion fallback request_label={} endpoint={} reason={}",
                    request_label,
                    resolved,
                    error.replace('\n', "\\n")
                ));
                log_warn(&format!(
                    "agent-core host_chat_completion fallback label={} endpoint={} reason={}",
                    request_label, resolved, error
                ));
            }
        }
    } else {
        append_debug_log(&format!(
            "host_chat_completion skipped request_label={} reason=empty_endpoint",
            request_label
        ));
    }

    let (target_url, api_key) = load_runtime_completion_target(&provider_hint)?;
    let outbound_dump = build_outbound_request_debug_dump(
        request_label,
        endpoint,
        &resolved,
        &target_url,
        &provider_hint,
        body_text,
    );
    write_file(
        "./data/agent-core-last-outbound-request.json",
        &outbound_dump,
    );

    let mut request = extism_pdk::HttpRequest::new(&target_url)
        .with_method("POST")
        .with_header("Content-Type", "application/json")
        .with_header("Authorization", &format!("Bearer {}", api_key))
        .with_header("User-Agent", "WEFT Desktop/1.0");

    if target_url.ends_with("/responses")
        && structured_strategy == StructuredOutputStrategy::NativeJsonSchema
    {
        let original_body = serde_json::from_str::<Value>(body_text)
            .map_err(|error| format!("completion request body was invalid json: {}", error))?;
        let input_messages = original_body
            .get("messages")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let model = original_body
            .get("model")
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()));
        let temperature = original_body.get("temperature").cloned();
        let response_format = original_body.get("response_format").cloned();
        let mut response_body = serde_json::json!({
            "model": model,
            "input": input_messages,
        });
        if let Some(temp) = temperature {
            response_body["temperature"] = temp;
        }
        if let Some(format) = response_format.and_then(|value| value.get("json_schema").cloned()) {
            response_body["text"] = serde_json::json!({
                "format": {
                    "type": "json_schema",
                    "name": format.get("name").cloned().unwrap_or_else(|| Value::String("agent_turn_plan".to_string())),
                    "schema": format.get("schema").cloned().unwrap_or(Value::Null),
                    "strict": format.get("strict").cloned().unwrap_or(Value::Bool(true)),
                }
            });
        }
        if let Some(provider_value) = original_body.get("x_provider").cloned() {
            let provider_hint_text = provider_value.as_str().unwrap_or("");
            if responses_request_supports_metadata(&target_url, provider_hint_text) {
                response_body["metadata"] = serde_json::json!({ "x_provider": provider_value });
            }
        }
        request = request.with_header("Accept", "application/json");
        let payload_text = serde_json::to_string(&response_body).unwrap_or_default();
        return send_completion_with_retry(&request, &payload_text);
    }

    send_completion_with_retry(&request, body_text)
}

fn normalize_path_candidate(candidate: &str) -> Option<String> {
    let trimmed = candidate
        .trim()
        .trim_start_matches(|ch: char| matches!(ch, '"' | '\'' | '`' | '(' | '[' | '{'))
        .trim_end_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\'' | '`' | ')' | ']' | '}' | ',' | '.' | ';' | ':' | '!' | '?'
            )
        })
        .trim();

    if trimmed.len() < 3 {
        return None;
    }

    let has_drive_prefix = trimmed
        .as_bytes()
        .get(1)
        .copied()
        .map(|byte| byte == b':')
        .unwrap_or(false);
    let has_separator = trimmed.contains('/') || trimmed.contains('\\');
    let looks_relative = trimmed.starts_with("./")
        || trimmed.starts_with(".\\")
        || trimmed.starts_with("../")
        || trimmed.starts_with("..\\");
    let looks_absolute = trimmed.starts_with('/') || trimmed.starts_with('\\');

    if (has_drive_prefix && has_separator) || looks_relative || looks_absolute {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn extract_explicit_file_path(content: &str) -> Option<String> {
    for quote in ['"', '\'', '`'] {
        let parts: Vec<&str> = content.split(quote).collect();
        for segment in parts.iter().skip(1).step_by(2) {
            if let Some(path) = normalize_path_candidate(segment) {
                return Some(path);
            }
        }
    }

    for token in content.split_whitespace() {
        if let Some(path) = normalize_path_candidate(token) {
            return Some(path);
        }
    }

    None
}

fn should_force_fs_read(content: &str) -> Option<String> {
    let path = extract_explicit_file_path(content)?;
    let lower = content.trim().to_lowercase();

    let workflow_intent = lower.contains("workflow")
        || lower.contains("orchestrat")
        || lower.contains("plan")
        || lower.contains("steps")
        || lower.contains("step by step")
        || content.contains("工作流")
        || content.contains("编排")
        || content.contains("流程")
        || content.contains("计划")
        || content.contains("方案")
        || content.contains("步骤")
        || content.contains("规划");

    if workflow_intent {
        return None;
    }

    let explicit_file_request = lower.contains("inspect")
        || lower.contains("read")
        || lower.contains("open")
        || lower.contains("summarize")
        || lower.contains("summary")
        || lower.contains("explain")
        || lower.contains("what is this")
        || content.contains("查看")
        || content.contains("看一下")
        || content.contains("读取")
        || content.contains("打开")
        || content.contains("总结")
        || content.contains("解释")
        || content.contains("鏌ョ湅")
        || content.contains("鐪嬩竴涓")
        || content.contains("璇诲彇")
        || content.contains("鎵撳紑")
        || content.contains("鎬荤粨")
        || content.contains("瑙ｉ噴");

    if explicit_file_request {
        Some(path)
    } else {
        None
    }
}

fn execute_local_tool_route(
    skills_package: &str,
    agent_name: &str,
    tool_name: &str,
    args: Value,
    workspace_root: &str,
) -> String {
    let tool_result =
        execute_skill_action(skills_package, agent_name, tool_name, args.clone(), workspace_root);
    let tool_outputs = vec![ToolExecution {
        name: normalize_js_runtime_tool_name(tool_name),
        args,
        is_error: tool_result_is_error(&tool_result),
        output: tool_result,
    }];
    let final_answer = build_local_tool_answer(&tool_outputs);
    let mut final_reply = if final_answer.trim().is_empty() {
        String::new()
    } else {
        final_answer
    };
    if final_reply.trim().is_empty() {
        final_reply = "Done.".to_string();
    }
    final_reply.push_str(&build_tool_markers(&tool_outputs));
    final_reply
}

fn extract_first_json_object(raw: &str) -> Option<String> {
    let chars: Vec<char> = raw.chars().collect();
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in chars.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if start.is_none() {
                    start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    if let Some(begin) = start {
                        return Some(chars[begin..=index].iter().collect());
                    }
                }
            }
            _ => {}
        }
    }

    None
}

fn extract_response_message(response_value: &Value) -> Option<Value> {
    response_value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .cloned()
        .or_else(|| {
            response_value
                .get("output")
                .and_then(Value::as_array)
                .and_then(|items| {
                    items.iter().find_map(|item| {
                        if item.get("type").and_then(Value::as_str) == Some("message") {
                            Some(item)
                        } else {
                            None
                        }
                    })
                })
                .map(build_message_from_responses_output)
        })
}

fn build_message_from_responses_output(message_item: &Value) -> Value {
    let mut content_segments = Vec::new();
    let mut tool_calls = Vec::new();

    if let Some(contents) = message_item.get("content").and_then(Value::as_array) {
        for content in contents {
            let item_type = content.get("type").and_then(Value::as_str).unwrap_or("");
            match item_type {
                "output_text" | "text" => {
                    let text = content
                        .get("text")
                        .or_else(|| content.get("output_text"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim();
                    if !text.is_empty() {
                        content_segments.push(text.to_string());
                    }
                }
                "tool_call" | "function_call" => {
                    let name = content
                        .get("name")
                        .or_else(|| content.get("tool_name"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim();
                    if name.is_empty() {
                        continue;
                    }
                    let arguments = content
                        .get("arguments")
                        .or_else(|| content.get("arguments_json"))
                        .and_then(Value::as_str)
                        .unwrap_or("{}")
                        .trim()
                        .to_string();
                    tool_calls.push(serde_json::json!({
                        "function": {
                            "name": name,
                            "arguments": if arguments.is_empty() { "{}" } else { arguments.as_str() }
                        }
                    }));
                }
                _ => {}
            }
        }
    }

    serde_json::json!({
        "content": content_segments.join("\n").trim().to_string(),
        "tool_calls": tool_calls,
    })
}

fn parse_agent_turn_plan(response_value: &Value) -> Result<AgentTurnPlan, String> {
    let message = extract_response_message(response_value).ok_or_else(|| {
        "agent turn response did not contain a supported message payload".to_string()
    })?;
    let raw_content = message["content"].as_str().unwrap_or("").trim();

    if raw_content.is_empty() {
        if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
            if !tool_calls.is_empty() {
                let mut normalized_calls = Vec::new();
                for entry in tool_calls {
                    let function = entry.get("function").unwrap_or(entry);
                    let name = function
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let arguments_json = function
                        .get("arguments")
                        .or_else(|| function.get("arguments_json"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or("{}")
                        .to_string();
                    normalized_calls.push(PlannedToolCall {
                        name,
                        arguments_json,
                    });
                }

                if !normalized_calls.is_empty() {
                    return Ok(AgentTurnPlan {
                        mode: "tool".to_string(),
                        assistant: String::new(),
                        tool_calls: normalized_calls,
                    });
                }
            }
        }

        return Err("agent turn plan was empty".to_string());
    }

    let parsed_value = parse_plan_json_value(raw_content)?;

    // planner 纯规划 schema 会带 subtasks 数组(planning_only)。把它序列化进 assistant,
    // 让 orchestrator 的 parse_planned_subtasks 拿到干净 JSON 做扇出。
    if let Some(subtasks) = parsed_value.get("subtasks").and_then(Value::as_array) {
        if !subtasks.is_empty() {
            return Ok(AgentTurnPlan {
                mode: "reply".to_string(),
                assistant: serde_json::to_string(subtasks).unwrap_or_default(),
                tool_calls: Vec::new(),
            });
        }
    }

    serde_json::from_value(parsed_value)
        .map_err(|error| format!("agent turn plan schema mismatch: {}", error))
}

fn parse_plan_json_value(raw_content: &str) -> Result<Value, String> {
    let raw_content = raw_content.trim();
    if let Ok(value) = serde_json::from_str::<Value>(raw_content) {
        return Ok(value);
    }
    if let Some(json_blob) = extract_first_json_object(raw_content) {
        if let Ok(value) = serde_json::from_str::<Value>(&json_blob) {
            return Ok(value);
        }
        let repaired = repair_common_json_escapes(&json_blob);
        return serde_json::from_str::<Value>(&repaired)
            .map_err(|error| format!("agent turn plan json parse failed: {}", error));
    }
    Err("agent turn plan did not contain valid json".to_string())
}

fn repair_common_json_escapes(raw: &str) -> String {
    let mut repaired = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if !in_string {
            if ch == '"' {
                in_string = true;
            }
            repaired.push(ch);
            continue;
        }
        if escaped {
            repaired.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => {
                let valid_escape = matches!(
                    chars.peek(),
                    Some('"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u')
                );
                if valid_escape {
                    repaired.push(ch);
                    escaped = true;
                } else {
                    repaired.push_str("\\\\");
                }
            }
            '"' => {
                let mut lookahead = chars.clone();
                while matches!(lookahead.peek(), Some(next) if next.is_whitespace()) {
                    lookahead.next();
                }
                let terminates = matches!(lookahead.peek(), Some(',' | '}' | ']' | ':'));
                if terminates {
                    in_string = false;
                    repaired.push(ch);
                } else {
                    repaired.push_str("\\\"");
                }
            }
            '\n' => repaired.push_str("\\n"),
            '\r' => repaired.push_str("\\r"),
            '\t' => repaired.push_str("\\t"),
            other => repaired.push(other),
        }
    }
    repaired
}

fn parse_tool_arguments_json(raw: &str) -> Result<Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str::<Value>(trimmed).or_else(|first_error| {
        let repaired = repair_common_json_escapes(trimmed);
        serde_json::from_str::<Value>(&repaired).map_err(|second_error| {
            format!(
                "agent turn plan arguments_json parse failed: {}; repair failed: {}",
                first_error, second_error
            )
        })
    })
}

fn repair_agent_turn_plan(
    config: &AgentConfig,
    response_value: &Value,
    parse_error: &str,
    tool_names: &[String],
    delegate_request: Option<&DelegateRequestInput>,
) -> Result<AgentTurnPlan, String> {
    let message = extract_response_message(response_value).ok_or_else(|| {
        "agent turn repair response did not contain a supported message payload".to_string()
    })?;
    let raw_content = message.get("content").and_then(Value::as_str).unwrap_or("");
    let schema = build_agent_turn_plan_schema(tool_names, None);
    let system = "You repair malformed JSON for an agent turn plan. Return JSON only. Output exactly one object matching the provided schema. Preserve the intended tool calls and arguments. arguments_json values must be valid compact JSON strings with inner quotes escaped.";
    let user_prompt = [
        format!("Parse error: {}", parse_error),
        format!(
            "Required schema:\n{}",
            serde_json::to_string_pretty(&schema).unwrap_or_default()
        ),
        format!("Malformed content:\n{}", raw_content),
        "Repair the agent turn plan now.".to_string(),
    ]
    .join("\n\n");
    let response_format_strategy = resolve_request_structured_output_strategy(
        &config.provider,
        &config.model,
        &config.completion_endpoint,
    );
    let mut body = serde_json::json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt},
        ],
        "temperature": 0.0,
    });
    maybe_attach_schema_response_format(
        &mut body,
        response_format_strategy,
        "agent_turn_plan_repair",
        schema,
    );
    if !config.provider.trim().is_empty() {
        body["x_provider"] = serde_json::json!(config.provider.trim());
    }
    apply_delegate_model_override(&mut body, delegate_request);
    let response_text = request_completion(
        "agent_turn_plan_repair",
        &config.completion_endpoint,
        &serde_json::to_string(&body).unwrap_or_default(),
        "",
    )?;
    let repaired_value: Value = serde_json::from_str(&response_text).unwrap_or_default();
    parse_agent_turn_plan(&repaired_value)
}

fn required_tool_fields(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn arguments_missing_required_fields(args: &Value, schema: &Value) -> bool {
    let required = required_tool_fields(schema);
    if required.is_empty() {
        return false;
    }

    let object = match args.as_object() {
        Some(object) => object,
        None => return true,
    };

    required.iter().any(|field| {
        let Some(value) = object.get(field) else {
            return true;
        };
        match value {
            Value::Null => true,
            Value::String(text) => text.trim().is_empty(),
            Value::Array(items) => items.is_empty(),
            Value::Object(map) => map.is_empty(),
            _ => false,
        }
    })
}

fn repair_tool_arguments(
    config: &AgentConfig,
    tool_name: &str,
    tool_schema: &Value,
    content: &str,
    delegate_request: Option<&DelegateRequestInput>,
    bad_arguments_json: &str,
) -> Result<String, String> {
    let mut system = String::new();
    system.push_str("You repair invalid tool arguments for an already-delegated agent action.\n");
    system.push_str("Return JSON only.\n");
    system.push_str("Output one object with a single field: arguments_json.\n");
    system.push_str(
        "arguments_json must be a compact JSON string matching the provided tool schema.\n",
    );
    system.push_str("Fill every required field.\n");
    system.push_str(
        "Do not leave arguments_json empty or {} when the delegated context resolves the target.\n",
    );
    if let Some(delegate_contract) = build_delegate_contract_block(delegate_request) {
        system.push_str("\n\n");
        system.push_str(&delegate_contract);
    }

    let user_prompt = [
        format!("Tool name: {}", tool_name),
        format!(
            "Tool schema:\n{}",
            serde_json::to_string_pretty(tool_schema).unwrap_or_default()
        ),
        format!("Current user content: {}", content),
        format!("Invalid arguments_json: {}", bad_arguments_json.trim()),
        "Repair the arguments now.".to_string(),
    ]
    .join("\n\n");

    let response_format_strategy = resolve_request_structured_output_strategy(
        &config.provider,
        &config.model,
        &config.completion_endpoint,
    );

    let mut body = serde_json::json!({
        "model": config.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user_prompt },
        ],
        "temperature": 0.0,
    });
    maybe_attach_schema_response_format(
        &mut body,
        response_format_strategy,
        "tool_argument_repair",
        serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "arguments_json": { "type": "string" }
            },
            "required": ["arguments_json"]
        }),
    );
    if !config.provider.trim().is_empty() {
        body["x_provider"] = serde_json::json!(config.provider.trim());
    }
    apply_delegate_model_override(&mut body, delegate_request);

    let response_text = request_completion(
        "tool_argument_repair",
        &config.completion_endpoint,
        &serde_json::to_string(&body).unwrap_or_default(),
        "",
    )?;
    let response_value: Value = serde_json::from_str(&response_text).unwrap_or_default();
    let message = extract_response_message(&response_value).ok_or_else(|| {
        "tool argument repair response did not contain a supported message payload".to_string()
    })?;
    let raw_content = message["content"].as_str().unwrap_or("").trim();
    if raw_content.is_empty() {
        return Err("tool argument repair returned empty content".to_string());
    }

    let parsed_value = if let Ok(value) = serde_json::from_str::<Value>(raw_content) {
        value
    } else if let Some(json_blob) = extract_first_json_object(raw_content) {
        serde_json::from_str::<Value>(&json_blob)
            .map_err(|error| format!("tool argument repair json parse failed: {}", error))?
    } else {
        return Err("tool argument repair did not contain valid json".to_string());
    };

    parsed_value
        .get("arguments_json")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "tool argument repair returned empty arguments_json".to_string())
}

fn run_agent_completion(
    session_id: &str,
    config: &AgentConfig,
    content: &str,
    history: &[Value],
    delegate_request: Option<&DelegateRequestInput>,
) -> Result<(String, String, String, String), String> {
    log_info(&format!(
        "agent-core run_agent_completion start agent={}",
        config.name
    ));
    let skills_package_name = non_empty_or(&config.skills_package, "skills").to_string();
    let evolved_skills =
        retrieve_applicable_evolved_skills(&skills_package_name, &config.name, content);
    let mut trajectory = new_agent_trajectory(&config.name, content);
    trajectory.injected_skill_ids = evolved_skills.skill_ids.clone();
    if !evolved_skills.context_block.trim().is_empty() {
        trajectory.steps.push(format!(
            "retrieved evolved skills: {}",
            trajectory.injected_skill_ids.join(", ")
        ));
    }
    save_agent_trajectory(&trajectory);

    let (_, memory_package, skills_package, channels_package, tool_schemas, _) =
        build_completion_request(
            config,
            content,
            history,
            delegate_request,
            None,
            Some(&evolved_skills.context_block),
        );
    if false {
        #[allow(unreachable_code)]
        if let Some(path) = should_force_fs_read(content) {
            if !std::path::Path::new(&path).is_dir() {
                append_debug_log(&format!(
                    "direct fs_read route agent={} path={}",
                    config.name,
                    path.replace('\n', "\\n")
                ));
                trajectory
                    .steps
                    .push(format!("direct tool route selected: fs_read path={}", path));
                trajectory.tool_names.push("fs_read".into());
                let execute_input = serde_json::json!({ "path": path });
                let final_reply = execute_local_tool_route(
                    &skills_package,
                    &config.name,
                    "fs_read",
                    execute_input,
                    &session_workspace_root(session_id),
                );
                append_debug_log(&format!(
                    "direct fs_read route done agent={} result_len={}",
                    config.name,
                    final_reply.len()
                ));
                finish_agent_trajectory_success(config, &mut trajectory, &final_reply);
                return Ok((
                    final_reply,
                    resolve_completion_endpoint(&config.completion_endpoint),
                    memory_package,
                    channels_package,
                ));
            }
        }
    }
    let trajectory_steps = RefCell::new(trajectory.steps.clone());
    let trajectory_tools = RefCell::new(trajectory.tool_names.clone());
    // Per-trajectory cache accounting. Aggregated across all rounds in this turn
    // so the trajectory record exposes a single ratio of `cache_read / prompt_tokens`
    // — the headline metric for whether the prefix-stable refactor is paying off.
    let trajectory_prompt_tokens = RefCell::new(0u64);
    let trajectory_completion_tokens = RefCell::new(0u64);
    let trajectory_cache_read = RefCell::new(0u64);
    let trajectory_cache_creation = RefCell::new(0u64);
    let trajectory_cache_miss = RefCell::new(0u64);
    // Prefix fingerprint tracker. The first round records the fingerprint; later
    // rounds compare and bump `invalidation_count` if the bytes shifted.
    let trajectory_prefix_fp = RefCell::new(String::new());
    let trajectory_prefix_invalidations = RefCell::new(0u64);
    let final_reply_result = execute_agent_turn_plan_loop(
        content,
        delegate_request.map(|r| r.planning_only).unwrap_or(false),
        |round, follow_up_prompt| {
            let (body_text, _, _, _, _, round_tool_names) = build_completion_request(
                config,
                content,
                history,
                delegate_request,
                follow_up_prompt,
                Some(&evolved_skills.context_block),
            );
            // Prefix fingerprint: extract the first system message from the
            // serialized body (which is the stable block after P1) and hash it.
            // Compare with the prior round's fingerprint to detect invalidation.
            if let Ok(body_value) = serde_json::from_str::<Value>(&body_text) {
                let prefix_text = body_value
                    .get("messages")
                    .and_then(Value::as_array)
                    .and_then(|msgs| msgs.first())
                    .and_then(|msg| {
                        // Non-Anthropic: {"role":"system","content":"..."}
                        msg.get("content").and_then(|c| {
                            if let Some(s) = c.as_str() {
                                Some(s.to_string())
                            } else if let Some(arr) = c.as_array() {
                                // Anthropic: content is [{type,text,cache_control}]
                                arr.first()
                                    .and_then(|item| item.get("text"))
                                    .and_then(Value::as_str)
                                    .map(|s| s.to_string())
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_default();
                if !prefix_text.is_empty() {
                    let fp = fingerprint_str(&prefix_text);
                    let mut prev = trajectory_prefix_fp.borrow_mut();
                    if prev.is_empty() {
                        // First round of this turn — also compare against the
                        // fingerprint persisted from the previous turn. Drift
                        // here means something invalidated the cache *between*
                        // user messages (e.g. agent system prompt edited via
                        // dashboard, MCP tool list changed, evolved-skill TTL
                        // expired and produced a different block). The
                        // breadcrumb in trajectory.steps lets ops bisect.
                        let prior_turn_fp = load_session_prefix_fp(session_id);
                        if !prior_turn_fp.is_empty() && prior_turn_fp != fp {
                            *trajectory_prefix_invalidations.borrow_mut() += 1;
                            trajectory_steps.borrow_mut().push(format!(
                                "[cross-turn-prefix-invalidation] prev={} new={}",
                                &prior_turn_fp, &fp
                            ));
                        }
                        *prev = fp;
                    } else if *prev != fp {
                        *trajectory_prefix_invalidations.borrow_mut() += 1;
                        trajectory_steps.borrow_mut().push(format!(
                            "[prefix-invalidation] round={} prev={} new={}",
                            round + 1,
                            &*prev,
                            &fp
                        ));
                        *prev = fp;
                    }
                }
            }
            append_debug_log(&format!(
            "run_agent_completion round_start agent={} round={} history_len={} request_len={} content={}",
            config.name,
            round + 1,
            history.len(),
            body_text.len(),
            content.replace('\n', "\\n")
        ));
            if delegate_request_requires_real_action(delegate_request) {
                write_file("./data/agent-core-last-request.json", &body_text);
            }
            log_info(&format!(
                "agent-core run_agent_completion completion start agent={} round={}",
                config.name,
                round + 1
            ));
            append_debug_log(&format!(
                "host completion start agent={} round={} endpoint={}",
                config.name,
                round + 1,
                resolve_completion_endpoint(&config.completion_endpoint)
            ));
            let response_text =
                request_completion("agent_turn", &config.completion_endpoint, &body_text, session_id)?;
            if delegate_request_requires_real_action(delegate_request) {
                let sanitized_response = sanitize_completion_debug_text(&response_text);
                write_file("./data/agent-core-last-response.json", &sanitized_response);
            }
            append_debug_log(&format!(
                "host completion done agent={} round={} response_len={}",
                config.name,
                round + 1,
                response_text.len()
            ));
            log_info(&format!(
                "agent-core run_agent_completion completion done agent={} round={}",
                config.name,
                round + 1
            ));
            let response_value: Value = serde_json::from_str(&response_text).unwrap_or_default();
            let mut repaired_plan_error: Option<String> = None;
            let parsed_message = extract_response_message(&response_value);
            append_debug_log(&format!(
                "host completion parsed agent={} round={} has_error={} has_tool_calls={}",
                config.name,
                round + 1,
                response_value.get("error").is_some(),
                parsed_message
                    .as_ref()
                    .and_then(|message| message.get("tool_calls"))
                    .and_then(Value::as_array)
                    .map(|items| !items.is_empty())
                    .unwrap_or(false)
            ));

            if let Some(error_message) = parse_completion_error(&response_value) {
                return Err(format!("completion error: {}", error_message));
            }

            let plan = match parse_agent_turn_plan(&response_value) {
                Ok(plan) => plan,
                Err(error) => {
                    match repair_agent_turn_plan(config, &response_value, &error, &round_tool_names, delegate_request)
                    {
                        Ok(plan) => {
                            repaired_plan_error = Some(error);
                            plan
                        }
                        Err(repair_error) => {
                            return Err(format!("{}; repair failed: {}", error, repair_error));
                        }
                    }
                }
            };
            trajectory_steps.borrow_mut().push(format!(
                "round {} planned mode={} tool_count={}",
                round + 1,
                plan.mode,
                plan.tool_calls.len()
            ));
            append_debug_log(&format!(
                "agent turn plan parsed agent={} round={} mode={} tool_count={}{}",
                config.name,
                round + 1,
                plan.mode,
                plan.tool_calls.len(),
                repaired_plan_error
                    .as_ref()
                    .map(|error| format!(" repaired_from_error={}", error.replace('\n', "\\n")))
                    .unwrap_or_default()
            ));
            if let Some(usage) = response_value.get("usage") {
                let input_tokens = usage.get("input_tokens")
                    .or_else(|| usage.get("prompt_tokens"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let output_tokens = usage.get("output_tokens")
                    .or_else(|| usage.get("completion_tokens"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                // Anthropic: cache_read_input_tokens / cache_creation_input_tokens
                // DeepSeek:  prompt_cache_hit_tokens / prompt_cache_miss_tokens
                let cache_read = usage.get("cache_read_input_tokens")
                    .or_else(|| usage.get("prompt_cache_hit_tokens"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let cache_creation = usage.get("cache_creation_input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let cache_miss = usage.get("prompt_cache_miss_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                *trajectory_prompt_tokens.borrow_mut() += input_tokens;
                *trajectory_completion_tokens.borrow_mut() += output_tokens;
                *trajectory_cache_read.borrow_mut() += cache_read;
                *trajectory_cache_creation.borrow_mut() += cache_creation;
                *trajectory_cache_miss.borrow_mut() += cache_miss;
                append_session_event(
                    session_id,
                    "cost",
                    serde_json::json!({
                        "agent": config.name,
                        "model": config.model,
                        "round": round + 1,
                        "input_tokens": input_tokens,
                        "output_tokens": output_tokens,
                        "cache_read_input_tokens": cache_read,
                        "cache_creation_input_tokens": cache_creation,
                        "prompt_cache_miss_tokens": cache_miss,
                    }),
                );
            }
            Ok(plan)
        },
        |round, tool_call| {
            let function_name = tool_call.name.trim().to_string();
            if function_name.is_empty() {
                return Err("agent turn plan contained empty tool name".to_string());
            }

            let mut function_args_str = tool_call.arguments_json.trim().to_string();
            let mut function_args: Value = parse_tool_arguments_json(&function_args_str)?;
            if delegate_request_requires_real_action(delegate_request) {
                if let Some(tool_schema) = tool_schemas.get(&function_name) {
                    if arguments_missing_required_fields(&function_args, tool_schema) {
                        function_args_str = repair_tool_arguments(
                            config,
                            &function_name,
                            tool_schema,
                            content,
                            delegate_request,
                            &function_args_str,
                        )?;
                        function_args =
                            parse_tool_arguments_json(&function_args_str).map_err(|error| {
                                format!("repaired arguments_json parse failed: {}", error)
                            })?;
                    }
                }
            }
            append_debug_log(&format!(
                "execute_tool start agent={} round={} tool={} args={}",
                config.name,
                round + 1,
                function_name,
                function_args_str.replace('\n', "\\n")
            ));
            trajectory_steps.borrow_mut().push(format!(
                "round {} executed tool {}",
                round + 1,
                function_name
            ));
            {
                let mut tools = trajectory_tools.borrow_mut();
                if !tools.iter().any(|tool| tool == &function_name) {
                    tools.push(function_name.clone());
                }
            }

            log_info(&format!(
                "agent-core run_agent_completion execute_tool start agent={} round={} tool={}",
                config.name,
                round + 1,
                function_name
            ));
            let tool_call_id = format!(
                "{}-r{}-{}-{}",
                config.name.replace(':', "-"),
                round + 1,
                function_name.replace(':', "-"),
                now_ms()
            );
            append_session_event(
                session_id,
                "tool.started",
                serde_json::json!({
                    "tool_call_id": tool_call_id,
                    "agent": config.name,
                    "round": round + 1,
                    "tool": function_name,
                    // fs_write 保留完整 content（工作区预览/内嵌 webview 需要完整 HTML，
                    // 否则截断到 1000 会导致 <style>/<body> 不闭合、渲染空白）；
                    // 其他工具仍用截断预览，避免事件过大。
                    "args": if function_name == "fs_write" {
                        function_args.clone()
                    } else {
                        tool_args_preview(&function_args)
                    },
                    "started_at": now_ms(),
                }),
            );
            // Hook: before_tool_call — external packages can observe or cancel via KV.
            append_session_event(
                session_id,
                "hook.before_tool_call",
                serde_json::json!({
                    "tool_call_id": tool_call_id,
                    "agent": config.name,
                    "round": round + 1,
                    "tool": function_name,
                    "args": tool_args_preview(&function_args),
                }),
            );
            // Approval gate: check KV deny flag for dangerous tools.
            const DANGEROUS_TOOLS: &[&str] = &["shell_exec", "fs_write", "git"];
            let deny_key = format!("deny:{}:{}", session_id, function_name);
            let is_denied = DANGEROUS_TOOLS.contains(&function_name.as_str())
                && kv_get(&deny_key).as_deref() == Some("1");
            if is_denied {
                let _ = kv_delete(&deny_key);
                append_session_event(
                    session_id,
                    "tool.denied",
                    serde_json::json!({
                        "tool_call_id": tool_call_id,
                        "agent": config.name,
                        "round": round + 1,
                        "tool": function_name,
                    }),
                );
                return Err(format!("tool {} was denied by approval policy", function_name));
            }
            // Emit approval_request for dangerous tools so the frontend can surface a prompt.
            // The agent proceeds immediately (non-blocking); denial must be set in KV before
            // the next call to the same tool.
            if DANGEROUS_TOOLS.contains(&function_name.as_str()) {
                append_session_event(
                    session_id,
                    "approval_request",
                    serde_json::json!({
                        "tool_call_id": tool_call_id,
                        "agent": config.name,
                        "round": round + 1,
                        "tool": function_name,
                        "args": tool_args_preview(&function_args),
                        "deny_key": deny_key,
                    }),
                );
            }
            if function_name == "shell_exec" || function_name == "git" {
                append_session_event(
                    session_id,
                    "shell.exec",
                    serde_json::json!({
                        "tool_call_id": tool_call_id,
                        "agent": config.name,
                        "round": round + 1,
                        "tool": function_name,
                        "command": function_args.get("command").cloned().unwrap_or(Value::Null),
                        "args": function_args.get("args").cloned().unwrap_or(Value::Null),
                        "cwd": function_args
                            .get("cwd")
                            .or_else(|| function_args.get("workdir"))
                            .cloned()
                            .unwrap_or(Value::Null),
                    }),
                );
            }
            let tool_result =
                if let Some(route) = find_virtual_capability_tool(config, &function_name) {
                    execute_virtual_capability_tool(session_id, config, &route, &function_args)
                } else {
                    execute_skill_action(
                        &skills_package,
                        &config.name,
                        &function_name,
                        function_args.clone(),
                        &session_workspace_root(session_id),
                    )
                };
            let is_error = tool_result_is_error(&tool_result);
            let display = build_tool_display(&function_name, &tool_result, is_error);
            append_session_event(
                session_id,
                "tool.finished",
                serde_json::json!({
                    "tool_call_id": tool_call_id,
                    "agent": config.name,
                    "round": round + 1,
                    "tool": function_name,
                    "status": if is_error { "error" } else { "ok" },
                    "output_preview": truncate_event_text(&tool_result, 4000),
                    "output_len": tool_result.len(),
                    "finished_at": now_ms(),
                    "display": display,
                }),
            );
            // Hook: after_tool_call — fire-and-forget notification.
            append_session_event(
                session_id,
                "hook.after_tool_call",
                serde_json::json!({
                    "tool_call_id": tool_call_id,
                    "agent": config.name,
                    "round": round + 1,
                    "tool": function_name,
                    "status": if is_error { "error" } else { "ok" },
                }),
            );
            if function_name == "fs_write" && !is_error {
                let written_path = function_args
                    .get("path")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                append_session_event(
                    session_id,
                    "file.write",
                    serde_json::json!({
                        "tool_call_id": tool_call_id,
                        "agent": config.name,
                        "round": round + 1,
                        "path": function_args.get("path").cloned().unwrap_or(Value::Null),
                        "bytes": function_args
                            .get("content")
                            .and_then(Value::as_str)
                            .map(|value| value.len())
                            .unwrap_or(0),
                    }),
                );
                // artifact 回填:把写过的文件路径累加到 KV,供 orchestrator 在 execute 推进时
                // 读取并存入 task.artifact_refs(run_agent_completion 不把工具产出回传上层)。
                if let Some(path) = written_path {
                    record_artifact(session_id, &path);
                }
            }
            log_info(&format!(
                "agent-core run_agent_completion execute_tool done agent={} round={} tool={}",
                config.name,
                round + 1,
                function_name
            ));
            append_debug_log(&format!(
                "execute_tool done agent={} round={} tool={} result_len={}",
                config.name,
                round + 1,
                function_name,
                tool_result.len()
            ));

            Ok(ToolExecution {
                name: normalize_js_runtime_tool_name(&function_name),
                args: function_args,
                is_error,
                output: tool_result,
            })
        },
    );

    let final_reply = match final_reply_result {
        Ok(reply) => reply,
        Err(error) => {
            trajectory.steps = trajectory_steps.into_inner();
            trajectory.tool_names = trajectory_tools.into_inner();
            trajectory.prompt_tokens = trajectory_prompt_tokens.into_inner();
            trajectory.completion_tokens = trajectory_completion_tokens.into_inner();
            trajectory.cache_read_tokens = trajectory_cache_read.into_inner();
            trajectory.cache_creation_tokens = trajectory_cache_creation.into_inner();
            trajectory.cache_miss_tokens = trajectory_cache_miss.into_inner();
            trajectory.last_prefix_fingerprint = trajectory_prefix_fp.into_inner();
            trajectory.prefix_invalidation_count = trajectory_prefix_invalidations.into_inner();
            // Persist even on failure: a failed turn that succeeded in sending
            // the request still established a prefix the upstream cached, and
            // the *next* turn should compare against it.
            save_session_prefix_fp(session_id, &trajectory.last_prefix_fingerprint);
            finish_agent_trajectory_failure(config, &mut trajectory, &error);
            return Err(error);
        }
    };
    trajectory.steps = trajectory_steps.into_inner();
    trajectory.tool_names = trajectory_tools.into_inner();
    trajectory.prompt_tokens = trajectory_prompt_tokens.into_inner();
    trajectory.completion_tokens = trajectory_completion_tokens.into_inner();
    trajectory.cache_read_tokens = trajectory_cache_read.into_inner();
    trajectory.cache_creation_tokens = trajectory_cache_creation.into_inner();
    trajectory.cache_miss_tokens = trajectory_cache_miss.into_inner();
    trajectory.last_prefix_fingerprint = trajectory_prefix_fp.into_inner();
    trajectory.prefix_invalidation_count = trajectory_prefix_invalidations.into_inner();
    save_session_prefix_fp(session_id, &trajectory.last_prefix_fingerprint);
    finish_agent_trajectory_success(config, &mut trajectory, &final_reply);

    log_info(&format!(
        "agent-core run_agent_completion return agent={}",
        config.name
    ));
    append_debug_log(&format!(
        "run_agent_completion return agent={} final_len={}",
        config.name,
        final_reply.len()
    ));
    Ok((
        final_reply,
        resolve_completion_endpoint(&config.completion_endpoint),
        memory_package,
        channels_package,
    ))
}

#[cfg(test)]
fn run_agent_completion_plan_loop_for_test<PlanFn, ToolFn>(
    content: &str,
    plan_round: PlanFn,
    execute_tool_call: ToolFn,
) -> Result<String, String>
where
    PlanFn: FnMut(usize, Option<&str>) -> Result<AgentTurnPlan, String>,
    ToolFn: FnMut(usize, PlannedToolCall) -> Result<ToolExecution, String>,
{
    execute_agent_turn_plan_loop(content, false, plan_round, execute_tool_call)
}

fn do_send_message_with_delegate(
    agent_name: &str,
    content: &str,
    delegate_request: Option<&DelegateRequestInput>,
) -> PackageResult {
    let config = match load_agent(agent_name) {
        Some(config) => config,
        None => return PackageResult::err(format!("agent '{}' not found", agent_name)),
    };

    let mut history = get_history(agent_name);
    history.push(serde_json::json!({"role": "user", "content": content}));

    let (final_reply, endpoint, memory_package, channels_package) =
        match run_agent_completion(
            session_id_from_agent_name(agent_name).as_deref().unwrap_or(""),
            &config,
            content,
            &history,
            delegate_request,
        ) {
            Ok(result) => result,
            Err(error) => return PackageResult::err(error),
        };
    let visible_reply = visible_assistant_reply(&final_reply);
    append_debug_log(&format!(
        "do_send_message_with_delegate completion_ready agent={} visible_len={}",
        agent_name,
        visible_reply.len()
    ));

    history.push(serde_json::json!({"role": "assistant", "content": visible_reply}));
    if history.len() > 50 {
        history = history[history.len() - 50..].to_vec();
    }
    append_debug_log(&format!(
        "do_send_message_with_delegate save_history_begin agent={}",
        agent_name
    ));
    save_history(agent_name, &history);
    append_debug_log(&format!(
        "do_send_message_with_delegate save_history_done agent={}",
        agent_name
    ));

    append_debug_log(&format!(
        "do_send_message_with_delegate return agent={}",
        agent_name
    ));
    PackageResult::ok(serde_json::json!({
        "reply": final_reply,
        "model": config.model,
        "provider": config.provider,
        "completion_endpoint": endpoint,
        "memory_package": memory_package,
        "channels_package": channels_package,
    }))
}

fn do_list_sessions() -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let mut sessions: Vec<SessionRecord> = get_sessions_index()
        .iter()
        .filter_map(|id| load_session(id))
        .collect();
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));

    PackageResult::ok(serde_json::json!({
        "sessions": sessions.into_iter().map(|session| serde_json::json!({
            "id": session.id,
            "title": session.title,
            "workspace_id": session.workspace_id,
            "workspace_root": session.workspace_root,
            "persistent": session.persistent,
            "created_at": session.created_at,
            "updated_at": session.updated_at,
            "agent_name": session.agent_name,
            "bound_skills": session.agent.skills,
        })).collect::<Vec<Value>>()
    }))
}

fn do_list_workspaces() -> PackageResult {
    let mut workspaces: Vec<WorkspaceRecord> = get_workspaces_index()
        .iter()
        .filter_map(|id| load_workspace(id))
        .collect();
    workspaces.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));

    PackageResult::ok(serde_json::json!({
        "workspaces": workspaces.into_iter().map(|workspace| serde_json::json!({
            "id": workspace.id,
            "name": workspace.name,
            "root_path": workspace.root_path,
            "created_at": workspace.created_at,
            "updated_at": workspace.updated_at,
        })).collect::<Vec<Value>>()
    }))
}

fn do_create_workspace(input: CreateWorkspaceInput) -> PackageResult {
    if input.id.trim().is_empty() {
        return PackageResult::err("missing workspace id");
    }

    let name = decode_transport_text(&input.name, &input.name_b64);
    let mut workspace = load_workspace(&input.id).unwrap_or_else(|| WorkspaceRecord {
        id: input.id.clone(),
        name: name.clone(),
        root_path: input.root_path.clone(),
        created_at: input.created_at,
        updated_at: input.updated_at,
    });

    workspace.name = name;
    workspace.root_path = input.root_path;
    workspace.created_at = if workspace.created_at == 0 {
        input.created_at
    } else {
        workspace.created_at.min(input.created_at)
    };
    workspace.updated_at = workspace.updated_at.max(input.updated_at);
    save_workspace(&workspace);

    let mut index = get_workspaces_index();
    if !index.iter().any(|entry| entry == &workspace.id) {
        index.push(workspace.id.clone());
        save_workspaces_index(&index);
    }

    PackageResult::ok(serde_json::json!({
        "id": workspace.id,
        "name": workspace.name,
        "root_path": workspace.root_path,
        "created_at": workspace.created_at,
        "updated_at": workspace.updated_at,
    }))
}

fn do_delete_workspace(input: WorkspaceIdInput) -> PackageResult {
    kv_set(&workspace_key(&input.id), "");

    let mut index = get_workspaces_index();
    index.retain(|entry| entry != &input.id);
    save_workspaces_index(&index);

    PackageResult::ok_empty()
}

fn do_create_session(input: CreateSessionInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    if input.id.trim().is_empty() {
        return PackageResult::err("missing session id");
    }

    let title = decode_transport_text(&input.title, &input.title_b64);

    let mut session = load_session(&input.id).unwrap_or_else(|| SessionRecord {
        id: input.id.clone(),
        title: title.clone(),
        workspace_id: input.workspace_id.clone(),
        workspace_root: input.workspace_root.clone(),
        persistent: input.persistent,
        created_at: input.created_at,
        updated_at: input.updated_at,
        agent_name: session_agent_name(&input.id),
        agent: default_session_agent_config(&input.id),
    });

    session.title = title;
    session.workspace_id = input.workspace_id;
    session.workspace_root = input.workspace_root;
    session.persistent = input.persistent;
    session.created_at = if session.created_at == 0 {
        input.created_at
    } else {
        session.created_at.min(input.created_at)
    };
    session.updated_at = session.updated_at.max(input.updated_at);
    session.agent_name = session_agent_name(&session.id);
    session.agent.name = session.agent_name.clone();

    if let Some(agent) = input.agent {
        merge_session_agent_config(&mut session.agent, &agent);
    }

    save_session(&session);

    let mut index = get_sessions_index();
    if !index.iter().any(|entry| entry == &session.id) {
        index.push(session.id.clone());
        save_sessions_index(&index);
    }

    sync_session_agent(&session);

    PackageResult::ok(serde_json::json!({
        "id": session.id,
        "title": session.title,
        "workspace_id": session.workspace_id,
        "workspace_root": session.workspace_root,
        "persistent": session.persistent,
        "created_at": session.created_at,
        "updated_at": session.updated_at,
        "agent_name": session.agent_name,
    }))
}

fn do_update_session_title(input: SessionTitleInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let mut session = match load_session(&input.id) {
        Some(session) => session,
        None => return PackageResult::err(format!("session '{}' not found", input.id)),
    };
    session.title = decode_transport_text(&input.title, &input.title_b64);
    session.updated_at = session.updated_at.max(input.updated_at);
    save_session(&session);
    PackageResult::ok_empty()
}

fn do_update_session_persistent(input: SessionPersistentInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let mut session = match load_session(&input.id) {
        Some(session) => session,
        None => return PackageResult::err(format!("session '{}' not found", input.id)),
    };
    session.persistent = input.persistent;
    session.updated_at = session.updated_at.max(input.updated_at);
    save_session(&session);
    PackageResult::ok_empty()
}

fn do_delete_session(input: SessionIdInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let session = match load_session(&input.id) {
        Some(session) => session,
        None => return PackageResult::ok_empty(),
    };

    kv_set(&session_key(&input.id), "");
    kv_set(&session_messages_key(&input.id), "");
    // 清掉折叠元数据与前缀指纹,避免长任务多次 fold 后 KV 累积孤儿条目。
    kv_set(&fold_meta_key(&input.id), "");
    kv_set(&session_prefix_fp_key(&input.id), "");

    let mut index = get_sessions_index();
    index.retain(|entry| entry != &input.id);
    save_sessions_index(&index);

    let _ = do_delete_agent(&session.agent_name);
    PackageResult::ok_empty()
}

fn do_touch_session(input: SessionIdInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let mut session = match load_session(&input.id) {
        Some(session) => session,
        None => return PackageResult::err(format!("session '{}' not found", input.id)),
    };
    session.updated_at = session.updated_at.max(input.updated_at);
    save_session(&session);
    PackageResult::ok_empty()
}

fn do_get_session_messages(input: SessionMessagesInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let mut messages = get_session_messages(&input.session_id);
    messages.sort_by_key(|message| message.timestamp);

    PackageResult::ok(serde_json::json!({
        "messages": messages.into_iter().map(|message| serde_json::json!({
            "id": message.id,
            "session_id": message.session_id,
            "role": message.role,
            "content": message.content,
            "tool_name": message.tool_name,
            "tool_args": message.tool_args,
            "tool_status": message.tool_status,
            "streaming": message.streaming,
            "timestamp": message.timestamp,
        })).collect::<Vec<Value>>()
    }))
}

fn do_get_session_context(input: SessionContextInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    PackageResult::ok(serde_json::json!({
        "messages": build_session_context(&input.session_id, input.limit)
    }))
}

fn do_get_session_skills(input: SessionMessagesInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let session = match load_session(&input.session_id) {
        Some(session) => session,
        None => return PackageResult::err(format!("session '{}' not found", input.session_id)),
    };

    PackageResult::ok(serde_json::json!({
        "skills": session.agent.skills,
    }))
}

fn do_set_session_skills(input: SessionSkillsInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let mut session = match load_session(&input.session_id) {
        Some(session) => session,
        None => return PackageResult::err(format!("session '{}' not found", input.session_id)),
    };

    session.agent.skills = input
        .skills
        .into_iter()
        .map(|skill| skill.trim().to_string())
        .filter(|skill| !skill.is_empty())
        .collect::<Vec<String>>();
    session.updated_at = session.updated_at.max(input.updated_at);
    save_session(&session);
    sync_session_agent(&session);

    PackageResult::ok(serde_json::json!({
        "skills": session.agent.skills,
    }))
}

fn do_update_session_message(input: UpdateSessionMessageInput) -> PackageResult {
    ensure_session_records_from_legacy_agents();

    let mut session = match load_session(&input.session_id) {
        Some(session) => session,
        None => return PackageResult::err(format!("session '{}' not found", input.session_id)),
    };

    let mut messages = get_session_messages(&input.session_id);
    if let Some(message) = messages.iter_mut().find(|message| message.id == input.id) {
        let content = decode_transport_text(&input.content, &input.content_b64);
        message.content = content;
        message.streaming = input.streaming;
        if !input.tool_status.trim().is_empty() {
            message.tool_status = Some(input.tool_status);
        }
        session.updated_at = session.updated_at.max(message.timestamp);
        save_session_messages(&input.session_id, &messages);
        save_session(&session);
        return PackageResult::ok_empty();
    }

    PackageResult::err(format!("message '{}' not found", input.id))
}

fn do_send_session_message(input: SendSessionMessageInput) -> PackageResult {
    log_info(&format!(
        "agent-core send_session_message start session={}",
        input.session_id
    ));
    ensure_session_records_from_legacy_agents();
    let input_content = decode_transport_text(&input.content, &input.content_b64);

    // Store selected_tools in session KV for the turn builder to read.
    if !input.selected_tools.is_empty() {
        let tools_json = serde_json::to_string(&input.selected_tools)
            .unwrap_or_else(|_| "[]".into());
        kv_set(&format!("selected_tools:{}", input.session_id), &tools_json);
    } else {
        let _ = kv_delete(&format!("selected_tools:{}", input.session_id));
    }

    let mut session = match load_session(&input.session_id) {
        Some(session) => session,
        None => return PackageResult::err(format!("session '{}' not found", input.session_id)),
    };

    if let Some(agent_input) = input.agent {
        merge_session_agent_config(&mut session.agent, &agent_input);
        save_session(&session);
        sync_session_agent(&session);
    } else if load_agent(&session.agent_name).is_none() {
        sync_session_agent(&session);
    }

    // Reasonix Pillar 1 — try to fold the head before building context so the prefix
    // bytes stay stable across the next turn. No-op when under threshold.
    fold_session_if_needed(&input.session_id, &session.agent);

    let mut history = build_session_context(&input.session_id, 64);
    let should_append_user = history
        .last()
        .map(|message| {
            message
                .get("role")
                .and_then(|role| role.as_str())
                .unwrap_or("")
                != "user"
                || message
                    .get("content")
                    .and_then(|content| content.as_str())
                    .unwrap_or("")
                    != input_content
        })
        .unwrap_or(true);
    if should_append_user {
        history.push(serde_json::json!({"role": "user", "content": input_content}));
    }

    let mut messages = get_session_messages(&input.session_id);
    messages.sort_by_key(|message| message.timestamp);

    let user_index = match find_reusable_user_message(&messages, &input_content) {
        Some(index) => index,
        None => {
            let user_timestamp = now_ms();
            messages.push(SessionMessageRecord {
                id: build_session_message_id(&input.session_id, "user", user_timestamp),
                session_id: input.session_id.clone(),
                role: "user".into(),
                content: input_content.clone(),
                tool_name: None,
                tool_args: None,
                tool_status: None,
                streaming: false,
                timestamp: user_timestamp,
            });
            messages.sort_by_key(|message| message.timestamp);
            let index = messages.len().saturating_sub(1);
            session.updated_at = session.updated_at.max(user_timestamp);
            save_session_messages(&input.session_id, &messages);
            save_session(&session);
            index
        }
    };

    let assistant_placeholder_index = find_reusable_assistant_placeholder(&messages, user_index);

    let (final_reply, endpoint, memory_package, channels_package) = match run_agent_completion(
        &input.session_id,
        &session.agent,
        &input_content,
        &history,
        input.delegate_request.as_ref(),
    ) {
        Ok(result) => result,
        Err(error) => return PackageResult::err(error),
    };
    let visible_reply = visible_assistant_reply(&final_reply);
    append_debug_log(&format!(
        "do_send_session_message completion_ready session={} visible_len={}",
        input.session_id,
        visible_reply.len()
    ));
    log_info(&format!(
        "agent-core send_session_message completion ready session={} reply_len={}",
        input.session_id,
        final_reply.len()
    ));

    let mut next_history = history;
    next_history.push(serde_json::json!({"role": "assistant", "content": visible_reply}));
    // Note: do not truncate next_history here — fold_session_if_needed handles
    // long-session compaction by replacing the head with a single summary message,
    // which preserves byte-stable prefix across turns. A naive tail slice would
    // shift every subsequent prefix and destroy upstream prompt-cache hits.
    append_debug_log(&format!(
        "do_send_session_message save_history_begin session={}",
        input.session_id
    ));
    save_history(&session.agent_name, &next_history);
    append_debug_log(&format!(
        "do_send_session_message save_history_done session={}",
        input.session_id
    ));

    let assistant_timestamp = now_ms();
    if let Some(index) = assistant_placeholder_index {
        if let Some(message) = messages.get_mut(index) {
            message.content = visible_reply.clone();
            message.streaming = false;
            message.timestamp = message.timestamp.max(assistant_timestamp);
        }
    } else {
        messages.push(SessionMessageRecord {
            id: build_session_message_id(&input.session_id, "assistant", assistant_timestamp),
            session_id: input.session_id.clone(),
            role: "assistant".into(),
            content: visible_reply.clone(),
            tool_name: None,
            tool_args: None,
            tool_status: None,
            streaming: false,
            timestamp: assistant_timestamp,
        });
    }
    messages.sort_by_key(|message| message.timestamp);
    append_debug_log(&format!(
        "do_send_session_message save_messages_begin session={}",
        input.session_id
    ));
    log_info(&format!(
        "agent-core send_session_message save messages begin session={}",
        input.session_id
    ));
    save_session_messages(&input.session_id, &messages);
    session.updated_at = session.updated_at.max(assistant_timestamp);
    save_session(&session);
    append_debug_log(&format!(
        "do_send_session_message save_messages_done session={}",
        input.session_id
    ));
    log_info(&format!(
        "agent-core send_session_message save messages done session={}",
        input.session_id
    ));

    log_info(&format!(
        "agent-core send_session_message return session={}",
        input.session_id
    ));
    append_debug_log(&format!(
        "do_send_session_message return session={}",
        input.session_id
    ));
    PackageResult::ok(serde_json::json!({
        "reply": final_reply,
        "model": session.agent.model,
        "provider": session.agent.provider,
        "completion_endpoint": endpoint,
        "memory_package": memory_package,
        "channels_package": channels_package,
        "agent_name": session.agent_name,
    }))
}

fn do_get_debug_trace() -> PackageResult {
    PackageResult::ok(serde_json::json!({
        "last": kv_get("agent-core:debug:last").unwrap_or_default(),
        "trace": kv_get("agent-core:debug:trace").unwrap_or_default(),
    }))
}

fn do_team_delegate_route(input: TeamDelegateRouteInput) -> PackageResult {
    let payload = serde_json::json!({
        "session_id": input.session_id,
        "board_id": input.board_id,
        "workflow_id": input.workflow_id,
        "from_role_id": input.from_role_id,
        "to_role_id": input.to_role_id,
        "task_id": input.task_id,
        "prompt": input.prompt,
        "reason": input.reason,
        "must_act": input.must_act,
        "execute": input.execute,
        "context_refs": input.context_refs,
        "metadata": input.metadata,
    });
    match call_package_ws_action(TEAM_RUNTIME_PLUGIN, "get_delegate_contract", &payload) {
        Ok(response) => match serde_json::from_str::<Value>(&response) {
            Ok(result) => PackageResult::ok(result),
            Err(_) => PackageResult::err("invalid delegate contract response from team-runtime"),
        },
        Err(error) => PackageResult::err(error),
    }
}

fn describe_result() -> PackageResult {
    PackageResult::ok(serde_json::json!({
        "package": "agent-runtime",
        "runtime": "wasm",
        "capabilities": [AGENT_RUNTIME_CAPABILITY, TEAM_DELEGATE_CAPABILITY],
        "actions": {
            AGENT_RUNTIME_CAPABILITY: [
                "describe",
                "health",
                "get_agents",
                "create_agent",
                "delete_agent",
                "send_message",
                "get_history",
                "clear_history",
                "list_sessions",
                "list_workspaces",
                "create_workspace",
                "delete_workspace",
                "create_session",
                "update_session_title",
                "update_session_persistent",
                "delete_session",
                "touch_session",
                "get_session_messages",
                "save_session_message",
                "update_session_message",
                "get_session_context",
                "get_session_skills",
                "set_session_skills",
                "send_session_message",
                "get_debug_trace"
            ],
            TEAM_DELEGATE_CAPABILITY: ["describe", "health", "route_delegate", "get_delegate_contract"],
        },
    }))
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("agent-core package initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    write_file("./data/agent-core-last-ws.txt", "stage=ws_received");
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });
    write_file(
        "./data/agent-core-last-ws.txt",
        &format!("stage=ws_parsed action={}", req.action),
    );

    let result = match req.action.as_str() {
        "describe" => describe_result(),
        "health" => PackageResult::ok(serde_json::json!({
            "healthy": true,
            "package": "agent-runtime",
            "capabilities": [AGENT_RUNTIME_CAPABILITY, TEAM_DELEGATE_CAPABILITY],
        })),
        "get_agents" => do_get_agents(),
        "create_agent" => {
            let config: AgentConfig =
                serde_json::from_value(req.data).unwrap_or_else(|_| default_agent_config());
            do_create_agent(&config)
        }
        "delete_agent" => {
            let name = req.data["name"].as_str().unwrap_or("");
            do_delete_agent(name)
        }
        "send_message" => {
            #[derive(Deserialize)]
            struct SendMessageActionInput {
                agent: String,
                #[serde(default)]
                content: String,
                #[serde(default)]
                content_b64: String,
                #[serde(default)]
                delegate_request: Option<DelegateRequestInput>,
                /// Per-message model override (shorthand — merged into delegate_request).
                #[serde(default)]
                model: Option<String>,
                /// Per-message provider override (shorthand — merged into delegate_request).
                #[serde(default)]
                provider: Option<String>,
                /// Privacy mode: skip memory storage for this turn.
                #[serde(default)]
                private: bool,
                /// Tool selection: only these tools (by name) will be injected for this turn.
                /// Empty = inject all (backward compatible). Non-empty = only these + always-on.
                #[serde(default)]
                selected_tools: Vec<String>,
            }

            match serde_json::from_value::<SendMessageActionInput>(req.data) {
                Ok(input) => {
                    let content = decode_transport_text(&input.content, &input.content_b64);
                    let sid = session_id_from_agent_name(&input.agent)
                        .unwrap_or_default();
                    // Set privacy flag in session KV so memory hooks can check it.
                    if input.private {
                        kv_set(&format!("private:{}", sid), "1");
                    }
                    // Store selected_tools in session KV for the turn builder to read.
                    if !input.selected_tools.is_empty() {
                        let tools_json = serde_json::to_string(&input.selected_tools)
                            .unwrap_or_else(|_| "[]".into());
                        kv_set(&format!("selected_tools:{}", sid), &tools_json);
                    } else {
                        let _ = kv_delete(&format!("selected_tools:{}", sid));
                    }
                    // Merge top-level model/provider into delegate_request if present.
                    let delegate = match input.delegate_request {
                        Some(mut dr) => {
                            if dr.model_override.is_none() {
                                dr.model_override = input.model;
                            }
                            if dr.provider_override.is_none() {
                                dr.provider_override = input.provider;
                            }
                            Some(dr)
                        }
                        None if input.model.is_some() || input.provider.is_some() => {
                            Some(DelegateRequestInput {
                                model_override: input.model,
                                provider_override: input.provider,
                                ..Default::default()
                            })
                        }
                        None => None,
                    };
                    do_send_message_with_delegate(
                        &input.agent,
                        &content,
                        delegate.as_ref(),
                    )
                }
                Err(_) => PackageResult::err("invalid send_message payload"),
            }
        }
        "get_history" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let history = get_history(agent);
            PackageResult::ok(serde_json::json!({"messages": history}))
        }
        "clear_history" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            save_history(agent, &[]);
            PackageResult::ok_empty()
        }
        "list_sessions" => do_list_sessions(),
        "list_workspaces" => do_list_workspaces(),
        "create_workspace" => do_create_workspace(serde_json::from_value(req.data).unwrap_or(
            CreateWorkspaceInput {
                id: String::new(),
                name: String::new(),
                name_b64: String::new(),
                root_path: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        )),
        "delete_workspace" => do_delete_workspace(
            serde_json::from_value(req.data).unwrap_or(WorkspaceIdInput { id: String::new() }),
        ),
        "create_session" => do_create_session(serde_json::from_value(req.data).unwrap_or(
            CreateSessionInput {
                id: String::new(),
                title: String::new(),
                title_b64: String::new(),
                workspace_id: String::new(),
                workspace_root: String::new(),
                persistent: 0,
                created_at: 0,
                updated_at: 0,
                agent: None,
            },
        )),
        "update_session_title" => do_update_session_title(
            serde_json::from_value(req.data).unwrap_or(SessionTitleInput {
                id: String::new(),
                title: String::new(),
                title_b64: String::new(),
                updated_at: 0,
            }),
        ),
        "update_session_persistent" => do_update_session_persistent(
            serde_json::from_value(req.data).unwrap_or(SessionPersistentInput {
                id: String::new(),
                persistent: 0,
                updated_at: 0,
            }),
        ),
        "delete_session" => {
            do_delete_session(serde_json::from_value(req.data).unwrap_or(SessionIdInput {
                id: String::new(),
                updated_at: 0,
            }))
        }
        "touch_session" => {
            do_touch_session(serde_json::from_value(req.data).unwrap_or(SessionIdInput {
                id: String::new(),
                updated_at: 0,
            }))
        }
        "get_session_messages" => do_get_session_messages(
            serde_json::from_value(req.data).unwrap_or(SessionMessagesInput {
                session_id: String::new(),
            }),
        ),
        "save_session_message" => upsert_session_message_record(
            serde_json::from_value(req.data).unwrap_or(SaveSessionMessageInput {
                id: String::new(),
                session_id: String::new(),
                role: String::new(),
                content: String::new(),
                content_b64: String::new(),
                tool_name: String::new(),
                tool_args: String::new(),
                tool_status: String::new(),
                streaming: false,
                timestamp: 0,
            }),
        ),
        "update_session_message" => do_update_session_message(
            serde_json::from_value(req.data).unwrap_or(UpdateSessionMessageInput {
                id: String::new(),
                session_id: String::new(),
                content: String::new(),
                content_b64: String::new(),
                streaming: false,
                tool_status: String::new(),
            }),
        ),
        "get_session_context" => do_get_session_context(
            serde_json::from_value(req.data).unwrap_or(SessionContextInput {
                session_id: String::new(),
                limit: default_context_limit(),
            }),
        ),
        "get_session_skills" => do_get_session_skills(serde_json::from_value(req.data).unwrap_or(
            SessionMessagesInput {
                session_id: String::new(),
            },
        )),
        "set_session_skills" => do_set_session_skills(serde_json::from_value(req.data).unwrap_or(
            SessionSkillsInput {
                session_id: String::new(),
                skills: Vec::new(),
                updated_at: 0,
            },
        )),
        "send_session_message" => do_send_session_message(
            serde_json::from_value(req.data).unwrap_or(SendSessionMessageInput {
                session_id: String::new(),
                content: String::new(),
                content_b64: String::new(),
                agent: None,
                delegate_request: None,
                selected_tools: Vec::new(),
            }),
        ),
        "route_delegate" | "get_delegate_contract" => {
            let payload: TeamDelegateRouteInput =
                serde_json::from_value(req.data).unwrap_or_default();
            do_team_delegate_route(payload)
        }
        "get_debug_trace" => do_get_debug_trace(),
        "undo_round" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let rounds = req.data["rounds"].as_u64().unwrap_or(1) as usize;
            do_undo_round(agent, rounds)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };
    write_file("./data/agent-core-last-ws.txt", "stage=ws_return");

    Ok(result.to_json())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_provider(name: &str, format: &str, base_url: &str) -> RuntimeProviderConfig {
        RuntimeProviderConfig {
            name: name.to_string(),
            format: format.to_string(),
            base_url: base_url.to_string(),
            keys: Vec::new(),
            response_format_json_schema: None,
        }
    }

    #[test]
    fn extract_evolved_skill_ids_supports_scored_and_legacy_shapes() {
        let data = serde_json::json!({
            "skills": [
                { "score": 0.91, "skill": { "id": " scored-skill " } },
                { "id": "legacy-skill" },
                { "score": 0.2, "skill": { "id": "" } },
                { "score": 0.1, "skill": {} },
                { "id": "   " }
            ]
        });

        assert_eq!(
            extract_evolved_skill_ids(&data),
            vec!["scored-skill".to_string(), "legacy-skill".to_string()]
        );
    }

    #[test]
    fn build_trajectory_promotion_payload_includes_success_verification_and_risk() {
        let trajectory = AgentTrajectory {
            id: "trajectory-1".to_string(),
            agent: "agent-a".to_string(),
            task: " do work ".to_string(),
            started_at: 1,
            completed_at: 2,
            status: "success".to_string(),
            steps: vec!["plan".to_string(), "execute".to_string()],
            tool_names: vec!["fs_read".to_string()],
            final_result: "done".to_string(),
            failure: String::new(),
            injected_skill_ids: vec![],
            promotion_result: Value::Null,
            ..AgentTrajectory::default()
        };

        let payload = build_trajectory_promotion_payload(&trajectory);

        assert_eq!(payload["agent"], serde_json::json!("agent-a"));
        assert_eq!(payload["trajectory_id"], serde_json::json!("trajectory-1"));
        assert_eq!(
            payload["summary"],
            serde_json::json!("Successful run for do work")
        );
        assert_eq!(payload["steps"], serde_json::json!(["plan", "execute"]));
        assert_eq!(payload["tools"], serde_json::json!(["fs_read"]));
        assert_eq!(payload["final_result"], serde_json::json!("done"));
        assert_eq!(payload["success"], serde_json::json!(true));
        assert_eq!(payload["verification_passed"], serde_json::json!(true));
        assert_eq!(
            payload["verification_evidence"],
            serde_json::json!(
                "Trajectory completed successfully with final result recorded; steps=2, tools=1."
            )
        );
        assert_eq!(payload["auto_activate"], serde_json::json!(false));
        assert_eq!(payload["risk_level"], serde_json::json!("medium"));
    }

    #[test]
    fn agent_trajectory_deserializes_old_json_without_promotion_fields() {
        let json = r#"{
            "id": "legacy-trajectory",
            "agent": "agent-a",
            "task": "legacy task",
            "started_at": 1,
            "completed_at": 2,
            "status": "success",
            "steps": ["step-a"],
            "tool_names": [],
            "final_result": "done",
            "failure": ""
        }"#;

        let trajectory: AgentTrajectory =
            serde_json::from_str(json).expect("legacy trajectory json should deserialize");

        assert_eq!(trajectory.id, "legacy-trajectory");
        assert_eq!(trajectory.injected_skill_ids, Vec::<String>::new());
        assert_eq!(trajectory.promotion_result, Value::Null);
        assert_eq!(
            build_trajectory_promotion_payload(&trajectory)["risk_level"],
            serde_json::json!("low")
        );
    }
    #[test]
    fn mark_agent_trajectory_success_records_completion_fields_without_host_kv() {
        let mut trajectory = AgentTrajectory {
            id: "trajectory-success".to_string(),
            agent: "agent-a".to_string(),
            task: "do work".to_string(),
            started_at: 7,
            completed_at: 0,
            status: "running".to_string(),
            steps: vec!["planned".to_string()],
            tool_names: vec!["web_search".to_string()],
            final_result: String::new(),
            failure: "previous failure".to_string(),
            injected_skill_ids: vec!["skill-a".to_string()],
            promotion_result: Value::Null,
            ..AgentTrajectory::default()
        };

        mark_agent_trajectory_success(&mut trajectory, "done");

        assert_eq!(trajectory.status, "success");
        assert_eq!(trajectory.final_result, "done");
        assert_eq!(trajectory.failure, "");
        assert!(trajectory.completed_at >= trajectory.started_at);
        assert_eq!(trajectory.steps, vec!["planned".to_string()]);
        assert_eq!(trajectory.tool_names, vec!["web_search".to_string()]);
        assert_eq!(trajectory.injected_skill_ids, vec!["skill-a".to_string()]);
    }

    #[test]
    fn mark_agent_trajectory_failure_records_failure_fields_without_host_kv() {
        let mut trajectory = AgentTrajectory {
            id: "trajectory-failure".to_string(),
            agent: "agent-a".to_string(),
            task: "do work".to_string(),
            started_at: 11,
            completed_at: 0,
            status: "running".to_string(),
            steps: vec!["planned".to_string()],
            tool_names: vec!["fs_read".to_string()],
            final_result: String::new(),
            failure: String::new(),
            injected_skill_ids: vec!["skill-a".to_string()],
            promotion_result: Value::Null,
            ..AgentTrajectory::default()
        };

        mark_agent_trajectory_failure(&mut trajectory, "completion error");

        assert_eq!(trajectory.status, "failure");
        assert_eq!(trajectory.failure, "completion error");
        assert_eq!(trajectory.final_result, "");
        assert!(trajectory.completed_at >= trajectory.started_at);
        assert_eq!(trajectory.steps, vec!["planned".to_string()]);
        assert_eq!(trajectory.tool_names, vec!["fs_read".to_string()]);
        assert_eq!(trajectory.injected_skill_ids, vec!["skill-a".to_string()]);
    }

    #[test]
    fn resolve_completion_endpoint_keeps_resolved_config_endpoint() {
        assert_eq!(
            resolve_completion_endpoint(" https://example.test/v1/chat/completions "),
            "https://example.test/v1/chat/completions".to_string()
        );
        assert_eq!(
            resolve_completion_endpoint("host://weft-core/completions"),
            default_completion_endpoint()
        );
        assert_eq!(
            resolve_completion_endpoint(""),
            default_completion_endpoint()
        );
    }

    #[test]
    fn extract_response_text_reads_chat_completion_message_content() {
        let response_value = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "hello from chat completions"
                    }
                }
            ]
        });

        assert_eq!(
            extract_response_text(&response_value),
            "hello from chat completions"
        );
    }

    #[test]
    fn delegate_request_requires_real_action_when_must_act_true() {
        let request = DelegateRequestInput {
            must_act: true,
            reason: "needs real action".to_string(),
            latest_user_query: "上网搜索帮我".to_string(),
            visible_history: vec![
                serde_json::json!({"role": "user", "content": "你能去搜索 uzi 的含义吗"}),
            ],
            session_context: vec![],
            runtime_context: Value::Null,
            skill_refs: vec![],
            action_refs: vec![],
            event: Value::Null,
            ..Default::default()
        };

        assert!(delegate_request_requires_real_action(Some(&request)));
    }

    #[test]
    fn build_mode_enum_forces_tool_when_delegate_requires_action() {
        let request = DelegateRequestInput {
            must_act: true,
            reason: "needs real action".to_string(),
            latest_user_query: "上网搜索帮我".to_string(),
            visible_history: vec![
                serde_json::json!({"role": "user", "content": "你能去搜索 uzi 的含义吗"}),
            ],
            session_context: vec![],
            runtime_context: Value::Null,
            skill_refs: vec![],
            action_refs: vec![],
            event: Value::Null,
            ..Default::default()
        };

        assert_eq!(
            build_mode_enum(&vec!["web_search".to_string()], Some(&request)),
            serde_json::json!(["tool"])
        );
    }

    #[test]
    fn build_tool_call_min_items_requires_real_tool_call_for_delegate() {
        let request = DelegateRequestInput {
            must_act: true,
            reason: "needs real action".to_string(),
            latest_user_query: "上网搜索帮我".to_string(),
            visible_history: vec![
                serde_json::json!({"role": "user", "content": "你能去搜索 uzi 的含义吗"}),
            ],
            session_context: vec![],
            runtime_context: Value::Null,
            skill_refs: vec![],
            action_refs: vec![],
            event: Value::Null,
            ..Default::default()
        };

        assert_eq!(build_tool_call_min_items(Some(&request)), 1);
    }

    #[test]
    fn build_agent_turn_response_format_forces_tool_mode_for_delegate() {
        let request = DelegateRequestInput {
            must_act: true,
            reason: "needs real action".to_string(),
            latest_user_query: "search it for me".to_string(),
            visible_history: vec![],
            session_context: vec![],
            runtime_context: Value::Null,
            skill_refs: vec![],
            action_refs: vec![],
            event: Value::Null,
            ..Default::default()
        };

        let schema =
            build_agent_turn_response_format(&vec!["web_search".to_string()], Some(&request));
        assert_eq!(
            schema["json_schema"]["schema"]["properties"]["mode"]["enum"],
            serde_json::json!(["tool"])
        );
        assert_eq!(
            schema["json_schema"]["schema"]["properties"]["tool_calls"]["minItems"],
            serde_json::json!(1)
        );
        assert_eq!(
            schema["json_schema"]["schema"]["properties"]["tool_calls"]["items"]["properties"]
                ["name"]["enum"],
            serde_json::json!(["web_search"])
        );
    }

    #[test]
    fn build_agent_turn_response_format_allows_reply_without_tools() {
        let schema = build_agent_turn_response_format(&Vec::new(), None);
        assert_eq!(
            schema["json_schema"]["schema"]["properties"]["mode"]["enum"],
            serde_json::json!(["reply"])
        );
        assert_eq!(
            schema["json_schema"]["schema"]["properties"]["tool_calls"]["minItems"],
            serde_json::json!(0)
        );
    }

    #[test]
    fn structured_output_strategy_defaults_to_native_json_schema_for_openai_family() {
        let provider = runtime_provider("openai", "openai", "https://api.openai.com/v1");

        assert_eq!(
            structured_output_strategy_for_provider(&provider, ""),
            StructuredOutputStrategy::NativeJsonSchema
        );
    }

    #[test]
    fn structured_output_strategy_falls_back_for_deepseek_family() {
        let provider = runtime_provider("deepseek", "openai", "https://api.deepseek.com");

        assert_eq!(
            structured_output_strategy_for_provider(&provider, ""),
            StructuredOutputStrategy::PromptValidatedJson
        );
    }

    #[test]
    fn generic_chat_completion_openai_models_use_prompt_validated_json() {
        let provider = runtime_provider(
            "ppchat-gpt-5-4",
            "generic-chat-completion-api",
            "https://example.com/v1",
        );
        assert_eq!(
            structured_output_strategy_for_provider(&provider, "gpt-5.4"),
            StructuredOutputStrategy::PromptValidatedJson
        );
        assert_eq!(
            resolve_request_structured_output_strategy(
                "ppchat-gpt-5-4",
                "gpt-5.4",
                "https://example.com/v1"
            ),
            StructuredOutputStrategy::PromptValidatedJson
        );
    }

    #[test]
    fn maybe_attach_schema_response_format_omits_unsupported_strategy() {
        let mut body = serde_json::json!({ "model": "deepseek-chat" });

        maybe_attach_schema_response_format(
            &mut body,
            StructuredOutputStrategy::PromptValidatedJson,
            "agent_turn_plan",
            serde_json::json!({"type": "object"}),
        );

        assert!(body.get("response_format").is_none());
    }

    #[test]
    fn maybe_attach_schema_response_format_keeps_native_json_schema_strategy() {
        let mut body = serde_json::json!({ "model": "gpt-4o-mini" });

        maybe_attach_schema_response_format(
            &mut body,
            StructuredOutputStrategy::NativeJsonSchema,
            "agent_turn_plan",
            serde_json::json!({"type": "object"}),
        );

        assert_eq!(
            body["response_format"]["type"],
            serde_json::json!("json_schema")
        );
        assert_eq!(
            body["response_format"]["json_schema"]["name"],
            serde_json::json!("agent_turn_plan")
        );
    }

    #[test]
    fn structured_output_strategy_family_hint_detects_deepseek_endpoints() {
        assert_eq!(
            structured_output_strategy_for_family_hint(
                "https://api.deepseek.com/v1/chat/completions"
            ),
            Some(StructuredOutputStrategy::PromptValidatedJson)
        );
    }

    #[test]
    fn resolve_request_structured_output_strategy_prefers_endpoint_when_provider_missing() {
        assert_eq!(
            resolve_request_structured_output_strategy(
                "",
                "",
                "https://api.deepseek.com/v1/chat/completions"
            ),
            StructuredOutputStrategy::PromptValidatedJson
        );
    }

    #[test]
    fn responses_request_only_attaches_metadata_for_openai_targets() {
        let original_body = serde_json::json!({
            "model": "gpt-5-mini",
            "messages": [
                {"role": "system", "content": "sys"},
                {"role": "user", "content": "hello"}
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "agent_turn_plan",
                    "strict": true,
                    "schema": {"type": "object"}
                }
            },
            "x_provider": "deepseek"
        });

        let original_body_text = serde_json::to_string(&original_body).unwrap();
        let target_url = "https://api.deepseek.com/v1/responses";
        let provider_value = original_body.get("x_provider").cloned().unwrap();
        let provider_hint_text = provider_value.as_str().unwrap_or("");
        let supports_metadata = responses_request_supports_metadata(target_url, provider_hint_text);
        let input_messages = original_body.get("messages").cloned().unwrap();
        let model = original_body.get("model").cloned().unwrap();
        let temperature = original_body.get("temperature").cloned();
        let response_format = original_body.get("response_format").cloned();
        let mut response_body = serde_json::json!({
            "model": model,
            "input": input_messages,
        });
        if let Some(temp) = temperature {
            response_body["temperature"] = temp;
        }
        if let Some(format) = response_format.and_then(|value| value.get("json_schema").cloned()) {
            response_body["text"] = serde_json::json!({
                "format": {
                    "type": "json_schema",
                    "name": format.get("name").cloned().unwrap_or_else(|| Value::String("agent_turn_plan".to_string())),
                    "schema": format.get("schema").cloned().unwrap_or(Value::Null),
                    "strict": format.get("strict").cloned().unwrap_or(Value::Bool(true)),
                }
            });
        }
        if let Some(provider_value) = original_body.get("x_provider").cloned() {
            let provider_hint_text = provider_value.as_str().unwrap_or("");
            if responses_request_supports_metadata(target_url, provider_hint_text) {
                response_body["metadata"] = serde_json::json!({ "x_provider": provider_value });
            }
        }

        assert_eq!(
            resolve_request_structured_output_strategy("deepseek", "gpt-5-mini", target_url),
            StructuredOutputStrategy::PromptValidatedJson
        );
        assert!(target_url.ends_with("/responses"));
        assert!(
            !supports_metadata,
            "deepseek responses requests should not attach metadata: {}",
            original_body_text
        );
        assert!(
            response_body.get("metadata").is_none(),
            "deepseek responses payload unexpectedly included metadata: {}",
            serde_json::to_string(&response_body).unwrap()
        );
    }

    #[test]
    fn build_outbound_request_debug_dump_keeps_metadata_and_body() {
        let dump = build_outbound_request_debug_dump(
            "agent_turn",
            "http://127.0.0.1:42617/v1/chat/completions",
            "http://127.0.0.1:42617/v1/chat/completions",
            "https://api.deepseek.com/chat/completions",
            "deepseek",
            r#"{"model":"deepseek-chat","messages":[],"temperature":0}"#,
        );

        let parsed: Value = serde_json::from_str(&dump).expect("debug dump should be json");
        assert_eq!(parsed["request_label"], serde_json::json!("agent_turn"));
        assert_eq!(parsed["provider_hint"], serde_json::json!("deepseek"));
        assert_eq!(
            parsed["target_url"],
            serde_json::json!("https://api.deepseek.com/chat/completions")
        );
        assert_eq!(parsed["body"]["model"], serde_json::json!("deepseek-chat"));
    }

    #[test]
    fn build_agent_turn_plan_schema_matches_existing_response_format_schema() {
        let tool_names = vec!["web_search".to_string()];
        let request = DelegateRequestInput {
            must_act: true,
            reason: "needs real action".to_string(),
            latest_user_query: "search it for me".to_string(),
            visible_history: vec![],
            session_context: vec![],
            runtime_context: Value::Null,
            skill_refs: vec![],
            action_refs: vec![],
            event: Value::Null,
            ..Default::default()
        };

        assert_eq!(
            build_agent_turn_plan_schema(&tool_names, Some(&request)),
            build_agent_turn_response_format(&tool_names, Some(&request))["json_schema"]["schema"]
        );
    }

    #[test]
    fn build_delegate_contract_block_embeds_delegate_context() {
        let request = DelegateRequestInput {
            must_act: true,
            reason: "needs real action".to_string(),
            latest_user_query: "上网搜索帮我".to_string(),
            visible_history: vec![
                serde_json::json!({"role": "user", "content": "你能去搜索 uzi 的含义吗"}),
            ],
            session_context: vec![],
            runtime_context: serde_json::json!({
                "routeDecision": { "route": "web_search" },
                "toolPolicy": { "allow": ["web"] }
            }),
            skill_refs: vec!["fs_read".to_string()],
            action_refs: vec!["read_skill".to_string()],
            event: serde_json::json!({"type": "tool_lifecycle"}),
            ..Default::default()
        };

        let block = build_delegate_contract_block(Some(&request))
            .expect("delegate contract block should exist");

        assert!(block.contains("already delegated by the companion layer"));
        assert!(block.contains("上网搜索帮我"));
        assert!(block.contains("你能去搜索 uzi 的含义吗"));
        assert!(block.contains("web_search"));
        assert!(block.contains("fs_read"));
        assert!(block.contains("tool_lifecycle"));
    }

    #[test]
    fn include_auxiliary_planning_context_is_disabled_for_real_action_delegate() {
        let request = DelegateRequestInput {
            must_act: true,
            reason: "needs real action".to_string(),
            latest_user_query: "search it for me".to_string(),
            visible_history: vec![],
            session_context: vec![],
            runtime_context: Value::Null,
            skill_refs: vec![],
            action_refs: vec![],
            event: Value::Null,
            ..Default::default()
        };

        assert!(!include_auxiliary_planning_context(Some(&request)));
        assert!(include_auxiliary_planning_context(None));
    }

    #[test]
    fn build_tool_catalog_prompt_compacts_delegate_tool_summaries() {
        let request = DelegateRequestInput {
            must_act: true,
            reason: "needs real action".to_string(),
            latest_user_query: "search it for me".to_string(),
            visible_history: vec![],
            session_context: vec![],
            runtime_context: Value::Null,
            skill_refs: vec![],
            action_refs: vec![],
            event: Value::Null,
            ..Default::default()
        };
        let tools = vec![serde_json::json!({
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Search the live web for current information.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "limit": { "type": "integer" },
                        "domain": { "type": "string" }
                    },
                    "required": ["query"]
                }
            }
        })];

        let prompt = build_tool_catalog_prompt(&tools, Some(&request));
        assert!(prompt.contains("- web_search: Search the live web for current information."));
        assert!(prompt.contains("required: query"));
        assert!(
            prompt.contains("optional: limit, domain")
                || prompt.contains("optional: domain, limit")
        );
        assert!(!prompt.contains("\"properties\""));
    }

    #[test]
    fn describe_result_exposes_team_delegate_from_agent_runtime() {
        let described = describe_result();
        let value = described
            .data
            .expect("describe result should include payload");

        assert_eq!(value["package"], serde_json::json!("agent-runtime"));
        assert!(value["capabilities"]
            .as_array()
            .expect("capabilities array")
            .iter()
            .any(|entry| entry == "team.delegate"));
        assert_eq!(
            value["actions"]["team.delegate"],
            serde_json::json!([
                "describe",
                "health",
                "route_delegate",
                "get_delegate_contract"
            ])
        );
    }

    #[test]
    fn summarize_web_fetch_output_includes_status_url_and_preview() {
        let tool = ToolExecution {
            name: "web_fetch".to_string(),
            args: serde_json::json!({
                "url": "https://example.com"
            }),
            output: serde_json::json!({
                "status": "ok",
                "data": {
                    "status": 200,
                    "body": "Example Domain\nThis domain is for use in illustrative examples in documents."
                }
            })
            .to_string(),
            is_error: false,
        };

        let summary = summarize_web_fetch_output(&tool).expect("expected summary");

        assert!(summary.contains("https://example.com"));
        assert!(summary.contains("HTTP 200"));
        assert!(summary.contains("Example Domain"));
    }

    #[test]
    fn build_local_tool_answer_includes_web_fetch_summary() {
        let tool = ToolExecution {
            name: "web_fetch".to_string(),
            args: serde_json::json!({
                "url": "https://example.com"
            }),
            output: serde_json::json!({
                "status": "ok",
                "data": {
                    "status": 200,
                    "body": "Example Domain"
                }
            })
            .to_string(),
            is_error: false,
        };

        let answer = build_local_tool_answer(&[tool]);

        assert!(answer.contains("Fetched https://example.com successfully (HTTP 200)."));
        assert!(answer.contains("Example Domain"));
    }

    #[test]
    fn build_completion_request_includes_virtual_capability_tools() {
        let mut config = default_agent_config();
        config.name = "weft-claw-http".to_string();

        let (_, _, _, _, tool_schemas, _) =
            build_completion_request(&config, "请帮我列出 MCP 服务器", &[], None, None, None);

        assert!(tool_schemas.contains_key("prompt_system_render"));
        assert!(tool_schemas.contains_key("workflow_orchestration_plan"));
        assert!(tool_schemas.contains_key("ext_mcp_list_servers"));
        assert!(tool_schemas.contains_key("ext_mcp_call_tool"));
        assert!(tool_schemas.contains_key("channel_bridge_register"));
        assert!(tool_schemas.contains_key("channel_bridge_list"));
        assert!(tool_schemas.contains_key("channel_bridge_send"));
    }

    #[test]
    fn parse_agent_turn_plan_accepts_native_tool_calls_when_content_is_empty() {
        let response_value = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "",
                        "tool_calls": [
                            {
                                "function": {
                                    "name": "web_search",
                                    "arguments": "{\"query\":\"Factory CLI BYOK docs\"}"
                                }
                            }
                        ]
                    }
                }
            ]
        });

        let plan = parse_agent_turn_plan(&response_value).expect("native tool calls should parse");
        assert_eq!(plan.mode, "tool");
        assert_eq!(plan.assistant, "");
        assert_eq!(plan.tool_calls.len(), 1);
        assert_eq!(plan.tool_calls[0].name, "web_search");
        assert_eq!(
            plan.tool_calls[0].arguments_json,
            r#"{"query":"Factory CLI BYOK docs"}"#
        );
    }

    #[test]
    fn parse_agent_turn_plan_accepts_responses_output_message_with_tool_call_segments() {
        let response_value = serde_json::json!({
            "output": [
                {
                    "type": "message",
                    "content": [
                        {
                            "type": "function_call",
                            "name": "web_search",
                            "arguments": "{\"query\":\"Factory CLI BYOK docs\"}"
                        }
                    ]
                }
            ]
        });

        let plan =
            parse_agent_turn_plan(&response_value).expect("responses tool calls should parse");
        assert_eq!(plan.mode, "tool");
        assert_eq!(plan.assistant, "");
        assert_eq!(plan.tool_calls.len(), 1);
        assert_eq!(plan.tool_calls[0].name, "web_search");
        assert_eq!(
            plan.tool_calls[0].arguments_json,
            r#"{"query":"Factory CLI BYOK docs"}"#
        );
    }

    #[test]
    fn parse_agent_turn_plan_accepts_responses_output_text_json_payload() {
        let response_value = serde_json::json!({
            "output": [
                {
                    "type": "message",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "{\"mode\":\"reply\",\"assistant\":\"Done\",\"tool_calls\":[]}"
                        }
                    ]
                }
            ]
        });

        let plan =
            parse_agent_turn_plan(&response_value).expect("responses output text should parse");
        assert_eq!(plan.mode, "reply");
        assert_eq!(plan.assistant, "Done");
        assert!(plan.tool_calls.is_empty());
    }

    #[test]
    fn execute_agent_turn_plan_loop_chains_follow_up_tool_rounds() {
        let mut seen_prompts: Vec<Option<String>> = Vec::new();
        let reply = run_agent_completion_plan_loop_for_test(
            "fetch a page, write summary.md, then read it back",
            |round, follow_up_prompt| {
                seen_prompts.push(follow_up_prompt.map(|value| value.to_string()));
                match round {
                    0 => Ok(AgentTurnPlan {
                        mode: "tool".to_string(),
                        assistant: String::new(),
                        tool_calls: vec![PlannedToolCall {
                            name: "web_fetch".to_string(),
                            arguments_json: r#"{"url":"https://example.com"}"#.to_string(),
                        }],
                    }),
                    1 => {
                        let prompt =
                            follow_up_prompt.expect("second round should receive follow-up prompt");
                        assert!(prompt.contains("Original user request"));
                        assert!(prompt.contains("[Tool: web_fetch"));
                        assert!(prompt.contains("Example Domain"));
                        Ok(AgentTurnPlan {
                            mode: "tool".to_string(),
                            assistant: String::new(),
                            tool_calls: vec![
                                PlannedToolCall {
                                    name: "fs_write".to_string(),
                                    arguments_json:
                                        r#"{"path":"summary.md","content":"Example summary"}"#
                                            .to_string(),
                                },
                                PlannedToolCall {
                                    name: "fs_read".to_string(),
                                    arguments_json: r#"{"path":"summary.md"}"#.to_string(),
                                },
                            ],
                        })
                    }
                    2 => {
                        let prompt =
                            follow_up_prompt.expect("third round should receive follow-up prompt");
                        assert!(prompt.contains("[Tool: fs_write"));
                        assert!(prompt.contains("[Tool: fs_read"));
                        Ok(AgentTurnPlan {
                            mode: "reply".to_string(),
                            assistant: "Summary saved and verified.".to_string(),
                            tool_calls: vec![],
                        })
                    }
                    _ => Err("unexpected extra round".to_string()),
                }
            },
            |_round, tool_call| match tool_call.name.as_str() {
                "web_fetch" => Ok(ToolExecution {
                    name: "web_fetch".to_string(),
                    args: serde_json::from_str(&tool_call.arguments_json)
                        .expect("valid web_fetch args"),
                    output: serde_json::json!({
                        "status": "ok",
                        "data": {
                            "status": 200,
                            "body": "Example Domain\nFetched content for summary"
                        }
                    })
                    .to_string(),
                    is_error: false,
                }),
                "fs_write" => Ok(ToolExecution {
                    name: "fs_write".to_string(),
                    args: serde_json::from_str(&tool_call.arguments_json)
                        .expect("valid fs_write args"),
                    output: serde_json::json!({
                        "status": "ok",
                        "data": { "written": true }
                    })
                    .to_string(),
                    is_error: false,
                }),
                "fs_read" => Ok(ToolExecution {
                    name: "fs_read".to_string(),
                    args: serde_json::from_str(&tool_call.arguments_json)
                        .expect("valid fs_read args"),
                    output: serde_json::json!({
                        "status": "ok",
                        "data": { "content": "Example summary" }
                    })
                    .to_string(),
                    is_error: false,
                }),
                other => Err(format!("unexpected tool {}", other)),
            },
        )
        .expect("loop should succeed");

        assert_eq!(seen_prompts.len(), 3);
        assert!(seen_prompts[0].is_none());
        assert!(seen_prompts[1]
            .as_deref()
            .unwrap_or("")
            .contains("[Tool: web_fetch"));
        assert!(seen_prompts[2]
            .as_deref()
            .unwrap_or("")
            .contains("[Tool: fs_read"));
        assert_eq!(reply, "Summary saved and verified.");
    }

    #[test]
    fn summarize_virtual_prompt_system_output_returns_rendered_prompt() {
        let tool = ToolExecution {
            name: "prompt_system_render".to_string(),
            args: serde_json::json!({"system_prompt": "你是测试助手"}),
            output: serde_json::json!({
                "status": "ok",
                "data": {
                    "package": "prompt-system",
                    "capability": "prompt.system",
                    "system_prompt": "你是测试助手"
                }
            })
            .to_string(),
            is_error: false,
        };

        let summary = summarize_known_tool_output(&tool).expect("expected summary");
        assert!(summary.contains("Rendered prompt.system successfully."));
        assert!(summary.contains("你是测试助手"));
    }

    #[test]
    fn summarize_virtual_workflow_output_returns_workflow_payload() {
        let tool = ToolExecution {
            name: "workflow_orchestration_plan".to_string(),
            args: serde_json::json!({"goal": "发布版本"}),
            output: serde_json::json!({
                "status": "ok",
                "data": {
                    "package": "workflow-orchestrator",
                    "accepted": true,
                    "workflow": {
                        "goal": "发布版本",
                        "steps": ["构建", "测试", "发布"]
                    }
                }
            })
            .to_string(),
            is_error: false,
        };

        let summary = summarize_known_tool_output(&tool).expect("expected summary");
        assert!(summary.contains("workflow.orchestration"));
        assert!(summary.contains("accepted=true"));
        assert!(summary.contains("发布版本"));
    }

    #[test]
    fn summarize_virtual_mcp_output_lists_registered_servers() {
        let tool = ToolExecution {
            name: "ext_mcp_list_servers".to_string(),
            args: serde_json::json!({}),
            output: serde_json::json!({
                "status": "ok",
                "data": {
                    "servers": [
                        {
                            "name": "filesystem",
                            "command": "npx",
                            "transport": "stdio",
                            "status": { "status": "running" }
                        }
                    ]
                }
            })
            .to_string(),
            is_error: false,
        };

        let summary = summarize_known_tool_output(&tool).expect("expected summary");
        assert!(summary.contains("Registered MCP servers"));
        assert!(summary.contains("filesystem"));
        assert!(summary.contains("running"));
    }

    #[test]
    fn summarize_virtual_channel_list_output_lists_registered_channels() {
        let tool = ToolExecution {
            name: "channel_bridge_list".to_string(),
            args: serde_json::json!({}),
            output: serde_json::json!({
                "status": "ok",
                "data": {
                    "channels": [
                        { "type": "webhook", "name": "alerts" }
                    ]
                }
            })
            .to_string(),
            is_error: false,
        };

        let summary = summarize_known_tool_output(&tool).expect("expected summary");
        assert!(summary.contains("Registered channels"));
        assert!(summary.contains("webhook"));
        assert!(summary.contains("alerts"));
    }

    #[test]
    fn summarize_virtual_mcp_call_output_includes_server_and_tool() {
        let tool = ToolExecution {
            name: "ext_mcp_call_tool".to_string(),
            args: serde_json::json!({"server": "filesystem", "tool": "read_file"}),
            output: serde_json::json!({
                "status": "ok",
                "data": {
                    "server": "filesystem",
                    "tool": "read_file",
                    "output": { "content": "hello" }
                }
            })
            .to_string(),
            is_error: false,
        };

        let summary = summarize_known_tool_output(&tool).expect("expected summary");
        assert!(summary.contains("filesystem"));
        assert!(summary.contains("read_file"));
        assert!(summary.contains("hello"));
    }

    #[test]
    fn summarize_virtual_channel_send_output_mentions_target() {
        let tool = ToolExecution {
            name: "channel_bridge_send".to_string(),
            args: serde_json::json!({"to": "agent-demo", "content": "ping"}),
            output: serde_json::json!({
                "status": "ok",
                "data": { "to": "agent-demo" }
            })
            .to_string(),
            is_error: false,
        };

        let summary = summarize_known_tool_output(&tool).expect("expected summary");
        assert!(summary.contains("agent-demo"));
        assert!(summary.contains("channel.bridge"));
    }

    #[test]
    fn build_virtual_capability_payload_defaults_agent_for_capability_tools() {
        let config = default_agent_config();
        let route = find_virtual_capability_tool(&config, "ext_mcp_list_servers")
            .expect("virtual MCP tool should exist");

        let payload =
            build_virtual_capability_payload(&route, "session-agent", &serde_json::json!({}));
        assert_eq!(payload["agent"], serde_json::json!("session-agent"));
    }

    #[test]
    fn arguments_missing_required_fields_detects_empty_query() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer" }
            },
            "required": ["query"]
        });

        assert!(arguments_missing_required_fields(
            &serde_json::json!({}),
            &schema
        ));
        assert!(arguments_missing_required_fields(
            &serde_json::json!({"query": ""}),
            &schema
        ));
        assert!(!arguments_missing_required_fields(
            &serde_json::json!({"query": "Uzi meaning"}),
            &schema
        ));
    }

    #[test]
    fn should_force_fs_read_skips_workflow_planning_requests_even_with_paths() {
        assert_eq!(
            should_force_fs_read("请基于 ./docs/release.md 给我做一个发布工作流计划"),
            None
        );
        assert_eq!(
            should_force_fs_read("Please plan a workflow for ./docs/release.md step by step"),
            None
        );
        assert_eq!(
            should_force_fs_read("请读取 ./docs/release.md 并总结内容"),
            Some("./docs/release.md".to_string())
        );
    }

    #[test]
    fn visible_assistant_reply_strips_tool_markers() {
        let reply = "I checked it for you.\n[Tool: web_search]\nresult line 1\nresult line 2";

        assert_eq!(visible_assistant_reply(reply), "I checked it for you.");
    }

    #[test]
    fn visible_assistant_reply_keeps_plain_text_replies() {
        let reply = "Hello!";

        assert_eq!(visible_assistant_reply(reply), "Hello!");
    }

    #[test]
    fn tool_result_is_error_detects_top_level_status_error() {
        let raw = r#"{"status":"error","error":"boom"}"#;
        assert!(tool_result_is_error(raw));
    }

    #[test]
    fn tool_result_is_error_detects_status_ok_with_nested_string_error_envelope() {
        let raw = r#"{"status":"ok","error":"{\"status\":\"error\",\"error\":\"shell command exited with status 1\"}"}"#;
        assert!(tool_result_is_error(raw));
    }

    #[test]
    fn tool_result_is_error_detects_nested_error_inside_data_field() {
        let raw =
            r#"{"status":"ok","data":{"status":"error","error":"git fatal: not a repository"}}"#;
        assert!(tool_result_is_error(raw));
    }

    #[test]
    fn tool_result_is_error_detects_string_response_field_with_error() {
        let raw = r#"{"status":"ok","response":"{\"status\":\"error\",\"error\":\"timeout\"}"}"#;
        assert!(tool_result_is_error(raw));
    }

    #[test]
    fn tool_result_is_error_returns_false_for_clean_ok_payload() {
        let raw = r#"{"status":"ok","data":{"stdout":"hello","stderr":"","status":0}}"#;
        assert!(!tool_result_is_error(raw));
    }

    #[test]
    fn tool_result_is_error_returns_false_for_empty_or_null_error_field() {
        let null_raw = r#"{"status":"ok","error":null}"#;
        let empty_raw = r#"{"status":"ok","error":""}"#;
        let empty_obj_raw = r#"{"status":"ok","error":{}}"#;
        assert!(!tool_result_is_error(null_raw));
        assert!(!tool_result_is_error(empty_raw));
        assert!(!tool_result_is_error(empty_obj_raw));
    }

    #[test]
    fn tool_result_is_error_returns_false_for_invalid_json() {
        assert!(!tool_result_is_error("not-json"));
        assert!(!tool_result_is_error(""));
    }

    #[test]
    fn fingerprint_is_stable_for_identical_input_and_distinct_for_different() {
        let a = fingerprint_str("hello world");
        let b = fingerprint_str("hello world");
        let c = fingerprint_str("hello world!");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn fingerprint_detects_whitespace_drift() {
        // A single trailing newline is enough to invalidate a prompt-cache prefix.
        let stable = fingerprint_str("You are an assistant.\n[Available tools]\n- foo");
        let drifted = fingerprint_str("You are an assistant.\n[Available tools]\n- foo\n");
        assert_ne!(stable, drifted);
    }

    #[test]
    fn canonical_json_emits_sorted_object_keys() {
        // Constructing the same object via two different key insertion orders
        // must produce byte-identical canonical JSON, otherwise the fingerprint
        // would drift purely from how upstream serialized its tool spec.
        let a = serde_json::json!({"b": 1, "a": 2, "c": 3});
        let b = serde_json::json!({"c": 3, "a": 2, "b": 1});
        assert_eq!(canonical_json_string(&a), canonical_json_string(&b));
        assert_eq!(canonical_json_string(&a), r#"{"a":2,"b":1,"c":3}"#);
    }

    #[test]
    fn build_tool_catalog_prompt_is_byte_stable_across_required_reorderings() {
        // Same tool described twice with `required` in different orders should
        // produce the same catalog text — required is a set, not a list, and
        // reordering it would otherwise invalidate prompt cache for every turn.
        let tool_a = serde_json::json!({
            "function": {
                "name": "fs_read",
                "description": "Read a file.",
                "parameters": {
                    "type": "object",
                    "properties": {"path": {"type": "string"}, "encoding": {"type": "string"}},
                    "required": ["path", "encoding"]
                }
            }
        });
        let tool_b = serde_json::json!({
            "function": {
                "name": "fs_read",
                "description": "Read a file.",
                "parameters": {
                    "type": "object",
                    "properties": {"path": {"type": "string"}, "encoding": {"type": "string"}},
                    "required": ["encoding", "path"]
                }
            }
        });
        let req = DelegateRequestInput {
            must_act: true,
            ..Default::default()
        };
        let prompt_a = build_tool_catalog_prompt(&[tool_a], Some(&req));
        let prompt_b = build_tool_catalog_prompt(&[tool_b], Some(&req));
        assert_eq!(prompt_a, prompt_b);
    }

    #[test]
    fn session_prefix_fp_key_is_namespaced_per_session() {
        // Two different sessions must not collide in the cross-turn fingerprint
        // store, otherwise a drift in session A would silently pollute session
        // B's invalidation count.
        let key_a = session_prefix_fp_key("sess-a");
        let key_b = session_prefix_fp_key("sess-b");
        assert_ne!(key_a, key_b);
        assert!(key_a.contains("sess-a"));
        assert!(key_b.contains("sess-b"));
    }

    #[test]
    fn save_session_prefix_fp_skips_empty_fingerprints() {
        // Persisting an empty string would make the *next* turn think the
        // previous turn had no measurable prefix and skip the comparison —
        // worse, it would erase whatever was previously stored. Guard against
        // that in `save_session_prefix_fp`.
        save_session_prefix_fp("sess-empty-test", "");
        // No assertion needed — the test_extism_host_stubs no-op kv_set won't
        // panic; the contract is "calling with empty must not crash and must
        // not blindly write".
    }

    #[test]
    fn required_action_state_tracks_fs_read_completion() {
        let mut state = RequiredActionState::from_request("请读取 D:\\test\\foo.txt 的内容");
        assert!(state.needs_fs_read);
        assert!(!state.missing_actions().is_empty());
        let tool = ToolExecution {
            name: "fs_read".to_string(),
            args: serde_json::json!({"path": "D:\\test\\foo.txt"}),
            output: r#"{"status":"ok","data":{"content":"hi"}}"#.to_string(),
            is_error: false,
        };
        state.observe(&tool);
        assert!(state.saw_fs_read_success);
        assert!(state.missing_actions().is_empty());
    }

    #[test]
    fn required_action_state_does_not_clear_when_tool_failed() {
        let mut state = RequiredActionState::from_request("please list the test directory");
        assert!(state.needs_fs_list);
        let tool = ToolExecution {
            name: "fs_list".to_string(),
            args: serde_json::json!({"path": "C:/test"}),
            output: r#"{"status":"error","error":"path not found"}"#.to_string(),
            is_error: true,
        };
        state.observe(&tool);
        assert!(!state.saw_fs_list_success);
        assert!(!state.missing_actions().is_empty());
    }

    #[test]
    fn required_action_state_distinguishes_git_query_from_commit() {
        let query_state = RequiredActionState::from_request("查看最近一次提交");
        assert!(query_state.needs_git_query);
        assert!(!query_state.needs_git_commit);

        let commit_state = RequiredActionState::from_request("请提交修改并给出 commit hash");
        assert!(commit_state.needs_git_commit);
    }

    #[test]
    fn summarize_shell_exec_output_shows_stdout() {
        let tool = ToolExecution {
            name: "shell_exec".to_string(),
            args: serde_json::json!({"command": "python", "args": ["test.py"]}),
            output: r#"{"status":"ok","data":{"status":0,"stdout":"FINAL_GATE_OK args=['one','two']","stderr":""}}"#.to_string(),
            is_error: false,
        };
        let summary = summarize_shell_exec_output(&tool).expect("expected summary");
        assert!(summary.contains("python"));
        assert!(summary.contains("FINAL_GATE_OK"));
    }

    #[test]
    fn summarize_shell_exec_output_shows_exit_code_on_failure() {
        let tool = ToolExecution {
            name: "shell_exec".to_string(),
            args: serde_json::json!({"command": "pwsh"}),
            output: r#"{"status":"ok","data":{"status":1,"stdout":"","stderr":"Access denied"}}"#
                .to_string(),
            is_error: false,
        };
        let summary = summarize_shell_exec_output(&tool).expect("expected summary");
        assert!(summary.contains("exit 1") || summary.contains("(exit 1)"));
        assert!(summary.contains("Access denied"));
    }

    #[test]
    fn summarize_git_output_shows_log_output() {
        let tool = ToolExecution {
            name: "git".to_string(),
            args: serde_json::json!({"args": ["-C", "/repo", "log", "--oneline", "-1"]}),
            output: r#"{"status":"ok","data":{"status":0,"stdout":"abc1234 initial commit","stderr":""}}"#.to_string(),
            is_error: false,
        };
        let summary = summarize_git_output(&tool).expect("expected summary");
        assert!(summary.contains("abc1234"));
        assert!(summary.contains("log"));
    }

    #[test]
    fn summarize_fs_list_output_shows_entries() {
        let tool = ToolExecution {
            name: "fs_list".to_string(),
            args: serde_json::json!({"path": "C:/test"}),
            output: r#"{"status":"ok","data":{"entries":[{"name":"foo.txt","kind":"file"},{"name":"bar","kind":"dir"}]}}"#.to_string(),
            is_error: false,
        };
        let summary = summarize_fs_list_output(&tool).expect("expected summary");
        assert!(summary.contains("foo.txt"));
        assert!(summary.contains("bar/"));
        assert!(summary.contains("C:/test"));
    }

    #[test]
    fn summarize_fs_list_output_handles_empty_directory() {
        let tool = ToolExecution {
            name: "fs_list".to_string(),
            args: serde_json::json!({"path": "C:/empty"}),
            output: r#"{"status":"ok","data":{"entries":[]}}"#.to_string(),
            is_error: false,
        };
        let summary = summarize_fs_list_output(&tool).expect("expected summary");
        assert!(summary.contains("empty") || summary.contains("C:/empty"));
    }

    #[test]
    fn summarize_fs_write_output_confirms_write() {
        let tool = ToolExecution {
            name: "fs_write".to_string(),
            args: serde_json::json!({"path": "C:/test/out.txt"}),
            output: r#"{"status":"ok","data":{"written":true}}"#.to_string(),
            is_error: false,
        };
        let summary = summarize_fs_write_output(&tool).expect("expected summary");
        assert!(summary.contains("C:/test/out.txt"));
        assert!(
            summary.to_lowercase().contains("wrote")
                || summary.to_lowercase().contains("written")
                || summary.to_lowercase().contains("success")
        );
    }

    #[test]
    fn needs_shell_not_triggered_by_bare_run_word() {
        // "run" alone in unrelated context should not trigger needs_shell
        let state = RequiredActionState::from_request("run by next week");
        assert!(!state.needs_shell);
    }

    #[test]
    fn needs_shell_triggered_by_run_script() {
        let state = RequiredActionState::from_request("please run the script test.py");
        assert!(state.needs_shell);
    }

    #[test]
    fn needs_shell_triggered_by_python_keyword() {
        let state = RequiredActionState::from_request("use python to process the data");
        assert!(state.needs_shell);
    }

    #[test]
    fn needs_web_fetch_not_triggered_by_bare_fetch() {
        // "fetch" without URL context should not trigger needs_web_fetch
        let state = RequiredActionState::from_request("fetch the latest version from npm");
        assert!(!state.needs_web_fetch);
    }

    #[test]
    fn needs_web_fetch_triggered_by_fetch_https() {
        let state = RequiredActionState::from_request(
            "please fetch https://example.com and show me the title",
        );
        assert!(state.needs_web_fetch);
    }

    #[test]
    fn needs_web_search_not_triggered_by_bare_search() {
        // "search" without web context should not trigger needs_web_search
        let state = RequiredActionState::from_request("search for the file in the project");
        assert!(!state.needs_web_search);
    }

    #[test]
    fn needs_web_search_triggered_by_search_the_web() {
        let state = RequiredActionState::from_request("search the web for the latest news");
        assert!(state.needs_web_search);
    }
}
