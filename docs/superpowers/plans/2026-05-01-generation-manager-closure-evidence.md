# Product Generation Manager Closure Evidence (Tasks 0-12)

## Scope and posture

This artifact closes the explicit evidence-mapping gap for `docs/architecture/vNext/runtime/19-generation-manager.md` Tasks 0-12 and the end-to-end acceptance matrix.

It is intentionally concise and factual:

- based on high-level inspection of the current implementation surface and prior execution evidence
- based on known foreground sessions from 2026-04-30
- not a new design doc
- not a claim that unrun live GUI smoke passed

## Primary implementation surfaces inspected

- Core config and scene model: `core/src/app/config.rs`
- Core generation store/index/state model: `core/src/app/generation.rs`
- Core generation API/diff/diagnostics surface: `core/src/api/generations.rs`
- Package Server preview metadata models: `server/src/package_server/models.rs`
- Frontend dashboard: `clients/frontend/src/renderer/components/WeftCoreDashboard.vue`
- Deterministic GUI contract probe: `clients/frontend/scripts/probe-weft-generation-manager-contract.ts`

## Foreground execution trail and Oracle blocker context

Known foreground sessions referenced by the request:

| Session | Focus | Evidence summary |
|---|---|---|
| `ses_221444846ffemsdZx7LoaN4e5X` | clippy cleanup / release gate | Session report says full `cargo clippy --all-targets --all-features -- -D warnings` was cleared, with targeted Rust tests/build re-run. User prompt explicitly cites Oracle release-gate blocker context. |
| `ses_2212b254affelw6pM39aduvqVF` | GUI Task 12A | Session history shows initial partial frontend work, then stop-state reporting when `WeftCoreDashboard.vue` was temporarily incomplete. User prompt explicitly cites Oracle GUI evidence blocker context. |
| `ses_220e7efbeffeEo0xwEu3pGF5C0` | Package Server Task 11 preview enrichment | Session report says preview DTOs were enriched in `server/src/package_server/*`, focused package-server tests passed, and package server remained advisory/non-mutating. User prompt explicitly cites Oracle preview blocker context. |
| `ses_220c1c625ffelrJElPOzmFLOkM` | GUI smoke / dashboard probe evidence | Session report says live Electron probe still timed out, but deterministic fallback contract probe passed and wrote `clients/frontend/output/playwright/dashboard-probe/generation-manager-contract.json`. User prompt explicitly cites Oracle behavior-level GUI evidence blocker context. |

No separate Oracle session IDs were available in inspected context. Oracle findings are only present indirectly through the initiating user prompts above.

## Task 0-12 closure map

