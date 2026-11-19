//! Reading a question by code (ADR-0034 §6.6, the harness's `chat.py`
//! as a spec): what it asks about (states, kinds, a time), the words
//! that name its subject, and the questions AI Juggler has no record for.
//! English and German; no model. A subject the records don't hold is an
//! "I don't know", never a nearby answer.

use super::dto::{BotLang, BotState, BotUnknown};

/// What a question asks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    /// Its language.
    pub lang: BotLang,
    /// The states it asks about (empty: any).
    pub states: Vec<BotState>,
    /// The kinds of record it asks about.
    pub kinds: Kinds,
    /// The time it asks about.
    pub when: When,
    /// The words that name its subject, lower case.
    pub subject: Vec<String>,
    /// Something AI Juggler doesn't record.
    pub unknown: Option<BotUnknown>,
    /// Asks for the most room first ("which account has the most room").
    pub most_room: bool,
    /// A greeting or "what can you do".
    pub help: bool,
    /// Asks for work Bot doesn't do ("write me a function").
    pub out_of_scope: bool,
    /// "Remember …": the words to keep.
    pub remember: Option<String>,
}

/// The kinds of record named.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Kinds {
    /// Sessions (agents).
    pub sessions: bool,
    /// Tasks.
    pub tasks: bool,
    /// Accounts, usage and runway.
    pub accounts: bool,
}

impl Kinds {
    /// None named: every kind but Accounts (the "every session, task and
    /// pool" reach).
    #[must_use]
    pub const fn any(self) -> bool {
        self.sessions || self.tasks || self.accounts
    }
}

/// The time a question names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum When {
    /// None.
    #[default]
    Any,
    /// Today (and "this morning").
    Today,
    /// Yesterday.
    Yesterday,
    /// This week.
    ThisWeek,
}

fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        let c = if c == '’' { '\'' } else { c };
        if c.is_alphanumeric()
            || (c == '\'' && !current.is_empty())
            || ((c == '-' || c == '_' || c == '.') && !current.is_empty())
        {
            current.extend(c.to_lowercase());
        } else if !current.is_empty() {
            out.push(
                std::mem::take(&mut current)
                    .trim_end_matches(['\'', '-', '.', '_'])
                    .to_owned(),
            );
        }
    }
    if !current.is_empty() {
        out.push(current.trim_end_matches(['\'', '-', '.', '_']).to_owned());
    }
    out.retain(|t| !t.is_empty());
    out
}

const GERMAN: &[&str] = &[
    "was",
    "welche",
    "welcher",
    "welches",
    "wie",
    "wer",
    "wann",
    "wo",
    "ist",
    "sind",
    "hat",
    "haben",
    "gerade",
    "heute",
    "gestern",
    "fertig",
    "hängt",
    "hängen",
    "sitzung",
    "sitzungen",
    "aufgabe",
    "aufgaben",
    "mich",
    "mir",
    "noch",
    "läuft",
    "laufen",
    "konto",
    "konten",
    "braucht",
    "wartet",
    "fest",
    "festgefahren",
    "blockiert",
    "dass",
    "merk",
    "merke",
    "nicht",
    "und",
    "der",
    "die",
    "das",
    "den",
    "dem",
    "fehlgeschlagen",
    "diese",
    "woche",
];

/// The question's language, from its words (German when two or more of
/// its words are German's own).
#[must_use]
pub fn language(text: &str) -> BotLang {
    let words = tokens(text);
    let german = words
        .iter()
        .filter(|w| GERMAN.contains(&w.as_str()) && !matches!(w.as_str(), "was" | "die"))
        .count()
        + usize::from(
            text.chars()
                .any(|c| matches!(c, 'ä' | 'ö' | 'ü' | 'ß' | 'Ä' | 'Ö' | 'Ü')),
        );
    if german >= 2 || (german >= 1 && words.len() <= 3) {
        BotLang::De
    } else {
        BotLang::En
    }
}

