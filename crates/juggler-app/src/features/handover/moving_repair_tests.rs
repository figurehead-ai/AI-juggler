//! AJ-E2328 req 7: the ownership check and Repair over a real app state,
//! fixture profiles (temp folders, never a real profile) and a scripted
//! system (no app is quit or started).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use super::*;
use crate::features::conversations::procs::Table;
use crate::features::handover::moving::fixtures;
use crate::features::ownership::ProcId;
use crate::features::places::tree::Member;

/// The scripted system: each profile's main process, quit when asked.
#[derive(Default)]
struct Fake {
    /// place id → (pid, alive).
    procs: Mutex<HashMap<String, (u32, bool)>>,
    quits: Mutex<Vec<u32>>,
    starts: Mutex<Vec<String>>,
    working: usize,
}

impl Fake {
    fn place_of(&self, pid: u32) -> Option<String> {
        self.procs
            .lock()
            .unwrap()
            .iter()
            .find(|(_, (p, _))| *p == pid)
            .map(|(id, _)| id.clone())
    }
}

impl MoveSystem for Fake {
    fn running(&self, _: &Arc<AppState>, id: &str) -> Option<(u32, Option<i64>)> {
        self.procs
            .lock()
            .unwrap()
            .get(id)
            .filter(|(_, alive)| *alive)
            .map(|(pid, _)| (*pid, Some(100)))
    }
    fn quit(&self, pid: u32) -> Result<(), String> {
        self.quits.lock().unwrap().push(pid);
        if let Some(id) = self.place_of(pid)
            && let Some(p) = self.procs.lock().unwrap().get_mut(&id)
        {
            p.1 = false;
        }
        Ok(())
    }
    fn identify(&self, pids: &[u32]) -> HashMap<u32, ProcId> {
        let procs = self.procs.lock().unwrap();
        pids.iter()
            .filter(|pid| procs.values().any(|(p, alive)| p == *pid && *alive))
            .map(|pid| {
                (
                    *pid,
                    ProcId {
                        pid: *pid,
                        start_micros: 100_000_000,
                    },
                )
            })
            .collect()
    }
    fn table(&self, _: &Arc<AppState>) -> Table {
        Table::default()
    }
    fn working(&self, _: &Arc<AppState>, _: u32) -> usize {
        self.working
    }
    fn start(&self, _: &Arc<AppState>, place: &KnownPlace) -> Result<(), String> {
        let id = fixtures::id_of(place);
        // Each start its own process (an Undo quits both by theirs).
        let pid = 900 + u32::try_from(self.starts.lock().unwrap().len()).unwrap();
        self.procs.lock().unwrap().insert(id.clone(), (pid, true));
        self.starts.lock().unwrap().push(id);
        Ok(())
    }
    fixtures::quiet_parts!(Duration::from_millis(300));
    fn tree(&self, _: &Arc<AppState>, place: &KnownPlace, roots: &[u32]) -> Vec<Member> {
        let (id, procs) = (fixtures::id_of(place), self.procs.lock().unwrap());
        roots
            .iter()
            .filter(|pid| procs.get(&id).is_some_and(|(p, alive)| *alive && p == *pid))
            .map(|pid| Member {
                pid: *pid,
                start: Some(100),
                name: "Claude".into(),
            })
            .collect()
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

/// Two fixture profiles A and B of the Claude app 5.11492.5; the
/// conversation "main" is in B, its worktree `wt-7` still owned by A (as a
/// 0.4.x move left it), and A's cleanup has seen it.
struct Lab {
    _tmp: tempfile::TempDir,
    state: Arc<AppState>,
    a: (String, PathBuf),
    b: (String, PathBuf),
    wt: PathBuf,
    root: PathBuf,
}

fn wt_file(host: &Path, leases: &[(&str, &Path, &str)], seen: &[&Path]) {
    let mut worktrees = serde_json::Map::new();
    for (name, folder, by) in leases {
        worktrees.insert(
            (*name).to_owned(),
            serde_json::json!({
                "name": name, "path": folder.display().to_string(), "leasedBy": by,
                "branch": format!("claude/{name}"), "sourceBranch": "main", "createdAt": 1,
            }),
        );
    }
    let sightings: serde_json::Map<String, serde_json::Value> = seen
        .iter()
        .map(|p| (p.display().to_string(), serde_json::json!(7)))
        .collect();
    std::fs::write(
        host.join("git-worktrees.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": 2, "originUrls": {}, "originPins": {},
            "worktrees": worktrees,
            "untrackedDirGc": {"sightings": sightings, "roots": {}, "cwds": {}},
            "pendingTombstones": [],
        }))
        .unwrap(),
    )
    .unwrap();
}

