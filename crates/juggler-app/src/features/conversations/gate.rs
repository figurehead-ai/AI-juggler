//! 0.4.5 (AJ-E805, idle CPU): which of the tools' files the conversation
//! list's pass reads.
//!
//! The pass looked at every account's files every 15 s, whatever had
//! happened: a listing of each folder and two `stat`s per transcript. On a
//! heavy Mac's 4,800 transcripts that was the backend's largest cost left
//! (`docs/qa/evidence/AJ-E805/idle-0.4.5.md`).
//!
//! Now each account's adapter names the folders its conversations are in
//! (`conversation_roots`, ADR-0019: the gate names no tool), the system's
//! file events there are heard, and a pass reads:
//! - a changed file the list holds: that file alone (read again if its
//!   size or time moved, dropped if it's gone);
//! - anything else changed in an account's folders (a new conversation, a
//!   new project folder): that account's files listed again;
//! - an account whose tool names no folder, or whose folder can't be
//!   watched (not made yet): every pass, as before;
//! - everything: the first pass, when the system lost track of its events,
//!   when the accounts or their folders change, and every [`SWEEP_EVERY`]
//!   (ADR-0024: events are hints; a late or lost one delays a change by
//!   that much at most).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_adapters::folder_wake::{FolderWake, Heard};

/// The longest between two passes that look at every file anyway.
pub const SWEEP_EVERY: Duration = Duration::from_secs(300);

/// What one pass reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    /// The accounts whose files are listed and read as they changed.
    pub accounts: BTreeSet<String>,
    /// Files the list holds that changed, each with its account.
    pub files: BTreeMap<PathBuf, String>,
}

impl Scope {
    /// Nothing to read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty() && self.files.is_empty()
    }
}

/// What a pass reads: everything, or only a [`Scope`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// Every account's files.
    Whole,
    /// Only these.
    Some(Scope),
}

/// One account as the gate sees it: its id and the folders its tool names.
pub type Watched = (String, Vec<PathBuf>);

/// Decides each pass what it reads, from the file events heard since the
/// last one.
pub struct Gate {
    wake: FolderWake,
    heard: Arc<Mutex<Heard>>,
    /// Each watched folder as the system names it, as the account's folder
    /// spells it, and whose it is.
    roots: Vec<(PathBuf, PathBuf, String)>,
    /// The accounts at the last pass, each with whether all its folders
    /// are watched.
    accounts: BTreeMap<String, bool>,
    swept_at: Option<Instant>,
}

impl std::fmt::Debug for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gate")
            .field("roots", &self.roots.len())
            .field("accounts", &self.accounts.len())
            .finish_non_exhaustive()
    }
}

impl Gate {
    /// A gate on the system's file events.
    #[must_use]
    pub fn new() -> Self {
        let (wake, heard) = FolderWake::trees_heard();
        Self::with(wake, heard)
    }

    fn with(wake: FolderWake, heard: Arc<Mutex<Heard>>) -> Self {
        Self {
            wake,
            heard,
            roots: Vec::new(),
            accounts: BTreeMap::new(),
            swept_at: None,
        }
    }

    /// A gate that hears only its hand (the tests' events).
    #[cfg(test)]
    pub(crate) fn by_hand() -> (Self, juggler_adapters::folder_wake::Hand) {
        let (wake, hand, heard) = FolderWake::by_hand_heard();
        (Self::with(wake, heard), hand)
    }

    /// What the pass at `now` reads, for `accounts` (each with the folders
    /// its tool names), `known` saying which files the list holds.
    pub fn plan(
        &mut self,
        accounts: &[Watched],
        known: &dyn Fn(&Path) -> bool,
        now: Instant,
    ) -> Plan {
        let moved = self.watch(accounts);
        let heard = Heard::take(&self.heard);
        let due = self
            .swept_at
            .is_none_or(|at| now.saturating_duration_since(at) >= SWEEP_EVERY);
        if moved || due || heard.lost || !self.wake.hears_the_system() {
            self.swept_at = Some(now);
            return Plan::Whole;
        }
        let mut scope = Scope {
            accounts: self
                .accounts
                .iter()
                .filter(|(_, watched)| !**watched)
                .map(|(account, _)| account.clone())
                .collect(),
            files: BTreeMap::new(),
        };
        for path in heard.paths {
            let Some((path, account)) = self.spelled(&path) else {
                continue;
            };
            if known(&path) {
                scope.files.insert(path, account);
            } else {
                scope.accounts.insert(account);
            }
        }
        scope
            .files
            .retain(|_, account| !scope.accounts.contains(account));
        Plan::Some(scope)
    }

