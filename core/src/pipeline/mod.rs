pub mod context;

use crate::config::AppConfig;
use crate::layers::key_selector::ApiKeyState;
use crate::layers::{
    ErrorAction, ErrorHandlerLayer, KeySelectorLayer, RequestError, RouterLayer,
};
use crate::types::{ChatRequest, ChatResponse};
use anyhow::{bail, Context, Result};
use bytes::Bytes;
use context::RequestContext;
use std::sync::Arc;

pub struct Pipeline {
    pub router: Arc<dyn RouterLayer>,
    pub key_selector: Arc<dyn KeySelectorLayer>,
    pub transforms: Arc<crate::defaults::transforms::TransformRegistry>,
    pub error_handler: Arc<dyn ErrorHandlerLayer>,
    pub http_client: reqwest::Client,
}

impl Pipeline {
    /// Execute a non-streaming chat request through the full pipeline.
    pub async fn execute(&self, request: &ChatRequest, config: &AppConfig) -> Result<ChatResponse> {
        let mut ctx = RequestContext::new();
        let max_attempts = config.fallback.retry_count + 1;
        let mut providers_tried: Vec<String> = vec![];

        loop {
            // 1. Route
            let provider_name = self
                .router
                .route(request, &config.providers)
                .await
                .context("Routing failed")?;
            ctx.selected_provider = Some(provider_name.clone());

            let provider = config
                .providers
                .iter()
                .find(|p| p.name == provider_name)
                .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", provider_name))?;

            // 2. Select key
            let key_states: Vec<ApiKeyState> = provider
                .keys
                .iter()
                .enumerate()
                .map(|(i, k)| ApiKeyState::from((i, k)))
                .collect();

            if key_states.is_empty() {
                bail!("No API keys configured for provider '{}'", provider_name);
            }

            let key_index = self
                .key_selector
                .select(&provider_name, &key_states)
                .await
                .context("Key selection failed")?;
            ctx.selected_key_index = Some(key_index);
            ctx.selected_key_value = Some(key_states[key_index].value.clone());

            // 3. Transform request
            let provider_req = self
                .transforms
                .for_format(&provider.format)
                .transform_request(request, &key_states[key_index].value, provider)
                .await
                .context("Request transform failed")?;

            // 4. Send HTTP request
            let result = self.send_request(&provider_req).await;

            match result {
                Ok((status, body)) => {
                    if (200..300).contains(&status) {
                        // 5. Transform response
                        let resp = self
                            .transforms
                            .for_format(&provider.format)
                            .transform_response(status, body, provider)
                            .await
                            .context("Response transform failed")?;
                        self.key_selector.mark_success(&provider_name, key_index);
                        return Ok(resp);
                    }

                    // Error from provider
                    let error = RequestError {
                        status: Some(status),
                        message: String::from_utf8_lossy(&body).to_string(),
                        provider: provider_name.clone(),
                        key_index,
                        retry_count: ctx.retry_count,
                    };

                    match self.error_handler.handle(&error).await {
                        ErrorAction::Retry { delay_ms } => {
                            ctx.retry_count += 1;
                            if ctx.retry_count > max_attempts {
                                bail!("Max retries exceeded: {}", error.message);
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        ErrorAction::SwitchKey => {
                            self.key_selector.mark_failed(&provider_name, key_index);
                            ctx.retry_count += 1;
                            if ctx.retry_count > max_attempts {
                                bail!("Max retries exceeded after key switch: {}", error.message);
                            }
                            continue;
                        }
                        ErrorAction::SwitchProvider => {
                            providers_tried.push(provider_name.clone());
                            // Find next provider from fallback priority
                            let next = config
                                .fallback
                                .priority
                                .iter()
                                .find(|p| !providers_tried.contains(p));
                            if let Some(_next_provider) = next {
                                ctx.retry_count += 1;
                                continue;
                            }
                            bail!("All providers exhausted: {}", error.message);
                        }
                        ErrorAction::Fail { message } => {
                            bail!("{}", message);
                        }
                    }
                }
                Err(e) => {
                    // Network error
                    let error = RequestError {
                        status: None,
                        message: e.to_string(),
                        provider: provider_name.clone(),
                        key_index,
                        retry_count: ctx.retry_count,
                    };

                    match self.error_handler.handle(&error).await {
                        ErrorAction::Retry { delay_ms } => {
                            ctx.retry_count += 1;
                            if ctx.retry_count > max_attempts {
                                bail!("Max retries exceeded: {}", e);
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        ErrorAction::SwitchProvider => {
                            providers_tried.push(provider_name);
                            let next = config
                                .fallback
                                .priority
                                .iter()
                                .find(|p| !providers_tried.contains(p));
                            if next.is_some() {
                                ctx.retry_count += 1;
                                continue;
                            }
                            bail!("All providers exhausted: {}", e);
                        }
                        _ => bail!("Request failed: {}", e),
                    }
                }
            }
        }
    }

    async fn send_request(
        &self,
        req: &crate::layers::transform::ProviderRequest,
    ) -> Result<(u16, Bytes)> {
        let mut builder = match req.method.as_str() {
            "POST" => self.http_client.post(&req.url),
            "GET" => self.http_client.get(&req.url),
            _ => bail!("Unsupported method: {}", req.method),
        };

        for (k, v) in &req.headers {
            builder = builder.header(k.as_str(), v.as_str());
        }

        let resp = builder
            .body(req.body.clone())
            .send()
            .await
            .context("HTTP request failed")?;

        let status = resp.status().as_u16();
        let body = resp.bytes().await.context("Failed to read response body")?;
        Ok((status, body))
    }

    /// Execute a streaming chat request. Returns the provider name and raw reqwest::Response.
    pub async fn execute_stream(
        &self,
        request: &ChatRequest,
        config: &AppConfig,
    ) -> Result<(String, reqwest::Response)> {
        // 1. Route
        let provider_name = self
            .router
            .route(request, &config.providers)
            .await
            .context("Routing failed")?;

        let provider = config
            .providers
            .iter()
            .find(|p| p.name == provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", provider_name))?;

        // 2. Select key
        let key_states: Vec<ApiKeyState> = provider
            .keys
            .iter()
            .enumerate()
            .map(|(i, k)| ApiKeyState::from((i, k)))
            .collect();

        if key_states.is_empty() {
            bail!("No API keys configured for provider '{}'", provider_name);
        }

        let key_index = self
            .key_selector
            .select(&provider_name, &key_states)
            .await
            .context("Key selection failed")?;

        // 3. Transform request (ensure stream=true)
        let mut stream_request = request.clone();
        stream_request.stream = true;

        let provider_req = self
            .transforms
            .for_format(&provider.format)
            .transform_request(&stream_request, &key_states[key_index].value, provider)
            .await
            .context("Request transform failed")?;

        // 4. Send HTTP request with streaming response
        let mut builder = match provider_req.method.as_str() {
            "POST" => self.http_client.post(&provider_req.url),
            "GET" => self.http_client.get(&provider_req.url),
            _ => bail!("Unsupported method: {}", provider_req.method),
        };

        for (k, v) in &provider_req.headers {
            builder = builder.header(k.as_str(), v.as_str());
        }

        let resp = builder
            .body(provider_req.body.clone())
            .send()
            .await
            .context("Streaming HTTP request failed")?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.bytes().await.unwrap_or_default();
            bail!(
                "Provider returned status {}: {}",
                status,
                String::from_utf8_lossy(&body)
            );
        }

        Ok((provider_name, resp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use crate::defaults::*;
    use crate::types::*;

    fn test_config() -> AppConfig {
        AppConfig {
            core: CoreConfig::default(),
            providers: vec![ProviderConfig {
                name: "test".into(),
                base_url: "http://localhost:9999".into(),
                format: "openai".into(),
                api: ProviderApi::ChatCompletions,
                keys: vec![ApiKeyConfig {
                    value: "sk-test".into(),
                    label: None,
                }],
                models: vec!["test-model".into()],
            }],
            routing: RoutingConfig {
                default_provider: Some("test".into()),
                default_model: Some("test-model".into()),
            },
            key_strategy: KeyStrategyConfig::default(),
            fallback: FallbackConfig {
                retry_count: 0,
                switch_key: false,
                switch_provider: false,
                priority: vec![],
            },
            virtual_keys: vec![],
            services: vec![],
            packages: vec![],
            registry: RegistryConfig::default(),
            package_aliases: Default::default(),
            web_search: Default::default(),
            team: Default::default(),
        }
    }

    fn test_request() -> ChatRequest {
        ChatRequest {
            model: "test-model".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hello".into(),
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

    fn test_pipeline() -> Pipeline {
        Pipeline {
            router: Arc::new(DefaultRouter {
                default_provider: "test".into(),
            }),
            key_selector: Arc::new(FailoverSelector),
            transforms: Arc::new(crate::defaults::transforms::TransformRegistry::with_defaults()),
            error_handler: Arc::new(DefaultErrorHandler { max_retries: 2 }),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .connect_timeout(std::time::Duration::from_secs(1))
                .build()
                .unwrap(),
        }
    }

    #[tokio::test]
    async fn test_pipeline_routes_correctly() {
        let pipeline = test_pipeline();
        let config = test_config();
        let result = pipeline.execute(&test_request(), &config).await;
        // Should fail with connection error (no server running)
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("error") || err.contains("connect") || err.contains("failed"),
            "Unexpected error: {}",
            err
        );
    }
}
