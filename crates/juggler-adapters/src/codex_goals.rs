//! ADR-0028 (`automation_handover`) for Codex, CLI and desktop alike.
//!
//! What keeps a Codex thread working by itself:
//! - its **goal** (`/goal`): Codex pursues it turn after turn until it's
//!   done, and one stopped by a usage limit goes on when usage returns.
//!   It lives in the account's Codex folder (`goals_1.sqlite`, shared by
//!   the CLI and the Codex desktop app), and Codex's app-server owns
//!   it: `thread/goal/get` and `thread/goal/set` (a cold thread is fine;
//!   the lab of 2026-11-05). So a switch reads it (the database, read
//!   only, so the confirm screen needs no Codex process), gives the copy
//!   the goal through the target's app-server, and pauses the original's
//!   through the source's; switching back sets it as it was.
//! - a **heartbeat** automation aimed at the thread (`automations/<id>/
//!   automation.toml`, `target_thread_id`): the desktop app keeps it
//!   with its installation and sign-in and syncs it to the account, so it
//!   isn't AI Juggler's to move. It is named before the switch, as staying.
//!
//! Every app-server here is the user's own `codex`, on the account's
//! folder, with AI Juggler named as its client; a server on another
//! folder is refused.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use juggler_adapter::automation::{
    Applied, Automation, AutomationHome, HostAutomation, renamed_to,
};
use juggler_adapter::error::AdapterError;
use juggler_adapter::runtime::lines::{Lines, Next};
use serde_json::{Map, Value, json};

use crate::codex::CodexAdapter;

/// How long the app-server has for each answer.
const ANSWER_WAIT: Duration = Duration::from_secs(20);
/// A goal's objective, in the switch screen's words, at most.
const OBJECTIVE_WORDS: usize = 60;

/// A goal status (the database's or the API's) that keeps the thread
/// going by itself: working, or waiting for usage to return.
fn runs_by_itself(status: &str) -> bool {
    matches!(status, "active" | "usage_limited" | "usageLimited")
}

/// The API's name of a database status.
fn api_status(status: &str) -> &str {
    match status {
        "usage_limited" => "usageLimited",
        "budget_limited" => "budgetLimited",
        other => other,
    }
}

/// The goals of `threads` in the Codex folder `home`, read only: thread →
/// (objective, status, token budget).
fn goals(home: &Path, threads: &[String]) -> Vec<(String, String, String, Option<i64>)> {
    let path = home.join("goals_1.sqlite");
    if !path.is_file() || threads.is_empty() {
        return Vec::new();
    }
    let Ok(db) = juggler_core::storage::open_hardened_with(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return Vec::new();
    };
    let _ = db.busy_timeout(Duration::from_secs(2));
    let Ok(mut statement) =
        db.prepare("SELECT objective, status, token_budget FROM thread_goals WHERE thread_id = ?1")
    else {
        return Vec::new();
    };
    threads
        .iter()
        .filter_map(|thread| {
            statement
                .query_row([thread], |row| {
                    Ok((
                        thread.clone(),
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                })
                .ok()
        })
        .collect()
}

/// The heartbeat automations in `home` aimed at one of `threads`, by name.
fn heartbeats(home: &Path, threads: &[String]) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(home.join("automations")) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let text = std::fs::read_to_string(entry.path().join("automation.toml")).ok()?;
            let table: toml::Table = text.parse().ok()?;
            let target = table.get("target_thread_id")?.as_str()?;
            let active = table.get("status").and_then(toml::Value::as_str) == Some("ACTIVE");
            (active && threads.iter().any(|t| t == target)).then(|| {
                table
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .unwrap_or("an automation")
                    .to_owned()
            })
        })
        .collect();
    names.sort();
    names
}

fn short(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= OBJECTIVE_WORDS {
        words.join(" ")
    } else {
        format!("{}…", words[..OBJECTIVE_WORDS].join(" "))
    }
}

/// What keeps `threads` working by themselves in the Codex folder `home`.
#[must_use]
pub fn read(home: &Path, threads: &[String]) -> Automation {
    let mut automation = Automation::default();
    let mut carried = Map::new();
    for (thread, objective, status, budget) in goals(home, threads) {
        if !runs_by_itself(&status) {
            continue;
        }
        automation.what.push(format!(
            "Its goal, “{}”, which Codex keeps working on",
            short(&objective)
        ));
        carried.insert(
            thread,
            json!({ "objective": objective, "status": api_status(&status), "tokenBudget": budget }),
        );
    }
    for name in heartbeats(home, threads) {
        automation.stays.push(format!(
            "The heartbeat “{name}” stays with this account: the Codex app keeps it with its sign-in"
        ));
    }
    automation.carried = Value::Object(carried);
    automation
}

