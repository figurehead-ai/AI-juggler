//! 0.4.5 (AJ-E805, idle CPU): when the usage pipeline's pass runs, and
//! what it reads.
//!
//! The pass ran every second whatever had happened: each time it listed
//! every Claude account's transcript folders, looked at the size of every
//! transcript in them and wrote its offsets back to storage, on a Mac
//! where nothing had been written since. On the idle lab (12 Accounts,
//! 342 transcripts) that was the backend's largest cost, and it grows with
//! every transcript a heavy Mac keeps.
//!
//! Now a pass runs when what it reads changed, and on its slower cadences:
//! - the system's file events on the trees it reads (the Claude accounts'
//!   `projects`, the Codex accounts' `sessions`, AI Juggler's own status
//!   line spools): a write is read at the next second, as before;
//! - Claude's own limit records' cadence (every
//!   [`RECORDS_EVERY`](crate::features::quota::records::RECORDS_EVERY)
//!   passes, or the next one when asked), which reads the transcripts and
//!   rollouts too: a late or lost event (ADR-0024: events are hints)
//!   delays a change by that much at most;
//! - the stores read by time (a tool's database, not a file tree) on their
//!   own cadence, as before, while an account has one (the first store
//!   pass after the accounts change finds out);
//! - where the system's events can't be heard, every pass, as before.
//!
//! What it watches is every account's own folder and AI Juggler's spools,
//! whatever the tool: which files in them a tool's usage is in is the
//! pipeline's to say, not the gate's (ADR-0019).
//!
//! A pass that only file events asked for reads only the files they named
//! ([`Gate::named`]: AJ-E805's follow-up, with sessions at work it walked
//! every transcript of every account every second; on the grown lab, 4,800
//! of them, the backend's largest cost under load). Round 3: the records'
//! cadence no longer walks them either (it listed every transcript every
//! 15 s, about 0.3 % of a core at 4,800): a pass walks everything first,
//! after an ask, a lost track or a change of the accounts watched, and on
//! the [`SWEEP_EVERY`] sweep (a write whose event never came is read
//! within it), and where events can't be heard, as before.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use juggler_adapters::folder_wake::{FolderWake, Heard};

use super::{FILE_SCAN_EVERY, ZCODE_EVERY};
use crate::state::AppState;

/// How often (passes, a second each) a pass walks every transcript and
/// rollout although no event asked: the bound for a write whose event
/// never came (5 min, as the conversation list's sweep).
pub(crate) const SWEEP_EVERY: u32 = 300;

/// Asked for a pass at the next second (an account's folder moved).
static SOON: AtomicBool = AtomicBool::new(false);

/// A tool's store read by time answered (some account has one).
static STORE_SEEN: AtomicBool = AtomicBool::new(false);

/// Runs a pass at the next second, whatever changed.
pub(crate) fn soon() {
    SOON.store(true, Ordering::Release);
}

/// An account's tool keeps its usage in a store read by time: the stores'
/// cadence goes on.
pub(crate) fn saw_a_store() {
    STORE_SEEN.store(true, Ordering::Release);
}

/// What one pass reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each source is read or not, on its own cadence"
)]
pub(crate) struct Due {
    /// The Claude accounts' status line spools and transcripts.
    pub claude: bool,
    /// The Codex accounts' rollouts.
    pub codex: bool,
    /// The stores read by time (another tool's database).
    pub stores: bool,
    /// `ZCode`'s database.
    pub zcode: bool,
}

impl Due {
    /// The cadence a pass at `tick` had before the gate (and has where
    /// events can't be heard): Claude every pass, the rest on theirs.
    pub fn every(tick: u32) -> Self {
        let files = tick.is_multiple_of(FILE_SCAN_EVERY);
        Self {
            claude: true,
            codex: files,
            stores: files,
            zcode: tick.is_multiple_of(ZCODE_EVERY),
        }
    }
}

