use std::time::{Duration, Instant};

use super::*;
use crate::features::ai::scripted::Scripted;
use crate::features::watches::draft::{Candidate, Outcome, draft};
use crate::features::watches::model::{Param, TargetKind, WatchTarget};

const OFFER: [&str; 8] = [
    "finishes",
    "ends",
    "asks_you",
    "fails",
    "stuck",
    "idle_for",
    "hits_limit",
    "leaves_processes",
];

fn sess(id: &str, name: &str, project: &str) -> Candidate {
    Candidate {
        target: WatchTarget::Session(id.into()),
        name: name.into(),
        aliases: Vec::new(),
        app: Some("Codex".into()),
        project: Some(project.into()),
        offered: OFFER.map(str::to_owned).to_vec(),
    }
}

fn owner(target: WatchTarget, name: &str, offered: &[&str]) -> Candidate {
    Candidate {
        target,
        name: name.into(),
        aliases: Vec::new(),
        app: None,
        project: None,
        offered: offered.iter().map(|o| (*o).to_owned()).collect(),
    }
}

/// v10's names (its misses are quoted in `v10-results/summary.md`; the
/// rest of the set stays unread), in a made-up context.
fn v10_candidates() -> Vec<Candidate> {
    let p = |name: &str| owner(WatchTarget::Project(format!("p-{name}")), name, &OFFER);
    vec![
        sess("S-ws", "Wetterstation-Sync", "wetter"),
        sess("S-pay", "Payment flow", "shop"),
        sess("S-bill", "billing-sync", "billing"),
        sess("S-dash", "dashboard filters", "dash"),
        sess("S-ing", "Ingest worker", "pipeline"),
        sess("S-lan", "Landing page copy", "site"),
        sess("S-chk", "checkout-api", "shop"),
        sess("S-scan", "scan deskew", "scanner"),
        sess("S-emb", "Embedding refresh", "search"),
        p("shop"),
        p("checkout-api"),
        p("billing-sync"),
    ]
}

fn run_on(c: &[Candidate], request: &str, model: &Scripted) -> Outcome {
    let ask = Ask {
        request,
        candidates: c,
        midnight_ms: 0,
        now_ms: 1_000,
    };
    draft(
        model,
        &ask,
        &Stop::at(Instant::now() + Duration::from_secs(5)),
        &|_| None,
    )
    .0
}

fn run(request: &str, model: &Scripted) -> Outcome {
    run_on(&v10_candidates(), request, model)
}

fn run11(request: &str, model: &Scripted) -> Outcome {
    run_on(&v11_candidates(), request, model)
}

/// Each row drafted as said, by code alone.
fn assert_said(c: &[Candidate], rows: &[Said]) {
    for (request, target, code, param, lifetime, quiet) in rows {
        let fake = Scripted::new(&["FAIL"]);
        let d = match run_on(c, request, &fake) {
            Outcome::Drafted(WatchDrafted::Filled { draft }) => draft,
            other => unreachable!("{request}: {other:?}"),
        };
        assert_eq!(d.targets, vec![target.clone()], "{request}");
        assert_eq!(d.conditions.len(), 1, "{request}: {:?}", d.conditions);
        assert_eq!(d.conditions[0].code, *code, "{request}");
        assert_eq!(d.conditions[0].param, *param, "{request}");
        assert_eq!(d.lifetime, *lifetime, "{request}");
        assert_eq!(d.even_in_quiet_hours, *quiet, "{request}");
        assert_eq!(fake.calls(), 0, "{request}: code's alone");
    }
}

fn told_back(request: &str, out: Outcome) -> (String, WatchDraftInfo) {
    match out {
        Outcome::Drafted(WatchDrafted::CantWatch {
            asked,
            draft: Some(d),
        }) if !d.conditions.is_empty() => (asked, d),
        other => unreachable!("{request}: {other:?}"),
    }
}

