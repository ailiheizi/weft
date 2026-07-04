# Memory Runtime Precise Long-Term Memory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade `memory-runtime` from durable searchable storage into a unified precise long-term memory mechanism with fact state, contradiction handling, and session extraction.

**Architecture:** Keep one fact object model in PostgreSQL. Extend `memory_entries` with stable state fields, reconcile same `entity_key + facet` facts on write, and make provider lifecycle hooks persist extracted facts. Verify through the host runtime script instead of UI or main-process changes.

**Tech Stack:** Python, PostgreSQL via `psycopg`, PowerShell service bootstrap, TypeScript host test via `tsx`

---

### Task 1: Lock The Desired Runtime Contract In Tests

**Files:**
- Modify: `weft/scripts/test-weft-memory-runtime-host.ts`

- [ ] Add assertions for new fact fields on `fact_store(list/search)`: `entity_key`, `facet`, `validity`, `source_kind`, `evidence`.
- [ ] Add a red test for version flow: write legacy backend fact, then current backend fact, then assert old fact becomes `superseded` and new fact stays `active`.
- [ ] Add a red test for contradiction flow: write two conflicting same-facet preference facts, then assert `contradict` returns them.
- [ ] Add a red test for session extraction via webhook lifecycle action and assert extracted facts are searchable.
- [ ] Run `npx tsx --no-warnings D:\weft\weft\scripts\test-weft-memory-runtime-host.ts` and confirm it fails for the new contract.

### Task 2: Implement Unified Fact State In PostgreSQL Store

**Files:**
- Modify: `weft-plugins/memory-runtime/hermes_port/postgres_store.py`

- [ ] Extend schema with fact-state columns and indexes: `entity_key`, `facet`, `validity`, `supersedes_fact_id`, `source_kind`, `evidence`.
- [ ] Add normalization/inference helpers for `entity_key`, `facet`, and stateful fact metadata.
- [ ] Update `add_fact`, `update_fact`, `search`, `list_facts`, and `explain_search` to read/write the new fields.
- [ ] Implement same-entity same-facet reconciliation so additions become duplicate/update, supersession, or contradiction.
- [ ] Implement `contradict` with deterministic same-object/facet conflict detection.

### Task 3: Implement Provider-Level Session Extraction

**Files:**
- Modify: `weft-plugins/memory-runtime/hermes_port/postgres_provider.py`
- Modify: `weft-plugins/memory-runtime/server.py`

- [ ] Extend tool args handling so `fact_store(add/update/search/list)` can accept and return the new fact fields.
- [ ] Implement a shared extraction path used by both `sync_turn` and `on_session_end`.
- [ ] Add webhook actions for `sync_turn` and `session_end` so the host test can exercise lifecycle extraction.
- [ ] Mirror builtin memory writes into PostgreSQL with explicit `source_kind='mirror'`.

### Task 4: Runtime Hygiene And Verification

**Files:**
- Add or Modify: `weft-plugins/memory-runtime/.gitignore`
- Modify: `weft-plugins/memory-runtime/start-memory-runtime.ps1`
- Modify: `weft-plugins/memory-runtime/plugin.toml`

- [ ] Ignore generated Python caches and local dependency folders.
- [ ] Ensure packaged runtime still includes only source assets plus declared requirements.
- [ ] Re-run host test and any required Python syntax checks.
- [ ] Re-run `npm run build` in `D:\weft\weft`.
- [ ] Review `git diff --stat` to ensure scope stays inside `memory-runtime` and its host test.

