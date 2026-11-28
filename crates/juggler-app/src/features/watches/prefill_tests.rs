//! AJ-E3830 (0.5.2): the pre-fill's rules, field by field, and its bar:
//! never a wrong pre-filled field. The rounds' sets (harness v6 to v13,
//! `docs/qa/evidence/AJ-E3709/heldout/`, every item, their misses among
//! them) run through the app's own drafting with lying labellers, and no
//! filled field may differ from the set's gold.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test: a failed step fails the test"
)]

use std::time::{Duration, Instant};

use serde_json::Value;

use super::*;
use crate::features::ai::engine::{Draft, Finish, Request, Stop, Usage};
use crate::features::ai::run::{AiFailure, JobModel};
use crate::features::watches::draft::Candidate;
use crate::features::watches::model::Param;

/// A labeller: for each label call, `slot(k, n)` labels the k-th of `n`
/// words; call 1 (the target pick) answers the last candidate there is,
/// a pick code never checks.
struct Labeller {
    slot: fn(usize, usize) -> &'static str,
    /// One slot short (a wrong length, refused and asked again).
    short: bool,
}

impl JobModel for Labeller {
    fn generate(&self, request: &Request, _stop: &Stop) -> Result<Draft, AiFailure> {
        let shape = request.shape.clone().unwrap_or(Value::Null);
        let text = match shape["name"].as_str() {
            Some("WatchWordSlots") => {
                let n = shape["properties"][0]["schema"]["min"]
                    .as_u64()
                    .unwrap_or(0) as usize;
                let n_said = if self.short { n.saturating_sub(1) } else { n };
                let slots: Vec<&str> = (0..n_said).map(|k| (self.slot)(k, n)).collect();
                serde_json::json!({ "slots": slots }).to_string()
            }
            Some("WatchCandidate") => {
                let values = shape["properties"][0]["schema"]["values"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let last = values
                    .iter()
                    .rev()
                    .find(|v| v.as_str() != Some("none"))
                    .and_then(Value::as_str)
                    .unwrap_or("none");
                serde_json::json!({ "target": last }).to_string()
            }
            _ => return Err(AiFailure::Refused),
        };
        Ok(Draft {
            text,
            finish: Finish::Stop,
            usage: Usage::default(),
        })
    }
}

const LIARS: [(&str, Labeller); 9] = [
    (
        "filler",
        Labeller {
            slot: |_, _| "filler",
            short: false,
        },
    ),
    (
        "lifetime",
        Labeller {
            slot: |_, _| "lifetime",
            short: false,
        },
    ),
    (
        "target",
        Labeller {
            slot: |_, _| "target",
            short: false,
        },
    ),
    (
        "quiet_hours",
        Labeller {
            slot: |_, _| "quiet_hours",
            short: false,
        },
    ),
    (
        "condition",
        Labeller {
            slot: |_, _| "condition",
            short: false,
        },
    ),
    (
        "other",
        Labeller {
            slot: |_, _| "other",
            short: false,
        },
    ),
    (
        "exclusion",
        Labeller {
            slot: |_, _| "exclusion",
            short: false,
        },
    ),
    (
        "mixed",
        Labeller {
            slot: |k, _| ["lifetime", "filler", "target", "quiet_hours"][k % 4],
            short: false,
        },
    ),
    (
        "short",
        Labeller {
            slot: |_, _| "filler",
            short: true,
        },
    ),
];

/// What each kind can report, more than enough (more on offer is more to
/// fill, a harder test).
const SESSION: &[&str] = &[
    "finishes",
    "ends",
    "asks_you",
    "fails",
    "stuck",
    "idle_for",
    "hits_limit",
    "leaves_processes",
];
const GROUP: &[&str] = &[
    "finishes",
    "ends",
    "asks_you",
    "fails",
    "stuck",
    "idle_for",
    "starts",
    "hits_limit",
    "leaves_processes",
];
const ACCOUNT: &[&str] = &["hits_limit", "runway_below", "runway_resets"];

/// A set item's targets as the app's candidates, every condition its kind
/// can report on offer (more on offer is more to fill, a harder test).
fn candidates(item: &Value) -> Vec<Candidate> {
    let c = &item["context"];
    let mut out = Vec::new();
    for (key, kind, offered) in [
        ("sessions", "session", SESSION),
        ("apps", "app", GROUP),
        ("projects", "project", GROUP),
        ("accounts", "account", ACCOUNT),
    ] {
        for e in c[key].as_array().into_iter().flatten() {
            let session = kind == "session";
            let shaped = serde_json::json!({
                "kind": kind,
                "id": e["id"],
                "name": e.get("title").unwrap_or(&e["name"]),
                "aliases": e.get("aliases").cloned().unwrap_or_default(),
                "app": if session { e["app"].clone() } else { Value::Null },
                "project": if session { e["project"].clone() } else { Value::Null },
                "offered": offered,
            });
            out.extend(Candidate::from_json(&shaped));
        }
    }
    out
}

const HOUR: i64 = 3_600_000;

fn stop() -> Stop {
    Stop::at(Instant::now() + Duration::from_secs(10))
}

fn prefill_of(request: &str, cands: &[Candidate], model: &dyn JobModel) -> Option<WatchPrefill> {
    let ask = Ask {
        request,
        candidates: cands,
        midnight_ms: 0,
        now_ms: 9 * HOUR,
    };
    prefill(model, &ask, &stop(), &|_| None).map(|(p, _)| p)
}

/// The gold's lifetime as the form's choice (the sets' clock: midnight 0).
fn gold_lifetime(g: &str) -> Option<LifetimeChoice> {
    match g {
        "once" => Some(LifetimeChoice::Once),
        "until_removed" => Some(LifetimeChoice::Always),
        _ => {
            let at = g.strip_prefix("until:")?;
            let hm = at.split('T').nth(1)?;
            let h: i64 = hm.get(0..2)?.parse().ok()?;
            let m: i64 = hm.get(3..5)?.parse().ok()?;
            Some(if (h, m) == (23, 59) {
                LifetimeChoice::Today
            } else {
                LifetimeChoice::Until {
                    at_ms: h * HOUR + m * 60_000,
                }
            })
        }
    }
}

fn gold_target(g: &Value) -> Option<WatchTarget> {
    if g["kind"].as_str()? == "any" {
        return Some(WatchTarget::Any);
    }
    Candidate::from_json(g).map(|c| c.target)
}

fn gold_conditions(g: &Value) -> Vec<(String, Option<u32>)> {
    let mut out: Vec<(String, Option<u32>)> = g
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| {
            let code = c["type"].as_str().unwrap_or_default();
            let code = if code == "asks" { "asks_you" } else { code };
            (
                code.to_owned(),
                c["param"].as_u64().and_then(|n| u32::try_from(n).ok()),
            )
        })
        .collect();
    out.sort();
    out
}

