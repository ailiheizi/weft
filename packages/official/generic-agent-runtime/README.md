# generic-agent-runtime

Experimental WEFT service package inspired by GenericAgent.

This module is intentionally isolated from WEFT core behavior. It does not
replace the existing companion, memory, or skill runtime paths. It is a local
prototype for evaluating whether a self-evolving task runtime should become a
first-class WEFT module later.

## Scope

- standalone service plugin
- no changes to WEFT core request pipeline
- no changes to companion cognition or memory-runtime
- supports four experimental actions:
  - `plan_task`
  - `run_task`
  - `verify_task`
  - `crystallize_skill`

## Phase 2 tool bridge

The local prototype can now optionally execute a real WEFT tool through
WEFT-core's package call API without modifying WEFT core internals.

- target plugin: `tool-runtime-core`
- transport: `POST /api/plugins/tool-runtime-core/call`
- default WEFT core base URL: `http://127.0.0.1:17830`

Example:

```json
{
  "action": "run_task",
  "data": {
    "task": "Read a local file through the isolated GenericAgent runtime",
    "tool": "fs_read",
    "args": {
      "path": "D:\\weft-workspace\\weft-plugins\\generic-agent-runtime\\README.md"
    }
  }
}
```

## API

POST `/webhook`

```json
{
  "action": "plan_task",
  "data": {
    "task": "Create a reusable workflow for summarizing release notes",
    "session_id": "session-123",
    "workspace_id": "D:\\workspace"
  }
}
```

## Actions

### `plan_task`
Builds a GenericAgent-style execution plan with exploration, execution,
verification, and crystallization phases.

### `run_task`
Runs a minimal deterministic task loop over the provided task description and
returns a task state snapshot. In Phase 2 it can optionally bridge into real
WEFT tool execution when `tool` + `args` are supplied.

### `verify_task`
Evaluates whether the provided execution result satisfies the stated intent.

### `crystallize_skill`
Produces a reusable skill draft and SOP-style artifact from a successful run.

## Local Testing

```powershell
cd D:\weft-workspace\weft-plugins\generic-agent-runtime
py -3 server.py
```

Health:

```powershell
Invoke-WebRequest http://127.0.0.1:43133/health | Select-Object -Expand Content
```

Plan action:

```powershell
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:43133/webhook -ContentType 'application/json' -Body '{"action":"plan_task","data":{"task":"Summarize release notes"}}'
```

## Why this exists

GenericAgent's strongest ideas are:

- tiny execution loop
- layered memory
- plan -> run -> verify -> crystallize workflow
- skill growth from successful task execution

This module tests whether those ideas fit WEFT as an isolated runtime mode
before any discussion of upstream integration.
