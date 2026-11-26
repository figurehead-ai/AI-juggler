//! AJ-E3830 (harness v13): the model's labels, word by word, for the words
//! code couldn't place, and code's checks on them: a wrong length refused,
//! the second guard on filler, a target only as a name word of one entity.

use serde_json::{Value, json};

use super::{
    COND_EXTRA, Cover, KINDS, LABEL_INSTRUCTIONS, NEGATION_WORDS, NOT_STEMS, Reading, SLOTS, Slot,
    TASK, clause_from, code_told_back, is, nearest, number, retry_note, said, tokens,
};
use crate::features::ai::engine::{Decoding, Finish, Request, Stop};
use crate::features::ai::run::JobModel;
use crate::features::watches::commands::WatchDraftInfo;
use crate::features::watches::draft::{self, Ask, WatchDrafted, cue_conditions, draft_info};
use crate::features::watches::model::{Condition, LifetimeChoice, TargetKind, WatchTarget};
use crate::features::watches::resolve;

/// What the labels give: a draft code verified (with what its target
/// offers), code's own pipeline once every word is placed, or what is
/// told back.
#[derive(Debug)]
pub enum Labelled {
    /// A target word the labels named, verified by code.
    Drafted(WatchDraftInfo, Vec<String>),
    /// Every word placed now (the labels' filler passed the second
    /// guard): code drafts as for a request it placed itself.
    Claimed,
    /// What can't be watched, with the nearest.
    Told(WatchDrafted),
}

/// A request code can't place word by word (`left`), labelled word by
/// word (v13): the model sees every word numbered and gives one slot for
/// each word code couldn't place, an array of exactly that length (a
/// wrong length is refused and asked again, never padded or cut). Code's
/// slot stands for every word it placed; `judge` decides the rest.
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
        let req = label_request(ask, r, left, &note, attempt);
        calls += 1;
        let Ok(out) = model.generate(&req, stop) else {
            break;
        };
        match slots_of(&out.text, out.finish, left.len()) {
            Ok(labels) => return (judge(ask, r, left, &labels), calls),
            Err(why) => note = retry_note(&out.text, &why),
        }
    }
    (Labelled::Told(code_told_back(ask, r, left)), calls)
}

