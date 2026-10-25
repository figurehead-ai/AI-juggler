//! AJ-E2302: the driven runtime end to end on the fake tool: the real
//! `fake-harness drive` process, the fake adapter's codec, and the
//! contract's rules (answers only to open asks with offered options, turns
//! only between turns, release at a boundary, the conversation found and
//! resumed like any other).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::io::BufReader;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use juggler_adapter::runtime::{
    AskAnswer, Connection, DrivenLaunch, DrivenSession, OptionEffect, PartAnswer, Release,
    RuntimeEvent, Transport, TurnEnd, TurnInput, TurnOrigin, ViewOf,
};
use juggler_adapter::{HarnessAdapter, LaunchMode};
use juggler_adapters::FakeHarnessAdapter;
use juggler_core::model::{Account, AccountId, Profile, ProfileId, SettingsOverlay};

struct World {
    _root: tempfile::TempDir,
    adapter: FakeHarnessAdapter,
    account: Account,
    profile: Profile,
    cwd: std::path::PathBuf,
}

fn world() -> World {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let state_dir = base.join("account");
    let cwd = base.join("project");
    std::fs::create_dir_all(&cwd).unwrap();
    let adapter = FakeHarnessAdapter::new(env!("CARGO_BIN_EXE_fake-harness"));
    adapter.prepare_account(&state_dir).unwrap();
    let account = Account {
        id: AccountId::new("acct"),
        harness: adapter.id(),
        label: "Fake".into(),
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
        settings_overlay: SettingsOverlay::default(),
        default_cwd: None,
    };
    World {
        _root: root,
        adapter,
        account,
        profile,
        cwd,
    }
}

fn start(world: &World, resume: Option<&str>) -> (Child, Box<dyn DrivenSession>) {
    let driven = world.adapter.driven().expect("the fake drives");
    let launch = DrivenLaunch {
        socket: world.cwd.join("unused.sock"),
        resume: resume.map(str::to_owned),
        fresh_id: "fake-driven-1".into(),
    };
    let spec = driven
        .driven_spec(&world.account, &world.profile, &world.cwd, &launch)
        .unwrap();
    assert_eq!(spec.transport, Transport::Stdio);
    let mut child = Command::new(&spec.launch.program)
        .args(&spec.launch.args)
        .envs(&spec.launch.env)
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
    (child, session)
}

/// Events until `stop` matches one (which is returned last), within 5 s.
fn until(
    session: &mut dyn DrivenSession,
    stop: impl Fn(&RuntimeEvent) -> bool,
) -> Vec<RuntimeEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        if let Some(event) = session.next_event(Duration::from_millis(200)).unwrap() {
            let done = stop(&event);
            seen.push(event);
            if done {
                return seen;
            }
        }
    }
    panic!("no matching event; saw {seen:?}");
}

fn turn_end(event: &RuntimeEvent) -> bool {
    matches!(event, RuntimeEvent::TurnEnded { .. })
}

