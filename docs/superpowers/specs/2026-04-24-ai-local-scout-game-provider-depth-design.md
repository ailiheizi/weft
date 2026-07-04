# AI Local Scout Game Provider Depth Design

## Goal

In `apps/ai-local-scout`, deepen the existing game-related evidence model so the scout reports more structured, provider-specific facts for already supported gaming ecosystems.

This work is not about adding fuzzy gaming detection or broad new ecosystem guesses.
It is about lifting more stable fields from existing local artifacts that the app already knows how to find.

Primary outcomes:

1. Keep generic-machine compatibility for Windows installs.
2. Stay evidence-first and artifact-backed.
3. Increase the depth of `gaming_profile` and related raw evidence without introducing keyword guessing or fallback inference.
4. Preserve current probe coverage while making existing providers more useful.

## Scope

This design is limited to:

- `apps/ai-local-scout/ai_local_scout/parsers.py`
- `apps/ai-local-scout/ai_local_scout/runtime.py`
- `apps/ai-local-scout/tests/test_scout_runtime.py`
- `apps/ai-local-scout/README.md`

This design covers only already supported game providers:

- `steam`
- `epic`
- `gog`
- `battle_net`
- `ea`
- `xbox`
- `itch`
- `amazon_games`
- `ubisoft`
- `playnite`

## Non-Goals

This design does not include:

- scanning arbitrary executables to guess that they are games
- scraping launcher UIs, logs, caches, or opaque binaries without a stable format
- inferring providers from install path keywords alone
- adding unsupported ecosystems just because they are popular
- merging unrelated non-game providers into this slice
- host-specific paths or hand-tuned one-machine rules

## Current State

The app already detects many gaming ecosystems, but the current normalized result is shallow.

Today, most providers end up contributing only:

- `platform`
- `platform_game_id`
- `name`
- `install_location`

`Steam` and `Epic` have slightly more raw parser detail available, but the derived profile currently compresses nearly everything into:

- `gaming_profile.is_gamer`
- `gaming_profile.platforms`
- `gaming_profile.installed_game_names`
- `gaming_profile.game_count`

This means the app can already answer "which launchers/games are present", but not "what kind of installs are present inside those launchers" with much precision.

## Design Principles

The implementation must follow these rules:

1. Only surface fields that come directly from stable local artifacts already being parsed.
2. If a field is provider-specific, keep it provider-specific instead of collapsing it into generic vague labels.
3. Derived profile fields should be deterministic summaries of raw evidence, not heuristic interpretations.
4. When formats vary across machines, use tolerant parsing of explicit field names, not semantic guessing.
5. Missing data should remain missing; do not synthesize values.

## Approaches Considered

### Approach A: Deepen only `gaming_profile`

Add more aggregated fields while leaving most raw provider structures unchanged.

Pros:

- smaller implementation
- lower immediate churn

Cons:

- derived profile becomes harder to trust because it is doing too much interpretation
- raw evidence remains too thin for future expansion

### Approach B: Deepen raw provider parsing first, then lift stable summaries

Expand provider parsers to keep more artifact-backed fields, then add clearly derived profile summaries on top.

Pros:

- strongest evidence chain
- easier to test red-green per provider
- easier to extend later without redesigning the profile layer

Cons:

- slightly more code touched up front

### Approach C: Add a fully generic game metadata schema for all providers

Force every provider into a large common schema now.

Pros:

- superficially uniform

Cons:

- encourages empty or guessed fields
- hides provider differences
- likely to create fake precision

## Decision

Use Approach B.

The app should deepen raw parser output first and then expose conservative derived summaries.
That keeps the evidence chain clear and avoids inventing semantics that the source artifacts do not actually guarantee.

## Delivery Phases

### Phase 1: Core launcher depth

Prioritize:

- `steam`
- `epic`

Reason:

- these are already central to the app's gaming output
- their local artifacts are structured and stable
- they offer the highest value for the lowest risk

