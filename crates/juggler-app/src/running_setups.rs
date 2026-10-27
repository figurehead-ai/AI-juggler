//! Which AI tools' setups are open right now, and where.
//!
//! A setup is a config folder (`~/.claude`, a profile manager's profile,
//! a `CLAUDE_CONFIG_DIR` target, a Codex home); the same folder can be
//! used by its tool's desktop app, by the tool in a terminal, or by both.
//! First run used to guess the tool from the folder's location and called
//! the default `~/.claude` "Terminal" — on the maintainer's Mac it was the
//! sixth Claude desktop window (0.0.6 dogfood). This reads the running
//! processes instead.
//!
//! Tools are data (ADR-0019): which processes are a tool's desktop app,
//! and which are its command-line sessions, come from the adapters (their
//! apps' bundles, their session rule), so the Claude app, the Codex app
//! inside `ChatGPT.app`, Claude Code and Codex in a terminal are found the
//! same way (the maintainer's report, 0.1.6: the Codex app was missing from
//! This Mac). A tool's own command line inside its app (the Claude app's
//! `claude`, the command line the Codex app ships) is the app's, never a root.
//!
//! Privacy (D-37, AJ-E2101): no process's environment is read, only names,
//! executables, parents and start times. Which folder a process uses comes
//! from where its own session file sits (`<folder>/sessions/<pid>.json`,
//! matched by pid and start time); a desktop app's folder from the session
//! files of the sessions it runs. A process with no such file has no known
//! folder, never a guessed one.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use juggler_adapter::live::{HostApp, HostKind};

use crate::features::conversations::places::outer_bundle;

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
    /// The folder its session file sits in, when one names its pid
    /// (never read from its environment).
    pub config_dir: Option<String>,
    /// Kernel start time, epoch seconds, when known (pins a measured root
    /// against pid reuse).
    pub start_time: Option<i64>,
}

/// Where a setup is in use: `desktop` (its tool's desktop app) and/or
/// `terminal` (the tool outside it).
pub type InUse = HashMap<PathBuf, BTreeSet<&'static str>>;

/// A tool's desktop app, as its adapter describes it, with its words.
#[derive(Clone, Debug)]
struct AppShape {
    tool: String,
    app: HostApp,
    label: String,
    who: String,
}

/// A tool's command-line sessions: its adapter's rule, and their words.
#[derive(Clone)]
struct CliShape {
    tool: String,
    adapter: juggler_adapter::SharedAdapter,
    label: String,
}

/// What the adapters say their tools' processes look like: each tool's own
/// desktop apps (by bundle) and its rule for a command-line session.
/// Nothing here names a tool.
#[derive(Clone, Default)]
pub struct ToolShapes {
    apps: Vec<AppShape>,
    clis: Vec<CliShape>,
}

impl ToolShapes {
    /// The shapes of every tool in `registry`.
    #[must_use]
    pub fn of(registry: &juggler_adapter::AdapterRegistry) -> Self {
        let adapters: Vec<juggler_adapter::SharedAdapter> = registry
            .ids()
            .iter()
            .filter_map(|id| registry.get(id))
            .collect();
        Self::from_adapters(&adapters)
    }

    /// The shapes of `adapters`' tools, in their order.
    #[must_use]
    pub fn from_adapters(adapters: &[juggler_adapter::SharedAdapter]) -> Self {
        let mut shapes = Self::default();
        for adapter in adapters {
            let spec = adapter.spec();
            for app in adapter
                .host_apps()
                .into_iter()
                .filter(|app| app.kind == HostKind::Desktop)
            {
                let (label, who) = app_words(&spec, &app);
                shapes.apps.push(AppShape {
                    tool: spec.id.clone(),
                    app,
                    label,
                    who,
                });
            }
            let command = adapter
                .shell_command()
                .map_or_else(|| spec.display_name.clone(), str::to_owned);
            shapes.clis.push(CliShape {
                tool: spec.id.clone(),
                adapter: std::sync::Arc::clone(adapter),
                label: format!("{command} in a terminal"),
            });
        }
        shapes
    }

    /// The bundles of the tools' desktop apps (`Claude.app`), each once.
    #[must_use]
    pub fn app_bundles(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for shape in &self.apps {
            if !out.contains(&shape.app.bundle) {
                out.push(shape.app.bundle.clone());
            }
        }
        out
    }

