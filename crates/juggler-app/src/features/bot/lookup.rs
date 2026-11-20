//! The look-up (AJ-E3805): a question read by code, run over the records,
//! and the rows that answer it, each its own source. Pure: the same
//! question over the same records gives the same rows.
//!
//! - **The scope rides on the question** (D-136 point 3): with a chip,
//!   only that session, App, project, pool or Account is looked at.
//! - **A subject is matched, never guessed:** every word the question
//!   names must be in one record's names. Nothing holds them: "I don't
//!   know", with the nearest screen, never a nearby answer.
//! - **What isn't recorded is said so** (test times, test results, costs,
//!   authors), with the nearest record labelled as such.
//! - **States follow the facts:** each row carries its own; the window's
//!   lead counts them one clause per state.

use super::dto::{
    BotLooked, BotRowInfo, BotRowKind, BotScope, BotScopeKind, BotShortcut, BotState, BotTally,
    BotUnknown,
};
use super::question::{Reading, When};
use super::world::World;

/// The most rows an answer shows (ADR-0034 §6.2).
pub const SHOWN: usize = 8;

const DAY_MS: i64 = 86_400_000;

/// What a look-up found.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// The rows, at most [`SHOWN`].
    pub rows: Vec<BotRowInfo>,
    /// How many matched.
    pub total: u32,
    /// Every match counted by kind and state (the lead's numbers).
    pub tallies: Vec<BotTally>,
    /// Why Bot doesn't know, if it doesn't.
    pub unknown: Option<BotUnknown>,
    /// The nearest screen.
    pub shortcut: Option<BotShortcut>,
    /// What it looked at.
    pub looked: BotLooked,
}

fn in_scope(row: &BotRowInfo, scope: &BotScope) -> bool {
    match scope.kind {
        BotScopeKind::Session => row.kind == BotRowKind::Session && row.id == scope.id,
        BotScopeKind::App => {
            row.kind == BotRowKind::Session
                && (scope.sessions.contains(&row.id)
                    || row.app_id.as_deref() == Some(scope.id.as_str()))
        }
        BotScopeKind::Project => {
            row.kind != BotRowKind::Account && row.project_id.as_deref() == Some(scope.id.as_str())
        }
        BotScopeKind::Pool => {
            row.kind == BotRowKind::Task && row.pool_id.as_deref() == Some(scope.id.as_str())
        }
        BotScopeKind::Account => row.kind == BotRowKind::Account && row.id == scope.id,
    }
}

fn name_words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// A question's word against a name's: the same, or one the start of
/// the other when both are four letters or more ("login" and "logins").
fn word_matches(asked: &str, name: &str) -> bool {
    asked == name
        || (asked.chars().count() >= 4
            && name.chars().count() >= 4
            && (name.starts_with(asked) || asked.starts_with(name)))
}

/// The words of the names a row is known by ([`super::check::names_of`]).
fn names_of(row: &BotRowInfo) -> Vec<String> {
    super::check::names_of(row).flat_map(name_words).collect()
}

/// Whether the row holds every subject word in its names.
fn holds(row: &BotRowInfo, subject: &[String]) -> bool {
    let names = names_of(row);
    subject.iter().all(|asked| {
        let asked_words = name_words(asked);
        asked_words
            .iter()
            .all(|a| names.iter().any(|n| word_matches(a, n)))
    })
}

/// The window a time word names, as `[from, to)`.
fn window(when: When, now_ms: i64, offset_hours: i64) -> Option<(i64, i64)> {
    let offset = offset_hours * 3_600_000;
    let local = now_ms + offset;
    let midnight = local - local.rem_euclid(DAY_MS) - offset;
    match when {
        When::Any => None,
        When::Today => Some((midnight, i64::MAX)),
        When::Yesterday => Some((midnight - DAY_MS, midnight)),
        When::ThisWeek => {
            // 1970-01-01 was a Thursday: Monday is 3 days before.
            let days = (local.div_euclid(DAY_MS) + 3).rem_euclid(7);
            Some((midnight - days * DAY_MS, i64::MAX))
        }
    }
}

