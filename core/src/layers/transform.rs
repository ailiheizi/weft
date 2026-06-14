use crate::config::ProviderConfig;
use crate::types::{ChatRequest, ChatResponse, StreamChunk};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;

/// Describes an outgoing HTTP request to a provider.
#[derive(Debug)]
pub struct ProviderRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

#[async_trait]
pub trait TransformLayer: Send + Sync {
    /// Convert a ChatRequest into the provider's HTTP format.
    async fn transform_request(
        &self,
        request: &ChatRequest,
        api_key: &str,
        provider: &ProviderConfig,
    ) -> Result<ProviderRequest>;

    /// Parse a provider's HTTP response into ChatResponse.
    async fn transform_response(
        &self,
        status: u16,
        body: Bytes,
        provider: &ProviderConfig,
    ) -> Result<ChatResponse>;

    /// Parse one SSE chunk from the provider into our StreamChunk.
    /// Returns None if the chunk is a keep-alive or should be skipped.
    async fn transform_stream_chunk(
        &self,
        chunk: &str,
        provider: &ProviderConfig,
    ) -> Result<Option<StreamChunk>>;
}
