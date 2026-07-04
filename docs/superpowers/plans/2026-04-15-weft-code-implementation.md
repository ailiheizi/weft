# WEFT-Code Implementation Plan

> **For agentic workers:** Execute this plan continuously in-session by default. Do not stop to ask the user which execution mode to use. Move from one natural checkpoint to the next smallest justified checkpoint autonomously, and only pause when a hard blocker, real ambiguity, or explicit user intervention requires it. Keep checkbox (`- [ ]`) tracking updated as work is completed.

**Goal:** Introduce `WEFT-Code` as a new product package and runtime/client architecture on top of the current WEFT vNext repository, while maximizing reuse of existing packages and preserving the current platform model. In the corrected package-managed architecture, `weft-core` starts `weft-code-runtime` as a service package, while `weft-code-tui` remains a client only.

**Architecture:** `WEFT-Code` will be introduced as a new product package plus two new product-facing feature packages, with existing runtime/tool/team packages reused underneath. The first implementation slice keeps one local runtime process and introduces an HTTP + event-stream control surface that future CLI/TUI clients can share, while avoiding premature rewrites of the whole repo. The runtime must be internally split into bootstrap, session, task, workflow-gateway, approval/policy, and event-publisher responsibilities so that later clients do not force another architectural rewrite.

**Tech Stack:** Rust workspace (`core`, `server`), TOML package manifests, Axum HTTP APIs, SSE/event streaming, existing WEFT vNext package/declaration/config/lock model, Markdown architecture docs.

---

## File Structure and Responsibilities

### New files to create

- `packages/weft-code/package.toml`
  - The new product package declaration for WEFT-Code.
- `packages/official/weft-code-core-feature/plugin.toml`
  - Product-facing feature package for single-agent coding workflow.
- `packages/official/weft-code-team-feature/plugin.toml`
  - Product-facing feature package for team mode.
- `docs/architecture/vNext/14-weft-code-runtime-model.md`
  - Canonical architecture note for `core + runtime + clients` and the protocol/resource model.
- `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
  - This plan.
- `server/src/weft_code/mod.rs`
  - Top-level runtime module wiring WEFT-Code routes.
- `server/src/weft_code/types.rs`
  - Shared transport/resource structs for project/session/task/mode/team/approval/event.
- `server/src/weft_code/routes.rs`
  - Route registration for WEFT-Code runtime endpoints.
- `server/src/weft_code/events.rs`
  - Event-stream types and SSE response helpers.
- `server/src/weft_code/state.rs`
  - In-memory runtime state for initial P0 bootstrap.
- `server/src/weft_code/service.rs`
  - Runtime orchestration surface between HTTP handlers and core-facing logic.
- `server/src/weft_code/session_service.rs`
  - Session lifecycle and lineage management.
- `server/src/weft_code/task_service.rs`
  - Task lifecycle, status transitions, and parent/child task relations.
- `server/src/weft_code/policy_service.rs`
  - Approval policy resolution and approval request state.
- `server/src/weft_code/workflow_gateway.rs`
  - Mode-to-workflow routing and task creation rules.

### Existing files to modify

- `packages/index.toml`
  - Register the new product package and feature packages in the package index.
- `docs/architecture/vNext/00-overview.md`
  - Add WEFT-Code as the first explicit code-product direction under the product package model.
- `docs/architecture/vNext/13-product-package-instance-model.md`
  - Clarify how a product package like WEFT-Code maps to runtime + clients.
- `packages/README.md`
  - Update package taxonomy/examples so WEFT-Code is visible as a real product package.
- `server/src/main.rs`
  - Extract WEFT-Code route wiring and stop growing main.rs as a monolith.
- `Cargo.toml`
  - Add any new workspace crates only if strictly necessary; avoid this unless the server split truly requires it.

### Existing packages to reuse as-is initially

- `packages/official/prompt-system/`
- `packages/official/workflow-orchestrator/`
- `packages/official/tool-runtime-core/`
- `packages/official/tool-shell/`
- `packages/official/tool-files/`
- `packages/official/tool-git/`
- `packages/official/tool-web/`
- `packages/official/mcp-client/`
- `packages/official/skills/` or `skills-runtime`-mapped package
- `packages/official/memory-runtime/` and `packages/official/memory/`
- `packages/official/team-runtime/`
- `packages/official/team-task-board/`
- `packages/official/workflow-template-devteam/`

### Responsibilities by layer

- **Core:** owns workflow, task, team, tool, approval, memory semantics.
- **Runtime:** exposes product-facing transport/resources/events; does not invent new core semantics.
- **Runtime Bootstrap:** loads product defaults and instance overrides into one running runtime context.
- **Session Service:** owns session truth, lifecycle, and lineage.
- **Task Service:** owns task truth and state transitions.
- **Workflow Gateway:** maps `coding / plan / team` into runtime actions without re-implementing core semantics.
- **Approval / Policy Service:** owns approval decisions and policy projection.
- **Clients:** future CLI/TUI/GUI programs consume runtime APIs; they do not directly own product truth.

---

## Phase 0: Product Package + Runtime Skeleton

### Task 1: Add WEFT-Code as a real product package

**Files:**
- Create: `packages/weft-code/package.toml`
- Modify: `packages/index.toml`
- Reference: `packages/weft-claw/package.toml`
- Reference: `docs/superpowers/specs/2026-04-15-weft-code-design.md`

- [x] **Step 1: Copy the Weft Claw product manifest structure into a new WEFT-Code package manifest**

Use `packages/weft-claw/package.toml` as the structural template, but make the new file product-specific.

Write this initial skeleton into `packages/weft-code/package.toml`:

```toml
schema_version = 2

[identity]
name = "weft-code"
version = "0.1.0"
display_name = "WEFT-Code"
description = "统一 runtime 驱动的 CLI/TUI code product"

[package]
kind = "product"
```

- [x] **Step 2: Define the first-pass required capabilities**

Append a first-pass `requires.capabilities` block that expresses the design spec without inventing new runtime capabilities yet:

```toml
[requires]
capabilities = [
  "agent.runtime",
  "prompt.system",
  "workflow.orchestration",
  "tool.runtime",
  "tool.shell",
  "tool.files",
  "tool.git",
  "memory.store",
  "ext.skills",
  "ext.mcp",
  "team.runtime",
  "team.taskboard",
  "team.handoff",
  "team.role.catalog",
  "team.delegate",
  "team.context.shared",
  "workflow.template.devteam",
  "weft_code.runtime",
]
```

- [x] **Step 3: Declare WEFT-Code’s two product-facing features**

Add a minimal feature section:

```toml
[features]
default_enabled = [
  "weft-code-core-feature",
]
optional = [
  "weft-code-team-feature",
]
```

- [x] **Step 4: Add initial feature bindings that explicitly reuse existing packages**

Add these feature binding sections:

```toml
[feature_bindings.weft-code-core-feature]
requires = ["agent.runtime", "prompt.system", "workflow.orchestration", "tool.runtime", "tool.shell", "tool.files", "tool.git", "memory.store"]
packages = ["weft-code-core-feature", "agent-runtime", "prompt-system", "workflow-orchestrator", "tool-runtime-core", "tool-shell", "tool-files", "tool-git", "memory-store"]

[feature_bindings.weft-code-team-feature]
requires = ["team.runtime", "team.taskboard", "team.handoff", "team.role.catalog", "team.delegate", "team.context.shared", "workflow.template.devteam", "memory.store", "ext.mcp", "ext.skills"]
packages = ["weft-code-team-feature", "team-runtime", "team-task-board", "workflow-template-devteam", "memory-store", "mcp-client", "skills-runtime"]

[bindings."weft_code.runtime"]
provider = "weft-code-runtime"
mutable = false
```

- [x] **Step 5: Register WEFT-Code in `packages/index.toml` using the current `package_sources` shape**

Add a `[[package_sources]]` entry matching the current index conventions and pointing at the product source directory:

```toml
[[package_sources]]
name = "weft-code"
package_kind = "product"
current_source = "packages/weft-code"
```

Keep the remaining required source fields aligned with the existing index entries already used by this repo. Do not use `[packages.weft-code]`.

- [x] **Step 6: Run a manifest sanity read**

Run the smallest possible verification commands that match the current repo, prove the product manifest parses through the repo-native loader, and prove the package index exposes `weft-code` without TOML parse errors:

```bash
cargo test -p weft-core weft_code_product_manifest_binds_runtime_service
cargo test -p weft-core weft_code_package_index_registers_runtime_service_and_product_source
```

Expected: both tests pass, `packages/weft-code/package.toml` binds `weft_code.runtime` to `weft-code-runtime`, the package index exposes `weft-code-runtime` as a service provider, and the `weft-code` entry remains a product with `current_source = "packages/weft-code"`.

> 2026-04-15 execution note
>
> - Completed this round: Steps 1-5 via `packages/weft-code/package.toml` and `packages/index.toml`.
> - Reference mismatch repaired in plan: this task now points at `packages/weft-claw/package.toml`.
> - Index registration repaired in plan: Step 5 now uses the repo's `[[package_sources]]` shape instead of `[packages.weft-code]`.
> - Verification contract repaired in plan: Step 6 now uses focused tests that prove `packages/weft-code/package.toml` parses through the repo-native loader and `weft-code` is discoverable from `packages/index.toml` without requiring a long-running process.
> - Step 6 has now passed with the focused manifest/index tests and the route syntax regression test is also green.
> - Historical note: earlier registry-server-focused checks belonged to an older transition step. That wording is migration history only and not the current product model.
> - Next: Task 1 is complete; move to Task 2.

Task 1 round result

- Files: `packages/weft-code/package.toml`, `packages/index.toml`, `server/src/main.rs`, `core/src/app/config.rs`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`, `.sisyphus/ralph-loop.local.md`
- Verified:
  - `cargo metadata --no-deps --format-version 1` confirmed the workspace package is `registry-server`, not `server`.
  - `cargo test -p weft-core weft_code_product_package_manifest_parses_with_repo_loader` passed and confirmed `packages/weft-code/package.toml` parses through the repo-native product package loader.
  - Historical note: a registry-server route check also passed during that earlier transition, but it is not the current architecture contract.
  - `cargo test -p registry-server production_route_patterns_do_not_panic_during_router_build` passed after updating the Axum path syntax from `:name` to `{name}`.
- Historical note: earlier registry-server checks passed during migration, but they are not the current contract for WEFT-Code package management.
- Notes:
  - Task 1 is now complete.
  - Step 5 is now executed with a repo-compatible `[[package_sources]]` entry for `weft-code`.
  - Step 6 is now verified by repo-native focused tests instead of a long-running `--help` process.
  - Scope review: this round stayed within Task 1 files only and did not modify any Task 2 feature package manifests.
- Next: move to Task 2.

---

### Task 2: Introduce explicit WEFT-Code feature packages

**Files:**
- Create: `packages/official/weft-code-core-feature/plugin.toml`
- Create: `packages/official/weft-code-team-feature/plugin.toml`
- Modify: `packages/index.toml`
- Reference: `packages/official/weft-claw-chat-feature/plugin.toml`
- Reference: `packages/official/weft-claw-devteam-feature/plugin.toml`

- [x] **Step 1: Inspect existing feature manifest shape and copy only the stable fields**

Open the existing Weft Claw feature manifests and mirror their field order and naming. Do not invent a new manifest shape.

- [x] **Step 2: Create the core feature package manifest**

Write an initial `plugin.toml` for `packages/official/weft-code-core-feature/` using the current embedded feature manifest shape already used by the Weft Claw feature packages:

```toml
schema_version = 1

[identity]
name = "weft-code-core-feature"
version = "0.1.0"
description = "WEFT-Code 单人 coding 工作流能力块"

[package]
entry = "README.md"
runtime = "embedded"
api_version = "v1"
provides = ["feature.weft_code.core"]

[capability]
provides = ["feature.weft_code.core"]
```

- [x] **Step 3: Create the team feature package manifest**

Write an initial `plugin.toml` for `packages/official/weft-code-team-feature/` using the current embedded feature manifest shape already used by the Weft Claw feature packages:

```toml
schema_version = 1

[identity]
name = "weft-code-team-feature"
version = "0.1.0"
description = "WEFT-Code team mode 工作流能力块"

[package]
entry = "README.md"
runtime = "embedded"
api_version = "v1"
provides = ["feature.weft_code.team"]

[capability]
provides = ["feature.weft_code.team"]
```

- [x] **Step 4: Register both new feature packages in the package index**

Add entries for both manifests in `packages/index.toml`, matching current index conventions.

- [x] **Step 5: Verify no existing feature names collide**

