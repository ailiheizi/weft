//! Per-package circuit breaker for WASM call fault isolation (A2).
//!
//! A repeatedly-trapping or panicking WASM package should not be invoked
//! forever — each failed call costs a lock, an instantiation attempt, and (for
//! crash loops) log spam. This breaker tracks recent call outcomes per package
//! and, once failures exceed a threshold within a sliding window, trips to
//! `Open`: subsequent calls are rejected immediately without touching the
//! plugin. After a cooldown it moves to `HalfOpen` and lets a single probe
//! through; success closes the breaker, failure re-opens it.
//!
//! "Failure" here means the host-side `Plugin::call` returned `Err` (a trap,
//! panic, or instantiation error) — NOT a well-formed `{"error":...}` JSON
//! payload, which is a normal business-level result the guest chose to return.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Tunable thresholds. Defaults are deliberately lenient so healthy packages
/// with occasional hiccups are never tripped.
#[derive(Debug, Clone, Copy)]
pub struct BreakerConfig {
    /// Sliding window size (most recent N outcomes considered).
    pub window: usize,
    /// Number of failures within the window required to trip Open.
    pub failure_threshold: usize,
    /// How long to stay Open before allowing a HalfOpen probe.
    pub cooldown: Duration,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            window: 10,
            failure_threshold: 5,
            cooldown: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
struct Entry {
    state: State,
    /// Recent outcomes: true = failure, false = success.
    window: VecDeque<bool>,
    /// When the breaker last tripped Open (for cooldown).
    opened_at: Option<Instant>,
}

impl Entry {
    fn new() -> Self {
        Self {
            state: State::Closed,
            window: VecDeque::new(),
            opened_at: None,
        }
    }

