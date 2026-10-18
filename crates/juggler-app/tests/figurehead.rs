//! AJ-E1202 guardrails for the optional Figurehead account, through the
//! command layer with a recording transport and an in-memory secret
//! store (the real Keychain is never touched by tests):
//!
//! - signed out is silent — zero requests of any kind;
//! - browser linking and pairing-code linking end to end;
//! - the device token exists only in the secret store — never in the
//!   database, settings or any file under the data dir;
//! - `401` anywhere unlinks locally: token gone, toggles off, notice
//!   shown, local data unchanged; Unlink cleans up even offline;
//! - entitlements drive features (`404` → Phase-1 defaults; paid →
//!   Upgrade) and a feature runs only when entitled AND toggled on;
//! - polling never runs faster than the server's interval.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_account::transport::NetworkError;
use juggler_account::{MemoryStore, RecordingTransport, SecretStore, Transport};
use juggler_adapter::AdapterRegistry;
use juggler_app::figurehead;
use juggler_app::state::AppState;

const BASE: &str = "https://figurehead.test";
const LINK: &str = r#"{"requestId":"lnk_1","userCode":"QVM3-GABM","verifyUrl":"https://figurehead.test/link?code=QVM3-GABM","pollToken":"fhp_pollsecret","expiresIn":600,"interval":3}"#;
const ACCOUNT: &str = r#"{"user":{"email":"you@example.com","name":"You"},"device":{"id":"dev_1","name":"Studio Mac"},
  "plan":{"id":"free","name":"Free"},
  "entitlements":{
    "cloudBackup":{"enabled":true,"tier":"free","maxBackups":30},
    "fleetDiscovery":{"enabled":true,"tier":"free","maxDevices":null},
    "usageSync":{"enabled":true,"tier":"free"},
    "transcriptVault":{"enabled":false,"tier":"paid"},
    "alerts":{"enabled":false,"tier":"free","available":false}},
  "upgradeUrl":"https://figurehead.test/dashboard"}"#;

struct Fixture {
    _dir: tempfile::TempDir,
    data: std::path::PathBuf,
    state: Arc<AppState>,
    transport: Arc<RecordingTransport>,
    secrets: Arc<MemoryStore>,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("app-data");
    let state =
        Arc::new(AppState::init_with_registry(data.clone(), AdapterRegistry::new()).unwrap());
    let transport = Arc::new(RecordingTransport::default());
    let secrets = Arc::new(MemoryStore::default());
    figurehead::use_for_test(
        &state,
        transport.clone() as Arc<dyn Transport>,
        secrets.clone() as Arc<dyn SecretStore>,
        BASE,
    );
    Fixture {
        _dir: dir,
        data,
        state,
        transport,
        secrets,
    }
}

fn link_by_browser(f: &Fixture) {
    f.transport.respond(201, LINK);
    let started = figurehead::link_start_impl(&f.state, Some("Studio Mac")).unwrap();
    assert_eq!(started.state, "linking");
    assert_eq!(started.link.as_ref().unwrap().user_code, "QVM3-GABM");
    figurehead::poll_now_for_test(&f.state);
    f.transport.respond(
        200,
        r#"{"status":"approved","deviceId":"dev_1","token":"fhd_devicesecret"}"#,
    );
    f.transport.respond(200, ACCOUNT);
    let linked = figurehead::link_poll_impl(&f.state).unwrap();
    assert_eq!(linked.state, "linked", "{linked:?}");
}

