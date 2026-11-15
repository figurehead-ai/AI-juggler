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

    let all = lab.send("s-cli", "session:s-cli", "pool", 4).unwrap();
    assert_eq!(
        lab.copy(&all, MEMBER),
        RecipientState::Unread,
        "the whole pool reaches it"
    );
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