/// The order rows are shown in: what needs you first, then the most
/// recent.
const fn rank(state: Option<BotState>) -> u8 {
    match state {
        Some(BotState::AsksYou) => 0,
        Some(BotState::Stuck) => 1,
        Some(BotState::Limit) => 2,
        Some(BotState::Failed) => 3,
        Some(BotState::Blocked) => 4,
        Some(BotState::InReview) => 5,
        Some(BotState::Working) => 6,
        Some(BotState::Finished) => 7,
        Some(BotState::Done) => 8,
        Some(BotState::Ended) => 9,
        _ => 10,
    }
}

fn shortcut_for(reading: &Reading) -> BotShortcut {
    if reading.unknown == Some(BotUnknown::Costs) {
        BotShortcut::Usage
    } else if reading.kinds.accounts {
        BotShortcut::Runway
    } else if reading.states.iter().any(|s| s.level() >= 1) {
        BotShortcut::NeedsMe
    } else if reading.states.contains(&BotState::Working) {
        BotShortcut::Running
    } else {
        BotShortcut::FinishedToday
    }
}

fn looked(reach: &str, scope: Option<BotScope>) -> BotLooked {
    BotLooked {
        reach: reach.to_owned(),
        scope,
        memories: 0,
    }
}

/// The rows a question may be about: the kinds it names (sessions and
/// tasks when it names none), within the chip.
fn candidates<'w>(
    reading: &Reading,
    world: &'w World,
    scope: Option<&BotScope>,
) -> Vec<&'w BotRowInfo> {
    let kinds = reading.kinds;
    let mut pool: Vec<&BotRowInfo> = Vec::new();
    if kinds.accounts && !kinds.sessions && !kinds.tasks {
        pool.extend(&world.accounts);
    } else {
        if kinds.sessions || !kinds.any() {
            pool.extend(&world.sessions);
        }
        if kinds.tasks || !kinds.any() {
            pool.extend(&world.tasks);
        }
        if kinds.accounts || scope.is_some_and(|s| s.kind == BotScopeKind::Account) {
            pool.extend(&world.accounts);
        }
    }
    if let Some(scope) = scope {
        pool.retain(|row| in_scope(row, scope));
    }
    pool
}

/// The question over the records.
#[must_use]
pub fn look_up(reading: &Reading, world: &World, scope: Option<&BotScope>) -> Found {
    let accounts_only = reading.kinds.accounts && !reading.kinds.sessions && !reading.kinds.tasks;
    let pool = candidates(reading, world, scope);
    // An Account named by name answers "how much of Work did I use".
    let subject = &reading.subject;
    let mut matched: Vec<&BotRowInfo> = if subject.is_empty() {
        pool.clone()
    } else {
        let mut hit: Vec<&BotRowInfo> = pool
            .iter()
            .copied()
            .filter(|row| holds(row, subject))
            .collect();
        if hit.is_empty() && scope.is_none() && !accounts_only {
            hit = world
                .accounts
                .iter()
                .filter(|row| holds(row, subject))
                .collect();
        }
        hit
    };
    let reach_scope = scope.cloned();
    let reach = if reach_scope.is_some() {
        "scope"
    } else if accounts_only {
        "accounts"
    } else {
        "everything"
    };
    // The subject's own record, for "Looked at" and the nearest record.
    let subject_scope = (!subject.is_empty() && reach_scope.is_none())
        .then(|| subject_named(&matched, subject))
        .flatten();
    if let Some(unknown) = reading.unknown {
        matched.truncate(1);
        let nearest: Vec<BotRowInfo> = matched.into_iter().cloned().collect();
        let reach = if subject_scope.is_some() {
            "subject"
        } else {
            reach
        };
        return Found {
            total: u32::try_from(nearest.len()).unwrap_or(0),
            tallies: Vec::new(),
            rows: nearest,
            unknown: Some(unknown),
            shortcut: Some(shortcut_for(reading)),
            looked: looked(reach, subject_scope.or(reach_scope)),
        };
    }
    if !subject.is_empty() && matched.is_empty() {
        return Found {
            rows: Vec::new(),
            total: 0,
            tallies: Vec::new(),
            unknown: Some(BotUnknown::NothingFound),
            shortcut: Some(shortcut_for(reading)),
            looked: looked("searched", reach_scope),
        };
    }
    let answer = narrow(reading, world, scope, matched, accounts_only);
    let total = u32::try_from(answer.len()).unwrap_or(u32::MAX);
    let tallies = tally(&answer);
    let rows = answer.into_iter().take(SHOWN).cloned().collect();
    let reach = if subject_scope.is_some() {
        "subject"
    } else {
        reach
    };
    Found {
        rows,
        total,
        tallies,
        unknown: None,
        shortcut: None,
        looked: looked(reach, subject_scope.or(reach_scope)),
    }
}

