//! QA on a real setup, read only: what the backend would show the user.
//!
//! Ignored, and a no-op unless `AIJ_QA_REAL_DATA` and `AIJ_QA_REAL_OUT` are
//! set, so it never runs in CI or on a plain `cargo test`. Run it with a
//! Finder-like environment (no `CLAUDE_*` variables) so discovery sees what
//! the app sees:
//!
//! ```sh
//! cargo test -p juggler-app --test qa_real_readonly --no-run
//! env -i HOME="$HOME" USER="$USER" TMPDIR="$TMPDIR" PATH=/usr/bin:/bin:/usr/sbin:/sbin \
//!   AIJ_QA_REAL_DATA=<fresh empty folder> AIJ_QA_REAL_OUT=<file.json> \
//!   <test binary> --ignored --nocapture
//! scripts/qa/real_truth.py --compare <file.json>
//! ```
//!
//! It does what first run does with "Use them where they are": scans, and
//! brings every setup found in where it is, into the FRESH data folder
//! given. Then it calls the commands the UI calls: projects (after the
//! history backfill), the session list, quota displays, cost rollups after
//! usage ingest passes, and This Mac over about a minute of the app's
//! process ticks (with `ps` pid/ppid/rss/cpu at each read). It never
//! launches, signs in or writes into a setup. The output holds real paths
//! and labels: keep it out of the repository; the compare step redacts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::PathBuf;
use std::sync::Arc;

use juggler_app::commands;
use juggler_app::state::AppState;
use tauri::Manager;

