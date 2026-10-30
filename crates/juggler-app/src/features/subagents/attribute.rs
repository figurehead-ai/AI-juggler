//! AJ-E1520 (0.3): which subagent probably started a process — never
//! certain (a subagent has no process of its own: it runs inside its
//! tool's one process), so always said as "probably".
//!
//! A process in a session is matched to the latest command one of the
//! session's subagents ran shortly before it started (within
//! [`WINDOW_MS`] after, [`EARLY_MS`] before), when one of that command's
//! parts (split at `&&`, `||`, `;` and `|`) appears in the process's
//! command line. A part shorter than [`SHORTEST`] characters (`ls`,
//! `cd x`) is never enough on its own.

use std::sync::Arc;

use super::AgentCommand;

/// A process may start this long after the command asked for it.
pub const WINDOW_MS: i64 = 120_000;

/// A process's start may read a little before the command's time.
pub const EARLY_MS: i64 = 2_000;

/// The shortest command part that can match.
pub const SHORTEST: usize = 6;

/// The subagent (id, task) that probably started the process whose
/// command line is `cmdline`, started at `started_ms`.
#[must_use]
pub fn started_by(
    commands: Option<&Arc<Vec<AgentCommand>>>,
    cmdline: &str,
    started_ms: i64,
) -> Option<(String, String)> {
    let commands = commands?;
    let line = cmdline.trim();
    if line.is_empty() {
        return None;
    }
    commands
        .iter()
        .rev()
        .filter(|c| c.at_ms <= started_ms + EARLY_MS && started_ms - c.at_ms <= WINDOW_MS)
        .find(|c| parts(&c.command).any(|part| line.contains(part)))
        .map(|c| (c.id.clone(), c.title.clone()))
}

/// A command's parts worth matching.
fn parts(command: &str) -> impl Iterator<Item = &str> {
    command
        .split(['&', '|', ';', '\n'])
        .map(str::trim)
        .filter(|part| part.chars().count() >= SHORTEST && !part.starts_with("cd "))
        .map(|part| {
            // A long part: its start (a command line is often cut).
            let end = part.char_indices().nth(60).map_or(part.len(), |(at, _)| at);
            &part[..end]
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn ran(at: i64, command: &str, id: &str) -> AgentCommand {
        AgentCommand {
            at_ms: at,
            command: command.to_owned(),
            id: id.to_owned(),
            title: format!("Task {id}"),
        }
    }

    #[test]
    fn a_process_is_matched_to_the_latest_command_shortly_before_it() {
        let commands = Arc::new(vec![
            ran(1_000, "npm run dev", "a1"),
            ran(5_000, "cd web && npm run dev -- --port 5173", "a2"),
            ran(9_000, "ls", "a3"),
        ]);
        let by =
            |cmdline: &str, at: i64| started_by(Some(&commands), cmdline, at).map(|(id, _)| id);
        assert_eq!(
            by("npm run dev -- --port 5173", 6_000).as_deref(),
            Some("a2")
        );
        assert_eq!(by("npm run dev", 2_000).as_deref(), Some("a1"));
        assert_eq!(by("npm run dev", 500_000), None, "too long after");
        assert_eq!(by("ls -la", 9_500), None, "too short to say");
        assert_eq!(by("cargo test", 6_000), None);
        assert_eq!(started_by(None, "npm run dev", 2_000), None);
        assert_eq!(by("", 2_000), None);
    }
}
