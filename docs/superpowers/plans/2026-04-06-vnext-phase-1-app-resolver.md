# WEFT vNext Phase 1 Resolver Implementation Plan

> Historical note, preserved for migration chronology only.

## Status

This 2026-04-06 plan documented an early resolver slice before the repository converged on the current Product Package + Instance model.

## What this plan represented at the time

- introduce a first resolver path in Core
- surface resolved product metadata in runtime state
- keep the existing plugin runtime intact during the first migration step

## Current semantic replacement

This plan is obsolete as an implementation source.

Current product semantics are:

- canonical product package: `packages/weft-claw/package.toml`
- canonical runtime instance config: `.weft/weft-claw/config.toml`
- canonical runtime instance lock: `.weft/weft-claw/lock.toml`

## Guidance

When reading old migration history, interpret this phase as the earliest resolver groundwork that was later superseded by the weft-claw Product Package + Instance model.
