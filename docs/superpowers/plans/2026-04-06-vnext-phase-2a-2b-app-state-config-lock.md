# WEFT vNext Phase 2A + 2B State, Config, and Lock Implementation Plan

> Historical note, preserved for migration chronology only.

## Status

This 2026-04-06 plan captured the stage where runtime state, config parsing, and lock parsing were first being formalized.

## What this plan represented at the time

- expose resolved and unresolved runtime state
- parse runtime config and lock as first-class Core inputs
- enrich product detail output with resolution metadata

## Current semantic replacement

This plan is obsolete as an implementation source.

Current live paths are:

- product package declaration: `packages/weft-claw/package.toml`
- instance config: `.weft/weft-claw/config.toml`
- instance lock: `.weft/weft-claw/lock.toml`

## Guidance

Read this phase only as historical scaffolding that eventually converged into weft-claw Product Package + Instance semantics.
