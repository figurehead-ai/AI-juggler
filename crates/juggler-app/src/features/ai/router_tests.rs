//! AJ-E3802's matrix: engines (none, the system model, the local engine
//! with its helper missing, a ready local engine, both, the system model
//! removed mid-session) ×
//! every routed job × verdicts × Beta switches × the job's flag. The whole
//! table is `fixtures/router-matrix.txt` (regenerate with
//! `AIJ_WRITE_MATRIX=1`); the properties every row must hold are checked
//! one by one beside it.

#![allow(
    clippy::panic,
    reason = "a matrix row that breaks a rule fails naming itself"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::engine::{AiEngine as _, Card, Readiness, Reason, Tier};
use super::engines::local::{Host, Local, Setup};
use super::jobs::{self, JobSpec};
use super::models::store::Installed;
use super::models::{Chip, Engine, Job, JobVerdict, ModelEntry, Sampling, Verdict};
use super::router::{Choice, Inputs, Route, Switches, route};

const MATRIX: &str = include_str!("fixtures/router-matrix.txt");

const SYSTEM: &str = "system-model";
const LOCAL: &str = "local-model";

fn entry(id: &str, engine: Engine, verdict: Verdict) -> ModelEntry {
    ModelEntry {
        id: id.to_owned(),
        name: id.to_owned(),
        engine,
        download: None,
        min_chip: Chip::M1,
        min_memory_gb: 8,
        min_free_gb: None,
        speed: None,
        sampling: Sampling::default(),
        verdicts: Job::ALL
            .iter()
            .map(|(job, _)| {
                (
                    *job,
                    JobVerdict {
                        verdict,
                        note: None,
                    },
                )
            })
            .collect(),
        withdrawn: false,
    }
}

fn system_card(ready: Readiness) -> Card {
    Card {
        engine: 0,
        model: SYSTEM.to_owned(),
        context_tokens: 8_192,
        tools: true,
        tier: Tier::Basic,
        uses_download: false,
        running: false,
        ready,
    }
}

fn local_card(ready: Readiness) -> Card {
    Card {
        engine: 1,
        model: LOCAL.to_owned(),
        context_tokens: 8_192,
        tools: true,
        tier: Tier::Basic,
        uses_download: true,
        running: true,
        ready,
    }
}

/// The local engine with `LOCAL` installed and its helper not in the
/// app: its card says why (nothing is started).
fn local_without_helper(models: &[ModelEntry]) -> Vec<Card> {
    let nowhere = std::path::PathBuf::from("/nonexistent/aij-local-ai");
    let installed = Installed {
        entry: entry(LOCAL, Engine::GgufLocal, Verdict::Ships),
        sha256: "0".repeat(64),
        size_bytes: 1,
        installed_at_ms: 0,
    };
    let engine = Local::with(Setup {
        program: Some(nowhere.clone()),
        models: Some((nowhere, vec![installed])),
        host: Some(Host {
            macos: true,
            apple_silicon: true,
            memory_gb: 32,
        }),
        pressure: Some(|| false),
        unsandboxed: true,
        sandbox_tool: None,
    });
    engine.probe();
    engine.cards(1, models)
}

/// The engine sets of the matrix, by name.
fn engine_sets(models: &[ModelEntry]) -> Vec<(&'static str, Vec<Card>)> {
    let no_helper = local_without_helper(models);
    let removed = Readiness::NotHere(Reason::TurnedOff);
    vec![
        ("none", vec![]),
        ("system", vec![system_card(Readiness::Ready)]),
        ("local-no-helper", no_helper.clone()),
        ("local", vec![local_card(Readiness::Ready)]),
        (
            "both",
            vec![system_card(Readiness::Ready), local_card(Readiness::Ready)],
        ),
        (
            "system-removed",
            vec![system_card(removed), local_card(Readiness::Ready)],
        ),
        (
            "system-removed-no-helper",
            [vec![system_card(removed)], no_helper].concat(),
        ),
        (
            "system-preparing",
            vec![system_card(Readiness::Preparing(Reason::GettingReady))],
        ),
    ]
}

const VERDICTS: [(Verdict, &str); 4] = [
    (Verdict::Ships, "ships"),
    (Verdict::Beta, "beta"),
    (Verdict::NotYet, "not_yet"),
    (Verdict::NotTested, "not_tested"),
];

fn words(route: Route, cards: &[Card]) -> String {
    let model = |card: usize| cards[card].model.clone();
    match route {
        Route::Off => "off".to_owned(),
        Route::NoEngine(None) => "no_engine".to_owned(),
        Route::NoEngine(Some(why)) => format!("no_engine ({why:?})"),
        Route::BetaOff => "beta_off".to_owned(),
        Route::Waiting { card, why } => format!("waiting {} ({why:?})", model(card)),
        Route::Run { card, why, beta } => {
            format!(
                "run {} ({why:?}{})",
                model(card),
                if beta { ", beta" } else { "" }
            )
        }
    }
}

