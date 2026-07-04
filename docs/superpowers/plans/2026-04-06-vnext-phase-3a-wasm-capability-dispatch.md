# WEFT vNext Phase 3A WASM Capability Dispatch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade `capability.call` from a registry-only dispatch stub into a real WASM-backed capability dispatch path for currently loaded WASM providers, while keeping the existing plugin runtime and `/api/plugins/{plugin_name}/call` behavior intact.

**Architecture:** Reuse the current capability registry, plugin binding data, `WasmHandle`, and `handle_ws_message` convention already used by `/api/plugins/{plugin_name}/call`. Phase 3A does not implement native providers or host adapter execution; it routes capability calls to resolved WASM providers via a shared dispatch helper, using `{ action, data }` envelopes and preserving current plugin runtime semantics.

**Tech Stack:** Rust workspace (`core` crate), Axum, existing `WasmHandle`, existing plugin WS dispatch convention, integration tests in `core/tests/integration.rs`

---

## File Structure

### Existing files to modify

- `core/src/api/capabilities.rs`
  - Replace registry-only stub with real WASM dispatch flow.
- `core/src/api/plugin_ws.rs`
  - Extract or reuse dispatch helper so capability dispatch and plugin dispatch share the same path.
- `core/src/app/registry.rs`
  - Extend registry metadata only if needed for action routing clarity.
- `core/src/tests/integration.rs`
  - Add tests for capability dispatch selection and error behavior.
- `docs/architecture/vNext/05-binding-and-resolution.md`
  - Reflect that Phase 3A upgrades `capability.call` from stub to WASM dispatch.

### New files to create

- None required unless helper extraction becomes too large; if needed, create `core/src/api/capability_dispatch.rs`.

### Existing files to inspect

- `core/src/api/capabilities.rs`
- `core/src/api/plugin_ws.rs`
- `core/src/plugin/bridge.rs`
- `core/src/api/openai_compat.rs`
- `core/plugins/sdk/src/lib.rs`
- `core/plugins/official/memory/src/lib.rs`
- `core/plugins/official/channels/src/lib.rs`
- `docs/architecture/vNext/00-overview.md`

---

### Task 1: Formalize Phase 3A dispatch envelope contract

**Files:**
- Modify: `docs/architecture/vNext/05-binding-and-resolution.md`
- Modify: `core/src/api/capabilities.rs`
- Test: none

- [ ] **Step 1: Document the Phase 3A dispatch envelope**

Append a short subsection to `docs/architecture/vNext/05-binding-and-resolution.md` describing that Phase 3A capability dispatch uses the existing plugin envelope:

