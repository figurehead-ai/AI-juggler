//! AJ-E3013 integration: a desktop conversation in a pool, connected (its
//! App has AI Juggler's connector, or one of its sessions called lately)
//! or Watched (no connector), as messages, Needs you and the pools' counts
//! see it. A fixture Claude app profile over temporary folders; nothing
//! launched.

use std::sync::Arc;

use juggler_agentbridge::wire::Response;
use juggler_core::agent_access::AccessLevel;
use juggler_core::model::{SessionId, SessionStatus};
use juggler_core::pools::SessionFacts;
use juggler_core::storage::attested::desktop_id;
use juggler_core::storage::{MemberRef, PoolRole, RecipientState};

use crate::features::messages::send::{self, Author, Outgoing, Sent};
use crate::state::AppState;

const FOLDER: &str = "acc-claude";
const CONVERSATION: &str = "conv-1";
const MEMBER: &str = "conversation:acc-claude/conv-1";

struct Lab {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    app: String,
    pool: String,
}

/// A Claude app profile (folder `acc-claude`), a terminal session `s-cli`
/// of another folder, and a pool "Team" with `s-cli` and the app's
/// conversation `conv-1` in it.
fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::ClaudeCodeAdapter::new()));
    let state = Arc::new(AppState::init_with_registry(dir.path().join("data"), registry).unwrap());
    // Any program of root's stands in for juggler-agent (never run here).
    state
        .agent_bridge
        .set_program(Some(std::path::PathBuf::from("/bin/echo")));
    let (app, pool) = {
        let storage = state.storage.lock().unwrap();
        for (id, folder) in [(FOLDER, "claude-config"), ("acc-cli", "cli")] {
            std::fs::create_dir_all(dir.path().join(folder)).unwrap();
            storage
                .accounts()
                .insert(&crate::test_support::account(
                    id,
                    juggler_adapters::CLAUDE_CODE_ID,
                    id,
                    dir.path().join(folder),
                ))
                .unwrap();
        }
        storage
            .profiles()
            .insert(&crate::test_support::launch_settings(
                "rec", "Main", "acc-cli",
            ))
            .unwrap();
        storage
            .sessions()
            .insert(&crate::test_support::instance(
                "s-cli",
                "rec",
                SessionStatus::Running,
                dir.path().join("w"),
            ))
            .unwrap();
        let apps = storage.apps();
        let app = apps
            .upsert_desktop(
                juggler_adapters::CLAUDE_CODE_ID,
                "claude-app",
                Some(&dir.path().join("instance").to_string_lossy()),
                1,
            )
            .unwrap();
        apps.set_folder(&app, Some(FOLDER)).unwrap();
        let pool = storage.pools().create("Team", None).unwrap().id;
        for member in [
            MemberRef::Session(SessionId::new("s-cli")),
            MemberRef::Conversation {
                folder: FOLDER.into(),
                id: CONVERSATION.into(),
            },
        ] {
            storage
                .pools()
                .join(&pool, &member, &[], PoolRole::Member, None)
                .unwrap();
        }
        (app, pool)
    };
    Lab {
        _dir: dir,
        state,
        app,
        pool,
    }
}

impl Lab {
    /// Keeps the facts an attestation of `conv-1`'s session would keep.
    fn attest(&self) {
        let storage = self.state.storage.lock().unwrap();
        let attested = storage.attested();
        attested.put(SessionFacts {
            app: Some(self.app.clone()),
            tool: Some(juggler_adapters::CLAUDE_CODE_ID.into()),
            conversation: Some((FOLDER.into(), CONVERSATION.into())),
            ..SessionFacts::bare(desktop_id(FOLDER, CONVERSATION), None)
        });
    }

    fn full(&self) {
        self.state
            .storage
            .lock()
            .unwrap()
            .agent_access()
            .set_level(
                &self.pool,
                AccessLevel::Full,
                &std::collections::BTreeSet::new(),
            )
            .unwrap();
    }

