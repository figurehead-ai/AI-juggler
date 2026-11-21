//! The watches on today's event stream, end to end: a session's status
//! published on the notification path reaches the engine on its thread,
//! fires, is logged, and is told once as a `watch` notice whose alert goes
//! through the notification path (AJ-E3817); the reads over IPC say it in
//! plain words.

use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_core::model::SessionStatus;
use juggler_core::storage::memory::Writers;

use super::*;
use crate::features::ai::memory::RedactionSecrets;
use crate::features::watches::commands;
use crate::features::watches::store::{Origin, WatchStore};
use crate::state::AppState;

/// A state with the fake terminal tool, an account, launch settings and
/// one running session `s1`, and the memory store's tables.
fn lab(dir: &std::path::Path) -> Arc<AppState> {
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::fake::FakeHarnessAdapter::new(
        "/bin/true",
    )));
    let state = Arc::new(AppState::init_with_registry(dir.to_path_buf(), registry).unwrap());
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&crate::test_support::account(
                "acct-1",
                juggler_adapters::fake::FAKE_HARNESS_ID,
                "Lab",
                dir.join("acct"),
            ))
            .unwrap();
        storage
            .profiles()
            .insert(&crate::test_support::launch_settings(
                "p1",
                "Lab terminal",
                "acct-1",
            ))
            .unwrap();
        storage
            .sessions()
            .insert(&crate::test_support::instance(
                "s1",
                "p1",
                SessionStatus::Running,
                dir.to_path_buf(),
            ))
            .unwrap();
    }
    state
}

