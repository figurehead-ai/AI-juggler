//! AJ-E1207 — the two-path law, one test per promise (ADR-0014). Every
//! test runs against a recording fake of figurehead.ai; none needs the
//! network.
//!
//! 1. Signed out is silent.
//! 2. Toggles mean it: a feature that's off never calls its endpoint.
//! 3. No plaintext leaves the Mac.
//! 4. No credentials: a backup never opens an Account folder or a
//!    Provider credential file.
//! 5. The device token lives only in the Keychain.
//! 6. A `401` unlinks locally: token gone, toggles off, local data kept.
//! 7. Entitlements drive behaviour: not in the plan means no call; a
//!    `402` stops uploads.
//! 8. The desktop path is unchanged (the full suite; see the manifest).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use juggler_account::transport::{HttpRequest, HttpResponse, NetworkError};
use juggler_account::{MemoryStore, SecretStore, Transport};
use juggler_adapter::AdapterRegistry;
use juggler_app::state::AppState;
use juggler_app::{cloud_backup, figurehead, fleet_account, vault};
use juggler_core::model::{Account, AccountId, HarnessId, LaunchRecipe, ProfileId};

const BASE: &str = "https://figurehead.test";
const TOKEN: &str = "fhd_guardrail_secret_token";

/// One recorded request: method, path, body.
type Seen = (String, String, Vec<u8>);

struct Recorder {
    seen: Mutex<Vec<Seen>>,
    entitlements: Mutex<String>,
    status_for_backups: Mutex<Option<u16>>,
}

impl Recorder {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            entitlements: Mutex::new(
                r#"{"cloudBackup":{"enabled":true,"tier":"free"},"fleetDiscovery":{"enabled":true,"tier":"free"},"usageSync":{"enabled":true,"tier":"free"}}"#
                    .to_owned(),
            ),
            status_for_backups: Mutex::new(None),
        })
    }
    fn paths(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(m, p, _)| format!("{m} {p}"))
            .collect()
    }
    fn calls_to(&self, prefix: &str) -> usize {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, p, _)| p.starts_with(prefix))
            .count()
    }
    fn uploads(&self) -> Vec<Vec<u8>> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, p, _)| m == "POST" && p == "backups")
            .map(|(_, _, body)| body.clone())
            .collect()
    }
}

fn reply(status: u16, body: &str) -> Result<HttpResponse, NetworkError> {
    Ok(HttpResponse {
        status,
        headers: Vec::new(),
        body: body.as_bytes().to_vec(),
    })
}