    /// The app whose main executable `exe` is: directly in its bundle's
    /// `Contents/MacOS` (its helpers, and a tool's command line it ships,
    /// sit deeper).
    fn app_main(&self, exe: &str) -> Option<&AppShape> {
        let exe = Path::new(exe);
        let (dir, bundle) = outer_bundle(exe)?;
        if exe.parent() != Some(dir.join("Contents/MacOS").as_path()) {
            return None;
        }
        self.apps.iter().find(|shape| shape.app.bundle == bundle)
    }

    /// Whether `exe` sits anywhere inside a tool's desktop app.
    fn in_app(&self, exe: &str) -> bool {
        outer_bundle(Path::new(exe))
            .is_some_and(|(_, bundle)| self.apps.iter().any(|shape| shape.app.bundle == bundle))
    }

    /// The tool whose command-line session `process` is, by its adapter's
    /// rule (names, executable and arguments only).
    fn cli(&self, process: &ProcFacts) -> Option<&CliShape> {
        let exe = process.exe.as_deref().map(Path::new);
        let args: Vec<String> = process.cmd.split_whitespace().map(str::to_owned).collect();
        self.clis
            .iter()
            .find(|shape| shape.adapter.session_process(exe, &args).is_some())
    }
}

/// An app's words on This Mac: "Claude app" when its tool is named after
/// it (the Claude app runs Claude Code), else the app with the tool it
/// holds (`ChatGPT (Codex)`: the Codex app ships inside `ChatGPT.app`, and
/// the Dock calls it `ChatGPT`). With how an alert names it.
fn app_words(spec: &juggler_adapter::ToolSpec, app: &HostApp) -> (String, String) {
    if spec.display_name.starts_with(&app.name) {
        let label = format!("{} app", app.name);
        let who = format!("The {label}");
        (label, who)
    } else {
        let label = format!("{} ({})", app.name, spec.display_name);
        (label.clone(), label)
    }
}

fn normalized(path: &str, home: &Path) -> PathBuf {
    let expanded = path
        .strip_prefix("~/")
        .map_or_else(|| PathBuf::from(path), |rest| home.join(rest));
    let trimmed = PathBuf::from(expanded.to_string_lossy().trim_end_matches('/'));
    std::fs::canonicalize(&trimmed).unwrap_or(trimmed)
}

/// One running AI tool: a desktop app's main process, or a tool's command
/// line outside it (AJ-E1402 measures each as a tree).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningTool {
    /// The tree's root pid.
    pub pid: u32,
    /// `desktop` or `terminal`.
    pub place: &'static str,
    /// The tool's id (its adapter's).
    pub tool: String,
    /// The group This Mac lists it under: its app's id (`claude-app`), or
    /// `terminal:<tool>` for the tool outside its app.
    pub group: String,
    /// It in words ("Claude app", `ChatGPT (Codex)`, "codex in a
    /// terminal").
    pub label: String,
    /// How an alert names it ("The Claude app").
    pub who: String,
    /// The config folder it uses, normalised, when its session files say
    /// (`None`: not known, never guessed).
    pub config_dir: Option<PathBuf>,
    /// The root's kernel start time, when known.
    pub start_time: Option<i64>,
}

/// Pure: the running AI tools, one per tree. A tool's command line started
/// by another (or inside a tool's desktop app) is part of that tree, never
/// a second root.
#[must_use]
pub fn tools(processes: &[ProcFacts], shapes: &ToolShapes, home: &Path) -> Vec<RunningTool> {
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
    let in_app = |process: &ProcFacts| process.exe.as_deref().is_some_and(|exe| shapes.in_app(exe));
    let mut out = Vec::new();
    for process in processes {
        let (place, tool, group, label, who) =
            if let Some(app) = process.exe.as_deref().and_then(|exe| shapes.app_main(exe)) {
                (
                    "desktop",
                    app.tool.clone(),
                    app.app.id.clone(),
                    app.label.clone(),
                    app.who.clone(),
                )
            } else if let Some(cli) = shapes.cli(process).filter(|_| {
                !in_app(process)
                    && !has_ancestor(process, &in_app)
                    && !has_ancestor(process, &|parent| {
                        shapes.cli(parent).is_some() && !in_app(parent)
                    })
            }) {
                (
                    "terminal",
                    cli.tool.clone(),
                    format!("terminal:{}", cli.tool),
                    cli.label.clone(),
                    cli.label.clone(),
                )
            } else {
                continue;
            };
        let config_dir = process
            .config_dir
            .as_deref()
            .filter(|dir| !dir.is_empty())
            .map(|dir| normalized(dir, home));
        out.push(RunningTool {
            pid: process.pid,
            place,
            tool,
            group,
            label,
            who,
            config_dir,
            start_time: process.start_time,
        });
    }
    out
}