/// The `watch` notices told so far: (key, title, detail).
fn watch_notices(state: &AppState) -> Vec<(String, String, String)> {
    let storage = state.storage.lock().unwrap();
    let mut stmt = storage
        .raw_conn()
        .prepare("SELECT key, title, detail FROM notices WHERE kind = 'watch' ORDER BY id")
        .unwrap();
    stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn wait_for_notices(state: &AppState, n: usize) -> Vec<(String, String, String)> {
    let started = Instant::now();
    loop {
        let told = watch_notices(state);
        if told.len() >= n || started.elapsed() > Duration::from_secs(10) {
            return told;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_session_status_fires_a_watch_on_its_thread() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let writers = Writers::for_tests();
    let now = crate::commands::now_millis();
    {
        let storage = state.storage.lock().unwrap();
        let registry = Registry::shipped();
        let store = WatchStore::new(&storage, &registry);
        let mut w = watch(
            "w-11",
            vec![session("s1")],
            vec![cond("fails")],
            Lifetime::Once,
        );
        w.created_ms = now;
        w.even_in_quiet_hours = true;
        store
            .create(
                &writers.watches,
                &RedactionSecrets,
                &w,
                &Origin::WatchesView,
                now,
            )
            .unwrap();
    }
    crate::features::notifications::session_status(&state, "s1", "failed");
    let told = wait_for_notices(&state, 1);
    assert_eq!(told.len(), 1, "the fire was told once: {told:?}");
    let (key, title, detail) = &told[0];
    assert!(key.starts_with("watch:w-11:s1:fails:"), "{key}");
    assert!(title.ends_with("” failed"), "{title}");
    assert_eq!(detail, "Your watch");
    let listed =
        commands::watches_list_impl(&state, &crate::paging::PageRequest::first(50), Lang::En)
            .unwrap()
            .items;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].told, 1);
    assert_eq!(listed[0].state, commands::WatchStateInfo::Ended);
    assert_eq!(listed[0].ended, Some(EndReason::FirstFire));
    assert_eq!(
        listed[0].ended_words.as_deref(),
        Some("Ended: it told you once, as asked.")
    );
    assert_eq!(state.watches.hearing(), 0, "a once watch is heard no more");
}

#[test]
fn the_reads_offer_what_the_lab_can_report_and_say_a_draft() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    // The fake terminal tool reports no account room: "runs below" isn't offered.
    let offered: Vec<String> =
        commands::watches_catalogue_impl(&state, &WatchTarget::Session("s1".into()), Lang::En)
            .unwrap()
            .into_iter()
            .map(|e| e.code)
            .collect();
    assert!(offered.contains(&"stuck".to_owned()) && offered.contains(&"hits_limit".to_owned()));
    assert!(
        !offered
            .iter()
            .any(|c| c == "runway_below" || c == "commits")
    );
    let draft = commands::WatchDraftInfo {
        targets: vec![WatchTarget::Session("s1".into())],
        matching: Matching::Each,
        conditions: vec![cond("finishes")],
        lifetime: LifetimeChoice::Unsaid,
        even_in_quiet_hours: false,
        chat_turn: None,
    };
    let preview = commands::watches_preview_impl(&state, &draft, Lang::En).unwrap();
    assert_eq!(preview.problem, None);
    assert_eq!(
        preview.echo.joined(),
        preview.words,
        "the parts are the words"
    );
    assert_eq!(
        preview.watch.lifetime,
        Lifetime::Once,
        "finishing on a session: once"
    );
    assert!(
        preview.words.starts_with("Tell you when “"),
        "{}",
        preview.words
    );
    assert!(
        preview.words.ends_with(" finishes. Once."),
        "{}",
        preview.words
    );
    let mut unsupported = draft.clone();
    unsupported.conditions = vec![cond_p("runway_below", Param::Percent(20))];
    unsupported.targets = vec![WatchTarget::Account("acct-1".into())];
    let refused = commands::watches_preview_impl(&state, &unsupported, Lang::En).unwrap();
    assert_eq!(
        refused.problem.as_deref(),
        Some("condition_not_for_target:runway_below:account")
    );
    // A read writes nothing.
    let storage = state.storage.lock().unwrap();
    let rows: i64 = storage
        .raw_conn()
        .query_row("SELECT count(*) FROM memory_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn without_the_memory_store_the_list_says_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(dir.path().to_path_buf());
    // A database without V54's store (one restored from before it).
    state
        .storage
        .lock()
        .unwrap()
        .raw_conn()
        .execute_batch("DROP TABLE memory_records;")
        .unwrap();
    let error =
        commands::watches_list_impl(&state, &crate::paging::PageRequest::first(50), Lang::En)
            .unwrap_err();
    assert!(error.to_string().contains("unavailable"), "{error}");
}

/// Creates `watch` in the store, as the Watches view's click does.
fn create(state: &AppState, writers: &Writers, w: &Watch) {
    let storage = state.storage.lock().unwrap();
    let registry = Registry::shipped();
    WatchStore::new(&storage, &registry)
        .create(
            &writers.watches,
            &RedactionSecrets,
            w,
            &Origin::WatchesView,
            w.created_ms,
        )
        .unwrap();
}

fn sink(state: &AppState) -> Arc<std::sync::Mutex<Vec<crate::features::notifications::Outgoing>>> {
    let list = Arc::new(std::sync::Mutex::new(Vec::new()));
    let into = Arc::clone(&list);
    state.notifier.set_sink(Box::new(move |out| {
        into.lock().unwrap().push(out.clone());
    }));
    list
}

fn wait_for_sent(
    list: &std::sync::Mutex<Vec<crate::features::notifications::Outgoing>>,
    n: usize,
) -> Vec<crate::features::notifications::Outgoing> {
    let started = Instant::now();
    loop {
        let sent = list.lock().unwrap().clone();
        if sent.len() >= n || started.elapsed() > Duration::from_secs(10) {
            return sent;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// AJ-E3817: quiet hours hold a watch's alert unless its own "Even during
/// quiet hours" is on; when they end, what they held comes as one.
#[test]
fn quiet_hours_hold_an_alert_unless_its_watch_breaks_through() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&crate::test_support::instance(
            "s2",
            "p1",
            SessionStatus::Running,
            dir.path().join("web"),
        ))
        .unwrap();
    let writers = Writers::for_tests();
    let now = crate::commands::now_millis();
    let mut loud = watch(
        "w-21",
        vec![session("s1")],
        vec![cond("fails")],
        Lifetime::Once,
    );
    loud.created_ms = now;
    loud.even_in_quiet_hours = true;
    let mut held = watch(
        "w-22",
        vec![session("s2")],
        vec![cond("fails")],
        Lifetime::Once,
    );
    held.created_ms = now;
    create(&state, &writers, &loud);
    create(&state, &writers, &held);
    let list = sink(&state);
    state.notifier.local_hour_seen(Some(23), now);
    crate::features::notifications::session_status(&state, "s1", "failed");
    crate::features::notifications::session_status(&state, "s2", "failed");
    assert_eq!(
        wait_for_notices(&state, 2).len(),
        2,
        "both told in the centre"
    );
    let sent = wait_for_sent(&list, 1);
    std::thread::sleep(Duration::from_millis(100));
    let sent_now = list.lock().unwrap().clone();
    assert_eq!(
        sent_now.len(),
        1,
        "only the watch set to break through: {sent:?}"
    );
    assert_eq!(sent_now[0].body, "Failed · your watch");
    // Morning: the held alert comes, once.
    state.notifier.local_hour_seen(Some(8), now);
    crate::features::notifications::show_due_at(&state, crate::commands::now_millis());
    let sent = list.lock().unwrap().clone();
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert!(sent[1].title.contains("web"), "{sent:?}");
    crate::features::notifications::show_due_at(&state, crate::commands::now_millis() + 120_000);
    assert_eq!(list.lock().unwrap().len(), 2, "told once");
}

/// AJ-E3817, ADR-0036 §2.3: what happened while AI Juggler was closed is
/// replayed from the journals past the cursor, once, and told so.
#[test]
fn a_fire_missed_while_closed_is_replayed_on_start_and_told_once() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let writers = Writers::for_tests();
    let now = crate::commands::now_millis();
    let mut w = watch(
        "w-31",
        vec![session("s1")],
        vec![cond("finishes")],
        Lifetime::Once,
    );
    w.created_ms = now - 3_600_000;
    create(&state, &writers, &w);
    {
        let storage = state.storage.lock().unwrap();
        // AI Juggler last heard an hour ago; then s1 finished.
        storage
            .app_settings()
            .set(
                super::super::runtime::CURSOR_KEY,
                &(now - 3_600_000).to_string(),
            )
            .unwrap();
        storage
            .sessions()
            .record_lifecycle(
                &juggler_core::model::SessionId::new("s1"),
                "ended: finished",
            )
            .unwrap();
    }
    let list = sink(&state);
    state.notifier.local_hour_seen(Some(12), now);
    super::super::runtime::start(&state);
    let told = wait_for_notices(&state, 1);
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0].2, "Your watch · while AI Juggler was closed");
    let sent = wait_for_sent(&list, 1);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(
        sent[0].body.ends_with("while AI Juggler was closed"),
        "{sent:?}"
    );
    // The cursor moved on: a second start replays nothing.
    let cursor: i64 = state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .get(super::super::runtime::CURSOR_KEY)
        .unwrap()
        .unwrap()
        .parse()
        .unwrap();
    assert!(cursor >= now);
    assert!(
        super::super::runtime::journal_since(&state.storage.lock().unwrap(), cursor).is_empty()
    );
}

