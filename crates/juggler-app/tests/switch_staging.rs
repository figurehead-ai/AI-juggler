//! AJ-E1513 integration: the switch prepared before the limit. Proves:
//! crossing the warning stages the dialog's pick (the switch check and a
//! redacted pack) in a folder only this user can read, and starts
//! nothing; under the warning nothing is staged; a refresh waits five
//! minutes AND a changed transcript; the staged switch goes when the
//! session ends, when its window resets, and at startup after a crash;
//! a transcript over the cap is left to the switch itself.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "support/set_up.rs"]
mod set_up;

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::staging;
use juggler_app::state::AppState;
use juggler_core::model::{
    InstanceKind, MachineId, ProfileId, SessionId, SessionRecord, SessionStatus, Timestamp,
    TranscriptRef,
};
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;

struct Lab {
    state: Arc<AppState>,
    session: String,
    account: String,
    transcript: PathBuf,
    stub_runs: PathBuf,
}

fn app_state(dir: &Path) -> (Arc<AppState>, PathBuf) {
    let stub = dir.join("claude-stub");
    let runs = dir.join("stub-runs");
    stand_in::write(
        &stub,
        format!("#!/bin/sh\necho \"$*\" >> '{}'\n", runs.to_string_lossy()),
    );
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    (
        Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap()),
        runs,
    )
}

fn transcript_body(cwd: &str, turns: usize, words: usize) -> String {
    let filler = "lorem ipsum dolor sit amet ".repeat(words / 5 + 1);
    let mut body = String::new();
    for n in 0..turns {
        let role = if n % 2 == 0 { "user" } else { "assistant" };
        body.push_str(&format!(
            "{{\"type\":\"{role}\",\"uuid\":\"u{n}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"{role}\",\"content\":\"turn {n} {filler}\"}}}}\n"
        ));
    }
    body
}

/// A running session on "Personal" (a second account, "Work", can take
/// it), with a transcript of `turns` turns.
fn lab(work: &Path, turns: usize, words: usize) -> Lab {
    let (state, stub_runs) = app_state(work);
    commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();
    let project = work.join("storefront");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let personal = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Personal", Some("max".into())).unwrap(),
    );
    let recipe = commands::create_profile(&state, &personal.id, "Mine", None, None).unwrap();
    let work_account = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Work", Some("max".into())).unwrap(),
    );
    commands::create_profile(&state, &work_account.id, "Main", None, None).unwrap();
    // A non-empty `.credentials.json` marks "Work" signed in: a test
    // marker under the temp dir, never a real credential.
    let work_config = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(&work_account.id);
    std::fs::create_dir_all(&work_config).unwrap();
    std::fs::write(work_config.join(".credentials.json"), r#"{"stub":true}"#).unwrap();
    let transcript = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(&personal.id)
        .join("projects")
        .join(juggler_adapter::handover::mangled_dir(&cwd))
        .join("native-7.jsonl");
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    std::fs::write(&transcript, transcript_body(&cwd, turns, words)).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&SessionRecord {
            id: SessionId::new("sess-staged"),
            conversation: None,
            profile: ProfileId::new(recipe.id.clone()),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: project.clone(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: Some(TranscriptRef {
                path: transcript.clone(),
                native_id: Some("native-7".into()),
            }),
            window: None,
        })
        .unwrap();
    Lab {
        state,
        session: "sess-staged".into(),
        account: personal.id,
        transcript,
        stub_runs,
    }
}

fn reading(lab: &Lab, used_pct: f64, resets_at: i64) {
    let now = Timestamp::now().millis();
    lab.state
        .storage
        .lock()
        .unwrap()
        .usage()
        .insert_sample(
            &lab.account,
            juggler_core::quota::WindowKind::FiveHour,
            &juggler_core::quota::QuotaSample {
                ts: now - MIN,
                used_pct,
                resets_at: Some(resets_at),
                confidence: juggler_core::model::Confidence::Measured,
            },
        )
        .unwrap();
}

fn set_status(lab: &Lab, status: SessionStatus) {
    let storage = lab.state.storage.lock().unwrap();
    let mut row = storage
        .sessions()
        .get(&SessionId::new(lab.session.clone()))
        .unwrap();
    row.status = status;
    storage.sessions().update(&row).unwrap();
}

fn sessions(state: &Arc<AppState>) -> Vec<(String, String)> {
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .list(None)
        .unwrap()
        .iter()
        .map(|row| (row.id.as_str().to_owned(), row.status.as_str().to_owned()))
        .collect()
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

/// Every file and folder under `dir`, with its mode.
fn modes(dir: &Path, out: &mut Vec<(PathBuf, u32)>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        out.push((path.clone(), mode(&path)));
        if path.is_dir() {
            modes(&path, out);
        }
    }
}

fn staged_names(state: &AppState) -> Vec<String> {
    std::fs::read_dir(staging::root(state))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn crossing_the_warning_stages_the_switch_privately_and_starts_nothing() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 200, 20);
    let now = Timestamp::now().millis();
    reading(&lab, 85.0, now + 2 * HOUR);
    let before = sessions(&lab.state);

    assert_eq!(staging::tick(&lab.state, now), 1);

    let ready = staging::ready_impl(&lab.state, &lab.session, now).expect("ready");
    assert_eq!(ready.target_account_label, "Work");
    assert_eq!(ready.prepared_at_ms, now);
    assert!(!ready.fidelity_words.is_empty());

    // Only this user can read it: folders 0700, files 0600.
    let root = staging::root(&lab.state);
    assert_eq!(mode(&root), 0o700);
    let mut all = Vec::new();
    modes(&root, &mut all);
    assert!(
        all.iter()
            .any(|(path, _)| path.ends_with("pack/conversation.md")),
        "{all:?}"
    );
    for (path, bits) in &all {
        let want = if path.is_dir() { 0o700 } else { 0o600 };
        assert_eq!(*bits, want, "{}", path.display());
    }
    // It lives in the app's own folder.
    assert!(root.starts_with(&lab.state.data_dir));

    // Nothing started, nothing changed state.
    assert_eq!(sessions(&lab.state), before);
    // The tool ran only to say its version (the switch check's
    // validation verdict): never a session.
    let runs = std::fs::read_to_string(&lab.stub_runs).unwrap_or_default();
    assert!(
        runs.lines().all(|args| args == "--version"),
        "only version checks: {runs:?}"
    );

    // Fresh: the next tick does nothing.
    assert_eq!(staging::tick(&lab.state, now + 30_000), 0);
}

