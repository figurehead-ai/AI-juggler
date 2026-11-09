//! AJ-E3002 R3: OpenCode and Grok Build take AI Juggler's agent tools per
//! launch, with no prompt for them: OpenCode in its inline settings,
//! Grok from a standing entry in a home AI Juggler made (never the
//! user's own) plus per-launch allow rules.
//!
//! `live_tools_probe` runs the real tools (`opencode mcp list`, `grok mcp
//! doctor`) against throwaway homes when `AIJ_LIVE_TOOLS=1`; nothing that
//! needs a sign-in. `AIJ_LIVE_LAB` names the throwaway folder,
//! `AIJ_AGENT_BIN` the built `juggler-agent`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    clippy::print_stdout
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use juggler_adapter::agent_bridge::BridgeLaunch;
use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::{
    Capabilities, LaunchMode, LaunchSpec, ToolConfig, ToolIdentity, ToolLaunch, ToolSignIn,
};
use juggler_adapters::{
    GROK_BUILD_ID, GrokBuildAdapter, KimiCodeAdapter, OPENCODE_ID, OpenCodeAdapter,
};
use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};

const TOOLS: [&str; 3] = ["whoami", "pool_roster", "set_status"];

fn account(harness: &str, state_dir: &Path, in_place: bool) -> Account {
    Account {
        id: AccountId::new("acct"),
        harness: HarnessId::new(harness),
        label: "A".into(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place,
    }
}

fn profile(env: &[(&str, &str)], model: Option<&str>) -> Profile {
    Profile {
        id: ProfileId::new("p1"),
        name: "p1".into(),
        account: AccountId::new("acct"),
        settings_overlay: SettingsOverlay {
            env: env
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            model: model.map(str::to_owned),
            ..SettingsOverlay::default()
        },
        default_cwd: None,
    }
}

fn bridge(program: &Path, state_dir: &Path, tools: &[&str]) -> BridgeLaunch {
    BridgeLaunch {
        program: program.to_path_buf(),
        server: "juggler".into(),
        tools: tools.iter().map(|tool| (*tool).to_owned()).collect(),
        env_names: vec!["JUGGLER_AGENT_TOKEN".into(), "JUGGLER_AGENT_SOCKET".into()],
        dir: state_dir.join("../overlays/agent/s1"),
        state_dir: state_dir.to_path_buf(),
    }
}

/// Every file under `dir`, with its contents.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

#[test]
fn opencode_and_grok_declare_the_tools_and_kimi_still_doesnt() {
    assert!(
        OpenCodeAdapter::new()
            .capabilities()
            .contains(Capabilities::AGENT_BRIDGE)
    );
    assert!(
        GrokBuildAdapter::new()
            .capabilities()
            .contains(Capabilities::AGENT_BRIDGE)
    );
    assert!(
        !KimiCodeAdapter::new()
            .capabilities()
            .contains(Capabilities::AGENT_BRIDGE)
    );
}

#[test]
fn an_opencode_launch_keeps_its_model_and_gets_the_server_by_variable_names() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let bin = root.path().join("opencode");
    std::fs::write(&bin, "#!/bin/sh\n").unwrap();
    let adapter = OpenCodeAdapter::with_bin(&bin).with_home(&home);
    let dir = root.path().join("accounts/oc");
    adapter.prepare_account(&dir).unwrap();
    let before = tree(&dir);
    let acct = account(OPENCODE_ID, &dir, false);
    let mut spec = adapter
        .launch_spec(
            &acct,
            &profile(&[("JUGGLER_AGENT_TOKEN", "the-key")], Some("x/model")),
            root.path(),
            &LaunchMode::Interactive,
        )
        .unwrap();
    let program = root.path().join("juggler-agent");
    let injection = adapter
        .agent_bridge(&bridge(&program, &dir, &TOOLS), &spec)
        .unwrap();
    injection.apply(&mut spec);
    let settings: serde_json::Value =
        serde_json::from_str(&spec.env["OPENCODE_CONFIG_CONTENT"]).unwrap();
    assert_eq!(settings["model"], "x/model");
    assert_eq!(
        settings["mcp"]["juggler"]["command"],
        serde_json::json!([program.to_string_lossy(), "--mcp"])
    );
    let allowed: Vec<&String> = settings["permission"].as_object().unwrap().keys().collect();
    assert_eq!(
        allowed,
        [
            "juggler_pool_roster",
            "juggler_set_status",
            "juggler_whoami"
        ]
    );
    assert!(!spec.env["OPENCODE_CONFIG_CONTENT"].contains("the-key"));
    assert!(!spec.args.join(" ").contains("the-key"));
    // Nothing written anywhere.
    assert_eq!(tree(&dir), before);
    assert!(injection.files.is_empty() && injection.home.is_none());
}

