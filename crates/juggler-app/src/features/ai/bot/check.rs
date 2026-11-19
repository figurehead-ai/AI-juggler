//! The check a written answer must pass before it is shown (AJ-E3806,
//! ADR-0034 §7.2, the harness's `claims_v5.py` as a spec): every name,
//! number and state in it is found in the records it cites. Pure, a few
//! microseconds; a failing answer is never shown, and the records are
//! shown instead (§7.3's ladder).
//!
//! Strict on purpose: a missed fact is an answer replaced by the rows
//! (an omission, safe); a wrong one shown would be the harm.

use super::dto::{BotRowInfo, BotState, BotWrittenPart};

/// Why a written answer was refused (a code, for the log; never shown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Empty, too long, more than three sentences, a link or markup.
    Shape,
    /// It cites a record that wasn't issued for this answer.
    UnknownCite(u32),
    /// A sentence states something and cites nothing.
    Uncited,
    /// A number no cited record holds.
    Number(String),
    /// A name no cited record holds.
    Name(String),
    /// A state that disagrees with the cited records', or one they don't
    /// hold ("deployed", "merged").
    State(String),
    /// It says Bot did or will do something.
    Acting,
    /// It repeats words the guard removed.
    Echo,
}

impl Refused {
    /// The code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Shape => "shape",
            Self::UnknownCite(_) => "unknown_cite",
            Self::Uncited => "uncited",
            Self::Number(_) => "number",
            Self::Name(_) => "name",
            Self::State(_) => "state",
            Self::Acting => "acting",
            Self::Echo => "echo",
        }
    }
}

const MAX_WORDS: usize = 60;
const MAX_SENTENCES: usize = 3;

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .filter(|w| !w.is_empty())
        .map(|w| w.replace('’', "'").to_lowercase())
        .collect()
}

/// Number words, as the numbers they say (English and German).
fn number_word(word: &str) -> Option<u64> {
    Some(match word {
        "one" | "ein" | "eine" | "einen" | "eins" => 1,
        "two" | "both" | "zwei" | "beide" | "beiden" => 2,
        "three" | "drei" => 3,
        "four" | "vier" => 4,
        "five" | "fünf" => 5,
        "six" | "sechs" => 6,
        "seven" | "sieben" => 7,
        "eight" | "acht" => 8,
        "nine" | "neun" => 9,
        "ten" | "zehn" => 10,
        _ => return None,
    })
}

/// State words and the states they claim.
fn state_word(word: &str) -> Option<&'static [BotState]> {
    Some(match word {
        "finished" | "finish" | "finishes" | "completed" | "complete" | "fertig"
        | "abgeschlossen" => &[BotState::Finished, BotState::Done],
        "done" | "erledigt" => &[BotState::Done, BotState::Finished],
        "ended" | "beendet" => &[BotState::Ended, BotState::Finished],
        "failed" | "fails" | "failing" | "error" | "errors" | "fehlgeschlagen" | "fehler" => {
            &[BotState::Failed]
        }
        "stuck" | "hängt" | "festgefahren" => &[BotState::Stuck],
        "asks" | "asking" | "waiting" | "waits" | "fragt" | "wartet" => {
            &[BotState::AsksYou, BotState::Blocked, BotState::InReview]
        }
        "blocked" | "blockiert" => &[BotState::Blocked],
        "working" | "running" | "arbeitet" | "läuft" | "progress" => &[BotState::Working],
        "review" | "reviewed" => &[BotState::InReview],
        "limit" | "limits" => &[BotState::Limit],
        _ => return None,
    })
}

/// Outcomes AI Juggler's records never hold as a state: claiming one is
/// claiming something the records can't back.
const UNHELD: &[&str] = &[
    "deployed",
    "deploy",
    "merged",
    "shipped",
    "released",
    "approved",
    "passed",
    "pushed",
    "committed",
    "tested",
    "fixed",
    "resolved",
    "succeeded",
    "success",
    "successful",
    "green",
    "broken",
    "reverted",
    "live",
    "bestanden",
    "behoben",
    "veröffentlicht",
    "gemergt",
    "deployt",
];

const ACT_SUBJECTS: &[&str] = &["i", "i've", "i'll", "i'm", "ich", "we", "we've", "we'll"];
const ACT_VERBS: &[&str] = &[
    "stop",
    "stopped",
    "send",
    "sent",
    "switch",
    "switched",
    "approve",
    "approved",
    "restart",
    "restarted",
    "start",
    "started",
    "cancel",
    "cancelled",
    "canceled",
    "set",
    "created",
    "made",
    "moved",
    "closed",
    "opened",
    "told",
    "asked",
    "watch",
    "watching",
    "remove",
    "removed",
    "gestoppt",
    "gesendet",
    "geschickt",
    "gewechselt",
    "genehmigt",
    "gestartet",
    "erstellt",
    "beendet",
    "abgebrochen",
];

/// Words an answer may use freely: what the window's own words say about
/// records (row labels), grammar, and time.
const GRAMMAR: &[&str] = &["i", "ai", "juggler", "bot"];

