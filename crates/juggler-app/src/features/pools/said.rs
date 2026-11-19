//! F-rc3g-7 (0.4.6 rc3g, the P4 walk): the words a person names a pool
//! by. The Pools screen's card read "web (project pool)" while the pool's
//! name, in a session's Pools tab and in the agent's `whoami`, is "web ·
//! all sessions (in lab)"; "share it in the web pool" was refused. A pool
//! is named by its id, its name on screen ([`super::shown_pools`]) and,
//! for a project's own pool, its project's name and the card's words.
//!
//! The agent bridge takes these only among the pools a session is in,
//! and refuses a name two of them answer to, listing them by name.

use std::collections::HashMap;

use juggler_core::storage::{Pool, PoolKind, Storage, default_pool_name};

use crate::error::AppResult;

/// How a pool answers to the words asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    /// Its id.
    Id,
    /// Its name on screen ("web · all sessions (in lab)").
    Name,
    /// Words a person reads for it: a project's own pool by its
    /// project's name ("web") or its card's words ("web (project pool)").
    Said,
}

/// Every pool that isn't deleted, with its name on screen and the other
/// words it answers to.
#[derive(Default)]
pub struct PoolNames {
    listed: Vec<(Pool, String, Vec<String>)>,
}

impl PoolNames {
    /// The pools as the screen names them now.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn read(storage: &Storage) -> AppResult<Self> {
        let projects = super::project_names(storage)?;
        let listed = super::shown_pools(storage)?
            .into_iter()
            .map(|(pool, name)| {
                let also = also_called(&pool, &projects);
                (pool, name, also)
            })
            .collect();
        Ok(Self { listed })
    }

    /// A pool's name on screen.
    #[must_use]
    pub fn name_of(&self, pool_id: &str) -> Option<&str> {
        self.listed
            .iter()
            .find(|(pool, _, _)| pool.id == pool_id)
            .map(|(_, name, _)| name.as_str())
    }

    /// How pool `pool_id` answers to `asked` (its id first, then its
    /// name, then the words a person reads for it); `None` when not.
    #[must_use]
    pub fn how(&self, pool_id: &str, asked: &str) -> Option<Named> {
        let (pool, name, also) = self.listed.iter().find(|(pool, _, _)| pool.id == pool_id)?;
        let wanted = plain(asked);
        if pool.id == asked.trim() {
            Some(Named::Id)
        } else if plain(name) == wanted {
            Some(Named::Name)
        } else if also.iter().any(|words| plain(words) == wanted) {
            Some(Named::Said)
        } else {
            None
        }
    }

    /// The ids of every pool that answers to `asked`.
    #[must_use]
    pub fn answering(&self, asked: &str) -> Vec<&str> {
        self.listed
            .iter()
            .filter(|(pool, _, _)| self.how(&pool.id, asked).is_some())
            .map(|(pool, _, _)| pool.id.as_str())
            .collect()
    }
}

/// Words compared as a person means them: case and spacing aside (a
/// pool's name here, a member's in `send_message`).
pub(crate) fn plain(words: &str) -> String {
    words
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// What a person reads for a project's own pool besides its name on
/// screen: its project's name, "web pool", the card's "web (project
/// pool)" and its name before it's told apart from a namesake ("web · all
/// sessions"); the no-project pool as "No project". A pool you made is
/// called by its name alone.
fn also_called(pool: &Pool, projects: &HashMap<String, String>) -> Vec<String> {
    if pool.kind != PoolKind::Default {
        return Vec::new();
    }
    let Some(project) = pool.project.as_ref() else {
        return vec![
            "No project".to_owned(),
            "sessions with no project".to_owned(),
        ];
    };
    let Some(name) = projects.get(project.as_str()) else {
        return Vec::new();
    };
    vec![
        name.clone(),
        format!("{name} pool"),
        format!("{name} project pool"),
        format!("{name} (project pool)"),
        default_pool_name(Some(name)),
    ]
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use std::path::Path;

    use super::{Named, PoolNames};
    use juggler_core::storage::{Scope, Storage, default_pool_id};

    #[test]
    fn a_projects_pool_answers_to_its_projects_name_and_its_cards_words() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("j.sqlite")).unwrap();
        let web = storage.projects().ensure(Path::new("/lab/web")).unwrap().id;
        let mine = storage
            .pools()
            .create_scoped("Writer and reviewer", &Scope::Every)
            .unwrap();
        let names = PoolNames::read(&storage).unwrap();
        let own = default_pool_id(Some(&web));
        assert_eq!(names.name_of(&own), Some("web · all sessions"));
        for asked in [
            "web",
            " WEB ",
            "web pool",
            "web (project pool)",
            "web  project pool",
        ] {
            assert_eq!(names.how(&own, asked), Some(Named::Said), "{asked}");
        }
        assert_eq!(names.how(&own, "web · all sessions"), Some(Named::Name));
        assert_eq!(names.how(&own, &own), Some(Named::Id));
        assert_eq!(names.how(&own, "we"), None);
        // A pool you made answers to its name only.
        assert_eq!(
            names.how(&mine.id, "writer and reviewer"),
            Some(Named::Name)
        );
        assert_eq!(names.how(&mine.id, "Writer"), None);
        assert_eq!(names.answering("web"), vec![own.as_str()]);
    }

    /// Two projects called web: each pool's name on screen says its
    /// folder ("(in lab)"), and each still answers to "web".
    #[test]
    fn namesake_projects_pools_are_told_apart_and_both_answer_to_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("j.sqlite")).unwrap();
        let lab = storage
            .projects()
            .ensure(Path::new("/u/lab/web"))
            .unwrap()
            .id;
        let work = storage
            .projects()
            .ensure(Path::new("/u/work/web"))
            .unwrap()
            .id;
        let names = PoolNames::read(&storage).unwrap();
        let (lab, work) = (default_pool_id(Some(&lab)), default_pool_id(Some(&work)));
        assert_eq!(names.name_of(&lab), Some("web · all sessions (in lab)"));
        assert_eq!(names.name_of(&work), Some("web · all sessions (in work)"));
        assert_eq!(
            names.how(&lab, "web · all sessions (in lab)"),
            Some(Named::Name)
        );
        assert_eq!(names.how(&lab, "web · all sessions"), Some(Named::Said));
        assert_eq!(names.how(&lab, "web (in work)"), None);
        let mut both = names.answering("web");
        both.sort_unstable();
        let mut want = vec![lab.as_str(), work.as_str()];
        want.sort_unstable();
        assert_eq!(both, want);
    }
}
