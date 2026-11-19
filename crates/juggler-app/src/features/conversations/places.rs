//! AJ-E2101/E2102 (ADR-0019): where a session runs, from its process's
//! parent chain: the first ancestor inside an app bundle is its host app,
//! that app's outermost process on the chain is the one Go to it brings
//! forward, and the app's instance comes from that process's arguments
//! (the Claude app's `--user-data-dir`). Never from an environment
//! variable (`TERM_PROGRAM` is inherited and lies).
//!
//! Host apps are data: the adapters describe their tools' own apps
//! ([`juggler_adapter::ToolDiscovery::host_apps`]) and `hosts.json` the
//! terminals and editors. An app in neither is still a place, named by its
//! bundle.

use std::path::{Path, PathBuf};

use juggler_adapter::live::{HostApp, HostKind};

use super::procs::{Table, ancestors};

/// The terminals and editors a session can run in (data, not branches).
const BUILTIN: &str = include_str!("hosts.json");

/// Every host app known: the tools' own, then the terminals and editors;
/// and AI Juggler's own process, where every chain it is on ends.
#[derive(Clone, Debug, Default)]
pub struct HostRegistry {
    apps: Vec<HostApp>,
    /// F-rc3g-2: AI Juggler's own process, by its pid and start time
    /// ([`own_process`]). `None`: no chain ends early.
    own: Option<(u32, Option<i64>)>,
}

/// This process (AI Juggler's own): its pid and its start time, read once.
#[must_use]
pub fn own_process() -> (u32, Option<i64>) {
    static OWN: std::sync::OnceLock<(u32, Option<i64>)> = std::sync::OnceLock::new();
    *OWN.get_or_init(|| {
        let pid = std::process::id();
        let start = super::procs::some(&[pid])
            .remove(&pid)
            .and_then(|process| process.start);
        (pid, start)
    })
}

impl HostRegistry {
    /// The tools' apps (`tool_apps`, from the adapters) and the built-in
    /// terminals and editors; this process is AI Juggler's own.
    #[must_use]
    pub fn new(tool_apps: Vec<HostApp>) -> Self {
        let mut apps = tool_apps;
        // The list ships inside the app; a broken one is a build bug the
        // test below catches, never a crash here.
        apps.extend(serde_json::from_str::<Vec<HostApp>>(BUILTIN).unwrap_or_default());
        Self {
            apps,
            own: Some(own_process()),
        }
    }

    /// The same, with AI Juggler's own process `pid`, started at `start`
    /// (the tests' seam: a scripted table's AI Juggler); `None`, none.
    #[must_use]
    pub fn with_own(mut self, own: Option<(u32, i64)>) -> Self {
        self.own = own.map(|(pid, start)| (pid, Some(start)));
        self
    }

    /// Whether `process` is AI Juggler's own: its pid, started then (a
    /// start not read: the pid alone, which no other process holds while
    /// this one runs).
    fn is_own(&self, process: &super::procs::Proc) -> bool {
        self.own.is_some_and(|(pid, start)| {
            process.pid == pid && juggler_adapter::live::same_start(start, process.start)
        })
    }

    /// The registry over every adapter in `registry`.
    #[must_use]
    pub fn of(registry: &juggler_adapter::AdapterRegistry) -> Self {
        let apps = registry
            .ids()
            .iter()
            .filter_map(|id| registry.get(id))
            .flat_map(|adapter| adapter.host_apps())
            .collect();
        Self::new(apps)
    }

    /// The app whose bundle folder is `bundle` (`Terminal.app`).
    #[must_use]
    pub fn by_bundle(&self, bundle: &str) -> Option<&HostApp> {
        self.apps.iter().find(|app| app.bundle == bundle)
    }

    /// The app with id `id`.
    #[must_use]
    pub fn by_id(&self, id: &str) -> Option<&HostApp> {
        self.apps.iter().find(|app| app.id == id)
    }
}