fn table() -> String {
    let mut out = String::from("engines | job | verdict | beta | flag -> route\n");
    let none = BTreeMap::new();
    for (verdict, verdict_name) in VERDICTS {
        let models = [
            entry(SYSTEM, Engine::AppleBuiltin, verdict),
            entry(LOCAL, Engine::GgufLocal, verdict),
        ];
        for (set, cards) in engine_sets(&models) {
            let inputs = Inputs {
                cards: &cards,
                models: &models,
                automatic: None,
                picks: &none,
            };
            for spec in jobs::JOBS {
                for beta in [false, true] {
                    for on in [true, false] {
                        let got = route(spec, Switches { on, beta }, &inputs);
                        let _ = writeln!(
                            out,
                            "{set} | {} | {verdict_name} | {} | {} -> {}",
                            spec.job.name(),
                            if beta { "on" } else { "off" },
                            if on { "on" } else { "off" },
                            words(got, &cards),
                        );
                    }
                }
            }
        }
    }
    out
}

#[test]
fn the_matrix_is_the_recorded_one() {
    let now = table();
    if std::env::var_os("AIJ_WRITE_MATRIX").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/features/ai/fixtures/router-matrix.txt");
        std::fs::write(path, &now).unwrap();
        return;
    }
    assert_eq!(
        now, MATRIX,
        "the router's answers changed: review, then AIJ_WRITE_MATRIX=1"
    );
}

/// Every row, checked against the rules rather than the record.
#[test]
fn every_row_keeps_the_rules() {
    let none = BTreeMap::new();
    for (verdict, _) in VERDICTS {
        let models = [
            entry(SYSTEM, Engine::AppleBuiltin, verdict),
            entry(LOCAL, Engine::GgufLocal, verdict),
        ];
        for (set, cards) in engine_sets(&models) {
            let inputs = Inputs {
                cards: &cards,
                models: &models,
                automatic: None,
                picks: &none,
            };
            let any_ready = cards.iter().any(|c| c.ready == Readiness::Ready);
            let any_here = cards
                .iter()
                .any(|c| !matches!(c.ready, Readiness::NotHere(_)));
            for spec in jobs::JOBS {
                for beta in [false, true] {
                    let off = route(spec, Switches { on: false, beta }, &inputs);
                    assert_eq!(off, Route::Off, "a job whose flag is off doesn't exist");
                    let got = route(spec, Switches { on: true, beta }, &inputs);
                    let what = format!("{set} {:?} {verdict:?} beta {beta}", spec.job);
                    if !verdict.passed() || !any_here {
                        assert!(matches!(got, Route::NoEngine(_)), "{what}: {got:?}");
                    } else if verdict == Verdict::Beta && !beta {
                        assert_eq!(got, Route::BetaOff, "{what}");
                    } else if any_ready {
                        let Route::Run {
                            card,
                            beta: as_beta,
                            ..
                        } = got
                        else {
                            panic!("{what}: {got:?}");
                        };
                        assert_eq!(as_beta, verdict == Verdict::Beta, "{what}");
                        assert_eq!(cards[card].ready, Readiness::Ready, "{what}");
                        if cards
                            .iter()
                            .any(|c| c.model == SYSTEM && c.ready == Readiness::Ready)
                        {
                            assert_eq!(cards[card].model, SYSTEM, "{what}: no download first");
                        }
                    } else {
                        assert!(matches!(got, Route::Waiting { .. }), "{what}: {got:?}");
                    }
                    assert_eq!(
                        got.shown(),
                        matches!(got, Route::Run { .. } | Route::Waiting { .. })
                    );
                }
            }
        }
    }
}

#[test]
fn a_job_with_no_engine_is_absent_from_the_query_answer() {
    let models = [entry(SYSTEM, Engine::AppleBuiltin, Verdict::Ships)];
    let none = BTreeMap::new();
    for cards in [
        vec![],
        vec![system_card(Readiness::NotHere(Reason::TurnedOff))],
    ] {
        let inputs = Inputs {
            cards: &cards,
            models: &models,
            automatic: None,
            picks: &none,
        };
        for spec in jobs::JOBS {
            let got = route(
                spec,
                Switches {
                    on: true,
                    beta: true,
                },
                &inputs,
            );
            assert!(!got.shown(), "{:?} with {cards:?}: {got:?}", spec.job);
        }
    }
}

