//! 0.5.1 (AJ-E2315): **whose a process or path is**, one rule for every
//! site that attributes a process to a desktop app instance (a Claude app
//! profile): Move to account's process tree, which instance runs
//! (`running_apps`, `running_now`), where a session runs (`host_of`), a
//! lab's own processes (`lab_scope`), which tool folder a profile's
//! sessions use (`places::link`), the agent bridge's attestation, and the
//! self rule ([`self_rule`]). No site reads a command line of its own.
//!
//! **Instance identity** ([`instance`]): the app's instance flag (from the
//! registry, never named here) is read strictly, exactly as the app reads
//! it; folders compare in their one real spelling ([`InstanceKey`]: links,
//! `..`, a trailing slash and letter case decide alike); where the flag
//! can't be trusted, the process holding the instance's lock decides.
//!
//! **Whose a process is**: given a process (its program, arguments,
//! instance flag and parents) and the profiles AI Juggler knows, is it
//! this profile's, another known app's or profile's, or nobody's that can
//! be told? The answer is by the strongest evidence, in this order:
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
//! account waited for it forever.

pub mod instance;
pub mod self_rule;
pub mod sessions;

use std::path::{Path, PathBuf};

pub use instance::{
    Holder, Instance, InstanceKey, ProcId, instance_arg, instance_of, same_instance, system_holder,
};

use crate::features::conversations::places::{HostRegistry, RunningApp, running_apps};
use crate::features::conversations::procs::{Proc, Table};
use crate::features::places::book::KnownPlace;

/// The main process `place` runs as now, in `table` (`home` places its
/// app's default instance): the one holding its instance's lock (asked of
/// `held`), else the one whose instance, read strictly, is its folder
/// however either is spelled. Never by the spelling of an id.
#[must_use]
pub fn running_place(
    table: &Table,
    hosts: &HostRegistry,
    home: &Path,
    place: &KnownPlace,
    held: Holder<'_>,
) -> Option<RunningApp> {
    let dir = place.instance_dir.as_deref();
    let holder = hosts
        .by_id(&place.app_id)
        .and_then(|app| instance::proven_running(table, app, &place.bundle, dir, held));
    // The lock's holder is it, whatever its command line says.
    if let Some(process) = holder.and_then(|pid| table.get(&pid)) {
        return Some(RunningApp {
            app_id: place.app_id.clone(),
            bundle: place.bundle.clone(),
            exe: process.exe.clone()?,
            pid: process.pid,
            start: process.start,
            given: place.given.clone(),
            instance: place.instance_dir.clone(),
        });
    }
    running_apps(table, hosts, home)
        .into_iter()
        .find(|app| app.runs(&place.app_id, dir))
}

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
    pub dir: &'a InstanceKey,
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
    pub dirs: Vec<InstanceKey>,
    /// Other apps' and profiles' main processes, running.
    pub mains: Vec<u32>,
}

impl Others {
    /// Every other profile's folder in `dirs` (each once, however it is
    /// spelled), with the main processes `mains`.
    #[must_use]
    pub fn of(dirs: impl IntoIterator<Item = PathBuf>, mains: Vec<u32>) -> Self {
        let mut others = Self {
            dirs: Vec::new(),
            mains,
        };
        for dir in dirs {
            others.add(&dir);
        }
        others
    }

    /// Adds another profile's folder `dir` (once).
    pub fn add(&mut self, dir: &Path) {
        let key = InstanceKey::of(dir);
        if !self.dirs.contains(&key) {
            self.dirs.push(key);
        }
    }
}

/// Whose `path` is: the deepest known folder holding it, however `path`
/// and the folders are spelled.
#[must_use]
pub fn whose_path(path: &Path, profile_dir: &InstanceKey, others: &Others) -> Whose {
    whose_real(
        &juggler_platform::real_path::canonical(path),
        profile_dir,
        others,
    )
}

/// [`whose_path`] for a path in its real spelling already.
fn whose_real(real: &Path, profile_dir: &InstanceKey, others: &Others) -> Whose {
    let depth = |folder: &InstanceKey| folder.holds(real).then(|| folder.depth());
    let this = depth(profile_dir);
    let another = others
        .dirs
        .iter()
        .filter(|other| *other != profile_dir)
        .filter_map(depth)
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
/// `table` is the process table its parents are read from. Its program
/// is the system's (its real spelling); the folders its arguments name
/// are resolved to theirs.
#[must_use]
pub fn whose(table: &Table, process: &Proc, profile: &Profile<'_>, others: &Others) -> Whose {
    let exe = process
        .exe
        .as_deref()
        .map(juggler_platform::real_path::lexical);
    let by_program = exe
        .as_deref()
        .map_or(Whose::Unknown, |exe| whose_real(exe, profile.dir, others));
    // Only the strict form: a flag the app might read otherwise tells
    // nothing (its parents may).
    let by_flag = profile
        .instance_arg
        .and_then(|flag| instance_arg(&process.args, flag).ok().flatten())
        .map_or(Whose::Unknown, |given| {
            let given = InstanceKey::of(Path::new(&given));
            if &given == profile.dir {
                Whose::This
            } else if others.dirs.contains(&given) {
                Whose::Another
            } else {
                Whose::Unknown
            }
        });
    // A wrapper of the app's own bundle: whose the program it runs is.
    let of_the_app = exe
        .as_deref()
        .is_some_and(|exe| exe.starts_with(juggler_platform::real_path::lexical(profile.bundle)));
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
mod tests;