#[test]
fn a_driven_session_runs_turns_and_carries_only_the_users_answers() {
    let world = world();
    let (mut child, mut session) = start(&world, None);
    let started = until(session.as_mut(), |event| {
        matches!(event, RuntimeEvent::Started { .. })
    });
    assert!(matches!(
        started.last(),
        Some(RuntimeEvent::Started { native_id, .. }) if native_id == "fake-driven-1"
    ));

    // A plain turn: started (with its origin), text, ended with usage.
    session
        .send_turn(TurnInput {
            text: "hello".into(),
            origin: TurnOrigin::Delivered {
                sender: "planner".into(),
            },
        })
        .unwrap();
    let events = until(session.as_mut(), turn_end);
    assert!(events.iter().any(|event| matches!(
        event,
        RuntimeEvent::TurnStarted { origin: Some(TurnOrigin::Delivered { sender }), .. } if sender == "planner"
    )));
    assert!(matches!(
        events.last(),
        Some(RuntimeEvent::TurnEnded { outcome: TurnEnd::Completed, usage: Some(usage) }) if usage.input == 100
    ));

    // An approval: only its offered options, only while it is open.
    session
        .send_turn(TurnInput::from_user("ask: rm -rf build"))
        .unwrap();
    let events = until(session.as_mut(), |event| {
        matches!(event, RuntimeEvent::ApprovalRequested { .. })
    });
    let Some(RuntimeEvent::ApprovalRequested { approval }) = events.last() else {
        unreachable!()
    };
    assert_eq!(approval.summary, "rm -rf build");
    assert!(
        approval
            .options
            .iter()
            .any(|option| matches!(option.effect, OptionEffect::AllowRule { .. }))
    );
    assert!(
        session.send_turn(TurnInput::from_user("again")).is_err(),
        "no new turn while one runs"
    );
    assert!(
        session
            .answer(
                &approval.id,
                AskAnswer::Choose {
                    option: "yes-to-all".into()
                }
            )
            .is_err(),
        "an option the tool didn't offer"
    );
    session
        .answer(
            &approval.id,
            AskAnswer::Choose {
                option: "once".into(),
            },
        )
        .unwrap();
    let events = until(session.as_mut(), turn_end);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, RuntimeEvent::AskResolved { id } if id == &approval.id))
    );
    assert!(events.iter().any(
        |event| matches!(event, RuntimeEvent::TextDelta { text } if text == "you chose once")
    ));
    assert!(
        session
            .answer(
                &approval.id,
                AskAnswer::Choose {
                    option: "once".into()
                }
            )
            .is_err(),
        "an answered ask can't be answered again"
    );

    // A question, answered with a reply.
    session
        .send_turn(TurnInput::from_user("question: which colour?"))
        .unwrap();
    let events = until(session.as_mut(), |event| {
        matches!(event, RuntimeEvent::QuestionAsked { .. })
    });
    let Some(RuntimeEvent::QuestionAsked { question }) = events.last() else {
        unreachable!()
    };
    assert_eq!(question.parts[0].choices.len(), 2);
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
    let events = until(session.as_mut(), turn_end);
    assert!(
        events.iter().any(
            |event| matches!(event, RuntimeEvent::TextDelta { text } if text == "you said Blue")
        )
    );

    // Limits, the model, an interrupt.
    session.limits().unwrap();
    let events = until(session.as_mut(), |event| {
        matches!(event, RuntimeEvent::LimitsUpdated { .. })
    });
    assert!(matches!(
        events.last(),
        Some(RuntimeEvent::LimitsUpdated { limits }) if limits.windows[0].resets_at_ms.is_some()
    ));
    assert!(
        session.limits().unwrap().is_some(),
        "the last report is kept"
    );
    session.set_model("fake-2").unwrap();
    until(
        session.as_mut(),
        |event| matches!(event, RuntimeEvent::ModelChanged { model } if model == "fake-2"),
    );
    session
        .send_turn(TurnInput::from_user("ask: something slow"))
        .unwrap();
    until(session.as_mut(), |event| {
        matches!(event, RuntimeEvent::ApprovalRequested { .. })
    });
    session.interrupt().unwrap();
    let events = until(session.as_mut(), turn_end);
    assert!(matches!(
        events.last(),
        Some(RuntimeEvent::TurnEnded {
            outcome: TurnEnd::Interrupted,
            ..
        })
    ));

    // Let go between turns: the fake exits when its input closes.
    let outcome = session
        .release(Release::AfterTurn(Duration::from_secs(2)))
        .unwrap();
    assert!(outcome.at_boundary);
    assert_eq!(outcome.native_id.as_deref(), Some("fake-driven-1"));
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn a_driven_conversation_is_found_and_resumed_in_either_runtime() {
    let world = world();
    let since = 0;
    let (mut child, mut session) = start(&world, None);
    session.send_turn(TurnInput::from_user("first")).unwrap();
    until(session.as_mut(), turn_end);
    let outcome = session
        .release(Release::AfterTurn(Duration::from_secs(2)))
        .unwrap();
    child.wait().unwrap();
    let id = outcome.native_id.unwrap();

    // The readers find it as they find any session of this account.
    let found = world
        .adapter
        .locate_transcript(&world.account.state_dir, &world.cwd, since, &[])
        .expect("the driven conversation is found");
    assert_eq!(found.native_id.as_deref(), Some(id.as_str()));

    // Resumed in the terminal…
    let spec = world
        .adapter
        .launch_spec(
            &world.account,
            &world.profile,
            &world.cwd,
            &LaunchMode::Resume {
                session_id: id.clone(),
                delta_prompt: None,
            },
        )
        .unwrap();
    assert!(spec.args.contains(&id));
    let view = world
        .adapter
        .driven()
        .unwrap()
        .terminal_view(
            &world.account,
            &world.profile,
            &world.cwd,
            &ViewOf {
                native_id: id.clone(),
                socket: None,
            },
        )
        .unwrap();
    assert!(matches!(
        view,
        juggler_adapter::TerminalView::AfterRelease(_)
    ));

    // …and driven again: the same file grows.
    let (mut child, mut session) = start(&world, Some(&id));
    session.send_turn(TurnInput::from_user("second")).unwrap();
    until(session.as_mut(), turn_end);
    drop(session.release(Release::Now).unwrap());
    child.wait().unwrap();
    let text = std::fs::read_to_string(&found.path).unwrap();
    assert!(
        text.contains("\"first\"") && text.contains("\"second\""),
        "{text}"
    );
}
