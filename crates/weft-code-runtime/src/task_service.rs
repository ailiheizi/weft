use crate::state::SharedWeftCodeState;
use crate::types::{ApprovalRequestInfo, TaskInfo, TaskStatus};

#[derive(Clone)]
pub struct TaskService {
    pub state: SharedWeftCodeState,
}

impl TaskService {
    pub fn new(state: SharedWeftCodeState) -> Self {
        Self { state }
    }

    pub async fn list_tasks_for_session(&self, session_id: &str) -> Vec<TaskInfo> {
        self.state
            .read()
            .await
            .tasks
            .iter()
            .filter(|task| task.session_id == session_id)
            .cloned()
            .collect()
    }

    pub async fn list_approvals(&self) -> Vec<ApprovalRequestInfo> {
        self.state.read().await.approvals.clone()
    }

    pub async fn decide_approval(
        &self,
        approval_id: &str,
        status: String,
    ) -> Option<ApprovalRequestInfo> {
        let mut state = self.state.write().await;
        let approval = state
            .approvals
            .iter_mut()
            .find(|approval| approval.id == approval_id)?;
        approval.status = status;
        Some(approval.clone())
    }

    pub async fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Option<TaskInfo> {
        let mut state = self.state.write().await;
        let task = state.tasks.iter_mut().find(|task| task.id == task_id)?;
        task.status = status;
        Some(task.clone())
    }
}
