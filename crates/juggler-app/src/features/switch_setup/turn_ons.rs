//! What a switch's target account turns on itself (a claude.ai skill, an
//! `OpenAI` plugin), kept as a reminder after the switch screen closes, with
//! the steps, until the target's next inventory read finds it there (it
//! syncs on that account's next session). Settings shows them beside the
//! set-ups, with Check again; nothing here writes to an account.
//!
//! Pages open in the browser only when the backend named them itself (an
//! offer's or a reminder's `open_url`, or the switching guide): the
//! interface can't open an address of its own.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use super::{GetItInfo, Sides, check_sides, for_a_switch, kebab, key};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The reminders, in AI Juggler's data folder.
const FILE: &str = "switch-turn-ons.json";

/// The user guide's section on what the other account doesn't have.
pub const GUIDE_URL: &str = "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/guide/switching.md#things-the-other-account-doesnt-have";

/// Held while the file is read and written again.
static LOCK: Mutex<()> = Mutex::new(());

/// The pages the backend has named this run, which alone may be opened.
static OPENABLE: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// One thing a target account still has to turn on.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
struct Reminder {
    id: String,
    /// The target account's id.
    account: String,
    /// The offer it came from.
    offer: String,
    /// Its kind, on the wire (`skill`, `plugin`).
    kind: String,
    name: String,
    get_it: GetItInfo,
    at_ms: i64,
}

/// One reminder, for Settings.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TurnOnInfo {
    /// Pass back to dismiss it.
    pub id: String,
    /// The account that still has to turn it on, by name.
    pub account: String,
    /// `skill`, `plugin`.
    pub kind: String,
    /// Its name.
    pub name: String,
    /// How.
    pub get_it: GetItInfo,
    /// When the switch was, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// Lets `url` be opened later: the backend named it.
pub(crate) fn may_open(url: &str) {
    if url.starts_with("https://") {
        OPENABLE
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(url.to_owned());
    }
}

/// Whether `url` is a page the backend named.
fn openable(url: &str) -> bool {
    url == GUIDE_URL
        || OPENABLE
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(url)
}

