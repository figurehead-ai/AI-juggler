//! AJ-E2327: Add on a desktop profile brings in the tool folder its
//! sessions use, by evidence (the shared ownership module), so they show
//! at once, from a fresh data folder (no Accounts) like the 0.5.0 install
//! the maintainer walked. Placeholder data only: `/Users/you`-shaped
//! folders in a temporary home, profiles `work`, `team-a` and `lab`, and
//! Claude Code folders whose names differ from them.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::*;
use crate::features::conversations::procs::fixture as proc;

const APP: &str = "/Applications/Claude.app";

/// A made-up Mac: a fresh data folder, the Claude Code folders discovery
/// finds (`ClaudeCodeProfiles/<name>`), and the processes running.
struct Mac {
    _root: tempfile::TempDir,
    home: PathBuf,
    state: Arc<AppState>,
}

impl Mac {
    /// Folders `folders` under `ClaudeCodeProfiles`, each a Claude Code
    /// folder (settings and a projects folder); `table` (given the home)
    /// running.
    fn new(folders: &[&str], table: impl FnOnce(&Path) -> Vec<(u32, procs::Proc)>) -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let installs = folders
            .iter()
            .map(|name| {
                let path = home
                    .join("Library/Application Support/ClaudeCodeProfiles")
                    .join(name);
                std::fs::create_dir_all(path.join("projects/-Users-you-dev")).unwrap();
                std::fs::create_dir_all(path.join("sessions")).unwrap();
                std::fs::write(path.join("settings.json"), "{}").unwrap();
                super::fakes::install(path)
            })
            .collect();
        let state = AppState::init(root.path().join("data")).unwrap();
        let _ = state.places.made_up.set(MadeUpMac {
            table: table(&home).into_iter().collect(),
            home: home.clone(),
            installs: Some(installs),
        });
        Self {
            _root: root,
            home,
            state: Arc::new(state),
        }
    }

    fn folder(&self, name: &str) -> PathBuf {
        self.home
            .join("Library/Application Support/ClaudeCodeProfiles")
            .join(name)
    }

    /// Profile `name`'s instance folder (the app's `ClaudeProfiles/<name>`).
    fn profile(&self, name: &str) -> PathBuf {
        profile_of(&self.home, name)
    }

    /// Profile `name` offered (seen running or beside one; not added).
    fn offer(&self, name: &str) -> String {
        let dir = self.profile(name);
        std::fs::create_dir_all(&dir).unwrap();
        let mut place = crate::test_support::claude_profile(&dir.to_string_lossy());
        place.instance = Some(name.to_owned());
        place.users = false;
        let id = book::place_id("claude-app", Some(&dir));
        with_book(&self.state, |book| book.places.insert(id.clone(), place)).unwrap();
        id
    }

    /// Its app's records of sessions `ids` (`claude-code-sessions`).
    fn records(&self, name: &str, ids: &[&str]) {
        let records = self.profile(name).join("claude-code-sessions/acct/org");
        std::fs::create_dir_all(&records).unwrap();
        for (at, id) in ids.iter().enumerate() {
            std::fs::write(
                records.join(format!("local_{id}.json")),
                format!(r#"{{"sessionId":"local_{id}","cliSessionId":"{id}","title":"T","lastActivityAt":{at}}}"#),
            )
            .unwrap();
        }
    }

    /// A transcript of session `id` in folder `name`.
    fn transcript(&self, name: &str, id: &str) {
        std::fs::write(
            self.folder(name)
                .join(format!("projects/-Users-you-dev/{id}.jsonl")),
            "{}\n",
        )
        .unwrap();
    }

    fn account_dir(&self, id: &str) -> Option<PathBuf> {
        let place = with_book(&self.state, |book| book.places.get(id).cloned()).unwrap()?;
        tool_folder(&self.state, &place)
    }
}

fn profile_of(home: &Path, name: &str) -> PathBuf {
    home.join("Library/Application Support/ClaudeProfiles")
        .join(name)
}

/// Profile `name` running as main process `main`, its Code tab's Claude
/// Code (`main + 4`, under the app's `disclaimer` wrapper) writing its pid
/// file in `folder`.
fn running(home: &Path, name: &str, main: u32) -> Vec<(u32, procs::Proc)> {
    let profile = profile_of(home, name);
    let exe = format!("{APP}/Contents/MacOS/Claude");
    let wrapper = format!("{APP}/Contents/Helpers/disclaimer");
    let code = profile.join("claude-code/2.1/abc/claude.app/Contents/MacOS/claude");
    let code = code.to_string_lossy().into_owned();
    let flag = format!("--user-data-dir={}", profile.display());
    vec![
        proc(main, 1, &exe, &[&exe, &flag]),
        proc(
            main + 3,
            main,
            &wrapper,
            &[&wrapper, "--pgroup", "--", &code],
        ),
        proc(main + 4, main + 3, &code, &[&code]),
    ]
}