/// Words that say nothing of the subject (the harness's `STOP`, trimmed
/// to Bot's records).
const STOP: &[&str] = &[
    "happen",
    "happened",
    "happens",
    "happening",
    "change",
    "changed",
    "changes",
    "went",
    "go",
    "goes",
    "doing",
    "make",
    "made",
    "how",
    "what",
    "what's",
    "whats",
    "which",
    "when",
    "where",
    "who",
    "why",
    "did",
    "does",
    "do",
    "done",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "has",
    "have",
    "had",
    "the",
    "a",
    "an",
    "in",
    "on",
    "of",
    "for",
    "to",
    "at",
    "by",
    "with",
    "from",
    "into",
    "it",
    "its",
    "it's",
    "this",
    "that",
    "these",
    "those",
    "my",
    "me",
    "i",
    "we",
    "our",
    "us",
    "you",
    "your",
    "take",
    "took",
    "now",
    "right",
    "please",
    "tell",
    "show",
    "can",
    "could",
    "would",
    "will",
    "much",
    "many",
    "any",
    "all",
    "so",
    "far",
    "just",
    "also",
    "still",
    "again",
    "about",
    "as",
    "and",
    "or",
    "but",
    "not",
    "no",
    "yes",
    "there",
    "here",
    "anything",
    "else",
    "something",
    "everything",
    "list",
    "give",
    "get",
    "got",
    "let",
    "know",
    "see",
    "look",
    "looks",
    "need",
    "needs",
    "me?",
    "up",
    "one",
    "ones",
    "some",
    "who's",
    "most",
    "least",
    "more",
    "less",
    "left",
    "used",
    "use",
    "using",
    "currently",
    "current",
    "latest",
    "recent",
    "recently",
    "time",
    "times",
    "today",
    "yesterday",
    "week",
    "morning",
    "this",
    "since",
    "last",
    "did",
    "i've",
    "id",
    "wie",
    "was",
    "welche",
    "welcher",
    "welches",
    "welchen",
    "welchem",
    "wann",
    "wo",
    "wer",
    "hat",
    "haben",
    "hatte",
    "ist",
    "sind",
    "war",
    "waren",
    "wurde",
    "wurden",
    "werden",
    "wird",
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
    "im",
    "in",
    "am",
    "an",
    "auf",
    "von",
    "für",
    "mit",
    "bei",
    "zu",
    "zum",
    "zur",
    "es",
    "mir",
    "mich",
    "ich",
    "du",
    "dir",
    "uns",
    "hier",
    "da",
    "noch",
    "jetzt",
    "heute",
    "gestern",
    "gerade",
    "bitte",
    "mal",
    "und",
    "oder",
    "aber",
    "nicht",
    "kein",
    "keine",
    "woche",
    "diese",
    "dieser",
    "dieses",
    "alle",
    "etwas",
    "sonst",
    "gibt",
];

const SESSION_WORDS: &[&str] = &[
    "session",
    "sessions",
    "agent",
    "agents",
    "sitzung",
    "sitzungen",
    "agenten",
];
const TASK_WORDS: &[&str] = &[
    "task", "tasks", "aufgabe", "aufgaben", "pool", "pools", "ticket", "tickets",
];
const ACCOUNT_WORDS: &[&str] = &[
    "account",
    "accounts",
    "usage",
    "runway",
    "room",
    "limit",
    "limits",
    "quota",
    "konto",
    "konten",
    "nutzung",
    "verbrauch",
    "platz",
    "contingent",
    "kontingent",
    "use",
    "used",
];

