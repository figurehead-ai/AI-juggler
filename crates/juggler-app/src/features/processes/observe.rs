//! AJ-E2401: what the 2 s tick already holds, gathered for the graph. Every
//! read here is of state another part of the tick keeps up to date (the
//! ledger, the collector's table and trees, the live view's model): no
//! process is scanned and no environment is read (D-37).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use juggler_core::model::SessionId;
use juggler_core::storage::LedgerRow;
use juggler_supervisor::{ObservedRoot, ObservedTree, ProcessRow};

use super::model::{AppRef, Live, OursInput, TheirsInput};
use crate::state::AppState;

/// A collector row as the graph's live facts.
#[must_use]
pub fn live_of(row: &ProcessRow) -> Live {
    Live {
        pid: row.pid,
        start_time: row.start_time,
        parent_pid: row.parent_pid,
        name: row.name.clone(),
        cpu_pct: row.cpu_pct,
        rss_bytes: row.rss_bytes,
        cpu_time_ms: row.cpu_time_ms,
        zombie: row.zombie,
    }
}

/// What one tick reads from the collector, under its lock once.
#[derive(Debug, Default)]
pub struct CollectorView {
    /// Each supervised session's live rows and CPU ring.
    pub trees: HashMap<String, (Vec<Live>, Vec<f32>)>,
    /// The observed tools' trees (This Mac's), by root.
    pub observed: HashMap<u32, ObservedTree>,
    /// The Processes view's own observed trees, by root.
    pub extra: HashMap<u32, ObservedTree>,
    /// Listening ports by pid.
    pub ports: HashMap<u32, Vec<u16>>,
    /// Live facts of the asked pids (the ledger's), from the table.
    pub live: HashMap<u32, Live>,
    /// Program and command line of the asked pids (role words only).
    pub commands: HashMap<u32, (String, String, i64)>,
}

/// Reads the collector once: the supervised trees, the observed trees, the
/// ports, and the facts of `pids` (from the tick's own table: their CPU
/// over the last second, never a new read).
#[must_use]
pub(crate) fn collector_view(
    state: &Arc<AppState>,
    sessions: &[SessionId],
    pids: &HashSet<u32>,
) -> CollectorView {
    let table = state.supervisor.snapshot_table();
    let Ok(collector) = state.resources.lock() else {
        return CollectorView::default();
    };
    let mut view = CollectorView::default();
    for session in sessions {
        let rows = collector.rows(session).unwrap_or_default();
        let ring: Vec<f32> = collector
            .ring(session)
            .iter()
            .map(|sample| super::model::f32_of(sample.cpu_pct.unwrap_or(0.0)))
            .collect();
        view.trees.insert(
            session.as_str().to_owned(),
            (rows.iter().map(live_of).collect(), ring),
        );
    }
    view.observed = collector.observed_snapshot();
    view.extra = collector.observed_extra_snapshot();
    for listener in collector.listeners() {
        view.ports
            .entry(listener.pid)
            .or_default()
            .push(listener.port);
    }
    let mut wanted: HashSet<u32> = pids.clone();
    for tree in view.observed.values().chain(view.extra.values()) {
        wanted.extend(tree.pids.iter().copied());
    }
    for (rows, _) in view.trees.values() {
        wanted.extend(rows.iter().map(|row| row.pid));
    }
    if let Some(table) = table.as_ref() {
        for pid in pids {
            let Some(facts) = table.facts.get(pid) else {
                continue;
            };
            view.live.insert(
                *pid,
                Live {
                    pid: *pid,
                    start_time: facts.start_time,
                    parent_pid: facts.ppid,
                    name: facts.name.clone(),
                    cpu_pct: collector.cpu_now(*pid),
                    rss_bytes: facts.rss_bytes,
                    cpu_time_ms: facts.cpu_time_ms,
                    zombie: facts.zombie,
                },
            );
            // A zombie's parent, to name what it waits for.
            if facts.zombie
                && let Some(parent) = facts.ppid
                && let Some(parent_facts) = table.facts.get(&parent)
            {
                view.live.entry(parent).or_insert_with(|| Live {
                    pid: parent,
                    start_time: parent_facts.start_time,
                    parent_pid: parent_facts.ppid,
                    name: parent_facts.name.clone(),
                    cpu_pct: None,
                    rss_bytes: parent_facts.rss_bytes,
                    cpu_time_ms: parent_facts.cpu_time_ms,
                    zombie: parent_facts.zombie,
                });
            }
        }
    }
    for pid in wanted {
        if let Some(brief) = collector.brief(pid) {
            view.commands.insert(
                pid,
                (
                    brief.exe.unwrap_or_else(|| brief.name.clone()),
                    brief.cmd,
                    brief.start_time,
                ),
            );
        }
    }
    view
}

