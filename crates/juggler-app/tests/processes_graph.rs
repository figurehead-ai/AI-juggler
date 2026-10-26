//! AJ-E2401/E2404: the Processes view's graph and page, from fixture
//! inputs (no OS, no store): every state in the one vocabulary, whose each
//! process is, the totals, what Needs attention shows, folding, search,
//! paging — and the budget on a large Mac (50 sessions, 2,000 processes).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::time::Instant;

use juggler_app::features::processes::dto::ProcessQueryInput;
use juggler_app::features::processes::model::{
    self, AppRef, Inputs, LeftoverInput, Live, OursInput, ProcState, StopPath, TheirsInput,
};
use juggler_app::features::processes::query;
use juggler_core::storage::{LedgerRow, MarkKind, Sighting};

const NOW: i64 = 1_800_000_000_000;
const OWN: u32 = 7;

fn row(
    session: &str,
    pid: u32,
    parent: Option<u32>,
    role: &str,
    state: &str,
    cmd: &str,
) -> LedgerRow {
    LedgerRow {
        pid: Some(pid),
        pgid: Some(pid),
        started_at: Some(i64::from(pid) * 10),
        exe_path: format!(
            "/usr/local/bin/{}",
            cmd.split_whitespace().next().unwrap_or("x")
        ),
        cmdline_hash: "h".into(),
        cmdline_trunc: cmd.into(),
        boot_id: "b".into(),
        session_id: session.into(),
        instance_id: None,
        provider: "tool-a".into(),
        account_id: Some("acct-work".into()),
        project_id: Some("web-app".into()),
        machine_id: "m".into(),
        root_pid: parent.map(|_| 100),
        parent_pid: parent,
        depth: u32::from(parent.is_some()),
        role: role.into(),
        state: state.into(),
        listens_on: None,
        cwd: "/x".into(),
        peak_rss: Some(1_000),
        peak_cpu: None,
        exit_kind: None,
        reaped_by_us: None,
        escaped_group: false,
        down_period: false,
        first_seen: 0,
        last_seen: 0,
    }
}

fn live(pid: u32, parent: Option<u32>, name: &str, cpu: f64, rss: u64) -> Live {
    Live {
        pid,
        start_time: i64::from(pid) * 10,
        parent_pid: parent,
        name: name.into(),
        cpu_pct: Some(cpu),
        rss_bytes: rss,
        cpu_time_ms: 1_000,
        zombie: false,
    }
}

