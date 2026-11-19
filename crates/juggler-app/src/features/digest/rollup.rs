//! The rollup (ADR-0036 §1.4): a pure function over the facts, no model.
//! It is what the digest shows with the AI off, unavailable or skipped;
//! the line an AI item falls back to; and the baseline an AI digest must
//! beat in blind pairs (D-130).
//!
//! Every changed agent is accounted for: shown, or behind "and N more";
//! the agents that didn't change are counted as quiet. Needs and failures
//! come first. A fact the journals don't hold is left out, never guessed.
//!
//! The round-3 lessons (`docs/qa/evidence/AJ-E3709/v5-final.md`; the
//! harness's `digest.py` is the spec), as rules on typed facts:
//! - **State words follow the facts**: the newest fact decides the state
//!   ("working" after a restart, never "failed" still).
//! - **Reverted work is said as reverted**, from the agent's own words,
//!   and never "partly done" or "not done" beside it.
//! - **Never "partly done" on idle work** (no turn in the window).
//! - **Never a pass count next to "not run"**: one tests fact per line,
//!   and a "not run" caveat is dropped where tests ran.
//! - **"Blocked" only where something blocks**: a need, a blocked task, or
//!   the agent's own words saying so.
//!
//! Facts only: every sentence is the window's (`ui/src/lib/feed-words.ts`).

use serde::{Deserialize, Serialize};

use crate::features::observe::{EndKind, Observed, SessionState, TaskState, TurnOutcome};

/// Items shown before "and N more agents" (ADR-0036 §1.3).
pub const SHOWN: usize = 6;
/// The longest quote a line carries (the agent's own question, failure or
/// last words), in characters.
pub const QUOTE_MAX: usize = 110;

/// What a turn's tests did, as the facts say.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tests {
    /// No fact about tests.
    #[default]
    Unknown,
    /// The turn says it didn't run them.
    NotRun,
    /// They ran.
    Ran {
        /// Passed.
        passed: u32,
        /// Failed.
        failed: u32,
    },
}

/// A caveat the agent's words or the turn's facts hold (ADR-0033 §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum Caveat {
    /// Tests not run.
    NotRun,
    /// Not done.
    NotDone,
    /// Partly done.
    Partial,
    /// Blocked.
    Blocked,
    /// The agent isn't sure.
    Unsure,
}

/// One turn that ended in the window (AJ-E3705 records them; until then a
/// session's digest line has none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Turn {
    /// When it ended.
    pub at_ms: i64,
    /// How it ended.
    pub outcome: Option<TurnOutcome>,
    /// Its tests.
    pub tests: Tests,
    /// Its caveat, when its words or facts hold one.
    pub caveat: Option<Caveat>,
    /// A's excerpt of its last message (AJ-E3708), while Last messages is
    /// on: the agent's words, untrusted.
    pub words: Option<String>,
    /// The agent's own question, when it asks.
    pub question: Option<String>,
    /// The agent's own failure sentence, when it failed.
    pub failure: Option<String>,
}

/// Whose a line is: the App's mark beside the name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DigestAuthorInfo {
    /// The App's tool (`claude-code`, `codex`…).
    pub provider: Option<String>,
    /// Its Account folder (`accounts.id`).
    pub folder: Option<String>,
    /// That folder's name.
    pub folder_label: Option<String>,
    /// It runs on the desktop.
    pub desktop: bool,
}

/// One agent's facts in the window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentFacts {
    /// The session.
    pub session: String,
    /// Its name, as its row names it.
    pub name: String,
    /// Its App.
    pub author: DigestAuthorInfo,
    /// What was observed, in order.
    pub events: Vec<(i64, Observed)>,
    /// The turns that ended, in order.
    pub turns: Vec<Turn>,
    /// Files changed, as git says now.
    pub files: Option<u32>,
}

/// The window a digest covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Window {
    /// Its start, epoch ms.
    pub start_ms: i64,
    /// Its end.
    pub end_ms: i64,
}

