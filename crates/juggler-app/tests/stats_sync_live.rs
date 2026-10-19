//! AJ-E1109 evidence against the website's local development server —
//! never production from a test. Opt-in:
//! `AIJ_E2E_FIGUREHEAD_URL=http://localhost:4321 cargo test -p juggler-app
//! --test stats_sync_live -- --ignored --nocapture`, then approve the
//! printed link in a browser within two minutes.

#![cfg(feature = "stats")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::io::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_account::{AccountClient, DeviceFields, PollOutcome, UreqTransport};
use juggler_app::usage_stats::enabled::{StatsTransport, envelope};
use juggler_stats::cycle::Transport as _;

#[test]
#[ignore = "needs the website's local dev server and a browser approval"]
fn a_synced_report_is_accepted_once_and_refused_after_unlink() {
    let base = std::env::var("AIJ_E2E_FIGUREHEAD_URL").expect("set AIJ_E2E_FIGUREHEAD_URL");
    assert!(
        base.starts_with("http://localhost") || base.starts_with("http://127.0.0.1"),
        "local servers only"
    );
    let client = AccountClient::new(&base, Arc::new(UreqTransport::new("0.1.0")));
    let link = client
        .start_link(&DeviceFields {
            name: "AIJ stats-sync e2e".into(),
            platform: "macOS · arm64".into(),
            app_version: "0.1.0".into(),
            public_key: String::new(),
        })
        .unwrap();
    eprintln!(
        "[e2e] approve {} (code {})",
        link.verify_url, link.user_code
    );
    let started = Instant::now();
    let mut interval = link.interval_s;
    let token = loop {
        std::thread::sleep(Duration::from_secs(interval));
        match client.poll_link(&link).unwrap() {
            PollOutcome::Pending => {}
            PollOutcome::SlowDown { interval_s } => interval = interval_s,
            PollOutcome::Approved { token, .. } => break token,
            other => panic!("link ended: {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(120),
            "not approved in time"
        );
    };
    eprintln!("[e2e] linked");

    // A real assembled report for last week (sample_rate 1, no pow).
    let dir = tempfile::tempdir().unwrap();
    let store = juggler_stats::Store::open(&dir.path().join("stats")).unwrap();
    let now = juggler_core::model::Timestamp::now().millis();
    let last = juggler_stats::week::label(now - 7 * 86_400_000);
    let start = juggler_stats::week::start_ms(&last).unwrap();
    let (y, m, d) = juggler_stats::week::civil(start.div_euclid(86_400_000));
    store.mark_active(&format!("{y:04}-{m:02}-{d:02}")).unwrap();
    store.add(&last, "app.launch", "{}", 4).unwrap();
    let envelope = juggler_stats::report::Envelope {
        app_version: "0.1.0".to_owned(),
        ..envelope()
    };
    let (report, _) = juggler_stats::report::assemble(&store, &last, &envelope, 1).unwrap();
    juggler_stats::report::validate(&report).unwrap();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&report.to_bytes()).unwrap();
    let body = gzip.finish().unwrap();
    eprintln!(
        "[e2e] report: {}",
        String::from_utf8_lossy(&report.to_bytes())
    );

    let transport = StatsTransport(Arc::new(UreqTransport::with_timeout(
        "0.1.0",
        Duration::from_secs(10),
    )));
    let url = format!("{base}/api/v1/stats/reports");
    let first = transport
        .post_gzip_json(&url, &body, Some(token.expose()))
        .unwrap();
    eprintln!(
        "[e2e] sync: {} {}",
        first.status,
        String::from_utf8_lossy(&first.body)
    );
    assert_eq!(first.status, 202);
    let again = transport
        .post_gzip_json(&url, &body, Some(token.expose()))
        .unwrap();
    eprintln!(
        "[e2e] again: {} {}",
        again.status,
        String::from_utf8_lossy(&again.body)
    );
    assert_eq!(again.status, 200);
    let anonymous = transport.post_gzip_json(&url, &body, None).unwrap();
    eprintln!("[e2e] without a token: {}", anonymous.status);
    assert_eq!(anonymous.status, 401);

    client.unlink(&token).unwrap();
    let after = transport
        .post_gzip_json(&url, &body, Some(token.expose()))
        .unwrap();
    eprintln!(
        "[e2e] after unlink: {} {}",
        after.status,
        String::from_utf8_lossy(&after.body)
    );
    assert_eq!(after.status, 401);
}
