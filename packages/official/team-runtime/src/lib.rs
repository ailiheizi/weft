use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const PACKAGE_NAME: &str = "team-runtime";
const TEAM_RUNTIME_CAPABILITY: &str = "team.runtime";
const ROLE_CATALOG_CAPABILITY: &str = "team.role.catalog";
const SHARED_CONTEXT_CAPABILITY: &str = "team.context.shared";
const SCHEMA_VERSION: u32 = 1;
const DEFAULT_ROLE_SET_ID: &str = "devteam-v1";
const AGENT_RUNTIME_PLUGIN: &str = "agent-runtime";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamRoleDefinition {
    role_id: String,
    title: String,
    summary: String,
    responsibilities: Vec<String>,
    default_phase_ids: Vec<String>,
    capability_hints: Vec<String>,
    delegate_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamRoleCatalog {
    schema_version: u32,
    catalog_id: String,
    title: String,
    roles: Vec<TeamRoleDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SharedContextProjection {
    projection_id: String,
    title: String,
    summary: String,
    source_capabilities: Vec<String>,
    fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SharedContextPolicy {
    policy_id: String,
    mode: String,
    writable: bool,
    default_scope: String,
    projections: Vec<SharedContextProjection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamMemberRecord {
    member_id: String,
    role_id: String,
    display_name: String,
    agent_binding: String,
    #[serde(default)]
    skill_set_refs: Vec<String>,
    state: String,
    #[serde(default)]
    current_task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamSessionRecord {
    schema_version: u32,
    session_id: String,
    board_id: String,
    workflow_id: String,
    role_set_id: String,
    active_role_id: String,
    status: String,
    members: Vec<TeamMemberRecord>,
    created_at: u64,
    updated_at: u64,
    metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SessionIndexEntry {
    session_id: String,
    updated_at: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamSessionInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    workflow_id: String,
    #[serde(default)]
    role_set_id: String,
    #[serde(default)]
    active_role_id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    members: Vec<TeamMemberRecord>,
    #[serde(default)]
    timestamp: Option<u64>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct SessionLookupInput {
    #[serde(default)]
    session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ActivateRoleInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    role_id: String,
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    workflow_id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    timestamp: Option<u64>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct InspectRuntimeInput {
    #[serde(default)]
    session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ResolveSharedContextInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    role_id: String,
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    workflow_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DelegateRouteInput {
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
    context_refs: Vec<String>,
    #[serde(default)]
    metadata: Value,
    /// 纯规划模式:透传给 agent-core,使该 delegate 不获得任何工具(强制只输出文本)。
    #[serde(default)]
    planning_only: bool,
}

fn read_delegate_depth(input: &DelegateRouteInput) -> u64 {
    let metadata_depth = input
        .metadata
        .get("delegate_depth")
        .and_then(Value::as_u64);
    let session_context_depth = input
        .metadata
        .get("session_context")
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(|item| item.get("delegate_depth").and_then(Value::as_u64)));
    metadata_depth.or(session_context_depth).unwrap_or(0)
}

fn set_delegate_depth(delegate_request: &mut Value, delegate_depth: u64) {
    if let Some(request) = delegate_request.as_object_mut() {
        let existing_runtime_context = request
            .get("runtime_context")
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        let mut runtime_context = match existing_runtime_context {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        runtime_context.insert("delegate_depth".to_string(), json!(delegate_depth));
        request.insert("runtime_context".to_string(), Value::Object(runtime_context));

        let existing_session_context = request
            .get("session_context")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new()));
        let mut session_context = match existing_session_context {
            Value::Array(items) => items,
            _ => Vec::new(),
        };
        if session_context.is_empty() {
            session_context.push(json!({ "delegate_depth": delegate_depth }));
        } else if let Some(first) = session_context.first_mut() {
            match first {
                Value::Object(map) => {
                    map.insert("delegate_depth".to_string(), json!(delegate_depth));
                }
                other => {
                    *other = json!({ "delegate_depth": delegate_depth });
                }
            }
        }
        request.insert("session_context".to_string(), Value::Array(session_context));
    }
}

fn extract_delegate_reply(data: &Value) -> Result<String, String> {
    let reply = data
        .get("reply")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if reply.is_empty() {
        Err("delegate agent returned empty reply".to_string())
    } else {
        Ok(reply)
    }
}

fn parse_package_ws_result(raw: Result<String, String>) -> Result<Value, String> {
    let payload = raw?;
    let parsed: PackageResult =
        serde_json::from_str(&payload).map_err(|error| format!("invalid package result: {}", error))?;
    if parsed.status != "ok" {
        return Err(parsed
            .error
            .unwrap_or_else(|| "package action failed".to_string()));
    }
    Ok(parsed.data.unwrap_or(Value::Null))
}

fn normalize_session_token(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_metadata(metadata: Value) -> Value {
    match metadata {
        Value::Null => Value::Object(Map::new()),
        other => other,
    }
}

fn ensure_non_empty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("missing {}", field))
    } else {
        Ok(())
    }
}

fn default_role_set_id(value: &str) -> String {
    if value.trim().is_empty() {
        DEFAULT_ROLE_SET_ID.to_string()
    } else {
        value.trim().to_string()
    }
}

fn default_status(value: &str) -> String {
    if value.trim().is_empty() {
        "active".to_string()
    } else {
        value.trim().to_string()
    }
}

fn now_ts() -> u64 {
    now_ms()
}

fn role_catalog() -> TeamRoleCatalog {
    TeamRoleCatalog {
        schema_version: SCHEMA_VERSION,
        catalog_id: DEFAULT_ROLE_SET_ID.to_string(),
        title: "Weft Claw DevTeam Canonical Roles".to_string(),
        roles: vec![
            TeamRoleDefinition {
                role_id: "planner".to_string(),
                title: "Planner".to_string(),
                summary: "Shapes scope, clarifies acceptance boundaries, and prepares the next execution slice.".to_string(),
                responsibilities: vec![
                    "Clarify task scope and constraints".to_string(),
                    "Map work to workflow phases".to_string(),
                    "Prepare review-ready handoff context".to_string(),
                ],
                default_phase_ids: vec!["intake".to_string(), "plan".to_string(), "review".to_string()],
                capability_hints: vec![
                    "workflow.template.devteam".to_string(),
                    "team.taskboard".to_string(),
                    "team.runtime".to_string(),
                ],
                delegate_targets: vec!["implementer".to_string(), "reviewer".to_string()],
            },
            TeamRoleDefinition {
                role_id: "implementer".to_string(),
                title: "Implementer".to_string(),
                summary: "Turns approved plan slices into concrete code and artifacts.".to_string(),
                responsibilities: vec![
                    "Deliver the requested implementation slice".to_string(),
                    "Keep task state current".to_string(),
                    "Leave evidence for review and integration".to_string(),
                ],
                default_phase_ids: vec!["execute".to_string(), "integrate".to_string()],
                capability_hints: vec![
                    "agent.runtime".to_string(),
                    "team.taskboard".to_string(),
                    "team.context.shared".to_string(),
                ],
                delegate_targets: vec!["reviewer".to_string(), "integrator".to_string()],
            },
            TeamRoleDefinition {
                role_id: "reviewer".to_string(),
                title: "Reviewer".to_string(),
                summary: "Checks correctness, risk, and readiness before the work is integrated.".to_string(),
                responsibilities: vec![
                    "Validate implementation against requested outcome".to_string(),
                    "Identify regressions or missing evidence".to_string(),
                    "Signal pass or change request".to_string(),
                ],
                default_phase_ids: vec!["review".to_string()],
                capability_hints: vec![
                    "team.handoff".to_string(),
                    "team.context.shared".to_string(),
                    "workflow.template.devteam".to_string(),
                ],
                delegate_targets: vec!["implementer".to_string(), "integrator".to_string()],
            },
            TeamRoleDefinition {
                role_id: "integrator".to_string(),
                title: "Integrator".to_string(),
                summary: "Finalizes accepted work, aligns artifacts, and closes the active runtime slice.".to_string(),
                responsibilities: vec![
                    "Complete integration-ready checks".to_string(),
                    "Confirm binding between task, review, and final status".to_string(),
                    "Advance or close the team session".to_string(),
                ],
                default_phase_ids: vec!["integrate".to_string(), "done".to_string()],
                capability_hints: vec![
                    "team.runtime".to_string(),
                    "team.handoff".to_string(),
                    "workflow.orchestration".to_string(),
                ],
                delegate_targets: vec!["planner".to_string()],
            },
        ],
    }
}

fn shared_context_policy() -> SharedContextPolicy {
    SharedContextPolicy {
        policy_id: "devteam-shared-context-v1".to_string(),
        mode: "projection".to_string(),
        writable: true,
        default_scope: "team-session".to_string(),
        projections: vec![
            SharedContextProjection {
                projection_id: "decision-log".to_string(),
                title: "Decision Log".to_string(),
                summary:
                    "Shared decisions, assumptions, and constraints carried across devteam phases."
                        .to_string(),
                source_capabilities: vec!["memory.store".to_string(), "team.runtime".to_string()],
                fields: vec![
                    "session_id".to_string(),
                    "board_id".to_string(),
                    "summary".to_string(),
                    "decisions".to_string(),
                    "constraints".to_string(),
                ],
            },
            SharedContextProjection {
                projection_id: "execution-brief".to_string(),
                title: "Execution Brief".to_string(),
                summary: "Compact handoff-oriented projection for implementers and reviewers."
                    .to_string(),
                source_capabilities: vec!["team.taskboard".to_string(), "team.handoff".to_string()],
                fields: vec![
                    "task_refs".to_string(),
                    "handoff_refs".to_string(),
                    "expected_outcome".to_string(),
                    "artifact_refs".to_string(),
                ],
            },
        ],
    }
}

fn sessions_index_key() -> String {
    format!("{}:sessions", PACKAGE_NAME)
}
fn session_key(session_id: &str) -> String {
    format!("{}:session:{}", PACKAGE_NAME, session_id.trim())
}

fn parse_json_or_default<T>(raw: Option<String>) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    raw.and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn write_json<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    let json = serde_json::to_string(value).map_err(|error| error.to_string())?;
    kv_set(key, &json);
    Ok(())
}

fn push_or_replace<T, F>(items: &mut Vec<T>, new_item: T, same: F)
where
    F: Fn(&T, &T) -> bool,
{
    if let Some(index) = items.iter().position(|existing| same(existing, &new_item)) {
        items[index] = new_item;
    } else {
        items.push(new_item);
    }
}

fn default_member_for_role(role: &TeamRoleDefinition) -> TeamMemberRecord {
    TeamMemberRecord {
        member_id: role.role_id.clone(),
        role_id: role.role_id.clone(),
        display_name: role.title.clone(),
        agent_binding: AGENT_RUNTIME_PLUGIN.to_string(),
        skill_set_refs: role.capability_hints.clone(),
        state: "idle".to_string(),
        current_task_id: String::new(),
    }
}

fn default_members() -> Vec<TeamMemberRecord> {
    role_catalog()
        .roles
        .iter()
        .map(default_member_for_role)
        .collect()
}

fn load_session_record(session_id: &str) -> Option<TeamSessionRecord> {
    kv_get(&session_key(session_id)).and_then(|json| serde_json::from_str(&json).ok())
}

fn persist_session(record: &TeamSessionRecord) -> Result<(), String> {
    write_json(&session_key(&record.session_id), record)?;
    let mut index: Vec<SessionIndexEntry> = parse_json_or_default(kv_get(&sessions_index_key()));
    push_or_replace(
        &mut index,
        SessionIndexEntry {
            session_id: record.session_id.clone(),
            updated_at: record.updated_at,
        },
        |left, right| left.session_id == right.session_id,
    );
    index.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then(left.session_id.cmp(&right.session_id))
    });
    write_json(&sessions_index_key(), &index)
}

fn save_session_result(input: TeamSessionInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.session_id, "session_id") {
        return PackageResult::err(error);
    }
    let timestamp = input.timestamp.unwrap_or_else(now_ts);
    let existing = load_session_record(&input.session_id);
    let members = if input.members.is_empty() {
        existing
            .as_ref()
            .map(|record| record.members.clone())
            .unwrap_or_else(default_members)
    } else {
        input.members
    };
    let record = TeamSessionRecord {
        schema_version: SCHEMA_VERSION,
        session_id: input.session_id.trim().to_string(),
        board_id: input.board_id.trim().to_string(),
        workflow_id: input.workflow_id.trim().to_string(),
        role_set_id: default_role_set_id(&input.role_set_id),
        active_role_id: input.active_role_id.trim().to_string(),
        status: default_status(&input.status),
        members,
        created_at: existing
            .as_ref()
            .map(|record| record.created_at)
            .unwrap_or(timestamp),
        updated_at: timestamp,
        metadata: normalize_metadata(input.metadata),
    };
    if let Err(error) = persist_session(&record) {
        return PackageResult::err(error);
    }
    PackageResult::ok(json!({ "session": record }))
}

fn get_session_result(input: SessionLookupInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.session_id, "session_id") {
        return PackageResult::err(error);
    }
    match load_session_record(&input.session_id) {
        Some(session) => PackageResult::ok(json!({ "session": session })),
        None => PackageResult::err(format!("session '{}' not found", input.session_id.trim())),
    }
}

