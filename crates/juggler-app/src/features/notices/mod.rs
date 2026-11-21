//! ADR-0035 §2 (AJ-E2023): notices, the `notices` section of the signed
//! remote configuration: a short, plain message that can reach everyone
//! (or the installs a rule targets) without a release.
//!
//! **A notice is data, never markup or an action:**
//! - an `id` (a dismissal is remembered by it);
//! - a `level`: `info`, `needs_you` or `broken`, D-117's L1, L2 and L3;
//! - a `title` (at most 80 characters) and an optional `body` (280), plain
//!   text: no control, direction or markup characters;
//! - at most one `link`, `https://` to [`LINK_HOSTS`] only, with an
//!   optional `link_label`; any other link is dropped and the notice kept;
//! - a `when`: the targeting every rule uses, evaluated on this Mac;
//! - `dismissable` (true unless the file says otherwise) and an optional
//!   `expires_at` (Unix seconds).
//!
//! **Never louder than the facts justify** ([`cap`], D-117): a notice is
//! `broken` only when its targeting names the versions this Mac runs (a
//! tool's, or AI Juggler's own), `needs_you` only when it targets this
//! setup at all (versions or the OS), and otherwise `info`, whatever the
//! file asks. An `info` notice can always be dismissed.
//!
//! **Dismissals stay on this Mac** in the app's settings
//! ([`DISMISSED_KEY`]), the newest [`MAX_DISMISSED`] kept.
//!
//! The section is registered by the remote-config service with the others;
//! the window reads [`active`] through [`commands`]. [`FLAG`] is the
//! notices' own kill switch: an older app that shows notices badly can be
//! silenced by a rule targeting its version.

pub mod commands;
pub mod dto;

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use juggler_remote_config::list::{Field, FieldKind, ListSpec};
use juggler_remote_config::target::When;
use juggler_remote_config::{Item, RemoteMode, SectionSpec};

use crate::error::{AppError, AppResult};
use crate::features::flags::{self, Flag};
use crate::features::remote_config::Snapshot;
use crate::state::AppState;
use dto::{RemoteNoticeInfo, RemoteNoticeLevel};

/// The section's name in the file.
pub const SECTION: &str = "notices";

/// The section's schema version this build reads.
pub const SCHEMA: u32 = 1;

/// The most notices one file may carry.
pub const MAX_NOTICES: u32 = 16;

/// The only hosts a notice may link to: our website.
pub const LINK_HOSTS: &[&str] = &["figurehead.ai", "www.figurehead.ai"];

/// The notices' kill switch: the file may turn notices off (for an app
/// version, say), never on.
pub const FLAG: Flag = Flag {
    name: "notices_on",
    default: true,
    remote: RemoteMode::OffOnly,
};

/// Where the dismissed ids are kept (a JSON list, oldest first).
pub const DISMISSED_KEY: &str = "remote_notices_dismissed";

/// The most dismissals kept: the oldest are forgotten first.
pub const MAX_DISMISSED: usize = 200;

/// The `notices` section, to register with the app's remote-config
/// registry.
#[must_use]
pub fn section() -> SectionSpec {
    let text = |max_chars| FieldKind::Text { max_chars };
    let list = ListSpec::new(MAX_NOTICES)
        .field(
            "level",
            Field::required(FieldKind::Choice {
                options: ["info", "needs_you", "broken"].map(str::to_owned).to_vec(),
            }),
        )
        .field("title", Field::required(text(80)))
        .field("body", Field::optional(text(280)))
        .field(
            "link",
            Field::optional(FieldKind::Link {
                hosts: LINK_HOSTS.iter().map(|host| (*host).to_owned()).collect(),
            }),
        )
        .field("link_label", Field::optional(text(40)))
        .field("dismissable", Field::optional(FieldKind::Flag))
        .field("expires_at", Field::optional(FieldKind::Time));
    SectionSpec::new(SECTION, SCHEMA).list(list)
}

/// The level a notice may have, given what its targeting says about this
/// Mac (see the module docs).
#[must_use]
pub fn cap(asked: RemoteNoticeLevel, when: &When) -> RemoteNoticeLevel {
    let target = match when {
        When::Target(target) => Some(target),
        When::Always | When::Unknown => None,
    };
    let versions = target.is_some_and(|t| t.tools.is_some() || t.app.is_some());
    let setup = versions || target.is_some_and(|t| t.os.is_some() || t.os_version.is_some());
    match asked {
        RemoteNoticeLevel::Broken if versions => RemoteNoticeLevel::Broken,
        RemoteNoticeLevel::Broken | RemoteNoticeLevel::NeedsYou if setup => {
            RemoteNoticeLevel::NeedsYou
        }
        _ => RemoteNoticeLevel::Info,
    }
}

