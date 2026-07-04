# Weft Build Guide

How to build Weft from source after cloning.

---

## Prerequisites

- Windows 10/11
- Node.js 24+
- Rust + Cargo (`rustup target add wasm32-unknown-unknown`)
- Python 3.11+

---

## 1. Clone all repos

```bash
# Core
git clone https://gitea.alhz.org/weft/WEFT-core
git clone https://gitea.alhz.org/weft/frontend
git clone https://gitea.alhz.org/weft/server

# Plugins
git clone https://gitea.alhz.org/weft-plugins/agent-core
git clone https://gitea.alhz.org/weft-plugins/channels
git clone https://gitea.alhz.org/weft-plugins/companion-core
git clone https://gitea.alhz.org/weft-plugins/context-engine
git clone https://gitea.alhz.org/weft-plugins/cron
git clone https://gitea.alhz.org/weft-plugins/mcp-client
git clone https://gitea.alhz.org/weft-plugins/memory-runtime
git clone https://gitea.alhz.org/weft-plugins/prompt-system
git clone https://gitea.alhz.org/weft-plugins/skills
git clone https://gitea.alhz.org/weft-plugins/suite-manager
git clone https://gitea.alhz.org/weft-plugins/tool-runtime-core
git clone https://gitea.alhz.org/weft-plugins/workflow-orchestrator

# SDK (required by all wasm plugins)
git clone https://gitea.alhz.org/weft-plugins/plugin-sdk
```

Expected layout:

```
D:/weft/
├── weft/              (frontend)
├── WEFT-core/
├── server/
├── packages/
│   └── sdk/            (plugin-sdk)
└── weft-plugins/
    ├── agent-core/
    ├── channels/
    ├── companion-core/
    ├── context-engine/
    ├── cron/
    ├── mcp-client/
    ├── memory-runtime/
    ├── prompt-system/
    ├── skills/
    ├── suite-manager/
    ├── tool-runtime-core/
    └── workflow-orchestrator/
```

The `plugin-sdk` must be at `../../packages/sdk` relative to each plugin (i.e. `D:/weft/packages/sdk`). This path is hardcoded in each plugin's `Cargo.toml`.

---

## 2. Build wasm plugins

Each plugin in `weft-plugins/` needs to be compiled to wasm before WEFT-core can load it.

```bash
# Add wasm target once
rustup target add wasm32-unknown-unknown

# Build each plugin
for plugin in agent-core channels cron mcp-client prompt-system suite-manager tool-runtime-core workflow-orchestrator; do
  cd D:/weft/weft-plugins/$plugin
  cargo build --target wasm32-unknown-unknown --release
  cp target/wasm32-unknown-unknown/release/plugin_*.wasm plugin.wasm
done
```

Or run them individually:

```bash
cd D:/weft/weft-plugins/tool-runtime-core
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/plugin_tool_runtime_core.wasm plugin.wasm
```

Plugin name → wasm filename mapping:

| Plugin | Wasm file |
|--------|-----------|
| agent-core | `plugin_agent_core.wasm` |
| channels | `plugin_channels.wasm` |
| cron | `plugin_cron.wasm` |
| mcp-client | `plugin_mcp_client.wasm` |
| prompt-system | `plugin_prompt_system.wasm` |
| suite-manager | `plugin_suite_manager.wasm` |
| tool-runtime-core | `plugin_tool_runtime_core.wasm` |
| workflow-orchestrator | `plugin_workflow_orchestrator.wasm` |

---

## 3. Build WEFT-core

```bash
cd D:/weft/WEFT-core
cargo build --release
```

The binary will be at `target/release/weft-core.exe`.

---

## 4. Run the frontend

```bash
cd D:/weft/weft
npm install
npm run dev
```

This starts the Electron app. The main process automatically starts and manages WEFT-core.

---

## 5. Verify

After startup:

```
GET http://127.0.0.1:42617/api/health
GET http://127.0.0.1:42617/api/plugins
```

Expected plugins: `agent-core`, `channels`, `cron`, `mcp-client`, `prompt-system`, `suite-manager`, `tool-runtime-core`, `workflow-orchestrator`, `companion-core`, `memory-runtime`, `context-engine`, `skills`.

---

## Notes

- `companion-core`, `context-engine`, `memory-runtime` are Python services — they start automatically via the plugin runtime, no manual build needed.
- `skills` contains JS scripts — no build needed.
- `plugin.wasm` files are gitignored — you must build them locally.
- The `plugin-sdk` crate is a library dependency, not a plugin. Do not try to load it as a plugin.