/// Pure: which setup each running tool uses, and from where.
#[must_use]
pub fn classify(processes: &[ProcFacts], shapes: &ToolShapes, home: &Path) -> InUse {
    let mut out: InUse = HashMap::new();
    for tool in tools(processes, shapes, home) {
        if let Some(dir) = tool.config_dir {
            out.entry(dir).or_default().insert(tool.place);
        }
    }
    out
}

/// Whether `path` is the folder `tool` uses (both normalised the same
/// way).
#[must_use]
pub fn uses_folder(tool: &RunningTool, path: &str, home: &Path) -> bool {
    tool.config_dir.as_deref() == Some(normalized(path, home).as_path())
}

/// Sets each process's folder from the session files that name its pid
/// (`pid_dirs`, already matched by start time), and a desktop app's from
/// the first session it runs that has one.
pub fn with_folders(
    processes: &mut [ProcFacts],
    shapes: &ToolShapes,
    pid_dirs: &BTreeMap<u32, PathBuf>,
) {
    let parents: HashMap<u32, Option<u32>> = processes.iter().map(|p| (p.pid, p.ppid)).collect();
    let mains: Vec<u32> = processes
        .iter()
        .filter(|process| {
            process
                .exe
                .as_deref()
                .is_some_and(|exe| shapes.app_main(exe).is_some())
        })
        .map(|process| process.pid)
        .collect();
    let mut desktop: HashMap<u32, String> = HashMap::new();
    for (pid, dir) in pid_dirs {
        let mut next = parents.get(pid).copied().flatten();
        for _ in 0..64 {
            let Some(at) = next else { break };
            if mains.contains(&at) {
                desktop
                    .entry(at)
                    .or_insert_with(|| dir.to_string_lossy().into_owned());
                break;
            }
            next = parents.get(&at).copied().flatten();
        }
    }
    for process in processes.iter_mut() {
        if let Some(dir) = pid_dirs.get(&process.pid) {
            process.config_dir = Some(dir.to_string_lossy().into_owned());
        } else if let Some(dir) = desktop.get(&process.pid) {
            process.config_dir = Some(dir.clone());
        }
    }
}

/// The pids whose session files sit in one of `dirs` (each tool's own
/// files, through its adapter), matched by start time against
/// `processes`: pid → folder.
#[must_use]
pub fn pid_dirs_in(
    adapters: &[juggler_adapter::SharedAdapter],
    dirs: &[PathBuf],
    processes: &[ProcFacts],
) -> BTreeMap<u32, PathBuf> {
    let starts: HashMap<u32, Option<i64>> =
        processes.iter().map(|p| (p.pid, p.start_time)).collect();
    let mut out = BTreeMap::new();
    for dir in dirs {
        for adapter in adapters {
            for session in adapter.live_sessions(dir) {
                let Some(pid) = session.pid else { continue };
                if let Some(start) = starts.get(&pid)
                    && juggler_adapter::live::same_start(session.started_secs, *start)
                {
                    out.insert(pid, dir.clone());
                }
            }
        }
    }
    out
}

/// The live process table, reduced to [`ProcFacts`]: executables,
/// arguments, parents and start times, never environments (the live
/// view's own read, `features::conversations::procs`).
#[must_use]
pub fn running_processes() -> Vec<ProcFacts> {
    crate::features::conversations::procs::table()
        .into_values()
        .map(|process| ProcFacts {
            pid: process.pid,
            ppid: process.ppid,
            name: process
                .exe
                .as_deref()
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            exe: process.exe.map(|exe| exe.to_string_lossy().into_owned()),
            cmd: process.args.join(" "),
            config_dir: None,
            start_time: process.start,
        })
        .collect()
}

