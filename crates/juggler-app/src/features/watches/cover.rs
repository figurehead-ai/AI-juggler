//! AJ-E3830 (D-149 amendment 3, harness v11): **full coverage, failing
//! closed by construction.** Code drafts a watch only when it accounts for
//! every word of the request:
//!
//! - **Code reads first** ([`read`]): each word is a target's name (or a
//!   kind word, or "any" before one), a condition's cue word, a
//!   condition's number with its unit, a lifetime or quiet-hours phrase,
//!   or a word of a closed filler list. A word it can't place, or a number
//!   no condition said takes, and code doesn't claim the request.
//! - **Then the model labels** ([`told_back`]): with a guided schema it
//!   splits the request into spans and gives each a slot (target,
//!   exclusion, condition, condition_param, lifetime, quiet_hours, filler,
//!   other). Code checks the spans repeat the request exactly and reads
//!   them only for what to say back and the nearest target; **the model
//!   never makes a request supported.** A request code can't account for
//!   is told back in its own words, always with the nearest option.
//!
//! The confirm card in plain words stays the last human check: only the
//! user's click adds a watch.

use serde_json::{Value, json};

use crate::features::ai::engine::{Decoding, Finish, Request, Stop};
use crate::features::ai::run::JobModel;
use crate::features::watches::commands::WatchDraftInfo;
use crate::features::watches::draft::{
    self, Ask, LIFE_ALWAYS, LIFE_ENDS, LIFE_ONCE, LIFE_TODAY, WatchDrafted, cue_conditions,
    draft_info, masked, number_words, request_times, words,
};
use crate::features::watches::model::{Condition, LifetimeChoice, Param, TargetKind, WatchTarget};
use crate::features::watches::resolve;

/// The labelling call's instructions (harness v11's text).
pub const LABEL_INSTRUCTIONS: &str = include_str!("draft_prompts/label.txt");
const TASK: &str = "Split the REQUEST into spans that repeat it word for word, and label each one.";

/// What a span of the request is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// What the request is about.
    Target,
    /// What it leaves out.
    Exclusion,
    /// The event.
    Condition,
    /// A number with its unit, for the condition.
    ConditionParam,
    /// How long.
    Lifetime,
    /// Quiet hours.
    QuietHours,
    /// Asking and small words.
    Filler,
    /// Anything else.
    Other,
}

const SLOTS: [(&str, Slot); 8] = [
    ("target", Slot::Target),
    ("exclusion", Slot::Exclusion),
    ("condition", Slot::Condition),
    ("condition_param", Slot::ConditionParam),
    ("lifetime", Slot::Lifetime),
    ("quiet_hours", Slot::QuietHours),
    ("filler", Slot::Filler),
    ("other", Slot::Other),
];

/// A number's unit, as a condition takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// Minutes (stuck, asks you, idle).
    Minutes,
    /// A share (runs below).
    Percent,
}

/// Code's reading of the request, word by word.
#[derive(Debug, Clone)]
pub struct Reading {
    /// The words (lowercased; `%` read as "percent").
    pub words: Vec<String>,
    /// Each word's slot, `None` where code can't place it.
    pub slots: Vec<Option<Slot>>,
    /// The numbers with their units: first word, last word, unit, value.
    pub params: Vec<(usize, usize, Unit, u32)>,
    /// A quiet-hours phrase asks to be told even then.
    pub quiet_on: bool,
}

/// What code makes of the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cover {
    /// Code accounts for every word: the draft is code's.
    Claimed,
    /// Every word placed, but no condition said: the form.
    NoCondition,
    /// Every word placed, but a number no condition said takes: its words.
    Untaken(String),
    /// Words code can't place (their indexes).
    Leftover(Vec<usize>),
}

// ============================================================================ the word lists

