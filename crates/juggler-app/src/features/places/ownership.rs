//! 0.5.1 (AJ-E2315): **whose a process or path is**, one rule for every
//! site that attributes a process to a desktop app profile (Move to
//! account's process tree today; any other attribution should call it
//! rather than read a command line of its own).
//!
//! The question: given a process (its program, arguments, instance flag
//! and parents) and the profiles AI Juggler knows, is it this profile's,
//! another known app's or profile's, or nobody's that can be told? The
//! answer is by the strongest evidence, in this order:
//!
//! 1. **Another's by its program**: the program, or the program a wrapper
//!    of the app's bundle runs (step 6), is inside another known
//!    profile's folder. A path belongs to the deepest known folder holding
//!    it, so a profile whose folder sits inside this one's keeps its own.
//! 2. **Another's by its instance flag**: the app's instance flag
//!    (`--user-data-dir`) names another known profile's folder.
//! 3. **By its nearest app main process**: walking up from the process
//!    itself, the first main process met decides: another app's or
//!    profile's makes it **another's**, this profile's makes it **this
//!    profile's**.
//! 4. **This profile's by its program**: inside this profile's folder.
//! 5. **This profile's by its instance flag**: naming this folder.
//! 6. **This profile's by the program it runs**: a program of the app's
//!    own bundle (a wrapper) whose wrapped program, the one after `--`
//!    (`disclaimer --pgroup -- <program> <its arguments>`), is inside
//!    this profile's folder (the deepest known folder holding it).
//!
//! Otherwise it is **unknown**. Another's evidence (1–3) always wins over
//! this profile's (4–6). A flag's value (`--add-dir`, `--settings`,
//! `--plugin-dir`, `--mcp-config`, any other) is never evidence: the live
//! bug was another profile's `disclaimer` with an `--add-dir=` into the
//! moving profile's folder, counted as the moving profile's, so Move to
//! account waited for it forever. Only the app's own instance flag (from
//! the registry, never named here) is read.

use std::path::{Path, PathBuf};

use crate::features::conversations::places::arg_value;
use crate::features::conversations::procs::{Proc, Table};

/// Whose a process or path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whose {
    /// The profile asked about.
    This,
    /// Another known app or profile.
    Another,
    /// Nothing tells.
    Unknown,
}

/// The profile asked about.
#[derive(Clone, Copy, Debug)]
pub struct Profile<'a> {
    /// Its instance folder.
    pub dir: &'a Path,
    /// Its app's bundle (`/Applications/Claude.app`).
    pub bundle: &'a Path,
    /// The app's instance flag (`--user-data-dir`), from the registry.
    pub instance_arg: Option<&'a str>,
    /// Its main processes running (none once it quit).
    pub mains: &'a [u32],
}

/// Every other app and profile known: the stronger evidence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Others {
    /// Other profiles' instance folders (known or seen running).
    pub dirs: Vec<PathBuf>,
    /// Other apps' and profiles' main processes, running.
    pub mains: Vec<u32>,
}

/// Whose `path` is: the deepest known folder holding it.
#[must_use]
pub fn whose_path(path: &Path, profile_dir: &Path, others: &Others) -> Whose {
    let depth = |folder: &Path| {
        path.starts_with(folder)
            .then(|| folder.components().count())
    };
    let this = depth(profile_dir);
    let another = others
        .dirs
        .iter()
        .filter(|other| other.as_path() != profile_dir)
        .filter_map(|other| depth(other))
        .max();
    match (this, another) {
        (Some(this), Some(another)) if another > this => Whose::Another,
        (Some(_), _) => Whose::This,
        (None, Some(_)) => Whose::Another,
        (None, None) => Whose::Unknown,
    }
}

/// The program a wrapper runs: the argument after its `--`. What follows
/// is that program's own arguments; a flag's value is never a program.
#[must_use]
pub fn program_run(args: &[String]) -> Option<&Path> {
    let rest = args.get(1..)?;
    let at = rest.iter().position(|arg| arg == "--")?;
    rest.get(at + 1).map(Path::new)
}

