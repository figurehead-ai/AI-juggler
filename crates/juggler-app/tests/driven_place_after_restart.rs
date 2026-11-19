//! F-rc3g-2 (the 0.4.6 rc3g walk, 2026-11-19): two Claude Code sessions
//! ran without a terminal on a CLI App; AI Juggler was quit and started
//! again from a shell inside a Claude app profile's Code tab (the walk's
//! launcher), and the restart pickup resumed them as its children. For the
//! first seconds, the one that had taken a turn read "Running in the
//! Claude app", "Window: Claude app · (that profile)": the live view read
//! its place from its process's parent chain past AI Juggler, up to the
//! Claude app that started AI Juggler's shell, and that profile's App
//! recorded the conversation as its own. Go to it would have brought that
//! window forward. Once the shell ended and launchd took AI Juggler over,
//! the page said "Runs in AI Juggler Dev".
//!
//! End to end over the real code: the app core runs a driven session
//! (`fake-claude`) to its first reply in a child and is killed there; a
//! new core on the same data picks it up; the live view reads the pid
//! file Claude Code keeps for it and the conversation list says where it
//! runs. Only the process table is scripted (AI Juggler's own process, the
//! resumed `fake-claude` as its child, and the Claude app profile whose
//! Code-tab shell started AI Juggler): AI Juggler's own session runs in
//! AI Juggler, in that window and after it, and the profile's App neither
//! lists it nor takes its account.
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

#[path = "support/set_up.rs"]
mod set_up;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_adapter::{AdapterRegistry, Runtime};
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::features::conversations::dto::ConversationInfo;
use juggler_app::features::conversations::places::own_process;
use juggler_app::features::conversations::procs::{self, Proc, Table};
use juggler_app::features::conversations::scan::Os;
use juggler_app::features::conversations::{self, commands};
use juggler_app::features::places::book::{conversation_key, place_id};
use juggler_app::features::runtime::{commands as runtime, pickup};
use juggler_app::state::AppState;
use juggler_core::model::{ProfileId, SessionId};

/// Set in the child that runs on the lab's home.
const CHILD: &str = "AIJ_TEST_DRIVEN_PLACE_CHILD";
/// Set in the app core that runs (and is killed) before the restart: its
/// data folder (a short one, under the system's temporary folder: a
/// driven session's socket must fit; the lab's home is longer).
const CORE: &str = "AIJ_TEST_DRIVEN_PLACE_CORE";
const CLAUDE_APP: &str = "/Applications/Claude.app/Contents/MacOS/Claude";
const DISCLAIMER: &str = "/Applications/Claude.app/Contents/Helpers/disclaimer";
/// High pids: never one a real process holds.
const APP_PID: u32 = 990_210;
const HELPER_PID: u32 = 990_211;
const TAB_PID: u32 = 990_212;
const SHELL_PID: u32 = 990_213;
/// The Claude app profile's start, before AI Juggler's.
const APP_STARTED: i64 = 1_795_000_000;

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

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Runs `test` again as a child on `HOME=<a new lab>` (the home is read
/// once per process); in the child, the lab.
fn in_a_lab(test: &str) -> Option<PathBuf> {
    if std::env::var_os(CHILD).is_some() {
        return Some(PathBuf::from(std::env::var_os("HOME").unwrap()));
    }
    let lab = tempfile::Builder::new()
        .prefix("aij-driven-place-")
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

fn fake_claude_bin() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    ["debug", "release"]
        .iter()
        .map(|profile| target.join(profile).join("fake-claude"))
        .find(|candidate| candidate.exists())
        .unwrap_or_else(|| panic!("fake-claude not found under {}", target.display()))
}

/// The app core on the data in `data`, with Claude Code as `fake-claude`.
fn core(data: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(fake_claude_bin())));
    Arc::new(AppState::init_with_registry(data.join("app-data"), registry).unwrap())
}

/// The lab's home (this child's).
fn lab_home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap())
        .canonicalize()
        .unwrap()
}

/// The folder of the account `profile` runs on.
fn folder_of(state: &Arc<AppState>, profile: &str) -> (String, PathBuf) {
    let storage = state.storage.lock().unwrap();
    let recipe = storage.profiles().get(&ProfileId::new(profile)).unwrap();
    let account = storage.accounts().get(&recipe.account).unwrap();
    (account.id.to_string(), account.state_dir)
}

