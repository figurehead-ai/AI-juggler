#![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]
//! AJ-E2027 R1: the trigger maths, by example and by property: the spring
//! gap and the autumn overlap in zones with whole-hour and half-hour
//! changes, both hemispheres; windows; jitter; every missed-run policy.

use jiff::civil::date;
use jiff::tz::TimeZone;
use proptest::prelude::*;

use super::*;

const ZONES: [&str; 7] = [
    "America/New_York",
    "Europe/Berlin",
    "Australia/Sydney",
    "Australia/Lord_Howe",
    "Asia/Kolkata",
    "America/Sao_Paulo",
    "UTC",
];

fn zone(name: &str) -> TimeZone {
    TimeZone::get(name).unwrap()
}

fn ms(zone: &TimeZone, y: i16, mo: i8, d: i8, h: i8, mi: i8) -> i64 {
    date(y, mo, d)
        .at(h, mi, 0, 0)
        .to_zoned(zone.clone())
        .unwrap()
        .timestamp()
        .as_millisecond()
}

fn utc(y: i16, mo: i8, d: i8, h: i8, mi: i8) -> i64 {
    ms(&TimeZone::UTC, y, mo, d, h, mi)
}

fn spec(trigger: Trigger) -> Spec {
    Spec {
        trigger,
        modifiers: Modifiers::always(),
        missed: MissedPolicy::RunOnce,
    }
}

fn at_base(base_ms: i64) -> Place {
    Place {
        base_ms,
        armed_ms: None,
    }
}

// ============================================================================ DST, by example

#[test]
fn the_spring_gap_runs_at_the_first_valid_time_after_it() {
    // New York, 2026-03-08: 2:00 EST jumps to 3:00 EDT (07:00 UTC).
    let ny = zone("America/New_York");
    let before = utc(2026, 3, 8, 0, 0);
    let got = calendar_after(
        &CalendarRule::Daily {
            at: ClockTime::new(2, 30),
        },
        before,
        &ny,
    )
    .unwrap();
    assert_eq!(got, utc(2026, 3, 8, 7, 0), "the gap's end, 3:00 EDT");
    let wall = local(got, &ny).unwrap();
    assert_eq!((wall.hour(), wall.minute()), (3, 0));
    // The next day is back on 2:30.
    let then = calendar_after(
        &CalendarRule::Daily {
            at: ClockTime::new(2, 30),
        },
        got,
        &ny,
    )
    .unwrap();
    assert_eq!(then, utc(2026, 3, 9, 6, 30));
}

#[test]
fn a_half_hour_gap_runs_at_its_end_too() {
    // Lord Howe, 2026-10-04: 2:00 (+10:30) jumps to 2:30 (+11:00).
    let lh = zone("Australia/Lord_Howe");
    let got = calendar_after(
        &CalendarRule::Daily {
            at: ClockTime::new(2, 15),
        },
        utc(2026, 10, 3, 0, 0),
        &lh,
    )
    .unwrap();
    let wall = local(got, &lh).unwrap();
    assert_eq!(wall.date(), date(2026, 10, 4));
    assert_eq!((wall.hour(), wall.minute()), (2, 30));
}

#[test]
fn the_autumn_overlap_runs_once_at_the_first() {
    // New York, 2026-11-01: 1:00–2:00 happens twice (EDT, then EST).
    let ny = zone("America/New_York");
    let rule = CalendarRule::Daily {
        at: ClockTime::new(1, 30),
    };
    let first = calendar_after(&rule, utc(2026, 11, 1, 0, 0), &ny).unwrap();
    assert_eq!(first, utc(2026, 11, 1, 5, 30), "1:30 EDT, the first");
    let second = calendar_after(&rule, first, &ny).unwrap();
    assert_eq!(second, utc(2026, 11, 2, 6, 30), "not 1:30 EST: tomorrow");
}

#[test]
fn every_is_absolute_time_across_a_clock_change() {
    let ny = zone("America/New_York");
    let s = spec(Trigger::Every {
        interval_ms: 3_600_000,
        anchor_ms: 0,
    });
    let ctx = Context::in_zone(ny);
    let mut place = at_base(utc(2026, 3, 8, 4, 30));
    for _ in 0..6 {
        let n = next(&s, place, &ctx, 1).unwrap();
        assert_eq!(n.occurrence_ms.rem_euclid(3_600_000), 0, "on the hour, UTC");
        if place.base_ms.rem_euclid(3_600_000) == 0 {
            assert_eq!(
                n.occurrence_ms - place.base_ms,
                3_600_000,
                "an hour, gap or not"
            );
        }
        place = at_base(n.occurrence_ms);
    }
}

