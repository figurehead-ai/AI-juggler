#![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]
//! AJ-E3814's gates: the replay, every condition, what is offered (D-127),
//! lifetimes, the quiet switch, the echo in English and German, the
//! registry's extension, the store and its log, and that nothing the
//! engine does writes a watch.

use std::sync::Arc;

use juggler_adapter::{Capabilities, Observes, ToolKind, ToolObserves};

use super::catalogue::{catalogue, check_offered};
use super::conditions::{ConditionRule, Memo, Registry, SHIPPED, Verdict};
use super::echo::{LocalClock, Names, Unnamed, echo};
use super::engine::{EndReason, Engine, Outcome};
use super::model::{
    Condition, Lifetime, LifetimeChoice, MAX_ACTIVE, Matching, Param, TargetKind, Watch,
    WatchError, WatchId, WatchTarget,
};
use super::words::{self, CondWords, Lang};
use crate::features::observe::{
    EndKind, Observation, Observed, Removed, SessionState, Subject, TaskState, TurnOutcome,
};

const T: i64 = 1_800_000_000_000;
const MIN: i64 = 60_000;

fn session(id: &str) -> WatchTarget {
    WatchTarget::Session(id.into())
}

fn cond(code: &str) -> Condition {
    Condition {
        code: code.into(),
        param: None,
    }
}

fn cond_p(code: &str, param: Param) -> Condition {
    Condition {
        code: code.into(),
        param: Some(param),
    }
}

fn watch(
    id: &str,
    targets: Vec<WatchTarget>,
    conditions: Vec<Condition>,
    lifetime: Lifetime,
) -> Watch {
    Watch {
        id: WatchId(id.into()),
        targets,
        matching: Matching::Each,
        conditions,
        lifetime,
        paused: false,
        even_in_quiet_hours: false,
        created_ms: T,
    }
}

fn subject(session: &str) -> Subject {
    Subject {
        session: Some(session.into()),
        apps: vec!["app-cli".into()],
        project: Some("proj-1".into()),
        account: Some("acct-1".into()),
        pools: vec!["pool-1".into()],
    }
}

fn at(at_ms: i64, subject: Subject, event: Observed) -> Observation {
    Observation {
        at_ms,
        subject,
        event,
        replayed: false,
    }
}

fn state(s: SessionState) -> Observed {
    Observed::StateEntered { state: s }
}

fn engine_with(watches: Vec<Watch>) -> Engine {
    let mut engine = Engine::new(Registry::shipped());
    for w in watches {
        w.validate(&Registry::shipped(), T).unwrap();
        engine.add(w, T);
    }
    engine
}

// ---------------------------------------------------------------------------
// The replay
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Replay {
    start_ms: i64,
    watches: Vec<Watch>,
    observations: Vec<Observation>,
    fires: serde_json::Value,
    ended: serde_json::Value,
}

fn run_replay(replay: &Replay) -> Outcome {
    let mut engine = Engine::new(Registry::shipped());
    for w in &replay.watches {
        w.validate(&Registry::shipped(), replay.start_ms).unwrap();
        engine.add(w.clone(), replay.start_ms);
    }
    let mut out = Outcome::default();
    for obs in &replay.observations {
        let step = engine.observe(obs);
        out.fires.extend(step.fires);
        out.ended.extend(step.ended);
    }
    out
}

/// A recorded event stream gives exactly the expected fires and endings,
/// and the same ones every time it is replayed.
#[test]
fn the_replay_gives_the_same_fires_every_time() {
    let replay: Replay = serde_json::from_str(include_str!("fixtures/replay.json")).unwrap();
    let first = run_replay(&replay);
    let second = run_replay(&replay);
    assert_eq!(first, second, "the same events gave different fires");
    assert_eq!(serde_json::to_value(&first.fires).unwrap(), replay.fires);
    assert_eq!(serde_json::to_value(&first.ended).unwrap(), replay.ended);
}

// ---------------------------------------------------------------------------
// Every condition: fires, doesn't fire
// ---------------------------------------------------------------------------

/// One case per condition: a watch on its target, an event that meets it,
/// one that doesn't, and when it should fire (deadlines included).
struct Case {
    code: &'static str,
    target: WatchTarget,
    param: Option<Param>,
    meets: Observation,
    misses: Observation,
    fires_at: i64,
}

