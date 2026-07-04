# ADR 0001: Product Package Server Authority Boundary

## Status

Accepted

## Context

WEFT is moving from local package metadata scaffolding toward a self-hosted Product Package Server. Current server code already exposes product catalog, package detail, bind preview, dependency tree preview, install/update plan preview, artifact and verification metadata preview, draft, candidate, review, and ingestion routes.

The architectural risk is authority drift. If the server starts returning final resolved bindings, runtime locks, active generations, activation results, or rollback decisions, it becomes a remote Core and conflicts with the vNext package/instance model.

The current client also depends on local runtime truth: product graph, suite lock, install plan artifacts, runtime sync state, product locks, and active Core state. Moving runtime authority to the server would break this model and make replay/rollback depend on live server availability.

## Decision

The Package Server is a **Product Package Server** whose authority is limited to package supply and planning facts.

The server owns:

- product and dependency package catalog records
- package metadata and manifest-derived records
- capability provider candidates
- dependency closure candidates
- bind, install, update, and rollback-support previews
- artifact metadata and later immutable artifact downloads
- verification metadata
- draft, candidate, review, ingestion, and publication metadata

Core owns:

- final binding selection
- final resolve
- lock generation and lock writes
- verification decision for an instance
- activation decision
- rollback decision
- active generation truth
- local materialization and offline replay

All server planning outputs must be explicitly advisory until Core produces a local lock.

## Consequences

Positive:

- Server APIs can become stronger without becoming runtime authority.
- Client can use server facts while preserving local runtime state.
- Package authoring and review workflows can evolve independently from Core activation.
- Offline replay and rollback remain possible for local materialized generations.

Negative:

- Some data may be duplicated between server previews and Core locks.
- Core integration must translate server plans into Core resolve inputs.
- UI must clearly distinguish package-side preview from runtime-side truth.

## Enforcement

Package Server responses must not expose these runtime-owned fields:

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

Preview and plan responses must include vocabulary such as:

```text
advisory
preview
metadata_only
core_ownership
```

where applicable.

## Related documents

- `../runtime/16-package-service.md`
- `../runtime/17-package-service-implementation-plan.md`
- `../runtime/18-self-hosted-package-server-blueprint.md`
- `../runtime/20-product-package-server-detailed-plan.md`
