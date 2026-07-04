# WEFT vNext Proposal: Self-hosted Package Server Implementation Plan

## 1. Purpose

This document turns the self-hosted WEFT Package Server blueprint from `18-self-hosted-package-server-blueprint.md` into a phased implementation plan, while preserving the Package Service boundary defined in `16-package-service.md`.

It does not redefine the vNext architecture. It organizes implementation work around concrete milestones while preserving the existing boundary:

- the Package Server owns package catalog truth, package metadata, candidate discovery support, dependency tree parsing, artifact metadata, artifact supply, and later ingestion
- Core owns product package declaration reading, instance config reading, feature composition, final resolve, lock generation, generation lifecycle, verify, activate, rollback, and runtime truth
- already locked and locally materialized generations must remain replayable and roll-backable without live Package Server availability

This plan is intentionally server-first and implementation-oriented, but it is not a Rust implementation task.

## 2. Normative inputs

This plan is based on the following documents:

- `16-package-service.md`
- `18-self-hosted-package-server-blueprint.md`
- `04-declaration-config-lock.md`
- `05-binding-and-resolution.md`
- `06-profiles-and-policy.md`
- `../foundations/03-package-model.md`
- `../foundations/13-product-package-instance-model.md`
- `../00-overview.md`

If this plan conflicts with those documents, those documents win.

## 3. Implementation principles

### 3.1 Keep the MVP small

The first usable milestone must prove the server/Core boundary with the smallest possible workflow:

1. package detail API
2. bind preview API
3. install plan API

Upload, artifact storage, artifact download, broad update orchestration, AI package generation, and GitHub ingestion come later.

### 3.2 Preserve server/Core ownership

The Package Server may answer what exists, what could satisfy a request, what dependencies are implied, and what artifacts are available.

Core still decides what is legal for a concrete instance.

The Package Server must not become:

- the owner of final resolve
- the owner of instance lock semantics
- the owner of generation activation
- a runtime control plane
- a replacement for declaration, config, lock, profile, or policy

### 3.3 Keep self-hosting simple

The first server should be easy to run locally or in a small team environment. It can start with a local package index and local metadata files behind an API boundary. The important milestone is the contract, not the backing store.

### 3.4 Design for later artifact and ingestion work

Even before artifact storage exists, API responses should carry stable package identity, source identity, version, digest fields, manifest digest fields, implementation form, and dependency metadata where known. That prevents a later artifact phase from changing the Core-facing contract.

### 3.5 Default latest, then immutable lock

Package selection may default to the latest compatible version when no lock exists or when an explicit update is being planned. That floating choice must end before activation: Core must lock exact package, artifact, source, digest, manifest digest, and binding identities before a generation can become active.

### 3.6 Keep Package Server code modular

The Package Server implementation should not keep growing inside `server/src/main.rs`.

Current acceptable transitional state:

```text
server/src/main.rs
server/src/package_server/mod.rs
```

Before implementing upload, application, review, approval, catalog mutation, artifact storage, or publishing, split `package_server/mod.rs` into focused modules:

```text
package_server/routes.rs
package_server/models.rs
package_server/state.rs
package_server/manifest.rs
package_server/catalog.rs
package_server/bindings.rs
package_server/plans.rs
package_server/artifacts.rs
package_server/drafts.rs
package_server/ingestion.rs
```

This keeps the self-hosted Package Server separate from legacy Gitea plugin/suite handling and prevents `package_server/mod.rs` from becoming a second monolith.

## 4. Target final expected UX

This plan does not design UI screens. The target user experience is only described to anchor the implementation milestones.

Expected end state:

1. A user points WEFT Core at a self-hosted Package Server.
2. Core can inspect a product package and ask the server for package details.
3. Core can ask for a bind preview before committing to any generation.
4. Core can ask for an install or update plan, then perform final resolve, verification, lock writing, and generation activation locally.
5. When artifact support is added, Core can fetch exact lock-pinned artifacts and replay local materialized generations without the server.
6. When AI package generation is added, generated packages and binding changes can be reviewed as candidate metadata before Core accepts a lock.
7. When GitHub ingestion is added, repositories can feed the server catalog without changing Core runtime semantics.

