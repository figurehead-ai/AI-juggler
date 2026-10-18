//! AJ-E704 integration: two app instances (two data dirs, one process)
//! pair over loopback through the command layer — the same path as
//! pairing by address over the user's VPN. Proves: Link is off by
//! default, the right code plus both confirmations pins both sides and
//! connects them, a wrong code aborts and rotates the code, the lockout
//! survives a restart, the attempt log never holds the code and its
//! chain verifies, and Link off drops back to `link_off`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

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
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn host_phase(state: &Arc<AppState>, phase: &str) -> Option<link::PairingStateInfo> {
    let info = link::pairing_host_state_impl(state).unwrap();
    (info.phase == phase).then_some(info)
}

fn join_phase(state: &Arc<AppState>, phase: &str) -> Option<link::PairingStateInfo> {
    let info = link::pairing_join_state_impl(state).unwrap();
    (info.phase == phase).then_some(info)
}

fn address_of(state: &Arc<AppState>) -> String {
    format!("127.0.0.1:{}", link::link_status_impl(state).port.unwrap())
}

fn wrong(code: &str) -> String {
    let digits: String = code.chars().filter(char::is_ascii_digit).collect();
    let first = digits.chars().next().unwrap().to_digit(10).unwrap();
    format!("{}{}", (first + 1) % 10, &digits[1..])
}

#[test]
fn link_is_off_by_default_and_pairing_needs_it_on() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(work.path());
    let status = link::link_status_impl(&a);
    assert!(!status.enabled);
    assert_eq!(status.port, None);
    assert!(link::pairing_host_open_impl(&a).is_err());
    assert!(link::pairing_join_start_impl(&a, None, Some("127.0.0.1"), "123456").is_err());
    assert!(link::discovered_impl(&a).is_empty());
}

#[test]
fn the_right_code_and_both_confirmations_pair_and_connect_both_macs() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    fleet::fleet_set_label_impl(&a, "Mac mini").unwrap();
    fleet::fleet_set_label_impl(&b, "Laptop").unwrap();
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();

    let started = Instant::now();
    let opened = link::pairing_host_open_impl(&a).unwrap();
    assert_eq!(opened.phase, "waiting");
    let code = opened.code.clone().unwrap();
    assert_eq!(code.chars().filter(char::is_ascii_digit).count(), 6);
    let pairing_link = opened.link.clone().unwrap();
    assert!(pairing_link.starts_with("aijuggler-pair:v1:"));

    // The joiner pastes the pairing link (code + the host's fingerprint).
    link::pairing_join_start_impl(&b, None, Some(&address_of(&a)), &pairing_link).unwrap();
    let on_b = wait_for("the joiner's confirm step", || join_phase(&b, "confirm"));
    let on_a = wait_for("the host's confirm step", || host_phase(&a, "confirm"));
    // Each side sees the OTHER machine's label and real fingerprint.
    assert_eq!(on_b.peer_label.as_deref(), Some("Mac mini"));
    assert_eq!(on_a.peer_label.as_deref(), Some("Laptop"));
    assert_eq!(
        on_b.peer_fingerprint.unwrap(),
        fleet::fleet_self_impl(&a).unwrap().fingerprint
    );
    assert_eq!(
        on_a.peer_fingerprint.unwrap(),
        fleet::fleet_self_impl(&b).unwrap().fingerprint
    );

    link::pairing_join_decide_impl(&b, true, Permission::View).unwrap();
    link::pairing_host_decide_impl(&a, true, Permission::View).unwrap();
    wait_for("both sides paired", || {
        (join_phase(&b, "paired").is_some() && host_phase(&a, "paired").is_some()).then_some(())
    });
    assert!(
        started.elapsed() < Duration::from_secs(60),
        "pairing took {:?}",
        started.elapsed()
    );

    let peers_a = fleet::fleet_peers_impl(&a).unwrap();
    let peers_b = fleet::fleet_peers_impl(&b).unwrap();
    assert_eq!(peers_a.len(), 1);
    assert_eq!(peers_b.len(), 1);
    assert_eq!(peers_a[0].label, "Laptop");
    assert_eq!(peers_a[0].permission, "view", "View only is the default");
    assert_eq!(peers_b[0].label, "Mac mini");

    // Both sides reconnect to each other by the remembered address.
    for (state, name) in [(&a, "a"), (&b, "b")] {
        let peer = wait_for(&format!("{name} connected"), || {
            fleet::fleet_peers_impl(state)
                .unwrap()
                .into_iter()
                .find(|peer| peer.connection == "connected")
        });
        assert_eq!(peer.protocol.as_deref(), Some("v1.0"));
    }

    // The attempt log names the outcome and never the code.
    let entries = fleet::verify_audit(&a).expect("the attempt log chain verifies");
    assert!(
        entries
            .iter()
            .any(|entry| entry["kind"] == "pairing" && entry["reason"] == "paired")
    );
    let raw =
        std::fs::read_to_string(work.path().join("a/app-data/fleet/link-audit.jsonl")).unwrap();
    let digits: String = code.chars().filter(char::is_ascii_digit).collect();
    assert!(!raw.contains(&digits), "the code must never be logged");

    // Link off: every peer reads link_off again.
    link::link_set_enabled_impl(&a, false).unwrap();
    assert!(!link::link_status_impl(&a).enabled);
    assert!(
        fleet::fleet_peers_impl(&a)
            .unwrap()
            .iter()
            .all(|peer| peer.connection == "link_off")
    );
    link::link_set_enabled_impl(&b, false).unwrap();
}

