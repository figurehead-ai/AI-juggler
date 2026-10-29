//! AJ-E2401/E2404: the page's rows from the graph: a view (Needs
//! attention, Everything running), filters (all off by default), search
//! (`port:5173`), folding (a healthy session is one line), then one page by
//! cursor. Pure over the graph; ≤ 50 ms at 2,000 processes (measured in
//! `tests/processes_graph.rs`).

use std::collections::HashSet;

use super::dto::{
    AppRowInfo, ProcRowInfo, ProcessGraphRowInfo, ProcessQueryInput, ProcessSessionInfo, RollupInfo,
};
use super::model::{
    AppNode, Graph, Origin, OtherProgram, ProcNode, ProcState, Rollup, SessionNode, StopPath,
};

/// Rows in a page when none is asked for.
pub const DEFAULT_LIMIT: usize = 100;
/// The most rows a page holds.
pub const MAX_LIMIT: usize = 200;

/// The views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// What needs the user (the default).
    Attention,
    /// Everything running.
    Running,
}

impl View {
    /// From its word (anything unknown is Needs attention).
    #[must_use]
    pub fn parse(word: &str) -> Self {
        match word {
            "running" => Self::Running,
            _ => Self::Attention,
        }
    }
}

/// What a search asks for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Search {
    ports: Vec<u16>,
    pids: Vec<u32>,
    words: Vec<String>,
}

fn search_of(text: &str) -> Search {
    let mut search = Search::default();
    for token in text.split_whitespace() {
        let lower = token.to_lowercase();
        if let Some(port) = lower
            .strip_prefix("port:")
            .or_else(|| lower.strip_prefix(':'))
            && let Ok(port) = port.parse()
        {
            search.ports.push(port);
            continue;
        }
        if let Some(pid) = lower.strip_prefix("pid:")
            && let Ok(pid) = pid.parse()
        {
            search.pids.push(pid);
            continue;
        }
        search.words.push(lower);
    }
    search
}

impl Search {
    fn is_empty(&self) -> bool {
        self.ports.is_empty() && self.pids.is_empty() && self.words.is_empty()
    }

    fn matches(&self, app: &AppNode, session: &SessionNode, node: &ProcNode) -> bool {
        if !self.ports.iter().all(|port| node.ports.contains(port)) {
            return false;
        }
        if !self.pids.is_empty() && !self.pids.contains(&node.pid) {
            return false;
        }
        self.words.iter().all(|word| {
            let hay = [
                node.name.as_str(),
                node.role.words(),
                node.label.as_deref().unwrap_or(""),
                session.title.as_str(),
                app.name.as_str(),
                node.state.word(),
            ];
            hay.iter()
                .any(|text| text.to_lowercase().contains(word.as_str()))
        })
    }
}

/// Whose, in words.
#[must_use]
pub fn owner_words(origin: Origin, app: &str) -> String {
    match origin {
        Origin::Ours => "Started by AI Juggler".to_owned(),
        Origin::Theirs => format!("In a session {app} runs"),
    }
}

fn origin_word(origin: Origin) -> &'static str {
    match origin {
        Origin::Ours => "ours",
        Origin::Theirs => "theirs",
    }
}

fn rollup_info(rollup: &Rollup) -> RollupInfo {
    RollupInfo {
        processes: rollup.processes,
        rss_bytes: rollup.rss_bytes,
        cpu_pct: rollup.cpu_pct,
        ports: rollup.ports.iter().copied().take(8).collect(),
        attention: rollup.attention,
        to_clean: rollup.to_clean,
        worst: rollup.worst.map(|state| state.word().to_owned()),
    }
}

