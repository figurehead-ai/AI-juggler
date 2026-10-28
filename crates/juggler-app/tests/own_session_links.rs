//! 0.2 app-level: an AI Juggler terminal session of a tool without hooks
//! (OpenCode, Grok Build, Kimi Code) learns the conversation its launch
//! created, through its adapter (`features::sessions::linking`), so the
//! rail shows the work once: the session's row carries the "+N agents"
//! count, and the conversation's row is linked to it (the rail folds it
//! in). When more than one conversation could be its own, nothing is
//! linked and the rail stays as it was. An account used in place (shared
//! with the user's own terminal) is linked by its launch only when its
//! tool's records name the process that holds a session. Over the
//! conformance kit's synthetic records for each tool, read-only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "../../juggler-adapters/tests/grok_fixture/mod.rs"]
mod grok_fixture;
#[path = "../../juggler-adapters/tests/kimi_fixture/mod.rs"]
mod kimi_fixture;
#[path = "../../juggler-adapters/tests/opencode_fixture/mod.rs"]
mod opencode_fixture;
#[path = "../../juggler-adapters/tests/subagent_fixture/mod.rs"]
mod subagent_fixture;

use std::path::PathBuf;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{
    GROK_BUILD_ID, GrokBuildAdapter, KIMI_CODE_ID, KimiCodeAdapter, OPENCODE_ID, OpenCodeAdapter,
};
use juggler_app::commands;
use juggler_app::features::sessions::linking;
use juggler_app::features::subagents::count;
use juggler_app::state::AppState;
use juggler_core::model::{
    InstanceKind, InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
};

const TOOLS: [&str; 3] = [OPENCODE_ID, GROK_BUILD_ID, KIMI_CODE_ID];

struct Lab {
    state: Arc<AppState>,
    tool: &'static str,
    state_dir: PathBuf,
    profile: ProfileId,
    cwd: PathBuf,
    _dir: tempfile::TempDir,
}

fn lab(tool: &'static str) -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let cwd = root.join("Dev/shop");
    std::fs::create_dir_all(&cwd).unwrap();
    let home = root.join("home");
    let mut registry = AdapterRegistry::new();
    match tool {
        OPENCODE_ID => registry.register(Arc::new(
            OpenCodeAdapter::with_bin("/usr/bin/true").with_home(&home),
        )),
        GROK_BUILD_ID => registry.register(Arc::new(
            GrokBuildAdapter::with_bin("/usr/bin/true").with_home(&home),
        )),
        _ => registry.register(Arc::new(
            KimiCodeAdapter::with_bin("/usr/bin/true").with_home(&home),
        )),
    }
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let account = commands::create_account(&state, tool, "Work", None).unwrap();
    let profile = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
    let state_dir = state.accounts_dir().join(tool).join(&account.id);
    Lab {
        state,
        tool,
        state_dir,
        profile: ProfileId::new(profile.id),
        cwd,
        _dir: dir,
    }
}

/// A session AI Juggler started in the lab's folder a moment ago, its
/// conversation not known.
fn started(lab: &Lab, id: &str) -> SessionId {
    let session = SessionId::new(id);
    lab.state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: lab.profile.clone(),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: lab.cwd.clone(),
            status: SessionStatus::Running,
            started_at: Timestamp::from_millis(Timestamp::now().millis() - 1_000),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            transcript_ref: None,
            attention: None,
            running: None,
            window: None,
        })
        .unwrap();
    session
}

/// The tool's record of the session it started, with subagents (one
/// still running), as the conformance kit writes it; its id.
fn tools_record(lab: &Lab) -> String {
    let (_, id) = match lab.tool {
        OPENCODE_ID => subagent_fixture::opencode(
            &OpenCodeAdapter::with_bin("/usr/bin/true").db_path(&lab.state_dir),
            opencode_fixture::Line::V2,
            &lab.cwd,
        ),
        GROK_BUILD_ID => subagent_fixture::grok(&lab.state_dir, &lab.cwd),
        _ => subagent_fixture::kimi(&lab.state_dir, &lab.cwd),
    };
    id
}

