//! AJ-E2323 R2 (the review's D2): the move engine on a full disk. The
//! engine has no fault seam, so the seam is the disk itself: a small APFS
//! disk image made in a temp folder, mounted out of sight (`-nobrowse`)
//! for the test, filled until the system says "no space left", and
//! detached at the end. Nothing outside the image is touched. macOS only;
//! skipped where an image can't be made.

#![cfg(target_os = "macos")]
// Test crates are pragmatic by policy (QUALITY.md §2); a skip says why.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stderr,
    clippy::pedantic
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use juggler_adapter::moving::{HoldWhy, MovePlan, MoveRequest, MoveSide, MoveStep, NotesChoice};
use juggler_handover::moving::{MoveError, Phase, Recovery, Stopped, Transaction};

/// A small APFS disk mounted in a temp folder; detached when dropped.
struct Disk {
    _tmp: tempfile::TempDir,
    mount: PathBuf,
    filler: Vec<PathBuf>,
}

impl Disk {
    /// 400 MB: room for the engine's 256 MB margin at the start.
    fn new() -> Option<Self> {
        let tmp = tempfile::tempdir().ok()?;
        let image = tmp.path().join("faults.sparseimage");
        let mount = tmp.path().join("disk");
        std::fs::create_dir(&mount).ok()?;
        let made = Command::new("/usr/bin/hdiutil")
            .args(["create", "-quiet", "-size", "400m", "-fs", "APFS"])
            .args(["-type", "SPARSE", "-volname", "aij-faults"])
            .arg(&image)
            .status()
            .ok()?;
        if !made.success() {
            eprintln!("skipped: no disk image here");
            return None;
        }
        let attached = Command::new("/usr/bin/hdiutil")
            .args([
                "attach",
                "-quiet",
                "-nobrowse",
                "-noautoopen",
                "-mountpoint",
            ])
            .arg(&mount)
            .arg(&image)
            .status()
            .ok()?;
        if !attached.success() {
            eprintln!("skipped: the disk image can't be mounted here");
            return None;
        }
        Some(Self {
            _tmp: tmp,
            mount,
            filler: Vec::new(),
        })
    }

    /// Writes until the disk is full: big writes, then small ones, then
    /// new files, until even a new file is refused for space.
    fn fill(&mut self) {
        for chunk in [8 << 20, 1 << 20, 4096, 512] {
            let path = self.mount.join(format!("filler-{chunk}"));
            let Ok(mut file) = std::fs::File::create(&path) else {
                break;
            };
            self.filler.push(path);
            let bytes = vec![0_u8; chunk];
            while file.write_all(&bytes).is_ok() {}
        }
        for n in 0.. {
            let path = self.mount.join(format!("filler-file-{n}"));
            match std::fs::write(&path, b"x") {
                Ok(()) => self.filler.push(path),
                Err(error) => {
                    assert_eq!(error.kind(), std::io::ErrorKind::StorageFull, "{error}");
                    break;
                }
            }
        }
    }

