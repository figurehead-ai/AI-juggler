//! AJ-E3830 (0.5.2, the coordinator's and maintainer's call): **Bot
//! pre-fills the Watches form; the user saves.** Each field is filled only
//! with what code verified, from `draft::draft_traced`'s own reading and
//! the model's labels as code checks them; a field nothing verified stays
//! empty for the user. **The bar: never a wrong pre-filled field.**
//!
//! - **Every field verified** (a draft code accounts for word by word, the
//!   target its own, not the model's pick): the form filled in, and, with
//!   the user's Beta on, Bot's confirm card in plain words instead.
//! - **Otherwise each field on its own** (`fields`): the words code
//!   couldn't place ("live" words, after the labels' filler passed the
//!   second guard) spoil the fields they may touch:
//!
//!   | A live word's label | Spoils |
//!   |---|---|
//!   | other, none, filler that bears meaning | the target in its clause; the condition, lifetime and quiet hours |
//!   | exclusion | the target and the condition; lifetime or quiet hours in their clause |
//!   | a negation word (no label is asked) | the target in its clause, the condition; lifetime or quiet hours in their clause |
//!   | target (not one code verified) | the target in its clause |
//!   | condition, condition_param | the condition |
//!   | lifetime | the lifetime |
//!   | quiet_hours | quiet hours |
//!
//!   and any live word that says something of its own (the second guard's
//!   `cover::qualifies`) spoils every field whose words share its
//!   clause, whatever its label.
//! - **The target** is code's own resolution only: the one session or
//!   account named whole outside what is said back (the nearest's rule,
//!   never widened to a project, an App or any session), or, when code
//!   placed every word, the one target code resolves. Never the model's pick.
//! - **The condition** only where nothing can't-be-watched is said (code's
//!   net, a threshold, a refusal), every number is taken and the target
//!   offers it; otherwise the nearest is offered beside the cards, not
//!   chosen.
//! - **Lifetime** said and verified is filled; unsaid and nothing unread is
//!   the form's suggestion (ADR-0036's default, labelled so); anything
//!   unread leaves it for the user to pick.
//! - **Quiet hours**: on only as code read it; anything unread leaves the
//!   switch off and the form says to check it.
//!
//! Only the user's click adds a watch: nothing here writes one.

use serde::{Deserialize, Serialize};

use super::cover::{self, Reading, Slot};
use super::{
    Ask, Outcome, Trace, WatchDrafted, all_offered, conditions_of, cue_conditions, lifetime_for,
    masked, threshold, unsupported_in,
};
use crate::features::ai::engine::Stop;
use crate::features::ai::run::JobModel;
use crate::features::watches::commands::WatchDraftInfo;
use crate::features::watches::fail_closed;
use crate::features::watches::model::{
    Condition, LifetimeChoice, Matching, TargetKind, WatchTarget,
};
use crate::features::watches::resolve;

/// A field of the Watches form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum PrefillField {
    /// Watch what.
    Target,
    /// For what (the conditions and their numbers).
    Condition,
    /// Until.
    Lifetime,
    /// Even during quiet hours.
    QuietHours,
}

/// Why some of the form is left for the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum PrefillWhy {
    /// Every field verified.
    Verified,
    /// More than one matches the name: the picker asks which.
    Several,
    /// Code knows no condition covers what is asked: said back, with the
    /// nearest it can.
    CantWatch,
    /// Words code couldn't verify: said back; what they may touch is left.
    Unread,
    /// No condition said.
    NoCondition,
}