#[test]
fn a_wrong_code_aborts_rotates_the_code_and_pins_nothing() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    let code = link::pairing_host_open_impl(&a).unwrap().code.unwrap();

    link::pairing_join_start_impl(&b, None, Some(&address_of(&a)), &wrong(&code)).unwrap();
    let failed = wait_for("the joiner's failure", || join_phase(&b, "failed"));
    assert_eq!(failed.reason.as_deref(), Some("mismatch"));
    assert!(failed.message.is_some());
    let after = wait_for("the code to rotate", || {
        link::pairing_host_state_impl(&a)
            .unwrap()
            .code
            .filter(|now| *now != code)
    });
    assert_ne!(after, code, "a failed attempt rotates the code");
    assert!(fleet::fleet_peers_impl(&a).unwrap().is_empty());
    assert!(fleet::fleet_peers_impl(&b).unwrap().is_empty());
    // An unpaired machine that skips pairing and just connects is
    // refused at Hello — and that attempt is logged too.
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let refused = runtime.block_on(juggler_link::client::connect(
        address_of(&a).parse().unwrap(),
        &fleet::node_certificate(&b).unwrap(),
        fleet::local_public_key(&a).unwrap(),
        "an-unpaired-machine",
    ));
    assert!(refused.is_err(), "an unpaired machine is never served");

    let entries = fleet::verify_audit(&a).expect("the attempt log chain verifies");
    assert!(
        entries
            .iter()
            .any(|entry| entry["kind"] == "pairing" && entry["reason"] == "mismatch")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry["kind"] == "hello_rejected" && entry["reason"] == "not_paired")
    );
    let raw =
        std::fs::read_to_string(work.path().join("a/app-data/fleet/link-audit.jsonl")).unwrap();
    let digits: String = code.chars().filter(char::is_ascii_digit).collect();
    assert!(!raw.contains(&digits), "the code must never be logged");
    if std::env::var_os("AIJ_PRINT_AUDIT").is_some() {
        eprintln!("--- link-audit.jsonl ---\n{raw}");
    }

    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

