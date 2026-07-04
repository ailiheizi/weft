# WEFT Agent Runtime Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stabilize WEFT’s agent foundation so chat, web search, deep thinking, coding actions, delegation, game companion output, and package/mod binding all use explicit contracts instead of shared prompt/context heuristics.

**Architecture:** Build a typed turn contract first, then split routing, context assembly, tool policy, executor output, and package/mod taxonomy into separate layers. Keep compatibility adapters during migration so the app stays usable while the new contracts are introduced, then remove the old paths after the new ones are validated.

**Tech Stack:** TypeScript, Vue, Electron, Rust/WASM plugin runtime, Python service runtime, TOML manifests, existing frontend test scripts, existing WEFT-core plugin APIs.

---

## File Structure Map

The refactor should be organized around narrow files with one job each:

- `clients/frontend/src/shared/types.ts` — shared turn/session/output contract types.
- `clients/frontend/src/main/services/weft-turn-contracts.ts` — new runtime contract helpers and debug snapshots.
- `clients/frontend/src/main/services/weft-context-assembler.ts` — turns sessions, transcript, memory, workspace, and activity into a model-ready context bundle.
- `clients/frontend/src/main/services/weft-runtime-router.ts` — deterministic route selection.
- `clients/frontend/src/main/services/weft-tool-policy.ts` — route-based tool allow/deny policy.
- `clients/frontend/src/main/companion-runtime-controller.ts` — turn ingress and output envelope wiring.
- `clients/frontend/src/main/companion-chat-client.ts` — companion output parser/adapter and compatibility line protocol.
- `clients/frontend/src/main/companion-config-prompt.ts` — companion persona prompt cleanup and mode separation.
- `clients/frontend/src/main/weft-core-session-store.ts` — session/workspace persistence contract.
- `clients/frontend/src/main/weft-core-api-client.ts` — split into focused adapters behind a backward-compatible facade.
- `clients/frontend/src/main/weft-core-host-service.ts` — session work-agent prompt and execution wiring.
- `clients/frontend/src/main/weft-core-plugin-client.ts` — execution transport with tool-policy enforcement hooks.
- `clients/frontend/src/main/services/weft-suite-sync-engine.ts` — package/mod graph and binding metadata.
- `clients/frontend/src/main/services/weft-suite-runtime-sync.ts` — session agent rebinding and suite sync.
- `clients/frontend/src/main/services/weft-plugin-manifest.ts` — package manifest validation and taxonomy support.
- `clients/frontend/src/main/companion-mirror-window-controller.ts` — overlay close/hide lifecycle.
- `clients/frontend/src/renderer/stores/chat.ts` — transcript/session/workspace presentation and routing state.
- `clients/frontend/src/renderer/components/MirrorCompanionApp.vue` — game companion surface rendering.
- `clients/frontend/src/renderer/components/CompanionSurfaceHost.vue` — companion surface host integration.
- `packages/official/companion-core/server.py` — companion routing and structured output runtime.
- `packages/official/agent-core/src/*` — session/delegation runtime once the contract is stable.
- `packages/official/tool-runtime-core/src/*` — tool execution hooks and policy-compatible tool handling.
- `packages/index.toml` and `packages/weft-claw/package.toml` — package/mod taxonomy and capability mapping.
- `clients/frontend/scripts/*` — contract and regression test scripts.

---

### Task 1: Add core turn contracts and debug snapshots

**Files:**
- Modify: `clients/frontend/src/shared/types.ts`
- Create: `clients/frontend/src/main/services/weft-turn-contracts.ts`
- Modify: `clients/frontend/src/main/weft-core-session-store.ts`
- Modify: `clients/frontend/src/main/companion-runtime-controller.ts`
- Modify: `clients/frontend/src/main/weft-core-api-client.ts`
- Test: `clients/frontend/scripts/test-agent-runtime-session-roundtrip.ts`
- Test: `clients/frontend/scripts/test-chat-store-companion-history-filter.ts`

- [ ] **Step 1: Write the failing contract tests**

Add assertions that a single turn is represented as separate transcript/session/workspace slices, not one overloaded `history` array.

