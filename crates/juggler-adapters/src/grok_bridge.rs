//! AJ-E3002 R3: Grok Build takes AI Juggler's agent tools from a
//! standing entry in a home AI Juggler made for the Account, plus a
//! per-launch allow rule for each tool. Never the user's own home.
//!
//! - Grok reads MCP servers only from its home's `config.toml` (and a
//!   project's `.grok/config.toml`); its per-launch overlay
//!   (`GROK_CONFIG`) is allowlisted and adds none (lab 2026-11-09, Grok
//!   Build 1.0.77: "No MCP servers configured"). So a managed home keeps
//!   `[mcp_servers.<server>]`: the bridge program with `--mcp`, and
//!   `env` naming the bridge's variables as `${NAME:-}`, which Grok fills
//!   in from its own environment when it loads the entry. No value is
//!   written.
//! - In a launch without the tools those variables are empty: the server
//!   still starts and lists its tools (Grok defers them behind
//!   `search_tool`), and each call answers, in words, that this session
//!   wasn't started with AI Juggler's agent tools. Nothing reaches the
//!   app without a key, and Grok shows a healthy server, never a failing
//!   one (an empty tool list would read as failing in `grok mcp
//!   doctor`, lab 2026-11-09).
//! - `--allow MCPTool(<server>__<tool>)` per tool (Grok names an MCP tool
//!   `<server>__<tool>`; the flag works in the TUI and headless, and is
//!   always enforced), and only them, so those calls never ask. They go
//!   before the `--` that ends Grok's options (the message follows it).
//!
//! Lab 2026-11-09 (throwaway homes, `live_tools_probe`): `grok mcp
//! doctor` loads such an entry, expands `${NAME:-}`, and reports the
//! handshake and all the tools, with and without the key.
//! Unverified until a signed-in run: that a call then runs without a
//! prompt.

use std::path::Path;

use juggler_adapter::LaunchSpec;
use juggler_adapter::agent_bridge::{BridgeInjection, BridgeLaunch, HomeEntry};

/// The home's settings file.
pub const CONFIG_FILE: &str = "config.toml";
/// The bridge's arguments in the standing entry.
pub const STANDING_ARGS: [&str; 1] = ["--mcp"];

/// Grok's name for an MCP server's tool.
#[must_use]
pub fn tool_name(server: &str, tool: &str) -> String {
    format!("{server}__{tool}")
}

/// The standing entry for `bridge`'s server.
#[must_use]
pub fn entry(bridge: &BridgeLaunch) -> toml_edit::Table {
    let mut env = toml_edit::InlineTable::new();
    for name in &bridge.env_names {
        env.insert(name, format!("${{{name}:-}}").into());
    }
    let mut args = toml_edit::Array::new();
    for arg in STANDING_ARGS {
        args.push(arg);
    }
    let mut table = toml_edit::Table::new();
    table.insert(
        "command",
        toml_edit::value(bridge.program.to_string_lossy().into_owned()),
    );
    table.insert("args", toml_edit::value(args));
    table.insert("env", toml_edit::value(env));
    table.insert("enabled", toml_edit::value(true));
    table
}

/// Keeps the standing entry in `home` (a home AI Juggler made: the caller
/// checks) and returns the launch's allow rules.
///
/// # Errors
///
/// Why the entry couldn't be kept, in words.
pub fn injection(
    bridge: &BridgeLaunch,
    launch: &LaunchSpec,
    home: &Path,
) -> Result<BridgeInjection, String> {
    let file = home.join(CONFIG_FILE);
    let table = entry(bridge);
    let key = format!("mcp_servers.{}", bridge.server);
    crate::config_edit::keep_table(&file, &key, &table)
        .map_err(|error| format!("its home's {CONFIG_FILE} couldn't take the server: {error}"))?;
    let args = bridge
        .tools
        .iter()
        .flat_map(|tool| {
            [
                "--allow".to_owned(),
                format!("MCPTool({})", tool_name(&bridge.server, tool)),
            ]
        })
        .collect();
    Ok(BridgeInjection {
        at: launch
            .args
            .iter()
            .position(|arg| arg == "--")
            .unwrap_or(launch.args.len()),
        args,
        home: Some(HomeEntry {
            file,
            entry: format!("[{key}]\n{table}"),
        }),
        ..BridgeInjection::default()
    })
}

#[cfg(test)]
mod tests {
    use super::{CONFIG_FILE, injection};
    use juggler_adapter::LaunchSpec;
    use juggler_adapter::agent_bridge::BridgeLaunch;
    use std::path::{Path, PathBuf};

