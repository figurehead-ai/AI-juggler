//! The Apple engine: what each of the model's answers makes of its card,
//! the helper protocol against fake helpers, the Swift source's rules
//! (ADR-0033 §4.1), and one live check on a Mac with the helper built.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use juggler_platform::ai::SystemModel;

use super::{Apple, Availability, Seen};
use crate::features::ai::engine::{AiEngine, EngineError, Readiness, Reason, Request, Stop};
use crate::features::ai::models::builtin;
use crate::features::ai::models::recommend::AppleAi;

const SOURCE: &str = include_str!("../../../../helpers/juggler-apple-ai.swift");

fn model(availability: Availability) -> Seen {
    Seen::Model {
        availability,
        context_tokens: 8_192,
    }
}

#[test]
fn each_answer_gives_its_readiness_and_unknown_fails_closed() {
    let cases = [
        (
            model(Availability::Available),
            Readiness::Ready,
            AppleAi::On,
        ),
        (
            model(Availability::NotEnabled),
            Readiness::NotHere(Reason::TurnedOff),
            AppleAi::Off,
        ),
        (
            model(Availability::NotReady),
            Readiness::Preparing(Reason::GettingReady),
            AppleAi::Off,
        ),
        (
            model(Availability::NotEligible),
            Readiness::NotHere(Reason::NotThisComputer),
            AppleAi::Unavailable,
        ),
        (
            model(Availability::NotThisOs),
            Readiness::NotHere(Reason::NotThisSystem),
            AppleAi::Unavailable,
        ),
        (
            model(Availability::Unknown),
            Readiness::NotHere(Reason::Unknown),
            AppleAi::Unavailable,
        ),
        (
            Seen::Os(SystemModel::NotThisChip),
            Readiness::NotHere(Reason::NotThisComputer),
            AppleAi::Unavailable,
        ),
        (
            Seen::Os(SystemModel::NotThisVersion),
            Readiness::NotHere(Reason::NotThisComputer),
            AppleAi::Unavailable,
        ),
        (
            Seen::Os(SystemModel::NotThisSystem),
            Readiness::NotHere(Reason::NotThisSystem),
            AppleAi::Unavailable,
        ),
        (
            Seen::Missing,
            Readiness::NotHere(Reason::HelperMissing),
            AppleAi::Unavailable,
        ),
        (
            Seen::NotAsked,
            Readiness::NotHere(Reason::Unknown),
            AppleAi::Unavailable,
        ),
        (
            Seen::Failed,
            Readiness::NotHere(Reason::Unknown),
            AppleAi::Unavailable,
        ),
    ];
    for (seen, ready, apple_ai) in cases {
        assert_eq!(seen.readiness(), ready, "{seen:?}");
        assert_eq!(seen.apple_ai(), apple_ai, "{seen:?}");
    }
}

#[test]
fn it_serves_only_the_built_in_entry_and_nothing_without_it() {
    let engine = Apple::with_program(None);
    assert!(engine.cards(0, &[]).is_empty(), "no entry, no card");
    let cards = engine.cards(0, &builtin());
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert!(!card.uses_download);
    assert!(card.tools, "look-ups run as guided steps");
    assert_eq!(
        card.ready,
        Readiness::NotHere(Reason::Unknown),
        "not asked yet"
    );
}

/// A fake helper: says `hello`, then answers every request with `reply`
/// (`ID` replaced by the request's id), after `pause` seconds.
#[cfg(unix)]
fn fake(dir: &Path, name: &str, hello: &str, reply: &str, pause: u32) -> PathBuf {
    let path = dir.join(name);
    let script = format!(
        "#!/bin/sh\necho '{hello}'\nwhile read line; do\n  case \"$line\" in *'\"op\":\"quit\"'*) exit 0;; esac\n  \
         sleep {pause}\n  id=$(echo \"$line\" | sed 's/.*\"id\":\\([0-9]*\\).*/\\1/')\n  echo '{reply}' | sed \"s/ID/$id/\"\ndone\n"
    );
    crate::test_support::script(&path, &script)
}

