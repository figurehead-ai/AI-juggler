//! One page of the feed (D-118, AJ-E3702 R1): every source read past the
//! page's start, at most a page each, merged newest first and cut to the
//! page. The cursor names the last item's place ([`FeedKey`]), so items
//! written meanwhile never shift, repeat or skip what was shown. Never a
//! load-all: a page reads at most `limit` rows from each source.

use juggler_core::storage::Storage;
use juggler_core::storage::feed::{FeedKey, FeedScope, rank as feed_rank};

use super::dto::{FeedFiltersInfo, FeedItemInfo};
use super::items::{
    self, ENDINGS, MESSAGE_KINDS, MINE, TASK_CHANGES, in_chip, in_state, stored_for,
};
use crate::error::{AppError, AppResult};
use crate::paging::{Page, PageRequest, SortKey};

/// Items in a page unless asked (D-118: 30).
pub const FEED_PAGE: usize = 30;

/// What a read is asked for: the filters, resolved.
#[derive(Clone, Debug, Default)]
pub struct Ask {
    /// The chip's category; none: All.
    pub category: Option<String>,
    /// The project, pool or folders.
    pub scope: FeedScope,
    /// The user's own acts too.
    pub include_mine: bool,
    /// AJ-E3713 R5: one state chip (`running`, `finished`, `failed`).
    pub state: Option<String>,
    /// The sessions running now (the Running chip keeps their items);
    /// `None` when it isn't asked.
    pub running: Option<std::collections::HashSet<String>>,
}

impl Ask {
    /// The window's filters as a read asks them.
    #[must_use]
    pub fn of(filters: &FeedFiltersInfo) -> Self {
        Self {
            category: filters
                .category
                .clone()
                .filter(|chip| !chip.is_empty() && chip != "all"),
            scope: FeedScope {
                project: filters.project.clone(),
                pool: filters.pool.clone(),
                folders: filters.folders.clone(),
            },
            include_mine: filters.include_mine,
            state: filters
                .state
                .clone()
                .filter(|state| !state.is_empty() && state != "all"),
            running: None,
        }
    }

    /// The same ask with the sessions running now (the Running chip).
    #[must_use]
    pub fn with_running(mut self, running: impl IntoIterator<Item = String>) -> Self {
        self.running = Some(running.into_iter().collect());
        self
    }

    fn chip(&self) -> Option<&str> {
        self.category.as_deref()
    }

    /// Whether an item passes what the reads can't say in SQL (a kind's
    /// chip that shares its stored word with another, the user's acts).
    #[must_use]
    pub fn keeps(&self, item: &FeedItemInfo) -> bool {
        (self.include_mine || !item.mine)
            && self.chip().is_none_or(|chip| {
                item.category == chip || (chip != "needs" && in_chip(&item.kind, chip))
            })
            && self.state.as_deref().is_none_or(|state| match state {
                "running" => item.target.session_id.as_ref().is_some_and(|id| {
                    self.running
                        .as_ref()
                        .is_some_and(|running| running.contains(id))
                }),
                state => in_state(&item.kind, state),
            })
    }
}

/// An item with its place in the feed.
pub type Placed = (FeedKey, FeedItemInfo);

/// What a source read: its items, and whether it read all it was allowed
/// (so more may follow).
#[derive(Debug, Default)]
pub struct Read {
    /// The items, newest first.
    pub items: Vec<Placed>,
    /// It read its whole limit.
    pub full: bool,
}

impl Read {
    fn of<R>(rows: &[R], limit: usize, item: impl Fn(&R) -> Option<Placed>) -> Self {
        Self {
            full: rows.len() >= limit,
            items: rows.iter().filter_map(item).collect(),
        }
    }
}

/// One journal the feed reads. F2 and F3 add theirs as one more source
/// here; nothing else changes.
pub trait FeedSource {
    /// Up to `limit` items strictly after `before`, newest first.
    ///
    /// # Errors
    ///
    /// The typed storage error.
    fn read(
        &self,
        storage: &Storage,
        before: Option<FeedKey>,
        ask: &Ask,
        limit: usize,
    ) -> AppResult<Read>;
}

struct Notices;
struct Lifecycle;
struct Switches;
struct Tasks;
struct Messages;

impl FeedSource for Notices {
    fn read(
        &self,
        storage: &Storage,
        before: Option<FeedKey>,
        ask: &Ask,
        limit: usize,
    ) -> AppResult<Read> {
        let chip = ask.chip();
        // Credits are quota notices with credits: one stored word, two
        // kinds; the item's own chip decides (`Ask::keeps`).
        let earlier: Vec<String> = items::EARLIER_KINDS
            .iter()
            .filter(|(_, feed)| {
                chip.is_none_or(|chip| {
                    in_chip(feed, chip) || (*feed == "quota" && in_chip("credits", chip))
                })
            })
            .map(|(stored, _)| (*stored).to_owned())
            .collect();
        let answered: Vec<String> = if chip.is_none_or(|chip| in_chip("answered", chip)) {
            items::ANSWERED_NEEDS
                .iter()
                .map(|kind| (*kind).to_owned())
                .collect()
        } else {
            Vec::new()
        };
        let rows = storage
            .feed()
            .notices(before, &ask.scope, &earlier, &answered, limit)?;
        Ok(Read::of(&rows, limit, |row| {
            items::of_notice(row).map(|item| (row.key, item))
        }))
    }
}

