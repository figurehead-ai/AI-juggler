# Home feed item catalogue

**Sources:** I read `origin/release/0.5` read-only. A Sonnet scan gathered the file:line evidence, and I re-checked the key claims myself:
- notices retention: `NOTICE_KEEP_DAYS = 30`, `storage/notices.rs:28`;
- the need and app kind registries: `notifications/centre.rs:35-64`;
- `switch_edges` has only `created_at`: `storage/mod.rs:519-540`;
- `task_events` is append-only: `storage/tasks.rs:82-98`;
- MorningSummary groups: `overnight.rs:112-127`;
- `final_answer`: `conversation_feed.rs:202`.

Crate paths are short: `notifications/…` means `crates/juggler-app/src/features/notifications/…`.

**Level** is the calm attention model:
- L3 Broken;
- L2 Needs you;
- L1 Note;
- L0 Calm.

**Pinned** means the item sits in the feed's top "Needs you now" zone while it's open. It is loud there, and that is the Home area's one loud mark. Its copy in the chronological stream is a quiet history line.

**Rollup key** is what coalesces items. "Window" means items with the same key within that span merge into one card. The card shows a count, up to 3 sample names and the newest time, and expands to the full list.

## 1. Sessions: AI Juggler's own and other apps' conversations

| # | Item | Source (exists?) | Level | Rollup rule | Example sentence | Click target |
|---|---|---|---|---|---|---|
| S1 | **Asks you** | `notices` need `asks` (`needs.rs:204`, `:218`). Own sessions: `NeedsAttention::WaitingUser`. Other apps: conversation `state=asks_you`. **Exists.** | L2, pinned | One per session. Updates in place ("waiting 24 min"). Resolved: it leaves the pinned zone, and a quiet "You answered" line stays. | **Migrate the billing webhooks** asks you: "Allow edits to 3 files in payments-api?" · Claude app · Work Max | Its page at the question. ↗ goes to the app. |
| S2 | **Probably stuck** | Need `stuck` (`needs.rs:205`), rule `stuck_no_output`. **Exists.** | L2 (ring), pinned | One per session | **Nightly tests** has shown no output for 18 min, so it's probably stuck · Claude Code · Work Max | Its page, terminal view |
| S3 | **Limit reached** | Need `limit` (`needs.rs:206`). **Exists.** | L2 with a fallback, L3 without | One per session. Rolls up per Account when there are 2 or more ("3 sessions on Work Max hit the limit"). | **chore/deps** hit Work Max's 5-hour limit. Resets at 3:13 AM. Personal has room. | The limit dialog (Continue in another app) |
| S4 | **Stopped with an error / crashed** | `instance_lifecycle` `ended: error\|crashed` (`model.rs:434-513`, with exit code or signal); need `failed`; notice `restarted` (`sources.rs:1166`). **Exists.** | L3, pinned until reviewed | One per session. An auto-restart merges in: "AI Juggler restarted it (attempt 1 of 3)". | **Port the settings parser** stopped with an error (exit code 1) after 42 min · Claude Code · Work Max. AI Juggler restarted it. | Its page at the end ("See what happened") |
| S5 | **Finished** | Own: notice `finished` (`sources.rs:998`) plus `instance_lifecycle ended: finished`. Other apps: conversation `state=done_review` after `last_activity_ms`. **Exists, but other apps' transitions are not persisted** (index is in memory). | L1 until reviewed, then L0 | (i) One session's lifecycle collapses into one card. (ii) The same App + project within 2 h rolls up: "finished 4 sessions in web-app". | Claude Code · Work Max finished **Fix the flaky login test** in 42 min · web-app · 3 files changed. ⟨quote: "Fixed the race in the login test by awaiting the session cookie; all 212 tests pass."⟩ | Its page. Inline: Mark reviewed, and ↗. |
| S6 | **Final answer excerpt** (part of S5) | `TextBlock.final_answer` via the bounded `last_messages` read (`conversations/last_messages.rs`). **Exists on demand**, gated by `show_last_messages`. | inherits | inside S5 | (the quoted line above) | Conversation view at that turn |
| S7 | **Started** | `instance_transitions` → `starting`; conversation `started_ms`. **Exists.** | L0 | Per App + project per hour: "started 3 sessions in web-app". Hidden when the same session already finished inside the window (S5 covers it). | Codex · Personal started 2 sessions in trip-notes | The App's page, filtered |
| S8 | **Stopped by you / archived / reviewed** | `instance_lifecycle` `ended: stopped by you`, `archived`, `reviewed`. **Exists.** | L0, **hidden by default** (your own actions aren't news) | Off unless the "Include my actions" filter is on | You stopped **Refactor the settings screen** | Its page |
| S9 | **Turn finished** (driven sessions) | `RuntimeEvent::TurnEnded{outcome,usage}` (`runtime/events.rs:308-422`); desktop `TurnOutcome::Completed\|ReviewReady` (`live.rs:33`). **Live only, not persisted (gap G4).** | L0 | Per session: a count of turns, shown on its S5 card ("in 6 turns") | — | — |
| S10 | **Subagents** | `subtask_count` in the index. **Exists (count only).** | L0 | Inside the S5 card ("with 3 helpers") | — | — |
| S11 | **Cut off: Resume it** | Attention rule `turn_cut_off`. **Exists.** | L2, pinned | One per session | **feat/read-model** was cut off when AI Juggler stopped it. Resume it to carry on. | Its page, Resume |
| S12 | **Left uncommitted files** | `git_dirty_count` at the end (live read, `read_model.rs:522`). **Live; record it at the end (gap G7).** | L1, inside S5 | inside S5 | … · 3 files not committed | Its page, Git |

## 2. Switches, moves and limits

| # | Item | Source | Level | Rollup | Example | Click |
|---|---|---|---|---|---|---|
| W1 | **Switch done** | `switch_edges` `state=Confirmed` (`model.rs:830`), with mechanism, from/to account and provider, and fidelity. **Exists, but only `created_at` is stored and the state is updated in place (gap G2: add `confirmed_at`).** | L1 | One per switch. A chain A→B→A within 1 h shows as one card with two steps. | AI Juggler moved **Answer the release question** from Claude Code · Work Max to Codex · Personal. History, tasks and 2 MCP servers carried. | The switch graph on its page |
| W2 | **Switch didn't carry** | Notice `failed` `switch-unconfirmed:{session}` (`handover/arrival.rs:116`); edge `Failed{stage,reason}`. **Exists.** | L2, pinned. The original is untouched, so it isn't L3. | One per switch | The switch to Codex didn't carry **Answer the release question**. The original is untouched and still open. | Its page ("Try again") |
| W3 | **Moved to another account** | `MoveRecordInfo` (`handover/moving.rs:130`), `at_ms`. **Exists** (file journal). | L1 | One per move | **Plan the trip notes app** moved to Personal. Undo is available for 10 min. | Its page |
| W4 | **Runway low** | Need `quota` with `meter` (`sources.rs:178`); alert FSM `TrayAmber`/`Actionable` (`alerts.rs:27`). **The need exists. Phase crossings are not persisted (gap G3).** | L1 at 11–25%; L2 at 10% or less with a live session | One per Account per window. It updates in place as it drops. | Work Max is at 12% of its 5-hour window and resets at 3:13 AM. 2 sessions are using it. | The Account page. Inline: Continue elsewhere. |
| W5 | **Out of room** | Alert `exhausted`; need `limit`. **Exists.** | L3 if a live session has no fallback; otherwise L1 | One per Account per window | Work Max is out of its 5-hour window until 3:13 AM. 1 session is waiting. | The Account page |
| W6 | **Room again (reset)** | Notice `reset` (`sources.rs:601`), with window and switch-back count. **Exists.** | L1 | One per Account per window | Personal has room again: its 5-hour window reset. 1 session can switch back. | Inline: Switch back |
| W7 | **Credits carry on** | Notice `quota` (credits variant, `sources.rs:242`). **Exists.** | L1 | One per window | Work Max used its 5-hour window. Credits carry on ($4.10 left). | The Account page |
| W8 | **Plan renews soon** | Notice `renewal` (`sources.rs:707`). **Exists.** | L1 | One per Account per day | Personal renews in 5 days | The Account page |
| W9 | **Monthly cap near or over** | Need `cap` (`needs.rs:571`), `BudgetInfo`. **Exists.** | L2 | One per cap | Work Max has spent $82 of its $100 monthly cap | Budget settings |
| W10 | **Daily usage digest** | `usage_events` and `quota_samples`. **The data exists, but no digest is produced (gap G6).** | L0 | One per day, at the day boundary | Yesterday: 1.4M tokens across 3 accounts and 18 sessions. web-app was the busiest. | Usage, for that day |

## 3. Pools and tasks

| # | Item | Source | Level | Rollup | Example | Click |
|---|---|---|---|---|---|---|
| P1 | **Task done** | `task_events` `verb=status` with detail "… → Done" (`storage/tasks.rs:915`), plus `closed_at`. A `noted` or `released` detail carries a free note. **Exists, but there is no work summary (gap G5).** | L0 (L1 if a review is wanted) | Per pool per hour: "Release pool finished 3 tasks" | builder-2 finished **Wire the export button** in the Release pool. Note: "CSV and JSON both covered." | The task |
| P2 | **Task needs review / blocked** | `task_events` status → `InReview` or `Blocked`. **Exists.** | L1 | Per pool per hour | **Export flow** is waiting for review in the Release pool | The task |
| P3 | **Task needs you** (held change) | `task_needs` with `answered_at` null; need `task` (`needs.rs:619`). **Exists.** | L2, pinned | One per task | The Release pool wants to mark **Ship 0.5 notes** done. Approve? | The task (Approve / Refuse) |
| P4 | **Pool conversation highlight** | `agent_messages`, kinds `Handoff`, `ReviewRequest`, `Question`, `Status` (`messaging.rs:21`), with full bodies. **Exists. There is no "highlight" concept; this proposes the kinds-based rule.** | L0 (L1 for a review request) | Per thread per 30 min: "4 messages in *Export flow*". `Note` chatter is never shown, only counted. | reviewer → builder-1 in *Export flow*: "The migration misses its down step; adding it before merge." | The thread at that message |
| P5 | **Message waits for you** | Need `message` (sub-kinds `deliver`, `ask`, `question`, `waiting`, `pool_paused`, `thread_paused`; `messages/needs.rs`). **Exists.** | L2, pinned | One per thread | builder-1 asks you in *Export flow*: "Use the v2 API or keep v1?" | The thread, reply box |
| P6 | **Pool paused** | `pool_pauses`; need `message` with `pool_paused`. **Exists.** | L2 | One per pool | The Release pool paused after 6 agent messages in a row without you | The pool |
| P7 | **Agent started work on a task** | `task_events` `claimed`. **Exists.** | L0 | Rolled into P1 for the same task | — | — |

## 4. Git and PRs

| # | Item | Source | Level | Rollup | Example | Click |
|---|---|---|---|---|---|---|
| G-a | **Commits made** | **Gap G1.** Only `git status` facts exist (branch, dirty, ahead/behind; `worktree.rs`). Proposal: when a session ends, run `git log --since=<start>` read-only in its folder. It is local and cheap. | L0 | Per session (into S5) or per project per hour | 3 commits on feat/read-model in web-app | The project page, Git |
| G-b | **PR opened / merged** | **Gap G1.** `gh` is used only on click, for linked issues (`tasks/issues.rs`). Proposal: if `gh` is installed and signed in, read `gh pr list --author @me --json …` for the session's repo when it ends. That is the user's own tool, read-only, opt-in through the existing Git setting. Undocumented route to check: the Claude app's "Auto-fix on pull request #N" automation (`claude_app_automation.rs:163`). | L1 | One per PR, with later merges updating it in place | Claude Code · Work Max opened PR #142 "Add the export button" in web-app | The PR on GitHub (↗), and the session |

## 5. This Mac and AI Juggler itself

| # | Item | Source | Level | Rollup | Example | Click |
|---|---|---|---|---|---|---|
| M1 | **Mac under pressure** (live) | Need `room` / `heavy` (`needs.rs:488`, `:517`); `OverloadAlertInfo` (`machine_alerts.rs:23`). **Exists.** | L2 (L3 at critical), pinned | One per reason | This Mac is under memory pressure: **Nightly tests** is using 9 GB | This Mac |
| M2 | **Pressure ended** | Notice `this-mac` ("It lasted N minutes", `sources.rs:1118`). **Exists.** | L0 | One per episode | This Mac was under memory pressure for 12 min. It ended at 2:10 PM. | This Mac |
| M3 | **Processes left running** | Need `leftovers` (`needs.rs:542`). **Exists.** | L1 (L2 when heavy) | One open item, updated | 7 processes are still running after their sessions ended. They hold 1.2 GB. | Cleanup |
| M4 | **App restarted by keep-alive** | Notice `app-started` (`sources.rs:1036`). **Exists.** | L1 | Per App per day | AI Juggler started Claude app · Personal again after it quit | The App's page |
| M5 | **Keep-alive paused** | Notice `keep-alive-paused` (`sources.rs:1077`). **Exists.** | L2 | One per App | Keep alive paused for Claude app · Lab: it quit 3 times in 10 min | The App's page |
| M6 | **Sign-in needed** | Need `signin` (`needs.rs:467`). **Exists.** | L2, pinned | One per Account | Personal needs you to sign in again before its sessions can start | Sign-in |
| M7 | **Update available / installed** | Notice `update` (`sources.rs:1248`). **Exists.** | L1, then L0 | One per version | AI Juggler 0.5.1 is ready. Restart to update. | Updates |
| M8 | **App found running, not added** | Notice `found` (`sources.rs:731`). **Exists.** | L1 (an offer only, D-84) | One per App | Cursor is running but isn't added. Add it? | The Add flow |

## 6. Digest items (made by the feed, not by a source)

| # | Item | Built from | Level | Rule | Example | Click |
|---|---|---|---|---|---|---|
| D1 | **While you were away** | Everything between `summary_seen_at_ms` (the existing key, `overnight.rs:22`) and now, when the gap is 2 h or more | L0 card; any L3 inside stays pinned | One card at the boundary: counts by kind and the top 3 items, expandable in place. Replaces MorningSummary's card. | While you were away (since 9:12 PM): 6 finished, 1 crashed, 2 switches. Work Max reset at 3:13 AM. | Expands in place. "Mark all seen" sets the key. |
| D2 | **Day separator** | Day change | — | "Today", "Yesterday", then the date | — | — |

## 7. Gaps to fill (backend), smallest first

| Gap | What | Size | Where |
|---|---|---|---|
| **G2** | Add `state_changed_at` / `confirmed_at` to `switch_edges`, keeping it append-only, as an update column only | S (migration + 3 call sites `switch_graph.rs:327/351/395`) | juggler-core |
| **G3** | Persist alert FSM phase changes (`TrayAmber`, `Actionable`, `Exhausted`, `ResetReached`) as `notices` Earlier rows, kind `runway` | S | `notifications/sources.rs` `quota_pass` |
| **G7** | Record `git_dirty_count` and the edited-file count when a session ends, on its `ended:` lifecycle event | S | `read_model.rs`, `model.rs` |
| **G4** | Persist other apps' conversation state changes (`asks_you`, `working`, `idle`, `done_review`) and driven-session `TurnEnded` as `conversation_events(conv_id, from, to, at_ms)`, retention 30 days like notices | M | conversations feature |
| **G6** | Daily usage digest from `usage_events` (one row per day, computed lazily at read time) | S | `usage_history` |
| **G5** | A task close note: the pool agents' MCP tool gains `close(summary)` into `task_events.detail` | S–M | tasks, MCP |
| **G1** | Commits (`git log` read-only at session end) and PRs (`gh pr list` if available and opted in) as `notices` Earlier kinds `commits` and `pr` | M | a new `git_activity` module behind the existing worktree reader. It is adapter-agnostic: any tool, any folder. |
| **Read model** | `feed_page(cursor, filters)`: one keyset query that unions `notices`, `instance_lifecycle` (`ended:`), `switch_edges`, `task_events` (done, review), `agent_messages` (highlight kinds), and the new rows above, then applies the rollup keys | M | a new `features/feed/` module. The UI renders it; templates live in one `feed-words.ts`. |
