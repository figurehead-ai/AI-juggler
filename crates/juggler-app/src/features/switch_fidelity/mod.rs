//! The measured switch fidelity (the live lab of 2026-10-28): for one hop
//! of a switch, what the SOURCE conversation held at switch time against
//! what the TARGET session actually received, measured rather than
//! spot-checked.
//!
//! Every conversation is read through the adapters' own readers
//! ([`juggler_adapters::parse_any`], each adapter's subagent reader, its
//! `used_in` and `inventory`); nothing here knows a tool's format or names
//! a tool. A hop's target received the history in one of three forms
//! ([`Received`]): the conversation itself (a transplant's copy), a
//! handover file (a replay into another tool), or, switching back, the
//! original conversation resumed with one update turn. Each is measured
//! in the source's own units: how many of its turns, tool calls, results
//! and images arrived whole, shortened or not at all; its task list with
//! each item's state; its subagents, rules files, MCP servers, folder and
//! uncommitted changes; and the facts planted in its early, middle and
//! late turns, probed on the target ([`facts`]).
//!
//! [`measure`] is pure (unit-tested on the conformance fixtures);
//! [`conversation_path`] and [`surroundings`] read what a live hop needs
//! from AI Juggler's state. [`HopReport::markdown`] renders the table and
//! the one-line verdict.

pub mod facts;
mod measure;
mod render;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::inventory::ItemKind;
use juggler_adapter::subagents::SubagentScope;
use juggler_adapters::ContentCounts;
use juggler_core::model::AccountId;

use crate::state::AppState;

pub use measure::measure;

/// One turn of a conversation, as the tool's reader gives it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Turn {
    /// The message's id, when the tool records one.
    pub uuid: Option<String>,
    /// Whether the person (rather than the model) said it.
    pub user: bool,
    /// What was said.
    pub text: String,
    /// One line per tool call the turn made.
    pub tools: Vec<String>,
}

/// A task list item and where it stands.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Task {
    /// What the item says.
    pub text: String,
    /// `done`, `in progress` or `open`.
    pub state: &'static str,
}

impl Task {
    /// The item as a handover or an update renders it (`[x] add`).
    #[must_use]
    pub fn rendered(&self) -> String {
        let mark = match self.state {
            "done" => "[x]",
            "in progress" => "[~]",
            _ => "[ ]",
        };
        format!("{mark} {}", self.text)
    }
}

/// A conversation as its tool's reader sees it: the main thread's turns
/// (a subagent's own lines and messages that only carry tool results
/// are left out), what it carries besides text, and its task list.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Conversation {
    /// Its turns, in order.
    pub turns: Vec<Turn>,
    /// Tool calls, results and images, counted by the reader.
    pub content: ContentCounts,
    /// The task list as the last call that set one left it.
    pub tasks: Option<Vec<Task>>,
}

