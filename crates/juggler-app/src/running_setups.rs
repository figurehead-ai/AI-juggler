//! Which Claude Code setups are open right now, and where.
//!
//! A setup is a config folder (`~/.claude`, a profile manager's profile,
//! a `CLAUDE_CONFIG_DIR` target); the same folder can be used by the
//! Claude desktop app, by `claude` in a terminal, or by both. First run
//! used to guess the tool from the folder's location and called the
//! default `~/.claude` "Terminal" — on the maintainer's Mac it was the
//! sixth Claude desktop window (0.0.6 dogfood). This reads the running
//! processes instead.
//!
//! Privacy: of each process's environment only `CLAUDE_CONFIG_DIR` is
//! read; nothing else is kept, logged or sent.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// One running process, as much as the classifier needs.
#[derive(Clone, Debug)]
pub struct ProcFacts {
    /// Process id.
    pub pid: u32,
    /// Parent process id, when known.
    pub ppid: Option<u32>,
    /// Executable path, when the OS reports it.
    pub exe: Option<String>,
    /// The process name (`Claude`, `claude`, `node`, …).
    pub name: String,
    /// The first command-line word and the rest, joined.
    pub cmd: String,
    /// `CLAUDE_CONFIG_DIR` from its environment, if set.
    pub config_dir: Option<String>,
}

/// Where a setup is in use: `desktop` (the Claude desktop app) and/or
/// `terminal` (`claude` outside it).
pub type InUse = HashMap<PathBuf, BTreeSet<&'static str>>;

fn is_desktop_main(process: &ProcFacts) -> bool {
    process
        .exe
        .as_deref()
        .is_some_and(|exe| exe.ends_with("/Claude.app/Contents/MacOS/Claude"))
}

fn inside_desktop_app(process: &ProcFacts) -> bool {
    process
        .exe
        .as_deref()
        .is_some_and(|exe| exe.contains("/Claude.app/"))
}

fn is_cli(process: &ProcFacts) -> bool {
    process.name == "claude"
        || process
            .exe
            .as_deref()
            .is_some_and(|exe| exe.ends_with("/claude"))
        || process.cmd.contains("@anthropic-ai/claude-code")
}

fn normalized(path: &str, home: &Path) -> PathBuf {
    let expanded = path
        .strip_prefix("~/")
        .map_or_else(|| PathBuf::from(path), |rest| home.join(rest));
    let trimmed = PathBuf::from(expanded.to_string_lossy().trim_end_matches('/'));
    std::fs::canonicalize(&trimmed).unwrap_or(trimmed)
}

/// One running Claude tool: the desktop app's main process, or `claude`
/// outside it (AJ-E1402 measures each as a tree).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningTool {
    /// The tree's root pid.
    pub pid: u32,
    /// `desktop` or `terminal`.
    pub place: &'static str,
    /// The config folder it uses, normalised.
    pub config_dir: PathBuf,
}

/// Pure: the running Claude tools, one per tree. A `claude` started by
/// another `claude` (or inside the desktop app) is part of that tree,
/// never a second root.
#[must_use]
pub fn tools(processes: &[ProcFacts], home: &Path) -> Vec<RunningTool> {
    let by_pid: HashMap<u32, &ProcFacts> = processes.iter().map(|p| (p.pid, p)).collect();
    let has_ancestor = |process: &ProcFacts, test: &dyn Fn(&ProcFacts) -> bool| {
        let mut seen = 0;
        let mut next = process.ppid;
        while let Some(pid) = next {
            seen += 1;
            if seen > 64 {
                return false; // ppid cycles must not loop forever
            }
            let Some(parent) = by_pid.get(&pid) else {
                return false;
            };
            if test(parent) {
                return true;
            }
            next = parent.ppid;
        }
        false
    };
    let default_dir = home.join(".claude");
    let mut out = Vec::new();
    for process in processes {
        let place = if is_desktop_main(process) {
            "desktop"
        } else if is_cli(process)
            && !inside_desktop_app(process)
            && !has_ancestor(process, &inside_desktop_app)
            && !has_ancestor(process, &|parent| {
                is_cli(parent) && !inside_desktop_app(parent)
            })
        {
            "terminal"
        } else {
            continue;
        };
        let config_dir = process
            .config_dir
            .as_deref()
            .filter(|dir| !dir.is_empty())
            .map_or_else(
                || normalized(&default_dir.to_string_lossy(), home),
                |dir| normalized(dir, home),
            );
        out.push(RunningTool {
            pid: process.pid,
            place,
            config_dir,
        });
    }
    out
}

