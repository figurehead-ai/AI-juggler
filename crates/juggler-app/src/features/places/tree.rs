//! 0.4.9: a desktop app profile's whole process tree, for Move to
//! account, which must not back up a conversation while anything of the
//! profile can still write it.
//!
//! Quitting a profile ends its main process first. Its helpers and the
//! tools it runs outlive it: in the live trace (2026-11-21) the Claude
//! app's main process and its nine Electron helpers ended within 0.1 s,
//! the Code tab's Claude Code wrote its last two lines after that and
//! ended at 0.3 s, and its `disclaimer` wrapper ended at 6.3 s. Once the
//! main process ends they belong to launchd, so a walk of its children
//! finds nothing. A profile's processes are therefore:
//!
//! - its main process and everything below it, looked at **before** it
//!   quits (the caller keeps each pid with its start time);
//! - every process that carries the profile's folder as its instance
//!   (`--user-data-dir=`, as Chromium's helpers do);
//! - every program inside the profile's own folder (the Claude app runs
//!   the Code tab's Claude Code from `<profile>/claude-code/…`);
//! - every program of the app's own bundle that runs a program inside the
//!   profile's folder: the one it names after `--` (the `disclaimer`
//!   wrapper runs `disclaimer --pgroup -- <profile>/claude-code/…/claude`);
//! - and everything below each of those.
//!
//! 0.5.1 (AJ-E2315): a wrapper is the profile's by the program it runs,
//! never by a flag's value. Another profile's Code tab gave its own
//! `disclaimer` an `--add-dir=` inside the moving profile's folder; the
//! old mark ("names a path inside the folder") counted that wrapper and
//! its `claude` as the moving profile's, they never ended, and every Move
//! to account was refused. Stronger evidence wins over every mark: a
//! process is never this profile's when its program is inside another
//! known profile's folder, when its instance flag names another profile's
//! folder, or when the nearest app main process above it is another app's
//! or profile's. The rule is shared, with its evidence order, in
//! [`crate::features::ownership`].
//!
//! AI Juggler's own process and what it runs are never one of them. The
//! process table is the live view's ([`procs::table`]); nothing here reads
//! an environment.
//!
//! [`procs::table`]: crate::features::conversations::procs::table

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use super::book::KnownPlace;
use crate::features::conversations::places::{HostRegistry, running_apps};
use crate::features::conversations::procs::{Proc, Table};
pub use crate::features::ownership::Others;
use crate::features::ownership::{self, InstanceKey, Profile, Whose};
use crate::state::AppState;

/// One process of a profile: what tells it apart, and its program's name
/// (no path), for words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// Its pid.
    pub pid: u32,
    /// Its start time (a pid the system gives another program later is
    /// told apart by it).
    pub start: Option<i64>,
    /// Its program's name (`claude`, `Claude Helper (Renderer)`).
    pub name: String,
}

/// Every process of `place` in `table` (see the module's notes): `roots`
/// (its main process, while it runs) and the processes its marks find,
/// with everything below them; never another app's or profile's
/// (`others`, the stronger evidence), never `own` (AI Juggler) or below
/// it. `instance_arg` is the app's instance flag (`--user-data-dir`).
#[must_use]
pub fn of(
    table: &Table,
    place: &KnownPlace,
    instance_arg: Option<&str>,
    roots: &[u32],
    own: u32,
    others: &Others,
) -> Vec<Member> {
    // A profile with no folder is its main process and what is below it.
    let key = place.instance_dir.as_deref().map(InstanceKey::of);
    let profile = key.as_ref().map(|dir| Profile {
        dir,
        bundle: &place.bundle,
        instance_arg,
        mains: roots,
    });
    let whose = |process: &Proc| {
        profile.as_ref().map_or(Whose::Unknown, |profile| {
            ownership::whose(table, process, profile, others)
        })
    };
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for process in table.values() {
        if let Some(parent) = process.ppid {
            children.entry(parent).or_default().push(process.pid);
        }
    }
    // AI Juggler and everything below it, whoever's they seem.
    let mut own_tree = BTreeSet::new();
    let mut below_own = vec![own];
    while let Some(pid) = below_own.pop() {
        if own_tree.insert(pid)
            && let Some(below) = children.get(&pid)
        {
            below_own.extend(below);
        }
    }
    let mut queue: Vec<u32> = roots
        .iter()
        .copied()
        .filter(|pid| table.contains_key(pid))
        .collect();
    queue.extend(
        table
            .values()
            .filter(|p| whose(p) == Whose::This)
            .map(|p| p.pid),
    );
    let mut found = BTreeSet::new();
    while let Some(pid) = queue.pop() {
        if pid <= 1 || own_tree.contains(&pid) || found.contains(&pid) {
            continue;
        }
        // Another's (the stronger evidence) never is, nor what it runs.
        if !roots.contains(&pid) && table.get(&pid).is_some_and(|p| whose(p) == Whose::Another) {
            continue;
        }
        found.insert(pid);
        if let Some(below) = children.get(&pid) {
            queue.extend(below);
        }
    }
    found
        .into_iter()
        .filter_map(|pid| table.get(&pid))
        .map(|process| Member {
            pid: process.pid,
            start: process.start,
            name: name_of(process),
        })
        .collect()
}

