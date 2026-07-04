# Weft Claw Package Blueprint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align the WEFT vNext blueprint and repository evolution path so Weft Claw becomes a multi-package assembled product package with explicit provider packages, foundation packages, feature packages, and a medium-thickness product shell.

**Architecture:** First make the documentation model internally consistent across `02/03/04/05/09`, then introduce a canonical capability/package matrix so naming and boundaries stop drifting. After the docs agree, stage the repository migration in narrow slices: capability naming compatibility, package taxonomy, feature-package composition, and finally `weft-claw` product package plus instance evolution.

**Tech Stack:** Markdown architecture docs, TOML package manifests and instance state, Rust core resolution model, existing package index + lock pipeline.

---

## File Structure and Responsibilities

- `docs/architecture/vNext/02-app-model.md`
  - User/product view of App; must describe medium-thickness app shell and feature-package composition.
- `docs/architecture/vNext/03-package-model.md`
  - Canonical package taxonomy; must describe provider/foundation/feature/app layering.
- `docs/architecture/vNext/04-declaration-config-lock.md`
  - Canonical declaration/config/lock model; must show how feature packages, provider bindings, overrides, and lock snapshots are represented.
- `docs/architecture/vNext/05-binding-and-resolution.md`
  - Canonical capability and binding semantics; must describe three-layer binding and capability naming migration.
- `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
  - Product-specific detailed blueprint; already expanded, but must be treated as source-of-truth input during sync.
- this approved design spec
  - Approved design source; use only as reference, do not let implementation drift away from it.
- `packages/index.toml`
  - Current package source index; later migration target for taxonomy/category metadata and transitional naming.
- `packages/weft-claw/package.toml`
  - Current product package declaration; migration target for finer capabilities, feature packages, and binding structure.
- `.weft/weft-claw/config.toml`
  - Current instance runtime config; migration target for feature toggles and provider overrides.
- `.weft/weft-claw/lock.toml`
  - Current lock snapshot; migration target for feature-package participation, richer binding snapshot, and migration compatibility.
- `core/src/app/mod.rs`
  - Shared app/package model types; likely update point for capability naming migration and feature package representation.
- `core/src/app/config.rs`
  - Declaration/config/lock load-save logic; likely update point for schema evolution.
- `core/src/app/packages.rs`
  - Package index parsing and source metadata; likely update point for taxonomy metadata.
- `core/src/app/resolve.rs`
  - Resolve/binding model; likely update point for new capability naming and feature composition.
- `core/src/api/apps.rs`
  - App API output; likely update point if app/feature/binding metadata becomes user-visible in developer view.
- `core/tests/integration.rs`
  - High-value regression suite for app/package/binding/generation behavior.

## Implementation Phases

1. **Blueprint sync** — make `02/03/04/05/09` agree on package layers, capability names, binding levels, and app shell boundaries.
2. **Capability/package catalog** — create one explicit matrix so later implementation cannot reinterpret names differently.
3. **Schema evolution plan** — specify how declaration/config/lock and package index represent provider/foundation/feature layers without breaking current runtime.
4. **Repository migration slices** — implement taxonomy and capability migration in compatibility-preserving steps.
5. **Verification gates** — add doc review and runtime checkpoints after each migration slice.

### Task 1: Synchronize the product model with the new Weft Claw shell boundary

**Files:**
- Modify: `docs/architecture/vNext/02-app-model.md`
- Reference: `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
- Reference: the approved design spec for this blueprint

- [ ] **Step 1: Diff the current product model against the new Weft Claw blueprint**

Read these sections and note every contradiction:

```text
docs/architecture/vNext/02-app-model.md
docs/architecture/vNext/09-weft-claw-product-blueprint.md
the approved design spec for this blueprint
```

Expected findings to capture before editing:

```text
- 02 still describes coarse capabilities like agent.capability/tool.registry/memory.backend
- 02 still implies app directly binds to packages without feature-package layer
- 02 does not define a medium-thickness app shell
```

- [ ] **Step 2: Rewrite the App definition section to include feature packages and app shell boundaries**

Add or rewrite content so `02-app-model.md` explicitly says:

```md
Product Package 仍然是一等用户产品单位，但内部不再只由“若干 Package + 默认 binding”构成。

在 Weft Claw 这类复杂产品中，产品的内部结构至少分为：
- provider packages
- foundation packages
- feature packages
- product shell

其中 product shell 只保留产品身份、默认装配、默认策略和对外入口，不承载大量底层实现。
```

- [ ] **Step 3: Replace coarse example bindings with layered examples**

Update the examples from this pattern:

```md
- `agent.capability -> agent-core`
- `tool.registry -> skills`
```

to this pattern:

```md
- `agent.runtime -> agent-runtime`
- `prompt.system -> prompt-system`
- `workflow.orchestration -> workflow-orchestrator`
- `tool.runtime -> tool-runtime-core`
- `tool.shell -> tool-shell`
- `memory.store -> memory-store`
- `channel.bridge -> channel-core`
- `ext.mcp -> mcp-client`
- `ext.skills -> skills-runtime`
```

- [ ] **Step 4: Add a short section defining “Product shell responsibility”**

Insert explicit bullets like:

```md
Product shell 保留：
- feature 启用列表
- 默认 bindings
- profile/workspace/feature 开关
- UI 与 product-level 路由

Product shell 不保留：
- agent loop 实现
- prompt 文本本体
- channel adapter 细节
- MCP/skills 装载实现
```

- [ ] **Step 5: Review the finished document for language drift**

Run a manual scan for stale phrases such as:

```text
agent.capability
tool.registry
memory.backend
execution.capability
communication provider
```

Expected: either removed, or explicitly marked as legacy/transitional names.

### Task 2: Rewrite the Package model around provider/foundation/feature layers

**Files:**
- Modify: `docs/architecture/vNext/03-package-model.md`
- Reference: `docs/architecture/vNext/09-weft-claw-product-blueprint.md`

- [ ] **Step 1: Identify the exact sections that still assume a single package layer**

Before editing, verify these current assumptions exist:

```md
- Package is described as one generic internal implementation unit
- No distinction between provider package and feature package
- No guidance for extension packages like mcp-client / skills-runtime
```

- [ ] **Step 2: Add a new section “Package taxonomy”**

Write this taxonomy into `03-package-model.md`:

```md
Package 在 vNext 中分为三类：

1. Provider Package
   - 提供原子 capability 实现
   - 例如 tool-shell、channel-telegram、memory-store

2. Foundation Package
   - 把一类 provider 组织成稳定能力面
   - 例如 agent-runtime、prompt-system、workflow-orchestrator、tool-runtime-core、channel-core

3. Feature Package
   - 面向具体产品语义组合多个 capability
- 例如 weft-claw-chat-feature、weft-claw-channel-feature、weft-claw-automation-feature、weft-claw-extensions-feature
```

- [ ] **Step 3: Keep MCP and skills explicitly simple**

Add an exception rule like:

```md
`mcp-client` 与 `skills-runtime` 仍然是独立 package，但第一阶段不再继续拆成更多层；它们保留可插拔性，不追求形式上的分层完整。
```

- [ ] **Step 4: Extend the package metadata section to cover taxonomy metadata**

Expand the package metadata list so it includes at least:

```md
- package kind (`provider` / `foundation` / `feature`)
- provided capabilities
- required capabilities
- optional feature dependencies
- compatibility/runtime info
```

- [ ] **Step 5: Add a migration note for current repository state**

Insert a section that explicitly says the current repo is still in a transitional state:

```md
当前仓库中的 package 仍以单层索引为主，但后续迁移会逐步引入 taxonomy metadata，而不是一次性重写所有 package manifest。
```

### Task 3: Rewrite Binding and Resolve around three-layer binding

**Files:**
- Modify: `docs/architecture/vNext/05-binding-and-resolution.md`
- Reference: `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
- Reference: `docs/architecture/vNext/03-package-model.md`

- [ ] **Step 1: Replace coarse capability examples with the new capability set**

Change examples like:

```md
- `agent.capability`
- `tool.registry`
- `memory.backend`
```

to the new set:

```md
- `agent.runtime`
- `prompt.system`
- `workflow.orchestration`
- `tool.runtime`
- `tool.shell`
- `tool.files`
- `tool.web`
- `tool.git`
- `memory.store`
- `channel.bridge`
- `ext.mcp`
- `ext.skills`
- `ui.surface`
```

- [ ] **Step 2: Insert a new “Three-layer binding” section**

Add this structure:

```md
1. App Default Binding
2. Feature-level Binding
3. User / Runtime Override
```

and define each one in detail, using Weft Claw examples.

- [ ] **Step 3: Rewrite Fixed/Mutable rules using the new categories**

Update the document so it no longer only says “UI fixed, memory mutable.” It should include:

```md
通常固定：ui.surface、产品入口、核心 feature package
通常 mutable：tool providers、channel adapters、memory.store、ext.mcp、ext.skills
条件 mutable：agent.runtime、prompt.system、workflow.orchestration
```

- [ ] **Step 4: Expand Resolve responsibilities to include feature composition**

Update the resolve sequence from:

```md
1. read required capabilities
2. collect packages
3. choose providers
```

to a layered flow like:

```md
1. read app required capabilities
2. expand enabled feature packages
3. collect candidate provider/foundation/feature packages
4. apply app default bindings
5. apply feature-level defaults
6. apply runtime overrides
7. apply profile/policy restrictions
8. generate candidate bindings and lock snapshot
```

- [ ] **Step 5: Add a compatibility subsection for naming migration**

Document the migration rule explicitly:

```md
旧 capability 名（如 agent.capability / tool.registry / memory.backend / execution.capability）在迁移期保留兼容映射，但新文档和新实现一律以新 capability 名为主。
```

### Task 4: Rewrite Declaration/Config/Lock for multi-package app assembly

**Files:**
- Modify: `docs/architecture/vNext/04-declaration-config-lock.md`
- Reference: `packages/weft-claw/package.toml`
- Reference: `.weft/weft-claw/config.toml`
- Reference: `.weft/weft-claw/lock.toml`

- [ ] **Step 1: Replace the simplified declaration example with a layered example**

Update the declaration sample from coarse capabilities to something like:

```toml
[product]
name = "weft-claw"
version = "0.1.0"
display_name = "Weft Claw"

