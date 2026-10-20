//! Environment scan + config import (M1-3b, ADR-0006): the onboarding
//! promise — find every existing Claude setup and import it read-only.
//!
//! Login probes are injected closures so tests are deterministic and never
//! touch the real keychain.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;

use juggler_adapter::HarnessAdapter;
use juggler_adapter::environment::{InstallOrigin, LoginProbe};
use juggler_adapters::ClaudeCodeAdapter;
use juggler_adapters::claude_environment::{
    MAX_IMPORT_BYTES, import_claude_config, import_settings_only, import_with_cap, inspect_source,
    scan_claude_environment,
};

fn logged_in_probe(path: &Path) -> LoginProbe {
    // Deterministic stand-in for the metadata-only keychain probe: any
    // dir containing `.credentials.json` counts as signed in.
    if path.join(".credentials.json").is_file() {
        LoginProbe::LoggedIn
    } else {
        LoginProbe::LoggedOut
    }
}

fn logged_out_probe(_path: &Path) -> LoginProbe {
    LoginProbe::LoggedOut
}

/// Builds a realistic existing setup: settings, credentials, one project
/// transcript, plus noise that must be skipped on import.
fn existing_setup(home: &Path, name: &str) -> std::path::PathBuf {
    let dir = home.join(name);
    std::fs::create_dir_all(dir.join("projects/-users-dev-app")).unwrap();
    std::fs::write(dir.join("settings.json"), r#"{"model":"sonnet"}"#).unwrap();
    std::fs::write(dir.join(".credentials.json"), "{}").unwrap();
    std::fs::write(
        dir.join("projects/-users-dev-app/session-1.jsonl"),
        "{\"type\":\"user\"}\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("shell-snapshots")).unwrap();
    std::fs::write(dir.join("shell-snapshots/x.sh"), "noise").unwrap();
    std::fs::create_dir_all(dir.join("statsig")).unwrap();
    std::fs::write(dir.join("statsig/cache.db"), "noise").unwrap();
    dir
}

#[test]
fn scan_finds_default_env_override_and_manual_patterns() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    let default = existing_setup(&home, ".claude");
    let manual = existing_setup(&home, ".claude-work");
    existing_setup(&home, ".claude-personal");

    // Noise the scan must ignore: the vendor's state FILE, an empty
    // look-alike dir, and an unrelated dotdir.
    std::fs::write(home.join(".claude.json"), "{}").unwrap();
    std::fs::create_dir_all(home.join(".claude-empty")).unwrap();
    std::fs::create_dir_all(home.join(".other")).unwrap();

    let env_override = existing_setup(work.path(), "managed-elsewhere");
    let found = scan_claude_environment(&home, Some(env_override.as_path()), &[], &logged_in_probe);

    let paths: Vec<String> = found
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(
        paths.contains(&".claude".to_string()),
        "default must be found: {paths:?}"
    );
    assert!(paths.contains(&".claude-work".to_string()), "{paths:?}");
    assert!(paths.contains(&".claude-personal".to_string()), "{paths:?}");
    assert!(
        paths.contains(&"managed-elsewhere".to_string()),
        "env override must be found: {paths:?}"
    );
    assert_eq!(found.len(), 4, "noise filtered: {paths:?}");

    let by_origin = |origin: InstallOrigin| found.iter().filter(|f| f.origin == origin).count();
    assert_eq!(by_origin(InstallOrigin::VendorDefault), 1);
    assert_eq!(by_origin(InstallOrigin::EnvOverride), 1);
    assert_eq!(by_origin(InstallOrigin::HomePattern), 2);

    // Default sorts first; every found install has user-facing notes and
    // login evidence.
    assert_eq!(found[0].origin, InstallOrigin::VendorDefault);
    assert_eq!(found[0].path, default.canonicalize().unwrap());
    assert_eq!(found[0].login, LoginProbe::LoggedIn);
    assert!(!found[0].notes.is_empty());
    assert!(found[0].bytes > 0);
    assert!(found[0].last_used_secs.is_some());
    assert!(!found[0].managed);
    let _ = manual;
}

#[test]
fn scan_dedupes_when_env_override_points_at_default() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    existing_setup(&home, ".claude");

    let same = home.join(".claude");
    let found = scan_claude_environment(&home, Some(same.as_path()), &[], &logged_in_probe);
    assert_eq!(found.len(), 1, "same path via two candidates → one entry");
    assert_eq!(found[0].origin, InstallOrigin::VendorDefault);
}