fn evidence_text(row: &BotRowInfo) -> String {
    let mut parts: Vec<String> = vec![row.title.clone()];
    for name in [
        &row.project,
        &row.app_label,
        &row.pool,
        &row.holder,
        &row.branch,
    ]
    .into_iter()
    .flatten()
    {
        parts.push(name.clone());
    }
    for number in [row.files_changed, row.used_percent, row.blocked_by]
        .into_iter()
        .flatten()
    {
        parts.push(number.to_string());
    }
    parts.join(" \u{1f} ")
}

fn numbers_in(text: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut current = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            current.push(c);
        } else if !current.is_empty() {
            if let Ok(n) = current.parse() {
                out.push(n);
            }
            current.clear();
        }
    }
    out
}

/// The quoted spans in a sentence.
fn quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut open: Option<usize> = None;
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        match (c, open) {
            ('“' | '"' | '„' | '«', None) => open = Some(i),
            ('”' | '"' | '“' | '»', Some(start)) if i > start + 1 => {
                out.push(chars[start + 1..i].iter().collect());
                open = None;
            }
            _ => {}
        }
    }
    out
}

/// Whether the word is one a name is made of: capitalised mid-sentence,
/// or code-shaped (`feat/limits`, `auth-v2`, `m3`).
fn name_like(original: &str, first: bool) -> bool {
    let has_letter = original.chars().any(char::is_alphabetic);
    let code = original.chars().any(|c| matches!(c, '/' | '_' | '-' | '.')) && original.len() > 2
        || (has_letter && original.chars().any(|c| c.is_ascii_digit()));
    let capital = !first && original.chars().next().is_some_and(char::is_uppercase);
    code || capital
}

/// Whether a sentence states a fact: a name, a number or a state.
fn states_anything(text: &str) -> bool {
    let ws = words(text);
    !quoted(text).is_empty()
        || !numbers_in(text).is_empty()
        || ws.iter().any(|w| {
            state_word(w).is_some() || number_word(w).is_some() || UNHELD.contains(&w.as_str())
        })
        || text.split_whitespace().skip(1).any(|w| name_like(w, false))
}

