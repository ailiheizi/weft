# Git Hygiene Report - 2026-04-24

## Scope

Current worktree: `D:/weft`

Current branch: `checkpoint/2026-04-03-full-workspace`

This report classifies the dirty worktree, the ignore boundary added in `.gitignore`, and the index cleanup performed with `git rm --cached`. No working-tree files were deleted or reverted.

## Repository Shape

- One git repository with multiple worktrees.
- Main development worktree: `D:/weft`.
- Integration/repair worktrees exist under `D:/weft-integration/` and should not be touched from this worktree.
- Remote: `origin https://gitea.alhz.org/weft/desktop.git`.
- `origin/master` is far behind the local checkpoint branch.

## Dirty Worktree Categories

### Likely Source Changes To Review

These look like intentional source/test changes and should be reviewed before any commit:

- `WEFT-core/src/api/plugins.rs`
- `WEFT-core/src/main.rs`
- `WEFT-core/src/plugin/bridge.rs`
- `weft-plugins/skills/openclaw-skills/donnycui-bilibili-youtube-watcher/README.md`
- `weft-plugins/skills/openclaw-skills/donnycui-bilibili-youtube-watcher/SKILL.md`
- `weft-plugins/skills/openclaw-skills/donnycui-bilibili-youtube-watcher/scripts/get_transcript.py`
- `weft/scripts/test-chat-model-selector-contract.ts`
- `weft/scripts/test-companion-memory-runtime-integration.py`
- `weft/scripts/test-context-engine-runtime-bridge-contract.ts`
- `weft/scripts/test-workspace-wiki-web-ready.ts`
- `weft/scripts/test-workspace-wiki-web-service.ts`
- `weft/src/main/services/workspace-wiki-web-service.ts`
- `weft/src/renderer/components/ChatPanel.vue`
- `weft/src/renderer/components/WeftDockPanel.vue`
- `weft/src/renderer/ui/primitives/UiSelect.vue`

### Runtime Artifacts Removed From Index

These were previously tracked. They are now staged as index removals via `git rm --cached`, so the files remain on disk but will stop being tracked after commit.

- `.playwright-cli/*`
- `output/*`
- `runtime-debug/*`
- root debug/log/test artifacts such as `debug*.txt`, `debug-stdout.log`, `test-100-*.txt`, `tmp-weft-*.log`, `tmp-weft-*.out.log`, `tmp-weft-*.err.log`
- `weft-plugins/companion-core/__pycache__/*`
- `weft-plugins/companion-core/data/inner-state.json`
- `weft-plugins/companion-core/data/proactive-state.json`
- `weft-plugins/companion-core/game/BepInEx/*.log`
- `weft-plugins/context-engine/__pycache__/*`
- `weft-plugins/memory-runtime/data/*`
- `weft-plugins/memory-runtime/wiki/data/local-file-storage/*`
- `weft-plugins/memory-runtime/wiki/web/.prod-start.log`
- `weft-plugins/skills/openclaw-skills/ianwchoi-duckduckgo-websearch/scripts/node_modules/*`
- `weft/scripts/__pycache__/*`
- `tmp-AIChat-1.9.0*`
- `tmp/zipformer-audio-tagging/venv/*`

### Untracked Runtime/Test Artifacts Now Covered By Ignore

New files matching these classes should stop appearing in future `git status` output:

- Python caches: `**/__pycache__/`, `**/*.pyc`, `**/.pytest_cache/`
- Local Python deps/envs: `**/.weft-pydeps/`, `**/.venv/`, `**/venv/`
- Logs/debug output: `*.log`, `*.out.log`, `*.err.log`, `runtime-debug/`, `output/`, `.playwright-cli/`
- Runtime DB/state: `data/palace/`, `weft-plugins/memory-runtime/data/`
- Companion volatile state: `inner-state.json`, `proactive-state.json`, `.wiki-seeded`
- Temporary test text artifacts: `test-*.txt`, `test-final*.txt`, `tmp-*`

## Notes

- `weft-plugins/companion-core/data/persona.md` and `proactive.json` remain trackable because they are source-of-truth configuration, not volatile runtime state.
- Existing tracked artifacts were untracked with `git rm --cached`; their working-tree files remain in place.
- The current `.gitignore` is conservative around companion data: only volatile state files are ignored, not the entire `companion-core/data/` directory.
- Database/binary ignores are scoped to known runtime data directories instead of global `*.bin` / `*.sqlite3` patterns, so future source assets are less likely to be hidden accidentally.
- Double check initially found 735 tracked files that already matched ignore rules. After cleanup, `git ls-files -c -i --exclude-standard | Measure-Object -Line` reports 0.
- `.gitattributes` was added to normalize source text to LF, keep Windows scripts as CRLF, and mark common runtime/binary artifacts as binary.
- `weft-plugins/companion-core/data/persona.md`, `weft-plugins/companion-core/data/proactive.json`, `weft-plugins/companion-core/plugin.toml`, `weft-plugins/context-engine/proactive-skills/gaming-companion.json`, `WEFT-core/Cargo.toml`, and `weft/package.json` were explicitly verified as still tracked and not ignored.

## Recommended Next Step

Run a separate cleanup pass after reviewing source changes:

1. Review the staged index removals before committing.
2. Commit `.gitignore`, `.gitattributes`, hygiene docs, and staged artifact removals as one repository hygiene commit.
3. Review and commit source/product changes separately.
