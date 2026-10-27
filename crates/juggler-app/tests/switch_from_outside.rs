//! AJ-E2108: a session another app runs (here a Claude conversation AI
//! Juggler never started) continues on another account or another tool
//! through the same switch as one AI Juggler runs: the same targets across
//! tools, the same checks and preflight, the same switch, and one lineage.
//! Its conversation is only read: its file stays byte for byte, it gets no
//! session row, and it stays listed as the other app's.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
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
use juggler_adapter::handover::sha256_hex;
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::features::conversations::commands::list_impl;
use juggler_app::features::conversations::switch_source::source_key;
use juggler_app::features::switch_setup;
use juggler_app::state::AppState;
use juggler_core::model::{EdgeState, SessionId, SwitchEdgeId, SwitchMechanism};
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// A stub that records its argv (one per line) under its own home.
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

fn app_state(dir: &Path) -> Arc<AppState> {
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
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// Waits for (and removes) a stub's argv record.
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

const NATIVE: &str = "0b5e0e5c-6c0b-4c55-9a6b-2a7d3c9f1e11";

/// A real-shaped Claude conversation in `home`, run in `cwd`.
fn conversation(home: &Path, cwd: &Path) -> PathBuf {
    let dir = home
        .join("projects")
        .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{NATIVE}.jsonl"));
    let cwd = cwd.display();
    let lines = [
        format!(
            r#"{{"type":"user","uuid":"u1","sessionId":"{NATIVE}","cwd":"{cwd}","timestamp":"2026-10-25T08:00:00.000Z","message":{{"role":"user","content":"Add a health check endpoint"}}}}"#
        ),
        format!(
            r#"{{"type":"assistant","uuid":"u2","parentUuid":"u1","sessionId":"{NATIVE}","cwd":"{cwd}","timestamp":"2026-10-25T08:00:05.000Z","message":{{"role":"assistant","content":[{{"type":"text","text":"Added GET /health."}}]}}}}"#
        ),
    ];
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    path
}

struct World {
    state: Arc<AppState>,
    /// The Claude account the conversation is on, and its launch settings.
    work: (String, String),
    /// Another Claude account's launch settings.
    home_recipe: String,
    /// A Codex account's launch settings, and its folder.
    codex_recipe: String,
    codex_home: PathBuf,
    transcript: PathBuf,
}

fn world(dir: &Path) -> World {
    let root = dir.canonicalize().unwrap();
    let state = app_state(&root);
    let project = root.join("storefront");
    std::fs::create_dir_all(project.join(".git")).unwrap();
    let account = |tool: &str, label: &str| {
        let account = commands::create_account(&state, tool, label, None).unwrap();
        let recipe = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
        (account.id, recipe.id)
    };
    let work = account(CLAUDE_CODE_ID, "Work");
    let (_, home_recipe) = account(CLAUDE_CODE_ID, "Home");
    let (codex, codex_recipe) = account(CODEX_ID, "Personal");
    let transcript = conversation(
        &state.accounts_dir().join(CLAUDE_CODE_ID).join(&work.0),
        &project,
    );
    World {
        codex_home: state.accounts_dir().join(CODEX_ID).join(&codex),
        state,
        work,
        home_recipe,
        codex_recipe,
        transcript,
    }
}

fn edge_of(state: &Arc<AppState>, edge: &str) -> juggler_core::model::SwitchEdge {
    state
        .storage
        .lock()
        .unwrap()
        .switch_graph()
        .edge(&SwitchEdgeId::new(edge))
        .unwrap()
}

fn epoch_of(
    state: &Arc<AppState>,
    edge: &juggler_core::model::SwitchEdge,
    id: &juggler_core::model::EpochId,
) -> juggler_core::model::Epoch {
    state
        .storage
        .lock()
        .unwrap()
        .switch_graph()
        .epochs_for_work(&edge.work)
        .unwrap()
        .into_iter()
        .find(|epoch| &epoch.id == id)
        .unwrap()
}

#[test]
fn a_session_another_app_runs_is_offered_every_profile_of_every_tool() {
    let work = tempfile::tempdir().unwrap();
    let w = world(work.path());
    let listed = list_impl(&w.state).unwrap();
    let found = listed.iter().find(|c| c.id == NATIVE).expect("indexed");
    assert!(found.session_id.is_none(), "AI Juggler never started it");
    let key = source_key(&w.work.0, NATIVE);

    let targets = commands::switch_targets_impl(&w.state, &key).unwrap();
    let codex = targets
        .iter()
        .find(|t| t.recipe_id == w.codex_recipe)
        .expect("Codex is offered for a Claude session");
    assert!(codex.cross_provider, "Codex is another tool");
    assert_eq!(codex.provider, CODEX_ID);
    assert!(!codex.badge.is_empty(), "every target carries its badge");
    let home = targets
        .iter()
        .find(|t| t.recipe_id == w.home_recipe)
        .expect("another Claude account is offered");
    assert!(!home.cross_provider);
    assert!(
        targets.iter().all(|t| t.recipe_id != w.work.1),
        "where it is already is never a target"
    );

    // The checks and the preflight read it the same way, and write nothing.
    let check = switch_setup::check_impl(&w.state, &key, &w.codex_recipe).unwrap();
    assert_eq!(check.from, "Work");
    let preflight = commands::switch_preflight_impl(&w.state, &key, &w.codex_recipe).unwrap();
    assert!(
        preflight.source_outside,
        "the screen says it stays as it is"
    );
    assert!(!preflight.source_live, "it isn't open now");
    assert!(preflight.source_app.is_none());
    let projects = commands::projects_list_impl(&w.state).unwrap();
    assert!(
        projects.iter().all(|p| p.name != "storefront"),
        "a look at the switch records no project"
    );
}

#[test]
fn switching_it_continues_the_conversation_and_leaves_the_original_alone() {
    let work = tempfile::tempdir().unwrap();
    let w = world(work.path());
    list_impl(&w.state).unwrap();
    let before = sha256_hex(&std::fs::read(&w.transcript).unwrap());
    commands::redaction_policy_set_global_impl(&w.state, Some("strip")).unwrap();
    let key = source_key(&w.work.0, NATIVE);

    // To another tool: the conversation replays into a new Codex session.
    let to_codex =
        commands::switch_execute_impl(&w.state, &key, &w.codex_recipe, "finish-turn", None, &[])
            .unwrap();
    let args = args_of(&w.codex_home);
    let handover = args
        .iter()
        .find(|line| line.ends_with("HANDOVER.md"))
        .expect("the prompt points at its handover");
    let brief = std::fs::read_to_string(handover).unwrap();
    assert!(
        brief.contains("Add a health check endpoint") && brief.contains("Added GET /health."),
        "the conversation travels"
    );
    let codex_session = {
        let storage = w.state.storage.lock().unwrap();
        storage
            .sessions()
            .get(&SessionId::new(to_codex.session_id.clone()))
            .unwrap()
    };
    assert!(
        codex_session.project.is_some(),
        "the switch records the project"
    );

    // The lineage: one confirmed replay edge from the conversation itself.
    let edge = edge_of(&w.state, &to_codex.edge_id);
    assert_eq!(edge.state, EdgeState::Confirmed);
    assert_eq!(edge.mechanism, SwitchMechanism::Replay);
    let from = epoch_of(&w.state, &edge, &edge.from_epoch);
    assert!(
        from.session_ref.is_none(),
        "the original is no session of ours"
    );
    assert_eq!(from.native_id.as_deref(), Some(NATIVE));
    assert_eq!(from.account.as_str(), w.work.0);
    let to = epoch_of(&w.state, &edge, edge.to_epoch.as_ref().unwrap());
    assert_eq!(
        to.session_ref.as_ref().map(SessionId::as_str),
        Some(to_codex.session_id.as_str())
    );

    // History › Replay names where it came from as every surface does:
    // the account's name and the tool (live lab 12 showed the id).
    {
        let storage = w.state.storage.lock().unwrap();
        let mut ended = storage
            .sessions()
            .get(&SessionId::new(to_codex.session_id.clone()))
            .unwrap();
        ended.status = juggler_core::model::SessionStatus::Finished;
        storage.sessions().update(&ended).unwrap();
    }
    let replay =
        juggler_app::history::instance_replay_impl(&w.state, &to_codex.session_id).unwrap();
    assert_eq!(replay.lineage.len(), 1, "one hop arrived here");
    assert_eq!(replay.lineage[0].direction, "in");
    assert_eq!(
        replay.lineage[0].other_account, "Work",
        "the account by its name, never its id"
    );
    assert_eq!(replay.lineage[0].other_provider, CLAUDE_CODE_ID);

    // To another Claude account: the same conversation, from the same epoch.
    let to_home =
        commands::switch_execute_impl(&w.state, &key, &w.home_recipe, "finish-turn", None, &[])
            .unwrap();
    let second = edge_of(&w.state, &to_home.edge_id);
    assert_eq!(second.mechanism, SwitchMechanism::Transplant);
    assert_eq!(
        second.from_epoch, edge.from_epoch,
        "every switch from it starts from its one epoch"
    );

    // The original: read only, no row, still the other app's in the list.
    assert_eq!(
        sha256_hex(&std::fs::read(&w.transcript).unwrap()),
        before,
        "the original's file is untouched"
    );
    {
        let storage = w.state.storage.lock().unwrap();
        assert!(
            storage
                .sessions()
                .get(&SessionId::new(key.clone()))
                .is_err()
        );
        assert!(
            storage
                .sessions()
                .conversation_links()
                .unwrap()
                .iter()
                .all(|(account, conversation, _)| account != &w.work.0 || conversation != NATIVE),
            "no session of ours claims the original"
        );
    }
    let listed = list_impl(&w.state).unwrap();
    let original = listed
        .iter()
        .find(|c| c.id == NATIVE && c.account_id == w.work.0)
        .expect("the original stays listed");
    assert!(original.session_id.is_none());

    // Going back reaches the original conversation on Work.
    let plan = commands::switch_back_plan_impl(&w.state, &to_home.session_id).unwrap();
    assert_eq!(plan.original_account, "Work");
    assert!(plan.original_alive, "its conversation is there to resume");

    // Undo has nothing of ours to resume: it says so, and stops nothing.
    let undo = commands::switch_undo_impl(&w.state, &to_codex.edge_id).unwrap_err();
    assert!(
        undo.to_string().contains("another app runs"),
        "undo explains the original is another app's"
    );
}

/// Live lab 12: the switch graph showed the original (a conversation
/// another app runs) as "no transcript", while switching back resumed
/// it. Its node is resumable, Resume continues it here, and its pack
/// exports.
#[test]
fn the_graph_resumes_and_exports_an_original_another_app_runs() {
    let work = tempfile::tempdir().unwrap();
    let w = world(work.path());
    list_impl(&w.state).unwrap();
    commands::redaction_policy_set_global_impl(&w.state, Some("strip")).unwrap();
    let key = source_key(&w.work.0, NATIVE);
    let to_codex =
        commands::switch_execute_impl(&w.state, &key, &w.codex_recipe, "finish-turn", None, &[])
            .unwrap();
    let _ = args_of(&w.codex_home);
    let edge = edge_of(&w.state, &to_codex.edge_id);
    let project = w
        .state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(to_codex.session_id.clone()))
        .unwrap()
        .project
        .unwrap();

    let works = commands::switch_graph_overview_impl(&w.state, project.as_str()).unwrap();
    let node = works[0]
        .epochs
        .iter()
        .find(|node| node.id == edge.from_epoch.as_str())
        .expect("the original is on the timeline");
    assert!(node.resumable, "its conversation is there to resume");
    assert!(node.session_id.is_none(), "it stays the other app's");

    let pack = commands::pack_export_impl(&w.state, edge.from_epoch.as_str()).unwrap();
    let conversation =
        std::fs::read_to_string(Path::new(&pack).join("conversation.md")).unwrap();
    assert!(
        conversation.contains("Add a health check endpoint"),
        "its pack holds its conversation"
    );

    let resumed = commands::epoch_resume_impl(&w.state, edge.from_epoch.as_str()).unwrap();
    let args = args_of(&w.state.accounts_dir().join(CLAUDE_CODE_ID).join(&w.work.0));
    assert!(
        args.iter().any(|arg| arg == NATIVE),
        "Resume continues the original conversation"
    );
    let row = w
        .state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(resumed.session_id))
        .unwrap();
    assert_eq!(row.conversation.as_ref().map(|c| c.as_str()), Some(NATIVE));
}

