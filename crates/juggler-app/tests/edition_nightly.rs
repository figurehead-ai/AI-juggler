//! D-23: running as AI Juggler Nightly. Its own process (an integration test
//! binary), so fixing the edition here can't leak into AI Juggler's
//! tests. Everything this edition touches is its own: data folder,
//! Keychain items, Fleet port, update channel.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::edition::{self, Edition};
use juggler_app::state::{self, AppState};
use juggler_app::updates::{self, Channel, UpdateManager};

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut state =
        AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap();
    state.updates = UpdateManager::for_test(true);
    Arc::new(state)
}

#[test]
fn nightly_has_its_own_data_keychain_port_and_channel() {
    edition::set_current(Edition::Nightly);
    assert_eq!(edition::current(), Edition::Nightly);

    // Its own data folder: never AI Juggler's.
    let data = state::resolve_data_dir().unwrap();
    assert_eq!(data.file_name().unwrap(), "AI Juggler Nightly");
    assert_ne!(data.file_name().unwrap(), "AI Juggler");

    // Its own Keychain items and Fleet port.
    assert_ne!(
        edition::current().keychain().service(),
        juggler_account::secrets::KEYCHAIN_SERVICE
    );
    assert_ne!(
        edition::current().vault_keychain().service(),
        juggler_account::vault::VAULT_SERVICE
    );
    assert_eq!(juggler_app::link::preferred_port(), 47_822);

    // Its channel, whatever was stored (a copied-over settings value
    // can't move it onto another edition's feed).
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let status = updates::update_status_impl(&state);
    assert_eq!(status.edition, "nightly");
    assert_eq!(status.app_name, "AI Juggler Nightly");
    assert_eq!(status.channel, "nightly");
    assert_eq!(status.channels, ["nightly"]);
    for other in ["beta", "stable"] {
        let refused = updates::update_set_channel_impl(&state, other).unwrap_err();
        assert!(
            refused
                .to_string()
                .contains("always follows the nightly channel"),
            "{refused}"
        );
    }
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set("update_channel", "beta")
        .unwrap();
    assert_eq!(updates::update_status_impl(&state).channel, "nightly");
    assert_eq!(updates::current_channel(), Channel::Nightly);
}
