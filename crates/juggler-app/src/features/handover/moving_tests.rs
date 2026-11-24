//! ADR-0031: the process side of a move over a real app state, real
//! fixture folders and a scripted system (no app is quit or started).

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::*;
use juggler_adapter::moving::MoveSide;

const SLUG: &str = "-w-proj";

/// A process the script runs: its pid, start, and whether it quits.
#[derive(Clone, Copy)]
struct Proc {
    pid: u32,
    start: Option<i64>,
    alive: bool,
}

/// A process of a profile besides its main one (a helper, the Code tab's
/// Claude Code): it outlives the main process as the live trace showed.
struct Kid {
    pid: u32,
    name: &'static str,
    /// Its arguments after its name (`--resume=main` makes it a writer of
    /// the moving conversation).
    args: &'static [&'static str],
    /// AJ-E2318: it is the profile's (in its tree); else it runs outside
    /// every profile (a terminal session).
    in_tree: bool,
    /// The place it belongs to.
    place: String,
    alive: Arc<AtomicBool>,
    /// Once its main process quits: after the first wait it appends the
    /// line to the file (its last write), after the second it ends.
    /// `None`: it never ends.
    after_quit: Option<(Duration, PathBuf, &'static [u8], Duration)>,
}

/// A process of the source profile's (`in_tree`) or outside every profile,
/// running `name` with `args`; it never ends.
fn kid(
    lab: &Lab,
    pid: u32,
    name: &'static str,
    args: &'static [&'static str],
    in_tree: bool,
) -> Kid {
    Kid {
        pid,
        name,
        args,
        in_tree,
        place: source_id(lab),
        alive: Arc::new(AtomicBool::new(true)),
        after_quit: None,
    }
}

/// Appends `line` to `path` when it is there (a writer never makes it).
fn append(path: &Path, line: &[u8]) {
    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(path) {
        file.write_all(line).unwrap();
    }
}

#[derive(Default)]
struct Scripted {
    procs: Mutex<HashMap<String, Proc>>,
    /// Places that never quit when asked.
    stubborn: Vec<String>,
    /// Places whose process isn't what it was (a reused pid).
    reused: Vec<String>,
    /// Places the user starts again from the Dock as soon as they quit.
    restarted_by_user: Vec<String>,
    /// Sessions of the accounts open outside the two profiles.
    elsewhere: usize,
    /// AJ-E2318: places whose pid is given to another program between the
    /// move's look and its signal.
    reused_after_read: Vec<String>,
    /// AJ-E2318: how long a main process takes to end once asked.
    slow: Option<Duration>,
    /// When each slow main process ends.
    dies_at: Mutex<HashMap<u32, std::time::Instant>>,
    /// How many profiles ran at each quit (itself included).
    running_at_quit: Mutex<Vec<usize>>,
    /// How often each pid was looked up.
    looks: Mutex<HashMap<u32, usize>>,
    working: usize,
    quits: Mutex<Vec<u32>>,
    starts: Mutex<Vec<String>>,
    sent: Mutex<Vec<(u32, String)>>,
    next_pid: Mutex<u32>,
    /// What else runs of each profile.
    kids: Vec<Kid>,
    /// Their last writes and ends, running.
    endings: Mutex<Vec<std::thread::JoinHandle<()>>>,
    /// A writer no process names: appends `line` to the file at each
    /// settle, this many times.
    writer: Mutex<Option<(PathBuf, &'static [u8], usize)>>,
    /// How long a profile is given to quit (300 ms when not set).
    wait: Option<Duration>,
    /// What the move told the switch screen.
    told: Mutex<Vec<MoveProgressInfo>>,
    /// The place an AI Juggler was started from inside (the self rule).
    juggler_in: Option<String>,
    /// AJ-E2317: places whose own sessions use another folder than the
    /// one they're linked to (AJ-E2327's evidence), and that folder.
    mismatched: Mutex<HashMap<String, PathBuf>>,
    /// AJ-E2317: one such place, found so only once the apps quit.
    mismatch_on_quit: Mutex<Option<(String, PathBuf)>>,
    /// AJ-E2320: the shared move area, and whether a profile was held in
    /// it at each quit.
    area: Option<PathBuf>,
    held_at_quit: Mutex<Vec<bool>>,
    /// AJ-E2322: the target app opens the conversation on the link (its
    /// Claude Code resumes it, [`OPENED_PID`], until the app quits).
    opens: bool,
    /// The app process the link went to, while it has it open.
    linked: Mutex<Option<u32>>,
    /// How long each settle asked for.
    settles: Mutex<Vec<Duration>>,
}

/// AJ-E2322: the target app's Claude Code that resumes the conversation.
const OPENED_PID: u32 = 777;

impl Scripted {
    /// Ends the slow main processes whose time came.
    fn settle_deaths(&self) {
        let now = std::time::Instant::now();
        let due: Vec<u32> = self
            .dies_at
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(pid, _)| *pid)
            .collect();
        let mut procs = self.procs.lock().unwrap();
        for p in procs.values_mut().filter(|p| due.contains(&p.pid)) {
            p.alive = false;
        }
    }

