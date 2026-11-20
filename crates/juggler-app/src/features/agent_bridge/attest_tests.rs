//! AJ-E3013 (ADR-0020's D-116 addendum): an attested desktop session in
//! the pool decisions, and a real attestation of this test's own process
//! chain against a fixture Claude Code registry and app record (nothing
//! launched, no real profile read).

use std::path::Path;
use std::sync::Arc;

use juggler_agentbridge::wire::{Args, Call, Refusal, Request};
use juggler_core::agent_access::{AccessLevel, AgentAction};
use juggler_core::model::SessionId;
use juggler_core::pools::SessionFacts;
use juggler_core::storage::attested::desktop_id;
use juggler_core::storage::{MemberRef, PoolRole, Storage};

use crate::state::AppState;

pub(super) const FOLDER: &str = "acc-desk";
const CONVERSATION: &str = "conv-desk-1";

/// A state with a pool "Team", a terminal session `s-cli` and a desktop
/// conversation of folder `acc-desk` in it, both members; the pool's id.
pub(super) fn lab(dir: &Path) -> (Arc<AppState>, String) {
    let state = crate::test_support::bare_state(dir.join("data"));
    crate::features::agent_bridge::settings::answer_impl(&state, true).unwrap();
    let pool = {
        let storage = state.storage.lock().unwrap();
        for (id, dir_name) in [("acc-cli", "cli"), (FOLDER, "desk")] {
            storage
                .accounts()
                .insert(&crate::test_support::account(
                    id,
                    juggler_adapters::CLAUDE_CODE_ID,
                    id,
                    dir.join(dir_name),
                ))
                .unwrap();
        }
        crate::test_support::cli_session(&storage, dir);
        let pool = storage.pools().create("Team", None).unwrap().id;
        storage
            .pools()
            .join(
                &pool,
                &MemberRef::Session(SessionId::new("s-cli")),
                &[],
                PoolRole::Member,
                None,
            )
            .unwrap();
        join_desktop(&storage, &pool, CONVERSATION.into());
        pool
    };
    (state, pool)
}

/// The desktop folder's `conversation` joins `pool`.
fn join_desktop(storage: &Storage, pool: &str, conversation: String) {
    storage
        .pools()
        .join(
            pool,
            &MemberRef::Conversation {
                folder: FOLDER.into(),
                id: conversation,
            },
            &[],
            PoolRole::Member,
            None,
        )
        .unwrap();
}

/// Desktop members `members` (see [`conversation`]) in `pool`, each
/// attested.
pub(super) fn desktop_members(
    storage: &Storage,
    pool: &str,
    members: impl IntoIterator<Item = u32>,
) {
    for n in members {
        join_desktop(storage, pool, conversation(n));
        storage.attested().put(facts_of(n));
    }
}

/// The lab at Full, with desktop members `members` in its pool.
pub(super) fn full_lab(
    dir: &Path,
    members: impl IntoIterator<Item = u32>,
) -> (Arc<AppState>, String) {
    let (state, pool) = lab(dir);
    {
        let storage = state.storage.lock().unwrap();
        storage
            .agent_access()
            .set_level(&pool, AccessLevel::Full, &std::collections::BTreeSet::new())
            .unwrap();
        desktop_members(&storage, &pool, members);
    }
    (state, pool)
}

/// The lab, its desktop member's facts kept as an attestation keeps them.
fn attested_lab(dir: &Path) -> (Arc<AppState>, String) {
    let (state, pool) = lab(dir);
    state
        .storage
        .lock()
        .unwrap()
        .attested()
        .put(desktop_facts());
    (state, pool)
}

fn desktop_facts() -> SessionFacts {
    crate::test_support::desktop_facts("app:claude-app:/x", FOLDER, CONVERSATION)
}

/// Desktop member `n` as its attestation answers: its tool's process
/// `tool_pid`, started at `tool_start`.
pub(super) fn caller(n: u32, tool_pid: u32, tool_start: i64) -> super::attest::DesktopCaller {
    super::attest::DesktopCaller {
        session: desktop_id(FOLDER, &conversation(n)),
        app_id: "app:claude-app:/x".into(),
        folder: FOLDER.into(),
        conversation: conversation(n),
        tool_pid,
        tool_start,
        can_wake: true,
        facts: facts_of(n),
    }
}

/// A connection from process `pid` (version 1), now.
pub(super) fn peer(pid: u32) -> juggler_platform::local_ipc::Peer {
    juggler_platform::local_ipc::Peer {
        pid: Some(pid),
        pid_version: Some(1),
        accepted: std::time::SystemTime::now(),
    }
}

