use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct RequestError {
    pub status: Option<u16>,
    pub message: String,
    pub provider: String,
    pub key_index: usize,
    pub retry_count: u32,
}

#[derive(Debug, Clone)]
pub enum ErrorAction {
    Retry { delay_ms: u64 },
    SwitchKey,
    SwitchProvider,
    Fail { message: String },
}

#[async_trait]
pub trait ErrorHandlerLayer: Send + Sync {
    async fn handle(&self, error: &RequestError) -> ErrorAction;
}