/// One record as a notice, capped; `None` if it has expired.
fn notice(item: &Item, now: u64) -> Option<RemoteNoticeInfo> {
    if item.time("expires_at").is_some_and(|at| at <= now) {
        return None;
    }
    let asked = match item.choice("level")? {
        "broken" => RemoteNoticeLevel::Broken,
        "needs_you" => RemoteNoticeLevel::NeedsYou,
        _ => RemoteNoticeLevel::Info,
    };
    let level = cap(asked, &item.when);
    let link = item.link("link").map(str::to_owned);
    Some(RemoteNoticeInfo {
        id: item.id.clone(),
        level,
        title: item.text("title")?.to_owned(),
        body: item.text("body").map(str::to_owned),
        link_label: link
            .as_ref()
            .and_then(|_| item.text("link_label").map(str::to_owned)),
        link,
        dismissable: level == RemoteNoticeLevel::Info || item.flag("dismissable").unwrap_or(true),
    })
}

/// Every notice in effect in `snapshot` at `now` (Unix seconds), the
/// dismissed left out, louder first, then in the file's order.
#[must_use]
pub fn active(
    snapshot: &Snapshot,
    dismissed: &BTreeSet<String>,
    now: u64,
) -> Vec<RemoteNoticeInfo> {
    if !flags::on_in(&snapshot.effective, &FLAG) {
        return Vec::new();
    }
    let mut notices: Vec<RemoteNoticeInfo> = snapshot
        .lists
        .get(SECTION)
        .into_iter()
        .flatten()
        .filter_map(|item| notice(item, now))
        .filter(|notice| !(notice.dismissable && dismissed.contains(&notice.id)))
        .collect();
    notices.sort_by_key(|notice| std::cmp::Reverse(notice.level));
    notices
}

/// The ids dismissed on this Mac, oldest first.
fn dismissed_list(state: &AppState) -> AppResult<Vec<String>> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let saved = storage.app_settings().get(DISMISSED_KEY)?;
    Ok(saved
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .unwrap_or_default())
}

/// The ids dismissed on this Mac.
///
/// # Errors
///
/// The settings can't be read.
pub fn dismissed(state: &AppState) -> AppResult<BTreeSet<String>> {
    Ok(dismissed_list(state)?.into_iter().collect())
}

/// Remembers that `id` was dismissed (the newest [`MAX_DISMISSED`] kept).
///
/// # Errors
///
/// An id that isn't one, or settings that can't be written.
pub fn remember_dismissed(state: &AppState, id: &str) -> AppResult<()> {
    let valid = !id.is_empty()
        && id.len() <= juggler_remote_config::list::MAX_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    if !valid {
        return Err(AppError::Validation {
            subject: "notice",
            message: "not a notice id".to_owned(),
        });
    }
    let mut ids = dismissed_list(state)?;
    ids.retain(|kept| kept != id);
    ids.push(id.to_owned());
    let over = ids.len().saturating_sub(MAX_DISMISSED);
    ids.drain(..over);
    let text = serde_json::to_string(&ids).map_err(|e| AppError::Validation {
        subject: "notice",
        message: e.to_string(),
    })?;
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    storage.app_settings().set(DISMISSED_KEY, &text)?;
    Ok(())
}

/// The notices to show now.
///
/// # Errors
///
/// The dismissals can't be read.
pub fn active_now(state: &AppState) -> AppResult<Vec<RemoteNoticeInfo>> {
    let snapshot = crate::features::remote_config::current();
    Ok(active(&snapshot, &dismissed(state)?, now_secs()))
}

/// Dismisses the notice `id`: remembered, so it doesn't show again. A
/// notice no longer shown is fine (it may have just gone); one that can't
/// be dismissed is refused.
///
/// # Errors
///
/// A notice that can't be dismissed, an id that isn't one, or settings
/// that can't be read or written.
pub fn dismiss(state: &AppState, id: &str) -> AppResult<Vec<RemoteNoticeInfo>> {
    let shown = active_now(state)?;
    if shown
        .iter()
        .any(|notice| notice.id == id && !notice.dismissable)
    {
        return Err(AppError::Refused(
            "This notice stays until the problem it's about is fixed.".to_owned(),
        ));
    }
    remember_dismissed(state, id)?;
    Ok(shown.into_iter().filter(|notice| notice.id != id).collect())
}

/// Now, Unix seconds.
fn now_secs() -> u64 {
    crate::commands::now_millis().max(0).unsigned_abs() / 1000
}
