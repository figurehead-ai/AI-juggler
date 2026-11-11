//! NY1 step 3, walked by test: only a real conversation reaches "Done,
//! review it" (the Claude app sets `review_ready` after a turn), so no
//! lab can seed it. End to end over the real code: the Claude app's own
//! record and a `claude` in its instance are read by the adapter through
//! the live scanner (only the process table is scripted), the index
//! joins them to the transcript, Needs you and the bell count it; Archive
//! is refused while it's open (only ended sessions are archived); opening
//! it marks it seen (the window's `conversation_seen` setting), and its
//! row leaves Needs you and the bell drops by one at once, and stays gone
//! over the next tick.
//!
//! Runs as a child on `HOME=<lab>` so nothing of this Mac's own (its
//! `~/.claude`, its Claude app profiles) is listed or read.

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
use juggler_app::error::AppError;
use juggler_app::features::conversations::procs::{Proc, Table};
use juggler_app::features::conversations::scan::Os;
use juggler_app::features::conversations::{self, archive, commands};
use juggler_app::features::notifications::{centre, needs, tray, views};
use juggler_app::state::AppState;
use juggler_core::model::Timestamp;

const CHILD: &str = "AIJ_TEST_NY1_REVIEW_CHILD";
const CLAUDE_APP: &str = "/Applications/Claude.app/Contents/MacOS/Claude";
const SESSION: &str = "0a1b2c3d-0000-4000-8000-00000000ny13";
/// `Sat Oct 24 08:12:34 2026` UTC, the `claude` process's start.
const STARTED: i64 = 1_792_829_554;
/// High pids: never one a real `claude` holds.
const APP_PID: u32 = 990_010;
const HELPER_PID: u32 = 990_011;
const CLAUDE_PID: u32 = 990_012;

