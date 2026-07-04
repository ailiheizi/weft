# WEFT Agent Runtime Refactor Design

**Date:** 2026-05-09

## Goal

Build a stable WEFT agent foundation that can support normal companion chat, web search, deep thinking, coding/local actions, game companion output, and subagent delegation without prompt/context/tool pollution.

The current problem is not one missing button or one broken prompt. The system has many working pieces, but the boundaries between them are not strict enough. `companion-core`, `agent-core`, `tool-runtime-core`, `context-engine`, suite sync, frontend chat state, and package metadata all participate in the same user turn, yet there is no single contract that says:

- what a user turn is,
- which runtime owns routing,
- which context is allowed into the model,
- which tools are allowed for the selected mode,
- how companion/game output must be shaped,
- how a background delegate task returns to the same session,
- how packages/mods map to runtime capabilities.

This design creates those contracts first, then refactors the runtime around them. The implementation should be split across multiple subagents, but every subagent must work against this document so the architecture does not drift again.

## Current State

### Existing runtime pieces

- `WEFT-core` exposes OpenAI-compatible chat completion, plugin execution, service orchestration, and suite/package APIs.
- `clients/frontend` owns the Electron shell, chat UI, companion surface, window lifecycle, session store adapter, and package/mod manager UI.
- `packages/official/companion-core` is a service runtime for companion turn handling, direct/delegate decisions, emotion/voice/subtitle generation, and prompt state.
- `packages/official/agent-core` is the wasm package intended to own sessions, agent routing, team delegation, and multi-agent primitives.
- `packages/official/tool-runtime-core` owns tools such as shell, files, web, git, wiki, desktop, and local scout.
- `packages/official/context-engine` provides current activity and coding-context detection.
- `packages/official/suite-manager` and frontend suite sync build product/plugin graphs and bind package capabilities.
- `packages/index.toml` and product `package.toml` files describe package source, package kind, runtime provider, capabilities, feature bindings, and product bindings.

### Main symptoms

1. A normal companion turn can be contaminated by coding/tool/delegation behavior.
2. Web search mode can incorrectly drift into local file or workspace inspection.
3. Deep thinking/model adaptation, web search, and agent runtime behavior are not isolated.
4. Heavy delegate tasks are slow and unstable, and they do not feel like part of the same companion session.
5. Short-term transcript memory is unreliable; recent chat context is not clearly separated from session memory or workspace context.
6. Game companion output regressed from structured performance data into ordinary text or fixed-language responses.
7. The game overlay lacks a clear close/hide/persist lifecycle.
8. Mod/package/product/runtime terminology is mixed, making it hard to reason about skins, features, tools, and runtime providers.
9. Testing is mostly end-to-end/manual, so prompt/context regressions reappear after each fix.

### Evidence from code inspection

- `clients/frontend/src/main/weft-core-api-client.ts` is a large adapter that mixes core HTTP calls, plugin calls, companion context encoding, product graph parsing, package topology, chat completion, context-engine calls, and product manifest parsing.
- `clients/frontend/src/main/companion-runtime-controller.ts` passes the same `history` into both `history` and `sessionContext`, which collapses transcript and session context into one channel.
- `clients/frontend/src/main/services/weft-core-host-service.ts` defines `WEFT_SESSION_AGENT_SYSTEM_PROMPT`, a work-agent prompt that contains routing guidance for search, local files, code inspection, and tools. That is valid for a session work agent, but it must not leak into companion persona or game mode.
- `packages/official/companion-core/server.py` currently does persona behavior, direct/delegate routing, JSON schema response formatting, structured line generation, and plugin calls in one service.
- `packages/official/companion-core/server.py` uses provider-dependent `response_format: json_schema`; providers such as DeepSeek may not support this consistently.
- `clients/frontend/src/main/companion-chat-client.ts` parses a structured line protocol like `[Emotion] ||| English voice ||| Chinese subtitle`, while `companion-core` also uses a JSON decision protocol. The two protocol layers are not represented as a shared typed envelope.
- `packages/weft-claw/package.toml` already declares capabilities such as `agent.runtime`, `tool.web`, `tool.files`, `team.delegate`, and `ui.surface`, but runtime tool isolation is not enforced as a central mode-based gate.
- `clients/frontend/src/main/services/weft-suite-runtime-sync.ts` can rebind entry/session agents and sync prompts/plugins, so suite sync can unintentionally alter the runtime prompts for existing sessions.