/// AJ-E1402: the running tools from the collector's process list, each
/// with the folder its session files name (`pid_dirs`, from the live
/// view; AJ-E2101: never an environment).
#[must_use]
pub fn tools_from(
    briefs: &[juggler_supervisor::ProcessBrief],
    shapes: &ToolShapes,
    home: &Path,
    pid_dirs: &BTreeMap<u32, PathBuf>,
) -> Vec<RunningTool> {
    let mut processes: Vec<ProcFacts> = briefs
        .iter()
        .map(|brief| ProcFacts {
            pid: brief.pid,
            ppid: brief.ppid,
            exe: brief.exe.clone(),
            name: brief.name.clone(),
            cmd: brief.cmd.clone(),
            config_dir: None,
            start_time: Some(brief.start_time),
        })
        .collect();
    with_folders(&mut processes, shapes, pid_dirs);
    tools(&processes, shapes, home)
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

    /// The built-in adapters' shapes (Claude Code's and Codex's).
    pub(super) fn shapes() -> ToolShapes {
        ToolShapes::of(&crate::state::default_registry())
    }

    const APP: &str = "/Applications/Claude.app/Contents/MacOS/Claude";

    fn proc(pid: u32, ppid: u32, name: &str, exe: &str, config: Option<&str>) -> ProcFacts {
        ProcFacts {
            pid,
            ppid: Some(ppid),
            exe: Some(exe.to_owned()),
            name: name.to_owned(),
            cmd: exe.to_owned(),
            config_dir: config.map(str::to_owned),
            start_time: None,
        }
    }

    #[test]
    fn desktop_windows_map_to_the_folder_their_session_files_name() {
        let home = Path::new("/Users/you");
        let profiles = "/Users/you/Library/Application Support/ClaudeCodeProfiles";
        let mut processes = vec![
            proc(10, 1, "Claude", APP, None),
            proc(11, 1, "Claude", APP, None),
            // The sixth window runs no session: its folder isn't known, and
            // is never guessed (the 0.1 code read its environment).
            proc(12, 1, "Claude", APP, None),
            // The desktop app's own `claude` child is still the desktop.
            proc(
                13,
                10,
                "disclaimer",
                "/Applications/Claude.app/Contents/Helpers/disclaimer",
                None,
            ),
            proc(14, 13, "claude", "/Users/you/.local/bin/claude", None),
            proc(15, 11, "claude", "/Users/you/.local/bin/claude", None),
        ];
        let pid_dirs = BTreeMap::from([
            (14, PathBuf::from(format!("{profiles}/work-a"))),
            (15, PathBuf::from(format!("{profiles}/personal-max/"))),
        ]);
        with_folders(&mut processes, &shapes(), &pid_dirs);
        let map = classify(&processes, &shapes(), home);
        assert_eq!(
            in_use_for(&map, &format!("{profiles}/work-a"), home),
            ["desktop"]
        );
        assert_eq!(
            in_use_for(&map, &format!("{profiles}/personal-max"), home),
            ["desktop"]
        );
        assert!(
            in_use_for(&map, "/Users/you/.claude", home).is_empty(),
            "no folder is guessed for a window with no session"
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
            proc(32, 30, "claude", "/Users/you/.local/bin/claude", None),
        ];
        let mut with_npm = processes.clone();
        with_npm[2].cmd =
            "node /usr/local/lib/node_modules/@anthropic-ai/claude-code/cli.js".to_owned();
        let pid_dirs = BTreeMap::from([
            (31, PathBuf::from("/Users/you/.claude")),
            (32, PathBuf::from("/Users/you/.claude")),
        ]);
        with_folders(&mut with_npm, &shapes(), &pid_dirs);
        let map = classify(&with_npm, &shapes(), home);
        assert_eq!(
            in_use_for(&map, "/Users/you/.claude-work", home),
            ["terminal"]
        );
        assert_eq!(
            in_use_for(&map, "/Users/you/.claude", home),
            ["desktop", "terminal"]
        );
        assert!(
            in_use_for(
                &classify(&processes[..1], &shapes(), home),
                "/Users/you/.claude",
                home
            )
            .is_empty()
        );
    }

    /// AJ-E2101: a session file counts only for the process that wrote it
    /// (its start time), never a later process on the same pid.
    #[test]
    fn a_session_file_names_its_pid_only_while_the_start_time_matches() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        std::fs::create_dir_all(&sessions).unwrap();
        std::fs::write(
            sessions.join("41.json"),
            r#"{"pid":41,"sessionId":"s","procStart":"Sat Oct 24 08:12:34 2026"}"#,
        )
        .unwrap();
        let adapters: Vec<juggler_adapter::SharedAdapter> = vec![std::sync::Arc::new(
            juggler_adapters::ClaudeCodeAdapter::new(),
        )];
        let mut process = proc(41, 1, "claude", "/Users/you/.local/bin/claude", None);
        process.start_time = Some(1_792_829_554);
        let dirs = [dir.path().to_path_buf()];
        assert_eq!(
            pid_dirs_in(&adapters, &dirs, std::slice::from_ref(&process)).get(&41),
            Some(&dirs[0])
        );
        process.start_time = Some(1_792_829_554 + 600);
        assert!(pid_dirs_in(&adapters, &dirs, &[process]).is_empty());
    }
}

