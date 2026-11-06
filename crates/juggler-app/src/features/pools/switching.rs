//! D-62 (AJ-E3009 req. 1): what a switch does to a session's pools, one
//! line per pool, before it's confirmed.
//!
//! The session's line of work moves to another folder (and maybe another
//! tool). Its pools are worked out again for where it lands: one it stays
//! in says whether it acts there with full tools or through its shell; one
//! it leaves says why (it came in through its App, Account or Tool, and the
//! new one isn't a member; or a sealed pool takes it); one it joins says
//! through what. A sealed pool it enters or leaves asks for a confirm.

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
) -> AppResult<Vec<SwitchPoolLineInfo>> {
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
