# Evidence manifest: AJ-E2027 (one scheduler for everything time-based, ADR-0039, D-151)

Built 2026-11-26 on release/0.5.2 (Opus). `jiff` 0.2.38 is a direct dependency of juggler-app
(features `std`, `tz-system`, `tzdb-zoneinfo`, `tzdb-bundle-platform`); Cargo.lock gained only
juggler-app's `"jiff"` name line (D-151's amendment), nothing new locked or fetched, built
`--offline`. Migration V57 (`schedules`), the next after V56, checked on every local and remote
branch before the commit: none claims 57. No other new crate.

R1: PASS — commit 7c9e7a874. `features/schedule/trigger.rs` (pure): At, Every (anchored to the wall
clock), Calendar (daily, weekdays, weekly, monthly; a missing day is the month's last), AfterEvent,
WhenIdle; window (quiet hours by reference), stable jitter, lifetimes; RunOnce, Skip, RunEach.
`trigger_tests.rs`: 31 tests, 6 properties × 400 cases over 7 zones (New York, Berlin, Sydney,
Lord Howe's half-hour change, Kolkata, São Paulo, UTC) — the spring gap runs at the gap's end
(`the_spring_gap_runs_at_the_first_valid_time_after_it`, `a_half_hour_gap_runs_at_its_end_too`), the
autumn overlap once at the first (`the_autumn_overlap_runs_once_at_the_first`,
`a_daily_rule_runs_once_a_day_through_every_change`), windows and jitter
(`the_due_time_respects_jitter_and_window`), every policy (`missed_runs_follow_the_policy`,
`a_calendar_schedule_counts_what_it_missed`).
R2: PASS — commit 8ccafb336. V57 (`juggler-core` `storage/schedules.rs`), `store.rs` (every change
checked against its handler, logged), `registry.rs` (only registered typed handlers; a payload
validated before save and before each run), `service.rs` (one run at a time per schedule, a timeout
per handler, retry ×2 from a minute up to five times, reminders > app > idle on at most two
lowest-priority runs, writer holds per read/write), the `schedule` log stream, recompute on launch,
wake and clock/zone change (`juggler-window-events::watch_time_changes`, audits 5 and 6 extended;
`juggler-platform::clock::JumpCheck` everywhere as the fallback; the zone read fresh,
`zone.rs`). Tests: `service_tests.rs` (15), `clock::tests::a_jump_between_the_clocks_is_a_wake_or_a_clock_change`,
`zone`, `idle`, `registry`; juggler-core `busy_timeout` (V57 undone and re-run), `tasks_storage`.
R3: PASS — commit 991d8a385. `reminder.rs` (the handler; data is only what it is about),
`watch_alerts::reminded` (told once per reminder and due time, through the bell and a notification;
late ones say so), `commands.rs` (`schedule_remind/list/summary/update/pause/pause_all/remove`),
`remind_draft.rs` (Bot's "remind me at…" read by code, no model), the feed's Remind me
(`lib/feed-watch.ts`), Bot's card (`BotReminderDraft.tsx`), the Reminders list beside the watches
(`RemindersSection.tsx`, Pause all). Notifications primed at creation only (registry row in
`docs/design/os-prompts.md`). Tests: `commands_tests.rs` (3), `remind_draft` (3), `reminder` (1);
vitest `reminders.test.tsx` (6), `home-feed-story.test.tsx` (Remind me), `system-notifications.test.tsx`.
PAT-USE's six answers: the task's Usability reasoning.
R4: PASS — commit af0215bac. `upkeep.rs` `Maintain` (WhenIdle: ADR-0037's maintenance with gyrus's
idle pass in `world::maintain::pass`; held the first two minutes after launch); the digest publishes
its due time as a transient At (`digest/runtime.rs` `publish`, `due_now`). Tests:
`upkeep::tests::maintenance_waits_for_idle_an_hour_after_its_last_run`,
`service_tests::idle_work_waits_for_idle_or_its_deadline`,
`digest::runtime::tests::the_digest_has_no_clock_of_its_own_its_due_time_is_the_schedulers`.
R5: PASS (backup), listed (the rest) — commit 0954d3602: Cloud backup's thread is `Every` hour on the
wall clock, RunOnce (`upkeep::tests::the_backup_check_is_hourly_on_the_wall_clock_not_from_launch`,
`…does_what_the_thread_did_nothing_while_cloud_backup_is_off`; `tests/cloud_backup.rs` unchanged).
Listed, not forced (ADR-0039 §11):
- the cleanup sweep: its cadence is the processes tick's 2 s with a 60 s grace measured on it, and it
  is a kill surface; a one-minute-or-longer `Every` would delay automatic cleanup by up to a minute
  and shift Tier 2's pacing; by §1 it is an observation loop, so it stays a tick;
- watches' time conditions (IdleFor, after-minutes): their deadlines live in the pure engine's slot
  state with the each/all `met` flags and memos; making only the deadlines durable would fire an
  "all" watch on half its state after a restart; it needs the engine's slot state stored as a unit
  (a design step);
- the bell: it has no timed re-ring of its own; a snooze ends on the needs tick, which also rereads
  the needs, so an `At` for the end alone would change nothing the user sees.
R6: PASS (the list) — commit 70a2a6b3d: Settings › Advanced › Scheduled work, read-only
(`ScheduledWorkSection.tsx`, `scheduled-work.test.tsx`, Settings' search). Left: the Sonnet walk, the
gallery re-capture with an Opus critique in both themes and the stamp, the re-rendered baselines
(68-home-feed shows Remind me now; Settings › Advanced and › Notifications changed), and the real-app
check (a reminder fires after a restart and after sleep).

| Requirement | Verdict | Proof |
|---|---|---|
| R1 trigger maths | PASS | 7c9e7a874, `trigger_tests.rs` (31 tests, 6 properties) |
| R2 table, service, registry, run controls, log, recompute | PASS | 8ccafb336, `service_tests.rs`, `clock` jump test |
| R3 reminders: feed, Bot, priming, the list | PASS | 991d8a385, `commands_tests.rs`, `reminders.test.tsx` |
| R4 WhenIdle maintenance, the digest through At | PASS | af0215bac, upkeep and digest tests |
| R5 backup migrated; sweep, watch deadlines, bell listed | PASS | 0954d3602, upkeep backup tests; reasons above |
| R6 Settings › Advanced's list | PASS | 70a2a6b3d, `scheduled-work.test.tsx` |
| R6 Sonnet walk, Opus critique, gallery and baselines | UNPROVEN | not run in this build session |
| R6 real-app check: a reminder after a restart and after sleep | UNPROVEN | the walk's (needs the built app and a sleep) |