#[test]
fn under_the_warning_nothing_is_staged() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 50, 5);
    let now = Timestamp::now().millis();
    reading(&lab, 50.0, now + 2 * HOUR);
    assert_eq!(staging::tick(&lab.state, now), 0);
    assert!(staged_names(&lab.state).is_empty());
    assert_eq!(staging::ready_impl(&lab.state, &lab.session, now), None);
}

#[test]
fn a_refresh_waits_five_minutes_and_a_changed_transcript() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 50, 5);
    let now = Timestamp::now().millis();
    reading(&lab, 85.0, now + 3 * HOUR);
    assert_eq!(staging::tick(&lab.state, now), 1);
    // Unchanged after six minutes: nothing to redo.
    assert_eq!(staging::tick(&lab.state, now + 6 * MIN), 0);
    // The session went on.
    let more = "{{\"type\":\"user\",\"uuid\":\"later\",\"message\":{{\"role\":\"user\",\"content\":\"one more\"}}}}\n";
    let mut body = std::fs::read_to_string(&lab.transcript).unwrap();
    body.push_str(more);
    std::fs::write(&lab.transcript, body).unwrap();
    // Changed, but too soon.
    assert_eq!(staging::tick(&lab.state, now + 4 * MIN), 0);
    // Changed and due.
    assert_eq!(staging::tick(&lab.state, now + 7 * MIN), 1);
    let ready = staging::ready_impl(&lab.state, &lab.session, now + 7 * MIN).unwrap();
    assert_eq!(ready.prepared_at_ms, now + 7 * MIN);
}

#[test]
fn the_staged_switch_goes_when_the_session_ends() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 50, 5);
    let now = Timestamp::now().millis();
    reading(&lab, 85.0, now + 2 * HOUR);
    assert_eq!(staging::tick(&lab.state, now), 1);
    // Stopped by its limit: kept (that's when it's wanted).
    set_status(&lab, SessionStatus::QuotaBlocked);
    staging::tick(&lab.state, now + MIN);
    assert_eq!(staged_names(&lab.state), std::slice::from_ref(&lab.session));
    // Ended: gone, and the folder is empty.
    set_status(&lab, SessionStatus::Finished);
    assert_eq!(staging::tick(&lab.state, now + 2 * MIN), 0);
    assert!(staged_names(&lab.state).is_empty());
    assert_eq!(staging::ready_impl(&lab.state, &lab.session, now), None);
}

#[test]
fn the_staged_switch_goes_when_its_window_resets() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 50, 5);
    let now = Timestamp::now().millis();
    let resets = now + HOUR;
    reading(&lab, 85.0, resets);
    assert_eq!(staging::tick(&lab.state, now), 1);
    // Not ready once the reset has passed, even before a tick.
    assert_eq!(staging::ready_impl(&lab.state, &lab.session, resets), None);
    // Even stopped by its limit, it isn't staged again after the reset.
    set_status(&lab, SessionStatus::QuotaBlocked);
    staging::tick(&lab.state, resets + MIN);
    assert!(staged_names(&lab.state).is_empty());
    staging::tick(&lab.state, resets + 10 * MIN);
    assert!(staged_names(&lab.state).is_empty());
}

#[test]
fn startup_drops_what_a_crash_left_behind() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 50, 5);
    let now = Timestamp::now().millis();
    reading(&lab, 85.0, now + 2 * HOUR);
    assert_eq!(staging::tick(&lab.state, now), 1);
    // The app went down; the session ended meanwhile.
    set_status(&lab, SessionStatus::Stopped);
    staging::sweep_at_startup(&lab.state);
    assert!(staged_names(&lab.state).is_empty());
}

#[test]
fn a_transcript_over_the_cap_is_left_to_the_switch() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 2, 5);
    let big = vec![b'x'; usize::try_from(staging::TRANSCRIPT_MAX).unwrap() + 1];
    std::fs::write(&lab.transcript, big).unwrap();
    let now = Timestamp::now().millis();
    reading(&lab, 95.0, now + 2 * HOUR);
    assert_eq!(staging::tick(&lab.state, now), 0);
    assert!(staged_names(&lab.state).is_empty());
}

/// The cost on a large, real-shaped session: 4,000 turns of ~150 words
/// (≈ 4 MB of transcript), staged once from one read.
#[test]
fn staging_a_large_session_stays_cheap() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 4_000, 150);
    let size = std::fs::metadata(&lab.transcript).unwrap().len();
    let now = Timestamp::now().millis();
    reading(&lab, 85.0, now + 2 * HOUR);
    let started = Instant::now();
    assert_eq!(staging::tick(&lab.state, now), 1);
    let took = started.elapsed();
    eprintln!(
        "[perf] staging 4,000 turns ({:.1} MB): {took:?}",
        size as f64 / 1_048_576.0
    );
    assert!(took.as_secs() < 5, "{took:?}");
}
