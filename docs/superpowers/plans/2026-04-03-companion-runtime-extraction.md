# Companion Runtime Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move companion product runtime control out of Electron main and into `companion-core`, leaving Electron responsible only for host capabilities such as windowing, renderer forwarding, named-pipe transport, and desktop notifications.

**Architecture:** `companion-core` becomes the single product runtime for a full companion turn: direct or delegate routing, delegate session ownership, agent-runtime execution, completion generation, and UI-semantic event production. Electron no longer decides how a turn runs; it only forwards a turn request into `companion-core`, executes generic host effects returned by the runtime, and forwards resulting events to renderer and bridge endpoints.

**Tech Stack:** Electron main process, TypeScript, Python service `companion-core`, Rust `agent-core` wasm plugin, WEFT-core plugin host, existing live scripts under `weft/scripts`

---

## File Structure

- Modify: `weft/src/shared/types.ts`
  Responsibility: define the runtime-facing result envelope and host effect types shared by Electron main.
- Modify: `weft/src/main/weft-core-api-client.ts`
  Responsibility: expose one high-level API for full companion runtime turns, instead of Electron manually chaining companion and agent calls.
- Modify: `weft-plugins/companion-core/server.py`
  Responsibility: own full companion turn execution, delegate session creation, agent-runtime calls, completion generation, UI event generation, and host effect planning.
- Modify: `weft/src/main/companion-host-service.ts`
  Responsibility: shrink from product orchestrator into a thin host adapter that starts bridge services, forwards turn requests, and applies host effects.
- Modify: `weft/src/main/index.ts`
  Responsibility: wire the thinner host adapter and remove product-logic branching from Electron startup and chat flow.
- Modify: `weft/scripts/test-companion-host-service.ts`
  Responsibility: verify Electron-side host application is thin and effect-driven.
- Modify: `weft/scripts/test-companion-live.ts`
  Responsibility: verify the real runtime path still works for direct and delegate through the full desktop stack.
- Optional create if needed during implementation: `weft/src/main/companion-runtime-host.ts`
  Responsibility: hold the generic host capability applier if `companion-host-service.ts` remains too large after extraction.

### Task 1: Define the Full Companion Runtime Contract

**Files:**
- Modify: `weft/src/shared/types.ts`
- Modify: `weft/src/main/weft-core-api-client.ts`
- Test: `weft/scripts/test-companion-host-service.ts`

- [ ] **Step 1: Write the failing Electron-side contract test**

```ts
import assert from 'node:assert/strict'
import type { CompanionUiEvent } from '../src/shared/types'

type CompanionHostEffect =
  | { kind: 'bridge_structured'; structured: string }
  | { kind: 'renderer_companion'; event: CompanionUiEvent }
  | { kind: 'renderer_tool_result'; name: string; output: string }
  | { kind: 'renderer_chat_done'; content: string }
  | { kind: 'desktop_notification'; title: string; body: string }

const runtimeResult = {
  kind: 'delegate',
  reason: 'search the web for the meaning of chicken',
  assistantReply: 'Chicken usually means the bird, but I will verify it online.',
  completionStructured: '[Think] ||| Let me check that. ||| 我来查一下。',
  toolResults: [{ name: 'web.search', output: 'example result' }],
  uiEvents: [
    {
      mode: 'append',
      reason: 'agent_complete',
      structured: '[Think] ||| Let me check that. ||| 我来查一下。',
      emotion: 'Think',
      voice: 'Let me check that.',
      subtitle: '我来查一下。',
      timestamp: 1,
    },
  ],
  hostEffects: [
    { kind: 'bridge_structured', structured: '[Think] ||| Let me check that. ||| 我来查一下。' },
    { kind: 'renderer_chat_done', content: 'Chicken usually means the bird, but I will verify it online.' },
  ] satisfies CompanionHostEffect[],
}

assert.equal(runtimeResult.hostEffects[0]?.kind, 'bridge_structured')
assert.equal(runtimeResult.uiEvents[0]?.reason, 'agent_complete')
```

