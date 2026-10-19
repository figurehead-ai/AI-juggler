//! AJ-E1206 integration against a fake Figurehead directory: with Fleet
//! discovery off, no `/devices` call; on, your other Macs are listed (at
//! most one read a minute); a Mac is pinned only after its fingerprint
//! is confirmed here, starting at View and labelled as account-paired; a
//! key that changed after the comparison (substitution) is never pinned;
//! the peer's real id binds once on its first Hello.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::sync::{Arc, Mutex};

use juggler_account::transport::{HttpRequest, HttpResponse, NetworkError};
use juggler_account::{MemoryStore, SecretStore, Transport};
use juggler_adapter::AdapterRegistry;
use juggler_app::state::AppState;
use juggler_app::{figurehead, fleet, fleet_account};
use juggler_link::{MachineIdentity, PublicIdentity};

const BASE: &str = "https://figurehead.test";

#[derive(Default)]
struct Directory {
    devices: Mutex<String>,
    log: Mutex<Vec<String>>,
}

impl Transport for Directory {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        let path = request
            .url
            .trim_start_matches(&format!("{BASE}/api/v1/"))
            .to_owned();
        self.log.lock().unwrap().push(path.clone());
        let body = match (request.method, path.as_str()) {
            ("POST", "devices/claim") => r#"{"deviceId":"dev_this","token":"fhd_this"}"#.to_owned(),
            ("GET", "account") => r#"{"user":{"email":"you@example.com"},"device":{"id":"dev_this","name":"Studio"},"plan":{"id":"free","name":"Free"},"entitlements":{"fleetDiscovery":{"enabled":true,"tier":"free"}}}"#.to_owned(),
            ("GET", "devices") => self.devices.lock().unwrap().clone(),
            _ => return Ok(HttpResponse { status: 404, headers: Vec::new(), body: b"{}".to_vec() }),
        };
        Ok(HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: body.into_bytes(),
        })
    }
}

