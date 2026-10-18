//! AJ-E702 integration: this Mac's identity and the signed peer
//! registry through the command layer — durable across restarts,
//! unpair revokes, tamper and rollback fail closed (and are reported,
//! never crash the app), reset trust / reset identity are explicit.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::fleet;
use juggler_app::state::AppState;
use juggler_link::{MachineIdentity, PairedVia, Peer, Permission};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn peer(dir: &Path, id: &str) -> Peer {
    let other = MachineIdentity::load_or_create(&dir.join("peers").join(id)).unwrap();
    Peer {
        machine_id: id.to_owned(),
        label: "Mac mini".to_owned(),
        public_key: other.public().to_base64(),
        fingerprint: other.public().fingerprint(),
        permission: Permission::View,
        ask_each_time: false,
        paired_via: PairedVia::Code,
        paired_at_ms: 1,
        last_seen_ms: None,
    }
}

#[test]
fn identity_is_durable_and_pins_survive_a_restart() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let first = fleet::fleet_self_impl(&state).unwrap();
    assert_eq!(first.trust, "trusted");
    assert_eq!(first.fingerprint.split(' ').count(), 8);
    assert!(!first.label.is_empty());
    fleet::fleet_pin_for_test(&state, peer(work.path(), "mini")).unwrap();
    drop(state);

    let reopened = app_state(work.path());
    let again = fleet::fleet_self_impl(&reopened).unwrap();
    assert_eq!(
        again.fingerprint, first.fingerprint,
        "same key after restart"
    );
    assert_eq!(again.machine_id, first.machine_id);
    let peers = fleet::fleet_peers_impl(&reopened).unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].permission, "view");
    assert_eq!(peers[0].permission_label, "View only");
    // Link is off until AJ-E704 turns it on — never a guessed state.
    assert_eq!(peers[0].connection, "link_off");
    assert!(peers[0].protocol.is_none());

    fleet::fleet_unpair_impl(&reopened, "mini").unwrap();
    assert!(fleet::fleet_peers_impl(&reopened).unwrap().is_empty());
    assert_eq!(
        fleet::fleet_unpair_impl(&reopened, "mini")
            .unwrap_err()
            .wire()
            .code,
        "not_found"
    );
}

#[test]
fn a_rolled_back_registry_is_reported_as_broken_trust_until_reset() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    fleet::fleet_pin_for_test(&state, peer(work.path(), "a")).unwrap();
    let registry = state.data_dir.join("fleet").join("peers.json");
    let old_copy = std::fs::read(&registry).unwrap();
    fleet::fleet_pin_for_test(&state, peer(work.path(), "b")).unwrap();
    // Restore the older, validly signed file and "restart".
    std::fs::write(&registry, &old_copy).unwrap();
    fleet::fleet_unload_for_test(&state);
    let info = fleet::fleet_self_impl(&state).unwrap();
    assert_eq!(info.trust, "broken");
    assert!(info.trust_reason.unwrap().contains("rollback"));
    assert!(
        fleet::fleet_peers_impl(&state).unwrap().is_empty(),
        "nothing is trusted"
    );
    // Writes are refused until the explicit reset.
    assert!(fleet::fleet_pin_for_test(&state, peer(work.path(), "c")).is_err());
    let reset = fleet::fleet_reset_trust_impl(&state).unwrap();
    assert_eq!(reset.trust, "trusted");
    fleet::fleet_pin_for_test(&state, peer(work.path(), "c")).unwrap();
}

#[test]
fn reset_identity_changes_the_key_and_drops_every_pin() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let before = fleet::fleet_self_impl(&state).unwrap();
    fleet::fleet_pin_for_test(&state, peer(work.path(), "a")).unwrap();
    let after = fleet::fleet_reset_identity_impl(&state).unwrap();
    assert_ne!(after.fingerprint, before.fingerprint);
    assert_eq!(after.peer_count, 0, "every peer must pair again");
    fleet::fleet_unload_for_test(&state);
    assert_eq!(
        fleet::fleet_self_impl(&state).unwrap().fingerprint,
        after.fingerprint
    );
    assert_eq!(fleet::fleet_self_impl(&state).unwrap().trust, "trusted");
}

#[test]
fn rename_is_local_and_validated() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let before = fleet::fleet_self_impl(&state).unwrap();
    let renamed = fleet::fleet_set_label_impl(&state, "  Studio Mac ").unwrap();
    assert_eq!(renamed.label, "Studio Mac");
    assert_eq!(renamed.fingerprint, before.fingerprint);
    assert_eq!(renamed.machine_id, before.machine_id);
    assert!(fleet::fleet_set_label_impl(&state, "   ").is_err());
    assert!(fleet::fleet_set_label_impl(&state, &"x".repeat(65)).is_err());
}

#[test]
fn the_private_key_never_appears_in_the_database_or_settings() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    fleet::fleet_self_impl(&state).unwrap();
    let key_file =
        std::fs::read_to_string(state.data_dir.join("fleet").join("identity.key")).unwrap();
    let secret = key_file.trim();
    for entry in std::fs::read_dir(&state.data_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let bytes = std::fs::read(&path).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains(secret), "{} holds the key", path.display());
        }
    }
}