```ts
import { strict as assert } from 'node:assert'
import { buildTurnEnvelope, buildContextBundle } from '../src/main/services/weft-turn-contracts'

const envelope = buildTurnEnvelope({
  sessionId: 's-1',
  workspaceId: 'w-1',
  userText: 'hello',
  transcriptWindow: [{ role: 'user', content: 'hello' }],
  sessionMemory: [{ key: 'm1', content: 'remembered' }],
  workspaceContext: { rootPath: 'D:/work' },
  currentActivity: null,
  requestedMode: 'chat',
  surface: 'desktop_chat',
  boundSkills: [],
  createdAt: 1,
})

assert.equal(envelope.transcriptWindow.length, 1)
assert.equal(envelope.sessionMemory.length, 1)
assert.equal(envelope.workspaceContext?.rootPath, 'D:/work')
```

- [ ] **Step 2: Run the tests and verify they fail before implementation**

Run:

```bash
npx tsx scripts/test-agent-runtime-session-roundtrip.ts
```

Expected: fail because the new contract helper does not exist yet.

- [ ] **Step 3: Implement the new contract helpers**

Add `UserTurnEnvelope`, `ContextBundle`, `RouterDecision`, `ToolPolicy`, `OutputEnvelope`, and `DelegationEvent` types in shared code. Add helpers that snapshot the assembled context in a debug-safe structure so router decisions can be inspected without rereading the whole chat stream.

- [ ] **Step 4: Re-run the contract tests**

Run:

```bash
npx tsx scripts/test-agent-runtime-session-roundtrip.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the contract layer**

```bash
git add clients/frontend/src/shared/types.ts clients/frontend/src/main/services/weft-turn-contracts.ts clients/frontend/src/main/weft-core-session-store.ts clients/frontend/src/main/companion-runtime-controller.ts clients/frontend/src/main/weft-core-api-client.ts clients/frontend/scripts/test-agent-runtime-session-roundtrip.ts clients/frontend/scripts/test-chat-store-companion-history-filter.ts
git commit -m "feat: add agent runtime turn contracts"
```

---

### Task 2: Separate transcript, memory, workspace, and activity context

**Files:**
- Modify: `clients/frontend/src/main/companion-runtime-controller.ts`
- Create: `clients/frontend/src/main/services/weft-context-assembler.ts`
- Modify: `clients/frontend/src/main/weft-core-session-store.ts`
- Modify: `clients/frontend/src/main/services/weft-core-api-client.ts`
- Modify: `clients/frontend/src/renderer/stores/chat.ts`
- Test: `clients/frontend/scripts/test-chat-store-workspace-active-session-reset.ts`
- Test: `clients/frontend/scripts/test-context-engine-proactive-main-routing.ts`
- Test: `clients/frontend/scripts/test-chat-runtime-single-path.ts`

- [ ] **Step 1: Write context-separation tests**

Add coverage that transcript-only follow-ups stay on transcript, workspace context only appears when active workspace data exists, and coding context does not leak into unrelated web/chat turns.

```ts
assert.deepEqual(bundle.transcript.map((m) => m.role), ['user', 'assistant'])
assert.equal(bundle.workspace?.rootPath, 'D:/project')
assert.ok(!bundle.transcript.some((m) => m.content.includes('workspace root')))
```

- [ ] **Step 2: Run the tests to confirm the current mixed context path is broken**

Run:

```bash
npx tsx scripts/test-chat-runtime-single-path.ts
```

Expected: fail or show the current mixed-context behavior.

- [ ] **Step 3: Implement a dedicated context assembler**

Build `assembleContextBundle()` so it explicitly merges session transcript, memory snippets, workspace snapshot, and current activity into separate fields. Remove the duplicated `history -> sessionContext` use and make the turn controller pass only the intended slice into each field.

- [ ] **Step 4: Re-run the context tests**

Run:

```bash
npx tsx scripts/test-chat-runtime-single-path.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the context separation layer**

```bash
git add clients/frontend/src/main/companion-runtime-controller.ts clients/frontend/src/main/services/weft-context-assembler.ts clients/frontend/src/main/weft-core-session-store.ts clients/frontend/src/main/services/weft-core-api-client.ts clients/frontend/src/renderer/stores/chat.ts clients/frontend/scripts/test-chat-runtime-single-path.ts clients/frontend/scripts/test-context-engine-proactive-main-routing.ts clients/frontend/scripts/test-chat-store-workspace-active-session-reset.ts
git commit -m "feat: separate transcript and workspace context"
```

---

### Task 3: Add the runtime router and tool policy gate

