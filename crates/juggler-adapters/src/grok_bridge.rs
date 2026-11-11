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
//!   `search_tool`); a call that got through would answer, in words,
//!   that this session wasn't started with AI Juggler's agent tools.
//!   Nothing reaches the app without a key, and Grok shows a healthy
//!   server, never a failing one (an empty tool list would read as
//!   failing in `grok mcp doctor`, lab 2026-11-09).
//! - `--allow MCPTool(<server>__<tool>)` per tool (Grok names an MCP tool
//!   `<server>__<tool>`; the flag works in the TUI and headless, and is
//!   always enforced), and only them, so those calls never ask. They go
//!   before the `--` that ends Grok's options (the message follows it).
//! - Without an allow rule Grok asks before each call, so a launch
//!   without the tools ([`without_tools`]) can't leave the entry as it is
//!   (the rc18 walk, E3002-R3 step 12). Grok 1.0.77 has no
//!   per-launch way to leave one configured server out: its overlay
//!   (`GROK_CONFIG`) drops `disabled_mcp_servers` and
//!   `mcp_servers.<server>.enabled`, and `--disallowed-tools` takes
//!   built-in tools only ("names no tool, so it removes nothing"). So the
//!   launch denies the server's tools, `--deny MCPTool(<server>__*)`
//!   (deny wins over every allow and ask, and never asks), and tells the
//!   agent why, and what to tell the user in plain words (Grok shows only
//!   its own rule string), with `--rules` (appended to its system
//!   prompt; the rc32 walk). Lab
//!   2026-11-09, throwaway home, a local stand-in model: the call comes
//!   back "Denied by permission policy" and never reaches the server
//!   (headless; the TUI by Grok's permissions guide, "any matching deny
//!   rejects", before an ask: on the live checklist).
//!
//! Lab 2026-11-09 (throwaway homes, `live_tools_probe`): `grok mcp
//! doctor` loads such an entry, expands `${NAME:-}`, and reports the
//! handshake and all the tools, with and without the key.
//! Unverified until a signed-in run: that a call then runs without a
//! prompt.

use std::path::Path;

use juggler_adapter::LaunchSpec;
use juggler_adapter::agent_bridge::{
    BridgeInjection, BridgeLaunch, BridgeOff, HomeEntry, SERVE_ARG,
};

/// The home's settings file.
pub const CONFIG_FILE: &str = "config.toml";
/// Grok's name for an MCP server's tool.
#[must_use]
pub fn tool_name(server: &str, tool: &str) -> String {
    format!("{server}__{tool}")
}

/// The standing entry for `bridge`'s server.
#[must_use]
pub fn entry(bridge: &BridgeLaunch) -> toml_edit::Table {
    let mut env = toml_edit::InlineTable::new();
    for (name, reference) in bridge.env_refs(|name| format!("${{{name}:-}}")) {
        env.insert(&name, reference.into());
    }
    let mut args = toml_edit::Array::new();
    args.push(SERVE_ARG);
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
        .tool_names(tool_name)
        .into_iter()
        .flat_map(|name| ["--allow".to_owned(), format!("MCPTool({name})")])
        .collect();
    Ok(BridgeInjection {
        at: options_end(launch),
        args,
        home: Some(HomeEntry {
            file,
            entry: format!("[{key}]\n{table}"),
        }),
        ..BridgeInjection::default()
    })
}

/// Where AI Juggler's options go: before the `--` that ends Grok's
/// options (the message follows it), or at the end.
fn options_end(launch: &LaunchSpec) -> usize {
    launch
        .args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(launch.args.len())
}

/// A launch without the tools, on a home that may hold the standing entry
/// (the caller checks it's one AI Juggler made): every tool of `off`'s
/// server denied, so none asks and none runs, and the agent told why.
#[must_use]
pub fn without_tools(off: &BridgeOff, launch: &LaunchSpec) -> BridgeInjection {
    BridgeInjection {
        at: options_end(launch),
        args: vec![
            "--deny".to_owned(),
            format!("MCPTool({})", tool_name(&off.server, "*")),
            "--rules".to_owned(),
            off.words(),
        ],
        ..BridgeInjection::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{CONFIG_FILE, injection};
    use juggler_adapter::LaunchSpec;
    use juggler_adapter::agent_bridge::BridgeLaunch;
    use std::path::{Path, PathBuf};

    fn bridge(home: &Path) -> BridgeLaunch {
        crate::common::bridge_sample::sample(home)
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

    /// AJ-E3002 R3 (the rc18 walk, step 12): a launch without the tools
    /// denies every tool of the server, allows none, and tells the agent
    /// why, before the `--` that ends Grok's options; nothing written.
    #[test]
    fn a_launch_without_the_tools_denies_them_and_says_why() {
        let home = tempfile::tempdir().unwrap();
        let sample = bridge(home.path());
        let off = juggler_adapter::BridgeOff {
            server: sample.server.clone(),
            tools: sample.tools.clone(),
            state_dir: home.path().to_path_buf(),
            why: "the user switched them off for this project".into(),
        };
        let spec = launch(&["--model", "grok-4", "--", "- fix it"]);
        let injection = super::without_tools(&off, &spec);
        let words = off.words();
        assert_eq!(
            injection.applied(&spec.args),
            [
                "--model",
                "grok-4",
                "--deny",
                "MCPTool(juggler__*)",
                "--rules",
                words.as_str(),
                "--",
                "- fix it",
            ]
        );
        assert!(words.contains("wasn't started with AI Juggler's agent tools"));
        assert!(words.contains("\"juggler\""));
        assert!(!injection.args.iter().any(|arg| arg == "--allow"));
        assert!(injection.env.is_empty() && injection.files.is_empty());
        assert!(injection.home.is_none());
        assert!(!injection.args.concat().contains("secret-key-value"));
        assert!(!injection.args.concat().contains("JUGGLER_AGENT_TOKEN"));
        assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
        // An interactive launch with no message: at the end.
        assert_eq!(super::without_tools(&off, &launch(&[])).at, 0);
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
