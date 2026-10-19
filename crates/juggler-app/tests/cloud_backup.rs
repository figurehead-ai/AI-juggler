//! AJ-E1205 integration against a fake Figurehead server (the API
//! contract's backup routes): upload → list → download → open → AJ-E1009
//! verify and restore on a second Mac with only the recovery code; with
//! Cloud backup off, or the recovery code unsaved, no `/backups` request
//! is ever made; `413`, `402` and the plan's size limit stop an upload
//! with a reason and delete nothing local; a download whose bytes don't
//! match `X-Backup-Sha256` is refused; the schedule only runs when due.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use juggler_account::transport::{HttpRequest, HttpResponse, NetworkError};
use juggler_account::{MemoryStore, SecretStore, Transport, VaultKey};
use juggler_adapter::AdapterRegistry;
use juggler_app::state::AppState;
use juggler_app::{backup, cloud_backup, figurehead, vault};
use sha2::Digest;

const BASE: &str = "https://figurehead.test";

fn account_json(max_bytes: Option<u64>) -> String {
    let limit = max_bytes.map_or(String::new(), |b| format!(r#","maxBytes":{b}"#));
    format!(
        r#"{{"user":{{"email":"you@example.com"}},"device":{{"id":"dev_{{ID}}","name":"Mac"}},"plan":{{"id":"free","name":"Free"}},
        "entitlements":{{"cloudBackup":{{"enabled":true,"tier":"free","maxBackups":30{limit}}}}}}}"#
    )
}

/// A fake Figurehead: links by pairing code, stores uploaded blobs, and
/// serves them back with their checksum.
#[derive(Default)]
struct FakeServer {
    device: Mutex<String>,
    max_bytes: Mutex<Option<u64>>,
    blobs: Mutex<Vec<(String, Vec<u8>, String)>>,
    log: Mutex<Vec<String>>,
    next_status: Mutex<Option<u16>>,
    corrupt_downloads: AtomicBool,
}