/// One session's processes from a tree that holds it: `root` and its
/// descendants, stopping at another session's root (a session started
/// inside another is its own).
#[must_use]
pub(crate) fn subtree(rows: &[ProcessRow], root: u32, other_roots: &HashSet<u32>) -> Vec<Live> {
    let mut children: HashMap<u32, Vec<&ProcessRow>> = HashMap::new();
    let mut by_pid: HashMap<u32, &ProcessRow> = HashMap::new();
    for row in rows {
        by_pid.insert(row.pid, row);
        if let Some(parent) = row.parent_pid {
            children.entry(parent).or_default().push(row);
        }
    }
    let Some(top) = by_pid.get(&root) else {
        return Vec::new();
    };
    let mut out = vec![live_of(top)];
    let mut stack: Vec<u32> = vec![root];
    let mut seen: HashSet<u32> = HashSet::from([root]);
    while let Some(pid) = stack.pop() {
        for child in children.get(&pid).into_iter().flatten() {
            if other_roots.contains(&child.pid) || !seen.insert(child.pid) {
                continue;
            }
            out.push(live_of(child));
            stack.push(child.pid);
        }
    }
    out
}

/// The sessions other apps run, from the live view's model: one per place
/// (a session open in two apps is two), never one AI Juggler started (its
/// pid is in the ledger or a supervised tree).
#[must_use]
pub(crate) fn theirs_sessions(
    model: &crate::features::conversations::scan::LiveModel,
    ours: &HashSet<u32>,
    view: &CollectorView,
    titles: &HashMap<(String, String), String>,
    folders: &HashMap<(String, String), std::path::PathBuf>,
) -> (Vec<TheirsInput>, Vec<ObservedRoot>) {
    let roots: HashSet<u32> = model
        .entries
        .iter()
        .flat_map(|entry| entry.places.iter().map(|place| place.pid))
        .collect();
    let mut sessions = Vec::new();
    let mut extra = Vec::new();
    for entry in &model.entries {
        for place in &entry.places {
            if ours.contains(&place.pid) {
                continue;
            }
            let others: HashSet<u32> = roots
                .iter()
                .copied()
                .filter(|pid| *pid != place.pid)
                .collect();
            let held = view
                .observed
                .values()
                .chain(view.extra.values())
                .find(|tree| tree.pids.contains(&place.pid));
            let (tree, ring) = match held {
                Some(tree) => {
                    let ring = if tree.rows.first().is_some_and(|row| row.pid == place.pid) {
                        tree.ring
                            .iter()
                            .map(|sample| super::model::f32_of(sample.cpu_pct.unwrap_or(0.0)))
                            .collect()
                    } else {
                        Vec::new()
                    };
                    (subtree(&tree.rows, place.pid, &others), ring)
                }
                None => (Vec::new(), Vec::new()),
            };
            // Not measured yet (or only inside a bigger tree): measure it.
            if !view.extra.contains_key(&place.pid)
                && held.is_none_or(|tree| tree.rows.first().is_none_or(|row| row.pid != place.pid))
            {
                extra.push(ObservedRoot {
                    pid: place.pid,
                    start_time: place.start,
                });
            }
            if tree.is_empty() {
                continue; // nothing measured yet: next listing
            }
            let host = place.host.as_ref();
            let title = entry
                .app_title
                .clone()
                .filter(|title| !title.trim().is_empty())
                .or_else(|| {
                    titles
                        .get(&(entry.tool.clone(), entry.session_id.clone()))
                        .cloned()
                })
                .unwrap_or_default();
            sessions.push(TheirsInput {
                tool: entry.tool.clone(),
                session_id: entry.session_id.clone(),
                account_id: entry.account_id.clone(),
                title,
                folder: folders
                    .get(&(entry.tool.clone(), entry.session_id.clone()))
                    .cloned(),
                app: AppRef {
                    app_id: host.and_then(|host| host.app_id.clone()),
                    name: host.map_or_else(|| "Terminal".to_owned(), |host| host.name.clone()),
                    pid: host.map(|host| host.pid),
                    kind: Some(
                        match host.and_then(|host| host.kind) {
                            Some(juggler_adapter::live::HostKind::Desktop) => "desktop",
                            Some(juggler_adapter::live::HostKind::Editor) => "editor",
                            _ => "terminal",
                        }
                        .to_owned(),
                    ),
                },
                root_pid: place.pid,
                root_start: place.start.unwrap_or_default(),
                busy: entry.state.map(|state| {
                    matches!(state, crate::features::conversations::scan::State::Working)
                }),
                ring,
                tree,
            });
        }
    }
    (sessions, extra)
}

