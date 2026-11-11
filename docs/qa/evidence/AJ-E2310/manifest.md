# Evidence manifest — AJ-E2310

Task: `docs/tasks/E23-session-runtime.md` AJ-E2310, Driven sessions come back after AI Juggler restarts. Branch `e23-runtime`. Status: **IN-PROGRESS**. It is built and tested on fakes, with the app core killed for real (a child process, `SIGKILL`). The live run is the runtime lab's step 5 (`docs/qa/runtime-lab.md`).

## What exists now

`crates/juggler-app/src/features/runtime/pickup.rs` holds the mark (`runtime.live`: the conversation id and whether a turn runs, never text), the start-up pass, Resume and End. `driver.rs` keeps the mark exact. `state.rs` keeps marked sessions out of the "ended with AI Juggler" pass. `quit.rs` counts them as ones that come back. On the UI side: `ui/src/pages/session/DrivenPanel.tsx` (the cut-off line with Resume and End it), `components/QuitSheet.tsx`, `lib/attention.ts`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the mark holds the conversation and the turn, nothing said; it goes with the last session | `cargo test -p juggler-app --lib pickup` (`a_mark_holds_the_conversation_and_the_turn_and_nothing_said`) | exact JSON; the key removed | pass | PASS | — | 2026-10-25 | agent |
| R2 between turns: back under the same id, driven, on the same thread, nothing sent | `cargo test -p juggler-app --test runtime a_session_between_turns_comes_back_driven_on_its_thread_after_a_kill` | the server ends with the killed core; the same thread; one rollout | pass (3 runs) | PASS | — | 2026-10-25 | agent |
| R2 mid-turn: cut off, needs you, nothing runs until Resume; Resume once, on the same thread | `cargo test -p juggler-app --test runtime a_session_killed_mid_turn_says_so_and_waits_for_resume` | no process for 500 ms; `turn_cut_off`; Resume refused twice | pass (3 runs) | PASS | — | 2026-10-25 | agent |
| R2 End on a cut-off session; a stopped session never comes back | `cargo test -p juggler-app --test runtime a_cut_off_session_can_be_ended_and_a_stopped_one_never_comes_back` | stopped, no mark; nothing driven | pass | PASS | — | 2026-10-25 | agent |
| R3 the quit sheet says they come back | `cargo test -p juggler-app --lib quit` (`a_driven_session_comes_back_and_the_plan_says_so`); `cd ui && npx vitest run src/components/feedback.test.tsx` ("says a driven session comes back…") | the counts and the words | pass | PASS | — | 2026-10-25 | agent |
| R4 the app core killed with SIGKILL, between turns and mid-turn, then a new core on the same data (a child process holding the app's state) | `cargo test -p juggler-app --test runtime a_session_between_turns_comes_back_driven_on_its_thread_after_a_kill a_session_killed_mid_turn_says_so_and_waits_for_resume` (the R2 rows' kill tests; rerun for 0.4.1's release check: `--test runtime` 11 pass) | the server ends with the killed core; back on its thread between turns; cut off and waiting mid-turn | pass (3 runs each on 2026-10-25; rerun 2026-11-09) | PASS | 41e8a5750 | 2026-11-09 | Opus (verified) |
| UI: the cut-off line sends nothing, Resume and End carry the click | `cd ui && npx vitest run src/components/ask-card.test.tsx` ("whose turn was cut off…") | no call without a click | pass | PASS | — | 2026-10-25 | agent |
| Existing restart, crash and switching suites unchanged | `cargo test -p juggler-app --test restart_crashed --test tiers --test switch_setup --test fleet_switch --test trust --test runtime_guardrails` | pass | pass | PASS | — | 2026-10-25 | agent |
| Live: quit, crash between turns, crash mid-turn, stopped first (Codex; Claude Beta if time) | `docs/qa/runtime-lab.md` step 5 | as written there | not run yet | UNPROVEN | — | 2026-10-25 | agent |
| Visual: a cut-off session's page | the gallery run | looked at | not run | UNPROVEN | — | 2026-10-25 | agent |
| Live (lab 3, Codex): a quit and a crash between turns, a crash mid-turn, stopped first | [lab 3](docs/qa/evidence/AJ-E2301/lab-3-2026-11-07.md) step 5 | as written there | back within a second each time on the same thread; mid-turn: no server, nothing ran, "Its turn was cut off" with Resume and End it (Home and the rail read "Asks you": fixed, 9fde62dfe); stopped first stays stopped. Resume and End themselves weren't on the lab list then (added cb43df601, run next) | PASS (Resume pending) | 3d97d92cb | 2026-11-07 | agent |
| Live (lab 4, Codex): Resume and End after a cut-off turn | [lab 4](docs/qa/evidence/AJ-E2301/lab-4-2026-11-07.md) | driven again on the same thread, nothing sent by itself; End stops it, nothing runs | Resume in 0.8 s, the thread knew the aborted `ls`; End: stopped, notice gone; after a quit the resumed one came back, the ended one didn't | PASS | cb43df601 | 2026-11-07 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
