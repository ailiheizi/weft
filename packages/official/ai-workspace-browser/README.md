# AI Workspace Browser Plugin

Plugin-shaped Electron browser package for Weft.

This package is currently self-contained so it can be developed and tested without a host package loader. The long-term shape is a Weft workspace package that provides an embedded browser window and screenshot-free browser-native context.

## Current layout

- `plugin.json` declares the package identity, entry, standalone entry, and browser capabilities.
- `src/plugin/plugin.ts` exports package metadata and a host-facing activation contract.
- `src/main/main.ts` is the standalone development wrapper.
- `src/main/browser-runtime.ts` is the browser engine.
- `src/renderer/App.tsx` is the browser chrome UI.
- `src/shared/types.ts` defines the browser API and context schema.

## Context policy

Default Observe does not take screenshots. Context comes from DOM, accessibility tree, browser state, runtime events, storage/resource/performance signals, and grounded element refs.

## Commands

- `npm test`
- `npm run typecheck`
- `npm run build`
- `npm run start`