[requires]
capabilities = [
  "agent.runtime",
  "prompt.system",
  "workflow.orchestration",
  "tool.runtime",
  "memory.store",
  "channel.bridge",
  "ext.mcp",
  "ext.skills",
  "ui.surface"
]

[features]
enabled = [
  "weft-claw-chat-feature",
  "weft-claw-automation-feature",
  "weft-claw-extensions-feature"
]
```

- [ ] **Step 2: Add explicit examples for fixed and mutable bindings**

Extend the sample with both strict and flexible bindings:

```toml
[bindings.ui.surface]
provider = "weft-claw-ui"
mutable = false

[bindings.memory.store]
provider = "memory-store"
mutable = true
allowed = ["memory-store", "future-memory-store"]

[bindings.ext.mcp]
provider = "mcp-client"
mutable = true
allowed = ["mcp-client"]
```

- [ ] **Step 3: Expand the config example to show feature toggles and overrides**

Add concrete sections like:

```toml
[app_runtime]
profile = "developer"
workspace = "D:/work/weft-claw"

[features]
channels = true
mcp = true
skills = true

[overrides.bindings]
"tool.shell" = "tool-shell"
"channel.bridge" = "channel-core"
```

- [ ] **Step 4: Expand the lock example to show assembled output, not just package list**

Update the lock example so it contains at least:

```toml
lock_version = 1
product = "weft-claw"
generation = 42

[[features]]
name = "weft-claw-chat-feature"
resolved = true

[[packages]]
name = "agent-runtime"
kind = "foundation"

[[packages]]
name = "tool-shell"
kind = "provider"

[bindings]
"agent.runtime" = "agent-runtime"
"tool.shell" = "tool-shell"
```

- [ ] **Step 5: Add a migration note for current runtime compatibility**

Insert text making this explicit:

```md
第一阶段实现允许 lock 先表达 richer snapshot，而不是一次性要求运行时代码立刻完整消费全部新字段；但新字段必须服务于未来 resolve/verify/activate 的一致性。
```

### Task 5: Create the canonical capability/package migration matrix

**Files:**
- Create: `docs/architecture/vNext/10-weft-claw-capability-package-matrix.md`
- Reference: `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
- Reference: `docs/architecture/vNext/03-package-model.md`
- Reference: `docs/architecture/vNext/05-binding-and-resolution.md`

- [ ] **Step 1: Create the matrix document with a strict schema**

Start the file with a table like:

```md
| Legacy capability | New capability | Package kind | Default provider | Mutable? | Notes |
|---|---|---|---|---|---|
| agent.capability | agent.runtime | foundation | agent-runtime | conditional | turn loop only |
| tool.registry | tool.runtime | foundation | tool-runtime-core | yes | registry/protocol layer |
| memory.backend | memory.store | provider | memory-store | yes | storage backend |
| execution.capability | tool.shell | provider | tool-shell | yes | command execution |
| communication.bridge | channel.bridge | foundation | channel-core | yes | bridge, not concrete adapters |
```

- [ ] **Step 2: Add package grouping tables below the migration table**

Include three explicit sections:

```md
## Provider Packages
## Foundation Packages
## Feature Packages
```

and list the initial Weft Claw package set in each.

- [ ] **Step 3: Add a “what stays in the app shell” table**

Include a table like:

```md
| Concern | Lives in app shell? | Lives in package? | Notes |
|---|---|---|---|
| default feature enablement | yes | no | product assembly |
| prompt text templates | no | yes (`prompt-system`) | reusable |
| channel adapter protocol | no | yes | adapter-specific |
```

- [ ] **Step 4: Cross-link the new matrix from 03/05/09**

