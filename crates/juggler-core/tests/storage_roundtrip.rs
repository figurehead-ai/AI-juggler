//! Round-trip property tests and edge cases for the storage layer (M0-3).

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::PathBuf;

use juggler_core::model::{
    Account, AccountId, Confidence, HarnessId, LaunchRecipe, LaunchRecipeId, MachineId, PlanTier,
    Profile, ProfileId, QuotaWindow, SessionId, SessionRecord, SessionStatus, SettingsOverlay,
    Timestamp, TokenCounts, TranscriptRef, UsageSnapshot,
};
use juggler_core::storage::Storage;
use juggler_core::{CoreError, fs};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// strategies

fn id_str() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,9}"
}

fn label_str() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ._-]{1,12}"
}

fn path_str() -> impl Strategy<Value = String> {
    ("[a-z]{1,6}").prop_map(|segment| format!("/tmp/ai-juggler/{segment}"))
}

fn arb_account() -> impl Strategy<Value = Account> {
    (
        id_str(),
        id_str(),
        label_str(),
        proptest::option::of(label_str()),
        path_str(),
    )
        .prop_map(|(id, harness, label, plan, state_dir)| Account {
            id: AccountId::new(id),
            harness: HarnessId::new(harness),
            label,
            plan: plan.map(PlanTier::new),
            state_dir: PathBuf::from(state_dir),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
        })
}

fn arb_overlay() -> impl Strategy<Value = SettingsOverlay> {
    (
        proptest::option::of(id_str()),
        proptest::collection::btree_map("[a-z]{1,5}", "[a-z0-9]{1,5}", 0..4),
    )
        .prop_map(|(model, env)| SettingsOverlay {
            model: model.map(|m| format!("model-{m}")),
            env,
        })
}

/// Epoch-millis range: wide but bounded, exercising sign handling.
fn millis_range() -> std::ops::Range<i64> {
    -(10_i64.pow(12))..10_i64.pow(12)
}

fn arb_usage() -> impl Strategy<Value = UsageSnapshot> {
    let tokens = (
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
    )
        .prop_map(
            |(input, output, cache_read, cache_write, reasoning)| TokenCounts {
                input,
                output,
                cache_read,
                cache_write,
                reasoning,
            },
        );
    let window =
        (0.0_f64..100.0, proptest::option::of(millis_range())).prop_map(|(used, resets)| {
            QuotaWindow {
                used_percentage: used,
                resets_at: resets.map(Timestamp::from_millis),
            }
        });
    (tokens, proptest::option::of(window), 0_usize..3).prop_map(|(tokens, window, c)| {
        let confidence = match c {
            0 => Confidence::Measured,
            1 => Confidence::Computed,
            _ => Confidence::Estimated,
        };
        UsageSnapshot {
            tokens,
            window,
            confidence,
        }
    })
}

fn arb_session(profile: ProfileId) -> impl Strategy<Value = SessionRecord> {
    (
        id_str(),
        proptest::option::of(id_str()),
        id_str(),
        path_str(),
        0_usize..8,
        millis_range(),
        proptest::option::of(millis_range()),
        arb_usage(),
        proptest::option::of((path_str(), proptest::option::of(id_str()))),
    )
        .prop_map(
            move |(id, native, machine, cwd, status, started, activity, usage, tr)| SessionRecord {
                id: SessionId::new(id),
                conversation: native.map(juggler_core::model::ConversationId::new),
                profile: profile.clone(),
                machine: MachineId::new(machine),
                kind: juggler_core::model::InstanceKind::Terminal,
                project: None,
                cwd: PathBuf::from(cwd),
                status: SessionStatus::all()[status],
                started_at: Timestamp::from_millis(started),
                last_activity: activity.map(Timestamp::from_millis),
                usage,
                transcript_ref: tr.map(|(path, native)| TranscriptRef {
                    path: PathBuf::from(path),
                    native_id: native,
                }),
                attention: None,
                running: None,
                window: None,
            },
        )
}

/// Field-wise session equality. The single float field
/// (`QuotaWindow::used_percentage`) is compared with a 1e-9 tolerance: the
/// ecosystem's default JSON float parser is ulp-lossy for rare f64 values,
/// and sub-nanopercent differences are meaningless for quota display.
fn sessions_equivalent(a: &SessionRecord, b: &SessionRecord) -> bool {
    match (&a.usage.window, &b.usage.window) {
        (None, None) => {}
        (Some(wa), Some(wb)) => {
            let windows_ok = wa.resets_at == wb.resets_at
                && (wa.used_percentage - wb.used_percentage).abs() < 1e-9;
            if !windows_ok {
                return false;
            }
        }
        _ => return false,
    }
    a.id == b.id
        && a.conversation == b.conversation
        && a.profile == b.profile
        && a.machine == b.machine
        && a.cwd == b.cwd
        && a.status == b.status
        && a.started_at == b.started_at
        && a.last_activity == b.last_activity
        && a.usage.tokens == b.usage.tokens
        && a.usage.confidence == b.usage.confidence
        && a.transcript_ref == b.transcript_ref
}

