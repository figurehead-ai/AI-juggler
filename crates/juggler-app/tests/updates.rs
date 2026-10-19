//! AJ-E1010 integration — the update policy against a fake feed: no
//! request in a build without the signing key or with the check turned
//! off; a check only reports (version + notes); a signature failure
//! never installs and leaves the running version untouched, with Retry;
//! a backup is written before any install; progress is reported at least
//! every 5 %; install waits while a switch is in flight; restart waits
//! while an Instance is live.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use juggler_adapter::AdapterRegistry;
use juggler_app::state::AppState;
use juggler_app::updates::{self, AnnouncedInfo, UpdateFeed, UpdateManager};

#[derive(Default)]
struct FakeFeed {
    checks: AtomicUsize,
    downloads: AtomicUsize,
    installs: AtomicUsize,
    bad_signature: bool,
}

impl UpdateFeed for FakeFeed {
    fn check(&self) -> Result<Option<AnnouncedInfo>, String> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        Ok(Some(AnnouncedInfo {
            version: "0.2.0".into(),
            notes: "Morning triage and History.\n\n## New\n- **History**: find any past session."
                .into(),
            date_ms: Some(1_800_000_000_000),
        }))
    }
    fn download(&self, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String> {
        self.downloads.fetch_add(1, Ordering::SeqCst);
        for _ in 0..1000 {
            progress(1_000, Some(1_000_000));
        }
        if self.bad_signature {
            Err("signature verification failed".into())
        } else {
            Ok(vec![0u8; 16])
        }
    }
    fn install(&self, _bytes: &[u8]) -> Result<(), String> {
        self.installs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn app_state(dir: &Path, configured: bool) -> Arc<AppState> {
    let mut state =
        AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap();
    state.updates = UpdateManager::for_test(configured);
    Arc::new(state)
}

#[test]
fn no_signing_key_or_check_off_means_no_request_at_all() {
    let work = tempfile::tempdir().unwrap();
    let feed = FakeFeed::default();
    let dev = app_state(&work.path().join("dev"), false);
    assert!(!updates::launch_check(&dev, &feed));
    assert!(updates::update_check_impl(&dev, &feed).is_err());
    assert_eq!(
        feed.checks.load(Ordering::SeqCst),
        0,
        "a development build never asks"
    );
    assert!(!updates::update_status_impl(&dev).configured);

    let release = app_state(&work.path().join("release"), true);
    assert!(
        updates::update_status_impl(&release).check_on_launch,
        "on by default"
    );
    updates::update_set_check_on_launch_impl(&release, false).unwrap();
    assert!(!updates::launch_check(&release, &feed));
    assert_eq!(
        feed.checks.load(Ordering::SeqCst),
        0,
        "turned off means no request"
    );
    updates::update_set_check_on_launch_impl(&release, true).unwrap();
    assert!(updates::launch_check(&release, &feed));
    assert_eq!(feed.checks.load(Ordering::SeqCst), 1);
    let status = updates::update_status_impl(&release);
    assert_eq!(status.state, "available");
    assert_eq!(status.channel, "beta");
    let available = status.available.unwrap();
    assert_eq!(available.version, "0.2.0");
    assert!(
        available.notes.contains("History"),
        "the changes are shown before installing"
    );
    assert_eq!(
        feed.installs.load(Ordering::SeqCst),
        0,
        "a check never installs"
    );
}

#[test]
fn a_bad_signature_never_installs_and_retry_works() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), true);
    let bad = FakeFeed {
        bad_signature: true,
        ..FakeFeed::default()
    };
    assert!(
        updates::update_install_impl(&state, &bad, &mut |_| {}).is_err(),
        "nothing to install before a check"
    );
    updates::update_check_impl(&state, &bad).unwrap();
    let mut seen = Vec::new();
    let status =
        updates::update_install_impl(&state, &bad, &mut |status| seen.push(status.clone()))
            .unwrap();
    assert_eq!(status.state, "failed");
    let error = status.error.unwrap();
    assert!(
        error.contains("signature verification failed") && error.contains("untouched"),
        "{error}"
    );
    assert_eq!(
        bad.installs.load(Ordering::SeqCst),
        0,
        "a package that fails its signature is never installed"
    );
    // A backup came first, and is still there.
    let backup = status.backup_path.expect("backup before install");
    assert!(Path::new(&backup).exists(), "{backup}");
    assert!(seen.first().is_some_and(|s| s.state == "backing_up"));
    // Progress at least every 5 %, only forward.
    let progress: Vec<u8> = seen.iter().filter_map(|s| s.progress).collect();
    assert!(
        progress
            .windows(2)
            .all(|pair| pair[1] > pair[0] && pair[1] - pair[0] <= 5),
        "{progress:?}"
    );
    assert_eq!(progress.last(), Some(&100));
    // Retry with a good package installs; still no restart.
    let good = FakeFeed::default();
    let status = updates::update_install_impl(&state, &good, &mut |_| {}).unwrap();
    assert_eq!(status.state, "installed");
    assert_eq!(good.installs.load(Ordering::SeqCst), 1);
    assert_eq!(
        updates::restart_blocker(&state).unwrap(),
        None,
        "no Instance live: restart allowed"
    );
}

