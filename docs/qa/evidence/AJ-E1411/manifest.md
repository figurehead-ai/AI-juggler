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
  - A menu whose lines keep their shape changes only its words, in place,
    so an open menu stays open. A new shape rebuilds the menu.
  - A click opens the window and emits `tray://open`. The UI goes to
    Home's This Mac panel, or to the session the line names. Quit quits.
  - Linux and Windows builds carry no tray (the `tray-icon` feature is
    enabled for macOS only), so they need no app-indicator library.
- This Mac gains `by_tool`: each AI tool's share. Our sessions are grouped
  by Provider, heaviest first, then the Claude app, then `claude` in a
  terminal. These are the menu's per-tool lines.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 number and shape only when it matters | `cargo test -p juggler-app --lib -- tray` (`a_calm_mac_is_one_quiet_icon_with_this_mac_behind_a_click`, `a_heavy_session_puts_a_number_up_and_comes_first`, `the_mac_itself_outranks_a_session_and_a_heavy_alert_counts_once`, `a_tight_mac_says_why_without_a_number`, `the_icon_is_a_template_and_attention_is_a_shape`) | pass | 8 passed | PASS | ee896edb | 2026-10-20 | claude-opus |
| Req.2 the menu is This Mac | same (the calm menu line by line; nothing running; before the first reading; a long name cut); `machine.test.ts` "breaks This Mac down per tool" (the UI's mock matches the backend's grouping) | pass | pass | PASS | ee896edb | 2026-10-20 | claude-opus |
| Req.2 a click opens the right place | `machine.test.ts` "opens the session a click names, else This Mac" (`trayTarget`); `App.tsx` listens for `tray://open` | pass | pass | PASS | ee896edb | 2026-10-20 | claude-opus |
| Req.3 live, in place | Debug build, `HOME` pointed at an empty folder (no user data), `AIJ_TRAY_TRACE=1 ./target/debug/juggler-app`: the menu printed every 5 s, keeping its shape (only the figures changed). Example line: `Getting tight — macOS reports memory pressure \| CPU 62% · Memory 78 GB of 103 GB \| Claude app · 5 windows · CPU 12% · about 14 GB \| Open This Mac \| Quit AI Juggler`, with no number beside the icon (no alert, no heavy session of ours) | as described | as described | PASS | ee896edb | 2026-10-20 | claude-opus |
| Native rendering | The status item itself couldn't be captured here: screen capture and System Events need Screen Recording or Automation access, which this agent doesn't have and must not grant. A maintainer glance at the menu bar is still due | — | not captured | OPEN | — | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
