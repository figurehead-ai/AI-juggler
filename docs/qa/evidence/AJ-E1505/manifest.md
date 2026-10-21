# Evidence manifest — AJ-E1505

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1505 (the settings SPEC §5.1
promised: stuck thresholds, terminal font and scrollback). Status:
**DONE-EVIDENCED**. 2026-10-21, reviewer claude-opus.

## What exists now

- **Settings › Sessions** (`SessionsSettingsCard` in
  `ui/src/components/SettingsSections.tsx`), a new section in the list.
  - **Stuck after** (minutes): "No output for this long, with the CPU
    near idle, marks a session Looks stuck. Raise it if your builds run
    quietly."
  - Under the **Terminal** disclosure: **Font size** (points, in 0.5
    steps) and **Scrollback** (lines; "more lines use more memory").
  - Each field saves on Enter or on leaving it (never mid-typing), says
    what it saved, and shows **Reset to …** once it differs from the
    default. A value outside its range is refused in words, and the old
    value stays.
- **Live, no restart.** `HeuristicEngine` now holds its thresholds behind
  a lock (`set_thresholds`), and `session_prefs_set` hands it the new
  window. The next tick judges by it, and the stuck reason shows the
  threshold it used ("threshold 10m"). "Stuck after" sets both the
  no-output and the transcript windows, which SPEC §5.1 calls the same
  window. At launch it is applied over `heuristics.json`. A per-tool
  override there (AJ-E1004) still wins for that tool. Open terminals
  follow the `aij:session-prefs` event: xterm takes both values live and
  refits.
- **Backend** (`crates/juggler-app/src/settings_policy.rs`):
  `session_prefs_get`/`session_prefs_set` with a typed `SessionPrefsInfo`.
  The ranges are 1–120 minutes, 9–24 points and 500–100,000 lines, and
  the values are stored in app settings.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 the three settings, each explained in one line, with a reset | vitest `session-prefs`: the fields, their descriptions (`aria-describedby`) and the closed Terminal disclosure; saves on Enter, not mid-typing; Reset appears off the default and resets; an out-of-range value is refused in words and nothing is stored | PASS |
| Req.2 the heuristics read it live | cargo `heuristics::a_changed_threshold_applies_at_the_next_evaluation` (an 8-minute quiet build: not Stuck at 10 minutes, Stuck again at 5, with the same engine); `tests/session_prefs.rs` (applied to the running engine at once, both windows set, other thresholds kept; survives a restart; reset; ranges, nothing half-saved) | PASS |
| Terminal font and scrollback reach open terminals | vitest: a save fires the event once, and `applyTerminalPrefs` sets both values and refits; walked: a terminal opened after the change reads 16 px | PASS |
| Visual: the Settings section | capture `65-settings-sections/sessions-*` (the section) and `sessions-terminal-*` (Terminal open, a Reset shown); regression `settings-sessions-*` (8 cells) | PASS |
| a11y | a11y "settings sessions": every field labelled and described, the disclosure opened by keyboard, axe 0 critical/serious in both themes | PASS |

## Gates

GATES_PLACEHOLDER

## Walked by hand

The vite dev server with the mock bridge. I opened Settings with ⌘, and
picked Sessions in the list. Defaults showed: 5 minutes; under Terminal,
12.5 points and 5,000 lines. I typed 12 into Stuck after and pressed
Enter, then 16 into Font size and pressed Tab. Both were saved, the
status read "Terminal text is 16 points.", and "Reset to 5" and "Reset to
12.5" appeared. A session's terminal opened afterwards read 16 px.

## Not covered here

- The real Mac app: the setting's effect on a live session's badge rides
  on the 30 s heuristic tick. It's proven against the engine (cargo), not
  watched on a real quiet session for ten minutes.
- Per-tool thresholds in `heuristics.json` stay a file-only expert
  setting. The section doesn't show them.