## Design Principles

1. **Contracts before prompts.** Prompts should express behavior, not define system architecture.
2. **One user turn, one envelope.** Every mode starts from a typed `UserTurnEnvelope`.
3. **Router decides, executors execute.** The router chooses a route and policy; it does not perform the work.
4. **Transcript is not memory.** Recent chat, durable memory, workspace context, and tool events are separate inputs with explicit priority.
5. **Tools are runtime-gated.** A prompt may describe tools, but only a runtime policy can allow tools.
6. **Companion output is structured.** Game/UI surfaces consume typed fields, not parsed prose.
7. **Delegation returns to the same session.** Background tasks are session events, not separate conversations.
8. **Product words may be friendly; engineering words must be precise.** Users can see “mods”; code must distinguish product, feature bundle, provider, capability, UI surface, and package source.
9. **Tests snapshot the assembled context.** Regressions should be caught before manual frontend testing.

## Target Architecture

```text
User / Game / UI Event
  -> Frontend Turn Ingress
  -> UserTurnEnvelope
  -> Session Context Assembler
  -> Runtime Router
  -> Tool Policy Gate
  -> Executor
       - direct companion executor
       - session work agent executor
       - web search executor
       - deep thinking executor
       - coding/local action executor
       - delegate/subagent executor
  -> Output Envelope
  -> Chat UI / Game Overlay / Task Timeline
  -> Transcript + Event Persistence
```

The important change is that routing, context assembly, tool authorization, execution, and output formatting become separate stages.

## Core Contracts

### 1. UserTurnEnvelope

Every user-visible turn should be represented as one envelope before any model call.

```ts
type WeftSurface =
  | 'desktop_chat'
  | 'game_companion'
  | 'coding_workspace'
  | 'background_task'
  | 'system_event'

type RequestedMode =
  | 'auto'
  | 'chat'
  | 'web_search'
  | 'deep_think'
  | 'coding'
  | 'delegate'
  | 'game'

type UserTurnEnvelope = {
  turnId: string
  sessionId: string
  workspaceId: string
  workspaceRoot?: string | null
  surface: WeftSurface
  requestedMode: RequestedMode
  userText: string
  transcriptWindow: TranscriptMessage[]
  sessionMemory: MemorySnippet[]
  workspaceContext: WorkspaceContextSnapshot | null
  currentActivity: ActivitySnapshot | null
  boundSkills: string[]
  sourceEvent?: Record<string, unknown>
  createdAt: number
}
```

Rules:

- `transcriptWindow` is the latest visible chat history only.
- `sessionMemory` is durable or summarized memory only.
- `workspaceContext` is project/desktop/coding context only.
- `currentActivity` is contextual signal, not instruction.
- The same message list must not be passed as both transcript and memory/context.
- User text is the highest-priority user instruction for this turn.

### 2. ContextBundle

Context assembly should produce a model-ready bundle with explicit sections.

```ts
type ContextBundle = {
  identityPrompt: string
  routePolicyPrompt: string
  outputProtocolPrompt: string
  transcript: TranscriptMessage[]
  memory: MemorySnippet[]
  workspace: WorkspaceContextSnapshot | null
  toolPolicy: ToolPolicy
  debugSnapshot: ContextDebugSnapshot
}
```

Priority order:

1. System safety and runtime constraints.
2. Selected surface identity, such as companion persona or work agent role.
3. Output protocol.
4. Latest user turn.
5. Transcript window.
6. Explicit session memory.
7. Workspace/current-activity context.
8. Package/product hints.