/// A fixed parent chain so property tests only exercise the entity under test.
fn seed_chain(storage: &Storage) -> (Account, Profile) {
    let account = Account {
        id: AccountId::new("seed-account"),
        harness: HarnessId::new("claude-code"),
        label: "Seed".into(),
        plan: Some(PlanTier::new("max-20x")),
        state_dir: PathBuf::from("/tmp/seed"),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
    };
    let profile = Profile {
        id: ProfileId::new("seed-profile"),
        account: account.id.clone(),
        name: "Seed profile".into(),
        settings_overlay: SettingsOverlay::default(),
        default_cwd: None,
    };
    storage.accounts().insert(&account).unwrap();
    storage.profiles().insert(&profile).unwrap();
    (account, profile)
}

// ---------------------------------------------------------------------------
// property round-trips

proptest! {
    #[test]
    fn account_roundtrip(account in arb_account()) {
        let storage = Storage::open_in_memory().unwrap();
        storage.accounts().insert(&account).unwrap();
        prop_assert_eq!(storage.accounts().get(&account.id).unwrap(), account);
    }

    #[test]
    fn profile_roundtrip(overlay in arb_overlay(), id in id_str()) {
        let storage = Storage::open_in_memory().unwrap();
        let (account, _) = seed_chain(&storage);
        let profile = Profile { id: ProfileId::new(id.clone()), name: format!("p-{id}"), account: account.id, settings_overlay: overlay, default_cwd: None };
        storage.profiles().insert(&profile).unwrap();
        prop_assert_eq!(storage.profiles().get(&profile.id).unwrap(), profile);
    }

    #[test]
    fn session_roundtrip(session in arb_session(ProfileId::new("seed-profile"))) {
        let storage = Storage::open_in_memory().unwrap();
        seed_chain(&storage);
        storage.sessions().insert(&session).unwrap();
        prop_assert!(sessions_equivalent(
            &storage.sessions().get(&session.id).unwrap(),
            &session
        ));
        let listed = storage.sessions().list(Some(&session.profile)).unwrap();
        prop_assert!(listed.iter().any(|s| s.id == session.id));
    }

    #[test]
    fn session_update_roundtrip(mut session in arb_session(ProfileId::new("seed-profile"))) {
        let storage = Storage::open_in_memory().unwrap();
        seed_chain(&storage);
        storage.sessions().insert(&session).unwrap();
        session.status = SessionStatus::QuotaBlocked;
        session.conversation = Some(juggler_core::model::ConversationId::new("native-1"));
        storage.sessions().update(&session).unwrap();
        prop_assert_eq!(storage.sessions().get(&session.id).unwrap().status, SessionStatus::QuotaBlocked);
    }
}

// ---------------------------------------------------------------------------
// edge cases

#[test]
fn missing_entities_are_not_found_not_panics() {
    let storage = Storage::open_in_memory().unwrap();
    let missing = AccountId::new("ghost");
    assert!(matches!(
        storage.accounts().get(&missing),
        Err(CoreError::NotFound { .. })
    ));
    assert!(matches!(
        storage.accounts().delete(&missing),
        Err(CoreError::NotFound { .. })
    ));
}

#[test]
fn deleting_account_cascades() {
    let storage = Storage::open_in_memory().unwrap();
    let (account, profile) = seed_chain(&storage);
    let session = SessionRecord {
        id: SessionId::new("s1"),
        conversation: None,
        profile: profile.id,
        machine: MachineId::new("this-mac"),
        kind: juggler_core::model::InstanceKind::Terminal,
        project: None,
        cwd: PathBuf::from("/tmp"),
        status: SessionStatus::Running,
        started_at: Timestamp::from_millis(1),
        last_activity: None,
        usage: UsageSnapshot::default(),
        attention: None,
        running: None,
        window: None,
        transcript_ref: None,
    };
    storage.sessions().insert(&session).unwrap();
    storage.accounts().delete(&account.id).unwrap();
    assert!(storage.sessions().list(None).unwrap().is_empty());
    assert!(
        storage
            .profiles()
            .list_by_account(&account.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn reopen_is_idempotent_and_future_schema_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");

    let storage = Storage::open(&db).unwrap();
    drop(storage);
    // Reopening an already-migrated database applies nothing and succeeds.
    Storage::open(&db).unwrap();

    // Simulate a database written by a newer build.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.pragma_update(None, "user_version", 9_999).unwrap();
    drop(conn);
    match Storage::open(&db) {
        Err(CoreError::SchemaTooNew {
            found,
            supported: _,
        }) => assert_eq!(found, 9_999),
        other => panic!("expected SchemaTooNew, got {other:?}"),
    }
}

#[test]
fn corrupt_file_degrades_to_not_a_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("garbage.sqlite");
    std::fs::write(&db, b"this is definitely not sqlite").unwrap();
    match Storage::open(&db) {
        Err(CoreError::NotADatabase { .. }) => {}
        other => panic!("expected NotADatabase, got {other:?}"),
    }
}

#[test]
fn atomic_write_replaces_and_cleans_up() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.json");
    fs::atomic_write(&path, b"first").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"first");
    fs::atomic_write(&path, b"second").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second");
    // No temporary siblings left behind.
    let leftovers = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".ai-juggler-tmp-")
        })
        .count();
    assert_eq!(leftovers, 0);
}

