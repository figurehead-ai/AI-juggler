# ADR-0013: Fleet at launch

**Status:** Accepted
**Date:** 2026-10-13
**Deciders:** Vuxtra engineering + maintainer (decision-log D-13)
**Implements:** D-13 (and SPEC §11)

## Context

Pass-2 recommended schema-only fleet (defer build post-1.0). The
maintainer overruled (D-13): Fleet ships with the launch, in beta, so
multi-machine integrates cleanly rather than being retrofitted.

## Decision

1. Fleet is a **Phase-0 architecture constraint**: identity
   (`MachineId`) on every ledger row, Switch Graph node, and usage
   event; ownership/verification/audit per machine; offline and
   conflict behaviour designed in.
2. Launch scope (beta): pairing (6-digit/QR, identity pinning),
   gRPC/mTLS transport (ADR-0004 preserved), remote Inspector (view),
   remote switch and cleanup under per-machine permissions
   (`View`/`Control`/`NoAccess`).
3. **Never**, at any tier: credential sync, login automation, traffic
   proxying, cloud relay. The §1/§8 invariants hold on every node; a
   console never acts on a node's spawn ledger without that node's
   permission.
4. The fleet security review (STRIDE: spoofed mDNS, MITM pre-pinning,
   replayed commands, stolen identity keys, malicious paired peer) is
   a **beta-exit gate** (AJ-E710).

## Alternatives considered

- **Schema-only now, build post-1.0** — the pass-2 recommendation;
  rejected by D-13 as retrofit risk.
- **Full fleet parity at launch (equal to local)** — rejected: remote
  control under permissions is the honest beta scope; full parity
  follows validation.

## Consequences

E7 ships identity/transport/pairing early and remote surfaces later in
beta; local designs must stay fleet-shaped in review; scope and
estimates re-planned around D-13 (recorded in EPICS phases).

## Open questions (beta-exit criteria, AJ-E511)

Recorded per D-13/D-15 without implementing transport — shaping the
data is this task's whole scope (`juggler-link` is SPEC §11):

1. **Identity**: machine labels are user-editable display names over
   the generated id (`This Mac` default) — when does a re-labelled
   machine propagate, and what wins on label collision?
2. **Sync/transport**: gRPC/mTLS per ADR-0004 — do ledger deltas sync
   row-level (composite `session_id+machine_id+boot_id` key) or as
   per-machine snapshots? PID/PGID are unique only per machine+boot;
   any merge without all three re-creates the false-kill bug at fleet
   scale (E511's disjoint-rows test is the standing guard).
3. **Remote ownership**: a console must never signal without the
   owning node's permission (`View`/`Control`/`NoAccess`) — is
   permission re-checked per command or cached per pairing? Remote
   cleanup rides the OWNING node's E507 ladder, never the console's.
4. **Offline**: stale foreign rows must not present as live — the
   inventory marks foreign rows display-only until a freshness
   heartbeat exists; what is the staleness threshold, and do orphaned
   foreign rows ever expire locally?
5. **Security**: the STRIDE review (spoofed mDNS, MITM pre-pinning,
   replayed commands, stolen identity keys, malicious paired peer) is
   the AJ-E710 beta-exit gate; identity pinning UX (6-digit/QR) is
   unvalidated.