/// v10's three unsupported-claimed misses and its two with no nearest:
/// each told back with the nearest, the model's labels or none.
#[test]
fn v10s_misses_are_told_back_with_the_nearest() {
    let rows = [
        (
            "Warne mich bei Fehlern, aber nur bei solchen, die nicht von der Wetterstation kommen",
            "fails",
        ),
        (
            "Tell me when the payment flow finishes unless it ran for under an hour",
            "finishes",
        ),
        (
            "Ping me if billing-sync has made 8 failed retries in a row",
            "fails",
        ),
        (
            "Tell me when the dashboard filters session logs grow past 500 megabytes",
            "finishes",
        ),
        (
            "Notify me about failures, but only the ones that aren't timeouts",
            "fails",
        ),
    ];
    for (request, near) in rows {
        let (asked, d) = told_back(request, run(request, &Scripted::new(&["FAIL"])));
        assert!(!asked.is_empty(), "{request}");
        assert_eq!(d.conditions[0].code, near, "{request}");
        assert!(
            d.targets.iter().all(|t| t.kind() == TargetKind::Session),
            "{request}: never widened"
        );
        assert!(
            !d.targets.contains(&WatchTarget::Session("S-ws".into())),
            "{request}: never what it leaves out"
        );
    }
    let (asked, _) = told_back(
        "x",
        run(
            "Tell me when the payment flow finishes unless it ran for under an hour",
            &Scripted::new(&["FAIL"]),
        ),
    );
    assert_eq!(asked, "unless it ran for under an hour");
}

/// A request, and its draft: target, condition, number, lifetime, quiet.
type Said = (
    &'static str,
    WatchTarget,
    &'static str,
    Option<Param>,
    LifetimeChoice,
    bool,
);

