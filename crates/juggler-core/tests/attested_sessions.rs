//! AJ-E3013 (ADR-0020's D-116 addendum): an attested desktop session's
//! facts reach the pool lookups through `pool_facts_of`, only while
//! attested, and its minted id can never be a session record's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "test crates are pragmatic by policy (QUALITY.md §2: tests excepted)"
)]

use std::path::PathBuf;

use juggler_core::model::{
    InstanceKind, InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
};
use juggler_core::pools::SessionFacts;
use juggler_core::storage::Storage;
use juggler_core::storage::attested::{DESKTOP_PREFIX, desktop_id, is_desktop_id};

fn desktop_facts(conversation: &str) -> SessionFacts {
    SessionFacts {
        app: Some("app:claude-app:/p".into()),
        tool: Some("claude-code".into()),
        conversation: Some(("folder-1".into(), conversation.into())),
        ..SessionFacts::bare(desktop_id("folder-1", conversation), None)
    }
}

#[test]
fn attested_facts_are_read_for_their_id_only_while_kept() {
    let storage = Storage::open_in_memory().unwrap();
    let id = desktop_id("folder-1", "conv-1");
    assert_ne!(
        id,
        desktop_id("folder-2", "conv-1"),
        "L3: the folder is part of it"
    );
    assert!(is_desktop_id(id.as_str()));
    assert_eq!(
        storage.sessions().pool_facts_of(&id).unwrap(),
        None,
        "never attested: in no pool"
    );
    storage.attested().put(desktop_facts("conv-1"));
    let facts = storage.sessions().pool_facts_of(&id).unwrap().unwrap();
    assert_eq!(
        facts.conversation,
        Some(("folder-1".into(), "conv-1".into()))
    );
    assert!(
        storage.sessions().pool_facts().unwrap().is_empty(),
        "every session's list is unchanged"
    );
    storage.attested().forget(&id);
    assert_eq!(storage.sessions().pool_facts_of(&id).unwrap(), None);
    // A real session's id never reads the attested map.
    storage
        .attested()
        .put(SessionFacts::bare(SessionId::new("sess-x"), None));
    assert_eq!(
        storage
            .sessions()
            .pool_facts_of(&SessionId::new("sess-x"))
            .unwrap(),
        None
    );
}

#[test]
fn a_desktop_id_can_never_be_a_session_record() {
    let storage = Storage::open_in_memory().unwrap();
    let record = InstanceRecord {
        id: SessionId::new(format!("{DESKTOP_PREFIX}conv-2")),
        conversation: None,
        profile: ProfileId::new("p"),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: PathBuf::from("/tmp/x"),
        status: SessionStatus::Running,
        started_at: Timestamp::from_millis(1),
        last_activity: None,
        usage: Default::default(),
        transcript_ref: None,
        attention: None,
        running: None,
        window: None,
    };
    let refused = storage.sessions().insert(&record).unwrap_err();
    assert!(refused.to_string().contains("session id"), "{refused}");
}
