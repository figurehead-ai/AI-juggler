//! ADR-0032: presence from process snapshots — the real setup's shapes,
//! then generated ones against a reference model written apart.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proptest::prelude::*;

use super::{Facts, check, live_owner, running};
use crate::features::conversations::places::{HostRegistry, host_of};
use crate::features::conversations::procs::{Proc, Table};
use crate::features::conversations::scan::Place;

const HOME: &str = "/Users/you";
const CLAUDE: &str = "/Applications/Claude.app/Contents/MacOS/Claude";
const CLAUDE_CODE_IN_APP: &str = "/Users/you/Library/Application Support/Claude/claude-code/2.1.0/claude.app/Contents/MacOS/claude";
const CHATGPT: &str = "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT";
const CODEX_HELPER: &str = "/Applications/ChatGPT.app/Contents/Frameworks/Codex Framework.framework/Versions/157.0.8103.51/Helpers/Codex (Aperitif).app/Contents/MacOS/Codex (Aperitif)";
const CODEX_IN_APP: &str =
    "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex";
const TERMINAL: &str = "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal";
const CODE: &str = "/Applications/Visual Studio Code.app/Contents/MacOS/Electron";
const SHELL: &str = "/bin/zsh";
const TMUX: &str = "/opt/homebrew/bin/tmux";
const CLAUDE_CLI: &str = "/Users/you/.local/share/claude/versions/2.1.0";
const CODEX_CLI: &str = "/opt/homebrew/bin/codex";

fn hosts() -> HostRegistry {
    HostRegistry::new(
        [
            juggler_adapters::claude_live::host_apps(),
            juggler_adapters::codex_live::host_apps(),
        ]
        .concat(),
    )
}

fn claude_hosts() -> Vec<String> {
    juggler_adapters::claude_live::host_apps()
        .into_iter()
        .map(|app| app.id)
        .collect()
}

fn codex_hosts() -> Vec<String> {
    juggler_adapters::codex_live::host_apps()
        .into_iter()
        .map(|app| app.id)
        .collect()
}

fn profile(name: &str) -> String {
    format!("{HOME}/Library/Application Support/ClaudeProfiles/{name}")
}

fn default_profile() -> String {
    format!("{HOME}/Library/Application Support/Claude")
}

fn proc(pid: u32, ppid: u32, exe: &str, args: &[String], start: i64) -> (u32, Proc) {
    (
        pid,
        Proc {
            pid,
            ppid: Some(ppid),
            exe: Some(PathBuf::from(exe)),
            args: args.to_vec(),
            start: Some(start),
        },
    )
}

fn data_dir(dir: &str) -> Vec<String> {
    vec![CLAUDE.to_owned(), format!("--user-data-dir={dir}")]
}

/// A session's place as the live view makes it: its process and the
/// host its chain names.
fn place(table: &Table, pid: u32) -> Place {
    Place {
        pid,
        start: table.get(&pid).and_then(|p| p.start),
        host: host_of(pid, table, &hosts(), Path::new(HOME)),
        profile: None,
        app_session: None,
    }
}

fn desktop(dir: &str) -> String {
    crate::features::places::book::place_id("claude-app", Some(Path::new(dir)))
}

