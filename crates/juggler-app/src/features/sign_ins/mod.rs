//! AJ-E1553 and AJ-E1545 R8: the sign-ins AI Juggler runs for its Apps,
//! one place for the setup wizard, the rail and the sign-in page.
//!
//! - A sign-in starts in its terminal (`terminal::spawn_login_terminal_impl`)
//!   and is watched ([`watch`]): its output is read by its adapter for the
//!   page and device code it shows (the friendly card), and its folder by
//!   the adapter's "signed in?" signal (file events plus the adapter's own
//!   check, metadata only; `juggler_adapter::sign_in_page::SignInWatch`).
//! - Signed in: the sign-in's process is ended, its App reads ready (the
//!   window reads the lists again), and its entry leaves the rail.
//! - Ended without signing in: an outcome ("Sign-in ended before it
//!   finished") with its last lines kept (made safe: an address without its
//!   query, no token-like word) and Try again; never an empty terminal.
//!   Those are kept over a restart (`sign-ins.json`), and cleared on the
//!   next start when their App reads signed in by then, or is gone.
//! - Everything per tool is the adapter's (its `sign_in_page` and
//!   `sign_in_stamp` hooks); nothing here names a tool.

pub mod commands;
mod watch;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use ts_rs::TS;

use juggler_core::model::{Account, SessionId};

use crate::error::{AppError, AppResult};
use crate::events::{AppEvent, EmitEvent as _};
use crate::state::AppState;

/// The file the ended sign-ins are kept in, in the app's data folder.
const KEPT_FILE: &str = "sign-ins.json";
/// The most sign-ins listed: the newest kept.
const MOST: usize = 16;
/// The most last lines an ended sign-in keeps.
const LAST_LINES: usize = 12;

/// Where a sign-in stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum SignInState {
    /// Its tool's sign-in runs; AI Juggler waits for it.
    Running,
    /// Its App reads signed in: done.
    SignedIn,
    /// It ended before its App read signed in.
    Ended,
}

/// One sign-in, as the window shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SignInInfo {
    /// Its terminal's id (`login-…`).
    pub id: String,
    /// The App (account) it signs in.
    pub account_id: String,
    /// That App's tool (its words come from the tool's spec).
    pub tool: String,
    /// "Sign in · <App>".
    pub title: String,
    /// Where it stands.
    pub state: SignInState,
    /// The host of the page its tool showed (opened by
    /// `sign_ins_open_page`), when its adapter read one.
    pub page_host: Option<String>,
    /// The one-time code the page should show, when its tool printed one.
    pub code: Option<String>,
    /// The page gives a code to paste back into the tool.
    pub pastes_code: bool,
    /// An ended sign-in's last lines (made safe).
    pub last_lines: Vec<String>,
    /// When it started (ms since the epoch).
    #[ts(type = "number")]
    pub started_at: i64,
}

/// A sign-in and what only the backend keeps of it.
#[derive(Clone, Debug)]
struct Entry {
    info: SignInInfo,
    /// The page to open (never sent to the window: it's opened here).
    url: Option<String>,
}

/// The sign-ins of this run, and those kept from the last.
#[derive(Debug, Default)]
pub struct SignIns {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    /// The kept ones were read (and stale ones cleared).
    loaded: bool,
    entries: Vec<Entry>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .unwrap_or_default()
}

/// The host of `url` (an `https` address its adapter read).
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let host = rest.split(['/', '?', '#', ':']).next()?;
    (!host.is_empty()).then(|| host.to_owned())
}

/// AJ-E1553 R3: which of the `kept` sign-ins still stand: an ended one
/// whose App still isn't signed in (`open` says so; a removed App's or
/// tool's says no). A running one from a run before can't be: its process
/// ended with that run, so it's kept as ended.
fn still_standing(kept: Vec<SignInInfo>, open: impl Fn(&SignInInfo) -> bool) -> Vec<SignInInfo> {
    kept.into_iter()
        .filter(|info| info.state != SignInState::SignedIn && open(info))
        .map(|mut info| {
            info.state = SignInState::Ended;
            info
        })
        .collect()
}

