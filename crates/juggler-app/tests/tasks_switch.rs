//! AJ-E1904, end to end: a switch keeps its task. A planner claims a task
//! through its agent tools, then its session is continued on another
//! account with the app's real switch (the stub harness): the new session
//! holds the same task, sees it as its own, can say where it is, its card
//! shows it, the switch history shows each session's task, and the task's
//! own history has the switch as a line.

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
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_agentbridge::wire::{
    self, Args, Call, Request, Response, TaskArgs, TaskUpdateArgs, TasksListArgs,
};
use juggler_app::commands;
use juggler_app::features::tasks::{Actor, apply, views};
use juggler_app::features::{agent_bridge, pools};
use juggler_app::state::AppState;
use juggler_core::model::SessionId;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn claude_stub(dir: &Path) -> PathBuf {
    let path = dir.join("claude-stub");
    stand_in::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/last-args.tmp\" && mv \"$CLAUDE_CONFIG_DIR/last-args.tmp\" \"$CLAUDE_CONFIG_DIR/last-args\"\n",
    );
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = claude_stub(dir);
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    let state = Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap());
    state
        .agent_bridge
        .set_program(Some(PathBuf::from("/bin/echo")));
    agent_bridge::settings::answer_impl(&state, true).unwrap();
    state
}

fn config_dir_of(state: &Arc<AppState>, account_id: &str) -> PathBuf {
    state.accounts_dir().join(CLAUDE_CODE_ID).join(account_id)
}