/// AJ-E1402: one root per tree.
#[cfg(test)]
mod tool_roots {
    use super::tests::shapes;
    use super::*;

    fn proc(pid: u32, ppid: u32, name: &str, exe: &str, config: Option<&str>) -> ProcFacts {
        ProcFacts {
            pid,
            ppid: Some(ppid),
            exe: Some(exe.to_owned()),
            name: name.to_owned(),
            cmd: exe.to_owned(),
            config_dir: config.map(str::to_owned),
            start_time: None,
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
        let roots = tools(&processes, &shapes(), home);
        let pids: Vec<(u32, &str)> = roots.iter().map(|tool| (tool.pid, tool.place)).collect();
        assert_eq!(pids, [(10, "desktop"), (21, "terminal")]);
        assert!(uses_folder(&roots[1], "/Users/you/.claude-work/", home));
        assert!(
            !uses_folder(&roots[0], "~/.claude", home),
            "a desktop window with no session names no folder"
        );
    }
}

/// QA on #126: Claude Code's executable run as its search helper
/// (`ugrep`), left behind by a finished session under an orphaned shell,
/// is not a `claude` in a terminal. A `claude` run by a path with spaces
/// still is.
#[cfg(test)]
mod helpers_by_another_name {
    use super::tests::shapes;
    use super::*;

    fn proc(pid: u32, ppid: u32, exe: &str, cmd: &str) -> ProcFacts {
        ProcFacts {
            pid,
            ppid: Some(ppid),
            exe: Some(exe.to_owned()),
            name: "claude".to_owned(),
            cmd: cmd.to_owned(),
            config_dir: None,
            start_time: None,
        }
    }

    #[test]
    fn a_claude_run_as_ugrep_is_not_a_tool() {
        let home = Path::new("/Users/you");
        let copy = "/Users/you/Library/Application Support/ClaudeProfiles/b/claude-code/2.1/abc/claude.app/Contents/MacOS/claude";
        let processes = vec![
            ProcFacts {
                name: "zsh".into(),
                ..proc(30, 1, "/bin/zsh", "/bin/zsh -c ugrep")
            },
            proc(31, 30, copy, "ugrep -r --json needle ."),
            ProcFacts {
                name: "rg".into(),
                ..proc(32, 1, "/opt/homebrew/bin/rg", "rg needle")
            },
            proc(40, 1, copy, &format!("{copy} --resume")),
            proc(41, 1, "/Users/you/.local/bin/claude", "claude"),
        ];
        let pids: Vec<u32> = tools(&processes, &shapes(), home)
            .iter()
            .map(|tool| tool.pid)
            .collect();
        assert_eq!(pids, [40, 41]);
    }
}

/// The maintainer's report (0.1.6): the Codex app was running, and This
/// Mac listed only the Claude app and other apps. The Codex app ships
/// inside `ChatGPT.app`; its helpers, its bundled `codex` and the nodes it
/// runs are the app's. `claude` from the native installer and `codex` in a
/// terminal are each their own tool's (the lab, 2026-10-26).
#[cfg(test)]
mod every_tools_app {
    use super::tests::shapes;
    use super::*;

    fn proc(pid: u32, ppid: u32, exe: &str, cmd: &str) -> ProcFacts {
        ProcFacts {
            pid,
            ppid: Some(ppid),
            exe: Some(exe.to_owned()),
            name: Path::new(exe)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            cmd: cmd.to_owned(),
            config_dir: None,
            start_time: None,
        }
    }

    const CHATGPT: &str = "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT";
    const CODEX_FW: &str = "/Applications/ChatGPT.app/Contents/Frameworks/Codex Framework.framework/Versions/156.0.8078.48/Helpers";
    const BUNDLED_CODEX: &str =
        "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex";
    const CLAUDE_APP: &str = "/Applications/Claude.app/Contents/MacOS/Claude";

