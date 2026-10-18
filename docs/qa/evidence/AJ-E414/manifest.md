# Evidence manifest — AJ-E414 (Focus Window service)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E414.
Branch: `stage2/e4-focus-window`. Local runs (macOS arm64) 2026-10-14,
reviewer zcode-primary. Rust suites run in CI (verify job); component
tests in the ui job; the real-window record is a local-only run by
design.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 focus tiers | `cargo test -p juggler-app --lib focus::` (12 tests) | activate → enumerate/match identity → clear `AXMinimized` → `AXRaise`; activation alone never reported as window focus | `focus.rs` `FocusService::attempt`: tier (a) `/usr/bin/open -b` (the `LaunchServices` equivalent of `NSRunningApplication.activate` — see mechanism pin below); tier (b) window enumeration + recorded-title match; tier (c) one atomic script `set AXMinimized false → perform AXRaise → set frontmost`; the 10/10 fixture asserts the order (minimized cleared BEFORE raise) and that only `Focused window` counts as window success | PASS | 2026-10-14 | zcode-primary |
| Mechanism pin (recorded deviation) | PR description + `focus.rs` module docs | task named `accessibility-sys`/`objc2-app-kit` | both require `unsafe` FFI; the workspace law `unsafe_code = "forbid"` (Cargo.toml `[workspace.lints.rust]`, QUALITY.md §1) bars them. Tiers are driven through `/usr/bin/osascript` + System Events and `/usr/bin/open -b` — same AX operations (`AXMinimized`, `kAXRaiseAction` semantics), no unsafe, fully scriptable for tests | PASS | 2026-10-14 | zcode-primary |
| Req.2 window identity | `cargo test -p juggler-core --test storage_roundtrip` (21) + `cargo test -p juggler-app --test desktop` | owning PID + bundle id at launch; title/`CGWindow` id where available; re-discovered every attempt; stale never assumed | `InstanceRecord.window` (schema v10 additive JSON blob — round-trip, default-None, malformed-degrades tests green); `desktop_launch` records pid+bundle at insert; first-sight title adopted and persisted (asserted); stale pid re-discovered by bundle id and persisted (asserted); `CGWindow` id honestly `null` (no non-FFI source; never fabricated) | PASS | 2026-10-14 | zcode-primary |
| Req.2 ambiguity honesty | `title_collision_reports_window_not_found_never_guesses`, `stale_title_reports_window_not_found` | collision recorded as `window not found` | both fixtures green: colliding titles / a stale title never raise a wrong window — no `AXRaise` script is even issued (asserted) | PASS | 2026-10-14 | zcode-primary |
| Req.3 permission degradation | `window_read_denied_after_ok_probe_degrades_with_flag` + `missing_permission_degrades_to_activation_with_flag` + UI tests | degrade to app activation; explainer copy-exact; never requested silently | activation runs, no raise attempted (asserted); `FocusOutcomeInfo.permission_missing` drives the one-time dismissible explainer `Juggler can open the app; exact-window focus needs Accessibility permission` with the fixed deep link (`open_accessibility_settings` — the URL is hardcoded in Rust, never caller-supplied); dismissal persists in localStorage (asserted across remounts) | PASS | 2026-10-14 | zcode-primary |
| Req.3 live-caught bug | real-record run (below) | classification must not misreport | the first real run exposed it: process-NAME reads are allowed WITHOUT the grant, so the upfront probe passed while window enumeration was denied (`-1728`) — misclassified as `App not running`. Fixed: every System Events error is classified by its stderr (assistive/-1743/-1719/… → permission-missing); regression fixture added; the probe script was removed entirely (also halving latency) | PASS | 2026-10-14 | zcode-primary |
| Req.4 hotkey + palette | `pnpm --dir ui test` (goto-palette 8 cases) + captures | global, user-configurable; palette lists live desktop AND terminal instances; Enter routes correctly | `tauri-plugin-global-shortcut` registered at startup (stored binding or the SPEC default `⌥Space`; conflicts log and leave ⌘K working); `go_to_hotkey_set` registers the new binding FIRST and restores the previous one on failure; the palette (⌘K / global hotkey) lists live instances desktop-first, keyboard-complete (combobox a11y asserted: `aria-expanded`/`activedescendant`/`role=listbox`), Enter → `focus window` (desktop) / `open panel` (terminal) asserted; rebind capture flow (`⌥Space` → `Alt+Space` accelerator) and inline failure surfacing asserted | PASS | 2026-10-14 | zcode-primary |
| Req.5 outcome honesty | focus suites + UI tests | `Focused window` / `App opened — window not found` / `App not running — Relaunch?` verbatim, as text on the Instance | all three asserted end-to-end (tier engine + command + panel `desktop-focus-outcome` node); a failed `AXRaise` reports window-not-found, never success; the panel shows the outcome plus the targeted window title | PASS | 2026-10-14 | zcode-primary |
| Quality gates | `just verify` | fixture focuses correct window 10/10; missing-permission fixture degrades, no crash; verify green | verify green (fmt, clippy `-D warnings`, workspace tests, docs, evidence, terminology); scripted two-window fixture 10/10 with trap-order asserts; missing-permission fixture degrades cleanly | PASS | 2026-10-14 | zcode-primary |
| Real-window record | `cargo test -p juggler-app --test focus_real -- --ignored --nocapture` → `real-two-window-focus.log` | real two-window app record | two REAL TextEdit windows (sacrificial temp-dir docs, distinct titles, one minimised between odd attempts): this machine lacks the Accessibility grant, so the record shows the DEGRADED tier honestly — 10/10 consistent `App opened — window not found` + `permission_missing` in 118–189 ms per attempt, windows pre-cleaned/closed (count 0 after), no crash. The granted-machine focused-window record needs the maintainer's one-toggle grant (System Settings → Privacy & Security → Accessibility) — re-run the same command; the 10/10 correct-window gate itself is the scripted fixture suite. Screen recording: `screencapture` requires its own Screen Recording grant and was not attempted silently (would produce a misleading wallpaper-only artifact) | PASS (degraded tier; focused tier scripted-gated) | 2026-10-14 | zcode-primary |
| Performance budget | timing in the real record + script-count analysis | focus <500 ms warm; enumeration <100 ms; hotkey-to-list <100 ms | degraded tier measured 118–189 ms (was 447–528 ms before the probe removal — the classification refactor halved it); the granted tier runs the same or fewer scripts (enumeration+raise); enumeration is one System Events round-trip; hotkey-to-list is the in-process `app://go-to` event → palette mount | PASS | 2026-10-14 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | panel before/after outcomes, palette list, explainer; both themes | 100/100 incl. new `_gallery/14-focus/` (4 captures): panel outcome after focus (dark 1280x800), permission explainer (vision-verified: copy-exact text, both buttons, honest outcome, no terminal card), palette dark + palette light 1024x680 (vision-verified: rows with `app`/`term` chips, `focus window`/`open panel` hints, footer with `Alt+Space — change…`) | PASS | 2026-10-14 | zcode-primary |
| Regression | full suites | prior gates intact | `just verify` green; 143/143 component (8 palette + 3 explainer/outcome new); 12/12 tier unit; 9/9 desktop integration (5 new command-level: tier persistence, ended → not-running, terminal refused by name, legacy-identity bundle re-discovery, permission flag); `pnpm --dir ui build` green | PASS | 2026-10-14 | zcode-primary |

