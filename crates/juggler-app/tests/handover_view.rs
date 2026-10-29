//! 0.3: after a switch to another tool, what it was told. The handover's
//! `state.json` records what the keys check hid and what it left as it was
//! (kinds and counts, never a value); the done screen and the switch's edge
//! in the Switch Graph say it in one line ("Hidden from Codex: 1 API key")
//! and open the handover read-only. "Save handover…" on a session's page
//! saves one (this user's only) and says the same; Reveal in Finder shows
//! only AI Juggler's own handovers. Temp folders and stub tools only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::features::handover::view;
use juggler_app::state::AppState;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// A stub that records its argv under its own home.
fn stub(dir: &Path, name: &str, home_var: &str) -> PathBuf {
    let path = dir.join(name);
    stand_in::write(
        &path,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"${home_var}/last-args.tmp\"\nmv \"${home_var}/last-args.tmp\" \"${home_var}/last-args\"\n"
        ),
    );
    path
}

fn wait_for_args(home: &Path) {
    let file = home.join("last-args");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !file.is_file() {
        assert!(std::time::Instant::now() < deadline, "the stub never ran");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    std::fs::remove_file(&file).unwrap();
}

const KEY: &str = "sk-proj-abcdefghijklmnopqrstuvwx0123456789";

struct Lab {
    state: Arc<AppState>,
    session: String,
    codex_profile: String,
    project: PathBuf,
    _bins: tempfile::TempDir,
    _work: tempfile::TempDir,
}

fn lab(policy: &str) -> Lab {
    let bins = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let work = tempfile::tempdir().unwrap();
    let dir = work.path();
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(stub(
        bins.path(),
        "claude-stub",
        "CLAUDE_CONFIG_DIR",
    )));
    let codex = Arc::new(CodexAdapter::with_bin(stub(
        bins.path(),
        "codex-stub",
        "CODEX_HOME",
    )));
    registry.register(claude.clone());
    registry.register_handover(claude);
    registry.register(codex.clone());
    registry.register_handover(codex);
    let state = Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap());
    commands::redaction_policy_set_global_impl(&state, Some(policy)).unwrap();
    let project = dir.join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let b = commands::create_account(&state, CODEX_ID, "Codex", None).unwrap();
    let profile_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let a_home = state.accounts_dir().join(CLAUDE_CODE_ID).join(&a.id);
    std::fs::create_dir_all(state.accounts_dir().join(CODEX_ID).join(&b.id)).unwrap();

    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();
    wait_for_args(&a_home);
    // The conversation the switch replays: a key in the first request.
    let dir = a_home.join("projects").join(cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let transcript = dir.join("claude-sess-1.jsonl");
    std::fs::write(
        &transcript,
        format!(
            "{{\"type\":\"user\",\"uuid\":\"u1\",\"cwd\":\"{cwd}\",\"sessionId\":\"claude-sess-1\",\"message\":{{\"role\":\"user\",\"content\":\"Deploy with OPENAI_KEY {KEY} please\"}}}}\n{{\"type\":\"assistant\",\"uuid\":\"u2\",\"parentUuid\":\"u1\",\"cwd\":\"{cwd}\",\"sessionId\":\"claude-sess-1\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"Deployed.\"}}]}}}}\n"
        ),
    )
    .unwrap();
    {
        let storage = state.storage.lock().unwrap();
        let id = juggler_core::model::SessionId::new(launched.session_id.clone());
        let mut record = storage.sessions().get(&id).unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: transcript,
            native_id: Some("claude-sess-1".to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }
    Lab {
        state,
        session: launched.session_id,
        codex_profile: profile_b.id,
        project,
        _bins: bins,
        _work: work,
    }
}

fn switch(lab: &Lab, choice: Option<&str>) -> String {
    commands::switch_execute_with(
        &lab.state,
        &lab.session,
        &lab.codex_profile,
        "finish-turn",
        choice,
        &[],
        &[],
    )
    .unwrap()
    .edge_id
}

fn state_json(lab: &Lab, edge: &str) -> String {
    std::fs::read_to_string(
        lab.state
            .data_dir
            .join("switches")
            .join(edge)
            .join("state.json"),
    )
    .unwrap()
}

#[test]
fn a_switch_to_codex_records_and_says_what_was_hidden() {
    let lab = lab("strip");
    let edge = switch(&lab, None);

    let saved = state_json(&lab, &edge);
    assert!(!saved.contains(KEY), "no value is ever stored");
    let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        saved["withheld"],
        serde_json::json!([{"kind": "sk-key", "count": 1}])
    );
    assert!(saved.get("kept").is_none(), "nothing was kept");

    let summary = view::summary_impl(&lab.state, &edge)
        .unwrap()
        .expect("a switch to another tool has a handover");
    assert_eq!(summary.to_tool, "Codex");
    assert_eq!(
        summary.line.as_deref(),
        Some("Hidden from Codex: 1 API key")
    );

    let sheet = view::open_impl(&lab.state, &edge).unwrap();
    assert_eq!(sheet.from_tool, "Claude Code");
    assert_eq!(sheet.from_account, "Work Max");
    assert_eq!(sheet.to_account, "Codex");
    assert!(!sheet.brief.contains(KEY), "the brief never holds the key");
    assert!(sheet.brief.contains("Deploy with OPENAI_KEY"));
    assert_eq!(sheet.carries[0], "the conversation (2 turns)");
    assert!(sheet.not_carried.is_empty());
    let withheld = sheet.withheld.expect("recorded");
    assert_eq!(
        (withheld[0].count, withheld[0].what.as_str()),
        (1, "API key")
    );
    assert!(sheet.kept.is_empty());

    // The Switch Graph's edge says the same.
    let project_id = {
        let storage = lab.state.storage.lock().unwrap();
        storage
            .projects()
            .by_root(&std::fs::canonicalize(&lab.project).unwrap())
            .unwrap()
            .expect("project recorded")
            .id
            .to_string()
    };
    let works = commands::switch_graph_overview_impl(&lab.state, &project_id).unwrap();
    let shown = works[0]
        .edges
        .iter()
        .find(|row| row.id == edge)
        .expect("the switch's edge");
    assert_eq!(shown.handover.as_ref(), Some(&summary));
}

