//! AJ-E1501: managing accounts without leaving AI Juggler — rename,
//! remove, and the account new sessions start with.
//!
//! - A rename changes the label everywhere it shows (every view reads it
//!   from storage), and the labels a project's imported history keeps.
//! - Remove forgets the account; nothing is deleted. An account used
//!   where it is (ADR-0018) keeps its folder untouched; a copy's folder
//!   stays, or goes to the Trash when the user chooses that. An account
//!   with open sessions is refused, naming them. Its sessions' history
//!   stays readable, marked "removed account".
//! - The default: a project's own pick, else the global one, else the
//!   most recently used account (the UI's rule, as before).
//! - Every change is an audit row (who, what, when).
//!
//! AJ-E1502: sessions and projects can be named, pinned, opened and
//! tidied — a session's label over its own title, pins that lead the
//! rail, a project's label (the folder is never renamed), open in the
//! user's editor or Finder, Forget (hidden until used again) and Relocate
//! (a moved folder). Labels and pins live in the database, so they
//! survive restarts and backups.

// Tauri command arguments arrive owned (deserialized).
#![allow(clippy::needless_pass_by_value)]

use std::path::Path;
use std::sync::Arc;

use juggler_core::model::{Account, AccountId, ProjectId, SessionStatus};
use serde::Serialize;
use ts_rs::TS;

use crate::commands::{AccountInfo, accounts_info, lock_storage_arc};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The `app_settings` key holding the global default account's id.
pub const DEFAULT_ACCOUNT_KEY: &str = "accounts.default";

/// Who made a change made in the app (the audit rows' actor).
const ACTOR_USER: &str = "user";

/// The longest label accepted (a label is a name, not a note).
pub(crate) const LABEL_MAX: usize = 64;

/// What Remove does with a copy's folder (an account used where it is
/// always keeps its folder).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyFolder {
    /// The copy's folder stays where it is.
    Keep,
    /// The copy's folder goes to the Trash (never deleted).
    Trash,
}

impl CopyFolder {
    /// The wire word (`keep` | `trash`); anything else keeps the folder.
    #[must_use]
    pub fn from_wire(word: &str) -> Self {
        if word == "trash" {
            Self::Trash
        } else {
            Self::Keep
        }
    }
}

/// What a removal did, for the UI's confirmation line.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountRemovedInfo {
    /// The account removed.
    pub id: String,
    /// Its label.
    pub label: String,
    /// `untouched` (used where it is), `kept` (a copy's folder stays) or
    /// `trashed` (a copy's folder went to the Trash).
    pub folder: String,
}

/// One audit row for the UI.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountAuditInfo {
    /// What was done (`rename`, `plan`, `remove`, `default`, `project_default`).
    pub action: String,
    /// The change in a few words.
    pub detail: Option<String>,
    /// Who did it.
    pub actor: String,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at: i64,
}

/// Statuses a session can still run or resume from: removing its
/// account would strand it.
fn is_open(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Starting
            | SessionStatus::Running
            | SessionStatus::WaitingUser
            | SessionStatus::Stuck
            | SessionStatus::QuotaBlocked
    )
}