- [ ] **Step 2: Run the contract test to verify the current code does not provide this envelope**

Run: `npx tsx scripts/test-companion-host-service.ts`

Expected: FAIL because current `WeftCoreExecutedCompanionTurn` does not yet include `uiEvents` and `hostEffects`, and Electron still executes delegate logic itself.

- [ ] **Step 3: Add shared runtime result and host-effect types**

```ts
export interface CompanionRuntimeHostEffectBridgeStructured {
  kind: 'bridge_structured'
  structured: string
}

export interface CompanionRuntimeHostEffectRendererCompanion {
  kind: 'renderer_companion'
  event: CompanionUiEvent
}

export interface CompanionRuntimeHostEffectRendererToolResult {
  kind: 'renderer_tool_result'
  name: string
  output: string
}

export interface CompanionRuntimeHostEffectRendererChatDone {
  kind: 'renderer_chat_done'
  content: string
}

export interface CompanionRuntimeHostEffectDesktopNotification {
  kind: 'desktop_notification'
  title: string
  body: string
}

export type CompanionRuntimeHostEffect =
  | CompanionRuntimeHostEffectBridgeStructured
  | CompanionRuntimeHostEffectRendererCompanion
  | CompanionRuntimeHostEffectRendererToolResult
  | CompanionRuntimeHostEffectRendererChatDone
  | CompanionRuntimeHostEffectDesktopNotification

export interface CompanionRuntimeTurnResult {
  kind: 'direct' | 'delegate'
  reason: string
  structured: string
  assistantReply?: string
  completionStructured?: string
  toolResults: Array<{ name: string; output: string }>
  uiEvents: CompanionUiEvent[]
  hostEffects: CompanionRuntimeHostEffect[]
}
```

- [ ] **Step 4: Update the WEFT-core client to return the new envelope from `companion-core`**

```ts
async companionExecuteRuntimeTurn(input: {
  provider: string
  model: string
  systemPrompt: string
  content: string
  sessionId?: string
  history?: WeftCoreCompanionContextMessage[]
  sessionContext?: WeftCoreCompanionContextMessage[]
}): Promise<CompanionRuntimeTurnResult> {
  const pluginName = this.requireBoundPlugin('companion_runtime')
  const agentRuntimePlugin = this.requireBoundPlugin('agent_runtime')
  const payload = await this.callPlugin(pluginName, 'execute_runtime_turn', {
    provider: input.provider,
    model: input.model,
    system_prompt: input.systemPrompt,
    content: '',
    content_b64: Buffer.from(input.content, 'utf8').toString('base64'),
    session_id: input.sessionId ?? '',
    agent_runtime_plugin: agentRuntimePlugin,
    history: encodeCompanionContextMessages(input.history),
    session_context: encodeCompanionContextMessages(input.sessionContext),
  }, 120000)

  return payload?.data as CompanionRuntimeTurnResult
}
```

- [ ] **Step 5: Run the contract test again**

Run: `npx tsx scripts/test-companion-host-service.ts`

Expected: PASS for type shape and payload parsing.

- [ ] **Step 6: Commit**

```bash
git add weft/src/shared/types.ts weft/src/main/weft-core-api-client.ts weft/scripts/test-companion-host-service.ts
git commit -m "feat: define companion runtime turn contract"
```

### Task 2: Move Delegate Session Ownership and Delegate Execution into `companion-core`

**Files:**
- Modify: `weft-plugins/companion-core/server.py`
- Test: `weft-plugins/companion-core/server.py`

- [ ] **Step 1: Add failing service-runtime tests for delegate session planning**

