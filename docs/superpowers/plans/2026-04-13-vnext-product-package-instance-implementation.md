# WEFT vNext Product Package + Instance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the source-level App concept from the vNext model, replace it with Product Package + Instance semantics, and stage the repository migration without breaking declaration/config/lock, binding, or generation workflows.

**Architecture:** First make the documentation model internally consistent by introducing one new blueprint as the canonical convergence target. Then rewrite the old App-centric docs to point to Product Package + Instance semantics. Only after the docs agree should code and API migration begin, with compatibility preserved only as migration history, not as the current product model.

**Tech Stack:** Markdown architecture docs, TOML package/config/lock files, Rust core resolver and app/config/lock model, server source authority APIs.

---

## File Structure and Responsibilities

- `docs/architecture/vNext/13-product-package-instance-model.md`
  - New canonical convergence blueprint; defines Product Package + Instance.
- `docs/architecture/vNext/00-overview.md`
  - Must stop presenting App as the final top-level source model.
- `docs/architecture/vNext/02-app-model.md`
  - Must be deprecated, redirected, or rewritten because its core premise changes.
- `docs/architecture/vNext/03-package-model.md`
  - Must extend package taxonomy to include `product`.
- `docs/architecture/vNext/04-declaration-config-lock.md`
  - Must move declaration ownership to product package and config/lock ownership to instance.
- `docs/architecture/vNext/05-binding-and-resolution.md`
  - Must rewrite resolve inputs from App triple to Product Package + Instance inputs.
- `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
  - Must be checked for App-shell assumptions that should move to product package language.
- `docs/superpowers/plans/2026-04-13-vnext-product-package-instance-implementation.md`
  - This implementation plan; source of execution order.
- `server/src/main.rs`
  - Current app/package source authority split; later migration target.
- `packages/weft-claw/package.toml`
  - Canonical product package declaration input.
- `.weft/weft-claw/config.toml`
  - Canonical instance config input.
- `.weft/weft-claw/lock.toml`
  - Canonical instance lock input.

## Implementation Phases

1. **Blueprint convergence** — add the new blueprint and align old docs around it.
2. **Terminology and schema convergence** — rewrite declaration/config/lock and binding docs.
3. **Repository migration plan** — define code/API/file-path migration slices with compatibility rules.
4. **Runtime execution migration** — move actual loader and API behavior from App-centric to Product Package + Instance.

### Task 1: Add the new convergence blueprint

**Files:**
- Create: `docs/architecture/vNext/13-product-package-instance-model.md`
- Reference: `docs/architecture/vNext/00-overview.md`
- Reference: `docs/architecture/vNext/02-app-model.md`
- Reference: `docs/architecture/vNext/03-package-model.md`
- Reference: `docs/architecture/vNext/04-declaration-config-lock.md`
- Reference: `docs/architecture/vNext/05-binding-and-resolution.md`

- [ ] **Step 1: Draft the new blueprint as the canonical convergence target**

Write a new architecture doc that explicitly defines:

```text
- source-level App is removed
- Package is the only source-level published object
- package taxonomy becomes provider/foundation/feature/product
- instance owns config/lock/generation state
```

- [ ] **Step 2: Ensure the new blueprint explicitly states what is preserved**

The blueprint must preserve these existing semantics:

```text
- capability registry remains Core-owned
- resolve/verify/activate/rollback remain intact
- lock still represents an assembled runtime result
- feature composition remains a first-class resolve input
```

- [ ] **Step 3: Add migration-oriented sections instead of only conceptual sections**

The new file must include these sections:

```text
- relation to existing docs
- migration principles
- source authority convergence
- declaration/config/lock ownership changes
```

- [ ] **Step 4: Link this blueprint from the vNext overview index**

Modify the overview index so the new file appears in reading order after the current `12-...` document.

Expected result:

```text
13. `13-product-package-instance-model.md` — Product Package + Instance 收敛模型
```

### Task 2: Rewrite overview language so App is no longer the final source model

**Files:**
- Modify: `docs/architecture/vNext/00-overview.md`
- Reference: `docs/architecture/vNext/13-product-package-instance-model.md`

- [ ] **Step 1: Find and replace top-level claims that say App is the final source object**

Search and rewrite statements like:

```text
- App 仍然是唯一面向用户的产品单位
- 用户看到和操作的是 App
- App / Package / Lock / Generation 主模型
```

Replace them with Product Package + Instance language while preserving the idea that users still interact with a stable product.

- [ ] **Step 2: Update the core objects section**

The overview should describe these top-level objects instead:

```text
- Core
- Package
- Instance
- Lock
- Generation
```

- [ ] **Step 3: Update the document index**

Insert the new `13-product-package-instance-model.md` into the ordered document list and mention that it captures the convergence away from the App source model.

### Task 3: Rewrite the package model to include product packages

**Files:**
- Modify: `docs/architecture/vNext/03-package-model.md`
- Reference: `docs/architecture/vNext/13-product-package-instance-model.md`

- [ ] **Step 1: Extend the package taxonomy from three kinds to four kinds**

Current taxonomy:

```text
- provider
- foundation
- feature
```

Target taxonomy:

```text
- provider
- foundation
- feature
- product
```

- [ ] **Step 2: Add a new section defining Product Package**

This section must describe:

```text
- product identity
- default feature enablement
- default binding policy
- validation/upgrade policy
- explicit non-ownership of user config and runtime lock
```

- [ ] **Step 3: Remove language that implies packages are only internal and never user-facing**

Rewrite any wording that makes `package` inherently invisible to users. The new model still hides most internal packages, but product package is now the top-level released product object.

### Task 4: Rewrite declaration/config/lock ownership

**Files:**
- Modify: `docs/architecture/vNext/04-declaration-config-lock.md`
- Reference: `docs/architecture/vNext/13-product-package-instance-model.md`

- [ ] **Step 1: Move declaration ownership from App to Product Package**

Rewrite the document so declaration answers:

```text
这个 product package 默认想装成什么样？
```

instead of:

```text
这个 App 想成为一个什么产品？
```

- [ ] **Step 2: Move config ownership from App to Instance**

Update the config section so it clearly states config belongs to runtime instance state, not to the package source tree.

- [ ] **Step 3: Move lock ownership from App to Instance**

Update the lock section so it clearly states lock records the assembled runtime instance result and should not be treated as source-package metadata.

- [ ] **Step 4: Replace the old file structure example**

Replace this pattern:

```text
packages/
  weft-claw/
    package.toml
