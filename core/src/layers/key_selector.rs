use crate::config::ApiKeyConfig;
use anyhow::Result;
use async_trait::async_trait;

/// Runtime key state (wraps config + usage tracking)
#[derive(Debug, Clone)]
pub struct ApiKeyState {
    pub index: usize,
    pub value: String,
    pub label: Option<String>,
    pub failed: bool,
    pub usage_count: u64,
}

impl From<(usize, &ApiKeyConfig)> for ApiKeyState {
    fn from((index, cfg): (usize, &ApiKeyConfig)) -> Self {
        Self {
            index,
            value: cfg.value.clone(),
            label: cfg.label.clone(),
            failed: false,
            usage_count: 0,
        }
    }
}

#[async_trait]
pub trait KeySelectorLayer: Send + Sync {
    /// Pick one key from the list.
    async fn select(&self, provider: &str, keys: &[ApiKeyState]) -> Result<usize>; // returns index

    /// Mark a key as failed (e.g. 401, 429).
    fn mark_failed(&self, provider: &str, index: usize);

    /// Mark a key as successful.
    fn mark_success(&self, provider: &str, index: usize);
}
