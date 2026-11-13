//! One pass over the tools' files (split from `mod.rs`, its size budget):
//! listing each account's conversations and reading the ones that
//! changed, newest first; and (AJ-E805) which files the app's background
//! pass reads, from [`super::gate`].

use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use super::{Reporter, gate, list_build, store, whole_read_due};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// AJ-E805: what the app's background pass reads ([`gate`]); every file
/// for any other caller, and until the list was read whole once.
pub(super) fn plan(state: &Arc<AppState>, accounts: &[juggler_core::model::Account]) -> gate::Plan {
    use std::sync::atomic::Ordering::Relaxed;
    let conversations = &state.conversations;
    if !conversations.background.load(Relaxed) || !conversations.first_done.load(Relaxed) {
        return gate::Plan::Whole;
    }
    let watched: Vec<gate::Watched> = accounts
        .iter()
        .map(|account| {
            let roots = state
                .registry
                .get(&account.harness)
                .map(|adapter| adapter.conversation_roots(&account.state_dir))
                .unwrap_or_default();
            (account.id.to_string(), roots)
        })
        .collect();
    let known = |path: &std::path::Path| {
        conversations
            .files
            .lock()
            .is_ok_and(|files| files.contains_key(path))
    };
    let Ok(mut slot) = conversations.gate.lock() else {
        return gate::Plan::Whole;
    };
    slot.get_or_insert_with(gate::Gate::new)
        .plan(&watched, &known, std::time::Instant::now())
}

/// One pass: every account's files (`scope` `None`), or only the
/// accounts and known files `scope` names (AJ-E805, [`gate`]).
pub(super) fn refresh_files(
    state: &Arc<AppState>,
    accounts: &[juggler_core::model::Account],
    every: std::time::Duration,
    progress: &dyn Fn(),
    scope: Option<&gate::Scope>,
) -> AppResult<bool> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut changed_any = false;
    // Every account's files, listed first (cheap), then read newest
    // first: a running session's file is among the newest.
    let mut found = Vec::new();
    let mut report = Reporter::new(every, progress);
    let listed: Vec<&juggler_core::model::Account> = accounts
        .iter()
        .filter(|account| scope.is_none_or(|scope| scope.accounts.contains(account.id.as_str())))
        .collect();
    // A known file that changed: looked at alone (gone: forgotten).
    let gone = known_changed(state, accounts, scope, &mut found);
    for account in listed.iter().copied() {
        let Some(adapter) = state.registry.get(&account.harness) else {
            continue;
        };
        // AJ-E1602: a tool that keeps every conversation in one store
        // gives them in one read; each is read again only once it changed.
        if read_stored(state, account, &adapter, &mut seen)? {
            changed_any = true;
            report.read();
        }
        for path in adapter.conversation_files(&account.state_dir) {
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            found.push((
                account,
                std::sync::Arc::clone(&adapter),
                path,
                meta.len(),
                meta.modified().ok(),
            ));
        }
    }
    found.sort_by_key(|entry| std::cmp::Reverse(entry.4));
    // So far `seen` holds the stored ones, read above.
    if scope.is_none() {
        list_build::started(state, seen.len() + found.len(), seen.len());
    }
    // Two rounds, newest first (QA batch 10: one Mac's Codex rollouts were
    // 6.6 GB, and reading each whole kept everything from showing for over
    // a minute). First every new or changed file's quick read (its start
    // and end) where its tool has one, so the list and what's live come
    // within seconds; then the whole of each file read quickly, for its
    // tokens. A file read whole in the first round is complete.
    let mut later = Vec::new();
    for (account, adapter, path, len, modified) in &found {
        report.maybe();
        state.conversations.build.step();
        seen.push(path.clone());
        let entry = state
            .conversations
            .files
            .lock()
            .map_err(|_| AppError::StatePoisoned)?
            .get(path)
            .map(|entry| {
                (
                    entry.len == *len && entry.modified == *modified,
                    entry.complete,
                )
            });
        match entry {
            Some((true, true)) => continue,
            Some((true, false)) => {
                later.push((account, adapter, path, len, modified));
                continue;
            }
            _ => {}
        }
        // Read outside the lock: a big transcript never holds up a reader.
        let (summary, complete) = match guarded(|| adapter.read_conversation_quick(path)) {
            Some(summary) => {
                later.push((account, adapter, path, len, modified));
                (Some(summary), false)
            }
            None => (guarded(|| adapter.read_conversation(path)), true),
        };
        store(state, account, path, *len, *modified, summary, complete)?;
        changed_any = true;
        report.read();
    }
    // The list is whole now (the long round only counts tokens).
    if scope.is_none() {
        list_build::listed(state);
    }
    // What's listed and live, at once, before the long round.
    report.flush();
    let now = std::time::Instant::now();
    for (account, adapter, path, len, modified) in later {
        if !whole_read_due(*len, counted_at(state, path)?, now) {
            continue;
        }
        report.maybe();
        let summary = guarded(|| adapter.read_conversation(path));
        store(state, account, path, *len, *modified, summary, true)?;
        changed_any = true;
        report.read();
    }
    let dirs: Vec<&std::path::Path> = listed.iter().map(|a| a.state_dir.as_path()).collect();
    let forgot = forget_unseen(
        state,
        seen,
        scope.map(|_| (dirs.as_slice(), gone.as_slice())),
    )?;
    Ok(forgot || changed_any)
}