    /// An agent's message from `session` (member `member_ref`) to `to`.
    fn send(&self, session: &str, member_ref: &str, to: &str, n: u32) -> Result<Sent, Response> {
        send::send(
            &self.state,
            &Outgoing {
                pool_id: self.pool.clone(),
                from: Author::Agent {
                    session: SessionId::new(session),
                    member_ref: member_ref.to_owned(),
                    name: "Planner".into(),
                    tool: "Claude Code".into(),
                },
                to: to.to_owned(),
                kind: None,
                subject: Some(format!("subject {n}")),
                body: format!("body {n}"),
                thread: None,
                reply_to: None,
                deliver: false,
            },
        )
    }

    /// Whether `recipient` holds a copy of `sent`.
    fn has_copy(&self, sent: &Sent, recipient: &str) -> bool {
        self.state
            .storage
            .lock()
            .unwrap()
            .messages()
            .recipients(&sent.message.id)
            .unwrap()
            .iter()
            .any(|r| r.recipient_ref == recipient)
    }

    /// `conv-1`'s session as its tool's process started at `start` keeps
    /// it (what an attestation keeps in memory).
    fn process(&self, start: i64) {
        self.state.agent_bridge.desktop.lock().unwrap().insert(
            desktop_id(FOLDER, CONVERSATION).as_str().to_owned(),
            (
                self.app.clone(),
                juggler_adapter::peer::AttestedSession {
                    conversation: CONVERSATION.into(),
                    host_session: "local_t1".into(),
                    tool_pid: 4242,
                    tool_start: start,
                    cwd: None,
                    title: None,
                },
            ),
        );
    }

    /// `conv-1`'s session reads its inbox (unread), through its tools.
    fn read(&self) -> serde_json::Value {
        let caller = crate::features::messages::agent::Caller {
            session: desktop_id(FOLDER, CONVERSATION),
            pool_id: self.pool.clone(),
            pool_name: "Team".into(),
            member_ref: MEMBER.into(),
        };
        let request = juggler_agentbridge::wire::Request::new(
            String::new(),
            juggler_agentbridge::wire::Call::ReadInbox,
            juggler_agentbridge::wire::Args::default(),
        );
        crate::features::messages::agent::answer(&self.state, &caller, &request)
            .unwrap_or_else(|refused| unreachable!("{refused:?}"))
    }

    /// The state of `sent`'s copy for `recipient`.
    fn copy(&self, sent: &Sent, recipient: &str) -> RecipientState {
        self.state
            .storage
            .lock()
            .unwrap()
            .messages()
            .recipients(&sent.message.id)
            .unwrap()
            .into_iter()
            .find(|r| r.recipient_ref == recipient)
            .unwrap_or_else(|| unreachable!("{recipient} isn't a recipient"))
            .state
    }
}

/// How `sent` reaches the desktop member, in words.
fn how(sent: &Sent) -> &'static str {
    sent.routes
        .iter()
        .find(|(name, _)| name == "A session in another app")
        .map(|(_, how)| *how)
        .unwrap_or_else(|| unreachable!("no route for it: {:?}", sent.routes))
}

/// Item 1: with no connector its copy waits as Watched; once its App has
/// the connector it's a full member, read at its next turn, and woken at
/// Full (D-119); the whole pool reaches it too.
#[test]
fn a_desktop_member_with_the_connector_is_messaged_as_a_full_member_watched_without_it() {
    let lab = lab();
    let watched = lab.send("s-cli", "session:s-cli", MEMBER, 1).unwrap();
    assert_eq!(lab.copy(&watched, MEMBER), RecipientState::Held);
    assert_eq!(
        how(&watched),
        crate::features::messages::talk::WATCHED_GETS_IT
    );
    assert!(how(&watched).contains("no AI Juggler connector"), "why");

    super::add_impl(&lab.state, &lab.app).unwrap();
    let next = lab.send("s-cli", "session:s-cli", MEMBER, 2).unwrap();
    assert_eq!(lab.copy(&next, MEMBER), RecipientState::Unread);
    assert_eq!(how(&next), send::DESKTOP_NEXT_TURN);
    assert!(!next.held);

    lab.full();
    let woken = lab.send("s-cli", "session:s-cli", MEMBER, 3).unwrap();
    assert_eq!(lab.copy(&woken, MEMBER), RecipientState::Unread);
    assert_eq!(how(&woken), send::DESKTOP_WOKEN);

    // D-119 item 6: the whole pool reaches it while it's open (a call of
    // its attested, or its tool's process on it); not before.
    let closed = lab.send("s-cli", "session:s-cli", "pool", 4).unwrap();
    assert!(!lab.has_copy(&closed, MEMBER), "not open: no copy");
    lab.attest();
    let all = lab.send("s-cli", "session:s-cli", "pool", 5).unwrap();
    assert_eq!(
        lab.copy(&all, MEMBER),
        RecipientState::Unread,
        "the whole pool reaches it"
    );
}

