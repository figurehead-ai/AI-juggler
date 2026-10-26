//! AJ-E1541: one line per tool call, and the task list, for a handover.
//!
//! A switch to another tool used to carry only what was said, so the new
//! session didn't know which files had been edited or which commands had
//! run, and the task list never arrived. Each tool's parser hands its tool
//! calls here: [`summarize`] says what a call did in a few words, and
//! [`task_list`] reads the task list a call set (Claude Code's `TodoWrite`,
//! Codex's `update_plan`), so the handover can say where the work stands.
//! Arguments are never copied whole: a command or a path, cut short.

use std::fmt::Write as _;

/// The most characters of a command or pattern kept in a summary.
const SHOWN: usize = 120;

/// One item of a task list, as the tool last set it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskItem {
    /// What the item says.
    pub text: String,
    /// Whether it's done.
    pub done: bool,
    /// Whether it's the one being worked on.
    pub in_progress: bool,
}

/// What one tool call did, in a few words: `Edit calc.py`, `ran
/// "python3 -m unittest"`, `searched for "def add"`.
#[must_use]
pub fn summarize(name: &str, input: &serde_json::Value) -> String {
    let field = |key: &str| input.get(key).and_then(serde_json::Value::as_str);
    let short = |text: &str| -> String {
        let one_line = text.replace(['\n', '\r'], " ");
        let mut cut: String = one_line.chars().take(SHOWN).collect();
        if one_line.chars().count() > SHOWN {
            cut.push('…');
        }
        cut
    };
    match name {
        "Read" | "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => {
            let path = field("file_path")
                .or_else(|| field("notebook_path"))
                .unwrap_or("a file");
            format!("{name} {}", tail_of_path(path))
        }
        "Bash" | "shell" | "exec_command" | "local_shell" => {
            let command = field("command")
                .map(str::to_owned)
                .or_else(|| {
                    // Codex's shell takes an argument vector.
                    input
                        .get("command")
                        .and_then(serde_json::Value::as_array)
                        .map(|parts| {
                            parts
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                })
                .unwrap_or_default();
            format!("ran \"{}\"", short(strip_shell_wrapper(&command)))
        }
        "Grep" => format!("searched for \"{}\"", short(field("pattern").unwrap_or(""))),
        "Glob" => format!("listed \"{}\"", short(field("pattern").unwrap_or(""))),
        "Task" | "Agent" => format!(
            "ran a subagent: {}",
            short(
                field("description")
                    .or_else(|| field("prompt"))
                    .unwrap_or("work")
            )
        ),
        "WebFetch" => format!("read {}", short(field("url").unwrap_or("a page"))),
        "WebSearch" => format!(
            "searched the web for \"{}\"",
            short(field("query").unwrap_or(""))
        ),
        "TodoWrite" | "update_plan" | "TaskCreate" | "TaskUpdate" => {
            "updated the task list".to_owned()
        }
        "apply_patch" => {
            let patch = field("input").or_else(|| field("patch")).unwrap_or("");
            let files = patched_files(patch);
            if files.is_empty() {
                "edited files".to_owned()
            } else {
                format!("edited {}", files.join(", "))
            }
        }
        other => format!("used {other}"),
    }
}

/// AJ-E1703: the files a tool call changed (written, edited or patched),
/// last two path parts each as [`summarize`] names them; none for a call
/// that changes no file (reading one isn't changing it).
#[must_use]
pub fn edited_files(name: &str, input: &serde_json::Value) -> Vec<String> {
    let field = |key: &str| input.get(key).and_then(serde_json::Value::as_str);
    match name {
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => field("file_path")
            .or_else(|| field("notebook_path"))
            .map(|path| vec![tail_of_path(path)])
            .unwrap_or_default(),
        "apply_patch" => patched_files(field("input").or_else(|| field("patch")).unwrap_or("")),
        _ => Vec::new(),
    }
}

/// The task list a tool call set, when it set one: Claude Code's
/// `TodoWrite` (`todos: [{content, status}]`) or Codex's `update_plan`
/// (`plan: [{step, status}]`).
#[must_use]
pub fn task_list(name: &str, input: &serde_json::Value) -> Option<Vec<TaskItem>> {
    let (key, text_key) = match name {
        "TodoWrite" => ("todos", "content"),
        "update_plan" => ("plan", "step"),
        _ => return None,
    };
    let items = input.get(key)?.as_array()?;
    Some(
        items
            .iter()
            .filter_map(|item| {
                let text = item.get(text_key)?.as_str()?.trim().to_owned();
                let status = item
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                Some(TaskItem {
                    text,
                    done: status == "completed",
                    in_progress: status == "in_progress",
                })
            })
            .collect(),
    )
}

/// A task list as lines a person (or a model) reads: `[x] add`,
/// `[ ] sub`, `[~] mul (in progress)`.
#[must_use]
pub fn render_tasks(items: &[TaskItem]) -> String {
    let mut out = String::new();
    for item in items {
        let mark = if item.done {
            "[x]"
        } else if item.in_progress {
            "[~]"
        } else {
            "[ ]"
        };
        let _ = writeln!(out, "{mark} {}", item.text);
    }
    out
}

/// The last two parts of a path (`src/calc.py`), never the home folder.
fn tail_of_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    match parts.as_slice() {
        [] => path.to_owned(),
        [only] => (*only).to_owned(),
        [.., parent, name] => format!("{parent}/{name}"),
    }
}