/// The closed filler list: asking words and small words, English and
/// German. Never a negation, a quantifier, a number or a unit.
const FILLER: &[&str] = &[
    // English
    "a",
    "an",
    "the",
    "me",
    "my",
    "mine",
    "i",
    "you",
    "your",
    "it",
    "its",
    "they",
    "them",
    "their",
    "this",
    "that",
    "these",
    "those",
    "there",
    "here",
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
    "does",
    "do",
    "did",
    "will",
    "would",
    "could",
    "can",
    "should",
    "please",
    "pls",
    "kindly",
    "hey",
    "hi",
    "ok",
    "okay",
    "so",
    "and",
    "or",
    "then",
    "also",
    "too",
    "when",
    "if",
    "once",
    "as",
    "soon",
    "s",
    "d",
    "m",
    "ll",
    "re",
    "ve",
    "tell",
    "let",
    "know",
    "notify",
    "notified",
    "ping",
    "pinged",
    "alert",
    "alerted",
    "warn",
    "warned",
    "message",
    "text",
    "page",
    "buzz",
    "nudge",
    "wake",
    "up",
    "hear",
    "want",
    "need",
    "like",
    "to",
    "of",
    "on",
    "in",
    "at",
    "for",
    "about",
    "with",
    "from",
    "by",
    "into",
    "inside",
    "within",
    "after",
    "get",
    "gets",
    "got",
    "told",
    "informed",
    "inform",
    "give",
    "send",
    "drop",
    "shout",
    "flag",
    "heads",
    "heads-up",
    "keep",
    "eye",
    "watch",
    "monitor",
    "track",
    "make",
    "sure",
    "bot",
    "just",
    "right",
    "away",
    "immediately",
    "asap",
    "quick",
    "quickly",
    "know",
    "whether",
    "only",
    "nur",
    "anywhere",
    "irgendwo",
    "wann",
    "telling",
    "care",
    // German
    "ich",
    "mich",
    "mir",
    "mein",
    "meine",
    "meinen",
    "meinem",
    "meiner",
    "meines",
    "du",
    "dich",
    "dir",
    "es",
    "er",
    "sie",
    "ihn",
    "ihm",
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
    "eines",
    "und",
    "oder",
    "dann",
    "wenn",
    "falls",
    "sobald",
    "bitte",
    "sag",
    "sage",
    "sagen",
    "gib",
    "geben",
    "bescheid",
    "benachrichtige",
    "benachrichtigen",
    "informiere",
    "informieren",
    "melde",
    "melden",
    "warne",
    "warnen",
    "weck",
    "wecke",
    "wecken",
    "ist",
    "sind",
    "war",
    "wird",
    "werden",
    "hat",
    "haben",
    "kannst",
    "könntest",
    "möchte",
    "will",
    "wissen",
    "zu",
    "im",
    "in",
    "an",
    "am",
    "auf",
    "bei",
    "beim",
    "mit",
    "für",
    "von",
    "vom",
    "so",
    "ja",
    "kurz",
    "gleich",
    "sofort",
    "beobachte",
    "überwache",
    "achte",
    "hey",
    "hallo",
    "darüber",
    "davon",
    "dazu",
    "sein",
    "seine",
    "seinen",
    "ihr",
    "ihre",
    "ihren",
    "posted",
    "lang",
    "among",
    "amongst",
];
/// Kind words: part of the target ("the X session", "the X one").
const KINDS: &[&str] = &[
    "session",
    "sessions",
    "sitzung",
    "sitzungen",
    "agent",
    "agents",
    "chat",
    "chats",
    "project",
    "projects",
    "projekt",
    "projekte",
    "projekten",
    "app",
    "apps",
    "application",
    "account",
    "accounts",
    "konto",
    "konten",
    "subscription",
    "abo",
    "plan",
    "job",
    "jobs",
    "lauf",
];
/// A broad word, a target's only before a kind word or a name.
const QUANTIFIERS: &[&str] = &[
    "any",
    "all",
    "every",
    "each",
    "alle",
    "allen",
    "aller",
    "jede",
    "jeder",
    "jedes",
    "jeden",
    "irgendeine",
    "irgendein",
    "irgendeiner",
];
/// Small words between a quantifier and its kind word or name.
const BETWEEN: &[&str] = &[
    "of", "the", "my", "meine", "meinen", "meiner", "der", "die", "den", "von", "aus",
];
/// Words beside a condition's cue word ("asks me anything", "looks stuck").
const COND_EXTRA: &[&str] = &[
    "anything",
    "something",
    "etwas",
    "irgendwas",
    "looks",
    "look",
    "seems",
    "seem",
    "appears",
    "goes",
    "go",
    "went",
    "becomes",
    "become",
    "again",
    "drops",
    "falls",
    "sinks",
    "fällt",
    "sinkt",
    "scheint",
    "wirkt",
    "hits",
    "hit",
    "reaches",
    "reached",
    "erreicht",
    "erreichen",
    "runway",
    "quota",
    "allowance",
    "left",
    "remaining",
    "übrig",
    "kontingent",
    "new",
    "neue",
    "neuer",
    "neues",
    "neuen",
    "out",
    "bleibt",
    "geblieben",
    "wieder",
    "needs",
    "need",
    "braucht",
    "brauche",
    "answer",
    "usage",
    "leaves",
    "running",
    "review",
    "throws",
    "making",
    "sits",
    "sitting",
    "up",
    "down",
    "spins",
    "gets",
    "comes",
    "back",
];
/// An indefinite word, a target's before a preposition and a name
/// ("anything in pier", "something with the docs session").
const INDEFINITE: &[&str] = &["anything", "something", "etwas", "irgendwas", "irgendetwas"];
const PREPOSITIONS: &[&str] = &[
    "in", "im", "with", "about", "from", "of", "on", "mit", "bei", "von", "aus", "inside",
];
/// Words that leave something out ("but", "except", "unless", "aren't").
const NEGATIONS: &[&str] = &[
    "but",
    "except",
    "excluding",
    "without",
    "not",
    "never",
    "unless",
    "other",
    "apart",
    "aside",
    "besides",
    "skip",
    "skipping",
    "ignore",
    "ignoring",
    "leave",
    "leaving",
    "only",
    "t",
    "nicht",
    "außer",
    "ohne",
    "aber",
    "nur",
    "ausgenommen",
    "ignoriere",
    "kein",
    "keine",
    "keinen",
];
/// Asking phrases whose words aren't filler alone.
const ASK_PHRASES: &[&str] = &[
    // Scope-neutral context (v12): filler only because the negation guard
    // has already turned away any request with a word that leaves
    // something out.
    "whatever else is running",
    "whatever else is going on",
    "whatever else is happening",
    "whatever else happens",
    "egal was sonst läuft",
    "egal was sonst passiert",
    "across everything i run",
    "across everything i have running",
    "von allen meinen sitzungen",
    "out of",
    "the moment",
    "the minute",
    "any time",
    "as soon as",
    "heads up",
];
const MINUTE_UNITS: &[&str] = &["min", "mins", "minute", "minutes", "minuten"];
const HOUR_UNITS: &[&str] = &["h", "hr", "hrs", "hour", "hours", "stunde", "stunden"];
const PERCENT_UNITS: &[&str] = &["percent", "prozent"];
/// Words before a number of minutes ("for more than 10 minutes").
const MINUTE_WORDS: &[&str] = &[
    "for",
    "after",
    "over",
    "longer",
    "more",
    "than",
    "at",
    "least",
    "nach",
    "seit",
    "über",
    "länger",
    "mehr",
    "als",
    "mindestens",
];
/// A quiet-hours phrase's core.
const QUIET_CORE: &[&str] = &[
    "quiet",
    "ruhezeit",
    "ruhezeiten",
    "nachtruhe",
    "night",
    "nights",
    "nachts",
    "nacht",
    "overnight",
    "1am",
    "2am",
    "3am",
    "4am",
    "5am",
];
/// Words before a quiet-hours core ("but hold it until quiet hours end").
const QUIET_BEFORE: &[&str] = &[
    "even", "during", "in", "at", "my", "the", "but", "not", "hold", "it", "wait", "save", "until",
    "till", "outside", "after", "middle", "of", "auch", "während", "der", "die", "aber", "nicht",
    "warte", "bis", "trotz", "wake", "me", "up", "if", "it's",
];
/// Words after a quiet-hours core ("quiet hours are over", "too").
const QUIET_AFTER: &[&str] = &[
    "hours", "hour", "end", "ends", "are", "is", "over", "too", "as", "well", "vorbei", "ist",
    "sind", "auch", "or", "not",
];
/// The words that keep a watch held through quiet hours (the default).
const QUIET_OFF: &[&str] = &[
    "not", "nicht", "hold", "wait", "save", "until", "till", "bis", "outside", "after", "warte",
];
/// The words that ask to be told even then.
const QUIET_ON: &[&str] = &["even", "auch", "trotz", "wake", "too", "during", "während"];

