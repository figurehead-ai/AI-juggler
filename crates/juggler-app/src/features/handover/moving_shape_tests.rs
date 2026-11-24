//! AJ-E2323 R1 (the review's D1): every ownership shape the move must
//! tell apart, as a process table built from `procs::fixture` (no OS: no
//! process is read, quit or started). Each shape runs through the move's
//! own decisions: the profiles' trees by the shared ownership rule
//! (`places::tree::of`, AJ-E2315), the writers by Claude's hook (AJ-E2318),
//! the self rule, and `stop_places`/the offer, and asserts **what
//! blocks** (what the move waits on or refuses for), **what is
//! reported** (the move's "still running" notes), and **the refusal
//! class** (`StopClass`, AJ-E2319).
//!
//! R5 (AJ-E2318's ≤ 500 ms): the offer's checks on a large table.

use std::ffi::OsStr;
use std::time::Instant;

use super::*;
use crate::error::{AppError, StopClass};
use crate::features::conversations::procs::{Proc as Row, fixture};
use crate::features::ownership::InstanceKey;
use crate::features::places::tree;

/// AI Juggler's own pid in every table.
const OWN: u32 = 9999;
/// Claude Code's moving family (the lab's `main` and its `kid`).
const FAMILY: [&str; 2] = ["main", "kid"];

/// One profile the table knows: its place id (when the book has it), its
/// folder, and its main process while it runs.
struct Known {
    id: Option<String>,
    dir: PathBuf,
    main: Option<u32>,
}

/// The scripted system over a fixture table: quitting a main process
/// ends it and its sessions (`ends`); everything else it started is
/// reparented to launchd and keeps running.
struct Shaped {
    table: Mutex<Table>,
    known: Vec<Known>,
    /// Processes that end when their profile's main process quits.
    ends: Vec<u32>,
    /// Pids the system gave another program since the table was read.
    reused: Vec<u32>,
    quits: Mutex<Vec<u32>>,
    starts: Mutex<Vec<String>>,
    /// How often the process table was read.
    reads: Mutex<usize>,
}

impl Shaped {
    fn new(fix: &Fix, rows: Vec<(u32, Row)>) -> Self {
        Self {
            table: Mutex::new(rows.into_iter().collect()),
            known: vec![
                Known {
                    id: Some(source_id(&fix.lab)),
                    dir: fix.a.clone(),
                    main: Some(100),
                },
                Known {
                    id: lab_id(&fix.lab, false),
                    dir: fix.b.clone(),
                    main: Some(200),
                },
            ],
            ends: Vec::new(),
            reused: Vec::new(),
            quits: Mutex::default(),
            starts: Mutex::default(),
            reads: Mutex::default(),
        }
    }

    /// Every other profile than `place` (its folder and main process).
    fn others(&self, table: &Table, place: &KnownPlace) -> tree::Others {
        let this = place.instance_dir.as_deref().map(InstanceKey::of);
        let rest: Vec<&Known> = self
            .known
            .iter()
            .filter(|k| Some(InstanceKey::of(&k.dir)) != this)
            .collect();
        tree::Others::of(
            rest.iter().map(|k| k.dir.clone()),
            rest.iter()
                .filter_map(|k| k.main)
                .filter(|pid| table.contains_key(pid))
                .collect(),
        )
    }

    fn tree_of(&self, place: &KnownPlace, roots: &[u32]) -> Vec<Member> {
        let table = self.table.lock().unwrap().clone();
        let others = self.others(&table, place);
        tree::of(&table, place, Some("--user-data-dir"), roots, OWN, &others)
    }

    fn quits(&self) -> Vec<u32> {
        self.quits.lock().unwrap().clone()
    }
}

