//! AJ-E2303: Codex driven through app-server.
//!
//! - The recorded 0.163.0 surface (`fixtures/codex-app-server/`) has every
//!   method and answer value the codec uses, and every field it must send.
//! - The recorded messages, fed to the codec, make the right events, and
//!   answers go back in Codex's shapes, only with options Codex offered.
//! - End to end against `fake-codex`: the server on a socket, the handshake
//!   (an honest client name, the profile's `codexHome` or a refusal), turns,
//!   approvals, a question, steer, interrupt, compact, models, limits, a
//!   usage-limit failure, release, and the thread found and resumed.
//! - The pinned range: too old, or a schema without a method, means the
//!   terminal, with the reason.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::BTreeMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use juggler_adapter::runtime::{
    AskAnswer, Connection, DrivenLaunch, DrivenSession, ErrorCategory, ItemKind, OptionEffect,
    PartAnswer, Release, RuntimeEvent, TerminalView, Transport, TurnEnd, TurnInput, ViewOf,
};
use juggler_adapter::{HarnessAdapter, LaunchMode};
use juggler_adapters::CodexAdapter;
use juggler_adapters::codex_appserver::codec::Codec;
use juggler_adapters::codex_appserver::rpc;
use juggler_adapters::codex_appserver::surface::{Kind, SURFACE};
use juggler_core::model::{Account, AccountId, Profile, ProfileId, SettingsOverlay};
use serde_json::{Value, json};

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/codex-app-server"
);

