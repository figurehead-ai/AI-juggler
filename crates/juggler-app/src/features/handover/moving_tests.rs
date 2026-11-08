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
            && let Some(p) = self.procs.lock().unwrap().get_mut(&id)
        {
            p.alive = false;
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
    fn send(&self, pid: u32, link: &str) -> Result<(), String> {
        self.sent.lock().unwrap().push((pid, link.to_owned()));
        Ok(())
    }
    fn working(&self, _: &Arc<AppState>, _: u32) -> usize {
        self.working
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
    let bundle = root.join("Claude.app");
    std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    let profile = root.join(format!("Profiles/{name}"));
    std::fs::create_dir_all(&profile).unwrap();
    let id = book::place_id("claude-app", Some(&profile));
    let known = KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude".into(),
        instance: Some(name.into()),
        bundle: bundle.clone(),
        exe: bundle.join("Contents/MacOS/Claude"),
        given: Some(profile.clone()),
        instance_dir: Some(profile.clone()),
        account_id: None,
        conversations: Vec::new(),
        seen_ms: 1,
        keep_alive: true,
        paused: None,
        found: false,
        users: true,
        declined: false,
    };
    crate::features::places::with_book(state, |book| {
        book.places.insert(id.clone(), known.clone());
    })
    .unwrap();
    (id, known, profile)
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

#[test]
fn a_move_quits_both_profiles_by_their_own_process_moves_and_opens_in_the_target() {
    if !clones() {
        return;
    }
    let lab = lab();
    let system = running_both(&lab);
    let done = run(&lab.state, &lab.sides, &system).unwrap();
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
