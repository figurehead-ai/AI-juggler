//! AJ-E2903 storage: what an Account's quality is rolled up from — each
//! session's figures kept at its end on its usage record (an older record
//! without them still reads), its stuck episodes and its last state change
//! from the recorded transitions.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_core::model::{
    Account, AccountId, EndQuality, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe,
    MachineId, ProfileId, SessionId, SessionStatus, Timestamp, UsageSnapshot,
};
use juggler_core::storage::Storage;

fn store() -> Storage {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc"),
            harness: HarnessId::new("claude-code"),
            label: "Work".into(),
            plan: None,
            state_dir: std::env::temp_dir().join("aij-quality-rollup"),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    storage
        .profiles()
        .insert(&LaunchRecipe {
            id: ProfileId::new("rec"),
            name: "Main".into(),
            account: AccountId::new("acc"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    storage
}

fn record(id: &str, status: SessionStatus) -> InstanceRecord {
    InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new("rec"),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: std::env::temp_dir(),
        status,
        started_at: Timestamp::now(),
        last_activity: None,
        usage: UsageSnapshot::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    }
}

#[test]
fn the_end_figures_are_kept_on_the_usage_record_and_old_records_still_read() {
    let storage = store();
    let mut done = record("done", SessionStatus::Finished);
    storage.sessions().insert(&done).unwrap();
    assert_eq!(storage.sessions().get(&done.id).unwrap().usage.at_end, None);
    done.usage.at_end = Some(EndQuality {
        errors: 2,
        turns: 41,
        context_used: Some(142_000),
        context_window: Some(200_000),
    });
    storage.sessions().update(&done).unwrap();
    assert_eq!(
        storage.sessions().get(&done.id).unwrap().usage.at_end,
        Some(EndQuality {
            errors: 2,
            turns: 41,
            context_used: Some(142_000),
            context_window: Some(200_000)
        })
    );
    // A record written before AJ-E2903 has no `at_end`: it still reads.
    let old: UsageSnapshot =
        serde_json::from_str(r#"{"tokens":{"input":1,"output":2,"cache_read":0,"cache_write":0,"reasoning":0},"confidence":"Estimated"}"#).unwrap();
    assert_eq!(old.at_end, None);
    // And a record without one writes none (older versions read it as before).
    assert!(
        !serde_json::to_string(&UsageSnapshot::default())
            .unwrap()
            .contains("at_end")
    );
}

#[test]
fn stuck_episodes_and_the_last_change_come_from_the_recorded_transitions() {
    let storage = store();
    let mut flappy = record("flappy", SessionStatus::Running);
    storage.sessions().insert(&flappy).unwrap();
    for status in [
        SessionStatus::Stuck,
        SessionStatus::Running,
        SessionStatus::Stuck,
        SessionStatus::Stopped,
    ] {
        flappy.status = status;
        storage.sessions().update(&flappy).unwrap();
    }
    storage
        .sessions()
        .insert(&record("calm", SessionStatus::Finished))
        .unwrap();
    let stuck = storage.sessions().stuck_episodes_since(0).unwrap();
    assert_eq!(stuck.get("flappy"), Some(&2));
    assert_eq!(stuck.get("calm"), None);
    // Nothing since the future.
    assert!(
        storage
            .sessions()
            .stuck_episodes_since(Timestamp::now().millis() + 60_000)
            .unwrap()
            .is_empty()
    );
    let last = storage.sessions().last_transitions().unwrap();
    let (from, to, _) = last.get("flappy").unwrap();
    assert_eq!((from.as_deref(), to.as_str()), (Some("stuck"), "stopped"));
    let (from, to, _) = last.get("calm").unwrap();
    assert_eq!((from.as_deref(), to.as_str()), (None, "finished"));
}
