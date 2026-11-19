//! AJ-E3210 (D-135): each App on its own sign-in's Account.
//!
//! A Claude app profile and its CLI can share one Claude Code folder that
//! holds two sign-ins (F-rc3c-3): the app's `claude` writes the app's
//! sign-in into the folder's state file, a terminal `claude` the CLI's.
//!
//! - **A desktop App's Account** is its own sign-in, from its records'
//!   folder names only ([`juggler_adapter::ToolSignIn::desktop_sign_in`],
//!   D-63 as amended): none before its first record ("Account not known
//!   yet"), never a guess.
//! - **A CLI App's Account** is its folder's CLI sign-in, and on a folder
//!   a desktop App shares, a read moves it only when a CLI wrote it
//!   ([`hold_app_writes`], Q2): with no CLI credential there it's "Not
//!   signed in"; a sign-in no desktop App there is on was a CLI's; the
//!   desktop App's own is held when the newest run there was the app's
//!   (the folder keeps its Account), and read otherwise.
//! - **A change** is stored per App, written once to the Account's history
//!   and told once ([`store_app_accounts`], [`tell`]); a read that changes
//!   nothing writes neither (Q5).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::{IdentityProbe, SignInNames, SignInWriter};
use juggler_core::storage::NewNotice;

use super::ProfileRead;
use crate::commands::lock_storage_arc;
use crate::state::AppState;

/// What the audit says moved an App (Q5).
pub const ACTOR_SIGN_IN_READ: &str = "sign-in read";
/// The audit row's action.
pub const ACTION_MOVED: &str = "sign_in_moved";
/// The notice's kind (Settings: "Apps that changed account").
pub const NOTICE_KIND: &str = "account-changed";

/// The words a CLI App's place says while the Claude app's sign-in is in
/// its folder and its own was never read.
pub const HELD_REASON: &str =
    "the Claude app ran here last, so the terminal's own sign-in shows once a terminal runs here";

/// One of the user's desktop Apps, and the Account its records name now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopApp {
    /// `app:<host>:<instance>`.
    pub id: String,
    /// The tool it runs.
    pub tool: String,
    /// The folder its sessions run on, when one is known.
    pub folder: Option<String>,
    /// Its own Account (`acct:…`), when its records name one; `None`
    /// when they don't yet, or its instance can't be read now.
    pub account: Option<String>,
    /// Its instance folder is there to read (a missing one keeps the
    /// Account it had).
    pub readable: bool,
}

/// The id a vendor's sign-in is stored under (`subscriptions::sign_in`'s
/// rule): one sign-in read by a CLI or named by an app's records is one
/// Account.
#[must_use]
pub fn account_id_of(vendor: &str, subject: &str, org: Option<&str>) -> String {
    juggler_core::storage::subscription_id(vendor, &identity_hash(vendor, subject, org))
}

/// The stored hash of a vendor's sign-in.
#[must_use]
pub fn identity_hash(vendor: &str, subject: &str, org: Option<&str>) -> String {
    juggler_adapter::handover::sha256_hex(
        format!("{vendor}|{subject}|{}", org.unwrap_or("")).as_bytes(),
    )
}

/// The user's desktop Apps with the Account each one's records name now,
/// each such Account stored (made the first time it's seen).
#[must_use]
pub fn desktop_apps(state: &Arc<AppState>) -> Vec<DesktopApp> {
    let Ok(apps) = lock_storage_arc(state).and_then(|storage| Ok(storage.apps().list()?)) else {
        return Vec::new();
    };
    let found: Vec<(DesktopApp, Option<(String, SignInNames)>)> = apps
        .into_iter()
        .filter(|app| app.kind == "desktop" && !app.hidden && app.is_users())
        .map(|app| {
            let instance = app.instance_dir.as_deref().map(PathBuf::from);
            let adapter = state
                .registry
                .get(&juggler_core::model::HarnessId::new(app.tool.clone()));
            let names = instance
                .as_deref()
                .zip(adapter.as_deref())
                .and_then(|(dir, adapter)| {
                    Some((adapter.sign_in_vendor(), adapter.desktop_sign_in(dir)?))
                });
            (
                DesktopApp {
                    readable: instance.as_deref().is_some_and(Path::is_dir),
                    id: app.id,
                    tool: app.tool,
                    folder: app.folder_id,
                    account: None,
                },
                names,
            )
        })
        .collect();
    let Ok(storage) = lock_storage_arc(state) else {
        return Vec::new();
    };
    let now = crate::commands::now_millis();
    found
        .into_iter()
        .map(|(mut app, names)| {
            app.account = names.and_then(|(vendor, names)| {
                let hash = identity_hash(&vendor, &names.subject, names.org.as_deref());
                storage.subscriptions().upsert(&vendor, &hash, now).ok()
            });
            app
        })
        .collect()
}

