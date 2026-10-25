# Evidence manifest — AJ-E2304

Task: `docs/tasks/E23-session-runtime.md` AJ-E2304, Open in terminal, and Start in terminal. Branch `e23-runtime`. Status: **DONE on fakes**: hand-over at a turn boundary (ADR-0022 decision 3's trade-off), the per-profile and per-session choice, and every fallback reason.

## What exists now

`features/runtime/commands.rs` (`runtime_handover`), `launch.rs`, `settings.rs`; `ui/src/pages/session/DrivenPanel.tsx`, `components/RuntimeSettings.tsx`, `RuntimePick.tsx`, `RightInspector.tsx` (the reason above the terminal).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 Open in terminal at a turn boundary, and back; never two writers | `cargo test -p juggler-app --test runtime` (`open_in_terminal_hands_the_thread_over_and_back`); `cargo test -p juggler-adapters --test codex_app_server` (`the_tui_takes_over_the_same_thread_at_a_turn_boundary`) | old session handed over; one writer | pass | PASS | 4749d6fc | 2026-10-25 | agent |
| Req.2 Start in terminal per session and per profile | `cargo test -p juggler-app --test runtime` (`the_terminal_runs_when_chosen_or_when_codex_cant_be_driven`); `src/runtime/settings.rs` unit tests | PTY with `--no-daemon` | pass | PASS | 4749d6fc | 2026-10-25 | agent |
| Req.3 each fallback with its reason, shown on the page | `cargo test -p juggler-app --test runtime` (same test: too old, another home); `cd ui && npx vitest run src/components/inspector-instance.test.tsx` ("…says why above its terminal") | the reason, in words | pass | PASS | — | 2026-10-25 | agent |
| UI: the runtime line, Open in terminal | `cd ui && npx vitest run src/components/ask-card.test.tsx` ("says how it runs, its limits honestly, and hands over to the terminal") | label, limits by length, the call | pass | PASS | e4b7e9af | 2026-10-25 | agent |
| Live: `codex resume <id> --no-daemon` after a hand-over | docs/qa/runtime-lab.md (the coordinator's live lab) | same thread | not run yet | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