```
#[test]
fn execute_runtime_turn_requires_no_electron_side_delegate_call() {
    let result = CompanionRuntimeTurnResult {
        kind: "delegate".to_string(),
        reason: "search the web for the meaning of chicken".to_string(),
        structured: "[Think] ||| Let me check that. ||| 我来查一下。".to_string(),
        assistant_reply: Some("verified answer".to_string()),
        completion_structured: Some("[Think] ||| I checked it. ||| 我查到了。".to_string()),
        delegate_call: None,
        tool_results: vec![ToolResult {
            name: "web.search".to_string(),
            output: "example".to_string(),
        }],
        ui_events: vec![],
        host_effects: vec![],
    };

    assert!(result.delegate_call.is_none());
    assert_eq!(result.assistant_reply.as_deref(), Some("verified answer"));
}

#[test]
fn build_delegate_session_id_is_stable_per_chat_session() {
    assert_eq!(
        build_delegate_session_id("chat-123"),
        "companion-delegate-chat-123".to_string()
    );
}
```

- [ ] **Step 2: Run service-runtime verification to confirm the current plugin still returns `delegate_call` for Electron to execute**

Run: `npx tsx scripts/test-companion-live.ts`

Expected: FAIL because the plugin still returns `delegate_call` and does not own session creation or agent execution.

- [ ] **Step 3: Add plugin-local helpers for delegate session lifecycle and agent invocation**

```
fn build_delegate_session_id(chat_session_id: &str) -> String {
    format!("companion-delegate-{}", chat_session_id.trim())
}

fn ensure_delegate_session(
    agent_runtime_plugin: &str,
    chat_session_id: &str,
) -> Result<String, String> {
    let delegate_session_id = build_delegate_session_id(chat_session_id);
    let now = now_ms();
    let payload = serde_json::json!({
        "id": delegate_session_id,
        "title": format!("Companion Delegate {}", chat_session_id.trim()),
        "persistent": 0,
        "created_at": now,
        "updated_at": now,
    });
    let _ = call_plugin_action(agent_runtime_plugin, "create_session", &payload)?;
    Ok(delegate_session_id)
}

fn run_delegate_session_message(
    agent_runtime_plugin: &str,
    delegate_session_id: &str,
    latest_user_query: &str,
    history: &[SessionContextMessage],
    session_context: &[SessionContextMessage],
    reason: &str,
) -> Result<Value, String> {
    let payload = serde_json::json!({
        "session_id": delegate_session_id,
        "content": latest_user_query.trim(),
        "content_b64": base64::engine::general_purpose::STANDARD.encode(latest_user_query.trim().as_bytes()),
        "delegate_request": {
            "must_act": true,
            "reason": trim_content(reason, 180),
            "latest_user_query": latest_user_query.trim(),
            "visible_history": history,
            "session_context": session_context,
        }
    });
    call_plugin_action(agent_runtime_plugin, "send_session_message", &payload)
}
```

- [ ] **Step 4: Replace `delegate_call` return planning with actual runtime execution inside `execute_runtime_turn`**

```
if result.kind == "delegate" {
    let delegate_session_id = ensure_delegate_session(&input.agent_runtime_plugin, &input.session_id)?;
    let delegated = run_delegate_session_message(
        &input.agent_runtime_plugin,
        &delegate_session_id,
        &decoded_content,
        &input.history,
        &input.session_context,
        &result.reason,
    )?;
    let delegated_reply = parse_delegated_agent_reply(&delegated)?;
    // continue inside companion-core; do not emit delegate_call back to Electron
}
```

- [ ] **Step 5: Run service runtime tests again**

Run: `npx tsx scripts/test-companion-live.ts`

Expected: PASS, with delegate session management no longer requiring Electron orchestration.

- [ ] **Step 6: Commit**

```bash
git add weft-plugins/companion-core/server.py
git commit -m "feat: move companion delegate execution into companion core"
```

### Task 3: Move Completion Generation and UI-Semantic Event Production into `companion-core`

**Files:**
- Modify: `weft-plugins/companion-core/server.py`
- Modify: `weft/src/shared/types.ts`
- Test: `weft-plugins/companion-core/server.py`