fn load(state: &Arc<AppState>) -> Vec<Reminder> {
    std::fs::read(state.data_dir.join(FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write(state: &Arc<AppState>, reminders: &[Reminder]) -> AppResult<()> {
    let path = state.data_dir.join(FILE);
    let staged = path.with_extension(format!("ai-juggler-{}", std::process::id()));
    let text = serde_json::to_string_pretty(reminders).unwrap_or_default();
    std::fs::write(&staged, text).map_err(|e| AppError::io("keep the reminder", &e))?;
    std::fs::rename(&staged, &path).map_err(|e| AppError::io("keep the reminder", &e))
}

/// After a switch: keeps a reminder for each thing the target turns on
/// itself that the conversation used (or nothing says), unless the user
/// picked one that may be copied, to copy it instead. One per account
/// and offer.
///
/// # Errors
///
/// As [`check_sides`]; a failed write.
pub(crate) fn remember(state: &Arc<AppState>, sides: &Sides, picked: &[String]) -> AppResult<()> {
    let check = check_sides(state, sides, &for_a_switch)?;
    let _held = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let mut reminders = load(state);
    let account = sides.target.id.to_string();
    let before = reminders.len();
    for offer in check.offers {
        let Some(get_it) = offer.get_it else {
            continue;
        };
        if offer.verdict != "turn-on"
            || offer.used == Some(false)
            || (offer.can_copy && picked.contains(&offer.id))
            || reminders
                .iter()
                .any(|reminder| reminder.account == account && reminder.offer == offer.id)
        {
            continue;
        }
        reminders.push(Reminder {
            id: format!("turn-on-{}", crate::ids::unique_suffix()),
            account: account.clone(),
            offer: offer.id,
            kind: offer.kind,
            name: offer.name,
            get_it,
            at_ms: juggler_core::model::Timestamp::now().millis(),
        });
    }
    if reminders.len() == before {
        return Ok(());
    }
    write(state, &reminders)
}

/// What accounts still have to turn on, newest first: each account's
/// inventory is read again, and what it has now (or an account that's
/// gone) leaves the list. This is Check again.
///
/// # Errors
///
/// Storage errors; a failed write.
pub fn list_impl(state: &Arc<AppState>) -> AppResult<Vec<TurnOnInfo>> {
    let _held = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let reminders = load(state);
    if reminders.is_empty() {
        return Ok(Vec::new());
    }
    let accounts: std::collections::HashMap<String, juggler_core::model::Account> = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .accounts()
            .list()?
            .into_iter()
            .map(|account| (account.id.to_string(), account))
            .collect()
    };
    let mut has: std::collections::HashMap<String, BTreeSet<(String, String)>> =
        std::collections::HashMap::new();
    let mut kept = Vec::new();
    for reminder in &reminders {
        let Some(account) = accounts.get(&reminder.account) else {
            continue;
        };
        let there = has.entry(reminder.account.clone()).or_insert_with(|| {
            state
                .registry
                .get(&account.harness)
                .map(|adapter| {
                    adapter
                        .inventory(&account.state_dir, None)
                        .iter()
                        .map(|item| {
                            let (kind, name) = key(item);
                            (kebab(&kind), name)
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
        if !there.contains(&(reminder.kind.clone(), reminder.name.clone())) {
            kept.push(reminder.clone());
        }
    }
    if kept.len() != reminders.len() {
        write(state, &kept)?;
    }
    let mut out: Vec<TurnOnInfo> = kept
        .into_iter()
        .filter_map(|reminder| {
            let account = accounts.get(&reminder.account)?;
            if let Some(url) = &reminder.get_it.open_url {
                may_open(url);
            }
            Some(TurnOnInfo {
                id: reminder.id,
                account: account.label.clone(),
                kind: reminder.kind,
                name: reminder.name,
                get_it: reminder.get_it,
                at_ms: reminder.at_ms,
            })
        })
        .collect();
    out.sort_by_key(|info| std::cmp::Reverse(info.at_ms));
    Ok(out)
}

/// Stops reminding about one thing (the user turned it on, or won't).
///
/// # Errors
///
/// [`AppError::Refused`] when there's no such reminder; a failed write.
pub fn dismiss_impl(state: &Arc<AppState>, id: &str) -> AppResult<String> {
    let _held = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let mut reminders = load(state);
    let at = reminders
        .iter()
        .position(|reminder| reminder.id == id)
        .ok_or_else(|| AppError::Refused("there's no such reminder".to_owned()))?;
    let reminder = reminders.remove(at);
    write(state, &reminders)?;
    Ok(format!("Stopped reminding you about {}", reminder.name))
}

/// What accounts still have to turn on (Check again re-reads them).
///
/// # Errors
///
/// As [`list_impl`].
#[tauri::command]
pub async fn switch_setup_turn_ons(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<TurnOnInfo>> {
    crate::ipc::blocking(state.inner(), "check what to turn on", list_impl).await
}

/// Stops one reminder.
///
/// # Errors
///
/// As [`dismiss_impl`].
#[tauri::command]
pub async fn switch_setup_turn_on_dismiss(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "dismiss the reminder", move |state| {
        dismiss_impl(state, &id)
    })
    .await
}

/// Opens a page the backend named (an offer's or a reminder's page, or
/// the switching guide) in the browser.
///
/// # Errors
///
/// [`AppError::Validation`] for any other address; the browser's error.
#[tauri::command]
pub async fn switch_setup_open(app: tauri::AppHandle, url: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt as _;
    if !openable(&url) {
        return Err(AppError::Validation {
            subject: "page",
            message: "That page isn't one AI Juggler offered.".to_owned(),
        });
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Io {
            context: "open the page",
            message: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pages_the_backend_named_open() {
        assert!(openable(GUIDE_URL));
        assert!(!openable("https://example.com/elsewhere"));
        may_open("https://example.com/named");
        assert!(openable("https://example.com/named"));
        may_open("http://example.com/plain");
        assert!(!openable("http://example.com/plain"), "https only");
    }
}
