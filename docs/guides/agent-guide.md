# Weft Agent Guide

How to work with the Weft system as an agent or AI collaborator.

---

## What is Weft

Weft is a desktop AI companion platform. It runs locally on Windows and consists of:

- **WEFT-core** — the backend runtime (Rust). Handles LLM routing, plugin execution, suite management, and all service orchestration.
- **frontend** — the desktop shell (Electron + Vite). Chat UI, companion surface, settings.
- **plugins** — wasm packages and service plugins that extend what the system can do.

---

## Repository Layout

All repos hosted at `https://gitea.alhz.org`.

Each repo is an independent git repository on disk. Local paths:

```
D:/weft/
├── weft/                    → weft/frontend
├── WEFT-core/               → weft/WEFT-core
├── docs/                     → weft/weft-docs
├── server/                   → weft/server
├── packages/
│   ├── sdk/                  → weft-plugins/plugin-sdk
│   └── official/
│       └── memory/           → weft-plugins/memory
└── weft-plugins/
    ├── agent-core/           → weft-plugins/agent-core
    ├── channels/             → weft-plugins/channels
    ├── companion-core/       → weft-plugins/companion-core
    ├── context-engine/       → weft-plugins/context-engine
    ├── cron/                 → weft-plugins/cron
    ├── mcp-client/           → weft-plugins/mcp-client
    ├── memory-runtime/       → weft-plugins/memory-runtime
    ├── prompt-system/        → weft-plugins/prompt-system
    ├── skills/               → weft-plugins/skills
    ├── suite-manager/        → weft-plugins/suite-manager
    ├── tool-runtime-core/    → weft-plugins/tool-runtime-core
    └── workflow-orchestrator/→ weft-plugins/workflow-orchestrator
```

Each directory has its own `.git` and `origin` remote. To push changes:

```bash
cd D:/weft/weft-plugins/companion-core
git add -A
git commit -m "feat: ..."
git push
```

### weft org

| Repo | What's in it |
|------|-------------|
| `weft/frontend` | Electron desktop app (TypeScript, Vite) |
| `weft/WEFT-core` | Rust backend core — LLM routing, plugin runtime, suite manager |
| `weft/weft-docs` | Architecture docs, specs, agent guide, deployment docs |
| `weft/server` | Server-side components |

### weft-plugins org

| Repo | Runtime | What it does |
|------|---------|-------------|
| `agent-core` | wasm | Agent sessions, team delegation, multi-agent routing |
| `channels` | wasm | Channel bridge |
| `companion-core` | service | Proactive companion — cognition, inner state, turn handling |
| `context-engine` | service | Screen/activity context sensing |
| `cron` | wasm | Scheduled tasks |
| `mcp-client` | wasm | MCP protocol bridge |
| `memory` | service | Memory package definition |
| `memory-runtime` | service | Wiki, palace (vector search), long-term memory |
| `plugin-sdk` | — | Shared Rust SDK for all wasm plugins |
| `prompt-system` | wasm | Prompt management |
| `skills` | wasm | Skill discovery, loading, execution (built-in + community) |
| `suite-manager` | wasm | Product assembly |
| `tool-runtime-core` | wasm | All tools: shell, files, web, git, wiki, desktop, local_scout |
| `workflow-orchestrator` | wasm | Workflow execution |

---

## System Architecture

```
User
 └── Electron frontend
      └── WEFT-core (port 42617)
           ├── /v1/chat/completions  (OpenAI-compatible)
           ├── /api/plugins          (plugin list)
           ├── /api/health
           └── Plugin runtime
                ├── agent-core        (wasm)
                ├── memory-runtime    (service, port 43129)
                ├── context-engine    (service, port 43131)
                ├── companion-core    (service, port 43127)
                ├── skills            (wasm)
                ├── tool-runtime-core (wasm) ← all tools here
                ├── channels          (wasm)
                ├── cron              (wasm)
                ├── mcp-client        (wasm)
                ├── suite-manager     (wasm)
                └── workflow-orchestrator (wasm)
```

---

## Installation

### Prerequisites

- Windows 10/11
- Node.js 24+
- Rust / Cargo
- Python 3.11+

### Clone and run

```bash
git clone https://gitea.alhz.org/weft/frontend
cd frontend
npm install
npm run dev
```

This starts the Electron app. The main process automatically starts and manages `WEFT-core`.