#[test]
fn every_is_anchored_to_the_wall_clock_not_to_launch() {
    let s = spec(Trigger::Every {
        interval_ms: 3_600_000,
        anchor_ms: utc(2026, 1, 1, 0, 15),
    });
    let ctx = Context::in_zone(TimeZone::UTC);
    // Launched at 10:47: the next is 11:15, not 11:47.
    let n = next(&s, at_base(utc(2026, 5, 5, 10, 47)), &ctx, 1).unwrap();
    assert_eq!(n.due_ms, utc(2026, 5, 5, 11, 15));
}

#[test]
fn calendar_rules_land_on_their_days() {
    let berlin = zone("Europe/Berlin");
    // 2026-11-26 is a Thursday.
    let thursday = ms(&berlin, 2026, 11, 26, 12, 0);
    let weekdays = CalendarRule::Weekdays {
        at: ClockTime::new(9, 0),
    };
    let fri = calendar_after(&weekdays, thursday, &berlin).unwrap();
    assert_eq!(fri, ms(&berlin, 2026, 11, 27, 9, 0));
    let mon = calendar_after(&weekdays, fri, &berlin).unwrap();
    assert_eq!(mon, ms(&berlin, 2026, 11, 30, 9, 0), "past the weekend");
    let weekly = CalendarRule::Weekly {
        days: vec![3, 7],
        at: ClockTime::new(18, 30),
    };
    assert_eq!(
        calendar_after(&weekly, thursday, &berlin).unwrap(),
        ms(&berlin, 2026, 11, 29, 18, 30),
        "Sunday"
    );
    let monthly = CalendarRule::Monthly {
        day: 31,
        at: ClockTime::new(8, 0),
    };
    assert_eq!(
        calendar_after(&monthly, thursday, &berlin).unwrap(),
        ms(&berlin, 2026, 11, 30, 8, 0),
        "November has no 31st: its last day"
    );
    let dec = calendar_after(&monthly, ms(&berlin, 2026, 11, 30, 8, 0), &berlin).unwrap();
    assert_eq!(dec, ms(&berlin, 2026, 12, 31, 8, 0));
}

// ============================================================================ windows and jitter, by example

/// A reminder at 23:10 in Berlin, in `window`.
fn late_evening(window: Window) -> (TimeZone, Spec) {
    let berlin = zone("Europe/Berlin");
    let mut s = spec(Trigger::At {
        at_ms: ms(&berlin, 2026, 6, 1, 23, 10),
    });
    s.modifiers.window = Some(window);
    (berlin, s)
}

#[test]
fn a_time_outside_the_window_waits_for_it_to_open() {
    let (berlin, mut s) = late_evening(Window::Between {
        from: ClockTime::new(8, 0),
        until: ClockTime::new(20, 0),
    });
    let ctx = Context::in_zone(berlin.clone());
    let n = next(&s, at_base(0), &ctx, 1).unwrap();
    assert_eq!(n.due_ms, ms(&berlin, 2026, 6, 2, 8, 0));
    // Across midnight: 22:00–06:00 is open at 23:10.
    s.modifiers.window = Some(Window::Between {
        from: ClockTime::new(22, 0),
        until: ClockTime::new(6, 0),
    });
    assert_eq!(
        next(&s, at_base(0), &ctx, 1).unwrap().due_ms,
        n.occurrence_ms
    );
}

#[test]
fn outside_quiet_hours_reads_them_by_reference() {
    let (berlin, s) = late_evening(Window::OutsideQuietHours);
    let mut ctx = Context::in_zone(berlin.clone());
    // No quiet hours: always open.
    assert_eq!(
        next(&s, at_base(0), &ctx, 1).unwrap().due_ms,
        ms(&berlin, 2026, 6, 1, 23, 10)
    );
    ctx.quiet = Some((ClockTime::new(22, 0), ClockTime::new(7, 30)));
    assert_eq!(
        next(&s, at_base(0), &ctx, 1).unwrap().due_ms,
        ms(&berlin, 2026, 6, 2, 7, 30)
    );
}

