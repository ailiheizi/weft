use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const PACKAGE_NAME: &str = "workflow-template-devteam";
const CAPABILITY_NAME: &str = "workflow.template.devteam";
const TEMPLATE_ID: &str = "devteam-v1";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PhaseDefinition {
    id: String,
    title: String,
    order: u32,
    terminal: bool,
    default_role: String,
    allowed_roles: Vec<String>,
    entry_statuses: Vec<String>,
    exit_statuses: Vec<String>,
    next: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WorkflowTemplate {
    schema_version: u32,
    template_id: String,
    capability: String,
    title: String,
    phases: Vec<PhaseDefinition>,
    default_start_phase: String,
    done_phase: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GetPhaseInput {
    #[serde(default)]
    phase_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct VerifyTransitionInput {
    #[serde(default)]
    from_phase: String,
    #[serde(default)]
    to_phase: String,
    #[serde(default)]
    role_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ResolveTemplateInput {
    #[serde(default)]
    workflow_id: String,
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    goal: String,
    #[serde(default)]
    requested_phase: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ActivateTemplateInput {
    #[serde(default)]
    workflow_id: String,
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    phase_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RunTemplateInput {
    #[serde(default)]
    workflow_id: String,
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    phase_id: String,
    #[serde(default)]
    role_id: String,
}

fn phase(
    id: &str,
    title: &str,
    order: u32,
    terminal: bool,
    default_role: &str,
    allowed_roles: &[&str],
    entry_statuses: &[&str],
    exit_statuses: &[&str],
    next: &[&str],
) -> PhaseDefinition {
    PhaseDefinition {
        id: id.to_string(),
        title: title.to_string(),
        order,
        terminal,
        default_role: default_role.to_string(),
        allowed_roles: allowed_roles
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        entry_statuses: entry_statuses
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        exit_statuses: exit_statuses
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        next: next.iter().map(|value| (*value).to_string()).collect(),
    }
}

fn template() -> WorkflowTemplate {
    WorkflowTemplate {
        schema_version: SCHEMA_VERSION,
        template_id: TEMPLATE_ID.to_string(),
        capability: CAPABILITY_NAME.to_string(),
        title: "Weft Claw DevTeam Default Workflow".to_string(),
        phases: vec![
            phase(
                "intake",
                "Intake",
                0,
                false,
                "planner",
                &["planner", "integrator"],
                &["new", "captured"],
                &["ready_for_plan"],
                &["plan"],
            ),
            phase(
                "plan",
                "Plan",
                1,
                false,
                "planner",
                &["planner"],
                &["planning", "scope_confirmed"],
                &["ready_for_execute"],
                &["execute"],
            ),
            phase(
                "execute",
                "Execute",
                2,
                false,
                "implementer",
                &["implementer"],
                &["in_progress", "blocked"],
                &["review_requested"],
                &["review"],
            ),
            phase(
                "review",
                "Review",
                3,
                false,
                "reviewer",
                &["reviewer", "planner"],
                &["in_review"],
                &["review_completed", "changes_requested"],
                &["execute", "integrate"],
            ),
            phase(
                "integrate",
                "Integrate",
                4,
                false,
                "integrator",
                &["integrator", "implementer"],
                &["ready_to_integrate"],
                &["integrated"],
                &["done"],
            ),
            phase(
                "done",
                "Done",
                5,
                true,
                "integrator",
                &["integrator", "planner"],
                &["completed"],
                &["archived"],
                &[],
            ),
        ],
        default_start_phase: "intake".to_string(),
        done_phase: "done".to_string(),
    }
}

fn find_phase<'a>(workflow: &'a WorkflowTemplate, phase_id: &str) -> Option<&'a PhaseDefinition> {
    workflow
        .phases
        .iter()
        .find(|phase| phase.id == phase_id.trim())
}

fn ensure_non_empty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("missing {}", field))
    } else {
        Ok(())
    }
}

fn default_phase_id(value: &str) -> String {
    if value.trim().is_empty() {
        "intake".to_string()
    } else {
        value.trim().to_string()
    }
}

fn build_plan_steps(workflow: &WorkflowTemplate, workflow_id: &str, board_id: &str) -> Vec<Value> {
    workflow
        .phases
        .iter()
        .map(|phase| {
            json!({
                "workflow_id": workflow_id,
                "board_id": board_id,
                "phase_id": phase.id,
                "title": phase.title,
                "order": phase.order,
                "default_role": phase.default_role,
                "next": phase.next,
            })
        })
        .collect()
}

fn describe_result() -> PackageResult {
    let workflow = template();
    PackageResult::ok(json!({
        "package": PACKAGE_NAME,
        "capability": CAPABILITY_NAME,
        "runtime": "wasm",
        "template": workflow,
        "actions": [
            "describe",
            "health",
            "list_phases",
            "get_phase",
            "resolve",
            "verify_transition",
            "activate",
            "run"
        ]
    }))
}

fn list_phases_result() -> PackageResult {
    let workflow = template();
    PackageResult::ok(json!({
        "template_id": workflow.template_id,
        "phases": workflow.phases,
    }))
}

fn get_phase_result(input: GetPhaseInput) -> PackageResult {
    let workflow = template();
    let phase_id = default_phase_id(&input.phase_id);
    match find_phase(&workflow, &phase_id) {
        Some(phase) => PackageResult::ok(json!({
            "template_id": workflow.template_id,
            "phase": phase,
        })),
        None => PackageResult::err(format!("unknown phase '{}'", phase_id)),
    }
}

fn verify_transition_result(input: VerifyTransitionInput) -> PackageResult {
    let workflow = template();
    if let Err(error) = ensure_non_empty(&input.from_phase, "from_phase") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.to_phase, "to_phase") {
        return PackageResult::err(error);
    }

    let from_phase = match find_phase(&workflow, &input.from_phase) {
        Some(phase) => phase,
        None => {
            return PackageResult::err(format!("unknown from_phase '{}'", input.from_phase.trim()))
        }
    };
    let to_phase = match find_phase(&workflow, &input.to_phase) {
        Some(phase) => phase,
        None => return PackageResult::err(format!("unknown to_phase '{}'", input.to_phase.trim())),
    };

    let role_id = input.role_id.trim();
    let allowed_edge = from_phase
        .next
        .iter()
        .any(|next| next == to_phase.id.as_str());
    let role_allowed =
        role_id.is_empty() || from_phase.allowed_roles.iter().any(|role| role == role_id);

    PackageResult::ok(json!({
        "template_id": workflow.template_id,
        "from_phase": from_phase.id,
        "to_phase": to_phase.id,
        "allowed": allowed_edge && role_allowed,
        "role_id": role_id,
        "reasons": {
            "transition_allowed": allowed_edge,
            "role_allowed": role_allowed,
        }
    }))
}