#[test]
fn every_status_roundtrips_through_its_stored_name() {
    for status in SessionStatus::all() {
        assert_eq!(SessionStatus::parse(status.as_str()), Some(status));
    }
    assert_eq!(SessionStatus::parse("sentient"), None);
}

#[test]
fn account_update_changes_fields() {
    let storage = Storage::open_in_memory().unwrap();
    let (mut account, _) = seed_chain(&storage);
    account.label = "Renamed".into();
    account.plan = Some(PlanTier::new("max-5x"));
    account.state_dir = PathBuf::from("/tmp/elsewhere");
    storage.accounts().update(&account).unwrap();
    let reloaded = storage.accounts().get(&account.id).unwrap();
    assert_eq!(reloaded, account);
    assert_eq!(storage.accounts().list().unwrap().len(), 1);
}

#[test]
fn profile_update_and_delete() {
    let storage = Storage::open_in_memory().unwrap();
    let (account, _) = seed_chain(&storage);
    let mut profile = Profile {
        id: ProfileId::new("p-upd"),
        name: "updatable".into(),
        account: account.id.clone(),
        settings_overlay: SettingsOverlay::default(),
        default_cwd: None,
    };
    storage.profiles().insert(&profile).unwrap();
    profile.settings_overlay = SettingsOverlay {
        model: Some("m2".into()),
        env: BTreeMap::from([("A".into(), "B".into())]),
    };
    profile.default_cwd = Some(PathBuf::from("/tmp/x"));
    storage.profiles().update(&profile).unwrap();
    assert_eq!(storage.profiles().get(&profile.id).unwrap(), profile);
    assert_eq!(
        storage
            .profiles()
            .list_by_account(&account.id)
            .unwrap()
            .len(),
        2
    );

    storage.profiles().delete(&profile.id).unwrap();
    assert!(matches!(
        storage.profiles().get(&profile.id),
        Err(CoreError::NotFound { .. })
    ));
}

#[test]
fn session_list_without_filter_returns_all() {
    let storage = Storage::open_in_memory().unwrap();
    let (_, profile) = seed_chain(&storage);
    let session = SessionRecord {
        id: SessionId::new("s-all"),
        conversation: None,
        profile: profile.id,
        machine: MachineId::new("this-mac"),
        kind: juggler_core::model::InstanceKind::Terminal,
        project: None,
        cwd: PathBuf::from("/tmp"),
        status: SessionStatus::Running,
        started_at: Timestamp::from_millis(7),
        last_activity: Some(Timestamp::from_millis(9)),
        usage: UsageSnapshot::default(),
        attention: None,
        running: None,
        window: None,
        transcript_ref: None,
    };
    storage.sessions().insert(&session).unwrap();
    let all = storage.sessions().list(None).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, session.id);
}

#[test]
fn projects_touch_creates_then_refreshes_activity() {
    let storage = Storage::open_in_memory().unwrap();
    let root = std::path::Path::new("/Users/dev/myapp");

    let first = storage.projects().touch(root, None).unwrap();
    assert_eq!(first.name, "myapp", "name defaults to the directory name");
    assert_eq!(first.root, root);
    assert!(first.last_activity.is_some());

    // Second touch refreshes activity, keeps identity and name.
    std::thread::sleep(std::time::Duration::from_millis(5));
    let second = storage.projects().touch(root, Some("Custom name")).unwrap();
    assert_eq!(second.id, first.id, "same project — keyed by root");
    assert_eq!(second.name, "myapp", "touch never renames");
    assert!(second.last_activity.unwrap() >= first.last_activity.unwrap());

    // Listing returns most-recently-active first.
    let other = storage
        .projects()
        .touch(std::path::Path::new("/Users/dev/other"), None)
        .unwrap();
    let listed = storage.projects().list().unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, other.id, "most recent activity first");

    assert!(storage.projects().get(&first.id).is_ok());
}