fn is(set: &[&str], w: &str) -> bool {
    set.contains(&w)
}

/// The request's words as the cover reads them ([`words`]' split, `%` a
/// word of its own, "percent"), with where each is in `text`.
#[must_use]
pub fn token_spans(text: &str) -> Vec<(String, usize, usize)> {
    const INNER: &str = "-_./:";
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let close = |out: &mut Vec<(String, usize, usize)>, a: usize, b: usize| {
        let raw = &text[a..b];
        let lead = raw.len() - raw.trim_start_matches(|c: char| INNER.contains(c)).len();
        let trail = raw.len() - raw.trim_end_matches(|c: char| INNER.contains(c)).len();
        if lead + trail < raw.len() {
            out.push((
                raw[lead..raw.len() - trail].to_lowercase(),
                a + lead,
                b - trail,
            ));
        }
    };
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() || INNER.contains(c) {
            start.get_or_insert(i);
            continue;
        }
        if let Some(a) = start.take() {
            close(&mut out, a, i);
        }
        if c == '%' {
            out.push(("percent".to_owned(), i, i + 1));
        }
    }
    if let Some(a) = start {
        close(&mut out, a, text.len());
    }
    out
}

/// [`token_spans`]' words.
#[must_use]
pub fn tokens(text: &str) -> Vec<String> {
    token_spans(text).into_iter().map(|(w, ..)| w).collect()
}

/// The request's own text from word `a` to word `b` (exclusive).
fn said(text: &str, from: usize, to: usize) -> String {
    let spans = token_spans(text);
    match (
        spans.get(from),
        to.checked_sub(1).and_then(|last| spans.get(last)),
    ) {
        (Some(first), Some(last)) if to > from => text[first.1..last.2].to_owned(),
        _ => String::new(),
    }
}

/// A number word's or digits' value.
fn number(w: &str) -> Option<u32> {
    if !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()) {
        return w.parse().ok();
    }
    number_words()
        .iter()
        .find(|(k, _)| *k == w)
        .map(|(_, v)| *v)
}

fn phrase_at(w: &[String], i: usize, p: &str) -> Option<usize> {
    let p = words(p);
    (!p.is_empty() && w.len() >= i + p.len() && w[i..i + p.len()] == p[..]).then_some(p.len())
}

// ============================================================================ code's reading

/// Code's reading of `ask` (see the module).
#[must_use]
pub fn read(ask: &Ask<'_>) -> Reading {
    let w = tokens(ask.request);
    let mut r = Reading {
        slots: vec![None; w.len()],
        words: w,
        params: Vec::new(),
        quiet_on: false,
    };
    names(ask, &mut r);
    quiet(&mut r);
    lifetimes(&mut r);
    for p in ASK_PHRASES {
        phrases(&mut r, p, Slot::Filler);
    }
    params(&mut r);
    cues(&mut r);
    rest(&mut r);
    r
}

fn put(r: &mut Reading, a: usize, b: usize, slot: Slot) {
    for x in &mut r.slots[a..b] {
        if x.is_none() {
            *x = Some(slot);
        }
    }
}

fn free(r: &Reading, a: usize, b: usize) -> bool {
    r.slots[a..b].iter().all(Option::is_none)
}

/// Every phrase `p` said, on words not yet placed.
fn phrases(r: &mut Reading, p: &str, slot: Slot) {
    for i in 0..r.words.len() {
        if let Some(n) = phrase_at(&r.words, i, p)
            && free(r, i, i + n)
        {
            put(r, i, i + n, slot);
        }
    }
}

