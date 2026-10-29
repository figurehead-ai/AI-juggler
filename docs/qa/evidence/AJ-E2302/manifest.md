# Evidence manifest — AJ-E2302

Task: `docs/tasks/E23-session-runtime.md` AJ-E2302, The runtime contract. Branch `e23-runtime`. Status: **DONE on fakes**: the contract, typed events, the line reader and ask book, the supervisor's driven spawn, and the conformance kit's checks 8–10 for every subject.

## What exists now

`crates/juggler-adapter/src/runtime/` (`mod.rs`, `events.rs`, `lines.rs`, `book.rs`), `reports.rs`; `crates/juggler-adapters/src/fake_driven.rs`; `crates/juggler-supervisor/src/piped.rs`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 Terminal or Driven per launch; per profile, per session | `cargo test -p juggler-adapter --lib runtime` (`runtimes_have_wire_words`); `cargo test -p juggler-app --test runtime` (`a_profile_choice_is_kept_and_a_bad_word_refused`) | wire words round-trip; a bad word refused | pass | PASS | c54ccfcc | 2026-10-25 | agent |
| Req.2–3 DrivenRuntime and DrivenSession; maturity and default as data | `cargo test -p juggler-adapters --test fake_driven` (`a_driven_session_runs_turns_and_carries_only_the_users_answers`) | turns, steer, interrupt, answer, model, compact, limits, release | pass | PASS | c54ccfcc | 2026-10-25 | agent |
| Req.4 typed events, bounded text | `cargo test -p juggler-adapter --lib runtime` (`events_round_trip_as_tagged_json`, `text_is_bounded_on_a_character`) | tagged JSON; 4 KB cut on a character | pass | PASS | c54ccfcc | 2026-10-25 | agent |
| Req.5 TurnInput with its origin (E18 delivers without a PTY) | `cargo test -p juggler-adapters --test fake_driven` | a delivered turn keeps its sender | pass | PASS | c54ccfcc | 2026-10-25 | agent |
| Req.6 kit: PROTOCOL_SERVER iff driven(); isolation; no bypass; the tool's own options | `cargo test -p juggler-adapters --test adapter_conformance` (`runs_both_runtimes_honestly`, `keeps_the_tools_own_option_rules`, `adds_state_reports_honestly`) | every subject | pass | PASS | 74324796 | 2026-10-25 | agent |
| The ledger owns a driven child; stopped with its session | `cargo test -p juggler-supervisor --test driven` (both tests) | nothing outlives the session | pass | PASS | af274ef9 | 2026-10-25 | agent |
| Perf: event to state < 50 ms (fake) | `cargo test -p juggler-app --test runtime` (`a_driven_session_is_exact_and_answered_only_by_the_user`) | < 50 ms | within the 150 ms emitter tick; not measured alone | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