/// The Lab: a Claude Code account on `<lab>/team` that may run without a
/// terminal (the Beta on, the folder trusted as Claude Code records it).
fn lab_profile(state: &Arc<AppState>) -> String {
    let team = lab_home().join("team");
    std::fs::create_dir_all(&team).unwrap();
    let account = set_up::set_up(
        state,
        juggler_app::features::account::create_account(state, CLAUDE_CODE_ID, "Lab", None).unwrap(),
    );
    let profile = juggler_app::features::profile::create_profile(
        state,
        &account.id,
        "lab",
        None,
        Some(team.to_string_lossy().into_owned()),
    )
    .unwrap()
    .id;
    runtime::beta_impl(state, true).unwrap();
    let file = folder_of(state, &profile).1.join(".claude.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    config["projects"] =
        serde_json::json!({ team.to_string_lossy(): {"hasTrustDialogAccepted": true} });
    std::fs::write(&file, config.to_string()).unwrap();
    profile
}

/// The app core before the quit, in a child: a session driven on the Lab
/// takes a turn (the walk's g-turn), its ids are written down, then it
/// waits to be killed (and ends by itself after a minute).
#[test]
#[ignore = "run by the test below, in a child process"]
fn the_core_before_the_quit() {
    let data = PathBuf::from(std::env::var(CORE).unwrap());
    let state = core(&data);
    let profile = lab_profile(&state);
    let launched = juggler_app::features::launch::launch_profile_as(
        &state,
        &profile,
        None,
        Some(Runtime::Driven),
    )
    .unwrap();
    let session = launched.session_id;
    runtime::send_impl(&state, &session, "Reply with the single word lab.").unwrap();
    wait_for("the reply", || {
        let snap = state.runtime.driver(&session).unwrap().snapshot();
        !snap.turn_running && snap.items.iter().any(|item| item.kind == "agent_message")
    });
    let conversation = state
        .runtime
        .driver(&session)
        .unwrap()
        .snapshot()
        .native_id
        .unwrap();
    wait_for("its mark: written, between turns", || {
        pickup::marks(&state.storage.lock().unwrap())
            .get(&session)
            .is_some_and(|mark| mark.native_id == conversation && mark.written && !mark.in_turn)
    });
    let partial = data.join("core-ready.tmp");
    std::fs::write(
        &partial,
        format!("{session} {conversation} {profile} {}", launched.pid),
    )
    .unwrap();
    std::fs::rename(&partial, data.join("core-ready")).unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

/// Whether process `pid` is alive.
fn alive(pid: u32) -> bool {
    procs::some(&[pid]).contains_key(&pid)
}

/// Runs [`the_core_before_the_quit`] in a child and kills it (the quit):
/// the session, its conversation and its profile.
fn run_then_quit(data: &Path) -> (String, String, String) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "the_core_before_the_quit",
            "--ignored",
            "--nocapture",
        ])
        .env(CORE, data)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let ready = data.join("core-ready");
    let deadline = Instant::now() + Duration::from_secs(60);
    while !ready.is_file() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("the app core ended before its session was ready: {status}");
        }
        assert!(Instant::now() < deadline, "the app core never got ready");
        std::thread::sleep(Duration::from_millis(50));
    }
    let text = std::fs::read_to_string(&ready).unwrap();
    let words: Vec<&str> = text.split_whitespace().collect();
    let tool: u32 = words[3].parse().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    // Its input closed with the core: the tool ends on its own.
    wait_for("the tool to end with the app core", || !alive(tool));
    (
        words[0].to_owned(),
        words[1].to_owned(),
        words[2].to_owned(),
    )
}

/// The resumed tool: this process's child that resumes `conversation`.
fn resumed(conversation: &str) -> Proc {
    let own = std::process::id();
    let mut found = None;
    wait_for("the resumed tool's process", || {
        found = procs::table().into_values().find(|process| {
            process.ppid == Some(own) && process.args.iter().any(|arg| arg == conversation)
        });
        found.is_some()
    });
    found.unwrap()
}

fn row(state: &Arc<AppState>, id: &str) -> ConversationInfo {
    commands::list_impl(state)
        .unwrap()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("{id} is indexed"))
}

/// One tick, as the app's threads run it: the files, then what's open.
fn tick(state: &Arc<AppState>, os: &Scripted, home: &Path) {
    conversations::tick(state).unwrap();
    conversations::live_tick_on(state, os, home).unwrap();
}