/// A session's name on screen: its branch, else its folder (the rail's
/// rule).
fn session_place(cwd: &Path) -> String {
    crate::worktree::branch_of(cwd).unwrap_or_else(|| {
        cwd.file_name().map_or_else(
            || cwd.to_string_lossy().into_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
    })
}

/// An active (not removed) account, or a refusal in words.
fn active_account(
    storage: &juggler_core::storage::Storage,
    account_id: &str,
) -> AppResult<juggler_core::model::Account> {
    let id = AccountId::new(account_id);
    let account = storage
        .accounts()
        .get(&id)
        .map_err(|_| AppError::NotFound {
            entity: "account",
            id: account_id.to_owned(),
        })?;
    if storage.accounts().is_removed(&id)? {
        return Err(AppError::Refused(format!(
            "«{}» was removed — add it again to use it",
            account.label
        )));
    }
    Ok(account)
}

/// Renames an account and sets its plan (`None` or blank clears it).
/// A label another account already has is refused in words.
///
/// # Errors
///
/// [`AppError::Refused`] for an empty, overlong or duplicate label or a
/// removed account; [`AppError::NotFound`] for an unknown id; storage
/// errors.
pub fn account_rename_impl(
    state: &Arc<AppState>,
    account_id: &str,
    label: &str,
    plan: Option<&str>,
) -> AppResult<AccountInfo> {
    let label = label.trim();
    if label.is_empty() {
        return Err(AppError::Refused("Give the account a name".into()));
    }
    if label.chars().count() > LABEL_MAX {
        return Err(AppError::Refused(format!(
            "Keep the name to {LABEL_MAX} characters or fewer"
        )));
    }
    let plan = plan.map(str::trim).filter(|plan| !plan.is_empty());
    let storage = lock_storage_arc(state)?;
    let mut account = active_account(&storage, account_id)?;
    if let Some(other) = storage.accounts().list()?.into_iter().find(|other| {
        other.id != account.id && other.label.trim().to_lowercase() == label.to_lowercase()
    }) {
        return Err(AppError::Refused(format!(
            "Another account is already called «{}» — pick another name",
            other.label
        )));
    }
    let old_label = account.label.clone();
    let old_plan = account.plan.as_ref().map(ToString::to_string);
    let renamed = old_label != label;
    let replanned = old_plan.as_deref() != plan;
    if !renamed && !replanned {
        drop(storage);
        return account_info(state, account_id);
    }
    label.clone_into(&mut account.label);
    account.plan = plan.map(juggler_core::model::PlanTier::new);
    storage.accounts().update(&account)?;
    if renamed {
        // Imported history keeps labels, not ids: follow the new name so
        // "accounts used here" (and the New session pick) don't lose it.
        for project in storage.projects().list()? {
            if project.history.accounts.contains(&old_label) {
                let mut history = project.history.clone();
                let mut seen = std::collections::BTreeSet::new();
                history.accounts = history
                    .accounts
                    .into_iter()
                    .map(|entry| {
                        if entry == old_label {
                            label.to_owned()
                        } else {
                            entry
                        }
                    })
                    .filter(|entry| seen.insert(entry.clone()))
                    .collect();
                storage.projects().merge_history(&project.id, &history)?;
            }
        }
        storage.account_audit().record(
            &account.id,
            "rename",
            Some(&format!("«{old_label}» → «{label}»")),
            ACTOR_USER,
        )?;
    }
    if replanned {
        storage.account_audit().record(
            &account.id,
            "plan",
            Some(&format!(
                "{} → {}",
                old_plan.as_deref().unwrap_or("none"),
                plan.unwrap_or("none")
            )),
            ACTOR_USER,
        )?;
    }
    drop(storage);
    account_info(state, account_id)
}

/// Before an account used where it is goes: the user's own status line
/// back, by the same rules as turning measuring off (exactly as it was,
/// or only AI Juggler's value taken out). If it can't be, the account
/// stays: a removed account must never leave their line pointing at AI
/// Juggler (AJ-E1530, the docs fact-check).
fn status_line_back(state: &Arc<AppState>, account: &Account) -> AppResult<()> {
    let measured = juggler_adapters::quota_ingest::outside_capture_record(
        &state.overlays_dir(),
        account.id.as_str(),
    )
    .is_some();
    if !measured {
        return Ok(());
    }
    crate::features::quota::capture::turn_off_impl(state, account.id.as_str())
        .map(|_| ())
        .map_err(|error| {
            AppError::Refused(format!(
                "«{}»'s status line couldn't be put back ({error}), so the account stays. \
                 Turn off measuring on its page, then remove it",
                account.label
            ))
        })
}

/// Removes (forgets) an account. Refused while it has open sessions,
/// naming them. An account used where it is never has its folder
/// touched; a copy's folder stays, or goes to the Trash through `trash`
/// when `folder` says so — only a folder inside AI Juggler's own data.
///
/// # Errors
///
/// [`AppError::Refused`] for open sessions, a removed account, or a
/// Trash choice that doesn't apply; [`AppError::Io`] when the Trash
/// refused (then nothing was removed); storage errors.
pub fn account_remove_impl(
    state: &Arc<AppState>,
    account_id: &str,
    folder: CopyFolder,
    trash: &dyn Fn(&Path) -> std::io::Result<()>,
) -> AppResult<AccountRemovedInfo> {
    let storage = lock_storage_arc(state)?;
    let account = active_account(&storage, account_id)?;
    let recipes: Vec<_> = storage
        .profiles()
        .list_by_account(&account.id)?
        .into_iter()
        .map(|recipe| recipe.id)
        .collect();
    let mut open: Vec<String> = storage
        .sessions()
        .list(None)?
        .into_iter()
        .filter(|record| recipes.contains(&record.profile) && is_open(record.status))
        .map(|record| session_place(&record.cwd))
        .collect();
    let count = open.len();
    open.sort();
    open.dedup();
    if count > 0 {
        let names = open.join(", ");
        return Err(AppError::Refused(format!(
            "«{}» has {} open: {names}. Stop {} first, then remove the account",
            account.label,
            if count == 1 {
                "a session".to_owned()
            } else {
                format!("{count} sessions")
            },
            if count == 1 { "it" } else { "them" },
        )));
    }
    // AJ-E1530: the user's own status line goes back first.
    let storage = if account.in_place && folder == CopyFolder::Keep {
        drop(storage);
        status_line_back(state, &account)?;
        lock_storage_arc(state)?
    } else {
        storage
    };
    let outcome = if account.in_place {
        if folder == CopyFolder::Trash {
            return Err(AppError::Refused(format!(
                "«{}» uses your own folder — AI Juggler never moves it",
                account.label
            )));
        }
        "untouched"
    } else if folder == CopyFolder::Trash {
        let ours = state.accounts_dir();
        let ours = ours.canonicalize().unwrap_or(ours);
        let dir = account
            .state_dir
            .canonicalize()
            .unwrap_or_else(|_| account.state_dir.clone());
        if !dir.starts_with(&ours) || dir == ours {
            return Err(AppError::Refused(format!(
                "«{}»'s folder isn't in AI Juggler's own data, so it stays where it is",
                account.label
            )));
        }
        if dir.exists() {
            // Before anything is forgotten: if the Trash refuses, nothing
            // changed.
            trash(&dir).map_err(|error| AppError::io("move the copy to the Trash", &error))?;
        }
        "trashed"
    } else {
        "kept"
    };
    storage.accounts().mark_removed(&account.id)?;
    storage.projects().clear_default_account(&account.id)?;
    if storage.app_settings().get(DEFAULT_ACCOUNT_KEY)?.as_deref() == Some(account.id.as_str()) {
        storage.app_settings().set(DEFAULT_ACCOUNT_KEY, "")?;
    }
    let detail = match outcome {
        "untouched" => "used where it is: folder untouched".to_owned(),
        "trashed" => format!(
            "copy's folder moved to the Trash ({})",
            account.state_dir.display()
        ),
        _ => format!("copy's folder kept ({})", account.state_dir.display()),
    };
    storage
        .account_audit()
        .record(&account.id, "remove", Some(&detail), ACTOR_USER)?;
    drop(storage);
    crate::usage_pipeline::forget_account_offsets(state, account.id.as_str());
    // AJ-E1601 (D-34): through another service, its link goes, and with
    // the account's last profile, the key in the Keychain.
    crate::features::service_accounts::profile_removed(state, account.id.as_str())?;
    Ok(AccountRemovedInfo {
        id: account.id.to_string(),
        label: account.label,
        folder: outcome.to_owned(),
    })
}

/// Makes an account the global default (`None` clears it: the most
/// recently used account is the default again).
///
/// # Errors
///
/// [`AppError::Refused`] for a removed account; [`AppError::NotFound`];
/// storage errors.
pub fn account_set_default_impl(
    state: &Arc<AppState>,
    account_id: Option<&str>,
) -> AppResult<Vec<AccountInfo>> {
    let storage = lock_storage_arc(state)?;
    let previous = storage.app_settings().get(DEFAULT_ACCOUNT_KEY)?;
    match account_id {
        Some(id) => {
            let account = active_account(&storage, id)?;
            if previous.as_deref() != Some(id) {
                storage.app_settings().set(DEFAULT_ACCOUNT_KEY, id)?;
                storage.account_audit().record(
                    &account.id,
                    "default",
                    Some("made the default for new sessions"),
                    ACTOR_USER,
                )?;
            }
        }
        None => {
            if let Some(previous) = previous {
                storage.app_settings().set(DEFAULT_ACCOUNT_KEY, "")?;
                storage.account_audit().record(
                    &AccountId::new(previous),
                    "default",
                    Some("no longer the default"),
                    ACTOR_USER,
                )?;
            }
        }
    }
    drop(storage);
    accounts_info(state)
}

/// Sets (or clears) the account a project's new sessions start with.
///
/// # Errors
///
/// [`AppError::Refused`] for a removed account; [`AppError::NotFound`]
/// for an unknown project or account; storage errors.
pub fn project_set_default_account_impl(
    state: &Arc<AppState>,
    project_id: &str,
    account_id: Option<&str>,
) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    let project_key = ProjectId::new(project_id);
    let project = storage
        .projects()
        .get(&project_key)
        .map_err(|_| AppError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        })?;
    let account = account_id
        .map(|id| active_account(&storage, id))
        .transpose()?;
    if project.default_account.as_ref() == account.as_ref().map(|a| &a.id) {
        return Ok(());
    }
    storage
        .projects()
        .set_default_account(&project_key, account.as_ref().map(|a| &a.id))?;
    let (audited, detail) = match (&account, &project.default_account) {
        (Some(account), _) => (
            account.id.clone(),
            format!("the default in {}", project.name),
        ),
        (None, Some(previous)) => (
            previous.clone(),
            format!("no longer the default in {}", project.name),
        ),
        (None, None) => return Ok(()),
    };
    storage
        .account_audit()
        .record(&audited, "project_default", Some(&detail), ACTOR_USER)?;
    Ok(())
}

