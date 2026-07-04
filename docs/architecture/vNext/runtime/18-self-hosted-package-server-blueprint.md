# WEFT vNext Blueprint: Self-Hosted Package Server

## 1. Purpose

This document defines the architecture blueprint for a self-hosted WEFT Package Server.

It extends the Package Service boundary described in `16-package-service.md` and gives it a concrete product shape:

- product package catalog
- dependency package catalog
- candidate generation support
- dependency closure support
- bind preview support
- install, update, and rollback planning support
- artifact metadata and distribution
- verification metadata
- AI-assisted package and bind generation flows

The important constraint is unchanged:

**The server owns package supply truth. Core owns runtime truth.**

That means the self-hosted Package Server can know what packages exist, which candidates are compatible, which dependency closures are possible, where artifacts live, and which metadata should be used for verification. It does not become the owner of a concrete instance's final resolve, lock, verify, activate, or rollback decision.

## 2. Goals

1. Provide a self-hosted package supply boundary that is easier to operate than a GitHub PR based registry as the primary product path.
2. Make product packages and dependency packages discoverable through one server-side catalog.
3. Give Core enough metadata to build strong candidate locks without forcing Core to own catalog indexing, dependency expansion, or artifact lookup.
4. Support bind and resolve preview so users and AI can understand possible assembly outcomes before a generation is accepted.
5. Support install, update, and rollback planning without moving final lifecycle authority away from Core.
6. Treat artifact identity, provenance, digest, signatures, and validation metadata as first-class package server data.
7. Support AI-generated package and bind proposals as structured drafts that still pass through Core resolve, verify, activate, and rollback flow.
8. Keep GitHub and PR workflows as future ingestion paths, not the runtime-facing source of truth for now.

## 3. Non-goals

The self-hosted Package Server is not intended to:

1. Replace Package as the source object.
2. Replace Instance as the runtime object.
3. Move final resolve into the server.
4. Move lock ownership into the server.
5. Move verify, activate, or rollback decisions into the server.
6. Become a remote runtime control plane.
7. Require a full rewrite of the existing vNext model.
8. Make online server availability mandatory for replaying or rolling back an already locked and locally materialized generation.
9. Make GitHub PR registry the primary package distribution model.
10. Hide binding and resolve as architectural concepts.

## 4. Current repo anchors

This blueprint should be read with these existing documents:

- `../00-overview.md`, the main vNext principle set
- `../foundations/03-package-model.md`, Package as source object
- `../foundations/13-product-package-instance-model.md`, Product Package plus Instance model
- `04-declaration-config-lock.md`, declaration, config, and lock separation
- `05-binding-and-resolution.md`, binding and resolve semantics
- `06-profiles-and-policy.md`, runtime governance inputs
- `16-package-service.md`, Package Service boundary
- `17-package-service-implementation-plan.md`, phased implementation direction
- `../workflows/08-ai-change-workflow.md`, AI propose, resolve, verify, activate flow

The current vNext model already says:

- Package is the only source-side published object.
- Product Package carries product default intent.
- Instance owns runtime config, current lock, active generation, and rollback history.
- Core performs binding, resolve, verify, activate, and rollback.
- The server-side package boundary supplies catalog data, candidates, dependency knowledge, artifacts, and metadata.

This blueprint keeps those anchors intact.

## 5. Why self-hosted server first

A GitHub PR based registry is useful for contribution review, public provenance, and future community package ingestion. It is not the best primary product architecture for the next step.

The self-hosted Package Server should be the primary path because it gives WEFT:

- a stable runtime-facing API
- controlled package metadata shape
- deterministic artifact lookup
- direct support for private packages
- faster iteration for product packages and dependency packages
- clearer support for AI-generated draft packages and bind proposals
- local or team-hosted operation without depending on public registry workflow

GitHub and PR workflows can still exist later as ingestion sources:

1. a package author submits a package source change through GitHub or another forge
2. CI builds and verifies package artifacts
3. an ingestion job imports accepted metadata and artifacts into the self-hosted Package Server
4. Core and runtime clients still talk to the Package Server, not directly to the PR registry

So the long-term model can include GitHub, but the runtime contract should be server-first.

