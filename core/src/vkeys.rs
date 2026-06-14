use crate::config::VirtualKeyConfig;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::sync::RwLock;

/// Runtime state for a virtual key.
#[derive(Debug, Clone)]
pub struct VirtualKeyState {
    pub key: String,
    pub label: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub max_rpm: Option<u32>,
    pub max_budget: Option<f64>,
    pub request_count: u64,
    pub minute_count: u32,
    pub last_minute_reset: u64,
}

impl From<&VirtualKeyConfig> for VirtualKeyState {
    fn from(cfg: &VirtualKeyConfig) -> Self {
        Self {
            key: cfg.key.clone(),
            label: cfg.label.clone(),
            provider: cfg.provider.clone(),
            model: cfg.model.clone(),
            max_rpm: cfg.max_rpm,
            max_budget: cfg.max_budget,
            request_count: 0,
            minute_count: 0,
            last_minute_reset: now_secs(),
        }
    }
}

pub struct VirtualKeyStore {
    keys: RwLock<HashMap<String, VirtualKeyState>>,
}

impl Default for VirtualKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualKeyStore {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
        }
    }

    /// Load virtual keys from config.
    pub fn load_from_config(&self, configs: &[VirtualKeyConfig]) {
        let mut keys = self.keys.write().unwrap();
        for cfg in configs {
            keys.insert(cfg.key.clone(), VirtualKeyState::from(cfg));
        }
    }

    /// Validate a virtual key and return its state.
    /// Also checks rate limits.
    pub fn validate(&self, key: &str) -> Result<VirtualKeyState> {
        let mut keys = self.keys.write().unwrap();
        let state = keys
            .get_mut(key)
            .ok_or_else(|| anyhow::anyhow!("Invalid virtual key"))?;

        // Reset minute counter if a new minute has started
        let now = now_secs();
        if now - state.last_minute_reset >= 60 {
            state.minute_count = 0;
            state.last_minute_reset = now;
        }

        // Check rate limit
        if let Some(max_rpm) = state.max_rpm {
            if state.minute_count >= max_rpm {
                bail!("Rate limit exceeded for virtual key");
            }
        }

        // Increment counters
        state.minute_count += 1;
        state.request_count += 1;

        Ok(state.clone())
    }

    /// Create a new virtual key.
    pub fn create(&self, cfg: VirtualKeyConfig) -> Result<()> {
        let mut keys = self.keys.write().unwrap();
        if keys.contains_key(&cfg.key) {
            bail!("Virtual key '{}' already exists", cfg.key);
        }
        keys.insert(cfg.key.clone(), VirtualKeyState::from(&cfg));
        Ok(())
    }

    /// Delete a virtual key.
    pub fn delete(&self, key: &str) -> Result<()> {
        let mut keys = self.keys.write().unwrap();
        if keys.remove(key).is_none() {
            bail!("Virtual key '{}' not found", key);
        }
        Ok(())
    }

    /// List all virtual keys (masked for display).
    pub fn list(&self) -> Vec<VirtualKeySummary> {
        let keys = self.keys.read().unwrap();
        keys.values()
            .map(|s| VirtualKeySummary {
                key: s.key.clone(),
                masked_key: mask_key(&s.key),
                label: s.label.clone(),
                provider: s.provider.clone(),
                model: s.model.clone(),
                max_rpm: s.max_rpm,
                request_count: s.request_count,
            })
            .collect()
    }

    /// Get usage for a specific key.
    pub fn usage(&self, key: &str) -> Option<VirtualKeyUsage> {
        let keys = self.keys.read().unwrap();
        keys.get(key).map(|s| VirtualKeyUsage {
            key: mask_key(&s.key),
            request_count: s.request_count,
            minute_count: s.minute_count,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VirtualKeySummary {
    pub key: String,
    pub masked_key: String,
    pub label: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub max_rpm: Option<u32>,
    pub request_count: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VirtualKeyUsage {
    pub key: String,
    pub request_count: u64,
    pub minute_count: u32,
}

/// Mask a key for display: "vk-weft-abcdef" -> "vk-weft-abc***"
fn mask_key(key: &str) -> String {
    if key.len() <= 10 {
        return format!("{}***", &key[..key.len().min(4)]);
    }
    format!("{}***", &key[..key.len() - 3])
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VirtualKeyConfig;

    fn make_vk(key: &str) -> VirtualKeyConfig {
        VirtualKeyConfig {
            key: key.into(),
            label: Some("test".into()),
            provider: Some("openrouter".into()),
            model: Some("claude-sonnet-4".into()),
            max_rpm: Some(10),
            max_budget: None,
        }
    }

    #[test]
    fn test_validate_ok() {
        let store = VirtualKeyStore::new();
        store.create(make_vk("vk-test-001")).unwrap();
        let state = store.validate("vk-test-001").unwrap();
        assert_eq!(state.provider, Some("openrouter".into()));
        assert_eq!(state.request_count, 1);
    }

    #[test]
    fn test_validate_invalid_key() {
        let store = VirtualKeyStore::new();
        assert!(store.validate("vk-nonexistent").is_err());
    }

    #[test]
    fn test_rate_limit() {
        let store = VirtualKeyStore::new();
        let mut cfg = make_vk("vk-rate-test");
        cfg.max_rpm = Some(2);
        store.create(cfg).unwrap();

        store.validate("vk-rate-test").unwrap();
        store.validate("vk-rate-test").unwrap();
        // Third should fail
        assert!(store.validate("vk-rate-test").is_err());
    }

    #[test]
    fn test_create_duplicate() {
        let store = VirtualKeyStore::new();
        store.create(make_vk("vk-dup")).unwrap();
        assert!(store.create(make_vk("vk-dup")).is_err());
    }

    #[test]
    fn test_delete() {
        let store = VirtualKeyStore::new();
        store.create(make_vk("vk-del")).unwrap();
        store.delete("vk-del").unwrap();
        assert!(store.validate("vk-del").is_err());
    }

    #[test]
    fn test_list_masks_keys() {
        let store = VirtualKeyStore::new();
        store.create(make_vk("vk-weft-abcdef")).unwrap();
        let list = store.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].key, "vk-weft-abcdef");
        assert!(list[0].masked_key.contains("***"));
        assert!(!list[0].masked_key.contains("abcdef"));
    }
}
