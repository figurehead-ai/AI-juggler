//! D-62: limits belong to the Account, so its quota notices do too. Two
//! folders signed in to one Account raise one notice, named by the
//! Account ("Claude Max 20x"), from its newest reading, acting on one of
//! its folders; a folder's own need from before gives way to it; its
//! room coming back is told once, by the Account. A folder no Account
//! lists keeps its own name.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::ClaudeCodeAdapter;
use juggler_app::commands::{self, AlertEventInfo};
use juggler_app::features::notifications::{centre, sources};
use juggler_app::features::subscriptions;
use juggler_app::state::AppState;
use juggler_core::model::{Confidence, Timestamp};
use juggler_core::quota::{QuotaSample, WindowKind};

#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn app_state(dir: &Path) -> Arc<AppState> {
    let bin: PathBuf = dir.join("claude");
    stand_in::write(&bin, "#!/bin/sh\nexit 0\n");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(bin)));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// A Claude folder named `name`, signed in as `subject` (or not at all).
fn folder(state: &Arc<AppState>, name: &str, subject: Option<&str>) -> String {
    let made = commands::create_account(state, "claude-code", name, None).unwrap();
    let dir = commands::find_account_arc(state, &made.id)
        .unwrap()
        .state_dir;
    if let Some(subject) = subject {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(".claude.json"),
            format!(
                r#"{{"oauthAccount":{{"accountUuid":"{subject}","organizationUuid":"org-{subject}","emailAddress":"you@example.invalid"}}}}"#
            ),
        )
        .unwrap();
    }
    made.id
}

/// A Measured 5-hour reading; its window resets 3 h after `window_from`.
fn reading_in(state: &Arc<AppState>, folder: &str, ts: i64, used: f64, window_from: i64) {
    state
        .storage
        .lock()
        .unwrap()
        .usage()
        .insert_sample(
            folder,
            WindowKind::FiveHour,
            &QuotaSample {
                ts,
                used_pct: used,
                resets_at: Some(window_from + 3 * 3_600_000),
                confidence: Confidence::Measured,
            },
        )
        .unwrap();
}

fn reading(state: &Arc<AppState>, folder: &str, ts: i64, used: f64) {
    reading_in(state, folder, ts, used, ts);
}

fn low(folder: &str) -> AlertEventInfo {
    AlertEventInfo {
        kind: "actionable".into(),
        account: folder.into(),
        text: String::new(),
        at_ms: Timestamp::now().millis(),
        quota: None,
    }
}

fn needs(state: &Arc<AppState>) -> Vec<centre::NoticeInfo> {
    centre::open_needs(
        &state.storage.lock().unwrap(),
        Timestamp::now().millis(),
        20,
    )
}

/// Two folders on one plan, named "Claude Max 20x" (its plan, set once).
fn one_account_two_folders(state: &Arc<AppState>) -> (String, String, String) {
    let work = folder(state, "Work Max", Some("subject-a"));
    let personal = folder(state, "Personal", Some("subject-a"));
    let id = subscriptions::folder_accounts(state)[&work].id.clone();
    subscriptions::terms::set_impl(state, &id, "plan", Some("max-20x")).unwrap();
    (work, personal, id)
}

#[test]
fn two_folders_on_one_account_raise_one_notice_named_by_the_account() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let (work, personal, id) = one_account_two_folders(&state);
    let account = &subscriptions::folder_accounts(&state)[&personal];
    assert_eq!(account.name, "Claude Max 20x");
    assert_eq!(account.folders, [work.clone(), personal.clone()]);

    let now = Timestamp::now().millis();
    // Personal's reading is the newest: the Account's runway is its.
    reading(&state, &work, now - 120_000, 93.0);
    reading(&state, &personal, now - 60_000, 95.0);
    sources::quota_pass(&state, &[low(&work), low(&personal)]);

    let open = needs(&state);
    assert_eq!(open.len(), 1, "one notice for one Account: {open:?}");
    let notice = &open[0];
    assert_eq!(notice.title, "Claude Max 20x is almost out of room");
    assert_eq!(notice.detail, "5% of its 5-hour window left");
    assert_eq!(notice.facts.account.as_deref(), Some("Claude Max 20x"));
    assert_eq!(notice.subject, format!("quota:{id}"));
    // No session open on either: it acts on the folder whose reading it shows.
    assert_eq!(notice.target.account_id.as_deref(), Some(personal.as_str()));
    for words in [&notice.title, &notice.detail] {
        assert!(!words.contains("Work Max") && !words.contains("Personal"));
    }
    // The digest's next reset names the Account too.
    let summary = centre::summary(
        &state.storage.lock().unwrap(),
        centre::Clock {
            now,
            utc_offset_hours: None,
        },
    )
    .unwrap();
    assert_eq!(
        summary.digest.next_reset_account.as_deref(),
        Some("Claude Max 20x")
    );

    // A later pass with no new alert keeps the one need.
    sources::quota_pass(&state, &[]);
    assert_eq!(needs(&state).len(), 1);

    // Room again on both (a new window): the need ends and the Account's
    // reset is told once.
    let next = now + 4 * 3_600_000;
    reading_in(&state, &work, now, 10.0, next);
    reading_in(&state, &personal, now + 1, 10.0, next);
    sources::quota_pass(&state, &[]);
    assert!(needs(&state).is_empty(), "{:?}", needs(&state));
    let earlier = centre::page(
        &state.storage.lock().unwrap(),
        "earlier",
        &juggler_app::paging::PageRequest::first(50),
        Timestamp::now().millis(),
    )
    .unwrap()
    .items;
    let resets: Vec<&str> = earlier
        .iter()
        .filter(|notice| notice.kind == "reset")
        .map(|notice| notice.title.as_str())
        .collect();
    assert_eq!(
        resets,
        ["Claude Max 20x has room again — its 5-hour window reset"]
    );
}

