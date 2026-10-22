//! AJ-E708 integration: two app instances (two data dirs, one process)
//! over loopback. Proves: a console that was offline catches up with
//! only the changes it missed (sequence-vector exchange, logged in the
//! node's outbox log), keeps last-known rows while offline and across
//! a restart, resumes from its durable high-water mark after a crash, a
//! reset node (new sync epoch) is re-synced afresh, and 1,000 queued
//! changes catch up well inside the 3 s budget.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::remote;
use juggler_app::state::AppState;
use juggler_app::{fleet, link};
use juggler_core::model::{
    Account, AccountId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId, ProfileId,
    SessionId, SessionStatus, Timestamp,
};
use juggler_link::Permission;

fn app_state(dir: &Path) -> Arc<AppState> {
    let state = Arc::new(
        AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    link::link_disable_discovery_for_test(&state);
    state
}

fn wait_for<T>(what: &str, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn phase(info: link::PairingStateInfo, phase: &str) -> Option<()> {
    (info.phase == phase).then_some(())
}

fn pair(a: &Arc<AppState>, b: &Arc<AppState>) {
    let code = link::pairing_host_open_impl(a).unwrap().code.unwrap();
    let address = format!("127.0.0.1:{}", link::link_status_impl(a).port.unwrap());
    link::pairing_join_start_impl(b, None, Some(&address), &code).unwrap();
    wait_for("joiner confirm", || {
        phase(link::pairing_join_state_impl(b).unwrap(), "confirm")
    });
    wait_for("host confirm", || {
        phase(link::pairing_host_state_impl(a).unwrap(), "confirm")
    });
    link::pairing_join_decide_impl(b, true, Permission::View).unwrap();
    link::pairing_host_decide_impl(a, true, Permission::View).unwrap();
    wait_for("paired", || {
        (phase(link::pairing_join_state_impl(b).unwrap(), "paired").is_some()
            && phase(link::pairing_host_state_impl(a).unwrap(), "paired").is_some())
        .then_some(())
    });
}

fn seed_account(state: &Arc<AppState>, dir: &Path) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-work"),
            harness: juggler_core::model::HarnessId::new("claude-code"),
            label: "Work Max".into(),
            plan: None,
            state_dir: dir.join("state-work"),
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
            id: ProfileId::new("rec-work"),
            name: "Work".into(),
            account: AccountId::new("acc-work"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
}

fn instance(id: &str, dir: &Path, status: SessionStatus) -> InstanceRecord {
    InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new("rec-work"),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: dir.join(format!("project-{id}")),
        status,
        started_at: Timestamp::now(),
        last_activity: Some(Timestamp::now()),
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    }
}

fn machine_id(state: &Arc<AppState>) -> String {
    fleet::fleet_self_impl(state).unwrap().machine_id
}

/// record id → payload, the node's latest durable row per Instance.
fn outbox_latest(state: &Arc<AppState>) -> (HashMap<String, Vec<u8>>, u64) {
    let storage = state.storage.lock().unwrap();
    let entries = storage.fleet_sync().outbox_since(0, 100_000).unwrap();
    let top = entries.iter().map(|entry| entry.seq).max().unwrap_or(0);
    let mut latest = HashMap::new();
    for entry in entries {
        latest.insert(entry.record_id, entry.payload);
    }
    (latest, top)
}

/// record id → payload as the console's replica holds it.
fn replica(console: &Arc<AppState>, origin: &str) -> HashMap<String, Vec<u8>> {
    let storage = console.storage.lock().unwrap();
    storage
        .fleet_sync()
        .replica_list(origin, "instance")
        .unwrap()
        .into_iter()
        .map(|(entry, _)| (entry.record_id, entry.payload))
        .collect()
}

/// The node's outbox log: (asked-from sequence, changes sent) per
/// catch-up, oldest first.
fn catch_ups(node: &Arc<AppState>) -> Vec<(u64, u64)> {
    fleet::verify_audit(node)
        .unwrap()
        .into_iter()
        .filter(|entry| entry["kind"] == "sync_catch_up")
        .map(|entry| {
            (
                entry["sequence"].as_u64().unwrap(),
                entry["count"].as_u64().unwrap(),
            )
        })
        .collect()
}

/// The console's high-water mark for `node`: what it will ask from.
fn console_mark(console: &Arc<AppState>, node: &Arc<AppState>, origin: &str) -> u64 {
    let epoch = node
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .get("fleet_sync_epoch")
        .unwrap()
        .unwrap_or_default();
    console
        .storage
        .lock()
        .unwrap()
        .fleet_sync()
        .replica_high_water(origin, &epoch)
        .unwrap()
}

fn converged(node: &Arc<AppState>, console: &Arc<AppState>, origin: &str) -> Option<()> {
    let (latest, _) = outbox_latest(node);
    (!latest.is_empty() && replica(console, origin) == latest).then_some(())
}

#[test]
fn an_offline_console_catches_up_with_only_what_it_missed_and_keeps_last_known_rows() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    fleet::fleet_set_label_impl(&a, "Mac mini").unwrap();
    seed_account(&a, work.path());
    {
        let storage = a.storage.lock().unwrap();
        for n in 0..10 {
            storage
                .sessions()
                .insert(&instance(
                    &format!("inst-{n}"),
                    work.path(),
                    SessionStatus::Running,
                ))
                .unwrap();
        }
    }
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b);
    let origin = machine_id(&a);

    // First pair: the one full transfer.
    wait_for("first sync", || converged(&a, &b, &origin));
    let first = catch_ups(&a);
    assert_eq!(first[0].0, 0, "a first pair asks from the start");

    // The console goes away (laptop on a train).
    link::link_set_enabled_impl(&b, false).unwrap();
    let offline = wait_for("offline", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| m.freshness == "offline")
    });
    assert_eq!(
        offline.instances.len(),
        10,
        "last-known rows stay while offline"
    );
    assert!(
        offline
            .instances
            .iter()
            .all(|row| row.cpu_percent.is_none()),
        "live numbers are never shown as if current"
    );
    let (_, high_water) = outbox_latest(&a);
    let catch_ups_before = catch_ups(&a).len();

    // The node keeps working: 200 edits across 3 of its 10 Instances
    // plus 2 new ones, each one a durable change in its outbox.
    let mut edits = 0;
    for round in 0..40_i64 {
        let storage = a.storage.lock().unwrap();
        for n in 0..3 {
            let mut row = storage
                .sessions()
                .get(&SessionId::new(format!("inst-{n}")))
                .unwrap();
            row.status = if round % 2 == 0 {
                SessionStatus::WaitingUser
            } else {
                SessionStatus::Running
            };
            row.last_activity = Some(Timestamp::from_millis(1_000_000 + round * 10 + n as i64));
            storage.sessions().update(&row).unwrap();
        }
        drop(storage);
        edits += link::link_sync_now_for_test(&a);
        if round == 0 {
            let storage = a.storage.lock().unwrap();
            storage
                .sessions()
                .insert(&instance("inst-new-1", work.path(), SessionStatus::Running))
                .unwrap();
            storage
                .sessions()
                .insert(&instance("inst-new-2", work.path(), SessionStatus::Running))
                .unwrap();
        }
        for n in 0..2 {
            let storage = a.storage.lock().unwrap();
            if let Ok(mut row) = storage
                .sessions()
                .get(&SessionId::new(format!("inst-new-{}", n + 1)))
            {
                row.last_activity = Some(Timestamp::from_millis(2_000_000 + round * 10 + n as i64));
                storage.sessions().update(&row).unwrap();
            }
        }
        edits += link::link_sync_now_for_test(&a);
    }
    // Every edit is one outbox sequence, whether the test's sync pass or
    // the 2 s background tick recorded it (AJ-E806 ledger: counting only
    // the test's passes raced the tick).
    let (latest, top) = outbox_latest(&a);
    let _ = edits;
    assert!(
        top - high_water >= 200,
        "{} durable changes queued",
        top - high_water
    );
    let missing = {
        let storage = a.storage.lock().unwrap();
        storage
            .fleet_sync()
            .outbox_since(high_water, 100_000)
            .unwrap()
            .len() as u64
    };
    assert_eq!(missing, 5, "the outbox keeps one latest change per record");
    // The node says how many changes wait for the console.
    let console = machine_id(&b);
    let waiting = fleet::fleet_peers_impl(&a)
        .unwrap()
        .into_iter()
        .find(|peer| peer.machine_id == console)
        .unwrap();
    assert_eq!(
        waiting.queued_changes,
        Some(missing),
        "Offline — 5 changes queued"
    );

    // Link off means nothing arrived while it was off (CI once saw the
    // console ask from 31, past `high_water`: its telemetry task outlived
    // Link off and kept applying).
    let mark = console_mark(&b, &a, &origin);
    assert!(
        mark <= high_water,
        "the console applied {mark} while its Link was off (high water {high_water})"
    );

    // Back home: the console catches up from its high-water mark with
    // exactly the missing changes — never a full transfer.
    link::link_set_enabled_impl(&b, true).unwrap();
    wait_for("caught up", || converged(&a, &b, &origin));
    let log = catch_ups(&a);
    let resumed = log[catch_ups_before..].first().copied().unwrap();
    assert!(
        resumed.0 > 0 && resumed.0 <= high_water && resumed.0 < top,
        "{log:?}"
    );
    assert_eq!(resumed.1, missing, "only the missing changes: {log:?}");
    assert_eq!(
        replica(&b, &origin),
        latest,
        "zero silent overwrites or losses"
    );
    wait_for("nothing queued", || {
        let peer = fleet::fleet_peers_impl(&a)
            .unwrap()
            .into_iter()
            .find(|peer| peer.machine_id == console)?;
        (peer.queued_changes == Some(0)).then_some(())
    });

    // Replaying the same batch (a retry) changes nothing.
    {
        let storage = b.storage.lock().unwrap();
        let sync = storage.fleet_sync();
        let entries = {
            let node = a.storage.lock().unwrap();
            node.fleet_sync().outbox_since(0, 100_000).unwrap()
        };
        for entry in &entries {
            assert!(!sync.replica_apply(&origin, entry, 1).unwrap());
        }
    }
    assert_eq!(replica(&b, &origin), latest);

    let machine = wait_for("live again", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| m.freshness == "live" && m.synced_at_ms.is_some())
    });
    assert_eq!(machine.instances.len(), 12);
    let skew = machine.clock_skew_ms.expect("measured from its Hello");
    assert!(
        skew.abs() < remote::CLOCK_SKEW_FLAG_MS,
        "same clock: {skew}"
    );
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

