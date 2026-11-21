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
//! - every program of the app's own bundle that names a path inside the
//!   profile's folder (the `disclaimer` wrapper names that Claude Code);
//! - and everything below each of those.
//!
//! AI Juggler's own process and what it runs are never one of them. The
//! process table is the live view's ([`procs::table`]); nothing here reads
//! an environment.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use super::book::KnownPlace;
use crate::features::conversations::places::{HostRegistry, arg_value};
use crate::features::conversations::procs::{self, Proc, Table};
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
/// with everything below them; never `own` (AI Juggler) or below it.
/// `instance_arg` is the app's instance flag (`--user-data-dir`).
#[must_use]
pub fn of(
    table: &Table,
    place: &KnownPlace,
    instance_arg: Option<&str>,
    roots: &[u32],
    own: u32,
) -> Vec<Member> {
    let dir = place.instance_dir.as_deref();
    let inside = |path: &Path| dir.is_some_and(|dir| path.starts_with(dir));
    let marked = |process: &Proc| -> bool {
        let Some(dir) = dir else {
            return false;
        };
        if process.exe.as_deref().is_some_and(inside) {
            return true;
        }
        if instance_arg
            .and_then(|flag| arg_value(&process.args, flag))
            .is_some_and(|given| Path::new(&given) == dir)
        {
            return true;
        }
        let of_the_app = process
            .exe
            .as_deref()
            .is_some_and(|exe| exe.starts_with(&place.bundle));
        of_the_app
            && process.args.iter().skip(1).any(|arg| {
                let value = arg.split_once('=').map_or(arg.as_str(), |(_, value)| value);
                inside(Path::new(value))
            })
    };
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for process in table.values() {
        if let Some(parent) = process.ppid {
            children.entry(parent).or_default().push(process.pid);
        }
    }
    let mut queue: Vec<u32> = roots
        .iter()
        .copied()
        .filter(|pid| table.contains_key(pid))
        .collect();
    queue.extend(table.values().filter(|p| marked(p)).map(|p| p.pid));
    let mut found = BTreeSet::new();
    while let Some(pid) = queue.pop() {
        if pid <= 1 || pid == own || !found.insert(pid) {
            continue;
        }
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

/// Every process of `place` running now (`roots`: its main process while
/// it runs), from the live view's process table.
#[must_use]
pub fn now(state: &Arc<AppState>, place: &KnownPlace, roots: &[u32]) -> Vec<Member> {
    let hosts = HostRegistry::of(&state.registry);
    let flag = hosts
        .by_id(&place.app_id)
        .and_then(|app| app.instance_arg.clone());
    of(
        &procs::table(),
        place,
        flag.as_deref(),
        roots,
        std::process::id(),
    )
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
        KnownPlace {
            app_id: "claude-app".into(),
            name: "Claude".into(),
            instance: Some("work".into()),
            bundle: PathBuf::from(APP),
            exe: PathBuf::from(format!("{APP}/Contents/MacOS/Claude")),
            given: Some(PathBuf::from(dir)),
            instance_dir: Some(PathBuf::from(dir)),
            account_id: None,
            conversations: Vec::new(),
            seen_ms: 1,
            keep_alive: false,
            paused: None,
            found: false,
            users: true,
            declined: false,
        }
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

    #[test]
    fn a_profile_is_its_main_process_helpers_and_the_tools_it_runs() {
        let found = of(
            &table(true),
            &place(PROFILE),
            Some("--user-data-dir"),
            &[10],
            999,
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
        let found = of(&table(true), &place(PROFILE), None, &[10], 14);
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
        );
        assert!(none.is_empty(), "{none:?}");
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
