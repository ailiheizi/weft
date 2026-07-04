# WEFT vNext Proposal: Dedicated Package Service Boundary

## 1. Why this proposal exists

The current vNext direction has already made one major simplification clear:

- Package is the sole source object
- Instance is the runtime object
- product package expresses default intent
- instance config and lock express runtime reality

That direction is correct, but package usage can still feel hard in practice.

The hard part is no longer only the package taxonomy itself. The harder part is that several different concerns are still mentally bundled together:

- matching a capability or binding request to concrete packages
- resolving package dependencies into a stable closure
- downloading or updating the selected artifacts
- tracking validation metadata for those artifacts
- distributing runtime payloads, especially wasm packages

When those concerns are treated as one blended responsibility inside the local runtime path, the user and the system both end up carrying too much package mechanism complexity.

This proposal introduces a dedicated **server-side Package Service** boundary to reduce that complexity.

The key constraint is that this boundary is **authoritative for package catalog truth and package supply**, but **not** for runtime truth. Core and the local instance remain authoritative for what is actually resolved, locked, verified, activated, and rolled back on a concrete machine.

This document is a proposal only. It does **not** claim that the boundary already exists in implementation.

## 2. Problem statement

The vNext package model is already converging around the right source and runtime objects, but package consumption still feels heavier than it should.

### 2.1 Why current package usage feels hard

From the perspective of a product runtime or client-facing product flow, package work currently tends to collapse multiple questions into one:

1. **What package should satisfy this capability or binding request?**
2. **What other packages does that choice pull in?**
3. **Where do those artifacts come from, and which version should be used?**
4. **How are updates evaluated without breaking the current instance?**
5. **How does a runtime obtain and distribute wasm payloads in a repeatable way?**

Those are related questions, but they are not the same question.

The result is friction in several places:

- product package declaration, feature composition, matching, and artifact transport are too easy to conflate
- lock generation risks looking like “download whatever seems current” instead of “record a verified resolution result”
- rollback can become harder to reason about if metadata and artifact provenance are not treated as first-class service concerns
- wasm distribution becomes an accidental side effect of resolve, rather than a formal distribution responsibility
- client and runtime surfaces can end up carrying package-source complexity that should stay behind a narrower boundary

In short, the package model is becoming clearer, but the package supply path is still too implicit.

## 3. Design goal

Introduce a dedicated Package Service that centralizes package discovery, metadata, matching, dependency resolution support, artifact download and update management, while preserving the existing vNext concepts:

- Package remains the sole source object
- Instance remains the runtime object
- provider / foundation / feature / product remain the package taxonomy
- declaration / config / lock remain separate objects
- binding / resolve remain formal assembly concepts
- profile / policy remain runtime governance inputs
- verify / activate / rollback remain Core lifecycle decisions
- WASM / Service / Native remain implementation forms, not taxonomy replacements

The Package Service should make package usage feel simpler **without** replacing the architectural meaning of Package, Instance, Core, or Runtime.

## 4. Proposed boundary

### 4.1 What the Package Service is

The Package Service is a dedicated server-side boundary for package supply and package assembly support.

It should be understood as:

- the canonical service for package index and package metadata
- the service that answers package matching and dependency-closure questions
- the service that manages package artifact retrieval and update metadata
- the service that distributes package payloads, especially wasm artifacts

It should **not** be understood as:

- a replacement for Core
- a replacement for resolve semantics
- a replacement for instance lock semantics
- a replacement for verify / activate / rollback
- a new top-level object that competes with Package or Instance

### 4.2 Why a dedicated service boundary helps

A dedicated service boundary helps because it separates **package supply and package selection support** from **runtime execution and runtime state truth**.

That separation gives clearer ownership:

- package catalog concerns stay centralized
- dependency and update knowledge can be reused across products and runtimes
- artifact distribution becomes a formal subsystem instead of an implementation detail
- local runtimes can stay focused on instance assembly and lifecycle decisions
- wasm package delivery gets a first-class home, which is especially important when wasm is the default extension route

This is not about making the architecture more distributed for its own sake. It is about removing accidental coupling between source/package logistics and runtime truth.

## 5. Ownership model

### 5.1 What the Package Service owns

The Package Service should own the following responsibilities.

#### A. Package index and metadata