fn cases() -> Vec<Case> {
    let s = || subject("api");
    let account = || Subject {
        account: Some("acct-1".into()),
        apps: vec!["app-cli".into()],
        ..Subject::default()
    };
    let pool = || Subject {
        pools: vec!["pool-1".into()],
        ..Subject::default()
    };
    let project = || Subject {
        project: Some("proj-1".into()),
        ..Subject::default()
    };
    vec![
        Case {
            code: "finishes",
            target: session("api"),
            param: None,
            meets: at(
                T + MIN,
                s(),
                Observed::TurnEnded {
                    outcome: TurnOutcome::Completed,
                },
            ),
            misses: at(
                T + MIN,
                s(),
                Observed::TurnEnded {
                    outcome: TurnOutcome::Interrupted,
                },
            ),
            fires_at: T + MIN,
        },
        Case {
            code: "ends",
            target: WatchTarget::App("app-cli".into()),
            param: None,
            meets: at(
                T + MIN,
                s(),
                Observed::Ended {
                    how: EndKind::Stopped,
                },
            ),
            misses: at(T + MIN, s(), state(SessionState::Finished)),
            fires_at: T + MIN,
        },
        Case {
            code: "asks_you",
            target: session("api"),
            param: Some(Param::Minutes(2)),
            meets: at(T + MIN, s(), Observed::AskOpened),
            misses: at(T + MIN, s(), Observed::AskResolved),
            fires_at: T + 3 * MIN,
        },
        Case {
            code: "fails",
            target: WatchTarget::Project("proj-1".into()),
            param: None,
            meets: at(
                T + MIN,
                s(),
                Observed::Ended {
                    how: EndKind::Error,
                },
            ),
            misses: at(
                T + MIN,
                s(),
                Observed::Ended {
                    how: EndKind::Clean,
                },
            ),
            fires_at: T + MIN,
        },
        Case {
            code: "stuck",
            target: session("api"),
            param: Some(Param::Minutes(10)),
            meets: at(T + MIN, s(), state(SessionState::Stuck)),
            misses: at(T + MIN, s(), state(SessionState::Working)),
            fires_at: T + 11 * MIN,
        },
        Case {
            code: "idle_for",
            target: WatchTarget::App("app-cli".into()),
            param: Some(Param::Minutes(30)),
            meets: at(T + MIN, s(), state(SessionState::Working)),
            misses: at(T + MIN, account(), Observed::RunwayReset),
            fires_at: T + 31 * MIN,
        },
        Case {
            code: "hits_limit",
            target: WatchTarget::Account("acct-1".into()),
            param: None,
            meets: at(T + MIN, s(), state(SessionState::LimitHit)),
            misses: at(T + MIN, s(), state(SessionState::Stuck)),
            fires_at: T + MIN,
        },
        Case {
            code: "runway_below",
            target: WatchTarget::Account("acct-1".into()),
            param: Some(Param::Percent(20)),
            meets: at(T + MIN, account(), Observed::RunwayLeft { percent: 15 }),
            misses: at(T + MIN, account(), Observed::RunwayLeft { percent: 25 }),
            fires_at: T + MIN,
        },
        Case {
            code: "runway_resets",
            target: WatchTarget::Any,
            param: None,
            meets: at(T + MIN, account(), Observed::RunwayReset),
            misses: at(T + MIN, account(), Observed::RunwayLeft { percent: 50 }),
            fires_at: T + MIN,
        },
        Case {
            code: "leaves_processes",
            target: session("api"),
            param: None,
            meets: at(T + MIN, s(), Observed::ProcessesLeft { count: 2 }),
            misses: at(T + MIN, s(), Observed::ProcessesLeft { count: 0 }),
            fires_at: T + MIN,
        },
        Case {
            code: "restarted",
            target: WatchTarget::App("app-cli".into()),
            param: None,
            meets: at(T + MIN, s(), Observed::Restarted),
            misses: at(T + MIN, s(), Observed::Started),
            fires_at: T + MIN,
        },
        Case {
            code: "pool_message",
            target: WatchTarget::Pool("pool-1".into()),
            param: Some(Param::Agent("agent-a".into())),
            meets: at(
                T + MIN,
                pool(),
                Observed::PoolMessage {
                    from: Some("agent-a".into()),
                },
            ),
            misses: at(
                T + MIN,
                pool(),
                Observed::PoolMessage {
                    from: Some("agent-b".into()),
                },
            ),
            fires_at: T + MIN,
        },
        Case {
            code: "task_changes",
            target: WatchTarget::Project("proj-1".into()),
            param: Some(Param::TaskState(TaskState::Blocked)),
            meets: at(
                T + MIN,
                project(),
                Observed::TaskMoved {
                    to: TaskState::Blocked,
                },
            ),
            misses: at(
                T + MIN,
                project(),
                Observed::TaskMoved {
                    to: TaskState::Done,
                },
            ),
            fires_at: T + MIN,
        },
        Case {
            code: "commits",
            target: WatchTarget::Project("proj-1".into()),
            param: None,
            meets: at(T + MIN, project(), Observed::Commit),
            misses: at(
                T + MIN,
                project(),
                Observed::TaskMoved {
                    to: TaskState::Done,
                },
            ),
            fires_at: T + MIN,
        },
        Case {
            code: "starts",
            target: WatchTarget::Any,
            param: None,
            meets: at(T + MIN, s(), Observed::Started),
            misses: at(
                T + MIN,
                s(),
                Observed::Ended {
                    how: EndKind::Clean,
                },
            ),
            fires_at: T + MIN,
        },
    ]
}

