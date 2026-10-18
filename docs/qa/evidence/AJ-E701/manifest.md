# Evidence manifest — AJ-E701

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E701 (Fleet architecture
ADR and threat model). 2026-10-18, author claude-opus; second reviewer:
an independent security-review agent (read-only), two rounds.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Supersedes D-2 schema-only timing, cites D-13 verbatim, keeps ADR-0004 transport | Req.1 | ADR-0013 "Architecture contract (AJ-E701)" opens with D-13 verbatim and keeps gRPC/mTLS/mDNS, no relay; ADR-0014 discovery noted as the one optional addition | PASS | 2026-10-18 | claude-opus + reviewer |
| Defines `MachineId`, node/console roles, identity lifecycle, permission levels | Req.2 | "Identity" (MachineId vs label, Ed25519 key, fingerprint, generate → pin → unpair → regenerate, signed registry with generation) and "Roles and permissions" (NoAccess / View / Control + Ask each time, node-side per-command enforcement, node-only permission raises, commands reference node-existing ids only, node-confirmed pack fetch) | PASS | 2026-10-18 | claude-opus + reviewer |
| STRIDE model covers spoofed mDNS, MITM before pinning, replay, stolen key, malicious peer | Req.3 | 18-row STRIDE table, each row with a mitigation and a named negative test; plus "Accepted risks" (same-user code, LAN metadata, pairing-lockout abuse) | PASS | 2026-10-18 | claude-opus + reviewer |
| Invariants: credentials/keychain/transcripts never cross except the user-clicked pack fetch | Req.4 | "What never crosses the link" + payload allowlist + node-confirmed pack fetch | PASS | 2026-10-18 | claude-opus + reviewer |
| Tailscale / LAN / relay / CRDT compared with a decision table | Req.5 | "Options considered" (6 rows incl. account directory and JSON-RPC) with verdicts | PASS | 2026-10-18 | claude-opus + reviewer |
| Closes the protobuf-vs-JSON-RPC open item | Handover note | Protobuf; SPEC §15 open item 1 struck through; ADR-0004 status updated | PASS | 2026-10-18 | claude-opus |
| Threat table reviewed by a second reviewer; no unresolved `TODO(threat)` | Evidence | Round 1: SIGN-OFF no (RCE via free-form launch, silent account pins, unbound key confirmation, lockout bypass, clock-dependent expiry, registry rollback, plus 12 missing threats and 9 contradictions). All addressed in the contract and in task-text fixes (AJ-E702/E704/E709/E710/E711, E7 out-of-scope, AJ-E1009, AJ-E1204, AJ-E1206, SPEC §11.1, ADR-0014, E1/E9 numbering notes). Round 2: 5 residual items, all fixed. Round 3: **SIGN-OFF: yes**. No `TODO(threat)` rows | PASS | 2026-10-18 | reviewer |
| Gates | quality gates | `cargo test -p juggler-core` green (no core change); ADR follows the template sections; links resolve (evidence gate) | PASS | 2026-10-18 | claude-opus |
