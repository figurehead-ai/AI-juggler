//! The one query end to end over a fake Apple helper: the compiled flags
//! and D-131's verdicts, the Beta switches, Apple Intelligence turned off
//! mid-session, a remote kill, a narrowed verdict, the recommendation as
//! Automatic, and laziness.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::json;

use super::{Probe, Service, answer_for};
use crate::features::ai::dto::{AiJob, AiJobInfo, AiJobState, AiJobWhy};
use crate::features::ai::engines::Registry;
use crate::features::ai::engines::apple::Apple;
use crate::features::ai::engines::local::Local;
use crate::features::ai::models::recommend::{AppleAi, HostChip, MacFacts};
use crate::features::ai::models::{Chip, Job, builtin};
use crate::features::remote_config::Snapshot;
use crate::features::remote_config::snapshot_fixtures::{facts, file, registry};

#[cfg(unix)]
fn helper(dir: &Path, available: &str) -> std::path::PathBuf {
    let path = dir.join("helper");
    let script = format!(
        "#!/bin/sh\necho '{{\"hello\":true,\"protocol\":2,\"available\":\"{available}\",\"context_size\":8192}}'\ncat > /dev/null\n"
    );
    crate::test_support::script(&path, &script)
}

/// The engines with Apple's helper (Apple Intelligence available) and
/// the local engine's own, nothing installed.
#[cfg(unix)]
fn apple_only(dir: &Path) -> Registry {
    Registry::with(
        Apple::with_program(Some(helper(dir, "available"))),
        Local::default(),
    )
}

fn state_of(jobs: &[AiJobInfo], job: AiJob) -> (AiJobState, bool, Option<AiJobWhy>) {
    let info = jobs.iter().find(|info| info.job == job).unwrap();
    (info.state, info.beta, info.why)
}

fn beta_for(on: &'static [Job]) -> impl Fn(Job) -> bool {
    move |job| on.contains(&job)
}

#[cfg(unix)]
#[test]
fn on_a_mac_with_apple_intelligence_bot_waits_for_its_beta() {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::new(apple_only(dir.path()));
    let compiled = Snapshot::default();
    let (jobs, changed) = service.answer(
        &compiled,
        &beta_for(&[]),
        &BTreeMap::new(),
        dir.path(),
        Probe::IfStale,
    );
    assert!(changed, "the first answer is news");
    assert_eq!(jobs.len(), 5, "every routed job, attention aside");
    for job in [AiJob::Feed, AiJob::Digest, AiJob::Watches] {
        assert_eq!(
            state_of(&jobs, job).0,
            AiJobState::Off,
            "{job:?}: not yet (D-131)"
        );
    }
    assert_eq!(
        state_of(&jobs, AiJob::Review).0,
        AiJobState::Off,
        "the review waits for AJ-E3820 (the 0.5 walk's F-02)"
    );
    assert_eq!(state_of(&jobs, AiJob::Bot).0, AiJobState::BetaOff);

    let (jobs, changed) = service.answer(
        &compiled,
        &beta_for(&[Job::Bot]),
        &BTreeMap::new(),
        dir.path(),
        Probe::Never,
    );
    assert!(changed);
    let chat = jobs.iter().find(|info| info.job == AiJob::Bot).unwrap();
    assert_eq!(
        (chat.state, chat.beta, chat.beta_on),
        (AiJobState::Ready, true, true)
    );
    assert_eq!(chat.model_id.as_deref(), Some(builtin()[0].id.as_str()));
    assert_eq!(chat.model_name.as_deref(), Some(builtin()[0].name.as_str()));
    assert!(
        matches!(chat.why, Some(AiJobWhy::Recommended | AiJobWhy::Only)),
        "{chat:?}"
    );
    assert_eq!(
        state_of(&jobs, AiJob::Review).0,
        AiJobState::Off,
        "Bot's Beta turns on Bot only; the review waits for AJ-E3820"
    );

    let (_, changed) = service.answer(
        &compiled,
        &beta_for(&[Job::Bot]),
        &BTreeMap::new(),
        dir.path(),
        Probe::IfStale,
    );
    assert!(
        !changed,
        "the same answer isn't news, and a fresh probe isn't repeated"
    );
}

