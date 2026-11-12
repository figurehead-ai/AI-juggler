//! 0.3.9: the maintainer's failed switch of 2026-11-04, at its real size.
//! The Claude app conversation was an 800 MB transcript with a 502 MB
//! folder of 1,719 files and 904 file-history files, still open (so still
//! written to) while it switched. Every switch of it failed on 0.3.8; the
//! unit fixtures were a few kilobytes.
//!
//! This drives the real Claude adapter through the real switch pipeline on
//! a synthetic tree of that shape and prints how long each part took. It
//! writes about 1.3 GB to the temp folder (removed after), so it's opt-in:
//!
//! ```text
//! cargo test -p juggler-adapters --release --test switch_real_size -- --ignored --nocapture
//! ```
//!
//! `AJ_REAL_SIZE_MB` sets the transcript's size (default 800).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

use juggler_adapter::handover::sha256_file;
use juggler_adapters::ClaudeCodeAdapter;
use juggler_core::model::{Account, AccountId, HarnessId, TranscriptRef};
use juggler_handover::pipeline::{self, FreezeChoice, Mechanism};

const SESSION: &str = "1a967018-3bce-4925-b01a-0b46b11a35ea";
const CWD: &str = "/Users/you/Dev/manuscript";

fn account(state_dir: &Path, label: &str) -> Account {
    Account {
        id: AccountId::new(label),
        harness: HarnessId::new("claude-code"),
        label: label.to_owned(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

fn turn(n: usize, parent: Option<&str>, content: &str) -> String {
    serde_json::json!({
        "uuid": format!("t{n}"),
        "parentUuid": parent,
        "sessionId": SESSION,
        "cwd": CWD,
        "type": if n.is_multiple_of(2) { "user" } else { "assistant" },
        "message": {"role": if n.is_multiple_of(2) { "user" } else { "assistant" }, "content": content},
    })
    .to_string()
}

/// The maintainer's shape: the transcript, its folder (subagents and tool
/// results), its file history.
fn seed(config: &Path, megabytes: u64) -> std::path::PathBuf {
    let dir = config.join("projects").join(CWD.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    let main = dir.join(format!("{SESSION}.jsonl"));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&main).unwrap());
    let body = "w".repeat(4000);
    let mut written = 0u64;
    let mut n: usize = 0;
    while written < megabytes * 1024 * 1024 {
        let parent = n.checked_sub(1).map(|p: usize| format!("t{p}"));
        // A compaction partway, and now and then a very large tool output.
        if n == 1000 {
            let mark = serde_json::json!({
                "uuid": "c1", "parentUuid": null, "sessionId": SESSION, "cwd": CWD,
                "type": "system", "subtype": "compact_boundary", "content": "Conversation compacted",
            });
            writeln!(out, "{mark}").unwrap();
        }
        let content = if n % 5000 == 4999 {
            "x".repeat(2 * 1024 * 1024)
        } else {
            body.clone()
        };
        let row = turn(n, parent.as_deref(), &content);
        writeln!(out, "{row}").unwrap();
        written += row.len() as u64 + 1;
        n += 1;
    }
    out.flush().unwrap();
    let folder = dir.join(SESSION);
    std::fs::create_dir_all(folder.join("subagents")).unwrap();
    std::fs::create_dir_all(folder.join("tool-results")).unwrap();
    let chunk = "r".repeat(290 * 1024);
    for i in 0..1719_usize {
        if i.is_multiple_of(4) {
            std::fs::write(
                folder.join("subagents").join(format!("agent-{i}.jsonl")),
                format!("{}\n", turn(i, None, "sub")),
            )
            .unwrap();
        } else {
            std::fs::write(folder.join("tool-results").join(format!("{i}.txt")), &chunk).unwrap();
        }
    }
    let history = config.join("file-history").join(SESSION);
    std::fs::create_dir_all(&history).unwrap();
    for i in 0..904 {
        std::fs::write(history.join(format!("{i:016x}@v1")), "r".repeat(22 * 1024)).unwrap();
    }
    main
}

#[test]
#[ignore = "real size: writes about 1.3 GB to the temp folder"]
fn the_maintainers_switch_at_its_real_size_completes() {
    let megabytes: u64 = std::env::var("AJ_REAL_SIZE_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(800);
    let work = tempfile::tempdir().unwrap();
    let a = work.path().join("default");
    let b = work.path().join("acme-max");
    let clock = Instant::now();
    let main = seed(&a, megabytes);
    let size = std::fs::metadata(&main).unwrap().len();
    println!(
        "seeded {} MB in {:.1?}",
        size / (1024 * 1024),
        clock.elapsed()
    );
    let (before, _) = sha256_file(&main, None).unwrap();

    let steps = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = steps.clone();
    let started = Instant::now();
    pipeline::on_progress(Box::new(move |step| {
        seen.lock()
            .unwrap()
            .push((step.as_str(), started.elapsed()));
    }));
    let adapter = ClaudeCodeAdapter::new();
    let (record, pid) = pipeline::execute(
        &adapter,
        &TranscriptRef {
            path: main.clone(),
            native_id: Some(SESSION.to_owned()),
        },
        &account(&b, "acme-max"),
        FreezeChoice::StopNowAndSwitch,
        Mechanism::Transplant,
        &mut |install| {
            // The Claude app still has it open and writes its next turn.
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&main)
                .unwrap();
            writeln!(
                file,
                "{}",
                turn(9_999_999, None, "a turn written meanwhile")
            )
            .unwrap();
            assert!(install.reference.path.is_file());
            Ok(4242)
        },
    )
    .unwrap_or_else(|failure| panic!("the switch failed: {failure} (at {:?})", failure.stage()));
    let took = started.elapsed();
    for (step, at) in steps.lock().unwrap().iter() {
        println!("  {step:<10} at {at:.1?}");
    }
    println!("switched in {took:.1?}");
    assert_eq!(pid, 4242);
    assert_eq!(
        sha256_file(&record.install.reference.path, None).unwrap().0,
        before,
        "the transcript lands byte-identical"
    );
    let landed = b.join("projects").join(CWD.replace('/', "-")).join(SESSION);
    let count = |dir: &Path| std::fs::read_dir(dir).unwrap().count();
    assert_eq!(
        count(&landed.join("tool-results")) + count(&landed.join("subagents")),
        1719
    );
    assert_eq!(count(&b.join("file-history").join(SESSION)), 904);
    assert_eq!(record.fidelity_label, "F2", "its compaction is seen");
    let stepped: Vec<&str> = steps.lock().unwrap().iter().map(|(s, _)| *s).collect();
    assert_eq!(stepped, ["copying", "checking", "starting", "confirming"]);
}

/// A small disk image, mounted for one test and always ejected.
struct SmallDisk {
    mount: std::path::PathBuf,
}

impl SmallDisk {
    fn new(dir: &Path, megabytes: u32) -> Self {
        let image = dir.join("small.dmg");
        let mount = dir.join("small");
        let made = std::process::Command::new("hdiutil")
            .args([
                "create",
                "-size",
                &format!("{megabytes}m"),
                "-fs",
                "APFS",
                "-volname",
                "AJSmall",
            ])
            .arg(&image)
            .output()
            .unwrap();
        assert!(
            made.status.success(),
            "{}",
            String::from_utf8_lossy(&made.stderr)
        );
        std::fs::create_dir_all(&mount).unwrap();
        let attached = std::process::Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-mountpoint"])
            .arg(&mount)
            .arg(&image)
            .output()
            .unwrap();
        assert!(
            attached.status.success(),
            "{}",
            String::from_utf8_lossy(&attached.stderr)
        );
        Self { mount }
    }
}

impl Drop for SmallDisk {
    fn drop(&mut self) {
        let _ = std::process::Command::new("hdiutil")
            .args(["detach", "-force"])
            .arg(&self.mount)
            .output();
    }
}

/// A switch never fills a disk: a target on a disk with less room than
/// the copy plus what AI Juggler leaves free stops before writing, says
/// why, and leaves nothing behind; the source is untouched.
#[test]
#[ignore = "mounts a small disk image (macOS)"]
fn a_switch_to_a_nearly_full_disk_stops_before_filling_it() {
    let work = tempfile::tempdir().unwrap();
    let a = work.path().join("default");
    let main = seed(&a, 40);
    let (before, _) = sha256_file(&main, None).unwrap();
    let disk = SmallDisk::new(work.path(), 64);
    let b = disk.mount.join("acme-max");
    std::fs::create_dir_all(&b).unwrap();
    let failure = pipeline::execute(
        &ClaudeCodeAdapter::new(),
        &TranscriptRef {
            path: main.clone(),
            native_id: Some(SESSION.to_owned()),
        },
        &account(&b, "acme-max"),
        FreezeChoice::StopNowAndSwitch,
        Mechanism::Transplant,
        &mut |_| panic!("nothing starts when the copy can't be made"),
    )
    .expect_err("a nearly full disk stops the switch");
    let words = failure.words("Default", "acme-max");
    println!("{failure}\n{words}");
    assert!(words.contains("isn't enough free disk space"), "{words}");
    assert!(words.contains("GB more"), "it says how much: {words}");
    assert!(failure.retryable());
    // Nothing partial on the small disk; the source as it was.
    let left: Vec<_> = walk(&b);
    assert!(
        left.iter().all(|path| path.is_dir()),
        "files left behind: {left:?}"
    );
    assert_eq!(sha256_file(&main, None).unwrap().0, before);
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        }
        out.push(path);
    }
    out
}
