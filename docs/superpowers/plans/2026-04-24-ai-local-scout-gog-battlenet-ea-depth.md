# AI Local Scout GOG Battle.net EA Depth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen `gaming_profile` for existing `GOG`, `Battle.net`, and `EA` providers using only explicit identifiers and install metadata already parsed from local artifacts.

**Architecture:** Reuse the current raw evidence pipeline and add failing tests first. Then extend `runtime.py` aggregation so `gaming_profile` exposes provider-specific counts, IDs, install locations, and Battle.net product codes without changing probe behavior or adding inference.

**Tech Stack:** Python, pytest, existing ai-local-scout parser/runtime code

---

### Task 1: Add failing Battle.net gaming profile assertions

**Files:**
- Modify: `apps/ai-local-scout/tests/test_scout_runtime.py`
- Test: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] **Step 1: Write the failing test**

Strengthen the existing Battle.net product DB tests so they expect:

```python
    assert gaming_profile["battle_net_game_count"] == 2
    assert gaming_profile["battle_net_game_ids"] == ["fen", "s2"]
    assert gaming_profile["battle_net_product_codes"] == ["Fen", "S2"]
    assert gaming_profile["battle_net_install_locations"] == [
        "D:\\Games\\Battle.net\\Diablo IV",
        "D:\\Games\\Battle.net\\StarCraft II",
    ]
```

Also strengthen the ProgramData discovery test so it expects:

```python
    assert gaming_profile["battle_net_game_count"] == 1
    assert gaming_profile["battle_net_game_ids"] == ["prometheus"]
    assert gaming_profile["battle_net_product_codes"] == ["Pro"]
    assert gaming_profile["battle_net_install_locations"] == ["D:\\Games\\Battle.net\\Overwatch"]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "battle_net_product_db" -q`
Expected: FAIL on missing `battle_net_*` keys in `gaming_profile`

- [ ] **Step 3: Write minimal implementation**

Extend `apps/ai-local-scout/ai_local_scout/runtime.py` so it aggregates explicit Battle.net IDs, product codes, install paths, and provider counts from `battle_net_product_db` evidence.

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "battle_net_product_db" -q`
Expected: PASS

### Task 2: Add failing EA gaming profile assertions

**Files:**
- Modify: `apps/ai-local-scout/tests/test_scout_runtime.py`
- Test: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] **Step 1: Write the failing test**

Strengthen the existing EA localcontent test so it expects:

```python
    assert gaming_profile["ea_game_count"] == 1
    assert gaming_profile["ea_game_ids"] == ["OFB-EAST:109552419"]
    assert gaming_profile["ea_install_locations"] == ["D:\\EA Games\\Mass Effect Legendary Edition"]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "origin_localcontent or ea_games" -q`
Expected: FAIL on missing `ea_*` keys in `gaming_profile`

- [ ] **Step 3: Write minimal implementation**

Extend `apps/ai-local-scout/ai_local_scout/runtime.py` so it aggregates explicit EA IDs and install locations from `origin_localcontent_manifest` evidence.

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "origin_localcontent or ea_games" -q`
Expected: PASS

### Task 3: Add failing GOG gaming profile assertions

**Files:**
- Modify: `apps/ai-local-scout/tests/test_scout_runtime.py`
- Test: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] **Step 1: Write the failing test**

Strengthen the existing GOG tests so they expect:

```python
    assert gaming_profile["gog_game_count"] == 1
    assert gaming_profile["gog_game_ids"] == ["gog-cp2077"]
    assert gaming_profile["gog_install_locations"] == ["D:\\GOG Games\\Cyberpunk 2077"]
```

And for the AppData discovery test:

```python
    assert gaming_profile["gog_game_count"] == 1
    assert gaming_profile["gog_game_ids"] == ["gog-w3"]
    assert gaming_profile["gog_install_locations"] == ["D:\\GOG Games\\The Witcher 3 Wild Hunt GOTY"]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "gog" -q`
Expected: FAIL on missing `gog_*` keys in `gaming_profile`

- [ ] **Step 3: Write minimal implementation**

Extend `apps/ai-local-scout/ai_local_scout/runtime.py` so it aggregates explicit GOG IDs and install paths from `gog_installed` evidence.

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "gog" -q`
Expected: PASS

### Task 4: Run broader game regression

**Files:**
- Test: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] **Step 1: Run targeted mixed-provider regression**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "gog or battle_net or origin_localcontent or ea_games or steam or epic" -q`
Expected: PASS

- [ ] **Step 2: Run full runtime suite**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -q`
Expected: PASS with 0 failures
