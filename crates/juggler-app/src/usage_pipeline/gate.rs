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
//!   own cadence, as before, while such an account exists;
//! - where the system's events can't be heard, every pass, as before.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use juggler_adapters::folder_wake::FolderWake;

use super::{FILE_SCAN_EVERY, ZCODE_EVERY};
use crate::state::AppState;

/// Asked for a pass at the next second (an account's folder moved).
static SOON: AtomicBool = AtomicBool::new(false);

/// Runs a pass at the next second, whatever changed.
pub(crate) fn soon() {
    SOON.store(true, Ordering::Release);
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

/// The kinds of account there are (what a pass can have to read).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Kinds {
    stores: bool,
    zcode: bool,
}

/// Decides each second whether a pass runs, from the file events heard
/// since the last one and the cadences above.
pub(crate) struct Gate {
    wake: FolderWake,
    woke: Arc<AtomicBool>,
    kinds: Kinds,
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
            kinds: Kinds::default(),
        }
    }

    /// What the pass at `tick` reads, or `None` when none is due.
    pub fn due(&self, tick: u32) -> Option<Due> {
        let every = Due::every(tick);
        if !self.wake.hears_the_system() {
            return Some(every);
        }
        let woke = self.woke.swap(false, Ordering::AcqRel) | SOON.swap(false, Ordering::AcqRel);
        decide(
            tick,
            woke,
            crate::features::quota::records::will_be_due(tick),
            self.kinds,
        )
    }

    /// Watches what the accounts' passes read, from now on (after each
    /// pass: a new account's folders are watched within a cadence).
    pub fn watch(&mut self, state: &Arc<AppState>) {
        let accounts = state
            .storage
            .lock()
            .ok()
            .and_then(|storage| storage.accounts().list().ok())
            .unwrap_or_default();
        let mut roots = BTreeSet::from([state.overlays_dir()]);
        let mut kinds = Kinds::default();
        for account in &accounts {
            let dir = &account.state_dir;
            if dir.as_os_str().is_empty() {
                continue;
            }
            match account.harness.as_str() {
                "claude-code" => {
                    roots.insert(dir.join("projects"));
                }
                "codex" => {
                    roots.insert(dir.join("sessions"));
                }
                "zcode" => kinds.zcode = true,
                _ => kinds.stores = true,
            }
        }
        // A folder that isn't there yet (an account with no transcript) is
        // tried again after the next pass; its first files are read on the
        // records' cadence until then.
        let roots: BTreeSet<PathBuf> = roots.into_iter().filter(|root| root.is_dir()).collect();
        self.wake.watch(&roots);
        self.kinds = kinds;
    }
}

/// The gate's rule (pure): what the pass at `tick` reads, given whether
/// a watched file changed since the last pass, whether Claude's records
/// are due, and which kinds of account exist.
fn decide(tick: u32, woke: bool, records: bool, kinds: Kinds) -> Option<Due> {
    let every = Due::every(tick);
    let files = woke || records;
    let due = Due {
        claude: files,
        codex: files,
        stores: kinds.stores && every.stores,
        zcode: kinds.zcode && every.zcode,
    };
    (due.claude || due.stores || due.zcode).then_some(due)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]

    use super::*;
    use crate::features::quota::records::RECORDS_EVERY;

    /// The idle rule: with nothing written and only Claude and Codex
    /// accounts, a pass runs on the records' cadence only (it ran every
    /// second, listing every transcript, AJ-E805).
    #[test]
    fn a_quiet_mac_runs_a_pass_only_on_the_records_cadence() {
        let quiet = Kinds::default();
        let passes: Vec<u32> = (2..=60)
            .filter(|tick| {
                decide(*tick, false, tick.is_multiple_of(RECORDS_EVERY), quiet).is_some()
            })
            .collect();
        assert_eq!(passes, vec![15, 30, 45, 60]);
    }

    /// A write under a watched tree is read at the next pass, Codex's
    /// rollouts with Claude's transcripts.
    #[test]
    fn a_write_is_read_at_the_next_pass() {
        let due = decide(7, true, false, Kinds::default()).unwrap();
        assert!(due.claude && due.codex);
        assert!(!due.stores && !due.zcode);
    }

    /// A store read by time keeps its own cadence (its database isn't a
    /// tree the events cover), without reading the transcripts with it.
    #[test]
    fn a_store_keeps_its_cadence() {
        let stores = Kinds {
            stores: true,
            zcode: false,
        };
        let due = decide(FILE_SCAN_EVERY, false, false, stores).unwrap();
        assert!(due.stores && !due.claude && !due.codex);
        assert!(decide(FILE_SCAN_EVERY + 1, false, false, stores).is_none());
        let zcode = Kinds {
            stores: false,
            zcode: true,
        };
        assert!(decide(ZCODE_EVERY, false, false, zcode).is_some_and(|due| due.zcode));
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