/// An account's audit rows, oldest first.
///
/// # Errors
///
/// Storage errors.
pub fn account_audit_impl(
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<Vec<AccountAuditInfo>> {
    Ok(lock_storage_arc(state)?
        .account_audit()
        .list_for(&AccountId::new(account_id))?
        .into_iter()
        .map(|row| AccountAuditInfo {
            action: row.action,
            detail: row.detail,
            actor: row.actor,
            at: row.at,
        })
        .collect())
}

/// One account's row, as the list shows it.
fn account_info(state: &Arc<AppState>, account_id: &str) -> AppResult<AccountInfo> {
    accounts_info(state)?
        .into_iter()
        .find(|info| info.id == account_id)
        .ok_or_else(|| AppError::NotFound {
            entity: "account",
            id: account_id.to_owned(),
        })
}

/// Moves a folder to the user's Trash (`NSFileManager trashItem` on
/// macOS — never Finder scripting, which would ask for automation
/// access). Elsewhere it refuses: nothing is ever deleted instead.
///
/// # Errors
///
/// The Trash's own refusal, or `Unsupported` off macOS.
pub fn move_to_trash(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        let mut context = trash::TrashContext::default();
        context.set_delete_method(DeleteMethod::NsFileManager);
        context
            .delete(path)
            .map_err(|error| std::io::Error::other(error.to_string()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "moving to the Trash isn't available on this system yet",
        ))
    }
}

