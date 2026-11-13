//! AJ-E1605 R3: a driven `ZCode` session against a stand-in for its
//! engine's app-server (the messages engine 0.16.9's own schemas take and
//! send; every value made up). No `ZCode` runs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test fails loudly on a broken fixture"
)]
#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::time::Duration;

use juggler_adapter::runtime::{
    AskAnswer, Connection, DrivenSession, ErrorCategory, ImportedHistory, OptionEffect, PartAnswer,
    RuntimeEvent, TurnEnd, TurnInput,
};
use juggler_adapters::zcode_appserver::session::{Open, ZCodeSession};
use serde_json::{Value, json};

const SESSION: &str = "sess_0a1b";

/// The stand-in: answers `session/create`/`resume` and `subscribe`,
/// reports every message AI Juggler sends, and writes what the test asks.
struct Engine {
    seen: mpsc::Receiver<Value>,
    say: mpsc::Sender<Value>,
}

fn start(open: Open<'_>) -> (Box<dyn DrivenSession>, Engine) {
    let (ours, theirs) = UnixStream::pair().unwrap();
    let (seen_tx, seen) = mpsc::channel();
    let (say, say_rx) = mpsc::channel::<Value>();
    let mut writer = theirs.try_clone().unwrap();
    std::thread::spawn(move || {
        while let Ok(line) = say_rx.recv() {
            let mut text = line.to_string();
            text.push('\n');
            if writer.write_all(text.as_bytes()).is_err() {
                return;
            }
        }
    });
    let reply = say.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(theirs).lines() {
            let Ok(line) = line else { return };
            let message: Value = serde_json::from_str(&line).unwrap();
            match message.get("method").and_then(Value::as_str) {
                Some("session/create" | "session/resume") => {
                    let id = message
                        .pointer("/params/sessionId")
                        .cloned()
                        .unwrap_or(json!(SESSION));
                    let _ = reply.send(
                        json!({"id": message["id"], "result": {"session": {"sessionId": id}}}),
                    );
                }
                Some("session/subscribe") => {
                    let _ = reply.send(
                        json!({"id": message["id"], "result": {"eventSeq": 0, "events": []}}),
                    );
                }
                Some("session/send") => {
                    let _ = reply.send(json!({"id": message["id"], "result": {"sessionId": SESSION, "accepted": true, "stateRevision": 1}}));
                }
                _ => {}
            }
            let _ = seen_tx.send(message);
        }
    });
    let io = Connection {
        reader: Box::new(BufReader::new(ours.try_clone().unwrap())),
        writer: Box::new(ours),
    };
    let cwd = "/work/cart";
    let session = ZCodeSession::connect(io, cwd, open).unwrap();
    (Box::new(session), Engine { seen, say })
}

impl Engine {
    fn next_seen(&self, method: &str) -> Value {
        loop {
            let message = self.seen.recv_timeout(Duration::from_secs(5)).unwrap();
            if message.get("method").and_then(Value::as_str) == Some(method)
                || (method.is_empty() && message.get("method").is_none())
            {
                return message;
            }
        }
    }

    fn event(&self, kind: &str, payload: Value) {
        self.event_of(SESSION, kind, payload);
    }

    fn event_of(&self, session: &str, kind: &str, payload: Value) {
        self.say
            .send(json!({"method": "session/event", "params": {
                "eventId": "e", "sessionId": session, "turnId": "turn_1", "seq": 1,
                "timestamp": 1, "type": kind, "payload": payload}}))
            .unwrap();
    }
}

fn until(
    session: &mut Box<dyn DrivenSession>,
    done: impl Fn(&RuntimeEvent) -> bool,
) -> Vec<RuntimeEvent> {
    let mut out = Vec::new();
    for _ in 0..200 {
        if let Some(event) = session.next_event(Duration::from_millis(50)).unwrap() {
            let stop = done(&event);
            out.push(event);
            if stop {
                return out;
            }
        }
    }
    panic!("never came: {out:#?}");
}

