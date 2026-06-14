use crate::state::SharedWeftCodeState;
use crate::types::{
    ApprovalPolicy, ApprovalPolicyInfo, ProjectInfo, SessionInfo, SessionMode, TeamInfo,
    TeamTaskInfo,
};

#[derive(Clone)]
pub struct WeftCodeService {
    pub state: SharedWeftCodeState,
}

impl WeftCodeService {
    pub fn new(state: SharedWeftCodeState) -> Self {
        Self { state }
    }

    pub async fn list_projects(&self) -> Vec<ProjectInfo> {
        self.state.read().await.projects.clone()
    }

    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        self.state.read().await.sessions.clone()
    }

    pub async fn set_session_mode(
        &self,
        session_id: &str,
        mode: SessionMode,
    ) -> Option<SessionInfo> {
        let mut state = self.state.write().await;
        let session = state
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)?;
        session.mode = mode;
        Some(session.clone())
    }

    pub async fn list_teams(&self) -> Vec<TeamInfo> {
        self.state.read().await.teams.clone()
    }

    pub async fn approval_policy(&self) -> ApprovalPolicyInfo {
        ApprovalPolicyInfo {
            policy: self.state.read().await.approval_policy.clone(),
        }
    }

    pub async fn set_approval_policy(&self, policy: ApprovalPolicy) -> ApprovalPolicyInfo {
        let mut state = self.state.write().await;
        state.approval_policy = policy.clone();
        ApprovalPolicyInfo { policy }
    }

    pub async fn create_team_for_session(&self, session_id: &str) -> Option<TeamInfo> {
        let mut state = self.state.write().await;
        let session = state
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)?;
        session.mode = SessionMode::Team;

        let team = TeamInfo {
            id: "weft-code-local-team".into(),
            session_id: session.id.clone(),
            roles: vec!["operator".into()],
        };
        let already_exists = state.teams.iter().any(|existing| existing.id == team.id);
        if !already_exists {
            state.teams.push(team.clone());
            state.team_tasks.push(TeamTaskInfo {
                id: "weft-code-local-team-task".into(),
                team_id: team.id.clone(),
                role: "operator".into(),
                phase: "bootstrap".into(),
                status: "queued".into(),
            });
        }

        Some(team)
    }

    pub async fn list_team_tasks(&self, team_id: &str) -> Vec<TeamTaskInfo> {
        self.state
            .read()
            .await
            .team_tasks
            .iter()
            .filter(|task| task.team_id == team_id)
            .cloned()
            .collect()
    }
}
