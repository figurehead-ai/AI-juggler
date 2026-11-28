//! The live resolvers (ADR-0038 §5 step 4, ADR-0037 §3): a node a walk
//! picked becomes one plain line, read **now** from the module that owns
//! what it points at. An entity node holds no state, so what a session is
//! doing, an Account's room or a task's state is never a remembered one.
//! A memory node's line is the record's own words, said as the user's
//! ("You said …", "You call … …") with its age composed by code. A turn's
//! line is the ledger's (ADR-0037 §5.4): the question in the user's words
//! and when, never the answer shown then. Agent-written names come from
//! the records already through the guard; a line whose source is gone is
//! no line ("no longer here" is never invented).

use std::fmt::Write as _;

use juggler_core::storage::Storage;
use juggler_core::storage::memory::Kind;

use crate::features::bot::dto::{BotRowInfo, BotRowKind};
use crate::features::bot::world::World;
use crate::features::bot::written::{neutral, state_words};

/// What a node is, in the user's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plain {
    /// A session.
    Session,
    /// A project.
    Project,
    /// An App or profile.
    App,
    /// An Account.
    Account,
    /// A pool.
    Pool,
    /// A task.
    Task,
    /// Something the user said or set.
    Memory,
    /// An earlier question.
    Earlier,
    /// A fact, rule or question node's own label.
    Note,
}

impl Plain {
    /// The word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Project => "project",
            Self::App => "App",
            Self::Account => "Account",
            Self::Pool => "pool",
            Self::Task => "task",
            Self::Memory => "memory",
            Self::Earlier => "earlier question",
            Self::Note => "note",
        }
    }
}

/// A node as one line, read now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// What it is.
    pub plain: Plain,
    /// Its short name (for "connected to …").
    pub name: String,
    /// The line for the model's context.
    pub line: String,
    /// For a memory: the record's kind and subject key (the breakdown
    /// counts by kind, once per subject).
    pub record: Option<(Kind, String)>,
    /// For an earlier question: its topic (another topic's stays there).
    pub topic: Option<String>,
}

/// "2 days ago", composed by code (ADR-0037 §3).
#[must_use]
pub fn age_words(now_ms: i64, then_ms: i64) -> String {
    let minutes = now_ms.saturating_sub(then_ms).max(0) / 60_000;
    match minutes {
        0 => "just now".to_owned(),
        1 => "a minute ago".to_owned(),
        m if m < 60 => format!("{m} minutes ago"),
        m if m < 120 => "an hour ago".to_owned(),
        m if m < 60 * 24 => format!("{} hours ago", m / 60),
        m if m < 60 * 48 => "yesterday".to_owned(),
        m => format!("{} days ago", m / (60 * 24)),
    }
}

/// The first 20 words, in curly quotes, neutralised (a record's words as
/// data).
#[must_use]
pub fn quoted(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().take(20).collect();
    format!("\u{201c}{}\u{201d}", neutral(&words.join(" ")))
}

fn row_line(row: &BotRowInfo) -> String {
    let kind = match row.kind {
        BotRowKind::Session => "session",
        BotRowKind::Task => "task",
        BotRowKind::Account => "Account",
        BotRowKind::Memory => "memory",
    };
    let mut line = format!("{kind} {}", quoted(&row.title));
    if let Some(state) = row.state {
        let _ = write!(line, ", {} now", state_words(state));
    }
    if let Some(project) = &row.project {
        let _ = write!(line, ", in project {}", quoted(project));
    }
    if let Some(app) = &row.app_label {
        let _ = write!(line, ", in {}", quoted(app));
    }
    if let Some(pool) = &row.pool {
        let _ = write!(line, ", pool {}", quoted(pool));
    }
    if let Some(used) = row.used_percent {
        let window = match row.window.as_deref() {
            Some("five_hour") => " of its five-hour window",
            Some("weekly") => " of its weekly window",
            _ => "",
        };
        let _ = write!(line, ", {used}% used{window} now");
    }
    line
}