/// Where a session runs: its host app's process and instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Host {
    /// The registry's id for the app, when it is one it knows.
    pub app_id: Option<String>,
    /// The app's name (the Dock's, or its bundle's).
    pub name: String,
    /// D-62: its name on screen ("Claude app"); words only, `name` matches.
    pub shown: String,
    /// What kind of app, when known.
    pub kind: Option<HostKind>,
    /// The app's process on the chain (the one to bring forward).
    pub pid: u32,
    /// That process's start time.
    pub start: Option<i64>,
    /// The instance's data folder, for an app that runs several.
    pub instance_dir: Option<PathBuf>,
    /// The instance's name, when it isn't the app's default one.
    pub instance: Option<String>,
}

/// The outermost `.app` bundle an executable is inside, and its folder
/// name: `/Applications/Claude.app/Contents/Helpers/x` is
/// `/Applications/Claude.app`, `Claude.app`.
///
/// An `.app` inside a `.framework` that no app holds isn't an app the
/// user runs or quits: it is how a framework ships a program. Homebrew's
/// `python3` runs as `…/Python.framework/Versions/3.x/Resources/
/// Python.app/Contents/MacOS/Python`, a command-line program (processes
/// lab 2026-10-26, bug 3). So a `.framework` before any `.app` means none.
#[must_use]
pub fn outer_bundle(exe: &Path) -> Option<(PathBuf, String)> {
    let mut path = PathBuf::new();
    for part in exe.components() {
        path.push(part);
        let name = part.as_os_str().to_string_lossy();
        if name.ends_with(".framework") {
            return None;
        }
        if name.ends_with(".app") && name.len() > 4 {
            return Some((path, name.into_owned()));
        }
    }
    None
}

/// The value of `flag` in `args` (`--flag=value` or `--flag value`).
#[must_use]
pub fn arg_value(args: &[String], flag: &str) -> Option<String> {
    juggler_adapter::conversation::value_after(args, &[flag])
}

/// The host of the session running as `pid`: the first ancestor inside an
/// app bundle the registry knows (else the outermost app bundle on the
/// chain), then the outermost process of that bundle on the chain. `home` places
/// an app's default instance folder.
///
/// A known app wins over a nearer unknown bundle: a tool's own command
/// line can come as a bundle (the Claude app runs Claude Code from a
/// `claude.app` of its own), and a session it starts (Codex's server for
/// Claude Code's Codex plugin) runs in the app that runs that tool, never
/// in a place named after the tool. With no known app, the outermost
/// bundle is the app the user sees (a terminal, not a tool inside it).
///
/// F-rc3g-2: the chain ends at AI Juggler's own process (by its pid and
/// start time, the registry's). A session AI Juggler runs (one it drives,
/// its terminal's shell) runs in AI Juggler: what is above AI Juggler is
/// only where AI Juggler itself was started from. The 0.4.6 rc3g walk
/// started it from a shell in a Claude app profile's Code tab; the Claude
/// app, a known app, won over AI Juggler's own unknown bundle, and a
/// session it drove read "Window: Claude app · (that profile)" (Go to it
/// would have brought that window forward, and the profile's App recorded
/// the conversation as its own) until the shell ended and launchd took AI
/// Juggler over.
#[must_use]
pub fn host_of(pid: u32, table: &Table, hosts: &HostRegistry, home: &Path) -> Option<Host> {
    let mut chain = ancestors(table, pid);
    if let Some(at) = chain.iter().position(|process| hosts.is_own(process)) {
        chain.truncate(at + 1);
    }
    let bundled: Vec<(usize, PathBuf, String)> = chain
        .iter()
        .enumerate()
        .filter_map(|(at, process)| {
            let (dir, name) = outer_bundle(process.exe.as_deref()?)?;
            Some((at, dir, name))
        })
        .collect();
    let (at, bundle_dir, bundle) = bundled
        .iter()
        .find(|(_, _, name)| hosts.by_bundle(name).is_some())
        .or_else(|| bundled.last())
        .cloned()?;
    // Up the chain while still inside the same bundle: the app's own
    // process, not a helper of it.
    let main = chain[at..]
        .iter()
        .take_while(|process| {
            process
                .exe
                .as_deref()
                .is_some_and(|exe| exe.starts_with(&bundle_dir))
        })
        .last()?;
    let app = hosts.by_bundle(&bundle);
    let instance_dir = app.and_then(|app| {
        let flag = app.instance_arg.as_deref()?;
        arg_value(&main.args, flag)
            .map(PathBuf::from)
            .or_else(|| app.default_instance.as_deref().map(|dir| home.join(dir)))
    });
    let default_dir = app
        .and_then(|app| app.default_instance.as_deref())
        .map(|dir| home.join(dir));
    let instance = instance_dir
        .as_ref()
        .filter(|dir| Some(*dir) != default_dir.as_ref())
        .and_then(|dir| dir.file_name())
        .map(|name| name.to_string_lossy().into_owned());
    // An app no adapter knows is named by its bundle.
    let unknown = || bundle.trim_end_matches(".app").to_owned();
    Some(Host {
        app_id: app.map(|app| app.id.clone()),
        name: app.map_or_else(unknown, |app| app.name.clone()),
        shown: app.map_or_else(unknown, |app| app.shown_name().to_owned()),
        kind: app.map(|app| app.kind),
        pid: main.pid,
        start: main.start,
        instance_dir,
        instance,
    })
}