#[test]
fn a_session_opens_subscribes_and_runs_a_turn_with_the_users_answers() {
    let (mut session, engine) = start(Open::New);
    let create = engine.next_seen("session/create");
    assert_eq!(
        create["params"],
        json!({"workspace": {"workspacePath": "/work/cart", "workspaceKey": "/work/cart"}, "persistence": "immediate"}),
        "a new session: no id of ours, no import"
    );
    let subscribe = engine.next_seen("session/subscribe");
    assert_eq!(
        subscribe["params"],
        json!({"sessionId": SESSION, "deliveryKind": "desktop-continuous"})
    );
    assert_eq!(session.native_id().as_deref(), Some(SESSION));
    assert!(until(&mut session, |e| matches!(e, RuntimeEvent::Started { .. })).len() == 1);

    session
        .send_turn(TurnInput::from_user("fix the cart"))
        .unwrap();
    let send = engine.next_seen("session/send");
    assert_eq!(
        send["params"],
        json!({"sessionId": SESSION, "content": "fix the cart"})
    );
    assert!(
        session.send_turn(TurnInput::from_user("again")).is_err(),
        "never two turns at once"
    );

    // Before its model call: headers, answered with no credential.
    engine
        .say
        .send(json!({"id": "server-1", "method": "interaction/requestProviderRuntimeHeaders",
            "params": {"requestId": "h1", "sessionId": SESSION, "providerId": "zai",
                "modelSelection": {"providerId": "zai", "modelId": "GLM-5.3"}, "reason": "model_request",
                "workspace": {"workspacePath": "/work/cart", "workspaceKey": "/work/cart"}}}))
        .unwrap();
    engine.event(
        "turn.started",
        json!({"turnNumber": 1, "input": "fix the cart"}),
    );
    engine.event(
        "part.delta",
        json!({"messageId": "m", "partId": "p", "field": "text", "delta": "Looking"}),
    );
    engine.event(
        "part.delta",
        json!({"messageId": "m", "partId": "p", "field": "reasoning", "delta": "hmm"}),
    );
    // Another session's (a sub-agent's) events aren't this feed.
    engine.event_of(
        "sess_child",
        "part.delta",
        json!({"messageId": "m", "partId": "p", "delta": "child"}),
    );
    let events = until(&mut session, |e| {
        matches!(e, RuntimeEvent::TextDelta { .. })
    });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::TurnStarted { .. }))
    );
    let headers = engine.next_seen("");
    assert_eq!(
        headers,
        json!({"id": "server-1", "result": {"headersApplied": true, "requestAuth": {}}})
    );

    // A permission ask: the engine's own options; nothing answers it but
    // the user, and the answer is that option's own decision.
    let ask = json!({"id": "server-2", "method": "interaction/requestPermission", "params": {
        "requestId": "perm_1", "sessionId": SESSION, "toolCallId": "call_1", "toolName": "Bash",
        "riskLevel": "medium", "reason": "Runs the tests", "input": {"command": "npm test"},
        "options": [
            {"optionId": "allow_once", "kind": "allow_once", "name": "Allow once",
             "response": {"decision": "allow", "reason": "Approved once"}},
            {"optionId": "allow_session", "kind": "allow_always", "name": "Allow for this session",
             "description": "Do not ask again for this tool in this session",
             "response": {"decision": "allow", "reason": "Approved for this session"}},
            {"optionId": "deny", "kind": "deny", "name": "Deny",
             "response": {"decision": "deny", "reason": "Denied"}}]}});
    engine.say.send(ask.clone()).unwrap();
    // Asked again (the engine re-announces): still one ask.
    let mut again = ask;
    again["id"] = json!("server-3");
    engine.say.send(again).unwrap();
    let events = until(&mut session, |e| {
        matches!(e, RuntimeEvent::ApprovalRequested { .. })
    });
    let Some(RuntimeEvent::ApprovalRequested { approval }) = events.last() else {
        panic!()
    };
    assert_eq!(approval.summary, "Bash: npm test");
    assert_eq!(approval.reason.as_deref(), Some("Runs the tests"));
    let effects: Vec<_> = approval.options.iter().map(|o| o.effect.clone()).collect();
    assert_eq!(
        effects,
        [
            OptionEffect::AllowOnce,
            OptionEffect::AllowRule {
                rule: "Do not ask again for this tool in this session".into()
            },
            OptionEffect::Deny,
        ]
    );
    let id = approval.id.clone();
    assert!(
        session
            .answer(
                &id,
                AskAnswer::Choose {
                    option: "always".into()
                }
            )
            .is_err()
    );
    session
        .answer(
            &id,
            AskAnswer::Choose {
                option: "deny".into(),
            },
        )
        .unwrap();
    // Either message of the ask names it (the engine takes both).
    let answer = engine.next_seen("");
    assert!(
        answer["id"] == "server-2" || answer["id"] == "server-3",
        "{answer}"
    );
    assert_eq!(
        answer["result"],
        json!({"decision": "deny", "reason": "Denied"})
    );
    let more = until(&mut session, |e| {
        matches!(e, RuntimeEvent::AskResolved { .. })
    });
    assert!(
        more.iter()
            .all(|e| !matches!(e, RuntimeEvent::ApprovalRequested { .. })),
        "asked once"
    );

    // A question.
    engine
        .say
        .send(json!({"id": "server-4", "method": "interaction/requestUserInput", "params": {
            "requestId": "q_1", "sessionId": SESSION, "toolCallId": "call_2", "toolName": "AskUserQuestion",
            "prompt": "Which?", "input": {}, "questions": [{"header": "DB", "question": "Which database?",
            "multiSelect": false, "options": [{"label": "Postgres", "value": "Postgres"}]}]}}))
        .unwrap();
    let events = until(&mut session, |e| {
        matches!(e, RuntimeEvent::QuestionAsked { .. })
    });
    let Some(RuntimeEvent::QuestionAsked { question }) = events.last() else {
        panic!()
    };
    assert_eq!(question.parts[0].choices[0].label, "Postgres");
    session
        .answer(
            &question.id.clone(),
            AskAnswer::Reply {
                parts: vec![PartAnswer {
                    part: "0".into(),
                    answers: vec!["Postgres".into()],
                }],
            },
        )
        .unwrap();
    assert_eq!(
        engine.next_seen(""),
        json!({"id": "server-4", "result": {"action": "accept", "content": {"answers": {"Which database?": "Postgres"}}}})
    );

    // Anything else the engine asks is refused.
    engine
        .say
        .send(json!({"id": "server-5", "method": "interaction/browserList", "params": {}}))
        .unwrap();
    // The session reads (and refuses) as it is pumped.
    for _ in 0..5 {
        let _ = session.next_event(Duration::from_millis(50)).unwrap();
    }
    let refusal = engine.next_seen("");
    assert_eq!(refusal["id"], "server-5");
    assert!(refusal.get("error").is_some());

    engine.event(
        "turn.completed",
        json!({"response": "done", "tokenCount": 1, "toolCallCount": 0,
        "duration": 1, "resultType": "success"}),
    );
    let events = until(&mut session, |e| {
        matches!(e, RuntimeEvent::TurnEnded { .. })
    });
    assert!(matches!(
        events.last(),
        Some(RuntimeEvent::TurnEnded {
            outcome: TurnEnd::Completed,
            ..
        })
    ));

    // Interrupt stops the running turn.
    session.send_turn(TurnInput::from_user("more")).unwrap();
    engine.next_seen("session/send");
    session.interrupt().unwrap();
    assert_eq!(
        engine.next_seen("session/stop")["params"],
        json!({"sessionId": SESSION})
    );
}