## 5. Phased implementation plan

Each phase must leave the system in a reviewable state. Do not start a later phase by weakening boundaries from an earlier one.

### Phase 0: Boundary contract and vocabulary freeze

Goal: create the written server/Core contract before implementation work starts.

Deliverables:

- Define Package Server request and response shapes for package detail, dependency tree, bind preview, install plan, update plan, artifact metadata, artifact download, verification metadata, AI package draft, AI bind draft, and GitHub ingestion.
- Define shared vocabulary for package identity, package kind, implementation form, capability, binding request, dependency edge, dependency tree, candidate closure, artifact identity, and verification metadata.
- Define which fields are advisory server facts and which fields are Core-owned runtime facts.
- Define the local cache contract needed later for lock-pinned artifact replay.

Dependencies:

- `16-package-service.md` for server responsibility.
- `04-declaration-config-lock.md` for lock semantics.
- `05-binding-and-resolution.md` for binding and resolve ownership.
- `06-profiles-and-policy.md` for Core policy authority.

Acceptance criteria:

- A reader can tell which component owns each lifecycle decision.
- The contract explicitly says bind preview and install plan are not final resolve.
- The MVP order is documented as package detail, bind preview, install plan.
- Artifact upload and download are marked as post-MVP.

Verification strategy:

- Architecture review against `16-package-service.md`.
- Checklist review that every endpoint returns candidates, metadata, or plans, not final runtime truth.
- Boundary review that Core remains the only lock writer.

### Phase 0.5: Package Server module split

Goal: establish the server code structure before adding stateful package management workflows.

Deliverables:

- Keep process startup, health, CORS, and legacy Gitea plugin/suite routes in `server/src/main.rs`.
- Keep Package Server routes under `server/src/package_server/`.
- Split Package Server code into focused modules for routes, models, state, manifest parsing, catalog/search, bindings/dependency preview, plans, artifacts/verification, AI drafts, and ingestion.
- Preserve all existing endpoint paths and response shapes during the split.

Dependencies:

- Phase 0 boundary vocabulary.
- Existing `server/src/package_server/mod.rs` transitional module.

Acceptance criteria:

- `main.rs` no longer owns Package Server DTOs, handlers, or helpers.
- `package_server/mod.rs` is not a large catch-all implementation file.
- Existing Package Server API tests still pass without route path changes.
- Legacy Gitea plugin/suite routes remain available but are visibly separate from the Package Server.

Verification strategy:

- Route compatibility tests for existing package, catalog, plan, draft, and ingestion endpoints.
- Manual QA against representative Package Server endpoints after the split.
- Compile check for the server crate.

Risks:

- Risk: route paths change during refactor.
  Mitigation: keep router tests path-based and compare existing response fields.
- Risk: shared cache/state visibility leaks too broadly.
  Mitigation: introduce package-server state access helpers instead of exposing unrelated legacy cache details.

Risks:

- Risk: server responses accidentally look like final locks.
  Mitigation: name server outputs as preview, plan, candidate, or metadata, never active generation.
- Risk: the contract starts too broad.
  Mitigation: keep Phase 1 through Phase 4 as the only MVP phases.

### Phase 1: Package detail API MVP

Goal: make the Package Server the read surface for package metadata without changing Core resolve.

Deliverables:

- `GET package detail` or equivalent API for exact package identity and version.
- Package detail response with package kind, implementation form, capabilities provided, requirements declared, dependencies declared, source metadata, provenance fields, digest fields if known, manifest digest fields if known, compatibility metadata, and verification metadata if known.
- Compatibility adapter for current package index data that lacks newer taxonomy fields.
- Error model for unknown package, ambiguous package, unsupported schema version, and incomplete metadata.

Dependencies:

- Phase 0 contract.
- Existing package model fields from `../foundations/03-package-model.md`.
- Current package index or metadata source as the first backing store.

Acceptance criteria:

- Core or a CLI caller can request package detail for a known package.
- The response contains enough metadata for Core to keep doing local resolve with server-supplied package facts.
- Missing artifact data does not block this phase.
- The server does not read or write instance config, lock, active generation, or rollback state.

Verification strategy:

- Contract tests for package detail response shape.
- Fixture tests for provider, foundation, feature, and product package metadata.
- Compatibility tests for older metadata with missing taxonomy fields.
- Negative tests for unknown package and malformed metadata.

Risks:

- Risk: current metadata is incomplete.
  Mitigation: return explicit incomplete fields and compatibility defaults rather than inventing runtime truth.
- Risk: package detail grows into install behavior.
  Mitigation: keep Phase 1 read-only.

### Phase 2: Dependency tree parser

Goal: give the server authoritative package catalog knowledge for declared dependency structure, while leaving feature composition and final legality in Core.

Deliverables:

- Parser for declared package dependencies from package metadata.
- Dependency tree API for a package identity and optional version constraint.
- Response format that separates direct dependencies, transitive dependencies, dependency edges, version constraints, compatibility notes, and unresolved edges.
- Cycle detection and clear diagnostics for missing or incompatible dependency edges.
- Parser fixtures for product, feature, foundation, and provider packages.

Dependencies:

- Phase 1 package detail API.
- Package dependency fields from current metadata.

Acceptance criteria:

- The server can return a dependency tree for a known package using catalog metadata only.
- The response distinguishes declared dependency tree from Core's candidate runtime graph.
- Cycles, missing packages, and unsupported constraints fail clearly.
- Core can consume dependency tree output as an input, but still performs final resolve.

Verification strategy:

- Unit tests for dependency parsing rules.
- Fixture tests for normal trees, shared dependencies, cycles, missing dependencies, and version conflicts.
- Contract tests proving the dependency tree response is not a lock.

Risks:

- Risk: dependency tree parser duplicates Core feature composition.
  Mitigation: server parses declared package dependencies only. Core still expands enabled features and applies profile, policy, and overrides.
- Risk: version constraints are underspecified.
  Mitigation: return unresolved or ambiguous edges explicitly until constraint semantics are stable.

### Phase 3: Bind preview API MVP

Goal: let Core ask the server for binding candidates and explanations before Core performs final resolve.

Deliverables:

- Bind preview API that accepts required capabilities, enabled feature hints, product default binding hints, optional runtime override hints, and optional profile label.
- Candidate response grouped by capability, with provider package candidates, compatibility metadata, declared requirements, dependency summary, source metadata, digest metadata if known, and reason codes.
- Explicit fields for fixed, mutable, and conditional mutable hints when supplied by Core.
- Rejection and warning diagnostics for missing providers, incompatible candidates, and unsupported override requests.

Dependencies:

- Phase 1 package detail API.
- Phase 2 dependency tree parser.
- Binding rules from `05-binding-and-resolution.md`.

Acceptance criteria:

- Core can preview candidate providers for a capability without committing to a lock.
- The preview explains why candidates were included, excluded, or warned.
- The response does not claim a final selected provider unless it is clearly marked as an advisory default candidate.
- Core remains responsible for product defaults, feature-level composition, runtime override legality, profile, policy, and final binding selection.

Verification strategy:

- Contract tests for bind preview request and response shape.
- Fixture tests for fixed, mutable, and conditional mutable cases.
- Tests that Safe, Developer, and Trusted labels only influence server-side hints when provided, while final policy enforcement stays in Core.
- Review checklist confirming no lock, generation, activation, or rollback field appears in the response.

Risks:

- Risk: bind preview becomes server-side resolve.
  Mitigation: response names must use candidates, warnings, and explanations. Core-only fields stay out.
- Risk: profile labels are treated as policy enforcement.
  Mitigation: server can annotate compatibility, but Core policy remains authoritative.

### Phase 4: Install plan API MVP