/// A short conversation with the user's `codex app-server` on `home`.
struct Server {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: Lines,
    next: i64,
}

impl Server {
    fn start(bin: &Path, home: &Path) -> Result<Self, AdapterError> {
        let failed =
            |why: String| AdapterError::InvalidLaunch(format!("Codex's app-server: {why}"));
        let mut command = juggler_adapter::user_path::command(bin);
        command.args(["app-server", "--listen", "stdio://"]);
        let default = juggler_adapter::home()
            .is_some_and(|me| crate::codex::is_codex_default_dir_in(home, &me));
        if default {
            command
                .env_remove("CODEX_HOME")
                .env_remove("CODEX_SQLITE_HOME");
        } else {
            command
                .env("CODEX_HOME", home)
                .env("CODEX_SQLITE_HOME", home);
        }
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        let mut child = command.spawn().map_err(|e| failed(e.to_string()))?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            return Err(failed("no pipes".into()));
        };
        let mut server = Self {
            child,
            stdin,
            lines: Lines::spawn(Box::new(std::io::BufReader::new(stdout))),
            next: 0,
        };
        let hello = server.call(
            "initialize",
            &json!({ "clientInfo": {
                "name": crate::codex_appserver::CLIENT_NAME,
                "title": crate::codex_appserver::CLIENT_TITLE,
                "version": env!("CARGO_PKG_VERSION"),
            }}),
        )?;
        let its_home = hello
            .get("codexHome")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let same = |a: &Path, b: &Path| a.canonicalize().ok() == b.canonicalize().ok();
        if !same(Path::new(its_home), home) {
            return Err(failed(format!(
                "it runs on {its_home}, not this account's folder"
            )));
        }
        server.send(&json!({ "jsonrpc": "2.0", "method": "initialized" }))?;
        Ok(server)
    }

    fn send(&mut self, value: &Value) -> Result<(), AdapterError> {
        juggler_adapter::runtime::lines::write_json(&mut self.stdin, value)
            .map_err(AdapterError::Io)
    }

    /// The result of `method`, or its error in words.
    fn call(&mut self, method: &str, params: &Value) -> Result<Value, AdapterError> {
        self.next += 1;
        let id = self.next;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        let deadline = Instant::now() + ANSWER_WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(AdapterError::InvalidLaunch(format!(
                    "Codex's app-server didn't answer {method} in time"
                )));
            }
            match self.lines.next(left) {
                Next::Line(line) => {
                    let Ok(message) = serde_json::from_str::<Value>(&line) else {
                        continue;
                    };
                    if message.get("id").and_then(Value::as_i64) != Some(id) {
                        continue;
                    }
                    if let Some(error) = message.get("error") {
                        return Err(AdapterError::Unsupported(
                            error
                                .get("message")
                                .and_then(Value::as_str)
                                .unwrap_or("refused")
                                .to_owned(),
                        ));
                    }
                    return Ok(message.get("result").cloned().unwrap_or(Value::Null));
                }
                Next::Timeout => {}
                Next::Closed => {
                    return Err(AdapterError::InvalidLaunch(format!(
                        "Codex's app-server ended before answering {method}"
                    )));
                }
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Whether Codex said the thread isn't in this folder (yet).
fn not_found(error: &AdapterError) -> bool {
    error.to_string().contains("thread not found")
}

impl CodexAdapter {
    /// The `codex` that serves goals: the one the adapter found, as for
    /// every other Codex feature (the CLI and the desktop app share the
    /// account's folder, so it serves both).
    fn goal_binary(&self) -> Result<PathBuf, AdapterError> {
        juggler_adapter::ToolDiscovery::locate(self).ok_or_else(|| {
            AdapterError::Unsupported(
                "Codex's command-line tool isn't installed here, so its goal couldn't be handed over"
                    .to_owned(),
            )
        })
    }

    fn set_goals(&self, home: &Path, goals: &[(String, Value)]) -> Result<Applied, AdapterError> {
        if goals.is_empty() {
            return Ok(Applied::Done);
        }
        let mut server = Server::start(&self.goal_binary()?, home)?;
        for (thread, goal) in goals {
            let mut params = goal.as_object().cloned().unwrap_or_default();
            params.insert("threadId".to_owned(), Value::String(thread.clone()));
            match server.call("thread/goal/set", &Value::Object(params)) {
                Ok(_) => {}
                Err(error) if not_found(&error) => return Ok(Applied::NotYet),
                Err(error) => return Err(error),
            }
        }
        Ok(Applied::Done)
    }
}

impl HostAutomation for CodexAdapter {
    fn home(&self) -> AutomationHome {
        AutomationHome::Account
    }

    fn read(&self, home: &Path, conversations: &[String]) -> Result<Automation, AdapterError> {
        Ok(read(home, conversations))
    }

    fn adopt(
        &self,
        home: &Path,
        carried: &Value,
        renamed: &[(String, String)],
    ) -> Result<Applied, AdapterError> {
        let goals: Vec<(String, Value)> = carried
            .as_object()
            .into_iter()
            .flatten()
            .map(|(thread, goal)| {
                let mut goal = goal.clone();
                // A goal a limit stopped goes on with the new account's room.
                if goal.get("status").and_then(Value::as_str) == Some("usageLimited") {
                    goal["status"] = Value::String("active".to_owned());
                }
                (renamed_to(renamed, thread).to_owned(), goal)
            })
            .collect();
        self.set_goals(home, &goals)
    }

    fn quiet(&self, home: &Path, conversations: &[String]) -> Result<Value, AdapterError> {
        let running: Vec<(String, String)> = goals(home, conversations)
            .into_iter()
            .filter(|(_, _, status, _)| runs_by_itself(status))
            .map(|(thread, _, status, _)| (thread, api_status(&status).to_owned()))
            .collect();
        let paused: Vec<(String, Value)> = running
            .iter()
            .map(|(thread, _)| (thread.clone(), json!({ "status": "paused" })))
            .collect();
        self.set_goals(home, &paused)?;
        Ok(Value::Array(
            running
                .into_iter()
                .map(|(thread, status)| json!({ "thread": thread, "status": status }))
                .collect(),
        ))
    }

    fn wake(&self, home: &Path, restore: &Value) -> Result<Applied, AdapterError> {
        let goals: Vec<(String, Value)> = restore
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                Some((
                    entry.get("thread")?.as_str()?.to_owned(),
                    json!({ "status": entry.get("status")?.as_str()? }),
                ))
            })
            .collect();
        self.set_goals(home, &goals)
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "test fixtures build databases raw (other tools' files, old schemas)"
)]
mod tests {
    use super::{heartbeats, read};