/// One process's row.
#[must_use]
pub fn process_row(
    app: &AppNode,
    session: &SessionNode,
    node: &ProcNode,
    has_children: bool,
) -> ProcessGraphRowInfo {
    let (stop, stop_reason) = match &node.stop {
        StopPath::Ours => ("ours", None),
        StopPath::Observed => ("observed", None),
        StopPath::Never(why) => ("never", Some((*why).to_owned())),
    };
    ProcessGraphRowInfo {
        key: node.key.clone(),
        kind: "process".into(),
        depth: node.depth.saturating_add(2),
        parent_key: Some(session.key.clone()),
        has_children,
        state: node.state.word().into(),
        origin: origin_word(node.origin).into(),
        owner_words: owner_words(node.origin, &app.name),
        cpu_pct: node.cpu_pct,
        rss_bytes: node.rss_bytes,
        ports: node.ports.clone(),
        app: None,
        session: None,
        process: Some(ProcRowInfo {
            pid: node.pid,
            start_time: node.start_time,
            name: node.name.clone(),
            role: node.role.key().into(),
            role_words: node.role.words().into(),
            role_sure: node.role.sure(),
            label: node.label.clone(),
            spark: node.spark.clone(),
            peak_rss: node.peak_rss,
            age_ms: node.age_ms,
            cpu_time_ms: node.cpu_time_ms,
            waits_for_pid: node.waits_for.as_ref().map(|(pid, _)| *pid),
            waits_for_name: node.waits_for.as_ref().map(|(_, name)| name.clone()),
            stop: stop.into(),
            stop_reason,
            mark: node.mark.map(|(kind, _)| kind.as_str().to_owned()),
            mark_until_ms: node.mark.and_then(|(_, until)| until),
            heavy: node.heavy,
            failed: node.failed.clone(),
            internal: node.role.ours_internal(),
        }),
        rollup: None,
        session_key: Some(session.key.clone()),
    }
}

fn session_row(
    app: &AppNode,
    session: &SessionNode,
    folded: bool,
    has_children: bool,
) -> ProcessGraphRowInfo {
    ProcessGraphRowInfo {
        key: session.key.clone(),
        kind: "session".into(),
        depth: 1,
        parent_key: Some(app.key.clone()),
        has_children,
        state: session.state.into(),
        origin: origin_word(session.origin).into(),
        owner_words: owner_words(session.origin, &app.name),
        cpu_pct: Some(session.rollup.cpu_pct),
        rss_bytes: session.rollup.rss_bytes,
        ports: session.rollup.ports.iter().copied().take(8).collect(),
        app: None,
        session: Some(ProcessSessionInfo {
            session_id: session.session_id.clone(),
            origin: origin_word(session.origin).into(),
            tool: session.tool.clone(),
            account_id: session.account_id.clone(),
            project_id: session.project_id.clone(),
            title: session.title.clone(),
            folder_name: session
                .folder
                .as_ref()
                .and_then(|folder| folder.file_name())
                .map(|name| name.to_string_lossy().into_owned()),
            state: session.state.into(),
            alive: session.alive,
            root_pid: session.root_pid,
            folded,
            shared_by: session.shared_by,
            started_ms: session
                .processes
                .iter()
                .find(|node| Some(node.pid) == session.root_pid)
                .or_else(|| session.processes.iter().min_by_key(|node| node.start_time))
                .map(|node| node.start_time)
                .filter(|start| *start > 0)
                .map(|start| start.saturating_mul(1000)),
        }),
        process: None,
        rollup: Some(rollup_info(&session.rollup)),
        session_key: Some(session.key.clone()),
    }
}

fn app_row(app: &AppNode, rollup: &Rollup, sessions: u32) -> ProcessGraphRowInfo {
    ProcessGraphRowInfo {
        key: app.key.clone(),
        kind: "app".into(),
        depth: 0,
        parent_key: None,
        // Listed only with a row under it (a session, or its shared
        // processes).
        has_children: true,
        state: rollup.worst.map_or("idle", ProcState::word).into(),
        origin: if app.ours { "ours" } else { "theirs" }.into(),
        owner_words: if app.ours {
            "Started by AI Juggler".to_owned()
        } else {
            format!("Sessions {} runs", app.name)
        },
        cpu_pct: Some(rollup.cpu_pct),
        rss_bytes: rollup.rss_bytes,
        ports: rollup.ports.iter().copied().take(8).collect(),
        app: Some(AppRowInfo {
            app_id: app.app_id.clone(),
            name: app.name.clone(),
            pid: app.pid,
            ours: app.ours,
            kind: app.kind.clone(),
            sessions,
        }),
        session: None,
        process: None,
        rollup: Some(rollup_info(rollup)),
        session_key: None,
    }
}

/// The words of the rest of this Mac's heading.
pub const OTHERS_NAME: &str = "Other apps on this computer";
/// Their owner words.
pub const OTHERS_OWNER: &str = "Not in an AI session";