Run a targeted search:

```bash
rg "weft-code-core-feature|weft-code-team-feature" packages docs
```

Expected: the new declarations appear in the newly added manifests and package-index entries, plus the expected existing references in the WEFT-Code product manifest and design/plan docs.

> 2026-04-15 execution note
>
> - Completed this round: created `packages/official/weft-code-core-feature/plugin.toml`, `packages/official/weft-code-team-feature/plugin.toml`, and registered both in `packages/index.toml`.
> - Added the referenced embedded feature package README assets so the new manifests' `entry = "README.md"` paths resolve consistently with the existing Weft Claw feature packages.
> - Plan text aligned with execution: Task 2 now documents the same embedded feature manifest shape and collision-search expectation used by the repo and by this round's implementation.
> - Verification used the plan-aligned focused checks only.
> - `cargo test -p registry-server package_source_route_returns_source_url_and_entries` remained green after the index updates.
> - `cargo test -p weft-core weft_code_product_package_manifest_parses_with_repo_loader` remained green after the feature package declarations were added.
> - Added a direct repo-native plugin manifest loader test for both new feature packages in `core/src/plugin/config.rs`.
> - Collision search for `weft-code-core-feature|weft-code-team-feature` now finds only the new declarations plus the expected references in existing spec/plan/product files.
> - Next: Task 2 is ready for review; do not start Task 3 until Task 2 is accepted.

Task 2 round result

- Files: `packages/official/weft-code-core-feature/plugin.toml`, `packages/official/weft-code-core-feature/README.md`, `packages/official/weft-code-team-feature/plugin.toml`, `packages/official/weft-code-team-feature/README.md`, `packages/index.toml`, `core/src/plugin/config.rs`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified:
  - `cargo test -p registry-server package_source_route_returns_source_url_and_entries` passed.
  - `cargo test -p weft-core weft_code_product_package_manifest_parses_with_repo_loader` passed.
  - `cargo test -p weft-core test_load_weft_code_feature_manifests` passed and confirmed both new feature manifests load through the repo-native plugin manifest loader.
  - Targeted collision search found the two new feature names only in the newly added manifests, their package-index entries, and expected existing spec/plan/product references.
  - The new embedded feature packages now contain the `README.md` assets referenced by their manifests' `entry` fields.
- Notes:
  - Task 2 stayed within feature package manifests and package index registration only.
  - Verification stayed within the focused checks explicitly chosen for this task.
  - The new feature manifests are now directly covered by a repo-native loader test, not only by indirect product/index checks.
- Next: request Oracle review for Task 2 completion.

---

### Task 3: Write the architecture convergence note for runtime + clients

**Files:**
- Create: `docs/architecture/vNext/14-weft-code-runtime-model.md`
- Modify: `docs/architecture/vNext/00-overview.md`
- Modify: `docs/architecture/vNext/13-product-package-instance-model.md`
- Reference: `docs/superpowers/specs/2026-04-15-weft-code-design.md`

- [x] **Step 1: Create the new canonical runtime note**

Start the file with this exact heading and summary:

```md
# WEFT vNext 蓝图：WEFT-Code Runtime + Clients 模型

本文定义 WEFT-Code 如何在不破坏 Product Package + Instance 模型的前提下，引入统一 runtime 与多客户端结构。
```

- [x] **Step 2: Add the three-layer statement exactly**

Include this architecture statement verbatim or near-verbatim:

```md
- Core 提供能力与领域真相
- Runtime 承载并暴露 WEFT-Code 的运行中接口
- CLI / TUI / GUI 作为 clients 连接 runtime
```

- [x] **Step 3: Add resource-model sections that match the spec**

The new doc must contain sections for:

```md
- project
- session
- task
- mode
- team
- approval
- event stream
```

- [x] **Step 4: Update the overview index to include the new runtime doc**

Add a new ordered item after the current product-package-instance note:

```md
12. `14-weft-code-runtime-model.md` — WEFT-Code runtime 与 clients 模型
```

- [x] **Step 5: Update the product-package-instance doc so runtime is treated as runtime, not a second product object**

Add language stating that a product package like `weft-code` can be realized by a runtime that exposes product-facing APIs to multiple clients, without reintroducing App as a separate source-level object.

> 2026-04-15 execution note
>
> - Completed this round by aligning the existing Task 3 documents to the plan rather than creating a second runtime note.
> - `docs/architecture/vNext/14-weft-code-runtime-model.md` already existed, so Step 1 was executed as an in-place canonicalization pass.
> - The runtime note now uses the planned title form `Runtime + Clients`, contains the required one-line summary, and includes the explicit three-layer statement.
> - `docs/architecture/vNext/00-overview.md` now uses the same index label for the runtime note.
> - `docs/architecture/vNext/13-product-package-instance-model.md` now explicitly states that runtime is runtime itself, not a second product object.
> - Next: Task 3 is ready for review; do not start Task 4 until Task 3 is accepted.

Task 3 round result

- Files: `docs/architecture/vNext/14-weft-code-runtime-model.md`, `docs/architecture/vNext/00-overview.md`, `docs/architecture/vNext/13-product-package-instance-model.md`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified:
  - `14-weft-code-runtime-model.md` now starts with the planned heading and summary.
  - The runtime note contains the required three-layer statement and the required resource sections for project / session / task / mode / team / approval / event stream.
  - `00-overview.md` includes the runtime note in the ordered index with the aligned `runtime + clients` label.
  - `13-product-package-instance-model.md` now explicitly states that runtime is runtime itself, not a second product object, while preserving the no-App-source-layer boundary.
- Notes:
  - Task 3 stayed within documentation alignment only.
  - No Task 4 runtime transport or server module files were modified.
- Next: request Oracle review for Task 3 before any completion claim.

---

## Phase 1: Runtime Protocol and Transport Surface

### Task 4: Extract WEFT-Code runtime modules from `server/src/main.rs`

**Files:**
- Create: `server/src/weft_code/mod.rs`
- Create: `server/src/weft_code/routes.rs`
- Create: `server/src/weft_code/types.rs`
- Create: `server/src/weft_code/state.rs`
- Create: `server/src/weft_code/service.rs`
- Create: `server/src/weft_code/session_service.rs`
- Create: `server/src/weft_code/task_service.rs`
- Create: `server/src/weft_code/policy_service.rs`
- Create: `server/src/weft_code/workflow_gateway.rs`
- Modify: `server/src/main.rs`
- Reference: `server/src/main.rs`

- [x] **Step 1: Add the new module declaration in `server/src/main.rs`**

Near the top of `main.rs`, add:

```rust
mod weft_code;
```

- [x] **Step 2: Create the runtime module root**

Write `server/src/weft_code/mod.rs` as:

```rust
pub mod events;
pub mod policy_service;
pub mod routes;
pub mod service;
pub mod session_service;
pub mod state;
pub mod task_service;
pub mod types;
pub mod workflow_gateway;
```

- [x] **Step 3: Create transport structs for the first-class resources**

In `server/src/weft_code/types.rs`, define at least these starter enums/structs:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    Coding,
    Plan,
    Team,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    Active,
    Compacting,
    Paused,
    Resumed,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Blocked,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub root: String,
    pub product: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub project_id: String,
    pub mode: SessionMode,
    pub status: SessionStatus,
    pub parent_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub status: TaskStatus,
    pub team_id: Option<String>,
    pub parent_task_id: Option<String>,
}
```

- [x] **Step 4: Create in-memory runtime state for P0**

In `server/src/weft_code/state.rs`, define a small state container rather than pushing more state into `main.rs`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{ProjectInfo, SessionInfo, TaskInfo, TeamInfo, TeamTaskInfo, ApprovalRequestInfo};

#[derive(Default)]
pub struct WeftCodeState {
    pub projects: Vec<ProjectInfo>,
    pub sessions: Vec<SessionInfo>,
    pub tasks: Vec<TaskInfo>,
    pub teams: Vec<TeamInfo>,
    pub team_tasks: Vec<TeamTaskInfo>,
    pub approvals: Vec<ApprovalRequestInfo>,
}

pub type SharedWeftCodeState = Arc<RwLock<WeftCodeState>>;
```

- [x] **Step 5: Create a tiny service layer that owns resource semantics**

In `server/src/weft_code/service.rs`, add a minimal service API:

```rust
use super::state::SharedWeftCodeState;
use super::types::{ProjectInfo, SessionInfo};

#[derive(Clone)]
pub struct WeftCodeService {
    pub state: SharedWeftCodeState,
}

impl WeftCodeService {
    pub async fn list_projects(&self) -> Vec<ProjectInfo> {
        self.state.read().await.projects.clone()
    }

    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        self.state.read().await.sessions.clone()
    }
}
```

- [x] **Step 5.1: Split session, task, policy, and workflow entrypoints into dedicated modules**

Add minimal skeletons so runtime responsibilities stop growing inside one service file:

```rust
// session_service.rs
use super::state::SharedWeftCodeState;

#[derive(Clone)]
pub struct SessionService {
    pub state: SharedWeftCodeState,
}
```

```rust
// task_service.rs
use super::state::SharedWeftCodeState;

#[derive(Clone)]
pub struct TaskService {
    pub state: SharedWeftCodeState,
}
```

```rust
// policy_service.rs
use super::state::SharedWeftCodeState;

#[derive(Clone)]
pub struct PolicyService {
    pub state: SharedWeftCodeState,
}
```

```rust
// workflow_gateway.rs
use super::state::SharedWeftCodeState;

#[derive(Clone)]
pub struct WorkflowGateway {
    pub state: SharedWeftCodeState,
}
```

- [x] **Step 6: Create route registration without yet implementing the full protocol**

In `server/src/weft_code/routes.rs` add an initial router factory:

```rust
use axum::{routing::get, Router};

use super::service::WeftCodeService;

pub fn router(service: WeftCodeService) -> Router {
    Router::new()
        .route("/api/weft-code/projects", get(list_projects))
        .route("/api/weft-code/sessions", get(list_sessions))
        .with_state(service)
}
```

- [x] **Step 7: Mount the WEFT-Code router in `server/src/main.rs`**

Wire the new router into the existing Axum application rather than adding more handlers inline.

- [x] **Step 8: Run formatting and compile check**

Run:

```bash
cargo fmt
cargo check -p server
```

Expected: both pass.

> 2026-04-15 execution note
>
> - Completed this round by extracting a compile-safe `server/src/weft_code/` skeleton and wiring it into `server/src/main.rs`.
> - For compile safety, the runtime module root also includes `events.rs`, because the plan's module list referenced it before Task 5 formally introduces the event surface.
> - Added a focused route smoke test for `/api/weft-code/projects` and `/api/weft-code/sessions` using the new stub runtime state.
> - `cargo fmt` and `cargo check -p registry-server` both passed; the targeted route smoke test also passed.
> - Remaining warnings are expected skeleton-stage dead-code warnings, not compile errors.
> - Next: Task 4 is ready for review; do not start Task 5 until Task 4 is accepted.

Task 4 round result

- Files: `server/src/main.rs`, `server/src/weft_code/mod.rs`, `server/src/weft_code/routes.rs`, `server/src/weft_code/types.rs`, `server/src/weft_code/state.rs`, `server/src/weft_code/service.rs`, `server/src/weft_code/session_service.rs`, `server/src/weft_code/task_service.rs`, `server/src/weft_code/policy_service.rs`, `server/src/weft_code/workflow_gateway.rs`, `server/src/weft_code/events.rs`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified:
  - `cargo fmt` passed.
  - `cargo check -p registry-server` passed.
  - `cargo test -p registry-server weft_code_routes_return_stub_runtime_resources` passed.
  - `lsp_diagnostics` on `server/src` reported zero errors.
- Notes:
  - Task 4 stayed within runtime module skeleton extraction only.
  - No Task 5 SSE endpoint or stream behavior was implemented beyond adding the placeholder `events.rs` module required to keep the module root coherent.
- Next: request Oracle review for Task 4 before any completion claim.

---

### Task 5: Add the first event-stream surface

**Files:**
- Create: `server/src/weft_code/events.rs`
- Modify: `server/src/weft_code/routes.rs`
- Modify: `server/src/main.rs`
- Modify: `server/Cargo.toml`

- [x] **Step 1: Define a simple event enum**