The service is the authoritative read surface for package catalog information, including at minimum:

- package identity
- package kind, provider / foundation / feature / product
- package version
- source and provenance metadata
- digest and manifest metadata
- declared capabilities and declared requirements
- implementation form, WASM / Service / Native
- compatibility information
- validation-related metadata

This aligns with `../foundations/03-package-model.md`, where package taxonomy and implementation form are distinct dimensions.

#### B. Package matching

The service should support matching queries such as:

- which provider packages can satisfy a capability
- which packages are valid candidates under product default binding
- which candidates remain valid under feature-level binding
- which candidates are allowed under runtime override constraints

The Package Service does not redefine binding semantics. It provides the candidate and metadata surface that allows binding and resolve to be carried out in a more predictable way.

#### C. Dependency closure support

The service should own dependency graph knowledge and package closure construction support, including:

- package dependency expansion
- transitive package closure construction
- version and compatibility filtering
- dependency metadata needed by downstream resolve

This service-side responsibility exists to simplify the assembly path, not to erase Core's responsibility for final runtime resolution decisions.

The intended split is strict:

- the Package Service may compute and return candidate dependency closures
- the Package Service may pre-filter candidates by version, compatibility, and declared constraints
- Core still performs the formal resolve for a concrete instance
- Core still decides whether a candidate closure is legal under declaration, config, feature enablement, profile, policy, and runtime mutability rules

In other words, the Package Service is authoritative for **catalog knowledge and candidate closure construction**, while Core remains authoritative for **instance-specific final resolve**.

#### D. Download and artifact retrieval

The service should own artifact retrieval workflows, including:

- package artifact location
- artifact download coordination
- content digest verification metadata
- cacheable payload delivery
- package payload availability checks

This is especially important for wasm packages because wasm is the default implementation route for most extensible package shapes in vNext.

#### E. Update management

The service should own update-facing package metadata and package update orchestration support, including:

- available version discovery
- update channel or upgrade-policy-aware metadata
- dependency-aware update candidate calculation
- metadata needed to compare current lock inputs against newer package sets

This does **not** mean the service forces updates. It means the service becomes the formal place where update knowledge is maintained.

#### F. Validation metadata

The service should own validation-related metadata about package artifacts, for example:

- signatures
- digests
- manifest digests
- compatibility claims
- package build or publication metadata
- metadata used as input to later verification

The service provides validation metadata. Core still decides whether a candidate generation is verified strongly enough to activate.

#### G. Artifact distribution, especially wasm

The service should be the primary distribution boundary for runtime-loadable artifacts, especially:

- wasm package binaries
- package manifests needed for lock generation
- package payload bundles required for cold start or rollback replay

Wasm deserves explicit mention because it is the default extensibility route in vNext, and therefore the most common artifact type that benefits from centralized distribution.

### 5.2 What Core still owns

Core must continue to own the runtime truth and lifecycle decisions.

Core still owns:

- execution of formal runtime resolve
- instance state
- active generation state
- profile and policy application
- final binding legality checks
- verify decisions
- activate decisions
- rollback decisions

That boundary matters.

The Package Service may propose candidates, dependency closures, and artifact metadata. Core still decides whether a specific instance can move from declaration + config + candidate package set into a verified and activatable generation.

### 5.3 Why this split is important

Without this split, package transport and package truth can drift into one mixed local runtime concern. With the split:

- the Package Service answers **what is available, compatible, and retrievable**
- Core answers **what is legal, resolved for this instance, verified, active, and currently running**

That preserves the architectural role of Core while reducing package handling burden around it.

## 6. Relationship to existing vNext concepts

### 6.1 Package remains the sole source object

This proposal does not introduce a second source object.

Package remains the canonical published unit. The Package Service is the server-side system that indexes, matches, and distributes packages. It is not a new replacement for package declarations.

### 6.2 Instance remains the runtime object

Instance still carries:

- runtime config
- current lock
- active generation
- rollback history

The Package Service does not become an instance store. It only supplies the package-side information and artifacts that instances need in order to assemble or update.

### 6.3 Taxonomy remains provider / foundation / feature / product

The Package Service must understand taxonomy metadata because matching and dependency rules differ by role.

For example:

- provider packages are common binding candidates for capabilities
- foundation packages may shape stable capability surfaces
- feature packages influence composition and additional dependencies
- product packages define product identity and default intent