fn case_watch(case: &Case) -> Watch {
    watch(
        "w-c0",
        vec![case.target.clone()],
        vec![Condition {
            code: case.code.into(),
            param: case.param.clone(),
        }],
        Lifetime::Always,
    )
}

/// Every condition of the closed set has a case.
#[test]
fn every_shipped_condition_is_tested() {
    let tested: Vec<&str> = cases().iter().map(|c| c.code).collect();
    for rule in SHIPPED {
        assert!(tested.contains(&rule.code()), "{} has no case", rule.code());
    }
    assert_eq!(
        SHIPPED.len(),
        15,
        "ADR-0036 §2.2's closed set is 15 conditions"
    );
}

#[test]
fn every_condition_fires_on_what_meets_it() {
    for case in cases() {
        let mut engine = engine_with(vec![case_watch(&case)]);
        let mut out = engine.observe(&case.meets);
        let later = engine.tick(case.fires_at + 1);
        out.fires.extend(later.fires);
        assert_eq!(
            out.fires.len(),
            1,
            "{} didn't fire once: {:?}",
            case.code,
            out.fires
        );
        assert_eq!(out.fires[0].condition, case.code);
        assert_eq!(
            out.fires[0].at_ms, case.fires_at,
            "{} fired at the wrong time",
            case.code
        );
    }
}

#[test]
fn no_condition_fires_on_what_misses_it() {
    for case in cases() {
        let mut engine = engine_with(vec![case_watch(&case)]);
        let mut out = engine.observe(&case.misses);
        // Idle counts from activity; its miss is an account's, not activity.
        let later = engine.tick(case.fires_at - 1);
        out.fires.extend(later.fires);
        assert!(
            out.fires.is_empty(),
            "{} fired on a miss: {:?}",
            case.code,
            out.fires
        );
    }
}

#[test]
fn a_deadline_is_disarmed_when_its_state_is_left() {
    let mut engine = engine_with(vec![watch(
        "w-d1",
        vec![session("api")],
        vec![cond_p("stuck", Param::Minutes(10))],
        Lifetime::Always,
    )]);
    engine.observe(&at(T + MIN, subject("api"), state(SessionState::Stuck)));
    assert_eq!(engine.next_wake(), Some(T + 11 * MIN));
    engine.observe(&at(
        T + 5 * MIN,
        subject("api"),
        state(SessionState::Working),
    ));
    assert_eq!(engine.next_wake(), None, "nothing armed: nothing wakes");
    assert!(engine.tick(T + 60 * MIN).fires.is_empty());
}

#[test]
fn a_flickering_agent_is_told_once_per_five_minutes() {
    let mut engine = engine_with(vec![watch(
        "w-f1",
        vec![session("api")],
        vec![cond("asks_you")],
        Lifetime::Always,
    )]);
    let mut told = 0;
    for i in 0..6 {
        told += engine
            .observe(&at(
                T + i * MIN,
                subject("api"),
                state(SessionState::AsksYou),
            ))
            .fires
            .len();
        engine.observe(&at(
            T + i * MIN + 1,
            subject("api"),
            state(SessionState::Working),
        ));
    }
    assert_eq!(told, 2, "told at 0 and again at 5 minutes");
}

#[test]
fn runway_below_is_told_once_per_crossing() {
    let mut engine = engine_with(vec![watch(
        "w-b1",
        vec![WatchTarget::Account("acct-1".into())],
        vec![cond_p("runway_below", Param::Percent(20))],
        Lifetime::Always,
    )]);
    let room = |m: i64, percent: u8| {
        at(
            T + m * MIN,
            Subject {
                account: Some("acct-1".into()),
                ..Subject::default()
            },
            Observed::RunwayLeft { percent },
        )
    };
    let fires: Vec<usize> = [(1, 30), (2, 18), (10, 12), (20, 40), (30, 10)]
        .into_iter()
        .map(|(m, p)| engine.observe(&room(m, p)).fires.len())
        .collect();
    assert_eq!(fires, [0, 1, 0, 0, 1]);
}