fn hello(available: &str) -> String {
    format!(r#"{{"hello":true,"protocol":2,"available":"{available}","context_size":8192}}"#)
}

const OK: &str = r#"{"id":ID,"text":"fine","finish":"stop","error":null}"#;

fn request() -> Request {
    Request {
        instructions: "i".to_owned(),
        prompt: "p".to_owned(),
        shape: None,
        max_tokens: 20,
    }
}

fn soon() -> Stop {
    Stop::at(Instant::now() + Duration::from_secs(10))
}

#[cfg(unix)]
#[test]
fn a_probe_reads_the_hello_and_a_job_gets_its_answer() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Apple::with_program(Some(fake(dir.path(), "on", &hello("available"), OK, 0)));
    engine.probe();
    assert_eq!(engine.seen(), model(Availability::Available));
    let card = &engine.cards(0, &builtin())[0];
    assert_eq!((card.ready, card.context_tokens), (Readiness::Ready, 8_192));
    assert!(!card.running, "a probe lets its helper go");
    let draft = engine.respond(&card.model, &request(), &soon()).unwrap();
    assert_eq!(draft.text, "fine");
    assert!(
        engine.cards(0, &builtin())[0].running,
        "it stays for the next job"
    );
    let again = engine.respond(&card.model, &request(), &soon()).unwrap();
    assert_eq!(
        again.text, "fine",
        "the same helper answers the next request"
    );
    engine.release();
    assert!(!engine.cards(0, &builtin())[0].running);
}

#[cfg(unix)]
#[test]
fn turned_off_is_not_here_and_never_asked_to_write() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Apple::with_program(Some(fake(
        dir.path(),
        "off",
        &hello("apple_intelligence_not_enabled"),
        OK,
        0,
    )));
    engine.probe();
    let card = &engine.cards(0, &builtin())[0];
    assert_eq!(card.ready, Readiness::NotHere(Reason::TurnedOff));
    assert_eq!(
        engine.respond(&card.model, &request(), &soon()),
        Err(EngineError::NotHere(Reason::TurnedOff))
    );
}

#[cfg(unix)]
#[test]
fn turned_off_mid_session_is_seen_when_the_helper_starts_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake(dir.path(), "helper", &hello("available"), OK, 0);
    let engine = Apple::with_program(Some(path.clone()));
    engine.probe();
    assert_eq!(engine.cards(0, &builtin())[0].ready, Readiness::Ready);
    // The user turns Apple Intelligence off; the next probe sees it.
    fake(
        dir.path(),
        "helper",
        &hello("apple_intelligence_not_enabled"),
        OK,
        0,
    );
    engine.probe();
    assert_eq!(
        engine.cards(0, &builtin())[0].ready,
        Readiness::NotHere(Reason::TurnedOff)
    );
}

#[cfg(unix)]
#[test]
fn a_slow_answer_is_stopped_at_the_deadline_and_a_stop_ends_it_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Apple::with_program(Some(fake(dir.path(), "slow", &hello("available"), OK, 5)));
    engine.probe();
    let started = Instant::now();
    let result = engine.respond(
        "m",
        &request(),
        &Stop::at(Instant::now() + Duration::from_millis(300)),
    );
    assert_eq!(result, Err(EngineError::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(
        !engine.cards(0, &builtin())[0].running,
        "a timed-out helper is stopped"
    );
    let stop = soon();
    stop.stop();
    assert_eq!(
        engine.respond("m", &request(), &stop),
        Err(EngineError::Stopped)
    );
}

#[cfg(unix)]
#[test]
fn three_crashes_in_an_hour_make_it_unhealthy() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Apple::with_program(Some(fake(
        dir.path(),
        "bad",
        &hello("available"),
        "not json",
        0,
    )));
    engine.probe();
    for _ in 0..3 {
        assert_eq!(
            engine.respond("m", &request(), &soon()),
            Err(EngineError::Crashed)
        );
    }
    assert_eq!(
        engine.cards(0, &builtin())[0].ready,
        Readiness::NotHere(Reason::Unhealthy)
    );
    assert_eq!(
        engine.respond("m", &request(), &soon()),
        Err(EngineError::NotHere(Reason::Unhealthy))
    );
}

#[cfg(unix)]
#[test]
fn a_refusal_and_a_cut_answer_reach_the_job_as_such() {
    let dir = tempfile::tempdir().unwrap();
    let refused = r#"{"id":ID,"text":"","finish":"stop","error":"refused"}"#;
    let engine = Apple::with_program(Some(fake(
        dir.path(),
        "no",
        &hello("available"),
        refused,
        0,
    )));
    engine.probe();
    assert_eq!(
        engine.respond("m", &request(), &soon()),
        Err(EngineError::Refused)
    );
    let cut = r#"{"id":ID,"text":"half","finish":"length","error":null}"#;
    let engine = Apple::with_program(Some(fake(dir.path(), "cut", &hello("available"), cut, 0)));
    engine.probe();
    let draft = engine.respond("m", &request(), &soon()).unwrap();
    assert_eq!(draft.finish, crate::features::ai::engine::Finish::Length);
}