fn surface() -> Value {
    let text = std::fs::read_to_string(format!("{FIXTURES}/surface-0.163.0.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn recorded(surface: &Value, method: &str) -> Option<Value> {
    surface["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["method"] == method)
        .cloned()
}

#[test]
fn the_recorded_surface_has_everything_the_codec_uses() {
    let surface = surface();
    for (kind, method) in SURFACE {
        let want = match kind {
            Kind::ClientRequest => "client_request",
            Kind::ServerRequest => "server_request",
            Kind::Notification => "notification",
        };
        let found = recorded(&surface, method)
            .unwrap_or_else(|| panic!("{method} isn't in Codex 0.163.0's schema"));
        assert_eq!(found["kind"], want, "{method} is a {}", found["kind"]);
    }
    let values = |name: &str| -> Vec<String> {
        surface["enums"][name]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };
    for decision in [
        "accept",
        "acceptForSession",
        "acceptWithExecpolicyAmendment",
        "applyNetworkPolicyAmendment",
        "decline",
        "cancel",
    ] {
        assert!(values("CommandExecutionApprovalDecision").contains(&decision.to_owned()));
    }
    for decision in ["accept", "acceptForSession", "decline", "cancel"] {
        assert!(values("FileChangeApprovalDecision").contains(&decision.to_owned()));
    }
    for scope in ["turn", "session"] {
        assert!(values("PermissionGrantScope").contains(&scope.to_owned()));
    }
    for action in ["decline", "cancel"] {
        assert!(values("McpServerElicitationAction").contains(&action.to_owned()));
    }
}

#[test]
fn every_request_the_codec_sends_has_its_required_fields() {
    let surface = surface();
    let mut codec = Codec::default();
    let mut sent = vec![
        codec.initialize(),
        codec.thread("/kit/project", None, Some("gpt-5.5-codex")),
        codec.thread(
            "/kit/project",
            Some("0199aa00-0000-7000-8000-000000000001"),
            None,
        ),
        codec.rate_limits(),
        codec.models(),
    ];
    // A thread and a running turn, for the requests that need them.
    for line in [
        r#"{"id":2,"result":{"thread":{"id":"t-1"}}}"#,
        r#"{"method":"turn/started","params":{"threadId":"t-1","turn":{"id":"u-1"}}}"#,
    ] {
        codec.incoming(rpc::classify(line).unwrap());
    }
    sent.push(codec.steer(&TurnInput::from_user("also")).unwrap());
    sent.push(codec.interrupt().unwrap());
    sent.push(codec.compact().unwrap());
    codec.incoming(
        rpc::classify(r#"{"method":"turn/completed","params":{"threadId":"t-1","turn":{"id":"u-1","status":"completed"}}}"#)
            .unwrap(),
    );
    sent.push(codec.turn_start(TurnInput::from_user("hello")).unwrap());
    for line in &sent {
        let method = line["method"].as_str().unwrap();
        let found = recorded(&surface, method).unwrap();
        assert_eq!(found["kind"], "client_request", "{method}");
        assert!(
            line.get("jsonrpc").is_none(),
            "app-server's lines carry no jsonrpc field"
        );
        for field in found["required"].as_array().unwrap() {
            let field = field.as_str().unwrap();
            assert!(
                line["params"].get(field).is_some(),
                "{method} lacks its required {field}: {line}"
            );
        }
    }
    let init = &sent[0]["params"];
    assert_eq!(init["clientInfo"]["name"], "ai_juggler");
    assert_eq!(init["clientInfo"]["title"], "AI Juggler");
    assert_eq!(init["capabilities"]["experimentalApi"], false);
}

#[test]
fn recorded_messages_make_the_right_events_and_answers() {
    let surface = surface();
    let text = std::fs::read_to_string(format!("{FIXTURES}/messages-0.163.0.jsonl")).unwrap();
    let mut codec = Codec::default();
    codec.initialize();
    codec.thread("/kit/project", None, None);
    codec.rate_limits();
    let mut events = Vec::new();
    let mut refusals = Vec::new();
    for line in text.lines() {
        let message: Value = serde_json::from_str(line).unwrap();
        if let Some(method) = message["method"].as_str()
            && method != "skills/changed"
            && method != "item/tool/call"
            && method != "thread/started"
        {
            let found = recorded(&surface, method)
                .unwrap_or_else(|| panic!("the fixture's {method} isn't in the schema"));
            for field in found["required"].as_array().unwrap() {
                let field = field.as_str().unwrap();
                assert!(
                    message["params"].get(field).is_some(),
                    "the fixture's {method} lacks {field}"
                );
            }
        }
        let (more, replies) = codec.incoming(rpc::classify(line).unwrap());
        events.extend(more);
        refusals.extend(replies);
    }
    assert_eq!(
        codec.handshake.codex_home.as_deref(),
        Some("/kit/codex-home")
    );
    assert!(
        matches!(&events[0], RuntimeEvent::Started { native_id, model, .. }
        if native_id == "0199aa00-0000-7000-8000-000000000001" && model.as_deref() == Some("gpt-5.5-codex"))
    );
    let limits = events
        .iter()
        .find_map(|e| match e {
            RuntimeEvent::LimitsUpdated { limits } => Some(limits.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(limits.windows.len(), 2);
    assert_eq!(limits.windows[0].window_mins, Some(300));
    assert_eq!(limits.windows[0].resets_at_ms, Some(1_761_397_200_000));
    // The lab's shape: only the weekly window, as `primary`. No 5-hour
    // window is made up.
    let weekly_only = events
        .iter()
        .rev()
        .find_map(|e| match e {
            RuntimeEvent::LimitsUpdated { limits } => Some(limits.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(weekly_only.windows.len(), 1);
    assert_eq!(weekly_only.windows[0].id, "seven_day");
    assert_eq!(weekly_only.windows[0].window_mins, Some(10_080));
    assert_eq!(limits.plan.as_deref(), Some("plus"));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::ApprovalRequested { approval }
        if approval.summary == "npm test"
            && approval.options.iter().any(|o| o.label == "Always allow `npm test`")))
    );
    // Lab 2: the live shape wraps the command in the shell; the ask shows
    // Codex's own parsed commands, all of them, and only its decisions.
    let wrapped = events
        .iter()
        .find_map(|e| match e {
            RuntimeEvent::ApprovalRequested { approval } if approval.id.0 == "4" => {
                Some(approval.clone())
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(wrapped.summary, "ls · pwd");
    let ids: Vec<&str> = wrapped.options.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, ["accept", "acceptWithExecpolicyAmendment", "cancel"]);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::AskResolved { id } if id.0 == "0"))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::ToolDelta { text, .. } if text == "12 passing\n"))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::Item { item } if item.text == "npm test"))
    );
    // A command is named when it starts (its item may never complete).
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::ItemStarted { item, summary }
        if item == "call_1" && summary.text == "npm test" && summary.kind == ItemKind::Command))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::QuestionAsked { question }
        if question.parts[0].free_text && question.parts[0].choices.len() == 2))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::Usage { total } if total.input == 5200))
    );
    assert!(events.iter().any(|e| matches!(e, RuntimeEvent::TurnEnded { outcome: TurnEnd::Completed, usage: Some(u) } if u.input == 1200)));
    assert!(events.iter().any(|e| matches!(
        e,
        RuntimeEvent::Error {
            category: ErrorCategory::RateLimit,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(e, RuntimeEvent::Compacted)));
    // A request AI Juggler doesn't handle is refused, never answered for
    // the user; an unknown notification is counted.
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0]["id"], 3);
    assert!(refusals[0].get("error").is_some());
    assert!(codec.unknown >= 2);
}

