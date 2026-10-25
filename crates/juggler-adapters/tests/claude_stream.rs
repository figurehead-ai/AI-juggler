//! AJ-E2308: Claude Code without a terminal (Beta), over stream-json.
//!
//! - Recorded messages (shaped as the Agent SDK's types and the lab of
//!   2026-10-25 saw them) make the right events; answers go back in Claude
//!   Code's shapes, only with options it offered; a control request AI
//!   Juggler doesn't handle is refused, never answered for the user.
//! - End to end against `fake-claude`: the launch (never `--bare` or a
//!   bypass; `--verbose` with stream-json), the handshake, turns, an
//!   approval with Claude Code's own suggested rule, AskUserQuestion,
//!   interrupt, the model, limits named by window, release, the
//!   transcript found and resumed in the terminal.
//! - Only in a folder Claude Code trusts; too old a Claude Code, and the
//!   terminal runs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::collections::BTreeMap;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use juggler_adapter::runtime::{
    AskAnswer, Connection, DrivenLaunch, DrivenSession, ErrorCategory, Maturity, OptionEffect,
    PartAnswer, Release, Runtime, RuntimeEvent, TerminalView, Transport, TurnEnd, TurnInput,
    ViewOf,
};
use juggler_adapter::{HarnessAdapter, LaunchMode};
use juggler_adapters::ClaudeCodeAdapter;
use juggler_adapters::claude_stream::codec::Codec;
use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};
use serde_json::{Value, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-stream/messages-2.1.302.jsonl"
);