### Phase 2: Structured secondary launchers

Then deepen:

- `gog`
- `battle_net`
- `ea`

Reason:

- these already expose explicit IDs and install-oriented metadata
- they can contribute useful counts and provider-specific details without guesswork

### Phase 3: Thin but valid ecosystems

Then deepen:

- `xbox`
- `itch`
- `amazon_games`
- `ubisoft`
- `playnite`

Reason:

- these are still worth enriching, but their source artifacts usually expose fewer stable fields

## Parser Changes

### Steam

Current parser already reads:

- `app_id`
- `name`
- `install_dir_name`
- `size_on_disk`
- `state_flags`

Phase 1 should preserve these raw fields and ensure derived aggregation exposes them.

New derived fields:

- `steam_game_count`
- `steam_app_ids`
- `steam_install_dir_names`
- `steam_sizes_on_disk`
- `steam_state_flags`
- `steam_library_roots`
- `steam_manifest_count`

Notes:

- `steam_library_roots` must come from `steam_library_index` entries and the expansion flow, not path guessing.
- `steam_sizes_on_disk` should remain raw string values unless a source artifact clearly guarantees numeric normalization.
- `steam_state_flags` should remain raw flags; no semantic decoding table in this phase.

### Epic

Current parser already reads:

- `catalog_item_id`
- `name`
- `app_name`
- `install_location`
- `launch_executable`
- `install_size`
- `technical_type`

Legendary/Heroic installed exports already contribute:

- `name`
- `app_name`
- `install_location`

Phase 1 should add derived fields:

- `epic_game_count`
- `epic_catalog_item_ids`
- `epic_app_names`
- `epic_install_locations`
- `epic_launch_executables`
- `epic_install_sizes`
- `epic_technical_types`
- `epic_executable_path_count`

Notes:

- executable paths should only be formed when both `install_location` and `launch_executable` exist.
- Legendary/Heroic entries should contribute only the fields they explicitly provide.

### GOG

Current parser reads:

- `name`
- `platform_game_id`
- `install_location`

Phase 2 should preserve any explicit raw source identifiers already present in `galaxy-installed.json` variants and add:

- `gog_game_count`
- `gog_game_ids`
- `gog_install_locations`

If the raw JSON exposes stable launcher-specific product identifiers beyond the current normalized ID field, they may be preserved in raw evidence, but the derived profile should still summarize only explicit extracted values.

### Battle.net

Current parser reads:

- `name`
- `platform_game_id`
- `install_location`
- `product_code`

Phase 2 should add:

- `battle_net_game_count`
- `battle_net_product_codes`
- `battle_net_install_locations`

Notes:

- keep `product_code` raw
- keep name mapping evidence-backed to the existing explicit code map or artifact name, not fuzzy lookup

### EA

Current parser reads:

- `name`
- `platform_game_id`
- `install_location`

Phase 2 should add:

- `ea_game_count`
- `ea_game_ids`
- `ea_install_locations`

### Xbox

Current parser reads:

- `name`
- `platform_game_id`
- `install_location`
- `publisher`

Phase 3 should add:

- `xbox_game_count`
- `xbox_package_names`
- `xbox_publishers`
- `xbox_install_locations`

### itch

Current parser reads:

- `name`
- `platform_game_id`
- `install_location`

Phase 3 should add:

- `itch_game_count`
- `itch_game_ids`
- `itch_install_locations`

### Amazon Games

Current parser reads:

- `name`
- `platform_game_id`
- `install_location`

Phase 3 should add:

- `amazon_games_game_count`
- `amazon_games_ids`
- `amazon_games_install_locations`

### Ubisoft

Registry-based launcher install data already contributes game rows.

Phase 3 should add:

- `ubisoft_game_count`
- `ubisoft_install_locations`

Only explicit values returned by the registry probe should be surfaced.

### Playnite

Current parser already preserves:

- `name`
- `platform`
- `platform_game_id`
- `install_location`
- `plugin_id`

