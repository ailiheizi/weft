//! WEFT-Code runtime checkpoint package.
//!
//! This package provides the `weft_code.runtime` capability through WASM action
//! dispatch. It intentionally does not start or claim ownership of HTTP
//! `/api/weft-code/*` routes; host-side compatibility can bridge those routes to
//! this capability in a later checkpoint.

use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const PROVIDER: &str = "weft-code-runtime";
const CAPABILITY: &str = "weft_code.runtime";
const BOOTSTRAP_APPROVAL_ID: &str = "weft-code-bootstrap-approval";
const SESSION_PREFIX: &str = "weft-code-runtime:sessions:";
const APPROVAL_PREFIX: &str = "weft-code-runtime:approvals:";
const NEXT_ID_KEY: &str = "weft-code-runtime:next-session-id";
const POLICY_KEY: &str = "weft-code-runtime:policy";
const DEFAULT_POLICY: &str = "on_request";
const HOST_EXECUTION_CAPABILITY: &str = "core.execution";
const HOST_EXECUTION_DESCRIBE_ACTION: &str = "describe";
const HOST_EXECUTION_RUN_ACTION: &str = "run";

#[extism_pdk::host_fn]
extern "ExtismHost" {
    fn host_capability_call(input: String) -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    id: String,
    title: String,
    prompt: String,
    mode: String,
    status: String,
    task_status: String,
    created_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApprovalState {
    id: String,
    session_id: String,
    prompt: String,
    target_id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct CreateTaskInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    natural_language_task: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    target_id: String,
}

#[derive(Debug, Deserialize)]
struct SessionInput {
    #[serde(default)]
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct TeamInput {
    #[serde(default)]
    team_id: String,
}

#[derive(Debug, Deserialize)]
struct ModeInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    mode: String,
}

#[derive(Debug, Deserialize)]
struct ApprovalDecisionInput {
    #[serde(default)]
    approval_id: String,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Deserialize)]
struct PolicyInput {
    #[serde(default, deserialize_with = "empty_string_on_null")]
    policy: String,
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("weft-code-runtime WASM package initialized");
    Ok(PackageResult::ok(json!({
        "provider": PROVIDER,
        "capability": CAPABILITY,
        "runtime": "wasm",
        "actions": supported_actions(),
        "http_routes": "not_implemented_in_package_checkpoint"
    }))
    .to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });

    let result = dispatch_action(&req.action, req.data);
    Ok(result.to_json())
}

#[plugin_fn]
pub fn status(input: String) -> FnResult<String> {
    let data = parse_optional_json(&input)?;
    Ok(do_status(data).to_json())
}

#[plugin_fn]
pub fn list_sessions(_input: String) -> FnResult<String> {
    Ok(do_list_sessions().to_json())
}

#[plugin_fn]
pub fn create_natural_language_task(input: String) -> FnResult<String> {
    let data = parse_required_json(&input)?;
    Ok(do_create_natural_language_task(data).to_json())
}

fn dispatch_action(action: &str, data: Value) -> PackageResult {
    match action {
        "status" => do_status(data),
        "list_sessions" => do_list_sessions(),
        "list_teams" => do_list_teams(data),
        "list_team_tasks" => do_list_team_tasks(data),
        "update_session_mode" => do_update_session_mode(data),
        "get_policy" => do_get_policy(),
        "update_policy" => do_update_policy(data),
        "list_approvals" => do_list_approvals(),
        "list_session_tasks" => do_list_session_tasks(data),
        "list_events" => do_list_events(),
        "approval_decision" => do_approval_decision(data),
        "execution_probe" => do_execution_probe(),
        "create_natural_language_task" => do_create_natural_language_task(data),
        "" => PackageResult::err("missing action"),
        other => PackageResult::err(format!("unknown action: {other}")),
    }
}

fn do_status(_data: Value) -> PackageResult {
    let sessions = session_views();
    PackageResult::ok(json!({
        "sessions": sessions,
        "policy": policy_view(),
        "teams": team_views(),
        "approvals": approval_views(),
        "tasks": task_views(None)
    }))
}

fn do_list_sessions() -> PackageResult {
    PackageResult::ok(Value::Array(session_views()))
}

fn do_list_teams(_data: Value) -> PackageResult {
    PackageResult::ok(Value::Array(team_views()))
}

