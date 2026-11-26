# ADR-0039: One scheduler: durable, typed, visible triggers for everything time-based

**Status:** PROPOSED (2026-11-26). For the maintainer's review before anything is built.
**Date:** 2026-11-26
**Deciders:** the maintainer (the scope and the trigger kinds); Opus (design)
**Asked for by:** the maintainer, 2026-11-26, on Home feed's "Remind me": "Make sure you build for future, a scheduled and periodic triggers and that kind of capability will be needed generally for all kinds of reasons, so make sure you build something that can serve many purposes."
**Serves:**
- [ADR-0019](./0019-extension-architecture.md): modules, typed handlers;
- [ADR-0030](./0030-storage-concurrency.md): one writer, short holds;
- [ADR-0036](./0036-digest-and-watches.md): the digest window and watches;
- [ADR-0037](./0037-memory-and-context.md): the append-only log and the maintenance batches;
- [ADR-0038](./0038-world-model-engine.md): gyrus `dream`;
- D-52: OS seams;
- PAT-OSP: no OS prompt unannounced;
- the architecture standard: no duplication;
- always-on: the app starts at login and keeps running.

## Context

Time-based work is scheduled six different ways today, each with its own thread, clock and missed-run behaviour:

| Today | How it runs | What it loses |
|---|---|---|
| Cloud backup (`cloud_backup.rs` `spawn_schedule`) | a bare thread: `sleep(3600)`, then run | runs counted from launch, not the wall clock; no missed-run rule; sleep pushes it later |
| The digest (`features/digest/scheduler.rs`) | event-driven, pure, `due_at` on the caller's clock, driven by the watches runtime | correct, but its own clock and its own "due" bookkeeping |
| Watches (`features/watches/runtime.rs`) | its own thread with a deadline heap (`IdleFor`, after-minutes conditions) | deadlines kept in memory: lost across a restart |
| The cleanup sweep (`features/cleanup/sweep.rs`) | called from the processes tick (`processes/tick.rs`) | a durable "last swept" exists only implicitly |
| ADR-0037's maintenance (§2.3) and gyrus `dream` (ADR-0038 §4) | "bounded idle batches", described, with no shared trigger | each would invent "idle" again |
| Feed "Remind me", Bot "remind me at…" | not possible: watches have no time trigger | — |

The roadmap adds more: scheduled Bot prompts and agent tasks (0.6), the Claude version compat check, the remote-config refresh, usage-window resets, and other tools' own scheduled tasks shown to the user (the ZCode lesson, 2026-11-24).

**What this ADR is not about:** `ticks.rs`'s sub-minute observation loops (keep-alive 2 s, automation 5 s, the read model, the process graph). Those watch live state and are not schedules: nothing is "due" and nothing is missed, so they stay as ticks. The line between them is in §1.

## Decision

### 1. One scheduler service, for every durable or wall-clock trigger

`features/schedule/` in juggler-app owns time. The trigger maths (next due, missed runs, DST) is a pure module with no I/O, so it can be tested exhaustively.

A job belongs to the scheduler when any of these is true:
- it must happen at a wall-clock time;
- its period is a minute or longer and a missed run matters;
- it must survive a restart;
- the user can see or change it.

Anything else stays a `ticks.rs` loop, for observation polling under a minute.

### 2. Trigger kinds (a closed set, typed)