In `events.rs`, define a transport-safe event model:

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeEvent {
    SessionCreated { session_id: String },
    TaskCreated { task_id: String, session_id: String },
    ModeChanged { session_id: String, mode: String },
    ApprovalRequested { approval_id: String, session_id: String },
    TeamCreated { team_id: String, session_id: String },
}
```

- [x] **Step 2: Add an SSE endpoint stub to the routes module**

The route should exist even if the first implementation emits only bootstrap events.

Add a route such as:

```rust
.route("/api/weft-code/events", get(stream_events))
```

- [x] **Step 3: Return a valid SSE response from the stub**

Implement `stream_events` with a simple stream that emits at least one event and then stays open or completes cleanly, using Axum’s SSE support already present in the server stack if available.

- [x] **Step 4: Verify the server still compiles after adding SSE**

Run:

```bash
cargo check -p server
```

Expected: pass with no unused import explosion left unresolved.

> 2026-04-15 execution note
>
> - Completed this round by turning the placeholder `events.rs` into the first transport-safe runtime event enum and by exposing `/api/weft-code/events` from the Task 4 route module.
> - The first SSE implementation intentionally emits one bootstrap `runtime_event` and completes cleanly; it does not yet attempt long-lived streaming semantics beyond the stub surface required by Task 5.
> - Added a focused route smoke test that verifies the SSE endpoint returns `text/event-stream` and emits a serialized `session_created` event.
> - `cargo check -p registry-server` remained green after adding the SSE endpoint stub.
> - Next: Task 5 is ready for review; do not start Task 6 until Task 5 is accepted.

Task 5 round result

- Files: `server/src/main.rs`, `server/src/weft_code/events.rs`, `server/src/weft_code/routes.rs`, `server/Cargo.toml`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`, `.sisyphus/ralph-loop.local.md`
- Verified:
  - `cargo test -p registry-server weft_code_events_route_returns_sse_response` passed.
  - `cargo check -p registry-server` passed after adding the SSE endpoint and `futures-util` dependency.
- Notes:
  - Task 5 stayed within the first event-stream surface only.
  - The SSE endpoint currently serves a bootstrap event stream stub and does not yet implement the broader lifecycle/event model planned for later tasks.
- Next: request Oracle review for Task 5 before any completion claim.

---

## Phase 2: Workflow and Team Semantics

### Task 6: Encode `coding / plan / team` at the transport layer first

**Files:**
- Modify: `server/src/weft_code/types.rs`
- Modify: `server/src/weft_code/service.rs`
- Modify: `server/src/weft_code/routes.rs`
- Reference: `docs/superpowers/specs/2026-04-15-weft-code-design.md`

- [x] **Step 1: Replace stringly-typed mode with an enum**

In `types.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    Coding,
    Plan,
    Team,
}
```

- [x] **Step 2: Update `SessionInfo` to use the enum**

Change the field from `String` to `SessionMode`.

- [x] **Step 3: Add a route for explicit mode transitions**

Expose a route such as:

```rust
POST /api/weft-code/session/:session_id/mode
```

with a request body carrying the target mode.

- [x] **Step 4: Add the minimal service method for mode transition**

The service method should validate that only the three supported modes are allowed and update the target session in shared state.

- [x] **Step 5: Add a compile-only integration test or route smoke test for mode switching**

Create a small test in the server crate for the mode route if test patterns already exist; otherwise add one route-level smoke test in the server module nearest the current style.

- [x] **Step 6: Run the focused test and compile check**

Run the smallest matching command for the new server test plus:

```bash
cargo test -p server
```

Expected: the new route test passes.

> 2026-04-15 execution note
>
> - Completed this round by adding the first explicit mode-transition transport route to the WEFT-Code runtime stub.
> - `SessionMode` already existed from Task 4 skeleton work, so this round executed Step 1/2 by confirming the enum-backed `SessionInfo` shape and then adding the missing request type, route, and service mutation path.
> - Added a focused route smoke test that posts `{"mode":"team"}` to `/api/weft-code/session/weft-code-local-session/mode` and verifies the updated session payload.
> - `cargo check -p registry-server` remained green after the mode route addition.
> - Next: Task 6 is ready for review; do not start Task 7 until Task 6 is accepted.

Task 6 round result

- Files: `server/src/main.rs`, `server/src/weft_code/types.rs`, `server/src/weft_code/service.rs`, `server/src/weft_code/routes.rs`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified:
  - `cargo test -p registry-server weft_code_mode_route_updates_session_mode` passed.
  - `cargo check -p registry-server` passed after adding the mode request type, route, and service mutation logic.
- Notes:
  - Task 6 stayed within mode transport only.
  - No Task 7 lineage, approval, or approval-queue resources were implemented.
- Next: request Oracle review for Task 6 before any completion claim.

---

### Task 7: Add session lineage, task state, and approval resources

**Files:**
- Modify: `server/src/weft_code/types.rs`
- Modify: `server/src/weft_code/state.rs`
- Modify: `server/src/weft_code/session_service.rs`
- Modify: `server/src/weft_code/task_service.rs`
- Modify: `server/src/weft_code/routes.rs`

- [x] **Step 1: Add transport types for lineage and approval**

Add starter approval/resource structs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequestInfo {
    pub id: String,
    pub session_id: String,
    pub action_kind: String,
    pub status: String,
}
```

- [x] **Step 2: Add route(s) to inspect session lineage and approval queue**

Expose first-pass endpoints such as:

```text
GET /api/weft-code/session/:session_id
GET /api/weft-code/session/:session_id/tasks
GET /api/weft-code/approvals
POST /api/weft-code/approval/:approval_id/decision
```

- [x] **Step 3: Implement minimal service logic for session parent linkage and task state changes**

The first implementation only needs to support:

- parent_session_id read/write
- task status updates through the allowed state enum
- approval records stored in runtime state

- [x] **Step 4: Add a small route smoke test that proves approval and task status serialize correctly**

Run:

```bash
cargo test -p server
```

Expected: the new route test passes and enum serialization is stable.

> 2026-04-15 execution note
>
> - Completed this round by exposing the first inspection surface for session lineage, task state, and approval resources.
> - Reused the existing `parent_session_id`, `TaskStatus`, and `ApprovalRequestInfo` transport model already present in the Task 4 skeleton, then added the missing request shape plus service/query routes needed to make those resources observable and mutable.
> - Added a focused smoke test that verifies session lookup, session-task listing, approval queue listing, and approval decision update serialization.
> - Closed the remaining Task 7 gap by adding explicit parent-session write support and task-status update support, then extending the focused smoke test to cover both write paths.
> - `cargo check -p registry-server` remained green after the new resource routes were added.
> - Next: Task 7 is ready for review; do not start Task 8 until Task 7 is accepted.

Task 7 round result

- Files: `server/src/main.rs`, `server/src/weft_code/types.rs`, `server/src/weft_code/state.rs`, `server/src/weft_code/session_service.rs`, `server/src/weft_code/task_service.rs`, `server/src/weft_code/routes.rs`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified:
  - `cargo test -p registry-server weft_code_session_lineage_and_task_status_routes_update_runtime_state` passed.
  - `cargo test -p registry-server weft_code_task_and_approval_routes_serialize_runtime_resources` passed.
  - `cargo check -p registry-server` passed after adding session/task/approval resource routes and service methods.
- Notes:
  - Task 7 stayed within lineage/task-state/approval resources only.
  - Parent-session write and task-status update paths are now explicitly present, so Task 7 no longer depends on read-only interpretation of those resources.
  - No Task 8 first-class team-resource routes or team runtime semantics were implemented.
- Next: request Oracle review for Task 7 before any completion claim.

---

### Task 8: Add first-class team resources without full orchestration yet

**Files:**
- Modify: `server/src/weft_code/types.rs`
- Modify: `server/src/weft_code/state.rs`
- Modify: `server/src/weft_code/service.rs`
- Modify: `server/src/weft_code/routes.rs`

- [x] **Step 1: Define team transport structs**

Add starter types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInfo {
    pub id: String,
    pub session_id: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTaskInfo {
    pub id: String,
    pub team_id: String,
    pub role: String,
    pub phase: String,
    pub status: String,
}
```

- [x] **Step 2: Extend runtime state with team collections**

Add `teams` and `team_tasks` collections to `WeftCodeState`.

- [x] **Step 3: Add routes for listing and creating team resources**

Expose first-pass endpoints such as:

```text
GET  /api/weft-code/teams
POST /api/weft-code/session/:session_id/team
GET  /api/weft-code/team/:team_id/tasks
```

- [x] **Step 4: Implement only minimal semantics**

For P0/P1, team creation should:

- require a source session
- set that session’s mode to `team`
- create an initial `TeamInfo`
- create zero or one starter team task

- [x] **Step 5: Emit a team event on team creation**

Add an event variant such as:

```rust
TeamCreated { team_id: String, session_id: String }
```

- [x] **Step 6: Verify compile and route tests again**

Run:

```bash
cargo test -p server
```

Expected: pass.

> 2026-04-15 execution note
>
> - Completed this round by adding the first first-class team resources as transport-visible runtime objects.
> - `TeamInfo` and `TeamTaskInfo` were aligned to the Task 8 shape, runtime state now keeps `teams` and `team_tasks`, and the first list/create/list routes are exposed.
> - The minimal semantics implemented for P0/P1 are: require a source session, switch that session to `team` mode, create a starter `TeamInfo`, create one starter `TeamTaskInfo`, and return a `TeamCreated` event on team creation.
> - `cargo check -p registry-server` remained green and the focused team route smoke test passed after the `TeamCreated` event response was added.
> - Next: Task 8 is ready for review; do not start Task 9 until Task 8 is accepted.

Task 8 round result

- Files: `server/src/main.rs`, `server/src/weft_code/types.rs`, `server/src/weft_code/service.rs`, `server/src/weft_code/routes.rs`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified:
  - `cargo test -p registry-server weft_code_team_routes_create_and_list_team_resources` passed.
  - `cargo check -p registry-server` passed after adding team routes, minimal team semantics, and `TeamCreated` event emission.
- Notes:
  - Task 8 stayed within first-class team resources only.
  - No Task 9 runtime-contract or client-surface documentation was implemented.
- Next: request Oracle review for Task 8 before any completion claim.

---

## Phase 3: Client-Facing Product Surface

### Task 9: Define the first CLI/TUI integration boundary in docs and config

**Files:**
- Modify: `docs/architecture/vNext/14-weft-code-runtime-model.md`
- Modify: `docs/superpowers/specs/2026-04-15-weft-code-design.md`
- Create or modify: project-local runtime config doc if needed under `docs/architecture/vNext/`

- [x] **Step 1: Document the first concrete runtime contract**

In the runtime doc, add a section titled:

```md
## 初始 runtime 对外接口
```

List the first supported resources and routes:

- projects
- sessions
- session mode transition
- teams
- events

- [x] **Step 2: Document what remains intentionally out of scope for P0**

Add a subsection that explicitly says the following are deferred:

- full CLI implementation
- full TUI implementation
- persistent task replay
- GUI client
- remote runtime hosting

- [x] **Step 3: Record that clients should consume runtime, not core packages directly**

Add a statement like:

```md
CLI / TUI / GUI 必须通过 WEFT-Code runtime 连接系统，不直接调用 core packages。
```

> 2026-04-15 execution note
>
> - Completed this round by writing the first explicit client-facing runtime contract into the existing runtime and design docs.
> - `docs/architecture/vNext/14-weft-code-runtime-model.md` now contains `## 初始 runtime 对外接口`, lists the first supported resources/routes, and explicitly records the P0 deferrals.
> - `docs/superpowers/specs/2026-04-15-weft-code-design.md` now states that CLI / TUI / GUI must consume the WEFT-Code runtime instead of calling core packages directly, and records the same P0 runtime-resource boundary.
> - Next: Task 9 is ready for review; do not start Task 10 until Task 9 is accepted.

Task 9 round result

- Files: `docs/architecture/vNext/14-weft-code-runtime-model.md`, `docs/superpowers/specs/2026-04-15-weft-code-design.md`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified:
  - `14-weft-code-runtime-model.md` now contains `## 初始 runtime 对外接口`, the first supported runtime resources/routes, the explicit P0 defer list, and the client-through-runtime statement.
  - `2026-04-15-weft-code-design.md` now records the same client boundary and P0 runtime-resource scope.
- Notes:
  - Task 9 stayed within runtime/client boundary documentation only.
  - No Task 10 client-surface note was created in this round.
- Next: request Oracle review for Task 9 before any completion claim.

---

### Task 10: Prepare the repository for later CLI/TUI clients without implementing them yet

**Files:**
- Create: `docs/architecture/vNext/15-weft-code-client-surface.md`
- Reference: `reference/opencode-dev/README.md`
- Reference: `reference/codex-main/docs/tui-chat-composer.md`
- Reference: `reference/kuberwastaken-claude-code/bootstrap/state.ts`