    fn place_of(&self, pid: u32) -> Option<String> {
        self.procs
            .lock()
            .unwrap()
            .iter()
            .find(|(_, p)| p.pid == pid)
            .map(|(id, _)| id.clone())
    }
}

impl MoveSystem for Scripted {
    fn running(&self, _: &Arc<AppState>, id: &str) -> Option<(u32, Option<i64>)> {
        self.settle_deaths();
        self.procs
            .lock()
            .unwrap()
            .get(id)
            .filter(|p| p.alive)
            .map(|p| (p.pid, p.start))
    }
    fn quit(&self, pid: u32) -> Result<(), String> {
        self.settle_deaths();
        self.quits.lock().unwrap().push(pid);
        {
            let mut linked = self.linked.lock().unwrap();
            if *linked == Some(pid) {
                *linked = None;
            }
        }
        if let Some(area) = &self.area {
            let held = crate::features::places::hold::any_held(area);
            self.held_at_quit.lock().unwrap().push(held);
        }
        if let Some((id, uses)) = self.mismatch_on_quit.lock().unwrap().take() {
            self.mismatched.lock().unwrap().insert(id, uses);
        }
        let running = self
            .procs
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.alive)
            .count();
        self.running_at_quit.lock().unwrap().push(running);
        if let Some(id) = self.place_of(pid) {
            for kid in self.kids.iter().filter(|kid| kid.place == id) {
                let Some((write_after, path, line, end_after)) = kid.after_quit.clone() else {
                    continue;
                };
                let alive = Arc::clone(&kid.alive);
                self.endings
                    .lock()
                    .unwrap()
                    .push(std::thread::spawn(move || {
                        std::thread::sleep(write_after);
                        append(&path, line);
                        std::thread::sleep(end_after);
                        alive.store(false, Ordering::SeqCst);
                    }));
            }
        }
        if let Some(id) = self.place_of(pid)
            && !self.stubborn.contains(&id)
        {
            let mut procs = self.procs.lock().unwrap();
            if self.restarted_by_user.contains(&id) {
                procs.insert(
                    id,
                    Proc {
                        pid: 9000 + pid,
                        start: Some(3000),
                        alive: true,
                    },
                );
            } else if let Some(slow) = self.slow {
                self.dies_at
                    .lock()
                    .unwrap()
                    .insert(pid, std::time::Instant::now() + slow);
            } else if let Some(p) = procs.get_mut(&id) {
                p.alive = false;
            }
        }
        Ok(())
    }
    fn identify(&self, pids: &[u32]) -> HashMap<u32, ProcId> {
        self.settle_deaths();
        let micros = |secs: i64| u128::try_from(secs).unwrap() * 1_000_000;
        let mut out = HashMap::new();
        for pid in pids {
            let looks = {
                let mut looks = self.looks.lock().unwrap();
                let n = looks.entry(*pid).or_default();
                *n += 1;
                *n
            };
            if let Some(kid) = self.kids.iter().find(|kid| kid.pid == *pid) {
                if kid.alive.load(Ordering::SeqCst) {
                    out.insert(
                        *pid,
                        ProcId {
                            pid: *pid,
                            start_micros: micros(i64::from(*pid)),
                        },
                    );
                }
                continue;
            }
            let Some(id) = self.place_of(*pid) else {
                continue;
            };
            let procs = self.procs.lock().unwrap();
            let Some(p) = procs.get(&id).filter(|p| p.alive) else {
                continue;
            };
            let start = p.start.unwrap_or_default();
            // Another program on the same pid: a later start.
            let reused =
                self.reused.contains(&id) || (self.reused_after_read.contains(&id) && looks > 1);
            let start = if reused { start + 7 } else { start };
            out.insert(
                *pid,
                ProcId {
                    pid: *pid,
                    start_micros: micros(start),
                },
            );
        }
        out
    }
    fn table(&self, _: &Arc<AppState>) -> Table {
        self.kids
            .iter()
            .filter(|kid| kid.alive.load(Ordering::SeqCst))
            .map(|kid| {
                let exe = format!("/Users/you/bin/{}", kid.name);
                let args: Vec<&str> = std::iter::once(kid.name)
                    .chain(kid.args.iter().copied())
                    .collect();
                crate::features::conversations::procs::fixture(kid.pid, 1, &exe, &args)
            })
            .map(|(pid, mut p)| {
                p.start = Some(i64::from(pid));
                (pid, p)
            })
            .chain(self.linked.lock().unwrap().map(|app| {
                let args = ["claude", "--resume=main"];
                let (pid, mut p) = crate::features::conversations::procs::fixture(
                    OPENED_PID,
                    app,
                    "/Users/you/bin/claude",
                    &args,
                );
                p.start = Some(i64::from(OPENED_PID));
                (pid, p)
            }))
            .collect()
    }
    // The links sent, in order (the move sends one, to the target).
    fn send(&self, pid: u32, link: &str) -> Result<(), String> {
        self.sent.lock().unwrap().push((pid, link.to_owned()));
        if self.opens {
            *self.linked.lock().unwrap() = Some(pid);
        }
        Ok(())
    }
    fn start(&self, _: &Arc<AppState>, place: &KnownPlace) -> Result<(), String> {
        let id = fixtures::id_of(place);
        let mut next = self.next_pid.lock().unwrap();
        *next += 1;
        self.procs.lock().unwrap().insert(
            id.clone(),
            Proc {
                pid: 1000 + *next,
                start: Some(2000),
                alive: true,
            },
        );
        self.starts.lock().unwrap().push(id);
        Ok(())
    }
    fn working(&self, _: &Arc<AppState>, _: u32) -> usize {
        self.working
    }
    fn open_elsewhere(&self, _: &Arc<AppState>, _: &[String], _: &[u32]) -> usize {
        self.elsewhere
    }
    fn settle(&self, for_: Duration) {
        self.settles.lock().unwrap().push(for_);
        let mut writer = self.writer.lock().unwrap();
        if let Some((path, line, left)) = writer.as_mut()
            && *left > 0
        {
            *left -= 1;
            append(path, line);
        }
    }
    fn quit_wait(&self) -> Duration {
        self.wait.unwrap_or(Duration::from_millis(300))
    }
    fn writer_wait(&self) -> Duration {
        self.wait.unwrap_or(Duration::from_millis(300))
    }
    fn tree(&self, _: &Arc<AppState>, place: &KnownPlace, roots: &[u32]) -> Vec<Member> {
        let id = fixtures::id_of(place);
        let procs = self.procs.lock().unwrap();
        let main = roots
            .iter()
            .filter(|pid| procs.get(&id).is_some_and(|p| p.alive && p.pid == **pid))
            .map(|pid| Member {
                pid: *pid,
                start: procs[&id].start,
                name: "Claude".into(),
            });
        let kids = self
            .kids
            .iter()
            .filter(|kid| kid.in_tree && kid.place == id && kid.alive.load(Ordering::SeqCst))
            .map(|kid| Member {
                pid: kid.pid,
                start: Some(i64::from(kid.pid)),
                name: kid.name.into(),
            });
        let opened = (*self.linked.lock().unwrap())
            .filter(|app| roots.contains(app))
            .map(|_| Member {
                pid: OPENED_PID,
                start: Some(i64::from(OPENED_PID)),
                name: "claude".into(),
            });
        main.chain(kids).chain(opened).collect()
    }
    fn tell(&self, progress: &MoveProgressInfo) {
        self.told.lock().unwrap().push(progress.clone());
    }
    fn quits_juggler(
        &self,
        _: &Arc<AppState>,
        place: &KnownPlace,
        _: u32,
        action: &str,
    ) -> Option<String> {
        let id = fixtures::id_of(place);
        (self.juggler_in.as_ref() == Some(&id)).then(|| {
            crate::features::ownership::self_rule::words(
                crate::features::ownership::self_rule::Juggler::Ours,
                &place.name,
                action,
            )
        })
    }
    fn link_mismatch(
        &self,
        _: &Arc<AppState>,
        place_id: &str,
        _: &KnownPlace,
        _: &Path,
    ) -> Option<PathBuf> {
        self.mismatched.lock().unwrap().get(place_id).cloned()
    }
}