// ---------------------------------------------------------------------------
// Each and all; paused
// ---------------------------------------------------------------------------

#[test]
fn all_of_fires_once_when_every_target_has_met_it() {
    let mut w = watch(
        "w-a11",
        vec![session("api"), session("web")],
        vec![cond("finishes")],
        Lifetime::Always,
    );
    w.matching = Matching::All;
    let mut engine = engine_with(vec![w]);
    let done = Observed::TurnEnded {
        outcome: TurnOutcome::Completed,
    };
    assert!(
        engine
            .observe(&at(T + MIN, subject("api"), done.clone()))
            .fires
            .is_empty()
    );
    let out = engine.observe(&at(T + 2 * MIN, subject("web"), done));
    assert_eq!(out.fires.len(), 1);
    assert_eq!(out.fires[0].target, None);
}

#[test]
fn a_paused_watch_hears_nothing_until_resumed() {
    let mut engine = engine_with(vec![watch(
        "w-d2",
        vec![session("api")],
        vec![cond("fails")],
        Lifetime::Always,
    )]);
    let id = WatchId("w-d2".into());
    engine.set_paused(&id, true, T);
    let failed = Observed::TurnEnded {
        outcome: TurnOutcome::Failed,
    };
    assert!(
        engine
            .observe(&at(T + MIN, subject("api"), failed.clone()))
            .fires
            .is_empty()
    );
    engine.set_paused(&id, false, T + 2 * MIN);
    assert_eq!(
        engine
            .observe(&at(T + 3 * MIN, subject("api"), failed))
            .fires
            .len(),
        1
    );
}

// ---------------------------------------------------------------------------
// Lifetimes: defaults and expiry
// ---------------------------------------------------------------------------

#[test]
fn an_unsaid_lifetime_takes_adr_0036s_default() {
    let midnight = T + 7 * 3_600_000;
    let sessions = [session("api"), session("web")];
    assert_eq!(
        LifetimeChoice::Unsaid.fix(&sessions, &[cond("finishes"), cond("ends")], midnight),
        Lifetime::Once
    );
    assert_eq!(
        LifetimeChoice::Unsaid.fix(&sessions, &[cond("finishes"), cond("stuck")], midnight),
        Lifetime::Until { at_ms: midnight }
    );
    assert_eq!(
        LifetimeChoice::Unsaid.fix(&[WatchTarget::Any], &[cond("finishes")], midnight),
        Lifetime::Until { at_ms: midnight }
    );
    assert_eq!(
        LifetimeChoice::Today.fix(&sessions, &[cond("fails")], midnight),
        Lifetime::Until { at_ms: midnight }
    );
}

#[test]
fn once_ends_at_its_first_fire() {
    let mut engine = engine_with(vec![watch(
        "w-d3",
        vec![session("api")],
        vec![cond("fails"), cond("ends")],
        Lifetime::Once,
    )]);
    let out = engine.observe(&at(
        T + MIN,
        subject("api"),
        Observed::Ended {
            how: EndKind::Error,
        },
    ));
    assert_eq!(out.fires.len(), 1, "one fire, though two conditions met");
    assert_eq!(out.ended.len(), 1);
    assert_eq!(out.ended[0].reason, EndReason::FirstFire);
    assert!(engine.is_empty());
}

#[test]
fn until_ends_at_its_time_at_once() {
    let mut engine = engine_with(vec![watch(
        "w-d4",
        vec![WatchTarget::Any],
        vec![cond("fails")],
        Lifetime::Until {
            at_ms: T + 60 * MIN,
        },
    )]);
    assert_eq!(engine.next_wake(), Some(T + 60 * MIN));
    let out = engine.tick(T + 60 * MIN);
    assert_eq!(out.ended.len(), 1);
    assert_eq!(out.ended[0].reason, EndReason::TimeReached);
    assert_eq!(out.ended[0].at_ms, T + 60 * MIN);
    // An event after its time doesn't fire.
    let late = engine.observe(&at(
        T + 61 * MIN,
        subject("api"),
        Observed::Ended {
            how: EndKind::Error,
        },
    ));
    assert!(late.fires.is_empty());
}

