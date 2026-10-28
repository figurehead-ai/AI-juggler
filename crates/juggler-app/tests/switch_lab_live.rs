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
//! With `AIJ_LAB_SUITE=notes` (`just switch-lab-notes`), the
//! `switch_lab_notes_live` suite checks D-51 instead: a note planted in A's
//! notes folder for the lab project travels to B when its offer is ticked
//! (and its undo takes it out), and into Codex's handover when its "include
//! in the handover" offer is ticked (`switch_lab/notes.rs`).
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
use juggler_app::features::switch_fidelity::facts::{Fact, Kind, Score};
use juggler_app::state::AppState;
use serde_json::{Value, json};

#[path = "switch_lab/fidelity.rs"]
mod fidelity;
#[path = "switch_lab/notes.rs"]
mod notes;

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
/// Facts the person states in A's conversation (middle and late), never
/// written to a file.
const TICKET: &str = "LAB-7341";
const BUCKET: &str = "lab-bucket-ruby";
/// The decision made only in A's conversation.
const ROUNDING: &str = "HALF-EVEN";
/// The decision made only in the Codex-started task's conversation.
const CASE_RULE: &str = "lowercase";
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
        .env("DISABLE_AUTOUPDATER", "1")
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
            "Read,Edit,Write,Bash(python3 -m unittest*),Skill,Task,Agent,TodoWrite,TaskCreate,TaskUpdate,TaskList,TaskGet,mcp__aij-lab-probe__probe_word",
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

/// A new throwaway project at `project`: a git repository with a README,
/// each tool's rules file holding a rule only it states, and one commit.
fn new_project(project: &Path) {
    std::fs::create_dir_all(project).unwrap();
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(project)
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
}