/// The maintainer's setup on 2026-11-11: six Claude profiles (the default
/// one started with its own folder given), ChatGPT on Codex, sessions in
/// some of them, a terminal session on one profile's folder, and one in
/// an editor.
fn the_real_setup() -> Table {
    let mut rows = vec![
        proc(100, 1, CLAUDE, &data_dir(&profile("b-olivia")), 100),
        proc(200, 1, CLAUDE, &data_dir(&default_profile()), 200),
        proc(300, 1, CLAUDE, &data_dir(&profile("b-anela")), 300),
        proc(400, 1, CLAUDE, &data_dir(&profile("b-mia")), 400),
        proc(500, 1, CLAUDE, &data_dir(&profile("b-luka")), 500),
        proc(600, 1, CLAUDE, &data_dir(&profile("personal-max")), 600),
        proc(700, 1, CHATGPT, &[CHATGPT.to_owned()], 700),
        proc(701, 700, CODEX_HELPER, &[], 701),
        proc(702, 700, CODEX_IN_APP, &["app-server".to_owned()], 702),
        // Claude Code sessions inside two profiles (the app's own claude).
        proc(110, 100, CLAUDE_CODE_IN_APP, &[], 110),
        proc(310, 300, CLAUDE_CODE_IN_APP, &[], 310),
        // A terminal session on anela's folder ("Jive CLI").
        proc(800, 1, TERMINAL, &[], 800),
        proc(801, 800, SHELL, &[], 801),
        proc(802, 801, CLAUDE_CLI, &[], 802),
        // One in an editor's terminal, one in tmux under launchd, one
        // with no app at all.
        proc(900, 1, CODE, &[], 900),
        proc(901, 900, SHELL, &[], 901),
        proc(902, 901, CLAUDE_CLI, &[], 902),
        proc(950, 1, TMUX, &[], 950),
        proc(951, 950, SHELL, &[], 951),
        proc(952, 951, CODEX_CLI, &[], 952),
        proc(990, 1, CLAUDE_CLI, &[], 990),
    ];
    rows.sort_by_key(|(pid, _)| *pid);
    rows.into_iter().collect()
}

#[test]
fn every_running_app_reads_running_once_codex_in_chatgpt_too() {
    let table = the_real_setup();
    let found: BTreeSet<String> = running(&table, &hosts(), Path::new(HOME))
        .into_keys()
        .collect();
    let want: BTreeSet<String> = [
        desktop(&profile("b-olivia")),
        desktop(&default_profile()),
        desktop(&profile("b-anela")),
        desktop(&profile("b-mia")),
        desktop(&profile("b-luka")),
        desktop(&profile("personal-max")),
        "app:codex-app:".to_owned(),
    ]
    .into_iter()
    .collect();
    assert_eq!(found, want);
}

#[test]
fn the_default_profile_is_one_app_with_or_without_its_folder_given() {
    let given = the_real_setup();
    let mut bare = the_real_setup();
    bare.insert(200, proc(200, 1, CLAUDE, &[CLAUDE.to_owned()], 200).1);
    let id = |table: &Table| {
        running(table, &hosts(), Path::new(HOME))
            .into_iter()
            .find(|(_, app)| app.pid == 200)
            .map(|(id, _)| id)
    };
    assert_eq!(id(&given), Some(desktop(&default_profile())));
    assert_eq!(id(&given), id(&bare));
}

#[test]
fn a_session_belongs_to_the_app_its_process_runs_in() {
    let table = the_real_setup();
    let owner = |pid: u32, tool_hosts: &[String]| {
        live_owner(tool_hosts, Some(&place(&table, pid)), "folder-anela")
    };
    // In a Claude profile: that profile.
    assert_eq!(owner(110, &claude_hosts()), desktop(&profile("b-olivia")));
    assert_eq!(owner(310, &claude_hosts()), desktop(&profile("b-anela")));
    // In a terminal on anela's folder: the folder's CLI App, never the
    // Claude app profile (the "Jive CLI" filing, 2026-11-11).
    assert_eq!(owner(802, &claude_hosts()), "cli:folder-anela");
    // In an editor, in tmux, with no app: CLI too.
    assert_eq!(owner(902, &claude_hosts()), "cli:folder-anela");
    assert_eq!(owner(952, &codex_hosts()), "cli:folder-anela");
    assert_eq!(owner(990, &claude_hosts()), "cli:folder-anela");
    // Codex's own server inside ChatGPT.app: the Codex app.
    assert_eq!(owner(702, &codex_hosts()), "app:codex-app:");
    // Another tool's desktop app on the chain doesn't make it that app's.
    assert_eq!(owner(702, &claude_hosts()), "cli:folder-anela");
    // No place at all (only its files say it's open): CLI.
    assert_eq!(live_owner(&claude_hosts(), None, "f"), "cli:f");
}

