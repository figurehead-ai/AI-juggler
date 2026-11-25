//! AJ-E3825: Bot's engine really started, over fake Apple helpers: a
//! lazy start with its hello checked, the idle stop leaving no process
//! behind, the next lazy start, every failure with its kind (and logged
//! by it), and today's counts.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test: a failed step fails the test"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{
    AiCount, AiEngineState, AiFailure, generate_on, note_usage, start_on, status_on, today,
};
use crate::features::ai::dto::AiJob;
use crate::features::ai::engine::{EngineError, Reason, Request, Stop};
use crate::features::ai::engines::Registry;
use crate::features::ai::engines::apple::Apple;
use crate::features::ai::engines::local::Local;
use crate::features::ai::models::{Engine as ModelEngine, Job};
use crate::features::ai::service::Service;
use crate::state::AppState;

/// A fake Apple helper: says hello with `available`, then answers each
/// request with `reply` (`ID`: the request's id) after `pause` seconds.
#[cfg(unix)]
pub(crate) fn fake_helper(
    dir: &Path,
    name: &str,
    available: &str,
    reply: &str,
    pause: u32,
) -> PathBuf {
    let hello =
        format!(r#"{{"hello":true,"protocol":2,"available":"{available}","context_size":4096}}"#);
    let script = format!(
        "#!/bin/sh\necho '{hello}'\nwhile read line; do\n  sleep {pause}\n  \
         id=$(echo \"$line\" | sed 's/.*\"id\":\\([0-9]*\\).*/\\1/')\n  echo '{reply}' | sed \"s/ID/$id/\"\ndone\n"
    );
    crate::test_support::script(&dir.join(name), &script)
}

const OK: &str =
    r#"{"id":ID,"text":"fine","finish":"stop","error":null,"tokens_in":120,"tokens_out":7}"#;

fn bot_on(dir: &Path) -> Arc<AppState> {
    let state = crate::test_support::bare_state(dir.join("data"));
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(&crate::features::ai::jobs::beta_key(Job::Bot), "1")
        .unwrap();
    state
}

fn engine_service(program: PathBuf, idle: Duration) -> Service {
    Service::new(Registry::with(
        Apple::with_idle(Some(program), idle),
        Local::default(),
    ))
}

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
fn process_alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn logged(state: &AppState, what: &str) -> Vec<serde_json::Value> {
    let storage = state.storage.lock().unwrap();
    storage
        .ai_log()
        .read(Some(juggler_core::storage::memory::Stream::Chat), 0, 1_000)
        .unwrap()
        .into_iter()
        .filter(|event| event.verb == what)
        .map(|event| event.payload)
        .collect()
}

/// R4 and the coordinator's R2/R3: nothing runs until Bot needs it; the
/// start checks the hello and keeps the helper; after its idle time the
/// helper is stopped and reaped (no process left); the next question
/// starts it again, lazily.
#[cfg(unix)]
#[test]
fn the_engine_starts_lazily_stops_when_idle_and_starts_again() {
    let dir = tempfile::tempdir().unwrap();
    let state = bot_on(dir.path());
    let program = fake_helper(dir.path(), "helper", "available", OK, 0);
    let idle = Duration::from_millis(400);
    let service = engine_service(program, idle);

    let before = status_on(&service, &state, AiJob::Bot);
    assert_eq!(before.state, AiEngineState::Idle, "{before:?}");
    assert_eq!(
        before.model_name.as_deref(),
        Some("Apple's on-device model")
    );
    assert_eq!(before.engine.as_deref(), Some("apple_builtin"));

    let started = start_on(&service, &state, AiJob::Bot);
    assert_eq!(started.state, AiEngineState::Running, "{started:?}");
    assert_eq!(
        started.context_tokens,
        Some(4096),
        "the helper's own figure"
    );
    assert!(started.started_ms.is_some());
    assert_eq!(logged(&state, "engine_started").len(), 1);

    // The idle stop: the helper goes, and nothing is left behind.
    let apple = apple_of(&service);
    let pid = apple.helper_pid().expect("the started helper");
    assert!(process_alive(pid));
    let until = Instant::now() + Duration::from_secs(5);
    while apple.helper_pid().is_some() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(apple.helper_pid(), None, "let go after its idle time");
    assert!(!process_alive(pid), "stopped and reaped, not orphaned");
    assert_eq!(
        status_on(&service, &state, AiJob::Bot).state,
        AiEngineState::Idle
    );

    // The next question starts it again, lazily, and is answered.
    let answer = generate_on(&service, &state, AiJob::Bot, &request(), &soon()).unwrap();
    assert_eq!(answer.draft.text, "fine");
    assert_eq!(
        answer.draft.usage.input,
        Some(120),
        "the engine's own count"
    );
    assert_eq!(answer.model_name, "Apple's on-device model");
    assert_eq!(logged(&state, "engine_started").len(), 2);
    let after = status_on(&service, &state, AiJob::Bot);
    assert_eq!(after.state, AiEngineState::Running);
    assert!(after.answered_ms.is_some());
    service.engines().release();
}

fn apple_of(service: &Service) -> &Apple {
    service.engines().apple()
}

/// R6: each failure is its kind, kept in the status and logged by it;
/// never a silent fallback.
#[cfg(unix)]
#[test]
fn every_failure_is_its_kind_and_logged() {
    let dir = tempfile::tempdir().unwrap();
    let state = bot_on(dir.path());
    let cases: [(&str, &str, AiFailure); 2] = [
        ("off", "apple_intelligence_not_enabled", AiFailure::Off),
        ("getting", "model_not_ready", AiFailure::NotDownloaded),
    ];
    for (name, available, kind) in cases {
        let service = engine_service(
            fake_helper(dir.path(), name, available, OK, 0),
            Duration::from_secs(60),
        );
        let status = start_on(&service, &state, AiJob::Bot);
        assert_eq!(
            (status.state, status.failure),
            (AiEngineState::Failed, Some(kind)),
            "{name}"
        );
        assert!(
            generate_on(&service, &state, AiJob::Bot, &request(), &soon()).is_err(),
            "{name}: nothing answers"
        );
    }
    // The helper missing from the build.
    let service = engine_service(dir.path().join("no-such-helper"), Duration::from_secs(60));
    let status = start_on(&service, &state, AiJob::Bot);
    assert_eq!(status.failure, Some(AiFailure::HelperDidntStart));

    // Refused and timed out, at the answer.
    let refused = r#"{"id":ID,"text":"","finish":"stop","error":"refused"}"#;
    let service_r = engine_service(
        fake_helper(dir.path(), "refuses", "available", refused, 0),
        Duration::from_secs(60),
    );
    assert_eq!(
        generate_on(&service_r, &state, AiJob::Bot, &request(), &soon()).err(),
        Some(AiFailure::Refused)
    );
    assert_eq!(
        status_on(&service_r, &state, AiJob::Bot).failure,
        Some(AiFailure::Refused)
    );
    service_r.engines().release();
    let service_t = engine_service(
        fake_helper(dir.path(), "slow", "available", OK, 5),
        Duration::from_secs(60),
    );
    let short = Stop::at(Instant::now() + Duration::from_millis(500));
    assert_eq!(
        generate_on(&service_t, &state, AiJob::Bot, &request(), &short).err(),
        Some(AiFailure::TimedOut)
    );
    service_t.engines().release();

    let kinds: Vec<String> = logged(&state, "engine_failed")
        .iter()
        .map(|payload| payload["kind"].as_str().unwrap().to_owned())
        .collect();
    for kind in [
        "off",
        "not_downloaded",
        "helper_didnt_start",
        "refused",
        "timed_out",
    ] {
        assert!(kinds.iter().any(|k| k == kind), "{kind} logged: {kinds:?}");
    }
}

/// No AI chosen (Bot's Beta off): the engine is Off, never started.
#[cfg(unix)]
#[test]
fn with_no_ai_chosen_nothing_starts() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(dir.path().join("data"));
    let service = engine_service(
        fake_helper(dir.path(), "helper", "available", OK, 0),
        Duration::from_secs(60),
    );
    let status = start_on(&service, &state, AiJob::Bot);
    assert_eq!((status.state, status.failure), (AiEngineState::Off, None));
    assert_eq!(apple_of(&service).helper_pid(), None);
}

