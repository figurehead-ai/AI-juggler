//! AJ-E3013 (D-116, D-119): AI Juggler's connector in a desktop App, on
//! the user's say-so, and a pool's desktop members as the canvas shows
//! them.
//!
//! - **What it is** comes from the App's tool's adapter, as data
//!   (`ToolRuntime::desktop_connector`): the Claude app's plugin folder,
//!   the Codex app's one `config.toml` table. The writer is one for every
//!   tool (`juggler_adapters::connector_files`): only into that App's own
//!   config folder, never through a link, never a secret, never the
//!   Keychain.
//! - **Preview** shows each file before and after; **Add** writes and
//!   records each file's fingerprint here (`connectors.json`, `0600`, in
//!   AI Juggler's data folder) so **Remove** takes out exactly what it
//!   wrote, all or nothing: a file someone changed refuses it, named, and
//!   **Remove anyway** (the person's confirmed choice) deletes it too;
//!   **status** says absent, current, outdated or changed.
//! - **A pool's desktop members** ([`desktop_members_impl`]): each one's
//!   App, its connector, whether one of its sessions called lately (the
//!   attestation's facts, `Storage::attested`), and whether a new pool
//!   message wakes it (the pool at Full, or a rule, and an App that can
//!   be woken) or waits for its next turn.
//!
//! Nothing here starts, stops or types into an app.

pub mod commands;
pub mod dto;
pub(crate) mod reach;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::SharedAdapter;
use juggler_adapter::connector::{ConnectorInput, ConnectorPlan};
use juggler_adapters::connector_files::{self, ConnectorRecord, ConnectorState};
use juggler_agentbridge::wire;
use juggler_core::model::{AccountId, HarnessId};
use juggler_core::storage::MemberRef;

use self::dto::{
    ConnectorFileInfo, ConnectorPreviewInfo, ConnectorRemovalInfo, ConnectorStatusInfo,
    DesktopMemberInfo,
};
use crate::error::{AppError, AppResult};
use crate::features::switch_setup::GetItInfo;
use crate::state::AppState;

/// The records of what Add wrote, by App, in AI Juggler's data folder.
const RECORDS: &str = "connectors.json";

/// Held while the records are read and written again (commands run in
/// parallel; two saves at once would drop one).
static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A desktop App the connector can go into.
struct Target {
    app_id: String,
    label: String,
    config_dir: PathBuf,
    adapter: SharedAdapter,
}

fn load(state: &Arc<AppState>) -> BTreeMap<String, ConnectorRecord> {
    std::fs::read_to_string(state.data_dir.join(RECORDS))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save(state: &Arc<AppState>, records: &BTreeMap<String, ConnectorRecord>) -> AppResult<()> {
    let text = serde_json::to_string_pretty(records)
        .map_err(|error| AppError::Refused(error.to_string()))?;
    juggler_platform::private_fs::write_private(&state.data_dir.join(RECORDS), text.as_bytes())
        .map_err(|error| AppError::io("keep what the connector wrote", &error))
}

/// Why the connector can't go into an App: a code the window words
/// (`ConnectorStatusInfo::unavailable`) and the words for logs (`why_not`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Why {
    /// AI Juggler doesn't know the App, or its tool.
    UnknownApp(String),
    /// The connector isn't offered there (a terminal App, a tool with no
    /// connector yet, a Codex app until lab probe C2).
    NotOffered(String),
    /// The App has no config folder AI Juggler knows yet.
    NoFolder(String),
    /// No juggler-agent beside AI Juggler (or no socket for its data).
    Missing(String),
    /// juggler-agent is there, in a folder others own or can write.
    UnsafeLocation(PathBuf),
}

impl Why {
    /// Its code on the wire.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::UnknownApp(_) => "unknown_app",
            Self::NotOffered(_) => "not_offered",
            Self::NoFolder(_) => "no_folder",
            Self::Missing(_) => "missing",
            Self::UnsafeLocation(_) => "unsafe_location",
        }
    }

    /// Its words (logs, `why_not`).
    pub(crate) fn words(&self) -> String {
        match self {
            Self::UnknownApp(words)
            | Self::NotOffered(words)
            | Self::NoFolder(words)
            | Self::Missing(words) => words.clone(),
            Self::UnsafeLocation(folder) => format!(
                "AI Juggler's juggler-agent is in a folder others can write ({}), so it isn't run",
                folder.display()
            ),
        }
    }
}

