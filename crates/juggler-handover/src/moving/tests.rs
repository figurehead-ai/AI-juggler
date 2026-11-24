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

/// `txn` backed up, applied and its checks before the apps start passed;
/// the writers' word, for what comes next.
fn applied_and_checked(txn: &mut Transaction) -> Stopped {
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    let report = txn.verify_cold(Vec::new()).unwrap();
    assert!(report.passed(), "{:?}", report.failures());
    stopped
}

/// Runs steps `0..until` as `apply` would, journaling each; the last one's
/// done line is left out when `crash` (the Mac stopped right after it).
fn run_steps(txn: &mut Transaction, until: usize, crash: bool) {
    txn.set_phase(Phase::Applying).unwrap();
    for step in 0..until {
        txn.log(&serde_json::json!({"event": "intent", "step": step}))
            .unwrap();
        txn.step(step, &txn.manifest.plan.steps[step].clone())
            .unwrap();
        if !(crash && step + 1 == until) {
            txn.log(&serde_json::json!({"event": "done", "step": step}))
                .unwrap();
        }
    }
}

/// A move of `id` backed up, then a step started that the journal never
/// heard end: only its intent line.
fn interrupted_at_start(lab: &Lab, id: &str) -> Transaction {
    let mut txn = begin(lab, id);
    txn.back_up(&Stopped::checked()).unwrap();
    txn.set_phase(Phase::Applying).unwrap();
    txn.log(&serde_json::json!({"event": "intent", "step": 0}))
        .unwrap();
    txn
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
    let stopped = applied_and_checked(&mut txn);
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
    run_steps(&mut txn, 3, true);
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
    let txn = interrupted_at_start(&lab, "m7");
    let reopened = Transaction::open(txn.dir()).unwrap();
    assert_eq!(reopened.recover().unwrap(), Recovery::Applying { next: 0 });
}