- [x] **Step 1: Create a short client-surface note**

Start the file with:

```md
# WEFT-Code 蓝图：Client Surface
```

- [x] **Step 2: Define the future client split in prose, not code**

Document:

- CLI for command-oriented and automation-friendly control
- TUI as default interactive shell
- GUI as future reuse target of the same runtime

- [x] **Step 3: Add one explicit non-goal statement**

Include this point:

```md
P0 不实现完整 CLI/TUI，只先固定 runtime 与资源模型。
```

> 2026-04-15 execution note
>
> - Completed this round by adding a dedicated client-surface note under `docs/architecture/vNext/15-weft-code-client-surface.md`.
> - The note keeps the split prose-only: CLI as command-oriented control, TUI as the default interactive shell, and GUI as a future reuse target of the same runtime.
> - The non-goal statement is explicit: P0 does not implement a full CLI/TUI and only fixes runtime/resources first.
> - Next: Task 10 is ready for review.

Task 10 round result

- Files: `docs/architecture/vNext/15-weft-code-client-surface.md`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified:
  - `15-weft-code-client-surface.md` starts with `# WEFT-Code 蓝图：Client Surface`.
  - The note defines the CLI / TUI / GUI split in prose, not code.
  - The note explicitly includes `P0 不实现完整 CLI/TUI，只先固定 runtime 与资源模型。`
- Notes:
  - Task 10 stayed within client-surface documentation only.
  - No CLI/TUI implementation or runtime/server changes were introduced.
- Next: request Oracle review for Task 10 before any completion claim.

2026-04-18 natural-language-runtime checkpoint note

- This continuation prioritized the package-first closure ahead of further CLI/TUI work.
- `server/src/weft_code/types.rs` now defines `NaturalLanguageTaskRequest` and `NaturalLanguageTaskResponse` so WEFT-Code can accept a natural language request through runtime.
- `server/src/weft_code/workflow_gateway.rs` now interprets a non-empty prompt into a staged runtime task placeholder tied to the session.
- `server/src/weft_code/routes.rs` now exposes `POST /api/weft-code/session/{session_id}/natural-language-task`.
- `server/src/main.rs` now contains an integration-style route test proving a natural language request creates a runtime task result and becomes visible in the session task list.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route_creates_runtime_task_result` passed, `cargo check -p registry-server` passed, and `lsp_diagnostics` on `server/src/weft_code/workflow_gateway.rs` returned no diagnostics.
- Scope limit: this checkpoint does not yet execute real tools, modify repository files, or run full workflow orchestration from the prompt. It only closes the first runtime-backed natural-language request loop.

2026-04-18 natural-language-runtime hardening note

- This continuation hardened the same runtime-backed natural-language loop without expanding into CLI/TUI work.
- `server/src/weft_code/workflow_gateway.rs` now distinguishes empty prompt vs missing session and generates unique task ids per submission instead of reusing a fixed `-nl-task` suffix.
- `server/src/main.rs` now covers three additional route behaviors: empty prompt returns `400`, unknown session returns `404`, and repeated submissions produce distinct task ids.
- Manual runtime QA passed by launching `registry-server` and calling `POST /api/weft-code/session/weft-code-local-session/natural-language-task` with a natural language prompt; the endpoint returned a structured task result with `task.id = weft-code-local-session-nl-task-1`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 errors, and the manual HTTP call returned the expected JSON payload.
- Scope limit: WEFT-Code still records and returns a runtime task placeholder only. It does not yet execute real tools or complete a genuine repository change from the prompt.

2026-04-18 natural-language-runtime action-mapping note

- This continuation extended the same package-first runtime endpoint from placeholder-only recording into real runtime state changes.
- `server/src/weft_code/workflow_gateway.rs` now maps prompts containing `plan` into session mode transition `coding -> plan` and returns a staged planning task.
- The same gateway now maps prompts containing `team` into session mode transition `coding/plan -> team`, creates a real `TeamInfo` plus queued `TeamTaskInfo`, and returns a staged team kickoff task with `created_team` in the response.
- `server/src/main.rs` now includes route tests proving plan-mode transition and team runtime creation through the natural-language endpoint.
- Manual runtime QA passed by starting `registry-server` and POSTing two prompts: one planning prompt returned `session.mode = plan`, and one team prompt returned `session.mode = team` plus a created team object with `id = weft-code-local-session-team-1`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 errors, and manual HTTP calls returned the expected plan/team JSON payloads.
- Scope limit: this is a real runtime action loop for mode/task/team state, but it still does not run full workflow orchestration, tool execution, or repository mutation from the prompt.

2026-04-18 natural-language-runtime formal-task note

- This continuation tightened the runtime loop so natural-language requests now map to more formal task semantics from the WEFT-Code blueprint.
- Generic coding prompts now return `task.kind = coding_task` and enqueue one related `verification_task` in `related_tasks`.
- Plan prompts now return `task.kind = plan_task` instead of the earlier ad-hoc `plan_request` label.
- Team prompts now return `task.kind = workflow_task`, include `created_team`, and enqueue one related `teammate_task` in `related_tasks`.
- `server/src/main.rs` route tests were updated to verify the new formal task kinds and the presence of related queued tasks for coding/team flows.
- Manual runtime QA passed after restarting `registry-server`: coding prompt returned `coding_task` plus `verification_task`; plan prompt returned `plan_task`; team prompt returned `workflow_task` plus `teammate_task` and `created_team.id = weft-code-local-session-team-1`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 errors (31 rust-analyzer hints only), and clean manual HTTP calls returned the expected coding/plan/team JSON payloads.
- Scope limit: task semantics are now closer to the blueprint, but execution remains runtime-state-only. No real tool execution, repository mutation, or full analyze→plan→execute→verify orchestration is wired yet.

2026-04-18 natural-language-runtime task-status note

- This continuation tightened lifecycle honesty without expanding scope.
- `server/src/weft_code/workflow_gateway.rs` now returns the primary `coding_task`, `plan_task`, and `workflow_task` with `status = queued` instead of `completed`.
- Related `verification_task` and `teammate_task` remain `queued`, so the runtime now consistently expresses "accepted and staged" rather than implying execution already happened.
- This note supersedes the earlier "completed runtime task placeholder" wording from the first natural-language checkpoint.
- `server/src/main.rs` route tests were updated to assert queued status for coding/plan/team primary tasks.
- Manual runtime QA passed after a clean restart of `registry-server`: coding, plan, and team prompts all returned primary tasks with `status = queued` while preserving the previously verified mode/team/task semantics.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 errors (31 rust-analyzer hints only), and clean manual HTTP calls returned the expected queued task payloads.
- Scope limit: this improves lifecycle semantics only. WEFT-Code still does not execute real tools, mutate repository files, or run full workflow orchestration from the prompt.

2026-04-19 natural-language-runtime repo-check note

- This continuation turns the generic coding path from staged-only semantics into a minimal real execution loop.
- `server/src/weft_code/workflow_gateway.rs` now runs the read-only command `cargo check -p registry-server` for `coding_task` requests and returns the command, exit code, stdout, and stderr in a new `execution` field.
- When the repository check succeeds, the primary `coding_task` and the related `verification_task` both become `completed`; if the check fails they become `failed`.
- `server/src/main.rs` route test now verifies `execution.command = cargo check -p registry-server`, `execution.exit_code = 0`, and completed statuses for the coding path.
- Manual runtime QA passed after rebuilding and cleanly restarting `registry-server`: a coding prompt returned `task.kind = coding_task`, `task.status = completed`, `related_tasks[0].kind = verification_task`, `related_tasks[0].status = completed`, and `execution.exit_code = 0` with the compiler warnings captured in `execution.stderr`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA returned the expected execution payload.
- Scope limit: this is still a safe read-only execution slice. WEFT-Code can now execute one repository check from natural language, but it still does not modify files or run a full autonomous coding workflow.

2026-04-19 natural-language-runtime lock-safety note

- This continuation keeps the same read-only repository check behavior but removes the long-running command from the `SharedWeftCodeState` write-lock window.
- `server/src/weft_code/workflow_gateway.rs` now stages the `coding_task` and `verification_task` as queued, drops the write lock, runs `cargo check -p registry-server`, then reacquires the lock only to persist final task statuses.
- This keeps runtime semantics unchanged for users while avoiding a write lock being held across repository check execution.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code/workflow_gateway.rs` returned no diagnostics, and clean manual HTTP QA still returned `coding_task.status = completed`, `verification_task.status = completed`, and `execution.exit_code = 0`.
- Scope limit: this is an internal runtime safety improvement only. It does not add new natural-language behaviors or expand beyond the existing read-only repo check slice.

2026-04-19 natural-language-runtime multi-step note

- This continuation preserves the same safe read-only coding path but upgrades it from a single command into a tiny workflow-first execution slice.
- `server/src/weft_code/workflow_gateway.rs` now returns `workflow_steps`, where the coding path first emits `analyze:repo-state` and then runs `cargo check -p registry-server` as the verify step.
- The final `execution` field still points at the last verification step so downstream clients can keep reading one canonical execution result, while `workflow_steps` exposes the full two-step path.
- `server/src/main.rs` route test now verifies both workflow steps, including `workflow_steps[0].command = analyze:repo-state` and `workflow_steps[1].command = cargo check -p registry-server`.
- Manual runtime QA passed after clean restart: a coding prompt returned `workflow_steps` with two successful entries, `coding_task.status = completed`, `verification_task.status = completed`, and `result` reporting that 2 read-only workflow checks were run.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 errors (rust-analyzer hints only), and clean manual HTTP QA returned the expected two-step payload.
- Scope limit: this is still a read-only execution slice. WEFT-Code now expresses a tiny analyze→verify chain for coding prompts, but it still does not perform code mutation or a full autonomous coding workflow.

2026-04-19 natural-language-runtime approval-gate note

- This continuation makes the runtime safety/permission story observable instead of implicit.
- `server/src/weft_code/workflow_gateway.rs` now generates or reuses an approval request for `coding_task` prompts. When approval is still pending, the endpoint returns `task.status = waiting_approval`, includes the `approval` object, and does not execute workflow steps.
- After the approval is updated to `approved` through `/api/weft-code/approval/{approval_id}/decision`, re-issuing the same coding prompt executes the existing two-step read-only workflow and returns completed coding/verification tasks.
- `server/src/main.rs` now covers both sides of the gate: pending approval blocks execution, and approved coding requests run successfully afterward.
- Manual runtime QA passed after clean restart: first coding request returned `waiting_approval` with `approval.status = pending`; approving `weft-code-local-session-approval-coding_task` and reissuing the prompt returned `coding_task.status = completed`, `workflow_steps` with two successful entries, and `execution.exit_code = 0`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA matched the blocked-then-approved flow.
- Scope limit: this is still not full policy orchestration or file mutation. It is a minimal observable approval gate protecting the existing read-only coding workflow.

2026-04-19 natural-language-runtime write-note note

- This continuation adds the first safe write-capable path after approval.
- When an approved coding prompt includes write/file/note language, `workflow_gateway.rs` now creates `.weft/weft-code/runtime-note.txt` under the repo root, reports the write in `file_mutation`, and includes the note path plus bytes written.
- The existing approval gate remains intact: a prompt still waits for approval first; the approved reissue then runs the workflow and, for write-oriented prompts, emits both `execution` and `file_mutation`.
- `server/src/main.rs` now verifies the file-mutation path by checking the note file exists and contains the prompt text after approval.
- Manual runtime QA passed after clean restart: pending coding prompt returned `waiting_approval`, approval was granted through `/api/weft-code/approval/{approval_id}/decision`, and the approved write-oriented prompt returned `file_mutation.path = .weft/weft-code/runtime-note.txt` with the note file written on disk.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA produced the expected approval-gated write result.
- Scope limit: this is a minimal, controlled file write. It is not a general-purpose code mutation engine and does not yet perform patch synthesis or full autonomous coding workflow execution.

2026-04-19 natural-language-runtime write-fail-closed note

- Oracle review identified that the initial write-note path swallowed file write failures while still completing the task if the repo check passed.
- `server/src/weft_code/workflow_gateway.rs` now treats runtime note write failure as task failure: `coding_task` and `verification_task` become `failed`, `file_mutation` is omitted, and the result states that the file write did not succeed.
- The write path now creates the `.weft/weft-code` parent directory under the repo root before writing `.weft/weft-code/runtime-note.txt`.
- `server/src/main.rs` now covers both successful note writing and the fail-closed case where the note path is blocked by a file at `.weft/weft-code`.
- Manual runtime QA passed after clean restart: pending approval blocked execution, approved write prompt produced `file_mutation.path = .weft/weft-code/runtime-note.txt`, and the note file contained the prompt text.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 10 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the approved write path.
- Scope limit: this closes the first approval-gated file mutation loop for a fixed runtime note path only. It still does not support arbitrary patch synthesis or general code editing.