| Kind | Meaning | Examples |
|---|---|---|
| `At { at_ms }` | once, at an instant | Remind me at 3 pm; a reminder from Bot |
| `Every { interval_ms, anchor }` | every interval, anchored to the wall clock (`anchor`), not to launch | cloud backup hourly; the compat check daily |
| `Calendar { rule }` | a small RRULE subset: `daily at HH:MM`, `weekdays at HH:MM`, `weekly on <days> at HH:MM`, `monthly on <day> at HH:MM`, in the user's local time zone | "every weekday at 9"; a scheduled digest |
| `AfterEvent { event, delay_ms, unless }` | `delay` after a typed event (the event bus's kinds), cancelled if `unless` happens first | "if the session isn't finished 30 min after it started"; ADR-0036's after-minutes conditions |
| `WhenIdle { max_wait_ms }` | at the next idle moment, or at `max_wait` | ADR-0037's maintenance; gyrus `dream`; index rebuilds |

**Shared modifiers:**
- `window`: only between `HH:MM` and `HH:MM` local; quiet hours reuse the user's quiet-hours setting by reference.
- `jitter_ms`: a random delay up to N, so periodic jobs don't all fire on the same second.
- `lifetime`: `once`, `until { at_ms }` or `always`, as ADR-0036's lifetimes.

**Not raw cron:** the `Calendar` subset covers what people say, and each rule has a plain-words rendering ("Weekdays at 9:00 AM").

**Idle** has one definition: no AI job running or queued (the router's queue), and the Mac neither in Low Power Mode nor showing the low-battery warning (`machine_health`'s `low_power_mode` and `battery_low`). A handler may also require mains power (`on_battery` false).

### 3. Durable: schedules live in SQLite

- **One table**, `schedules`, in the next migration (the next V after every branch is checked). Each row holds:
  - `id`, `owner` (a module id), `handler` (a registered handler id) and `payload` (typed JSON, validated by the handler);
  - `trigger` and `modifiers` (typed JSON);
  - `next_due_ms` (computed and persisted), `last_run_ms`, `last_outcome`;
  - `missed_policy`, `paused`;
  - `user_visible`, `created_by` (`user` / `app` / `bot`, `bot` reserved for 0.6);
  - `created_ms`, `title` (plain words, for the list).
- **Missed runs:** each schedule picks a policy:
  - `RunOnce` (default): one run on wake or launch, however many were missed;
  - `Skip`: move on to the next due time;
  - `RunEach { cap }`: run each missed occurrence, up to `cap` (for things like a usage sample that must not lose a slot).
- **Recompute** `next_due_ms` for every schedule on launch, on wake from sleep, and on a wall-clock or time-zone change.
  - These arrive through an OS seam (D-52). `juggler-platform::clock`, today a time formatter, gains a change observer: on macOS, `NSWorkspaceDidWakeNotification` and `NSSystemClockDidChangeNotification` / `NSSystemTimeZoneDidChangeNotification`. Where a system can't tell, the next tick compares the wall clock with the monotonic clock: a jump beyond a minute is a clock change or a wake.
- **The closed-app case** is covered by "missed run on launch". Nothing runs while AI Juggler is closed (§9).

### 4. Typed handlers, never code

- **Registration:** a module registers `ScheduleHandler`s at start (ADR-0019): `id`, a `payload` type with a validator, `run(payload, ctx) -> Outcome`, a `timeout`, a default `priority`, and whether user-made schedules may use it.
- **Only registered handlers run.** A schedule names a handler by id and carries data, never code.
- **Agent text can't create or edit a schedule.** User schedules come from the UI or from Bot's typed draft-and-confirm (the watches pattern: the model fills a constrained form, code validates, the user clicks).
- **Ownership:** each schedule has an owner module, which alone may change its app-made schedules. The user may pause, edit or delete their own.

### 5. Run controls

- **One run at a time per schedule.** A schedule still running when it comes due again is skipped, and the skip is logged.
- **A `timeout` per handler.** A run past it is abandoned and counted as a failure.
- **Retry with backoff** for failures a handler marks retryable (×2 up to a cap, then the next regular due time).
- **Priority:** user-visible reminders run first, then app jobs, then idle work. At most two runs at once, on a small worker pool at the lowest QoS, as `ticks.rs` does.
- **Writer holds stay short** (ADR-0030): a handler takes the writer lock only for its write, never for its whole run.

### 6. Visible

- **One list for user-made schedules:** reminders, timed watches, a scheduled digest. Each row shows its plain-words trigger, the next time it's due, its last outcome, and Edit, Pause and Delete. The list also has Pause all.
  - It lives beside Watches (both are "things AI Juggler will do for me later"), and Settings links to it.
- **Settings › Advanced** shows the app's own schedules read-only: what, when next, last run, with the same plain words.
- **Built for other tools' schedules:** each row has a `source` (AI Juggler, or later another tool: ZCode's scheduled tasks, a Claude app routine), so other tools' schedules can be surfaced read-only with "Open in <tool>". 0.5.2 builds only AI Juggler's.

### 7. Logged

Every run, skip, miss, retry and failure is appended to ADR-0037's append-only log (stream `schedule`): the schedule id, the handler, the outcome, and the duration. A missed run carries the policy that applied.

### 8. Delivery

- **Reminders reach the user** through the existing bell and system notifications (the notifier's sink, AJ-E2904).
- **No OS prompt arrives unannounced:** the first time a user creates a reminder while Notifications aren't allowed yet, the creation flow primes the Notifications prompt (PAT-OSP, AJ-E1547's gate) before the schedule is saved. Nothing asks at fire time.