fn said_conditions(cs: &[Condition]) -> Vec<(String, Option<u32>)> {
    let mut out: Vec<(String, Option<u32>)> = cs
        .iter()
        .map(|c| {
            let n = match c.param {
                Some(Param::Minutes(n)) => Some(n),
                Some(Param::Percent(n)) => Some(u32::from(n)),
                _ => None,
            };
            (c.code.clone(), n)
        })
        .collect();
    out.sort();
    out
}

/// The filled fields that differ from the gold (none is the bar).
fn wrong_fields(item: &Value, p: &WatchPrefill) -> Vec<String> {
    let g = &item["gold"];
    let mut wrong = Vec::new();
    if g["supported"].as_bool() != Some(true) {
        // Nothing can be watched as asked: no condition may be filled.
        if !p.draft.conditions.is_empty() {
            wrong.push(format!(
                "condition on an unsupported request: {:?}",
                p.draft.conditions
            ));
        }
        return wrong;
    }
    if let Some(t) = p.draft.targets.first()
        && gold_target(&g["target"]).as_ref() != Some(t)
    {
        wrong.push(format!("target {t:?}"));
    }
    let mut gold = gold_conditions(&g["conditions"]);
    // v6 and v7's gold predates the idle condition (v8 split "idle" from
    // "stuck"; the app reads an idle session as `idle_for`).
    let request = item["request"].as_str().unwrap_or_default().to_lowercase();
    if old_gold(item) && request.contains("idle") {
        for c in &mut gold {
            if c.0 == "stuck" {
                c.0 = "idle_for".to_owned();
            }
        }
        gold.sort();
    }
    if !p.draft.conditions.is_empty() && said_conditions(&p.draft.conditions) != gold {
        wrong.push(format!("conditions {:?}", p.draft.conditions));
    }
    let life_filled =
        !p.left.contains(&PrefillField::Lifetime) && p.draft.lifetime != LifetimeChoice::Unsaid;
    if life_filled
        && gold_lifetime(g["lifetime"].as_str().unwrap_or_default()) != Some(p.draft.lifetime)
    {
        wrong.push(format!("lifetime {:?}", p.draft.lifetime));
    }
    let gold_quiet = g["even_in_quiet_hours"].as_bool().unwrap_or(false);
    if !p.left.contains(&PrefillField::QuietHours) && p.draft.even_in_quiet_hours != gold_quiet {
        wrong.push(format!("quiet hours {}", p.draft.even_in_quiet_hours));
    }
    wrong
}