#[test]
fn jitter_is_stable_and_bounded() {
    for occ in [0_i64, 1, 1_800_000_000_000, -5] {
        let j = jitter_of(seed_of("s-1"), occ, 30_000);
        assert!((0..=30_000).contains(&j));
        assert_eq!(j, jitter_of(seed_of("s-1"), occ, 30_000), "stable");
    }
    assert_eq!(jitter_of(7, 7, 0), 0);
    // Different schedules spread out.
    let spread: std::collections::HashSet<i64> = (0..50)
        .map(|i| jitter_of(seed_of(&format!("s-{i}")), 1_800_000_000_000, 60_000))
        .collect();
    assert!(spread.len() > 40, "{}", spread.len());
}

// ============================================================================ missed runs, by example

fn hourly(policy: MissedPolicy) -> Spec {
    Spec {
        trigger: Trigger::Every {
            interval_ms: 3_600_000,
            anchor_ms: 0,
        },
        modifiers: Modifiers::always(),
        missed: policy,
    }
}

#[test]
fn run_once_runs_one_however_many_were_missed() {
    let ctx = Context::in_zone(TimeZone::UTC);
    let base = utc(2026, 5, 5, 10, 0);
    let now = utc(2026, 5, 5, 15, 30);
    let f = fire(&hourly(MissedPolicy::RunOnce), at_base(base), now, &ctx, 1);
    assert_eq!((f.runs, f.missed, f.on_time), (1, 4, false));
    assert_eq!(f.place.base_ms, utc(2026, 5, 5, 15, 0));
    assert_eq!(f.next.unwrap().due_ms, utc(2026, 5, 5, 16, 0));
}

#[test]
fn skip_runs_only_on_time() {
    let ctx = Context::in_zone(TimeZone::UTC);
    let base = utc(2026, 5, 5, 10, 0);
    let late = fire(
        &hourly(MissedPolicy::Skip),
        at_base(base),
        utc(2026, 5, 5, 13, 20),
        &ctx,
        1,
    );
    assert_eq!((late.runs, late.missed), (0, 3));
    assert_eq!(late.next.unwrap().due_ms, utc(2026, 5, 5, 14, 0));
    let on_time = fire(
        &hourly(MissedPolicy::Skip),
        at_base(base),
        utc(2026, 5, 5, 11, 0) + 20_000,
        &ctx,
        1,
    );
    assert_eq!(
        (on_time.runs, on_time.missed, on_time.on_time),
        (1, 0, true)
    );
}

#[test]
fn run_each_runs_each_up_to_its_cap() {
    let ctx = Context::in_zone(TimeZone::UTC);
    let base = utc(2026, 5, 5, 10, 0);
    let now = utc(2026, 5, 5, 13, 30);
    let f = fire(
        &hourly(MissedPolicy::RunEach { cap: 5 }),
        at_base(base),
        now,
        &ctx,
        1,
    );
    assert_eq!((f.runs, f.missed), (3, 0));
    let f = fire(
        &hourly(MissedPolicy::RunEach { cap: 2 }),
        at_base(base),
        now,
        &ctx,
        1,
    );
    assert_eq!((f.runs, f.missed), (2, 1));
}

#[test]
fn a_reminder_missed_while_closed_runs_once_on_launch_and_ends() {
    let ctx = Context::in_zone(TimeZone::UTC);
    let at = utc(2026, 5, 5, 15, 0);
    let s = Spec {
        trigger: Trigger::At { at_ms: at },
        modifiers: Modifiers::once(),
        missed: MissedPolicy::RunOnce,
    };
    let place = first_place(&s.trigger, utc(2026, 5, 5, 9, 0));
    assert_eq!(next(&s, place, &ctx, 1).unwrap().due_ms, at);
    let f = fire(&s, place, utc(2026, 5, 6, 8, 0), &ctx, 1);
    assert_eq!((f.runs, f.missed, f.on_time), (1, 0, false));
    assert_eq!(f.next, None, "ended");
    // Made for a time already past: due at once.
    let past = first_place(&s.trigger, at + 5_000);
    assert_eq!(next(&s, past, &ctx, 1).unwrap().due_ms, at);
}

