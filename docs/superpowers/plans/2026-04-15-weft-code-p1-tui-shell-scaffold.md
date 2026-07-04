# WEFT-Code P1 TUI Shell Scaffold Implementation Plan

> **For agentic workers:** Execute this plan continuously in-session by default. Do not stop to ask the user which execution mode to use. Move from one natural checkpoint to the next smallest justified checkpoint autonomously, and only pause when a hard blocker, real ambiguity, or explicit user intervention requires it. Keep checkbox (`- [ ]`) tracking updated as work is completed.

**Goal:** Build the smallest P1 TUI shell scaffold that uses a local mock snapshot shaped after the current WEFT-Code runtime resources to validate a terminal-facing information architecture without implementing a full interactive TUI.

**Architecture:** Add a new `weft-code-tui` workspace crate that renders a static local snapshot mirroring the current registry-server runtime resource shapes. The first slice is read-only and local: it models the TUI shell layout, validates sections with tests, and fixes the shell information architecture before any real runtime client or transport integration is added.

**Tech Stack:** Rust workspace, `serde`, `serde_json`, existing `registry-server` runtime resource shapes, snapshot-style string tests using plain `cargo test`.

---

## File Structure

- Create `clients/weft-code-tui/Cargo.toml` — package manifest for the new TUI scaffold crate.
- Create `clients/weft-code-tui/src/lib.rs` — public API for rendering a read-only shell snapshot.
- Create `clients/weft-code-tui/src/model.rs` — local view model mirroring the existing runtime resources needed by the shell.
- Create `clients/weft-code-tui/src/render.rs` — deterministic text renderer for the first shell layout.
- Create `clients/weft-code-tui/src/main.rs` — minimal binary entrypoint that prints the scaffold snapshot.
- Modify `Cargo.toml` — add `clients/weft-code-tui` to the workspace members.
- Modify `docs/architecture/vNext/15-weft-code-client-surface.md` — record the P1 TUI shell scaffold boundary after implementation.

This plan intentionally does not add terminal event handling, keybindings, palette commands, persistent history, live SSE subscription, or a real runtime client. Those are later P1 slices.

This plan also does not claim that the scaffold already consumes live runtime resources. In this slice, the crate only mirrors the current runtime resource shapes with local sample data.

---

### Task 1: Workspace crate skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `clients/weft-code-tui/Cargo.toml`
- Create: `clients/weft-code-tui/src/lib.rs`
- Create: `clients/weft-code-tui/src/main.rs`

- [x] **Step 1: Add the crate to the workspace**

In root `Cargo.toml`, extend the workspace members list:

```toml
[workspace]
members = [
    "core",
    "packages/sdk",
    "server",
    "clients/weft-code-tui",
]
```

- [x] **Step 2: Create the TUI crate manifest**

Create `clients/weft-code-tui/Cargo.toml`:

```toml
[package]
name = "weft-code-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
```

- [x] **Step 3: Create the library module root**

Create `clients/weft-code-tui/src/lib.rs`:

```rust
pub mod model;
pub mod render;

pub use model::ShellSnapshot;
pub use render::render;
```

- [x] **Step 4: Create a minimal binary entrypoint**

Create `clients/weft-code-tui/src/main.rs`:

```rust
use weft_code_tui::{render, ShellSnapshot};

fn main() {
    println!("{}", render(&ShellSnapshot::sample()));
}
```

- [x] **Step 5: Verify the new crate compiles enough to fail on missing modules**

Run:

```bash
cargo check -p weft-code-tui
```

Expected: FAIL because `model` and `render` modules do not exist yet.

---

### Task 2: TUI shell view model

**Files:**
- Create: `clients/weft-code-tui/src/model.rs`

- [x] **Step 1: Write the view model**

Create `clients/weft-code-tui/src/model.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellSnapshot {
    pub project: ProjectView,
    pub session: SessionView,
    pub tasks: Vec<TaskView>,
    pub teams: Vec<TeamView>,
    pub approvals: Vec<ApprovalView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectView {
    pub id: String,
    pub root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionView {
    pub id: String,
    pub mode: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskView {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamView {
    pub id: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalView {
    pub id: String,
    pub status: String,
}

impl ShellSnapshot {
    pub fn sample() -> Self {
        Self {
            project: ProjectView {
                id: "weft-code-local-project".into(),
                root: ".".into(),
            },
            session: SessionView {
                id: "weft-code-local-session".into(),
                mode: "coding".into(),
                status: "active".into(),
            },
            tasks: vec![TaskView {
                id: "weft-code-bootstrap-task".into(),
                status: "queued".into(),
            }],
            teams: vec![TeamView {
                id: "weft-code-local-team".into(),
                roles: vec!["operator".into()],
            }],
            approvals: vec![ApprovalView {
                id: "weft-code-bootstrap-approval".into(),
                status: "pending".into(),
            }],
        }
    }
}
```

