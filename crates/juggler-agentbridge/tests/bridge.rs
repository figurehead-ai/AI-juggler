//! AJ-E1704: the agent bridge, end to end, on fixtures only.
//!
//! - A fake tool (the `fake-harness`, launched by the app's own launch
//!   path) reads its per-launch `--mcp-config` file, starts the **real**
//!   `juggler-agent --mcp` with only the file's environment, and speaks
//!   MCP to it; the bridge calls the real app core over its socket.
//! - The key is in the launch's environment, never its arguments or a
//!   file; nothing is written in the account's folder; the launch's file
//!   is in AI Juggler's own overlay folder, `0600`.
//! - The socket is `0600` in a `0700` folder; another key, an ended
//!   session or a switched-off project is refused.
//! - Every call goes through `allowed(pool, action)` and is audited;
//!   each level refuses what it doesn't allow, in words.
//! - A call answers in under 50 ms.
//! - The ledger knows the program as an MCP server: never cleaned up.
//!
//! Nothing real is launched or read: the tool is the fake, the folders
//! are temporary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::{AdapterRegistry, LaunchSpec};
use juggler_agentbridge::wire::{self, Call, Refusal, Request, Response};
use juggler_app::features::agent_bridge::{self, Prepared, calls, settings};
use juggler_app::features::pools;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, ProjectId, SessionId, SessionStatus, SettingsOverlay, Timestamp,
};

const BRIDGE: &str = env!("CARGO_BIN_EXE_juggler-agent");

/// The built `fake-harness` (built with the workspace).
fn fake_harness() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let bin = target.join("debug").join("fake-harness");
    assert!(
        bin.exists(),
        "build it first: cargo build -p juggler-adapters --bin fake-harness"
    );
    bin
}

struct World {
    _dir: tempfile::TempDir,
    root: PathBuf,
    state: Arc<AppState>,
    project: ProjectId,
    account_dir: PathBuf,
}

fn world(env: &[(&str, &str)]) -> World {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::FakeHarnessAdapter::new(
        fake_harness(),
    )));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    state.agent_bridge.set_program(Some(PathBuf::from(BRIDGE)));
    let account_dir = root.join("fake-work");
    let project_root = root.join("shop");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-work"),
                harness: HarnessId::new("fake"),
                label: "Work Max".into(),
                plan: Some(juggler_core::model::PlanTier::new("Max 20x plan")),
                state_dir: account_dir.clone(),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
                in_place: false,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-work"),
                name: "Main".into(),
                account: AccountId::new("acc-work"),
                settings_overlay: SettingsOverlay {
                    model: None,
                    env: env
                        .iter()
                        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                        .collect(),
                    restart_on_crash: None,
                },
                default_cwd: None,
            })
            .unwrap();
        storage
            .projects()
            .touch(&project_root, Some("shop"))
            .unwrap()
            .id
    };
    World {
        _dir: dir,
        root,
        state,
        project,
        account_dir,
    }
}

/// A session row, as if launched (no process).
fn session(world: &World, id: &str, status: SessionStatus) -> SessionId {
    let session = SessionId::new(id);
    world
        .state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: ProfileId::new("rec-work"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(world.project.clone()),
            cwd: world.root.join("shop"),
            status,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
    session
}

fn spec() -> LaunchSpec {
    juggler_adapter::launch::interactive_spec(
        PathBuf::from("/bin/true"),
        vec!["run".into()],
        BTreeMap::new(),
        PathBuf::from("/"),
    )
}

/// Prepares a launch for `session` the way the app does; its spec.
fn prepare(world: &World, session: &SessionId) -> (Prepared, LaunchSpec) {
    let adapter = juggler_adapters::FakeHarnessAdapter::new(fake_harness());
    let mut spec = spec();
    let prepared = agent_bridge::prepare(
        &world.state,
        &adapter,
        &mut spec,
        session,
        Some(world.project.as_str()),
        "acc-work",
    );
    (prepared, spec)
}

fn token(spec: &LaunchSpec) -> String {
    spec.env.get(wire::TOKEN_ENV).cloned().unwrap()
}

fn ask(world: &World, token: &str, call: Call, text: Option<&str>) -> Response {
    let request = Request {
        v: wire::VERSION,
        token: token.to_owned(),
        call,
        text: text.map(str::to_owned),
    };
    calls::handle(&world.state, &serde_json::to_string(&request).unwrap())
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            if entry.file_type().unwrap().is_dir() {
                stack.push(entry.path());
            } else {
                out.push(entry.path());
            }
        }
    }
    out.sort();
    out
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path)
        .unwrap()
        .permissions()
        .mode()
        & 0o777
}

