# Evidence manifest — AJ-E2310

Task: `docs/tasks/E23-session-runtime.md` AJ-E2310, Driven sessions come back after AI Juggler restarts. Branch `e23-runtime`. Status: **IN-PROGRESS**. It is built and tested on fakes, with the app core killed for real (a child process, `SIGKILL`). The live run is the runtime lab's step 5 (`docs/qa/runtime-lab.md`).

## What exists now

`crates/juggler-app/src/features/runtime/pickup.rs` holds the mark (`runtime.live`: the conversation id and whether a turn runs, never text), the start-up pass, Resume and End. `driver.rs` keeps the mark exact. `state.rs` keeps marked sessions out of the "ended with AI Juggler" pass. `quit.rs` counts them as ones that come back. On the UI side: `ui/src/pages/session/DrivenPanel.tsx` (the cut-off line with Resume and End it), `components/QuitSheet.tsx`, `lib/attention.ts`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 the mark holds the conversation and the turn, nothing said; it goes with the last session | `cargo test -p juggler-app --lib pickup` (`a_mark_holds_the_conversation_and_the_turn_and_nothing_said`) | exact JSON; the key removed | pass | PASS | — | 2026-10-25 | agent |
| Req.2 between turns: back under the same id, driven, on the same thread, nothing sent | `cargo test -p juggler-app --test runtime a_session_between_turns_comes_back_driven_on_its_thread_after_a_kill` | the server ends with the killed core; the same thread; one rollout | pass (3 runs) | PASS | — | 2026-10-25 | agent |
| Req.2 mid-turn: cut off, needs you, nothing runs until Resume; Resume once, on the same thread | `cargo test -p juggler-app --test runtime a_session_killed_mid_turn_says_so_and_waits_for_resume` | no process for 500 ms; `turn_cut_off`; Resume refused twice | pass (3 runs) | PASS | — | 2026-10-25 | agent |
| Req.2 End on a cut-off session; a stopped session never comes back | `cargo test -p juggler-app --test runtime a_cut_off_session_can_be_ended_and_a_stopped_one_never_comes_back` | stopped, no mark; nothing driven | pass | PASS | — | 2026-10-25 | agent |
| Req.3 the quit sheet says they come back | `cargo test -p juggler-app --lib quit` (`a_driven_session_comes_back_and_the_plan_says_so`); `cd ui && npx vitest run src/components/feedback.test.tsx` ("says a driven session comes back…") | the counts and the words | pass | PASS | — | 2026-10-25 | agent |
| UI: the cut-off line sends nothing, Resume and End carry the click | `cd ui && npx vitest run src/components/ask-card.test.tsx` ("whose turn was cut off…") | no call without a click | pass | PASS | — | 2026-10-25 | agent |
| Existing restart, crash and switching suites unchanged | `cargo test -p juggler-app --test restart_crashed --test tiers --test switch_setup --test fleet_switch --test trust --test runtime_guardrails` | pass | pass | PASS | — | 2026-10-25 | agent |
| Live: quit, crash between turns, crash mid-turn, stopped first (Codex; Claude Beta if time) | `docs/qa/runtime-lab.md` step 5 | as written there | not run yet | UNPROVEN | — | 2026-10-25 | agent |
| Visual: a cut-off session's page | the gallery run | looked at | not run | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
