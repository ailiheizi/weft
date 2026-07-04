# AI Local Scout Meaningful Breadth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a lightweight `meaningful_activity_profile` that preserves broad local context while keeping AI Local Scout one-shot and read-only.

**Architecture:** Reuse existing parsed evidence and derived profiles. Add deterministic helper functions in `runtime.py` and cover them through `run_scout` tests using fixture evidence.

**Tech Stack:** Python, pytest, existing `ai_local_scout.runtime` profile derivation.

---

### Task 1: Meaningful Activity Profile

**Files:**
- Modify: `apps/ai-local-scout/tests/test_scout_runtime.py`
- Modify: `apps/ai-local-scout/ai_local_scout/runtime.py`
- Modify: `apps/ai-local-scout/README.md`

- [ ] Add failing tests for `meaningful_activity_profile` primary modes, setup hints, gaming style hints, evidence quality, and cautious summary.
- [ ] Run the targeted pytest tests and confirm failure because the new profile section does not exist.
- [ ] Implement the smallest deterministic helper in `runtime.py` using existing derived profile data and existing evidence lists.
- [ ] Re-run targeted tests and then the full `apps/ai-local-scout` test suite.
- [ ] Update README profile list and notes.

### Task 2: Verification

**Files:**
- Verify: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] Run `python -m pytest tests/test_scout_runtime.py -q` from `apps/ai-local-scout`.
- [ ] Run a real sample command if safe and inspect the new JSON profile section.