    fn bridge(home: &Path) -> BridgeLaunch {
        BridgeLaunch {
            program: PathBuf::from("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
            server: "juggler".into(),
            tools: vec!["whoami".into(), "pool_roster".into(), "set_status".into()],
            env_names: vec!["JUGGLER_AGENT_TOKEN".into(), "JUGGLER_AGENT_SOCKET".into()],
            dir: PathBuf::from("/data/overlays/agent/sess-1"),
            state_dir: home.to_path_buf(),
        }
    }

    fn launch(args: &[&str]) -> LaunchSpec {
        let mut env = std::collections::BTreeMap::new();
        env.insert("JUGGLER_AGENT_TOKEN".into(), "secret-key-value".into());
        LaunchSpec {
            program: PathBuf::from("/grok"),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            env,
            cwd: PathBuf::from("/p"),
            needs_pty: true,
        }
    }

    #[test]
    fn the_entry_names_the_variables_and_the_rules_allow_only_its_tools() {
        let home = tempfile::tempdir().unwrap();
        let spec = launch(&["--model", "grok-4", "--", "- fix it"]);
        let injection = injection(&bridge(home.path()), &spec, home.path()).unwrap();
        assert_eq!(
            injection.applied(&spec.args),
            [
                "--model",
                "grok-4",
                "--allow",
                "MCPTool(juggler__whoami)",
                "--allow",
                "MCPTool(juggler__pool_roster)",
                "--allow",
                "MCPTool(juggler__set_status)",
                "--",
                "- fix it",
            ]
        );
        assert!(injection.env.is_empty() && injection.files.is_empty());
        let written = std::fs::read_to_string(home.path().join(CONFIG_FILE)).unwrap();
        assert_eq!(
            written,
            "[mcp_servers.juggler]\n\
             command = \"/Applications/AI Juggler.app/Contents/MacOS/juggler-agent\"\n\
             args = [\"--mcp\"]\n\
             env = { JUGGLER_AGENT_TOKEN = \"${JUGGLER_AGENT_TOKEN:-}\", JUGGLER_AGENT_SOCKET = \"${JUGGLER_AGENT_SOCKET:-}\" }\n\
             enabled = true\n"
        );
        let home_entry = injection.home.unwrap();
        assert_eq!(home_entry.file, home.path().join(CONFIG_FILE));
        assert_eq!(home_entry.entry, written);
        assert!(!written.contains("secret-key-value"));
        // An interactive launch with no message: at the end.
        let plain = launch(&[]);
        assert_eq!(
            super::injection(&bridge(home.path()), &plain, home.path())
                .unwrap()
                .at,
            0
        );
    }

    #[test]
    fn the_users_other_settings_stay_and_a_second_launch_writes_nothing() {
        let home = tempfile::tempdir().unwrap();
        let file = home.path().join(CONFIG_FILE);
        let before = "# mine\n[models]\ndefault = \"grok-4\"\n\n[mcp_servers.docs]\nurl = \"https://d\" # keep\n";
        std::fs::write(&file, before).unwrap();
        injection(&bridge(home.path()), &launch(&[]), home.path()).unwrap();
        let once = std::fs::read_to_string(&file).unwrap();
        assert!(once.starts_with(before), "{once}");
        assert!(once.contains("[mcp_servers.juggler]"));
        let modified = std::fs::metadata(&file).unwrap().modified().unwrap();
        injection(&bridge(home.path()), &launch(&[]), home.path()).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), once);
        assert_eq!(
            std::fs::metadata(&file).unwrap().modified().unwrap(),
            modified
        );
        // A moved app: the entry follows it, the rest stays.
        let mut moved = bridge(home.path());
        moved.program = PathBuf::from("/Apps/AI Juggler.app/Contents/MacOS/juggler-agent");
        injection(&moved, &launch(&[]), home.path()).unwrap();
        let after = std::fs::read_to_string(&file).unwrap();
        assert!(after.starts_with(before) && after.contains("/Apps/AI Juggler.app"));
        assert_eq!(after.matches("[mcp_servers.juggler]").count(), 1);
    }

    #[test]
    fn a_home_that_isnt_toml_is_left_alone_and_says_why() {
        let home = tempfile::tempdir().unwrap();
        let file = home.path().join(CONFIG_FILE);
        std::fs::write(&file, "not = [toml").unwrap();
        let why = injection(&bridge(home.path()), &launch(&[]), home.path()).unwrap_err();
        assert!(why.contains(CONFIG_FILE), "{why}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "not = [toml");
    }
}