- [x] **Step 2: Verify the crate still fails only because render is missing**

Run:

```bash
cargo check -p weft-code-tui
```

Expected: FAIL because `clients/weft-code-tui/src/render.rs` does not exist yet.

---

### Task 3: Deterministic shell renderer

**Files:**
- Create: `clients/weft-code-tui/src/render.rs`

- [x] **Step 1: Write a failing renderer test first**

Create `clients/weft-code-tui/src/render.rs` with this test and a stub:

```rust
use crate::ShellSnapshot;

pub fn render(_snapshot: &ShellSnapshot) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_first_tui_information_architecture() {
        let output = render(&ShellSnapshot::sample());

        assert!(output.contains("WEFT-Code TUI Shell"));
        assert!(output.contains("Project: weft-code-local-project"));
        assert!(output.contains("Session: weft-code-local-session [coding/active]"));
        assert!(output.contains("Tasks"));
        assert!(output.contains("Teams"));
        assert!(output.contains("Approvals"));
        assert!(output.contains("Events: subscribe via runtime"));
    }
}
```

- [x] **Step 2: Run the failing test**

Run:

```bash
cargo test -p weft-code-tui renders_first_tui_information_architecture
```

Expected: FAIL because `render()` returns an empty string.

- [x] **Step 3: Implement the minimal renderer**

Replace the `render` function in `clients/weft-code-tui/src/render.rs`:

```rust
pub fn render(snapshot: &ShellSnapshot) -> String {
    let tasks = snapshot
        .tasks
        .iter()
        .map(|task| format!("- {} [{}]", task.id, task.status))
        .collect::<Vec<_>>()
        .join("\n");
    let teams = snapshot
        .teams
        .iter()
        .map(|team| format!("- {} roles={}", team.id, team.roles.join(",")))
        .collect::<Vec<_>>()
        .join("\n");
    let approvals = snapshot
        .approvals
        .iter()
        .map(|approval| format!("- {} [{}]", approval.id, approval.status))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "WEFT-Code TUI Shell\n\nProject: {}\nRoot: {}\nSession: {} [{}/{}]\n\nTasks\n{}\n\nTeams\n{}\n\nApprovals\n{}\n\nEvents: subscribe via runtime",
        snapshot.project.id,
        snapshot.project.root,
        snapshot.session.id,
        snapshot.session.mode,
        snapshot.session.status,
        tasks,
        teams,
        approvals
    )
}
```

- [x] **Step 4: Run the renderer test again**

Run:

```bash
cargo test -p weft-code-tui renders_first_tui_information_architecture
```

Expected: PASS.

---

### Task 4: Binary smoke test and documentation record

**Files:**
- Modify: `docs/architecture/vNext/15-weft-code-client-surface.md`
- Modify: this plan file

- [x] **Step 1: Run the binary manually**

Run:

```bash
cargo run -p weft-code-tui
```

Expected output contains:

```text
WEFT-Code TUI Shell
Project: weft-code-local-project
Session: weft-code-local-session [coding/active]
Tasks
Teams
Approvals
Events: subscribe via runtime
```

- [x] **Step 2: Run compile and test checks**

Run:

```bash
cargo test -p weft-code-tui
cargo check -p weft-code-tui
```

Expected: both pass.

- [x] **Step 3: Update the client surface doc**

Append this section to `docs/architecture/vNext/15-weft-code-client-surface.md`:

```md
## P1 最小 TUI Shell Scaffold

P1 的第一刀不是完整 TUI，而是一个只读 shell scaffold，用本地 mock snapshot 验证与 runtime resources 对齐的 TUI 信息架构。

这个 scaffold 只负责展示：

- project
- session
- task
- team
- approval
- event stream 入口提示

它不实现完整 CLI/TUI、不接管 core truth、不实现 persistent task replay。
```

- [x] **Step 4: Record the round result in this plan**

Add an execution note under this task with the exact files changed and command outputs observed.

> 2026-04-18 execution note
>
> - Completed this round by running the TUI scaffold binary, confirming the shell snapshot output, and updating the client-surface note with the P1 scaffold boundary.
> - `cargo run -p weft-code-tui` printed the expected shell sections for project / session / task / team / approval / event entry.
> - `cargo test -p weft-code-tui` and `cargo check -p weft-code-tui` both passed.
> - Next: Task 4 is complete; the next continuation should be a new P1 slice, not another step in this scaffold plan.

