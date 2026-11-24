//! AJ-E2318 (the review's A1.1–A1.5, A3.6, A9.2, B0.2, B0.3 and B2): what
//! a move waits on, decided before anything quits, and both profiles
//! quit at once. Moved out of `moving.rs` for its size.
//!
//! **The wait set** is the main process of each profile the move closes,
//! plus the processes the tool's adapter names as **writers** of the
//! moving family ([`InPlaceMove::writers`]: Claude Code's pid files and
//! `--resume`), asked of both accounts' folders whatever the place.
//! Nothing else is waited on. Everything else the profile ran (a dev
//! server its session started, `caffeinate`, a wrapper, Electron's
//! helpers and crash reporter) is reported in plain words, never waited
//! on: before, the move waited for the whole tree it saw before the quit,
//! and one background server refused every Move, on every retry, after
//! both apps had been quit.
//!
//! **Decide before disrupting.** A writer that no profile the move closes
//! runs (a terminal session, another app or profile) can't end by the
//! move's quit: the offer and the move refuse it up front, naming it, with
//! the self rule, before anything quits.
//!
//! **Quitting.** Every profile's main process is asked at once, then the
//! union is waited on, each class to its own deadline (the apps
//! [`MoveSystem::quit_wait`], then their sessions
//! [`MoveSystem::writer_wait`]). Each signal goes through the process's
//! [`ProcId`], checked right before it ([`signal::quit_checked`]).
//!
//! Files that keep changing with no writer named are caught by the
//! stillness check before the backup (`hold_still`) and the open-file
//! guard (`still::open_writers`), the backups for writers no hook names.
//! AJ-E2324: both waits are woken by the exits ([`MoveSystem::wait_exit`],
//! the kernel's on the real system), never polled.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use juggler_adapter::moving::{InPlaceMove, MoveRequest, ProcView, Writer};

use super::{
    KnownPlace, Member, MoveProgressInfo, MoveSystem, StoppedPlace, place_name, start_again, tree,
};
use crate::error::{AppResult, MoveHolder, MoveStop, StopClass};
use crate::features::conversations::procs::Table;
use crate::features::ownership::ProcId;
use crate::features::ownership::signal::{self, NotAsked};
use crate::state::AppState;

/// What the writers hook is asked: the tool's mover, both accounts'
/// folders, and the moving family's conversations.
pub(super) struct Writing<'a> {
    pub mover: &'a dyn InPlaceMove,
    pub dirs: Vec<PathBuf>,
    pub conversations: Vec<String>,
}

/// The writers of the family in `table`, asked of every folder, each
/// process once.
pub(super) fn writers_now(table: &Table, writing: &Writing<'_>) -> Vec<Writer> {
    let view: Vec<ProcView<'_>> = table
        .values()
        .map(|process| ProcView {
            pid: process.pid,
            ppid: process.ppid,
            exe: process.exe.as_deref(),
            args: &process.args,
            start_secs: process.start,
        })
        .collect();
    let mut out: Vec<Writer> = Vec::new();
    let mut dirs = writing.dirs.clone();
    dirs.dedup();
    for dir in &dirs {
        for writer in writing.mover.writers(dir, &writing.conversations, &view) {
            if !out.iter().any(|known| known.pid == writer.pid) {
                out.push(writer);
            }
        }
    }
    out.sort_by_key(|writer| writer.pid);
    out
}

/// A writer as words name it: its program and pid.
fn member_of(table: &Table, writer: &Writer) -> Member {
    Member {
        pid: writer.pid,
        start: writer.start_secs,
        name: table.get(&writer.pid).map_or_else(
            || "a session".to_owned(),
            |process| {
                process
                    .exe
                    .as_deref()
                    .or_else(|| process.args.first().map(std::path::Path::new))
                    .and_then(std::path::Path::file_name)
                    .map_or_else(
                        || format!("process {}", process.pid),
                        |name| name.to_string_lossy().into_owned(),
                    )
            },
        ),
    }
}

/// The writers no profile the move closes runs (`inside`: every process
/// of the running ones), as words name them.
fn outside(table: &Table, writers: &[Writer], inside: &[u32]) -> Vec<Member> {
    writers
        .iter()
        .filter(|writer| !inside.contains(&writer.pid))
        .map(|writer| member_of(table, writer))
        .collect()
}

