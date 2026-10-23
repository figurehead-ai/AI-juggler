//! AJ-E1538 evidence: searching the words of sessions started outside AI
//! Juggler, opt-in. Off by default (no file, no hits); the size shown
//! before turning it on matches the built index within 10 %; the index
//! is built incrementally from the tools' own files; a session AI
//! Juggler owns is left to its own index; a transcript the tool deletes
//! leaves the index; turning it off leaves nothing behind.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::outside_search;
use juggler_app::history;
use juggler_app::state::AppState;

/// A Claude Code transcript `name` for `cwd` with these user messages.
fn transcript(config: &Path, cwd: &Path, name: &str, said: &[&str]) -> PathBuf {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.jsonl"));
    let mut text = String::new();
    for words in said {
        text.push_str(&format!(
            "{{\"type\":\"user\",\"cwd\":\"{}\",\"message\":{{\"role\":\"user\",\"content\":\"{words}\"}}}}\n",
            cwd.display()
        ));
    }
    std::fs::write(&path, text).unwrap();
    path
}

struct Lab {
    state: Arc<AppState>,
    data: PathBuf,
    config: PathBuf,
    cwd: PathBuf,
    _dir: tempfile::TempDir,
}

fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let cwd = root.join("Dev/shop");
    std::fs::create_dir_all(cwd.join(".git")).unwrap();
    let config = root.join("claude");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    // A realistic spread: many short conversations and a few long ones,
    // in words drawn the way prose draws them (a few common, most rare):
    // an index of one sentence said over and over packs far tighter than
    // any part of it, which no sampled estimate could follow.
    let mut seed = 1538_u64;
    let mut word = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let u = (seed >> 33) % 4_000;
        format!("w{}", u * u / 4_000)
    };
    for n in 0..40 {
        let said: Vec<String> = (0..(5 + n * 7))
            .map(|_| (0..120).map(|_| word()).collect::<Vec<_>>().join(" "))
            .collect();
        let said: Vec<&str> = said.iter().map(String::as_str).collect();
        transcript(&config, &cwd, &format!("conv-{n}"), &said);
    }
    transcript(
        &config,
        &cwd,
        "conv-webhook",
        &["Fix the stripe webhook retry", "and log it"],
    );
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let data = root.join("app-data");
    let state = Arc::new(AppState::init_with_registry(data.clone(), registry).unwrap());
    commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap();
    juggler_app::features::conversations::tick(&state).unwrap();
    Lab {
        state,
        data,
        config,
        cwd,
        _dir: dir,
    }
}

/// Passes until the index has caught up.
fn build(state: &Arc<AppState>) {
    for _ in 0..200 {
        if !outside_search::pass(state, Duration::from_secs(5)).unwrap() {
            return;
        }
    }
    panic!("the index never caught up");
}

fn index_files(data: &Path) -> Vec<String> {
    std::fs::read_dir(data)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("outside-search"))
        .collect()
}

fn outside_hits(state: &Arc<AppState>, query: &str) -> Vec<history::TranscriptHitInfo> {
    history::transcript_search_impl(state, query)
        .unwrap()
        .into_iter()
        .filter(|hit| hit.conversation_id.is_some())
        .collect()
}

#[test]
fn off_by_default_nothing_is_indexed_or_found() {
    let lab = lab();
    let status = outside_search::status_impl(&lab.state).unwrap();
    assert!(!status.enabled);
    assert_eq!(status.size_bytes, None);
    assert_eq!(status.sessions, 41);
    assert!(!outside_search::pass(&lab.state, Duration::from_secs(5)).unwrap());
    assert!(outside_hits(&lab.state, "webhook").is_empty());
    assert!(index_files(&lab.data).is_empty(), "no file while off");
}

#[test]
fn the_size_shown_first_is_the_built_size_within_ten_percent() {
    let lab = lab();
    let estimate = outside_search::estimate_impl(&lab.state).unwrap();
    assert_eq!(estimate.sessions, 41);
    outside_search::set_impl(&lab.state, true).unwrap();
    build(&lab.state);
    let status = outside_search::status_impl(&lab.state).unwrap();
    assert_eq!((status.indexed, status.sessions), (41, 41));
    let built = status.size_bytes.unwrap();
    let off = (estimate.size_bytes as f64 - built as f64).abs() / built as f64;
    assert!(
        off <= 0.10,
        "estimate {} vs built {built}: {off:.3}",
        estimate.size_bytes
    );
}

