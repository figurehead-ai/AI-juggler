//! AJ-E3013 (D-116, D-119): a pool's desktop conversations as messages
//! and agents see them.
//!
//! - **Connected:** its App has AI Juggler's connector (current, or an
//!   older edition's), or one of its sessions called through it lately (an
//!   attested call). It is a full member: a message to it goes to its
//!   inbox under the pool's talk rules, and the pool's waking rule (D-119:
//!   the pool at Full, or a rule the user made, and an App that can be
//!   woken) decides whether a new one wakes it or waits for its next turn.
//! - **Watched** (D-62 decision 15): no connector. It can't reply until it
//!   is continued in AI Juggler; a message to it waits with that reason.
//!
//! A pool also holds a desktop session through another member (its App,
//! its account …) while one of its calls is attested: listed here too,
//! connected.
//!
//! One answer for the message pipeline (who a message reaches and how),
//! Needs you, and the agents' `pool_roster` and `whoami`.

use std::collections::HashMap;
use std::sync::Arc;

use juggler_core::model::HarnessId;
use juggler_core::storage::{App, MemberRef};

use crate::error::AppResult;
use crate::state::AppState;

/// What a Watched member's state says, for agents.
pub(crate) const WATCHED_WORDS: &str =
    "watched: can't reply until it's continued in AI Juggler (its app has no AI Juggler connector)";
/// What a connected member's state says when a new message wakes it.
pub(crate) const WAKES_WORDS: &str = "wakes on new messages";
/// What a connected member's state says when it reads at its next turn.
pub(crate) const NEXT_TURN_WORDS: &str = "reads messages on its next turn";

/// A desktop conversation a pool holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DesktopConversation {
    /// Its member ref, as messages address it (`conversation:<folder>/<id>`).
    pub member: String,
    /// Its session id while one of its calls is attested (`desktop:…`).
    pub session: Option<String>,
    /// Its title, else what it is.
    pub name: String,
    /// Its tool, in words.
    pub tool: Option<String>,
    /// Its account's label.
    pub account: Option<String>,
    /// Where it runs: its App, in words ("Claude app · Work").
    pub place: Option<String>,
    /// Its role in the pool.
    pub role: String,
    /// It reads its messages through AI Juggler's connector (else Watched).
    pub connected: bool,
    /// A new pool message wakes it (D-119); else it reads it on its next
    /// turn. Never for a Watched member.
    pub wakes: bool,
    /// Its App can be woken at all (the person's Deliver wakes it, the
    /// pool's level aside). Never for a Watched member.
    pub can_wake: bool,
    /// Its desktop App (`apps.id`), when known.
    pub app_id: Option<String>,
    /// How it's in the pool: `conversation` (named on its own) or the
    /// broad member that brought it in (`app`, `account`, …).
    pub via: String,
}

impl DesktopConversation {
    /// D-119 item 6: it's open now: one of its calls is attested, or its
    /// tool's process is on it (`live`: the conversation index's live
    /// set). A pool-wide message is copied to it only then.
    pub(crate) fn is_open(&self, live: &[crate::features::conversations::live::Live]) -> bool {
        if self.session.is_some() {
            return true;
        }
        let Some(MemberRef::Conversation { folder, id }) = MemberRef::parse(&self.member) else {
            return false;
        };
        live.iter()
            .any(|(account, _, conversation)| *account == folder && *conversation == id)
    }

    /// How a new pool message reaches it, in words (the roster's state).
    pub(crate) const fn reach_words(&self) -> &'static str {
        if !self.connected {
            WATCHED_WORDS
        } else if self.wakes {
            WAKES_WORDS
        } else {
            NEXT_TURN_WORDS
        }
    }
}

/// An App's place in words: its host app's name, and its own label, else
/// its instance folder's name ("Claude app · aij-test", as the window
/// names it).
fn place_of(state: &Arc<AppState>, app: &App) -> String {
    let base = state
        .registry
        .get(&HarnessId::new(app.tool.as_str()))
        .and_then(|adapter| {
            adapter
                .host_apps()
                .into_iter()
                .find(|host| app.host_app_id.as_deref() == Some(host.id.as_str()))
                .map(|host| host.shown_name().to_owned())
        })
        .unwrap_or_else(|| app.tool.clone());
    let label = app
        .label
        .clone()
        .filter(|label| !label.is_empty())
        .or_else(|| {
            app.instance_dir
                .as_deref()
                .and_then(|dir| std::path::Path::new(dir).file_name())
                .map(|name| name.to_string_lossy().into_owned())
        });
    match label {
        Some(label) => format!("{base} · {label}"),
        None => base,
    }
}

/// Whether a session of `app` can be woken (D-119: its tool's adapter has
/// a way in).
fn can_wake(state: &Arc<AppState>, app: &App) -> bool {
    state
        .registry
        .get(&HarnessId::new(app.tool.as_str()))
        .and_then(|adapter| {
            adapter
                .desktop_peer()
                .map(juggler_adapter::peer::DesktopPeer::can_wake)
        })
        .unwrap_or(false)
}

/// The title the app's own record gave a session attested lately (its
/// first calls can come before the conversation index has it).
fn kept_title(state: &Arc<AppState>, session: &str) -> Option<String> {
    state
        .agent_bridge
        .desktop
        .lock()
        .ok()?
        .get(session)
        .and_then(|(_, kept)| kept.title.clone())
}

/// A tool's name in words, from its harness id.
fn tool_words(state: &Arc<AppState>, tool: &str) -> String {
    state
        .registry
        .get(&HarnessId::new(tool))
        .map_or_else(|| tool.to_owned(), |adapter| adapter.spec().display_name)
}