/// What the `outside` writers do, in words: each program with its pid
/// ("claude (process 4242) has this conversation open outside the apps
/// the move closes (in a terminal, say)"). `apps`: the move closes app
/// profiles (else it moves terminal folders).
pub(super) fn outside_words(outside: &[Member], apps: bool) -> String {
    let named: Vec<String> = outside
        .iter()
        .map(|member| format!("{} (process {})", member.name, member.pid))
        .collect();
    let named = match named.as_slice() {
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        [] => String::new(),
    };
    if apps {
        format!(
            "{named} {} this conversation open outside the apps the move closes (in a terminal, say)",
            if outside.len() == 1 { "has" } else { "have" },
        )
    } else {
        format!("this conversation is open in a session ({named})")
    }
}

/// How the `outside` writers clear for `action`, in words.
pub(super) fn outside_clears(outside: &[Member], apps: bool, action: &str) -> String {
    format!(
        "{}; {action} becomes available once you end {}",
        outside_words(outside, apps),
        if outside.len() == 1 { "it" } else { "them" }
    )
}

/// The offer's look (no quit): the writers of the family running outside
/// every profile the move would close (`running`: each running place
/// with its main process).
pub(super) fn outside_now(
    state: &Arc<AppState>,
    system: &dyn MoveSystem,
    running: &[(KnownPlace, u32)],
    writing: &Writing<'_>,
) -> Vec<Member> {
    let table = system.table(state);
    let writers = writers_now(&table, writing);
    if writers.is_empty() {
        return Vec::new();
    }
    let inside: Vec<u32> = running
        .iter()
        .flat_map(|(place, pid)| system.tree(state, place, &[*pid]))
        .map(|member| member.pid)
        .collect();
    outside(&table, &writers, &inside)
}

/// Which of `waited` still run as the processes they were (by index).
fn still(system: &dyn MoveSystem, waited: &[(ProcId, Member, String)]) -> Vec<usize> {
    let pids: Vec<u32> = waited.iter().map(|(id, _, _)| id.pid).collect();
    let now = system.identify(&pids);
    waited
        .iter()
        .enumerate()
        .filter(|(_, (id, _, _))| now.get(&id.pid) == Some(id))
        .map(|(at, _)| at)
        .collect()
}

/// One profile the move closes, decided before anything quits.
struct Closing {
    /// The place's id, for the holder's door (AJ-E2319).
    id: String,
    name: String,
    main: ProcId,
    /// Everything of it looked at before the quit (reported, not waited).
    tree: Vec<Member>,
}

/// What [`stop_places`] did: the profiles it stopped, and what of theirs
/// keeps running, in words (the move's notes).
pub(super) struct Stopping {
    pub stopped: Vec<StoppedPlace>,
    pub notes: Vec<String>,
    /// AJ-E2322: the wait set, each with its program, reason and
    /// evidence, and what of the trees remained (the journal's timeline).
    pub waited: Vec<serde_json::Value>,
    pub remained: Vec<serde_json::Value>,
}

