# Memory Runtime Preference Card Benchmark Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current preference-question benchmark adapter with a structured preference card, raw evidence, and answer contract path so clean Haiku can use remembered user preferences precisely.

**Architecture:** Keep retrieval unchanged. For `single-session-preference`, compile retrieved evidence into a compact preference card plus verbatim evidence excerpts, then synthesize answers under an explicit contract. Leave non-preference categories on the existing path.

**Tech Stack:** Python, LongMemEval runner, local benchmark fixtures, PowerShell verification

---

### Task 1: Lock The New Preference-Card Contract In Tests

**Files:**
- Modify: `weft/scripts/test-weft-memory-runtime-evidence-compiler.py`

- [ ] **Step 1: Write the failing test**

Add assertions that require the compiler to produce:

- `Must Match`
- `Preferred Signals`
- `Avoid / Not Preferred`
- `Current Setup`
- `Open Unknowns`
- `Evidence Anchors`

Also require the answer prompt to contain explicit instructions to follow the preference card before giving suggestions.

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
py -3.13 D:\weft\weft\scripts\test-weft-memory-runtime-evidence-compiler.py
```

Expected: FAIL because the current compiler still emits the old four-bucket structure and lacks the new contract wording.

- [ ] **Step 3: Commit the red test snapshot**

```powershell
git add D:\weft\weft\scripts\test-weft-memory-runtime-evidence-compiler.py
git commit -m "test: lock preference-card benchmark adapter contract"
```

### Task 2: Implement Preference Card Compilation

**Files:**
- Modify: `weft-plugins/memory-runtime/hermes_port/evidence_compiler.py`

- [ ] **Step 1: Implement minimal helpers for preference-card prompt assembly**

Add helpers that:

- collect ranked retrieved items
- keep compact raw evidence blocks
- build the new preference-card extraction prompt
- build the new preference answer synthesis prompt

- [ ] **Step 2: Keep non-preference behavior untouched**

Implement the new preference-card path so it is only used when `entry["question_type"] == "single-session-preference"`.

- [ ] **Step 3: Run focused test**

Run:

```powershell
py -3.13 D:\weft\weft\scripts\test-weft-memory-runtime-evidence-compiler.py
```

Expected: PASS

- [ ] **Step 4: Commit**

```powershell
git add D:\weft\weft\scripts\test-weft-memory-runtime-evidence-compiler.py D:\weft\weft-plugins\memory-runtime\hermes_port\evidence_compiler.py
git commit -m "feat: add preference-card evidence compilation"
```

### Task 3: Wire Preference Questions To The New Adapter

**Files:**
- Modify: `tmp/longmemeval/run_claude_cli_longmemeval.py`

- [ ] **Step 1: Write a failing runner-level check**

Add a focused test or script assertion path that proves preference questions call the preference-card compiler path while other question types keep the old prompt path.

- [ ] **Step 2: Run the runner-level check to verify failure**

Run the focused verification command for the new check.

Expected: FAIL before the runner is updated.

- [ ] **Step 3: Implement minimal routing**

Route generation so:

- preference questions use preference-card extraction + synthesis
- non-preference questions continue using `prepare_prompt(...)`

- [ ] **Step 4: Run the focused verification again**

Expected: PASS

- [ ] **Step 5: Commit**

```powershell
git add D:\weft\tmp\longmemeval\run_claude_cli_longmemeval.py
git commit -m "feat: route preference questions through preference-card adapter"
```

### Task 4: Verify With Clean Haiku Pref30

**Files:**
- Verify only:
  - `tmp/longmemeval/claude_clean_wrapper.cmd`
  - `tmp/longmemeval/qa_logs/pref30_v26_haiku_clean_v3_hyp.jsonl`
  - `tmp/longmemeval/qa_logs/pref30_v26_haiku_clean_v3_eval.jsonl`

- [ ] **Step 1: Re-run clean Haiku generation**

Run:

```powershell
$env:CLAUDE_BIN='D:\weft\tmp\longmemeval\claude_clean_wrapper.cmd'
py -3.13 D:\weft\tmp\longmemeval\run_claude_cli_longmemeval.py generate --in-file D:\weft\tmp\longmemeval\retrieval_logs\memory-runtime\session\pref30_v26_dual_retrievallog_session_memory-runtime --out-file D:\weft\tmp\longmemeval\qa_logs\pref30_v26_haiku_clean_v3_hyp.jsonl --model haiku --workers 2
```

- [ ] **Step 2: Check for contamination**

Run:

```powershell
rg -n "superpowers|brainstorming|using-superpowers|compound-engineering|local URL|browser|permissions|skill" D:\weft\tmp\longmemeval\qa_logs\pref30_v26_haiku_clean_v3_hyp.jsonl
```

Expected: no matches

- [ ] **Step 3: Run evaluation**

Run:

```powershell
$env:CLAUDE_BIN='D:\weft\tmp\longmemeval\claude_clean_wrapper.cmd'
py -3.13 D:\weft\tmp\longmemeval\run_claude_cli_longmemeval.py evaluate --hyp-file D:\weft\tmp\longmemeval\qa_logs\pref30_v26_haiku_clean_v3_hyp.jsonl --ref-file D:\weft\tmp\longmemeval-data\pref30_only.json --out-file D:\weft\tmp\longmemeval\qa_logs\pref30_v26_haiku_clean_v3_eval.jsonl --model haiku --workers 2
```

- [ ] **Step 4: Print metrics**

Run:

```powershell
py -3.13 D:\weft\tmp\longmemeval\run_claude_cli_longmemeval.py metrics --in-file D:\weft\tmp\longmemeval\qa_logs\pref30_v26_haiku_clean_v3_eval.jsonl --ref-file D:\weft\tmp\longmemeval-data\pref30_only.json
```

Expected: `Overall Accuracy` greater than `0.3333`

- [ ] **Step 5: Final scope check**

Run:

```powershell
git diff --stat
```

Confirm the change stays within:

- `docs/superpowers/*`
- `weft/scripts/test-weft-memory-runtime-evidence-compiler.py`
- `weft-plugins/memory-runtime/hermes_port/evidence_compiler.py`
- `tmp/longmemeval/run_claude_cli_longmemeval.py`