/// A Mac with: an AI Juggler session running healthily (claude → npm →
/// vite :5173, an MCP server, a hook), one whose own process ended and left
/// a dev server and a watcher behind, a session picked up after restart,
/// one left while AI Juggler was closed, a zombie, a failed stop, one that
/// won't be touched; another app's live session, and another app's ended
/// session's leftover.
fn fixture() -> Inputs {
    let mut inputs = Inputs {
        now_ms: NOW,
        own_pid: OWN,
        ..Inputs::default()
    };
    // Healthy: 100 claude → 101 npm → 102 node vite; 103 mcp; 104 hook.
    let healthy_rows = vec![
        row("s-ok", 100, None, "root", "running", "claude"),
        row("s-ok", 101, Some(100), "helper", "running", "npm run dev"),
        row(
            "s-ok",
            102,
            Some(101),
            "helper",
            "running",
            "node node_modules/.bin/vite dev",
        ),
        row(
            "s-ok",
            103,
            Some(100),
            "helper",
            "running",
            "node mcp-server-github",
        ),
        row(
            "s-ok",
            104,
            Some(100),
            "helper",
            "running",
            "juggler-agent hook PostToolUse",
        ),
    ];
    for r in &healthy_rows {
        let pid = r.pid.unwrap();
        inputs
            .live
            .insert(pid, live(pid, r.parent_pid, "node", 0.5, 50_000_000));
    }
    inputs.live.get_mut(&100).unwrap().cpu_pct = Some(12.0);
    inputs.ports.insert(102, vec![5173]);
    // What started a process, from the tool's own events (a Codex command
    // item when it starts, a Claude hook): matched to its command line.
    inputs.labels.insert(
        "s-ok".into(),
        vec![(1_000_000, "a command: npm run dev".into())],
    );
    inputs.ours.push(OursInput {
        session_id: "s-ok".into(),
        tool: "tool-a".into(),
        account_id: Some("acct-work".into()),
        project_id: Some("web-app".into()),
        title: "Refactor billing".into(),
        rows: healthy_rows,
        ring: vec![1.0, 2.0, 3.0],
        ..OursInput::default()
    });
    // Ended, left behind: 200 (gone) → 201 vite :3000, 202 nodemon.
    let mut left = vec![
        row(
            "s-left",
            201,
            Some(200),
            "helper",
            "orphaned",
            "node vite --port 3000",
        ),
        row(
            "s-left",
            202,
            Some(200),
            "helper",
            "orphaned",
            "nodemon server.js",
        ),
    ];
    left[0].listens_on = Some("3000".into());
    for r in &left {
        let pid = r.pid.unwrap();
        inputs
            .live
            .insert(pid, live(pid, Some(1), "node", 0.0, 80_000_000));
    }
    inputs.ours.push(OursInput {
        session_id: "s-left".into(),
        tool: "tool-a".into(),
        account_id: Some("acct-work".into()),
        title: "Payments API".into(),
        finished: true,
        marked_ended: true,
        rows: left,
        ..OursInput::default()
    });
    // Picked up after restart: 300 root, 301 next dev.
    let picked = vec![
        row("s-pick", 300, None, "root", "running", "claude --resume"),
        row(
            "s-pick",
            301,
            Some(300),
            "helper",
            "running",
            "next dev -p 8080",
        ),
    ];
    for r in &picked {
        let pid = r.pid.unwrap();
        inputs
            .live
            .insert(pid, live(pid, r.parent_pid, "node", 0.2, 10_000_000));
    }
    inputs.ours.push(OursInput {
        session_id: "s-pick".into(),
        tool: "tool-a".into(),
        title: "Docs".into(),
        adopted: true,
        rows: picked,
        ..OursInput::default()
    });
    // While closed (down period), a zombie, a failed stop, won't touch.
    let mut odd = vec![
        row("s-odd", 400, None, "root", "running", "codex"),
        row(
            "s-odd",
            401,
            Some(400),
            "helper",
            "orphaned",
            "jest --watchAll",
        ),
        row(
            "s-odd",
            402,
            Some(400),
            "helper",
            "running",
            "node worker.js",
        ),
        row(
            "s-odd",
            403,
            Some(400),
            "helper",
            "running",
            "python3 -m http.server 8000",
        ),
        row(
            "s-odd",
            404,
            Some(400),
            "helper",
            "unverifiable",
            "postgres -D /db",
        ),
    ];
    odd[1].down_period = true;
    for r in &odd {
        let pid = r.pid.unwrap();
        inputs
            .live
            .insert(pid, live(pid, r.parent_pid, "node", 0.1, 20_000_000));
    }
    inputs.live.get_mut(&402).unwrap().zombie = true;
    inputs.zombie_since.insert((402, 4020), NOW - 120_000);
    inputs.failed_stops.insert(403, "pid 403: refused".into());
    inputs.ours.push(OursInput {
        session_id: "s-odd".into(),
        tool: "tool-b".into(),
        title: "Odd one".into(),
        rows: odd,
        ..OursInput::default()
    });
    // Another app's live session: 500 → 501 tsc --watch, 502 a hook.
    inputs.theirs.push(TheirsInput {
        tool: "tool-a".into(),
        session_id: "conv-live".into(),
        account_id: Some("acct-personal".into()),
        title: "Blog images".into(),
        app: AppRef {
            app_id: Some("an-app".into()),
            name: "An App".into(),
            pid: Some(50),
            kind: Some("desktop".into()),
        },
        root_pid: 500,
        root_start: 5000,
        tree: vec![
            live(500, Some(50), "tool", 4.0, 300_000_000),
            live(501, Some(500), "node", 0.3, 90_000_000),
        ],
        ..TheirsInput::default()
    });
    inputs.commands.insert(
        501,
        ("/usr/local/bin/node".into(), "node tsc --watch".into()),
    );
    // Another app's ended session, its leftover: 601 vite :5174.
    inputs.leftovers.push(LeftoverInput {
        sighting: Sighting {
            pid: 601,
            start_time: 6010,
            boot_id: "b".into(),
            exe_name: "node".into(),
            cmd_hash: "x".into(),
            tool: "tool-a".into(),
            session_id: "conv-ended".into(),
            app: Some("An App".into()),
            session_root: false,
            parent_pid: Some(1),
            first_seen: 0,
            last_seen: NOW,
        },
        live: live(601, Some(1), "node", 0.0, 120_000_000),
    });
    inputs.commands.insert(
        601,
        ("/usr/local/bin/node".into(), "node vite --port 5174".into()),
    );
    inputs.ports.insert(601, vec![5174]);
    inputs.titles.insert(
        ("tool-a".into(), "conv-ended".into()),
        "Landing page".into(),
    );
    inputs
}

