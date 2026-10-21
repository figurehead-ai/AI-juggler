# Evidence manifest — AJ-E1503

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1503 (one feedback system: toasts
with undo, a quit warning, loading placeholders). Status:
**DONE-EVIDENCED**. 2026-10-21, reviewer claude-opus.

## What exists now

- **One toast** (`ui/src/components/Toast.tsx`). Every result is said in
  words at the bottom of the window, with **Undo** and a countdown where
  the action can be undone. ⌘Z outside a text field or the terminal undoes
  the newest. The store owns the timing, so a held action is sent even if
  the toast never renders. `ToastHost` is mounted once, by App. The switch
  undo window (AJ-E206, the old `SwitchUndoToast`) is now this toast,
  with its 10 s window, focus on Undo when the dialog closes, and failure
  words kept. Its tests are kept and pass against the new component.
- **Stop is held 5 s** (`ui/src/lib/stop.ts`). This covers the inspector's
  Stop button, the Actions tab's Stop row (it only refreshed before, and
  never stopped anything) and ⌘. (which used to read a stale selection:
  its listener was set up once). The toast reads "Stopping api · Work Max.
  Undo keeps it running." Undo in time means the stop is never sent, ✕
  sends it now, and quitting sends whatever is held. Holding the same
  session twice sends one stop. "Stop all" asks first and has no undo,
  per the maintainer's plan: the existing "Stop all orphans…" confirm, and
  the quit sheet's "Stop them first".
- **Undo on Mark reviewed** (a new `instance_unmark_reviewed` records an
  `unreviewed` step, so both steps stay in the history). Also on **snooze,
  mute and unsnooze** (the row's earlier state comes back, applied to the
  state as it is at that moment) and on **archive and unarchive** in
  History (the other way round). Home's and History's one-off status lines
  became toasts; errors are warning toasts.
- **The quit warning** (`crates/juggler-app/src/quit.rs`,
  `ui/src/components/QuitSheet.tsx`). The app menu's Quit item (⌘Q) is our
  own, and the window's close button asks too. With nothing live, the app
  quits at once. Otherwise: "3 sessions are running. They end when AI
  Juggler quits." with Cancel, Stop them first and Quit anyway. A
  terminal session runs on a terminal this app holds, and closing it hangs
  the session up. This was checked on this Mac: a `pty.fork` child died of
  SIGHUP when the master closed. So the sheet says terminal sessions
  **end**, and that desktop or re-adopted sessions keep running. Stop them
  first stops each one cleanly, in parallel, then quits.
- **Loading placeholders** (`ui/src/components/Placeholder.tsx`). The
  rail, Home and Accounts show rows shaped like their own while the first
  load answers. They used to say "No sessions yet" and "Bring in your
  accounts to start." for a moment. The placeholders are announced once
  ("Loading sessions"), and the pulse stops under reduced motion.
- **Screen readers.** One polite live region is always in the page and
  says the result once, never the countdown. Toasts never take focus. The
  shared `Modal` became a real dialog: named, focus moves in, Tab stays
  inside, Esc closes, focus returns.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 one toast; SwitchUndoToast folded in | vitest `switch-flow` (the AJ-E206 undo tests, unchanged assertions, now on `ToastHost`); keyboard flow "switch, 1-click" (focus lands on Undo); a11y "switch flow — 1-click popover + undo toast" | PASS |