But the taxonomy itself remains part of the package model, not a new service-defined invention.

### 6.4 Declaration / config / lock remain separate

This proposal strengthens the separation from `04-declaration-config-lock.md` rather than weakening it.

- declaration still expresses product default intent
- config still expresses runtime-specific choices and overrides
- lock still records the immutable resolved result

The Package Service may provide metadata that helps generate a lock, but it does not replace the lock or absorb its meaning.

### 6.5 Binding / resolve stay formal

This proposal should be read alongside `05-binding-and-resolution.md`.

Binding still means which provider package satisfies a capability in a given assembled instance generation.

Resolve still means turning declaration, config, enabled features, policy, and package candidates into a candidate runtime graph and candidate lock.

The Package Service helps by making candidate discovery, dependency knowledge, and artifact retrieval much simpler. It does not remove the need for formal resolve.

### 6.6 Profile / policy stay in the runtime governance path

Profile and policy remain runtime governance inputs.

The Package Service may expose metadata that helps evaluate whether a package is allowed in a certain profile or policy regime, but profile and policy enforcement still belongs to Core and the runtime lifecycle path.

### 6.7 Verify / activate / rollback stay with Core

This proposal is intentionally strict here.

The Package Service may provide:

- package digests
- signatures
- publication metadata
- artifact availability
- prior validation metadata

But Core still performs the actual runtime-side gatekeeping for:

- verify
- activate
- rollback

That preserves the rule that the running system, not the package catalog alone, decides what becomes active.

### 6.8 Implementation forms remain WASM / Service / Native

The Package Service must understand implementation form metadata because distribution and retrieval differ by form.

Examples:

- wasm packages need runtime-loadable artifact distribution
- service packages may need endpoint, image, or managed-runtime metadata
- native packages may need platform and compatibility metadata

But WASM / Service / Native still remain implementation forms defined by the package model, not by the Package Service.

## 7. How the Package Service supports existing assembly concepts

The key requirement is support, not replacement.

### 7.1 Product default binding

Product packages still declare default bindings and mutability constraints.

The Package Service supports product default binding by:

- exposing product-declared candidate package metadata
- indexing which packages provide which capabilities
- returning candidate sets that satisfy product defaults and allowed-provider constraints

The Package Service does not redefine product identity or product defaults.

### 7.2 Feature-level binding

Feature packages still express stable composition blocks.

The Package Service supports feature-level binding by:

- indexing feature package dependency metadata
- exposing feature-induced capability requirements
- helping expand package closure after feature enablement

This keeps feature packages meaningful without forcing product packages to manually reason about every transitive dependency edge.

### 7.3 Runtime override

Runtime override still comes from instance config and still must stay inside declaration and policy boundaries.

The Package Service supports runtime override by:

- returning candidate alternatives for mutable or conditionally mutable bindings
- attaching compatibility and validation metadata to those candidates
- helping compare current lock state against the requested override path

The legal decision still belongs to Core.

### 7.4 Lock generation

Lock generation remains a Core-facing runtime result.

The Package Service supports lock generation by providing the package-side facts needed to produce a strong lock, including:

- exact package versions
- source metadata
- digest and manifest metadata
- dependency closure metadata
- artifact identity information

That makes lock generation more explicit and reproducible, while keeping the lock itself as the instance-side immutable record.

### 7.5 Rollback

Rollback remains an instance lifecycle action, not a package catalog action.

The Package Service supports rollback by:

- retaining artifact identity and retrievability metadata for previously locked packages
- enabling reliable re-fetch or re-distribution of prior wasm artifacts
- exposing historical compatibility and provenance metadata needed to replay an earlier lock

Core still decides whether rollback is valid and whether the rolled-back generation can become active.

### 7.6 Offline replay and rollback

This proposal must not weaken local-runtime guarantees.

If an instance already holds a valid lock plus the required local artifacts, lock replay and rollback should remain possible **without live Package Service availability**.

That means the Package Service should be understood as the canonical boundary for:

- package catalog truth
- package supply
- candidate closure construction
- artifact distribution and re-distribution

But it should **not** be understood as a mandatory online authority at activation time for an already locked and locally materialized generation.

