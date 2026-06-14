# Weft

Weft is a modular AI agent platform. A Rust core runtime exposes an
OpenAI-compatible API and a capability-based plugin system; behavior is
extended by WASM and native **packages**; and a cross-platform Flutter desktop
client drives it all — including multi-agent team orchestration, persistent
memory, skills, and tool use.

---

## Table of contents

- [What Weft can do](#what-weft-can-do)
- [Architecture](#architecture)
- [The capability system](#the-capability-system)
- [Official packages](#official-packages)
- [Repository layout](#repository-layout)
- [Build](#build)
- [Run](#run)
- [Configuration](#configuration)
- [Continuous integration](#continuous-integration)
- [License](#license)

---

## What Weft can do

- **Talk to many LLM providers behind one API.** The core speaks the
  OpenAI-compatible protocol (`/v1/chat/completions`) and routes to OpenAI,
  Anthropic, DeepSeek, OpenRouter and others, with API-key failover/rotation
  and pluggable routing strategies.
- **Run multi-agent teams.** A workflow orchestrator fans a task out into
  parallel sub-tasks (a `depends_on` DAG), delegates them to role-based agents
  (planner, executor, reviewer, …), and streams each agent's output back to the
  UI. Agents can self-delegate large tasks and ask the user to choose between
  options mid-turn.
- **Use tools.** Built-in tool packages cover shell execution, file I/O, web
  access, and git — exposed to agents through a uniform tool-runtime contract.
- **Remember.** A curated, persistent memory runtime keeps context across
  sessions; a context engine ingests external signals and proactively suggests
  relevant skills.
- **Extend through skills and MCP.** Load and run skills, register external
  [MCP](https://modelcontextprotocol.io/) servers, and surface their tools to
  agents.
- **Schedule work.** A cron capability runs scheduled jobs and maintenance
  ticks.
- **Ship products on top.** Product packages such as `weft-claw` (a multi-role
  AI development assistant) and `ai-director` (a style-learning AI video-editing
  assistant) are declared on top of the same runtime.

---

## Architecture

Weft has three layers: the **client**, the **core runtime**, and the
**packages** the core loads.

```
┌─────────────────────────────┐
│  weft_client (Flutter)       │   Windows / macOS / Linux desktop UI
│  chat · teams · DAG view ·   │
│  apps · packages · config    │
└──────────────┬──────────────┘
               │  HTTP  (OpenAI-compatible + management API)
               │  127.0.0.1:3004  (loopback, token-guarded)
┌──────────────▼──────────────┐
│  weft-core (Rust service)    │
│                              │
│  • OpenAI-compatible API     │  /v1/chat/completions, /v1/models
│  • Management API            │  /api/apps, /api/capabilities, /api/packages…
│  • Provider router           │  multi-provider + key failover/rotation
│  • Capability registry       │  binds capability ids → providers
│  • Package loader (Extism)   │  loads WASM + native packages
│  • Pipeline                  │  request → transform → provider → response
└──────────────┬──────────────┘
               │  capability calls
┌──────────────▼──────────────┐
│  Packages                    │
│  agent-runtime · memory ·    │  WASM (Extism) or native, declared in
│  tools · skills · mcp ·      │  packages/index.toml (the source authority)
│  workflow · team-runtime …   │
└─────────────────────────────┘
```

**Client ⇄ core** is process-to-process over a loopback HTTP API, so the same
core can serve the desktop client, scripts, or any OpenAI-compatible tool. The
core loads capability packages at runtime rather than compiling them in.

---

## The capability system

Everything the core can *do* beyond raw LLM routing is modeled as a
**capability** — a stable string id (e.g. `agent.runtime`, `memory.store`,
`tool.shell`, `workflow.orchestration`) that a package *provides* and that apps
*require*. `packages/index.toml` is the **source authority**: it maps each
capability to the package that provides it, the package kind (`provider` /
`product`), and its runtime (`wasm` / `native` / `embedded`).

At startup the core builds a capability registry, resolves each app's required
capabilities to concrete providers (`bindings`), and dispatches capability calls
to the right package. This is what lets products like `weft-claw` be assembled
declaratively from agent-runtime + memory + tools + skills without hard-coding
any of them.

---

## Official packages

The 28 packages under `packages/official/` group into a few families:

**Agent & orchestration**
| Package | Provides | Role |
|---|---|---|
| `agent-core` | `agent.runtime`, `team.delegate` | Agent turns, session-aware dialog, tool dispatch |
| `workflow-orchestrator` | `workflow.orchestration` | Task proposal, verification, DAG step orchestration |
| `team-runtime` | `team.runtime`, `team.role.catalog`, `team.context.shared` | Devteam roles, shared context, delegate routing |
| `team-task-board` | `team.taskboard`, `team.handoff` | Task board + cross-role handoff |
| `generic-agent-runtime` | `generic_agent.plan/run/verify/crystallize` | Experimental self-evolving task runtime |
| `workflow-template-devteam`, `workflow-template-creative` | workflow templates | Prebuilt team/creative workflows |

**Memory & context**
| Package | Provides | Role |
|---|---|---|
| `memory` / `memory-runtime` | `memory.store`, `memory.runtime`, `memory.curated` | Persistent curated memory |
| `context-engine` | `context.engine`, `context.ingest`, `context.match` | Ingests signals, suggests skills |
| `prompt-system` | `prompt.system` | System-prompt management |
| `session-events` | session events | Session lifecycle events |

**Tools & extension**
| Package | Provides | Role |
|---|---|---|
| `tool-runtime-core` | `tool.runtime` | Uniform tool dispatch contract |
| `tool-shell` / `tool-files` / `tool-web` / `tool-git` | `tool.shell/files/web/git` | Built-in tools |
| `skills` | `ext.skills`, `skills.evolution/governance/review/maintenance` | Skill discovery, loading, execution |
| `mcp-client` | `ext.mcp` | Register MCP servers, expose their tools |
| `js-extension-runtime` | JS extensions | Run JavaScript extensions |
| `channels` | `channel.bridge` | Channel bridge & routing |
| `cron` | `scheduler.cron`, `maintenance.tick` | Scheduled jobs |

**Products & media**
| Package | Role |
|---|---|
| `weft-claw` | Multi-role AI development assistant (product) |
| `ai-director` | Style-learning AI video-editing assistant (product) |
| `ffmpeg-runtime`, `image-gen` | Media: ffmpeg runtime, image generation |
| `ai-workspace-browser`, `creative-role-catalog` | Workspace browser, creative role catalog |

> Capabilities the **core itself** provides (not a package) include
> `core.execution` (command execution, dry-run gated) and `core.files`
> (workspace file access).

---

## Repository layout

```
core/                 Rust core runtime (weft-core, weft, weft-sign, weft-rpc)
crates/               Supporting crates (weft-code-runtime)
packages/
  sdk/                Shared package SDK (path dependency for all packages)
  official/           The 28 official packages
  installed/          Installed package metadata (manifests; wasm built separately)
  index.toml          Source authority: capability → package mapping
  weft-code/          Product declaration
clients/
  weft_client/        Flutter desktop client (Windows/macOS/Linux)
config.example.toml   Provider config template
.github/workflows/    CI
scripts/precheck.ps1  Pre-commit safety check
```

---

## Build

### Core (Rust)

Requires a stable Rust toolchain.

```bash
cargo build --workspace --release
```

Produces `weft-core` (the service) plus `weft`, `weft-sign`, `weft-rpc`.

WASM packages are built separately for the `wasm32-wasip1` target:

```bash
cargo build -p <package> --target wasm32-wasip1 --release
```

### Client (Flutter)

Requires the Flutter SDK (`>=3.12`).

```bash
cd clients/weft_client
flutter pub get
flutter run -d windows   # or: macos / linux
```

---

## Run

```bash
# 1. Configure providers
cp config.example.toml config/config.toml
#    edit config/config.toml and add your provider API keys

# 2. Start the core (listens on 127.0.0.1:3004 by default)
cargo run --release --bin weft-core

# 3. Launch the desktop client (connects to the core automatically)
cd clients/weft_client && flutter run -d windows
```

---

## Configuration

Copy `config.example.toml` to `config/config.toml` and fill in provider keys:

```toml
[core]
host = "127.0.0.1"
port = 3004

[[providers]]
# name, base_url, api keys, ...
```

`config/config.toml` is gitignored — only the example template is committed.

---

## Continuous integration

GitHub Actions ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs on
every push and pull request:

- **Rust** — `cargo build --workspace` and `cargo test` for the native crates.
- **Flutter** — `flutter pub get` and `flutter analyze` for the client.

---

## License

Licensed under the [Apache License 2.0](LICENSE).
