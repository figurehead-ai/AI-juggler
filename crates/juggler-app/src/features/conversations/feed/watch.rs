//! The thread that reads followed conversations' changes (ADR-0024).
//!
//! One named thread, `conversation-feeds`, started by the first follow.
//! It is woken by the folders of the followed files changing (not
//! recursively, only those folders, only while followed) and checks the
//! files every second as the fallback (an event can be dropped or late:
//! sleep and wake, a busy disk). A burst of appends is batched: a pass
//! runs 150 ms after the last change, and at most 500 ms after the first
//! while they keep coming. With nothing followed it sleeps until a page
//! follows something (or, while an unfollowed feed is kept, until that
//! one is due to go).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use juggler_adapters::folder_wake::FolderWake;

use super::{FeedKey, Feeds, lock};
use crate::state::AppState;

/// Quiet after the last change before a pass.
pub const SETTLE: Duration = Duration::from_millis(150);

/// The longest a change waits while changes keep coming.
pub const SETTLE_MAX: Duration = Duration::from_millis(500);

/// How often followed files are checked when nothing wakes the thread.
pub const FALLBACK: Duration = Duration::from_secs(1);

/// What wakes the thread.
pub(crate) enum Wake {
    /// Files changed in a watched folder.
    Changed(Vec<PathBuf>),
    /// A page followed something (or let it go).
    Nudge,
}

/// Batches changes: due [`SETTLE`] after the last one, at most
/// [`SETTLE_MAX`] after the first.
#[derive(Debug, Default, Clone, Copy)]
pub struct Debounce {
    first: Option<Instant>,
    last: Option<Instant>,
}

impl Debounce {
    /// A change at `now`.
    pub fn changed(&mut self, now: Instant) {
        self.first.get_or_insert(now);
        self.last = Some(now);
    }

    /// How long until a pass is due; `None` with no change waiting.
    #[must_use]
    pub fn wait(&self, now: Instant) -> Option<Duration> {
        let (first, last) = (self.first?, self.last?);
        let due = (last + SETTLE).min(first + SETTLE_MAX);
        Some(due.saturating_duration_since(now))
    }

    /// The pass ran.
    pub fn fired(&mut self) {
        *self = Self::default();
    }
}

/// Starts the thread unless it runs; wakes it either way.
pub(crate) fn ensure(feeds: &Arc<Feeds>, state: &Arc<AppState>) {
    let mut wake = lock(&feeds.wake);
    if let Some(sender) = wake.as_ref()
        && sender.send(Wake::Nudge).is_ok()
    {
        return;
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    let feeds_weak = Arc::downgrade(feeds);
    let state_weak = Arc::downgrade(state);
    let events = sender.clone();
    let started = std::thread::Builder::new()
        .name("conversation-feeds".to_owned())
        .spawn(move || run(&feeds_weak, &state_weak, &receiver, events));
    if started.is_ok() {
        *wake = Some(sender);
    }
}

/// Wakes the thread (a follow ended: its folders may go).
pub(crate) fn nudge(feeds: &Feeds) {
    if let Some(sender) = lock(&feeds.wake).as_ref() {
        let _ = sender.send(Wake::Nudge);
    }
}

fn run(
    feeds: &Weak<Feeds>,
    state: &Weak<AppState>,
    receiver: &Receiver<Wake>,
    events: Sender<Wake>,
) {
    let mut folders = FolderWake::new(move |paths| {
        let _ = events.send(Wake::Changed(paths));
    });
    let mut debounce = Debounce::default();
    let mut names: BTreeSet<std::ffi::OsString> = BTreeSet::new();
    loop {
        let Some(feeds_now) = feeds.upgrade() else {
            return;
        };
        let (any_followed, any_idle, behind) = feeds_now.activity();
        drop(feeds_now);
        let now = Instant::now();
        let wait = if behind {
            Some(Duration::ZERO)
        } else if let Some(wait) = debounce.wait(now) {
            Some(wait)
        } else if any_followed {
            // poll: a stat of each followed file every second, only while
            // a page shows one; folder events wake it sooner.
            Some(FALLBACK)
        } else if any_idle {
            Some(super::IDLE_KEEP)
        } else {
            None
        };
        let woke = match wait {
            Some(wait) => receiver.recv_timeout(wait),
            None => receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match woke {
            Ok(Wake::Changed(paths)) => {
                // By name: the watcher may spell a folder another way
                // (`/private/var` for `/var`).
                if paths
                    .iter()
                    .any(|path| path.file_name().is_some_and(|name| names.contains(name)))
                {
                    debounce.changed(Instant::now());
                }
                if debounce.wait(Instant::now()) != Some(Duration::ZERO) {
                    continue;
                }
            }
            Ok(Wake::Nudge) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        debounce.fired();
        let (Some(feeds_now), Some(state_now)) = (feeds.upgrade(), state.upgrade()) else {
            return;
        };
        let enabled = crate::features::conversations::last_messages::enabled(&state_now);
        let resolve = |key: &FeedKey, current: &std::path::Path| -> Option<PathBuf> {
            if key.subagent.is_some() {
                Some(current.to_path_buf())
            } else {
                state_now
                    .conversations
                    .path_of(&key.account, &key.conversation)
            }
        };
        feeds_now.pass(Instant::now(), enabled, &resolve);
        let followed = feeds_now.followed_paths();
        names = followed
            .iter()
            .filter_map(|path| path.file_name().map(std::ffi::OsStr::to_os_string))
            .collect();
        let parents: BTreeSet<PathBuf> = followed
            .iter()
            .filter_map(|path| path.parent().map(std::path::Path::to_path_buf))
            .collect();
        folders.watch(&parents);
    }
}
