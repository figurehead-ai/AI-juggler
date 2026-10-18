# ADR-0012: Two-clock quota model

**Status:** Accepted
**Date:** 2026-10-13
**Deciders:** Vuxtra engineering + maintainer (decision-log D-12)
**Implements:** D-12 (and SPEC §6)

## Context

"Quota" conflates two different clocks: vendor usage windows (5h /
weekly pools, credits) and the subscription's renewal/expiry. Pass-1
proposed inferring renewal dates; the maintainer ruled them
user-entered and optional unless a sanctioned source is proven
(D-12's mandatory research: AJ-E909/E601-E605).

## Decision

1. **Window clock** (5h/weekly/credits): measured/computed/estimated
   from per-message token events + quota feeds; drives aggregates,
   burn-down projections, reset timers, threshold alerts — and the
   switch *suggestion* at the wall. It is a trigger, not the headline
   (D-01).
2. **Subscription clock** (renewal/expiry): user-entered, optional.
   Without a date: `Unknown — no renewal date entered`, never
   inferred. Automated population only from a sanctioned, read-only,
   live-verified Provider source proven by AJ-E909; unproven stays
   unknown.
3. Honesty labels (Measured/Computed/Estimated) and per-model
   attribution are normative in every surface.

## Alternatives considered

- **Infer renewal from quota-reset patterns** — rejected: guesses
  billing facts; D-12 explicitly forbids.
- **Single unified "quota" number** — rejected: hides which clock is
  about to bite, exactly when the user needs to decide to switch.

## Consequences

E6 builds the two engines; expiry ships beta on manual entry; AJ-E909
records the per-Provider research verdict table that gates any future
automation.
