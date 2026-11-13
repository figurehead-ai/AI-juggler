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

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use juggler_adapters::folder_wake::FolderWake;

use super::{FILE_SCAN_EVERY, ZCODE_EVERY};
use crate::state::AppState;

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
    woke: Arc<AtomicBool>,
    /// The folders watched for the accounts known at the last pass.
    roots: BTreeSet<PathBuf>,
    /// A store pass ran since the accounts last changed.
    stores_probed: bool,
}

impl Gate {
    /// A gate on the system's file events (nothing watched until
    /// [`Gate::watch`]).
    pub fn new() -> Self {
        let woke = Arc::new(AtomicBool::new(false));
        let heard = Arc::clone(&woke);
        Self {
            wake: FolderWake::trees(move |_| heard.store(true, Ordering::Release)),
            woke,
            roots: BTreeSet::new(),
            stores_probed: false,
        }
    }

    /// What the pass at `tick` reads, or `None` when none is due.
    pub fn due(&mut self, tick: u32) -> Option<Due> {
        let every = Due::every(tick);
        if !self.wake.hears_the_system() {
            return Some(every);
        }
        let woke = self.woke.swap(false, Ordering::AcqRel) | SOON.swap(false, Ordering::AcqRel);
        let stores = !self.stores_probed || STORE_SEEN.load(Ordering::Acquire);
        let due = decide(
            tick,
            woke,
            crate::features::quota::records::will_be_due(tick),
            stores,
        );
        if due.is_some_and(|due| due.stores) {
            self.stores_probed = true;
        }
        due
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
        let roots: BTreeSet<PathBuf> = std::iter::once(state.overlays_dir())
            .chain(accounts.into_iter().map(|account| account.state_dir))
            .filter(|root| !root.as_os_str().is_empty() && root.is_dir())
            .collect();
        if roots != self.roots {
            self.stores_probed = false;
            self.wake.watch(&roots);
            self.roots = roots;
        }
    }
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