/// D-119 item 6: a connected desktop member that isn't open gets no copy
/// of a pool-wide message; back from a new process of its tool, its first
/// read has it, and says so once.
#[test]
fn a_desktop_member_back_from_a_new_process_reads_what_the_whole_pool_was_told() {
    let lab = lab();
    super::add_impl(&lab.state, &lab.app).unwrap();
    let gone = lab.send("s-cli", "session:s-cli", "pool", 1).unwrap();
    assert!(!lab.has_copy(&gone, MEMBER));
    lab.attest();
    lab.process(100);
    let first = lab.read();
    assert_eq!(first["messages"][0]["id"], gone.message.id.as_str());
    assert_eq!(first["caught_up"][0]["copied"], 1);
    assert_eq!(first["caught_up"][0]["earlier"], 0);
    assert!(lab.read()["caught_up"].is_null(), "said once");
    // A new process with nothing missed since its last read: nothing.
    lab.process(200);
    let back = lab.read();
    assert!(back["caught_up"].is_null());
    assert!(back["messages"].as_array().unwrap().is_empty());
    assert_eq!(lab.copy(&gone, MEMBER), RecipientState::Read, "one copy");
}

/// Item 1: attested lately (no connector record needed), it's a full
/// member both ways: a message to it goes to its inbox, and it writes to
/// a member and to the whole pool (not to itself) under its own role.
#[test]
fn an_attested_desktop_member_reads_and_writes_as_a_full_member() {
    let lab = lab();
    lab.attest();
    let to_it = lab.send("s-cli", "session:s-cli", MEMBER, 1).unwrap();
    assert_eq!(lab.copy(&to_it, MEMBER), RecipientState::Unread);
    assert_eq!(how(&to_it), send::DESKTOP_NEXT_TURN);
    let desk = desktop_id(FOLDER, CONVERSATION);
    let from_it = lab.send(desk.as_str(), MEMBER, "s-cli", 2).unwrap();
    assert_eq!(lab.copy(&from_it, "session:s-cli"), RecipientState::Unread);
    let to_all = lab.send(desk.as_str(), MEMBER, "pool", 3).unwrap();
    let recipients = lab
        .state
        .storage
        .lock()
        .unwrap()
        .messages()
        .recipients(&to_all.message.id)
        .unwrap();
    assert_eq!(recipients.len(), 1, "everyone but itself: {recipients:?}");
    assert_eq!(recipients[0].recipient_ref, "session:s-cli");
    // Its attested session's id names it as well as its ref.
    let by_id = lab
        .send("s-cli", "session:s-cli", desk.as_str(), 4)
        .unwrap();
    assert_eq!(lab.copy(&by_id, MEMBER), RecipientState::Unread);
}

/// Item 1: a message held for another reason (the pool asks first) and
/// then allowed once in Needs you goes to a connected member's inbox; a
/// Watched member's copy goes on waiting.
#[test]
fn allow_once_releases_a_connected_members_copy_and_keeps_a_watched_ones() {
    let lab = lab();
    crate::features::pools::model::set_talk_impl(
        &lab.state,
        &lab.pool,
        "role:member",
        "role:member",
        Some("ask"),
    )
    .unwrap();
    let release = |n: u32| {
        let sent = lab.send("s-cli", "session:s-cli", MEMBER, n).unwrap();
        let ask = crate::features::messages::needs::needs(&lab.state)
            .unwrap()
            .into_iter()
            .find(|need| need.message_id.as_deref() == Some(sent.message.id.as_str()))
            .expect("held in Needs you");
        crate::features::messages::needs::answer(
            &lab.state,
            ask.ask_id.as_deref().unwrap(),
            "once",
        )
        .unwrap();
        lab.copy(&sent, MEMBER)
    };
    assert_eq!(release(1), RecipientState::Held, "Watched: still waits");
    lab.attest();
    assert_eq!(release(2), RecipientState::Unread, "connected: it reads it");
}