impl From<Why> for AppError {
    fn from(why: Why) -> Self {
        Self::Refused(why.words())
    }
}

/// The App `app_id`, when the connector can go into it; else why not.
fn target(state: &Arc<AppState>, app_id: &str) -> Result<Target, Why> {
    let storage = state
        .storage
        .lock()
        .map_err(|_| Why::UnknownApp("AI Juggler is busy".to_owned()))?;
    let app = storage
        .apps()
        .get(app_id)
        .ok()
        .flatten()
        .ok_or_else(|| Why::UnknownApp("AI Juggler doesn't know this app".to_owned()))?;
    if app.kind != "desktop" {
        return Err(Why::NotOffered("only a desktop app takes AI Juggler's connector: terminal sessions get it from AI Juggler's launch".to_owned()));
    }
    let adapter = state
        .registry
        .get(&HarnessId::new(app.tool.as_str()))
        .ok_or_else(|| {
            Why::UnknownApp("AI Juggler has no adapter for this app's tool".to_owned())
        })?;
    if adapter.desktop_connector().is_none() {
        return Err(Why::NotOffered(format!(
            "AI Juggler has no connector for {}'s app yet",
            adapter.spec().display_name
        )));
    }
    let folder = app.folder_id.clone().ok_or_else(|| {
        Why::NoFolder("AI Juggler hasn't seen which folder this app's sessions use yet: open a session in it once".to_owned())
    })?;
    let config_dir = storage
        .accounts()
        .get(&AccountId::new(folder))
        .map_err(|_| Why::NoFolder("this app's folder isn't one AI Juggler keeps".to_owned()))?
        .state_dir;
    let label = app.label.clone().unwrap_or_else(|| {
        app.instance_dir
            .as_deref()
            .and_then(|dir| Path::new(dir).file_name())
            .map_or_else(
                || adapter.spec().display_name,
                |name| name.to_string_lossy().into_owned(),
            )
    });
    Ok(Target {
        app_id: app.id,
        label,
        config_dir,
        adapter,
    })
}

/// The App `app_id`, when the connector is offered there (its adapter
/// may say it isn't yet: a Codex app thread can't be told apart).
fn offered(state: &Arc<AppState>, app_id: &str) -> Result<Target, Why> {
    let target = target(state, app_id)?;
    match target
        .adapter
        .desktop_peer()
        .and_then(juggler_adapter::peer::DesktopPeer::not_offered)
    {
        Some(why) => Err(Why::NotOffered(why)),
        None => Ok(target),
    }
}

/// The connector for this edition of AI Juggler in `target`.
fn plan_for(state: &Arc<AppState>, target: &Target) -> Result<ConnectorPlan, Why> {
    let agent = state
        .agent_bridge
        .program_or_why()
        .map_err(|why| match why {
            crate::features::agent_bridge::Unavailable::Missing => {
                Why::Missing("AI Juggler's juggler-agent isn't beside it".to_owned())
            }
            crate::features::agent_bridge::Unavailable::UnsafeLocation(folder) => {
                Why::UnsafeLocation(folder)
            }
        })?;
    let socket = crate::features::agent_bridge::socket_path(&state.data_dir).ok_or_else(|| {
        Why::Missing("AI Juggler's data folder's path is too long for its socket".to_owned())
    })?;
    let connector = target
        .adapter
        .desktop_connector()
        .ok_or_else(|| Why::NotOffered("no connector for this app".to_owned()))?;
    Ok(connector.plan(&ConnectorInput {
        agent: &agent,
        socket: &socket,
        version: env!("CARGO_PKG_VERSION"),
        server: wire::SERVER_NAME,
        tools: &wire::TOOLS,
    }))
}