#[test]
fn sessions_carry_instance_kind_and_project_link() {
    let storage = Storage::open_in_memory().unwrap();
    let (_account, profile) = seed_chain(&storage);

    let project = storage
        .projects()
        .touch(std::path::Path::new("/Users/dev/myapp"), None)
        .unwrap();
    let mut session = SessionRecord {
        id: SessionId::new("sess-kind"),
        conversation: None,
        profile: profile.id,
        machine: MachineId::new("this-mac"),
        kind: juggler_core::model::InstanceKind::Desktop,
        project: Some(project.id.clone()),
        cwd: PathBuf::from("/Users/dev/myapp"),
        status: SessionStatus::Running,
        started_at: Timestamp::from_millis(1),
        last_activity: None,
        usage: UsageSnapshot::default(),
        attention: None,
        running: None,
        window: None,
        transcript_ref: None,
    };
    storage.sessions().insert(&session).unwrap();
    session.project = Some(project.id.clone());
    let loaded = storage.sessions().get(&session.id).unwrap();
    assert_eq!(loaded.kind, juggler_core::model::InstanceKind::Desktop);
    assert_eq!(loaded.project.as_ref(), Some(&project.id));

    // Update can re-point the project link (backfill path), not the kind.
    let mut updated = loaded.clone();
    updated.project = None;
    storage.sessions().update(&updated).unwrap();
    assert_eq!(storage.sessions().get(&session.id).unwrap().project, None);
}

#[test]
fn v4_database_upgrades_in_place_to_v5_with_identical_values() {
    // AJ-E106: a genuine v4 database (old table/column names, real
    // values) auto-migrates on Storage::open; user_version lands on 5
    // exactly once; every stored value is byte-identical after.
    use juggler_core::model::InstanceKind;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("v4.sqlite");
    {
        let conn = Connection::open(&file).unwrap();
        let migrations = Storage::migrations();
        conn.execute_batch(&migrations[..4].join(";")).unwrap();
        conn.pragma_update(None, "user_version", 4).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, harness, label, plan, state_dir, created_at)
             VALUES ('acc-1', 'claude-code', 'Work Max', 'max-20x', '/tmp/acc1', 111)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO profiles (id, account_id, name, settings_json, default_cwd, created_at)
             VALUES ('rec-1', 'acc-1', 'Main', '{}', NULL, 111)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (id, native_id, profile_id, machine, kind, project, cwd,
                                   status, started_at, last_activity, usage_json, transcript_ref_json)
             VALUES ('sess-1', 'native-1', 'rec-1', 'this-mac', 'terminal', NULL, '/tmp/fx',
                     'running', 1234567890, NULL,
                     ?, NULL)",
            [serde_json::to_string(&UsageSnapshot::default()).unwrap().as_str()],
        )
        .unwrap();
    }

    let storage = Storage::open(&file).unwrap(); // auto-upgrade path
    let version: i64 = storage
        .raw_conn()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        version, 21,
        "v4 → current (v21, AJ-E307 import provenance) exactly once, on open"
    );

    let account = storage.accounts().get(&AccountId::new("acc-1")).unwrap();
    assert_eq!(account.harness.as_str(), "claude-code");
    assert_eq!(account.label, "Work Max");
    assert_eq!(account.plan.as_ref().map(|p| p.as_str()), Some("max-20x"));

    let recipe = storage.profiles().get(&ProfileId::new("rec-1")).unwrap();
    assert_eq!(recipe.name, "Main");

    let record = storage.sessions().get(&SessionId::new("sess-1")).unwrap();
    assert_eq!(record.attention, None, "v4 rows upgrade without attention");
    assert_eq!(
        record.conversation.as_ref().map(|c| c.as_str()),
        Some("native-1")
    );
    assert_eq!(record.kind, InstanceKind::Terminal);
    assert_eq!(record.status, SessionStatus::Running);

    // Reopening a current-DB does not re-migrate or corrupt.
    drop(storage);
    let storage = Storage::open(&file).unwrap();
    let version: i64 = storage
        .raw_conn()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 21);
    assert!(storage.sessions().get(&SessionId::new("sess-1")).is_ok());
}

#[test]
fn launch_recipe_alias_roundtrip_pre_rename_values() {
    // AJ-E105 evidence: values written through the old `Profile` alias
    // read identically through canonical `LaunchRecipe` — zero drift.
    let storage = Storage::open_in_memory().unwrap();
    let (account, _) = seed_chain(&storage);
    let written = LaunchRecipe {
        id: LaunchRecipeId::new("rec-1"),
        name: "Main".into(),
        account: account.id.clone(),
        settings_overlay: SettingsOverlay {
            model: Some("opus".into()),
            env: BTreeMap::new(),
        },
        default_cwd: None,
    };
    storage.profiles().insert(&written).unwrap();
    let read: LaunchRecipe = storage.profiles().get(&ProfileId::new("rec-1")).unwrap();
    assert_eq!(read, written);
    let as_alias: Profile = read;
    assert_eq!(as_alias.name, "Main");
}