/// Another conversation of the tool in the same folder, just started.
fn another_record(lab: &Lab) {
    match lab.tool {
        OPENCODE_ID => {
            let db = OpenCodeAdapter::with_bin("/usr/bin/true").db_path(&lab.state_dir);
            let conn = rusqlite::Connection::open(&db).unwrap();
            opencode_fixture::add_session(
                &conn,
                opencode_fixture::Line::V2,
                "ses_another",
                &lab.cwd,
                None,
                opencode_fixture::now_ms(),
            );
        }
        GROK_BUILD_ID => {
            grok_fixture::add_session(
                &lab.state_dir,
                "0199bb00-0000-7000-8000-0000000000b0",
                &lab.cwd,
                "another",
            );
        }
        _ => {
            kimi_fixture::add_session(&lab.state_dir, "kit-kimi-another", &lab.cwd, "another");
        }
    }
}

/// Marks the lab's account as used in place (ADR-0018): the user's own
/// folder, which their own terminal uses too.
fn in_place(lab: &Lab) {
    let storage = lab.state.storage.lock().unwrap();
    let profile = storage.profiles().get(&lab.profile).unwrap();
    let mut account = storage.accounts().get(&profile.account).unwrap();
    account.in_place = true;
    storage.accounts().update(&account).unwrap();
}

fn conversation_of(lab: &Lab, session: &SessionId) -> Option<String> {
    let storage = lab.state.storage.lock().unwrap();
    storage
        .sessions()
        .get(session)
        .unwrap()
        .conversation
        .map(|id| id.as_str().to_owned())
}

fn index(lab: &Lab) {
    juggler_app::features::conversations::tick(&lab.state).unwrap();
}

#[test]
fn a_hookless_tools_session_is_one_rail_row_carrying_its_count() {
    for tool in TOOLS {
        let lab = lab(tool);
        let session = started(&lab, "sess-own");
        let id = tools_record(&lab);
        index(&lab);

        // Before: the conversation is a row of its own, linked to nothing.
        let row = |lab: &Lab| {
            juggler_app::features::conversations::commands::list_impl(&lab.state)
                .unwrap()
                .into_iter()
                .find(|row| row.id == id)
                .unwrap_or_else(|| panic!("{tool}: the conversation isn't listed"))
        };
        assert_eq!(row(&lab).session_id, None, "{tool}");

        assert_eq!(linking::tick(&lab.state), 1, "{tool}: linked");
        assert_eq!(
            conversation_of(&lab, &session).as_deref(),
            Some(id.as_str()),
            "{tool}: the conversation its launch created"
        );
        assert_eq!(linking::tick(&lab.state), 0, "{tool}: once");

        // After: the conversation's row is the session's (the rail folds
        // it in), and the session's row carries the count.
        assert_eq!(
            row(&lab).session_id.as_deref(),
            Some("sess-own"),
            "{tool}: linked rows"
        );
        count::tick(&lab.state);
        let sessions = {
            let storage = lab.state.storage.lock().unwrap();
            commands::session_infos_snapshot(&storage).unwrap()
        };
        let own = count::counted(&lab.state, sessions)
            .into_iter()
            .find(|row| row.id == "sess-own")
            .unwrap();
        assert_eq!(own.subtask_count, Some(1), "{tool}: the count on its row");
    }
}

#[test]
fn two_conversations_in_its_folder_link_none() {
    for tool in TOOLS {
        let lab = lab(tool);
        let session = started(&lab, "sess-own");
        tools_record(&lab);
        another_record(&lab);
        index(&lab);
        assert_eq!(linking::tick(&lab.state), 0, "{tool}: guessed between two");
        assert_eq!(conversation_of(&lab, &session), None, "{tool}");
    }
}