fn node(graph: &model::Graph, pid: u32) -> &model::ProcNode {
    graph
        .apps
        .iter()
        .flat_map(|app| app.sessions.iter())
        .flat_map(|session| session.processes.iter())
        .find(|node| node.pid == pid)
        .unwrap_or_else(|| panic!("pid {pid} in the graph"))
}

#[test]
fn every_state_is_in_the_one_vocabulary() {
    let graph = model::build(&fixture());
    let expect = [
        (100, ProcState::Working),
        (101, ProcState::Idle),
        (201, ProcState::LeftBehind),
        (202, ProcState::LeftBehind),
        (301, ProcState::PickedUp),
        (401, ProcState::LeftBehindClosed),
        (402, ProcState::Zombie),
        (403, ProcState::DidntStop),
        (404, ProcState::WontTouch),
        (501, ProcState::Idle),
        (601, ProcState::LeftBehind),
    ];
    for (pid, state) in expect {
        assert_eq!(node(&graph, pid).state, state, "pid {pid}");
    }
    // The zombie names the parent only it can wait for, and offers no stop.
    let zombie = node(&graph, 402);
    assert_eq!(zombie.waits_for.as_ref().map(|(pid, _)| *pid), Some(400));
    assert!(matches!(zombie.stop, StopPath::Never(_)));
    assert!(
        zombie.attention,
        "a zombie that waited a minute needs the user"
    );
    // AI Juggler's own pid never shows.
    assert!(graph.apps.iter().all(|app| {
        app.sessions
            .iter()
            .all(|s| s.processes.iter().all(|n| n.pid != OWN))
    }));
}

#[test]
fn whose_and_how_each_may_be_stopped() {
    let graph = model::build(&fixture());
    assert_eq!(
        graph.apps[0].name, "AI Juggler",
        "AI Juggler's own sessions first"
    );
    assert_eq!(node(&graph, 201).stop, StopPath::Ours);
    // A live session's own process: stopped from its page, never here
    // (0.1.5 took it out of Cleanup); its helpers stay stoppable.
    assert_eq!(
        node(&graph, 100).stop,
        StopPath::Never(model::never::OUR_SESSION)
    );
    assert_eq!(node(&graph, 102).stop, StopPath::Ours);
    assert!(
        matches!(node(&graph, 404).stop, StopPath::Never(_)),
        "won't be touched"
    );
    // Another app's: never while its session runs; its leftover by D-41b.
    assert!(matches!(node(&graph, 500).stop, StopPath::Never(_)));
    assert!(matches!(node(&graph, 501).stop, StopPath::Never(_)));
    assert_eq!(node(&graph, 601).stop, StopPath::Observed);
    // The hook is AI Juggler's: hidden, never counted, never offered.
    let hook = node(&graph, 104);
    assert!(hook.role.ours_internal());
    assert!(matches!(hook.stop, StopPath::Never(_)));
    // Roles in words.
    assert_eq!(node(&graph, 102).role.words(), "dev server");
    assert_eq!(node(&graph, 103).role.words(), "MCP server");
    assert_eq!(node(&graph, 202).role.words(), "watcher");
    assert_eq!(node(&graph, 100).role.words(), "the session itself");
    assert_eq!(node(&graph, 102).name, "node (vite)");
    assert_eq!(
        node(&graph, 101).label.as_deref(),
        Some("Started by a command: npm run dev")
    );
    assert_eq!(node(&graph, 102).label, None, "not its command line");
}

