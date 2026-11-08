//! The engine on its own, with plans made by hand (the Claude planner and
//! the engine together are `juggler-adapters/tests/claude_move_matrix.rs`).

use std::path::{Path, PathBuf};

use juggler_adapter::moving::{
    Appended, HoldWhy, MoveCheck, MovePlan, MoveRequest, MoveSide, MoveStep, NotesChoice,
};

use super::*;

struct Lab {
    _tmp: tempfile::TempDir,
    a: PathBuf,
    b: PathBuf,
    moves: PathBuf,
}

fn lab() -> Lab {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(a.join("p/s/subagents")).unwrap();
    std::fs::create_dir_all(b.join("p")).unwrap();
    std::fs::write(
        a.join("p/s.jsonl"),
        b"{\"type\":\"user\"}\n{\"type\":\"assistant\"}\n",
    )
    .unwrap();
    std::fs::write(a.join("p/s/subagents/x.jsonl"), b"{\"n\":1}\n").unwrap();
    std::fs::write(a.join("r.json"), b"{\"id\":1}").unwrap();
    let moves = tmp.path().join("data/moves");
    Lab {
        a,
        b,
        moves,
        _tmp: tmp,
    }
}

fn request(lab: &Lab) -> MoveRequest {
    let side = |dir: &Path| MoveSide {
        tool_dir: dir.to_path_buf(),
        host_dir: None,
        version: None,
    };
    MoveRequest {
        conversation: "s".into(),
        source: side(&lab.a),
        target: side(&lab.b),
        notes: NotesChoice::KeepTarget,
    }
}

fn plan(lab: &Lab) -> MovePlan {
    MovePlan {
        conversation: "s".into(),
        resume: "s".into(),
        roots: vec![lab.a.clone(), lab.b.clone()],
        steps: vec![
            MoveStep::Rename {
                from: lab.a.join("p/s.jsonl"),
                to: lab.b.join("p/s.jsonl"),
                history: true,
            },
            MoveStep::MakeDir {
                path: lab.b.join("p/s"),
            },
            MoveStep::Rename {
                from: lab.a.join("p/s/subagents"),
                to: lab.b.join("p/s/subagents"),
                history: false,
            },
            MoveStep::WriteRecord {
                path: lab.b.join("r.json"),
                value: serde_json::json!({"id": 1, "moved": true}),
            },
            MoveStep::Hold {
                path: lab.a.join("r.json"),
                why: HoldWhy::RetiredRecord,
            },
        ],
        ..MovePlan::default()
    }
}

fn begin(lab: &Lab, id: &str) -> Transaction {
    Transaction::begin(&lab.moves, id, "fake", request(lab), plan(lab), None, 1).unwrap()
}

fn clones() -> bool {
    let tmp = tempfile::tempdir().unwrap();
    juggler_platform::transfer::clones_supported(tmp.path()).unwrap_or(false)
}

#[test]
fn a_move_applies_checks_and_commits() {
    if !clones() {
        return;
    }
    let lab = lab();
    let id_before = juggler_platform::transfer::file_id(&lab.a.join("p/s.jsonl")).unwrap();
    let mut txn = begin(&lab, "m1");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    let report = txn.verify_cold(Vec::new()).unwrap();
    assert!(report.passed(), "{:?}", report.failures());
    assert_eq!(
        juggler_platform::transfer::file_id(&lab.b.join("p/s.jsonl")).unwrap(),
        id_before,
        "the history arrived as the same file"
    );
    // The app adds bookkeeping and a notice when it starts again.
    let mut grown = std::fs::read(lab.b.join("p/s.jsonl")).unwrap();
    grown.extend_from_slice(b"{\"type\":\"custom-title\"}\n{\"type\":\"notice\"}\n");
    std::fs::write(lab.b.join("p/s.jsonl"), grown).unwrap();
    let classify = |line: &[u8]| {
        if line.windows(6).any(|w| w == b"notice") {
            Appended::SystemNotice
        } else {
            Appended::Bookkeeping
        }
    };
    let report = txn.verify_opened(&classify, Vec::new()).unwrap();
    assert!(report.passed(), "{:?}", report.failures());
    assert_eq!(report.added.bookkeeping, 1);
    assert_eq!(report.added.notices, 1);
    assert_eq!(txn.manifest.phase, Phase::Committed);
    // A committed move can't run again (replay guard).
    assert!(txn.apply(&stopped, 0).is_err());
}

#[test]
fn a_history_whose_old_bytes_changed_fails_the_check_after() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m2");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    assert!(txn.verify_cold(Vec::new()).unwrap().passed());
    std::fs::write(lab.b.join("p/s.jsonl"), b"{\"type\":\"user\"}\n").unwrap();
    let report = txn
        .verify_opened(&|_| Appended::Bookkeeping, Vec::new())
        .unwrap();
    assert!(!report.passed());
    assert_ne!(txn.manifest.phase, Phase::Committed);
}