## 6. Ownership split

### 6.1 Package owns source identity

Package remains the source-side object. It describes:

- package identity
- kind, provider, foundation, feature, or product
- provided capabilities
- required capabilities
- dependencies
- implementation form, WASM, Service, or Native
- default binding intent when it is a product package
- package source and build inputs

The Package Server indexes packages. It does not replace package declarations.

### 6.2 Instance owns runtime state

Instance owns the concrete runtime reality for one product package in one environment:

- instance config
- enabled feature set
- binding overrides
- profile and policy inputs
- current lock
- active generation
- rollback history
- local materialization state

The server may store useful package metadata for an instance plan, but it is not the owner of instance truth.

### 6.3 Server owns package supply data

The Package Server owns:

- catalog records
- candidate records
- dependency graph metadata
- artifact metadata
- artifact retrieval endpoints or locations
- provenance metadata
- digest and signature metadata
- compatibility metadata
- validation metadata
- install and update plan suggestions
- bind preview inputs and candidate explanations
- AI-generated package and bind draft records before Core acceptance

### 6.4 Core owns final runtime decisions

Core owns:

- reading product package declaration
- reading instance config
- applying feature enablement
- applying runtime overrides
- applying profile and policy
- final binding selection
- final resolve
- candidate lock generation
- verification decision
- activation decision
- rollback decision
- local cache and materialization checks

The server can propose. Core decides.

## 6.5 Server code boundary

The runtime-facing Package Server should be implemented as its own server-side area, separate from legacy plugin and suite routes.

Current transitional layout:

```text
server/src/
  main.rs                    # process startup, health, CORS, legacy Gitea plugin/suite routes
  package_server/
    mod.rs                   # current Package Server API surface and helpers
  weft_code/                # Weft Code service APIs
```

Target layout before adding upload, approval, artifact storage, or publishing logic:

```text
server/src/package_server/
  mod.rs          # public module boundary and re-exports
  routes.rs       # Package Server route assembly
  models.rs       # shared request/response DTOs
  state.rs        # package catalog/cache access helpers
  manifest.rs     # package.toml parsing and compatibility adapters
  catalog.rs      # package detail, product catalog, provider catalog, search
  bindings.rs     # dependency tree and bind preview
  plans.rs        # install and update plan previews
  artifacts.rs    # artifact and verification metadata
  drafts.rs       # AI package/bind drafts and draft promotion
  ingestion.rs    # upload/apply/review candidate skeletons
```

The legacy Gitea plugin and suite routes may remain in `main.rs` during transition, but they should not be treated as the long-term Package Server source of truth. Upload, application, review, approval, publication, artifact storage, and catalog mutation should belong to the self-hosted Package Server modules.

## 7. Domain model

### 7.1 ProductPackageRecord

A server catalog entry for a package with `kind = product`.

Recommended fields:

- `package_id`
- `name`
- `version`
- `source_ref`
- `manifest_digest`
- `artifact_set_id`
- `default_features`
- `default_bindings`
- `allowed_binding_overrides`
- `required_capabilities`
- `provided_product_surface`
- `compatibility`
- `release_channel`
- `verification_metadata`

Purpose:

- lets users install a product by name
- gives Core the product package declaration and metadata needed to start resolve
- gives AI a stable target when proposing product-level changes

### 7.2 DependencyPackageRecord

A server catalog entry for provider, foundation, or feature packages.

Recommended fields:

- `package_id`
- `kind`
- `name`
- `version`
- `source_ref`
- `manifest_digest`
- `artifact_set_id`
- `provides`
- `requires`
- `dependencies`
- `implementation_form`
- `platform_constraints`
- `compatibility`
- `verification_metadata`

Purpose:

- supports capability matching
- supports dependency closure generation
- supports update and rollback planning
- keeps implementation-form metadata visible without turning WASM, Service, or Native into taxonomy replacements

### 7.3 CandidateSet

A server-computed set of candidate packages for a requested capability, feature, product default binding, or runtime override.

Recommended fields:

- `request_id`
- `target_product_package`
- `requested_capability`
- `binding_context`
- `candidate_packages`
- `compatibility_notes`
- `policy_relevant_metadata`
- `rejection_reasons`