/// Link off means off: once a console turns its Link off, nothing from
/// its peers is applied, even while the node keeps changing and keeps its
/// own Link on. `stop` used to abort the reconcile loop only, which
/// detached (never ended) each connection's telemetry task: it kept
/// streaming and writing the replica.
#[test]
fn a_console_with_its_link_off_applies_nothing() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    seed_account(&a, work.path());
    {
        let storage = a.storage.lock().unwrap();
        storage
            .sessions()
            .insert(&instance("inst-off", work.path(), SessionStatus::Running))
            .unwrap();
    }
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b);
    let origin = machine_id(&a);
    wait_for("first sync", || converged(&a, &b, &origin));

    link::link_set_enabled_impl(&b, false).unwrap();
    let mark = console_mark(&b, &a, &origin);
    let replica_before = replica(&b, &origin);
    // The node keeps working, Link on. Its own 2 s pass records each
    // change and sends it to whoever is subscribed (a test's own sync pass
    // would record it without sending it).
    for round in 0..4_i64 {
        {
            let storage = a.storage.lock().unwrap();
            let mut row = storage.sessions().get(&SessionId::new("inst-off")).unwrap();
            row.last_activity = Some(Timestamp::from_millis(2_000_000 + round));
            storage.sessions().update(&row).unwrap();
        }
        let before = outbox_latest(&a).1;
        wait_for("the node's pass", || {
            (outbox_latest(&a).1 > before).then_some(())
        });
        // Time for a subscriber to receive and apply it.
        std::thread::sleep(Duration::from_millis(500));
    }
    assert!(outbox_latest(&a).1 > mark, "the node did change");
    assert_eq!(
        console_mark(&b, &a, &origin),
        mark,
        "nothing applied after Link off"
    );
    assert_eq!(replica(&b, &origin), replica_before);
}