impl Scripted {
    /// Waits for every scripted ending to finish.
    fn join(&self) {
        for ending in self.endings.lock().unwrap().drain(..) {
            ending.join().unwrap();
        }
    }

    /// The stages it told, each once, in order.
    fn stages(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for told in self.told.lock().unwrap().iter() {
            if out.last() != Some(&told.stage) {
                out.push(told.stage.clone());
            }
        }
        out
    }
}

struct Lab {
    _tmp: tempfile::TempDir,
    state: Arc<AppState>,
    sides: Sides,
    a_code: PathBuf,
    b_code: PathBuf,
    a_folder: PathBuf,
    b_folder: PathBuf,
}

fn place(state: &Arc<AppState>, root: &Path, name: &str) -> (String, KnownPlace, PathBuf) {
    crate::features::places::remember_test_profile(state, root, name, None)
}

/// The two accounts' Claude Code folders (`acct-a`'s and `acct-b`'s), each
/// signed in and holding the project's folder.
fn code_folders(a_code: &Path, b_code: &Path) {
    for (code, acct) in [(a_code, "acct-a"), (b_code, "acct-b")] {
        std::fs::create_dir_all(code.join("projects").join(SLUG)).unwrap();
        std::fs::write(
            code.join(".claude.json"),
            format!(r#"{{"oauthAccount":{{"accountUuid":"{acct}","organizationUuid":"org"}}}}"#),
        )
        .unwrap();
    }
}

fn lab() -> Lab {
    let (tmp, root, state) = fixtures::lab_state();
    let (a_id, a_place, a_host) = place(&state, &root, "b-a");
    let (b_id, b_place, b_host) = place(&state, &root, "b-b");
    let a_code = root.join("code/a");
    let b_code = root.join("code/b");
    let a_folder = a_host.join("claude-code-sessions/acct-a/org");
    let b_folder = b_host.join("claude-code-sessions/acct-b/org");
    for dir in [&a_folder, &b_folder] {
        std::fs::create_dir_all(dir).unwrap();
    }
    code_folders(&a_code, &b_code);
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();
    for (local, cli, parent) in [
        ("local_main", "main", None),
        ("local_kid", "kid", Some("local_main")),
    ] {
        let parent = parent.map_or_else(String::new, |p: &str| {
            format!(r#","spawnedFrom":{{"sessionId":"{p}"}}"#)
        });
        std::fs::write(
            a_folder.join(format!("{local}.json")),
            format!(
                r#"{{"sessionId":"{local}","cliSessionId":"{cli}","title":"T","cwd":"{}"{parent}}}"#,
                work.display()
            ),
        )
        .unwrap();
        std::fs::write(
            a_code
                .join("projects")
                .join(SLUG)
                .join(format!("{cli}.jsonl")),
            "{\"type\":\"user\"}\n",
        )
        .unwrap();
    }
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(
            juggler_adapters::claude_code::CLAUDE_CODE_ID,
        ))
        .unwrap();
    let side = |code: &Path, host: &Path| MoveSide {
        tool_dir: code.to_path_buf(),
        host_dir: Some(host.to_path_buf()),
        version: Some("5.11492.5".into()),
    };
    let sides = Sides {
        request: MoveRequest {
            conversation: "main".into(),
            source: side(&a_code, &a_host),
            target: side(&b_code, &b_host),
            notes: NotesChoice::KeepTarget,
        },
        tool: "claude-code".into(),
        adapter,
        source_place: Some((a_id, a_place)),
        target_place: Some((b_id, b_place)),
        source_label: "A".into(),
        target_label: "B".into(),
        source_live: false,
        accounts: vec!["acct-a".into(), "acct-b".into()],
        stop: None,
    };
    Lab {
        _tmp: tmp,
        state,
        sides,
        a_code,
        b_code,
        a_folder,
        b_folder,
    }
}

fn running_both(lab: &Lab) -> Scripted {
    let script = Scripted {
        opens: true,
        ..Scripted::default()
    };
    for (n, (id, _)) in [&lab.sides.source_place, &lab.sides.target_place]
        .into_iter()
        .flatten()
        .enumerate()
    {
        script.procs.lock().unwrap().insert(
            id.clone(),
            Proc {
                pid: 500 + u32::try_from(n).unwrap(),
                start: Some(100),
                alive: true,
            },
        );
    }
    script
}

fn clones() -> bool {
    let tmp = tempfile::tempdir().unwrap();
    juggler_platform::transfer::clones_supported(tmp.path()).unwrap_or(false)
}

fn unchanged(lab: &Lab) -> bool {
    lab.a_folder.join("local_main.json").is_file()
        && lab
            .a_code
            .join("projects")
            .join(SLUG)
            .join("main.jsonl")
            .is_file()
        && !lab.b_folder.join("local_main.json").exists()
}

/// The lab, when this disk clones (the move needs it).
fn cloning_lab() -> Option<Lab> {
    clones().then(lab)
}

/// The target profile's id.
fn target_id(lab: &Lab) -> String {
    lab.sides.target_place.as_ref().unwrap().0.clone()
}

/// Both profiles' ids.
fn place_ids(lab: &Lab) -> Vec<String> {
    [&lab.sides.source_place, &lab.sides.target_place]
        .into_iter()
        .flatten()
        .map(|(id, _)| id.clone())
        .collect()
}

/// The same move the other way: back from B to A.
fn moved_back(lab: &Lab) -> Sides {
    Sides {
        request: MoveRequest {
            source: lab.sides.request.target.clone(),
            target: lab.sides.request.source.clone(),
            ..lab.sides.request.clone()
        },
        source_place: lab.sides.target_place.clone(),
        target_place: lab.sides.source_place.clone(),
        adapter: lab.sides.adapter.clone(),
        tool: lab.sides.tool.clone(),
        source_label: "B".into(),
        target_label: "A".into(),
        source_live: false,
        accounts: lab.sides.accounts.clone(),
        stop: None,
    }
}

/// Both profiles running, and a writer no process names that never stops
/// writing the source's history.
fn still_writing(lab: &Lab) -> Scripted {
    let system = running_both(lab);
    *system.writer.lock().unwrap() = Some((
        history(&lab.a_code),
        b"{\"type\":\"cost-state\"}\n",
        usize::MAX,
    ));
    system
}

/// Both profiles running; the target never quits when asked.
fn stubborn_target(lab: &Lab) -> Scripted {
    Scripted {
        stubborn: vec![target_id(lab)],
        ..running_both(lab)
    }
}

/// Both profiles running; the person starts the target again as soon as
/// it quits.
fn restarted_target(lab: &Lab) -> Scripted {
    Scripted {
        restarted_by_user: vec![target_id(lab)],
        ..running_both(lab)
    }
}

/// The lab's move run with `system`, its history checked where it went.
fn moved_with(lab: &Lab, system: &Scripted) -> MoveDoneInfo {
    let done = run(&lab.state, &lab.sides, system).unwrap();
    assert_eq!(done.payload.state, "verified", "{}", done.payload.detail);
    done
}

/// Nothing changed and no move begun.
fn assert_nothing_begun(lab: &Lab) {
    assert!(unchanged(lab));
    assert!(
        !crate::features::places::hold::journals(&crate::features::places::hold::area_at(
            &lab.state.data_dir
        ))
        .exists(),
        "no move begun"
    );
}

/// A move run once with both profiles running, when this disk clones.
fn moved() -> Option<(Lab, Scripted, MoveDoneInfo)> {
    if !clones() {
        return None;
    }
    let lab = lab();
    let system = running_both(&lab);
    let done = run(&lab.state, &lab.sides, &system).unwrap();
    Some((lab, system, done))
}

#[test]
fn a_move_quits_both_profiles_by_their_own_process_moves_and_opens_in_the_target() {
    let Some((lab, system, done)) = moved() else {
        return;
    };
    assert_eq!(done.payload.state, "verified", "{}", done.payload.detail);
    assert_eq!(done.catalog.state, "verified", "{}", done.catalog.detail);
    assert_eq!(done.opened.state, "verified", "{}", done.opened.detail);
    assert_eq!(
        done.model.state, "pending",
        "a reply is the user's to check"
    );
    assert_eq!(
        *system.quits.lock().unwrap(),
        [500, 501],
        "exactly the two profiles"
    );
    assert_eq!(system.starts.lock().unwrap().len(), 2, "both started again");
    let target_id = &lab.sides.target_place.as_ref().unwrap().0;
    let target_pid = system.procs.lock().unwrap()[target_id].pid;
    assert_eq!(
        *system.sent.lock().unwrap(),
        [(target_pid, "claude://resume?session=main".to_owned())],
        "the link went to the target profile's own new process"
    );
    assert!(lab.b_folder.join("local_main.json").is_file());
    assert!(lab.b_folder.join("local_kid.json").is_file());
    assert!(
        lab.b_code
            .join("projects")
            .join(SLUG)
            .join("kid.jsonl")
            .is_file()
    );
    assert!(
        !crate::features::places::any_move(&lab.state),
        "the hold let go"
    );
    // Undo: back as it was, opened in the source.
    let said = undo_with(&lab.state, &done.move_id, &system).unwrap();
    assert!(said.words.starts_with("Undone"), "{said:?}");
    // R4 (the 0.5.0 re-walk): no raw path in its words; the folder is for
    // Copy details.
    assert!(!said.words.contains('/'), "{said:?}");
    assert!(
        said.kept.ends_with(&done.move_id) || said.kept.contains(&done.move_id),
        "{said:?}"
    );
    assert!(unchanged(&lab));
    let moves = moves(&lab.state);
    assert_eq!(moves[0].state, "undone");
    // Its own folder stays a kept dependency (cleanup must keep it).
    assert!(
        kept_paths(&lab.state)
            .iter()
            .any(|p| p.ends_with(&done.move_id))
    );
    assert!(!moves[0].kept.is_empty());
}

#[test]
fn a_busy_profile_refuses_before_anything_is_quit_or_written() {
    let lab = lab();
    let system = Scripted {
        working: 2,
        ..running_both(&lab)
    };
    let error = run(&lab.state, &lab.sides, &system)
        .unwrap_err()
        .to_string();
    assert!(error.contains("2 sessions are working"), "{error}");
    assert!(system.quits.lock().unwrap().is_empty());
    assert_nothing_begun(&lab);
}

#[test]
fn a_reused_pid_is_never_quit() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let source = lab.sides.source_place.as_ref().unwrap().0.clone();
    let system = Scripted {
        reused: vec![source],
        ..running_both(&lab)
    };
    let error = run(&lab.state, &lab.sides, &system)
        .unwrap_err()
        .to_string();
    assert!(error.contains("couldn't be sure which process"), "{error}");
    assert!(
        system.quits.lock().unwrap().is_empty(),
        "nothing quit on a guess"
    );
    assert!(unchanged(&lab));
}