/// What the graph knows of one session AI Juggler started (cached; read
/// from storage every few ticks).
#[derive(Clone, Debug, Default)]
pub struct OursMeta {
    /// Its folder's name (the title when its conversation has none).
    pub title: String,
    /// The tool's id for its conversation, when known.
    pub conversation: Option<String>,
    /// Its folder.
    pub folder: Option<std::path::PathBuf>,
    /// Its stored status says it ended.
    pub marked_ended: bool,
}

/// Reads the stored title and status of each session.
#[must_use]
pub fn ours_meta(state: &Arc<AppState>, sessions: &[String]) -> HashMap<String, OursMeta> {
    let Ok(storage) = state.storage.lock() else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for id in sessions {
        let Ok(record) = storage.sessions().get(&SessionId::new(id.clone())) else {
            continue;
        };
        let folder = record
            .cwd
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let status = record.status.as_str();
        out.insert(
            id.clone(),
            OursMeta {
                title: folder,
                folder: Some(record.cwd.clone()),
                conversation: record
                    .conversation
                    .as_ref()
                    .map(|id| id.as_str().to_owned()),
                marked_ended: matches!(status, "stopped" | "finished" | "exited" | "failed"),
            },
        );
    }
    out
}

/// The sessions AI Juggler started, from the ledger's live rows and the
/// supervisor's roots.
#[must_use]
pub(crate) fn ours_sessions(
    rows: Vec<LedgerRow>,
    roots: &[(SessionId, u32, bool)],
    adopted: &HashSet<String>,
    meta: &HashMap<String, OursMeta>,
    titles: &HashMap<(String, String), String>,
    view: &CollectorView,
) -> Vec<OursInput> {
    let mut by_session: HashMap<String, Vec<LedgerRow>> = HashMap::new();
    for row in rows {
        by_session
            .entry(row.session_id.clone())
            .or_default()
            .push(row);
    }
    for (session, _, _) in roots {
        by_session.entry(session.as_str().to_owned()).or_default();
    }
    let finished: HashMap<&str, bool> = roots
        .iter()
        .map(|(session, _, finished)| (session.as_str(), *finished))
        .collect();
    by_session
        .into_iter()
        .map(|(session_id, rows)| {
            let first = rows.first();
            let (tree, ring) = view.trees.get(&session_id).cloned().unwrap_or_default();
            let meta = meta.get(&session_id).cloned().unwrap_or_default();
            let tool = first.map(|row| row.provider.clone()).unwrap_or_default();
            let title = meta
                .conversation
                .as_ref()
                .and_then(|id| titles.get(&(tool.clone(), id.clone())))
                .filter(|title| !title.trim().is_empty())
                .cloned()
                .unwrap_or(meta.title);
            OursInput {
                tool,
                account_id: first.and_then(|row| row.account_id.clone()),
                project_id: first.and_then(|row| row.project_id.clone()),
                title,
                folder: meta.folder.clone(),
                finished: finished.get(session_id.as_str()).copied().unwrap_or(true),
                adopted: adopted.contains(&session_id),
                marked_ended: meta.marked_ended,
                busy: None,
                ring,
                rows,
                tree,
                session_id,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, parent: u32) -> ProcessRow {
        ProcessRow {
            pid,
            parent_pid: Some(parent),
            name: format!("p{pid}"),
            ..ProcessRow::default()
        }
    }

    #[test]
    fn a_subtree_stops_at_another_sessions_root() {
        // App 1 → session 10 (→ 11 → 12) and session 20 (→ 21); 10 runs 30,
        // a session of its own.
        let rows = vec![
            row(1, 0),
            row(10, 1),
            row(20, 1),
            row(11, 10),
            row(21, 20),
            row(12, 11),
            row(30, 10),
            row(31, 30),
        ];
        let others = HashSet::from([20, 30]);
        let mut pids: Vec<u32> = subtree(&rows, 10, &others).iter().map(|l| l.pid).collect();
        pids.sort_unstable();
        assert_eq!(pids, [10, 11, 12]);
        assert!(subtree(&rows, 99, &others).is_empty());
    }
}
