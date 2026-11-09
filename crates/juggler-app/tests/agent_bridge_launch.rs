//! AJ-E3002 R3: a launch either gets the agent tools as its adapter
//! declares, or says why not, in words the launch's debug line carries
//! ([`agent_bridge::Prepared::why_not`]). The app never branches on a
//! tool: what a tool takes, or why it can't, is its adapter's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::PathBuf;
use std::sync::Arc;

use juggler_adapter::{AdapterRegistry, HarnessAdapter};
use juggler_adapters::{
    FAKE_HARNESS_ID, FakeHarnessAdapter, GrokBuildAdapter, KimiCodeAdapter, OpenCodeAdapter,
};
use juggler_agentbridge::wire;
use juggler_app::features::agent_bridge::{self, Prepared};
use juggler_app::state::AppState;
use juggler_core::model::SessionId;

struct Lab {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    account: String,
}

fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(FakeHarnessAdapter::new(PathBuf::from(
        "/bin/true",
    ))));
    let state = Arc::new(AppState::init_with_registry(dir.path().join("a"), registry).unwrap());
    state
        .agent_bridge
        .set_program(Some(PathBuf::from("/bin/echo")));
    agent_bridge::settings::answer_impl(&state, true).unwrap();
    let account =
        juggler_app::features::account::create_account(&state, FAKE_HARNESS_ID, "Lab", None)
            .unwrap()
            .id;
    Lab {
        _dir: dir,
        state,
        account,
    }
}

fn prepare(lab: &Lab, adapter: &dyn HarnessAdapter, session: &str) -> (Prepared, Vec<String>) {
    let mut spec = juggler_adapter::launch::interactive_spec(
        PathBuf::from("/bin/true"),
        vec!["--".into(), "hello".into()],
        std::collections::BTreeMap::new(),
        PathBuf::from("/"),
    );
    let prepared = agent_bridge::prepare(
        &lab.state,
        adapter,
        &mut spec,
        &SessionId::new(session),
        None,
        &lab.account,
    );
    assert!(
        spec.env.contains_key(wire::TOKEN_ENV),
        "the command line comes with every launch that isn't refused"
    );
    (prepared, spec.args)
}

#[test]
fn a_tool_that_cant_take_the_server_says_why_for_the_debug_line() {
    let lab = lab();
    let (prepared, args) = prepare(&lab, &KimiCodeAdapter::new(), "s-kimi");
    let Prepared::CommandLine(why) = &prepared else {
        panic!("{prepared:?}")
    };
    assert_ne!(why.trim(), "");
    assert_eq!(args, ["--", "hello"]);
    let (step, logged) = prepared.why_not().unwrap();
    assert_eq!(step, "agent_tools_command_line_only");
    assert_eq!(logged, why);
}

#[test]
fn tools_that_declare_the_bridge_get_it_and_log_nothing() {
    let lab = lab();
    // The Account's folder is one AI Juggler made (as preparing it for
    // Grok Build makes it): a home Grok's entry may go in.
    let folder = lab
        .state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .get(&juggler_core::model::AccountId::new(lab.account.as_str()))
        .unwrap()
        .state_dir;
    juggler_adapter::ToolSignIn::prepare_account(&GrokBuildAdapter::new(), &folder).unwrap();
    let (prepared, args) = prepare(&lab, &GrokBuildAdapter::new(), "s-grok");
    assert_eq!(prepared, Prepared::Added, "{args:?}");
    assert!(args.iter().any(|arg| arg == "MCPTool(juggler__whoami)"));
    assert_eq!(&args[args.len() - 2..], ["--", "hello"]);
    assert_eq!(prepared.why_not(), None);
    let kept = std::fs::read_to_string(folder.join("config.toml")).unwrap();
    assert!(kept.contains("[mcp_servers.juggler]"), "{kept}");
    let (prepared, _) = prepare(&lab, &OpenCodeAdapter::new(), "s-opencode");
    assert_eq!(prepared, Prepared::Added);
}

#[test]
fn a_refused_launch_says_why_too() {
    let lab = lab();
    agent_bridge::settings::answer_impl(&lab.state, false).unwrap();
    let mut spec = juggler_adapter::launch::interactive_spec(
        PathBuf::from("/bin/true"),
        vec![],
        std::collections::BTreeMap::new(),
        PathBuf::from("/"),
    );
    let prepared = agent_bridge::prepare(
        &lab.state,
        &OpenCodeAdapter::new(),
        &mut spec,
        &SessionId::new("s-off"),
        None,
        &lab.account,
    );
    let (step, why) = prepared.why_not().unwrap();
    assert_eq!(step, "no_agent_tools");
    assert_ne!(why.trim(), "");
    assert!(!spec.env.contains_key(wire::TOKEN_ENV));
}

/// AJ-E3002 R3: a member is labelled by what its App's launches get, in
/// its adapter's words: a Grok Build App on a home AI Juggler didn't make
/// shows no agent tools and says why; one AI Juggler set up shows them.
#[test]
fn an_app_says_whether_its_sessions_get_the_tools_before_any_launch() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        GrokBuildAdapter::with_bin("/bin/true").with_home(dir.path().join("home")),
    ));
    registry.register(Arc::new(KimiCodeAdapter::new()));
    let state = Arc::new(AppState::init_with_registry(dir.path().join("a"), registry).unwrap());
    let create = |tool: &str, label: &str| {
        juggler_app::features::account::create_account(&state, tool, label, None)
            .unwrap()
            .id
    };
    let managed = create(juggler_adapters::GROK_BUILD_ID, "Grok set up");
    let theirs = create(juggler_adapters::GROK_BUILD_ID, "Grok theirs");
    let kimi = create(juggler_adapters::KIMI_CODE_ID, "Kimi");
    // The second one's home is no longer AI Juggler's (as an App used
    // where it is would be).
    let folder = juggler_app::commands::find_account_arc(&state, &theirs)
        .unwrap()
        .state_dir;
    std::fs::remove_file(folder.join(juggler_adapters::claude_code::MANAGED_MARKER)).unwrap();
    let before = std::fs::read_dir(&folder).unwrap().count();

    let infos = juggler_app::features::account::accounts_info(&state).unwrap();
    let why = |id: &str| {
        infos
            .iter()
            .find(|info| info.id == id)
            .unwrap()
            .agent_tools_why
            .clone()
    };
    assert_eq!(why(&managed), None);
    let said = why(&theirs).expect("an App on a home AI Juggler didn't make says why");
    assert!(said.contains("didn't make"), "{said}");
    assert!(why(&kimi).is_some());
    assert_eq!(
        std::fs::read_dir(&folder).unwrap().count(),
        before,
        "asking wrote nothing"
    );
    // A task's first prompt points at the tools only where a launch gets them.
    agent_bridge::settings::answer_impl(&state, true).unwrap();
    state
        .agent_bridge
        .set_program(Some(PathBuf::from("/bin/echo")));
    assert!(!agent_bridge::tools_offered(&state, None, &theirs));
    assert!(!agent_bridge::tools_offered(&state, None, &kimi));
    assert_eq!(
        agent_bridge::tools_offered(&state, None, &managed),
        juggler_platform::local_ipc::SUPPORTED
    );
}
