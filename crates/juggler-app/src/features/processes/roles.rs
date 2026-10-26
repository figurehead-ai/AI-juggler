//! AJ-E2401: what a process probably is, in words. The vocabulary is the
//! cleanup's own ([`crate::commands::guess_role`], pass2 §5), read from the
//! program and its command line and always shown as a guess ("probably a
//! dev server"), plus three roles that are known, not guessed: the
//! session's own tool, and AI Juggler's hook and tools processes (never a
//! cleanup candidate).

use juggler_agentbridge::wire::PROGRAM as AGENT;

/// A process's role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// The session's own process: the tool itself.
    Tool,
    /// `juggler-agent hook …`: AI Juggler's hook, run by the tool for a
    /// moment on each event.
    Hook,
    /// `juggler-agent --mcp`: AI Juggler's tools for the session.
    Bridge,
    /// An MCP server.
    McpServer,
    /// A database.
    Database,
    /// A dev server.
    DevServer,
    /// A file watcher.
    Watcher,
    /// A build.
    Build,
    /// A shell.
    Shell,
    /// Anything else.
    Helper,
}

impl Role {
    /// The stable key the UI and the filters read.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Hook => "hook",
            Self::Bridge => "bridge",
            Self::McpServer => "mcp_server",
            Self::Database => "database",
            Self::DevServer => "dev_server",
            Self::Watcher => "watcher",
            Self::Build => "build",
            Self::Shell => "shell",
            Self::Helper => "helper",
        }
    }

    /// The role in words, as a row shows it.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Tool => "the session itself",
            Self::Hook => "AI Juggler hook",
            Self::Bridge => "AI Juggler's tools",
            Self::McpServer => "MCP server",
            Self::Database => "database",
            Self::DevServer => "dev server",
            Self::Watcher => "watcher",
            Self::Build => "build",
            Self::Shell => "shell",
            Self::Helper => "helper",
        }
    }

    /// Known for certain (the session's tool, AI Juggler's own), not a
    /// guess from the command line.
    #[must_use]
    pub const fn sure(self) -> bool {
        matches!(self, Self::Tool | Self::Hook | Self::Bridge)
    }

    /// AI Juggler's own processes: never offered for cleanup, never counted
    /// as left behind, hidden unless asked for.
    #[must_use]
    pub const fn ours_internal(self) -> bool {
        matches!(self, Self::Hook | Self::Bridge)
    }

    /// The cleanup vocabulary's name ([`crate::commands::guess_role`]),
    /// which the automatic rules decide on.
    #[must_use]
    pub const fn cleanup_word(self) -> &'static str {
        match self {
            Self::Tool => "Harness",
            Self::Hook | Self::Bridge | Self::McpServer => "McpServer",
            Self::Database => "Database",
            Self::DevServer => "DevServer",
            Self::Watcher => "Watcher",
            Self::Build => "BuildTool",
            Self::Shell => "TerminalShell",
            Self::Helper => "Unknown",
        }
    }
}

/// The program's file name (`/usr/local/bin/node` is `node`).
#[must_use]
pub fn program_name(exe_or_name: &str) -> &str {
    exe_or_name.rsplit('/').next().unwrap_or(exe_or_name)
}

/// What a process probably is, from its program (`exe`, or its name) and
/// command line. `session_root`: it is the session's own process.
#[must_use]
pub fn role_of(exe: &str, cmdline: &str, session_root: bool) -> Role {
    if session_root {
        return Role::Tool;
    }
    let program = program_name(exe);
    let first = cmdline.split_whitespace().next().map(program_name);
    if program == AGENT || first == Some(AGENT) {
        let words: Vec<&str> = cmdline.split_whitespace().collect();
        return if words.contains(&"hook") {
            Role::Hook
        } else {
            Role::Bridge
        };
    }
    match crate::commands::guess_role(exe, cmdline) {
        "McpServer" => Role::McpServer,
        "Database" => Role::Database,
        "DevServer" => Role::DevServer,
        "Watcher" => Role::Watcher,
        "BuildTool" => Role::Build,
        "TerminalShell" => Role::Shell,
        // The tool's own name on a helper: another copy of a tool.
        _ => Role::Helper,
    }
}

/// A process's short name for a row: the program, with the script it runs
/// when the program is an interpreter (`node (vite)`). Words of the command
/// line beyond the script are never shown here.
#[must_use]
pub fn short_name(name: &str, cmdline: Option<&str>) -> String {
    const INTERPRETERS: [&str; 8] = [
        "node", "python", "python3", "ruby", "bun", "deno", "java", "npx",
    ];
    let program = program_name(name);
    let Some(line) = cmdline else {
        return program.to_owned();
    };
    if !INTERPRETERS.contains(&program) {
        return program.to_owned();
    }
    let script = line
        .split_whitespace()
        .skip(1)
        .find(|word| !word.starts_with('-'))
        .map(program_name)
        .filter(|word| !word.is_empty());
    match script {
        Some(script) => {
            let script: String = script.chars().take(40).collect();
            format!("{program} ({script})")
        }
        None => program.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hook_and_the_bridge_are_ai_jugglers_own() {
        assert_eq!(
            role_of(
                "/x/juggler-agent",
                "/x/juggler-agent hook PostToolUse",
                false
            ),
            Role::Hook
        );
        assert_eq!(
            role_of("/x/juggler-agent", "/x/juggler-agent --mcp", false),
            Role::Bridge
        );
        assert!(Role::Hook.ours_internal() && Role::Bridge.ours_internal());
        assert!(!Role::DevServer.ours_internal());
    }

    #[test]
    fn roles_use_the_cleanup_vocabulary() {
        assert_eq!(
            role_of(
                "/usr/local/bin/node",
                "node node_modules/.bin/vite dev",
                false
            ),
            Role::DevServer
        );
        assert_eq!(
            role_of(
                "/usr/local/bin/node",
                "node esbuild --watch src/x.ts",
                false
            ),
            Role::DevServer
        );
        assert_eq!(
            role_of("/usr/bin/nodemon", "nodemon server.js", false),
            Role::Watcher
        );
        assert_eq!(
            role_of("/opt/homebrew/bin/postgres", "postgres -D /x", false),
            Role::Database
        );
        assert_eq!(
            role_of("/bin/zsh", "/bin/zsh -c npm test", false),
            Role::Shell
        );
        assert_eq!(role_of("/x/tool", "tool --flag", true), Role::Tool);
        assert_eq!(Role::DevServer.cleanup_word(), "DevServer");
        assert_eq!(Role::Watcher.words(), "watcher");
    }

    #[test]
    fn a_short_name_shows_the_script_of_an_interpreter_only() {
        assert_eq!(
            short_name(
                "node",
                Some("node /x/node_modules/.bin/vite dev --port 5173")
            ),
            "node (vite)"
        );
        assert_eq!(
            short_name("python3", Some("python3 -m http.server 8000")),
            "python3 (http.server)"
        );
        assert_eq!(
            short_name("/usr/bin/esbuild", Some("esbuild --watch")),
            "esbuild"
        );
        assert_eq!(short_name("node", None), "node");
    }
}