    fn goals_db(home: &std::path::Path, rows: &[(&str, &str, &str)]) {
        let db = rusqlite::Connection::open(home.join("goals_1.sqlite")).unwrap();
        db.execute_batch(
            "CREATE TABLE thread_goals (thread_id TEXT PRIMARY KEY NOT NULL, goal_id TEXT NOT NULL, objective TEXT NOT NULL, status TEXT NOT NULL, token_budget INTEGER, tokens_used INTEGER NOT NULL DEFAULT 0, time_used_seconds INTEGER NOT NULL DEFAULT 0, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL);",
        )
        .unwrap();
        for (thread, objective, status) in rows {
            db.execute(
                "INSERT INTO thread_goals VALUES (?1, 'g', ?2, ?3, NULL, 0, 0, 0, 0)",
                [thread, objective, status],
            )
            .unwrap();
        }
    }

    #[test]
    fn a_working_or_limit_stopped_goal_is_handed_over_a_paused_one_isnt() {
        let home = tempfile::tempdir().unwrap();
        goals_db(
            home.path(),
            &[
                ("t1", "Ship 0.4", "active"),
                ("t2", "Wait", "usage_limited"),
                ("t3", "Done", "paused"),
            ],
        );
        let threads = ["t1", "t2", "t3", "t4"].map(str::to_owned);
        let found = read(home.path(), &threads);
        assert_eq!(found.what.len(), 2);
        assert!(found.what[0].contains("“Ship 0.4”"));
        assert_eq!(found.carried["t2"]["status"], "usageLimited");
        assert!(found.carried.get("t3").is_none());
        assert!(read(home.path(), &["none".to_owned()]).what.is_empty());
    }

    #[test]
    fn a_heartbeat_on_the_thread_is_named_as_staying() {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join("automations/brief");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("automation.toml"),
            "version = 1\nid = \"brief\"\nkind = \"heartbeat\"\nname = \"Morning brief\"\nstatus = \"ACTIVE\"\ntarget_thread_id = \"t1\"\n",
        )
        .unwrap();
        assert_eq!(
            heartbeats(home.path(), &["t1".to_owned()]),
            vec!["Morning brief".to_owned()]
        );
        assert!(heartbeats(home.path(), &["t2".to_owned()]).is_empty());
        assert_eq!(read(home.path(), &["t1".to_owned()]).stays.len(), 1);
    }
}
