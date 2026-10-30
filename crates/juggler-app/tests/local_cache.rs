//! ADR-0023: the local cache through the app — its three caches listed
//! in plain words, last messages turned off dropping the subagent lists'
//! words at once, the switch removing the files and kept across a
//! restart, and the limit kept.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::features::cache;
use juggler_app::features::conversations::last_messages;
use juggler_app::features::subagents::index;
use juggler_app::settings_policy::app_setting_set_impl;
use juggler_app::state::AppState;

fn state_in(data: &Path) -> Arc<AppState> {
    let state =
        Arc::new(AppState::init_with_registry(data.to_path_buf(), AdapterRegistry::new()).unwrap());
    cache::register_defaults(&state);
    state
}

#[test]
fn turning_last_messages_off_drops_the_cached_words_at_once() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = dir.path().join("data");
    let state = state_in(&data);
    let gate = state.cache.gate();
    let folder = data.join(index::DIR);
    let with_words = index::path_for(&folder, "acc", "s1", Path::new("/p/s1.jsonl"));
    assert!(index::save(&gate, &with_words, true, b"{\"prompt\":\"redacted words\"}").unwrap());
    // Another setting leaves it.
    app_setting_set_impl(&state, "some_other_setting", "1").unwrap();
    assert!(with_words.exists());
    // Last messages off: gone, not just unused.
    app_setting_set_impl(&state, last_messages::SETTING_KEY, "0").unwrap();
    assert!(!with_words.exists(), "the words are dropped at once");
    // Saved without them, it stays; turned on again, that one goes too
    // (it lacks what the tab would show).
    assert!(index::save(&gate, &with_words, false, b"{}").unwrap());
    app_setting_set_impl(&state, last_messages::SETTING_KEY, "0").unwrap();
    assert!(with_words.exists());
    app_setting_set_impl(&state, last_messages::SETTING_KEY, "").unwrap();
    assert!(!with_words.exists());
}

#[test]
fn the_switch_removes_the_files_and_is_kept_with_the_limit_across_a_restart() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let data = dir.path().join("data");
    let state = state_in(&data);
    let info = cache::status_impl(&state).unwrap();
    assert!(info.on, "on by default");
    assert!(info.automatic);
    let names: Vec<&str> = info.caches.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, vec!["Subagent lists", "App icons", "Search index"]);
    assert!(
        info.caches
            .iter()
            .all(|row| !row.speeds_up.is_empty() && !row.slower.is_empty())
    );
    let search = &info.caches[2];
    assert!(search.whole_only && search.own_switch.is_some());
    assert!(!search.on, "search is off until turned on in Privacy");
    assert_eq!(search.own_switch_section.as_deref(), Some("search"));
    assert!(info.budget_bytes >= cache::budget::FLOOR);

    let gate = state.cache.gate();
    let folder = data.join(index::DIR);
    let file = index::path_for(&folder, "acc", "s1", Path::new("/p/s1.jsonl"));
    assert!(index::save(&gate, &file, true, b"{}").unwrap());
    let info = cache::status_impl(&state).unwrap();
    assert!(info.used_bytes > 0);

    let off = cache::set_on_impl(&state, false).unwrap();
    assert!(!off.on);
    assert!(!folder.exists(), "its files are removed");
    assert_eq!(off.used_bytes, 0);
    assert!(
        !index::save(&gate, &file, true, b"{}").unwrap(),
        "nothing written while off"
    );

    let limited = cache::set_limit_impl(&state, Some(512)).unwrap();
    assert_eq!(limited.limit_mb, Some(512));
    assert!(!limited.automatic);
    drop(state);

    // A restart keeps both.
    let again = state_in(&data);
    let info = cache::status_impl(&again).unwrap();
    assert!(!info.on);
    assert_eq!(info.limit_mb, Some(512));
    let on = cache::set_on_impl(&again, true).unwrap();
    assert!(on.on);
    let auto = cache::set_limit_impl(&again, None).unwrap();
    assert!(auto.automatic);
    assert!(index::save(&again.cache.gate(), &file, true, b"{}").unwrap());
    let cleared = cache::clear_impl(&again).unwrap();
    assert_eq!(cleared.used_bytes, 0);
    assert!(!folder.exists());
}
