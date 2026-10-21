# Evidence manifest — AJ-E1411

Task: `docs/tasks/E14-machine-health.md` AJ-E1411 (This Mac in the menu
bar). Status: **DONE-EVIDENCED**.

## What exists now

- `crates/juggler-app/src/tray.rs` holds two pure functions:
  - `tray_model(&ThisMacInfo, name_of)` builds the menu-bar item as a
    value: the title (a number only while something needs a glance), the
    attention shape, the tooltip and the menu's lines.
  - `icon_rgba(attention)` draws the icon: three balls in an arc, 36 px,
    black with alpha (a template image). The attention shape adds a dot
    at the top right.
- The macOS glue installs the status item at startup and follows This
  Mac on the 5 s cadence, in `machine_health::emit_this_mac`.
  - The menu is never replaced. Lines that keep their place change their
    words, and only the run that changed is removed and inserted, so an
    open menu stays open.
  - A click opens the window and emits `tray://open`. The UI goes to
    Home's This Mac panel, or to the session the line names. Quit quits.
  - Linux and Windows builds carry no tray (the `tray-icon` feature is
    enabled for macOS only), so they need no app-indicator library.
- This Mac gains `by_tool`: each AI tool's share. Our sessions are grouped
  by Provider, heaviest first, then the Claude app, then `claude` in a
  terminal. These are the menu's per-tool lines.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 number and shape only when it matters | `cargo test -p juggler-app --lib -- tray` (`a_calm_mac_is_one_quiet_icon_with_this_mac_behind_a_click`, `a_heavy_session_puts_a_number_up_and_comes_first`, `the_mac_itself_outranks_a_session_and_a_heavy_alert_counts_once`, `a_tight_mac_says_why_without_a_number`, `the_icon_is_a_template_and_attention_is_a_shape`) | pass | 11 passed | PASS | ee896edb | 2026-10-20 | claude-opus |
| Req.2 the menu is This Mac | same (the calm menu line by line; nothing running; before the first reading; a long name cut); `machine.test.ts` "breaks This Mac down per tool" (the UI's mock matches the backend's grouping) | pass | pass | PASS | ee896edb | 2026-10-20 | claude-opus |
| Req.2 a click opens the right place | `machine.test.ts` "opens the session a click names, else This Mac" (`trayTarget`); `App.tsx` listens for `tray://open` | pass | pass | PASS | ee896edb | 2026-10-20 | claude-opus |
| Req.3 live, in place | Debug build, `HOME` pointed at an empty folder (no user data), `AIJ_TRAY_TRACE=1 ./target/debug/juggler-app`: the menu printed every 5 s, keeping its shape (only the figures changed). Example line: `Getting tight — macOS reports memory pressure \| CPU 62% · Memory 78 GB of 103 GB \| Claude app · 5 windows · CPU 12% · about 14 GB \| Open This Mac \| Quit AI Juggler`, with no number beside the icon (no alert, no heavy session of ours) | as described | as described | PASS | ee896edb | 2026-10-20 | claude-opus |
| Native rendering | The status item itself couldn't be captured here: screen capture and System Events need Screen Recording or Automation access, which this agent doesn't have and must not grant. A maintainer glance at the menu bar is still due | — | not captured | OPEN | — | 2026-10-20 | claude-opus |