impl Conversation {
    /// Reads a transcript's bytes, whatever tool wrote them.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        use juggler_adapters::MessageRole;
        let parsed = juggler_adapters::parse_any(bytes);
        let turns = parsed
            .messages
            .into_iter()
            .filter(|message| !message.sidechain)
            .filter_map(|message| {
                let user = match message.role {
                    MessageRole::User => true,
                    MessageRole::Assistant => false,
                    MessageRole::Other => return None,
                };
                let said = !message.text.trim().is_empty() || !message.tools.is_empty();
                said.then_some(Turn {
                    uuid: message.uuid,
                    user,
                    text: message.text,
                    tools: message.tools,
                })
            })
            .collect();
        let tasks = parsed.tasks.map(|items| {
            items
                .into_iter()
                .map(|item| Task {
                    text: item.text,
                    state: if item.done {
                        "done"
                    } else if item.in_progress {
                        "in progress"
                    } else {
                        "open"
                    },
                })
                .collect()
        });
        Self {
            turns,
            content: parsed.content,
            tasks,
        }
    }

    /// Reads the transcript at `path` (compressed or not).
    ///
    /// # Errors
    ///
    /// The read's error.
    pub fn read(path: &Path) -> std::io::Result<Self> {
        juggler_adapters::read_session_file(path).map(|bytes| Self::from_bytes(&bytes))
    }

    /// Turns the person said.
    #[must_use]
    pub fn user_turns(&self) -> usize {
        self.turns.iter().filter(|turn| turn.user).count()
    }

    /// Messages the model sent (with words, tool calls or both).
    #[must_use]
    pub fn assistant_messages(&self) -> usize {
        self.turns.iter().filter(|turn| !turn.user).count()
    }

    /// Every word of the conversation, tool lines included.
    #[must_use]
    pub fn all_text(&self) -> String {
        let mut out = String::new();
        for turn in &self.turns {
            out.push_str(&turn.text);
            out.push('\n');
            for tool in &turn.tools {
                out.push_str(tool);
                out.push('\n');
            }
        }
        out
    }

    /// This conversation followed by `other` (a chain of hops).
    #[must_use]
    pub fn then(mut self, other: Self) -> Self {
        self.turns.extend(other.turns);
        self.content.tool_calls += other.content.tool_calls;
        self.content.tool_results += other.content.tool_results;
        self.content.tool_errors += other.content.tool_errors;
        self.content.images += other.content.images;
        if other.tasks.is_some() {
            self.tasks = other.tasks;
        }
        self
    }
}

/// The work around a conversation when it was measured.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Surroundings {
    /// The folder the session runs in.
    pub folder: PathBuf,
    /// `git diff --shortstat HEAD` there, when it's a repository.
    pub diff_stat: Option<String>,
    /// The uncommitted files there (`git status --short` lines).
    pub changed: Vec<String>,
    /// The rules files in the folder that this side's tool reads itself.
    pub rules: Vec<String>,
    /// The conversation's subagents (the tool's ids for them).
    pub subagents: Vec<String>,
    /// The folders their own conversations are kept in.
    pub subagent_folders: Vec<PathBuf>,
    /// MCP servers: on a source, the ones its conversation used; on a
    /// target, the ones its account has set up.
    pub mcp: Vec<String>,
}

/// How the target received the history.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Received {
    /// The conversation itself, in the target tool's own transcript (a
    /// transplant's copy).
    Native(Conversation),
    /// A handover file the target reads first (a replay into another
    /// tool), and how many turns its writer says it holds and left out.
    Handover {
        /// The file's text.
        text: String,
        /// Turns quoted in it.
        included: usize,
        /// Turns left out (named by topic).
        left_out: usize,
    },
    /// Switching back: the original conversation, resumed as it was, and
    /// the one "While you were away" update turn.
    Return {
        /// The original conversation, before the update.
        original: Conversation,
        /// The update turn's text.
        update: String,
    },
}

/// One side of a hop.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Side<T> {
    /// "Lab A (Claude Code)": the account and its tool, as the app names
    /// them.
    pub label: String,
    /// What it held (source) or received (target).
    pub history: T,
    /// The work around it.
    pub around: Surroundings,
}

/// One hop to measure: the source at switch time and the target right
/// after the switch, with the planted facts and how the target answered
/// them.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Hop {
    /// "A → B".
    pub name: String,
    /// "transplant", "replay", "switch back".
    pub mechanism: String,
    /// The source.
    pub source: Side<Conversation>,
    /// The target.
    pub target: Side<Received>,
    /// The planted facts, each with the target's answer.
    pub facts: Vec<facts::Probed>,
    /// The opening sentences a replay's first turn starts with: such a
    /// turn is the handover itself, not work done on the hop.
    pub openings: Vec<String>,
}

/// Whether a measure arrived whole, was reduced by design (and the
/// switch says so), or was lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Carry {
    /// Everything arrived.
    Whole,
    /// Less arrived, by the mechanism's design, and the screen or the
    /// handover says what doesn't travel.
    ByDesign,
    /// Something was lost that should have arrived.
    Lost,
}