#[test]
fn totals_roll_up_and_the_top_bar_counts_what_to_clean() {
    let graph = model::build(&fixture());
    // Left behind: 201, 202 (ours), 401 (while closed), 601 (theirs);
    // didn't stop: 403. The hook isn't counted anywhere.
    assert_eq!(graph.counts.left_behind, 4);
    assert_eq!(graph.counts.didnt_stop, 1);
    assert_eq!(graph.counts.zombies, 1);
    assert_eq!(graph.counts.wont_touch, 1);
    assert_eq!(graph.counts.to_clean, 5);
    let healthy = graph.apps[0]
        .sessions
        .iter()
        .find(|s| s.session_id == "s-ok")
        .unwrap();
    assert_eq!(healthy.rollup.processes, 4, "the hook is AI Juggler's own");
    assert_eq!(
        healthy.rollup.ports.iter().copied().collect::<Vec<_>>(),
        [5173]
    );
    assert_eq!(healthy.rollup.attention, 0);
    assert_eq!(healthy.state, "working");
    // Keep and Remind me later take a process out of the count.
    let mut inputs = fixture();
    inputs.marks.insert((201, 2010), (MarkKind::Keep, None));
    inputs
        .marks
        .insert((202, 2020), (MarkKind::Snooze, Some(NOW + 3_600_000)));
    let marked = model::build(&inputs);
    assert_eq!(marked.counts.to_clean, 3);
    assert!(!node(&marked, 201).attention && !node(&marked, 202).attention);
    // A reminder that has passed counts again.
    inputs
        .marks
        .insert((202, 2020), (MarkKind::Snooze, Some(NOW - 1)));
    assert_eq!(model::build(&inputs).counts.to_clean, 4);
}

fn input(view: &str) -> ProcessQueryInput {
    ProcessQueryInput {
        view: view.into(),
        ..ProcessQueryInput::default()
    }
}

#[test]
fn needs_attention_shows_what_needs_the_user_under_its_parents() {
    let graph = model::build(&fixture());
    let rows = query::rows(&graph, &input("attention"));
    let pids: Vec<u32> = rows
        .iter()
        .filter_map(|row| row.process.as_ref().map(|p| p.pid))
        .collect();
    for pid in [201, 202, 401, 402, 403, 601] {
        assert!(pids.contains(&pid), "pid {pid} needs the user: {pids:?}");
    }
    // The zombie's and the failed stop's parent shows for context.
    assert!(pids.contains(&400));
    // Healthy sessions don't show at all.
    assert!(
        rows.iter()
            .all(|row| row.session.as_ref().is_none_or(|s| s.session_id != "s-ok"))
    );
    // Theirs says whose: "In a session An App runs".
    let theirs = rows
        .iter()
        .find(|row| row.process.as_ref().is_some_and(|p| p.pid == 601))
        .unwrap();
    assert_eq!(theirs.owner_words, "In a session An App runs");
    assert_eq!(theirs.process.as_ref().unwrap().stop, "observed");
}

