//! AJ-E1601 (D-34): accounts through another service (OpenCode Go, Kimi)
//! on Claude Code and Codex.
//!
//! The guardrails:
//! - the key is kept only in the Keychain (an in-memory store here: these
//!   tests never touch a real Keychain), never under the data folder or a
//!   profile's folder, in the database, a log or the spawn ledger;
//! - it reaches only that account's sessions, at launch, in the variable
//!   the tool reads, never their arguments (D-37: AI Juggler's own key
//!   reaches only the launch it starts);
//! - every session starts through the one spawn path that adds it;
//! - only AI Juggler's own profile folders are pointed at the service's
//!   published address;
//! - it goes with the account's last profile.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_account::{MemoryStore, SecretStore};
use juggler_adapter::endpoint::WireApi;
use juggler_adapter::{AdapterRegistry, LaunchMode};
use juggler_adapters::{ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands::{self, AccountInfo};
use juggler_app::error::AppResult;
use juggler_app::features::service_accounts::commands::{
    ServiceAccountRequest, ServiceProfileRequest, service_account_add_impl,
    service_account_set_key_impl, services_list_impl,
};
use juggler_app::features::service_accounts::links;
use juggler_app::features::sessions::spawn::launched_spec;
use juggler_app::manage::{self, CopyFolder};
use juggler_app::state::AppState;
use juggler_core::model::AccountId;

#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// A made-up key: never a real one.
const KEY: &str = "fixture-service-key-0123456789abcdef";

fn stub(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    stand_in::write(&path, "#!/bin/sh\nexit 0\n");
    path
}

fn app_state(dir: &Path) -> (Arc<AppState>, Arc<MemoryStore>) {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(stub(dir, "claude"))));
    registry.register(Arc::new(CodexAdapter::with_bin(stub(dir, "codex"))));
    let state = Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap());
    let store = Arc::new(MemoryStore::default());
    state.service_keys.install(store.clone());
    (state, store)
}

fn request(service: &str, key: &str, profiles: &[(&str, &str, &str)]) -> ServiceAccountRequest {
    ServiceAccountRequest {
        service: service.to_owned(),
        plan: None,
        key: key.to_owned(),
        profiles: profiles
            .iter()
            .map(|(tool, label, model)| ServiceProfileRequest {
                tool: (*tool).to_owned(),
                label: (*label).to_owned(),
                model: (*model).to_owned(),
            })
            .collect(),
    }
}

/// OpenCode Go on both tools.
fn add_go(state: &Arc<AppState>) -> AppResult<Vec<AccountInfo>> {
    let mut go = request(
        "opencode-go",
        &format!("  {KEY}\n"),
        &[
            ("claude-code", "Go", "minimax-m3"),
            ("codex", "Go Codex", "a-model"),
        ],
    );
    go.plan = Some("Go Plus".to_owned());
    service_account_add_impl(state, &go)
}

/// Every file under `dir` whose bytes hold `needle` (links not followed).
fn files_holding(dir: &Path, needle: &[u8]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).into_iter().flatten().flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file()
                && std::fs::read(&path)
                    .is_ok_and(|bytes| bytes.windows(needle.len()).any(|w| w == needle))
            {
                out.push(path);
            }
        }
    }
    out
}

fn account(state: &Arc<AppState>, id: &str) -> juggler_core::model::Account {
    commands::find_account_arc(state, id).unwrap()
}

fn recipe(state: &Arc<AppState>, id: &str) -> juggler_core::model::LaunchRecipe {
    state
        .storage
        .lock()
        .unwrap()
        .profiles()
        .list_by_account(&AccountId::new(id))
        .unwrap()
        .remove(0)
}

fn preset_endpoint(service: &str, wire: WireApi) -> juggler_adapter::endpoint::ServiceEndpoint {
    juggler_adapters::services::preset(service)
        .unwrap()
        .endpoint(wire)
        .unwrap()
        .clone()
}

#[test]
fn each_service_offers_the_tools_that_speak_its_protocol() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = app_state(dir.path());
    let services = services_list_impl(&state);
    let ids: Vec<&str> = services.iter().map(|s| s.service.id.as_str()).collect();
    assert_eq!(ids, ["opencode-go", "kimi"]);
    for service in &services {
        let mut tools: Vec<&str> = service.tools.iter().map(|t| t.tool.as_str()).collect();
        tools.sort_unstable();
        assert_eq!(tools, ["claude-code", "codex"], "{}", service.service.id);
    }
    assert_eq!(services[0].service.usage, "estimate");
    assert_eq!(services[1].service.usage, "unknown");
    // Nothing a key could be in: the list is the services' published facts.
    let text = format!("{:?}", services);
    assert!(!text.to_lowercase().contains("sk-"));
}