No lower-priority context may override a higher-priority instruction.

### 3. RouterDecision

The router returns a typed decision only.

```ts
type RouteKind =
  | 'direct_companion'
  | 'session_agent'
  | 'web_search'
  | 'deep_think'
  | 'coding_action'
  | 'delegate_task'
  | 'game_companion'

type RouterDecision = {
  route: RouteKind
  reason: string
  confidence: number
  executor: string
  allowedTools: ToolName[]
  outputProtocol: OutputProtocol
  delegation?: DelegationRequest
  warnings: string[]
}
```

Routing rules:

- Explicit web/search/browse intent routes to `web_search` unless the user asks to search local files.
- Explicit local path, codebase, file, shell, git, desktop, or runtime inspection routes to `coding_action` or `delegate_task`.
- Normal greeting or lightweight conversation routes to `direct_companion` or `game_companion` depending on surface.
- Heavy, slow, multi-step, or uncertain work routes to `delegate_task`.
- Deep thinking is a reasoning route, not automatically a tool route.
- Router must not answer the user directly.

### 4. ToolPolicy

Tools must be gated by the selected route, not by prompt wording.

```ts
type ToolPolicy = {
  mode: RouteKind
  allow: ToolName[]
  deny: ToolName[]
  requireUserConfirmation: ToolName[]
  filesystemRoots: string[]
  networkAllowed: boolean
  shellAllowed: boolean
  gitAllowed: boolean
  maxRuntimeMs: number
}
```

Baseline policies:

| Route | Allowed by default | Explicitly denied by default |
|---|---|---|
| `direct_companion` | none or safe memory read | shell, git, fs write, broad fs read, web |
| `game_companion` | companion state, game output event | shell, git, broad fs, web unless asked |
| `web_search` | web search, web fetch | fs read/write, shell, git |
| `deep_think` | none, optional memory read | web/fs/shell/git unless requested |
| `coding_action` | scoped fs, git read, diagnostics | destructive shell/git unless confirmed |
| `delegate_task` | subagent/delegation APIs | direct host mutation outside delegated policy |
| `session_agent` | policy from requested mode | anything not in policy |

This fixes the “web search mode reads fs” class of bug.

### 5. OutputEnvelope

All executors return structured output.

```ts
type OutputEnvelope = {
  turnId: string
  sessionId: string
  route: RouteKind
  text: string
  performance?: CompanionPerformance
  task?: TaskEventSummary
  toolEvents: ToolEventSummary[]
  diagnostics: RuntimeDiagnostic[]
}

type CompanionPerformance = {
  emotion: 'Happy' | 'Confused' | 'Sad' | 'Fun' | 'Agree' | 'Drink' | 'Wave' | 'Think'
  voiceTextEn: string
  subtitleZh: string
  hostEffects: Array<Record<string, unknown>>
}
```

Rules:

- Chat UI can render `text`.
- Game overlay consumes `performance`.
- The old `emotion ||| voice ||| subtitle` line may remain as a compatibility adapter, but it must be generated from `CompanionPerformance`, not treated as the source of truth.
- JSON schema support must be optional. If a provider does not support native structured output, use plain text JSON instruction plus robust parser/repair, then validate the final object.

### 6. DelegationEvent

Delegation must feel like one session, not a separate hidden conversation.

```ts
type DelegationEvent = {
  taskId: string
  parentTurnId: string
  sessionId: string
  status: 'queued' | 'running' | 'blocked' | 'completed' | 'failed' | 'cancelled'
  summary: string
  progress: string[]
  result?: OutputEnvelope
  createdAt: number
  updatedAt: number
}
```

Rules:

- Companion posts a short acknowledgement when a delegate task starts.
- The task timeline is persisted under the same session.
- Completion returns as a session event and can produce a companion landing message.
- Slow tasks must not block the main session input loop.
- Delegated agents receive a scoped `ToolPolicy` and `ContextBundle`, not the entire frontend/runtime context.

