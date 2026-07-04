# Memory Runtime MemPalace Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deploy official MemPalace locally and add a minimal bridge so memory-runtime can write to and search verbatim MemPalace drawers.

**Architecture:** Keep `PostgresMemoryProvider` as the runtime service owner, but add a small `MemPalaceBridge` that manages local Chroma drawer storage and raw retrieval. First phase adds deployment readiness, bridge write/search, and focused runtime tests only.

**Tech Stack:** Python, ChromaDB, local MemPalace checkout, memory-runtime service, PowerShell focused verification

---

### Task 1: Lock A MemPalace Bridge Contract In Tests

**Files:**
- Create: `weft/scripts/test-weft-memory-runtime-mempalace-bridge.py`

- [ ] **Step 1: Write the failing test**

Create a focused script that:

- creates a temp runtime workspace
- enables bridge env vars
- loads `memory-runtime`
- writes one durable memory record
- queries the new bridge search action
- expects verbatim content and bridge metadata

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
py -3.13 D:\weft\weft\scripts\test-weft-memory-runtime-mempalace-bridge.py
```

Expected: FAIL because the bridge module and dispatch path do not exist yet.

### Task 2: Add The Bridge Module

**Files:**
- Create: `weft-plugins/memory-runtime/hermes_port/mempalace_bridge.py`

- [ ] **Step 1: Implement minimal bridge**

Add:

- `MemPalaceBridgeConfig`
- `MemPalaceBridge.ensure_ready()`
- `MemPalaceBridge.ingest_memory_entry(...)`
- `MemPalaceBridge.search(...)`

The bridge should use local `chromadb` directly and store runtime entries as verbatim drawer documents with simple `wing` / `room` metadata derived from existing entry fields.

- [ ] **Step 2: Keep the bridge optional**

If env is not enabled, bridge creation should return `None` and runtime behavior must stay unchanged.

### Task 3: Wire The Bridge Into Postgres Memory Provider

**Files:**
- Modify: `weft-plugins/memory-runtime/hermes_port/postgres_provider.py`

- [ ] **Step 1: Attach bridge lifecycle**

Initialize the bridge during provider startup when enabled.

- [ ] **Step 2: Mirror writes**

When facts are written successfully, mirror them into MemPalace drawers.

- [ ] **Step 3: Expose bridge search**

Add a narrow tool action for searching bridge verbatim drawers and returning normalized results.

### Task 4: Expose The Bridge Through The Runtime Service

**Files:**
- Modify: `weft-plugins/memory-runtime/server.py`

- [ ] **Step 1: Route a runtime action**

Expose a dispatch action that calls the provider-side bridge search path.

- [ ] **Step 2: Keep existing actions stable**

Do not change non-bridge runtime behavior.

### Task 5: Verify Focused End-To-End Behavior

**Files:**
- Verify:
  - `weft/scripts/test-weft-memory-runtime-mempalace-bridge.py`

- [ ] **Step 1: Run the bridge test**

Run:

```powershell
py -3.13 D:\weft\weft\scripts\test-weft-memory-runtime-mempalace-bridge.py
```

Expected: PASS

- [ ] **Step 2: Re-run existing nearby focused tests**

Run:

```powershell
py -3.13 D:\weft\weft\scripts\test-weft-memory-runtime-file-backed-fact-store.py
py -3.13 D:\weft\weft\scripts\test-weft-memory-runtime-wiki-dispatch.py
```

Expected: PASS

- [ ] **Step 3: Scope check**

Run:

```powershell
git -C D:\weft diff --stat
```

Confirm the change stays inside:

- `docs/superpowers/*`
- `weft-plugins/memory-runtime/hermes_port/*`
- `weft-plugins/memory-runtime/server.py`
- `weft/scripts/test-weft-memory-runtime-mempalace-bridge.py`