/// Link off ends a join in flight too: this Mac's confirm, arriving after
/// Link went off, pairs nothing, even though the other Mac confirms.
#[test]
fn a_join_in_flight_when_link_goes_off_pairs_nothing() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    let code = link::pairing_host_open_impl(&a).unwrap().code.unwrap();
    let address = format!("127.0.0.1:{}", link::link_status_impl(&a).port.unwrap());
    link::pairing_join_start_impl(&b, None, Some(&address), &code).unwrap();
    wait_for("joiner confirm", || {
        phase(link::pairing_join_state_impl(&b).unwrap(), "confirm")
    });
    wait_for("host confirm", || {
        phase(link::pairing_host_state_impl(&a).unwrap(), "confirm")
    });

    link::link_set_enabled_impl(&b, false).unwrap();
    // Both users confirm, this Mac's after its Link went off.
    let _ = link::pairing_join_decide_impl(&b, true, Permission::View);
    let _ = link::pairing_host_decide_impl(&a, true, Permission::View);
    std::thread::sleep(Duration::from_secs(2));

    assert!(
        fleet::fleet_peers_impl(&b).unwrap().is_empty(),
        "nothing paired after Link off"
    );
    assert_ne!(link::pairing_join_state_impl(&b).unwrap().phase, "paired");
}

