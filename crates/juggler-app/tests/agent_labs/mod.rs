//! The rig the agent labs share (E18 messaging, E19 tasks): AI Juggler's
//! backend over a fresh data folder, with the app's own background work,
//! two real tools and the real bridge program, driven the way the window
//! and the user would drive them. See `agent_labs_live.rs`.
//!
//! What it records goes to `AIJ_LAB_OUT/report-<suite>.json` and holds
//! real paths and ids: it stays out of the repository (the evidence is
//! written from it by hand, redacted).

pub mod messaging;
pub mod tasks;
mod util;

pub use util::*;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID};
use juggler_app::commands;
use juggler_app::features::{agent_bridge, messages, pools, runtime};
use juggler_app::state::AppState;
use juggler_core::model::SessionId;
use serde_json::{Value, json};

/// What the lab is given (every `AIJ_LAB_*` variable).
pub struct Env {
    pub suite: String,
    pub data: PathBuf,
    pub project: PathBuf,
    pub claude: PathBuf,
    /// A second Claude profile, for the tasks lab's switch step (6).
    pub claude_b: Option<PathBuf>,
    pub codex: PathBuf,
    pub out: PathBuf,
    /// The real `juggler-agent`, as built.
    pub agent: PathBuf,
}

impl Env {
    /// The lab's variables, or `None` (nothing to do) when one is missing.
    /// Refuses a used data folder, an existing project folder, or a
    /// `CLAUDE_CONFIG_DIR`/`CODEX_HOME` in the environment.
    pub fn read() -> Option<Self> {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
        let (Some(data), Some(project), Some(claude), Some(codex), Some(out), Some(agent)) = (
            var("AIJ_LAB_DATA"),
            var("AIJ_LAB_PROJECT"),
            var("AIJ_LAB_CLAUDE"),
            var("AIJ_LAB_CODEX"),
            var("AIJ_LAB_OUT"),
            var("AIJ_LAB_AGENT"),
        ) else {
            eprintln!("AIJ_LAB_* not set: nothing to do");
            return None;
        };
        let suite = std::env::var("AIJ_LAB_SUITE").unwrap_or_default();
        assert!(
            matches!(suite.as_str(), "messaging" | "tasks"),
            "AIJ_LAB_SUITE is messaging or tasks"
        );
        assert!(
            std::env::var_os("CLAUDE_CONFIG_DIR").is_none()
                && std::env::var_os("CODEX_HOME").is_none(),
            "run the lab with CLAUDE_CONFIG_DIR and CODEX_HOME unset (env -u …)"
        );
        assert_clean_env();
        assert_eq!(
            std::env::var("DISABLE_AUTOUPDATER").as_deref(),
            Ok("1"),
            "run the lab with DISABLE_AUTOUPDATER=1 (every tool it starts inherits it)"
        );
        assert!(
            !data.exists() || std::fs::read_dir(&data).unwrap().next().is_none(),
            "AIJ_LAB_DATA must be a fresh, empty folder"
        );
        assert!(
            !project.exists(),
            "AIJ_LAB_PROJECT must not exist yet: the lab creates it"
        );
        assert!(
            agent.is_file(),
            "AIJ_LAB_AGENT must be the built juggler-agent"
        );
        Some(Self {
            suite,
            data,
            project,
            claude,
            claude_b: var("AIJ_LAB_CLAUDE_B"),
            codex,
            out,
            agent,
        })
    }
}

/// One tool call the bridge answered, from the wrapper's logs.
#[derive(Clone, Debug)]
pub struct BridgeCall {
    pub tool: String,
    pub args: Value,
    pub text: String,
    pub error: bool,
}

impl BridgeCall {
    /// The call as a report records it.
    pub fn seen(&self) -> Value {
        json!({"tool": self.tool, "args": self.args, "answer": self.text, "error": self.error})
    }
}

/// The lab: the app's state, its background work, the sessions it started
/// and what each step saw.
pub struct Rig {
    pub env: Env,
    pub state: Arc<AppState>,
    pub project: PathBuf,
    pub claude_profile: String,
    pub codex_profile: String,
    pub claude_b_profile: Option<String>,
    pub steps: Vec<Value>,
    /// Every session the lab started, stopped at the end.
    pub started: Vec<String>,
    logs: PathBuf,
    ticking: Arc<AtomicBool>,
    pub tick_panics: Arc<AtomicUsize>,
    tick_thread: Option<std::thread::JoinHandle<()>>,
}

