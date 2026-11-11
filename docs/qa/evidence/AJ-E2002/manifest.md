# Evidence manifest — AJ-E2002

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2002, the agent
bridge's security design. Status: **DONE-EVIDENCED** (agent self-check,
2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | ADR-0020 holds each rule: a 0700 directory and a 0600 socket, the peer-uid check and a per-run key kept as its SHA-256 ("Decision"); no token passthrough (the key in the environment only, never argv or a file; SECURITY-GUIDELINES' agent-bridge row); messages as data with provenance (E18 addendum); no state change without the user's click (E18/E19 addenda, the guardrail suite); capability-tagged data against confused deputies ("Capability-tagged data", added 2026-11-06). |
| R2 | PASS | The threat-model rows (ADR-0020 "STRIDE", the addenda, and the confused-deputy row in "Capability-tagged data"; SECURITY-GUIDELINES' agent-bridge row) and the test list each row names (`messages_guardrails.rs`, `wire.rs`, the guardrail suite). |
