# Evidence manifest — AJ-E709

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E709 (Fleet indicators,
permissions and alert routing). 2026-10-18, reviewer claude-opus.
Contract: ADR-0013.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Every Instance row names its machine, Provider, Account, model, status (dot + text) and quota (`Unknown` where not known) | Req.1 | With a paired machine reporting, the board shows a **This Mac** section and one section per machine; remote rows carry a machine chip in text, Provider, model (or "model unknown"), status words and "quota unknown" (D-12 — never inferred). Each row's accessible name says "Work Max on Mac mini — waiting user" (Playwright). Local rows keep their existing indicator row (quota bar or "Quota: no data") | PASS | 2026-10-18 | claude-opus |
| Machines filter the board; "All machines" default, persisted across restarts | Req.2 | Board toolbar **Machine** filter (All / This Mac / each paired machine), stored in `aij.machine.filter`; Playwright reloads and finds the choice kept. Overview and Usage stay this-Mac-only until usage data travels the link (AJ-E708) | PARTIAL | 2026-10-18 | claude-opus |
| Fleet alerts route to every paired console, source machine named, deep-linking to the inspector | Req.3 | The node publishes each alert its own alert engine emits (same words) as an additive `AlertDelta`; consoles keep the 10 most recent per machine, de-duplicated (`fleet_link :: alerts_on_one_mac_reach_its_paired_console_named_by_machine`), plus a console-side "<machine> has been offline for over 10 minutes". The alert region shows "Mac mini · …" with **Open** → that machine's inspector (Playwright). Alerts carry an Account, not an Instance id, so the deep link lands on the machine overview | PASS | 2026-10-18 | claude-opus |
| Permissions enforced node-side; forged commands from a View-only console rejected + audited | Req.4 | Node pipeline (AJ-E703): `transport :: view_only_peers_get_telemetry_but_no_commands` sends forged commands straight over gRPC and gets `permission_denied`, audited; console-side the button is disabled AND the call refuses — the node is the authority | PASS | 2026-10-18 | claude-opus |
| Changing a peer's permission needs confirmation on the node machine itself | Req.5 | **Access…** on each paired machine opens a sheet on THIS Mac (Cancel focused) — View only / View + control, ask each time / View + control; no link call exists that can change a permission. The console learns the change by re-asking every ~30 s (and immediately after a re-pair). `fleet_link :: ask_each_time_holds_a_command_until_this_macs_user_decides` sets it on A and B sees `control` | PASS | 2026-10-18 | claude-opus |
| "Ask each time" = a local prompt before anything runs | Req.5 / AJ-E701 | A control command from an "ask each time" machine waits (≤ 20 s, inside the 30 s command expiry) for this Mac's user: an alertdialog names the machine and the action, focuses **Deny**, and silence is a Deny. Deny → `not_confirmed`, nothing runs; Allow → the node's own checks continue (same test) | PASS | 2026-10-18 | claude-opus |
| Tray aggregate | Req.6 | AI Juggler has no menu-bar tray in this build; nothing to aggregate into. Task-text note: re-scope when a tray exists | N/A | 2026-10-18 | claude-opus |
| `⌘1..⌘5` filters by machine | UI/UX | Not bound: `⌘1–⌘6` already switch inspector tabs (D-06). Task-text note | N/A (conflict) | 2026-10-18 | claude-opus |
| Colour never the only channel | quality gate | Machine names, freshness (`● live` / `◐ stale` / `○ offline`) and status are text everywhere; axe 0 serious/critical on the board with machine rows, the Access sheet and the prompt, both themes. While here: the alert region and renewal line used a hard-coded `amber-500` that fails AA on light — now the warning token | PASS | 2026-10-18 | claude-opus |
| Visual: board All / single machine, alert, Access sheet, prompt — 1440×900 + 1024×768, light/dark | visual | Gallery `46-fleet-board` | PASS | 2026-10-18 | claude-opus |
| Perf: 60 fps at 5 × 20; alert < 2 s; filter < 100 ms | perf budget | Alerts arrive with the next telemetry frame and paint within the 2 s poll; frame-rate at scale measured on gate hardware in AJ-E710 | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |
| Screenshot-identification test (10 rows < 30 s) + 3-machine alert walk | evidence | Human study at AJ-E710 | DEFERRED-TO-AJ-E710 | 2026-10-18 | claude-opus |

## Also fixed

- The top bar showed a hard-coded "Claude Code" chip on every screen;
  it now shows the selected Instance's Provider, or nothing.
- At 1024 px the top bar clipped **New**; nav spacing tightens below xl.
- The at-a-glance card said "this-mac" and named a task id; it now says
  "This Mac".

## Test counts

- `cargo test -p juggler-app --test fleet_link`: 7/7.
- vitest `fleet-prompt-dialog.test.tsx` 2/2 (+ the existing suites).
- Playwright `46 fleet board` 8, the persistence/alert/access flow 1,
  `fleet — board machines, access, prompt — axe 0` 2.