/// The lab runs only in a clean environment (`scripts/qa/lab_env.sh`):
/// started from inside a Claude Code session, the tools it runs would
/// inherit that session's markers and routing (the labs of 2026-10-28).
/// Names only are reported, never a value.
fn assert_clean_env() {
    let inherited: Vec<String> = std::env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter(|name| {
            name == "CLAUDECODE"
                || name.starts_with("CLAUDE_CODE_")
                || name.starts_with("ANTHROPIC_")
                || name.starts_with("OPENAI_")
        })
        .collect();
    assert!(
        inherited.is_empty(),
        "run the lab in a clean environment (scripts/qa/lab_env.sh); inherited: {inherited:?}"
    );
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
    assert_clean_env();
    assert!(
        !data.exists() || std::fs::read_dir(&data).unwrap().next().is_none(),
        "AIJ_LAB_DATA must be a fresh, empty folder"
    );
    assert!(
        !project.exists(),
        "AIJ_LAB_PROJECT must not exist yet: the lab creates it"
    );
    new_project(&project);

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
    // The set-up probe (AJ-E1542): A alone gets a probe MCP server and
    // skill, through Claude Code's own `mcp add`, from a folder beside the
    // project (never in it); removed again at the end.
    let probe_root = project.with_extension("probe");
    let planted = lab.probe.clone().map(|name| {
        std::fs::create_dir_all(probe_root.join("probe")).unwrap();
        std::fs::write(probe_root.join("probe").join("server.py"), PROBE_SERVER).unwrap();
        plant_probe_claude(&probe_root, &claude_a, &name)
    });
    let account_a = import(&state, CLAUDE_CODE_ID, &claude_a, "Lab A");
    let account_b = import(&state, CLAUDE_CODE_ID, &claude_b, "Lab B");
    let account_x = import(&state, CODEX_ID, &codex, "Lab Codex");
    let (profile_a, profile_b, profile_x) = (
        profile_of(&state, &account_a),
        profile_of(&state, &account_b),
        profile_of(&state, &account_x),
    );

    let mut book = fidelity::Book::new(&state);
    let claude = tool_name(&state, CLAUDE_CODE_ID);
    let codex_tool = tool_name(&state, CODEX_ID);
    let label_a = format!("Lab A ({claude})");
    let label_b = format!("Lab B ({claude})");
    let label_x = format!("Lab Codex ({codex_tool})");

    // 1. The task on Claude A: a conversation long enough to mean
    // something (nine short turns, several tool calls, one subagent),
    // with facts planted early, in the middle and late, some said only in
    // a reply and one decision made only in the conversation.
    let native = uuid_v4();
    let mut step1_ok = true;
    let mut said = Vec::new();
    for (at, prompt) in source_turns(lab.probe.is_some()).iter().enumerate() {
        let turn = claude_turn(
            &lab,
            &lab.claude_a.clone(),
            (at == 0).then_some(native.as_str()),
            (at > 0).then_some(native.as_str()),
            prompt,
        );
        step1_ok &= turn["is_error"] == false;
        said.push(turn["result"].clone());
    }
    let step1_ok = step1_ok && tests_pass(&lab.project);
    let source_a = fidelity::source(&state, &account_a, &native, &lab.project, &label_a);
    let facts_a = source_a
        .as_ref()
        .map(|source| planted_on_a(&source.side.history))
        .unwrap_or_default();
    lab.steps.push(
        json!({"step": "1 task on A", "ok": step1_ok && source_a.is_some(),
                          "turns": said.len(), "results": said, "session": native,
                          "facts_planted": facts_a.iter().map(|f| &f.key).collect::<Vec<_>>()}),
    );

    // 2. Switch A → B.
    let source = adopt(&state, &profile_a, &native, &lab.project);
    let preflight_b = commands::switch_preflight_impl(&state, &source, &profile_b);
    let setup_b = set_up_probe(&state, &mut lab, &source, &profile_b);
    let to_b = commands::switch_execute_impl(&state, &source, &profile_b, "stop-now", None, &[]);
    let mut b_side: Option<(PathBuf, String)> = None;
    let step2 = match &to_b {
        Ok(done) => {
            let argv = take_launch(&state, &done.session_id, done.pid);
            let link = transcript_of(&state, &done.session_id);
            let resume_id = link
                .as_ref()
                .and_then(|r| r.native_id.clone())
                .unwrap_or_else(|| native.clone());
            let folder = session_cwd(&state, &done.session_id);
            // Measured before B says anything: what the switch delivered.
            let target = link.as_ref().map(|link| {
                fidelity::native(
                    &state, &account_b, &link.path, &resume_id, &folder, &label_b,
                )
            });
            // Asked in a fork that isn't kept, so the probe's answers never
            // become part of the conversation the next hop carries.
            let probe = claude_ask(
                &lab,
                &lab.claude_b.clone(),
                &lab.project.clone(),
                &resume_id,
                &fidelity::facts::question(&facts_a),
            );
            let answer = probe["result"].as_str().unwrap_or_default().to_owned();
            let probed = fidelity::probe(&facts_a, &answer, &lab.project);
            let score = |key: &str| {
                probed
                    .iter()
                    .any(|p| p.fact.key == key && p.score == Score::Answered)
            };
            let grade = match (source_a.clone(), target) {
                (Some(source), Some(target)) => {
                    book.hop("A → B", "transplant", source, target, probed.clone())
                }
                _ => {
                    book.missing(
                        "A → B",
                        "the source's or the target's transcript wasn't found",
                    );
                    "NOT MEASURED"
                }
            };
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
            if let Some(link) = &link {
                b_side = Some((link.path.clone(), resume_id.clone()));
            }
            let carried = score("CODENAME");
            json!({"step": "2 switch A→B", "ok": carried && tests_pass(&lab.project) && grade != "FAIL",
                   "transcript_resume": carried, "tasks": score("OPEN_TASKS"), "rules": score("DOCSTRING_RULE"),
                   "fidelity": grade, "subagents": subagents, "file_history": file_history,
                   "probe": answer, "continued": cont["result"], "launch": argv,
                   "link": link.map(|r| r.path.display().to_string()), "session": done.session_id,
                   "preflight": preflight_b.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
        }
        Err(error) => json!({"step": "2 switch A→B", "ok": false, "error": format!("{error:?}"),
                            "preflight": preflight_b.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()}),
    };
    let on_b = to_b.as_ref().ok().map(|done| done.session_id.clone());
    lab.steps.push(step2);
    let mul_fact = fidelity::fact(
        "MUL_NAME",
        "the name decided for the multiply function",
        &[MUL_NAME],
        "B, its 1st turn (late)",
        Kind::Decision,
    );

    // 3. Switch B → Codex.
    let mut on_codex = None;
    if let (Some(on_b), Some((b_path, b_id))) = (&on_b, &b_side) {
        let source_b =
            fidelity::source_at(&state, &account_b, b_id, b_path, &lab.project, &label_b);
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
                let folder = session_cwd(&state, &done.session_id);
                let target = handover["path"].as_str().map(|file| {
                    fidelity::handover(&state, &account_x, Path::new(file), &folder, &label_x)
                });
                // B finished sub: only mul is open now.
                let mut facts_x: Vec<Fact> = facts_a
                    .iter()
                    .filter(|fact| fact.kind != Kind::Tasks)
                    .cloned()
                    .collect();
                facts_x.push(fidelity::fact(
                    "OPEN_TASKS",
                    "the task list's open items",
                    &["mul"],
                    "A, turn 1 (its task list); sub done on B",
                    Kind::Tasks,
                ));
                facts_x.push(mul_fact.clone());
                let (thread, answer, ok) = codex_turn(
                    &lab,
                    None,
                    &format!(
                        "{brief}\n\n---\nFirst read the handover file named above. Then: {}",
                        fidelity::facts::question(&facts_x)
                    ),
                );
                let probed = fidelity::probe(&facts_x, &answer, &lab.project);
                let score = |key: &str| {
                    probed
                        .iter()
                        .any(|p| p.fact.key == key && p.score == Score::Answered)
                };
                let grade = match (source_b, target) {
                    (Some(source), Some(target)) => {
                        book.hop("B → Codex", "replay", source, target, probed.clone())
                    }
                    _ => {
                        book.missing(
                            "B → Codex",
                            "the source's transcript or the handover wasn't found",
                        );
                        "NOT MEASURED"
                    }
                };
                let carried = score("CODENAME") && score("MUL_NAME");
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
                json!({"step": "3 switch B→Codex", "ok": ok && carried && tests_pass(&lab.project) && grade != "FAIL",
                       "transcript_resume": carried, "rules": score("DOCSTRING_RULE"), "fidelity": grade,
                       "probe": answer, "continued": cont,
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
    if let Some((away, thread)) = &on_codex {
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
        // Every hop since A, as one source: B's conversation, then Codex's.
        let away_chain = fidelity::chain(
            [
                b_side.as_ref().and_then(|(path, id)| {
                    fidelity::source_at(&state, &account_b, id, path, &lab.project, &label_b)
                }),
                thread.as_ref().and_then(|thread| {
                    fidelity::source(&state, &account_x, thread, &lab.project, &label_x)
                }),
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let back =
            commands::switch_back_execute_to_impl(&state, away, &[], false, start.as_deref());
        let step4 = match &back {
            Ok(done) => {
                let argv = take_launch(&state, &done.session_id, done.pid);
                // `claude --resume <id> "<delta>"`: the delta is the last argument.
                let delta = argv.last().cloned().unwrap_or_default();
                let folder = session_cwd(&state, &done.session_id);
                let target = fidelity::back(&state, &account_a, &native, &folder, &delta, &label_a);
                let mut facts_back: Vec<Fact> = facts_a
                    .iter()
                    .filter(|fact| !matches!(fact.kind, Kind::Tasks | Kind::Rule))
                    .cloned()
                    .collect();
                facts_back.push(mul_fact.clone());
                facts_back.push(fidelity::fact(
                    "RELEASE",
                    "the release name picked while you were away",
                    &[RELEASE],
                    "Codex, its 2nd turn (late)",
                    Kind::ReplyOnly,
                ));
                let probe = claude_turn(
                    &lab,
                    &lab.claude_a.clone(),
                    None,
                    Some(&native),
                    &format!("{delta}\n\n---\n{}", fidelity::facts::question(&facts_back)),
                );
                let answer = probe["result"].as_str().unwrap_or_default().to_owned();
                let probed = fidelity::probe(&facts_back, &answer, &lab.project);
                let score = |key: &str| {
                    probed
                        .iter()
                        .any(|p| p.fact.key == key && p.score == Score::Answered)
                };
                let grade = match away_chain {
                    Some(source) => book.hop(
                        "Codex → A (back)",
                        "switch back",
                        source,
                        target,
                        probed.clone(),
                    ),
                    None => {
                        book.missing("Codex → A (back)", "the away conversations weren't found");
                        "NOT MEASURED"
                    }
                };
                // Only the conversation knows the release name: it was never
                // written to a file.
                let carried = score("CODENAME") && score("RELEASE");
                json!({"step": "4 switch back to A", "ok": carried && tests_pass(&lab.project) && grade != "FAIL",
                       "switch_back": carried, "fidelity": grade, "probe": answer, "delta": delta,
                       "plan": plan.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
            }
            Err(error) => {
                json!({"step": "4 switch back to A", "ok": false, "error": format!("{error:?}"),
                                "plan": plan.as_ref().map(|p| serde_json::to_value(p).unwrap()).ok()})
            }
        };
        lab.steps.push(step4);
    }

    let accounts = Accounts {
        b: account_b.clone(),
        x: account_x.clone(),
        a: account_a.clone(),
    };
    forward_codex_to_claude(
        &state, &mut lab, &mut book, &accounts, &profile_x, &profile_b,
    );
    worktree_leg(
        &state, &mut lab, &mut book, &accounts, &profile_a, &profile_b,
    );
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
    if let (Some(true), Some(name)) = (planted, lab.probe.clone()) {
        let removed = unplant_probe_claude(&probe_root, &claude_a, &name);
        lab.steps
            .push(json!({"step": "5b the probe removed from A", "ok": removed}));
    } else if planted == Some(false) {
        lab.steps
            .push(json!({"step": "0 the probe planted on A", "ok": false}));
    }

    let after = snapshot(&roots);
    let report = json!({
        "tools": {
            "claude": run(Command::new("claude").arg("--version"), Duration::from_secs(20)).1.trim(),
            "codex": run(Command::new("codex").arg("--version"), Duration::from_secs(20)).1.trim(),
        },
        "steps": lab.steps,
        "fidelity": book.hops,
        "snapshot": snapshot_diff(&before, &after),
    });
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    eprintln!("lab report: {}", out.display());
    eprintln!("fidelity report: {}", book.write(&out).display());
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

/// D-51's suite: a project note on A travels to B and into Codex's
/// handover, each only with the (harness's) tick (`switch_lab/notes.rs`).
#[test]
#[ignore = "the live switching lab's notes suite; needs AIJ_LAB_* and real, agreed accounts"]
fn switch_lab_notes_live() {
    notes::journey();
}

/// The three accounts the main journey brought in.
struct Accounts {
    a: String,
    b: String,
    x: String,
}

/// A's conversation, turn by turn: tiny prompts, nine turns, several
/// tool calls and one subagent. Facts are planted early (the codename;
/// a port said only in the reply), in the middle (a ticket; a job name
/// said only in the reply) and late (a decision made only here; a bucket;
/// a timeout said only in the reply). None is ever written to a file.
fn source_turns(with_probe: bool) -> Vec<String> {
    let fourth = if with_probe {
        "Call the probe_word tool of the aij-lab-probe MCP server and tell me its answer. Then \
         read README.md and tell me its title."
    } else {
        "Read README.md and tell me its title."
    };
    vec![
        format!(
            "This project's codename is {CODENAME}; remember it. Create calc.py with add(a, b), \
             and test_calc.py with a unittest for it, then run `python3 -m unittest`. Keep a task \
             list for this work with three items: add, sub, mul. Mark add done. Also use a \
             subagent (the Task tool) to count the lines of README.md, and tell me the count."
        ),
        "Pick a dev-server port between 5000 and 5999 for this project. Say it only in your \
         reply, as PORT=<number>; don't write it in any file."
            .to_owned(),
        "Run the tests again and tell me how many pass.".to_owned(),
        fourth.to_owned(),
        format!(
            "The ticket for this work is {TICKET}; remember it, don't write it down. Also pick a \
             one-word code name for the nightly job and say it only in your reply, as JOB=<word>."
        ),
        "Add a one-line module docstring to calc.py, then run the tests.".to_owned(),
        format!(
            "Decision: rounding in this project is {ROUNDING} (banker's rounding). Don't write \
             it anywhere; just confirm in one line."
        ),
        format!("The staging bucket is {BUCKET}; remember it, don't write it down. Reply ok."),
        "Pick a request timeout in milliseconds between 100 and 999. Say it only in your reply, \
         as TIMEOUT=<number>."
            .to_owned(),
    ]
}

/// The facts A's conversation holds, with the values A chose in its
/// replies (a value it didn't say in the asked form isn't probed).
fn planted_on_a(conversation: &juggler_app::features::switch_fidelity::Conversation) -> Vec<Fact> {
    use juggler_app::features::switch_fidelity::facts::said_as;
    let mut facts = vec![fidelity::fact(
        "CODENAME",
        "this project's codename",
        &[CODENAME],
        "A, turn 1 of 9 (early)",
        Kind::Said,
    )];
    let chosen = |marker: &str| said_as(conversation, marker);
    if let Some(port) = chosen("PORT") {
        facts.push(fidelity::fact(
            "PORT",
            "the dev-server port that was picked",
            &[&port],
            "A, turn 2 of 9 (early)",
            Kind::ReplyOnly,
        ));
    }
    facts.push(fidelity::fact(
        "TICKET",
        "the ticket for this work",
        &[TICKET],
        "A, turn 5 of 9 (middle)",
        Kind::Said,
    ));
    if let Some(job) = chosen("JOB") {
        facts.push(fidelity::fact(
            "JOB",
            "the nightly job's code name",
            &[&job],
            "A, turn 5 of 9 (middle)",
            Kind::ReplyOnly,
        ));
    }
    facts.push(fidelity::fact(
        "ROUNDING",
        "the rounding mode that was decided",
        &["half", "even"],
        "A, turn 7 of 9 (late)",
        Kind::Decision,
    ));
    facts.push(fidelity::fact(
        "BUCKET",
        "the staging bucket",
        &[BUCKET],
        "A, turn 8 of 9 (late)",
        Kind::Said,
    ));
    if let Some(timeout) = chosen("TIMEOUT") {
        facts.push(fidelity::fact(
            "TIMEOUT",
            "the request timeout that was picked, in ms",
            &[&timeout],
            "A, turn 9 of 9 (late)",
            Kind::ReplyOnly,
        ));
    }
    facts.push(fidelity::fact(
        "OPEN_TASKS",
        "the task list's open items",
        &["sub", "mul"],
        "A, turn 1 (its task list)",
        Kind::Tasks,
    ));
    facts.push(fidelity::fact(
        "DOCSTRING_RULE",
        "what this project's rules say about docstrings",
        &["LAB:"],
        "CLAUDE.md",
        Kind::Rule,
    ));
    facts
}

/// A question to a Claude conversation asked in a fork that isn't kept,
/// so its answer never becomes part of the conversation (the next hop
/// carries that conversation, and the answer would repeat every fact).
fn claude_ask(lab: &Lab, config: &Path, cwd: &Path, resume: &str, prompt: &str) -> Value {
    let unkept = [
        "--fork-session".to_owned(),
        "--no-session-persistence".to_owned(),
    ];
    let turn = claude_turn_in(lab, config, cwd, None, Some(resume), &unkept, prompt);
    if turn["is_error"] == false {
        return turn;
    }
    claude_turn_in(lab, config, cwd, None, Some(resume), &unkept[..1], prompt)
}

/// The folder a session runs in, as AI Juggler stored it.
fn session_cwd(state: &Arc<AppState>, session: &str) -> PathBuf {
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&juggler_core::model::SessionId::new(session))
        .map(|record| record.cwd)
        .unwrap_or_default()
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

/// Step 6, the forward Codex → Claude hop: a task started on Codex (three
/// short turns: a codename, a number said only in a reply, a decision
/// made only there), switched forward to Claude account B (a replay
/// through the handover file), and asked what only the Codex
/// conversation and its rules know.
fn forward_codex_to_claude(
    state: &Arc<AppState>,
    lab: &mut Lab,
    book: &mut fidelity::Book,
    accounts: &Accounts,
    profile_x: &str,
    profile_b: &str,
) {
    let (label_x, label_b) = (
        format!("Lab Codex ({})", tool_name(state, CODEX_ID)),
        format!("Lab B ({})", tool_name(state, CLAUDE_CODE_ID)),
    );
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
    let (_, lane, _) = codex_turn(
        lab,
        Some(&thread),
        "Pick a lane number between 10 and 99 for this work. Say it only in your reply, as \
         LANE=<number>; don't write it in any file.",
    );
    let (_, decided, _) = codex_turn(
        lab,
        Some(&thread),
        &format!(
            "Decision: error messages in this project are {CASE_RULE}. Don't write it anywhere; \
             just confirm in one line."
        ),
    );
    let source_x = fidelity::source(state, &accounts.x, &thread, &lab.project, &label_x);
    let mut facts_x = vec![fidelity::fact(
        "CODENAME_2",
        "this project's second codename",
        &[CODENAME_X],
        "Codex, turn 1 of 3 (early)",
        Kind::Said,
    )];
    if let Some(lane) = source_x
        .as_ref()
        .and_then(|source| fidelity_said(&source.side.history, "LANE"))
    {
        facts_x.push(fidelity::fact(
            "LANE",
            "the lane number that was picked",
            &[&lane],
            "Codex, turn 2 of 3 (middle)",
            Kind::ReplyOnly,
        ));
    }
    facts_x.push(fidelity::fact(
        "ERROR_CASE",
        "how error messages are written, as decided",
        &[CASE_RULE],
        "Codex, turn 3 of 3 (late)",
        Kind::Decision,
    ));
    facts_x.push(fidelity::fact(
        "OPEN_PLAN",
        "the plan's open items",
        &["mod"],
        "Codex, turn 1 (its plan)",
        Kind::Tasks,
    ));
    facts_x.push(fidelity::fact(
        "TEST_RULE",
        "what this project's rules say about test method names",
        &["_lab"],
        "AGENTS.md",
        Kind::Rule,
    ));
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
            let folder = session_cwd(state, &done.session_id);
            let handover = handover_of(&pointer);
            let target = handover["path"].as_str().map(|file| {
                fidelity::handover(state, &accounts.b, Path::new(file), &folder, &label_b)
            });
            let probe = claude_turn_in(
                lab,
                &lab.claude_b.clone(),
                &lab.project.clone(),
                None,
                None,
                &extra,
                &format!(
                    "{pointer}\n\n---\nFirst read the handover file named above. Then: {}",
                    fidelity::facts::question(&facts_x)
                ),
            );
            let answer = probe["result"].as_str().unwrap_or_default().to_owned();
            let probed = fidelity::probe(&facts_x, &answer, &lab.project);
            let score = |key: &str| {
                probed
                    .iter()
                    .any(|p| p.fact.key == key && p.score == Score::Answered)
            };
            let grade = match (source_x, target) {
                (Some(source), Some(target)) => book.hop(
                    "Codex → B (forward)",
                    "replay",
                    source,
                    target,
                    probed.clone(),
                ),
                _ => {
                    book.missing(
                        "Codex → B (forward)",
                        "the source's transcript or the handover wasn't found",
                    );
                    "NOT MEASURED"
                }
            };
            let carried = score("CODENAME_2");
            json!({"step": "6 switch Codex→B (forward)", "ok": carried && tests_pass(&lab.project) && grade != "FAIL",
                   "transcript_resume": carried, "todos": score("OPEN_PLAN"),
                   "rules": score("TEST_RULE"), "fidelity": grade, "probe": answer, "launch": argv,
                   "add_dir": !extra.is_empty(), "handover": handover,
                   "probe_session": probe["session_id"],
                   "codex_turns": [first, lane, decided]})
        }
        Err(error) => {
            json!({"step": "6 switch Codex→B (forward)", "ok": false, "error": format!("{error:?}")})
        }
    };
    // The conversation B's probe started (found again by its opening).
    let on_b = step["probe_session"].as_str().map(str::to_owned);
    lab.steps.push(step);
    if let (Ok(done), Some(conversation)) = (&to_b, on_b) {
        switch_back_to_codex(
            state,
            lab,
            book,
            accounts,
            &done.session_id,
            &conversation,
            &thread,
        );
    }
}

/// The value a reply said as `MARKER=value`.
fn fidelity_said(
    conversation: &juggler_app::features::switch_fidelity::Conversation,
    marker: &str,
) -> Option<String> {
    juggler_app::features::switch_fidelity::facts::said_as(conversation, marker)
}

/// Step 6b: B decides something only said, then switches straight back to
/// the Codex task, which must know it.
fn switch_back_to_codex(
    state: &Arc<AppState>,
    lab: &mut Lab,
    book: &mut fidelity::Book,
    accounts: &Accounts,
    away: &str,
    conversation: &str,
    thread: &str,
) {
    let (label_x, label_b) = (
        format!("Lab Codex ({})", tool_name(state, CODEX_ID)),
        format!("Lab B ({})", tool_name(state, CLAUDE_CODE_ID)),
    );
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
    let source_b = fidelity::source(state, &accounts.b, conversation, &lab.project, &label_b);
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
            let folder = session_cwd(state, &back.session_id);
            let target = fidelity::back(state, &accounts.x, thread, &folder, &delta, &label_x);
            let facts_back = vec![
                fidelity::fact(
                    "RELEASE_2",
                    "the release name picked while you were away",
                    &[RELEASE_X],
                    "B, its 2nd turn (late)",
                    Kind::ReplyOnly,
                ),
                fidelity::fact(
                    "CODENAME_2",
                    "this project's second codename",
                    &[CODENAME_X],
                    "Codex, turn 1 of 3 (early)",
                    Kind::Said,
                ),
            ];
            let (_, answer, ok) = codex_turn(
                lab,
                Some(thread),
                &format!("{delta}\n\n---\n{}", fidelity::facts::question(&facts_back)),
            );
            let probed = fidelity::probe(&facts_back, &answer, &lab.project);
            let carried = probed
                .iter()
                .any(|p| p.fact.key == "RELEASE_2" && p.score == Score::Answered);
            let grade = match source_b {
                Some(source) => book.hop("B → Codex (back)", "switch back", source, target, probed),
                None => {
                    book.missing("B → Codex (back)", "B's conversation wasn't found");
                    "NOT MEASURED"
                }
            };
            json!({"step": "6b switch back B→Codex", "ok": ok && carried && grade != "FAIL",
                   "switch_back": carried, "fidelity": grade, "probe": answer, "said": said["result"],
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
fn worktree_leg(
    state: &Arc<AppState>,
    lab: &mut Lab,
    book: &mut fidelity::Book,
    accounts: &Accounts,
    profile_a: &str,
    profile_b: &str,
) {
    let claude = tool_name(state, CLAUDE_CODE_ID);
    let (label_a, label_b) = (format!("Lab A ({claude})"), format!("Lab B ({claude})"));
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
    let source_a = fidelity::source(state, &accounts.a, &native, &wt, &label_a);
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
    let cwd = session_cwd(state, &done.session_id);
    let in_worktree = cwd == wt;
    let link = transcript_of(state, &done.session_id);
    let resume_id = link
        .as_ref()
        .and_then(|r| r.native_id.clone())
        .unwrap_or_else(|| native.clone());
    let target = link
        .as_ref()
        .map(|link| fidelity::native(state, &accounts.b, &link.path, &resume_id, &cwd, &label_b));
    let codename = [fidelity::fact(
        "CODENAME_WT",
        "this worktree's codename",
        &[CODENAME_WT],
        "A, turn 1 of 1 (early)",
        Kind::Said,
    )];
    let probe = claude_turn_in(
        lab,
        &lab.claude_b.clone(),
        &wt,
        None,
        Some(&resume_id),
        &[],
        &format!(
            "{} Then decide that the next helper will be named `{HELPER}`; remember that, but \
             don't write it.",
            fidelity::facts::question(&codename)
        ),
    );
    let answer = probe["result"].as_str().unwrap_or_default().to_owned();
    let probed = fidelity::probe(&codename, &answer, &wt);
    let carried = probed.iter().all(|p| p.score == Score::Answered);
    let grade = match (source_a, target) {
        (Some(source), Some(target)) => {
            book.hop("A → B (worktree)", "transplant", source, target, probed)
        }
        _ => {
            book.missing(
                "A → B (worktree)",
                "the source's or the target's transcript wasn't found",
            );
            "NOT MEASURED"
        }
    };
    let neg_in_wt =
        std::fs::read_to_string(wt.join("calc.py")).is_ok_and(|s| s.contains("def neg("));
    let neg_in_root =
        std::fs::read_to_string(lab.project.join("calc.py")).is_ok_and(|s| s.contains("def neg("));
    lab.steps.push(json!({"step": "7 worktree A→B",
                          "ok": carried && in_worktree && neg_in_wt && !neg_in_root && grade != "FAIL",
                          "cwd_worktree": in_worktree, "transcript_resume": carried, "fidelity": grade,
                          "probe": answer, "neg_in_worktree": neg_in_wt, "neg_in_root": neg_in_root,
                          "launch": argv}));

    // Straight back to A (the direct return, same tool).
    let source_b = link.as_ref().and_then(|link| {
        fidelity::source_at(state, &accounts.b, &resume_id, &link.path, &wt, &label_b)
    });
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
            let folder = session_cwd(state, &back.session_id);
            let target = fidelity::back(state, &accounts.a, &native, &folder, &delta, &label_a);
            let facts_back = [
                fidelity::fact(
                    "HELPER",
                    "the next helper's name, decided while you were away",
                    &[HELPER],
                    "B, its 1st turn (late)",
                    Kind::Decision,
                ),
                codename[0].clone(),
            ];
            let probe = claude_turn_in(
                lab,
                &lab.claude_a.clone(),
                &wt,
                None,
                Some(&native),
                &[],
                &format!("{delta}\n\n---\n{}", fidelity::facts::question(&facts_back)),
            );
            let answer = probe["result"].as_str().unwrap_or_default().to_owned();
            let probed = fidelity::probe(&facts_back, &answer, &wt);
            let carried = probed
                .iter()
                .any(|p| p.fact.key == "HELPER" && p.score == Score::Answered);
            let grade = match source_b {
                Some(source) => book.hop(
                    "B → A (worktree, back)",
                    "switch back",
                    source,
                    target,
                    probed,
                ),
                None => {
                    book.missing("B → A (worktree, back)", "B's conversation wasn't found");
                    "NOT MEASURED"
                }
            };
            json!({"step": "7b switch back B→A in the worktree", "ok": carried && grade != "FAIL",
                   "switch_back": carried, "fidelity": grade, "probe": answer, "delta": delta})
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

// ---------------------------------------------------------------------------
// Labs 12+ (0.1.8, AJ-E2108): a switch whose source is a session another
// app runs (its key `conversation:<account>:<id>` stands where a session id
// goes), and the old path (a session AI Juggler runs) again, on ONE Claude
// profile and the Mac's Codex home. `AIJ_LAB_SUITE=outside`, run by
// `just switch-lab-outside`:
//
// - J1: a Claude conversation started outside AI Juggler (`claude -p` in a
//   process the lab owns) → Codex → back to Claude.
// - J2: a Codex conversation started outside (`codex exec`), in a git
//   worktree → Claude → back to Codex.
// - J3: the old path, in a git worktree: the Claude conversation is picked
//   up with "Continue in AI Juggler" (AI Juggler's own session) → Codex →
//   back to Claude.
//
// Every probe asks what only the conversation can know, and every area is
// a check that can fail. The source conversation is hashed before and
// after: AI Juggler only reads it (a switch back resumes it, so after that
// its earlier bytes must be untouched: appended to, never copied over).

const J1_CODENAME: &str = "MAGPIE";
/// Said only in a reply on Codex (J1).
const J1_RELEASE: &str = "Amber Lantern";
const J2_CODENAME: &str = "TERN";
/// Said only in a reply on Claude (J2).
const J2_RELEASE: &str = "Quiet Harbor";
const J3_CODENAME: &str = "WREN";
/// Said only in a reply on Codex (J3).
const J3_MILESTONE: &str = "Velvet Comet";
/// A secret-shaped string planted in each first turn (never a real key).
const FAKE_KEY: &str = "sk-lab-NOT-A-REAL-KEY-4f9c2a7e1b3d5f60";
/// How a switch back's delta turn starts.
const DELTA_MARK: &str = "While you were away";
/// The labels the lab gives the two accounts.
const CLAUDE_LABEL: &str = "Lab Luka";
const CODEX_LABEL: &str = "Lab Codex";

/// A probe MCP server (standard library only): its tool answers
/// `KESTREL`, and only with its environment carried.
const PROBE_SERVER: &str = r#"import json, os, sys
WORD = "KESTREL" if os.environ.get("AIJ_PROBE_ENV") == "on" else "NO-ENV"
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except ValueError:
        continue
    mid = msg.get("id")
    if mid is None:
        continue
    method = msg.get("method")
    if method == "initialize":
        version = (msg.get("params") or {}).get("protocolVersion", "2025-06-18")
        result = {"protocolVersion": version, "capabilities": {"tools": {}},
                  "serverInfo": {"name": "aij-lab-probe", "version": "1.0"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "probe_word", "description": "The lab's probe word.",
                             "inputSchema": {"type": "object", "properties": {}}}]}
    elif method == "tools/call":
        result = {"content": [{"type": "text", "text": WORD}]}
    elif method == "ping":
        result = {}
    else:
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": mid,
                                     "error": {"code": -32601, "message": "unknown"}}) + "\n")
        sys.stdout.flush()
        continue
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": mid, "result": result}) + "\n")
    sys.stdout.flush()
"#;

/// The probe skill (its word is `PEREGRINE`).
fn probe_skill(name: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: The AI Juggler switch lab's probe skill. Use it when asked \
         for the lab's skill word.\n---\nThe lab's skill word is {SKILL_WORD}.\n"
    )
}

/// What the suite shares across journeys.
struct Suite {
    state: Arc<AppState>,
    root: PathBuf,
    claude: PathBuf,
    codex: PathBuf,
    account_c: String,
    profile_c: String,
    account_x: String,
    profile_x: String,
    probe: Option<String>,
    steps: Vec<Value>,
}

impl Suite {
    /// A journey's lab in `project` (the shared helpers take a [`Lab`]).
    fn lab(&self, project: &Path) -> Lab {
        Lab {
            project: project
                .canonicalize()
                .unwrap_or_else(|_| project.to_path_buf()),
            claude_a: self.claude.clone(),
            claude_b: self.claude.clone(),
            codex: self.codex.clone(),
            codex_b: None,
            steps: Vec::new(),
            probe: self.probe.clone(),
            records: Vec::new(),
        }
    }

    /// Records a journey's steps (and undoes its set-ups).
    fn finish(&mut self, journey: &str, lab: Lab) {
        let undone: Vec<Value> = lab
            .records
            .iter()
            .rev()
            .map(|record| {
                match juggler_app::features::switch_setup::undo_impl(&self.state, record) {
                    Ok(message) => json!({"record": record, "ok": true, "message": message}),
                    Err(error) => {
                        json!({"record": record, "ok": false, "error": format!("{error:?}")})
                    }
                }
            })
            .collect();
        for mut step in lab.steps {
            step["journey"] = json!(journey);
            self.steps.push(step);
        }
        // Only a journey that set something up has something to undo (J3
        // has no probe to offer).
        if lab.probe.is_some() && !undone.is_empty() {
            let ok = undone.iter().all(|u| u["ok"] == true);
            self.steps.push(
                json!({"journey": journey, "step": format!("{journey} undo every set-up"),
                                   "ok": ok, "undone": undone}),
            );
        }
    }
}

/// The source conversation's file as it was before a switch.
struct Original {
    path: PathBuf,
    sha: String,
    len: usize,
}

impl Original {
    /// `account`'s conversation `id`, from the index (refreshed first).
    fn of(state: &Arc<AppState>, account: &str, id: &str) -> Option<Self> {
        // The app's background pass, which the lab doesn't run: a
        // conversation begun after the first read is read now.
        let _ = juggler_app::features::conversations::refresh_impl(state);
        let path = state.conversations.path_of(account, id)?;
        let bytes = std::fs::read(&path).ok()?;
        Some(Self {
            sha: juggler_adapter::handover::sha256_hex(&bytes),
            len: bytes.len(),
            path,
        })
    }

    /// Byte for byte as it was.
    fn unchanged(&self) -> bool {
        std::fs::read(&self.path)
            .is_ok_and(|bytes| juggler_adapter::handover::sha256_hex(&bytes) == self.sha)
    }

    /// Its earlier bytes as they were (it may have grown since).
    fn prefix_kept(&self) -> bool {
        std::fs::read(&self.path).is_ok_and(|bytes| {
            bytes.len() >= self.len
                && juggler_adapter::handover::sha256_hex(&bytes[..self.len]) == self.sha
        })
    }

    /// How many user turns in it carry a switch back's delta.
    fn delta_turns(&self) -> usize {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        text.lines()
            .filter(|line| line.contains(DELTA_MARK))
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|event| {
                // Claude: one `user` line per turn. Codex: one user
                // `message` item per turn (its events repeat it; lab 13).
                event["type"] == "user"
                    || (event["type"] == "response_item"
                        && event["payload"]["type"] == "message"
                        && event["payload"]["role"] == "user")
            })
            .count()
    }
}

/// How the conversations list shows `account`'s conversation `id`.
fn listed(state: &Arc<AppState>, account: &str, id: &str) -> Value {
    let _ = juggler_app::features::conversations::refresh_impl(state);
    let all = juggler_app::features::conversations::commands::list_impl(state).unwrap_or_default();
    match all.iter().find(|c| c.id == id && c.account_id == account) {
        Some(found) => json!({"listed": true, "session_id": found.session_id,
                              "account": found.account_label, "tool": found.tool}),
        None => json!({"listed": false}),
    }
}

/// A tool's display name, as every surface shows it.
fn tool_name(state: &Arc<AppState>, tool: &str) -> String {
    state
        .registry
        .get(&juggler_core::model::HarnessId::new(tool))
        .map_or_else(|| tool.to_owned(), |adapter| adapter.spec().display_name)
}

/// History › Replay's lineage for `session` (the view model the dialog
/// renders: "Arrived from <account> (<tool>)").
fn replay_lineage(state: &Arc<AppState>, session: &str) -> Value {
    match juggler_app::history::instance_replay_impl(state, session) {
        Ok(info) => Value::Array(
            info.lineage
                .iter()
                .map(|step| {
                    json!({"direction": step.direction, "account": step.other_account,
                           "tool": tool_name(state, &step.other_provider), "state": step.state})
                })
                .collect(),
        ),
        Err(error) => json!({"error": format!("{error:?}")}),
    }
}

/// The project's switch graph as the timeline shows it.
fn graph_view(state: &Arc<AppState>, session: &str) -> Value {
    let Some(project) = session_row(state, session).and_then(|row| row.project) else {
        return json!({"error": "no project"});
    };
    match commands::switch_graph_overview_impl(state, project.as_str()) {
        Ok(works) => Value::Array(
            works
                .iter()
                .map(|work| {
                    json!({
                        "epochs": work.epochs.iter().map(|e| json!({
                            "account": e.account_label, "tool": tool_name(state, &e.provider),
                            "resumable": e.resumable, "session": e.session_id})).collect::<Vec<_>>(),
                        "edges": work.edges.iter().map(|e| json!({
                            "label": e.label, "switch_back": e.switch_back,
                            "failed": e.failed_reason})).collect::<Vec<_>>(),
                    })
                })
                .collect(),
        ),
        Err(error) => json!({"error": format!("{error:?}")}),
    }
}

fn session_row(state: &Arc<AppState>, id: &str) -> Option<juggler_core::model::InstanceRecord> {
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&juggler_core::model::SessionId::new(id))
        .ok()
}

/// The switch graph's record of `edge`: its mechanism, ends and manifest.
fn edge_facts(state: &Arc<AppState>, edge: &str) -> Value {
    let storage = state.storage.lock().unwrap();
    let graph = storage.switch_graph();
    let id = juggler_core::model::SwitchEdgeId::new(edge);
    let Ok(edge) = graph.edge(&id) else {
        return json!({"error": "no edge"});
    };
    let epoch = |id: &juggler_core::model::EpochId| {
        graph.epoch(id).ok().flatten().map(|e| {
            json!({"session": e.session_ref.map(|s| s.to_string()), "native": e.native_id,
                   "account": e.account.to_string(), "tool": e.provider.to_string(),
                   "resumed_from": e.resumed_from_epoch.map(|r| r.to_string())})
        })
    };
    let manifest: Vec<Value> = graph
        .manifest_for(&id)
        .unwrap_or_default()
        .iter()
        .map(|row| {
            json!({"path": row.rel_path, "kind": row.kind, "disposition": row.disposition,
                   "sha256": row.sha256, "bytes": row.bytes})
        })
        .collect();
    json!({"mechanism": format!("{:?}", edge.mechanism), "state": format!("{:?}", edge.state),
           "from": epoch(&edge.from_epoch), "to": edge.to_epoch.as_ref().and_then(epoch),
           "from_tool": edge.from_provider.to_string(), "to_tool": edge.to_provider.to_string(),
           "from_account": edge.from_account.to_string(), "to_account": edge.to_account.to_string(),
           "from_epoch": edge.from_epoch.to_string(), "manifest": manifest})
}

/// Reads the command line of the session a switch started, what its
/// screen showed (only whether it asks to trust the folder), then stops it.
fn take_launch_seen(state: &Arc<AppState>, session: &str, pid: u32) -> (Vec<String>, Value) {
    std::thread::sleep(Duration::from_millis(800));
    let argv = argv_of(pid);
    let screen = state
        .supervisor
        .output(&juggler_core::model::SessionId::new(session))
        .unwrap_or_default();
    let text = String::from_utf8_lossy(&screen).to_lowercase();
    let _ = juggler_app::terminal::terminal_kill_impl(state, session);
    std::thread::sleep(Duration::from_millis(500));
    mark_stopped(state, session);
    (
        argv,
        json!({"screen_bytes": screen.len(), "asks_trust": text.contains("trust")}),
    )
}

/// Stores a stopped session as stopped, as the app's lifecycle forwarder
/// does (the lab doesn't run it: it would also notify and restart).
fn mark_stopped(state: &Arc<AppState>, session: &str) {
    let storage = state.storage.lock().unwrap();
    if let Ok(mut row) = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session))
        && matches!(row.status.as_str(), "starting" | "running")
    {
        row.status = juggler_core::model::SessionStatus::Stopped;
        let _ = storage.sessions().update(&row);
    }
}

/// What a handover file holds, checked against the conversation and the
/// project (its text stays out of the report).
fn handover_checks(prompt: &str, codename: &str, project: &Path) -> Value {
    let Some(path) = prompt
        .lines()
        .map(str::trim)
        .find(|line| line.ends_with("HANDOVER.md"))
    else {
        return json!({"found": false});
    };
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let changed = git_changes(project);
    let lists_changes =
        !changed.is_empty() && changed.iter().all(|file| text.contains(file.as_str()));
    json!({"found": true, "bytes": text.len(), "codename": text.contains(codename),
           "rules_files": text.contains("CLAUDE.md") && text.contains("AGENTS.md"),
           "claude_rule": text.contains("LAB:"), "agents_rule": text.contains("_lab"),
           "task_list": text.contains("Task list"), "changes_listed": lists_changes,
           "git_changes": changed, "secret_absent": !text.contains(FAKE_KEY),
           "state_json": Path::new(path).with_file_name("state.json").is_file()})
}

/// The project's uncommitted files (`git status`), as paths.
fn git_changes(project: &Path) -> Vec<String> {
    Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(project)
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.get(3..).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `name`'s docstring in `file` starts with `LAB:` (CLAUDE.md's
/// rule).
fn docstring_has_lab(file: &Path, name: &str) -> bool {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let Some(at) = text.find(&format!("def {name}(")) else {
        return false;
    };
    text[at..]
        .lines()
        .skip(1)
        .map(str::trim)
        .find(|line| !line.is_empty())
        .is_some_and(|line| {
            line.trim_start_matches(['"', '\'', 'r'])
                .starts_with("LAB:")
        })
}

/// The test method names in `file`.
fn test_methods(file: &Path) -> Vec<String> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().strip_prefix("def test"))
        .filter_map(|rest| rest.split('(').next())
        .map(|rest| format!("test{rest}"))
        .collect()
}

/// Whether any file in `project` (outside `.git`) holds `word`.
fn written_anywhere(project: &Path, word: &str) -> bool {
    let mut stack = vec![project.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if std::fs::read_to_string(&path).is_ok_and(|text| text.contains(word)) {
                return true;
            }
        }
    }
    false
}

/// How many files named `<native>.jsonl` the Claude profile holds.
fn claude_files_named(config: &Path, native: &str) -> usize {
    std::fs::read_dir(config.join("projects")).map_or(0, |dirs| {
        dirs.flatten()
            .filter(|dir| dir.path().join(format!("{native}.jsonl")).is_file())
            .count()
    })
}

/// How many Codex rollouts carry `thread` in their name.
fn codex_files_named(home: &Path, thread: &str) -> usize {
    let mut count = 0;
    let mut stack = vec![home.join("sessions")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().contains(thread))
            {
                count += 1;
            }
        }
    }
    count
}

