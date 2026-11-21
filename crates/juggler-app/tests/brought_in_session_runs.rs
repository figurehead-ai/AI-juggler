//! AJ-E2312 item 4 (SK-21, the PF1 walk): after "Find my apps" and "Bring
//! in", the second setup's running session read as ended for about 30 s,
//! then as running. The live view reads the per-session pid files of a
//! list of folders it kept for 15 s, and bringing a setup in didn't
//! renew it: the conversation pass indexed the new folder's session while
//! the live view still didn't read that folder's pid file, so the row
//! said it had ended.
//!
//! End to end over the real code (the conversation index, the live view,
//! the folders' stamp in the database); only the process table is
//! scripted. Runs as a child on `HOME=<lab>` so nothing of this Mac's own
//! is read.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test"
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::features::conversations::procs::{Proc, Table};
use juggler_app::features::conversations::scan::Os;
use juggler_app::features::conversations::{self, commands};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, LaunchRecipe, ProfileId, SettingsOverlay, Timestamp,
};

const CHILD: &str = "AIJ_TEST_BROUGHT_IN_RUNS_CHILD";
const SESSION: &str = "0a1b2c3d-0000-4000-8000-000000000021";
/// `Sat Oct 24 08:12:34 2026` UTC, the `claude` process's start (its pid
/// file's `procStart`).
const STARTED: i64 = 1_792_829_554;
/// High pids: never one a real process holds.
const TERMINAL_PID: u32 = 990_310;
const SHELL_PID: u32 = 990_311;
const CLAUDE_PID: u32 = 990_312;

struct Scripted(Table);

impl Os for Scripted {
    fn table(&self) -> Table {
        self.0.clone()
    }
    fn some(&self, pids: &[u32]) -> Table {
        pids.iter()
            .filter_map(|pid| self.0.get(pid).map(|p| (*pid, p.clone())))
            .collect()
    }
    fn open_files(&self, _pid: u32) -> Option<Vec<PathBuf>> {
        None
    }
}

fn put(table: &mut Table, pid: u32, ppid: u32, exe: &str, args: &[&str], start: i64) {
    table.insert(
        pid,
        Proc {
            pid,
            ppid: Some(ppid),
            exe: Some(PathBuf::from(exe)),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            start: Some(start),
        },
    );
}

fn write(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

/// Runs this test again as a child on `HOME=<a new lab>` (the home is
/// read once per process); in the child, the lab.
fn in_a_lab(test: &str) -> Option<PathBuf> {
    if std::env::var_os(CHILD).is_some() {
        return Some(PathBuf::from(std::env::var_os("HOME").unwrap()));
    }
    let lab = tempfile::Builder::new()
        .prefix("aij-sk21-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--nocapture"])
        .env(CHILD, "1")
        .env("HOME", lab.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    None
}

#[test]
fn a_running_session_in_a_setup_just_brought_in_reads_running_at_once() {
    let Some(home) = in_a_lab("a_running_session_in_a_setup_just_brought_in_reads_running_at_once")
    else {
        return;
    };
    let home = home.canonicalize().unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(
        home.join("no-claude"),
    )));
    let state = Arc::new(AppState::init_with_registry(home.join("app-data"), registry).unwrap());

    // The second setup found on the Mac (PF1's Work): Claude Code runs in
    // it, in a terminal, outside AI Juggler, with its pid file and its
    // conversation where Claude Code keeps them.
    let folder = home.join(".claude-work");
    let cwd = home.join("repos/billing");
    std::fs::create_dir_all(&cwd).unwrap();
    let cwd_s = cwd.display().to_string();
    let at = chrono_like(Timestamp::now().millis() - 60_000);
    write(
        &folder.join(format!(
            "projects/{}/{SESSION}.jsonl",
            cwd_s.replace('/', "-")
        )),
        &format!(
            "{}\n",
            format_args!(
                r#"{{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"{cwd_s}","sessionId":"{SESSION}","version":"2.1.296","type":"user","message":{{"role":"user","content":"Fix the invoice totals"}},"uuid":"m-1","timestamp":"{at}"}}"#
            ),
        ),
    );
    write(
        &folder.join(format!("sessions/{CLAUDE_PID}.json")),
        &format!(
            r#"{{"pid":{CLAUDE_PID},"sessionId":"{SESSION}","cwd":"{cwd_s}","status":"busy","procStart":"Sat Oct 24 08:12:34 2026","entrypoint":"cli","kind":"interactive"}}"#
        ),
    );
    let terminal = "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal";
    let mut table = Table::new();
    put(&mut table, 1, 0, "/sbin/launchd", &[], 0);
    put(
        &mut table,
        TERMINAL_PID,
        1,
        terminal,
        &[terminal],
        STARTED - 100,
    );
    put(
        &mut table,
        SHELL_PID,
        TERMINAL_PID,
        "/bin/zsh",
        &["-zsh"],
        STARTED - 50,
    );
    let claude = home.join(".local/bin/claude");
    put(
        &mut table,
        CLAUDE_PID,
        SHELL_PID,
        claude.to_str().unwrap(),
        &["claude"],
        STARTED,
    );
    let os = Scripted(table);

    // AI Juggler is up and has looked at what runs: no setup of its own
    // yet, only each tool's default folder.
    conversations::live_tick_on(&state, &os, &home).unwrap();

    // Bring in: the setup is AI Juggler's now, where it is (the write
    // Bring in makes: its folder and a launch setting for it).
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("claude-code-work"),
                harness: HarnessId::new(CLAUDE_CODE_ID),
                label: "Work".into(),
                plan: None,
                state_dir: folder.clone(),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: Some(folder.display().to_string()),
                import_origin: Some("found".into()),
                imported_at: Some(Timestamp::now().millis()),
                in_place: true,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("prof-work"),
                name: "Main".into(),
                account: AccountId::new("claude-code-work"),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            })
            .unwrap();
    }

    // The next pass lists its conversation, and the next look (2 s on) says
    // what runs: within seconds of the bring-in, never a turn of the 15 s
    // folder list later.
    conversations::tick(&state).unwrap();
    conversations::live_tick_on(&state, &os, &home).unwrap();
    let row = commands::list_impl(&state)
        .unwrap()
        .into_iter()
        .find(|c| c.id == SESSION)
        .expect("its conversation is listed");
    assert!(row.live, "it runs: never drawn as ended ({row:?})");
}

/// `ms` as a transcript writes its time (`2026-11-21T09:00:00.000Z`).
fn chrono_like(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.000Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}