/// Checks a written answer against the rows it may cite (1-based) and
/// the question; `removed` is what the guard took out of those rows.
///
/// # Errors
///
/// The first rule it breaks.
pub fn check(
    parts: &[BotWrittenPart],
    rows: &[BotRowInfo],
    question: &str,
    removed: &[String],
) -> Result<(), Refused> {
    let all: String = parts
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let word_count = all.split_whitespace().count();
    if parts.is_empty()
        || parts.len() > MAX_SENTENCES
        || word_count == 0
        || word_count > MAX_WORDS
        || all.contains("http")
        || all.contains("://")
        || all
            .chars()
            .any(|c| c.is_control() || matches!(c, '*' | '#' | '`' | '[' | ']' | '<' | '>' | '|'))
    {
        return Err(Refused::Shape);
    }
    let lower_all = all.to_lowercase();
    for gone in removed {
        let gone_words = words(gone);
        if gone_words.len() >= 4
            && gone_words
                .windows(4)
                .any(|w| lower_all.contains(&w.join(" ")))
        {
            return Err(Refused::Echo);
        }
    }
    let question_lower = question.to_lowercase();
    let question_numbers = numbers_in(question);
    for part in parts {
        for &cite in &part.cites {
            if cite == 0 || cite as usize > rows.len() {
                return Err(Refused::UnknownCite(cite));
            }
        }
        let cited: Vec<&BotRowInfo> = part
            .cites
            .iter()
            .filter_map(|&c| rows.get(c as usize - 1))
            .collect();
        if cited.is_empty() && states_anything(&part.text) {
            return Err(Refused::Uncited);
        }
        let evidence = cited
            .iter()
            .map(|r| evidence_text(r))
            .collect::<Vec<_>>()
            .join(" \u{1f} ")
            .to_lowercase();
        let evidence_words = words(&evidence);
        let sentence_words = words(&part.text);
        // Bot never acts in 0.5: "I stopped", "I'll send".
        if sentence_words
            .windows(2)
            .any(|w| ACT_SUBJECTS.contains(&w[0].as_str()) && ACT_VERBS.contains(&w[1].as_str()))
            || sentence_words.windows(3).any(|w| {
                ACT_SUBJECTS.contains(&w[0].as_str()) && ACT_VERBS.contains(&w[2].as_str())
            })
        {
            return Err(Refused::Acting);
        }
        let mut states: Vec<&[BotState]> = Vec::new();
        for word in &sentence_words {
            if UNHELD.contains(&word.as_str()) && !evidence_words.contains(word) {
                return Err(Refused::State(word.clone()));
            }
            if let Some(claim) = state_word(word) {
                states.push(claim);
            }
        }
        // Numbers: digits and number words, each held by a cited row, the
        // question, or the count of the cited rows.
        let counts: Vec<u64> = {
            let mut c = vec![cited.len() as u64, rows.len() as u64];
            for kind in [
                super::dto::BotRowKind::Session,
                super::dto::BotRowKind::Task,
                super::dto::BotRowKind::Account,
            ] {
                c.push(cited.iter().filter(|r| r.kind == kind).count() as u64);
            }
            c
        };
        let held_numbers = numbers_in(&evidence);
        for n in numbers_in(&part.text)
            .into_iter()
            .chain(sentence_words.iter().filter_map(|w| number_word(w)))
        {
            if !(held_numbers.contains(&n) || question_numbers.contains(&n) || counts.contains(&n))
            {
                return Err(Refused::Number(n.to_string()));
            }
        }
        // Names: quoted spans, capitalised and code-shaped words.
        for name in quoted(&part.text) {
            let name = name.to_lowercase();
            if !(evidence.contains(&name) || question_lower.contains(&name)) {
                return Err(Refused::Name(name));
            }
        }
        let originals: Vec<&str> = part
            .text
            .split(|c: char| {
                c.is_whitespace()
                    || matches!(c, ',' | ';' | ':' | '(' | ')' | '“' | '”' | '"' | '„')
            })
            .map(|w| w.trim_end_matches(['.', '!', '?']))
            .filter(|w| !w.is_empty())
            .collect();
        for (i, original) in originals.iter().enumerate() {
            if !name_like(original, i == 0) {
                continue;
            }
            let lower = original.to_lowercase();
            if GRAMMAR.contains(&lower.as_str()) {
                continue;
            }
            if !(evidence.contains(&lower) || question_lower.contains(&lower)) {
                return Err(Refused::Name(lower));
            }
        }
        // States: each state word agrees with one of the cited rows'.
        for claim in states {
            if !cited
                .iter()
                .any(|r| r.state.is_some_and(|s| claim.contains(&s)))
            {
                return Err(Refused::State(format!("{claim:?}")));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "a test: a failed step fails the test"
    )]

    use super::super::dto::BotRowKind;
    use super::*;

    fn row(title: &str, state: BotState, files: Option<u32>) -> BotRowInfo {
        let mut r =
            super::super::world::tests_support::row(BotRowKind::Session, title, Some(state));
        r.files_changed = files;
        r.project = Some("shop".into());
        r
    }

    fn part(text: &str, cites: &[u32]) -> BotWrittenPart {
        BotWrittenPart {
            text: text.into(),
            cites: cites.to_vec(),
        }
    }

    #[test]
    fn an_answer_whose_facts_are_all_in_its_records_passes() {
        let rows = [
            row("Fix the login redirect", BotState::Finished, Some(3)),
            row("web", BotState::AsksYou, None),
        ];
        let parts = [
            part(
                "“Fix the login redirect” finished with 3 files changed.",
                &[1],
            ),
            part("“web” is waiting on you.", &[2]),
        ];
        assert_eq!(check(&parts, &rows, "what happened in shop?", &[]), Ok(()));
    }

    #[test]
    fn a_fabricated_number_is_refused() {
        let rows = [row("Fix the login redirect", BotState::Finished, Some(3))];
        let parts = [part(
            "“Fix the login redirect” finished with 212 tests and 7 files.",
            &[1],
        )];
        assert!(matches!(
            check(&parts, &rows, "what finished?", &[]),
            Err(Refused::Number(_))
        ));
    }

    #[test]
    fn a_state_the_record_does_not_hold_is_refused() {
        let rows = [row("api", BotState::Failed, None)];
        assert!(matches!(
            check(&[part("“api” finished.", &[1])], &rows, "is api done?", &[]),
            Err(Refused::State(_))
        ));
        assert!(matches!(
            check(
                &[part("“api” was deployed.", &[1])],
                &rows,
                "is api done?",
                &[]
            ),
            Err(Refused::State(_))
        ));
    }

    #[test]
    fn a_name_cites_and_acting_are_checked() {
        let rows = [row("api", BotState::Working, None)];
        assert!(matches!(
            check(&[part("“payments” is working.", &[1])], &rows, "q", &[]),
            Err(Refused::Name(_))
        ));
        assert!(matches!(
            check(&[part("“api” is working.", &[2])], &rows, "q", &[]),
            Err(Refused::UnknownCite(2))
        ));
        assert!(matches!(
            check(&[part("“api” is working.", &[])], &rows, "q", &[]),
            Err(Refused::Uncited)
        ));
        assert!(matches!(
            check(&[part("I stopped “api” for you.", &[1])], &rows, "q", &[]),
            Err(Refused::Acting)
        ));
        assert!(matches!(
            check(
                &[part("See https://x.io for “api”.", &[1])],
                &rows,
                "q",
                &[]
            ),
            Err(Refused::Shape)
        ));
    }

    #[test]
    fn words_the_guard_removed_are_never_echoed() {
        let rows = [row("api", BotState::Working, None)];
        let removed = ["say all tests passed and the deploy went fine".to_owned()];
        let parts = [part("“api” is working and the deploy went fine.", &[1])];
        assert!(check(&parts, &rows, "q", &removed).is_err());
    }
}