fn add_dir_of(argv: &[String]) -> Vec<String> {
    argv.iter()
        .position(|arg| arg == "--add-dir")
        .and_then(|at| argv.get(at + 1))
        .map(|dir| vec!["--add-dir".to_owned(), dir.clone()])
        .unwrap_or_default()
}

/// The prompt a primed start was given (Codex: after `--`; Claude: the
/// argument naming the handover).
fn primed_prompt(argv: &[String]) -> String {
    argv.iter()
        .position(|a| a == "--")
        .and_then(|at| argv.get(at + 1))
        .or_else(|| argv.iter().find(|arg| arg.contains("HANDOVER.md")))
        .cloned()
        .unwrap_or_default()
}

/// A forward switch and every check that doesn't need the target to
/// answer.
struct Forward {
    done: commands::SwitchExecutedInfo,
    argv: Vec<String>,
    facts: Value,
}

/// The preflight (what the screen lists), the set-up step, the switch, the
/// launch, the graph's edge and manifest, the handover, the new session's
/// row, the source left as it was, and (for a source another app runs)
/// undo refusing in words.
fn switch_forward(
    s: &Suite,
    lab: &mut Lab,
    source: &str,
    target_profile: &str,
    original: &Original,
    codename: &str,
) -> Result<Forward, Value> {
    let state = &s.state;
    let offered = commands::switch_targets_impl(state, source)
        .map(|all| all.iter().any(|t| t.recipe_id == target_profile))
        .unwrap_or(false);
    let preflight = commands::switch_preflight_impl(state, source, target_profile)
        .map(|p| serde_json::to_value(p).unwrap())
        .unwrap_or_else(|error| json!({"error": format!("{error:?}")}));
    let setup = set_up_probe(state, lab, source, target_profile);
    let switched =
        commands::switch_execute_impl(state, source, target_profile, "finish-turn", None, &[]);
    let done = match switched {
        Ok(done) => done,
        Err(error) => {
            return Err(json!({"offered": offered, "preflight": preflight,
                              "error": format!("{error:?}")}));
        }
    };
    let (argv, screen) = take_launch_seen(state, &done.session_id, done.pid);
    let row = session_row(state, &done.session_id);
    let edge = edge_facts(state, &done.edge_id);
    let read_only = original.unchanged();
    // Undo after a switch from a session another app runs: nothing of
    // ours to resume, said in words, and the original untouched.
    let undo = juggler_app::features::conversations::switch_source::is_source_key(source).then(|| {
        let refused = commands::switch_undo_impl(state, &done.edge_id);
        json!({"refused": refused.is_err(),
               "says_another_app": refused.as_ref().err().is_some_and(|e| e.to_string().contains("another app runs")),
               "original_unchanged": original.unchanged()})
    });
    let prompt = primed_prompt(&argv);
    let facts = json!({
        "offered": offered,
        "preflight": preflight,
        "setup": setup,
        "launch": argv,
        "screen": screen,
        "edge": edge,
        "handover": handover_checks(&prompt, codename, &lab.project),
        "row": row.as_ref().map(|r| json!({"profile": r.profile.to_string(), "cwd": r.cwd.display().to_string(),
                                            "same_folder": r.cwd == lab.project, "status": r.status.as_str()})),
        "source_read_only": read_only,
        "undo": undo,
    });
    Ok(Forward { done, argv, facts })
}