/// Rename an account (and set its plan).
///
/// # Errors
///
/// As [`account_rename_impl`].
#[tauri::command]
pub async fn account_rename(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    label: String,
    plan: Option<String>,
) -> AppResult<AccountInfo> {
    crate::ipc::blocking(state.inner(), "renaming the account", move |state| {
        account_rename_impl(state, &account_id, &label, plan.as_deref())
    })
    .await
}

/// Remove an account; `folder` is `keep` or `trash` (a copy's folder).
///
/// # Errors
///
/// As [`account_remove_impl`].
#[tauri::command]
pub async fn account_remove(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    folder: String,
) -> AppResult<AccountRemovedInfo> {
    crate::ipc::blocking(state.inner(), "removing the account", move |state| {
        account_remove_impl(
            state,
            &account_id,
            CopyFolder::from_wire(&folder),
            &move_to_trash,
        )
    })
    .await
}

/// Make an account the default for new sessions (`null` clears it).
///
/// # Errors
///
/// As [`account_set_default_impl`].
#[tauri::command]
pub async fn account_set_default(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: Option<String>,
) -> AppResult<Vec<AccountInfo>> {
    crate::ipc::blocking(state.inner(), "setting the default account", move |state| {
        account_set_default_impl(state, account_id.as_deref())
    })
    .await
}

