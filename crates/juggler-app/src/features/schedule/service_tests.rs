#![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]
//! AJ-E2027 R2: the table, the handler registry, the run controls, the
//! log stream and the recompute, over a real database in memory.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use jiff::tz::TimeZone;
use juggler_core::storage::Storage;
use juggler_core::storage::memory::Stream;
use serde_json::{Value, json};

use super::*;
use crate::features::schedule::registry::{Outcome, Priority, RunCtx, ScheduleHandler};
use crate::features::schedule::store::{self, CreatedBy, NewSchedule, ScheduleError};
use crate::features::schedule::trigger::{
    CalendarRule, ClockTime, Context, MissedPolicy, Modifiers, Spec, Trigger,
};

const NOW: i64 = 1_800_000_000_000;

struct Fake {
    id: &'static str,
    priority: Priority,
    user_made: bool,
}

impl ScheduleHandler for Fake {
    fn id(&self) -> &'static str {
        self.id
    }
    fn owner(&self) -> &'static str {
        "test"
    }
    fn timeout(&self) -> Duration {
        Duration::from_secs(5)
    }
    fn priority(&self) -> Priority {
        self.priority
    }
    fn user_made(&self) -> bool {
        self.user_made
    }
    fn validate(&self, payload: &Value) -> Result<(), &'static str> {
        if payload.get("bad").is_some() {
            Err("bad")
        } else {
            Ok(())
        }
    }
    fn run(&self, _payload: &Value, _ctx: &RunCtx<'_>) -> Outcome {
        Outcome::Done
    }
}

fn registry() -> Registry {
    let mut r = Registry::default();
    for (id, priority, user_made) in [
        ("remind", Priority::User, true),
        ("job", Priority::App, false),
        ("idle_job", Priority::Idle, false),
    ] {
        r.register(Arc::new(Fake {
            id,
            priority,
            user_made,
        }))
        .unwrap();
    }
    r
}

fn storage() -> Storage {
    Storage::open_in_memory().unwrap()
}

fn utc() -> Context {
    Context::in_zone(TimeZone::UTC)
}

fn new(
    handler: &'static str,
    trigger: Trigger,
    modifiers: Modifiers,
    by: CreatedBy,
) -> NewSchedule {
    NewSchedule {
        owner: "test",
        handler,
        payload: json!({}),
        spec: Spec {
            trigger,
            modifiers,
            missed: MissedPolicy::RunOnce,
        },
        user_visible: by != CreatedBy::App,
        created_by: by,
        title: "A test".to_owned(),
    }
}

fn reminder_at(s: &Storage, r: &Registry, at_ms: i64) -> store::Schedule {
    store::create(
        s,
        r,
        new(
            "remind",
            Trigger::At { at_ms },
            Modifiers::once(),
            CreatedBy::User,
        ),
        &utc(),
        NOW,
    )
    .unwrap()
}

fn verbs(s: &Storage, id: &str) -> Vec<String> {
    let mut events = s.ai_log().of_subject(store::SUBJECT, id, 50).unwrap();
    events.reverse();
    assert!(events.iter().all(|e| e.stream == Stream::Schedule.as_str()));
    events.into_iter().map(|e| e.verb).collect()
}

#[test]
fn a_reminder_is_stored_with_its_due_time_runs_once_and_ends() {
    let (s, r) = (storage(), registry());
    let made = reminder_at(&s, &r, NOW + 3_600_000);
    let row = s.schedules().get(&made.row.id).unwrap().unwrap();
    assert_eq!(row.next_due_ms, Some(NOW + 3_600_000));
    assert_eq!(s.schedules().soonest().unwrap(), Some(NOW + 3_600_000));
    let none = HashSet::new();
    assert!(take_due(&s, &r, &utc(), NOW + 1_000, Some(true), &none, 2).is_empty());
    let taken = take_due(&s, &r, &utc(), NOW + 3_600_500, Some(true), &none, 2);
    assert_eq!(taken.len(), 1);
    assert!(!taken[0].late && taken[0].ends);
    // Handed out: not due again while it runs.
    assert!(take_due(&s, &r, &utc(), NOW + 3_601_000, Some(true), &none, 2).is_empty());
    finish(
        &s,
        &taken[0],
        Ended::Returned(Outcome::Done),
        NOW + 3_600_600,
        12,
    );
    let row = s.schedules().get(&made.row.id).unwrap().unwrap();
    assert_eq!(row.last_outcome.as_deref(), Some("ok"));
    assert_eq!(row.ended_ms, Some(NOW + 3_600_600));
    assert_eq!(row.next_due_ms, None);
    assert_eq!(verbs(&s, &made.row.id), ["created", "ran"]);
    let ran = &s
        .ai_log()
        .of_subject(store::SUBJECT, &made.row.id, 1)
        .unwrap()[0];
    assert_eq!(ran.payload["duration_ms"], 12);
    assert_eq!(ran.payload["handler"], "remind");
}