#[test]
fn an_adapters_failed_check_stops_before_the_apps_start() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m3");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    let report = txn
        .verify_cold(vec![MoveCheck {
            what: "the child is linked".into(),
            pass: false,
        }])
        .unwrap();
    assert!(!report.passed());
    assert_eq!(txn.manifest.phase, Phase::Applied);
}

#[test]
fn undo_restores_the_baseline_and_keeps_the_work_done_since() {
    if !clones() {
        return;
    }
    let lab = lab();
    let original = std::fs::read(lab.a.join("p/s.jsonl")).unwrap();
    let mut txn = begin(&lab, "m4");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    // Work after the move, in the target.
    let mut later = original.clone();
    later.extend_from_slice(b"{\"type\":\"user\",\"later\":true}\n");
    std::fs::write(lab.b.join("p/s.jsonl"), &later).unwrap();
    std::fs::write(lab.b.join("p/s/subagents/y.jsonl"), b"new agent\n").unwrap();
    let kept = txn.undo(&stopped, 99).unwrap();
    assert_eq!(std::fs::read(lab.a.join("p/s.jsonl")).unwrap(), original);
    assert!(lab.a.join("p/s/subagents/x.jsonl").is_file());
    assert_eq!(std::fs::read(lab.a.join("r.json")).unwrap(), b"{\"id\":1}");
    assert!(!lab.b.join("p/s.jsonl").exists());
    assert!(!lab.b.join("r.json").exists());
    assert!(
        !lab.b.join("p/s").exists(),
        "the folder the move made went once empty"
    );
    // The later work is kept, whole.
    let mut found_history = false;
    let mut found_agent = false;
    for entry in walk(&kept) {
        let bytes = std::fs::read(&entry).unwrap();
        found_history |= bytes == later;
        found_agent |= bytes == b"new agent\n";
    }
    assert!(found_history && found_agent);
    assert_eq!(txn.manifest.phase, Phase::Undone);
    assert!(txn.undo(&stopped, 100).is_err(), "undone once");
}

#[test]
fn a_corrupt_backup_stops_undo_before_anything_changes() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m5");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    let backup = txn
        .manifest
        .entries
        .iter()
        .find(|e| e.path == lab.a.join("p/s.jsonl"))
        .and_then(|e| e.backup.clone())
        .unwrap();
    std::fs::write(txn.dir().join("backups").join(backup), b"damaged").unwrap();
    let moved = std::fs::read(lab.b.join("p/s.jsonl")).unwrap();
    let error = txn.undo(&stopped, 5).unwrap_err().to_string();
    assert!(error.contains("doesn't match"), "{error}");
    assert_eq!(std::fs::read(lab.b.join("p/s.jsonl")).unwrap(), moved);
    assert_eq!(txn.manifest.phase, Phase::Applied);
}

#[test]
fn an_interrupted_move_is_read_back_from_its_journal_and_goes_on() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m6");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    // Steps 0 and 1 done; step 2 started and its rename happened, but the
    // journal never heard (the Mac stopped).
    txn.set_phase(Phase::Applying).unwrap();
    for step in 0..2 {
        txn.log(&serde_json::json!({"event": "intent", "step": step}))
            .unwrap();
        txn.step(step, &txn.manifest.plan.steps[step].clone())
            .unwrap();
        txn.log(&serde_json::json!({"event": "done", "step": step}))
            .unwrap();
    }
    txn.log(&serde_json::json!({"event": "intent", "step": 2}))
        .unwrap();
    txn.step(2, &txn.manifest.plan.steps[2].clone()).unwrap();
    // A torn journal line, as a crash leaves.
    std::fs::OpenOptions::new()
        .append(true)
        .open(txn.dir().join("journal.jsonl"))
        .unwrap()
        .write_all(b"{\"event\":\"do")
        .unwrap();
    let reopened = Transaction::open(txn.dir()).unwrap();
    assert_eq!(reopened.recover().unwrap(), Recovery::Applying { next: 3 });
    let mut reopened = reopened;
    reopened.apply(&stopped, 3).unwrap();
    assert!(reopened.verify_cold(Vec::new()).unwrap().passed());
}

#[test]
fn an_interrupted_step_that_never_happened_runs_again() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m7");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.set_phase(Phase::Applying).unwrap();
    txn.log(&serde_json::json!({"event": "intent", "step": 0}))
        .unwrap();
    let reopened = Transaction::open(txn.dir()).unwrap();
    assert_eq!(reopened.recover().unwrap(), Recovery::Applying { next: 0 });
}

