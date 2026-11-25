# Evidence manifest: AJ-E3825 (Bot is a conversation through the real model)

Built 2026-11-25 on release/0.5.2 (Opus). Every route is tested over a fake engine that counts its
calls (`features/bot/fake.rs`) and the engine's lifecycle over fake Apple helpers (shell scripts in
temp folders, the existing engine test seam). The real model was not run here: AJ-E3829's check
runs `tests/bot_real_model.rs` on the reference Mac (Apple Intelligence on).

How the look-up is offered (R1, the builder's call): code runs the look-up first and passes the rows
as grounding the model must cite. The Apple helper's guided output (FoundationModels' generation
schema) is what the helper exposes; tool calling through the helper would need a protocol change
and a check Bot can't make on the model's own calls, so "facts first" (ADR-0034 §6.3) stays.

R1: PASS — `bot::answer::tests::no_reply_is_made_without_the_model_while_ai_is_ready` (13 questions: general, which model, my session, needs me, a named session, hi, help, code work, not recorded, nothing found, accounts, nonsense, German: each made ≥ 1 engine call and ended answered/general/unverified/failed, never rows or a code-made "nothing matches"; "Remember …" is the one code reply, an echo card, ADR-0037 §2.3), `records_found_are_grounded_and_cited` (citation check kept; a fabricated number isn't shown and no row stands in). UI: an unverified answer shows no rows (`BotTurn`).
R2: PASS — `bot::answer::tests::a_general_question_gets_a_general_answer_with_the_scope_line`, `bot::tests::asking_keeps_turns_that_page_find_and_clear` (greeting and code work answered by the model, off-topic scope line), vitest `bot-model.test.tsx` "a general answer (D-148)".
R3: PASS — `bot::answer::tests::which_model_comes_from_the_engine` (name and version given as facts; a reply naming GPT/OpenAI isn't shown), `bot::general::tests::which_model_must_name_its_own_and_no_other`, `bot::question::tests::work_and_which_model_are_told_apart`.
R4: PASS — `ai::run::tests::the_engine_starts_lazily_stops_when_idle_and_starts_again` (start checks the hello, `engine_started` logged, status Running with the helper's `context_size`), `with_no_ai_chosen_nothing_starts`; command `ai_engine_status` / `ai_engine_start`, DTO `AiEngineStatusInfo` (state, failure, model, engine, version, context, started/answered, today's counts) for AJ-E3826.
R5: PASS — vitest `bot-model.test.tsx` "the header" ("Apple on-device · Running", "Starting…", "Idle", "Failed: Apple Intelligence is off", Beta pill); `BotEngineBadge` in the panel head and on the Bot page.
R6: PASS — `ai::run::tests::every_failure_is_its_kind_and_logged` (off, not downloaded, helper missing, refused, timed out; each in the status and in the AI log as `engine_failed {kind}`; also `tracing::warn` with the kind), `engine_errors_map_to_their_kinds`, `bot::answer::tests::every_failure_is_said_with_its_kind` (no rows, no answer in its place), vitest "a failure" (each kind one line with its fix and Try again).
R7: PASS — `bot::general::tests::a_plain_answer_claiming_work_facts_is_caught` (a name, a count, a state, German too), `bot::answer::tests::a_plain_answer_claiming_work_facts_goes_grounded` (asked again on the grounded route, never shown as general; the model's own `about_their_work` too); grounded answers show record chips, general ones "Bot's own answer · not from your records" and none (vitest).
R8: PASS — `bot::answer::tests::details_and_topic_totals_come_from_the_engines_counts` (the engine's input count kept as a count, a missing output count labelled estimated, topic totals, today's totals, New topic resets), `ai::run::tests::todays_counts_add_up_and_say_when_estimated`, vitest "the details line" and "the topic's totals". Durations only: the helpers don't stream, so there is no first-token time.
R9: PASS — `bot::answer::tests::history_and_memory_go_with_a_follow_up_and_are_counted` (the five context parts, the alias used counted under preferences, earlier exchanges under asked, an answer marked wrong never sent back), vitest "the topic's totals" (context and memory breakdowns open, labels, "≈ … est.", Manage what Bot remembers). Memory read through `bot::memory::with_aliases` (ADR-0037's interface), nothing straight from its tables.
R10: PASS — `ai::run::tests::the_engine_starts_lazily_stops_when_idle_and_starts_again` (idle 400 ms: the helper is let go, its process reaped (`kill -0` fails), status Idle, the next question starts it again); the Apple engine's idle watch (`IDLE_STOP` = the helper's own 60 s) drops the helper through its `Drop` (stdin closed, kill, wait). "Starting Apple's on-device model…" from the engine's name (vitest).
R11: PASS — `bot::model::ask_for_check` (turn, outcome code, model name) and `tests/bot_real_model.rs` (ignored; `AIJ_APPLE_AI_HELPER=<helper>`; prints one JSON line per question). Its run is AJ-E3829's.

## Usability answers (PAT-USE), as built

1. Someone with Apple's AI on expects an assistant that answers: match — every question reaches the model; "what is 3+5" gets "8." labelled as Bot's own answer.
2. Each answer says what it rests on; the header names the model: match — grounded answers cite record chips; general answers say "Bot's own answer · not from your records"; the head reads "Apple on-device · Running · Beta".
3. It replaces the records-only reply; with no AI, Bot stays its setup guide: match — `ai_ready_or_refuse` unchanged; no second chat surface.
4. A failure is one plain line with its fix, never "nothing matches": match — `BotFailedAnswer` with Try again and Details.
5. Starting the helper may show macOS's prompt only when its signature changed: match — AJ-E1555.
6. Watch for: invented records (citation check, fabrication guard), general shown as records (separate outcome and label), a long wait (the "Starting …" line, the header's Answering…), the model naming itself wrongly (facts given, other names refused), a failure looking like "nothing matches" (separate outcome).

Details line and totals (R8/R9): (1) cost and grounding on demand: match, a muted line under each answer; (2) "model · in · out · time · records" and "This topic … · context … / … · memory n": match; (3) replaces nothing; the Models card reads the same DTO: match; (4) no answer yet: no totals line; no engine count: "≈ … est.": match; (5) none; (6) numbers stay muted (text-faint/text-muted, PAT-COLOR), estimates labelled, breakdown parts sum to the estimate of what was sent.

PAT-SURF: no new borders; popovers reuse the menu surface (tone + soft shadow). PAT-COLOR: muted greys for numbers, danger only for "Failed". PAT-OSP: no new prompt. PAT-SER-7: no internal terms on screen ("helper" is the registry's own word for Bot's helper).

Left: AJ-E3829's real-model check on the reference Mac; the a11y gate and an Opus critique in both themes with captures (the coordinating session's walk).

## Verdicts

| Req | Verdict | Date |
|---|---|---|
| R1 | PASS | 2026-11-25 |
| R2 | PASS | 2026-11-25 |
| R3 | PASS | 2026-11-25 |
| R4 | PASS | 2026-11-25 |
| R5 | PASS | 2026-11-25 |
| R6 | PASS | 2026-11-25 |
| R7 | PASS | 2026-11-25 |
| R8 | PASS | 2026-11-25 |
| R9 | PASS | 2026-11-25 |
| R10 | PASS | 2026-11-25 |
| R11 | PASS | 2026-11-25 |
