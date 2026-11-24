//! F65 (0.5.1): every door that starts a desktop app's profile (Open,
//! Restart, keep alive, a move's own restarts) asks the one starter for
//! exactly the profile's own launch settings: its bundle, a further
//! instance, its `--user-data-dir` and its account's `CLAUDE_CONFIG_DIR`,
//! nothing else. The real starter hands them to `open` with none of AI
//! Juggler's environment (`juggler_platform::apps::launch_command`, tested
//! there). Fixtures only: a made-up Mac, temp folders, a recorder in place
//! of every start.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::features::places::fakes::{FakeDesktop, Recorder};
use crate::features::places::{self as places, QuitWait, RestartOf};
use crate::state::AppState;

/// A made-up Mac with one Claude app profile on an account whose tool
/// folder is `CodeProfiles/work`; its id, and the settings it starts with.
fn lab(root: &Path) -> (Arc<AppState>, String, Expected) {
    let state = places::fakes::on_made_up_mac(AppState::init(root.join("data")).unwrap(), root);
    let (id, place, profile) =
        places::remember_test_profile(&state, root, "b-work", Some("acct-work"));
    let folder = root.join("CodeProfiles/work");
    let account = juggler_core::model::Account {
        id: juggler_core::model::AccountId::new("acct-work"),
        harness: juggler_core::model::HarnessId::new("claude-code"),
        label: "Work Max".into(),
        plan: None,
        state_dir: folder.clone(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    crate::commands::lock_storage_arc(&state)
        .unwrap()
        .accounts()
        .insert(&account)
        .unwrap();
    let expected = Expected {
        bundle: place.bundle,
        args: vec![format!("--user-data-dir={}", profile.to_string_lossy())],
        env: vec![(
            "CLAUDE_CONFIG_DIR".to_owned(),
            folder.to_string_lossy().into_owned(),
        )],
    };
    (state, id, expected)
}

struct Expected {
    bundle: PathBuf,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

/// `starter` was asked once, for exactly the profile's own settings.
fn started_as_the_profile(door: &str, starter: &Recorder, expected: &Expected) {
    let starts = starter.starts.lock().unwrap();
    assert_eq!(starts.len(), 1, "{door}: one start");
    let (bundle, how, args) = &starts[0];
    assert_eq!(bundle, &expected.bundle, "{door}: its own bundle");
    assert!(how.new_instance, "{door}: a further instance");
    assert_eq!(args, &expected.args, "{door}: only its own folder");
    assert_eq!(
        *starter.envs.lock().unwrap(),
        std::slice::from_ref(&expected.env),
        "{door}: only its account's folder variable"
    );
}

#[test]
fn every_door_starts_a_profile_with_only_its_own_settings() {
    // Open.
    let tmp = tempfile::tempdir().unwrap();
    let (state, id, expected) = lab(tmp.path());
    let starter = Arc::new(Recorder::default());
    places::open(
        &state,
        &id,
        None,
        Arc::new(FakeDesktop::default()),
        starter.clone(),
        None,
    )
    .unwrap();
    started_as_the_profile("Open", &starter, &expected);

    // Restart.
    let tmp = tempfile::tempdir().unwrap();
    let (state, id, expected) = lab(tmp.path());
    let starter = Recorder::default();
    places::restart(
        &state,
        &id,
        &starter,
        true,
        RestartOf::Confirmed,
        QuitWait::default(),
        10_000_000,
    )
    .unwrap();
    started_as_the_profile("Restart", &starter, &expected);

    // Keep alive.
    let tmp = tempfile::tempdir().unwrap();
    let (state, id, expected) = lab(tmp.path());
    places::set_keep_alive(&state, &id, true).unwrap();
    let starter = Recorder::default();
    let start = 10_000_000;
    state.places.watch.lock().unwrap().settled_at(start);
    let mut now = start;
    while starter.starts.lock().unwrap().is_empty() && now < start + 2 * places::keep::WAIT_MS {
        now += 2_000;
        places::tick_with(&state, now, &starter, &|| false);
    }
    started_as_the_profile("Keep alive", &starter, &expected);

    // A move's own restarts (after it, its undo, a repair, a crashed
    // move's recovery: `MoveSystem::start` on the real system).
    let tmp = tempfile::tempdir().unwrap();
    let (state, id, expected) = lab(tmp.path());
    let place = places::with_book(&state, |book| book.places.get(&id).cloned())
        .unwrap()
        .unwrap();
    let starter = Recorder::default();
    places::start_held(&state, &place, &starter).unwrap();
    started_as_the_profile("A move's restart", &starter, &expected);
}