- [ ] **Step 1: Add failing service runtime tests for UI event and host effect generation**

```
#[test]
fn delegated_runtime_turn_emits_ui_events_and_host_effects() {
    let event = build_companion_ui_event(
        "append",
        "agent_complete",
        Some("chat-1"),
        "[Think] ||| I checked it. ||| 我查到了。",
        42,
    )
    .expect("ui event");

    assert_eq!(event["reason"], "agent_complete");
    assert_eq!(event["sessionId"], "chat-1");
    assert_eq!(event["subtitle"], "我查到了。");
}

#[test]
fn direct_runtime_turn_still_emits_replace_stream_event() {
    let event = build_companion_ui_event(
        "replace_stream",
        "direct",
        Some("chat-2"),
        "[Wave] ||| Hey. ||| 你好。",
        7,
    )
    .expect("ui event");

    assert_eq!(event["mode"], "replace_stream");
    assert_eq!(event["reason"], "direct");
}
```

- [ ] **Step 2: Run service-runtime verification to confirm this functionality does not yet exist in the plugin**

Run: `npx tsx scripts/test-companion-live.ts`

Expected: FAIL because `CompanionUiEvent` and host effect payloads are still built in Electron.

- [ ] **Step 3: Add plugin-side helpers that produce canonical UI events and host effects**

```
fn build_companion_ui_event(
    mode: &str,
    reason: &str,
    session_id: Option<&str>,
    structured: &str,
    timestamp: u64,
) -> Result<Value, String> {
    let parsed = parse_structured_fields(structured)?;
    Ok(serde_json::json!({
        "mode": mode,
        "reason": reason,
        "sessionId": session_id.filter(|value| !value.trim().is_empty()),
        "structured": parsed.structured,
        "emotion": parsed.emotion,
        "voice": parsed.voice,
        "subtitle": parsed.subtitle,
        "timestamp": timestamp,
    }))
}

fn push_host_effect_bridge_structured(effects: &mut Vec<Value>, structured: &str) {
    effects.push(serde_json::json!({
        "kind": "bridge_structured",
        "structured": structured,
    }));
}
```

- [ ] **Step 4: Have `execute_runtime_turn` return the full runtime envelope**

```
Ok(CompanionRuntimeTurnResult {
    kind: "delegate".to_string(),
    reason: plan.reason,
    structured: preface_structured.clone(),
    assistant_reply: Some(assistant_reply.clone()),
    completion_structured: Some(completion_structured.clone()),
    tool_results,
    ui_events: vec![
        build_companion_ui_event("append", "agent_preface", Some(&input.session_id), &preface_structured, now_ms())?,
        build_companion_ui_event("append", "agent_complete", Some(&input.session_id), &completion_structured, now_ms())?,
    ],
    host_effects: vec![
        serde_json::json!({ "kind": "bridge_structured", "structured": preface_structured }),
        serde_json::json!({ "kind": "renderer_chat_done", "content": assistant_reply }),
        serde_json::json!({ "kind": "bridge_structured", "structured": completion_structured }),
        serde_json::json!({ "kind": "desktop_notification", "title": "Companion", "body": subtitle }),
    ],
})
```

- [ ] **Step 5: Run service-runtime tests again**

Run: `npx tsx scripts/test-companion-live.ts`

Expected: PASS for direct and delegate event generation.

- [ ] **Step 6: Commit**

```bash
git add weft-plugins/companion-core/server.py weft/src/shared/types.ts
git commit -m "feat: move companion ui event planning into companion core"
```

### Task 4: Shrink `CompanionHostService` into a Thin Host Adapter

**Files:**
- Modify: `weft/src/main/companion-host-service.ts`
- Optional create: `weft/src/main/companion-runtime-host.ts`
- Test: `weft/scripts/test-companion-host-service.ts`

- [ ] **Step 1: Add a failing host-adapter test that proves Electron no longer runs delegate logic**

