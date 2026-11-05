//! AJ-E1202 end-to-end against a running Figurehead server (a local
//! development server for evidence; never production from a test).
//! Opt-in: `AIJ_E2E_FIGUREHEAD_URL=http://localhost:4321 cargo test -p
//! juggler-account --test live_server -- --ignored --nocapture`, then
//! approve the printed link in a browser within two minutes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_account::{AccountClient, ApiError, DeviceFields, PollOutcome, UreqTransport};

#[test]
#[ignore = "needs a running Figurehead server and a human approval"]
fn link_approve_account_rename_unlink() {
    let base = std::env::var("AIJ_E2E_FIGUREHEAD_URL").expect("set AIJ_E2E_FIGUREHEAD_URL");
    assert!(
        base.starts_with("http://localhost") || base.starts_with("http://127.0.0.1"),
        "end-to-end tests only ever talk to a local server"
    );
    let client = AccountClient::new(&base, Arc::new(UreqTransport::new("0.0.1")));
    let device = DeviceFields {
        name: "AIJ e2e Mac".into(),
        platform: "macOS 26 · arm64".into(),
        app_version: "0.0.1".into(),
        public_key: "ed25519:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=;x25519:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
    };
    let started = Instant::now();
    let link = client.start_link(&device).unwrap();
    eprintln!(
        "[e2e] approve {} (code {})",
        link.verify_url, link.user_code
    );
    let mut interval = link.interval_s;
    let (device_id, token) = loop {
        std::thread::sleep(Duration::from_secs(interval));
        match client.poll_link(&link).unwrap() {
            PollOutcome::Pending => {}
            PollOutcome::SlowDown { interval_s } => interval = interval_s,
            PollOutcome::Approved { device_id, token } => break (device_id, token),
            other => panic!("link ended: {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(120),
            "not approved in time"
        );
    };
    eprintln!(
        "[e2e] approved as {device_id} after {:?}",
        started.elapsed()
    );
    let account = client.account(&token).unwrap();
    // Checked, never logged: nothing read from an Account goes to the log
    // (CodeQL alerts 192, 193), not even whether a plan was read.
    assert!(
        account.plan_name().is_some() || account.fallback,
        "the account read names a plan or says it fell back"
    );
    eprintln!("[e2e] account read");
    client
        .rename(&token, "AIJ e2e Mac (renamed)", "0.0.1")
        .unwrap();
    let renamed = client.account(&token).unwrap();
    assert_eq!(renamed.device_name(), Some("AIJ e2e Mac (renamed)"));
    eprintln!("[e2e] device renamed");
    client.unlink(&token).unwrap();
    assert_eq!(
        client.account(&token),
        Err(ApiError::Unlinked),
        "the token stops working at once"
    );
    eprintln!("[e2e] unlinked; the old token now answers 401");
}