/// Processes lab 2026-10-26, bug 1: a ledger from before one process kept
/// one row could hold a left-behind row and a running one for the same
/// process. It is listed once, left behind, in every view.
#[test]
fn a_process_with_two_ledger_rows_is_listed_once_as_left_behind() {
    let mut inputs = fixture();
    let left = inputs
        .ours
        .iter_mut()
        .find(|session| session.session_id == "s-left")
        .unwrap();
    let mut again = left.rows[0].clone();
    again.state = "running".into();
    left.rows.push(again);
    let graph = model::build(&inputs);
    let nodes: Vec<_> = graph
        .apps
        .iter()
        .flat_map(|app| app.sessions.iter())
        .flat_map(|session| session.processes.iter())
        .filter(|node| node.pid == 201)
        .collect();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].state, ProcState::LeftBehind);
    for view in ["attention", "running"] {
        let rows = query::rows(&graph, &input(view));
        assert_eq!(
            rows.iter()
                .filter(|row| row.process.as_ref().is_some_and(|p| p.pid == 201))
                .count(),
            1,
            "once in each view"
        );
    }
}

#[test]
fn everything_running_folds_healthy_sessions_to_one_line() {
    let graph = model::build(&fixture());
    let rows = query::rows(&graph, &input("running"));
    let healthy = rows
        .iter()
        .find(|row| row.session.as_ref().is_some_and(|s| s.session_id == "s-ok"))
        .unwrap();
    assert!(healthy.session.as_ref().unwrap().folded);
    let rollup = healthy.rollup.as_ref().unwrap();
    assert_eq!(rollup.processes, 4);
    assert!(
        !rows
            .iter()
            .any(|row| row.process.as_ref().is_some_and(|p| p.pid == 102))
    );
    // Unfolded on request, with the tree's depth.
    let mut open = input("running");
    open.expanded = vec!["s:ours:s-ok".into()];
    let rows = query::rows(&graph, &open);
    let vite = rows
        .iter()
        .find(|row| row.process.as_ref().is_some_and(|p| p.pid == 102))
        .unwrap();
    assert_eq!(vite.depth, 4, "app 0, session 1, claude 2, npm 3, vite 4");
    // The hook only when asked.
    assert!(
        !rows
            .iter()
            .any(|row| row.process.as_ref().is_some_and(|p| p.pid == 104))
    );
    open.show_internal = true;
    assert!(
        query::rows(&graph, &open)
            .iter()
            .any(|row| row.process.as_ref().is_some_and(|p| p.pid == 104))
    );
}

#[test]
fn search_filters_and_one_session_narrow_the_rows() {
    let graph = model::build(&fixture());
    let mut search = input("running");
    search.search = "port:5173".into();
    let rows = query::rows(&graph, &search);
    let pids: Vec<u32> = rows
        .iter()
        .filter_map(|row| row.process.as_ref().map(|p| p.pid))
        .collect();
    assert!(pids.contains(&102), "{pids:?}");
    assert!(!pids.contains(&201));
    let mut words = input("running");
    words.search = "watcher".into();
    let pids: Vec<u32> = query::rows(&graph, &words)
        .iter()
        .filter_map(|row| row.process.as_ref().map(|p| p.pid))
        .collect();
    // nodemon, jest --watchAll and tsc --watch, each under its parents.
    assert_eq!(pids, [400, 401, 202, 500, 501]);
    // A session's Processes tab: that session alone, unfolded, any app's.
    let mut tab = input("running");
    tab.session_id = Some("conv-live".into());
    let rows = query::rows(&graph, &tab);
    let pids: Vec<u32> = rows
        .iter()
        .filter_map(|row| row.process.as_ref().map(|p| p.pid))
        .collect();
    assert_eq!(pids, [500, 501]);
    // Filters: has a port; a state.
    let mut ports = input("running");
    ports.has_port = true;
    let pids: Vec<u32> = query::rows(&graph, &ports)
        .iter()
        .filter_map(|row| row.process.as_ref().map(|p| p.pid))
        .collect();
    assert!(pids.contains(&102) && pids.contains(&201) && pids.contains(&601));
    assert!(!pids.contains(&202));
    let mut zombies = input("running");
    zombies.states = vec!["zombie".into()];
    let pids: Vec<u32> = query::rows(&graph, &zombies)
        .iter()
        .filter_map(|row| row.process.as_ref().map(|p| p.pid))
        .filter(|pid| *pid != 400)
        .collect();
    assert_eq!(pids, [402]);
}