```ts
import assert from 'node:assert/strict'

const hostSource = await Bun.file('src/main/companion-host-service.ts').text()

assert.equal(hostSource.includes('companionGenerateAgentCompletion('), false)
assert.equal(hostSource.includes('runPluginAction('), false)
assert.equal(hostSource.includes('ensureDelegateSession('), false)
```

- [ ] **Step 2: Run the test to confirm the current file is still a product orchestrator**

Run: `npx tsx scripts/test-companion-host-service.ts`

Expected: FAIL because `CompanionHostService` still performs delegate session creation, delegate plugin execution, and completion generation.

- [ ] **Step 3: Refactor the service into host-effect application only**

```ts
async handleUserMessage(
  content: string,
  history?: { role: string; content: string }[],
  sessionId?: string,
): Promise<boolean> {
  const backendConfig = this.getBackendConfig()
  if (!backendConfig || !this.apiClient) return false

  const sessionContext = await this.getSessionContext(sessionId)
  const runtimeTurn = await this.apiClient.companionExecuteRuntimeTurn({
    provider: backendConfig.provider,
    model: backendConfig.model,
    systemPrompt: this.config.systemPrompt,
    content,
    sessionId,
    history: sanitizeHistoryForBackend(history),
    sessionContext,
  })

  this.applyHostEffects(runtimeTurn.hostEffects)
  return true
}

private applyHostEffects(effects: CompanionRuntimeHostEffect[]): void {
  for (const effect of effects) {
    switch (effect.kind) {
      case 'bridge_structured':
        this.bridge.broadcastStructuredResponse(effect.structured)
        break
      case 'renderer_companion':
        this.pushToRenderer('chat:companion', effect.event)
        break
      case 'renderer_tool_result':
        this.pushToRenderer('chat:tool-result', { name: effect.name, output: effect.output })
        break
      case 'renderer_chat_done':
        this.pushToRenderer('chat:done', effect.content)
        break
      case 'desktop_notification':
        this.showDesktopNotification(effect.title, effect.body)
        break
    }
  }
}
```

- [ ] **Step 4: Delete Electron-only product helpers that no longer belong there**

```ts
// remove:
// - ensureDelegateSession(...)
// - buildDelegateSessionId(...)
// - parseDelegatedAgentReply(...)
// - splitAgentReplyAndToolResults(...)
// - emitCompanionEvent(...)
```

- [ ] **Step 5: Run the host-adapter test again**

Run: `npx tsx scripts/test-companion-host-service.ts`

Expected: PASS, with Electron reduced to bridge lifecycle plus host-effect application.

- [ ] **Step 6: Commit**

```bash
git add weft/src/main/companion-host-service.ts weft/scripts/test-companion-host-service.ts
git commit -m "refactor: thin companion host service down to host effects"
```

### Task 5: Rewire Electron Startup and Chat Flow Around the New Runtime Boundary

**Files:**
- Modify: `weft/src/main/index.ts`
- Modify: `weft/src/main/weft-core-api-client.ts`
- Test: `weft/scripts/test-companion-live.ts`

- [ ] **Step 1: Add a failing live-path test for the new runtime action**

```ts
import assert from 'node:assert/strict'
import { WeftCoreApiClient } from '../src/main/weft-core-api-client'

const client = new WeftCoreApiClient('127.0.0.1', 42617)
const turn = await client.companionExecuteRuntimeTurn({
  provider: 'openai',
  model: 'gpt-5.4',
  systemPrompt: 'You are the companion.',
  content: '帮我上网搜索 chicken 的含义',
  sessionId: 'chat-test',
  history: [],
  sessionContext: [],
})

assert.equal(turn.kind, 'delegate')
assert.ok(Array.isArray(turn.hostEffects))
assert.ok(turn.hostEffects.some((effect) => effect.kind === 'bridge_structured'))
assert.ok(turn.hostEffects.some((effect) => effect.kind === 'renderer_chat_done'))
```

- [ ] **Step 2: Run the live-path test to confirm the old action name and payload are still in use**