### 9. Out of scope

- Nothing runs outside the running app: no launchd agents, no Background Task items, no helper that wakes the Mac.
- The app starts at login and stays running (always-on). A schedule that falls due while it's closed is handled by its missed-run policy on the next launch.
- No wake-from-sleep timers. Recompute on wake covers what came due during sleep.

### 10. Time zones and DST

`Calendar` and `window` need the user's local time zone, with correct DST transitions for future dates. The workspace today compiles `time` 0.3 (an offset only, no zone rules). `jiff` 0.2.38 and `chrono` are in Cargo.lock (through `serde_with`'s optional features) but aren't compiled.

**The question for the maintainer:**
- **(a)** Use `jiff`, adding it as a direct dependency of juggler-app with its system-tzdb feature. Cargo.lock doesn't change: the crate and its tzdb crates are already locked. It's maintained, correct across DST, and reads the system's zone database on macOS and Linux, with a bundled one on Windows.
- **(b)** A platform seam over libc's `localtime_r`/`mktime` (`tm_isdst = -1`). That would be juggler-platform's third audited `unsafe` call, needing a D-number like D-146.
- **(c)** Parse the system's TZif files ourselves: no `unsafe`, but a parser to maintain, and a different seam on Windows.

**My recommendation is (a):** correctness across DST is exactly the part not to hand-write, and it changes nothing in Cargo.lock.

**DST rules:**
- A wall time that doesn't exist (the spring gap) runs at the first valid time after it.
- A wall time that happens twice (the autumn overlap) runs once, at the first.
- `Every` is in absolute time; `Calendar` follows wall time.

### 11. Consumers

**0.5.2, built:**
1. `RemindAt` (`At` + the reminder handler): the feed's Remind me, and Bot's "remind me at…" through the same typed draft and confirm as watches.
2. ADR-0037's maintenance batches (`WhenIdle`).
3. gyrus `dream` (`WhenIdle`).
4. The digest window: the digest's pure `due_at` is unchanged; it publishes its next due time as an `At` the scheduler fires, so the digest no longer needs its own clock.

**0.5.2, migrated one per commit with behaviour-unchanged tests:**
1. Cloud backup: `Every { 1 h, anchored }`, `RunOnce`.
2. The cleanup sweep, moved off the processes tick: `Every`, `Skip`.
3. Watches' time-based conditions (`IdleFor`, after-minutes): `AfterEvent`, durable across restarts.
4. The bell's timed re-ring, if any.

Any migration that looks risky is listed with the reason, not forced.

**Designed, not built:** scheduled Bot prompts and agent tasks (0.6, through the bot's approval rules, `created_by: bot`); the Claude version compat check; the remote-config refresh; usage-window resets; other tools' schedules surfaced read-only.

## Alternatives considered

- **A watches-only time condition** (the first plan for Remind me): rejected by the maintainer's ruling. It would be the seventh timer.
- **launchd / Background Tasks:** it runs while the app is closed, but means a second process, entitlements, and OS prompts for behaviour users don't expect. Always-on plus missed-run-on-launch covers the need.
- **Raw cron strings:** too powerful for users, hard to render in plain words, and DST-ambiguous.
- **Keeping each timer and sharing only a helper:** keeps six missed-run behaviours and nothing durable.

## Consequences

- One place answers "what will AI Juggler do, and when?", for the user and for us.
- Reminders, timed watches and maintenance survive restarts and sleep.
- Every time-based feature after this is a handler plus a trigger: no new thread and no new clock.
- The migration touches working code (backup, sweep, watches). Each step lands alone with a behaviour-unchanged test, and risky ones are listed, not forced.

## Order of work (after acceptance)

1. The pure trigger maths: next due, missed runs, DST, windows, jitter. Property tests, including the spring gap and autumn overlap.
2. The `schedules` table, the service, the handler registry, run controls, the log stream, and recompute on launch, wake and clock change (the OS seam).
3. The reminder handler, with the feed's Remind me, Bot's "remind me at…" and the Notifications priming. The user list, with Pause all.
4. `WhenIdle` consumers: ADR-0037's maintenance and gyrus `dream`. The digest window through `At`.
5. Migrations, one per commit: cloud backup, the cleanup sweep, watches' time conditions, the bell.
6. Settings › Advanced's read-only list. A Sonnet walk, an Opus critique, and the real-app check: a reminder fires after a restart and after sleep.
