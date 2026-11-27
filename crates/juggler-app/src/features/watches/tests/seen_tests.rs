//! 0.5.2 G45 (RW1 W1): a session AI Juggler didn't start (a `claude` in
//! the user's own terminal, on an App they added) is watchable as one AI
//! Juggler runs: it is open in its project and App (so "Is idle for" is
//! offered there, G32's rule), it is a target of its own by its key, and
//! what the live look sees it do reaches a watch on it and is told.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_core::storage::memory::Writers;

use super::*;
use crate::features::ai::memory::RedactionSecrets;
use crate::features::conversations::scan::{LiveEntry, LiveModel, Place, State};
use crate::features::sessions::seen;
use crate::features::watches::commands;
use crate::features::watches::runtime::Facts;
use crate::features::watches::store::{Origin, WatchStore};
use crate::state::AppState;

const ACCOUNT: &str = "acct-work";
const CONVERSATION: &str = "c-outside-1";
const TITLE: &str = "Update the summary cell in the report notebook";

/// The Claude Code adapter, App `cli:acct-work` ("work") added by the
/// user, project "docs", and in the index a conversation started in the
/// user's own terminal in docs, on that App's folder (no AI Juggler
/// session runs it).
fn lab(dir: &Path) -> (Arc<AppState>, String) {
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::ClaudeCodeAdapter::with_bin(
        dir.join("no-claude"),
    )));
    let state = Arc::new(AppState::init_with_registry(dir.join("data"), registry).unwrap());
    let docs = dir.join("repos/docs");
    std::fs::create_dir_all(docs.join(".git")).unwrap();
    let docs = docs.canonicalize().unwrap();
    let project = {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&crate::test_support::account(
                ACCOUNT,
                juggler_adapters::CLAUDE_CODE_ID,
                "work",
                dir.join("work"),
            ))
            .unwrap();
        storage.projects().ensure(&docs).unwrap().id.to_string()
    };
    state
        .conversations
        .put_for_test(crate::features::conversations::Conversation {
            summary: juggler_adapter::conversation::ConversationSummary {
                id: CONVERSATION.into(),
                cwd: Some(docs.clone()),
                title: Some(TITLE.into()),
                started_ms: Some(crate::commands::now_millis() - 60_000),
                last_ms: Some(crate::commands::now_millis() - 30_000),
                ..Default::default()
            },
            account_id: ACCOUNT.into(),
            tool: juggler_adapters::CLAUDE_CODE_ID.into(),
            path: dir
                .join("work/projects/docs")
                .join(format!("{CONVERSATION}.jsonl")),
            folder: Some(docs),
        });
    (state, project)
}

/// One live look: the conversation open in a terminal in `state`, or
/// (`None`) nothing open.
fn look(state: &Arc<AppState>, dir: &Path, open: Option<State>) {
    let entries = open
        .map(|state| LiveEntry {
            tool: juggler_adapters::CLAUDE_CODE_ID.into(),
            account_id: Some(ACCOUNT.into()),
            session_id: CONVERSATION.into(),
            places: vec![Place {
                pid: 990_201,
                start: None,
                host: None,
                profile: Some(dir.join("work")),
                app_session: None,
            }],
            state: Some(state),
            probably: false,
            viewed_ms: None,
            app_title: None,
            background: false,
        })
        .into_iter()
        .collect();
    crate::features::conversations::ConversationIndex::look_for_test(
        state,
        LiveModel {
            entries,
            ..LiveModel::default()
        },
    )
    .unwrap();
}

fn offered(state: &Arc<AppState>, target: &WatchTarget) -> Vec<String> {
    commands::watches_catalogue_impl(state, target, Lang::En)
        .unwrap()
        .into_iter()
        .map(|entry| entry.code)
        .collect()
}

fn key() -> String {
    seen::source_key(ACCOUNT, CONVERSATION)
}