#[test]
fn scan_marks_juggler_managed_dirs() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let dir = existing_setup(&home, ".claude");

    // A dir prepared through the adapter carries the managed marker.
    let adapter = ClaudeCodeAdapter::new();
    adapter.prepare_account(&dir).unwrap();

    let found = scan_claude_environment(&home, None, &[], &logged_out_probe);
    assert_eq!(found.len(), 1);
    assert!(found[0].managed);
    assert!(
        found[0]
            .notes
            .iter()
            .any(|note| note.contains("Already managed"))
    );
}

#[test]
fn import_copies_state_skips_caches_and_never_touches_source() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let source = existing_setup(&home, ".claude");
    let source_snapshot = file_tree(&source);
    let dest = work.path().join("app-data/accounts/claude-code/acc-1");

    let outcome = import_with_cap(&source, &dest, &logged_in_probe, MAX_IMPORT_BYTES).unwrap();

    // Settings, credentials, and transcripts carried over.
    assert!(dest.join("settings.json").is_file());
    assert!(dest.join(".credentials.json").is_file());
    assert!(
        dest.join("projects/-users-dev-app/session-1.jsonl")
            .is_file()
    );
    // Noise skipped, with recorded reasons.
    assert!(!dest.join("shell-snapshots").exists());
    assert!(!dest.join("statsig").exists());
    assert!(
        outcome
            .skipped
            .iter()
            .any(|s| s.contains("shell-snapshots"))
    );
    // The managed marker marks the copy as ours.
    assert!(juggler_adapters::claude_code::is_managed_dir(&dest));
    assert_eq!(outcome.files_copied, 3);
    assert!(outcome.bytes_copied > 0);
    // Login carried (file-based credentials) with the matching note.
    assert_eq!(outcome.login, LoginProbe::LoggedIn);
    assert_eq!(
        outcome.login_note.as_deref(),
        Some("Login carried over with the import.")
    );

    // The source is byte-identical and unmarked — read-only guarantee.
    assert_eq!(file_tree(&source), source_snapshot);
    assert!(!juggler_adapters::claude_code::is_managed_dir(&source));
}

#[test]
fn import_explains_keychain_locked_logins() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    // A macOS-style setup: logged in via keychain (probe says LoggedIn for
    // the source) but the copied dir has no credentials file.
    let source = existing_setup(&home, ".claude");
    std::fs::remove_file(source.join(".credentials.json")).unwrap();
    let dest = work.path().join("dest");

    // Source logged in (simulated keychain), destination not.
    let probe = |path: &Path| {
        if path == source.canonicalize().unwrap() {
            LoginProbe::LoggedIn
        } else {
            LoginProbe::LoggedOut
        }
    };
    let outcome = import_with_cap(&source, &dest, &probe, MAX_IMPORT_BYTES).unwrap();
    assert_eq!(outcome.login, LoginProbe::LoggedOut);
    let note = outcome.login_note.expect("keychain note");
    assert!(note.contains("Keychain"), "explains the real cause: {note}");
    assert!(note.contains("Sign in once"), "gives the next step: {note}");
}

#[test]
fn import_refuses_implausible_managed_and_existing() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    // Not a config dir: no evidence files.
    let bare = home.join(".claude-noise");
    std::fs::create_dir_all(&bare).unwrap();
    let err = import_with_cap(
        &bare,
        &work.path().join("d1"),
        &logged_out_probe,
        MAX_IMPORT_BYTES,
    )
    .unwrap_err();
    assert!(err.to_string().contains("does not look like"));

    // Already managed: refuse to import our own dir.
    let managed = existing_setup(&home, ".claude");
    ClaudeCodeAdapter::new().prepare_account(&managed).unwrap();
    let err = import_with_cap(
        &managed,
        &work.path().join("d2"),
        &logged_out_probe,
        MAX_IMPORT_BYTES,
    )
    .unwrap_err();
    assert!(err.to_string().contains("already managed"));

    // Destination already exists.
    let source = existing_setup(&home, ".claude-second");
    let dest = work.path().join("d3");
    std::fs::create_dir_all(&dest).unwrap();
    let err = import_with_cap(&source, &dest, &logged_out_probe, MAX_IMPORT_BYTES).unwrap_err();
    assert!(err.to_string().contains("destination already exists"));
}