2026-04-19 natural-language-runtime unique-note-path note

- This continuation removes the last test-isolation hazard by making the runtime note filename unique per task id.
- `workflow_gateway.rs` now derives the note path from the request prompt plus the task id, so two coding requests cannot collide on the same file while the approval gate remains intact.
- `server/src/main.rs` now asserts the note file path includes the task id for the approved write path and still verifies that the fail-closed test cannot write when the path is blocked.
- Manual runtime QA passed after clean restart: the approved write request returned `file_mutation.path = .weft/weft-code/runtime-note-weft-code-local-session-nl-task-3.txt` and the file existed on disk with the prompt content.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 10 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA returned the unique note path with actual file content.
- Scope limit: this is still a single controlled note write path. It does not broaden into generic file editing or patch synthesis.

2026-04-19 natural-language-runtime controlled-patch note

- This continuation extends the approval-gated write path from note files into a single-file patch-style target while keeping the scope constrained.
- `workflow_gateway.rs` now recognizes prompts containing `patch` and writes to `.weft/weft-code/patch-target-<session-id>.txt`, appending the patch text to a single file rather than applying a general repository-wide diff.
- The patch path still goes through the approval gate and preserves fail-closed behavior: if the patch file cannot be written, the task fails and no mutation is reported.
- `server/src/main.rs` now covers the approved patch path end-to-end and confirms the file exists with the patch text on disk.
- Manual runtime QA passed after clean restart: a patch-oriented prompt returned `file_mutation.action = patch`, `file_mutation.path = .weft/weft-code/patch-target-weft-code-local-session.txt`, and the patch file contained the prompt text.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 10 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the approved patch write path.
- Scope limit: this is still a single-file controlled write target. It is not yet a general patch application engine over arbitrary source files.

2026-04-19 natural-language-runtime source-patch note

- This continuation narrows the patch path wording further so future readers do not overread it as general editing.
- The runtime now writes a single controlled patch target file that can be treated as a source-adjacent artifact, but it is still not a general arbitrary-source patch engine.
- `server/src/main.rs` verifies the approved patch flow by checking the target file exists and contains the patch text; the fail-closed test still ensures blocked writes do not create the target.
- Manual runtime QA passed after clean restart: the approved patch request returned `file_mutation.path = .weft/weft-code/patch-target-weft-code-local-session.txt` and the file content appended the patch payload.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 11 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the controlled patch target was written.
- Scope limit: this is still an artifact-level patch target and not a general source patch engine. The doc now makes that limitation explicit.

2026-04-19 natural-language-runtime fixed-source-patch note

- This continuation promotes the default patch target from an artifact file to one fixed Rust source file while preserving the same approval gate and fail-closed semantics.
- `server/src/weft_code/patch_target.rs` is now a real module included from `server/src/weft_code/mod.rs`, and the default `controlled_patch_target` is `server/src/weft_code/patch_target.rs`.
- Patch prompts append Rust comment-form patch payloads to that fixed source file, so `cargo check -p registry-server` and rust-analyzer continue to accept the result.
- `server/src/main.rs` now backs up and restores `patch_target.rs` in the patch test while verifying `file_mutation.path = server/src/weft_code/patch_target.rs` and actual source-file content.
- Manual runtime QA passed after clean restart: an approved patch request returned `file_mutation.path = server/src/weft_code/patch_target.rs`, appended a `// PATCH:` block to the source file, and the test cleanup restored the file.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 11 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the fixed source target write.
- Scope limit: this is still one fixed source file, not arbitrary user-selected paths, multi-file editing, or general diff application.

2026-04-19 natural-language-runtime multiline-source-patch note

- Oracle review found one remaining blocker in the fixed-source patch slice: multi-line prompts were not guaranteed to stay valid Rust comments.
- `workflow_gateway.rs` now comments every line of a Rust patch prompt individually, so the appended patch payload stays valid Rust comment syntax even when the prompt spans multiple lines.
- `server/src/main.rs` now includes a multiline patch prompt test that verifies both lines are prefixed as comments in `server/src/weft_code/patch_target.rs`.
- Manual runtime QA passed after clean restart: an approved multiline patch request appended two `//` comment lines to `server/src/weft_code/patch_target.rs`, and the file was restored after verification.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 12 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed comment-safe multiline patch content.
- Scope limit: this completes the fixed-source-file comment patch slice only. It still does not support arbitrary structured code edits or general diff application.

2026-04-19 natural-language-runtime allowlist-source-patch note

- This continuation adds an explicit allowlist guard around the fixed source patch path.
- `state.rs` now carries `allowed_patch_targets`, and the default runtime state seeds it with `server/src/weft_code/patch_target.rs` only.
- `workflow_gateway.rs` now rejects patch attempts whose configured target is outside the allowlist, causing the coding task to fail closed with no `file_mutation`.
- `server/src/main.rs` now verifies both the happy path for the allowlisted source target and the rejection path when the target is configured outside the allowlist.
- Manual runtime QA passed after clean restart: an approved patch request still wrote `server/src/weft_code/patch_target.rs`, while the guardrail continues to prevent arbitrary path selection in the runtime configuration.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 13 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the allowlisted fixed-source patch path.
- Scope limit: this is still a single-file allowlist path, not user-selected files, multi-file editing, or general patch application.

2026-04-19 natural-language-runtime multi-target-allowlist note

- This continuation extends the fixed-source allowlist from one file to a small, explicit set of two Rust source targets.
- `state.rs` now seeds `allowed_patch_targets` with `server/src/weft_code/patch_target.rs` and `server/src/weft_code/policy_service.rs`.
- `workflow_gateway.rs` now selects the second allowlisted target when the prompt mentions `policy`; otherwise it keeps using the default `patch_target.rs`.
- `server/src/main.rs` now verifies the second allowlisted happy path by patching `policy_service.rs`, asserting the file mutation path, and restoring the file content afterward.
- Manual runtime QA passed after clean restart: an approved prompt `Please patch the policy service file for this request.` returned `file_mutation.path = server/src/weft_code/policy_service.rs` and appended a `// PATCH:` block to that file.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 15 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the second allowlisted source target write.
- Scope limit: this is still a tiny explicit allowlist, not arbitrary path selection, multi-file editing, or general patch application.

2026-04-19 natural-language-runtime production-default-allowlist note

- This continuation aligns the test harness with the production default allowlist and proves that both shipped targets are reachable through the default runtime state.
- `server/src/main.rs` now makes the default test state mirror production defaults: `patch_target.rs` remains the default target, and `policy_service.rs` remains the second allowlisted target selected by `policy`-oriented prompts.
- The route suite now includes a default-state test proving generic patch prompts still target `patch_target.rs`, alongside the existing test proving `policy` prompts target `policy_service.rs`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 16 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the second shipped allowlist target write.
- Scope limit: this still remains a tiny explicit allowlist of two fixed source files, not user-selected paths, multi-file editing, or general patch application.

2026-04-19 natural-language-runtime explicit-target-id note

- This continuation removes the need to rely solely on prompt keywords for patch target routing.
- `NaturalLanguageTaskRequest` now supports an optional `target_id` field.
- `workflow_gateway.rs` now resolves `target_id = patch_target` to `server/src/weft_code/patch_target.rs` and `target_id = policy_service` to `server/src/weft_code/policy_service.rs`. Unknown `target_id` values fail closed through the existing allowlist path.
- Historical note: at this checkpoint `target_id` omission still fell back to the previous minimal heuristic; later checkpoints remove that fallback.
- `server/src/main.rs` now covers explicit target-id success for both shipped targets, invalid target-id failure, and the legacy fallback path.
- Manual runtime QA passed after clean restart: an approved request with `{"prompt":"Please patch this file for this request.","target_id":"policy_service"}` returned `file_mutation.path = server/src/weft_code/policy_service.rs` and appended the patch comment block to that file.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 18 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the explicit `target_id` flow.
- Scope limit: `target_id` does not open arbitrary path patching. It only selects among the tiny explicit allowlist of fixed source files.

2026-04-19 natural-language-runtime closed-target-registry note

- This continuation tightens patch target selection from basename-style matching into a small explicit registry.
- `workflow_gateway.rs` now resolves `target_id` through a closed registry: `patch_target -> server/src/weft_code/patch_target.rs`, `policy_service -> server/src/weft_code/policy_service.rs`.
- Historical note: at this checkpoint the fallback heuristic still existed for omitted `target_id`; later checkpoints remove that fallback. Explicit `target_id` no longer depends on `ends_with(...)` path matching.
- `server/src/main.rs` now covers explicit `target_id` success for both registry entries, invalid `target_id` failure, and fallback behavior with the same route suite.
- Manual runtime QA passed after clean restart: an approved request with `{"prompt":"Please patch this file for this request.","target_id":"policy_service"}` returned `file_mutation.path = server/src/weft_code/policy_service.rs` and appended the patch comment block to that file.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 19 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` reported 0 errors, and clean manual HTTP QA confirmed the explicit registry-based target selection.
- Scope limit: this is still a tiny closed registry over fixed source files, not arbitrary path patching, multi-file editing, or general patch application.

2026-04-19 natural-language-runtime mandatory-target-id note

- This continuation removes the last patch-target heuristic from the patch path itself.
- `workflow_gateway.rs` now requires explicit `target_id` for patch prompts. If a patch prompt arrives without `target_id`, the task fails closed rather than guessing a target from the prompt text.
- The closed registry remains the same: `patch_target` and `policy_service` resolve to the two fixed allowlisted source files, and invalid `target_id` values still fail closed.
- `server/src/main.rs` now updates the former fallback tests so they assert failure when patch prompts omit `target_id`, while explicit `target_id` tests continue to pass for both supported targets.
- Manual runtime QA passed after clean restart: an approved request with `{"prompt":"Please patch this file for this request.","target_id":"policy_service"}` still patched `policy_service.rs`, while patch prompts without `target_id` now stop at task failure after approval.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 19 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the mandatory `target_id` flow.
- Scope limit: this does not broaden patching. It simply makes target selection explicit and mandatory for patch requests, keeping the tiny fixed allowlist and fail-closed behavior.

2026-04-19 natural-language-runtime structured-patch-format note

- This continuation upgrades the source-file patch content from a single `// PATCH:` comment into a small structured patch block.
- Rust source targets now receive blocks shaped like:
  `// WEFT-PATCH-BEGIN`
  `// target: <path>`
  `// line_count: <n>`
  `// <prompt lines...>`
  `// WEFT-PATCH-END`
- `server/src/main.rs` now verifies the structured markers and target metadata in both the fixed source target and the explicit `policy_service` target path.
- Manual runtime QA passed after clean restart: an approved request with `{"prompt":"Please patch this file for this request.","target_id":"policy_service"}` appended a structured block to `policy_service.rs` with begin/end markers, target path, and line count.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 19 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the structured patch format.
- Scope limit: this is still a comment-only structured patch block, not semantic code transformation or diff application.

2026-04-19 natural-language-runtime patch-record-metadata note

- This continuation turns the structured comment block into a minimally auditable patch record.
- Rust source patch blocks now include `patch_id`, `session_id`, `task_id`, `target`, and `line_count` between `WEFT-PATCH-BEGIN` and `WEFT-PATCH-END`.
- The values are sourced from the live runtime task/session objects, so the comment block can be traced back to the exact task that produced it.
- `server/src/main.rs` now asserts the metadata fields for both the fixed source target and the explicit `policy_service` path.
- Manual runtime QA passed after clean restart: an approved request with `{"prompt":"Please patch this file for this request.","target_id":"policy_service"}` appended a block containing `patch_id: weft-code-local-session-nl-task-3`, `session_id: weft-code-local-session`, and `task_id: weft-code-local-session-nl-task-3`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 19 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the metadata-rich patch record.
- Scope limit: this is still a comment-only auditable record, not semantic code transformation, diff application, or replay execution.

2026-04-19 natural-language-runtime per-request-approval note