/// Launches one session and pins its transcript (the watcher's job).
fn launch_with_transcript(
    state: &Arc<AppState>,
    profile: &str,
    account: &str,
    cwd: &str,
    native: &str,
) -> String {
    let launched = commands::launch_profile(state, profile, Some(cwd.to_owned())).unwrap();
    let args = config_dir_of(state, account).join("last-args");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !args.is_file() {
        assert!(std::time::Instant::now() < deadline, "the stub never ran");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let dir = config_dir_of(state, account)
        .join("projects")
        .join(cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let transcript = dir.join(format!("{native}.jsonl"));
    std::fs::write(
        &transcript,
        format!("{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"message\":{{}}}}\n{{\"type\":\"assistant\",\"cwd\":\"{cwd}\"}}\n"),
    )
    .unwrap();
    let storage = state.storage.lock().unwrap();
    let mut record = storage
        .sessions()
        .get(&SessionId::new(launched.session_id.clone()))
        .unwrap();
    record.transcript_ref = Some(juggler_core::model::TranscriptRef {
        path: transcript,
        native_id: Some(native.to_owned()),
    });
    storage.sessions().update(&record).unwrap();
    drop(storage);
    launched.session_id
}

/// A fresh agent-tools key for a session, minted as a launch mints it.
fn key(state: &Arc<AppState>, session: &str, account: &str, project: &str) -> String {
    let adapter = ClaudeCodeAdapter::with_bin(Path::new("/bin/true"));
    let mut spec = juggler_adapter::launch::interactive_spec(
        PathBuf::from("/bin/true"),
        vec![],
        std::collections::BTreeMap::new(),
        PathBuf::from("/"),
    );
    let prepared = agent_bridge::prepare(
        state,
        &adapter,
        &mut spec,
        &SessionId::new(session),
        Some(project),
        account,
    );
    assert!(
        !matches!(prepared, agent_bridge::Prepared::Not(_)),
        "{prepared:?}"
    );
    spec.env.get(wire::TOKEN_ENV).cloned().unwrap()
}

fn call(state: &Arc<AppState>, key: &str, what: Call, args: Args) -> Response {
    let request = Request::new(key.to_owned(), what, args);
    agent_bridge::calls::handle(state, &serde_json::to_string(&request).unwrap())
}

#[test]
fn a_switch_keeps_its_task_and_the_history_shows_each_sessions() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project_dir = work.path().join("storefront");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();
    let a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let profile_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let planner = launch_with_transcript(&state, &profile_a.id, &a.id, &cwd, "native-p");
    let project = state
        .storage
        .lock()
        .unwrap()
        .projects()
        .by_root(&std::fs::canonicalize(&cwd).unwrap())
        .unwrap()
        .unwrap()
        .id;
    let pool = juggler_core::storage::default_pool_id(Some(&project));
    // The pool's lead writes it (the app writes no task, D-44).
    let lead = Actor {
        session: None,
        member_ref: "session:s-lead".into(),
        name: "Lead".into(),
        role: juggler_core::tasks::Role::Lead,
        refs: vec!["session:s-lead".into()],
    };
    let id = apply::create(
        &state,
        &lead,
        &pool,
        &wire::TaskCreateArgs {
            title: "Build the API".into(),
            body: Some("Orders, then the cart.".into()),
            priority: Some("high".into()),
            ..wire::TaskCreateArgs::default()
        },
    )
    .unwrap()
    .id;
    let planner_key = key(&state, &planner, &a.id, project.as_str());
    let claimed = call(
        &state,
        &planner_key,
        Call::TaskClaim,
        Args {
            task: Some(TaskArgs { task: id.clone() }),
            ..Args::default()
        },
    );
    assert!(claimed.result.is_some(), "{claimed:?}");

    // The planner hits its limit and is continued on the other account.
    let next =
        commands::switch_execute_impl(&state, &planner, &profile_b.id, "stop-now", None, &[])
            .unwrap()
            .session_id;
    assert_ne!(next, planner);

    // AJ-E1904 (the 2026-10-28 lab, step 6): the switch is a line of the
    // task's own history, and the new session is one of its sessions,
    // before it has done anything with the task.
    let detail = views::detail(&state, &id, None).unwrap();
    let line = detail
        .history
        .iter()
        .find(|event| event.what.starts_with("continued on"))
        .unwrap_or_else(|| panic!("no switch line: {:?}", detail.history));
    assert_eq!(line.what, "continued on Personal (from Work Max)");
    assert!(line.by_agent, "the holder's line, not the user's");
    assert_eq!(
        detail.history[0].id, line.id,
        "the newest line: {:?}",
        detail.history
    );
    let how: Vec<(&str, &str)> = detail
        .sessions
        .iter()
        .map(|s| (s.session_id.as_str(), s.how.as_str()))
        .collect();
    assert!(
        how.contains(&(planner.as_str(), "claimed")) && how.contains(&(next.as_str(), "switched")),
        "{how:?}"
    );

    // The new session holds the same task: its own, workable, on its card.
    let next_key = key(&state, &next, &b.id, project.as_str());
    let listed = call(
        &state,
        &next_key,
        Call::TasksList,
        Args {
            tasks: Some(TasksListArgs {
                mine: true,
                ..TasksListArgs::default()
            }),
            ..Args::default()
        },
    );
    let mine = listed.result.unwrap()["tasks"].as_array().unwrap().clone();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0]["id"], id.as_str());
    assert_eq!(mine[0]["yours"], true);
    assert_eq!(mine[0]["status"], "in_progress");
    let card = pools::cards::cards_impl(&state, &pool)
        .unwrap()
        .cards
        .into_iter()
        .find(|c| c.session_id == next)
        .unwrap();
    assert!(
        card.task.value.unwrap().starts_with(&id),
        "the card shows it"
    );
    let review = call(
        &state,
        &next_key,
        Call::TaskUpdate,
        Args {
            task_change: Some(TaskUpdateArgs {
                task: id.clone(),
                status: Some("in_review".into()),
                ..TaskUpdateArgs::default()
            }),
            ..Args::default()
        },
    );
    assert!(review.result.is_some(), "{review:?}");

    // The switch history shows each session's task.
    let graph = commands::switch_graph_overview_impl(&state, project.as_str()).unwrap();
    let nodes: Vec<_> = graph.iter().flat_map(|w| w.epochs.iter()).collect();
    for session in [&planner, &next] {
        let node = nodes
            .iter()
            .find(|n| n.session_id.as_deref() == Some(session.as_str()))
            .unwrap_or_else(|| panic!("no node for {session}: {nodes:?}"));
        assert_eq!(
            node.task.as_deref(),
            Some(format!("{id} · Build the API").as_str())
        );
    }
    let detail = views::detail(&state, &id, None).unwrap();
    let worked: Vec<&str> = detail
        .sessions
        .iter()
        .map(|s| s.session_id.as_str())
        .collect();
    assert!(
        worked.contains(&planner.as_str()) && worked.contains(&next.as_str()),
        "{worked:?}"
    );
}
