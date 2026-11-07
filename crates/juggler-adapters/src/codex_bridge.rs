//! AJ-E1704 (D-30): Codex takes AI Juggler's agent tools as `-c`
//! overrides on its command line (TOML values), never through the
//! user's `config.toml`:
//!
//! - `mcp_servers.juggler.command` and `.args`: the bridge program with
//!   `--mcp`;
//! - `.env_vars`: the names of the bridge's variables, which Codex
//!   forwards from its own environment (where AI Juggler put them). No
//!   value is on the command line;
//! - `.enabled_tools`: AI Juggler's tools, and only them;
//! - `.tools.<tool>.approval_mode = "approve"` for each of them, so the
//!   session never asks about them (the pool's level is the real check),
//!   with the server's default left at `"prompt"`.
//!
//! They go before the `--` that ends Codex's options (the prompt comes
//! after it). Unverified until the spike on spare accounts (AJ-E1701):
//! that `env_vars` passes the key, and that the per-tool approval holds.

use juggler_adapter::agent_bridge::{BridgeInjection, BridgeLaunch, toml_list, toml_string};

/// The injection for one launch with `args`.
#[must_use]
pub fn injection(bridge: &BridgeLaunch, args: &[String]) -> BridgeInjection {
    let key = |field: &str| format!("mcp_servers.{}.{field}", bridge.server);
    let mut settings = vec![
        format!(
            "{}={}",
            key("command"),
            toml_string(&bridge.program.to_string_lossy())
        ),
        format!("{}={}", key("args"), toml_list(&["--mcp".to_owned()])),
        format!("{}={}", key("env_vars"), toml_list(&bridge.env_names)),
        format!("{}={}", key("enabled_tools"), toml_list(&bridge.tools)),
        format!(
            "{}={}",
            key("default_tools_approval_mode"),
            toml_string("prompt")
        ),
    ];
    settings.extend(bridge.tools.iter().map(|tool| {
        format!(
            "{}={}",
            key(&format!("tools.{tool}.approval_mode")),
            toml_string("approve")
        )
    }));
    BridgeInjection {
        at: args
            .iter()
            .position(|arg| arg == "--")
            .unwrap_or(args.len()),
        args: settings
            .into_iter()
            .flat_map(|setting| ["-c".to_owned(), setting])
            .collect(),
        files: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::injection;
    use juggler_adapter::agent_bridge::BridgeLaunch;
    use std::path::PathBuf;

    fn bridge() -> BridgeLaunch {
        BridgeLaunch {
            program: PathBuf::from("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
            server: "juggler".into(),
            tools: vec!["whoami".into(), "pool_roster".into(), "set_status".into()],
            env_names: vec!["JUGGLER_AGENT_TOKEN".into(), "JUGGLER_AGENT_SOCKET".into()],
            dir: PathBuf::from("/data/overlays/agent/sess-1"),
        }
    }

    #[test]
    fn overrides_go_before_the_prompt_and_name_variables_not_values() {
        let args: Vec<String> = ["--no-daemon", "--", "- fix it"].map(String::from).to_vec();
        let applied = injection(&bridge(), &args).applied(&args);
        assert_eq!(applied.first().map(String::as_str), Some("--no-daemon"));
        assert_eq!(&applied[applied.len() - 2..], ["--", "- fix it"]);
        let settings: Vec<&str> = applied
            .windows(2)
            .filter(|pair| pair[0] == "-c")
            .map(|pair| pair[1].as_str())
            .collect();
        assert!(settings.contains(
            &r#"mcp_servers.juggler.command="/Applications/AI Juggler.app/Contents/MacOS/juggler-agent""#
        ));
        assert!(settings.contains(
            &r#"mcp_servers.juggler.env_vars=["JUGGLER_AGENT_TOKEN","JUGGLER_AGENT_SOCKET"]"#
        ));
        assert!(settings.contains(
            &r#"mcp_servers.juggler.enabled_tools=["whoami","pool_roster","set_status"]"#
        ));
        assert!(
            settings.contains(&r#"mcp_servers.juggler.tools.set_status.approval_mode="approve""#)
        );
        assert!(settings.contains(&r#"mcp_servers.juggler.default_tools_approval_mode="prompt""#));
        assert_eq!(
            settings
                .iter()
                .filter(|s| s.ends_with(r#"approval_mode="approve""#))
                .count(),
            3
        );
        // Interactive: no `--`, so at the end.
        let plain: Vec<String> = vec!["--no-daemon".into()];
        assert_eq!(injection(&bridge(), &plain).at, 1);
    }
}