/// The estimate on a big setup indexes a spread of the files, not all of
/// them: driven here with a sample of a quarter of the bytes (on a real
/// setup 32 MB is hundreds of files; here a quarter is ten).
#[test]
fn a_sampled_estimate_is_within_ten_percent_too() {
    let lab = lab();
    let total: u64 = std::fs::read_dir(
        lab.config
            .join("projects")
            .join(lab.cwd.to_string_lossy().replace('/', "-")),
    )
    .unwrap()
    .flatten()
    .map(|entry| entry.metadata().unwrap().len())
    .sum();
    let estimate = outside_search::estimate_with(&lab.state, total / 4).unwrap();
    outside_search::set_impl(&lab.state, true).unwrap();
    build(&lab.state);
    let built = outside_search::status_impl(&lab.state)
        .unwrap()
        .size_bytes
        .unwrap();
    let off = (estimate.size_bytes as f64 - built as f64).abs() / built as f64;
    println!(
        "[estimate] sampled {} of {total} bytes: estimate {} vs built {built} ({:.1} %)",
        total / 4,
        estimate.size_bytes,
        off * 100.0
    );
    assert!(
        off <= 0.10,
        "estimate {} vs built {built}: {off:.3}",
        estimate.size_bytes
    );
}

#[test]
fn it_finds_outside_words_in_context_and_follows_the_files() {
    let lab = lab();
    outside_search::set_impl(&lab.state, true).unwrap();
    build(&lab.state);
    let hits = outside_hits(&lab.state, "stripe webhook");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].conversation_id.as_deref(), Some("conv-webhook"));
    assert!(hits[0].account_id.is_some());
    assert_eq!(hits[0].snippet, "Fix the stripe webhook retry");

    // New words are indexed from where the last pass stopped.
    let path = lab
        .config
        .join("projects")
        .join(lab.cwd.to_string_lossy().replace('/', "-"))
        .join("conv-webhook.jsonl");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(&format!(
        "{{\"type\":\"user\",\"cwd\":\"{}\",\"message\":{{\"role\":\"user\",\"content\":\"now the refund path\"}}}}\n",
        lab.cwd.display()
    ));
    std::fs::write(&path, text).unwrap();
    juggler_app::features::conversations::tick(&lab.state).unwrap();
    build(&lab.state);
    assert_eq!(outside_hits(&lab.state, "refund").len(), 1);

    // The tool deletes it: its words leave the index.
    std::fs::remove_file(&path).unwrap();
    juggler_app::features::conversations::tick(&lab.state).unwrap();
    build(&lab.state);
    assert!(outside_hits(&lab.state, "webhook").is_empty());
}

#[test]
fn a_session_ai_juggler_owns_is_left_to_its_own_index() {
    let lab = lab();
    let account = lab.state.storage.lock().unwrap().accounts().list().unwrap()[0].clone();
    let profile =
        commands::create_profile(&lab.state, account.id.as_str(), "Main", None, None).unwrap();
    {
        use juggler_core::model::*;
        let storage = lab.state.storage.lock().unwrap();
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: SessionId::new("s-own"),
                conversation: Some(ConversationId::new("conv-webhook")),
                profile: ProfileId::new(profile.id.as_str()),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: lab.cwd.clone(),
                status: SessionStatus::Finished,
                started_at: Timestamp::now(),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
    }
    outside_search::set_impl(&lab.state, true).unwrap();
    build(&lab.state);
    assert_eq!(
        outside_search::status_impl(&lab.state).unwrap().sessions,
        40
    );
    assert!(outside_hits(&lab.state, "webhook").is_empty());
}

#[test]
fn turning_it_off_leaves_no_index_behind() {
    let lab = lab();
    outside_search::set_impl(&lab.state, true).unwrap();
    build(&lab.state);
    assert!(!index_files(&lab.data).is_empty());
    let off = outside_search::set_impl(&lab.state, false).unwrap();
    assert!(!off.enabled);
    assert!(
        index_files(&lab.data).is_empty(),
        "{:?}",
        index_files(&lab.data)
    );
    assert!(outside_hits(&lab.state, "webhook").is_empty());
    // A pass after it was turned off never makes the file again.
    assert!(!outside_search::pass(&lab.state, Duration::from_secs(5)).unwrap());
    assert!(index_files(&lab.data).is_empty());
}

/// E1529's first pass is as fast with this on: the builder does nothing
/// until the conversations index has read the tools' files (and skips
/// any round while a pass is reading).
#[test]
fn nothing_is_built_before_the_first_conversations_pass() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let cwd = root.join("Dev/shop");
    std::fs::create_dir_all(cwd.join(".git")).unwrap();
    let config = root.join("claude");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &cwd, "conv-1", &["Fix the stripe webhook retry"]);
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap();
    outside_search::set_impl(&state, true).unwrap();
    // The conversations index hasn't read anything yet: no work done.
    assert!(!outside_search::pass(&state, Duration::from_secs(5)).unwrap());
    assert_eq!(outside_search::status_impl(&state).unwrap().indexed, 0);
    juggler_app::features::conversations::tick(&state).unwrap();
    build(&state);
    assert_eq!(outside_search::status_impl(&state).unwrap().indexed, 1);
}
