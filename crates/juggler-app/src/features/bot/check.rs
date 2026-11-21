//! The check a written answer must pass before it is shown (AJ-E3806,
//! ADR-0034 §7.2, the harness's `claims_v5.py` as a spec): every name,
//! number and state in it is found in the records it cites. Pure; a
//! failing answer is never shown, and the records are shown instead
//! (§7.3's ladder).
//!
//! **Fail closed** (the Bot review's M2): every word of a sentence must be
//! the check's own vocabulary (grammar, the rows' field words, the state
//! words it checks), a word of a cited record's name or of the question,
//! or a number a cited record holds. Anything else, a negation, a first
//! person, a time the rows can't back, a state word off the list, a
//! format character: the records are shown alone. Words inside a record's
//! name never support a claim (M1): a state is checked against the row's
//! state, a number against its measured fields, and an outcome the
//! records never hold ("deployed", "merged") is refused whoever says it.

use super::dto::{BotRowInfo, BotRowKind, BotState, BotWrittenPart};
use super::written::is_format;

/// Why a written answer was refused (a code, for the log; never shown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Empty, too long, more than three sentences, a link, markup or a
    /// format character.
    Shape,
    /// It cites a record that wasn't issued for this answer.
    UnknownCite(u32),
    /// A sentence states something and cites nothing.
    Uncited,
    /// A number no cited record holds.
    Number(String),
    /// A word no cited record, the question or the vocabulary holds.
    Name(String),
    /// A state that disagrees with the cited records', or one they don't
    /// hold ("deployed", "crashed").
    State(String),
    /// A time the records can't back ("yesterday").
    Time(String),
    /// A quantifier ("all", "only", "every") over fewer rows than were
    /// found: it claims more than the cited rows hold.
    Quantifier,
    /// A negation ("not", "nothing", "kein"): unclear, so refused.
    Negation,
    /// A first person: Bot never did or will do anything.
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
            Self::Time(_) => "time",
            Self::Quantifier => "quantifier",
            Self::Negation => "negation",
            Self::Acting => "acting",
            Self::Echo => "echo",
        }
    }
}

const MAX_WORDS: usize = 60;
const MAX_SENTENCES: usize = 3;

/// Lower-cased words, apostrophes kept inside a word.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .map(|w| w.trim_matches(['\'', '’']))
        .filter(|w| !w.is_empty())
        .map(|w| w.replace('’', "'").to_lowercase())
        .collect()
}

/// Number words, as the numbers they say (English and German), up to a
/// hundred; a word for a bigger amount reads as a number no row holds.
fn number_word(word: &str) -> Option<u64> {
    const EN: [&str; 21] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
    ];
    const DE: [&str; 13] = [
        "null", "eins", "zwei", "drei", "vier", "fünf", "sechs", "sieben", "acht", "neun", "zehn",
        "elf", "zwölf",
    ];
    if let Some(n) = EN
        .iter()
        .position(|w| *w == word)
        .or_else(|| DE.iter().position(|w| *w == word))
    {
        return u64::try_from(n).ok();
    }
    Some(match word {
        "ein" | "eine" | "einen" | "einer" | "einem" | "single" => 1,
        "both" | "beide" | "beiden" | "pair" | "couple" => 2,
        "dozen" | "dutzend" => 12,
        "thirty" | "dreißig" => 30,
        "forty" | "vierzig" => 40,
        "fifty" | "fünfzig" => 50,
        "sixty" | "sechzig" => 60,
        "seventy" | "siebzig" => 70,
        "eighty" | "achtzig" => 80,
        "ninety" | "neunzig" => 90,
        "zwanzig" => 20,
        "hundred" | "hundert" | "thousand" | "tausend" | "million" | "millionen" | "billion" => {
            1_000_000_000
        }
        _ => return None,
    })
}

/// The numbers in a text: a run of digits with separators inside it
/// (`1,000`, `1.000`, `3.5`, `1'000`) is one number, its digits joined.
fn numbers_in(text: &str) -> Vec<u64> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut digits = String::new();
        while i < chars.len() {
            let c = chars[i];
            if c.is_ascii_digit() {
                digits.push(c);
            } else if matches!(c, ',' | '.' | '\'' | '’' | '\u{202f}' | '\u{2009}' | '_')
                && chars.get(i + 1).is_some_and(char::is_ascii_digit)
            {
                // a separator inside the number: skipped
            } else {
                break;
            }
            i += 1;
        }
        out.push(digits.parse().unwrap_or(u64::MAX));
    }
    out
}

