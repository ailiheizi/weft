# WEFT Existing Tool Plugin Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Use WEFT's existing physical `tool-*` plugins as the real callable tool layer, and leave `skills` as the `SKILL.md` catalog/runtime.

**Architecture:** WEFT already has physical tool plugins: `tool-runtime-core`, `tool-web`, `tool-files`, `tool-shell`, and `tool-git`. The bug is not absence of tool plugins; the bug is that `agent-core` currently gets callable tool specs and runs `execute_tool` through the `skills` plugin. This plan wires `agent-core`, `companion-core`, and `context-engine` to existing `tool-*` plugins, then removes callable builtin tool execution from `skills`.

**Tech Stack:** Rust Extism plugins (`agent-core`, `skills`, `tool-*`), Python service plugins (`companion-core`, `context-engine`), TypeScript/Python contract tests, WASI target `wasm32-wasip1`.

---

## Current Reality

Existing physical tool plugins:

- `weft-plugins/tool-runtime-core`: currently exposes `describe`, `health`, `dispatch/call`; mostly a placeholder dispatcher.
- `weft-plugins/tool-web`: currently exposes `describe`, `health`, `fetch/call`; placeholder-ish web provider.
- `weft-plugins/tool-files`: currently exposes `list`, `read`, `write`; real file operations exist here.
- `weft-plugins/tool-shell`: currently exposes `run/call`; real shell execution exists here.
- `weft-plugins/tool-git`: physical plugin exists.

Current actual agent path:

- `agent-core` calls `skills.get_tool_specs`.
- `agent-core` calls `skills.handle_ws_message` with `action: execute_tool`.
- `companion-core` calls `skills` for executable tools like `search_web` and `wiki_write`.
- `context-engine` calls `skills` for executable `blink_once`.

So the correction is: **do not create a new tool plugin. Wire the existing physical tool plugins into the agent tool path.**

---

## Target Boundary

- `skills`: only `SKILL.md` discovery, `list_skills`, `read_skill`, agent skill allowlist, prompt guidance.
- `tool-runtime-core`: aggregator/router for callable tool specs and dispatch.
- `tool-files`: file tools such as `fs_read`, `fs_write`, `fs_list`, `read_file` where appropriate.
- `tool-shell`: shell execution.
- `tool-web`: `fetch_url`, `web_fetch`, `search_web` provider chain.
- `tool-runtime-core` or a dedicated existing tool plugin: desktop/context tools such as `blink_once`, `read_document`, `take_screenshot`, `analyze_screen_deep`, `meeting_notes`, `meeting_record`, `mode_tool`, `wiki_write` until later split further.

---

## Task 1: Lock the Existing-Tool-Plugin Contract

**Files:**
- Create: `weft/scripts/test-existing-tool-plugin-wiring-contract.ts`

- [ ] **Step 1: Add contract test**

Create `weft/scripts/test-existing-tool-plugin-wiring-contract.ts`:

```ts
import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'
import assert from 'node:assert/strict'

const repoRoot = path.resolve(import.meta.dirname, '..', '..')

for (const plugin of ['tool-runtime-core', 'tool-web', 'tool-files', 'tool-shell', 'tool-git']) {
  assert.equal(existsSync(path.join(repoRoot, 'weft-plugins', plugin, 'plugin.toml')), true, `${plugin} should already exist`)
  assert.equal(existsSync(path.join(repoRoot, 'weft-plugins', plugin, 'src', 'lib.rs')), true, `${plugin} should have Rust source`)
}

const agentSource = readFileSync(path.join(repoRoot, 'weft-plugins', 'agent-core', 'src', 'lib.rs'), 'utf8')
const skillsSource = readFileSync(path.join(repoRoot, 'weft-plugins', 'skills', 'src', 'lib.rs'), 'utf8')
const toolRuntimeCore = readFileSync(path.join(repoRoot, 'weft-plugins', 'tool-runtime-core', 'src', 'lib.rs'), 'utf8')

assert.match(agentSource, /tool-runtime-core/, 'agent-core should use existing tool-runtime-core for tool specs/dispatch')
assert.doesNotMatch(agentSource, /call_plugin\(&skills_plugin, "get_tool_specs"/, 'agent-core must not get tool specs from skills')
assert.doesNotMatch(agentSource, /execute_skill_action\(skills_plugin/, 'agent-core must not execute tools through skills')

assert.match(toolRuntimeCore, /get_tool_specs/, 'tool-runtime-core should expose get_tool_specs')
assert.match(toolRuntimeCore, /execute_tool/, 'tool-runtime-core should expose execute_tool')

assert.doesNotMatch(skillsSource, /pub fn get_tool_specs/, 'skills must not export callable tool specs')
assert.doesNotMatch(skillsSource, /pub fn execute_tool/, 'skills must not execute tools')
assert.match(skillsSource, /do_read_skill/, 'skills should keep SKILL.md reading')
assert.match(skillsSource, /list_skill_catalog/, 'skills should keep SKILL.md catalog')

console.log('existing tool plugin wiring contract passed')
```