impl FeedSource for Lifecycle {
    fn read(
        &self,
        storage: &Storage,
        before: Option<FeedKey>,
        ask: &Ask,
        limit: usize,
    ) -> AppResult<Read> {
        let mut prefixes = stored_for(ENDINGS, ask.chip());
        if ask.include_mine {
            prefixes.extend(stored_for(MINE, ask.chip()));
        }
        let rows = storage
            .feed()
            .lifecycle(before, &ask.scope, &prefixes, limit)?;
        Ok(Read::of(&rows, limit, |row| {
            items::of_lifecycle(row).map(|item| (row.key, item))
        }))
    }
}

impl FeedSource for Switches {
    fn read(
        &self,
        storage: &Storage,
        before: Option<FeedKey>,
        ask: &Ask,
        limit: usize,
    ) -> AppResult<Read> {
        let wanted = ask
            .chip()
            .is_none_or(|chip| in_chip("switch", chip) || in_chip("switch_failed", chip));
        if !wanted {
            return Ok(Read::default());
        }
        let rows = storage.feed().switches(before, &ask.scope, limit)?;
        Ok(Read::of(&rows, limit, |row| {
            Some((row.key, items::of_switch(row)))
        }))
    }
}

impl FeedSource for Tasks {
    fn read(
        &self,
        storage: &Storage,
        before: Option<FeedKey>,
        ask: &Ask,
        limit: usize,
    ) -> AppResult<Read> {
        let to = stored_for(TASK_CHANGES, ask.chip());
        let rows = storage.feed().task_events(before, &ask.scope, &to, limit)?;
        Ok(Read::of(&rows, limit, |row| {
            items::of_task(row).map(|item| (row.key, item))
        }))
    }
}

impl FeedSource for Messages {
    fn read(
        &self,
        storage: &Storage,
        before: Option<FeedKey>,
        ask: &Ask,
        limit: usize,
    ) -> AppResult<Read> {
        if ask.chip().is_some_and(|chip| !in_chip("message", chip)) {
            return Ok(Read::default());
        }
        let kinds: Vec<String> = MESSAGE_KINDS
            .iter()
            .map(|kind| (*kind).to_owned())
            .collect();
        let rows = storage.feed().messages(before, &ask.scope, &kinds, limit)?;
        Ok(Read::of(&rows, limit, |row| {
            Some((row.key, items::of_message(row)))
        }))
    }
}

/// The journals F1 reads, in rank order, and the agent digests
/// (AJ-E3712).
pub const SOURCES: &[&dyn FeedSource] = &[
    &Notices,
    &Lifecycle,
    &Switches,
    &Tasks,
    &Messages,
    &crate::features::digest::source::SOURCE,
];

/// An item's cursor: its key's numbers (time, rank and row id, each
/// negated so the list's ascending order is newest first) and its id.
#[must_use]
pub fn cursor_of(key: FeedKey, id: &str) -> String {
    SortKey::new([-key.at_ms, -key.source, -key.rowid], id).cursor()
}

/// The key a cursor names.
///
/// # Errors
///
/// [`crate::error::AppError::Validation`] for a cursor this app didn't make.
pub fn key_of(cursor: &str) -> AppResult<FeedKey> {
    let key = SortKey::parse(cursor)?;
    match key.ranks.as_slice() {
        [at, source, rowid] => Ok(FeedKey {
            at_ms: -at,
            source: -source,
            rowid: -rowid,
        }),
        _ => Err(AppError::Validation {
            subject: "page",
            message: "That list moved on; read it again from the top.".to_owned(),
        }),
    }
}

/// One page: every source read past the start, merged, cut to `limit`.
/// `more` are items read elsewhere (the conversation index: it isn't in
/// this storage), already past the start.
///
/// # Errors
///
/// A bad cursor; the typed storage error.
pub fn page(
    storage: &Storage,
    request: &PageRequest,
    filters: &FeedFiltersInfo,
    more: Read,
) -> AppResult<Page<FeedItemInfo>> {
    page_for(storage, request, &Ask::of(filters), more)
}

/// [`page`] for an ask already resolved (the Running chip's sessions).
///
/// # Errors
///
/// As [`page`].
pub fn page_for(
    storage: &Storage,
    request: &PageRequest,
    ask: &Ask,
    more: Read,
) -> AppResult<Page<FeedItemInfo>> {
    let limit = request.limit.map_or(FEED_PAGE, |_| request.limit());
    let before = request.cursor.as_deref().map(key_of).transpose()?;
    let ask = ask.clone();
    let mut full = more.full;
    let mut placed = more.items;
    for source in SOURCES {
        let read = source.read(storage, before, &ask, limit)?;
        full |= read.full;
        placed.extend(read.items);
    }
    placed.sort_by_key(|(key, _)| std::cmp::Reverse(*key));
    let over = placed.len() > limit;
    placed.truncate(limit);
    // The cursor is the last item read, kept or not, so a page whose items
    // a filter drops still moves on.
    let next_cursor = if over || full {
        placed.last().map(|(key, item)| cursor_of(*key, &item.id))
    } else {
        None
    };
    Ok(Page {
        items: placed
            .into_iter()
            .map(|(_, item)| item)
            .filter(|item| ask.keeps(item))
            .collect(),
        next_cursor,
        total: None,
    })
}

/// The rank of the conversation index's items (after every journal).
pub const CONVERSATIONS: i64 = feed_rank::MESSAGES + 1;