/// AJ-E2405 Req.6: the snapshot holds names, states, numbers and keyed
/// hashes, never a session's title or folder, or a command line.
#[test]
fn the_snapshot_holds_names_and_hashes_and_never_a_title_folder_or_line() {
    let inputs = fixture();
    let graph = model::build(&inputs);
    let json = juggler_app::features::processes::ended::snapshot_json(&graph, &|pid, _| {
        Some(format!("hash-{pid}"))
    });
    assert!(json.contains("\"command_hash\": \"hash-102\""), "{json}");
    assert!(json.contains("\"name\": \"node (vite)\""), "{json}");
    for session in graph.apps.iter().flat_map(|app| app.sessions.iter()) {
        if !session.title.trim().is_empty() {
            assert!(
                !json.contains(&session.title),
                "title {:?} leaked",
                session.title
            );
        }
        if let Some(folder) = &session.folder {
            assert!(!json.contains(&*folder.to_string_lossy()), "folder leaked");
        }
    }
    for (_, cmdline) in inputs.commands.values() {
        if cmdline.contains(' ') {
            assert!(
                !json.contains(cmdline.as_str()),
                "command line {cmdline:?} leaked"
            );
        }
    }
    for line in [
        "node node_modules/.bin/vite dev",
        "npm run dev",
        "juggler-agent hook",
    ] {
        assert!(!json.contains(line), "{line:?} leaked");
    }
}

/// "Include other apps": the rest of this Mac, by program, in Everything
/// running only when asked for: names and memory, no pid, nothing to stop.
#[test]
fn the_rest_of_the_mac_shows_only_when_asked_for_and_only_names_and_memory() {
    let mut inputs = fixture();
    inputs.others = vec![
        model::OtherProgram {
            name: "Safari".into(),
            processes: 14,
            rss_bytes: 2 << 30,
        },
        model::OtherProgram {
            name: "Finder".into(),
            processes: 1,
            rss_bytes: 90 << 20,
        },
    ];
    let graph = model::build(&inputs);
    // Off by default, and never in Needs attention.
    assert!(
        query::rows(&graph, &input("running"))
            .iter()
            .all(|row| row.origin != "other")
    );
    let mut asked = input("attention");
    asked.others = true;
    assert!(
        query::rows(&graph, &asked)
            .iter()
            .all(|row| row.origin != "other")
    );
    // Asked for in Everything running: a heading, then most memory first.
    asked.view = "running".into();
    let rows = query::rows(&graph, &asked);
    let others: Vec<_> = rows.iter().filter(|row| row.origin == "other").collect();
    assert_eq!(others.len(), 3);
    assert_eq!(others[0].key, "a:other");
    assert_eq!(others[0].kind, "app");
    assert_eq!(others[0].rollup.as_ref().unwrap().processes, 15);
    assert_eq!(others[1].app.as_ref().unwrap().name, "Safari");
    assert_eq!(others[2].app.as_ref().unwrap().name, "Finder");
    for row in &others[1..] {
        assert_eq!(row.kind, "program");
        assert!(row.process.is_none() && row.session.is_none());
        assert!(row.app.as_ref().unwrap().pid.is_none());
        assert!(row.ports.is_empty() && row.cpu_pct.is_none());
    }
    // Search by name keeps a program; a port or pid search keeps none.
    asked.search = "safari".into();
    let found: Vec<_> = query::rows(&graph, &asked)
        .into_iter()
        .filter(|row| row.origin == "other")
        .collect();
    assert_eq!(found.len(), 2);
    asked.search = "port:5173".into();
    assert!(
        query::rows(&graph, &asked)
            .iter()
            .all(|row| row.origin != "other")
    );
    // A state filter or one session's tab leaves them out.
    asked.search.clear();
    asked.states = vec!["left_behind".into()];
    assert!(
        query::rows(&graph, &asked)
            .iter()
            .all(|row| row.origin != "other")
    );
}