#[test]
fn a_session_started_outside_is_open_in_its_project_and_app_and_a_target_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let (state, project) = lab(dir.path());
    let app = WatchTarget::App(juggler_core::storage::cli_app_id(ACCOUNT));
    let docs = WatchTarget::Project(project.clone());
    // Not open: the group targets can't count idle from it.
    look(&state, dir.path(), None);
    assert!(!offered(&state, &docs).contains(&"idle_for".to_owned()));
    // Open and idle, as Open now lists it.
    look(&state, dir.path(), Some(State::Idle));
    let open = seen::outside_open(&state);
    assert_eq!(open.len(), 1, "{open:?}");
    assert_eq!(seen::key_of(&open[0]), key());
    assert_eq!(open[0].project_id.as_deref(), Some(project.as_str()));
    assert_eq!(
        open[0].app.as_deref(),
        Some(juggler_core::storage::cli_app_id(ACCOUNT).as_str())
    );
    for target in [&docs, &app, &WatchTarget::Any] {
        let codes = offered(&state, target);
        assert!(
            codes.contains(&"idle_for".to_owned()),
            "{target:?}: {codes:?}"
        );
    }
    // A target of its own: what the live look sees of it, never what AI
    // Juggler sees only of the sessions it runs.
    let own = offered(&state, &WatchTarget::Session(key()));
    for code in ["idle_for", "finishes", "asks_you", "ends"] {
        assert!(own.contains(&code.to_owned()), "{code}: {own:?}");
    }
    for code in ["leaves_processes", "restarted"] {
        assert!(!own.contains(&code.to_owned()), "{code}: {own:?}");
    }
    // Its subject is the one an observation about it resolves to, and a
    // group target's open members hold it (G32's arm_open).
    let storage = state.storage.lock().unwrap();
    let facts = Facts::new(&storage, &state).with_outside(open);
    let subject = facts.session_subject(&key());
    assert_eq!(subject.project.as_deref(), Some(project.as_str()));
    assert_eq!(subject.account.as_deref(), Some(ACCOUNT));
    assert!(
        facts.open_subjects().iter().any(
            |s| super::super::engine::covers(&docs, s) && super::super::engine::covers(&app, s)
        )
    );
    assert_eq!(facts.exists(&WatchTarget::Session(key())), Some(true));
    assert_eq!(
        facts.exists(&WatchTarget::Session(seen::source_key(ACCOUNT, "gone"))),
        Some(false)
    );
}

#[test]
fn what_the_live_look_sees_it_do_reaches_a_watch_on_it_and_is_told_by_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = lab(dir.path());
    // The look at the start: what was open all along is told nothing.
    look(&state, dir.path(), Some(State::Idle));
    let writers = Writers::for_tests();
    let now = crate::commands::now_millis();
    {
        let storage = state.storage.lock().unwrap();
        let registry = Registry::shipped();
        let mut w = watch(
            "w-6a45",
            vec![WatchTarget::Session(key())],
            vec![cond("finishes")],
            Lifetime::Once,
        );
        w.created_ms = now;
        w.even_in_quiet_hours = true;
        WatchStore::new(&storage, &registry)
            .create(
                &writers.watches,
                &RedactionSecrets,
                &w,
                &Origin::WatchesView,
                now,
            )
            .unwrap();
    }
    look(&state, dir.path(), Some(State::Working));
    look(&state, dir.path(), Some(State::Idle));
    let started = Instant::now();
    let told = loop {
        let told: Vec<(String, String)> = {
            let storage = state.storage.lock().unwrap();
            let mut stmt = storage
                .raw_conn()
                .prepare("SELECT key, title FROM notices WHERE kind = 'watch' ORDER BY id")
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        if !told.is_empty() || started.elapsed() > Duration::from_secs(10) {
            break told;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(told.len(), 1, "its turn's end told once: {told:?}");
    assert!(told[0].0.starts_with("watch:w-6a45:"), "{told:?}");
    assert!(told[0].1.contains(TITLE), "named as its row: {told:?}");
}