/// A process's program name, without its folder.
fn name_of(process: &Proc) -> String {
    process
        .exe
        .as_deref()
        .or_else(|| process.args.first().map(Path::new))
        .and_then(Path::file_name)
        .map_or_else(
            || format!("process {}", process.pid),
            |name| name.to_string_lossy().into_owned(),
        )
}

/// What [`now`] and [`quits_juggler_now`] look at: the process table, the
/// app's instance flag and every other app and profile.
fn looked_at(
    state: &Arc<AppState>,
    place: &KnownPlace,
    roots: &[u32],
) -> (Table, Option<String>, Others) {
    let hosts = HostRegistry::of(&state.registry);
    let flag = hosts
        .by_id(&place.app_id)
        .and_then(|app| app.instance_arg.clone());
    let table = state.places.table();
    let others = others_now(state, &hosts, &table, place, roots);
    (table, flag, others)
}

/// Every process of `place` running now (`roots`: its main process while
/// it runs), from the live view's process table.
#[must_use]
pub fn now(state: &Arc<AppState>, place: &KnownPlace, roots: &[u32]) -> Vec<Member> {
    let (table, flag, others) = looked_at(state, place, roots);
    of(
        &table,
        place,
        flag.as_deref(),
        roots,
        std::process::id(),
        &others,
    )
}

/// AJ-E2315 (the self rule, [`ownership::self_rule`]): why closing `place`
/// (`name`, its main process `pid`) for `action` (Move, Undo) would close
/// an AI Juggler, in words: this one runs below a process of its tree (it
/// was started from the profile's Code tab), or another one is in it or
/// below it. `None`: neither. Asked before anything quits.
#[must_use]
pub fn quits_juggler_now(
    state: &Arc<AppState>,
    place: &KnownPlace,
    pid: u32,
    name: &str,
    action: &str,
) -> Option<String> {
    let (table, flag, others) = looked_at(state, place, &[pid]);
    let own = std::process::id();
    let members: Vec<u32> = of(&table, place, flag.as_deref(), &[pid], own, &others)
        .iter()
        .map(|member| member.pid)
        .collect();
    let exe = std::env::current_exe().ok()?;
    let juggler = ownership::self_rule::quits_juggler(&table, &members, own, exe.file_name()?)?;
    Some(ownership::self_rule::words(juggler, name, action))
}

/// Why `name`'s sessions that write the moving conversation (`left`)
/// didn't finish within `secs` after it closed, so nothing was `outcome`,
/// in words.
#[must_use]
pub fn not_closed(name: &str, secs: u64, outcome: &str, left: &[Member]) -> String {
    format!(
        "{name} didn't finish closing within {secs} seconds: {} still {}, so nothing was {outcome}. Try again in a moment.",
        words(left),
        if left.len() == 1 { "runs" } else { "run" }
    )
}