fn list_sessions_result() -> PackageResult {
    let index: Vec<SessionIndexEntry> = parse_json_or_default(kv_get(&sessions_index_key()));
    let sessions = index
        .into_iter()
        .filter_map(|entry| load_session_record(&entry.session_id))
        .collect::<Vec<_>>();
    PackageResult::ok(json!({ "sessions": sessions, "count": sessions.len() }))
}

fn activate_role_result(input: ActivateRoleInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.session_id, "session_id") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.role_id, "role_id") {
        return PackageResult::err(error);
    }
    let catalog = role_catalog();
    let role = match catalog
        .roles
        .iter()
        .find(|candidate| candidate.role_id == input.role_id.trim())
    {
        Some(role) => role,
        None => return PackageResult::err(format!("unknown role '{}'", input.role_id.trim())),
    };
    let existing = load_session_record(&input.session_id);
    let timestamp = input.timestamp.unwrap_or_else(now_ts);
    let mut members = existing
        .as_ref()
        .map(|record| record.members.clone())
        .unwrap_or_else(default_members);
    for member in &mut members {
        member.state = if member.role_id == role.role_id {
            "active".to_string()
        } else {
            "idle".to_string()
        };
    }
    let record = TeamSessionRecord {
        schema_version: SCHEMA_VERSION,
        session_id: input.session_id.trim().to_string(),
        board_id: if input.board_id.trim().is_empty() {
            existing
                .as_ref()
                .map(|record| record.board_id.clone())
                .unwrap_or_default()
        } else {
            input.board_id.trim().to_string()
        },
        workflow_id: if input.workflow_id.trim().is_empty() {
            existing
                .as_ref()
                .map(|record| record.workflow_id.clone())
                .unwrap_or_default()
        } else {
            input.workflow_id.trim().to_string()
        },
        role_set_id: existing
            .as_ref()
            .map(|record| record.role_set_id.clone())
            .unwrap_or_else(|| DEFAULT_ROLE_SET_ID.to_string()),
        active_role_id: role.role_id.clone(),
        status: if input.status.trim().is_empty() {
            format!("role:{}", role.role_id)
        } else {
            input.status.trim().to_string()
        },
        members,
        created_at: existing
            .as_ref()
            .map(|record| record.created_at)
            .unwrap_or(timestamp),
        updated_at: timestamp,
        metadata: normalize_metadata(input.metadata),
    };
    if let Err(error) = persist_session(&record) {
        return PackageResult::err(error);
    }
    PackageResult::ok(json!({ "activated": true, "session": record, "role": role }))
}