/// What a switch back to where the work started is asked, and of whom.
struct BackProbe<'a> {
    native: &'a str,
    label: &'a str,
    said_only: &'a str,
    question: &'a str,
    /// Asks the original's own conversation (resumed by id) `prompt`;
    /// returns the answer.
    resume: &'a dyn Fn(&Lab, &str) -> String,
}

/// The switch back to where the work started, its delta and its probe.
fn switch_back_to_start(
    s: &Suite,
    lab: &Lab,
    away: &str,
    original: &Original,
    ask: &BackProbe<'_>,
) -> Value {
    let state = &s.state;
    let plan = commands::switch_back_plan_impl(state, away);
    let start = plan.as_ref().ok().and_then(|plan| {
        plan.returns
            .iter()
            .find(|r| r.start)
            .map(|r| r.epoch_id.clone())
    });
    let plan_facts = plan.as_ref().map(|p| {
        json!({"original_account": p.original_account, "native": p.original_native_id,
               "alive": p.original_alive, "returns": p.returns.len(),
               "base_turns": p.base_turns, "original_only_turns": p.original_only_turns,
               "away_only_turns": p.away_only_turns, "divergence": p.divergence,
               "delta_head": p.delta_preview.lines().next().unwrap_or_default(),
               "preview_has_said_only": p.delta_preview.contains(ask.said_only)})
    });
    let offered_back = s.probe.as_ref().map(|probe| {
        let target = profile_for_label(state, ask.label);
        juggler_app::features::switch_setup::check_impl(state, away, &target)
            .map(|check| check.offers.iter().any(|o| &o.name == probe))
            .unwrap_or(true)
    });
    let unchanged_before = original.unchanged();
    let back = commands::switch_back_execute_to_impl(state, away, &[], false, start.as_deref());
    let back = match back {
        Ok(back) => back,
        Err(error) => {
            return json!({"ok": false, "error": format!("{error:?}"),
                          "plan": plan_facts.ok()});
        }
    };
    let (argv, screen) = take_launch_seen(state, &back.session_id, back.pid);
    let delta = argv.last().cloned().unwrap_or_default();
    let resumes_original = argv.iter().any(|a| a == ask.native)
        && argv.iter().any(|a| a == "--resume" || a == "resume");
    // The launch itself may have handed the delta over before it stopped:
    // then the probe asks only its question (the delta arrives once).
    let delivered_by_launch = !original.unchanged();
    let prompt = if delivered_by_launch {
        ask.question.to_owned()
    } else {
        format!("{delta}\n\n---\n{}", ask.question)
    };
    let answer = (ask.resume)(lab, &prompt);
    let row = session_row(state, &back.session_id);
    json!({
        "ok": true,
        "plan": plan_facts.ok(),
        "already_has_probe_not_offered": offered_back.map(|offered| !offered),
        "original_unchanged_until_back": unchanged_before,
        "launch": argv, "screen": screen,
        "resumes_original_id": resumes_original,
        "delta_has_mark": delta.contains(DELTA_MARK),
        "delta_has_said_only": delta.contains(ask.said_only),
        "delta_bytes": delta.len(),
        "delivered_by_launch": delivered_by_launch,
        "probe": answer,
        "said_only_known": mentions(&answer, ask.said_only),
        "delta_turns_in_original": original.delta_turns(),
        "original_prefix_kept": original.prefix_kept(),
        "row": row.as_ref().map(|r| json!({"profile": r.profile.to_string(), "cwd": r.cwd.display().to_string(),
                                            "same_folder": r.cwd == lab.project})),
        "edge": back_edge_facts(state, &back.session_id),
        "session": back.session_id,
    })
}

