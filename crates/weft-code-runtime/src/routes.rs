use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream;
use std::convert::Infallible;

use crate::events::RuntimeEvent;
use crate::service::WeftCodeService;
use crate::session_service::SessionService;
use crate::task_service::TaskService;
use crate::types::{
    ApprovalDecisionRequest, ApprovalPolicyInfo, ApprovalPolicyUpdateRequest, ApprovalRequestInfo,
    NaturalLanguageTaskRequest, NaturalLanguageTaskResponse, ProjectInfo, SessionInfo,
    SessionModeUpdateRequest, SessionParentUpdateRequest, TaskInfo, TaskStatusUpdateRequest,
    TeamCreatedResponse, TeamInfo, TeamTaskInfo,
};
use crate::workflow_gateway::{NaturalLanguageTaskError, WorkflowGateway};

pub fn router() -> Router<WeftCodeService> {
    Router::new()
        .route("/api/weft-code/projects", get(list_projects))
        .route("/api/weft-code/sessions", get(list_sessions))
        .route("/api/weft-code/teams", get(list_teams))
        .route("/api/weft-code/session/{session_id}", get(get_session))
        .route(
            "/api/weft-code/session/{session_id}/team",
            post(create_team),
        )
        .route(
            "/api/weft-code/session/{session_id}/parent",
            post(update_session_parent),
        )
        .route(
            "/api/weft-code/session/{session_id}/tasks",
            get(list_session_tasks),
        )
        .route("/api/weft-code/team/{team_id}/tasks", get(list_team_tasks))
        .route(
            "/api/weft-code/task/{task_id}/status",
            post(update_task_status),
        )
        .route("/api/weft-code/approvals", get(list_approvals))
        .route(
            "/api/weft-code/policy",
            get(get_policy).post(update_policy),
        )
        .route(
            "/api/weft-code/approval/{approval_id}/decision",
            post(decide_approval),
        )
        .route(
            "/api/weft-code/session/{session_id}/mode",
            post(update_session_mode),
        )
        .route(
            "/api/weft-code/session/{session_id}/natural-language-task",
            post(create_natural_language_task),
        )
        .route("/api/weft-code/events", get(stream_events))
}

async fn list_projects(State(service): State<WeftCodeService>) -> Json<Vec<ProjectInfo>> {
    Json(service.list_projects().await)
}

async fn list_sessions(State(service): State<WeftCodeService>) -> Json<Vec<SessionInfo>> {
    Json(service.list_sessions().await)
}

async fn list_teams(State(service): State<WeftCodeService>) -> Json<Vec<TeamInfo>> {
    Json(service.list_teams().await)
}

async fn get_session(
    Path(session_id): Path<String>,
    State(service): State<WeftCodeService>,
) -> Result<Json<SessionInfo>, axum::http::StatusCode> {
    SessionService::new(service.state.clone())
        .get_session(&session_id)
        .await
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

async fn create_team(
    Path(session_id): Path<String>,
    State(service): State<WeftCodeService>,
) -> Result<Json<TeamCreatedResponse>, axum::http::StatusCode> {
    let team = service
        .create_team_for_session(&session_id)
        .await
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let event = RuntimeEvent::TeamCreated {
        team_id: team.id.clone(),
        session_id: team.session_id.clone(),
    };

    Ok(Json(TeamCreatedResponse { team, event }))
}

async fn update_session_parent(
    Path(session_id): Path<String>,
    State(service): State<WeftCodeService>,
    Json(request): Json<SessionParentUpdateRequest>,
) -> Result<Json<SessionInfo>, axum::http::StatusCode> {
    SessionService::new(service.state.clone())
        .set_parent_session(&session_id, request.parent_session_id)
        .await
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

async fn list_session_tasks(
    Path(session_id): Path<String>,
    State(service): State<WeftCodeService>,
) -> Json<Vec<TaskInfo>> {
    Json(
        TaskService::new(service.state.clone())
            .list_tasks_for_session(&session_id)
            .await,
    )
}

async fn list_team_tasks(
    Path(team_id): Path<String>,
    State(service): State<WeftCodeService>,
) -> Json<Vec<TeamTaskInfo>> {
    Json(service.list_team_tasks(&team_id).await)
}

async fn list_approvals(State(service): State<WeftCodeService>) -> Json<Vec<ApprovalRequestInfo>> {
    Json(
        TaskService::new(service.state.clone())
            .list_approvals()
            .await,
    )
}

async fn get_policy(State(service): State<WeftCodeService>) -> Json<ApprovalPolicyInfo> {
    Json(service.approval_policy().await)
}

async fn update_policy(
    State(service): State<WeftCodeService>,
    Json(request): Json<ApprovalPolicyUpdateRequest>,
) -> Json<ApprovalPolicyInfo> {
    Json(service.set_approval_policy(request.policy).await)
}

async fn update_task_status(
    Path(task_id): Path<String>,
    State(service): State<WeftCodeService>,
    Json(request): Json<TaskStatusUpdateRequest>,
) -> Result<Json<TaskInfo>, axum::http::StatusCode> {
    TaskService::new(service.state.clone())
        .update_task_status(&task_id, request.status)
        .await
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

async fn decide_approval(
    Path(approval_id): Path<String>,
    State(service): State<WeftCodeService>,
    Json(request): Json<ApprovalDecisionRequest>,
) -> Result<Json<ApprovalRequestInfo>, axum::http::StatusCode> {
    TaskService::new(service.state.clone())
        .decide_approval(&approval_id, request.status)
        .await
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

async fn update_session_mode(
    Path(session_id): Path<String>,
    State(service): State<WeftCodeService>,
    Json(request): Json<SessionModeUpdateRequest>,
) -> Result<Json<SessionInfo>, axum::http::StatusCode> {
    service
        .set_session_mode(&session_id, request.mode)
        .await
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

async fn create_natural_language_task(
    Path(session_id): Path<String>,
    State(service): State<WeftCodeService>,
    Json(request): Json<NaturalLanguageTaskRequest>,
) -> Result<Json<NaturalLanguageTaskResponse>, axum::http::StatusCode> {
    WorkflowGateway::new(service.state.clone())
        .handle_natural_language_task(
            &session_id,
            &request.prompt,
            request.target_id.as_deref(),
            request.action_kind,
        )
        .await
        .map(Json)
        .map_err(|error| match error {
            NaturalLanguageTaskError::EmptyPrompt => axum::http::StatusCode::BAD_REQUEST,
            NaturalLanguageTaskError::SessionNotFound => axum::http::StatusCode::NOT_FOUND,
        })
}

async fn stream_events(
    State(_service): State<WeftCodeService>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let bootstrap_event = RuntimeEvent::SessionCreated {
        session_id: "weft-code-local-session".into(),
    };
    let payload = serde_json::to_string(&bootstrap_event).expect("runtime event should serialize");

    Sse::new(stream::once(async move {
        Ok(Event::default().event("runtime_event").data(payload))
    }))
}