fn state_word(state: ConnectorState) -> &'static str {
    match state {
        ConnectorState::Absent => "absent",
        ConnectorState::Current => "current",
        ConnectorState::Outdated => "outdated",
        ConnectorState::Changed => "changed",
    }
}

fn unavailable(app_id: &str, why: &Why) -> ConnectorStatusInfo {
    let home = juggler_platform::user::home();
    ConnectorStatusInfo {
        app_id: app_id.to_owned(),
        state: "unavailable".to_owned(),
        unavailable: Some(why.code().to_owned()),
        unsafe_folder: match why {
            Why::UnsafeLocation(folder) => Some(home.as_deref().map_or_else(
                || folder.display().to_string(),
                |home| crate::features::places::profiles::shown(folder, home),
            )),
            _ => None,
        },
        changed: Vec::new(),
        why_not: Some(why.words()),
        folder: None,
        picked_up: None,
        attests: false,
        can_wake: false,
    }
}

fn shown(target: &Target, rel: &Path) -> String {
    target.config_dir.join(rel).to_string_lossy().into_owned()
}

fn status_of(
    target: &Target,
    plan: &ConnectorPlan,
    record: Option<&ConnectorRecord>,
) -> ConnectorStatusInfo {
    let found = connector_files::status(&target.config_dir, plan, record);
    let peer = target.adapter.desktop_peer();
    ConnectorStatusInfo {
        app_id: target.app_id.clone(),
        state: state_word(found.state).to_owned(),
        // Offered here and not added yet.
        unavailable: (found.state == ConnectorState::Absent).then(|| "no_connector".to_owned()),
        unsafe_folder: None,
        changed: found.changed.iter().map(|rel| shown(target, rel)).collect(),
        why_not: None,
        folder: Some(target.config_dir.to_string_lossy().into_owned()),
        picked_up: Some(plan.picked_up.clone()),
        attests: peer.is_some_and(juggler_adapter::peer::DesktopPeer::attests),
        can_wake: peer.is_some_and(juggler_adapter::peer::DesktopPeer::can_wake),
    }
}

/// Where the connector stands in the App `app_id`.
///
/// # Errors
///
/// None in practice: an App it can't go into is `unavailable`, with why.
pub fn status_impl(state: &Arc<AppState>, app_id: &str) -> AppResult<ConnectorStatusInfo> {
    let target = match offered(state, app_id) {
        Ok(target) => target,
        Err(why) => return Ok(unavailable(app_id, &why)),
    };
    let plan = match plan_for(state, &target) {
        Ok(plan) => plan,
        Err(why) => return Ok(unavailable(app_id, &why)),
    };
    let records = load(state);
    Ok(status_of(&target, &plan, records.get(app_id)))
}

/// What Add would write into the App `app_id`, each file before and after.
///
/// # Errors
///
/// [`AppError::Refused`] when a part is outside the App's folder or
/// through a link (nothing is written either way).
pub fn preview_impl(state: &Arc<AppState>, app_id: &str) -> AppResult<ConnectorPreviewInfo> {
    let target = match offered(state, app_id) {
        Ok(target) => target,
        Err(why) => {
            return Ok(ConnectorPreviewInfo {
                status: unavailable(app_id, &why),
                files: Vec::new(),
                refused: None,
                guide: None,
            });
        }
    };
    let plan = plan_for(state, &target)?;
    let records = load(state);
    let record = records.get(app_id);
    let files = connector_files::preview(&target.config_dir, &plan, record)
        .map_err(|error| AppError::Refused(error.to_string()))?;
    let in_the_way: Vec<String> = files
        .iter()
        .filter(|file| file.action == connector_files::Action::Refuse)
        .map(|file| shown(&target, &file.path))
        .collect();
    Ok(ConnectorPreviewInfo {
        status: status_of(&target, &plan, record),
        refused: (!in_the_way.is_empty()).then(|| {
            format!(
                "{} is already there and isn't AI Juggler's as it wrote it, so Add won't replace it",
                in_the_way.join(", ")
            )
        }),
        files: files
            .into_iter()
            .map(|file| ConnectorFileInfo {
                path: shown(&target, &file.path),
                action: match file.action {
                    connector_files::Action::Create => "create",
                    connector_files::Action::Replace => "replace",
                    connector_files::Action::Merge => "merge",
                    connector_files::Action::Same => "same",
                    connector_files::Action::Refuse => "refuse",
                }
                .to_owned(),
                before: file.before,
                after: file.after,
            })
            .collect(),
        guide: Some(GetItInfo::of(&plan.guide, &target.label)),
    })
}