#[test]
fn grok_keeps_its_entry_only_in_a_home_ai_juggler_made() {
    let root = tempfile::tempdir().unwrap();
    let user_home = root.path().join("home");
    let adapter = GrokBuildAdapter::with_bin(root.path().join("grok")).with_home(&user_home);
    let program = root.path().join("juggler-agent");
    let spec = LaunchSpec {
        program: root.path().join("grok"),
        args: vec!["--".into(), "hi".into()],
        env: BTreeMap::new(),
        cwd: root.path().to_path_buf(),
        needs_pty: true,
    };

    // Grok's own home, even with something that looks like ours in it.
    let default = user_home.join(".grok");
    std::fs::create_dir_all(&default).unwrap();
    std::fs::write(default.join("config.toml"), "[models]\ndefault = \"g\"\n").unwrap();
    std::fs::write(
        default.join(".ai-juggler-managed"),
        "managed-by: ai-juggler\n",
    )
    .unwrap();
    let before = tree(&default);
    let why = adapter
        .agent_bridge(&bridge(&program, &default, &TOOLS), &spec)
        .unwrap_err();
    assert!(why.contains("didn't make"), "{why}");
    assert_eq!(adapter.agent_tools_for(&default), Err(why));
    assert_eq!(tree(&default), before);

    // A home the user brought in place, elsewhere.
    let theirs = root.path().join("grok-work");
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(theirs.join("config.toml"), "# mine\n").unwrap();
    let before = tree(&theirs);
    assert!(
        adapter
            .agent_bridge(&bridge(&program, &theirs, &TOOLS), &spec)
            .is_err()
    );
    assert!(adapter.agent_tools_for(&theirs).is_err());
    assert_eq!(tree(&theirs), before);

    // A home AI Juggler made: the entry, and only AI Juggler's tools.
    let managed = root.path().join("accounts/g");
    adapter.prepare_account(&managed).unwrap();
    assert_eq!(adapter.agent_tools_for(&managed), Ok(()));
    assert!(
        !managed.join("config.toml").exists(),
        "asking writes nothing"
    );
    let injection = adapter
        .agent_bridge(&bridge(&program, &managed, &TOOLS), &spec)
        .unwrap();
    let allowed: Vec<&String> = injection
        .args
        .iter()
        .filter(|arg| *arg != "--allow")
        .collect();
    assert_eq!(
        allowed,
        [
            "MCPTool(juggler__whoami)",
            "MCPTool(juggler__pool_roster)",
            "MCPTool(juggler__set_status)"
        ]
    );
    let entry = std::fs::read_to_string(managed.join("config.toml")).unwrap();
    assert!(entry.contains("args = [\"--mcp\"]"), "{entry}");
    assert!(entry.contains("JUGGLER_AGENT_TOKEN = \"${JUGGLER_AGENT_TOKEN:-}\""));
}

