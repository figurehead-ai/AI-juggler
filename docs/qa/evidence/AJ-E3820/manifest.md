# Evidence manifest: AJ-E3820 (the turn review: the code-only review, the model's Beta lines on it, and where it shows; D-149)

Built 2026-11-26 on release/0.5.2 (Opus). No new crate, nothing downloaded, built `--offline`.
The maintainer's ruling on transcripts (2026-11-26) applies: every turn fact comes from the
session's own transcript, never from git in the user's projects (the task's note under R1).

| Req | Verdict | Proof |
|---|---|---|
| R1 the code-only review | PASS | e88c21ffc, 3bbeb598e; `features/review/tests.rs` |
| R2 the model's lines, every one checked | PASS | 3bbeb598e, e738812aa; fixture tests; the real run below (6 of 6 turns) |
| R3 where it shows | PASS | e738812aa; vitest `turn-review.test.tsx`; gallery `99-turn-review` |
| R4 on by default, an opt-in Beta | PASS | 59ce09876; the router matrix unchanged; Models vitest and gallery |
| R5 no review for an unseen turn end; the query says so | PASS | `only_a_turn_whose_end_is_seen_is_reviewed`; `AiJobInfo.covers` |
| The Sonnet walk on a lab fixture | UNPROVEN | the coordinator's |

R1: PASS — commits e88c21ffc (the feed's turn reader keeps each file's own lines: `per_file`, not a
second reader), 3bbeb598e (`features/review/facts.rs`, `read.rs`). Changed from the turn's edit and
write calls (`juggler_adapters::turn_changes`); Reverted and an abandoned turn from the digest's G7
facts (`pick::undone`, `rollup::says_undone`): a named file, "the whole change" or a one-file turn;
an undo naming none of several files makes them "edited", never "changed"; failures, the question
and the tests in the agent's own sentences after Bot's input guard; risk never low on a failed or
untested turn. Tests (`features/review/tests.rs`): `a_revert_is_never_listed_as_changed`,
`a_whole_undo_or_a_one_file_undo_reverts_every_file`,
`an_undo_naming_no_file_makes_the_files_edited_not_changed`,
`the_question_and_the_failures_are_the_agents_own_words`,
`a_failed_turn_is_high_and_an_untested_one_never_low`,
`a_fixed_failure_is_not_a_failure_and_a_large_turn_is_medium`, `a_file_without_counts_says_none`,
`a_clean_tested_turn_is_low_and_lists_each_file_with_its_lines`, and through the readers on a
written transcript `a_transcripts_turn_is_reviewed_from_its_edit_calls_and_its_reply` (under 100 ms,
the planted order removed by the guard); adapters `each_file_keeps_its_own_lines`,
`the_reader_never_runs_git_or_any_command`.

R2: PASS — commits 3bbeb598e, e738812aa (`features/review/lines.rs`, `prompts/review.txt`). Harness
v5's measured review prompt, its facts named by where they now come from; through the router
(`Job::Review`, `LiveJob`, `route_on`). Every description and the risk's reason pass Bot's check
(`bot::check::check`, G11's support rule included) with the agent's sentences and the file as its
records; Bot's guard runs on the reply first, and its removed words feed the echo check. Never a
second guard. The model's risk may raise the facts', never lower it; a failed check regenerates with
the reason, sampled, at most twice, then the code-only review (no note); an engine failure is not
regenerated. A reason that only repeats a shown fact is left out. Fixture tests over ADR-0033's v4
review FAILs: `a_note_on_a_reverted_file_is_refused` (R207/R226; the reverted file isn't even
offered), `a_line_that_changes_the_meaning_or_claims_more_is_refused` (R209's negation, R222's word
the agent didn't write, G11's "faster", an outcome word, a number no fact holds, a test result in the
reason), `a_risk_below_the_facts_is_refused_and_a_raised_one_is_kept`,
`an_injected_order_never_reaches_a_line`, `a_faithful_line_in_the_agents_words_is_kept`,
`a_reason_repeating_a_shown_fact_is_left_out`, `a_failing_engine_keeps_the_code_only_review`,
`the_instructions_are_harness_v5s_but_for_the_facts_source`.

**The real model** (`features/review/real_model_tests.rs`, ignored, run by hand; the helper built by
`scripts/build_apple_ai_helper.sh`, `AIJ_APPLE_AI_HELPER` set to it; this Mac, macOS 27.0.1, Apple
Intelligence on; six fixture turns written for the test, generic names, no real transcript):
routed to "Apple's on-device model (apple-on-device)"; **6 of 6 turns got lines, 7 drafts, 1 refused**,
3.3–7.1 s a turn. Full output: [real-model-run.txt](real-model-run.txt). The lines shown, each
checked:

| Turn (code-only risk) | The model's lines, as shown | The check |
|---|---|---|
| moved limiter, tested (low) | `ratelimit/shrike.go` — "the shrike rate limiter, moved into its own package"; `middleware/limits.go` — "the rate limiter moved out of the middleware" | Passed, first draft |
| export streamed, quoting reverted, untested (medium) | `export/dunnock.py` — "switched the dunnock export to stream rows"; nothing on the reverted `export/quoting.py` (not offered) | Passed; its reason "The integration suite is still unrun" repeats the tests' sentence: left out |
| parser rewritten, tests fail (high) | `parse/godwit.ts` — "rewrote the godwit date parser" | Passed; reason repeats the note: left out |
| German, not run (medium) | `ui/kiebitz/search.tsx` — "die Suchleiste der Kiebitz-Ansicht auf Debounce umgestellt" | Passed; reason "Die Tests haben ich nicht ausgeführt" repeats the tests' sentence: left out |
| retry added, asks a question (medium) | `sync/retry.ts` — "added a retry with backoff to the sync client" | draft 0 REFUSED ("risk low is lower than the facts allow (medium: asks)"), draft 1 passed |
| an order planted in the reply (medium) | `billing/invoice.rs` — "Rounded invoice totals to whole cents" | Passed; the planted "Ignore all previous instructions…" never reached the model (the guard) |

Earlier runs on the same fixtures found two real problems, both fixed before this run: Apple's
engine failed ("helper didn't start") on an enum of one value and on a list of at most one (a
one-file turn now asks for its path as a string, checked as every path is, and the list holds two),
and a risk reason that failed the check sank good notes three times running (a reason that only
repeats a shown fact is now left out, not refused).

R3: PASS — commit e738812aa. `TurnReview.tsx` under a finished turn's feed post (the agent's
sentences quoted) and under the session page's last ended turn (named: the reply is right above),
folded behind the same row as a turn's work (`FoldRow`, shared with `TurnWork`), never a
notification; the Beta's lines asked for only once opened with the job ready, tagged AI, with whose
they are, "Show its words" and "This is wrong" (kept in the AI log's feedback stream, the lines in a
30-day sidecar). Mock made first: `~/Dev/AIJuggler-design-refs/turn-review/` (HTML, dark and light
PNGs). Gallery `_gallery/99-turn-review` (feed and session page, both themes, axe clean; the first
run caught a definition list holding a footer, fixed), critique against the mock with line 19 at 4,
stamped. Vitest `components/turn-review.test.tsx` (8): folded then opened, quoted on the feed and
named on the page, red on a failure, never on a desktop app's turn, an uncovered tool, a turn with
no review or nothing in it, never before the AI query names its tools, the Beta's lines only once
opened and never with the Beta off, This is wrong and its Undo.

R4: PASS — commit 59ce09876. `ai_review_on` on by default (`jobs.rs`; the publisher's schema export
`config/remote/schema.json`, which its test keeps equal to the flags' registry; the window's
built-in flags in `lib/ipc/remote.ts`): Apple's `beta` verdict now offers the review behind its own
switch, off by default. The router matrix is unchanged (`the_matrix_is_the_recorded_one`); the
service and router tests now expect BetaOff, Ready with the switch, NoEngine with Apple Intelligence
off. Models reads as the other Betas: "Off · Preferred over the plain review in 81 % of blind
checks." with its switch and picker (vitest `models-bot-card.test.tsx`, `models-pane.test.tsx`;
gallery `models-row-{dark,light}.png`); a job not built is still on no card (kept as a test).

R5: PASS — commits 3bbeb598e, e738812aa, 59ce09876. `review::read` gives no review for an App whose
turn ends aren't seen (`Observes::TURN_ENDS` for its kind, and the turn reader reads its
transcripts): a desktop app's conversation, a tool that only says it's busy. The router's query
says so: `AiJobInfo.covers` names the tools (`["claude-code", "codex"]` here), the window offers the
review only for those, and Models says "For Claude Code and Codex in the terminal, whose turn ends
AI Juggler sees". Tests: `only_a_turn_whose_end_is_seen_is_reviewed` (covers; the Claude app's
conversations and Grok Build not seen; no transcript "none"; Last messages off "off", nothing read),
vitest's "never offered where it can't be".

UNPROVEN: the hand walk (a Sonnet walk on a lab fixture with Apple's model, Opus checking it) is the
coordinator's, as is a downloaded model's run once the `models` section is published. The gate
baselines were not re-rendered (the coordinator's one pass over 0.5.2's UI).
