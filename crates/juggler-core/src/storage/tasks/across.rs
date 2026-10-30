//! 0.3: the tasks store's reads across pools — how many items a query
//! matches in each honest status (the Tasks screen's column counts).

use rusqlite::params;

use super::{EFFECTIVE, TaskQuery, TasksRepo, collect, json, like_words};
use crate::error::CoreError;
use crate::tasks::TaskStatus;

impl TasksRepo<'_> {
    /// 0.3: how many items `query` matches in each honest status (its
    /// pools, holders, kinds, parent and words; its statuses and cursor
    /// aside): the Tasks screen's column counts, across pools.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn counts_where(&self, query: &TaskQuery<'_>) -> Result<Vec<(TaskStatus, u32)>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT eff, COUNT(*) FROM (SELECT {EFFECTIVE} AS eff FROM tasks t
              WHERE (?1 IS NULL OR t.pool_id = ?1)
                AND (?2 IS NULL OR t.assignee_ref IN (SELECT value FROM json_each(?2)))
                AND (?3 IS NULL OR t.kind IN (SELECT value FROM json_each(?3)))
                AND (?4 IS NULL OR (?4 = '' AND t.parent_id IS NULL) OR t.parent_id = ?4)
                AND (?5 IS NULL OR t.title LIKE '%' || ?5 || '%' ESCAPE '\\')
                AND (?6 IS NULL OR t.pool_id IN (SELECT value FROM json_each(?6)))
                AND (?7 = 0 OR t.assignee_ref IS NULL))
              GROUP BY eff"
        ))?;
        let kinds = query
            .kinds
            .map(|list| json(&list.iter().map(|k| k.as_str()).collect::<Vec<_>>()));
        let rows = stmt.query_map(
            params![
                query.pool_id,
                query.assignees.map(json),
                kinds,
                query.parent,
                query.words.map(like_words),
                query.pools.map(json),
                query.unheld,
            ],
            |row| Ok((TaskStatus::stored(&row.get::<_, String>(0)?), row.get(1)?)),
        )?;
        let mut counts = collect(rows)?;
        counts.sort_by_key(|(status, _)| status.rank());
        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{NewTask, TASK_USER};
    use super::*;
    use crate::storage::Storage;
    use crate::tasks::{Priority, TaskKind};

    fn new<'a>(pool: &'a str, id: &'a str) -> NewTask<'a> {
        NewTask {
            id,
            pool_id: pool,
            kind: TaskKind::Task,
            parent_id: None,
            title: id,
            body: "",
            acceptance: "",
            priority: Priority::Normal,
            status: TaskStatus::Ready,
            assignee: None,
            created_by: TASK_USER,
            created_by_name: "You",
            external_url: None,
            blocked_by: &[],
        }
    }

    #[test]
    fn across_pools_lists_and_counts_what_each_filter_names() {
        let storage = Storage::open_in_memory().unwrap();
        let team = storage.pools().create("Team", None).unwrap().id;
        let docs = storage.pools().create("Docs", None).unwrap().id;
        let other = storage.pools().create("Other", None).unwrap().id;
        let repo = storage.tasks();
        repo.create(&new(&team, "aj-0001")).unwrap();
        let held = NewTask {
            assignee: Some(("session:s1", "Builder")),
            status: TaskStatus::InProgress,
            ..new(&docs, "aj-0002")
        };
        repo.create(&held).unwrap();
        repo.create(&new(&other, "aj-0003")).unwrap();
        let two = [team.clone(), docs.clone()];
        let ids = |query: &TaskQuery<'_>| {
            repo.list(query, 50)
                .unwrap()
                .into_iter()
                .map(|t| t.id)
                .collect::<Vec<_>>()
        };
        let both = TaskQuery {
            pools: Some(&two),
            ..TaskQuery::default()
        };
        assert_eq!(ids(&both), ["aj-0001", "aj-0002"], "the two pools named");
        let unheld = TaskQuery {
            unheld: true,
            ..both
        };
        assert_eq!(ids(&unheld), ["aj-0001"], "only what no one holds");
        let counts = repo.counts_where(&both).unwrap();
        assert_eq!(
            counts,
            [(TaskStatus::Ready, 1), (TaskStatus::InProgress, 1)],
            "one ready, one in progress across the two pools"
        );
        assert_eq!(
            repo.counts_where(&TaskQuery::default()).unwrap().len(),
            2,
            "every pool: ready and in progress"
        );
    }
}