/// An account's conversations kept in one store (AJ-E1602): each one
/// that changed read into the index, every key onto `seen`; whether any
/// was.
fn read_stored(
    state: &Arc<AppState>,
    account: &juggler_core::model::Account,
    adapter: &juggler_adapter::SharedAdapter,
    seen: &mut Vec<PathBuf>,
) -> AppResult<bool> {
    let mut changed_any = false;
    for stored in adapter.stored_conversations(&account.state_dir) {
        seen.push(stored.key.clone());
        let changed = u64::try_from(stored.changed_ms)
            .ok()
            .map(|ms| std::time::UNIX_EPOCH + std::time::Duration::from_millis(ms));
        let unchanged = state
            .conversations
            .files
            .lock()
            .map_err(|_| AppError::StatePoisoned)?
            .get(&stored.key)
            .is_some_and(|entry| entry.modified == changed);
        if !unchanged {
            store(
                state,
                account,
                &stored.key,
                0,
                changed,
                Some(stored.summary),
                true,
            )?;
            changed_any = true;
        }
    }
    Ok(changed_any)
}

/// A file the pass lists: its account, its tool, its size and time.
type Found<'a> = (
    &'a juggler_core::model::Account,
    juggler_adapter::SharedAdapter,
    PathBuf,
    u64,
    Option<SystemTime>,
);

/// The known files `scope` names that changed, onto `found` with their
/// size and time; the ones gone since, returned.
fn known_changed<'a>(
    state: &Arc<AppState>,
    accounts: &'a [juggler_core::model::Account],
    scope: Option<&gate::Scope>,
    found: &mut Vec<Found<'a>>,
) -> Vec<PathBuf> {
    let mut gone = Vec::new();
    for (path, id) in scope.map(|scope| &scope.files).into_iter().flatten() {
        let account = accounts.iter().find(|account| account.id.as_str() == id);
        let adapter = account.and_then(|account| state.registry.get(&account.harness));
        match (account, adapter, std::fs::metadata(path)) {
            (Some(account), Some(adapter), Ok(meta)) => {
                found.push((
                    account,
                    adapter,
                    path.clone(),
                    meta.len(),
                    meta.modified().ok(),
                ));
            }
            _ => gone.push(path.clone()),
        }
    }
    gone
}

/// One transcript's read; one whose reader panics reads as unreadable
/// (the panic is still kept as a crash report), so one odd file never
/// stops the pass, and with it the first build, on every tick.
fn guarded<T>(read: impl FnOnce() -> Option<T>) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(read))
        .ok()
        .flatten()
}

/// When `path` was last read whole, if it was.
fn counted_at(
    state: &Arc<AppState>,
    path: &std::path::Path,
) -> AppResult<Option<std::time::Instant>> {
    Ok(state
        .conversations
        .files
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .get(path)
        .and_then(|entry| entry.counted_at))
}

