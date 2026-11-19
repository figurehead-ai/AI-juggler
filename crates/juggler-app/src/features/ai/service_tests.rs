//! The one query end to end over a fake Apple helper: the compiled flags
//! and D-131's verdicts, the Beta switches, Apple Intelligence turned off
//! mid-session, a remote kill, a narrowed verdict, the recommendation as
//! Automatic, and laziness.

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
    use std::os::unix::fs::PermissionsExt as _;
    let path = dir.join("helper");
    let script = format!(
        "#!/bin/sh\necho '{{\"hello\":true,\"protocol\":1,\"available\":\"{available}\",\"context_size\":8192}}'\ncat > /dev/null\n"
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
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
fn on_a_mac_with_apple_intelligence_review_and_chat_wait_for_their_beta() {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::new(Registry::with(
        Apple::with_program(Some(helper(dir.path(), "available"))),
        Local::default(),
    ));
    let compiled = Snapshot::default();
    let (jobs, changed) = service.answer(&compiled, &beta_for(&[]), dir.path(), Probe::IfStale);
    assert!(changed, "the first answer is news");
    assert_eq!(jobs.len(), 5, "every routed job, attention aside");
    for job in [AiJob::Feed, AiJob::Digest, AiJob::Watches] {
        assert_eq!(
            state_of(&jobs, job).0,
            AiJobState::Off,
            "{job:?}: not yet (D-131)"
        );
    }
    assert_eq!(state_of(&jobs, AiJob::Review).0, AiJobState::BetaOff);
    assert_eq!(state_of(&jobs, AiJob::Chat).0, AiJobState::BetaOff);

    let (jobs, changed) =
        service.answer(&compiled, &beta_for(&[Job::Chat]), dir.path(), Probe::Never);
    assert!(changed);
    let chat = jobs.iter().find(|info| info.job == AiJob::Chat).unwrap();
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
        AiJobState::BetaOff,
        "one job's Beta only"
    );

    let (_, changed) = service.answer(
        &compiled,
        &beta_for(&[Job::Chat]),
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
    let service = Service::new(Registry::with(
        Apple::with_program(Some(helper(dir.path(), "available"))),
        Local::default(),
    ));
    let compiled = Snapshot::default();
    let beta = beta_for(&[Job::Review, Job::Chat]);
    let (jobs, _) = service.answer(&compiled, &beta, dir.path(), Probe::Now);
    assert_eq!(state_of(&jobs, AiJob::Chat).0, AiJobState::Ready);
    helper(dir.path(), "apple_intelligence_not_enabled");
    let (jobs, changed) = service.answer(&compiled, &beta, dir.path(), Probe::Now);
    assert!(changed);
    for job in [AiJob::Review, AiJob::Chat] {
        assert_eq!(
            state_of(&jobs, job),
            (AiJobState::NoEngine, false, Some(AiJobWhy::TurnedOff))
        );
    }
    helper(dir.path(), "available");
    let (jobs, changed) = service.answer(&compiled, &beta, dir.path(), Probe::Now);
    assert!(changed, "turned back on, the jobs return");
    assert_eq!(state_of(&jobs, AiJob::Chat).0, AiJobState::Ready);
}

#[cfg(unix)]
#[test]
fn a_remote_kill_and_a_narrowed_verdict_each_hide_a_job() {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::new(Registry::with(
        Apple::with_program(Some(helper(dir.path(), "available"))),
        Local::default(),
    ));
    let beta = beta_for(&[Job::Review, Job::Chat]);
    let killed = Snapshot::build(
        &registry(),
        Some(&file(
            json!({ "flags": { "schema": 1, "values": { "ai_chat_on": false, "ai_feed_on": true } } }),
        )),
        &facts(1),
    );
    let (jobs, _) = service.answer(&killed, &beta, dir.path(), Probe::Now);
    assert_eq!(
        state_of(&jobs, AiJob::Chat).0,
        AiJobState::Off,
        "the kill switch"
    );
    assert_eq!(
        state_of(&jobs, AiJob::Feed).0,
        AiJobState::Off,
        "an OffOnly flag is never turned on remotely"
    );
    assert_eq!(state_of(&jobs, AiJob::Review).0, AiJobState::Ready);

    let mut apple = serde_json::to_value(json!({
        "id": builtin()[0].id, "name": "Built-in", "engine": "apple_builtin", "min_chip": "m1",
        "min_memory_gb": 8, "review": "not_yet", "chat": "ships", "attention": "ships"
    }))
    .unwrap();
    let narrowed = Snapshot::build(
        &registry(),
        Some(&file(
            json!({ "models": { "schema": 1, "items": [apple.clone()] } }),
        )),
        &facts(1),
    );
    let (jobs, _) = service.answer(&narrowed, &beta_for(&[]), dir.path(), Probe::Never);
    assert_eq!(
        state_of(&jobs, AiJob::Review).0,
        AiJobState::NoEngine,
        "not_yet: the plain path"
    );
    assert_eq!(
        state_of(&jobs, AiJob::Chat).0,
        AiJobState::Ready,
        "ships needs no Beta switch"
    );
    assert!(!state_of(&jobs, AiJob::Chat).1, "and isn't labelled Beta");

    apple["withdrawn"] = json!(true);
    let withdrawn = Snapshot::build(
        &registry(),
        Some(&file(
            json!({ "models": { "schema": 1, "items": [apple] } }),
        )),
        &facts(1),
    );
    let (jobs, _) = service.answer(&withdrawn, &beta, dir.path(), Probe::Never);
    assert_eq!(
        state_of(&jobs, AiJob::Chat).0,
        AiJobState::NoEngine,
        "a withdrawn model is offered for nothing"
    );
}

#[cfg(unix)]
#[test]
fn nothing_is_started_before_a_query_asks() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("started");
    let program = dir.path().join("helper");
    std::fs::write(
        &program,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let service = Service::new(Registry::with(
        Apple::with_program(Some(program)),
        Local::default(),
    ));
    let (jobs, _) = service.answer(
        &Snapshot::default(),
        &beta_for(&[Job::Chat]),
        dir.path(),
        Probe::Never,
    );
    assert!(!marker.exists(), "a re-answer never starts the helper");
    assert_eq!(
        state_of(&jobs, AiJob::Chat),
        (AiJobState::NoEngine, false, Some(AiJobWhy::Unknown)),
        "not asked yet: unknown fails closed"
    );
    let _ = service.answer(
        &Snapshot::default(),
        &beta_for(&[Job::Chat]),
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
    let engines = Registry::with(
        Apple::with_program(Some(helper(dir.path(), "available"))),
        Local::default(),
    );
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
            mac,
            &Snapshot::default(),
            &beta_for(&[Job::Chat]),
        );
        state_of(&jobs, AiJob::Chat)
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
        ("beta on", &[Job::Review, Job::Chat][..]),
    ] {
        let beta = move |job: Job| on.contains(&job);
        let (jobs, _) = service.answer(&Snapshot::default(), &beta, dir.path(), Probe::Now);
        for info in jobs {
            println!("{label}: {}", serde_json::to_string(&info).unwrap());
        }
    }
}