Task 4 round result

- Files: `docs/architecture/vNext/15-weft-code-client-surface.md`, `docs/superpowers/plans/2026-04-15-weft-code-p1-tui-shell-scaffold.md`
- Verified:
  - `cargo run -p weft-code-tui` printed the expected shell snapshot.
  - `cargo test -p weft-code-tui` passed.
  - `cargo check -p weft-code-tui` passed.
  - `15-weft-code-client-surface.md` now includes the `## P1 最小 TUI Shell Scaffold` section.
- Notes:
  - This round stayed within Task 4 only: binary smoke test, crate verification, and client-surface doc update.
  - No terminal event handling, palette commands, persistent history, live SSE subscription, or real runtime client integration were implemented.
- Next: the P1 TUI shell scaffold plan is complete.

2026-04-18 consistency note

- The plan checkboxes are now aligned with the repository state verified in this continuation: the workspace member, crate manifest, library root, binary entrypoint, view model, renderer, client-surface documentation, and Task 4 record are all present.
- Verification rerun in this continuation: `cargo test -p weft-code-tui` passed, `cargo check -p weft-code-tui` passed, and `cargo run -p weft-code-tui` printed the expected shell snapshot. Markdown LSP diagnostics could not run because this environment has no `.md` LSP server configured.
- This note is documentation-only and does not change the scaffold implementation or expand the P1 scope. The next continuation should start a new P1 slice from the WEFT-Code truth sources instead of extending this scaffold plan.

2026-04-18 scope correction note

- Oracle review correctly identified that the existing crate is a local mock scaffold, not a real runtime-resource projection.
- The plan goal and architecture are therefore narrowed to match the repository reality: this slice validates TUI information architecture with a local snapshot shaped after current runtime resources.
- Live runtime consumption remains a later P1 slice and is not claimed as completed by this scaffold plan.

2026-04-18 top-bar-and-focus checkpoint note

- This continuation advanced the local mock scaffold by extending `clients/weft-code-tui/src/model.rs` and `clients/weft-code-tui/src/render.rs` only.
- The scaffold now renders a static `Top Bar` with project / session / mode / model / approval / policy plus three static observation sections: `Conversation Focus`, `Task Focus`, and `Team Focus`.
- The implementation still uses `ShellSnapshot::sample()` and does not consume live runtime resources, implement focus switching, or add interactive input / overlay behavior.
- Verification for this continuation: `cargo test -p weft-code-tui` passed, `cargo check -p weft-code-tui` passed, `lsp_diagnostics` on `clients/weft-code-tui` returned 0 diagnostics, and `cargo run -p weft-code-tui` printed the new top-bar and focus sections.
- Oracle post-repair acceptance: PASS. Oracle verified this as a fully recorded, validated minimal closure and returned `<promise>VERIFIED</promise>` for the top-bar/focus checkpoint.
- Next: this top-bar/focus checkpoint is complete; any runtime-backed TUI behavior must start as a separate P1 slice.

---

## Spec Coverage Checklist

- **P1 TUI 信息架构** → Task 3 renders project/session/task/team/approval/event sections.
- **P1 TUI top bar + focus scaffold** → 2026-04-18 top-bar-and-focus checkpoint adds top bar metadata plus conversation/task/team static focus sections.
- **Clients consume runtime, not core** → Task 4 doc update states this scaffold mirrors runtime resource shapes without claiming live runtime consumption.
- **No full CLI/TUI implementation** → Task 4 explicitly keeps full CLI/TUI and persistent replay out of scope.
- **No P2 expansion** → This plan does not touch GUI, remote runtime hosting, team orchestration expansion, or multi-client persistence.

## Placeholder Scan

Checked for and avoided:

- no `TBD`
- no `TODO`
- no “implement later” as a plan step
- no unspecified test commands
- no references to missing functions without defining them in an earlier task

## Type/Name Consistency Check

This plan consistently uses:

- crate: `weft-code-tui`
- model root: `ShellSnapshot`
- renderer: `render(&ShellSnapshot) -> String`
- binary package command: `cargo run -p weft-code-tui`

Plan complete and saved to `docs/superpowers/plans/2026-04-15-weft-code-p1-tui-shell-scaffold.md`.

Execution default for future workers:

- Continue in-session without asking the user to choose an execution mode.
- Progress one smallest justified checkpoint at a time.
- Ask the user only when blocked by a real decision, missing external information, or conflicting requirements.
