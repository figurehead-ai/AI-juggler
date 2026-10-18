# ADR-0011: Switch Graph and lineage

**Status:** Accepted
**Date:** 2026-10-13
**Deciders:** Vuxtra engineering + maintainer (decision-log D-03, D-05)
**Implements:** D-03, D-05 (and SPEC §3.1, §7)

## Context

The prior model was one directed handover plus a linear
`prev_handover` chain. The maintainer's workflow requires switching
"as many times I want and switching back and forth" — cycles, forks,
and N-hop returns. A chain forces the second return to pretend the
first never happened; and the v0 implementation over-claimed fidelity
(one-file transplant).

## Decision

1. **Switch Graph**: Work item (per Project) → instance-epoch nodes →
   append-only `SwitchEdge`s (mechanism: native-resume | transplant |
   replay | pack; pack id; fidelity shown; artifact manifest hash).
   Invariants S-G1 (append-only; undo = compensating edge), S-G2
   (per-Project; cross-project moves are exports), S-G3 (cycle-safe
   message-uuid dedupe, first-wins).
2. **Hybrid door (D-03):** one click + undo window for same-Provider
   proven pairs; itemised wizard (Copy/Translate/Regenerate/Drop +
   beta badge) for cross-Provider, low-fidelity, or diverged switches.
   Lossy switches can never happen silently.
3. **Switch-back (D-05):** resume the *original* native session (new
   epoch, same `native_id`) + one summarised delta turn with the pack
   attached. Copy-back is prohibited. Both-progressed divergence is
   surfaced by uuid three-way comparison, never silently merged.
4. Execution pipeline: freeze → stage (pack v2) → verify → launch →
   rollback; the freeze protocol covers mid-task switches (the quota
   wall hits mid-flight — that is the canonical case).

## Alternatives considered

- **Keep linear handover chains** — rejected: cannot represent
  A→B→A→C, forks, or merge; the product promise is cycles.
- **Switch-back as a second forward transplant (copy-back)** —
  rejected (D-05): overwrites the canonical original and destroys
  divergence information.
- **Full CRDT-style merge on divergence** — rejected for now: no
  evidence the vendors' formats support it safely; surfaced divergence
  + user choice is honest and cheap.

## Consequences

E2 builds the graph, pack v2, fidelity matrix, executor, door, and
switch-back; E209 renders the graph in the UI; per-pair validation
(AJ-E210/E808) gates every fidelity badge. `juggler-handover` crate
becomes the switching engine's home.
