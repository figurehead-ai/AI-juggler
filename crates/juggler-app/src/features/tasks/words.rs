//! D-25: on screen a task is its title, never its short id. What reaches
//! the window about tasks — Needs you's lines, a task's history, a
//! switch history's task, a message's subject — names each task by its
//! title, quoted and kept short; a task that's gone says so. Text written
//! before this (an id in a stored line) is named at read time.

use juggler_core::messaging::one_line;
use juggler_core::storage::Storage;
use juggler_core::tasks::{ID_PREFIX, is_short_id};

/// The longest title a line quotes.
pub const TITLE_MAX: usize = 60;

/// What a task that was removed is called.
pub const REMOVED: &str = "a task that was removed";

/// A title as a line quotes it: «Round tax per line», shortened.
#[must_use]
pub fn quoted(title: &str) -> String {
    let line = one_line(title, TITLE_MAX + 1);
    if line.chars().count() > TITLE_MAX {
        let cut: String = line.chars().take(TITLE_MAX - 1).collect();
        format!("«{}…»", cut.trim_end())
    } else {
        format!("«{line}»")
    }
}

/// A task by its id, as a line names it: its title quoted, or
/// [`REMOVED`].
#[must_use]
pub fn task_named(storage: &Storage, id: &str) -> String {
    storage
        .tasks()
        .get(id)
        .ok()
        .flatten()
        .map_or_else(|| REMOVED.to_owned(), |task| quoted(&task.title))
}

/// `text` with every task id in it named by its title ("linked aj-7f3c
/// blocks aj-b10c" → "linked «Orders API» blocks «Totals in cents»").
/// Only whole words that are short ids are named; the rest is as it was.
#[must_use]
pub fn name_tasks(storage: &Storage, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(ID_PREFIX) {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        let end = ID_PREFIX.len()
            + from[ID_PREFIX.len()..]
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(from.len() - ID_PREFIX.len());
        let word = &from[..end];
        let starts_a_word = !out
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_');
        if starts_a_word && is_short_id(word) {
            out.push_str(&task_named(storage, word));
        } else {
            out.push_str(word);
        }
        rest = &from[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_core::storage::{NewTask, TASK_USER};
    use juggler_core::tasks::{Priority, TaskKind, TaskStatus};

    #[test]
    fn ids_are_named_by_their_titles_and_a_gone_one_says_so() {
        let storage = Storage::open_in_memory().unwrap();
        let pool = storage.pools().create("Team", None).unwrap().id;
        let long = "Orders API: create and fetch an order, with totals in integer cents everywhere";
        storage
            .tasks()
            .create(&NewTask {
                id: "aj-7f3c",
                pool_id: &pool,
                kind: TaskKind::Task,
                parent_id: None,
                title: long,
                body: "",
                acceptance: "",
                priority: Priority::Normal,
                status: TaskStatus::Ready,
                assignee: None,
                created_by: TASK_USER,
                created_by_name: "You",
                external_url: None,
                blocked_by: &[],
            })
            .unwrap();
        let named = name_tasks(
            &storage,
            "linked aj-7f3c blocks aj-b10c (not xaj-1234, aj-zz)",
        );
        assert!(
            !named.contains("aj-7f3c") && !named.contains("aj-b10c"),
            "no id is left"
        );
        assert!(
            named.starts_with("linked «Orders API"),
            "named by its title"
        );
        assert!(
            named.contains(&format!("blocks {REMOVED}")),
            "a gone task says so"
        );
        assert!(
            named.ends_with("(not xaj-1234, aj-zz)"),
            "only whole short ids"
        );
        assert!(
            quoted(long).chars().count() <= TITLE_MAX + 2,
            "a long title is shortened"
        );
    }
}