/// Pick the account a project's new sessions start with (`null` clears).
///
/// # Errors
///
/// As [`project_set_default_account_impl`].
#[tauri::command]
pub async fn project_set_default_account(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    account_id: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(
        state.inner(),
        "setting the project's account",
        move |state| project_set_default_account_impl(state, &project_id, account_id.as_deref()),
    )
    .await
}

/// An account's audit rows.
///
/// # Errors
///
/// As [`account_audit_impl`].
#[tauri::command]
pub async fn account_audit(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<Vec<AccountAuditInfo>> {
    crate::ipc::blocking(
        state.inner(),
        "reading the account's history",
        move |state| account_audit_impl(state, &account_id),
    )
    .await
}

// ---------------------------------------------------------------------------
// AJ-E1502: sessions and projects

/// The `app_settings` key holding the editor projects open in (an app's
/// name, as Finder knows it); unset = Finder.
pub const EDITOR_KEY: &str = "projects.editor";

/// The longest session label accepted.
const SESSION_LABEL_MAX: usize = 80;

/// Editors AI Juggler offers when they are installed: (name shown and
/// passed to `open -a`, the app bundle's file name).
const KNOWN_EDITORS: &[(&str, &str)] = &[
    ("Visual Studio Code", "Visual Studio Code.app"),
    ("Cursor", "Cursor.app"),
    ("Windsurf", "Windsurf.app"),
    ("Zed", "Zed.app"),
    ("Xcode", "Xcode.app"),
    ("Sublime Text", "Sublime Text.app"),
    ("Nova", "Nova.app"),
    ("BBEdit", "BBEdit.app"),
    ("IntelliJ IDEA", "IntelliJ IDEA.app"),
    ("WebStorm", "WebStorm.app"),
    ("PyCharm", "PyCharm.app"),
];

/// Where a project opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenIn {
    /// The editor chosen in Settings, else a Finder window.
    Editor,
    /// Finder, with the folder selected.
    Finder,
}

/// What opening a project does: the folder, and the app (`None` =
/// Finder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenTarget {
    /// The project's folder.
    pub folder: std::path::PathBuf,
    /// The editor's name, or `None` for Finder.
    pub app: Option<String>,
    /// Reveal (select it in its parent) rather than open it.
    pub reveal: bool,
}

/// The editor choice for Settings.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct EditorChoicesInfo {
    /// The installed editors AI Juggler knows, by name.
    pub installed: Vec<String>,
    /// The one chosen, if any (`null` = Finder).
    pub chosen: Option<String>,
}

/// A session, or a refusal in words.
fn known_session(
    storage: &juggler_core::storage::Storage,
    session_id: &str,
) -> AppResult<juggler_core::model::InstanceRecord> {
    storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session_id))
        .map_err(|_| AppError::NotFound {
            entity: "session",
            id: session_id.to_owned(),
        })
}