**Files:**
- Create: `clients/frontend/src/main/services/weft-runtime-router.ts`
- Create: `clients/frontend/src/main/services/weft-tool-policy.ts`
- Modify: `clients/frontend/src/main/companion-runtime-controller.ts`
- Modify: `clients/frontend/src/main/weft-core-plugin-client.ts`
- Modify: `clients/frontend/src/main/services/weft-core-host-service.ts`
- Test: `clients/frontend/scripts/test-agent-search-quality-gate-contract.ts`
- Test: `clients/frontend/scripts/test-agent-core-memory-readonly-contract.ts`
- Test: `clients/frontend/scripts/test-companion-core-no-keyword-routing-or-fallback.ts`

- [ ] **Step 1: Write deterministic routing tests**

Add explicit cases for search, local file access, deep thinking, delegate work, and companion greeting. The tests should validate the route decision object directly, not a full model call.

```ts
assert.equal(decideRoute('search online for Uzi').route, 'web_search')
assert.equal(decideRoute('read D:/foo/a.txt').route, 'coding_action')
assert.equal(decideRoute('think deeply about the architecture').route, 'deep_think')
```

- [ ] **Step 2: Run the tests and confirm the current behavior is still heuristic-only**

Run:

```bash
npx tsx scripts/test-agent-search-quality-gate-contract.ts
```

Expected: fail until the router exists.

- [ ] **Step 3: Implement the router and policy gate**

Add a deterministic router that emits `RouterDecision`, then enforce a `ToolPolicy` before any plugin/tool invocation. Web search should be web-only, companion mode should not open shell/git/files by default, and coding/local action should be workspace-scoped unless explicitly expanded.

- [ ] **Step 4: Re-run routing and policy tests**

Run:

```bash
npx tsx scripts/test-agent-search-quality-gate-contract.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the router/policy layer**

```bash
git add clients/frontend/src/main/services/weft-runtime-router.ts clients/frontend/src/main/services/weft-tool-policy.ts clients/frontend/src/main/companion-runtime-controller.ts clients/frontend/src/main/weft-core-plugin-client.ts clients/frontend/src/main/services/weft-core-host-service.ts clients/frontend/scripts/test-agent-search-quality-gate-contract.ts clients/frontend/scripts/test-agent-core-memory-readonly-contract.ts clients/frontend/scripts/test-companion-core-no-keyword-routing-or-fallback.ts
git commit -m "feat: add runtime routing and tool policy"
```

---

### Task 4: Stabilize companion and game companion output

**Files:**
- Modify: `packages/official/companion-core/server.py`
- Modify: `clients/frontend/src/main/companion-chat-client.ts`
- Modify: `clients/frontend/src/main/companion-config-prompt.ts`
- Modify: `clients/frontend/src/main/companion-runtime-controller.ts`
- Modify: `clients/frontend/src/renderer/components/MirrorCompanionApp.vue`
- Modify: `clients/frontend/src/renderer/components/CompanionSurfaceHost.vue`
- Test: `clients/frontend/scripts/test-companion-chat-client-emotions.ts`
- Test: `clients/frontend/scripts/test-companion-runtime-provider-fallback.ts`
- Test: `clients/frontend/scripts/test-companion-message-tone.ts`
- Test: `clients/frontend/scripts/test-companion-core-service-refresh.ts`

- [ ] **Step 1: Write structured-output compatibility tests**

Cover both JSON-schema-capable and JSON-schema-rejecting providers. Validate that the compatibility adapter still yields `emotion`, `voiceTextEn`, and `subtitleZh` even when the provider returns extra prose or the old line protocol.

```ts
assert.equal(parsed.performance?.emotion, 'Happy')
assert.match(parsed.performance?.voiceTextEn ?? '', /hello/i)
assert.match(parsed.performance?.subtitleZh ?? '', /你好/)
```

- [ ] **Step 2: Run the tests and confirm the current output protocol is still fragile**

Run:

```bash
npx tsx scripts/test-companion-runtime-provider-fallback.ts
```

Expected: fail or expose the current dependency on fragile parser behavior.

- [ ] **Step 3: Rework companion-core to emit the structured envelope first**

Make `CompanionPerformance` the source of truth, keep the old `|||` line only as a compatibility adapter, and separate companion persona prompt from work-agent prompt so game mode cannot inherit coding/session routing instructions.

- [ ] **Step 4: Wire the frontend to consume the structured envelope**

Update the chat client and game surface host so UI text, emotion, TTS voice line, and subtitle are consumed from the new envelope, not reconstructed ad hoc from free text.

- [ ] **Step 5: Re-run the companion tests**

Run:

```bash
npx tsx scripts/test-companion-chat-client-emotions.ts
```

Expected: PASS.

- [ ] **Step 6: Commit the companion protocol refactor**

```bash
git add packages/official/companion-core/server.py clients/frontend/src/main/companion-chat-client.ts clients/frontend/src/main/companion-config-prompt.ts clients/frontend/src/main/companion-runtime-controller.ts clients/frontend/src/renderer/components/MirrorCompanionApp.vue clients/frontend/src/renderer/components/CompanionSurfaceHost.vue clients/frontend/scripts/test-companion-chat-client-emotions.ts clients/frontend/scripts/test-companion-runtime-provider-fallback.ts clients/frontend/scripts/test-companion-message-tone.ts clients/frontend/scripts/test-companion-core-service-refresh.ts
git commit -m "feat: stabilize companion output protocol"
```

---

### Task 5: Stabilize delegation and session continuity

**Files:**
- Modify: `packages/official/agent-core/src/*`
- Modify: `clients/frontend/src/main/services/weft-core-host-service.ts`
- Modify: `clients/frontend/src/renderer/stores/chat.ts`
- Modify: `clients/frontend/src/main/companion-runtime-controller.ts`
- Modify: `packages/official/tool-runtime-core/src/*` if delegate tasks need tool hooks
- Test: `clients/frontend/scripts/test-agent-runtime-list-agents.ts`
- Test: `clients/frontend/scripts/test-agent-runtime-session-roundtrip.ts`
- Test: `clients/frontend/scripts/test-agent-core-live.ts`
- Test: `clients/frontend/scripts/test-companion-live-delegate-session-inherits-skills.ts`

- [ ] **Step 1: Write delegation lifecycle tests**

Validate that a heavy task becomes a queued/running/completed event chain inside the same session, and that a second user message can still be added while the task is running.

```ts
assert.equal(task.status, 'queued')
assert.equal(task.parentTurnId, 'turn-1')
assert.ok(task.progress.length >= 0)
```

- [ ] **Step 2: Run the tests and confirm the current lifecycle is still weak**

Run:

```bash
npx tsx scripts/test-companion-live-delegate-session-inherits-skills.ts
```

Expected: fail or demonstrate incomplete continuity.

- [ ] **Step 3: Add delegated task state and landing behavior**

Persist `DelegationEvent` under the parent session, emit start/progress/completion events, and make sure the companion returns a short landing summary when the task completes.

- [ ] **Step 4: Re-run the delegation tests**

Run:

```bash
npx tsx scripts/test-agent-runtime-session-roundtrip.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the delegation continuity layer**

```bash
git add packages/official/agent-core/src/* clients/frontend/src/main/services/weft-core-host-service.ts clients/frontend/src/renderer/stores/chat.ts clients/frontend/src/main/companion-runtime-controller.ts clients/frontend/scripts/test-agent-runtime-list-agents.ts clients/frontend/scripts/test-agent-runtime-session-roundtrip.ts clients/frontend/scripts/test-agent-core-live.ts clients/frontend/scripts/test-companion-live-delegate-session-inherits-skills.ts
git commit -m "feat: preserve delegation in the active session"
```

---

### Task 6: Split the frontend/core adapter into focused clients

**Files:**
- Modify: `clients/frontend/src/main/weft-core-api-client.ts`
- Create: `clients/frontend/src/main/weft-core-http-client.ts`
- Create: `clients/frontend/src/main/weft-core-plugin-bridge.ts`
- Create: `clients/frontend/src/main/weft-core-session-client.ts`
- Create: `clients/frontend/src/main/weft-core-package-client.ts`
- Create: `clients/frontend/src/main/weft-core-context-client.ts`
- Create: `clients/frontend/src/main/weft-core-completion-client.ts`
- Test: `clients/frontend/scripts/test-electron-ipc-handlers-imports.ts`
- Test: `clients/frontend/scripts/test-chat-tool-result-single-source.ts`

- [ ] **Step 1: Write a facade regression test**

Prove the current large adapter still resolves all old call sites, then add tests for the new focused clients.

```ts
assert.ok(typeof apiClient.chatCompletion === 'function')
assert.ok(typeof apiClient.listSessions === 'function')
assert.ok(typeof apiClient.listPackageTopologies === 'function')
```

- [ ] **Step 2: Run the facade test and observe the current monolith**

Run:

```bash
npx tsx scripts/test-electron-ipc-handlers-imports.ts
```

Expected: pass now, then keep passing after the split through the facade.

- [ ] **Step 3: Extract focused clients behind the facade**

Move HTTP, plugin, session, package, context, and completion concerns into small files. Keep `weft-core-api-client.ts` as a compatibility facade until the new callers are migrated.

- [ ] **Step 4: Re-run the facade and tool-result tests**

Run:

```bash
npx tsx scripts/test-chat-tool-result-single-source.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the adapter split**

```bash
git add clients/frontend/src/main/weft-core-api-client.ts clients/frontend/src/main/weft-core-http-client.ts clients/frontend/src/main/weft-core-plugin-bridge.ts clients/frontend/src/main/weft-core-session-client.ts clients/frontend/src/main/weft-core-package-client.ts clients/frontend/src/main/weft-core-context-client.ts clients/frontend/src/main/weft-core-completion-client.ts clients/frontend/scripts/test-electron-ipc-handlers-imports.ts clients/frontend/scripts/test-chat-tool-result-single-source.ts
git commit -m "refactor: split core adapter into focused clients"
```

---

### Task 7: Clean up package/mod taxonomy and binding validation

**Files:**
- Modify: `packages/index.toml`
- Modify: `packages/weft-claw/package.toml`
- Modify: `packages/weft-code/package.toml`
- Modify: `clients/frontend/src/main/services/weft-suite-sync-engine.ts`
- Modify: `clients/frontend/src/main/services/weft-suite-runtime-sync.ts`
- Modify: `clients/frontend/src/main/services/weft-plugin-manifest.ts`
- Test: `clients/frontend/scripts/test-agent-core-single-skill-live.ts`
- Test: `clients/frontend/scripts/test-install-openclaw-skill-live.ts`
- Test: `clients/frontend/scripts/test-agent-core-delete-session-does-not-delete-agent.ts`

- [ ] **Step 1: Write manifest taxonomy tests**

Add checks that product manifests, feature bundles, capability providers, and UI surfaces map to explicit metadata instead of generic mod labels.

```ts
assert.equal(manifest.identity.name, 'weft-claw')
assert.ok(manifest.requires.capabilities.includes('team.delegate'))
```

- [ ] **Step 2: Run the manifest/binding tests**

Run:

```bash
npx tsx scripts/test-agent-core-single-skill-live.ts
```

Expected: current suite may pass or expose missing binding metadata.

- [ ] **Step 3: Add taxonomy and validation fields**

Clarify which package is a product, feature bundle, provider package, UI surface package, or companion pack, then validate suite sync against those roles.

- [ ] **Step 4: Re-run the package graph tests**

Run:

```bash
npx tsx scripts/test-install-openclaw-skill-live.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the taxonomy cleanup**

```bash
git add packages/index.toml packages/weft-claw/package.toml packages/weft-code/package.toml clients/frontend/src/main/services/weft-suite-sync-engine.ts clients/frontend/src/main/services/weft-suite-runtime-sync.ts clients/frontend/src/main/services/weft-plugin-manifest.ts clients/frontend/scripts/test-agent-core-single-skill-live.ts clients/frontend/scripts/test-install-openclaw-skill-live.ts clients/frontend/scripts/test-agent-core-delete-session-does-not-delete-agent.ts
git commit -m "feat: clarify package and mod taxonomy"
```

---

### Task 8: Add overlay close/hide lifecycle and companion UX

**Files:**
- Modify: `clients/frontend/src/main/companion-mirror-window-controller.ts`
- Modify: `clients/frontend/src/renderer/components/MirrorCompanionApp.vue`
- Modify: `clients/frontend/src/renderer/components/CompanionSurfaceHost.vue`
- Modify: `clients/frontend/src/renderer/stores/chat.ts`
- Test: `clients/frontend/scripts/test-companion-mirror-window-controller.ts`
- Test: `clients/frontend/scripts/test-companion-surface-visibility-sync.ts`
- Test: `clients/frontend/scripts/test-main-companion-host-mirror-gating.ts`

- [ ] **Step 1: Write lifecycle tests for close/hide behavior**

Assert that the overlay can be closed or hidden and that the preference persists across restarts.

```ts
assert.equal(state.visible, false)
assert.equal(state.persistedVisibility, 'hidden')
```

- [ ] **Step 2: Run the overlay tests to expose the current gap**

Run:

```bash
npx tsx scripts/test-companion-mirror-window-controller.ts
```

Expected: fail until close/hide persistence is implemented.

- [ ] **Step 3: Implement the overlay lifecycle controls**

Add close/hide affordances, remember user choice, and ensure minimize does not keep the overlay interfering with gameplay.

- [ ] **Step 4: Re-run the overlay and visibility tests**

Run:

```bash
npx tsx scripts/test-companion-surface-visibility-sync.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the overlay lifecycle change**

```bash
git add clients/frontend/src/main/companion-mirror-window-controller.ts clients/frontend/src/renderer/components/MirrorCompanionApp.vue clients/frontend/src/renderer/components/CompanionSurfaceHost.vue clients/frontend/src/renderer/stores/chat.ts clients/frontend/scripts/test-companion-mirror-window-controller.ts clients/frontend/scripts/test-companion-surface-visibility-sync.ts clients/frontend/scripts/test-main-companion-host-mirror-gating.ts
git commit -m "feat: add companion overlay close and hide controls"
```

---

### Task 9: Add the regression matrix and route snapshots

**Files:**
- Create or modify: `clients/frontend/scripts/test-agent-runtime-route-fixtures.ts`
- Create or modify: `clients/frontend/scripts/test-agent-runtime-context-fixtures.ts`
- Create or modify: `clients/frontend/scripts/test-agent-runtime-tool-policy-fixtures.ts`
- Create or modify: `clients/frontend/scripts/test-agent-runtime-delegation-fixtures.ts`
- Create or modify: `clients/frontend/scripts/test-agent-runtime-package-fixtures.ts`

- [ ] **Step 1: Write route/context/tool/delegation/package fixture tests**

Each fixture file should assert one contract surface only, for example a web-search fixture should verify that file tools are denied and the output protocol is companion-neutral.

```ts
assert.equal(decision.route, 'web_search')
assert.deepEqual(decision.allowedTools, ['web_search', 'web_fetch'])
```

- [ ] **Step 2: Run the fixture suite and verify it fails where contracts are missing**

Run:

```bash
npx tsx scripts/test-agent-runtime-route-fixtures.ts
```

Expected: fail until the new router/policy/output contracts are fully wired.

- [ ] **Step 3: Complete the contract fixtures and route snapshots**

Ensure the assembled context and output envelopes are snapshot-tested so future prompt drift is visible immediately.

- [ ] **Step 4: Re-run the full fixture suite**

Run:

```bash
npx tsx scripts/test-agent-runtime-package-fixtures.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the regression harness**

```bash
git add clients/frontend/scripts/test-agent-runtime-route-fixtures.ts clients/frontend/scripts/test-agent-runtime-context-fixtures.ts clients/frontend/scripts/test-agent-runtime-tool-policy-fixtures.ts clients/frontend/scripts/test-agent-runtime-delegation-fixtures.ts clients/frontend/scripts/test-agent-runtime-package-fixtures.ts
git commit -m "test: add agent runtime regression fixtures"
```

---

## Integration Order

Do not let the subagents freely edit the same files at the same time. Use this order:

1. Task 1 — contracts.
2. Task 2 — context separation.
3. Task 3 — router and tool policy.
4. Task 4 — companion/game output.
5. Task 5 — delegation continuity.
6. Task 6 — split the large core adapter.
7. Task 7 — package/mod taxonomy.
8. Task 8 — overlay lifecycle.
9. Task 9 — regression fixtures and snapshots.

Each task should be reviewable on its own and should keep the app runnable.

## Verification Gates

After each task, run the narrowest possible test set first, then the relevant UI typecheck or broader smoke test if the task touches renderer/main contracts.

Minimum gates before moving to the next task:

- `npx tsx <target test script>` for the changed contract.
- `npm run typecheck:ui` when renderer/shared types change.
- Any repo-specific smoke test already associated with the touched area.

Final gate before declaring the refactor done:

- all new contract tests pass,
- all old compatibility adapters still work,
- the companion overlay can be shown/hidden without breaking gameplay,
- the package/mod graph validates,
- a web-search turn cannot reach filesystem tools,
- a companion turn still produces structured emotion/voice/subtitle output.