/// The Watches form as Bot pre-fills it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WatchPrefill {
    /// The fields as verified: no target or no condition where none was;
    /// the lifetime unsaid where it is the form's suggestion or left;
    /// quiet hours off unless verified on.
    pub draft: WatchDraftInfo,
    /// The fields left for the user (empty: every field verified).
    pub left: Vec<PrefillField>,
    /// Why.
    pub why: PrefillWhy,
    /// The request's own words Bot couldn't read or can't watch.
    #[ts(optional = nullable)]
    pub asked: Option<String>,
    /// The condition nearest to what was asked, offered beside the cards
    /// (never chosen) when the condition is left.
    #[ts(optional = nullable)]
    pub nearest: Option<String>,
    /// Bot offers its confirm card (every field verified by code, the
    /// form's own check passed, and the user's Beta on).
    pub card: bool,
    /// 0.5.2 G60: with the target left, what the request's names match
    /// (a project and a session in it both called "web"): the picker opens
    /// on them, the user picks one; none is ever filled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<WatchTarget>>", optional)]
    pub choices: Vec<WatchTarget>,
}

/// The pre-fill for `ask`, and the calls made; `None` when the words don't
/// ask to be told. `check` is the form's own check of a whole draft (its
/// problem's words, or none): only a draft that passes it may be offered
/// as a card.
pub fn prefill(
    model: &dyn JobModel,
    ask: &Ask<'_>,
    stop: &Stop,
    check: &dyn Fn(&WatchDraftInfo) -> Option<String>,
) -> Option<(WatchPrefill, u32)> {
    let (out, calls, trace) = super::draft_traced(model, ask, stop, &|_| None);
    let mut got = from_trace(ask, out, &trace)?;
    got.card = got.left.is_empty() && check(&got.draft).is_none();
    Some((got, calls))
}

/// The pre-fill from a draft's outcome and its trace (`card` false: the
/// caller decides it). `None` for a request that isn't a watch request.
#[must_use]
pub fn from_trace(ask: &Ask<'_>, out: Outcome, trace: &Trace) -> Option<WatchPrefill> {
    let r = trace.reading.as_ref()?;
    let ask = &Ask {
        request: &trace.request,
        candidates: ask.candidates,
        midnight_ms: ask.midnight_ms,
        now_ms: ask.now_ms,
    };
    let drafted = match out {
        Outcome::NotAWatch => return None,
        Outcome::Drafted(d) => d,
    };
    let done = |draft: WatchDraftInfo, left: Vec<PrefillField>, why| WatchPrefill {
        draft,
        left,
        why,
        asked: None,
        nearest: None,
        card: false,
        choices: Vec::new(),
    };
    let mut got = match drafted {
        WatchDrafted::Filled { mut draft } if trace.picked => {
            draft.targets.clear();
            Some(done(draft, vec![PrefillField::Target], PrefillWhy::Several))
        }
        WatchDrafted::Filled { mut draft } => {
            if draft.targets.iter().all(|t| target_said(ask, r, t)) {
                Some(done(draft, Vec::new(), PrefillWhy::Verified))
            } else {
                // A name its kind doesn't settle: the picker asks.
                draft.targets.clear();
                Some(done(draft, vec![PrefillField::Target], PrefillWhy::Several))
            }
        }
        WatchDrafted::Pick { draft } => {
            Some(done(draft, vec![PrefillField::Target], PrefillWhy::Several))
        }
        WatchDrafted::CantWatch { asked, draft } => {
            Some(fields(ask, r, trace, Some(asked), draft.as_ref()))
        }
        WatchDrafted::Form => Some(fields(ask, r, trace, None, None)),
        // A reminder isn't drafted here (Bot's own path reads it first).
        WatchDrafted::Remind { .. } | WatchDrafted::Prefilled { .. } => None,
    }?;
    // 0.5.2 G60: the target left, the picker opens on what the names
    // match (RW1 R31: "tell me when web finishes" with the project web and
    // a session in it left an empty picker to search).
    if got.left.contains(&PrefillField::Target) {
        got.choices = choices_of(ask, r, trace);
    }
    Some(got)
}