/// Item 3: a connected desktop member counts once in its pool (with the
/// sessions, not again as a Watched one), stays on the canvas as its
/// conversation (never "continued in AI Juggler"), and counts as one of
/// its App's sessions now.
#[test]
fn a_connected_desktop_member_counts_once_and_shows_on_its_apps_node() {
    let lab = lab();
    let count = || {
        let storage = lab.state.storage.lock().unwrap();
        crate::features::pools::pools_info(&storage)
            .unwrap()
            .pools
            .into_iter()
            .find(|pool| pool.id == lab.pool)
            .unwrap()
            .members
    };
    let before = count();
    assert_eq!(before, 2, "s-cli and the Watched conversation");
    lab.attest();
    assert_eq!(count(), 2, "attested: counted once, with the sessions");
    let watched = crate::features::pools::watched::watched_impl(&lab.state, &lab.pool).unwrap();
    assert_eq!(watched.len(), 1);
    assert_eq!(
        watched[0].continued_session, None,
        "not continued in AI Juggler"
    );
    // Its App as a member of another pool: its node counts the session.
    let other = {
        let storage = lab.state.storage.lock().unwrap();
        let other = storage.pools().create("Apps", None).unwrap().id;
        storage
            .pools()
            .join(
                &other,
                &MemberRef::App(lab.app.clone()),
                &[],
                PoolRole::Member,
                None,
            )
            .unwrap();
        other
    };
    let rules = {
        let storage = lab.state.storage.lock().unwrap();
        crate::features::pools::model::rules_of(&storage, &other).unwrap()
    };
    let node = rules
        .members
        .iter()
        .find(|rule| rule.member == MemberRef::App(lab.app.clone()).to_wire())
        .unwrap();
    assert_eq!(node.sessions, 1, "{node:?}");
    let reached = super::reach::of_pool(&lab.state, &other).unwrap();
    assert_eq!(reached.len(), 1, "held through its App: {reached:?}");
    assert!(reached[0].connected);
    assert_eq!(reached[0].member, MEMBER);
}

/// Items 2 and 4: the agents' roster says how each desktop member is
/// reached (never "can't reply" for a connected one; Watched says why),
/// marks the caller itself, and `whoami` fills an attested desktop
/// session's name, place and tool.
#[test]
fn the_roster_and_whoami_speak_of_a_connected_desktop_member_as_a_member() {
    use crate::features::agent_bridge::calls;
    let lab = lab();
    let pool = serde_json::json!({});
    let cli = SessionId::new("s-cli");
    let entry_of = |of: &SessionId| {
        calls::roster(&lab.state, of, &lab.pool, &pool).unwrap()["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["session_id"] == MEMBER || member["watched"] == MEMBER)
            .cloned()
            .expect("the desktop member in the roster")
    };
    let watched = entry_of(&cli);
    let words = watched["state"].as_str().unwrap();
    assert!(words.starts_with("watched: can't reply"), "{words}");
    assert!(words.contains("no AI Juggler connector"), "why: {words}");
    assert_eq!(watched["watched"], MEMBER);

    lab.attest();
    let connected = entry_of(&cli);
    assert_eq!(connected["session_id"], MEMBER, "written to by its ref");
    assert_eq!(connected["state"], super::reach::NEXT_TURN_WORDS);
    assert!(
        !connected["state"].as_str().unwrap().contains("can't reply"),
        "{connected}"
    );
    assert_eq!(connected["you"], false);
    lab.full();
    assert_eq!(entry_of(&cli)["state"], super::reach::WAKES_WORDS);
    let desk = desktop_id(FOLDER, CONVERSATION);
    assert_eq!(entry_of(&desk)["you"], true, "itself, from its own call");

    let (name, place, tool) = calls::who_am_i(&lab.state, &desk, &lab.pool, MEMBER).unwrap();
    assert_eq!(name.as_deref(), Some("A session in another app"));
    assert!(
        place.is_some_and(|place| place.contains("Claude")),
        "its App"
    );
    assert!(tool.is_some_and(|tool| tool.contains("Claude")), "its tool");
}

/// Item 4 (the lab's A6/A9): a connected desktop session held through its
/// App is listed for the pool's member and recipient lists, with the ref a
/// message to it takes; a Watched one isn't (pools_watched has it).
#[test]
fn a_connected_desktop_session_is_listed_for_the_members_and_recipients() {
    let lab = lab();
    let listed = super::desktop_sessions_impl(&lab.state, &lab.pool).unwrap();
    assert!(listed.is_empty(), "Watched only: {listed:?}");
    let apps = {
        let storage = lab.state.storage.lock().unwrap();
        let apps = storage.pools().create("Apps", None).unwrap().id;
        storage
            .pools()
            .join(
                &apps,
                &MemberRef::App(lab.app.clone()),
                &[],
                PoolRole::Member,
                None,
            )
            .unwrap();
        apps
    };
    lab.attest();
    let listed = super::desktop_sessions_impl(&lab.state, &apps).unwrap();
    assert_eq!(listed.len(), 1, "{listed:?}");
    let one = &listed[0];
    assert_eq!(one.member, MEMBER);
    assert_eq!(
        one.session_id.as_deref(),
        Some(desktop_id(FOLDER, CONVERSATION).as_str())
    );
    assert_eq!(one.via, "app");
    assert_eq!(one.app_id.as_deref(), Some(lab.app.as_str()));
    assert_eq!(one.wake, "next_turn");
    assert!(
        one.place
            .as_deref()
            .is_some_and(|place| place.contains("Claude"))
    );
    // In its own conversation's pool too, named on its own.
    let own = super::desktop_sessions_impl(&lab.state, &lab.pool).unwrap();
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].via, "conversation");
}