| Req.2 held stop: undo-before-send, send-after-window | vitest `feedback.test.tsx`: no kill at 4 999 ms, one kill at 5 000 ms; Undo means no kill after 60 s; the same id twice sends one; ✕ and `flushHeld` send; ⌘Z (`undoLatest`) | PASS |
| Req.2 undo on reviewed, snooze | cargo `triage_lifecycle::undoing_reviewed_puts_it_back_and_keeps_both_steps`; vitest "snoozing says so, and Undo brings the row's earlier state back" | PASS |
| Req.3 quit warning with 0, 1 and n | cargo `quit::tests` (0 quits at once; 1 asks; 4 = 2 end + 2 keep running); vitest words for 1, n, all-keep and mixed; the sheet shows only on request and quits only from a click; Cancel and Esc stop nothing | PASS |
| Req.4 placeholders | vitest: the rail and Home show placeholders while loading and their empty states after; captures `75-feedback/loading-home-*`, `loading-accounts-*`; regression `loading-home-*`, `loading-accounts-*` | PASS |
| Req.5 announced, never steals focus; a11y gate | vitest: the polite region, the countdown kept out of it, focus kept elsewhere; a11y "feedback — toast, quit sheet, placeholders" (axe 0 critical/serious in both themes, Stop keeps focus, the sheet traps Tab) | PASS |
| Performance: a toast renders < 16 ms; the hold adds exactly the window | the hold is one `setTimeout` of `STOP_HOLD_SECONDS * 1000` (vitest at 4 999 and 5 000 ms); the toast is one small node, rendered from a store subscription | PASS |
| Full suites | see "Gates" | PASS |

## Gates

All on 2026-10-21. "Merged head" means the branch with main merged in
(#96, #97, #100); "pre-merge" means before that.

| Gate | Result | Verdict |
|---|---|---|
| `just verify` (fmt, clippy both feature sets, cargo test workspace, docs, evidence, terminology, task counts, stats, polling audit, release self-tests) | exit 0 (pre-merge; the merge touched no crate); evidence, terminology and task counts re-run on the merged head: OK | PASS |
| `cargo deny check` | advisories, bans, licenses, sources ok | PASS |
| vitest | 559/559 on the merged head (feedback 16, switch-flow 7) | PASS |
| Playwright, full suite | 913 passed pre-merge; 915 passed on the merged head | PASS |
| `scripts/a11y_gate.sh` | 686 tests; 860 gallery states with 0 critical, serious, moderate or minor | PASS |
| `scripts/visual_gate.sh --update` (pinned container) | 32 new baselines (toast-stop, quit-sheet, loading-home, loading-accounts × 8 cells) and 8 `add-account-*` re-rendered (stale on main since #95; not this change) | PASS |
| `scripts/visual_gate.sh` (compare, merged head) | 156 passed; 5 load timeouts with no pixel diff (load average 41 from other sessions' suites), all passed on re-run, the overview cells with `--timeout=120000` | PASS |
| `scripts/evidence_index.sh` | regenerated | PASS |

## Walked by hand

- **vite dev server, mock bridge, 1280×800 dark.** I selected the running
  session and pressed ⌘. (a synthetic key: the browser pane keeps a real
  ⌘. for itself). The toast read "Stopping ai-juggler · Work Max. Undo
  keeps it running. Undo (5s)", the announcer said it once, and the stop
  was sent after 5 s. Then Stop, and Undo at "Undo (4s)": "ai-juggler ·
  Work Max keeps running.", and no stop after 6 s. Mark reviewed on Home:
  focus stayed on the next row, and the toast took nothing. A second Mark
  reviewed, then a real ⌘Z: the newest was undone ("payments-api ·
  Personal is back on the list.") and the row came back. Rail row menu ›
  Snooze 15m: "Snoozed ai-juggler · Work Max for 15 min." with Undo.
- **The Dev build** (`cargo tauri build --debug --no-bundle --config
  crates/juggler-app/tauri.dev.conf.json`) on a throwaway HOME with a
  stand-in `claude` on PATH. It starts with the new app menu (built in
  setup; an error there would stop the launch), makes its data folder
  under the throwaway HOME, logs nothing, and stops on SIGTERM. This Mac
  has no Accessibility grant for agent sessions, so ⌘Q and the close
  button weren't pressed in the real window. The decision they run is the
  tested `quit::decide`, and the sheet is the tested `QuitSheet`.

## Not covered here

- The Dock's Quit, logging out and an AppleScript `quit` reach AI Juggler
  as `terminate:`, past the menu, and tao offers no
  `applicationShouldTerminate` hook. They still quit without asking.
  Follow-up.
- Forget and remove (AJ-E1501/E1502, session e15-manage) will use
  `showToast` with Undo. The API note is in the shared notes
  (`feedback-api.md`).
- Keeping terminal sessions alive across a quit, as SPEC's "keep running"
  hopes, would need a terminal host outside the app. That's a separate
  decision for the maintainer.