/// The launch settings of the account labelled `label`.
fn profile_for_label(state: &Arc<AppState>, label: &str) -> String {
    let account = {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .list()
            .unwrap()
            .into_iter()
            .find(|a| a.label == label)
            .map(|a| a.id.to_string())
            .unwrap_or_default()
    };
    profile_of(state, &account)
}

/// The edge a switch back recorded into `session`'s epoch.
fn back_edge_facts(state: &Arc<AppState>, session: &str) -> Value {
    let edge = {
        let storage = state.storage.lock().unwrap();
        storage
            .switch_graph()
            .edges_for_session(session)
            .unwrap_or_default()
            .into_iter()
            .find(|(_, outgoing)| !outgoing)
            .map(|(edge, _)| edge.id.to_string())
    };
    edge.map_or(Value::Null, |edge| edge_facts(state, &edge))
}

/// Sets the probe server and skill up on a Claude profile, with the
/// tool's own command (no file of it is read).
fn plant_probe_claude(root: &Path, config: &Path, name: &str) -> bool {
    let added = Command::new("claude")
        .env("CLAUDE_CONFIG_DIR", config)
        .env("DISABLE_AUTOUPDATER", "1")
        .current_dir(root)
        .args([
            "mcp",
            "add",
            "--scope",
            "user",
            name,
            "-e",
            "AIJ_PROBE_ENV=on",
            "--",
            "python3",
        ])
        .arg(root.join("probe").join("server.py"))
        .output()
        .is_ok_and(|out| out.status.success());
    let skill = config.join("skills").join(name);
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), probe_skill(name)).unwrap();
    added
}

