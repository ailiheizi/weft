# WEFT vNext Detailed Plan: Product Package Server

## 1. Purpose

This document turns the self-hosted Package Server blueprint into a detailed product, architecture, and implementation plan.

It is not an MVP reset. The current server already has a meaningful Package Server surface under `server/src/package_server/`:

- product catalog
- catalog search
- provider catalog
- package detail
- bind preview
- dependency tree preview
- install and update plan preview
- artifact and verification metadata preview
- package store status
- upload draft lifecycle
- candidate and review queue lifecycle
- AI package and binding drafts
- ingestion candidate scaffolding

The next step is to harden this surface into a coherent **Product Package Server**.

The invariant remains unchanged:

**Package Server owns package supply truth. Core owns runtime truth.**

## 2. Product shape

The server is a product-aware package supply and planning service. It is not a generic plugin store and it is not a remote Core.

The primary objects are:

1. **Product packages** such as `weft-claw` and `weft-code`.
2. **Feature packages** that express product-level capabilities.
3. **Foundation packages** that provide stable runtime capabilities.
4. **Provider packages** that satisfy capability requirements.
5. **Drafts, candidates, reviews, and ingestion records** used to manage package supply.

The user sees products and capabilities. Developers submit packages and package proposals. Reviewers audit supply risk. Core consumes package facts and still performs final resolve, lock, verify, activate, and rollback.

## 3. User experience model

### 3.1 Normal user

The normal user should experience the Package Server through a product-oriented surface:

- discover `Weft Claw` and `WEFT-Code`
- inspect product capabilities
- choose optional capabilities
- preview install or update impact
- ask Core to verify and install locally
- see local runtime status after Core activates a generation
- roll back using local materialized state

The user should not need to understand artifact digests, closure candidates, or lock structure during the common path.

Important UI language:

- Server output is `Catalog`, `Candidate`, `Preview`, `Plan`, or `Published`.
- Local/Core output is `Resolved`, `Locked`, `Verified`, `Materialized`, or `Active`.
- A published package is not automatically installed.
- An install preview is not a lock.
- Server verification metadata is not instance verification.

When the server is unavailable, installed products must still appear as runnable if local Core state supports them. New install, update, and missing artifact fetch may be unavailable, but active local generations remain local truth.

### 3.2 Developer

The developer submits a package proposal, not a runtime mutation.

Developer workflow:

1. Create or generate package metadata.
2. Declare identity, package kind, capabilities, dependencies, runtime form, lifecycle hooks, validation checks, and governance metadata.
3. Submit an upload draft, AI draft, ingestion candidate, or CI-produced candidate.
4. Receive structured feedback for missing metadata, validation issues, and risk notes.
5. Revise until the candidate is reviewable.

Developers cannot publish directly to runtime, write user locks, or activate packages.

### 3.3 Reviewer

The reviewer audits package supply risk:

- package identity and version
- capability declarations
- dependency and closure impact
- binding impact on products
- artifact digest and provenance
- permissions and lifecycle hooks
- validation and rollback readiness
- compatibility with product packages

Reviewer actions are supply actions:

- request changes
- reject
- approve metadata
- approve artifact metadata
- publish metadata
- yank or deprecate versions

Reviewer actions never write instance locks or activate runtime generations.

### 3.4 Client

The client should show two fact planes:

1. **Package-side facts from server**: products, packages, provider candidates, previews, artifact availability, verification metadata, review/publication state.
2. **Runtime-side truth from Core/local state**: lock, product graph, active generation, runtime sync, active bindings, rollback availability, health.

This separation preserves the existing frontend model and prevents preview data from masquerading as runtime truth.

## 4. Non-negotiable boundaries

### 4.1 Server may do

- serve product and package catalog records
- serve package details and manifest metadata
- compute provider candidates
- compute dependency closure candidates
- produce bind, install, update, and rollback-support previews
- store drafts, candidates, review events, and ingestion records
- store artifact metadata and later serve immutable artifact downloads
- expose provenance, digest, signature, compatibility, and validation metadata
- publish metadata into catalog truth after review

### 4.2 Server must not do

- final resolve
- final binding selection
- lock generation or lock writes
- instance verification decision
- activation
- rollback decision
- active generation ownership
- local materialization ownership
- remote runtime control plane behavior
- Core calls to make server tests pass

### 4.3 Core remains owner of

- product declaration reading
- instance config reading
- feature enablement
- policy and profile application
- final binding legality
- final resolve
- candidate lock generation
- verification decision
- activation decision
- rollback decision
- local cache, artifact materialization, and offline replay

## 5. Existing code anchors

Current Package Server implementation is under:

```text
server/src/package_server/
  routes.rs
  models.rs
  state.rs
  manifest.rs
  catalog.rs
  bindings.rs
  plans.rs
  artifacts.rs
  drafts.rs
  draft_store.rs
  ingestion.rs
  uploads.rs
  candidates.rs
  reviews.rs
  store.rs
  tests.rs
```