#[test]
fn a_reused_pid_is_a_new_process() {
    let table = the_real_setup();
    let before = running(&table, &hosts(), Path::new(HOME));
    // b-mia quits; its pid comes back as a terminal shell.
    let mut after = table.clone();
    after.insert(400, proc(400, 1, SHELL, &[], 4_000).1);
    let now = running(&after, &hosts(), Path::new(HOME));
    assert!(before.contains_key(&desktop(&profile("b-mia"))));
    assert!(!now.contains_key(&desktop(&profile("b-mia"))));
}

#[test]
fn the_checks_name_each_mismatch() {
    let facts = Facts {
        listed: BTreeMap::from([
            ("app:claude-app:/a".to_owned(), Some(1)),
            ("app:claude-app:/b".to_owned(), None),
            ("app:claude-app:/c".to_owned(), Some(3)),
        ]),
        running: BTreeSet::from([
            "app:claude-app:/b".to_owned(),
            "app:claude-app:/c".to_owned(),
        ]),
        live_owners: BTreeMap::from([
            ("x/claude-code/1".to_owned(), "app:claude-app:/a".to_owned()),
            ("x/claude-code/2".to_owned(), "cli:x".to_owned()),
        ]),
    };
    let broken = check(&facts);
    assert_eq!(broken.len(), 3, "{broken:?}");
    assert!(broken[0].contains("/a reads running"));
    assert!(broken[1].contains("/b runs but reads closed"));
    assert!(broken[2].contains("x/claude-code/1"));
}

// ── Generated snapshots against a reference model ──────────────────────

/// One generated app main process: which app, which profile (`None`: the
/// default, given or not), and whether it quit.
#[derive(Clone, Debug)]
struct App {
    codex: bool,
    profile: Option<u8>,
    given: bool,
    quit: bool,
}

/// One generated session: its tool, and what it runs under.
#[derive(Clone, Debug)]
struct Session {
    codex: bool,
    under: Under,
}

#[derive(Clone, Debug)]
enum Under {
    App(usize),
    Terminal,
    Editor,
    Tmux,
    Nothing,
}

fn app() -> impl Strategy<Value = App> {
    (
        any::<bool>(),
        proptest::option::of(0u8..4),
        any::<bool>(),
        proptest::bool::weighted(0.2),
    )
        .prop_map(|(codex, profile, given, quit)| App {
            codex,
            profile,
            given,
            quit,
        })
}

fn session() -> impl Strategy<Value = Session> {
    let under = prop_oneof![
        (0usize..8).prop_map(Under::App),
        Just(Under::Terminal),
        Just(Under::Editor),
        Just(Under::Tmux),
        Just(Under::Nothing),
    ];
    (any::<bool>(), under).prop_map(|(codex, under)| Session { codex, under })
}

/// The reference model, from the generated facts alone (no process
/// table): an App's id, the apps that run, and each session's owner.
fn model_id(app: &App) -> String {
    if app.codex {
        return "app:codex-app:".to_owned();
    }
    desktop(
        &app.profile
            .map_or_else(default_profile, |n| profile(&format!("p{n}"))),
    )
}

struct Built {
    table: Table,
    /// Session pid → (its tool's hosts, the owner the model expects).
    sessions: Vec<(u32, Vec<String>, String)>,
    running: BTreeSet<String>,
}