/// Quits `places` (see the module's notes) and waits for the wait set.
/// On a refusal nothing was changed, and what it quit starts again.
#[allow(
    clippy::too_many_lines,
    reason = "decide, quit, wait, report: one sequence, read top to bottom"
)]
pub(super) fn stop_places(
    state: &Arc<AppState>,
    (places, request): (&[(String, KnownPlace)], &MoveRequest),
    system: &dyn MoveSystem,
    outcome: &str,
    writing: &Writing<'_>,
) -> AppResult<Stopping> {
    // Decide: each running profile by its exact process, its tree, and
    // the writers (the table read once).
    let table = system.table(state);
    let writers = writers_now(&table, writing);
    let running: Vec<(usize, u32, Option<i64>)> = places
        .iter()
        .enumerate()
        .filter_map(|(at, (id, _))| system.running(state, id).map(|(pid, s)| (at, pid, s)))
        .collect();
    let ids = system.identify(&running.iter().map(|(_, pid, _)| *pid).collect::<Vec<_>>());
    // By the places' order: words and quits in it.
    let mut closing: BTreeMap<usize, Closing> = BTreeMap::new();
    for (at, pid, start) in &running {
        let place = &places[*at].1;
        let name = place_name(state, place);
        // The pid is this profile's process still (not one the system
        // gave another program since): its start, to the microsecond.
        let Some(main) = ids.get(pid).copied().filter(|id| id.started_in(*start)) else {
            // AJ-E2319: it was starting or quitting just then.
            let words = format!(
                "AI Juggler couldn't be sure which process is {name}, so nothing was quit or {outcome}"
            );
            return Err(MoveStop::new(StopClass::Transient, words.clone(), words)
                .held_by(vec![held(&places[*at].0, &name)])
                .into());
        };
        // All of it, looked at before it quits: once its main process
        // ends, what it started belongs to launchd.
        let tree = system.tree(state, place, &[*pid]);
        let id = places[*at].0.clone();
        closing.insert(
            *at,
            Closing {
                id,
                name,
                main,
                tree,
            },
        );
    }
    let inside: Vec<u32> = closing
        .values()
        .flat_map(|c| c.tree.iter().map(|member| member.pid))
        .collect();
    let elsewhere = outside(&table, &writers, &inside);
    if !elsewhere.is_empty() {
        // AJ-E2319: a program outside the profiles writes it: the person's.
        let words = format!(
            "{}, so nothing was quit or {outcome}",
            outside_words(&elsewhere, !places.is_empty())
        );
        let holders = elsewhere
            .iter()
            .map(|member| MoveHolder {
                name: member.name.clone(),
                place_id: None,
            })
            .collect();
        return Err(MoveStop::new(StopClass::NeedsYou, words.clone(), words)
            .held_by(holders)
            .into());
    }
    // The writers to wait for, by their exact process, each with the
    // profile it runs in.
    let writer_ids = system.identify(&writers.iter().map(|w| w.pid).collect::<Vec<_>>());
    let mut waited: Vec<(ProcId, Member, String)> = writers
        .iter()
        .filter_map(|writer| {
            let id = writer_ids
                .get(&writer.pid)
                .copied()
                .filter(|id| id.started_in(writer.start_secs))?;
            let owner = closing
                .values()
                .find(|c| c.tree.iter().any(|m| m.pid == writer.pid))?;
            Some((id, member_of(&table, writer), owner.name.clone()))
        })
        .collect();

    // Quit: every main process at once.
    let names: Vec<&str> = closing.values().map(|c| c.name.as_str()).collect();
    if !names.is_empty() {
        system.tell(&MoveProgressInfo::step(
            "closing",
            format!("Waiting for {} to finish closing…", names.join(" and ")),
        ));
    }
    let stopped: Vec<StoppedPlace> = places
        .iter()
        .enumerate()
        .map(|(at, (id, place))| StoppedPlace {
            id: id.clone(),
            place: place.clone(),
            was_running: closing.contains_key(&at),
        })
        .collect();
    for c in closing.values() {
        let now = |pid| system.identify(&[pid]).get(&pid).copied();
        let asked = signal::quit_checked(c.main, &now, &|pid| system.quit(pid));
        if let Err(not) = asked {
            start_again(state, &stopped, request, system);
            // AJ-E2319: one that ended just then can clear by itself; a
            // quit the system refused is never expected.
            let (class, words) = match not {
                NotAsked::NotThatProcess => (
                    StopClass::Transient,
                    format!(
                        "AI Juggler couldn't be sure which process is {}, so nothing was quit or {outcome}",
                        c.name
                    ),
                ),
                NotAsked::Refused(why) => {
                    (StopClass::Bug, format!("couldn't quit {}: {why}", c.name))
                }
            };
            return Err(MoveStop::new(class, words.clone(), words)
                .held_by(vec![held(&c.id, &c.name)])
                .into());
        }
    }
    // The apps, to their deadline.
    let wait = system.quit_wait();
    let until = Instant::now() + wait;
    loop {
        let mains: Vec<u32> = closing.values().map(|c| c.main.pid).collect();
        let now = system.identify(&mains);
        let left: Vec<&Closing> = closing
            .values()
            .filter(|c| now.get(&c.main.pid) == Some(&c.main))
            .collect();
        if left.is_empty() {
            break;
        }
        if Instant::now() >= until {
            start_again(state, &stopped, request, system);
            // AJ-E2319: it is asking something; only the person answers.
            let words = format!(
                "{} didn't quit within {} seconds, so nothing was {outcome}. It may be asking something: look at its window",
                left[0].name,
                wait.as_secs().max(1)
            );
            return Err(MoveStop::new(StopClass::NeedsYou, words.clone(), words)
                .held_by(vec![held(&left[0].id, &left[0].name)])
                .into());
        }
        // AJ-E2324: woken by an exit, never polled.
        let pids: Vec<u32> = left.iter().map(|c| c.main.pid).collect();
        system.wait_exit(&pids, until.saturating_duration_since(Instant::now()));
    }
    // Their sessions that write the family, to theirs; one that started
    // since is waited for too.
    let wait = system.writer_wait();
    let until = Instant::now() + wait;
    loop {
        let alive = still(system, &waited);
        if alive.is_empty() {
            let table = system.table(state);
            let fresh: Vec<Writer> = writers_now(&table, writing)
                .into_iter()
                .filter(|w| !waited.iter().any(|(id, _, _)| id.pid == w.pid))
                .collect();
            let ids = system.identify(&fresh.iter().map(|w| w.pid).collect::<Vec<_>>());
            let first = names.first().map_or_else(String::new, |n| (*n).to_owned());
            let more: Vec<(ProcId, Member, String)> = fresh
                .iter()
                .filter_map(|w| {
                    let id = ids
                        .get(&w.pid)
                        .copied()
                        .filter(|id| id.started_in(w.start_secs))?;
                    Some((id, member_of(&table, w), first.clone()))
                })
                .collect();
            if more.is_empty() {
                break;
            }
            waited.extend(more);
            continue;
        }
        if Instant::now() >= until {
            start_again(state, &stopped, request, system);
            let left: Vec<Member> = alive.iter().map(|at| waited[*at].1.clone()).collect();
            let owner = waited[alive[0]].2.clone();
            let secs = wait.as_secs().max(1);
            // AJ-E2319: writers still finishing: tried once more.
            let holders = closing
                .values()
                .filter(|c| c.name == owner)
                .map(|c| held(&c.id, &c.name))
                .collect();
            return Err(MoveStop::new(
                StopClass::Transient,
                tree::not_closed(&owner, secs, outcome, &left),
                format!(
                    "still writing {secs} s after the apps closed: {}",
                    left.iter()
                        .map(|m| format!("{} (process {})", m.name, m.pid))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
            .held_by(holders)
            .into());
        }
        let pids: Vec<u32> = alive.iter().map(|at| waited[*at].0.pid).collect();
        system.wait_exit(&pids, until.saturating_duration_since(Instant::now()));
    }
    // AJ-E2322: the wait set, for the timeline.
    let mut wait_set: Vec<serde_json::Value> = closing
        .values()
        .map(|c| {
            serde_json::json!({
                "program": c.name, "pid": c.main.pid,
                "reason": "the app", "evidence": "its main process",
            })
        })
        .collect();
    wait_set.extend(waited.iter().map(|(id, member, owner)| {
        let evidence = writers
            .iter()
            .find(|w| w.pid == id.pid)
            .map_or("found as the apps closed", |w| {
                super::timeline::evidence_words(w.evidence)
            });
        serde_json::json!({
            "program": member.name, "pid": id.pid,
            "reason": format!("writes the conversation in {owner}"), "evidence": evidence,
        })
    }));
    let mut remained = Vec::new();
    // The rest of each tree: reported, never waited on.
    let mut notes = Vec::new();
    for c in closing.values() {
        let rest: Vec<&Member> = c
            .tree
            .iter()
            .filter(|m| m.pid != c.main.pid && !waited.iter().any(|(id, _, _)| id.pid == m.pid))
            .collect();
        let now = system.identify(&rest.iter().map(|m| m.pid).collect::<Vec<_>>());
        let still: Vec<Member> = rest
            .into_iter()
            .filter(|m| now.get(&m.pid).is_some_and(|id| id.started_in(m.start)))
            .cloned()
            .collect();
        remained.extend(still.iter().map(|m| {
            serde_json::json!({"program": m.name, "pid": m.pid, "reason": format!("{} started it", c.name)})
        }));
        if let Some(note) = kept_running(&c.name, &still) {
            notes.push(note);
        }
    }
    Ok(Stopping {
        stopped,
        notes,
        waited: wait_set,
        remained,
    })
}

/// What `name` started that keeps running after it closed, in words
/// ("node that Claude app · Work started is still running; the move
/// doesn't need it closed"). `None`: nothing.
/// A profile that holds the files, with its door (AJ-E2319).
fn held(id: &str, name: &str) -> MoveHolder {
    MoveHolder {
        name: name.to_owned(),
        place_id: Some(id.to_owned()),
    }
}

pub(super) fn kept_running(name: &str, still: &[Member]) -> Option<String> {
    if still.is_empty() {
        return None;
    }
    let one = still.len() == 1;
    Some(format!(
        "{} that {name} started {} still running; the move doesn't need {} closed",
        tree::words(still),
        if one { "is" } else { "are" },
        if one { "it" } else { "them" },
    ))
}
