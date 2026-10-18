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
use juggler_app::state::AppState;
use juggler_app::{fleet, link};
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