#[test]
fn pages_follow_a_cursor_without_repeats_or_gaps() {
    let graph = model::build(&large(50, 40));
    let mut all = input("running");
    all.expanded = (0..50).map(|n| format!("s:ours:s{n}")).collect();
    let rows = query::rows(&graph, &all);
    let total = rows.len();
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let (items, next, count) = query::page(rows.clone(), cursor.as_deref(), Some(60));
        assert_eq!(count as usize, total);
        assert!(items.len() <= 60);
        seen.extend(items.into_iter().map(|row| row.key));
        match next {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    let keys: Vec<String> = rows.into_iter().map(|row| row.key).collect();
    assert_eq!(seen, keys);
}

/// `sessions` AI Juggler sessions of `per` processes each, a quarter of
/// them ended and left behind.
fn large(sessions: u32, per: u32) -> Inputs {
    let mut inputs = Inputs {
        now_ms: NOW,
        own_pid: OWN,
        ..Inputs::default()
    };
    for s in 0..sessions {
        let base = 10_000 + s * 1_000;
        let ended = s % 4 == 0;
        let mut rows = Vec::new();
        for p in 0..per {
            let pid = base + p;
            let parent = (p > 0).then(|| base + (p - 1) / 3);
            let state = if ended && p > 0 {
                "orphaned"
            } else {
                "running"
            };
            let cmd = match p % 5 {
                0 => "claude",
                1 => "node vite dev",
                2 => "node tsc --watch",
                3 => "node mcp-server-x",
                _ => "python3 worker.py",
            };
            rows.push(row(
                &format!("s{s}"),
                pid,
                parent,
                if p == 0 { "root" } else { "helper" },
                state,
                cmd,
            ));
            inputs
                .live
                .insert(pid, live(pid, parent, "node", f64::from(p % 7), 30_000_000));
            inputs
                .sparks
                .insert((pid, i64::from(pid) * 10), vec![1.0; 30]);
            if p % 9 == 1 {
                inputs
                    .ports
                    .insert(pid, vec![u16::try_from(3000 + p).unwrap()]);
            }
        }
        if ended {
            rows.retain(|r| r.role != "root");
        }
        inputs.ours.push(OursInput {
            session_id: format!("s{s}"),
            tool: "tool-a".into(),
            title: format!("Session {s}"),
            finished: ended,
            rows,
            ring: vec![1.0; 150],
            ..OursInput::default()
        });
    }
    inputs
}

/// The budget (ADR §perf): at 50 sessions and 2,000 processes, building
/// the graph and answering a page each take well under 50 ms. Measured in
/// the test's own build (debug here; `--release` is the shipped figure);
/// the debug bound is the budget × 10.
#[test]
fn the_graph_and_a_page_fit_the_budget_at_two_thousand_processes() {
    let inputs = large(50, 40);
    let started = Instant::now();
    let mut graph = model::build(&inputs);
    for _ in 0..9 {
        graph = model::build(&inputs);
    }
    let build_ms = started.elapsed().as_secs_f64() * 100.0;
    assert_eq!(
        graph.counts.processes,
        2_000 - 13,
        "50 × 40, the 13 ended sessions' roots gone"
    );
    let mut queries: HashMap<&str, f64> = HashMap::new();
    for view in ["attention", "running"] {
        let mut ask = input(view);
        ask.expanded = (0..50).map(|n| format!("s:ours:s{n}")).collect();
        let started = Instant::now();
        for _ in 0..10 {
            let rows = query::rows(&graph, &ask);
            let _ = query::page(rows, None, Some(60));
        }
        queries.insert(view, started.elapsed().as_secs_f64() * 100.0);
    }
    let mut search = input("running");
    search.search = "port:3010 vite".into();
    let started = Instant::now();
    let _ = query::rows(&graph, &search);
    let search_ms = started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "processes budget: build {build_ms:.2} ms, attention page {:.2} ms, running page {:.2} ms, search {search_ms:.2} ms",
        queries["attention"], queries["running"]
    );
    let bound = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
    assert!(build_ms < bound, "build {build_ms} ms");
    assert!(queries.values().all(|ms| *ms < bound), "{queries:?}");
}