Goal: provide a concrete install plan that Core can use as input to final resolve and lock generation.

Deliverables:

- Install plan API that accepts product package identity, target version or range, requested features, runtime override hints, and current local package context when available.
- Plan response with package detail references, dependency tree references, candidate closure, bind preview summary, missing metadata, artifact requirements by identity, verification metadata needed later, and human-readable reasons.
- Plan status values such as ready_for_core_resolve, needs_user_choice, missing_metadata, incompatible_request, and blocked_by_policy_hint.
- No artifact upload, artifact download, or artifact persistence requirement in this phase.

Dependencies:

- Phase 1 package detail API.
- Phase 2 dependency tree parser.
- Phase 3 bind preview API.

Acceptance criteria:

- Core can request an install plan for one product package and receive all package-side facts needed to attempt final resolve.
- The plan includes exact package identities where known and unresolved choices where not known.
- Core still creates the candidate lock after final resolve and verification.
- This phase proves the MVP without implementing artifact storage.

Verification strategy:

- Golden fixture tests for a simple product package install plan.
- Tests for missing provider, optional feature disabled, optional feature enabled, and mutable binding override.
- Boundary tests that install plan output cannot be activated directly.
- Manual review against `04-declaration-config-lock.md` to confirm the lock remains Core-owned.

Risks:

- Risk: install plan is mistaken for lock.
  Mitigation: avoid lock-shaped response fields such as active generation id, verification result, or final resolved binding snapshot.
- Risk: MVP expands into artifact store work.
  Mitigation: represent artifact requirements as metadata only until Phase 6.

### Phase 5: Update plan API

Goal: add update planning support while keeping update acceptance in Core.

Deliverables:

- Update plan API that accepts current lock summary or current package identity set, requested update target, and optional channel or compatibility hints.
- Available version lookup.
- Dependency-aware update candidate calculation.
- Plan response with changed packages, unchanged packages, new candidates, removed candidates, compatibility notes, verification metadata changes, and artifact requirements.
- Clear distinction between update candidate and accepted generation.

Dependencies:

- Phase 4 install plan API.
- Existing lock identity fields from `04-declaration-config-lock.md`.

Acceptance criteria:

- Core can ask what package-side update candidates exist for a current generation.
- The server can explain why a candidate update is available or blocked.
- Core still applies declaration, config, mutability, profile, policy, verification, and lock writing.

Verification strategy:

- Fixture tests for patch, minor, major, incompatible, and missing update paths.
- Tests comparing current lock summary to newer package metadata.
- Boundary tests proving update plan does not mutate local instance state.

Risks:

- Risk: server starts forcing updates.
  Mitigation: update plan is advisory. Core accepts or rejects.
- Risk: lock summary leaks private instance data.
  Mitigation: request shape should accept only package identity and digest fields needed for comparison.

### Phase 6: Artifact metadata and download

Goal: make artifact supply a formal server responsibility after the planning MVP is stable.

Deliverables:

- Artifact metadata API by exact package identity, artifact identity, digest, or manifest digest.
- Artifact availability response with implementation form, platform constraints where relevant, size, digest, manifest digest, signature metadata, and download locator.
- Download API or redirect flow for wasm artifacts as the first complete path.
- Metadata placeholders for Service and Native package artifact forms.
- Core-side local materialization contract documented for lock-pinned artifacts and manifests.

Dependencies:

- Phase 4 install plan API.
- Phase 5 update plan API if update artifact requirements are included.
- Lock identity requirements from `04-declaration-config-lock.md`.

Acceptance criteria:

- Core can fetch artifact metadata for an exact package selected during final resolve.
- Core can download a wasm artifact by immutable identity.
- Downloaded artifacts can be checked against expected digest and manifest digest.
- Already materialized locked generations can be replayed or rolled back without live server access.

Verification strategy:

- Artifact digest tests.
- Manifest digest tests.
- Negative tests for missing artifact, digest mismatch, and unsupported implementation form.
- Offline replay and rollback tests with server unavailable after local materialization.

