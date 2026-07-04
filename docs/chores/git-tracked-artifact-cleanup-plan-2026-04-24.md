# Tracked Artifact Cleanup Plan - 2026-04-24

## Purpose

`.gitignore` now prevents new runtime artifacts from appearing, but it does not affect files that are already tracked. A double check found 735 tracked files that match the current ignore rules.

This document records the cleanup pass that untracked ignored artifacts with `git rm --cached`. The working-tree files were left on disk.

## Current Finding

Command used:

```powershell
git ls-files -c -i --exclude-standard | Measure-Object -Line
```

Initial result:

```text
735 tracked ignored files
```

Final result after cleanup:

```text
0 tracked ignored files
```

## High Confidence Untrack Candidates

These were removed from the git index:

- `.playwright-cli/`
- `output/`
- `runtime-debug/`
- `weft/scripts/__pycache__/`
- `weft-plugins/companion-core/__pycache__/`
- `weft-plugins/skills/openclaw-skills/ianwchoi-duckduckgo-websearch/scripts/node_modules/`
- `tmp-AIChat-1.9.0/`
- `tmp/zipformer-audio-tagging/venv/`
- root temporary logs matching `tmp-weft-*.log`, `tmp-companion-*.log`, `tmp-weft-*.out.log`, `tmp-weft-*.err.log`
- root generated text runs matching `test-100-*.txt`, `test-final*.txt`

## Runtime Data Candidates

These runtime state files were removed from the git index:

- `data/palace/`
- `weft-plugins/memory-runtime/data/`
- `weft-plugins/companion-core/data/inner-state.json`
- `weft-plugins/companion-core/data/proactive-state.json`

Do not untrack these companion source-of-truth files:

- `weft-plugins/companion-core/data/persona.md`
- `weft-plugins/companion-core/data/proactive.json`

## Commands Used

The cleanup was done with `git rm --cached` so files remain in the working tree. The first all-files attempt hit a Windows pathspec encoding issue on `tmp-AIChat-1.9.0/请读我.html`, so cleanup was completed by explicit directory/file groups.

```powershell
git rm -r --cached --ignore-unmatch -- ".playwright-cli" "output" "runtime-debug" "weft/scripts/__pycache__" "weft-plugins/companion-core/__pycache__" "weft-plugins/skills/openclaw-skills/ianwchoi-duckduckgo-websearch/scripts/node_modules" "tmp-AIChat-1.9.0" "tmp/zipformer-audio-tagging/venv" "data/palace" "weft-plugins/memory-runtime/data"
```

The second cleanup batch covered:

- root debug text files: `debug-stdout.log`, `debug2.txt`, `debug3.txt`, `debug_response.txt`
- companion volatile state/log files: `inner-state.json`, `proactive-state.json`, BepInEx logs
- context-engine Python cache: `weft-plugins/context-engine/__pycache__`
- memory-runtime local file storage and prod-start log
- root test run files: `test-100-*.txt`
- root temp AIChat files: `tmp-AIChat-1.9.0.zip`, `tmp-aichat-*.cs`
- root temp process logs: `tmp-companion-*.log`, `tmp-weft-*.log`, `tmp-weft-*.out.log`, `tmp-weft-*.err.log`

Verification commands:

```powershell
git status --short
git ls-files -c -i --exclude-standard | Measure-Object -Line
git check-ignore -v "weft-plugins/companion-core/data/persona.md" "weft-plugins/companion-core/data/proactive.json"; if ($LASTEXITCODE -eq 1) { exit 0 } else { exit $LASTEXITCODE }
```

Additional verification confirmed these files remain tracked and are not ignored:

- `WEFT-core/Cargo.toml`
- `weft/package.json`
- `weft-plugins/companion-core/plugin.toml`
- `weft-plugins/companion-core/data/persona.md`
- `weft-plugins/companion-core/data/proactive.json`
- `weft-plugins/context-engine/proactive-skills/gaming-companion.json`

Additional verification confirmed representative untracked files remain on disk:

- `output/electron-postfix-live.out.log`
- `weft-plugins/companion-core/data/inner-state.json`
- `weft-plugins/memory-runtime/wiki/data/local-file-storage/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/1c431a2f-780a-48ea-9b4c-22780160787a/source.png`
- `tmp-AIChat-1.9.0/请读我.html`

## Commit Strategy

Use separate commits:

1. Hygiene metadata and index cleanup: `.gitignore`, `.gitattributes`, `docs/git-hygiene-2026-04-24.md`, this cleanup record, and staged `git rm --cached` deletions.
2. Product/source changes: reviewed source diffs only.

This keeps the cleanup auditable and avoids mixing product work with repository maintenance.
