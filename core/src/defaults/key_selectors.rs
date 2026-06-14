use crate::layers::key_selector::{ApiKeyState, KeySelectorLayer};
use anyhow::{bail, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Round-robin key selection.
pub struct RoundRobinSelector {
    counters: Mutex<HashMap<String, AtomicUsize>>,
}

impl Default for RoundRobinSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl RoundRobinSelector {
    pub fn new() -> Self {
        Self {
            counters: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl KeySelectorLayer for RoundRobinSelector {
    async fn select(&self, provider: &str, keys: &[ApiKeyState]) -> Result<usize> {
        let available: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(_, k)| !k.failed)
            .map(|(i, _)| i)
            .collect();

        if available.is_empty() {
            bail!("No available keys for provider '{}'", provider);
        }

        let mut counters = self.counters.lock().unwrap();
        let counter = counters
            .entry(provider.to_string())
            .or_insert_with(|| AtomicUsize::new(0));
        let idx = counter.fetch_add(1, Ordering::Relaxed) % available.len();
        Ok(available[idx])
    }

    fn mark_failed(&self, _provider: &str, _index: usize) {
        // State is tracked externally via ApiKeyState.failed
    }

    fn mark_success(&self, _provider: &str, _index: usize) {}
}

/// Failover: always pick the first non-failed key.
pub struct FailoverSelector;

#[async_trait]
impl KeySelectorLayer for FailoverSelector {
    async fn select(&self, provider: &str, keys: &[ApiKeyState]) -> Result<usize> {
        keys.iter()
            .enumerate()
            .find(|(_, k)| !k.failed)
            .map(|(i, _)| i)
            .ok_or_else(|| anyhow::anyhow!("No available keys for provider '{}'", provider))
    }

    fn mark_failed(&self, _provider: &str, _index: usize) {}
    fn mark_success(&self, _provider: &str, _index: usize) {}
}

/// Random key selection.
pub struct RandomSelector;

#[async_trait]
impl KeySelectorLayer for RandomSelector {
    async fn select(&self, provider: &str, keys: &[ApiKeyState]) -> Result<usize> {
        let available: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(_, k)| !k.failed)
            .map(|(i, _)| i)
            .collect();

        if available.is_empty() {
            bail!("No available keys for provider '{}'", provider);
        }

        use rand::Rng;
        let idx = rand::thread_rng().gen_range(0..available.len());
        Ok(available[idx])
    }

    fn mark_failed(&self, _provider: &str, _index: usize) {}
    fn mark_success(&self, _provider: &str, _index: usize) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keys() -> Vec<ApiKeyState> {
        vec![
            ApiKeyState {
                index: 0,
                value: "sk-aaa".into(),
                label: None,
                failed: false,
                usage_count: 0,
            },
            ApiKeyState {
                index: 1,
                value: "sk-bbb".into(),
                label: None,
                failed: false,
                usage_count: 0,
            },
        ]
    }

    #[tokio::test]
    async fn test_round_robin() {
        let sel = RoundRobinSelector::new();
        let keys = make_keys();
        let a = sel.select("p", &keys).await.unwrap();
        let b = sel.select("p", &keys).await.unwrap();
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn test_failover_picks_first() {
        let sel = FailoverSelector;
        let keys = make_keys();
        let idx = sel.select("p", &keys).await.unwrap();
        assert_eq!(idx, 0);
    }

    #[tokio::test]
    async fn test_failover_skips_failed() {
        let sel = FailoverSelector;
        let mut keys = make_keys();
        keys[0].failed = true;
        let idx = sel.select("p", &keys).await.unwrap();
        assert_eq!(idx, 1);
    }

    #[tokio::test]
    async fn test_all_failed_errors() {
        let sel = FailoverSelector;
        let mut keys = make_keys();
        keys[0].failed = true;
        keys[1].failed = true;
        assert!(sel.select("p", &keys).await.is_err());
    }
}