/// The candidates' names said whole (with dashes as spaces too), longest
/// first: the target's words.
fn names(ask: &Ask<'_>, r: &mut Reading) {
    let mut all: Vec<Vec<String>> = ask
        .candidates
        .iter()
        .flat_map(|c| {
            std::iter::once(c.name.as_str())
                .chain(c.aliases.iter().map(String::as_str))
                .chain(c.app.as_deref())
                .chain(c.project.as_deref())
        })
        .filter(|n| n.chars().count() >= 2)
        .flat_map(|n| [tokens(n), resolve::spaced(n)])
        .filter(|n| !n.is_empty())
        .collect();
    all.sort_by_key(|n| std::cmp::Reverse(n.len()));
    all.dedup();
    for n in &all {
        for i in 0..r.words.len() {
            if r.words.len() >= i + n.len()
                && r.words[i..i + n.len()] == n[..]
                && free(r, i, i + n.len())
            {
                put(r, i, i + n.len(), Slot::Target);
            }
        }
    }
}

/// Quiet-hours phrases: a core word ("quiet hours", "at night", "3am")
/// and the words around it; on ("even during quiet hours") or off ("but
/// hold it until quiet hours end"). One that says both, or a night with
/// neither, is left unplaced.
fn quiet(r: &mut Reading) {
    let len = r.words.len();
    let mut at = 0;
    while at < len {
        if r.slots[at].is_some() || !is(QUIET_CORE, &r.words[at]) {
            at += 1;
            continue;
        }
        let mut a = at;
        while a > 0 && r.slots[a - 1].is_none() && is(QUIET_BEFORE, &r.words[a - 1]) {
            a -= 1;
        }
        let mut b = at + 1;
        while b < len && r.slots[b].is_none() && is(QUIET_AFTER, &r.words[b]) {
            b += 1;
        }
        let said = &r.words[a..b];
        // "quiet hours or not": even then.
        let either = said.ends_with(&["or".to_owned(), "not".to_owned()]);
        let said = if either {
            &said[..said.len() - 2]
        } else {
            said
        };
        let any = |set: &[&str]| said.iter().any(|x| is(set, x));
        let off = any(QUIET_OFF);
        let strong = either || any(&["even", "auch", "trotz", "wake", "too"]);
        let quiet_said = any(&["quiet", "ruhezeit", "ruhezeiten", "nachtruhe"]);
        // "not even at night": both; a night with no "even": neither.
        let on = !off && (strong || any(QUIET_ON));
        if !(strong && off) && (quiet_said || strong) {
            put(r, a, b, Slot::QuietHours);
            r.quiet_on |= on;
        }
        at = b;
    }
}

/// Lifetime phrases, and "until"/"by"/"bis" with a clock time.
fn lifetimes(r: &mut Reading) {
    for p in LIFE_ENDS
        .iter()
        .chain(LIFE_ONCE)
        .chain(LIFE_ALWAYS)
        .chain(LIFE_TODAY)
    {
        phrases(r, p, Slot::Lifetime);
    }
    for i in 0..r.words.len() {
        if !is(&["until", "till", "by", "bis"], &r.words[i]) || r.slots[i].is_some() {
            continue;
        }
        for len in [1, 2] {
            if i + 1 + len <= r.words.len()
                && free(r, i, i + 1 + len)
                && !request_times(&r.words[i + 1..i + 1 + len].join(" ")).is_empty()
                && (len == 1 || is(&["am", "pm", "uhr"], &r.words[i + 2]))
            {
                put(r, i, i + 1 + len, Slot::Lifetime);
                break;
            }
        }
    }
}

/// Numbers with a unit: "10 minutes", "an hour", "half an hour", "20%".
fn params(r: &mut Reading) {
    for i in 1..r.words.len() {
        if r.slots[i].is_some() {
            continue;
        }
        let u = r.words[i].as_str();
        let (unit, scale) = if is(MINUTE_UNITS, u) {
            (Unit::Minutes, 1)
        } else if is(HOUR_UNITS, u) {
            (Unit::Minutes, 60)
        } else if is(PERCENT_UNITS, u) {
            (Unit::Percent, 1)
        } else {
            continue;
        };
        let before = r.words[i - 1].as_str();
        let half = unit == Unit::Minutes
            && scale == 60
            && i >= 2
            && ((r.words[i - 2] == "half" && before == "an")
                || (["eine", "einer"].contains(&r.words[i - 2].as_str())
                    && ["halbe", "halben"].contains(&before)));
        let (start, value) = if half {
            (i - 2, 30)
        } else if let Some(n) = number(before) {
            (i - 1, n * scale)
        } else if unit == Unit::Minutes && is(&["a", "an", "one", "eine", "einer", "einem"], before)
        {
            (i - 1, scale)
        } else {
            continue;
        };
        if !free(r, start, i + 1) {
            continue;
        }
        let mut a = start;
        if unit == Unit::Minutes {
            while a > 0 && r.slots[a - 1].is_none() && is(MINUTE_WORDS, &r.words[a - 1]) {
                a -= 1;
            }
        }
        put(r, a, i + 1, Slot::ConditionParam);
        r.params.push((a, i, unit, value));
    }
}

/// A word is a cue: alone, or with its neighbour where neither is alone
/// ("goes wrong", "no progress", "below 20"). A number is never one.
fn cue_at(w: &[String], i: usize) -> bool {
    if number(&w[i]).is_some() {
        return false;
    }
    let alone = |k: usize| !cue_conditions(&w[k]).is_empty();
    if alone(i) {
        // "the question I asked": not "asks you".
        return !(i > 0
            && ["i", "you"].contains(&w[i - 1].as_str())
            && cue_conditions(&w[i]) == ["asks_you"]);
    }
    // Only what the words say together: neither (none) a cue alone.
    let pair = |a: usize, b: usize| {
        b < w.len() && !alone(a) && !alone(b) && !cue_conditions(&w[a..=b].join(" ")).is_empty()
    };
    let triple = |a: usize| {
        a + 2 < w.len()
            && (a..a + 3).all(|k| !alone(k))
            && !cue_conditions(&w[a..a + 3].join(" ")).is_empty()
    };
    (i > 0 && pair(i - 1, i)) || pair(i, i + 1) || triple(i) || (i > 0 && triple(i - 1))
}