impl MoveSystem for Shaped {
    fn running(&self, _: &Arc<AppState>, id: &str) -> Option<(u32, Option<i64>)> {
        let table = self.table.lock().unwrap();
        let main = self
            .known
            .iter()
            .find(|k| k.id.as_deref() == Some(id))?
            .main?;
        table.get(&main).map(|row| (main, row.start))
    }
    fn quit(&self, pid: u32) -> Result<(), String> {
        self.quits.lock().unwrap().push(pid);
        let mut table = self.table.lock().unwrap();
        let below: Vec<u32> = table
            .values()
            .filter(|row| {
                crate::features::conversations::procs::ancestors(&table, row.pid)
                    .iter()
                    .any(|above| above.pid == pid)
            })
            .map(|row| row.pid)
            .collect();
        table.remove(&pid);
        for gone in below.iter().filter(|p| self.ends.contains(p)) {
            table.remove(gone);
        }
        // Launchd takes what outlives it.
        for row in table.values_mut().filter(|row| row.ppid == Some(pid)) {
            row.ppid = Some(1);
        }
        Ok(())
    }
    fn identify(&self, pids: &[u32]) -> HashMap<u32, ProcId> {
        let table = self.table.lock().unwrap();
        pids.iter()
            .filter_map(|pid| table.get(pid))
            .map(|row| {
                let later = if self.reused.contains(&row.pid) { 7 } else { 0 };
                let secs = u128::try_from(row.start.unwrap_or_default() + later).unwrap();
                (
                    row.pid,
                    ProcId {
                        pid: row.pid,
                        start_micros: secs * 1_000_000,
                    },
                )
            })
            .collect()
    }
    fn table(&self, _: &Arc<AppState>) -> Table {
        *self.reads.lock().unwrap() += 1;
        self.table.lock().unwrap().clone()
    }
    fn start(&self, _: &Arc<AppState>, place: &KnownPlace) -> Result<(), String> {
        self.starts.lock().unwrap().push(fixtures::id_of(place));
        Ok(())
    }
    fn working(&self, _: &Arc<AppState>, _: u32) -> usize {
        0
    }
    fixtures::quiet_parts!(Duration::from_millis(200));
    fn tree(&self, _: &Arc<AppState>, place: &KnownPlace, roots: &[u32]) -> Vec<Member> {
        self.tree_of(place, roots)
    }
    // As `tree::quits_juggler_now`, over the fixture table.
    fn quits_juggler(
        &self,
        state: &Arc<AppState>,
        place: &KnownPlace,
        pid: u32,
        action: &str,
    ) -> Option<String> {
        let members: Vec<u32> = self.tree_of(place, &[pid]).iter().map(|m| m.pid).collect();
        let table = self.table.lock().unwrap().clone();
        let juggler = crate::features::ownership::self_rule::quits_juggler(
            &table,
            &members,
            OWN,
            OsStr::new("juggler-app"),
        )?;
        Some(crate::features::ownership::self_rule::words(
            juggler,
            &place_name(state, place),
            action,
        ))
    }
    fn link_mismatch(
        &self,
        _: &Arc<AppState>,
        _: &str,
        _: &KnownPlace,
        _: &Path,
    ) -> Option<PathBuf> {
        None
    }
}

fn lab_id(lab: &Lab, source: bool) -> Option<String> {
    let side = if source {
        &lab.sides.source_place
    } else {
        &lab.sides.target_place
    };
    side.as_ref().map(|(id, _)| id.clone())
}

/// The move lab, with its folders in their real spelling (as the system
/// names a process's program).
struct Fix {
    lab: Lab,
    root: PathBuf,
    bundle: PathBuf,
    /// The source and target profiles' folders.
    a: PathBuf,
    b: PathBuf,
}

fn fix() -> Fix {
    let lab = lab();
    let real = |p: &Path| std::fs::canonicalize(p).unwrap();
    let place = |side: &Option<(String, KnownPlace)>| {
        real(side.as_ref().unwrap().1.instance_dir.as_deref().unwrap())
    };
    let a = place(&lab.sides.source_place);
    let b = place(&lab.sides.target_place);
    let bundle = real(&lab.sides.source_place.as_ref().unwrap().1.bundle);
    let root = a.parent().unwrap().parent().unwrap().to_path_buf();
    Fix {
        lab,
        root,
        bundle,
        a,
        b,
    }
}

