//! Paged lists over IPC (0.1.5 review: Usage and History loaded hundreds
//! of rows at once and stayed empty for seconds).
//!
//! A command whose list can grow takes a [`PageRequest`] and answers a
//! [`Page`]: at most [`MAX_LIMIT`] rows ([`DEFAULT_LIMIT`] unless asked),
//! in a stable order, with the cursor of the next page. The cursor is a
//! keyset one: it names the last row's [`SortKey`], so rows added or
//! removed meanwhile never shift, repeat or skip what was already shown.
//! The UI reads pages with `usePagedList` (`ui/src/lib/paged.ts`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Rows in a page when the UI doesn't say.
pub const DEFAULT_LIMIT: usize = 50;

/// The most rows a page ever holds, whatever is asked.
pub const MAX_LIMIT: usize = 200;

/// Which page to read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PageRequest {
    /// The `next_cursor` of the page before; none for the first page.
    #[serde(default)]
    pub cursor: Option<String>,
    /// How many rows: [`DEFAULT_LIMIT`] when none, never more than
    /// [`MAX_LIMIT`].
    #[serde(default)]
    pub limit: Option<u32>,
}

impl PageRequest {
    /// The first page of `limit` rows.
    #[must_use]
    pub fn first(limit: usize) -> Self {
        Self {
            cursor: None,
            limit: u32::try_from(limit).ok(),
        }
    }

    /// The page after `cursor`, of `limit` rows.
    #[must_use]
    pub fn after(cursor: &str, limit: usize) -> Self {
        Self {
            cursor: Some(cursor.to_owned()),
            limit: u32::try_from(limit).ok(),
        }
    }

    /// The rows to read: between 1 and [`MAX_LIMIT`].
    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit
            .and_then(|limit| usize::try_from(limit).ok())
            .unwrap_or(DEFAULT_LIMIT)
            .clamp(1, MAX_LIMIT)
    }

    /// The key the page starts after.
    ///
    /// # Errors
    ///
    /// [`AppError::Validation`] for a cursor this app didn't make.
    pub fn start_after(&self) -> AppResult<Option<SortKey>> {
        self.cursor.as_deref().map(SortKey::parse).transpose()
    }
}

/// One page of a list.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct Page<T> {
    /// The page's rows, in the list's order.
    pub items: Vec<T>,
    /// Where the next page starts; none on the last page.
    pub next_cursor: Option<String>,
    /// How many rows the whole list holds, when known.
    #[ts(type = "number | null")]
    pub total: Option<u64>,
}

impl<T> Page<T> {
    /// An empty list.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            next_cursor: None,
            total: Some(0),
        }
    }

    /// Each row as another type (the page and its cursor kept).
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            next_cursor: self.next_cursor,
            total: self.total,
        }
    }
}

/// A row's place in its list: its numbers compared in turn (ascending;
/// a time negated sorts newest first), then its id, unique in the list.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SortKey {
    /// The ordering numbers, most significant first.
    pub ranks: Vec<i64>,
    /// The row's id: ties end here, so the order is total.
    pub id: String,
}

const CURSOR_PREFIX: &str = "k1";

impl SortKey {
    /// A key from its numbers and the row's id.
    #[must_use]
    pub fn new(ranks: impl Into<Vec<i64>>, id: impl Into<String>) -> Self {
        Self {
            ranks: ranks.into(),
            id: id.into(),
        }
    }

    /// The cursor that names this key (opaque to the UI).
    #[must_use]
    pub fn cursor(&self) -> String {
        use std::fmt::Write as _;
        let ranks: Vec<String> = self.ranks.iter().map(ToString::to_string).collect();
        let mut id = String::with_capacity(self.id.len() * 2);
        for byte in self.id.bytes() {
            let _ = write!(id, "{byte:02x}");
        }
        format!("{CURSOR_PREFIX}:{}:{id}", ranks.join(","))
    }

    /// The key a cursor names.
    ///
    /// # Errors
    ///
    /// [`AppError::Validation`] for a cursor this app didn't make.
    pub fn parse(cursor: &str) -> AppResult<Self> {
        let bad = || AppError::Validation {
            subject: "page",
            message: "That list moved on; read it again from the top.".to_owned(),
        };
        let mut parts = cursor.splitn(3, ':');
        if parts.next() != Some(CURSOR_PREFIX) {
            return Err(bad());
        }
        let ranks = parts.next().ok_or_else(bad)?;
        let ranks = if ranks.is_empty() {
            Vec::new()
        } else {
            ranks
                .split(',')
                .map(|rank| rank.parse::<i64>().map_err(|_| bad()))
                .collect::<AppResult<Vec<_>>>()?
        };
        let hex = parts.next().ok_or_else(bad)?;
        if hex.len() % 2 != 0 {
            return Err(bad());
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).map_err(|_| bad()))
            .collect::<AppResult<Vec<u8>>>()?;
        let id = String::from_utf8(bytes).map_err(|_| bad())?;
        Ok(Self { ranks, id })
    }
}