### First-time check

After startup, verify:

```
GET http://127.0.0.1:42617/api/health
GET http://127.0.0.1:42617/api/plugins
```

Expected plugins: `agent-core`, `memory-runtime`, `skills`, `channels`, `cron`, `companion-core`, `weft-code-runtime`, `native-echo`, `js-extension-runtime`.

> Note: `mcp-client` and `tool-runtime-core` are in `packages/official/` but not loaded in `packages/installed/` as of 2026-05-21.

### Configuration

Config lives at:
```
%APPDATA%/weft-desktop/weft-core-config/config/config.toml
```

Key fields:
```toml
[provider]
model = "claude-opus-4-5"
api_key = "sk-ant-..."
base_url = "https://api.anthropic.com"

[server]
port = 42617
```

---

## How to Call WEFT-core

Base URL: `http://127.0.0.1:42617`

### LLM chat (OpenAI-compatible)
```
POST /v1/chat/completions
{"model": "...", "messages": [...]}
```

### Call a plugin action
```
POST /api/plugin/{plugin-name}/action
{"action": "...", "data": {...}}
```

---

## Tools

All tools are in `tool-runtime-core`. Get specs:

```
POST /api/plugin/tool-runtime-core/action
{"action": "get_tool_specs"}
```

Execute a tool:
```
POST /api/plugin/tool-runtime-core/action
{"action": "execute_tool", "data": {"tool": "<name>", "args": {...}}}
```

| Tool | What it does |
|------|-------------|
| `shell_exec` | Run a shell command |
| `fs_read` | Read a file |
| `fs_write` | Write a file |
| `fs_list` | List a directory |
| `git_run` | Run a git command |
| `web_fetch` | HTTP request |
| `fetch_url` | Fetch URL content |
| `search_web` | Web search |
| `wiki_write` | Write a durable wiki page |
| `wiki_contract` | Get wiki authoring contract |
| `blink_once` | One-shot screen context capture |
| `screen_ocr` | OCR the current screen |
| `read_document` | Read content from an open window |
| `meeting_record` | Start meeting recording |
| `meeting_notes` | Get meeting notes |
| `mode_tool` | Resolve companion mode for a context event |
| `local_scout` | Scan local system for installed apps, games, browsers, dev tools |

---

## Memory System

Memory lives in `memory-runtime` (port 43129).

- **Wiki** — durable authored pages
- **Palace** — compiled vector search index built from wiki
- **Curated** — preference cards, structured profile data

```
POST /api/plugin/memory-runtime/action
{"action": "wiki_write", "data": {"title": "...", "content": "..."}}

POST /api/plugin/memory-runtime/action
{"action": "mempalace_search", "data": {"query": "..."}}
```

---

## Skills

Skills are agent-callable workflows in the `skills` plugin.

- Built-in scripts: `skills/desktop-runtime/scripts/`
- Community skills: `skills/openclaw-skills/`

```
POST /api/plugin/skills/action
{"action": "list_skills"}

POST /api/plugin/skills/action
{"action": "run_skill", "data": {"skill": "skill-name", "args": {...}}}
```

---

## Writing a Plugin

All plugins use `plugin.toml` + a wasm binary built with `weft-plugins/plugin-sdk`.

```toml
[identity]
name = "my-plugin"
version = "0.1.0"

[package]
entry = "plugin.wasm"
runtime = "wasm"
api_version = "v1"
provides = ["my.capability"]

[capability]
provides = ["my.capability"]
```

```rust
use weft_plugin_sdk::*;

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or_default();
    let result = match req.action.as_str() {
        "describe" => PluginResult::ok(serde_json::json!({"package": "my-plugin"})),
        "health"   => PluginResult::ok(serde_json::json!({"healthy": true})),
        _ => PluginResult::err(format!("unknown action: {}", req.action)),
    };
    Ok(result.to_json())
}
```

Build:
```bash
cargo build --target wasm32-unknown-unknown --release
```

SDK: `https://gitea.alhz.org/weft-plugins/plugin-sdk`

---

## Key Design Principles

- **Core owns runtime truth** — suite lock, plugin activation, rollback all live in WEFT-core
- **wasm first** — plugins should be wasm unless they genuinely need a separate process
- **Tools are unified** — all tool calls go through `tool-runtime-core`
- **Wiki is the durable surface** — anything worth remembering goes to wiki
