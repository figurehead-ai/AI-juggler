//! AJ-E1608 R2: Muse Code driven over `muse serve`, against the MIT SDK's
//! own golden transcripts of its session protocol (vendored in
//! `tests/muse_fixture/transcripts/`, see its README).
//!
//! Each transcript is replayed through AI Juggler's codec: every client
//! frame in it is made by the codec's own call (and checked against the
//! transcript's: same method, same session, same choice), and every server
//! frame is fed back. What the user sees is then checked: the turn, its
//! text, its tools, the asks with Muse's own choices, the ends.
//!
//! One live check, `live_echo_session` (ignored by default), drives a real
//! `muse serve --provider echo` (no account, no network, no prompt to
//! Meta) in a throwaway home: `AJ_MUSE_LIVE_BIN` names the binary,
//! `AJ_MUSE_LIVE_HOME` the throwaway home, `AJ_MUSE_LIVE_SANDBOX` an
//! optional `sandbox-exec` profile it runs under.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "test crates are pragmatic by policy (QUALITY.md §2)"
)]

use std::collections::BTreeMap;
use std::path::Path;

use juggler_adapter::runtime::session::LineCodec;
use juggler_adapter::runtime::{
    AskAnswer, AskId, ErrorCategory, OptionEffect, PartAnswer, RuntimeEvent, TurnEnd,
};
use juggler_adapters::muse_serve::codec::Codec;
use serde_json::{Value, json};

fn transcript(name: &str) -> Vec<(bool, Value)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/muse_fixture/transcripts")
        .join(format!("{name}.ndjson"));
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| {
            let frame: Value = serde_json::from_str(line).unwrap();
            let raw: Value = serde_json::from_str(frame["raw"].as_str().unwrap()).unwrap();
            (frame["dir"] == "client", raw)
        })
        .collect()
}

/// How the user answers when the transcript's client does.
#[derive(Default)]
struct Replay {
    codec: Codec,
    events: Vec<RuntimeEvent>,
    /// The transcript's request ids to the codec's own.
    ids: BTreeMap<String, i64>,
}

impl Replay {
    fn key(id: &Value) -> String {
        id.to_string()
    }

