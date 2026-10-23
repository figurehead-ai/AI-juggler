//! AJ-E1540: a resumed or primed session runs where its work is.
//!
//! A switch resumes the conversation on the target account in the source's
//! folder. The target's launch settings may name a default folder for
//! fresh sessions; it took precedence for every launch, so a switched
//! session could resume somewhere else than its conversation and files.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

use juggler_adapter::{HarnessAdapter, LaunchMode};
use juggler_adapters::{ClaudeCodeAdapter, CodexAdapter};
use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};

fn account(harness: &str, dir: &Path) -> Account {
    Account {
        id: AccountId::new("b"),
        harness: HarnessId::new(harness),
        label: "B".into(),
        plan: None,
        state_dir: dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

fn profile(default_cwd: &Path) -> Profile {
    Profile {
        id: ProfileId::new("b-main"),
        name: "Main".into(),
        account: AccountId::new("b"),
        settings_overlay: SettingsOverlay::default(),
        default_cwd: Some(default_cwd.to_path_buf()),
    }
}

fn check(adapter: &dyn HarnessAdapter, harness: &str) {
    let work = tempfile::tempdir().unwrap();
    let home = work.path().join("home");
    let elsewhere: PathBuf = work.path().join("usual-folder");
    let project: PathBuf = work.path().join("the-project");
    for dir in [&home, &elsewhere, &project] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let account = account(harness, &home);
    let profile = profile(&elsewhere);
    let resume = adapter
        .launch_spec(
            &account,
            &profile,
            &project,
            &LaunchMode::Resume {
                session_id: "s-1".into(),
                delta_prompt: None,
            },
        )
        .unwrap();
    assert_eq!(
        resume.cwd, project,
        "{harness}: a resume runs where its work is"
    );
    let primed = adapter
        .launch_spec(
            &account,
            &profile,
            &project,
            &LaunchMode::Primed {
                prompt: "carry on".into(),
                read_dirs: Vec::new(),
            },
        )
        .unwrap();
    assert_eq!(
        primed.cwd, project,
        "{harness}: a replay runs where its work is"
    );
    let fresh = adapter
        .launch_spec(&account, &profile, &project, &LaunchMode::Interactive)
        .unwrap();
    assert_eq!(
        fresh.cwd, elsewhere,
        "{harness}: a fresh session starts in the usual folder"
    );
}

/// A stand-in program, so the adapters resolve a binary without a real tool.
fn stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("tool");
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn claude_resumes_and_replays_in_the_work_folder() {
    let bin = tempfile::tempdir().unwrap();
    check(
        &ClaudeCodeAdapter::with_bin(stub(bin.path())),
        "claude-code",
    );
}

#[test]
fn codex_resumes_and_replays_in_the_work_folder() {
    let bin = tempfile::tempdir().unwrap();
    check(&CodexAdapter::with_bin(stub(bin.path())), "codex");
}