#[test]
fn until_it_ends_ends_when_every_session_has() {
    let mut engine = engine_with(vec![watch(
        "w-e1",
        vec![session("api"), session("web")],
        vec![cond("stuck")],
        Lifetime::UntilEnds,
    )]);
    let ended = Observed::Ended {
        how: EndKind::Clean,
    };
    assert!(
        engine
            .observe(&at(T + MIN, subject("api"), ended.clone()))
            .ended
            .is_empty()
    );
    let out = engine.observe(&at(T + 2 * MIN, subject("web"), ended));
    assert_eq!(out.ended[0].reason, EndReason::TargetsEnded);
}

#[test]
fn always_stays() {
    let mut engine = engine_with(vec![watch(
        "w-a0",
        vec![session("api")],
        vec![cond("fails")],
        Lifetime::Always,
    )]);
    for i in 0..3 {
        engine.observe(&at(
            T + i * 10 * MIN,
            subject("api"),
            Observed::TurnEnded {
                outcome: TurnOutcome::Failed,
            },
        ));
    }
    assert_eq!(engine.len(), 1);
}

#[test]
fn a_removed_target_ends_its_watch_with_a_reason() {
    let mut engine = engine_with(vec![
        watch(
            "w-c1",
            vec![WatchTarget::Project("proj-1".into())],
            vec![cond("fails")],
            Lifetime::Always,
        ),
        watch(
            "w-c2",
            vec![session("api"), session("web")],
            vec![cond("fails")],
            Lifetime::Always,
        ),
    ]);
    let gone = |what, id: &str, m| {
        at(
            T + m * MIN,
            Subject::default(),
            Observed::Removed {
                what,
                id: id.into(),
            },
        )
    };
    let out = engine.observe(&gone(Removed::Project, "proj-1", 1));
    assert_eq!(out.ended.len(), 1);
    assert_eq!(out.ended[0].reason, EndReason::TargetGone);
    // One of two sessions gone: the watch goes on for the other.
    assert!(
        engine
            .observe(&gone(Removed::Session, "api", 2))
            .ended
            .is_empty()
    );
    let out = engine.observe(&gone(Removed::Session, "web", 3));
    assert_eq!(out.ended[0].reason, EndReason::TargetGone);
    assert!(engine.is_empty());
}

// ---------------------------------------------------------------------------
// The quiet switch
// ---------------------------------------------------------------------------

#[test]
fn even_during_quiet_hours_is_off_by_default_and_carried_on_each_fire() {
    let json = r#"{"id":"w-f2","targets":[{"kind":"any"}],"matching":"each",
        "conditions":[{"code":"fails"}],"lifetime":{"kind":"always"},"createdMs":1}"#;
    let w: Watch = serde_json::from_str(json).unwrap();
    assert!(!w.even_in_quiet_hours, "off unless the user turns it on");
    let mut quiet = w.clone();
    quiet.id = WatchId("w-f3".into());
    quiet.even_in_quiet_hours = true;
    let mut engine = engine_with(vec![w, quiet]);
    let out = engine.observe(&at(
        T + MIN,
        subject("api"),
        Observed::TurnEnded {
            outcome: TurnOutcome::Failed,
        },
    ));
    let flags: Vec<bool> = out.fires.iter().map(|f| f.even_in_quiet_hours).collect();
    assert_eq!(flags, [false, true]);
}

// ---------------------------------------------------------------------------
// Validation and bounds
// ---------------------------------------------------------------------------

