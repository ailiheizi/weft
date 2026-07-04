# ADR 0003: Package Server Client Experience Boundary

## Status

Accepted

## Context

Current WEFT clients derive important runtime views from local state:

- product package manifests
- product lock files
- managed suite lock files
- install plan artifacts
- product graph
- runtime sync state
- Core active state

The Package Server will add richer product catalog, package detail, provider candidate, install/update preview, artifact metadata, verification metadata, and review/publication state. Without a clear UI boundary, users may confuse server previews with local runtime truth.

## Decision

Client experiences must present two fact planes:

1. **Package-side facts** from Package Server.
2. **Runtime-side truth** from local Core and local runtime state.

Package-side facts include:

- catalog entries
- package details
- provider candidates
- dependency previews
- install/update previews
- artifact availability
- verification metadata availability
- candidate, review, and publication status

Runtime-side truth includes:

- final resolved bindings
- local locks
- active generation
- product graph
- active plugins
- runtime sync state
- health
- rollback availability

The UI must use different labels for these states.

Recommended package-side labels:

```text
Catalog
Candidate
Preview
Plan
Reviewable
Approved
Published
Yanked
Deprecated
Artifact Available
Verification Metadata Available
```

Recommended runtime-side labels:

```text
Resolved
Locked
Verified
Materialized
Active
Rollback Available
Runtime Healthy
Runtime Degraded
```

## User-facing copy

Install preview copy should say:

```text
This is a server-generated install preview. Local Core will resolve, verify, lock, and activate before runtime changes are applied.
```

Candidate provider copy should say:

```text
This is an available provider candidate, not the current binding. Current binding is determined by the local lock.
```

Server unavailable copy should say:

```text
Package Server is unavailable. Installed products can continue using local runtime state. New installs, updates, and missing artifact fetches may be unavailable.
```

## Consequences

Positive:

- Users can discover and preview product changes without losing local runtime confidence.
- Developers and reviewers get package lifecycle views without implying installation.
- Server downtime does not erase local product state from the client.

Negative:

- Client UI must carry more precise state labels.
- Some screens need both server-backed and local-backed sections.

## Related documents

- `../runtime/20-product-package-server-detailed-plan.md`
- `0001-product-package-server-authority.md`
- `0002-package-server-review-publication-boundary.md`