#[test]
fn a_step_the_disk_cant_explain_goes_to_a_person() {
    if !clones() {
        return;
    }
    let lab = lab();
    let txn = interrupted_at_start(&lab, "m8");
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
        refusals.iter().any(|r| r.words.contains("outside")),
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

/// AJ-E2315 (the review's E11): two spellings of one folder are one lock
/// (the old key was the raw path, so both moves ran).
#[test]
fn two_spellings_of_one_folder_are_one_lock() {
    let lab = lab();
    let held = Lock::take(std::slice::from_ref(&lab.a)).unwrap();
    let slash = PathBuf::from(format!("{}/", lab.a.display()));
    let dots = lab.a.join("x").join("..");
    for spelling in [slash, dots] {
        assert!(
            Lock::take(std::slice::from_ref(&spelling)).is_err(),
            "{spelling:?}"
        );
    }
    #[cfg(unix)]
    {
        let link = lab.a.with_extension("link");
        std::os::unix::fs::symlink(&lab.a, &link).unwrap();
        assert!(Lock::take(&[link]).is_err(), "through a link");
    }
    drop(held);
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
    run_steps(&mut txn, 6, true);
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

/// The journal's events, in order (phases by name).
fn journal(txn: &Transaction) -> Vec<String> {
    std::fs::read_to_string(txn.dir().join("journal.jsonl"))
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .map(|value| match value["event"].as_str() {
            Some("phase") => value["phase"].as_str().unwrap_or_default().to_owned(),
            Some(event) => event.to_owned(),
            None => String::new(),
        })
        .collect()
}

fn appends(path: &Path, line: &[u8]) {
    std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(line)
        .unwrap();
}

/// 0.4.9 (a 406 MB move refused: "the backup of … doesn't match it"):
/// the Claude app's Code tab wrote its last lines as the app closed,
/// while the history was backed up. A file that changes once while it is
/// read is backed up again after the caller's settle, and the journal
/// says so.
#[test]
fn a_file_that_changed_while_backed_up_is_backed_up_again_after_settling() {
    if !clones() {
        return;
    }
    let lab = lab();
    let history = lab.a.join("p/s.jsonl");
    let mut txn = begin(&lab, "m15");
    let mut wrote = false;
    let mut settled = 0;
    txn.back_up_retrying(
        &Stopped::checked(),
        &mut |path| {
            if path == history && !wrote {
                wrote = true;
                appends(&history, b"{\"type\":\"last-prompt\"}\n");
            }
        },
        &mut |_, _| {},
        &mut || settled += 1,
    )
    .unwrap();
    assert_eq!(settled, 1, "settled once before the second try");
    assert_eq!(txn.manifest.phase, Phase::BackedUp);
    assert_eq!(journal(&txn), ["planned", "backup_changed", "backed_up"]);
    let entry = txn
        .manifest
        .entries
        .iter()
        .find(|e| e.path == history)
        .unwrap();
    assert!(
        state::same_bytes(&history, &entry.before),
        "the grown file is the baseline"
    );
}

/// A file still changing on the second try refuses with its path for
/// Copy details; that try's clones go, and the undone move reads
/// planned → undone.
#[test]
fn a_file_still_changing_on_the_second_try_refuses_and_leaves_no_clone() {
    if !clones() {
        return;
    }
    let lab = lab();
    let history = lab.a.join("p/s.jsonl");
    let mut txn = begin(&lab, "m16");
    let error = txn
        .back_up_retrying(
            &Stopped::checked(),
            &mut |path| {
                if path == history {
                    appends(&history, b"{\"type\":\"cost-state\"}\n");
                }
            },
            &mut |_, _| {},
            &mut || {},
        )
        .unwrap_err();
    assert!(
        matches!(&error, MoveError::Changed(path) if *path == history),
        "{error}"
    );
    assert_eq!(txn.manifest.phase, Phase::Planned);
    assert!(walk(&txn.dir().join("backups")).is_empty(), "no clone left");
    txn.undo(&Stopped::checked(), 7).unwrap();
    assert_eq!(
        journal(&txn),
        ["planned", "backup_changed", "backup_changed", "undone"]
    );
}

/// 0.4.9: a long backup says how far it is: the bytes read rise to the
/// total (AJ-E2324: every file read once, its clone's bytes).
#[test]
fn a_large_backup_reports_its_progress_as_it_reads() {
    if !clones() {
        return;
    }
    let lab = lab();
    let history = lab.a.join("p/s.jsonl");
    let line = b"{\"type\":\"assistant\",\"pad\":\"................................\"}\n";
    let big = line.repeat((6 << 20) / line.len() + 1);
    std::fs::write(&history, &big).unwrap();
    let mut txn = begin(&lab, "m17");
    let mut seen: Vec<(u64, u64)> = Vec::new();
    txn.back_up_with(
        &Stopped::checked(),
        &mut |done, total| seen.push((done, total)),
        &mut || {},
    )
    .unwrap();
    let total = seen.last().unwrap().1;
    assert!(total >= big.len() as u64, "{total}");
    assert!(total < 2 * big.len() as u64, "read once: {total}");
    assert!(
        seen.len() >= 6,
        "about one report per MiB read: {}",
        seen.len()
    );
    assert!(seen.windows(2).all(|w| w[0].0 <= w[1].0), "never goes back");
    assert!(
        seen.iter().any(|(done, _)| *done > 0 && *done < total),
        "reported part way"
    );
    assert_eq!(seen.last().unwrap().0, total);
}

/// 0.4.9: an undone move's backups go (nothing is left to restore); one
/// in any other phase keeps its own, and work Undo kept aside stays.
#[test]
fn only_an_undone_moves_backups_are_let_go() {
    if !clones() {
        return;
    }
    let lab = lab();
    let stopped = Stopped::checked();
    let mut undone = begin(&lab, "m18");
    undone.back_up(&stopped).unwrap();
    undone.apply(&stopped, 0).unwrap();
    std::fs::write(lab.b.join("p/s/subagents/y.jsonl"), b"since\n").unwrap();
    let kept = undone.undo(&stopped, 9).unwrap();
    assert!(!walk(&undone.dir().join("backups")).is_empty());
    // Another move of the same files, backed up and applied: not undone.
    let mut live = begin(&lab, "m19");
    live.back_up(&stopped).unwrap();
    live.apply(&stopped, 0).unwrap();
    assert!(
        live.drop_backups().is_err(),
        "a live move keeps its backups"
    );
    assert_eq!(sweep_undone(&lab.moves), 1);
    assert!(walk(&undone.dir().join("backups")).is_empty());
    assert!(!walk(&kept).is_empty(), "the work kept aside stays");
    assert!(!walk(&live.dir().join("backups")).is_empty());
    assert_eq!(sweep_undone(&lab.moves), 0, "nothing more to let go");
    let reopened = Transaction::open(undone.dir()).unwrap();
    assert_eq!(
        journal(&reopened).last().map(String::as_str),
        Some("backups_dropped")
    );
}

/// AJ-E2328: another app file (two profiles' worktree files) is written
/// whole only while it is exactly its backup, checked after, and Undo puts
/// both back byte for byte.
#[test]
fn an_app_file_edit_is_backed_up_checked_and_undone_byte_for_byte() {
    if !clones() {
        return;
    }
    let lab = lab();
    let a_file = lab.a.join("git-worktrees.json");
    let b_file = lab.b.join("git-worktrees.json");
    let a_bytes =
        b"{\"schemaVersion\":2,  \"worktrees\":{\"w\":{\"leasedBy\":\"local_x\"}},\"future\":[1]}";
    let b_bytes = b"{\"schemaVersion\":2,\"worktrees\":{}}\n";
    std::fs::write(&a_file, a_bytes).unwrap();
    std::fs::write(&b_file, b_bytes).unwrap();
    let mut plan = plan(&lab);
    plan.steps.push(MoveStep::WriteAppFile {
        path: a_file.clone(),
        value: serde_json::json!({"schemaVersion": 2, "worktrees": {}, "future": [1]}),
    });
    plan.steps.push(MoveStep::WriteAppFile {
        path: b_file.clone(),
        value: serde_json::json!({"schemaVersion": 2, "worktrees": {"w": {"leasedBy": "local_x"}}}),
    });
    assert!(plan.shape().iter().any(|line| line.starts_with("edit ")));
    let mut txn =
        Transaction::begin(&lab.moves, "edit-1", "fake", request(&lab), plan, None, 1).unwrap();
    let stopped = applied_and_checked(&mut txn);
    let now: serde_json::Value = serde_json::from_slice(&std::fs::read(&b_file).unwrap()).unwrap();
    assert_eq!(now["worktrees"]["w"]["leasedBy"], "local_x");
    txn.undo(&stopped, 2).unwrap();
    assert_eq!(std::fs::read(&a_file).unwrap(), a_bytes);
    assert_eq!(std::fs::read(&b_file).unwrap(), b_bytes);
}

/// AJ-E2328: an app file that changed after its backup stops the move
/// there (the app wrote it meanwhile), nothing written over it.
#[test]
fn an_app_file_changed_since_its_backup_stops_the_move() {
    if !clones() {
        return;
    }
    let lab = lab();
    let file = lab.b.join("git-worktrees.json");
    std::fs::write(&file, b"{\"schemaVersion\":2}").unwrap();
    let mut plan = plan(&lab);
    plan.steps.insert(
        0,
        MoveStep::WriteAppFile {
            path: file.clone(),
            value: serde_json::json!({"schemaVersion": 2, "worktrees": {}}),
        },
    );
    let mut txn =
        Transaction::begin(&lab.moves, "edit-2", "fake", request(&lab), plan, None, 1).unwrap();
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    std::fs::write(&file, b"{\"schemaVersion\":2,\"app\":\"wrote\"}").unwrap();
    assert!(txn.apply(&stopped, 0).is_err());
    assert_eq!(
        std::fs::read(&file).unwrap(),
        b"{\"schemaVersion\":2,\"app\":\"wrote\"}"
    );
}

/// AJ-E2321 (A5.1): a move a crash left backed up never changed a file:
/// it is let go without touching any path, even one an app wrote since.
#[test]
fn a_backed_up_move_left_by_a_crash_is_let_go_without_touching_a_file() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m30");
    txn.back_up(&Stopped::checked()).unwrap();
    std::fs::write(lab.a.join("p/s.jsonl"), b"written since\n").unwrap();
    txn.abandon().unwrap();
    assert_eq!(txn.manifest.phase, Phase::Undone);
    assert_eq!(
        std::fs::read(lab.a.join("p/s.jsonl")).unwrap(),
        b"written since\n",
        "the app's own write is never put aside"
    );
    assert!(walk(&txn.dir().join("backups")).is_empty());
    assert!(journal(&txn).iter().any(|event| event == "abandoned"));
    // One that changed files is Undo's, never let go.
    let mut applied = begin(&lab, "m31");
    applied.back_up(&Stopped::checked()).unwrap();
    applied.apply(&Stopped::checked(), 0).unwrap();
    assert!(applied.abandon().is_err());
}

/// AJ-E2321 (A6.4): a committed move's backups retire; Undo then has
/// nothing to restore from, and says so.
#[test]
fn a_committed_moves_backups_retire_and_its_undo_goes_with_them() {
    if !clones() {
        return;
    }
    let lab = lab();
    let stopped = Stopped::checked();
    let mut txn = begin(&lab, "m32");
    txn.back_up(&stopped).unwrap();
    assert!(txn.retire().is_err(), "a live move keeps its backups");
    txn.apply(&stopped, 0).unwrap();
    txn.verify_cold(Vec::new()).unwrap();
    txn.verify_opened(&|_| Appended::Turn, Vec::new()).unwrap();
    assert_eq!(txn.manifest.phase, Phase::Committed);
    assert!(txn.backups_kept());
    assert!(txn.retire().unwrap());
    assert!(!txn.backups_kept());
    assert!(!txn.retire().unwrap(), "nothing more to let go");
    assert!(
        txn.journal()
            .iter()
            .any(|line| line["event"] == "backups_retired")
    );
}

/// AJ-E2321 (A6.1): the room a move asks for counts Undo's: its records
/// written again and each path's entries, above the margin.
#[test]
fn the_room_a_move_needs_counts_its_undo() {
    let lab = lab();
    let plan = plan(&lab);
    let record = serde_json::json!({"id": 1, "moved": true})
        .to_string()
        .len() as u64;
    let needed = room_needed(&plan);
    assert!(needed >= ROOM_MARGIN + 2 * record, "{needed}");
    let mut bigger = plan.clone();
    bigger.steps.push(MoveStep::MakeDir {
        path: lab.b.join("q"),
    });
    assert!(room_needed(&bigger) > needed, "every path's entries count");
}

/// AJ-E2322 (the review's B7): every phase is in the journal with its
/// time (ms), and the caller's timeline lines carry how long a phase took
/// and its wait set, never overwriting the event's own fields.
#[test]
fn the_journal_keeps_a_timeline_of_every_phase_in_ms() {
    let lab = lab();
    let mut txn = begin(&lab, "1-timeline");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.timeline(
        "closing",
        1234,
        serde_json::json!({"waited": [{"program": "claude", "pid": 42, "reason": "writes it", "evidence": "its session file"}], "remained": [], "event": "not this"}),
    )
    .unwrap();
    let lines = txn.journal();
    let phases: Vec<&serde_json::Value> = lines.iter().filter(|l| l["event"] == "phase").collect();
    assert_eq!(phases.len(), 2, "{lines:?}");
    assert!(
        phases.iter().all(|l| l["at_ms"].as_i64().unwrap_or(0) > 0),
        "{lines:?}"
    );
    let timeline = lines.iter().find(|l| l["event"] == "timeline").unwrap();
    assert_eq!(timeline["phase"], "closing");
    assert_eq!(timeline["took_ms"], 1234);
    assert!(timeline["at_ms"].as_i64().unwrap_or(0) > 0);
    assert_eq!(timeline["waited"][0]["program"], "claude");
    assert!(timeline["remained"].as_array().unwrap().is_empty());
}

/// AJ-E2324 R3 (the review's A6.2 and C3): a whole move (backup, apply,
/// the check before the apps start and the one after) reads every byte
/// of the moved history twice (the clone's hash and the after-open
/// prefix), not four times.
#[test]
fn a_move_reads_every_byte_twice_not_four_times() {
    if !clones() {
        return;
    }
    let lab = lab();
    let line = b"{\"type\":\"assistant\",\"pad\":\"................................\"}\n";
    let big = line.repeat((4 << 20) / line.len() + 1);
    std::fs::write(lab.a.join("p/s.jsonl"), &big).unwrap();
    let mut txn = begin(&lab, "m30");
    state::READ.with(|read| read.set(0));
    let _stopped = applied_and_checked(&mut txn);
    let report = txn
        .verify_opened(&|_| Appended::Bookkeeping, Vec::new())
        .unwrap();
    assert!(report.passed(), "{:?}", report.failures());
    let read = state::READ.with(std::cell::Cell::get);
    let big = big.len() as u64;
    assert!(read >= 2 * big, "{read}");
    assert!(
        read < 2 * big + (64 << 10),
        "two reads, not four: {read} of {big}"
    );
}

/// AJ-E2324 R3: the check before the apps start reads no byte, yet a file
/// written after the backup (same size, a new time of last write) fails
/// it.
#[test]
fn the_cold_check_by_identity_size_and_time_catches_a_write() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m31");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    // The same length, other bytes, in place (the same file).
    let moved = lab.b.join("p/s.jsonl");
    let len = std::fs::metadata(&moved).unwrap().len();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(&moved)
        .unwrap();
    file.write_all(&vec![b'x'; usize::try_from(len).unwrap()])
        .unwrap();
    drop(file);
    let report = txn.verify_cold(Vec::new()).unwrap();
    assert!(!report.passed());
    assert!(
        report
            .failures()
            .iter()
            .any(|f| f.ends_with("s.jsonl has every byte it had")),
        "{:?}",
        report.failures()
    );
}

/// AJ-E2324 R3: a move recorded before 0.5.1 (no times of last write)
/// is still checked, by its bytes.
#[test]
fn a_move_recorded_without_times_is_checked_by_its_bytes() {
    if !clones() {
        return;
    }
    let lab = lab();
    let mut txn = begin(&lab, "m32");
    let stopped = Stopped::checked();
    txn.back_up(&stopped).unwrap();
    for entry in &mut txn.manifest.entries {
        match &mut entry.before {
            State::File(file) => file.mtime_ns = None,
            State::Dir(dir) => dir.files.iter_mut().for_each(|(_, f)| f.mtime_ns = None),
            State::Absent => {}
        }
    }
    txn.apply(&stopped, 0).unwrap();
    state::READ.with(|read| read.set(0));
    assert!(txn.verify_cold(Vec::new()).unwrap().passed());
    assert!(state::READ.with(std::cell::Cell::get) > 0, "read its bytes");
}
