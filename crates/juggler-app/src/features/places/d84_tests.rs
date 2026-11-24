//! D-84 (2026-11-06): AI Juggler never adds an app by itself. One seen
//! running that the user never added is offered ("Found on this computer"),
//! never listed, opened or kept alive; Add makes it an App, Not now
//! hides the offer. A profile under the system's temporary folder is
//! never remembered at all, and a reused pid is never the app.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::*;
use crate::features::conversations::places::RunningApp;
use crate::features::conversations::procs::Proc;

/// A Claude profile running in `dir` (as pid 4242, never signalled).
fn claude_in(root: &Path, dir: &Path) -> RunningApp {
    let bundle = root.join("Claude.app");
    std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    RunningApp {
        app_id: "claude-app".into(),
        exe: bundle.join("Contents/MacOS/Claude"),
        bundle,
        pid: 4242,
        start: Some(1_000),
        given: Some(dir.to_path_buf()),
        instance: Some(dir.to_path_buf()),
    }
}

/// A profile folder outside the system's temporary folder (those are
/// never remembered), removed when the test ends.
/// A profile folder outside the system's temporary folders (the app
/// skips places there): the crate's folder, or the build's target folder
/// when the checkout itself is temporary (the preflight's copy).
fn profile_folder() -> tempfile::TempDir {
    let base = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .filter(|dir| {
            !dir.to_str()
                .is_some_and(juggler_core::storage::in_temporary_folder)
        })
        .filter(|_| juggler_core::storage::in_temporary_folder(env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    std::fs::create_dir_all(&base).unwrap();
    tempfile::Builder::new()
        .prefix(".d84-profile-")
        .tempdir_in(base)
        .unwrap()
}

fn state(root: &Path) -> Arc<AppState> {
    super::fakes::on_made_up_mac(AppState::init(root.join("data")).unwrap(), root)
}

fn listed(state: &Arc<AppState>) -> (Vec<String>, Vec<String>) {
    let places = commands::known(state)
        .unwrap()
        .into_iter()
        .map(|place| place.id)
        .collect();
    let apps = crate::features::apps::list_impl(state)
        .unwrap()
        .into_iter()
        .map(|app| app.id)
        .collect();
    (places, apps)
}

#[test]
fn a_profile_seen_running_but_never_added_is_only_offered() {
    let root = tempfile::tempdir().unwrap();
    let folder = profile_folder();
    let state = state(root.path());
    observe(&state, &[claude_in(root.path(), folder.path())], &[]);
    let id = book::place_id("claude-app", Some(folder.path()));

    let (places, apps) = listed(&state);
    assert!(!places.contains(&id), "not a place: {places:?}");
    assert!(!apps.contains(&id), "not an App: {apps:?}");
    let offers = offers(&state).unwrap();
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert_eq!(offers[0].0, id);
    // Nothing acts on an offer.
    assert!(set_keep_alive(&state, &id, true).is_err());
    let refused = open(
        &state,
        &id,
        None,
        Arc::new(super::fakes::FakeDesktop::default()),
        Arc::new(super::fakes::Recorder::default()),
        None,
    )
    .unwrap_err();
    assert!(refused.to_string().contains("add it first"), "{refused}");

    // Add: it's the user's App now, and no longer offered.
    answer_offer(&state, &id, true).unwrap();
    let (places, apps) = listed(&state);
    assert!(places.contains(&id) && apps.contains(&id));
    assert!(offers_of(&state).is_empty());
}

/// AJ-E1552 (D-84): an Account's Apps (`subscriptions::with_apps`), the
/// Apps list (`apps::users_apps`) and the desktop Apps the UI draws (the
/// places list) read one rule, `App::is_users`. A profile seen running
/// whose sessions' folder is linked to the Account, but never added, is
/// on none of them (only offered); one the user added is on all three,
/// now and after a restart alike. So an Account never names an App no
/// list shows (the card that waited for ever on its model).
#[test]
fn an_accounts_apps_are_listed_apps_and_a_linked_offer_is_on_none() {
    use crate::features::subscriptions::{ProfileRead, SubscriptionProfileInfo, group, with_apps};
    let root = tempfile::tempdir().unwrap();
    let (added_dir, offer_dir) = (profile_folder(), profile_folder());
    let state = state(root.path());
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&crate::test_support::account(
            "work",
            "claude-code",
            "Work",
            root.path().join("work"),
        ))
        .unwrap();
    observe(
        &state,
        &[
            claude_in(root.path(), added_dir.path()),
            claude_in(root.path(), offer_dir.path()),
        ],
        &[],
    );
    let added = book::place_id("claude-app", Some(added_dir.path()));
    let offer = book::place_id("claude-app", Some(offer_dir.path()));
    answer_offer(&state, &added, true).unwrap();
    // Both profiles' sessions write in the Account's folder (the link
    // `link::link_to` makes by evidence), and both are on its sign-in.
    with_book(&state, |book| {
        for id in [&added, &offer] {
            book.places.get_mut(id).unwrap().account_id = Some("work".into());
        }
    })
    .unwrap();
    let mut accounts = group(vec![ProfileRead {
        profile: SubscriptionProfileInfo {
            id: "work".into(),
            label: "Work".into(),
            folder: "~/work".into(),
            plan: None,
            renewal_date: None,
            fee_usd: None,
            unchecked: None,
            accepted_name: None,
        },
        tool: "claude-code".into(),
        probe: juggler_adapter::IdentityProbe::SignedIn {
            subject: "u-alice".into(),
            label: "alice@example.com".into(),
            org: None,
        },
        service: None,
    }]);
    // The Account's id as `subscriptions::list_impl` stores it.
    let account = "acct-alice".to_owned();
    accounts[0].account_id = Some(account.clone());
    {
        let storage = state.storage.lock().unwrap();
        for id in [&added, &offer] {
            let row = storage.apps().get(id).unwrap().unwrap();
            assert_eq!(row.folder_id.as_deref(), Some("work"), "{id}: linked");
            storage.apps().set_account(id, Some(&account)).unwrap();
        }
    }
    for when in ["now", "after a restart"] {
        if when == "after a restart" {
            *state.places.book.lock().unwrap() = None;
        }
        let stored = crate::features::apps::list_impl(&state).unwrap();
        with_apps(&mut accounts, &stored);
        let named_apps = accounts[0].apps.clone().unwrap();
        let (places, apps) = listed(&state);
        assert!(
            named_apps.iter().all(|id| apps.contains(id)),
            "{when}: every App the Account names is listed ({} of them, {} listed)",
            named_apps.len(),
            apps.len()
        );
        let desktop: std::collections::BTreeSet<&String> =
            apps.iter().filter(|id| !id.starts_with("cli:")).collect();
        assert_eq!(
            desktop,
            places.iter().collect(),
            "{when}: the Apps list and the places list are one set of desktop Apps"
        );
        assert!(
            named_apps.contains(&added) && apps.contains(&added) && places.contains(&added),
            "{when}: the added profile is on all three"
        );
        assert!(
            !named_apps.contains(&offer) && !apps.contains(&offer) && !places.contains(&offer),
            "{when}: a linked offer is on none"
        );
        assert!(
            offers_of(&state).iter().any(|(id, _)| *id == offer),
            "{when}: it's still offered"
        );
    }
}

#[test]
fn not_now_hides_the_offer_and_adds_nothing() {
    let root = tempfile::tempdir().unwrap();
    let folder = profile_folder();
    let state = state(root.path());
    let running = [claude_in(root.path(), folder.path())];
    observe(&state, &running, &[]);
    let id = book::place_id("claude-app", Some(folder.path()));
    answer_offer(&state, &id, false).unwrap();
    assert!(offers_of(&state).is_empty());
    // Seen running again: still declined, still no App.
    observe(&state, &running, &[]);
    assert!(offers_of(&state).is_empty());
    let (places, apps) = listed(&state);
    assert!(!places.contains(&id) && !apps.contains(&id));
}

#[test]
fn an_offer_whose_folder_is_gone_is_forgotten() {
    let root = tempfile::tempdir().unwrap();
    let folder = profile_folder();
    let state = state(root.path());
    observe(&state, &[claude_in(root.path(), folder.path())], &[]);
    let gone = folder.path().to_path_buf();
    drop(folder);
    assert!(!gone.exists());
    assert!(offers_of(&state).is_empty());
}

#[test]
fn a_profile_in_the_temporary_folder_is_never_remembered() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let temp = root.path().join("Lab Claude profile");
    std::fs::create_dir_all(&temp).unwrap();
    assert!(juggler_core::storage::in_temporary_folder(
        temp.to_str().unwrap()
    ));
    observe(&state, &[claude_in(root.path(), &temp)], &[]);
    assert!(offers_of(&state).is_empty());
    assert!(with_book(&state, |book| book.places.is_empty()).unwrap());
}