/// State words and the states they claim.
fn state_word(word: &str) -> Option<&'static [BotState]> {
    Some(match word {
        "finished" | "finish" | "finishes" | "completed" | "complete" | "fertig"
        | "abgeschlossen" => &[BotState::Finished, BotState::Done],
        "done" | "erledigt" => &[BotState::Done, BotState::Finished],
        "ended" | "beendet" => &[BotState::Ended, BotState::Finished],
        "failed" | "error" | "fehler" | "fehlgeschlagen" => &[BotState::Failed],
        "stuck" | "hängt" | "festgefahren" | "fest" => &[BotState::Stuck],
        "asks" | "asking" | "fragt" => &[BotState::AsksYou],
        "waiting" | "waits" | "wartet" => {
            &[BotState::AsksYou, BotState::Blocked, BotState::InReview]
        }
        "blocked" | "blockiert" => &[BotState::Blocked],
        "working" | "running" | "arbeitet" | "läuft" | "progress" => &[BotState::Working],
        "review" => &[BotState::InReview],
        "limit" | "reached" | "reaching" | "erreicht" => &[BotState::Limit],
        "stopped" | "gestoppt" => &[BotState::Ended, BotState::Failed],
        "needs" | "need" | "braucht" => &[
            BotState::AsksYou,
            BotState::Stuck,
            BotState::Limit,
            BotState::Blocked,
            BotState::InReview,
        ],
        "ready" | "bereit" => &[BotState::Ready],
        _ => return None,
    })
}

/// Outcomes and states the records never hold as they are said: claiming
/// one is claiming what no row backs, whatever a name holds (M1).
const UNHELD: &[&str] = &[
    "deployed",
    "good",
    "well",
    "great",
    "nice",
    "bad",
    "perfect",
    "smooth",
    "smoothly",
    "gut",
    "alles",
    "deploy",
    "deploys",
    "merged",
    "merge",
    "shipped",
    "released",
    "approved",
    "passed",
    "passes",
    "pass",
    "pushed",
    "committed",
    "tested",
    "fixed",
    "resolved",
    "succeeded",
    "success",
    "successful",
    "successfully",
    "green",
    "red",
    "broken",
    "reverted",
    "live",
    "crashed",
    "crash",
    "crashes",
    "hung",
    "hangs",
    "paused",
    "idle",
    "aborted",
    "killed",
    "timed",
    "timeout",
    "cancelled",
    "canceled",
    "failing",
    "fails",
    "fine",
    "ok",
    "okay",
    "healthy",
    "safe",
    "secure",
    "bestanden",
    "behoben",
    "veröffentlicht",
    "gemergt",
    "deployt",
    "abgestürzt",
    "gescheitert",
    "abgebrochen",
    "pausiert",
    "grün",
    "rot",
];

/// Words that say a time the rows don't carry as words.
const TIME: &[&str] = &[
    "yesterday",
    "today",
    "tonight",
    "tomorrow",
    "morning",
    "afternoon",
    "evening",
    "night",
    "ago",
    "earlier",
    "later",
    "recently",
    "soon",
    "hour",
    "hours",
    "minute",
    "minutes",
    "second",
    "seconds",
    "day",
    "days",
    "week",
    "weeks",
    "month",
    "months",
    "year",
    "years",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "gestern",
    "heute",
    "morgen",
    "vorhin",
    "gleich",
    "stunde",
    "stunden",
    "minuten",
    "tag",
    "tage",
    "tagen",
    "woche",
    "wochen",
    "monat",
    "jahr",
    "am",
    "pm",
    "uhr",
];

/// Words that speak for every row (or say there is no other): allowed only
/// when the sentence cites every row found (R1).
const QUANTIFIERS: &[&str] = &[
    "all",
    "every",
    "each",
    "only",
    "everything",
    "everyone",
    "entire",
    "whole",
    "total",
    "alle",
    "allen",
    "jede",
    "jeder",
    "jedes",
    "nur",
    "sämtliche",
    "einzige",
    "einzig",
];