fn inspect_runtime_result(input: InspectRuntimeInput) -> PackageResult {
    let catalog = role_catalog();
    let policy = shared_context_policy();
    let session = if input.session_id.trim().is_empty() {
        None
    } else {
        load_session_record(&input.session_id)
    };
    PackageResult::ok(json!({
        "package": PACKAGE_NAME,
        "runtime": {
            "healthy": true,
            "schema_version": SCHEMA_VERSION,
            "default_role_set_id": DEFAULT_ROLE_SET_ID,
            "session_count": parse_json_or_default::<Vec<SessionIndexEntry>>(kv_get(&sessions_index_key())).len(),
            "delegate_provider": AGENT_RUNTIME_PLUGIN,
        },
        "catalog": catalog,
        "shared_context": policy,
        "session": session,
    }))
}

fn resolve_shared_context_result(input: ResolveSharedContextInput) -> PackageResult {
    let session = if input.session_id.trim().is_empty() {
        None
    } else {
        load_session_record(&input.session_id)
    };
    let policy = shared_context_policy();
    let role_id = if input.role_id.trim().is_empty() {
        session
            .as_ref()
            .map(|record| record.active_role_id.clone())
            .unwrap_or_else(|| "planner".to_string())
    } else {
        input.role_id.trim().to_string()
    };
    PackageResult::ok(json!({
        "capability": SHARED_CONTEXT_CAPABILITY,
        "session_id": input.session_id.trim(),
        "board_id": if input.board_id.trim().is_empty() { session.as_ref().map(|record| record.board_id.clone()).unwrap_or_default() } else { input.board_id.trim().to_string() },
        "workflow_id": if input.workflow_id.trim().is_empty() { session.as_ref().map(|record| record.workflow_id.clone()).unwrap_or_default() } else { input.workflow_id.trim().to_string() },
        "role_id": role_id,
        "projection_mode": policy.mode,
        "scope": policy.default_scope,
        "projections": policy.projections,
    }))
}