#[test]
fn removing_the_system_model_mid_session_hides_only_what_it_alone_served() {
    let models = [
        entry(SYSTEM, Engine::AppleBuiltin, Verdict::Ships),
        entry(LOCAL, Engine::GgufLocal, Verdict::Ships),
    ];
    let none = BTreeMap::new();
    let on = Switches {
        on: true,
        beta: false,
    };
    let before = vec![system_card(Readiness::Ready)];
    let after = vec![system_card(Readiness::NotHere(Reason::TurnedOff))];
    let with_local = vec![
        system_card(Readiness::NotHere(Reason::TurnedOff)),
        local_card(Readiness::Ready),
    ];
    let chat = |cards: &[Card]| {
        route(
            &jobs::CHAT,
            on,
            &Inputs {
                cards,
                models: &models,
                automatic: None,
                picks: &none,
            },
        )
    };
    assert!(matches!(chat(&before), Route::Run { card: 0, .. }));
    assert_eq!(
        chat(&after),
        Route::NoEngine(Some(Reason::TurnedOff)),
        "the reason says what the user can do"
    );
    assert!(
        matches!(
            chat(&with_local),
            Route::Run {
                card: 1,
                why: Choice::Only,
                ..
            }
        ),
        "another engine still serves it"
    );
}

#[test]
fn the_order_picked_then_recommended_then_no_download_then_tier_then_running() {
    let models = [
        entry(SYSTEM, Engine::AppleBuiltin, Verdict::Ships),
        entry(LOCAL, Engine::GgufLocal, Verdict::Ships),
    ];
    let cards = vec![system_card(Readiness::Ready), local_card(Readiness::Ready)];
    let on = Switches {
        on: true,
        beta: false,
    };
    let none = BTreeMap::new();
    let picked = BTreeMap::from([(Job::Chat, LOCAL.to_owned())]);
    let run = |automatic: Option<&str>, picks: &BTreeMap<Job, String>, cards: &[Card]| {
        route(
            &jobs::CHAT,
            on,
            &Inputs {
                cards,
                models: &models,
                automatic,
                picks,
            },
        )
    };
    assert_eq!(
        run(None, &picked, &cards),
        Route::Run {
            card: 1,
            why: Choice::Picked,
            beta: false
        }
    );
    assert_eq!(
        run(Some(LOCAL), &none, &cards),
        Route::Run {
            card: 1,
            why: Choice::Recommended,
            beta: false
        }
    );
    assert_eq!(
        run(Some(SYSTEM), &none, &cards),
        Route::Run {
            card: 0,
            why: Choice::Recommended,
            beta: false
        }
    );
    assert_eq!(
        run(None, &none, &cards),
        Route::Run {
            card: 0,
            why: Choice::NoDownload,
            beta: false
        }
    );
    let mut stronger = cards.clone();
    stronger[1].uses_download = false;
    stronger[1].tier = Tier::Capable;
    assert_eq!(
        run(None, &none, &stronger),
        Route::Run {
            card: 1,
            why: Choice::Quality,
            beta: false
        }
    );
    stronger[1].tier = Tier::Basic;
    assert_eq!(
        run(None, &none, &stronger),
        Route::Run {
            card: 1,
            why: Choice::Running,
            beta: false
        }
    );
    stronger[1].running = false;
    assert_eq!(
        run(None, &none, &stronger),
        Route::Run {
            card: 0,
            why: Choice::First,
            beta: false
        }
    );
    // A pick the job can't use (a model gone) leaves the Automatic order.
    let gone = BTreeMap::from([(Job::Chat, "removed".to_owned())]);
    assert_eq!(
        run(None, &gone, &cards),
        Route::Run {
            card: 0,
            why: Choice::NoDownload,
            beta: false
        }
    );
}

#[test]
fn needs_decide_which_cards_can_serve_a_job() {
    let models = [entry(SYSTEM, Engine::AppleBuiltin, Verdict::Ships)];
    let none = BTreeMap::new();
    let on = Switches {
        on: true,
        beta: false,
    };
    let mut small = system_card(Readiness::Ready);
    small.context_tokens = 2_048;
    small.tools = false;
    let cards = [small];
    let inputs = Inputs {
        cards: &cards,
        models: &models,
        automatic: None,
        picks: &none,
    };
    let feed = JobSpec {
        flag: &jobs::REVIEW_ON,
        ..jobs::FEED
    };
    assert!(
        route(&feed, on, &inputs).shown(),
        "1,024 tokens, no tools: fits"
    );
    assert_eq!(
        route(&jobs::CHAT, on, &inputs),
        Route::NoEngine(None),
        "Chat needs 4,096 and look-ups"
    );
    let strong = JobSpec {
        needs: super::engine::Needs {
            tier: Tier::Strong,
            ..jobs::REVIEW.needs
        },
        ..jobs::REVIEW
    };
    assert_eq!(
        route(&strong, on, &inputs),
        Route::NoEngine(None),
        "an orchestrator's tier has no engine"
    );
}