/// An item of harness v6 or v7 (their ids start so).
fn old_gold(item: &Value) -> bool {
    item["id"]
        .as_str()
        .is_some_and(|id| id.starts_with("V6-") || id.starts_with("V7-"))
}

fn set(v: u32) -> Vec<Value> {
    let path = format!(
        "{}/../../docs/qa/evidence/AJ-E3709/heldout/v{v}/watch-v{v}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let raw = std::fs::read_to_string(&path).expect("the round's set");
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    parsed
        .as_array()
        .cloned()
        .or_else(|| parsed["items"].as_array().cloned())
        .unwrap_or_default()
}

/// The bar, over every request of harness v6 to v13 (480, each round's
/// misses among them), with nine labellers that lie in every way a label
/// can (all filler, all lifetime, all target, all quiet hours, all
/// condition, all other, all exclusion, a mix, a wrong length) and a
/// target pick code never checks: no filled field differs from the gold.
#[test]
fn no_labeller_makes_a_pre_filled_field_wrong_on_any_round_s_set() {
    let mut wrong = Vec::new();
    let mut filled = 0;
    for v in 6..=13 {
        for item in set(v) {
            let cands = candidates(&item);
            let request = item["request"].as_str().unwrap_or_default();
            for (name, liar) in &LIARS {
                let Some(p) = prefill_of(request, &cands, liar) else {
                    continue;
                };
                filled += 4 - p.left.len();
                for w in wrong_fields(&item, &p) {
                    wrong.push(format!("v{v} {} [{name}] {w}", item["id"]));
                }
            }
        }
    }
    assert!(filled > 0, "something is filled");
    assert!(
        wrong.is_empty(),
        "wrong pre-filled fields:\n{}",
        wrong.join("\n")
    );
}

/// The same with no model at all (code alone): nothing filled is wrong.
#[test]
fn code_alone_never_fills_a_wrong_field() {
    struct NoModel;
    impl JobModel for NoModel {
        fn generate(&self, _r: &Request, _s: &Stop) -> Result<Draft, AiFailure> {
            Err(AiFailure::TimedOut)
        }
    }
    let mut wrong = Vec::new();
    for v in 6..=13 {
        for item in set(v) {
            let cands = candidates(&item);
            let request = item["request"].as_str().unwrap_or_default();
            if let Some(p) = prefill_of(request, &cands, &NoModel) {
                for w in wrong_fields(&item, &p) {
                    wrong.push(format!("v{v} {} {w}", item["id"]));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "wrong pre-filled fields:\n{}",
        wrong.join("\n")
    );
}

// ============================================================================ each field's rule

/// v13's names, in a made-up context of the same shape.
fn v13_context() -> Vec<Candidate> {
    let one = |kind: &str, id: &str, name: &str, project: Option<&str>| {
        Candidate::from_json(&serde_json::json!({
            "kind": kind, "id": id, "name": name, "app": project.map(|_| "Codex"), "project": project,
            "offered": if kind == "session" { SESSION } else { ACCOUNT },
        }))
        .unwrap()
    };
    vec![
        one("session", "s-inv", "Rechnungsexport", Some("kassen")),
        one("session", "s-dark", "Dark mode toggle", Some("studio")),
        one("session", "s-notes", "Release notes", Some("docs-site")),
        one("session", "s-rate", "Rate limiter", Some("gate")),
        one("session", "s-flaky", "Flaky tests", Some("ci-box")),
        one("session", "s-auth", "Auth migration", Some("gate")),
        one("account", "a-hangar", "Hangar Max", None),
        one("account", "a-shop", "Werkstatt Pro", None),
    ]
}

/// The words the labels give, by word: a model that labels what it was
/// asked about by meaning (as Apple's did in v13).
fn by_meaning(map: &'static [(&'static str, &'static str)]) -> impl JobModel {
    struct Meaning(&'static [(&'static str, &'static str)]);
    impl JobModel for Meaning {
        fn generate(&self, request: &Request, _stop: &Stop) -> Result<Draft, AiFailure> {
            let shape = request.shape.clone().unwrap_or(Value::Null);
            if shape["name"].as_str() != Some("WatchWordSlots") {
                return Err(AiFailure::Refused);
            }
            let asked = request
                .prompt
                .split("in this order): ")
                .nth(1)
                .and_then(|t| t.split("\n\n").next())
                .unwrap_or_default();
            let slots: Vec<&str> = asked
                .split(" · ")
                .map(|w| {
                    let word = w.split_once(' ').map_or("", |(_, w)| w);
                    self.0
                        .iter()
                        .find(|(k, _)| *k == word)
                        .map_or("other", |(_, v)| *v)
                })
                .collect();
            Ok(Draft {
                text: serde_json::json!({ "slots": slots }).to_string(),
                finish: Finish::Stop,
                usage: Usage::default(),
            })
        }
    }
    Meaning(map)
}

/// v13's W03: an unread lifetime leaves only the lifetime; the target and
/// the condition code read are filled.
#[test]
fn an_unread_lifetime_leaves_only_the_lifetime() {
    let model = by_meaning(&[
        ("bis", "lifetime"),
        ("ich", "lifetime"),
        ("entferne", "lifetime"),
        ("jedes", "lifetime"),
        ("mal", "lifetime"),
    ]);
    let p = prefill_of(
        "Sag mir jedes Mal Bescheid, wenn der Rechnungsexport fertig ist, bis ich es entferne.",
        &v13_context(),
        &model,
    )
    .unwrap();
    assert_eq!(p.draft.targets, vec![WatchTarget::Session("s-inv".into())]);
    assert_eq!(
        said_conditions(&p.draft.conditions),
        vec![("finishes".to_owned(), None)]
    );
    assert!(p.left.contains(&PrefillField::Lifetime), "{p:?}");
    assert_eq!(p.draft.lifetime, LifetimeChoice::Unsaid);
    assert!(!p.card);
}

/// v13's W32: "yes, even overnight" unread leaves quiet hours off for the
/// user to check; the account and its limit are filled.
#[test]
fn unread_quiet_hours_stay_off_and_are_left() {
    let model = by_meaning(&[
        ("yes", "filler"),
        ("even", "quiet_hours"),
        ("overnight", "quiet_hours"),
    ]);
    let p = prefill_of(
        "Tell me when Hangar Max hits its limit, yes, even overnight.",
        &v13_context(),
        &model,
    )
    .unwrap();
    assert_eq!(
        p.draft.targets,
        vec![WatchTarget::Account("a-hangar".into())]
    );
    assert_eq!(
        said_conditions(&p.draft.conditions),
        vec![("hits_limit".to_owned(), None)]
    );
    if p.left.contains(&PrefillField::QuietHours) {
        assert!(!p.draft.even_in_quiet_hours);
    } else {
        assert!(p.draft.even_in_quiet_hours, "{p:?}");
    }
}

/// v13's W28: "wants something from me" is no cue code knows: the
/// session is filled, the condition left with the nearest beside it, never
/// chosen.
#[test]
fn an_unread_condition_is_left_with_the_nearest_offered_not_chosen() {
    let model = by_meaning(&[
        ("wants", "condition"),
        ("something", "condition"),
        ("from", "condition"),
    ]);
    let p = prefill_of(
        "let me know when the release notes session wants something from me",
        &v13_context(),
        &model,
    )
    .unwrap();
    assert_eq!(
        p.draft.targets,
        vec![WatchTarget::Session("s-notes".into())]
    );
    assert!(p.draft.conditions.is_empty(), "{p:?}");
    assert!(p.left.contains(&PrefillField::Condition));
    assert_eq!(p.why, PrefillWhy::Unread);
    assert!(p.asked.is_some());
}

/// v13's W19: "yo" alone in its clause, labelled other: the session stays
/// (its clause holds no unread word); what "other" may say is left.
#[test]
fn an_other_word_in_its_own_clause_leaves_the_target_filled() {
    let model = by_meaning(&[("yo", "other")]);
    let p = prefill_of(
        "yo, shout when the dark mode toggle is done",
        &v13_context(),
        &model,
    )
    .unwrap();
    assert_eq!(p.draft.targets, vec![WatchTarget::Session("s-dark".into())]);
    assert!(p.draft.conditions.is_empty());
    assert!(p.left.contains(&PrefillField::Lifetime) && p.left.contains(&PrefillField::QuietHours));
}

/// A word that says something of its own beside the condition ("in
/// staging") spoils the condition, whatever its label claims.
#[test]
fn a_qualifier_beside_the_condition_spoils_it_whatever_its_label() {
    for label in ["lifetime", "quiet_hours", "target", "filler"] {
        let map: &'static [(&'static str, &'static str)] = match label {
            "lifetime" => &[("staging", "lifetime"), ("box", "lifetime")],
            "quiet_hours" => &[("staging", "quiet_hours"), ("box", "quiet_hours")],
            "target" => &[("staging", "target"), ("box", "target")],
            _ => &[("staging", "filler"), ("box", "filler")],
        };
        let p = prefill_of(
            "Tell me when the auth migration fails on the staging box",
            &v13_context(),
            &by_meaning(map),
        )
        .unwrap();
        assert!(p.draft.conditions.is_empty(), "{label}: {p:?}");
    }
}

/// Every field verified (code's own draft): filled whole, offered as a
/// card once the caller's Beta allows (`card` is the form's check here).
#[test]
fn a_request_code_reads_whole_is_filled_whole_and_may_be_a_card() {
    let none = by_meaning(&[]);
    let p = prefill_of(
        "Tell me when the flaky tests session ends, today",
        &v13_context(),
        &none,
    );
    let p = p.unwrap();
    assert!(p.left.is_empty(), "{p:?}");
    assert_eq!(p.why, PrefillWhy::Verified);
    assert_eq!(
        p.draft.targets,
        vec![WatchTarget::Session("s-flaky".into())]
    );
    assert_eq!(
        said_conditions(&p.draft.conditions),
        vec![("ends".to_owned(), None)]
    );
    assert_eq!(p.draft.lifetime, LifetimeChoice::Today);
    assert!(p.card, "the form's check passed");
    let ask = Ask {
        request: "Tell me when the flaky tests session ends, today",
        candidates: &v13_context(),
        midnight_ms: 0,
        now_ms: 9 * HOUR,
    };
    let refused = prefill(&none, &ask, &stop(), &|_| Some("no".into()))
        .unwrap()
        .0;
    assert!(!refused.card, "a draft the form refuses is never a card");
}

/// The model's pick among several names is never a filled target.
#[test]
fn the_model_s_pick_among_several_is_never_filled() {
    let cands: Vec<Candidate> = [("s-a", "api tests"), ("s-b", "api docs")]
        .iter()
        .filter_map(|(id, name)| {
            Candidate::from_json(&serde_json::json!({ "kind": "session", "id": id, "name": name, "offered": SESSION }))
        })
        .collect();
    let liar = Labeller {
        slot: |_, _| "filler",
        short: false,
    };
    if let Some(p) = prefill_of("Tell me when the api session finishes", &cands, &liar) {
        assert!(p.draft.targets.is_empty() || !p.card, "{p:?}");
        if p.why == PrefillWhy::Several {
            assert!(p.draft.targets.is_empty());
            assert!(p.left.contains(&PrefillField::Target));
        }
    }
}

/// A negation leaves the condition and the target in its clause; what it
/// leaves out is never the target.
#[test]
fn a_negation_never_fills_what_it_leaves_out() {
    let none = by_meaning(&[]);
    let p = prefill_of(
        "Tell me when any session fails except the auth migration",
        &v13_context(),
        &none,
    )
    .unwrap();
    assert!(p.draft.conditions.is_empty());
    assert_ne!(p.draft.targets, vec![WatchTarget::Session("s-auth".into())]);
    assert_ne!(p.draft.targets, vec![WatchTarget::Any]);
}

/// v12's W39: "unless it's the Rechnungsexport session" after its own
/// commas: the name after the negation is what it leaves out, never the
/// target (the told-back nearest named it).
#[test]
fn a_name_after_a_negation_is_never_the_target() {
    let none = by_meaning(&[]);
    for request in [
        "Sag mir, wenn eine Sitzung hängt, es sei denn, es ist die Rechnungsexport-Sitzung",
        "Tell me when a session is stuck, unless, of course, it is Rechnungsexport",
    ] {
        let p = prefill_of(request, &v13_context(), &none).unwrap();
        assert!(p.draft.targets.is_empty(), "{request}: {p:?}");
        assert!(p.draft.conditions.is_empty(), "{request}: {p:?}");
    }
}

/// Not a watch request: no pre-fill (Bot answers as it would).
#[test]
fn a_question_is_no_pre_fill() {
    let none = by_meaning(&[]);
    assert!(prefill_of("What is the rate limiter doing?", &v13_context(), &none).is_none());
}

/// The clauses: punctuation and a spaced dash split them.
#[test]
fn clauses_split_at_punctuation_and_dashes() {
    assert_eq!(clauses("yo, shout when it is done"), vec![0, 1, 1, 1, 1, 1]);
    assert_eq!(clauses("tell me - api: done"), vec![0, 0, 1, 2]);
}

/// The RW1 re-walk's lab, as Bot's candidates hold it: the project web,
/// an open session in it not titled yet (named by its folder, or by the
/// one rule's facts), a failed one in it, and a session in api.
fn rw1_context(open_name: &str) -> Vec<Candidate> {
    let one = |kind: &str, id: &str, name: &str, project: Option<&str>| {
        Candidate::from_json(&serde_json::json!({
            "kind": kind, "id": id, "name": name, "app": project.map(|_| "Work"), "project": project,
            "offered": if kind == "session" { SESSION } else { GROUP },
        }))
        .unwrap()
    };
    vec![
        one("session", "s-open", open_name, Some("web")),
        one(
            "session",
            "s-failed",
            "Rewrite the date parser to accept two-digit years",
            Some("web"),
        ),
        one(
            "session",
            "s-api",
            "Add retries to the sync client",
            Some("api"),
        ),
        one("app", "work", "Work", None),
        one("project", "p-web", "web", None),
        one("project", "p-api", "api", None),
    ]
}

/// 0.5.2 G60 (RW1 R31): "web" names the project and the open session in
/// it: the target is left, never guessed, and the picker opens on both,
/// the open session among them; the rest is as read (the lifetime unsaid:
/// the form's suggestion once a target is picked).
#[test]
fn an_ambiguous_name_leaves_the_target_with_its_choices() {
    for open in ["web", "web · started 3:23 AM"] {
        let p = prefill_of(
            "tell me when web finishes",
            &rw1_context(open),
            &by_meaning(&[]),
        )
        .unwrap();
        assert!(p.draft.targets.is_empty(), "{open}: {p:?}");
        assert!(p.left.contains(&PrefillField::Target), "{open}: {p:?}");
        assert!(
            p.choices.contains(&WatchTarget::Session("s-open".into())),
            "{open}: {p:?}"
        );
        assert!(
            p.choices.contains(&WatchTarget::Project("p-web".into())),
            "{open}: {p:?}"
        );
        assert!(
            !p.choices.contains(&WatchTarget::Session("s-api".into())),
            "{open}: {p:?}"
        );
        assert_eq!(p.draft.lifetime, LifetimeChoice::Unsaid, "{open}");
        assert!(!p.card);
    }
}

/// 0.5.2 G60 (RW1 R32): words code couldn't read beside the condition
/// leave it, and the condition its own cue words name is offered beside
/// the cards as the nearest, never chosen; the name before the negation
/// still opens the picker on its matches.
#[test]
fn an_unread_qualifier_leaves_the_condition_with_its_cue_as_nearest() {
    let p = prefill_of(
        "tell me when web finishes unless it's after dark",
        &rw1_context("web · started 3:23 AM"),
        &by_meaning(&[]),
    )
    .unwrap();
    assert!(p.draft.conditions.is_empty(), "{p:?}");
    assert!(p.left.contains(&PrefillField::Condition), "{p:?}");
    assert_eq!(p.nearest.as_deref(), Some("finishes"), "{p:?}");
    assert!(p.draft.targets.is_empty(), "{p:?}");
    assert!(
        p.choices.contains(&WatchTarget::Project("p-web".into())),
        "{p:?}"
    );
}

/// A filled target opens no picker: no choices beside it.
#[test]
fn a_filled_target_has_no_choices() {
    let p = prefill_of(
        "Tell me when the flaky tests session ends, today",
        &v13_context(),
        &by_meaning(&[]),
    )
    .unwrap();
    assert!(!p.draft.targets.is_empty());
    assert!(p.choices.is_empty(), "{p:?}");
}