/// Before 0.4 each folder had a need of its own: the Account's takes over.
#[test]
fn a_folders_own_need_from_before_gives_way_to_its_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let (work, _personal, id) = one_account_two_folders(&state);
    let now = Timestamp::now().millis();
    {
        let old = sources::low_notice(
            &sources::QuotaSubject {
                id: work.clone(),
                name: "Work Max".into(),
                folder: work.clone(),
            },
            "5-hour",
            Some(93.0),
            None,
            0,
            false,
        );
        centre::open_need(&state.storage.lock().unwrap(), &old, now).unwrap();
    }
    reading(&state, &work, now, 93.0);
    sources::quota_pass(&state, &[]);
    let open = needs(&state);
    assert_eq!(open.len(), 1, "{open:?}");
    assert_eq!(open[0].subject, format!("quota:{id}"));
    assert_eq!(open[0].title, "Claude Max 20x is almost out of room");
}

/// A folder no Account lists (not signed in) keeps its own name.
#[test]
fn a_folder_no_account_lists_keeps_its_own_name() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let out = folder(&state, "Spare", None);
    let now = Timestamp::now().millis();
    reading(&state, &out, now, 93.0);
    sources::quota_pass(&state, &[low(&out)]);
    let open = needs(&state);
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].title, "Spare is almost out of room");
    assert_eq!(open[0].subject, format!("quota:{out}"));
    assert_eq!(open[0].target.account_id.as_deref(), Some(out.as_str()));
}

/// D-62: the alert pass's own words (what a paired Mac shows as it is,
/// "Mac mini · …", and a queued resume coming due) name the Account the
/// folder is signed in to; a folder no Account lists keeps its own name.
#[test]
fn the_alert_pass_names_the_account_and_a_lone_folder_itself() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let (work, _personal, _id) = one_account_two_folders(&state);
    let spare = folder(&state, "Spare", None);
    let now = Timestamp::now().millis();
    reading(&state, &work, now - 1_000, 95.0);
    reading(&state, &spare, now - 1_000, 95.0);
    {
        let storage = state.storage.lock().unwrap();
        storage
            .usage()
            .queue_reset("rq-work", "conv:c-work", &work, now - 1)
            .unwrap();
        storage
            .usage()
            .queue_reset("rq-spare", "conv:c-spare", &spare, now - 1)
            .unwrap();
    }
    let events = commands::alerts_tick_inner(&state, Some(12));
    let find = |kind: &str, folder: &str| {
        events
            .iter()
            .find(|event| event.kind == kind && event.account == folder)
            .unwrap_or_else(|| panic!("no {kind} for {folder}: {events:?}"))
    };
    let alert = find("actionable", &work);
    assert!(
        alert
            .text
            .starts_with("Claude Max 20x has 5% left in its 5-hour window"),
        "{}",
        alert.text
    );
    assert!(!alert.text.contains("Work Max"), "{}", alert.text);
    assert_eq!(
        alert.quota.as_ref().unwrap().account_label,
        "Claude Max 20x"
    );
    let lone = find("actionable", &spare);
    assert!(lone.text.starts_with("Spare has 5% left"), "{}", lone.text);
    assert_eq!(lone.quota.as_ref().unwrap().account_label, "Spare");
    assert_eq!(
        find("reset-reached", &work).text,
        "Claude Max 20x has reset: its session can pick up again"
    );
    assert_eq!(
        find("reset-reached", &spare).text,
        "Spare has reset: its session can pick up again"
    );
}

/// D-62: a renewal is the Account's, so the digest's line names it; a
/// folder no Account lists keeps its own name.
#[test]
fn a_renewal_line_names_the_account_and_a_lone_folder_itself() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let (work, _personal, _id) = one_account_two_folders(&state);
    let spare = folder(&state, "Spare", None);
    let digest = |folder: &str| AlertEventInfo {
        kind: "digest".into(),
        account: folder.into(),
        text: "Renews in 6 days — check your plan".into(),
        at_ms: Timestamp::now().millis(),
        quota: None,
    };
    sources::quota_pass(&state, &[digest(&work), digest(&spare)]);
    let earlier = centre::page(
        &state.storage.lock().unwrap(),
        "earlier",
        &juggler_app::paging::PageRequest::first(50),
        Timestamp::now().millis(),
    )
    .unwrap()
    .items;
    let mut renewals: Vec<(&str, Option<&str>)> = earlier
        .iter()
        .filter(|notice| notice.kind == "renewal")
        .map(|notice| (notice.title.as_str(), notice.facts.account.as_deref()))
        .collect();
    renewals.sort_unstable();
    assert_eq!(
        renewals,
        [
            (
                "Claude Max 20x: Renews in 6 days — check your plan",
                Some("Claude Max 20x")
            ),
            ("Spare: Renews in 6 days — check your plan", Some("Spare")),
        ]
    );
}
