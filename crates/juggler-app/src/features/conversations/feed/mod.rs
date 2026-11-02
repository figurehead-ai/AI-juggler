//! 0.3.5 (ADR-0024): the conversations a page follows, as feeds.
//!
//! A page that shows a conversation follows it: it's given the latest
//! turns at once ([`Feeds::follow`]) and then only what changes, on its
//! own channel (per call, T3), as turns new or grown under stable ids.
//! One [`Feed`] per conversation (or subagent) is shared by every page
//! that shows it. It holds the transcript's tail and the turns read so
//! far (redacted as they are read, D-14), in memory only (D-37): nothing
//! here writes a file, a setting or a log line. A feed nobody follows is
//! let go a minute later, and at most a few are kept (by the machine's
//! memory tier).
//!
//! A follow is a lease: the page says it still follows every half
//! minute ([`Feeds::keep`]); a page that went away without unfollowing
//! (reloaded, closed) stops being sent anything when its lease runs out.
//!
//! Changes are read by one thread ([`watch`]): woken by the folders of
//! the followed files changing, with a check every second as the
//! fallback, batched so a burst of appends is one change. A file that is
//! replaced, made shorter or swapped for another (a resumed Codex thread
//! writes a new rollout) is read again from its end and sent as a
//! `reset`; one stored compressed now has ended; a file gone keeps what
//! the page shows and says so.

pub mod dto;
pub(crate) mod watch;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use juggler_adapters::conversation_feed::{Lines, Scrub, TurnLog};
use juggler_adapters::last_turns::{Back, scan_back};
use juggler_adapters::transcript::is_compressed;
use juggler_adapters::transcript_tail::TranscriptTail;

use dto::{
    ConversationDeltaInfo, ConversationSnapshotInfo, EarlierPageInfo, layout_name, turn_info,
};

/// The most turns a snapshot or a page holds.
pub const PAGE_MAX: usize = 60;

/// Turns a page gets unless it asks for fewer.
pub const PAGE_DEFAULT: usize = 30;

/// The most turns one change carries; more is sent as a `reset`.
pub const DELTA_MAX: usize = 100;

/// The most bytes one pass reads of one file (more: the next pass).
pub const READ_BUDGET: u64 = 16 * 1024 * 1024;

/// Past this much unread (a long sleep), the file is read again from
/// its end instead of caught up with.
pub const RESEED_GAP: u64 = 64 * 1024 * 1024;

/// How long a follow lasts unless the page says it still follows.
pub const LEASE: Duration = Duration::from_secs(90);

/// How long a feed nobody follows is kept.
pub const IDLE_KEEP: Duration = Duration::from_secs(60);

/// Feeds kept on a machine of the middle memory tier (4/8/16 by tier).
pub const FEEDS_KEPT: usize = 8;

/// The most feeds held at all: past it the least recently used goes,
/// even when followed (its pages hear `closed` and follow again).
pub const FEEDS_MAX: usize = 32;

/// Where a page's changes go.
pub trait DeltaSink: Send + Sync {
    /// Sends one change; false when the page can't be reached.
    fn send(&self, delta: ConversationDeltaInfo) -> bool;
}

impl DeltaSink for tauri::ipc::Channel<ConversationDeltaInfo> {
    fn send(&self, delta: ConversationDeltaInfo) -> bool {
        tauri::ipc::Channel::send(self, delta).is_ok()
    }
}

/// Which conversation a feed reads.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeedKey {
    /// The account.
    pub account: String,
    /// The conversation.
    pub conversation: String,
    /// One of its subagents, when it's that one's conversation.
    pub subagent: Option<String>,
}

impl FeedKey {
    fn lines(&self) -> Lines {
        if self.subagent.is_some() {
            Lines::Subagent
        } else {
            Lines::Session
        }
    }
}

/// D-14: the patterns a context pack's redaction uses.
struct Redactor;

impl Scrub for Redactor {
    fn scrub(&self, pieces: &mut [String]) -> u32 {
        let (clean, counts) = juggler_handover::redaction::strip_segments(
            pieces,
            true,
            &std::collections::BTreeSet::new(),
        );
        for (piece, clean) in pieces.iter_mut().zip(clean) {
            *piece = clean;
        }
        counts.iter().map(|count| count.count).sum()
    }
}

struct Subscriber {
    id: u64,
    sink: Arc<dyn DeltaSink>,
    until: Instant,
}

