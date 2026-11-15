//! 0.3.5 (ADR-0024): a wake-up when a file in one of a few folders
//! changes, for the conversation feed and the live view: only the folders
//! they name are watched, not recursively, and only while they name them.
//! 0.4.5 (AJ-E805): a [`FolderWake::trees`] watch takes in each folder's
//! subfolders too (the usage pipeline's transcript trees: one system
//! stream, where it listed every folder every second).
//!
//! Events are hints, never the truth. The system's file events can come
//! late (minutes on a Mac whose event service is saturated by another
//! program's disk traffic), merged, or not at all (sleep and wake; `FSEvents`'
//! `MustScanSubDirs` when it dropped some). So:
//! - a wake-up that says the system lost track is passed on as
//!   [`Woke::Rescan`] (anything in the watched folders may have changed),
//!   never dropped for naming no file;
//! - every user also checks its files on its own bounded timer (the feed
//!   every second while a page follows one; the live view's session files
//!   every 5 s and the apps' records every 30 s), so a late or lost event
//!   only delays a change by that much, never leaves it unseen.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// What a wake-up says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Woke {
    /// These paths changed.
    Paths(Vec<PathBuf>),
    /// The system lost track of some changes (it dropped or merged
    /// events, or its watcher failed): anything in the watched folders may
    /// have changed.
    Rescan,
}

impl Woke {
    /// Whether this wake-up may concern a file `named` picks out: a
    /// rescan concerns every file.
    pub fn concerns(&self, named: impl Fn(&std::path::Path) -> bool) -> bool {
        match self {
            Self::Paths(paths) => paths.iter().any(|path| named(path)),
            Self::Rescan => true,
        }
    }
}

/// AJ-E805 (0.4.5): what a watch's events said since it was last taken:
/// the paths they named, or that anything may have changed. The
/// conversation list's and the usage pipeline's gates read only what their
/// events named, and everything after a lost track.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heard {
    /// The paths the events named.
    pub paths: BTreeSet<PathBuf>,
    /// The system lost track ([`Woke::Rescan`]), or more paths changed than
    /// are kept ([`Heard::MAX`]): anything may have changed.
    pub lost: bool,
}

impl Heard {
    /// The most paths kept between two takes.
    pub const MAX: usize = 4096;

    /// Notes one wake-up.
    pub fn note(&mut self, woke: Woke) {
        match woke {
            Woke::Rescan => self.lost = true,
            Woke::Paths(paths) => {
                self.paths.extend(paths);
                if self.paths.len() > Self::MAX {
                    self.paths.clear();
                    self.lost = true;
                }
            }
        }
    }

    /// Something was heard.
    #[must_use]
    pub fn any(&self) -> bool {
        self.lost || !self.paths.is_empty()
    }

    /// What was heard since the last take, emptied.
    #[must_use]
    pub fn take(heard: &Mutex<Self>) -> Self {
        heard
            .lock()
            .map(|mut heard| std::mem::take(&mut *heard))
            .unwrap_or_default()
    }
}

/// Notes `woke` into `heard`.
fn note_into(heard: &Mutex<Heard>, woke: Woke) {
    if let Ok(mut heard) = heard.lock() {
        heard.note(woke);
    }
}

/// The callback a watch wakes.
type OnChange = Arc<Mutex<Box<dyn Fn(Woke) + Send>>>;

/// Where wake-ups come from.
enum Source {
    /// The system's file events.
    System(notify::RecommendedWatcher),
    /// Only what a [`Hand`] says (the tests' events, exactly when they
    /// choose).
    Hand,
    /// None: the system's watcher couldn't be made (the users' timers
    /// still notice changes).
    Off,
}

/// Watches a set of folders and calls back with what changed.
pub struct FolderWake {
    source: Source,
    folders: BTreeSet<PathBuf>,
    /// Each folder with everything under it ([`FolderWake::trees`]).
    recursive: bool,
}

/// Wakes a [`FolderWake::by_hand`] watch, as the system would.
#[derive(Clone)]
pub struct Hand {
    on_change: OnChange,
}

impl Hand {
    /// Calls the watch's callback with `woke`.
    pub fn wake(&self, woke: Woke) {
        if let Ok(on_change) = self.on_change.lock() {
            on_change(woke);
        }
    }
}