#[test]
fn a_step_the_disk_cant_explain_goes_to_a_person() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m8");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.set_phase(Phase::Applying).unwrap();
    txn.log(&serde_json::json!({"event": "intent", "step": 0}))
        .unwrap();
    // Someone replaced the history by hand: neither side is the original.
    std::fs::remove_file(lab.a.join("p/s.jsonl")).unwrap();
    std::fs::write(lab.b.join("p/s.jsonl"), b"other").unwrap();
    let reopened = Transaction::open(txn.dir()).unwrap();
    assert!(matches!(reopened.recover().unwrap(), Recovery::Unclear(_)));
}

#[test]
fn a_file_changed_after_the_backup_stops_the_step() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m9");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    // Replaced (a new file) after the backup: another writer ran.
    std::fs::remove_file(lab.a.join("p/s.jsonl")).unwrap();
    std::fs::write(lab.a.join("p/s.jsonl"), b"x").unwrap();
    let error = txn.apply(&stopped, 0).unwrap_err().to_string();
    assert!(error.contains("changed since the backup"), "{error}");
    assert!(!lab.b.join("p/s.jsonl").exists());
}

#[test]
fn a_step_outside_the_roots_is_refused_before_anything() {
    let lab = lab();
    let mut plan = plan(&lab);
    plan.steps.push(MoveStep::Hold {
        path: PathBuf::from("/etc/hosts"),
        why: HoldWhy::RetiredRecord,
    });
    let refusals = preflight(&plan, &lab.moves);
    assert!(
        refusals.iter().any(|r| r.contains("outside")),
        "{refusals:?}"
    );
    assert!(Transaction::begin(&lab.moves, "m10", "fake", request(&lab), plan, None, 1).is_err());
    assert!(!lab.moves.join("m10").exists());
}

#[test]
fn a_new_plan_of_another_shape_is_refused() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m11");
    let mut changed = plan(&lab);
    changed.steps.pop();
    assert!(txn.replan(changed).is_err());
    let mut same = plan(&lab);
    if let MoveStep::WriteRecord { value, .. } = &mut same.steps[3] {
        *value = serde_json::json!({"id": 1, "moved": true, "written": "at quit"});
    }
    txn.replan(same).unwrap();
}

#[test]
fn a_second_move_of_the_same_folders_waits() {
    let lab = lab();
    let roots = vec![lab.a.clone(), lab.b.clone()];
    let held = Lock::take(&roots).unwrap();
    assert!(Lock::take(std::slice::from_ref(&lab.b)).is_err());
    drop(held);
    assert!(Lock::take(&roots).is_ok());
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn reading_an_interrupted_clone_back_changes_nothing() {
    if !clones() {
        return;
    }
    let lab = lab();
    std::fs::create_dir_all(lab.a.join("p/memory")).unwrap();
    std::fs::write(lab.a.join("p/memory/MEMORY.md"), b"notes").unwrap();
    let mut plan = plan(&lab);
    plan.steps.push(MoveStep::Clone {
        from: lab.a.join("p/memory"),
        to: lab.b.join("p/memory"),
    });
    let mut txn =
        Transaction::begin(&lab.moves, "m12", "fake", request(&lab), plan, None, 1).unwrap();
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    // As if the Mac stopped right after the clone, before its done line.
    txn.set_phase(Phase::Applying).unwrap();
    for step in 0..6 {
        txn.log(&serde_json::json!({"event": "intent", "step": step}))
            .unwrap();
        txn.step(step, &txn.manifest.plan.steps[step].clone()).unwrap();
        if step < 5 {
            txn.log(&serde_json::json!({"event": "done", "step": step}))
                .unwrap();
        }
    }
    std::fs::write(lab.b.join("p/memory/LATER.md"), b"written since").unwrap();
    let reopened = Transaction::open(txn.dir()).unwrap();
    assert!(matches!(reopened.recover().unwrap(), Recovery::Unclear(_)));
    assert!(
        lab.b.join("p/memory/LATER.md").is_file(),
        "reading removed nothing"
    );
}

#[test]
fn a_record_changed_after_the_backup_is_never_written_over() {
    if !clones() {
        return;
    }
    let lab = lab();
    std::fs::write(lab.b.join("r.json"), b"{\"theirs\":1}").unwrap();
    let mut txn = begin(&lab, "m13");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    std::fs::write(lab.b.join("r.json"), b"{\"theirs\":2}").unwrap();
    let error = txn.apply(&stopped, 0).unwrap_err().to_string();
    assert!(error.contains("changed since the backup"), "{error}");
    assert_eq!(
        std::fs::read(lab.b.join("r.json")).unwrap(),
        b"{\"theirs\":2}"
    );
}

#[test]
fn an_interrupted_undo_keeps_one_preserved_folder() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m14");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    txn.manifest.preserved = Some("first".into());
    let kept = txn.undo(&stopped, 99).unwrap();
    assert!(kept.ends_with("first"));
}
