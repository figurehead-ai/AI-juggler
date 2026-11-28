//! Projects as a feature module (ADR-0019 §3). Their wire type lives
//! here; the commands move from `commands.rs` with AJ-E2017.

pub mod backfill;
pub mod commands;
pub mod git;
pub mod icon;
pub mod list;
pub mod mark;

/// Each tracked project's `value`, by its root folder, under each of its
/// spellings: as recorded, and as the file system resolves it (0.5.2 G24:
/// a tool that records the resolved folder, as Codex records `/private/tmp/…`
/// for a project kept as `/tmp/…`, found no project, so its posts shared
/// one scope).
///
/// # Errors
///
/// The projects could not be read.
pub(crate) fn by_root<T: Clone>(
    storage: &juggler_core::storage::Storage,
    value: impl Fn(juggler_core::model::ProjectRecord) -> T,
) -> crate::error::AppResult<std::collections::HashMap<std::path::PathBuf, T>> {
    let mut out = std::collections::HashMap::new();
    for project in storage.projects().list()? {
        let root = project.root.clone();
        let resolved = std::fs::canonicalize(&root)
            .ok()
            .filter(|real| *real != root);
        let value = value(project);
        if let Some(real) = resolved {
            out.entry(real).or_insert_with(|| value.clone());
        }
        out.insert(root, value);
    }
    Ok(out)
}

#[cfg(test)]
mod by_root_tests {
    /// 0.5.2 G24: a project kept under a folder reached through a link is
    /// found under the folder the link resolves to as well. A folder link
    /// is made by unix's `symlink` (Windows needs a privilege for one: the
    /// build-windows job, PR #257).
    #[cfg(unix)]
    #[test]
    fn a_project_is_found_by_its_resolved_folder_too() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir_all(real.join("web")).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let state = crate::test_support::bare_state(dir.path().join("data"));
        let storage = state.storage.lock().unwrap();
        let kept = link.join("web");
        storage.projects().ensure(&kept).unwrap();
        let roots = super::by_root(&storage, |project| project.name).unwrap();
        assert_eq!(roots.get(&kept).map(String::as_str), Some("web"));
        let resolved = std::fs::canonicalize(real.join("web")).unwrap();
        assert_eq!(roots.get(&resolved).map(String::as_str), Some("web"));
    }
}

/// A tracked project for the UI — the juggling anchor (ADR-0007).
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProjectInfo {
    /// Project id.
    pub id: String,
    /// Canonical root directory.
    pub root: String,
    /// Display name.
    pub name: String,
    /// Last activity, epoch milliseconds (when known).
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// The instance currently managing this project, when live.
    pub active_instance: Option<crate::features::sessions::dto::InstanceInfo>,
    /// Account labels that have ever run in this project (AI Juggler runs
    /// plus imported pre-existing history).
    pub accounts_used: Vec<String>,
    /// Total instances run through AI Juggler in this project.
    #[ts(type = "number")]
    pub instance_count: u64,
    /// Sessions discovered in imported transcripts (pre-AI-Juggler work).
    #[ts(type = "number")]
    pub history_sessions: u64,
    /// When the user last worked here outside AI Juggler (the newest
    /// imported transcript's write time), epoch ms.
    #[ts(type = "number | null")]
    pub history_last_used_ms: Option<i64>,
    /// The repository this folder is a checkout of (its `origin` remote,
    /// normalised: `github.com/owner/name`), so clones group together.
    pub repo: Option<String>,
    /// True when the root directory no longer exists on disk (AJ-E408
    /// Overview danger banner + `Relocate…`).
    pub missing: bool,
    /// AJ-E1501: the account this project's new sessions start with,
    /// when the user picked one.
    #[ts(optional = nullable)]
    pub default_account_id: Option<String>,
    /// AJ-E1502: pinned to the top of the rail.
    #[ts(optional)]
    pub pinned: Option<bool>,
    /// AJ-E1502: forgotten — hidden from the rail and New session until
    /// it is used again.
    #[ts(optional)]
    pub forgotten: Option<bool>,
    /// AJ-E1532: its mark — its own icon, or letters on a colour — the
    /// same on every screen. Set once its repository is known.
    #[ts(optional)]
    pub mark: Option<mark::ProjectMarkInfo>,
}
