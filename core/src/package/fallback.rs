use crate::config::ProviderConfig;
use crate::layers::{
    key_selector::ApiKeyState, ErrorAction, ErrorHandlerLayer, KeySelectorLayer, RequestError,
    RouterLayer,
};
use crate::types::ChatRequest;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Tries the primary (JS) router; on failure, falls back to the default.
pub struct FallbackRouter {
    pub primary: Arc<dyn RouterLayer>,
    pub fallback: Arc<dyn RouterLayer>,
}

#[async_trait]
impl RouterLayer for FallbackRouter {
    async fn route(&self, request: &ChatRequest, providers: &[ProviderConfig]) -> Result<String> {
        match self.primary.route(request, providers).await {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::warn!("Primary router failed, using fallback: {}", e);
                self.fallback.route(request, providers).await
            }
        }
    }
}

/// Tries the primary (JS) key selector; on failure, falls back to the default.
pub struct FallbackKeySelector {
    pub primary: Arc<dyn KeySelectorLayer>,
    pub fallback: Arc<dyn KeySelectorLayer>,
}

#[async_trait]
impl KeySelectorLayer for FallbackKeySelector {
    async fn select(&self, provider: &str, keys: &[ApiKeyState]) -> Result<usize> {
        match self.primary.select(provider, keys).await {
            Ok(idx) => Ok(idx),
            Err(e) => {
                tracing::warn!("Primary key selector failed, using fallback: {}", e);
                self.fallback.select(provider, keys).await
            }
        }
    }

    fn mark_failed(&self, provider: &str, index: usize) {
        self.fallback.mark_failed(provider, index);
    }

    fn mark_success(&self, provider: &str, index: usize) {
        self.fallback.mark_success(provider, index);
    }
}

/// Tries the primary (JS) error handler; on failure, falls back to the default.
pub struct FallbackErrorHandler {
    pub primary: Arc<dyn ErrorHandlerLayer>,
    pub fallback: Arc<dyn ErrorHandlerLayer>,
}

#[async_trait]
impl ErrorHandlerLayer for FallbackErrorHandler {
    async fn handle(&self, error: &RequestError) -> ErrorAction {
        // ErrorHandlerLayer doesn't return Result, so we just use the primary.
        // The JS bridge already has retry+fallback-to-Fail logic internally.
        self.primary.handle(error).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::{ChatMessage, ChatRequest};

    fn test_request() -> ChatRequest {
        ChatRequest {
            model: "test".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
                tool_calls: None,
                tool_call_id: None,
            }],
            stream: false,
            temperature: None,
            max_tokens: None,
            top_p: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            x_provider: None,
        }
    }

    struct AlwaysFailRouter;

    #[async_trait]
    impl RouterLayer for AlwaysFailRouter {
        async fn route(&self, _: &ChatRequest, _: &[ProviderConfig]) -> Result<String> {
            anyhow::bail!("always fails")
        }
    }

    struct FixedRouter(String);

    #[async_trait]
    impl RouterLayer for FixedRouter {
        async fn route(&self, _: &ChatRequest, _: &[ProviderConfig]) -> Result<String> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn test_fallback_router_uses_primary() {
        let router = FallbackRouter {
            primary: Arc::new(FixedRouter("primary-provider".into())),
            fallback: Arc::new(FixedRouter("fallback-provider".into())),
        };
        let req = test_request();
        let result = router.route(&req, &[]).await.unwrap();
        assert_eq!(result, "primary-provider");
    }

    #[tokio::test]
    async fn test_fallback_router_falls_back() {
        let router = FallbackRouter {
            primary: Arc::new(AlwaysFailRouter),
            fallback: Arc::new(FixedRouter("fallback-provider".into())),
        };
        let req = test_request();
        let result = router.route(&req, &[]).await.unwrap();
        assert_eq!(result, "fallback-provider");
    }
}
