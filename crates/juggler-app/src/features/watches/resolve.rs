//! AJ-E3830 (D-149's amendment): the one resolver from a watch request's
//! words to its target candidates, by code, before any model is asked.
//! Harness v7's `watch3.resolve` is the same rules.
//!
//! - **Exclusions first:** "ignore X", "except X", "not the X one",
//!   "other than X", "außer X": what they name (and the sessions of an
//!   excluded project or App) is no candidate.
//! - **Names:** a candidate's name or aliases said whole (or with its
//!   dashes as spaces) is a full match; else the count of its name's
//!   longer words the request says.
//! - **Owners:** "anything in X", "a session in the X project": the
//!   project or App named, not one of its sessions.
//! - **Kind words** ("project", "session", "account", "App") keep that
//!   kind among full matches; **any words** give "any" when nothing is
//!   named.
//! - **Shared names:** twins the request's other words (a project or App
//!   named) settle are settled; else the form's picker asks.
//!
//! The name lookup is the one place gyrus's `seed` (an exact alias
//! match) takes over once it lands: [`full_match`] is that seam.

use super::draft::{Candidate, words};
use super::model::TargetKind;

/// What code found for the request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolved {
    /// The candidates, best first (indexes into the request's list).
    pub candidates: Vec<usize>,
    /// What the request excluded.
    pub excluded: Vec<usize>,
    /// The request says "any", "every", "all".
    pub any: bool,
    /// Twins nothing settles: the picker asks which.
    pub picker: Vec<usize>,
}

const EXCLUDE: &[&str] = &[
    "ignore",
    "ignoring",
    "except",
    "excluding",
    "apart from",
    "other than",
    "but not",
    "not the",
    "not for",
    "never the",
    "skip",
    "skipping",
    "leave out",
    "außer",
    "ausgenommen",
    "ohne",
    "nicht die",
    "nicht das",
    "nicht den",
    "nicht der",
    "ignoriere",
];
const STOP: &[&str] = &["and", "but", "or", "und", "aber", "oder"];
const ANY: &[&str] = &[
    "any",
    "every",
    "all",
    "anything",
    "anyone",
    "alle",
    "jede",
    "jeder",
    "jedes",
    "irgendeine",
    "irgendwas",
];

fn holds(said: &[String], needle: &[String]) -> bool {
    !needle.is_empty() && said.windows(needle.len()).any(|w| w == needle)
}

fn spaced(name: &str) -> Vec<String> {
    words(&name.replace(['-', '_', '.', '/'], " "))
}

/// The seam gyrus's `seed` takes over: `name` said whole in `said`.
#[must_use]
pub fn full_match(said: &[String], name: &str) -> bool {
    holds(said, &words(name)) || holds(said, &spaced(name))
}

/// The request's words with every exclusion clause taken out, and the
/// clauses themselves.
#[must_use]
pub fn split_exclusions(request: &str) -> (Vec<String>, Vec<Vec<String>>) {
    // A clause ends at its punctuation: "except aspen, tell me if it fails"
    // excludes aspen, and the rest is what to watch.
    let mut kept = Vec::new();
    let mut spans = Vec::new();
    for clause in request.split([',', ';', '.', '!', '?']) {
        let (k, s) = split_clause(clause);
        kept.extend(k);
        spans.extend(s);
    }
    (kept, spans)
}

fn split_clause(clause: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let w = words(clause);
    let mut kept = Vec::new();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < w.len() {
        let hit = EXCLUDE
            .iter()
            .map(|p| words(p))
            .find(|p| w.len() >= i + p.len() && w[i..i + p.len()] == p[..]);
        if let Some(p) = hit {
            let mut j = i + p.len();
            let start = j;
            while j < w.len() && !STOP.contains(&w[j].as_str()) && j - start < 8 {
                j += 1;
            }
            spans.push(w[start..j].to_vec());
            i = j;
        } else {
            kept.push(w[i].clone());
            i += 1;
        }
    }
    (kept, spans)
}

fn names(c: &Candidate) -> impl Iterator<Item = &str> {
    std::iter::once(c.name.as_str()).chain(c.aliases.iter().map(String::as_str))
}

fn score(said: &[String], c: &Candidate) -> u32 {
    if names(c).any(|n| full_match(said, n)) {
        return 100 + u32::try_from(words(&c.name).len()).unwrap_or(0);
    }
    let parts: Vec<String> = spaced(&c.name)
        .into_iter()
        .filter(|p| p.chars().count() >= 4)
        .collect();
    u32::try_from(parts.iter().filter(|p| said.contains(p)).count()).unwrap_or(0)
}

fn owner_said(said: &[String], c: &Candidate) -> bool {
    [c.project.as_deref(), c.app.as_deref()]
        .into_iter()
        .flatten()
        .any(|o| full_match(said, o))
}

fn owner_of(all: &[Candidate], c: &Candidate, said: &[String]) -> Option<usize> {
    all.iter().position(|o| {
        let named = match o.target.kind() {
            TargetKind::Project => c.project.as_deref() == Some(o.name.as_str()),
            TargetKind::App => c.app.as_deref() == Some(o.name.as_str()),
            _ => false,
        };
        named && full_match(said, &o.name)
    })
}

