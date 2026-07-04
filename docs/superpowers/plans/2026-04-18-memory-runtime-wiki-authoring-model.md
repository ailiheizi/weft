# Memory Runtime Wiki Authoring Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make wiki the only durable authoring surface for memory, compile every authored memory shape into palace drawers, and add bullet-journal, scope, timestamp, and workspace-reference weighting without reintroducing palace-direct authoring.

**Architecture:** All durable memory enters through wiki authoring contracts only. `article`, `memo`, and `artifact` remain the three authoring units, but they gain `scope`, `workspace_id`, `page_family`, and block-level timestamps. Retrieval still queries palace only, but ranking becomes scope-aware and time-aware, and wiki pages become the canonical editable source for both raw bullet notes and distilled reference pages.

**Tech Stack:** Python, asyncpg, Chroma-backed MemPalace, FastMCP wiki tools, pytest, Markdown wiki pages

---

### Task 1: Lock the new authoring contract with failing tests

**Files:**
- Modify: `weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_write_contract.py`
- Modify: `weft-plugins/memory-runtime/wiki/tests/unit/test_wiki_palace_sync_service.py`

- [ ] **Step 1: Add a failing write-contract test for scope and page family metadata**

Add a test that creates a wiki page through `_create_note(...)` and expects metadata like:

```python
assert json.loads(metadata) == {
    "authoring_unit": "memo",
    "scope": "workspace",
    "workspace_id": "weft",
    "page_family": "bullet-journal",
}
```

- [ ] **Step 2: Run the write-contract test and verify it fails**

Run:

```powershell
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_write_contract.py -k "scope or page_family" -q
```

Expected: FAIL because `write.py` does not accept or persist `scope`, `workspace_id`, or `page_family` yet.

- [ ] **Step 3: Add a failing sync test for block timestamps and scope metadata**

Add a test that compiles a bullet-journal page and expects every semantic block to retain:

```python
assert block["authoring_unit"] == "memo"
assert block["scope"] == "workspace"
assert block["workspace_id"] == "weft"
assert block["page_family"] == "bullet-journal"
assert block["observed_at"] == "2026-04-18T10:42:00Z"
```

- [ ] **Step 4: Run the sync test and verify it fails**

Run:

```powershell
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_wiki_palace_sync_service.py -k "bullet_journal or observed_at or workspace_id" -q
```

Expected: FAIL because `wiki_semantic_adapter.py` does not compile those metadata fields into palace blocks yet.

- [ ] **Step 5: Commit after implementation is complete**

```bash
git add weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_write_contract.py weft-plugins/memory-runtime/wiki/tests/unit/test_wiki_palace_sync_service.py
git commit -m "test: lock wiki authoring metadata contract"
```

### Task 2: Extend wiki create/append/edit to support bullet-journal authoring

**Files:**
- Modify: `weft-plugins/memory-runtime/wiki/mcp/tools/write.py`
- Modify: `weft-plugins/memory-runtime/wiki/mcp/tools/wiki_contract.py`
- Modify: `weft-plugins/memory-runtime/wiki/mcp/tools/guide.py`

- [ ] **Step 1: Add normalized enums and metadata merge helpers**

Implement support for:

```python
_SCOPES = {"global", "workspace"}
_PAGE_FAMILIES = {"reference", "topic", "bullet-journal"}
```

and helpers:

```python
def _normalize_scope(value: str) -> str: ...
def _normalize_page_family(value: str) -> str: ...
def _merge_wiki_authoring_metadata(existing: Any, authoring_unit: str, scope: str, workspace_id: str, page_family: str) -> dict[str, Any]: ...
```

- [ ] **Step 2: Extend `write(...)` create signature with scope/page-family/workspace fields**

Update the MCP tool signature so `create` accepts:

```python
scope: Literal["global", "workspace", ""] = ""
workspace_id: str = ""
page_family: Literal["reference", "topic", "bullet-journal", ""] = ""
observed_at: str = ""
```

- [ ] **Step 3: Add path defaults for bullet journals**

When AI writes a tiny operational memory, normalize the path into one of:

```text
/global/bullet-journal/YYYY-MM.md
/workspaces/<workspace_id>/bullet-journal/YYYY-MM.md
```

Use current UTC month if the page filename is not explicitly provided.

- [ ] **Step 4: Add a minimal bullet-journal markdown spine**

For `memo + bullet-journal`, normalize content into:

```md
# WEFT Bullet Journal 2026-04

## 2026-04-18

- [10:42] User prefers wiki-first authoring
  - tags: architecture, memory
  - source: chat
  - scope: workspace
  - workspace: weft
  - observed_at: 2026-04-18T10:42:00Z
```

- [ ] **Step 5: Preserve authoring metadata through append/edit**

Update append and edit paths so they preserve:

```python
{
    "authoring_unit": "...",
    "scope": "...",
    "workspace_id": "...",
    "page_family": "...",
}
```

- [ ] **Step 6: Update user-facing guide and contract text**

Make the guide explicitly say:

```text
Wiki is the only durable authoring surface.
Use memo + bullet-journal for fragmented notes.
Use article + reference/topic for distilled knowledge.
Use artifact for heavy files.
```

- [ ] **Step 7: Run focused tests**

Run:

```powershell
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_write_contract.py -q
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_wiki_contract.py weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_tools_registry.py -q
```

Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add weft-plugins/memory-runtime/wiki/mcp/tools/write.py weft-plugins/memory-runtime/wiki/mcp/tools/wiki_contract.py weft-plugins/memory-runtime/wiki/mcp/tools/guide.py weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_write_contract.py weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_wiki_contract.py weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_tools_registry.py
git commit -m "feat: add wiki bullet journal authoring contract"
```

### Task 3: Compile scope, workspace, and timestamps into palace blocks

**Files:**
- Modify: `weft-plugins/memory-runtime/wiki/api/services/wiki_semantic_adapter.py`
- Modify: `weft-plugins/memory-runtime/wiki/api/services/wiki_palace_sync.py`

- [ ] **Step 1: Extend canonical bundle metadata**

In `document_to_canonical_bundle(...)`, attach:

```python
bundle = {
    ...,
    "authoring_unit": authoring_unit,
    "scope": scope,
    "workspace_id": workspace_id,
    "page_family": page_family,
    "observed_at": observed_at,
    "created_at": created_at,
    "updated_at": updated_at,
}
```

- [ ] **Step 2: Carry metadata into each semantic block**

Extend `bundle_to_semantic_blocks(...)` so every block gets:

```python
{
    "authoring_unit": authoring_unit,
    "scope": scope,
    "workspace_id": workspace_id,
    "page_family": page_family,
    "observed_at": observed_at,
    "created_at": created_at,
    "updated_at": updated_at,
}
```

- [ ] **Step 3: Keep internal wiki links weak**

Preserve the current behavior:

```python
if block["role"] == "reference" and block["link_scope"] == "internal":
    return "Body: Internal wiki reference"
