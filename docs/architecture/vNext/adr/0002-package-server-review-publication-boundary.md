# ADR 0002: Package Server Review and Publication Boundary

## Status

Accepted

## Context

The Package Server includes upload drafts, AI drafts, ingestion candidates, package candidates, candidate events, review queue, approval, and rejection routes. These flows are necessary to turn package proposals into catalog-visible package supply records.

The risk is conflating review approval with runtime installation. A package can be safe enough to publish as catalog metadata but still unsuitable for a specific instance after local policy, profile, feature selection, binding overrides, artifact availability, or compatibility constraints are applied.

## Decision

Review and publication are package supply actions only.

Approved or published package metadata means:

- the package is eligible to appear in the server catalog
- the package may become a candidate in server planning responses
- artifact and verification metadata may be exposed to Core

Approved or published package metadata does not mean:

- the package is installed
- the package is selected for a concrete binding
- a local lock has been generated
- artifacts have been verified for a local instance
- a runtime generation has been activated

Candidate promotion from upload, AI draft, or ingestion must go through reviewable package candidate states. Publication mutates catalog metadata only.

## Candidate state model

The long-term candidate lifecycle should use these states:

```text
draft
quarantined
needs_more_input
reviewable
approved
rejected
withdrawn
promoted
published_metadata
yanked
deprecated
```

State transitions must be auditable.

## Publication rules

Publication requires:

- approved candidate state
- complete package identity
- complete package kind and runtime metadata
- known manifest digest when available
- no blocking validation issues
- explicit version conflict handling

Publication must not:

- silently overwrite immutable versions
- write instance locks
- activate runtime generations
- call Core to make catalog mutation succeed

## Consequences

Positive:

- Reviewers can approve supply metadata without becoming runtime operators.
- Developers receive clear package-level feedback.
- Core remains the only final authority for local instance acceptance.

Negative:

- Users may need UI copy explaining that published packages are not installed packages.
- Server and client must display supply state separately from runtime state.

## Related documents

- `../runtime/18-self-hosted-package-server-blueprint.md`
- `../runtime/20-product-package-server-detailed-plan.md`
- `0001-product-package-server-authority.md`