fn kinds_said(said: &[String]) -> Vec<TargetKind> {
    let has = |ws: &[&str]| said.iter().any(|w| ws.contains(&w.as_str()));
    let mut out = Vec::new();
    if has(&["project", "projects", "projekt", "projekte"]) {
        out.push(TargetKind::Project);
    }
    if has(&[
        "session",
        "sessions",
        "sitzung",
        "sitzungen",
        "agent",
        "chat",
    ]) {
        out.push(TargetKind::Session);
    }
    if has(&[
        "account",
        "accounts",
        "konto",
        "konten",
        "subscription",
        "abo",
    ]) {
        out.push(TargetKind::Account);
    }
    if has(&["app", "apps", "application"]) {
        out.push(TargetKind::App);
    }
    out
}

/// See the module.
#[must_use]
pub fn resolve(request: &str, all: &[Candidate]) -> Resolved {
    let (said, spans) = split_exclusions(request);
    let excluded = excluded_by(all, &spans);
    let live: Vec<usize> = (0..all.len()).filter(|i| !excluded.contains(i)).collect();
    let any = said.iter().any(|w| ANY.contains(&w.as_str()));
    let mut cands = named(all, &live, &said, any);
    if cands.is_empty() {
        cands = owners_named(all, &live, &said, &excluded);
    }
    let picker = settle_twins(all, &live, &said, &mut cands);
    cands.truncate(6);
    Resolved {
        candidates: cands,
        excluded,
        any,
        picker,
    }
}

/// What the exclusion clauses name, and the sessions of an excluded
/// project or App.
fn excluded_by(all: &[Candidate], spans: &[Vec<String>]) -> Vec<usize> {
    let mut excluded: Vec<usize> = (0..all.len())
        .filter(|&i| {
            spans
                .iter()
                .any(|s| names(&all[i]).any(|n| full_match(s, n)))
        })
        .collect();
    let owners_out: Vec<String> = excluded.iter().map(|&i| all[i].name.clone()).collect();
    for (i, c) in all.iter().enumerate() {
        let gone = [c.project.as_deref(), c.app.as_deref()]
            .into_iter()
            .flatten()
            .any(|o| owners_out.iter().any(|x| x == o));
        if c.target.kind() == TargetKind::Session && gone && !excluded.contains(&i) {
            excluded.push(i);
        }
    }
    excluded
}

/// The candidates the request names, full matches first (a kind word, or
/// "any" beside a name, keeps that kind); else the partial ones.
fn named(all: &[Candidate], live: &[usize], said: &[String], any: bool) -> Vec<usize> {
    let kinds = kinds_said(said);
    let ranked = |min: u32| {
        let mut out: Vec<(usize, u32)> = live
            .iter()
            .map(|&i| (i, score(said, &all[i])))
            .filter(|(_, s)| *s >= min)
            .collect();
        out.sort_by_key(|(i, s)| (std::cmp::Reverse(*s), *i));
        out.into_iter().map(|(i, _)| i).collect::<Vec<usize>>()
    };
    let full = ranked(100);
    if full.is_empty() {
        return ranked(1).into_iter().take(5).collect();
    }
    let keep: Vec<usize> = full
        .iter()
        .copied()
        .filter(|&i| {
            let kind = all[i].target.kind();
            if kinds.is_empty() {
                any && kind != TargetKind::Session
            } else {
                kinds.contains(&kind)
            }
        })
        .collect();
    if keep.is_empty() { full } else { keep }
}

/// "anything in X", "a session in the X project": the owners named.
fn owners_named(
    all: &[Candidate],
    live: &[usize],
    said: &[String],
    excluded: &[usize],
) -> Vec<usize> {
    let mut owners: Vec<usize> = Vec::new();
    for &i in live {
        if all[i].target.kind() == TargetKind::Session
            && owner_said(said, &all[i])
            && let Some(o) = owner_of(all, &all[i], said)
            && !owners.contains(&o)
            && !excluded.contains(&o)
        {
            owners.push(o);
        }
    }
    owners
}

/// Twins of the top candidate: settled by an owner the request names
/// (that one first), else the picker's list.
fn settle_twins(
    all: &[Candidate],
    live: &[usize],
    said: &[String],
    cands: &mut Vec<usize>,
) -> Vec<usize> {
    let Some(&top) = cands.first() else {
        return Vec::new();
    };
    let twins: Vec<usize> = live
        .iter()
        .copied()
        .filter(|&i| {
            all[i].target.kind() == all[top].target.kind()
                && words(&all[i].name) == words(&all[top].name)
        })
        .collect();
    match twins.len() {
        0 | 1 => {
            // The other twins were excluded: the one left is the target.
            if let Some(&one) = twins.first() {
                cands.retain(|i| *i == one || words(&all[*i].name) != words(&all[one].name));
            }
            Vec::new()
        }
        _ => {
            let settled: Vec<usize> = twins
                .iter()
                .copied()
                .filter(|&i| owner_said(said, &all[i]))
                .collect();
            if let [one] = settled[..] {
                cands.retain(|i| !twins.contains(i));
                cands.insert(0, one);
                Vec::new()
            } else {
                twins
            }
        }
    }
}