/// Q2: on each folder a desktop App shares, a read the CLI didn't write
/// is held: its probe becomes "couldn't tell" (so the folder keeps the
/// Account it was last read as), and the folder is returned. A folder
/// with no CLI sign-in there reads "Not signed in". `dirs`: each folder's
/// state folder.
pub fn hold_app_writes(
    state: &Arc<AppState>,
    rows: &mut [ProfileRead],
    desktop: &[DesktopApp],
    dirs: &BTreeMap<String, PathBuf>,
) -> BTreeSet<String> {
    let mut held = BTreeSet::new();
    for row in rows.iter_mut().filter(|row| row.service.is_none()) {
        let folder = row.profile.id.clone();
        let on_folder: Vec<&DesktopApp> = desktop
            .iter()
            .filter(|app| app.folder.as_deref() == Some(folder.as_str()))
            .collect();
        let (Some(dir), false) = (dirs.get(&folder), on_folder.is_empty()) else {
            continue;
        };
        let Some(adapter) = state
            .registry
            .get(&juggler_core::model::HarnessId::new(row.tool.clone()))
        else {
            continue;
        };
        match decide(
            &row.probe,
            &adapter.sign_in_vendor(),
            &on_folder,
            || adapter.login_state(dir),
            || adapter.sign_in_writer(dir),
        ) {
            Decision::Read => {}
            Decision::NotSignedIn => row.probe = IdentityProbe::SignedOut,
            Decision::Hold => {
                row.probe = IdentityProbe::Unverifiable {
                    reason: HELD_REASON.to_owned(),
                };
                held.insert(folder);
            }
        }
    }
    held
}

/// What a read of a folder a desktop App shares does to its CLI App.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Taken as read (the CLI's).
    Read,
    /// No CLI credential there: the CLI App isn't signed in.
    NotSignedIn,
    /// The app's write: the folder keeps its Account.
    Hold,
}

/// [`hold_app_writes`]' rule for one folder (pure but for its two asks,
/// made only when needed).
pub fn decide(
    probe: &IdentityProbe,
    vendor: &str,
    on_folder: &[&DesktopApp],
    login: impl FnOnce() -> juggler_adapter::environment::LoginState,
    writer: impl FnOnce() -> SignInWriter,
) -> Decision {
    use juggler_adapter::environment::LoginState;
    // Couldn't tell, or signed out: as read (a sign-out is never the
    // app's write: its `claude` writes its own sign-in).
    let IdentityProbe::SignedIn { subject, org, .. } = probe else {
        return Decision::Read;
    };
    // (b) The folder's own CLI credential, by its existence only.
    if login() == LoginState::LoggedOut {
        return Decision::NotSignedIn;
    }
    // A sign-in no desktop App here is on: a CLI wrote it.
    let read = account_id_of(vendor, subject, org.as_deref());
    if !on_folder
        .iter()
        .any(|app| app.account.as_deref() == Some(read.as_str()))
    {
        return Decision::Read;
    }
    // (a) The desktop App's own: the app's write when the newest run here
    // was the app's. With no run to say (a `/login` writes no transcript,
    // and the app's Code tab always does), it's the CLI's.
    if writer() == SignInWriter::DesktopApp {
        Decision::Hold
    } else {
        Decision::Read
    }
}

/// One App's change of Account (Q5), told once the Accounts are named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changed {
    /// The App.
    pub app: String,
    /// The Account it was on (`None`: none, or not known yet).
    pub was: Option<String>,
    /// The Account it's on now.
    pub now: Option<String>,
}