#[test]
fn an_account_through_another_service_keeps_its_key_only_in_the_keychain() {
    let dir = tempfile::tempdir().unwrap();
    let (state, store) = app_state(dir.path());
    let made = add_go(&state).unwrap();
    assert_eq!(made.len(), 2);
    for info in &made {
        assert_eq!(info.service.as_deref(), Some("OpenCode Go"));
        assert_eq!(info.login_state, "signed_in", "ready: its key is there");
        assert_eq!(info.plan.as_deref(), Some("Go Plus"));
    }
    // One account, one key, kept as pasted (trimmed) in the store.
    let linked = links(&state).unwrap();
    assert_eq!(linked.len(), 2);
    let accounts: std::collections::BTreeSet<&str> =
        linked.values().map(|link| link.account.as_str()).collect();
    assert_eq!(accounts.len(), 1);
    let id = *accounts.iter().next().unwrap();
    assert_eq!(store.get(id).as_deref(), Some(KEY));

    // Each profile's own folder points at the published address.
    let claude = made.iter().find(|i| i.provider == "claude-code").unwrap();
    let codex = made.iter().find(|i| i.provider == "codex").unwrap();
    let claude_dir = account(&state, &claude.id).state_dir;
    let codex_dir = account(&state, &codex.id).state_dir;
    assert!(claude_dir.starts_with(state.accounts_dir()));
    assert!(codex_dir.starts_with(state.accounts_dir()));
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(claude_dir.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        settings["env"]["ANTHROPIC_BASE_URL"],
        preset_endpoint("opencode-go", WireApi::Anthropic)
            .base_url
            .as_str()
    );
    assert_eq!(settings["env"]["ANTHROPIC_MODEL"], "minimax-m3");
    let config = std::fs::read_to_string(codex_dir.join("config.toml")).unwrap();
    assert!(config.contains(&preset_endpoint("opencode-go", WireApi::Responses).base_url));

    // The key is nowhere on disk: not the data folder, the profiles'
    // folders, the database or a log.
    assert_eq!(
        files_holding(dir.path(), KEY.as_bytes()),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn the_key_reaches_only_that_accounts_launch_and_never_its_arguments() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = app_state(dir.path());
    let made = add_go(&state).unwrap();
    let plain = commands::create_account(&state, "claude-code", "Work", None).unwrap();
    let cwd = dir.path().join("project");
    std::fs::create_dir_all(&cwd).unwrap();

    for (info, variable) in [
        (
            made.iter().find(|i| i.provider == "claude-code").unwrap(),
            "ANTHROPIC_AUTH_TOKEN",
        ),
        (
            made.iter().find(|i| i.provider == "codex").unwrap(),
            "OPENCODE_API_KEY",
        ),
    ] {
        let account = account(&state, &info.id);
        let adapter = state.registry.get(&account.harness).unwrap();
        let spec = adapter
            .launch_spec(
                &account,
                &recipe(&state, &info.id),
                &cwd,
                &LaunchMode::Interactive,
            )
            .unwrap();
        // What the adapter builds (and the ledger records) has no key.
        assert!(!spec.args.iter().any(|arg| arg.contains(KEY)));
        assert!(!spec.env.values().any(|value| value.contains(KEY)));
        // At launch, in the variable the service's guide names, and only
        // there.
        let launched = launched_spec(&state, &account, &spec).unwrap();
        assert_eq!(launched.env.get(variable).map(String::as_str), Some(KEY));
        assert_eq!(launched.args, spec.args);
        assert_eq!(
            launched
                .env
                .values()
                .filter(|value| value.contains(KEY))
                .count(),
            1
        );
    }
    // Another account's session never gets it.
    let other = account(&state, &plain.id);
    let adapter = state.registry.get(&other.harness).unwrap();
    let spec = adapter
        .launch_spec(
            &other,
            &juggler_core::model::LaunchRecipe {
                id: juggler_core::model::ProfileId::new("p"),
                name: "p".into(),
                account: other.id.clone(),
                settings_overlay: Default::default(),
                default_cwd: None,
            },
            &cwd,
            &LaunchMode::Interactive,
        )
        .unwrap();
    let launched = launched_spec(&state, &other, &spec).unwrap();
    assert!(!launched.env.values().any(|value| value.contains(KEY)));

    // A real start, through the app's own launch: nothing it records
    // holds the key.
    let claude = made.iter().find(|i| i.provider == "claude-code").unwrap();
    let started = commands::launch_profile(
        &state,
        recipe(&state, &claude.id).id.as_str(),
        Some(cwd.to_string_lossy().into_owned()),
    )
    .unwrap();
    assert!(started.pid > 0);
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(
        files_holding(dir.path(), KEY.as_bytes()),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn removing_the_last_profile_removes_the_key() {
    let dir = tempfile::tempdir().unwrap();
    let (state, store) = app_state(dir.path());
    let made = add_go(&state).unwrap();
    let id = links(&state)
        .unwrap()
        .values()
        .next()
        .unwrap()
        .account
        .clone();
    let keep = |_: &Path| Ok(());
    manage::account_remove_impl(&state, &made[0].id, CopyFolder::Keep, &keep).unwrap();
    assert_eq!(
        store.get(&id).as_deref(),
        Some(KEY),
        "its other profile still runs"
    );
    manage::account_remove_impl(&state, &made[1].id, CopyFolder::Keep, &keep).unwrap();
    assert_eq!(
        store.get(&id),
        None,
        "the account's last profile took its key"
    );
    assert!(links(&state).unwrap().is_empty());
}

#[test]
fn without_its_key_here_nothing_starts_until_it_is_pasted_again() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = app_state(dir.path());
    let made = add_go(&state).unwrap();
    // A Mac whose Keychain doesn't have it (a restored backup).
    state.service_keys.install(Arc::new(MemoryStore::default()));
    let claude = made.iter().find(|i| i.provider == "claude-code").unwrap();
    let account = account(&state, &claude.id);
    let spec = state
        .registry
        .get(&account.harness)
        .unwrap()
        .launch_spec(
            &account,
            &recipe(&state, &claude.id),
            dir.path(),
            &LaunchMode::Interactive,
        )
        .unwrap();
    let error = launched_spec(&state, &account, &spec)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("Keychain"), "{error}");
    assert!(!error.contains(KEY));
    let listed = commands::accounts_info(&state).unwrap();
    assert!(
        listed
            .iter()
            .filter(|info| info.service.is_some())
            .all(|info| info.login_state == "logged_out")
    );
    // Pasted again on its account: it starts.
    service_account_set_key_impl(&state, &claude.id, KEY).unwrap();
    assert!(launched_spec(&state, &account, &spec).is_ok());
}

#[test]
fn a_service_key_or_tool_that_wont_do_is_refused_and_nothing_is_made() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = app_state(dir.path());
    let refused = [
        request("no-such-service", KEY, &[("claude-code", "A", "m")]),
        request(
            "opencode-go",
            "has a space in it",
            &[("claude-code", "A", "m")],
        ),
        request("opencode-go", "short", &[("claude-code", "A", "m")]),
        request("opencode-go", KEY, &[]),
        request(
            "opencode-go",
            KEY,
            &[("claude-code", "A", "m"), ("claude-code", "B", "m")],
        ),
        request("opencode-go", KEY, &[("claude-code", "A", "two words")]),
        request("opencode-go", KEY, &[("claude-code", "", "m")]),
        request("opencode-go", KEY, &[("not-a-tool", "A", "m")]),
        request(
            "kimi",
            KEY,
            &[("claude-code", "A", "m"), ("codex", "a", "m")],
        ),
    ];
    for bad in refused {
        let error = service_account_add_impl(&state, &bad)
            .err()
            .unwrap()
            .to_string();
        assert!(!error.contains(KEY), "never said back: {error}");
    }
    let mut plan = request("opencode-go", KEY, &[("claude-code", "A", "m")]);
    plan.plan = Some("Platinum".into());
    assert!(service_account_add_impl(&state, &plan).is_err());
    assert!(commands::accounts_info(&state).unwrap().is_empty());
    assert!(links(&state).unwrap().is_empty());
    let folders = std::fs::read_dir(state.accounts_dir()).map_or(0, |dir| dir.count());
    assert_eq!(folders, 0, "no profile folder was made");
}