fn cues(r: &mut Reading) {
    for i in 0..r.words.len() {
        if r.slots[i].is_none() && cue_at(&r.words, i) {
            r.slots[i] = Some(Slot::Condition);
        }
    }
    for i in 0..r.words.len() {
        // "new" before its session, whatever between: a start's.
        let span = if is(&["new", "neue", "neuer", "neues", "neuen"], &r.words[i]) {
            6
        } else {
            3
        };
        let near = (i.saturating_sub(span)..(i + span + 1).min(r.words.len()))
            .any(|k| r.slots[k] == Some(Slot::Condition));
        if r.slots[i].is_none() && is(COND_EXTRA, &r.words[i]) && near {
            r.slots[i] = Some(Slot::Condition);
        }
    }
}

/// Kind words, quantifiers before a target, then the filler list.
fn rest(r: &mut Reading) {
    let n = r.words.len();
    for i in 0..n {
        let after_name = i > 0 && r.slots[i - 1] == Some(Slot::Target);
        if r.slots[i].is_none() && (is(KINDS, &r.words[i]) || (r.words[i] == "one" && after_name)) {
            r.slots[i] = Some(Slot::Target);
        }
    }
    for i in 0..n {
        let indefinite = is(INDEFINITE, &r.words[i]);
        if r.slots[i].is_some() || !(is(QUANTIFIERS, &r.words[i]) || indefinite) {
            continue;
        }
        let between = |w: &str| is(BETWEEN, w) || (indefinite && is(PREPOSITIONS, w));
        let next = (i + 1..n).find(|&k| !between(&r.words[k]));
        // "anything in pier": a preposition first; "any session": none.
        let joined = !indefinite || (i + 1 < n && is(PREPOSITIONS, &r.words[i + 1]));
        if joined && next.is_some_and(|k| r.slots[k] == Some(Slot::Target)) {
            r.slots[i] = Some(Slot::Target);
        } else if !indefinite && r.slots.get(i + 1) == Some(&Some(Slot::Condition)) {
            // "any errors", "every failure": what is asked.
            r.slots[i] = Some(Slot::Condition);
        }
    }
    for i in 0..n {
        if r.slots[i].is_none() && is(FILLER, &r.words[i]) {
            r.slots[i] = Some(Slot::Filler);
        }
    }
}

impl Reading {
    /// What code makes of the request (see [`Cover`]).
    #[must_use]
    pub fn cover(&self, ask: &Ask<'_>) -> Cover {
        let left: Vec<usize> = (0..self.words.len())
            .filter(|&i| self.slots[i].is_none())
            .collect();
        if !left.is_empty() {
            return Cover::Leftover(left);
        }
        if !self.slots.contains(&Some(Slot::Condition)) {
            return Cover::NoCondition;
        }
        let said = cue_conditions(&masked(ask));
        for &(a, b, unit, _) in &self.params {
            let takes = match unit {
                Unit::Minutes => ["stuck", "asks_you", "idle_for"].as_slice(),
                Unit::Percent => ["runway_below"].as_slice(),
            };
            if !said.iter().any(|c| takes.contains(&c.as_str())) {
                return Cover::Untaken(self.words[a..=b].join(" "));
            }
        }
        Cover::Claimed
    }

    /// The request's own words from the first word in `slot` to the last.
    #[must_use]
    pub fn said_as(&self, request: &str, slot: Slot) -> String {
        let idx: Vec<usize> = (0..self.words.len())
            .filter(|&i| self.slots[i] == Some(slot))
            .collect();
        let (a, b) = self.run(&idx);
        said(request, a, b)
    }

    /// The words from the first to the last of `idx`, with a number and
    /// its unit beside them ("unless it ran for under an hour").
    fn run(&self, idx: &[usize]) -> (usize, usize) {
        let mut a = idx.iter().copied().min().unwrap_or(0);
        let mut b = idx.iter().copied().max().map_or(0, |b| b + 1);
        let param = |i: usize| self.slots.get(i) == Some(&Some(Slot::ConditionParam));
        while a > 0 && param(a - 1) {
            a -= 1;
        }
        while param(b) {
            b += 1;
        }
        (a, b)
    }
}

// ============================================================================ the negation guard

/// Words that leave something out (v12's guard; "only" and "nur" are
/// scope words, not these).
const NEGATION_WORDS: &[&str] = &[
    "not",
    "never",
    "nie",
    "niemals",
    "nicht",
    "except",
    "excluding",
    "exclude",
    "außer",
    "ausser",
    "ausgenommen",
    "unless",
    "without",
    "ohne",
    "ignore",
    "ignoring",
    "ignoriere",
    "skip",
    "skipping",
    "cannot",
];
const NEGATION_PHRASES: &[&str] = &["es sei denn", "but not", "other than", "apart from"];
/// The first half of an "n't" ("don't" is "don", "t").
const NOT_STEMS: &[&str] = &[
    "don", "doesn", "didn", "isn", "aren", "wasn", "weren", "won", "can", "couldn", "shouldn",
    "wouldn", "haven", "hasn", "hadn", "ain", "mustn", "needn",
];