/// Every byte under the data dir (DB, WAL, settings, logs…).
fn all_bytes(dir: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in walk(dir) {
        if let Ok(bytes) = std::fs::read(&entry) {
            out.extend(bytes);
        }
    }
    out
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

#[test]
fn signed_out_is_silent() {
    let f = fixture();
    // A scripted signed-out session: launch-time resume, status, every
    // local Fleet surface, a refresh and an unlink attempt.
    figurehead::resume_if_linked(Arc::clone(&f.state));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let status = figurehead::status_impl(&f.state).unwrap();
    assert_eq!(status.state, "signed_out");
    assert!(status.features.is_empty());
    juggler_app::fleet::fleet_self_impl(&f.state).unwrap();
    juggler_app::fleet::fleet_peers_impl(&f.state).unwrap();
    figurehead::refresh_impl(&f.state).unwrap();
    figurehead::link_poll_impl(&f.state).unwrap();
    figurehead::unlink_impl(&f.state).unwrap();
    for feature in figurehead::KNOWN_FEATURES {
        assert!(!figurehead::feature_on(&f.state, feature));
    }
    assert_eq!(
        f.transport.count(),
        0,
        "no request of any kind while signed out"
    );
}

#[test]
fn a_browser_link_stores_the_token_only_in_the_secret_store() {
    let f = fixture();
    link_by_browser(&f);
    let status = figurehead::status_impl(&f.state).unwrap();
    assert_eq!(status.email.as_deref(), Some("you@example.com"));
    assert_eq!(status.plan.as_deref(), Some("Free"));
    assert_eq!(status.device_name, "Studio Mac");
    let keys: Vec<_> = status
        .features
        .iter()
        .map(|f| (f.key.as_str(), f.access.as_str()))
        .collect();
    assert_eq!(
        keys,
        vec![
            ("cloudBackup", "included"),
            ("fleetDiscovery", "included"),
            ("usageSync", "included"),
            ("transcriptVault", "upgrade"),
        ],
        "unavailable features (alerts) are hidden; paid-and-missing shows Upgrade"
    );
    assert!(
        status.features.iter().all(|f| !f.on),
        "nothing is on before consent"
    );

    assert_eq!(f.secrets.get("dev_1").as_deref(), Some("fhd_devicesecret"));
    let bytes = all_bytes(&f.data);
    assert!(
        !contains(&bytes, "fhd_"),
        "the device token is never on disk outside the Keychain"
    );
    assert!(!contains(&bytes, "fhp_"), "neither is the poll token");

    // The link request carried this Mac's identity keys, not a secret.
    let sent: serde_json::Value = serde_json::from_slice(&f.transport.requests()[0].body).unwrap();
    let key = sent["publicKey"].as_str().unwrap();
    assert!(
        key.starts_with("ed25519:") && key.contains(";x25519:"),
        "{key}"
    );
    assert_eq!(sent["name"], "Studio Mac");
    // The account call used the device token.
    let account = &f.transport.requests()[2];
    assert_eq!(account.url, format!("{BASE}/api/v1/account"));
    assert_eq!(account.bearer.as_deref(), Some("fhd_devicesecret"));
}

#[test]
fn polling_never_runs_faster_than_the_interval() {
    let f = fixture();
    f.transport.respond(201, LINK);
    figurehead::link_start_impl(&f.state, None).unwrap();
    for _ in 0..5 {
        figurehead::link_poll_impl(&f.state).unwrap();
    }
    assert_eq!(
        f.transport.count(),
        1,
        "only the start: every early poll is answered locally"
    );
    figurehead::poll_now_for_test(&f.state);
    f.transport.respond(410, r#"{"status":"denied"}"#);
    let denied = figurehead::link_poll_impl(&f.state).unwrap();
    assert_eq!(denied.link.unwrap().phase, "denied");
    figurehead::link_cancel_impl(&f.state);
    assert_eq!(
        figurehead::status_impl(&f.state).unwrap().state,
        "signed_out"
    );
}

#[test]
fn a_pairing_code_links_and_a_wrong_one_says_so() {
    let f = fixture();
    f.transport
        .respond(400, r#"{"error":"invalid_or_expired_code"}"#);
    let error = figurehead::claim_impl(&f.state, "AAAA-BBBB", None).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("pairing code is wrong or has expired"),
        "{error}"
    );
    f.transport
        .respond(201, r#"{"deviceId":"dev_9","token":"fhd_claimed"}"#);
    f.transport.respond(404, r#"{"error":"not_found"}"#);
    let linked = figurehead::claim_impl(&f.state, "qvm3-gabm", Some("Mini")).unwrap();
    assert_eq!(linked.state, "linked");
    assert_eq!(f.secrets.get("dev_9").as_deref(), Some("fhd_claimed"));
    // `404` from /account: the Phase-1 defaults.
    let keys: Vec<_> = linked.features.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(keys, vec!["cloudBackup", "fleetDiscovery", "usageSync"]);
}

#[test]
fn a_401_unlinks_locally_and_keeps_local_data() {
    let f = fixture();
    link_by_browser(&f);
    let mut choices = figurehead::cloud_choices(&f.state);
    choices.features.insert("cloudBackup".into(), true);
    choices.features.insert("fleetDiscovery".into(), true);
    choices.consent_version = 1;
    figurehead::save_cloud_choices(&f.state, &choices).unwrap();
    assert!(figurehead::feature_on(&f.state, "cloudBackup"));
    juggler_app::fleet::fleet_set_label_impl(&f.state, "Kept label").unwrap();

    f.transport.respond(401, r#"{"error":"unauthorized"}"#);
    let error = figurehead::refresh_impl(&f.state).unwrap_err();
    assert!(error.to_string().contains("unlinked"), "{error}");
    let status = figurehead::status_impl(&f.state).unwrap();
    assert_eq!(status.state, "signed_out");
    assert_eq!(
        status.notice.as_deref(),
        Some("This Mac was unlinked from Figurehead.")
    );
    assert!(f.secrets.get("dev_1").is_none(), "the token is deleted");
    let after = figurehead::cloud_choices(&f.state);
    assert!(after.features.values().all(|on| !on), "every toggle is off");
    for feature in figurehead::KNOWN_FEATURES {
        assert!(!figurehead::feature_on(&f.state, feature));
    }
    assert_eq!(
        juggler_app::fleet::fleet_self_impl(&f.state).unwrap().label,
        "Kept label",
        "local data is untouched"
    );
    let dismissed = figurehead::dismiss_notice_impl(&f.state).unwrap();
    assert!(dismissed.notice.is_none());
}

#[test]
fn unlink_cleans_up_even_offline() {
    let f = fixture();
    link_by_browser(&f);
    for _ in 0..4 {
        f.transport.push(Err(NetworkError("offline".into())));
    }
    let status = figurehead::unlink_impl(&f.state).unwrap();
    assert_eq!(status.state, "signed_out");
    assert!(
        status.notice.is_none(),
        "a deliberate unlink needs no notice"
    );
    assert!(f.secrets.get("dev_1").is_none());
    let delete = f
        .transport
        .requests()
        .into_iter()
        .find(|r| r.method == "DELETE")
        .unwrap();
    assert_eq!(delete.url, format!("{BASE}/api/v1/devices/self"));
}

#[test]
fn a_feature_runs_only_when_entitled_and_turned_on() {
    let f = fixture();
    link_by_browser(&f);
    assert!(
        !figurehead::feature_on(&f.state, "cloudBackup"),
        "entitled but not turned on"
    );
    let mut choices = figurehead::cloud_choices(&f.state);
    choices.features.insert("cloudBackup".into(), true);
    choices.features.insert("transcriptVault".into(), true);
    figurehead::save_cloud_choices(&f.state, &choices).unwrap();
    assert!(figurehead::feature_on(&f.state, "cloudBackup"));
    assert!(
        !figurehead::feature_on(&f.state, "transcriptVault"),
        "a toggle can't turn on a feature the plan doesn't include"
    );
    let before = f.transport.count();
    figurehead::status_impl(&f.state).unwrap();
    assert_eq!(f.transport.count(), before, "status and gates are local");
}

#[test]
fn consent_stores_only_what_the_plan_and_this_build_allow() {
    let f = fixture();
    let before = figurehead::status_impl(&f.state).unwrap();
    assert!(
        figurehead::save_choices_impl(&f.state, &Default::default()).is_err(),
        "signed out"
    );
    assert_eq!(before.consent_version, 0);
    link_by_browser(&f);
    let status = figurehead::status_impl(&f.state).unwrap();
    assert!(
        status.consent_version < status.consent_current,
        "the consent screen is due"
    );
    let wanted: std::collections::BTreeMap<String, bool> = [
        ("cloudBackup", true),
        ("fleetDiscovery", true),
        ("usageSync", true),
        ("transcriptVault", true),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    let saved = figurehead::save_choices_impl(&f.state, &wanted).unwrap();
    assert_eq!(saved.consent_version, saved.consent_current);
    let on: Vec<_> = saved
        .features
        .iter()
        .filter(|f| f.on)
        .map(|f| f.key.as_str())
        .collect();
    assert_eq!(
        on,
        vec!["cloudBackup", "fleetDiscovery"],
        "usage sync waits for usage statistics; a paid-missing feature stays off"
    );
    assert!(
        figurehead::cloud_choices(&f.state)
            .consented_at_ms
            .is_some()
    );
    assert_eq!(
        f.transport.count(),
        3,
        "consent is local: start + poll + account only"
    );
}