#[test]
fn a_console_restarted_mid_sync_resumes_from_its_durable_high_water_mark() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    seed_account(&a, work.path());
    {
        let storage = a.storage.lock().unwrap();
        storage
            .sessions()
            .insert(&instance(
                "inst-durable",
                work.path(),
                SessionStatus::Running,
            ))
            .unwrap();
    }
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b);
    let origin = machine_id(&a);
    wait_for("first sync", || converged(&a, &b, &origin));

    // The console process dies (no clean Link off) and comes back.
    let b_dir = work.path().join("b");
    link::link_set_enabled_impl(&b, false).unwrap();
    drop(b);
    let b = app_state(&b_dir);
    // Before any connection: what it knew survived the restart, shown
    // as last known.
    let before = remote::fleet_remote_impl(&b).unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].freshness, "offline");
    assert_eq!(before[0].instances.len(), 1);
    assert_eq!(before[0].instances[0].id, "inst-durable");

    // Meanwhile the node changed; the console resumes, not restarts.
    {
        let storage = a.storage.lock().unwrap();
        let mut row = storage
            .sessions()
            .get(&SessionId::new("inst-durable"))
            .unwrap();
        row.status = SessionStatus::WaitingUser;
        storage.sessions().update(&row).unwrap();
    }
    link::link_sync_now_for_test(&a);
    let catch_ups_before = catch_ups(&a).len();
    link::link_set_enabled_impl(&b, true).unwrap();
    wait_for("resumed", || converged(&a, &b, &origin));
    let log = catch_ups(&a);
    let resumed = log[catch_ups_before..].first().copied().unwrap();
    assert!(resumed.0 > 0, "resumed from the stored mark: {log:?}");
    assert_eq!(resumed.1, 1, "only the one change: {log:?}");
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

#[test]
fn a_reset_node_gets_a_new_sync_epoch_and_the_console_starts_it_afresh() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    seed_account(&a, work.path());
    {
        let storage = a.storage.lock().unwrap();
        storage
            .sessions()
            .insert(&instance("inst-kept", work.path(), SessionStatus::Running))
            .unwrap();
    }
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b);
    let origin = machine_id(&a);
    wait_for("first sync", || converged(&a, &b, &origin));
    // A record the node no longer has (as after a reset).
    {
        let storage = b.storage.lock().unwrap();
        storage
            .fleet_sync()
            .replica_apply(
                &origin,
                &juggler_core::storage::SyncEntry {
                    seq: 1_000_000,
                    kind: "instance".into(),
                    record_id: "ghost".into(),
                    payload: Vec::new(),
                    wall_ms: 0,
                },
                0,
            )
            .unwrap();
    }
    // The node is reset: a new epoch. Its next Hello tells the console.
    {
        let storage = a.storage.lock().unwrap();
        storage
            .app_settings()
            .set("fleet_sync_epoch", "0123456789abcdef")
            .unwrap();
    }
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&a, true).unwrap();
    wait_for("re-synced afresh", || {
        let rows = replica(&b, &origin);
        (!rows.contains_key("ghost") && rows.contains_key("inst-kept")).then_some(())
    });
    assert_eq!(
        catch_ups(&a).last().unwrap().0,
        0,
        "a new epoch asks from 0"
    );
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