/// The first word that leaves something out, outside a name and a
/// quiet-hours phrase code placed ("but not during quiet hours" keeps the
/// default): `None` when there is none. Whatever the labels say.
#[must_use]
pub fn negated(r: &Reading) -> Option<usize> {
    let w = &r.words;
    (0..w.len()).find(|&i| {
        if matches!(r.slots[i], Some(Slot::Target | Slot::QuietHours)) {
            return false;
        }
        is(NEGATION_WORDS, &w[i])
            || w[i].starts_with("kein")
            || (w[i] == "t" && i > 0 && is(NOT_STEMS, &w[i - 1]))
            || NEGATION_PHRASES
                .iter()
                .any(|p| phrase_at(w, i, p).is_some())
    })
}

/// The request's own words from word `from` to its clause's end (at most
/// eight words), and where that ends.
#[must_use]
pub fn clause_from(request: &str, from: usize) -> (String, usize) {
    let t = token_spans(request);
    let mut end = from + 1;
    while end < t.len() && end < from + 8 {
        let gap = &request[t[end - 1].2..t[end].1];
        if gap.contains(['.', ';', '!', '?']) || (gap.contains(',') && end > from + 1) {
            break;
        }
        end += 1;
    }
    let end = end.min(t.len());
    let start = if from > 0 && t[from].0 == "t" {
        from - 1
    } else {
        from
    };
    (said(request, start, end), end)
}

// ============================================================================ told back

/// The nearest option for a request code can't draft: the target its
/// words name outside `skip` (one candidate, else none: the form asks), and the
/// first condition said there that the target offers, else the first it
/// offers of finishes, fails, ends, asks you, stuck. Always one.
#[must_use]
pub fn nearest(
    ask: &Ask<'_>,
    r: &Reading,
    skip: (usize, usize),
    cue_skip: (usize, usize),
) -> WatchDraftInfo {
    let outside = |(a, b): (usize, usize)| {
        r.words
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < a || *i >= b)
            .map(|(_, x)| x.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };
    // The target: the one the asking clause names (exclusions honoured),
    // a session or an account, said whole outside what is told back.
    let kept = tokens(&outside(skip));
    // A word that leaves something out, unplaced: no target at all, so
    // the nearest is never the one left out ("everything but form-kit").
    let leaves_out = (0..r.words.len()).any(|i| r.slots[i].is_none() && is(NEGATIONS, &r.words[i]));
    let clause = crate::features::watches::fail_closed::watch_clause(ask)
        .unwrap_or_else(|| ask.request.to_owned());
    let found = resolve::resolve(&clause, ask.candidates);
    let one = match (found.candidates.as_slice(), found.picker.is_empty()) {
        ([c], true) if !leaves_out => Some(&ask.candidates[*c]).filter(|c| {
            matches!(c.target.kind(), TargetKind::Session | TargetKind::Account)
                && std::iter::once(&c.name)
                    .chain(&c.aliases)
                    .any(|n| resolve::full_match(&kept, n))
        }),
        _ => None,
    };
    let offered: Vec<String> =
        one.map_or_else(|| draft::all_offered(ask.candidates), |c| c.offered.clone());
    let cue_text = {
        let sub = Ask {
            request: &outside(cue_skip),
            candidates: ask.candidates,
            midnight_ms: ask.midnight_ms,
            now_ms: ask.now_ms,
        };
        masked(&sub)
    };
    let code = draft::nearest_first(ask)
        .into_iter()
        .chain(cue_conditions(&cue_text))
        .chain(["finishes", "fails", "ends", "asks_you", "stuck"].map(str::to_owned))
        .find(|c| offered.iter().any(|o| o == c) && draft::definition_of(c))
        .unwrap_or_else(|| "finishes".to_owned());
    // No one target named: the form asks for it (never widened to any).
    draft_info(
        one.map(|c| c.target.clone()).into_iter().collect(),
        vec![Condition { code, param: None }],
        LifetimeChoice::Unsaid,
    )
}

/// What the labels give: a draft code verified (with what its target
/// offers), or what is told back.
#[derive(Debug)]
pub enum Labelled {
    /// Every span verified by code.
    Drafted(WatchDraftInfo, Vec<String>),
    /// What can't be watched, with the nearest.
    Told(WatchDrafted),
}

/// A request code can't place word by word (`left`), labelled by the
/// model (v12: verified labels may draft). Code checks the spans repeat
/// the request exactly; an exclusion or other span is told back; else the
/// draft stands only when every check of [`verify`] holds. Labels code
/// can't verify are asked again, then code tells back its own reading.
/// Returns the answer and the calls made.
pub fn labelled(
    model: &dyn JobModel,
    ask: &Ask<'_>,
    r: &Reading,
    left: &[usize],
    stop: &Stop,
) -> (Labelled, u32) {
    let mut calls = 0;
    let mut note = String::new();
    for attempt in 0..=draft::RETRIES {
        let req = label_request(ask, &note, attempt);
        calls += 1;
        let Ok(out) = model.generate(&req, stop) else {
            break;
        };
        let why = match spans_of(&out.text, out.finish, &r.words) {
            Ok(spans) => match verify(ask, r, &spans) {
                Ok((d, offered)) => return (Labelled::Drafted(d, offered), calls),
                Err(Check::Told(slot, a, b)) => {
                    return (
                        Labelled::Told(told_span(ask, r, &spans, (slot, a, b))),
                        calls,
                    );
                }
                Err(Check::Unverified(why)) => why,
            },
            Err(why) => why,
        };
        note = retry_note(&out.text, &why);
    }
    (Labelled::Told(code_told_back(ask, r, left)), calls)
}