#[test]
fn five_wrong_codes_lock_pairing_and_the_lock_survives_a_restart() {
    let work = tempfile::tempdir().unwrap();
    let a_dir = work.path().join("a");
    let a = app_state(&a_dir);
    let b = app_state(&work.path().join("b"));
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    link::pairing_host_open_impl(&a).unwrap();
    for attempt in 0..5 {
        let code = link::pairing_host_state_impl(&a).unwrap().code.unwrap();
        link::pairing_join_start_impl(&b, None, Some(&address_of(&a)), &wrong(&code)).unwrap();
        wait_for(&format!("failure {attempt}"), || join_phase(&b, "failed"));
        if attempt < 4 {
            wait_for("the code to rotate", || {
                link::pairing_host_state_impl(&a)
                    .unwrap()
                    .code
                    .filter(|now| *now != code)
            });
        }
        link::pairing_join_reset_impl(&b);
    }
    // Locked: a new window can't open.
    link::pairing_host_close_impl(&a);
    let locked = link::pairing_host_open_impl(&a).unwrap();
    assert_eq!(locked.phase, "failed");
    assert_eq!(locked.reason.as_deref(), Some("locked_out"));
    assert!(locked.code.is_none(), "no code while locked");

    // "Restart": a fresh app state over the same data dir.
    link::link_set_enabled_impl(&a, false).unwrap();
    drop(a);
    let a = app_state(&a_dir);
    link::link_set_enabled_impl(&a, true).unwrap();
    let still = link::pairing_host_open_impl(&a).unwrap();
    assert_eq!(
        still.reason.as_deref(),
        Some("locked_out"),
        "the lockout is persisted"
    );
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

/// Pairs `a` (host) and `b` (joiner) by address; `a` grants `b`
/// `grant_b`, `b` grants `a` View only.
fn pair(a: &Arc<AppState>, b: &Arc<AppState>, grant_b: Permission) {
    let code = link::pairing_host_open_impl(a).unwrap().code.unwrap();
    link::pairing_join_start_impl(b, None, Some(&address_of(a)), &code).unwrap();
    wait_for("joiner confirm", || join_phase(b, "confirm"));
    wait_for("host confirm", || host_phase(a, "confirm"));
    link::pairing_join_decide_impl(b, true, Permission::View).unwrap();
    link::pairing_host_decide_impl(a, true, grant_b).unwrap();
    wait_for("paired", || {
        (join_phase(b, "paired").is_some() && host_phase(a, "paired").is_some()).then_some(())
    });
}

/// One Instance on `state` (storage only; nothing is spawned).
fn seed_instance(state: &Arc<AppState>, dir: &Path) {
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
    storage
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new("inst-remote-1"),
            conversation: None,
            profile: ProfileId::new("rec-work"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.join("web-shop"),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
}

#[test]
fn a_console_sees_the_other_macs_instances_live_and_its_commands_go_through_the_node() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    fleet::fleet_set_label_impl(&a, "Mac mini").unwrap();
    seed_instance(&a, work.path());
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b, Permission::View);

    // B (the console) sees A's resources and Instance, source-labelled
    // and live — never a path, only the project folder's name.
    let machine = wait_for("A reported to B", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| m.freshness == "live" && m.resources.is_some() && !m.instances.is_empty())
    });
    assert_eq!(machine.label, "Mac mini");
    assert_eq!(
        machine.permission.as_deref(),
        Some("view"),
        "what A lets B do"
    );
    let instance = &machine.instances[0];
    assert_eq!(instance.id, "inst-remote-1");
    assert_eq!(instance.account_label, "Work Max");
    assert_eq!(
        instance.status, "running",
        "the status word, not a debug string"
    );
    assert_eq!(instance.project_name.as_deref(), Some("web-shop"));
    assert!(instance.measured_at_ms > 0);

    // View only: the console never even sends a stop.
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let refused = runtime
        .block_on(remote::fleet_remote_stop_impl(
            Arc::clone(&b),
            machine.machine_id.clone(),
            "inst-remote-1".into(),
        ))
        .unwrap();
    assert!(!refused.accepted);
    assert_eq!(refused.reason_code.as_deref(), Some("permission_denied"));

    // Re-pair with View + control: now the node decides — an Instance it
    // doesn't have is refused in its own words, through the full
    // nonce/sequence/expiry pipeline.
    pair(&a, &b, Permission::Control);
    let machine = wait_for("control granted", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| m.permission.as_deref() == Some("control") && m.freshness == "live")
    });
    let unknown = wait_for("the node's verdict", || {
        let outcome = runtime
            .block_on(remote::fleet_remote_stop_impl(
                Arc::clone(&b),
                machine.machine_id.clone(),
                "no-such-instance".into(),
            ))
            .unwrap();
        // The first command may race the first heartbeat's clock offset.
        (outcome.reason_code.as_deref() != Some("clock_unmeasured")).then_some(outcome)
    });
    assert!(!unknown.accepted);
    assert_eq!(
        unknown.reason_code.as_deref(),
        Some("unknown_record"),
        "{unknown:?}"
    );
    let entries = fleet::verify_audit(&a).unwrap();
    assert!(
        entries
            .iter()
            .any(|e| e["kind"] == "command_rejected" && e["reason"] == "unknown_record")
    );

    // Link off on the console: the live cache is cleared and the machine
    // offline; its rows stay as last known (AJ-E708), with no live
    // numbers shown as if current.
    link::link_set_enabled_impl(&b, false).unwrap();
    let after = remote::fleet_remote_impl(&b).unwrap();
    assert!(after.iter().all(|m| m.freshness == "offline"
        && m.resources.is_none()
        && m.instances.iter().all(|row| row.cpu_percent.is_none())));
    link::link_set_enabled_impl(&a, false).unwrap();
}