/// One row of a hop's table.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Row {
    /// What is measured.
    pub measure: &'static str,
    /// How many the source had.
    pub source: usize,
    /// How many arrived.
    pub carried: usize,
    /// Whole, reduced by design, or lost.
    pub carry: Carry,
    /// How it arrived, in a few words.
    pub notes: String,
}

/// A hop's measured fidelity.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct HopReport {
    /// "A → B".
    pub name: String,
    /// The source's and target's labels and the mechanism.
    pub route: String,
    /// What form the history took, with how many turns of each.
    pub form: String,
    /// The table.
    pub rows: Vec<Row>,
    /// Each planted fact.
    pub facts: Vec<facts::FactRow>,
    /// PASS, PARTIAL or FAIL.
    pub grade: &'static str,
    /// The one-line verdict.
    pub verdict: String,
}

/// The transcript of `account`'s conversation `id`, from the
/// conversations index (refreshed first, so a conversation begun a
/// moment ago is found).
#[must_use]
pub fn conversation_path(state: &Arc<AppState>, account: &str, id: &str) -> Option<PathBuf> {
    let _ = crate::features::conversations::refresh_impl(state);
    state
        .conversations
        .conversation_of(account, id)
        .map(|conversation| conversation.path)
}

/// What a side's own reader says about where it is: the folder, its
/// uncommitted changes, the rules files there its tool reads itself, the
/// conversation's subagents and folders, and MCP servers — on a source
/// (`used`), those the conversation used; on a target, those its account
/// has set up.
#[must_use]
pub fn surroundings(
    state: &Arc<AppState>,
    account: &str,
    conversation: Option<(&Path, &str)>,
    folder: &Path,
    used: bool,
) -> Surroundings {
    let record = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.accounts().get(&AccountId::new(account)).ok());
    let adapter = record
        .as_ref()
        .and_then(|record| state.registry.get(&record.harness));
    let changes = crate::worktree::work_changes(folder);
    let mut around = Surroundings {
        folder: folder.to_path_buf(),
        diff_stat: changes.as_ref().and_then(|changes| changes.stat.clone()),
        changed: changes.map(|changes| changes.files).unwrap_or_default(),
        ..Surroundings::default()
    };
    let Some(adapter) = adapter else {
        return around;
    };
    around.rules = adapter
        .spec()
        .rules_files
        .into_iter()
        .filter(|name| folder.join(name).is_file())
        .collect();
    if let (Some((path, id)), Some(record)) = (conversation, record.as_ref())
        && let Some(mut reader) = crate::features::subagents::count::reader_for(
            &state.registry,
            record.harness.as_str(),
            SubagentScope { path, id },
        )
        && reader.refresh().is_ok()
    {
        for agent in reader.subagents() {
            if let Some(dir) = agent.transcript.as_ref().and_then(|file| file.parent())
                && !around.subagent_folders.iter().any(|known| known == dir)
            {
                around.subagent_folders.push(dir.to_path_buf());
            }
            around.subagents.push(agent.id);
        }
        around.subagents.sort();
    }
    around.mcp = match (used, conversation, record.as_ref()) {
        (true, Some((path, _)), _) => adapter
            .used_in(path)
            .unwrap_or_default()
            .into_iter()
            .filter(|(kind, _)| *kind == ItemKind::McpServer)
            .map(|(_, name)| name)
            .collect(),
        (false, _, Some(record)) => adapter
            .inventory(&record.state_dir, Some(folder))
            .into_iter()
            .filter(|item| item.kind == ItemKind::McpServer)
            .map(|item| item.name)
            .collect(),
        _ => Vec::new(),
    };
    around.mcp.sort();
    around.mcp.dedup();
    around
}

/// The opening sentence every registered tool's replay starts with
/// ([`Hop::openings`]).
#[must_use]
pub fn replay_openings(state: &Arc<AppState>) -> Vec<String> {
    state
        .registry
        .ids()
        .into_iter()
        .map(|id| crate::commands::replay_header(state, &id))
        .filter(|line| !line.trim().is_empty())
        .collect()
}

#[cfg(test)]
mod tests;