/// 0.5.2 G60: the targets the request's names match, for the picker to
/// offer first when the target is left (never filled): what the clause
/// that asks names, every namesake said whole, and the sessions in a
/// project or App named; without what it excludes, and nothing named
/// after a negation (that is what it leaves out). "Any session" is never
/// one.
fn choices_of(ask: &Ask<'_>, r: &Reading, trace: &Trace) -> Vec<WatchTarget> {
    let said = match trace.negated {
        Some(n) => r.words.get(..n).map(|w| w.join(" ")).unwrap_or_default(),
        None => fail_closed::watch_clause(ask).unwrap_or_else(|| ask.request.to_owned()),
    };
    let found = resolve::resolve(&said, ask.candidates);
    // Every target whose own name is said whole as well: the resolver
    // keeps one of namesakes ("web" the session's folder name and "web"
    // the project), the user may mean either.
    let words = match trace.negated {
        Some(n) => r.words.get(..n).unwrap_or_default(),
        None => &r.words[..],
    };
    let whole = (0..ask.candidates.len()).filter(|&i| {
        ask.candidates.get(i).is_some_and(|c| {
            std::iter::once(c.name.as_str())
                .chain(c.aliases.iter().map(String::as_str))
                .filter(|n| n.chars().count() >= 2)
                .any(|n| !said_at(words, n).is_empty())
        })
    });
    let mut out: Vec<WatchTarget> = Vec::new();
    let named: Vec<usize> = found
        .candidates
        .iter()
        .chain(&found.picker)
        .copied()
        .chain(whole)
        .collect();
    for i in named {
        if found.excluded.contains(&i) {
            continue;
        }
        if let Some(c) = ask.candidates.get(i)
            && c.target != WatchTarget::Any
            && !out.contains(&c.target)
        {
            out.push(c.target.clone());
        }
    }
    // A project or an App named: the sessions in it too (the open one in
    // "web" is likely what "when web finishes" means; the picker puts open
    // sessions first).
    let groups: Vec<(TargetKind, String)> = out
        .iter()
        .filter_map(|t| {
            let c = ask.candidates.iter().find(|c| &c.target == t)?;
            matches!(t.kind(), TargetKind::Project | TargetKind::App)
                .then(|| (t.kind(), c.name.clone()))
        })
        .collect();
    for (i, c) in ask.candidates.iter().enumerate() {
        let inside = groups.iter().any(|(kind, name)| {
            let owner = match kind {
                TargetKind::Project => c.project.as_deref(),
                _ => c.app.as_deref(),
            };
            owner == Some(name.as_str())
        });
        if c.target.kind() == TargetKind::Session
            && inside
            && !found.excluded.contains(&i)
            && !out.contains(&c.target)
        {
            out.push(c.target.clone());
        }
    }
    out.truncate(CHOICES_MAX);
    out
}

/// The most targets the picker is opened on.
const CHOICES_MAX: usize = 6;

// ============================================================================ the words code couldn't place

/// What a live word's label says it is, as the pre-fill reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Live {
    /// No label, "other", or filler that bears meaning.
    Unknown,
    /// The model's "exclusion".
    Excludes,
    /// A word that leaves something out, with no label asked (the
    /// negation guard stops before the labels).
    Negation,
    /// A name word code didn't verify.
    Target,
    /// A condition's word, or its number.
    Condition,
    /// How long.
    Lifetime,
    /// Quiet hours.
    Quiet,
}

/// The words code couldn't place and the labels didn't pass as plain
/// filler: each word's index and what it is.
fn live_words(ask: &Ask<'_>, r: &Reading, labels: Option<&[Slot]>) -> Vec<(usize, Live)> {
    let left: Vec<usize> = (0..r.words.len())
        .filter(|&i| r.slots[i].is_none())
        .collect();
    left.iter()
        .enumerate()
        .filter_map(|(k, &i)| {
            let live = match labels.and_then(|l| l.get(k).copied()) {
                Some(Slot::Filler) if cover::plain_filler(ask, r, i) => return None,
                Some(Slot::Exclusion) => Live::Excludes,
                Some(Slot::Target) => Live::Target,
                Some(Slot::Condition | Slot::ConditionParam) => Live::Condition,
                Some(Slot::Lifetime) => Live::Lifetime,
                Some(Slot::QuietHours) => Live::Quiet,
                None if cover::leaves_out(r, i) => Live::Negation,
                // Filler that bears meaning, "other", or no label.
                Some(Slot::Filler | Slot::Other) | None => Live::Unknown,
            };
            Some((i, live))
        })
        .collect()
}