const NEGATION: &[&str] = &[
    "not",
    "no",
    "never",
    "nothing",
    "none",
    "nobody",
    "neither",
    "nor",
    "without",
    "isn't",
    "aren't",
    "wasn't",
    "weren't",
    "hasn't",
    "haven't",
    "hadn't",
    "doesn't",
    "don't",
    "didn't",
    "can't",
    "cannot",
    "won't",
    "wouldn't",
    "shouldn't",
    "nicht",
    "kein",
    "keine",
    "keiner",
    "keinen",
    "keinem",
    "keines",
    "nie",
    "niemals",
    "nichts",
    "niemand",
    "ohne",
];

const FIRST_PERSON: &[&str] = &[
    "i", "i'm", "i've", "i'll", "i'd", "me", "my", "mine", "myself", "we", "we're", "we've",
    "we'll", "our", "ours", "us", "ich", "mich", "mir", "mein", "meine", "wir", "uns", "unser",
    "unsere",
];

/// The check's own words: grammar, and the words the window writes about
/// rows (kinds, fields, the state words it checks). English and German.
const VOCAB: &[&str] = &[
    "a",
    "an",
    "the",
    "and",
    "or",
    "but",
    "with",
    "in",
    "on",
    "at",
    "of",
    "for",
    "to",
    "from",
    "by",
    "as",
    "is",
    "are",
    "was",
    "were",
    "has",
    "have",
    "had",
    "been",
    "be",
    "it",
    "its",
    "it's",
    "they",
    "their",
    "them",
    "this",
    "that",
    "these",
    "those",
    "there",
    "here",
    "which",
    "who",
    "also",
    "still",
    "now",
    "currently",
    "right",
    "all",
    "each",
    "every",
    "other",
    "another",
    "same",
    "only",
    "just",
    "so",
    "far",
    "while",
    "both",
    "one",
    "you",
    "your",
    "for",
    "into",
    "under",
    "held",
    "holds",
    "hold",
    "by",
    "session",
    "sessions",
    "task",
    "tasks",
    "pool",
    "pools",
    "project",
    "projects",
    "account",
    "accounts",
    "app",
    "apps",
    "file",
    "files",
    "changed",
    "change",
    "changes",
    "branch",
    "used",
    "use",
    "percent",
    "window",
    "reached",
    "reaching",
    "looks",
    "look",
    "needs",
    "need",
    "on",
    "stopped",
    "an",
    "error",
    "records",
    "record",
    "found",
    "show",
    "shows",
    "der",
    "die",
    "das",
    "den",
    "dem",
    "des",
    "ein",
    "eine",
    "einen",
    "einem",
    "einer",
    "und",
    "oder",
    "aber",
    "mit",
    "im",
    "auf",
    "von",
    "für",
    "zu",
    "zum",
    "zur",
    "ist",
    "sind",
    "war",
    "waren",
    "hat",
    "haben",
    "es",
    "sie",
    "dich",
    "noch",
    "gerade",
    "auch",
    "nur",
    "alle",
    "jede",
    "jeder",
    "sitzung",
    "sitzungen",
    "aufgabe",
    "aufgaben",
    "projekt",
    "projekte",
    "konto",
    "konten",
    "datei",
    "dateien",
    "geändert",
    "zweig",
    "genutzt",
    "erreicht",
    "prozent",
];

/// A record's names, each as written: its own, its project's, its App's
/// and pool's, its holder's and branch (none empty).
pub(super) fn names_of(row: &BotRowInfo) -> impl Iterator<Item = &str> {
    [
        Some(&row.title),
        row.project.as_ref(),
        row.app_label.as_ref(),
        row.pool.as_ref(),
        row.holder.as_ref(),
        row.branch.as_ref(),
    ]
    .into_iter()
    .flatten()
    .map(String::as_str)
    .filter(|n| !n.is_empty())
}

/// The numbers a row holds as measured facts (never from a name: L2).
fn held_numbers(row: &BotRowInfo) -> Vec<u64> {
    [row.files_changed, row.used_percent, row.blocked_by]
        .into_iter()
        .flatten()
        .map(u64::from)
        .collect()
}