#[test]
fn ask_each_time_holds_a_command_until_this_macs_user_decides() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    fleet::fleet_set_label_impl(&b, "Laptop").unwrap();
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b, Permission::View);
    let b_on_a = fleet::fleet_peers_impl(&a).unwrap()[0].machine_id.clone();
    let a_on_b = fleet::fleet_peers_impl(&b).unwrap()[0].machine_id.clone();

    // The change is made ON A (the node) — a console can't raise its own access.
    let peers = fleet::fleet_set_permission_impl(&a, &b_on_a, "control", true).unwrap();
    assert_eq!(peers[0].permission, "control");
    assert!(peers[0].ask_each_time);
    wait_for("B connected to A", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| m.freshness == "live")
    });
    link::refresh_permissions_for_test(&b);
    assert_eq!(
        remote::fleet_remote_impl(&b).unwrap()[0]
            .permission
            .as_deref(),
        Some("control")
    );

    let send = |instance: &str| {
        let b = Arc::clone(&b);
        let id = a_on_b.clone();
        let instance = instance.to_owned();
        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(remote::fleet_remote_stop_impl(b, id, instance))
                .unwrap()
        })
    };
    // Deny: nothing runs, and the console hears why.
    let pending = send("no-such-instance");
    let prompt = wait_for("A's prompt", || link::prompts_impl(&a).into_iter().next());
    assert_eq!(prompt.peer_label, "Laptop");
    assert!(prompt.action.starts_with("stop "), "{}", prompt.action);
    link::prompt_decide_impl(&a, prompt.id, false);
    let denied = pending.join().unwrap();
    assert_eq!(
        denied.reason_code.as_deref(),
        Some("not_confirmed"),
        "{denied:?}"
    );
    assert!(link::prompts_impl(&a).is_empty());

    // Allow: the node goes on to its own checks (no such Instance here).
    let pending = send("no-such-instance");
    let prompt = wait_for("A's second prompt", || {
        link::prompts_impl(&a).into_iter().next()
    });
    link::prompt_decide_impl(&a, prompt.id, true);
    let allowed = pending.join().unwrap();
    assert_eq!(
        allowed.reason_code.as_deref(),
        Some("unknown_record"),
        "{allowed:?}"
    );

    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

#[test]
fn alerts_on_one_mac_reach_its_paired_console_named_by_machine() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    fleet::fleet_set_label_impl(&a, "Mac mini").unwrap();
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b, Permission::View);
    wait_for("B subscribed to A", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| m.freshness == "live")
    });
    let event = juggler_app::commands::AlertEventInfo {
        kind: "exhausted".into(),
        account: "Work Max".into(),
        text: "Work Max hit its 5-hour limit — resets at 14:00.".into(),
        at_ms: 1_800_000_000_000,
    };
    // Twice: a re-delivered alert is shown once.
    link::publish_alerts(&a, std::slice::from_ref(&event));
    link::publish_alerts(&a, std::slice::from_ref(&event));
    let machine = wait_for("the alert on B", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| !m.alerts.is_empty())
    });
    assert_eq!(machine.label, "Mac mini");
    assert_eq!(machine.alerts.len(), 1, "deduplicated");
    assert_eq!(machine.alerts[0].kind, "exhausted");
    assert_eq!(machine.alerts[0].account, "Work Max");
    assert!(machine.alerts[0].text.contains("5-hour limit"));
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

/// A scripted focus runner on the node: one window, every AX call works.
struct OneWindow;

impl juggler_app::focus::ScriptRunner for OneWindow {
    fn osascript(&self, source: &str) -> Result<String, String> {
        if source.contains("get name of every window") {
            return Ok("notes-a".into());
        }
        Ok(String::new())
    }
    fn activate(&self, _bundle_id: &str) -> Result<(), String> {
        Ok(())
    }
}