#[test]
#[ignore = "QA on a real setup; needs AIJ_QA_REAL_DATA and AIJ_QA_REAL_OUT"]
fn real_setup_read_models() {
    let (Some(data), Some(out)) = (
        std::env::var_os("AIJ_QA_REAL_DATA").map(PathBuf::from),
        std::env::var_os("AIJ_QA_REAL_OUT").map(PathBuf::from),
    ) else {
        eprintln!("AIJ_QA_REAL_DATA / AIJ_QA_REAL_OUT not set: nothing to do");
        return;
    };
    assert!(
        !data.exists() || std::fs::read_dir(&data).unwrap().next().is_none(),
        "AIJ_QA_REAL_DATA must be a fresh, empty folder"
    );
    let state = Arc::new(AppState::init(data).unwrap());
    let mut report = serde_json::Map::new();

    // First run: the scan, then "Bring in N accounts" where they are.
    let scan = commands::environment_scan_full_impl(&state, None).unwrap();
    // `AIJ_QA_REAL_LABELS=redacted` names the accounts "Account A"… in
    // the oracle's order (the default ~/.claude, then the profiles by
    // name, then Codex), so screenshots of the UI carry no real labels.
    let redacted = std::env::var("AIJ_QA_REAL_LABELS").as_deref() == Ok("redacted");
    let home = dirs::home_dir().unwrap_or_default();
    let mut order: Vec<&commands::DiscoveredInfo> = scan.installs.iter().collect();
    order.sort_by_key(|found| {
        let path = std::path::Path::new(found.path.trim_end_matches('/'));
        (
            found.harness != "claude-code",
            path != home.join(".claude"),
            found.path.clone(),
        )
    });
    let mut imports = Vec::new();
    for (n, found) in order.into_iter().enumerate() {
        let label = if redacted {
            format!("Account {}", char::from(b'A' + u8::try_from(n).unwrap()))
        } else {
            std::path::Path::new(&found.path)
                .file_name()
                .map(|name| name.to_string_lossy().trim_start_matches('.').to_string())
                .unwrap_or_else(|| found.harness.clone())
        };
        let result = commands::account_import_mode_impl(
            &state,
            &found.harness,
            &found.path,
            &label,
            commands::ImportRequest {
                origin: Some(found.origin.clone()),
                in_place: true,
                ..commands::ImportRequest::default()
            },
        );
        imports.push(match result {
            Ok(done) => serde_json::json!({"path": found.path, "ok": true, "login": done.login,
                                           "account": done.account.id.as_str(), "in_place": done.account.in_place}),
            Err(error) => serde_json::json!({"path": found.path, "ok": false, "error": format!("{error:?}")}),
        });
    }
    report.insert("scan".into(), serde_json::to_value(&scan).unwrap());
    report.insert("imports".into(), imports.into());
    let accounts = state.storage.lock().unwrap().accounts().list().unwrap();
    report.insert(
        "accounts".into(),
        accounts
            .iter()
            .map(|a| {
                serde_json::json!({"id": a.id.as_str(), "harness": a.harness.as_str(),
                                         "label": a.label, "state_dir": a.state_dir})
            })
            .collect::<Vec<_>>()
            .into(),
    );

    // The rail and History: what the UI reads once the backfill is done.
    let projects_before = commands::projects_list_impl(&state).unwrap();
    let backfilled = commands::projects_backfill_impl(&state).unwrap();
    let projects = commands::projects_list_impl(&state).unwrap();
    report.insert(
        "projects_before_backfill".into(),
        serde_json::to_value(&projects_before).unwrap(),
    );
    report.insert("projects_backfilled".into(), backfilled.into());
    // AJ-E1529: every account's conversations, and which are live.
    juggler_app::features::conversations::tick(&state).unwrap();
    report.insert(
        "conversations".into(),
        serde_json::to_value(
            juggler_app::features::conversations::commands::list_impl(&state).unwrap(),
        )
        .unwrap(),
    );
    report.insert("projects".into(), serde_json::to_value(&projects).unwrap());

    let app = tauri::test::mock_builder()
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(Arc::clone(&state));
    let managed = app.state::<Arc<AppState>>();
    report.insert(
        "sessions".into(),
        serde_json::to_value(commands::sessions_list(managed.clone()).unwrap()).unwrap(),
    );

    // Usage: the pipeline's passes (spool, transcripts, rollouts), then
    // the read models the Home runway, Accounts and Usage screens use.
    report.insert(
        "ingested_ms".into(),
        juggler_core::model::Timestamp::now().millis().into(),
    );
    for tick in 0..4 {
        juggler_app::usage_pipeline::ingest_pass(&state, tick);
    }
    report.insert(
        "usage_sources".into(),
        serde_json::to_value(juggler_app::usage_pipeline::source_health(&state)).unwrap(),
    );
    report.insert(
        "quota_display_all".into(),
        serde_json::to_value(commands::quota_display_all(managed.clone())).unwrap(),
    );
    report.insert(
        "cost_rollups".into(),
        serde_json::to_value(commands::cost_rollups(managed.clone(), None)).unwrap(),
    );

    // This Mac: the app's 2 s process tick (collector table, then the
    // machine tick) for about a minute, read as the UI polls it, with
    // every alert the notifications would have shown. A fresh store has
    // no ledger rows, so the tick's cleanup sweep has nothing to act on.
    // `ps` (pid, ppid, rss, cpu; no arguments) is taken right after each
    // read, for the compare step.
    let mut alerts = Vec::new();
    let mut panels = Vec::new();
    for n in 0..36 {
        commands::processes_tick(&state);
        if n % 5 == 4 {
            let now = juggler_core::model::Timestamp::now().millis();
            let panel = juggler_app::machine_health::this_mac_impl(&state, now).unwrap();
            let ps = std::process::Command::new("ps")
                .args(["-axo", "pid=,ppid=,rss=,pcpu="])
                .output()
                .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
                .unwrap_or_default();
            let mut panel = serde_json::to_value(panel).unwrap();
            panel["qa_ps"] = ps
                .lines()
                .filter_map(|line| {
                    let f: Vec<&str> = line.split_whitespace().collect();
                    (f.len() == 4).then(|| {
                        serde_json::json!([
                            f[0].parse::<u32>().ok(),
                            f[1].parse::<u32>().ok(),
                            f[2].parse::<u64>().ok(),
                            f[3].parse::<f64>().ok()
                        ])
                    })
                })
                .collect::<Vec<_>>()
                .into();
            panels.push(panel);
        }
        alerts.extend(
            juggler_app::machine_health::machine_alerts_take_impl(&state)
                .unwrap()
                .into_iter()
                .map(|alert| serde_json::to_value(alert).unwrap()),
        );
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    report.insert("this_mac".into(), panels.into());
    report.insert("machine_alerts".into(), alerts.into());
    report.insert(
        "generated_ms".into(),
        juggler_core::model::Timestamp::now().millis().into(),
    );

    std::fs::write(
        &out,
        serde_json::to_vec_pretty(&serde_json::Value::Object(report)).unwrap(),
    )
    .unwrap();
    eprintln!("wrote {}", out.display());
}