    fn failures(&self) -> usize {
        self.window.iter().filter(|&&f| f).count()
    }
}

/// Thread-safe, clonable per-package circuit breaker.
#[derive(Clone)]
pub struct CircuitBreaker {
    config: BreakerConfig,
    entries: Arc<Mutex<HashMap<String, Entry>>>,
}

impl CircuitBreaker {
    pub fn new(config: BreakerConfig) -> Self {
        Self {
            config,
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns `Err(reason)` if the package is currently tripped Open and still
    /// within its cooldown — the caller should reject the call without invoking
    /// the plugin. Otherwise `Ok(())` (Closed or a HalfOpen probe is allowed).
    ///
    /// `now` is injected so the logic is deterministic in tests.
    pub fn check_at(&self, package: &str, now: Instant) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.entry(package.to_string()).or_insert_with(Entry::new);

        if entry.state == State::Open {
            let cooled = entry
                .opened_at
                .map(|t| now.duration_since(t) >= self.config.cooldown)
                .unwrap_or(true);
            if cooled {
                // Allow a single probe through.
                entry.state = State::HalfOpen;
                return Ok(());
            }
            return Err(format!(
                "package '{package}' circuit is open (too many recent failures); \
                 rejecting call until cooldown elapses"
            ));
        }

        Ok(())
    }

    /// Records the outcome of a call and updates the breaker state.
    pub fn record_at(&self, package: &str, success: bool, now: Instant) {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.entry(package.to_string()).or_insert_with(Entry::new);

        match entry.state {
            State::HalfOpen => {
                if success {
                    // Probe succeeded → close and reset.
                    entry.state = State::Closed;
                    entry.window.clear();
                    entry.opened_at = None;
                } else {
                    // Probe failed → re-open, restart cooldown.
                    entry.state = State::Open;
                    entry.opened_at = Some(now);
                }
            }
            State::Closed => {
                entry.window.push_back(!success);
                while entry.window.len() > self.config.window {
                    entry.window.pop_front();
                }
                if entry.failures() >= self.config.failure_threshold {
                    entry.state = State::Open;
                    entry.opened_at = Some(now);
                }
            }
            State::Open => {
                // Outcome arriving while Open (race with a probe); ignore.
            }
        }
    }

    /// True if the package is currently tripped Open (diagnostic / status use).
    pub fn is_open(&self, package: &str) -> bool {
        self.entries
            .lock()
            .unwrap()
            .get(package)
            .map(|e| e.state == State::Open)
            .unwrap_or(false)
    }

    /// Manually reset a package's breaker (e.g. after a reload/upgrade).
    pub fn reset(&self, package: &str) {
        if let Some(entry) = self.entries.lock().unwrap().get_mut(package) {
            entry.state = State::Closed;
            entry.window.clear();
            entry.opened_at = None;
        }
    }

    // Convenience wrappers using the real clock for production call sites.
    pub fn check(&self, package: &str) -> Result<(), String> {
        self.check_at(package, Instant::now())
    }
    pub fn record(&self, package: &str, success: bool) {
        self.record_at(package, success, Instant::now())
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(BreakerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> BreakerConfig {
        BreakerConfig {
            window: 5,
            failure_threshold: 3,
            cooldown: Duration::from_secs(10),
        }
    }

    #[test]
    fn healthy_package_stays_closed() {
        let cb = CircuitBreaker::new(cfg());
        let t = Instant::now();
        for _ in 0..20 {
            assert!(cb.check_at("p", t).is_ok());
            cb.record_at("p", true, t);
        }
        assert!(!cb.is_open("p"));
    }

    #[test]
    fn trips_open_after_threshold_failures() {
        let cb = CircuitBreaker::new(cfg());
        let t = Instant::now();
        for _ in 0..3 {
            assert!(cb.check_at("bad", t).is_ok());
            cb.record_at("bad", false, t);
        }
        assert!(cb.is_open("bad"));
        // Subsequent call rejected within cooldown.
        assert!(cb.check_at("bad", t).is_err());
    }

    #[test]
    fn occasional_failures_below_threshold_stay_closed() {
        let cb = CircuitBreaker::new(cfg());
        let t = Instant::now();
        // window=5, threshold=3: fail 1 in every 4 calls → at most 2 failures
        // in any window of 5, never reaching the threshold.
        for i in 0..15 {
            cb.check_at("p", t).ok();
            let success = i % 4 != 0; // fail on 0,4,8,12 → spaced 4 apart
            cb.record_at("p", success, t);
        }
        assert!(!cb.is_open("p"));
    }

    #[test]
    fn halfopen_probe_success_closes() {
        let cb = CircuitBreaker::new(cfg());
        let t0 = Instant::now();
        for _ in 0..3 {
            cb.check_at("p", t0).ok();
            cb.record_at("p", false, t0);
        }
        assert!(cb.check_at("p", t0).is_err()); // open, within cooldown
        let t1 = t0 + Duration::from_secs(11); // past cooldown
        assert!(cb.check_at("p", t1).is_ok()); // half-open probe allowed
        cb.record_at("p", true, t1); // probe succeeds
        assert!(!cb.is_open("p"));
        assert!(cb.check_at("p", t1).is_ok());
    }

    #[test]
    fn halfopen_probe_failure_reopens() {
        let cb = CircuitBreaker::new(cfg());
        let t0 = Instant::now();
        for _ in 0..3 {
            cb.check_at("p", t0).ok();
            cb.record_at("p", false, t0);
        }
        let t1 = t0 + Duration::from_secs(11);
        assert!(cb.check_at("p", t1).is_ok()); // probe allowed
        cb.record_at("p", false, t1); // probe fails
        assert!(cb.is_open("p"));
        assert!(cb.check_at("p", t1).is_err()); // open again
    }

    #[test]
    fn reset_clears_open_state() {
        let cb = CircuitBreaker::new(cfg());
        let t = Instant::now();
        for _ in 0..3 {
            cb.check_at("p", t).ok();
            cb.record_at("p", false, t);
        }
        assert!(cb.is_open("p"));
        cb.reset("p");
        assert!(!cb.is_open("p"));
        assert!(cb.check_at("p", t).is_ok());
    }
}