Risks:

- Risk: artifact lookup uses floating latest identity.
  Mitigation: require exact identity or digest for lock-pinned downloads.
- Risk: server availability leaks into rollback.
  Mitigation: local materialization is a Core activation readiness requirement.

### Phase 7: Verification hardening

Goal: strengthen the trust path using server metadata without moving verify ownership out of Core.

Deliverables:

- Verification metadata API or fields for digest, manifest digest, signatures, provenance, build metadata, compatibility claims, and publication metadata.
- Verification policy input mapping that lets Core decide which metadata matters under Safe, Developer, or Trusted profiles.
- Clear failure reasons for missing signature, digest mismatch, provenance mismatch, unsupported source, or incompatible implementation form.
- Audit-friendly metadata snapshots suitable for inclusion in Core-owned locks.

Dependencies:

- Phase 6 artifact metadata and download.
- Profile and policy rules from `06-profiles-and-policy.md`.

Acceptance criteria:

- Core receives all server-side verification facts needed for final verification.
- Core remains the only component that marks a candidate generation verified.
- Lock output can include exact package identity, digest, manifest digest, source, verification result, and resolved binding snapshot.

Verification strategy:

- Positive and negative verification metadata fixtures.
- Tests for profile-specific verification requirements.
- Boundary review that the server never returns verified_for_instance as a final fact.

Risks:

- Risk: server metadata is treated as proof of runtime safety.
  Mitigation: call it verification metadata, not verification result.
- Risk: weak metadata blocks all local development.
  Mitigation: allow Core policy to vary strictness by profile while keeping audit records.

### Phase 8: Lock and generation integration

Goal: connect package server outputs to Core lock and generation flows while keeping Core authoritative.

Deliverables:

- Core consumption path for package detail, dependency tree, bind preview, install plan, update plan, artifact metadata, and verification metadata.
- Candidate lock generation from Core final resolve using server-supplied facts.
- Generation materialization flow that records local artifact availability.
- Replay and rollback behavior for locally materialized locked generations when the server is unavailable.
- Migration path from older local package index reads to the server-backed contract.

Dependencies:

- Phases 1 through 7.
- Existing Core resolve, verify, activate, rollback, and generation model.

Acceptance criteria:

- Core can produce a lock from a server-backed install plan after final resolve.
- Core can activate only after verification and required local materialization pass.
- Core can replay current local generation without server access.
- Core can roll back to a previous locally materialized generation without server access.
- Historical locks remain readable.

Verification strategy:

- End-to-end install plan to Core lock fixture.
- End-to-end update plan to new generation fixture.
- Server-unavailable replay tests.
- Server-unavailable rollback tests.
- Migration tests for older package metadata and older lock formats.

Risks:

- Risk: migration becomes a flag day rewrite.
  Mitigation: add a Core adapter seam first and demote old direct reads only after server-backed behavior passes.
- Risk: lock semantics weaken into version-only state.
  Mitigation: require digest, manifest digest, source, and resolved binding snapshots in Core-owned lock output.

### Phase 9: AI-generated package and bind workflow

Goal: support AI-assisted package and binding changes as reviewable candidates, not as automatic runtime truth.

Deliverables:

- AI package draft API or workflow for submitting generated package metadata before publication.
- AI bind draft API or workflow for proposing binding changes against a product package or instance intent.
- Draft edit API or workflow for revising AI-generated package and bind drafts before promotion, for example `PATCH /ai/drafts/bindings/{draft_id}` or an equivalent non-HTTP workflow.
- Validation report for generated package metadata, dependency declarations, capability declarations, implementation form, missing artifacts, and verification metadata gaps.
- Bind change preview that reuses bind preview and install plan contracts.
- Review and approval states before generated metadata becomes catalog truth.

Dependencies:

- Phase 4 install plan API.
- Phase 7 verification hardening.
- Package declaration rules from `04-declaration-config-lock.md` and binding rules from `05-binding-and-resolution.md`.

