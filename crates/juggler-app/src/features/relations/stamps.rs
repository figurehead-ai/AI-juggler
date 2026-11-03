//! AJ-E3102: what the relation counts are made from ([`Source`]), the
//! change events the app already has ([`Change`]), and which sources each
//! one makes stale ([`Change::invalidates`]). A count is made from the
//! sources' stamps as they were; any stamp moved since means it's read
//! again (the database's own stamps, moved by triggers, back the events
//! up: `juggler_core::storage::RelationsRepo::stamps`).

use std::sync::atomic::{AtomicU64, Ordering};

/// What the counts are read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    /// The sessions AI Juggler ran, their state and lifecycle, and which
    /// conversation each is on.
    Sessions,
    /// The tools' folders, their launch settings and the Account each
    /// folder's sign-in was read as.
    Folders,
    /// The projects and the worktrees' homes (a conversation's project is
    /// worked out from them).
    Projects,
    /// The tools' own files (the conversation index) and which of them
    /// are open now.
    Conversations,
    /// Where each desktop App was last seen showing a session.
    Places,
}

/// How many sources there are.
pub const SOURCES: usize = 5;

impl Source {
    /// Every source.
    pub const ALL: [Self; SOURCES] = [
        Self::Sessions,
        Self::Folders,
        Self::Projects,
        Self::Conversations,
        Self::Places,
    ];

    /// Its place in a [`Snapshot`].
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Sessions => 0,
            Self::Folders => 1,
            Self::Projects => 2,
            Self::Conversations => 3,
            Self::Places => 4,
        }
    }
}

/// A change the app tells of. The first ones are its events (their names
/// beside them, [`Change::event`]); the last two are told inside the
/// backend, where the change is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Change {
    /// `session://updated`: a session started, ended, moved, was archived
    /// or learned its conversation (a start may add its project).
    SessionsUpdated,
    /// `session://resources`: the 5 s resource tick.
    SessionsResources,
    /// `session://quota`: the 30 s quota tick.
    SessionsQuota,
    /// `conversations://changed` (`index`): the tools' files changed (new
    /// folders become projects with them).
    ConversationsIndex,
    /// `conversations://changed` (`live`): which conversations are open.
    ConversationsLive,
    /// `places-changed`: a desktop App seen, started or forgotten.
    PlacesChanged,
    /// `tasks://changed`.
    TasksChanged,
    /// `messages://changed`.
    MessagesChanged,
    /// `processes://changed`.
    ProcessesChanged,
    /// `notifications://changed`.
    NoticesChanged,
    /// `runtime://changed`.
    RuntimeChanged,
    /// `machine://this-mac`: This Mac's 5 s reading.
    ThisMac,
    /// A folder's sign-in was read as another Account; a folder or its
    /// launch settings added or removed.
    AccountsChanged,
    /// A project added, removed or moved.
    ProjectsChanged,
}

impl Change {
    /// Every change.
    pub const ALL: [Self; 14] = [
        Self::SessionsUpdated,
        Self::SessionsResources,
        Self::SessionsQuota,
        Self::ConversationsIndex,
        Self::ConversationsLive,
        Self::PlacesChanged,
        Self::TasksChanged,
        Self::MessagesChanged,
        Self::ProcessesChanged,
        Self::NoticesChanged,
        Self::RuntimeChanged,
        Self::ThisMac,
        Self::AccountsChanged,
        Self::ProjectsChanged,
    ];

    /// The event the UI hears for it, when it is one.
    #[must_use]
    pub const fn event(self) -> Option<&'static str> {
        Some(match self {
            Self::SessionsUpdated => crate::EVENT_SESSIONS_UPDATED,
            Self::SessionsResources => crate::EVENT_SESSIONS_RESOURCES,
            Self::SessionsQuota => crate::EVENT_SESSIONS_QUOTA,
            Self::ConversationsIndex | Self::ConversationsLive => {
                crate::features::conversations::EVENT_CHANGED
            }
            Self::PlacesChanged => crate::features::places::EVENT_CHANGED,
            Self::TasksChanged => crate::features::tasks::EVENT_CHANGED,
            Self::MessagesChanged => crate::features::messages::EVENT_CHANGED,
            Self::ProcessesChanged => crate::features::processes::EVENT_CHANGED,
            Self::NoticesChanged => crate::features::notifications::views::EVENT_CHANGED,
            Self::RuntimeChanged => crate::features::runtime::EVENT_CHANGED,
            Self::ThisMac => crate::EVENT_THIS_MAC,
            Self::AccountsChanged | Self::ProjectsChanged => return None,
        })
    }

    /// The sources it makes stale. Resource, quota, process, task,
    /// message and notice changes move no session between pages, so no
    /// count is read again for them.
    #[must_use]
    pub const fn invalidates(self) -> &'static [Source] {
        match self {
            Self::SessionsUpdated => &[Source::Sessions, Source::Projects],
            Self::ConversationsIndex => &[Source::Conversations, Source::Projects],
            Self::ConversationsLive => &[Source::Conversations],
            Self::PlacesChanged => &[Source::Places],
            Self::AccountsChanged => &[Source::Folders],
            Self::ProjectsChanged => &[Source::Projects],
            Self::SessionsResources
            | Self::SessionsQuota
            | Self::TasksChanged
            | Self::MessagesChanged
            | Self::ProcessesChanged
            | Self::NoticesChanged
            | Self::RuntimeChanged
            | Self::ThisMac => &[],
        }
    }
}

/// Where each source stood when something was read.
pub type Snapshot = [u64; SOURCES];

/// One counter per source, moved by the changes; read without a lock.
#[derive(Debug, Default)]
pub struct Stamps {
    counters: [AtomicU64; SOURCES],
}

impl Stamps {
    /// `change` happened: every source it invalidates moves on.
    pub fn bump(&self, change: Change) {
        for source in change.invalidates() {
            self.counters[source.index()].fetch_add(1, Ordering::AcqRel);
        }
    }

    /// Where every source stands now.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        std::array::from_fn(|at| self.counters[at].load(Ordering::Acquire))
    }
}

/// Whether anything in `deps` moved between `then` and `now`.
#[must_use]
pub fn moved(deps: &[Source], then: &Snapshot, now: &Snapshot) -> bool {
    deps.iter()
        .any(|source| then[source.index()] != now[source.index()])
}