impl Transport for Recorder {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        let path = request
            .url
            .trim_start_matches(&format!("{BASE}/api/v1/"))
            .to_owned();
        self.seen.lock().unwrap().push((
            request.method.to_owned(),
            path.clone(),
            request.body.clone(),
        ));
        match (request.method, path.as_str()) {
            ("POST", "devices/claim") => {
                reply(201, &format!(r#"{{"deviceId":"dev_g","token":"{TOKEN}"}}"#))
            }
            ("GET", "account") => reply(
                200,
                &format!(
                    r#"{{"user":{{"email":"you@example.com"}},"device":{{"id":"dev_g","name":"Studio"}},"plan":{{"id":"free","name":"Free"}},"entitlements":{}}}"#,
                    self.entitlements.lock().unwrap()
                ),
            ),
            ("POST", "backups") => {
                if let Some(status) = *self.status_for_backups.lock().unwrap() {
                    return reply(status, r#"{"error":"refused","feature":"cloudBackup"}"#);
                }
                use sha2::Digest;
                let sha: String = sha2::Sha256::digest(&request.body)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                reply(
                    201,
                    &format!(
                        r#"{{"id":"bak_1","sha256":"{sha}","sizeBytes":{}}}"#,
                        request.body.len()
                    ),
                )
            }
            ("GET", "devices") => reply(200, r#"{"self":"dev_g","devices":[]}"#),
            ("GET", "backups") => reply(200, r#"{"backups":[]}"#),
            _ => reply(404, r#"{"error":"not_found"}"#),
        }
    }
}

struct Mac {
    _dir: tempfile::TempDir,
    data: PathBuf,
    state: Arc<AppState>,
    server: Arc<Recorder>,
    secrets: Arc<MemoryStore>,
    vault: MemoryStore,
}

fn mac() -> Mac {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("app-data");
    let state =
        Arc::new(AppState::init_with_registry(data.clone(), AdapterRegistry::new()).unwrap());
    let server = Recorder::new();
    let secrets = Arc::new(MemoryStore::default());
    figurehead::use_for_test(
        &state,
        server.clone() as Arc<dyn Transport>,
        secrets.clone() as Arc<dyn SecretStore>,
        BASE,
    );
    Mac {
        _dir: dir,
        data,
        state,
        server,
        secrets,
        vault: MemoryStore::default(),
    }
}

fn link(m: &Mac) {
    figurehead::claim_impl(&m.state, "ABCD-1234", Some("Studio")).unwrap();
}

fn turn(m: &Mac, features: &[(&str, bool)]) {
    let choices = features
        .iter()
        .map(|(k, v)| ((*k).to_owned(), *v))
        .collect();
    figurehead::save_choices_impl(&m.state, &choices).unwrap();
}

fn ready_vault(m: &Mac) {
    vault::vault_setup_impl(&m.state, &m.vault).unwrap();
    vault::vault_confirm_saved_impl(&m.state, &m.vault).unwrap();
}

/// Every cloud-facing call the app makes, attempted once.
fn try_everything(m: &Mac) {
    let _ = cloud_backup::cloud_backup_now_impl(&m.state, &m.vault);
    let _ = cloud_backup::cloud_backup_tick(&m.state, &m.vault);
    let _ = cloud_backup::cloud_backup_list_impl(&m.state);
    let _ = cloud_backup::cloud_backup_prepare_restore_impl(&m.state, &m.vault, "bak_1", None);
    let _ = fleet_account::fleet_account_macs_impl(&m.state);
    let _ = fleet_account::fleet_account_pin_impl(&m.state, "dev_x", "fp");
    let _ = figurehead::status_impl(&m.state);
}

fn all_bytes(dir: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = std::fs::read(&path) {
                out.extend(bytes);
            }
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

#[test]
fn g1_signed_out_is_silent() {
    let m = mac();
    ready_vault(&m);
    try_everything(&m);
    assert_eq!(
        m.server.paths(),
        Vec::<String>::new(),
        "no connection of any kind"
    );
}

#[test]
fn g2_a_feature_that_is_off_never_calls_its_endpoint() {
    let m = mac();
    link(&m);
    ready_vault(&m);
    turn(
        &m,
        &[
            ("cloudBackup", false),
            ("fleetDiscovery", false),
            ("usageSync", false),
        ],
    );
    let before = m.server.paths().len();
    try_everything(&m);
    assert_eq!(m.server.calls_to("backups"), 0, "backup off: no /backups");
    assert_eq!(
        m.server.calls_to("devices") - m.server.calls_to("devices/claim"),
        0,
        "discovery off: no /devices"
    );
    assert_eq!(
        m.server.calls_to("usage"),
        0,
        "usage sync off: no usage call"
    );
    assert_eq!(
        m.server.paths().len(),
        before,
        "nothing at all: {:?}",
        m.server.paths()
    );
}

#[test]
fn g3_g4_no_plaintext_and_no_credentials_leave_the_mac() {
    let m = mac();
    link(&m);
    ready_vault(&m);
    turn(&m, &[("cloudBackup", true)]);
    // Seeds: an Account label, a project path, a fake $HOME, and sign-in
    // material in the Account folder.
    let home = m.data.parent().unwrap().join("fake-home-u9q2");
    let state_dir = m.data.join("accounts").join("acc-g");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        state_dir.join(".credentials.json"),
        r#"{"oauth":"sk-ant-oat01-credential-never-read"}"#,
    )
    .unwrap();
    {
        let storage = m.state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-g"),
                harness: HarnessId::new("claude-code"),
                label: "Acme Legal Team".into(),
                plan: None,
                state_dir: state_dir.clone(),
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
                id: ProfileId::new("rec-g"),
                name: "Main".into(),
                account: AccountId::new("acc-g"),
                settings_overlay: Default::default(),
                default_cwd: None,
            })
            .unwrap();
        storage
            .projects()
            .touch(&home.join("clients/acme-merger"), None)
            .unwrap();
    }
    cloud_backup::cloud_backup_now_impl(&m.state, &m.vault).unwrap();
    let uploads = m.server.uploads();
    assert_eq!(uploads.len(), 1);
    let body = &uploads[0];
    assert_eq!(&body[..4], b"FHB1", "only the sealed container leaves");
    for seeded in [
        "Acme Legal Team",
        "acme-merger",
        "fake-home-u9q2",
        "sk-ant-oat01",
        "credential-never-read",
    ] {
        assert!(!contains(body, seeded), "{seeded} must not leave the Mac");
    }
    // G4: the export opened no Account folder and no credential file.
    let reads = m.state.export_reads.lock().unwrap().clone();
    assert!(!reads.is_empty());
    for read in &reads {
        assert!(
            !read.starts_with(m.data.join("accounts")),
            "opened {}",
            read.display()
        );
        assert!(
            !read.starts_with(m.data.join("fleet")),
            "opened {}",
            read.display()
        );
        let name = read.file_name().unwrap().to_string_lossy();
        assert!(
            !name.contains("credential") && !name.contains(".claude"),
            "opened {}",
            read.display()
        );
    }
}

#[test]
fn g5_the_device_token_exists_only_in_the_keychain() {
    let m = mac();
    link(&m);
    ready_vault(&m);
    turn(&m, &[("cloudBackup", true), ("fleetDiscovery", true)]);
    try_everything(&m);
    assert_eq!(
        m.secrets.get("dev_g").as_deref(),
        Some(TOKEN),
        "in the (test) Keychain"
    );
    assert!(
        !contains(&all_bytes(&m.data), "fhd_"),
        "never in the database, settings or any file"
    );
    for body in m.server.uploads() {
        assert!(!contains(&body, "fhd_"), "never in an uploaded body");
    }
}

#[test]
fn g6_a_401_unlinks_locally_and_keeps_local_data() {
    let m = mac();
    link(&m);
    ready_vault(&m);
    turn(&m, &[("cloudBackup", true), ("fleetDiscovery", true)]);
    let projects_before = m
        .state
        .storage
        .lock()
        .unwrap()
        .projects()
        .list()
        .unwrap()
        .len();
    *m.server.status_for_backups.lock().unwrap() = Some(401);
    assert!(cloud_backup::cloud_backup_now_impl(&m.state, &m.vault).is_err());
    assert!(m.secrets.get("dev_g").is_none(), "token removed");
    let status = figurehead::status_impl(&m.state).unwrap();
    assert!(status.features.iter().all(|f| !f.on), "every toggle off");
    assert_eq!(
        m.state
            .storage
            .lock()
            .unwrap()
            .projects()
            .list()
            .unwrap()
            .len(),
        projects_before
    );
    // And now silent again.
    let before = m.server.paths().len();
    try_everything(&m);
    assert_eq!(m.server.paths().len(), before);
}

#[test]
fn g7_entitlements_drive_behaviour() {
    let m = mac();
    *m.server.entitlements.lock().unwrap() =
        r#"{"cloudBackup":{"enabled":false,"tier":"paid"},"fleetDiscovery":{"enabled":false,"tier":"paid"}}"#.to_owned();
    link(&m);
    ready_vault(&m);
    turn(&m, &[("cloudBackup", true), ("fleetDiscovery", true)]);
    let status = figurehead::status_impl(&m.state).unwrap();
    let backup = status
        .features
        .iter()
        .find(|f| f.key == "cloudBackup")
        .unwrap();
    assert_eq!(backup.access, "upgrade", "paid and not included: Upgrade");
    assert!(!backup.on, "not stored as on");
    try_everything(&m);
    assert_eq!(m.server.calls_to("backups"), 0);
    assert_eq!(
        m.server.calls_to("devices") - m.server.calls_to("devices/claim"),
        0
    );

    // Included, but the service says 402 at upload: it stops, named.
    let m = mac();
    link(&m);
    ready_vault(&m);
    turn(&m, &[("cloudBackup", true)]);
    *m.server.status_for_backups.lock().unwrap() = Some(402);
    let refused = cloud_backup::cloud_backup_now_impl(&m.state, &m.vault)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("plan doesn't include"), "{refused}");
}