fn do_list_team_tasks(data: Value) -> PackageResult {
    let input: TeamInput = serde_json::from_value(data).unwrap_or(TeamInput {
        team_id: String::new(),
    });
    let tasks = if input.team_id == "weft-code-local-team" {
        vec![json!({
            "id": "weft-code-local-team-task",
            "team_id": "weft-code-local-team",
            "role": "operator",
            "phase": "bootstrap",
            "status": "queued"
        })]
    } else {
        vec![]
    };
    PackageResult::ok(Value::Array(tasks))
}

fn do_update_session_mode(data: Value) -> PackageResult {
    let input: ModeInput = match serde_json::from_value(data) {
        Ok(input) => input,
        Err(err) => return PackageResult::err(format!("parse error: {err}")),
    };
    if input.session_id.trim().is_empty() || input.mode.trim().is_empty() {
        return PackageResult::err("missing session_id or mode");
    }

    let mut session = load_sessions()
        .into_iter()
        .find(|session| session.id == input.session_id)
        .unwrap_or_else(|| default_session(&input.session_id));
    session.mode = input.mode;
    save_session(&session);

    PackageResult::ok(session_view(&session))
}

fn do_get_policy() -> PackageResult {
    PackageResult::ok(policy_view())
}

fn do_update_policy(data: Value) -> PackageResult {
    let input: PolicyInput = match serde_json::from_value(data) {
        Ok(input) => input,
        Err(err) => return PackageResult::err(format!("parse error: {err}")),
    };
    let policy = input.policy.trim();
    if policy.is_empty() {
        return PackageResult::err("missing policy");
    }

    kv_set(POLICY_KEY, policy);
    PackageResult::ok(policy_view())
}

fn do_list_approvals() -> PackageResult {
    PackageResult::ok(Value::Array(approval_views()))
}

fn do_list_session_tasks(data: Value) -> PackageResult {
    let input: SessionInput = serde_json::from_value(data).unwrap_or(SessionInput {
        session_id: String::new(),
    });
    PackageResult::ok(Value::Array(task_views(Some(&input.session_id))))
}

fn do_list_events() -> PackageResult {
    let mut sequence = 0_u64;
    let mut events = vec![timeline_event(
        &mut sequence,
        "runtime-bootstrap",
        "runtime.bootstrap",
        "runtime",
        PROVIDER,
        "weft-code-runtime is available",
        json!({
            "provider": PROVIDER,
            "capability": CAPABILITY
        }),
    )];

    let policy = load_policy();
    events.push(timeline_event(
        &mut sequence,
        "policy-current",
        "policy.current",
        "policy",
        "current",
        &format!("policy is {policy}"),
        json!({ "policy": policy }),
    ));

    for session in load_sessions() {
        events.push(timeline_event(
            &mut sequence,
            &format!("session-current-{}", session.id),
            "session.current",
            "session",
            &session.id,
            &format!("session {} is {}", session.id, session.status),
            json!({
                "mode": session.mode,
                "status": session.status,
                "task_status": session.task_status
            }),
        ));
    }

    for approval in approval_views() {
        let id = approval
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let status = approval
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        events.push(timeline_event(
            &mut sequence,
            &format!("approval-current-{id}"),
            "approval.current",
            "approval",
            id,
            &format!("approval {id} is {status}"),
            json!({ "status": status }),
        ));
    }

    PackageResult::ok(Value::Array(events))
}

fn do_approval_decision(data: Value) -> PackageResult {
    let input: ApprovalDecisionInput =
        serde_json::from_value(data).unwrap_or(ApprovalDecisionInput {
            approval_id: String::new(),
            status: String::new(),
        });
    let status = if input.status.trim().is_empty() {
        "approved".to_string()
    } else {
        input.status.trim().to_string()
    };
    if let Some(mut approval) = load_approval_by_id(input.approval_id.trim()) {
        approval.status = status.clone();
        save_approval(&approval);
    }
    PackageResult::ok(json!({
        "id": input.approval_id,
        "status": status
    }))
}

fn do_execution_probe() -> PackageResult {
    PackageResult::ok(call_allowlisted_host_capability(
        HOST_EXECUTION_CAPABILITY,
        HOST_EXECUTION_RUN_ACTION,
    ))
}