- Oracle review correctly identified that the previous approval design behaved like a session-level latch instead of a per-request-shape gate.
- `workflow_gateway.rs` now derives approval identity from a request key based on `prompt + target_id`, so every distinct coding request shape creates or reuses its own approval record instead of inheriting approval from prior unrelated requests in the same session.
- `server/src/main.rs` now captures approval ids from the pending response body and uses those ids in approval-decision calls, so the route suite no longer depends on hard-coded session-level approval ids.
- Manual runtime QA passed after clean restart: a pending patch request returned `approval.id = weft-code-local-session-approval-ec8c8bfad88b9b1b`, approving that exact id allowed only that request shape to run, and subsequent distinct request shapes still generated fresh pending approvals.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 20 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed request-shape approval behavior.
- Scope limit: approval is now keyed per request shape, but it is still a minimal runtime gate. This does not yet introduce richer approval policies, expiry, replay protection, or role-based approval semantics.

2026-04-20 natural-language-runtime approval-shape-note

- This continuation narrows the wording of the approval checkpoint to match the real implementation precisely.
- Approval is currently keyed by request shape (`prompt + target_id`) rather than by one-shot submission nonce, so identical resubmissions reuse the same approval record.
- `server/src/main.rs` now includes an explicit regression test showing that the same prompt + `target_id` pair reuses the same pending approval id, while a distinct request shape still receives a fresh pending approval id.
- The client-surface doc now describes this accurately as request-shape approval rather than implying a fresh approval object for every submission.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route -- --nocapture` passed with 21 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed request-shape approval ids flowing through a full approved patch request.
- Scope limit: this is still not nonce-based approval, replay protection, expiry, or multi-approver workflow. It is a minimal request-shape gate only.

2026-04-20 natural-language-runtime patch-record-response note

- This continuation makes patch-record metadata available directly in the API response instead of requiring clients to parse source-file comment blocks.
- `NaturalLanguageTaskResponse` now includes an optional `patch_record` object for successful patch writes.
- `workflow_gateway.rs` now fills `patch_record` with `patch_id`, `session_id`, `task_id`, `target_id`, `target_path`, and `line_count` when a patch mutation succeeds.
- `server/src/main.rs` now asserts the returned `patch_record` fields for both the default patch target and the explicit `policy_service` target.
- The route test harness now serializes file-mutating `patch_target.rs` tests with an async `tokio::sync::Mutex`, which stabilized the `--nocapture` route suite for the current workspace.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route -- --nocapture` passed with 22 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the approved patch response included the expected `patch_record` metadata.
- Scope limit: this exposes metadata for existing structured comment patches only. It does not introduce semantic patch application, AST transforms, or multi-file patch orchestration.

2026-04-20 natural-language-runtime approval-policy note

- This continuation introduces a minimal approval policy model into the runtime surface.
- `ApprovalPolicy` now supports `always_allow`, `on_request`, `on_sensitive_actions`, and `read_only_mode`, and the current runtime state carries one active policy value.
- `routes.rs` now exposes `/api/weft-code/policy` for reading/updating the current policy.
- `workflow_gateway.rs` now branches the coding path by policy: `always_allow` executes immediately, `on_request` preserves the approval gate, `on_sensitive_actions` now prefers explicit `action_kind` when provided and otherwise falls back to the current mutation-like heuristic, and `read_only_mode` blocks patch execution while allowing non-patch coding requests to run a read-only analyze step.
- `server/src/main.rs` now covers policy read/update, immediate execution in `always_allow`, selective gating in `on_sensitive_actions`, explicit `action_kind` sensitivity, and patch blocking in `read_only_mode`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 27 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed `on_sensitive_actions` respects explicit `action_kind = write_note` by routing the request into approval even when the prompt text is non-sensitive.
- Scope limit: this is still a minimal runtime policy slice only. It does not yet implement a full action taxonomy across team/plan flows, per-tool policy granularity, persistent project/session policy layering, or the broader `on_sensitive_actions` categories named in the product blueprint beyond current coding actions.

2026-04-20 natural-language-runtime action-kind note

- This continuation introduces the first explicit coding action taxonomy into the natural-language request path.
- `NaturalLanguageTaskRequest` and `NaturalLanguageTaskResponse` now support `action_kind`, currently with `analyze`, `patch`, and `write_note`.
- `workflow_gateway.rs` now classifies coding requests into a concrete `CodingActionKind` and uses that action kind for `on_sensitive_actions` policy gating instead of relying only on prompt keywords.
- Approval metadata for coding requests now aligns with the coding action model, so pending approval records expose the same `action_kind` used by coding-path policy gating.
- If a caller provides `action_kind`, the runtime uses it directly; otherwise it falls back to the current minimal classifier for backward compatibility.
- `server/src/main.rs` now includes a route test and manual HTTP QA proving that a non-sensitive prompt with `action_kind = write_note` still requires approval under `on_sensitive_actions`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 27 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed explicit `action_kind` sensitivity.
- Scope limit: this is only a coding-path action taxonomy. Team/plan flows still use the older task-kind split, and the runtime does not yet expose a broader tool/action ontology.

2026-04-20 natural-language-runtime plan-action-kind note

- This continuation extends the current action model one step beyond the coding path by making plan requests return explicit `action_kind = plan`.
- `workflow_gateway.rs` now surfaces `action_kind: plan` on the existing natural-language plan branch while keeping plan behavior otherwise unchanged (`task.kind = plan_task`, `status = queued`).
- `server/src/main.rs` now asserts `action_kind = plan` in the plan-route test.
- Manual HTTP QA passed after clean restart: `Please make a plan for this repository.` returned `task.kind = plan_task`, `task.status = queued`, and `action_kind = plan`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 29 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the plan action kind response.
- Scope limit: this is still not a full unified action ontology. Team flows remain outside the action-kind model, and no approval/policy behavior was added to plan requests in this slice.

2026-04-20 natural-language-runtime team-action-kind note

- This continuation extends the same minimal action-model exposure into the team path.
- `workflow_gateway.rs` now surfaces `action_kind = team_kickoff` on the existing natural-language team branch while keeping team behavior otherwise unchanged (`task.kind = workflow_task`, teammate task creation, and no new approval/policy semantics).
- `server/src/main.rs` now asserts `action_kind = team_kickoff` in the existing team-route test.
- Manual HTTP QA passed after clean restart: `Create a team to review this change.` returned `task.kind = workflow_task`, `status = queued`, `created_team`, and `action_kind = team_kickoff`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 29 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed the team action kind response.
- Scope limit: this still does not add team approval/policy behavior or a full unified action ontology. It only exposes action_kind on the team response path.

2026-04-20 natural-language-runtime team-on-request note

- This continuation gives the team path its first minimal runtime policy meaning under `on_request`.
- With `ApprovalPolicy::OnRequest`, the first team kickoff request now returns a `waiting_approval` `workflow_task` plus approval metadata (`action_kind = team_kickoff`) and does not create any team side effects yet.
- After approving that exact request and retrying it, the runtime preserves the existing team behavior: session switches to `team`, `created_team` is returned, and a queued `teammate_task` is created.
- `server/src/main.rs` now includes focused route tests for both the pending team-approval step and the approved retry path.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 33 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed the pending-then-approved team kickoff flow.
- Scope limit: this is only the minimal `on_request` gate for team kickoff. It does not yet add team behavior to `on_sensitive_actions`, `read_only_mode`, or a broader team policy model.

2026-04-20 natural-language-runtime team-read-only note

- This continuation gives the team path a minimal `read_only_mode` meaning.
- With `ApprovalPolicy::ReadOnlyMode`, team kickoff now returns a `blocked` `workflow_task` with `action_kind = team_kickoff` and does not switch the session to `team`, create `created_team`, or enqueue a `teammate_task`.
- This preserves the existing `always_allow` immediate team behavior and the existing `on_request` pending-then-approved team flow.
- `server/src/main.rs` now includes a focused route test for read-only team blocking.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 34 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed `read_only_mode` blocks team kickoff before side effects.
- Scope limit: this does not add `on_sensitive_actions` semantics for team and does not introduce a broader team policy model. It only defines read-only blocking for team kickoff.

2026-04-20 natural-language-runtime team-on-sensitive-actions note

- This continuation completes the minimal team policy matrix by treating `team_kickoff` as sensitive under `on_sensitive_actions`.
- With `ApprovalPolicy::OnSensitiveActions`, the first team kickoff request now returns a `waiting_approval` `workflow_task` plus approval metadata and no side effects.
- After approving that exact request and retrying it, the runtime preserves the existing team creation behavior: session switches to `team`, `created_team` is returned, and a queued `teammate_task` is created.
- `server/src/main.rs` now includes a focused route test for the pending-then-approved team flow under `on_sensitive_actions`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 35 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed the pending-then-approved team kickoff flow under `on_sensitive_actions`.
- Scope limit: this still does not introduce a broader team policy model beyond the current four policy modes, and it does not change the existing always_allow/on_request/read_only_mode team semantics.

2026-04-20 weft-code live-cli-shell note

- This continuation upgrades `clients/weft-code-tui` from a mock-only shell scaffold into the first live CLI shell for WEFT-Code.
- The crate now accepts a natural-language prompt, calls the live runtime HTTP API, and prints the structured response instead of rendering only `ShellSnapshot::sample()`.
- It also supports a minimal follow-up approval command: `weft-code-tui approve <APPROVAL_ID>`.
- Verification for this continuation: `cargo test -p weft-code-tui` passed, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a manual end-to-end run against a live `registry-server` confirmed the CLI could submit a patch request, show the pending approval response, and then approve the exact approval id.
- Scope limit: this is a minimal live CLI shell only. It is not yet a full interactive CLI/TUI product, and the approve command currently only approves the request rather than automatically reissuing it.

2026-04-20 weft-code live-cli-session-team note

- This continuation adds the first runtime inspection commands to the live CLI shell.
- `clients/weft-code-tui` now supports `sessions`, `teams`, and `team-tasks <TEAM_ID>` by calling the existing live runtime endpoints instead of rendering mock shell data.
- The existing prompt submission and `approve <APPROVAL_ID>` commands are preserved unchanged.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 6 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed the new CLI commands produced real runtime output for sessions, teams, and team tasks.
- Scope limit: this is still a minimal command-oriented shell, not a persistent interactive session browser or TUI dashboard.

2026-04-20 weft-code live-cli-mode note

- This continuation adds the first runtime control command to the live CLI shell.
- `clients/weft-code-tui` now supports `mode <coding|plan|team>` and calls the existing `/api/weft-code/session/{session_id}/mode` endpoint.
- The command supports the existing `--session` option and prints the updated live session returned by the runtime.
- Existing prompt submission, `approve`, `sessions`, `teams`, and `team-tasks` commands are preserved.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 7 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed `mode plan` changed the live session to `plan`, `sessions` showed `[plan/active]`, and `mode coding` switched it back to `[coding/active]`.
- Scope limit: this is still a minimal one-shot CLI command, not an interactive mode switcher, shell loop, or persistent session browser.

2026-04-20 weft-code live-cli-status note

- This continuation adds the first compact live status dashboard command to the CLI shell.
- `clients/weft-code-tui` now supports `status`, which calls existing runtime endpoints and aggregates policy, sessions, teams, approvals, and current-session tasks into one command-oriented view.
- The command does not add new server semantics and does not replace the existing dedicated `sessions`, `teams`, or `team-tasks` commands.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 8 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed `cargo run -p weft-code-tui -- status` printed live policy/session/team/approval/task state.
- Scope limit: this is still a one-shot command-oriented status view, not a persistent interactive dashboard, event stream consumer, or full TUI.

2026-04-20 weft-code live-tui-screen note

- This continuation adds the first live TUI path to `clients/weft-code-tui`.
- The new `tui` command uses the existing live runtime status aggregation and renders a panel-style single-screen `WEFT-CODE LIVE TUI` view with session, policy, teams, approvals, and tasks.
- The live TUI path does not use `ShellSnapshot::sample()`; the original mock renderer remains only as scaffold/test support.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 9 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed `cargo run -p weft-code-tui -- tui` rendered runtime-backed status data.
- Scope limit: this is still a one-shot live TUI screen, not an alternate-screen interactive TUI, key-handling loop, refresh loop, or event-stream dashboard.

2026-04-20 weft-code live-tui-watch note

- This continuation adds the smallest live refresh/watch capability to the TUI path.
- `clients/weft-code-tui` now supports `tui --watch`, plus finite `--frames` and `--interval-ms` options, and re-renders the live runtime status view for the requested number of frames.
- The watch mode still uses the existing status aggregation and panel-style renderer; it does not add alternate-screen behavior, keyboard input, or event-stream subscription.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 11 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed `cargo run -p weft-code-tui -- tui --watch --frames 2 --interval-ms 50` rendered two live frames.
- Scope limit: this is a finite refresh loop only. It is not yet a persistent interactive TUI or event-stream-driven dashboard.

