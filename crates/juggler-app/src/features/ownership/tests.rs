//! AJ-E2315: the shared ownership rule's fixtures (the review's D1: pure,
//! `procs::fixture`, no OS but a temporary folder where a link must be
//! real). Placeholder data only: `/Users/you`, profiles `work`, `team-a`
//! and `lab`.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::self_rule::{Juggler, quits_juggler};
use super::sessions::{Under, folder_of_sessions};
use super::*;
use crate::features::conversations::procs::{Proc, fixture as proc};

const APP: &str = "/Applications/Claude.app";
const WORK: &str = "/Users/you/Library/Application Support/ClaudeProfiles/work";
const OTHER: &str = "/Users/you/Library/Application Support/ClaudeProfiles/team-a";

static WORK_KEY: LazyLock<InstanceKey> = LazyLock::new(|| InstanceKey::of(Path::new(WORK)));

fn profile(mains: &[u32]) -> Profile<'_> {
    Profile {
        dir: &WORK_KEY,
        bundle: Path::new(APP),
        instance_arg: Some("--user-data-dir"),
        mains,
    }
}

fn others() -> Others {
    Others::of([PathBuf::from(OTHER)], vec![20])
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
    let known = Others::of([PathBuf::from(&nested)], Vec::new());
    let dir = &*WORK_KEY;
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

// --- AJ-E2315 part 2: the review's D1 fixtures ---------------------------

fn main_exe() -> String {
    format!("{APP}/Contents/MacOS/Claude")
}

fn helper_exe() -> String {
    format!("{APP}/Contents/Frameworks/Claude Helper.app/Contents/MacOS/Claude Helper")
}

/// The Claude app as the registry has it (its instance flag and lock).
fn claude_app() -> juggler_adapter::live::HostApp {
    juggler_adapters::claude_live::host_apps().remove(0)
}

/// A main process `pid` of the Claude app on profile folder `dir`.
fn main_on(pid: u32, dir: &str) -> (u32, Proc) {
    proc(
        pid,
        1,
        &main_exe(),
        &[&main_exe(), &format!("--user-data-dir={dir}")],
    )
}

/// A profile's Code tab: its main process `main` on `dir`, its wrapper
/// (`main + 3`) running `program`, and that program (`main + 4`).
fn code_tab(main: u32, dir: &str, program: &str) -> [(u32, Proc); 3] {
    let d = disclaimer();
    [
        main_on(main, dir),
        proc(main + 3, main, &d, &[&d, "--pgroup", "--", program]),
        proc(main + 4, main + 3, program, &[program]),
    ]
}

fn one(pid: u32, ppid: u32, exe: &str, args: &[&str]) -> Table {
    [proc(pid, ppid, exe, args)].into_iter().collect()
}

/// D1.7: a profile's folder spelled with `.`, `..` or a trailing slash is
/// that profile (the old raw comparison made it nobody's).
#[test]
fn an_instance_flag_spelled_another_way_is_the_same_profile() {
    for spelling in [
        format!("{WORK}/"),
        format!("{WORK}/x/.."),
        "/Users/you/Library/Application Support/./ClaudeProfiles/work".to_owned(),
    ] {
        let flag = format!("--user-data-dir={spelling}");
        let t = one(30, 1, &helper_exe(), &[&helper_exe(), &flag]);
        assert_eq!(
            of(&t, 30, &profile(&[]), &others()),
            Whose::This,
            "{spelling}"
        );
    }
    let other = format!("--user-data-dir={OTHER}/");
    let t = one(31, 1, &helper_exe(), &[&helper_exe(), &other]);
    assert_eq!(of(&t, 31, &profile(&[]), &others()), Whose::Another);
}

/// D1.7: a symlinked profile folder, and `..` through the link, decide as
/// the folder it points to (the old prefix match decided by spelling).
#[cfg(unix)]
#[test]
fn a_symlinked_profile_folder_is_the_folder_it_points_to() {
    let root = tempfile::tempdir().unwrap();
    let work = root.path().join("ClaudeProfiles/work");
    let team = root.path().join("ClaudeProfiles/team-a");
    for dir in [&work, &team] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let link = root.path().join("work-link");
    std::os::unix::fs::symlink(&work, &link).unwrap();
    let key = InstanceKey::of(&work);
    assert_eq!(InstanceKey::of(&link), key);
    assert_eq!(InstanceKey::of(&root.path().join("work-link/")), key);
    let known = Others::of([team.clone()], Vec::new());
    for path in [
        link.join("claude-code/claude"),
        link.join("x/../claude-code/claude"),
        work.join("claude-code/claude"),
    ] {
        assert_eq!(whose_path(&path, &key, &known), Whose::This, "{path:?}");
    }
    assert_eq!(
        whose_path(
            &root.path().join("ClaudeProfiles/work/../team-a/x"),
            &key,
            &known
        ),
        Whose::Another
    );
    let profile = Profile {
        dir: &key,
        bundle: Path::new(APP),
        instance_arg: Some("--user-data-dir"),
        mains: &[],
    };
    let flag = format!("--user-data-dir={}", link.display());
    let t = one(30, 1, &helper_exe(), &[&helper_exe(), &flag]);
    assert_eq!(whose(&t, &t[&30], &profile, &known), Whose::This);
    // `/tmp/lab1` never holds `/tmp/lab10/…`: folders compare part by part.
    let lab1 = InstanceKey::of(&root.path().join("lab1"));
    assert!(!lab1.holds(InstanceKey::of(&root.path().join("lab10/x")).path()));
}

/// D1.7: a flag the app might read otherwise (after `--`, given twice, in
/// the two-word form) never makes a process a profile's (the old loose
/// read took the first value it saw).
#[test]
fn an_instance_flag_the_app_might_read_otherwise_tells_nothing() {
    let work = format!("--user-data-dir={WORK}");
    let team = format!("--user-data-dir={OTHER}");
    for args in [
        vec!["--", work.as_str()],
        vec![work.as_str(), work.as_str()],
        vec!["--user-data-dir", WORK],
        vec![team.as_str(), work.as_str()],
    ] {
        let mut all = vec![helper_exe()];
        all.extend(args.iter().map(|arg| (*arg).to_owned()));
        let all: Vec<&str> = all.iter().map(String::as_str).collect();
        let t = one(30, 1, &helper_exe(), &all);
        assert_eq!(
            of(&t, 30, &profile(&[]), &others()),
            Whose::Unknown,
            "{args:?}"
        );
    }
}

/// D1.7: the instance a main process runs: its flag read strictly, its
/// default without one; a flag that can't be trusted is its instance only
/// by the lock it holds (the operating system decides, not the spelling).
#[test]
fn an_untrusted_instance_flag_is_decided_by_the_lock_its_process_holds() {
    let app = claude_app();
    let home = Path::new("/Users/you");
    let flag = format!("--user-data-dir={WORK}");
    let (_, strict) = proc(10, 1, &main_exe(), &[&main_exe(), &flag]);
    assert_eq!(
        instance_of(&strict, &app, home),
        Ok(Instance {
            given: Some(PathBuf::from(WORK)),
            dir: Some(PathBuf::from(WORK)),
        })
    );
    let (_, plain) = proc(11, 1, &main_exe(), &[&main_exe()]);
    assert_eq!(
        instance_of(&plain, &app, home).unwrap().dir,
        Some(home.join("Library/Application Support/Claude"))
    );
    let (_, two_word) = proc(12, 1, &main_exe(), &[&main_exe(), "--user-data-dir", WORK]);
    let nobody = |_: &InstanceKey, _: &str| None;
    assert!(instance::instance_with(&two_word, &app, Some(home), &nobody).is_err());
    let work = InstanceKey::of(Path::new(WORK));
    let held_by_12 = |key: &InstanceKey, _: &str| (*key == work).then_some(12);
    assert_eq!(
        instance::instance_with(&two_word, &app, Some(home), &held_by_12)
            .unwrap()
            .dir,
        Some(PathBuf::from(WORK))
    );
    // After `--` the app reads no switch: it runs its default, as its lock
    // says.
    let (_, after) = proc(13, 1, &main_exe(), &[&main_exe(), "--", &flag]);
    let default = InstanceKey::of(&home.join("Library/Application Support/Claude"));
    let held_default = |key: &InstanceKey, _: &str| (*key == default).then_some(13);
    let instance = instance::instance_with(&after, &app, Some(home), &held_default).unwrap();
    assert_eq!(instance.given, None);
    assert_eq!(
        instance.dir,
        Some(home.join("Library/Application Support/Claude"))
    );
    // Only the main process holding its lock proves it runs.
    let table: Table = [
        proc(10, 1, &main_exe(), &[&main_exe()]),
        proc(14, 10, &helper_exe(), &[&helper_exe()]),
    ]
    .into_iter()
    .collect();
    let by = |pid: u32| move |_: &InstanceKey, _: &str| Some(pid);
    let dir = Some(Path::new(WORK));
    assert_eq!(
        instance::proven_running(&table, &app, Path::new(APP), dir, &by(10)),
        Some(10)
    );
    assert_eq!(
        instance::proven_running(&table, &app, Path::new(APP), dir, &by(14)),
        None
    );
}

/// D1.8: two profiles on one account (both on one tool folder): each one's
/// processes are its own, whichever runs, by its main process and flag.
#[test]
fn two_profiles_on_one_account_keep_their_own_processes() {
    let code = code("/Users/you/Library/Application Support/ClaudeCodeProfiles/team-a");
    let t: Table = code_tab(10, WORK, &code)
        .into_iter()
        .chain(code_tab(20, OTHER, &code))
        .collect();
    for pid in [10, 13, 14] {
        assert_eq!(
            of(&t, pid, &profile(&[10]), &others()),
            Whose::This,
            "{pid}"
        );
    }
    for pid in [20, 23, 24] {
        assert_eq!(
            of(&t, pid, &profile(&[10]), &others()),
            Whose::Another,
            "{pid}"
        );
    }
}

/// A profile folder nested in another: its processes are the inner
/// profile's, never the outer one's, and the other way round.
#[test]
fn a_nested_profile_folder_keeps_its_own_processes() {
    let nested = format!("{WORK}/lab");
    let known = Others::of([PathBuf::from(&nested)], vec![20]);
    let t: Table = [
        main_on(20, &nested),
        proc(
            21,
            1,
            &helper_exe(),
            &[&helper_exe(), &format!("--user-data-dir={nested}/")],
        ),
        proc(22, 1, &code(&nested), &["claude"]),
        proc(24, 1, &code(WORK), &["claude"]),
    ]
    .into_iter()
    .collect();
    for pid in [20, 21, 22] {
        assert_eq!(of(&t, pid, &profile(&[]), &known), Whose::Another, "{pid}");
    }
    assert_eq!(of(&t, 24, &profile(&[]), &known), Whose::This);
    // Seen from the nested profile, the outer one's program isn't its.
    let inner = InstanceKey::of(Path::new(&nested));
    let outer = Others::of([PathBuf::from(WORK)], Vec::new());
    assert_eq!(
        whose_path(Path::new(&code(WORK)), &inner, &outer),
        Whose::Another
    );
    assert_eq!(
        whose_path(Path::new(&code(&nested)), &inner, &outer),
        Whose::This
    );
}

/// The live tree of the self rule: Work's main (10), its wrapper (13), the
/// Code tab's Claude Code (14), a shell it runs (15), and below the shell
/// AI Juggler (70) with its own helper (71); `team-a` (20) beside it.
fn juggler_table(juggler_parent: u32) -> Table {
    code_tab(10, WORK, &code(WORK))
        .into_iter()
        .chain([
            proc(15, 14, "/bin/zsh", &["/bin/zsh", "-c", "cargo tauri dev"]),
            proc(
                70,
                juggler_parent,
                "/Users/you/aij/target/debug/juggler-app",
                &["juggler-app"],
            ),
            proc(
                71,
                70,
                "/Users/you/aij/target/debug/juggler-agent",
                &["juggler-agent"],
            ),
            main_on(20, OTHER),
            proc(
                80,
                1,
                "/Applications/AI Juggler.app/Contents/MacOS/juggler-app",
                &["juggler-app"],
            ),
        ])
        .collect()
}

const JUGGLER: &str = "juggler-app";

/// D1.5: AI Juggler started from the profile's Code tab: closing the
/// profile would close it, so the move refuses (ours), before any quit.
#[test]
fn a_move_never_quits_the_ai_juggler_started_from_the_profile() {
    let t = juggler_table(15);
    let members = [10, 13, 14, 15];
    let program = std::ffi::OsStr::new(JUGGLER);
    assert_eq!(
        quits_juggler(&t, &members, 70, program),
        Some(Juggler::Ours)
    );
    // Started from the Dock (launchd's), nothing of ours is closed.
    let t = juggler_table(1);
    assert_eq!(quits_juggler(&t, &members, 70, program), None);
    // Another profile's move never touches it.
    assert_eq!(quits_juggler(&juggler_table(15), &[20], 70, program), None);
    let words = self_rule::words(Juggler::Ours, "Claude app · work", "Move");
    assert!(
        words.contains("started from inside Claude app · work"),
        "{words}"
    );
    assert!(
        words.ends_with("Move becomes available once you start AI Juggler from the Dock or Finder")
    );
}

/// D1.6: another AI Juggler (the Dev build) started from the profile's
/// Code tab, seen by the installed one (80): the move refuses (another).
#[test]
fn a_move_never_quits_another_ai_juggler_started_from_the_profile() {
    let t = juggler_table(15);
    let program = std::ffi::OsStr::new(JUGGLER);
    assert_eq!(
        quits_juggler(&t, &[10, 13, 14, 15], 80, program),
        Some(Juggler::Another(70))
    );
    // As a member itself (its own main process in the tree).
    assert_eq!(
        quits_juggler(&t, &[10, 70], 80, program),
        Some(Juggler::Another(70))
    );
    // Its helper, another program, is no AI Juggler of its own.
    assert_eq!(
        quits_juggler(&t, &[71], 80, Path::new("x").as_os_str()),
        None
    );
    let words = self_rule::words(Juggler::Another(70), "Claude app · work", "Undo");
    assert!(
        words.starts_with("another AI Juggler was started from inside"),
        "{words}"
    );
}

/// D1 (link, the review's E7): the tool folder a profile's sessions use is
/// the one its running sessions' pid files are in, under its own main
/// process, never the folder named like it. Work's Code tab writes in
/// `ClaudeCodeProfiles/team-a` (its launcher's `CLAUDE_CONFIG_DIR`); the
/// folder named `work` holds only a nested command line's pid file.
#[test]
fn a_profiles_tool_folder_is_where_its_sessions_pid_files_are() {
    use juggler_adapter::live::{LiveSession, LiveStatus};
    let tools = "/Users/you/Library/Application Support/ClaudeCodeProfiles";
    let t: Table = code_tab(10, WORK, &code(WORK))
        .into_iter()
        .chain([
            proc(15, 14, "/bin/zsh", &["/bin/zsh"]),
            proc(16, 15, "/Users/you/.local/bin/claude", &["claude"]),
            main_on(20, OTHER),
        ])
        .collect();
    let session = |pid: u32, start: i64| LiveSession {
        session_id: format!("s-{pid}"),
        pid: Some(pid),
        started_secs: Some(start),
        cwd: None,
        status: LiveStatus::Idle,
        host_session_id: None,
        entrypoint: None,
        name: None,
        background: false,
    };
    let folders = vec![
        (
            PathBuf::from(format!("{tools}/work")),
            vec![session(16, 160)],
        ),
        (
            PathBuf::from(format!("{tools}/team-a")),
            vec![session(14, 140)],
        ),
    ];
    let app = claude_app();
    let key = InstanceKey::of(Path::new(WORK));
    let under = Under {
        app: &app,
        bundle: Path::new(APP),
        home: Path::new("/Users/you"),
        instance: &key,
    };
    assert_eq!(
        folder_of_sessions(&t, &under, &folders),
        Some(Path::new(&format!("{tools}/team-a")))
    );
    // A pid the system gave another program since tells nothing.
    let stale = vec![(
        PathBuf::from(format!("{tools}/team-a")),
        vec![session(14, 9_999)],
    )];
    assert_eq!(folder_of_sessions(&t, &under, &stale), None);
    // Another profile's sessions never link this one; with none of its
    // own running, nothing tells.
    let theirs = InstanceKey::of(Path::new(OTHER));
    let other = Under {
        instance: &theirs,
        ..under
    };
    assert_eq!(folder_of_sessions(&t, &other, &folders), None);
}

/// D1.7, `running_now`'s search (the review's A2.1): a running profile
/// spelled another way on its command line is found by its instance, and
/// one whose flag can't be trusted by the lock it holds. The old search
/// compared ids made from the raw spelling, so it read as not running and
/// the move never quit it.
#[test]
fn a_running_profile_is_found_however_its_folder_was_spelled() {
    let hosts = HostRegistry::new(juggler_adapters::claude_live::host_apps());
    let home = Path::new("/Users/you");
    let place = crate::test_support::claude_profile(WORK);
    let nobody = |_: &InstanceKey, _: &str| None;
    for spelling in [format!("{WORK}/"), format!("{WORK}/x/..")] {
        let flag = format!("--user-data-dir={spelling}");
        let t = one(10, 1, &main_exe(), &[&main_exe(), &flag]);
        let found = running_place(&t, &hosts, home, &place, &nobody);
        assert_eq!(found.map(|app| app.pid), Some(10), "{spelling}");
    }
    // The two-word form: the app's own lock says which it runs.
    let t = one(10, 1, &main_exe(), &[&main_exe(), "--user-data-dir", WORK]);
    assert!(running_place(&t, &hosts, home, &place, &nobody).is_none());
    let work = InstanceKey::of(Path::new(WORK));
    let held = |key: &InstanceKey, _: &str| (*key == work).then_some(10);
    assert_eq!(
        running_place(&t, &hosts, home, &place, &held).map(|app| app.pid),
        Some(10)
    );
    // Another profile running is never it.
    let t: Table = [main_on(20, OTHER)].into_iter().collect();
    assert!(running_place(&t, &hosts, home, &place, &nobody).is_none());
}

// ---------------------------------------------------------------------
// AJ-E2327: which tool folder a profile uses, by the strongest evidence.

mod link_evidence {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    use super::super::sessions::{Evidence, LinkSource, contradicts, decide, folder_of_records};

    const CLIENT_A: &str = "/Users/you/Library/Application Support/ClaudeCodeProfiles/client-a";
    const TEAM_A: &str = "/Users/you/Library/Application Support/ClaudeCodeProfiles/team-a";
    const WORK: &str = "/Users/you/Library/Application Support/ClaudeCodeProfiles/work";

    fn holding(folder: &str, ids: &[&str]) -> (PathBuf, HashSet<String>) {
        (
            PathBuf::from(folder),
            ids.iter().map(|id| (*id).to_owned()).collect(),
        )
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| (*id).to_owned()).collect()
    }

    /// The fresh 0.5.0 install: profile `team-a` writes in `client-a`;
    /// the folder named like it (`team-a`) holds nothing of its. Its own
    /// records find it while it's closed.
    #[test]
    fn a_closed_profile_is_linked_by_where_its_records_are() {
        let folders = [
            holding(TEAM_A, &[]),
            holding(CLIENT_A, &["s-1", "s-2", "s-3"]),
            holding(WORK, &["s-9"]),
        ];
        let found = folder_of_records(&ids(&["s-1", "s-2", "s-3"]), &folders, &[Path::new(TEAM_A)]);
        assert_eq!(found, Some(Path::new(CLIENT_A)));
        // None of its conversations anywhere: nothing tells.
        assert_eq!(folder_of_records(&ids(&["s-7"]), &folders, &[]), None);
        assert_eq!(folder_of_records(&[], &folders, &[]), None);
    }

    /// A switch copies a conversation into another profile's folder under
    /// the same id: a tie is never decided by counting. Its link so far
    /// (or the folder named like it) among the tied is confirmed; else
    /// nothing is decided.
    #[test]
    fn a_tie_is_confirmed_by_its_link_or_left() {
        let folders = [holding(CLIENT_A, &["s-1"]), holding(WORK, &["s-1"])];
        let mine = ids(&["s-1"]);
        assert_eq!(folder_of_records(&mine, &folders, &[]), None);
        assert_eq!(
            folder_of_records(&mine, &folders, &[Path::new(WORK)]),
            Some(Path::new(WORK))
        );
        assert_eq!(
            folder_of_records(&mine, &folders, &[Path::new(TEAM_A)]),
            None,
            "a preference outside the tie decides nothing"
        );
    }

    /// 0.4.3 (D-102), kept: its own folder wins over one holding switch
    /// copies of some of its sessions; the folder holding most of them
    /// wins where its own holds none.
    #[test]
    fn the_folder_holding_most_of_its_sessions_wins() {
        let mine: Vec<String> = (1..=21).map(|n| format!("s-{n}")).collect();
        let refs: Vec<&str> = mine.iter().map(String::as_str).collect();
        let folders = [holding(CLIENT_A, &refs), holding(WORK, &refs[..11])];
        assert_eq!(
            folder_of_records(&mine, &folders, &[]),
            Some(Path::new(CLIENT_A))
        );
        let folders = [
            holding(TEAM_A, &[]),
            holding(CLIENT_A, &refs[..3]),
            holding(WORK, &refs[..9]),
        ];
        assert_eq!(
            folder_of_records(&mine, &folders, &[Path::new(TEAM_A)]),
            Some(Path::new(WORK))
        );
    }

    /// The order: its running sessions, its records, its app's declared
    /// folder, AI Juggler's own record, then the name: a fallback only.
    #[test]
    fn the_strongest_evidence_decides() {
        let (a, b, c) = (Path::new(CLIENT_A), Path::new(WORK), Path::new(TEAM_A));
        let all = Evidence {
            running: Some(a),
            records: Some(b),
            recorded: Some(c),
            declared: None,
            named: Some(c),
        };
        assert_eq!(decide(&all), Some((a, LinkSource::Running)));
        let closed = Evidence {
            running: None,
            ..all
        };
        assert_eq!(decide(&closed), Some((b, LinkSource::Records)));
        let recalled = Evidence {
            records: None,
            ..closed
        };
        assert_eq!(decide(&recalled), Some((c, LinkSource::Recorded)));
        let guessed = Evidence {
            recorded: None,
            ..recalled
        };
        assert_eq!(decide(&guessed), Some((c, LinkSource::Named)));
        assert_eq!(decide(&Evidence::default()), None);
        for source in [
            LinkSource::Running,
            LinkSource::Records,
            LinkSource::Recorded,
            LinkSource::Declared,
            LinkSource::Named,
        ] {
            assert_eq!(LinkSource::of_word(source.word()), Some(source));
        }
        assert!(LinkSource::Running.is_evidence() && !LinkSource::Named.is_evidence());
    }

    /// For AJ-E2317: only proof contradicts a link; the name or a recorded
    /// folder never does.
    #[test]
    fn only_proof_contradicts_a_link() {
        let (a, b) = (Path::new(CLIENT_A), Path::new(TEAM_A));
        let running = Evidence {
            running: Some(a),
            named: Some(b),
            ..Evidence::default()
        };
        assert_eq!(contradicts(&running, b), Some((a, LinkSource::Running)));
        assert_eq!(contradicts(&running, a), None, "the link is the evidence");
        let guessed = Evidence {
            named: Some(a),
            recorded: Some(a),
            ..Evidence::default()
        };
        assert_eq!(contradicts(&guessed, b), None);
        let records = Evidence {
            records: Some(a),
            ..Evidence::default()
        };
        assert_eq!(contradicts(&records, b), Some((a, LinkSource::Records)));
    }
}
