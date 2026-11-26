use std::time::{Duration, Instant};

use super::*;

type Fake = crate::features::ai::scripted::Scripted;

const ALL: [&str; 6] = ["finishes", "ends", "asks_you", "fails", "stuck", "idle_for"];

fn session(id: &str, name: &str, project: &str) -> Candidate {
    Candidate {
        target: WatchTarget::Session(id.to_owned()),
        name: name.to_owned(),
        app: Some("Claude Code".to_owned()),
        project: Some(project.to_owned()),
        offered: ALL.map(str::to_owned).to_vec(),
    }
}

fn candidates() -> Vec<Candidate> {
    vec![
        session("S-vega", "vega-sync", "vega"),
        session("S-deneb", "deneb-ui", "deneb"),
        session("S-api-a", "api", "altair"),
        session("S-api-r", "api", "rigel"),
        Candidate {
            target: WatchTarget::Account("acc-s".to_owned()),
            name: "studio max".to_owned(),
            app: None,
            project: None,
            offered: vec!["runway_below".to_owned(), "hits_limit".to_owned()],
        },
    ]
}

fn run(request: &str, fake: &Fake) -> Outcome {
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

fn filled(outcome: &Outcome) -> &WatchDraftInfo {
    match outcome {
        Outcome::Drafted(WatchDrafted::Filled { draft }) => draft,
        other => unreachable!("not filled: {other:?}"),
    }
}

#[test]
fn the_prompts_are_the_ones_harness_v6_measured() {
    // Read from the harness at test time (the docs don't ship in the app).
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/qa/evidence/AJ-E3709/harness/v6/prompts/watch2.md"
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

#[test]
fn two_calls_fill_the_form_the_target_first() {
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"s1"}"#,
        r#"{"condition":"finishes","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
    ]);
    let out = run("Let me know when vega-sync finishes.", &fake);
    let draft = filled(&out);
    assert_eq!(
        draft.targets,
        vec![WatchTarget::Session("S-vega".to_owned())]
    );
    assert_eq!(draft.conditions[0].code, "finishes");
    let seen = fake.seen.borrow();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].prompt.contains("s1 session “vega-sync”"));
    assert!(
        seen[1].prompt.starts_with("TARGET: s1 session “vega-sync”"),
        "the second call is built for the target"
    );
    let target = &seen[0].shape.as_ref().unwrap()["properties"][1]["schema"];
    assert_eq!(target["type"], "enum", "one target, never a list");
    assert_eq!(
        seen[0].decoding,
        Decoding::Tested,
        "a first attempt keeps the tested decoding"
    );
}

#[test]
fn a_target_the_request_does_not_name_is_asked_again_sampled() {
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"s2"}"#,
        r#"{"intent":"create","target":"s1"}"#,
        r#"{"condition":"stuck","param":"20","condition2":"none","param2":"none","lifetime":"today","until":"none","unsupported":"none"}"#,
    ]);
    let out = run(
        "Tell me today when vega-sync is stuck for 20 minutes.",
        &fake,
    );
    let draft = filled(&out);
    assert_eq!(draft.conditions[0].param, Some(Param::Minutes(20)));
    assert_eq!(draft.lifetime, LifetimeChoice::Today);
    let seen = fake.seen.borrow();
    assert!(
        seen[1].prompt.contains("is not named in the request"),
        "the reason is in the retry"
    );
    assert!(
        matches!(seen[1].decoding, Decoding::Sampled { .. }),
        "D-149: a retry samples"
    );
}

#[test]
fn what_cannot_be_watched_is_said_in_the_requests_words_with_the_nearest() {
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"s1"}"#,
        r#"{"condition":"none","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"tests"}"#,
    ]);
    match run("Tell me when the tests pass in vega-sync.", &fake) {
        Outcome::Drafted(WatchDrafted::CantWatch { asked, draft }) => {
            assert_eq!(asked, "tests pass");
            assert_eq!(draft.unwrap().conditions[0].code, "finishes");
        }
        other => unreachable!("{other:?}"),
    }
    // A draft that claims to watch the tests is caught by the net, never a watch.
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"s1"}"#,
        r#"{"condition":"finishes","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
    ]);
    assert!(matches!(
        run("Tell me when the tests pass in vega-sync.", &fake),
        Outcome::Drafted(WatchDrafted::CantWatch { .. })
    ));
    // A kind the request doesn't say is retried.
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"s1"}"#,
        r#"{"condition":"finishes","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"deploy"}"#,
        r#"{"condition":"finishes","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
    ]);
    filled(&run("Let me know when vega-sync finishes.", &fake));
    assert!(
        fake.seen.borrow()[2]
            .prompt
            .contains("says nothing of deploy")
    );
}