    /// Makes the transcript's client frame with the codec's own call, and
    /// checks it says the same.
    fn client(&mut self, frame: &Value) {
        let method = frame.get("method").and_then(Value::as_str);
        let params = frame.get("params").cloned().unwrap_or(Value::Null);
        let ours = match method {
            Some("initialize") => Some(self.codec.initialize()),
            Some("session/start") => {
                Some(self.codec.start(params["workspaceRoot"].as_str().unwrap()))
            }
            Some("session/resume") => {
                Some(self.codec.resume(params["sessionId"].as_str().unwrap()))
            }
            Some("turn/start") => {
                let text = params["input"][0]["text"].as_str().unwrap();
                Some(self.codec.send(text).unwrap())
            }
            Some("turn/interrupt" | "turn/cancel") => self.codec.stop(),
            Some("model/list") => Some(self.codec.models().unwrap()),
            Some("approval/decide") => {
                let ask = AskId(params["approvalId"].as_str().unwrap().to_owned());
                let option = params["choiceId"].as_str().unwrap().to_owned();
                let (line, events) =
                    LineCodec::answer(&mut self.codec, &ask, &AskAnswer::Choose { option })
                        .unwrap();
                assert_eq!(line["params"]["requirementId"], params["requirementId"]);
                self.events.extend(events);
                Some(line)
            }
            Some("userInput/answer") => {
                let ask = AskId(params["userInputId"].as_str().unwrap().to_owned());
                let parts = params["answers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|answer| PartAnswer {
                        part: answer["questionId"].as_str().unwrap().to_owned(),
                        answers: vec![answer["selectedLabel"].as_str().unwrap().to_owned()],
                    })
                    .collect();
                let (line, events) =
                    LineCodec::answer(&mut self.codec, &ask, &AskAnswer::Reply { parts }).unwrap();
                assert_eq!(line["params"]["answers"], params["answers"]);
                self.events.extend(events);
                Some(line)
            }
            // `initialized`, and the acknowledgements the codec already
            // sent as replies (checked there).
            _ => None,
        };
        let Some(ours) = ours else { return };
        assert_eq!(ours["jsonrpc"], "2.0");
        // AI Juggler stops a turn with `turn/interrupt`, the transcript's
        // client with `turn/cancel`: the same envelope (MSP §3.4/§3.5).
        let method = method.map(|method| {
            if method == "turn/cancel" {
                "turn/interrupt"
            } else {
                method
            }
        });
        assert_eq!(ours["method"].as_str(), method, "{ours}");
        for key in [
            "sessionId",
            "approvalId",
            "choiceId",
            "userInputId",
            "workspaceRoot",
        ] {
            // A new session's id is Muse's to name (the transcript's client
            // chose one; AI Juggler leaves it to Muse).
            if key == "sessionId" && method == Some("session/start") {
                continue;
            }
            if let Some(want) = params.get(key) {
                assert_eq!(&ours["params"][key], want, "{key} in {ours}");
            }
        }
        if let Some(command) = ours["params"].get("commandId") {
            let command = command.as_str().unwrap();
            assert_eq!(&command[14..15], "7", "a UUIDv7 commandId: {command}");
        }
        self.ids
            .insert(Self::key(&frame["id"]), ours["id"].as_i64().unwrap());
    }

    fn server(&mut self, frame: &Value, replies: &mut Vec<Value>) {
        let mut frame = frame.clone();
        let is_response = frame.get("method").is_none();
        if is_response {
            let Some(ours) = self.ids.get(&Self::key(&frame["id"])) else {
                return;
            };
            frame["id"] = json!(ours);
        }
        let (events, sent) = self.codec.take_line(&frame.to_string());
        self.events.extend(events);
        replies.extend(sent);
    }

    fn run(name: &str) -> Self {
        Self::run_in(name, None)
    }

    /// A transcript that starts inside an open session (`session`).
    fn run_in(name: &str, session: Option<&str>) -> Self {
        let mut replay = Self::default();
        replay.codec.handshake.session = session.map(str::to_owned);
        let mut replies: Vec<Value> = Vec::new();
        for (client, frame) in transcript(name) {
            if client {
                if frame.get("method").is_none() {
                    // The transcript's acknowledgement of a Muse request:
                    // the codec sent the same.
                    let sent = replies.remove(0);
                    assert_eq!(sent["id"], frame["id"], "{name}");
                    assert_eq!(sent["result"], frame["result"], "{name}");
                    continue;
                }
                replay.client(&frame);
            } else {
                replay.server(&frame, &mut replies);
            }
        }
        assert!(
            replies.is_empty(),
            "{name}: replies the transcript didn't send: {replies:?}"
        );
        replay
    }

    fn text(&self) -> String {
        self.events
            .iter()
            .filter_map(|event| match event {
                RuntimeEvent::TextDelta { text } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn ended(&self) -> Option<TurnEnd> {
        self.events.iter().rev().find_map(|event| match event {
            RuntimeEvent::TurnEnded { outcome, .. } => Some(*outcome),
            _ => None,
        })
    }
}

#[test]
fn a_session_starts_and_names_its_log() {
    let replay = Replay::run("session-start");
    assert_eq!(
        replay.codec.handshake.muse_home.as_deref(),
        Some("/home/fixture/.muse")
    );
    assert!(matches!(
        &replay.events[..],
        [RuntimeEvent::Started { native_id, path: Some(path), .. }]
            if native_id == "0198f0aa-1111-7000-8000-0000000000aa" && path.ends_with("session.jsonl")
    ));
}

#[test]
fn a_turn_streams_its_text_and_reports_its_tokens() {
    let replay = Replay::run("text-run-single-turn");
    assert_eq!(
        replay.text(),
        "All 214 tests pass except two in tbh-agent..."
    );
    let usage = replay.events.iter().find_map(|event| match event {
        RuntimeEvent::TurnEnded { usage, .. } => *usage,
        _ => None,
    });
    let usage = usage.expect("the turn's tokens");
    assert_eq!(
        (
            usage.input,
            usage.output,
            usage.cached_input,
            usage.reasoning
        ),
        (48_210, 1_211, 40_100, 384)
    );
    assert_eq!(replay.ended(), Some(TurnEnd::Completed));
    assert!(!replay.codec.turn_running());
}

#[test]
fn an_approval_is_the_users_ask_with_muses_own_choices() {
    let replay = Replay::run("approval-round-trip");
    let approval = replay
        .events
        .iter()
        .find_map(|event| match event {
            RuntimeEvent::ApprovalRequested { approval } => Some(approval.clone()),
            _ => None,
        })
        .expect("asked once");
    let asked = replay
        .events
        .iter()
        .filter(|event| matches!(event, RuntimeEvent::ApprovalRequested { .. }))
        .count();
    assert_eq!(asked, 1, "the request and its view event are one ask");
    assert_eq!(approval.id.as_str(), "0198f0ac-7777-7000-8000-0000000000e1");
    assert_eq!(approval.summary, "write_file: /home/me/src/proj/Cargo.toml");
    let effects: Vec<&OptionEffect> = approval
        .options
        .iter()
        .map(|option| &option.effect)
        .collect();
    assert_eq!(
        effects,
        [
            &OptionEffect::AllowOnce,
            &OptionEffect::AllowRule {
                rule: "write /home/me/src/proj/Cargo.toml".into()
            },
            &OptionEffect::DenyAndStop
        ]
    );
    let resolved = replay
        .events
        .iter()
        .filter(|event| matches!(event, RuntimeEvent::AskResolved { .. }))
        .count();
    assert_eq!(resolved, 1, "resolved once, by the user's answer");
    assert_eq!(replay.ended(), Some(TurnEnd::Completed));
}

#[test]
fn a_denied_approval_ends_the_turn_as_muse_says() {
    let replay = Replay::run("approval-deny-round-trip");
    assert!(replay.text().contains("approval was denied"));
    assert_eq!(replay.ended(), Some(TurnEnd::Completed));
}

#[test]
fn a_question_on_resume_is_answered_with_an_offered_label() {
    let replay = Replay::run("userinput-answer-round-trip");
    let question = replay
        .events
        .iter()
        .find_map(|event| match event {
            RuntimeEvent::QuestionAsked { question } => Some(question.clone()),
            _ => None,
        })
        .expect("asked");
    assert_eq!(question.parts[0].id, "db_choice");
    assert_eq!(question.parts[0].choices.len(), 2);
    assert!(!question.parts[0].multi);
    assert_eq!(replay.ended(), Some(TurnEnd::Completed));
}

#[test]
fn an_interrupt_ends_the_turn_as_interrupted() {
    let replay = Replay::run("cancel-mid-turn");
    assert!(replay.events.iter().any(|event| matches!(
        event,
        RuntimeEvent::ToolDelta { text, .. } if text.contains("Compiling")
    )));
    assert_eq!(replay.ended(), Some(TurnEnd::Interrupted));
}

#[test]
fn a_failed_turn_says_why_and_whole_text_still_shows() {
    let replay = Replay::run_in(
        "provider-failure-turn",
        Some("018f7294-0000-7000-8000-5f0e73500000"),
    );
    assert!(replay.events.iter().any(|event| matches!(
        event,
        RuntimeEvent::Error { category: ErrorCategory::Other, message, .. } if message.contains("503")
    )));
    assert!(
        replay.text().contains("Two tests fail"),
        "an agent message that came whole is shown"
    );
}

#[test]
fn the_plans_windows_come_from_usage_read_and_its_changes() {
    let mut codec = Codec::default();
    let read = codec.limits_request().unwrap();
    assert_eq!(read["method"], "usage/read");
    let id = read["id"].as_i64().unwrap();
    let answer = json!({"jsonrpc": "2.0", "id": id, "result": {"usage": {
        "observedAtMs": 1_794_589_000_000_i64, "tier": "t",
        "window": {"usedPercent": 104, "resetsAtMs": 1_794_600_000_000_i64, "windowDurationMins": 300},
        "weekly": {"usedPercent": 30, "resetsAtMs": 1_795_000_000_000_i64}}}});
    let (events, _) = codec.take_line(&answer.to_string());
    let limits = codec.limits().expect("windows");
    assert!(matches!(&events[..], [RuntimeEvent::LimitsUpdated { .. }]));
    assert_eq!(limits.windows[0].id, "five_hour");
    assert_eq!(limits.windows[1].id, "seven_day");
    assert!(limits.reached, "over 100 is reached, and kept as reported");
    assert!((limits.windows[0].used_pct - 104.0).abs() < f64::EPSILON);
    // Nothing seen yet: no windows invented.
    let mut fresh = Codec::default();
    let read = fresh.limits_request().unwrap();
    let empty = json!({"jsonrpc": "2.0", "id": read["id"], "result": {}});
    assert!(fresh.take_line(&empty.to_string()).0.is_empty());
    assert!(fresh.limits().is_none());
}

#[test]
fn a_session_takes_its_own_mcp_servers_and_model() {
    let mut codec = Codec::default();
    codec.model = Some("muse-spark-1.3".into());
    codec.mcp_servers.insert(
        "files".into(),
        json!({"transport": "stdio", "command": "files-mcp", "mode": "optional"}),
    );
    let start = codec.start("/w");
    assert_eq!(start["params"]["modelId"], "muse-spark-1.3");
    assert_eq!(
        start["params"]["config"]["mcpServers"]["files"]["command"],
        "files-mcp"
    );
}

#[test]
fn an_unknown_request_is_refused_and_a_stray_line_counted() {
    let mut codec = Codec::default();
    let (events, replies) = codec
        .take_line(r#"{"jsonrpc":"2.0","id":9,"method":"computerUseSetup/start","params":{}}"#);
    assert!(events.is_empty());
    assert_eq!(replies[0]["error"]["code"], -32601);
    codec.take_line("not json");
    assert_eq!(codec.unknown, 1);
}

/// A real `muse serve --provider echo` (no account, no network, no prompt
/// to Meta) in a throwaway home, through the same `connect` the app uses.
#[test]
#[ignore = "needs an installed Muse Code: AJ_MUSE_LIVE_BIN and AJ_MUSE_LIVE_HOME"]
fn live_echo_session() {
    use juggler_adapter::runtime::{Connection, DrivenSession, Release, TurnInput};
    use std::time::Duration;
    let bin = std::env::var("AJ_MUSE_LIVE_BIN").expect("AJ_MUSE_LIVE_BIN");
    let home =
        std::path::PathBuf::from(std::env::var("AJ_MUSE_LIVE_HOME").expect("AJ_MUSE_LIVE_HOME"));
    let project = home.join("proj");
    for dir in [
        &project,
        &home.join(".ai-juggler/no-codex"),
        &home.join(".ai-juggler/no-claude"),
    ] {
        std::fs::create_dir_all(dir).unwrap();
    }
    // Under `sandbox-exec` with AJ_MUSE_LIVE_SANDBOX's profile when set
    // (no network, no Keychain, no write outside the throwaway home).
    let mut command = match std::env::var("AJ_MUSE_LIVE_SANDBOX") {
        Ok(profile) => {
            let mut command = std::process::Command::new("/usr/bin/sandbox-exec");
            command.args(["-f", &profile, &bin]);
            command
        }
        Err(_) => std::process::Command::new(&bin),
    };
    let mut child = command
        .args(["serve", "--provider", "echo"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("CODEX_HOME", home.join(".ai-juggler/no-codex"))
        .env("CLAUDE_CONFIG_DIR", home.join(".ai-juggler/no-claude"))
        .env("MUSE_NO_AUTO_UPDATE", "1")
        .env("TBH_DISABLE_TELEMETRY", "1")
        .current_dir(&project)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let io = Connection {
        reader: Box::new(std::io::BufReader::new(child.stdout.take().unwrap())),
        writer: Box::new(child.stdin.take().unwrap()),
    };
    let muse_home = home.join(".local/share/muse");
    let project_text = project.to_string_lossy().into_owned();
    let open = juggler_adapters::muse_serve::session::Open {
        muse_home: &muse_home,
        cwd: &project_text,
        resume: None,
        model: None,
    };
    let mut session = juggler_adapters::muse_serve::session::connect(io, &open).unwrap();
    let id = session.native_id().expect("Muse named the session");
    session
        .send_turn(TurnInput::from_user("hello from the kit"))
        .unwrap();
    let mut text = String::new();
    let mut ended = None;
    for _ in 0..200 {
        match session.next_event(Duration::from_millis(200)).unwrap() {
            Some(RuntimeEvent::TextDelta { text: delta }) => text.push_str(&delta),
            Some(RuntimeEvent::TurnEnded { outcome, .. }) => {
                ended = Some(outcome);
                break;
            }
            _ => {}
        }
    }
    assert_eq!(ended, Some(TurnEnd::Completed));
    assert!(text.contains("echo: hello from the kit"), "{text}");
    // The plan's windows, as Muse last saw them: none without an account.
    assert_eq!(session.limits().unwrap(), None);
    let released = Box::new(session).release(Release::Now).unwrap();
    assert_eq!(released.native_id.as_deref(), Some(id.as_str()));
    let _ = child.kill();
    let _ = child.wait();
    // The session is in Muse's own index, read the way the app reads it.
    let index = juggler_adapters::muse_sessions::index_of(&muse_home);
    let listed = juggler_adapters::muse_sessions::sessions(&index).unwrap();
    assert!(listed.iter().any(|session| session.id == id), "{listed:?}");
}