fn other_app_row(
    key: &str,
    name: &str,
    kind: &str,
    depth: u8,
    rollup: RollupInfo,
) -> ProcessGraphRowInfo {
    ProcessGraphRowInfo {
        key: key.to_owned(),
        kind: if depth == 0 { "app" } else { "program" }.into(),
        depth,
        parent_key: (depth > 0).then(|| "a:other".to_owned()),
        has_children: depth == 0,
        state: "idle".into(),
        origin: "other".into(),
        owner_words: OTHERS_OWNER.into(),
        cpu_pct: None,
        rss_bytes: rollup.rss_bytes,
        ports: Vec::new(),
        app: Some(AppRowInfo {
            app_id: None,
            name: name.to_owned(),
            pid: None,
            ours: false,
            kind: Some(kind.to_owned()),
            sessions: 0,
        }),
        session: None,
        process: None,
        rollup: Some(rollup),
        session_key: None,
    }
}

/// The rest of this Mac, when asked for in Everything running: a heading,
/// then each program the search keeps (by name; a port or pid search
/// keeps none, as their ports and pids aren't read).
fn other_rows(others: &[OtherProgram], search: &Search) -> Vec<ProcessGraphRowInfo> {
    if !search.ports.is_empty() || !search.pids.is_empty() {
        return Vec::new();
    }
    let kept: Vec<&OtherProgram> = others
        .iter()
        .filter(|program| {
            let name = program.name.to_lowercase();
            search.words.iter().all(|word| name.contains(word.as_str()))
        })
        .collect();
    if kept.is_empty() {
        return Vec::new();
    }
    let total = |f: fn(&OtherProgram) -> u64| kept.iter().map(|program| f(program)).sum::<u64>();
    let mut out = vec![other_app_row(
        "a:other",
        OTHERS_NAME,
        "other",
        0,
        RollupInfo {
            processes: u32::try_from(total(|program| u64::from(program.processes)))
                .unwrap_or(u32::MAX),
            rss_bytes: total(|program| program.rss_bytes),
            ..RollupInfo::default()
        },
    )];
    out.extend(kept.iter().map(|program| {
        other_app_row(
            &format!("o:{}", program.name),
            &program.name,
            "program",
            1,
            RollupInfo {
                processes: program.processes,
                rss_bytes: program.rss_bytes,
                ..RollupInfo::default()
            },
        )
    }));
    out
}

/// Whether a session is the asked one: its id (the supervisor's for ours,
/// the tool's for theirs). The app's shared processes count for each
/// session that shares them (its Processes tab shows them too).
fn is_session(session: &SessionNode, id: &str) -> bool {
    session.session_id == id
        || session.key == id
        || session.sharers.iter().any(|sharer| sharer == id)
}

/// A process passes the filters (not the view).
fn passes(input: &ProcessQueryInput, node: &ProcNode, states: &HashSet<ProcState>) -> bool {
    (input.show_internal || !node.role.ours_internal())
        && (states.is_empty() || states.contains(&node.state))
        && (!input.has_port || !node.ports.is_empty())
        && (!input.heavy || node.heavy)
}