#[test]
fn a_thousand_queued_changes_catch_up_inside_three_seconds() {
    use prost::Message as _;
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b);
    let origin = machine_id(&a);
    link::link_set_enabled_impl(&b, false).unwrap();
    let mut writes = Vec::with_capacity(1_000);
    {
        let storage = a.storage.lock().unwrap();
        let sync = storage.fleet_sync();
        for n in 0..1_000 {
            let row = juggler_link::proto::SessionDelta {
                instance_id: format!("bulk-{n}"),
                provider_id: "claude-code".into(),
                account_label: "Work Max".into(),
                status: "running".into(),
                updated_at_ms: 1,
                started_at_ms: n,
                ..Default::default()
            };
            let (wall, cpu) = (Instant::now(), cpu_time::ThreadTime::now());
            sync.outbox_append("instance", &row.instance_id, &row.encode_to_vec(), 1)
                .unwrap();
            writes.push((cpu.elapsed(), wall.elapsed()));
        }
    }
    // The budget is per write at p95, in what the write costs the app —
    // its thread's CPU time. Wall time adds the disk's commit latency,
    // which another program saturating the disk can stretch (AJ-E806
    // ledger); it is printed beside it.
    let p95 = |mut times: Vec<Duration>| {
        times.sort();
        times[times.len() * 95 / 100]
    };
    let cpu_p95 = p95(writes.iter().map(|(cpu, _)| *cpu).collect());
    let wall_p95 = p95(writes.iter().map(|(_, wall)| *wall).collect());
    eprintln!("[perf] outbox write p95 over 1,000: {cpu_p95:?} CPU, {wall_p95:?} wall");
    assert!(
        cpu_p95 < Duration::from_millis(10),
        "outbox write budget: p95 {cpu_p95:?} CPU"
    );
    let started = Instant::now();
    link::link_set_enabled_impl(&b, true).unwrap();
    wait_for("1,000 caught up", || {
        (replica(&b, &origin).len() >= 1_000).then_some(())
    });
    let took = started.elapsed();
    eprintln!("[perf] catch-up of 1,000 changes (incl. reconnect): {took:?}");
    assert!(took < Duration::from_secs(3), "{took:?}");
    let shown = remote::fleet_remote_impl(&b).unwrap();
    assert_eq!(
        shown[0].instances.len(),
        50,
        "the board shows the newest 50"
    );
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

/// Link off means off however clicks race: several "on" at once start
/// one Link, and the "off" after them leaves none running. (That its
/// port is free is `juggler-link`'s `a_stopped_nodes_port_is_free_at_once`:
/// tests here share the preferred port.)
#[test]
fn starts_at_once_run_one_link_and_off_ends_it() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let ports: Vec<Option<u16>> = std::thread::scope(|scope| {
        let starts: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| link::link_set_enabled_impl(&a, true).unwrap().port))
            .collect();
        starts
            .into_iter()
            .map(|start| start.join().unwrap())
            .collect()
    });
    let first = ports[0];
    assert!(first.is_some());
    assert!(
        ports.iter().all(|port| *port == first),
        "one Link: {ports:?}"
    );
    let off = link::link_set_enabled_impl(&a, false).unwrap();
    assert!(!off.enabled && off.port.is_none());
}

/// Off during the launch start (Link left on last time): the start
/// waits its turn or finds the setting off, and Link stays off.
#[test]
fn off_during_the_launch_start_stays_off() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&a, false).unwrap();
    {
        // As a quit with Link on leaves it.
        let storage = a.storage.lock().unwrap();
        storage
            .app_settings()
            .set("fleet_link_enabled", "1")
            .unwrap();
    }
    link::resume_if_enabled(Arc::clone(&a));
    let off = link::link_set_enabled_impl(&a, false).unwrap();
    assert!(!off.enabled);
    std::thread::sleep(Duration::from_secs(3));
    let status = link::link_status_impl(&a);
    assert!(
        !status.enabled && status.port.is_none(),
        "the setting says off and nothing runs: {status:?}"
    );
}

/// A join started while Link goes off: it either never starts ("Turn Link
/// on first") or is ended by the off. Nothing pairs and no join is left
/// "connecting".
#[test]
fn a_join_racing_link_off_pairs_nothing() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    link::link_set_enabled_impl(&a, true).unwrap();
    let address = format!("127.0.0.1:{}", link::link_status_impl(&a).port.unwrap());
    for _ in 0..5 {
        link::link_set_enabled_impl(&b, true).unwrap();
        let code = link::pairing_host_open_impl(&a).unwrap().code.unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let _ = link::pairing_join_start_impl(&b, None, Some(&address), &code);
            });
            scope.spawn(|| link::link_set_enabled_impl(&b, false).unwrap());
        });
        // Whatever the order, a confirm now finds nothing to finish.
        let _ = link::pairing_join_decide_impl(&b, true, Permission::View);
        let _ = link::pairing_host_decide_impl(&a, true, Permission::View);
        let phase = link::pairing_join_state_impl(&b).unwrap().phase;
        if !link::link_status_impl(&b).enabled {
            assert!(
                phase != "connecting" && phase != "confirm",
                "a join left {phase} after Link off"
            );
        }
        link::pairing_host_close_impl(&a);
    }
    std::thread::sleep(Duration::from_secs(1));
    assert!(
        fleet::fleet_peers_impl(&b).unwrap().is_empty(),
        "nothing paired after Link off"
    );
}
