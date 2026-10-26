//! This Mac's AI tools on their own (#120): AI Juggler's sessions grouped
//! by tool (heaviest first), then each tool's desktop app (the Claude app,
//! the Codex app in `ChatGPT.app`), then each tool in a terminal, with each
//! group's totals. The groups and their words come from the adapters'
//! shapes (`running_setups`), never from a list here. Split from
//! `machine_health` (the size budget: a feature to a module).

use crate::machine_health::{OBSERVED_TOOL, TreeInput};

/// AI tools' share of the Mac.
#[derive(Clone, PartialEq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolTotalsInfo {
    /// How many trees.
    pub count: u32,
    /// The memory includes estimates (a tool AI Juggler didn't start).
    pub memory_estimate: bool,
    /// Share of the whole Mac's CPU (all cores), percent; `null` until
    /// measured.
    pub cpu_pct: Option<f64>,
    /// Memory, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
}

/// One AI tool's share of This Mac (the menu bar, AJ-E1411).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolGroupInfo {
    /// The tool's id (its adapter's) for sessions AI Juggler started; for
    /// the tools it didn't, the group's key: an app's id (`claude-app`,
    /// `codex-app`) or `terminal:<tool>`.
    pub tool: String,
    /// The tool in words ("Claude Code", "Claude app", `ChatGPT (Codex)`,
    /// "claude in a terminal").
    pub label: String,
    /// How an alert names a tool AI Juggler didn't start ("The Claude
    /// app"); `null` for our sessions (each is named by its own name).
    pub who: Option<String>,
    /// Whether these are sessions AI Juggler started.
    pub ours: bool,
    /// The tool's figures, as [`ToolTotalsInfo`].
    pub totals: ToolTotalsInfo,
    /// CPU as Home's rows count it: the tool's trees summed, 100% is one
    /// core busy (Activity Monitor's measure). `None` until every tree
    /// is measured.
    pub cpu_one_core_pct: Option<f64>,
}

#[allow(clippy::cast_precision_loss)] // shares for display
pub(crate) fn totals(trees: &[&TreeInput], cores: u32) -> ToolTotalsInfo {
    let measured: Vec<f64> = trees.iter().filter_map(|tree| tree.stats.cpu_pct).collect();
    ToolTotalsInfo {
        count: u32::try_from(trees.len()).unwrap_or(u32::MAX),
        memory_estimate: trees.iter().any(|tree| tree.place.is_some()),
        cpu_pct: (!trees.is_empty() && measured.len() == trees.len())
            .then(|| (measured.iter().sum::<f64>() / f64::from(cores.max(1))).min(100.0))
            .or_else(|| trees.is_empty().then_some(0.0)),
        rss_bytes: trees.iter().map(|tree| tree.stats.rss_bytes).sum(),
    }
}

/// CPU summed over trees as Home's rows count it (100% is one core);
/// `None` while any tree is unmeasured.
pub(crate) fn one_core(trees: &[&TreeInput]) -> Option<f64> {
    trees
        .iter()
        .map(|tree| tree.stats.cpu_pct)
        .sum::<Option<f64>>()
}

/// The group a tree is listed under: its [`ToolGroupInfo::tool`] — the
/// tool of a session AI Juggler started (the observed tool when its tool
/// isn't known yet), or the group of one it didn't (its app, or its tool
/// in a terminal).
pub(crate) fn group_of(tree: &TreeInput) -> String {
    match (&tree.observed, tree.place) {
        (Some(words), _) => words.group.clone(),
        (None, Some(place)) => place.to_string(),
        (None, None) => tree
            .tool
            .clone()
            .unwrap_or_else(|| OBSERVED_TOOL.to_string()),
    }
}

/// Each AI tool on its own: our sessions grouped by Provider (heaviest
/// first), then each tool's desktop app, then each tool in a terminal
/// (each heaviest first).
pub(crate) fn by_tool(
    ours: &[&TreeInput],
    theirs: &[&TreeInput],
    cores: u32,
) -> Vec<ToolGroupInfo> {
    let mut providers: std::collections::BTreeMap<String, Vec<&TreeInput>> =
        std::collections::BTreeMap::new();
    for tree in ours {
        providers.entry(group_of(tree)).or_default().push(tree);
    }
    let mut groups: Vec<ToolGroupInfo> = providers
        .into_iter()
        .map(|(provider, trees)| ToolGroupInfo {
            label: crate::commands::provider_name(&provider),
            tool: provider,
            who: None,
            ours: true,
            totals: totals(&trees, cores),
            cpu_one_core_pct: one_core(&trees),
        })
        .collect();
    groups.sort_by_key(|group| std::cmp::Reverse(group.totals.rss_bytes));
    for place in ["desktop", "terminal"] {
        let mut keyed: std::collections::BTreeMap<String, Vec<&TreeInput>> =
            std::collections::BTreeMap::new();
        for tree in theirs
            .iter()
            .copied()
            .filter(|tree| tree.place == Some(place))
        {
            keyed.entry(group_of(tree)).or_default().push(tree);
        }
        let mut found: Vec<ToolGroupInfo> = keyed
            .into_iter()
            .map(|(key, trees)| {
                let words = trees.iter().find_map(|tree| tree.observed.as_ref());
                ToolGroupInfo {
                    label: words.map_or_else(|| key.clone(), |words| words.label.clone()),
                    who: words.map(|words| words.who.clone()),
                    tool: key,
                    ours: false,
                    totals: totals(&trees, cores),
                    cpu_one_core_pct: one_core(&trees),
                }
            })
            .collect();
        found.sort_by_key(|group| std::cmp::Reverse(group.totals.rss_bytes));
        groups.extend(found);
    }
    groups
}
