//! D-62 (AJ-E3204): the Apps, as their page and lists read them. An App
//! is one app in one of its profiles: a CLI tool on a config folder
//! (`cli:<folder>`), or a desktop app in one of its instances
//! (`app:<host>:<instance folder>`, the places book's id). These rows
//! are identity only: the user's name for it, its folder, the Account
//! its folder was last read as. What runs and what it can start come
//! from the places (`places_list`, the same ids) and the folders.

pub mod commands;

use std::collections::{BTreeMap, BTreeSet};

use juggler_core::storage::App;
use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One App.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StoredAppInfo {
    /// `cli:<folder>` or `app:<host>:<instance folder>`.
    pub id: String,
    /// `cli` or `desktop`.
    pub kind: String,
    /// Its tool's id (Claude Code's, Codex's).
    pub tool: String,
    /// A desktop App's host app (`claude-app`, its icon).
    pub host_app_id: Option<String>,
    /// The user's name for it; `null`: the default ("Claude Code · Work").
    pub label: Option<String>,
    /// The config folder it uses (a CLI App's own; a desktop App's once
    /// its sessions name one).
    pub folder_id: Option<String>,
    /// The stored Account its folder was last read as.
    pub account_id: Option<String>,
    /// When it was last seen running, epoch ms (a CLI App: when made).
    #[ts(type = "number")]
    pub last_seen_ms: i64,
    /// AJ-E3206: its colour, given once and stored (an index into the
    /// UI's `MARK_COLOURS`); the same on every screen that marks it.
    #[ts(optional = nullable)]
    pub colour_slot: Option<u8>,
}

/// Every App the user hasn't hidden ([`users_apps`]): a CLI App per
/// folder in use (made here for a folder added since v40), and each
/// desktop App the user added.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn list_impl(state: &Arc<AppState>) -> AppResult<Vec<StoredAppInfo>> {
    let storage = lock_storage_arc(state)?;
    let now = crate::commands::now_millis();
    let folders = storage.accounts().list()?;
    let in_use: BTreeSet<String> = folders
        .iter()
        .map(|folder| folder.id.as_str().to_owned())
        .collect();
    let run_in_a_terminal = storage.apps().folders_run_in_a_terminal()?;
    let before = storage.apps().list()?;
    let profiles = profile_folders(&before);
    for folder in &folders {
        let id = folder.id.as_str();
        if !profiles.contains(id) || run_in_a_terminal.contains(id) {
            storage
                .apps()
                .upsert_cli(folder.harness.as_str(), id, now)?;
        }
    }
    let accounts: BTreeMap<String, Option<String>> =
        storage.subscriptions().folders()?.into_iter().collect();
    let slots = storage.apps().colour_slots()?;
    Ok(
        users_apps(storage.apps().list()?, &in_use, &run_in_a_terminal)
            .into_iter()
            .map(|app| StoredAppInfo {
                account_id: app
                    .folder_id
                    .as_ref()
                    .and_then(|folder| accounts.get(folder).cloned().flatten()),
                colour_slot: slots.get(&app.id).copied(),
                id: app.id,
                kind: app.kind,
                tool: app.tool,
                host_app_id: app.host_app_id,
                label: app.label,
                folder_id: app.folder_id,
                last_seen_ms: app.last_seen,
            })
            .collect(),
    )
}

/// The folders the user's desktop Apps run their sessions on.
fn profile_folders(apps: &[App]) -> BTreeSet<String> {
    apps.iter()
        .filter(|app| app.kind == "desktop" && !app.hidden && app.is_users())
        .filter_map(|app| app.folder_id.clone())
        .collect()
}

/// D-84: the user's Apps, as every list, count and the Floor read them:
/// the user's own ([`App::is_users`]), not hidden, and a CLI App only
/// while its folder is in use (`folders_in_use`). An app only seen
/// running is an offer, never an App. rc22 (A3): a folder brought in for
/// a desktop profile (its Code sessions' folder, the user's Add) is that
/// profile's, not a second App: its CLI App is one only once AI Juggler
/// ran a terminal session on it (`run_in_a_terminal`) or the user named
/// it.
#[must_use]
pub fn users_apps(
    apps: Vec<App>,
    folders_in_use: &BTreeSet<String>,
    run_in_a_terminal: &BTreeSet<String>,
) -> Vec<App> {
    let profiles = profile_folders(&apps);
    apps.into_iter()
        .filter(|app| !app.hidden)
        .filter(App::is_users)
        .filter(|app| {
            app.kind != "cli"
                || app.folder_id.as_deref().is_some_and(|id| {
                    folders_in_use.contains(id)
                        && (!profiles.contains(id)
                            || run_in_a_terminal.contains(id)
                            || app.label.is_some())
                })
        })
        .collect()
}