#[test]
fn a_driven_session_resumed_after_a_restart_runs_in_ai_juggler_not_the_app_that_started_it() {
    let Some(home) = in_a_lab(
        "a_driven_session_resumed_after_a_restart_runs_in_ai_juggler_not_the_app_that_started_it",
    ) else {
        return;
    };
    let home = home.canonicalize().unwrap();
    let data = tempfile::Builder::new()
        .prefix("aij-f2-")
        .tempdir()
        .unwrap();
    let (session, conversation, profile) = run_then_quit(data.path());

    // AI Juggler again, on the same data: the pickup resumes the session
    // as this process's child.
    let state = core(data.path());
    pickup::after_restart(&state);
    wait_for("the session to come back driven", || {
        state.runtime.driver(&session).is_some()
    });
    let tool = resumed(&conversation);
    // The pid file Claude Code keeps for the run, in its account's folder.
    let (lab_account, folder) = folder_of(&state, &profile);
    let team = home.join("team");
    std::fs::create_dir_all(folder.join("sessions")).unwrap();
    std::fs::write(
        folder.join(format!("sessions/{}.json", tool.pid)),
        format!(
            r#"{{"pid":{},"sessionId":"{conversation}","cwd":"{}","status":"idle","entrypoint":"sdk-cli","version":"2.1.325"}}"#,
            tool.pid,
            team.display()
        ),
    )
    .unwrap();

    // The walk's launcher: a shell in a Claude app profile's Code tab
    // started AI Juggler, which runs the resumed tool.
    let instance = home.join("Library/Application Support/ClaudeProfiles/b-work");
    let profile_app = place_id("claude-app", Some(&instance));
    let code_tab = format!(
        "{}/claude-code/2.1.325/x/claude.app/Contents/MacOS/claude",
        instance.display()
    );
    let ours = format!(
        "{}/Applications/AI Juggler Dev.app/Contents/MacOS/juggler-app",
        home.display()
    );
    let (own, own_start) = own_process();
    let own_start = own_start.expect("this process's start time");
    let mut table = Table::new();
    put(&mut table, 1, 0, "/sbin/launchd", &[], 0);
    let flag = format!("--user-data-dir={}", instance.display());
    put(
        &mut table,
        APP_PID,
        1,
        CLAUDE_APP,
        &[CLAUDE_APP, &flag],
        APP_STARTED,
    );
    put(
        &mut table,
        HELPER_PID,
        APP_PID,
        DISCLAIMER,
        &[DISCLAIMER],
        APP_STARTED,
    );
    put(
        &mut table,
        TAB_PID,
        HELPER_PID,
        &code_tab,
        &[&code_tab],
        APP_STARTED + 5,
    );
    put(
        &mut table,
        SHELL_PID,
        TAB_PID,
        "/bin/zsh",
        &["/bin/zsh", "-c", "lab_env.sh"],
        own_start,
    );
    put(&mut table, own, SHELL_PID, &ours, &[&ours], own_start);
    let args: Vec<&str> = tool.args.iter().map(String::as_str).collect();
    let exe = tool.exe.clone().unwrap();
    put(
        &mut table,
        tool.pid,
        own,
        &exe.to_string_lossy(),
        &args,
        tool.start.unwrap(),
    );
    let os = Scripted(Mutex::new(table));

    let in_ai_juggler = |when: &str| {
        let row = row(&state, &conversation);
        assert!(row.live, "{when}: it runs: {row:?}");
        assert_eq!(row.session_id.as_deref(), Some(session.as_str()), "{when}");
        let place = &row.places[0];
        assert_eq!(
            (
                place.app_id.as_deref(),
                place.instance.as_deref(),
                place.kind.as_deref()
            ),
            (None, None, None),
            "{when}: never the Claude app profile above AI Juggler: {place:?}"
        );
        assert_eq!(
            (place.app.as_str(), place.app_pid),
            ("AI Juggler Dev", Some(own)),
            "{when}: in AI Juggler, its window the one Go to it brings forward"
        );
        assert_eq!(
            row.app.as_deref(),
            Some(juggler_core::storage::cli_app_id(&lab_account).as_str()),
            "{when}: its App is its folder's CLI App"
        );
        let (listed, account) = juggler_app::features::places::with_book(&state, |book| {
            let place = book.places.get(&profile_app).expect("the profile is seen");
            (place.conversations.clone(), place.account_id.clone())
        })
        .unwrap();
        assert!(
            !listed.contains(&conversation_key(CLAUDE_CODE_ID, &conversation)),
            "{when}: the profile's App doesn't list it: {listed:?}"
        );
        assert_ne!(
            account.as_deref(),
            Some(lab_account.as_str()),
            "{when}: nor takes its account"
        );
    };

    // The window: AI Juggler still the shell's child.
    tick(&state, &os, &home);
    in_ai_juggler("while the shell that started AI Juggler runs");

    // The shell ends; launchd takes AI Juggler over.
    os.0.lock().unwrap().get_mut(&own).unwrap().ppid = Some(1);
    conversations::look_for_started(&state);
    tick(&state, &os, &home);
    in_ai_juggler("after");

    let session = SessionId::new(session.as_str());
    state
        .supervisor
        .stop(&session, Duration::from_secs(2))
        .unwrap();
    wait_for("the session to end", || {
        state.supervisor.is_finished(&session)
    });
}
