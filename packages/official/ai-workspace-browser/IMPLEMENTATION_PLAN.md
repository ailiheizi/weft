# AI Workspace Browser MVP Implementation Plan

**Goal:** Build a Weft plugin-shaped Electron browser package with an embedded Chromium browser (`WebContentsView`), browser-native chrome, screenshot-free context extraction, and grounded page control APIs.

**Architecture:** Electron main owns all browser tab `WebContentsView` instances. Renderer owns only app chrome UI and calls typed preload APIs. Page context and browser actions go through IPC so untrusted web content never gets Node access.

**Tech Stack:** Electron, Vite, React, TypeScript, `@mozilla/readability`, typed IPC through `contextBridge`.

---

## MVP Scope

1. Browser shell: top bar, address bar, back/forward/reload, tab list.
2. Embedded browser: each tab is a main-process `WebContentsView`.
3. Page context tools: read current page text/metadata, extract article markdown-ish text, scan DOM/AX/browser-native context without automatic screenshots.
4. Package shell placeholder: standalone wrapper for development until a Weft host loader is wired.
5. Safety defaults: `contextIsolation: true`, `nodeIntegration: false`, no cookies/localStorage exposed to renderer.

## File Structure

- `package.json` — plugin-shaped package metadata, exports, scripts, and dependencies.
- `plugin.json` — Weft package manifest and screenshot-free context policy.
- `tsconfig.json` — shared TypeScript settings.
- `vite.config.ts` — renderer build/dev server.
- `src/plugin/plugin.ts` — package entry placeholder and host-facing activation contract.
- `src/main/main.ts` — standalone Electron dev wrapper and window creation.
- `src/main/browser-runtime.ts` — `WebContentsView` tab lifecycle, navigation, bounds, page tools.
- `src/main/ipc.ts` — IPC handlers between renderer and main.
- `src/preload/preload.ts` — safe `window.browserAPI` bridge.
- `src/shared/types.ts` — shared types for tabs, tools, IPC payloads.
- `src/renderer/App.tsx` — browser chrome + AI sidebar UI.
- `src/renderer/main.tsx` — React entry.
- `src/renderer/styles.css` — layout and styling.
- `index.html` — Vite renderer root.

## Implementation Tasks

### Task 1: Project scaffold

- Create Electron/Vite/React TypeScript project files.
- Add scripts: `dev`, `build`, `typecheck`.
- Add minimal README.

### Task 2: Browser runtime

- Create `BaseWindow` and `WebContentsView` for renderer UI.
- Create one active browser tab as `WebContentsView`.
- Maintain tab state: id, url, title, loading, canGoBack, canGoForward.
- Resize active `WebContentsView` below top bar and next to sidebar.

### Task 3: Renderer browser chrome

- Display tabs.
- Add address bar and navigation buttons.
- Send navigation commands over preload IPC.
- Listen for tab state updates.

### Task 4: Page context tools

- Implement `readPage()` using `webContents.executeJavaScript` in the active tab.
- Implement `extractMarkdown()` with a simple DOM-to-text/Readability-style fallback.
- Implement screenshot-free Observe using DOM, AX, forms, tables, runtime events, performance/storage signals, and grounded refs.

### Task 5: Plugin-shaped browser tools

- Add browser-native tools for extraction, Observe, refs, memory, and grounded actions.
- Keep standalone development available without requiring Weft host dependencies.

### Task 6: Verification

- Run install.
- Run typecheck.
- Run build.
- Launch dev app and manually verify navigation and page tools.

## Acceptance Criteria

- App launches with an embedded browser visible.
- Address bar can navigate to a URL.
- Back/forward/reload buttons work when available.
- Read Page returns current page title, URL, and text excerpt.
- Observe returns screenshot-free DOM/AX/page/runtime context.
- TypeScript typecheck passes.
- Production build passes.