- [ ] **Step 2: Run and confirm failure**

```powershell
cd D:\weft\weft
npx tsx scripts/test-existing-tool-plugin-wiring-contract.ts
```

Expected: fails because `agent-core` still calls `skills.get_tool_specs` and `tool-runtime-core` does not yet expose `get_tool_specs`/`execute_tool`.

- [ ] **Step 3: Commit failing contract**

```powershell
cd D:\weft
```

---

## Task 2: Promote `tool-runtime-core` Into The Tool Aggregator

**Files:**
- Modify: `weft-plugins/tool-runtime-core/src/lib.rs`
- Modify: `weft-plugins/tool-runtime-core/plugin.toml`

- [ ] **Step 1: Add `get_tool_specs` export**

Implement `get_tool_specs` in `tool-runtime-core` returning OpenAI-style function specs for the current callable tools.

Start with tools that already exist in `skills` today:

```text
fs_read, fs_write, fs_list, shell_exec, web_fetch, fetch_url, search_web,
read_file, read_document, take_screenshot, blink_once, analyze_screen_deep,
meeting_notes, meeting_record, mode_tool, wiki_write, wiki_contract
```

- [ ] **Step 2: Add `execute_tool` export**

Implement `execute_tool` in `tool-runtime-core` as a router:

```text
fs_* / read_file -> tool-files
shell_exec -> tool-shell
web_fetch / fetch_url / search_web -> tool-web
everything not yet physically split -> temporary local implementation moved from skills or delegated to a transitional internal module
```

Do not route execution through `skills`.

- [ ] **Step 3: Update `plugin.toml`**

Ensure `tool-runtime-core/plugin.toml` declares:

```toml
provides = ["tool.runtime", "tool.registry", "tool.execution"]
```

- [ ] **Step 4: Compile**

```powershell
cd D:\weft\weft-plugins\tool-runtime-core
cargo check --target wasm32-wasip1
```

Expected: pass.

- [ ] **Step 5: Commit**

```powershell
cd D:\weft
```

---

## Task 3: Move Callable Tool Implementations Out Of `skills`

**Files:**
- Modify: `weft-plugins/skills/src/lib.rs`
- Modify: `weft-plugins/tool-runtime-core/src/lib.rs`
- Modify: `weft-plugins/tool-web/src/lib.rs`
- Modify: `weft-plugins/tool-files/src/lib.rs`
- Modify: `weft-plugins/tool-shell/src/lib.rs`

- [ ] **Step 1: Move file/shell/web code to existing tool plugins**

Move or duplicate first, then remove from `skills` after tests pass:

```text
fs_read/fs_write/fs_list/read_file -> tool-files
shell_exec -> tool-shell
web_fetch/fetch_url/search_web/Tavily/Brave/DuckDuckGo provider chain -> tool-web
```

- [ ] **Step 2: Move remaining desktop/context tools to `tool-runtime-core`**

Move these from `skills` into `tool-runtime-core` for now:

```text
read_document, take_screenshot, blink_once, analyze_screen_deep,
meeting_notes, meeting_record, mode_tool, wiki_write, wiki_contract
```

They can be split into smaller tool plugins later. The immediate goal is separating tools from skills.

- [ ] **Step 3: Remove callable exports from `skills`**

Remove from `skills/src/lib.rs`:

```text
get_tool_specs
execute_tool
list_tools action branch
get_tool_specs action branch
execute_tool action branch
builtin_tool_defs
do_execute_tool
```

Keep:

```text
list_skills
list_available
read_skill
enable/disable/set_for_agent
OpenClaw skill discovery
```

- [ ] **Step 4: Compile affected plugins**

```powershell
cd D:\weft\weft-plugins\tool-runtime-core
cargo check --target wasm32-wasip1
cd D:\weft\weft-plugins\tool-web
cargo check --target wasm32-wasip1
cd D:\weft\weft-plugins\tool-files
cargo check --target wasm32-wasip1
cd D:\weft\weft-plugins\tool-shell
cargo check --target wasm32-wasip1
cd D:\weft\weft-plugins\skills
cargo check --target wasm32-wasip1
```

Expected: all pass.

- [ ] **Step 5: Commit move**

```powershell
cd D:\weft
```

---

## Task 4: Wire `agent-core` To Existing Tool Plugins

**Files:**
- Modify: `weft-plugins/agent-core/src/lib.rs`

- [ ] **Step 1: Add separate tool plugin config**

Add a default:

```rust
fn default_tool_plugin() -> String {
    "tool-runtime-core".into()
}
```

Add `tool_plugin` to config structs with serde default. Keep `skills_plugin` for `read_skill` only.

- [ ] **Step 2: Use `tool_plugin` for specs and execution**