fn build(apps: &[App], sessions: &[Session]) -> Built {
    let mut table = Table::new();
    let mut mains: Vec<Option<u32>> = Vec::new();
    let mut want = BTreeSet::new();
    for (at, app) in apps.iter().enumerate() {
        let pid = 1_000 + u32::try_from(at).unwrap_or(0) * 10;
        let start = i64::from(pid);
        if app.quit {
            // Quit: its pid reused by something else.
            table.insert(pid, proc(pid, 1, SHELL, &[], start + 7).1);
            mains.push(None);
            continue;
        }
        let row = if app.codex {
            proc(pid, 1, CHATGPT, &[CHATGPT.to_owned()], start)
        } else {
            let args = match (app.profile, app.given) {
                (None, false) => vec![CLAUDE.to_owned()],
                (None, true) => data_dir(&default_profile()),
                (Some(n), _) => data_dir(&profile(&format!("p{n}"))),
            };
            proc(pid, 1, CLAUDE, &args, start)
        };
        table.insert(pid, row.1);
        // Two mains of one instance: the oldest is the app, the other a
        // duplicate the model still counts once.
        want.insert(model_id(app));
        mains.push(Some(pid));
    }
    table.insert(50, proc(50, 1, TERMINAL, &[], 50).1);
    table.insert(51, proc(51, 1, CODE, &[], 51).1);
    table.insert(52, proc(52, 1, TMUX, &[], 52).1);
    let mut out = Vec::new();
    for (at, session) in sessions.iter().enumerate() {
        let pid = 5_000 + u32::try_from(at).unwrap_or(0) * 10;
        let hosts_of = if session.codex {
            codex_hosts()
        } else {
            claude_hosts()
        };
        let exe = if session.codex { CODEX_CLI } else { CLAUDE_CLI };
        let (parent, owner) = match &session.under {
            Under::App(index) => match apps.get(*index).zip(mains.get(*index).copied().flatten()) {
                Some((app, main)) => {
                    let mine = app.codex == session.codex;
                    (
                        main,
                        if mine {
                            model_id(app)
                        } else {
                            "cli:f".to_owned()
                        },
                    )
                }
                None => (1, "cli:f".to_owned()),
            },
            Under::Terminal => (50, "cli:f".to_owned()),
            Under::Editor => (51, "cli:f".to_owned()),
            Under::Tmux => (52, "cli:f".to_owned()),
            Under::Nothing => (1, "cli:f".to_owned()),
        };
        table.insert(
            pid + 1,
            proc(pid + 1, parent, SHELL, &[], i64::from(pid) + 1).1,
        );
        table.insert(pid, proc(pid, pid + 1, exe, &[], i64::from(pid)).1);
        out.push((pid, hosts_of, owner));
    }
    Built {
        table,
        sessions: out,
        running: want,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, .. ProptestConfig::default() })]

    /// The running set is exactly the apps whose main runs, each once;
    /// every session has the owner the model gives it; and a live
    /// session's desktop owner is in the running set.
    #[test]
    fn presence_matches_the_model(apps in proptest::collection::vec(app(), 0..8), sessions in proptest::collection::vec(session(), 0..12)) {
        let built = build(&apps, &sessions);
        let found = running(&built.table, &hosts(), Path::new(HOME));
        let ids: BTreeSet<String> = found.keys().cloned().collect();
        prop_assert_eq!(&ids, &built.running);
        let mut live_owners = BTreeMap::new();
        for (pid, tool_hosts, want) in &built.sessions {
            let got = live_owner(tool_hosts, Some(&place(&built.table, *pid)), "f");
            prop_assert_eq!(&got, want, "session {}", pid);
            live_owners.insert(pid.to_string(), got);
        }
        let facts = Facts {
            listed: ids.iter().map(|id| (id.clone(), found.get(id).map(|app| app.pid))).collect(),
            running: ids.clone(),
            live_owners,
        };
        prop_assert!(check(&facts).is_empty(), "{:?}", check(&facts));
    }
}