/// The pid file the Code child (`pid`, started at `pid * 10` s) writes
/// in the folder it was given.
fn pid_file(folder: &Path, pid: u32, session: &str) {
    let start = i64::from(pid) * 10;
    let at = chrono_free_utc(start);
    std::fs::write(
        folder.join(format!("sessions/{pid}.json")),
        format!(
            r#"{{"pid":{pid},"sessionId":"{session}","procStart":"{at}","kind":"interactive"}}"#
        ),
    )
    .unwrap();
}

/// `secs` after the epoch as `1970-01-01THH:MM:SSZ` (under a day).
fn chrono_free_utc(secs: i64) -> String {
    format!(
        "1970-01-01T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// The fresh 0.5.0 install: no Accounts; profiles `team-a` and `lab` run
/// with live Code sessions in `client-a` and `acme-north` (names that
/// aren't theirs); a decoy `team-a` folder exists. Add links each to the
/// folder its Code child writes in, brought in where it is, and its
/// sessions are listed at once. 0.5.0 took the decoy by its name for
/// `team-a`, and nothing for `lab`.
#[test]
fn add_on_a_running_profile_links_the_folder_its_sessions_write_in() {
    let mac = Mac::new(&["team-a", "client-a", "acme-north"], |home| {
        let mut all = running(home, "team-a", 100);
        all.extend(running(home, "lab", 200));
        all
    });
    pid_file(&mac.folder("client-a"), 104, "s-1");
    pid_file(&mac.folder("acme-north"), 204, "s-7");
    mac.records("team-a", &["s-1", "s-2"]);
    mac.transcript("client-a", "s-1");
    mac.transcript("client-a", "s-2");
    let (team, lab) = (mac.offer("team-a"), mac.offer("lab"));
    assert!(
        crate::commands::lock_storage_arc(&mac.state)
            .unwrap()
            .accounts()
            .list()
            .unwrap()
            .is_empty(),
        "a fresh data folder"
    );

    let added = answer_offer(&mac.state, &team, true).unwrap();
    assert_eq!(
        added.folder.as_deref(),
        Some(mac.folder("client-a").as_path())
    );
    assert_eq!(added.by, Some(ownership::sessions::LinkSource::Running));
    assert_eq!(
        mac.account_dir(&team),
        Some(mac.folder("client-a")),
        "brought in where it is"
    );
    let added = answer_offer(&mac.state, &lab, true).unwrap();
    assert_eq!(
        added.folder.as_deref(),
        Some(mac.folder("acme-north").as_path())
    );
    assert_eq!(mac.account_dir(&lab), Some(mac.folder("acme-north")));

    // Nothing copied: the Account is the folder itself.
    let accounts = crate::commands::lock_storage_arc(&mac.state)
        .unwrap()
        .accounts()
        .list()
        .unwrap();
    assert_eq!(accounts.len(), 2);
    assert!(accounts.iter().all(|account| account.in_place));
    // Its sessions are listed at once: its records read for its folder.
    let folder = with_book(&mac.state, |book| book.places[&team].account_id.clone())
        .unwrap()
        .unwrap();
    let index = record_index(&mac.state);
    assert!(index.listed(&folder, "claude-code/s-1") && index.listed(&folder, "claude-code/s-2"));
    // Its row says which folder and what told.
    let row = commands::known(&mac.state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == team)
        .unwrap();
    let said = row.folder.unwrap();
    assert_eq!(said.name.as_deref(), Some("client-a"));
    assert_eq!(said.by.as_deref(), Some("running"));
    assert_eq!(said.tool.as_deref(), Some("Claude Code"));
}

/// A profile added while it isn't running is linked by its own records:
/// the folder holding its sessions' transcripts (named unlike it), its
/// sessions listed. The code before AJ-E2327 waited for it to run.
#[test]
fn a_profile_added_while_closed_is_linked_by_its_records() {
    let mac = Mac::new(&["team-a", "client-a", "work"], |_| Vec::new());
    mac.records("team-a", &["s-1", "s-2", "s-3"]);
    for id in ["s-1", "s-2", "s-3"] {
        mac.transcript("client-a", id);
    }
    // A switch's copy of one in another folder doesn't outweigh them.
    mac.transcript("work", "s-1");
    let team = mac.offer("team-a");
    let added = answer_offer(&mac.state, &team, true).unwrap();
    assert_eq!(added.by, Some(ownership::sessions::LinkSource::Records));
    assert_eq!(mac.account_dir(&team), Some(mac.folder("client-a")));
    let folder = with_book(&mac.state, |book| book.places[&team].account_id.clone())
        .unwrap()
        .unwrap();
    assert!(record_index(&mac.state).listed(&folder, "claude-code/s-3"));
}

/// Restarting a linked profile starts it with its folder: the app reads
/// `CLAUDE_CONFIG_DIR`, so its sessions stay in the folder they're in.
#[test]
fn restarting_it_starts_it_with_its_folder() {
    let mac = Mac::new(&["client-a"], |_| Vec::new());
    mac.records("team-a", &["s-1"]);
    mac.transcript("client-a", "s-1");
    let team = mac.offer("team-a");
    answer_offer(&mac.state, &team, true).unwrap();
    let place = with_book(&mac.state, |book| book.places[&team].clone()).unwrap();
    let hosts = HostRegistry::of(&mac.state.registry);
    let launch = start::plan(
        &place,
        &hosts,
        tool_folder(&mac.state, &place).as_deref(),
        false,
    )
    .unwrap();
    let folder = mac.folder("client-a").to_string_lossy().into_owned();
    assert!(
        launch
            .env
            .contains(&("CLAUDE_CONFIG_DIR".to_owned(), folder)),
        "{:?}",
        launch.env
    );
}

/// No evidence (closed, its app lists no session): Add adds it and says
/// why no folder came with it; its row says so too, never a silent grey
/// row, and Choose its folder… links the one the user picks. A guess by
/// name links only when nothing else can tell, and evidence corrects it.
#[test]
fn no_evidence_says_why_and_choose_its_folder_links_it() {
    let mac = Mac::new(&["client-a"], |_| Vec::new());
    let team = mac.offer("team-a");
    let added = answer_offer(&mac.state, &team, true).unwrap();
    assert_eq!(added.folder, None);
    let why = added.why.unwrap();
    assert!(
        why.contains("can't tell which Claude Code folder it uses"),
        "{why}"
    );
    let row = commands::known(&mac.state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == team)
        .unwrap();
    assert_eq!(row.account_id, None);
    assert_eq!(row.folder.and_then(|folder| folder.why), Some(why));

    let chosen = link::choose_folder(&mac.state, &team, &mac.folder("client-a")).unwrap();
    assert_eq!(chosen.by, Some(ownership::sessions::LinkSource::Recorded));
    assert_eq!(mac.account_dir(&team), Some(mac.folder("client-a")));
    // A folder that isn't a Claude Code one is refused.
    let empty = mac.home.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(link::choose_folder(&mac.state, &team, &empty).is_err());
}

/// The name is a fallback only: with no other evidence the folder named
/// like the profile links (so a new profile's first sessions show), and
/// once its records say otherwise the link follows them.
#[test]
fn a_name_guess_is_corrected_by_evidence() {
    let mac = Mac::new(&["work", "client-b"], |_| Vec::new());
    let work = mac.offer("work");
    let added = answer_offer(&mac.state, &work, true).unwrap();
    assert_eq!(added.by, Some(ownership::sessions::LinkSource::Named));
    assert_eq!(mac.account_dir(&work), Some(mac.folder("work")));
    // Its app records sessions that are in `client-b`: corrected.
    mac.records("work", &["s-1", "s-2"]);
    mac.transcript("client-b", "s-1");
    mac.transcript("client-b", "s-2");
    mac.state.places.link_retry.lock().unwrap().clear();
    assert!(link::link_its_folder(&mac.state, &work, false));
    assert_eq!(mac.account_dir(&work), Some(mac.folder("client-b")));
    assert!(link::proven(&mac.state, &work));
}

/// For AJ-E2317: evidence that contradicts a link is said (the Move
/// refuses on it), a link the evidence agrees with isn't.
#[test]
fn a_mismatch_between_the_link_and_the_evidence_is_said() {
    let mac = Mac::new(&["client-a", "work"], |home| running(home, "team-a", 100));
    pid_file(&mac.folder("client-a"), 104, "s-1");
    let team = mac.offer("team-a");
    answer_offer(&mac.state, &team, true).unwrap();
    assert_eq!(link::folder_mismatch(&mac.state, &team), None);
    // Linked by hand to another folder (an Account of its own).
    let other = crate::features::account::import::account_import_mode_impl(
        &mac.state,
        "claude-code",
        &mac.folder("work").to_string_lossy(),
        "work",
        crate::features::account::import::ImportRequest {
            in_place: true,
            ..Default::default()
        },
    )
    .unwrap();
    with_book(&mac.state, |book| {
        book.places.get_mut(&team).unwrap().account_id = Some(other.account.id.as_str().to_owned());
    })
    .unwrap();
    let (folder, by) = link::folder_mismatch(&mac.state, &team).unwrap();
    assert_eq!(folder, mac.folder("client-a"));
    assert_eq!(by, ownership::sessions::LinkSource::Running);
}

/// First run's "Your apps": each profile paired with its folder by
/// evidence, and whether that's certain, without adding anything.
#[test]
fn pairings_pair_each_profile_by_evidence_and_add_nothing() {
    let mac = Mac::new(&["team-a", "client-a", "lab"], |home| {
        running(home, "team-a", 100)
    });
    pid_file(&mac.folder("client-a"), 104, "s-1");
    let team = mac.offer("team-a");
    let lab = mac.offer("lab");
    let all = commands::pairings(&mac.state).unwrap();
    let of = |id: &str| all.iter().find(|one| one.id == id).unwrap().clone();
    assert_eq!(of(&team).folder.name.as_deref(), Some("client-a"));
    assert!(of(&team).certain && !of(&team).added);
    assert_eq!(
        of(&lab).folder.name.as_deref(),
        Some("lab"),
        "by its name only"
    );
    assert!(!of(&lab).certain);
    assert!(
        crate::commands::lock_storage_arc(&mac.state)
            .unwrap()
            .accounts()
            .list()
            .unwrap()
            .is_empty()
    );
}

/// 0.5.1 F2: in "Your apps" a profile not yet added takes "Choose its
/// folder…" too. The pick is checked as an App's is, kept with the offer
/// (nothing added, brought in or linked: D-84), pairs it for sure, and
/// its Add links that folder.
#[test]
fn a_folder_chosen_before_add_is_kept_and_linked_by_add() {
    let mac = Mac::new(&["client-a", "team-a"], |_| Vec::new());
    let team = mac.offer("team-a");
    let accounts = |mac: &Mac| {
        crate::commands::lock_storage_arc(&mac.state)
            .unwrap()
            .accounts()
            .list()
            .unwrap()
            .len()
    };
    // Guessed by its name, so Your apps asks.
    let all = commands::pairings(&mac.state).unwrap();
    let row = all.iter().find(|one| one.id == team).unwrap();
    assert_eq!(row.folder.name.as_deref(), Some("team-a"));
    assert!(!row.certain && !row.added);

    // A folder that isn't a Claude Code one is refused, as for an App.
    let empty = mac.home.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(link::choose_folder(&mac.state, &team, &empty).is_err());

    let chosen = link::choose_folder(&mac.state, &team, &mac.folder("client-a")).unwrap();
    assert_eq!(chosen.folder, Some(mac.folder("client-a")));
    assert_eq!(chosen.by, Some(ownership::sessions::LinkSource::Recorded));
    // Kept with the offer only: not added, nothing brought in or linked.
    assert_eq!(accounts(&mac), 0);
    assert_eq!(mac.account_dir(&team), None);
    let all = commands::pairings(&mac.state).unwrap();
    let row = all.iter().find(|one| one.id == team).unwrap();
    assert_eq!(row.folder.name.as_deref(), Some("client-a"));
    assert!(row.certain, "the user's pick settles the pairing");
    assert!(!row.added);

    // Their Add links the chosen folder, brought in where it is.
    let added = answer_offer(&mac.state, &team, true).unwrap();
    assert_eq!(added.folder, Some(mac.folder("client-a")));
    assert_eq!(added.by, Some(ownership::sessions::LinkSource::Recorded));
    assert_eq!(mac.account_dir(&team), Some(mac.folder("client-a")));
    assert_eq!(accounts(&mac), 1);
}

/// Proof outranks a pick made before Add: its running sessions in another
/// folder refuse the pick, as they do for an App.
#[test]
fn proof_refuses_a_pick_before_add() {
    let mac = Mac::new(&["client-a", "work"], |home| running(home, "team-a", 100));
    pid_file(&mac.folder("client-a"), 104, "s-1");
    let team = mac.offer("team-a");
    let why = link::choose_folder(&mac.state, &team, &mac.folder("work")).unwrap_err();
    assert!(why.contains("client-a"), "{why}");
}

/// A declined offer ("Not now") takes no pick.
#[test]
fn a_declined_offer_takes_no_pick() {
    let mac = Mac::new(&["client-a"], |_| Vec::new());
    let team = mac.offer("team-a");
    answer_offer(&mac.state, &team, false).unwrap();
    assert!(link::choose_folder(&mac.state, &team, &mac.folder("client-a")).is_err());
}