Purpose:

- makes candidate discovery explainable
- lets Core choose only after applying instance-specific rules
- lets UI and AI show users why some packages are available or rejected

### 7.4 DependencyClosureCandidate

A server-computed candidate dependency closure.

Recommended fields:

- `closure_id`
- `root_packages`
- `packages`
- `dependency_edges`
- `version_constraints`
- `compatibility_notes`
- `artifact_requirements`
- `known_conflicts`
- `server_explanation`

Purpose:

- removes dependency graph indexing burden from Core
- gives Core a clear candidate input for final resolve
- keeps closure advisory, not authoritative for runtime acceptance

### 7.5 ArtifactRecord

A server-owned metadata record for a concrete artifact.

Recommended fields:

- `artifact_id`
- `package_id`
- `package_version`
- `implementation_form`
- `artifact_kind`
- `uri`
- `size`
- `digest`
- `manifest_digest`
- `signature_refs`
- `build_provenance`
- `created_at`
- `retention_policy`

Purpose:

- makes artifact lookup deterministic
- supports exact lock-pinned downloads
- supports rollback by preserving identity for older artifacts
- keeps WASM payload delivery first-class while leaving room for Service and Native forms

### 7.6 VerificationMetadata

Server-side data that helps Core verify a candidate generation.

Recommended fields:

- package digest
- manifest digest
- artifact digest
- signatures
- signer identity
- build provenance
- source revision
- compatibility claims
- vulnerability or advisory metadata when available
- previous validation result references

Purpose:

- strengthens Core verification inputs
- avoids treating download success as validation
- preserves the rule that Core decides whether verification passes

### 7.7 InstallPlan

A server-proposed plan for installing a product package.

Recommended fields:

- `plan_id`
- `product_package`
- `requested_features`
- `candidate_sets`
- `dependency_closure_candidates`
- `required_artifacts`
- `expected_materialization_size`
- `verification_metadata`
- `explanation`

Purpose:

- gives Core and UI an explainable install preview
- does not write an instance lock
- does not activate anything

### 7.8 UpdatePlan

A server-proposed plan for moving from one package set to newer candidates.

Recommended fields:

- `plan_id`
- `current_lock_summary`
- `available_updates`
- `candidate_sets`
- `dependency_closure_candidates`
- `artifact_delta`
- `compatibility_risks`
- `rollback_materialization_requirements`
- `explanation`

Purpose:

- helps compare current lock inputs against newer package metadata
- keeps update acceptance in Core
- makes rollback readiness visible before activation

### 7.9 RollbackSupportRecord

Server metadata needed to support rollback for previously accepted locks.

Recommended fields:

- `lock_package_identities`
- `artifact_records`
- `artifact_availability`
- `historical_metadata_refs`
- `re_fetch_possible`
- `known_retention_risks`

Purpose:

- helps Core determine whether missing local artifacts can be re-fetched
- supports audit and recovery
- does not decide whether rollback is legal

### 7.10 AiDraftPackage

A server-side draft created by AI or an AI-assisted tool.

Recommended fields:

- `draft_id`
- `base_package_ref`
- `draft_manifest`
- `generated_source_refs`
- `intended_capabilities`
- `required_capabilities`
- `proposed_bindings`
- `test_hints`
- `risk_notes`
- `status`

Purpose:

- gives AI work a structured staging area
- supports generated WASM package and bind proposals
- prevents generated content from bypassing Core verification

## 8. API blueprint

These APIs are architectural shapes, not implementation code.

### 8.1 Catalog APIs

#### `GET /catalog/products`

Returns product packages that users can install.

Filters:

- name
- version range
- release channel
- capability surface
- compatibility target

#### `GET /catalog/packages`

Returns provider, foundation, feature, and product packages.

Filters:

- package kind
- capability provided
- capability required
- implementation form
- platform
- version range

#### `GET /catalog/packages/{package_id}`

Returns full package metadata, including manifest digest, dependencies, capabilities, compatibility, artifact set, and verification metadata.

### 8.2 Candidate and dependency APIs

#### `POST /candidates/bind`

Input:

- product package ref
- requested capability
- binding context
- optional instance profile summary
- optional policy hints
- optional current lock summary

Output:

- candidate packages
- compatibility notes
- policy-relevant metadata
- rejection reasons

This is a candidate API. It does not decide final binding.

#### `POST /candidates/closure`

Input:

- root package refs
- requested features
- candidate binding selections or constraints
- compatibility target

Output:

- one or more dependency closure candidates
- dependency edges
- artifact requirements
- conflicts and explanation

This is a closure support API. It does not produce the final lock.

### 8.3 Plan APIs

#### `POST /plans/install`

Input:

- product package ref
- requested features
- optional binding preferences
- target runtime compatibility

Output:

- install plan
- candidate sets
- dependency closure candidates
- required artifacts
- verification metadata
- explanation

Core consumes this plan as input to final resolve.

#### `POST /plans/update`

Input:

- current lock summary
- update intent
- allowed channels
- optional package pins

Output:

- update plan
- package deltas
- artifact deltas
- candidate closures
- compatibility notes
- rollback readiness notes

Core still decides whether to accept the update.

#### `POST /plans/rollback-support`

Input:

- previous lock summary
- local materialization summary when available

Output:

- artifact availability
- historical metadata
- re-fetch options
- retention risks

Core still decides whether rollback can proceed.

### 8.4 Artifact APIs

#### `GET /artifacts/{artifact_id}/metadata`

Returns immutable metadata for a concrete artifact.

#### `GET /artifacts/{artifact_id}/download`

Returns or redirects to the artifact payload.

For WASM packages, this is the primary runtime-loadable payload path.

For Service and Native packages, the same API can return metadata or handoff instructions if payload delivery differs.

### 8.5 Verification metadata APIs

#### `GET /verification/packages/{package_id}`

Returns package-level verification metadata.

#### `GET /verification/artifacts/{artifact_id}`

Returns artifact-level verification metadata.

Core uses this data as verification input. Core does not outsource verification acceptance to this endpoint.

### 8.6 AI draft APIs

#### `POST /ai/drafts/packages`

Creates a draft package record from AI-generated source, manifest, or capability intent.

Output:

- draft id
- inferred package metadata
- missing fields
- risk notes
- next required validation steps

#### `POST /ai/drafts/bindings`

Creates a draft binding proposal.

Input:

- target product package
- current instance summary
- requested behavior
- proposed capability binding changes

Output:

- candidate binding proposal
- affected capabilities
- needed package candidates
- dependency closure candidates
- risks and explanation

#### `PATCH /ai/drafts/bindings/{draft_id}`

Revises an AI-generated draft binding proposal before promotion.

Input:

- revised capability binding changes
- updated target package or provider preferences
- updated reason or review notes

Output:

- updated draft binding proposal
- changed capabilities
- updated risks and explanation

This edit keeps the binding proposal in draft state. It does not make the bind catalog-visible, lock-visible, or active.

#### `POST /ai/drafts/{draft_id}/promote-candidate`

Promotes a draft into a candidate package or candidate binding proposal visible to Core resolve.

This promotion does not activate anything. It only makes the proposal available for Core-owned resolve and verify flow.

## 9. Data flow

### 9.1 Install data flow

1. User selects a product package in CLI, TUI, GUI, or AI-assisted flow.
2. Runtime asks Core to prepare an install.
3. Core queries the Package Server for product metadata.
4. Core asks the server for bind candidates and dependency closure candidates.
5. The server returns an install plan with candidate sets, closure candidates, artifact metadata, and verification metadata.
6. Core performs final resolve using product declaration, instance config, feature choices, profile, policy, overrides, and server results.
7. Core creates a candidate lock.
8. Core fetches required artifacts through the server artifact path.
9. Core materializes lock-pinned artifacts locally.
10. Core verifies the candidate generation.
11. Core activates the generation if verification passes.
12. Instance records the lock, active generation, and rollback point.

### 9.2 Update data flow