/// Names App `id` (`None` or blank: back to its default name).
///
/// # Errors
///
/// [`AppError::Refused`] for an App that isn't stored; storage errors.
pub fn rename_impl(state: &Arc<AppState>, id: &str, label: Option<&str>) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    if storage.apps().get(id)?.is_none() {
        return Err(AppError::Refused(
            "AI Juggler doesn't know that app".to_owned(),
        ));
    }
    if label.is_some_and(|label| label.trim().chars().count() > 60) {
        return Err(AppError::Refused(
            "A name is 60 characters at most".to_owned(),
        ));
    }
    storage.apps().rename(id, label)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_core::model::{Account, AccountId, HarnessId};

    fn folder(id: &str) -> Account {
        Account {
            id: AccountId::new(id),
            harness: HarnessId::new("claude-code"),
            label: id.into(),
            plan: None,
            state_dir: format!("/tmp/aij-test/{id}").into(),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        }
    }

    #[test]
    fn every_folder_in_use_is_a_cli_app_and_a_named_one_keeps_its_name() {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(root.path().join("data")).unwrap());
        {
            let storage = state.storage.lock().unwrap();
            storage.accounts().insert(&folder("work")).unwrap();
            storage.accounts().insert(&folder("old")).unwrap();
            storage
                .accounts()
                .mark_removed(&AccountId::new("old"))
                .unwrap();
        }
        let apps = list_impl(&state).unwrap();
        let ids: Vec<&str> = apps.iter().map(|app| app.id.as_str()).collect();
        assert!(ids.contains(&"cli:work"), "{ids:?}");
        assert!(!ids.contains(&"cli:old"), "a removed folder's App goes");
        rename_impl(&state, "cli:work", Some(" Day job ")).unwrap();
        let named = list_impl(&state).unwrap();
        let work = named.iter().find(|app| app.id == "cli:work").unwrap();
        assert_eq!(work.label.as_deref(), Some("Day job"));
        assert!(rename_impl(&state, "cli:nope", Some("x")).is_err());
        assert!(rename_impl(&state, "cli:work", Some(&"x".repeat(61))).is_err());
    }

    /// AJ-E3206: an App's colour is the one stored for it, the same on its
    /// row as on its folder's, and a name never moves it.
    #[test]
    fn an_apps_colour_is_its_stored_slot_on_every_row_that_names_it() {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(root.path().join("data")).unwrap());
        {
            let storage = state.storage.lock().unwrap();
            for id in ["zeta", "alpha"] {
                storage.accounts().insert(&folder(id)).unwrap();
            }
        }
        // A folder's row first: it makes its CLI App (and slot) at once.
        let folders = crate::features::account::accounts_info(&state).unwrap();
        let slot_of = |id: &str| {
            folders
                .iter()
                .find(|folder| folder.id == id)
                .and_then(|folder| folder.app_colour_slot)
        };
        let (zeta, alpha) = (slot_of("zeta"), slot_of("alpha"));
        assert!(zeta.is_some() && alpha.is_some());
        assert_ne!(zeta, alpha, "two Apps, two colours");
        let listed = |state: &Arc<AppState>| {
            list_impl(state)
                .unwrap()
                .into_iter()
                .map(|app| (app.id, app.colour_slot))
                .collect::<BTreeMap<_, _>>()
        };
        let apps = listed(&state);
        assert_eq!(apps["cli:zeta"], zeta);
        assert_eq!(apps["cli:alpha"], alpha);
        rename_impl(&state, "cli:zeta", Some("Aardvark")).unwrap();
        assert_eq!(listed(&state)["cli:zeta"], zeta, "renamed: the same colour");
        assert!(
            crate::features::account::accounts_info(&state)
                .unwrap()
                .iter()
                .all(|folder| folder.account_colour_slot.is_none()),
            "no Account read yet: none to colour"
        );
    }
}