#[test]
fn a_model_missing_from_the_list_serves_nothing() {
    let none = BTreeMap::new();
    let cards = [system_card(Readiness::Ready)];
    let inputs = Inputs {
        cards: &cards,
        models: &[],
        automatic: None,
        picks: &none,
    };
    for spec in jobs::JOBS {
        assert_eq!(
            route(
                spec,
                Switches {
                    on: true,
                    beta: true
                },
                &inputs
            ),
            Route::NoEngine(None)
        );
    }
}

#[test]
fn the_jobs_flags_are_declared_and_off_only() {
    for spec in jobs::JOBS {
        assert!(
            crate::features::flags::DECLARED.contains(&spec.flag),
            "{} isn't in the app's flags",
            spec.flag.name
        );
        assert_eq!(
            spec.flag.remote,
            juggler_remote_config::RemoteMode::OffOnly,
            "D-130: a kill switch only"
        );
        assert_eq!(jobs::by_name(spec.job.name()), Some(spec));
    }
    let on: Vec<&str> = jobs::JOBS
        .iter()
        .filter(|spec| spec.flag.default)
        .map(|spec| spec.job.name())
        .collect();
    assert_eq!(
        on,
        ["review", "chat"],
        "D-131's table: review and Chat as opt-in Beta"
    );
    assert!(
        jobs::by_name("attention").is_none(),
        "attention is code, not routed"
    );
}

/// D-131: the local engine itself (a fake helper in place of
/// `juggler-local-ai`) gives a ready card for an installed model, and the
/// one query routes to it: Automatic picks the downloaded model the
/// recommendation names (AJ-E3811) on a Mac without Apple's model, and a
/// job whose Beta is off says so.
#[cfg(unix)]
#[test]
fn the_local_engine_serves_its_installed_model_and_automatic_picks_it() {
    use std::os::unix::fs::PermissionsExt as _;

    use super::dto::{AiJob, AiJobState, AiJobWhy};
    use super::engines::Registry;
    use super::engines::apple::Apple;
    use super::models::recommend::{AppleAi, HostChip, MacFacts};
    use super::service::answer_for;
    use crate::features::remote_config::Snapshot;

    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("juggler-local-ai");
    std::fs::write(
        &program,
        "#!/bin/sh\necho '{\"kind\":\"hello\",\"protocol\":3,\"gpu\":true,\"sandboxed\":false}'\ncat > /dev/null\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let local_entry = entry(LOCAL, Engine::GgufLocal, Verdict::Beta);
    let installed = Installed {
        entry: local_entry.clone(),
        sha256: "a".repeat(64),
        size_bytes: 1,
        installed_at_ms: 0,
    };
    let local = Local::with(Setup {
        program: Some(program),
        models: Some((dir.path().to_path_buf(), vec![installed])),
        host: Some(Host {
            macos: true,
            apple_silicon: true,
            memory_gb: 32,
        }),
        pressure: Some(|| false),
        unsandboxed: true,
        sandbox_tool: None,
    });
    let apple = Apple::with_program(Some(dir.path().join("no-apple-helper")));
    let engines = Registry::with(apple, local);
    engines.probe();
    let offered = [
        entry(SYSTEM, Engine::AppleBuiltin, Verdict::Beta),
        local_entry.clone(),
    ];
    let cards = engines.cards(&offered);
    let card = cards.iter().find(|c| c.model == LOCAL).unwrap();
    assert_eq!(card.ready, Readiness::Ready);
    assert!(card.uses_download && !card.running);

    let mac = MacFacts {
        chip: HostChip::Apple(Chip::M2),
        memory_gb: 32,
        free_disk_bytes: u64::MAX / 2,
        apple_ai: AppleAi::Unavailable,
    };
    let beta = |job: Job| job == Job::Review;
    let jobs = answer_for(
        &engines,
        &offered,
        std::slice::from_ref(&local_entry),
        Some(&mac),
        &Snapshot::default(),
        &beta,
        &BTreeMap::new(),
    );
    let review = jobs.iter().find(|info| info.job == AiJob::Review).unwrap();
    assert_eq!(review.state, AiJobState::Ready);
    assert_eq!(review.model_id.as_deref(), Some(LOCAL));
    assert_eq!(review.why, Some(AiJobWhy::Recommended));
    let chat = jobs.iter().find(|info| info.job == AiJob::Chat).unwrap();
    assert_eq!(chat.state, AiJobState::BetaOff, "Chat's Beta is off");
    engines.release();
}
