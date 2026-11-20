//! SK-11 (the 0.4.6 soak, persona M4): after Continue in another app ›
//! Switch, the new session read "web · started 9:40 AM" and nothing on its
//! page showed the carried conversation. Its row now names the session it
//! continues (its page's door), and until its own copy is read it goes by
//! that conversation's title. A failed switch made nothing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "test code: a missing fixture fails the test loudly"
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::state::AppState;
use juggler_core::model::{
    AccountId, ConversationId, EdgeActor, EdgeState, Epoch, EpochId, HarnessId, InstanceKind,
    InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, SwitchEdge, SwitchEdgeId,
    SwitchMechanism, Timestamp,
};

const ASKED: &str = "Fix the login test";
const C_1: &str = "1c1c1c1c-0000-4000-8000-000000000001";

fn transcript(config: &Path, cwd: &Path, id: &str, ask: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"gitBranch\":\"main\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"{ask}\"}}}}\n",
            cwd.display()
        ),
    )
    .unwrap();
}

#[test]
fn a_switched_session_names_what_it_continues_and_goes_by_its_title() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let repo = root.join("Dev/web");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let config = root.join("work-setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    transcript(&config, &repo, C_1, ASKED);

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let from = juggler_app::features::account::import::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap()
    .account;
    let to =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Personal", None)
            .unwrap();
    let to_profile =
        juggler_app::features::profile::create_profile(&state, &to.id, "Main", None, None).unwrap();
    let (from_id, to_id) = (
        AccountId::new(from.id.as_str()),
        AccountId::new(to.id.as_str()),
    );
    // The conversations are read (the copy's isn't yet: it has no file).
    let _ = juggler_app::features::conversations::commands::list_impl(&state).unwrap();

    {
        let storage = state.storage.lock().unwrap();
        let from_profile = storage
            .profiles()
            .list_by_account(&from_id)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let project = storage.projects().touch(&repo, None).unwrap();
        let row = |id: &str, profile, conversation: &str, status| InstanceRecord {
            id: SessionId::new(id),
            conversation: Some(ConversationId::new(conversation)),
            profile,
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd: repo.clone(),
            status,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        };
        storage
            .sessions()
            .insert(&row(
                "sess-orig",
                from_profile.id.clone(),
                C_1,
                SessionStatus::HandedOver,
            ))
            .unwrap();
        for id in ["sess-copy", "sess-failed"] {
            storage
                .sessions()
                .insert(&row(
                    id,
                    ProfileId::new(to_profile.id.as_str()),
                    "c-copy",
                    SessionStatus::Running,
                ))
                .unwrap();
        }
        let graph = storage.switch_graph();
        let item = graph.create_work(&project.id, "web").unwrap();
        let epoch = |id: &str, session: &str, account: &AccountId, native: &str| Epoch {
            id: EpochId::new(id),
            work: item.id.clone(),
            session_ref: Some(SessionId::new(session)),
            native_id: Some(native.to_owned()),
            account: account.clone(),
            provider: HarnessId::new(CLAUDE_CODE_ID),
            resumed_from_epoch: None,
            machine: None,
        };
        graph
            .insert_epoch(&epoch("ep-a", "sess-orig", &from_id, C_1))
            .unwrap();
        for (edge, to_epoch, session) in [
            ("edge-ok", "ep-b", "sess-copy"),
            ("edge-bad", "ep-c", "sess-failed"),
        ] {
            graph
                .insert_epoch(&epoch(to_epoch, session, &to_id, "c-copy"))
                .unwrap();
            graph
                .append_edge(&SwitchEdge {
                    id: SwitchEdgeId::new(edge),
                    work: item.id.clone(),
                    from_epoch: EpochId::new("ep-a"),
                    to_epoch: None,
                    from_account: from_id.clone(),
                    to_account: to_id.clone(),
                    from_provider: HarnessId::new(CLAUDE_CODE_ID),
                    to_provider: HarnessId::new(CLAUDE_CODE_ID),
                    mechanism: SwitchMechanism::Transplant,
                    pack_id: None,
                    fidelity_shown: Some("F1".to_owned()),
                    manifest_sha256: None,
                    state: EdgeState::Staged,
                    actor: EdgeActor::User,
                    parent_edge_a: None,
                    parent_edge_b: None,
                    created_at: Timestamp::now(),
                    to_machine: None,
                })
                .unwrap();
            graph
                .mark_launched(&SwitchEdgeId::new(edge), &EpochId::new(to_epoch))
                .unwrap();
        }
        graph.mark_confirmed(&SwitchEdgeId::new("edge-ok")).unwrap();
        graph
            .mark_failed(
                &SwitchEdgeId::new("edge-bad"),
                juggler_core::model::FailureStage::Launch,
                "it didn't start",
            )
            .unwrap();
    }

    let rows = juggler_app::features::sessions::list::instances_shared_snapshot(&state).unwrap();
    let row = |id: &str| rows.iter().find(|row| row.id == id).unwrap();
    let copy = row("sess-copy");
    let continues = copy.continues.as_ref().expect("it continues the original");
    assert_eq!(continues.session_id.as_deref(), Some("sess-orig"));
    assert_eq!(continues.account_id, from.id);
    assert_eq!(continues.account_label, "Work");
    assert_eq!(continues.provider, CLAUDE_CODE_ID);
    assert_eq!(continues.conversation_id.as_deref(), Some(C_1));
    // Its copy isn't read yet: it goes by the conversation it continues.
    assert_eq!(copy.title.as_deref(), Some(ASKED));
    // The original is no continuation; a failed switch made nothing.
    assert_eq!(row("sess-orig").continues, None);
    assert_eq!(row("sess-orig").title.as_deref(), Some(ASKED));
    assert_eq!(row("sess-failed").continues, None);
    assert_eq!(row("sess-failed").title, None);
}