/// F18 (0.5.1 PL1): a run whose home is under the temporary folder (a
/// lab's HOME, `/tmp/aij-lab…`) found none of its own running profiles:
/// every one is under `/tmp` too, so the rule above dropped them, and the
/// wizard, the rail's "Running, not added" and Settings had nothing to
/// offer. A profile inside the run's own home is its user's: offered
/// (D-84: never added by itself). One outside it still isn't remembered.
#[test]
fn a_profile_inside_a_temporary_home_is_offered() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let home = state.places.home();
    let profiles = home.join("Library/Application Support/ClaudeProfiles");
    let work = profiles.join("work");
    let personal = profiles.join("personal");
    let outside = root.path().join("Lab Claude profile");
    for dir in [&work, &personal, &outside] {
        std::fs::create_dir_all(dir).unwrap();
    }
    assert!(juggler_core::storage::in_temporary_folder(
        home.to_str().unwrap()
    ));
    let at = |dir: &Path, pid: u32| RunningApp {
        pid,
        ..claude_in(root.path(), dir)
    };
    observe(
        &state,
        &[at(&work, 4242), at(&personal, 4243), at(&outside, 4244)],
        &[],
    );
    let mut ids: Vec<String> = offers_of(&state).into_iter().map(|(id, _)| id).collect();
    ids.sort();
    let mut expected = vec![
        book::place_id("claude-app", Some(&personal)),
        book::place_id("claude-app", Some(&work)),
    ];
    expected.sort();
    assert_eq!(ids, expected, "both lab profiles, not the one outside home");
    let (places, apps) = listed(&state);
    assert!(places.is_empty() && apps.is_empty(), "offered, never added");
}