fn hex(bytes: &[u8]) -> String {
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn json(status: u16, body: &str) -> Result<HttpResponse, NetworkError> {
    Ok(HttpResponse {
        status,
        headers: Vec::new(),
        body: body.as_bytes().to_vec(),
    })
}

impl Transport for FakeServer {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        let path = request
            .url
            .trim_start_matches(&format!("{BASE}/api/v1/"))
            .to_owned();
        self.log
            .lock()
            .unwrap()
            .push(format!("{} {path}", request.method));
        if path.starts_with("backups")
            && let Some(status) = self.next_status.lock().unwrap().take()
        {
            return json(status, r#"{"error":"refused","feature":"cloudBackup"}"#);
        }
        let device = self.device.lock().unwrap().clone();
        match (request.method, path.as_str()) {
            ("POST", "devices/claim") => json(
                201,
                &format!(r#"{{"deviceId":"{device}","token":"fhd_{device}"}}"#),
            ),
            ("GET", "account") => json(
                200,
                &account_json(*self.max_bytes.lock().unwrap()).replace("{ID}", &device),
            ),
            ("POST", "backups") => {
                assert_eq!(request.content_type, "application/octet-stream");
                assert!(request.long_transfer);
                let mut blobs = self.blobs.lock().unwrap();
                let id = format!("bak_{}", blobs.len() + 1);
                let created = format!("2026-10-18T12:00:0{}Z", blobs.len());
                blobs.push((id.clone(), request.body.clone(), created));
                json(
                    201,
                    &format!(
                        r#"{{"id":"{id}","sha256":"{}","sizeBytes":{}}}"#,
                        hex(&request.body),
                        request.body.len()
                    ),
                )
            }
            ("GET", "backups") => {
                let rows: Vec<String> = self
                    .blobs
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(id, body, created)| {
                        format!(
                            r#"{{"id":"{id}","label":"AI Juggler","sizeBytes":{},"sha256":"{}","deviceId":"dev_a","createdAt":"{created}"}}"#,
                            body.len(),
                            hex(body)
                        )
                    })
                    .collect();
                json(200, &format!(r#"{{"backups":[{}]}}"#, rows.join(",")))
            }
            ("GET", other) if other.starts_with("backups/") => {
                let id = other.trim_start_matches("backups/");
                let blobs = self.blobs.lock().unwrap();
                let Some((_, body, _)) = blobs.iter().find(|(blob_id, _, _)| blob_id == id) else {
                    return json(404, r#"{"error":"not_found"}"#);
                };
                let mut sent = body.clone();
                if self.corrupt_downloads.load(Ordering::SeqCst) {
                    sent[40] ^= 1;
                }
                Ok(HttpResponse {
                    status: 200,
                    headers: vec![("x-backup-sha256".to_owned(), hex(body))],
                    body: sent,
                })
            }
            _ => json(404, r#"{"error":"not_found"}"#),
        }
    }
}

impl FakeServer {
    fn backup_calls(&self) -> usize {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|line| line.contains("backups"))
            .count()
    }
}

struct Mac {
    state: Arc<AppState>,
    vault: MemoryStore,
}

fn mac(dir: &Path, server: &Arc<FakeServer>, device: &str) -> Mac {
    let state =
        Arc::new(AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap());
    figurehead::use_for_test(
        &state,
        server.clone() as Arc<dyn Transport>,
        Arc::new(MemoryStore::default()) as Arc<dyn SecretStore>,
        BASE,
    );
    *server.device.lock().unwrap() = device.to_owned();
    figurehead::claim_impl(&state, "ABCD-1234", Some("Mac")).unwrap();
    Mac {
        state,
        vault: MemoryStore::default(),
    }
}

fn turn_on(mac: &Mac, on: bool) {
    let choices = [("cloudBackup".to_owned(), on)].into_iter().collect();
    figurehead::save_choices_impl(&mac.state, &choices).unwrap();
}

#[test]
fn off_or_unsaved_code_means_no_backups_request_at_all() {
    let work = tempfile::tempdir().unwrap();
    let server = Arc::new(FakeServer::default());
    let a = mac(&work.path().join("a"), &server, "dev_a");
    // Linked, Cloud backup off.
    assert!(cloud_backup::cloud_backup_now_impl(&a.state, &a.vault).is_err());
    assert!(cloud_backup::cloud_backup_list_impl(&a.state).is_err());
    assert!(
        cloud_backup::cloud_backup_prepare_restore_impl(&a.state, &a.vault, "bak_1", None).is_err()
    );
    assert!(!cloud_backup::cloud_backup_tick(&a.state, &a.vault));
    assert_eq!(server.backup_calls(), 0, "off means no /backups call");
    // On, but the recovery code isn't saved yet.
    turn_on(&a, true);
    vault::vault_setup_impl(&a.state, &a.vault).unwrap();
    let refused = cloud_backup::cloud_backup_now_impl(&a.state, &a.vault)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("recovery code"), "{refused}");
    assert!(!cloud_backup::cloud_backup_tick(&a.state, &a.vault));
    assert_eq!(
        server.backup_calls(),
        0,
        "no upload before the code is saved"
    );
    assert_eq!(
        cloud_backup::cloud_backup_status_impl(&a.state, &a.vault).schedule,
        "weekly"
    );
}

#[test]
fn round_trip_two_macs_with_the_recovery_code() {
    let work = tempfile::tempdir().unwrap();
    let server = Arc::new(FakeServer::default());
    let a = mac(&work.path().join("a"), &server, "dev_a");
    a.state
        .storage
        .lock()
        .unwrap()
        .projects()
        .touch(Path::new("/work/storefront"), None)
        .unwrap();
    turn_on(&a, true);
    let code = vault::vault_setup_impl(&a.state, &a.vault).unwrap().code;
    vault::vault_confirm_saved_impl(&a.state, &a.vault).unwrap();
    let local_before = std::fs::read_dir(&a.state.data_dir).unwrap().count();
    let stored = cloud_backup::cloud_backup_now_impl(&a.state, &a.vault).unwrap();
    assert!(stored.ok && stored.size_bytes.unwrap() > 0);
    assert_eq!(
        std::fs::read_dir(&a.state.data_dir).unwrap().count(),
        local_before,
        "its scratch files are gone; nothing else changed"
    );
    // What reached the server is ciphertext.
    let blob = server.blobs.lock().unwrap()[0].1.clone();
    assert_eq!(&blob[..4], b"FHB1");
    assert!(!blob.windows(10).any(|w| w == b"storefront"));
    let status = cloud_backup::cloud_backup_status_impl(&a.state, &a.vault);
    assert!(status.last.unwrap().ok);

    // A new Mac: linked, Cloud backup on, only the recovery code.
    let b = mac(&work.path().join("b"), &server, "dev_b");
    turn_on(&b, true);
    let list = cloud_backup::cloud_backup_list_impl(&b.state).unwrap();
    assert_eq!(list.len(), 1);
    assert!(!list[0].this_mac);
    let refused =
        cloud_backup::cloud_backup_prepare_restore_impl(&b.state, &b.vault, &list[0].id, None)
            .unwrap_err()
            .to_string();
    assert!(refused.contains("enter your recovery code"), "{refused}");
    let started = std::time::Instant::now();
    let plan = cloud_backup::cloud_backup_prepare_restore_impl(
        &b.state,
        &b.vault,
        &list[0].id,
        Some(&code),
    )
    .unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert!(plan.other_machine);
    assert_eq!(
        VaultKey::load(&b.vault).unwrap(),
        VaultKey::from_recovery_code(&code).unwrap(),
        "kept for next time"
    );
    let restored =
        backup::backup_restore_impl(&b.state, Path::new(&plan.path), &plan.token).unwrap();
    assert_eq!(restored.counts, plan.manifest.counts);
    assert_eq!(restored.counts["projects"], 1);

    // A download that doesn't match its checksum is refused.
    server.corrupt_downloads.store(true, Ordering::SeqCst);
    let refused =
        cloud_backup::cloud_backup_prepare_restore_impl(&b.state, &b.vault, &list[0].id, None)
            .unwrap_err()
            .to_string();
    assert!(refused.contains("changed on the way"), "{refused}");
}

#[test]
fn too_large_and_plan_refusals_stop_with_a_reason_and_keep_local_data() {
    let work = tempfile::tempdir().unwrap();
    let server = Arc::new(FakeServer::default());
    let a = mac(&work.path().join("a"), &server, "dev_a");
    turn_on(&a, true);
    vault::vault_setup_impl(&a.state, &a.vault).unwrap();
    vault::vault_confirm_saved_impl(&a.state, &a.vault).unwrap();
    let db = a.state.data_dir.join("juggler.sqlite");
    let db_len = std::fs::metadata(&db).unwrap().len();

    *server.next_status.lock().unwrap() = Some(413);
    let refused = cloud_backup::cloud_backup_now_impl(&a.state, &a.vault)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("larger than your plan allows"),
        "{refused}"
    );
    *server.next_status.lock().unwrap() = Some(402);
    let refused = cloud_backup::cloud_backup_now_impl(&a.state, &a.vault)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("plan doesn't include"), "{refused}");
    let last = cloud_backup::cloud_backup_status_impl(&a.state, &a.vault)
        .last
        .unwrap();
    assert!(!last.ok && last.error.is_some(), "the failure is shown");

    // The plan's limit is enforced before anything is sent.
    *server.max_bytes.lock().unwrap() = Some(64);
    figurehead::refresh_impl(&a.state).unwrap();
    let calls = server.backup_calls();
    let refused = cloud_backup::cloud_backup_now_impl(&a.state, &a.vault)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("nothing was uploaded"), "{refused}");
    assert_eq!(server.backup_calls(), calls, "never sent over the limit");
    assert!(
        std::fs::metadata(&db).unwrap().len() >= db_len,
        "local data is never deleted"
    );
}

#[test]
fn the_schedule_runs_only_when_due() {
    let work = tempfile::tempdir().unwrap();
    let server = Arc::new(FakeServer::default());
    let a = mac(&work.path().join("a"), &server, "dev_a");
    turn_on(&a, true);
    vault::vault_setup_impl(&a.state, &a.vault).unwrap();
    vault::vault_confirm_saved_impl(&a.state, &a.vault).unwrap();
    cloud_backup::cloud_backup_set_schedule_impl(&a.state, &a.vault, "daily").unwrap();
    assert!(cloud_backup::cloud_backup_set_schedule_impl(&a.state, &a.vault, "hourly").is_err());
    assert!(
        cloud_backup::cloud_backup_tick(&a.state, &a.vault),
        "never backed up: due"
    );
    assert_eq!(server.blobs.lock().unwrap().len(), 1);
    assert!(
        !cloud_backup::cloud_backup_tick(&a.state, &a.vault),
        "just backed up: not due"
    );
    cloud_backup::cloud_backup_set_schedule_impl(&a.state, &a.vault, "off").unwrap();
    assert!(!cloud_backup::cloud_backup_tick(&a.state, &a.vault));
    assert_eq!(server.blobs.lock().unwrap().len(), 1);
}
