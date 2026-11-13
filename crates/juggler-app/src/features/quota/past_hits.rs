//! D-103.10: limit hits a version before 0.4.4 credited to the wrong
//! Account, given back once, at start: the app's own data migration.
//!
//! A conversation moved to another Account (ADR-0031) takes its history
//! along, and before 0.4.4 a limit hit in it was read where the
//! conversation sat, so the Account it moved to (and any it passed
//! through) held the hit and read as out of room. 0.4.4 credits each new
//! hit by the move journal (`handover::moved_lines`); this pass takes the
//! ones stored before off every Account that holds one wrongly and puts it
//! on the Account it happened on, or on none when that one was removed
//! (never the wrong one). It reads only what the moves brought in, through
//! each tool's adapter, and touches only those hits' own rows. Running it
//! again changes nothing: the second run finds no hit held wrongly. It runs
//! after the database's own upgrade and before anything reads usage; its
//! counts are kept ([`RECORD_KEY`]) for the presence report
//! (`scripts/qa/presence_check.py`), and keeping them is what makes it run
//! once.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use juggler_adapter::AdapterRegistry;
use juggler_core::CoreError;
use juggler_core::model::HarnessId;
use juggler_core::quota::{QuotaSample, WindowKind};
use juggler_core::storage::Storage;

use crate::features::handover::moved_lines::{Moves, folders_of};

/// Where the pass's counts are kept; once they are, it doesn't run again.
pub const RECORD_KEY: &str = "limits.past_hits_given_back";

/// What the pass found and did, in limit hits (one held by two wrong
/// Accounts is one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Outcome {
    /// Held by an Account they didn't happen on, before the pass.
    pub wrong_before: usize,
    /// Of those, now on the Account they happened on only.
    pub moved: usize,
    /// Of those, now on none: the Account they happened on was removed.
    pub cleared: usize,
    /// In a moved conversation and on the Account they happened on only
    /// (written after the move, or credited right): left there.
    pub left: usize,
    /// Held by an Account they didn't happen on, after the pass.
    pub wrong_after: usize,
}

/// A limit hit in a moved conversation: as its transcript records it, the
/// Account it was spent on (`None`: removed) and those a move carried it
/// into, where it was read (none: it was written where it is).
struct MovedHit {
    kind: WindowKind,
    hit: QuotaSample,
    spent: Option<String>,
    read_in: Vec<String>,
}

impl MovedHit {
    /// Of `holders`, the Accounts holding it that it didn't happen on.
    fn held_wrongly(&self, holders: &[String]) -> Vec<String> {
        holders
            .iter()
            .filter(|holder| self.read_in.contains(holder) && self.spent.as_ref() != Some(holder))
            .cloned()
            .collect()
    }
}

/// At start, once: the pass, when it hasn't run on this data yet, and its
/// counts kept. `None` when it ran before, or couldn't run now (noted:
/// nothing changed, the pass being one transaction, and the next start
/// tries again).
pub fn once_at_start(
    storage: &Storage,
    data_dir: &Path,
    registry: &AdapterRegistry,
) -> Option<Outcome> {
    let settings = storage.app_settings();
    match settings.get(RECORD_KEY) {
        Ok(None) => {}
        Ok(Some(_)) => return None,
        Err(error) => {
            tracing::warn!("past limit hits weren't checked: {error}");
            return None;
        }
    }
    let outcome = match give_back(storage, &Moves::read_at(data_dir), registry) {
        Ok(outcome) => outcome,
        Err(error) => {
            tracing::warn!("past limit hits weren't given back: {error}");
            return None;
        }
    };
    let record = serde_json::to_string(&outcome).ok()?;
    if let Err(error) = settings.set(RECORD_KEY, &record) {
        tracing::warn!("past limit hits: the counts weren't kept: {error}");
    }
    tracing::info!(
        wrong_before = outcome.wrong_before,
        moved = outcome.moved,
        cleared = outcome.cleared,
        left = outcome.left,
        wrong_after = outcome.wrong_after,
        "past limit hits given back"
    );
    Some(outcome)
}

/// The pass: every limit hit in what the moves brought in, taken off each
/// Account holding it that it didn't happen on and put on the one it did
/// (on none when that one was removed).
///
/// # Errors
///
/// [`CoreError::Storage`] when the database can't be read or written;
/// nothing was given back then.
pub fn give_back(
    storage: &Storage,
    moves: &Moves,
    registry: &AdapterRegistry,
) -> Result<Outcome, CoreError> {
    let folders = folders_of(&storage.accounts().list()?);
    let hits = carried_hits(moves, &folders, registry);
    let usage = storage.usage();
    let mut outcome = Outcome::default();
    let mut given = Vec::new();
    for hit in &hits {
        let holders = usage.limit_hit_holders(hit.kind, &hit.hit)?;
        let wrong = hit.held_wrongly(&holders);
        if wrong.is_empty() {
            let on_its_own = hit.spent.as_ref().is_some_and(|on| holders.contains(on));
            outcome.left += usize::from(on_its_own);
            continue;
        }
        outcome.wrong_before += 1;
        if hit.spent.is_some() {
            outcome.moved += 1;
        } else {
            outcome.cleared += 1;
        }
        given.extend(
            wrong
                .into_iter()
                .map(|from| (from, hit.spent.clone(), hit.kind, hit.hit)),
        );
    }
    if !given.is_empty() {
        usage.give_back_limit_hits(&given)?;
    }
    for hit in &hits {
        let holders = usage.limit_hit_holders(hit.kind, &hit.hit)?;
        outcome.wrong_after += usize::from(!hit.held_wrongly(&holders).is_empty());
    }
    Ok(outcome)
}

/// Every limit hit in what the moves brought in (a hit written after the
/// move too, read in none but where it is), read by its tool's adapter,
/// each once.
fn carried_hits(
    moves: &Moves,
    folders: &[(PathBuf, String)],
    registry: &AdapterRegistry,
) -> Vec<MovedHit> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (tool, at) in moves.brought_in() {
        let Some(adapter) = registry.get(&HarnessId::new(tool)) else {
            continue;
        };
        for (file, kind, hit) in adapter.limit_hits_at(at) {
            let Some(here) = folder_holding(folders, &file) else {
                continue;
            };
            if !seen.insert((file.clone(), kind.as_str(), hit.ts)) {
                continue;
            }
            let read_in = moves.carried_into(folders, here, &file, hit.ts);
            let spent = moves.spent_on(folders, here, &file, hit.ts);
            out.push(MovedHit {
                kind,
                hit,
                spent,
                read_in,
            });
        }
    }
    out
}

/// The Account whose folder holds `file` (the innermost, when one folder
/// is inside another): where the usage reading reads it.
fn folder_holding<'a>(folders: &'a [(PathBuf, String)], file: &Path) -> Option<&'a str> {
    folders
        .iter()
        .filter(|(dir, _)| !dir.as_os_str().is_empty() && file.starts_with(dir))
        .max_by_key(|(dir, _)| dir.components().count())
        .map(|(_, id)| id.as_str())
}