/// One agent's line: typed facts the window says in words.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DigestLineInfo {
    /// The session it is about (its link).
    pub session_id: String,
    /// The agent's name.
    pub name: String,
    /// Its App.
    pub author: DigestAuthorInfo,
    /// D-117: 3 broke, 2 needs you, 1 a note.
    pub level: u8,
    /// Its state now, from the newest fact: `failed`, `asks`, `limit`,
    /// `stuck`, `finished`, `stopped`, `idle` or `working`.
    pub state: String,
    /// Something in it wants a look (a need, a failure, an ending,
    /// processes left, a restart, room running low, a task).
    pub notable: bool,
    /// Looked stuck this long by the window's end, minutes.
    pub stuck_min: Option<u32>,
    /// Processes it left running.
    pub left_processes: Option<u32>,
    /// Its account's room left, percent, when it crossed low.
    pub runway_left: Option<u8>,
    /// Started again after a crash.
    pub restarted: bool,
    /// A task it moved: done, blocked or in review (the newest).
    pub task: Option<TaskState>,
    /// Turns ended in the window, when they are known.
    pub turns: Option<u32>,
    /// Files changed, when git says.
    pub files: Option<u32>,
    /// Its tests: `passed`, `failed` or `not_run`; none when unknown.
    pub tests: Option<String>,
    /// Tests passed, with `tests`.
    pub passed: Option<u32>,
    /// Tests failed, with `tests`.
    pub failed: Option<u32>,
    /// Its own words say it undid its change.
    pub reverted: bool,
    /// The caveat its facts hold, only where the facts allow it.
    pub caveat: Option<Caveat>,
    /// The agent's own words: its question (`asks`), its failure
    /// (`failure`) or its last words (`last`). Agent-written: quoted.
    pub quote: Option<String>,
    /// See `quote`.
    pub quote_kind: Option<String>,
    /// Its newest activity in the window.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// A digest: every changed agent's line, in order, and the quiet count.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DigestInfo {
    /// Its window's start, epoch ms.
    #[ts(type = "number")]
    pub start_ms: i64,
    /// Its window's end.
    #[ts(type = "number")]
    pub end_ms: i64,
    /// Every agent that changed, ordered: L3, L2, a fact that wants a
    /// look, other news, then "on" by newest activity.
    pub lines: Vec<DigestLineInfo>,
    /// Lines shown before "and N more agents".
    pub shown: u32,
    /// Open agents that didn't change ("2 quiet").
    pub quiet: u32,
    /// Written by a model and checked (never in 0.5: the AI digest is
    /// "not yet", v5-final.md).
    pub ai: bool,
}

/// The rollup over `agents` (the changed ones; an agent with no event
/// and no turn is counted quiet), with `quiet` open agents that didn't
/// change. Deterministic: the same facts give the same digest.
#[must_use]
pub fn rollup(window: Window, agents: &[AgentFacts], quiet: u32) -> DigestInfo {
    let mut lines = Vec::new();
    let mut idle = 0;
    for agent in agents {
        match line_of(agent) {
            Some(line) => lines.push(line),
            None => idle += 1,
        }
    }
    lines.sort_by(|a, b| {
        order_key(a)
            .cmp(&order_key(b))
            .then(b.at_ms.cmp(&a.at_ms))
            .then(a.session_id.cmp(&b.session_id))
    });
    DigestInfo {
        start_ms: window.start_ms,
        end_ms: window.end_ms,
        shown: u32::try_from(lines.len().min(SHOWN)).unwrap_or(0),
        lines,
        quiet: quiet + idle,
        ai: false,
    }
}

/// L3, L2, then a fact that wants a look (stuck, low room, a crash,
/// processes left, a blocked task), then other news, then "on".
fn order_key(line: &DigestLineInfo) -> (u8, u8) {
    let strong = line.stuck_min.is_some()
        || line.runway_left.is_some()
        || line.restarted
        || line.left_processes.is_some()
        || line.task == Some(TaskState::Blocked);
    let rank = if strong {
        0
    } else if line.notable {
        1
    } else {
        2
    };
    (3 - line.level.min(3), rank)
}

/// What the newest facts say the agent is doing now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Now {
    Failed,
    Asks,
    Limit,
    Stuck,
    Finished,
    Stopped,
    Idle,
    Working,
}

impl Now {
    const fn word(self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::Asks => "asks",
            Self::Limit => "limit",
            Self::Stuck => "stuck",
            Self::Finished => "finished",
            Self::Stopped => "stopped",
            Self::Idle => "idle",
            Self::Working => "working",
        }
    }

    const fn level(self) -> u8 {
        match self {
            Self::Failed => 3,
            Self::Asks | Self::Limit | Self::Stuck => 2,
            _ => 1,
        }
    }
}