#[cfg(unix)]
#[test]
fn apple_intelligence_turned_off_mid_session_hides_the_jobs_with_the_reason() {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::new(apple_only(dir.path()));
    let compiled = Snapshot::default();
    let beta = beta_for(&[Job::Review, Job::Bot]);
    let (jobs, _) = service.answer(&compiled, &beta, &BTreeMap::new(), dir.path(), Probe::Now);
    assert_eq!(state_of(&jobs, AiJob::Bot).0, AiJobState::Ready);
    helper(dir.path(), "apple_intelligence_not_enabled");
    let (jobs, changed) =
        service.answer(&compiled, &beta, &BTreeMap::new(), dir.path(), Probe::Now);
    assert!(changed);
    assert_eq!(
        state_of(&jobs, AiJob::Bot),
        (AiJobState::NoEngine, false, Some(AiJobWhy::TurnedOff))
    );
    assert_eq!(state_of(&jobs, AiJob::Review).0, AiJobState::Off);
    helper(dir.path(), "available");
    let (jobs, changed) =
        service.answer(&compiled, &beta, &BTreeMap::new(), dir.path(), Probe::Now);
    assert!(changed, "turned back on, the jobs return");
    assert_eq!(state_of(&jobs, AiJob::Bot).0, AiJobState::Ready);
}

#[cfg(unix)]
#[test]
fn a_remote_kill_and_a_narrowed_verdict_each_hide_a_job() {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::new(apple_only(dir.path()));
    let beta = beta_for(&[Job::Review, Job::Bot]);
    let killed = Snapshot::build(
        &registry(),
        Some(&file(
            json!({ "flags": { "schema": 1, "values": { "ai_chat_on": false, "ai_feed_on": true } } }),
        )),
        &facts(1),
    );
    let (jobs, _) = service.answer(&killed, &beta, &BTreeMap::new(), dir.path(), Probe::Now);
    assert_eq!(
        state_of(&jobs, AiJob::Bot).0,
        AiJobState::Off,
        "the kill switch"
    );
    assert_eq!(
        state_of(&jobs, AiJob::Feed).0,
        AiJobState::Off,
        "an OffOnly flag is never turned on remotely"
    );
    assert_eq!(
        state_of(&jobs, AiJob::Review).0,
        AiJobState::Off,
        "off by default until AJ-E3820"
    );

    let mut apple = serde_json::to_value(json!({
        "id": builtin()[0].id, "name": "Built-in", "engine": "apple_builtin", "min_chip": "m1",
        "min_memory_gb": 8, "review": "beta", "chat": "not_yet", "attention": "ships"
    }))
    .unwrap();
    let narrowed = Snapshot::build(
        &registry(),
        Some(&file(
            json!({ "models": { "schema": 1, "items": [apple.clone()] } }),
        )),
        &facts(1),
    );
    let (jobs, _) = service.answer(
        &narrowed,
        &beta_for(&[]),
        &BTreeMap::new(),
        dir.path(),
        Probe::Never,
    );
    assert_eq!(
        state_of(&jobs, AiJob::Bot).0,
        AiJobState::NoEngine,
        "not_yet: the plain path"
    );

    apple["chat"] = json!("ships");
    let shipped = Snapshot::build(
        &registry(),
        Some(&file(
            json!({ "models": { "schema": 1, "items": [apple.clone()] } }),
        )),
        &facts(1),
    );
    let (jobs, _) = service.answer(
        &shipped,
        &beta_for(&[]),
        &BTreeMap::new(),
        dir.path(),
        Probe::Never,
    );
    assert_eq!(
        state_of(&jobs, AiJob::Bot).0,
        AiJobState::Ready,
        "ships needs no Beta switch"
    );
    assert!(!state_of(&jobs, AiJob::Bot).1, "and isn't labelled Beta");

    apple["withdrawn"] = json!(true);
    let withdrawn = Snapshot::build(
        &registry(),
        Some(&file(
            json!({ "models": { "schema": 1, "items": [apple] } }),
        )),
        &facts(1),
    );
    let (jobs, _) = service.answer(
        &withdrawn,
        &beta,
        &BTreeMap::new(),
        dir.path(),
        Probe::Never,
    );
    assert_eq!(
        state_of(&jobs, AiJob::Bot).0,
        AiJobState::NoEngine,
        "a withdrawn model is offered for nothing"
    );
}