#[test]
fn a_profile_that_wont_quit_stops_the_move_and_the_other_starts_again() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = stubborn_target(&lab);
    let error = run(&lab.state, &lab.sides, &system)
        .unwrap_err()
        .to_string();
    assert!(error.contains("didn't quit"), "{error}");
    assert_eq!(
        system.starts.lock().unwrap().len(),
        1,
        "the source started again"
    );
    assert!(unchanged(&lab));
}

/// R4 (the 0.5.0 re-walk): an Undo a profile wouldn't quit for said
/// "so nothing was moved", the Move's words; it says what it didn't do.
#[test]
fn an_undo_a_profile_wont_quit_for_says_nothing_was_undone() {
    let Some((lab, system, done)) = moved() else {
        return;
    };
    let target = lab.sides.target_place.as_ref().unwrap().0.clone();
    let stubborn = Scripted {
        stubborn: vec![target],
        ..running_both(&lab)
    };
    drop(system);
    let error = undo_with(&lab.state, &done.move_id, &stubborn)
        .unwrap_err()
        .to_string();
    assert!(error.contains("didn't quit"), "{error}");
    assert!(error.contains("nothing was undone"), "{error}");
    assert!(!error.contains("moved"), "{error}");
}

#[test]
fn a_held_profile_is_left_alone_by_open_and_restart() {
    let lab = lab();
    let id = lab.sides.target_place.as_ref().unwrap().0.clone();
    let hold =
        crate::features::places::MoveHold::take(&lab.state, std::slice::from_ref(&id), "m-1")
            .unwrap();
    assert!(crate::features::places::held_for_move(&lab.state, &id));
    assert!(
        crate::features::places::MoveHold::take(&lab.state, std::slice::from_ref(&id), "m-2")
            .is_err()
    );
    drop(hold);
    assert!(!crate::features::places::held_for_move(&lab.state, &id));
}

