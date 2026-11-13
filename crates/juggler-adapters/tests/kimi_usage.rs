//! AJ-E1603 R3 (D-106): a Kimi Code account's plan usage from a `kimi
//! web` started for the request, end to end against `fake-kimi` (a
//! stand-in that registers its pid, listens on a free port, prints a token
//! banner and answers healthz and usage). Nothing real is run or read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "test code: a failed step should stop the test where it failed"
)]

mod kimi_fixture;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use juggler_adapter::prelude::*;
use juggler_adapters::kimi_usage::{self, RunLimits};
use juggler_adapters::{KIMI_CODE_ID, KimiCodeAdapter};
use juggler_core::model::{Account, AccountId, HarnessId};

/// One self-started server at a time is the rule under test: the tests
/// here take turns so they never see each other's.
static TURN: Mutex<()> = Mutex::new(());

fn turn() -> std::sync::MutexGuard<'static, ()> {
    TURN.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn fake_kimi() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-kimi"))
}

fn limits(run_ms: u64, grace_ms: u64) -> RunLimits {
    RunLimits {
        run: Duration::from_millis(run_ms),
        stop_grace: Duration::from_millis(grace_ms),
    }
}

fn mode(home: &Path, mode: &str) {
    kimi_fixture::write(
        &home.join("fake-kimi.json"),
        &serde_json::json!({ "mode": mode }).to_string(),
    );
}

fn seen(home: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(home.join("fake-kimi-seen.json")).unwrap())
        .unwrap()
}

fn launches(home: &Path) -> usize {
    std::fs::read_to_string(home.join("fake-kimi-launches.log"))
        .unwrap_or_default()
        .lines()
        .count()
}

/// The stand-in's process, once it has been reaped: gone.
fn gone(home: &Path) -> bool {
    let pid = u32::try_from(seen(home)["pid"].as_u64().unwrap()).unwrap();
    !juggler_platform::process::alive(pid)
}