fn worktree(root: &Path, name: &str) -> PathBuf {
    let git = root.join("work/repo/.git/worktrees").join(name);
    std::fs::create_dir_all(&git).unwrap();
    std::fs::write(git.join("HEAD"), format!("ref: refs/heads/claude/{name}\n")).unwrap();
    let folder = root.join("work/repo/.claude/worktrees").join(name);
    std::fs::create_dir_all(folder.join("src")).unwrap();
    std::fs::write(folder.join(".git"), format!("gitdir: {}\n", git.display())).unwrap();
    std::fs::write(folder.join("src/work.rs"), b"fn main() {}\n").unwrap();
    folder
}

fn record(host: &Path, wt: &Path) {
    let org = host.join("claude-code-sessions/acct-b/org");
    std::fs::create_dir_all(&org).unwrap();
    std::fs::write(
        org.join("local_main.json"),
        serde_json::to_vec(&serde_json::json!({
            "sessionId": "local_main", "cliSessionId": "main", "title": "Fix the parser",
            "cwd": wt.display().to_string(),
            "worktreeName": wt.file_name().unwrap().to_string_lossy(),
            "worktreePath": wt.display().to_string(),
            "branch": format!("claude/{}", wt.file_name().unwrap().to_string_lossy()),
        }))
        .unwrap(),
    )
    .unwrap();
}

fn lab() -> Lab {
    let (tmp, root, state) = fixtures::lab_state();
    let (a_id, _, a_host) =
        crate::features::places::remember_test_profile(&state, &root, "lab-a", None);
    let (b_id, _, b_host) =
        crate::features::places::remember_test_profile(&state, &root, "lab-b", None);
    let plist = root.join("Claude.app/Contents/Info.plist");
    std::fs::write(
        &plist,
        "<?xml version=\"1.0\"?><plist><dict><key>CFBundleShortVersionString</key><string>5.11492.5</string></dict></plist>",
    )
    .unwrap();
    let wt = worktree(&root, "wt-7");
    record(&b_host, &wt);
    wt_file(&a_host, &[("wt-7", &wt, "local_main")], &[&wt]);
    wt_file(&b_host, &[], &[]);
    Lab {
        _tmp: tmp,
        state,
        a: (a_id, a_host),
        b: (b_id, b_host),
        wt,
        root,
    }
}

fn both_running(lab: &Lab) -> Fake {
    let fake = Fake::default();
    let mut procs = fake.procs.lock().unwrap();
    procs.insert(lab.a.0.clone(), (500, true));
    procs.insert(lab.b.0.clone(), (501, true));
    drop(procs);
    fake
}

fn clones() -> bool {
    let tmp = tempfile::tempdir().unwrap();
    juggler_platform::transfer::clones_supported(tmp.path()).unwrap_or(false)
}

