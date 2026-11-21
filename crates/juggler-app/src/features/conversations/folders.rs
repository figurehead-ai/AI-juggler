//! The folders the live view reads each session's pid file from (AJ-E2101):
//! every account's, the folders accounts were copied from, and each tool's
//! own default, listed from the accounts (moved out of `mod.rs` whole for
//! its size budget, with AJ-E2312 item 4's stamp).

use std::sync::Arc;

use super::scan;
use crate::state::AppState;

/// A list of the folders as kept: when it was made, the folders' stamp
/// then, and the folders.
pub(super) type Listed = (std::time::Instant, Option<i64>, Vec<scan::Profile>);

/// How often the profile folders are listed again from the accounts.
const PROFILES_EVERY: std::time::Duration = std::time::Duration::from_secs(15);

/// The folders' stamp as committed now: the database's own count of
/// folders added, removed or moved (read on its own connection, never
/// behind a writer); `None` when it can't be read.
fn folders_stamp(state: &AppState) -> Option<i64> {
    crate::features::relations::stored_now(state)
        .ok()
        .map(|stamps| stamps.folders)
}

/// The folders whose per-session files are read: every account's, the
/// folders accounts were copied from, and each tool's own default. Listed
/// again every [`PROFILES_EVERY`], and at once when the folders changed:
/// AJ-E2312 item 4 (SK-21), a setup just brought in had its running
/// session read as ended until the list's next turn (its pid file's
/// folder wasn't read), about 30 s on the PF1 walk.
pub(super) fn profiles(state: &Arc<AppState>) -> Vec<scan::Profile> {
    let now = std::time::Instant::now();
    let stamp = folders_stamp(state);
    if let Ok(cached) = state.conversations.profiles.lock()
        && let Some((at, seen, profiles)) = cached.as_ref()
        && now.saturating_duration_since(*at) < PROFILES_EVERY
        && *seen == stamp
    {
        return profiles.clone();
    }
    // A refresh never waits on storage: a busy lock keeps the last list.
    // The first list waits (0.3.5): with none yet, a launch's first scan
    // found no profile folder, so its places went out unnamed and the
    // rail said the window's folder ("b-work") until the next scan.
    let last = state
        .conversations
        .profiles
        .lock()
        .ok()
        .and_then(|cached| cached.as_ref().map(|(_, _, profiles)| profiles.clone()));
    let accounts = match (state.storage.try_lock(), last) {
        (Ok(storage), _) => storage.accounts().list().unwrap_or_default(),
        (Err(_), Some(last)) => return last,
        (Err(_), None) => state
            .storage
            .lock()
            .map(|storage| storage.accounts().list().unwrap_or_default())
            .unwrap_or_default(),
    };
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out: Vec<scan::Profile> = Vec::new();
    let mut add = |tool: &str, dir: &std::path::Path, account: Option<String>| {
        let dir = canonical(dir);
        if !out
            .iter()
            .any(|known| known.tool == tool && known.dir == dir)
        {
            out.push(scan::Profile {
                tool: tool.to_owned(),
                dir,
                account_id: account,
            });
        }
    };
    for account in &accounts {
        add(
            account.harness.as_str(),
            &account.state_dir,
            Some(account.id.to_string()),
        );
    }
    for account in &accounts {
        if let Some(source) = account.import_source_path.as_deref() {
            add(account.harness.as_str(), std::path::Path::new(source), None);
        }
    }
    for id in state.registry.ids() {
        if let Some(dir) = state.registry.get(&id).and_then(|a| a.vendor_default_dir()) {
            add(id.as_str(), &dir, None);
        }
    }
    if let Ok(mut cached) = state.conversations.profiles.lock() {
        *cached = Some((now, stamp, out.clone()));
    }
    out
}
