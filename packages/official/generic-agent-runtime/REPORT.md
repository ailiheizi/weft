# GenericAgent Runtime Experiment Report

## Goal

Build an isolated local prototype of a GenericAgent-style runtime as a WEFT
service plugin, without modifying WEFT core behavior, companion cognition, or
memory-runtime.

## Why this module exists

GenericAgent's strongest ideas are not its exact Python implementation, but its
execution model:

- minimal execution loop
- layered memory
- plan -> run -> verify -> crystallize workflow
- growing reusable skills from successful execution

This experiment tests whether those ideas can fit WEFT as a standalone module.

## Isolation guarantees

- no WEFT core request-pipeline modification
- no change to `companion-core`
- no change to `memory-runtime`
- no registration into the existing formal skill library
- runs as its own independent service package on port `43133`

## Implemented prototype scope

This prototype currently exposes five actions through `/webhook`:

1. `plan_task`
2. `run_task`
3. `verify_task`
4. `crystallize_skill`
5. `get_runtime_state`

Phase 1 was intentionally structural and deterministic. In Phase 2, `run_task`
can now optionally execute a real WEFT tool through WEFT-core's package call
API while still remaining fully isolated from WEFT core code changes.

## Architecture

### `server.py`

Service entry point. Hosts:

- `GET /health`
- `POST /webhook`

### `ga_runtime/planner.py`

Builds a GenericAgent-style plan with four phases:

- explore
- execute
- verify
- crystallize

### `ga_runtime/runner.py`

Runs a minimal deterministic execution loop and can optionally bridge into real
WEFT tool execution.

### `ga_runtime/bridge.py`

Calls WEFT-core's package call API at:

- `/api/plugins/tool-runtime-core/call`

This keeps the experiment isolated: the GenericAgent runtime consumes WEFT as
an external boundary instead of modifying WEFT internals.

### `ga_runtime/verifier.py`

Performs prototype-level structural verification.

### `ga_runtime/crystallizer.py`

Generates a reusable skill/SOP draft from a successful run and stores it in
`data/skill-drafts/`.

### `ga_runtime/storage.py`

Persists plans, runs, verifications, and crystallized drafts into
`data/runtime-state.json`.

## Validation performed

Local server startup:

- service started successfully on `127.0.0.1:43133`

API checks completed successfully:

- `/health`
- `plan_task`
- `run_task`
- `verify_task`
- `crystallize_skill`

Phase 2 bridge validation completed successfully:

- `run_task` accepts `tool` + `args`
- bridge path targets `tool-runtime-core` through WEFT-core package call API
- local health and webhook path stay unchanged
- bridge execution returns real tool output when WEFT-core is aligned and `tool-runtime-core` is mounted

Observed outputs:

- task plan returned expected four-phase structure
- run result returned deterministic loop trace
- verify result returned `PASS`
- crystallize result wrote skill draft markdown to local data directory
- bridge configuration is in place for real WEFT tool execution through HTTP API
- live bridge invocation now records successful tool execution attempts and results

## What this proves

This proves that WEFT can host a GenericAgent-inspired runtime as an isolated,
modular service package without touching the existing core architecture.

## Adaptation to WEFT

This experiment is a strong architectural fit for WEFT because it maps cleanly
onto WEFT's existing decomposition instead of fighting it.

### What GenericAgent contributes conceptually

- a tiny execution loop
- a strict `plan -> run -> verify -> crystallize` workflow
- task success turning into reusable skill drafts
- a preference for small atomic tools over large hard-coded workflows

### What WEFT already has

- `tool-runtime-core` for atomic tool execution
- `skills-runtime` for reusable skills
- `memory-runtime` for durable memory and wiki storage
- `agent-core` for session and agent state
- `companion-core` for social layer and user-facing interaction
- `cron` and `context-engine` for proactive triggers and background flows

### Why the fit is good

GenericAgent does **not** need to replace WEFT's core. Instead, it fits best as
an additional runtime mode that consumes WEFT's existing services:

- WEFT keeps the stable platform surface
- GenericAgent-style runtime adds self-evolving task execution behavior
- successful execution paths can later become WEFT skill drafts

That means WEFT can gain GenericAgent's strongest ideas without sacrificing the
stability of the current companion, memory, and session model.

### Why isolation matters

Keeping this experiment as a separate service package is important because:

- it avoids destabilizing the current companion path
- it lets the team test GenericAgent-style workflows without rewriting WEFT-core
- it makes rollback trivial: disable or remove one plugin
- it allows gradual promotion of ideas instead of a large risky merge

This is especially important for WEFT because the current product already has
sensitive mechanisms around companion behavior, memory, and session routing.

## User Benefits

If this module evolves beyond prototype, the main user-facing benefits are:

### 1. Faster repeated task execution

Today many complex tasks require fresh planning every time. A GenericAgent-style
runtime can turn a successful execution path into a reusable draft, reducing the
need to rediscover the same workflow repeatedly.

### 2. Better long-task handling

This model is naturally aligned with tasks that require:

- exploration
- retries
- explicit verification
- converting the result into a reusable method

That makes it a strong fit for "do something new, then make it repeatable"
scenarios.

### 3. Lower operating cost over time

The more successful paths are crystallized, the less the system needs to spend
tokens on rediscovering solutions. This matches WEFT's broader need to keep
high-agency execution efficient over long use.

### 4. Safer experimentation for power users

Because this runtime is isolated, advanced users can try self-evolving task
execution without changing the default WEFT interaction path.

### 5. A path from execution to productized capability

The strongest long-term value is not just that the runtime can do tasks, but
that it can eventually generate artifacts that WEFT users can review and adopt
as reusable skills or SOPs.

In other words, this module can help WEFT move from:

- one-off execution

to:

- reusable, reviewable, user-owned capability growth

It still does **not** yet prove:

- broad real-tool execution stability across multiple WEFT tools
- real memory integration with durable namespaces
- safe promotion into production workflow

## Recommended next steps

### Phase 2 status

Completed in working form:

- `run_task` can target WEFT tool-runtime through WEFT-core package call API
- execution trace records bridged tool execution intent and result location
- live bridge validation succeeded for `fs_read`, `fs_list`, `shell_exec`, and `web_fetch`

Still needed:

- richer tool selection logic
- stronger verification of real tool outcomes
- robust multi-step bridged execution

### Phase 3

Add optional draft export into WEFT skill workflows:

- generate skill draft markdown
- expose draft review path
- keep manual approval before any formal registration

## Recommendation for repo placement

If this experiment is kept:

- local prototype path: `weft-plugins/generic-agent-runtime`
- upstream target if accepted later: `weft-plugins/generic-agent-runtime`

Do **not** merge into `WEFT-core` or existing runtime plugins directly.

## Current verdict

Prototype is successful as an isolated local experiment.
It is suitable for discussion and further iteration, but not ready for direct
promotion into mainline production behavior.