/// The aij-test proof, run 3: a desktop session claimed a pool task; Tasks
/// said "In progress · An agent", "whose session has ended", offered Free
/// it, and its Sessions read "A session that ended took it": the holder
/// was looked up among sessions AI Juggler runs only, by a name from their
/// roster only. Now it's the desktop session, named as the diagram names
/// it, running while it's attested, with no Free it offered.
#[test]
fn a_desktop_session_that_claims_a_task_is_its_running_named_holder() {
    use juggler_agentbridge::wire::{Args, Call, Request, TaskArgs, TaskCreateArgs};
    let lab = lab();
    lab.attest();
    // Its kept session names its title (the app's own record).
    lab.state.agent_bridge.desktop.lock().unwrap().insert(
        desktop_id(FOLDER, CONVERSATION).as_str().to_owned(),
        (
            lab.app.clone(),
            juggler_adapter::peer::AttestedSession {
                conversation: CONVERSATION.into(),
                host_session: "local_t1".into(),
                tool_pid: 1,
                tool_start: 0,
                cwd: None,
                title: Some("AI Juggler pool whoami result".into()),
            },
        ),
    );
    let desk = crate::features::tasks::agent::Caller {
        session: desktop_id(FOLDER, CONVERSATION),
        pool_id: lab.pool.clone(),
        pool_name: "Team".into(),
        member_ref: MEMBER.into(),
        role: "member".into(),
    };
    let created = crate::features::tasks::agent::answer(
        &lab.state,
        &desk,
        &Request::new(
            String::new(),
            Call::TaskCreate,
            Args {
                new_task: Some(TaskCreateArgs {
                    title: "lab task".into(),
                    ..TaskCreateArgs::default()
                }),
                ..Args::default()
            },
        ),
    )
    .unwrap();
    let task = created["task"]["id"].as_str().unwrap().to_owned();
    crate::features::tasks::agent::answer(
        &lab.state,
        &desk,
        &Request::new(
            String::new(),
            Call::TaskClaim,
            Args {
                task: Some(TaskArgs { task: task.clone() }),
                ..Args::default()
            },
        ),
    )
    .unwrap();
    let detail = crate::features::tasks::views::detail(&lab.state, &task, None).unwrap();
    assert_eq!(
        detail.task.holder.as_deref(),
        Some("AI Juggler pool whoami result"),
        "named as the diagram names it, never \"An agent\""
    );
    assert_eq!(
        detail.task.holder_session_id.as_deref(),
        Some(desktop_id(FOLDER, CONVERSATION).as_str()),
        "its session runs"
    );
    assert!(!detail.actions.free, "no Free it while it's open");
    let session = detail
        .sessions
        .iter()
        .find(|one| one.session_id == desktop_id(FOLDER, CONVERSATION).as_str())
        .expect("its session took it");
    assert!(session.live);
    assert_eq!(session.name, "AI Juggler pool whoami result");
}