#[test]
fn account_renewal_date_roundtrip_and_default_none() {
    use juggler_core::model::HarnessId;
    let storage = Storage::open_in_memory().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-renewal"),
            harness: HarnessId::new("claude-code"),
            label: "Work Max".into(),
            plan: None,
            state_dir: PathBuf::from("/tmp/renewal"),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
        })
        .unwrap();
    // A row stored WITHOUT the field loads as None — never a guess (D-12).
    assert_eq!(
        storage
            .accounts()
            .get(&AccountId::new("acc-renewal"))
            .unwrap()
            .renewal_date,
        None
    );

    let mut account = storage
        .accounts()
        .get(&AccountId::new("acc-renewal"))
        .unwrap();
    account.renewal_date = Some("2026-11-02".into());
    storage.accounts().update(&account).unwrap();
    assert_eq!(
        storage
            .accounts()
            .get(&AccountId::new("acc-renewal"))
            .unwrap()
            .renewal_date,
        Some("2026-11-02".into())
    );

    account.renewal_date = None; // Clear
    storage.accounts().update(&account).unwrap();
    assert_eq!(
        storage
            .accounts()
            .get(&AccountId::new("acc-renewal"))
            .unwrap()
            .renewal_date,
        None
    );
}

#[test]
fn session_running_model_roundtrip_and_default_none() {
    use juggler_core::model::{
        InstanceKind, InstanceRecord, MachineId, ModelSource, ProfileId, RunningModel,
        SessionStatus, Timestamp,
    };
    let storage = Storage::open_in_memory().unwrap();
    seed_chain(&storage);
    let mut session = InstanceRecord {
        id: SessionId::new("sess-running"),
        conversation: None,
        profile: ProfileId::new("seed-profile"),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: PathBuf::from("/tmp/running"),
        status: SessionStatus::Running,
        started_at: Timestamp::from_millis(1),
        last_activity: None,
        usage: Default::default(),
        transcript_ref: None,
        attention: None,
        running: None,
        window: None,
    };
    storage.sessions().insert(&session).unwrap();
    // Stored without the field: loads as None (never the launch overlay).
    assert_eq!(storage.sessions().get(&session.id).unwrap().running, None);

    session.running = Some(RunningModel {
        model: "sonnet-5".into(),
        source: ModelSource::InSession,
    });
    storage.sessions().update(&session).unwrap();
    assert_eq!(
        storage.sessions().get(&session.id).unwrap().running,
        Some(RunningModel {
            model: "sonnet-5".into(),
            source: ModelSource::InSession,
        })
    );

    session.running = None;
    storage.sessions().update(&session).unwrap();
    assert_eq!(storage.sessions().get(&session.id).unwrap().running, None);
}

/// AJ-E414 (schema v10): the window identity blob round-trips, defaults
/// to None, and a malformed blob degrades to None — never a
/// row-breaking value, never a guessed identity.
#[test]
fn session_window_identity_roundtrip_and_degrade() {
    use juggler_core::model::{
        InstanceKind, InstanceRecord, MachineId, ProfileId, SessionStatus, Timestamp,
        WindowIdentity,
    };
    let storage = Storage::open_in_memory().unwrap();
    seed_chain(&storage);
    let mut session = InstanceRecord {
        id: SessionId::new("sess-window"),
        conversation: None,
        profile: ProfileId::new("seed-profile"),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Desktop,
        project: None,
        cwd: PathBuf::from("/tmp/window"),
        status: SessionStatus::Running,
        started_at: Timestamp::from_millis(1),
        last_activity: None,
        usage: Default::default(),
        transcript_ref: None,
        attention: None,
        running: None,
        window: None,
    };
    storage.sessions().insert(&session).unwrap();
    // Terminal-era rows (no field): None, never fabricated.
    assert_eq!(storage.sessions().get(&session.id).unwrap().window, None);

    session.window = Some(WindowIdentity {
        pid: 29340,
        bundle_id: Some("com.apple.TextEdit".into()),
        title: Some("aij-e414-a.txt".into()),
        cgwindow_id: None,
    });
    storage.sessions().update(&session).unwrap();
    assert_eq!(
        storage.sessions().get(&session.id).unwrap().window,
        Some(WindowIdentity {
            pid: 29340,
            bundle_id: Some("com.apple.TextEdit".into()),
            title: Some("aij-e414-a.txt".into()),
            cgwindow_id: None,
        })
    );

    // Malformed blob: degrade to None, row still loads.
    storage
        .raw_conn()
        .execute(
            "UPDATE instances SET window_identity_json = '{not-json' WHERE id = 'sess-window'",
            [],
        )
        .unwrap();
    assert_eq!(storage.sessions().get(&session.id).unwrap().window, None);
}