/// A desktop app's main process running now (0.3.3): its bundle, and the
/// instance it runs as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningApp {
    /// The registry's id for the app.
    pub app_id: String,
    /// The bundle folder (`/Applications/Claude.app`).
    pub bundle: PathBuf,
    /// Its main executable (inside `Contents/MacOS`).
    pub exe: PathBuf,
    /// The main process.
    pub pid: u32,
    /// Its start time.
    pub start: Option<i64>,
    /// The instance folder it was started with (`--user-data-dir`), when
    /// it was given one: what starting it again passes.
    pub given: Option<PathBuf>,
    /// Its instance folder: the one given, else the app's default one.
    /// `None` for an app that runs one instance.
    pub instance: Option<PathBuf>,
}

/// The desktop apps the registry knows running now, by their main
/// process (an app's helpers carry its arguments too, as Chromium's do,
/// and never count): one per instance folder, the oldest process first.
#[must_use]
pub fn running_apps(table: &Table, hosts: &HostRegistry, home: &Path) -> Vec<RunningApp> {
    let mut found: Vec<RunningApp> = Vec::new();
    let mut processes: Vec<&super::procs::Proc> = table.values().collect();
    processes.sort_by_key(|process| (process.start, process.pid));
    for process in processes {
        let Some(exe) = process.exe.as_deref() else {
            continue;
        };
        let Some((dir, bundle)) = outer_bundle(exe) else {
            continue;
        };
        let Some(app) = hosts.by_bundle(&bundle) else {
            continue;
        };
        if app.kind != HostKind::Desktop || exe.parent() != Some(&dir.join("Contents/MacOS")) {
            continue;
        }
        let given = app
            .instance_arg
            .as_deref()
            .and_then(|flag| arg_value(&process.args, flag))
            .map(PathBuf::from);
        let instance = app.instance_arg.as_ref().and_then(|_| {
            given
                .clone()
                .or_else(|| app.default_instance.as_deref().map(|d| home.join(d)))
        });
        if found
            .iter()
            .any(|known| known.app_id == app.id && known.instance == instance)
        {
            continue;
        }
        found.push(RunningApp {
            app_id: app.id.clone(),
            bundle: dir,
            exe: exe.to_path_buf(),
            pid: process.pid,
            start: process.start,
            given,
            instance,
        });
    }
    found.sort_by(|a, b| (&a.app_id, &a.instance).cmp(&(&b.app_id, &b.instance)));
    found
}