/// Each word's clause: a new one after `.`, `,`, `;`, `:`, `!`, `?` or a
/// dash between words.
fn clauses(request: &str) -> Vec<usize> {
    let spans = cover::token_spans(request);
    let mut at = 0;
    let mut out = Vec::with_capacity(spans.len());
    for (k, span) in spans.iter().enumerate() {
        if k > 0 {
            let gap = &request[spans[k - 1].2..span.1];
            if gap.contains(['.', ',', ';', ':', '!', '?', '—', '–']) || gap.contains(" - ") {
                at += 1;
            }
        }
        out.push(at);
    }
    out
}

/// Which fields the live words spoil.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per form field, each spoiled on its own"
)]
struct Spoiled {
    target: bool,
    condition: bool,
    lifetime: bool,
    quiet: bool,
}

fn spoiled(ask: &Ask<'_>, r: &Reading, request: &str, live: &[(usize, Live)]) -> Spoiled {
    let clause = clauses(request);
    let clause_of = |i: usize| clause.get(i).copied().unwrap_or(usize::MAX);
    // The clauses holding a field's own words (code placed them).
    let held = |slots: &[Slot]| -> Vec<usize> {
        (0..r.words.len())
            .filter(|&i| r.slots[i].is_some_and(|s| slots.contains(&s)))
            .map(clause_of)
            .collect()
    };
    let target_c = held(&[Slot::Target]);
    let condition_c = held(&[Slot::Condition, Slot::ConditionParam]);
    let lifetime_c = held(&[Slot::Lifetime]);
    let quiet_c = held(&[Slot::QuietHours]);
    let mut s = Spoiled::default();
    for &(i, live) in live {
        let c = clause_of(i);
        let near = |of: &[usize]| of.contains(&c);
        // Any word code couldn't read in a lifetime or quiet-hours phrase's
        // clause may change it, and a word for a time or the night may be
        // one, whatever the labels say.
        s.lifetime |= near(&lifetime_c) || cover::time_word(&r.words[i]);
        s.quiet |= near(&quiet_c) || cover::time_word(&r.words[i]);
        // A word that says something of its own changes the condition in
        // its clause, and the target it stands before or qualifies.
        if cover::qualifies(r, i) {
            s.target |= modifies_target(r, i);
            s.condition |= near(&condition_c);
        }
        // A word of another name beside the target's may be naming that
        // one ("Claude in the Dock" is no account "Dock").
        s.target |= near(&target_c) && name_part(ask, &r.words[i]);
        match live {
            Live::Unknown => {
                s.target |= near(&target_c);
                s.condition = true;
                s.lifetime = true;
                s.quiet = true;
            }
            Live::Excludes | Live::Negation => {
                // The model's exclusion may be about the target wherever
                // it stands; a negation code read, about a name in its
                // clause or any name after it ("…, es sei denn, es ist die
                // X-Sitzung": X is what it leaves out).
                let after = (i + 1..r.words.len()).any(|k| r.slots[k] == Some(Slot::Target));
                s.target |= live == Live::Excludes || near(&target_c) || after;
                s.condition = true;
            }
            Live::Target => s.target |= near(&target_c),
            Live::Condition => s.condition = true,
            Live::Lifetime => s.lifetime = true,
            Live::Quiet => s.quiet = true,
        }
    }
    s
}

/// `word` is a word of some target's names (its own, its App's, its
/// project's).
fn name_part(ask: &Ask<'_>, word: &str) -> bool {
    ask.candidates.iter().any(|c| {
        cover::every_name(c)
            .flat_map(|n| cover::tokens(n).into_iter().chain(resolve::spaced(n)))
            .any(|t| t == word)
    })
}