#[test]
fn an_idle_job_arms_again_from_its_run() {
    let ctx = Context::in_zone(TimeZone::UTC);
    let s = Spec {
        trigger: Trigger::WhenIdle {
            max_wait_ms: 3_600_000,
            after_ms: 3_600_000,
        },
        modifiers: Modifiers::always(),
        missed: MissedPolicy::RunOnce,
    };
    assert_eq!(s.validate(), Ok(()));
    let made = utc(2026, 5, 5, 9, 0);
    let place = first_place(&s.trigger, made);
    let n = next(&s, place, &ctx, 1).unwrap();
    assert_eq!(n.due_ms, made + 3_600_000);
    assert_eq!(
        idle_deadline(&s.trigger, n.occurrence_ms),
        Some(made + 7_200_000)
    );
    let ran = made + 5_000_000;
    let f = fire(&s, place, ran, &ctx, 1);
    assert_eq!(f.runs, 1);
    assert_eq!(f.next.unwrap().due_ms, ran + 3_600_000);
    // A repeating idle job with no gap would run back to back: refused.
    let mut tight = s.clone();
    tight.trigger = Trigger::WhenIdle {
        max_wait_ms: 60_000,
        after_ms: 0,
    };
    assert_eq!(tight.validate(), Err("wait"));
}

#[test]
fn an_event_delay_waits_for_its_event() {
    let ctx = Context::in_zone(TimeZone::UTC);
    let s = spec(Trigger::AfterEvent {
        event: "session.started".into(),
        delay_ms: 1_800_000,
        unless: Some("session.finished".into()),
    });
    let place = first_place(&s.trigger, 1_000);
    assert_eq!(next(&s, place, &ctx, 1), None, "not armed");
    let armed = Place {
        base_ms: 1_000,
        armed_ms: Some(10_000),
    };
    assert_eq!(next(&s, armed, &ctx, 1).unwrap().due_ms, 1_810_000);
    let f = fire(&s, armed, 1_810_000, &ctx, 1);
    assert_eq!((f.runs, f.next), (1, None), "waits for the next event");
}

#[test]
fn a_lifetime_until_stops_the_occurrences() {
    let ctx = Context::in_zone(TimeZone::UTC);
    let mut s = hourly(MissedPolicy::RunEach { cap: 100 });
    s.modifiers.lifetime = Lifetime::Until {
        at_ms: utc(2026, 5, 5, 12, 0),
    };
    let f = fire(
        &s,
        at_base(utc(2026, 5, 5, 9, 0)),
        utc(2026, 5, 6, 0, 0),
        &ctx,
        1,
    );
    assert_eq!(f.runs, 2, "10:00 and 11:00; 12:00 is the end");
    assert_eq!(f.next, None);
}

#[test]
fn bounds_are_checked() {
    let bad = [
        spec(Trigger::Every {
            interval_ms: 1_000,
            anchor_ms: 0,
        }),
        spec(Trigger::Calendar {
            rule: CalendarRule::Daily {
                at: ClockTime::new(24, 0),
            },
        }),
        spec(Trigger::Calendar {
            rule: CalendarRule::Weekly {
                days: vec![],
                at: ClockTime::new(9, 0),
            },
        }),
        spec(Trigger::Calendar {
            rule: CalendarRule::Monthly {
                day: 32,
                at: ClockTime::new(9, 0),
            },
        }),
        spec(Trigger::AfterEvent {
            event: "a sentence".into(),
            delay_ms: 0,
            unless: None,
        }),
        Spec {
            missed: MissedPolicy::RunEach { cap: 0 },
            ..hourly(MissedPolicy::RunOnce)
        },
    ];
    for s in bad {
        assert!(s.validate().is_err(), "{s:?}");
    }
    assert_eq!(hourly(MissedPolicy::Skip).validate(), Ok(()));
}

#[test]
fn every_policy_has_a_code() {
    assert_eq!(MissedPolicy::default().code(), "run_once");
    assert_eq!(MissedPolicy::Skip.code(), "skip");
    assert_eq!(MissedPolicy::RunEach { cap: 3 }.code(), "run_each");
}

// ============================================================================ properties

fn any_zone() -> impl Strategy<Value = TimeZone> {
    proptest::sample::select(ZONES.to_vec()).prop_map(zone)
}

/// 2025-01-01 … 2030-12-31, which holds every zone's changes.
fn any_instant() -> impl Strategy<Value = i64> {
    1_735_689_600_000_i64..1_924_905_600_000_i64
}

fn any_time() -> impl Strategy<Value = ClockTime> {
    (0_u8..24, 0_u8..60).prop_map(|(h, m)| ClockTime::new(h, m))
}

