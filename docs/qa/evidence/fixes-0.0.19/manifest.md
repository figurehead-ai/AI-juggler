# Evidence manifest: Link off follow-up (fix/link-off-follow-up)

Not a task's evidence. This covers the two Lows from #135's re-review. The third, an explicit pairing ack, needs a protocol change and is recorded in the queue note.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| 1. No question is asked while Link goes off | The prompt queue has a `closed` flag under its own lock. `stop` closes it and answers every waiting question "no"; `start` opens it. `ask` on a closed queue answers "no" at once, so a peer command that arrives between the drain and the node's end can't hold the off for 20 s. `link::unfinished_pairing_tests :: a_closed_queue_answers_no_without_asking` (closed: "no" in under 1 s, nothing queued; opened again: asked and answered) | — | passed | PASS | cf499ac0 | 2026-10-22 | claude-opus |
| 2. Nothing sent says so | `run_outbound` says how it ended: `Done`, `NotSent` (Link was off, it never started) or `Ended` (Link went off while it ran). A command that never went out reads "Link is off on this Mac — nothing was sent."; one cut off after sending keeps "it may still have run there". `remote::link_off_tests :: link_off_says_whether_anything_was_sent`, and `nothing_starts_or_lands_outside_its_run` (now `NotSent`) | — | passed | PASS | cf499ac0 | 2026-10-22 | claude-opus |
| #137 review 1: questions are asked again after off-on | `fleet_link :: after_link_off_and_on_a_question_is_asked_and_allow_runs` (Link off, then on; a peer's "Ask each time" command is asked, Allow lets it run to the node's own checks) | fails without the reopen | with `prompts.open()` removed: timed out waiting for the question; now passes | PASS | 21a56c4f | 2026-10-22 | claude-opus |
| #137 review 2: a closed question says Link off | `ask` answers Allowed, Denied or LinkOff; LinkOff reads "Link is off on that Mac — nothing ran." (`link_off`), not "Not allowed". `link::unfinished_pairing_tests :: a_question_refused_by_link_off_says_link_off` | — | passed | PASS | 21a56c4f | 2026-10-22 | claude-opus |
| #137 review 3 and 4: one mapping, one message each | `finished_or_link_off` maps Done, NotSent and Ended in one place; the test calls it with each and checks each message, so a swap fails. `NOTHING_RAN` and `NOTHING_SENT` are constants (used in link.rs, remote.rs and crossmachine.rs) | — | passed | PASS | 21a56c4f | 2026-10-22 | claude-opus |

## Gates

| Gate | Head | Result |
|---|---|---|
| `just verify` (`CARGO_INCREMENTAL=0`) | bdc6b21c (with the #137 review) | 150 test binaries, 1,229 passed, 0 failed; every step after the tests OK; nothing left running |
| `cargo deny check` | bdc6b21c | advisories, bans, licenses, sources ok |
| vitest, visual, a11y | — | nothing under `ui/` changed |
