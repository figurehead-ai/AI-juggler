//! 0.5.2 G48: one reading of a finished turn, shared by the turn review,
//! the feed line and the digest. **Results first, words second.**
//!
//! - **Tests:** a test runner's exit beats what the agent says of its
//!   tests ("with the old file restored, the new test fails" is not a
//!   failing run when the last run passed). Only a runner makes a command
//!   a test run (`ls tests` is not one), and a run the tool refused pending
//!   approval is "not run", never "failed" (the adapter's `blocked`).
//! - **Undo:** read from the reply's own sentences only, negation-aware
//!   ("I did not revert anything" is no undo), and never a verify-and-
//!   restore ("I put the fix back afterwards", "against the old code").
//! - **What reaches a model or a screen** is the guarded text: the caller
//!   puts the agent's words through Bot's input guard first (G49).
//!
//! Everything is read from the session's own transcript through the
//! page's reader; nothing runs a command or reads git.

use std::path::Path;

use juggler_adapters::turn_changes::SLACK_MS;

use super::facts::Tests;
use crate::features::conversations::feed::dto::TurnInfo;
use crate::features::conversations::last_messages::{self, Ask, TURNS_MAX};
use crate::features::digest::rollup::says_undone;
use crate::features::feed::pick::{holds_phrase, sentences, undone};

/// Words that make a command a test run, wherever they stand.
const RUNNERS: &[&str] = &[
    "unittest", "pytest", "nose2", "tox", "jest", "vitest", "mocha", "rspec", "phpunit", "nextest",
    "ctest", "testthat",
];
/// Tools whose next word `test` runs tests (`cargo test`, `go test`).
const TEST_TOOLS: &[&str] = &[
    "cargo",
    "go",
    "npm",
    "pnpm",
    "yarn",
    "bun",
    "make",
    "just",
    "mvn",
    "gradle",
    "gradlew",
    "dotnet",
    "swift",
    "deno",
    "playwright",
    "mix",
    "flutter",
    "dart",
];

/// How the last test run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    /// It ran and passed.
    Passed,
    /// It ran and failed.
    Failed,
    /// The tool didn't run it (approval, sandbox, the user's no).
    Blocked,
}

/// The turn's tool calls, as their results say.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Runs {
    /// Calls per tool, in the order first used.
    pub by_tool: Vec<(String, u32)>,
    /// The commands it ran, each with whether it failed, in order.
    pub commands: Vec<(String, bool)>,
    /// How many test runs.
    pub test_runs: u32,
    /// How the last one ended; `None` when none ran.
    pub last: Option<Run>,
    /// Earlier calls the page's reader didn't keep (not in the counts).
    pub hidden: u32,
}

impl Runs {
    /// The tests, results first: the last run's, else the words'.
    #[must_use]
    pub fn tests(&self, words: Option<Tests>) -> Option<Tests> {
        match self.last {
            Some(Run::Passed) => Some(Tests::Passed),
            // A refusal the adapter didn't recognise still exits non-zero:
            // the agent saying the tests weren't run is the better word.
            Some(Run::Failed) if words == Some(Tests::NotRun) => Some(Tests::NotRun),
            Some(Run::Failed) => Some(Tests::Failed),
            Some(Run::Blocked) => Some(Tests::NotRun),
            None => words,
        }
    }
}

/// Whether `command` runs tests: a runner's name, or a tool and `test`
/// (`cargo test`, `npm run test`, `python3 manage.py test`).
#[must_use]
pub fn is_test_runner(command: &str) -> bool {
    let words: Vec<String> = command
        .to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.' || c == '-'))
        .filter(|w| !w.is_empty())
        // A runner named by its path (`./node_modules/.bin/jest`): its
        // file name, whatever the separator.
        .map(|w| {
            std::path::Path::new(w)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(w)
                .to_owned()
        })
        .collect();
    words.iter().enumerate().any(|(i, w)| {
        RUNNERS.contains(&w.as_str())
            || (w == "test"
                && i > 0
                && (TEST_TOOLS.contains(&words[i - 1].as_str())
                    || words[i - 1] == "manage.py"
                    || (words[i - 1] == "run"
                        && i > 1
                        && TEST_TOOLS.contains(&words[i - 2].as_str()))))
    })
}