/// AJ-E414 (schema v10): the app-settings kv — get/set/upsert, empty
/// value deletes (reset is a real state), list is ordered.
#[test]
fn app_settings_kv_roundtrip() {
    let storage = Storage::open_in_memory().unwrap();
    assert_eq!(
        storage.app_settings().get("go_to_hotkey").unwrap(),
        None,
        "unset keys read as None — the caller applies its default"
    );
    storage
        .app_settings()
        .set("go_to_hotkey", "Alt+Space")
        .unwrap();
    assert_eq!(
        storage.app_settings().get("go_to_hotkey").unwrap(),
        Some("Alt+Space".to_string())
    );
    // Upsert, not duplicate.
    storage
        .app_settings()
        .set("go_to_hotkey", "CmdOrCtrl+G")
        .unwrap();
    assert_eq!(
        storage.app_settings().get("go_to_hotkey").unwrap(),
        Some("CmdOrCtrl+G".to_string())
    );
    // Empty value = reset (row deleted).
    storage.app_settings().set("go_to_hotkey", "").unwrap();
    assert_eq!(storage.app_settings().get("go_to_hotkey").unwrap(), None);
    assert!(storage.app_settings().list().unwrap().is_empty());
}

/// AJ-E415 (schema v11): the per-Account preview opt-in round-trips and
/// defaults to off (consent is never implicit).
#[test]
fn account_preview_opt_in_roundtrip_and_default_off() {
    let storage = Storage::open_in_memory().unwrap();
    seed_chain(&storage);
    let account_id = AccountId::new("seed-account");
    let mut account = storage.accounts().get(&account_id).unwrap();
    assert!(!account.preview_opt_in, "default off — consent is explicit");
    account.preview_opt_in = true;
    storage.accounts().update(&account).unwrap();
    assert!(storage.accounts().get(&account_id).unwrap().preview_opt_in);
    account.preview_opt_in = false;
    storage.accounts().update(&account).unwrap();
    assert!(!storage.accounts().get(&account_id).unwrap().preview_opt_in);
}

