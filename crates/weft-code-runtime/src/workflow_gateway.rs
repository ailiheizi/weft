use crate::state::SharedWeftCodeState;
use crate::types::{
    ApprovalPolicy, ApprovalRequestInfo, CodingActionKind, FileMutationInfo,
    NaturalLanguageActionInfo, NaturalLanguageLifecycleInfo, NaturalLanguageTaskResponse,
    PatchRecordInfo, RuntimeLifecycleTransition, RuntimeLifecycleTransitionRecord, SessionInfo,
    SessionMode, SessionStatus, TaskExecutionInfo, TaskInfo, TaskStatus, TeamInfo, TeamTaskInfo,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NaturalLanguageTaskError {
    EmptyPrompt,
    SessionNotFound,
}

#[derive(Clone)]
pub struct WorkflowGateway {
    pub state: SharedWeftCodeState,
}

fn run_command(command: &str, program: &str, args: &[&str]) -> TaskExecutionInfo {
    let output = std::process::Command::new(program).args(args).output();

    match output {
        Ok(output) => TaskExecutionInfo {
            command: command.to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Err(error) => TaskExecutionInfo {
            command: command.to_string(),
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("failed to run repository check: {}", error),
        },
    }
}

fn analyze_repo_state() -> TaskExecutionInfo {
    TaskExecutionInfo {
        command: "analyze:repo-state".to_string(),
        exit_code: 0,
        stdout: "Detected package-first runtime workspace with weft-code product package and weft-code-runtime target.".to_string(),
        stderr: String::new(),
    }
}

fn request_approval_key(prompt: &str, target_id: Option<&str>) -> String {
    let mut hash: u64 = 14695981039346656037;
    let serialized = format!(
        "prompt_len:{}\nprompt:{}\ntarget_len:{}\ntarget:{}",
        prompt.len(),
        prompt,
        target_id.unwrap_or("").len(),
        target_id.unwrap_or("")
    );
    for byte in serialized.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{:016x}", hash)
}

fn approval_request_for_task(
    task: &TaskInfo,
    request_key: &str,
    coding_action_kind: &CodingActionKind,
) -> ApprovalRequestInfo {
    ApprovalRequestInfo {
        id: format!("{}-approval-{}", task.session_id, request_key),
        session_id: task.session_id.clone(),
        action_kind: Some(coding_action_kind.clone()),
        status: "pending".to_string(),
    }
}

fn action_summary(kind: CodingActionKind, task: &TaskInfo) -> NaturalLanguageActionInfo {
    NaturalLanguageActionInfo {
        kind,
        task_kind: task.kind.clone(),
        status: task.status.clone(),
    }
}

fn lifecycle_summary_with_from(
    task: &TaskInfo,
    from: Option<TaskStatus>,
) -> NaturalLanguageLifecycleInfo {
    let transition = match task.status {
        TaskStatus::Queued => RuntimeLifecycleTransition::Accepted,
        TaskStatus::Running => RuntimeLifecycleTransition::InProgress,
        TaskStatus::WaitingApproval => RuntimeLifecycleTransition::ApprovalPending,
        TaskStatus::Blocked => RuntimeLifecycleTransition::PolicyBlocked,
        TaskStatus::Completed => RuntimeLifecycleTransition::Completed,
        TaskStatus::Failed => RuntimeLifecycleTransition::Failed,
        TaskStatus::Cancelled => RuntimeLifecycleTransition::Cancelled,
    };

    NaturalLanguageLifecycleInfo {
        state: task.status.clone(),
        transition: transition.clone(),
        record: RuntimeLifecycleTransitionRecord {
            from,
            to: task.status.clone(),
            reason: transition,
        },
    }
}

fn lifecycle_summary(task: &TaskInfo) -> NaturalLanguageLifecycleInfo {
    lifecycle_summary_with_from(task, None)
}

fn run_workflow_steps() -> Vec<TaskExecutionInfo> {
    vec![
        analyze_repo_state(),
        run_command(
            "cargo check -p weft-code-runtime",
            "cargo",
            &["check", "-p", "weft-code-runtime"],
        ),
    ]
}

fn runtime_note_directory_name(runtime_note_namespace: Option<&str>) -> String {
    match runtime_note_namespace {
        Some(namespace) if !namespace.is_empty() => format!("weft-code-{}", namespace),
        _ => "weft-code".to_string(),
    }
}

fn maybe_write_task_note(
    note_key: &str,
    runtime_note_namespace: Option<&str>,
    prompt: &str,
) -> Result<Option<FileMutationInfo>, String> {
    let lower = prompt.to_ascii_lowercase();
    if !(lower.contains("write") || lower.contains("note") || lower.contains("file")) {
        return Ok(None);
    }

    let safe_slug = note_key
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    let safe_slug = safe_slug
        .trim_matches('-')
        .chars()
        .take(120)
        .collect::<String>();
    let suffix = if safe_slug.is_empty() {
        "prompt".to_string()
    } else {
        safe_slug
    };
    let relative_path = format!(
        ".weft/{}/runtime-note-{}.txt",
        runtime_note_directory_name(runtime_note_namespace),
        suffix
    );
    let absolute_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(&relative_path);
    let content = format!("WEFT-Code runtime note\n\nPrompt:\n{}\n", prompt.trim());

    if let Some(parent) = absolute_path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return Err(format!(
                "failed to create note directory for '{}'",
                relative_path
            ));
        }
    }

    match std::fs::write(&absolute_path, content.as_bytes()) {
        Ok(()) => Ok(Some(FileMutationInfo {
            path: relative_path.to_string(),
            action: "write".to_string(),
            bytes_written: content.len(),
        })),
        Err(error) => Err(format!(
            "failed to write runtime note '{}': {}",
            relative_path, error
        )),
    }
}

