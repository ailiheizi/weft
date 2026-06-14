use crate::state::SharedWeftCodeState;
use crate::types::SessionInfo;

#[derive(Clone)]
pub struct SessionService {
    pub state: SharedWeftCodeState,
}

impl SessionService {
    pub fn new(state: SharedWeftCodeState) -> Self {
        Self { state }
    }

    pub async fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        self.state
            .read()
            .await
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .cloned()
    }

    pub async fn set_parent_session(
        &self,
        session_id: &str,
        parent_session_id: Option<String>,
    ) -> Option<SessionInfo> {
        let mut state = self.state.write().await;
        let session = state
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)?;
        session.parent_session_id = parent_session_id;
        Some(session.clone())
    }
}