/// The quoted spans in a sentence, and the sentence with them taken out.
fn split_quotes(text: &str) -> (Vec<String>, String) {
    let mut quoted = Vec::new();
    let mut rest = String::new();
    let mut open: Option<String> = None;
    for c in text.chars() {
        match (&mut open, c) {
            (None, '“' | '"' | '„' | '«') => open = Some(String::new()),
            (Some(inner), '”' | '"' | '“' | '»') => {
                quoted.push(std::mem::take(inner));
                open = None;
                rest.push(' ');
            }
            (Some(inner), c) => inner.push(c),
            (None, c) => rest.push(c),
        }
    }
    if let Some(inner) = open {
        rest.push_str(&inner);
    }
    (quoted, rest)
}

/// Whether a sentence states a fact: a name, a number or a state.
fn states_anything(text: &str) -> bool {
    let (quoted, rest) = split_quotes(text);
    !quoted.is_empty()
        || !numbers_in(text).is_empty()
        || words(&rest).iter().any(|w| {
            !VOCAB.contains(&w.as_str()) || state_word(w).is_some() || number_word(w).is_some()
        })
}

fn shape(parts: &[BotWrittenPart]) -> Result<String, Refused> {
    let all: String = parts
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let word_count = all.split_whitespace().count();
    let bad_char = |c: char| {
        c.is_control()
            || is_format(c)
            || matches!(
                c,
                '*' | '#' | '`' | '[' | ']' | '<' | '>' | '|' | '_' | '\u{2028}' | '\u{2029}'
            )
    };
    if parts.is_empty()
        || parts.len() > MAX_SENTENCES
        || word_count == 0
        || word_count > MAX_WORDS
        || all.contains("http")
        || all.contains("://")
        || all.contains("www.")
        || all.chars().any(bad_char)
    {
        return Err(Refused::Shape);
    }
    Ok(all)
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
    let all = shape(parts)?;
    let all_words = words(&all).join(" ");
    for gone in removed {
        let gone_words = words(gone);
        if gone_words.len() >= 4
            && gone_words
                .windows(4)
                .any(|w| all_words.contains(&w.join(" ")))
        {
            return Err(Refused::Echo);
        }
    }
    let question_words = words(question);
    let question_numbers = numbers_in(question);
    for part in parts {
        check_part(part, rows, &question_words, &question_numbers)?;
    }
    Ok(())
}

