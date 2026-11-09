//! D-62 (AJ-E3009 req. 1): what a switch does to a session's pools, one
//! line per pool, before it's confirmed.
//!
//! The session's line of work moves to another folder (and maybe another
//! tool). Its pools are worked out again for where it lands: one it stays
//! in says whether it acts there with full tools or through its shell; one
//! it leaves says why (it came in through its App, Account or Tool, and the
//! new one isn't a member; or a sealed pool takes it); one it joins says
//! through what. A sealed pool it enters or leaves asks for a confirm.
//!
//! 0.4.1 (the rc5 walk: the confirm said "Stays in «X1 Team»" and the
//! Desktop copy wasn't in it): a switch that continues in the desktop app
//! leaves a session that app runs, which a pool holds only as a Watched
//! member (D-62 decision 15). It stays in every pool the original is in,
//! as a Watched member with the original's role and level
//! ([`keep_watched`]), and the line says so: watched, with no tools and no
//! rules until it's continued in AI Juggler.

use std::sync::Arc;

use juggler_adapter::Capabilities;
use juggler_core::model::{ProfileId, SessionId};
use juggler_core::pools::SkipReason;

use super::dto::SwitchPoolLineInfo;
use super::{members, pool_names};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The lines for switching `session_id` to `target_profile_id`.
///
/// # Errors
///
/// [`AppError::Refused`] for a session or profile that isn't there;
/// storage failures.
pub fn switch_lines_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    destination: Option<&str>,
) -> AppResult<Vec<SwitchPoolLineInfo>> {
    if destination == Some("desktop") {
        return watched_lines(state, session_id, target_profile_id);
    }
    let storage = lock_storage_arc(state)?;
    let session = SessionId::new(session_id);
    let gone = |what: &str| AppError::Refused(format!("That {what} isn't there any more"));
    let facts = storage
        .sessions()
        .pool_facts_of(&session)?
        .ok_or_else(|| gone("session"))?;
    let profile = storage
        .profiles()
        .get(&ProfileId::new(target_profile_id))
        .map_err(|_| gone("profile"))?;
    let folder = storage
        .accounts()
        .get(&profile.account)
        .map_err(|_| gone("account"))?;
    let folder_id = folder.id.as_str().to_owned();
    let account = storage
        .subscriptions()
        .folders()?
        .into_iter()
        .find(|(id, _)| *id == folder_id)
        .and_then(|(_, account)| account);
    let tool = folder.harness.as_str().to_owned();
    let landed = juggler_core::pools::SessionFacts {
        app: Some(format!("cli:{folder_id}")),
        account,
        tool: Some(tool),
        // A new launch carries no conversation of another app's.
        conversation: None,
        ..facts.clone()
    };
    let memberships = members::memberships(&storage)?;
    let before = memberships.resolve(&facts);
    let after = memberships.resolve(&landed);
    let names = pool_names(&storage)?;
    drop(storage);
    let name = |id: &str| {
        names
            .get(id)
            .cloned()
            .unwrap_or_else(|| "a pool".to_owned())
    };
    let adapter = state.registry.get(&folder.harness);
    let full = adapter
        .as_ref()
        .is_some_and(|adapter| adapter.capabilities().contains(Capabilities::AGENT_BRIDGE));
    let tool_name = adapter.map_or_else(
        || "The new tool".to_owned(),
        |adapter| adapter.spec().display_name,
    );
    Ok(lines(&before, &after, &name, full, &tool_name))
}

/// The pools session `session_id` is in now (each once), with its role
/// and level there; `None` when the session isn't there.
fn pools_of(
    storage: &juggler_core::storage::Storage,
    session_id: &str,
) -> AppResult<Option<Vec<juggler_core::pools::Placement>>> {
    let Some(resolution) = members::resolution_of(storage, &SessionId::new(session_id))? else {
        return Ok(None);
    };
    Ok(Some(resolution.placements))
}