```

Do not let internal cross-links become high-value semantic anchors again.

- [ ] **Step 4: Persist new metadata in Chroma drawer metadatas**

Update `upsert_semantic_block(...)` so palace metadata stores:

```python
"scope": ...,
"workspace_id": ...,
"page_family": ...,
"observed_at": ...,
"created_at": ...,
"updated_at": ...,
```

- [ ] **Step 5: Reflect metadata back out of cluster expansion and wiki view**

Update:

```python
expand_cluster(...)
build_wiki_view_from_cluster_rows(...)
```

so the returned wiki view pages/blocks still know their scope, page family, and timestamps.

- [ ] **Step 6: Run focused sync tests**

Run:

```powershell
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_wiki_palace_sync_service.py -k "bullet_journal or authoring_unit or observed_at or internal_wiki_cross_links" -q
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add weft-plugins/memory-runtime/wiki/api/services/wiki_semantic_adapter.py weft-plugins/memory-runtime/wiki/api/services/wiki_palace_sync.py weft-plugins/memory-runtime/wiki/tests/unit/test_wiki_palace_sync_service.py
git commit -m "feat: compile wiki scope and timestamps into palace blocks"
```

### Task 4: Make retrieval scope-aware and time-aware

**Files:**
- Modify: `weft-plugins/memory-runtime/wiki/api/services/wiki_semantic_adapter.py`
- Modify: `weft-plugins/memory-runtime/hermes_port/palace_provider.py`

- [ ] **Step 1: Add scope-weighting helpers**

Implement helpers like:

```python
def _scope_weight(metadata: dict[str, Any], active_workspace_id: str | None) -> float: ...
def _recency_weight(metadata: dict[str, Any]) -> float: ...
```

Rules:

```text
workspace reference > workspace topic > workspace bullet-journal > global > other workspace
```

- [ ] **Step 2: Apply weights in wiki-side retrieval**

Update:

```python
resolve_cluster_id_by_consensus(...)
combined_score(...)
```

so score becomes:

```python
score = lexical + semantic_bonus + scope_weight + recency_weight
```

and support/reference links still stay below primary text for broad comparison queries.

- [ ] **Step 3: Add workspace context to provider-side prefetch**

Adjust `palace_provider.py` so prefetch can pass a current workspace hint into palace-side wiki retrieval projection metadata when available, without reintroducing direct palace authoring.

- [ ] **Step 4: Add focused tests for scope-aware ranking**

Create or extend tests so these expectations hold:

```python
assert result["resolved_cluster_id"] == "cluster-weft-reference"
assert top_clusters[0] == "cluster-weft-reference"
```

when the active workspace is `weft` and both global and foreign-workspace pages exist.

- [ ] **Step 5: Run focused ranking tests**

Run:

```powershell
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_wiki_palace_sync_service.py -k "scope or chinese_broad_query or external_support_links" -q
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add weft-plugins/memory-runtime/wiki/api/services/wiki_semantic_adapter.py weft-plugins/memory-runtime/hermes_port/palace_provider.py weft-plugins/memory-runtime/wiki/tests/unit/test_wiki_palace_sync_service.py
git commit -m "feat: add scope-aware palace ranking for wiki memory"
```

### Task 5: Remove palace-direct durable authoring from the AI path

**Files:**
- Modify: `weft-plugins/memory-runtime/hermes_port/palace_provider.py`
- Modify: `weft-plugins/memory-runtime/wiki/mcp/tools/guide.py`
- Modify: `weft-plugins/memory-runtime/wiki/mcp/tools/wiki_contract.py`

- [ ] **Step 1: Make provider prompt prefer wiki authoring, not direct palace authoring**

Change the system prompt block from:

```text
file memory directly into palace
```

to:

```text
durable memory must be authored through wiki pages or wiki bullet journals;
palace is the compiled retrieval index
```

- [ ] **Step 2: Restrict direct `fact_store add` to migration/maintenance only**

Do not delete the tool yet, but mark it internally as:

```python
"write_path": "maintenance-only"
```

and prevent it from being the default durable-memory path in prompts.

- [ ] **Step 3: Keep `sync_turn()` disabled**

Retain:

```python
def sync_turn(...):
    return None
```

so there is still no passive fallback persistence.

- [ ] **Step 4: Run focused provider tests**

Run:

```powershell
python -m pytest weft-plugins/memory-runtime/tests_test_palace_provider_contract.py -q
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add weft-plugins/memory-runtime/hermes_port/palace_provider.py weft-plugins/memory-runtime/wiki/mcp/tools/guide.py weft-plugins/memory-runtime/wiki/mcp/tools/wiki_contract.py weft-plugins/memory-runtime/tests_test_palace_provider_contract.py
git commit -m "feat: make wiki the only durable memory authoring surface"
```

### Task 6: Verify end-to-end behavior and cleanly close

**Files:**
- Modify: `docs/superpowers/plans/2026-04-18-memory-runtime-wiki-authoring-model.md`

- [ ] **Step 1: Re-run all focused wiki contract and sync suites**

Run:

```powershell
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_write_contract.py -q
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_wiki_contract.py weft-plugins/memory-runtime/wiki/tests/unit/test_mcp_tools_registry.py -q
python -m pytest weft-plugins/memory-runtime/wiki/tests/unit/test_wiki_palace_sync_service.py -q
```

Expected: PASS

- [ ] **Step 2: Re-run one real integration script**

Run:

```powershell
python D:\weft\tmp\memory_runtime_live_compare_three_platforms_zh.py
```

Expected:

```text
niconico_specific resolves to cluster-niconico
broad comparison no longer regresses to stale乱码 pages
```

- [ ] **Step 3: Review diff contamination**

Run:

```powershell
git status --short
git diff --name-only HEAD
```

Expected: stage only memory-runtime/wiki files for this plan's commits.

- [ ] **Step 4: Update plan status notes**

At the bottom of this plan, add:

```md
## Implementation Notes

- [ ] Task 1 complete
- [ ] Task 2 complete
- [ ] Task 3 complete
- [ ] Task 4 complete
- [ ] Task 5 complete
- [ ] Task 6 complete
```

- [ ] **Step 5: Final commit**

```bash
git add docs/superpowers/plans/2026-04-18-memory-runtime-wiki-authoring-model.md
git commit -m "docs: finalize wiki-only memory authoring plan"
```

## Implementation Notes

- [x] Task 1 complete
- [x] Task 2 complete
- [x] Task 3 complete
- [x] Task 4 complete
- [x] Task 5 complete
- [x] Task 6 complete