/// Decides each second whether a pass runs, from the file events heard
/// since the last one and the cadences above.
pub(crate) struct Gate {
    wake: FolderWake,
    heard: Arc<Mutex<Heard>>,
    /// The folders watched for the accounts known at the last pass: each
    /// as the system names it, and as the accounts spell it.
    roots: BTreeSet<(PathBuf, PathBuf)>,
    /// A store pass ran since the accounts last changed.
    stores_probed: bool,
    /// The files the events named, for a pass only they asked for.
    named: Option<BTreeSet<PathBuf>>,
    /// The accounts watched changed: the next pass walks everything.
    walk_next: bool,
}

impl Gate {
    /// A gate on the system's file events (nothing watched until
    /// [`Gate::watch`]).
    pub fn new() -> Self {
        let (wake, heard) = FolderWake::trees_heard();
        Self {
            wake,
            heard,
            roots: BTreeSet::new(),
            stores_probed: false,
            named: None,
            walk_next: true,
        }
    }

    /// What the pass at `tick` reads, or `None` when none is due.
    pub fn due(&mut self, tick: u32) -> Option<Due> {
        let every = Due::every(tick);
        if !self.wake.hears_the_system() {
            return Some(every);
        }
        let heard = Heard::take(&self.heard);
        let soon = SOON.swap(false, Ordering::AcqRel);
        let records = crate::features::quota::records::will_be_due(tick);
        let stores = !self.stores_probed || STORE_SEEN.load(Ordering::Acquire);
        let due = decide(tick, heard.any() || soon, records, stores);
        if due.is_some_and(|due| due.stores) {
            self.stores_probed = true;
        }
        // A change of the accounts watched is walked at the next pass that runs.
        let reads_files = due.is_some_and(|due| due.claude || due.codex);
        let asked = soon || (reads_files && std::mem::take(&mut self.walk_next));
        self.named = named(walks(tick, asked), &heard, &self.roots);
        due
    }

    /// The files the last [`Gate::due`]'s events named, when only they
    /// asked for its pass (`None`: walk everything).
    pub fn named(&mut self) -> Option<BTreeSet<PathBuf>> {
        self.named.take()
    }

    /// Watches every account's folder and AI Juggler's spools from now on
    /// (after each pass: a new account is watched within a cadence, and
    /// its store, if it has one, is looked for again).
    pub fn watch(&mut self, state: &Arc<AppState>) {
        let accounts = state
            .storage
            .lock()
            .ok()
            .and_then(|storage| storage.accounts().list().ok())
            .unwrap_or_default();
        let roots: BTreeSet<(PathBuf, PathBuf)> = std::iter::once(state.overlays_dir())
            .chain(accounts.into_iter().map(|account| account.state_dir))
            .filter(|root| !root.as_os_str().is_empty() && root.is_dir())
            .map(|root| {
                (
                    std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone()),
                    root,
                )
            })
            .collect();
        if roots != self.roots {
            self.stores_probed = false;
            self.walk_next = true;
            self.wake
                .watch(&roots.iter().map(|(real, _)| real.clone()).collect());
            self.roots = roots;
        }
    }
}

/// Whether the pass at `tick` walks every file (pure): the first, one
/// `asked` (an ask, or the accounts watched changed) and the sweep's.
/// Claude's records' cadence reads the records alone.
fn walks(tick: u32, asked: bool) -> bool {
    asked || tick <= 1 || tick.is_multiple_of(SWEEP_EVERY)
}

/// The files a pass reads when only the events `heard` asked for it:
/// theirs alone, as the accounts spell them; `None` (walk everything) when
/// the pass walks ([`walks`]), or after a lost track.
fn named(
    walk: bool,
    heard: &Heard,
    roots: &BTreeSet<(PathBuf, PathBuf)>,
) -> Option<BTreeSet<PathBuf>> {
    (!walk && !heard.lost).then(|| {
        heard
            .paths
            .iter()
            .filter_map(|path| spelled(roots, path))
            .collect()
    })
}

/// `path` (as the system names it) as the account's folder spells it:
/// the offsets are kept by that spelling (`/tmp`, not `/private/tmp`).
fn spelled(roots: &BTreeSet<(PathBuf, PathBuf)>, path: &Path) -> Option<PathBuf> {
    roots.iter().find_map(|(real, spelled)| {
        let rest = path
            .strip_prefix(real)
            .or_else(|_| path.strip_prefix(spelled))
            .ok()?;
        Some(spelled.join(rest))
    })
}

