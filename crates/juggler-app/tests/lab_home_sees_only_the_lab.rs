//! AJ-E3306: a run on `HOME=<lab>` sees the lab's Claude app profiles and
//! none of this Mac's. The release candidate, started on the A6 lab's
//! HOME without the bridge, offered six of the user's real profiles under
//! Accounts › an App › Find them, beside the lab's own: they were running,
//! and the process table wasn't scoped outside the bridge's lab mode.
//!
//! Each test runs again as a child on `HOME=<lab>`, a lab made under the
//! build's own scratch folder (not the system's temporary one, whose
//! profiles are never remembered). The user's real home is only
//! ever a spelling here: nothing under it is listed or read.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use juggler_app::features::conversations::places::{HostRegistry, running_apps};
use juggler_app::features::conversations::procs::{Proc, Table};
use juggler_app::features::places::book::{self, Book, KnownPlace};
use juggler_app::features::places::profiles;

const CLAUDE: &str = "/Applications/Claude.app/Contents/MacOS/Claude";
const PROFILES: &str = "Library/Application Support/ClaudeProfiles";

const CHILD: &str = "AIJ_TEST_LAB_HOME_CHILD";

/// Runs `test` again as a child on `HOME=<a new lab>` (the home is read
/// once per process, so it can't be changed in this one), the lab holding
/// one Claude app profile folder, `lab-one`. `None` in the parent, which
/// has checked the child passed; in the child, the lab (this `HOME`).
fn in_a_lab(test: &str) -> Option<PathBuf> {
    if std::env::var_os(CHILD).is_some() {
        return Some(PathBuf::from(std::env::var_os("HOME").unwrap()));
    }
    let lab = tempfile::Builder::new()
        .prefix("aij-lab-home-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    std::fs::create_dir_all(lab.path().join(PROFILES).join("lab-one")).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--nocapture"])
        .env(CHILD, "1")
        .env("HOME", lab.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    None
}

fn hosts() -> HostRegistry {
    HostRegistry::new(juggler_adapters::claude_live::host_apps())
}

/// A Claude app main process started on `profile`.
fn claude_on(pid: u32, profile: &Path) -> (u32, Proc) {
    (
        pid,
        Proc {
            pid,
            ppid: Some(1),
            exe: Some(PathBuf::from(CLAUDE)),
            args: vec![
                CLAUDE.to_owned(),
                format!("--user-data-dir={}", profile.display()),
            ],
            start: Some(i64::from(pid)),
        },
    )
}

fn names(dirs: impl IntoIterator<Item = PathBuf>) -> Vec<String> {
    dirs.into_iter()
        .filter_map(|dir| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
}

#[test]
fn the_home_every_part_resolves_is_the_labs() {
    let Some(lab) = in_a_lab("the_home_every_part_resolves_is_the_labs") else {
        return;
    };
    let lab = lab.as_path();
    assert_eq!(juggler_platform::user::home().as_deref(), Some(lab));
    if let Some(real) = juggler_platform::user::record_home() {
        assert_ne!(lab, real.as_path());
        assert_eq!(juggler_platform::user::other_home().as_deref(), Some(lab));
    }
}

/// The running profiles: the lab's is offered, this Mac's never.
#[test]
fn a_lab_home_offers_its_own_running_profile_and_none_of_this_macs() {
    let Some(lab) = in_a_lab("a_lab_home_offers_its_own_running_profile_and_none_of_this_macs")
    else {
        return;
    };
    let lab = lab.as_path();
    let home = juggler_platform::user::home().unwrap();
    // The user's real profiles folder, as a running app names it on its
    // command line (a made-up profile name; nothing there is read).
    let real = juggler_platform::user::record_home()
        .expect("the user record's home")
        .join(PROFILES)
        .join("real-profile");
    let mut table: Table = [
        claude_on(101, &lab.join(PROFILES).join("lab-one")),
        claude_on(202, &real),
    ]
    .into_iter()
    .collect();
    juggler_app::lab_scope::keep(&mut table);
    let hosts = hosts();
    let running = running_apps(&table, &hosts, &home);
    assert_eq!(
        names(running.iter().filter_map(|app| app.instance.clone())),
        ["lab-one"],
        "only the lab's own running profile"
    );
    // The book (what Find them and Settings › Desktop apps offer) holds
    // the lab's profile and nothing of the real one.
    let mut book = Book::default();
    book.observe(&running, &[], &hosts, &home, 1_000);
    let offered: Vec<&KnownPlace> = book.places.values().collect();
    assert_eq!(
        names(
            offered
                .iter()
                .filter_map(|place| place.instance_dir.clone())
        ),
        ["lab-one"]
    );
    assert!(
        offered.iter().all(|place| place
            .instance_dir
            .as_deref()
            .is_some_and(|dir| dir.starts_with(lab))),
        "nothing outside the lab"
    );
}

/// Find them, with the home the app resolves: the profiles beside the
/// lab's are offered, and a folder outside the lab is never listed.
#[test]
fn find_them_on_a_lab_home_offers_exactly_the_labs_profiles() {
    let Some(lab) = in_a_lab("find_them_on_a_lab_home_offers_exactly_the_labs_profiles") else {
        return;
    };
    let lab = lab.as_path();
    let home = juggler_platform::user::home().unwrap();
    std::fs::create_dir_all(lab.join(PROFILES).join("lab-two")).unwrap();
    // A home that isn't this run's (a stand-in for the user's real one,
    // made here so nothing real is listed even if the scan were wrong).
    let elsewhere = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    std::fs::create_dir_all(elsewhere.path().join(PROFILES).join("not-the-lab")).unwrap();
    std::fs::create_dir_all(elsewhere.path().join(PROFILES).join("not-the-lab-2")).unwrap();
    let bundle = PathBuf::from("/Applications/Claude.app");
    let ran = |dir: PathBuf| KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude app".into(),
        instance: dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
        bundle: bundle.clone(),
        exe: bundle.join("Contents/MacOS/Claude"),
        given: Some(dir.clone()),
        instance_dir: Some(dir),
        account_id: None,
        conversations: Vec::new(),
        seen_ms: 1,
        keep_alive: false,
        paused: None,
        found: false,
        users: true,
        declined: false,
    };
    let mut book = Book::default();
    for dir in [
        lab.join(PROFILES).join("lab-one"),
        elsewhere.path().join(PROFILES).join("not-the-lab"),
    ] {
        book.places
            .insert(book::place_id("claude-app", Some(&dir)), ran(dir));
    }
    let hosts = hosts();
    assert_eq!(
        profiles::folders(&book, &hosts, &home)
            .into_iter()
            .map(|folder| folder.folder)
            .collect::<Vec<_>>(),
        [lab.join(PROFILES)],
        "only the lab's profiles folder is read"
    );
    profiles::find(&mut book, &hosts, &home, 2);
    let found: Vec<PathBuf> = book
        .places
        .values()
        .filter(|place| place.found)
        .filter_map(|place| place.instance_dir.clone())
        .collect();
    assert_eq!(found, [lab.join(PROFILES).join("lab-two")]);
}