/// A connector's keyless wait of `secs` seconds.
pub(super) fn wait_of(secs: u32) -> juggler_agentbridge::wire::Wait {
    juggler_agentbridge::wire::Wait {
        v: juggler_agentbridge::wire::VERSION,
        token: String::new(),
        wait_secs: secs,
    }
}

/// A state that knows Claude Code, its agent tools turned on.
fn claude_state(dir: &Path) -> Arc<AppState> {
    let state = crate::test_support::state_with(
        dir.join("data"),
        vec![Arc::new(juggler_adapters::ClaudeCodeAdapter::new())],
    );
    crate::features::agent_bridge::settings::answer_impl(&state, true).unwrap();
    state
}

/// This test's parent process, standing in for the Code tab's Claude
/// Code: its registry entry in `config` (`fields`: its status and such),
/// and its pid and start (seconds).
fn stand_in(config: &Path, fields: &str) -> (u32, i64) {
    let me = std::process::id();
    let table = juggler_proc_list::list().unwrap();
    let parent = table.iter().find(|p| p.pid == me).unwrap().ppid;
    let start = table.iter().find(|p| p.pid == parent).unwrap().start_secs;
    let start = i64::try_from(start).unwrap();
    registry_entry(
        config,
        parent,
        &format!(
            r#"{{"pid":{parent},"sessionId":"{CONVERSATION}",{fields},"procStart":"{}","hostSessionId":"local_t1","entrypoint":"claude-desktop"}}"#,
            ctime(start)
        ),
    );
    (parent, start)
}

/// Claude Code's registry entry for process `pid`, in `config`.
fn registry_entry(config: &Path, pid: u32, body: &str) {
    std::fs::create_dir_all(config.join("sessions")).unwrap();
    std::fs::write(config.join(format!("sessions/{pid}.json")), body).unwrap();
}

#[test]
fn attested_facts_resolve_like_a_cli_sessions_and_only_while_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = lab(dir.path());
    let desk = desktop_id(FOLDER, CONVERSATION);
    let cli = SessionId::new("s-cli");
    let in_no_pool = |state: &Arc<AppState>| {
        let storage = state.storage.lock().unwrap();
        crate::features::pools::members::resolution_of(&storage, &desk)
            .unwrap()
            .is_none_or(|resolution| resolution.placements.is_empty())
    };
    assert!(in_no_pool(&state), "never attested: in no pool");
    assert_eq!(
        crate::features::pools::access::allowed(
            &state,
            desk.as_str(),
            &pool,
            AgentAction::ReadRoster,
            None
        ),
        juggler_core::agent_access::Decision::Deny
    );
    state
        .storage
        .lock()
        .unwrap()
        .attested()
        .put(desktop_facts());
    {
        let storage = state.storage.lock().unwrap();
        let resolution = crate::features::pools::members::resolution_of(&storage, &desk)
            .unwrap()
            .unwrap();
        assert_eq!(resolution.primary().unwrap().pool_id, pool);
        assert_eq!(
            crate::features::pools::members::role_in(&storage, &desk, &pool).unwrap(),
            crate::features::pools::members::role_in(&storage, &cli, &pool).unwrap()
        );
        let refs = crate::features::messages::refs_of(&storage, &desk);
        assert!(
            refs.contains(&format!("conversation:{FOLDER}/{CONVERSATION}")),
            "what was written to its Watched member reaches it: {refs:?}"
        );
    }
    // Every level decides every action for it as for the CLI member.
    for level in [
        AccessLevel::Read,
        AccessLevel::WorkTogether,
        AccessLevel::Full,
    ] {
        state
            .storage
            .lock()
            .unwrap()
            .agent_access()
            .set_level(&pool, level, &std::collections::BTreeSet::new())
            .unwrap();
        for action in AgentAction::ALL {
            let decide = |who: &str| {
                crate::features::pools::access::allowed(&state, who, &pool, action, None)
            };
            assert_eq!(
                decide(desk.as_str()),
                decide(cli.as_str()),
                "{level:?} {action:?}"
            );
        }
    }
    state.storage.lock().unwrap().attested().forget(&desk);
    assert!(in_no_pool(&state), "forgotten: in no pool again");
}

#[test]
fn a_keyless_call_without_a_peer_or_an_attestation_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = lab(dir.path());
    let line =
        serde_json::to_string(&Request::new(String::new(), Call::Whoami, Args::default())).unwrap();
    let refused = super::calls::handle(&state, &line);
    assert_eq!(refused.refused, Some(Refusal::Unattested));
    // A real peer that no desktop App holds: refused, never trusted.
    let peer = juggler_platform::local_ipc::Peer {
        pid: Some(std::process::id()),
        pid_version: juggler_proc_list::pid_version(std::process::id()),
        accepted: std::time::SystemTime::now(),
    };
    let refused = super::calls::handle_line_from(&state, &line, Some(peer));
    assert_eq!(refused.refused, Some(Refusal::Unattested), "{refused:?}");
    assert!(
        state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .pool_facts_of(&desktop_id(FOLDER, CONVERSATION))
            .unwrap()
            .is_none(),
        "nothing kept"
    );
}

