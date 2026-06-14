#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeEvent {
    SessionCreated {
        session_id: String,
    },
    TaskCreated {
        task_id: String,
        session_id: String,
    },
    ModeChanged {
        session_id: String,
        mode: String,
    },
    ApprovalRequested {
        approval_id: String,
        session_id: String,
    },
    TeamCreated {
        team_id: String,
        session_id: String,
    },
}