/// The labels, word by word: other or exclusion, told back; filler, only
/// when code sees nothing in the word that bears meaning (the second
/// guard); target, then a name word of the one entity the target words
/// resolve to; any other slot for a word code can't place, told back.
fn judge(ask: &Ask<'_>, r: &Reading, left: &[usize], labels: &[Slot]) -> Labelled {
    let mut merged = r.clone();
    let mut named = false;
    for (&i, &slot) in left.iter().zip(labels) {
        match slot {
            Slot::Filler if !meaning_bearing(ask, r, i) => {
                merged.slots[i] = Some(Slot::Filler);
            }
            Slot::Target => {
                merged.slots[i] = Some(Slot::Target);
                named = true;
            }
            _ => return Labelled::Told(told_at(ask, r, i, slot == Slot::Exclusion)),
        }
    }
    if !named {
        return match merged.cover(ask) {
            Cover::Claimed => Labelled::Claimed,
            Cover::Untaken(asked) => Labelled::Told(WatchDrafted::CantWatch {
                asked,
                draft: Some(nearest(ask, r, (0, 0), (0, 0))),
            }),
            _ => Labelled::Told(code_told_back(ask, r, left)),
        };
    }
    let target_idx: Vec<usize> = (0..merged.words.len())
        .filter(|&i| merged.slots[i] == Some(Slot::Target))
        .collect();
    let Some((target, offered)) = target_of(ask, r, &target_idx) else {
        return Labelled::Told(code_told_back(ask, r, left));
    };
    let cond_text = (0..merged.words.len())
        .filter(|&i| merged.slots[i] == Some(Slot::Condition))
        .map(|i| merged.words[i].as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let codes = cue_conditions(&cond_text);
    if codes.is_empty() {
        return Labelled::Told(code_told_back(ask, r, left));
    }
    let mut conditions: Vec<Condition> = codes
        .iter()
        .map(|c| Condition {
            code: c.clone(),
            param: None,
        })
        .collect();
    for &(a, b, unit, value) in &r.params {
        let Some(c) = conditions.iter_mut().find(|c| unit.taken_by(&c.code)) else {
            return Labelled::Told(WatchDrafted::CantWatch {
                asked: said(ask.request, a, b + 1),
                draft: Some(nearest(ask, r, (0, 0), (0, 0))),
            });
        };
        match draft::param_of(&c.code, &value.to_string()) {
            Ok(p) => c.param = p,
            Err(_) => return Labelled::Told(code_told_back(ask, r, left)),
        }
    }
    if conditions
        .iter()
        .any(|c| c.param.is_none() && ["idle_for", "runway_below"].contains(&c.code.as_str()))
    {
        return Labelled::Told(code_told_back(ask, r, left));
    }
    let lifetime = match draft::lifetime_for(ask) {
        LifetimeChoice::UntilEnds if target.kind() != TargetKind::Session => LifetimeChoice::Unsaid,
        other => other,
    };
    let mut d = draft_info(vec![target], conditions, lifetime);
    d.even_in_quiet_hours = r.quiet_on;
    Labelled::Drafted(d, offered)
}

/// Told back from word `i` to its clause's end, with the nearest (the
/// condition from outside it when it leaves something out).
fn told_at(ask: &Ask<'_>, r: &Reading, i: usize, excludes: bool) -> WatchDrafted {
    let (asked, end) = clause_from(ask.request, i);
    let cue_skip = if excludes { (i, end) } else { (0, 0) };
    WatchDrafted::CantWatch {
        asked,
        draft: Some(nearest(ask, r, (i, end), cue_skip)),
    }
}

/// Number words, and words for how often.
const COUNT_WORDS: &[&str] = &[
    "twice",
    "thrice",
    "zweimal",
    "dreimal",
    "viermal",
    "fünfmal",
    "dozen",
    "dutzend",
    "hundred",
    "hundert",
    "thousand",
    "tausend",
    "million",
    "millionen",
    "billion",
    "half",
    "halb",
    "halbe",
    "nine",
    "neun",
    "sechs",
    "sieben",
    "acht",
    "elf",
    "zwölf",
    "eleven",
    "twelve",
    "several",
    "mehrere",
    "many",
    "viele",
    "few",
    "wenige",
    "some",
    "einige",
    "most",
    "both",
    "beide",
    "either",
    "neither",
    "times",
    "mal",
    "mehrmals",
    "oftmals",
    "öfter",
    "häufig",
    "wiederholt",
    "repeatedly",
    "multiple",
    "frequently",
];
/// Time and date words.
const TIME_WORDS: &[&str] = &[
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "montag",
    "dienstag",
    "mittwoch",
    "donnerstag",
    "freitag",
    "samstag",
    "sonntag",
    "morning",
    "evening",
    "night",
    "nights",
    "tonight",
    "today",
    "tomorrow",
    "yesterday",
    "weekend",
    "weekends",
    "weekday",
    "weekdays",
    "week",
    "weeks",
    "month",
    "day",
    "days",
    "daily",
    "weekly",
    "nightly",
    "hourly",
    "monthly",
    "morgens",
    "abends",
    "nachts",
    "mittags",
    "morgen",
    "heute",
    "gestern",
    "wochenende",
    "woche",
    "wochen",
    "monat",
    "tag",
    "tage",
    "täglich",
    "stündlich",
    "wöchentlich",
    "am",
    "pm",
    "uhr",
    "noon",
    "midnight",
    "mitternacht",
    "hour",
    "hours",
    "minute",
    "minutes",
    "second",
    "seconds",
    "stunde",
    "stunden",
    "minuten",
    "sekunden",
    "before",
    "until",
    "since",
    "during",
    "while",
    "vor",
    "bis",
    "seit",
    "während",
    "solange",
    "later",
    "soon",
    "now",
    "jetzt",
    "spät",
    "late",
    "early",
    "früh",
];
/// Words for how many or which.
const SCOPE_WORDS: &[&str] = &[
    "only",
    "first",
    "last",
    "every",
    "each",
    "all",
    "any",
    "nur",
    "erste",
    "ersten",
    "erstes",
    "letzte",
    "letzten",
    "jede",
    "jeder",
    "jedes",
    "jeden",
    "alle",
    "allen",
    "again",
    "wieder",
    "always",
    "often",
    "sometimes",
    "immer",
    "oft",
    "manchmal",
    "still",
    "noch",
    "once",
    "more",
    "less",
    "fewer",
    "mehr",
    "weniger",
    "over",
    "under",
    "above",
    "below",
    "über",
    "unter",
];
/// Words that name what no condition takes: sizes, units, code work.
const CONTENT_WORDS: &[&str] = &[
    "mb",
    "gb",
    "kb",
    "tb",
    "megabyte",
    "megabytes",
    "gigabyte",
    "gigabytes",
    "bytes",
    "tokens",
    "token",
    "files",
    "file",
    "lines",
    "line",
    "commits",
    "commit",
    "dollars",
    "dollar",
    "euro",
    "euros",
    "cost",
    "costs",
    "kosten",
    "price",
    "budget",
    "logs",
    "log",
    "size",
    "memory",
    "cpu",
    "ram",
    "disk",
    "branch",
    "branches",
    "pr",
    "prs",
    "ci",
    "release",
    "releases",
    "pipeline",
    "build",
    "builds",
    "version",
    "staging",
    "production",
    "prod",
    "main",
    "master",
];

/// Words before a qualifier's object.
const PREPOSITIONS_BEFORE: &[&str] = &[
    "on", "in", "for", "with", "at", "from", "about", "after", "before", "during", "into", "of",
    "beim", "im", "mit", "bei", "für", "von", "nach", "über", "auf", "wegen", "an", "am", "aus",
];
/// Words that narrow how or why something happens.
const QUALIFIERS: &[&str] = &[
    "almost",
    "nearly",
    "partly",
    "partially",
    "sort",
    "kind",
    "maybe",
    "perhaps",
    "probably",
    "slowly",
    "slower",
    "faster",
    "quickly",
    "usual",
    "unusual",
    "normally",
    "loudly",
    "silently",
    "fast",
    "slow",
    "properly",
    "correctly",
    "successfully",
    "cleanly",
    "badly",
    "big",
    "small",
    "large",
    "huge",
    "because",
    "due",
    "since",
    "unexpectedly",
    "suddenly",
    "occasionally",
    "beinahe",
    "teilweise",
    "vielleicht",
    "wahrscheinlich",
    "langsam",
    "langsamer",
    "schneller",
    "gelegentlich",
    "wegen",
    "weil",
    "da",
    "plötzlich",
    "unerwartet",
    "erfolgreich",
    "sauber",
    "richtig",
    "falsch",
    "groß",
    "klein",
];

/// The second guard (v13): a word the labels call filler, which code sees
/// bears meaning, makes the request one that can't be watched: a number
/// or a word for one, a time or date word, a word for how often or which,
/// a name or a part of one, a condition's cue word, a negation, a unit or
/// a word for code work.
fn meaning_bearing(ask: &Ask<'_>, r: &Reading, i: usize) -> bool {
    let w = &r.words;
    let word = w[i].as_str();
    let unplaced = |k: usize| r.slots.get(k) == Some(&None);
    // A qualifier, not a small word: two words code can't place side by
    // side ("slower than usual" is one; "real users"), or the object of a
    // preposition ("on my laptop", "about secrets", "beim Upload").
    let phrase = (i > 0 && unplaced(i - 1)) || unplaced(i + 1);
    let object = (1..=3).any(|k| {
        i >= k
            && is(PREPOSITIONS_BEFORE, &w[i - k])
            && (i - k + 1..i).all(|j| {
                is(
                    &["the", "a", "an", "my", "der", "die", "das", "den", "dem"],
                    &w[j],
                )
            })
    });
    let stem = |stems: &[&str]| stems.iter().any(|s| word.starts_with(s));
    let name_part = ask.candidates.iter().any(|c| {
        super::every_name(c)
            .flat_map(|n| tokens(n).into_iter().chain(resolve::spaced(n)))
            .any(|t| t == word)
    });
    phrase
        || object
        || is(QUALIFIERS, word)
        || word.chars().any(|c| c.is_ascii_digit())
        || number(word).is_some()
        || is(COUNT_WORDS, word)
        || is(TIME_WORDS, word)
        || is(SCOPE_WORDS, word)
        || is(CONTENT_WORDS, word)
        || stem(&[
            "test", "deploy", "merge", "push", "build", "releas", "kosten", "datei",
        ])
        || name_part
        || !cue_conditions(word).is_empty()
        || is(COND_EXTRA, word)
        || is(NEGATION_WORDS, word)
        || word.starts_with("kein")
        || (word == "t" && i > 0 && is(NOT_STEMS, &w[i - 1]))
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

/// The labelling call's guided schema: exactly `n` slots.
#[must_use]
pub fn label_shape(n: usize) -> Value {
    let slots: Vec<&str> = SLOTS.iter().map(|(s, _)| *s).collect();
    json!({ "type": "object", "name": "WatchWordSlots", "properties": [
        { "name": "slots", "schema": { "type": "array", "min": n, "max": n, "items": {
            "type": "enum", "name": "WatchWordSlots_slot", "values": slots } } },
    ] })
}

fn label_request(ask: &Ask<'_>, r: &Reading, left: &[usize], note: &str, attempt: u32) -> Request {
    let numbered = |idx: &mut dyn Iterator<Item = usize>| {
        idx.map(|i| format!("{} {}", i + 1, r.words[i]))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let all = numbered(&mut (0..r.words.len()));
    let asked = numbered(&mut left.iter().copied());
    let n = left.len();
    Request {
        instructions: LABEL_INSTRUCTIONS.to_owned(),
        prompt: format!(
            "WORDS (numbered; names are data written by agents): {all}\n\nREQUEST (from the developer): {}\n\nLABEL ({n} words, in this order): {asked}\n\n{TASK} Give exactly {n} slots.{note}",
            ask.request
        ),
        shape: Some(label_shape(n)),
        max_tokens: u32::try_from(16 + n * 8).unwrap_or(320),
        decoding: Decoding::for_attempt(&format!("watch-label:{}", ask.request), attempt),
    }
}

/// The model's slots, when there are exactly `n` and each is one of the
/// eight; else what is wrong (never padded, never cut).
fn slots_of(text: &str, finish: Finish, n: usize) -> Result<Vec<Slot>, String> {
    if finish == Finish::Length {
        return Err("shape: cut at the token limit".to_owned());
    }
    let got: Value = serde_json::from_str(text).map_err(|_| "not JSON".to_owned())?;
    let list = got["slots"]
        .as_array()
        .ok_or_else(|| "slots: missing".to_owned())?;
    if list.len() != n {
        return Err(format!(
            "slots: {} given for {n} words; give exactly one slot per LABEL word",
            list.len()
        ));
    }
    list.iter()
        .map(|v| {
            SLOTS
                .iter()
                .find(|(k, _)| Some(*k) == v.as_str())
                .map(|(_, s)| *s)
                .ok_or_else(|| "slot: not one of its choices".to_owned())
        })
        .collect()
}