#[test]
fn a_failed_turn_says_a_limit_or_a_sign_in_plainly() {
    let (mut session, engine) = start(Open::Resume(SESSION));
    assert_eq!(
        engine.next_seen("session/resume")["params"]["sessionId"],
        SESSION
    );
    session.send_turn(TurnInput::from_user("go")).unwrap();
    engine.event("turn.failed", json!({"turnPhase": "model", "error": {"type": "ModelRateLimited",
        "message": "[1308][Usage limit reached for 5 hour. Your limit will reset at 2026-10-21 18:30:00][0123456789abcdef0123456789abcd]"}}));
    let events = until(&mut session, |e| {
        matches!(e, RuntimeEvent::TurnEnded { .. })
    });
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

    session.send_turn(TurnInput::from_user("go")).unwrap();
    engine.event(
        "turn.failed",
        json!({"turnPhase": "model", "error": {"type": "AuthFailed",
        "message": "401 Unauthorized"}}),
    );
    let events = until(&mut session, |e| {
        matches!(e, RuntimeEvent::TurnEnded { .. })
    });
    let words = events.iter().find_map(|e| match e {
        RuntimeEvent::Error {
            category: ErrorCategory::SignIn,
            message,
            ..
        } => Some(message.clone()),
        _ => None,
    });
    assert_eq!(
        words.as_deref(),
        Some("ZCode couldn't use this App's sign-in: 401 Unauthorized")
    );
}