/// A span that says what can't be watched, told back: the target from
/// outside every such span, the condition from outside what is left out.
fn told_span(
    ask: &Ask<'_>,
    r: &Reading,
    spans: &[(Slot, usize, usize)],
    (slot, a, b): (Slot, usize, usize),
) -> WatchDrafted {
    let told = |s: Slot| matches!(s, Slot::Other | Slot::Exclusion);
    let skip = spans
        .iter()
        .filter(|(s, ..)| told(*s))
        .fold((a, b), |(x, y), (_, p, q)| (x.min(*p), y.max(*q)));
    let cue_skip = if slot == Slot::Exclusion {
        (a, b)
    } else {
        (0, 0)
    };
    WatchDrafted::CantWatch {
        asked: said(ask.request, a, b),
        draft: Some(nearest(ask, r, skip, cue_skip)),
    }
}

/// Why labels don't draft.
enum Check {
    /// An exclusion or other span: told back (its slot and words).
    Told(Slot, usize, usize),
    /// Code can't verify them: why.
    Unverified(String),
}

/// The word indexes of the spans in `want`.
fn indexes(spans: &[(Slot, usize, usize)], want: Slot) -> Vec<usize> {
    spans
        .iter()
        .filter(|(s, ..)| *s == want)
        .flat_map(|(_, a, b)| *a..*b)
        .collect()
}

/// v12's checks on labels that repeat the request, all by code: no
/// exclusion or other span; a word code can't place only in a target span,
/// and then a word of the one entity the target spans resolve to (a name
/// said in part also says its kind, or two of its words); the condition
/// spans name exactly one condition (the cue table and its closed
/// paraphrase map); every number in a parameter span whose unit that
/// condition takes. The negation guard ran before any label.
fn verify(
    ask: &Ask<'_>,
    r: &Reading,
    spans: &[(Slot, usize, usize)],
) -> Result<(WatchDraftInfo, Vec<String>), Check> {
    let bad = |why: String| Err(Check::Unverified(why));
    if let Some(&(slot, a, b)) = spans
        .iter()
        .find(|(s, ..)| matches!(s, Slot::Other | Slot::Exclusion))
    {
        return Err(Check::Told(slot, a, b));
    }
    for &(slot, a, b) in spans {
        for i in a..b {
            if r.slots[i].is_none() && slot != Slot::Target {
                return bad(format!("“{}” is no {}", r.words[i], slot_word(slot)));
            }
            if number(&r.words[i]).is_some()
                && slot != Slot::ConditionParam
                && !matches!(r.slots[i], Some(Slot::Lifetime | Slot::QuietHours))
            {
                return bad(format!(
                    "the number “{}” belongs in a condition_param span",
                    r.words[i]
                ));
            }
        }
    }
    let target_idx = indexes(spans, Slot::Target);
    let (target, offered) = target_of(ask, r, &target_idx).ok_or_else(|| {
        Check::Unverified(
            "the target spans name no one session, project, App or account".to_owned(),
        )
    })?;
    let cond_text = indexes(spans, Slot::Condition)
        .iter()
        .map(|&i| r.words[i].as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let [code] = &cue_conditions(&cond_text)[..] else {
        return bad("the condition spans name no one condition".to_owned());
    };
    let param = param_of(r, &indexes(spans, Slot::ConditionParam), code)?;
    if param.is_none() && ["idle_for", "runway_below"].contains(&code.as_str()) {
        return bad("that condition needs its number".to_owned());
    }
    let lifetime = match draft::lifetime_for(ask) {
        LifetimeChoice::UntilEnds if target.kind() != TargetKind::Session => LifetimeChoice::Unsaid,
        other => other,
    };
    let mut d = draft_info(
        vec![target],
        vec![Condition {
            code: code.clone(),
            param,
        }],
        lifetime,
    );
    d.even_in_quiet_hours = r.quiet_on;
    Ok((d, offered))
}

/// The parameter span's number, read by code: one number with its unit,
/// a unit `code` takes (else told back: a number no condition takes).
fn param_of(r: &Reading, idx: &[usize], code: &str) -> Result<Option<Param>, Check> {
    let (Some(&first), Some(&last)) = (idx.first(), idx.last()) else {
        return Ok(None);
    };
    let mut sub = Reading {
        words: idx.iter().map(|&i| r.words[i].clone()).collect(),
        slots: vec![None; idx.len()],
        params: Vec::new(),
        quiet_on: false,
    };
    params(&mut sub);
    let [(_, _, unit, value)] = sub.params[..] else {
        return Err(Check::Unverified(
            "a condition_param span holds one number with its unit".to_owned(),
        ));
    };
    let takes = match unit {
        Unit::Minutes => ["stuck", "asks_you", "idle_for"].contains(&code),
        Unit::Percent => code == "runway_below",
    };
    if !takes {
        return Err(Check::Told(Slot::Other, first, last + 1));
    }
    draft::param_of(code, &value.to_string()).map_err(Check::Unverified)
}

fn slot_word(slot: Slot) -> &'static str {
    SLOTS
        .iter()
        .find(|(_, s)| *s == slot)
        .map_or("slot", |(k, _)| k)
}