impl Fix {
    fn main_exe(&self) -> String {
        self.bundle
            .join("Contents/MacOS/Claude")
            .display()
            .to_string()
    }

    /// A profile's main process, `pid`, on `dir`.
    fn main(&self, pid: u32, dir: &Path) -> (u32, Row) {
        let exe = self.main_exe();
        fixture(
            pid,
            1,
            &exe,
            &[&exe, &format!("--user-data-dir={}", dir.display())],
        )
    }

    /// One of the app's helpers below `parent`, on `dir`.
    fn helper(&self, pid: u32, parent: u32, dir: &Path) -> (u32, Row) {
        let exe = self
            .bundle
            .join("Contents/Frameworks/Claude Helper (Renderer).app/Contents/MacOS/Claude Helper (Renderer)")
            .display()
            .to_string();
        fixture(
            pid,
            parent,
            &exe,
            &[
                &exe,
                "--type=renderer",
                &format!("--user-data-dir={}", dir.display()),
            ],
        )
    }

    /// A profile's Code tab below `main`: the app's `disclaimer` wrapper
    /// (`pid`) running the profile's Claude Code (`pid + 1`) with `args`.
    fn code_tab(&self, pid: u32, main: u32, dir: &Path, args: &[&str]) -> [(u32, Row); 2] {
        let code = dir
            .join("claude-code/2.1/abc/claude.app/Contents/MacOS/claude")
            .display()
            .to_string();
        let wrapper = self
            .bundle
            .join("Contents/Helpers/disclaimer")
            .display()
            .to_string();
        let mut wrapped = vec![wrapper.as_str(), "--pgroup", "--", code.as_str()];
        wrapped.extend(args);
        let mut own = vec![code.as_str()];
        own.extend(args);
        [
            fixture(pid, main, &wrapper, &wrapped),
            fixture(pid + 1, pid, &code, &own),
        ]
    }

    /// Another profile's folder `name`, made.
    fn profile(&self, name: &str) -> PathBuf {
        let dir = self.root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(dir).unwrap()
    }

    fn writing(&self) -> Writing<'_> {
        Writing {
            mover: self.lab.sides.adapter.in_place_move().unwrap(),
            dirs: vec![self.lab.a_code.clone(), self.lab.b_code.clone()],
            conversations: FAMILY.iter().map(|c| (*c).to_owned()).collect(),
        }
    }

    fn places(&self) -> Vec<(String, KnownPlace)> {
        [&self.lab.sides.source_place, &self.lab.sides.target_place]
            .into_iter()
            .flatten()
            .cloned()
            .collect()
    }

    /// The move's single-writer phase over `system`: quits, waits,
    /// reports (no file is touched).
    fn stop(&self, system: &Shaped) -> AppResult<quit::Stopping> {
        let places = self.places();
        stop_places(
            &self.lab.state,
            (&places, &self.lab.sides.request),
            system,
            "moved",
            &self.writing(),
        )
    }

    fn source_name(&self) -> String {
        place_name(
            &self.lab.state,
            &self.lab.sides.source_place.as_ref().unwrap().1,
        )
    }

    /// Claude Code's pid file for `pid` on `session`, recording `start`.
    fn pid_file(&self, pid: u32, session: &str, start: i64) {
        pid_file_in(&self.lab.a_code, pid, session, start);
    }
}

