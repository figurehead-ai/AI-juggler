//! AJ-E1539: the live switching lab. Real Claude Code and Codex, real
//! accounts, a throwaway project, and AI Juggler's own switch code.
//!
//! Ignored, and a no-op unless every `AIJ_LAB_*` variable below is set, so
//! it never runs in CI or on a plain `cargo test`. It starts real sessions
//! and spends a little of each account's usage: run it only on accounts
//! whose owner agreed, and only from a clean environment with no
//! `CLAUDE_CONFIG_DIR` or `CODEX_HOME` set (the lab sets them per command):
//!
//! ```sh
//! cargo test -p juggler-app --test switch_lab_live --no-run
//! env -u CLAUDE_CONFIG_DIR -u CODEX_HOME \
//!   AIJ_LAB_DATA=<fresh empty folder> AIJ_LAB_PROJECT=<new folder> \
//!   AIJ_LAB_CLAUDE_A=<Claude config folder> AIJ_LAB_CLAUDE_B=<another one> \
//!   AIJ_LAB_CODEX=<Codex home> AIJ_LAB_OUT=<report.json> \
//!   <test binary> --ignored --nocapture
//! ```
//!
//! The journey (the maintainer's, 2026-10-22): a small coding task on Claude
//! account A, switched to Claude account B, then to Codex, then back to A.
//! After every switch a probe asks what only the earlier conversation knows
//! (a codename chosen in the first turn, the name picked for the next
//! function, the open tasks), and the lab checks the answer, the files and
//! the tests. The first turn, the probes and the continuations run headless
//! (`claude -p`, `codex exec`): that is the same tool and account the switch
//! prepared, without the folder-trust prompts an interactive start shows.
//! The interactive session each switch starts is stopped as soon as its
//! command line has been read.
//!
//! With `AIJ_LAB_PROBE=<name>` it also checks the switch's set-up step
//! (AJ-E1542): account A alone must have an MCP server and a skill of that
//! name. The server's tool `probe_word` answers `KESTREL` (only with its
//! environment carried), and the skill's word is `PEREGRINE`. Before each
//! switch the lab sets up only those two on the target, asks the target
//! for both words in a fresh session (the switched conversation already
//! holds the words, so asking there would prove nothing), and at the end
//! undoes every set-up.
//!
//! It never signs in, never reads a credential, and starts nothing outside
//! the lab project. Before and after, it records every account folder's
//! paths, sizes and modification times (never contents); the report lists
//! what changed. The report holds real paths: keep it out of the repository.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID};
use juggler_app::commands;
use juggler_app::state::AppState;
use serde_json::{Value, json};

/// The codename the first turn is asked to remember.
const CODENAME: &str = "BLUE-HERON";
/// The name chosen on account B for the multiply function.
const MUL_NAME: &str = "times";
/// What the probe MCP server's tool answers, and the probe skill's word.
const MCP_WORD: &str = "KESTREL";
const SKILL_WORD: &str = "PEREGRINE";
/// A release name chosen on Codex and only said, never written to a
/// file: switching back can know it only from the conversation.
const RELEASE: &str = "Copper Kettle";
/// Each tool's own rules file holds a rule only it states: the other tool
/// can know it only from the handover (AJ-E1541 Req.2).
const CLAUDE_RULE: &str =
    "Rule for this project: every Python function's docstring starts with `LAB:`.";
const AGENTS_RULE: &str =
    "Rule for this project: every unittest test method name ends with `_lab`.";
/// The codename a Codex-started task chooses (the forward Codex → Claude hop).
const CODENAME_X: &str = "OSPREY";
/// The worktree leg's codename, and the helper name decided on B.
const CODENAME_WT: &str = "EGRET";
const HELPER: &str = "flip";
/// A name decided on Claude B after the forward hop, only said: switching
/// back to Codex can know it only from the conversation.
const RELEASE_X: &str = "Silver Finch";
/// The Codex → Codex leg's codename (`AIJ_LAB_CODEX_B`).
const CODENAME_XX: &str = "PLOVER";

struct Lab {
    project: PathBuf,
    claude_a: PathBuf,
    claude_b: PathBuf,
    codex: PathBuf,
    /// A second Codex home (`AIJ_LAB_CODEX_B`), for the Codex → Codex leg.
    codex_b: Option<PathBuf>,
    steps: Vec<Value>,
    /// The probe's name (`AIJ_LAB_PROBE`), when the set-up step is checked.
    probe: Option<String>,
    /// Every set-up record, undone at the end.
    records: Vec<String>,
}

/// Every file under `roots`: path → (size, modified ns). Contents are
/// never read.
fn snapshot(roots: &[PathBuf]) -> BTreeMap<String, (u64, u128)> {
    let mut out = BTreeMap::new();
    let mut stack: Vec<PathBuf> = roots.to_vec();
    while let Some(path) = stack.pop() {
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                stack.extend(entries.flatten().map(|entry| entry.path()));
            }
            continue;
        }
        let modified = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_nanos());
        out.insert(path.display().to_string(), (meta.len(), modified));
    }
    out
}