/// The calls of `turns`, read from their results.
#[must_use]
pub fn runs_of(turns: &[&TurnInfo]) -> Runs {
    let mut runs = Runs::default();
    let steps = || {
        turns
            .iter()
            .flat_map(|t| &t.blocks)
            .filter(|b| b.kind == "steps")
    };
    runs.hidden = steps().map(|b| b.hidden).sum();
    for call in steps().flat_map(|b| &b.calls) {
        match runs.by_tool.iter_mut().find(|(t, _)| *t == call.tool) {
            Some((_, n)) => *n += 1,
            None => runs.by_tool.push((call.tool.clone(), 1)),
        }
        let Some(command) = call.command.as_deref() else {
            continue;
        };
        let command = command.split_whitespace().collect::<Vec<_>>().join(" ");
        let blocked = call.result_kind.as_deref() == Some("blocked");
        if is_test_runner(&command) && call.done {
            runs.test_runs += 1;
            runs.last = Some(if blocked {
                Run::Blocked
            } else if call.failed {
                Run::Failed
            } else {
                Run::Passed
            });
        }
        runs.commands.push((command, call.failed && !blocked));
    }
    runs
}

/// A sentence that checks a fix by undoing it for a moment ("with the
/// old load.py restored, the new test fails", "I put the fix back
/// afterwards", "against the original code"): a verification, never an
/// undo or a failure of the turn.
#[must_use]
pub fn verification(sentence: &str) -> bool {
    let low = sentence.to_lowercase().replace('’', "'");
    holds_phrase(
        &low,
        &[
            "put the fix back",
            "put it back",
            "put the change back",
            "back afterwards",
            "with the old",
            "with the original",
            "against the old",
            "against the original",
            "on the old code",
            "on the original code",
            "without the fix",
            "original line",
            "not from a run",
            "from reading the",
            "before the fix",
            "to check that",
            "to prove",
            "to make sure the test",
            "catches the bug",
            "catch this bug",
            "would fail",
            "would have failed",
            "fail there",
            "fails there",
        ],
    )
}

/// Whether one of the agent's sentences says it undid its change:
/// negation-aware, never a verification.
#[must_use]
pub fn undone_sentence(sentence: &str) -> bool {
    if verification(sentence) {
        return false;
    }
    let low = sentence.to_lowercase().replace('’', "'");
    let negated = holds_phrase(
        &low,
        &[
            "not ",
            "n't ",
            "no need",
            "nothing to",
            "never ",
            "without ",
            "nicht ",
            "kein",
        ],
    );
    says_undone(sentence) || (undone(sentence) && !negated)
}

/// Whether the reply says the change was undone (its own sentences).
#[must_use]
pub fn undone_in(reply: &str) -> bool {
    sentences(reply).iter().any(|s| undone_sentence(s))
}

/// Words that say a part of the request isn't done, or that the agent
/// stopped for the user (G50, the plain excerpt's lead).
const NOT_DONE: &[&str] = &[
    "couldn't",
    "could not",
    "i can't",
    "i cannot",
    "unable to",
    "wasn't able",
    "not able to",
    "not possible",
    "isn't installed",
    "is not installed",
    "no network",
    "still fails",
    "still failing",
    "still open",
    "stop here",
    "stopping here",
    "haven't changed anything",
    "haven't done",
    "haven't pointed",
    "haven't built",
    "haven't implemented",
    "didn't publish",
    "didn't push",
    "need you to",
    "needs your",
    "before i change",
    "before i build",
    "before i go on",
    "konnte nicht",
    "kann ich nicht",
    "nicht möglich",
    "habe nichts geändert",
    "höre hier auf",
    "brauche ich",
];