fn do_create_natural_language_task(data: Value) -> PackageResult {
    let action_text = action_text_from_task_input(&data);
    let input: CreateTaskInput = match serde_json::from_value(data) {
        Ok(input) => input,
        Err(err) => return PackageResult::err(format!("parse error: {err}")),
    };

    let prompt = first_non_empty(&[&input.prompt, &input.natural_language_task]);
    if prompt.is_empty() {
        return PackageResult::err("missing prompt");
    }

    let id = if input.session_id.trim().is_empty() {
        next_session_id()
    } else {
        input.session_id.trim().to_string()
    };
    let title = if input.title.trim().is_empty() {
        derive_title(prompt)
    } else {
        input.title.trim().to_string()
    };

    if load_policy() == "read_only_mode"
        && is_mutating_natural_language_task(prompt, &action_text, &input.target_id)
    {
        let session = Session {
            id,
            title,
            prompt: prompt.to_string(),
            mode: "coding".to_string(),
            status: "blocked".to_string(),
            task_status: "blocked".to_string(),
            created_at_ms: u128::from(now_ms()),
        };

        save_session(&session);

        return PackageResult::ok(blocked_task_response(&session, prompt));
    }

    if load_policy() == "read_only_mode" {
        let session = Session {
            id,
            title,
            prompt: prompt.to_string(),
            mode: "coding".to_string(),
            status: "active".to_string(),
            task_status: "completed".to_string(),
            created_at_ms: u128::from(now_ms()),
        };

        save_session(&session);

        return PackageResult::ok(read_only_analysis_response(&session, prompt));
    }

    let approval_key = approval_key(&id, prompt, &input.target_id);
    let mut approval = load_approval(&approval_key).unwrap_or_else(|| ApprovalState {
        id: approval_id(&id, &approval_key),
        session_id: id.clone(),
        prompt: prompt.to_string(),
        target_id: input.target_id.trim().to_string(),
        status: "pending".to_string(),
    });

    let approved = approval.status == "approved";
    let task_status = if approved {
        "completed"
    } else {
        "waiting_approval"
    };
    let lifecycle_from = if approved {
        "waiting_approval"
    } else {
        "queued"
    };
    let lifecycle_transition = if approved {
        "completed"
    } else {
        "approval_pending"
    };
    let result = if approved {
        "Task approved and completed by weft-code-runtime WASM compatibility bridge."
    } else {
        "Task is waiting for approval before weft-code-runtime WASM compatibility bridge completion."
    };
    let next_steps = if approved {
        vec!["Continue with follow-up natural language tasks in this session."]
    } else {
        vec![
            "Approve the generated approval request via /api/weft-code/approval/{approval_id}/decision.",
            "Re-issue the same natural language request after approval in this slice.",
        ]
    };

    let session = Session {
        id,
        title,
        prompt: prompt.to_string(),
        mode: "coding".to_string(),
        status: if approved {
            "active"
        } else {
            "waiting_approval"
        }
        .to_string(),
        task_status: task_status.to_string(),
        created_at_ms: u128::from(now_ms()),
    };

    save_session(&session);
    save_approval(&approval);

    if approved {
        approval.status = "approved".to_string();
    }

    let mut response = json!({
        "session": session_view(&session),
        "task": task_view_with_status(&session, task_status),
        "related_tasks": [],
        "approval": approval_view(&approval),
        "execution_intent": execution_intent(
            !approved,
            if approved {
                "Approval was already granted, but this checkpoint only reports routing intent and performs no execution."
            } else {
                "Approval is pending, so this checkpoint reports routing intent and performs no execution."
            }
        ),
        "action_kind": "coding_task",
        "action": {
            "kind": "natural_language_task",
            "task_kind": "coding_task",
            "status": task_status
        },
        "lifecycle": {
            "state": task_status,
            "transition": lifecycle_transition,
            "record": {
                "from": lifecycle_from,
                "to": task_status,
                "reason": "weft-code-runtime wasm compatibility bridge"
            }
        },
        "interpretation": prompt,
        "result": result,
        "next_steps": next_steps,
        "created_team": Value::Null
    });

    if approved {
        response["execution"] = Value::Null;
        response["workflow_steps"] = json!([]);
        response["execution_record"] = dry_run_core_execution_record();
    }

    PackageResult::ok(response)
}

fn parse_optional_json(input: &str) -> Result<Value, extism_pdk::Error> {
    if input.trim().is_empty() {
        Ok(Value::Null)
    } else {
        parse_required_json(input)
    }
}

fn parse_required_json(input: &str) -> Result<Value, extism_pdk::Error> {
    serde_json::from_str(input).map_err(|err| extism_pdk::Error::msg(format!("parse error: {err}")))
}

fn empty_string_on_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn supported_actions() -> Vec<&'static str> {
    vec![
        "status",
        "list_sessions",
        "list_teams",
        "list_team_tasks",
        "update_session_mode",
        "get_policy",
        "update_policy",
        "list_approvals",
        "list_session_tasks",
        "list_events",
        "approval_decision",
        "execution_probe",
        "create_natural_language_task",
    ]
}

