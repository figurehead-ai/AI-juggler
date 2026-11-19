//! The 0.4.6 rc2 walk (main's ruling: "Open now must show what is actually
//! open, where it's open"): a Claude app conversation was continued in AI
//! Juggler's terminal and stopped there, then opened again in its Claude
//! app profile. While the app's own `claude` ran it, every list showed it
//! only as the ended terminal run under the CLI App, and the profile said
//! "nothing open".
//!
//! End to end over the real code (only the process table is scripted): the
//! adapter reads the pid file and the app's records through the live
//! scanner, the index joins them to the transcripts, and the conversation
//! list says where each is. While the app's process runs the conversation
//! (the same id, or the app's copy of it under its own id), the row is
//! open, its App is the Claude app profile, and no ended AI Juggler session
//! stands in for it. Before (the terminal run ended, nothing open), the
//! ended rule of 0.4.6 holds: the terminal run stands for it.
//!
//! Runs as a child on `HOME=<lab>` so nothing of this Mac's own is read.

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
use std::sync::{Arc, Mutex};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::features::conversations::dto::ConversationInfo;
use juggler_app::features::conversations::procs::{Proc, Table};
use juggler_app::features::conversations::scan::Os;
use juggler_app::features::conversations::{self, commands};
use juggler_app::state::AppState;
use juggler_core::model::{
    ConversationId, InstanceKind, InstanceRecord, ProfileId, SessionId, SessionStatus, Timestamp,
    UsageSnapshot,
};

const CHILD: &str = "AIJ_TEST_OPEN_IN_APP_CHILD";
const CLAUDE_APP: &str = "/Applications/Claude.app/Contents/MacOS/Claude";
/// The conversation the Claude app started, then continued in AI
/// Juggler's terminal.
const ECHO: &str = "0a1b2c3d-0000-4000-8000-00000000ec01";
/// The app's copy of it under its own id (a fork: its file starts with
/// the original's lines).
const COPY: &str = "0a1b2c3d-0000-4000-8000-00000000ec02";
/// Another conversation of the profile's with the same title, its own
/// from the start (the rc2 walk's 8307935d: 2026-11-10, record
/// `local_ef4f5f0d`), no AI Juggler session on it.
const OTHER: &str = "0a1b2c3d-0000-4000-8000-00000000ec03";
/// AI Juggler's terminal run of [`ECHO`], stopped by the user.
const RUN: &str = "sess-echo-terminal";
/// `Sat Oct 24 08:12:34 2026` UTC, the app's `claude` process's start.
const STARTED: i64 = 1_792_829_554;
/// High pids: never one a real `claude` holds.
const APP_PID: u32 = 990_110;
const HELPER_PID: u32 = 990_111;
const CLAUDE_PID: u32 = 990_112;

/// The process table, changed between ticks.
struct Scripted(Mutex<Table>);

