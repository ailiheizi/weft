# OpenClaw Web Search Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace WEFT's hard-coded web search fallback with an OpenClaw-style provider chain.

**Architecture:** `search_web` resolves providers automatically. Brave is API-only and requires `BRAVE_API_KEY`; DuckDuckGo HTML is the keyless fallback with cache and challenge detection.

**Tech Stack:** Rust WASI plugin, Node DDG helper script, TypeScript contract tests, existing WEFT plugin direct-call API.

---

### Task 1: Lock Provider Contract Tests

**Files:**
- Modify: `weft/scripts/test-proactive-tool-contract.ts`
- Modify: `weft/scripts/test-openclaw-external-skill-live.ts`

- [ ] Add assertions that Brave HTML scraping is absent.
- [ ] Add assertions that Brave API key handling exists.
- [ ] Add assertions that DDG remains the keyless fallback.
- [ ] Run `npx tsx "weft/scripts/test-proactive-tool-contract.ts"` and expect failure before implementation if the code still contains Brave HTML fallback.

### Task 2: Replace Brave HTML With API-Only Provider

**Files:**
- Modify: `weft-plugins/skills/src/lib.rs`

- [ ] Remove `search.brave.com` HTML fallback code and parser.
- [ ] Add `resolve_brave_api_key()` using env/config-compatible lookup.
- [ ] Add Brave API call to `https://api.search.brave.com/res/v1/web/search` with `X-Subscription-Token`.
- [ ] Return missing-key as provider unavailable during auto selection.
- [ ] Run `cargo build --release --target wasm32-wasip1` in `weft-plugins/skills` and expect success.

### Task 3: Make DDG The Keyless Provider

**Files:**
- Modify: `weft-plugins/skills/openclaw-skills/ianwchoi-duckduckgo-websearch/scripts/ddg_search.js`
- Modify: `weft-plugins/skills/src/lib.rs`

- [ ] Reduce DDG helper to one primary endpoint: `https://html.duckduckgo.com/html` or Lite if explicitly selected by contract.
- [ ] Add cache keyed by query, count, region, and safe-search.
- [ ] Keep bot challenge detection and return explicit failure.
- [ ] Ensure provider output is `provider: "duckduckgo"` and `source: "DuckDuckGo HTML"`.

### Task 4: Add Provider Auto Selection

**Files:**
- Modify: `weft-plugins/skills/src/lib.rs`

- [ ] Implement provider order: configured Brave API first, then DuckDuckGo keyless.
- [ ] Allow fallback only when provider was auto-selected.
- [ ] If user explicitly selects a provider and it fails, return that error.
- [ ] Preserve the `search_web` public tool schema.

### Task 5: Verify And Deploy Runtime Artifacts

**Files:**
- Runtime copy targets under `C:\Users\Admin\AppData\Roaming\weft-desktop\weft-core-config\plugins\installed\skills\`

- [ ] Run `npx tsx "weft/scripts/test-proactive-tool-contract.ts"`.
- [ ] Run `npx tsx "weft/scripts/test-openclaw-external-skill-live.ts"`.
- [ ] Run `cargo build --release --target wasm32-wasip1` in `weft-plugins/skills`.
- [ ] Copy the built `plugin_skills.wasm` to installed runtime.
- [ ] Copy the updated DDG script to installed runtime.
- [ ] Restart or refresh core if needed.
- [ ] Live-call `search_web` and confirm provider/source are explicit.