#[test]
fn answers_are_codexs_shapes_and_only_to_open_asks() {
    let mut codec = Codec::default();
    let ask = r#"{"id":5,"method":"item/commandExecution/requestApproval","params":{"threadId":"t","turnId":"u","itemId":"i","startedAtMs":1,"command":"rm -rf build"}}"#;
    let (events, _) = codec.incoming(rpc::classify(ask).unwrap());
    let RuntimeEvent::ApprovalRequested { approval } = &events[0] else {
        unreachable!()
    };
    assert!(
        !approval
            .options
            .iter()
            .any(|o| o.id == "acceptWithExecpolicyAmendment"),
        "no rule Codex didn't propose"
    );
    assert!(matches!(
        approval.options.last().unwrap().effect,
        OptionEffect::DenyAndStop
    ));
    let id = approval.id.clone();
    let choose = |option: &str| AskAnswer::Choose {
        option: option.into(),
    };
    assert!(
        codec
            .answer(&id, &choose("acceptWithExecpolicyAmendment"))
            .is_err()
    );
    let (line, events) = codec.answer(&id, &choose("acceptForSession")).unwrap();
    assert_eq!(
        line,
        json!({"id": 5, "result": {"decision": "acceptForSession"}})
    );
    assert!(matches!(&events[0], RuntimeEvent::AskResolved { .. }));
    assert!(codec.answer(&id, &choose("accept")).is_err(), "never twice");
    let question = r#"{"id":"q","method":"item/tool/requestUserInput","params":{"threadId":"t","turnId":"u","itemId":"i","isBlocking":true,"questions":[{"id":"db","header":"DB","question":"Which?","options":null}]}}"#;
    let (events, _) = codec.incoming(rpc::classify(question).unwrap());
    let RuntimeEvent::QuestionAsked { question } = &events[0] else {
        unreachable!()
    };
    assert!(
        codec.answer(&question.id, &choose("accept")).is_err(),
        "a question takes a reply"
    );
    let reply = AskAnswer::Reply {
        parts: vec![PartAnswer {
            part: "db".into(),
            answers: vec!["Postgres".into()],
        }],
    };
    let (line, _) = codec.answer(&question.id, &reply).unwrap();
    assert_eq!(
        line,
        json!({"id": "q", "result": {"answers": {"db": {"answers": ["Postgres"]}}}})
    );
}

// ---------------------------------------------------------------------------
// End to end against fake-codex

struct World {
    _root: tempfile::TempDir,
    base: PathBuf,
    account: Account,
    profile: Profile,
    cwd: PathBuf,
}

