//! ADR-0031: the process side of a move over a real app state, real
//! fixture folders and a scripted system (no app is quit or started).

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::*;
use crate::features::places::book;

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
    /// The place it belongs to.
    place: String,
    alive: Arc<AtomicBool>,
    /// Once its main process quits: after the first wait it appends the
    /// line to the file (its last write), after the second it ends.
    /// `None`: it never ends.
    after_quit: Option<(Duration, PathBuf, &'static [u8], Duration)>,
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
}

impl Scripted {
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
        self.procs
            .lock()
            .unwrap()
            .get(id)
            .filter(|p| p.alive)
            .map(|p| (p.pid, p.start))
    }
    fn quit(&self, pid: u32) -> Result<(), String> {
        self.quits.lock().unwrap().push(pid);
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
            } else if let Some(p) = procs.get_mut(&id) {
                p.alive = false;
            }
        }
        Ok(())
    }
    fn alive(&self, pid: u32, start: Option<i64>) -> bool {
        if let Some(kid) = self.kids.iter().find(|kid| kid.pid == pid) {
            return kid.alive.load(Ordering::SeqCst) && start == Some(i64::from(pid));
        }
        let Some(id) = self.place_of(pid) else {
            return false;
        };
        if self.reused.contains(&id) {
            return false;
        }
        self.procs
            .lock()
            .unwrap()
            .get(&id)
            .is_some_and(|p| p.alive && p.start == start)
    }
    // The links sent, in order (the move sends one, to the target).
    fn send(&self, pid: u32, link: &str) -> Result<(), String> {
        self.sent.lock().unwrap().push((pid, link.to_owned()));
        Ok(())
    }
    fn start(&self, _: &Arc<AppState>, place: &KnownPlace) -> Result<(), String> {
        let id = book::place_id(&place.app_id, place.instance_dir.as_deref());
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
    fn settle(&self, _: Duration) {
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
    fn tree(&self, _: &Arc<AppState>, place: &KnownPlace, roots: &[u32]) -> Vec<Member> {
        let id = book::place_id(&place.app_id, place.instance_dir.as_deref());
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
            .filter(|kid| kid.place == id && kid.alive.load(Ordering::SeqCst))
            .map(|kid| Member {
                pid: kid.pid,
                start: Some(i64::from(kid.pid)),
                name: kid.name.into(),
            });
        main.chain(kids).collect()
    }
    fn tell(&self, progress: &MoveProgressInfo) {
        self.told.lock().unwrap().push(progress.clone());
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

fn lab() -> Lab {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let state = Arc::new(AppState::init(root.join("data")).unwrap());
    let (a_id, a_place, a_host) = place(&state, &root, "b-a");
    let (b_id, b_place, b_host) = place(&state, &root, "b-b");
    let a_code = root.join("code/a");
    let b_code = root.join("code/b");
    let a_folder = a_host.join("claude-code-sessions/acct-a/org");
    let b_folder = b_host.join("claude-code-sessions/acct-b/org");
    for dir in [&a_folder, &b_folder] {
        std::fs::create_dir_all(dir).unwrap();
    }
    for (code, acct) in [(&a_code, "acct-a"), (&b_code, "acct-b")] {
        std::fs::create_dir_all(code.join("projects").join(SLUG)).unwrap();
        std::fs::write(
            code.join(".claude.json"),
            format!(r#"{{"oauthAccount":{{"accountUuid":"{acct}","organizationUuid":"org"}}}}"#),
        )
        .unwrap();
    }
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
        version: Some("4.1.0".into()),
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
    let script = Scripted::default();
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
    assert!(said.kept.ends_with(&done.move_id) || said.kept.contains(&done.move_id), "{said:?}");
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
    assert!(unchanged(&lab));
    assert!(!lab.state.data_dir.join("moves").exists(), "no move begun");
}

#[test]
fn a_reused_pid_is_never_quit() {
    if !clones() {
        return;
    }
    let lab = lab();
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
    if !clones() {
        return;
    }
    let lab = lab();
    let target = lab.sides.target_place.as_ref().unwrap().0.clone();
    let mut system = running_both(&lab);
    system.stubborn = vec![target];
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
        crate::features::places::MoveHold::take(&lab.state, std::slice::from_ref(&id)).unwrap();
    assert!(crate::features::places::held_for_move(&lab.state, &id));
    assert!(
        crate::features::places::MoveHold::take(&lab.state, std::slice::from_ref(&id)).is_none()
    );
    drop(hold);
    assert!(!crate::features::places::held_for_move(&lab.state, &id));
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
    if !clones() {
        return;
    }
    let lab = lab();
    let target = lab.sides.target_place.as_ref().unwrap().0.clone();
    let system = Scripted {
        restarted_by_user: vec![target],
        ..running_both(&lab)
    };
    let error = run(&lab.state, &lab.sides, &system)
        .unwrap_err()
        .to_string();
    assert!(error.contains("was started while the move ran"), "{error}");
    assert!(unchanged(&lab));
}

#[test]
fn undo_waits_for_a_later_move_of_the_same_files_and_for_quiet() {
    if !clones() {
        return;
    }
    let lab = lab();
    let system = running_both(&lab);
    let first = run(&lab.state, &lab.sides, &system).unwrap();
    // Moved back: the later move owns the files now.
    let back = Sides {
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
    };
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
    let moves = lab.state.data_dir.join("moves");
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
    if !clones() {
        return;
    }
    let lab = lab();
    let line: &'static [u8] = b"{\"type\":\"last-prompt\"}\n";
    let system = Scripted {
        kids: vec![Kid {
            pid: 700,
            name: "claude",
            place: source_id(&lab),
            alive: Arc::new(AtomicBool::new(true)),
            after_quit: Some((
                Duration::from_millis(300),
                history(&lab.a_code),
                line,
                Duration::from_millis(50),
            )),
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
    if !clones() {
        return;
    }
    let lab = lab();
    let system = Scripted {
        kids: vec![Kid {
            pid: 700,
            name: "claude",
            place: source_id(&lab),
            alive: Arc::new(AtomicBool::new(true)),
            after_quit: None,
        }],
        ..running_both(&lab)
    };
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    assert!(
        matches!(
            error,
            AppError::Move {
                unchanged: true,
                ..
            }
        ),
        "calm, both accounts as they were: {error}"
    );
    let words = error.wire().user_message;
    assert!(words.contains("didn't finish closing within"), "{words}");
    assert!(
        words.contains("claude still runs, so nothing was moved"),
        "{words}"
    );
    assert_eq!(system.starts.lock().unwrap().len(), 1, "the source again");
    assert_eq!(moves(&lab.state)[0].state, "undone");
    assert!(backups(&lab).is_empty());
    assert!(unchanged(&lab));
}

/// A file still changing after the apps closed refuses in plain words:
/// who was writing, never a path (that is Copy details'), calm, and the
/// undone move keeps no backup.
#[test]
fn files_still_changing_after_the_apps_closed_refuse_in_plain_words() {
    if !clones() {
        return;
    }
    let lab = lab();
    let system = running_both(&lab);
    *system.writer.lock().unwrap() = Some((
        history(&lab.a_code),
        b"{\"type\":\"cost-state\"}\n",
        usize::MAX,
    ));
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
    if !clones() {
        return;
    }
    let lab = lab();
    let line: &'static [u8] = b"{\"type\":\"cost-state\"}\n";
    let system = running_both(&lab);
    *system.writer.lock().unwrap() = Some((history(&lab.a_code), line, 1));
    let done = run(&lab.state, &lab.sides, &system).unwrap();
    assert_eq!(done.payload.state, "verified", "{}", done.payload.detail);
    assert!(std::fs::read(history(&lab.b_code)).unwrap().ends_with(line));
}

/// 0.4.9: a long move never looks frozen: it says each stage, and the
/// backup's bytes rise to its total.
#[test]
fn a_long_backup_says_how_far_it_is() {
    if !clones() {
        return;
    }
    let lab = lab();
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