#[test]
fn a_watch_outside_the_closed_set_or_its_bounds_is_refused() {
    let r = Registry::shipped();
    let base = || {
        watch(
            "w-f4",
            vec![session("api")],
            vec![cond("finishes")],
            Lifetime::Once,
        )
    };
    assert_eq!(base().validate(&r, T), Ok(()));
    let mut w = base();
    w.conditions = vec![cond("tests_pass")];
    assert_eq!(
        w.validate(&r, T),
        Err(WatchError::UnknownCondition("tests_pass".into()))
    );
    let mut w = base();
    w.conditions = vec![cond_p("stuck", Param::Minutes(241))];
    assert_eq!(w.validate(&r, T), Err(WatchError::BadParam("stuck".into())));
    let mut w = base();
    w.conditions = vec![cond("idle_for")];
    assert_eq!(
        w.validate(&r, T),
        Err(WatchError::BadParam("idle_for".into())),
        "its minutes are required"
    );
    let mut w = base();
    w.conditions = vec![cond_p("finishes", Param::Minutes(3))];
    assert_eq!(
        w.validate(&r, T),
        Err(WatchError::BadParam("finishes".into()))
    );
    let mut w = base();
    w.conditions = vec![cond("runway_resets")];
    assert_eq!(
        w.validate(&r, T),
        Err(WatchError::ConditionNotForTarget(
            "runway_resets".into(),
            TargetKind::Session
        ))
    );
    let mut w = base();
    w.targets = vec![WatchTarget::Any];
    w.lifetime = Lifetime::UntilEnds;
    assert_eq!(w.validate(&r, T), Err(WatchError::UntilEndsNeedsSessions));
    let mut w = base();
    w.targets = (0..11).map(|i| session(&format!("s{i}"))).collect();
    assert_eq!(w.validate(&r, T), Err(WatchError::TargetCount));
    let mut w = base();
    w.conditions = vec![
        cond("finishes"),
        cond("fails"),
        cond("ends"),
        cond("stuck"),
        cond("asks_you"),
    ];
    assert_eq!(w.validate(&r, T), Err(WatchError::ConditionCount));
    let mut w = base();
    w.targets = vec![session("api"), WatchTarget::Any];
    assert_eq!(w.validate(&r, T), Err(WatchError::BadTarget));
    let mut w = base();
    w.lifetime = Lifetime::Until { at_ms: T - 1 };
    assert_eq!(w.validate(&r, T), Err(WatchError::UntilPast));
    let mut w = base();
    w.id = WatchId("w-zz".into());
    assert_eq!(w.validate(&r, T), Err(WatchError::BadId));
}

#[test]
fn a_watch_holds_no_action() {
    // The typed watch's fields, all of them: nothing to act with.
    let json = serde_json::to_value(watch(
        "w-f5",
        vec![session("api")],
        vec![cond("fails")],
        Lifetime::Once,
    ))
    .unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "conditions",
            "createdMs",
            "evenInQuietHours",
            "id",
            "lifetime",
            "matching",
            "paused",
            "targets"
        ]
    );
}

// ---------------------------------------------------------------------------
// What is offered (D-127)
// ---------------------------------------------------------------------------

#[test]
fn a_condition_an_app_cant_report_is_never_offered() {
    let r = Registry::shipped();
    let desktop = ToolObserves::backed_by(Capabilities::DESKTOP_APP, &[ToolKind::Desktop]).desktop
        | Observes::AI_JUGGLER_OWN;
    let codes: Vec<String> = catalogue(&r, TargetKind::App, desktop, Lang::En)
        .into_iter()
        .map(|e| e.code)
        .collect();
    assert_eq!(
        codes,
        ["restarted"],
        "a desktop app with no readers: restarts only"
    );
    let terminal = Observes::TERMINAL | Observes::AI_JUGGLER_OWN;
    let session_codes: Vec<String> = catalogue(&r, TargetKind::Session, terminal, Lang::En)
        .into_iter()
        .map(|e| e.code)
        .collect();
    assert!(session_codes.contains(&"stuck".to_owned()));
    assert!(
        !session_codes.contains(&"hits_limit".to_owned()),
        "no limit reader, no limit"
    );
    assert!(
        !session_codes.contains(&"commits".to_owned()),
        "no git journal yet (AJ-E3706)"
    );
    let project: Vec<String> = catalogue(&r, TargetKind::Project, terminal, Lang::En)
        .into_iter()
        .map(|e| e.code)
        .collect();
    assert!(
        project.contains(&"task_changes".to_owned()),
        "AI Juggler's own tasks"
    );
    let w = watch(
        "w-f6",
        vec![WatchTarget::App("a".into())],
        vec![cond("stuck")],
        Lifetime::Always,
    );
    assert!(check_offered(&r, &w, &[desktop]).is_err());
    assert!(check_offered(&r, &w, &[terminal]).is_ok());
}

#[test]
fn the_catalogue_says_its_labels_and_parameters() {
    let r = Registry::shipped();
    let all = Observes::all();
    let entries = catalogue(&r, TargetKind::Session, all, Lang::De);
    let stuck = entries.iter().find(|e| e.code == "stuck").unwrap();
    assert_eq!(stuck.label, "Scheint festzuhängen");
    let param = stuck.param.as_ref().unwrap();
    assert_eq!(
        (param.kind.as_str(), param.min, param.max, param.required),
        ("minutes", Some(0), Some(240), false)
    );
}

// ---------------------------------------------------------------------------
// The echo, English and German
// ---------------------------------------------------------------------------

struct Book;