/// Adds the connector to the App `app_id` (the user's click on the card,
/// after the preview), and records what it wrote.
///
/// # Errors
///
/// [`AppError::Refused`] when it can't go there, or something AI Juggler
/// didn't write is in the way (nothing is written then); the write's
/// error (what was written is taken back).
pub fn add_impl(state: &Arc<AppState>, app_id: &str) -> AppResult<ConnectorStatusInfo> {
    let target = offered(state, app_id)?;
    let (plan, _writing, mut records) = writing(state, &target)?;
    let record = connector_files::add(&target.config_dir, &plan, records.get(app_id))
        .map_err(|error| AppError::Refused(error.to_string()))?;
    records.insert(app_id.to_owned(), record);
    if let Err(error) = save(state, &records) {
        // Unrecorded, it couldn't be removed exactly: take it back.
        if let Some(record) = records.get(app_id) {
            let _ = connector_files::remove(&target.config_dir, record);
        }
        return Err(error);
    }
    Ok(status_of(&target, &plan, records.get(app_id)))
}

/// Takes the connector out of the App `app_id`, all or nothing: when a
/// file it wrote was changed since, nothing is taken out and the refusal
/// names it (the window then offers [`remove_anyway_impl`]).
///
/// # Errors
///
/// [`AppError::Refused`] when AI Juggler has no record of adding it there,
/// or a file was changed since (named); a write's error.
pub fn remove_impl(state: &Arc<AppState>, app_id: &str) -> AppResult<ConnectorRemovalInfo> {
    remove_as(state, app_id, false)
}

/// The person's "Remove anyway (also deletes `<file>`)", after the window
/// confirmed it: the connector taken out whole, a changed file too.
///
/// # Errors
///
/// As [`remove_impl`], but never for a changed file.
pub fn remove_anyway_impl(state: &Arc<AppState>, app_id: &str) -> AppResult<ConnectorRemovalInfo> {
    remove_as(state, app_id, true)
}

/// What a write to `target` starts from: its plan, then the one writer
/// (held while the guard lives) and the records it reads under it.
fn writing(
    state: &Arc<AppState>,
    target: &Target,
) -> AppResult<(
    ConnectorPlan,
    std::sync::MutexGuard<'static, ()>,
    BTreeMap<String, ConnectorRecord>,
)> {
    let plan = plan_for(state, target)?;
    let guard = WRITING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok((plan, guard, load(state)))
}

fn remove_as(state: &Arc<AppState>, app_id: &str, anyway: bool) -> AppResult<ConnectorRemovalInfo> {
    let target = target(state, app_id)?;
    let (plan, _writing, mut records) = writing(state, &target)?;
    let record = records.get(app_id).cloned().ok_or_else(|| {
        AppError::Refused(
            "AI Juggler has no record of adding its connector to this app, so it takes nothing out"
                .to_owned(),
        )
    })?;
    let removal = if anyway {
        connector_files::remove_anyway(&target.config_dir, &record)
    } else {
        connector_files::remove(&target.config_dir, &record)
    }
    .map_err(|error| AppError::Refused(error.to_string()))?;
    match &removal.left {
        Some(left) => records.insert(app_id.to_owned(), left.clone()),
        None => records.remove(app_id),
    };
    save(state, &records)?;
    Ok(ConnectorRemovalInfo {
        status: status_of(&target, &plan, records.get(app_id)),
        removed: removal
            .removed
            .iter()
            .map(|rel| shown(&target, rel))
            .collect(),
        kept: removal.kept.iter().map(|rel| shown(&target, rel)).collect(),
    })
}