fn unplant_probe_claude(root: &Path, config: &Path, name: &str) -> bool {
    let removed = Command::new("claude")
        .env("CLAUDE_CONFIG_DIR", config)
        .env("DISABLE_AUTOUPDATER", "1")
        .current_dir(root)
        .args(["mcp", "remove", "--scope", "user", name])
        .output()
        .is_ok_and(|out| out.status.success());
    let aside = root.join("removed").join("claude-skill");
    std::fs::create_dir_all(aside.parent().unwrap()).unwrap();
    removed && std::fs::rename(config.join("skills").join(name), aside).is_ok()
}

/// The same on the Codex home (its own skills folder).
fn plant_probe_codex(root: &Path, home: &Path, name: &str) -> bool {
    let added = Command::new("codex")
        .env("CODEX_HOME", home)
        .current_dir(root)
        .args([
            "mcp",
            "add",
            name,
            "--env",
            "AIJ_PROBE_ENV=on",
            "--",
            "python3",
        ])
        .arg(root.join("probe").join("server.py"))
        .output()
        .is_ok_and(|out| out.status.success());
    let skill = home.join("skills").join(name);
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), probe_skill(name)).unwrap();
    added
}

fn unplant_probe_codex(root: &Path, home: &Path, name: &str) -> bool {
    let removed = Command::new("codex")
        .env("CODEX_HOME", home)
        .current_dir(root)
        .args(["mcp", "remove", name])
        .output()
        .is_ok_and(|out| out.status.success());
    let aside = root.join("removed").join("codex-skill");
    std::fs::create_dir_all(aside.parent().unwrap()).unwrap();
    removed && std::fs::rename(home.join("skills").join(name), aside).is_ok()
}

/// A fresh session's answer to the set-up check, on Claude.
fn claude_uses_probe(lab: &Lab) -> Value {
    let turn = claude_turn(lab, &lab.claude_a.clone(), None, None, PROBE_ASK);
    probe_answer(turn["result"].as_str().unwrap_or_default())
}

/// The same on Codex (its automatic review approves the tool call).
fn codex_uses_probe(lab: &Lab) -> Value {
    probe_answer(&codex_turn_with(lab, None, &["--approve-for-me"], PROBE_ASK).1)
}

/// A worktree of `project` at `.wt/<name>` on branch `lab-<name>`.
fn worktree_of(project: &Path, name: &str) -> Option<PathBuf> {
    let wt = project.join(".wt").join(name);
    Command::new("git")
        .args(["worktree", "add", "-q"])
        .arg(&wt)
        .args(["-b", &format!("lab-{name}")])
        .current_dir(project)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|_| wt.canonicalize().ok())
}

fn remove_worktree(project: &Path, wt: &Path) {
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(wt)
        .current_dir(project)
        .output();
}

/// Each switch back's own checks, recorded on its step.
fn finish_back(mut back: Value, step: &str, areas: Value) -> Value {
    back["ok"] = json!(back["ok"] == true && all_true(&areas));
    back["areas"] = areas;
    back["step"] = json!(step);
    back
}

/// Whether a switch back delivered: the decision only said away is known,
/// with the codename, the original id resumed, one delta turn, and the
/// original's earlier bytes untouched.
fn back_ok(back: &Value, codename: &str) -> bool {
    let probe = back["probe"].as_str().unwrap_or_default();
    back["said_only_known"] == true
        && mentions(probe, codename)
        && back["resumes_original_id"] == true
        && back["delta_turns_in_original"] == 1
        && back["original_prefix_kept"] == true
}

