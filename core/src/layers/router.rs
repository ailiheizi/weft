use crate::config::ProviderConfig;
use crate::types::ChatRequest;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait RouterLayer: Send + Sync {
    /// Choose which provider should handle this request.
    /// Returns the provider name.
    async fn route(&self, request: &ChatRequest, providers: &[ProviderConfig]) -> Result<String>;
}