/// F19 (0.5.1 PL1 D2–D6): Remove (Forget) of a running App brings it
/// back as an offer at the next look, so every Add (the rail's "Running,
/// not added", Settings › Desktop apps' Found on this computer, the
/// wizard's rows, ⌘K's Find my apps) can add it again.
#[test]
fn a_removed_running_profile_is_offered_again() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let work = state
        .places
        .home()
        .join("Library/Application Support/ClaudeProfiles/work");
    std::fs::create_dir_all(&work).unwrap();
    let running = [claude_in(root.path(), &work)];
    observe(&state, &running, &[]);
    let id = book::place_id("claude-app", Some(&work));
    answer_offer(&state, &id, true).unwrap();
    assert!(listed(&state).1.contains(&id), "added");
    assert!(offers_of(&state).is_empty());

    forget(&state, &id).unwrap();
    assert!(!listed(&state).1.contains(&id), "removed");
    observe(&state, &running, &[]);
    let offered: Vec<String> = offers_of(&state).into_iter().map(|(id, _)| id).collect();
    assert_eq!(
        offered,
        std::slice::from_ref(&id),
        "offered again while it runs"
    );
    answer_offer(&state, &id, true).unwrap();
    assert!(listed(&state).1.contains(&id), "added again");
}

#[test]
fn temporary_not_home_spares_only_the_runs_own_home() {
    let lab = Path::new("/tmp/aij-lab-x");
    let profile = Path::new("/tmp/aij-lab-x/Library/Application Support/ClaudeProfiles/work");
    assert!(!book::temporary_not_home(profile, lab));
    // Spelled through /private, either way round.
    assert!(!book::temporary_not_home(
        Path::new("/private/tmp/aij-lab-x/p"),
        Path::new("/private/tmp/aij-lab-x")
    ));
    // Another lab's (`aij-lab-x1` isn't inside `aij-lab-x`), an installer's.
    assert!(book::temporary_not_home(
        Path::new("/tmp/aij-lab-x1/p"),
        lab
    ));
    assert!(book::temporary_not_home(
        Path::new("/private/var/folders/ab/T/installer/p"),
        lab
    ));
    // The user's own home: nothing under /tmp is theirs.
    let home = Path::new("/Users/someone");
    assert!(book::temporary_not_home(profile, home));
    assert!(!book::temporary_not_home(
        Path::new("/Users/someone/Library/Application Support/Claude"),
        home
    ));
    assert!(book::temporary_not_home(profile, Path::new("")));
}