fn default_session(id: &str) -> Session {
    Session {
        id: id.to_string(),
        title: id.to_string(),
        prompt: String::new(),
        mode: "coding".to_string(),
        status: "active".to_string(),
        task_status: "queued".to_string(),
        created_at_ms: u128::from(now_ms()),
    }
}

fn session_view(session: &Session) -> Value {
    json!({
        "id": session.id,
        "mode": session.mode,
        "status": session.status,
    })
}

fn session_views() -> Vec<Value> {
    let mut sessions = load_sessions();
    if sessions.is_empty() {
        sessions.push(default_session("weft-code-local-session"));
    }
    sessions.iter().map(session_view).collect()
}

fn task_view(session: &Session) -> Value {
    task_view_with_status(session, session.task_status.trim())
}

fn task_view_with_status(session: &Session, status: &str) -> Value {
    json!({
        "id": format!("{}-task", session.id),
        "session_id": session.id,
        "kind": "coding_task",
        "status": status,
        "team_id": Value::Null,
        "parent_task_id": Value::Null,
    })
}

fn task_views(session_id: Option<&str>) -> Vec<Value> {
    load_sessions()
        .into_iter()
        .filter(|session| {
            session_id
                .filter(|id| !id.trim().is_empty())
                .map(|id| id == session.id)
                .unwrap_or(true)
        })
        .map(|session| task_view(&session))
        .collect()
}

fn team_views() -> Vec<Value> {
    vec![json!({
        "id": "weft-code-local-team",
        "session_id": "weft-code-local-session",
        "roles": ["operator"]
    })]
}

fn approval_views() -> Vec<Value> {
    let mut approvals: Vec<Value> = kv_list(APPROVAL_PREFIX)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|key| kv_get(&key))
        .filter_map(|json| serde_json::from_str::<ApprovalState>(&json).ok())
        .map(|approval| approval_view(&approval))
        .collect();
    if approvals.is_empty() {
        approvals.push(json!({
            "id": BOOTSTRAP_APPROVAL_ID,
            "status": "pending"
        }));
    }
    approvals
}

fn approval_view(approval: &ApprovalState) -> Value {
    json!({
        "id": approval.id,
        "status": approval.status
    })
}

fn timeline_event(
    sequence: &mut u64,
    id: &str,
    kind: &str,
    resource_type: &str,
    resource_id: &str,
    summary: &str,
    data: Value,
) -> Value {
    let event = json!({
        "id": id,
        "kind": kind,
        "source": PROVIDER,
        "resource": {
            "type": resource_type,
            "id": resource_id
        },
        "summary": summary,
        "sequence": *sequence,
        "data": data
    });
    *sequence += 1;
    event
}

fn policy_view() -> Value {
    json!({ "policy": load_policy() })
}

fn load_policy() -> String {
    kv_get(POLICY_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_POLICY.to_string())
}

fn first_non_empty<'a>(values: &[&'a str]) -> &'a str {
    values
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or("")
}