fn seed_desktop(state: &Arc<AppState>, dir: &Path) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-desk"),
            harness: juggler_core::model::HarnessId::new("claude-code"),
            label: "Desk".into(),
            plan: None,
            state_dir: dir.join("state-desk"),
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
            id: ProfileId::new("rec-desk"),
            name: "Desk".into(),
            account: AccountId::new("acc-desk"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    storage
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new("inst-desk"),
            conversation: None,
            profile: ProfileId::new("rec-desk"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Desktop,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: Some(juggler_core::model::WindowIdentity {
                pid: 4242,
                bundle_id: Some("com.example.notes".into()),
                title: None,
                cgwindow_id: None,
            }),
        })
        .unwrap();
}

#[test]
fn a_remote_focus_runs_on_the_owning_mac_and_reports_its_honest_outcome() {
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    fleet::fleet_set_label_impl(&a, "Studio Mac").unwrap();
    seed_desktop(&a, work.path());
    seed_instance(&a, work.path()); // a terminal Instance too
    juggler_app::focus::set_link_runner_for_test(Some(Arc::new(OneWindow)));
    juggler_app::focus::set_screen_locked_for_test(Some(false));
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b, Permission::Control);
    let machine = wait_for("A reported its desktop Instance", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.permission.as_deref() == Some("control")
                    && m.instances
                        .iter()
                        .any(|i| i.id == "inst-desk" && i.kind == "desktop")
            })
    });
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let focus = |instance: &str| {
        wait_for("a verdict", || {
            let outcome = runtime
                .block_on(remote::fleet_remote_focus_impl(
                    Arc::clone(&b),
                    machine.machine_id.clone(),
                    instance.to_owned(),
                ))
                .unwrap();
            (outcome.reason_code.as_deref() != Some("clock_unmeasured")).then_some(outcome)
        })
    };

    let focused = focus("inst-desk");
    assert!(focused.accepted, "{focused:?}");
    assert_eq!(
        focused.message, "Focused window",
        "the node's own outcome words"
    );

    let terminal = focus("inst-remote-1");
    assert!(!terminal.accepted);
    assert_eq!(terminal.reason_code.as_deref(), Some("not_desktop"));

    juggler_app::focus::set_screen_locked_for_test(Some(true));
    let locked = focus("inst-desk");
    assert!(!locked.accepted);
    assert_eq!(locked.reason_code.as_deref(), Some("screen_locked"));
    assert_eq!(
        locked.message,
        "Studio Mac is locked — nothing was focused."
    );

    // Audited on both machines.
    let node = fleet::verify_audit(&a).unwrap();
    assert!(node.iter().any(|e| e["kind"] == "command_accepted"));
    assert!(
        node.iter()
            .any(|e| e["kind"] == "command_rejected" && e["reason"] == "screen_locked")
    );
    let console = fleet::verify_audit(&b).unwrap();
    assert!(
        console
            .iter()
            .any(|e| e["kind"] == "command_sent" && e["reason"] == "accepted")
    );
    assert!(
        console
            .iter()
            .any(|e| e["kind"] == "command_sent" && e["reason"] == "screen_locked")
    );

    juggler_app::focus::set_screen_locked_for_test(None);
    juggler_app::focus::set_link_runner_for_test(None);
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}