Add one-line references such as:

```md
详细 capability/package 对照见 `10-weft-claw-capability-package-matrix.md`。
```

### Task 6: Plan the repository migration sequence without breaking runtime

**Files:**
- Modify: the approved design spec for this blueprint
- Create: runtime migration notes for this blueprint
- Reference: `packages/index.toml`
- Reference: `packages/weft-claw/package.toml`
- Reference: `.weft/weft-claw/config.toml`
- Reference: `.weft/weft-claw/lock.toml`
- Reference: `core/src/app/mod.rs`
- Reference: `core/src/app/config.rs`
- Reference: `core/src/app/packages.rs`
- Reference: `core/src/app/resolve.rs`

- [ ] **Step 1: Write the migration stages explicitly in the notes file**

Create the migration stages as text, in order:

```md
1. Documentation canonicalization
2. Capability-name compatibility layer
3. Package taxonomy metadata in index/manifests
4. Feature package representation in declaration/config/lock
5. Resolve/binding runtime awareness of feature composition
6. Weft Claw product package migration to new names/defaults
7. Removal of legacy names after compatibility window
```

- [ ] **Step 2: Add “do not break current runtime” constraints**

Write explicit constraints like:

```md
- Do not rename all capabilities in one patch.
- Keep legacy capability aliases during the migration window.
- Do not require package index consumers to understand all new taxonomy fields immediately.
- Evolve lock format additively before tightening validation.
```

- [ ] **Step 3: Add a file-by-file runtime evolution map**

For each runtime file, describe the role of future edits:

```md
- `core/src/app/mod.rs`: add taxonomy and richer capability/binding types
- `core/src/app/config.rs`: extend declaration/config/lock parsing and serialization
- `core/src/app/packages.rs`: parse taxonomy metadata from package index
- `core/src/app/resolve.rs`: implement feature expansion and compatibility alias resolution
```

- [ ] **Step 4: Add sequencing rules for package creation**

Document that package creation should happen in this order:

```md
1. foundation skeletons
2. provider adapters
3. feature packages
4. app shell rewiring
```

### Task 7: Define verification checkpoints before implementation starts

**Files:**
- Modify: this implementation plan
- Reference: `core/tests/integration.rs`
- Reference: `.vnext-migration/STATUS.md`

- [ ] **Step 1: Add doc-sync verification checkpoints**

Append a section like:

```md
Doc sync is complete only when:
- 02/03/04/05/09 use the same package taxonomy
- old capability names are either removed or explicitly marked legacy
- app shell boundaries are stated the same way in all docs
```

- [ ] **Step 2: Add runtime migration verification checkpoints**

Define runtime acceptance targets before implementation begins:

```md
- `cargo test -p weft-core` stays green throughout migration
- existing product APIs keep working during compatibility phase
- lock files continue to serialize and deserialize existing snapshots before stricter format rollout
- package index can parse additive taxonomy fields without breaking current paths
```

- [ ] **Step 3: Add developer-review checkpoints**

Insert explicit review gates:

```md
Checkpoint A: docs only
Checkpoint B: capability alias layer landed
Checkpoint C: taxonomy metadata landed
Checkpoint D: weft-claw product package switched to new blueprint
```

### Task 8: Self-review the plan against the approved spec

**Files:**
- Modify: this implementation plan
- Reference: the approved design spec for this blueprint

- [ ] **Step 1: Spec coverage scan**

Before finalizing, verify the plan covers these approved areas:

```text
- provider/foundation/feature/app shell layering
- MCP and skills as single-layer packages
- capability naming migration
- binding model updates
- declaration/config/lock impacts
- migration sequencing
- verification checkpoints
```

- [ ] **Step 2: Placeholder scan**

Search the plan for the usual forbidden placeholder patterns from the writing-plans checklist.

```text
[placeholder markers]
[unfinished implementation notes]
[hand-wavy error-handling filler]
[cross-task shorthand instead of explicit instructions]
```

Expected: no matches.

- [ ] **Step 3: Consistency scan**

Manually verify the same names are used everywhere:

```text
agent.runtime
prompt.system
workflow.orchestration
tool.runtime
tool.shell
memory.store
channel.bridge
ext.mcp
ext.skills
```

If any alternate naming appears, fix it before execution starts.

## Plan Exit Criteria

This plan is ready for execution only when all of the following are true:

- `09-weft-claw-product-blueprint.md` is treated as the product-specific source of truth
- `02/03/04/05` are updated to match it
- `10-weft-claw-capability-package-matrix.md` exists and resolves naming ambiguity
- migration notes clearly sequence runtime evolution without breaking current behavior
- verification gates are explicit enough that later implementation cannot hide ambiguity behind “we’ll refine it later”