/// J1: outside Claude → Codex → back to Claude.
fn j1_outside_claude(s: &mut Suite) {
    let project = s.root.join("j1");
    new_project(&project);
    let mut lab = s.lab(&project);
    let native = uuid_v4();
    let first = claude_turn(
        &lab,
        &s.claude.clone(),
        Some(&native),
        None,
        &format!(
            "This project's codename is {J1_CODENAME}; remember it. The lab's dummy key is \
             {FAKE_KEY}: it is fake, never write it to a file. Create calc.py with add(a, b), and \
             test_calc.py with a unittest for it, then run `python3 -m unittest`. Keep a task list \
             for this work with three items: add, sub, mul. Mark add done. Also use a subagent (the \
             Task tool) to count the lines of README.md, and tell me the count."
        ),
    );
    let original = Original::of(&s.state, &s.account_c, &native);
    let before = listed(&s.state, &s.account_c, &native);
    let step1_ok = first["is_error"] == false
        && tests_pass(&lab.project)
        && original.is_some()
        && before["listed"] == true
        && before["session_id"].is_null();
    lab.steps
        .push(json!({"step": "J1.1 outside Claude task", "ok": step1_ok,
        "result": first["result"], "session": native, "listed": before,
        "subagents_on_source": has_session_dir(&s.claude, &native, "subagents"),
        "file_history_on_source": s.claude.join("file-history").join(&native).is_dir()}));
    let Some(original) = original else {
        s.finish("J1", lab);
        return;
    };
    // Control: Codex alone doesn't know CLAUDE.md's rule.
    let (_, control, _) = codex_turn(
        &lab,
        None,
        "Answer only from the instructions you were given, without running anything or opening any \
         file: what do this project's rules say about docstrings? If you were given none, say NONE.",
    );
    let key =
        juggler_app::features::conversations::switch_source::source_key(&s.account_c, &native);
    let profile_x = s.profile_x.clone();
    let forward = switch_forward(s, &mut lab, &key, &profile_x, &original, J1_CODENAME);
    let forward = match forward {
        Ok(forward) => forward,
        Err(facts) => {
            lab.steps
                .push(json!({"step": "J1.2 switch Claude→Codex", "ok": false, "facts": facts}));
            s.finish("J1", lab);
            return;
        }
    };
    let brief = primed_prompt(&forward.argv);
    let (thread, answer, ok) = codex_turn(
        &lab,
        None,
        &format!(
            "{brief}\n\n---\nWithout changing any file: what is this project's codename, which tasks \
             are still open, what do this project's rules say about docstrings, and what is the \
             dummy key mentioned earlier, exactly (say REDACTED if you can't see it)? One short line."
        ),
    );
    let (_, cont, cont_ok) = match &thread {
        Some(id) => codex_turn(
            &lab,
            Some(id),
            &format!(
                "Now add sub(a, b) to calc.py with a test, following this project's rules, run \
                 `python3 -m unittest`, and mark sub done in your plan. Then tell me, only in your \
                 reply and in no file, that this version's release name is \"{J1_RELEASE}\"."
            ),
        ),
        None => (None, String::new(), false),
    };
    let uses = s.probe.as_ref().map(|_| codex_uses_probe(&lab));
    let sub_lab = docstring_has_lab(&lab.project.join("calc.py"), "sub");
    let areas = json!({
        "transcript-resume": mentions(&answer, J1_CODENAME),
        "todos": mentions(&answer, "sub") && mentions(&answer, "mul"),
        "rules-carry": mentions(&answer, "LAB:") && sub_lab && !mentions(&control, "LAB:"),
        "secret-stripped": !answer.contains(FAKE_KEY) && forward.facts["handover"]["secret_absent"] == true,
        "mcp-references": s.probe.as_ref().map(|_| setup_ok(&forward.facts["setup"], uses.as_ref())),
        "cwd-worktree-edges": forward.facts["row"]["same_folder"] == true,
        "source-read-only": original.unchanged(),
        "tests": cont_ok && tests_pass(&lab.project),
        "said-only-not-written": !written_anywhere(&lab.project, J1_RELEASE),
    });
    let ok = ok && all_true(&areas);
    lab.steps.push(json!({"step": "J1.2 switch Claude→Codex", "ok": ok, "areas": areas,
        "control": control, "probe": answer, "continued": cont, "thread": thread,
        "sub_docstring_lab": sub_lab, "test_methods": test_methods(&lab.project.join("test_calc.py")),
        "uses": uses, "listed_after": listed(&s.state, &s.account_c, &native),
        "lineage": replay_lineage(&s.state, &forward.done.session_id),
        "graph": graph_view(&s.state, &forward.done.session_id), "facts": forward.facts}));

    // Back to Claude, where it started.
    let claude = s.claude.clone();
    let resume = |lab: &Lab, prompt: &str| {
        claude_turn(lab, &claude, None, Some(&native), prompt)["result"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    let back = switch_back_to_start(
        s,
        &lab,
        &forward.done.session_id,
        &original,
        &BackProbe {
            native: &native,
            label: CLAUDE_LABEL,
            said_only: J1_RELEASE,
            question: "Without changing or reading any file: what is the codename, what release \
                       name was picked while you were away, and which items of your task list are \
                       still open? One short line.",
            resume: &resume,
        },
    );
    let areas = json!({
        "switch-back": back_ok(&back, J1_CODENAME),
        "todos": mentions(back["probe"].as_str().unwrap_or_default(), "mul"),
        "one-file": claude_files_named(&s.claude, &native) == 1,
        "cwd-worktree-edges": back["row"]["same_folder"] == true,
        "tests": tests_pass(&lab.project),
    });
    let mut back = finish_back(back, "J1.3 switch back Codex→Claude", areas);
    back["listed_after"] = listed(&s.state, &s.account_c, &native);
    lab.steps.push(back);
    s.finish("J1", lab);
}

/// J2: outside Codex (in a worktree) → Claude → back to Codex.
fn j2_outside_codex(s: &mut Suite) {
    let project = s.root.join("j2");
    new_project(&project);
    let Some(wt) = worktree_of(&project, "tern") else {
        s.steps
            .push(json!({"journey": "J2", "step": "J2.0 worktree", "ok": false}));
        return;
    };
    let mut lab = s.lab(&wt);
    let (thread, first, ok) = codex_turn(
        &lab,
        None,
        &format!(
            "This project's codename is {J2_CODENAME}; remember it. The lab's dummy key is \
             {FAKE_KEY}: it is fake, never write it to a file. Create calc.py with add(a, b) and \
             test_calc.py with a unittest for it, then run `python3 -m unittest`. Keep a plan with \
             three items: add, sub, mul; mark add done."
        ),
    );
    let thread = thread.filter(|_| ok);
    let original = thread
        .as_deref()
        .and_then(|t| Original::of(&s.state, &s.account_x, t));
    let before = thread
        .as_deref()
        .map_or(Value::Null, |t| listed(&s.state, &s.account_x, t));
    let step1_ok = original.is_some()
        && tests_pass(&lab.project)
        && before["listed"] == true
        && before["session_id"].is_null();
    lab.steps.push(
        json!({"step": "J2.1 outside Codex task", "ok": step1_ok, "result": first,
                          "thread": thread, "listed": before}),
    );
    let (Some(thread), Some(original)) = (thread, original) else {
        remove_worktree(&project, &wt);
        s.finish("J2", lab);
        return;
    };
    // Control: Claude alone, in the same folder.
    let control = claude_turn(
        &lab,
        &s.claude.clone(),
        None,
        None,
        "Answer only from the instructions you were given, without running anything or opening any \
         file: what do this project's rules say about test method names? If you were given none, \
         say NONE.",
    );
    let control = control["result"].as_str().unwrap_or_default().to_owned();
    let key =
        juggler_app::features::conversations::switch_source::source_key(&s.account_x, &thread);
    let profile_c = s.profile_c.clone();
    let forward = match switch_forward(s, &mut lab, &key, &profile_c, &original, J2_CODENAME) {
        Ok(forward) => forward,
        Err(facts) => {
            lab.steps
                .push(json!({"step": "J2.2 switch Codex→Claude", "ok": false, "facts": facts}));
            remove_worktree(&project, &wt);
            s.finish("J2", lab);
            return;
        }
    };
    let pointer = primed_prompt(&forward.argv);
    let extra = add_dir_of(&forward.argv);
    let probe = claude_turn_in(
        &lab,
        &s.claude.clone(),
        &lab.project.clone(),
        None,
        None,
        &extra,
        &format!(
            "{pointer}\n\n---\nWithout changing any file: what is this project's codename, which plan \
             items are still open, what do this project's rules say about test method names, and \
             what is the dummy key mentioned earlier, exactly (say REDACTED if you can't see it)? \
             One short line."
        ),
    );
    let answer = probe["result"].as_str().unwrap_or_default().to_owned();
    let conversation = probe["session_id"].as_str().map(str::to_owned);
    let before_tests = test_methods(&lab.project.join("test_calc.py"));
    let cont = conversation.as_deref().map(|id| {
        claude_turn_in(
            &lab,
            &s.claude.clone(),
            &lab.project.clone(),
            None,
            Some(id),
            &[],
            &format!(
                "Now add sub(a, b) to calc.py with a test, following this project's rules, run \
                 `python3 -m unittest`, and mark sub done in your task list. Then decide that this \
                 version's release name is \"{J2_RELEASE}\". Say it only in your reply, in no file."
            ),
        )
    });
    let uses = s.probe.as_ref().map(|_| claude_uses_probe(&lab));
    let names = test_methods(&lab.project.join("test_calc.py"));
    let new_names: Vec<&String> = names.iter().filter(|n| !before_tests.contains(n)).collect();
    let areas = json!({
        "transcript-resume": mentions(&answer, J2_CODENAME),
        "todos": mentions(&answer, "sub") && mentions(&answer, "mul"),
        "rules-carry": mentions(&answer, "_lab") && !new_names.is_empty()
            && new_names.iter().all(|n| n.ends_with("_lab")) && !mentions(&control, "_lab"),
        "secret-stripped": !answer.contains(FAKE_KEY) && forward.facts["handover"]["secret_absent"] == true,
        "mcp-references": s.probe.as_ref().map(|_| setup_ok(&forward.facts["setup"], uses.as_ref())),
        "cwd-worktree-edges": forward.facts["row"]["same_folder"] == true
            && docstring_has_lab(&lab.project.join("calc.py"), "sub")
            && !project.join("calc.py").exists(),
        "source-read-only": original.unchanged(),
        "tests": cont.as_ref().is_some_and(|c| c["is_error"] == false) && tests_pass(&lab.project),
        "said-only-not-written": !written_anywhere(&project, J2_RELEASE),
    });
    let ok = all_true(&areas);
    lab.steps.push(
        json!({"step": "J2.2 switch Codex→Claude", "ok": ok, "areas": areas,
        "control": control, "probe": answer, "continued": cont.map(|c| c["result"].clone()),
        "conversation": conversation, "new_test_methods": new_names, "uses": uses,
        "listed_after": listed(&s.state, &s.account_x, &thread),
        "lineage": replay_lineage(&s.state, &forward.done.session_id),
        "graph": graph_view(&s.state, &forward.done.session_id), "facts": forward.facts}),
    );

    let resume = |lab: &Lab, prompt: &str| codex_turn(lab, Some(&thread), prompt).1;
    let back = switch_back_to_start(
        s,
        &lab,
        &forward.done.session_id,
        &original,
        &BackProbe {
            native: &thread,
            label: CODEX_LABEL,
            said_only: J2_RELEASE,
            question: "Without changing or reading any file: what is the codename, what release \
                       name was picked while you were away, and which plan items are still open? \
                       One short line.",
            resume: &resume,
        },
    );
    let areas = json!({
        "switch-back": back_ok(&back, J2_CODENAME),
        "todos": mentions(back["probe"].as_str().unwrap_or_default(), "mul"),
        "one-file": codex_files_named(&s.codex, &thread) == 1,
        "cwd-worktree-edges": back["row"]["same_folder"] == true,
        "tests": tests_pass(&lab.project),
    });
    let mut back = finish_back(back, "J2.3 switch back Claude→Codex", areas);
    back["listed_after"] = listed(&s.state, &s.account_x, &thread);
    lab.steps.push(back);
    remove_worktree(&project, &wt);
    s.finish("J2", lab);
}

/// J3: the old path, in a worktree: Claude picked up with "Continue in AI
/// Juggler" (AI Juggler's own session) → Codex → back to Claude.
fn j3_old_path(s: &mut Suite) {
    let project = s.root.join("j3");
    new_project(&project);
    let Some(wt) = worktree_of(&project, "wren") else {
        s.steps
            .push(json!({"journey": "J3", "step": "J3.0 worktree", "ok": false}));
        return;
    };
    let mut lab = s.lab(&wt);
    let native = uuid_v4();
    let first = claude_turn_in(
        &lab,
        &s.claude.clone(),
        &lab.project.clone(),
        Some(&native),
        None,
        &[],
        &format!(
            "This worktree's codename is {J3_CODENAME}; remember it. The lab's dummy key is \
             {FAKE_KEY}: it is fake, never write it to a file. Create calc.py with neg(a), and \
             test_calc.py with a unittest for it, then run `python3 -m unittest`. Keep a task list \
             with two items: neg and absval. Mark neg done."
        ),
    );
    let original = Original::of(&s.state, &s.account_c, &native);
    // "Continue in AI Juggler": AI Juggler's own session on it.
    let session = match juggler_app::features::conversations::continue_impl(
        &s.state,
        &s.account_c,
        &native,
    ) {
        Ok(done) => {
            let (argv, _) = take_launch_seen(&s.state, &done.session_id, done.pid);
            let ours = listed(&s.state, &s.account_c, &native);
            let ok = first["is_error"] == false
                && tests_pass(&lab.project)
                && original.is_some()
                && ours["session_id"] == json!(done.session_id)
                && argv.iter().any(|a| a == &native);
            lab.steps.push(
                json!({"step": "J3.1 Claude task, continued in AI Juggler", "ok": ok,
                "result": first["result"], "session": done.session_id, "native": native,
                "listed": ours, "launch": argv}),
            );
            done.session_id
        }
        Err(error) => {
            lab.steps.push(
                json!({"step": "J3.1 Claude task, continued in AI Juggler", "ok": false,
                                  "error": format!("{error:?}"), "result": first["result"]}),
            );
            remove_worktree(&project, &wt);
            s.finish("J3", lab);
            return;
        }
    };
    let Some(original) = original else {
        remove_worktree(&project, &wt);
        s.finish("J3", lab);
        return;
    };
    let profile_x = s.profile_x.clone();
    let forward = match switch_forward(s, &mut lab, &session, &profile_x, &original, J3_CODENAME) {
        Ok(forward) => forward,
        Err(facts) => {
            lab.steps.push(
                json!({"step": "J3.2 switch Claude→Codex (old path)", "ok": false,
                                  "facts": facts}),
            );
            remove_worktree(&project, &wt);
            s.finish("J3", lab);
            return;
        }
    };
    let source_row = session_row(&s.state, &session);
    let relinked = source_row
        .as_ref()
        .and_then(|r| r.transcript_ref.as_ref())
        .is_some_and(|r| r.path == original.path);
    let brief = primed_prompt(&forward.argv);
    let (thread, answer, ok) = codex_turn(
        &lab,
        None,
        &format!(
            "{brief}\n\n---\nWithout changing any file: what is this worktree's codename, which tasks \
             are still open, and what is the dummy key mentioned earlier, exactly (say REDACTED if \
             you can't see it)? One short line."
        ),
    );
    let (_, cont, cont_ok) = match &thread {
        Some(id) => codex_turn(
            &lab,
            Some(id),
            &format!(
                "Now add absval(a) to calc.py with a test, following this project's rules, run \
                 `python3 -m unittest`, and mark it done. Then tell me, only in your reply and in no \
                 file, that the next milestone is called \"{J3_MILESTONE}\"."
            ),
        ),
        None => (None, String::new(), false),
    };
    // The portable pack, exported from the source's epoch (ADR-0009).
    let pack = forward.facts["edge"]["from_epoch"]
        .as_str()
        .map(|epoch| pack_facts(&s.state, epoch, &lab.project));
    let areas = json!({
        "transcript-resume": mentions(&answer, J3_CODENAME),
        "todos": mentions(&answer, "absval"),
        "secret-stripped": !answer.contains(FAKE_KEY) && forward.facts["handover"]["secret_absent"] == true,
        "cwd-worktree-edges": forward.facts["row"]["same_folder"] == true
            && std::fs::read_to_string(lab.project.join("calc.py")).is_ok_and(|t| t.contains("def absval("))
            && !project.join("calc.py").exists(),
        "source-handed-over": source_row.as_ref().is_some_and(|r| r.status.as_str() == "handed_over"),
        "link-found-again": relinked,
        "pack-diffs": pack.as_ref().is_some_and(|p| p["patch_has_every_change"] == true
            && p["conversation_secret_absent"] == true),
        "source-read-only": original.unchanged(),
        "tests": cont_ok && tests_pass(&lab.project),
    });
    let ok = ok && all_true(&areas);
    lab.steps.push(
        json!({"step": "J3.2 switch Claude→Codex (old path)", "ok": ok, "areas": areas,
        "probe": answer, "continued": cont, "thread": thread, "pack": pack,
        "source_status": source_row.map(|r| r.status.as_str().to_owned()),
        "lineage_target": replay_lineage(&s.state, &forward.done.session_id),
        "lineage_source": replay_lineage(&s.state, &session),
        "graph": graph_view(&s.state, &forward.done.session_id), "facts": forward.facts}),
    );

    let claude = s.claude.clone();
    let resume = |lab: &Lab, prompt: &str| {
        claude_turn_in(
            lab,
            &claude,
            &lab.project.clone(),
            None,
            Some(&native),
            &[],
            prompt,
        )["result"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    let back = switch_back_to_start(
        s,
        &lab,
        &forward.done.session_id,
        &original,
        &BackProbe {
            native: &native,
            label: CLAUDE_LABEL,
            said_only: J3_MILESTONE,
            question: "Without changing or reading any file: what is this worktree's codename, and \
                       what is the next milestone called? One short line.",
            resume: &resume,
        },
    );
    let areas = json!({
        "switch-back": back_ok(&back, J3_CODENAME),
        "one-file": claude_files_named(&s.claude, &native) == 1,
        "cwd-worktree-edges": back["row"]["same_folder"] == true,
    });
    lab.steps.push(finish_back(
        back,
        "J3.3 switch back Codex→Claude (old path)",
        areas,
    ));
    remove_worktree(&project, &wt);
    s.finish("J3", lab);
}

/// A pack exported from `epoch`: its files, the redaction it records, and
/// whether its `diffs/` hold the project's changes.
fn pack_facts(state: &Arc<AppState>, epoch: &str, project: &Path) -> Value {
    let dir = match commands::pack_export_impl(state, epoch) {
        Ok(dir) => PathBuf::from(dir),
        Err(error) => return json!({"error": format!("{error:?}")}),
    };
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap_or_default();
    let state_json: Value = serde_json::from_str(&read("state.json")).unwrap_or(Value::Null);
    let manifest: Value = serde_json::from_str(&read("manifest.json")).unwrap_or(Value::Null);
    let diffs: Vec<String> = std::fs::read_dir(dir.join("diffs"))
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    // ADR-0009: the pack's `diffs/` hold the project's changes.
    let changed = git_changes(project);
    let patch = read("diffs/uncommitted.patch");
    let patch_has_every_change = !changed.is_empty()
        && changed
            .iter()
            .all(|file| patch.contains(&format!("b/{file}")))
        && !patch.contains(FAKE_KEY);
    let files: BTreeMap<&str, bool> = ["PACK.md", "conversation.md", "state.json", "manifest.json"]
        .into_iter()
        .map(|name| (name, dir.join(name).is_file()))
        .collect();
    let rows: Vec<Value> = manifest
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| json!({"path": row["rel_path"], "kind": row["kind"], "disposition": row["disposition"]}))
                .collect()
        })
        .unwrap_or_default();
    json!({"files": files,
           "conversation_secret_absent": !read("conversation.md").contains(FAKE_KEY),
           "pack_secret_absent": !read("PACK.md").contains(FAKE_KEY),
           "redactions": state_json["redactions"], "lineage": state_json["lineage"],
           "manifest_rows": rows, "diffs": diffs, "git_changes": changed,
           "patch_has_every_change": patch_has_every_change,
           "pack_md_git_line": read("PACK.md").lines().find(|l| l.starts_with("- Git:")).unwrap_or_default()})
}

fn all_true(areas: &Value) -> bool {
    areas
        .as_object()
        .is_some_and(|map| map.values().all(|v| v.is_null() || *v == true))
}

#[test]
#[ignore = "the live switching lab (0.1.8); needs AIJ_LAB_* and real, agreed accounts"]
fn switch_lab_outside_live() {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let (Some(data), Some(root), Some(claude), Some(codex), Some(out)) = (
        var("AIJ_LAB_DATA"),
        var("AIJ_LAB_PROJECT"),
        var("AIJ_LAB_CLAUDE_A"),
        var("AIJ_LAB_CODEX"),
        var("AIJ_LAB_OUT"),
    ) else {
        eprintln!("AIJ_LAB_* not set: nothing to do");
        return;
    };
    if std::env::var("AIJ_LAB_SUITE").as_deref() != Ok("outside") {
        eprintln!("AIJ_LAB_SUITE is not outside: nothing to do");
        return;
    }
    assert!(
        std::env::var_os("CLAUDE_CONFIG_DIR").is_none() && std::env::var_os("CODEX_HOME").is_none(),
        "run the lab with CLAUDE_CONFIG_DIR and CODEX_HOME unset (env -u …)"
    );
    assert_clean_env();
    assert!(
        !data.exists() || std::fs::read_dir(&data).unwrap().next().is_none(),
        "AIJ_LAB_DATA must be a fresh, empty folder"
    );
    assert!(
        !root.exists(),
        "AIJ_LAB_PROJECT must not exist yet: the lab creates it"
    );
    std::fs::create_dir_all(root.join("probe")).unwrap();
    std::fs::write(root.join("probe").join("server.py"), PROBE_SERVER).unwrap();
    let root = root.canonicalize().unwrap();
    let journeys = std::env::var("AIJ_LAB_JOURNEYS").unwrap_or_else(|_| "j1,j2,j3".to_owned());

    let home = dirs::home_dir().unwrap();
    let roots: Vec<PathBuf> = vec![
        claude.clone(),
        codex.clone(),
        home.join(".claude.json"),
        home.join(".agents"),
    ];
    let before = snapshot(&roots);

    let state = Arc::new(AppState::init(data).unwrap());
    // The user's policy, chosen before any switch (D-14): strip secrets.
    commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();
    let account_c = import(&state, CLAUDE_CODE_ID, &claude, CLAUDE_LABEL);
    let account_x = import(&state, CODEX_ID, &codex, CODEX_LABEL);
    let mut suite = Suite {
        profile_c: profile_of(&state, &account_c),
        profile_x: profile_of(&state, &account_x),
        state,
        root: root.clone(),
        claude: claude.clone(),
        codex: codex.clone(),
        account_c,
        account_x,
        probe: std::env::var("AIJ_LAB_PROBE").ok(),
        steps: Vec::new(),
    };

    let probe = suite.probe.clone();
    if journeys.contains("j1") {
        let planted = probe
            .as_ref()
            .map(|name| plant_probe_claude(&root, &claude, name));
        j1_outside_claude(&mut suite);
        if let (Some(name), Some(planted)) = (&probe, planted) {
            let removed = unplant_probe_claude(&root, &claude, name);
            suite.steps.push(
                json!({"journey": "J1", "step": "J1 probe on Claude, then removed",
                                    "ok": planted && removed}),
            );
        }
    }
    if journeys.contains("j2") {
        let planted = probe
            .as_ref()
            .map(|name| plant_probe_codex(&root, &codex, name));
        j2_outside_codex(&mut suite);
        if let (Some(name), Some(planted)) = (&probe, planted) {
            let removed = unplant_probe_codex(&root, &codex, name);
            suite.steps.push(
                json!({"journey": "J2", "step": "J2 probe on Codex, then removed",
                                    "ok": planted && removed}),
            );
        }
    }
    if journeys.contains("j3") {
        j3_old_path(&mut suite);
    }

    let after = snapshot(&roots);
    let report = json!({
        "tools": {
            "claude": run(Command::new("claude").arg("--version"), Duration::from_secs(20)).1.trim(),
            "codex": run(Command::new("codex").arg("--version"), Duration::from_secs(20)).1.trim(),
        },
        "steps": suite.steps,
        "snapshot": snapshot_diff(&before, &after),
    });
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    eprintln!("lab report: {}", out.display());
    let failed = report["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["ok"] != true)
        .count();
    assert!(failed == 0, "lab steps failed; see the report");
}