#[test]
fn nothing_is_added_before_the_question_is_answered() {
    let world = world(&[]);
    let session = session(&world, "s-1", SessionStatus::Running);
    let (prepared, spec) = prepare(&world, &session);
    assert!(matches!(prepared, Prepared::Not(_)), "{prepared:?}");
    assert_eq!(spec, self::spec(), "the launch is untouched");
    assert!(!world.root.join("app-data/agent").exists(), "no socket yet");
}

#[test]
fn the_key_is_in_the_environment_only_and_nothing_lands_in_the_account_folder() {
    let world = world(&[]);
    settings::answer_impl(&world.state, true).unwrap();
    std::fs::create_dir_all(&world.account_dir).unwrap();
    let before = files_under(&world.account_dir);
    let session = session(&world, "s-1", SessionStatus::Running);
    let (prepared, spec) = prepare(&world, &session);
    assert_eq!(prepared, Prepared::Added);
    let token = token(&spec);
    assert_eq!(token.len(), 64, "32 random bytes");
    assert!(
        !spec.args.iter().any(|arg| arg.contains(&token)),
        "never in argv: {:?}",
        spec.args
    );
    // The tool's file names the variable, never its value; it's ours, 0600.
    let file = world.root.join("app-data/overlays/agent/s-1/fake-mcp.json");
    assert!(spec.args.contains(&file.to_string_lossy().into_owned()));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(!text.contains(&token));
    assert!(text.contains("${JUGGLER_AGENT_TOKEN}"));
    assert_eq!(mode(&file), 0o600);
    assert_eq!(mode(file.parent().unwrap()), 0o700);
    // Only the hash is kept.
    let epochs = std::fs::read_to_string(world.root.join("app-data/agent/epochs.json")).unwrap();
    assert!(!epochs.contains(&token));
    assert!(epochs.contains(&agent_bridge::key_hash(&token)));
    for path in files_under(&world.root.join("app-data")) {
        if path
            .extension()
            .is_some_and(|ext| ext == "sqlite" || ext == "db")
            || path.to_string_lossy().contains("-wal")
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_default();
        assert!(
            !String::from_utf8_lossy(&bytes).contains(&token),
            "{} holds the key",
            path.display()
        );
    }
    assert_eq!(
        files_under(&world.account_dir),
        before,
        "nothing written in the account's folder"
    );
}

#[test]
fn the_socket_is_0600_in_a_0700_folder_and_answers_only_current_keys() {
    let world = world(&[]);
    settings::answer_impl(&world.state, true).unwrap();
    let session = session(&world, "s-1", SessionStatus::Running);
    let (_, spec) = prepare(&world, &session);
    let socket = PathBuf::from(spec.env.get(wire::SOCKET_ENV).unwrap());
    assert_eq!(socket, world.root.join("app-data/agent/bridge.sock"));
    assert_eq!(mode(&socket), 0o600);
    assert_eq!(mode(socket.parent().unwrap()), 0o700);
    assert!(juggler_agentbridge::client::trusted_socket(&socket));
    let target = juggler_agentbridge::client::Target {
        socket: socket.clone(),
        token: token(&spec),
    };
    let me = juggler_agentbridge::client::call(&target, Call::Whoami, None).unwrap();
    assert_eq!(me["session_id"], "s-1");
    let stranger = juggler_agentbridge::client::Target {
        socket,
        token: "0".repeat(64),
    };
    assert!(matches!(
        juggler_agentbridge::client::call(&stranger, Call::Whoami, None),
        Err(juggler_agentbridge::client::CallError::Refused(
            Refusal::Unknown,
            _
        ))
    ));
}

