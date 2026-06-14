# AI Local Scout Delivery

This package contains the standalone one-shot local context scout we built.

## What It Does

AI Local Scout scans local evidence and produces a JSON report with raw evidence plus derived profiles.

Current coverage includes:

- AI/agent config: Claude, Codex, Cursor, AGENTS.md, MCP config
- developer context: git repos, editor workspaces, shell history, language/tooling config
- games: Steam and other launcher manifests where available
- browser context: bookmarks, history domains, search/download signals, extensions, sessions
- Desktop Office documents: `.docx`, `.pptx`, `.xlsx` with capped preview and skill routes
- recent documents: Windows Recent shortcut routing where available
- Downloads: filesystem inventory plus browser download records
- knowledge tools: Obsidian and Joplin signals
- sync/storage: OneDrive, Syncthing, Dropbox, Nextcloud where available
- local runtime: Docker, WSL, terminal, cloud CLI config where available
- hardware/app hints from installed applications

## Privacy Model

- One-shot scan only; no daemon or background monitor.
- No screenshots, keylogging, live window capture, or audio capture.
- Desktop Office documents are high-sensitivity evidence and only store capped previews plus skill routes.
- Recent documents and downloads keep metadata and routes, not full file contents.
- Steam public profile playtime is opt-in and only reads public Steam Community pages.

## Run A Scan

From this folder:

```powershell
.\run-scout.ps1
```

With Steam public profile opt-in:

```powershell
.\run-scout.ps1 -SteamPublicProfile
```

The report is written to `output/ai-local-scout-report.json` by default.

## Run Tests

```powershell
.\run-tests.ps1
```

Recent full-suite verification during packaging:

```text
130 passed in 829.82s
```

## Reports Included

The delivery package includes a `reports/` folder with the latest local scan JSON files copied from this machine.

Use those as examples of the current schema and derived profiles.