/// The state the newest fact says, and when the agent entered it.
fn now_of(agent: &AgentFacts) -> Option<(Now, i64)> {
    fn set(now: &mut Option<(Now, i64)>, state: Now, at: i64) {
        if now.is_none_or(|(was, _)| was != state) {
            *now = Some((state, at));
        }
    }
    let mut now: Option<(Now, i64)> = None;
    // The turns and the events, in time order (a turn's end before an
    // event at the same moment).
    let mut facts: Vec<(i64, u8, Option<&Observed>, Option<&Turn>)> = agent
        .turns
        .iter()
        .map(|turn| (turn.at_ms, 0, None, Some(turn)))
        .chain(
            agent
                .events
                .iter()
                .map(|(at, event)| (*at, 1, Some(event), None)),
        )
        .collect();
    facts.sort_by_key(|(at, order, _, _)| (*at, *order));
    for (at, _, event, turn) in facts {
        if let Some(turn) = turn {
            match turn.outcome {
                Some(TurnOutcome::Failed) => set(&mut now, Now::Failed, at),
                Some(TurnOutcome::Completed) if turn.question.is_some() => {
                    set(&mut now, Now::Asks, at);
                }
                Some(TurnOutcome::Completed) => set(&mut now, Now::Idle, at),
                Some(TurnOutcome::Interrupted) => set(&mut now, Now::Stopped, at),
                None => {}
            }
            continue;
        }
        let Some(event) = event else { continue };
        match event {
            Observed::Started | Observed::Restarted | Observed::AskResolved => {
                set(&mut now, Now::Working, at);
            }
            Observed::StateEntered { state } => set(
                &mut now,
                match state {
                    SessionState::Working => Now::Working,
                    SessionState::Idle => Now::Idle,
                    SessionState::AsksYou => Now::Asks,
                    SessionState::Stuck => Now::Stuck,
                    SessionState::LimitHit => Now::Limit,
                    SessionState::Finished => Now::Finished,
                    SessionState::Failed => Now::Failed,
                    SessionState::Stopped | SessionState::HandedOver => Now::Stopped,
                },
                at,
            ),
            Observed::TurnEnded { outcome } => set(
                &mut now,
                match outcome {
                    TurnOutcome::Completed => Now::Idle,
                    TurnOutcome::Interrupted => Now::Stopped,
                    TurnOutcome::Failed => Now::Failed,
                },
                at,
            ),
            Observed::AskOpened => set(&mut now, Now::Asks, at),
            // An ending keeps what the state said (a limit, a failure);
            // alone it says how it ended.
            Observed::Ended { how } => {
                let ended = match how {
                    EndKind::Clean => Now::Finished,
                    EndKind::Stopped => Now::Stopped,
                    EndKind::Error => Now::Failed,
                };
                let keeps = matches!(now, Some((Now::Limit | Now::Failed, _)));
                if !keeps {
                    set(&mut now, ended, at);
                }
            }
            _ => {}
        }
    }
    now
}

/// What happened to an agent in the window, as its newest event of each
/// kind says.
struct Happened {
    left_processes: Option<u32>,
    runway_left: Option<u8>,
    restarted: bool,
    task: Option<TaskState>,
}

fn happened(agent: &AgentFacts) -> Happened {
    let mut out = Happened {
        left_processes: None,
        runway_left: None,
        restarted: false,
        task: None,
    };
    for (_, event) in &agent.events {
        match event {
            Observed::ProcessesLeft { count } => {
                out.left_processes = (*count > 0).then_some(*count);
            }
            Observed::RunwayLeft { percent } => out.runway_left = Some(*percent),
            Observed::Restarted => out.restarted = true,
            Observed::TaskMoved { to } => out.task = Some(*to),
            _ => {}
        }
    }
    out
}