#[test]
fn a_pack_comes_in_by_zcodes_own_import() {
    let history = ImportedHistory {
        title: "Handover from Claude Code".into(),
        turns: vec![
            ("user".into(), "Fix the cart".into()),
            (
                "assistant".into(),
                "Done: the cart totals now round.".into(),
            ),
            ("tool".into(), "never imported".into()),
        ],
    };
    let id = "sess_0b1c2d3e-0000-4000-8000-0000000000aa";
    let (session, engine) = start(Open::Import {
        id,
        history: &history,
    });
    let create = engine.next_seen("session/create");
    assert_eq!(
        create["params"],
        json!({
            "sessionId": id,
            "workspace": {"workspacePath": "/work/cart", "workspaceKey": "/work/cart"},
            "persistence": "immediate",
            "titleGenerationEnabled": false,
            "importedHistory": {"source": "claudeCode", "title": "Handover from Claude Code",
                "messages": [{"role": "user", "content": "Fix the cart"},
                    {"role": "assistant", "content": "Done: the cart totals now round."}]}
        })
    );
    assert_eq!(session.native_id().as_deref(), Some(id));
}

/// The live import check (AJ-E1605 R3, evidence): the real engine's
/// app-server, started as the adapter starts it, in a throwaway home
/// (`AI_JUGGLER_ZCODE_LIVE_HOME`: `HOME` and the App's tree there), no
/// sign-in, takes a two-turn conversation by `session/create` with
/// `importedHistory` (no model call: nothing is sent), then the store is
/// read `immutable` and checked. Run by hand only:
/// `AI_JUGGLER_ZCODE_LIVE_HOME=<empty folder> cargo test -p juggler-adapters
/// --test zcode_app_server live_import -- --ignored --nocapture`.
#[test]
#[ignore = "runs the installed ZCode's engine; by hand, in a throwaway home"]
#[allow(
    clippy::disallowed_methods,
    reason = "the check spawns the engine and opens the store it wrote, read-only and immutable"
)]
fn live_import() {
    use juggler_adapter::ToolSignIn as _;
    use juggler_adapter::runtime::{DrivenLaunch, DrivenRuntime as _};
    let home = std::path::PathBuf::from(
        std::env::var_os("AI_JUGGLER_ZCODE_LIVE_HOME").expect("a throwaway home"),
    );
    assert!(
        std::fs::read_dir(&home).unwrap().next().is_none(),
        "the throwaway home must start empty"
    );
    let adapter = juggler_adapters::ZCodeAdapter::new().with_home(&home);
    let app = home.join("acc");
    adapter.prepare_account(&app).unwrap();
    let account = juggler_core::model::Account {
        id: juggler_core::model::AccountId::new("live"),
        harness: juggler_core::model::HarnessId::new("zcode"),
        label: "live".into(),
        plan: None,
        state_dir: app.clone(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    let profile = juggler_core::model::Profile {
        id: juggler_core::model::ProfileId::new("p"),
        name: "p".into(),
        account: account.id.clone(),
        settings_overlay: Default::default(),
        default_cwd: None,
    };
    let work = home.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let history = ImportedHistory {
        title: "Live import check".into(),
        turns: vec![
            ("user".into(), "Make the cart total round to cents.".into()),
            (
                "assistant".into(),
                "Done: totals now round half up to cents.".into(),
            ),
        ],
    };
    let spec = adapter
        .driven_spec(
            &account,
            &profile,
            &work,
            &DrivenLaunch {
                socket: home.join("unused.sock"),
                resume: None,
                fresh_id: "0b1c2d3e-0000-4000-8000-0000000000aa".into(),
                history: Some(history.clone()),
            },
        )
        .unwrap();
    let mut command = std::process::Command::new(&spec.launch.program);
    command
        .args(&spec.launch.args)
        .current_dir(&spec.launch.cwd)
        .env("HOME", &home)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    // Its own process group, as the supervisor spawns every tool: the
    // engine starts plugin hosts (its bundled official plugins) that can
    // outlive it, and the whole group is reaped below.
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    for (key, value) in &spec.launch.env {
        if value == juggler_adapter::launch::ENV_UNSET {
            command.env_remove(key);
        } else {
            command.env(key, value);
        }
    }
    let mut child = command.spawn().unwrap();
    let io = Connection {
        reader: Box::new(BufReader::new(child.stdout.take().unwrap())),
        writer: Box::new(child.stdin.take().unwrap()),
    };
    let id = spec.native_id.clone().unwrap();
    let opened = ZCodeSession::connect(
        io,
        &spec.launch.cwd.to_string_lossy(),
        Open::Import {
            id: &id,
            history: &history,
        },
    );
    let opened = opened.map(|session| session.native_id());
    // Close its stdin: the server ends.
    let mut ended = false;
    for _ in 0..100 {
        if child.try_wait().unwrap().is_some() {
            ended = true;
            break;
        }
        // poll: a bounded wait (10 s) for the server to end on its own.
        std::thread::sleep(Duration::from_millis(100));
    }
    if !ended {
        let _ = child.kill();
        let _ = child.wait();
    }
    println!("server ended by itself: {ended}");
    // Reap the rest of its group (the engine's plugin hosts, its bundled
    // official plugins, are in it), then nothing of the group may remain.
    let pgid = child.id().to_string();
    let group = format!("-{pgid}");
    let _ = std::process::Command::new("/bin/kill")
        .args(["-TERM", "--", &group])
        .status();
    // poll: a bounded wait (5 s) for the group to go.
    let mut left = String::new();
    for _ in 0..50 {
        let out = std::process::Command::new("/usr/bin/pgrep")
            .args(["-g", &pgid])
            .output()
            .unwrap();
        left = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if left.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !left.is_empty() {
        let _ = std::process::Command::new("/bin/kill")
            .args(["-KILL", "--", &group])
            .status();
    }
    assert!(
        left.is_empty(),
        "the engine's process group outlived the run: {left}"
    );
    assert_eq!(opened.unwrap().as_deref(), Some(id.as_str()));

    // The store, read immutable: the session, its two messages and parts.
    let db = juggler_adapters::zcode::db_path_of(&app);
    let conn = rusqlite::Connection::open_with_flags(
        format!("file:{}?immutable=1", db.display()),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();
    let (title, directory): (String, String) = conn
        .query_row(
            "SELECT title, directory FROM session WHERE id = ?1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    println!(
        "session: title {title:?}, directory is the work folder: {}",
        directory == work.to_string_lossy()
    );
    assert_eq!(title, "Live import check");
    let turns = juggler_adapters::zcode_sessions::turns(&db, &id).unwrap();
    let got: Vec<(String, String)> = turns
        .iter()
        .map(|t| (t.role.to_owned(), t.text.clone()))
        .collect();
    println!("turns read back by the adapter: {}", got.len());
    assert_eq!(got, history.turns);
    let count = |sql: &str| -> i64 { conn.query_row(sql, [&id], |row| row.get(0)).unwrap() };
    // The engine marks the user's turn as its import, and writes the
    // assistant's with no cost and no tokens (engine 0.16.9).
    let imported = count(
        "SELECT count(*) FROM message WHERE session_id = ?1
         AND json_extract(data, '$.role') = 'user'
         AND json_extract(data, '$.metadata.migrationSource') = 'claudeCode'",
    );
    let free = count(
        "SELECT count(*) FROM message WHERE session_id = ?1
         AND json_extract(data, '$.role') = 'assistant'
         AND json_extract(data, '$.cost') = 0 AND json_extract(data, '$.tokens.input') = 0
         AND json_extract(data, '$.tokens.output') = 0",
    );
    let texts = count(
        "SELECT count(*) FROM part WHERE session_id = ?1 AND json_extract(data, '$.type') = 'text'",
    );
    println!(
        "user turns marked as ZCode's import: {imported}; assistant turns at no cost: {free}; text parts: {texts}"
    );
    assert_eq!((imported, free, texts), (1, 1, 2));
    let tokens: i64 = conn
        .query_row(
            "SELECT count(*) FROM model_usage WHERE session_id = ?1",
            [&id],
            |row| row.get(0),
        )
        .unwrap();
    println!("model calls recorded for it: {tokens}");
    assert_eq!(tokens, 0, "an import makes no model call");
}