Current primary routes include:

```text
GET  /api/catalog/products
GET  /api/catalog/search/{query}
GET  /api/catalog/providers
GET  /api/package-store/status
GET  /api/sources/packages
GET  /api/packages/{name}
GET  /api/packages/bind-preview/{capability}
GET  /api/packages/{name}/bind-preview
GET  /api/packages/{name}/dependency-tree
GET  /api/packages/{name}/install-plan
GET  /api/packages/{name}/update-plan
GET  /api/packages/{name}/artifacts
GET  /api/packages/{name}/verification
POST /api/ingestion/packages
POST /api/packages/uploads
PATCH /api/packages/uploads/{upload_id}
POST /api/packages/uploads/{upload_id}/withdraw
GET  /api/package-review-queue
GET  /api/package-candidates
GET  /api/package-candidate-events
GET  /api/package-candidates/{candidate_id}
GET  /api/package-candidates/{candidate_id}/events
POST /api/ai/drafts/packages
POST /api/ai/drafts/bindings
PATCH /api/ai/drafts/bindings/{draft_id}
POST /api/ai/drafts/{draft_id}/promote-candidate
GET  /api/package-reviews
GET  /api/package-reviews/by-id/{review_id}
POST /api/package-reviews/{candidate_id}/approve
POST /api/package-reviews/{candidate_id}/reject
```

The current implementation is correctly metadata-first. The plan below hardens semantics, adds persistent stores, and graduates previews into Core-consumable planning contracts without moving runtime authority into the server.

## 6. Domain model target

### 6.1 ProductPackageRecord

Product packages are first-class catalog objects.

Recommended fields:

```text
package_ref
name
version
display_name
description
package_kind = product
release_channel
source_authority
source_ref
source_revision
manifest_digest
artifact_set_id
required_capabilities
provided_product_surface
default_features
optional_features
feature_bindings
default_bindings
allowed_binding_overrides
validation_checks
upgrade_strategy
compatibility
verification_metadata
created_at
updated_at
```

### 6.2 DependencyPackageRecord

Dependency packages cover provider, foundation, and feature packages.

Recommended fields:

```text
package_ref
name
version
package_kind
implementation_form
runtime_provider
display_name
description
source_authority
source_ref
source_revision
manifest_digest
artifact_set_id
provides
requires
platform_constraints
compatibility
validation_checks
verification_metadata
created_at
updated_at
```

### 6.3 PackageVersionRecord

The server must eventually model version explicitly instead of treating `current_source` as identity.

```text
name
version
channel
source_ref
source_revision
manifest_digest
package_record_ref
artifact_set_id
published_at
published_by
review_status
```

### 6.4 CandidateSet

Candidate sets are advisory.

```text
request_id
product_package_ref
capability
binding_context
candidates
recommended_candidate
rejected_candidates
compatibility_notes
policy_relevant_metadata
explanation
```

Do not call a candidate `selected_provider`; final selection is Core-owned.

### 6.5 DependencyClosureCandidate

```text
closure_id
root_packages
packages
dependency_edges
unresolved_edges
version_constraints
known_conflicts
artifact_requirements
compatibility_notes
server_explanation
```

This is not a lock.

### 6.6 InstallPlan

```text
plan_id
plan_kind = install
product_package
requested_features
binding_preferences
candidate_sets
dependency_closure_candidates
required_artifacts
verification_metadata
unresolved_choices
compatibility_risks
estimated_materialization
explanation
core_ownership
```

`core_ownership` must say:

```text
final_resolve = core_owned
lock = core_owned
verify = core_owned
activate = core_owned
rollback = core_owned
```

### 6.7 UpdatePlan

```text
plan_id
plan_kind = update
current_lock_summary
update_intent
available_versions
changed_packages
unchanged_packages
removed_packages
candidate_sets
dependency_closure_candidates
artifact_delta
verification_metadata_delta
compatibility_risks
rollback_readiness_notes
explanation
core_ownership
```

### 6.8 ArtifactRecord

```text
artifact_id
package_ref
implementation_form
artifact_kind
uri
size_bytes
digest
manifest_digest
signature_refs
build_provenance
platform_constraints
created_at
retention_policy
availability_status
```

Artifact downloads must use immutable artifact identity and digest verification metadata. No floating `latest` download path should become a Core materialization input.

### 6.9 VerificationMetadata

```text
package_digest
manifest_digest
artifact_digest
signatures
signer_identity
source_revision
build_provenance
compatibility_claims
validation_report_refs
vulnerability_refs
generated_by
verified_by_server_process
```

This metadata may support Core verification but must not be named or treated as instance verification.

### 6.10 PackageCandidateRecord

