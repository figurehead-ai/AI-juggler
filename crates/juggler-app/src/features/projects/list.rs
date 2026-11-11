//! The projects list: each project with its live session, account history and
//! totals (ADR-0007).

use std::path::PathBuf;
use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::projects::ProjectInfo;
use crate::features::projects::backfill::repo_identity;
use crate::features::sessions::read_model::{
    SessionInfo, desktop_isolation_labels, live_stats_of, session_infos,
};
use crate::state::AppState;

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub fn projects_list_impl(state: &Arc<AppState>) -> AppResult<Vec<ProjectInfo>> {
    use std::collections::HashMap;
    // Collector first (never locked together with storage), then the
    // storage guard for the whole join below.
    let live_stats = live_stats_of(state);
    let storage = lock_storage_arc(state)?;
    let sessions = session_infos(
        &storage,
        &live_stats,
        &desktop_isolation_labels(&state.registry),
        &state.supervisor.alive_adopted_sessions(),
    )?;
    // Statuses that mean "occupying the project right now".
    let live = |status: &str| matches!(status, "starting" | "running");
    let choices = crate::features::projects::mark::choices(&storage)?;
    let mut created: HashMap<String, i64> = HashMap::new();
    let mut infos: Vec<ProjectInfo> = storage
        .projects()
        .list()?
        .into_iter()
        .map(|project| {
            created.insert(project.id.to_string(), project.created_at.millis());
            let id = project.id.to_string();
            let owned: Vec<&SessionInfo> = sessions
                .iter()
                .filter(|s| s.project_id.as_deref() == Some(id.as_str()))
                .collect();
            let active_instance = owned.iter().find(|s| live(&s.status)).map(|s| (*s).clone());
            let mut accounts_used: Vec<String> = owned
                .iter()
                .map(|s| s.account_label.clone())
                .filter(|label| !label.is_empty())
                .collect();
            accounts_used.extend(project.history.accounts.iter().cloned());
            accounts_used.sort();
            accounts_used.dedup();
            let forgotten = crate::manage::is_forgotten(&project).then_some(true);
            ProjectInfo {
                id,
                root: project.root.to_string_lossy().into_owned(),
                name: project.name,
                last_activity_ms: project.last_activity.map(juggler_core::Timestamp::millis),
                active_instance,
                accounts_used,
                instance_count: u64::try_from(owned.len()).unwrap_or_default(),
                history_sessions: project.history.native_sessions,
                history_last_used_ms: project.history.last_used_ms,
                repo: None,
                missing: !project.root.exists(),
                forgotten,
                pinned: project.pinned_at.map(|_| true),
                default_account_id: project.default_account.map(|id| id.to_string()),
                mark: None,
            }
        })
        .collect();
    // File reads (each repository's .git/config, each icon) never under
    // the storage lock; cached briefly so a slow volume can't stall every
    // refresh.
    drop(storage);
    let mut icons = HashMap::new();
    for info in &mut infos {
        info.repo = repo_identity_cached(std::path::Path::new(&info.root));
        if let Some(icon) =
            crate::features::projects::icon::find_icon_cached(std::path::Path::new(&info.root))
        {
            icons.insert(info.id.clone(), icon);
        }
    }
    let subjects: Vec<_> = infos
        .iter()
        .map(|info| crate::features::projects::mark::Subject {
            id: &info.id,
            name: &info.name,
            repo: info.repo.as_deref(),
            created_ms: created.get(&info.id).copied().unwrap_or_default(),
            icon: icons.get(&info.id).map(|icon| icon.version.clone()),
        })
        .collect();
    let mut marks = crate::features::projects::mark::marks(&subjects, &choices);
    // Clones share one icon: each serves the file its mark uses.
    let repos: HashMap<String, Option<String>> = infos
        .iter()
        .map(|i| (i.id.clone(), i.repo.clone()))
        .collect();
    let mut served = HashMap::new();
    for info in &mut infos {
        info.mark = marks.remove(&info.id);
        let Some(version) = info.mark.as_ref().and_then(|m| m.icon.clone()) else {
            continue;
        };
        let shown = icons.iter().find(|(id, icon)| {
            icon.version == version
                && (**id == info.id || (info.repo.is_some() && repos.get(*id) == Some(&info.repo)))
        });
        if let Some((_, icon)) = shown {
            served.insert(info.id.clone(), icon.path.clone());
        }
    }
    crate::features::projects::icon::remember_served(state, served);
    Ok(infos)
}

/// [`repo_identity`], remembered for five minutes per folder.
pub(crate) fn repo_identity_cached(root: &std::path::Path) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    type Seen = HashMap<PathBuf, (Option<String>, Instant)>;
    static CACHE: OnceLock<Mutex<Seen>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(map) = cache.lock()
        && let Some((repo, at)) = map.get(root)
        && at.elapsed() < Duration::from_secs(300)
    {
        return repo.clone();
    }
    let repo = repo_identity(root);
    if let Ok(mut map) = cache.lock() {
        map.insert(root.to_path_buf(), (repo.clone(), Instant::now()));
    }
    repo
}