/// One sentence against the rows it cites.
fn check_part(
    part: &BotWrittenPart,
    rows: &[BotRowInfo],
    question_words: &[String],
    question_numbers: &[u64],
) -> Result<(), Refused> {
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
    let (quoted, rest) = split_quotes(&part.text);
    let sentence = words(&rest);
    if sentence.iter().any(|w| FIRST_PERSON.contains(&w.as_str())) {
        return Err(Refused::Acting);
    }
    if sentence
        .iter()
        .chain(
            quoted
                .iter()
                .flat_map(|q| words(q))
                .collect::<Vec<_>>()
                .iter(),
        )
        .any(|w| NEGATION.contains(&w.as_str()))
    {
        return Err(Refused::Negation);
    }
    if cited.len() < rows.len() && sentence.iter().any(|w| QUANTIFIERS.contains(&w.as_str())) {
        return Err(Refused::Quantifier);
    }
    // A quoted name is one of the cited records' names, whole.
    let names: Vec<&str> = cited.iter().flat_map(|r| names_of(r)).collect();
    for name in &quoted {
        let name = name.trim();
        if !names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name) || n.to_lowercase() == name.to_lowercase())
        {
            return Err(Refused::Name(name.to_lowercase()));
        }
    }
    let name_words: Vec<String> = names.iter().flat_map(|n| words(n)).collect();
    // Numbers: digits and number words, each a measured fact of a cited
    // row, a number the question says, or a count of the cited rows.
    // A count is of the cited rows, never of every row found (R1).
    let mut counts = vec![cited.len() as u64];
    for kind in [BotRowKind::Session, BotRowKind::Task, BotRowKind::Account] {
        counts.push(cited.iter().filter(|r| r.kind == kind).count() as u64);
    }
    let held: Vec<u64> = cited.iter().flat_map(|r| held_numbers(r)).collect();
    for n in numbers_in(&rest)
        .into_iter()
        .chain(sentence.iter().filter_map(|w| number_word(w)))
    {
        if !(held.contains(&n) || question_numbers.contains(&n) || counts.contains(&n)) {
            return Err(Refused::Number(n.to_string()));
        }
    }
    for word in &sentence {
        if word.chars().all(|c| c.is_ascii_digit()) || number_word(word).is_some() {
            continue;
        }
        if UNHELD.contains(&word.as_str()) {
            return Err(Refused::State(word.clone()));
        }
        if TIME.contains(&word.as_str()) {
            if question_words.contains(word) {
                continue;
            }
            return Err(Refused::Time(word.clone()));
        }
        if let Some(claim) = state_word(word) {
            // Each cited row's own state, never one of several (R1).
            if cited.is_empty()
                || !cited
                    .iter()
                    .all(|r| r.state.is_some_and(|s| claim.contains(&s)))
            {
                return Err(Refused::State(word.clone()));
            }
            continue;
        }
        if VOCAB.contains(&word.as_str())
            || name_words.contains(word)
            || question_words.contains(word)
        {
            continue;
        }
        return Err(Refused::Name(word.clone()));
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

    fn one(text: &str, rows: &[BotRowInfo]) -> Result<(), Refused> {
        check(&[part(text, &[1])], rows, "what is going on in shop?", &[])
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
        assert_eq!(
            check(
                &[part("In shop, the session “web” asks you.", &[2])],
                &rows,
                "q",
                &[]
            ),
            Ok(())
        );
    }

    #[test]
    fn a_fabricated_number_is_refused_in_any_form() {
        let rows = [row("Fix the login redirect", BotState::Finished, Some(3))];
        assert!(matches!(
            one(
                "“Fix the login redirect” finished with 212 files changed.",
                &rows
            ),
            Err(Refused::Number(_))
        ));
        // MUT-C7: a number word.
        assert_eq!(
            one("Twelve sessions finished.", &rows),
            Err(Refused::Number("12".into()))
        );
        assert!(matches!(
            one("Four sessions finished.", &rows),
            Err(Refused::Number(_))
        ));
        // Digit groups are one number, never 1 and 0.
        assert_eq!(
            one("1,000 files changed in “Fix the login redirect”.", &rows),
            Err(Refused::Number("1000".into()))
        );
        assert!(matches!(
            one("“Fix the login redirect” changed 3.5 files.", &rows),
            Err(Refused::Number(_))
        ));
    }

    #[test]
    fn a_number_from_a_name_is_not_held() {
        let rows = [row("api 212", BotState::Failed, None)];
        assert!(matches!(
            one("“api 212” failed with 212 files changed.", &rows),
            Err(Refused::Number(_))
        ));
    }

    #[test]
    fn states_negation_and_time_are_refused_unless_the_rows_back_them() {
        let rows = [row("api", BotState::Failed, None)];
        assert!(matches!(
            one("“api” finished.", &rows),
            Err(Refused::State(_))
        ));
        assert!(matches!(
            one("“api” was deployed.", &rows),
            Err(Refused::State(_))
        ));
        assert!(matches!(
            one("“api” crashed.", &rows),
            Err(Refused::State(_))
        ));
        assert!(matches!(
            one("“api” ist abgestürzt.", &rows),
            Err(Refused::State(_))
        ));
        assert_eq!(one("“api” has not failed.", &rows), Err(Refused::Negation));
        assert_eq!(
            one("Nothing is stuck.", &[row("api", BotState::Stuck, None)]),
            Err(Refused::Negation)
        );
        assert!(matches!(
            one("“api” failed yesterday.", &rows),
            Err(Refused::Time(_))
        ));
        assert_eq!(
            check(
                &[part("“api” failed yesterday.", &[1])],
                &rows,
                "what failed yesterday?",
                &[]
            ),
            Ok(())
        );
    }

    #[test]
    fn every_word_is_a_name_the_rows_hold_or_the_check_s_own() {
        let rows = [row("api", BotState::Working, None)];
        // MUT-C5: a name as a capitalised first word, or in lower case.
        assert_eq!(
            one("Payments is working.", &rows),
            Err(Refused::Name("payments".into()))
        );
        assert_eq!(
            one("The payments session is working.", &rows),
            Err(Refused::Name("payments".into()))
        );
        assert!(matches!(
            one("“payments” is working.", &rows),
            Err(Refused::Name(_))
        ));
        // A quoted name is a whole name, never part of one.
        let rapid = [row("rapid deploy", BotState::Working, None)];
        assert!(matches!(
            one("“api” is working.", &rapid),
            Err(Refused::Name(_))
        ));
    }

    #[test]
    fn names_never_unlock_a_claim() {
        let mut r = row("web", BotState::Working, None);
        r.branch = Some("feat/deployed-green".into());
        assert!(matches!(
            one("“web” was deployed and is green.", &[r]),
            Err(Refused::State(_))
        ));
    }

    #[test]
    fn cites_acting_and_shape_are_checked() {
        let rows = [row("api", BotState::Working, None)];
        assert!(matches!(
            check(&[part("“api” is working.", &[2])], &rows, "q", &[]),
            Err(Refused::UnknownCite(2))
        ));
        assert!(matches!(
            check(&[part("“api” is working.", &[])], &rows, "q", &[]),
            Err(Refused::Uncited)
        ));
        assert_eq!(one("I stopped “api” for you.", &rows), Err(Refused::Acting));
        assert_eq!(one("I have paused “api”.", &rows), Err(Refused::Acting));
        assert_eq!(
            one("See https://x.io for “api”.", &rows),
            Err(Refused::Shape)
        );
        // MUT-C9: markup.
        assert_eq!(one("“api” is *working*.", &rows), Err(Refused::Shape));
        // A direction override hides text.
        assert_eq!(one("“api” is working\u{202e}.", &rows), Err(Refused::Shape));
        // MUT-C12: more than 60 words.
        let long = format!("“api” is working{}.", " and working".repeat(30));
        assert_eq!(one(&long, &rows), Err(Refused::Shape));
        let fits = format!("“api” is working{}.", " and working".repeat(20));
        assert_eq!(one(&fits, &rows), Ok(()));
    }

    /// The re-review's R1: claim words in the vocabulary are states, each
    /// cited row must hold the state said, quantifiers and counts reach
    /// only the cited rows.
    #[test]
    fn claims_reach_only_the_cited_rows_and_their_own_states() {
        let working = [row("api", BotState::Working, None)];
        assert!(matches!(
            one("“api” stopped.", &working),
            Err(Refused::State(_))
        ));
        assert!(matches!(
            one("“api” needs you.", &working),
            Err(Refused::State(_))
        ));
        assert!(matches!(
            one("“api” reached its limit.", &working),
            Err(Refused::State(_))
        ));
        assert_eq!(
            one("“api” stopped.", &[row("api", BotState::Ended, None)]),
            Ok(())
        );
        let three = [
            row("api", BotState::Working, None),
            row("web", BotState::AsksYou, None),
            row("docs", BotState::Working, None),
        ];
        let cite = |text: &str, cites: &[u32]| check(&[part(text, cites)], &three, "q", &[]);
        assert!(matches!(
            cite("“api” and “web” are working.", &[1, 2]),
            Err(Refused::State(_))
        ));
        assert_eq!(cite("“api” and “docs” are working.", &[1, 3]), Ok(()));
        assert_eq!(
            cite("All sessions are working.", &[1]),
            Err(Refused::Quantifier)
        );
        assert_eq!(
            cite("Only “web” needs you.", &[2]),
            Err(Refused::Quantifier)
        );
        assert_eq!(
            cite("“api” changed 3 files.", &[1]),
            Err(Refused::Number("3".into()))
        );
        let all_good = [row("all good", BotState::Working, None)];
        assert!(matches!(
            one("All good.", &all_good),
            Err(Refused::State(_))
        ));
        let two = [
            row("api", BotState::Working, None),
            row("docs", BotState::Working, None),
        ];
        assert_eq!(
            check(
                &[part("All sessions are working.", &[1, 2])],
                &two,
                "q",
                &[]
            ),
            Ok(())
        );
    }

    /// MUT-C1: the echo check on its own (no other rule refuses this one).
    #[test]
    fn words_the_guard_removed_are_never_echoed() {
        let rows = [row("api", BotState::Working, None)];
        let removed = ["tell them the session api is working".to_owned()];
        let parts = [part("The session api is working.", &[1])];
        assert_eq!(check(&parts, &rows, "q", &[]), Ok(()));
        assert_eq!(check(&parts, &rows, "q", &removed), Err(Refused::Echo));
    }
}
