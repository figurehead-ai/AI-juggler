//! 0.4.6 (AJ-E3011): who leads, never a dead end. D-92 keeps Lead one
//! session's role, so a pool of Apps with no session open had no Lead to
//! pick: the newcomer walks all stopped there. A pool may now wait for
//! an App's next session ("Claude Code · Work: its next session here
//! leads"). When a session of that App joins the pool and the pool has no
//! Lead, it joins as a session member with Lead and the wait ends; one
//! session leads, always.
//!
//! - The user sets or clears the wait ([`set_impl`]); making a session
//!   Lead by hand, or taking the App out of the pool, ends it.
//! - A session claims it when it starts ([`claim`], called from the one
//!   start path). The claim is one write transaction in storage, so of
//!   two sessions starting at once only one leads.
//! - Nothing here starts, stops or types into a session.

use std::sync::Arc;

use juggler_core::model::SessionId;
use juggler_core::storage::{MemberRef, PoolRole, Storage};

use super::{live_pool, member_of, members, model};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Sets the App whose next session leads `pool_id` (`app`, an App's id),
/// or clears the wait (`None`); the wait now. The pool's rules carry it
/// too (`PoolRulesInfo::lead_next`).
///
/// # Errors
///
/// [`AppError::Refused`] for a missing pool, an App that isn't a member
/// of it, or a pool that has a Lead (make it a Member first).
pub fn set_impl(
    state: &Arc<AppState>,
    pool_id: &str,
    app: Option<&str>,
) -> AppResult<Option<String>> {
    let storage = lock_storage_arc(state)?;
    let pool = live_pool(&storage, pool_id)?;
    if let Some(app) = app {
        let wire = MemberRef::App(app.to_owned()).to_wire();
        let rows = storage.pools().members()?;
        let here = rows.iter().filter(|row| row.pool_id == pool.id);
        let mut is_member = false;
        let mut led = false;
        for row in here {
            is_member |= row.member.to_wire() == wire;
            led |= row.role == PoolRole::Lead;
        }
        if !is_member {
            return Err(AppError::Refused(
                "Add the App to the pool first: its next session leads once it's in.".to_owned(),
            ));
        }
        if led {
            return Err(AppError::Refused(
                "This pool has a Lead. Make it a Member first, then pick who leads next."
                    .to_owned(),
            ));
        }
    }
    storage.pools().set_lead_next(&pool.id, app)?;
    model::membership_step(state, "lead_next", &pool.id, &MemberRef::Everyone, None);
    Ok(storage.pools().lead_next(&pool.id)?)
}

/// Ends `pool_id`'s wait for a next session to lead: a Lead was picked by
/// hand. Nothing when it waits for none.
///
/// # Errors
///
/// Storage failures.
pub(super) fn picked(storage: &Storage, pool_id: &str) -> AppResult<()> {
    if storage.pools().lead_next(pool_id)?.is_some() {
        storage.pools().set_lead_next(pool_id, None)?;
    }
    Ok(())
}

/// Ends `pool_id`'s wait when it was for `member`, an App leaving the pool.
///
/// # Errors
///
/// Storage failures.
pub(super) fn left(storage: &Storage, pool_id: &str, member: &MemberRef) -> AppResult<()> {
    if let MemberRef::App(app) = member
        && storage.pools().lead_next(pool_id)?.as_deref() == Some(app.as_str())
    {
        storage.pools().set_lead_next(pool_id, None)?;
    }
    Ok(())
}

/// A session that just started: in each pool that waits for its App's
/// next session and holds it, it takes the Lead when the pool has none.
/// The pools it led, by id. Called from the one start path; a failure
/// here never fails the start (the pool keeps waiting).
pub fn claim(state: &Arc<AppState>, session: &SessionId) -> Vec<String> {
    let led = lock_storage_arc(state).and_then(|storage| claim_in(&storage, session));
    match led {
        Ok(led) => {
            for pool in &led {
                let member = lock_storage_arc(state)
                    .and_then(|storage| member_of(&storage, session))
                    .map_or_else(
                        |_| MemberRef::Session(session.clone()),
                        |(member, _)| member,
                    );
                model::membership_step(
                    state,
                    "lead_next_claimed",
                    pool,
                    &member,
                    Some(PoolRole::Lead),
                );
            }
            led
        }
        Err(error) => {
            tracing::warn!(%error, "a pool's next-session lead wasn't claimed");
            Vec::new()
        }
    }
}

/// [`claim`] on held storage.
fn claim_in(storage: &Storage, session: &SessionId) -> AppResult<Vec<String>> {
    let Some(facts) = storage.sessions().pool_facts_of(session)? else {
        return Ok(Vec::new());
    };
    let Some(app) = facts.app.clone() else {
        return Ok(Vec::new());
    };
    let waiting = storage.pools().waiting_for_lead(&app)?;
    if waiting.is_empty() {
        return Ok(Vec::new());
    }
    let resolution = members::memberships(storage)?.resolve(&facts);
    let (member, _) = member_of(storage, session)?;
    let mut led = Vec::new();
    for pool in waiting {
        if resolution.in_pool(&pool).is_some()
            && storage.pools().claim_lead_next(&pool, &app, &member)?
        {
            led.push(pool);
        }
    }
    Ok(led)
}
