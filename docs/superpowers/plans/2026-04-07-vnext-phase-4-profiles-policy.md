# WEFT vNext Phase 4: Profiles & Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Safe/Developer/Trusted profile model and Core Policy enforcement, so that `capability.call` respects the active profile and capabilities are gated accordingly.

**Architecture:** Per `docs/architecture/vNext/06-profiles-and-policy.md`, users choose a profile (Safe/Developer/Trusted), and Core decides what capabilities are actually granted. The profile is read from instance `config.toml` (`app_runtime.profile`). Policy enforcement happens at `capability.call` time.

**Tech Stack:** Rust workspace (`core` crate), Axum, existing AppState, existing capability registry, integration tests in `core/tests/integration.rs`

---

## File Structure

### Existing files to modify

- `core/src/app/mod.rs` — re-export new policy module
- `core/src/app/config.rs` — add `AppProfile` enum, parse from string
- `core/src/app/state.rs` — add `profile` field to `ResolvedApp`
- `core/src/api/openai_compat.rs` — add `active_profile` and `core_policy` to `AppState`
- `core/src/api/capabilities.rs` — add policy check before dispatch
- `core/src/api/mod.rs` — register new profile/policy routes
- `core/src/main.rs` — load profile from app config at startup, build policy
- `core/tests/integration.rs` — add profile/policy tests

### New files to create

- `core/src/app/policy.rs` — `AppProfile` enum, `CorePolicy`, `PolicyDecision`, enforcement logic

---

### Task 1: Add AppProfile enum and CorePolicy struct

**Files:**
- Create: `core/src/app/policy.rs`
- Modify: `core/src/app/mod.rs`
- Test: `core/tests/integration.rs`

- [ ] **Step 1: Create `core/src/app/policy.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime profile that determines capability boundaries.
/// Users pick a profile; Core decides what's granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AppProfile {
    #[default]
    Safe,
    Developer,
    Trusted,
}

impl AppProfile {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "developer" | "dev" => Self::Developer,
            "trusted" | "trust" | "full" => Self::Trusted,
            _ => Self::Safe,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Developer => "developer",
            Self::Trusted => "trusted",
        }
    }
}

/// What the policy decided for a specific capability call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: String,
}

/// Capability-level policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPolicyRule {
    /// Minimum profile required to use this capability.
    pub min_profile: AppProfile,
    /// Optional description of what this rule controls.
    pub description: String,
}

/// Core Policy: maps capability names to their access rules.
/// Capabilities not listed are allowed by default (open-by-default for WASM capabilities).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorePolicy {
    pub rules: HashMap<String, CapabilityPolicyRule>,
}

impl CorePolicy {
    /// Build the default policy with sensible defaults for known capability groups.
    pub fn default_policy() -> Self {
        let mut rules = HashMap::new();

        // Files capability requires at least Developer
        rules.insert("core.files".into(), CapabilityPolicyRule {
            min_profile: AppProfile::Developer,
            description: "File system read/write operations".into(),
        });

        // Execution capability requires at least Developer
        rules.insert("core.execution".into(), CapabilityPolicyRule {
            min_profile: AppProfile::Developer,
            description: "Command execution and process management".into(),
        });

        // Native execution requires Trusted
        rules.insert("core.native_execution".into(), CapabilityPolicyRule {
            min_profile: AppProfile::Trusted,
            description: "Native/dll/so plugin execution".into(),
        });

        // WASM capabilities (agent, tool, memory, communication) are open at Safe
        // They don't need explicit rules since default is allow.

        Self { rules }
    }

    /// Check whether a capability call is allowed under the given profile.
    pub fn check(&self, capability: &str, profile: AppProfile) -> PolicyDecision {
        if let Some(rule) = self.rules.get(capability) {
            let allowed = profile_rank(profile) >= profile_rank(rule.min_profile);
            PolicyDecision {
                allowed,
                reason: if allowed {
                    format!("Profile '{}' meets minimum '{}' for '{}'",
                        profile.as_str(), rule.min_profile.as_str(), capability)
                } else {
                    format!("Profile '{}' does not meet minimum '{}' required for '{}'",
                        profile.as_str(), rule.min_profile.as_str(), capability)
                },
            }
        } else {
            // No rule = allowed by default
            PolicyDecision {
                allowed: true,
                reason: format!("No policy restriction on '{}'", capability),
            }
        }
    }
}

/// Numeric rank for profile comparison.
fn profile_rank(profile: AppProfile) -> u8 {
    match profile {
        AppProfile::Safe => 0,
        AppProfile::Developer => 1,
        AppProfile::Trusted => 2,
    }
}
```