/// The desktop conversations `pool_id` holds: its conversation members not
/// continued in AI Juggler, Watched or connected, and the sessions attested
/// lately that it holds through another member. Cheap: the conversation
/// rows, the connector's records and status, and what the attestation
/// keeps in memory; no attestation of its own.
///
/// # Errors
///
/// [`crate::error::AppError::Refused`] for a pool that isn't there;
/// storage failures.
pub(crate) fn of_pool(state: &Arc<AppState>, pool_id: &str) -> AppResult<Vec<DesktopConversation>> {
    let watched = crate::features::pools::watched::watched_impl(state, pool_id)?;
    let members: HashMap<String, super::dto::DesktopMemberInfo> =
        super::desktop_members_impl(state, pool_id)?
            .into_iter()
            .map(|member| (member.member.clone(), member))
            .collect();
    let (fresh, memberships, apps, wakes) = {
        let storage = crate::commands::lock_storage_arc(state)?;
        (
            storage.attested().fresh(),
            crate::features::pools::members::memberships(&storage)?,
            storage.apps().list()?,
            crate::features::messages::delivery::pool_delivers(&storage, pool_id),
        )
    };
    let app_of = |id: Option<&str>| apps.iter().find(|app| Some(app.id.as_str()) == id);
    let attested = |member: &str| {
        fresh.iter().find(|facts| {
            facts.conversation.as_ref().is_some_and(|(folder, id)| {
                MemberRef::Conversation {
                    folder: folder.clone(),
                    id: id.clone(),
                }
                .to_wire()
                    == member
            })
        })
    };
    let mut out: Vec<DesktopConversation> = watched
        .into_iter()
        .filter(|one| one.continued_session.is_none())
        .map(|one| {
            let info = members.get(&one.member);
            let connected = info.is_some_and(|info| {
                info.connected || matches!(info.connector.as_str(), "current" | "outdated")
            });
            // What the index doesn't say, the facts an attested call kept do.
            let facts = attested(&one.member);
            DesktopConversation {
                session: facts.map(|facts| facts.session.as_str().to_owned()),
                name: one
                    .title
                    .or_else(|| facts.and_then(|facts| kept_title(state, facts.session.as_str())))
                    .unwrap_or_else(|| "A session in another app".to_owned()),
                tool: one.tool.or_else(|| {
                    facts
                        .and_then(|facts| facts.tool.as_deref())
                        .map(|tool| tool_words(state, tool))
                }),
                account: one.account_label,
                place: info
                    .and_then(|info| app_of(info.app_id.as_deref()))
                    .or_else(|| facts.and_then(|facts| app_of(facts.app.as_deref())))
                    .map(|app| place_of(state, app)),
                role: one.role,
                connected,
                wakes: connected && info.is_some_and(|info| info.wake == "wakes"),
                can_wake: connected && info.is_some_and(|info| info.wake != "cant_wake"),
                app_id: info
                    .and_then(|info| info.app_id.clone())
                    .or_else(|| facts.and_then(|facts| facts.app.clone())),
                via: "conversation".to_owned(),
                member: one.member,
            }
        })
        .collect();
    let held_otherwise = through_others(state, pool_id, &out, &fresh, &memberships, &apps, wakes);
    out.extend(held_otherwise);
    Ok(out)
}

/// The sessions attested lately that `pool_id` holds through another
/// member (their App, their account …), beside its own conversation rows
/// (`listed`): connected, named from the conversation index when it has
/// them.
fn through_others(
    state: &Arc<AppState>,
    pool_id: &str,
    listed: &[DesktopConversation],
    fresh: &[juggler_core::pools::SessionFacts],
    memberships: &juggler_core::pools::Memberships,
    apps: &[App],
    wakes: bool,
) -> Vec<DesktopConversation> {
    let mut others = Vec::new();
    for facts in fresh {
        let Some((folder, id)) = facts.conversation.clone() else {
            continue;
        };
        let member = MemberRef::Conversation { folder, id }.to_wire();
        if listed.iter().any(|one| one.member == member) {
            continue;
        }
        let resolution = memberships.resolve(facts);
        let Some(place) = resolution.in_pool(pool_id) else {
            continue;
        };
        let app = apps
            .iter()
            .find(|app| Some(app.id.as_str()) == facts.app.as_deref());
        others.push((member, facts, place.role, place.via().as_str(), app));
    }
    if others.is_empty() {
        return Vec::new();
    }
    let index: HashMap<(String, String), String> =
        crate::features::conversations::commands::list_indexed(state)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|info| Some(((info.account_id, info.id), info.title?)))
            .collect();
    others
        .into_iter()
        .map(|(member, facts, role, via, app)| DesktopConversation {
            name: facts
                .conversation
                .as_ref()
                .and_then(|key| index.get(key).cloned())
                .or_else(|| kept_title(state, facts.session.as_str()))
                .unwrap_or_else(|| "A session in another app".to_owned()),
            session: Some(facts.session.as_str().to_owned()),
            tool: facts.tool.as_deref().map(|tool| tool_words(state, tool)),
            account: None,
            place: app.map(|app| place_of(state, app)),
            role: role.as_str().to_owned(),
            connected: true,
            wakes: wakes && app.is_some_and(|app| can_wake(state, app)),
            can_wake: app.is_some_and(|app| can_wake(state, app)),
            app_id: facts.app.clone(),
            via: via.to_owned(),
            member,
        })
        .collect()
}