#[test]
fn recorded_messages_make_the_right_events() {
    let mut codec = Codec::default();
    codec.initialize();
    let text = std::fs::read_to_string(FIXTURE).unwrap();
    let mut events = Vec::new();
    let mut refusals = Vec::new();
    for line in text.lines() {
        let (more, replies) = codec.incoming(line);
        events.extend(more);
        refusals.extend(replies);
    }
    assert!(codec.initialized && codec.initializing.is_none());
    assert!(
        events.iter().any(
            |e| matches!(e, RuntimeEvent::ModelChanged { model } if model == "claude-opus-5-5")
        )
    );
    assert!(
        events.iter().any(
            |e| matches!(e, RuntimeEvent::Item { item } if item.text == "I'll run the tests.")
        )
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::Item { item } if item.text == "Bash: npm test"))
    );
    let approval = events
        .iter()
        .find_map(|e| match e {
            RuntimeEvent::ApprovalRequested { approval } => Some(approval.clone()),
            _ => None,
        })
        .unwrap();
    let labels: Vec<&str> = approval.options.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Allow once",
            "Always allow Bash(npm test:*), in this project (your local settings)",
            "Deny",
            "Deny and stop"
        ]
    );
    assert!(matches!(
        approval.options[1].effect,
        OptionEffect::AllowRule { .. }
    ));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::QuestionAsked { question }
        if question.parts[0].id == "Which database?" && question.parts[0].choices.len() == 2))
    );
    let limits = events
        .iter()
        .find_map(|e| match e {
            RuntimeEvent::LimitsUpdated { limits } => Some(limits.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        limits.windows.len(),
        1,
        "only the window in force, never a made-up one"
    );
    assert_eq!(limits.windows[0].id, "five_hour");
    assert_eq!(limits.windows[0].window_mins, Some(300));
    assert!((limits.windows[0].used_pct - 27.0).abs() < 0.01);
    assert_eq!(limits.windows[0].resets_at_ms, Some(1_761_397_200_000));
    assert!(events.iter().any(|e| matches!(e, RuntimeEvent::TurnEnded { outcome: TurnEnd::Completed, usage: Some(u) } if u.input == 1200 && u.cached_input == 800)));
    assert!(events.iter().any(|e| matches!(
        e,
        RuntimeEvent::TurnEnded {
            outcome: TurnEnd::Interrupted,
            ..
        }
    )));
    assert_eq!(
        refusals.len(),
        1,
        "the hook callback is refused, never answered"
    );
    assert_eq!(refusals[0]["response"]["subtype"], "error");
    assert!(codec.unknown >= 2);
}

#[test]
fn answers_go_back_in_claude_codes_shapes() {
    let mut codec = Codec::default();
    let text = std::fs::read_to_string(FIXTURE).unwrap();
    for line in text.lines().take(5) {
        codec.incoming(line);
    }
    let choose = |option: &str| AskAnswer::Choose {
        option: option.into(),
    };
    let tool = juggler_adapter::runtime::AskId("cli-1".into());
    assert!(
        codec.answer(&tool, &choose("rule:7")).is_err(),
        "a rule it didn't suggest"
    );
    let (line, _) = codec.answer(&tool, &choose("rule:0")).unwrap();
    assert_eq!(line["type"], "control_response");
    assert_eq!(line["response"]["request_id"], "cli-1");
    assert_eq!(line["response"]["response"]["behavior"], "allow");
    assert_eq!(
        line["response"]["response"]["updatedInput"],
        json!({"command": "npm test"})
    );
    assert_eq!(
        line["response"]["response"]["updatedPermissions"][0]["rules"][0]["ruleContent"],
        "npm test:*"
    );
    assert!(
        codec.answer(&tool, &choose("allow")).is_err(),
        "never twice"
    );
    let question = juggler_adapter::runtime::AskId("cli-2".into());
    let (line, _) = codec
        .answer(
            &question,
            &AskAnswer::Reply {
                parts: vec![PartAnswer {
                    part: "Which database?".into(),
                    answers: vec!["Postgres".into()],
                }],
            },
        )
        .unwrap();
    assert_eq!(
        line["response"]["response"]["updatedInput"]["answers"],
        json!({"Which database?": "Postgres"})
    );
}

// ---------------------------------------------------------------------------
// End to end against fake-claude

struct World {
    _root: tempfile::TempDir,
    base: PathBuf,
    account: Account,
    profile: Profile,
    cwd: PathBuf,
}

fn world(trust: bool) -> World {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let state_dir = base.join("claude-config");
    let cwd = base.join("project");
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&state_dir).unwrap();
    if trust {
        let projects =
            json!({ cwd.to_string_lossy(): {"hasTrustDialogAccepted": true, "allowedTools": []} });
        std::fs::write(
            state_dir.join(".claude.json"),
            json!({"projects": projects, "oauthAccount": {"emailAddress": "never-kept@example.com"}}).to_string(),
        )
        .unwrap();
    }
    let account = Account {
        id: AccountId::new("claude-acct"),
        harness: HarnessId::new("claude-code"),
        label: "Claude".into(),
        plan: None,
        state_dir,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    let profile = Profile {
        id: ProfileId::new("p"),
        name: "p".into(),
        account: account.id.clone(),
        settings_overlay: SettingsOverlay {
            model: Some("claude-fake-1".into()),
            env: BTreeMap::new(),
            restart_on_crash: None,
        },
        default_cwd: None,
    };
    World {
        _root: root,
        base,
        account,
        profile,
        cwd,
    }
}

struct Proc(Child);

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

const SESSION: &str = "0b1c2d3e-0000-4000-8000-0000000000bb";

fn start(
    world: &World,
    adapter: &ClaudeCodeAdapter,
    resume: Option<&str>,
) -> (Proc, Box<dyn DrivenSession>) {
    let driven = adapter.driven().unwrap();
    let launch = DrivenLaunch {
        socket: world.base.join("unused.sock"),
        resume: resume.map(str::to_owned),
        fresh_id: SESSION.into(),
    };
    let spec = driven
        .driven_spec(&world.account, &world.profile, &world.cwd, &launch)
        .unwrap();
    assert_eq!(spec.transport, Transport::Stdio);
    assert!(!juggler_adapter::runtime::has_forbidden_arg(
        &spec.launch.args
    ));
    assert!(spec.launch.args.contains(&"--verbose".to_owned()));
    assert!(
        spec.launch
            .args
            .windows(2)
            .any(|w| w[0] == "--permission-prompt-tool" && w[1] == "stdio")
    );
    assert_eq!(
        spec.launch.env.get("CLAUDE_CONFIG_DIR").map(PathBuf::from),
        Some(world.account.state_dir.clone())
    );
    let mut child = Command::new(&spec.launch.program)
        .args(&spec.launch.args)
        .envs(
            spec.launch
                .env
                .iter()
                .filter(|(_, v)| *v != juggler_adapter::launch::ENV_UNSET),
        )
        .current_dir(&spec.launch.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let io = Connection {
        reader: Box::new(BufReader::new(child.stdout.take().unwrap())),
        writer: Box::new(child.stdin.take().unwrap()),
    };
    let session = driven.connect(io, &spec, &world.account).unwrap();
    (Proc(child), session)
}

fn until(
    session: &mut dyn DrivenSession,
    stop: impl Fn(&RuntimeEvent) -> bool,
) -> Vec<RuntimeEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        if let Some(event) = session.next_event(Duration::from_millis(100)).unwrap() {
            let done = stop(&event);
            seen.push(event);
            if done {
                return seen;
            }
        }
    }
    panic!("no matching event; saw {seen:?}");
}