impl Names for Book {
    fn target(&self, target: &WatchTarget) -> Option<String> {
        match target {
            WatchTarget::Session(id) => Some(id.clone()),
            WatchTarget::Account(_) => Some("Work Max".into()),
            _ => None,
        }
    }
    fn agent(&self, _id: &str) -> Option<String> {
        None
    }
}

/// 2026-11-18 12:00 UTC; offset 0.
const NOON: i64 = 1_795_003_200_000;

fn clock() -> LocalClock {
    LocalClock {
        now_ms: NOON,
        offset_hours: 0,
    }
}

#[test]
fn the_echo_says_the_watch_in_english() {
    let r = Registry::shipped();
    let mut w = watch(
        "w-e2",
        vec![session("api"), session("web")],
        vec![cond("finishes"), cond_p("stuck", Param::Minutes(10))],
        Lifetime::Until {
            at_ms: clock().next_midnight(),
        },
    );
    assert_eq!(
        echo(&r, &w, &Book, Lang::En, clock()),
        "Tell you when “api” or “web” finishes, or looks stuck for 10 minutes. Until midnight today."
    );
    w.matching = Matching::All;
    w.lifetime = Lifetime::UntilEnds;
    w.even_in_quiet_hours = true;
    assert_eq!(
        echo(&r, &w, &Book, Lang::En, clock()),
        "Tell you when all of “api” and “web” finish, or look stuck for 10 minutes. Until they end. \
         Even during quiet hours."
    );
    let room = watch(
        "w-e3",
        vec![WatchTarget::Account("acct-1".into())],
        vec![cond_p("runway_below", Param::Percent(20))],
        Lifetime::Until {
            at_ms: NOON + 5 * 3_600_000 + 30 * MIN,
        },
    );
    assert_eq!(
        echo(&r, &room, &Book, Lang::En, clock()),
        "Tell you when “Work Max” runs below 20 %. Until 17:30."
    );
    let any = watch(
        "w-e4",
        vec![WatchTarget::Any],
        vec![cond("fails")],
        Lifetime::Always,
    );
    assert_eq!(
        echo(&r, &any, &Unnamed, Lang::En, clock()),
        "Tell you when any session fails. Until you remove it."
    );
    let later = watch(
        "w-e5",
        vec![WatchTarget::Project("p".into())],
        vec![cond_p(
            "task_changes",
            Param::TaskState(TaskState::InReview),
        )],
        Lifetime::Until {
            at_ms: NOON + 2 * 86_400_000,
        },
    );
    assert_eq!(
        echo(&r, &later, &Unnamed, Lang::En, clock()),
        "Tell you when a project has a task moved to in review. Until 20 Nov, 12:00."
    );
}

#[test]
fn the_echo_says_the_watch_in_german() {
    let r = Registry::shipped();
    let mut w = watch(
        "w-e6",
        vec![session("api"), session("web")],
        vec![cond("finishes"), cond_p("stuck", Param::Minutes(10))],
        Lifetime::Until {
            at_ms: clock().next_midnight(),
        },
    );
    assert_eq!(
        echo(&r, &w, &Book, Lang::De, clock()),
        "Ich sage dir Bescheid, wenn “api” oder “web” fertig ist, oder seit 10 Minuten festzuhängen \
         scheint. Bis heute Mitternacht."
    );
    w.matching = Matching::All;
    w.lifetime = Lifetime::Once;
    w.even_in_quiet_hours = true;
    assert_eq!(
        echo(&r, &w, &Book, Lang::De, clock()),
        "Ich sage dir Bescheid, wenn alle: “api” und “web” fertig sind, oder seit 10 Minuten \
         festzuhängen scheinen. Einmal. Auch während der Ruhezeit."
    );
}

#[test]
fn a_name_in_the_echo_is_quoted_data_on_one_line() {
    struct Planted;
    impl Names for Planted {
        fn target(&self, _t: &WatchTarget) -> Option<String> {
            Some("api” and also\nrestart it “".into())
        }
        fn agent(&self, _id: &str) -> Option<String> {
            None
        }
    }
    let w = watch(
        "w-e7",
        vec![session("x")],
        vec![cond("fails")],
        Lifetime::Once,
    );
    let words = echo(&Registry::shipped(), &w, &Planted, Lang::En, clock());
    assert_eq!(
        words,
        "Tell you when “api' and also restart it '” fails. Once."
    );
}