```text
candidate_id
candidate_source = upload_draft | ai_draft | ingestion | admin_import | ci_import
candidate_kind = package_metadata | artifact_metadata | binding_proposal | product_update
status = draft | quarantined | needs_more_input | reviewable | approved | rejected | withdrawn | promoted | published_metadata
package_metadata
artifact_metadata
binding_proposal
validation_issues
missing_fields
risk_notes
review_decisions
events
catalog_visibility
catalog_mutation_performed
lock_mutation_performed = false
activation_performed = false
```

## 7. API evolution plan

### 7.1 Stabilize existing GET APIs

Existing browse and preview APIs should remain stable, but their vocabulary must become explicit:

- `advisory`
- `preview`
- `metadata_only`
- `authority`
- `core_ownership`
- `package_ref`

Priority endpoints:

```text
GET /api/catalog/products
GET /api/packages/{name}
GET /api/catalog/providers
GET /api/packages/{name}/bind-preview
GET /api/packages/{name}/dependency-tree
GET /api/packages/{name}/install-plan
GET /api/packages/{name}/update-plan
GET /api/packages/{name}/artifacts
GET /api/packages/{name}/verification
```

### 7.2 Add Core-facing POST APIs

GET previews remain useful for browse and smoke tests. Core-facing planning should move to request-shaped POST APIs.

Recommended additions:

```text
POST /api/candidates/bind
POST /api/candidates/closure
POST /api/plans/install
POST /api/plans/update
POST /api/plans/rollback-support
```

These endpoints accept product refs, current local summaries, requested features, compatibility targets, and binding preferences. They still return advisory candidate and plan records.

### 7.3 Add artifact and verification APIs

When artifact storage is available:

```text
GET /api/artifacts/{artifact_id}/metadata
GET /api/artifacts/{artifact_id}/download
GET /api/verification/packages/{package_id}
GET /api/verification/artifacts/{artifact_id}
```

Download availability does not imply activation authority.

### 7.4 Add publication APIs

After persistence and review hardening:

```text
POST /api/package-candidates/{candidate_id}/publish-metadata
POST /api/packages/{name}/versions/{version}/yank
POST /api/packages/{name}/versions/{version}/deprecate
```

Publication mutates catalog metadata only.

## 8. Storage plan

Current state is local index plus ephemeral draft store. The target logical stores are:

```text
PackageCatalogStore
PackageVersionStore
DependencyGraphStore
ArtifactMetadataStore
VerificationMetadataStore
CandidateStore
DraftStore
ReviewStore
IngestionAuditLog
PlanCacheStore
```

Recommended migration:

1. File snapshots and existing local `packages/index.toml` adapters.
2. SQLite for self-hosted metadata persistence.
3. SQLite plus filesystem artifact store.
4. Optional Postgres plus object storage for hosted/team deployments.

## 9. Implementation plan

### Phase 1: Contract hardening

Scope:

- stabilize response vocabulary
- add boundary fields to catalog/detail/tree/plan responses
- strengthen tests against runtime-owned fields
- keep behavior metadata-only

Acceptance:

- product catalog includes `weft-claw` and `weft-code`
- product catalog declares catalog-only authority
- package detail declares metadata-only authority
- dependency tree declares preview-only authority
- install/update plans include full Core ownership including rollback
- no Package Server endpoint returns `runtime_lock`, `active_generation`, `activation_result`, or `rollback_result`

### Phase 2: Product catalog authority

Scope:

- build richer product records from `packages/index.toml` and product `package.toml`
- expose default features, optional features, required capabilities, validation checks, and upgrade strategy
- add diagnostics when product metadata is incomplete

Acceptance:

- `weft-claw` detail exposes bindings and validation checks
- `weft-code` detail exposes `weft_code.runtime`
- product catalog no longer acts as a name-only list

### Phase 3: Capability and dependency index

Scope:

- build capability-to-provider index
- build package-to-requires and package-to-provides maps
- enrich provider catalog with candidate reason and compatibility notes

Acceptance:

- `agent.runtime`, `memory.store`, and `team.delegate` provider lists are explainable
- missing provider cases return diagnostics

### Phase 4: Dependency closure engine

Scope:

- add closure candidate computation
- detect unresolved edges
- detect cycles
- separate package-name dependencies from capability requirements

Acceptance:

- default `weft-claw` closure candidate computes
- default `weft-code` closure candidate computes
- missing package and cycle fixtures return explicit diagnostics
- closure response does not contain lock fields

### Phase 5: Bind Preview V2

Scope:

- add `POST /api/candidates/bind`
- explain fixed vs mutable binding
- return accepted and rejected candidates with reasons
- include dependency and artifact impact summaries

Acceptance:

- fixed binding cannot be represented as server-selected mutable state
- mutable binding returns allowed candidates
- unknown capability returns diagnostic, not fake success

