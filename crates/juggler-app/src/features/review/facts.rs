//! The code-only review (ADR-0033 §6, AJ-E3820 R1): built from the turn's
//! facts alone, no model. It is the default and the no-model path, and
//! what the model's lines are checked against.
//!
//! **Where each part comes from** (the maintainer's ruling, 2026-11-26:
//! from the session's own transcript, never from git in the user's
//! projects):
//! - **Changed:** the turn's own edit and write calls, through the feed's
//!   reader ([`juggler_adapters::turn_changes`], AJ-E3713 R6a); a tool
//!   whose calls carry no line counts shows the files only.
//! - **Reverted**, and an abandoned turn: the agent's own words, read by
//!   the digest's facts (G7: [`crate::features::feed::pick::undone`] and
//!   [`crate::features::digest::rollup::says_undone`]). A file the undo
//!   sentence names is reverted; "the whole change" (or a turn of one
//!   file) reverts every file; an undo that names none of several files
//!   makes them "edited", never "changed". **A revert is never listed as
//!   changed.**
//! - **Failures, the question and the tests:** the agent's own sentences
//!   from its reply, word for word (A's excerpt, AJ-E3708's layer), after
//!   the shared input guard.
//! - **Risk:** from those facts; never low on a failed or untested turn.

use juggler_adapters::turn_changes::TurnChanges;

use super::dto::{ReviewFileInfo, TurnReviewInfo};
use crate::features::digest::rollup::says_undone;
use crate::features::feed::pick::{FAILURE, holds_phrase, sentences, undone};

/// The longest sentence quoted (characters), as the digest's quote.
pub const QUOTE_MAX: usize = 240;
/// Files or lines past which a turn is large (risk at least medium).
pub const LARGE_FILES: usize = 10;
/// See [`LARGE_FILES`].
pub const LARGE_LINES: u64 = 500;
/// Questions kept.
const QUESTIONS_MAX: usize = 2;
/// Failure sentences kept.
const FAILURES_MAX: usize = 3;

/// What the review is built from.
#[derive(Clone, Copy, Debug, Default)]
pub struct Facts<'a> {
    /// The turn's key (the feed's: its file and start, hashed).
    pub turn: &'a str,
    /// What its edit and write calls changed; none when they changed no
    /// file.
    pub changes: Option<&'a TurnChanges>,
    /// The agent's reply, redacted and through the guard (untrusted
    /// data); none when the transcript has none.
    pub reply: Option<&'a str>,
    /// The turn ended failed (the feed's `failed` or `crashed`).
    pub failed: bool,
}

/// Its tests, as the agent's words say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tests {
    /// They pass.
    Passed,
    /// Some fail.
    Failed,
    /// Not run.
    NotRun,
}

impl Tests {
    const fn key(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::NotRun => "not_run",
        }
    }
}

/// The risk, lowest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    /// Nothing in the facts asks for a look.
    Low,
    /// Untested, asking, partly undone or large.
    Medium,
    /// Failed.
    High,
}

