//! AJ-E1411: each AI tool's share of This Mac — our sessions by Provider,
//! then each tool AI Juggler didn't start — as the menu bar lists them.
//! Names and counts come from the backend's own words (a Provider's
//! display name; [`crate::running_setups::place_words`]), so no surface
//! spells a tool.

use crate::machine_health::{ToolTotalsInfo, TreeInput, totals};
use crate::running_setups::place_words;

/// One AI tool's share of This Mac.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolGroupInfo {
    /// The Provider's id for sessions AI Juggler started; the place
    /// (`desktop` | `terminal`) for the tools it didn't.
    pub tool: String,
    /// The tool in words ("Claude Code", "Claude app").
    pub label: String,
    /// How many, in words ("2 sessions", "3 windows").
    pub count_words: String,
    /// Whether these are sessions AI Juggler started.
    pub ours: bool,
    /// The tool's figures, as [`ToolTotalsInfo`].
    pub totals: ToolTotalsInfo,
    /// CPU as Home's rows count it: the tool's trees summed, 100% is one
    /// core busy (Activity Monitor's measure). `None` until every tree
    /// is measured.
    pub cpu_one_core_pct: Option<f64>,
}

/// CPU summed over trees as Home's rows count it (100% is one core);
/// `None` while any tree is unmeasured.
fn one_core(trees: &[&TreeInput]) -> Option<f64> {
    trees
        .iter()
        .map(|tree| tree.stats.cpu_pct)
        .sum::<Option<f64>>()
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Pure: each AI tool on its own — our sessions grouped by Provider
/// (heaviest first), then each tool AI Juggler didn't start, in the
/// order its places are listed.
#[must_use]
pub fn by_tool(ours: &[&TreeInput], theirs: &[&TreeInput], cores: u32) -> Vec<ToolGroupInfo> {
    let mut providers: std::collections::BTreeMap<Option<String>, Vec<&TreeInput>> =
        std::collections::BTreeMap::new();
    for tree in ours {
        providers
            .entry(tree.provider.clone())
            .or_default()
            .push(tree);
    }
    let mut groups: Vec<ToolGroupInfo> = providers
        .into_iter()
        .map(|(provider, trees)| ToolGroupInfo {
            // A session with no launch record yet is still ours; it's
            // grouped as plain sessions rather than guessed at.
            label: provider.as_deref().map_or_else(
                || "Sessions".to_string(),
                |id| crate::commands::provider_display(id).0,
            ),
            tool: provider.unwrap_or_default(),
            count_words: count(trees.len(), "session", "sessions"),
            ours: true,
            totals: totals(&trees, cores),
            cpu_one_core_pct: one_core(&trees),
        })
        .collect();
    groups.sort_by_key(|group| std::cmp::Reverse(group.totals.rss_bytes));
    let mut places: Vec<&str> = Vec::new();
    for tree in theirs {
        if let Some(place) = tree.place
            && !places.contains(&place)
        {
            places.push(place);
        }
    }
    places.sort_unstable();
    for place in places {
        let trees: Vec<&TreeInput> = theirs
            .iter()
            .copied()
            .filter(|tree| tree.place == Some(place))
            .collect();
        let words = place_words(place);
        groups.push(ToolGroupInfo {
            tool: place.to_string(),
            label: words.name.to_string(),
            count_words: count(trees.len(), words.one, words.many),
            ours: false,
            totals: totals(&trees, cores),
            cpu_one_core_pct: one_core(&trees),
        });
    }
    groups
}
