//! AJ-E3830 (D-149 amendment 2): Bot's watch drafts fail closed. Code
//! accepts a draft only when the request parses into a shape it fully
//! understands; what it can't account for is told back as what can't be
//! watched, with the nearest option, before any model call (harness v9's
//! `watch5`, as measured):
//!
//! - a meaning-changing word outside a name (but, except, skipping,
//!   besides, without, other than, apart from, aside from, excluding,
//!   leave out, not the … one; außer, ohne, bis auf, nicht das …): "but
//!   only" / "but just", which narrow to what follows, are kept;
//! - a broad quantifier (all, every, everything, any, each; alle, jede,
//!   irgendein) beside a named entity, unless the entity is clearly the
//!   target ("of all my sessions, … checkout-api"; "anything in pier");
//! - a number in a unit no condition takes (tokens, cost, files, lines,
//!   commits), before or after its unit, in digits or words.
//!
//! [`watch_clause`] reads a request of several clauses by the one that
//! asks to be told.

use super::draft::{Ask, WatchDrafted, cue_conditions, draft_info, masked, threshold, words};
use super::model::{Condition, LifetimeChoice};
use super::resolve;

const MEANING: &[&str] = &[
    "but",
    "except",
    "skipping",
    "skip",
    "besides",
    "excluding",
    "without",
    "außer",
    "ohne",
];
const MEANING_PHRASES: &[&str] = &[
    "other than",
    "apart from",
    "aside from",
    "leave out",
    "leaving out",
    "bis auf",
    "nicht das",
    "nicht die",
    "nicht den",
    "nicht der",
];
const QUANTIFIERS: &[&str] = &[
    "all",
    "every",
    "everything",
    "any",
    "anything",
    "each",
    "alle",
    "allen",
    "jede",
    "jeder",
    "jedes",
    "irgendeine",
    "irgendein",
];
const UNITS: &[&str] = &[
    "tokens",
    "token",
    "cost",
    "costs",
    "spend",
    "spending",
    "money",
    "files",
    "lines",
    "commits",
    "tokenanzahl",
    "kosten",
    "dateien",
    "zeilen",
];
const PAST: &[&str] = &[
    "passes",
    "exceeds",
    "reaches",
    "hits",
    "tops",
    "crosses",
    "over",
    "above",
    "überschreitet",
    "übersteigt",
    "über",
];
const BIG: &[&str] = &[
    "one",
    "two",
    "three",
    "five",
    "ten",
    "twenty",
    "fifty",
    "hundred",
    "thousand",
    "million",
    "eine",
    "einer",
    "tausend",
    "million",
    "millionen",
];

/// The answer for a request code can't account for, `None` when it can.
#[must_use]
pub fn refused(ask: &Ask<'_>) -> Option<WatchDrafted> {
    let read = masked(ask);
    let w = words(&read);
    let conds = cue_conditions(&read);
    let near = |targetless: bool| {
        let code = conds.first().map_or("finishes", String::as_str).to_owned();
        let _ = targetless;
        Some(draft_info(
            Vec::new(),
            vec![Condition { code, param: None }],
            LifetimeChoice::Unsaid,
        ))
    };
    if let Some(i) = meaning_at(&w) {
        // Said back in the request's own words, names included.
        let raw = words(ask.request);
        let asked = meaning_at(&raw).map_or_else(
            || w[i..(i + 6).min(w.len())].join(" "),
            |j| raw[j..(j + 6).min(raw.len())].join(" "),
        );
        return Some(WatchDrafted::CantWatch {
            asked,
            draft: near(true),
        });
    }
    if let Some(said) = unit_first(&w) {
        let found = resolve::resolve(ask.request, ask.candidates);
        let target = (found.candidates.len() == 1)
            .then(|| ask.candidates[found.candidates[0]].target.clone());
        let code = if ["token", "cost", "money", "kosten"]
            .iter()
            .any(|u| said.contains(u))
        {
            "hits_limit"
        } else {
            "finishes"
        };
        return Some(WatchDrafted::CantWatch {
            asked: said,
            draft: Some(draft_info(
                target.into_iter().collect(),
                vec![Condition {
                    code: code.to_owned(),
                    param: None,
                }],
                LifetimeChoice::Unsaid,
            )),
        });
    }
    if threshold(&read).is_some() {
        return None; // the draft's own threshold answer, with its target (v8)
    }
    let found = resolve::resolve(ask.request, ask.candidates);
    let named = !found.candidates.is_empty() && read.contains(" _ ");
    if let Some(q) = w.iter().position(|x| QUANTIFIERS.contains(&x.as_str()))
        && named
        && !selection(&w, q)
        && !within(&read, &w[q])
    {
        return Some(WatchDrafted::CantWatch {
            asked: w[q..(q + 5).min(w.len())].join(" "),
            draft: near(true),
        });
    }
    None
}