```markdown
### Phase 3A WASM capability dispatch

In Phase 3A, `capability.call(name, payload)` for WASM-backed providers routes through the existing `handle_ws_message` plugin convention.

Core builds an envelope of:

```json
{
  "action": "<capability action>",
  "data": { ...payload... }
}
```

Provider selection still comes from capability registry and app bindings. Native dispatch remains out of scope for Phase 3A.
```
```

- [ ] **Step 2: Add a small internal note in `core/src/api/capabilities.rs`**

Above `capability_call`, add this comment block:

```rust
/// Phase 3A: capability calls dispatch to WASM providers using the existing
/// `handle_ws_message` convention and `{ action, data }` envelope.
/// Native provider execution remains out of scope for this phase.
```

- [ ] **Step 3: Verify doc update is saved**

Run:

```powershell
git diff -- docs/architecture/vNext/05-binding-and-resolution.md core/src/api/capabilities.rs
```

Expected: shows the new Phase 3A dispatch contract text and the capability_call note.

### Task 2: Extract reusable WASM plugin payload dispatcher

**Files:**
- Modify: `core/src/api/plugin_ws.rs`
- Modify: `core/src/api/capabilities.rs`
- Test: `core/tests/integration.rs`

- [ ] **Step 1: Write the failing dispatch-helper test**

Add this test to `core/tests/integration.rs`:

```rust
#[tokio::test]
async fn test_capability_call_returns_not_found_for_unknown_capability() {
    let app = build_router(test_state());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/unknown.capability/call")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"action":"noop","data":{}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run test to verify existing behavior still passes**

Run:

```powershell
cargo test -p weft-core test_capability_call_returns_not_found_for_unknown_capability -- --nocapture
```

Expected: PASS with current capability API.

- [ ] **Step 3: Extract the plugin dispatch helper from `plugin_ws.rs`**

In `core/src/api/plugin_ws.rs`, change:

```rust
async fn dispatch_to_plugin(
```

to:

```rust
pub async fn dispatch_to_wasm_plugin_payload(
```

and keep the body unchanged.

Then update the local caller inside `dispatch_plugin_payload` from:

```rust
dispatch_to_plugin(plugin_name, &payload, &wasm_handle).await
```

to:

```rust
dispatch_to_wasm_plugin_payload(plugin_name, &payload, &wasm_handle).await
```

- [ ] **Step 4: Reuse that helper from `core/src/api/capabilities.rs`**

Add import at top:

```rust
use crate::api::plugin_ws::dispatch_to_wasm_plugin_payload;
```

Do not yet change behavior in this task; just wire the helper import and ensure both modules compile.

- [ ] **Step 5: Run focused compile/test check**

Run:

```powershell
cargo test -p weft-core test_capability_call_returns_not_found_for_unknown_capability -- --nocapture
```

Expected: PASS

### Task 3: Upgrade capability.call from stub to real WASM dispatch

**Files:**
- Modify: `core/src/api/capabilities.rs`
- Test: `core/tests/integration.rs`

- [ ] **Step 1: Write the failing provider-selection test**

Add this test:

```rust
#[tokio::test]
async fn test_capability_call_prefers_binding_provider_when_present() {
    let state = test_state();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "memory.backend".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "memory.backend".into(),
                providers: vec![
                    weft_core::app::CapabilityProviderRecord {
                        provider: "memory".into(),
                        runtime: "wasm".into(),
                    },
                    weft_core::app::CapabilityProviderRecord {
                        provider: "other-memory".into(),
                        runtime: "wasm".into(),
                    },
                ],
                bindings: vec![weft_core::app::CapabilityBindingRecord {
                    app: "weft-claw".into(),
                    provider: "memory".into(),
                }],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/memory.backend/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "action": "recall",
                        "data": {"agent":"tester","query":"hello","limit":1}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let json = response_json(resp).await;
    assert_eq!(json["provider"], "memory");
}
```

- [ ] **Step 2: Write the failing no-wasm-runtime test**

Add this test:

```rust
#[tokio::test]
async fn test_capability_call_returns_bad_gateway_when_wasm_runtime_missing() {
    let state = test_state();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "channels.bridge".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "channels.bridge".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "channels".into(),
                    runtime: "wasm".into(),
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
                .uri("/api/capabilities/channels.bridge/call")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"action":"send","data":{}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}
```

- [ ] **Step 3: Replace the registry-only stub with WASM dispatch path**

In `core/src/api/capabilities.rs`, replace the body of `capability_call` with this logic:

```rust
    let registry = state.capability_registry.read().await;
    let capability = if let Some(capability) = registry.get(&name) {
        capability.clone()
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Capability '{}' not found", name)
            })),
        ));
    };
    drop(registry);

    let selected_provider = payload
        .get("provider")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| capability.bindings.first().map(|binding| binding.provider.clone()))
        .or_else(|| capability.providers.first().map(|provider| provider.provider.clone()));

    let Some(provider) = selected_provider else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Capability '{}' has no available provider", name)
            })),
        ));
    };

    let action = payload
        .get("action")
        .and_then(|value| value.as_str())
        .unwrap_or("call");
    let data = payload
        .get("data")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

    let envelope = serde_json::json!({
        "action": action,
        "data": data,
    });

    let wasm_handle = state.wasm_handle.read().await.clone();
    let response = dispatch_to_wasm_plugin_payload(&provider, &envelope, &wasm_handle).await;

    if response.get("error").is_some() {
        Err((StatusCode::BAD_GATEWAY, Json(response)))
    } else {
        Ok(Json(serde_json::json!({
            "capability": name,
            "provider": provider,
            "response": response,
            "status": "executed",
            "mode": "wasm-phase"
        })))
    }
```

- [ ] **Step 4: Run focused tests**

Run:

```powershell
cargo test -p weft-core test_capability_call_prefers_binding_provider_when_present -- --nocapture
cargo test -p weft-core test_capability_call_returns_bad_gateway_when_wasm_runtime_missing -- --nocapture
```

Expected: PASS

### Task 4: Add realistic call-shape tests for current providers

**Files:**
- Modify: `core/tests/integration.rs`

- [ ] **Step 1: Add a registry-backed payload-shape test**

Add this test:

```rust
#[tokio::test]
async fn test_capability_call_returns_selected_provider_in_response_wrapper() {
    let state = test_state();
    {
        let mut registry = state.capability_registry.write().await;
        registry.insert(
            "tool.registry".into(),
            weft_core::app::CapabilityRegistryEntry {
                capability: "tool.registry".into(),
                providers: vec![weft_core::app::CapabilityProviderRecord {
                    provider: "skills".into(),
                    runtime: "wasm".into(),
                }],
                bindings: vec![weft_core::app::CapabilityBindingRecord {
                    app: "weft-claw".into(),
                    provider: "skills".into(),
                }],
            },
        );
    }

    let app = build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/capabilities/tool.registry/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "action": "list_available",
                        "data": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let json = response_json(resp).await;
    assert_eq!(json["provider"], "skills");
}
```

- [ ] **Step 2: Run focused test**

Run:

```powershell
cargo test -p weft-core test_capability_call_returns_selected_provider_in_response_wrapper -- --nocapture
```

Expected: PASS

### Task 5: Full verification and branch review

**Files:**
- Modify: none

- [ ] **Step 1: Run targeted Phase 3A tests**

Run:

```powershell
cargo test -p weft-core test_list_capabilities_returns_registry_entries -- --nocapture
cargo test -p weft-core test_get_single_capability_returns_not_found_for_missing_capability -- --nocapture
cargo test -p weft-core test_capability_call_prefers_binding_provider_when_present -- --nocapture
cargo test -p weft-core test_capability_call_returns_bad_gateway_when_wasm_runtime_missing -- --nocapture
cargo test -p weft-core test_capability_call_returns_selected_provider_in_response_wrapper -- --nocapture
```

Expected: all PASS

- [ ] **Step 2: Run full core suite**

Run:

```powershell
cargo test -p weft-core
```

Expected: PASS

- [ ] **Step 3: Run full build**

Run:

```powershell
cargo build
```

Expected: PASS

- [ ] **Step 4: Trigger review gate**

After tests pass, run the post-implementation review skill in the controller session and inspect any findings before moving to the next phase.