impl Risk {
    /// Its key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// The risk a key names.
    #[must_use]
    pub fn of(key: &str) -> Option<Self> {
        match key {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }
}

/// Words that cancel a failure word just before it ("0 failed", "no
/// errors", "nicht fehlgeschlagen").
const CANCELS: &[&str] = &[
    "0", "no", "zero", "not", "never", "without", "none", "kein", "keine", "keinen", "nicht",
    "nie", "ohne",
];

/// A failure put right.
const FIXED: &[&str] = &[
    "fixed",
    "fixes",
    "fix ",
    "resolved",
    "repaired",
    "no longer",
    "is gone",
    "are gone",
    "behoben",
    "gelöst",
    "repariert",
];

/// A failure still there, fix or not.
const STILL: &[&str] = &["still", "again", "remain", "noch", "weiterhin"];

/// Words that say a sentence is about tests.
const TEST_WORDS: &[&str] = &[
    "test",
    "tests",
    "tested",
    "testing",
    "suite",
    "spec",
    "specs",
    "pytest",
    "jest",
    "vitest",
    "tests,",
    "testlauf",
    "testsuite",
];

/// A test run not done.
const NOT_RUN: &[&str] = &[
    "not run",
    "didn't run",
    "didn’t run",
    "did not run",
    "haven't run",
    "haven’t run",
    "have not run",
    "unrun",
    "untested",
    "not tested",
    "not yet run",
    "still unrun",
    "skipped",
    "couldn't run",
    "couldn’t run",
    "could not run",
    "nicht ausgeführt",
    "nicht gelaufen",
    "nicht getestet",
];

/// A test run that passed.
const PASSED: &[&str] = &[
    "pass",
    "passed",
    "passes",
    "passing",
    "green",
    "bestanden",
    "laufen durch",
    "grün",
];

/// Undoing everything, whichever file.
const WHOLE: &[&str] = &[
    "whole",
    "everything",
    "all of it",
    "all my changes",
    "all the changes",
    "all changes",
    "no code changes remain",
    "no changes remain",
    "nothing was kept",
    "back to where",
    "back to how",
    "komplett",
    "alles",
];

fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether the sentence says something failed: a failure word not
/// cancelled by the word or two before it.
fn says_failed(sentence: &str) -> bool {
    let words = words_of(sentence);
    let low = sentence.to_lowercase();
    // A failure fixed is not a failure ("fixed the failing test"), unless
    // it is still there.
    if holds_phrase(&low, FIXED) && !holds_phrase(&low, STILL) {
        return false;
    }
    for (i, word) in words.iter().enumerate() {
        let hit = FAILURE
            .iter()
            .any(|f| !f.contains(' ') && word.starts_with(f))
            || FAILURE.iter().filter(|f| f.contains(' ')).any(|f| {
                let mut parts = f.split(' ');
                parts.next() == Some(word.as_str())
                    && parts.next() == words.get(i + 1).map(String::as_str)
            });
        if !hit {
            continue;
        }
        let cancelled = words[i.saturating_sub(2)..i]
            .iter()
            .any(|w| CANCELS.contains(&w.as_str()));
        if !cancelled {
            return true;
        }
    }
    false
}

fn about_tests(sentence: &str) -> bool {
    words_of(sentence)
        .iter()
        .any(|w| TEST_WORDS.contains(&w.as_str()) || w.starts_with("test"))
}

/// The tests its sentences say, most serious first, with the sentence.
fn tests_of(said: &[String]) -> Option<(Tests, String)> {
    let mut best: Option<(Tests, String)> = None;
    let rank = |t: Tests| match t {
        Tests::Failed => 3,
        Tests::NotRun => 2,
        Tests::Passed => 1,
    };
    for sentence in said.iter().filter(|s| about_tests(s)) {
        let low = sentence.to_lowercase();
        let found = if says_failed(sentence) {
            Some(Tests::Failed)
        } else if holds_phrase(&low, NOT_RUN) {
            Some(Tests::NotRun)
        } else if holds_phrase(&low, PASSED) {
            Some(Tests::Passed)
        } else {
            None
        };
        if let Some(found) = found
            && best.as_ref().is_none_or(|(t, _)| rank(found) > rank(*t))
        {
            best = Some((found, sentence.clone()));
        }
    }
    best
}

/// A sentence cut to [`QUOTE_MAX`] characters.
fn quoted(sentence: &str) -> String {
    let s = sentence.trim();
    if s.chars().count() <= QUOTE_MAX {
        return s.to_owned();
    }
    let cut: String = s.chars().take(QUOTE_MAX - 1).collect();
    format!("{}…", cut.trim_end())
}

/// A file's last part ("src/quoting.py" → "quoting.py"), lower case.
fn last_part(name: &str) -> String {
    std::path::Path::new(name)
        .file_name()
        .map_or_else(|| name.to_owned(), |n| n.to_string_lossy().into_owned())
        .to_lowercase()
}

/// A file's stem ("src/quoting.py" → "quoting"), lower case.
fn stem(name: &str) -> String {
    let last = last_part(name);
    last.split('.').next().unwrap_or(&last).to_owned()
}

/// Whether the sentence names the file: its name, its last part, or its
/// stem as a word.
fn names_file(sentence: &str, name: &str) -> bool {
    let low = sentence.to_lowercase();
    let last = last_part(name);
    let stem = stem(name);
    low.contains(&name.to_lowercase())
        || low.contains(&last)
        || (stem.chars().count() >= 3 && words_of(sentence).contains(&stem))
}

/// The reply's sentences, trimmed, none empty.
#[must_use]
pub fn said(reply: &str) -> Vec<String> {
    sentences(reply)
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The code-only review of a turn (pure, under a millisecond).
#[must_use]
pub fn review(facts: &Facts<'_>) -> TurnReviewInfo {
    let said = facts.reply.map(said).unwrap_or_default();
    // Undo: the digest's facts, sentence by sentence.
    let undo: Vec<&String> = said
        .iter()
        .filter(|s| undone(s) || says_undone(s))
        .collect();
    let files: Vec<ReviewFileInfo> = facts
        .changes
        .map(|changes| {
            changes
                .per_file
                .iter()
                .map(|file| ReviewFileInfo {
                    name: file.name.clone(),
                    added: file
                        .lines
                        .map(|l| u32::try_from(l.added).unwrap_or(u32::MAX)),
                    removed: file
                        .lines
                        .map(|l| u32::try_from(l.removed).unwrap_or(u32::MAX)),
                    about: None,
                })
                .collect()
        })
        .unwrap_or_default();
    let whole = undo.iter().any(|s| holds_phrase(&s.to_lowercase(), WHOLE))
        || (files.len() == 1 && !undo.is_empty());
    let mut changed = Vec::new();
    let mut reverted = Vec::new();
    for file in files {
        if whole || undo.iter().any(|s| names_file(s, &file.name)) {
            reverted.push(file.name);
        } else {
            changed.push(file);
        }
    }
    // An undo that names none of several files: none is said as changed.
    let undone_unclear = !undo.is_empty() && !whole && reverted.is_empty() && !changed.is_empty();
    let tests = tests_of(&said);
    // The tests' own sentence is said once, as the tests.
    let tests_sentence = tests.as_ref().map(|(_, s)| s.as_str());
    let failures: Vec<String> = said
        .iter()
        .filter(|s| says_failed(s) && !s.trim_end().ends_with('?'))
        .filter(|s| Some(s.as_str()) != tests_sentence)
        .take(FAILURES_MAX)
        .map(|s| quoted(s))
        .collect();
    let questions: Vec<&String> = said
        .iter()
        .filter(|s| s.trim_end().ends_with('?'))
        .collect();
    let question = (!questions.is_empty()).then(|| {
        quoted(
            &questions[questions.len().saturating_sub(QUESTIONS_MAX)..]
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        )
    });
    let (risk, risk_why) = risk_of(
        facts,
        &changed,
        tests.as_ref(),
        &failures,
        question.as_ref(),
        &reverted,
        undone_unclear,
    );
    TurnReviewInfo {
        state: "shown".to_owned(),
        turn: facts.turn.to_owned(),
        changed,
        reverted,
        undone_unclear,
        reverted_words: undo.first().map(|s| quoted(s)),
        failures,
        question,
        tests: tests.as_ref().map(|(t, _)| t.key().to_owned()),
        tests_words: tests.map(|(_, s)| quoted(&s)),
        risk: risk.key().to_owned(),
        risk_why,
        ai: None,
    }
}

/// The risk from the facts, and why (most serious first).
fn risk_of(
    facts: &Facts<'_>,
    changed: &[ReviewFileInfo],
    tests: Option<&(Tests, String)>,
    failures: &[String],
    question: Option<&String>,
    reverted: &[String],
    unclear: bool,
) -> (Risk, Vec<String>) {
    let mut why: Vec<(Risk, &str)> = Vec::new();
    let tests = tests.map(|(t, _)| *t);
    if facts.failed {
        why.push((Risk::High, "failed"));
    }
    if tests == Some(Tests::Failed) {
        why.push((Risk::High, "tests_failed"));
    } else if !failures.is_empty() {
        why.push((Risk::High, "failures"));
    }
    let edited = !changed.is_empty();
    if edited && tests == Some(Tests::NotRun) {
        why.push((Risk::Medium, "tests_not_run"));
    } else if edited && tests.is_none() {
        why.push((Risk::Medium, "untested"));
    }
    if question.is_some() {
        why.push((Risk::Medium, "asks"));
    }
    if !reverted.is_empty() || unclear {
        why.push((Risk::Medium, "reverted"));
    }
    let lines: u64 = changed
        .iter()
        .map(|f| u64::from(f.added.unwrap_or(0)) + u64::from(f.removed.unwrap_or(0)))
        .sum();
    if changed.len() >= LARGE_FILES || lines >= LARGE_LINES {
        why.push((Risk::Medium, "large"));
    }
    let risk = why.iter().map(|(r, _)| *r).max().unwrap_or(Risk::Low);
    (risk, why.into_iter().map(|(_, w)| w.to_owned()).collect())
}

/// The risk floor the facts set: the model may raise it, never lower it.
#[must_use]
pub fn floor(review: &TurnReviewInfo) -> Risk {
    Risk::of(&review.risk).unwrap_or(Risk::High)
}
