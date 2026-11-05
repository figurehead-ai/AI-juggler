//! 0.4: what a switch names before anything moves, and what stays behind.
//!
//! The confirm step names the conversation that moves (0.3.9 moved one
//! the user didn't pick), and says, before the switch, what a switch of
//! one conversation leaves where it is: the sessions started from it
//! (separate conversations, nested under it) and the pages on the tool's
//! website it links to (they belong to the account that made them; the
//! other account sees "This artifact is private").

use std::sync::Arc;

use juggler_core::model::{Account, TranscriptRef};

use super::switch_words::CarryItemInfo;
use crate::state::AppState;

/// The moving conversation's title and the lines on what stays.
pub struct Named {
    /// The title it shows under, when it has one.
    pub title: Option<String>,
    /// "Stays" lines for the summary (none when nothing stays).
    pub stays: Vec<CarryItemInfo>,
}

/// How many child titles a line names before "and N more".
const NAMED: usize = 3;

/// The conversation `reference` holds on `from`, switched to `to`.
#[must_use]
pub fn of(state: &Arc<AppState>, reference: &TranscriptRef, from: &Account, to: &Account) -> Named {
    let id = reference.native_id.clone().or_else(|| {
        reference
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
    });
    let title = id
        .as_deref()
        .and_then(|id| crate::features::conversations::title_of(state, from.id.as_str(), id));
    let mut stays = Vec::new();
    if let Some(id) = id.as_deref() {
        let children = children_of(state, from, id);
        if !children.is_empty() {
            stays.push(sessions_line(&children, &from.label));
        }
    }
    // A tool whose conversations link to its own website's pages says
    // which; a read that fails only leaves the line out.
    let pages = state
        .registry
        .handover(&from.harness)
        .and_then(|support| support.account_pages(reference).ok())
        .unwrap_or_default();
    if !pages.is_empty() {
        stays.push(pages_line(pages.len(), &from.label, &to.label));
    }
    Named { title, stays }
}

/// The titles of the sessions started from `id` on `from` (by the tool's
/// own record of which session started which).
fn children_of(state: &Arc<AppState>, from: &Account, id: &str) -> Vec<String> {
    let mut children: Vec<String> = crate::features::conversations::commands::started_from(state)
        .into_iter()
        .filter(|((tool, _), parent)| parent == id && *tool == from.harness.as_str())
        .map(|((_, child), _)| {
            crate::features::conversations::title_of(state, from.id.as_str(), &child)
                .unwrap_or_else(|| "an untitled session".to_owned())
        })
        .collect();
    children.sort();
    children
}

/// "3 sessions started from it": they stay, by name.
fn sessions_line(children: &[String], from: &str) -> CarryItemInfo {
    let count = children.len();
    let named = children
        .iter()
        .take(NAMED)
        .map(|title| format!("“{title}”"))
        .collect::<Vec<_>>()
        .join(", ");
    let more = count.saturating_sub(NAMED);
    let rest = if more > 0 {
        format!(" and {more} more")
    } else {
        String::new()
    };
    CarryItemInfo {
        label: if count == 1 {
            "1 session started from it".to_owned()
        } else {
            format!("{count} sessions started from it")
        },
        hint: format!(
            "Each is its own conversation and stays on {from}: {named}{rest}. Switch each one from its own page to move it too."
        ),
        how: "stays".to_owned(),
        kind: "sessions".to_owned(),
    }
}

/// "2 pages it links to": they belong to the account that made them.
fn pages_line(count: usize, from: &str, to: &str) -> CarryItemInfo {
    CarryItemInfo {
        label: if count == 1 {
            "1 page it links to".to_owned()
        } else {
            format!("{count} pages it links to")
        },
        hint: format!(
            "Pages published on the tool's website belong to the account that published them. Those {from} published stay with it, and on {to} they show as private."
        ),
        how: "stays".to_owned(),
        kind: "pages".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{pages_line, sessions_line};

    #[test]
    fn the_sessions_line_names_them_and_counts_the_rest() {
        let children: Vec<String> = ["A", "B", "C", "D", "E"]
            .iter()
            .map(|t| (*t).to_owned())
            .collect();
        let line = sessions_line(&children, "olivia");
        assert_eq!(line.label, "5 sessions started from it");
        assert_eq!(line.how, "stays");
        assert!(
            line.hint
                .contains("stays on olivia: “A”, “B”, “C” and 2 more."),
            "{}",
            line.hint
        );
        let one = sessions_line(&["Only".to_owned()], "olivia");
        assert_eq!(one.label, "1 session started from it");
        assert!(one.hint.contains("“Only”."), "{}", one.hint);
    }

    #[test]
    fn the_pages_line_says_whose_they_are() {
        let line = pages_line(17, "olivia", "luka");
        assert_eq!(line.label, "17 pages it links to");
        assert!(
            line.hint.contains("on luka they show as private"),
            "{}",
            line.hint
        );
    }
}