#[test]
fn a_reminder_that_came_due_while_closed_runs_late_on_launch() {
    let (s, r) = (storage(), registry());
    let made = reminder_at(&s, &r, NOW + 60_000);
    // A day later: the launch recomputes, then it's due, late.
    recompute(&s, &utc(), "launch", NOW + 86_400_000);
    let taken = take_due(
        &s,
        &r,
        &utc(),
        NOW + 86_400_000,
        Some(true),
        &HashSet::new(),
        2,
    );
    assert_eq!(taken.len(), 1);
    assert!(taken[0].late);
    assert_eq!(taken[0].schedule.row.id, made.row.id);
}

#[test]
fn one_run_at_a_time_a_second_due_while_running_is_skipped_and_logged() {
    let (s, r) = (storage(), registry());
    let every = store::create(
        &s,
        &r,
        new(
            "job",
            Trigger::Every {
                interval_ms: 60_000,
                anchor_ms: 0,
            },
            Modifiers::always(),
            CreatedBy::App,
        ),
        &utc(),
        NOW,
    )
    .unwrap();
    let running: HashSet<String> = [every.row.id.clone()].into();
    assert!(take_due(&s, &r, &utc(), NOW + 61_000, Some(true), &running, 2).is_empty());
    assert_eq!(verbs(&s, &every.row.id), ["created", "skipped"]);
    // Moved on to the next minute.
    let row = s.schedules().get(&every.row.id).unwrap().unwrap();
    assert!(row.next_due_ms.unwrap() > NOW + 61_000);
}

#[test]
fn a_retryable_failure_backs_off_then_a_success_clears_it() {
    let (s, r) = (storage(), registry());
    let made = store::create(
        &s,
        &r,
        new(
            "job",
            Trigger::Every {
                interval_ms: 86_400_000,
                anchor_ms: 0,
            },
            Modifiers::always(),
            CreatedBy::App,
        ),
        &utc(),
        NOW,
    )
    .unwrap();
    let due = made.row.next_due_ms.unwrap();
    let none = HashSet::new();
    let first = take_due(&s, &r, &utc(), due, Some(true), &none, 2)
        .pop()
        .unwrap();
    let fail = Ended::Returned(Outcome::Failed {
        retry: true,
        code: "offline",
    });
    finish(&s, &first, fail, due, 5);
    let row = s.schedules().get(&made.row.id).unwrap().unwrap();
    assert_eq!((row.retries, row.next_due_ms), (1, Some(due + 60_000)));
    let second = take_due(&s, &r, &utc(), due + 60_000, Some(true), &none, 2)
        .pop()
        .unwrap();
    assert_eq!(second.retries, 1);
    finish(&s, &second, fail, due + 60_000, 5);
    assert_eq!(
        s.schedules()
            .get(&made.row.id)
            .unwrap()
            .unwrap()
            .next_due_ms,
        Some(due + 60_000 + 120_000),
        "doubled"
    );
    let third = take_due(&s, &r, &utc(), due + 180_000, Some(true), &none, 2)
        .pop()
        .unwrap();
    finish(&s, &third, Ended::Returned(Outcome::Done), due + 180_000, 5);
    let row = s.schedules().get(&made.row.id).unwrap().unwrap();
    assert_eq!((row.retries, row.next_due_ms), (0, Some(due + 86_400_000)));
    assert_eq!(
        verbs(&s, &made.row.id),
        ["created", "retried", "retried", "ran"]
    );
    assert_eq!(retry_at(0, 30), RETRY_CAP_MS, "capped");
}