#[test]
fn a_shared_name_opens_the_form_with_its_picker_and_the_context_settles_one() {
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"s3"}"#,
        r#"{"condition":"finishes","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
    ]);
    match run("Let me know when api finishes.", &fake) {
        Outcome::Drafted(WatchDrafted::Pick { draft }) => {
            assert!(draft.targets.is_empty());
            assert_eq!(draft.conditions[0].code, "finishes");
        }
        other => unreachable!("{other:?}"),
    }
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"s4"}"#,
        r#"{"intent":"create","target":"s3"}"#,
        r#"{"condition":"finishes","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
    ]);
    let out = run("Let me know when the api in altair finishes.", &fake);
    assert_eq!(
        filled(&out).targets,
        vec![WatchTarget::Session("S-api-a".to_owned())]
    );
}

#[test]
fn not_a_watch_fails_and_engine_errors() {
    let fake = Fake::new(&[r#"{"intent":"not_a_watch","target":"none"}"#]);
    assert!(matches!(
        run("Tell me when vega-sync last finished?", &fake),
        Outcome::NotAWatch
    ));
    assert_eq!(fake.calls(), 1);
    let fake = Fake::new(&[r#"{"intent":"create","target":"zz"}"#]);
    assert!(matches!(
        run("Let me know when vega-sync finishes.", &fake),
        Outcome::Drafted(WatchDrafted::Form)
    ));
    assert_eq!(
        fake.calls(),
        3,
        "three tries of call 1, then the empty form"
    );
    let fake = Fake::new(&["FAIL"]);
    assert!(matches!(
        run("Let me know when vega-sync finishes.", &fake),
        Outcome::Drafted(WatchDrafted::Form)
    ));
}

#[test]
fn the_forms_own_check_and_parameters_are_held() {
    let c = candidates();
    let ask = Ask {
        request: "Warn me when studio max drops below 15 percent.",
        candidates: &c,
        midnight_ms: 0,
        now_ms: 1,
    };
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"acct1"}"#,
        r#"{"condition":"runway_below","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
        r#"{"condition":"runway_below","param":"15","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
    ]);
    let stop = Stop::at(Instant::now() + Duration::from_secs(5));
    let (out, calls) = draft(&fake, &ask, &stop, &|_| None);
    assert_eq!(filled(&out).conditions[0].param, Some(Param::Percent(15)));
    assert_eq!(calls, 3, "a missing percent asked again");
    assert!(fake.seen.borrow()[1].prompt.contains("NUMBERS: none, 15"));
    let fake = Fake::new(&[
        r#"{"intent":"create","target":"s1"}"#,
        r#"{"condition":"finishes","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
    ]);
    let ask = Ask {
        request: "Let me know when vega-sync finishes.",
        candidates: &c,
        midnight_ms: 0,
        now_ms: 1,
    };
    let (out, _) = draft(&fake, &ask, &stop, &|_| Some("limit".to_owned()));
    assert!(
        matches!(out, Outcome::Drafted(WatchDrafted::Form)),
        "the form's own refusal is never drafted past"
    );
}

#[test]
fn the_words_code_reads() {
    assert!(is_watch_request("Let me know when vega-sync finishes."));
    assert!(is_watch_request(
        "Gib mir Bescheid, sobald deneb dich etwas fragt."
    ));
    assert!(!is_watch_request("When did vega-sync finish?"));
    assert!(!is_watch_request("What needs me?"));
    assert_eq!(
        request_numbers("stuck for 20 minutes or an hour, below fifteen %"),
        vec![15, 20, 60]
    );
    assert_eq!(
        request_times("until 6pm or 18:30, bis 9 Uhr"),
        vec!["18:00", "18:30", "09:00"]
    );
    assert_eq!(
        asked_words("ping me when the pull request is merged", "merge").as_deref(),
        Some("merged")
    );
    assert_eq!(asked_words("tell me when it finishes", "deploy"), None);
}