- [ ] **Step 2: Update `core/src/app/mod.rs` to re-export policy module**

Add `pub mod policy;` and re-export key types:

```rust
pub mod policy;
pub use policy::{AppProfile, CorePolicy, PolicyDecision};
```

- [ ] **Step 3: Add unit tests for policy logic**

Add these tests to `core/tests/integration.rs`:

```rust
#[test]
fn test_app_profile_from_str_loose_parses_known_profiles() {
    use weft_core::app::AppProfile;
    assert_eq!(AppProfile::from_str_loose("safe"), AppProfile::Safe);
    assert_eq!(AppProfile::from_str_loose("developer"), AppProfile::Developer);
    assert_eq!(AppProfile::from_str_loose("dev"), AppProfile::Developer);
    assert_eq!(AppProfile::from_str_loose("trusted"), AppProfile::Trusted);
    assert_eq!(AppProfile::from_str_loose("trust"), AppProfile::Trusted);
    assert_eq!(AppProfile::from_str_loose("unknown"), AppProfile::Safe);
}

#[test]
fn test_core_policy_allows_unrestricted_capabilities() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("agent.capability", AppProfile::Safe);
    assert!(decision.allowed);
}

#[test]
fn test_core_policy_blocks_files_under_safe_profile() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("core.files", AppProfile::Safe);
    assert!(!decision.allowed);
}

#[test]
fn test_core_policy_allows_files_under_developer_profile() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("core.files", AppProfile::Developer);
    assert!(decision.allowed);
}

#[test]
fn test_core_policy_blocks_native_execution_under_developer_profile() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("core.native_execution", AppProfile::Developer);
    assert!(!decision.allowed);
}

#[test]
fn test_core_policy_allows_native_execution_under_trusted_profile() {
    use weft_core::app::{AppProfile, CorePolicy};
    let policy = CorePolicy::default_policy();
    let decision = policy.check("core.native_execution", AppProfile::Trusted);
    assert!(decision.allowed);
}
```

- [ ] **Step 4: Run tests**

```powershell
cargo test -p weft-core test_app_profile ; cargo test -p weft-core test_core_policy
```

Expected: all PASS

### Task 2: Integrate profile and policy into AppState and startup

**Files:**
- Modify: `core/src/api/openai_compat.rs`
- Modify: `core/src/main.rs`
- Modify: `core/src/app/state.rs`
- Test: `core/tests/integration.rs`

- [ ] **Step 1: Add `active_profile` and `core_policy` to `AppState`**

In `core/src/api/openai_compat.rs`, add two new fields to `AppState`:

```rust
pub active_profile: Arc<RwLock<crate::app::AppProfile>>,
pub core_policy: Arc<crate::app::CorePolicy>,
```

- [ ] **Step 2: Add `profile` field to `ResolvedApp`**

In `core/src/app/state.rs`, add to `ResolvedApp`:

```rust
pub profile: crate::app::policy::AppProfile,
```

- [ ] **Step 3: Update `test_state()` in integration tests**

Add the new fields to `test_state()`:

```rust
active_profile: Arc::new(RwLock::new(weft_core::app::AppProfile::Developer)),
core_policy: Arc::new(weft_core::app::CorePolicy::default_policy()),
```

- [ ] **Step 4: Update `main.rs` to load profile from first resolved app and build policy**

After resolving apps, determine the active profile from the first resolved app's config:

```rust
// Determine active profile from first resolved app config
let active_profile = {
    let mut profile = weft_core::app::AppProfile::Safe;
    for app in resolved_apps.values() {
        if app.status == ResolvedAppStatus::Resolved {
            if let Some(ref config_path) = app.config_path {
                let app_dir = std::path::Path::new(config_path).parent().unwrap_or(std::path::Path::new("."));
                if let Ok(app_config) = weft_core::app::load_app_config(app_dir) {
                    profile = weft_core::app::AppProfile::from_str_loose(&app_config.app_runtime.profile);
                    break;
                }
            }
        }
    }
    profile
};
let core_policy = weft_core::app::CorePolicy::default_policy();
```

Then add to AppState construction:

```rust
active_profile: Arc::new(RwLock::new(active_profile)),
core_policy: Arc::new(core_policy),
```

- [ ] **Step 5: Run build and tests**

```powershell
cargo build -p weft-core ; cargo test -p weft-core
```

Expected: PASS

