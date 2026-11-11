#![no_main]
//! Fuzz target: a backup archive being staged for a restore (AJ-E1009,
//! AJ-E2006, AJ-E2008). Any bytes, as a `.ajbackup` someone sent, must be
//! refused or staged without a panic, and nothing is ever written outside
//! the staging folder. Run: `cargo +nightly fuzz run backup_staging`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(dir) = tempfile::tempdir() else { return };
    let archive = dir.path().join("in.ajbackup");
    let staging = dir.path().join("staging");
    if std::fs::write(&archive, data).is_err() || std::fs::create_dir_all(&staging).is_err() {
        return;
    }
    let _ = juggler_core::backup_archive::stage_archive(&archive, &staging);
    for entry in walk(dir.path()) {
        assert!(
            entry == archive || entry.starts_with(&staging),
            "written outside staging: {}",
            entry.display()
        );
    }
});

fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            out.push(path);
        }
    }
    out
}