#[test]
fn install_waits_for_a_switch_in_flight_and_restart_for_live_instances() {
    use juggler_core::model::{
        Account, AccountId, EdgeActor, EdgeState, Epoch, EpochId, HarnessId, InstanceKind,
        InstanceRecord, LaunchRecipe, MachineId, ProfileId, SessionId, SessionStatus, SwitchEdge,
        SwitchEdgeId, SwitchMechanism, Timestamp,
    };
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), true);
    let feed = FakeFeed::default();
    updates::update_check_impl(&state, &feed).unwrap();
    assert!(
        updates::restart_blocker(&state)
            .unwrap()
            .unwrap()
            .contains("nothing has been installed")
    );
    {
        let storage = state.storage.lock().unwrap();
        let project = storage
            .projects()
            .touch(Path::new("/w/p"), None)
            .unwrap()
            .id;
        let work_item = storage.switch_graph().create_work(&project, "w").unwrap();
        storage
            .switch_graph()
            .insert_epoch(&Epoch {
                id: EpochId::new("e1"),
                work: work_item.id.clone(),
                session_ref: None,
                native_id: None,
                account: AccountId::new("a"),
                provider: HarnessId::new("claude-code"),
                resumed_from_epoch: None,
                machine: None,
            })
            .unwrap();
        storage
            .switch_graph()
            .append_edge(&SwitchEdge {
                id: SwitchEdgeId::new("x1"),
                work: work_item.id,
                from_epoch: EpochId::new("e1"),
                to_epoch: None,
                from_account: AccountId::new("a"),
                to_account: AccountId::new("b"),
                from_provider: HarnessId::new("claude-code"),
                to_provider: HarnessId::new("claude-code"),
                mechanism: SwitchMechanism::Transplant,
                pack_id: None,
                fidelity_shown: None,
                manifest_sha256: None,
                state: EdgeState::Staged,
                actor: EdgeActor::User,
                parent_edge_a: None,
                parent_edge_b: None,
                created_at: Timestamp::now(),
                to_machine: None,
            })
            .unwrap();
    }
    let refused = updates::update_install_impl(&state, &feed, &mut |_| {})
        .unwrap_err()
        .to_string();
    assert!(refused.contains("switch is in progress"), "{refused}");
    assert_eq!(feed.downloads.load(Ordering::SeqCst), 0);
    state
        .storage
        .lock()
        .unwrap()
        .raw_conn()
        .execute("UPDATE switch_edges SET state = 'confirmed'", [])
        .unwrap();
    assert_eq!(
        updates::update_install_impl(&state, &feed, &mut |_| {})
            .unwrap()
            .state,
        "installed"
    );

    // A live Instance holds the restart back, with the reason.
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc"),
            harness: HarnessId::new("claude-code"),
            label: "W".into(),
            plan: None,
            state_dir: work.path().join("s"),
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
            id: ProfileId::new("r"),
            name: "Main".into(),
            account: AccountId::new("acc"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    storage
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new("live"),
            conversation: None,
            profile: ProfileId::new("r"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: work.path().to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
    drop(storage);
    let reason = updates::restart_blocker(&state).unwrap().unwrap();
    assert!(reason.contains("1 Instance is still running"), "{reason}");
}

/// D-20 / D-23: one feed per channel. AI Juggler follows beta until the
/// first stable release and chooses between stable and beta (nightly is
/// the separate AI Juggler Nightly app); the choice persists; anything
/// else is refused; switching forgets what the previous feed announced
/// (nothing downloads until the next check and Install).
#[test]
fn the_channel_is_a_persisted_choice_with_its_own_feed() {
    use juggler_app::updates::Channel;
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), true);
    let status = updates::update_status_impl(&state);
    assert_eq!(status.channel, "beta");
    assert_eq!(status.edition, "main");
    assert_eq!(status.app_name, "AI Juggler");
    assert_eq!(status.channels, ["stable", "beta"]);

    for (channel, feed) in [
        (Channel::Stable, "/stable-channel/latest.json"),
        (Channel::Beta, "/beta-channel/latest.json"),
        (Channel::Nightly, "/nightly-channel/latest.json"),
    ] {
        assert!(channel.endpoint().ends_with(feed), "{}", channel.endpoint());
        assert_eq!(Channel::parse(channel.as_str()), Some(channel));
    }
    assert_eq!(Channel::parse("canary"), None);

    // A check on beta finds 0.2.0 …
    let feed = FakeFeed::default();
    let found = updates::update_check_impl(&state, &feed).unwrap();
    assert_eq!(found.state, "available");
    // … switching to stable forgets it, and the choice persists.
    let switched = updates::update_set_channel_impl(&state, "stable").unwrap();
    assert_eq!(switched.channel, "stable");
    assert_eq!(switched.state, "idle");
    assert!(switched.available.is_none());
    assert_eq!(updates::current_channel(), Channel::Stable);
    drop(state);
    let reopened = app_state(dir.path(), true);
    assert_eq!(updates::update_status_impl(&reopened).channel, "stable");

    let refused = updates::update_set_channel_impl(&reopened, "canary").unwrap_err();
    assert!(
        refused.to_string().contains("choose stable or beta"),
        "{refused}"
    );
    let nightly = updates::update_set_channel_impl(&reopened, "nightly").unwrap_err();
    assert!(
        nightly.to_string().contains("AI Juggler Nightly"),
        "{nightly}"
    );
    assert_eq!(updates::update_status_impl(&reopened).channel, "stable");
}