/// `secs` as Claude Code records `procStart` (`ps -o lstart` in UTC).
fn ctime(secs: i64) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    let (year, month, day) = crate::features::app::diagnostics::zip::civil_day(days);
    format!(
        "{} {} {:2} {:02}:{:02}:{:02} {}",
        DAYS[usize::try_from(days.rem_euclid(7)).unwrap()],
        MONTHS[usize::try_from(month - 1).unwrap()],
        day,
        rest / 3600,
        rest % 3600 / 60,
        rest % 60,
        year
    )
}

#[test]
fn ctime_reads_back_as_the_same_second() {
    for secs in [0, 1_793_000_000, 1_700_000_061] {
        assert_eq!(
            juggler_adapter::live::ctime_utc_secs(&ctime(secs)),
            Some(secs)
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn this_processs_own_chain_is_attested_end_to_end_and_its_call_is_answered() {
    let dir = tempfile::tempdir().unwrap();
    let state = claude_state(dir.path());
    let config = dir.path().join("config");
    let instance = dir.path().join("instance");
    let me = std::process::id();
    let (parent, parent_start) = stand_in(&config, r#""status":"idle","kind":"interactive""#);
    std::fs::create_dir_all(instance.join("claude-code-sessions/a/o")).unwrap();
    std::fs::write(
        instance.join("claude-code-sessions/a/o/local_t1.json"),
        format!(r#"{{"sessionId":"local_t1","cliSessionId":"{CONVERSATION}"}}"#),
    )
    .unwrap();
    let pool = {
        let storage = state.storage.lock().unwrap();
        crate::test_support::desktop_app(&storage, FOLDER, "Desk", config.clone(), &instance);
        let pool = storage.pools().create("Team", None).unwrap().id;
        join_desktop(&storage, &pool, CONVERSATION.into());
        pool
    };
    let peer = juggler_platform::local_ipc::Peer {
        pid: Some(me),
        pid_version: juggler_proc_list::pid_version(me),
        accepted: std::time::SystemTime::now(),
    };
    // M1: the registry entry above was written by a "script" for its own
    // parent (this test's runner): that process isn't this App's signed
    // Claude Code, so the call is refused.
    let forged = super::attest::attest(&state, peer).unwrap_err();
    assert!(forged.contains("Claude Code"), "{forged}");
    // The rest of the path, with M1's check of the program set aside
    // (it is tested on its own).
    super::attest::ACCEPT_ANY_TOOL.with(|accept| accept.set(true));
    let caller = super::attest::attest(&state, peer).unwrap();
    assert_eq!(caller.session, desktop_id(FOLDER, CONVERSATION));
    assert_eq!(caller.tool_pid, parent);
    assert!(caller.can_wake);
    let line =
        serde_json::to_string(&Request::new(String::new(), Call::Whoami, Args::default())).unwrap();
    let answer = super::calls::handle_line_from(&state, &line, Some(peer));
    assert!(answer.result.is_some(), "{answer:?}");
    let result = answer.result.unwrap();
    assert_eq!(result["pool"]["id"], pool.as_str());
    assert_eq!(
        result["session_id"],
        desktop_id(FOLDER, CONVERSATION).as_str()
    );
    // The registry going stale (pid reused) refuses the next call and
    // drops what was kept.
    registry_entry(
        &config,
        parent,
        &format!(
            r#"{{"pid":{parent},"sessionId":"{CONVERSATION}","procStart":"{}","hostSessionId":"local_t1","entrypoint":"claude-desktop"}}"#,
            ctime(parent_start - 3600)
        ),
    );
    let refused = super::calls::handle_line_from(&state, &line, Some(peer));
    assert_eq!(refused.refused, Some(Refusal::Unattested));
    assert!(
        state
            .storage
            .lock()
            .unwrap()
            .attested()
            .get(&desktop_id(FOLDER, CONVERSATION))
            .is_none()
    );
}

/// The aij-test proof, run 2: the session's first call was attested
/// seconds after it started, before the conversation index had read its
/// transcript. Its project was "none", kept so for as long as it called:
/// it landed in the "No project" pool, not the pool of its App and folder,
/// unnamed. Now its project comes from the app's own record of its folder
/// on the first call, "none" is never kept, and it's named by the record's
/// title and its App by its profile folder's name.
#[test]
fn the_first_call_finds_the_sessions_project_and_name_before_any_index() {
    let dir = tempfile::tempdir().unwrap();
    let state = claude_state(dir.path());
    let config = dir.path().join("config");
    let instance = dir.path().join("aij-test");
    let web = dir.path().join("lab/web");
    std::fs::create_dir_all(&web).unwrap();
    let me = std::process::id();
    stand_in(&config, r#""status":"busy""#);
    std::fs::create_dir_all(instance.join("claude-code-sessions/a/o")).unwrap();
    let record = |cwd: Option<&Path>| {
        let cwd = cwd.map_or_else(String::new, |cwd| format!(r#","cwd":"{}""#, cwd.display()));
        std::fs::write(
            instance.join("claude-code-sessions/a/o/local_t1.json"),
            format!(
                r#"{{"sessionId":"local_t1","cliSessionId":"{CONVERSATION}","title":"Fix the login"{cwd}}}"#
            ),
        )
        .unwrap();
    };
    let (pool, project) = {
        let storage = state.storage.lock().unwrap();
        let app =
            crate::test_support::desktop_app(&storage, FOLDER, "Desk", config.clone(), &instance);
        let project = storage.projects().ensure(&web).unwrap().id;
        let pool = storage.pools().create("aij46 run2", None).unwrap().id;
        storage
            .pools()
            .set_scope(
                &pool,
                &juggler_core::storage::Scope::These([Some(project.clone())].into()),
            )
            .unwrap();
        storage
            .pools()
            .join(&pool, &MemberRef::App(app), &[], PoolRole::Member, None)
            .unwrap();
        (pool, project)
    };
    let peer = juggler_platform::local_ipc::Peer {
        pid: Some(me),
        pid_version: juggler_proc_list::pid_version(me),
        accepted: std::time::SystemTime::now(),
    };
    super::attest::ACCEPT_ANY_TOOL.with(|accept| accept.set(true));
    let whoami = || {
        let line =
            serde_json::to_string(&Request::new(String::new(), Call::Whoami, Args::default()))
                .unwrap();
        super::calls::handle_line_from(&state, &line, Some(peer))
    };
    // A record that doesn't name the folder yet: no project on this call…
    record(None);
    let caller = super::attest::attest(&state, peer).unwrap();
    assert_eq!(caller.facts.project, None);
    // …and "none" isn't kept: once the record names it, it's found.
    record(Some(&web));
    let caller = super::attest::attest(&state, peer).unwrap();
    assert_eq!(caller.facts.project, Some(project));
    let answer = whoami();
    assert!(answer.result.is_some(), "{answer:?}");
    let result = answer.result.unwrap();
    assert_eq!(
        result["pool"]["id"],
        pool.as_str(),
        "its App's pool for its folder"
    );
    assert_eq!(result["name"], "Fix the login", "the app's own title");
    let place = result["place"].as_str().unwrap_or_default();
    assert!(
        place.contains("aij-test"),
        "its App by its profile folder: {place}"
    );
    // The pool's member and recipient list has it (f1b9bd354's read).
    let listed = crate::features::connector::desktop_sessions_impl(&state, &pool).unwrap();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0].name, "Fix the login");
    assert_eq!(listed[0].via, "app");
}

#[test]
fn only_the_attestation_writes_the_attested_sessions() {
    // ADR-0020's D-116 addendum: `attested().put(` (and run 3's
    // `attested().renew(`) appears in one file of the app's code, the
    // attestation (tests aside).
    fn walk(dir: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                let code = text.split("#[cfg(test)]\nmod").next().unwrap_or_default();
                // Test files (`…_tests.rs`, a `tests.rs` module) aside, as
                // test modules are.
                let test_file = path.file_name().is_some_and(|name| {
                    let name = name.to_string_lossy();
                    name.ends_with("_tests.rs") || name == "tests.rs"
                });
                // As written, a call split over lines too.
                let flat: String = code.split_whitespace().collect();
                let writes = flat.contains("attested().put(") || flat.contains("attested().renew(");
                if writes && !test_file {
                    found.push(path.to_string_lossy().into_owned());
                }
            }
        }
    }
    let mut found = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].ends_with("features/agent_bridge/attest.rs"),
        "{found:?}"
    );
}

/// Puts a message from `sender` (a session ref, or the user) to the
/// desktop member's Watched ref, unread, in a new thread.
fn message(state: &Arc<AppState>, pool: &str, id: &str, sender: &str, kind: &str) {
    use juggler_core::storage::{NewMessage, RecipientState};
    let storage = state.storage.lock().unwrap();
    let repo = storage.messages();
    repo.create_thread(&format!("t-{id}"), pool, "s", sender)
        .unwrap();
    repo.insert_message(&NewMessage {
        id,
        thread_id: &format!("t-{id}"),
        pool_id: pool,
        sender_ref: sender,
        sender_kind: kind,
        sender_name: "Planner",
        kind: "note",
        subject: "s",
        body: "the words never reach the notice",
        body_sha256: id,
        hop: 1,
        reply_to: None,
        to_all: false,
        redacted: 0,
        recipients: &[(
            format!("conversation:{FOLDER}/{CONVERSATION}"),
            "Desk".into(),
            RecipientState::Unread,
        )],
    })
    .unwrap();
}

#[test]
fn a_wake_needs_the_pools_waking_rule_and_keeps_to_its_budget() {
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = attested_lab(dir.path());
    let desk = desktop_id(FOLDER, CONVERSATION);
    // Nothing yet, and nothing before this point counts.
    assert_eq!(super::desktop::wake_for(&state, &desk), None);
    std::thread::sleep(std::time::Duration::from_millis(5));
    // Work together: an agent's message waits for its next turn.
    message(&state, &pool, "m-1", "session:s-cli", "agent");
    assert_eq!(super::desktop::wake_for(&state, &desk), None);
    // Full: the next agent's message wakes it, facts only.
    state
        .storage
        .lock()
        .unwrap()
        .agent_access()
        .set_level(&pool, AccessLevel::Full, &std::collections::BTreeSet::new())
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    message(&state, &pool, "m-2", "session:s-cli", "agent");
    let notice = super::desktop::wake_for(&state, &desk).unwrap();
    assert!(notice.contains("Planner"), "{notice}");
    assert!(!notice.contains("never reach"), "never a message's words");
    // The budget: no second wake within a minute.
    std::thread::sleep(std::time::Duration::from_millis(5));
    message(&state, &pool, "m-3", "session:s-cli", "agent");
    assert_eq!(super::desktop::wake_for(&state, &desk), None);
}

#[test]
fn a_persons_message_or_a_paused_pool_never_wakes() {
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = attested_lab(dir.path());
    state
        .storage
        .lock()
        .unwrap()
        .agent_access()
        .set_level(&pool, AccessLevel::Full, &std::collections::BTreeSet::new())
        .unwrap();
    let desk = desktop_id(FOLDER, "conv-desk-1");
    assert_eq!(super::desktop::wake_for(&state, &desk), None);
    std::thread::sleep(std::time::Duration::from_millis(5));
    message(
        &state,
        &pool,
        "u-1",
        juggler_core::storage::SENDER_USER,
        "user",
    );
    assert_eq!(super::desktop::wake_for(&state, &desk), None, "a person's");
    state
        .storage
        .lock()
        .unwrap()
        .messages()
        .pause_pool(&pool, "held", "user")
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    message(&state, &pool, "m-9", "session:s-cli", "agent");
    assert_eq!(super::desktop::wake_for(&state, &desk), None, "paused");
}

/// L3: a call uses the facts its own attestation gave, never the shared
/// map: a desktop id with no facts carried is refused even while the map
/// holds some for it.
#[test]
fn a_call_carries_its_own_attested_facts() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = attested_lab(dir.path());
    let storage = state.storage.lock().unwrap();
    let desk = desktop_id(FOLDER, CONVERSATION);
    assert!(super::calls::whose(&storage, &desk, None).is_err());
    let other = SessionFacts {
        session: desktop_id("another-folder", CONVERSATION),
        ..desktop_facts()
    };
    assert!(
        super::calls::whose(&storage, &desk, Some(&other)).is_err(),
        "facts of another session"
    );
    let (_, folder, member) = super::calls::whose(&storage, &desk, Some(&desktop_facts())).unwrap();
    assert_eq!(folder, FOLDER);
    assert_eq!(
        member.unwrap().to_wire(),
        format!("conversation:{FOLDER}/{CONVERSATION}")
    );
}

/// A desktop session kept as an attestation keeps it: a Claude app's
/// config folder with its registry entry for `pid`, its app record, and
/// the App with that folder.
struct Kept {
    state: Arc<AppState>,
    app: String,
    config: std::path::PathBuf,
    records: std::path::PathBuf,
    pid: u32,
    start: i64,
}

impl Kept {
    fn new(dir: &Path, pid: u32, start: i64) -> Self {
        let state = crate::test_support::state_with(
            dir.join("data"),
            vec![Arc::new(juggler_adapters::ClaudeCodeAdapter::new())],
        );
        let config = dir.join("config");
        let instance = dir.join("instance");
        std::fs::create_dir_all(config.join("sessions")).unwrap();
        let records = instance.join("claude-code-sessions/acct/org");
        std::fs::create_dir_all(&records).unwrap();
        let app = crate::test_support::desktop_app(
            &state.storage.lock().unwrap(),
            FOLDER,
            "Desk",
            config.clone(),
            &instance,
        );
        let kept = Self {
            state,
            app,
            config,
            records,
            pid,
            start,
        };
        kept.entry(start);
        kept.app_record(false);
        kept
    }

    /// Its registry entry, naming its process's start as `start`.
    fn entry(&self, start: i64) {
        std::fs::write(
            self.registry(),
            format!(
                r#"{{"pid":{},"sessionId":"{CONVERSATION}","procStart":"{}","hostSessionId":"local_t1","entrypoint":"claude-desktop"}}"#,
                self.pid,
                ctime(start)
            ),
        )
        .unwrap();
    }

    fn registry(&self) -> std::path::PathBuf {
        self.config.join(format!("sessions/{}.json", self.pid))
    }

    fn app_record(&self, archived: bool) {
        std::fs::write(
            self.records.join("local_t1.json"),
            format!(
                r#"{{"sessionId":"local_t1","cliSessionId":"{CONVERSATION}","isArchived":{archived}}}"#
            ),
        )
        .unwrap();
    }

    /// What an attestation keeps for it.
    fn keep(&self) {
        self.state
            .storage
            .lock()
            .unwrap()
            .attested()
            .put(desktop_facts());
        crate::test_support::keep_desktop(
            &self.state,
            &self.app,
            FOLDER,
            CONVERSATION,
            self.pid,
            self.start,
        );
    }

    fn kept(&self) -> bool {
        let desk = desktop_id(FOLDER, CONVERSATION);
        self.state
            .storage
            .lock()
            .unwrap()
            .attested()
            .get(&desk)
            .is_some()
            && self
                .state
                .agent_bridge
                .desktop
                .lock()
                .unwrap()
                .contains_key(desk.as_str())
    }
}

/// H3: on every attested call, what is kept for each desktop session is
/// dropped at once when its tool's registry entry is gone or names
/// another start time, (round 2) when the app archives its conversation,
/// or (run 3) when its process ended, not only after ten minutes.
#[test]
fn kept_facts_go_when_the_registry_entry_goes_its_start_changes_or_it_is_archived() {
    let dir = tempfile::tempdir().unwrap();
    let start = 1_793_000_000;
    let lab = Kept::new(dir.path(), 4242, start);
    let alive = std::cell::Cell::new(true);
    let running = |pid: u32, at: i64| Some(alive.get() && pid == 4242 && at == start);
    let sweep =
        || super::attest::sweep(&lab.state, &super::attest::candidates(&lab.state), &running);
    lab.keep();
    assert!(!sweep());
    assert!(lab.kept(), "still its registry's");
    lab.entry(start + 3600);
    assert!(sweep());
    assert!(!lab.kept(), "its start time changed: dropped at once");
    lab.entry(start);
    lab.keep();
    std::fs::remove_file(lab.registry()).unwrap();
    sweep();
    assert!(!lab.kept(), "its registry entry gone: dropped at once");
    lab.entry(start);
    lab.keep();
    sweep();
    assert!(lab.kept(), "back");
    alive.set(false);
    sweep();
    assert!(
        !lab.kept(),
        "its process ended, its entry left behind: dropped at once"
    );
    alive.set(true);
    lab.keep();
    lab.app_record(true);
    sweep();
    assert!(!lab.kept(), "its conversation archived: dropped at once");
}

/// Run 3 (aij-test quit, its App still said "1 open now"): a desktop
/// session counts as open and as a running holder or recipient while its
/// tool's process runs, and stops at the live view's next tick once its
/// registry entry goes, no timer passing; the tick says so (the window
/// is told). This test's own process stands in for its Claude Code.
#[test]
fn a_desktop_session_stops_counting_as_open_when_its_registry_entry_goes() {
    let dir = tempfile::tempdir().unwrap();
    let pid = std::process::id();
    let start = juggler_proc_list::list()
        .and_then(|all| all.into_iter().find(|process| process.pid == pid))
        .map_or(1_793_000_000, |process| {
            i64::try_from(process.start_secs).unwrap()
        });
    let lab = Kept::new(dir.path(), pid, start);
    lab.keep();
    let member = format!("conversation:{FOLDER}/{CONVERSATION}");
    let open = || {
        let storage = lab.state.storage.lock().unwrap();
        let desk = desktop_id(FOLDER, CONVERSATION);
        storage
            .attested()
            .fresh()
            .iter()
            .any(|facts| facts.session == desk)
            && crate::features::messages::session_now(&storage, &member).is_some()
    };
    assert!(
        !super::attest::sweep_now(&lab.state),
        "it runs: nothing went"
    );
    assert!(open());
    std::fs::remove_file(lab.registry()).unwrap();
    assert!(super::attest::sweep_now(&lab.state), "the window is told");
    assert!(!open(), "not open, not a running holder or recipient");
    assert!(
        !super::attest::sweep_now(&lab.state),
        "none kept: nothing to do"
    );
    // Gone on another session's call: told at the next tick all the same.
    lab.entry(start);
    lab.keep();
    std::fs::remove_file(lab.registry()).unwrap();
    let unknown = |_: u32, _: i64| None;
    super::attest::sweep(&lab.state, &super::attest::candidates(&lab.state), &unknown);
    assert!(!open());
    assert!(super::attest::sweep_now(&lab.state), "the window is told");
}

/// The A9 lab walk (2026-11-15): an idle desktop member never woke for
/// the person's message sent with Deliver. Its connector's first waits
/// were refused (the start race), and then: a person's message never
/// counted for a wake, even delivered, and what came before the first
/// answered wait was taken as already seen. Now the person's Deliver wakes
/// it, and a message since its Claude Code started counts, however late
/// the first answered wait comes. A person's message without Deliver
/// still waits for its turn (D-119).
#[test]
fn the_persons_deliver_wakes_a_desktop_member_even_after_a_refused_start() {
    use crate::features::messages::send::{self, Author, Outgoing};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = lab(dir.path());
    desktop_members(&state.storage.lock().unwrap(), &pool, [60]);
    let started = juggler_core::model::Timestamp::now().millis() / 1_000 - 60;
    let ready = Arc::new(AtomicBool::new(false));
    let attestor: super::desktop::Attestor = {
        let ready = Arc::clone(&ready);
        Arc::new(move |_state, _peer| {
            if !ready.load(Ordering::SeqCst) {
                return Err(
                    "the caller wasn't started by a Code-tab session of this Claude app profile"
                        .to_owned(),
                );
            }
            Ok(caller(60, u32::MAX, started))
        })
    };
    let peer = peer(6_060);
    let wait = |state: &Arc<AppState>, attestor: &super::desktop::Attestor| {
        tauri::async_runtime::block_on(super::desktop::wait_with(
            Arc::clone(state),
            wait_of(1),
            peer,
            Arc::clone(attestor),
            Duration::from_millis(50),
        ))
    };
    let member = format!("conversation:{FOLDER}/{}", conversation(60));
    let person = |deliver: bool, n: u32| {
        send::send(
            &state,
            &Outgoing {
                pool_id: pool.clone(),
                from: Author::User,
                to: member.clone(),
                kind: None,
                subject: Some(format!("lab wake {n}")),
                body: format!("Lab wake {n} from the person."),
                thread: None,
                reply_to: None,
                deliver,
            },
        )
        .unwrap()
    };
    // Its connector's first wait: refused (the start race).
    let first = wait(&state, &attestor);
    assert_eq!(first.refused, Some(Refusal::Unattested));
    // The person's message without Deliver: it waits for its turn.
    let plain = person(false, 1);
    assert!(
        plain
            .routes
            .iter()
            .any(|(_, how)| *how == send::DESKTOP_NEXT_TURN),
        "{:?}",
        plain.routes
    );
    ready.store(true, Ordering::SeqCst);
    let quiet = wait(&state, &attestor);
    assert!(
        quiet.result.unwrap()["wake"].is_null(),
        "a person's message alone"
    );
    // With Deliver: queued by the click, and the next wait wakes it.
    let delivered = person(true, 2);
    assert!(
        delivered
            .routes
            .iter()
            .any(|(_, how)| *how == send::DESKTOP_WOKEN),
        "{:?}",
        delivered.routes
    );
    let woke = wait(&state, &attestor);
    let notice = woke.result.unwrap()["wake"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(
        notice.contains("You"),
        "the person's Deliver wakes it: {notice}"
    );
}

/// The A9 lab walk: an agent's message that came while the connector's
/// first waits were refused still wakes the member at its first answered
/// wait (since its Claude Code started, not since that wait).
#[test]
fn a_message_from_before_the_first_answered_wait_still_wakes() {
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = full_lab(dir.path(), [61]);
    let started = juggler_core::model::Timestamp::now().millis() / 1_000 - 60;
    // The message comes before any wait was answered.
    message_for(&state, &pool, "m-61", &conversation(61));
    std::thread::sleep(Duration::from_millis(20));
    let attestor: super::desktop::Attestor =
        Arc::new(move |_state, _peer| Ok(caller(61, u32::MAX, started)));
    let woke = tauri::async_runtime::block_on(super::desktop::wait_with(
        Arc::clone(&state),
        wait_of(1),
        peer(6_061),
        attestor,
        Duration::from_millis(50),
    ));
    let notice = woke.result.unwrap()["wake"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(notice.contains("Planner"), "{notice}");
}

/// Desktop member `n`'s conversation.
pub(super) fn conversation(n: u32) -> String {
    format!("conv-{n}")
}

/// Desktop member `n`'s facts.
pub(super) fn facts_of(n: u32) -> SessionFacts {
    SessionFacts {
        conversation: Some((FOLDER.into(), conversation(n))),
        ..SessionFacts::bare(desktop_id(FOLDER, &conversation(n)), None)
    }
}

/// An agent's message from the terminal member to desktop member
/// `conversation`, unread.
pub(super) fn message_for(state: &Arc<AppState>, pool: &str, id: &str, conversation: &str) {
    use juggler_core::storage::{NewMessage, RecipientState};
    let storage = state.storage.lock().unwrap();
    let repo = storage.messages();
    repo.create_thread(&format!("t-{id}"), pool, "s", "session:s-cli")
        .unwrap();
    repo.insert_message(&NewMessage {
        id,
        thread_id: &format!("t-{id}"),
        pool_id: pool,
        sender_ref: "session:s-cli",
        sender_kind: "agent",
        sender_name: "Planner",
        kind: "note",
        subject: "s",
        body: "the words never reach the notice",
        body_sha256: id,
        hop: 1,
        reply_to: None,
        to_all: false,
        redacted: 0,
        recipients: &[(
            format!("conversation:{FOLDER}/{conversation}"),
            "Desk".into(),
            RecipientState::Unread,
        )],
    })
    .unwrap();
}

/// L8: the wait is attested again before its notice goes out, and
/// (round 2) before anything is marked: when the session's conversation
/// changed meanwhile (a /clear, a resume), the notice is dropped, its
/// messages stay due and its wake budget is unspent.
#[test]
fn a_notice_is_dropped_when_the_conversation_changed_during_the_wait() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = full_lab(dir.path(), 0..4);
    let run = |changes: bool| {
        let calls = Arc::new(AtomicUsize::new(0));
        let attestor: super::desktop::Attestor = {
            let calls = Arc::clone(&calls);
            Arc::new(move |_state, _peer| {
                // Unchanged: conversation 0 throughout. Changed: 2 when it
                // starts to wait, 3 by the time its notice would go.
                let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
                let n = match (changes, first) {
                    (false, _) => 0,
                    (true, true) => 2,
                    (true, false) => 3,
                };
                Ok(caller(
                    n,
                    1,
                    juggler_core::model::Timestamp::now().millis() / 1_000,
                ))
            })
        };
        let peer = peer(if changes { 2_001 } else { 2_002 });
        let wait = wait_of(3);
        let state = Arc::clone(&state);
        let pool = pool.clone();
        tauri::async_runtime::block_on(async move {
            let handle = tauri::async_runtime::spawn(super::desktop::wait_with(
                Arc::clone(&state),
                wait,
                peer,
                attestor,
                Duration::from_millis(50),
            ));
            tokio::time::sleep(Duration::from_millis(200)).await;
            message_for(
                &state,
                &pool,
                if changes { "m-a" } else { "m-b" },
                &conversation(if changes { 2 } else { 0 }),
            );
            handle.await.unwrap()
        })
    };
    let kept = run(false);
    assert!(
        kept.result.unwrap()["wake"].is_string(),
        "the same conversation"
    );
    let dropped = run(true);
    assert!(
        dropped.result.unwrap()["wake"].is_null(),
        "the conversation changed: dropped"
    );
    // L8 (round 2): it was the re-attestation that dropped it, before
    // anything was marked: the old conversation's message is still due
    // and its wake budget unspent.
    let due = super::desktop::wake_for(&state, &desktop_id(FOLDER, &conversation(2)));
    assert!(
        due.is_some_and(|notice| notice.contains("Planner")),
        "nothing marked seen, no wake spent"
    );
}

/// L4: a real connection's audit token carries the pid version the
/// kernel reports for the live process that connected (this one).
#[cfg(target_os = "macos")]
#[test]
fn the_tokens_pid_version_is_the_live_processs() {
    let root = tempfile::Builder::new()
        .prefix("aij-tok")
        .tempdir_in("/tmp")
        .unwrap();
    let endpoint = root.path().join("d").join("s.sock");
    let me = juggler_platform::user::current().unwrap();
    let bound = juggler_platform::local_ipc::bind(&endpoint, me).unwrap();
    let peer = tauri::async_runtime::block_on(async move {
        let listener = bound.into_async().unwrap();
        let connecting = std::thread::spawn({
            let endpoint = endpoint.clone();
            move || juggler_platform::local_ipc::connect(&endpoint).unwrap()
        });
        let stream = listener.accept().await.unwrap().unwrap();
        drop(connecting.join().unwrap());
        stream.peer()
    });
    assert_eq!(peer.pid, Some(std::process::id()));
    assert_eq!(
        peer.pid_version,
        juggler_proc_list::pid_version(std::process::id())
    );
}
