//! D-30: whether sessions AI Juggler starts get the agent tools. Asked
//! once ("Let agents in your projects see each other and work
//! together?"); after a yes it's on, and the user can switch it off for
//! a project or a profile in Settings › Agent access. Until the question
//! is answered, nothing is added.

use std::collections::BTreeSet;
use std::sync::Arc;

use juggler_core::storage::Storage;

use super::dto::AgentBridgeInfo;
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The user's answer: `on` or `off` (absent: not asked yet).
const ANSWER: &str = "agent_bridge.answer";
/// The projects switched off, a JSON list of ids.
const OFF_PROJECTS: &str = "agent_bridge.off_projects";
/// The profiles (account folders) switched off, a JSON list of ids.
const OFF_PROFILES: &str = "agent_bridge.off_profiles";

/// What the user chose.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Choice {
    /// `None` until the question is answered.
    pub on: Option<bool>,
    /// Projects switched off.
    pub off_projects: BTreeSet<String>,
    /// Profiles switched off.
    pub off_profiles: BTreeSet<String>,
}

fn list(storage: &Storage, key: &str) -> BTreeSet<String> {
    storage
        .app_settings()
        .get(key)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// What the user chose, as stored.
#[must_use]
pub fn read(storage: &Storage) -> Choice {
    let on = match storage.app_settings().get(ANSWER).ok().flatten().as_deref() {
        Some("on") => Some(true),
        Some("off") => Some(false),
        _ => None,
    };
    Choice {
        on,
        off_projects: list(storage, OFF_PROJECTS),
        off_profiles: list(storage, OFF_PROFILES),
    }
}

/// Whether a launch for `project` on `account` gets the tools, or why
/// not.
///
/// # Errors
///
/// The reason, in words, when it doesn't.
pub fn enabled_for(
    state: &Arc<AppState>,
    project: Option<&str>,
    account: &str,
) -> Result<(), &'static str> {
    let storage = state
        .storage
        .lock()
        .map_err(|_| "the settings couldn't be read")?;
    allows(&read(&storage), project, account)
}

/// Pure: whether `choice` lets a launch for `project` on `account` have
/// the tools.
///
/// # Errors
///
/// The reason, in words, when it doesn't.
pub fn allows(choice: &Choice, project: Option<&str>, account: &str) -> Result<(), &'static str> {
    match choice.on {
        None => Err("the user hasn't been asked yet"),
        Some(false) => Err("the user switched the agent tools off"),
        Some(true) if project.is_some_and(|id| choice.off_projects.contains(id)) => {
            Err("the user switched them off for this project")
        }
        Some(true) if choice.off_profiles.contains(account) => {
            Err("the user switched them off for this profile")
        }
        Some(true) => Ok(()),
    }
}

/// What Settings shows.
///
/// # Errors
///
/// Storage failures.
pub fn info(state: &Arc<AppState>) -> AppResult<AgentBridgeInfo> {
    let choice = read(&*lock_storage_arc(state)?);
    Ok(AgentBridgeInfo {
        asked: choice.on.is_some(),
        on: choice.on == Some(true),
        available: state.agent_bridge.program().is_some(),
        off_projects: choice.off_projects.into_iter().collect(),
        off_profiles: choice.off_profiles.into_iter().collect(),
    })
}

/// The user's answer to the question, or the switch for every session.
///
/// # Errors
///
/// Storage failures.
pub fn answer_impl(state: &Arc<AppState>, on: bool) -> AppResult<AgentBridgeInfo> {
    lock_storage_arc(state)?
        .app_settings()
        .set(ANSWER, if on { "on" } else { "off" })?;
    info(state)
}

/// Switches the tools on or off for one project (`scope` = `project`) or
/// one profile (`profile`). Counts from the next launch, and from the
/// next call for sessions already running.
///
/// # Errors
///
/// [`AppError::Validation`] for another scope; storage failures.
pub fn set_impl(
    state: &Arc<AppState>,
    scope: &str,
    id: &str,
    on: bool,
) -> AppResult<AgentBridgeInfo> {
    let key = match scope {
        "project" => OFF_PROJECTS,
        "profile" => OFF_PROFILES,
        other => {
            return Err(AppError::Validation {
                subject: "scope",
                message: format!("{other} isn't a project or a profile"),
            });
        }
    };
    {
        let storage = lock_storage_arc(state)?;
        let mut off = list(&storage, key);
        if on {
            off.remove(id);
        } else {
            off.insert(id.to_owned());
        }
        let raw = serde_json::to_string(&off).map_err(|error| AppError::Io {
            context: "the agent tools' settings",
            message: error.to_string(),
        })?;
        storage.app_settings().set(key, &raw)?;
    }
    info(state)
}

#[cfg(test)]
mod tests {
    use super::{Choice, allows};

    #[test]
    fn off_until_asked_then_on_except_where_switched_off() {
        let mut choice = Choice::default();
        assert!(
            allows(&choice, Some("p"), "a").is_err(),
            "not before the question"
        );
        choice.on = Some(false);
        assert!(allows(&choice, Some("p"), "a").is_err());
        choice.on = Some(true);
        assert!(allows(&choice, Some("p"), "a").is_ok());
        assert!(allows(&choice, None, "a").is_ok());
        choice.off_projects.insert("p".into());
        assert!(allows(&choice, Some("p"), "a").is_err());
        assert!(allows(&choice, Some("q"), "a").is_ok());
        choice.off_profiles.insert("a".into());
        assert!(allows(&choice, Some("q"), "a").is_err());
        assert!(allows(&choice, Some("q"), "b").is_ok());
    }
}