fn account(state_dir: &Path) -> Account {
    Account {
        id: AccountId::new("kimi-acct"),
        harness: HarnessId::new(KIMI_CODE_ID),
        label: "Kimi".into(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

#[test]
fn a_started_server_answers_with_the_per_run_secret_and_is_stopped_at_once() {
    let _turn = turn();
    let home = tempfile::tempdir().unwrap();
    assert_eq!(kimi_usage::self_started_live(), 0);
    let reading =
        kimi_usage::usage(Some(&fake_kimi()), home.path(), limits(20_000, 3_000)).unwrap();
    let windows: Vec<(&str, f64)> = reading
        .windows
        .iter()
        .map(|(kind, sample)| (kind.as_str(), sample.used_pct))
        .collect();
    assert_eq!(
        windows,
        [("five_hour", 42.0), ("weekly", 10.0), ("monthly", 20.0)]
    );
    assert_eq!(reading.parts.len(), 1);
    // Stopped and reaped before the answer came back: the ledger is 0.
    assert_eq!(kimi_usage::self_started_live(), 0);
    assert!(gone(home.path()), "the server is still running");
    assert_eq!(launches(home.path()), 1);

    let seen = seen(home.path());
    assert_eq!(seen["password_set"], true);
    // Only the helper seam's variables, the home and the secret.
    let env: Vec<&str> = seen["env"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    let allowed = |name: &str| {
        name == "PATH"
            || name == "KIMI_CODE_HOME"
            || name == "KIMI_CODE_PASSWORD"
            || juggler_platform::process::ENV_ALLOWLIST.contains(&name)
    };
    assert!(env.iter().all(|name| allowed(name)), "{env:?}");
    // The secret was set on that child only, never on this process.
    assert!(std::env::var_os("KIMI_CODE_PASSWORD").is_none());
    let requests = seen["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["path"], "/api/v1/healthz");
    assert!(
        requests[0]["authorized"].is_null(),
        "healthz goes without it"
    );
    assert_eq!(requests[1]["path"], "/api/v1/oauth/usage");
    assert_eq!(
        requests[1]["authorized"], true,
        "the per-run secret, not the banner's token"
    );
    for request in requests {
        assert_eq!(request["origin"], false);
        assert!(request["host"].as_str().unwrap().starts_with("127.0.0.1:"));
    }
}

#[test]
fn a_server_that_never_registers_is_reaped_at_the_deadline() {
    let _turn = turn();
    let home = tempfile::tempdir().unwrap();
    mode(home.path(), "never-register");
    let started = Instant::now();
    let error =
        kimi_usage::usage(Some(&fake_kimi()), home.path(), limits(1_500, 3_000)).unwrap_err();
    assert_eq!(error, "Kimi Code's server didn't start in time");
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(kimi_usage::self_started_live(), 0);
    assert!(gone(home.path()));
    assert!(!error.contains("fake-banner-token"));
}

#[test]
fn one_that_ignores_sigterm_is_killed_and_one_that_crashes_is_never_restarted() {
    let _turn = turn();
    let home = tempfile::tempdir().unwrap();
    mode(home.path(), "ignore-term");
    assert!(kimi_usage::usage(Some(&fake_kimi()), home.path(), limits(800, 400)).is_err());
    assert_eq!(kimi_usage::self_started_live(), 0);
    assert!(gone(home.path()));

    let crashed = tempfile::tempdir().unwrap();
    mode(crashed.path(), "exit");
    let error =
        kimi_usage::usage(Some(&fake_kimi()), crashed.path(), limits(5_000, 1_000)).unwrap_err();
    assert_eq!(error, "Kimi Code's server stopped before it was ready");
    assert_eq!(launches(crashed.path()), 1, "started once, never again");
    assert_eq!(kimi_usage::self_started_live(), 0);
}

#[test]
fn at_most_one_runs_across_every_account() {
    let _turn = turn();
    let first = tempfile::tempdir().unwrap();
    mode(first.path(), "never-register");
    let path = first.path().to_path_buf();
    let running = std::thread::spawn(move || {
        kimi_usage::usage(Some(&fake_kimi()), &path, limits(2_500, 1_000))
    });
    // Until the first has started.
    let until = Instant::now() + Duration::from_secs(5);
    while kimi_usage::self_started_live() == 0 && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(kimi_usage::self_started_live(), 1);
    let second = tempfile::tempdir().unwrap();
    assert_eq!(
        kimi_usage::usage(Some(&fake_kimi()), second.path(), limits(5_000, 1_000)).unwrap_err(),
        "another Kimi Code usage check is running"
    );
    assert_eq!(launches(second.path()), 0);
    assert!(running.join().unwrap().is_err());
    assert_eq!(kimi_usage::self_started_live(), 0);
}

#[test]
fn an_answer_that_fails_the_format_check_is_unknown_and_the_server_still_stops() {
    let _turn = turn();
    let home = tempfile::tempdir().unwrap();
    kimi_fixture::write(
        &home.path().join("fake-kimi.json"),
        &serde_json::json!({
            "mode": "ok",
            "usage": r#"{"code":0,"data":{"kind":"ok","quota":{"usages":{"limit_5h":{"usedRatio":0.5}},"extraUsage":null}}}"#,
        })
        .to_string(),
    );
    let error =
        kimi_usage::usage(Some(&fake_kimi()), home.path(), limits(20_000, 3_000)).unwrap_err();
    assert!(error.contains("doesn't read yet"), "{error}");
    assert_eq!(kimi_usage::self_started_live(), 0);
    assert!(gone(home.path()));
}

#[test]
fn only_a_membership_account_is_asked_through_the_adapter() {
    let _turn = turn();
    let adapter = KimiCodeAdapter::with_bin(fake_kimi());
    let member = tempfile::tempdir().unwrap();
    kimi_fixture::sign_in_membership(member.path(), "made-up-sign-in");
    assert!(adapter.plan_usage_offered(member.path()));
    let reading = adapter
        .plan_usage(&account(member.path()))
        .unwrap()
        .unwrap();
    assert_eq!(reading.windows.len(), 3);
    assert_eq!(kimi_usage::self_started_live(), 0);

    let key = tempfile::tempdir().unwrap();
    kimi_fixture::sign_in_api_key(key.path());
    assert!(!adapter.plan_usage_offered(key.path()));
    assert!(adapter.plan_usage(&account(key.path())).is_none());
    assert_eq!(launches(key.path()), 0, "an API key's account gets no call");
    let none = tempfile::tempdir().unwrap();
    assert!(adapter.plan_usage(&account(none.path())).is_none());
}