/// Forgets every file a pass didn't see (gone from its folder); whether
/// any was. A pass over a scope (`only`: the folders it listed, and the
/// known files it found gone) forgets only there.
fn forget_unseen(
    state: &Arc<AppState>,
    seen: Vec<PathBuf>,
    only: Option<(&[&std::path::Path], &[PathBuf])>,
) -> AppResult<bool> {
    let mut files = state
        .conversations
        .files
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let before = files.len();
    let seen: std::collections::HashSet<PathBuf> = seen.into_iter().collect();
    files.retain(|path, _| match only {
        None => seen.contains(path),
        Some((listed, gone)) => {
            !gone.contains(path)
                && (seen.contains(path) || !listed.iter().any(|dir| path.starts_with(dir)))
        }
    });
    let forgot = files.len() != before;
    drop(files);
    if forgot {
        state.conversations.placed_changed();
        crate::features::relations::changed(
            state,
            crate::features::relations::Change::ConversationsIndex,
        );
    }
    Ok(forgot)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "test code")]

    use super::*;
    use std::path::Path;

    fn transcript(config: &Path, cwd: &Path, id: &str, ask: &str) -> PathBuf {
        let dir = config
            .join("projects")
            .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{id}.jsonl"));
        std::fs::write(
            &path,
            format!(
                "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"{ask}\"}}}}\n",
                cwd.display()
            ),
        )
        .unwrap();
        path
    }

    fn titles(state: &Arc<AppState>) -> Vec<String> {
        let mut titles: Vec<String> = state
            .conversations
            .all()
            .unwrap()
            .into_iter()
            .filter_map(|c| c.summary.title)
            .collect();
        titles.sort();
        titles
    }

    /// AJ-E805: a pass over a scope reads the known files it names (a
    /// gone one forgotten) and lists only the accounts it names; every
    /// other file is left as last read. The background pass looked at
    /// every file of every account every 15 s.
    #[test]
    fn a_scoped_pass_reads_only_what_it_names() {
        let work = tempfile::tempdir().unwrap();
        let root = work.path().canonicalize().unwrap();
        let repo = root.join("Dev/app");
        std::fs::create_dir_all(&repo).unwrap();
        let config = root.join("setup");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("settings.json"), "{}").unwrap();
        let one = transcript(&config, &repo, "c-1", "First ask");
        let two = transcript(&config, &repo, "c-2", "Second ask");
        let mut registry = juggler_adapter::AdapterRegistry::new();
        registry.register(Arc::new(juggler_adapters::ClaudeCodeAdapter::with_bin(
            "/bin/true",
        )));
        let state = Arc::new(AppState::init_with_registry(root.join("data"), registry).unwrap());
        let account = crate::test_support::claude_in_place(&state, &config, "Work");
        let accounts = state.storage.lock().unwrap().accounts().list().unwrap();
        let every = std::time::Duration::ZERO;
        refresh_files(&state, &accounts, every, &|| {}, None).unwrap();
        assert_eq!(titles(&state), ["First ask", "Second ask"]);
        // Both change on disk; the scope names only the first.
        transcript(&config, &repo, "c-1", "First ask, again");
        transcript(&config, &repo, "c-2", "Second ask, again");
        let id = account.id.to_string();
        let scope = gate::Scope {
            accounts: std::collections::BTreeSet::new(),
            files: std::collections::BTreeMap::from([(one.clone(), id.clone())]),
        };
        assert!(refresh_files(&state, &accounts, every, &|| {}, Some(&scope)).unwrap());
        assert_eq!(titles(&state), ["First ask, again", "Second ask"]);
        // A named file gone is forgotten; the other is kept as read.
        std::fs::remove_file(&one).unwrap();
        assert!(refresh_files(&state, &accounts, every, &|| {}, Some(&scope)).unwrap());
        assert_eq!(titles(&state), ["Second ask"]);
        // A new file: its account listed again reads it, and the changed one.
        let three = transcript(&config, &repo, "c-3", "Third ask");
        let listed = gate::Scope {
            accounts: std::collections::BTreeSet::from([id]),
            files: std::collections::BTreeMap::new(),
        };
        assert!(refresh_files(&state, &accounts, every, &|| {}, Some(&listed)).unwrap());
        assert_eq!(titles(&state), ["Second ask, again", "Third ask"]);
        assert!(three.exists() && two.exists());
    }
}