/// A project, or a refusal in words.
fn known_project(
    storage: &juggler_core::storage::Storage,
    project_id: &str,
) -> AppResult<juggler_core::model::ProjectRecord> {
    storage
        .projects()
        .get(&ProjectId::new(project_id))
        .map_err(|_| AppError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        })
}

/// Names a session (a label over its own title); `None` or blank goes
/// back to the title.
///
/// # Errors
///
/// [`AppError::Refused`] for an overlong label; [`AppError::NotFound`];
/// storage errors.
pub fn session_rename_impl(
    state: &Arc<AppState>,
    session_id: &str,
    label: Option<&str>,
) -> AppResult<()> {
    let label = label.map(str::trim).filter(|label| !label.is_empty());
    if label.is_some_and(|label| label.chars().count() > SESSION_LABEL_MAX) {
        return Err(AppError::Refused(format!(
            "Keep the name to {SESSION_LABEL_MAX} characters or fewer"
        )));
    }
    let storage = lock_storage_arc(state)?;
    let session = known_session(&storage, session_id)?;
    storage.sessions().set_label(&session.id, label)?;
    Ok(())
}

/// Pins a session to the top of its group in the rail, or unpins it.
///
/// # Errors
///
/// [`AppError::NotFound`]; storage errors.
pub fn session_set_pinned_impl(
    state: &Arc<AppState>,
    session_id: &str,
    pinned: bool,
) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    let session = known_session(&storage, session_id)?;
    storage.sessions().set_pinned(&session.id, pinned)?;
    Ok(())
}

/// Renames a project: its label only, the folder is never renamed.
/// Blank goes back to the folder's name.
///
/// # Errors
///
/// [`AppError::Refused`] for an overlong name; [`AppError::NotFound`];
/// storage errors.
pub fn project_rename_impl(state: &Arc<AppState>, project_id: &str, name: &str) -> AppResult<()> {
    let name = name.trim();
    if name.chars().count() > LABEL_MAX {
        return Err(AppError::Refused(format!(
            "Keep the name to {LABEL_MAX} characters or fewer"
        )));
    }
    let storage = lock_storage_arc(state)?;
    let project = known_project(&storage, project_id)?;
    let name = if name.is_empty() {
        project.root.file_name().map_or_else(
            || project.root.to_string_lossy().into_owned(),
            |folder| folder.to_string_lossy().into_owned(),
        )
    } else {
        name.to_owned()
    };
    storage.projects().rename(&project.id, &name)?;
    Ok(())
}

/// Pins a project to the top of the rail, or unpins it.
///
/// # Errors
///
/// [`AppError::NotFound`]; storage errors.
pub fn project_set_pinned_impl(
    state: &Arc<AppState>,
    project_id: &str,
    pinned: bool,
) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    let project = known_project(&storage, project_id)?;
    storage.projects().set_pinned(&project.id, pinned)?;
    Ok(())
}

/// Forgets a project — hidden from the rail and New session until it is
/// used again (a session started there, or work there outside AI
/// Juggler) — or brings it back. Nothing on disk changes; its sessions
/// stay in History. Refused while a session there is open, naming it.
///
/// # Errors
///
/// [`AppError::Refused`] for open sessions; [`AppError::NotFound`];
/// storage errors.
pub fn project_forget_impl(
    state: &Arc<AppState>,
    project_id: &str,
    forgotten: bool,
) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    let project = known_project(&storage, project_id)?;
    if forgotten {
        let mut open: Vec<String> = storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter(|record| record.project.as_ref() == Some(&project.id) && is_open(record.status))
            .map(|record| session_place(&record.cwd))
            .collect();
        let count = open.len();
        open.sort();
        open.dedup();
        if count > 0 {
            return Err(AppError::Refused(format!(
                "{} has {} open: {}. Stop {} first, then forget the project",
                project.name,
                if count == 1 {
                    "a session".to_owned()
                } else {
                    format!("{count} sessions")
                },
                open.join(", "),
                if count == 1 { "it" } else { "them" },
            )));
        }
    }
    storage.projects().set_forgotten(&project.id, forgotten)?;
    Ok(())
}