/// One followed conversation (see the module docs).
pub struct Feed {
    key: FeedKey,
    path: PathBuf,
    generation: u32,
    resets: u32,
    revision: u64,
    tail: TranscriptTail<TurnLog>,
    status: Option<&'static str>,
    subscribers: Vec<Subscriber>,
    idle_since: Option<Instant>,
    used: Instant,
    /// The last pass read up to its budget: more is waiting.
    behind: bool,
}

/// The generation of a read of `path` (file `file`, read again `resets`
/// times): ids of one generation are offsets into one file's bytes.
#[must_use]
pub fn generation_of(path: &Path, file: u64, resets: u32) -> u32 {
    // FNV-1a: stable across runs, so a feed let go and made again for
    // the same file gives the page the same ids.
    let mut hash: u32 = 0x811c_9dc5;
    let bytes = path.to_string_lossy();
    for byte in bytes
        .as_bytes()
        .iter()
        .chain(&file.to_le_bytes())
        .chain(&resets.to_le_bytes())
    {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// The log a conversation is read with: its lines, redacted.
pub(crate) fn template(lines: Lines) -> TurnLog {
    TurnLog::new(lines).with_scrub(Arc::new(Redactor))
}

impl Feed {
    /// Reads `path`'s latest turns for `key`; why not, when it can't.
    fn open(key: FeedKey, path: PathBuf, resets: u32, now: Instant) -> Result<Self, &'static str> {
        let page = match scan_back(&path, None, PAGE_MAX, &template(key.lines())) {
            Ok(Back::Page(page)) => page,
            Ok(Back::Compressed) => return Err("compressed"),
            Err(_) => return Err("unreadable"),
        };
        Ok(Self {
            generation: generation_of(&path, page.file, resets),
            tail: TranscriptTail::resume(page.log, page.end, Some(page.file)),
            key,
            path,
            resets,
            revision: 0,
            status: None,
            subscribers: Vec::new(),
            idle_since: Some(now),
            used: now,
            behind: false,
        })
    }

    fn layout(&self) -> &'static str {
        layout_name(self.tail.log().layout())
    }

    /// The latest `max` turns on the wire, and where they start.
    fn latest(&self, max: usize) -> (Vec<dto::TurnInfo>, Option<u64>) {
        let log = self.tail.log();
        let turns = log.turns();
        let skip = turns.len().saturating_sub(max);
        let layout = log.layout();
        let last = turns.len().saturating_sub(1);
        let shown: Vec<dto::TurnInfo> = turns
            .iter()
            .enumerate()
            .skip(skip)
            .map(|(index, turn)| {
                turn_info(
                    turn,
                    self.generation,
                    index == last && log.last_open(),
                    layout,
                )
            })
            .collect();
        let earlier = if skip > 0 {
            shown.first().map(|turn| turn.offset)
        } else {
            log.earlier()
        };
        (shown, earlier)
    }

    fn snapshot(&self, subscription: u64, max: usize) -> ConversationSnapshotInfo {
        let (turns, earlier) = self.latest(max);
        ConversationSnapshotInfo {
            state: "shown".to_owned(),
            subscription: Some(subscription),
            generation: self.generation,
            revision: self.revision,
            layout: self.layout().to_owned(),
            turns,
            earlier,
            status: self.status.map(str::to_owned),
            skipped: self.tail.log().skipped(),
        }
    }

    /// A change of `kind` (its revision counted).
    fn change(
        &mut self,
        kind: &str,
        upserts: Vec<dto::TurnInfo>,
        dropped: Option<u64>,
        earlier: Option<u64>,
    ) -> ConversationDeltaInfo {
        self.revision += 1;
        ConversationDeltaInfo {
            subscription: 0,
            generation: self.generation,
            revision: self.revision,
            kind: kind.to_owned(),
            upserts,
            dropped_before: dropped,
            earlier,
            layout: self.layout().to_owned(),
            status: self.status.map(str::to_owned),
            skipped: self.tail.log().skipped(),
        }
    }

    fn set_status(&mut self, status: Option<&'static str>) -> Option<ConversationDeltaInfo> {
        if self.status == status {
            return None;
        }
        self.status = status;
        Some(self.change("state", Vec::new(), None, None))
    }

    /// Reads the file at `path` again from its end: a `reset`.
    fn reseed(&mut self, path: PathBuf, now: Instant) -> Option<ConversationDeltaInfo> {
        match Self::open(self.key.clone(), path, self.resets + 1, now) {
            Ok(fresh) => {
                self.path = fresh.path;
                self.generation = fresh.generation;
                self.resets = fresh.resets;
                self.tail = fresh.tail;
                self.status = None;
                self.behind = false;
                let (turns, earlier) = self.latest(PAGE_MAX);
                Some(self.change("reset", turns, None, earlier))
            }
            Err("compressed") => self.set_status(Some("ended")),
            Err(_) => self.set_status(Some("unreadable")),
        }
    }

    /// What changed in the file since the last read (see the module
    /// docs); `resolve` names the file the conversation is now.
    fn refresh(
        &mut self,
        resolve: &dyn Fn(&FeedKey, &Path) -> Option<PathBuf>,
        now: Instant,
    ) -> Option<ConversationDeltaInfo> {
        if self.status == Some("ended") {
            return None;
        }
        let Some(path) = resolve(&self.key, &self.path) else {
            return self.set_status(Some("gone"));
        };
        if path != self.path {
            return self.reseed(path, now);
        }
        if is_compressed(&path) {
            return self.set_status(Some("ended"));
        }
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return self.set_status(Some("gone"));
            }
            Err(_) => return self.set_status(Some("unreadable")),
        };
        if self.tail.replaced_by(&metadata)
            || metadata.len().saturating_sub(self.tail.offset()) > RESEED_GAP
        {
            return self.reseed(path, now);
        }
        let back = self.set_status(None);
        if metadata.len() == self.tail.offset() {
            return back;
        }
        match self.tail.read_within(&path, READ_BUDGET) {
            Ok((caught_up, _)) => self.behind = !caught_up,
            Err(_) => return self.set_status(Some("unreadable")),
        }
        let delta = self.tail.log_mut().take_delta();
        if delta.is_empty() {
            return back;
        }
        if delta.upserts.len() > DELTA_MAX {
            let (turns, earlier) = self.latest(PAGE_MAX);
            return Some(self.change("reset", turns, None, earlier));
        }
        let log = self.tail.log();
        let last = log.turns().back().map(|turn| turn.offset);
        let layout = log.layout();
        let upserts = delta
            .upserts
            .iter()
            .map(|turn| {
                turn_info(
                    turn,
                    self.generation,
                    Some(turn.offset) == last && log.last_open(),
                    layout,
                )
            })
            .collect();
        Some(self.change("append", upserts, delta.dropped_before, None))
    }

    /// Sends `delta` to every follower; drops those that can't be reached.
    fn deliver(&mut self, delta: &ConversationDeltaInfo) {
        self.subscribers.retain(|subscriber| {
            subscriber.sink.send(ConversationDeltaInfo {
                subscription: subscriber.id,
                ..delta.clone()
            })
        });
    }

    fn followed(&self) -> bool {
        !self.subscribers.is_empty()
    }
}