/// A pool's desktop members: each one's App, its connector, whether it's
/// connected, and how a new pool message reaches it.
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there; storage failures.
pub fn desktop_members_impl(
    state: &Arc<AppState>,
    pool_id: &str,
) -> AppResult<Vec<DesktopMemberInfo>> {
    let (rows, apps, wakes, fresh) = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .pools()
            .get(pool_id)
            .map_err(|_| AppError::Refused("that pool isn't there".to_owned()))?;
        let rows: Vec<MemberRef> = storage
            .pools()
            .members()?
            .into_iter()
            .filter(|row| row.pool_id == pool_id)
            .map(|row| row.member)
            .collect();
        let apps: Vec<juggler_core::storage::App> = storage
            .apps()
            .list()?
            .into_iter()
            .filter(|app| app.kind == "desktop" && !app.hidden)
            .collect();
        (
            rows,
            apps,
            crate::features::messages::delivery::pool_delivers(&storage, pool_id),
            storage.attested().fresh(),
        )
    };
    let mut statuses: BTreeMap<String, ConnectorStatusInfo> = BTreeMap::new();
    let mut status = |app_id: &str| {
        statuses
            .entry(app_id.to_owned())
            .or_insert_with(|| {
                status_impl(state, app_id).unwrap_or_else(|error| {
                    unavailable(app_id, &Why::UnknownApp(error.to_string()))
                })
            })
            .clone()
    };
    let mut out = Vec::new();
    for member in rows {
        let (app, connected) = match &member {
            MemberRef::Conversation { folder, id } => {
                let mut held = apps
                    .iter()
                    .filter(|app| app.folder_id.as_deref() == Some(folder.as_str()));
                let app = match (held.next(), held.next()) {
                    (Some(one), None) => Some(one),
                    _ => None,
                };
                let connected = fresh.iter().any(|facts| {
                    facts.conversation.as_ref() == Some(&(folder.clone(), id.clone()))
                });
                if app.is_none() && !connected {
                    continue; // another tool's terminal conversation, not a desktop app's
                }
                (app, connected)
            }
            MemberRef::App(id) => {
                let Some(app) = apps.iter().find(|app| app.id == *id) else {
                    continue;
                };
                let connected = fresh
                    .iter()
                    .any(|facts| facts.app.as_deref() == Some(id.as_str()));
                (Some(app), connected)
            }
            _ => continue,
        };
        let found = app.map(|app| status(&app.id));
        let can_wake = app
            .and_then(|app| state.registry.get(&HarnessId::new(app.tool.as_str())))
            .and_then(|adapter| {
                adapter
                    .desktop_peer()
                    .map(juggler_adapter::peer::DesktopPeer::can_wake)
            });
        let wake = match can_wake {
            Some(false) => "cant_wake",
            Some(true) if wakes => "wakes",
            _ => "next_turn",
        };
        out.push(DesktopMemberInfo {
            member: member.to_wire(),
            app_id: app.map(|app| app.id.clone()),
            connector: found.map_or_else(|| "unavailable".to_owned(), |found| found.state),
            connected,
            wake: wake.to_owned(),
        });
    }
    Ok(out)
}

/// AJ-E3013 (the lab's A6/A9): the desktop sessions `pool_id` holds that
/// read their messages through AI Juggler's connector, for its member and
/// recipient lists ([`dto::DesktopSessionInfo`]); Watched ones are
/// `pools_watched`'s.
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there; storage failures.
pub fn desktop_sessions_impl(
    state: &Arc<AppState>,
    pool_id: &str,
) -> AppResult<Vec<dto::DesktopSessionInfo>> {
    Ok(reach::of_pool(state, pool_id)?
        .into_iter()
        .filter(|one| one.connected)
        .map(|one| dto::DesktopSessionInfo {
            wake: if one.wakes { "wakes" } else { "next_turn" }.to_owned(),
            member: one.member,
            session_id: one.session,
            name: one.name,
            place: one.place,
            tool: one.tool,
            account: one.account,
            role: one.role,
            app_id: one.app_id,
            via: one.via,
        })
        .collect())
}

#[cfg(test)]
mod reach_tests;
#[cfg(test)]
mod tests;