#[test]
fn the_accounts_page_shows_one_account_with_both_profiles() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = app_state(dir.path());
    let made = add_go(&state).unwrap();
    commands::create_account(&state, "claude-code", "Work", None).unwrap();
    let accounts = juggler_app::features::subscriptions::list_impl(&state).unwrap();
    let go = accounts.iter().find(|a| a.service.is_some()).unwrap();
    let mut ids: Vec<&str> = go.profiles.iter().map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    let mut want: Vec<&str> = made.iter().map(|i| i.id.as_str()).collect();
    want.sort_unstable();
    assert_eq!(ids, want);
    let service = go.service.as_ref().unwrap();
    assert_eq!(
        (service.name.as_str(), service.usage.as_str()),
        ("OpenCode Go", "estimate")
    );
    assert_eq!(go.plan.as_deref(), Some("Go Plus"));
    assert_eq!(go.state, "signed_in");
    assert!(accounts.iter().filter(|a| a.service.is_none()).count() >= 1);
}

#[test]
fn every_session_starts_through_the_one_spawn_path() {
    // The spawn path adds the key: a session started any other way would
    // start without it, or another account's could be given it.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    common::source::sources(&src, &mut files);
    let mut found = Vec::new();
    for path in files {
        let text = common::source::product(&std::fs::read_to_string(&path).unwrap());
        let flat: String = common::source::without_comments(&text)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let count = flat.matches(".supervisor.spawn(").count();
        if count > 0 {
            let rel = path
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            found.push((rel, count));
        }
    }
    found.sort();
    assert_eq!(
        found,
        [
            // The account sessions' one path.
            ("features/sessions/spawn.rs".to_owned(), 1),
            // The tool's own sign-in, in a terminal (no session, no key).
            ("terminal.rs".to_owned(), 1),
        ]
    );
}