#[derive(Default)]
struct Inner {
    feeds: HashMap<FeedKey, Arc<Mutex<Feed>>>,
    subscriptions: HashMap<u64, FeedKey>,
}

/// The feeds (T2: the feature's own state, `app.manage`d).
#[derive(Default)]
pub struct Feeds {
    inner: Mutex<Inner>,
    next: AtomicU64,
    pub(crate) wake: Mutex<Option<std::sync::mpsc::Sender<watch::Wake>>>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // What these locks guard is rebuilt from the files at worst.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Feeds {
    fn feed_of(&self, subscription: u64) -> Option<Arc<Mutex<Feed>>> {
        let inner = lock(&self.inner);
        let key = inner.subscriptions.get(&subscription)?;
        inner.feeds.get(key).cloned()
    }

    /// Follows `key` (its transcript at `path`): its latest `max` turns
    /// now, then each change sent to `sink`. `kept` is how many feeds
    /// nobody follows may stay.
    pub fn follow(
        &self,
        key: FeedKey,
        path: &Path,
        max: usize,
        sink: Arc<dyn DeltaSink>,
        kept: usize,
        now: Instant,
    ) -> ConversationSnapshotInfo {
        let existing = lock(&self.inner).feeds.get(&key).cloned();
        let feed = if let Some(feed) = existing {
            feed
        } else {
            let fresh = match Feed::open(key.clone(), path.to_path_buf(), 0, now) {
                Ok(feed) => feed,
                Err(state) => return without(state),
            };
            let mut inner = lock(&self.inner);
            // Another page may have opened it meanwhile: share that one.
            Arc::clone(
                inner
                    .feeds
                    .entry(key.clone())
                    .or_insert_with(|| Arc::new(Mutex::new(fresh))),
            )
        };
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let snapshot = {
            let mut feed = lock(&feed);
            // Up to date before it's shown (the others hear the change).
            let resolve = |_: &FeedKey, _: &Path| Some(path.to_path_buf());
            if let Some(delta) = feed.refresh(&resolve, now) {
                feed.deliver(&delta);
            }
            feed.subscribers.push(Subscriber {
                id,
                sink,
                until: now + LEASE,
            });
            feed.idle_since = None;
            feed.used = now;
            feed.snapshot(id, max.clamp(1, PAGE_MAX))
        };
        let mut inner = lock(&self.inner);
        inner.subscriptions.insert(id, key);
        evict(&mut inner, kept, now);
        snapshot
    }