#[test]
fn a_grok_launch_without_the_tools_leaves_a_working_server() {
    // No variable the standing entry names is required (`:-` defaults):
    // without a key the bridge still starts, and each call says why not
    // (juggler-agentbridge's own tests; `live_tools_probe` runs it).
    let root = tempfile::tempdir().unwrap();
    let adapter =
        GrokBuildAdapter::with_bin(root.path().join("grok")).with_home(root.path().join("home"));
    let managed = root.path().join("accounts/g");
    adapter.prepare_account(&managed).unwrap();
    let acct = account(GROK_BUILD_ID, &managed, false);
    let spec = adapter
        .launch_spec(
            &acct,
            &profile(&[], None),
            root.path(),
            &LaunchMode::Interactive,
        )
        .unwrap();
    adapter
        .agent_bridge(&bridge(&root.path().join("ja"), &managed, &TOOLS), &spec)
        .unwrap();
    let doc: toml::Table = std::fs::read_to_string(managed.join("config.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let server = &doc["mcp_servers"]["juggler"];
    assert_eq!(server["args"].as_array().unwrap().len(), 1);
    assert_eq!(server["args"][0].as_str(), Some("--mcp"));
    for (_, value) in server["env"].as_table().unwrap() {
        assert!(value.as_str().unwrap().ends_with(":-}"), "{value}");
    }
    assert_eq!(server["enabled"].as_bool(), Some(true));
    // A later launch without the tools changes nothing in the home.
    let before = tree(&managed);
    let _ = adapter
        .launch_spec(
            &acct,
            &profile(&[], None),
            root.path(),
            &LaunchMode::Interactive,
        )
        .unwrap();
    assert_eq!(tree(&managed), before);
}

// ---------------------------------------------------------------------------
// Live probe (opt-in)

/// `spec`'s environment over a clean one: only the lab's home, `PATH`
/// and what the launch sets.
fn command_for(spec: &LaunchSpec, home: &Path, args: &[&str]) -> std::process::Command {
    let mut command = std::process::Command::new("/usr/bin/perl");
    command
        .args(["-e", "alarm 60; exec @ARGV"])
        .arg(&spec.program)
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/opt/homebrew/bin")
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .current_dir(home);
    for (key, value) in &spec.env {
        if value != ENV_UNSET {
            command.env(key, value);
        }
    }
    command
}

fn run(mut command: std::process::Command) -> String {
    let out = command.output().unwrap();
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn live_tools_probe() {
    if std::env::var("AIJ_LIVE_TOOLS").as_deref() != Ok("1") {
        return;
    }
    let lab =
        PathBuf::from(std::env::var("AIJ_LIVE_LAB").expect("AIJ_LIVE_LAB: a throwaway folder"));
    let agent = PathBuf::from(std::env::var("AIJ_AGENT_BIN").expect("AIJ_AGENT_BIN"));
    let home = lab.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let tools: Vec<&str> = juggler_agentbridge_tools();
    let with_key = |spec: &mut LaunchSpec| {
        spec.env
            .insert("JUGGLER_AGENT_TOKEN".into(), "lab-probe-key".into());
        spec.env.insert(
            "JUGGLER_AGENT_SOCKET".into(),
            lab.join("no-app.sock").to_string_lossy().into_owned(),
        );
    };

    // OpenCode.
    let opencode = OpenCodeAdapter::with_bin("/opt/homebrew/bin/opencode").with_home(&home);
    let dir = lab.join("accounts/oc");
    opencode.prepare_account(&dir).unwrap();
    let acct = account(OPENCODE_ID, &dir, false);
    let mut spec = opencode
        .launch_spec(
            &acct,
            &profile(&[], Some("lab/model")),
            &home,
            &LaunchMode::Interactive,
        )
        .unwrap();
    with_key(&mut spec);
    opencode
        .agent_bridge(&bridge(&agent, &dir, &tools), &spec)
        .unwrap()
        .apply(&mut spec);
    let listed = run(command_for(&spec, &home, &["mcp", "list"]));
    println!("== opencode mcp list (with the tools)\n{listed}");
    assert!(
        listed.contains("juggler") && listed.contains("connected"),
        "{listed}"
    );
    let config = run(command_for(&spec, &home, &["debug", "config"]));
    let start = config.find('{').unwrap();
    let resolved: serde_json::Value = serde_json::from_str(&config[start..]).unwrap();
    println!(
        "== opencode debug config\nmodel = {}\npermission = {}",
        resolved["model"], resolved["permission"]
    );
    assert_eq!(resolved["model"], "lab/model");
    assert_eq!(resolved["permission"]["juggler_whoami"], "allow");

    // Grok Build.
    let grok_bin = home_dir().join(".grok/bin/grok");
    let grok = GrokBuildAdapter::with_bin(&grok_bin).with_home(&home);
    let dir = lab.join("accounts/grok");
    grok.prepare_account(&dir).unwrap();
    let acct = account(GROK_BUILD_ID, &dir, false);
    let plain = grok
        .launch_spec(&acct, &profile(&[], None), &home, &LaunchMode::Interactive)
        .unwrap();
    let mut spec = plain.clone();
    with_key(&mut spec);
    grok.agent_bridge(&bridge(&agent, &dir, &tools), &spec)
        .unwrap()
        .apply(&mut spec);
    let socket = lab.join("leader.sock").to_string_lossy().into_owned();
    let doctor = |spec: &LaunchSpec| {
        run(command_for(
            spec,
            &home,
            &["--leader-socket", &socket, "mcp", "doctor", "juggler"],
        ))
    };
    let on = doctor(&spec);
    println!("== grok mcp doctor juggler (with the tools)\n{on}");
    assert!(on.contains("handshake OK"), "{on}");
    assert!(
        on.contains(&format!("{} tools discovered", tools.len())),
        "{on}"
    );
    let off = doctor(&plain);
    println!("== grok mcp doctor juggler (a launch without the tools)\n{off}");
    assert!(
        off.contains("handshake OK") && off.contains("1 healthy"),
        "{off}"
    );
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("AIJ_REAL_HOME").unwrap_or_else(|_| std::env::var("HOME").unwrap()))
}

/// AI Juggler's tools, as the app passes them.
fn juggler_agentbridge_tools() -> Vec<&'static str> {
    vec![
        "whoami",
        "pool_roster",
        "set_status",
        "send_message",
        "read_inbox",
        "ack",
        "share_doc",
        "read_doc",
        "tasks_list",
        "task_get",
        "task_create",
        "task_claim",
        "task_update",
        "task_assign",
    ]
}