/// One page of `rows` in `key` order, after the request's cursor.
///
/// For lists held in memory (the conversations index, a query's
/// matches). A list read from SQLite pages in its query instead, with the
/// same keys ([`page_of_fetched`]).
///
/// # Errors
///
/// [`AppError::Validation`] for a cursor this app didn't make.
pub fn page<T>(
    rows: Vec<T>,
    key: impl Fn(&T) -> SortKey,
    request: &PageRequest,
) -> AppResult<Page<T>> {
    let after = request.start_after()?;
    let limit = request.limit();
    let total = u64::try_from(rows.len()).ok();
    let mut keyed: Vec<(SortKey, T)> = rows.into_iter().map(|row| (key(&row), row)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    let start = after.map_or(0, |after| keyed.partition_point(|(key, _)| *key <= after));
    let mut rest = keyed.into_iter().skip(start);
    let items: Vec<(SortKey, T)> = rest.by_ref().take(limit).collect();
    let more = rest.next().is_some();
    Ok(Page {
        next_cursor: if more {
            items.last().map(|(key, _)| key.cursor())
        } else {
            None
        },
        items: items.into_iter().map(|(_, row)| row).collect(),
        total,
    })
}

/// A page from rows a query read in key order after the cursor, one more
/// than the page holds (that one only says another page follows).
#[must_use]
pub fn page_of_fetched<T>(
    mut fetched: Vec<(SortKey, T)>,
    limit: usize,
    total: Option<u64>,
) -> Page<T> {
    let more = fetched.len() > limit;
    fetched.truncate(limit);
    Page {
        next_cursor: if more {
            fetched.last().map(|(key, _)| key.cursor())
        } else {
            None
        },
        items: fetched.into_iter().map(|(_, row)| row).collect(),
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, TS)]
    struct Row {
        id: String,
        at: i64,
    }

    fn row(id: &str, at: i64) -> Row {
        Row {
            id: id.to_owned(),
            at,
        }
    }

    /// Newest first, then id.
    fn newest(row: &Row) -> SortKey {
        SortKey::new([-row.at], row.id.clone())
    }

    fn ids(page: &Page<Row>) -> Vec<&str> {
        page.items.iter().map(|row| row.id.as_str()).collect()
    }

    #[test]
    fn the_limit_has_a_default_and_a_ceiling() {
        assert_eq!(PageRequest::default().limit(), DEFAULT_LIMIT);
        assert_eq!(PageRequest::first(0).limit(), 1);
        assert_eq!(PageRequest::first(10_000).limit(), MAX_LIMIT);
    }

    #[test]
    fn pages_walk_the_whole_list_once_in_order() {
        let rows: Vec<Row> = (0..7).map(|n| row(&format!("r{n}"), n)).collect();
        let first = page(rows.clone(), newest, &PageRequest::first(3)).unwrap();
        assert_eq!(ids(&first), ["r6", "r5", "r4"]);
        assert_eq!(first.total, Some(7));
        let cursor = first.next_cursor.clone().unwrap();
        let second = page(rows.clone(), newest, &PageRequest::after(&cursor, 3)).unwrap();
        assert_eq!(ids(&second), ["r3", "r2", "r1"]);
        let last = page(
            rows,
            newest,
            &PageRequest::after(&second.next_cursor.unwrap(), 3),
        )
        .unwrap();
        assert_eq!(ids(&last), ["r0"]);
        assert_eq!(last.next_cursor, None, "the last page says so");
    }

    #[test]
    fn rows_added_meanwhile_never_shift_or_repeat_what_was_shown() {
        let mut rows: Vec<Row> = (0..6).map(|n| row(&format!("r{n}"), n * 10)).collect();
        let first = page(rows.clone(), newest, &PageRequest::first(3)).unwrap();
        assert_eq!(ids(&first), ["r5", "r4", "r3"]);
        // A new session at the top, one between the pages, one removed.
        rows.push(row("new-top", 100));
        rows.push(row("new-between", 25));
        rows.retain(|r| r.id != "r4");
        let second = page(
            rows,
            newest,
            &PageRequest::after(&first.next_cursor.unwrap(), 3),
        )
        .unwrap();
        assert_eq!(ids(&second), ["new-between", "r2", "r1"]);
    }

    #[test]
    fn equal_times_are_ordered_by_id_and_never_split_or_doubled() {
        let rows: Vec<Row> = ["c", "a", "d", "b"].iter().map(|id| row(id, 5)).collect();
        let first = page(rows.clone(), newest, &PageRequest::first(2)).unwrap();
        let second = page(
            rows,
            newest,
            &PageRequest::after(first.next_cursor.as_deref().unwrap(), 2),
        )
        .unwrap();
        assert_eq!(ids(&first), ["a", "b"]);
        assert_eq!(ids(&second), ["c", "d"]);
    }

    #[test]
    fn a_cursor_round_trips_any_id_and_refuses_a_made_up_one() {
        let key = SortKey::new([0, -1_729_000_000_000], "acc:1/conv é,:x");
        assert_eq!(SortKey::parse(&key.cursor()).unwrap(), key);
        for bad in ["", "k1", "k2:1:61", "k1:x:61", "k1:1:6", "k1:1:zz"] {
            assert!(SortKey::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn fetched_rows_page_the_same_way() {
        let fetched: Vec<(SortKey, Row)> = (0..4)
            .map(|n| {
                (
                    newest(&row(&format!("r{n}"), -n)),
                    row(&format!("r{n}"), -n),
                )
            })
            .collect();
        let page = page_of_fetched(fetched, 3, Some(10));
        assert_eq!(ids(&page), ["r0", "r1", "r2"]);
        assert_eq!(page.next_cursor, Some(newest(&row("r2", -2)).cursor()));
        assert_eq!(page.total, Some(10));
    }
}