If the runtime already has the lock-pinned artifacts needed by a prior generation, Core should be able to verify, replay, or roll back locally according to existing instance and lifecycle rules.

## 8. Suggested interaction model

This section describes an architectural direction, not a fixed API contract.

### 8.1 High-level flow

1. Core reads product package declaration and instance config.
2. Core determines the requested feature set, binding intent, profile, and policy context.
3. Core queries the Package Service for candidate packages, dependency closure information, and artifact metadata.
4. Core performs formal resolve using declaration, config, profile, policy, and Package Service results.
5. Core verifies the candidate generation.
6. If accepted, Core writes instance lock and activates the new generation.
7. When runtime payloads are needed, especially wasm artifacts, Core or runtime fetches them through the Package Service distribution path.

### 8.2 What should stay out of the client surface

Following the runtime boundary established in `../products/weft-code/14-runtime-model.md`, clients should not directly carry package-index complexity.

The intended layering is:

- clients talk to runtime
- runtime and Core talk to the Package Service as needed
- package source and artifact complexity stay behind those boundaries

This keeps package logistics out of CLI / TUI / GUI business truth.

## 9. Scope boundaries

### 9.1 In scope for this proposal

- a dedicated server-side Package Service boundary
- package index and metadata authority
- matching support for capability and binding candidate selection
- candidate dependency-closure support
- package download and update management
- validation metadata serving
- artifact distribution, especially wasm package delivery
- explicit separation between Package Service ownership and Core ownership

### 9.2 Out of scope for this proposal

- redefining Package, Instance, or Core
- removing feature composition from resolve
- replacing declaration / config / lock with a service-owned model
- moving verify / activate / rollback out of Core
- specifying a final transport protocol or endpoint schema
- specifying the exact storage backend for the service
- claiming that every package form is handled identically

## 10. Non-goals

This proposal is **not** trying to do the following:

1. Turn the Package Service into a new runtime control plane
2. Hide or erase binding and resolve as architectural concepts
3. Make the lock optional
4. Make rollback a best-effort artifact cache trick
5. Collapse product package and feature package semantics into a package marketplace view
6. Replace local runtime truth with server-side package truth
7. Rewrite the whole vNext architecture around package distribution

## 11. Open questions

The boundary is useful even before every detail is settled. The main open questions are:

1. **How rich should candidate closure construction become inside the service?**
   The service should at least provide dependency closure knowledge, but the final limit between service-side candidate closure construction and Core-local final resolve still needs a stricter contract.

2. **What is the right caching model for wasm artifacts?**
   A dedicated distribution boundary helps, but the exact division between server cache, local runtime cache, and lock-pinned replay still needs to be designed.

3. **How should Service-form packages be represented for distribution?**
   Wasm distribution is the clearest immediate use case. Service-form packages may need a different retrieval and compatibility model.

4. **How should update policy interact with profile and policy enforcement?**
   The Package Service can expose update candidates, but the governance boundary between package update metadata and runtime policy decisions needs a stricter contract.

5. **How much historical metadata is required to make rollback operationally strong?**
   A lock alone is not enough unless earlier artifacts remain identifiable and retrievable.

6. **Should product packages consume the Package Service directly during authoring workflows, or only through Core-facing tooling?**
   This affects how much package complexity leaks into authoring and AI-assisted editing paths.

## 12. Architectural conclusion

The vNext model already has the right primary objects:

- Package as the sole source object
- Instance as the runtime object

What is still missing is a cleaner package supply boundary.

A dedicated server-side Package Service is the most direct way to provide that boundary.

It centralizes package index, metadata, matching, candidate dependency-closure knowledge, download, update management, validation metadata, and artifact distribution, especially for wasm packages. At the same time, it leaves Core in charge of formal runtime resolve execution, instance state, and verify / activate / rollback decisions.

That split makes package usage simpler without weakening the existing vNext architecture. It makes package logistics more explicit, keeps runtime truth where it belongs, preserves offline lock replay and rollback for already materialized generations, and gives wasm-first distribution a proper architectural home.

## 13. Related documents

- `../foundations/03-package-model.md`
- `04-declaration-config-lock.md`
- `05-binding-and-resolution.md`
- `06-profiles-and-policy.md`
- `../foundations/13-product-package-instance-model.md`
- `../products/weft-code/14-runtime-model.md`