#[test]
fn a_missing_helper_says_so() {
    let engine = Apple::with_program(Some(PathBuf::from("/nonexistent/juggler-apple-ai")));
    engine.probe();
    assert_eq!(engine.seen(), Seen::Missing);
    assert_eq!(
        engine.cards(0, &builtin())[0].ready,
        Readiness::NotHere(Reason::HelperMissing)
    );
}

/// ADR-0033 §4.1: the helper may import only Foundation,
/// FoundationModels and Network (for `NWPathMonitor` alone: AJ-E3813's
/// pause on a metered link), uses the on-device model only, and opens no
/// connection, file or process.
#[test]
fn the_apple_helper_never_names_private_cloud_compute() {
    let imports: Vec<&str> = SOURCE
        .lines()
        .filter_map(|line| line.trim().strip_prefix("import "))
        .collect();
    assert_eq!(imports, ["Foundation", "FoundationModels", "Network"]);
    let code: String = SOURCE
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for banned in [
        "PrivateCloudCompute",
        "URLSession",
        "NWConnection",
        "NWListener",
        "NWBrowser",
        "NWEndpoint",
        "NWParameters",
        "NWProtocol",
        "FileManager",
        "FileHandle(forWriting",
        "Process(",
        "contentsOf:",
    ] {
        assert!(!code.contains(banned), "the helper names {banned}");
    }
    assert!(code.contains("SystemLanguageModel.default"));
    assert!(code.contains("temperature: 0"));
}

#[cfg(unix)]
#[test]
fn the_network_path_comes_from_the_helper_and_unknown_is_none() {
    use super::{NetworkPath, network_path};
    let dir = tempfile::tempdir().unwrap();
    let metered = r#"{"id":ID,"satisfied":true,"expensive":true,"constrained":false,"error":null}"#;
    let path = fake(dir.path(), "net", &hello("not_this_os"), metered, 0);
    assert_eq!(
        network_path(Some(path)),
        Some(NetworkPath {
            satisfied: true,
            expensive: true,
            constrained: false,
        })
    );
    let cannot =
        r#"{"id":ID,"satisfied":false,"expensive":false,"constrained":false,"error":"unknown"}"#;
    let path = fake(dir.path(), "cannot", &hello("available"), cannot, 0);
    assert_eq!(network_path(Some(path)), None);
    let old = r#"{"hello":true,"protocol":1,"available":"available","context_size":8192}"#;
    let path = fake(dir.path(), "old", old, metered, 0);
    assert_eq!(network_path(Some(path)), None, "an older helper can't tell");
    assert_eq!(
        network_path(Some(PathBuf::from("/nonexistent/juggler-apple-ai"))),
        None
    );
    // The helper reads the path only; the source opens no connection.
    assert!(SOURCE.contains("NWPathMonitor()"));
}

/// The real helper on this Mac (run by hand after
/// `scripts/build_apple_ai_helper.sh`): whatever Apple Intelligence says,
/// the engine reads it, and when it's on, a short answer comes back.
#[test]
#[ignore = "needs the built helper and the system model; run by hand"]
fn the_real_helper_answers_on_this_mac() {
    let exe = std::env::current_exe().unwrap();
    let program = exe.parent().unwrap().parent().unwrap().join(super::PROGRAM);
    let engine = Apple::with_program(Some(program));
    engine.probe();
    let card = engine.cards(0, &builtin()).remove(0);
    println!("seen: {:?}; card: {card:?}", engine.seen());
    if card.ready == Readiness::Ready {
        let draft = engine
            .respond(
                &card.model,
                &Request {
                    instructions: "Answer in one short sentence.".to_owned(),
                    prompt: "Say hello.".to_owned(),
                    shape: None,
                    max_tokens: 20,
                },
                &Stop::at(Instant::now() + Duration::from_secs(30)),
            )
            .unwrap();
        println!("draft: {draft:?}");
        assert!(!draft.text.is_empty());
    }
    engine.release();
}
