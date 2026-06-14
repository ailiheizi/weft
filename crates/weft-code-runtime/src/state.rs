use std::sync::Arc;

use tokio::sync::RwLock;

use super::types::{
    ApprovalPolicy, ApprovalRequestInfo, ProjectInfo, SessionInfo, SessionMode, SessionStatus,
    TaskInfo, TaskStatus, TeamInfo, TeamTaskInfo,
};

#[derive(Default)]
pub struct WeftCodeState {
    pub projects: Vec<ProjectInfo>,
    pub sessions: Vec<SessionInfo>,
    pub tasks: Vec<TaskInfo>,
    pub teams: Vec<TeamInfo>,
    pub team_tasks: Vec<TeamTaskInfo>,
    pub approvals: Vec<ApprovalRequestInfo>,
    pub approval_policy: ApprovalPolicy,
    pub runtime_note_namespace: Option<String>,
    pub controlled_patch_target: Option<String>,
    pub allowed_patch_targets: Vec<String>,
}

pub type SharedWeftCodeState = Arc<RwLock<WeftCodeState>>;

pub fn shared_state_with_defaults() -> SharedWeftCodeState {
    shared_state_with_defaults_with_namespace(None)
}

pub fn shared_state_with_defaults_with_namespace(
    runtime_note_namespace: Option<String>,
) -> SharedWeftCodeState {
    shared_state_with_defaults_custom(
        runtime_note_namespace,
        Some("crates/weft-code-runtime/src/patch_target.rs".to_string()),
        vec![
            "crates/weft-code-runtime/src/patch_target.rs".to_string(),
            "crates/weft-code-runtime/src/policy_service.rs".to_string(),
        ],
    )
}

pub fn shared_state_with_defaults_custom(
    runtime_note_namespace: Option<String>,
    controlled_patch_target: Option<String>,
    allowed_patch_targets: Vec<String>,
) -> SharedWeftCodeState {
    shared_state_with_defaults_custom_and_policy(
        runtime_note_namespace,
        controlled_patch_target,
        allowed_patch_targets,
        ApprovalPolicy::OnRequest,
    )
}

pub fn shared_state_with_defaults_custom_and_policy(
    runtime_note_namespace: Option<String>,
    controlled_patch_target: Option<String>,
    allowed_patch_targets: Vec<String>,
    approval_policy: ApprovalPolicy,
) -> SharedWeftCodeState {
    Arc::new(RwLock::new(WeftCodeState {
        projects: vec![ProjectInfo {
            id: "weft-code-local-project".into(),
            root: ".".into(),
            product: "weft-code".into(),
        }],
        sessions: vec![SessionInfo {
            id: "weft-code-local-session".into(),
            project_id: "weft-code-local-project".into(),
            mode: SessionMode::Coding,
            status: SessionStatus::Active,
            parent_session_id: None,
        }],
        tasks: vec![TaskInfo {
            id: "weft-code-bootstrap-task".into(),
            session_id: "weft-code-local-session".into(),
            kind: "bootstrap".into(),
            status: TaskStatus::Queued,
            team_id: None,
            parent_task_id: None,
        }],
        teams: Vec::new(),
        team_tasks: Vec::new(),
        approvals: vec![ApprovalRequestInfo {
            id: "weft-code-bootstrap-approval".into(),
            session_id: "weft-code-local-session".into(),
            action_kind: None,
            status: "pending".into(),
        }],
        approval_policy,
        runtime_note_namespace,
        controlled_patch_target,
        allowed_patch_targets,
    }))
}