fn read(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn the_check_names_a_split_conversation_the_profiles_repair_quits_and_the_app_version() {
    let lab = lab();
    let fake = both_running(&lab);
    let info = findings(&lab.state, &fake);
    assert_eq!(info.unreadable, None);
    assert_eq!(info.apps.len(), 1, "{:?}", info.apps);
    assert_eq!(info.apps[0].version.as_deref(), Some("5.11492.5"));
    assert!(info.apps[0].checked);
    assert_eq!(info.findings.len(), 1, "{:?}", info.findings);
    let one = &info.findings[0];
    assert_eq!(one.title, "Fix the parser");
    assert_eq!(one.worktree, "wt-7");
    assert!(
        one.words.contains("wt-7") && one.words.contains("still belongs to"),
        "{}",
        one.words
    );
    assert_eq!(one.quits.len(), 2, "both profiles run: {:?}", one.quits);
    // 0.5.1 F90: both Apps it involves by their place id, for the sheet's
    // own live read of which run.
    let mut apps: Vec<&str> = one.apps.iter().map(|a| a.place_id.as_str()).collect();
    apps.sort_unstable();
    let mut ids = vec![lab.a.0.as_str(), lab.b.0.as_str()];
    ids.sort_unstable();
    assert_eq!(apps, ids, "{:?}", one.apps);
    assert!(one.apps.iter().all(|a| !a.name.is_empty()));
    // No internal terms on screen.
    for words in std::iter::once(&one.words).chain(&one.notes) {
        for term in ["lease", "sighting", "untracked"] {
            assert!(!words.to_lowercase().contains(term), "{words}");
        }
    }
}

#[test]
fn nothing_to_repair_shows_no_finding_and_an_unchecked_version_says_so() {
    let lab = lab();
    // Put right by hand: B owns it, A forgot it.
    wt_file(&lab.a.1, &[], &[]);
    wt_file(&lab.b.1, &[("wt-7", &lab.wt, "local_main")], &[]);
    std::fs::write(
        lab.root.join("Claude.app/Contents/Info.plist"),
        "<?xml version=\"1.0\"?><plist><dict><key>CFBundleShortVersionString</key><string>5.11610.2</string></dict></plist>",
    )
    .unwrap();
    let info = findings(&lab.state, &Fake::default());
    assert!(info.findings.is_empty(), "{:?}", info.findings);
    assert!(info.tidied.is_empty(), "{:?}", info.tidied);
    assert!(!info.apps[0].checked);
    // Repair of a key that holds no more refuses, in words.
    let err = resolve(&lab.state, "wt-7@lab-a").err().unwrap();
    assert!(err.to_string().contains("nothing to repair"), "{err}");
}

#[test]
fn a_worktree_another_profile_tidies_is_an_info_line() {
    let lab = lab();
    wt_file(&lab.a.1, &[], &[&lab.wt]);
    wt_file(&lab.b.1, &[("wt-7", &lab.wt, "local_main")], &[]);
    let info = findings(&lab.state, &Fake::default());
    assert!(info.findings.is_empty());
    assert_eq!(info.tidied.len(), 1, "{:?}", info.tidied);
    let words = &info.tidied[0].words;
    assert!(
        words.contains("also tidies this folder; keep")
            && words.ends_with("running in it or commit your work"),
        "{words}"
    );
}

#[test]
fn a_conversation_with_two_worktrees_keeps_both() {
    let lab = lab();
    let fresh = worktree(&lab.root, "wt-8");
    record(&lab.b.1, &fresh);
    wt_file(&lab.b.1, &[("wt-8", &fresh, "local_main")], &[]);
    let info = findings(&lab.state, &Fake::default());
    assert_eq!(info.findings.len(), 1, "{:?}", info.findings);
    assert_eq!(info.findings[0].worktree, "wt-7");
    assert!(
        info.findings[0]
            .notes
            .iter()
            .any(|n| n == "This conversation has two worktrees; both are kept"),
        "{:?}",
        info.findings[0].notes
    );
}

/// The repair of `repairing` refused in words before anything quit.
fn refused_before_any_quit(lab: &Lab, repairing: &Repairing, fake: &Fake) {
    let err = run_repair(&lab.state, repairing, fake).err().unwrap();
    assert!(
        err.to_string().contains("Repair becomes available"),
        "{err}"
    );
    assert!(fake.quits.lock().unwrap().is_empty(), "nothing quit");
}

#[test]
fn a_busy_profile_refuses_the_repair_before_anything_is_quit() {
    let lab = lab();
    let mut fake = both_running(&lab);
    fake.working = 1;
    let key = findings(&lab.state, &fake).findings[0].key.clone();
    let repairing = resolve(&lab.state, &key).unwrap();
    refused_before_any_quit(&lab, &repairing, &fake);
    assert_eq!(
        read(&lab.a.1.join("git-worktrees.json"))["worktrees"]["wt-7"]["leasedBy"],
        "local_main"
    );
}

#[test]
fn an_unchecked_app_version_refuses_the_repair_in_words() {
    let lab = lab();
    let fake = both_running(&lab);
    let key = findings(&lab.state, &fake).findings[0].key.clone();
    let mut repairing = resolve(&lab.state, &key).unwrap();
    repairing.version = Some("5.11610.2".into());
    refused_before_any_quit(&lab, &repairing, &fake);
}

#[test]
fn repair_quits_only_the_two_profiles_moves_the_ownership_and_never_touches_the_worktree() {
    if !clones() {
        return;
    }
    let lab = lab();
    let fake = both_running(&lab);
    let before = std::fs::read(lab.wt.join("src/work.rs")).unwrap();
    let key = findings(&lab.state, &fake).findings[0].key.clone();
    let repairing = resolve(&lab.state, &key).unwrap();
    let done = run_repair(&lab.state, &repairing, &fake).unwrap();
    assert!(done.words.starts_with("Repaired"), "{done:?}");
    assert!(done.words.contains("weren't touched"), "{done:?}");
    let mut quits = fake.quits.lock().unwrap().clone();
    quits.sort_unstable();
    assert_eq!(
        quits,
        [500, 501],
        "exactly the two profiles, by their process"
    );
    assert_eq!(fake.starts.lock().unwrap().len(), 2, "both started again");
    let b = read(&lab.b.1.join("git-worktrees.json"));
    assert_eq!(b["worktrees"]["wt-7"]["leasedBy"], "local_main");
    let a = read(&lab.a.1.join("git-worktrees.json"));
    assert!(a["worktrees"].get("wt-7").is_none());
    assert!(
        a["untrackedDirGc"]["sightings"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    assert_eq!(std::fs::read(lab.wt.join("src/work.rs")).unwrap(), before);
    assert!(
        findings(&lab.state, &fake).findings.is_empty(),
        "re-detected: nothing left"
    );
}

/// 0.5.1 F91 (PL1 I13, F-PL1-23): a repair's row in Moves names the
/// conversation and says it was a repair (never "A conversation" and
/// "Moved"); its Undo gives the worktree back to the App it belonged to,
/// opens nothing, and says so.
#[test]
fn a_repairs_row_names_the_conversation_and_its_undo_gives_the_worktree_back() {
    if !clones() {
        return;
    }
    let lab = lab();
    let fake = both_running(&lab);
    let key = findings(&lab.state, &fake).findings[0].key.clone();
    let repairing = resolve(&lab.state, &key).unwrap();
    run_repair(&lab.state, &repairing, &fake).unwrap();
    let door = crate::features::handover::moving::moves(&lab.state);
    assert_eq!(door.len(), 1, "{door:?}");
    let row = &door[0];
    assert!(row.repair, "{row:?}");
    assert_eq!(row.title.as_deref(), Some("Fix the parser"));
    assert_eq!(row.state, "committed");
    assert!(row.can_undo, "a repair can be undone: {row:?}");
    let undone =
        crate::features::handover::moving::undo_with(&lab.state, &row.move_id, &fake).unwrap();
    assert!(
        undone
            .words
            .starts_with("Repair undone: the worktree wt-7 belongs to"),
        "{}",
        undone.words
    );
    let a = read(&lab.a.1.join("git-worktrees.json"));
    assert_eq!(
        a["worktrees"]["wt-7"]["leasedBy"], "local_main",
        "back as it was"
    );
    assert_eq!(findings(&lab.state, &fake).findings.len(), 1, "split again");
}