### Task 3: Add policy enforcement to capability.call

**Files:**
- Modify: `core/src/api/capabilities.rs`
- Test: `core/tests/integration.rs`

- [ ] **Step 1: Add policy check at the top of `capability_call`**

After the registry lookup (after getting the capability entry), add:

```rust
// Policy enforcement: check if the active profile allows this capability
{
    let profile = *state.active_profile.read().await;
    let decision = state.core_policy.check(&name, profile);
    if !decision.allowed {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": format!("Policy denied: {}", decision.reason),
                "capability": name,
                "profile": profile.as_str(),
            })),
        ));
    }
}
```

- [ ] **Step 2: Add integration test for policy enforcement**

```rust
#[tokio::test]
async fn test_capability_call_returns_forbidden_when_policy_denies() {
    let state = test_state();
    // Override profile to Safe
    *state.active_profile.write().await = weft_core::app::AppProfile::Safe;
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "native".into(),
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.files/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"list","data":{"path":"."}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let json = response_json(resp).await;
    assert!(json["error"].as_str().unwrap().contains("Policy denied"));
}

#[tokio::test]
async fn test_capability_call_passes_policy_when_profile_sufficient() {
    let state = test_state();
    // Profile is Developer by default in test_state
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "core.files".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "core.files".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "native".into(),
                }],
                bindings: vec![],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/core.files/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"list","data":{"path":"."}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should pass policy (Developer >= Developer), but may fail at dispatch (no WASM for core provider)
    // The key assertion is that it's NOT FORBIDDEN
    assert_ne!(resp.status(), StatusCode::FORBIDDEN);
}
```

- [ ] **Step 3: Run focused tests**

```powershell
cargo test -p weft-core test_capability_call_returns_forbidden ; cargo test -p weft-core test_capability_call_passes_policy
```

Expected: PASS

### Task 4: Add profile and policy API endpoints

**Files:**
- Create: `core/src/api/profile.rs`
- Modify: `core/src/api/mod.rs`
- Test: `core/tests/integration.rs`

- [ ] **Step 1: Create `core/src/api/profile.rs`**

```rust
use crate::api::openai_compat::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

/// GET /api/profile — return current active profile
pub async fn get_profile(State(state): State<AppState>) -> Json<serde_json::Value> {
    let profile = state.active_profile.read().await;
    Json(serde_json::json!({
        "profile": profile.as_str(),
    }))
}

/// PUT /api/profile — switch active profile
pub async fn set_profile(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let profile_str = body["profile"]
        .as_str()
        .ok_or_else(|| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Missing 'profile' field"})),
        ))?;

    let new_profile = crate::app::AppProfile::from_str_loose(profile_str);
    *state.active_profile.write().await = new_profile;

    Ok(Json(serde_json::json!({
        "profile": new_profile.as_str(),
        "status": "switched",
    })))
}

/// GET /api/policy — return current policy rules
pub async fn get_policy(State(state): State<AppState>) -> Json<serde_json::Value> {
    let profile = state.active_profile.read().await;
    Json(serde_json::json!({
        "profile": profile.as_str(),
        "rules": state.core_policy.rules,
    }))
}
```

- [ ] **Step 2: Register routes in `core/src/api/mod.rs`**

Add `pub mod profile;` and register routes:

```rust
.route("/api/profile", get(profile::get_profile))
.route("/api/profile", axum::routing::put(profile::set_profile))
.route("/api/policy", get(profile::get_policy))
```

- [ ] **Step 3: Add integration tests for profile API**

```rust
#[tokio::test]
async fn test_get_profile_returns_current_profile() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/profile")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert_eq!(json["profile"], "developer");
}

#[tokio::test]
async fn test_get_policy_returns_rules() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/policy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    assert!(json["rules"].is_object());
    assert!(json["rules"]["core.files"].is_object());
}
```

- [ ] **Step 4: Run all tests and build**

```powershell
cargo test -p weft-core ; cargo build
```

Expected: all PASS

### Task 5: Full verification

- [ ] **Step 1: Run all Phase 4 tests**

```powershell
cargo test -p weft-core test_app_profile ; cargo test -p weft-core test_core_policy ; cargo test -p weft-core test_capability_call_returns_forbidden ; cargo test -p weft-core test_capability_call_passes_policy ; cargo test -p weft-core test_get_profile ; cargo test -p weft-core test_get_policy
```

Expected: all PASS

- [ ] **Step 2: Run full suite**

```powershell
cargo test -p weft-core ; cargo build
```

Expected: PASS