#[test]
fn a_new_epoch_gets_a_new_key_and_the_old_one_stops() {
    let world = world(&[]);
    settings::answer_impl(&world.state, true).unwrap();
    let first = session(&world, "s-1", SessionStatus::Running);
    let (_, one) = prepare(&world, &first);
    let (_, two) = prepare(&world, &first);
    assert_ne!(token(&one), token(&two), "a resume renews it");
    assert_eq!(
        ask(&world, &token(&one), Call::Whoami, None).refused,
        Some(Refusal::Unknown)
    );
    assert!(
        ask(&world, &token(&two), Call::Whoami, None)
            .result
            .is_some()
    );
    // A switch: a new session with its own key; the old one ends.
    let switched = session(&world, "s-2", SessionStatus::Running);
    let (_, three) = prepare(&world, &switched);
    assert_ne!(token(&two), token(&three));
    {
        let storage = world.state.storage.lock().unwrap();
        let mut record = storage.sessions().get(&first).unwrap();
        record.status = SessionStatus::HandedOver;
        storage.sessions().update(&record).unwrap();
    }
    assert_eq!(
        ask(&world, &token(&two), Call::Whoami, None).refused,
        Some(Refusal::Unknown),
        "an ended session's key"
    );
    assert!(
        ask(&world, &token(&three), Call::Whoami, None)
            .result
            .is_some()
    );
}