## Notes for the next agent

- E415 (desktop live preview) is the last E4 task; then E5/E2/E3 per
  the catalog order.
- The focused-window REAL record still needs one maintainer action:
  grant this app/terminal Accessibility (System Settings → Privacy &
  Security → Accessibility), then
  `cargo test -p juggler-app --test focus_real -- --ignored --nocapture`
  — the same test verifies 10/10 correct-window focus with AX frontmost
  checks once the grant exists.
- Focus classification lives ONLY in `is_permission_denied` (stderr
  strings macOS emits: `assistive`, `not authorized`, `accessibility`,
  `-1743`, `-1719`, and now the live-observed `-1728` prefix-free
  `assistive access` text). When a new denial string appears on a
  future macOS, add it there with a fixture — never broaden to
  "any error = permission" (that would misreport dead apps).
- The window identity is a v10 JSON blob on the Instance row; titles
  are adopted at first sight and must NEVER be guessed beyond
  `window[0]` (collision/stale → `window not found` — the handover
  trap).
- `CGWindow` ids need CoreGraphics FFI (barred by the unsafe law); if
  that ever changes, fill `cgwindow_id` and use it to disambiguate
  colliding titles instead of reporting not-found.
- The full command palette (actions/search/recents, deep-review §4.17)
  remains unowned; the Go-to slice here is built to be absorbed by it
  (combobox roles, keyboard contract, and the hotkey kv are reusable).