Run: `npx tsx scripts/test-companion-live.ts`

Expected: FAIL because Electron still uses `companionExecuteUserTurn()` and expects to finish the delegate branch itself.

- [ ] **Step 3: Update startup wiring to the new thin host boundary**

```ts
companionHost = new CompanionHostService({
  config: appConfig.companion,
  apiClient,
  getBackendConfig: () => getCurrentBackendConfig(),
  waitForCoreAgentReady,
  sessionStore,
  pushToRenderer,
  showDesktopNotification: (title, body) => {
    if (!Notification.isSupported()) return
    new Notification({ title, body, silent: false }).show()
  },
})
```

```ts
// keep startup ownership in Electron, but do not let index.ts branch on direct vs delegate
// that branching now belongs exclusively to companion-core
```

- [ ] **Step 4: Remove old API names if they become misleading**

```ts
// rename:
// companionExecuteUserTurn -> companionExecuteRuntimeTurn
// keep companionGenerateAgentCompletion only if another caller still needs it
// otherwise delete the Electron-facing call site from companion runtime flow
```

- [ ] **Step 5: Run build and live test**

Run: `npm run build`

Expected: PASS for Electron build.

Run: `npx tsx scripts/test-companion-live.ts`

Expected: PASS for both direct and delegate runtime envelopes through the real core path.

- [ ] **Step 6: Commit**

```bash
git add weft/src/main/index.ts weft/src/main/weft-core-api-client.ts weft/scripts/test-companion-live.ts
git commit -m "refactor: route companion turns through runtime envelope"
```

### Task 6: Full Product Verification on the Real Desktop Path

**Files:**
- Modify if needed: `weft/scripts/test-companion-live.ts`
- Test: `weft/scripts/test-companion-live.ts`
- Test: `weft/scripts/test-companion-host-service.ts`

- [ ] **Step 1: Run companion service verification for the plugin runtime**

Run: `npx tsx scripts/test-companion-live.ts`

Expected: PASS

- [ ] **Step 2: Run Electron build verification**

Run: `npm run build`

Expected: PASS

- [ ] **Step 3: Run host-side scripted verification**

Run: `npx tsx scripts/test-companion-host-service.ts`

Expected: PASS

- [ ] **Step 4: Run live core-path verification**

Run: `npx tsx scripts/test-companion-live.ts`

Expected:
- direct input like `Hey` returns a direct runtime turn quickly
- delegate input like `帮我上网搜索 chicken 的含义` returns a delegate runtime turn with no Electron-side `delegate_call`
- returned host effects include bridge, renderer, and notification effects where applicable

- [ ] **Step 5: Run manual desktop verification**

Run: `npm run dev`

Expected manual checks:
- open Electron window successfully
- send `Hey`, confirm no long hang and no timeout
- send `帮我上网搜索 chicken 的含义`, confirm delegate actually executes and returns answer
- confirm Unity bridge still receives structured payloads
- confirm renderer still receives `chat:companion`, `chat:tool-result`, and `chat:done`
- confirm there is no fallback, keyword path, or Electron-side direct/delegate branching left in the companion flow

- [ ] **Step 6: Commit**

```bash
git add weft/scripts/test-companion-live.ts weft/scripts/test-companion-host-service.ts
git commit -m "test: verify thin-electron companion runtime path"
```

## Self-Review

- Spec coverage:
  - Move product runtime control into `companion-core`: covered by Tasks 2-5.
  - Leave Electron as thin host: covered by Task 4 and Task 5.
  - Keep bridge and UI semantics working on the real path: covered by Task 3 and Task 6.
- Placeholder scan:
  - No `TODO`, `TBD`, or "handle appropriately" placeholders remain.
- Type consistency:
  - The plan uses `CompanionRuntimeTurnResult`, `CompanionRuntimeHostEffect`, `uiEvents`, and `hostEffects` consistently across plugin, client, and Electron host code.