#[test]
fn import_enforces_ceiling_before_writing() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let source = existing_setup(&home, ".claude");
    let dest = work.path().join("dest");

    // Ceiling smaller than the tree: fail with the plain-language message,
    // and write nothing.
    let err = import_with_cap(&source, &dest, &logged_out_probe, 1).unwrap_err();
    assert!(err.to_string().contains("import ceiling"), "{err}");
    assert!(!dest.exists(), "no partial import on ceiling breach");
}

/// Path→size snapshot for read-only verification of a source tree.
fn file_tree(root: &Path) -> Vec<(String, u64)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, u64)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let meta = entry.metadata().unwrap();
            let rel = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if meta.is_dir() {
                walk(&entry.path(), root, out);
            } else {
                out.push((rel, meta.len()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn scan_finds_other_manager_tools_profile_roots() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    // A manager tool (t3.codes-shaped) profile root with two profiles and
    // one non-config child that must be filtered.
    let root = work.path().join("ClaudeCodeProfiles");
    let profile_a = existing_setup(&root, "client-a");
    existing_setup(&root, "personal-max");
    std::fs::create_dir_all(root.join("not-a-profile")).unwrap();

    let found = scan_claude_environment(&home, None, &[root], &logged_in_probe);
    let manager: Vec<_> = found
        .iter()
        .filter(|f| f.origin == InstallOrigin::ManagerTool)
        .collect();
    assert_eq!(manager.len(), 2, "two profiles, noise filtered: {found:?}");
    assert!(
        manager
            .iter()
            .any(|f| f.path == profile_a.canonicalize().unwrap())
    );
    assert!(
        manager
            .iter()
            .all(|f| f.notes.iter().any(|n| n.contains("never touched")))
    );
    // Manager-tool entries rank after the vendor default and home patterns.
    assert_eq!(found.len(), 2, "home has no other setups: {found:?}");
}

// ---- AJ-E307: inspection, degraded import, session-scoped cleanup ----

#[test]
fn inspect_source_reports_breakdown_and_skips_cache_e307() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = existing_setup(&home, ".claude-work");
    let inspect = inspect_source(&source, &|_| LoginProbe::LoggedOut).unwrap();
    assert!(inspect.is_config, "fixture is a config dir");
    assert!(!inspect.already_managed);
    assert_eq!(inspect.ceiling_bytes, MAX_IMPORT_BYTES);
    // The breakdown names the top-level entries that WILL copy;
    // rebuildable cache dirs (shell-snapshots/, statsig/) never list.
    let names: Vec<&str> = inspect.folders.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"projects/"), "breakdown: {inspect:?}");
    assert!(names.contains(&"settings.json"), "breakdown: {inspect:?}");
    assert!(!names.iter().any(|n| n.starts_with("shell-snapshots")));
    // projects/ carries the transcript's bytes; settings.json its own.
    let projects = inspect
        .folders
        .iter()
        .find(|(n, _)| n == "projects/")
        .unwrap();
    assert!(projects.1 > 0);
    let total: u64 = inspect.folders.iter().map(|(_, b)| b).sum();
    assert!(total <= inspect.total_bytes);
}

#[test]
fn settings_only_import_skips_history_and_flags_degraded_e307() {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = existing_setup(&home, ".claude-big");
    let dest = work.path().join("managed");
    let outcome = import_settings_only(&source, &dest, &|_| LoginProbe::LoggedOut).unwrap();
    assert!(outcome.degraded);
    // Settings came; history did not.
    assert!(dest.join("settings.json").is_file());
    assert!(!dest.join("projects").exists(), "no transcript files");
    let skipped_history = outcome
        .skipped
        .iter()
        .find(|s| s.starts_with("projects/"))
        .expect("the skip names what stayed behind");
    assert!(skipped_history.contains("history — skipped at your choice"));
    // The full import WOULD have brought projects/ (the choice was real).
    let full = import_claude_config(&source, &work.path().join("managed-full"), &|_| {
        LoginProbe::LoggedOut
    })
    .unwrap();
    assert!(!full.degraded);
    assert!(full.files_copied > outcome.files_copied);
}