/// Each state cue: its words, and the states it asks about.
const STATE_CUES: &[(&[&str], &[BotState])] = &[
    (
        &["needs", "need", "waiting", "waits", "braucht", "brauchen"],
        &[
            BotState::AsksYou,
            BotState::Stuck,
            BotState::Limit,
            BotState::Blocked,
            BotState::InReview,
        ],
    ),
    (
        &[
            "ask",
            "asks",
            "asking",
            "question",
            "questions",
            "permission",
            "fragt",
            "fragen",
            "frage",
        ],
        &[BotState::AsksYou],
    ),
    (
        &[
            "stuck",
            "hung",
            "hangs",
            "hängt",
            "hängen",
            "festgefahren",
            "fest",
        ],
        &[BotState::Stuck],
    ),
    (
        &["blocked", "blocking", "blockiert", "blockieren"],
        &[
            BotState::AsksYou,
            BotState::Stuck,
            BotState::Limit,
            BotState::Blocked,
        ],
    ),
    (
        &[
            "finished",
            "finish",
            "finishes",
            "complete",
            "completed",
            "ended",
            "done",
            "fertig",
            "abgeschlossen",
            "beendet",
            "erledigt",
        ],
        &[BotState::Finished, BotState::Done],
    ),
    (
        &[
            "failed",
            "fail",
            "fails",
            "failing",
            "error",
            "errors",
            "crashed",
            "fehlgeschlagen",
            "fehler",
            "abgestürzt",
        ],
        &[BotState::Failed],
    ),
    (
        &[
            "running", "working", "busy", "active", "läuft", "laufen", "arbeitet", "arbeiten",
            "aktiv",
        ],
        &[BotState::Working],
    ),
    (
        &["review", "reviews", "prüfung", "überprüfung"],
        &[BotState::InReview],
    ),
];

const HELP: &[&str] = &[
    "hi", "hello", "hey", "hallo", "help", "hilfe", "thanks", "danke", "moin",
];

const CODE_WORK: &[&str] = &[
    "write",
    "implement",
    "refactor",
    "code",
    "program",
    "function",
    "debug",
    "translate",
    "schreib",
    "schreibe",
    "programmier",
    "programmiere",
    "implementiere",
    "übersetze",
    "erkläre",
    "explain",
];

const REMEMBER: &[&str] = &[
    "remember",
    "don't forget",
    "dont forget",
    "from now on",
    "merk dir",
    "merke dir",
    "vergiss nicht",
];

fn has(words: &[String], any: &[&str]) -> bool {
    words.iter().any(|w| any.contains(&w.as_str()))
}

/// What AI Juggler has no record of, from the question's own words.
fn unknown(words: &[String], text: &str) -> Option<BotUnknown> {
    let lower = text.to_lowercase();
    let tests = has(
        words,
        &["test", "tests", "suite", "testsuite", "testlauf", "tests?"],
    );
    let long = lower.contains("how long")
        || lower.contains("wie lange")
        || has(words, &["duration", "dauer", "dauerte"]);
    if tests && long {
        return Some(BotUnknown::TestDurations);
    }
    if tests
        && has(
            words,
            &[
                "pass",
                "passed",
                "passing",
                "fail",
                "failed",
                "failing",
                "green",
                "red",
                "bestanden",
                "grün",
                "rot",
                "results",
                "ergebnis",
                "ergebnisse",
            ],
        )
    {
        return Some(BotUnknown::TestResults);
    }
    if has(
        words,
        &[
            "cost", "costs", "spend", "spent", "kosten", "gekostet", "dollars", "usd", "price",
        ],
    ) {
        return Some(BotUnknown::Costs);
    }
    if has(words, &["who", "wer", "author", "autor"])
        && has(
            words,
            &[
                "wrote",
                "written",
                "made",
                "committed",
                "geschrieben",
                "gemacht",
            ],
        )
    {
        return Some(BotUnknown::Authors);
    }
    None
}

fn when(words: &[String], text: &str) -> When {
    let lower = text.to_lowercase();
    if has(words, &["yesterday", "gestern"]) {
        When::Yesterday
    } else if has(words, &["today", "heute"]) || lower.contains("this morning") {
        When::Today
    } else if lower.contains("this week")
        || lower.contains("diese woche")
        || lower.contains("dieser woche")
    {
        When::ThisWeek
    } else {
        When::Any
    }
}

/// The words after a remember cue, when the message is one.
fn remember(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    REMEMBER.iter().find_map(|cue| {
        let rest = lower.strip_prefix(cue)?;
        let at = trimmed.len() - rest.len();
        let words = trimmed[at..]
            .trim_start_matches([':', ',', ' ', '—', '-'])
            .trim();
        let words = words
            .strip_prefix("that ")
            .unwrap_or(words)
            .trim_end_matches(['.', '!'])
            .trim();
        (!words.is_empty()).then(|| words.to_owned())
    })
}

