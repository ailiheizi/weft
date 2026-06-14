pub mod anthropic;
pub mod openai;

pub use anthropic::AnthropicTransform;
pub use openai::OpenAITransform;

use crate::layers::TransformLayer;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry that selects a [`TransformLayer`] by provider `format` at runtime.
///
/// Previously the pipeline hard-wired `OpenAITransform`, so providers declaring
/// `format = "anthropic"` were silently transformed as OpenAI. This registry
/// dispatches on the provider's declared format instead.
pub struct TransformRegistry {
    map: HashMap<String, Arc<dyn TransformLayer>>,
    fallback: Arc<dyn TransformLayer>,
}

impl TransformRegistry {
    /// Build the default registry seeded with the built-in transforms.
    pub fn with_defaults() -> Self {
        let openai: Arc<dyn TransformLayer> = Arc::new(OpenAITransform);
        let mut map: HashMap<String, Arc<dyn TransformLayer>> = HashMap::new();
        map.insert("openai".to_string(), openai.clone());
        map.insert("anthropic".to_string(), Arc::new(AnthropicTransform));
        Self {
            map,
            fallback: openai,
        }
    }

    /// Resolve the transform for a provider's declared `format`.
    /// Unknown formats fall back to OpenAI (with a warning) to preserve the
    /// previous behaviour rather than failing the request.
    pub fn for_format(&self, format: &str) -> Arc<dyn TransformLayer> {
        match self.map.get(format) {
            Some(t) => t.clone(),
            None => {
                tracing::warn!(
                    format = %format,
                    "No transform registered for provider format; falling back to OpenAI transform"
                );
                self.fallback.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_distinct_transforms_per_format() {
        let reg = TransformRegistry::with_defaults();
        let openai = reg.for_format("openai");
        let anthropic = reg.for_format("anthropic");
        // openai and anthropic must be DIFFERENT transforms — the original bug
        // was that everything resolved to OpenAI regardless of format.
        assert!(
            !Arc::ptr_eq(&openai, &anthropic),
            "anthropic format must not resolve to the OpenAI transform"
        );
    }

    #[test]
    fn unknown_format_falls_back_to_openai() {
        let reg = TransformRegistry::with_defaults();
        let openai = reg.for_format("openai");
        let unknown = reg.for_format("some-unregistered-format");
        assert!(
            Arc::ptr_eq(&openai, &unknown),
            "unknown format should fall back to the OpenAI transform"
        );
    }
}