1. User, runtime, or AI requests an update preview.
2. Core sends the current lock summary and update intent to the Package Server.
3. The server returns newer package candidates, dependency closure candidates, artifact deltas, and compatibility notes.
4. Core evaluates the update under declaration, config, profile, policy, mutability, and local runtime constraints.
5. Core creates a candidate lock for the update.
6. Core fetches and materializes required artifacts.
7. Core verifies the candidate generation.
8. Core activates the update only if verification passes.
9. The previous generation remains available as a rollback target if its artifacts are still local or re-fetchable.

### 9.3 Rollback data flow

1. User or system requests rollback to a previous generation.
2. Core checks instance rollback history and previous lock.
3. Core checks local materialization for required artifacts.
4. If artifacts are missing, Core may query the Package Server for rollback support metadata and artifact re-fetch options.
5. Core verifies the rollback target locally after artifacts are available.
6. Core activates the previous generation only if verification passes.

If the previous generation is already locked and fully local, rollback must not require live Package Server availability.

## 10. Bind and resolve preview

Bind and resolve preview exists to help users and AI understand possible outcomes before Core accepts a generation.

### 10.1 Bind preview

Bind preview answers:

- which packages can satisfy this capability
- which candidates match product defaults
- which candidates are allowed by mutable or conditional binding rules
- which candidates are likely blocked by compatibility or policy metadata
- what dependency changes each candidate would introduce

Bind preview should be explainable. A user should be able to see why one provider is recommended, why another is rejected, and what would change if a binding override is chosen.

### 10.2 Resolve preview

Resolve preview answers:

- what packages might be included
- which dependency closure candidates are possible
- what artifacts would be needed
- what verification metadata is available
- what conflicts or compatibility risks exist
- what Core still needs to decide locally

Resolve preview is not a final lock. It is a plan-shaped candidate set.

### 10.3 Preview authority rule

The server may say:

- this candidate exists
- this candidate appears compatible by catalog metadata
- this closure is computable
- this artifact is available
- this update appears possible

The server must not say:

- this concrete instance is resolved
- this lock is final
- this generation is verified
- this generation is active
- rollback is approved

Those statements belong to Core and Instance state.

## 11. Install, update, and rollback flow

### 11.1 Install flow

Install should feel simple to the user:

1. choose product
2. select latest compatible package metadata by default when no lock exists or an explicit version is not requested
3. preview package and binding plan
4. accept
5. Core resolves
6. Core writes an immutable lock with exact package, artifact, source, digest, manifest digest, and binding identities
7. Core verifies
8. Core activates

Default latest is only a selection convenience. It must not remain part of runtime truth after acceptance.

Behind that simple UX, the Package Server supplies:

- product metadata
- package candidates
- dependency closure candidates
- artifact records
- verification metadata
- plan explanation

Core supplies:

- final resolve
- lock
- local materialization
- verify
- activate
- rollback point

### 11.2 Update flow

Update should be plan-first:

1. current lock is summarized
2. server calculates available package updates and likely closures
3. UI shows what will change
4. Core resolves against real instance constraints
5. Core verifies and activates only if safe

Update plans should clearly separate:

- package version changes
- dependency changes
- binding changes
- artifact changes
- policy or profile concerns
- rollback readiness

### 11.3 Rollback flow

Rollback should be lock-first:

1. choose previous generation
2. check local materialization
3. re-fetch missing artifacts only if needed
4. verify previous lock and artifacts
5. activate previous generation

The server helps with historical metadata and artifact re-fetch. It does not own rollback history and does not decide activation.

## 12. Artifact metadata and verification

Artifact metadata must be exact enough to support reproducibility, audit, and rollback.

Minimum metadata:

- package id
- package version
- implementation form
- artifact id
- artifact kind
- artifact digest
- manifest digest
- source reference
- build provenance
- signature references
- compatibility claims
- created time
- retention policy

Package selection may default to the latest compatible version before acceptance, but Core should write locks using immutable artifact identity, not floating latest lookup.

Verification should include at least:

- manifest digest match
- artifact digest match
- package declaration consistency
- capability declaration consistency
- dependency closure consistency
- profile and policy acceptance
- runtime load or probe when relevant

For WASM packages, the server should make payload distribution and digest metadata first-class. For Native and Service packages, metadata must still be first-class even if actual delivery uses a different handoff.

## 13. AI-generated package and bind flow

