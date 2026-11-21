//! The feed's digest item (ADR-0036 §1.9, AJ-E3712 R2): one more
//! [`FeedSource`] over ADR-0037's store, a page at a time like every
//! other. A digest is about every agent, so it shows in the unfiltered
//! feed only; a filter by chip, project, pool or App leaves it out.

use juggler_core::storage::Storage;
use juggler_core::storage::feed::FeedKey;

use super::store;
use crate::error::AppResult;
use crate::features::feed::dto::{FeedAuthorInfo, FeedItemInfo};
use crate::features::feed::page::{Ask, FeedSource, Read};
use crate::features::notifications::centre::NoticeTarget;

/// The digests, newest first.
pub struct Digests;

/// The source, for the feed's list.
pub const SOURCE: Digests = Digests;

/// The feed's kind for a digest.
pub const KIND: &str = "agent_digest";

impl FeedSource for Digests {
    fn read(
        &self,
        storage: &Storage,
        before: Option<FeedKey>,
        ask: &Ask,
        limit: usize,
    ) -> AppResult<Read> {
        let scoped =
            ask.scope.project.is_some() || ask.scope.pool.is_some() || ask.scope.folders.is_some();
        if scoped || ask.category.is_some() {
            return Ok(Read::default());
        }
        let now = crate::commands::now_millis();
        let rows = storage.feed().digests(before, now, limit)?;
        let full = rows.len() >= limit;
        let items = rows
            .iter()
            .filter_map(|row| {
                let digest = store::read(&row.payload)?;
                Some((
                    row.key,
                    FeedItemInfo {
                        id: format!("d:{}", row.key.rowid),
                        at_ms: row.key.at_ms,
                        kind: KIND.to_owned(),
                        category: "digest".to_owned(),
                        // A note: the digest never replaces a need, and
                        // says one in normal ink (D-117).
                        level: 1,
                        mine: false,
                        coalesce: None,
                        rollup: None,
                        rollup_window_ms: 0,
                        scope: None,
                        author: FeedAuthorInfo {
                            kind: "juggler".to_owned(),
                            ..FeedAuthorInfo::default()
                        },
                        subject: String::new(),
                        facts: crate::features::feed::dto::FeedFactsInfo::default(),
                        target: NoticeTarget::default(),
                        digest: Some(digest),
                    },
                ))
            })
            .collect();
        Ok(Read { items, full })
    }
}