impl Directory {
    fn device_calls(&self) -> usize {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.as_str() == "devices")
            .count()
    }
    fn list(&self, entries: &[(&str, &str, Option<&PublicIdentity>)]) {
        let rows: Vec<String> = entries
            .iter()
            .map(|(id, name, key)| {
                let key = key.map_or("null".to_owned(), |k| format!(r#""ed25519:{};x25519:AAAA""#, k.to_base64()));
                format!(r#"{{"id":"{id}","name":"{name}","platform":"macos","appVersion":"0.1.0","publicKey":{key},"lastSeenAt":"2026-10-18T10:00:00Z"}}"#)
            })
            .collect();
        *self.devices.lock().unwrap() =
            format!(r#"{{"self":"dev_this","devices":[{}]}}"#, rows.join(","));
    }
}

fn linked(dir: &std::path::Path, server: &Arc<Directory>) -> Arc<AppState> {
    let state =
        Arc::new(AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap());
    figurehead::use_for_test(
        &state,
        server.clone() as Arc<dyn Transport>,
        Arc::new(MemoryStore::default()) as Arc<dyn SecretStore>,
        BASE,
    );
    figurehead::claim_impl(&state, "ABCD-1234", Some("Studio")).unwrap();
    state
}

fn discovery(state: &Arc<AppState>, on: bool) {
    let choices = [("fleetDiscovery".to_owned(), on)].into_iter().collect();
    figurehead::save_choices_impl(state, &choices).unwrap();
}

#[test]
fn off_means_no_devices_call() {
    let work = tempfile::tempdir().unwrap();
    let server = Arc::new(Directory::default());
    let state = linked(work.path(), &server);
    assert!(fleet_account::fleet_account_macs_impl(&state).is_err());
    assert!(fleet_account::fleet_account_pin_impl(&state, "dev_mini", "x").is_err());
    assert_eq!(server.device_calls(), 0);
}

#[test]
fn listed_macs_pin_only_on_a_confirmed_fingerprint_and_bind_on_hello() {
    let work = tempfile::tempdir().unwrap();
    let server = Arc::new(Directory::default());
    let state = linked(&work.path().join("app"), &server);
    discovery(&state, true);
    let mini = MachineIdentity::load_or_create(&work.path().join("mini"))
        .unwrap()
        .public();
    let attacker = MachineIdentity::load_or_create(&work.path().join("attacker"))
        .unwrap()
        .public();
    let this_key = PublicIdentity::from_bytes(&fleet::local_public_key(&state).unwrap()).unwrap();
    server.list(&[
        ("dev_this", "Studio", Some(&this_key)),
        ("dev_mini", "Mac mini", Some(&mini)),
        ("dev_old", "Old laptop", None),
    ]);

    let listed = fleet_account::fleet_account_macs_impl(&state).unwrap();
    assert_eq!(listed.this_fingerprint, this_key.fingerprint());
    let names: Vec<(&str, &str)> = listed
        .macs
        .iter()
        .map(|m| (m.name.as_str(), m.state.as_str()))
        .collect();
    assert_eq!(
        names,
        [("Mac mini", "not_paired"), ("Old laptop", "no_key")],
        "this Mac is not listed"
    );
    assert_eq!(
        listed.macs[0].fingerprint.as_deref(),
        Some(mini.fingerprint().as_str())
    );
    // At most one read a minute.
    fleet_account::fleet_account_macs_impl(&state).unwrap();
    assert_eq!(server.device_calls(), 1);

    // Nothing is pinned by listing, or by a wrong confirmation.
    assert!(fleet::fleet_peers_impl(&state).unwrap().is_empty());
    let refused =
        fleet_account::fleet_account_pin_impl(&state, "dev_mini", &attacker.fingerprint())
            .unwrap_err()
            .to_string();
    assert!(refused.contains("fingerprint changed"), "{refused}");
    assert!(
        fleet_account::fleet_account_pin_impl(&state, "dev_old", "anything").is_err(),
        "no key, no pin"
    );
    assert!(fleet::fleet_peers_impl(&state).unwrap().is_empty());

    // Confirmed: pinned at View, account-paired, waiting for the other Mac.
    fleet_account::fleet_account_pin_impl(&state, "dev_mini", &mini.fingerprint()).unwrap();
    let peers = fleet::fleet_peers_impl(&state).unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].paired_via, "account");
    assert_eq!(peers[0].permission, "view");
    assert_eq!(peers[0].fingerprint, mini.fingerprint());
    assert!(
        peers[0]
            .machine_id
            .starts_with(juggler_link::PROVISIONAL_PREFIX)
    );
    assert_eq!(
        fleet_account::fleet_account_macs_impl(&state).unwrap().macs[0].state,
        "waiting"
    );

    // Its first Hello binds the real id — only for that key, only once.
    assert!(fleet::bind_account_peer(&state, &attacker, "mini-real").is_none());
    let bound = fleet::bind_account_peer(&state, &mini, "mini-real").unwrap();
    assert_eq!(bound.machine_id, "mini-real");
    assert!(fleet::bind_account_peer(&state, &mini, "mini-again").is_none());
    assert_eq!(
        fleet::fleet_peers_impl(&state).unwrap()[0].machine_id,
        "mini-real"
    );
    assert_eq!(
        fleet_account::fleet_account_macs_impl(&state).unwrap().macs[0].state,
        "paired"
    );
}

#[test]
fn a_key_swapped_after_the_comparison_is_never_pinned() {
    let work = tempfile::tempdir().unwrap();
    let server = Arc::new(Directory::default());
    let state = linked(&work.path().join("app"), &server);
    discovery(&state, true);
    let mini = MachineIdentity::load_or_create(&work.path().join("mini"))
        .unwrap()
        .public();
    let attacker = MachineIdentity::load_or_create(&work.path().join("attacker"))
        .unwrap()
        .public();
    server.list(&[("dev_mini", "Mac mini", Some(&mini))]);
    let shown = fleet_account::fleet_account_macs_impl(&state).unwrap().macs[0]
        .fingerprint
        .clone()
        .unwrap();
    // The directory now offers an attacker's key for the same Mac.
    server.list(&[("dev_mini", "Mac mini", Some(&attacker))]);
    if let Ok(mut cache) = state.account_directory.lock() {
        *cache = None; // the next read sees the swap
    }
    let refused = fleet_account::fleet_account_pin_impl(&state, "dev_mini", &shown)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("compare it again"), "{refused}");
    assert!(
        fleet::fleet_peers_impl(&state).unwrap().is_empty(),
        "the attacker's key is not pinned"
    );
}
