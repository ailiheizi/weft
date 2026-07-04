# AI Local Scout Meaningful Breadth Design

## Goal

Improve `apps/ai-local-scout` without turning it into a heavy monitor. Keep the project as a standalone, read-only, one-shot scanner, but make its output more useful by adding a lightweight meaning layer over the broad local evidence it already collects.

## Constraints

- Keep one-shot execution only. No daemon, no background recording, no screenshots, no keylogging, and no live window polling.
- Preserve breadth across games, apps, browser activity, dev tools, creator tools, knowledge tools, sync tools, and local runtimes.
- Prefer strong local artifacts over guesses. Every inference should expose evidence quality.
- Keep sensitive values redacted and avoid reading private message content.
- Make the smallest changes needed to produce better derived output.

## Approach

Add a new derived profile section named `meaningful_activity_profile`. It will not add new invasive probes. It will summarize existing derived sections into a readable machine profile:

- `primary_modes`: broad life/work modes detected from existing profiles, such as `gaming`, `building`, `ai_tool_use`, `knowledge_work`, `content_creation`, `collaboration`, `sync_storage`, `terminal_work`, `cloud_work`, and `local_linux_runtime`.
- `gaming_style_hints`: lightweight game-library hints based on installed game names and platform mix. Examples: `cozy_indie`, `competitive_multiplayer`, `rpg_story`, `strategy`, `sandbox_creative`, `aaa_single_player`, `modding_friendly`.
- `pc_setup_hints`: broad setup signals such as `multi_launcher_game_library`, `developer_workstation`, `ai_augmented_workstation`, `notes_and_vaults`, `creator_streaming_setup`, `cross_device_sync`, `containerized_dev`, and `cloud_cli_ready`.
- `evidence_quality`: per-area quality labels: `strong`, `medium`, `weak`, or `absent`.
- `wholesome_summary`: 2-4 cautious, friendly lines grounded in evidence. It should avoid psychological claims and say when a signal is only install-based.

## Breadth Policy

Breadth matters, but it should stay cheap. The first implementation will not add many new artifact types. Instead it will make all already-collected categories visible in the meaning layer. This gives broad understanding without expanding scan cost.

Future breadth additions should follow this bar:

- Add a parser only when the artifact is stable, local, and high-signal.
- Prefer manifest/config/library files over raw content stores.
- Do not add probes that require continuous runtime access.
- Add tests with fixture files for every new artifact type.

## Data Flow

`run_scout` remains unchanged at the top level. `_derive_profile` will continue building specific profiles, then call a small helper to create `meaningful_activity_profile` from the already-derived values and evidence lists.

The helper should be deterministic and testable. It should not call subprocesses, read files, or mutate evidence.

## Testing

Add tests that cover:

- broad mode detection from existing developer, gaming, AI, knowledge, creator, sync, terminal, cloud, container, and Linux runtime profiles
- gaming style hints from known game names
- evidence quality labels for strong install/library evidence versus weak browser/search-only signals
- wholesome summary staying cautious and evidence-grounded

## Non-Goals

- No UI or dashboard in this pass.
- No long-term local memory.
- No live activity capture.
- No deep content analysis of personal notes, chats, or browser page contents.
