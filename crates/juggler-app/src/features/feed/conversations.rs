//! Other apps' conversations in the feed (D-118 R1, the conversation
//! index): "worked on" items, from what the index holds now (its own
//! files, read by the adapters' scanners; nothing is read here). Their
//! state changes are not kept yet (F2's G4, AJ-E3705), so an item says
//! only what is known: when it was last active.

use std::sync::Arc;

use juggler_core::storage::feed::FeedKey;

use super::dto::{FeedAuthorInfo, FeedFactsInfo, FeedItemInfo};
use super::items::{SAME_VERB_MS, category_of};
use super::page::{Ask, CONVERSATIONS, Read};
use crate::error::AppResult;
use crate::features::conversations::dto::ConversationInfo;
use crate::features::notifications::centre::NoticeTarget;
use crate::state::AppState;

/// A stable row id for a conversation (it has no row of its own): FNV-1a
/// of its account and id, kept positive.
fn rowid_of(account: &str, id: &str) -> i64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in account.bytes().chain([0]).chain(id.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    i64::try_from(hash >> 1).unwrap_or(i64::MAX)
}

/// One conversation as an item.
#[must_use]
pub fn item_of(row: &ConversationInfo) -> Option<(FeedKey, FeedItemInfo)> {
    let at_ms = row.last_activity_ms?;
    let key = FeedKey {
        at_ms,
        source: CONVERSATIONS,
        rowid: rowid_of(&row.account_id, &row.id),
    };
    let desktop = row
        .app
        .as_deref()
        .is_some_and(|app| !app.starts_with("cli:"))
        || row.started_in.is_some();
    let folder_name = row
        .cwd
        .as_deref()
        .and_then(|cwd| std::path::Path::new(cwd).file_name())
        .map(|name| name.to_string_lossy().into_owned());
    let item = FeedItemInfo {
        id: format!("c:{}:{}", row.account_id, row.id),
        at_ms,
        kind: "worked".to_owned(),
        category: category_of("worked").to_owned(),
        level: 0,
        mine: false,
        coalesce: Some(format!("c:{}:{}", row.account_id, row.id)),
        rollup: Some(format!(
            "v:worked:{}:{}:L0",
            row.account_id,
            row.project_id.as_deref().unwrap_or("-")
        )),
        rollup_window_ms: SAME_VERB_MS,
        scope: Some(format!("f:{}", row.account_id)),
        author: FeedAuthorInfo {
            kind: "app".to_owned(),
            name: None,
            provider: Some(row.tool.clone()),
            folder: Some(row.account_id.clone()),
            folder_label: Some(row.account_label.clone()).filter(|label| !label.is_empty()),
            desktop,
        },
        subject: row
            .title
            .clone()
            .or(folder_name)
            .unwrap_or_else(|| "A conversation".to_owned()),
        facts: FeedFactsInfo {
            project_id: row.project_id.clone(),
            cwd: row.cwd.clone(),
            ..FeedFactsInfo::default()
        },
        target: NoticeTarget {
            conversation_id: Some(row.id.clone()),
            account_id: Some(row.account_id.clone()),
            project_id: row.project_id.clone(),
            ..NoticeTarget::default()
        },
    };
    Some((key, item))
}

/// The conversations past `before` that `ask` keeps, at most `limit`.
///
/// # Errors
///
/// The typed storage error; a poisoned lock.
pub fn read(
    state: &Arc<AppState>,
    before: Option<FeedKey>,
    ask: &Ask,
    limit: usize,
) -> AppResult<Read> {
    if ask.scope.pool.is_some() || ask.category.is_some() {
        return Ok(Read::default());
    }
    let start = before.map_or(i64::MAX, |key| key.at_ms);
    let rows = crate::features::conversations::commands::feed_rows(state, start, limit)?;
    let full = rows.len() >= limit;
    let items = rows
        .iter()
        .filter(|row| {
            ask.scope
                .project
                .as_ref()
                .is_none_or(|id| row.project_id.as_ref() == Some(id))
        })
        .filter(|row| {
            ask.scope
                .folders
                .as_ref()
                .is_none_or(|folders| folders.contains(&row.account_id))
        })
        .filter_map(item_of)
        .filter(|(key, _)| before.is_none_or(|start| *key < start))
        .collect();
    Ok(Read { items, full })
}