/// A pid file in `code`'s `sessions/`, as Claude Code writes it (its
/// `procStart` a ctime in UTC; under a day after the epoch here).
fn pid_file_in(code: &Path, pid: u32, session: &str, start: i64) {
    let dir = code.join("sessions");
    std::fs::create_dir_all(&dir).unwrap();
    let at = format!(
        "Thu Jan  1 {:02}:{:02}:{:02} 1970",
        start / 3600,
        start / 60 % 60,
        start % 60
    );
    std::fs::write(
        dir.join(format!("{pid}.json")),
        format!(r#"{{"pid":{pid},"sessionId":"{session}","status":"idle","procStart":"{at}"}}"#),
    )
    .unwrap();
}

/// The stop a refusal carries.
fn stop_of<T>(result: AppResult<T>) -> crate::error::MoveStop {
    let stop = match result.err() {
        Some(AppError::Move(stop)) => Some(*stop),
        _ => None,
    };
    stop.expect("refused with a move stop")
}

fn pids(members: &[Member]) -> Vec<u32> {
    members.iter().map(|m| m.pid).collect()
}

fn source_place(fix: &Fix) -> KnownPlace {
    fix.lab.sides.source_place.clone().unwrap().1
}

/// Both profiles running, the source with `rows` more.
fn both(fix: &Fix, rows: impl IntoIterator<Item = (u32, Row)>) -> Vec<(u32, Row)> {
    let mut out = vec![fix.main(100, &fix.a), fix.main(200, &fix.b)];
    out.extend(rows);
    out
}

/// Shape 1: The live trace: main, 9 helpers, the Code tab's wrapper and Claude
/// Code (resuming the moving conversation), its node server. Waited on:
/// the main process and the session; once main is gone, the helpers and
/// node (launchd's now) are only reported. A session that never ends
/// refuses as transient, naming it and its profile.
#[test]
fn d1_01_the_live_trace_waits_on_main_and_the_session_only() {
    let fix = fix();
    let mut rows = both(&fix, fix.code_tab(110, 100, &fix.a, &["--resume=main"]));
    rows.extend((101..=109).map(|pid| fix.helper(pid, 100, &fix.a)));
    rows.push(fixture(
        112,
        111,
        "/usr/local/bin/node",
        &["node", "mcp-server.js"],
    ));
    let mut system = Shaped::new(&fix, rows.clone());
    system.ends = vec![110, 111];
    let tree = system.tree_of(&source_place(&fix), &[100]);
    assert_eq!(
        pids(&tree),
        (100..=112).collect::<Vec<_>>(),
        "its whole tree"
    );
    let done = fix.stop(&system).unwrap();
    assert_eq!(
        system.quits(),
        [100, 200],
        "only the main processes are asked"
    );
    let note = done.notes.join(" | ");
    assert!(
        note.contains("node") && note.contains("9 Claude Helper processes"),
        "{note}"
    );
    assert!(
        !note.contains("claude,") && !note.contains("disclaimer"),
        "{note}"
    );
    // The session that doesn't end: what blocks, as transient.
    let system = Shaped::new(&fix, rows);
    let stop = stop_of(fix.stop(&system));
    assert_eq!(stop.class, StopClass::Transient);
    assert!(stop.words.contains("claude still runs"), "{}", stop.words);
    assert!(
        !stop.words.contains("node") && !stop.words.contains("Helper"),
        "{}",
        stop.words
    );
    assert_eq!(stop.holders[0].place_id, lab_id(&fix.lab, true));
}

/// Another app profile AI Juggler knows of (no id), on `dir`, its main
/// process `main`.
fn other_app(dir: PathBuf, main: u32) -> Known {
    Known {
        id: None,
        dir,
        main: Some(main),
    }
}

/// The two profiles stop with nothing else waited on or reported.
fn stops_both_quietly(fix: &Fix, system: &Shaped) {
    let done = fix.stop(system).unwrap();
    assert_eq!(system.quits(), [100, 200]);
    assert!(done.notes.is_empty(), "{:?}", done.notes);
}

/// The offer holds Move back as the person's to clear, saying `words`.
fn offer_asks_you(fix: &Fix, system: &Shaped, words: &str) {
    let offer = offer_of(&fix.lab.state, &fix.lab.sides, system);
    assert!(
        offer
            .blocks
            .iter()
            .any(|b| b.kind == MoveBlockKind::UserCanClear && b.words.contains(words)),
        "{:?}",
        offer.blocks
    );
}

/// Shape 2: AJ-E2315: another profile's `disclaimer` and Claude Code carrying
/// `--add-dir=` into this profile's folder are never this profile's: not
/// in its tree, never waited on, never reported, never quit.
#[test]
fn d1_02_another_profiles_add_dir_is_never_this_profiles() {
    let fix = fix();
    let other = fix.profile("Profiles/team-a");
    let add = format!("--add-dir={}/local-agent-mode-sessions", fix.a.display());
    let mut rows = both(&fix, [fix.main(300, &other)]);
    rows.extend(fix.code_tab(301, 300, &other, &[&add]));
    let mut system = Shaped::new(&fix, rows);
    system.known.push(other_app(other, 300));
    assert_eq!(pids(&system.tree_of(&source_place(&fix), &[100])), [100]);
    // Even with its main process gone (launchd's wrapper).
    system.table.lock().unwrap().get_mut(&301).unwrap().ppid = Some(1);
    assert_eq!(pids(&system.tree_of(&source_place(&fix), &[100])), [100]);
    stops_both_quietly(&fix, &system);
}

/// Shape 3: A `nohup` dev server below the Code tab's Claude Code: reported,
/// not waited on.
#[test]
fn d1_03_a_nohup_dev_server_is_reported_not_waited_on() {
    let fix = fix();
    let mut rows = both(&fix, fix.code_tab(110, 100, &fix.a, &["--resume=main"]));
    rows.push(fixture(
        113,
        111,
        "/usr/local/bin/node",
        &["node", "server.js"],
    ));
    let mut system = Shaped::new(&fix, rows);
    system.ends = vec![110, 111];
    let done = fix.stop(&system).unwrap();
    assert_eq!(system.quits(), [100, 200]);
    assert_eq!(
        done.notes,
        [format!(
            "node that {} started is still running; the move doesn't need it closed",
            fix.source_name()
        )]
    );
}

/// Shape 4: The crash reporter: started by the main process, then launchd's,
/// is reported, not waited on. One already launchd's when the move looks,
/// naming the profile only in `--database=`, is nobody's that can be told
/// (a flag's value is never evidence): neither waited on nor reported.
#[test]
fn d1_04_crashpad_is_reported_never_waited_on() {
    let fix = fix();
    let exe = fix
        .bundle
        .join("Contents/Frameworks/Electron Framework.framework/Helpers/chrome_crashpad_handler")
        .display()
        .to_string();
    let database = format!("--database={}/Crashpad", fix.a.display());
    let system = Shaped::new(
        &fix,
        both(&fix, [fixture(114, 100, &exe, &[&exe, &database])]),
    );
    let done = fix.stop(&system).unwrap();
    assert_eq!(system.quits(), [100, 200]);
    assert!(
        done.notes[0].starts_with("chrome_crashpad_handler that"),
        "{:?}",
        done.notes
    );
    let system = Shaped::new(
        &fix,
        both(&fix, [fixture(114, 1, &exe, &[&exe, &database])]),
    );
    assert_eq!(pids(&system.tree_of(&source_place(&fix), &[100])), [100]);
    let done = fix.stop(&system).unwrap();
    assert!(done.notes.is_empty(), "{:?}", done.notes);
}

/// Shape 5: This AI Juggler (the Dev build) started from the profile's Code
/// tab: the self rule refuses before anything quits, as needs you.
#[test]
fn d1_05_ai_juggler_from_the_code_tab_is_refused_before_any_quit() {
    let fix = fix();
    let mut rows = both(&fix, fix.code_tab(110, 100, &fix.a, &[]));
    rows.push(fixture(
        OWN,
        111,
        "/Users/you/Dev/target/debug/juggler-app",
        &["juggler-app"],
    ));
    let system = Shaped::new(&fix, rows);
    offer_asks_you(&fix, &system, "AI Juggler itself was started from inside");
    let stop = stop_of(run(&fix.lab.state, &fix.lab.sides, &system));
    assert_eq!(stop.class, StopClass::NeedsYou);
    assert!(stop.words.contains("AI Juggler itself"), "{}", stop.words);
    assert!(system.quits().is_empty(), "nothing quit");
    assert!(unchanged(&fix.lab));
}

/// Shape 6: Another AI Juggler started from the profile: the same, naming it.
#[test]
fn d1_06_another_ai_juggler_in_the_profile_is_refused_before_any_quit() {
    let fix = fix();
    let mut rows = both(&fix, fix.code_tab(110, 100, &fix.a, &[]));
    rows.push(fixture(
        OWN,
        1,
        "/Applications/AI Juggler.app/Contents/MacOS/juggler-app",
        &["juggler-app"],
    ));
    rows.push(fixture(
        950,
        111,
        "/Users/you/Dev/target/debug/juggler-app",
        &["juggler-app"],
    ));
    let system = Shaped::new(&fix, rows);
    let stop = stop_of(run(&fix.lab.state, &fix.lab.sides, &system));
    assert_eq!(stop.class, StopClass::NeedsYou);
    assert!(
        stop.words
            .contains("another AI Juggler was started from inside"),
        "{}",
        stop.words
    );
    assert!(system.quits().is_empty(), "nothing quit");
}

/// Shape 7: Instance spellings of a helper launchd took (only its flag tells):
/// a trailing slash, a link to the folder and another letter case are
/// this profile's (reported, never waited on); the two-word form
/// (`--user-data-dir X`), a second `--user-data-dir` or one after `--` is
/// refused by the strict parse and tells nothing (the lock's holder
/// decides a main process: `ownership::tests::
/// an_untrusted_instance_flag_is_decided_by_the_lock_its_process_holds`).
#[test]
fn d1_07_instance_spellings_decide_alike_and_strictly() {
    let fix = fix();
    let link = fix.root.join("link-to-a");
    std::os::unix::fs::symlink(&fix.a, &link).unwrap();
    let a = fix.a.display().to_string();
    let upper = a.replace("/Profiles/", "/PROFILES/");
    let b = fix.b.display().to_string();
    let spelled = |pid: u32, name: &str, args: &[&str]| {
        let exe = fix
            .bundle
            .join("Contents/Frameworks")
            .join(name)
            .display()
            .to_string();
        let mut all = vec![exe.as_str()];
        all.extend(args);
        fixture(pid, 1, &exe, &all)
    };
    let (slash, two, linked, case) = (
        format!("--user-data-dir={a}/"),
        a.clone(),
        format!("--user-data-dir={}", link.display()),
        format!("--user-data-dir={upper}"),
    );
    let (twice_a, twice_b) = (
        format!("--user-data-dir={a}"),
        format!("--user-data-dir={b}"),
    );
    let rows = both(
        &fix,
        [
            spelled(120, "slash", &[&slash]),
            spelled(121, "twoword", &["--user-data-dir", &two]),
            spelled(122, "linked", &[&linked]),
            spelled(123, "cased", &[&case]),
            spelled(124, "twice", &[&twice_a, &twice_b]),
            spelled(125, "afterdashes", &["--", &twice_a]),
        ],
    );
    let system = Shaped::new(&fix, rows);
    assert_eq!(
        pids(&system.tree_of(&source_place(&fix), &[100])),
        [100, 120, 122, 123]
    );
    let done = fix.stop(&system).unwrap();
    assert_eq!(system.quits(), [100, 200], "none waited on");
    let note = done.notes.join(" | ");
    for name in ["slash", "linked", "cased"] {
        assert!(note.contains(name), "{name}: {note}");
    }
    assert!(
        !note.contains("twoword") && !note.contains("twice") && !note.contains("afterdashes"),
        "{note}"
    );
}

/// Shape 8: Two profiles on one account, the conversation in the one not
/// running: the move closes the profile holding the record (the source,
/// by its records: `picking::two_profiles_on_one_account_the_source_by_
/// its_records_the_target_by_the_pick`), never the running one beside it.
#[test]
fn d1_08_the_holder_is_closed_never_its_running_sibling() {
    let fix = fix();
    let sibling = fix.profile("Profiles/team-a");
    let mut rows = vec![fix.main(200, &fix.b), fix.main(300, &sibling)];
    rows.extend(fix.code_tab(301, 300, &sibling, &[]));
    let mut system = Shaped::new(&fix, rows.clone());
    system.known.push(Known {
        id: None,
        dir: sibling.clone(),
        main: Some(300),
    });
    let done = fix.stop(&system).unwrap();
    assert_eq!(
        system.quits(),
        [200],
        "the holder isn't running; its sibling isn't quit"
    );
    assert!(!done.stopped[0].was_running && done.notes.is_empty());
    // The holder running: it is the one quit.
    rows.push(fix.main(100, &fix.a));
    let mut system = Shaped::new(&fix, rows);
    system.known.push(other_app(sibling, 300));
    fix.stop(&system).unwrap();
    assert_eq!(system.quits(), [100, 200]);
}

/// Shape 9: Two Apps on the target's account, the second picked: it is the
/// target (by its id: `picking::the_target_is_the_app_picked_by_its_id_
/// when_two_share_an_account`); the other is never quit.
#[test]
fn d1_09_the_picked_target_is_closed_never_the_other_app() {
    let fix = fix();
    let first = fix.profile("Profiles/team-b");
    let mut system = Shaped::new(&fix, both(&fix, [fix.main(400, &first)]));
    system.known.push(other_app(first, 400));
    stops_both_quietly(&fix, &system);
}

/// Shape 10: A pid the system gave another program between the move's look and
/// its wait: a session's is neither waited on nor reported; the main
/// process's is never signalled (transient, nothing quit).
#[test]
fn d1_10_a_reused_pid_is_never_waited_on_or_signalled() {
    let fix = fix();
    let rows = both(&fix, fix.code_tab(110, 100, &fix.a, &["--resume=main"]));
    let mut system = Shaped::new(&fix, rows.clone());
    system.ends = vec![110];
    system.reused = vec![111];
    stops_both_quietly(&fix, &system);
    let mut system = Shaped::new(&fix, rows);
    system.reused = vec![100];
    let stop = stop_of(fix.stop(&system));
    assert_eq!(stop.class, StopClass::Transient);
    assert!(
        stop.words.contains("couldn't be sure which process"),
        "{}",
        stop.words
    );
    assert!(system.quits().is_empty(), "never signalled");
}

/// Shape 11: A pid file in `sessions/` whose `procStart` isn't the process's
/// start: stale, not a writer (only reported); its start matching: a
/// writer, waited on (transient when it doesn't end).
#[test]
fn d1_11_a_stale_pid_file_is_no_writer() {
    let fix = fix();
    let rows = both(&fix, fix.code_tab(110, 100, &fix.a, &[]));
    fix.pid_file(111, "main", 3000);
    let mut system = Shaped::new(&fix, rows.clone());
    system.ends = vec![110];
    let done = fix.stop(&system).unwrap();
    assert!(done.notes[0].starts_with("claude that"), "{:?}", done.notes);
    fix.pid_file(111, "main", 1110);
    let mut system = Shaped::new(&fix, rows);
    system.ends = vec![110];
    let stop = stop_of(fix.stop(&system));
    assert_eq!(stop.class, StopClass::Transient);
    assert!(stop.words.contains("claude still runs"), "{}", stop.words);
}

/// Shape 12: A headless `claude -p --resume <id>` on the source's folder from a
/// terminal (no live-view entry): a writer no profile runs, named and
/// refused as needs you, before anything quits.
#[test]
fn d1_12_a_headless_resume_outside_the_apps_needs_you() {
    let fix = fix();
    let rows = both(
        &fix,
        [
            fixture(600, 1, "/bin/zsh", &["-zsh"]),
            fixture(
                700,
                600,
                "/Users/you/.local/bin/claude",
                &["claude", "-p", "--resume", "kid"],
            ),
        ],
    );
    let system = Shaped::new(&fix, rows);
    offer_asks_you(
        &fix,
        &system,
        "claude (process 700) has this conversation open outside",
    );
    let stop = stop_of(fix.stop(&system));
    assert_eq!(stop.class, StopClass::NeedsYou);
    assert_eq!(stop.holders[0].name, "claude");
    assert!(system.quits().is_empty(), "nothing quit");
}

/// Shape 13: A process each profile's evidence gives the other (the target's
/// program, the source's instance flag): the ownership rule makes it
/// neither's, so it is never a wait. Writing the family, it is refused
/// before anything quits, named (needs you: the person ends it); writing
/// nothing, it is ignored.
#[test]
fn d1_13_a_process_two_profiles_claim_is_never_a_wait() {
    let fix = fix();
    let code = fix
        .b
        .join("claude-code/2.1/abc/claude.app/Contents/MacOS/claude")
        .display()
        .to_string();
    let flag = format!("--user-data-dir={}", fix.a.display());
    let claimed = |args: &[&str]| {
        let mut all = vec![code.as_str(), flag.as_str()];
        all.extend(args);
        fixture(130, 1, &code, &all)
    };
    let system = Shaped::new(&fix, both(&fix, [claimed(&["--resume=main"])]));
    let target = fix.lab.sides.target_place.clone().unwrap().1;
    assert_eq!(pids(&system.tree_of(&source_place(&fix), &[100])), [100]);
    assert_eq!(pids(&system.tree_of(&target, &[200])), [200]);
    let stop = stop_of(fix.stop(&system));
    assert_eq!(stop.class, StopClass::NeedsYou);
    assert!(
        stop.words.contains("claude (process 130)"),
        "{}",
        stop.words
    );
    assert!(system.quits().is_empty(), "never a wait: nothing quit");
    let system = Shaped::new(&fix, both(&fix, [claimed(&[])]));
    stops_both_quietly(&fix, &system);
}

/// R5 (AJ-E2318 R3): the offer's checks (one process table, the pid
/// files of both accounts' folders, one tree per profile, the self rule)
/// stay within 500 ms on a large table: 3,000 other processes, six
/// running profiles with helpers and Code tabs, 400 pid files.
#[test]
fn the_offers_checks_stay_within_500_ms_on_a_large_table() {
    let fix = fix();
    let mut rows = both(&fix, []);
    let mut known = Vec::new();
    for n in 0..4u32 {
        let dir = fix.profile(&format!("Profiles/p{n}"));
        let main = 300 + n * 100;
        rows.push(fix.main(main, &dir));
        known.push(Known {
            id: None,
            dir,
            main: Some(main),
        });
    }
    let mut dirs: Vec<(u32, PathBuf)> = vec![(100, fix.a.clone()), (200, fix.b.clone())];
    dirs.extend(known.iter().map(|k| (k.main.unwrap(), k.dir.clone())));
    for (main, dir) in &dirs {
        rows.extend((1..=9).map(|n| fix.helper(main + n, *main, dir)));
        for tab in 0..3 {
            let at = main + 20 + tab * 10;
            rows.extend(fix.code_tab(at, *main, dir, &["--resume=other"]));
            rows.push(fixture(
                at + 2,
                at + 1,
                "/usr/local/bin/node",
                &["node", "server.js"],
            ));
        }
    }
    for pid in 2000..5000u32 {
        let exe = format!("/usr/libexec/daemon{}", pid % 97);
        let parent = if pid % 5 == 0 { 1 } else { pid - 1 };
        let flag = format!("--config=/Library/Preferences/{pid}.plist");
        rows.push(fixture(pid, parent, &exe, &[&exe, &flag]));
    }
    for pid in 2000..2200u32 {
        pid_file_in(&fix.lab.a_code, pid, "other", 1);
        pid_file_in(&fix.lab.b_code, pid + 200, "another", 1);
    }
    let mut system = Shaped::new(&fix, rows);
    system.known.extend(known);
    let mut best = Duration::MAX;
    for _ in 0..3 {
        let at = Instant::now();
        let offer = offer_of(&fix.lab.state, &fix.lab.sides, &system);
        best = best.min(at.elapsed());
        assert!(offer.blocks.is_empty(), "{:?}", offer.blocks);
    }
    assert!(
        best <= Duration::from_millis(500),
        "the offer's checks took {best:?}"
    );
    assert_eq!(
        *system.reads.lock().unwrap(),
        3,
        "one process table per offer"
    );
    eprintln!(
        "the offer's checks on {} processes: {best:?}",
        system.table.lock().unwrap().len()
    );
}