fn maybe_apply_controlled_patch(
    session_id: &str,
    task_id: &str,
    runtime_note_namespace: Option<&str>,
    controlled_patch_target: Option<&str>,
    allowed_patch_targets: &[String],
    prompt: &str,
) -> Result<Option<FileMutationInfo>, String> {
    let lower = prompt.to_ascii_lowercase();
    if !lower.contains("patch") {
        return Ok(None);
    }

    let relative_path = controlled_patch_target
        .map(|path| path.to_string())
        .unwrap_or_else(|| {
            format!(
                ".weft/{}/patch-target-{}.txt",
                runtime_note_directory_name(runtime_note_namespace),
                session_id
            )
        });
    if allowed_patch_targets.is_empty()
        || !allowed_patch_targets
            .iter()
            .any(|path| path == &relative_path)
    {
        return Err(format!("patch target '{}' is not allowed", relative_path));
    }
    let absolute_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(&relative_path);

    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create patch directory for '{}': {}",
                relative_path, e
            )
        })?;
    }

    let existing = match std::fs::read_to_string(&absolute_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if relative_path.ends_with(".rs") {
                "// WEFT-Code patch target\n".to_string()
            } else if relative_path.ends_with(".toml") {
                "# WEFT-Code patch target\n".to_string()
            } else {
                "WEFT-Code patch target\n".to_string()
            }
        }
        Err(error) => {
            return Err(format!(
                "failed to read patch target '{}': {}",
                relative_path, error
            ));
        }
    };

    let patch_block = if relative_path.ends_with(".rs") {
        let trimmed_prompt = prompt.trim();
        let line_count = trimmed_prompt.lines().count();
        let commented_prompt = trimmed_prompt
            .lines()
            .map(|line| format!("// {}", line))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "\n// WEFT-PATCH-BEGIN\n// patch_id: {}\n// session_id: {}\n// task_id: {}\n// target: {}\n// line_count: {}\n{}\n// WEFT-PATCH-END\n",
            task_id,
            session_id,
            task_id,
            relative_path,
            line_count,
            commented_prompt
        )
    } else if relative_path.ends_with(".toml") {
        format!(
            "\n# WEFT-PATCH-BEGIN\n# patch_id: {}\n# session_id: {}\n# task_id: {}\n# target: {}\n# line_count: {}\n# {}\n# WEFT-PATCH-END\n",
            task_id,
            session_id,
            task_id,
            relative_path,
            prompt.trim().lines().count(),
            prompt.trim()
        )
    } else {
        format!(
            "\nWEFT-PATCH-BEGIN\npatch_id: {}\nsession_id: {}\ntask_id: {}\ntarget: {}\nline_count: {}\n{}\nWEFT-PATCH-END\n",
            task_id,
            session_id,
            task_id,
            relative_path,
            prompt.trim().lines().count(),
            prompt.trim()
        )
    };
    let new_content = format!("{}{}", existing, patch_block);

    std::fs::write(&absolute_path, new_content.as_bytes())
        .map_err(|e| format!("failed to patch '{}': {}", relative_path, e))?;

    Ok(Some(FileMutationInfo {
        path: relative_path,
        action: "patch".to_string(),
        bytes_written: patch_block.len(),
    }))
}