/// Whether a project is forgotten now: forgotten, and not worked in
/// since (outside AI Juggler, by its imported history).
#[must_use]
pub fn is_forgotten(project: &juggler_core::model::ProjectRecord) -> bool {
    project.forgotten_at.is_some_and(|at| {
        project
            .history
            .last_used_ms
            .is_none_or(|used| used <= at.millis())
    })
}

/// Repoints a project at the folder it moved to. The folder must exist,
/// and no other project may have it.
///
/// # Errors
///
/// [`AppError::Refused`] for a missing folder or one another project
/// has; [`AppError::NotFound`]; storage errors.
pub fn project_relocate_impl(
    state: &Arc<AppState>,
    project_id: &str,
    folder: &Path,
) -> AppResult<()> {
    if !folder.is_dir() {
        return Err(AppError::Refused(format!(
            "{} isn't a folder on this Mac — pick the folder the project moved to",
            folder.display()
        )));
    }
    let root = folder
        .canonicalize()
        .map_err(|error| AppError::io("relocate", &error))?;
    let storage = lock_storage_arc(state)?;
    let project = known_project(&storage, project_id)?;
    if project.root == root {
        return Ok(());
    }
    if let Some(other) = storage.projects().by_root(&root)? {
        return Err(AppError::Refused(format!(
            "{} already uses that folder",
            other.name
        )));
    }
    storage.projects().relocate(&project.id, &root)?;
    Ok(())
}

/// What opening a project does (shared with tests; the command opens
/// it). Refused when the folder is gone.
///
/// # Errors
///
/// [`AppError::Refused`] for a missing folder; [`AppError::NotFound`];
/// storage errors.
pub fn project_open_target_impl(
    state: &Arc<AppState>,
    project_id: &str,
    how: OpenIn,
) -> AppResult<OpenTarget> {
    let storage = lock_storage_arc(state)?;
    let project = known_project(&storage, project_id)?;
    if !project.root.is_dir() {
        return Err(AppError::Refused(format!(
            "{}'s folder isn't there any more — use Relocate… to point it at where it moved",
            project.name
        )));
    }
    let app = match how {
        OpenIn::Editor => storage.app_settings().get(EDITOR_KEY)?,
        OpenIn::Finder => None,
    };
    Ok(OpenTarget {
        folder: project.root,
        reveal: how == OpenIn::Finder,
        app,
    })
}

/// The editors installed in `apps` folders that AI Juggler knows, and
/// the one chosen.
///
/// # Errors
///
/// Storage errors.
pub fn editor_choices_impl(
    state: &Arc<AppState>,
    apps: &[std::path::PathBuf],
) -> AppResult<EditorChoicesInfo> {
    let installed = KNOWN_EDITORS
        .iter()
        .filter(|(_, bundle)| apps.iter().any(|dir| dir.join(bundle).exists()))
        .map(|(name, _)| (*name).to_owned())
        .collect();
    let chosen = lock_storage_arc(state)?.app_settings().get(EDITOR_KEY)?;
    Ok(EditorChoicesInfo { installed, chosen })
}

/// Chooses the editor projects open in (`None` = Finder). Only an
/// editor AI Juggler knows is accepted.
///
/// # Errors
///
/// [`AppError::Refused`] for an unknown editor; storage errors.
pub fn editor_set_impl(state: &Arc<AppState>, editor: Option<&str>) -> AppResult<()> {
    if let Some(name) = editor
        && !KNOWN_EDITORS.iter().any(|(known, _)| *known == name)
    {
        return Err(AppError::Refused(format!(
            "AI Juggler doesn't know how to open {name}"
        )));
    }
    lock_storage_arc(state)?
        .app_settings()
        .set(EDITOR_KEY, editor.unwrap_or(""))?;
    Ok(())
}

/// The folders apps are installed in.
fn application_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = vec![std::path::PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Applications"));
    }
    dirs
}