#[test]
fn engine_errors_map_to_their_kinds() {
    let cases = [
        (EngineError::NotHere(Reason::TurnedOff), AiFailure::Off),
        (
            EngineError::NotHere(Reason::GettingReady),
            AiFailure::NotDownloaded,
        ),
        (
            EngineError::NotHere(Reason::HelperMissing),
            AiFailure::HelperDidntStart,
        ),
        (EngineError::Crashed, AiFailure::HelperDidntStart),
        (
            EngineError::NotHere(Reason::Unhealthy),
            AiFailure::HelperDidntStart,
        ),
        (EngineError::TimedOut, AiFailure::TimedOut),
        (EngineError::Refused, AiFailure::Refused),
        (EngineError::TooLong, AiFailure::TooLong),
        (
            EngineError::NotHere(Reason::NotThisComputer),
            AiFailure::NotThisMac,
        ),
    ];
    for (error, kind) in cases {
        assert_eq!(
            AiFailure::of(error, Some(ModelEngine::AppleBuiltin)),
            kind,
            "{error:?}"
        );
    }
    assert_eq!(
        AiFailure::of(EngineError::Unsupported, Some(ModelEngine::GgufLocal)),
        AiFailure::NotDownloaded,
        "a downloaded model that's gone"
    );
}

/// R8: today's counts add up, an estimate stays labelled, and a new day
/// starts at zero.
#[test]
fn todays_counts_add_up_and_say_when_estimated() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(dir.path().join("data"));
    assert_eq!(today(&state).answers, 0);
    note_usage(
        &state,
        AiCount {
            n: 100,
            estimated: false,
        },
        AiCount {
            n: 10,
            estimated: false,
        },
    );
    note_usage(
        &state,
        AiCount {
            n: 50,
            estimated: true,
        },
        AiCount {
            n: 5,
            estimated: false,
        },
    );
    let now = today(&state);
    assert_eq!(now.answers, 2);
    assert_eq!(
        now.tokens_in,
        AiCount {
            n: 150,
            estimated: true
        }
    );
    assert_eq!(
        now.tokens_out,
        AiCount {
            n: 15,
            estimated: false
        }
    );
    assert_eq!(
        AiCount::counted_or_estimated(None, "seven c"),
        AiCount {
            n: 2,
            estimated: true
        }
    );
}