    /// A synthetic table shaped like the maintainer's Mac (names and
    /// parents only).
    fn table() -> Vec<ProcFacts> {
        vec![
            proc(100, 1, CHATGPT, CHATGPT),
            proc(
                101,
                100,
                &format!("{CODEX_FW}/Codex (Renderer).app/Contents/MacOS/Codex (Renderer)"),
                "Codex (Renderer) --type=renderer",
            ),
            proc(
                102,
                100,
                &format!("{CODEX_FW}/Codex (Service).app/Contents/MacOS/Codex (Service)"),
                "Codex (Service)",
            ),
            proc(
                103,
                100,
                BUNDLED_CODEX,
                &format!("{BUNDLED_CODEX} app-server"),
            ),
            proc(
                104,
                103,
                "/Applications/ChatGPT.app/Contents/Resources/cua_node/bin/node",
                "node",
            ),
            // The bundled CLI run as a session, still inside the app.
            proc(105, 100, BUNDLED_CODEX, &format!("{BUNDLED_CODEX} exec hi")),
            // A node the app runs from its support folder.
            proc(
                106,
                103,
                "/Users/you/Library/Application Support/Codex/runtime/node",
                "node server.js",
            ),
            proc(200, 1, CLAUDE_APP, CLAUDE_APP),
            proc(
                201,
                200,
                "/Applications/Claude.app/Contents/Helpers/disclaimer",
                "disclaimer",
            ),
            proc(
                202,
                201,
                "/Users/you/Library/Application Support/Claude/claude-code/2.1.299/abc/claude.app/Contents/MacOS/claude",
                "claude --output-format stream-json",
            ),
            // Terminal: the native installer's claude, and codex.
            proc(
                300,
                1,
                "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal",
                "Terminal",
            ),
            proc(301, 300, "/bin/zsh", "-zsh"),
            proc(
                302,
                301,
                "/Users/you/.local/share/claude/versions/2.1.310",
                "claude",
            ),
            proc(303, 301, "/opt/homebrew/bin/codex", "codex"),
            // codex started by that claude (a plugin's): the claude's tree.
            proc(304, 302, "/opt/homebrew/bin/codex", "codex exec review"),
        ]
    }

    #[test]
    fn each_tools_app_and_command_line_is_its_own_root() {
        let roots = tools(&table(), &shapes(), Path::new("/Users/you"));
        let got: Vec<(u32, &str, &str, &str, &str)> = roots
            .iter()
            .map(|tool| {
                (
                    tool.pid,
                    tool.place,
                    tool.tool.as_str(),
                    tool.group.as_str(),
                    tool.label.as_str(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (100, "desktop", "codex", "codex-app", "ChatGPT (Codex)"),
                (200, "desktop", "claude-code", "claude-app", "Claude app"),
                (
                    302,
                    "terminal",
                    "claude-code",
                    "terminal:claude-code",
                    "claude in a terminal"
                ),
                (
                    303,
                    "terminal",
                    "codex",
                    "terminal:codex",
                    "codex in a terminal"
                ),
            ]
        );
        assert_eq!(roots[1].who, "The Claude app");
        assert_eq!(roots[0].who, "ChatGPT (Codex)");
    }

    #[test]
    fn the_apps_are_the_adapters_own_never_a_list_here() {
        let shapes = shapes();
        assert_eq!(shapes.app_bundles(), ["Claude.app", "ChatGPT.app"]);
        // With no adapter for a tool, its app is just another app.
        let none = ToolShapes::default();
        assert!(tools(&table(), &none, Path::new("/Users/you")).is_empty());
    }

    /// D-37: a running tool is found from names, executables, parents and
    /// start times; nothing else of a process is kept.
    #[test]
    fn a_running_tool_keeps_no_command_line() {
        let mut table = table();
        table[12].cmd = "claude --append-system-prompt PLANTED-PRIVATE".to_owned();
        let shown = format!("{:?}", tools(&table, &shapes(), Path::new("/Users/you")));
        assert!(!shown.contains("PLANTED"), "{shown}");
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
        let shapes = super::ToolShapes::of(&crate::state::default_registry());
        let map = super::classify(&super::running_processes(), &shapes, &home);
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
