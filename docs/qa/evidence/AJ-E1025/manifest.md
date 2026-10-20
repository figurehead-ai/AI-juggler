# Evidence manifest — AJ-E1025

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1025 (plain words
in the inspector views; no task codes on screen, D-25). Status:
**DONE-EVIDENCED**. 2026-10-20, reviewer claude-opus.

## What exists now

- **Honest empties in plain words.** Each of the six tabs (D-06) says what's
  missing ("No usage measured for this yet.", "No history for this yet.",
  "No quality measurements yet. AI Juggler never makes up a score.")
  instead of naming the task that would fill it.
- **Unbuilt actions aren't shown** (AJ-E407 Req.4, amended in
  `docs/tasks/E4-indicators-and-inspector.md` and in D-25's amendment in the
  decision log). Gone until they work: Export pack, Fork, Archive, Delete,
  Queue resume at reset, Check for an update, Open tool settings, New launch
  settings…, Rename, Delete account…, Relocate…, Hand over project, Open in
  editor, Pin, Archive project. Actions that exist but don't apply now
  (Resume while running) stay disabled with the reason beside them.
- **No codes or raw ids.** Removed: "lands with AJ-E606+", "Switch Graph =
  E2", per-turn and transcript notes, "audit-logged (D-08)", the conflicts
  step's "(AJ-E207)", the process row's "(AJ-E507)", "No data — arrives with
  statusline", "version: No data" chips. Tools are named ("Claude Code", in
  chips, "not reported by …" lines, the switch graph's screen-reader text,
  the account's "tool:"), states use the rail's words ("Hit its limit"), the
  project says "sessions: 2" (it said "instances"). The tool view's quota
  card skips the empty bar for a signed-out account ("signed out" already
  says it) and is titled "Quota, lowest first".
- **The tool view has a way in.** The tool chip in a session's or an
  account's header is a button, "About Claude Code", that opens the tool's
  view: what's installed ("Version 2.1.296 · tested with 2.1.x", "Not
  installed on this Mac", "Couldn't check what's installed"), its accounts
  and their quota. The header names the tool (it said "Home"), and an
  account's view is titled with the account's name (also "Home" before).
- **A bug found on the walk.** An account's History said "now ago" for every
  session (a countdown to a start time already past); it now says "12h ago",
  and the account's own history names each session by its place.
- **The lint.** `scripts/terminology_lint.sh` fails on a task code
  (`AJ-E…`, `E3+`, `(E2)`) or decision code (`D-08`) in JSX text, in JSX text
  that wraps onto its own line, in a visible attribute (`aria-label`,
  `title`, `reason`, `hint`, …) or in a user-facing string; comments may cite
  them. It also catches "instances" as a line-start label. The selftest has
  one bad line per form (8, plus #75's four provider lines) and good lines for comments and code.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 no codes, the lint | `terminology_lint.sh` OK on the tree, selftest 12/12 bad lines flagged (with #75's four provider lines), 0 good lines; on #75's RightInspector, SwitchFlow and ProcessRow it flags 44 lines | PASS |
| Req.2 empties, actions | vitest `inspector-shell` (empties in plain words), `inspector-instance` (real actions present, unbuilt absent; no code on any tab of running/finished/limit sessions), `inspector-session`, `inspector-aggregate` (no code, raw id, "instance", "No data" or "now ago" on any tab of the tool, account and both projects) | PASS |
| Req.3 names and words | vitest: "names the tool, not its id, and the state in the rail's words"; `quality-tab`, `running-model` ("not reported by Claude Code"); `tier-toggles` | PASS |
| Req.4 install words, entry | vitest `installWords` (all four outcomes); capture `72-tool-view` at 1280×800 and 1024×680, light and dark: the session's "About Claude Code" opens the tool view, header "Claude Code", no self-link, "Version …", no code on any tab, axe 0 on every capture | PASS |
| Full suites | vitest 469/469; Playwright 827 (823 first run; the 4 capture assertions that still expected the old text were updated and pass); a11y gate 835 states, 0 critical/serious/moderate/minor; visual gate re-rendered 8 `inspector-instance-*` baselines (the header's tool chip is now a button) and passes | PASS |

## Walked by hand (vite preview, mock bridge)

1280×800 dark, 1024×680 light, 860×680 dark (rail folded):

- A running session: all six tabs read for codes, raw ids and placeholders
  (found and fixed: `claude-code` in Details and Quality, "No data — arrives
  with statusline", the indexing note, `status running`). Actions shows
  Focus terminal, Stop, Resume (disabled, "still running"), Hand over….
- The header chip: hand cursor, Tab focus ring, click and Enter both open
  the tool view; the chip isn't shown on the tool's own view. From there,
  Home and a rail row go back as before.
- The tool view: six tabs; Overview "2 accounts · 1 running", "Version
  2.1.296", quota with Personal "signed out" and no empty bar.
- An account (Accounts › the tool glyph): header "Work Max" with the tool
  chip; History said "now ago" (fixed); "Juggler does not read billing"
  now says "AI Juggler".
- A project (rail header): "instances: 2" (fixed); the switch graph's
  screen-reader text said "provider claude-code" (fixed).
- No console errors; no sideways scroll at 860 px.

## Not covered here

- Backend strings that may reach the screen with codes in them (a process
  verdict's "(D-13)", the recovery pass's "AJ-E510 §11.2", a fidelity reason
  naming "AJ-E808"): changing them needs a Rust build; listed as a
  follow-up in the PR.
- Raw tool ids outside the inspector (the go-to palette, the morning
  summary, the remote tabs, Fleet, a rail row's details): same fix, other
  screens; follow-up.
- `primitives.tsx` ("Quota: no data" on an empty quota bar, the "MEAS" tag)
  and `FirstRun.tsx` wait for #76.
