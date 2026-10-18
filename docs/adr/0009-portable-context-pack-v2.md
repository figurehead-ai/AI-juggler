# ADR-0009: Portable context pack v2

**Status:** Accepted
**Date:** 2026-10-13
**Deciders:** Vuxtra engineering + maintainer (decision-log D-14)
**Implements:** D-14 (and SPEC §3.3, §7)

## Context

The pack is the staging/preview artifact for every switch and export.
Pass-1 proposed an always-withhold redaction default; the maintainer
ruled (D-14) that secret handling is the user's remembered choice — a
hidden product default must never decide it.

## Decision

1. Pack v2 layout: `PACK.md`, `conversation.md`, `state.json`
   (including Switch Graph lineage embedding), `manifest.json`
   (redaction manifest), `diffs/`. Deterministic generation from the
   Transcript IR.
2. **Secret policy is user-owned:** a global policy chosen explicitly
   at first run + per-Provider overrides, remembered. The manifest
   lists every secret-shaped artifact detected and how the *active*
   policy treated it; the active policy for the destination Provider
   is visible before any switch executes.
3. The pack is the substrate that lets a Provider-pair's fidelity
   graduate (Copy/Translate/Regenerate/Drop per artifact, §7.1).

## Alternatives considered

- **Always-withhold redaction default** — rejected by D-14: imposes a
  product decision the maintainer wants the user to own.
- **Prompt on every switch** — rejected: violates the one-click
  proven-pair path (D-03); remembered policy + visible-before-execute
  gives the same safety without the friction.

## Consequences

First-run adds an explicit policy choice; Provider settings gain an
override; E2/E208 implement storage and enforcement; the manifest makes
redaction auditable after the fact.
