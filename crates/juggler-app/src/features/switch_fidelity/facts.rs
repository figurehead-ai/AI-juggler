//! Planted facts: distinct things said in the early, middle and late
//! turns of a source conversation (some only in the model's replies,
//! never written to a file, and a decision made only in the
//! conversation), asked of the target after a switch, and scored
//! answered, wrong or unknown.

use std::fmt::Write as _;

/// How a fact got into the conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Kind {
    /// The person said it.
    Said,
    /// The model chose it and said it only in its reply.
    ReplyOnly,
    /// A decision made in the conversation, written nowhere.
    Decision,
    /// A rule only a rules file states (the target's tool may not read
    /// that file itself).
    Rule,
    /// The open items of the task list.
    Tasks,
}

impl Kind {
    /// The words the table uses.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::Said => "said by the person",
            Self::ReplyOnly => "only in a reply",
            Self::Decision => "a decision, only in the conversation",
            Self::Rule => "a rules file",
            Self::Tasks => "the task list",
        }
    }
}

/// One fact to probe.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Fact {
    /// The answer line's key (`PORT`).
    pub key: String,
    /// The question, a few words.
    pub ask: String,
    /// Every word a right answer holds (any case).
    pub expect: Vec<String>,
    /// Where it was planted: "A, turn 2 of 9 (early)".
    pub planted: String,
    /// How it got there.
    pub kind: Kind,
}

/// How the target answered a fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Score {
    /// Right.
    Answered,
    /// Something else.
    Wrong,
    /// It said it doesn't know, or said nothing.
    Unknown,
}

impl Score {
    /// The word the table uses.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Answered => "answered",
            Self::Wrong => "wrong",
            Self::Unknown => "unknown",
        }
    }
}

/// A fact with what the target said about it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Probed {
    /// The fact.
    pub fact: Fact,
    /// Whether any file in the project holds it (a planted fact must
    /// not: only the conversation can know it).
    pub in_a_file: bool,
    /// The value the target gave (its answer line).
    pub given: String,
    /// The score.
    pub score: Score,
}

/// One row of a hop's facts table.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct FactRow {
    /// The probed fact.
    pub probed: Probed,
    /// Whether the source conversation held it at switch time.
    pub in_source: bool,
    /// Whether what the target received holds it.
    pub received: bool,
}

/// The probe: every fact's question, answered one per line as
/// `KEY=value`, `UNKNOWN` for what the target doesn't know.
#[must_use]
pub fn question(facts: &[Fact]) -> String {
    let mut out = String::from(
        "Answer from this conversation only; don't change any file. Give one line per item, \
         exactly `KEY=value`, and `KEY=UNKNOWN` when you don't know:",
    );
    for fact in facts {
        let _ = write!(out, "\n{}: {}", fact.key, fact.ask);
    }
    out
}

/// Scores `answer` for `fact`: its `KEY=` (or `KEY:`) line, else the
/// whole answer when no line names the key.
#[must_use]
pub fn score(answer: &str, fact: &Fact, in_a_file: bool) -> Probed {
    let line = answer.lines().find_map(|line| value_of(line, &fact.key));
    let given = line.clone().unwrap_or_default();
    let holds_all = |text: &str| {
        let text = text.to_lowercase();
        fact.expect
            .iter()
            .all(|word| text.contains(&word.to_lowercase()))
    };
    let score = match &line {
        Some(value) if value.trim().is_empty() || value.to_lowercase().contains("unknown") => {
            Score::Unknown
        }
        Some(value) if holds_all(value) => Score::Answered,
        Some(_) => Score::Wrong,
        None if !fact.expect.is_empty() && holds_all(answer) => Score::Answered,
        None => Score::Unknown,
    };
    Probed {
        fact: fact.clone(),
        in_a_file,
        given,
        score,
    }
}

/// The value of `key` on `line` (`KEY=value`, `- **KEY**: value`), any
/// case; `None` when the line is about something else.
fn value_of(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim_start_matches(|c: char| !c.is_alphanumeric());
    let rest = trimmed.get(..key.len())?;
    if !rest.eq_ignore_ascii_case(key) {
        return None;
    }
    let after = trimmed[key.len()..].trim_start_matches(['*', '`', ' ']);
    let value = after
        .strip_prefix('=')
        .or_else(|| after.strip_prefix(':'))?;
    Some(value.trim().trim_matches(['`', '*', '"']).trim().to_owned())
}

/// The value the model gave as `MARKER=value` in a reply of
/// `conversation` (a fact it chose and said only there).
#[must_use]
pub fn said_as(conversation: &super::Conversation, marker: &str) -> Option<String> {
    let prefix = format!("{marker}=");
    conversation
        .turns
        .iter()
        .filter(|turn| !turn.user)
        .find_map(|turn| {
            let at = turn.text.find(&prefix)? + prefix.len();
            let value: String = turn.text[at..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
                .collect();
            (!value.is_empty()).then_some(value)
        })
}