#[test]
fn a_run_past_its_timeout_counts_as_a_failure() {
    let (s, r) = (storage(), registry());
    let made = reminder_at(&s, &r, NOW + 1_000);
    let d = take_due(&s, &r, &utc(), NOW + 1_000, Some(true), &HashSet::new(), 2)
        .pop()
        .unwrap();
    finish(&s, &d, Ended::TimedOut, NOW + 1_000, 5_000);
    let row = s.schedules().get(&made.row.id).unwrap().unwrap();
    assert_eq!(row.last_outcome.as_deref(), Some("timeout"));
    assert_eq!(verbs(&s, &made.row.id), ["created", "timeout"]);
}

#[test]
fn reminders_go_first_and_at_most_the_room_is_taken() {
    let (s, r) = (storage(), registry());
    let mut every = new(
        "job",
        Trigger::Every {
            interval_ms: 60_000,
            anchor_ms: 0,
        },
        Modifiers::always(),
        CreatedBy::App,
    );
    every.spec.trigger = Trigger::Every {
        interval_ms: 60_000,
        anchor_ms: NOW,
    };
    store::create(&s, &r, every, &utc(), NOW).unwrap();
    let reminder = reminder_at(&s, &r, NOW + 90_000);
    let taken = take_due(
        &s,
        &r,
        &utc(),
        NOW + 120_000,
        Some(true),
        &HashSet::new(),
        1,
    );
    assert_eq!(taken.len(), 1);
    assert_eq!(
        taken[0].schedule.row.id, reminder.row.id,
        "the reminder first"
    );
}

#[test]
fn idle_work_waits_for_idle_or_its_deadline() {
    let (s, r) = (storage(), registry());
    let made = store::create(
        &s,
        &r,
        new(
            "idle_job",
            Trigger::WhenIdle {
                max_wait_ms: 600_000,
                after_ms: 3_600_000,
            },
            Modifiers::always(),
            CreatedBy::App,
        ),
        &utc(),
        NOW,
    )
    .unwrap();
    let none = HashSet::new();
    let eligible = NOW + 3_600_000;
    assert!(
        take_due(&s, &r, &utc(), eligible + 1, Some(false), &none, 2).is_empty(),
        "busy"
    );
    assert!(
        take_due(&s, &r, &utc(), eligible + 600_000, None, &none, 2).is_empty(),
        "just launched: idle work waits"
    );
    let at_deadline = take_due(&s, &r, &utc(), eligible + 600_000, Some(false), &none, 2);
    assert_eq!(at_deadline.len(), 1, "the deadline runs it anyway");
    finish(
        &s,
        &at_deadline[0],
        Ended::Returned(Outcome::Done),
        eligible + 600_000,
        1,
    );
    let row = s.schedules().get(&made.row.id).unwrap().unwrap();
    assert_eq!(
        row.next_due_ms,
        Some(eligible + 600_000 + 3_600_000),
        "armed again from its run"
    );
    assert_eq!(
        take_due(
            &s,
            &r,
            &utc(),
            eligible + 600_000 + 3_600_001,
            Some(true),
            &none,
            2
        )
        .len(),
        1,
        "idle: at once"
    );
}

#[test]
fn a_skip_policy_logs_the_miss_and_moves_on() {
    let (s, r) = (storage(), registry());
    let mut job = new(
        "job",
        Trigger::Every {
            interval_ms: 3_600_000,
            anchor_ms: 0,
        },
        Modifiers::always(),
        CreatedBy::App,
    );
    job.spec.missed = MissedPolicy::Skip;
    let made = store::create(&s, &r, job, &utc(), NOW).unwrap();
    let late = made.row.next_due_ms.unwrap() + 5 * 3_600_000 + 600_000;
    assert!(take_due(&s, &r, &utc(), late, Some(true), &HashSet::new(), 2).is_empty());
    assert_eq!(verbs(&s, &made.row.id), ["created", "missed"]);
    let event = &s
        .ai_log()
        .of_subject(store::SUBJECT, &made.row.id, 1)
        .unwrap()[0];
    assert_eq!(
        (
            event.payload["policy"].as_str(),
            event.payload["missed"].as_u64()
        ),
        (Some("skip"), Some(6))
    );
    assert!(
        s.schedules()
            .get(&made.row.id)
            .unwrap()
            .unwrap()
            .next_due_ms
            .unwrap()
            > late
    );
}

