# Evidence manifest — AJ-E2406 Privacy lines, guide, and the processes lab

Task: `docs/tasks/E24-processes.md` AJ-E2406. Re-checked against the code,
the docs and their tests on `release/0.4.1`, 2026-11-07.

The Privacy lines `process-sightings` and `observed-stop` (and
`this-mac-other-apps`, extended to Processes' "Include other apps") are in
`ui/src/lib/trust-claims.json` and `docs/guide/trust.md`; the guide is
`docs/guide/processes.md` with `docs/guide/this-mac.md` pointing at it; the
checklist is `docs/qa/processes-lab.md`, run once
([lab-1-2026-10-26.md](lab-1-2026-10-26.md)).

Cargo tests and the docs lints are cited as written and run in CI; the
vitest file was run here: `cd ui && pnpm -s exec vitest run
src/screens/privacy-page.test.tsx` → 1 file, 13 tests passed.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 Privacy lines (sightings; the user's stop of other apps' leftovers), each tied to tests that exist | `cargo test -p juggler-app --test trust every_trust_claim_names_tests_that_exist`, `a_claim_needs_a_running_test`; the lines' tests: `cargo test -p juggler-core --test sightings_storage a_sighting_never_holds_the_environment_or_the_command_line`, `a_sighting_is_forgotten_seven_days_after_it_was_last_seen`; `cargo test -p juggler-app --lib features::processes::sightings::tests::the_hash_is_keyed_and_never_the_line`; `cargo test -p juggler-app --test observed_stop no_automatic_rule_ever_stops_another_apps_leftover`, `every_refusal_sends_zero_signals`, `a_verified_leftover_of_an_ended_session_stops_and_is_audited`; `cargo test -p juggler-app --lib features::processes::stop::tests::the_identity_check_fails_closed_on_every_leg`; `src/screens/privacy-page.test.tsx` "says every claim, grouped as the list groups them", "every claim names at least one test" | each named test exists | every test the two lines name exists at the path named (checked 2026-11-07); vitest pass; the trust tests run in CI | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R2 guide sections: the Processes page, its states, Keep and reminders, the rules' dry run, other apps' leftovers | `docs/guide/processes.md`: "What it shows", "The states", "Find something", "Stop a process" (Keep, Remind me in 1 h), "Another app's leftovers", "Automatic cleanup" (Preview, Pause all), "Export snapshot", "What AI Juggler keeps"; lints: `scripts/plain_words_lint.py`, `scripts/terminology_lint.sh`, `scripts/docs_links_check.py` (run in CI) | every section; true to the code | every section is there. **One sentence is wrong**: "**Also after a crash** does the same for what a session left when AI Juggler closed unexpectedly" — the second rule (ADR-0010 Tier 2; `cleanup/sweep.rs`, `evaluate_tier`) is for a session whose own process crashed (a non-zero or signal exit); a session that ended while AI Juggler was closed has an unknown exit and no rule touches it. Missing: that sentence corrected, and (once ruled) a line on Claude Code's background tasks and the rules (AJ-E2405) **Closed 2026-11-07:** `docs/guide/processes.md` now says "Also after a crash" is for a session whose own process crashed, and that a session which ended while AI Juggler was closed is left to you; the ruled group rule (AJ-E2405) is in the same section | PASS | a8bd9979f | 2026-11-07 | Claude (self) |
| R3 `docs/qa/processes-lab.md`: a live-lab checklist using only lab-started processes | the checklist read against the code and lab 1 (details below) | runnable as written | not runnable as written for Claude Code: steps 2, 5, 6 and 7 expect a leftover after Stop or `/exit`, which Claude Code never leaves (the crash route is needed, and the rules don't allow it yet); step 8's one-liner trips Claude Code's safeguards; steps 1, 2, 4, 6, 8 and 10 expect words the code doesn't say. Needs the rewrite below **Closed 2026-11-07:** `docs/qa/processes-lab.md` rewritten from lab 1: the crash route (`kill -KILL` on the lab session's own `claude`, restart on crash off), two leftovers (the shell and the server, "Left 2 processes running"), `stubborn.py` for Didn't stop, `waits.py` for a zombie, and each step's expected words as the code says them; lab-started processes only | PASS | a8bd9979f | 2026-11-07 | Claude (self) |
| R4 evidence manifests for AJ-E2401–E2405 and E2407 | `ls docs/qa/evidence/AJ-E240{1,2,3,4,5,7}/manifest.md`; each read for a row per requirement | each present, current, a row per requirement | all six present. E2404, E2405 and E2407 rewritten here 2026-11-07; E2401, E2402 and E2403 were being rewritten the same day in the shared tree (uncommitted when this was written), each now with a row per requirement. Their own verdicts (several GAPs) are theirs, not this requirement's | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| Gate: the trust-claims test; the plain-words and terminology lints; the docs link check | `cargo test -p juggler-app --test trust`; the three lints (run in CI); `src/screens/privacy-page.test.tsx` | pass | vitest 13/13 pass; the rest runs in CI | PASS | f6fb096cb | 2026-11-07 | Claude (self) |

Visual (the Privacy page lines): `_gallery/84-privacy/page-{dark,light}.png`
(`privacy-gallery.spec.ts` "the Privacy page — ${theme}", last written
6a06e7483) shows the page's top only; the process lines are proven by the
vitest above, not by a capture.

## The checklist against the code (R3)

What the code needs to see, so the rewrite asks for it:

- **A leftover of AI Juggler's session** (`juggler-supervisor`
  `live_stats.rs`, the reconciler): the process was in the session's tree
  while its `claude` ran, long enough for a 2 s helper upsert to write its
  ledger row; then the session's own process exited (root row `exited`);
  then, after the provider's grace (30 s default,
  `DEFAULT_ORPHAN_GRACE_SECS`) and two agreeing ticks, it is still alive
  and its parent chain no longer reaches a live session root: `orphaned`,
  shown as Left behind (lab 1: 34–36 s).
- **A leftover of another app's session** (`features/processes/sightings.rs`):
  the process was sighted in that session's tree after it had run 10 s
  (`MIN_AGE_MS`); the session's own process then ended while it lives on
  with the same pid and start time: Left behind at the next tick (lab 1:
  4 s).
- **Claude Code 2.1.310 never leaves one on Stop or `/exit`**: it stops its
  background tasks itself (even TERM-ignoring or `nohup`ed ones). So the
  leftover route is the crash route: `kill -KILL` on the lab session's own
  `claude` (its pid from the session's "the session itself" row, checked
  with `ps -o pid=,comm= -p PID`). Its exit is a signal, which the rules
  call a crash (tier 2). Settings › Sessions
  "Restart sessions that crash" (`sessions.restart_on_crash`, off by
  default; a launch setting can choose for itself) must stay off. Claude
  Code's "Move to background and exit" on `/exit` is untried.
- **A zombie** (`juggler-supervisor/src/zombies.rs`, `live_stats.rs`
  `tree_rows`): a `Z` process whose parent is in a watched tree (a session's
  or another app's), read by `/bin/ps -A -o pid=,ppid=,stat=,etime=,ucomm=`
  (C locale) at a full scan (every 5th 2 s tick, only while a tree is
  watched). Shown as Waiting for its parent, naming the parent as the
  process table names it; never written to the ledger, never stoppable; in
  Needs attention after 60 s (`ZOMBIE_LINGER_MS`). It goes when the parent
  collects it or ends.
- **Never shown:** a server started with `nohup … &` from a one-line
  foreground command (it leaves the tree before a tick sees it; the guide
  says so). The lab must ask for a background task, not that.

Step by step, as `docs/qa/processes-lab.md` reads now:

1. Session runs `python3 -m http.server 8765` in the background; expects
   `python3 (http.server)` and "Started by Bash: …" on that row. The row is
   named `Python` for a Python that runs from `Python.app` (Homebrew, the
   command-line tools: `roles.rs` `short_name` knows only lowercase
   interpreter names; lab finding 7), and "Started by Bash: …" sits on the
   `zsh` row the Bash tool ran, with the server under it.
2. "Stop the session from its page, leaving the server running": Claude
   Code stops the server. Needs the crash route. Expects one leftover and
   "Left 1 process running": the shell and the server are both left
   behind, so 2 and "Left 2 processes running" (finding 3, unchanged).
3. Keep and Remind: right as written.
4. Stop… then Ended and the history: right, except the history names the
   session by its folder (`observe.rs` `ours_meta` title = the folder;
   finding 5, unchanged), not "the lab session's name".
5. TERM-ignoring script, "Stop the session": Claude Code stops it too;
   needs the crash route. The inline `python3 -c "…SIG_IGN…"` should be a
   lab script file (lab 1 used `stubborn.py`).
6. Dry run: "Would tidy python3 now" / "Nothing now; 1 after their wait"
   can't happen for a Claude Code task: every row is `never`, "It left its
   session's group", "Nothing to tidy now", at tier 1 and tier 2
   (AJ-E2405's open ruling). Keep's "You chose Keep" is right.
7. Another app's session: "End that session (`/exit`), leaving the server
   running": `/exit` offers "Exit and stop tasks", which stops it; needs the
   crash route on the outside `claude` after the server has run 10 s. Its
   title may be empty (finding 8). The D-41b stop now works for Homebrew
   python (lab bug 3, fixed); the history names it by its program name
   (`Python`), not `python3`.
8. Zombie: `python3 -c "import os,time; os.fork() or os._exit(0);
   time.sleep(180)"` made Claude Code (Opus 5.5) stop at its safeguards
   prompt. Needs a neutral lab script file run as a background task, e.g.
   one that starts `/usr/bin/true` with `subprocess.Popen` and sleeps 180 s
   without waiting for it (the supervisor's own zombie tests do the same).
   Expects "only python3 (pid) can clear it": the parent is named `Python`
   for such a python. "Stopping the parent (it is left behind once the
   session ends…)": after the session ends the parent's tree is no longer
   watched, so the zombie isn't shown under it; clear it with Stop… on the
   parent while the session runs, or let the 180 s run out.
9. Restart: AI Juggler's terminal session ends with the app (lab 1), so
   there's nothing to pick up unless the session outlives the app. Still
   needs the window.
10. This Mac: a `claude` inside the Claude app's tree is counted in the
    "Claude app" row and not listed again by title (by design,
    `machine_tools::graph_trees`; lab "bug 5"); only other tools' sessions
    (ChatGPT, "Codex in Terminal") are listed by title.
11. Sightings in SQLite: right as written (32-character hashes).
12. Nothing left: right; `scripts/qa/runtime_lab_snapshot.sh` now sets
    `LC_ALL=C` itself.

## Lab 1 (2026-10-26), what changed since

Run 2026-10-26 on the backend through a harness; steps 1, 3, 4, 5, 11 and
12 passed, 2 and 6 failed as written, 7's stop, 8 and 10 failed (bugs 3, 4,
5), 9 BLOCKED, the window and notification checks BLOCKED. Since then:
bug 1 (listed twice), bug 2 (a stopped process called "a different
program"), bug 3 (Homebrew python refused) and bug 4 (zombie never shown)
are fixed, each with a test; bug 5 is by design (step 10 above). A window
run of the rewritten checklist is still owed.

## Binary artifacts

None added; cited: `_gallery/84-privacy/page-*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