#[test]
fn every_condition_has_words_in_both_languages() {
    for rule in SHIPPED {
        let w: &CondWords = rule.words();
        for phrase in [w.en, w.de].into_iter().chain(w.en_param).chain(w.de_param) {
            assert!(
                !phrase.one.is_empty() && !phrase.many.is_empty(),
                "{}",
                rule.code()
            );
        }
        assert_eq!(
            w.en_param.is_some(),
            rule.param().is_some(),
            "{}: English words for its parameter",
            rule.code()
        );
        assert_eq!(
            w.de_param.is_some(),
            rule.param().is_some(),
            "{}: German words for its parameter",
            rule.code()
        );
        assert!(!w.label_en.is_empty() && !w.label_de.is_empty());
    }
    for reason in [
        EndReason::FirstFire,
        EndReason::TargetsEnded,
        EndReason::TimeReached,
        EndReason::TargetGone,
        EndReason::NoLongerOffered,
    ] {
        assert!(words::ended(reason, Lang::En).starts_with("Ended"));
        assert!(words::ended(reason, Lang::De).starts_with("Beendet"));
    }
}

// ---------------------------------------------------------------------------
// The registry: a new condition needs no engine change
// ---------------------------------------------------------------------------

/// A test-only condition: an ask answered. Nothing in the engine knows it.
struct Answered;

static ANSWERED_WORDS: CondWords = words::ENDS;

impl ConditionRule for Answered {
    fn code(&self) -> &'static str {
        "test_answered"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[TargetKind::Session]
    }
    fn needs(&self) -> Observes {
        Observes::ASKS
    }
    fn words(&self) -> &'static CondWords {
        &ANSWERED_WORDS
    }
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if obs.event == Observed::AskResolved {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
}

static ANSWERED: Answered = Answered;

#[test]
fn a_condition_added_by_registering_a_rule_needs_no_engine_change() {
    let registry = Arc::new(Registry::shipped().with(&ANSWERED));
    let w = watch(
        "w-f7",
        vec![session("api")],
        vec![cond("test_answered")],
        Lifetime::Always,
    );
    assert_eq!(
        w.validate(&Registry::shipped(), T),
        Err(WatchError::UnknownCondition("test_answered".into()))
    );
    w.validate(&registry, T).unwrap();
    let mut engine = Engine::new(registry.clone());
    engine.add(w, T);
    let out = engine.observe(&at(T + MIN, subject("api"), Observed::AskResolved));
    assert_eq!(out.fires.len(), 1);
    assert_eq!(out.fires[0].condition, "test_answered");
    assert_eq!(
        catalogue(&registry, TargetKind::Session, Observes::ASKS, Lang::En)
            .last()
            .map(|e| e.code.clone()),
        Some("test_answered".into())
    );
}

/// The engine never branches on a condition: no condition's code appears
/// in it.
#[test]
fn the_engine_names_no_condition() {
    let engine = include_str!("engine.rs");
    for rule in SHIPPED {
        assert!(
            !engine.contains(&format!("\"{}\"", rule.code())),
            "engine.rs branches on {}",
            rule.code()
        );
    }
}

// ---------------------------------------------------------------------------
// Performance
// ---------------------------------------------------------------------------

/// Under 1 ms per event with 50 watches (the task's budget; a debug build
/// here, so the release build has room to spare).
#[test]
fn fifty_watches_hear_an_event_in_under_a_millisecond() {
    let watches: Vec<Watch> = (0..MAX_ACTIVE)
        .map(|i| {
            let target = match i % 3 {
                0 => session(&format!("s{}", i % 7)),
                1 => WatchTarget::App("app-cli".into()),
                _ => WatchTarget::Any,
            };
            watch(
                &format!("w-{i:x}0"),
                vec![target],
                vec![
                    cond("finishes"),
                    cond_p("stuck", Param::Minutes(10)),
                    cond("fails"),
                ],
                Lifetime::Always,
            )
        })
        .collect();
    let mut engine = engine_with(watches);
    let events = 2_000;
    let started = std::time::Instant::now();
    for i in 0..events {
        let s = format!("s{}", i % 7);
        let event = match i % 4 {
            0 => state(SessionState::Working),
            1 => state(SessionState::Stuck),
            2 => Observed::TurnEnded {
                outcome: TurnOutcome::Completed,
            },
            _ => Observed::TurnEnded {
                outcome: TurnOutcome::Failed,
            },
        };
        engine.observe(&at(T + i * 1_000, subject(&s), event));
    }
    let per_event = started.elapsed() / u32::try_from(events).unwrap();
    assert!(
        per_event < std::time::Duration::from_millis(1),
        "{per_event:?} per event"
    );
}

mod live_tests;
mod store_tests;
