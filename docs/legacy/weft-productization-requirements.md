`# WEFT Productization Requirements: Lobe-style UX, Self-evolving Skills, MCP, Tools, and Web Search

## Purpose

This document captures the complete product requirements discussed for WEFT's next productization phase. The goal is to turn WEFT into a polished desktop agent experience with Lobe-style interaction quality, visible execution, self-evolving skills, reliable MCP/tool/web-search orchestration, and real natural-language operation from the Electron UI.

## Product Vision

WEFT should behave like a normal modern agent:

1. The user types natural language in the desktop UI.
2. The LLM participates in understanding intent and deciding whether to answer directly or use capabilities.
3. Deterministic rules provide safety, policy, and fallback rather than replacing LLM reasoning.
4. Tools, web search, skills, MCP, memory, and package/runtime status are visible and inspectable.
5. The UI makes execution understandable instead of hiding planning, routing, tool calls, failures, and results.
6. The system can evolve by discovering, installing, using, validating, and maintaining skills.

## Requirement Groups

### R1. Lobe-style Frontend and Interaction Experience

#### R1.1 Lobe-inspired chat UX

The frontend should provide a polished chat surface inspired by LobeHub-style agent products:

- Clear user and assistant message separation.
- Modern session list and persistent conversation management.
- Immediate visible feedback after submit.
- Streaming or staged response states where applicable.
- Companion persona output that feels natural while preserving agent execution transparency.
- Clear empty, loading, error, degraded, and ready states.
- No silent failures.
- No raw backend/internal errors shown as primary user copy.

#### R1.2 Execution visibility

The chat UI must visibly expose agent execution state:

- Planning started.
- Route selected.
- Tool call started.
- Tool result received.
- Response generation started.
- Response completed.
- Error/degraded/fallback state if execution cannot proceed.

The UI should distinguish:

- Companion preface.
- Actual agent/tool work.
- Final companion/user-facing answer.
- Tool output summaries.
- Diagnostics/debug details.

#### R1.3 Tool activity timeline

The UI must show a stable activity timeline for tool-backed turns:

- Tool call name, such as `web_search`, `web_fetch`, `fs_read`, or MCP echo.
- Tool status: running, finished, failed, skipped, blocked.
- Tool result summary.
- Policy reason if the tool is denied.
- Retryable vs non-retryable failures.

For web search specifically, the UI must make it clear that web search happened, even if no result was found.

#### R1.4 Dashboard/package graph visibility

The frontend should include a dashboard or management surface that exposes:

- Installed packages/plugins.
- Package graph/bindings.
- Missing bindings.
- Warnings and actionable diagnostics.
- Plugin runtime states.
- Tool/skill/MCP availability.
- Workspace wiki and plugin surfaces where applicable.

Warnings are allowed, but must be inspectable with title, detail, and recommended action.

#### R1.5 Electron UI validation requirement

All core user flows must be validated through real Electron UI tests, not only service/runtime tests:

- User can submit natural language prompt from the UI.
- Assistant reply appears.
- Tool activity is visible for tool-backed prompts.
- Web search activity is visible for web prompts.
- Skills/MCP activity is visible for relevant prompts.
- Dashboard and workspace/wiki surfaces load and expose expected states.

## R2. LLM-first Routing with Rule Fallback

### R2.1 Desired routing behavior

Routing should follow this order:

1. Build a safe fallback context using deterministic rules.
2. Ask the LLM/companion planner to classify or plan the turn.
3. Use LLM preface/reason/planned turn as the primary routing signal when valid.
4. Validate the route against an allowed route set and tool policy.
5. Fall back to deterministic rules when the LLM route is missing, invalid, unsafe, or too vague.
6. Execute with the selected route and corresponding tool policy.

The system should not rely solely on keyword matching to decide whether to search the web, use tools, or call skills.

### R2.2 Route categories

Supported route categories:

- `direct_companion`: direct answer, no tools.
- `web_search`: online lookup/search/fetch.
- `deep_think`: reasoning-heavy but non-tool task.
- `coding_action`: local code/file/workspace operation.
- `delegate_task`: heavier multi-step delegated work.
- `session_agent`: session-level agent work.
- `game_companion`: game companion surface.

### R2.3 Rule fallback responsibilities

Rules are still required for:

- Safety gating.
- Tool allow/deny policy.
- Filesystem root scoping.
- Network permission scoping.
- Shell/git denial or confirmation rules.
- Provider/auth diagnostics.
- Handling missing/invalid LLM output.
- Regression coverage for obvious intents.

Rules should be a safety net and deterministic fallback, not the primary intelligence layer.

### R2.4 Chinese natural-language routing

Chinese prompts must be treated as first-class input.

Examples that should route to `web_search`:

- `搜索一下无尽爱丽丝`
- `搜一下无尽爱丽丝`
- `查一下无尽爱丽丝`
- `联网查无尽爱丽丝`
- `上网查无尽爱丽丝`
- `网络搜索无尽爱丽丝`

The system should not fail with an empty plan for these prompts.

### R2.5 Empty-plan handling

The frontend must not expose raw `agent turn plan was empty` as the main user-facing answer.

Required behavior:

- Convert empty-plan failures to friendly user-facing text.
- Preserve raw backend error in trace/debug detail.
- Make the failure retryable when appropriate.
- Prefer route/tool fallback if possible.
- Never silently drop the turn.

## R3. Web Search Requirements

### R3.1 Natural-language web search

A user should be able to type a natural-language web request in the Electron UI and have WEFT:

1. Identify the need for web search.
2. Route to `web_search`.
3. Apply web-only tool policy.
4. Call web search/fetch tools.
5. Show tool activity in UI.
6. Return a concise answer or a clear no-result/degraded message.

### R3.2 Web tool policy

When route is `web_search`:

- Allow: web tools.
- Deny: filesystem, shell, git.
- Network allowed: true.
- Shell/git allowed: false.
- Filesystem roots: empty unless explicitly needed and separately authorized.

### R3.3 Web tool aliases

The system must consistently handle available web tool names and aliases:

- `web_search`
- `websearch`
- `web_fetch`

Tool aliasing should be normalized so planner/tool execution/UI display agree.

### R3.4 Web search result UX

The UI should handle:

- Successful search with results.
- Search with no concise result.
- Provider/auth failure.
- Network failure.
- Tool policy denial.
- Search timeout.

No-result is not the same as tool failure. It should be shown as a completed search with a no-result summary.

## R4. Tool System Requirements

### R4.1 Tool orchestration

WEFT must support natural-language access to tools through the agent runtime.

Tool flow:

1. Planner decides whether tools are needed.
2. Runtime selects available tool(s) under policy.
3. Tool calls execute with bounded permissions.
4. Tool results are recorded.
5. Renderer receives tool events.
6. UI exposes tool activity and final answer.

### R4.2 Tool policy enforcement

Tool policy must be authoritative and route-aware:

- Direct companion: no tools.
- Web search: only web tools.
- Coding/session/delegate: local tools only when workspace root and policy allow.
- Shell/git require explicit route and should remain constrained.
- MCP and skills should be explicit capabilities, not accidental unrestricted tools.

### R4.3 Tool event visibility

For every tool-backed turn, renderer/UI should expose:

- Tool call event.
- Tool result event.
- Tool error event if applicable.
- Final completion event.

These should be testable through Electron UI artifacts.

### R4.4 Diagnostics

Tool failures must be classified clearly:

- Provider/auth diagnostic.
- Tool unavailable.
- Policy denied.
- Network unavailable.
- Runtime unavailable.
- Invalid arguments.
- Timeout.
- No result.

Diagnostics must not leak secrets, API keys, tokens, or raw credentials.

## R5. Skills Requirements

### R5.1 Skills as first-class capabilities

Skills are official WEFT capabilities that can be discovered, listed, invoked, and maintained.

Requirements:

- Skills plugin available in managed suite.
- Skills listed in frontend/package surfaces.
- Natural-language planner can select skill-backed actions.
- Runtime can invoke skills safely.
- Renderer can receive skill/tool results.
- UI can display skill activity.

### R5.2 Skill examples

The validation baseline must include at least one deterministic skill path, such as:

- `fs_read` skill for reading a prepared test file/marker.
- A skill listing path.
- A skill maintenance/list-available path.

### R5.3 Skill UI requirement

For a natural-language skill prompt in the Electron UI, the user should see:

- That a skill/tool is being used.
- Which skill/tool is used.
- Whether it completed or failed.
- A final answer using the skill result.

### R5.4 Skill safety

Skills must obey:

- Tool policy.
- Workspace scoping.
- Filesystem scoping.
- No secret leakage.
- No destructive action unless explicitly authorized.

## R6. MCP Requirements

### R6.1 MCP client capability

MCP must be supported as a managed capability:

- MCP plugin loaded.
- MCP tools discoverable.
- MCP tool calls executable.
- MCP echo/test server path available for validation.
- MCP diagnostics visible.

### R6.2 Natural-language MCP invocation

A user should be able to ask for an MCP-backed action and have WEFT:

1. Detect that MCP/tool use is needed.
2. Select the MCP tool under policy.
3. Execute the MCP tool.
4. Display MCP tool activity in UI.
5. Include MCP result in the final answer.

### R6.3 MCP validation

Validation must include:

- IPC/tooling-level MCP echo test.
- Runtime natural-language MCP echo test.
- Electron UI natural-language MCP-visible test.

The current known gap is the Electron UI visible test.

## R7. Self-evolving Skills Requirements

### R7.1 Self-evolving MVP

The system should support a self-evolving skills loop:

1. Discover missing capability or repeated user need.
2. Propose a skill.
3. Create or install the skill.
4. Register it in the skill/plugin system.
5. Validate it.
6. Make it available to future agent turns.
7. Run maintenance/cron tasks as needed.

### R7.2 Components

The MVP covers the following areas:

- `packages/official/skills`
- `packages/official/agent-core`
- `packages/official/cron`

### R7.3 Safety and governance

Self-evolution must be constrained by:

- Human-visible proposal/trace.
- No silent destructive changes.
- Validation before promotion.
- Clear failure diagnostics.
- Version/source metadata.
- Ability to inspect installed skills.

### R7.4 Validation

Self-evolving skill behavior should be validated at:

- Plugin/package layer.
- Runtime invocation layer.
- UI listing/visibility layer.
- Maintenance/cron layer.

## R8. Package Graph, Plugin, and Dashboard Requirements

### R8.1 Managed suite health

The managed suite must correctly configure package/plugin bindings, including:

- `agent_runtime`
- `companion_runtime`
- `skills`
- `mcp-client`
- `memory`
- `context-engine`
- web tools

Binding identity must be stable and not drift between package directory name and internal manifest identity.

### R8.2 Package graph diagnostics

Dashboard/package graph should show:

- Installed packages.
- Missing bindings.
- Parse warnings.
- Multiple candidates.
- Optional vs required packages.
- Actionable warnings.

Warnings do not fail the product if they are inspectable and non-blocking.

### R8.3 Workspace wiki/plugin surfaces

Workspace wiki/plugin surfaces must load independently when optional services are unavailable where appropriate. Optional wiki standalone readiness should not block dashboard validation unless the specific wiki test requires it.

## R9. Validation Requirements

### R9.1 Validation layers

Every core feature should be validated through multiple layers:

1. Unit/contract tests.
2. Runtime/service tests.
3. IPC/tooling tests.
4. Electron UI tests.
5. Evidence artifact consolidation.

### R9.2 Required validation commands/scripts

Relevant scripts include:

- `npm run lint`
- `npm run build`
- `npm run typecheck:ui`
- `scripts/test-weft-runtime-router-contract.ts`
- `scripts/test-companion-runtime-controller.ts`
- `scripts/test-agent-natural-language-tool-chain.ts`
- `scripts/test-weft-electron-tooling-ipc.ts`
- `scripts/test-companion-electron-natural-language-smoke.ts`
- `scripts/probe-weft-dashboard-window.ts`
- `scripts/probe-weft-plugin-workspace-wiki-surface.ts`
- `scripts/test-validation-cross-flow-closure.ts`

### R9.3 Required evidence

Evidence artifacts should include:

- Run id.
- Source script.
- Assertion ids.
- Screenshot for UI tests.
- Lifecycle JSON.
- Tool events.
- Diagnostics classification.
- Redaction status.
- Cross-flow closure status.

### R9.4 Redaction

Validation artifacts must not expose:

- API keys.
- OAuth tokens.
- Provider secrets.
- Local credentials.
- Raw sensitive config.

### R9.5 Current validation status

Known validated areas:

- Router contract for Chinese web intent.
- Runtime controller LLM-first/fallback behavior.
- Runtime natural-language tool-chain for web search, skills, MCP, and renderer host effects.
- Tooling IPC for web tools, skills, MCP.
- Dashboard/package graph/workspace wiki probes from prior validation.

Known remaining gap:

- True Electron UI chat prompt must visibly show tool/websearch/skill/MCP activity, not only produce a companion reply.

## R10. Acceptance Criteria

### R10.1 LLM-first routing acceptance

Accepted when:

- LLM preface/plannedTurn can influence final route/toolPolicy.
- Invalid or absent LLM signal falls back to deterministic route rules.
- Route/tool policy remains safe.
- Tests prove a route can be derived from LLM preface, not only original user keyword matching.

### R10.2 Websearch acceptance

Accepted when:

- `搜索一下无尽爱丽丝` from runtime routes to `web_search`.
- Web-only policy is applied.
- Web tool result or clear no-result diagnostic is produced.
- Electron UI shows that web search happened.

### R10.3 Tools acceptance

Accepted when:

- Natural-language runtime can call tools.
- Renderer receives tool results.
- Electron UI visibly shows tool call/result lifecycle.

### R10.4 Skills acceptance

Accepted when:

- Skills are discoverable.
- A deterministic skill can be invoked from natural language.
- Skill result reaches final answer.
- Electron UI visibly shows skill/tool activity.

### R10.5 MCP acceptance

Accepted when:

- MCP echo/tool is discoverable.
- MCP can be invoked from natural language.
- MCP result reaches final answer.
- Electron UI visibly shows MCP activity.

### R10.6 Lobe-style frontend acceptance

Accepted when:

- Chat UI is polished and session-aware.
- Execution timeline is visible.
- Tool states are stable and inspectable.
- Dashboard/package graph surfaces are usable.
- Failures are friendly but traceable.

## Current Implementation Snapshot

> Last updated: 2026-05-21

### Implemented ✅

- weft-claw: send_message, list_sessions, get_session_messages, list_events, reset_session, cancel_turn (KV flag mechanism)
- agent-core: loop detection (consecutive identical tool calls auto-abort), RequiredActionState intent gating (16 fields), session events integration, evolved skill trajectory promotion
- web_search: routes via tool-web (js-extension-runtime/Exa) with Brave HTML fallback
- web_fetch: implemented via curl in tool-runtime-core
- fs_read / fs_write / fs_list / shell_exec / git: implemented in tool-runtime-core
- Python script path support in shell_exec (.py files auto-detected)
- tool-web: complete rewrite delegating to js-extension-runtime
- Self-evolving skills MVP across official skills/agent-core/cron areas
- Dashboard/package graph readiness and probes
- Workspace wiki/plugin surface validation
- Runtime natural-language tool-chain for web search, skills, MCP
- Chinese web intent route support
- LLM-first route resolution with deterministic fallback in frontend controller
- Friendly empty-plan error mapping in chat store

### Partially implemented ⚠️

- mcp-client: implemented in `packages/official/` but **not loaded** in `packages/installed/` — MCP tool calls unavailable at runtime
- prompt-system: stub only — agent prompt rendering falls back to agent-core defaults
- workflow-orchestrator: stub only — no multi-stage task orchestration
- Lobe-style frontend execution visibility
- Electron chat UI natural-language web search flow
- UI artifact capture for activity/tool states

### Not yet complete ❌

- Real Electron UI proof that tool/websearch calls are visibly shown after a typed prompt
- Real Electron UI proof for visible skill invocation
- Real Electron UI proof for visible MCP invocation
- Strong UI assertions for `toolCallVisible`, `toolResultVisible`, and `webSearchResultVisible`
- Approval gate wired to tool execution (policy model designed, not connected)
- ask_user tool (agent-initiated user input pause)

## Open Work Items

1. Fix Electron chat UI activity capture so tool calls/results are visible in artifacts.
2. Ensure `test-companion-electron-natural-language-smoke.ts` proves websearch visibility, not just companion reply.
3. Add or strengthen separate Electron UI flows for skill and MCP prompts.
4. Decide whether tool activity should live in the chat transcript, side timeline, or both.
5. Ensure no-result web search still produces visible completed tool state.
6. Preserve friendly user-facing errors while retaining raw diagnostics in trace/debug metadata.
7. Consolidate final GUI evidence manifest after UI tool visibility is proven.

## Non-goals

- Do not grant unrestricted shell/git/filesystem access from general chat.
- Do not treat keyword matching as the only intelligence layer.
- Do not hide tool execution from the user.
- Do not expose secrets in diagnostics or artifacts.
- Do not require optional standalone wiki availability to block unrelated dashboard/chat validation.

## Guiding Principle

WEFT should feel like a capable desktop agent: the LLM understands, rules protect, tools act, UI explains, and skills evolve safely.
`