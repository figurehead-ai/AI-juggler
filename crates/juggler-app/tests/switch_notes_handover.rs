//! D-51 for a tool that keeps no notes (Codex): the switch check offers to
//! include this project's notes in the handover, unticked; only with the
//! user's tick does the switch quote their text in the handover file,
//! under its own heading. A note the keys check finds a key in is left out
//! whole, and the offer and the handover's state say which. Nothing is
//! written to the other tool's own files. Temp folders and stub tools
//! only.

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
use juggler_adapters::claude_memory::{INDEX, memory_folder};
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::features::handover::view;
use juggler_app::features::switch_setup::{apply_impl, check_impl};
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

fn args_of(home: &Path) -> Vec<String> {
    let file = home.join("last-args");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !file.is_file() {
        assert!(std::time::Instant::now() < deadline, "the stub never ran");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let args = std::fs::read_to_string(&file)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    std::fs::remove_file(&file).unwrap();
    args
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

const KEY: &str = "sk-proj-abcdefghijklmnopqrstuvwx0123456789";

struct Lab {
    state: Arc<AppState>,
    session: String,
    codex_profile: String,
    codex_home: PathBuf,
    _work: tempfile::TempDir,
}

fn lab() -> Lab {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path();
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(stub(
        dir,
        "claude-stub",
        "CLAUDE_CONFIG_DIR",
    )));
    let codex = Arc::new(CodexAdapter::with_bin(stub(
        dir,
        "codex-stub",
        "CODEX_HOME",
    )));
    registry.register(claude.clone());
    registry.register_handover(claude);
    registry.register(codex.clone());
    registry.register_handover(codex);
    let state = Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap());
    commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();
    let project = dir.join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let b = commands::create_account(&state, CODEX_ID, "Codex", None).unwrap();
    let profile_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let a_home = state.accounts_dir().join(CLAUDE_CODE_ID).join(&a.id);
    let codex_home = state.accounts_dir().join(CODEX_ID).join(&b.id);
    std::fs::create_dir_all(&codex_home).unwrap();

    // Work Max's notes for the shop: two plain ones, and one with a key.
    let folder = memory_folder(&a_home, &project);
    write(
        &folder.join(INDEX),
        "- [Codename](project-codename.md)\n- [Tests](testing.md)\n- [Deploy](deploy.md)\n",
    );
    write(
        &folder.join("project-codename.md"),
        "The codename is Bluebird.\n",
    );
    write(&folder.join("testing.md"), "Run just test.\n");
    write(
        &folder.join("deploy.md"),
        &format!("Deploy with OPENAI_API_KEY={KEY}\n"),
    );

    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();
    let _ = args_of(&a_home);
    // The conversation the switch replays.
    let dir = a_home.join("projects").join(cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let transcript = dir.join("claude-sess-1.jsonl");
    std::fs::write(
        &transcript,
        format!(
            "{{\"type\":\"user\",\"uuid\":\"u1\",\"cwd\":\"{cwd}\",\"sessionId\":\"claude-sess-1\",\"message\":{{\"role\":\"user\",\"content\":\"What is the codename?\"}}}}\n{{\"type\":\"assistant\",\"uuid\":\"u2\",\"parentUuid\":\"u1\",\"cwd\":\"{cwd}\",\"sessionId\":\"claude-sess-1\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"I noted it.\"}}]}}}}\n"
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
        codex_home,
        _work: work,
    }
}

/// The handover the new session's first prompt points at, and its state.
fn handover_of(args: &[String]) -> (String, serde_json::Value) {
    let path = args
        .iter()
        .find(|line| line.ends_with("HANDOVER.md"))
        .unwrap_or_else(|| panic!("the prompt points at its handover: {args:?}"));
    let path = Path::new(path);
    let state = std::fs::read_to_string(path.with_file_name("state.json")).unwrap();
    (
        std::fs::read_to_string(path).unwrap(),
        serde_json::from_str(&state).unwrap(),
    )
}

#[test]
fn the_offer_is_to_include_them_in_the_handover_never_ticked_and_says_what_is_left_out() {
    let lab = lab();
    let check = check_impl(&lab.state, &lab.session, &lab.codex_profile).unwrap();
    let offer = check
        .offers
        .iter()
        .find(|offer| offer.kind == "memory")
        .unwrap_or_else(|| panic!("no notes offer: {:?}", check.offers));
    assert_eq!(offer.verdict, "handover");
    assert_eq!(offer.count, Some(2), "the note with a key isn't counted");
    assert!(!offer.suggested, "included only when the user ticks it");
    assert_eq!(
        offer.note.as_deref(),
        Some("deploy.md looks like it holds a key or password, so it's left out")
    );
    assert_eq!(check.notes_stay, Some(3), "unticked, all three stay");

    // Ticked and applied before the switch: nothing to set up, nothing
    // written to Codex's own folder.
    let listing = |dir: &Path| {
        let mut there: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        there.sort();
        there
    };
    let before = listing(&lab.codex_home);
    let applied = apply_impl(
        &lab.state,
        &lab.session,
        &lab.codex_profile,
        std::slice::from_ref(&offer.id),
    )
    .unwrap();
    assert!(applied.is_empty(), "{applied:?}");
    assert_eq!(
        listing(&lab.codex_home),
        before,
        "Codex's folder is untouched"
    );
}

#[test]
fn ticked_the_handover_quotes_them_under_a_heading_without_the_key() {
    let lab = lab();
    let check = check_impl(&lab.state, &lab.session, &lab.codex_profile).unwrap();
    let offer = check
        .offers
        .iter()
        .find(|offer| offer.kind == "memory")
        .unwrap();
    let edge = commands::switch_execute_with(
        &lab.state,
        &lab.session,
        &lab.codex_profile,
        "finish-turn",
        None,
        &[],
        std::slice::from_ref(&offer.id),
    )
    .unwrap()
    .edge_id;
    let (brief, state) = handover_of(&args_of(&lab.codex_home));
    assert!(
        brief.contains("This project's notes from Work Max: what Claude Code noted for itself"),
        "{brief}"
    );
    assert!(
        brief.contains("project-codename.md:\n> The codename is Bluebird."),
        "{brief}"
    );
    assert!(brief.contains("testing.md:\n> Run just test."));
    assert!(!brief.contains(KEY), "the key never travels");
    assert!(
        !brief.contains("OPENAI_API_KEY"),
        "nor the note that held it"
    );
    assert!(brief.contains(
        "(Not quoted: deploy.md looks like it holds a key or password, so it's left out.)"
    ));
    // The notes come after the conversation.
    assert!(brief.find("What is the codename?").unwrap() < brief.find("Bluebird").unwrap());
    assert_eq!(
        state["notes_quoted"],
        serde_json::json!(["project-codename.md", "testing.md"])
    );
    assert_eq!(state["notes_left_out"], serde_json::json!(["deploy.md"]));

    // Afterwards: what Codex was told, in one line and in the sheet. The
    // note with a key was left out whole: left out, not "hidden".
    let summary = view::summary_impl(&lab.state, &edge).unwrap().unwrap();
    assert_eq!(summary.line.as_deref(), Some("Left out: 1 note"));
    let sheet = view::open_impl(&lab.state, &edge).unwrap();
    assert_eq!(
        sheet.carries,
        ["the conversation (2 turns)", "2 notes for this project"]
    );
    assert_eq!(
        sheet.not_carried,
        ["deploy.md: a key or password was found in it, or it didn't fit"]
    );
    assert_eq!(sheet.withheld, Some(Vec::new()), "nothing in the conversation");
}

#[test]
fn not_ticked_the_handover_has_no_notes() {
    let lab = lab();
    commands::switch_execute_with(
        &lab.state,
        &lab.session,
        &lab.codex_profile,
        "finish-turn",
        None,
        &[],
        // An offer id that isn't this one's is no approval.
        &["memory-0000000000000000".to_owned()],
    )
    .unwrap();
    let (brief, state) = handover_of(&args_of(&lab.codex_home));
    assert!(!brief.contains("Bluebird"), "{brief}");
    assert!(!brief.contains("This project's notes"));
    assert!(state.get("notes_quoted").is_none());
}
