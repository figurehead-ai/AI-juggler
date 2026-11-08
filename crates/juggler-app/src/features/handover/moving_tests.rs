//! ADR-0031: the process side of a move over a real app state, real
//! fixture folders and a scripted system (no app is quit or started).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
    fn settle(&self, _: Duration) {}
    fn quit_wait(&self) -> Duration {
        Duration::from_millis(300)
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
    assert!(said.starts_with("Undone"), "{said}");
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
    assert!(error.contains("2 sessions working"), "{error}");
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