#[test]
fn a_zone_change_recomputes_a_calendar_schedule() {
    let (s, r) = (storage(), registry());
    let nine = Trigger::Calendar {
        rule: CalendarRule::Daily {
            at: ClockTime::new(9, 0),
        },
    };
    let made = store::create(
        &s,
        &r,
        new("job", nine, Modifiers::always(), CreatedBy::App),
        &Context::in_zone(TimeZone::get("Europe/Berlin").unwrap()),
        NOW,
    )
    .unwrap();
    let berlin = made.row.next_due_ms.unwrap();
    let moved = recompute(
        &s,
        &Context::in_zone(TimeZone::get("America/New_York").unwrap()),
        "zone",
        NOW,
    );
    assert_eq!(moved, 1);
    let ny = s
        .schedules()
        .get(&made.row.id)
        .unwrap()
        .unwrap()
        .next_due_ms
        .unwrap();
    assert_eq!(
        (ny - berlin).rem_euclid(86_400_000),
        6 * 3_600_000,
        "9:00 in New York"
    );
    assert_eq!(
        recompute(
            &s,
            &Context::in_zone(TimeZone::get("America/New_York").unwrap()),
            "launch",
            NOW
        ),
        0
    );
}

#[test]
fn only_registered_handlers_run_and_bad_data_is_stopped() {
    let (s, r) = (storage(), registry());
    let made = reminder_at(&s, &r, NOW + 1_000);
    // A build without that handler: the schedule stops, logged, not run.
    let bare = Registry::default();
    assert!(
        take_due(
            &s,
            &bare,
            &utc(),
            NOW + 2_000,
            Some(true),
            &HashSet::new(),
            2
        )
        .is_empty()
    );
    let row = s.schedules().get(&made.row.id).unwrap().unwrap();
    assert_eq!(
        (row.last_outcome.as_deref(), row.next_due_ms),
        (Some("no_handler"), None)
    );
    // Creation checks the handler, the data, and who may use it.
    let mut unknown = new(
        "rm",
        Trigger::At { at_ms: NOW + 5 },
        Modifiers::once(),
        CreatedBy::User,
    );
    assert_eq!(
        store::create(&s, &r, unknown.clone(), &utc(), NOW).unwrap_err(),
        ScheduleError::NoHandler
    );
    unknown.handler = "job";
    assert_eq!(
        store::create(&s, &r, unknown.clone(), &utc(), NOW).unwrap_err(),
        ScheduleError::NotForUsers
    );
    unknown.handler = "remind";
    unknown.payload = json!({ "bad": 1 });
    assert_eq!(
        store::create(&s, &r, unknown.clone(), &utc(), NOW).unwrap_err(),
        ScheduleError::Invalid("bad")
    );
    unknown.payload = json!({});
    unknown.title = "  ".into();
    assert_eq!(
        store::create(&s, &r, unknown, &utc(), NOW).unwrap_err(),
        ScheduleError::Title
    );
}