### Phase 6: Install Plan V2

Scope:

- add `POST /api/plans/install`
- accept requested features and binding preferences
- include candidate sets, closure candidates, artifacts, verification metadata, unresolved choices, and Core ownership

Acceptance:

- `weft-claw` default install plan includes default features
- `weft-code` default install plan includes core feature packages
- optional features alter candidate closure preview
- plan cannot be directly treated as lock

### Phase 7: Update Plan V2

Scope:

- add `POST /api/plans/update`
- accept current lock summary
- compare versions and channels
- model changed, unchanged, added, removed packages
- include rollback readiness notes

Acceptance:

- no-op update is explicit
- pinned packages are respected
- update plan does not write local state

### Phase 8: Persistent candidate, draft, and review store

Scope:

- introduce SQLite-backed metadata store
- persist drafts, candidates, review decisions, and events
- preserve existing API compatibility where possible

Acceptance:

- candidate survives server restart
- review decision survives server restart
- approved candidate still does not mutate runtime

### Phase 9: Catalog publication workflow

Scope:

- add metadata publication from approved candidates
- enforce version immutability
- add catalog mutation audit
- support yanked and deprecated states

Acceptance:

- unapproved candidate cannot publish
- duplicate version cannot silently overwrite
- published metadata appears in catalog
- no lock or activation is performed

### Phase 10: Artifact metadata and download

Scope:

- persist artifact metadata
- add immutable artifact ids
- serve metadata and download endpoints
- validate digest metadata

Acceptance:

- artifact download uses immutable artifact id
- digest metadata is present before Core materialization
- digest mismatch fixtures fail clearly

### Phase 11: Verification metadata hardening

Scope:

- store signature, digest, provenance, and validation report metadata
- expose package and artifact verification metadata APIs
- allow Core to snapshot verification metadata into locks

Acceptance:

- missing signatures are metadata gaps
- server response never claims `verified_for_instance`

### Phase 12: Core integration

Scope:

- extend Core package service client beyond index fetch
- map server candidate plans into Core resolve inputs
- keep Core lock and activation ownership
- support server-unavailable replay from local state

Acceptance:

- Core can consume install planning facts
- Core still writes local lock
- active local generation remains usable without server

### Phase 13: Client integration

Scope:

- add package-side facts panels
- keep runtime-side truth local
- label catalog/candidate/preview/plan vs resolved/locked/active clearly

Acceptance:

- client shows product catalog and package detail from server
- client still shows lock/product graph from local state
- server unavailable does not erase runtime dashboard

### Phase 14: AI package and binding workflow

Scope:

- enrich AI drafts with missing fields, risk notes, validation requirements, and package impact
- route all promotions through reviewable candidates

Acceptance:

- AI draft cannot publish catalog directly
- AI binding draft cannot mutate lock

### Phase 15: Ingestion and external source integration

Scope:

- import from local, CI, GitHub/Gitea, release assets
- capture provenance
- quarantine before review

Acceptance:

- external source does not become runtime dependency
- imported package goes through same candidate/review/publication path

## 10. ADR set

The implementation should be governed by ADRs under `docs/architecture/vNext/adr/`.

Initial ADRs:

1. `0001-product-package-server-authority.md`: Server owns package supply truth; Core owns runtime truth.
2. `0002-package-server-review-publication-boundary.md`: Review publishes metadata only; no runtime mutation.
3. `0003-package-server-client-experience-boundary.md`: Client separates package-side facts from runtime-side truth.

## 11. Verification strategy

### 11.1 Boundary tests

Every Package Server route should be protected against runtime authority leaks.

Forbidden fields:

```text
runtime_lock
active_generation
activation_result
rollback_result
verified_for_instance
final_resolved_bindings
core_resolve_call
core_verify_call
core_activate_call
```

### 11.2 Contract tests

Cover:

- product catalog
- package detail
- provider catalog
- bind preview
- dependency tree and closure
- install plan
- update plan
- artifact metadata
- verification metadata
- candidate lifecycle
- review lifecycle

### 11.3 Client compatibility tests

Ensure local frontend/Core state remains readable:

- suite lock
- install plan artifact
- product graph
- runtime sync
- product lock

### 11.4 Offline tests

After artifact support:

- install while server is available
- replay while server is unavailable
- rollback to materialized generation while server is unavailable
- fail clearly when rollback requires missing artifact fetch

## 12. Recommended first implementation round

The first implementation round after this document should be small:

1. Add missing authority/core ownership fields to existing response DTOs.
2. Add tests proving catalog/detail/tree/plan responses cannot be confused with runtime truth.
3. Keep all existing server behavior metadata-only.
4. Run `cargo test -p weft-server package_server` or the closest package-server test filter.

This establishes the vocabulary and test harness required for the larger phases.