fn named(plain: Plain, name: &str, line: String) -> Resolved {
    Resolved {
        plain,
        name: name.to_owned(),
        line,
        record: None,
        topic: None,
    }
}

/// The node with this key as one line, read now; `None` when what it
/// points at is gone (or is only structure, like a topic).
#[must_use]
pub fn resolve(
    storage: &Storage,
    world: &World,
    key: &str,
    label: Option<&str>,
) -> Option<Resolved> {
    let (kind, id) = key.split_once(':')?;
    let find = |k: BotRowKind| world.find(k, id);
    match kind {
        "session" => {
            find(BotRowKind::Session).map(|r| named(Plain::Session, &r.title, row_line(r)))
        }
        "task" => find(BotRowKind::Task).map(|r| named(Plain::Task, &r.title, row_line(r))),
        "account" => {
            find(BotRowKind::Account).map(|r| named(Plain::Account, &r.title, row_line(r)))
        }
        "project" => world.project_name(id).map(|name| {
            let running = world
                .sessions
                .iter()
                .filter(|r| r.project_id.as_deref() == Some(id))
                .filter(|r| r.state == Some(crate::features::bot::dto::BotState::Working))
                .count();
            named(
                Plain::Project,
                &name,
                format!(
                    "project {}, {running} of its sessions working now",
                    quoted(&name)
                ),
            )
        }),
        "app" => world
            .sessions
            .iter()
            .find(|r| r.app_id.as_deref() == Some(id))
            .and_then(|r| r.app_label.clone())
            .map(|name| named(Plain::App, &name, format!("App {}", quoted(&name)))),
        "pool" => world
            .pool_name(id)
            .map(|name| named(Plain::Pool, &name, format!("pool {}", quoted(&name)))),
        "memory" => memory_line(storage, world.now_ms, id),
        "turn" => {
            let turn = storage.chat().turn(id).ok().flatten()?;
            // A forgotten turn, or one whose answer the user marked wrong
            // (AJ-E1548 #2: never sent back), is no line.
            let flagged = serde_json::from_str::<serde_json::Value>(&turn.entities)
                .ok()
                .and_then(|meta| meta.get("flagged").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            if turn.redacted || flagged || turn.question.trim().is_empty() {
                return None;
            }
            Some(Resolved {
                plain: Plain::Earlier,
                name: quoted(&turn.question),
                line: format!(
                    "earlier you asked {} ({})",
                    quoted(&turn.question),
                    age_words(world.now_ms, turn.at_ms)
                ),
                record: None,
                topic: Some(turn.topic),
            })
        }
        "fact" | "rule" | "question" => label
            .filter(|l| !l.trim().is_empty())
            .map(|l| named(Plain::Note, l, format!("note {}", quoted(l)))),
        _ => None,
    }
}

fn memory_line(storage: &Storage, now_ms: i64, subject_id: &str) -> Option<Resolved> {
    let record = storage
        .memory()
        .current_by_subject(subject_id)
        .ok()
        .flatten()?;
    if record.untrusted {
        return None;
    }
    let age = age_words(now_ms, record.as_of_ms);
    let about = record
        .payload
        .get("about")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let line = match record.kind {
        Kind::Preference if record.subject_key.starts_with("alias:") => format!(
            "you call {} {} (you said, {age})",
            quoted(about.as_deref().unwrap_or("it")),
            quoted(&record.text)
        ),
        Kind::Preference => format!(
            "what matters most to you: {} (you said, {age})",
            quoted(&record.text)
        ),
        Kind::Note => format!("you said: {} ({age})", quoted(&record.text)),
        Kind::Asked => format!("you asked {} ({age})", quoted(&record.text)),
        Kind::StandingRequest => format!("you asked to be told: {} ({age})", quoted(&record.text)),
        _ => return None,
    };
    Some(Resolved {
        plain: Plain::Memory,
        name: quoted(&record.text),
        line,
        record: Some((record.kind, record.subject_key.clone())),
        topic: None,
    })
}
