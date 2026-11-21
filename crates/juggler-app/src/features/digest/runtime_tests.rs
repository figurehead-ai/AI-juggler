//! A digest end to end: made from the window's observations and the
//! session rows, kept on the store, read back as the feed's digest item,
//! and its calm news told as one notification; seven finishes with the
//! digest on are that one notification; the switch off makes none.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use juggler_core::storage::memory::Writers;

use super::*;
use crate::features::feed::dto::FeedFiltersInfo;
use crate::features::feed::page::{self, Read};
use crate::features::notifications::batch::BATCH_MS;
use crate::features::notifications::{Outgoing, prefs, session_status, show_due_at};
use crate::features::observe::EndKind;
use crate::paging::PageRequest;

fn finished(at: i64) -> Vec<(i64, Observed)> {
    vec![
        (
            at,
            Observed::StateEntered {
                state: SessionState::Finished,
            },
        ),
        (
            at,
            Observed::Ended {
                how: EndKind::Clean,
            },
        ),
    ]
}

fn window(events: &BTreeMap<String, Vec<(i64, Observed)>>, end: i64) -> Due {
    Due {
        start_ms: end - 30 * 60_000,
        end_ms: end,
        agents: events.keys().cloned().collect(),
    }
}

fn digests_in_feed(
    state: &AppState,
    filters: &FeedFiltersInfo,
) -> Vec<super::super::rollup::DigestInfo> {
    let storage = state.storage.lock().unwrap();
    page::page(&storage, &PageRequest::first(30), filters, Read::default())
        .unwrap()
        .items
        .into_iter()
        .filter(|item| item.kind == "agent_digest")
        .filter_map(|item| item.digest)
        .collect()
}

#[test]
fn a_digest_is_kept_and_shown_in_the_feed_needs_first_every_agent_counted() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::seven_sessions(dir.path());
    let writers = Writers::for_tests();
    let now = crate::commands::now_millis();
    let mut events = BTreeMap::new();
    events.insert(
        "s2".to_owned(),
        vec![(
            now - 60_000,
            Observed::StateEntered {
                state: SessionState::Failed,
            },
        )],
    );
    events.insert(
        "s1".to_owned(),
        vec![(
            now - 50_000,
            Observed::StateEntered {
                state: SessionState::AsksYou,
            },
        )],
    );
    events.insert("s3".to_owned(), finished(now - 40_000));
    assert!(make_with(
        &state,
        &writers.digest,
        &window(&events, now),
        events
    ));
    let shown = digests_in_feed(&state, &FeedFiltersInfo::default());
    assert_eq!(shown.len(), 1, "one digest item");
    let digest = &shown[0];
    let order: Vec<(&str, &str)> = digest
        .lines
        .iter()
        .map(|l| (l.name.as_str(), l.state.as_str()))
        .collect();
    assert_eq!(
        order,
        [("web", "failed"), ("api", "asks"), ("docs", "finished")]
    );
    assert_eq!(digest.quiet, 4, "the four open sessions with nothing new");
    assert!(!digest.ai, "the rollup wrote it (the AI digest is not yet)");
    // A digest is about every agent: a filter leaves it out.
    let needs = FeedFiltersInfo {
        category: Some("needs".into()),
        ..FeedFiltersInfo::default()
    };
    assert!(digests_in_feed(&state, &needs).is_empty());
    // Its log event says the rollup wrote it, and why.
    let made: i64 = state
        .storage
        .lock()
        .unwrap()
        .raw_conn()
        .query_row(
            "SELECT count(*) FROM ai_log WHERE stream = 'digest' AND verb = 'made'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(made, 1);
}

#[test]
fn seven_finishes_with_the_digest_on_are_one_notification() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::seven_sessions(dir.path());
    let writers = Writers::for_tests();
    prefs::notifications_set_impl(
        &state,
        prefs::SystemNotificationsInfo {
            finished: true,
            digest: true,
            ..Default::default()
        },
    )
    .unwrap();
    let base = crate::commands::now_millis();
    state.notifier.local_hour_seen(Some(12), base);
    for i in 1..=7 {
        session_status(&state, &format!("s{i}"), "finished");
    }
    let list = Arc::new(Mutex::new(Vec::<Outgoing>::new()));
    let into = Arc::clone(&list);
    state
        .notifier
        .set_sink(Box::new(move |out| into.lock().unwrap().push(out.clone())));
    show_due_at(&state, base);
    assert!(
        list.lock().unwrap().is_empty(),
        "the finishes wait for the digest"
    );
    let events: BTreeMap<String, Vec<(i64, Observed)>> =
        (1..=7).map(|i| (format!("s{i}"), finished(base))).collect();
    assert!(make_with(
        &state,
        &writers.digest,
        &window(&events, base),
        events
    ));
    show_due_at(&state, crate::commands::now_millis() + BATCH_MS + 1_000);
    let sent = list.lock().unwrap().clone();
    assert_eq!(sent.len(), 1, "one notification, not seven: {sent:?}");
    assert_eq!(sent[0].title, "Digest · 7 agents");
    assert_eq!(sent[0].body, "7 agents finished");
}

#[test]
fn the_switch_off_makes_no_digest() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::seven_sessions(dir.path());
    let writers = Writers::for_tests();
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(ON_KEY, "false")
        .unwrap();
    let now = crate::commands::now_millis();
    let mut events = BTreeMap::new();
    events.insert("s1".to_owned(), finished(now));
    assert!(!make_with(
        &state,
        &writers.digest,
        &window(&events, now),
        events
    ));
    assert!(digests_in_feed(&state, &FeedFiltersInfo::default()).is_empty());
}

#[test]
fn nothing_heard_nothing_scheduled_and_a_heard_burst_is_one_due_digest() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::seven_sessions(dir.path());
    assert_eq!(state.digests.due_at(), None, "quiet: no timer");
    let now = crate::commands::now_millis();
    let obs = |session: &str| Observation {
        at_ms: now,
        subject: crate::features::observe::Subject {
            session: Some(session.into()),
            ..Default::default()
        },
        event: Observed::Ended {
            how: EndKind::Clean,
        },
        replayed: false,
    };
    heard(&state, &[obs("s1"), obs("s2"), obs("s3")]);
    assert_eq!(
        state.digests.due_at(),
        Some(now + super::super::scheduler::NOTABLE_DELAY_MS),
        "three endings: one digest a minute later"
    );
    // Replayed observations are the watches' alone.
    let dir2 = tempfile::tempdir().unwrap();
    let other = crate::test_support::seven_sessions(dir2.path());
    let mut replayed = obs("s1");
    replayed.replayed = true;
    heard(&other, &[replayed]);
    assert_eq!(other.digests.due_at(), None);
}