```

with this pattern:

```text
packages/
  weft-claw/
    package.toml

  %AppData%/WEFT/instances/weft-claw/
  config.toml
  lock.toml
```

and optionally the project-local variant.

### Task 5: Rewrite binding and resolution inputs

**Files:**
- Modify: `docs/architecture/vNext/05-binding-and-resolution.md`
- Reference: `docs/architecture/vNext/13-product-package-instance-model.md`

- [ ] **Step 1: Replace App-centric resolve inputs**

Rewrite resolve inputs from:

```text
- App declaration
- App config
- App lock
```

to:

```text
- product package declaration
- instance config
- instance lock
```

- [ ] **Step 2: Keep feature composition semantics intact**

Ensure the document still clearly says:

```text
- enabled feature packages expand the candidate graph
- product-level defaults are applied before instance-level overrides
- runtime override must still respect mutable/fixed constraints
```

- [ ] **Step 3: Add a note on source authority convergence**

Add a short section stating that package sources remain publishable artifacts, while instance data remains local runtime state.

### Task 6: Convert the Weft Claw blueprint from legacy shell language to Product Package language

**Files:**
- Modify: `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
- Reference: `docs/architecture/vNext/13-product-package-instance-model.md`

- [ ] **Step 1: Identify all sections where old product-shell wording is acting as a source artifact**

Mark these sections for rewrite:

```text
- app shell
- weft-claw product package
- app declaration/config/lock examples
```

- [ ] **Step 2: Rewrite those sections so `weft-claw` becomes a product package**

Keep the existing four-layer composition idea, but rename the top source object from App shell to Product Package where appropriate.

- [ ] **Step 3: Preserve runtime-instance semantics**

Where the document talks about config, lock, and generation, rewrite it to point at instance state rather than at source-level app files.

### Task 7: Define code and API migration slices before code changes begin

**Files:**
- Modify: `docs/superpowers/plans/2026-04-13-vnext-product-package-instance-implementation.md`
- Reference: `server/src/main.rs`
- Reference: `packages/weft-claw/package.toml`
- Reference: `.weft/weft-claw/config.toml`
- Reference: `.weft/weft-claw/lock.toml`

- [ ] **Step 1: Add a repository migration section**

This section must list the first code migration targets:

```text
- product package declaration loader
- instance config/lock loader
- server product source index
- package taxonomy parsing
- app-focused API routes
```

- [ ] **Step 2: Add compatibility constraints**

The plan must explicitly preserve these temporary guarantees:

```text
- migration notes may mention old app declaration paths during transition
- instance config/lock can be introduced before old files are removed
- package source is canonical, old app source wording is historical only
```

- [ ] **Step 3: Add verification gates**

The plan must require verification after each migration slice:

```text
- architecture docs stay internally consistent
- no schema boundary contradiction remains in 00/03/04/05/09/13
- runtime compatibility path is still documented before code removal
```

### Task 8: Final review and execution handoff

**Files:**
- Review: `docs/architecture/vNext/00-overview.md`
- Review: `docs/architecture/vNext/03-package-model.md`
- Review: `docs/architecture/vNext/04-declaration-config-lock.md`
- Review: `docs/architecture/vNext/05-binding-and-resolution.md`
- Review: `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
- Review: `docs/architecture/vNext/13-product-package-instance-model.md`
- Review: `docs/superpowers/plans/2026-04-13-vnext-product-package-instance-implementation.md`

- [ ] **Step 1: Check for stale “App is the source object” wording**

Search for stale phrases such as:

```text
- App 是唯一用户入口
- App declaration/config/lock 是最终模型
- Package 只是内部实现，永远不对用户暴露
```

- [ ] **Step 2: Check that new terminology is internally consistent**

The final doc set should consistently distinguish:

```text
- product package
- feature package
- provider package
- instance
- generation
```

- [ ] **Step 3: Hand off execution ordering**

When documentation is fully aligned, implementation should proceed in this order:

```text
1. package taxonomy parsing
2. declaration/config/lock loader compatibility
3. server source authority convergence
4. config/lock instance-path adoption
5. old app-path removal
```