## Component Responsibilities

### Frontend turn ingress

Owns:

- building `UserTurnEnvelope`,
- selecting active session/workspace/surface,
- persisting user-visible messages,
- rendering `OutputEnvelope`,
- managing overlay window lifecycle.

Does not own:

- final routing policy,
- tool authorization,
- subagent execution internals.

Likely files:

- `clients/frontend/src/renderer/stores/chat.ts`
- `clients/frontend/src/main/companion-runtime-controller.ts`
- `clients/frontend/src/main/companion-chat-client.ts`
- `clients/frontend/src/main/companion-mirror-window-controller.ts`
- `clients/frontend/src/renderer/components/MirrorCompanionApp.vue`

### Session/context layer

Owns:

- session records,
- transcript windows,
- session memory pointers,
- workspace binding,
- context bundle debug snapshots.

Likely files:

- `clients/frontend/src/main/weft-core-session-store.ts`
- `packages/official/agent-core/src/*`
- `packages/official/memory-runtime/*`
- `packages/official/context-engine/*`

### Router layer

Owns:

- mapping `UserTurnEnvelope` to `RouterDecision`,
- applying route policy,
- selecting executor,
- producing route diagnostics.

Target location:

- Prefer a small shared frontend/main service first for MVP, then move into `agent-core` once the contract is stable.

Reason:

- The frontend currently assembles most inputs and can enforce behavior faster.
- Moving too much into wasm before the contract is proven increases risk.

### Executor layer

Owns:

- direct companion reply,
- web search execution,
- deep thinking call,
- coding/local action orchestration,
- delegate/subagent lifecycle,
- converting results into `OutputEnvelope`.

Likely files:

- `packages/official/companion-core/server.py`
- `packages/official/agent-core/src/*`
- `packages/official/tool-runtime-core/src/*`
- `clients/frontend/src/main/weft-core-plugin-client.ts`

### Package/mod layer

Owns:

- package source index,
- product manifests,
- feature bindings,
- capability providers,
- UI surface/package metadata,
- enable/disable state.

Likely files:

- `packages/index.toml`
- `packages/weft-claw/package.toml`
- `packages/weft-code/package.toml`
- `clients/frontend/src/main/services/weft-suite-sync-engine.ts`
- `clients/frontend/src/main/services/weft-suite-runtime-sync.ts`
- `clients/frontend/src/main/services/weft-plugin-manifest.ts`

## Package and Mod Terminology

User-facing terminology can stay simple: “mod”. Engineering terminology must be strict.

| Engineering term | Meaning | Example |
|---|---|---|
| Product | Installable top-level experience | `weft-claw`, `weft-code` |
| Feature bundle | A selectable slice of product behavior | chat feature, automation feature |
| Provider package | Runtime implementation of a capability | `tool-runtime-core`, `agent-core` |
| Capability | Stable interface required/provided by products | `tool.web`, `team.delegate` |
| UI surface package | UI/skin/overlay package | companion skin, mod manager UI |
| Companion pack | Persona + output protocol + assets | game companion personality pack |
| Package source | Remote/local source metadata | entry in `packages/index.toml` |

Rules:

- A mod shown in the UI maps to one product, feature bundle, UI surface package, or companion pack.
- Provider packages should not be casually toggled by users unless wrapped by a product/feature.
- Capability providers are selected by binding policy, not by arbitrary UI toggles.
- Skin/UI mods cannot silently grant tool capabilities.
- Feature mods must declare required capabilities and any tool access.

## Migration Plan

### Phase 0: Freeze contracts and add diagnostics

Deliverables:

- Add TypeScript contract types for `UserTurnEnvelope`, `ContextBundle`, `RouterDecision`, `ToolPolicy`, `OutputEnvelope`, and `DelegationEvent`.
- Add runtime logging/debug snapshot for assembled context.
- Add a compatibility adapter from old companion line protocol to `OutputEnvelope`.
- Add snapshot fixtures for current known flows.