impl std::fmt::Debug for Hand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hand").finish_non_exhaustive()
    }
}

/// The wake-up one system event makes: a lost track (`FSEvents`'
/// `MustScanSubDirs`, an overflowed queue, a watcher error) is a rescan; an
/// event naming nothing is none.
fn woke_of(event: notify::Result<notify::Event>) -> Option<Woke> {
    match event {
        Err(_) => Some(Woke::Rescan),
        Ok(event) if event.need_rescan() => Some(Woke::Rescan),
        Ok(event) if event.paths.is_empty() => None,
        Ok(event) => Some(Woke::Paths(event.paths)),
    }
}

impl FolderWake {
    /// A watch on the system's file events calling `on_change`. When the
    /// system's watcher can't be made, nothing is watched (the users'
    /// timers still notice changes).
    pub fn new(on_change: impl Fn(Woke) + Send + 'static) -> Self {
        let source = notify::recommended_watcher(move |event| {
            if let Some(woke) = woke_of(event) {
                on_change(woke);
            }
        })
        .map_or(Source::Off, Source::System);
        Self {
            source,
            folders: BTreeSet::new(),
            recursive: false,
        }
    }

    /// Like [`FolderWake::new`], each folder watched with everything under
    /// it.
    pub fn trees(on_change: impl Fn(Woke) + Send + 'static) -> Self {
        Self {
            recursive: true,
            ..Self::new(on_change)
        }
    }

    /// A [`FolderWake::trees`] watch keeping what its events said, to take
    /// with [`Heard::take`].
    #[must_use]
    pub fn trees_heard() -> (Self, Arc<Mutex<Heard>>) {
        let heard = Arc::new(Mutex::new(Heard::default()));
        let into = Arc::clone(&heard);
        (Self::trees(move |woke| note_into(&into, woke)), heard)
    }

    /// A [`FolderWake::by_hand`] watch keeping what its hand says, as
    /// [`FolderWake::trees_heard`] keeps the system's.
    #[must_use]
    pub fn by_hand_heard() -> (Self, Hand, Arc<Mutex<Heard>>) {
        let heard = Arc::new(Mutex::new(Heard::default()));
        let into = Arc::clone(&heard);
        let (wake, hand) = Self::by_hand(move |woke| note_into(&into, woke));
        (wake, hand, heard)
    }

    /// Whether the system's file events can wake it (`false`: its
    /// watcher couldn't be made; its users' own timers must look).
    #[must_use]
    pub fn hears_the_system(&self) -> bool {
        !matches!(self.source, Source::Off)
    }

    /// A watch that hears only its [`Hand`]: every folder asked for counts
    /// as watched, and nothing wakes it unless the hand says so.
    pub fn by_hand(on_change: impl Fn(Woke) + Send + 'static) -> (Self, Hand) {
        let hand = Hand {
            on_change: Arc::new(Mutex::new(Box::new(on_change))),
        };
        let wake = Self {
            source: Source::Hand,
            folders: BTreeSet::new(),
            recursive: false,
        };
        (wake, hand)
    }

    /// Watches exactly `folders` from now on (each not recursively, unless
    /// made by [`FolderWake::trees`]).
    pub fn watch(&mut self, folders: &BTreeSet<PathBuf>) {
        use notify::Watcher as _;
        let mode = if self.recursive {
            notify::RecursiveMode::Recursive
        } else {
            notify::RecursiveMode::NonRecursive
        };
        let watcher = match &mut self.source {
            Source::System(watcher) => watcher,
            Source::Hand => {
                self.folders.clone_from(folders);
                return;
            }
            Source::Off => return,
        };
        for gone in self.folders.difference(folders) {
            let _ = watcher.unwatch(gone);
        }
        let mut now_watched = BTreeSet::new();
        for folder in folders {
            if self.folders.contains(folder) || watcher.watch(folder, mode).is_ok() {
                now_watched.insert(folder.clone());
            }
        }
        self.folders = now_watched;
    }