/// AJ-E2320 R1: both profiles are held in the area every AI Juggler
/// shares before the first quit; the committed move lets go.
#[test]
fn a_move_holds_both_profiles_for_every_ai_juggler_before_the_first_quit() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let area = crate::features::places::hold::area_at(&lab.state.data_dir);
    let system = Scripted {
        area: Some(area.clone()),
        ..running_both(&lab)
    };
    run(&lab.state, &lab.sides, &system).unwrap();
    let held = system.held_at_quit.lock().unwrap().clone();
    assert!(!held.is_empty() && held.iter().all(|h| *h), "{held:?}");
    assert!(
        !crate::features::places::hold::any_held(&area),
        "committed: let go"
    );
    assert_eq!(
        crate::features::places::hold::all_moves(&lab.state.data_dir).len(),
        1,
        "its journal in the shared area"
    );
}

#[test]
fn a_terminal_session_on_either_account_refuses_the_move() {
    let lab = lab();
    let system = Scripted {
        elsewhere: 1,
        ..running_both(&lab)
    };
    let error = run(&lab.state, &lab.sides, &system)
        .unwrap_err()
        .to_string();
    assert!(error.contains("open outside the app"), "{error}");
    assert!(system.quits.lock().unwrap().is_empty());
    assert!(unchanged(&lab));
}

