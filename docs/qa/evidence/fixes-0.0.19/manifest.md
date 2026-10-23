# Evidence manifest: Link off follow-up (fix/link-off-follow-up)

Not a task's evidence. This covers the two Lows from #135's re-review. The third, an explicit pairing ack, needs a protocol change and is recorded in the queue note.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| 1. No question is asked while Link goes off | The prompt queue has a `closed` flag under its own lock. `stop` closes it and answers every waiting question "no"; `start` opens it. `ask` on a closed queue answers "no" at once, so a peer command that arrives between the drain and the node's end can't hold the off for 20 s. `link::unfinished_pairing_tests :: a_closed_queue_answers_no_without_asking` (closed: "no" in under 1 s, nothing queued; opened again: asked and answered) | — | passed | PASS | cf499ac0 | 2026-10-22 | claude-opus |
| 2. Nothing sent says so | `run_outbound` says how it ended: `Done`, `NotSent` (Link was off, it never started) or `Ended` (Link went off while it ran). A command that never went out reads "Link is off on this Mac — nothing was sent."; one cut off after sending keeps "it may still have run there". `remote::link_off_tests :: link_off_says_whether_anything_was_sent`, and `nothing_starts_or_lands_outside_its_run` (now `NotSent`) | — | passed | PASS | cf499ac0 | 2026-10-22 | claude-opus |