fn ended(event: &RuntimeEvent) -> bool {
    matches!(event, RuntimeEvent::TurnEnded { .. })
}

fn said(events: &[RuntimeEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            RuntimeEvent::Item { item } => Some(item.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn a_headless_claude_session_is_driven_end_to_end() {
    let world = world(true);
    let adapter = ClaudeCodeAdapter::with_bin(env!("CARGO_BIN_EXE_fake-claude"));
    let driven = adapter.driven().unwrap();
    assert_eq!(driven.maturity(), Maturity::Beta);
    assert_eq!(
        driven.default_runtime(),
        Runtime::Terminal,
        "never the default"
    );
    assert_eq!(driven.unavailable(), None);
    let (proc, mut session) = start(&world, &adapter, None);
    until(
        session.as_mut(),
        |e| matches!(e, RuntimeEvent::Started { native_id, .. } if native_id == SESSION),
    );

    session.send_turn(TurnInput::from_user("hello")).unwrap();
    let events = until(session.as_mut(), ended);
    assert!(said(&events).contains("done"));
    assert!(events.iter().any(|e| matches!(e, RuntimeEvent::LimitsUpdated { limits } if limits.windows[0].id == "five_hour")));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::Transcript { .. })),
        "its file, once written"
    );

    session
        .send_turn(TurnInput::from_user("ask: rm -rf build"))
        .unwrap();
    let events = until(session.as_mut(), |e| {
        matches!(e, RuntimeEvent::ApprovalRequested { .. })
    });
    let Some(RuntimeEvent::ApprovalRequested { approval }) = events.last() else {
        unreachable!()
    };
    assert!(
        approval
            .options
            .iter()
            .any(|o| o.label.starts_with("Always allow Bash(rm:*)"))
    );
    session
        .answer(
            &approval.id,
            AskAnswer::Choose {
                option: "rule:0".into(),
            },
        )
        .unwrap();
    let events = until(session.as_mut(), ended);
    assert!(said(&events).contains("ran it and kept the rule"));

    session
        .send_turn(TurnInput::from_user("question: which colour?"))
        .unwrap();
    let events = until(session.as_mut(), |e| {
        matches!(e, RuntimeEvent::QuestionAsked { .. })
    });
    let Some(RuntimeEvent::QuestionAsked { question }) = events.last() else {
        unreachable!()
    };
    session
        .answer(
            &question.id,
            AskAnswer::Reply {
                parts: vec![PartAnswer {
                    part: question.parts[0].id.clone(),
                    answers: vec!["Blue".into()],
                }],
            },
        )
        .unwrap();
    let events = until(session.as_mut(), ended);
    assert!(said(&events).contains("Blue"));

    session
        .send_turn(TurnInput::from_user("slow: think"))
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    session.interrupt().unwrap();
    let events = until(session.as_mut(), ended);
    assert!(matches!(
        events.last(),
        Some(RuntimeEvent::TurnEnded {
            outcome: TurnEnd::Interrupted,
            ..
        })
    ));
    assert!(!events.iter().any(|e| matches!(
        e,
        RuntimeEvent::Error {
            category: ErrorCategory::RateLimit,
            ..
        }
    )));

    session.set_model("claude-fake-2").unwrap();
    until(
        session.as_mut(),
        |e| matches!(e, RuntimeEvent::ModelChanged { model } if model == "claude-fake-2"),
    );
    let outcome = session
        .release(Release::AfterTurn(Duration::from_secs(1)))
        .unwrap();
    assert!(outcome.at_boundary);
    drop(proc);

    // The conversation is Claude Code's own file: found, and resumed in
    // the terminal (the hand-over), or headless again.
    let found = adapter
        .locate_transcript(&world.account.state_dir, &world.cwd, 0, &[])
        .expect("found like any session");
    assert_eq!(found.native_id.as_deref(), Some(SESSION));
    let view = driven
        .terminal_view(
            &world.account,
            &world.profile,
            &world.cwd,
            &ViewOf {
                native_id: SESSION.into(),
                socket: None,
            },
        )
        .unwrap();
    let TerminalView::AfterRelease(spec) = view else {
        panic!("one writer at a time")
    };
    assert!(
        spec.args
            .windows(2)
            .any(|w| w[0] == "--resume" && w[1] == SESSION)
    );
    assert!(spec.needs_pty);
    let (_proc, mut again) = start(&world, &adapter, Some(SESSION));
    again.send_turn(TurnInput::from_user("again")).unwrap();
    until(again.as_mut(), ended);
    let text = std::fs::read_to_string(&found.path).unwrap();
    assert!(text.contains("\"hello\"") && text.contains("\"again\""));
    assert!(
        !text.contains("AI Juggler"),
        "no product text in the conversation"
    );
}

