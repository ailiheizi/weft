use crate::layers::error_handler::{ErrorAction, ErrorHandlerLayer, RequestError};
use async_trait::async_trait;

pub struct DefaultErrorHandler {
    pub max_retries: u32,
}

#[async_trait]
impl ErrorHandlerLayer for DefaultErrorHandler {
    async fn handle(&self, error: &RequestError) -> ErrorAction {
        match error.status {
            // Rate limited -> switch key
            Some(429) => ErrorAction::SwitchKey,

            // Auth error -> switch key (key might be revoked)
            Some(401) | Some(403) => ErrorAction::SwitchKey,

            // Server error -> retry with backoff
            Some(s) if s >= 500 => {
                if error.retry_count < self.max_retries {
                    ErrorAction::Retry {
                        delay_ms: 1000 * (error.retry_count as u64 + 1),
                    }
                } else {
                    ErrorAction::SwitchProvider
                }
            }

            // Connection error (no status) -> retry
            None => {
                if error.retry_count < self.max_retries {
                    ErrorAction::Retry { delay_ms: 500 }
                } else {
                    ErrorAction::SwitchProvider
                }
            }

            // Client errors -> fail immediately
            _ => ErrorAction::Fail {
                message: error.message.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_error(status: Option<u16>, retry_count: u32) -> RequestError {
        RequestError {
            status,
            message: "test error".into(),
            provider: "test".into(),
            key_index: 0,
            retry_count,
        }
    }

    #[tokio::test]
    async fn test_429_switches_key() {
        let h = DefaultErrorHandler { max_retries: 2 };
        let action = h.handle(&make_error(Some(429), 0)).await;
        assert!(matches!(action, ErrorAction::SwitchKey));
    }

    #[tokio::test]
    async fn test_500_retries_then_switches_provider() {
        let h = DefaultErrorHandler { max_retries: 2 };
        let a1 = h.handle(&make_error(Some(500), 0)).await;
        assert!(matches!(a1, ErrorAction::Retry { .. }));
        let a2 = h.handle(&make_error(Some(500), 2)).await;
        assert!(matches!(a2, ErrorAction::SwitchProvider));
    }

    #[tokio::test]
    async fn test_400_fails() {
        let h = DefaultErrorHandler { max_retries: 2 };
        let action = h.handle(&make_error(Some(400), 0)).await;
        assert!(matches!(action, ErrorAction::Fail { .. }));
    }
}