fn snapshot_diff(
    before: &BTreeMap<String, (u64, u128)>,
    after: &BTreeMap<String, (u64, u128)>,
) -> Value {
    let added: Vec<&String> = after.keys().filter(|k| !before.contains_key(*k)).collect();
    let removed: Vec<&String> = before.keys().filter(|k| !after.contains_key(*k)).collect();
    let changed: Vec<&String> = after
        .iter()
        .filter(|(k, v)| before.get(*k).is_some_and(|old| old != *v))
        .map(|(k, _)| k)
        .collect();
    json!({"added": added, "removed": removed, "changed": changed})
}

/// Runs `cmd` to completion, killing it after `limit`.
fn run(cmd: &mut Command, limit: Duration) -> (bool, String, String) {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the tool starts");
    let started = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if started.elapsed() > limit {
            let _ = child.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let out = child.wait_with_output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A headless Claude Code turn in the lab project, on the account whose
/// folder is `config`. Returns the JSON result.
fn claude_turn(
    lab: &Lab,
    config: &Path,
    session: Option<&str>,
    resume: Option<&str>,
    prompt: &str,
) -> Value {
    claude_turn_in(lab, config, &lab.project, session, resume, &[], prompt)
}

/// [`claude_turn`] in `cwd`, with extra flags (`--add-dir` for a handover).
fn claude_turn_in(
    _lab: &Lab,
    config: &Path,
    cwd: &Path,
    session: Option<&str>,
    resume: Option<&str>,
    extra: &[String],
    prompt: &str,
) -> Value {
    let mut cmd = Command::new("claude");
    cmd.current_dir(cwd)
        .env("CLAUDE_CONFIG_DIR", config)
        .arg("-p")
        .arg(prompt)
        .args([
            "--output-format",
            "json",
            "--permission-mode",
            "acceptEdits",
        ])
        .args([
            "--allowedTools",
            "Read,Edit,Write,Bash(python3 -m unittest*),Skill,Task,Agent,mcp__aij-lab-probe__probe_word",
        ])
        .args(["--max-turns", "12"])
        .args(extra);
    if let Some(id) = session {
        cmd.args(["--session-id", id]);
    }
    if let Some(id) = resume {
        cmd.args(["--resume", id]);
    }
    let (ok, stdout, stderr) = run(&mut cmd, Duration::from_secs(300));
    serde_json::from_str::<Value>(stdout.trim())
        .unwrap_or_else(|_| json!({"is_error": true, "ok": ok, "stdout": stdout, "stderr": stderr}))
}

/// A headless Codex turn in the lab project. Returns (thread id, last
/// agent message, ok).
fn codex_turn(lab: &Lab, resume: Option<&str>, prompt: &str) -> (Option<String>, String, bool) {
    codex_turn_with(lab, resume, &[], prompt)
}

/// [`codex_turn`] with extra `codex exec` flags.
fn codex_turn_with(
    lab: &Lab,
    resume: Option<&str>,
    flags: &[&str],
    prompt: &str,
) -> (Option<String>, String, bool) {
    codex_turn_in(lab, &lab.codex, resume, flags, prompt)
}

/// [`codex_turn_with`] on the Codex home `home`.
fn codex_turn_in(
    lab: &Lab,
    home: &Path,
    resume: Option<&str>,
    flags: &[&str],
    prompt: &str,
) -> (Option<String>, String, bool) {
    let mut cmd = Command::new("codex");
    cmd.current_dir(&lab.project)
        .env("CODEX_HOME", home)
        .arg("exec")
        .args(flags);
    if let Some(id) = resume {
        cmd.args(["resume", id]);
    } else {
        // Codex's automatic review (`--approve-for-me`) picks its own
        // sandbox and refuses `-s` alongside it (lab 7).
        cmd.arg("--json");
        if !flags.contains(&"--approve-for-me") {
            cmd.args(["-s", "workspace-write"]);
        }
        cmd.arg("-C").arg(&lab.project);
    }
    cmd.arg(prompt);
    let (ok, stdout, stderr) = run(&mut cmd, Duration::from_secs(300));
    let mut thread = None;
    let mut last = String::new();
    for line in stdout.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            last.push_str(line);
            last.push('\n');
            continue;
        };
        if event["type"] == "thread.started" {
            thread = event["thread_id"].as_str().map(str::to_owned);
        }
        if let Some(text) = event["item"]["text"].as_str() {
            last = text.to_owned();
        }
    }
    if last.is_empty() {
        last = stderr;
    }
    (thread, last, ok)
}

fn tests_pass(project: &Path) -> bool {
    Command::new("python3")
        .args(["-m", "unittest", "-q"])
        .current_dir(project)
        .output()
        .is_ok_and(|out| out.status.success())
}