| Task | High-level implementing files | Verification / evidence |
|---|---|---|
| 0. Freeze architecture and tests-first helpers | `docs/architecture/vNext/runtime/19-generation-manager.md`; session evidence also references `core/tests/integration.rs` helper cleanup | Architecture doc is the controlling plan; clippy-cleanup session reports integration-test helper cleanup and preserved compiling tests. |
| 1. Scene schema load/save | `core/src/app/config.rs` | `AppSceneBindingPin`, `AppScenePackagePin`, scene metadata, and scene-aware config structures are present in inspected file; session evidence indicates config/generation Rust verification passed. |
| 2. Lock schema v2 metadata | `core/src/app/config.rs` | Current codebase inspection plus session evidence indicates generation/lock metadata additions landed and Rust checks passed; high-level mapping only here, not a full field-by-field lock diff audit. |
| 3. Canonical identity hashing | `core/src/api/generations.rs` imports `binding_set_id_from_lock_bindings`, `closure_digest_from_lock_packages`, `closure_id_from_lock_packages`, `scene_digest`; corresponding app-layer identity helpers are referenced from Core | Evidence is presence of canonical identity usage at API layer plus successful Rust verification from prior sessions. |
| 4. Generation summary expansion | `core/src/app/generation.rs`; `core/src/api/generations.rs` | Inspected `AppGeneration` now carries `scene`, `binding_set_id`, `closure_id`, `lock_digest`, `lock_path`, `parent_generation`, `created_by`, `validation_results`; API file exposes summary/diff snapshots. |
| 5. Persistent generation index | `core/src/app/generation.rs` | Inspected `AppGenerationIndex`, pointer/diagnostic structs, consistency report, startup diagnostics, and persistence-plan structures provide the generation-index/pointer model. |
| 6. Scene-aware candidate generation | `core/src/api/generations.rs`; Core app layer referenced by that API | API surface for generation lifecycle/diff/diagnostics exists; prior Rust verification sessions indicate this path compiled and tested. This artifact records existence/evidence, not full behavioral replay. |
| 7. Verification policy | `core/src/app/generation.rs`; `core/src/api/generations.rs` | `validation_results` are present on generation records; diagnostics and generation responses are present; prior sessions reported focused Rust tests and release-gate checks passed. |
| 8. Atomic activation and rollback | `core/src/app/generation.rs`; `core/src/api/generations.rs` | Inspected activation persistence step model includes lock write, previous pointer write, active pointer replace, root lock mirror replace, and generation index update; API file includes diagnostics and immutable-lock diff support. |
| 9. CLI lifecycle | `core/src/bin/weft.rs`; `core/Cargo.toml` bin registration | Fresh evidence: `cargo test -p weft-core --bin weft -- --nocapture` passed 22 CLI tests covering `weft gen current/list/diff/generate/verify/activate/rollback`, `weft scene create/bind/list/show`, help handling, and successful command-to-endpoint calls against a mock Core HTTP server; `cargo run --bin weft -- --help` exits successfully and prints the command surface. |
| 10. Store metadata verification | `core/src/app/generation.rs`; API diagnostics/diff surface in `core/src/api/generations.rs` | High-level evidence only. Generation/store diagnostics structures exist; release-gate session reported relevant Rust tests/build/clippy success. |
| 11. Package Server preview enrichment | `server/src/package_server/models.rs`; session evidence references `server/src/package_server/{artifacts,bindings,plans,tests}.rs` | Inspected model file shows enriched preview response shapes including `required_artifacts` / `verification_metadata`; package-server session reports replay-evidence placeholders added and focused package-server tests passed while keeping server non-mutating. |
| 12. GUI read-only, then guarded mutation | `clients/frontend/src/renderer/components/WeftCoreDashboard.vue`; `clients/frontend/scripts/probe-weft-generation-manager-contract.ts`; `clients/frontend/scripts/probe-weft-generation-manager-render.ts` | This implementation round intentionally satisfies the read-only inspection half: dashboard contains generation-manager snapshot wiring, read-only copy, and disabled guarded lifecycle controls. Fresh render evidence verifies the actual Vue component renders the panel and disabled controls. Live Electron smoke is explicitly **not** claimed as passing. |

## Acceptance matrix closure map

| Acceptance scenario | Implementing files / surface | Evidence / limitation |
|---|---|---|
| Legacy instance with only `config.toml` and `lock.toml` starts and imports active generation | `core/src/app/config.rs`, `core/src/app/generation.rs`, `core/src/main.rs` | Fresh full-suite evidence includes `cargo test` passing `tests::generation_store_restores_runtime_only_instance_lock_using_resolved_app_bindings`, `tests::generation_store_restores_from_explicit_instance_lock_path`, and integration coverage that loads weft-claw config/lock metadata. |
| Create scene from current | `core/src/app/config.rs`, scene API in `core/src/api/scenes.rs`, CLI scene surface in `core/src/bin/weft.rs` | Fresh evidence: POST scene API and CLI support for `weft scene create <app> <scene>` are implemented. `cargo test -p weft-core api::scenes -- --nocapture` verifies scene file creation, and `cargo test -p weft-core --bin weft -- --nocapture` verifies parser and mock-Core endpoint calls. |
| Bind package in scene | `core/src/app/config.rs` scene pin structs; `core/src/api/scenes.rs`, `core/src/api/generations.rs` scene pin application | Fresh evidence: POST scene bind API and CLI support for `weft scene bind <app> <scene>` are implemented. `cargo test -p weft-core api::scenes -- --nocapture` verifies `active_scene` persistence, and `cargo test -p weft-core --bin weft -- --nocapture` verifies parser and mock-Core endpoint calls; generation APIs continue to validate scene pins/conflicts. |
| Generate scene | `core/src/api/generations.rs`; `core/src/bin/weft.rs` `gen generate` | Fresh evidence: `cargo test test_generation_api_full_lifecycle -- --nocapture` passed; CLI parser tests cover `weft gen generate`; CLI command mapping posts to `/api/apps/{app}/propose`. |
| Verify candidate with full digests in safe profile | Core generation/verification layer | Fresh evidence: `cargo test test_generation_api_full_lifecycle -- --nocapture`, `cargo test test_weft_claw_full_generation_lifecycle_via_api -- --nocapture`, and full `cargo test` passed. |
| Verify candidate missing digest in safe profile | Core verification policy | Fresh evidence: `cargo test verify_generation_rejects_safe_candidate_when_selected_package_digest_metadata_missing -- --nocapture` passed. |
| Verify candidate missing digest in developer profile | Core verification policy | Fresh evidence: `cargo test verify_generation_allows_developer_candidate_with_dev_unsealed_warning -- --nocapture` passed. |
| Activate verified generation | `core/src/app/generation.rs` activation persistence model; `core/src/api/generations.rs` | Fresh evidence: `cargo test test_generation_api_full_lifecycle -- --nocapture` and `cargo test test_weft_claw_full_generation_lifecycle_via_api -- --nocapture` passed. |
| Activate unverified generation | Core verification/activation layer | Fresh evidence: `cargo test test_generation_store_activate_requires_verified -- --nocapture` passed. |
| Rollback | `core/src/app/generation.rs`; `core/src/api/generations.rs`; dashboard read-only surface also shows rollback slot | Fresh evidence: `cargo test test_generation_store_rollback_restores_previous -- --nocapture` and `cargo test test_weft_claw_full_generation_lifecycle_via_api -- --nocapture` passed. |
| Package Server offline with active generation | Core generation/store model | Fresh evidence is indirect but behavioral: lifecycle and restore tests pass without requiring Package Server; full `cargo test` also exercises local package index/cache paths. No separate live outage process was started. |
| Package Server offline for new unresolved package | Core/package-server boundary | Fresh evidence remains bounded to tests that reject metadata-only/unresolved required sources and package-server preview boundary tests; no separate live outage process was started. |
| Corrupt generation index but valid pointers/locks | `core/src/app/generation.rs` consistency/repair diagnostics | Fresh evidence: full `cargo test` includes generation index consistency/repair tests; targeted diagnostics test below verifies pointer mismatch handling. |
| Active pointer targets missing lock | `core/src/app/generation.rs`; `core/src/api/generations.rs` diagnostics surface | Fresh evidence: `cargo test generation_diagnostics_reports_pointer_mismatch_from_instance_files -- --nocapture` passed; full suite includes repair/diagnostics coverage. |