Success criteria:

- Existing UI still opens.
- Existing companion responses still render.
- Debug snapshot shows separated transcript, memory, workspace, current activity, and tool policy.

### Phase 1: Fix transcript/context separation

Deliverables:

- Remove the duplicated `history` -> `sessionContext` mapping.
- Build transcript from chat messages only.
- Build session context from `loadSessionContext`, memory runtime, or explicit context-engine output only.
- Add tests for context priority.

Success criteria:

- A short greeting does not receive coding/workspace instructions unless explicitly relevant.
- A follow-up question resolves from transcript.
- Workspace context appears only in the workspace section.

### Phase 2: Introduce router and tool policy gate

Deliverables:

- Add `RuntimeRouter` service.
- Route explicit search to web-only policy.
- Route explicit local/code action to coding policy.
- Route heavy work to delegate policy.
- Enforce allowed tools before plugin/tool execution.
- Add route decision diagnostics to UI/dev logs.

Success criteria:

- Web search cannot call fs/shell/git.
- Companion chat cannot call fs/shell/git.
- Coding route can access only scoped workspace roots unless explicitly expanded.
- Router decisions are testable without calling an LLM.

### Phase 3: Stabilize companion/game output

Deliverables:

- Make `CompanionPerformance` the source of truth.
- Support provider fallback when native JSON schema is unavailable.
- Keep old `|||` parser only as compatibility input/output adapter.
- Separate companion persona prompt from work-agent prompt.
- Add game overlay event contract.

Success criteria:

- “你好呀~” produces natural companion text plus emotion, English TTS, and Chinese subtitle.
- Game overlay receives structured fields, not parsed prose.
- Work-agent prompt never appears in companion/game mode.
- DeepSeek/openai-compatible providers both work without native schema dependency.

### Phase 4: Stabilize delegation/session continuity

Deliverables:

- Persist `DelegationEvent` under the parent session.
- Add queued/running/completed/failed task lifecycle.
- Add companion acknowledgement and completion landing message.
- Scope delegated context and tool policy.
- Keep user able to continue chatting while a task runs.

Success criteria:

- A heavy task appears as one session timeline, not a separate hidden chat.
- Completion resumes in the same session with a concise companion summary.
- Subagents cannot use tools outside the delegated policy.
- Slow websearch/deep tasks do not freeze the companion session.

### Phase 5: Refactor adapters and split god files

Deliverables:

- Split `weft-core-api-client.ts` into focused clients:
  - core HTTP client,
  - plugin client,
  - session client,
  - package/product client,
  - context-engine client,
  - completion client.
- Keep a backward-compatible facade during migration.
- Move context encoding into a dedicated context assembler.

Success criteria:

- Existing call sites compile through the facade.
- New runtime path uses focused clients.
- No single adapter owns both package graph parsing and companion turn context.

### Phase 6: Package/mod boundary cleanup

Deliverables:

- Document package/mod taxonomy in package manifests.
- Add manifest fields for UI surface, companion pack, feature bundle, and provider package where needed.
- Update suite sync to validate capability/provider boundaries.
- Prevent UI/skin mods from granting tool capabilities unless explicitly declared by a feature/product.

Success criteria:

- Mod manager can show product-friendly categories.
- Runtime binding remains capability/provider based.
- Package graph diagnostics explain missing/conflicting capabilities.

### Phase 7: Overlay lifecycle and mod manager UX

Deliverables:

- Add close/hide button to game companion overlay.
- Persist overlay visibility preference.
- Support hidden-until-reopened mode.
- Ensure minimize does not interfere with games.
- Improve mod manager basics: list, enable/disable, category, source/status.

Success criteria:

- User can close/hide overlay during gameplay.
- Overlay state survives restart.
- Mod manager distinguishes skin/UI/feature/provider categories.

### Phase 8: Test matrix and regression harness

Deliverables:

- Unit/contract tests for router decisions.
- Snapshot tests for assembled context bundles.
- Tool policy tests for each route.
- Companion output parser/validator tests.
- Delegation lifecycle tests.
- Package graph validation tests.
- UI smoke tests for overlay close/hide and mod manager display.

Success criteria:

- A prompt/context regression fails a contract test before frontend manual testing.
- Web search/local file/coding/game modes have separate test fixtures.
- Structured companion output is validated for every supported provider mode.

## Parallel Subagent Workstreams

Implementation should be delegated in parallel only after the contract files and interfaces are agreed.

### Workstream A: Session and context foundation

Scope:

- contract types,
- context assembler,
- transcript/session/workspace separation,
- context snapshots.

Primary files:

- `clients/frontend/src/shared/types.ts`
- `clients/frontend/src/main/weft-core-session-store.ts`
- `clients/frontend/src/main/companion-runtime-controller.ts`
- new `clients/frontend/src/main/services/weft-turn-contracts.ts`
- new `clients/frontend/src/main/services/weft-context-assembler.ts`

### Workstream B: Router and tool policy

Scope:

- `RuntimeRouter`,
- deterministic routing rules,
- tool policy allow/deny,
- route diagnostics.

Primary files:

- new `clients/frontend/src/main/services/weft-runtime-router.ts`
- new `clients/frontend/src/main/services/weft-tool-policy.ts`
- `clients/frontend/src/main/weft-core-plugin-client.ts`
- `packages/official/tool-runtime-core/src/*`

### Workstream C: Companion/game protocol

Scope:

- `OutputEnvelope`,
- `CompanionPerformance`,
- provider structured output fallback,
- prompt isolation,
- game overlay event adapter.

Primary files:

- `packages/official/companion-core/server.py`
- `clients/frontend/src/main/companion-chat-client.ts`
- `clients/frontend/src/main/companion-config-prompt.ts`
- `clients/frontend/src/renderer/components/MirrorCompanionApp.vue`
- `clients/frontend/src/renderer/components/CompanionSurfaceHost.vue`

### Workstream D: Delegation/session continuity

Scope:

- `DelegationEvent`,
- background lifecycle,
- subagent context scoping,
- completion landing messages,
- non-blocking session flow.

Primary files:

- `packages/official/agent-core/src/*`
- `clients/frontend/src/main/services/weft-core-host-service.ts`
- `clients/frontend/src/renderer/stores/chat.ts`
- `packages/official/tool-runtime-core/src/*` when delegate tasks need tool execution hooks

### Workstream E: Package/mod boundary

Scope:

- taxonomy,
- manifest validation,
- package graph diagnostics,
- mod manager categorization.

Primary files:

- `packages/index.toml`
- `packages/weft-claw/package.toml`
- `clients/frontend/src/main/services/weft-suite-sync-engine.ts`
- `clients/frontend/src/main/services/weft-suite-runtime-sync.ts`
- `clients/frontend/src/main/services/weft-plugin-manifest.ts`

### Workstream F: Overlay and UI UX

Scope:

- overlay close/hide,
- persisted visibility,
- mod manager basic affordances,
- visible route/task diagnostics where useful.

Primary files:

- `clients/frontend/src/main/companion-mirror-window-controller.ts`
- `clients/frontend/src/renderer/components/MirrorCompanionApp.vue`
- `clients/frontend/src/renderer/stores/chat.ts`
- mod/package manager components under `clients/frontend/src/renderer`

### Workstream G: Test harness

Scope:

- route fixtures,
- context snapshots,
- tool policy tests,
- companion output validation,
- delegation lifecycle tests,
- package graph tests.

Primary files:

- new scripts under `clients/frontend/scripts` or repo-level `scripts`, matching existing project style,
- existing package/plugin test locations where present.

## Testing Matrix

### Router fixtures