#[test]
fn only_in_a_trusted_folder_and_a_new_enough_claude_code() {
    let untrusted = world(false);
    let adapter = ClaudeCodeAdapter::with_bin(env!("CARGO_BIN_EXE_fake-claude"));
    let launch = DrivenLaunch {
        socket: untrusted.base.join("x.sock"),
        resume: None,
        fresh_id: SESSION.into(),
    };
    let refused = adapter
        .driven()
        .unwrap()
        .driven_spec(
            &untrusted.account,
            &untrusted.profile,
            &untrusted.cwd,
            &launch,
        )
        .unwrap_err()
        .to_string();
    assert!(refused.contains("trusted"), "{refused}");
    let old = untrusted.base.join("old-claude");
    std::fs::write(
        &old,
        format!(
            "#!/bin/sh\nFAKE_CLAUDE_VERSION=2.1.200 exec {:?} \"$@\"\n",
            env!("CARGO_BIN_EXE_fake-claude")
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o755)).unwrap();
    let why = ClaudeCodeAdapter::with_bin(&old)
        .driven()
        .unwrap()
        .unavailable()
        .unwrap();
    assert!(why.contains("older than 2.1.302"), "{why}");
    // The trust check keeps only the trust flag: the state file's other
    // values (an email here) are skipped while parsing.
    let trusted = world(true);
    assert!(juggler_adapters::claude_stream::trusted(
        &trusted.account.state_dir,
        &trusted.cwd
    ));
    let _ = Value::Null;
}

#[test]
fn the_fake_refuses_what_claude_code_refuses() {
    // The lab's finding, kept true by the fake: stream-json needs --verbose.
    let out = Command::new(env!("CARGO_BIN_EXE_fake-claude"))
        .args([
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("requires --verbose"));
    let terminal = world(true);
    let spec = ClaudeCodeAdapter::with_bin(env!("CARGO_BIN_EXE_fake-claude"))
        .launch_spec(
            &terminal.account,
            &terminal.profile,
            &terminal.cwd,
            &LaunchMode::Headless {
                prompt: "hi".into(),
            },
        )
        .unwrap();
    assert!(
        spec.args.contains(&"--verbose".to_owned()),
        "the headless launch, fixed"
    );
}