/// A `sleep` reparented to launchd (no zombie for the test to reap).
fn detached_sleep() -> u32 {
    let out = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("/bin/sleep 120 >/dev/null 2>&1 & echo $!")
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

fn alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

fn helper_row(
    state: &Arc<AppState>,
    pid: u32,
    started_at: i64,
    exe: &str,
) -> juggler_core::storage::LedgerRow {
    juggler_core::storage::LedgerRow {
        pid: Some(pid),
        pgid: None,
        started_at: Some(started_at),
        exe_path: exe.into(),
        cmdline_hash: "fleet-test".into(),
        cmdline_trunc: "sleep 120".into(),
        boot_id: state.ledger.boot_id().to_string(),
        session_id: "inst-remote-1".into(),
        instance_id: None,
        provider: "claude-code".into(),
        account_id: Some("acc-work".into()),
        project_id: None,
        machine_id: juggler_app::commands::local_machine_id(state)
            .as_str()
            .to_string(),
        root_pid: None,
        parent_pid: None,
        depth: 1,
        role: "helper".into(),
        state: "orphaned".into(),
        listens_on: None,
        cwd: "/".into(),
        peak_rss: None,
        peak_cpu: None,
        exit_kind: None,
        reaped_by_us: None,
        escaped_group: false,
        down_period: false,
        first_seen: Timestamp::now().millis() - 60_000,
        last_seen: Timestamp::now().millis(),
    }
}

#[test]
fn a_console_cleans_only_what_the_node_verifies_and_hears_each_outcome() {
    use juggler_supervisor::tree::{IdentityProbe as _, LedgerStore as _, OsIdentityProbe};
    let work = tempfile::tempdir().unwrap();
    let a = app_state(&work.path().join("a"));
    let b = app_state(&work.path().join("b"));
    fleet::fleet_set_label_impl(&b, "Laptop").unwrap();
    seed_instance(&a, work.path());
    let owned = detached_sleep();
    let impostor = detached_sleep();
    let facts = OsIdentityProbe.probe(owned).expect("probe the owned sleep");
    let exe = facts.exe.to_string_lossy().into_owned();
    a.ledger
        .upsert_helpers(
            &SessionId::new("inst-remote-1"),
            &[
                helper_row(&a, owned, facts.start_time, &exe),
                // Recorded with the WRONG start time: never ours to touch.
                helper_row(&a, impostor, 1, &exe),
            ],
        )
        .unwrap();
    // One sampler tick publishes the process table (the app ticks every 2 s).
    a.resources
        .lock()
        .unwrap()
        .tick(&a.supervisor, Timestamp::now().millis());
    link::link_set_enabled_impl(&a, true).unwrap();
    link::link_set_enabled_impl(&b, true).unwrap();
    pair(&a, &b, Permission::Control);

    // B sees A's processes, with A's own verification verdicts.
    let machine = wait_for("A's processes on B", || {
        remote::fleet_remote_impl(&b)
            .unwrap()
            .into_iter()
            .find(|m| {
                m.permission.as_deref() == Some("control")
                    && m.processes.iter().any(|p| p.pid == impostor)
            })
    });
    let verified = |pid: u32| {
        machine
            .processes
            .iter()
            .find(|p| p.pid == pid)
            .unwrap()
            .verified
    };
    assert!(verified(owned), "{:?}", machine.processes);
    assert!(!verified(impostor), "{:?}", machine.processes);
    assert!(
        machine.processes.iter().all(|p| p.name == "sleep"),
        "only the executable's name travels — never a command line"
    );

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let clean = |pid: u32| {
        wait_for("a verdict", || {
            let outcome = runtime
                .block_on(remote::fleet_remote_clean_impl(
                    Arc::clone(&b),
                    machine.machine_id.clone(),
                    "inst-remote-1".into(),
                    pid,
                ))
                .unwrap();
            (outcome.reason_code.as_deref() != Some("clock_unmeasured")).then_some(outcome)
        })
    };

    // The impostor: the node re-verifies, refuses, sends zero signals.
    let refused = clean(impostor);
    assert!(!refused.accepted, "{refused:?}");
    assert_eq!(refused.reason_code.as_deref(), Some("refused"));
    assert!(alive(impostor), "a refused process is never signalled");

    // The owned one: cleaned by the node's own ladder, outcome in words.
    let cleaned = clean(owned);
    assert!(cleaned.accepted, "{cleaned:?}");
    assert!(
        cleaned.message.contains(&format!("Exited (pid {owned})"))
            || cleaned
                .message
                .contains(&format!("Needed SIGKILL (pid {owned})")),
        "{}",
        cleaned.message
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(owned) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!alive(owned));
    // The node's cleanup audit names the console that asked.
    assert!(
        a.audit
            .history(0)
            .unwrap()
            .iter()
            .any(|row| row.pid == owned && row.actor == "fleet:Laptop"),
        "audited on the node as fleet:<console>"
    );
    let console = fleet::verify_audit(&b).unwrap();
    assert!(
        console
            .iter()
            .any(|e| e["kind"] == "command_sent" && e["reason"] == "refused")
    );

    let _ = std::process::Command::new("/bin/kill")
        .args(["-9", &impostor.to_string()])
        .status();
    link::link_set_enabled_impl(&a, false).unwrap();
    link::link_set_enabled_impl(&b, false).unwrap();
}