#[test]
#[cfg(unix)]
fn partial_failure_cleans_up_this_sessions_dir_e307() {
    // Injected copy failure: a source file whose permissions deny the
    // read (owner included — the bits apply to the owner on Unix).
    // Collect passes (metadata only), the copy fails MID-loop, and
    // cleanup removes this session's partial managed dir.
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = existing_setup(&home, ".claude-fail");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        source.join("projects/-users-dev-app/session-1.jsonl"),
        std::fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let dest = work.path().join("managed");
    let err = import_claude_config(&source, &dest, &|_| LoginProbe::LoggedOut).unwrap_err();
    assert!(
        err.to_string().contains("partial import was removed"),
        "error carries the recovery copy: {err}"
    );
    // Nothing at dest — orphans never accumulate silently.
    assert!(!dest.exists());
    // Restoring the mode keeps the fixture tidy for tempdir cleanup.
    let _ = std::fs::set_permissions(
        source.join("projects/-users-dev-app/session-1.jsonl"),
        std::fs::Permissions::from_mode(0o644),
    );
}

#[test]
fn source_immutability_full_and_degraded_e307() {
    // Req: quality gate — tree snapshot (path+size) before/after must
    // be byte-identical; the import never writes into the source.
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = existing_setup(&home, ".claude-immutable");
    let before = snapshot_tree(&source);
    import_claude_config(&source, &work.path().join("m1"), &|_| LoginProbe::LoggedOut).unwrap();
    import_settings_only(&source, &work.path().join("m2"), &|_| LoginProbe::LoggedOut).unwrap();
    inspect_source(&source, &|_| LoginProbe::LoggedOut).unwrap();
    assert_eq!(
        before,
        snapshot_tree(&source),
        "source is read-only to imports"
    );
}

fn snapshot_tree(dir: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, u64)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let rel = entry
                .path()
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if entry.path().is_dir() {
                walk(base, &entry.path(), out);
            } else {
                out.push((rel, entry.metadata().unwrap().len()));
            }
        }
    }
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// 0.0.7 dogfood (2026-10-20): a symlinked `skills/<name>` made the whole
/// import fail ("neither a regular file nor a symlink to a regular file").
/// Links are now recreated as links — never followed, so nothing outside
/// the setup is copied; links inside the setup point into the copy; a
/// linked sign-in file is not linked (the copy signs in once).
#[test]
#[cfg(unix)]
fn symlinks_are_recreated_as_links_never_followed() {
    use std::os::unix::fs::symlink;
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let source = existing_setup(&home, ".claude-links");
    // Outside the setup: a skill folder and a secret file.
    let dotfiles = work.path().join("dotfiles/ego-browser");
    std::fs::create_dir_all(&dotfiles).unwrap();
    std::fs::write(dotfiles.join("SKILL.md"), "a skill").unwrap();
    let secret = work.path().join("id_ed25519");
    std::fs::write(&secret, "PRIVATE KEY").unwrap();
    std::fs::create_dir_all(source.join("skills")).unwrap();
    symlink(&dotfiles, source.join("skills/ego-browser")).unwrap();
    symlink(&secret, source.join("skills/key")).unwrap();
    // Inside the setup, absolute and relative; and a dangling link.
    symlink(
        source.join("settings.json"),
        source.join("skills/settings-abs"),
    )
    .unwrap();
    symlink("../settings.json", source.join("skills/settings-rel")).unwrap();
    symlink(work.path().join("gone"), source.join("skills/dangling")).unwrap();
    // A linked sign-in file.
    std::fs::remove_file(source.join(".credentials.json")).ok();
    symlink(&secret, source.join(".credentials.json")).unwrap();

    let dest = work.path().join("managed");
    let outcome = import_claude_config(&source, &dest, &|_| LoginProbe::LoggedOut).unwrap();

    let link = |name: &str| std::fs::read_link(dest.join(name)).unwrap();
    assert!(
        std::fs::symlink_metadata(dest.join("skills/ego-browser"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        link("skills/ego-browser"),
        dotfiles,
        "an outside link keeps its target and isn't copied"
    );
    assert_eq!(
        link("skills/key"),
        secret,
        "the secret's contents were not copied into the managed home"
    );
    assert_eq!(
        link("skills/settings-abs"),
        dest.join("settings.json"),
        "an inside link points into the copy"
    );
    assert_eq!(
        link("skills/settings-rel"),
        std::path::PathBuf::from("../settings.json")
    );
    assert_eq!(link("skills/dangling"), work.path().join("gone"));
    assert!(
        !dest.join(".credentials.json").exists(),
        "a linked sign-in is not carried over"
    );
    assert!(
        outcome
            .skipped
            .iter()
            .any(|note| note.contains("skills/ego-browser (kept as a link"))
    );
    // Copy-never-move: the source's links are untouched.
    assert_eq!(
        std::fs::read_link(source.join("skills/ego-browser")).unwrap(),
        dotfiles
    );
}