    /// Stops sending `subscription`'s page anything.
    pub fn unfollow(&self, subscription: u64, now: Instant) {
        let feed = self.feed_of(subscription);
        lock(&self.inner).subscriptions.remove(&subscription);
        if let Some(feed) = feed {
            let mut feed = lock(&feed);
            feed.subscribers
                .retain(|subscriber| subscriber.id != subscription);
            if !feed.followed() {
                feed.idle_since = Some(now);
            }
        }
    }

    /// The page still follows `subscriptions`: their leases renewed.
    /// Those that ended (to follow again).
    pub fn keep(&self, subscriptions: &[u64], now: Instant) -> Vec<u64> {
        let mut gone = Vec::new();
        for id in subscriptions {
            let renewed = self.feed_of(*id).is_some_and(|feed| {
                let mut feed = lock(&feed);
                let found = feed
                    .subscribers
                    .iter_mut()
                    .find(|subscriber| subscriber.id == *id);
                found
                    .map(|subscriber| subscriber.until = now + LEASE)
                    .is_some()
            });
            if !renewed {
                gone.push(*id);
            }
        }
        gone
    }

    /// The `max` turns before `before` in `subscription`'s conversation.
    #[must_use]
    pub fn earlier(&self, subscription: u64, before: u64, max: usize) -> EarlierPageInfo {
        let Some(feed) = self.feed_of(subscription) else {
            return EarlierPageInfo::without("gone");
        };
        // Read with the feed let go: a long scan never holds up its
        // changes.
        let (path, generation, log) = {
            let feed = lock(&feed);
            (feed.path.clone(), feed.generation, feed.tail.log().fresh())
        };
        match scan_back(&path, Some(before), max.clamp(1, PAGE_MAX), &log) {
            Ok(Back::Page(page)) => {
                let layout = page.log.layout();
                EarlierPageInfo {
                    state: "shown".to_owned(),
                    generation,
                    turns: page
                        .log
                        .turns()
                        .iter()
                        .map(|turn| turn_info(turn, generation, false, layout))
                        .collect(),
                    earlier: page.log.earlier(),
                }
            }
            Ok(Back::Compressed) => EarlierPageInfo::without("compressed"),
            Err(_) => EarlierPageInfo::without("unreadable"),
        }
    }

    /// One pass: each followed feed's changes read and sent, leases that
    /// ran out let go, feeds idle past [`IDLE_KEEP`] dropped. Off in
    /// Settings (`enabled` false): every page hears `off` and all is let
    /// go at once.
    pub fn pass(
        &self,
        now: Instant,
        enabled: bool,
        resolve: &dyn Fn(&FeedKey, &Path) -> Option<PathBuf>,
    ) {
        let feeds: Vec<Arc<Mutex<Feed>>> = lock(&self.inner).feeds.values().cloned().collect();
        if !enabled {
            for feed in &feeds {
                let mut feed = lock(feed);
                feed.status = None;
                let delta = feed.change("state", Vec::new(), None, None);
                feed.deliver(&ConversationDeltaInfo {
                    status: Some("off".to_owned()),
                    ..delta
                });
            }
            *lock(&self.inner) = Inner::default();
            return;
        }
        for feed in &feeds {
            let mut feed = lock(feed);
            feed.subscribers.retain(|subscriber| subscriber.until > now);
            if !feed.followed() {
                feed.idle_since.get_or_insert(now);
                continue;
            }
            if let Some(delta) = feed.refresh(resolve, now) {
                feed.deliver(&delta);
            }
        }
        let mut inner = lock(&self.inner);
        inner.feeds.retain(|_, feed| {
            let feed = lock(feed);
            feed.followed()
                || feed
                    .idle_since
                    .is_none_or(|since| now.duration_since(since) < IDLE_KEEP)
        });
        let live: std::collections::HashSet<u64> = inner
            .feeds
            .values()
            .flat_map(|feed| {
                lock(feed)
                    .subscribers
                    .iter()
                    .map(|s| s.id)
                    .collect::<Vec<_>>()
            })
            .collect();
        inner.subscriptions.retain(|id, _| live.contains(id));
    }

