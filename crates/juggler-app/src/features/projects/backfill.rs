//! AJ-E1529: backfilling the projects rail from every folder any account's
//! conversations ran in.

use std::path::PathBuf;
use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// Core implementation shared with tests.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub fn projects_backfill_impl(state: &Arc<AppState>) -> AppResult<u32> {
    projects_backfill_with(state, juggler_platform::user::home().as_deref())
}

/// [`projects_backfill_impl`] with the other managers' worktrees read
/// under `home` (AJ-E1519: Claude Squad's, Vibe Kanban's), remembered
/// first so a worktree they remove later still folds into its project.
/// `None` reads none.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub fn projects_backfill_with(
    state: &Arc<AppState>,
    home: Option<&std::path::Path>,
) -> AppResult<u32> {
    use std::collections::{BTreeMap, BTreeSet};

    /// What the conversations say about one project.
    #[derive(Default)]
    struct Found {
        accounts: Vec<String>,
        conversations: BTreeSet<(String, String)>,
        last_ms: Option<i64>,
    }

    // Read outside the storage lock: their files, not ours.
    let managed: Vec<juggler_core::storage::WorktreeHome> = home
        .map(juggler_adapters::other_managers::worktrees)
        .unwrap_or_default()
        .into_iter()
        .map(|worktree| juggler_core::storage::WorktreeHome {
            folder: worktree.worktree,
            repo: worktree.repo,
            manager: worktree.manager.to_owned(),
        })
        .collect();

    crate::features::conversations::read_if_cold(state)?;
    let labels: std::collections::HashMap<String, String> = lock_storage_arc(state)?
        .accounts()
        .list()?
        .into_iter()
        .map(|account| (account.id.to_string(), account.label))
        .collect();
    let homes = {
        let storage = lock_storage_arc(state)?;
        let known = storage.projects().worktree_homes()?;
        // Written only when new or moved: this runs on every change.
        let changed: Vec<_> = managed
            .into_iter()
            .filter(|home| !known.contains(home))
            .collect();
        storage.projects().remember_worktree_homes(&changed)?;
        if changed.is_empty() {
            known
        } else {
            storage.projects().worktree_homes()?
        }
    };
    let mut found: BTreeMap<PathBuf, Found> = BTreeMap::new();
    for conversation in state.conversations.all()? {
        let Some(folder) = conversation.folder.as_deref() else {
            continue; // no folder recorded: nothing to attach it to
        };
        let root = juggler_core::storage::project_root_with(folder, &homes);
        let entry = found.entry(root).or_default();
        if let Some(label) = labels.get(&conversation.account_id)
            && !entry.accounts.contains(label)
        {
            entry.accounts.push(label.clone());
        }
        entry
            .conversations
            .insert((conversation.tool.clone(), conversation.summary.id.clone()));
        entry.last_ms = entry.last_ms.max(conversation.summary.last_ms);
    }

    let storage = lock_storage_arc(state)?;
    // Rows an earlier backfill made for a worktree or a repository's
    // subfolder fold into their project: removed when nothing refers to them.
    for project in storage.projects().list()? {
        if juggler_core::storage::project_root_with(&project.root, &homes) != project.root {
            let _ = storage.projects().delete_if_unused(&project.id);
        }
    }
    let mut enriched = 0;
    for (root, found) in found {
        let record = storage.projects().ensure(&root)?;
        storage.projects().merge_history(
            &record.id,
            &juggler_core::model::ProjectHistory {
                accounts: found.accounts,
                native_sessions: u64::try_from(found.conversations.len()).unwrap_or(u64::MAX),
                last_used_ms: found.last_ms,
            },
        )?;
        enriched += 1;
    }
    // Times an earlier backfill set to when it ran come down to the truth.
    storage.projects().settle_activity()?;
    Ok(enriched)
}

/// The `origin` remote of the repository at `root`, normalised to
/// `host/owner/name` (scheme, user, port and `.git` dropped; lowercased),
/// so clones of one repository group together. Reads only `.git/config`
/// (following a worktree's `.git` file to its repository).
pub(crate) fn repo_identity(root: &std::path::Path) -> Option<String> {
    let repo =
        juggler_core::storage::worktree_repo_root(root).unwrap_or_else(|| root.to_path_buf());
    let config = std::fs::read_to_string(repo.join(".git").join("config")).ok()?;
    let mut in_origin = false;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_origin = line == "[remote \"origin\"]";
            continue;
        }
        if in_origin
            && let Some(url) = line
                .strip_prefix("url")
                .map(str::trim)
                .and_then(|rest| rest.strip_prefix('='))
        {
            return normalise_remote(url.trim());
        }
    }
    None
}

/// `git@github.com:Owner/Name.git`, `https://user@github.com/Owner/Name`
/// and `ssh://git@github.com:22/Owner/Name.git` all become
/// `github.com/owner/name`.
#[must_use]
pub fn normalise_remote(url: &str) -> Option<String> {
    if url.contains('\\') {
        return None; // a Windows path: a local remote, nothing to group by
    }
    let (host, path) = if let Some((_, rest)) = url.split_once("://") {
        // scheme://[user[:secret]@]host[:port]/path — the port is the
        // authority's, never part of the path.
        let (authority, path) = rest.split_once('/')?;
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let host = host.split_once(':').map_or(host, |(host, _)| host);
        (host, path)
    } else {
        // scp-like [user@]host:path (no port in this form).
        let (left, path) = url.split_once(':')?;
        let host = left.rsplit_once('@').map_or(left, |(_, host)| host);
        if host.len() < 2 || host.contains('/') {
            return None; // `C:` or a local path
        }
        (host, path)
    };
    let host = host.to_lowercase();
    let path = path
        .trim_matches('/')
        .trim_end_matches(".git")
        .to_lowercase();
    (!host.is_empty() && !path.is_empty()).then(|| format!("{host}/{path}"))
}