/// Pure: which setup each running Claude process uses, and from where.
#[must_use]
pub fn classify(processes: &[ProcFacts], home: &Path) -> InUse {
    let mut out: InUse = HashMap::new();
    for tool in tools(processes, home) {
        out.entry(tool.config_dir).or_default().insert(tool.place);
    }
    out
}

/// Whether `path` is the folder `tool` uses (both normalised the same
/// way).
#[must_use]
pub fn uses_folder(tool: &RunningTool, path: &str, home: &Path) -> bool {
    tool.config_dir == normalized(path, home)
}

/// The live process table, reduced to [`ProcFacts`].
#[must_use]
pub fn running_processes() -> Vec<ProcFacts> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always)
            .with_environ(UpdateKind::Always),
    );
    system
        .processes()
        .values()
        .map(|process| ProcFacts {
            pid: process.pid().as_u32(),
            ppid: process.parent().map(sysinfo::Pid::as_u32),
            exe: process.exe().map(|exe| exe.to_string_lossy().into_owned()),
            name: process.name().to_string_lossy().into_owned(),
            cmd: process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
            config_dir: process.environ().iter().find_map(|entry| {
                entry
                    .to_str()
                    .and_then(|entry| entry.strip_prefix("CLAUDE_CONFIG_DIR="))
                    .map(str::to_owned)
            }),
        })
        .collect()
}

/// AJ-E1402: the running tools from a process list WITHOUT environments
/// (the collector's table), then `CLAUDE_CONFIG_DIR` read for the roots
/// alone — a handful of pids, not every process on the Mac.
#[must_use]
pub fn tools_from(briefs: &[juggler_supervisor::ProcessBrief], home: &Path) -> Vec<RunningTool> {
    let mut processes: Vec<ProcFacts> = briefs
        .iter()
        .map(|brief| ProcFacts {
            pid: brief.pid,
            ppid: brief.ppid,
            exe: brief.exe.clone(),
            name: brief.name.clone(),
            cmd: brief.cmd.clone(),
            config_dir: None,
        })
        .collect();
    let roots: Vec<u32> = tools(&processes, home)
        .iter()
        .map(|tool| tool.pid)
        .collect();
    if roots.is_empty() {
        return Vec::new();
    }
    let dirs = config_dirs_of(&roots);
    for process in &mut processes {
        if let Some(dir) = dirs.get(&process.pid) {
            process.config_dir = Some(dir.clone());
        }
    }
    tools(&processes, home)
}

/// `CLAUDE_CONFIG_DIR` of the given pids — nothing else of their
/// environments is kept; the table that read them is dropped at once.
fn config_dirs_of(pids: &[u32]) -> HashMap<u32, String> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let targets: Vec<Pid> = pids.iter().map(|pid| Pid::from_u32(*pid)).collect();
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&targets),
        false,
        ProcessRefreshKind::nothing().with_environ(UpdateKind::Always),
    );
    system
        .processes()
        .values()
        .filter_map(|process| {
            process.environ().iter().find_map(|entry| {
                entry
                    .to_str()
                    .and_then(|entry| entry.strip_prefix("CLAUDE_CONFIG_DIR="))
                    .map(|dir| (process.pid().as_u32(), dir.to_owned()))
            })
        })
        .collect()
}

