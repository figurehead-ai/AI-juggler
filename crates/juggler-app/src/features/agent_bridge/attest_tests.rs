//! AJ-E3013 (ADR-0020's D-116 addendum): an attested desktop session in
//! the pool decisions, and a real attestation of this test's own process
//! chain against a fixture Claude Code registry and app record (nothing
//! launched, no real profile read).

use std::path::Path;
use std::sync::Arc;

use juggler_agentbridge::wire::{Args, Call, Refusal, Request};
use juggler_core::agent_access::{AccessLevel, AgentAction};
use juggler_core::model::{SessionId, SessionStatus};
use juggler_core::pools::SessionFacts;
use juggler_core::storage::attested::desktop_id;
use juggler_core::storage::{MemberRef, PoolRole};

use crate::state::AppState;

const FOLDER: &str = "acc-desk";
const CONVERSATION: &str = "conv-desk-1";

/// A state with a pool "Team", a terminal session `s-cli` and a desktop
/// conversation of folder `acc-desk` in it, both members; the pool's id.
fn lab(dir: &Path) -> (Arc<AppState>, String) {
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
                dir.join("w"),
            ))
            .unwrap();
        let pool = storage.pools().create("Team", None).unwrap().id;
        let pools = storage.pools();
        pools
            .join(
                &pool,
                &MemberRef::Session(SessionId::new("s-cli")),
                &[],
                PoolRole::Member,
                None,
            )
            .unwrap();
        pools
            .join(
                &pool,
                &MemberRef::Conversation {
                    folder: FOLDER.into(),
                    id: CONVERSATION.into(),
                },
                &[],
                PoolRole::Member,
                None,
            )
            .unwrap();
        pool
    };
    (state, pool)
}