#[test]
fn a_reused_pid_is_not_the_app() {
    let exe = PathBuf::from("/Applications/Claude.app/Contents/MacOS/Claude");
    let process = |exe: &str, start: Option<i64>| Proc {
        pid: 4242,
        ppid: None,
        exe: Some(PathBuf::from(exe)),
        args: Vec::new(),
        start,
    };
    let same = process(exe.to_str().unwrap(), Some(1_000));
    assert!(is_its_process(&exe, Some(1_000), &same));
    assert!(is_its_process(&exe, Some(1_001), &same), "to the second");
    assert!(!is_its_process(&exe, Some(2_000), &same), "started later");
    assert!(!is_its_process(&exe, None, &same), "no start recorded");
    assert!(
        !is_its_process(&exe, Some(1_000), &process("/bin/zsh", Some(1_000))),
        "another program"
    );
    assert!(!is_its_process(
        &exe,
        Some(1_000),
        &process(exe.to_str().unwrap(), None)
    ));
}

fn offers_of(state: &Arc<AppState>) -> Vec<(String, String)> {
    offers(state).unwrap()
}

#[test]
fn a_session_started_in_a_closed_profile_is_that_profiles() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let place = |dir: &str, account: Option<&str>, users: bool| book::KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude app".into(),
        instance: Some(dir.into()),
        bundle: PathBuf::new(),
        exe: PathBuf::new(),
        given: None,
        instance_dir: Some(PathBuf::from("/u").join(dir)),
        account_id: account.map(str::to_owned),
        conversations: Vec::new(),
        seen_ms: 1,
        keep_alive: false,
        paused: None,
        found: false,
        users,
        declined: false,
    };
    with_book(&state, |book| {
        book.places
            .insert("south".into(), place("b-south", Some("f-south"), true));
        book.places
            .insert("twin-1".into(), place("b-twin", Some("f-twin"), true));
        book.places
            .insert("twin-2".into(), place("b-twin2", Some("f-twin"), true));
        book.places
            .insert("offer".into(), place("b-offer", Some("f-offer"), false));
        assert_eq!(
            book.users_place_on("claude-app", "f-south"),
            Some(&"south".to_owned())
        );
        assert_eq!(
            book.users_place_on("codex-app", "f-south"),
            None,
            "another app"
        );
        assert_eq!(
            book.users_place_on("claude-app", "f-twin"),
            None,
            "which one isn't known"
        );
        assert_eq!(
            book.users_place_on("claude-app", "f-offer"),
            None,
            "an offer is no App"
        );
    })
    .unwrap();
}

/// The go sitting (2026-11-07, finding 1), AJ-E2315 (the review's E7): Add
/// on a profile brings in the folder its sessions are written to, by
/// evidence: the one whose pid files name its running sessions under its
/// own main process. Profile `team-a`'s Code tab writes in
/// `ClaudeCodeProfiles/work` (its launcher's folder); the old rule took the
/// folder named like the profile, where its sessions never were.
#[test]
fn an_added_profiles_folder_is_the_one_its_sessions_write_in() {
    use crate::features::conversations::procs::fixture as proc;
    let root = tempfile::tempdir().unwrap();
    let mk = |name: &str| {
        let path = root.path().join("ClaudeCodeProfiles").join(name);
        std::fs::create_dir_all(path.join("sessions")).unwrap();
        super::fakes::install(path)
    };
    let found = vec![mk("team-a"), mk("work")];
    let profile = root.path().join("ClaudeProfiles/team-a");
    let code = profile.join("claude-code/2.1/abc/claude.app/Contents/MacOS/claude");
    let code = code.to_string_lossy();
    let main = "/Applications/Claude.app/Contents/MacOS/Claude";
    let d = "/Applications/Claude.app/Contents/Helpers/disclaimer";
    let flag = format!("--user-data-dir={}", profile.display());
    // The Code tab's Claude Code (14, started at 140 s) writes its pid file.
    std::fs::write(
        found[1].path.join("sessions/14.json"),
        r#"{"pid":14,"sessionId":"s-1","procStart":"1970-01-01T00:02:20Z","kind":"interactive"}"#,
    )
    .unwrap();
    let state = AppState::init(root.path().join("data")).unwrap();
    let _ = state.places.made_up.set(MadeUpMac {
        table: [
            proc(10, 1, main, &[main, &flag]),
            proc(13, 10, d, &[d, "--pgroup", "--", &code]),
            proc(14, 13, &code, &[&code]),
        ]
        .into_iter()
        .collect(),
        home: root.path().join("home"),
        installs: None,
    });
    let state = Arc::new(state);
    let mut place = crate::test_support::claude_profile(&profile.to_string_lossy());
    place.instance = Some("team-a".into());
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new("claude-code"))
        .unwrap();
    let host = adapter.host_apps().remove(0);
    let picked = link::folder_of_its_sessions(&state, &adapter, &host, &place, &found).unwrap();
    assert!(
        picked.path.ends_with("ClaudeCodeProfiles/work"),
        "{picked:?}"
    );
    // With its session gone, nothing tells: never the folder named like it.
    std::fs::remove_file(found[1].path.join("sessions/14.json")).unwrap();
    assert!(link::folder_of_its_sessions(&state, &adapter, &host, &place, &found).is_none());
}

