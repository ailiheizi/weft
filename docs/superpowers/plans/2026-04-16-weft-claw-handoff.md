# WEFT-plug weft-claw Migration Handoff

## Current status

This workspace is mid-migration from legacy product naming to `weft-claw`, with the bigger architectural shift from the old app-centric model to **Product Package + Instance** already substantially underway.

The following is already true in the current repo state:

- Canonical product package moved to `packages/weft-claw/package.toml`
- Canonical instance moved to `.weft/weft-claw/config.toml` and `.weft/weft-claw/lock.toml`
- Core/server runtime and HTTP responses now expose `weft-claw` as the product name
- product listing and product detail HTTP responses were already observed returning:
  - `name = "weft-claw"`
  - `display_name = "Weft Claw"`
  - `source_authority = "product-package-instance"`
  - manifest/config/lock paths under `packages/weft-claw` and `.weft/weft-claw`
- `weft-claw-ui` now resolves in runtime metadata and the product detail response returns non-null `ui`
- Installed-first runtime selection for `companion-core` is working
- Companion natural-language flow now reaches delegate mode and can execute at least `fs_read`

## What has been completed

### Architecture migration

- Core loader semantics were changed toward package/instance-first
- Legacy source-declaration fallback was removed from `core/src/app/config.rs` path selection helpers
- `product_package_declaration_path()` no longer falls back to legacy declaration filenames
- `instance_config_path()` / `instance_lock_path()` no longer fall back to legacy instance filenames
- Activation/generation restore logic was repaired to rebuild active generation from `ResolvedApp` instead of runtime-only instance lock bindings/packages

### Runtime/package registry

- Runtime plugin precedence was changed so installed runtime packages can override official ones in runtime selection
- `companion-core` installed service is the active runtime service now
- `weft-claw-*` feature packages are now discovered in runtime startup logs
- `weft-claw-ui` was added as a proper embedded provider package under `packages/installed/weft-claw/`

### Rename work already done

- Product package name converged to `weft-claw`
- Product display name converged to `Weft Claw`
- Instance directory converged to `.weft/weft-claw`
- Feature names began migration:
- legacy product-specific feature names converged to `weft-claw-*`
- Official feature package directories already renamed:
  - `packages/official/weft-claw-chat-feature`
  - `packages/official/weft-claw-extensions-feature`
  - `packages/official/weft-claw-channel-feature`
  - `packages/official/weft-claw-automation-feature`
  - `packages/official/weft-claw-devteam-feature`
- Installed UI asset directory already renamed:
  - `packages/installed/weft-claw/ui/*`

### Tests / builds already observed passing

These passed at least once during this session after relevant fixes:

- `cargo build -p weft-core`
- `cargo build -p registry-server`
- `cargo test -p weft-core get_app_returns_ui_metadata -- --nocapture`
- `cargo test -p weft-core app_ui_index_alias_serves_existing_weft_claw_asset -- --nocapture`
- `cargo test -p weft-core test_activate_generation_creates_lock_from_manifest_config_and_bindings -- --nocapture`
- `cargo test -p registry-server app_source_route_returns_authority_metadata_for_existing_app -- --nocapture`
- `cargo test -p registry-server app_source_route_prefers_product_and_instance_paths -- --nocapture`

### Real HTTP evidence already observed

Observed from actual running `weft-core`:

- `GET /api/health` returned `{"status":"ok","version":"0.1.0"}`
- `GET /api/apps` returned `weft-claw` with:
  - `display_name = "Weft Claw"`
  - `status = "Resolved"`
  - `ui.provider = "weft-claw-ui"`
  - `source_index.manifest_path = ...packages/weft-claw/package.toml`
  - `source_index.config_path = .../.weft/weft-claw/config.toml`
  - `source_index.lock_path = .../.weft/weft-claw/lock.toml`
- the product detail endpoint returned matching `ui` metadata

## Important remaining problems

This migration is **not finished**. The main remaining issues are now consistency / cleanup issues, not fundamental architecture blockers.

### 1. Old legacy naming still remains outside reference/