/// Every other app and profile, from what runs now and the profiles AI
/// Juggler knows: never `place` itself (by its app and folder, or its
/// main process in `roots`).
fn others_now(
    state: &Arc<AppState>,
    hosts: &HostRegistry,
    table: &Table,
    place: &KnownPlace,
    roots: &[u32],
) -> Others {
    let mut others = Others::default();
    let this = place.instance_dir.as_deref().map(InstanceKey::of);
    for app in running_apps(table, hosts, &state.places.home()) {
        if roots.contains(&app.pid) || app.runs(&place.app_id, place.instance_dir.as_deref()) {
            continue;
        }
        others.mains.push(app.pid);
        if let Some(dir) = app.instance {
            others.add(&dir);
        }
    }
    let known = super::with_book(state, |book| {
        book.places
            .values()
            .filter_map(|known| known.instance_dir.clone())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    for dir in known {
        others.add(&dir);
    }
    // Never `place` itself, however its folder is spelled.
    others.dirs.retain(|dir| Some(dir) != this.as_ref());
    others
}

/// What still runs, in words: each program once, the most first, a
/// helper's kind left out ("claude, disclaimer and 9 Claude Helper
/// processes"); at most three names, then how many more.
#[must_use]
pub fn words(members: &[Member]) -> String {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for member in members {
        let name = member
            .name
            .split_once(" (")
            .map_or(member.name.as_str(), |(name, _)| name)
            .to_owned();
        match counts.iter_mut().find(|(known, _)| *known == name) {
            Some((_, count)) => *count += 1,
            None => counts.push((name, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut parts: Vec<String> = counts
        .iter()
        .take(3)
        .map(|(name, count)| {
            if *count == 1 {
                name.clone()
            } else {
                format!("{count} {name} processes")
            }
        })
        .collect();
    let more: usize = counts.iter().skip(3).map(|(_, count)| count).sum();
    if more > 0 {
        parts.push(format!(
            "{more} more {}",
            if more == 1 { "process" } else { "processes" }
        ));
    }
    match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const APP: &str = "/Applications/Claude.app";
    const PROFILE: &str = "/Users/you/Library/Application Support/ClaudeProfiles/work";

    use crate::features::conversations::procs::fixture as proc;

    fn place(dir: &str) -> KnownPlace {
        crate::test_support::claude_profile(dir)
    }

    /// The live trace's tree (2026-11-21), with another profile beside it
    /// and a terminal session: the main process (10), its helpers (11,
    /// 12), the `disclaimer` wrapper (13) and the Code tab's Claude Code
    /// (14) with a server it started (15).
    fn table(main_alive: bool) -> Table {
        let main = format!("{APP}/Contents/MacOS/Claude");
        let flag = format!("--user-data-dir={PROFILE}");
        let helper = format!(
            "{APP}/Contents/Frameworks/Claude Helper (Renderer).app/Contents/MacOS/Claude Helper (Renderer)"
        );
        let gpu = format!(
            "{APP}/Contents/Frameworks/Claude Helper (GPU).app/Contents/MacOS/Claude Helper (GPU)"
        );
        let code = format!("{PROFILE}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude");
        let disclaimer = format!("{APP}/Contents/Helpers/disclaimer");
        // After the main process ends, what it started is launchd's.
        let parent = if main_alive { 10 } else { 1 };
        let mut rows: Vec<(u32, Proc)> = vec![
            proc(1, 0, "/sbin/launchd", &[]),
            proc(
                11,
                parent,
                &helper,
                &["Claude Helper (Renderer)", "--type=renderer", &flag],
            ),
            proc(
                12,
                parent,
                &gpu,
                &["Claude Helper (GPU)", "--type=gpu-process", &flag],
            ),
            proc(
                13,
                parent,
                &disclaimer,
                &["disclaimer", "--pgroup", "--", &code],
            ),
            proc(14, 13, &code, &["claude", "--resume=s-1"]),
            proc(15, 14, "/opt/homebrew/bin/node", &["node", "server.js"]),
            // Another profile of the same app, and a terminal session.
            proc(
                20,
                1,
                &main,
                &[&main, "--user-data-dir=/Users/you/P/personal"],
            ),
            proc(
                21,
                20,
                &helper,
                &[
                    "Claude Helper (Renderer)",
                    "--user-data-dir=/Users/you/P/personal",
                ],
            ),
            proc(
                30,
                1,
                "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal",
                &[],
            ),
            proc(31, 30, "/Users/you/.local/bin/claude", &["claude"]),
        ];
        if main_alive {
            rows.push(proc(10, 1, &main, &[&main, &flag]));
        }
        rows.into_iter().collect()
    }

    fn pids(members: &[Member]) -> Vec<u32> {
        members.iter().map(|m| m.pid).collect()
    }

    const OTHER: &str = "/Users/you/P/personal";

    /// The other profile beside it, as [`now`] reads it: its folder and
    /// its running main process (20).
    fn others() -> Others {
        Others::of([PathBuf::from(OTHER)], vec![20])
    }

    /// AJ-E2315's shape (the live trace, 2026-11-22): the other profile's
    /// Code tab (its main process 20) runs its own `disclaimer` (22) and
    /// `claude` (23), with an `--add-dir=` into a folder inside the moving
    /// profile's folder.
    fn with_the_other_code_tab(mut table: Table, parent: u32) -> Table {
        let disclaimer = format!("{APP}/Contents/Helpers/disclaimer");
        let code = format!("{OTHER}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude");
        let add_dir = format!("--add-dir={PROFILE}/local-agent-mode-sessions");
        table.extend([
            proc(
                22,
                parent,
                &disclaimer,
                &[
                    &disclaimer,
                    "--pgroup",
                    "--",
                    &code,
                    "--resume=s-2",
                    &add_dir,
                ],
            ),
            proc(23, 22, &code, &[&code, "--resume=s-2", &add_dir]),
        ]);
        table
    }

    #[test]
    fn a_profile_is_its_main_process_helpers_and_the_tools_it_runs() {
        let found = of(
            &table(true),
            &place(PROFILE),
            Some("--user-data-dir"),
            &[10],
            999,
            &others(),
        );
        assert_eq!(pids(&found), [10, 11, 12, 13, 14, 15]);
        assert_eq!(found[4].name, "claude");
        assert_eq!(found[4].start, Some(140));
    }

    /// Once the main process ended, its helpers and the Code tab are
    /// launchd's: they are still the profile's, by their marks.
    #[test]
    fn what_outlives_the_main_process_is_still_the_profiles() {
        let found = of(
            &table(false),
            &place(PROFILE),
            Some("--user-data-dir"),
            &[],
            999,
            &others(),
        );
        assert_eq!(pids(&found), [11, 12, 13, 14, 15]);
        assert_eq!(
            words(&found),
            "2 Claude Helper processes, claude, disclaimer and 1 more process"
        );
    }

    /// A default profile (no flag) is found by what it ran below its main
    /// process and by its own folder; another profile and AI Juggler are
    /// never in it.
    #[test]
    fn another_profile_and_ai_juggler_are_never_in_it() {
        let found = of(&table(true), &place(PROFILE), None, &[10], 14, &others());
        assert_eq!(
            pids(&found),
            [10, 11, 12, 13],
            "AI Juggler (14) and below left out"
        );
        let none = of(
            &table(false),
            &place("/Users/you/P/other"),
            Some("--user-data-dir"),
            &[],
            999,
            &others(),
        );
        assert!(none.is_empty(), "{none:?}");
    }

    /// AJ-E2315: another profile's wrapper and its `claude` are never the
    /// moving profile's, before it quits or after, though a flag of theirs
    /// names a folder inside its folder.
    #[test]
    fn another_profiles_wrapper_naming_a_folder_inside_it_is_never_in_it() {
        let before = of(
            &with_the_other_code_tab(table(true), 20),
            &place(PROFILE),
            Some("--user-data-dir"),
            &[10],
            999,
            &others(),
        );
        assert_eq!(pids(&before), [10, 11, 12, 13, 14, 15]);
        let after = of(
            &with_the_other_code_tab(table(false), 20),
            &place(PROFILE),
            Some("--user-data-dir"),
            &[],
            999,
            &others(),
        );
        assert_eq!(pids(&after), [11, 12, 13, 14, 15]);
    }

    /// A flag's value never marks a wrapper, even with nothing else known:
    /// the other profile gone (its wrapper launchd's) and unknown to AI
    /// Juggler, its wrapper runs a program outside the moving profile.
    #[test]
    fn a_flags_path_alone_never_marks_a_wrapper() {
        let found = of(
            &with_the_other_code_tab(table(false), 1),
            &place(PROFILE),
            Some("--user-data-dir"),
            &[],
            999,
            &Others::default(),
        );
        assert_eq!(pids(&found), [11, 12, 13, 14, 15]);
    }

    /// Stronger evidence wins over a mark: a program inside the moving
    /// profile's folder that another profile's main process runs, and a
    /// helper of a profile whose folder is inside the moving one's (the
    /// deepest known folder owns a path).
    #[test]
    fn another_profiles_evidence_wins_over_a_mark() {
        let nested = format!("{PROFILE}/nested");
        let mut rows = table(false);
        rows.extend([
            // Another profile's Code tab runs the moving profile's copy.
            proc(
                24,
                20,
                &format!("{PROFILE}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude"),
                &["claude"],
            ),
            proc(
                25,
                1,
                &format!("{nested}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude"),
                &["claude"],
            ),
        ]);
        let mut known = others();
        known.add(Path::new(&nested));
        let found = of(
            &rows,
            &place(PROFILE),
            Some("--user-data-dir"),
            &[],
            999,
            &known,
        );
        assert_eq!(pids(&found), [11, 12, 13, 14, 15]);
    }

    #[test]
    fn words_name_each_program_once() {
        let member = |pid, name: &str| Member {
            pid,
            start: None,
            name: name.into(),
        };
        assert_eq!(words(&[member(1, "claude")]), "claude");
        assert_eq!(
            words(&[member(1, "claude"), member(2, "disclaimer")]),
            "claude and disclaimer"
        );
        assert_eq!(words(&[]), "");
    }
}
