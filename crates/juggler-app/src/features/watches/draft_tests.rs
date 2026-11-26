use std::time::{Duration, Instant};

use super::*;

type Fake = crate::features::ai::scripted::Scripted;

const SESSION: [&str; 8] = [
    "finishes",
    "ends",
    "asks_you",
    "fails",
    "stuck",
    "idle_for",
    "hits_limit",
    "leaves_processes",
];

fn session(id: &str, name: &str, project: &str, app: &str) -> Candidate {
    Candidate {
        target: WatchTarget::Session(id.to_owned()),
        name: name.to_owned(),
        aliases: Vec::new(),
        app: Some(app.to_owned()),
        project: Some(project.to_owned()),
        offered: SESSION.map(str::to_owned).to_vec(),
    }
}

fn owner(target: WatchTarget, name: &str, offered: &[&str], aliases: &[&str]) -> Candidate {
    Candidate {
        target,
        name: name.to_owned(),
        aliases: aliases.iter().map(|a| (*a).to_owned()).collect(),
        app: None,
        project: None,
        offered: offered.iter().map(|o| (*o).to_owned()).collect(),
    }
}

fn candidates() -> Vec<Candidate> {
    vec![
        session("S-cedar", "cedar-api", "cedar", "Claude Code"),
        session("S-aspen", "aspen-board", "aspen", "Codex"),
        session("S-b1", "Index rebuild", "birch-docs", "Claude Code"),
        session("S-b2", "Link check", "birch-docs", "Claude Code"),
        session("S-api-c", "api", "cedar", "Claude Code"),
        session("S-api-a", "api", "aspen", "Codex"),
        owner(WatchTarget::Project("cedar".into()), "cedar", &SESSION, &[]),
        owner(WatchTarget::Project("aspen".into()), "aspen", &SESSION, &[]),
        owner(
            WatchTarget::Project("birch-docs".into()),
            "birch-docs",
            &SESSION,
            &[],
        ),
        owner(
            WatchTarget::Account("acc-w".into()),
            "work max",
            &["hits_limit", "runway_below", "runway_resets"],
            &["the work plan"],
        ),
    ]
}

fn run_with(request: &str, fake: &Fake) -> Outcome {
    let c = candidates();
    let ask = Ask {
        request,
        candidates: &c,
        midnight_ms: 0,
        now_ms: 1_000,
    };
    draft(
        fake,
        &ask,
        &Stop::at(Instant::now() + Duration::from_secs(5)),
        &|_| None,
    )
    .0
}

fn filled(outcome: &Outcome) -> Option<&WatchDraftInfo> {
    match outcome {
        Outcome::Drafted(WatchDrafted::Filled { draft }) => Some(draft),
        _ => None,
    }
}

fn codes(d: &WatchDraftInfo) -> Vec<&str> {
    d.conditions.iter().map(|c| c.code.as_str()).collect()
}

#[test]
fn the_prompts_are_the_ones_harness_v7_measured() {
    // Read from the harness at test time (the docs don't ship in the app).
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/qa/evidence/AJ-E3709/harness/v7/prompts/watch3.md"
    );
    let measured = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        measured.contains(TARGET_INSTRUCTIONS),
        "call 1's instructions drifted from the harness"
    );
    assert!(
        measured.contains(WHEN_INSTRUCTIONS),
        "call 2's instructions drifted from the harness"
    );
}

/// v6's misses, each decided by code now with no model call.
type Case = (
    &'static str,
    WatchTarget,
    &'static [&'static str],
    Option<Param>,
    LifetimeChoice,
);

#[test]
fn code_decides_the_target_conditions_numbers_and_lifetime() {
    let cases: [Case; 8] = [
        (
            "Tell me when cedar-api asks something, and ignore aspen-board entirely.",
            WatchTarget::Session("S-cedar".into()),
            &["asks_you"],
            None,
            LifetimeChoice::Unsaid,
        ),
        (
            "Ping me when cedar-api finishes or fails.",
            WatchTarget::Session("S-cedar".into()),
            &["finishes", "fails"],
            None,
            LifetimeChoice::Unsaid,
        ),
        (
            "Alert me when anything in birch-docs stops moving for 25 minutes.",
            WatchTarget::Project("birch-docs".into()),
            &["stuck"],
            Some(Param::Minutes(25)),
            LifetimeChoice::Unsaid,
        ),
        (
            "Tell me if any session is idle for an hour.",
            WatchTarget::Any,
            &["idle_for"],
            Some(Param::Minutes(60)),
            LifetimeChoice::Unsaid,
        ),
        (
            "Let me know when the work plan hits its limit.",
            WatchTarget::Account("acc-w".into()),
            &["hits_limit"],
            None,
            LifetimeChoice::Unsaid,
        ),
        (
            "Let me know when api finishes, not the aspen one.",
            WatchTarget::Session("S-api-c".into()),
            &["finishes"],
            None,
            LifetimeChoice::Unsaid,
        ),
        (
            "Today, tell me when cedar-api is done.",
            WatchTarget::Session("S-cedar".into()),
            &["finishes"],
            None,
            LifetimeChoice::Today,
        ),
        (
            "Tell me when a session in the birch-docs project gets stuck for 15 minutes.",
            WatchTarget::Project("birch-docs".into()),
            &["stuck"],
            Some(Param::Minutes(15)),
            LifetimeChoice::Unsaid,
        ),
    ];
    for (request, target, want, param, lifetime) in cases {
        let fake = Fake::new(&["FAIL"]);
        let out = run_with(request, &fake);
        let d = filled(&out).unwrap_or_else(|| unreachable!("{request}: {out:?}"));
        assert_eq!(d.targets, vec![target], "{request}");
        assert_eq!(codes(d), want, "{request}");
        assert_eq!(d.conditions[0].param, param, "{request}");
        assert_eq!(d.lifetime, lifetime, "{request}");
        assert_eq!(fake.calls(), 0, "{request}: code decided, no call");
    }
}