## GUI evidence note

The GUI evidence is closure-relevant but limited:

- `clients/frontend/src/renderer/components/WeftCoreDashboard.vue` contains the Generation Manager read-only pane and guarded lifecycle controls.
- `clients/frontend/scripts/probe-weft-generation-manager-contract.ts` is present and passed, writing `clients/frontend/output/playwright/dashboard-probe/generation-manager-contract.json`.
- `clients/frontend/scripts/probe-weft-generation-manager-render.ts` renders the actual Vue dashboard component with Pinia state and passed, writing `clients/frontend/output/playwright/dashboard-probe/generation-manager-render.json`.
- Live Electron automation still timed out in this environment.

Therefore this closure artifact records **deterministic renderer-contract evidence plus actual Vue SSR render evidence**, not a claim that live Electron smoke passed.

## Verification commands to keep with closure evidence

These are the final verification commands referenced by the request and prior execution trail:

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
npm run typecheck:ui
npm run build
npx tsx scripts/probe-weft-generation-manager-contract.ts
cargo test package_server::tests -- --nocapture
cargo test -p weft-core --bin weft -- --nocapture
cargo test -p weft-core api::scenes -- --nocapture
cargo test test_generation_api_full_lifecycle -- --nocapture
cargo test test_weft_claw_full_generation_lifecycle_via_api -- --nocapture
cargo test test_generation_store_full_lifecycle -- --nocapture
cargo test test_generation_store_rollback_restores_previous -- --nocapture
cargo test test_generation_store_activate_requires_verified -- --nocapture
cargo test verify_generation_rejects_safe_candidate_when_selected_package_digest_metadata_missing -- --nocapture
cargo test verify_generation_allows_developer_candidate_with_dev_unsealed_warning -- --nocapture
cargo test activate_existing_generation_blocks_safe_profile_when_verified_store_object_is_missing -- --nocapture
cargo test generation_diagnostics_reports_pointer_mismatch_from_instance_files -- --nocapture
cargo run --bin weft -- --help
npx tsx scripts/probe-weft-generation-manager-render.ts
```

## Net closure statement

The explicit closure gap was not absence of implementation alone; it was absence of a concise evidence map. Based on the inspected implementation surfaces and the four known foreground sessions, Tasks 0-12 and the acceptance matrix now have a factual closure index pointing to:

- Core config / generation / API files for Tasks 1-10
- Package Server preview model and prior package-server verification for Task 11
- Frontend dashboard plus deterministic contract probe for Task 12

Remaining limitations are recorded above rather than hidden: this artifact does **not** claim a fresh live Electron smoke pass. Scene CLI/API mutation support now includes `scene create/bind` in addition to list/show, and the generation lifecycle itself is directly covered by fresh targeted Rust lifecycle, policy, activation, rollback, diagnostics, CLI parser, and mock-Core command-to-endpoint evidence.