    /// Room again.
    fn free(&mut self) {
        for path in self.filler.drain(..) {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        let _ = Command::new("/usr/bin/hdiutil")
            .args(["detach", "-quiet", "-force"])
            .arg(&self.mount)
            .status();
    }
}

/// Two accounts' folders on `disk` and the move's folder beside them.
struct Lab {
    a: PathBuf,
    b: PathBuf,
    moves: PathBuf,
}

const HISTORY: &[u8] = b"{\"type\":\"user\"}\n{\"type\":\"assistant\"}\n";

fn lab(disk: &Disk) -> Lab {
    let a = disk.mount.join("a");
    let b = disk.mount.join("b");
    std::fs::create_dir_all(a.join("p/s/subagents")).unwrap();
    std::fs::create_dir_all(b.join("p")).unwrap();
    std::fs::write(a.join("p/s.jsonl"), HISTORY).unwrap();
    std::fs::write(a.join("p/s/subagents/x.jsonl"), b"{\"n\":1}\n").unwrap();
    std::fs::write(a.join("r.json"), b"{\"id\":1}").unwrap();
    Lab {
        moves: disk.mount.join("data/moves"),
        a,
        b,
    }
}

fn begin(lab: &Lab, id: &str) -> Transaction {
    let side = |dir: &Path| MoveSide {
        tool_dir: dir.to_path_buf(),
        host_dir: None,
        version: None,
    };
    let request = MoveRequest {
        conversation: "s".into(),
        source: side(&lab.a),
        target: side(&lab.b),
        notes: NotesChoice::KeepTarget,
    };
    let plan = MovePlan {
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
    };
    Transaction::begin(&lab.moves, id, "fake", request, plan, None, 1).unwrap()
}

fn is_full(error: &MoveError) -> bool {
    matches!(error, MoveError::Io { source, .. } if source.kind() == std::io::ErrorKind::StorageFull)
}

/// No half-written manifest left beside the move's own.
fn no_temp_files(dir: &Path) {
    let temps: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(temps.is_empty(), "{temps:?}");
}

/// The source as it was before the move.
fn baseline(lab: &Lab) -> bool {
    std::fs::read(lab.a.join("p/s.jsonl")).is_ok_and(|b| b == HISTORY)
        && lab.a.join("p/s/subagents/x.jsonl").is_file()
        && std::fs::read(lab.a.join("r.json")).is_ok_and(|b| b == b"{\"id\":1}")
        && !lab.b.join("p/s.jsonl").exists()
        && !lab.b.join("r.json").exists()
}

/// D2: the disk fills part way through applying (two steps done): the
/// move stops with "no space", its manifest and journal still read back
/// to the same place, and once room frees it goes on from there and its
/// checks pass.
#[test]
fn a_disk_that_fills_mid_apply_leaves_the_move_recoverable_and_it_finishes_once_room_frees() {
    let Some(mut disk) = Disk::new() else {
        return;
    };
    let lab = lab(&disk);
    let stopped = Stopped::checked();
    let mut txn = begin(&lab, "full-apply");
    txn.back_up(&stopped).unwrap();
    // Two steps done, then the third stops (its folder briefly elsewhere,
    // put back): an apply left part way, as the journal says.
    let subagents = lab.a.join("p/s/subagents");
    let aside = lab.a.join("p/aside");
    std::fs::rename(&subagents, &aside).unwrap();
    assert!(txn.apply(&stopped, 0).is_err());
    std::fs::rename(&aside, &subagents).unwrap();
    let left = Transaction::open(txn.dir()).unwrap();
    assert_eq!(left.recover().unwrap(), Recovery::Applying { next: 2 });

    disk.fill();
    let mut going_on = Transaction::open(txn.dir()).unwrap();
    let error = going_on.apply(&stopped, 2).unwrap_err();
    assert!(is_full(&error), "{error}");
    // Read back from the disk: where it was, nothing torn.
    let after = Transaction::open(txn.dir()).unwrap();
    assert_eq!(after.manifest.phase, Phase::Applying);
    assert_eq!(after.recover().unwrap(), Recovery::Applying { next: 2 });
    no_temp_files(txn.dir());
    assert!(lab.b.join("p/s.jsonl").is_file() && subagents.is_dir());

    disk.free();
    let mut finished = Transaction::open(txn.dir()).unwrap();
    finished.apply(&stopped, 2).unwrap();
    let report = finished.verify_cold(Vec::new()).unwrap();
    assert!(report.passed(), "{:?}", report.failures());
    assert_eq!(
        std::fs::read(lab.b.join("p/s.jsonl")).unwrap(),
        HISTORY,
        "the history moved byte for byte"
    );
}

/// D2: the disk is full when Undo runs: nothing is half undone, the move
/// reads back as it was, and once room frees Undo puts the baseline back.
#[test]
fn a_full_disk_during_undo_changes_nothing_and_undo_finishes_once_room_frees() {
    let Some(mut disk) = Disk::new() else {
        return;
    };
    let lab = lab(&disk);
    let stopped = Stopped::checked();
    let mut txn = begin(&lab, "full-undo");
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    assert!(!baseline(&lab));

    disk.fill();
    let error = txn.undo(&stopped, 2).unwrap_err();
    assert!(is_full(&error), "{error}");
    let after = Transaction::open(txn.dir()).unwrap();
    assert_eq!(after.manifest.phase, Phase::Applied, "nothing half undone");
    no_temp_files(txn.dir());
    assert!(lab.b.join("p/s.jsonl").is_file(), "still moved");

    disk.free();
    let mut undoing = Transaction::open(txn.dir()).unwrap();
    undoing.undo(&stopped, 3).unwrap();
    assert_eq!(undoing.manifest.phase, Phase::Undone);
    assert!(baseline(&lab), "the baseline is back");
}

/// D2: an Undo the disk filled under after it began (its phase written,
/// as a crash or a full disk leaves it) is safe to run again once room
/// frees.
#[test]
fn an_undo_cut_short_by_a_full_disk_runs_again_once_room_frees() {
    let Some(mut disk) = Disk::new() else {
        return;
    };
    let lab = lab(&disk);
    let stopped = Stopped::checked();
    let mut txn = begin(&lab, "full-undoing");
    txn.back_up(&stopped).unwrap();
    txn.apply(&stopped, 0).unwrap();
    // The undo began: its phase is on disk.
    let manifest = txn.dir().join("manifest.json");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        text.replace("\"phase\": \"applied\"", "\"phase\": \"undoing\""),
    )
    .unwrap();
    let mut undoing = Transaction::open(txn.dir()).unwrap();
    assert_eq!(undoing.recover().unwrap(), Recovery::Undoing);

    disk.fill();
    assert!(is_full(&undoing.undo(&stopped, 4).unwrap_err()));
    assert_eq!(
        Transaction::open(txn.dir()).unwrap().recover().unwrap(),
        Recovery::Undoing
    );
    no_temp_files(txn.dir());

    disk.free();
    let mut again = Transaction::open(txn.dir()).unwrap();
    again.undo(&stopped, 5).unwrap();
    assert!(baseline(&lab));
}
