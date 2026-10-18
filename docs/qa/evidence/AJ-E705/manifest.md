# Evidence manifest — AJ-E705

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E705 (remote inspector).
2026-10-18, reviewer claude-opus. Contract: ADR-0013.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Remote selections fill the same six tabs; fields a peer doesn't send read "Unavailable — <reason>" | Req.1 | The remote selection renders inside the existing `RightInspector` shell (same header, tab order, ⌘1–6, arrow keys) with remote tab bodies (`RemoteTabs.tsx`) — no fleet-only panel. Playwright `a remote Instance fills the same six-tab inspector …` asserts the six tab names and that each panel renders; vitest asserts "Unavailable — not reported" for a missing model and "Unavailable — Mac mini doesn't share usage and quota / conversation history over the link yet" | PASS | 2026-10-18 | claude-opus |
| Every value carries source machine, measured-at, freshness (`Live` < 5 s, `Stale`, `Offline — last seen`) | Req.2 | Each field: "from Mac mini · measured 2 s ago" (times on that Mac's clock say so). Freshness is computed on the console: `live` only while the link is up AND something arrived within 5 s (`remote::tests::freshness_is_never_live_without_a_live_link`); the header line is `aria-live="polite"` with a shape + words (`● Live …`, `◐ Stale …`, `○ Offline — … last seen …`), never colour alone | PASS | 2026-10-18 | claude-opus |
| Remote Actions permission-gated; View only disabled with "View-only pairing — change in Fleet settings"; control still needs an explicit click | Req.3 | Stop is disabled with the visible reason "View-only pairing — change in Fleet settings on Mac mini" (also when offline / not running). With View + control it is one explicit click, sent through the node's pipeline (nonce, per-peer sequence above a wall-clock seed, 30 s expiry, measured clock). The node decides and the UI shows its words. `juggler-app --test fleet_link :: a_console_sees_the_other_macs_instances_live_and_its_commands_go_through_the_node`: View only → `permission_denied` without sending; after re-pair with control, a stop for a record the node doesn't have → `unknown_record`, audited on the node | PASS | 2026-10-18 | claude-opus |
| Render < 200 ms from cached deltas; a slow/offline peer never blocks the local board | Req.4 | The console keeps a per-peer cache fed by a telemetry subscription; the inspector reads `fleet_remote` (local memory, polled every 2 s, async). An offline peer flips the same view to Offline with no spinner (Playwright asserts no "Loading…"). | PASS | 2026-10-18 | claude-opus |
| Quality/Performance shows only D-07 signals, no composite score | Req.5 | Process health (process count), stuck/waiting (from status), errors/retries and context use "Unavailable — not reported" until a peer sends them; "Measurable signals only — no composite score" | PASS | 2026-10-18 | claude-opus |
| Switching machines preserves tab and scroll per selection | Req.6 | The tab is remembered per selection type (Instance / Machine), as for local selections; per-selection scroll position is not kept | PARTIAL | 2026-10-18 | claude-opus |
| Breadcrumb `Fleet / mac-mini / Instance` with ⌘[ back | UI/UX | `remote-breadcrumb` (nav "Breadcrumb", `aria-current`); ⌘[ returns to Fleet (Playwright) | PASS | 2026-10-18 | claude-opus |
| Telemetry carries no path or secret | contract | New `SessionDelta` fields are additive (snapshot blessed): model + source, project folder **name only**, start/last activity, CPU/RSS/process count; the field allowlist test still passes; v1 never relays (a delta whose origin isn't the peer is dropped) | PASS | 2026-10-18 | claude-opus |
| Link off really drops connections, even with a console subscribed | contract (found here) | `LinkNode::stop` now cuts every connection's IO (graceful shutdown alone waited forever on the endless telemetry stream): `transport :: stopping_a_node_never_waits_on_an_open_telemetry_stream` | PASS | 2026-10-18 | claude-opus |
| Visual: Overview / Processes (live, stale, offline, view-only) + machine overview, 1440×900 + 1024×768, light/dark; a11y | visual | Gallery `45-remote` (instance-overview-live / stale / offline, instance-processes, instance-actions-view-only, machine-overview); axe 0 serious/critical both themes on the breadcrumb, freshness line, tabs and disabled Stop | PASS | 2026-10-18 | claude-opus |
| Perf: 60 fps at 5 machines × 20 Instances; delta apply < 50 ms; payload < 200 KB | perf budget | Not instrumented here — measured on gate hardware in AJ-E710 | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |
| Two-machine dogfood (select remote Instance, read Usage/Processes, timing) | evidence | AJ-E710 gate item | DEFERRED-TO-AJ-E710 | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test fleet_link`: 5/5; `--lib remote`: 1.
- `cargo test -p juggler-link --test transport`: 12/12.
- vitest `remote-tabs.test.tsx` 7/7.
- Playwright: `45 remote` 12, remote flow 1, `fleet — remote inspector — axe 0` 2.
