//! This Mac's AI tools on their own (#120): AI Juggler's sessions grouped
//! by tool (heaviest first), then each tool's desktop app (the Claude app,
//! the Codex app in `ChatGPT.app`), then each tool in a terminal, then
//! every other app's AI sessions the Processes view found (AJ-E2407: any
//! tool, any app), with each group's totals. The groups and their words
//! come from the adapters' shapes (`running_setups`) and the Processes
//! graph, never from a list here. Split from
//! `machine_health` (the size budget: a feature to a module).

use std::collections::{HashMap, HashSet};

use juggler_supervisor::ObservedTree;

use crate::features::processes::model::Graph;
use crate::machine_health::{OBSERVED_TOOL, ObservedWords, TreeInput};

/// AJ-E2407: the AI sessions other apps run, from the Processes view's
/// graph (any tool, any app), measured by the collector's trees for that
/// view. A session whose root an observed tree already counts (the Claude
/// tools' own scan: the Claude app's whole tree) is left to that row.
pub(crate) fn graph_trees(
    graph: &Graph,
    observed: &HashMap<u32, ObservedTree>,
    extra: &HashMap<u32, ObservedTree>,
) -> Vec<TreeInput> {
    let counted: HashSet<u32> = observed
        .values()
        .flat_map(|tree| tree.pids.iter().copied())
        .collect();
    let mut out = Vec::new();
    for app in graph.apps.iter().filter(|app| !app.ours) {
        for session in app.sessions.iter().filter(|session| session.alive) {
            let Some(root) = session.root_pid else {
                continue;
            };
            let Some(tree) = extra.get(&root).filter(|_| !counted.contains(&root)) else {
                continue;
            };
            let mut input = TreeInput {
                root_pid: Some(root),
                session_id: None,
                place: Some(match app.kind.as_deref() {
                    Some("terminal") => "terminal",
                    Some("editor") => "editor",
                    _ => "desktop",
                }),
                tool: Some(session.tool.clone()),
                account_label: None,
                stats: tree.stats,
                ring: tree.ring.iter().copied().collect(),
                hog_rules: Vec::new(),
                app: Some(app.name.clone()),
                title: (!session.title.trim().is_empty()).then(|| session.title.clone()),
                observed: None,
            };
            // Its group and words, as the Claude tools' scan gives theirs.
            let label = app_group_label(&input);
            input.observed = Some(ObservedWords {
                group: format!("app:{}:{}", app.name, session.tool),
                who: label.clone(),
                label,
            });
            out.push(input);
        }
    }
    out
}

/// Another app's group in words: the app for a desktop app (`ChatGPT`),
/// else the tool in it (`Codex in Terminal`).
fn app_group_label(tree: &TreeInput) -> String {
    let app = tree.app.clone().unwrap_or_default();
    match (tree.place, tree.tool.as_deref()) {
        (Some("terminal" | "editor"), Some(tool)) => {
            format!("{} in {app}", crate::commands::provider_name(tool))
        }
        _ => app,
    }
}

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
/// isn't known yet), or the group of one it didn't: its app, its tool in
/// a terminal (both from the adapters' shapes), or `app:<app>:<tool>` for
/// another app's sessions the Processes view found.
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
            .filter(|tree| tree.place == Some(place) && tree.app.is_none())
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
    // AJ-E2407: every other app's sessions, by app and tool, heaviest first.
    let mut apps: std::collections::BTreeMap<String, Vec<&TreeInput>> =
        std::collections::BTreeMap::new();
    for tree in theirs.iter().copied().filter(|tree| tree.app.is_some()) {
        apps.entry(group_of(tree)).or_default().push(tree);
    }
    let mut others: Vec<ToolGroupInfo> = apps
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
    others.sort_by_key(|group| std::cmp::Reverse(group.totals.rss_bytes));
    groups.extend(others);
    groups
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::features::processes::model::{AppNode, Origin, Rollup, SessionNode};
    use juggler_supervisor::LiveProcessStats;

    fn session(tool: &str, title: &str, root: u32, alive: bool) -> SessionNode {
        SessionNode {
            key: format!("s:theirs:{tool}:{root}"),
            origin: Origin::Theirs,
            session_id: format!("{root}"),
            tool: tool.into(),
            account_id: None,
            project_id: None,
            title: title.into(),
            folder: None,
            alive,
            state: if alive { "working" } else { "ended" },
            processes: Vec::new(),
            rollup: Rollup::default(),
            spark: Vec::new(),
            root_pid: Some(root),
        }
    }

    fn app(name: &str, kind: &str, sessions: Vec<SessionNode>) -> AppNode {
        AppNode {
            key: format!("a:{name}"),
            app_id: None,
            name: name.into(),
            pid: None,
            ours: false,
            kind: Some(kind.into()),
            sessions,
            rollup: Rollup::default(),
        }
    }

    fn measured(rss: u64, pids: &[u32]) -> ObservedTree {
        ObservedTree {
            stats: LiveProcessStats {
                child_count: 1,
                cpu_pct: Some(3.0),
                rss_bytes: rss,
            },
            pids: pids.to_vec(),
            ..ObservedTree::default()
        }
    }

    /// AJ-E2407: This Mac lists every other app's AI sessions the Processes
    /// view found, by app and tool, each by its title; a session a Claude
    /// tool's own row already counts, or one that ended, isn't counted twice.
    #[test]
    fn other_apps_sessions_come_from_the_graph_grouped_by_app_and_tool() {
        let graph = Graph {
            apps: vec![
                app(
                    "ChatGPT",
                    "desktop",
                    vec![
                        session("codex", "Landing page", 7300, true),
                        session("codex", "Old one", 7310, false),
                    ],
                ),
                app(
                    "Terminal",
                    "terminal",
                    vec![session("codex", "", 7400, true)],
                ),
                app(
                    "Claude",
                    "desktop",
                    vec![session("claude-code", "Blog", 9001, true)],
                ),
            ],
            ..Graph::default()
        };
        // The Claude app's own tree (its scan's row) holds 9001.
        let observed = HashMap::from([(9000, measured(4 << 30, &[9000, 9001]))]);
        let extra = HashMap::from([
            (7300, measured(195 << 20, &[7300])),
            (7310, measured(10 << 20, &[7310])),
            (7400, measured(700 << 20, &[7400])),
            (9001, measured(300 << 20, &[9001])),
        ]);
        let trees = graph_trees(&graph, &observed, &extra);
        let roots: Vec<Option<u32>> = trees.iter().map(|tree| tree.root_pid).collect();
        assert_eq!(roots, vec![Some(7300), Some(7400)]);
        assert_eq!(trees[0].title.as_deref(), Some("Landing page"));
        assert_eq!(trees[1].title, None, "no title: its app's name shows");
        let theirs: Vec<&TreeInput> = trees.iter().collect();
        let groups = by_tool(&[], &theirs, 8);
        let words: Vec<(&str, &str)> = groups
            .iter()
            .map(|group| (group.tool.as_str(), group.label.as_str()))
            .collect();
        assert_eq!(
            words,
            vec![
                ("app:Terminal:codex", "Codex in Terminal"),
                ("app:ChatGPT:codex", "ChatGPT"),
            ],
            "heaviest first, none of them under the Claude app"
        );
        assert!(groups.iter().all(|group| !group.ours));
    }
}
