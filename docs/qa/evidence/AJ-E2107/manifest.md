# Evidence manifest — AJ-E2107

Task: `docs/tasks/E21-desktop-first.md` AJ-E2107, Needs you from real
signals. Re-audited against the code, its tests and the gallery on
`release/0.4.1`, 2026-11-07. Built in E21 (ac0e41dc0, 0.1.3); the tail's
turn for tools with no status word (Codex) in 0.1.5 (7a84fdf4f).

Status: R1, R3 and R4 evidenced; **R2 is a GAP for the Codex CLI**: Claude
Code's terminal tails are read for a pending permission or question, but
Codex has no `pending_ask` reader, and the approval events its tail-turn
reader looks for are, by its own note, kept out of Codex's rollouts today.
Not yet seen on the real setup either (a terminal session waiting on a
permission, on the maintainer's Mac).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the Claude app's recorded status maps blocked → Asks you, review ready → Done, review it, completed → Idle; only the status word is read | cargo `juggler-app`, `src/features/conversations/scan/tests.rs::the_states_map_to_e21s_words`; cargo `juggler-adapters`, `src/claude_live.rs` tests `an_app_session_keeps_its_status_word_and_never_its_summary_text`, `app_session_words_map_and_other_shapes_are_left_out`, `a_session_waiting_on_the_user_reads_as_waiting_and_its_words_are_never_kept` (cargo runs in CI); `cd ui && pnpm -s exec vitest run src/lib/rail-model.test.ts`: "every app's status maps onto Asks you, Working, Idle, Done (review it), Ended" | Idle+Blocked → AsksYou, ReviewReady → DoneReview, Completed → Idle, Waiting → AsksYou; `needs_action`, `status_detail`, `waitingFor`, `needs`, `detail` never in the read value | cargo cited as written (CI); vitest pass. The fixture puts a SECRET in every text field and asserts it is absent from the read session | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R2 terminal sessions read from the transcript's tail: a pending permission or question → Asks you | Claude Code: `juggler-adapters` `claude_asks.rs` tests and `juggler-app` `features/conversations/asks.rs` "a_question_asks_at_once_and_a_pending_call_only_after_a_quiet_minute"; Codex CLI: `juggler-adapters` `codex_turn.rs` `a_call_with_no_output_is_pending_and_its_output_answers_it`, `a_question_tool_asks_and_a_never_asking_turn_never_waits`, `a_finished_turn_or_the_users_next_message_leaves_nothing_pending`; conformance 98/98 | both tools' tails; only types, ids, roles read | cargo 9/9 codex_turn (92d6e0ce6); a look on the real setup with a live `codex` waiting on an approval is pending (the maintainer's setup) | PASS | 92d6e0ce6 | 2026-11-07 | Claude (self) |
| R3 "Done, review it" clears once the user has looked (a seen mark kept by AI Juggler) | `cd ui && pnpm -s exec vitest run src/lib/rail-model.test.ts`: "Done, review it clears once the user looked: AI Juggler's seen mark, or the app's own view" (`ui/src/lib/seen.ts`, newest 300 marks in AI Juggler's settings; `markSeen` from `HomeNeeds.tsx`, `row/RowShell.tsx` and `AppActions.tsx`) | clears on the seen mark or the app's `viewed_ms`; back after new activity | pass. `seen.ts` itself (the store and its 300 cap) has no test of its own; the clearing rule above is tested | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R4 anything inferred says "probably" | `rail-model.test.ts`: "anything worked out says probably"; `src/components/place-board.test.tsx`: "a row's label leads with its state; one worked out says probably"; cargo `asks.rs::a_question_asks_at_once_and_a_pending_call_only_after_a_quiet_minute` (asserts `probably`); gallery `_gallery/94-places/home-typical-dark.png` | "Probably asks you" on the row and in its label; recorded words never "probably" | pass; the capture (opened) shows "Answer the release question — Probably asks you" beside recorded "Asks you" and "Done, review it" | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| Gate: fixtures per signal | `cd ui && pnpm -s exec vitest run src/lib/rail-model.test.ts src/components/place-board.test.tsx` (run 2026-11-07); the cargo tests above (`cargo test -p juggler-adapters --lib claude_live`, `… --lib claude_asks`, `… --lib codex_turn`, `cargo test -p juggler-app --lib features::conversations`) — run in CI | every signal has a fixture | 2 files, 37 tests pass; cargo not run here (CI). No fixture for a Codex CLI tool call waiting on approval as Codex really writes it (see R2) | PASS (vitest); CI for cargo | 7a84fdf4f | 2026-11-07 | Claude (self) |

## What's missing for R2 (to close the GAP)

1. **A Codex `pending_ask`.** In `impl ToolLive for CodexAdapter`
   (`crates/juggler-adapters/src/codex.rs`, beside `tail_turn`), add
   `fn pending_ask(&self, tail: &str) -> Option<Ask>` backed by a new
   reader (say `crates/juggler-adapters/src/codex_asks.rs`, the twin of
   `claude_asks.rs`): within the open turn, a `response_item` of type
   `function_call` / `custom_tool_call` whose `call_id` has no matching
   `function_call_output` / `custom_tool_call_output` yet → `Ask::Pending`
   (`asks.rs` already waits `PENDING_FOR`, a quiet minute, before saying
   Asks you); a `request_user_input` call with no output → `Ask::Question`.
   Read only `type`, `payload.type` and `call_id`, never input or output
   text.
2. **Fixtures and tests** in `codex_asks.rs`, from
   `crates/juggler-adapters/tests/fixtures/codex/`: a turn with an open
   exec call and no output → `Some(Ask::Pending)`; the same with its
   output → `None`; an ended turn (`task_complete`) → `None`; a cut or odd
   line ignored; and a fixture taken from a real `codex` CLI rollout
   waiting on an approval (`approval_policy: untrusted`), redacted, to
   prove what Codex writes then.
3. **The real-setup look**: a `codex` and a `claude` in Terminal, each
   waiting on a permission, show "Probably asks you" within a tick
   (read-only, the maintainer's Mac or the lab).

## What exists now

1. **The apps' recorded words.** The Claude app's `status_category` maps
   blocked → Asks you, review_ready → Done, review it, completed → Idle.
   Claude Code's own session file records `status: "waiting"` when a
   session waits on a permission or a question; it reads as
   `LiveStatus::Waiting` → Asks you, recorded, not inferred. Only the
   status word is kept (`crates/juggler-adapters/src/claude_live.rs`).
2. **Terminal sessions' tails.** For a live, interactive session in no
   desktop app whose tool didn't say it asks, AI Juggler reads the last
   64 KB of its transcript, again only when the file changed
   (`crates/juggler-app/src/features/conversations/asks.rs`). The adapter
   says what the tail waits on (`pending_ask`; Claude Code: `claude_asks.rs`,
   keeping only line types and tool ids) and how its last turn stands
   (`tail_turn`; Codex: `codex_turn.rs`). An open question tool asks you at
   once; another tool call with no result asks you once nothing was
   written for a minute. Either says **"Probably asks you"**.
3. **Seen mark.** Opening a session another app runs, or going to it,
   marks it seen (`ui/src/lib/seen.ts`, kept in AI Juggler's settings, the
   newest 300). "Done, review it" clears once the seen mark, or the app's
   own last view, is after its last activity.
4. **The count.** The rail's Needs you, Home's heading and Needs you card
   count AI Juggler's sessions and the sessions other apps run that ask
   you or have something to review (`needsYou` in `ui/src/lib/rail-items.ts`;
   `rail-model.test.ts` "Needs you counts sessions other apps run, asks
   first, the longest waiting first").

## Binary artifacts

None added; cited: `_gallery/94-places/home-typical-{dark,light}.png`
(dark opened 2026-11-07: Asks you, Probably asks you, Done, review it).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