/// A Desktop switch's lines: the copy stays in each of the original's
/// pools, watched.
fn watched_lines(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<Vec<SwitchPoolLineInfo>> {
    let storage = lock_storage_arc(state)?;
    let gone = |what: &str| AppError::Refused(format!("That {what} isn't there any more"));
    let placements = pools_of(&storage, session_id)?.ok_or_else(|| gone("session"))?;
    let profile = storage
        .profiles()
        .get(&ProfileId::new(target_profile_id))
        .map_err(|_| gone("profile"))?;
    let harness = storage
        .accounts()
        .get(&profile.account)
        .map_err(|_| gone("account"))?
        .harness;
    let names = pool_names(&storage)?;
    drop(storage);
    let app = state
        .registry
        .get(&harness)
        .and_then(|adapter| adapter.spec().desktop_app)
        .unwrap_or_else(|| "its desktop app".to_owned());
    Ok(placements
        .iter()
        .map(|place| watched_line(place, names.get(&place.pool_id).map(String::as_str), &app))
        .collect())
}

/// "Stays in «X1 Team», watched: …".
fn watched_line(
    place: &juggler_core::pools::Placement,
    name: Option<&str>,
    app: &str,
) -> SwitchPoolLineInfo {
    SwitchPoolLineInfo {
        pool_id: place.pool_id.clone(),
        kind: "watched".to_owned(),
        words: format!(
            "Stays in «{}», watched: it runs in {app}, so the pool's tools and rules reach it once it's continued in AI Juggler.",
            name.unwrap_or("a pool")
        ),
        confirm: false,
    }
}

/// After a switch that continued in the desktop app: the copy,
/// conversation `conversation` of folder `folder`, joins each pool the
/// original `session_id` is in as a Watched member, with the original's
/// role and level there (what [`watched_lines`] said it would). Returns
/// the pools it joined.
///
/// # Errors
///
/// Storage failures.
pub fn keep_watched(
    state: &Arc<AppState>,
    session_id: &str,
    folder: &str,
    conversation: &str,
) -> AppResult<Vec<String>> {
    if folder.is_empty() || conversation.is_empty() {
        return Ok(Vec::new());
    }
    let storage = lock_storage_arc(state)?;
    let Some(placements) = pools_of(&storage, session_id)? else {
        return Ok(Vec::new());
    };
    let member = juggler_core::storage::MemberRef::Conversation {
        folder: folder.to_owned(),
        id: conversation.to_owned(),
    };
    let mut joined = Vec::new();
    for place in placements {
        storage.pools().join(
            &place.pool_id,
            &member,
            &[],
            place.role,
            place.level.as_ref(),
        )?;
        joined.push(place.pool_id);
    }
    drop(storage);
    for pool in &joined {
        super::model::membership_changed(state, "joined", pool, &member);
    }
    Ok(joined)
}

/// When the work leaves a Desktop copy again (the switch undone, or
/// switched back): the copy, session `key` (a conversation's source key),
/// leaves the pools [`keep_watched`] put it in. A session id that isn't a
/// conversation's is left alone. Returns the pools it left.
///
/// # Errors
///
/// Storage failures.
pub fn leave_watched(state: &Arc<AppState>, key: &str) -> AppResult<Vec<String>> {
    let Some((folder, id)) = crate::features::conversations::switch_source::parse_source_key(key)
    else {
        return Ok(Vec::new());
    };
    let member = juggler_core::storage::MemberRef::Conversation {
        folder: folder.to_owned(),
        id: id.to_owned(),
    };
    let storage = lock_storage_arc(state)?;
    let mut left = Vec::new();
    for row in storage.pools().members()? {
        if row.member == member
            && storage
                .pools()
                .leave(&row.pool_id, std::slice::from_ref(&member))?
                > 0
        {
            left.push(row.pool_id);
        }
    }
    drop(storage);
    for pool in &left {
        super::model::membership_changed(state, "left", pool, &member);
    }
    Ok(left)
}

/// The lines, from where the session is and where it would be.
fn lines(
    before: &juggler_core::pools::Resolution,
    after: &juggler_core::pools::Resolution,
    name: &dyn Fn(&str) -> String,
    full: bool,
    tool_name: &str,
) -> Vec<SwitchPoolLineInfo> {
    let mut lines = Vec::new();
    for place in &before.placements {
        if after.in_pool(&place.pool_id).is_some() {
            lines.push(SwitchPoolLineInfo {
                pool_id: place.pool_id.clone(),
                kind: if full { "stays" } else { "shell" }.to_owned(),
                words: if full {
                    format!(
                        "Stays in «{}»; {tool_name} acts there with full tools.",
                        name(&place.pool_id)
                    )
                } else {
                    format!(
                        "Stays in «{}»; {tool_name} acts there through its shell only.",
                        name(&place.pool_id)
                    )
                },
                confirm: false,
            });
            continue;
        }
        let skip = after
            .skipped
            .iter()
            .find(|skip| skip.pool_id == place.pool_id);
        let (words, confirm) = match skip.map(|skip| &skip.reason) {
            Some(SkipReason::SealedElsewhere { sealed_pool }) => (
                format!(
                    "Leaves «{}» for «{}», which is sealed: its sessions are in no other pool.",
                    name(&place.pool_id),
                    name(sealed_pool)
                ),
                true,
            ),
            Some(SkipReason::OutOfScope) => (
                format!(
                    "Leaves «{}»: it doesn't take its project.",
                    name(&place.pool_id)
                ),
                false,
            ),
            Some(SkipReason::HeldElsewhere { by_pool }) => (
                format!(
                    "Leaves «{}»: «{}» names it on its own.",
                    name(&place.pool_id),
                    name(by_pool)
                ),
                false,
            ),
            None => (
                format!(
                    "Leaves «{}»: it came in through its App, Account or tool, and the new one isn't a member there.",
                    name(&place.pool_id)
                ),
                place.sealed,
            ),
        };
        lines.push(SwitchPoolLineInfo {
            pool_id: place.pool_id.clone(),
            kind: "leaves".to_owned(),
            words,
            confirm,
        });
    }
    for place in &after.placements {
        if before.in_pool(&place.pool_id).is_none() {
            lines.push(SwitchPoolLineInfo {
                pool_id: place.pool_id.clone(),
                kind: if place.sealed { "sealed" } else { "joins" }.to_owned(),
                words: if place.sealed {
                    format!(
                        "Joins «{}», which is sealed: it leaves every other pool.",
                        name(&place.pool_id)
                    )
                } else {
                    format!(
                        "Joins «{}» through its {}.",
                        name(&place.pool_id),
                        place.via().as_str()
                    )
                },
                confirm: place.sealed,
            });
        }
    }
    lines
}