/// The command line of a running process, read once.
fn argv_of(pid: u32) -> Vec<String> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut system = System::new();
    let pid = Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cmd(sysinfo::UpdateKind::Always),
    );
    system
        .process(pid)
        .map(|process| {
            process
                .cmd()
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn import(lab_state: &Arc<AppState>, harness: &str, folder: &Path, label: &str) -> String {
    let done = commands::account_import_mode_impl(
        lab_state,
        harness,
        &folder.display().to_string(),
        label,
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap_or_else(|error| panic!("bringing in {label}: {error:?}"));
    done.account.id.as_str().to_owned()
}

fn profile_of(state: &Arc<AppState>, account: &str) -> String {
    let storage = state.storage.lock().unwrap();
    storage
        .profiles()
        .list_by_account(&juggler_core::model::AccountId::new(account))
        .unwrap()
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{account} has launch settings"))
        .id
        .as_str()
        .to_owned()
}

/// Records a session started outside AI Juggler (the lab's first turn) as
/// an ended session of `profile`, the way "Continue in AI Juggler" would
/// before resuming it.
fn adopt(state: &Arc<AppState>, profile: &str, native: &str, cwd: &Path) -> String {
    use juggler_core::model::{
        ConversationId, InstanceKind, InstanceRecord, ProfileId, SessionId, SessionStatus,
        Timestamp,
    };
    let session = SessionId::new(format!("lab-{native}"));
    // Before the lock: it takes the storage lock itself.
    let machine = commands::local_machine_id(state);
    let storage = state.storage.lock().unwrap();
    let project = storage
        .projects()
        .touch(&juggler_core::storage::project_root(cwd), None)
        .unwrap();
    storage
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: Some(ConversationId::new(native)),
            profile: ProfileId::new(profile),
            machine,
            kind: InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd: cwd.to_path_buf(),
            status: SessionStatus::Finished,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            window: None,
            transcript_ref: None,
        })
        .unwrap();
    session.to_string()
}

fn transcript_of(
    state: &Arc<AppState>,
    session: &str,
) -> Option<juggler_core::model::TranscriptRef> {
    let storage = state.storage.lock().unwrap();
    storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session))
        .ok()
        .and_then(|record| record.transcript_ref)
}

/// Stops the interactive session a switch started, after reading its
/// command line.
fn take_launch(state: &Arc<AppState>, session: &str, pid: u32) -> Vec<String> {
    std::thread::sleep(Duration::from_millis(800));
    let argv = argv_of(pid);
    let _ = juggler_app::terminal::terminal_kill_impl(state, session);
    std::thread::sleep(Duration::from_millis(500));
    argv
}

fn mentions(text: &str, needle: &str) -> bool {
    text.to_lowercase().contains(&needle.to_lowercase())
}