/// Where `path` is in use right now (`desktop`, `terminal`), sorted.
#[must_use]
pub fn in_use_for(map: &InUse, path: &str, home: &Path) -> Vec<String> {
    map.get(&normalized(path, home))
        .map(|places| places.iter().map(|place| (*place).to_owned()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: &str = "/Applications/Claude.app/Contents/MacOS/Claude";

    fn proc(pid: u32, ppid: u32, name: &str, exe: &str, config: Option<&str>) -> ProcFacts {
        ProcFacts {
            pid,
            ppid: Some(ppid),
            exe: Some(exe.to_owned()),
            name: name.to_owned(),
            cmd: exe.to_owned(),
            config_dir: config.map(str::to_owned),
        }
    }

    #[test]
    fn desktop_windows_map_to_their_profile_and_the_default_folder() {
        let home = Path::new("/Users/you");
        let profiles = "/Users/you/Library/Application Support/ClaudeCodeProfiles";
        let processes = vec![
            proc(10, 1, "Claude", APP, Some(&format!("{profiles}/work-a"))),
            proc(
                11,
                1,
                "Claude",
                APP,
                Some(&format!("{profiles}/personal-max/")),
            ),
            // The sixth window: no override — the default ~/.claude.
            proc(12, 1, "Claude", APP, None),
            // The desktop app's own `claude` child is still the desktop.
            proc(
                13,
                10,
                "disclaimer",
                "/Applications/Claude.app/Contents/Helpers/disclaimer",
                None,
            ),
            proc(
                14,
                13,
                "claude",
                "/Users/you/.local/bin/claude",
                Some(&format!("{profiles}/work-a")),
            ),
        ];
        let map = classify(&processes, home);
        assert_eq!(
            in_use_for(&map, &format!("{profiles}/work-a"), home),
            ["desktop"]
        );
        assert_eq!(
            in_use_for(&map, &format!("{profiles}/personal-max"), home),
            ["desktop"]
        );
        assert_eq!(
            in_use_for(&map, "/Users/you/.claude", home),
            ["desktop"],
            "the default folder is the desktop app's too"
        );
    }

    #[test]
    fn a_terminal_claude_is_terminal_and_a_folder_can_be_both() {
        let home = Path::new("/Users/you");
        let processes = vec![
            proc(20, 1, "zsh", "/bin/zsh", None),
            proc(
                21,
                20,
                "claude",
                "/Users/you/.local/bin/claude",
                Some("~/.claude-work"),
            ),
            proc(22, 20, "node", "/usr/local/bin/node", None),
            proc(30, 1, "Claude", APP, None),
            proc(31, 20, "claude", "/Users/you/.local/bin/claude", None),
        ];
        let mut with_npm = processes.clone();
        with_npm[2].cmd =
            "node /usr/local/lib/node_modules/@anthropic-ai/claude-code/cli.js".to_owned();
        let map = classify(&with_npm, home);
        assert_eq!(
            in_use_for(&map, "/Users/you/.claude-work", home),
            ["terminal"]
        );
        assert_eq!(
            in_use_for(&map, "/Users/you/.claude", home),
            ["desktop", "terminal"]
        );
        assert!(
            in_use_for(&classify(&processes[..1], home), "/Users/you/.claude", home).is_empty()
        );
    }
}

/// AJ-E1402: one root per tree.
#[cfg(test)]
mod tool_roots {
    use super::*;

    fn proc(pid: u32, ppid: u32, name: &str, exe: &str, config: Option<&str>) -> ProcFacts {
        ProcFacts {
            pid,
            ppid: Some(ppid),
            exe: Some(exe.to_owned()),
            name: name.to_owned(),
            cmd: exe.to_owned(),
            config_dir: config.map(str::to_owned),
        }
    }

    #[test]
    fn nested_claudes_and_the_desktop_apps_own_claude_are_not_extra_roots() {
        let home = Path::new("/Users/you");
        let app = "/Applications/Claude.app/Contents/MacOS/Claude";
        let cli = "/Users/you/.local/bin/claude";
        let processes = vec![
            proc(10, 1, "Claude", app, None),
            proc(11, 10, "claude", cli, None), // the desktop app's own
            proc(20, 1, "zsh", "/bin/zsh", None),
            proc(21, 20, "claude", cli, Some("/Users/you/.claude-work")),
            proc(22, 21, "zsh", "/bin/zsh", None),
            proc(23, 22, "claude", cli, None), // `claude -p` run by a claude
        ];
        let roots = tools(&processes, home);
        let pids: Vec<(u32, &str)> = roots.iter().map(|tool| (tool.pid, tool.place)).collect();
        assert_eq!(pids, [(10, "desktop"), (21, "terminal")]);
        assert!(uses_folder(&roots[1], "/Users/you/.claude-work/", home));
        assert!(uses_folder(&roots[0], "~/.claude", home));
    }
}

/// A by-hand probe on a real Mac (read-only): which setups are open now.
/// `cargo test -p juggler-app --lib running_setups::live -- --ignored --nocapture`
#[cfg(test)]
mod live {
    #[test]
    #[ignore = "reads this Mac's real process table; run by hand"]
    fn print_open_setups() {
        let home = dirs::home_dir().unwrap_or_default();
        let map = super::classify(&super::running_processes(), &home);
        let mut rows: Vec<_> = map.iter().collect();
        rows.sort();
        for (dir, places) in rows {
            println!(
                "{} → {:?}",
                dir.display()
                    .to_string()
                    .replace(&home.display().to_string(), "~"),
                places
            );
        }
    }
}