| Input | Expected route | Expected tools |
|---|---|---|
| `你好呀~` | `direct_companion` or `game_companion` | none |
| `search online for Uzi` | `web_search` | web only |
| `read D:/foo/a.txt` | `coding_action` | scoped fs read |
| `fix this repo and run tests` | `delegate_task` or `coding_action` | fs/git/shell scoped |
| `think deeply about the architecture` | `deep_think` | none unless requested |
| `open the game companion` | `game_companion` | companion/game UI only |

### Context fixtures

- Transcript-only follow-up.
- Transcript plus workspace context.
- Workspace context present but irrelevant.
- Memory relevant but older than latest user correction.
- Coding context detected while user asks a general web question.

### Tool policy fixtures

- Web route denies fs read.
- Companion route denies shell.
- Deep route denies web unless user asks.
- Coding route confines fs to workspace root.
- Delegate route passes a scoped policy to the subagent.

### Companion output fixtures

- Provider supports JSON schema.
- Provider rejects JSON schema.
- Provider returns extra prose around JSON.
- Provider returns old `|||` line.
- Emotion is invalid.
- Subtitle missing.

### Delegation fixtures

- Task queued.
- Task progress event.
- Task completed.
- Task failed.
- User sends another message while task runs.
- Completion lands in same session.

### Package/mod fixtures

- UI skin mod without tools.
- Feature mod requiring `tool.web`.
- Provider package not directly user-toggleable.
- Conflicting providers.
- Missing capability provider.

## Risks and Mitigations

### Risk: Big refactor breaks the app for too long

Mitigation:

- Keep compatibility facades.
- Add contracts first.
- Use feature flags or staged routing switch where practical.
- Keep old companion protocol adapter until new envelope is proven.

### Risk: Subagents edit overlapping files

Mitigation:

- Contract files are created first.
- Each workstream owns a narrow file list.
- One integration pass merges workstreams.
- No subagent should redefine core types independently.

### Risk: Prompt behavior still drifts

Mitigation:

- Snapshot assembled context bundles.
- Keep prompts small and route-specific.
- Enforce tool policy outside prompt.

### Risk: Provider structured output remains inconsistent

Mitigation:

- Treat native JSON schema as optimization only.
- Always validate parsed output.
- Add fallback parser and repair path.

### Risk: Package/mod taxonomy becomes too heavy

Mitigation:

- Keep product UI simple.
- Enforce taxonomy only in manifests/runtime validation.
- Do not require users to understand provider/capability internals.

## Out of Scope

- Re-signing package metadata and lock files.
- Rewriting all plugin packages from scratch.
- Force-changing remote repository layout.
- Building a full marketplace.
- Replacing WEFT-core HTTP/plugin architecture.
- Adding new model providers beyond making structured output fallback robust.

## Acceptance Criteria

The refactor is successful when:

1. A user can chat with one companion/session without losing transcript continuity.
2. Web search, deep thinking, coding action, and delegation choose separate routes with visible diagnostics.
3. Web search cannot access local files unless explicitly routed through a local/coding action.
4. Game companion output always has structured emotion, English voice text, and Chinese subtitle.
5. Heavy delegate work returns progress and completion into the same session.
6. The game overlay can be closed/hidden and remembers the user preference.
7. Mod/package UI can distinguish UI skins, feature bundles, provider packages, and products.
8. Contract tests cover routing, context priority, tool policy, structured output, delegation lifecycle, and package graph validation.
9. Future prompt or package changes fail tests if they reintroduce context/tool pollution.

## Recommended Implementation Order

1. Add shared contract types and debug snapshots.
2. Separate transcript/session/workspace context assembly.
3. Add runtime router and tool policy gate.
4. Convert companion/game output to `OutputEnvelope` and `CompanionPerformance`.
5. Stabilize delegation events under the same session.
6. Split large frontend/core API adapter into focused clients behind a facade.
7. Clean up package/mod taxonomy and validation.
8. Add overlay close/hide lifecycle.
9. Add the full regression matrix.
10. Remove old compatibility paths only after tests pass and the UI works through the new contracts.
