use crate::state::SharedWeftCodeState;

#[derive(Clone)]
pub struct PolicyService {
    pub state: SharedWeftCodeState,
}