/// AJ-E3816: what the Watches view reads. Its echo in parts is the echo
/// word for word; the list splits into live and ended; the counts; a
/// removal's Undo brings the watch back; pause and resume by id.
#[test]
fn the_view_reads_parts_counts_and_brings_back_a_removed_watch() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let draft = |conditions: Vec<Condition>, lifetime, quiet| commands::WatchDraftInfo {
        targets: vec![WatchTarget::Session("s1".into())],
        matching: Matching::Each,
        conditions,
        lifetime,
        even_in_quiet_hours: quiet,
        chat_turn: None,
    };
    let first = commands::watches_create_impl(
        &state,
        &draft(
            vec![cond("finishes"), cond_p("stuck", Param::Minutes(15))],
            LifetimeChoice::Always,
            true,
        ),
        Lang::En,
    )
    .unwrap();
    assert_eq!(first.echo.joined(), first.words, "the parts are the echo");
    let names: Vec<&str> = first
        .echo
        .head
        .iter()
        .filter(|span| span.name)
        .map(|span| span.text.as_str())
        .collect();
    assert_eq!(names.len(), 1, "{:?}", first.echo);
    assert!(names[0].starts_with('“') && names[0].ends_with('”'));
    assert_eq!(first.echo.lifetime, "Until you remove it.");
    assert_eq!(
        first.echo.quiet.as_deref(),
        Some("Even during quiet hours.")
    );
    assert!(
        first
            .echo
            .head_text()
            .ends_with(" finishes, or looks stuck for 15 minutes.")
    );
    assert_eq!((first.last_told_ms, first.ended_ms), (None, None));

    let second = commands::watches_create_impl(
        &state,
        &draft(vec![cond("fails")], LifetimeChoice::Today, false),
        Lang::En,
    )
    .unwrap();
    let paused = commands::watches_pause_impl(&state, &second.watch.id, true, Lang::En).unwrap();
    assert_eq!(paused.state, commands::WatchStateInfo::Paused);
    let summary = commands::watches_summary_impl(&state).unwrap();
    assert_eq!(
        (
            summary.watching,
            summary.paused,
            summary.never_told,
            summary.even_in_quiet_hours,
            summary.limit
        ),
        (1, 1, 2, 1, 50)
    );
    let resumed = commands::watches_pause_impl(&state, &second.watch.id, false, Lang::En).unwrap();
    assert_eq!(resumed.state, commands::WatchStateInfo::Active);

    // Removed, then brought back by Undo: the same id, watching again.
    commands::watches_remove_impl(&state, &first.watch.id, Lang::En).unwrap();
    let live = |which| {
        commands::watches_list_of(
            &state,
            &crate::paging::PageRequest::first(10),
            which,
            Lang::En,
        )
        .unwrap()
        .items
        .into_iter()
        .map(|w| w.watch.id)
        .collect::<Vec<_>>()
    };
    assert_eq!(
        live(commands::WatchListWhich::Live),
        vec![second.watch.id.clone()]
    );
    let back = commands::watches_restore_impl(&state, &first.watch.id, Lang::En).unwrap();
    assert_eq!(back.watch.id, first.watch.id);
    assert_eq!(back.state, commands::WatchStateInfo::Active);
    assert_eq!(live(commands::WatchListWhich::Live).len(), 2);
    assert!(live(commands::WatchListWhich::Ended).is_empty());
    assert!(
        commands::watches_restore_impl(&state, &first.watch.id, Lang::En).is_err(),
        "only a removed watch comes back"
    );

    // It fires: the list says when, and what it told.
    crate::features::notifications::session_status(&state, "s1", "failed");
    wait_for_notices(&state, 1);
    let all = commands::watches_list_of(
        &state,
        &crate::paging::PageRequest::first(10),
        commands::WatchListWhich::All,
        Lang::En,
    )
    .unwrap()
    .items;
    let told = all.iter().find(|w| w.watch.id == second.watch.id).unwrap();
    assert_eq!(told.told, 1);
    assert_eq!(told.last_told_code.as_deref(), Some("fails"));
    assert_eq!(told.last_told_words.as_deref(), Some("failed"));
    assert!(told.last_told_ms.is_some());
    let summary = commands::watches_summary_impl(&state).unwrap();
    assert_eq!(summary.told_today, 1, "{summary:?}");
}