/// The process table: the Claude app's instance, its helper, and the
/// `claude` it runs for the conversation.
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
        .prefix("aij-ny1-")
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
fn a_review_ready_conversation_is_not_archived_while_open_and_seeing_it_clears_needs_you() {
    let Some(home) = in_a_lab(
        "a_review_ready_conversation_is_not_archived_while_open_and_seeing_it_clears_needs_you",
    ) else {
        return;
    };
    let home = home.canonicalize().unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(
        home.join("no-claude"),
    )));
    let state = Arc::new(AppState::init_with_registry(home.join("app-data"), registry).unwrap());
    let account =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Work", None)
            .unwrap();
    juggler_app::features::profile::create_profile(&state, &account.id, "Main", None, None)
        .unwrap();
    let folder = state.accounts_dir().join(CLAUDE_CODE_ID).join(&account.id);
    let instance = home.join("Library/Application Support/ClaudeProfiles/b-work");
    let cwd = home.join("Dev/shop");
    std::fs::create_dir_all(&cwd).unwrap();
    let cwd_s = cwd.display().to_string();

    // The conversation's transcript, its last turn on 2026-11-01.
    write(
        &folder.join(format!(
            "projects/{}/{SESSION}.jsonl",
            cwd_s.replace('/', "-")
        )),
        &format!(
            "{}\n{}\n",
            format_args!(
                r#"{{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"{cwd_s}","sessionId":"{SESSION}","version":"2.1.296","type":"user","message":{{"role":"user","content":"Tidy the README"}},"uuid":"m-1","timestamp":"2026-11-01T09:00:00.000Z"}}"#
            ),
            format_args!(
                r#"{{"parentUuid":"m-1","isSidechain":false,"userType":"external","cwd":"{cwd_s}","sessionId":"{SESSION}","version":"2.1.296","type":"assistant","message":{{"id":"msg_1","type":"message","role":"assistant","model":"claude-opus-4-6","content":[{{"type":"text","text":"Done."}}],"stop_reason":"end_turn","usage":{{"input_tokens":10,"output_tokens":2}}}},"uuid":"m-2","timestamp":"2026-11-01T09:00:05.000Z"}}"#
            ),
        ),
    );
    // Claude Code's pid file for it: idle, held by the app's session
    // `local_ny13`.
    write(
        &folder.join(format!("sessions/{CLAUDE_PID}.json")),
        &format!(
            r#"{{"pid":{CLAUDE_PID},"sessionId":"{SESSION}","cwd":"{cwd_s}","status":"idle","procStart":"Sat Oct 24 08:12:34 2026","hostSessionId":"local_ny13","entrypoint":"claude-desktop","kind":"interactive"}}"#
        ),
    );
    // The Claude app's own record: its last turn is ready for review,
    // last looked at before that turn ended.
    write(
        &instance.join("claude-code-sessions/acct/org/local_ny13.json"),
        &format!(
            r#"{{"sessionId":"local_ny13","cliSessionId":"{SESSION}","cwd":"{cwd_s}","title":"Tidy the README","isArchived":false,"lastActivityAt":1793523605000,"lastFocusedAt":1793520000000,"postTurnSummary":{{"status_category":"review_ready","needs_action":"words","status_detail":"words"}}}}"#
        ),
    );
    let mut table = Table::new();
    let flag = format!("--user-data-dir={}", instance.display());
    put(&mut table, 1, 0, "/sbin/launchd", &[], 0);
    put(
        &mut table,
        APP_PID,
        1,
        CLAUDE_APP,
        &[CLAUDE_APP, &flag],
        STARTED - 100,
    );
    put(
        &mut table,
        HELPER_PID,
        APP_PID,
        "/Applications/Claude.app/Contents/Helpers/disclaimer",
        &[],
        STARTED - 50,
    );
    let exe = format!(
        "{}/claude-code/2.1/x/claude.app/Contents/MacOS/claude",
        instance.display()
    );
    put(
        &mut table,
        CLAUDE_PID,
        HELPER_PID,
        &exe,
        &["claude"],
        STARTED,
    );
    let os = Scripted(table);

    // One tick, as the app's threads run it: the files, what's open, and
    // Needs you worked out.
    let tick = || {
        conversations::tick(&state).unwrap();
        conversations::live_tick_on(&state, &os, &home).unwrap();
        needs::refresh(&state);
    };
    let rows = || -> Vec<(String, String)> {
        let storage = state.storage.lock().unwrap();
        centre::open_needs(&storage, Timestamp::now().millis(), tray::NEEDS_LISTED)
            .into_iter()
            .filter(|row| row.target.conversation_id.as_deref() == Some(SESSION))
            .map(|row| (row.kind, row.title))
            .collect()
    };
    let bell = || views::summary_now(&state).count;

    // 1. The adapter's read: open in the Claude app, "Done, review it".
    tick();
    let row = commands::list_impl(&state)
        .unwrap()
        .into_iter()
        .find(|c| c.id == SESSION)
        .expect("the conversation is indexed");
    assert!(row.live, "open in its Claude app instance");
    assert_eq!(row.state.as_deref(), Some("done_review"), "the app's word");

    // 2. Needs you holds it, and the bell counts it.
    let before = rows();
    assert_eq!(before.len(), 1, "finished, not looked at: {before:?}");
    assert_eq!(before[0].0, "review");
    let counted = bell();
    assert!(counted >= 1, "the bell counts it");

    // 3. Archive is refused while it's open (only ended sessions are
    // archived; the UI shows it disabled with these words), and nothing
    // moves.
    let refused = archive::set(&state, &account.id, SESSION, true).unwrap_err();
    match refused {
        AppError::Refused(words) => assert!(words.contains("open in its app"), "{words}"),
        other => panic!("refused with its reason, got {other:?}"),
    }
    assert!(archive::archived(&state).is_empty());
    assert_eq!(rows().len(), 1, "still in Needs you");
    assert_eq!(bell(), counted);

    // 4. Opening it marks it seen (the window's `markSeen` saves this
    // setting): its row leaves Needs you and the bell drops by one at once.
    let seen = format!(
        r#"{{"c:{}/{SESSION}":{}}}"#,
        account.id,
        Timestamp::now().millis()
    );
    juggler_app::settings_policy::app_setting_set_impl(&state, "conversation_seen", &seen).unwrap();
    assert!(rows().is_empty(), "seen: out of Needs you at once");
    assert_eq!(bell(), counted - 1, "the bell dropped by one at once");

    // ... and the next tick keeps it out (still open, still review-ready).
    tick();
    assert!(rows().is_empty(), "the tick doesn't bring it back");
    assert_eq!(bell(), counted - 1);
}