Change:

```rust
call_plugin(&skills_plugin, "get_tool_specs", &tools_input)
```

to:

```rust
call_plugin(&tool_plugin, "get_tool_specs", &tools_input)
```

Change tool execution from `skills_plugin` to `tool_plugin`.

- [ ] **Step 3: Keep bound skill prompt context on `skills_plugin`**

Do not change `build_bound_skill_context`; it should still call `read_skill` on `skills`.

- [ ] **Step 4: Compile**

```powershell
cd D:\weft\weft-plugins\agent-core
cargo check --target wasm32-wasip1
```

Expected: pass.

- [ ] **Step 5: Commit**

```powershell
cd D:\weft
```

---

## Task 5: Wire Services To Existing Tool Plugins

**Files:**
- Modify: `weft-plugins/companion-core/server.py`
- Modify: `weft-plugins/context-engine/server.py`
- Modify tests that assert plugin names.

- [ ] **Step 1: Change companion executable calls**

All companion `execute_tool` calls should target `tool-runtime-core`, not `skills`.

- [ ] **Step 2: Change context-engine `blink_once` calls**

Context-engine visual enrichment should call `tool-runtime-core.execute_tool` for `blink_once`.

- [ ] **Step 3: Run tests**

```powershell
cd D:\weft\weft
python -m pytest -q scripts/test-companion-proactive-mode-tool-contract.py
npx tsx scripts/test-context-engine-runtime-bridge-contract.ts
npx tsx scripts/test-existing-tool-plugin-wiring-contract.ts
```

Expected: pass.

- [ ] **Step 4: Commit**

```powershell
cd D:\weft
```

---

## Task 6: Update Packaging And Official Index

**Files:**
- Modify: `packages/index.toml`
- Create or modify: `packages/official/tool-runtime-core/plugin.toml` if official fixture packaging needs it.
- Modify: runtime packaging tests.

- [ ] **Step 1: Add official package index entries if missing**

`packages/index.toml` currently lists `skills-runtime` but not the physical `tool-*` plugins. Add package entries for at least:

```text
tool-runtime-core
tool-web
tool-files
tool-shell
tool-git
```

Use existing package entry format and local fixture signing rules already used in this repo.

- [ ] **Step 2: Update runtime packaging tests**

Ensure active install includes all `tool-*` plugins and their `plugin.wasm` files.

- [ ] **Step 3: Run packaging tests**

```powershell
cd D:\weft\weft
npx tsx scripts/test-weft-service-runtime-packaging.ts
```

Expected: pass.

- [ ] **Step 4: Commit packaging**

```powershell
cd D:\weft
```

---

## Task 7: Build, Sync, And Verify

**Files:**
- Modify `.wasm` artifacts for changed plugins.

- [ ] **Step 1: Build changed WASM plugins**

```powershell
cd D:\weft\weft-plugins\tool-runtime-core
cargo build --target wasm32-wasip1 --release
cd D:\weft\weft-plugins\tool-web
cargo build --target wasm32-wasip1 --release
cd D:\weft\weft-plugins\tool-files
cargo build --target wasm32-wasip1 --release
cd D:\weft\weft-plugins\tool-shell
cargo build --target wasm32-wasip1 --release
cd D:\weft\weft-plugins\skills
cargo build --target wasm32-wasip1 --release
cd D:\weft\weft-plugins\agent-core
cargo build --target wasm32-wasip1 --release
```

- [ ] **Step 2: Copy release artifacts to each plugin's `plugin.wasm`**

Copy each built release `.wasm` from `target/wasm32-wasip1/release/` to its plugin root as `plugin.wasm`.

- [ ] **Step 3: Refresh installed runtime**

Use the existing WEFT runtime refresh path. Verify installed hashes for changed plugins match repo hashes.

- [ ] **Step 4: Full verification**

```powershell
cd D:\weft\weft
npx tsx scripts/test-existing-tool-plugin-wiring-contract.ts
npx tsx scripts/test-proactive-tool-contract.ts
npx tsx scripts/test-openclaw-skills-plugin-contract.ts
npx tsx scripts/test-context-engine-runtime-bridge-contract.ts
npx tsx scripts/test-core-agent-ready-order-contract.ts
npx tsx scripts/test-agent-core-memory-readonly-contract.ts
python -m pytest -q scripts/test-companion-proactive-mode-tool-contract.py
```

Expected: all pass.

- [ ] **Step 5: Live smoke**

Run a live agent web-search/write test and confirm debug trace shows tool execution through `tool-runtime-core`, not `skills`.

- [ ] **Step 6: Commit artifacts**

```powershell
cd D:\weft
```

---

## Self-Review

- This plan corrects the earlier wrong assumption that WEFT lacks physical tool plugins.
- The actual target is wiring existing `tool-*` plugins into the agent execution path and removing tool execution from `skills`.
- No new `tool-runtime` plugin is proposed.