Acceptance criteria:

- AI-generated package metadata can be validated without being installed.
- AI-generated bind changes can be previewed without changing lock or active generation.
- AI-generated bind drafts can be edited before promotion without becoming catalog-visible or lock-visible.
- Generated metadata cannot bypass Core final resolve, policy, verification, lock generation, or activation.
- Review state is explicit before catalog publication.

Verification strategy:

- Tests for valid draft, missing capability, invalid dependency, fixed binding violation, conditional mutable violation, and missing artifact metadata.
- Contract tests that AI draft responses are draft or candidate states only.
- Review checklist confirming generated changes require Core resolve before runtime use.

Risks:

- Risk: AI workflow bypasses review.
  Mitigation: draft and approved states are separate, and only approved package metadata becomes catalog-visible.
- Risk: generated bind changes violate product identity.
  Mitigation: Core still enforces fixed and conditional mutable binding rules.

### Phase 10: GitHub ingestion later phase

Goal: ingest package metadata and artifacts from GitHub after the self-hosted server contract is stable.

Deliverables:

- Repository ingestion configuration for allowed owners, repositories, branches, tags, release assets, and package metadata paths.
- Ingestion parser for package metadata from repository contents or release bundles.
- Provenance mapping from GitHub repository, commit, tag, release, and asset digest into server package metadata.
- Quarantine or draft state before ingested packages become catalog-visible.
- Re-ingestion and update detection workflow.

Dependencies:

- Phase 6 artifact metadata and download.
- Phase 7 verification hardening.
- Phase 9 draft and approval workflow if ingested packages use the same review path.

Acceptance criteria:

- The server can ingest a package candidate from a configured GitHub source into a draft or quarantine state.
- Provenance metadata records repository, commit or tag, release asset identity when present, and digest fields.
- Ingested packages use the same package detail, dependency tree, bind preview, install plan, artifact, and verification contracts as locally authored packages.
- Core does not need GitHub-specific runtime logic.

Verification strategy:

- Fixture ingestion tests using local repository snapshots.
- Tests for tag change, asset digest mismatch, missing package metadata, unsupported schema, and duplicate package identity.
- Security review of allowed source configuration and provenance recording.

Risks:

- Risk: GitHub becomes a hidden runtime dependency.
  Mitigation: GitHub is ingestion only. Core talks to the Package Server contract.
- Risk: supply-chain trust is assumed from source location alone.
  Mitigation: record provenance and digest metadata, then let Core verification policy decide acceptance.

## 6. MVP scope

The MVP is only Phases 1 through 4, with Phase 0 as preparation.

MVP includes:

- package detail API
- dependency tree parser as supporting infrastructure for bind preview and install plan
- bind preview API
- install plan API
- explicit boundary documentation proving server outputs are not locks

MVP excludes:

- package upload
- artifact store
- artifact download
- full update orchestration
- verification hardening beyond metadata fields already present
- Core activation changes beyond consuming planning data as optional input
- AI-generated package or bind workflow
- GitHub ingestion

MVP success criteria:

1. Core or a CLI caller can inspect package detail from the self-hosted server.
2. Core or a CLI caller can preview binding candidates for required capabilities.
3. Core or a CLI caller can request an install plan for one product package.
4. The install plan contains package-side facts, candidate closure, dependency information, and unresolved choices, but not a final lock.
5. Core remains the only owner of final resolve, verification decision, lock generation, activation, rollback, and runtime truth.

## 7. Cross-phase verification strategy

Verification must cover contracts, behavior, and architectural boundaries.

### 7.1 Contract verification

- Validate every request and response shape against fixtures.
- Include negative fixtures for missing metadata, unknown package, unsupported schema, and ambiguous identity.
- Confirm response names and fields do not imply active generation or final runtime truth.

### 7.2 Boundary verification

- Review every phase against `16-package-service.md`.
- Confirm the server never writes instance config, lock, active generation, or rollback state.
- Confirm Core remains responsible for final resolve, policy, verify, activate, rollback, and lock writing.