fn resolve_result(input: ResolveTemplateInput) -> PackageResult {
    let workflow = template();
    let workflow_id = if input.workflow_id.trim().is_empty() {
        format!("{}:{}", TEMPLATE_ID, input.board_id.trim())
    } else {
        input.workflow_id.trim().to_string()
    };
    let active_phase = default_phase_id(&input.requested_phase);
    let Some(active_phase_def) = find_phase(&workflow, &active_phase) else {
        return PackageResult::err(format!("unknown requested_phase '{}'", active_phase));
    };

    PackageResult::ok(json!({
        "template_id": workflow.template_id,
        "workflow_id": workflow_id,
        "board_id": input.board_id.trim(),
        "session_id": input.session_id.trim(),
        "goal": input.goal,
        "active_phase": active_phase_def.id,
        "resolve": {
            "provider": PACKAGE_NAME,
            "capability": CAPABILITY_NAME,
            "default_start_phase": workflow.default_start_phase,
            "done_phase": workflow.done_phase,
        },
        "plan_steps": build_plan_steps(&workflow, &workflow_id, input.board_id.trim()),
    }))
}

fn activate_result(input: ActivateTemplateInput) -> PackageResult {
    let workflow = template();
    let phase_id = default_phase_id(&input.phase_id);
    match find_phase(&workflow, &phase_id) {
        Some(phase) => PackageResult::ok(json!({
            "template_id": workflow.template_id,
            "workflow_id": input.workflow_id.trim(),
            "board_id": input.board_id.trim(),
            "session_id": input.session_id.trim(),
            "activated": true,
            "phase": phase,
        })),
        None => PackageResult::err(format!("unknown phase '{}'", phase_id)),
    }
}

fn run_result(input: RunTemplateInput) -> PackageResult {
    let workflow = template();
    let phase_id = default_phase_id(&input.phase_id);
    let Some(phase) = find_phase(&workflow, &phase_id) else {
        return PackageResult::err(format!("unknown phase '{}'", phase_id));
    };

    let role_id = if input.role_id.trim().is_empty() {
        phase.default_role.clone()
    } else {
        input.role_id.trim().to_string()
    };
    let role_allowed = phase.allowed_roles.iter().any(|role| role == &role_id);

    PackageResult::ok(json!({
        "template_id": workflow.template_id,
        "workflow_id": input.workflow_id.trim(),
        "board_id": input.board_id.trim(),
        "session_id": input.session_id.trim(),
        "phase_id": phase.id,
        "role_id": role_id,
        "accepted": role_allowed,
        "phase_contract": {
            "entry_statuses": phase.entry_statuses,
            "exit_statuses": phase.exit_statuses,
            "next": phase.next,
            "terminal": phase.terminal,
        }
    }))
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("workflow-template-devteam initialized");
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
            "capability": CAPABILITY_NAME,
            "template_id": TEMPLATE_ID,
        })),
        "list_phases" => list_phases_result(),
        "get_phase" => {
            let payload: GetPhaseInput =
                serde_json::from_value(req.data).unwrap_or(GetPhaseInput {
                    phase_id: String::new(),
                });
            get_phase_result(payload)
        }
        "verify_transition" => {
            let payload: VerifyTransitionInput =
                serde_json::from_value(req.data).unwrap_or(VerifyTransitionInput {
                    from_phase: String::new(),
                    to_phase: String::new(),
                    role_id: String::new(),
                });
            verify_transition_result(payload)
        }
        "resolve" | "call" => {
            let payload: ResolveTemplateInput =
                serde_json::from_value(req.data).unwrap_or(ResolveTemplateInput {
                    workflow_id: String::new(),
                    board_id: String::new(),
                    session_id: String::new(),
                    goal: String::new(),
                    requested_phase: String::new(),
                });
            resolve_result(payload)
        }
        "activate" => {
            let payload: ActivateTemplateInput =
                serde_json::from_value(req.data).unwrap_or(ActivateTemplateInput {
                    workflow_id: String::new(),
                    board_id: String::new(),
                    session_id: String::new(),
                    phase_id: String::new(),
                });
            activate_result(payload)
        }
        "run" => {
            let payload: RunTemplateInput =
                serde_json::from_value(req.data).unwrap_or(RunTemplateInput {
                    workflow_id: String::new(),
                    board_id: String::new(),
                    session_id: String::new(),
                    phase_id: String::new(),
                    role_id: String::new(),
                });
            run_result(payload)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