#[test]
fn the_user_changes_only_their_own_schedules() {
    let (s, r) = (storage(), registry());
    let mine = reminder_at(&s, &r, NOW + 3_600_000);
    let app = store::create(
        &s,
        &r,
        new(
            "job",
            Trigger::Every {
                interval_ms: 3_600_000,
                anchor_ms: 0,
            },
            Modifiers::always(),
            CreatedBy::App,
        ),
        &utc(),
        NOW,
    )
    .unwrap();
    assert_eq!(
        store::pause_users(&s, &app.row.id, true, NOW).unwrap_err(),
        ScheduleError::NotYours
    );
    assert_eq!(
        store::remove_users(&s, &app.row.id, NOW).unwrap_err(),
        ScheduleError::NotYours
    );
    assert!(
        store::pause_users(&s, &mine.row.id, true, NOW)
            .unwrap()
            .row
            .paused
    );
    // Paused: never due.
    assert!(
        take_due(
            &s,
            &r,
            &utc(),
            NOW + 7_200_000,
            Some(true),
            &HashSet::new(),
            2
        )
        .iter()
        .all(|d| d.schedule.row.id != mine.row.id)
    );
    assert_eq!(store::pause_all_users(&s, false, NOW).unwrap(), 1);
    let edited = store::edit_users(
        &s,
        &r,
        &mine.row.id,
        Trigger::At {
            at_ms: NOW + 7_200_000,
        },
        Modifiers::once(),
        "Look at the api",
        &utc(),
        NOW,
    )
    .unwrap();
    assert_eq!(
        (edited.row.next_due_ms, edited.row.title.as_str()),
        (Some(NOW + 7_200_000), "Look at the api")
    );
    store::remove_users(&s, &mine.row.id, NOW).unwrap();
    assert!(s.schedules().get(&mine.row.id).unwrap().is_none());
    assert_eq!(
        verbs(&s, &mine.row.id),
        ["created", "paused", "edited", "removed"]
    );
}

#[test]
fn an_owner_keeps_one_app_schedule_in_step() {
    let (s, r) = (storage(), registry());
    let hourly = new(
        "job",
        Trigger::Every {
            interval_ms: 3_600_000,
            anchor_ms: 0,
        },
        Modifiers::always(),
        CreatedBy::App,
    );
    let first = store::ensure_app(&s, &r, hourly.clone(), &utc(), NOW).unwrap();
    let same = store::ensure_app(&s, &r, hourly.clone(), &utc(), NOW + 5).unwrap();
    assert_eq!(first.row.id, same.row.id);
    assert_eq!(same.row.updated_ms, NOW, "unchanged: not rewritten");
    let mut daily = hourly;
    daily.spec.trigger = Trigger::Every {
        interval_ms: 86_400_000,
        anchor_ms: 0,
    };
    let changed = store::ensure_app(&s, &r, daily, &utc(), NOW + 10).unwrap();
    assert_eq!(changed.row.id, first.row.id);
    assert_eq!(s.schedules().of_handler("test", "job").unwrap().len(), 1);
}

#[test]
fn an_event_arms_its_delay_and_unless_disarms_it() {
    let (s, r) = (storage(), registry());
    let made = store::create(
        &s,
        &r,
        new(
            "job",
            Trigger::AfterEvent {
                event: "started".into(),
                delay_ms: 1_800_000,
                unless: Some("finished".into()),
            },
            Modifiers::always(),
            CreatedBy::App,
        ),
        &utc(),
        NOW,
    )
    .unwrap();
    assert_eq!(made.row.next_due_ms, None);
    store::on_event(&s, "test", "job", "started", &utc(), NOW + 10).unwrap();
    assert_eq!(
        s.schedules()
            .get(&made.row.id)
            .unwrap()
            .unwrap()
            .next_due_ms,
        Some(NOW + 10 + 1_800_000)
    );
    store::on_event(&s, "test", "job", "finished", &utc(), NOW + 20).unwrap();
    assert_eq!(
        s.schedules()
            .get(&made.row.id)
            .unwrap()
            .unwrap()
            .next_due_ms,
        None
    );
}

#[test]
fn the_user_list_pages_newest_first() {
    let (s, r) = (storage(), registry());
    for i in 0..5 {
        store::create(
            &s,
            &r,
            new(
                "remind",
                Trigger::At {
                    at_ms: NOW + 10_000,
                },
                Modifiers::once(),
                CreatedBy::User,
            ),
            &utc(),
            NOW + i,
        )
        .unwrap();
    }
    let first = s.schedules().page(true, None, 3).unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(first[0].created_ms, NOW + 4);
    let key = (first[2].created_ms, first[2].id.clone());
    let rest = s.schedules().page(true, Some(&key), 3).unwrap();
    assert_eq!(rest.len(), 2);
    assert_eq!(rest[1].created_ms, NOW);
    assert!(s.schedules().page(false, None, 3).unwrap().is_empty());
}