/// Live lab 12: back from a replay, the delta said "0 shared before the
/// switch, N new here" for an original that hadn't moved. A replay shares
/// no message ids with what it replayed: what was shared is the original
/// as the switch read it, and only what it gained since is new there.
#[test]
fn back_from_a_replay_counts_the_original_as_the_switch_read_it() {
    let work = tempfile::tempdir().unwrap();
    let w = world(work.path());
    list_impl(&w.state).unwrap();
    commands::redaction_policy_set_global_impl(&w.state, Some("strip")).unwrap();
    let key = source_key(&w.work.0, NATIVE);
    let to_codex =
        commands::switch_execute_impl(&w.state, &key, &w.codex_recipe, "finish-turn", None, &[])
            .unwrap();
    let _ = args_of(&w.codex_home);

    let plan = commands::switch_back_plan_impl(&w.state, &to_codex.session_id).unwrap();
    assert_eq!(plan.base_turns, 2, "both of its turns went into the replay");
    assert_eq!(plan.original_only_turns, 0, "nothing happened there since");
    assert!(
        !plan.delta_preview.contains("new here"),
        "the delta claims no new turns on the original"
    );

    // A turn on the original after the switch is new there, and only it.
    let mut text = std::fs::read_to_string(&w.transcript).unwrap();
    text.push_str(&format!(
        r#"{{"type":"user","uuid":"u3","parentUuid":"u2","sessionId":"{NATIVE}","message":{{"role":"user","content":"And a readiness check"}}}}"#
    ));
    text.push('\n');
    std::fs::write(&w.transcript, text).unwrap();
    let plan = commands::switch_back_plan_impl(&w.state, &to_codex.session_id).unwrap();
    assert_eq!(plan.base_turns, 2);
    assert_eq!(plan.original_only_turns, 1, "one turn there since the switch");
}