/// Run 3, the same resolution in Messages: a message from a desktop
/// session names its session as the sender's, and a copy to it says it
/// reads through the connector (never "Not running").
#[test]
fn messages_name_a_desktop_session_as_sender_and_recipient() {
    let lab = lab();
    lab.attest();
    let desk = desktop_id(FOLDER, CONVERSATION);
    let to_it = lab.send("s-cli", "session:s-cli", MEMBER, 1).unwrap();
    let page = crate::features::messages::views::thread(&lab.state, &to_it.message.thread_id, None)
        .unwrap();
    let copy = page.messages[0]
        .recipients
        .iter()
        .find(|r| r.id == MEMBER)
        .unwrap()
        .clone();
    assert_eq!(copy.session_id.as_deref(), Some(desk.as_str()));
    assert!(copy.how.contains("connector"), "{}", copy.how);
    let from_it = lab.send(desk.as_str(), MEMBER, "s-cli", 2).unwrap();
    let page =
        crate::features::messages::views::thread(&lab.state, &from_it.message.thread_id, None)
            .unwrap();
    assert_eq!(
        page.messages[0].from_session_id.as_deref(),
        Some(desk.as_str())
    );
}

/// Round 7 #7: a desktop session's node is its own `conversation:` ref.
/// A message between it and a CLI session draws their line with its
/// count; a Blocked rule set on that line (that pair's own) refuses the
/// send at the bridge, while the role pair's rule stays as it was.
#[test]
fn a_line_between_a_cli_and_a_desktop_session_counts_and_its_own_rule_holds() {
    use crate::features::pools::{canvas, model};
    use juggler_core::pools::talk::RefuseReason;

    let lab = lab();
    lab.attest();
    let desk = desktop_id(FOLDER, CONVERSATION);
    lab.send("s-cli", "session:s-cli", MEMBER, 1).unwrap();
    lab.send(desk.as_str(), MEMBER, "s-cli", 2).unwrap();
    lab.send(desk.as_str(), MEMBER, "s-cli", 3).unwrap();
    let drawn =
        canvas::canvas_impl(&lab.state, &lab.pool, vec!["s-cli".into(), MEMBER.into()]).unwrap();
    let line = |from: &str, to: &str| {
        drawn
            .edges
            .iter()
            .find(|edge| edge.from_session_id == from && edge.to_session_id == to)
            .map(|edge| edge.messages)
    };
    assert_eq!(line("s-cli", MEMBER), Some(1));
    assert_eq!(line(MEMBER, "s-cli"), Some(2));

    // The line's own rule, as the window sets it (its ends' refs).
    let rules = model::set_talk_impl(
        &lab.state,
        &lab.pool,
        "session:s-cli",
        MEMBER,
        Some("blocked"),
    )
    .unwrap();
    assert_eq!(rules.talk.len(), 1, "the pair's own, no role pair's");
    assert_eq!(
        (rules.talk[0].from.as_str(), rules.talk[0].to.as_str()),
        ("session:s-cli", MEMBER)
    );
    let edge = model::edge_talk_impl(&lab.state, &lab.pool, "s-cli", MEMBER).unwrap();
    assert_eq!(
        (edge.rule.as_str(), edge.pair_rule.as_str(), edge.own),
        ("blocked", "allowed", true)
    );
    let refused = lab.send("s-cli", "session:s-cli", MEMBER, 4).unwrap_err();
    assert_eq!(
        refused.reason.as_deref(),
        Some(RefuseReason::Blocked.words())
    );
    // The other way is its own line, following the role pair.
    let back = model::edge_talk_impl(&lab.state, &lab.pool, MEMBER, "s-cli").unwrap();
    assert_eq!((back.rule.as_str(), back.own), ("allowed", false));
    assert!(lab.send(desk.as_str(), MEMBER, "s-cli", 5).is_ok());
}