/// Whose the nearest app main process at or above `process` is (step 3).
fn by_parents(table: &Table, process: &Proc, profile: &Profile<'_>, others: &Others) -> Whose {
    let mut at = Some(process);
    // A parent chain is short; the bound only guards a looped table.
    for _ in 0..64 {
        let Some(here) = at else {
            break;
        };
        if profile.mains.contains(&here.pid) {
            return Whose::This;
        }
        if others.mains.contains(&here.pid) {
            return Whose::Another;
        }
        at = here
            .ppid
            .filter(|parent| *parent > 1 && *parent != here.pid)
            .and_then(|parent| table.get(&parent));
    }
    Whose::Unknown
}

/// Whose `process` is, by the strongest evidence (the module's notes).
/// `table` is the process table its parents are read from.
#[must_use]
pub fn whose(table: &Table, process: &Proc, profile: &Profile<'_>, others: &Others) -> Whose {
    let by_program = process
        .exe
        .as_deref()
        .map_or(Whose::Unknown, |exe| whose_path(exe, profile.dir, others));
    let by_flag = profile
        .instance_arg
        .and_then(|flag| arg_value(&process.args, flag))
        .map_or(Whose::Unknown, |given| {
            let given = Path::new(&given);
            if given == profile.dir {
                Whose::This
            } else if others.dirs.iter().any(|other| other == given) {
                Whose::Another
            } else {
                Whose::Unknown
            }
        });
    // A wrapper of the app's own bundle: whose the program it runs is.
    let of_the_app = process
        .exe
        .as_deref()
        .is_some_and(|exe| exe.starts_with(profile.bundle));
    let by_program_run = program_run(&process.args)
        .filter(|_| of_the_app)
        .map_or(Whose::Unknown, |program| {
            whose_path(program, profile.dir, others)
        });
    // 1, 2: another's by its program (or the one it runs) or its
    // instance flag.
    if by_program == Whose::Another || by_program_run == Whose::Another || by_flag == Whose::Another
    {
        return Whose::Another;
    }
    // 3: the nearest main process decides.
    match by_parents(table, process, profile, others) {
        Whose::Unknown => {}
        decided => return decided,
    }
    // 4, 5: this profile's by its program or its instance flag.
    if by_program == Whose::This || by_flag == Whose::This {
        return Whose::This;
    }
    // 6: a wrapper of the app's own bundle, by the program it runs.
    by_program_run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::conversations::procs::fixture as proc;

    const APP: &str = "/Applications/Claude.app";
    const WORK: &str = "/Users/you/Library/Application Support/ClaudeProfiles/work";
    const OTHER: &str = "/Users/you/Library/Application Support/ClaudeProfiles/team-a";

    fn profile(mains: &[u32]) -> Profile<'_> {
        Profile {
            dir: Path::new(WORK),
            bundle: Path::new(APP),
            instance_arg: Some("--user-data-dir"),
            mains,
        }
    }

    fn others() -> Others {
        Others {
            dirs: vec![PathBuf::from(OTHER)],
            mains: vec![20],
        }
    }

    fn code(dir: &str) -> String {
        format!("{dir}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude")
    }

    fn disclaimer() -> String {
        format!("{APP}/Contents/Helpers/disclaimer")
    }

    /// Work's main (10) and the other profile's (20), each with a wrapper
    /// and its Claude Code; the other profile's carries an `--add-dir=`
    /// into Work's folder.
    fn table() -> Table {
        let main = format!("{APP}/Contents/MacOS/Claude");
        let add_dir = format!("--add-dir={WORK}/local-agent-mode-sessions");
        let (work, other) = (code(WORK), code(OTHER));
        let d = disclaimer();
        [
            proc(10, 1, &main, &[&main, &format!("--user-data-dir={WORK}")]),
            proc(13, 10, &d, &[&d, "--pgroup", "--", &work]),
            proc(14, 13, &work, &[&work]),
            proc(20, 1, &main, &[&main, &format!("--user-data-dir={OTHER}")]),
            proc(22, 20, &d, &[&d, "--pgroup", "--", &other, &add_dir]),
            proc(23, 22, &other, &[&other, &add_dir]),
        ]
        .into_iter()
        .collect()
    }

    fn of(table: &Table, pid: u32, profile: &Profile<'_>, others: &Others) -> Whose {
        whose(table, &table[&pid], profile, others)
    }

    #[test]
    fn a_flags_path_never_makes_a_process_this_profiles() {
        let t = table();
        // With everything known, before and after Work quits.
        for mains in [&[10][..], &[]] {
            assert_eq!(of(&t, 22, &profile(mains), &others()), Whose::Another);
            assert_eq!(of(&t, 23, &profile(mains), &others()), Whose::Another);
        }
        // Its folder known but nothing running: another's by what it runs.
        let folders_only = Others {
            dirs: others().dirs,
            mains: Vec::new(),
        };
        assert_eq!(of(&t, 22, &profile(&[]), &folders_only), Whose::Another);
        // With nothing else known, it is still not Work's.
        assert_eq!(
            of(&t, 22, &profile(&[]), &Others::default()),
            Whose::Unknown
        );
    }

    #[test]
    fn this_profiles_wrapper_is_its_by_the_program_it_runs() {
        let t = table();
        for mains in [&[10][..], &[]] {
            assert_eq!(of(&t, 13, &profile(mains), &others()), Whose::This);
            assert_eq!(of(&t, 14, &profile(mains), &others()), Whose::This);
            assert_eq!(of(&t, 10, &profile(mains), &others()), Whose::This);
        }
    }

    #[test]
    fn another_profiles_evidence_wins() {
        let mut t = table();
        let work = code(WORK);
        let helper =
            format!("{APP}/Contents/Frameworks/Claude Helper.app/Contents/MacOS/Claude Helper");
        t.extend([
            // The other profile's main runs Work's copy of Claude Code.
            proc(24, 20, &work, &[&work]),
            // A helper naming the other profile's folder, launchd's.
            proc(
                25,
                1,
                &helper,
                &[&helper, &format!("--user-data-dir={OTHER}")],
            ),
            // the other profile's own Claude Code, launchd's.
            proc(26, 1, &code(OTHER), &["claude"]),
        ]);
        for pid in [24, 25, 26] {
            assert_eq!(
                of(&t, pid, &profile(&[]), &others()),
                Whose::Another,
                "{pid}"
            );
        }
    }

    #[test]
    fn the_deepest_known_folder_owns_a_path() {
        let nested = format!("{WORK}/nested");
        let known = Others {
            dirs: vec![PathBuf::from(&nested)],
            mains: Vec::new(),
        };
        let dir = Path::new(WORK);
        assert_eq!(
            whose_path(Path::new(&code(&nested)), dir, &known),
            Whose::Another
        );
        assert_eq!(whose_path(Path::new(&code(WORK)), dir, &known), Whose::This);
        assert_eq!(
            whose_path(Path::new("/usr/bin/env"), dir, &known),
            Whose::Unknown
        );
    }

    #[test]
    fn the_program_a_wrapper_runs_is_the_one_after_its_separator() {
        let args = |list: &[&str]| list.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            program_run(&args(&[
                "disclaimer",
                "--pgroup",
                "--",
                "/x/claude",
                "--add-dir=/y"
            ])),
            Some(Path::new("/x/claude"))
        );
        assert_eq!(program_run(&args(&["disclaimer", "--add-dir=/y"])), None);
        assert_eq!(program_run(&args(&["disclaimer", "--"])), None);
    }
}
