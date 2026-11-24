# Evidence manifest — AJ-E1701

Task: `docs/tasks/E17-agent-pools.md` AJ-E1701 — the spike, ADR-0020
and the policy rulings (P0, Effort S).
Status: **Req.1 complete; Req.2's headless lab complete — the
interactive §17 pass remains the maintainer's** (same class as
AJ-E804/E805's waiting remainders). 2026-11-23, zcode-primary.

| Claim | Requirement | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| ADR-0020 "Agent bridge" + D-28–D-33 as ruled | Req.1 | [ADR-0020](../../../docs/adr/0020-agent-bridge.md) written (landed with AJ-E1704); D-28–D-33 ruled 2026-10-24 and recorded in [decision-log.md](../../../docs/review/decision-log.md) (the plan doc's recommendations adopted) | PASS | 2026-10-25 | maintainer (rulings); claude-opus (ADR) |
| `--mcp-config` loads in Claude Code without an approval prompt | Req.2 (a) | The 2026-10-25 lab ([lab-2026-10-25.md](../AJ-E1704/lab-2026-10-25.md), run on the maintainer's lab accounts under D-107 with the maintainer's permission): init event listed `juggler` connected with all three tools; `permission_denials: []` — **PASS headless**; the interactive TUI's own approval screen (and `/mcp`, and trust-this-folder) not exercised — the maintainer's §17 pass | PASS (headless); interactive = maintainer's | 2026-10-25 | claude-opus (lab); maintainer pending |
| Codex's `-c mcp_servers.*` with `env_vars` passes the token | Req.2 (b) | Same lab: `whoami` succeeded through `env_vars` (a keyless server answers "no key"); the per-tool `approval_mode="approve"` let all three calls through with NO approval event, and the negative control (`prompt` mode) correctly refused | PASS (headless) | 2026-10-25 | claude-opus |
| A hook from a `--settings` file adds a turn note | Req.2 (c) | The lab exercised the `--settings` overlay path on every Claude launch (the bridge rides it); the turn-note hook specifically was not separately captured — folded into the maintainer's §17 pass | PARTIAL — maintainer's §17 | 2026-10-25 | — |
| Captures in `docs/research/evidence/` | Req.2 | The capture lives as [docs/qa/evidence/AJ-E1704/lab-2026-10-25.md](../AJ-E1704/lab-2026-10-25.md) (redacted: no names, emails, ids or paths; usage + before/after folder diffs included). `docs/research/evidence/` currently holds only `billing/` — the redacted lab record's canonical home is the evidence tree; noted here rather than duplicated | PASS (relocated) | 2026-10-25 | claude-opus |
| The spike's findings absorbed | (bonus) | The lab's one bug — headless Claude refusing `stream-json` without `--verbose` — is FIXED in `crates/juggler-adapters/src/claude_code.rs` (the Headless arm, comment citing the lab); protocol versions recorded (Claude 2026-07-28 stateless via `server/discover`; Codex 2025-06-18 via `initialize`) | PASS | 2026-10-25 | — |

## What remains (the maintainer's, not an agent's)

MAINTAINER-STEPS §17's checkboxes — the interactive walkthrough in AI
Juggler Dev: the "Yes, let them" answer in the window, a session on a
spare account asking "Call the juggler whoami tool", the TUI's
approval screen (or its absence) seen with eyes, `/mcp` listing
`juggler` with the user's own servers still present, and the
turn-note hook. The headless lab proves the mechanics; only the
interactive surface is unobserved. Until then the three "unverified"
lines in AJ-E1704's manifest stand as written.

## Test counts

Docs-only change; no gates beyond the repo's evidence checks apply.
The lab's own accounting: 4 Claude runs + 3 Codex runs, ≤ $0.20
list-price equivalent total (the lab's usage table).