/// The gate's rule (pure): what the pass at `tick` reads, given whether
/// a watched file changed since the last pass, whether Claude's records
/// are due, and whether a store read by time may exist.
fn decide(tick: u32, woke: bool, records: bool, stores: bool) -> Option<Due> {
    let every = Due::every(tick);
    let files = woke || records;
    let due = Due {
        claude: files,
        codex: files,
        stores: stores && every.stores,
        zcode: every.zcode,
    };
    (files || due.stores).then_some(due)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]

    use super::*;
    use crate::features::quota::records::RECORDS_EVERY;

    /// Round 3 (AJ-E805): the records' cadence reads the records alone;
    /// every transcript is walked first, when asked, and every 5 min (the
    /// records' cadence walked every one every 15 s).
    #[test]
    fn the_records_cadence_walks_nothing_and_the_sweep_walks_everything() {
        let walked: Vec<u32> = (1..=SWEEP_EVERY * 2)
            .filter(|tick| walks(*tick, false))
            .collect();
        assert_eq!(walked, vec![1, SWEEP_EVERY, SWEEP_EVERY * 2]);
        assert!(!walks(RECORDS_EVERY, false), "the records' own pass");
        assert!(walks(RECORDS_EVERY + 1, true), "an ask, or new accounts");
    }

    /// The idle rule: with nothing written and no store read by time, a
    /// pass runs on the records' cadence only (it ran every second,
    /// listing every transcript, AJ-E805).
    #[test]
    fn a_quiet_mac_runs_a_pass_only_on_the_records_cadence() {
        let passes: Vec<u32> = (2..=60)
            .filter(|tick| {
                decide(*tick, false, tick.is_multiple_of(RECORDS_EVERY), false).is_some()
            })
            .collect();
        assert_eq!(passes, vec![15, 30, 45, 60]);
    }

    /// A write under a watched folder is read at the next pass, every
    /// tool's files with it.
    #[test]
    fn a_write_is_read_at_the_next_pass() {
        let due = decide(7, true, false, false).unwrap();
        assert!(due.claude && due.codex);
        assert!(!due.stores && !due.zcode);
    }

    /// A store read by time keeps its own cadence (a database isn't a tree
    /// the events cover), without reading the files with it.
    #[test]
    fn a_store_keeps_its_cadence() {
        let due = decide(FILE_SCAN_EVERY, false, false, true).unwrap();
        assert!(due.stores && !due.claude && !due.codex);
        assert!(decide(FILE_SCAN_EVERY + 1, false, false, true).is_none());
        assert!(decide(FILE_SCAN_EVERY, false, false, false).is_none());
    }

    /// AJ-E805: a pass only events asked for reads the files they named,
    /// spelled as the account's folder is (its offsets' keys); the records'
    /// cadence, an ask or a lost track walks everything. With sessions at
    /// work, every event walked every transcript of every account.
    #[test]
    fn a_pass_on_events_reads_the_files_they_named() {
        let roots = BTreeSet::from([(
            PathBuf::from("/private/tmp/lab/.claude"),
            PathBuf::from("/tmp/lab/.claude"),
        )]);
        let heard = Heard {
            paths: BTreeSet::from([
                PathBuf::from("/private/tmp/lab/.claude/projects/p/a.jsonl"),
                PathBuf::from("/elsewhere/b.jsonl"),
            ]),
            lost: false,
        };
        assert_eq!(
            named(false, &heard, &roots),
            Some(BTreeSet::from([PathBuf::from(
                "/tmp/lab/.claude/projects/p/a.jsonl"
            )]))
        );
        assert_eq!(named(true, &heard, &roots), None, "a walk");
        let lost = Heard {
            lost: true,
            ..Heard::default()
        };
        assert_eq!(named(false, &lost, &roots), None, "a lost track");
    }

    /// Before the gate (and where events can't be heard): Claude every
    /// pass, the rest every 4th.
    #[test]
    fn the_old_cadence_is_kept_where_events_cant_be_heard() {
        assert_eq!(
            Due::every(4),
            Due {
                claude: true,
                codex: true,
                stores: true,
                zcode: false
            }
        );
        assert!(Due::every(3).claude && !Due::every(3).codex);
    }
}