/// Every matched row counted by kind and state, in first-seen order.
fn tally(answer: &[&BotRowInfo]) -> Vec<BotTally> {
    let mut out: Vec<BotTally> = Vec::new();
    for row in answer {
        match out
            .iter_mut()
            .find(|c| c.kind == row.kind && c.state == row.state)
        {
            Some(c) => c.n = c.n.saturating_add(1),
            None => out.push(BotTally {
                kind: row.kind,
                state: row.state,
                n: 1,
            }),
        }
    }
    out
}

/// The matched rows narrowed by the question's states and time, in the
/// order they are shown.
fn narrow<'w>(
    reading: &Reading,
    world: &World,
    scope: Option<&BotScope>,
    matched: Vec<&'w BotRowInfo>,
    accounts_only: bool,
) -> Vec<&'w BotRowInfo> {
    let subject = &reading.subject;
    let by_subject = !subject.is_empty();
    let mut answer: Vec<&BotRowInfo> = matched.clone();
    if !reading.states.is_empty() {
        answer.retain(|row| {
            row.kind == BotRowKind::Account
                || row.state.is_some_and(|s| reading.states.contains(&s))
        });
    }
    if let Some((from, to)) = window(reading.when, world.now_ms, world.offset_hours) {
        answer.retain(|row| {
            row.kind == BotRowKind::Account || row.at_ms.is_some_and(|at| at >= from && at < to)
        });
    }
    // A named record whose state isn't the one asked about still answers
    // the question ("Is the login redirect stuck?": it finished).
    if answer.is_empty() && by_subject {
        answer = matched;
    }
    if !by_subject
        && reading.states.is_empty()
        && reading.when == When::Any
        && scope.is_none()
        && !accounts_only
    {
        // "What's going on?": what is open now, never every past session.
        answer.retain(|row| {
            row.kind == BotRowKind::Account
                || row.state.is_some_and(|s| {
                    matches!(
                        s,
                        BotState::AsksYou
                            | BotState::Stuck
                            | BotState::Limit
                            | BotState::Working
                            | BotState::Blocked
                            | BotState::InReview
                    )
                })
        });
    }
    if reading.most_room || accounts_only {
        answer.sort_by_key(|row| (row.used_percent.unwrap_or(101), row.title.to_lowercase()));
    } else {
        answer.sort_by_key(|row| (rank(row.state), std::cmp::Reverse(row.at_ms.unwrap_or(0))));
    }
    answer
}

/// The record the question's words name, as a scope for "Looked at":
/// one row named by its own name, or a project every match is in.
fn subject_named(matched: &[&BotRowInfo], subject: &[String]) -> Option<BotScope> {
    let own: Vec<&&BotRowInfo> = matched
        .iter()
        .filter(|row| {
            let names = name_words(&row.title);
            subject.iter().all(|a| {
                name_words(a)
                    .iter()
                    .all(|w| names.iter().any(|n| word_matches(w, n)))
            })
        })
        .collect();
    if let [row] = own.as_slice()
        && row.kind != BotRowKind::Task
    {
        return Some(BotScope {
            kind: if row.kind == BotRowKind::Account {
                BotScopeKind::Account
            } else {
                BotScopeKind::Session
            },
            id: row.id.clone(),
            name: row.title.clone(),
            sessions: Vec::new(),
        });
    }
    let project = matched
        .first()
        .and_then(|row| row.project.clone().zip(row.project_id.clone()))?;
    let names = name_words(&project.0);
    let all_in = matched
        .iter()
        .all(|row| row.project_id.as_deref() == Some(project.1.as_str()));
    let project_named = subject
        .iter()
        .all(|a| names.iter().any(|n| word_matches(a, n)));
    (all_in && project_named).then(|| BotScope {
        kind: BotScopeKind::Project,
        id: project.1,
        name: project.0,
        sessions: Vec::new(),
    })
}