/// 按角色 id 查 KV 里的 role_routing(core 从 config [team.roleRouting] 注入),
/// 返回 (model_override, provider_override),未命中则为 (None, None)。
fn lookup_role_model(role_id: &str) -> (Option<String>, Option<String>) {
    let raw = match kv_get("team:role_routing") {
        Some(s) if !s.trim().is_empty() => s,
        _ => return (None, None),
    };
    let map: std::collections::HashMap<String, Value> = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(_) => return (None, None),
    };
    let entry = match map.get(role_id) {
        Some(v) => v,
        None => return (None, None),
    };
    let pick = |key: &str| {
        entry
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    (pick("model"), pick("provider"))
}

fn build_delegate_payload(
    input: &DelegateRouteInput,
    session: Option<&TeamSessionRecord>,
) -> Result<Value, String> {
    let from_role_id = if input.from_role_id.trim().is_empty() {
        session
            .map(|record| record.active_role_id.clone())
            .unwrap_or_else(|| "planner".to_string())
    } else {
        input.from_role_id.trim().to_string()
    };
    let to_role_id = if input.to_role_id.trim().is_empty() {
        return Err("missing to_role_id".to_string());
    } else {
        input.to_role_id.trim().to_string()
    };
    let catalog = role_catalog();
    let from_role = catalog
        .roles
        .iter()
        .find(|role| role.role_id == from_role_id)
        .ok_or_else(|| format!("unknown from_role_id '{}'", input.from_role_id.trim()))?;
    let to_role = catalog
        .roles
        .iter()
        .find(|role| role.role_id == to_role_id)
        .ok_or_else(|| format!("unknown to_role_id '{}'", input.to_role_id.trim()))?;
    if from_role.role_id != to_role.role_id
        && !from_role
            .delegate_targets
            .iter()
            .any(|candidate| candidate == &to_role.role_id)
    {
        return Err(format!(
            "role '{}' cannot delegate to '{}'",
            from_role.role_id, to_role.role_id
        ));
    }
    let target_member = session
        .and_then(|record| {
            record
                .members
                .iter()
                .find(|member| member.role_id == to_role.role_id)
        })
        .cloned()
        .unwrap_or_else(|| default_member_for_role(to_role));
    let content = if input.prompt.trim().is_empty() {
        format!(
            "Delegate task '{}' from {} to {}",
            input.task_id.trim(),
            from_role.role_id,
            to_role.role_id
        )
    } else {
        input.prompt.clone()
    };
    // 模型分层:按目标角色查 KV 里的 role_routing(core 启动时从 config [team.roleRouting] 写入),
    // 命中则把 model/provider 注入 delegate_request,agent-core 据此覆盖默认模型。
    let (model_override, provider_override) = lookup_role_model(&to_role.role_id);
    Ok(json!({
        "delegate_provider": AGENT_RUNTIME_PLUGIN,
        "session_id": input.session_id.trim(),
        "board_id": if input.board_id.trim().is_empty() { session.map(|record| record.board_id.clone()).unwrap_or_default() } else { input.board_id.trim().to_string() },
        "workflow_id": if input.workflow_id.trim().is_empty() { session.map(|record| record.workflow_id.clone()).unwrap_or_default() } else { input.workflow_id.trim().to_string() },
        "task_id": input.task_id.trim(),
        "from_role_id": from_role.role_id,
        "to_role_id": to_role.role_id,
        "target_member": target_member,
        "route": {
            "mode": "team-delegate",
            "reason": input.reason,
            "must_act": input.must_act,
            "context_refs": input.context_refs,
        },
        "agent_request": {
            "agent": target_member.agent_binding,
            "content": content,
            "delegate_request": {
                "must_act": input.must_act,
                "reason": input.reason,
                "latest_user_query": content,
                "visible_history": [],
                "planning_only": input.planning_only,
                "model_override": model_override,
                "provider_override": provider_override,
                "session_context": [json!({
                    "session_id": input.session_id.trim(),
                    "board_id": input.board_id.trim(),
                    "workflow_id": input.workflow_id.trim(),
                    "task_id": input.task_id.trim(),
                    "from_role_id": input.from_role_id.trim(),
                    "to_role_id": input.to_role_id.trim(),
                    "context_refs": input.context_refs,
                    "metadata": normalize_metadata(input.metadata.clone()),
                })],
            }
        }
    }))
}