Phase 3 should add:

- `playnite_game_count`
- `playnite_plugin_ids`
- `playnite_platforms`

Notes:

- `playnite_platforms` must reflect the current explicit plugin-to-platform mapping logic already in code.
- no new keyword classifier should be introduced in this slice.

## Derived Profile Changes

`gaming_profile` should remain the primary summary surface, but it should become deeper and provider-specific.

Add fields in three layers:

### Layer 1: Existing global summary

Keep:

- `is_gamer`
- `platforms`
- `installed_game_names`
- `game_count`

### Layer 2: Provider counts

Add:

- `steam_game_count`
- `epic_game_count`
- `gog_game_count`
- `battle_net_game_count`
- `ea_game_count`
- `xbox_game_count`
- `itch_game_count`
- `amazon_games_game_count`
- `ubisoft_game_count`
- `playnite_game_count`

### Layer 3: Provider-specific detail lists

Add only deterministic list summaries sourced from raw evidence, such as:

- IDs
- install locations
- manifest-backed install dir names
- raw state flags
- launch executable relative paths
- publisher names
- product codes
- plugin IDs

All list outputs should be:

- deduplicated
- sorted
- capped consistently if needed

## Data Flow

1. Probe existing launcher/config/database artifacts as today.
2. Parse provider-specific explicit fields.
3. Normalize into game rows without inventing missing values.
4. Aggregate provider-specific summaries in `runtime.py`.
5. Expose these summaries through `derived_profile.gaming_profile`.

No new AI expansion step is required for this slice.

## Error Handling

Error handling should stay conservative:

- unreadable or malformed artifacts return empty provider results
- partial artifacts keep partial explicit fields
- no parser should fail the whole scout run because one launcher artifact is broken
- derived summaries should tolerate absent fields and produce empty lists or `0` counts

## Testing Strategy

Use strict TDD per provider slice.

### Red-Green order

1. add failing assertions in `tests/test_scout_runtime.py`
2. run targeted tests for the provider being deepened
3. implement minimum parser/profile changes
4. rerun targeted tests
5. rerun full `test_scout_runtime.py`

### Phase 1 tests

Strengthen existing Steam/Epic tests to assert:

- Steam provider count and app ID aggregation
- Steam install dir names
- Steam raw size/state flag lifting into derived profile
- Steam library/manifests summary counts
- Epic catalog/app/install metadata lifting
- Epic executable path count
- mixed Epic launcher plus Legendary aggregation without fake fields

### Phase 2 tests

Strengthen existing GOG/Battle.net/EA tests to assert:

- product IDs and product codes
- provider-specific install locations
- provider counts in the same combined profile

### Phase 3 tests

Strengthen existing Xbox/itch/Amazon/Ubisoft/Playnite tests to assert:

- package or product identifiers when present
- publisher/plugin summaries where explicitly available
- stable per-provider counts

## README Changes

Update `apps/ai-local-scout/README.md` to reflect that gaming coverage is no longer just presence detection.

The README should mention that the tool now summarizes provider-specific install metadata for supported game ecosystems, still using local deterministic artifacts only.

## Success Criteria

The slice is complete when all of the following are true:

1. `gaming_profile` still preserves current top-level compatibility.
2. `Steam` and `Epic` expose deeper provider-specific derived fields from existing artifacts.
3. Secondary providers expose count and identifier/install summaries without heuristic guessing.
4. No new keyword-based provider inference is added.
5. Existing tests remain green and new assertions prove the deeper fields.
6. Full regression for `tests/test_scout_runtime.py` passes.

## Recommended Implementation Order

1. `Steam`
2. `Epic`
3. `GOG`
4. `Battle.net`
5. `EA`
6. `Xbox`
7. `itch`
8. `Amazon Games`
9. `Playnite`
10. `Ubisoft`

This order maximizes early value and keeps the work aligned with the richest, most stable artifacts first.
