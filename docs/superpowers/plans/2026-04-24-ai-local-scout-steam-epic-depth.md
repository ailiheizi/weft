# AI Local Scout Steam Epic Depth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen `gaming_profile` for existing `Steam` and `Epic` providers using only stable fields already parsed from local artifacts.

**Architecture:** Keep the current probe and parser flow intact, add failing tests first, then extend `runtime.py` aggregation so `gaming_profile` exposes provider-specific `Steam` and `Epic` summaries. Avoid adding new detection logic or heuristic provider inference.

**Tech Stack:** Python, pytest, existing ai-local-scout parser/runtime code

---

### Task 1: Add failing Steam gaming profile assertions

**Files:**
- Modify: `apps/ai-local-scout/tests/test_scout_runtime.py`
- Test: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] **Step 1: Write the failing test**

Add assertions to the existing Steam/Epic gaming test so it expects:

```python
    assert gaming_profile["steam_game_count"] == 1
    assert gaming_profile["steam_app_ids"] == ["570"]
    assert gaming_profile["steam_install_dir_names"] == ["dota 2 beta"]
    assert gaming_profile["steam_sizes_on_disk"] == ["987654321"]
    assert gaming_profile["steam_state_flags"] == ["4"]
    assert gaming_profile["steam_library_roots"] == [str(steam_root), str(steam_library)]
    assert gaming_profile["steam_manifest_count"] == 1
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "system_game_probe_entries_expand_steam_and_epic_games" -q`
Expected: FAIL on missing `steam_*` keys in `gaming_profile`

- [ ] **Step 3: Write minimal implementation**

Extend `gaming_profile` aggregation in `apps/ai-local-scout/ai_local_scout/runtime.py` so it summarizes existing Steam manifest and library-index evidence into the expected fields.

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "system_game_probe_entries_expand_steam_and_epic_games" -q`
Expected: PASS

### Task 2: Add failing Epic gaming profile assertions

**Files:**
- Modify: `apps/ai-local-scout/tests/test_scout_runtime.py`
- Test: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] **Step 1: Write the failing test**

Add assertions to the same Steam/Epic gaming test so it expects:

```python
    assert gaming_profile["epic_game_count"] == 1
    assert gaming_profile["epic_catalog_item_ids"] == ["fortnite-catalog"]
    assert gaming_profile["epic_app_names"] == ["Fortnite"]
    assert gaming_profile["epic_install_locations"] == ["D:\\Epic Games\\Fortnite"]
    assert gaming_profile["epic_launch_executables"] == ["FortniteGame\\Binaries\\Win64\\FortniteLauncher.exe"]
    assert gaming_profile["epic_install_sizes"] == [123456789]
    assert gaming_profile["epic_technical_types"] == ["gamesoftware"]
    assert gaming_profile["epic_executable_path_count"] == 1
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "system_game_probe_entries_expand_steam_and_epic_games" -q`
Expected: FAIL on missing `epic_*` keys in `gaming_profile`

- [ ] **Step 3: Write minimal implementation**

Extend `gaming_profile` aggregation in `apps/ai-local-scout/ai_local_scout/runtime.py` so it summarizes existing Epic manifest evidence into the expected fields, without inventing values for Legendary/Heroic entries.

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "system_game_probe_entries_expand_steam_and_epic_games" -q`
Expected: PASS

### Task 3: Cover mixed Epic launcher plus Legendary aggregation

**Files:**
- Modify: `apps/ai-local-scout/tests/test_scout_runtime.py`
- Test: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] **Step 1: Write the failing test**

Strengthen the existing Legendary/Epic combined test so it expects:

```python
    assert gaming_profile["epic_game_count"] == 2
    assert gaming_profile["epic_app_names"] == ["AlanWake2", "Fortnite"]
    assert gaming_profile["epic_catalog_item_ids"] == ["fortnite-catalog"]
    assert gaming_profile["epic_launch_executables"] == ["FortniteGame\\Binaries\\Win64\\FortniteLauncher.exe"]
    assert gaming_profile["epic_executable_path_count"] == 1
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "legendary or epic" -q`
Expected: FAIL on missing or incomplete combined Epic summary fields

- [ ] **Step 3: Write minimal implementation**

Keep Epic summary aggregation field-aware so manifest-backed fields contribute only when present, while Legendary entries still increase counts and app-name/install-location coverage.

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "legendary or epic" -q`
Expected: PASS

### Task 4: Update README and run regression

**Files:**
- Modify: `apps/ai-local-scout/README.md`
- Test: `apps/ai-local-scout/tests/test_scout_runtime.py`

- [ ] **Step 1: Update README**

Add one short bullet under gaming coverage clarifying that `Steam` and `Epic` summaries now expose provider-specific install metadata from local artifacts.

- [ ] **Step 2: Run targeted Steam/Epic tests**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -k "steam or epic or legendary" -q`
Expected: PASS

- [ ] **Step 3: Run full runtime suite**

Run: `python -m pytest D:\weft\apps\ai-local-scout\tests\test_scout_runtime.py -q`
Expected: PASS with 0 failures