#[cfg(unix)]
#[test]
fn nothing_is_started_before_a_query_asks() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("started");
    let program = dir.path().join("helper");
    crate::test_support::script(
        &program,
        &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    );
    let service = Service::new(Registry::with(
        Apple::with_program(Some(program)),
        Local::default(),
    ));
    let (jobs, _) = service.answer(
        &Snapshot::default(),
        &beta_for(&[Job::Bot]),
        &BTreeMap::new(),
        dir.path(),
        Probe::Never,
    );
    assert!(!marker.exists(), "a re-answer never starts the helper");
    assert_eq!(
        state_of(&jobs, AiJob::Bot),
        (AiJobState::NoEngine, false, Some(AiJobWhy::Unknown)),
        "not asked yet: unknown fails closed"
    );
    let _ = service.answer(
        &Snapshot::default(),
        &beta_for(&[Job::Bot]),
        &BTreeMap::new(),
        dir.path(),
        Probe::IfStale,
    );
    assert!(marker.exists(), "the first query asks");
}

/// The recommendation is the router's Automatic default (AJ-E3811): with
/// Apple Intelligence on it names Apple's model; with it off nothing is
/// recommended and Apple's card, if it could still run, is the only one.
#[cfg(unix)]
#[test]
fn the_recommendation_is_the_automatic_default() {
    let dir = tempfile::tempdir().unwrap();
    let engines = apple_only(dir.path());
    engines.probe();
    let mac = |apple_ai| MacFacts {
        chip: HostChip::Apple(Chip::M4),
        memory_gb: 16,
        free_disk_bytes: 200_000_000_000,
        apple_ai,
    };
    let chat = |mac: Option<&MacFacts>| {
        let jobs = answer_for(
            &engines,
            &builtin(),
            &[],
            mac,
            &Snapshot::default(),
            &beta_for(&[Job::Bot]),
            &BTreeMap::new(),
        );
        state_of(&jobs, AiJob::Bot)
    };
    assert_eq!(
        chat(Some(&mac(AppleAi::On))),
        (AiJobState::Ready, true, Some(AiJobWhy::Recommended))
    );
    assert_eq!(
        chat(Some(&mac(AppleAi::Off))),
        (AiJobState::Ready, true, Some(AiJobWhy::Only))
    );
    assert_eq!(chat(None), (AiJobState::Ready, true, Some(AiJobWhy::Only)));
}

/// AJ-E3812's per-job override: the user's pick wins and the answer says
/// so, and carries the pick; only a model whose verdict passed the job may
/// be picked (D-127).
#[cfg(unix)]
#[test]
fn a_pick_wins_and_only_a_model_that_passed_may_be_picked() {
    let dir = tempfile::tempdir().unwrap();
    let engines = apple_only(dir.path());
    engines.probe();
    let off = MacFacts {
        chip: HostChip::Apple(Chip::M4),
        memory_gb: 16,
        free_disk_bytes: 200_000_000_000,
        apple_ai: AppleAi::Off,
    };
    let picks = BTreeMap::from([(Job::Bot, "apple-on-device".to_owned())]);
    let jobs = answer_for(
        &engines,
        &builtin(),
        &[],
        Some(&off),
        &Snapshot::default(),
        &beta_for(&[Job::Bot]),
        &picks,
    );
    let chat = jobs.iter().find(|info| info.job == AiJob::Bot).unwrap();
    assert_eq!(chat.why, Some(AiJobWhy::Picked));
    assert_eq!(chat.pick.as_deref(), Some("apple-on-device"));
    let review = jobs.iter().find(|info| info.job == AiJob::Review).unwrap();
    assert_eq!(review.pick, None, "Automatic");
    // Apple's model passed review and Chat, not the feed line.
    assert!(super::may_pick(&builtin(), Job::Bot, "apple-on-device"));
    assert!(!super::may_pick(&builtin(), Job::Feed, "apple-on-device"));
    assert!(!super::may_pick(&builtin(), Job::Bot, "no-such-model"));
}

/// The query on this Mac with the real helper (run by hand after
/// `scripts/build_apple_ai_helper.sh`): every job with the Beta switches
/// off, then with the review's and Chat's on.
#[test]
#[ignore = "needs the built helper; run by hand"]
fn the_query_on_this_mac() {
    let exe = std::env::current_exe().unwrap();
    let program = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(crate::features::ai::engines::apple::PROGRAM);
    let dir = tempfile::tempdir().unwrap();
    let service = Service::new(Registry::with(
        Apple::with_program(Some(program)),
        Local::default(),
    ));
    for (label, on) in [
        ("beta off", &[][..]),
        ("beta on", &[Job::Review, Job::Bot][..]),
    ] {
        let beta = move |job: Job| on.contains(&job);
        let (jobs, _) = service.answer(
            &Snapshot::default(),
            &beta,
            &BTreeMap::new(),
            dir.path(),
            Probe::Now,
        );
        for info in jobs {
            println!("{label}: {}", serde_json::to_string(&info).unwrap());
        }
    }
}