/// Where a meaning-changing word or phrase starts.
fn meaning_at(w: &[String]) -> Option<usize> {
    (0..w.len()).find(|&i| {
        let next = w.get(i + 1).map_or("", String::as_str);
        (MEANING.contains(&w[i].as_str()) && !(w[i] == "but" && ["only", "just"].contains(&next)))
            || MEANING_PHRASES.iter().any(|p| {
                let p = words(p);
                w.len() >= i + p.len() && w[i..i + p.len()] == p[..]
            })
            || (w[i] == "not"
                && next == "the"
                && w.get(i + 2..(i + 5).min(w.len()))
                    .is_some_and(|r| r.iter().any(|x| x == "one")))
    })
}

/// "of all my sessions", "out of all", "von allen": a selection, not a scope.
fn selection(w: &[String], q: usize) -> bool {
    q > 0 && ["of", "among", "from", "von", "unter"].contains(&w[q - 1].as_str())
}

/// "anything in pier", "any session of the shop project": the quantifier
/// inside the named one (blanked `_` in `read`), which is then the target.
fn within(read: &str, quantifier: &str) -> bool {
    let Some(at) = read.find(quantifier) else {
        return false;
    };
    let after: String = read[at..]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(48)
        .collect();
    [
        " in _",
        " im _",
        " of _",
        " from _",
        " von _",
        " aus _",
        " inside _",
        " within _",
        " in the _",
        " of the _",
        " im projekt _",
        " in the project _",
    ]
    .iter()
    .any(|p| after.contains(p))
}

/// A unit no condition takes, then a verb of passing and a number ("the
/// token count … passes one million"): its words.
fn unit_first(w: &[String]) -> Option<String> {
    let u = w.iter().position(|x| UNITS.contains(&x.as_str()))?;
    let end = (u + 10).min(w.len());
    let v = (u + 1..end).find(|&i| PAST.contains(&w[i].as_str()))?;
    let n = (v + 1..(v + 4).min(w.len())).find(|&i| {
        w[i].chars().next().is_some_and(|c| c.is_ascii_digit()) || BIG.contains(&w[i].as_str())
    })?;
    let last = (n + 1..(n + 3).min(w.len()))
        .take_while(|&i| BIG.contains(&w[i].as_str()))
        .last()
        .unwrap_or(n);
    Some(w[u..=last].join(" "))
}

/// The clause that asks to be told, when the request has several: its
/// sentence, from the last comma before the ask when what is left still
/// names a condition. `None` when that is the whole request.
#[must_use]
pub fn watch_clause(ask: &Ask<'_>) -> Option<String> {
    const ASKS: &[&str] = &[
        "let me know",
        "tell me",
        "ping me",
        "notify me",
        "alert me",
        "warn me",
        "wake me",
        "keep an eye",
        "sag mir",
        "gib mir bescheid",
        "melde dich",
        "benachrichtige mich",
        "watch",
    ];
    let whole = ask
        .request
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .trim()
        .to_owned();
    for sentence in ask.request.split(['.', ';', '!', '?']) {
        let lower = sentence.to_lowercase();
        let Some(at) = ASKS.iter().filter_map(|a| lower.find(a)).min() else {
            continue;
        };
        let start = lower[..at].rfind(',').map_or(0, |c| c + 1);
        let clip = sentence[start..].trim().to_owned();
        let part = if start > 0 && !cue_conditions(&clip.to_lowercase()).is_empty() {
            clip
        } else {
            sentence.trim().to_owned()
        };
        return (part != whole).then_some(part);
    }
    None
}