/// Q3/Q5: each App's Account stored: a CLI App's is its folder's (as this
/// read linked it: `stored`, by folder), a desktop App's its records' (one
/// whose instance can't be read now keeps its own). Each change is written to
/// the Account's history once; the changes are returned to be told.
pub fn store_app_accounts(
    state: &Arc<AppState>,
    (rows, stored): (&[ProfileRead], &BTreeMap<String, String>),
    desktop: &[DesktopApp],
) -> Vec<Changed> {
    // Each folder's CLI App: on the Account its folder is linked to now.
    let folder_accounts: BTreeMap<String, Option<String>> = rows
        .iter()
        .filter(|row| row.service.is_none())
        .map(|row| (row.profile.id.clone(), stored.get(&row.profile.id).cloned()))
        .collect();
    let Ok(storage) = lock_storage_arc(state) else {
        return Vec::new();
    };
    let wanted = folder_accounts
        .iter()
        .map(|(folder, account)| (juggler_core::storage::cli_app_id(folder), account.clone()))
        .chain(
            desktop
                .iter()
                .filter(|app| app.account.is_some() || app.readable)
                .map(|app| (app.id.clone(), app.account.clone())),
        );
    let mut changed = Vec::new();
    for (app, account) in wanted {
        match storage.apps().set_account(&app, account.as_deref()) {
            Ok(Some(was)) => {
                let detail = format!(
                    "{app}: {} → {}",
                    was.as_deref().unwrap_or("none"),
                    account.as_deref().unwrap_or("none")
                );
                let audited = account.clone().or_else(|| was.clone()).unwrap_or_default();
                if let Err(error) = storage.account_audit().record(
                    &juggler_core::model::AccountId::new(audited),
                    ACTION_MOVED,
                    Some(&detail),
                    ACTOR_SIGN_IN_READ,
                ) {
                    tracing::warn!(%error, app, "an App's change of Account wasn't recorded");
                }
                changed.push(Changed {
                    app,
                    was,
                    now: account,
                });
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, app, "an App's Account wasn't stored"),
        }
    }
    changed
}

/// Q5: each App that moved from one Account to another, told once
/// ("Claude Code CLI · aij-test is signed in to Team now"). One that
/// gained its first Account, or lost one, is only recorded.
pub fn tell(state: &Arc<AppState>, changed: &[Changed], name_of: impl Fn(&str) -> String) {
    let moves: Vec<(&Changed, &str, &str)> = changed
        .iter()
        .filter_map(|c| Some((c, c.was.as_deref()?, c.now.as_deref()?)))
        .collect();
    if moves.is_empty() {
        return;
    }
    let names = app_names(state);
    let Ok(storage) = lock_storage_arc(state) else {
        return;
    };
    let now_ms = crate::commands::now_millis();
    let mut told = false;
    for (change, was, now) in moves {
        let app = names
            .get(&change.app)
            .cloned()
            .unwrap_or_else(|| "An app".to_owned());
        let notice = NewNotice {
            key: format!("{NOTICE_KIND}:{}:{now}:{now_ms}", change.app),
            kind: NOTICE_KIND.to_owned(),
            title: moved_title(&app, &name_of(now)),
            detail: moved_detail(&name_of(was)),
            data: serde_json::json!({ "app_id": change.app, "account_id": now }).to_string(),
            ..NewNotice::default()
        };
        match crate::features::notifications::centre::record(&storage, &notice, now_ms) {
            Ok(Some(_)) => told = true,
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, "an App's change of Account wasn't told"),
        }
    }
    drop(storage);
    if told {
        crate::features::notifications::changed(state);
    }
}

/// The notice's line: "Claude Code CLI · aij-test is signed in to Team now".
#[must_use]
pub fn moved_title(app: &str, account: &str) -> String {
    format!("{app} is signed in to {account} now")
}

/// Its second line: where it was, and what follows.
#[must_use]
pub fn moved_detail(was: &str) -> String {
    format!("It was on {was}. Its room and new sessions count on the new account from now on.")
}

/// Each App's name as the rail says it, for a notice: a desktop App's
/// identity (`places`), a CLI App's tool and folder.
fn app_names(state: &Arc<AppState>) -> BTreeMap<String, String> {
    use crate::features::presence::{Naming, identity};
    let mut names: BTreeMap<String, String> = crate::features::places::commands::identities(state)
        .into_iter()
        .map(|(id, known)| (id, known.label))
        .collect();
    let Ok(storage) = lock_storage_arc(state) else {
        return names;
    };
    let labels: BTreeMap<String, Option<String>> = storage
        .apps()
        .list()
        .unwrap_or_default()
        .into_iter()
        .map(|app| (app.id, app.label))
        .collect();
    for folder in storage.accounts().list().unwrap_or_default() {
        let id = juggler_core::storage::cli_app_id(folder.id.as_str());
        let tool = state.registry.get(&folder.harness).map_or_else(
            || folder.harness.to_string(),
            |adapter| adapter.spec().display_name,
        );
        let given = labels.get(&id).cloned().flatten();
        let label = identity(Naming {
            id: &id,
            desktop: false,
            app: &tool,
            instance_dir: None,
            default_instance: false,
            folder_label: Some(&folder.label),
            folder_shared: false,
            named: given.as_deref(),
        })
        .label;
        names.insert(id, label);
    }
    names
}
