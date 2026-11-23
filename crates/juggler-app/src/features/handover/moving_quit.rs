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
//! stillness check before the backup (`hold_still`), the backup for
//! writers no hook names.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use juggler_adapter::moving::{InPlaceMove, MoveRequest, ProcView, Writer};

use super::{
    EVERY, KnownPlace, Member, MoveProgressInfo, MoveSystem, StoppedPlace, place_name, refused,
    start_again, tree,
};
use crate::error::{AppError, AppResult};
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
            return Err(refused(format!(
                "AI Juggler couldn't be sure which process is {name}, so nothing was quit or {outcome}"
            )));
        };
        // All of it, looked at before it quits: once its main process
        // ends, what it started belongs to launchd.
        let tree = system.tree(state, place, &[*pid]);
        closing.insert(*at, Closing { name, main, tree });
    }
    let inside: Vec<u32> = closing
        .values()
        .flat_map(|c| c.tree.iter().map(|member| member.pid))
        .collect();
    let elsewhere = outside(&table, &writers, &inside);
    if !elsewhere.is_empty() {
        return Err(refused(format!(
            "{}, so nothing was quit or {outcome}",
            outside_words(&elsewhere, !places.is_empty())
        )));
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
            return Err(refused(match not {
                NotAsked::NotThatProcess => format!(
                    "AI Juggler couldn't be sure which process is {}, so nothing was quit or {outcome}",
                    c.name
                ),
                NotAsked::Refused(why) => format!("couldn't quit {}: {why}", c.name),
            }));
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
            return Err(refused(format!(
                "{} didn't quit within {} seconds, so nothing was {outcome}. It may be asking something: look at its window",
                left[0].name,
                wait.as_secs().max(1)
            )));
        }
        std::thread::sleep(EVERY);
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
            return Err(AppError::Move {
                words: tree::not_closed(&owner, secs, outcome, &left),
                detail: format!(
                    "still writing {secs} s after the apps closed: {}",
                    left.iter()
                        .map(|m| format!("{} (process {})", m.name, m.pid))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                unchanged: true,
            });
        }
        std::thread::sleep(EVERY);
    }
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
        if let Some(note) = kept_running(&c.name, &still) {
            notes.push(note);
        }
    }
    Ok(Stopping { stopped, notes })
}

/// What `name` started that keeps running after it closed, in words
/// ("node that Claude app · Work started is still running; the move
/// doesn't need it closed"). `None`: nothing.
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
