//! AJ-E302 evidence: the widened scan's probes over a COMMITTED
//! fixture home (rc assignments incl. malformed lines + dangling
//! targets, fish config, a cc-switch DB in each state, project
//! seeds, desktop presence) — plus the static guardrail that pins
//! the scan metadata-only discipline.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};

use juggler_adapter::environment::ScanHealth;
use juggler_adapters::scan_signals::{
    desktop_apps_present, project_seed_names, rc_files, scan_cc_switch, scan_rc_assignments,
};

fn fixture_home() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/homes/rich")
}

/// Copy the fixture home into a tempdir (tests may create target dirs
/// the dangling assignments point at, without touching the fixture).
fn fixture_home_copy() -> tempfile::TempDir {
    let work = tempfile::tempdir().unwrap();
    copy_tree(&fixture_home(), work.path());
    work
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn rc_scan_finds_assignments_with_source_lines_and_flags_dangling() {
    let home = fixture_home_copy();
    // Make one fish target exist so its hint is non-dangling.
    std::fs::create_dir_all(home.path().join("there-fish")).unwrap();
    let (hints, health) = scan_rc_assignments(home.path());
    assert_eq!(hints.len(), 4, "zshrc ×2 + bashrc ×1 + fish ×1: {hints:?}");

    let zsh_first = hints
        .iter()
        .find(|hint| hint.source_file.ends_with(".zshrc") && hint.variable == "CLAUDE_CONFIG_DIR")
        .unwrap();
    assert_eq!(zsh_first.line, 2, "source line recorded");
    assert!(zsh_first.path.ends_with("there"));
    assert!(zsh_first.target_exists, "~/ expanded against the home");

    let codex = hints
        .iter()
        .find(|hint| hint.variable == "CODEX_HOME")
        .unwrap();
    assert_eq!(codex.path, std::path::PathBuf::from("/opt/codex-fixture"));

    // The malformed shell line produced NO hint (never evaluated) and
    // OTHER_VAR was never parsed (name-scoped).
    assert!(
        hints
            .iter()
            .all(|hint| hint.variable == "CLAUDE_CONFIG_DIR" || hint.variable == "CODEX_HOME")
    );
    // The dangling ~/gone target flagged → Partial.
    assert!(matches!(health, ScanHealth::Partial(_)));
}

#[test]
fn rc_files_cover_exactly_the_scoped_set() {
    let home = fixture_home_copy();
    let files = rc_files(home.path());
    // .zshrc + .bashrc + the fish dir's *.fish (the non-.fish file
    // under fish/ is excluded).
    assert!(files.iter().any(|f| f.ends_with(".zshrc")));
    assert!(files.iter().any(|f| f.ends_with(".bashrc")));
    assert!(files.iter().any(|f| f.ends_with("alias.fish")));
    assert!(!files.iter().any(|f| f.ends_with("config.fath/config.fish")));
    assert_eq!(files.len(), 3);
}

#[test]
fn cc_switch_probe_reports_count_and_names_only() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join(".cc-switch");
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("cc-switch.db");
    {
        let connection = rusqlite::Connection::open(&db).unwrap();
        connection
            .execute(
                "CREATE TABLE providers (id INTEGER PRIMARY KEY, name TEXT, token TEXT)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO providers (name, token) VALUES ('work', 'never-read'), ('personal', 'never-read')",
                [],
            )
            .unwrap();
    }
    let hint = scan_cc_switch(work.path());
    assert!(hint.present);
    assert!(hint.detail.contains("2 provider(s)"));
    assert!(hint.detail.contains("work"));
    assert!(hint.detail.contains("personal"));
    // Names + count only — the token column never appears anywhere.
    assert!(!hint.detail.contains("never-read"));
    assert_eq!(hint.health, ScanHealth::Ok);
}

#[test]
fn cc_switch_probe_degrades_on_locked_or_corrupt_db() {
    // Corrupt (not a DB): open is lazy, the query fails → Partial,
    // existence-only.
    let corrupt = tempfile::tempdir().unwrap();
    let dir = corrupt.path().join(".cc-switch");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("cc-switch.db"), b"this is not a database at all").unwrap();
    let hint = scan_cc_switch(corrupt.path());
    assert!(hint.present);
    assert!(hint.detail.contains("existence only"));
    assert!(matches!(hint.health, ScanHealth::Partial(_)));

    // Unreadable: Denied, existence-only, never an error.
    let denied = tempfile::tempdir().unwrap();
    let dir = denied.path().join(".cc-switch");
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("cc-switch.db");
    std::fs::write(&db, b"x").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o000)).unwrap();
    let hint = scan_cc_switch(denied.path());
    assert!(hint.present);
    assert!(matches!(hint.health, ScanHealth::Denied(_)));

    // Missing: not present, Ok.
    let empty = tempfile::tempdir().unwrap();
    let hint = scan_cc_switch(empty.path());
    assert!(!hint.present);
    assert_eq!(hint.health, ScanHealth::Ok);
}

#[test]
fn project_seeds_return_directory_names_only() {
    let home = fixture_home_copy();
    let seeds = project_seed_names(&[home.path().join("claude")]);
    assert_eq!(seeds, ["-users-dev-app", "-users-dev-other"]);
}

#[test]
fn desktop_probe_checks_known_app_presence_only() {
    let work = tempfile::tempdir().unwrap();
    assert!(desktop_apps_present(work.path()).is_empty());
    std::fs::create_dir_all(work.path().join("Claude.app")).unwrap();
    assert_eq!(desktop_apps_present(work.path()), ["Claude.app"]);
}

/// The static guardrail (Req.7): the scan's probe code contains no
/// credential-content reads — no credential path literals, no token
/// columns, and the rc read is the ONLY file-content read (rc files
/// are paths, not credentials). Pinned by grepping the committed
/// source so a future edit that widens the surface fails here.
#[test]
fn scan_code_never_reads_credential_contents() {
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/scan_signals.rs"))
            .unwrap();
    for banned in [
        ".credentials",
        "auth.json",
        "authorization",
        "api_key",
        "keychain",
    ] {
        let lowered = source.to_lowercase();
        assert!(
            !lowered.contains(&banned.to_lowercase()),
            "scan_signals.rs must not mention {banned:?} — metadata only"
        );
    }
    // The rc read is the one sanctioned file-content read, and the
    // cc-switch query names its columns explicitly.
    assert!(source.contains("read_to_string") && source.contains("rc_files"));
    assert!(source.contains("SELECT id, name FROM providers"));
}
