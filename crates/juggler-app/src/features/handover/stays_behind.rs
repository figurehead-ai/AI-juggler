//! 0.4: what a switch names before anything moves.
//!
//! The confirm step names the conversation that moves (0.3.9 moved one
//! the user didn't pick); the sessions started from it, which move with
//! it (ADR-0028), counted and named as the switch will find them; and
//! what stays: a family member whose conversation isn't on this computer, a
//! family across tools, and the pages on the tool's website it links to
//! (they belong to the account that made them; the other account sees
//! "This artifact is private").

use std::sync::Arc;

use juggler_core::model::{Account, TranscriptRef};

use super::switch_words::CarryItemInfo;
use crate::state::AppState;

/// The moving conversation's title, its family, and the lines on what
/// moves with it and what stays.
pub struct Named {
    /// The title it shows under, when it has one.
    pub title: Option<String>,
    /// ADR-0028: how many sessions started from it move with it.
    pub family: u32,
    /// Lines for the summary: the family that moves, what stays (none
    /// when nothing does).
    pub stays: Vec<CarryItemInfo>,
    /// ADR-0028: what wakes it by itself, and what happens to it.
    pub automation: super::automation::Preview,
}

/// What moves, as the confirm step names it (the preflight's
/// `source_title`, `source_account` and `source_family`).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct Moving {
    /// The conversation's title (None when it has none).
    pub source_title: Option<String>,
    /// The account it is on now.
    pub source_account: String,
    /// ADR-0028: how many sessions started from it move with it.
    pub source_family: u32,
    /// ADR-0028: what wakes it by itself, handed over to the copy.
    #[serde(flatten)]
    pub automation: super::automation::Preview,
}

impl Named {
    /// What moves, from `from`.
    #[must_use]
    pub fn moving(&self, from: &Account) -> Moving {
        Moving {
            source_title: self.title.clone(),
            source_account: from.label.clone(),
            source_family: self.family,
            automation: self.automation.clone(),
        }
    }
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
    let mut family = 0;
    // ADR-0028: the family the switch moves (the host app's side sessions,
    // and the threads the tool keeps itself), named here exactly as the
    // switch will find it.
    let host = super::family::find(state, from, reference);
    let own = state
        .registry
        .handover(&from.harness)
        .map_or(0, |support| support.own_family(reference).len());
    let titles: Vec<String> = host
        .members
        .iter()
        .map(|m| {
            m.title
                .clone()
                .unwrap_or_else(|| "an untitled session".to_owned())
        })
        .collect();
    if from.harness == to.harness {
        family = u32::try_from(host.members.len() + own).unwrap_or(u32::MAX);
        if family > 0 {
            stays.push(moves_line(&titles, family as usize));
        }
        if !host.missing.is_empty() {
            stays.push(missing_line(&host.missing, &from.label));
        }
    } else if !titles.is_empty() || own > 0 || !host.missing.is_empty() {
        let mut all = titles;
        all.extend(host.missing);
        stays.push(other_tool_line(&all, own, &from.label));
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
    let automation = super::automation::preview(state, from, to, reference, &host.refs);
    Named {
        title,
        family,
        stays,
        automation,
    }
}

/// "“A”, “B”, “C” and 2 more".
fn listed(titles: &[String], count: usize) -> String {
    let named = titles
        .iter()
        .take(NAMED)
        .map(|title| format!("“{title}”"))
        .collect::<Vec<_>>()
        .join(", ");
    let more = count.saturating_sub(titles.len().min(NAMED));
    match (named.is_empty(), more) {
        (true, n) => format!("{n}"),
        (false, 0) => named,
        (false, n) => format!("{named} and {n} more"),
    }
}

fn sessions(count: usize) -> String {
    if count == 1 {
        "1 session started from it".to_owned()
    } else {
        format!("{count} sessions started from it")
    }
}

/// The family that moves with it.
fn moves_line(titles: &[String], count: usize) -> CarryItemInfo {
    CarryItemInfo {
        label: sessions(count),
        hint: format!(
            "{}: each moves with it, whole, with its own subagents, and sits under it again on the other account.",
            listed(titles, count)
        ),
        how: "moves".to_owned(),
        kind: "sessions".to_owned(),
    }
}

/// Members whose conversation file isn't on this computer.
fn missing_line(titles: &[String], from: &str) -> CarryItemInfo {
    CarryItemInfo {
        label: format!(
            "{} not on this computer",
            if titles.len() == 1 {
                "1 of them is".to_owned()
            } else {
                format!("{} of them are", titles.len())
            }
        ),
        hint: format!(
            "{}: the app lists {} but {} conversation isn't here, so {} on {from}.",
            listed(titles, titles.len()),
            if titles.len() == 1 { "it" } else { "them" },
            if titles.len() == 1 { "its" } else { "their" },
            if titles.len() == 1 {
                "it stays"
            } else {
                "they stay"
            },
        ),
        how: "stays".to_owned(),
        kind: "sessions".to_owned(),
    }
}

/// Across tools a conversation replays into one new one: its family stays.
fn other_tool_line(titles: &[String], own: usize, from: &str) -> CarryItemInfo {
    let count = titles.len() + own;
    CarryItemInfo {
        label: sessions(count),
        hint: format!(
            "{}: a switch to another tool starts one new conversation from this one, so they stay on {from}.",
            listed(titles, count)
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
    use super::{moves_line, other_tool_line, pages_line};

    #[test]
    fn the_family_line_names_them_and_counts_the_rest() {
        let titles: Vec<String> = ["A", "B", "C", "D", "E"]
            .iter()
            .map(|t| (*t).to_owned())
            .collect();
        let line = moves_line(&titles, 20);
        assert_eq!(line.label, "20 sessions started from it");
        assert_eq!(line.how, "moves");
        assert!(
            line.hint
                .starts_with("“A”, “B”, “C” and 17 more: each moves with it"),
            "{}",
            line.hint
        );
        let one = moves_line(&["Only".to_owned()], 1);
        assert_eq!(one.label, "1 session started from it");
        // Threads the tool keeps itself have no titles here: counted.
        assert!(
            moves_line(&[], 4).hint.starts_with("4: each moves"),
            "{}",
            moves_line(&[], 4).hint
        );
    }

    #[test]
    fn across_tools_the_family_stays_and_says_why() {
        let line = other_tool_line(&["A".to_owned()], 0, "olivia");
        assert_eq!(line.how, "stays");
        assert!(
            line.hint
                .contains("another tool starts one new conversation"),
            "{}",
            line.hint
        );
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
