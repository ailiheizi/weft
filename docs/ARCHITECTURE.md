# Weft Architecture

Weft is built in three layers — a **client**, a **core runtime**, and the
**packages** the core loads — connected by a capability system that lets
products be assembled declaratively instead of compiled in.

- [The big picture](#the-big-picture)
- [Layer 1 — the client](#layer-1--the-client)
- [Layer 2 — the core runtime](#layer-2--the-core-runtime)
- [Layer 3 — packages](#layer-3--packages)
- [The capability system](#the-capability-system)
- [Package runtimes](#package-runtimes)
- [The request pipeline](#the-request-pipeline)
- [How a product is assembled](#how-a-product-is-assembled)
- [Repository layout](#repository-layout)

---

## The big picture

```
┌─────────────────────────────┐
│  weft_client (Flutter)       │   Windows / macOS / Linux desktop UI
│  chat · teams · DAG view ·   │
│  apps · packages · config    │
└──────────────┬──────────────┘
               │  HTTP  (OpenAI-compatible + management API)
               │  127.0.0.1:17830  (loopback, token-guarded)
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

## Layer 1 — the client

A cross-platform **Flutter desktop** app (`clients/weft_client`) for Windows,
macOS, and Linux. It is a thin, reactive UI over the core's HTTP API:

- **App shell + router** — a left nav rail wraps every screen (Dashboard, Chat,
  Orchestration, Packages, Providers, Services, Settings) plus a dynamic
  `/apps/:name` route for package-delivered surfaces.
- **Riverpod state** — screens watch providers that fetch from the core
  (apps, packages, providers, services, sessions, connection status) and
  re-render reactively, with loading skeletons and explicit error/offline states.
- **No business logic baked in** — the client never hard-codes what an app does.
  It asks the core which apps exist, which expose a `ui.surface`, and renders
  accordingly. New products show up in the launcher without rebuilding the client.

---

## Layer 2 — the core runtime

`weft-core` is the Rust service (in `core/`) that does the real work. It exposes
two HTTP API families on a loopback, token-guarded port (default
`127.0.0.1:17830`):

**OpenAI-compatible API**

| Path | Method | Purpose |
|---|---|---|
| `/v1/chat/completions` | POST | OpenAI-compatible chat (SSE streaming) |
| `/v1/models` | GET | List available models |

**Management API**

| Path | Method | Purpose |
|---|---|---|
| `/api/health` | GET | Health check |
| `/api/apps` | GET | Resolved apps + status |
| `/api/capabilities` | GET | Capability registry & bindings |
| `/api/packages` | GET | Installed packages |
| `/api/providers` | GET | Configured providers |

Internally the core is responsible for:

- **Provider router** — routes each request to one of many LLM providers
  (OpenAI, Anthropic, DeepSeek, OpenRouter, …) with API-key failover/rotation
  and pluggable routing strategies.
- **Capability registry** — at startup it reads the source authority, builds a
  registry mapping capability ids to providers, and resolves each app's required
  capabilities to concrete bindings.
- **Package loader** — discovers and loads packages (WASM via Extism, native, or
  embedded) and dispatches capability calls to them.
- **Pipeline** — the request → transform → provider → response flow, where each
  stage can be overridden by a package.

The core also provides a few capabilities itself rather than via packages:
`core.execution` (command execution, dry-run gated) and `core.files` (workspace
file access).

---

## Layer 3 — packages

Packages are the unit of capability. A package declares what it **provides** and
**requires**, and the core wires it in. There are 31 official packages grouped
into agent/orchestration, memory/context, tools/extension, and products/media
families (see [FEATURES.md](FEATURES.md#capabilities-reference) for the full
table).

Packages come in three **kinds**:

- **`foundation`** — core building blocks other packages depend on
  (agent-runtime, session-events, tool-runtime-core, …).
- **`provider`** — provide a specific capability (a tool, a memory store, an
  MCP bridge, …).
- **`product`** — user-facing apps assembled from other capabilities
  (`weft-claw`, `ai-director`).

---

## The capability system

This is the heart of Weft. Everything the core can *do* beyond raw LLM routing
is modeled as a **capability** — a stable string id such as `agent.runtime`,
`memory.store`, `tool.shell`, or `workflow.orchestration`.

- A package **provides** one or more capabilities.
- An app **requires** a set of capabilities.
- `packages/index.toml` is the **source authority**: it maps each capability to
  the package that provides it, the package kind, its runtime, and trust
  metadata (signatures, public keys, source authority).

At startup the core:

1. Reads `index.toml` and builds the capability registry.
2. Resolves each app's required capabilities to concrete providers (**bindings**).
3. Dispatches capability calls at runtime to the bound package.

Because binding is declarative, a product is just a list of required
capabilities — no provider is hard-coded.

---

## Package runtimes

`index.toml` declares how each package runs:

- **`wasm`** — compiled to `wasm32-wasip1` and sandboxed via **Extism**. Most
  tools and foundation packages are WASM: portable and isolated.
- **`service`** — long-lived native services managed by the core (memory
  runtime, context engine, generic-agent-runtime, JS extension runtime). These
  are what the [resident service manager](FEATURES.md#resident-service-manager)
  controls.
- **`embedded`** — compiled into the core (e.g. the AI workspace browser surface).

Trust is explicit: packages carry ed25519 signatures and a `source_authority`
(`official`, `local-installed`, …) recorded in the index.

---

## The request pipeline

A chat request flows through a staged pipeline in the core:

```
Request → Router → KeySelector → Transform → HTTP → Transform (response)
                                                ↘ ErrorHandler (on failure)
```

- **Router** picks the target provider.
- **KeySelector** chooses an API key (rotation / failover).
- **Transform** adapts the request to the provider's dialect, and the response
  back to OpenAI-compatible shape.
- **ErrorHandler** handles provider failures (e.g. failover to another key).

Each stage can be overridden by a package, which is how new providers or
behaviors are added without touching the core.

---

## How a product is assembled

`weft-claw` (the multi-role AI development assistant) is a good example. Its
package declares:

```toml
provides = ["weft_claw.turn", "ui.surface"]
requires = ["agent.runtime", "ext.skills", "ext.mcp",
            "memory.store", "tool.runtime", "session.events"]
```

At startup the core resolves each required capability to the package that
provides it — `agent-core` for `agent.runtime`, `skills` for `ext.skills`,
`memory` for `memory.store`, and so on — and binds them. The product itself
contains no provider logic; it's an orchestration of capabilities. Swapping the
memory implementation, adding a tool, or changing the agent runtime needs no
change to the product.

---

## Repository layout

```
core/                 Rust core runtime (weft-core, weft, weft-sign, weft-rpc)
crates/               Supporting crates (weft-code-runtime)
packages/
  sdk/                Shared package SDK (path dependency for all packages)
  official/           The 31 official packages
  installed/          Installed package metadata (manifests; wasm built separately)
  index.toml          Source authority: capability → package mapping
  weft-code/          Product declaration
clients/
  weft_client/        Flutter desktop client (Windows/macOS/Linux)
config.example.toml   Provider config template
docs/                 This documentation + screenshots
.github/workflows/    CI + release
scripts/              Build helpers (e.g. build-wasm-packages.sh)
```
