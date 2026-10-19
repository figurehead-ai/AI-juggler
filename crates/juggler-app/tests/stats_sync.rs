//! AJ-E1109: the personal stats sync end to end in the app — a linked
//! Mac that shares and turned the sync on sends a fully covered week to
//! its account with the device token, and never also anonymously; a
//! `401` unlinks, keeps the report here and sends nothing else.

#![cfg(feature = "stats")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use juggler_account::transport::{HttpRequest, HttpResponse, NetworkError};
use juggler_account::{MemoryStore, SecretStore, Transport};
use juggler_adapter::AdapterRegistry;
use juggler_app::state::AppState;
use juggler_app::{figurehead, usage_stats};

const BASE: &str = "https://figurehead.test";
const TOKEN: &str = "fhd_sync_secret_token";

/// (method, url, bearer).
type Seen = (String, String, Option<String>);

struct Server {
    seen: Mutex<Vec<Seen>>,
    stats_status: Mutex<u16>,
}

fn reply(status: u16, body: &str) -> Result<HttpResponse, NetworkError> {
    Ok(HttpResponse {
        status,
        headers: Vec::new(),
        body: body.as_bytes().to_vec(),
    })
}

impl Transport for Server {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        self.seen.lock().unwrap().push((
            request.method.to_owned(),
            request.url.clone(),
            request.bearer.clone(),
        ));
        let path = request.url.trim_start_matches(&format!("{BASE}/api/v1/"));
        match (request.method, path) {
            ("POST", "devices/claim") => {
                reply(201, &format!(r#"{{"deviceId":"dev_s","token":"{TOKEN}"}}"#))
            }
            ("GET", "account") => reply(
                200,
                r#"{"user":{"email":"you@example.com"},"device":{"id":"dev_s","name":"Studio"},"plan":{"id":"free","name":"Free"},"entitlements":{"usageSync":{"enabled":true,"tier":"free"}}}"#,
            ),
            ("POST", "stats/reports") => {
                let status = *self.stats_status.lock().unwrap();
                reply(
                    status,
                    if status == 401 {
                        r#"{"error":"unauthorized"}"#
                    } else {
                        r#"{"ok":true}"#
                    },
                )
            }
            _ => reply(404, r#"{"error":"not_found"}"#),
        }
    }
}

struct Mac {
    _dir: tempfile::TempDir,
    data: PathBuf,
    state: Arc<AppState>,
    server: Arc<Server>,
}

fn sharing_mac_with_sync(stats_status: u16) -> Mac {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("app-data");
    let state =
        Arc::new(AppState::init_with_registry(data.clone(), AdapterRegistry::new()).unwrap());
    let server = Arc::new(Server {
        seen: Mutex::new(Vec::new()),
        stats_status: Mutex::new(stats_status),
    });
    figurehead::use_for_test(
        &state,
        server.clone() as Arc<dyn Transport>,
        Arc::new(MemoryStore::default()) as Arc<dyn SecretStore>,
        BASE,
    );
    figurehead::claim_impl(&state, "ABCD-1234", Some("Studio")).unwrap();
    usage_stats::usage_stats_set_impl(&state, "share").unwrap();
    let overview = usage_stats::usage_stats_set_sync_impl(&state, true).unwrap();
    assert!(overview.sync.offered && overview.sync.on);
    assert_eq!(overview.sync.email.as_deref(), Some("you@example.com"));
    Mac {
        _dir: dir,
        data,
        state,
        server,
    }
}

/// Last week, fully shared and synced, with one counter.
fn seed_last_week(m: &Mac) -> String {
    let now = juggler_core::model::Timestamp::now().millis();
    let last = juggler_stats::week::label(now - 7 * 86_400_000);
    let store = juggler_stats::Store::open(&m.data.join("stats")).unwrap();
    store.set_meta("share_since", "0").unwrap();
    store.set_meta("sync_since", "0").unwrap();
    store.add(&last, "app.launch", "{}", 2).unwrap();
    last
}

fn run_week(m: &Mac, last: &str) -> juggler_stats::cycle::TickOutcome {
    let env = juggler_stats::Environment::default();
    let envelope = juggler_stats::report::Envelope {
        app_version: "0.1.0".to_owned(),
        ..usage_stats::enabled::envelope()
    };
    let transport = usage_stats::enabled::StatsTransport(m.server.clone() as Arc<dyn Transport>);
    let closed = juggler_stats::week::end_ms(last).unwrap();
    usage_stats::enabled::tick(&m.state, &env, &envelope, &transport, closed).unwrap();
    let store = juggler_stats::Store::open(&m.data.join("stats")).unwrap();
    let now = juggler_core::model::Timestamp::now().millis();
    store
        .set_meta(&format!("upload_at:{last}"), &(now - 1).to_string())
        .unwrap();
    usage_stats::enabled::tick(&m.state, &env, &envelope, &transport, now).unwrap()
}

#[test]
fn a_synced_week_goes_to_the_account_with_the_token_and_never_anonymously() {
    let m = sharing_mac_with_sync(202);
    let last = seed_last_week(&m);
    let out = run_week(&m, &last);
    assert_eq!(
        out.uploads,
        vec![(last.clone(), juggler_stats::cycle::UploadOutcome::Sent)]
    );
    let seen = m.server.seen.lock().unwrap().clone();
    let stats: Vec<&Seen> = seen
        .iter()
        .filter(|(_, url, _)| url.contains("stats"))
        .collect();
    assert_eq!(stats.len(), 1, "{stats:?}");
    assert_eq!(stats[0].1, format!("{BASE}/api/v1/stats/reports"));
    assert_eq!(stats[0].2.as_deref(), Some(TOKEN));
    assert!(
        !seen
            .iter()
            .any(|(_, url, _)| url.starts_with("https://stats.figurehead.ai")),
        "never both: {seen:?}"
    );
    let overview = usage_stats::usage_stats_overview_impl(&m.state).unwrap();
    assert!(overview.reports.iter().any(|r| r.week == last && r.synced));
}

#[test]
fn a_401_unlinks_keeps_the_report_and_sends_nothing_anonymously() {
    let m = sharing_mac_with_sync(401);
    let last = seed_last_week(&m);
    let out = run_week(&m, &last);
    assert_eq!(
        out.uploads[0].1,
        juggler_stats::cycle::UploadOutcome::Skipped("unlinked".into())
    );
    let seen = m.server.seen.lock().unwrap().clone();
    assert!(
        !seen
            .iter()
            .any(|(_, url, _)| url.starts_with("https://stats.figurehead.ai")),
        "{seen:?}"
    );
    let status = figurehead::status_impl(&m.state).unwrap();
    assert_eq!(status.state, "signed_out", "the Mac is unlinked locally");
    let overview = usage_stats::usage_stats_overview_impl(&m.state).unwrap();
    assert!(!overview.sync.on);
    assert_eq!(
        overview.sync.notice.as_deref(),
        Some("This Mac is no longer linked, so stats sync is off.")
    );
    let report = overview.reports.iter().find(|r| r.week == last).unwrap();
    assert_eq!(report.status, "skipped");
    // Usage stats themselves stay as the user chose.
    assert_eq!(overview.info.chosen, "share");
    usage_stats::delete_local(&m.state).unwrap();
}