2026-04-20 weft-code live-tui-interactive-loop note

- This continuation adds the first keyboard-driven interaction to the live TUI path.
- Non-watch `tui` now enters a minimal refresh/quit loop: it renders live runtime status, prints `Commands: [r] refresh  [q] quit`, refreshes on `r`/Enter, and exits on `q`.
- The implementation remains runtime-backed and uses the existing status client/render path; it does not add alternate-screen handling, event-stream subscription, or a full dashboard loop.
- A verification issue was fixed in this slice: the first implementation attempted to call `Handle::block_on` from inside the Tokio runtime, which panicked. The loop is now async and awaits `client.status(...)` directly.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 12 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed piping `q` into `cargo run -p weft-code-tui -- tui` rendered the live TUI screen and exited cleanly.
- Scope limit: this is a minimal stdin-command loop only. It is not an alternate-screen TUI, curses-style key handler, event-stream dashboard, or persistent multi-panel application.

2026-04-20 weft-code live-tui-multiview note

- This continuation adds the first multi-view interaction to the live TUI path.
- Non-watch `tui` now supports `1/2/3` to switch between session, teams, and tasks views while keeping the existing `r` refresh and `q` quit commands.
- The implementation remains runtime-backed by reusing the same status fetch and panel renderer; it does not add alternate-screen behavior, scrolling, or event-stream subscription.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 14 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed piping `2` then `q` into `cargo run -p weft-code-tui -- tui` switched from the session view to the teams view before exiting.
- Scope limit: this is still a tiny multi-view shell, not a full dashboard, navigator, or interactive TUI framework.

2026-04-20 weft-code live-tui-details note

- This continuation adds the first focus/details surface to the live TUI.
- The non-watch `tui` loop now maintains a selected index per view, supports `j`/`down` and `k`/`up`, and renders the selected row with `>`.
- Session, teams, and tasks views now include a small `Details` block for the selected item using only data already present in the live `StatusView`.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 15 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed switching to the tasks view rendered the selected task plus its Details block.
- Scope limit: this is only selection/focus display. It does not add activation/open actions, scrolling, alternate-screen rendering, or event-stream updates.

2026-04-20 weft-code live-cli-approve-and-run note

- This continuation shortens the approval loop into one explicit CLI action without hiding the original prompt requirement.
- `clients/weft-code-tui` now supports `approve-and-run <APPROVAL_ID> <prompt...>`, which first approves the given request and then immediately reissues the provided prompt through the live runtime.
- The command preserves `--session` and `--target-id` so it can be used for exact follow-up retries on the same request shape.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 17 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed the command approved `weft-code-local-session-approval-2cc984c6bc68af80` and immediately printed the resulting completed patch response.
- Scope limit: this does not infer prompts from approval objects or store prompt history. The user still provides the follow-up prompt explicitly.

2026-04-20 weft-code live-tui-focus-open note

- This continuation adds the first minimal open/drill-down action to the live TUI.
- In non-watch `tui`, `enter` / `open` now switches the current selected item into a focused detail screen, and `b` / `back` returns to the list view.
- The focused view reuses the same live `StatusView` data and does not add server calls, activation side effects, or navigation stacks.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 16 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed the sequence `3 -> enter -> b -> q` switched to tasks view, opened the focused task detail, returned to the task list, and exited cleanly.
- Scope limit: this is still a minimal focused detail view only. It does not introduce activation actions, edits, scrolling, alternate-screen rendering, or event-stream updates.

2026-04-20 weft-code live-tui-approval-action note

- This continuation adds the first minimal action from a focused TUI detail view.
- When the focused item is an approval in the session view, `a` / `approve` now approves that exact approval id through the live runtime and refreshes the TUI.
- If the user explicitly types `approve-and-run <prompt...>` while focused on that approval, the TUI also supports a minimal inline retry path that approves first and then reissues the user-provided prompt. No prompt history or hidden replay was added.
- Verification for this continuation: `cargo test -p weft-code-tui` passed with 18 tests, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and a live server/manual QA run confirmed the sequence `open -> approve -> q` approved a live pending approval and refreshed the focused approval detail to `status: approved`.
- Scope limit: this is still a minimal focused approval action. It does not add generic item activation, hidden prompt replay, alternate-screen interaction, or a broader TUI command system.

2026-04-22 weft-code full-screen-tui-rebuild plan note

- The current `clients/weft-code-tui` implementation is now good enough to prove a runtime-backed CLI shell and a minimal live TUI path, but it is still structurally a transitional text shell: the non-watch TUI path is driven by `stdin.read_line()` and string rendering rather than a full terminal UI framework.
- The next client slice is therefore not “add a few more commands to the existing loop”, but a deliberate rebuild of the TUI path onto `ratatui + crossterm` while keeping the runtime as the single source of truth.
- This rebuild should preserve the existing runtime-backed CLI commands and reuse the current HTTP client layer, but replace the current pseudo-TUI render/loop architecture with a full-screen event-driven application shell.

### Task 11: Rebuild `clients/weft-code-tui` as a full-screen runtime-backed TUI shell

**Files:**
- Modify: `clients/weft-code-tui/Cargo.toml`
- Modify: `clients/weft-code-tui/src/main.rs`
- Modify: `clients/weft-code-tui/src/client.rs`
- Modify: `clients/weft-code-tui/src/model.rs`
- Modify or replace: `clients/weft-code-tui/src/render.rs`
- Create: `clients/weft-code-tui/src/legacy_render.rs`
- Create: `clients/weft-code-tui/src/app/mod.rs`
- Create: `clients/weft-code-tui/src/app/state.rs`
- Create: `clients/weft-code-tui/src/app/action.rs`
- Create: `clients/weft-code-tui/src/app/update.rs`
- Create: `clients/weft-code-tui/src/app/runtime.rs`
- Create: `clients/weft-code-tui/src/tui/mod.rs`
- Create: `clients/weft-code-tui/src/tui/terminal.rs`
- Create: `clients/weft-code-tui/src/tui/events.rs`
- Create: `clients/weft-code-tui/src/tui/layout.rs`
- Create: `clients/weft-code-tui/src/tui/render.rs`
- Create: `clients/weft-code-tui/src/tui/widgets/top_bar.rs`
- Create: `clients/weft-code-tui/src/tui/widgets/sidebar.rs`
- Create: `clients/weft-code-tui/src/tui/widgets/main_pane.rs`
- Create: `clients/weft-code-tui/src/tui/widgets/detail_pane.rs`
- Create: `clients/weft-code-tui/src/tui/widgets/composer.rs`
- Modify: `docs/architecture/vNext/15-weft-code-client-surface.md`
- Modify: `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`

- [ ] **Step 1: Add the real TUI framework dependencies and stop treating the current text shell as the long-term UI foundation**

Update `clients/weft-code-tui/Cargo.toml` to add:

```toml
ratatui = "<current-compatible-version>"
crossterm = "<current-compatible-version>"
```

Choose versions that compile cleanly with the workspace toolchain already used by this repo. Do not add extra architectural crates yet unless the first shell pass proves they are necessary.

Expected outcome:

- The client crate has the two core dependencies required for a real full-screen TUI.
- No event stream client, component framework, or theme/config package is introduced in this first rebuild step.

- [ ] **Step 2: Introduce explicit TUI terminal lifecycle management**

Create `clients/weft-code-tui/src/tui/terminal.rs` with a small, explicit terminal lifecycle surface responsible for:

- enabling raw mode
- entering alternate screen
- optionally hiding the cursor while the app is active
- restoring the terminal on every exit path

Expected outcome:

- `tui` no longer writes a pseudo-screen into the normal shell buffer as its primary interaction mode.
- The app can enter and leave a full-screen terminal session without leaving the terminal in a broken state.

- [ ] **Step 3: Define a real app-state model instead of encoding UI behavior in command branches**

Create `clients/weft-code-tui/src/app/state.rs` and define the minimum state model needed for a first full-screen shell. At minimum include:

- `AppState`
- `Route`
- `FocusTarget`
- `InputMode`
- route-local selection indexes
- composer buffer
- loading / flash / quit flags

The state should be UI-local only. Do not duplicate runtime truth such as session/task/team semantics inside this layer.

- [ ] **Step 4: Define action/update boundaries so input handling and state mutation are no longer tangled inside `main.rs`**

Create:

- `clients/weft-code-tui/src/app/action.rs`
- `clients/weft-code-tui/src/app/update.rs`

Introduce a small action vocabulary for the first shell, such as:

- `Quit`
- `Refresh`
- `SetRoute`
- `MoveUp`
- `MoveDown`
- `FocusNext`
- `FocusPrev`
- `EnterInsert`
- `ExitInsert`
- `ComposerInput`
- `ComposerBackspace`
- `SubmitPrompt`
- `ApproveSelected`

Expected outcome:

- Future keybindings drive actions.
- State updates happen through an explicit reducer/update layer instead of ad hoc branching in the event loop.

- [ ] **Step 5: Split runtime-facing client behavior from TUI-local behavior**

Create `clients/weft-code-tui/src/app/runtime.rs` as a thin adapter over the existing `WeftCodeCliClient` so the TUI can call named operations like:

- refresh current status
- submit composer prompt
- approve currently selected approval
- optionally switch mode or session later

Do not reimplement HTTP transport or duplicate DTOs here. Reuse `src/client.rs` and `src/model.rs` as the transport/source-of-truth layer.

- [ ] **Step 6: Replace the pseudo-TUI string renderer with a ratatui frame renderer**

Create:

- `clients/weft-code-tui/src/tui/layout.rs`
- `clients/weft-code-tui/src/tui/render.rs`
- `clients/weft-code-tui/src/tui/widgets/top_bar.rs`
- `clients/weft-code-tui/src/tui/widgets/sidebar.rs`
- `clients/weft-code-tui/src/tui/widgets/main_pane.rs`
- `clients/weft-code-tui/src/tui/widgets/detail_pane.rs`
- `clients/weft-code-tui/src/tui/widgets/composer.rs`

Render the first full-screen shell with these regions:

- top bar
- sidebar
- main pane
- detail pane
- composer

The first version does not need to be visually elaborate, but it must be a real full-screen TUI frame rather than a formatted string block.

- [ ] **Step 7: Preserve the existing command-oriented CLI outputs by moving text rendering into a legacy module**

Move the current string-based CLI render helpers out of `src/render.rs` into a new `clients/weft-code-tui/src/legacy_render.rs`.

Keep text-mode commands such as:

- prompt submission output
- `approve`
- `approve-and-run`
- `sessions`
- `teams`
- `team-tasks`
- `mode`
- `status`

on the legacy renderer path.

Expected outcome:

- The repo keeps its useful text CLI outputs.
- The `tui` subcommand is no longer forced to share the same render implementation as one-shot CLI commands.

- [ ] **Step 8: Replace `run_live_tui_loop` with a real event loop**

Modify `clients/weft-code-tui/src/main.rs` so that `CliCommand::Tui` calls `tui::run_tui_app(...)` instead of the current `stdin.read_line()` loop.

Create `clients/weft-code-tui/src/tui/events.rs` and implement a first event loop that handles:

- key events
- terminal resize
- tick/refresh timing if needed

The first keymap should support at least:

- `q` or `Esc` for quit/back semantics where appropriate
- `1/2/3/4` or equivalent route switching
- `Tab` / `Shift+Tab` for focus movement
- arrow keys or `j/k` for selection movement
- `Enter` for primary action
- `i` for entering composer input mode
- `r` for refresh
- `a` for approval on approval focus

- [ ] **Step 9: Use live runtime data in the full-screen shell from the first real pass**

The first full-screen TUI should remain runtime-backed, not mock-backed.

At minimum it must display current live data for:

- policy
- sessions
- teams
- approvals
- tasks

The `ShellSnapshot::sample()` path may remain for tests/scaffold support, but it must not be the primary live TUI path.

- [ ] **Step 10: Make the composer a first-class interaction surface**

Add a real composer input area to the full-screen shell.

Phase-1 composer requirements:

- focusable input surface
- editable text buffer
- submit via `Enter` in insert/input mode
- runtime call through `create_natural_language_task(...)`
- visible loading/error/result feedback inside the TUI shell

Do not fall back to line-buffered stdin for prompt submission.

- [ ] **Step 11: Preserve and migrate the current approval loop into the new full-screen shell**

Migrate the already-working approval behavior into the new shell:

- select an approval in the relevant route/detail view
- approve it via the live runtime
- refresh the local status view after approval