/// ADR-0032 amendment 4: an app's launch and quit reach the window within
/// the bound. A stand-in `Claude.app` (a copy of `bash` in a bundle of
/// that name under a temporary folder, with its own `--user-data-dir`,
/// waiting on its own `read -t 30`) starts and stops; a forced look (what `NSWorkspace`'s notification
/// runs) sees each at once, and the timed look never waits longer than
/// `RECONCILE_EVERY_MS`. Bounded: the stand-in waits 30 s at most and is
/// stopped and reaped here.
#[test]
fn a_launch_and_a_quit_show_within_the_bound() {
    use crate::features::places::{RECONCILE_EVERY_MS, reconcile};
    const _: () = assert!(RECONCILE_EVERY_MS <= 5_000);
    let root = tempfile::tempdir().unwrap();
    let state =
        std::sync::Arc::new(crate::state::AppState::init(root.path().join("data")).unwrap());
    let bundle = root.path().join("Claude.app/Contents/MacOS");
    std::fs::create_dir_all(&bundle).unwrap();
    let exe = bundle.join("Claude");
    std::fs::copy("/bin/bash", &exe).unwrap();
    let now = crate::commands::now_millis();
    reconcile(&state, now, true);
    // `read -t 30` is bash's own: no child process, so the stand-in is
    // the only process to stop (own-child `kill`, the signal rule's).
    let mut child = std::process::Command::new(&exe)
        .args(["-c", "read -t 30 _"])
        .arg(format!(
            "--user-data-dir={}",
            root.path().join("profile").display()
        ))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let launched = (0..20).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(100));
        reconcile(&state, crate::commands::now_millis(), true)
    });
    let _ = child.kill();
    let _ = child.wait();
    assert!(launched, "the launch showed within 2 s");
    // Unforced, inside the bound: no look yet; past it: a look, and the quit.
    let at = crate::commands::now_millis();
    assert!(!reconcile(&state, at, false));
    assert!(
        reconcile(&state, at + RECONCILE_EVERY_MS, false),
        "the quit showed by the bound"
    );
}

/// ADR-0032 amendment 5: one name per App, the same on every surface.
#[test]
fn every_app_has_one_name_that_says_desktop_or_cli() {
    use super::{Naming, identity};
    let base = Naming {
        id: "app:claude-app:/p",
        desktop: true,
        app: "Claude app",
        instance_dir: None,
        default_instance: false,
        folder_label: None,
        folder_shared: false,
        named: None,
    };
    let luka = Path::new("/Users/you/Library/Application Support/ClaudeProfiles/b-luka");
    let default = Path::new("/Users/you/Library/Application Support/Claude");
    let test = Path::new("/Users/you/Library/Application Support/ClaudeProfiles/aij-test");
    let name = |naming: Naming<'_>| identity(naming).label;
    assert_eq!(
        name(Naming {
            instance_dir: Some(luka),
            folder_label: Some("bookauth-luka"),
            ..base
        }),
        "Claude app · bookauth-luka"
    );
    // The default instance: no profile, whatever its folder is called.
    assert_eq!(
        name(Naming {
            instance_dir: Some(default),
            default_instance: true,
            folder_label: Some("Default"),
            ..base
        }),
        "Claude app"
    );
    // A test profile on the default's folder goes by its own name.
    assert_eq!(
        name(Naming {
            instance_dir: Some(test),
            folder_label: Some("Default"),
            folder_shared: true,
            ..base
        }),
        "Claude app · aij-test"
    );
    assert_eq!(
        name(Naming {
            app: "Codex app",
            ..base
        }),
        "Codex app"
    );
    let cli = identity(Naming {
        id: "cli:x",
        desktop: false,
        app: "Claude Code",
        folder_label: Some("bookauth-luka"),
        ..base
    });
    assert_eq!(
        (cli.label.as_str(), cli.desktop),
        ("Claude Code CLI · bookauth-luka", false)
    );
    assert_eq!(
        name(Naming {
            named: Some("Day job"),
            ..base
        }),
        "Day job"
    );
}

/// ADR-0032: a folder two desktop Apps share is its app's default
/// instance's when exactly one of them is (the real data: a test profile
/// linked to `~/.claude` beside the default Claude app); otherwise no one's.
#[test]
fn a_shared_folder_is_the_default_instances() {
    use crate::features::places::sole_holders;
    let held = sole_holders([
        ("app:default", Some("dot-claude"), true),
        ("app:aij-test", Some("dot-claude"), false),
        ("app:luka", Some("luka"), false),
        ("app:a", Some("shared"), false),
        ("app:b", Some("shared"), false),
    ]);
    assert_eq!(
        held.get("dot-claude").map(String::as_str),
        Some("app:default")
    );
    assert_eq!(held.get("luka").map(String::as_str), Some("app:luka"));
    assert_eq!(held.get("shared"), None);
}
