## WEFT Frontend Upstream Gap

Date: 2026-04-16

### Conclusion

Current local desktop Electron code in `D:\weft\weft` cannot be cleanly upstreamed to current Gitea `weft/frontend` as a minimal MR for commit `0dd6a38`.

Reason: the public `weft/frontend` repository is not carrying the same desktop host/runtime layer as local `D:\weft\weft`.

### Evidence

- Local desktop repo path:
  - `D:\weft\weft`
- Gitea frontend clone path:
  - `D:\tmpweft\frontend-mr`

Main-process file count:

- Local `D:\weft\weft\src\main`: 61 files
- Gitea `D:\tmpweft\frontend-mr\src\main`: 1 file

Current Gitea frontend main process file:

- `src/main/index.ts`

Local-only desktop host/runtime layers include:

- `src/main/services/weft-core-host-service.ts`
- `src/main/services/managed-service.ts`
- `src/main/services/service-manager.ts`
- `src/main/services/weft-core-runtime-bootstrap.ts`
- `src/main/services/weft-core-runtime-path.ts`
- `src/main/services/weft-plugin-package-manifest.ts`
- `src/main/services/weft-plugin-source-roots.ts`
- `src/main/services/weft-suite-sync-engine.ts`
- `src/main/weft-core-api-client.ts`
- `src/main/weft-core-session-store.ts`
- `src/main/ipc-handlers.ts`

### Why `0dd6a38` cannot be sent as a clean frontend MR now

Commit `0dd6a38` changes:

- `src/main/services/weft-core-host-service.ts`
- `scripts/test-weft-plugin-refreshes-on-wasm-hash-change.ts`

Those files and their supporting host-service graph do not exist in current Gitea `weft/frontend`.

So a direct MR would require first upstreaming a much larger desktop host/runtime base, not just the wasm refresh fix.

### What was actually upstreamable

The actually upstreamable runtime-compatible fix in this session was `agent-core`, because the public `weft-plugins/agent-core` repo contains the matching codebase and accepted a clean MR.

Existing MR:

- `https://gitea.alhz.org/weft-plugins/agent-core/pulls/3`

### Practical next step

Before any desktop/frontend MR for `0dd6a38`, one of these must happen:

1. The public `weft/frontend` repo is first updated to include the current desktop host/runtime layer.
2. A different public upstream repo is identified that already contains this host/runtime layer.
3. The local desktop host/runtime layer is intentionally split and published before sending follow-up fixes.