#[test]
fn a_profile_started_by_hand_mid_move_stops_it_before_anything_changes() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = restarted_target(&lab);
    let error = run(&lab.state, &lab.sides, &system)
        .unwrap_err()
        .to_string();
    assert!(error.contains("was started while the move ran"), "{error}");
    assert!(unchanged(&lab));
}

#[test]
fn undo_waits_for_a_later_move_of_the_same_files_and_for_quiet() {
    let Some((lab, system, first)) = moved() else {
        return;
    };
    // Moved back: the later move owns the files now.
    let back = moved_back(&lab);
    let second = run(&lab.state, &back, &system).unwrap();
    let error = undo_with(&lab.state, &first.move_id, &system)
        .unwrap_err()
        .to_string();
    assert!(error.contains("moved again since"), "{error}");
    // Busy work refuses Undo as it refuses a move.
    let busy = Scripted {
        working: 1,
        ..running_both(&lab)
    };
    let error = undo_with(&lab.state, &second.move_id, &busy)
        .unwrap_err()
        .to_string();
    assert!(error.contains("working"), "{error}");
    // Quiet: the later one undoes, then the first.
    undo_with(&lab.state, &second.move_id, &system).unwrap();
    undo_with(&lab.state, &first.move_id, &system).unwrap();
    assert!(unchanged(&lab));
}

#[test]
fn undo_refuses_a_profile_ai_juggler_no_longer_knows() {
    let Some((lab, system, done)) = moved() else {
        return;
    };
    let target = lab.sides.target_place.as_ref().unwrap().0.clone();
    crate::features::places::with_book(&lab.state, |book| {
        book.places.remove(&target);
    })
    .unwrap();
    let error = undo_with(&lab.state, &done.move_id, &system)
        .unwrap_err()
        .to_string();
    assert!(error.contains("isn't one AI Juggler knows"), "{error}");
    assert!(
        lab.b_folder.join("local_main.json").is_file(),
        "nothing undone"
    );
}

/// D-127: busy work is the user's to clear: Move waits, and says how.
#[test]
fn a_busy_profile_waits_in_the_offer_and_says_how() {
    let lab = lab();
    let system = Scripted {
        working: 2,
        ..running_both(&lab)
    };
    let offer = offer_of(&lab.state, &lab.sides, &system);
    assert!(offer.applies);
    let waits: Vec<&MoveBlockInfo> = offer
        .blocks
        .iter()
        .filter(|b| b.kind == MoveBlockKind::UserCanClear)
        .collect();
    assert!(
        waits
            .iter()
            .any(|b| b.words.contains("2 sessions are working in")
                && b.words.ends_with("Move becomes available once they finish")),
        "{:?}",
        offer.blocks
    );
    if clones() {
        assert!(
            offer
                .blocks
                .iter()
                .all(|b| b.kind == MoveBlockKind::UserCanClear),
            "{:?}",
            offer.blocks
        );
    }
}

