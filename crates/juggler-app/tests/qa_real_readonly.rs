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
//! process ticks (with `ps` pid/ppid/rss/cpu/elapsed/name, and `vm_stat` with
//! the pressure level, swap and physical memory, at each read). It never
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

use juggler_app::state::AppState;
use tauri::Manager;

mod common;

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
    // This test's own pid: the app leaves its own process tree out of This
    // Mac, so the compare leaves it out of the ps side too.
    report.insert("qa_pid".into(), std::process::id().into());

    // First run: the scan, then "Bring in N accounts" where they are.
    // `AIJ_QA_REAL_LABELS=redacted` names the accounts "Account A"… in
    // the oracle's order (the default ~/.claude, then the profiles by
    // name, then Codex), so screenshots of the UI carry no real labels.
    let redacted = std::env::var("AIJ_QA_REAL_LABELS").as_deref() == Ok("redacted");
    let home = dirs::home_dir().unwrap_or_default();
    let (scan, imports) = common::first_run::bring_in_all(&state, &home, redacted);
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
    let projects_before =
        juggler_app::features::projects::list::projects_list_impl(&state).unwrap();
    let backfilled =
        juggler_app::features::projects::backfill::projects_backfill_impl(&state).unwrap();
    let projects = juggler_app::features::projects::list::projects_list_impl(&state).unwrap();
    report.insert(
        "projects_before_backfill".into(),
        serde_json::to_value(&projects_before).unwrap(),
    );
    report.insert("projects_backfilled".into(), backfilled.into());
    // AJ-E1529: the accounts as the UI reads them (the replica fixture).
    report.insert(
        "accounts_info".into(),
        serde_json::to_value(juggler_app::features::account::accounts_info(&state).unwrap())
            .unwrap(),
    );
    // AJ-E1529: every account's conversations, and which are live.
    juggler_app::features::conversations::tick(&state).unwrap();
    // AJ-E2101: what is open now, from the apps' own files (the app's
    // live thread runs this every couple of seconds; twice here so the
    // first full look and its follow-up both happen).
    juggler_app::features::conversations::live_tick(&state).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));
    juggler_app::features::conversations::live_tick(&state).unwrap();
    // AJ-E3203, D-84: "Find them" on, as the user turns it on: the Claude
    // app profiles on disk, running or not, offered (never added).
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let now_ms = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    juggler_app::features::places::profiles::set(&state, true, &home, now_ms).unwrap();
    report.insert(
        "profile_offers".into(),
        serde_json::to_value(juggler_app::features::places::offers(&state).unwrap()).unwrap(),
    );
    // AJ-E2109: accounts as subscriptions, their profiles beneath.
    report.insert(
        "subscriptions".into(),
        serde_json::to_value(juggler_app::features::subscriptions::list_impl(&state).unwrap())
            .unwrap(),
    );
    report.insert(
        "conversations".into(),
        serde_json::to_value(
            juggler_app::features::conversations::commands::list_impl(&state).unwrap(),
        )
        .unwrap(),
    );
    report.insert("projects".into(), serde_json::to_value(&projects).unwrap());
    // AJ-E3303 (L2): what every page heads its tabs with, from the
    // relations cache: everything, each project, each folder (a CLI
    // App), each Account and each tool. The compare checks them against
    // the transcripts' own tallies.
    {
        use juggler_app::features::relations::{EntityKind, EntityRef, MAX_ASKED, counts_many};
        let subs = juggler_app::features::subscriptions::list_impl(&state).unwrap();
        let mut asks = vec![EntityRef {
            kind: EntityKind::All,
            id: String::new(),
        }];
        asks.extend(projects.iter().map(|p| EntityRef {
            kind: EntityKind::Project,
            id: p.id.clone(),
        }));
        for sub in &subs {
            asks.extend(sub.profiles.iter().map(|p| EntityRef {
                kind: EntityKind::Folder,
                id: p.id.clone(),
            }));
            if let Some(id) = &sub.account_id {
                asks.push(EntityRef {
                    kind: EntityKind::Subscription,
                    id: id.clone(),
                });
            }
        }
        let mut tools: Vec<String> = subs.iter().map(|s| s.tool.clone()).collect();
        tools.sort();
        tools.dedup();
        asks.extend(tools.into_iter().map(|id| EntityRef {
            kind: EntityKind::Tool,
            id,
        }));
        let mut counts = Vec::new();
        for chunk in asks.chunks(MAX_ASKED) {
            counts.extend(counts_many(&state, chunk).unwrap());
        }
        report.insert(
            "relation_counts".into(),
            serde_json::to_value(&counts).unwrap(),
        );
    }
    // AJ-E1532: each project's own icon, as the UI reads it — only when the
    // replica fixture asks (`just replica-fixture`); a pass over real data
    // records none.
    if std::env::var_os("AIJ_QA_ICONS").is_some() {
        let icons: serde_json::Map<String, serde_json::Value> = projects
            .iter()
            .filter_map(|p| {
                let url = juggler_app::features::projects::icon::icon_read_impl(&state, &p.id)
                    .unwrap()?;
                Some((p.id.clone(), url.into()))
            })
            .collect();
        report.insert("project_icons".into(), icons.into());
    }

    let app = tauri::test::mock_builder()
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(Arc::clone(&state));
    let managed = app.state::<Arc<AppState>>();
    report.insert(
        "sessions".into(),
        serde_json::to_value(
            tauri::async_runtime::block_on(
                juggler_app::features::sessions::commands::sessions_list(managed.clone()),
            )
            .unwrap(),
        )
        .unwrap(),
    );
    // The launch settings each import made ("Main"), as the UI lists them.
    report.insert(
        "launch_recipes".into(),
        serde_json::to_value(
            tauri::async_runtime::block_on(
                juggler_app::features::profile::commands::profiles_list(managed.clone()),
            )
            .unwrap(),
        )
        .unwrap(),
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
    // The passes read for a while: what live writers add meanwhile may or
    // may not be in. The compare accepts anything between the two moments.
    report.insert(
        "ingest_done_ms".into(),
        juggler_core::model::Timestamp::now().millis().into(),
    );
    report.insert(
        "usage_sources".into(),
        serde_json::to_value(juggler_app::usage_pipeline::source_health(&state)).unwrap(),
    );
    report.insert(
        "quota_display_all".into(),
        serde_json::to_value(
            tauri::async_runtime::block_on(
                juggler_app::features::quota::commands::quota_display_all(managed.clone()),
            )
            .unwrap(),
        )
        .unwrap(),
    );
    report.insert(
        "cost_rollups".into(),
        serde_json::to_value(
            tauri::async_runtime::block_on(juggler_app::features::cost::commands::cost_rollups(
                managed.clone(),
                None,
            ))
            .unwrap(),
        )
        .unwrap(),
    );

    // This Mac: the app's 2 s process tick (collector table, then the
    // machine tick) for about a minute, read as the UI polls it, with
    // every alert the notifications would have shown. A fresh store has
    // no ledger rows, so the tick's cleanup sweep has nothing to act on.
    // `ps` (pid, ppid, rss, cpu, elapsed, the command's file name; no
    // arguments) and `vm_stat` (with the pressure level, swap and physical
    // memory) are taken right after each read, for the compare step.
    let mut alerts = Vec::new();
    let mut panels = Vec::new();
    for n in 0..36 {
        juggler_app::features::processes::tick::processes_tick(&state);
        if n % 5 == 4 {
            let now = juggler_core::model::Timestamp::now().millis();
            let panel = juggler_app::machine_health::this_mac_impl(&state, now).unwrap();
            let ps = std::process::Command::new("ps")
                .args(["-axo", "pid=,ppid=,rss=,pcpu=,etime=,comm="])
                .output()
                .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
                .unwrap_or_default();
            let mut panel = serde_json::to_value(panel).unwrap();
            panel["qa_ps"] = ps
                .lines()
                .filter_map(|line| {
                    // pid, ppid, rss (KB), cpu, elapsed, and the command's
                    // file name only (never its folder or arguments).
                    let f: Vec<&str> = line.split_whitespace().collect();
                    (f.len() >= 6).then(|| {
                        let command = f[5..].join(" ");
                        let name = command.rsplit('/').next().unwrap_or("").to_string();
                        serde_json::json!([
                            f[0].parse::<u32>().ok(),
                            f[1].parse::<u32>().ok(),
                            f[2].parse::<u64>().ok(),
                            f[3].parse::<f64>().ok(),
                            f[4],
                            name
                        ])
                    })
                })
                .collect::<Vec<_>>()
                .into();
            // macOS's own memory figures, as numbers only.
            let run = |cmd: &str, args: &[&str]| {
                std::process::Command::new(cmd)
                    .args(args)
                    .output()
                    .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
                    .unwrap_or_default()
            };
            panel["qa_vm"] = serde_json::json!({
                "vm_stat": run("vm_stat", &[]),
                "pressure_level": run("sysctl", &["-n", "kern.memorystatus_vm_pressure_level"]),
                "swapusage": run("sysctl", &["-n", "vm.swapusage"]),
                "memsize": run("sysctl", &["-n", "hw.memsize"]),
            });
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

/// Before a release: the update's own backup step, run on a COPY of the
/// user's real AI Juggler data (`AIJ_QA_APP_DATA_COPY`, a folder holding a
/// consistent copy of `juggler.sqlite`, e.g. from `sqlite3 … ".backup …"`).
/// Two updates in a row failed on the real setup and nowhere else (an
/// unsigned version, then a project name read as a key); this runs what
/// the update runs, on what the user really has. Ignored and a no-op
/// unless the variable is set; it never opens the real folder.
#[test]
#[ignore = "QA before a release; needs AIJ_QA_APP_DATA_COPY"]
fn update_preflight_on_a_copy_of_the_real_data() {
    let Some(copy) = std::env::var_os("AIJ_QA_APP_DATA_COPY").map(PathBuf::from) else {
        eprintln!("AIJ_QA_APP_DATA_COPY not set: nothing to do");
        return;
    };
    let real = dirs::data_dir().unwrap_or_default().join("AI Juggler");
    assert!(
        !copy.starts_with(&real),
        "AIJ_QA_APP_DATA_COPY must be a copy, never the real folder"
    );
    assert!(
        copy.join("juggler.sqlite").exists(),
        "the copy holds juggler.sqlite"
    );
    let state = Arc::new(AppState::init(copy.clone()).unwrap());
    let dest = copy.join("preflight-before-update.ajbackup");
    let _ = std::fs::remove_file(&dest);
    let written = juggler_app::backup::backup_before_update(&state, &dest, &mut |_, _| {});
    assert!(
        written.is_ok(),
        "the update's backup would fail: {}",
        written.err().map(|e| e.to_string()).unwrap_or_default()
    );
    assert!(dest.exists());
    eprintln!(
        "update preflight: the backup before an update is written ({} bytes)",
        std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0)
    );
}
