//! AJ-E412 app-level: running-model detection over real transcript
//! files — observation, classification (`transcript` vs `in-session`),
//! incremental tail scans, and the read-model wire classification.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId, ModelSource,
    ProfileId, RunningModel, SessionId, SessionStatus, SettingsOverlay, Timestamp, TranscriptRef,
};

fn seeded_state(dir: &Path, launch_model: Option<&str>) -> Arc<AppState> {
    let state = Arc::new(
        AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-model"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Work Max".into(),
                plan: None,
                state_dir: dir.join("state"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-model"),
                name: "Main".into(),
                account: AccountId::new("acc-model"),
                settings_overlay: SettingsOverlay {
                    model: launch_model.map(str::to_string),
                    env: Default::default(),
                },
                default_cwd: None,
            })
            .unwrap();
    }
    state
}

fn seed_instance(state: &Arc<AppState>, dir: &Path, id: &str, transcript: &Path) -> SessionId {
    let session = SessionId::new(id);
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: ProfileId::new("rec-model"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            transcript_ref: Some(TranscriptRef {
                native_id: Some("native-1".into()),
                path: transcript.to_path_buf(),
            }),
            attention: None,
            running: None,
            window: None,
        })
        .unwrap();
    session
}

fn transcript_with_model(model: Option<&str>) -> String {
    match model {
        Some(model) => {
            format!(
                "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"model\":\"{model}\"}}}}\n"
            )
        }
        None => "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\"}}\n".to_string(),
    }
}

#[test]
fn running_model_observed_classified_and_incremental() {
    let dir = tempfile::tempdir().unwrap();
    // Launch overlay says opus-4.8.
    let state_ref = &seeded_state(dir.path(), Some("opus-4.8"));
    let transcript = dir.path().join("t.jsonl");
    std::fs::write(&transcript, transcript_with_model(Some("opus-4.8"))).unwrap();
    let session = seed_instance(state_ref, dir.path(), "sess-model", &transcript);

    // First scan: observed opus == launch → source `transcript`.
    assert!(commands::detect_running_models_core(state_ref, None));
    {
        let storage = state_ref.storage.lock().unwrap();
        assert_eq!(
            storage.sessions().get(&session).unwrap().running,
            Some(RunningModel {
                model: "opus-4.8".into(),
                source: ModelSource::Transcript,
            })
        );
    }

    // No new bytes: no change, no write.
    assert!(!commands::detect_running_models_core(state_ref, None));

    // Append an in-session /model change → source `in-session`.
    std::fs::write(
        &transcript,
        std::fs::read_to_string(&transcript).unwrap() + &transcript_with_model(Some("sonnet-5")),
    )
    .unwrap();
    assert!(commands::detect_running_models_core(state_ref, None));
    {
        let storage = state_ref.storage.lock().unwrap();
        assert_eq!(
            storage.sessions().get(&session).unwrap().running,
            Some(RunningModel {
                model: "sonnet-5".into(),
                source: ModelSource::InSession,
            })
        );
    }

    // Read-model classification: launch vs running vs source.
    let infos = {
        let storage = state_ref.storage.lock().unwrap();
        juggler_app::commands::session_infos_snapshot(&storage).unwrap()
    };
    let row = infos.iter().find(|info| info.id == "sess-model").unwrap();
    assert_eq!(row.launch_model.as_deref(), Some("opus-4.8"));
    assert_eq!(row.running_model.as_deref(), Some("sonnet-5"));
    assert_eq!(row.model.as_deref(), Some("sonnet-5")); // display prefers the observation
    assert_eq!(row.model_source, "in-session");
}

#[test]
fn launch_only_never_poses_as_running() {
    let dir = tempfile::tempdir().unwrap();
    let state_ref = &seeded_state(dir.path(), Some("opus-4.8"));
    let transcript = dir.path().join("t.jsonl");
    // Transcript exists but records NO model (the no-model Provider shape).
    std::fs::write(
        &transcript,
        transcript_with_model(None) + &transcript_with_model(None),
    )
    .unwrap();
    let session = seed_instance(state_ref, dir.path(), "sess-launch-only", &transcript);

    assert!(!commands::detect_running_models_core(state_ref, None));
    {
        let storage = state_ref.storage.lock().unwrap();
        assert_eq!(storage.sessions().get(&session).unwrap().running, None);
    }
    let infos = {
        let storage = state_ref.storage.lock().unwrap();
        juggler_app::commands::session_infos_snapshot(&storage).unwrap()
    };
    let row = infos
        .iter()
        .find(|info| info.id == "sess-launch-only")
        .unwrap();
    assert_eq!(row.running_model, None);
    assert_eq!(row.model.as_deref(), Some("opus-4.8")); // launch intent displayed
    assert_eq!(row.model_source, "launch"); // not verified live
}

#[test]
fn no_launch_and_no_observation_is_unknown_default() {
    let dir = tempfile::tempdir().unwrap();
    let state_ref = &seeded_state(dir.path(), None);
    let transcript = dir.path().join("t.jsonl");
    std::fs::write(&transcript, transcript_with_model(None)).unwrap();
    seed_instance(state_ref, dir.path(), "sess-unknown", &transcript);

    let infos = {
        let storage = state_ref.storage.lock().unwrap();
        juggler_app::commands::session_infos_snapshot(&storage).unwrap()
    };
    let row = infos.iter().find(|info| info.id == "sess-unknown").unwrap();
    assert_eq!(row.running_model, None);
    assert_eq!(row.launch_model, None);
    assert_eq!(row.model, None); // UI renders `default model`
    assert_eq!(row.model_source, "unknown");
}
