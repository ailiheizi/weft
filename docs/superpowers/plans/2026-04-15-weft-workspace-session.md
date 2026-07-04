# WEFT Workspace Session Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real desktop workspace layer above sessions, persist session workspace ownership, and make terminal creation inherit the active workspace root.

**Architecture:** Desktop owns workspace tabs and active workspace selection. Sessions are persisted with `workspaceId/workspaceRoot` through the existing main -> agent-core session path. Terminal remains session-bound, but `terminal:create` receives the current workspace root as `cwd`.

**Tech Stack:** Vue 3, Pinia, Electron IPC, TypeScript, Rust wasm plugin

---

### Task 1: Lock the new mechanism with failing contract tests

**Files:**
- Create: `weft/scripts/test-workspace-session-contract.ts`
- Create: `weft/scripts/test-terminal-workspace-cwd-contract.ts`

- [ ] Step 1: Run the workspace contract test and confirm it fails

Run: `npx tsx weft/scripts/test-workspace-session-contract.ts`
Expected: FAIL because `DesktopWorkspace`, `workspaceId`, and workspace IPC/store hooks do not exist yet.

- [ ] Step 2: Run the terminal cwd contract test and confirm it fails

Run: `npx tsx weft/scripts/test-terminal-workspace-cwd-contract.ts`
Expected: FAIL because `terminal:create` does not accept `cwd` yet.

- [ ] Step 3: Commit the failing tests after implementation is complete, not before

Commit message: `test: lock workspace session contract`

### Task 2: Add workspace/session metadata to shared/main/agent-core

**Files:**
- Modify: `weft/src/shared/types.ts`
- Modify: `weft/src/main/weft-core-api-client.ts`
- Modify: `weft/src/main/weft-core-session-store.ts`
- Modify: `weft/src/main/services/weft-host-primitives.ts`
- Modify: `weft-plugins/agent-core/src/lib.rs`

- [ ] Step 1: Add shared desktop workspace and session workspace typing
- [ ] Step 2: Extend main-process session summary/config types with `workspaceId` and `workspaceRoot`
- [ ] Step 3: Extend agent-core session record and create/update/list payloads with `workspace_id` and `workspace_root`
- [ ] Step 4: Default missing legacy session metadata to global workspace

### Task 3: Add desktop workspace persistence and renderer store behavior

**Files:**
- Modify: `weft/src/preload/index.ts`
- Modify: `weft/src/main/ipc-handlers.ts`
- Modify: `weft/src/renderer/global.d.ts`
- Modify: `weft/src/renderer/stores/chat.ts`

- [ ] Step 1: Add `db:workspaces-list` and `db:workspaces-save` IPC using existing storage layer
- [ ] Step 2: Add `DesktopWorkspace` state to `chatStore`
- [ ] Step 3: Ensure default global workspace is always present
- [ ] Step 4: Hydrate legacy sessions into the default workspace when `workspaceId` is missing
- [ ] Step 5: Add `filteredSessions`, `activeWorkspace`, `createWorkspace`, and `switchWorkspace`

### Task 4: Thread workspace root into terminal creation

**Files:**
- Modify: `weft/src/preload/index.ts`
- Modify: `weft/src/main/services/terminal-pty-service.ts`
- Modify: `weft/src/renderer/components/use-session-terminal-workspace.ts`
- Modify: `weft/src/renderer/components/TerminalHost.vue`

- [ ] Step 1: Extend preload terminal `create` signature to accept `cwd`
- [ ] Step 2: Extend main terminal IPC and PTY spawn to accept and apply `cwd`
- [ ] Step 3: Pass the active session workspace root into `useSessionTerminalWorkspace`
- [ ] Step 4: Keep the existing split-pane terminal model unchanged

### Task 5: Switch the UI from session tabs to workspace tabs

**Files:**
- Modify: `weft/src/renderer/components/ChatPanel.vue`
- Modify: `weft/src/renderer/components/Sidebar.vue`

- [ ] Step 1: Replace top session tabs with workspace tabs
- [ ] Step 2: Replace the old `+` new-chat button with folder-open workspace creation/open action
- [ ] Step 3: Keep session creation in the sidebar, but create sessions inside the active workspace
- [ ] Step 4: Filter the sidebar lists by current workspace only

### Task 6: Verify and cleanly close

**Files:**
- Modify: `docs/superpowers/specs/2026-04-15-weft-workspace-session-design.md`
- Modify: `docs/superpowers/plans/2026-04-15-weft-workspace-session.md`

- [ ] Step 1: Run `npx tsx weft/scripts/test-workspace-session-contract.ts`
- [ ] Step 2: Run `npx tsx weft/scripts/test-terminal-workspace-cwd-contract.ts`
- [ ] Step 3: Run `npm run typecheck:ui` in `D:\weft\weft`
- [ ] Step 4: Review diffs for unrelated file contamination and stage only workspace-session files
- [ ] Step 5: Commit with message `feat: add desktop workspace session model`