/// AJ-E501 (schema v12): the process ledger lifecycle — Intent row
/// BEFORE spawn (no PID), spawn patch (pid/pgid/start), batched helper
/// upserts (no duplicates, peaks refreshed), exit patch, candidate
/// query, and the 7-day prune that never touches running rows.
#[test]
fn process_ledger_lifecycle_roundtrip() {
    use juggler_core::storage::{LedgerRow, ProcessLedgerRepo, STATE_EXITED, STATE_RUNNING};

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("ledger.sqlite");
    let repo = ProcessLedgerRepo::open(&db).unwrap();

    let root = LedgerRow {
        pid: None,
        pgid: None,
        started_at: None,
        exe_path: "/bin/sh".into(),
        cmdline_hash: "hash-1".into(),
        cmdline_trunc: "sh -c claude".into(),
        boot_id: "boot-42".into(),
        session_id: "sess-ledger".into(),
        instance_id: None,
        provider: "claude-code".into(),
        account_id: Some("acc-1".into()),
        project_id: Some("proj-1".into()),
        machine_id: "machine-stable".into(),
        root_pid: None,
        parent_pid: None,
        depth: 0,
        role: juggler_core::storage::ROLE_ROOT.into(),
        state: String::new(),
        listens_on: None,
        cwd: "/tmp/x".into(),
        peak_rss: None,
        peak_cpu: None,
        exit_kind: None,
        reaped_by_us: None,
        escaped_group: false,
        down_period: false,
        first_seen: 1_000,
        last_seen: 1_000,
    };

    // 1. Intent BEFORE spawn: state=starting, no pid.
    repo.intent(&root).unwrap();
    let rows = repo.rows_for_session("sess-ledger").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, juggler_core::storage::STATE_STARTING);
    assert_eq!(rows[0].pid, None, "intent rows carry no pid");

    // 2. Spawn patch: pid + pgid + OS start time; state → running.
    repo.spawned("sess-ledger", 4242, Some(4242), Some(123_456), 1_050)
        .unwrap();
    let rows = repo.rows_for_session("sess-ledger").unwrap();
    assert_eq!(rows[0].pid, Some(4242));
    assert_eq!(
        rows[0].started_at,
        Some(123_456),
        "OS start time, not wall clock"
    );
    assert_eq!(rows[0].state, STATE_RUNNING);

    // 3. Helper upserts batched; repeats refresh, never duplicate.
    let helper = |pid: u32, rss: Option<i64>, escaped: bool| LedgerRow {
        pid: Some(pid),
        pgid: Some(4242),
        started_at: Some(123_456),
        exe_path: "node".into(),
        cmdline_hash: format!("hash-{pid}"),
        cmdline_trunc: "node vite".into(),
        boot_id: "boot-42".into(),
        session_id: "sess-ledger".into(),
        instance_id: None,
        provider: "claude-code".into(),
        account_id: Some("acc-1".into()),
        project_id: Some("proj-1".into()),
        machine_id: "machine-stable".into(),
        root_pid: Some(4242),
        parent_pid: Some(4242),
        depth: 1,
        role: String::new(),
        state: String::new(),
        listens_on: None,
        cwd: "/tmp/x".into(),
        peak_rss: rss,
        peak_cpu: None,
        exit_kind: None,
        reaped_by_us: None,
        escaped_group: escaped,
        down_period: false,
        first_seen: 2_000,
        last_seen: 2_000,
    };
    repo.upsert_helpers(
        "sess-ledger",
        &[helper(5001, Some(1_000), false), helper(5002, None, false)],
    )
    .unwrap();
    repo.upsert_helpers("sess-ledger", &[helper(5001, Some(2_000), false)])
        .unwrap();
    let rows = repo.rows_for_session("sess-ledger").unwrap();
    assert_eq!(rows.len(), 3, "root + two helpers, no duplicates");
    let h1 = rows.iter().find(|r| r.pid == Some(5001)).unwrap();
    assert_eq!(h1.role, juggler_core::storage::ROLE_HELPER);
    assert_eq!(h1.peak_rss, Some(2_000), "peaks take the max");
    assert_eq!(h1.depth, 1);

    // 3b. AJ-E502: a setsid'd helper lands flagged with its live pgid;
    // the flag is STICKY — a later tick computing false (probe raced
    // the exit) never revokes the no-containment verdict.
    let mut escapee = helper(5003, None, true);
    escapee.pgid = Some(5003);
    repo.upsert_helpers("sess-ledger", &[escapee]).unwrap();
    let esc = repo
        .rows_for_session("sess-ledger")
        .unwrap()
        .into_iter()
        .find(|r| r.pid == Some(5003))
        .unwrap();
    assert!(esc.escaped_group);
    assert_eq!(esc.pgid, Some(5003), "live pgid recorded on the row");
    repo.upsert_helpers("sess-ledger", &[helper(5003, None, false)])
        .unwrap();
    let esc = repo
        .rows_for_session("sess-ledger")
        .unwrap()
        .into_iter()
        .find(|r| r.pid == Some(5003))
        .unwrap();
    assert!(esc.escaped_group, "escape flag survives a clean re-tick");

    // 4. Exit patch (reaped root) + candidates exclude it.
    repo.mark_exited(4242, "boot-42", "code:0", true, 3_000)
        .unwrap();
    let candidates = repo.candidates("machine-stable").unwrap();
    assert!(candidates.iter().all(|r| r.pid != Some(4242)));
    let exited = repo
        .rows_for_session("sess-ledger")
        .unwrap()
        .into_iter()
        .find(|r| r.pid == Some(4242))
        .unwrap();
    assert_eq!(exited.state, STATE_EXITED);
    assert_eq!(exited.exit_kind.as_deref(), Some("code:0"));
    assert_eq!(exited.reaped_by_us, Some(true));

    // 5. Prune: exited-before-cutoff gone; running helpers untouched.
    let removed = repo.prune(3_500).unwrap();
    assert!(removed >= 1, "the exited root is older than the cutoff");
    let rows = repo.rows_for_session("sess-ledger").unwrap();
    assert!(rows.iter().all(|r| r.pid != Some(4242)));
    assert!(
        rows.iter()
            .any(|r| r.pid == Some(5001) && r.state == STATE_RUNNING)
    );

    // 5b. AJ-E503: identity failure flips a running row to the
    // PERMANENT unverifiable state — out of candidates, immune to
    // the prune (neither running nor exited).
    repo.mark_unverifiable(5001, "boot-42", 3_600).unwrap();
    let flipped = repo
        .rows_for_session("sess-ledger")
        .unwrap()
        .into_iter()
        .find(|r| r.pid == Some(5001))
        .unwrap();
    assert_eq!(flipped.state, juggler_core::storage::STATE_UNVERIFIABLE);
    let candidates = repo.candidates("machine-stable").unwrap();
    assert!(
        candidates.iter().all(|r| r.pid != Some(5001)),
        "unverifiable rows never re-enter the candidate set"
    );
    repo.prune(i64::MAX).unwrap();
    let flipped_still = repo
        .rows_for_session("sess-ledger")
        .unwrap()
        .into_iter()
        .any(|r| r.pid == Some(5001));
    assert!(flipped_still, "the prune never removes an unverifiable row");

    // 5c. AJ-E504: orphaned rows STAY candidates (re-verified each
    // tick) and resolve to `exited` when they later die.
    repo.mark_orphaned(5002, "boot-42", 3_700).unwrap();
    let orphaned = repo
        .rows_for_session("sess-ledger")
        .unwrap()
        .into_iter()
        .find(|r| r.pid == Some(5002))
        .unwrap();
    assert_eq!(orphaned.state, juggler_core::storage::STATE_ORPHANED);
    let candidates = repo.candidates("machine-stable").unwrap();
    assert!(
        candidates.iter().any(|r| r.pid == Some(5002)),
        "orphaned rows remain candidates (Running|Orphaned)"
    );
    assert!(
        candidates.iter().all(|r| r.pid != Some(5001)),
        "unverifiable rows still never return"
    );
    repo.mark_exited(5002, "boot-42", "unknown", false, 3_800)
        .unwrap();
    let resolved = repo
        .rows_for_session("sess-ledger")
        .unwrap()
        .into_iter()
        .find(|r| r.pid == Some(5002))
        .unwrap();
    assert_eq!(resolved.state, STATE_EXITED, "a dead orphan resolves");

    // 6. Persistence: reopen the same file — everything survives
    // (root exit, app exit, reboot are all just "reopen later").
    drop(repo);
    let repo = ProcessLedgerRepo::open(&db).unwrap();
    assert_eq!(repo.rows_for_session("sess-ledger").unwrap().len(), 3);
}