fn desktop_facts() -> SessionFacts {
    SessionFacts {
        app: Some("app:claude-app:/x".into()),
        tool: Some(juggler_adapters::CLAUDE_CODE_ID.into()),
        conversation: Some((FOLDER.into(), CONVERSATION.into())),
        ..SessionFacts::bare(desktop_id(FOLDER, CONVERSATION), None)
    }
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
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
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
    use juggler_core::model::HarnessId;
    let dir = tempfile::tempdir().unwrap();
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::ClaudeCodeAdapter::new()));
    let state = Arc::new(AppState::init_with_registry(dir.path().join("data"), registry).unwrap());
    crate::features::agent_bridge::settings::answer_impl(&state, true).unwrap();
    let config = dir.path().join("config");
    let instance = dir.path().join("instance");
    // This test's parent stands in for the Code tab's Claude Code process.
    let me = std::process::id();
    let table = juggler_proc_list::list().unwrap();
    let parent = table.iter().find(|p| p.pid == me).unwrap().ppid;
    let parent_start = table.iter().find(|p| p.pid == parent).unwrap().start_secs;
    std::fs::create_dir_all(config.join("sessions")).unwrap();
    std::fs::write(
        config.join(format!("sessions/{parent}.json")),
        format!(
            r#"{{"pid":{parent},"sessionId":"{CONVERSATION}","status":"idle","procStart":"{}","hostSessionId":"local_t1","entrypoint":"claude-desktop","kind":"interactive"}}"#,
            ctime(i64::try_from(parent_start).unwrap())
        ),
    )
    .unwrap();
    std::fs::create_dir_all(instance.join("claude-code-sessions/a/o")).unwrap();
    std::fs::write(
        instance.join("claude-code-sessions/a/o/local_t1.json"),
        format!(r#"{{"sessionId":"local_t1","cliSessionId":"{CONVERSATION}"}}"#),
    )
    .unwrap();
    let pool = {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&crate::test_support::account(
                FOLDER,
                HarnessId::new(juggler_adapters::CLAUDE_CODE_ID).as_str(),
                "Desk",
                config.clone(),
            ))
            .unwrap();
        let app = storage
            .apps()
            .upsert_desktop(
                juggler_adapters::CLAUDE_CODE_ID,
                "claude-app",
                Some(&instance.to_string_lossy()),
                1,
            )
            .unwrap();
        storage.apps().set_folder(&app, Some(FOLDER)).unwrap();
        let pool = storage.pools().create("Team", None).unwrap().id;
        storage
            .pools()
            .join(
                &pool,
                &MemberRef::Conversation {
                    folder: FOLDER.into(),
                    id: CONVERSATION.into(),
                },
                &[],
                PoolRole::Member,
                None,
            )
            .unwrap();
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
    std::fs::write(
        config.join(format!("sessions/{parent}.json")),
        format!(
            r#"{{"pid":{parent},"sessionId":"{CONVERSATION}","procStart":"{}","hostSessionId":"local_t1","entrypoint":"claude-desktop"}}"#,
            ctime(i64::try_from(parent_start).unwrap() - 3600)
        ),
    )
    .unwrap();
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

#[test]
fn only_the_attestation_writes_the_attested_sessions() {
    // ADR-0020's D-116 addendum: `attested().put(` appears in one file of
    // the app's code, the attestation (tests aside).
    fn walk(dir: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                let code = text.split("#[cfg(test)]\nmod").next().unwrap_or_default();
                if code.contains("attested().put(") && !path.ends_with("attest_tests.rs") {
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
    let (state, pool) = lab(dir.path());
    state
        .storage
        .lock()
        .unwrap()
        .attested()
        .put(desktop_facts());
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
    let (state, pool) = lab(dir.path());
    state
        .storage
        .lock()
        .unwrap()
        .attested()
        .put(desktop_facts());
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
    let (state, _) = lab(dir.path());
    state
        .storage
        .lock()
        .unwrap()
        .attested()
        .put(desktop_facts());
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

/// H3: on every attested call, what is kept for each desktop session is
/// dropped at once when its tool's registry entry is gone or names
/// another start time, not only after ten minutes.
#[test]
fn kept_facts_go_when_the_registry_entry_goes_or_its_start_changes() {
    use juggler_adapter::peer::AttestedSession;
    let dir = tempfile::tempdir().unwrap();
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::ClaudeCodeAdapter::new()));
    let state = Arc::new(AppState::init_with_registry(dir.path().join("data"), registry).unwrap());
    let config = dir.path().join("config");
    let instance = dir.path().join("instance");
    std::fs::create_dir_all(config.join("sessions")).unwrap();
    std::fs::create_dir_all(&instance).unwrap();
    let start = 1_793_000_000;
    let entry = |text: &str| {
        std::fs::write(
            config.join("sessions/4242.json"),
            format!(
                r#"{{"pid":4242,"sessionId":"{CONVERSATION}","procStart":"{text}","hostSessionId":"local_t1","entrypoint":"claude-desktop"}}"#
            ),
        )
        .unwrap();
    };
    entry(&ctime(start));
    let app = {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&crate::test_support::account(
                FOLDER,
                juggler_adapters::CLAUDE_CODE_ID,
                "Desk",
                config.clone(),
            ))
            .unwrap();
        let app = storage
            .apps()
            .upsert_desktop(
                juggler_adapters::CLAUDE_CODE_ID,
                "claude-app",
                Some(&instance.to_string_lossy()),
                1,
            )
            .unwrap();
        storage.apps().set_folder(&app, Some(FOLDER)).unwrap();
        app
    };
    let desk = desktop_id(FOLDER, CONVERSATION);
    let keep = || {
        state
            .storage
            .lock()
            .unwrap()
            .attested()
            .put(desktop_facts());
        state.agent_bridge.desktop.lock().unwrap().insert(
            desk.as_str().to_owned(),
            (
                app.clone(),
                AttestedSession {
                    conversation: CONVERSATION.into(),
                    host_session: "local_t1".into(),
                    tool_pid: 4242,
                    tool_start: start,
                },
            ),
        );
    };
    let kept = || {
        state
            .storage
            .lock()
            .unwrap()
            .attested()
            .get(&desk)
            .is_some()
            && state
                .agent_bridge
                .desktop
                .lock()
                .unwrap()
                .contains_key(desk.as_str())
    };
    let sweep = || super::attest::sweep(&state, &super::attest::candidates(&state));
    keep();
    sweep();
    assert!(kept(), "still its registry's");
    entry(&ctime(start + 3600));
    sweep();
    assert!(!kept(), "its start time changed: dropped at once");
    entry(&ctime(start));
    keep();
    std::fs::remove_file(config.join("sessions/4242.json")).unwrap();
    sweep();
    assert!(!kept(), "its registry entry gone: dropped at once");
}

/// L7: keyless lines are limited by their peer before any attestation:
/// past 30 in 10 s the answer is Limit, not another attempt.
#[test]
fn a_looping_keyless_caller_is_limited_before_it_is_attested() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = lab(dir.path());
    let peer = juggler_platform::local_ipc::Peer {
        pid: Some(4_000_000),
        pid_version: Some(1),
        accepted: std::time::SystemTime::now(),
    };
    let line =
        serde_json::to_string(&Request::new(String::new(), Call::Whoami, Args::default())).unwrap();
    let answers: Vec<_> = (0..31)
        .map(|_| super::calls::handle_line_from(&state, &line, Some(peer)).refused)
        .collect();
    assert!(
        answers[..30]
            .iter()
            .all(|refusal| *refusal == Some(Refusal::Unattested))
    );
    assert_eq!(
        answers[30],
        Some(Refusal::Limit),
        "the 31st isn't attested at all"
    );
    // Another peer has its own allowance.
    let other = juggler_platform::local_ipc::Peer {
        pid: Some(4_000_001),
        ..peer
    };
    assert_eq!(
        super::calls::handle_line_from(&state, &line, Some(other)).refused,
        Some(Refusal::Unattested)
    );
    // A connector hook's keyless report is limited the same way.
    let report = serde_json::to_string(&juggler_agentbridge::wire::Report {
        v: juggler_agentbridge::wire::VERSION,
        token: String::new(),
        hook: juggler_agentbridge::wire::HookReport {
            event: "UserPromptSubmit".into(),
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(
        super::calls::handle_line_from(&state, &report, Some(peer)).refused,
        Some(Refusal::Limit)
    );
}

/// M2: 40 connectors wait at once; a hostile caller loops (unattested,
/// and as a session that already waits): its work stays bounded by the
/// peer limit and one wait a session, no connector is refused, and a
/// member a message is for still wakes.
#[test]
fn forty_connectors_and_a_hostile_loop_stay_bounded_and_members_still_wake() {
    use juggler_agentbridge::wire::{Refusal, Wait};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = lab(dir.path());
    {
        let storage = state.storage.lock().unwrap();
        storage
            .agent_access()
            .set_level(&pool, AccessLevel::Full, &std::collections::BTreeSet::new())
            .unwrap();
        for n in 0..40 {
            storage
                .pools()
                .join(
                    &pool,
                    &MemberRef::Conversation {
                        folder: FOLDER.into(),
                        id: conversation(n),
                    },
                    &[],
                    PoolRole::Member,
                    None,
                )
                .unwrap();
            storage.attested().put(facts_of(n));
        }
    }
    let asked: Arc<Mutex<HashMap<u32, usize>>> = Arc::default();
    let attestor: super::desktop::Attestor = {
        let asked = Arc::clone(&asked);
        Arc::new(move |_state, peer| {
            let pid = peer.pid.unwrap_or_default();
            *asked.lock().unwrap().entry(pid).or_default() += 1;
            if !(1000..1040).contains(&pid) {
                return Err("not a desktop session".to_owned());
            }
            let n = pid - 1000;
            Ok(super::attest::DesktopCaller {
                session: desktop_id(FOLDER, &conversation(n)),
                app_id: "app:claude-app:/x".into(),
                folder: FOLDER.into(),
                conversation: conversation(n),
                tool_pid: 1,
                can_wake: true,
                facts: facts_of(n),
            })
        })
    };
    let peer = |pid: u32| juggler_platform::local_ipc::Peer {
        pid: Some(pid),
        pid_version: Some(1),
        accepted: std::time::SystemTime::now(),
    };
    let wait = Wait {
        v: juggler_agentbridge::wire::VERSION,
        token: String::new(),
        wait_secs: 3,
    };
    let look = Duration::from_millis(50);
    tauri::async_runtime::block_on(async {
        let waiting: Vec<_> = (1000..1040)
            .map(|pid| {
                tauri::async_runtime::spawn(super::desktop::wait_with(
                    Arc::clone(&state),
                    wait.clone(),
                    peer(pid),
                    Arc::clone(&attestor),
                    look,
                ))
            })
            .collect();
        tokio::time::sleep(Duration::from_millis(300)).await;
        // A hostile loop, unattested: limited by its peer before more
        // attestations than the limit allows.
        let mut limited = 0;
        for _ in 0..200 {
            let answer = super::desktop::wait_with(
                Arc::clone(&state),
                wait.clone(),
                peer(9_999),
                Arc::clone(&attestor),
                look,
            )
            .await;
            assert!(answer.result.is_none(), "{answer:?}");
            limited += usize::from(answer.refused == Some(Refusal::Limit));
        }
        assert_eq!(limited, 170, "past 30 in 10 s: Limit");
        assert_eq!(asked.lock().unwrap()[&9_999], 30, "at most 30 attestations");
        // The same, as a session that already waits: never a second wait.
        for _ in 0..50 {
            let again = super::desktop::wait_with(
                Arc::clone(&state),
                wait.clone(),
                peer(1_007),
                Arc::clone(&attestor),
                look,
            )
            .await;
            assert_eq!(again.refused, Some(Refusal::Limit));
        }
        // A message for member 7 wakes it while the others wait on.
        message_for(&state, &pool, "m-7", &conversation(7));
        let mut answers = Vec::new();
        for handle in waiting {
            answers.push(handle.await.unwrap());
        }
        for (n, answer) in answers.iter().enumerate() {
            assert!(
                answer.refused.is_none(),
                "connector {n} refused: {answer:?}"
            );
            let wake = &answer.result.as_ref().unwrap()["wake"];
            if n == 7 {
                assert!(wake.as_str().unwrap().contains("Planner"), "{wake}");
            } else {
                assert!(wake.is_null(), "connector {n}: {wake}");
            }
        }
    });
}

/// Desktop member `n`'s conversation.
fn conversation(n: u32) -> String {
    format!("conv-{n}")
}

/// Desktop member `n`'s facts.
fn facts_of(n: u32) -> SessionFacts {
    SessionFacts {
        conversation: Some((FOLDER.into(), conversation(n))),
        ..SessionFacts::bare(desktop_id(FOLDER, &conversation(n)), None)
    }
}

/// An agent's message from the terminal member to desktop member
/// `conversation`, unread.
fn message_for(state: &Arc<AppState>, pool: &str, id: &str, conversation: &str) {
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