#[test]
fn each_level_refuses_what_it_doesnt_allow_and_every_call_is_audited() {
    let world = world(&[]);
    settings::answer_impl(&world.state, true).unwrap();
    let session = session(&world, "s-1", SessionStatus::Running);
    let (_, spec) = prepare(&world, &session);
    let key = token(&spec);
    let pool = juggler_core::storage::default_pool_id(Some(&world.project));
    let level = |name: &str, actions: &[&str]| {
        pools::access::set_level_impl(
            &world.state,
            &pool,
            name,
            &actions.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>(),
        )
        .unwrap();
    };

    // Work together (where every pool starts): reads and its own status.
    assert!(ask(&world, &key, Call::Whoami, None).result.is_some());
    let said = ask(&world, &key, Call::SetStatus, Some("Fixing checkout"));
    assert_eq!(said.result.unwrap()["says"], "Fixing checkout");
    let roster = ask(&world, &key, Call::PoolRoster, None).result.unwrap();
    assert_eq!(roster["members"][0]["says"], "Fixing checkout");
    assert_eq!(roster["members"][0]["you"], true);

    // Read: looking is fine; setting a status is the user's to allow.
    level("read", &[]);
    assert!(ask(&world, &key, Call::Cards, None).result.is_some());
    let refused = ask(&world, &key, Call::SetStatus, Some("x"));
    assert_eq!(refused.refused, Some(Refusal::Ask));
    assert!(
        refused
            .reason
            .unwrap()
            .contains("Only the user can allow it")
    );

    // Custom with nothing: even whoami asks.
    level("custom", &[]);
    assert_eq!(
        ask(&world, &key, Call::Whoami, None).refused,
        Some(Refusal::Ask)
    );
    level("custom", &["read_roster"]);
    assert!(ask(&world, &key, Call::PoolRoster, None).result.is_some());
    assert_eq!(
        ask(&world, &key, Call::Cards, None).refused,
        Some(Refusal::Ask)
    );

    // An observer only looks, whatever the level.
    level("full", &[]);
    pools::pool_set_role_impl(&world.state, session.as_str(), "observer").unwrap();
    assert!(ask(&world, &key, Call::Cards, None).result.is_some());
    assert_eq!(
        ask(&world, &key, Call::SetStatus, Some("x")).refused,
        Some(Refusal::Ask)
    );

    // Switched off for the project: refused at once, even mid-session.
    settings::set_impl(&world.state, "project", world.project.as_str(), false).unwrap();
    assert_eq!(
        ask(&world, &key, Call::Whoami, None).refused,
        Some(Refusal::Off)
    );
    settings::set_impl(&world.state, "project", world.project.as_str(), true).unwrap();

    // A request that names anything else isn't read at all.
    let sneaky = format!(r#"{{"v":1,"token":"{key}","call":"whoami","pool":"pool-other"}}"#);
    assert_eq!(
        calls::handle(&world.state, &sneaky).refused,
        Some(Refusal::Bad)
    );

    // Every call that reached the decision is in the audit.
    let audit = world
        .state
        .storage
        .lock()
        .unwrap()
        .agent_access()
        .audit(100)
        .unwrap();
    let ours: Vec<_> = audit
        .iter()
        .filter(|row| row.actor == "session:s-1")
        .collect();
    assert_eq!(ours.len(), 10, "{audit:?}");
    assert!(
        ours.iter()
            .any(|row| row.action == "set_status" && row.decision == "ask")
    );
    assert!(
        ours.iter()
            .any(|row| row.action == "read_cards" && row.decision == "allow")
    );
}

#[test]
fn agents_see_account_labels_never_emails_or_plans() {
    let world = world(&[]);
    settings::answer_impl(&world.state, true).unwrap();
    let session = session(&world, "s-1", SessionStatus::Running);
    let (_, spec) = prepare(&world, &session);
    let key = token(&spec);
    for call in [Call::Whoami, Call::PoolRoster, Call::Cards] {
        let text = serde_json::to_string(&ask(&world, &key, call, None)).unwrap();
        assert!(!text.contains("Max 20x"), "no plan: {text}");
        assert!(!text.contains("acc-work"), "no account id: {text}");
        assert!(!text.contains("fake-work"), "no folder: {text}");
        assert!(!text.contains('@'), "no email: {text}");
    }
    let roster = ask(&world, &key, Call::PoolRoster, None).result.unwrap();
    assert_eq!(roster["members"][0]["account"], "Work Max");
}

#[test]
fn calls_answer_in_under_50_ms() {
    let world = world(&[]);
    settings::answer_impl(&world.state, true).unwrap();
    for n in 0..8 {
        session(&world, &format!("s-{n}"), SessionStatus::Running);
    }
    let (_, spec) = prepare(&world, &SessionId::new("s-0"));
    let target = juggler_agentbridge::client::Target {
        socket: PathBuf::from(spec.env.get(wire::SOCKET_ENV).unwrap()),
        token: token(&spec),
    };
    let mut slowest = Duration::ZERO;
    for call in [
        Call::Whoami,
        Call::PoolRoster,
        Call::Cards,
        Call::Whoami,
        Call::PoolRoster,
    ] {
        let started = Instant::now();
        juggler_agentbridge::client::call(&target, call, None).unwrap();
        slowest = slowest.max(started.elapsed());
    }
    println!("slowest call over the socket: {slowest:?}");
    assert!(slowest < Duration::from_millis(50), "{slowest:?}");
}

#[test]
fn the_command_line_reads_the_key_from_its_environment_only() {
    let world = world(&[]);
    settings::answer_impl(&world.state, true).unwrap();
    let session = session(&world, "s-1", SessionStatus::Running);
    let (_, spec) = prepare(&world, &session);
    let run = |args: &[&str], with_key: bool| {
        let mut command = Command::new(BRIDGE);
        command.args(args).env_clear();
        if with_key {
            command
                .env(wire::TOKEN_ENV, token(&spec))
                .env(wire::SOCKET_ENV, spec.env.get(wire::SOCKET_ENV).unwrap());
        }
        command.output().unwrap()
    };
    let whoami = run(&["whoami"], true);
    assert!(whoami.status.success());
    let me: serde_json::Value = serde_json::from_slice(&whoami.stdout).unwrap();
    assert_eq!(me["session_id"], "s-1");
    let without = run(&["whoami"], false);
    assert_eq!(without.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&without.stderr).contains("no key"));
    let passed = run(&["whoami", "--token", &token(&spec)], false);
    assert_eq!(passed.status.code(), Some(64), "no option takes a key");
    let pool = juggler_core::storage::default_pool_id(Some(&world.project));
    pools::access::set_level_impl(&world.state, &pool, "read", &[]).unwrap();
    let refused = run(&["status", "hello"], true);
    assert_eq!(refused.status.code(), Some(2), "refused by the level");
}