/// Name a session (`null` or blank: its own title again).
///
/// # Errors
///
/// As [`session_rename_impl`].
#[tauri::command]
pub async fn session_rename(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    label: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "naming the session", move |state| {
        session_rename_impl(state, &session_id, label.as_deref())
    })
    .await
}

/// Pin a session to the top of the rail, or unpin it.
///
/// # Errors
///
/// As [`session_set_pinned_impl`].
#[tauri::command]
pub async fn session_set_pinned(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pinned: bool,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "pinning the session", move |state| {
        session_set_pinned_impl(state, &session_id, pinned)
    })
    .await
}

/// Rename a project (blank: the folder's name again).
///
/// # Errors
///
/// As [`project_rename_impl`].
#[tauri::command]
pub async fn project_rename(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    name: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "renaming the project", move |state| {
        project_rename_impl(state, &project_id, &name)
    })
    .await
}

/// Pin a project to the top of the rail, or unpin it.
///
/// # Errors
///
/// As [`project_set_pinned_impl`].
#[tauri::command]
pub async fn project_set_pinned(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    pinned: bool,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "pinning the project", move |state| {
        project_set_pinned_impl(state, &project_id, pinned)
    })
    .await
}

/// Forget a project (or bring it back).
///
/// # Errors
///
/// As [`project_forget_impl`].
#[tauri::command]
pub async fn project_forget(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    forgotten: bool,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "forgetting the project", move |state| {
        project_forget_impl(state, &project_id, forgotten)
    })
    .await
}

/// Point a project at the folder it moved to. The folder is picked here,
/// in the system's own dialog, never sent by the webview (SECURITY C1).
/// `false` when the user cancelled the dialog.
///
/// # Errors
///
/// As [`project_relocate_impl`].
#[tauri::command]
pub async fn project_relocate(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<bool> {
    use tauri_plugin_dialog::DialogExt;
    crate::ipc::blocking(state.inner(), "relocating the project", move |state| {
        let Some(picked) = app
            .dialog()
            .file()
            .set_title("Where the project is now")
            .blocking_pick_folder()
        else {
            return Ok(false);
        };
        let folder = picked.into_path().map_err(|error| AppError::Io {
            context: "relocate",
            message: error.to_string(),
        })?;
        project_relocate_impl(state, &project_id, &folder)?;
        Ok(true)
    })
    .await
}

/// Open a project in the chosen editor (`editor`) or show it in Finder
/// (`finder`).
///
/// # Errors
///
/// As [`project_open_target_impl`], or when the app can't be opened.
#[tauri::command]
pub async fn project_open(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    how: String,
) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let how = if how == "finder" {
        OpenIn::Finder
    } else {
        OpenIn::Editor
    };
    let target = crate::ipc::blocking(state.inner(), "opening the project", move |state| {
        project_open_target_impl(state, &project_id, how)
    })
    .await?;
    let result = if target.reveal {
        app.opener().reveal_item_in_dir(&target.folder)
    } else {
        app.opener()
            .open_path(target.folder.to_string_lossy(), target.app.as_deref())
    };
    result.map_err(|error| AppError::Io {
        context: if target.reveal {
            "reveal in Finder"
        } else {
            "open in the editor"
        },
        message: error.to_string(),
    })
}

/// The editors this Mac has, and the one chosen.
///
/// # Errors
///
/// As [`editor_choices_impl`].
#[tauri::command]
pub async fn editor_choices(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<EditorChoicesInfo> {
    crate::ipc::blocking(state.inner(), "finding the editors", |state| {
        editor_choices_impl(state, &application_dirs())
    })
    .await
}

/// Choose the editor projects open in (`null` = Finder).
///
/// # Errors
///
/// As [`editor_set_impl`].
#[tauri::command]
pub async fn editor_set(
    state: tauri::State<'_, Arc<AppState>>,
    editor: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "choosing the editor", move |state| {
        editor_set_impl(state, editor.as_deref())
    })
    .await
}