    /// The folders watched now.
    #[must_use]
    pub fn folders(&self) -> &BTreeSet<PathBuf> {
        &self.folders
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The system's events, as wake-ups: a dropped-events flag or an error
    /// is a rescan (it used to be passed on as the folder's path, or not
    /// at all, and a user looking for its file's name took it for noise).
    #[test]
    fn a_lost_track_is_a_rescan_and_an_empty_event_is_nothing() {
        let file = PathBuf::from("/w/s.jsonl");
        let changed = notify::Event::new(notify::EventKind::Any).add_path(file.clone());
        assert_eq!(woke_of(Ok(changed)), Some(Woke::Paths(vec![file])));
        let dropped = notify::Event::new(notify::EventKind::Other)
            .set_flag(notify::event::Flag::Rescan)
            .add_path(PathBuf::from("/w"));
        assert_eq!(woke_of(Ok(dropped)), Some(Woke::Rescan));
        assert_eq!(
            woke_of(Err(notify::Error::generic("queue overflow"))),
            Some(Woke::Rescan)
        );
        assert_eq!(
            woke_of(Ok(notify::Event::new(notify::EventKind::Any))),
            None
        );
        // A rescan concerns every file; paths only theirs.
        let named = |path: &std::path::Path| path.ends_with("s.jsonl");
        assert!(Woke::Rescan.concerns(named));
        assert!(Woke::Paths(vec![PathBuf::from("/w/s.jsonl")]).concerns(named));
        assert!(!Woke::Paths(vec![PathBuf::from("/w")]).concerns(named));
    }

    #[test]
    fn a_watch_by_hand_follows_its_folders_and_wakes_only_when_told() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (mut wake, hand) = FolderWake::by_hand(move |woke| {
            let _ = tx.send(woke);
        });
        wake.watch(&BTreeSet::from([PathBuf::from("/a"), PathBuf::from("/b")]));
        assert_eq!(wake.folders().len(), 2);
        assert!(rx.try_recv().is_err(), "nothing wakes it by itself");
        hand.wake(Woke::Rescan);
        assert_eq!(rx.try_recv(), Ok(Woke::Rescan));
        wake.watch(&BTreeSet::new());
        assert!(wake.folders().is_empty());
    }

    /// The system's watcher takes a folder and lets it go (no waiting on
    /// its events: when they come is the system's business).
    #[test]
    fn the_system_watch_takes_a_folder_and_lets_it_go() {
        let dir = tempfile::tempdir().unwrap();
        let mut wake = FolderWake::new(|_| {});
        let folder = dir.path().canonicalize().unwrap();
        wake.watch(&BTreeSet::from([folder]));
        assert_eq!(wake.folders().len(), 1);
        wake.watch(&BTreeSet::new());
        assert!(wake.folders().is_empty());
    }

    /// The system's own events reach the callback, naming the file. Not in
    /// the default run: `FSEvents`' delivery time is the OS's, and on a Mac
    /// whose `fseventsd` is saturated by another program's disk traffic it
    /// was seen past 60 s (rc21 preflight, 2026-11-09). The product never
    /// waits on it (each user's timer bounds a change's delay); this checks
    /// only the wiring, with a 5-minute bound for that reason. Run with
    /// `just fs-events-live`.
    #[test]
    #[ignore = "real FSEvents delivery: minutes late on a loaded Mac; `just fs-events-live`"]
    fn a_real_change_in_a_watched_folder_wakes_it() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut wake = FolderWake::new(move |woke| {
            let _ = tx.send(woke);
        });
        let folder = dir.path().canonicalize().unwrap();
        wake.watch(&BTreeSet::from([folder.clone()]));
        std::fs::write(folder.join("s.jsonl"), b"{}\n").unwrap();
        // Events may come in any order (one about the folder itself, made
        // just before the watch, can come first), merged, or late: the file
        // is changed again every 2 s until a wake-up may concern it.
        let started = std::time::Instant::now();
        let deadline = started + std::time::Duration::from_secs(300);
        loop {
            assert!(
                std::time::Instant::now() < deadline,
                "no wake-up named the file"
            );
            match rx.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(woke) if woke.concerns(|path| path.ends_with("s.jsonl")) => break,
                Ok(_) => {}
                Err(_) => std::fs::write(folder.join("s.jsonl"), b"{}\n{}\n").unwrap(),
            }
        }
        eprintln!("woke after {:?}", started.elapsed());
    }
}