AI can help users create and modify packages, but AI cannot bypass the lifecycle gates.

### 13.1 AI package generation

1. User describes desired behavior.
2. AI proposes a package draft with manifest, capabilities, dependencies, and source refs.
3. The Package Server stores the draft as `AiDraftPackage`.
4. The server or build worker produces artifact metadata if a build is available.
5. The draft becomes a candidate package only after required metadata exists.
6. Core may include the candidate in a resolve attempt.
7. Core verifies before activation.

AI-generated packages should be marked as draft, generated, or untrusted until the normal verification path accepts them.

### 13.2 AI bind generation

1. User asks for a behavior change.
2. AI proposes binding changes against a product package or instance config.
3. The server returns bind candidates and dependency closure candidates.
4. AI explains tradeoffs to the user.
5. Core resolves the chosen proposal against declaration, config, profile, policy, and mutability rules.
6. Core verifies and activates only if safe.

The AI can recommend a binding. It cannot make the binding final.

### 13.3 AI update generation

1. AI reads current lock summary and user intent.
2. AI asks the server for update candidates.
3. The server returns plan-shaped data.
4. AI explains expected package, binding, and artifact changes.
5. Core owns final update resolve and activation.

### 13.4 Safety constraints

AI flows must preserve these rules:

- draft first, never active first
- plan first, never direct mutation
- Core resolve before lock
- Core verify before activate
- rollback point before switching active generation
- generated package metadata visible to the user and audit trail

## 14. Ingestion model

The Package Server should support multiple ingestion paths over time.

### 14.1 Primary ingestion for now

For the self-hosted first version, ingestion can be direct and controlled:

- local package source import
- admin upload
- CI build output import
- AI draft promotion
- internal package publishing command

This keeps the product path simple and operable.

### 14.2 Future GitHub or PR ingestion

GitHub PR based registry can become an ingestion path later:

1. contributor opens PR with package source or manifest changes
2. CI builds artifacts and produces metadata
3. maintainers review and merge
4. importer publishes metadata and artifacts into the self-hosted Package Server

The server remains the runtime-facing authority even when GitHub is used for contribution workflow.

## 15. Storage shape

The server can use any backend that preserves these logical stores:

- package catalog store
- dependency graph store
- artifact metadata store
- verification metadata store
- draft package and draft binding store
- plan cache store
- ingestion audit log

The storage backend is intentionally not specified here. SQLite, Postgres, object storage, local filesystem, or a hybrid layout can all work if the API contract and identity rules remain stable.

## 16. Final UX

The target user experience should hide package supply complexity without hiding architectural truth.

### 16.1 User installs a product

User sees:

- product name
- version or channel
- short capability summary
- recommended package set
- important binding choices
- download size
- verification status
- rollback readiness

User should not need to manually inspect every dependency package unless they ask.

### 16.2 User previews a bind change

User sees:

- current provider
- proposed provider
- reason for recommendation
- packages added or removed
- compatibility notes
- verification requirements
- whether rollback will remain available

### 16.3 User updates an instance

User sees:

- what package versions change
- whether bindings change
- what artifacts will be downloaded
- risk notes
- validation status after Core verify
- safe rollback point

### 16.4 AI customizes a product

User says what they want. AI proposes a package or bind change. The UI shows:

- proposed behavior
- generated or selected packages
- dependency impact
- artifact impact
- verification status
- final Core decision

The final button should not be “trust AI and mutate runtime”. It should be closer to:

1. preview proposal
2. resolve candidate
3. verify candidate
4. activate verified generation

## 17. Blueprint summary

The self-hosted WEFT Package Server should be the primary package supply boundary for vNext.

It owns catalog, candidate, dependency, artifact, provenance, verification metadata, install plan, update plan, rollback support, and AI draft surfaces. It keeps GitHub and PR workflows as future ingestion paths instead of making them the runtime-facing registry.

The core architectural split remains strict:

- Package is the source object.
- Instance owns config, lock, generation, and rollback history.
- Package Server owns package supply truth.
- Core owns final resolve, lock, verify, activate, and rollback.

That split gives WEFT a practical self-hosted package system without rewriting the runtime model or weakening local instance authority.
