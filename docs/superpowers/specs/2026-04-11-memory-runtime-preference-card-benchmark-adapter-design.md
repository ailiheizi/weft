# Memory Runtime Preference Card Benchmark Adapter Design

## Goal

Repair the `single-session-preference` evaluation path for `memory-runtime` by replacing the current "large haystack plus loose retrieval snippets" benchmark adapter with a structured three-part input:

- `Preference Card`
- `Verbatim Evidence`
- `Answer Contract`

The goal is not to change the underlying storage stack or add benchmark-specific hacks. The goal is to make already-retrieved user preference memory usable by smaller answer models such as Haiku.

## Problem

The current clean Haiku `pref30` run shows a stable failure mode:

- retrieved preference evidence is present in the input record
- the final answer still drifts into generic advice
- benchmark judgments fail because the answer does not obey the remembered user-specific preference

This means the main problem is not "memory absent from storage". The main problem is "memory not compiled and handed to the answer model in a binding form".

## Scope

This design only changes the benchmark adapter path used by:

- `weft-plugins/memory-runtime/hermes_port/evidence_compiler.py`
- `tmp/longmemeval/run_claude_cli_longmemeval.py`
- focused tests for the evidence compiler and benchmark prompt packing

This design does not change:

- PostgreSQL schema
- Chroma / embedding / hybrid retrieval core
- general desktop runtime behavior
- non-preference LongMemEval categories

## Core Decision

Use a benchmark adapter that behaves like a compact memory handoff layer, not a raw transcript dump.

For `single-session-preference` questions, the answer model should receive:

1. `Preference Card`
   A short, structured summary of what must be respected in the answer.

2. `Verbatim Evidence`
   A few raw user utterances that directly justify the preference card.

3. `Answer Contract`
   An explicit generation contract that requires the answer to satisfy the card before offering suggestions.

This follows the useful part of `mempalace`:

- raw-first evidence
- short compiled memory in front of generation
- generation constrained by memory, not by a huge undifferentiated context

But it does not copy `mempalace`'s spatial object model.

## Preference Card Format

The preference card must be deterministic and small. It uses fixed sections:

- `Must Match`
- `Preferred Signals`
- `Avoid / Not Preferred`
- `Current Setup`
- `Open Unknowns`
- `Evidence Anchors`

The card is built only from retrieved evidence. No outside facts, no inferred product lists, no benchmark labels.

### Section intent

- `Must Match`
  Constraints that should directly shape the recommendation.

- `Preferred Signals`
  Positive hints that improve fit but are not strict hard rules.

- `Avoid / Not Preferred`
  Explicit dislikes or negative boundaries when supported by evidence.

- `Current Setup`
  User state that narrows the answer space, for example existing tools, owned gear, current workflow.

- `Open Unknowns`
  Gaps that remain unresolved after retrieval and should not be hallucinated away.

- `Evidence Anchors`
  Short references to the raw supporting evidence blocks.

## Verbatim Evidence Format

The adapter should attach one to three raw user evidence excerpts for the strongest preference signals.

Each evidence block should include:

- source session or corpus id
- date when available
- raw user wording

This should be short and direct. The purpose is to preserve "why this preference exists" without flooding the model with full haystack sessions.

## Answer Contract

The generation prompt for preference questions must explicitly instruct the model to:

- answer naturally
- follow `Must Match` first
- use `Preferred Signals` where possible
- do not violate `Avoid / Not Preferred`
- use `Current Setup` to narrow recommendations
- if evidence is partial, stay conservative only on the unsupported parts
- do not fall back to broad generic recommendation lists when the card is specific

This contract must not mention benchmark scoring or hidden labels.

## Non-Preference Questions

Non-preference question types keep the current prompt path.

This design is intentionally narrow:

- preference questions get the new adapter
- other categories remain unchanged

That keeps the change testable and reduces blast radius.

## Testing Strategy

Focused tests must prove four things:

1. The preference card is built with the new fixed sections.
2. Raw user evidence is preserved and attached as verbatim evidence.
3. The answer synthesis prompt includes the explicit answer contract.
4. Existing known `pref30` failures are representable through the card without benchmark-specific hardcoding.

## Success Criteria

This work is successful when:

1. The clean Haiku path remains free of Claude Code tool-persona leakage.
2. `single-session-preference` prompt packing becomes structured and deterministic.
3. `pref30` improves over the current clean-Haiku baseline of `0.3333`.
4. No new keyword hacks, regex special-casing, fallback answer injection, or benchmark-only branching are introduced.
