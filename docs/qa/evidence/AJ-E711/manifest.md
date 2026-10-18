# Evidence manifest — AJ-E711

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E711 (remote Focus window).
2026-10-18, reviewer claude-opus. Builds on AJ-E414 (tiered focus) and
AJ-E709 (permissions, "Ask each time").

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| A remote focus is sent over the link and executed by the owning machine with the SAME tiered focus; nothing is injected remotely | Req.1 | New additive command `FocusInstance{instance_id}`; the node runs `desktop_focus_with` — the AJ-E414 tiers (activate, match window, un-minimise, `AXRaise`) — with its own Accessibility permission. No second, fleet-only focus path exists | PASS | 2026-10-18 | claude-opus |
| Needs Control (Ask each time or stronger) on the target; View only rejected with a named reason | Req.2 | The node's pipeline requires Control; an "Ask each time" machine also waits for its user's Allow (AJ-E709). The console disables Focus with "View-only pairing — change in Fleet settings on <machine>" and refuses to send | PASS | 2026-10-18 | claude-opus |
| The same honest outcome words round-trip | Req.3 | The node returns its own outcome — `Focused window`, `App opened — window not found`, `App not running — Relaunch?`, and `Permission needed on <machine>` when the Accessibility grant is missing — shown as text in the remote inspector and after a palette jump. `fleet_link :: a_remote_focus_runs_on_the_owning_mac_and_reports_its_honest_outcome` (scripted AX runner on the node): `Focused window` accepted; a terminal Instance → `not_desktop` ("runs in a terminal — there's no window to focus") | PASS | 2026-10-18 | claude-opus |
| Audit on both machines; no focus while the target's screen is locked (reported instead) | Req.4 | The node audits every accepted/rejected command; the console now logs every command it sends (`command_sent` + outcome) in its own hash-chained log — both asserted in the same test. The node reads `CGSSessionScreenIsLocked` (via `ioreg`, no FFI) before focusing: locked → `screen_locked` "Studio Mac is locked — nothing was focused."; an unreadable state also refuses (`lock_unknown`) — never a guess | PASS | 2026-10-18 | claude-opus |
| Preview vs Focus distinct; preview never fakes focus | Req.5 | Remote preview doesn't exist (AJ-E415 is local), so the palette offers only `Focus on <machine>` / `inspect` — nothing pretends | N/A | 2026-10-18 | claude-opus |
| The palette lists remote Instances from cached Fleet state, with staleness | perf / UI | Go-to (⌘K) lists this Mac's live Instances first, then paired machines' (desktop first), each "on Mac mini · ● live / ◐ stale / ○ offline", action "Focus on Mac mini" where permitted, else "inspect"; data is the local Fleet cache (no network call to open it) | PASS | 2026-10-18 | claude-opus |
| Visual: four outcomes, mixed palette, view-only — 1280×800, light/dark; a11y | visual | Gallery `47-remote-focus`; axe 0 serious/critical on the palette with a remote row and the outcome. The first axe scan of the palette found two AJ-E414 defects, now fixed: `text-faint` footer/secondary text (fails AA) → muted, and buttons nested inside `role=option` → the option is the target | PASS | 2026-10-18 | claude-opus |
| Round-trip < 1.5 s on LAN | perf budget | Loopback: the command round trip is milliseconds plus the node's AX tiers (< 500 ms budget, AJ-E414); two-Mac timing at AJ-E710 | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |
| Two-node run log + target screen recording | evidence | Loopback two-node harness above; the screen recording on two physical Macs is an AJ-E710 gate item | DEFERRED-TO-AJ-E710 | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test fleet_link`: 8/8; `focus::lock_tests` 1.
- vitest `remote-tabs.test.tsx` 8/8.
- Playwright `47 remote focus` 4, `fleet — remote focus palette + outcome — axe 0` 2, existing Go-to captures green.