/// Word `i` stands just before a name ("the staging api session") or is
/// the object of a preposition right after one ("api in staging").
fn modifies_target(r: &Reading, i: usize) -> bool {
    const ARTICLES: &[&str] = &[
        "the", "a", "an", "my", "der", "die", "das", "den", "dem", "des",
    ];
    const PREPS: &[&str] = &[
        "in", "on", "at", "of", "for", "from", "im", "am", "auf", "bei", "von", "aus", "für",
    ];
    let is_target = |k: usize| r.slots.get(k) == Some(&Some(Slot::Target));
    let mut next = i + 1;
    while r
        .words
        .get(next)
        .is_some_and(|w| ARTICLES.contains(&w.as_str()))
    {
        next += 1;
    }
    let mut back = i;
    while back > 0 && ARTICLES.contains(&r.words[back - 1].as_str()) {
        back -= 1;
    }
    let object = back >= 2 && PREPS.contains(&r.words[back - 1].as_str()) && is_target(back - 2);
    is_target(next) || object
}

// ============================================================================ the fields

/// The form's fields for a request code didn't draft whole: each field
/// filled only as verified (see the module).
fn fields(
    ask: &Ask<'_>,
    r: &Reading,
    trace: &Trace,
    asked: Option<String>,
    told: Option<&WatchDraftInfo>,
) -> WatchPrefill {
    let live = live_words(ask, r, trace.labels.as_deref());
    let s = spoiled(ask, r, &trace.request, &live);
    let every_word = live.is_empty() && trace.negated.is_none();
    // The target: code's own, never the model's pick: what the told-back
    // draft names, else the one named outside the words code couldn't read.
    let live_idx: Vec<usize> = live.iter().map(|(i, _)| *i).collect();
    // A name after the negation guard's word is what it leaves out.
    let after_negation = trace
        .negated
        .is_some_and(|n| (n..r.words.len()).any(|k| r.slots[k] == Some(Slot::Target)));
    let target = if trace.picked || s.target || after_negation {
        None
    } else if let Some(one) = told.and_then(|d| match d.targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }) {
        Some(one)
    } else if every_word {
        code_target(ask)
    } else {
        cover::nearest(ask, r, span_of(&live_idx), (0, 0))
            .targets
            .first()
            .cloned()
    }
    .filter(|t| target_said(ask, r, t));
    // What the target offers; with none filled, what every target the
    // request's names resolve to offers (else any's).
    let offered: Vec<String> = match &target {
        Some(t) => ask
            .candidates
            .iter()
            .find(|c| &c.target == t)
            .map_or_else(Vec::new, |c| c.offered.clone()),
        None => named_offer(ask),
    };
    // The condition: never where code knows it can't be watched (a draft
    // told back with every word placed), never past a negation.
    let may_read = trace.negated.is_none() && (told.is_none() || !every_word) && !s.condition;
    let conditions = if may_read {
        conditions_said(ask, r, &offered)
    } else {
        Vec::new()
    };
    let lifetime = if s.lifetime {
        None
    } else {
        Some(match lifetime_for(ask) {
            LifetimeChoice::UntilEnds
                if target
                    .as_ref()
                    .is_some_and(|t| t.kind() != TargetKind::Session) =>
            {
                LifetimeChoice::Unsaid
            }
            other => other,
        })
    };
    let quiet = (!s.quiet).then_some(r.quiet_on);
    // The told-back draft's condition, else (0.5.2 G60, RW1 R32: "tell me
    // when web finishes unless it's after dark" tagged nothing) the one
    // the request's own cue words name, before any negation: tagged
    // beside the cards, never chosen.
    let nearest = told
        .and_then(|d| d.conditions.first())
        .map(|c| c.code.clone())
        .or_else(|| cue_nearest(ask, r, trace))
        .filter(|c| conditions.is_empty() && offered.contains(c));
    let mut left = Vec::new();
    if target.is_none() {
        left.push(PrefillField::Target);
    }
    if conditions.is_empty() {
        left.push(PrefillField::Condition);
    }
    if lifetime.is_none() {
        left.push(PrefillField::Lifetime);
    }
    if quiet.is_none() {
        left.push(PrefillField::QuietHours);
    }
    let why = match (told, every_word) {
        (Some(_), true) => PrefillWhy::CantWatch,
        (None, true) if conditions.is_empty() => PrefillWhy::NoCondition,
        _ if left.is_empty() => PrefillWhy::Verified,
        _ => PrefillWhy::Unread,
    };
    WatchPrefill {
        draft: WatchDraftInfo {
            targets: target.into_iter().collect(),
            matching: Matching::Each,
            conditions,
            lifetime: lifetime.unwrap_or(LifetimeChoice::Unsaid),
            even_in_quiet_hours: quiet.unwrap_or(false),
            chat_turn: None,
        },
        left,
        why,
        asked,
        nearest,
        card: false,
        choices: Vec::new(),
    }
}