/// The kept sign-ins in `dir` (none when there's no file, or it can't be
/// read).
fn read_kept(dir: &Path) -> Vec<SignInInfo> {
    std::fs::read(dir.join(KEPT_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Keeps the ended ones of `entries` in `dir`, for after a restart (owner
/// only; their lines were made safe).
fn write_kept(dir: &Path, entries: &[Entry]) {
    let ended: Vec<&SignInInfo> = entries
        .iter()
        .map(|entry| &entry.info)
        .filter(|info| info.state == SignInState::Ended)
        .collect();
    let path: PathBuf = dir.join(KEPT_FILE);
    if ended.is_empty() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(&ended) {
        let _ = write_private(&path, &bytes);
    }
}

/// Writes `bytes` to `path`, readable by its owner only.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)?.write_all(bytes)
}

/// Whether `info`'s App is still there and not signed in (AJ-E1553 R3).
fn still_open(state: &Arc<AppState>, info: &SignInInfo) -> bool {
    let Ok(account) = crate::commands::find_account_arc(state, &info.account_id) else {
        return false;
    };
    state.registry.get(&account.harness).is_some_and(|adapter| {
        !juggler_adapter::sign_in_page::signed_in(&*adapter, &account.state_dir)
    })
}

impl SignIns {
    /// Reads the kept sign-ins once, clearing the stale ones (R3).
    fn loaded<'a>(&'a self, state: &Arc<AppState>) -> Option<std::sync::MutexGuard<'a, Inner>> {
        {
            let inner = self.inner.lock().ok()?;
            if inner.loaded {
                return Some(inner);
            }
        }
        // Read outside the lock: the check asks each App's adapter.
        let kept = read_kept(&state.data_dir);
        let count = kept.len();
        let standing = still_standing(kept, |info| still_open(state, info));
        let mut inner = self.inner.lock().ok()?;
        if !inner.loaded {
            inner.loaded = true;
            let mut entries: Vec<Entry> = standing
                .into_iter()
                .map(|info| Entry { info, url: None })
                .collect();
            entries.append(&mut inner.entries);
            inner.entries = entries;
            if inner.entries.len() != count {
                write_kept(&state.data_dir, &inner.entries);
            }
        }
        Some(inner)
    }

    /// Every sign-in, oldest first.
    pub fn list(&self, state: &Arc<AppState>) -> Vec<SignInInfo> {
        self.loaded(state)
            .map(|inner| {
                inner
                    .entries
                    .iter()
                    .map(|entry| entry.info.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether sign-in `id` is still listed (not dismissed).
    fn contains(&self, id: &str) -> bool {
        self.inner
            .lock()
            .is_ok_and(|inner| inner.entries.iter().any(|entry| entry.info.id == id))
    }

    /// Adds a sign-in that just started; the oldest finished ones go past
    /// [`MOST`].
    fn add(&self, state: &Arc<AppState>, info: SignInInfo) {
        let Some(mut inner) = self.loaded(state) else {
            return;
        };
        inner.entries.retain(|entry| entry.info.id != info.id);
        inner.entries.push(Entry { info, url: None });
        while inner.entries.len() > MOST {
            let Some(at) = inner
                .entries
                .iter()
                .position(|entry| entry.info.state != SignInState::Running)
            else {
                break;
            };
            inner.entries.remove(at);
        }
    }

    /// Changes sign-in `id` by `change`; keeps the ended ones when one
    /// ends. `false`: it's gone (dismissed).
    fn change(&self, data_dir: &Path, id: &str, change: impl FnOnce(&mut Entry)) -> bool {
        let Ok(mut inner) = self.inner.lock() else {
            return false;
        };
        let Some(entry) = inner.entries.iter_mut().find(|entry| entry.info.id == id) else {
            return false;
        };
        let was = entry.info.state;
        change(entry);
        let ended = was != SignInState::Ended && entry.info.state == SignInState::Ended;
        if ended {
            write_kept(data_dir, &inner.entries);
        }
        true
    }

    /// Takes sign-in `id` off the list; whether it was running.
    fn remove(&self, data_dir: &Path, id: &str) -> Option<SignInState> {
        let mut inner = self.inner.lock().ok()?;
        let at = inner.entries.iter().position(|entry| entry.info.id == id)?;
        let entry = inner.entries.remove(at);
        if entry.info.state == SignInState::Ended {
            write_kept(data_dir, &inner.entries);
        }
        Some(entry.info.state)
    }

    /// Sign-in `id`'s page, when its tool showed one.
    fn page(&self, id: &str) -> Option<String> {
        let inner = self.inner.lock().ok()?;
        inner
            .entries
            .iter()
            .find(|entry| entry.info.id == id)
            .and_then(|entry| entry.url.clone())
    }
}

/// Tells the window the sign-ins changed (carries them all).
fn tell(app: &AppHandle, state: &Arc<AppState>) {
    app.emit_event(AppEvent::SignInsChanged, state.sign_ins.list(state));
}

/// A sign-in for `account` started in terminal `id`: listed, and watched
/// from `watch` (how its folder stood before it started).
pub(crate) fn started(
    app: &AppHandle,
    state: &Arc<AppState>,
    account: &Account,
    id: &SessionId,
    watch: juggler_adapter::sign_in_page::SignInWatch,
) {
    state.sign_ins.add(
        state,
        SignInInfo {
            id: id.as_str().to_owned(),
            account_id: account.id.to_string(),
            tool: account.harness.to_string(),
            title: format!("Sign in · {}", account.label),
            state: SignInState::Running,
            page_host: None,
            code: None,
            pastes_code: false,
            last_lines: Vec::new(),
            started_at: now_ms(),
        },
    );
    tell(app, state);
    watch::spawn(
        app.clone(),
        Arc::clone(state),
        id.clone(),
        account.clone(),
        watch,
    );
}

/// Takes sign-in `id` off the list (the outcome's Dismiss, a finished
/// one's Done, or before Try again), ending its process if it still runs.
///
/// # Errors
///
/// [`AppError::NotFound`] when no such sign-in is listed.
pub fn dismiss(app: &AppHandle, state: &Arc<AppState>, id: &str) -> AppResult<()> {
    let was = state
        .sign_ins
        .remove(&state.data_dir, id)
        .ok_or_else(|| AppError::NotFound {
            entity: "sign-in",
            id: id.to_owned(),
        })?;
    if was == SignInState::Running {
        let _ = state
            .supervisor
            .stop(&SessionId::new(id), std::time::Duration::from_secs(3));
    }
    tell(app, state);
    Ok(())
}

/// Opens the page sign-in `id`'s tool showed, in the browser: the address
/// its adapter read (`https`, a host of the adapter's), never one the
/// window sends. Opening a web page raises no system prompt
/// (`docs/design/os-prompts.md`, "Checked and not a prompt today").
///
/// # Errors
///
/// [`AppError::NotFound`] when it has no page; the browser's own error.
pub fn open_page(app: &AppHandle, state: &Arc<AppState>, id: &str) -> AppResult<()> {
    let url = state.sign_ins.page(id).ok_or_else(|| AppError::NotFound {
        entity: "sign-in page",
        id: id.to_owned(),
    })?;
    crate::webview_guard::open_web(app, url, crate::webview_guard::Opened::ByApp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(id: &str, account: &str, state: SignInState) -> SignInInfo {
        SignInInfo {
            id: id.into(),
            account_id: account.into(),
            tool: "made-up-tool".into(),
            title: format!("Sign in · {account}"),
            state,
            page_host: Some("auth.example.com".into()),
            code: Some("WDJB-MJHT".into()),
            pastes_code: false,
            last_lines: vec!["Signing in was cancelled".into()],
            started_at: 1,
        }
    }

    /// R3: after a restart, an App that reads signed in by then (or is
    /// gone) has no sign-in entry left; one still signed out keeps its
    /// outcome, and one that was running when AI Juggler quit is ended.
    #[test]
    fn a_restart_clears_the_sign_ins_whose_app_is_signed_in_or_gone() {
        let kept = vec![
            info("login-a", "signed-in-since", SignInState::Ended),
            info("login-b", "still-out", SignInState::Ended),
            info("login-c", "gone", SignInState::Ended),
            info("login-d", "was-running", SignInState::Running),
            info("login-e", "done", SignInState::SignedIn),
        ];
        let open = |info: &SignInInfo| {
            matches!(
                info.account_id.as_str(),
                "still-out" | "was-running" | "done"
            )
        };
        let left = still_standing(kept, open);
        let ids: Vec<&str> = left.iter().map(|info| info.id.as_str()).collect();
        assert_eq!(ids, ["login-b", "login-d"]);
        assert!(left.iter().all(|info| info.state == SignInState::Ended));
    }

    /// Only ended sign-ins are kept on disk, and the file goes when none is.
    #[test]
    fn only_ended_sign_ins_are_kept_over_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let entries = vec![
            Entry {
                info: info("login-a", "a", SignInState::Running),
                url: Some("https://auth.example.com/x".into()),
            },
            Entry {
                info: info("login-b", "b", SignInState::Ended),
                url: None,
            },
            Entry {
                info: info("login-c", "c", SignInState::SignedIn),
                url: None,
            },
        ];
        write_kept(dir.path(), &entries);
        let kept = read_kept(dir.path());
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "login-b");
        let text = std::fs::read_to_string(dir.path().join(KEPT_FILE)).unwrap();
        assert!(!text.contains("https://"), "a page's address is never kept");
        write_kept(dir.path(), &entries[..1]);
        assert!(!dir.path().join(KEPT_FILE).exists());
        assert!(read_kept(dir.path()).is_empty());
    }

    #[test]
    fn a_page_is_named_by_its_host() {
        assert_eq!(
            host_of("https://auth.example.com/device?user_code=X").as_deref(),
            Some("auth.example.com")
        );
        assert_eq!(
            host_of("https://auth.example.com:443/d").as_deref(),
            Some("auth.example.com")
        );
        assert_eq!(host_of("nonsense"), None);
    }
}