#[test]
fn a_fake_tool_speaks_mcp_to_the_real_binary_through_the_app_launch() {
    // The fake waits before its first call, as a model would.
    let world = world(&[("FAKE_HARNESS_MCP_DELAY_MS", "300")]);
    settings::answer_impl(&world.state, true).unwrap();
    std::fs::create_dir_all(&world.account_dir).unwrap();
    let cwd = world.root.join("shop");
    let launched = juggler_app::commands::launch_profile(
        &world.state,
        "rec-work",
        Some(cwd.to_string_lossy().into_owned()),
    )
    .unwrap();
    let log = cwd.join(".fake-mcp.jsonl");
    let deadline = Instant::now() + Duration::from_secs(20);
    let lines = loop {
        let text = std::fs::read_to_string(&log).unwrap_or_default();
        if text.lines().count() >= 7 || Instant::now() > deadline {
            break text;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let entries: Vec<serde_json::Value> = lines
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(entries.len(), 7, "{lines}");
    let reply = |id: i64| {
        entries
            .iter()
            .find(|entry| entry["request"]["id"] == id)
            .map(|entry| entry["reply"].clone())
            .unwrap()
    };
    assert_eq!(reply(1)["result"]["serverInfo"]["name"], "juggler");
    assert_eq!(reply(2)["result"]["tools"].as_array().unwrap().len(), 3);
    let me = &reply(3)["result"];
    assert_eq!(me["isError"], false, "{me}");
    assert_eq!(
        me["structuredContent"]["session_id"],
        launched.session_id.as_str()
    );
    assert_eq!(
        me["structuredContent"]["pool"]["id"],
        juggler_core::storage::default_pool_id(Some(&world.project))
    );
    assert_eq!(reply(4)["result"]["isError"], false);
    let roster = &reply(5)["result"]["structuredContent"];
    assert_eq!(roster["members"][0]["says"], "Writing the fake's turns");
    assert!(
        reply(6)["result"]["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Writing the fake's turns")
    );
    for entry in &entries {
        let method = entry["request"]["method"].as_str().unwrap();
        if method == "tools/call" || method == "resources/read" {
            assert!(entry["micros"].as_u64().unwrap() < 50_000, "{entry}");
        }
    }
    // The card the user sees says it too.
    let pool = juggler_core::storage::default_pool_id(Some(&world.project));
    let cards = pools::cards::cards_impl(&world.state, &pool).unwrap();
    assert_eq!(
        cards.cards[0].said.value.as_deref(),
        Some("Writing the fake's turns")
    );
    // Nothing of AI Juggler's in the account's folder: only the fake's
    // own transcripts.
    for path in files_under(&world.account_dir) {
        assert!(
            path.starts_with(world.account_dir.join("transcripts")),
            "{}",
            path.display()
        );
    }
}

#[test]
fn the_ledger_knows_the_bridge_as_an_mcp_server_and_never_cleans_it_up() {
    let role = juggler_app::commands::guess_role(
        "/Applications/AI Juggler.app/Contents/MacOS/juggler-agent",
        "/Applications/AI Juggler.app/Contents/MacOS/juggler-agent --mcp",
    );
    assert_eq!(role, "McpServer");
    assert_eq!(
        juggler_app::commands::guess_role(
            "(unattributed)",
            "/Applications/AI Juggler Nightly.app/Contents/MacOS/juggler-agent --mcp"
        ),
        "McpServer"
    );
    for (clean, tier1, tier2) in [(Some(true), true, true), (Some(false), true, true)] {
        let verdict =
            juggler_supervisor::tree::evaluate_tier(&juggler_supervisor::tree::TierCandidate {
                pid: 42,
                role_guess: role.to_owned(),
                is_root: false,
                state: "orphaned".into(),
                escaped_group: false,
                root_exit_clean: clean,
                machine_local: true,
                tier1_opt_in: tier1,
                tier2_opt_in: tier2,
            });
        assert!(
            matches!(
                verdict,
                juggler_supervisor::tree::TierVerdict::ObserveOnly(_)
            ),
            "{verdict:?}"
        );
    }
}

#[test]
fn what_agents_reach_has_no_path_to_a_session_a_setting_or_a_secret() {
    // D-28's hard lines: agents never change pools, levels or rules, never
    // start a session or pick an account, and never get credentials. The
    // code an agent's call runs through (the socket and the calls) names
    // none of what would do it.
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../juggler-app/src/features/agent_bridge");
    let forbidden = [
        "supervisor",
        "write_input",
        "launch_profile",
        "set_level",
        "set_delivery",
        "revoke",
        "always_allow",
        "allow_once",
        "answer_impl",
        "set_impl",
        "pool_create",
        "pool_rename",
        "pool_delete",
        "pool_move",
        "pool_set_role",
        "pool_set_spans_macs",
        "keychain",
        "credential",
        "quota",
    ];
    for file in ["calls.rs", "server.rs"] {
        let code: String = std::fs::read_to_string(dir.join(file))
            .unwrap()
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        for word in forbidden {
            assert!(!code.contains(word), "{file} names {word}");
        }
    }
}