/// The first condition the request's cue words name in the clause that
/// asks, its exclusions out and nothing after a negation; none when the
/// words name none.
fn cue_nearest(ask: &Ask<'_>, r: &Reading, trace: &Trace) -> Option<String> {
    let said = match trace.negated {
        Some(n) => r.words.get(..n).map(|w| w.join(" ")).unwrap_or_default(),
        None => fail_closed::watch_clause(ask).unwrap_or_else(|| ask.request.to_owned()),
    };
    let part = Ask {
        request: &said,
        candidates: ask.candidates,
        midnight_ms: ask.midnight_ms,
        now_ms: ask.now_ms,
    };
    let (kept, _) = resolve::split_exclusions(&masked(&part));
    cue_conditions(&kept.join(" ")).into_iter().next()
}

const SESSION_WORDS: &[&str] = &[
    "session",
    "sessions",
    "sitzung",
    "sitzungen",
    "agent",
    "agents",
    "chat",
    "chats",
    "lauf",
    "job",
];
const PROJECT_WORDS: &[&str] = &[
    "project",
    "projects",
    "projekt",
    "projekte",
    "projekten",
    "repo",
];
const APP_WORDS: &[&str] = &["app", "apps", "application"];
const ACCOUNT_WORDS: &[&str] = &[
    "account",
    "accounts",
    "konto",
    "konten",
    "subscription",
    "abo",
    "plan",
];

/// Where `name` is said whole in the words (its words, spaced or dashed).
fn said_at(words: &[String], name: &str) -> Vec<(usize, usize)> {
    let forms = [
        cover::tokens(name),
        resolve::spaced(name),
        vec![resolve::spaced(name).join("-")],
    ];
    let mut out = Vec::new();
    for form in forms.iter().filter(|f| !f.is_empty()) {
        for a in 0..words.len() {
            if words.len() >= a + form.len() && words[a..a + form.len()] == form[..] {
                out.push((a, a + form.len()));
            }
        }
    }
    out
}

/// A target's own name is said whole, and nothing else could be meant by
/// it: a project or an App only with its kind word beside the name ("the
/// shop project", "im Projekt shop"; its sessions are often called by
/// it); a session or an account whose name another kind shares, only
/// with its own kind word beside it. Any session as code read it.
fn target_said(ask: &Ask<'_>, r: &Reading, t: &WatchTarget) -> bool {
    if t.kind() == TargetKind::Any {
        return true;
    }
    let Some(c) = ask.candidates.iter().find(|c| &c.target == t) else {
        return false;
    };
    let own: Vec<&str> = std::iter::once(c.name.as_str())
        .chain(c.aliases.iter().map(String::as_str))
        .filter(|n| n.chars().count() >= 2)
        .collect();
    let at: Vec<(usize, usize)> = own.iter().flat_map(|n| said_at(&r.words, n)).collect();
    if at.is_empty() {
        return false;
    }
    let beside = |kinds: &[&str]| {
        at.iter().any(|&(a, b)| {
            (a.saturating_sub(2)..a)
                .chain(b..(b + 2).min(r.words.len()))
                .any(|k| kinds.contains(&r.words[k].as_str()))
        })
    };
    let spaced: Vec<Vec<String>> = own.iter().map(|n| resolve::spaced(n)).collect();
    let shared = ask.candidates.iter().any(|o| {
        o.target.kind() != t.kind()
            && std::iter::once(o.name.as_str())
                .chain(o.aliases.iter().map(String::as_str))
                .any(|n| spaced.contains(&resolve::spaced(n)))
    });
    match t.kind() {
        TargetKind::Project => beside(PROJECT_WORDS),
        TargetKind::App => beside(APP_WORDS),
        TargetKind::Session => !shared || beside(SESSION_WORDS),
        TargetKind::Account => !shared || beside(ACCOUNT_WORDS),
        TargetKind::Pool | TargetKind::Any => false,
    }
}

