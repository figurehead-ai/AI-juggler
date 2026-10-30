# Architecture Decision Records

ADRs capture the *why* behind non-obvious decisions: context, decision, alternatives, consequences. They are immutable once accepted (superseded by a new ADR, never edited in place).

| # | Decision | Status |
|---|---|---|
| [0001](./0001-tauri-2-desktop-framework.md) | Desktop framework: Tauri 2 | Accepted |
| [0002](./0002-harness-adapter-architecture.md) | Harness integration via adapter trait + capabilities | Accepted |
| [0003](./0003-automation-policy-compliance-guardrails.md) | Automation policy & compliance guardrails | Accepted |
| [0004](./0004-fleet-transport-grpc-mtls.md) | Fleet transport: gRPC over mTLS + mDNS pairing | Accepted |
| [0005](./0005-mit-license.md) | License: MIT | Accepted |
| [0006 — Smart onboarding: environment scan and config import](0006-smart-onboarding-scan-and-import.md) | Accepted (§2 as the only mode superseded by 0018) |
| [0007 — Instance-centric information architecture](0007-instance-centric-information-architecture.md) | Accepted |
| [0008](./0008-terminology-charter.md) | Terminology charter: Providers/Accounts/Instances; Profile demoted | Accepted |
| [0009](./0009-portable-context-pack-v2.md) | Portable context pack v2; user-owned redaction policy | Accepted |
| [0010](./0010-spawn-ledger-and-process-ownership.md) | Spawn ledger; verified ownership; cleanup tiers (D-08 amendment) | Accepted |
| [0011](./0011-switch-graph-and-lineage.md) | Switch Graph; hybrid door; switch-back resume+delta | Accepted |
| [0012](./0012-two-clock-quota-model.md) | Two-clock quota model; user-entered expiry | Accepted |
| [0013](./0013-fleet-at-launch.md) | Fleet at launch (beta); per-machine permissions | Accepted |
| [0014](./0014-optional-figurehead-account.md) | Optional Figurehead account and cloud services (two paths) | Accepted |
| [0015](./0015-usage-statistics.md) | Opt-in usage statistics: on-device counters, weekly identifier-free reports, a closed schema | Accepted |
| [0016](./0016-backup-archive-format.md) | Backup archive format v1: allowlisted, scrubbed, verified before restore | Accepted |
| [0017](./0017-encrypted-cloud-backup-container.md) | Encrypted Cloud backup container (FHB1): vault key, recovery code, XChaCha20-Poly1305 | Accepted |
| [0018](./0018-use-setups-where-they-are.md) | Use setups where they are (in-place accounts); partial bring-in is the user's call | Accepted |
| [0019](./0019-extension-architecture.md) | Extension architecture: tools as data (`ToolSpec`), features as modules, a tool-literal lint | Accepted |
| [0020](./0020-agent-bridge.md) | The agent bridge: `juggler-agent`, a per-edition socket, a per-epoch key (live spike pending) | Accepted |
| [0021](./0021-hand-built-svg-chart-kit.md) | A hand-built SVG chart kit, no charting library | Accepted |
| [0022](./0022-session-runtime.md) | How AI Juggler runs the sessions it starts: a runtime per tool (D-40) | Accepted |
| [0023](./0023-local-cache.md) | One local cache system: sized by the machine, invalidated by stamp, the user's to clear (D-58) | Accepted |

To add one: copy `0000-template.md`, number sequentially, PR it with the change that implements the decision.