#[test]
fn several_candidates_the_model_picks_one_of_them_sampled_on_retry() {
    let fake = Fake::new(&[r#"{"target":"zz"}"#, r#"{"target":"s2"}"#]);
    let out = run_with("Tell me when cedar-api or aspen-board finishes.", &fake);
    assert_eq!(
        filled(&out).map(|d| d.targets.clone()),
        Some(vec![WatchTarget::Session("S-aspen".into())])
    );
    let seen = fake.seen.borrow();
    let values = &seen[0]
        .shape
        .as_ref()
        .map(|s| s["properties"][0]["schema"]["values"].clone())
        .unwrap_or_default();
    assert_eq!(
        values.as_array().map(Vec::len),
        Some(3),
        "the candidates only, and none"
    );
    assert!(
        matches!(seen[1].decoding, Decoding::Sampled { .. }),
        "D-149: a retry samples"
    );
}

#[test]
fn no_condition_word_the_model_picks_from_the_offer_with_definitions() {
    let fake = Fake::new(&[r#"{"condition":"idle_for","condition2":"none","unsupported":"none"}"#]);
    let out = run_with(
        "Let me know if cedar-api sits there doing nothing for 20 minutes.",
        &fake,
    );
    let d = filled(&out).unwrap_or_else(|| unreachable!("{out:?}"));
    assert_eq!(
        d.conditions,
        vec![Condition {
            code: "idle_for".into(),
            param: Some(Param::Minutes(20))
        }]
    );
    let seen = fake.seen.borrow();
    assert!(
        seen[0].prompt.contains("stuck · AI Juggler sees it stuck")
            && seen[0].prompt.contains("idle_for · no activity at all")
    );
    assert!(
        seen[0]
            .prompt
            .contains("hits_limit · it reaches a usage limit"),
        "the offer mirrors the app's, hits_limit included"
    );
}

#[test]
fn what_cannot_be_watched_always_gets_the_nearest() {
    for (request, asked_text, near) in [
        (
            "Tell me when the tests pass in cedar-api.",
            "tests pass",
            "finishes",
        ),
        (
            "Remind me at 5pm to look at aspen-board.",
            "at 5pm",
            "finishes",
        ),
    ] {
        let fake = Fake::new(&["FAIL"]);
        match run_with(request, &fake) {
            Outcome::Drafted(WatchDrafted::CantWatch { asked, draft }) => {
                assert_eq!(asked, asked_text, "{request}");
                assert_eq!(
                    draft.map(|d| d.conditions[0].code.clone()).as_deref(),
                    Some(near),
                    "{request}"
                );
            }
            other => unreachable!("{request}: {other:?}"),
        }
        assert_eq!(fake.calls(), 0);
    }
    // Quiet hours and a usage limit are no false alarm.
    let out = run_with(
        "Wake me even at 3am if cedar-api fails.",
        &Fake::new(&["FAIL"]),
    );
    assert!(filled(&out).is_some(), "{out:?}");
    let out = run_with(
        "Tell me when the work plan hits its limit, the costs matter.",
        &Fake::new(&["FAIL"]),
    );
    assert!(filled(&out).is_some(), "{out:?}");
}

#[test]
fn twins_nothing_settles_open_the_picker_and_not_a_watch_passes() {
    match run_with("Let me know when api finishes.", &Fake::new(&["FAIL"])) {
        Outcome::Drafted(WatchDrafted::Pick { draft }) => {
            assert!(draft.targets.is_empty());
            assert_eq!(codes(&draft), ["finishes"]);
        }
        other => unreachable!("{other:?}"),
    }
    assert!(matches!(
        run_with("When did cedar-api last finish?", &Fake::new(&["FAIL"])),
        Outcome::NotAWatch
    ));
    let c = candidates();
    let ask = Ask {
        request: "Let me know when cedar-api finishes.",
        candidates: &c,
        midnight_ms: 0,
        now_ms: 1,
    };
    let stop = Stop::at(Instant::now() + Duration::from_secs(5));
    let (out, _) = draft(&Fake::new(&["FAIL"]), &ask, &stop, &|_| {
        Some("limit".to_owned())
    });
    assert!(
        matches!(out, Outcome::Drafted(WatchDrafted::Form)),
        "the form's own refusal is never drafted past"
    );
}

#[test]
fn the_resolver_reads_exclusions_owners_aliases_and_kinds() {
    let c = candidates();
    let r = resolve::resolve("tell me when cedar-api asks, but not aspen-board", &c);
    assert_eq!(r.candidates, vec![0]);
    assert!(r.excluded.contains(&1));
    assert_eq!(
        resolve::resolve("anything in birch-docs that fails", &c).candidates,
        vec![8],
        "the project, not its sessions"
    );
    assert_eq!(
        resolve::resolve("when the work plan runs low", &c).candidates,
        vec![9],
        "an alias"
    );
    assert_eq!(resolve::resolve("when api finishes", &c).picker, vec![4, 5]);
    assert!(resolve::resolve("when any session fails", &c).any);
}

#[test]
fn the_words_code_reads() {
    assert!(is_watch_request(
        "Gib mir Bescheid, sobald deneb dich etwas fragt."
    ));
    assert!(!is_watch_request("What needs me?"));
    assert_eq!(
        cue_conditions("tell me when it finishes or errors"),
        ["finishes", "fails"]
    );
    assert_eq!(
        cue_conditions("warn me when we hit the rate limit"),
        ["hits_limit"]
    );
    assert_eq!(
        cue_conditions("tell me when the account drops below 10 percent"),
        ["runway_below"]
    );
    assert_eq!(cue_conditions("tell me once it ends up done"), ["finishes"]);
    assert_eq!(
        request_times("until 6pm or 18:30, bis 9 Uhr"),
        vec!["18:00", "18:30", "09:00"]
    );
    assert_eq!(
        param_for("drops below 20 percent", "runway_below").as_deref(),
        Some("20")
    );
}

/// v7's five "unsupported claimed" rows (in fresh words of their shapes)
/// and the thresholds: each is told back with the nearest option, never
/// drafted as a watch, and decided by code with no call.
#[test]
fn v7s_unsupported_claimed_rows_are_told_back_with_the_nearest() {
    let rows: [(&str, &str); 7] = [
        (
            "Tell me when all sessions finish, except the Link check one.",
            "finishes",
        ), // W05
        (
            "For every project except aspen, tell me if it fails.",
            "fails",
        ), // W19
        (
            "Tell me once if anything fails, but not the Link check one.",
            "fails",
        ), // W32
        (
            "Tell me when the cedar-api session uses more than 50k tokens.",
            "hits_limit",
        ), // W43
        (
            "Gib mir Bescheid, wenn alle Sitzungen fertig sind, außer Link check.",
            "finishes",
        ), // W51
        (
            "Let me know if cedar-api costs more than $20.",
            "hits_limit",
        ),
        (
            "Tell me when cedar-api has more than 30 files changed.",
            "finishes",
        ),
    ];
    for (request, near) in rows {
        let fake = Fake::new(&["FAIL"]);
        match run_with(request, &fake) {
            Outcome::Drafted(WatchDrafted::CantWatch { asked, draft }) => {
                assert!(!asked.is_empty(), "{request}");
                let d = draft.unwrap_or_else(|| unreachable!("{request}: no nearest"));
                assert_eq!(codes(&d), [near], "{request}");
                assert!(
                    d.targets.iter().all(|t| t.kind() == TargetKind::Session),
                    "{request}: never widened"
                );
            }
            other => unreachable!("{request}: {other:?}"),
        }
        assert_eq!(fake.calls(), 0, "{request}: decided by code");
    }
}

/// Never widen: whatever the model answers, a request that excludes
/// something never gets a watch on any session, a project, an App or a
/// pool.
#[test]
fn a_request_that_excludes_something_is_never_widened() {
    for request in [
        "Ping me when any session asks me something, except aspen-board.",
        "Tell me when everything in birch-docs finishes but not the Link check one.",
        "Sag mir, wenn irgendeine Sitzung fehlschlägt, außer cedar-api.",
        "Alert me if anything at all gets stuck, other than aspen-board.",
    ] {
        let fake = Fake::new(&[
            r#"{"target":"none"}"#,
            r#"{"condition":"fails","condition2":"none","unsupported":"none"}"#,
        ]);
        let out = run_with(request, &fake);
        if let Some(d) = filled(&out) {
            assert!(
                d.targets
                    .iter()
                    .all(|t| t.kind() == TargetKind::Session || t.kind() == TargetKind::Account),
                "{request}: widened to {:?}",
                d.targets
            );
        }
        if let Outcome::Drafted(WatchDrafted::CantWatch { draft, .. }) = &out {
            assert!(draft.is_some(), "{request}: the nearest is offered");
        }
    }
    // An ignore beside a named session still drafts that session.
    let out = run_with(
        "Tell me when cedar-api asks something, and ignore aspen-board entirely.",
        &Fake::new(&["FAIL"]),
    );
    assert_eq!(
        filled(&out).map(|d| d.targets.clone()),
        Some(vec![WatchTarget::Session("S-cedar".into())])
    );
    let out = run_with("Always tell me if anything fails.", &Fake::new(&["FAIL"]));
    assert_eq!(
        filled(&out).map(|d| d.lifetime),
        Some(LifetimeChoice::Always)
    );
}