/// The one entity the target spans resolve to (or any session), and what
/// it offers: a word code can't place must be one of that entity's own
/// name words, and a name said only in part comes with its kind word or
/// two of its words.
fn target_of(ask: &Ask<'_>, r: &Reading, idx: &[usize]) -> Option<(WatchTarget, Vec<String>)> {
    let said_words: Vec<String> = idx.iter().map(|&i| r.words[i].clone()).collect();
    if said_words.is_empty() {
        return None;
    }
    let found = resolve::resolve(&said_words.join(" "), ask.candidates);
    let unplaced: Vec<&str> = idx
        .iter()
        .filter(|&&i| r.slots[i].is_none())
        .map(|&i| r.words[i].as_str())
        .collect();
    match (found.candidates.as_slice(), found.picker.is_empty()) {
        ([c], true) => {
            let c = &ask.candidates[*c];
            let names: Vec<&str> = std::iter::once(c.name.as_str())
                .chain(c.aliases.iter().map(String::as_str))
                .collect();
            let mut own: Vec<String> = names
                .iter()
                .flat_map(|n| tokens(n).into_iter().chain(resolve::spaced(n)))
                .collect();
            own.sort();
            own.dedup();
            if !unplaced.iter().all(|u| own.iter().any(|o| o == u)) {
                return None;
            }
            let whole = names.iter().any(|n| resolve::full_match(&said_words, n));
            let kind = said_words.iter().any(|x| is(KINDS, x));
            let parts = own.iter().filter(|o| said_words.contains(o)).count();
            (whole || kind || parts >= 2).then(|| (c.target.clone(), c.offered.clone()))
        }
        ([], _) if found.any && unplaced.is_empty() => {
            Some((WatchTarget::Any, draft::all_offered(ask.candidates)))
        }
        _ => None,
    }
}

/// Code's own told-back answer: the words from the first it can't place
/// to the last, the target named outside them.
#[must_use]
pub fn code_told_back(ask: &Ask<'_>, r: &Reading, left: &[usize]) -> WatchDrafted {
    let (a, b) = r.run(left);
    WatchDrafted::CantWatch {
        asked: draft::told_kind(ask).unwrap_or_else(|| said(ask.request, a, b)),
        draft: Some(nearest(ask, r, (a, b), (0, 0))),
    }
}

fn retry_note(previous: &str, why: &str) -> String {
    let previous: String = previous.chars().take(500).collect();
    let why: String = why.chars().take(300).collect();
    format!(
        "\n\nYour previous answer was rejected because: {why}.\nYour previous answer was: {previous}\nLabel the spans again and fix that."
    )
}

/// The labelling call's guided schema.
#[must_use]
pub fn label_shape() -> Value {
    let slots: Vec<&str> = SLOTS.iter().map(|(s, _)| *s).collect();
    json!({ "type": "object", "name": "WatchSpans", "properties": [
        { "name": "spans", "schema": { "type": "array", "min": 1, "max": 16, "items": {
            "type": "object", "name": "WatchSpan", "properties": [
                { "name": "text", "schema": { "type": "string" } },
                { "name": "slot", "schema": { "type": "enum", "name": "WatchSpan_slot", "values": slots } },
                { "name": "number", "schema": { "type": "integer", "min": 0, "max": 100_000 } },
                { "name": "unit", "schema": { "type": "enum", "name": "WatchSpan_unit",
                    "values": ["none", "minutes", "hours", "percent", "other"] } },
            ] } } },
    ] })
}

fn label_request(ask: &Ask<'_>, note: &str, attempt: u32) -> Request {
    let mut names: Vec<String> = ask
        .candidates
        .iter()
        .map(|c| draft::quoted_name(&c.name))
        .collect();
    names.dedup();
    names.truncate(30);
    Request {
        instructions: LABEL_INSTRUCTIONS.to_owned(),
        prompt: format!(
            "NAMES (data written by agents): {}\n\nREQUEST (from the developer): {}\n\n{TASK}{note}",
            names.join(" · "),
            ask.request
        ),
        shape: Some(label_shape()),
        max_tokens: 320,
        decoding: Decoding::for_attempt(&format!("watch-label:{}", ask.request), attempt),
    }
}

/// The model's spans as word ranges of `w`, when they repeat it exactly
/// (no gap, no overlap, nothing added); else what is wrong.
fn spans_of(text: &str, finish: Finish, w: &[String]) -> Result<Vec<(Slot, usize, usize)>, String> {
    if finish == Finish::Length {
        return Err("shape: cut at the token limit".to_owned());
    }
    let got: Value = serde_json::from_str(text).map_err(|_| "not JSON".to_owned())?;
    let mut out = Vec::new();
    let mut at = 0;
    for s in got["spans"].as_array().into_iter().flatten() {
        let slot = SLOTS
            .iter()
            .find(|(k, _)| Some(*k) == s["slot"].as_str())
            .map(|(_, v)| *v)
            .ok_or_else(|| "slot: not one of its choices".to_owned())?;
        let t = tokens(s["text"].as_str().unwrap_or_default());
        if t.is_empty() {
            continue;
        }
        if w.len() < at + t.len() || w[at..at + t.len()] != t[..] {
            let expected = w
                .get(at..(at + 4).min(w.len()))
                .map(|x| x.join(" "))
                .unwrap_or_default();
            return Err(format!(
                "the spans must repeat the request word for word, in order; after word {at} the request goes on “{expected}”"
            ));
        }
        out.push((slot, at, at + t.len()));
        at += t.len();
    }
    if at != w.len() {
        return Err(format!(
            "the spans left out the end of the request: “{}”",
            w[at..].join(" ")
        ));
    }
    Ok(out)
}

#[cfg(test)]
#[path = "cover_tests.rs"]
mod tests;