There are still many non-reference hits in:

- `core/src/*`
- `server/src/*`
- `core/tests/integration.rs`
- `docs/architecture/vNext/*.md`
- `config/config.toml`
- `packages/index.toml`

The user explicitly changed scope: **do not preserve the old source-declaration concept**. It should be removed, not kept as compatibility fallback.

### 2. `packages/index.toml` and runtime/source registry cleanup is incomplete

Current runtime now discovers `weft-claw-*` feature packages, but there may still be stale old-name references or mixed source entries in code and docs. Re-scan after each batch.

### 3. Agentic natural-language chain is only partially proven

Status of the chain:

- Proven working:
  - direct app runtime tool calls (`tool.files`, `tool.shell`, `tool.web`, `workflow.orchestration`)
  - companion runtime HTTP entry
  - companion delegate mode
  - companion service installed-first selection
- Not yet fully proven end-to-end:
  - **natural language -> AI chooses tools -> writes code file -> reads/runs it automatically** as a complete autonomous chain

At one point the companion response was:

- `kind = "delegate"`
- `tool_results` included only `fs_read`
- no generated output file was written

So the final requested agentic proof is still incomplete.

## Highest-priority files to continue with

### Core/runtime

- `core/src/app/config.rs`
- `core/src/main.rs`
- `core/src/api/apps.rs`
- `core/src/api/app_detail.rs`
- `core/src/api/generations.rs`
- `core/src/plugin/mod.rs`

### Package / instance / source registry

- `packages/index.toml`
- `packages/weft-claw/package.toml`
- `.weft/weft-claw/config.toml`
- `.weft/weft-claw/lock.toml`
- `packages/installed/weft-claw/plugin.toml`
- `packages/installed/weft-claw/ui/*`

### Feature packages

- `packages/official/weft-claw-chat-feature/*`
- `packages/official/weft-claw-extensions-feature/*`
- `packages/official/weft-claw-channel-feature/*`
- `packages/official/weft-claw-automation-feature/*`
- `packages/official/weft-claw-devteam-feature/*`

### Tests

- `core/tests/integration.rs`
- `core/src/api/app_detail.rs` tests
- `server/src/main.rs` tests

### Docs

- `docs/architecture/vNext/00-overview.md`
- `docs/architecture/vNext/02-app-model.md`
- `docs/architecture/vNext/03-package-model.md`
- `docs/architecture/vNext/04-declaration-config-lock.md`
- `docs/architecture/vNext/05-binding-and-resolution.md`
- `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
- `docs/architecture/vNext/10-weft-claw-capability-package-matrix.md`
- `docs/architecture/vNext/11-weft-claw-devteam-feature-design.md`
- `docs/architecture/vNext/12-weft-claw-devteam-capability-package-matrix.md`
- `docs/architecture/vNext/13-product-package-instance-model.md`

## Recommended execution order for the next loop

1. Remove remaining runtime and code references to legacy source-authority strings outside reference/
2. Finish renaming all product-specific feature/ui package IDs to `weft-claw-*`
3. Finish `core/tests/integration.rs` bulk migration
4. Finish vNext document rename sweep
5. Re-run focused builds/tests in small batches
6. Re-run real HTTP verification
7. Re-run full natural-language companion/agent chain proof

## Suggested verification checklist for the next worker

- `cargo build -p weft-core`
- `cargo build -p registry-server`
- focused weft-core tests around app detail / generation activation / integration fixtures
- focused registry-server route tests
- real HTTP:
  - `GET /api/health`
  - `GET /api/apps`
- product detail response
- product UI asset response
- real agentic request:
  - natural language prompt requesting code creation + tool use
  - prove actual file write happened
  - prove optional script execution happened

## Important scope rule from user

The user explicitly changed requirements during this session:

- **Do not preserve old app-centric source wording**
- keep product-facing naming aligned to `weft-claw`
- rename exposed package names too
- keep generic reusable packages unchanged
- product-specific feature/ui packages should become `weft-claw-*`
- references under `reference/` are not the main migration target
