//! AJ-E3203: the profiles beside one that ran, found by name only, after
//! the user's yes; started like the one beside them; gone with their
//! folder or the yes.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use juggler_platform::apps::Start;

use super::*;
use crate::features::places::start::{self, Starter};

/// A home with the Claude app installed and its profiles folder holding
/// `names`; the profiles folder and the bundle.
fn home_with(home: &Path, names: &[&str]) -> (PathBuf, PathBuf) {
    let profiles = home.join("Library/Application Support/ClaudeProfiles");
    for name in names {
        std::fs::create_dir_all(profiles.join(name)).unwrap();
    }
    let bundle = home.join("Applications/Claude.app");
    std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    (profiles, bundle)
}

fn hosts() -> HostRegistry {
    HostRegistry::new(juggler_adapters::claude_live::host_apps())
}

/// A profile seen running from `dir`, started from `bundle`.
fn ran(dir: &Path, bundle: &Path, seen_ms: i64) -> KnownPlace {
    KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude app".into(),
        instance: dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
        bundle: bundle.to_path_buf(),
        exe: bundle.join("Contents/MacOS/Claude"),
        given: Some(dir.to_path_buf()),
        instance_dir: Some(dir.to_path_buf()),
        account_id: Some("folder-work".into()),
        conversations: vec!["claude-code/s-1".into()],
        seen_ms,
        keep_alive: false,
        paused: None,
        found: false,
    }
}

fn book_with(places: Vec<KnownPlace>) -> Book {
    let mut book = Book::default();
    for place in places {
        book.places.insert(
            book::place_id(&place.app_id, place.instance_dir.as_deref()),
            place,
        );
    }
    book
}

fn found_names(book: &Book) -> Vec<String> {
    let mut names: Vec<String> = book
        .places
        .values()
        .filter(|place| place.found)
        .filter_map(|place| place.instance.clone())
        .collect();
    names.sort();
    names
}

#[derive(Default)]
struct Recorder(Mutex<Vec<(PathBuf, Start, Vec<String>)>>);

impl Starter for Recorder {
    fn trusted(&self, _: &Path) -> Result<(), String> {
        Ok(())
    }
    fn start(&self, bundle: &Path, how: Start, args: &[String]) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .push((bundle.to_path_buf(), how, args.to_vec()));
        Ok(())
    }
}

#[test]
fn the_profiles_beside_one_that_ran_become_apps_that_start_in_their_own_folder() {
    let home = tempfile::tempdir().unwrap();
    let (profiles, bundle) = home_with(home.path(), &["b-work", "b-side", "b-home"]);
    let mut book = book_with(vec![ran(&profiles.join("b-work"), &bundle, 10)]);
    assert!(find(&mut book, &hosts(), home.path(), 99));
    assert_eq!(found_names(&book), ["b-home", "b-side"]);
    let side = &book.places[&book::place_id("claude-app", Some(&profiles.join("b-side")))];
    assert_eq!(
        side.named(None),
        "Claude app · b-side",
        "named by its own folder"
    );
    assert_eq!(
        side.account_id, None,
        "no Account until its sessions show one"
    );
    assert_eq!(side.seen_ms, 99);
    // Started from the bundle its sibling ran from, with only its folder.
    let starter = Recorder::default();
    start::start(side, &hosts(), &starter, false).unwrap();
    let starts = starter.0.lock().unwrap();
    let (started, how, args) = &starts[0];
    assert_eq!(started, &bundle);
    assert!(how.new_instance);
    assert_eq!(
        args,
        &[format!(
            "--user-data-dir={}",
            profiles.join("b-side").display()
        )]
    );
    // Found again a minute later: nothing new.
    assert!(!find(&mut book, &hosts(), home.path(), 60_099));
}

#[test]
fn a_found_profile_goes_with_its_folder_and_stays_once_it_ran() {
    let home = tempfile::tempdir().unwrap();
    let (profiles, bundle) = home_with(home.path(), &["b-work", "b-side", "b-play"]);
    let mut book = book_with(vec![ran(&profiles.join("b-work"), &bundle, 10)]);
    find(&mut book, &hosts(), home.path(), 20);
    std::fs::remove_dir(profiles.join("b-play")).unwrap();
    assert!(find(&mut book, &hosts(), home.path(), 30));
    assert_eq!(found_names(&book), ["b-side"]);
    // b-side is seen running (given as `b-side/`): one App, no longer found.
    let side = profiles.join("b-side");
    let id = book::place_id("claude-app", Some(&side));
    book.places.get_mut(&id).unwrap().found = false;
    assert!(!find(&mut book, &hosts(), home.path(), 40));
    assert!(found_names(&book).is_empty());
    assert_eq!(book.places.len(), 2);
    // Turning finding off forgets only what was found and never ran.
    std::fs::create_dir_all(profiles.join("b-new")).unwrap();
    find(&mut book, &hosts(), home.path(), 50);
    assert_eq!(found_names(&book), ["b-new"]);
    assert!(forget_found(&mut book));
    assert_eq!(book.places.len(), 2);
}