/// G50: the reply's first sentence that says a part isn't done or that it
/// waits on the user, in its own words; `None` when it says none.
#[must_use]
pub fn lead(reply: &str) -> Option<String> {
    let said: Vec<String> = sentences(reply)
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    let says = |s: &String| {
        let low = s.to_lowercase().replace('’', "'");
        !verification(s) && holds_phrase(&low, NOT_DONE)
    };
    // A reply that opens with its undo or its own "not done" leads with it
    // already (the first sentence is the excerpt).
    if said.first().is_some_and(|s| undone_sentence(s) || says(s)) {
        return None;
    }
    said.into_iter().find(says)
}

fn text_of(turn: &TurnInfo) -> String {
    turn.blocks
        .iter()
        .filter(|b| b.kind == "text" || b.kind == "narration")
        .map(|b| b.text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn asked(turn: &TurnInfo) -> bool {
    matches!(turn.speaker.as_str(), "you" | "user") && !text_of(turn).is_empty()
}

/// A finished turn as its transcript holds it (the agent's words not yet
/// guarded: the caller guards what it shows or sends).
#[derive(Clone, Debug, Default)]
pub struct TurnRead {
    /// The user's request that started it.
    pub request: Option<String>,
    /// The session's opening request, when the turn isn't the first.
    pub opening: Option<String>,
    /// The agent's messages in order (its words and narration).
    pub messages: Vec<String>,
    /// Its reply: its tool's marked final answer, else its last words when
    /// the turn ends on them.
    pub reply: Option<String>,
    /// Its calls, from their results.
    pub runs: Runs,
}

/// Reads the turn that ended by `until` (the last one when `None`).
///
/// # Errors
///
/// `"none"` when the transcript holds no readable turns (compressed or
/// empty), `"unreadable"` when it can't be read.
pub fn read(path: &Path, until: Option<i64>) -> Result<TurnRead, &'static str> {
    let page = last_messages::read(
        Some(path),
        &Ask {
            max: Some(TURNS_MAX),
            ..Ask::default()
        },
    );
    match page.state.as_str() {
        "shown" => {}
        "compressed" | "none" => return Err("none"),
        _ => return Err("unreadable"),
    }
    let turns = &page.turns;
    let Some(end) = turns.iter().rposition(|t| {
        t.speaker == "assistant"
            && match (until, t.at_ms) {
                (Some(until), Some(at)) => at <= until + SLACK_MS,
                (Some(_), None) => false,
                (None, _) => true,
            }
    }) else {
        return Ok(TurnRead::default());
    };
    let start = turns[..end].iter().rposition(asked);
    let agent: Vec<&TurnInfo> = turns[start.map_or(0, |s| s + 1)..=end]
        .iter()
        .filter(|t| t.speaker == "assistant")
        .collect();
    let messages: Vec<String> = agent
        .iter()
        .flat_map(|t| {
            t.blocks
                .iter()
                .filter(|b| b.kind == "text" || b.kind == "narration")
        })
        .map(|b| b.text.trim().to_owned())
        .filter(|t| !t.is_empty())
        .collect();
    let last = &turns[end];
    let marked = last
        .blocks
        .iter()
        .rev()
        .find(|block| block.kind == "text" && block.final_answer);
    let reply = marked
        .or_else(|| last.blocks.last().filter(|block| block.kind == "text"))
        .map(|block| block.text.trim().to_owned())
        .filter(|text| !text.is_empty());
    let opening = turns
        .iter()
        .position(asked)
        .filter(|&i| start.is_some_and(|s| i < s))
        .map(|i| text_of(&turns[i]));
    Ok(TurnRead {
        request: start.map(|s| text_of(&turns[s])),
        opening,
        messages,
        reply,
        runs: runs_of(&agent),
    })
}

#[cfg(test)]
#[path = "turn_tests.rs"]
pub(crate) mod tests;