    /// Watches every account's folders that exist; whether the accounts
    /// or what is watched changed since the last pass.
    fn watch(&mut self, accounts: &[Watched]) -> bool {
        let mut roots = Vec::new();
        let mut watched = BTreeMap::new();
        for (account, folders) in accounts {
            let present: Vec<&PathBuf> = folders.iter().filter(|folder| folder.is_dir()).collect();
            watched.insert(
                account.clone(),
                !folders.is_empty() && present.len() == folders.len(),
            );
            for folder in present {
                // The system names what changed by the folder's real path
                // (`/private/tmp` for `/tmp`).
                let real = std::fs::canonicalize(folder).unwrap_or_else(|_| folder.clone());
                roots.push((real, folder.clone(), account.clone()));
            }
        }
        let folders: BTreeSet<PathBuf> = roots.iter().map(|(real, _, _)| real.clone()).collect();
        if &folders != self.wake.folders() {
            self.wake.watch(&folders);
        }
        // A folder the system wouldn't watch is read every pass.
        for (real, _, account) in &roots {
            if !self.wake.folders().contains(real) {
                watched.insert(account.clone(), false);
            }
        }
        let moved = watched != self.accounts || roots != self.roots;
        self.accounts = watched;
        self.roots = roots;
        moved
    }

    /// `path` as its account's folder spells it, and whose it is.
    fn spelled(&self, path: &Path) -> Option<(PathBuf, String)> {
        self.roots.iter().find_map(|(real, spelled, account)| {
            let rest = path
                .strip_prefix(real)
                .or_else(|_| path.strip_prefix(spelled))
                .ok()?;
            Some((spelled.join(rest), account.clone()))
        })
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, reason = "test code")]

    use super::*;
    use juggler_adapters::folder_wake::Woke;

    fn at(t0: Instant, secs: u64) -> Instant {
        t0 + Duration::from_secs(secs)
    }

    /// AJ-E805: at rest, a pass reads nothing until the sweep; a write to a
    /// file the list holds reads that file; anything else new under an
    /// account lists that account again; a lost track reads everything.
    /// The pass looked at every file of every account every 15 s.
    #[test]
    fn a_pass_reads_what_changed_and_sweeps_rarely() {
        let dir = tempfile::tempdir().unwrap();
        let claude = dir.path().join("a/projects");
        let codex = dir.path().join("b/sessions");
        std::fs::create_dir_all(&claude).unwrap();
        std::fs::create_dir_all(&codex).unwrap();
        let held = claude.join("p/one.jsonl");
        let accounts: Vec<Watched> = vec![
            ("a".into(), vec![claude.clone()]),
            ("b".into(), vec![codex.clone()]),
            // A tool that names no folder is read every pass.
            ("c".into(), vec![]),
        ];
        let known = |path: &Path| path == held;
        let (mut gate, hand) = Gate::by_hand();
        let t0 = Instant::now();
        assert_eq!(
            gate.plan(&accounts, &known, t0),
            Plan::Whole,
            "the first pass"
        );
        let quiet = Scope {
            accounts: BTreeSet::from(["c".to_owned()]),
            files: BTreeMap::new(),
        };
        for tick in 1..20 {
            assert_eq!(
                gate.plan(&accounts, &known, at(t0, tick * 15)),
                Plan::Some(quiet.clone())
            );
        }
        // A write to a held file: that file.
        let real = |path: &Path| {
            std::fs::canonicalize(dir.path())
                .unwrap()
                .join(path.strip_prefix(dir.path()).unwrap())
        };
        hand.wake(Woke::Paths(vec![real(&held)]));
        let Plan::Some(scope) = gate.plan(&accounts, &known, at(t0, 290)) else {
            panic!("not a sweep yet");
        };
        assert_eq!(
            scope.files,
            BTreeMap::from([(held.clone(), "a".to_owned())])
        );
        // A new file in b: b listed again.
        hand.wake(Woke::Paths(vec![real(
            &codex.join("2026/11/13/rollout-x.jsonl"),
        )]));
        let Plan::Some(scope) = gate.plan(&accounts, &known, at(t0, 295)) else {
            panic!("not a sweep yet");
        };
        assert_eq!(
            scope.accounts,
            BTreeSet::from(["b".to_owned(), "c".to_owned()])
        );
        // The sweep, then a lost track.
        assert_eq!(gate.plan(&accounts, &known, at(t0, 300)), Plan::Whole);
        hand.wake(Woke::Rescan);
        assert_eq!(gate.plan(&accounts, &known, at(t0, 315)), Plan::Whole);
        // A new account, or a folder made since: everything.
        let more: Vec<Watched> = accounts
            .iter()
            .cloned()
            .chain([("d".to_owned(), vec![dir.path().join("d/projects")])])
            .collect();
        assert_eq!(gate.plan(&more, &known, at(t0, 330)), Plan::Whole);
        let Plan::Some(scope) = gate.plan(&more, &known, at(t0, 345)) else {
            panic!("not a sweep yet");
        };
        assert!(
            scope.accounts.contains("d"),
            "a folder not made yet: every pass"
        );
        std::fs::create_dir_all(dir.path().join("d/projects")).unwrap();
        assert_eq!(gate.plan(&more, &known, at(t0, 360)), Plan::Whole);
    }
}