/// AJ-E3816: a watch on an App targets it by its App id, as every other
/// surface does. It fires for a session of that App under each launch
/// setting; the App is named as the rail names it; and a watch stored
/// before (an App by its launch settings' id) moves to the App id once.
#[test]
fn an_app_watch_hears_every_launch_setting_and_old_ones_move_to_the_app_id() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let writers = Writers::for_tests();
    let now = crate::commands::now_millis();
    {
        let storage = state.storage.lock().unwrap();
        storage
            .profiles()
            .insert(&crate::test_support::launch_settings(
                "p2",
                "Lab terminal, quick",
                "acct-1",
            ))
            .unwrap();
        storage
            .sessions()
            .insert(&crate::test_support::instance(
                "s2",
                "p2",
                SessionStatus::Running,
                dir.path().to_path_buf(),
            ))
            .unwrap();
    }
    let app = WatchTarget::App(juggler_core::storage::cli_app_id("acct-1"));
    let mut on_app = watch(
        "w-a0",
        vec![app.clone()],
        vec![cond("fails")],
        Lifetime::Always,
    );
    on_app.created_ms = now;
    // Stored before App ids: its App named by launch settings `p1`.
    let mut legacy = watch(
        "w-a1",
        vec![WatchTarget::App("p1".into())],
        vec![cond("fails")],
        Lifetime::Always,
    );
    legacy.created_ms = now;
    create(&state, &writers, &on_app);
    create(&state, &writers, &legacy);
    {
        let storage = state.storage.lock().unwrap();
        let facts = crate::features::watches::runtime::Facts::new(&storage, &state);
        for session in ["s1", "s2"] {
            assert_eq!(
                facts.session_subject(session).apps,
                vec![juggler_core::storage::cli_app_id("acct-1")],
                "{session}'s App, by its App id"
            );
        }
        assert_eq!(facts.exists(&app), Some(true));
        assert_eq!(
            facts.exists(&WatchTarget::App("cli:gone".into())),
            Some(false)
        );
    }
    crate::features::notifications::session_status(&state, "s1", "failed");
    crate::features::notifications::session_status(&state, "s2", "failed");
    let told = wait_for_notices(&state, 4);
    for session in ["s1", "s2"] {
        assert!(
            told.iter()
                .any(|(key, _, _)| key.starts_with(&format!("watch:w-a0:{session}:fails:"))),
            "the App watch told {session}: {told:?}"
        );
        assert!(
            told.iter()
                .any(|(key, _, _)| key.starts_with(&format!("watch:w-a1:{session}:fails:"))),
            "the moved watch told {session}: {told:?}"
        );
    }
    let listed =
        commands::watches_list_impl(&state, &crate::paging::PageRequest::first(10), Lang::En)
            .unwrap()
            .items;
    let moved = listed.iter().find(|w| w.watch.id.0 == "w-a1").unwrap();
    assert_eq!(
        moved.watch.targets,
        vec![app.clone()],
        "moved to the App id"
    );
    let named = listed.iter().find(|w| w.watch.id.0 == "w-a0").unwrap();
    assert!(
        named.words.starts_with("Tell you when “") && named.echo.head.iter().any(|s| s.name),
        "the App is named: {}",
        named.words
    );
}