### 7.3 Resolve-path verification

- Use representative product package and instance config fixtures.
- Confirm server outputs are sufficient inputs for Core resolve.
- Confirm Core still applies feature composition, fixed binding, mutable binding, conditional mutable binding, profile, policy, and runtime overrides.

### 7.4 Artifact and offline verification

This starts in Phase 6 and becomes release-blocking for artifact-backed activation.

Required cases:

1. server available, install plan accepted by Core, artifacts fetched, verified, activated
2. server unavailable, previously materialized active generation replayed locally
3. server unavailable, rollback to previously materialized older generation succeeds locally
4. server unavailable, rollback to a not-yet-materialized generation fails clearly without corrupting current active state

### 7.5 Migration verification

- Older package metadata without full taxonomy fields still resolves through compatibility logic.
- Older lock formats remain readable historical records.
- Mixed deployments can run while server-backed and older local paths coexist.

## 8. Main risks and mitigations

### 8.1 Server absorbs final resolve

Why it matters:

This collapses the main vNext boundary and makes instance legality harder to reason about.

Mitigation:

- Keep server responses framed as detail, dependency tree, preview, plan, metadata, draft, or ingestion candidate.
- Keep lock writing and final resolve in Core-owned modules and tests.

### 8.2 MVP becomes too large

Why it matters:

Adding upload, artifact storage, and GitHub ingestion too early delays the boundary proof.

Mitigation:

- Treat package detail, bind preview, and install plan as the first usable slice.
- Move upload, artifact store, AI workflow, and GitHub ingestion to later phases.

### 8.3 Online dependency leaks into rollback

Why it matters:

Already local locked generations must remain replayable and roll-backable offline.

Mitigation:

- Make local materialization a Core activation readiness rule once artifacts exist.
- Test server-unavailable replay and rollback as release-blocking scenarios.

### 8.4 Verification metadata is treated as verification result

Why it matters:

The server can provide facts, but Core decides whether a concrete generation is acceptable.

Mitigation:

- Use verification metadata naming in the server contract.
- Keep verified status in Core-owned lock and generation state.

### 8.5 GitHub ingestion weakens supply-chain assumptions

Why it matters:

Repository origin alone does not prove runtime safety.

Mitigation:

- Keep GitHub as ingestion, not runtime resolution.
- Record provenance, digest, manifest digest, and signature metadata where available.
- Reuse the same verification and approval path as other package sources.

## 9. Recommended implementation checkpoints

Before moving from one phase to the next, confirm:

1. Does this phase keep Core as the owner of final resolve, lock, verify, activate, rollback, and runtime truth?
2. Does this phase preserve or prepare offline replay and rollback for already local locked generations?
3. Does this phase make package supply more explicit without forcing a destructive migration?
4. Does this phase avoid adding upload or artifact storage before the MVP proves package detail, bind preview, and install plan?
5. Does this phase align with declaration, config, lock, binding, resolve, profile, and policy docs?

If any answer is no, correct the phase before adding more scope.

## 10. Execution summary

The self-hosted WEFT Package Server should be implemented as a package-supply and planning boundary, not as a replacement for Core.

The delivery order should be:

1. freeze the contract
2. ship package detail API
3. add dependency tree parsing
4. add bind preview API
5. add install plan API as the MVP proof
6. add update plan API
7. add artifact metadata and download
8. harden verification metadata
9. integrate with Core lock and generation flows
10. add AI-generated package and bind workflows
11. add GitHub ingestion later

This sequence gives WEFT a concrete self-hosted package server path while preserving the product package plus instance model, lock semantics, Core policy authority, and offline generation guarantees that vNext depends on.

## 11. Related documents

- `16-package-service.md`
- `04-declaration-config-lock.md`
- `05-binding-and-resolution.md`
- `06-profiles-and-policy.md`
- `../foundations/03-package-model.md`
- `../foundations/13-product-package-instance-model.md`
- `../00-overview.md`
