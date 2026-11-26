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

/// v10's names (its misses are quoted in `v10-results/summary.md`; the
/// rest of the set stays unread), in a made-up context.
fn v10_candidates() -> Vec<Candidate> {
    let s = |id: &str, name: &str, project: &str| Candidate {
        target: WatchTarget::Session(id.into()),
        name: name.into(),
        aliases: Vec::new(),
        app: Some("Codex".into()),
        project: Some(project.into()),
        offered: OFFER.map(str::to_owned).to_vec(),
    };
    let p = |name: &str| Candidate {
        target: WatchTarget::Project(format!("p-{name}")),
        name: name.into(),
        aliases: Vec::new(),
        app: None,
        project: None,
        offered: OFFER.map(str::to_owned).to_vec(),
    };
    vec![
        s("S-ws", "Wetterstation-Sync", "wetter"),
        s("S-pay", "Payment flow", "shop"),
        s("S-bill", "billing-sync", "billing"),
        s("S-dash", "dashboard filters", "dash"),
        s("S-ing", "Ingest worker", "pipeline"),
        s("S-lan", "Landing page copy", "site"),
        s("S-chk", "checkout-api", "shop"),
        s("S-scan", "scan deskew", "scanner"),
        s("S-emb", "Embedding refresh", "search"),
        p("shop"),
        p("checkout-api"),
        p("billing-sync"),
    ]
}

fn run(request: &str, model: &Scripted) -> Outcome {
    let c = v10_candidates();
    let ask = Ask {
        request,
        candidates: &c,
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
    for (request, target, code, param, lifetime, quiet) in rows {
        let fake = Scripted::new(&["FAIL"]);
        let d = match run(request, &fake) {
            Outcome::Drafted(WatchDrafted::Filled { draft }) => draft,
            other => unreachable!("{request}: {other:?}"),
        };
        assert_eq!(d.targets, vec![target], "{request}");
        assert_eq!(d.conditions[0].code, code, "{request}");
        assert_eq!(d.conditions[0].param, param, "{request}");
        assert_eq!(d.lifetime, lifetime, "{request}");
        assert_eq!(d.even_in_quiet_hours, quiet, "{request}");
        assert_eq!(fake.calls(), 0, "{request}: code's alone");
    }
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
    let request =
        "Warne mich bei Fehlern, aber nur bei solchen, die nicht von der Wetterstation kommen";
    let fake = Scripted::new(&[
        r#"{"spans":[{"text":"Warne mich bei","slot":"filler","number":0,"unit":"none"},{"text":"Fehlern","slot":"condition","number":0,"unit":"none"},{"text":"aber nur bei solchen, die nicht von der Wetterstation kommen","slot":"exclusion","number":0,"unit":"none"}]}"#,
    ]);
    let (asked, d) = told_back(request, run(request, &fake));
    assert_eq!(
        asked,
        "aber nur bei solchen, die nicht von der Wetterstation kommen"
    );
    assert!(d.targets.is_empty(), "the form asks: {:?}", d.targets);
    assert_eq!(d.conditions[0].code, "fails");
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