/// `bash -lc "…"` and `zsh -c "…"` wrappers say nothing: the command
/// inside them does.
fn strip_shell_wrapper(command: &str) -> &str {
    for prefix in ["bash -lc ", "bash -c ", "zsh -lc ", "zsh -c ", "sh -c "] {
        if let Some(rest) = command.strip_prefix(prefix) {
            return rest.trim_matches(['"', '\'']);
        }
    }
    command
}

/// The script inside a shell wrapper given as an argument vector
/// (`["/bin/zsh", "-lc", "npm test"]` → `npm test`), when it is one: a
/// shell by its name, `-c` or `-lc`, then the script.
#[must_use]
pub(crate) fn shell_script<'a>(argv: &[&'a str]) -> Option<&'a str> {
    let [shell, flag, script] = argv else {
        return None;
    };
    let name = shell.rsplit('/').next().unwrap_or(shell);
    (matches!(name, "sh" | "bash" | "zsh" | "dash") && matches!(*flag, "-c" | "-lc"))
        .then_some(*script)
}

/// The files a Codex patch touches (`*** Update File: x`, `*** Add File:
/// x`, `*** Delete File: x`), last two path parts each.
fn patched_files(patch: &str) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for line in patch.lines() {
        for marker in ["*** Update File: ", "*** Add File: ", "*** Delete File: "] {
            if let Some(path) = line.strip_prefix(marker) {
                let name = tail_of_path(path.trim());
                if !files.contains(&name) {
                    files.push(name);
                }
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn each_tool_says_what_it_did() {
        assert_eq!(
            summarize(
                "Edit",
                &json!({"file_path": "/Users/you/Dev/shop/src/calc.py"})
            ),
            "Edit src/calc.py"
        );
        assert_eq!(
            summarize("Bash", &json!({"command": "python3 -m unittest"})),
            "ran \"python3 -m unittest\""
        );
        assert_eq!(
            summarize("shell", &json!({"command": ["bash", "-lc", "pytest -q"]})),
            "ran \"pytest -q\""
        );
        assert_eq!(
            summarize(
                "exec_command",
                &json!({"command": "bash -lc \"pytest -q\""})
            ),
            "ran \"pytest -q\""
        );
        assert_eq!(
            summarize(
                "apply_patch",
                &json!({"input": "*** Begin Patch\n*** Update File: calc.py\n*** Add File: tests/test_calc.py\n*** End Patch"})
            ),
            "edited calc.py, tests/test_calc.py"
        );
        assert_eq!(
            summarize("mcp__github__create_issue", &json!({})),
            "used mcp__github__create_issue"
        );
        let long = "x".repeat(400);
        assert!(summarize("Bash", &json!({"command": long})).chars().count() < 140);
    }

    #[test]
    fn both_tools_task_lists_read_the_same() {
        let claude = task_list(
            "TodoWrite",
            &json!({"todos": [
                {"content": "add", "status": "completed"},
                {"content": "sub", "status": "in_progress"},
                {"content": "mul", "status": "pending"}
            ]}),
        )
        .unwrap();
        let codex = task_list(
            "update_plan",
            &json!({"plan": [
                {"step": "add", "status": "completed"},
                {"step": "sub", "status": "in_progress"},
                {"step": "mul", "status": "pending"}
            ]}),
        )
        .unwrap();
        assert_eq!(claude, codex);
        assert_eq!(render_tasks(&claude), "[x] add\n[~] sub\n[ ] mul\n");
        assert!(task_list("Edit", &json!({})).is_none());
    }
}