/// One agent's line; `None` when nothing about it changed in the window.
#[must_use]
pub fn line_of(agent: &AgentFacts) -> Option<DigestLineInfo> {
    if agent.events.is_empty() && agent.turns.is_empty() {
        return None;
    }
    let (now, since) = now_of(agent).unwrap_or((Now::Working, 0));
    let at_ms = agent
        .events
        .iter()
        .map(|(at, _)| *at)
        .chain(agent.turns.iter().map(|turn| turn.at_ms))
        .max()
        .unwrap_or(0);
    let Happened {
        left_processes,
        runway_left,
        restarted,
        task,
    } = happened(agent);
    let newest = agent.turns.last();
    // The newest turn whose tests the facts know (as the harness reads it).
    let tests = agent
        .turns
        .iter()
        .rev()
        .map(|turn| turn.tests)
        .find(|tests| *tests != Tests::Unknown)
        .unwrap_or_default();
    let reverted = newest
        .and_then(|turn| turn.words.as_deref())
        .is_some_and(says_undone);
    let level = now.level();
    let blocks = level == 2
        || task == Some(TaskState::Blocked)
        || newest
            .and_then(|turn| turn.words.as_deref())
            .is_some_and(says_blocked);
    let caveat = newest
        .and_then(|turn| turn.caveat)
        .filter(|caveat| caveat_holds(*caveat, &agent.turns, tests, reverted, blocks));
    let (quote, quote_kind) = quote_of(now, agent);
    let stuck_min = (now == Now::Stuck).then(|| {
        let end = at_ms.max(since);
        u32::try_from((end - since).max(0) / 60_000).unwrap_or(u32::MAX)
    });
    let (tests_word, passed, failed) = match tests {
        Tests::Unknown => (None, None, None),
        Tests::NotRun => (Some("not_run"), None, None),
        Tests::Ran { passed, failed } => (
            Some(if failed > 0 { "failed" } else { "passed" }),
            Some(passed),
            Some(failed),
        ),
    };
    let ended_or_finished = matches!(now, Now::Finished | Now::Stopped | Now::Failed);
    Some(DigestLineInfo {
        session_id: agent.session.clone(),
        name: agent.name.clone(),
        author: agent.author.clone(),
        level,
        state: now.word().to_owned(),
        notable: level >= 2
            || ended_or_finished
            || left_processes.is_some()
            || restarted
            || runway_left.is_some()
            || task.is_some(),
        stuck_min,
        left_processes,
        runway_left,
        restarted,
        task,
        turns: (!agent.turns.is_empty())
            .then(|| u32::try_from(agent.turns.len()).unwrap_or(u32::MAX)),
        files: agent.files.filter(|files| *files > 0),
        tests: tests_word.map(str::to_owned),
        passed,
        failed,
        reverted,
        caveat,
        quote,
        quote_kind,
        at_ms,
    })
}

/// A caveat stays only where the facts allow its words.
fn caveat_holds(
    caveat: Caveat,
    turns: &[Turn],
    tests: Tests,
    reverted: bool,
    blocks: bool,
) -> bool {
    match caveat {
        // Never "tests not run" where tests ran.
        Caveat::NotRun => !matches!(tests, Tests::Ran { .. }),
        // Never "partly done" or "not done" on work it undid, nor "partly
        // done" on idle work (no turn: nothing was done).
        Caveat::Partial => !reverted && !turns.is_empty(),
        Caveat::NotDone => !reverted,
        // Only where something blocks.
        Caveat::Blocked => blocks,
        Caveat::Unsure => true,
    }
}

/// The agent's own words a line quotes: its question when it asks, its
/// failure when it failed, else its last words' first sentence.
fn quote_of(now: Now, agent: &AgentFacts) -> (Option<String>, Option<String>) {
    let newest_first = || agent.turns.iter().rev();
    let pick = match now {
        Now::Asks => newest_first()
            .find_map(|turn| turn.question.clone())
            .map(|q| (q, "asks")),
        Now::Failed => newest_first()
            .find_map(|turn| turn.failure.clone())
            .map(|f| (f, "failure")),
        _ => None,
    }
    .or_else(|| {
        agent
            .turns
            .last()
            .and_then(|turn| turn.words.as_deref())
            .map(first_sentence)
            .filter(|words| !words.is_empty())
            .map(|words| (words, "last"))
    });
    match pick {
        Some((words, kind)) => (Some(cap(&words, QUOTE_MAX)), Some(kind.to_owned())),
        None => (None, None),
    }
}

fn first_sentence(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let end = text
        .char_indices()
        .find(|(i, c)| {
            matches!(c, '.' | '!' | '?')
                && text[i + c.len_utf8()..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace)
        })
        .map_or(text.len(), |(i, c)| i + c.len_utf8());
    text[..end].trim().to_owned()
}

/// At most `max` characters, cut at a word with "…".
#[must_use]
pub fn cap(text: &str, max: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        return text;
    }
    let cut: String = text.chars().take(max.saturating_sub(1)).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end())
}

fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

const NEGATIONS: &[&str] = &[
    "not", "didn't", "didn’t", "never", "no", "don't", "don’t", "won't", "won’t", "nicht", "nie",
    "kein", "keine",
];