If an inline retry path remains, it must continue to require an explicit prompt from the user. Do not add hidden prompt replay or synthetic prompt history in this slice.

- [ ] **Step 12: Retire the current pseudo-TUI loop once parity is reached**

After the full-screen shell supports:

- entering/exiting cleanly
- route switching
- focus movement
- live status display
- composer submit
- approval action

remove the old `run_live_tui_loop(...)` path and any dead code that only existed to support the line-oriented pseudo-TUI.

Expected outcome:

- The crate has one real TUI path instead of a transitional shell loop plus a future TUI that never arrives.

- [ ] **Step 13: Verify the rebuild as a UI architecture change, not just a compile change**

Minimum verification for this task:

```bash
cargo test -p weft-code-tui
cargo check -p weft-code-tui
```

Plus:

- `lsp_diagnostics` on `clients/weft-code-tui/src`
- manual QA for entering/exiting the full-screen shell
- manual QA for route switching and focus movement
- manual QA for prompt submission
- manual QA for approving a live approval

Success criteria:

- `cargo run -p weft-code-tui -- tui` opens a real full-screen TUI shell
- the terminal is restored correctly on exit
- the TUI is runtime-backed, not mock-only
- the old `read_line()`-driven pseudo-TUI is no longer the main path

Scope limits for Task 11:

- This task does **not** yet require event-stream subscription
- This task does **not** yet require mouse support
- This task does **not** yet require command palette or slash commands
- This task does **not** yet require theme/keybind config files
- This task does **not** yet require a full OpenCode-equivalent feature set

Task 11 is complete when WEFT-Code has crossed the architectural boundary from “runtime-backed text shell with a pseudo-TUI mode” to “runtime-backed full-screen TUI application foundation”.

> 2026-04-23 execution note
>
> - Completed this round by carrying `clients/weft-code-tui` across the full Task 11 P1 boundary into a real full-screen TUI foundation.
> - The crate now depends on `ratatui` + `crossterm`, preserves one-shot CLI output in `src/legacy_render.rs`, and uses explicit `app/` + `tui/` module boundaries instead of encoding TUI behavior inside `main.rs` string branches.
> - Non-watch `tui` now enters raw mode + alternate screen, renders a real frame-based layout with top bar / sidebar / main pane / detail pane / composer, and exits cleanly on `q` / `Esc`.
> - The full-screen shell is runtime-backed from first paint and now supports route switching, focus movement, bounded selection, runtime refresh, insert-mode composer input, prompt submission, and approval action against the selected live approval.
> - The old `run_live_tui_loop(...)` path and `stdin.read_line()`-driven pseudo-TUI main path have been retired; the remaining legacy text renderer is retained only for one-shot CLI output and scaffold/test support.
> - Next: run final live manual QA against `weft-core` with package-managed `weft-code-runtime`, then request Oracle acceptance before any completion claim.

Task 11 round result

- Files: `clients/weft-code-tui/Cargo.toml`, `clients/weft-code-tui/src/main.rs`, `clients/weft-code-tui/src/lib.rs`, `clients/weft-code-tui/src/render.rs`, `clients/weft-code-tui/src/legacy_render.rs`, `clients/weft-code-tui/src/app/mod.rs`, `clients/weft-code-tui/src/app/state.rs`, `clients/weft-code-tui/src/app/action.rs`, `clients/weft-code-tui/src/app/update.rs`, `clients/weft-code-tui/src/app/runtime.rs`, `clients/weft-code-tui/src/tui/mod.rs`, `clients/weft-code-tui/src/tui/terminal.rs`, `clients/weft-code-tui/src/tui/events.rs`, `clients/weft-code-tui/src/tui/layout.rs`, `clients/weft-code-tui/src/tui/render.rs`, `clients/weft-code-tui/src/tui/widgets/mod.rs`, `clients/weft-code-tui/src/tui/widgets/top_bar.rs`, `clients/weft-code-tui/src/tui/widgets/sidebar.rs`, `clients/weft-code-tui/src/tui/widgets/main_pane.rs`, `clients/weft-code-tui/src/tui/widgets/detail_pane.rs`, `clients/weft-code-tui/src/tui/widgets/composer.rs`, `docs/architecture/vNext/15-weft-code-client-surface.md`, `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`
- Verified so far:
  - `cargo test -p weft-code-tui` passed after the full-screen foundation, navigation, composer, and approval checkpoints landed.
  - `cargo check -p weft-code-tui` passed after the same checkpoint set.
  - `lsp_diagnostics` on `clients/weft-code-tui/src` reported 0 diagnostics.
  - Search confirms the old `run_live_tui_loop` / `stdin.read_line()` pseudo-TUI path is no longer present in `clients/weft-code-tui/src`.
- Remaining before completion:
  - Run final live manual QA against a running `weft-core` instance that starts package-managed `weft-code-runtime` for full-screen entry/exit, route/focus movement, prompt submission, and approval action.
  - Request Oracle acceptance and wait for the result before claiming Task 11 complete.

2026-04-20 natural-language-runtime lifecycle-contract note

- This continuation upgrades the additive `lifecycle` response field from a loose projection into a typed shared contract.
- `NaturalLanguageLifecycleInfo` now returns a typed `transition` enum and a minimal `record` object containing `to` and `reason`.
- `workflow_gateway.rs` derives lifecycle uniformly from returned task status across coding, plan, and team responses, without changing the underlying runtime behavior.
- `server/src/main.rs` now asserts representative lifecycle values on coding, plan, and team paths, including `approval_pending`, `accepted`, `policy_blocked`, and `completed`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 35 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed the blocked team response now includes `lifecycle = { state: blocked, transition: policy_blocked, record: { to: blocked, reason: policy_blocked } }`.
- Scope limit: this is still a minimal transition record only. It is not a workflow graph, event log, or historical state-transition system.

2026-04-20 natural-language-runtime lifecycle-from-state note

- This continuation makes the minimal transition record slightly more honest by filling `record.from` only where the runtime genuinely knows a prior state.
- Coding paths that stage queued work first and then return a later result now include `record.from = queued`, for example `queued -> waiting_approval` and `queued -> completed`.
- Paths where the runtime does not truly know a prior state from the response construction point (such as plan accepted or blocked team kickoff) still leave `record.from = null`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 35 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed a coding approval response now includes `record = { from: queued, to: waiting_approval, reason: approval_pending }`.
- Scope limit: this is still not historical transition tracking. It only fills `from` where the current runtime execution path explicitly knows the prior queued state.

2026-04-20 natural-language-runtime plan-freeze note

- This continuation gives `plan` mode a real execution semantic instead of only response metadata.
- `workflow_gateway.rs` now treats a session already in `plan` mode as execution-frozen for later coding-like requests: the runtime returns a `blocked` `coding_task`, preserves `action_kind`, and skips patch / write / verify execution.
- The original plan request behavior remains unchanged: entering plan mode still returns a queued `plan_task` with `action_kind = plan`.
- `server/src/main.rs` now covers the freeze behavior with a focused route test that switches the session into plan mode and then issues a patch-style request, asserting blocked status and no execution side effects.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 30 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed the blocked follow-up coding request while the session remained in `plan` mode.
- Scope limit: this is a minimal plan-mode execution freeze only. It does not add plan approval semantics, orchestration, or broader policy layering.

2026-04-20 natural-language-runtime plan-resume note

- This continuation documents and verifies the minimal supported way to resume execution after plan freeze.
- The runtime reuses the existing session mode route: `POST /api/weft-code/session/{session_id}/mode` with `{"mode":"coding"}`.
- After switching the session back to `coding`, later coding-like requests execute normally again under the active policy instead of remaining frozen by `plan` mode.
- Manual HTTP QA passed after clean restart: enter `plan` mode, observe a blocked patch request, switch the mode back to `coding`, then reissue the same patch request and observe normal execution with `file_mutation` and `patch_record`.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 31 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed resume-by-mode-switch behavior.
- Scope limit: this is only a minimal resume-by-mode-switch path. It does not add plan approval semantics, orchestration resume, or a richer session lifecycle model.

2026-04-20 natural-language-runtime action-summary note

- This continuation adds a small additive `action` summary object to `NaturalLanguageTaskResponse`.
- The new summary contains only `kind`, `task_kind`, and `status`, and it is populated on the coding, plan, and team paths from the same runtime truth already exposed through `task` and `action_kind`.
- Existing detailed fields (`task`, `approval`, `patch_record`, `created_team`, `file_mutation`) are unchanged; the summary is purely a client-facing convenience layer.
- `server/src/main.rs` now asserts `action.kind`, `action.task_kind`, and `action.status` on representative coding, plan, and team responses.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 29 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and manual HTTP QA confirmed the response includes the expected `action` summary.
- Scope limit: this is a small response-level unification only. It does not replace existing fields, introduce a workflow graph, or broaden approval/policy semantics.

2026-04-20 natural-language-runtime read-only-action-kind note

- This continuation aligns `read_only_mode` with the current coding action taxonomy.
- `workflow_gateway.rs` now uses the resolved `CodingActionKind` rather than raw prompt text to decide read-only behavior.
- In `read_only_mode`, `patch` and `write_note` are both blocked, while `analyze` is allowed to run the analyze-only path even if the raw prompt text still looks mutation-oriented.
- `server/src/main.rs` now includes route tests proving that explicit `action_kind = write_note` is blocked in read_only mode, while explicit `action_kind = analyze` with a patch-looking prompt still runs only the analyze step.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route` passed with 29 tests, `cargo check -p registry-server` passed, and `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics.
- Scope limit: this still does not make every policy fully action-kind-native across every flow. It only closes the mismatch between `read_only_mode` and the current coding-path taxonomy.

2026-04-20 natural-language-runtime approval-key-hardening note

- Oracle review found a collision risk in the first per-request approval key because prompt bytes and target_id bytes were concatenated without separators.
- `workflow_gateway.rs` now serializes approval-key input with explicit field names and length prefixes before hashing, so `(prompt="ab", target_id="c")` cannot collide with `(prompt="a", target_id="bc")`.
- `server/src/main.rs` now includes a regression test proving those two request shapes receive distinct approval ids.
- The route suite was also aligned to consume approval ids from the pending response body instead of relying on stale hard-coded approval ids, which keeps tests robust under per-request gating.
- Verification for this continuation: `cargo test -p registry-server weft_code_natural_language_route -- --nocapture` passed with 21 tests, `cargo check -p registry-server` passed, `lsp_diagnostics` on `server/src/weft_code` returned 0 diagnostics, and clean manual HTTP QA confirmed an approval id of the form `weft-code-local-session-approval-<hash>` flowing through a full patch request.
- Note: on Windows, rerunning `cargo check` immediately after the nocapture test run may require clearing lingering `registry_server` processes first to release the test executable lock; this was done in verification and does not affect runtime behavior.
- Scope limit: this hardens the request-key identity model only. It does not introduce richer approval policies, replay protection, or multi-approver semantics.

---

## Spec Coverage Checklist

The plan covers the approved spec requirements as follows:

- **独立产品身份** → Tasks 1, 3
- **package reuse 优先** → Tasks 1, 2
- **core/runtime/clients separation** → Tasks 3, 4, 8, 9
- **project/session/task/team/approval/event stream 资源模型** → Tasks 4, 5, 6, 7, 8
- **session lifecycle / lineage / task state** → Tasks 4, 7
- **coding/plan/team 模式** → Task 6
- **team mode 正式执行模型** → Task 8
- **P0/P1/P2 staged rollout** → This whole phased plan plus Tasks 9–10 for forward-compatible boundaries

No spec section is intentionally omitted. What is deferred is explicitly called out as P1/P2 or future client work rather than left implicit.

## Placeholder Scan

Checked for and removed the following failure patterns:

- no `TBD`
- no `TODO`
- no “implement later” placeholders
- no “similar to above” references without concrete paths
- each code-bearing step includes concrete sample content

## Type/Name Consistency Check

This plan consistently uses the following vocabulary:

- product: `WEFT-Code`
- package: `weft-code`
- features: `weft-code-core-feature`, `weft-code-team-feature`
- runtime resource types: `ProjectInfo`, `SessionInfo`, `TaskInfo`, `TeamInfo`, `TeamTaskInfo`
- modes: `Coding`, `Plan`, `Team`

No conflicting alternative names are introduced.

---

Plan complete and saved to `docs/superpowers/plans/2026-04-15-weft-code-implementation.md`.

Execution default for future workers:

- Continue in-session without asking the user to choose an execution mode.
- Progress one smallest justified checkpoint at a time.
- Ask the user only when blocked by a real decision, missing external information, or conflicting requirements.