## Review fixes (2026-10-20)

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| No deadlock (Req.4) | `update` takes the icon and menu handles under the lock, releases it, then makes the native calls, and stores the result under the lock again. `on_click` takes no lock (`action_of` reads the clicked line's own id). The settings command is async, so it never runs on the main thread the native calls wait for | by construction | as described | PASS | 12de06db | 2026-10-20 | claude-opus |
| Clicks by action (Req.4) | `cargo test -p juggler-app --lib -- tray` (`a_click_resolves_by_its_own_action_even_after_the_menu_changed`: the old heavy line's position holds Quit-side lines after the menu calms; its id still means that session; unknown ids do nothing) | pass | pass | PASS | 12de06db | 2026-10-20 | claude-opus |
| In place, never replaced (Req.3) | `the_menu_changes_in_place_never_replaced` (figures only, so only words change; a heavy section appears and goes; to and from "not measured"; applying the plan to the old lines gives the new menu) | pass | pass | PASS | 12de06db | 2026-10-20 | claude-opus |
| Staleness (Req.3) | `emit_this_mac` counts failed readings; at two in a row the menu bar shows `unmeasured_model()` ("Not measured right now", no number) | as described | as described | PASS | 12de06db | 2026-10-20 | claude-opus |
| Setting (Req.5) | `settings-screen.test.tsx` "shows This Mac in the menu bar by default, and turns it off"; `menu_bar_get` returns `None` off macOS (the row hides); `install` leaves the item hidden when the setting says off | pass | pass | PASS | 12de06db | 2026-10-20 | claude-opus |
| Screen reader (Req.6) | `voiceover_hears_what_the_number_counts_and_words_for_unknowns` | pass | pass | PASS | 12de06db | 2026-10-20 | claude-opus |
| Live again | Debug build, `HOME` pointed at an empty folder, `AIJ_TRAY_TRACE=1`: 6 updates in 35 s. "Measuring this Mac…" became the full menu by inserting lines in place, then only figures changed | as described | as described | PASS | 12de06db | 2026-10-20 | claude-opus |

## The 0.0.11 hand walk (M1–M3, Q9–Q10; `docs/qa/evidence/walks/2026-10-21-0.0.11.md`)

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| M1 one CPU measure | A tool's line uses Home's measure: its trees summed, where 100% is one core busy (`ToolGroupInfo.cpu_one_core_pct`). The Mac's line says "of all CPU cores", as Home does. Test: `a_calm_mac_is_one_quiet_icon_with_this_mac_behind_a_click` ("about 1.6 GB · CPU 14%") | pass | pass | PASS | @@SHA@@ | 2026-10-21 | claude-opus |
| M2 one figure for the Claude app | An alert names a tool AI Juggler didn't start by all its windows (the same group total as its line). Tests: `machine_alerts::the_claude_app_is_named_by_all_its_windows_never_one` (six windows, the heaviest 4.9 GB: "about 12 GB"), `tray::the_menu_and_home_read_one_fixture` | pass | pass | PASS | @@SHA@@ | 2026-10-21 | claude-opus |
| M3 Home's words | "CPU —"; memory before CPU; "Memory: X of Y in use"; nothing running said once. Tests: `nothing_running_says_so`, `voiceover_hears_what_the_number_counts_and_words_for_unknowns` | pass | pass | PASS | @@SHA@@ | 2026-10-21 | claude-opus |
| Pinned to Home | One fixture, both read models: `tray::the_menu_and_home_read_one_fixture` writes `ui/src/lib/__fixtures__/menu-bar.json` and fails if it's stale. `this-mac.test.tsx` "the menu bar and Home read one fixture" renders Home's This Mac from it and checks three things: a one-tree tool's menu line ends with Home's own row figures; the alert's subject is Home's `alertSubject`; the Mac's memory words are Home's meter words | pass | 3 passed | PASS | @@SHA@@ | 2026-10-21 | claude-opus |
| Q9 a mark, not a ball | The attention dot is r 3 at the top-right corner (32, 4), outside the arc (balls are r 4.5). Test: `the_icon_is_a_template_and_attention_is_a_shape` (the dot adds under half a ball's ink) | pass | pass | PASS | @@SHA@@ | 2026-10-21 | claude-opus |
| Q10 calm at tight | At "tight" the first line is Home's "Getting tight"; "macOS reports …" only comes with the alert (number and dot). Test: `a_tight_mac_stays_calm_in_shape_and_words` | pass | pass | PASS | @@SHA@@ | 2026-10-21 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
