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
    run_on(&candidates(), request, fake)
}

fn run_on(c: &[Candidate], request: &str, fake: &Fake) -> Outcome {
    let ask = Ask {
        request,
        candidates: c,
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
fn the_prompts_are_the_ones_the_harness_runs() {
    // Read from the harness at test time (the docs don't ship in the app).
    let read = |path: &str| {
        std::fs::read_to_string(format!(
            "{}/../../docs/qa/evidence/AJ-E3709/harness/{path}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap_or_default()
    };
    assert!(
        read("v7/prompts/watch3.md").contains(TARGET_INSTRUCTIONS),
        "call 1's instructions drifted from the harness"
    );
    let v11 = read("v11/prompts/watch7.md");
    assert!(
        v11.contains(TARGET_INSTRUCTIONS.trim_end())
            && v11.contains(cover::LABEL_INSTRUCTIONS.trim_end())
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
    let cases: [Case; 6] = [
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

/// v11: the model never picks a condition. No cue word, no condition:
/// the form; words code can't place: told back, the model only labelling.
#[test]
fn no_condition_word_and_the_model_never_picks_one() {
    let fake = Fake::new(&["FAIL"]);
    assert!(matches!(
        run_with("Tell me about cedar-api when you can.", &fake),
        Outcome::Drafted(WatchDrafted::Form)
    ));
    assert_eq!(fake.calls(), 0, "no call to pick a condition");
    let request = "Let me know if cedar-api sits there doing nothing for 20 minutes.";
    let fake = Fake::new(&["FAIL"]);
    let d = assert_told_back(request, run_with(request, &fake), &fake);
    assert_eq!(d.targets, vec![WatchTarget::Session("S-cedar".into())]);
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
        assert_only_labelled(request, &fake);
    }
    // Quiet hours and a usage limit are no false alarm.
    let out = run_with(
        "Wake me even at 3am if cedar-api fails.",
        &Fake::new(&["FAIL"]),
    );
    assert!(filled(&out).is_some(), "{out:?}");
    // v11: words code can't place ("the costs matter") are told back, with
    // the very watch as the nearest.
    let request = "Tell me when the work plan hits its limit, the costs matter.";
    match run_with(request, &Fake::new(&["FAIL"])) {
        Outcome::Drafted(WatchDrafted::CantWatch { asked, draft }) => {
            assert_eq!(asked, "costs matter");
            let d = draft.unwrap_or_else(|| unreachable!("no nearest"));
            assert_eq!(d.targets, vec![WatchTarget::Account("acc-w".into())]);
            assert_eq!(codes(&d), ["hits_limit"]);
        }
        other => unreachable!("{other:?}"),
    }
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
        let d = assert_told_back(request, run_with(request, &fake), &fake);
        assert_eq!(codes(&d), [near], "{request}");
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
    // v11: an exclusion is told back, even beside a named session; its
    // nearest names no target (the form asks), so never the one left out.
    let request = "Tell me when cedar-api asks something, and ignore aspen-board entirely.";
    let fake = Fake::new(&["FAIL"]);
    let d = assert_told_back(request, run_with(request, &fake), &fake);
    assert!(d.targets.is_empty(), "{:?}", d.targets);
    assert_eq!(codes(&d), ["asks_you"]);
    let out = run_with("Always tell me if anything fails.", &Fake::new(&["FAIL"]));
    assert_eq!(
        filled(&out).map(|d| d.lifetime),
        Some(LifetimeChoice::Always)
    );
}

fn misses_candidates() -> Vec<Candidate> {
    vec![
        session("S-ret", "retry logic", "hooks", "Codex"),
        session("S-web", "webhook retries", "hooks", "Claude Code"),
        session("S-gl", "glossary hub", "docs", "Codex"),
        session("S-fk", "form-kit", "ui", "Claude Code"),
        session("S-ca", "checkout-api", "shop", "Codex"),
        session("S-bt", "beta tracker", "beta-tracker", "Codex"),
        session("S-kf", "kiosk firmware", "pier", "Claude Code"),
        session("S-pb", "pier board", "pier", "Codex"),
        session("S-ce", "coupon engine", "coupon-engine", "Codex"),
        owner(
            WatchTarget::Project("otter-billing".into()),
            "otter-billing",
            &SESSION,
            &[],
        ),
        owner(WatchTarget::Project("ui".into()), "ui", &SESSION, &[]),
        owner(
            WatchTarget::Project("beta-tracker".into()),
            "beta-tracker",
            &SESSION,
            &[],
        ),
        owner(WatchTarget::Project("pier".into()), "pier", &SESSION, &[]),
        owner(
            WatchTarget::Project("coupon-engine".into()),
            "coupon-engine",
            &SESSION,
            &[],
        ),
    ]
}

fn misses_run(request: &str, fake: &Fake) -> Outcome {
    run_on(&misses_candidates(), request, fake)
}

/// Told back with a nearest that never widens, decided by code.
fn assert_told_back(request: &str, out: Outcome, fake: &Fake) -> WatchDraftInfo {
    let Outcome::Drafted(WatchDrafted::CantWatch { asked, draft }) = out else {
        unreachable!("{request}: {out:?}");
    };
    assert!(!asked.is_empty(), "{request}");
    let d = draft.unwrap_or_else(|| unreachable!("{request}: no nearest"));
    assert!(
        !d.conditions.is_empty(),
        "{request}: the nearest is offered"
    );
    assert!(
        d.targets.iter().all(|t| t.kind() == TargetKind::Session),
        "{request}: never widened"
    );
    assert_only_labelled(request, fake);
    d
}

/// The only call made was the labelling one (v11: the model labels, code
/// decides).
fn assert_only_labelled(request: &str, fake: &Fake) {
    assert!(
        fake.seen
            .borrow()
            .iter()
            .all(|r| r.shape.as_ref().is_some_and(|s| s["name"] == "WatchSpans")),
        "{request}: decided by code; the model only labels"
    );
}

/// D-149 amendment 2: every unsupported-claimed miss so far (v7 W05, W19,
/// W32, W43, W51; v8 W27, W54, W60), each told back with the nearest and
/// never a watch, with no model call. W05, W19, W43, W27, W54 and W60 are
/// their own requests (v7 and v8 `summary.md`); W32 and W51 are in fresh
/// words of their recorded drafts' shape.
#[test]
fn every_unsupported_claimed_miss_so_far_fails_closed() {
    let rows = [
        "tell me when all sessions finish, except the retry logic one", // v7 W05
        "for every project except otter-billing, tell me if it fails",  // v7 W19
        "Tell me once if anything fails, but not the retry logic one.", // v7 W32's shape
        "tell me when the webhook retries session uses more than 50k tokens", // v7 W43
        "Gib mir Bescheid, wenn alle Sitzungen fertig sind, außer retry logic.", // v7 W51's shape
        "Watch all my sessions for getting stuck 30 minutes, skipping the glossary hub.", // v8 W27
        "Let me know if everything but form-kit asks me a question.",   // v8 W54, the inverse
        "Ping me when the token count of any session passes one million.", // v8 W60
    ];
    for request in rows {
        let fake = Fake::new(&["FAIL"]);
        let d = assert_told_back(request, misses_run(request, &fake), &fake);
        assert!(
            !d.targets.contains(&WatchTarget::Session("S-fk".into())),
            "{request}: never the excluded one"
        );
    }
}

/// What fail closed keeps: a selection that names its target, a quantifier
/// inside its owner, a clause that asks, a session over its namesake
/// project, and a bare "once" as "when".
#[test]
fn what_code_fully_understands_still_drafts() {
    let cases: [(&str, WatchTarget, &str, LifetimeChoice); 3] = [
        (
            "Of all my sessions, tell me when checkout-api fails.",
            WatchTarget::Session("S-ca".into()),
            "fails",
            LifetimeChoice::Unsaid,
        ),
        (
            "Ping me if anything in pier fails.",
            WatchTarget::Project("pier".into()),
            "fails",
            LifetimeChoice::Unsaid,
        ),
        (
            "Tell me once checkout-api finishes.",
            WatchTarget::Session("S-ca".into()),
            "finishes",
            LifetimeChoice::Unsaid,
        ),
    ];
    for (request, target, code, lifetime) in cases {
        let out = misses_run(request, &Fake::new(&["FAIL"]));
        let d = filled(&out).unwrap_or_else(|| unreachable!("{request}: {out:?}"));
        assert_eq!(d.targets, vec![target], "{request}");
        assert_eq!(codes(d), [code], "{request}");
        assert_eq!(d.lifetime, lifetime, "{request}");
    }
    // v11, full coverage: a clause code can't place is told back, the
    // watch it names offered as the nearest.
    for (request, target, code) in [
        (
            "Tell me when the kiosk firmware session fails; it was split off from the pier board one.",
            "S-kf",
            "fails",
        ),
        (
            "Once the coupon engine is done, the reminder mails can start, so ping me just once when the coupon engine finishes.",
            "S-ce",
            "finishes",
        ),
    ] {
        let fake = Fake::new(&["FAIL"]);
        let d = assert_told_back(request, misses_run(request, &fake), &fake);
        assert_eq!(
            d.targets,
            vec![WatchTarget::Session(target.into())],
            "{request}"
        );
        assert_eq!(codes(&d), [code], "{request}");
    }
    // "but only": a word that leaves something out, so no target at all.
    let request = "I'm comparing the beta tracker with the glossary hub, but only alert me if the beta tracker asks me something.";
    let fake = Fake::new(&["FAIL"]);
    let d = assert_told_back(request, misses_run(request, &fake), &fake);
    assert!(d.targets.is_empty() && codes(&d) == ["asks_you"], "{d:?}");
}

fn v9_candidates() -> Vec<Candidate> {
    let app = |id: &str, name: &str| owner(WatchTarget::App(id.into()), name, &SESSION, &[]);
    vec![
        session("S-km", "Karten mischen", "karten", "Codex"),
        session("S-gp", "gartenplaner", "garten", "Claude Code"),
        session("S-pe", "Pattern export", "pattern", "Codex"),
        session("S-ci", "Chapter index", "book", "Codex"),
        session("S-tr", "Till rounding", "till", "Codex"),
        session("S-bs", "billing-sync", "billing", "Codex"),
        session("S-ck", "cache keys", "cache", "Codex"),
        owner(
            WatchTarget::Project("pattern".into()),
            "pattern",
            &SESSION,
            &[],
        ),
        owner(WatchTarget::Project("till".into()), "till", &SESSION, &[]),
        owner(
            WatchTarget::Project("monorepo-ci".into()),
            "monorepo-ci",
            &SESSION,
            &[],
        ),
        owner(
            WatchTarget::Account("acc-h".into()),
            "Harbor Team",
            &["hits_limit", "runway_below", "runway_resets"],
            &[],
        ),
        app("cursor", "Cursor"),
        app("opencode", "OpenCode"),
        app("studio", "Claude Code CLI · Studio"),
        Candidate {
            offered: ["finishes", "starts", "fails", "asks_you"]
                .map(str::to_owned)
                .to_vec(),
            ..app("any-apps", "Codex")
        },
    ]
}

/// v10: each v9 miss (V9-W01, W02, W03, W18, W21, W24, W25, W28, W30, W32,
/// W36, W37, W40, W41, W55, W56, W58; their own requests where v9's
/// `summary.md` quotes them, else fresh words of their recorded shape),
/// drafted as gold reads it, by code with no call.
#[test]
fn v9s_misses_draft_as_said() {
    let always = LifetimeChoice::Always;
    let cases: [(&str, WatchTarget, &str, Option<Param>, LifetimeChoice); 13] = [
        (
            "Melde dich, wenn Karten mischen etwas fragt, dauerhaft",
            WatchTarget::Session("S-km".into()),
            "asks_you",
            None,
            always,
        ),
        (
            "page me during quiet hours too when Harbor Team goes below 15 percent, permanently",
            WatchTarget::Account("acc-h".into()),
            "runway_below",
            Some(Param::Percent(15)),
            always,
        ),
        (
            "warn me when Harbor Team drops below 30% runway, and keep that on permanently",
            WatchTarget::Account("acc-h".into()),
            "runway_below",
            Some(Param::Percent(30)),
            always,
        ),
        (
            "Wenn Cursor sein Limit erreicht, gib mir einmal Bescheid",
            WatchTarget::App("cursor".into()),
            "hits_limit",
            None,
            LifetimeChoice::Once,
        ),
        (
            "Tell me just this once when billing-sync finishes.",
            WatchTarget::Session("S-bs".into()),
            "finishes",
            None,
            LifetimeChoice::Once,
        ),
        (
            "Tell me the next time billing-sync fails.",
            WatchTarget::Session("S-bs".into()),
            "fails",
            None,
            LifetimeChoice::Once,
        ),
        (
            "Sag mir nur beim nächsten Mal Bescheid, wenn billing-sync fertig ist.",
            WatchTarget::Session("S-bs".into()),
            "finishes",
            None,
            LifetimeChoice::Once,
        ),
        (
            "Bei all meinen Projekten: melde mir bitte, wenn gartenplaner scheitert, für heute",
            WatchTarget::Session("S-gp".into()),
            "fails",
            None,
            LifetimeChoice::Today,
        ),
        (
            "Tell me when all monorepo-ci sessions fail.",
            WatchTarget::Project("monorepo-ci".into()),
            "fails",
            None,
            LifetimeChoice::Unsaid,
        ),
        (
            "Tell me when billing-sync fails, but hold it until quiet hours are over.",
            WatchTarget::Session("S-bs".into()),
            "fails",
            None,
            LifetimeChoice::Unsaid,
        ),
        (
            "Ping me if any OpenCode session fails.",
            WatchTarget::App("opencode".into()),
            "fails",
            None,
            LifetimeChoice::Unsaid,
        ),
        (
            "Let me know any time Claude Code CLI · Studio hits a limit.",
            WatchTarget::App("studio".into()),
            "hits_limit",
            None,
            LifetimeChoice::Unsaid, // "any time" is "when" (v10's W54)
        ),
        (
            "Tell me if anything goes wrong with Chapter index.",
            WatchTarget::Session("S-ci".into()),
            "fails",
            None,
            LifetimeChoice::Unsaid,
        ),
    ];
    let c = v9_candidates();
    // v12: a scope-neutral context clause is filler.
    let out = run_on(
        &c,
        "whatever else is going on, I need to know the moment Till rounding finishes",
        &Fake::new(&["FAIL"]),
    );
    assert_eq!(
        filled(&out).map(|d| d.targets.clone()),
        Some(vec![WatchTarget::Session("S-tr".into())])
    );
    for (request, target, code, param, lifetime) in cases {
        let fake = Fake::new(&["FAIL"]);
        let out = run_on(&c, request, &fake);
        let d = filled(&out).unwrap_or_else(|| unreachable!("{request}: {out:?}"));
        assert_eq!(d.targets, vec![target], "{request}");
        assert_eq!(codes(d), [code], "{request}");
        assert_eq!(
            d.conditions[0].param, param,
            "{request}: no invented number"
        );
        assert_eq!(d.lifetime, lifetime, "{request}");
        assert_eq!(fake.calls(), 0, "{request}");
    }
    // Still fail closed: the two unsupported misses get their nearest; v11
    // tells back what code can't place, the watch named as the nearest.
    for request in [
        "I want to know about every new session, leave out OpenCode",
        "warn me when cache keys uses more than 1.5 million tokens today",
        "out of every session I have open, let me know when the Pattern export one asks me something",
        "Let me know, say, when billing-sync finishes.",
    ] {
        let fake = Fake::new(&["FAIL"]);
        assert_told_back(request, run_on(&c, request, &fake), &fake);
    }
}
