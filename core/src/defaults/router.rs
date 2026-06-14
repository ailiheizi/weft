use crate::config::ProviderConfig;
use crate::layers::RouterLayer;
use crate::types::ChatRequest;
use anyhow::{bail, Result};
use async_trait::async_trait;

/// Routes to the provider that lists the requested model.
/// Falls back to default_provider if no match.
pub struct DefaultRouter {
    pub default_provider: String,
}

#[async_trait]
impl RouterLayer for DefaultRouter {
    async fn route(&self, request: &ChatRequest, providers: &[ProviderConfig]) -> Result<String> {
        // If x_provider is set, use it directly
        if let Some(ref p) = request.x_provider {
            if providers.iter().any(|prov| prov.name == *p) {
                return Ok(p.clone());
            }
            bail!("Requested provider '{}' not configured", p);
        }

        // Find provider that has the requested model
        for prov in providers {
            if prov.models.contains(&request.model) {
                return Ok(prov.name.clone());
            }
        }

        // Fallback to default
        if providers.iter().any(|p| p.name == self.default_provider) {
            return Ok(self.default_provider.clone());
        }

        // Last resort: first provider
        providers
            .first()
            .map(|p| p.name.clone())
            .ok_or_else(|| anyhow::anyhow!("No providers configured"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderApi, ProviderConfig};
    use crate::types::{ChatMessage, ChatRequest};

    fn make_request(model: &str) -> ChatRequest {
        ChatRequest {
            model: model.into(),
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

    fn make_providers() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig {
                name: "openrouter".into(),
                base_url: "https://openrouter.ai/api/v1".into(),
                format: "openai".into(),
                api: ProviderApi::ChatCompletions,
                keys: vec![],
                models: vec!["claude-sonnet-4".into(), "gpt-4o".into()],
            },
            ProviderConfig {
                name: "anthropic".into(),
                base_url: "https://api.anthropic.com/v1".into(),
                format: "anthropic".into(),
                api: ProviderApi::ChatCompletions,
                keys: vec![],
                models: vec!["claude-sonnet-4".into()],
            },
        ]
    }

    #[tokio::test]
    async fn test_route_by_model() {
        let router = DefaultRouter {
            default_provider: "openrouter".into(),
        };
        let req = make_request("claude-sonnet-4");
        let result = router.route(&req, &make_providers()).await.unwrap();
        // First provider with the model wins
        assert_eq!(result, "openrouter");
    }

    #[tokio::test]
    async fn test_route_fallback_to_default() {
        let router = DefaultRouter {
            default_provider: "openrouter".into(),
        };
        let req = make_request("unknown-model");
        let result = router.route(&req, &make_providers()).await.unwrap();
        assert_eq!(result, "openrouter");
    }

    #[tokio::test]
    async fn test_route_x_provider() {
        let router = DefaultRouter {
            default_provider: "openrouter".into(),
        };
        let mut req = make_request("claude-sonnet-4");
        req.x_provider = Some("anthropic".into());
        let result = router.route(&req, &make_providers()).await.unwrap();
        assert_eq!(result, "anthropic");
    }
}