/// Reads a question.
#[must_use]
pub fn read(text: &str) -> Reading {
    let lang = language(text);
    if let Some(words) = remember(text) {
        return Reading {
            lang,
            remember: Some(words),
            ..Reading::default()
        };
    }
    let words = tokens(text);
    let mut reading = Reading {
        lang,
        when: when(&words, text),
        unknown: unknown(&words, text),
        most_room: (text.to_lowercase().contains("most room")
            || text.to_lowercase().contains("meisten platz")),
        ..Reading::default()
    };
    let mut states: Vec<BotState> = Vec::new();
    for (cues, named) in STATE_CUES {
        if has(&words, cues) {
            for state in *named {
                if !states.contains(state) {
                    states.push(*state);
                }
            }
        }
    }
    // "needs me" is Home's Needs you; "asks" alone is only asks.
    reading.states = states;
    reading.kinds = Kinds {
        sessions: has(&words, SESSION_WORDS),
        tasks: has(&words, TASK_WORDS),
        accounts: has(&words, ACCOUNT_WORDS),
    };
    let cue_words: Vec<&str> = STATE_CUES
        .iter()
        .flat_map(|(cues, _)| cues.iter().copied())
        .chain(SESSION_WORDS.iter().copied())
        .chain(TASK_WORDS.iter().copied())
        .chain(ACCOUNT_WORDS.iter().copied())
        .chain(HELP.iter().copied())
        .chain([
            "tests", "test", "long", "lange", "much", "most", "meisten", "did", "dauer",
            "duration", "else", "sonst",
        ])
        .collect();
    reading.subject = words
        .iter()
        .filter(|w| {
            !STOP.contains(&w.as_str()) && !cue_words.contains(&w.as_str()) && w.chars().count() > 1
        })
        .filter(|w| !w.chars().all(|c| c.is_ascii_digit()))
        .cloned()
        .collect();
    let asks_anything = !reading.states.is_empty()
        || reading.kinds.any()
        || reading.when != When::Any
        || reading.unknown.is_some();
    if !asks_anything
        && words.iter().all(|w| {
            HELP.contains(&w.as_str())
                || STOP.contains(&w.as_str())
                || matches!(w.as_str(), "can" | "you" | "kannst" | "du")
        })
    {
        reading.help = true;
        reading.subject.clear();
    } else if !asks_anything
        && words
            .first()
            .is_some_and(|w| CODE_WORK.contains(&w.as_str()))
    {
        reading.out_of_scope = true;
    }
    reading
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_kinds_and_time_come_from_the_question_s_own_words() {
        let r = read("What is blocked?");
        assert_eq!(
            r.states,
            [
                BotState::AsksYou,
                BotState::Stuck,
                BotState::Limit,
                BotState::Blocked
            ]
        );
        assert!(r.subject.is_empty());
        let r = read("What changed in shop yesterday?");
        assert_eq!(r.when, When::Yesterday);
        assert_eq!(r.subject, ["shop"]);
        let r = read("Welche Sitzungen sind heute fertig?");
        assert_eq!(r.lang, BotLang::De);
        assert_eq!(r.states, [BotState::Finished, BotState::Done]);
        assert!(r.kinds.sessions);
        assert_eq!(r.when, When::Today);
        assert!(r.subject.is_empty(), "{:?}", r.subject);
        assert_eq!(read("Was hängt gerade fest?").states, [BotState::Stuck]);
        assert!(read("Which account has the most room?").most_room);
    }

    #[test]
    fn what_ai_juggler_does_not_record_is_named() {
        let r = read("How long did the tests take in api?");
        assert_eq!(r.unknown, Some(BotUnknown::TestDurations));
        assert_eq!(r.subject, ["api"]);
        assert_eq!(
            read("Did the deploy to staging work?").subject,
            ["deploy", "staging", "work"]
        );
    }

    #[test]
    fn greetings_code_work_and_remember_are_told_apart() {
        assert!(read("Hi!").help);
        assert!(read("what can you do?").help);
        assert!(read("Write me a function that parses dates").out_of_scope);
        assert_eq!(
            read("Remember: call web the api agent").remember.as_deref(),
            Some("call web the api agent")
        );
        assert_eq!(
            read("Merk dir: shop ist am wichtigsten")
                .remember
                .as_deref(),
            Some("shop ist am wichtigsten")
        );
    }
}