fn patch_target_registry() -> [(&'static str, &'static str); 2] {
    [
        (
            "patch_target",
            "crates/weft-code-runtime/src/patch_target.rs",
        ),
        (
            "policy_service",
            "crates/weft-code-runtime/src/policy_service.rs",
        ),
    ]
}

fn classify_coding_action(prompt: &str, target_id: Option<&str>) -> CodingActionKind {
    let lower = prompt.to_ascii_lowercase();
    if target_id.is_some() || lower.contains("patch") {
        CodingActionKind::Patch
    } else if lower.contains("write") || lower.contains("note") || lower.contains("file") {
        CodingActionKind::WriteNote
    } else {
        CodingActionKind::Analyze
    }
}

fn is_sensitive_coding_action(action_kind: &CodingActionKind) -> bool {
    matches!(
        action_kind,
        CodingActionKind::Patch | CodingActionKind::WriteNote
    )
}

fn resolve_patch_target_id(target_id: &str) -> Option<&'static str> {
    patch_target_registry()
        .into_iter()
        .find(|(id, _)| *id == target_id)
        .map(|(_, path)| path)
}

fn patch_target_id_from_path(path: &str) -> String {
    patch_target_registry()
        .into_iter()
        .find(|(_, registered_path)| *registered_path == path)
        .map(|(id, _)| id.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn select_patch_target(
    target_id: Option<&str>,
    prompt: &str,
    _default_target: Option<&str>,
    _allowed_patch_targets: &[String],
) -> Option<String> {
    if prompt.to_ascii_lowercase().contains("patch") {
        return target_id
            .and_then(resolve_patch_target_id)
            .map(|path| path.to_string())
            .or_else(|| Some("__missing_target_id__".to_string()));
    }

    None
}

fn note_key_for_task(task_id: &str, runtime_note_namespace: Option<&str>) -> String {
    match runtime_note_namespace {
        Some(namespace) if !namespace.is_empty() => format!("{}-{}", namespace, task_id),
        _ => task_id.to_string(),
    }
}

impl WorkflowGateway {
    pub fn new(state: SharedWeftCodeState) -> Self {
        Self { state }
    }

    pub async fn handle_natural_language_task(
        &self,
        session_id: &str,
        prompt: &str,
        target_id: Option<&str>,
        action_kind: Option<CodingActionKind>,
    ) -> Result<NaturalLanguageTaskResponse, NaturalLanguageTaskError> {
        let trimmed_prompt = prompt.trim();
        if trimmed_prompt.is_empty() {
            return Err(NaturalLanguageTaskError::EmptyPrompt);
        }

        let mut state = self.state.write().await;
        let session_index = state
            .sessions
            .iter()
            .position(|session| session.id == session_id)
            .ok_or(NaturalLanguageTaskError::SessionNotFound)?;
        let existing_task_count = state
            .tasks
            .iter()
            .filter(|task| task.session_id == session_id)
            .count();
        let existing_team_count = state.teams.len();
        let existing_team_task_count = state.team_tasks.len();

        state.sessions[session_index].status = SessionStatus::Active;
        let session_id_owned = state.sessions[session_index].id.clone();
        let session_project_id = state.sessions[session_index].project_id.clone();
        let session_parent_id = state.sessions[session_index].parent_session_id.clone();
        let runtime_note_namespace = state.runtime_note_namespace.clone();
        let controlled_patch_target = state.controlled_patch_target.clone();
        let allowed_patch_targets = state.allowed_patch_targets.clone();
        let approval_policy = state.approval_policy.clone();

        let lower_prompt = trimmed_prompt.to_ascii_lowercase();
        let coding_action_kind =
            action_kind.unwrap_or_else(|| classify_coding_action(trimmed_prompt, target_id));
        let mut created_team: Option<TeamInfo> = None;
        let mut response_action_kind: Option<CodingActionKind> = None;
        let mut related_tasks: Vec<TaskInfo> = Vec::new();
        let execution: Option<TaskExecutionInfo> = None;
        let workflow_steps: Vec<TaskExecutionInfo> = Vec::new();
        let file_mutation: Option<FileMutationInfo> = None;
        let mut interpretation = format!(
            "Interpreted natural language request for session '{}' as a coding workflow task.",
            session_id
        );
        let mut result = format!(
            "Recorded prompt: '{}' and staged a runtime task placeholder.",
            trimmed_prompt
        );
        let mut next_steps = vec![
            "Connect this endpoint to real workflow orchestration.".into(),
            "Replace placeholder completion with actual tool-backed execution.".into(),
        ];

        let task_kind = if lower_prompt.contains("team") {
            response_action_kind = Some(CodingActionKind::TeamKickoff);
            interpretation = format!(
                "Interpreted natural language request for session '{}' as a team-mode kickoff.",
                session_id
            );
            result = format!(
                "Team kickoff request for prompt '{}' is ready to create runtime team state.",
                trimmed_prompt
            );
            next_steps = vec![
                "Approve the request when policy requires explicit confirmation for team kickoff.".into(),
                "Retry the same request after approval to create the team runtime state in this slice.".into(),
            ];
            "workflow_task".to_string()
        } else if lower_prompt.contains("plan") {
            state.sessions[session_index].mode = SessionMode::Plan;
            response_action_kind = Some(CodingActionKind::Plan);
            interpretation = format!(
                "Interpreted natural language request for session '{}' as a planning task.",
                session_id
            );
            result = format!(
                "Moved session '{}' into plan mode for prompt '{}'.",
                session_id, trimmed_prompt
            );
            next_steps = vec![
                "Read the updated session resource to confirm mode=plan.".into(),
                "Add planning-specific task expansion in a later slice.".into(),
            ];
            "plan_task".to_string()
        } else {
            "coding_task".to_string()
        };

        let mut session_snapshot = SessionInfo {
            id: session_id_owned.clone(),
            project_id: session_project_id,
            mode: state.sessions[session_index].mode.clone(),
            status: state.sessions[session_index].status.clone(),
            parent_session_id: session_parent_id,
        };

        let task = TaskInfo {
            id: format!(
                "{}-nl-task-{}",
                session_snapshot.id,
                existing_task_count + 1
            ),
            session_id: session_snapshot.id.clone(),
            kind: task_kind,
            status: TaskStatus::Queued,
            team_id: None,
            parent_task_id: Some("weft-code-bootstrap-task".into()),
        };

        let session_mode = session_snapshot.mode.clone();

        if task.kind == "coding_task" && matches!(session_mode, SessionMode::Plan) {
            let mut task = task.clone();
            task.status = TaskStatus::Blocked;
            state.tasks.push(task.clone());
            let action = action_summary(coding_action_kind.clone(), &task);
            let lifecycle = lifecycle_summary(&task);
            return Ok(NaturalLanguageTaskResponse {
                session: session_snapshot,
                task,
                related_tasks,
                approval: None,
                execution: None,
                workflow_steps: Vec::new(),
                file_mutation: None,
                patch_record: None,
                action_kind: Some(coding_action_kind.clone()),
                action: Some(action),
                lifecycle,
                interpretation: format!(
                    "Interpreted natural language request for session '{}' as a coding workflow task frozen by plan mode.",
                    session_id
                ),
                result: "Session is currently in plan mode, so coding execution remains frozen until the mode changes.".into(),
                next_steps: vec![
                    "Keep using plan prompts to refine the plan, or switch the session out of plan mode before requesting coding execution.".into(),
                ],
                created_team: None,
            });
        }

        let sensitive_action = is_sensitive_coding_action(&coding_action_kind);
        let approval_action_kind = response_action_kind
            .clone()
            .unwrap_or_else(|| coding_action_kind.clone());

        let approval = if (task.kind == "coding_task"
            && (approval_policy == ApprovalPolicy::OnRequest
                || (approval_policy == ApprovalPolicy::OnSensitiveActions && sensitive_action)))
            || (task.kind == "workflow_task"
                && matches!(response_action_kind, Some(CodingActionKind::TeamKickoff))
                && (approval_policy == ApprovalPolicy::OnRequest
                    || approval_policy == ApprovalPolicy::OnSensitiveActions))
        {
            let request_key = request_approval_key(trimmed_prompt, target_id);
            match state
                .approvals
                .iter()
                .find(|approval| {
                    approval.id == format!("{}-approval-{}", task.session_id, request_key)
                })
                .cloned()
            {
                Some(existing) => Some(existing),
                None => {
                    let approval =
                        approval_request_for_task(&task, &request_key, &approval_action_kind);
                    state.approvals.push(approval.clone());
                    Some(approval)
                }
            }
        } else {
            None
        };

        if matches!(response_action_kind, Some(CodingActionKind::TeamKickoff)) {
            if approval_policy == ApprovalPolicy::ReadOnlyMode {
                let mut task = task.clone();
                task.status = TaskStatus::Blocked;
                state.tasks.push(task.clone());
                let action = action_summary(CodingActionKind::TeamKickoff, &task);
                let lifecycle = lifecycle_summary(&task);
                return Ok(NaturalLanguageTaskResponse {
                    session: session_snapshot,
                    task,
                    related_tasks,
                    approval: None,
                    execution: None,
                    workflow_steps: Vec::new(),
                    file_mutation: None,
                    patch_record: None,
                    action_kind: Some(CodingActionKind::TeamKickoff),
                    action: Some(action),
                    lifecycle,
                    interpretation: format!(
                        "Interpreted natural language request for session '{}' as a team workflow task blocked by read_only_mode.",
                        session_id
                    ),
                    result: "Current approval policy is read_only_mode, so team kickoff is blocked before runtime team creation.".into(),
                    next_steps: vec![
                        "Switch policy to on_request or always_allow to enable team kickoff.".into(),
                    ],
                    created_team: None,
                });
            }

            let approval_is_allowed = approval
                .as_ref()
                .map(|approval| approval.status == "approved")
                .unwrap_or(true);

            if !approval_is_allowed {
                let mut task = task.clone();
                task.status = TaskStatus::WaitingApproval;
                state.tasks.push(task.clone());
                let action = action_summary(CodingActionKind::TeamKickoff, &task);
                let lifecycle = lifecycle_summary_with_from(&task, Some(TaskStatus::Queued));
                return Ok(NaturalLanguageTaskResponse {
                    session: session_snapshot,
                    task,
                    related_tasks,
                    approval,
                    execution: None,
                    workflow_steps: Vec::new(),
                    file_mutation: None,
                    patch_record: None,
                    action_kind: Some(CodingActionKind::TeamKickoff),
                    action: Some(action),
                    lifecycle,
                    interpretation: format!(
                        "Interpreted natural language request for session '{}' as a team workflow task pending approval.",
                        session_id
                    ),
                    result: "Team kickoff is waiting for approval before runtime team creation.".into(),
                    next_steps: vec![
                        "Approve the generated approval request via /api/weft-code/approval/{approval_id}/decision.".into(),
                        "Re-issue the same natural language request after approval in this slice.".into(),
                    ],
                    created_team: None,
                });
            }

            state.sessions[session_index].mode = SessionMode::Team;
            session_snapshot.mode = SessionMode::Team;
            let team = TeamInfo {
                id: format!("{}-team-{}", session_id_owned, existing_team_count + 1),
                session_id: session_id_owned.clone(),
                roles: vec!["operator".into()],
            };
            state.teams.push(team.clone());
            state.team_tasks.push(TeamTaskInfo {
                id: format!("{}-task-{}", team.id, existing_team_task_count + 1),
                team_id: team.id.clone(),
                role: "operator".into(),
                phase: "bootstrap".into(),
                status: "queued".into(),
            });
            created_team = Some(team.clone());
            result = format!(
                "Created runtime team '{}' for prompt '{}'.",
                team.id, trimmed_prompt
            );
            next_steps = vec![
                "Inspect /api/weft-code/teams to view the created team.".into(),
                "Inspect /api/weft-code/team/{team_id}/tasks for queued team work.".into(),
            ];
        }

        if task.kind == "coding_task" {
            if approval_policy == ApprovalPolicy::ReadOnlyMode && sensitive_action {
                let blocked_action = match coding_action_kind {
                    CodingActionKind::Patch => "patch",
                    CodingActionKind::WriteNote => "write_note",
                    _ => "sensitive",
                };
                let mut task = task.clone();
                task.status = TaskStatus::Blocked;
                state.tasks.push(task.clone());
                let action = action_summary(coding_action_kind.clone(), &task);
                let lifecycle = lifecycle_summary(&task);
                return Ok(NaturalLanguageTaskResponse {
                    session: session_snapshot,
                    task,
                    related_tasks,
                    approval: None,
                    execution: None,
                    workflow_steps: Vec::new(),
                    file_mutation: None,
                    patch_record: None,
                    action_kind: Some(coding_action_kind.clone()),
                    action: Some(action),
                    lifecycle,
                    interpretation: format!(
                        "Interpreted natural language request for session '{}' as a coding workflow task blocked by read_only_mode.",
                        session_id
                    ),
                    result: format!(
                        "Current approval policy is read_only_mode, so {} execution is blocked.",
                        blocked_action
                    ),
                    next_steps: vec![
                        "Switch policy to on_request or always_allow to enable sensitive execution.".into(),
                    ],
                    created_team: None,
                });
            }

            if approval_policy == ApprovalPolicy::ReadOnlyMode {
                let mut task = task.clone();
                task.status = TaskStatus::Completed;
                state.tasks.push(task.clone());
                let action = action_summary(coding_action_kind.clone(), &task);
                let lifecycle = lifecycle_summary_with_from(&task, Some(TaskStatus::Queued));
                return Ok(NaturalLanguageTaskResponse {
                    session: session_snapshot,
                    task,
                    related_tasks,
                    approval: None,
                    execution: None,
                    workflow_steps: vec![analyze_repo_state()],
                    file_mutation: None,
                    patch_record: None,
                    action_kind: Some(coding_action_kind.clone()),
                    action: Some(action),
                    lifecycle,
                    interpretation: format!(
                        "Interpreted natural language request for session '{}' as a read-only coding task.",
                        session_id
                    ),
                    result: "Current approval policy is read_only_mode, so only the read-only analyze step was executed.".into(),
                    next_steps: vec![
                        "Switch policy to on_request or always_allow to enable patch execution.".into(),
                    ],
                    created_team: None,
                });
            }

            let approval_is_allowed = approval
                .as_ref()
                .map(|approval| approval.status == "approved")
                .unwrap_or(
                    approval_policy == ApprovalPolicy::AlwaysAllow
                        || (approval_policy == ApprovalPolicy::OnSensitiveActions
                            && !sensitive_action),
                );
            if !approval_is_allowed {
                let mut task = task.clone();
                task.status = TaskStatus::WaitingApproval;
                state.tasks.push(task.clone());
                let action = action_summary(coding_action_kind.clone(), &task);
                let lifecycle = lifecycle_summary_with_from(&task, Some(TaskStatus::Queued));
                return Ok(NaturalLanguageTaskResponse {
                    session: session_snapshot,
                    task,
                    related_tasks,
                    approval,
                    execution: None,
                    workflow_steps: Vec::new(),
                    file_mutation: None,
                    patch_record: None,
                    action_kind: Some(coding_action_kind.clone()),
                    action: Some(action),
                    lifecycle,
                    interpretation: format!(
                        "Interpreted natural language request for session '{}' as a coding workflow task pending approval.",
                        session_id
                    ),
                    result: "Coding task is waiting for approval before execution.".into(),
                    next_steps: vec![
                        "Approve the generated approval request via /api/weft-code/approval/{approval_id}/decision.".into(),
                        "Re-issue the natural language request after approval in this slice.".into(),
                    ],
                    created_team: None,
                });
            }

            let verification_task_id = format!(
                "{}-verify-task-{}",
                session_snapshot.id,
                existing_task_count + 1
            );
            let task_id = task.id.clone();
            state.tasks.push(task.clone());
            state.tasks.push(TaskInfo {
                id: verification_task_id.clone(),
                session_id: session_snapshot.id.clone(),
                kind: "verification_task".into(),
                status: TaskStatus::Queued,
                team_id: None,
                parent_task_id: Some(task_id.clone()),
            });
            drop(state);

            let workflow_steps = run_workflow_steps();
            let all_steps_passed = workflow_steps.iter().all(|step| step.exit_code == 0);
            let execution = workflow_steps
                .last()
                .cloned()
                .expect("workflow steps should include repository check");
            let file_mutation = if all_steps_passed {
                let selected_patch_target = select_patch_target(
                    target_id,
                    trimmed_prompt,
                    controlled_patch_target.as_deref(),
                    &allowed_patch_targets,
                );
                let patch_attempt = maybe_apply_controlled_patch(
                    &session_snapshot.id,
                    &task_id,
                    runtime_note_namespace.as_deref(),
                    selected_patch_target.as_deref(),
                    &allowed_patch_targets,
                    trimmed_prompt,
                );
                match patch_attempt {
                    Ok(Some(file_mutation)) => Ok(Some(file_mutation)),
                    Ok(None) => {
                        let note_key =
                            note_key_for_task(&task_id, runtime_note_namespace.as_deref());
                        maybe_write_task_note(
                            &note_key,
                            runtime_note_namespace.as_deref(),
                            trimmed_prompt,
                        )
                    }
                    Err(error) => Err(error),
                }
            } else {
                Ok(None)
            };
            let mut task_status = if all_steps_passed {
                TaskStatus::Completed
            } else {
                TaskStatus::Failed
            };
            let file_mutation = match file_mutation {
                Ok(file_mutation) => file_mutation,
                Err(_error) => {
                    task_status = TaskStatus::Failed;
                    None
                }
            };
            let patch_record = file_mutation.as_ref().and_then(|file_mutation| {
                if file_mutation.action == "patch" {
                    Some(PatchRecordInfo {
                        patch_id: task_id.clone(),
                        session_id: session_snapshot.id.clone(),
                        task_id: task_id.clone(),
                        target_id: patch_target_id_from_path(&file_mutation.path),
                        target_path: file_mutation.path.clone(),
                        line_count: trimmed_prompt.lines().count(),
                    })
                } else {
                    None
                }
            });
            let mut state = self.state.write().await;
            let task = state
                .tasks
                .iter_mut()
                .find(|task| task.id == task_id)
                .ok_or(NaturalLanguageTaskError::SessionNotFound)?;
            task.status = task_status.clone();
            let task_snapshot = task.clone();
            let verification_task = state
                .tasks
                .iter_mut()
                .find(|task| task.id == verification_task_id)
                .ok_or(NaturalLanguageTaskError::SessionNotFound)?;
            verification_task.status = match task_status {
                TaskStatus::Completed => TaskStatus::Completed,
                _ => TaskStatus::Failed,
            };
            let verification_task_snapshot = verification_task.clone();
            related_tasks.push(verification_task_snapshot);
            let action = action_summary(coding_action_kind.clone(), &task_snapshot);
            let lifecycle = lifecycle_summary_with_from(&task_snapshot, Some(TaskStatus::Queued));

            return Ok(NaturalLanguageTaskResponse {
                session: session_snapshot,
                task: task_snapshot,
                related_tasks,
                approval,
                execution: Some(execution.clone()),
                workflow_steps: workflow_steps.clone(),
                file_mutation: file_mutation.clone(),
                patch_record,
                action_kind: Some(coding_action_kind.clone()),
                action: Some(action),
                lifecycle,
                interpretation,
                result: format!(
                    "Recorded prompt: '{}' and ran {} workflow steps; final exit code {}{}{}.",
                    trimmed_prompt,
                    workflow_steps.len(),
                    execution.exit_code,
                    if let Some(file_mutation) = &file_mutation {
                        format!(", then wrote '{}'", file_mutation.path)
                    } else {
                        String::new()
                    },
                    if matches!(task_status, TaskStatus::Failed)
                        && trimmed_prompt.to_ascii_lowercase().contains("write")
                    {
                        ", file write did not succeed"
                    } else {
                        ""
                    }
                ),
                next_steps: vec![
                    "Inspect the execution field for stdout/stderr from the repository check.".into(),
                    "Inspect the file_mutation field when the prompt requests a runtime note/file write.".into(),
                ],
                created_team: None,
            });
        }

        if let Some(team) = &created_team {
            // Tie the task to the just-created team for observability.
            let mut task = task.clone();
            task.team_id = Some(team.id.clone());
            state.tasks.push(task.clone());

            let teammate_task = TaskInfo {
                id: format!("{}-teammate-task-{}", team.id, existing_team_task_count + 1),
                session_id: session_snapshot.id.clone(),
                kind: "teammate_task".into(),
                status: TaskStatus::Queued,
                team_id: Some(team.id.clone()),
                parent_task_id: Some(task.id.clone()),
            };
            state.tasks.push(teammate_task.clone());
            related_tasks.push(teammate_task);
            let action = response_action_kind
                .clone()
                .map(|kind| action_summary(kind, &task));
            let lifecycle = lifecycle_summary_with_from(&task, Some(TaskStatus::Queued));

            return Ok(NaturalLanguageTaskResponse {
                session: session_snapshot,
                task,
                related_tasks,
                approval,
                execution: execution.clone(),
                workflow_steps: workflow_steps.clone(),
                file_mutation: file_mutation.clone(),
                patch_record: None,
                action_kind: response_action_kind.clone(),
                action,
                lifecycle,
                interpretation,
                result,
                next_steps,
                created_team,
            });
        }

        state.tasks.push(task.clone());
        let action = response_action_kind
            .clone()
            .map(|kind| action_summary(kind, &task));
        let lifecycle = lifecycle_summary(&task);

        Ok(NaturalLanguageTaskResponse {
            session: session_snapshot,
            task,
            related_tasks,
            approval,
            execution,
            workflow_steps,
            file_mutation,
            patch_record: None,
            action_kind: response_action_kind,
            action,
            lifecycle,
            interpretation,
            result,
            next_steps,
            created_team: None,
        })
    }
}