/// AJ-E502 (schema v13): a genuine v12 ledger file (pre-`escaped_group`)
/// gains the column on every open path — the repo's standalone init and
/// the main store's migration — idempotently, with existing rows
/// reading `escaped_group = false` (never claimed escaped, never
/// claimed contained).
#[test]
fn v12_ledger_upgrades_in_place_to_v13() {
    use juggler_core::storage::ProcessLedgerRepo;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("v12-ledger.sqlite");
    {
        let conn = Connection::open(&file).unwrap();
        let migrations = Storage::migrations();
        conn.execute_batch(&migrations[..12].join(";")).unwrap();
        conn.pragma_update(None, "user_version", 12).unwrap();
        conn.execute(
            "INSERT INTO process_ledger
                 (pid, pgid, started_at, exe_path, cmdline_hash, cmdline_trunc, boot_id,
                  session_id, instance_id, provider, account_id, project_id, machine_id,
                  root_pid, parent_pid, depth, role, state, listens_on, cwd,
                  peak_rss, peak_cpu, exit_kind, reaped_by_us, first_seen, last_seen)
             VALUES (9001, 9001, NULL, '/bin/sh', 'h', 'sh', 'boot-1',
                     'sess-old', NULL, 'claude-code', NULL, NULL, 'machine-stable',
                     NULL, NULL, 0, 'root', 'running', NULL, '/tmp',
                     NULL, NULL, NULL, NULL, 1, 1)",
            [],
        )
        .unwrap();
    }

    // Production ordering: the main store migrates (exactly once)…
    let storage = Storage::open(&file).unwrap();
    let version: i64 = storage
        .raw_conn()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        version, 21,
        "v12 → current (v21, AJ-E307) exactly once, on open"
    );
    drop(storage);

    // …then the ledger repo opens the same file (CREATE no-ops, the
    // migration's column is already there) and reads v12 rows as
    // not-escaped. Reopening is idempotent.
    let repo = ProcessLedgerRepo::open(&file).unwrap();
    let rows = repo.rows_for_session("sess-old").unwrap();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].escaped_group, "v12 rows default to not-escaped");
    assert!(
        !rows[0].down_period,
        "v12 rows default to not-left-behind (v16 column)"
    );
    drop(repo);
    let repo = ProcessLedgerRepo::open(&file).unwrap();
    assert_eq!(repo.rows_for_session("sess-old").unwrap().len(), 1);
}

#[test]
fn account_provenance_roundtrip_e307() {
    // AJ-E307 Req.1: provenance persists verbatim; absent = unknown
    // (never guessed), on both insert and update paths.
    let dir = tempfile::tempdir().unwrap();
    let storage = juggler_core::storage::Storage::open(&dir.path().join("s.db")).unwrap();
    let account = Account {
        id: AccountId::new("acc-prov"),
        harness: HarnessId::new("claude-code"),
        label: "Provenanced".into(),
        plan: None,
        state_dir: dir.path().join("managed"),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: Some("/Users/you/.cc-switch/profiles/work".into()),
        import_origin: Some("cc-switch".into()),
        imported_at: Some(1_728_800_000_000),
    };
    storage.accounts().insert(&account).unwrap();
    let loaded = storage.accounts().get(&AccountId::new("acc-prov")).unwrap();
    assert_eq!(
        loaded.import_source_path.as_deref(),
        Some("/Users/you/.cc-switch/profiles/work")
    );
    assert_eq!(loaded.import_origin.as_deref(), Some("cc-switch"));
    assert_eq!(loaded.imported_at, Some(1_728_800_000_000));
    // An update that leaves provenance untouched keeps it.
    storage
        .accounts()
        .update(&Account {
            label: "Renamed".into(),
            ..account
        })
        .unwrap();
    let reloaded = storage.accounts().get(&AccountId::new("acc-prov")).unwrap();
    assert_eq!(reloaded.label, "Renamed");
    assert_eq!(reloaded.import_origin.as_deref(), Some("cc-switch"));
}