/// Every row the view shows, in tree order.
#[must_use]
#[allow(clippy::too_many_lines)] // the view's rules in one place, in order
pub fn rows(graph: &Graph, input: &ProcessQueryInput) -> Vec<ProcessGraphRowInfo> {
    let view = View::parse(&input.view);
    let search = search_of(&input.search);
    let states: HashSet<ProcState> = input
        .states
        .iter()
        .filter_map(|word| ProcState::parse(word))
        .collect();
    let expanded: HashSet<&str> = input.expanded.iter().map(String::as_str).collect();
    let collapsed: HashSet<&str> = input.collapsed.iter().map(String::as_str).collect();
    let one_session = input.session_id.as_deref();
    let narrowed = !search.is_empty() || !states.is_empty() || input.has_port || input.heavy;
    let mut out = Vec::new();
    for app in &graph.apps {
        if let Some(wanted) = input.app.as_deref()
            && app.key != wanted
            && !app.name.eq_ignore_ascii_case(wanted)
            && app.app_id.as_deref() != Some(wanted)
        {
            continue;
        }
        let mut app_rows: Vec<ProcessGraphRowInfo> = Vec::new();
        let mut rollup = Rollup::default();
        let mut sessions = 0;
        for session in &app.sessions {
            if one_session.is_some_and(|id| !is_session(session, id)) {
                continue;
            }
            if input
                .account_id
                .as_deref()
                .is_some_and(|account| session.account_id.as_deref() != Some(account))
            {
                continue;
            }
            if input
                .project_id
                .as_deref()
                .is_some_and(|project| session.project_id.as_deref() != Some(project))
            {
                continue;
            }
            // The processes this view and the filters keep.
            let keep: Vec<&ProcNode> = session
                .processes
                .iter()
                .filter(|node| passes(input, node, &states))
                .filter(|node| search.is_empty() || search.matches(app, session, node))
                .filter(|node| one_session.is_some() || view == View::Running || node.attention)
                .collect();
            // A live session whose processes are all its app's (a desktop
            // app's session) still has its line in Everything running.
            let bare = session.alive && !session.is_shared() && session.processes.is_empty();
            if keep.is_empty()
                && one_session.is_none()
                && (narrowed || view == View::Attention || !bare)
            {
                continue;
            }
            // With their ancestors, so a subprocess shows under its parent.
            let mut shown: HashSet<&str> = keep.iter().map(|node| node.key.as_str()).collect();
            if view == View::Attention || narrowed {
                let by_pid: std::collections::HashMap<u32, &ProcNode> = session
                    .processes
                    .iter()
                    .map(|node| (node.pid, node))
                    .collect();
                for node in &keep {
                    let mut parent = node.parent_pid;
                    let mut hops = 0;
                    while let Some(pid) = parent {
                        hops += 1;
                        let Some(up) = by_pid.get(&pid) else { break };
                        if hops > 64 || !shown.insert(up.key.as_str()) {
                            break;
                        }
                        parent = up.parent_pid;
                    }
                }
            } else {
                shown = session
                    .processes
                    .iter()
                    .filter(|node| passes(input, node, &states))
                    .map(|node| node.key.as_str())
                    .collect();
            }
            let healthy = session.rollup.attention == 0;
            let folded =
                if one_session.is_some() || narrowed || expanded.contains(session.key.as_str()) {
                    false
                } else if collapsed.contains(session.key.as_str()) {
                    true
                } else {
                    healthy && view == View::Running
                };
            let listed: Vec<&ProcNode> = session
                .processes
                .iter()
                .filter(|node| shown.contains(node.key.as_str()))
                .collect();
            sessions += u32::from(!session.is_shared());
            rollup.processes += session.rollup.processes;
            rollup.rss_bytes += session.rollup.rss_bytes;
            rollup.cpu_pct += session.rollup.cpu_pct;
            rollup.ports.extend(session.rollup.ports.iter().copied());
            rollup.attention += session.rollup.attention;
            rollup.to_clean += session.rollup.to_clean;
            rollup.worst = match (rollup.worst, session.rollup.worst) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
            app_rows.push(session_row(app, session, folded, !listed.is_empty()));
            if folded {
                continue;
            }
            for (index, node) in listed.iter().enumerate() {
                let has_children = listed
                    .get(index + 1)
                    .is_some_and(|next| next.depth > node.depth);
                app_rows.push(process_row(app, session, node, has_children));
            }
        }
        if app_rows.is_empty() {
            continue;
        }
        out.push(app_row(app, &rollup, sessions));
        out.extend(app_rows);
    }
    let plain = input.states.is_empty() && !input.has_port && !input.heavy;
    if input.others
        && view == View::Running
        && one_session.is_none()
        && plain
        && input.account_id.is_none()
        && input.project_id.is_none()
        && input.app.as_deref().is_none_or(|app| app == "a:other")
    {
        out.extend(other_rows(&graph.others, &search));
    }
    out
}

/// A cursor: the next page starts after the row with this key (or, when
/// the row has gone meanwhile, at this offset).
#[must_use]
pub fn cursor_of(offset: usize, key: &str) -> String {
    format!("{offset}:{key}")
}

/// One page of `rows` after `cursor`.
#[must_use]
pub fn page(
    rows: Vec<ProcessGraphRowInfo>,
    cursor: Option<&str>,
    limit: Option<u32>,
) -> (Vec<ProcessGraphRowInfo>, Option<String>, u64) {
    let limit = limit
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);
    let total = rows.len() as u64;
    let start = cursor.map_or(0, |cursor| {
        let (offset, key) = cursor.split_once(':').unwrap_or(("0", cursor));
        rows.iter()
            .position(|row| row.key == key)
            .map_or_else(|| offset.parse::<usize>().unwrap_or(0), |at| at + 1)
    });
    let end = (start + limit).min(rows.len());
    let next = (end < rows.len()).then(|| cursor_of(end, &rows[end - 1].key));
    let items = rows
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();
    (items, next, total)
}