#[test]
fn keys_sent_as_the_user_chose_are_never_called_hidden() {
    let lab = lab("strip");
    let edge = switch(&lab, Some("keep"));
    let saved: serde_json::Value = serde_json::from_str(&state_json(&lab, &edge)).unwrap();
    assert_eq!(saved["withheld"], serde_json::json!([]));
    assert_eq!(
        saved["kept"],
        serde_json::json!([{"kind": "sk-key", "count": 1}])
    );
    let summary = view::summary_impl(&lab.state, &edge).unwrap().unwrap();
    assert_eq!(
        summary.line.as_deref(),
        Some("Sent to Codex as you chose: 1 API key")
    );
}

#[test]
fn only_this_macs_switches_open_and_nothing_outside_is_revealed() {
    let lab = lab("strip");
    for edge in ["edge-nope", "../edge-nope", "", "a/b"] {
        assert_eq!(
            view::summary_impl(&lab.state, edge).unwrap(),
            None,
            "no handover for a made-up edge"
        );
        assert!(view::open_impl(&lab.state, edge).is_err());
    }
    // An edge the graph knows, but whose folder isn't there, has none.
    let edge = switch(&lab, None);
    std::fs::remove_dir_all(lab.state.data_dir.join("switches").join(&edge)).unwrap();
    assert_eq!(view::summary_impl(&lab.state, &edge).unwrap(), None);

    assert!(view::reveal_target(&lab.state, "/etc").is_err());
    assert!(
        view::reveal_target(&lab.state, &lab.state.data_dir.display().to_string()).is_err(),
        "not AI Juggler's whole folder"
    );
}

#[test]
fn save_handover_from_the_session_page_is_private_says_what_was_hidden_and_reveals() {
    use std::os::unix::fs::PermissionsExt as _;
    let lab = lab("strip");
    let saved = view::save_impl(&lab.state, &lab.session).unwrap();
    let dir = Path::new(&saved.path);
    assert!(dir.starts_with(lab.state.data_dir.join("exports")));
    assert_eq!(saved.line.as_deref(), Some("Hidden: 1 API key"));
    let mode = std::fs::metadata(dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700, "this user's only");
    let conversation = std::fs::read_to_string(dir.join("conversation.md")).unwrap();
    assert!(!conversation.contains(KEY));

    // The sheet "Start from a saved handover…" shows says the same.
    let opened = juggler_app::features::packs::preview_impl(&lab.state, dir).unwrap();
    assert_eq!(
        opened
            .withheld
            .iter()
            .map(|row| (row.count, row.what.as_str()))
            .collect::<Vec<_>>(),
        [(1, "API key")]
    );
    assert!(opened.kept.is_empty());

    let target = view::reveal_target(&lab.state, &saved.path).unwrap();
    assert_eq!(target, std::fs::canonicalize(dir).unwrap());
}

#[test]
fn save_handover_waits_for_the_users_choice_about_keys() {
    let lab = lab("strip");
    commands::redaction_policy_set_global_impl(&lab.state, None).unwrap();
    let refused = view::save_impl(&lab.state, &lab.session).unwrap_err();
    assert!(
        refused.to_string().contains("Choose what happens to keys"),
        "it asks for the choice first"
    );
}