    /// The files followed now.
    #[must_use]
    pub fn followed_paths(&self) -> BTreeSet<PathBuf> {
        lock(&self.inner)
            .feeds
            .values()
            .filter_map(|feed| {
                let feed = lock(feed);
                feed.followed().then(|| feed.path.clone())
            })
            .collect()
    }

    /// Whether any feed is followed, any is kept idle, any is behind.
    pub(crate) fn activity(&self) -> (bool, bool, bool) {
        let inner = lock(&self.inner);
        let mut followed = false;
        let mut idle = false;
        let mut behind = false;
        for feed in inner.feeds.values() {
            let feed = lock(feed);
            followed |= feed.followed();
            idle |= !feed.followed();
            behind |= feed.followed() && feed.behind;
        }
        (followed, idle, behind)
    }

    /// How many feeds it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        lock(&self.inner).feeds.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Feeds nobody follows past `kept` go, the longest idle first; past
/// [`FEEDS_MAX`] the least recently used goes, followed or not.
fn evict(inner: &mut Inner, kept: usize, now: Instant) {
    let mut idle: Vec<(Instant, FeedKey)> = inner
        .feeds
        .iter()
        .filter_map(|(key, feed)| {
            let feed = lock(feed);
            (!feed.followed()).then(|| (feed.idle_since.unwrap_or(now), key.clone()))
        })
        .collect();
    idle.sort();
    let mut over = idle.len().saturating_sub(kept);
    for (_, key) in idle {
        if over == 0 {
            break;
        }
        inner.feeds.remove(&key);
        over -= 1;
    }
    while inner.feeds.len() > FEEDS_MAX {
        let oldest = inner
            .feeds
            .iter()
            .min_by_key(|(_, feed)| lock(feed).used)
            .map(|(key, _)| key.clone());
        let Some(key) = oldest else {
            break;
        };
        if let Some(feed) = inner.feeds.remove(&key) {
            let mut feed = lock(&feed);
            let delta = feed.change("state", Vec::new(), None, None);
            feed.deliver(&ConversationDeltaInfo {
                status: Some("closed".to_owned()),
                ..delta
            });
        }
    }
}

/// Core of `conversations_follow` and `subagents_follow`: `key`'s
/// conversation, its transcript at `path` (none: there is none on this
/// Mac), followed for `sink`. Nothing is read when the setting is off.
pub fn follow_impl(
    state: &Arc<crate::state::AppState>,
    feeds: &Arc<Feeds>,
    key: FeedKey,
    path: Option<PathBuf>,
    max: Option<u32>,
    sink: Arc<dyn DeltaSink>,
) -> ConversationSnapshotInfo {
    if !super::last_messages::enabled(state) {
        return without("off");
    }
    let Some(path) = path else {
        return without("none");
    };
    let max = max
        .and_then(|max| usize::try_from(max).ok())
        .unwrap_or(PAGE_DEFAULT);
    let kept = state.cache.gate().tier().scale(FEEDS_KEPT);
    let snapshot = feeds.follow(key, &path, max, sink, kept, Instant::now());
    if snapshot.subscription.is_some() {
        watch::ensure(feeds, state);
    }
    snapshot
}

/// Core of `conversations_unfollow`.
pub fn unfollow_impl(feeds: &Feeds, subscription: u64) {
    feeds.unfollow(subscription, Instant::now());
    watch::nudge(feeds);
}

/// A snapshot that says why there's nothing to follow.
#[must_use]
pub fn without(state: &str) -> ConversationSnapshotInfo {
    ConversationSnapshotInfo {
        state: state.to_owned(),
        subscription: None,
        generation: 0,
        revision: 0,
        layout: "stretches".to_owned(),
        turns: Vec::new(),
        earlier: None,
        status: None,
        skipped: 0,
    }
}

#[cfg(test)]
mod tests;