impl Os for Scripted {
    fn table(&self) -> Table {
        self.0.lock().unwrap().clone()
    }
    fn some(&self, pids: &[u32]) -> Table {
        let table = self.0.lock().unwrap();
        pids.iter()
            .filter_map(|pid| table.get(pid).map(|p| (*pid, p.clone())))
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

/// Runs `test` again as a child on `HOME=<a new lab>` (the home is read
/// once per process); in the child, the lab.
fn in_a_lab(test: &str) -> Option<PathBuf> {
    if std::env::var_os(CHILD).is_some() {
        return Some(PathBuf::from(std::env::var_os("HOME").unwrap()));
    }
    let lab = tempfile::Builder::new()
        .prefix("aij-open-in-app-")
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

/// One line of a Claude app conversation's transcript.
fn line(session: &str, cwd: &str, uuid: &str, role: &str, text: &str, at: &str) -> String {
    format!(
        r#"{{"parentUuid":null,"isSidechain":false,"userType":"external","entrypoint":"claude-desktop","cwd":"{cwd}","sessionId":"{session}","version":"2.1.325","type":"{role}","message":{{"role":"{role}","content":"{text}"}},"uuid":"{uuid}","timestamp":"{at}"}}"#
    )
}

/// The lab: the Claude app's profile `aij-test` (its records list both
/// conversations), its Claude Code folder with both transcripts, and AI
/// Juggler's stopped terminal run of [`ECHO`]. The app runs; its `claude`
/// for `resumed` is in the table only once [`Lab::open_in_app`] puts it.
struct Lab {
    state: Arc<AppState>,
    home: PathBuf,
    os: Scripted,
    exe: String,
    resumed: &'static str,
    /// The Claude app profile's App id.
    app: String,
}

impl Lab {
    fn new(home: &Path, resumed: &'static str) -> Self {
        let home = home.canonicalize().unwrap();
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(ClaudeCodeAdapter::with_bin(
            home.join("no-claude"),
        )));
        let state =
            Arc::new(AppState::init_with_registry(home.join("app-data"), registry).unwrap());
        let account = juggler_app::features::account::create_account(
            &state,
            CLAUDE_CODE_ID,
            "aij-test",
            None,
        )
        .unwrap();
        let profile =
            juggler_app::features::profile::create_profile(&state, &account.id, "Main", None, None)
                .unwrap();
        let folder = state.accounts_dir().join(CLAUDE_CODE_ID).join(&account.id);
        let instance = home.join("Library/Application Support/ClaudeProfiles/aij-test");
        let cwd = home.join("lab/web");
        std::fs::create_dir_all(&cwd).unwrap();
        let cwd_s = cwd.display().to_string();
        let projects = folder.join("projects").join(cwd_s.replace('/', "-"));
        let original = [
            line(
                ECHO,
                &cwd_s,
                "e-1",
                "user",
                "echo lab",
                "2026-11-10T22:42:09.000Z",
            ),
            line(
                ECHO,
                &cwd_s,
                "e-2",
                "assistant",
                "lab",
                "2026-11-10T22:42:13.000Z",
            ),
        ];
        write(
            &projects.join(format!("{ECHO}.jsonl")),
            &format!("{}\n", original.join("\n")),
        );
        // The copy: the original's lines, then its own.
        write(
            &projects.join(format!("{COPY}.jsonl")),
            &format!(
                "{}\n{}\n",
                original.join("\n"),
                line(
                    COPY,
                    &cwd_s,
                    "c-1",
                    "user",
                    "again",
                    "2026-11-16T17:46:41.000Z"
                )
            ),
        );
        write(
            &projects.join(format!("{OTHER}.jsonl")),
            &format!(
                "{}\n{}\n",
                line(
                    OTHER,
                    &cwd_s,
                    "o-1",
                    "user",
                    "echo lab",
                    "2026-11-11T01:22:12.482Z"
                ),
                line(
                    OTHER,
                    &cwd_s,
                    "o-2",
                    "assistant",
                    "lab",
                    "2026-11-11T01:22:16.041Z"
                ),
            ),
        );
        // The Claude app's own records of all three (the other one focused
        // as the app opens it).
        for (local, cli, focused) in [
            ("local_ec01", ECHO, 1_794_368_628_175_i64),
            ("local_ec02", COPY, 1_794_368_628_175),
            ("local_ec03", OTHER, 1_794_869_198_818),
        ] {
            write(
                &instance.join(format!("claude-code-sessions/acct/org/{local}.json")),
                &format!(
                    r#"{{"sessionId":"{local}","cliSessionId":"{cli}","cwd":"{cwd_s}","title":"Echo lab","isArchived":false,"lastActivityAt":1794368535070,"lastFocusedAt":{focused},"spawnedFrom":null}}"#
                ),
            );
        }
        // "Continue in AI Juggler" ran it in a terminal; Stop ended it.
        let machine = juggler_app::commands::local_machine_id(&state);
        {
            let storage = state.storage.lock().unwrap();
            storage
                .sessions()
                .insert(&InstanceRecord {
                    id: SessionId::new(RUN),
                    conversation: Some(ConversationId::new(ECHO)),
                    profile: ProfileId::new(profile.id.as_str()),
                    machine,
                    kind: InstanceKind::Terminal,
                    project: None,
                    cwd: cwd.clone(),
                    status: SessionStatus::Stopped,
                    started_at: Timestamp::from_millis(1_794_869_150_088),
                    last_activity: Some(Timestamp::from_millis(1_794_869_178_013)),
                    usage: UsageSnapshot::default(),
                    attention: None,
                    running: None,
                    window: None,
                    transcript_ref: None,
                })
                .unwrap();
        }
        // The app's `claude` for the conversation records its pid file as
        // it starts (it is in the table once the app opens it).
        write(
            &folder.join(format!("sessions/{CLAUDE_PID}.json")),
            &format!(
                r#"{{"pid":{CLAUDE_PID},"sessionId":"{resumed}","cwd":"{cwd_s}","status":"idle","procStart":"Sat Oct 24 08:12:34 2026","startedAt":1792829554000,"hostSessionId":"local_ec0{}","entrypoint":"claude-desktop","kind":"interactive","name":"Echo lab","version":"2.1.325","peerProtocol":1}}"#,
                &resumed[resumed.len() - 1..]
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
        let exe = format!(
            "{}/claude-code/2.1.325/x/claude.app/Contents/MacOS/claude",
            instance.display()
        );
        let app = juggler_app::features::places::book::place_id("claude-app", Some(&instance));
        Self {
            state,
            home,
            os: Scripted(Mutex::new(table)),
            exe,
            resumed,
            app,
        }
    }

    /// The Claude app's `claude` for the conversation runs now, under the
    /// app's process ("Open in Claude app").
    fn open_in_app(&self) {
        let resume = format!("--resume={}", self.resumed);
        let code = [
            self.exe.as_str(),
            "--output-format",
            "stream-json",
            "--verbose",
            "--input-format",
            "stream-json",
            &resume,
        ];
        let mut table = self.os.0.lock().unwrap();
        // The app's own wrapper, then its `claude` (rc2: 55939, 55940).
        let disclaimer = "/Applications/Claude.app/Contents/Helpers/disclaimer";
        let wrapper: Vec<&str> = [disclaimer, "--pgroup", "--"]
            .into_iter()
            .chain(code)
            .collect();
        put(
            &mut table, HELPER_PID, APP_PID, disclaimer, &wrapper, STARTED,
        );
        put(
            &mut table, CLAUDE_PID, HELPER_PID, &self.exe, &code, STARTED,
        );
    }

    /// One tick, as the app's threads run it: the files, then what's open.
    fn tick(&self) {
        conversations::tick(&self.state).unwrap();
        conversations::live_tick_on(&self.state, &self.os, &self.home).unwrap();
    }

    fn row(&self, id: &str) -> ConversationInfo {
        commands::list_impl(&self.state)
            .unwrap()
            .into_iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("{id} is indexed"))
    }
}

/// While the Claude app's own process runs `id`: open, in the Claude app
/// profile, its own row (no ended AI Juggler session stands in for it),
/// in the window's working set too.
fn open_in_the_app(lab: &Lab, id: &str) {
    let row = lab.row(id);
    assert!(row.live, "{id}: the app's claude runs it: {row:?}");
    assert_eq!(
        row.app.as_deref(),
        Some(lab.app.as_str()),
        "{id}: its App is the Claude app profile it's open in"
    );
    assert_eq!(
        row.session_id, None,
        "{id}: open in the app, it's its own row, not the ended terminal run's"
    );
    assert_eq!(row.state.as_deref(), Some("idle"));
    let working = commands::working_impl(&lab.state, Timestamp::now().millis()).unwrap();
    let shown = working
        .iter()
        .find(|c| c.id == id)
        .expect("in the window's set");
    assert!(shown.live && shown.session_id.is_none() && shown.app == row.app);
}

/// Before the app opens it: the terminal run ended, nothing open; the
/// ended rule (0.4.6) files it by that run (the rail shows the run's row,
/// under the CLI App).
fn ended_as_the_terminal_run(lab: &Lab) {
    let row = lab.row(ECHO);
    assert!(!row.live, "nothing runs it yet: {row:?}");
    assert_eq!(row.session_id.as_deref(), Some(RUN), "its terminal run");
}

#[test]
fn a_conversation_the_claude_app_reopens_after_a_terminal_run_is_open_in_that_app() {
    let Some(home) =
        in_a_lab("a_conversation_the_claude_app_reopens_after_a_terminal_run_is_open_in_that_app")
    else {
        return;
    };
    let lab = Lab::new(&home, ECHO);
    lab.tick();
    ended_as_the_terminal_run(&lab);

    lab.open_in_app();
    lab.tick();
    open_in_the_app(&lab, ECHO);
}

#[test]
fn the_claude_app_s_copy_of_a_conversation_after_its_terminal_run_is_open_in_that_app() {
    let Some(home) = in_a_lab(
        "the_claude_app_s_copy_of_a_conversation_after_its_terminal_run_is_open_in_that_app",
    ) else {
        return;
    };
    let lab = Lab::new(&home, COPY);
    lab.tick();
    ended_as_the_terminal_run(&lab);

    lab.open_in_app();
    lab.tick();
    open_in_the_app(&lab, COPY);
    // The original stays its terminal run's: ended, under the CLI App.
    ended_as_the_terminal_run(&lab);
}

#[test]
fn another_conversation_the_claude_app_opens_after_a_terminal_run_is_open_in_that_app() {
    let Some(home) = in_a_lab(
        "another_conversation_the_claude_app_opens_after_a_terminal_run_is_open_in_that_app",
    ) else {
        return;
    };
    let lab = Lab::new(&home, OTHER);
    lab.tick();
    // The profile is one of the user's Apps (rc2: aij-test, added).
    juggler_app::features::places::answer_offer(&lab.state, &lab.app, true).unwrap();
    lab.tick();
    ended_as_the_terminal_run(&lab);
    assert!(!lab.row(OTHER).live);

    lab.open_in_app();
    lab.tick();
    open_in_the_app(&lab, OTHER);
    ended_as_the_terminal_run(&lab);
}

/// The rc2 walk's own order: the profile wasn't running; "Open in Claude
/// app" started it (pid 52844, between the walk's pids of 17:42 and
/// 17:46), and its `claude` came a few seconds later, after the live
/// view's last full read of the process table.
#[test]
fn another_conversation_in_a_profile_the_open_just_started_is_open_in_it() {
    let Some(home) =
        in_a_lab("another_conversation_in_a_profile_the_open_just_started_is_open_in_it")
    else {
        return;
    };
    let lab = Lab::new(&home, OTHER);
    let app = lab.os.0.lock().unwrap().remove(&APP_PID).unwrap();
    lab.tick();
    ended_as_the_terminal_run(&lab);

    lab.os.0.lock().unwrap().insert(APP_PID, app);
    lab.open_in_app();
    lab.tick();
    open_in_the_app(&lab, OTHER);
}

/// F-rc3g-4 (the 0.4.6 rc3g walk): after Stop, g-turn's rail row read
/// "Reply with the single word l…", its first prompt, for a moment: its
/// conversation stood as its own row (an ended session's conversation
/// that is open now; the live pass hadn't yet seen its process go), under
/// its transcript's title. The name the person gave a session of AI
/// Juggler's on a conversation names that conversation in every row, its
/// own row too, over the transcript's and its app's titles.
#[test]
fn a_name_the_person_gave_its_session_names_the_conversation_standing_on_its_own() {
    let Some(home) =
        in_a_lab("a_name_the_person_gave_its_session_names_the_conversation_standing_on_its_own")
    else {
        return;
    };
    let lab = Lab::new(&home, ECHO);
    juggler_app::manage::session_rename_impl(&lab.state, RUN, Some("echo work")).unwrap();
    lab.tick();
    ended_as_the_terminal_run(&lab);
    assert_eq!(lab.row(ECHO).title.as_deref(), Some("echo work"));

    // Open now, the conversation is its own row: named as the person
    // named it, never "Echo lab" (its app's title, its first prompt).
    lab.open_in_app();
    lab.tick();
    open_in_the_app(&lab, ECHO);
    assert_eq!(lab.row(ECHO).title.as_deref(), Some("echo work"));
    let working = commands::working_impl(&lab.state, Timestamp::now().millis()).unwrap();
    assert_eq!(
        working
            .iter()
            .find(|c| c.id == ECHO)
            .and_then(|c| c.title.as_deref()),
        Some("echo work")
    );
    // Another conversation in the folder keeps its own title.
    assert_ne!(lab.row(OTHER).title.as_deref(), Some("echo work"));
}