/// The instances of each app that runs several, from its running main
/// processes (`--user-data-dir`, else its default folder): each app id with
/// its instance folders, every folder once.
#[must_use]
pub fn running_instances(
    table: &Table,
    hosts: &HostRegistry,
    home: &Path,
) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    for app in running_apps(table, hosts, home) {
        let Some(instance) = app.instance else {
            continue;
        };
        if !out.iter().any(|(_, known)| *known == instance) {
            out.push((app.app_id, instance));
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::super::procs::Proc;
    use super::*;

    const CLAUDE_APP: &str = "/Applications/Claude.app/Contents/MacOS/Claude";

    #[test]
    fn a_frameworks_program_is_no_app_but_an_apps_framework_is_the_apps() {
        assert_eq!(
            outer_bundle(Path::new(CLAUDE_APP)).map(|(_, name)| name),
            Some("Claude.app".to_owned())
        );
        // Homebrew's python3 (processes lab 2026-10-26, bug 3).
        let python = "/opt/homebrew/Cellar/python@3.14/3.14.0/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python";
        assert_eq!(outer_bundle(Path::new(python)), None);
        // A framework inside an app is still that app's.
        let helper = "/Applications/Claude.app/Contents/Frameworks/Electron Framework.framework/Helpers/chrome_crashpad_handler";
        assert_eq!(
            outer_bundle(Path::new(helper)).map(|(_, name)| name),
            Some("Claude.app".to_owned())
        );
        assert_eq!(outer_bundle(Path::new("/opt/homebrew/bin/node")), None);
    }

    fn proc(pid: u32, ppid: u32, exe: &str, args: &[&str]) -> (u32, Proc) {
        (
            pid,
            Proc {
                pid,
                ppid: Some(ppid),
                exe: Some(PathBuf::from(exe)),
                args: args.iter().map(|a| (*a).to_owned()).collect(),
                start: Some(i64::from(pid) * 10),
            },
        )
    }

    fn hosts() -> HostRegistry {
        HostRegistry::new(vec![crate::test_support::claude_app_host(Some(
            "com.anthropic.claudefordesktop",
        ))])
    }

    #[test]
    fn the_builtin_hosts_read() {
        let registry = HostRegistry::new(Vec::new());
        assert!(registry.by_bundle("Terminal.app").is_some());
        assert_eq!(
            registry
                .by_id("cursor")
                .and_then(|app| app.folder_link.clone())
                .as_deref(),
            Some("cursor://file/{folder}")
        );
    }

    #[test]
    fn a_desktop_session_is_in_its_claude_instance_by_its_argument() {
        let home = Path::new("/Users/you");
        let profile = "/Users/you/Library/Application Support/ClaudeProfiles/b-work";
        let table: Table = [
            proc(1, 0, "/sbin/launchd", &[]),
            proc(
                10,
                1,
                CLAUDE_APP,
                &[CLAUDE_APP, &format!("--user-data-dir={profile}")],
            ),
            proc(
                11,
                10,
                "/Applications/Claude.app/Contents/Helpers/disclaimer",
                &["disclaimer"],
            ),
            proc(
                12,
                11,
                &format!("{profile}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude"),
                &["claude", "--resume=s-1"],
            ),
            proc(20, 1, CLAUDE_APP, &[CLAUDE_APP]),
            proc(
                21,
                20,
                "/Applications/Claude.app/Contents/Helpers/disclaimer",
                &[],
            ),
            proc(
                22,
                21,
                "/Users/you/Library/Application Support/Claude/claude-code/c/claude",
                &["claude"],
            ),
        ]
        .into_iter()
        .collect();
        let host = host_of(12, &table, &hosts(), home).unwrap();
        assert_eq!(host.app_id.as_deref(), Some("claude-app"));
        assert_eq!(host.name, "Claude");
        assert_eq!(host.pid, 10, "the instance's own process, not its helper");
        assert_eq!(host.start, Some(100));
        assert_eq!(host.instance.as_deref(), Some("b-work"));
        assert_eq!(host.instance_dir.as_deref(), Some(Path::new(profile)));
        // No argument: the default instance, named by the app alone.
        let default = host_of(22, &table, &hosts(), home).unwrap();
        assert_eq!(default.pid, 20);
        assert_eq!(default.instance, None);
        assert_eq!(
            default.instance_dir.as_deref(),
            Some(Path::new("/Users/you/Library/Application Support/Claude"))
        );
        let instances = running_instances(&table, &hosts(), home);
        assert_eq!(instances.len(), 2);
    }

    /// D-38: a Codex server that Claude Code's Codex plugin started under
    /// a Claude app session is in that Claude instance, never in a place
    /// named after Claude Code's own `claude.app` (which drew Claude
    /// Code's mark on a Codex session).
    #[test]
    fn a_session_a_tool_starts_is_in_the_app_running_that_tool() {
        let home = Path::new("/Users/you");
        let profile = "/Users/you/Library/Application Support/ClaudeProfiles/b-work";
        let cli = format!("{profile}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude");
        let table: Table = [
            proc(1, 0, "/sbin/launchd", &[]),
            proc(
                10,
                1,
                CLAUDE_APP,
                &[CLAUDE_APP, &format!("--user-data-dir={profile}")],
            ),
            proc(
                11,
                10,
                "/Applications/Claude.app/Contents/Helpers/disclaimer",
                &["disclaimer"],
            ),
            proc(12, 11, &cli, &["claude", "--resume=s-1"]),
            proc(13, 12, "/opt/homebrew/bin/codex", &["codex", "app-server"]),
            // A terminal nobody knows still names its place, not the tool
            // bundled inside it.
            proc(20, 1, "/Applications/Some Term.app/Contents/MacOS/st", &[]),
            proc(21, 20, &cli, &["claude"]),
            proc(22, 21, "/opt/homebrew/bin/codex", &["codex"]),
        ]
        .into_iter()
        .collect();
        let host = host_of(13, &table, &hosts(), home).unwrap();
        assert_eq!(host.app_id.as_deref(), Some("claude-app"));
        assert_eq!(host.pid, 10);
        assert_eq!(host.instance.as_deref(), Some("b-work"));
        let unknown = host_of(22, &table, &hosts(), home).unwrap();
        assert_eq!(unknown.app_id, None);
        assert_eq!(
            unknown.name, "Some Term",
            "the outermost bundle, when none is known"
        );
        assert_eq!(unknown.pid, 20);
    }

    /// F-rc3g-2: AI Juggler started from a shell in a Claude app profile's
    /// Code tab (the rc3g walk's restart). What it runs is in AI Juggler,
    /// never in that profile's window: a session it drives, its terminal's
    /// shell, and a session of a Claude app profile it started itself (in
    /// that profile, below AI Juggler). A build in no bundle names no app.
    #[test]
    fn what_ai_juggler_runs_is_in_ai_juggler_whatever_started_it() {
        let home = Path::new("/Users/you");
        let profile = "/Users/you/Library/Application Support/ClaudeProfiles/b-work";
        let code_tab = format!("{profile}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude");
        let ours = "/Users/you/Applications/AI Juggler Dev.app/Contents/MacOS/juggler-app";
        let other = "/Users/you/Library/Application Support/ClaudeProfiles/b-other";
        let table: Table = [
            proc(1, 0, "/sbin/launchd", &[]),
            proc(
                10,
                1,
                CLAUDE_APP,
                &[CLAUDE_APP, &format!("--user-data-dir={profile}")],
            ),
            proc(
                11,
                10,
                "/Applications/Claude.app/Contents/Helpers/disclaimer",
                &[],
            ),
            proc(12, 11, &code_tab, &["claude"]),
            proc(13, 12, "/bin/zsh", &["/bin/zsh", "-c", "lab_env.sh"]),
            // AI Juggler, the shell's child until the shell ends.
            proc(70, 13, ours, &[ours]),
            // A session it drives, and one in its terminal.
            proc(
                71,
                70,
                "/Users/you/.local/bin/claude",
                &["claude", "-p", "--resume", "c-1"],
            ),
            proc(72, 70, "/bin/zsh", &["-zsh"]),
            proc(73, 72, "/Users/you/.local/bin/claude", &["claude"]),
            // A Claude app profile it started itself, and a session there.
            proc(
                80,
                70,
                CLAUDE_APP,
                &[CLAUDE_APP, &format!("--user-data-dir={other}")],
            ),
            proc(
                81,
                80,
                "/Applications/Claude.app/Contents/Helpers/disclaimer",
                &[],
            ),
            proc(
                82,
                81,
                &format!("{other}/claude-code/2.1/abc/claude.app/Contents/MacOS/claude"),
                &["claude"],
            ),
        ]
        .into_iter()
        .collect();
        let registry = hosts().with_own(Some((70, 700)));
        for session in [71, 73] {
            let host = host_of(session, &table, &registry, home).unwrap();
            assert_eq!(
                (host.app_id.as_deref(), host.name.as_str(), host.pid),
                (None, "AI Juggler Dev", 70),
                "{session}: in AI Juggler, not in the Claude app above it"
            );
            assert_eq!((host.instance, host.instance_dir), (None, None));
        }
        let started = host_of(82, &table, &registry, home).unwrap();
        assert_eq!(
            (started.app_id.as_deref(), started.pid),
            (Some("claude-app"), 80)
        );
        assert_eq!(started.instance.as_deref(), Some("b-other"));
        // The Code tab's own session is still that profile's.
        let tab = host_of(13, &table, &registry, home).unwrap();
        assert_eq!((tab.pid, tab.instance.as_deref()), (10, Some("b-work")));
        // A build in no bundle (a dev run): no app, never the one above.
        let mut bare = table.clone();
        bare.insert(
            70,
            proc(70, 13, "/Users/you/target/debug/juggler-app", &[]).1,
        );
        assert_eq!(host_of(71, &bare, &registry, home), None);
        // Without the cut (no own process, or another process on its pid),
        // the old answer: the Claude app profile that started AI Juggler.
        for registry in [hosts().with_own(None), hosts().with_own(Some((70, 9_000)))] {
            let before = host_of(71, &table, &registry, home).unwrap();
            assert_eq!(before.instance.as_deref(), Some("b-work"));
        }
    }

    #[test]
    fn a_terminal_session_is_in_its_terminal_and_an_unknown_app_by_its_name() {
        let home = Path::new("/Users/you");
        let terminal = "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal";
        let code = "/Applications/Visual Studio Code.app/Contents/MacOS/Electron";
        let helper = "/Applications/Visual Studio Code.app/Contents/Frameworks/Code Helper (Plugin).app/Contents/MacOS/Code Helper (Plugin)";
        let table: Table = [
            proc(30, 1, terminal, &[]),
            proc(31, 30, "/usr/bin/login", &[]),
            proc(32, 31, "/bin/zsh", &[]),
            proc(33, 32, "/Users/you/.local/bin/claude", &["claude"]),
            proc(40, 1, code, &[]),
            proc(41, 40, helper, &[]),
            proc(42, 41, "/bin/zsh", &[]),
            proc(43, 42, "/Users/you/.local/bin/claude", &["claude"]),
            proc(50, 1, "/Applications/Some Term.app/Contents/MacOS/st", &[]),
            proc(51, 50, "/bin/zsh", &[]),
            proc(52, 51, "/Users/you/.local/bin/claude", &["claude"]),
            proc(60, 1, "/opt/homebrew/bin/tmux", &[]),
            proc(61, 60, "/Users/you/.local/bin/claude", &["claude"]),
        ]
        .into_iter()
        .collect();
        let registry = hosts();
        let in_terminal = host_of(33, &table, &registry, home).unwrap();
        assert_eq!(
            (in_terminal.name.as_str(), in_terminal.pid),
            ("Terminal", 30)
        );
        assert_eq!(in_terminal.kind, Some(HostKind::Terminal));
        let in_editor = host_of(43, &table, &registry, home).unwrap();
        assert_eq!(
            (in_editor.app_id.as_deref(), in_editor.pid),
            (Some("vscode"), 40),
            "the editor's own process, not its plugin helper"
        );
        let unknown = host_of(52, &table, &registry, home).unwrap();
        assert_eq!((unknown.name.as_str(), unknown.app_id), ("Some Term", None));
        assert_eq!(
            host_of(61, &table, &registry, home),
            None,
            "tmux names no app"
        );
    }
}