#[test]
#[ignore = "the live switching lab; needs AIJ_LAB_* and real, agreed accounts"]
fn switch_lab_live() {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let (Some(data), Some(project), Some(claude_a), Some(claude_b), Some(codex), Some(out)) = (
        var("AIJ_LAB_DATA"),
        var("AIJ_LAB_PROJECT"),
        var("AIJ_LAB_CLAUDE_A"),
        var("AIJ_LAB_CLAUDE_B"),
        var("AIJ_LAB_CODEX"),
        var("AIJ_LAB_OUT"),
    ) else {
        eprintln!("AIJ_LAB_* not set: nothing to do");
        return;
    };
    assert!(
        std::env::var_os("CLAUDE_CONFIG_DIR").is_none() && std::env::var_os("CODEX_HOME").is_none(),
        "run the lab with CLAUDE_CONFIG_DIR and CODEX_HOME unset (env -u …)"
    );
    assert!(
        !data.exists() || std::fs::read_dir(&data).unwrap().next().is_none(),
        "AIJ_LAB_DATA must be a fresh, empty folder"
    );
    assert!(
        !project.exists(),
        "AIJ_LAB_PROJECT must not exist yet: the lab creates it"
    );
    std::fs::create_dir_all(&project).unwrap();
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&project)
            .output()
            .unwrap();
    };
    git(&["init", "-q"]);
    std::fs::write(
        project.join("README.md"),
        "# Switch lab\n\nA throwaway project.\n",
    )
    .unwrap();
    std::fs::write(project.join("CLAUDE.md"), format!("{CLAUDE_RULE}\n")).unwrap();
    std::fs::write(project.join("AGENTS.md"), format!("{AGENTS_RULE}\n")).unwrap();
    std::fs::write(project.join(".gitignore"), ".wt/\n").unwrap();
    git(&["add", "-A"]);
    git(&[
        "-c",
        "user.name=lab",
        "-c",
        "user.email=lab@example.com",
        "commit",
        "-qm",
        "start",
    ]);

    let home = dirs::home_dir().unwrap();
    let codex_b = var("AIJ_LAB_CODEX_B");
    let mut roots: Vec<PathBuf> = vec![
        claude_a.clone(),
        claude_b.clone(),
        codex.clone(),
        home.join(".claude.json"),
        home.join(".agents"),
    ];
    roots.extend(codex_b.clone());
    let before = snapshot(&roots);

    let state = Arc::new(AppState::init(data).unwrap());
    let mut lab = Lab {
        project: project.canonicalize().unwrap(),
        claude_a: claude_a.clone(),
        claude_b: claude_b.clone(),
        codex: codex.clone(),
        codex_b: codex_b.clone(),
        steps: Vec::new(),
        probe: std::env::var("AIJ_LAB_PROBE").ok(),
        records: Vec::new(),
    };
    let account_a = import(&state, CLAUDE_CODE_ID, &claude_a, "Lab A");
    let account_b = import(&state, CLAUDE_CODE_ID, &claude_b, "Lab B");
    let account_x = import(&state, CODEX_ID, &codex, "Lab Codex");
    let (profile_a, profile_b, profile_x) = (
        profile_of(&state, &account_a),
        profile_of(&state, &account_b),
        profile_of(&state, &account_x),
    );

    // 1. The task on Claude A.
    let native = uuid_v4();
    let first = claude_turn(
        &lab,
        &lab.claude_a.clone(),
        Some(&native),
        None,
        &format!(
            "This project's codename is {CODENAME}; remember it. Create calc.py with add(a, b), \
             and test_calc.py with a unittest for it, then run `python3 -m unittest`. Keep a task \
             list for this work with three items: add, sub, mul. Mark add done. Also use a \
             subagent (the Task tool) to count the lines of README.md, and tell me the count."
        ),
    );
    let step1_ok = first["is_error"] == false && tests_pass(&lab.project);
    lab.steps.push(
        json!({"step": "1 task on A", "ok": step1_ok, "result": first["result"],
                          "session": native}),
    );

    // 2. Switch A → B.
    let source = adopt(&state, &profile_a, &native, &lab.project);
    let preflight_b = commands::switch_preflight_impl(&state, &source, &profile_b);
    let setup_b = set_up_probe(&state, &mut lab, &source, &profile_b);
    let to_b = commands::switch_execute_impl(&state, &source, &profile_b, "stop-now", None, &[]);
    let step2 = match &to_b {
        Ok(done) => {
            let argv = take_launch(&state, &done.session_id, done.pid);
            let link = transcript_of(&state, &done.session_id);
            let resume_id = link
                .as_ref()
                .and_then(|r| r.native_id.clone())
                .unwrap_or_else(|| native.clone());
            let probe = claude_turn(
                &lab,
                &lab.claude_b.clone(),
                None,
                Some(&resume_id),
                "Without changing any file: what is this project's codename, and which items of \
                 your task list are still open? Answer in one short line.",
            );
            let answer = probe["result"].as_str().unwrap_or_default().to_owned();
            let carried = mentions(&answer, CODENAME);
            let tasks_carried = mentions(&answer, "sub") && mentions(&answer, "mul");
            // What travels beside the transcript (AJ-E1540 Req.2).
            let subagents = json!({"on_a": has_session_dir(&lab.claude_a, &native, "subagents"),
                                   "on_b": has_session_dir(&lab.claude_b, &resume_id, "subagents")});
            let file_history = json!({"on_a": lab.claude_a.join("file-history").join(&native).is_dir(),
                                      "on_b": lab.claude_b.join("file-history").join(&resume_id).is_dir()});
            let cont = claude_turn(
                &lab,
                &lab.claude_b.clone(),
                None,
                Some(&resume_id),
                &format!(
                    "Add sub(a, b) to calc.py with a test, run the tests, and mark sub done. Then \
                     decide that the multiply function will be named `{MUL_NAME}`; remember that, \
                     but don't write it yet."
                ),
            );
            let uses = lab.probe.as_ref().map(|_| {
                // A fresh session: the switched one can't have heard the words yet,
                // but the next target's copy will, so every check starts clean.
                let turn = claude_turn(&lab, &lab.claude_b.clone(), None, None, PROBE_ASK);
                probe_answer(turn["result"].as_str().unwrap_or_default())
            });
            // Only when the set-up step is checked (lab 9 ran without it).
            if lab.probe.is_some() {
                lab.steps.push(
                    json!({"step": "2b set-up on B", "ok": setup_ok(&setup_b, uses.as_ref()),
                                      "setup": setup_b, "uses": uses}),
                );
            }
            let rules_probe = claude_turn(
                &lab,
                &lab.claude_b.clone(),
                None,
                Some(&resume_id),
                "Without changing any file: what do this project's rules say about docstrings? One short line.",
            );
            let rules = mentions(rules_probe["result"].as_str().unwrap_or_default(), "LAB:");
            json!({"step": "2 switch A→B", "ok": carried && tests_pass(&lab.project),
                   "transcript_resume": carried, "tasks": tasks_carried, "rules": rules,
                   "rules_probe": rules_probe["result"], "subagents": subagents,
                   "file_history": file_history,
                   "probe": answer, "continued": cont["result"], "launch": argv,
                   "link": link.map(|r| r.path.display().to_string()), "session": done.session_id,
                   "preflight": preflight_b.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
        }
        Err(error) => json!({"step": "2 switch A→B", "ok": false, "error": format!("{error:?}"),
                            "preflight": preflight_b.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()}),
    };
    let on_b = to_b.as_ref().ok().map(|done| done.session_id.clone());
    lab.steps.push(step2);

    // 3. Switch B → Codex.
    let mut on_codex = None;
    if let Some(on_b) = &on_b {
        let preflight_x = commands::switch_preflight_impl(&state, on_b, &profile_x);
        let setup_x = set_up_probe(&state, &mut lab, on_b, &profile_x);
        let to_x =
            commands::switch_execute_impl(&state, on_b, &profile_x, "stop-now", Some("strip"), &[]);
        let step3 = match &to_x {
            Ok(done) => {
                let argv = take_launch(&state, &done.session_id, done.pid);
                // What the interactive start was primed with: a pointer at
                // the switch's handover file (AJ-E1541).
                let brief = argv
                    .iter()
                    .position(|a| a == "--")
                    .and_then(|at| argv.get(at + 1))
                    .cloned()
                    .unwrap_or_default();
                let handover = handover_of(&brief);
                let (thread, answer, ok) = codex_turn(
                    &lab,
                    None,
                    &format!(
                        "{brief}\n\n---\nWithout changing any file: what is this project's \
                         codename, what name was chosen for the multiply function, which \
                         tasks are still open, and what do this project's rules say about \
                         docstrings? Answer in one short line."
                    ),
                );
                let carried = mentions(&answer, CODENAME) && mentions(&answer, MUL_NAME);
                let rules = mentions(&answer, "LAB:");
                let (_, cont, _) = match &thread {
                    Some(id) => codex_turn(
                        &lab,
                        Some(id),
                        &format!(
                            "Now add the multiply function under the name chosen earlier, with a \
                             test, run `python3 -m unittest`, and mark it done. Then tell me, \
                             only in your reply and in no file, that this version's release name \
                             is \"{RELEASE}\"."
                        ),
                    ),
                    None => (None, String::new(), false),
                };
                // A fresh session: the switched thread was primed with B's
                // conversation, which already holds both words (lab 6 passed on
                // that alone, with the tool call blocked). Codex's automatic
                // review approves the tool call; nothing else runs.
                let uses = lab.probe.as_ref().map(|_| {
                    probe_answer(&codex_turn_with(&lab, None, &["--approve-for-me"], PROBE_ASK).1)
                });
                if lab.probe.is_some() {
                    lab.steps.push(json!({"step": "3b set-up on Codex",
                                          "ok": setup_ok(&setup_x, uses.as_ref()),
                                          "setup": setup_x, "uses": uses}));
                }
                on_codex = Some((done.session_id.clone(), thread.clone()));
                json!({"step": "3 switch B→Codex", "ok": ok && carried && tests_pass(&lab.project),
                       "transcript_resume": carried, "rules": rules, "probe": answer, "continued": cont,
                       "pointer_bytes": brief.len(), "handover": handover, "thread": thread,
                       "session": done.session_id,
                       "mul_written": std::fs::read_to_string(lab.project.join("calc.py"))
                           .map(|s| s.contains(&format!("def {MUL_NAME}("))).unwrap_or(false),
                       "preflight": preflight_x.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
            }
            Err(error) => {
                json!({"step": "3 switch B→Codex", "ok": false, "error": format!("{error:?}"),
                                "preflight": preflight_x.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
            }
        };
        lab.steps.push(step3);
    }

    // 4. Switch back to A.
    if let Some((away, _thread)) = &on_codex {
        // Back to where the work started (A), not the last hop (B).
        let plan = commands::switch_back_plan_impl(&state, away);
        let start = plan.as_ref().ok().and_then(|plan| {
            plan.returns
                .iter()
                .find(|r| r.start)
                .map(|r| r.epoch_id.clone())
        });
        if let Some(probe) = &lab.probe {
            let offers = juggler_app::features::switch_setup::check_impl(&state, away, &profile_a)
                .map(|check| serde_json::to_value(check).unwrap())
                .unwrap_or_else(|error| json!({"error": format!("{error:?}")}));
            let offered = offers["offers"]
                .as_array()
                .is_some_and(|all| all.iter().any(|o| o["name"] == probe.as_str()));
            lab.steps
                .push(json!({"step": "4b A already has the probe", "ok": !offered,
                                  "check": offers}));
        }
        let back =
            commands::switch_back_execute_to_impl(&state, away, &[], false, start.as_deref());
        let step4 = match &back {
            Ok(done) => {
                let argv = take_launch(&state, &done.session_id, done.pid);
                // `claude --resume <id> "<delta>"`: the delta is the last argument.
                let delta = argv.last().cloned().unwrap_or_default();
                let probe = claude_turn(
                    &lab,
                    &lab.claude_a.clone(),
                    None,
                    Some(&native),
                    &format!(
                        "{delta}\n\n---\nWithout changing or reading any file: what is the \
                         codename, what release name was picked while you were away, and what was \
                         done meanwhile? One short line."
                    ),
                );
                let answer = probe["result"].as_str().unwrap_or_default().to_owned();
                // Only the conversation knows the release name: it was never
                // written to a file.
                let carried = mentions(&answer, CODENAME) && mentions(&answer, RELEASE);
                json!({"step": "4 switch back to A", "ok": carried && tests_pass(&lab.project),
                       "switch_back": carried, "probe": answer, "delta": delta,
                       "plan": plan.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
            }
            Err(error) => {
                json!({"step": "4 switch back to A", "ok": false, "error": format!("{error:?}"),
                                "plan": plan.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
            }
        };
        lab.steps.push(step4);
    }

    forward_codex_to_claude(&state, &mut lab, &profile_x, &profile_b);
    worktree_leg(&state, &mut lab, &profile_a, &profile_b);
    if let Some(home_b) = lab.codex_b.clone() {
        let account_xb = import(&state, CODEX_ID, &home_b, "Lab Codex B");
        codex_to_codex(
            &state,
            &mut lab,
            &profile_x,
            &profile_of(&state, &account_xb),
            &home_b,
        );
    }

    if lab.probe.is_some() {
        let undone: Vec<Value> = lab
            .records
            .iter()
            .rev()
            .map(
                |record| match juggler_app::features::switch_setup::undo_impl(&state, record) {
                    Ok(message) => json!({"record": record, "ok": true, "message": message}),
                    Err(error) => {
                        json!({"record": record, "ok": false, "error": format!("{error:?}")})
                    }
                },
            )
            .collect();
        let all = undone.iter().all(|u| u["ok"] == true) && !undone.is_empty();
        lab.steps
            .push(json!({"step": "5 undo every set-up", "ok": all, "undone": undone}));
    }

    let after = snapshot(&roots);
    let report = json!({
        "tools": {
            "claude": run(Command::new("claude").arg("--version"), Duration::from_secs(20)).1.trim(),
            "codex": run(Command::new("codex").arg("--version"), Duration::from_secs(20)).1.trim(),
        },
        "steps": lab.steps,
        "snapshot": snapshot_diff(&before, &after),
    });
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    eprintln!("lab report: {}", out.display());
    let failed: Vec<&Value> = report["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["ok"] != true)
        .collect();
    assert!(
        failed.is_empty(),
        "{} lab step(s) failed; see the report",
        failed.len()
    );
}

/// The question that shows whether a target can use the probe server and
/// skill. It changes no file.
const PROBE_ASK: &str = "Without changing any file: call the probe_word tool of the aij-lab-probe \
    MCP server, and use the aij-lab-probe skill to find the lab's skill word. Reply with both \
    words on one line.";

/// Whether an answer has both probe words.
fn probe_answer(answer: &str) -> Value {
    let blocked = [
        "blocked",
        "couldn't",
        "could not",
        "unable",
        "prior session",
    ]
    .iter()
    .any(|word| answer.to_lowercase().contains(word));
    json!({"answer": answer,
           "mcp": answer.contains(MCP_WORD) && !answer.contains("NO-ENV") && !blocked,
           "skill": answer.contains(SKILL_WORD)})
}

/// The check before a switch, then setting up only the probe's offers.
/// Everything else the target lacks is reported and left alone.
fn set_up_probe(state: &Arc<AppState>, lab: &mut Lab, session: &str, target: &str) -> Value {
    let Some(probe) = lab.probe.clone() else {
        return Value::Null;
    };
    let check = match juggler_app::features::switch_setup::check_impl(state, session, target) {
        Ok(check) => check,
        Err(error) => return json!({"error": format!("{error:?}")}),
    };
    let picked: Vec<String> = check
        .offers
        .iter()
        .filter(|offer| offer.name == probe)
        .map(|offer| offer.id.clone())
        .collect();
    let applied = juggler_app::features::switch_setup::apply_impl(state, session, target, &picked)
        .unwrap_or_default();
    lab.records
        .extend(applied.iter().filter_map(|a| a.record.clone()));
    json!({"check": check, "applied": applied})
}

/// The set-up went through: both probe offers were set up, and the target
/// answered with both words.
fn setup_ok(setup: &Value, uses: Option<&Value>) -> bool {
    let applied = setup["applied"].as_array().map_or(0, |all| {
        all.iter().filter(|a| a["record"].is_string()).count()
    });
    applied == 2 && uses.is_some_and(|u| u["mcp"] == true && u["skill"] == true)
}

/// A random v4 UUID, for `claude --session-id`.
fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    getrandom(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn getrandom(buf: &mut [u8]) {
    use std::io::Read;
    std::fs::File::open("/dev/urandom")
        .unwrap()
        .read_exact(buf)
        .unwrap();
}

/// Whether `config` holds `<native>/<sub>` for a session (in any project
/// folder Claude Code keeps it under).
fn has_session_dir(config: &Path, native: &str, sub: &str) -> bool {
    std::fs::read_dir(config.join("projects")).is_ok_and(|dirs| {
        dirs.flatten()
            .any(|dir| dir.path().join(native).join(sub).is_dir())
    })
}

/// The handover a pointer prompt names: its path and size (never its
/// contents in the report).
fn handover_of(pointer: &str) -> Value {
    match pointer.lines().find(|line| line.ends_with("HANDOVER.md")) {
        Some(path) => json!({"path": path,
                             "bytes": std::fs::metadata(path).map(|m| m.len()).ok(),
                             "names_rules": std::fs::read_to_string(path)
                                 .map(|text| text.contains("CLAUDE.md") && text.contains("AGENTS.md"))
                                 .unwrap_or(false)}),
        None => Value::Null,
    }
}

/// Step 6, the forward Codex → Claude hop: a task started on Codex, switched
/// forward to Claude account B (a replay through the handover file), and
/// asked what only the Codex conversation and its rules know.
fn forward_codex_to_claude(state: &Arc<AppState>, lab: &mut Lab, profile_x: &str, profile_b: &str) {
    let (thread, first, ok) = codex_turn(
        lab,
        None,
        &format!(
            "This project's second codename is {CODENAME_X}; remember it. Add div(a, b) to calc.py \
             with a test, run `python3 -m unittest`. Keep a plan with two items: div and mod; mark \
             div done."
        ),
    );
    let Some(thread) = thread.filter(|_| ok) else {
        lab.steps
            .push(json!({"step": "6 Codex task", "ok": false, "result": first}));
        return;
    };
    let source = adopt(state, profile_x, &thread, &lab.project);
    let to_b =
        commands::switch_execute_impl(state, &source, profile_b, "stop-now", Some("strip"), &[]);
    let step = match &to_b {
        Ok(done) => {
            let argv = take_launch(state, &done.session_id, done.pid);
            let pointer = argv
                .iter()
                .find(|arg| arg.contains("HANDOVER.md"))
                .cloned()
                .unwrap_or_default();
            let extra: Vec<String> = argv
                .iter()
                .position(|arg| arg == "--add-dir")
                .and_then(|at| argv.get(at + 1))
                .map(|dir| vec!["--add-dir".to_owned(), dir.clone()])
                .unwrap_or_default();
            let probe = claude_turn_in(
                lab,
                &lab.claude_b.clone(),
                &lab.project.clone(),
                None,
                None,
                &extra,
                &format!(
                    "{pointer}\n\n---\nWithout changing any file: what is the second codename, \
                     which plan items are still open, and what do this project's rules say about \
                     test method names? Answer in one short line."
                ),
            );
            let answer = probe["result"].as_str().unwrap_or_default().to_owned();
            let carried = mentions(&answer, CODENAME_X);
            json!({"step": "6 switch Codex→B (forward)", "ok": carried && tests_pass(&lab.project),
                   "transcript_resume": carried, "todos": mentions(&answer, "mod"),
                   "rules": mentions(&answer, "_lab"), "probe": answer, "launch": argv,
                   "add_dir": !extra.is_empty(), "handover": handover_of(&pointer),
                   "probe_session": probe["session_id"],
                   "codex_first": first})
        }
        Err(error) => {
            json!({"step": "6 switch Codex→B (forward)", "ok": false, "error": format!("{error:?}")})
        }
    };
    // The conversation B's probe started (found again by its opening).
    let on_b = step["probe_session"].as_str().map(str::to_owned);
    lab.steps.push(step);
    if let (Ok(done), Some(conversation)) = (&to_b, on_b) {
        switch_back_to_codex(state, lab, &done.session_id, &conversation, &thread);
    }
}

/// Step 6b: B decides something only said, then switches straight back to
/// the Codex task, which must know it.
fn switch_back_to_codex(
    state: &Arc<AppState>,
    lab: &mut Lab,
    away: &str,
    conversation: &str,
    thread: &str,
) {
    let said = claude_turn_in(
        lab,
        &lab.claude_b.clone(),
        &lab.project.clone(),
        None,
        Some(conversation),
        &[],
        &format!(
            "Decide that this version's release name is \"{RELEASE_X}\". Say it only in your reply, \
             in no file."
        ),
    );
    let plan = commands::switch_back_plan_impl(state, away);
    let start = plan.as_ref().ok().and_then(|plan| {
        plan.returns
            .iter()
            .find(|r| r.start)
            .map(|r| r.epoch_id.clone())
    });
    let back = commands::switch_back_execute_to_impl(state, away, &[], false, start.as_deref());
    let step = match &back {
        Ok(back) => {
            let argv = take_launch(state, &back.session_id, back.pid);
            let delta = argv.last().cloned().unwrap_or_default();
            let (_, answer, ok) = codex_turn(
                lab,
                Some(thread),
                &format!(
                    "{delta}\n\n---\nWithout changing or reading any file: what release name was \
                     picked while you were away? One short line."
                ),
            );
            json!({"step": "6b switch back B→Codex", "ok": ok && mentions(&answer, RELEASE_X),
                   "switch_back": mentions(&answer, RELEASE_X), "probe": answer, "said": said["result"],
                   "launch": argv,
                   "plan": plan.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
        }
        Err(error) => {
            json!({"step": "6b switch back B→Codex", "ok": false, "error": format!("{error:?}"),
                             "said": said["result"],
                             "plan": plan.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
        }
    };
    lab.steps.push(step);
}

/// Step 7, a switch inside a git worktree, then a direct return: A works in a
/// worktree of the project, switches to B (which must continue in the
/// worktree, not the project's root), B decides something only said, and
/// switching back resumes A there with what B decided.
fn worktree_leg(state: &Arc<AppState>, lab: &mut Lab, profile_a: &str, profile_b: &str) {
    let wt = lab.project.join(".wt").join("egret");
    let added = Command::new("git")
        .args(["worktree", "add", "-q"])
        .arg(&wt)
        .args(["-b", "lab-egret"])
        .current_dir(&lab.project)
        .output()
        .is_ok_and(|out| out.status.success());
    let wt = wt.canonicalize().unwrap_or(wt);
    if !added {
        lab.steps
            .push(json!({"step": "7 worktree", "ok": false, "error": "git worktree add failed"}));
        return;
    }
    let native = uuid_v4();
    let first = claude_turn_in(
        lab,
        &lab.claude_a.clone(),
        &wt,
        Some(&native),
        None,
        &[],
        &format!(
            "This worktree's codename is {CODENAME_WT}; remember it. Add neg(a) to calc.py with a \
             test, and run `python3 -m unittest`."
        ),
    );
    let source = adopt(state, profile_a, &native, &wt);
    let to_b = commands::switch_execute_impl(state, &source, profile_b, "stop-now", None, &[]);
    let Ok(done) = to_b else {
        lab.steps.push(
            json!({"step": "7 worktree A→B", "ok": false, "first": first["result"],
                              "error": format!("{:?}", to_b.err())}),
        );
        return;
    };
    let argv = take_launch(state, &done.session_id, done.pid);
    let cwd = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&juggler_core::model::SessionId::new(
            done.session_id.as_str(),
        ))
        .map(|record| record.cwd)
        .ok();
    let in_worktree = cwd.as_deref() == Some(wt.as_path());
    let resume_id = transcript_of(state, &done.session_id)
        .and_then(|r| r.native_id)
        .unwrap_or_else(|| native.clone());
    let probe = claude_turn_in(
        lab,
        &lab.claude_b.clone(),
        &wt,
        None,
        Some(&resume_id),
        &[],
        &format!(
            "Without changing any file: what is this worktree's codename? Then decide that the \
             next helper will be named `{HELPER}`; remember that, but don't write it. One short line."
        ),
    );
    let answer = probe["result"].as_str().unwrap_or_default().to_owned();
    let carried = mentions(&answer, CODENAME_WT);
    let neg_in_wt =
        std::fs::read_to_string(wt.join("calc.py")).is_ok_and(|s| s.contains("def neg("));
    let neg_in_root =
        std::fs::read_to_string(lab.project.join("calc.py")).is_ok_and(|s| s.contains("def neg("));
    lab.steps.push(json!({"step": "7 worktree A→B", "ok": carried && in_worktree && neg_in_wt && !neg_in_root,
                          "cwd_worktree": in_worktree, "transcript_resume": carried, "probe": answer,
                          "neg_in_worktree": neg_in_wt, "neg_in_root": neg_in_root, "launch": argv}));

    // Straight back to A (the direct return, same tool).
    let plan = commands::switch_back_plan_impl(state, &done.session_id);
    let start = plan.as_ref().ok().and_then(|plan| {
        plan.returns
            .iter()
            .find(|r| r.start)
            .map(|r| r.epoch_id.clone())
    });
    let back = commands::switch_back_execute_to_impl(
        state,
        &done.session_id,
        &[],
        false,
        start.as_deref(),
    );
    let step = match &back {
        Ok(back) => {
            let argv = take_launch(state, &back.session_id, back.pid);
            let delta = argv.last().cloned().unwrap_or_default();
            let probe = claude_turn_in(
                lab,
                &lab.claude_a.clone(),
                &wt,
                None,
                Some(&native),
                &[],
                &format!(
                    "{delta}\n\n---\nWithout changing or reading any file: what helper name was \
                     decided while you were away? One short line."
                ),
            );
            let answer = probe["result"].as_str().unwrap_or_default().to_owned();
            json!({"step": "7b switch back B→A in the worktree", "ok": mentions(&answer, HELPER),
                   "switch_back": mentions(&answer, HELPER), "probe": answer, "delta": delta})
        }
        Err(error) => json!({"step": "7b switch back B→A in the worktree", "ok": false,
                             "error": format!("{error:?}")}),
    };
    lab.steps.push(step);
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(&wt)
        .current_dir(&lab.project)
        .output();
}

/// Step 8, Codex → Codex (`AIJ_LAB_CODEX_B`): a task begun in the Mac's
/// Codex home is switched to the second home the way the product does it
/// — the conversation (its rollout) moves, no sign-in file is read,
/// copied or moved — and resumed there, asked what only it knows.
fn codex_to_codex(
    state: &Arc<AppState>,
    lab: &mut Lab,
    profile_x: &str,
    profile_xb: &str,
    home_b: &Path,
) {
    let (thread, first, ok) = codex_turn(
        lab,
        None,
        &format!(
            "This project's third codename is {CODENAME_XX}; remember it. Keep a plan with two items, \
             modulo and power, and mark modulo done. Don't change any file."
        ),
    );
    let Some(thread) = thread.filter(|_| ok) else {
        lab.steps
            .push(json!({"step": "8 Codex task", "ok": false, "result": first}));
        return;
    };
    let source = adopt(state, profile_x, &thread, &lab.project);
    let to_xb = commands::switch_execute_impl(state, &source, profile_xb, "stop-now", None, &[]);
    let step = match &to_xb {
        Ok(done) => {
            let argv = take_launch(state, &done.session_id, done.pid);
            // `codex … resume <id>`: the conversation as it lives in the second home.
            let resumed = argv
                .iter()
                .position(|arg| arg == "resume")
                .and_then(|at| argv.get(at + 1))
                .cloned()
                .unwrap_or_else(|| thread.clone());
            let installed = std::fs::read_dir(home_b.join("sessions")).is_ok();
            let (_, answer, ok) = codex_turn_in(
                lab,
                home_b,
                Some(&resumed),
                &[],
                "Without changing any file: what is the third codename, and which plan items are still \
                 open? Answer in one short line.",
            );
            let carried = mentions(&answer, CODENAME_XX);
            json!({"step": "8 switch Codex→Codex", "ok": ok && carried,
                   "transcript_resume": carried, "todos": mentions(&answer, "power"),
                   "probe": answer, "launch": argv, "resumed": resumed,
                   "installed_in_second_home": installed})
        }
        Err(error) => {
            json!({"step": "8 switch Codex→Codex", "ok": false, "error": format!("{error:?}")})
        }
    };
    lab.steps.push(step);
}