fn delegate_contract_result(input: DelegateRouteInput) -> PackageResult {
    let session = if input.session_id.trim().is_empty() {
        None
    } else {
        load_session_record(&input.session_id)
    };
    let payload = match build_delegate_payload(&input, session.as_ref()) {
        Ok(payload) => payload,
        Err(error) => return PackageResult::err(error),
    };
    PackageResult::ok(json!({
        "capability": "team.delegate",
        "delegate": payload,
    }))
}

fn execute_delegate_result(input: DelegateRouteInput) -> PackageResult {
    let session = if input.session_id.trim().is_empty() {
        None
    } else {
        load_session_record(&input.session_id)
    };
    let payload = match build_delegate_payload(&input, session.as_ref()) {
        Ok(payload) => payload,
        Err(error) => return PackageResult::err(error),
    };

    let from_role_id = input.from_role_id.trim();
    let to_role_id = input.to_role_id.trim();
    // 允许同角色"自委托"工作(如 planner 在 plan 阶段对任务做规划/分解):
    // 这是合法的阶段内工作执行,不是无限递归。由下面的 delegate_depth<=3 上界防环,
    // 且下方 delegate_targets 白名单校验对 from==to 放行。
    let _ = (from_role_id, to_role_id);

    let delegate_depth = read_delegate_depth(&input);
    if delegate_depth >= 3 {
        return PackageResult::err("delegate depth exceeded".to_string());
    }

    let agent = payload
        .get("agent_request")
        .and_then(|value| value.get("agent"))
        .and_then(Value::as_str)
        .unwrap_or(AGENT_RUNTIME_PLUGIN)
        .trim()
        .to_string();
    let content = payload
        .get("agent_request")
        .and_then(|value| value.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mut delegate_request = payload
        .get("agent_request")
        .and_then(|value| value.get("delegate_request"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    set_delegate_depth(&mut delegate_request, delegate_depth.saturating_add(1));

    let board_id = normalize_session_token(
        payload.get("board_id").and_then(Value::as_str).unwrap_or(""),
        "unknown-board",
    );
    let task_id = normalize_session_token(
        payload.get("task_id").and_then(Value::as_str).unwrap_or(""),
        "unknown-task",
    );
    let to_role_id = normalize_session_token(
        payload.get("to_role_id").and_then(Value::as_str).unwrap_or(""),
        "unknown-role",
    );
    let session_id = format!("team-delegate-{}-{}-{}", board_id, task_id, to_role_id);
    let timestamp = now_ms();

    let create_result = call_package_ws_action(
        &agent,
        "create_session",
        &json!({
            "id": session_id,
            "title": format!("delegate {}", to_role_id),
            "persistent": 0,
            "created_at": timestamp,
            "updated_at": timestamp,
        }),
    );
    if let Err(error) = parse_package_ws_result(create_result) {
        let create_error = error.trim().to_lowercase();
        if !create_error.contains("exist") && !create_error.contains("already") {
            log_warn(&format!(
                "team-runtime create_session failed for delegate session {}: {}",
                session_id, error
            ));
        }
    }

    let send_result = parse_package_ws_result(call_package_ws_action(
        &agent,
        "send_session_message",
        &json!({
            "session_id": session_id,
            "content": content,
            "content_b64": "",
            "delegate_request": delegate_request,
        }),
    ));
    let send_data = match send_result {
        Ok(data) => data,
        Err(error) => return PackageResult::err(error),
    };

    let reply = match extract_delegate_reply(&send_data) {
        Ok(reply) => reply,
        Err(error) => return PackageResult::err(error),
    };

    PackageResult::ok(json!({
        "status": "executed",
        "session_id": session_id,
        "agent": agent,
        "reply": reply,
        "delegate": payload,
    }))
}

fn describe_result() -> PackageResult {
    PackageResult::ok(json!({
        "package": PACKAGE_NAME,
        "runtime": "wasm",
        "capabilities": [TEAM_RUNTIME_CAPABILITY, ROLE_CATALOG_CAPABILITY, SHARED_CONTEXT_CAPABILITY],
        "actions": {
            TEAM_RUNTIME_CAPABILITY: ["describe", "health", "save_session", "get_session", "list_sessions", "inspect_runtime", "activate", "activate_role", "execute_delegate"],
            ROLE_CATALOG_CAPABILITY: ["describe", "health", "list_roles", "get_catalog"],
            SHARED_CONTEXT_CAPABILITY: ["describe", "health", "resolve_shared_context", "inspect_shared_context", "get_delegate_contract"],
        },
    }))
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("team-runtime initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });
    let result = match req.action.as_str() {
        "describe" => describe_result(),
        "health" => PackageResult::ok(json!({
            "healthy": true,
            "package": PACKAGE_NAME,
            "capabilities": [TEAM_RUNTIME_CAPABILITY, ROLE_CATALOG_CAPABILITY, SHARED_CONTEXT_CAPABILITY],
        })),
        "save_session" | "call" => {
            let payload: TeamSessionInput =
                serde_json::from_value(req.data).unwrap_or(TeamSessionInput {
                    session_id: String::new(),
                    board_id: String::new(),
                    workflow_id: String::new(),
                    role_set_id: String::new(),
                    active_role_id: String::new(),
                    status: String::new(),
                    members: Vec::new(),
                    timestamp: None,
                    metadata: Value::Null,
                });
            save_session_result(payload)
        }
        "get_session" => get_session_result(serde_json::from_value(req.data).unwrap_or(
            SessionLookupInput {
                session_id: String::new(),
            },
        )),
        "list_sessions" => list_sessions_result(),
        "activate" | "activate_role" => {
            let payload: ActivateRoleInput =
                serde_json::from_value(req.data).unwrap_or(ActivateRoleInput {
                    session_id: String::new(),
                    role_id: String::new(),
                    board_id: String::new(),
                    workflow_id: String::new(),
                    status: String::new(),
                    timestamp: None,
                    metadata: Value::Null,
                });
            activate_role_result(payload)
        }
        "inspect_runtime" => inspect_runtime_result(serde_json::from_value(req.data).unwrap_or(
            InspectRuntimeInput {
                session_id: String::new(),
            },
        )),
        "list_roles" | "get_catalog" => PackageResult::ok(json!({ "catalog": role_catalog() })),
        "resolve_shared_context" | "inspect_shared_context" => {
            let payload: ResolveSharedContextInput =
                serde_json::from_value(req.data).unwrap_or(ResolveSharedContextInput {
                    session_id: String::new(),
                    role_id: String::new(),
                    board_id: String::new(),
                    workflow_id: String::new(),
                });
            resolve_shared_context_result(payload)
        }
        "get_delegate_contract" => {
            let payload: DelegateRouteInput =
                serde_json::from_value(req.data).unwrap_or(DelegateRouteInput {
                    session_id: String::new(),
                    board_id: String::new(),
                    workflow_id: String::new(),
                    from_role_id: String::new(),
                    to_role_id: String::new(),
                    task_id: String::new(),
                    prompt: String::new(),
                    reason: String::new(),
                    must_act: false,
                    context_refs: Vec::new(),
                    metadata: Value::Null,
                    planning_only: false,
                });
            delegate_contract_result(payload)
        }
        "execute_delegate" => {
            let payload: DelegateRouteInput =
                serde_json::from_value(req.data).unwrap_or(DelegateRouteInput {
                    session_id: String::new(),
                    board_id: String::new(),
                    workflow_id: String::new(),
                    from_role_id: String::new(),
                    to_role_id: String::new(),
                    task_id: String::new(),
                    prompt: String::new(),
                    reason: String::new(),
                    must_act: false,
                    context_refs: Vec::new(),
                    metadata: Value::Null,
                    planning_only: false,
                });
            execute_delegate_result(payload)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };
    Ok(result.to_json())
}