fn world() -> World {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let state_dir = base.join("codex-home");
    let cwd = base.join("project");
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&state_dir).unwrap();
    let account = Account {
        id: AccountId::new("codex-acct"),
        harness: juggler_core::model::HarnessId::new("codex"),
        label: "Codex".into(),
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
            model: Some("gpt-fake".into()),
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

/// `fake-codex` behind a script that sets `env` (tests can't set their own
/// environment safely).
fn fake_with(world: &World, name: &str, env: &str) -> PathBuf {
    let path = world.base.join(name);
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\n{env} exec {:?} \"$@\"\n",
            env!("CARGO_BIN_EXE_fake-codex")
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start(
    world: &World,
    adapter: &CodexAdapter,
    resume: Option<&str>,
) -> (
    Server,
    Result<Box<dyn DrivenSession>, juggler_adapter::AdapterError>,
) {
    let driven = adapter.driven().expect("Codex is driven");
    let launch = DrivenLaunch {
        socket: world.base.join("unused.sock"),
        resume: resume.map(str::to_owned),
        fresh_id: "unused".into(),
    };
    let spec = driven
        .driven_spec(&world.account, &world.profile, &world.cwd, &launch)
        .unwrap();
    assert_eq!(
        spec.transport,
        Transport::Stdio,
        "one server per session, over stdio"
    );
    assert_eq!(spec.launch.args, ["app-server", "--listen", "stdio://"]);
    assert_eq!(
        spec.launch.env.get("CODEX_HOME").map(PathBuf::from),
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
    let server = Server(child);
    (server, driven.connect(io, &spec, &world.account))
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

fn approval(events: &[RuntimeEvent]) -> juggler_adapter::runtime::Approval {
    events
        .iter()
        .find_map(|e| match e {
            RuntimeEvent::ApprovalRequested { approval } => Some(approval.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn a_codex_session_is_driven_end_to_end_through_its_app_server() {
    let world = world();
    let adapter = CodexAdapter::with_bin(env!("CARGO_BIN_EXE_fake-codex")).with_home(&world.base);
    let (server, session) = start(&world, &adapter, None);
    let mut session = session.unwrap();
    let thread = session.native_id().expect("the thread started");
    // The first limits read, before any turn: windows with resets.
    let events = until(session.as_mut(), |e| {
        matches!(e, RuntimeEvent::LimitsUpdated { .. })
    });
    assert!(events.iter().any(
        |e| matches!(e, RuntimeEvent::Started { model, .. } if model.as_deref() == Some("gpt-fake"))
    ));
    let RuntimeEvent::LimitsUpdated { limits } = events.last().unwrap() else {
        unreachable!()
    };
    // Named by length, shortest first, whatever primary/secondary held.
    assert_eq!(limits.windows[0].id, "five_hour");
    assert_eq!(limits.windows[1].id, "seven_day");
    assert_eq!(limits.windows[1].window_mins, Some(10_080));
    assert!(limits.windows[0].resets_at_ms.is_some());

    // A plain turn.
    session.send_turn(TurnInput::from_user("hello")).unwrap();
    let events = until(session.as_mut(), ended);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::TextDelta { text } if text == "done"))
    );
    assert!(
        matches!(events.last(), Some(RuntimeEvent::TurnEnded { outcome: TurnEnd::Completed, usage: Some(u) }) if u.input == 100)
    );

    // An approval with Codex's proposed rule; the rule chosen goes back.
    session
        .send_turn(TurnInput::from_user("ask: rm -rf build"))
        .unwrap();
    let events = until(session.as_mut(), |e| {
        matches!(e, RuntimeEvent::ApprovalRequested { .. })
    });
    let asked = approval(&events);
    assert_eq!(asked.summary, "rm -rf build");
    // Steer into the running turn while it waits.
    session
        .steer(TurnInput::from_user("and the cache"))
        .unwrap();
    until(
        session.as_mut(),
        |e| matches!(e, RuntimeEvent::TextDelta { text } if text == "steered: and the cache"),
    );
    assert!(
        session.send_turn(TurnInput::from_user("no")).is_err(),
        "only between turns"
    );
    session
        .answer(
            &asked.id,
            AskAnswer::Choose {
                option: "acceptWithExecpolicyAmendment".into(),
            },
        )
        .unwrap();
    let events = until(session.as_mut(), ended);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::TextDelta { text }
        if text.contains(r#""execpolicy_amendment":["rm","-rf"]"#))),
        "{events:?}"
    );

    // A question, replied to.
    session
        .send_turn(TurnInput::from_user("question: which colour?"))
        .unwrap();
    let events = until(session.as_mut(), |e| {
        matches!(e, RuntimeEvent::QuestionAsked { .. })
    });
    let RuntimeEvent::QuestionAsked { question } = events
        .iter()
        .find(|e| matches!(e, RuntimeEvent::QuestionAsked { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    session
        .answer(
            &question.id,
            AskAnswer::Reply {
                parts: vec![PartAnswer {
                    part: "colour".into(),
                    answers: vec!["Blue".into()],
                }],
            },
        )
        .unwrap();
    let events = until(session.as_mut(), ended);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::TextDelta { text } if text.contains("Blue")))
    );

    // Deny and stop ends the turn; interrupt does too.
    session
        .send_turn(TurnInput::from_user("file: README"))
        .unwrap();
    let events = until(session.as_mut(), |e| {
        matches!(e, RuntimeEvent::ApprovalRequested { .. })
    });
    let asked = approval(&events);
    session
        .answer(
            &asked.id,
            AskAnswer::Choose {
                option: "cancel".into(),
            },
        )
        .unwrap();
    let events = until(session.as_mut(), ended);
    assert!(matches!(
        events.last(),
        Some(RuntimeEvent::TurnEnded {
            outcome: TurnEnd::Interrupted,
            ..
        })
    ));
    session
        .send_turn(TurnInput::from_user("ask: sleep 100"))
        .unwrap();
    until(session.as_mut(), |e| {
        matches!(e, RuntimeEvent::ApprovalRequested { .. })
    });
    session.interrupt().unwrap();
    let events = until(session.as_mut(), ended);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::AskResolved { .. }))
    );

    // A usage limit is a RateLimit stop, not a crash.
    session
        .send_turn(TurnInput::from_user("fail: now"))
        .unwrap();
    let events = until(session.as_mut(), ended);
    assert!(events.iter().any(|e| matches!(
        e,
        RuntimeEvent::Error {
            category: ErrorCategory::RateLimit,
            ..
        }
    )));
    assert!(matches!(
        events.last(),
        Some(RuntimeEvent::TurnEnded {
            outcome: TurnEnd::Failed,
            ..
        })
    ));

    // Compact, models (hidden ones left out), the model for the next turn.
    session.compact().unwrap();
    until(session.as_mut(), |e| matches!(e, RuntimeEvent::Compacted));
    session.list_models().unwrap();
    let events = until(session.as_mut(), |e| {
        matches!(e, RuntimeEvent::Models { .. })
    });
    assert!(
        matches!(events.last(), Some(RuntimeEvent::Models { models }) if models.len() == 1 && models[0].default)
    );
    session.set_model("gpt-fake-mini").unwrap();
    until(
        session.as_mut(),
        |e| matches!(e, RuntimeEvent::ModelChanged { model } if model == "gpt-fake-mini"),
    );

    let outcome = session
        .release(Release::AfterTurn(Duration::from_secs(1)))
        .unwrap();
    assert!(outcome.at_boundary);
    assert_eq!(outcome.native_id.as_deref(), Some(thread.as_str()));
    drop(server);

    // The thread is a rollout like any other: found, and resumed in either
    // runtime.
    let found = adapter
        .locate_transcript(&world.account.state_dir, &world.cwd, 0, &[])
        .expect("the driven thread is found");
    assert_eq!(found.native_id.as_deref(), Some(thread.as_str()));
    let spec = adapter
        .launch_spec(
            &world.account,
            &world.profile,
            &world.cwd,
            &LaunchMode::Resume {
                session_id: thread.clone(),
                delta_prompt: None,
            },
        )
        .unwrap();
    assert!(spec.args.contains(&thread) && spec.args.contains(&"--no-daemon".to_owned()));
    let (_server, again) = start(&world, &adapter, Some(&thread));
    let mut again = again.unwrap();
    assert_eq!(again.native_id().as_deref(), Some(thread.as_str()));
    again.send_turn(TurnInput::from_user("second")).unwrap();
    until(again.as_mut(), ended);
    let rollout = std::fs::read_to_string(&found.path).unwrap();
    assert!(rollout.contains("hello") && rollout.contains("second"));
}

#[test]
fn the_tui_takes_over_the_same_thread_at_a_turn_boundary() {
    let world = world();
    let adapter = CodexAdapter::with_bin(env!("CARGO_BIN_EXE_fake-codex")).with_home(&world.base);
    let view = adapter
        .driven()
        .unwrap()
        .terminal_view(
            &world.account,
            &world.profile,
            &world.cwd,
            &ViewOf {
                native_id: "0199aa00-0000-7000-8000-000000000001".into(),
                socket: Some(world.base.join("ignored.sock")),
            },
        )
        .unwrap();
    // Over stdio there is no socket to attach to: the terminal resumes the
    // thread once the driver let go (ADR-0022, the lab of 2026-10-25).
    let TerminalView::AfterRelease(spec) = view else {
        panic!("a hand-over, never two writers")
    };
    assert!(spec.args.contains(&"resume".to_owned()));
    assert!(
        spec.args
            .contains(&"0199aa00-0000-7000-8000-000000000001".to_owned())
    );
    assert!(spec.args.contains(&"--no-daemon".to_owned()));
    assert!(spec.needs_pty);
    assert_eq!(
        spec.env.get("CODEX_HOME").map(PathBuf::from),
        Some(world.account.state_dir.clone())
    );
}

#[test]
fn a_server_on_another_home_is_refused() {
    let world = world();
    let bin = fake_with(&world, "codex-elsewhere", "FAKE_CODEX_HOME=/somewhere/else");
    let adapter = CodexAdapter::with_bin(bin).with_home(&world.base);
    let (_server, session) = start(&world, &adapter, None);
    let error = session.err().expect("refused").to_string();
    assert!(error.contains("/somewhere/else"), "{error}");
}

#[test]
fn resuming_an_unknown_thread_fails_the_handshake() {
    let world = world();
    let adapter = CodexAdapter::with_bin(env!("CARGO_BIN_EXE_fake-codex")).with_home(&world.base);
    let (_server, session) = start(
        &world,
        &adapter,
        Some("0199aa00-0000-7000-8000-00000000dead"),
    );
    assert!(session.is_err());
}

#[test]
fn only_a_pinned_protocol_is_driven() {
    let world = world();
    let check = |name: &str, env: &str| {
        let adapter = CodexAdapter::with_bin(fake_with(&world, name, env)).with_home(&world.base);
        adapter.driven().unwrap().unavailable()
    };
    assert_eq!(check("codex-ok", "FAKE_CODEX_VERSION=0.163.0"), None);
    assert_eq!(check("codex-newer", "FAKE_CODEX_VERSION=0.170.2"), None);
    let old = check("codex-old", "FAKE_CODEX_VERSION=0.150.0").unwrap();
    assert!(old.contains("older than 0.163.0"), "{old}");
    let drift = check(
        "codex-drift",
        "FAKE_CODEX_VERSION=0.171.0 FAKE_CODEX_DROP=turn/steer",
    )
    .unwrap();
    assert!(drift.contains("turn/steer"), "{drift}");
    // The check runs the generator with a scratch home: nothing lands in
    // the account's folder.
    assert!(
        std::fs::read_dir(&world.account.state_dir)
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn the_driven_launch_never_carries_a_bypass_or_an_inherited_key() {
    let world = world();
    let adapter = CodexAdapter::with_bin(env!("CARGO_BIN_EXE_fake-codex")).with_home(&world.base);
    let launch = DrivenLaunch {
        socket: world.base.join("x.sock"),
        resume: None,
        fresh_id: "unused".into(),
    };
    let spec = adapter
        .driven()
        .unwrap()
        .driven_spec(&world.account, &world.profile, &world.cwd, &launch)
        .unwrap();
    assert!(!juggler_adapter::runtime::has_forbidden_arg(
        &spec.launch.args
    ));
    for key in ["OPENAI_API_KEY", "CODEX_API_KEY"] {
        assert_eq!(
            spec.launch.env.get(key).map(String::as_str),
            Some(juggler_adapter::launch::ENV_UNSET)
        );
    }
    assert_eq!(spec.model.as_deref(), Some("gpt-fake"));
    let _ = Path::new("");
}

/// The installed Codex, checked offline (`--version` and the schema
/// generator with a scratch `CODEX_HOME`; no account is touched). Run by
/// hand: `cargo test -p juggler-adapters --test codex_app_server -- --ignored`.
#[test]
#[ignore = "needs Codex installed on this Mac"]
fn the_installed_codex_has_the_pinned_surface() {
    let adapter = CodexAdapter::new();
    assert_eq!(adapter.driven().unwrap().unavailable(), None);
}
