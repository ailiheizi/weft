use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    Coding,
    Plan,
    Team,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    AlwaysAllow,
    #[default]
    OnRequest,
    OnSensitiveActions,
    ReadOnlyMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    Active,
    Compacting,
    Paused,
    Resumed,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Blocked,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub root: String,
    pub product: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub project_id: String,
    pub mode: SessionMode,
    pub status: SessionStatus,
    pub parent_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionModeUpdateRequest {
    pub mode: SessionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPolicyInfo {
    pub policy: ApprovalPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPolicyUpdateRequest {
    pub policy: ApprovalPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodingActionKind {
    Analyze,
    Patch,
    WriteNote,
    TeamKickoff,
    Plan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionParentUpdateRequest {
    pub parent_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub status: TaskStatus,
    pub team_id: Option<String>,
    pub parent_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInfo {
    pub id: String,
    pub session_id: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTaskInfo {
    pub id: String,
    pub team_id: String,
    pub role: String,
    pub phase: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamCreatedResponse {
    pub team: TeamInfo,
    pub event: crate::events::RuntimeEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequestInfo {
    pub id: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_kind: Option<CodingActionKind>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecisionRequest {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusUpdateRequest {
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalLanguageTaskRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_kind: Option<CodingActionKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalLanguageTaskResponse {
    pub session: SessionInfo,
    pub task: TaskInfo,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_tasks: Vec<TaskInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<ApprovalRequestInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<TaskExecutionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workflow_steps: Vec<TaskExecutionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_mutation: Option<FileMutationInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_record: Option<PatchRecordInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_kind: Option<CodingActionKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<NaturalLanguageActionInfo>,
    pub lifecycle: NaturalLanguageLifecycleInfo,
    pub interpretation: String,
    pub result: String,
    pub next_steps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_team: Option<TeamInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalLanguageActionInfo {
    pub kind: CodingActionKind,
    pub task_kind: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalLanguageLifecycleInfo {
    pub state: TaskStatus,
    pub transition: RuntimeLifecycleTransition,
    pub record: RuntimeLifecycleTransitionRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeLifecycleTransitionRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<TaskStatus>,
    pub to: TaskStatus,
    pub reason: RuntimeLifecycleTransition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLifecycleTransition {
    Accepted,
    InProgress,
    ApprovalPending,
    PolicyBlocked,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionInfo {
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMutationInfo {
    pub path: String,
    pub action: String,
    pub bytes_written: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchRecordInfo {
    pub patch_id: String,
    pub session_id: String,
    pub task_id: String,
    pub target_id: String,
    pub target_path: String,
    pub line_count: usize,
}
