# Evidence manifest — AJ-E3106 Beside a session's conversation

Task: `docs/tasks/E31-everything-links.md` AJ-E3106, beside a session's
conversation what the tools' own apps show (subagents running, the
sessions it started) and each tool's own marks. Req. 1–2 built in 0.3.4
(`ui/src/pages/session/ConversationSwitcher.tsx`, made a tree in
1a27d21e9); Req. 3–4 built 2026-11-04 (17883c512). Audited against the
code on `release/0.4.1`, 2026-11-07.

Vitest run 2026-11-07: `cd ui && pnpm -s exec vitest run
src/pages/session/conversation-switcher.test.tsx
src/pages/session/turn-marks.test.tsx`: 2 files, 6 tests, all passed.
Cargo tests and the Playwright gallery spec are cited as written and run in CI.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 beside the Conversation card: subagents running now, then the last few ended ("Done · N ›" opens the Subagents tab); each opens its own conversation in the same card, with a way back | `ui/src/pages/session/conversation-switcher.test.tsx`: "drills into a running subagent in place, keeps the session's own pane mounted, and Esc comes back", "shows no list when there's nothing besides the conversation"; Playwright `ui/tests/visual/subagents-gallery.spec.ts` "0.3.4: the Overview switches its conversation to a subagent's, in place, and back" (runs in CI); gallery `_gallery/88-subagents/switcher-own-1280x800-dark.png`, `switcher-subagent-1280x800-light.png` (opened) | running listed; ended folded with a count; a pick shows in place; Esc/Back returns; own pane never remounted | tests pass: "Subagents running · 1", "Finished subagents · 1", the pick hides (never unmounts) the own pane, Esc returns (mounts = 1). The captures show "Subagents running · 3", "Sessions it started · 1", "Finished subagents · 7" folded, then a finished subagent's conversation in place with Back. The fold's last line "All N in Subagents ›" (`switcher-all`, `showTab("subagents")`) is in the code but no test or capture clicks it | PASS | d79e285cc | 2026-11-07 | Claude (self) |
| R2 the same list shows the sessions this one started, from the Claude app's own record (`spawnedFrom.sessionId`; archived left out); each opens its page | cargo `juggler-app`, `src/features/conversations/scan/tests.rs` `the_sessions_a_session_started_come_from_its_apps_record_newest_first_archived_left_out` (runs in CI); cargo `juggler-adapters`, `src/claude_live.rs` `an_app_session_keeps_its_status_word_and_never_its_summary_text` (reads `spawned_from`, runs in CI); vitest `conversation-switcher.test.tsx` "lists the sessions it started and shows one in place, with its page a click away" | read from the app's record; archived and unknown parents left out; each one opens its page | the record and the archived rule are tested (newest first, an archived child and an unknown parent dropped). The vitest only renders the not-found branch ("isn't in this computer's list"): the found branch, its "Open its page" button (`switcher-open-page` → `host.openConversation`), is never rendered or clicked by any test or capture **Closed 2026-11-07:** vitest `src/pages/session/conversation-switcher.test.tsx` "a started session on this computer's list shows in place and opens its page (AJ-E3106 R2)" seeds it in the store, opens it in place and clicks `switcher-open-page` (`host.openConversation` gets it) | PASS | 6cf26c72d | 2026-11-07 | Claude (self) |
| R3 Codex's `final_answer` (its `phase`) decides a turn's reply; commentary folded with the work even after it | cargo `juggler-adapters`, `src/conversation_feed_tests.rs` `codexs_final_answer_is_its_own_block_and_marked` (runs in CI); vitest `ui/src/pages/session/turn-marks.test.tsx` "Codex's final answer is the reply, its commentary work, even after it", "with no mark, the last words are the reply, as before" | the marked block is the reply; commentary before and after is work | the feed keeps three blocks, only the `final_answer` one marked; `foldOf` takes it as the reply and both commentaries as work; unmarked turns keep the old rule | PASS | 17883c512 | 2026-11-07 | Claude (self) |
| R4 a step's line is its own description when recorded (Bash `description`), with the command beside it | vitest `turn-marks.test.tsx` "a step shows its own description with its command beside it"; cargo `juggler-adapters`, `src/conversation_feed_tests.rs` `a_scrubbing_log_keeps_only_redacted_words_and_counts_them` (the Bash `description` read into `step.input.description`, redacted; runs in CI); `CallInfo.description` in `crates/juggler-app/src/features/conversations/feed/dto.rs` | description shown, command beside it | `last-turn-step-described` holds "Run the full preflight" and `ran "just preflight"` | PASS | 17883c512 | 2026-11-07 | Claude (self) |
| R5 read-only and paged, like every list (D-37) | code read: `ConversationSwitcher.tsx` `useSwitcherLists` | nothing written; every list bounded with a way to the rest | read-only holds by the code (only reads: `subagentsList`, `conversationsStarted`, `subagentsFollow`). Running is the first 30 (`limit: 30`) with no "more" past them; Done the newest 8 then the paged Subagents tab. "Sessions it started" is unbounded: `conversations_started` (`crates/juggler-app/src/features/conversations/commands.rs`, `started_impl`) returns every link. No test asserts any bound or the read-only rule **Closed 2026-11-07:** bounded everywhere: Running shows 30 with a last line to all of them in Subagents ("more running than it lists…"), Sessions it started ten at a time ("…ten at a time, the rest a click away"), and the backend keeps at most `STARTED_MAX` 100, newest first (`cargo test -p juggler-app --lib features::conversations::commands::started_tests::the_sessions_it_started_come_newest_first_and_bounded`); read-only by the code (reads only) | PASS | 6cf26c72d | 2026-11-07 | Claude (self) |
| Gate: the touched screens' tests | `cd ui && pnpm -s exec vitest run src/pages/session/conversation-switcher.test.tsx src/pages/session/turn-marks.test.tsx` | pass | 6 / 6 passed (2026-11-07) | PASS | 17883c512 | 2026-11-07 | Claude (self) |

## Closed (2026-11-07)

R2 and R5 are closed by 6cf26c72d (rows above). Also tested there: the
finished ones' last line opens the Subagents tab (R1, "the finished ones'
last line opens the Subagents tab"). Still not on file: the K-walk on a
live session with running and ended subagents and started sessions (a lab
runs no live session; it waits for a live run with the maintainer's
approval of one tool).

## Binary artifacts

None added; cited: `_gallery/88-subagents/switcher-own-1280x800-{dark,light}.png`,
`_gallery/88-subagents/switcher-subagent-1280x800-{dark,light}.png` (last
re-shot f5a3fc2af; their top bar predates the header's Back/Forward
change, which this task doesn't touch).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none; fixture words only)