#[test]
fn two_unlinked_sessions_in_one_folder_link_none() {
    for tool in TOOLS {
        let lab = lab(tool);
        let first = started(&lab, "sess-a");
        let second = started(&lab, "sess-b");
        tools_record(&lab);
        index(&lab);
        assert_eq!(
            linking::tick(&lab.state),
            0,
            "{tool}: either session could own it"
        );
        assert_eq!(conversation_of(&lab, &first), None, "{tool}");
        assert_eq!(conversation_of(&lab, &second), None, "{tool}");
    }
}

#[test]
fn a_conversation_linked_to_another_session_is_never_taken() {
    for tool in TOOLS {
        let lab = lab(tool);
        let id = tools_record(&lab);
        let other = started(&lab, "sess-other");
        {
            let storage = lab.state.storage.lock().unwrap();
            let mut record = storage.sessions().get(&other).unwrap();
            record.conversation = Some(juggler_core::model::ConversationId::new(id.as_str()));
            storage.sessions().update(&record).unwrap();
        }
        let session = started(&lab, "sess-own");
        index(&lab);
        assert_eq!(linking::tick(&lab.state), 0, "{tool}");
        assert_eq!(conversation_of(&lab, &session), None, "{tool}");
    }
}

#[test]
fn a_session_that_has_ended_is_left_alone() {
    let lab = lab(KIMI_CODE_ID);
    let session = started(&lab, "sess-own");
    {
        let storage = lab.state.storage.lock().unwrap();
        let mut record = storage.sessions().get(&session).unwrap();
        record.status = SessionStatus::Finished;
        storage.sessions().update(&record).unwrap();
    }
    tools_record(&lab);
    index(&lab);
    assert_eq!(linking::tick(&lab.state), 0);
    assert_eq!(conversation_of(&lab, &session), None);
}

#[test]
fn in_place_without_a_process_in_its_records_keeps_both_rows() {
    // OpenCode and Grok Build record no process per session: on an account
    // the user's own terminal shares, the conversation could be theirs.
    for tool in [OPENCODE_ID, GROK_BUILD_ID] {
        let lab = lab(tool);
        in_place(&lab);
        let session = started(&lab, "sess-own");
        let id = tools_record(&lab);
        index(&lab);
        assert_eq!(linking::tick(&lab.state), 0, "{tool}: a guess");
        assert_eq!(conversation_of(&lab, &session), None, "{tool}");
        let row = juggler_app::features::conversations::commands::list_impl(&lab.state)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("{tool}: the conversation isn't listed"));
        assert_eq!(row.session_id, None, "{tool}: its own row, as before");
    }
}

#[test]
fn in_place_with_the_process_in_its_records_still_links_by_launch() {
    // Kimi Code's session locks name the process: a session the user ran
    // by hand would be open in a process of theirs, which refuses the link.
    let lab = lab(KIMI_CODE_ID);
    in_place(&lab);
    let session = started(&lab, "sess-own");
    let id = tools_record(&lab);
    index(&lab);
    assert_eq!(linking::tick(&lab.state), 1);
    assert_eq!(
        conversation_of(&lab, &session).as_deref(),
        Some(id.as_str())
    );
}

#[test]
fn which_tools_records_name_a_sessions_process() {
    use juggler_adapter::{Capabilities, HarnessAdapter};
    let recorded = |adapter: &dyn HarnessAdapter| {
        adapter
            .capabilities()
            .contains(Capabilities::SESSION_PROCESS_RECORDED)
    };
    assert!(!recorded(&OpenCodeAdapter::with_bin("/usr/bin/true")));
    assert!(!recorded(&GrokBuildAdapter::with_bin("/usr/bin/true")));
    assert!(recorded(&KimiCodeAdapter::with_bin("/usr/bin/true")));
    // A managed account (only AI Juggler uses it) is linked by launch for
    // every tool: `a_hookless_tools_session_is_one_rail_row_carrying_its_count`.
}
