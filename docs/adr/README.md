# Architecture Decision Records

ADRs capture the *why* behind non-obvious decisions: context, decision, alternatives, consequences. They are immutable once accepted (superseded by a new ADR, never edited in place).

| # | Decision | Status |
|---|---|---|
| [0001](./0001-tauri-2-desktop-framework.md) | Desktop framework: Tauri 2 | Accepted |
| [0002](./0002-harness-adapter-architecture.md) | Harness integration via adapter trait + capabilities | Accepted |
| [0003](./0003-automation-policy-compliance-guardrails.md) | Automation policy & compliance guardrails | Accepted |
| [0004](./0004-fleet-transport-grpc-mtls.md) | Fleet transport: gRPC over mTLS + mDNS pairing | Accepted |
| [0005](./0005-mit-license.md) | License: MIT | Accepted |
| [0006 — Smart onboarding: environment scan and config import](0006-smart-onboarding-scan-and-import.md) | Accepted |
| [0007 — Instance-centric information architecture](0007-instance-centric-information-architecture.md) | Accepted |
| [0008](./0008-terminology-charter.md) | Terminology charter: Providers/Accounts/Instances; Profile demoted | Accepted |
| [0009](./0009-portable-context-pack-v2.md) | Portable context pack v2; user-owned redaction policy | Accepted |
| [0010](./0010-spawn-ledger-and-process-ownership.md) | Spawn ledger; verified ownership; cleanup tiers (D-08 amendment) | Accepted |
| [0011](./0011-switch-graph-and-lineage.md) | Switch Graph; hybrid door; switch-back resume+delta | Accepted |
| [0012](./0012-two-clock-quota-model.md) | Two-clock quota model; user-entered expiry | Accepted |
| [0013](./0013-fleet-at-launch.md) | Fleet at launch (beta); per-machine permissions | Accepted |

To add one: copy `0000-template.md`, number sequentially, PR it with the change that implements the decision.
