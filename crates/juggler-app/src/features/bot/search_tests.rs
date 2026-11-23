#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test: a failed step fails the test"
)]

//! AJ-E3824: Search finds each kind of record through Bot's look-up.

use std::sync::Arc;

use super::super::dto::{BotRowInfo, BotRowKind, BotState};
use super::super::world::World;
use super::super::world::tests_support::row;
use super::{SearchPoolInfo, SearchRecordsInfo, records_in, search_records_impl};
use crate::state::AppState;

/// 2026-11-19 12:00 UTC.
const NOW: i64 = 1_795_521_600_000;
const MIN: i64 = 60_000;

fn session(id: &str, title: &str, state: BotState) -> BotRowInfo {
    BotRowInfo {
        id: id.into(),
        project: Some("shop".into()),
        project_id: Some("p-shop".into()),
        at_ms: Some(NOW - 10 * MIN),
        ..row(BotRowKind::Session, title, Some(state))
    }
}

fn task(id: &str, title: &str, state: BotState, pool: &str) -> BotRowInfo {
    BotRowInfo {
        id: id.into(),
        pool: Some(pool.into()),
        pool_id: Some(format!("pool-{pool}")),
        at_ms: Some(NOW - 20 * MIN),
        ..row(BotRowKind::Task, title, Some(state))
    }
}

/// Generic names only.
fn world() -> World {
    World {
        now_ms: NOW,
        offset_hours: 0,
        sessions: vec![
            session("s-web", "web", BotState::AsksYou),
            session("s-login", "Fix the login redirect", BotState::Stuck),
            session("s-api", "api", BotState::Working),
            session("s-docs", "Draft the docs", BotState::Finished),
        ],
        tasks: vec![
            task(
                "t-export",
                "Wire the export button",
                BotState::Blocked,
                "Release",
            ),
            task(
                "t-notes",
                "Write the release notes",
                BotState::Ready,
                "Release",
            ),
            task("t-icons", "Draw the icons", BotState::Ready, "Design"),
        ],
        accounts: vec![
            BotRowInfo {
                used_percent: Some(54),
                ..row(BotRowKind::Account, "Team", None)
            },
            BotRowInfo {
                used_percent: Some(36),
                ..row(BotRowKind::Account, "Personal", None)
            },
        ],
        removed: Vec::new(),
    }
}

fn ids(rows: &[BotRowInfo]) -> Vec<&str> {
    rows.iter().map(|r| r.id.as_str()).collect()
}

#[test]
fn a_session_is_found_by_its_name() {
    let found = records_in("login", &world());
    assert_eq!(ids(&found.sessions), ["s-login"]);
    assert!(found.tasks.is_empty() && found.pools.is_empty() && found.usage.is_empty());
}

#[test]
fn a_session_is_found_by_its_state() {
    let found = records_in("What needs me?", &world());
    assert!(ids(&found.sessions).contains(&"s-web"));
    assert!(!ids(&found.sessions).contains(&"s-api"));
}

#[test]
fn a_task_is_found_with_its_pool() {
    let found = records_in("export", &world());
    assert_eq!(ids(&found.tasks), ["t-export"]);
    assert_eq!(
        found.pools,
        [SearchPoolInfo {
            id: "pool-Release".into(),
            name: "Release".into(),
            tasks: 1,
        }]
    );
    assert!(found.sessions.is_empty() && found.usage.is_empty());
}

#[test]
fn a_pool_is_found_by_its_name_once() {
    let found = records_in("release pool", &world());
    assert_eq!(found.pools.len(), 1, "{:?}", found.pools);
    assert_eq!(found.pools[0].name, "Release");
    assert_eq!(found.pools[0].tasks, 2);
    assert_eq!(ids(&found.tasks), ["t-export", "t-notes"]);
}

#[test]
fn usage_is_found_by_the_word_and_by_an_account_name() {
    let all = records_in("usage", &world());
    assert_eq!(all.usage.len(), 2);
    assert!(all.sessions.is_empty() && all.tasks.is_empty());
    let one = records_in("Personal", &world());
    assert_eq!(ids(&one.usage), ["Personal"]);
    assert_eq!(one.usage[0].used_percent, Some(36));
}

#[test]
fn words_that_ask_for_nothing_find_nothing() {
    for text in [
        "",
        " ",
        "a",
        "hi",
        "what can you do",
        "remember: ship on Friday",
    ] {
        assert_eq!(
            records_in(text, &world()),
            SearchRecordsInfo::default(),
            "{text:?}"
        );
    }
}

#[test]
fn what_no_record_holds_finds_nothing() {
    for text in ["zebra", "how long did the tests take", "what did it cost"] {
        assert_eq!(
            records_in(text, &world()),
            SearchRecordsInfo::default(),
            "{text:?}"
        );
    }
}

/// The command's own path: the records read now, nothing written.
#[test]
fn the_command_reads_the_records_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(
            dir.path().to_path_buf(),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    );
    assert_eq!(
        search_records_impl(&state, "What needs me?").unwrap(),
        SearchRecordsInfo::default()
    );
    assert_eq!(
        search_records_impl(&state, "hi").unwrap(),
        SearchRecordsInfo::default()
    );
    assert!(
        super::super::commands::bot_page_impl(&state, None, 20)
            .unwrap()
            .turns
            .is_empty()
    );
}

/// D-144: Search's Suggested entries (Bot's former shortcuts) each find
/// their records here.
#[test]
fn each_suggested_entry_finds_its_records() {
    let world = world();
    assert_eq!(
        ids(&records_in("What needs me", &world).sessions),
        ["s-web", "s-login"]
    );
    assert_eq!(
        ids(&records_in("What finished today", &world).sessions),
        ["s-docs"]
    );
    assert_eq!(records_in("Runway by account", &world).usage.len(), 2);
    assert_eq!(
        ids(&records_in("What\u{2019}s running", &world).sessions),
        ["s-api"]
    );
}
