# Evidence manifest — AJ-E1703

Task: `docs/tasks/E17-agent-pools.md` AJ-E1703, context cards (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-24, claude-opus.

Gallery: `docs/qa/evidence/_gallery/97-pool-canvas/` (`pools-screen-*`, `canvas-*`, `inspect-member-*`, `list-*`; 1280×800 and 1024×700, both themes, axe 0 critical or serious), rendered by `ui/tests/visual/pool-canvas-gallery.spec.ts`. The 0.2 captures of the pool page's cards and access (`91-pools/pool-cards-*`, `pool-access-*`, `pool-sheet-*`) were retired with the 0.3 canvas (AJ-E2808): a member's context card is the inspector's member view now, and the List view keeps its rows.

## What exists now

1. **What a session is working on, read from its transcript** (`juggler-adapters`):
   - `transcript_tail`: the incremental tail reader the subtasks tick had, made generic over what it feeds (`TailLog`), so there is one reader, not two copies. `SubtaskTail` is `TranscriptTail<SubtaskLog>`.
   - `activity`: the five files a session changed most recently (newest first, each once, last two path parts as handovers name them) and the step its own task list has in progress (≤ 140 characters). Each tool's parser hands its tool calls over (`activity_line` in the Claude Code and Codex parsers); `tool_summary::edited_files` and `task_list` say what a call changed and set. Nothing else is kept: no message, command or output.
2. **Read on the read model's 2 s tick** (`features/pools/cards.rs::tick`, beside the subtasks tick): only live sessions, only what was appended since the last read; a session that ended keeps what was read while it ran, until AI Juggler quits. Readers see the last published view.
3. **The cards** (`cards_impl`, command `pools_cards`, a Read the Dev bridge serves): one per member, lead first, then the most recently active. Name, place, tool (its display name), state, role and last activity come from the read model; the said line and files from the tick's view. Archived sessions are left out.
4. **Honest fields (D-06)**: each unknown says "Unavailable — <why>": a transcript not read yet ("read only while it runs in AI Juggler"), nothing said ("it hasn't said"), a finished task list ("nothing on its task list is in progress"), the task (E19: "AI Juggler doesn't keep tasks yet"). "No file changed yet" is a known empty, not an unknown.
5. **The agent's words are its own**: the said line is the step from its own task list, shown in quotes and marked "said by the agent".
6. **The account (D-33)**: a card carries the account's label only, never its id, an email or a plan, and none when the label is an email address. The same card is what agents will be shown (AJ-E1704).
7. **UI**: the pool page (`pages/PoolPage.tsx`, lazy; a "pool" page kind in the page registry). Members: rows like Home's (state, project mark, name and place, tool and account label, Lead, last activity), the rest behind each row's disclosure (role, It says, Task, Files changed). Then Agent access. A rail pool header opens it.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 built only from what AI Juggler knows | `crates/juggler-app/tests/pool_cards.rs::a_card_says_what_ai_juggler_knows_and_unavailable_for_the_rest`, `the_tick_reads_the_transcript_and_the_card_says_whose_words_they_are`; adapters `activity::tests` (5, Claude Code and Codex lines, the tail) | PASS |
| Req.1 the agent's own line, marked as said by the agent | vitest `pools.test.tsx` › context cards ("said by the agent"); gallery `97-pool-canvas/inspect-member-*` (the member's card in the inspector, its line marked as said by the agent) | PASS |
| Req.2 missing data says "Unavailable" | `pool_cards.rs` (before the tick, no transcript, the task); vitest "says Unavailable and why, never an empty guess" | PASS |
| Req.2 account details never shown (D-33: the label only) | `pool_cards.rs::a_card_carries_the_account_label_and_never_an_email_or_plan` (no id, plan, `@`, launch settings; none for an email label) | PASS |
| Performance: built within the read model's tick | the tick reads only appended bytes of live sessions (16 MiB at most per read); `cards_impl` only joins the tick's view with the sessions list | PASS |
| Quality gate: honest-field tests (D-06) | the four `pool_cards.rs` tests | PASS |
| Visual: the pool view with cards | gallery `97-pool-canvas/` `canvas-*`, `inspect-member-*` and `list-*` (both sizes and themes; until 0.3, `91-pools/pool-cards-*`); keyboard walk in `pool-canvas-gallery.spec.ts` (arrows between members, Enter opens one in the inspector) | PASS |

## Not covered here

- The current task: tasks are E19; the field is unavailable until then.
- What agents themselves report (`set_status`) and the cards resource: AJ-E1704. When `set_status` lands it takes the said line's place.