#[test]
fn a_folder_given_with_a_trailing_slash_is_the_same_profile() {
    let home = tempfile::tempdir().unwrap();
    let (profiles, bundle) = home_with(home.path(), &["b-work", "b-side"]);
    let slashed = PathBuf::from(format!("{}/", profiles.join("b-work").display()));
    let mut book = book_with(vec![ran(&slashed, &bundle, 10)]);
    find(&mut book, &hosts(), home.path(), 20);
    assert_eq!(found_names(&book), ["b-side"], "b-work isn't found twice");
}

#[test]
fn finding_never_forgets_a_profile_that_ran_to_make_room() {
    let home = tempfile::tempdir().unwrap();
    let names: Vec<String> = (0..10).map(|n| format!("p{n}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let (profiles, bundle) = home_with(home.path(), &refs);
    let mut places = vec![ran(&profiles.join("p0"), &bundle, 10)];
    for n in 0..MAX - 3 {
        let mut other = ran(Path::new(&format!("/elsewhere/q{n}")), &bundle, 5);
        other.instance_dir = Some(PathBuf::from(format!("/elsewhere/q{n}")));
        places.push(other);
    }
    let mut book = book_with(places);
    let before = book.places.len();
    find(&mut book, &hosts(), home.path(), 20);
    assert_eq!(book.places.len(), MAX);
    assert_eq!(found_names(&book).len(), MAX - before);
    assert!(book.places.values().filter(|place| !place.found).count() == before);
}

#[test]
fn the_default_instance_and_an_app_without_profiles_side_by_side_find_nothing() {
    let home = tempfile::tempdir().unwrap();
    let (_, bundle) = home_with(home.path(), &[]);
    let support = home.path().join("Library/Application Support");
    std::fs::create_dir_all(support.join("Claude")).unwrap();
    std::fs::create_dir_all(support.join("Another App")).unwrap();
    let mut book = book_with(vec![ran(&support.join("Claude"), &bundle, 10)]);
    assert!(!find(&mut book, &hosts(), home.path(), 20));
    assert!(folders(&book, &hosts(), home.path()).is_empty());
}

#[test]
fn nothing_is_listed_until_the_user_says_yes_and_no_forgets_what_was_found() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let (profiles, bundle) = home_with(&home, &["b-work", "b-side"]);
    let state = Arc::new(AppState::init(root.path().join("data")).unwrap());
    let work = ran(&profiles.join("b-work"), &bundle, 10);
    super::super::with_book(&state, |book| {
        book.places.insert(
            book::place_id("claude-app", work.instance_dir.as_deref()),
            work,
        );
    })
    .unwrap();
    let hosts = HostRegistry::of(&state.registry);
    assert_eq!(answer(&state).unwrap(), Answer::Ask);
    assert!(
        !after_scan(&state, &hosts, &home, 20),
        "not asked yet: nothing read"
    );
    let shown_folders =
        super::super::with_book(&state, |book| folders(book, &hosts, &home)).unwrap();
    assert_eq!(shown_folders.len(), 1);
    assert_eq!(
        shown(&shown_folders[0].folder, &home),
        format!(
            "~{0}Library{0}Application Support{0}ClaudeProfiles",
            std::path::MAIN_SEPARATOR
        )
    );
    set(&state, true, &home, 30).unwrap();
    assert_eq!(answer(&state).unwrap(), Answer::On);
    let book = super::super::with_book(&state, |book| book.clone()).unwrap();
    assert_eq!(found_names(&book), ["b-side"]);
    // Kept as a desktop App's row, and read back the same.
    let id = book::place_id("claude-app", Some(&profiles.join("b-side")));
    let row = state
        .storage
        .lock()
        .unwrap()
        .apps()
        .get(&id)
        .unwrap()
        .unwrap();
    assert_eq!(row.kind, "desktop");
    assert_eq!(row.host_app_id.as_deref(), Some("claude-app"));
    assert_eq!(
        row.instance_dir,
        Some(profiles.join("b-side").to_string_lossy().into_owned())
    );
    // The window can't say yes for the user.
    assert!(crate::settings_policy::app_setting_set_impl(&state, KEY, "on").is_err());
    set(&state, false, &home, 40).unwrap();
    assert_eq!(answer(&state).unwrap(), Answer::Off);
    let book = super::super::with_book(&state, |book| book.clone()).unwrap();
    assert!(found_names(&book).is_empty());
    assert!(
        state
            .storage
            .lock()
            .unwrap()
            .apps()
            .get(&id)
            .unwrap()
            .is_none()
    );
    assert!(!after_scan(&state, &hosts, &home, 50), "off: nothing read");
}