/// AJ-E2315 (the self rule, the review's A1.5): AI Juggler started from
/// inside the source profile (its Code tab) is never quit by its own move:
/// the offer waits, saying why and how it clears, and a move run anyway
/// refuses before anything quits.
#[test]
fn a_move_that_would_quit_ai_juggler_refuses_before_anything_quits() {
    let lab = lab();
    let system = Scripted {
        juggler_in: Some(source_id(&lab)),
        ..running_both(&lab)
    };
    let offer = offer_of(&lab.state, &lab.sides, &system);
    assert!(
        offer
            .blocks
            .iter()
            .any(|b| b.kind == MoveBlockKind::UserCanClear
                && b.words
                    .contains("AI Juggler itself was started from inside")
                && b.words.ends_with(
                    "Move becomes available once you start AI Juggler from the Dock or Finder"
                )),
        "{:?}",
        offer.blocks
    );
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    assert!(
        error
            .wire()
            .user_message
            .contains("AI Juggler itself was started from inside"),
        "{error}"
    );
    assert!(system.quits.lock().unwrap().is_empty(), "nothing quit");
    assert!(system.starts.lock().unwrap().is_empty(), "nothing started");
    assert!(unchanged(&lab));
}

/// D-128: a new major of the app whose records fit is offered (the 0.4.6
/// rc2 walk: the Claude app had updated to 5.11492.5 and Move showed
/// greyed with "proven for version 4.x").
#[test]
fn a_new_major_whose_records_fit_is_offered() {
    let mut lab = lab();
    lab.sides.request.source.version = Some("5.11492.5".into());
    lab.sides.request.target.version = Some("5.11492.5".into());
    let offer = offer_of(&lab.state, &lab.sides, &Scripted::default());
    assert!(offer.applies);
    if clones() {
        assert!(offer.blocks.is_empty(), "{:?}", offer.blocks);
    }
    assert!(
        !lab.state.data_dir.join(SHAPE_FILE).exists(),
        "nothing to note"
    );
}

/// D-127/D-128: records of a shape the move doesn't know are a block the
/// user can't clear (the switch screen hides Move), and what didn't fit is
/// kept with the app's version for the gate watch.
#[test]
fn records_the_move_doesnt_know_hide_it_and_are_kept_for_the_gate_watch() {
    let mut lab = lab();
    lab.sides.request.source.version = Some("6.0.1".into());
    lab.sides.request.target.version = Some("6.0.1".into());
    std::fs::write(lab.b_folder.join("sessions.db"), b"").unwrap();
    let offer = offer_of(&lab.state, &lab.sides, &Scripted::default());
    assert!(
        offer.blocks.iter().any(|b| b.kind == MoveBlockKind::Setup),
        "{:?}",
        offer.blocks
    );
    let kept: serde_json::Value =
        serde_json::from_slice(&std::fs::read(lab.state.data_dir.join(SHAPE_FILE)).unwrap())
            .unwrap();
    let seen = kept["seen"].as_array().unwrap();
    assert!(
        seen.iter().any(|e| e["tool"] == "claude-code"
            && e["version"] == "6.0.1"
            && e["feature"] == "move"
            && e["what"].as_str().unwrap().contains("sessions.db")),
        "{kept}"
    );
    // Asked again: one entry, not two.
    let _ = offer_of(&lab.state, &lab.sides, &Scripted::default());
    let kept: serde_json::Value =
        serde_json::from_slice(&std::fs::read(lab.state.data_dir.join(SHAPE_FILE)).unwrap())
            .unwrap();
    assert_eq!(kept["seen"].as_array().unwrap().len(), seen.len());
}

/// The moved conversation's history on the code folder `code`.
fn history(code: &Path) -> PathBuf {
    code.join("projects").join(SLUG).join("main.jsonl")
}

fn source_id(lab: &Lab) -> String {
    lab.sides.source_place.as_ref().unwrap().0.clone()
}

/// A move's own folder of backups.
fn backups(lab: &Lab) -> Vec<PathBuf> {
    let moves = crate::features::places::hold::journals(&crate::features::places::hold::area_at(
        &lab.state.data_dir,
    ));
    let mut out = Vec::new();
    for entry in std::fs::read_dir(moves).unwrap().flatten() {
        for backup in std::fs::read_dir(entry.path().join("backups"))
            .unwrap()
            .flatten()
        {
            out.push(backup.path());
        }
    }
    out
}

/// 0.4.9, the live trace (a 406 MB move refused: "the backup of … doesn't
/// match it"): the profile's main process ends, the Code tab's Claude
/// Code writes its last line after it, then ends. The move waits for it,
/// so that line moves with the history and nothing is left behind.
#[test]
fn a_move_waits_for_what_outlives_the_profiles_main_process() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let line: &'static [u8] = b"{\"type\":\"last-prompt\"}\n";
    let system = Scripted {
        kids: vec![Kid {
            after_quit: Some((
                Duration::from_millis(300),
                history(&lab.a_code),
                line,
                Duration::from_millis(50),
            )),
            ..kid(&lab, 700, "claude", &["--resume=main"], true)
        }],
        wait: Some(Duration::from_secs(5)),
        ..running_both(&lab)
    };
    let done = run(&lab.state, &lab.sides, &system);
    system.join();
    let done = done.unwrap();
    assert_eq!(done.payload.state, "verified", "{}", done.payload.detail);
    let moved = std::fs::read(history(&lab.b_code)).unwrap();
    assert!(moved.ends_with(line), "the last line moved with it");
    assert!(!history(&lab.a_code).exists(), "nothing left behind");
    assert!(
        system
            .told
            .lock()
            .unwrap()
            .iter()
            .any(|t| t.stage == "closing" && t.words.ends_with("to finish closing…")),
        "said what it waits for"
    );
}

