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

struct Lab {
    project: PathBuf,
    claude_a: PathBuf,
    claude_b: PathBuf,
    codex: PathBuf,
    steps: Vec<Value>,
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
    let mut cmd = Command::new("claude");
    cmd.current_dir(&lab.project)
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
            "Read,Edit,Write,Bash(python3 -m unittest*)",
        ])
        .args(["--max-turns", "12"]);
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
    let mut cmd = Command::new("codex");
    cmd.current_dir(&lab.project)
        .env("CODEX_HOME", &lab.codex)
        .arg("exec");
    if let Some(id) = resume {
        cmd.args(["resume", id]);
    } else {
        cmd.args(["--json", "-s", "workspace-write", "-C"])
            .arg(&lab.project);
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
    let roots: Vec<PathBuf> = vec![
        claude_a.clone(),
        claude_b.clone(),
        codex.clone(),
        home.join(".claude.json"),
        home.join(".agents"),
    ];
    let before = snapshot(&roots);

    let state = Arc::new(AppState::init(data).unwrap());
    let mut lab = Lab {
        project: project.canonicalize().unwrap(),
        claude_a: claude_a.clone(),
        claude_b: claude_b.clone(),
        codex: codex.clone(),
        steps: Vec::new(),
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
             list for this work with three items: add, sub, mul. Mark add done."
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
            json!({"step": "2 switch A→B", "ok": carried && tests_pass(&lab.project),
                   "transcript_resume": carried, "tasks": tasks_carried,
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
        let to_x =
            commands::switch_execute_impl(&state, on_b, &profile_x, "stop-now", Some("strip"), &[]);
        let step3 = match &to_x {
            Ok(done) => {
                let argv = take_launch(&state, &done.session_id, done.pid);
                // What the interactive start was primed with: the replay.
                let brief = argv
                    .iter()
                    .position(|a| a == "--")
                    .and_then(|at| argv.get(at + 1))
                    .cloned()
                    .unwrap_or_default();
                let (thread, answer, ok) = codex_turn(
                    &lab,
                    None,
                    &format!(
                        "{brief}\n\n---\nWithout changing any file: what is this project's \
                         codename, what name was chosen for the multiply function, and which \
                         tasks are still open? Answer in one short line."
                    ),
                );
                let carried = mentions(&answer, CODENAME) && mentions(&answer, MUL_NAME);
                let (_, cont, _) = match &thread {
                    Some(id) => codex_turn(
                        &lab,
                        Some(id),
                        &format!(
                            "Now add the multiply function under the name chosen earlier, with a \
                             test, run `python3 -m unittest`, and mark it done."
                        ),
                    ),
                    None => (None, String::new(), false),
                };
                on_codex = Some((done.session_id.clone(), thread.clone()));
                json!({"step": "3 switch B→Codex", "ok": ok && carried && tests_pass(&lab.project),
                       "transcript_resume": carried, "probe": answer, "continued": cont,
                       "brief_bytes": brief.len(), "thread": thread, "session": done.session_id,
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
        let plan = commands::switch_back_plan_impl(&state, away);
        let back = commands::switch_back_execute_impl(&state, away, &[], false);
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
                        "{delta}\n\n---\nWithout changing any file: what is the codename, what was \
                         done while you were away, and which tasks are still open? One short line."
                    ),
                );
                let answer = probe["result"].as_str().unwrap_or_default().to_owned();
                let carried = mentions(&answer, CODENAME) && mentions(&answer, MUL_NAME);
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