fn action_text_from_task_input(data: &Value) -> String {
    ["action", "action_kind", "task_kind"]
        .into_iter()
        .filter_map(|key| data.get(key))
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_mutating_natural_language_task(prompt: &str, action_text: &str, target_id: &str) -> bool {
    if !target_id.trim().is_empty() {
        return true;
    }

    text_implies_mutation(prompt) || text_implies_mutation(action_text)
}

fn text_implies_mutation(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["patch", "write", "note", "file"]
        .into_iter()
        .any(|needle| lower.contains(needle))
}

fn blocked_task_response(session: &Session, prompt: &str) -> Value {
    json!({
        "session": session_view(session),
        "task": task_view_with_status(session, "blocked"),
        "related_tasks": [],
        "approval": Value::Null,
        "execution_intent": execution_intent(
            true,
            "Read-only policy blocked mutating-looking work, so this checkpoint reports routing intent and performs no execution."
        ),
        "action_kind": "coding_task",
        "action": {
            "kind": "natural_language_task",
            "task_kind": "coding_task",
            "status": "blocked"
        },
        "lifecycle": {
            "state": "blocked",
            "transition": "policy_blocked",
            "record": {
                "from": "queued",
                "to": "blocked",
                "reason": "read_only_mode policy"
            }
        },
        "interpretation": prompt,
        "result": "Current approval policy is read_only_mode, so mutating-looking execution is blocked before approval or completion.",
        "next_steps": [
            "Switch policy to on_request or always_allow to enable sensitive execution.",
            "Re-issue the same natural language request after enabling sensitive execution."
        ],
        "created_team": Value::Null
    })
}

fn read_only_analysis_response(session: &Session, prompt: &str) -> Value {
    let execution_record = dry_run_core_execution_record();

    json!({
        "session": session_view(session),
        "task": task_view_with_status(session, "completed"),
        "related_tasks": [],
        "approval": Value::Null,
        "execution_intent": execution_intent(
            false,
            "Read-only analysis is complete; this checkpoint only reports routing intent and performs no execution."
        ),
        "action_kind": "coding_task",
        "action": {
            "kind": "natural_language_task",
            "task_kind": "coding_task",
            "status": "completed"
        },
        "lifecycle": {
            "state": "completed",
            "transition": "completed",
            "record": {
                "from": "queued",
                "to": "completed",
                "reason": "read_only_mode dry-run analysis preview"
            }
        },
        "interpretation": prompt,
        "result": "Read-only analysis completed without execution or mutation.",
        "next_steps": [],
        "created_team": Value::Null,
        "execution": Value::Null,
        "workflow_steps": [],
        "execution_record": execution_record,
        "tool_preview": {
            "capability": "core.execution",
            "operation": "analyze_repo_state",
            "dry_run": true,
            "would_execute": false,
            "would_mutate": false
        }
    })
}

fn describe_core_execution_record() -> Value {
    call_allowlisted_host_capability(HOST_EXECUTION_CAPABILITY, HOST_EXECUTION_DESCRIBE_ACTION)
}

const _: fn() -> Value = describe_core_execution_record;

fn dry_run_core_execution_record() -> Value {
    call_allowlisted_host_capability(HOST_EXECUTION_CAPABILITY, HOST_EXECUTION_RUN_ACTION)
}

fn call_allowlisted_host_capability(capability: &str, action: &str) -> Value {
    if capability != HOST_EXECUTION_CAPABILITY
        || !matches!(
            action,
            HOST_EXECUTION_DESCRIBE_ACTION | HOST_EXECUTION_RUN_ACTION
        )
    {
        return json!({
            "status": "host_capability_rejected",
            "engine": "host_mediated",
            "capability": capability,
            "action": action,
            "would_execute": false,
            "would_mutate": false,
            "error": "capability/action is not allowlisted"
        });
    }

    let request = if action == HOST_EXECUTION_RUN_ACTION {
        json!({
            "capability": HOST_EXECUTION_CAPABILITY,
            "action": HOST_EXECUTION_RUN_ACTION,
            "data": {
                "mode": "dry_run",
                "command": "weft-core-version"
            },
            "provider": "core",
            "app": "weft-claw"
        })
    } else {
        json!({
            "capability": HOST_EXECUTION_CAPABILITY,
            "action": HOST_EXECUTION_DESCRIBE_ACTION,
            "data": {}
        })
    };
    let raw_response = host_capability_call_once(request.to_string());
    let response = serde_json::from_str::<Value>(&raw_response).unwrap_or_else(|err| {
        json!({
            "error": format!("host capability response parse error: {err}"),
            "raw": raw_response
        })
    });

    json!({
        "status": "host_capability_called",
        "engine": "host_mediated",
        "capability": HOST_EXECUTION_CAPABILITY,
        "action": action,
        "would_execute": false,
        "would_mutate": false,
        "request": request,
        "response": response
    })
}

#[cfg(not(test))]
fn host_capability_call_once(request: String) -> String {
    unsafe { host_capability_call(request) }.unwrap_or_else(|err| {
        json!({
            "error": format!("host_capability_call failed: {err}")
        })
        .to_string()
    })
}

#[cfg(test)]
fn host_capability_call_once(request: String) -> String {
    let request = serde_json::from_str::<Value>(&request).unwrap_or(Value::Null);
    if request.get("action").and_then(Value::as_str) == Some(HOST_EXECUTION_RUN_ACTION) {
        json!({
            "capability": "core.execution",
            "provider": "core",
            "status": "executed",
            "mode": "core",
            "response": {
                "mode": "dry_run",
                "command": "weft-core-version",
                "dry_run": true,
                "would_execute": false,
                "exit_code": 0,
                "stdout": "weft-core-version",
                "stderr": ""
            }
        })
        .to_string()
    } else {
        json!({
            "capability": "core.execution",
            "actions": ["describe", "health", "run"],
            "runtime": "core"
        })
        .to_string()
    }
}

fn execution_intent(requires_approval: bool, reason: &str) -> Value {
    json!({
        "status": "preview_only",
        "engine": "weft_execution_engine",
        "capability": "tool.runtime",
        "requested_capabilities": ["tool.shell", "tool.files", "tool.git"],
        "mutation_allowed": false,
        "requires_approval": requires_approval,
        "reason": reason,
        "steps": [{
            "id": "analyze",
            "kind": "analyze",
            "capability": "tool.runtime",
            "mutation_allowed": false
        }]
    })
}

fn derive_title(prompt: &str) -> String {
    let title: String = prompt.trim().chars().take(48).collect();
    if title.is_empty() {
        "Untitled task".to_string()
    } else {
        title
    }
}

fn next_session_id() -> String {
    let next = kv_get(NEXT_ID_KEY)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    kv_set(NEXT_ID_KEY, &(next + 1).to_string());
    format!("task-{next:06}")
}

fn load_sessions() -> Vec<Session> {
    let mut sessions: Vec<Session> = kv_list(SESSION_PREFIX)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|key| kv_get(&key))
        .filter_map(|json| serde_json::from_str::<Session>(&json).ok())
        .collect();

    for session in &mut sessions {
        if session.mode.trim().is_empty() {
            session.mode = "coding".to_string();
        }
        if session.task_status.trim().is_empty() {
            session.task_status = "queued".to_string();
        }
    }

    sessions.sort_by(|left, right| {
        left.created_at_ms
            .cmp(&right.created_at_ms)
            .then(left.id.cmp(&right.id))
    });
    sessions
}