/// What outlives the profile too long refuses calmly, naming it: the
/// profile starts again, the journal reads planned → undone, and no
/// backup is kept.
#[test]
fn a_profile_whose_tools_outlive_it_too_long_is_refused_calmly() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = Scripted {
        kids: vec![kid(&lab, 700, "claude", &["--resume=main"], true)],
        ..running_both(&lab)
    };
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    assert!(
        matches!(&error, AppError::Move(stop) if stop.unchanged),
        "calm, both accounts as they were: {error}"
    );
    let words = error.wire().user_message;
    assert!(words.contains("didn't finish closing within"), "{words}");
    assert!(
        words.contains("claude still runs, so nothing was moved"),
        "{words}"
    );
    // AJ-E2318: both quit at once, so both start again.
    assert_eq!(system.starts.lock().unwrap().len(), 2, "both again");
    assert_eq!(moves(&lab.state)[0].state, "undone");
    assert!(backups(&lab).is_empty());
    assert!(unchanged(&lab));
}

/// A file still changing after the apps closed refuses in plain words:
/// who was writing, never a path (that is Copy details'), calm, and the
/// undone move keeps no backup.
#[test]
fn files_still_changing_after_the_apps_closed_refuse_in_plain_words() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = still_writing(&lab);
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    let source = place_name(&lab.state, &lab.sides.source_place.as_ref().unwrap().1);
    let wire = error.wire();
    assert_eq!(
        wire.user_message,
        format!(
            "{source} was still writing this conversation as it closed, so nothing was moved. Try again in a moment."
        )
    );
    assert!(!wire.user_message.contains('/'), "no path in the words");
    assert!(
        wire.detail
            .as_deref()
            .is_some_and(|d| d.contains("main.jsonl")),
        "{wire:?}"
    );
    assert_eq!((wire.code.as_str(), wire.retryable), ("move_refused", true));
    assert_eq!(system.starts.lock().unwrap().len(), 2, "both started again");
    assert_eq!(moves(&lab.state)[0].state, "undone");
    assert!(backups(&lab).is_empty(), "an undone move keeps no backup");
    assert!(unchanged(&lab));
}

/// A file that settles after one more write moves with that write.
#[test]
fn a_file_that_settles_after_one_more_write_moves_with_it() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let line: &'static [u8] = b"{\"type\":\"cost-state\"}\n";
    let system = running_both(&lab);
    *system.writer.lock().unwrap() = Some((history(&lab.a_code), line, 1));
    moved_with(&lab, &system);
    assert!(std::fs::read(history(&lab.b_code)).unwrap().ends_with(line));
}

/// 0.4.9: a long move never looks frozen: it says each stage, and the
/// backup's bytes rise to its total.
#[test]
fn a_long_backup_says_how_far_it_is() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let line = b"{\"type\":\"assistant\",\"pad\":\"................................\"}\n";
    let big = line.repeat((6 << 20) / line.len() + 1);
    std::fs::write(history(&lab.a_code), &big).unwrap();
    let system = running_both(&lab);
    run(&lab.state, &lab.sides, &system).unwrap();
    assert_eq!(
        system.stages(),
        [
            "closing",
            "still",
            "backing_up",
            "moving",
            "starting",
            "opening",
            "checking"
        ]
    );
    let told = system.told.lock().unwrap();
    let backing: Vec<&MoveProgressInfo> = told.iter().filter(|t| t.stage == "backing_up").collect();
    assert!(backing.len() >= 3, "{}", backing.len());
    assert!(backing.windows(2).all(|w| w[0].done <= w[1].done));
    let last = backing.last().unwrap();
    assert!(last.total >= 2 * big.len() as u64);
    assert_eq!(last.done, last.total);
    assert!(
        last.words.starts_with("Backing up the conversation ("),
        "{}",
        last.words
    );
}

/// 0.4.9: Undo lets the move's backups go (nothing is left to restore).
#[test]
fn an_undone_move_lets_its_backups_go() {
    let Some((lab, system, done)) = moved() else {
        return;
    };
    assert!(!backups(&lab).is_empty(), "a done move keeps them for Undo");
    undo_with(&lab.state, &done.move_id, &system).unwrap();
    assert!(backups(&lab).is_empty());
    assert!(unchanged(&lab));
}

#[path = "moving_fault_tests.rs"]
mod faults;
#[path = "moving_open_tests.rs"]
mod open;
#[path = "moving_pick_tests.rs"]
mod picking;
#[path = "moving_recover_tests.rs"]
mod recovering;
#[path = "moving_shape_tests.rs"]
mod shapes;
#[path = "moving_stop_tests.rs"]
mod stops;
#[path = "moving_wait_tests.rs"]
mod wait;