/// The go sitting's rerun (2026-11-07): an added profile's page said "No
/// sessions yet" while its own records held a dozen, because a session
/// was its only when seen open in it. Its records make them its own on
/// Add, and a new one is read while the profile is closed.
#[test]
fn an_added_profiles_sessions_are_its_records_running_or_not() {
    let root = tempfile::tempdir().unwrap();
    let folder = profile_folder();
    let records = folder.path().join("claude-code-sessions/acct/org");
    std::fs::create_dir_all(&records).unwrap();
    let record = |local: &str, cli: &str, at: i64| {
        std::fs::write(
            records.join(format!("{local}.json")),
            format!(r#"{{"sessionId":"{local}","cliSessionId":"{cli}","title":"T","lastActivityAt":{at}}}"#),
        )
        .unwrap();
    };
    record("local_a", "s-1", 1_000);
    record("local_b", "s-2", 2_000);
    let state = state(root.path());
    observe(&state, &[claude_in(root.path(), folder.path())], &[]);
    let id = book::place_id("claude-app", Some(folder.path()));
    // Only offered: its records aren't read yet.
    assert!(desktop_app_by_session(&state).is_empty());

    answer_offer(&state, &id, true).unwrap();
    let by_session = desktop_app_by_session(&state);
    assert_eq!(by_session.get("claude-code/s-1"), Some(&id));
    assert_eq!(by_session.get("claude-code/s-2"), Some(&id));

    // Closed now; a session made in it before it closed is read too.
    record("local_c", "s-3", 3_000);
    observe(&state, &[], &[]);
    assert_eq!(
        desktop_app_by_session(&state).get("claude-code/s-3"),
        Some(&id)
    );
}

/// The go sitting's rerun (2026-11-07): rows named a closed profile's
/// sessions by their first prompt, not by the app's own title. A linked
/// profile's records give each its title, for the copy in its folder only.
#[test]
fn a_linked_profiles_records_name_its_sessions_as_the_app_does() {
    let root = tempfile::tempdir().unwrap();
    let folder = profile_folder();
    let records = folder.path().join("claude-code-sessions/acct/org");
    std::fs::create_dir_all(&records).unwrap();
    std::fs::write(
        records.join("local_a.json"),
        r#"{"sessionId":"local_a","cliSessionId":"s-1","title":"Nightly: skip docs-only changes","lastActivityAt":1}"#,
    )
    .unwrap();
    let state = state(root.path());
    observe(&state, &[claude_in(root.path(), folder.path())], &[]);
    let id = book::place_id("claude-app", Some(folder.path()));
    answer_offer(&state, &id, true).unwrap();
    with_book(&state, |book| {
        book.places.get_mut(&id).unwrap().account_id = Some("folder-aij".into());
    })
    .unwrap();
    observe(&state, &[], &[]);
    assert_eq!(
        record_title(&state, "folder-aij", "claude-code/s-1").as_deref(),
        Some("Nightly: skip docs-only changes")
    );
    assert_eq!(
        record_title(&state, "folder-other", "claude-code/s-1"),
        None
    );
}

/// 0.4.3: a profile on the real Mac had 3 records but about 40 transcripts
/// its app wrote; every one counted as its. The record index says which
/// conversations each linked profile's own records list (and archive),
/// for its folder's copy only, and which profiles were read: one read
/// with no records lists nothing; an unlinked one isn't read.
#[test]
fn the_record_index_is_what_the_linked_profiles_records_list() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let record = |dir: &Path, local: &str, cli: &str, archived: bool| {
        std::fs::write(
            dir.join("claude-code-sessions/acct/org")
                .join(format!("{local}.json")),
            format!(
                r#"{{"sessionId":"{local}","cliSessionId":"{cli}","isArchived":{archived},"lastActivityAt":1}}"#
            ),
        )
        .unwrap();
    };
    let profile = |name: &str| {
        let dir = root.path().join("ClaudeProfiles").join(name);
        std::fs::create_dir_all(dir.join("claude-code-sessions/acct/org")).unwrap();
        dir
    };
    let (lab, empty, loose) = (profile("Lab"), profile("Empty"), profile("Loose"));
    record(&lab, "local_a", "s-1", false);
    record(&lab, "local_b", "s-2", true);
    record(&loose, "local_c", "s-3", false);
    let place = |dir: &Path, folder: Option<&str>| book::KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude app".into(),
        instance: dir.file_name().map(|n| n.to_string_lossy().into_owned()),
        bundle: PathBuf::new(),
        exe: PathBuf::new(),
        given: None,
        instance_dir: Some(dir.to_path_buf()),
        account_id: folder.map(str::to_owned),
        conversations: Vec::new(),
        seen_ms: 1,
        keep_alive: false,
        paused: None,
        found: false,
        users: true,
        declined: false,
    };
    let id = |dir: &Path| book::place_id("claude-app", Some(dir));
    with_book(&state, |book| {
        book.places.insert(id(&lab), place(&lab, Some("f-lab")));
        book.places
            .insert(id(&empty), place(&empty, Some("f-empty")));
        book.places.insert(id(&loose), place(&loose, None));
    })
    .unwrap();
    // Already looked for a folder this run: none is linked here.
    state.places.looked.lock().unwrap().insert(id(&loose));
    assert!(read_records(&state));
    let index = record_index(&state);
    assert!(index.read(&id(&lab)));
    assert!(index.read(&id(&empty)), "read: its app lists nothing");
    assert!(
        !index.read(&id(&loose)),
        "unlinked: its copies aren't known"
    );
    assert!(index.listed("f-lab", "claude-code/s-1"));
    assert!(!index.archived("f-lab", "claude-code/s-1"));
    assert!(index.listed("f-lab", "claude-code/s-2"));
    assert!(index.archived("f-lab", "claude-code/s-2"));
    assert!(
        !index.listed("f-other", "claude-code/s-1"),
        "another folder's copy"
    );
    assert!(!index.listed("f-lab", "claude-code/s-3"));
    // Its lists leave out what its records don't list; another App's never.
    assert!(!index.leaves_out(&id(&lab), "f-lab", "claude-code/s-1"));
    assert!(index.leaves_out(&id(&lab), "f-lab", "claude-code/s-9"));
    assert!(index.leaves_out(&id(&empty), "f-empty", "claude-code/s-1"));
    assert!(!index.leaves_out(&id(&loose), "f-lab", "claude-code/s-9"));
    assert!(!index.leaves_out("cli:f-lab", "f-lab", "claude-code/s-9"));

    // Read again, the same: no change.
    assert!(!read_records(&state));
    // Archived back in the app, then deleted there: each is a change.
    record(&lab, "local_b", "s-2", false);
    assert!(read_records(&state));
    assert!(!record_index(&state).archived("f-lab", "claude-code/s-2"));
    std::fs::remove_file(lab.join("claude-code-sessions/acct/org/local_a.json")).unwrap();
    assert!(read_records(&state));
    assert!(!record_index(&state).listed("f-lab", "claude-code/s-1"));
    assert!(record_index(&state).leaves_out(&id(&lab), "f-lab", "claude-code/s-1"));
}