/// v10's exact misses, each now as said.
#[test]
fn v10s_supported_misses_draft_as_said() {
    let any = WatchTarget::Any;
    let s = |id: &str| WatchTarget::Session(id.into());
    let rows: [Said; 8] = [
        (
            "From now on, tell me when the ingest worker asks me anything",
            s("S-ing"),
            "asks_you",
            None,
            LifetimeChoice::Always,
            false,
        ),
        (
            "Every time the landing page copy session finishes, let me know",
            s("S-lan"),
            "finishes",
            None,
            LifetimeChoice::Always,
            false,
        ),
        (
            "Tell me if any session looks stuck for half an hour",
            any,
            "stuck",
            Some(Param::Minutes(30)),
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Wake me up, even during quiet hours, if checkout-api fails",
            s("S-chk"),
            "fails",
            None,
            LifetimeChoice::Unsaid,
            true,
        ),
        (
            "Of all my sessions, ping me when checkout-api fails",
            s("S-chk"),
            "fails",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Any time the billing-sync session asks me something, tell me",
            s("S-bill"),
            "asks_you",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Tell me when the embedding refresh is done",
            s("S-emb"),
            "finishes",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Tell me when the scan deskew session finishes, but hold it until quiet hours end",
            s("S-scan"),
            "finishes",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
    ];
    assert_said(&v10_candidates(), &rows);
}

/// The model labels; whatever it says, a request with words code can't
/// place is never drafted, and a label's span names what is told back.
#[test]
fn the_labels_never_make_a_request_supported() {
    let request = "Ping me if billing-sync has made 8 failed retries in a row";
    // A liar: every word filler, then every word a condition.
    for lie in [
        r#"{"spans":[{"text":"Ping me if billing-sync has made 8 failed retries in a row","slot":"filler","number":0,"unit":"none"}]}"#,
        r#"{"spans":[{"text":"Ping me if","slot":"filler","number":0,"unit":"none"},{"text":"billing-sync","slot":"target","number":0,"unit":"none"},{"text":"has made 8 failed retries in a row","slot":"condition","number":8,"unit":"other"}]}"#,
    ] {
        let fake = Scripted::new(&[lie]);
        let (asked, d) = told_back(request, run(request, &fake));
        assert_eq!(d.conditions[0].code, "fails");
        assert!(!asked.is_empty());
    }
    // An honest labeller: its other span is what is told back.
    let fake = Scripted::new(&[
        r#"{"spans":[{"text":"Ping me if","slot":"filler","number":0,"unit":"none"},{"text":"billing-sync","slot":"target","number":0,"unit":"none"},{"text":"has made 8 failed retries in a row","slot":"other","number":8,"unit":"other"}]}"#,
    ]);
    let (asked, d) = told_back(request, run(request, &fake));
    assert_eq!(asked, "has made 8 failed retries in a row");
    assert_eq!(d.targets, vec![WatchTarget::Session("S-bill".into())]);
    // Spans that don't repeat the request: asked again, then code's own.
    let fake = Scripted::new(&[
        r#"{"spans":[{"text":"Ping me when billing sync fails","slot":"filler","number":0,"unit":"none"}]}"#,
    ]);
    let (asked, _) = told_back(request, run(request, &fake));
    assert_eq!(fake.calls(), 3, "the first try and two retries");
    assert_eq!(asked, "made 8 failed retries in a row");
    let seen = fake.seen.borrow();
    assert!(
        seen[1].prompt.contains("word for word"),
        "the retry says why"
    );
    assert_eq!(seen[0].shape, Some(label_shape()));
}

/// An exclusion's span: what it names is neither the nearest's target nor
/// its condition's source.
#[test]
fn an_exclusion_label_keeps_what_it_names_out_of_the_nearest() {
    let request = "Tell me when the payment flow fails, leaving out the checkout-api one";
    let fake = Scripted::new(&[
        r#"{"spans":[{"text":"Tell me when","slot":"filler","number":0,"unit":"none"},{"text":"the payment flow","slot":"target","number":0,"unit":"none"},{"text":"fails","slot":"condition","number":0,"unit":"none"},{"text":"leaving out the checkout-api one","slot":"exclusion","number":0,"unit":"none"}]}"#,
    ]);
    let (asked, d) = told_back(request, run(request, &fake));
    assert_eq!(asked, "leaving out the checkout-api one");
    assert!(d.targets.is_empty(), "the form asks: {:?}", d.targets);
    assert_eq!(d.conditions[0].code, "fails");
}

/// v12's negation guard, before any label: v10's W03 is told back from
/// its "nicht", with no call.
#[test]
fn the_negation_guard_needs_no_label() {
    let request =
        "Warne mich bei Fehlern, aber nur bei solchen, die nicht von der Wetterstation kommen";
    let fake = Scripted::new(&["FAIL"]);
    let (asked, d) = told_back(request, run(request, &fake));
    assert_eq!(asked, "nicht von der Wetterstation kommen");
    assert!(d.targets.is_empty());
    assert_eq!(d.conditions[0].code, "fails");
    assert_eq!(fake.calls(), 0);
}

/// The invariant: an answer that something can't be watched always comes
/// with the nearest option, however little the request names.
#[test]
fn what_cannot_be_watched_always_comes_with_the_nearest() {
    for request in [
        "Tell me when the moon is full",
        "Ping me if anything happens at 5pm",
        "Let me know when the deploy is out, except on Fridays",
        "Sag mir Bescheid, wenn die Tests grün sind",
        "Tell me when payment flow fails twice, but not the checkout-api one",
        "Warn me when more than 3 sessions are stuck",
        "Tell me when billing-sync finishes after 10 minutes",
        "Alert me when the Payment flow session uses 20% of its budget",
        "Notify me about failures in sessions I didn't start",
        "Remind me at 9 to check the ingest worker",
    ] {
        for model in [Scripted::new(&["FAIL"]), Scripted::new(&["{}"])] {
            let out = run(request, &model);
            if let Outcome::Drafted(WatchDrafted::CantWatch { draft, .. }) = &out {
                assert!(
                    draft.as_ref().is_some_and(|d| !d.conditions.is_empty()),
                    "{request}: an unsupported answer with no nearest is a bug"
                );
            }
            assert!(
                !matches!(out, Outcome::Drafted(WatchDrafted::Filled { .. })),
                "{request}: {out:?}"
            );
        }
    }
}

/// Code's reading: each word placed, numbers only with their units.
#[test]
fn code_places_every_word_or_claims_nothing() {
    let c = v10_candidates();
    let ask = |request| Ask {
        request,
        candidates: &c,
        midnight_ms: 0,
        now_ms: 0,
    };
    let a = ask("Tell me if the ingest worker is stuck for more than 20 minutes, from now on");
    let r = read(&a);
    assert_eq!(r.cover(&a), Cover::Claimed);
    assert_eq!(r.params, vec![(8, 12, Unit::Minutes, 20)]);
    let a = ask("Tell me when billing-sync finishes after 10 minutes");
    assert_eq!(
        read(&a).cover(&a),
        Cover::Untaken("after 10 minutes".into())
    );
    let a = ask("Tell me when checkout-api fails 3 times");
    assert!(matches!(read(&a).cover(&a), Cover::Leftover(_)));
    let a = ask("Keep an eye on checkout-api");
    assert_eq!(read(&a).cover(&a), Cover::NoCondition);
    assert_eq!(
        tokens("Drops below 20% today."),
        ["drops", "below", "20", "percent", "today"]
    );
}

/// v11's names (its misses are quoted in `v11-results/summary.md`), in a
/// made-up context.
fn v11_candidates() -> Vec<Candidate> {
    let mut project = OFFER.to_vec();
    project.push("starts");
    vec![
        sess("s-pine", "Flag parsing", "cli"),
        sess("s-wetter", "Sensordaten", "wetter"),
        sess("s-tide", "Axis labels", "charts"),
        sess("s-bahn", "Fahrplan-Import", "bahn"),
        sess("s-dach", "Terminvergabe", "praxis"),
        sess("s-orbit", "Timezone fix", "orbit"),
        sess("s-img", "Image resizer", "media"),
        sess("s-flaky", "flaky tests", "ci"),
        owner(
            WatchTarget::Project("p-pilz".into()),
            "pilzfinder",
            &project,
        ),
        owner(
            WatchTarget::Project("p-quiz".into()),
            "quiz-engine",
            &project,
        ),
        owner(
            WatchTarget::Project("p-sage".into()),
            "sage-billing",
            &project,
        ),
        owner(
            WatchTarget::Project("p-lager".into()),
            "lagerhaus",
            &project,
        ),
        owner(
            WatchTarget::App("app-cur".into()),
            "Cursor",
            &["finishes", "fails", "restarted", "starts", "hits_limit"],
        ),
    ]
}

/// v11's exact misses, each drafted as gold by code alone.
#[test]
fn v11s_misses_draft_as_said() {
    let s = |id: &str| WatchTarget::Session(id.into());
    let p = |id: &str| WatchTarget::Project(id.into());
    let rows: [Said; 13] = [
        (
            "Ping me if the Flag parsing session sits idle for 45 minutes",
            s("s-pine"),
            "idle_for",
            Some(Param::Minutes(45)),
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Egal was sonst läuft, informiere mich, wenn in pilzfinder eine neue Sitzung startet",
            p("p-pilz"),
            "starts",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Von allen meinen Sitzungen: sag mir nur Bescheid, wenn die Sensordaten-Sitzung fehlschlägt",
            s("s-wetter"),
            "fails",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Whatever else is running, let me know when the Axis labels session ends",
            s("s-tide"),
            "ends",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Nur einmal: melde dich, wenn die Fahrplan-Import-Sitzung fertig ist",
            s("s-bahn"),
            "finishes",
            None,
            LifetimeChoice::Once,
            false,
        ),
        (
            "Permanently watch for new sessions starting in quiz-engine",
            p("p-quiz"),
            "starts",
            None,
            LifetimeChoice::Always,
            false,
        ),
        (
            "Sag mir, wenn die Terminvergabe-Sitzung fertig ist",
            s("s-dach"),
            "finishes",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Keep telling me whenever any session in sage-billing fails, permanently",
            p("p-sage"),
            "fails",
            None,
            LifetimeChoice::Always,
            false,
        ),
        (
            "Tell me about any new session that starts anywhere",
            WatchTarget::Any,
            "starts",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Ich möchte wissen, wenn in lagerhaus eine Sitzung endet",
            p("p-lager"),
            "ends",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Across everything I run, I only care when the Timezone fix session finishes",
            s("s-orbit"),
            "finishes",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Tell me when Cursor crashes and gets started again",
            WatchTarget::App("app-cur".into()),
            "restarted",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
        (
            "Notify me when a sage-billing session ends and leaves processes running",
            p("p-sage"),
            "leaves_processes",
            None,
            LifetimeChoice::Unsaid,
            false,
        ),
    ];
    assert_said(&v11_candidates(), &rows);
    // W55: not read as a request before; now told back with the nearest.
    let request = "Ich will wissen, wann Sitzungen fertig sind, nicht von Cursor";
    let (asked, d) = told_back(request, run11(request, &Scripted::new(&["FAIL"])));
    assert_eq!(asked, "nicht von Cursor");
    assert_eq!(d.conditions[0].code, "finishes");
    assert!(d.targets.is_empty());
}

/// v12: labels code verifies may draft (a name said in part with its
/// kind); a liar never drafts what can't be watched.
#[test]
fn verified_labels_draft_and_liars_never_do() {
    let span = |t: &str, slot: &str| {
        format!(r#"{{"text":"{t}","slot":"{slot}","number":0,"unit":"none"}}"#)
    };
    let labels = |spans: &[String]| -> &'static str {
        Box::leak(format!(r#"{{"spans":[{}]}}"#, spans.join(",")).into_boxed_str())
    };
    // Honest: "the Image session" is Image resizer, said in part with its kind.
    let request = "Tell me when the Image session fails";
    let fake = Scripted::new(&[labels(&[
        span("Tell me when", "filler"),
        span("the Image session", "target"),
        span("fails", "condition"),
    ])]);
    match run11(request, &fake) {
        Outcome::Drafted(WatchDrafted::Filled { draft }) => {
            assert_eq!(draft.targets, vec![WatchTarget::Session("s-img".into())]);
            assert_eq!(draft.conditions[0].code, "fails");
        }
        other => unreachable!("{other:?}"),
    }
    // A liar's target that names the exclusion, or an "unless" clause as
    // filler: the guard, before any label.
    for (request, lie) in [
        (
            "Tell me when the Flag parsing session fails, nicht von der Wetterstation",
            labels(&[
                span("Tell me when the Flag parsing session fails,", "filler"),
                span("nicht von der Wetterstation", "target"),
            ]),
        ),
        (
            "Tell me when the Image resizer finishes unless it ran for under an hour",
            labels(&[
                span("Tell me when", "filler"),
                span("the Image resizer", "target"),
                span("finishes", "condition"),
                span("unless it ran for under an hour", "filler"),
            ]),
        ),
    ] {
        let fake = Scripted::new(&[lie]);
        told_back(request, run11(request, &fake));
        assert_eq!(fake.calls(), 0, "{request}: the guard asks no label");
    }
    // A liar hiding words in filler, or a part name with no kind: never
    // drafted, told back after the retries.
    for (request, lie) in [
        (
            "Tell me when the Image resizer fails on the main branch",
            labels(&[
                span("Tell me when", "filler"),
                span("the Image resizer", "target"),
                span("fails", "condition"),
                span("on the main branch", "filler"),
            ]),
        ),
        (
            "Tell me when the tests fail",
            labels(&[
                span("Tell me when", "filler"),
                span("the tests", "target"),
                span("fail", "condition"),
            ]),
        ),
        (
            "Tell me when the Image resizer fails 3 times",
            labels(&[
                span("Tell me when", "filler"),
                span("the Image resizer", "target"),
                span("fails 3 times", "condition"),
            ]),
        ),
    ] {
        let fake = Scripted::new(&[lie]);
        told_back(request, run11(request, &fake));
        assert_eq!(fake.calls(), 3, "{request}: asked again, then code's own");
    }
}