fn save_session(session: &Session) {
    let key = format!("{SESSION_PREFIX}{}", session.id);
    let json = serde_json::to_string(session).unwrap_or_else(|_| "{}".to_string());
    kv_set(&key, &json);
}

fn approval_key(session_id: &str, prompt: &str, target_id: &str) -> String {
    let mut hasher = DefaultHasher::new();
    session_id.trim().hash(&mut hasher);
    prompt.trim().hash(&mut hasher);
    target_id.trim().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn approval_id(session_id: &str, approval_key: &str) -> String {
    format!("{}-approval-{}", session_id.trim(), approval_key)
}

fn load_approval(approval_key: &str) -> Option<ApprovalState> {
    kv_get(&format!("{APPROVAL_PREFIX}{approval_key}"))
        .and_then(|json| serde_json::from_str::<ApprovalState>(&json).ok())
}

fn load_approval_by_id(approval_id: &str) -> Option<ApprovalState> {
    kv_list(APPROVAL_PREFIX)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|key| kv_get(&key))
        .filter_map(|json| serde_json::from_str::<ApprovalState>(&json).ok())
        .find(|approval| approval.id == approval_id)
}

fn save_approval(approval: &ApprovalState) {
    let key = format!(
        "{APPROVAL_PREFIX}{}",
        approval_key(&approval.session_id, &approval.prompt, &approval.target_id)
    );
    let json = serde_json::to_string(approval).unwrap_or_else(|_| "{}".to_string());
    kv_set(&key, &json);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_task_detection_blocks_target_id_and_mutation_words() {
        assert!(is_mutating_natural_language_task(
            "Analyze the runtime",
            "",
            "patch_target"
        ));
        assert!(is_mutating_natural_language_task(
            "Please patch the runtime",
            "",
            ""
        ));
        assert!(is_mutating_natural_language_task(
            "Summarize the runtime",
            "write_note",
            ""
        ));
    }

    #[test]
    fn mutating_task_detection_allows_read_only_analyze_prompt() {
        assert!(!is_mutating_natural_language_task(
            "Analyze the runtime policy behavior",
            "analyze",
            ""
        ));
    }

    #[test]
    fn read_only_blocked_response_uses_policy_blocked_lifecycle_without_approval() {
        let session = Session {
            id: "task-000001".to_string(),
            title: "Patch runtime".to_string(),
            prompt: "Patch runtime".to_string(),
            mode: "coding".to_string(),
            status: "blocked".to_string(),
            task_status: "blocked".to_string(),
            created_at_ms: 1,
        };

        let response = blocked_task_response(&session, "Patch runtime");

        assert_eq!(response["task"]["status"], "blocked");
        assert!(response["approval"].is_null());
        assert_eq!(response["lifecycle"]["state"], "blocked");
        assert_eq!(response["lifecycle"]["transition"], "policy_blocked");
        assert_eq!(response["execution_intent"]["status"], "preview_only");
        assert_eq!(
            response["execution_intent"]["engine"],
            "weft_execution_engine"
        );
        assert_eq!(response["execution_intent"]["capability"], "tool.runtime");
        assert_eq!(
            response["execution_intent"]["requested_capabilities"],
            json!(["tool.shell", "tool.files", "tool.git"])
        );
        assert_eq!(response["execution_intent"]["mutation_allowed"], false);
        assert_eq!(response["execution_intent"]["requires_approval"], true);
        assert!(response["execution_intent"]["reason"]
            .as_str()
            .unwrap()
            .contains("no execution"));
        assert_eq!(response["execution_intent"]["steps"][0]["id"], "analyze");
        assert_eq!(response["execution_intent"]["steps"][0]["kind"], "analyze");
        assert_eq!(
            response["execution_intent"]["steps"][0]["capability"],
            "tool.runtime"
        );
        assert_eq!(
            response["execution_intent"]["steps"][0]["mutation_allowed"],
            false
        );
        assert!(response["result"]
            .as_str()
            .unwrap()
            .contains("read_only_mode"));
        assert!(response["next_steps"][0]
            .as_str()
            .unwrap()
            .contains("enable sensitive execution"));
    }

    #[test]
    fn read_only_analysis_response_includes_dry_run_preview_without_execution() {
        let session = Session {
            id: "task-000002".to_string(),
            title: "Analyze runtime".to_string(),
            prompt: "Analyze runtime".to_string(),
            mode: "coding".to_string(),
            status: "active".to_string(),
            task_status: "completed".to_string(),
            created_at_ms: 1,
        };

        let response = read_only_analysis_response(&session, "Analyze runtime");

        assert_eq!(response["task"]["status"], "completed");
        assert!(response["approval"].is_null());
        assert_eq!(response["execution_intent"]["status"], "preview_only");
        assert_eq!(
            response["execution_intent"]["engine"],
            "weft_execution_engine"
        );
        assert_eq!(response["execution_intent"]["capability"], "tool.runtime");
        assert_eq!(
            response["execution_intent"]["requested_capabilities"],
            json!(["tool.shell", "tool.files", "tool.git"])
        );
        assert_eq!(response["execution_intent"]["mutation_allowed"], false);
        assert_eq!(response["execution_intent"]["requires_approval"], false);
        assert!(response["execution_intent"]["reason"]
            .as_str()
            .unwrap()
            .contains("no execution"));
        assert_eq!(response["execution_intent"]["steps"][0]["id"], "analyze");
        assert_eq!(response["execution_intent"]["steps"][0]["kind"], "analyze");
        assert_eq!(
            response["execution_intent"]["steps"][0]["capability"],
            "tool.runtime"
        );
        assert_eq!(
            response["execution_intent"]["steps"][0]["mutation_allowed"],
            false
        );
        assert!(response["execution"].is_null());
        assert_eq!(response["workflow_steps"], json!([]));
        assert_eq!(
            response["execution_record"]["status"],
            "host_capability_called"
        );
        assert_eq!(response["execution_record"]["engine"], "host_mediated");
        assert_eq!(response["execution_record"]["capability"], "core.execution");
        assert_eq!(response["execution_record"]["action"], "run");
        assert_eq!(response["execution_record"]["would_execute"], false);
        assert_eq!(response["execution_record"]["would_mutate"], false);
        assert_eq!(
            response["execution_record"]["request"],
            json!({
                "capability": "core.execution",
                "action": "run",
                "data": {
                    "mode": "dry_run",
                    "command": "weft-core-version"
                },
                "provider": "core",
                "app": "weft-claw"
            })
        );
        assert_eq!(response["execution_record"]["response"]["capability"], "core.execution");
        assert_eq!(response["execution_record"]["response"]["provider"], "core");
        assert_eq!(response["execution_record"]["response"]["status"], "executed");
        assert_eq!(response["execution_record"]["response"]["mode"], "core");
        assert_eq!(
            response["execution_record"]["response"]["response"]["mode"],
            "dry_run"
        );
        assert_eq!(
            response["execution_record"]["response"]["response"]["command"],
            "weft-core-version"
        );
        assert_eq!(
            response["execution_record"]["response"]["response"]["dry_run"],
            true
        );
        assert_eq!(
            response["execution_record"]["response"]["response"]["would_execute"],
            false
        );
        assert_eq!(response["tool_preview"]["capability"], "core.execution");
        assert_eq!(response["tool_preview"]["operation"], "analyze_repo_state");
        assert_eq!(response["tool_preview"]["dry_run"], true);
        assert_eq!(response["tool_preview"]["would_execute"], false);
        assert_eq!(response["tool_preview"]["would_mutate"], false);
    }

    #[test]
    fn execution_probe_calls_host_core_execution_run_dry_run_only() {
        let result = dispatch_action("execution_probe", Value::Null);
        assert_eq!(result.status, "ok");
        let response = result.data.expect("execution probe returns data");

        assert_eq!(response["status"], "host_capability_called");
        assert_eq!(response["engine"], "host_mediated");
        assert_eq!(response["capability"], "core.execution");
        assert_eq!(response["action"], "run");
        assert_eq!(response["would_execute"], false);
        assert_eq!(response["would_mutate"], false);
        assert_eq!(
            response["request"],
            json!({
                "capability": "core.execution",
                "action": "run",
                "data": {
                    "mode": "dry_run",
                    "command": "weft-core-version"
                },
                "provider": "core",
                "app": "weft-claw"
            })
        );
        assert_eq!(response["response"]["capability"], "core.execution");
        assert_eq!(response["response"]["provider"], "core");
        assert_eq!(response["response"]["status"], "executed");
        assert_eq!(response["response"]["mode"], "core");
        assert_eq!(response["response"]["response"]["mode"], "dry_run");
        assert_eq!(
            response["response"]["response"]["command"],
            "weft-core-version"
        );
        assert_eq!(response["response"]["response"]["dry_run"], true);
        assert_eq!(response["response"]["response"]["would_execute"], false);
        assert_eq!(response["response"]["response"]["exit_code"], 0);
        assert_eq!(
            response["response"]["response"]["stdout"],
            "weft-core-version"
        );
        assert_eq!(response["response"]["response"]["stderr"], "");
    }

    #[test]
    fn approved_retry_response_includes_fixed_dry_run_execution_record() {
        let session_id = "unit-approved-retry";
        let prompt = "Ship approved checkpoint";
        let target_id = "target-approved-retry";
        let key = approval_key(session_id, prompt, target_id);
        save_approval(&ApprovalState {
            id: approval_id(session_id, &key),
            session_id: session_id.to_string(),
            prompt: prompt.to_string(),
            target_id: target_id.to_string(),
            status: "approved".to_string(),
        });

        let result = do_create_natural_language_task(json!({
            "session_id": session_id,
            "prompt": prompt,
            "natural_language_task": prompt,
            "target_id": target_id
        }));

        assert_eq!(result.status, "ok");
        let response = result.data.expect("approved retry returns data");
        assert_eq!(response["task"]["status"], "completed");
        assert_eq!(response["approval"]["status"], "approved");
        assert!(response["execution"].is_null());
        assert_eq!(response["workflow_steps"], json!([]));
        assert_eq!(response["execution_record"]["action"], "run");
        assert_eq!(
            response["execution_record"]["request"]["data"],
            json!({
                "mode": "dry_run",
                "command": "weft-core-version"
            })
        );
        assert_eq!(
            response["execution_record"]["response"]["response"]["dry_run"],
            true
        );
    }

    #[test]
    fn list_events_returns_read_only_timeline_snapshot() {
        let session = Session {
            id: "unit-events-session".to_string(),
            title: "Timeline session".to_string(),
            prompt: "Timeline session".to_string(),
            mode: "coding".to_string(),
            status: "active".to_string(),
            task_status: "completed".to_string(),
            created_at_ms: 1,
        };
        save_session(&session);
        save_approval(&ApprovalState {
            id: "unit-events-approval".to_string(),
            session_id: session.id.clone(),
            prompt: session.prompt.clone(),
            target_id: "target-events".to_string(),
            status: "approved".to_string(),
        });

        let result = do_list_events();
        assert_eq!(result.status, "ok");
        let events = result
            .data
            .expect("events data")
            .as_array()
            .expect("events are an array")
            .clone();

        assert_eq!(events[0]["id"], "runtime-bootstrap");
        assert_eq!(events[0]["kind"], "runtime.bootstrap");
        assert_eq!(events[0]["sequence"], 0);
        assert_eq!(events[0]["resource"]["type"], "runtime");
        assert_eq!(events[0]["resource"]["id"], PROVIDER);
        assert_eq!(events[1]["id"], "policy-current");
        assert_eq!(events[1]["kind"], "policy.current");
        assert_eq!(events[1]["sequence"], 1);
        assert_eq!(events[1]["data"]["policy"], DEFAULT_POLICY);
        assert!(events.iter().any(|event| {
            event["kind"] == "session.current"
                && event["resource"]["id"] == "unit-events-session"
                && event["data"]["task_status"] == "completed"
        }));
        assert!(events.iter().any(|event| {
            event["kind"] == "approval.current"
                && event["resource"]["id"] == "unit-events-approval"
                && event["data"]["status"] == "approved"
        }));
    }
}