fn any_rule() -> impl Strategy<Value = CalendarRule> {
    prop_oneof![
        any_time().prop_map(|at| CalendarRule::Daily { at }),
        any_time().prop_map(|at| CalendarRule::Weekdays { at }),
        (proptest::collection::btree_set(1_u8..=7, 1..=7), any_time()).prop_map(|(days, at)| {
            CalendarRule::Weekly {
                days: days.into_iter().collect(),
                at,
            }
        }),
        (1_u8..=31, any_time()).prop_map(|(day, at)| CalendarRule::Monthly { day, at }),
    ]
}

fn any_policy() -> impl Strategy<Value = MissedPolicy> {
    prop_oneof![
        Just(MissedPolicy::RunOnce),
        Just(MissedPolicy::Skip),
        (1_u32..=RUN_EACH_MAX).prop_map(|cap| MissedPolicy::RunEach { cap }),
    ]
}

/// `s` from `base`, fired `late` after its first due time: that first,
/// the time it fired, and what firing gave.
fn fired_late(s: &Spec, zone: TimeZone, base: i64, late: i64) -> (Next, i64, Fired) {
    let ctx = Context::in_zone(zone);
    let place = Place {
        base_ms: base,
        armed_ms: None,
    };
    let first = next(s, place, &ctx, 1).unwrap();
    let now = first.due_ms + late;
    (first, now, fire(s, place, now, &ctx, 1))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// A calendar occurrence is strictly later, on a day the rule names,
    /// at the rule's wall time, or (only in a spring gap) at the first
    /// valid time after it; and nothing the rule names lies between.
    #[test]
    fn a_calendar_occurrence_is_the_next_wall_time(rule in any_rule(), zone in any_zone(), after in any_instant()) {
        let got = calendar_after(&rule, after, &zone).unwrap();
        prop_assert!(got > after);
        let wall = local(got, &zone).unwrap();
        let at = rule.at();
        let exact = (wall.hour(), wall.minute(), wall.second()) == (i8::try_from(at.hour).unwrap(), i8::try_from(at.minute).unwrap(), 0);
        if exact {
            prop_assert!(rule.matches(wall.date()));
        } else {
            // A gap: the wall time didn't exist that day, and this is the
            // transition that skipped it.
            let asked = wall.date().to_datetime(at.time().unwrap());
            let skipped = matches!(zone.to_ambiguous_timestamp(asked).offset(), AmbiguousOffset::Gap { .. })
                || {
                    // ...or the gap was on the day before (a late-night rule whose
                    // gap's end falls after midnight).
                    let prev = wall.date().yesterday().unwrap().to_datetime(at.time().unwrap());
                    matches!(zone.to_ambiguous_timestamp(prev).offset(), AmbiguousOffset::Gap { .. })
                };
            prop_assert!(skipped, "{rule:?} {wall} in {:?}", zone.iana_name());
        }
        // Nothing earlier after `after`: one minute back finds the same.
        prop_assert_eq!(calendar_after(&rule, got - 1, &zone), Some(got));
    }

    /// Stepping a daily rule from occurrence to occurrence never gives the
    /// same day twice (the overlap runs once) and never skips a day (the
    /// gap still runs).
    #[test]
    fn a_daily_rule_runs_once_a_day_through_every_change(at in any_time(), zone in any_zone(), start in any_instant()) {
        let rule = CalendarRule::Daily { at };
        let mut prev = calendar_after(&rule, start, &zone).unwrap();
        let mut prev_day = local(prev, &zone).unwrap().date();
        for _ in 0..400 {
            let next = calendar_after(&rule, prev, &zone).unwrap();
            let day = local(next, &zone).unwrap().date();
            let days = prev_day.until(day).unwrap().get_days();
            prop_assert_eq!(days, 1, "{} → {} for {:?} in {:?}", prev_day, day, at, zone.iana_name());
            prop_assert!(next - prev >= 22 * 3_600_000 && next - prev <= 26 * 3_600_000 + 1);
            prev = next;
            prev_day = day;
        }
    }

    /// Every: each occurrence is `anchor + k * interval`, the first after
    /// the base, in absolute time.
    #[test]
    fn every_is_anchored_absolute_time(interval in EVERY_MIN_MS..(3 * DAY_MS), anchor in any_instant(), base in any_instant()) {
        let place = Place { base_ms: base, armed_ms: None };
        let t = Trigger::Every { interval_ms: interval, anchor_ms: anchor };
        let occ = occurrence_after(&t, place, &TimeZone::UTC).unwrap();
        prop_assert!(occ > base && occ - base <= interval);
        prop_assert_eq!((occ - anchor).rem_euclid(interval), 0);
    }

    /// The due time is never before the occurrence, within its jitter when
    /// there is no window, and inside the window when there is one (unless
    /// the window's opening fell in a gap).
    #[test]
    fn the_due_time_respects_jitter_and_window(
        occ in any_instant(), zone in any_zone(), jitter in 0..=JITTER_MAX_MS,
        from in any_time(), until in any_time(), windowed in any::<bool>(), seed in any::<u64>(),
    ) {
        let s = Spec {
            trigger: Trigger::At { at_ms: occ },
            modifiers: Modifiers {
                window: windowed.then_some(Window::Between { from, until }),
                jitter_ms: jitter,
                lifetime: Lifetime::Once,
            },
            missed: MissedPolicy::RunOnce,
        };
        prop_assert_eq!(s.validate(), Ok(()));
        let ctx = Context::in_zone(zone.clone());
        let n = next(&s, Place { base_ms: occ - 1, armed_ms: None }, &ctx, seed).unwrap();
        prop_assert!(n.due_ms >= occ);
        if !windowed || from == until {
            prop_assert!(n.due_ms <= occ + jitter);
        } else {
            let opening_in_gap = local(n.due_ms, &zone).map(|w| (w.hour(), w.minute()))
                != Some((i8::try_from(from.hour).unwrap(), i8::try_from(from.minute).unwrap()));
            prop_assert!(in_window(n.due_ms, from, until, &zone) || opening_in_gap);
            prop_assert!(n.due_ms - occ <= jitter + 26 * 3_600_000);
        }
        // Stable: the same again.
        prop_assert_eq!(next(&s, Place { base_ms: occ - 1, armed_ms: None }, &ctx, seed), Some(n));
    }

    /// Every policy: runs never exceed what was due, runs + missed is what
    /// was due, the next is after now, and `RunOnce` always runs once.
    #[test]
    fn missed_runs_follow_the_policy(
        policy in any_policy(), interval in EVERY_MIN_MS..(2 * DAY_MS), anchor in any_instant(),
        base in any_instant(), late in 0_i64..(60 * DAY_MS),
    ) {
        let s = Spec {
            trigger: Trigger::Every { interval_ms: interval, anchor_ms: anchor },
            modifiers: Modifiers::always(),
            missed: policy,
        };
        let (first, now, f) = fired_late(&s, TimeZone::UTC, base, late);
        let due = u32::try_from((now - first.occurrence_ms) / interval + 1).unwrap();
        prop_assert_eq!(f.runs + f.missed, due);
        prop_assert!(f.next.unwrap().due_ms > now);
        prop_assert!(f.place.base_ms <= now);
        match policy {
            MissedPolicy::RunOnce => prop_assert_eq!(f.runs, 1),
            MissedPolicy::Skip => prop_assert_eq!(f.runs, u32::from(now - (anchor + ((now - anchor).div_euclid(interval)) * interval) <= ON_TIME_MS)),
            MissedPolicy::RunEach { cap } => prop_assert_eq!(f.runs, due.min(cap)),
        }
        prop_assert_eq!(f.on_time, now - f.place.base_ms <= ON_TIME_MS);
    }

    /// A calendar schedule fired late with `RunEach` counts the days it
    /// missed, in any zone, and is next due after now.
    #[test]
    fn a_calendar_schedule_counts_what_it_missed(rule in any_rule(), zone in any_zone(), base in any_instant(), late in 0_i64..(40 * DAY_MS)) {
        let s = Spec {
            trigger: Trigger::Calendar { rule: rule.clone() },
            modifiers: Modifiers::always(),
            missed: MissedPolicy::RunEach { cap: RUN_EACH_MAX },
        };
        let (_, now, f) = fired_late(&s, zone.clone(), base, late);
        // Walk them one by one: the same count.
        let mut walked = 0_u32;
        let mut at = base;
        while let Some(occ) = calendar_after(&rule, at, &zone) {
            if occ > now { break; }
            walked += 1;
            at = occ;
        }
        prop_assert_eq!(f.runs + f.missed, walked);
        prop_assert!(f.runs >= 1);
        prop_assert!(f.next.unwrap().due_ms > now);
    }
}
