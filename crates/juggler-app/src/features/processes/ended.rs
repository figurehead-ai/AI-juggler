//! AJ-E2401/E2405: what ran at one point (the Ended view, from the
//! ledger's ended rows and the other apps' sightings, 7 days) and the
//! snapshot to save (names and keyed hashes only).

use juggler_core::storage::{LedgerRow, Sighting};

use super::dto::{ProcRowInfo, ProcessGraphRowInfo};
use super::model::{Graph, Origin};
use super::roles::{program_name, role_of, short_name};

fn ended_row(
    key: String,
    origin: &str,
    owner_words: String,
    session_key: String,
    process: ProcRowInfo,
) -> ProcessGraphRowInfo {
    ProcessGraphRowInfo {
        key,
        kind: "process".into(),
        depth: 0,
        parent_key: None,
        has_children: false,
        state: "ended".into(),
        origin: origin.into(),
        owner_words,
        cpu_pct: None,
        rss_bytes: 0,
        ports: Vec::new(),
        app: None,
        session: None,
        process: Some(process),
        rollup: None,
        session_key: Some(session_key),
    }
}

fn process(
    pid: u32,
    start: i64,
    name: String,
    role: super::roles::Role,
    peak_rss: u64,
    age_ms: i64,
) -> ProcRowInfo {
    ProcRowInfo {
        pid,
        start_time: start,
        name,
        role: role.key().into(),
        role_words: role.words().into(),
        role_sure: role.sure(),
        label: None,
        subagent_id: None,
        subagent_title: None,
        spark: Vec::new(),
        peak_rss,
        age_ms,
        cpu_time_ms: 0,
        waits_for_pid: None,
        waits_for_name: None,
        stop: "never".into(),
        stop_reason: Some(super::model::never::ENDED.into()),
        mark: None,
        mark_until_ms: None,
        heavy: false,
        failed: None,
        internal: role.ours_internal(),
    }
}

/// An ended process AI Juggler started, from its ledger row. Its age is
/// how long it ran.
#[must_use]
pub fn ours_row(row: &LedgerRow, _now_ms: i64) -> ProcessGraphRowInfo {
    let pid = row.pid.unwrap_or_default();
    let start = row.started_at.unwrap_or_default();
    let role = role_of(&row.exe_path, &row.cmdline_trunc, row.role == "root");
    let ran = if start > 0 {
        (row.last_seen - start.saturating_mul(1000)).max(0)
    } else {
        0
    };
    ended_row(
        format!("e:{pid}:{start}"),
        "ours",
        "Started by AI Juggler".into(),
        format!("s:ours:{}", row.session_id),
        process(
            pid,
            start,
            short_name(program_name(&row.exe_path), Some(&row.cmdline_trunc)),
            role,
            row.peak_rss.map_or(0, i64::unsigned_abs),
            ran,
        ),
    )
}

/// An ended process of another app's session, from its sighting (its
/// program's name only).
#[must_use]
pub fn theirs_row(sighting: &Sighting, _now_ms: i64) -> ProcessGraphRowInfo {
    let role = role_of(&sighting.exe_name, "", sighting.session_root);
    let app = sighting.app.clone().unwrap_or_else(|| "Terminal".into());
    let ran = (sighting.last_seen - sighting.start_time.saturating_mul(1000)).max(0);
    ended_row(
        format!("e:{}:{}", sighting.pid, sighting.start_time),
        "theirs",
        super::query::owner_words(Origin::Theirs, &app),
        format!("s:theirs:{}:{}", sighting.tool, sighting.session_id),
        process(
            sighting.pid,
            sighting.start_time,
            sighting.exe_name.clone(),
            role,
            0,
            ran,
        ),
    )
}

/// The snapshot: apps by name, sessions by tool and whose, processes by
/// name, role, state, numbers and a keyed hash of the command line
/// (`hash_of(pid, start)`). No title, folder, command line or environment.
#[must_use]
pub fn snapshot_json(graph: &Graph, hash_of: &dyn Fn(u32, i64) -> Option<String>) -> String {
    let apps: Vec<serde_json::Value> = graph
        .apps
        .iter()
        .map(|app| {
            let sessions: Vec<serde_json::Value> = app
                .sessions
                .iter()
                .map(|session| {
                    let processes: Vec<serde_json::Value> = session
                        .processes
                        .iter()
                        .map(|node| {
                            serde_json::json!({
                                "pid": node.pid,
                                "name": node.name,
                                "role": node.role.key(),
                                "state": node.state.word(),
                                "depth": node.depth,
                                "cpu_pct": node.cpu_pct,
                                "rss_bytes": node.rss_bytes,
                                "peak_rss": node.peak_rss,
                                "ports": node.ports,
                                "age_ms": node.age_ms,
                                "cpu_time_ms": node.cpu_time_ms,
                                "command_hash": hash_of(node.pid, node.start_time),
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "tool": session.tool,
                        "whose": match session.origin {
                            Origin::Ours => "started by AI Juggler",
                            Origin::Theirs => "another app's session",
                        },
                        "state": session.state,
                        "processes": processes,
                    })
                })
                .collect();
            serde_json::json!({ "app": app.name, "sessions": sessions })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "taken_at_ms": graph.built_ms,
        "counts": {
            "processes": graph.counts.processes,
            "left_behind": graph.counts.left_behind,
            "zombies": graph.counts.zombies,
            "didnt_stop": graph.counts.didnt_stop,
        },
        "apps": apps,
        "note": "Names and hashed command lines only. A hash can't be turned back into the command line.",
    }))
    .unwrap_or_default()
}