/// What every target the request's names resolve to offers (each one's
/// offer, intersected); with no name, what any session offers.
fn named_offer(ask: &Ask<'_>) -> Vec<String> {
    let clause = fail_closed::watch_clause(ask).unwrap_or_else(|| ask.request.to_owned());
    let found = resolve::resolve(&clause, ask.candidates);
    let mut named = found
        .candidates
        .iter()
        .chain(&found.picker)
        .map(|&i| &ask.candidates[i]);
    let Some(first) = named.next() else {
        return all_offered(ask.candidates);
    };
    let mut out = first.offered.clone();
    for c in named {
        out.retain(|o| c.offered.contains(o));
    }
    out
}

/// The words from the first index to the last (exclusive end).
fn span_of(idx: &[usize]) -> (usize, usize) {
    match (idx.iter().min(), idx.iter().max()) {
        (Some(&a), Some(&b)) => (a, b + 1),
        _ => (0, 0),
    }
}

/// The one target code resolves when it placed every word: the clause
/// that asks, exclusions first (never widened past one), else none.
fn code_target(ask: &Ask<'_>) -> Option<WatchTarget> {
    let (_, spans) = resolve::split_exclusions(ask.request);
    if !spans.is_empty() {
        return None;
    }
    let clause = fail_closed::watch_clause(ask).unwrap_or_else(|| ask.request.to_owned());
    let found = resolve::resolve(&clause, ask.candidates);
    match (found.candidates.as_slice(), found.picker.is_empty()) {
        ([one], true) => Some(ask.candidates[*one].target.clone()),
        ([], true) if found.any => Some(WatchTarget::Any),
        _ => None,
    }
}

/// The conditions the request's cue words say, with code's numbers, when
/// nothing in it can't be watched, every number is one a condition takes,
/// each needed number is said, and the target offers each; else none.
fn conditions_said(ask: &Ask<'_>, r: &Reading, offered: &[String]) -> Vec<Condition> {
    if !r.slots.contains(&Some(Slot::Condition)) {
        return Vec::new();
    }
    if fail_closed::refused(ask).is_some() || threshold(&masked(ask)).is_some() {
        return Vec::new();
    }
    let clause = fail_closed::watch_clause(ask);
    let part = Ask {
        request: clause.as_deref().unwrap_or(ask.request),
        candidates: ask.candidates,
        midnight_ms: ask.midnight_ms,
        now_ms: ask.now_ms,
    };
    let (kept, spans) = resolve::split_exclusions(&masked(&part));
    if !spans.is_empty() {
        return Vec::new();
    }
    let read = kept.join(" ");
    let codes = cue_conditions(&read);
    if codes.is_empty() || unsupported_in(&read, &codes).is_some() {
        return Vec::new();
    }
    if !r
        .params
        .iter()
        .all(|&(_, _, unit, _)| codes.iter().any(|c| unit.taken_by(c)))
    {
        return Vec::new();
    }
    let conditions = conditions_of(&masked(&part), &codes);
    let needs_number = |c: &Condition| {
        c.param.is_none() && ["idle_for", "runway_below"].contains(&c.code.as_str())
    };
    if conditions.len() != codes.len()
        || conditions.iter().any(needs_number)
        || !conditions.iter().all(|c| offered.contains(&c.code))
    {
        return Vec::new();
    }
    conditions
}

#[cfg(test)]
#[path = "prefill_tests.rs"]
mod tests;