impl Rig {
    /// A new lab project (git, a one-line README), AI Juggler over the
    /// fresh data folder, both accounts brought in in place, the agent
    /// tools answered yes, the bridge program wrapped so its stdin and
    /// stdout lines are logged (never its environment), and the app's
    /// background work running.
    pub fn new(env: Env) -> Self {
        std::fs::create_dir_all(&env.project).unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(&env.project)
                .output()
                .unwrap();
        };
        git(&["init", "-q"]);
        std::fs::write(env.project.join("README.md"), "lab\n").unwrap();
        let project = env.project.canonicalize().unwrap();

        std::fs::create_dir_all(&env.out).unwrap();
        set_mode(&env.out, 0o700);
        let logs = env.out.join(format!("bridge-{}", env.suite));
        std::fs::create_dir_all(&logs).unwrap();
        set_mode(&logs, 0o700);
        let wrapper = env.out.join(format!("juggler-agent-{}", env.suite));
        std::fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\n# The agent lab's bridge: logs its stdin and stdout lines, never its environment.\n\
                 mode=$(printf %s \"$1\" | tr -cd 'a-z')\nlog='{}'/\"$$-$mode\"\n\
                 tee -a \"$log.in\" | '{}' \"$@\" | tee -a \"$log.out\"\n",
                logs.display(),
                env.agent.display()
            ),
        )
        .unwrap();
        set_mode(&wrapper, 0o700);

        let state = Arc::new(AppState::init(env.data.clone()).unwrap());
        state.agent_bridge.set_program(Some(wrapper));
        assert!(
            state.agent_bridge.program().is_some(),
            "the wrapped bridge program isn't trusted (its folder must be the user's, 0700)"
        );
        agent_bridge::settings::answer_impl(&state, true).unwrap();
        let claude_profile = import(&state, CLAUDE_CODE_ID, &env.claude, "Lab Luka");
        let codex_profile = import(&state, CODEX_ID, &env.codex, "Lab Codex");
        let claude_b_profile = env
            .claude_b
            .clone()
            .map(|folder| import(&state, CLAUDE_CODE_ID, &folder, "Lab Anela"));

        // The app's own background work, as lib.rs runs it without a window.
        juggler_app::features::sessions::lifecycle::spawn_forwarder(Arc::clone(&state));
        let ticking = Arc::new(AtomicBool::new(true));
        let tick_panics = Arc::new(AtomicUsize::new(0));
        let tick_thread = {
            let state = Arc::clone(&state);
            let ticking = Arc::clone(&ticking);
            let panics = Arc::clone(&tick_panics);
            std::thread::Builder::new()
                .name("lab-ticks".into())
                .spawn(move || {
                    let mut round = 0u64;
                    while ticking.load(Ordering::Relaxed) {
                        let part = |work: &dyn Fn()| {
                            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).is_err()
                            {
                                panics.fetch_add(1, Ordering::Relaxed);
                            }
                        };
                        // The 2 s tick: processes, the graph, the pools' cards.
                        part(&|| commands::processes_tick(&state));
                        part(&|| {
                            let _ = juggler_app::features::processes::tick(&state);
                        });
                        part(&|| pools::cards::tick(&state));
                        // The 5 s one's Needs you refresh.
                        if round.is_multiple_of(3) {
                            part(&|| {
                                let _ =
                                    juggler_app::features::notifications::needs::refresh(&state);
                            });
                        }
                        round += 1;
                        std::thread::sleep(Duration::from_secs(2));
                    }
                })
                .unwrap()
        };
        Self {
            env,
            state,
            project,
            claude_profile,
            codex_profile,
            claude_b_profile,
            steps: Vec::new(),
            started: Vec::new(),
            logs,
            ticking,
            tick_panics,
            tick_thread: Some(tick_thread),
        }
    }

    /// Records one checklist step.
    pub fn step(&mut self, step: &str, result: &str, seen: Value) {
        eprintln!("[lab] {step}: {result}");
        self.steps
            .push(json!({"step": step, "result": result, "seen": seen}));
    }

    /// Names a session (its menu › Rename).
    pub fn name(&self, session: &str, name: &str) {
        self.state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .set_label(&SessionId::new(session), Some(name))
            .unwrap();
    }

    /// Starts Codex in the lab folder, driven, after answering AI
    /// Juggler's trust sheet Open restricted. Its session id and what the
    /// sheet said.
    pub fn start_builder(&mut self, name: &str) -> (Option<String>, Value) {
        let folder = self.project.display().to_string();
        let sheet = runtime::trust::trust_impl(&self.state, &self.codex_profile, Some(&folder))
            .map(|info| serde_json::to_value(info).unwrap())
            .unwrap_or_else(|error| json!({"error": format!("{error:?}")}));
        let chose = runtime::trust::trust_choose_impl(
            &self.state,
            &self.codex_profile,
            Some(&folder),
            false,
        )
        .map(|info| serde_json::to_value(info).unwrap())
        .unwrap_or_else(|error| json!({"error": format!("{error:?}")}));
        runtime::commands::choose_impl(&self.state, &self.codex_profile, Some("driven")).unwrap();
        let launched = commands::launch_profile(&self.state, &self.codex_profile, Some(folder));
        let Ok(launched) = launched else {
            return (
                None,
                json!({"sheet": sheet, "chose": chose, "error": format!("{:?}", launched.err())}),
            );
        };
        let session = launched.session_id.clone();
        self.started.push(session.clone());
        let driven = self
            .state
            .runtime
            .is_driven(&SessionId::new(session.as_str()));
        let ready = wait_for(Duration::from_secs(120), || self.between_turns(&session));
        self.name(&session, name);
        let info = self
            .state
            .runtime
            .info(&session)
            .map(|info| serde_json::to_value(info).unwrap_or_default());
        (
            Some(session),
            json!({"sheet": sheet, "chose": chose, "driven": driven, "ready": ready,
                   "runtime": info}),
        )
    }

    /// Starts Claude Code in the lab folder in AI Juggler's terminal and
    /// answers Claude's own folder-trust question yes, for the lab folder
    /// only (its default is "No, exit": Down, then Enter).
    pub fn start_planner(&mut self, profile: &str, name: &str) -> (Option<String>, Value) {
        runtime::commands::choose_impl(&self.state, profile, Some("terminal")).unwrap();
        let folder = self.project.display().to_string();
        let launched = commands::launch_profile(&self.state, profile, Some(folder));
        let Ok(launched) = launched else {
            return (None, json!({"error": format!("{:?}", launched.err())}));
        };
        let session = launched.session_id.clone();
        self.started.push(session.clone());
        let _ = juggler_app::terminal::terminal_resize_impl(&self.state, &session, 120, 40);
        let trust = self.answer_claude_trust(&session);
        self.name(&session, name);
        (Some(session), json!({"trust": trust, "pid": launched.pid}))
    }

    /// Claude Code's folder-trust question, when it asks: yes for the lab
    /// folder only.
    pub fn answer_claude_trust(&self, session: &str) -> Value {
        let leaf = self
            .project
            .file_name()
            .map(|name| squash(&name.to_string_lossy()))
            .unwrap_or_default();
        let asked = wait_for(Duration::from_secs(40), || {
            let screen = squash(&self.screen(session));
            screen.contains("trust") && screen.contains("exit") && screen.contains(&leaf)
        });
        // The dialog as it showed (its last lines; the report stays out of
        // the repository), so a changed dialog is seen, not guessed at.
        let dialog: Vec<String> = self
            .screen(session)
            .lines()
            .rev()
            .filter(|line| !line.trim().is_empty())
            .take(12)
            .map(str::to_owned)
            .collect();
        // Its options: "No, exit" (selected first), then "Yes, I trust this
        // folder". Under load the dialog redraws while Claude Code finishes
        // starting, and a redraw puts the selection back on "No, exit" (the
        // lab of 2026-10-28: a Down, then a redraw, then Enter would have
        // exited). So each key waits for the screen to settle and is chosen
        // from the selection the newest frame shows, until the dialog is
        // gone.
        let mut confirmed = false;
        for _ in 0..8 {
            if !asked {
                break;
            }
            self.wait_quiet(session, Duration::from_secs(2), Duration::from_secs(30));
            let screen = self.screen(session);
            let recent: Vec<String> = screen
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(squash)
                .collect();
            let recent = &recent[recent.len().saturating_sub(10)..];
            if !recent.iter().any(|line| line.contains("entertoconfirm")) {
                confirmed = true;
                break;
            }
            let selected = recent.iter().rev().find(|line| line.contains('❯'));
            if selected.is_some_and(|line| line.contains("❯yes,itrustthisfolder")) {
                self.keys(session, "\r");
            } else {
                self.keys(session, "\x1b[B");
            }
        }
        let quiet = self.wait_quiet(session, Duration::from_secs(4), Duration::from_secs(60));
        let running = !self.state.supervisor.is_finished(&SessionId::new(session));
        // "No, exit" ends it: still running after the dialog is the yes.
        json!({"asked_for_the_lab_folder": asked, "answered_yes": asked && running,
               "dialog_gone_from_the_last_lines": confirmed, "quiet_after": quiet,
               "dialog_last_lines": dialog, "still_running": running,
               "screen_after": self.screen(session).lines().rev().filter(|l| !l.trim().is_empty())
                   .take(8).map(str::to_owned).collect::<Vec<_>>()})
    }

    /// The terminal's retained output as plain text (escape sequences
    /// removed).
    pub fn screen(&self, session: &str) -> String {
        self.state
            .supervisor
            .output(&SessionId::new(session))
            .map(|bytes| plain(&String::from_utf8_lossy(&bytes)))
            .unwrap_or_default()
    }

    /// Keys into a terminal, as the user presses them.
    pub fn keys(&self, session: &str, keys: &str) {
        let _ = juggler_app::terminal::terminal_write_impl(&self.state, session, keys);
    }

    /// A prompt typed into a terminal, then Enter, then the wait for the
    /// tool to go quiet (its turn over).
    pub fn type_prompt(&self, session: &str, prompt: &str, cap: Duration) -> bool {
        self.keys(session, prompt);
        std::thread::sleep(Duration::from_millis(500));
        self.keys(session, "\r");
        // Its turn starts (the screen changes), then it goes quiet.
        std::thread::sleep(Duration::from_secs(3));
        self.wait_quiet(session, Duration::from_secs(6), cap)
    }

    /// Waits until a terminal's output hasn't changed for `quiet`.
    pub fn wait_quiet(&self, session: &str, quiet: Duration, cap: Duration) -> bool {
        let started = Instant::now();
        let mut last = self.output_len(session);
        let mut since = Instant::now();
        while started.elapsed() < cap {
            std::thread::sleep(Duration::from_millis(500));
            let now = self.output_len(session);
            if now != last {
                last = now;
                since = Instant::now();
            } else if since.elapsed() >= quiet {
                return true;
            }
        }
        false
    }

    fn output_len(&self, session: &str) -> (usize, u64) {
        use std::hash::{Hash, Hasher};
        let bytes = self
            .state
            .supervisor
            .output(&SessionId::new(session))
            .unwrap_or_default();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        (bytes.len(), hasher.finish())
    }

    /// The user writes in a driven session's message box.
    pub fn builder_say(&self, session: &str, text: &str) -> Result<(), String> {
        runtime::commands::send_impl(&self.state, session, text).map_err(|e| format!("{e:?}"))
    }

    /// Whether a driven session is between turns (and the user isn't
    /// typing in it).
    pub fn between_turns(&self, session: &str) -> bool {
        messages::delivery::between_turns(&self.state, &SessionId::new(session))
    }

    /// Waits for a driven session's turn to start and end.
    pub fn wait_builder_turn(&self, session: &str, cap: Duration) -> bool {
        let _ = wait_for(Duration::from_secs(15), || !self.between_turns(session));
        wait_for(cap, || self.between_turns(session))
    }

    /// [`Self::wait_builder_turn`], answering each ask the tool opens
    /// meanwhile with the option labelled `word` (the user's answer, as
    /// the checklist names it); what was asked is returned for the report.
    pub fn builder_turn_answering(
        &self,
        session: &str,
        cap: Duration,
        word: &str,
    ) -> (bool, Vec<Value>) {
        let started = Instant::now();
        let mut answered = Vec::new();
        let _ = wait_for(Duration::from_secs(15), || !self.between_turns(session));
        loop {
            let left = cap.saturating_sub(started.elapsed());
            let settled = wait_for(left, || {
                self.between_turns(session) || !self.asks(session).is_empty()
            });
            if !settled {
                return (false, answered);
            }
            if self.asks(session).is_empty() {
                return (true, answered);
            }
            answered.push(self.answer_ask(session, word));
            if answered.len() > 5 {
                return (false, answered);
            }
        }
    }

    /// A driven session's conversation lines.
    pub fn builder_lines(&self, session: &str) -> Vec<Value> {
        runtime::commands::get_impl(&self.state, session)
            .ok()
            .flatten()
            .map(|view| {
                view.items
                    .into_iter()
                    .map(|item| json!({"kind": item.kind, "text": item.text}))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The open asks of `session`'s tool.
    pub fn asks(&self, session: &str) -> Vec<runtime::dto::AskInfo> {
        runtime::commands::asks_impl(&self.state, 0, 50)
            .into_iter()
            .filter(|ask| ask.session_id == session)
            .collect()
    }

    /// The user answers an ask with the option whose label has `word`.
    pub fn answer_ask(&self, session: &str, word: &str) -> Value {
        let Some(ask) = self.asks(session).into_iter().next() else {
            return json!({"error": "no ask open"});
        };
        let option = ask
            .options
            .iter()
            .find(|option| option.label.to_lowercase().contains(word))
            .map(|option| option.id.clone());
        let Some(option) = option else {
            return json!({"error": "no such option", "ask": serde_json::to_value(&ask).unwrap()});
        };
        let answered = runtime::commands::answer_impl(
            &self.state,
            session,
            &ask.id,
            runtime::dto::AnswerInput {
                option: Some(option.clone()),
                replies: None,
            },
        );
        json!({"ask": serde_json::to_value(&ask).unwrap(), "option": option,
               "ok": answered.is_ok(), "error": answered.err().map(|e| format!("{e:?}"))})
    }

    /// Every tool call the bridge answered so far, in each process's order.
    pub fn calls(&self) -> Vec<BridgeCall> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.logs) else {
            return out;
        };
        let mut stems: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "in"))
            .collect();
        stems.sort();
        for input in stems {
            let output = input.with_extension("out");
            let mut asked: BTreeMap<String, (String, Value)> = BTreeMap::new();
            for line in std::fs::read_to_string(&input).unwrap_or_default().lines() {
                let Ok(value) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if value["method"] == "tools/call" {
                    asked.insert(
                        value["id"].to_string(),
                        (
                            value["params"]["name"]
                                .as_str()
                                .unwrap_or_default()
                                .to_owned(),
                            value["params"]["arguments"].clone(),
                        ),
                    );
                }
            }
            for line in std::fs::read_to_string(&output).unwrap_or_default().lines() {
                let Ok(value) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                let Some((tool, args)) = asked.remove(&value["id"].to_string()) else {
                    continue;
                };
                let text = value["result"]["content"]
                    .as_array()
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|part| part["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_else(|| value["error"].to_string());
                out.push(BridgeCall {
                    tool,
                    args,
                    text,
                    error: value["result"]["isError"] == true || value.get("error").is_some(),
                });
            }
        }
        out
    }

    /// The calls of one tool.
    pub fn calls_of(&self, tool: &str) -> Vec<BridgeCall> {
        self.calls()
            .into_iter()
            .filter(|call| call.tool == tool)
            .collect()
    }

    /// [`Self::wait_call`] for a call whose arguments `matches`, `before`
    /// counting only those (another
    /// session's call to the same tool can land meanwhile: the lab of
    /// 2026-10-28 took Builder's refused assign for Planner's).
    pub fn wait_call_where(
        &self,
        tool: &str,
        before: usize,
        cap: Duration,
        matches: impl Fn(&Value) -> bool,
    ) -> Option<BridgeCall> {
        let mut found = None;
        wait_for(cap, || {
            // Calls are listed log by log (one per bridge process), not in
            // time order: `before` counts the matching ones.
            found = self
                .calls_of(tool)
                .into_iter()
                .filter(|call| matches(&call.args))
                .nth(before);
            found.is_some()
        });
        found
    }

    /// Waits until `tool` has been answered more than `before` times.
    pub fn wait_call(&self, tool: &str, before: usize, cap: Duration) -> Option<BridgeCall> {
        let mut found = None;
        wait_for(cap, || {
            let calls = self.calls_of(tool);
            if calls.len() > before {
                found = calls.last().cloned();
                true
            } else {
                false
            }
        });
        found
    }

    /// What the bridge printed outside its MCP server (its hooks).
    pub fn hook_output(&self) -> String {
        let mut text = String::new();
        if let Ok(entries) = std::fs::read_dir(&self.logs) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name.ends_with("-hook.out") {
                    text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
                    text.push('\n');
                }
            }
        }
        text
    }

    /// The pool `session` is in.
    pub fn pool_of(&self, session: &str) -> Option<String> {
        let storage = self.state.storage.lock().unwrap();
        pools::members::place_of(&storage, &SessionId::new(session))
            .ok()
            .flatten()
            .map(|(pool, _)| pool)
    }

    /// The pool as the window lists it.
    pub fn pool_info(&self, pool: &str) -> Value {
        pools::pools_list_impl(&self.state)
            .ok()
            .and_then(|info| info.pools.into_iter().find(|p| p.id == pool))
            .map(|p| serde_json::to_value(p).unwrap())
            .unwrap_or(Value::Null)
    }

    /// Counts from AI Juggler's database (counts only, never text).
    pub fn db_counts(&self, sql: &str) -> String {
        std::process::Command::new("sqlite3")
            .arg(self.env.data.join("juggler.sqlite"))
            .arg(sql)
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
            .unwrap_or_default()
    }

    /// Stops one session the lab started and waits for it to end.
    pub fn stop(&self, session: &str) -> bool {
        let id = SessionId::new(session);
        let _ = self.state.supervisor.stop(&id, Duration::from_secs(5));
        wait_for(Duration::from_secs(20), || {
            self.state.supervisor.is_finished(&id)
        })
    }

    /// Stops everything, writes the report and fails on a failed step.
    pub fn finish(mut self) {
        for session in self.started.clone() {
            self.stop(&session);
        }
        self.ticking.store(false, Ordering::Relaxed);
        if let Some(thread) = self.tick_thread.take() {
            let _ = thread.join();
        }
        // A stopped Claude Code can take a few seconds to finish writing
        // its conversation and exit (the lab of 2026-10-28, with transcript
        // saving on): wait for that, bounded, and say how long it took.
        let stopping = Instant::now();
        let gone = || {
            own_processes().is_empty()
                && matching_processes(&[
                    &self.env.out.display().to_string(),
                    &self.env.agent.display().to_string(),
                ])
                .is_empty()
        };
        wait_for(Duration::from_secs(30), gone);
        let waited_ms = stopping.elapsed().as_millis();
        let left = own_processes();
        let stray = matching_processes(&[
            &self.env.out.display().to_string(),
            &self.env.agent.display().to_string(),
        ]);
        let clean = left.is_empty() && stray.is_empty();
        self.step(
            "9 clean up",
            if clean { "PASS" } else { "FAIL" },
            json!({"own_processes_left": left, "bridge_processes_left": stray,
                   "all_gone_after_ms": waited_ms,
                   "tick_panics": self.tick_panics.load(Ordering::Relaxed)}),
        );
        let report = json!({
            "suite": self.env.suite,
            "tools": {
                "claude": version("claude"),
                "codex": version("codex"),
                "gh": version("gh"),
            },
            "steps": self.steps,
        });
        let path = self.env.out.join(format!("report-{}.json", self.env.suite));
        std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        eprintln!("lab report: {}", path.display());
        let failed = self
            .steps
            .iter()
            .filter(|step| step["result"] == "FAIL")
            .count();
        assert_eq!(failed, 0, "{failed} lab step(s) failed; see the report");
    }
}

fn import(state: &Arc<AppState>, harness: &str, folder: &Path, label: &str) -> String {
    let done = commands::account_import_mode_impl(
        state,
        harness,
        &folder.display().to_string(),
        label,
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap_or_else(|error| panic!("bringing in {label}: {error:?}"));
    let account = done.account.id;
    let storage = state.storage.lock().unwrap();
    storage
        .profiles()
        .list_by_account(&juggler_core::model::AccountId::new(account.as_str()))
        .unwrap()
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{label} has launch settings"))
        .id
        .as_str()
        .to_owned()
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
