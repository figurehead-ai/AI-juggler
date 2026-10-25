//! AJ-E2101/E2102 (ADR-0019): where a session runs, from its process's
//! parent chain: the first ancestor inside an app bundle is its host app,
//! that app's outermost process on the chain is the one Go to it brings
//! forward, and the app's instance comes from that process's arguments
//! (the Claude app's `--user-data-dir`). Never from an environment
//! variable (`TERM_PROGRAM` is inherited and lies).
//!
//! Host apps are data: the adapters describe their tools' own apps
//! ([`juggler_adapter::HarnessAdapter::host_apps`]) and `hosts.json` the
//! terminals and editors. An app in neither is still a place, named by its
//! bundle.

use std::path::{Path, PathBuf};

use juggler_adapter::live::{HostApp, HostKind};

use super::procs::{Table, ancestors};

/// The terminals and editors a session can run in (data, not branches).
const BUILTIN: &str = include_str!("hosts.json");

/// Every host app known: the tools' own, then the terminals and editors.
#[derive(Clone, Debug, Default)]
pub struct HostRegistry {
    apps: Vec<HostApp>,
}

impl HostRegistry {
    /// The tools' apps (`tool_apps`, from the adapters) and the built-in
    /// terminals and editors.
    #[must_use]
    pub fn new(tool_apps: Vec<HostApp>) -> Self {
        let mut apps = tool_apps;
        // The list ships inside the app; a broken one is a build bug the
        // test below catches, never a crash here.
        apps.extend(serde_json::from_str::<Vec<HostApp>>(BUILTIN).unwrap_or_default());
        Self { apps }
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
#[must_use]
pub fn outer_bundle(exe: &Path) -> Option<(PathBuf, String)> {
    let mut path = PathBuf::new();
    for part in exe.components() {
        path.push(part);
        let name = part.as_os_str().to_string_lossy();
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
#[must_use]
pub fn host_of(pid: u32, table: &Table, hosts: &HostRegistry, home: &Path) -> Option<Host> {
    let chain = ancestors(table, pid);
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
    Some(Host {
        app_id: app.map(|app| app.id.clone()),
        name: app.map_or_else(
            || bundle.trim_end_matches(".app").to_owned(),
            |app| app.name.clone(),
        ),
        kind: app.map(|app| app.kind),
        pid: main.pid,
        start: main.start,
        instance_dir,
        instance,
    })
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
    for process in table.values() {
        let Some(exe) = process.exe.as_deref() else {
            continue;
        };
        let Some((dir, bundle)) = outer_bundle(exe) else {
            continue;
        };
        let Some(app) = hosts.by_bundle(&bundle) else {
            continue;
        };
        let Some(flag) = app.instance_arg.as_deref() else {
            continue;
        };
        // The app's main executable only (its helpers carry the argument
        // too, as Chromium's do).
        if exe.parent() != Some(&dir.join("Contents/MacOS")) {
            continue;
        }
        let Some(instance) = arg_value(&process.args, flag)
            .map(PathBuf::from)
            .or_else(|| app.default_instance.as_deref().map(|d| home.join(d)))
        else {
            continue;
        };
        if !out.iter().any(|(_, known)| *known == instance) {
            out.push((app.id.clone(), instance));
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
        HostRegistry::new(vec![HostApp {
            id: "claude-app".into(),
            name: "Claude".into(),
            bundle: "Claude.app".into(),
            bundle_id: Some("com.anthropic.claudefordesktop".into()),
            kind: HostKind::Desktop,
            instance_arg: Some("--user-data-dir".into()),
            default_instance: Some("Library/Application Support/Claude".into()),
            thread_link: None,
            folder_link: None,
            folder_command: None,
            tab_script: None,
        }])
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