/// The words say the agent undid its change ("…so I reverted the whole
/// change", "rolled it back", "threw the experiment away"), not negated.
#[must_use]
pub fn says_undone(text: &str) -> bool {
    const ONE: &[&str] = &[
        "revert",
        "reverted",
        "reverts",
        "undid",
        "undone",
        "zurückgenommen",
        "zurückgesetzt",
        "verworfen",
        "weggeworfen",
    ];
    const WORK: &[&str] = &[
        "change",
        "changes",
        "experiment",
        "rewrite",
        "rebuild",
        "branch",
        "work",
        "attempt",
        "prototype",
        "refactor",
    ];
    const FILLER: &[&str] = &["the", "my", "this", "that", "whole", "all", "it", "them"];
    let words = words_of(text);
    let negated = |i: usize| {
        words[i.saturating_sub(3)..i]
            .iter()
            .any(|w| NEGATIONS.contains(&w.as_str()))
    };
    for (i, word) in words.iter().enumerate() {
        let next = |k: usize| words.get(i + k).map(String::as_str);
        let hit = ONE.contains(&word.as_str())
            || (word == "rolled"
                && (next(1) == Some("back")
                    || (next(1).is_some_and(|w| FILLER.contains(&w)) && next(2) == Some("back"))))
            || (word == "rückgängig" && next(1) == Some("gemacht"))
            || (matches!(word.as_str(), "threw" | "thrown")
                && (1..=4).any(|k| next(k) == Some("away")))
            || (matches!(
                word.as_str(),
                "discarded" | "abandoned" | "scrapped" | "dropped"
            ) && (1..=4)
                .map(next)
                .take_while(|w| w.is_some_and(|w| FILLER.contains(&w) || WORK.contains(&w)))
                .any(|w| w.is_some_and(|w| WORK.contains(&w))));
        if hit && !negated(i) {
            return true;
        }
    }
    false
}

/// The words say something blocks the agent ("blocked", "waiting on…",
/// "can't continue", "needs your approval").
#[must_use]
pub fn says_blocked(text: &str) -> bool {
    let words = words_of(text);
    words.iter().enumerate().any(|(i, word)| {
        let next = |k: usize| words.get(i + k).map(String::as_str);
        match word.as_str() {
            "blocked" | "blockiert" => true,
            "blocking" | "blocks" => matches!(next(1), Some("me" | "us" | "it" | "on")),
            "waiting" => matches!(next(1), Some("on" | "for")),
            "warte" | "warten" => next(1) == Some("auf"),
            "can't" | "can’t" | "cannot" => matches!(next(1), Some("continue" | "proceed")),
            "need" | "needs" => {
                matches!(next(1), Some("your" | "an" | "a"))
                    && matches!(
                        next(2),
                        Some("approval" | "answer" | "decision" | "key" | "token" | "access")
                    )
            }
            "komme" => next(1) == Some("nicht") && next(2) == Some("weiter"),
            _ => false,
        }
    })
}

/// The digest's one notification, composed by code from the calm news it
/// carries (ADR-0036 §1.7: needs keep their own path and are never folded
/// in; a model's words never reach a notification). `None` when it
/// carries nothing calm worth a ping.
#[must_use]
pub fn notice_words(digest: &DigestInfo) -> Option<(String, String)> {
    let calm: Vec<&DigestLineInfo> = digest.lines.iter().filter(|l| l.level < 2).collect();
    let finished: Vec<&&DigestLineInfo> = calm.iter().filter(|l| l.state == "finished").collect();
    let mut parts = Vec::new();
    match finished.len() {
        0 => {}
        1 => parts.push(format!("{} finished", finished[0].name)),
        n => parts.push(format!("{n} agents finished")),
    }
    let failing: Vec<String> = calm
        .iter()
        .filter(|l| l.tests.as_deref() == Some("failed"))
        .map(|l| l.name.clone())
        .collect();
    if !failing.is_empty() {
        parts.push(format!("tests failed on {}", list(&failing)));
    }
    let left: Vec<String> = calm
        .iter()
        .filter(|l| l.left_processes.is_some())
        .map(|l| l.name.clone())
        .collect();
    if !left.is_empty() {
        parts.push(format!("{} left processes running", list(&left)));
    }
    let restarted: Vec<String> = calm
        .iter()
        .filter(|l| l.restarted)
        .map(|l| l.name.clone())
        .collect();
    if !restarted.is_empty() {
        parts.push(format!("{} restarted after a crash", list(&restarted)));
    }
    if parts.is_empty() {
        return None;
    }
    let agents = digest.lines.len();
    let title = format!(
        "Digest · {agents} {}",
        if agents == 1 { "agent" } else { "agents" }
    );
    Some((title, parts.join(" · ")))
}

fn list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        [a, b, rest @ ..] => format!("{a}, {b} and {} more", rest.len()),
    }
}

#[cfg(test)]
#[path = "rollup_tests.rs"]
mod tests;
