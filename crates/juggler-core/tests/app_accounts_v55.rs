//! AJ-E3210 (v55, D-135): each App on its own sign-in's Account, and each
//! reading tagged with the Account it was read under.
//!
//! - The upgrade puts each CLI App on its folder's Account (the folder's
//!   link is the CLI's sign-in); a desktop App starts on none ("Account
//!   not known yet") until its records are read.
//! - An App's Account changes only when it differs (the change is told
//!   once by the caller).
//! - A folder's own display reads its own Account's readings (and those
//!   from before the tag); another Account's display on it reads only
//!   that Account's. A run of equal readings never crosses Accounts.
//! - The step is safe to run again (a database rolled back a version).

#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test (QUALITY.md §2): its fixtures unwrap and panic on a broken setup"
)]

use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, UsageSource, WindowKind};
use juggler_core::storage::{SignedInAs, SignedSamples, Storage};
use rusqlite::Connection;

/// 0.5's schema before AJ-E3210 (V54, the memory store).
const SCHEMA_BEFORE: usize = 54;

#[test]
fn the_upgrade_puts_each_cli_app_on_its_folders_account_and_desktop_ones_on_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    {
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        for (index, sql) in Storage::migrations().iter().enumerate().take(SCHEMA_BEFORE) {
            conn.execute_batch(sql).unwrap();
            conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
                .unwrap();
        }
        conn.execute_batch(
            "INSERT INTO subscriptions (id, vendor, identity_hash, created_at)
                 VALUES ('acct:anthropic:team', 'anthropic', 'team', 1);
             INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at, subscription_id)
                 VALUES ('shared', 'claude-code', 'aij-test', NULL, '/u/shared', 1, 'acct:anthropic:team'),
                        ('out', 'claude-code', 'Out', NULL, '/u/out', 1, NULL);
             INSERT INTO apps (id, kind, tool, folder_id, first_seen, last_seen)
                 VALUES ('cli:shared', 'cli', 'claude-code', 'shared', 1, 1),
                        ('cli:out', 'cli', 'claude-code', 'out', 1, 1);
             INSERT INTO apps (id, kind, tool, host_app_id, instance_dir, folder_id, first_seen, last_seen)
                 VALUES ('app:claude-app:/u/aij-test', 'desktop', 'claude-code', 'claude-app', '/u/aij-test', 'shared', 1, 1);
             INSERT INTO quota_samples (account, window, ts, used_pct, resets_at, confidence)
                 VALUES ('shared', 'five_hour', 5, 100.0, NULL, 'measured');",
        )
        .unwrap();
    }
    let storage = Storage::open(&path).unwrap();
    let accounts = storage.apps().accounts().unwrap();
    assert_eq!(
        accounts["cli:shared"].as_deref(),
        Some("acct:anthropic:team")
    );
    assert_eq!(accounts["cli:out"], None, "not signed in: none");
    assert_eq!(
        accounts["app:claude-app:/u/aij-test"], None,
        "a desktop App's own Account isn't known until its records are read"
    );
    // A reading from before stays the folder's own.
    let own = storage
        .usage()
        .sourced_samples_since_as(
            "shared",
            WindowKind::FiveHour,
            0,
            SignedInAs::Own(Some("acct:anthropic:team")),
        )
        .unwrap();
    assert_eq!(own.len(), 1);
}

fn sample(ts: i64, used: f64) -> QuotaSample {
    QuotaSample {
        ts,
        used_pct: used,
        resets_at: None,
        confidence: Confidence::Measured,
        held_until: None,
    }
}

#[test]
fn an_apps_account_changes_only_when_it_differs() {
    let storage = Storage::open_in_memory().unwrap();
    let id = storage
        .apps()
        .upsert_desktop("claude-code", "claude-app", Some("/u/aij-test"), 1)
        .unwrap();
    let team = storage
        .subscriptions()
        .upsert("anthropic", "team", 1)
        .unwrap();
    let personal = storage
        .subscriptions()
        .upsert("anthropic", "personal", 1)
        .unwrap();
    let apps = storage.apps();
    assert_eq!(apps.set_account(&id, Some(&personal)).unwrap(), Some(None));
    assert_eq!(
        apps.set_account(&id, Some(&personal)).unwrap(),
        None,
        "the same: no change"
    );
    assert_eq!(
        apps.set_account(&id, Some(&team)).unwrap(),
        Some(Some(personal.clone()))
    );
    assert_eq!(
        apps.accounts().unwrap()[&id].as_deref(),
        Some(team.as_str())
    );
    assert_eq!(apps.set_account("app:nope", Some(&team)).unwrap(), None);
}

#[test]
fn each_display_reads_only_its_own_accounts_readings() {
    let storage = Storage::open_in_memory().unwrap();
    let usage = storage.usage();
    let five = WindowKind::FiveHour;
    // The shared folder: the Team seat's meter (6%), the personal plan's
    // (100%, read while the app's sign-in was in the folder), and one
    // reading from before the tag.
    usage
        .insert_samples(&[("shared".into(), five, sample(1, 3.0))])
        .unwrap();
    usage
        .insert_signed_samples(&SignedSamples {
            windows: vec![
                (
                    "shared".into(),
                    five,
                    sample(10, 6.0),
                    UsageSource::DesktopApp,
                    Some("team".into()),
                ),
                (
                    "shared".into(),
                    five,
                    sample(20, 100.0),
                    UsageSource::DesktopApp,
                    Some("personal".into()),
                ),
            ],
            model_weeks: vec![(
                "shared".into(),
                "Fable".into(),
                sample(20, 90.0),
                UsageSource::UsageCache,
                Some("personal".into()),
            )],
        })
        .unwrap();
    let read = |who| {
        usage
            .sourced_samples_since_as("shared", five, 0, who)
            .unwrap()
            .iter()
            .map(|(s, _)| s.used_pct)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        read(SignedInAs::Own(Some("team"))),
        [3.0, 6.0],
        "never the personal plan's 100%"
    );
    assert_eq!(read(SignedInAs::Only("personal")), [100.0]);
    assert_eq!(
        read(SignedInAs::Own(None)),
        [3.0],
        "no Account: only what came before the tag"
    );
    assert_eq!(read(SignedInAs::Any).len(), 3);
    assert!(
        usage
            .model_week_samples_as("shared", SignedInAs::Own(Some("team")))
            .unwrap()
            .is_empty(),
        "the personal plan's model week isn't the Team seat's"
    );
    assert_eq!(
        usage.other_sign_ins("shared", Some("team"), 0).unwrap(),
        ["personal"]
    );
    assert!(
        usage
            .other_sign_ins("shared", Some("personal"), 30)
            .unwrap()
            .is_empty()
    );

    // The same reading under two Accounts is two runs, never one.
    usage
        .insert_signed_samples(&SignedSamples {
            windows: vec![
                (
                    "shared".into(),
                    five,
                    sample(30, 6.0),
                    UsageSource::DesktopApp,
                    Some("team".into()),
                ),
                (
                    "shared".into(),
                    five,
                    sample(31, 100.0),
                    UsageSource::DesktopApp,
                    Some("personal".into()),
                ),
                (
                    "shared".into(),
                    five,
                    sample(32, 6.0),
                    UsageSource::DesktopApp,
                    Some("team".into()),
                ),
            ],
            model_weeks: vec![],
        })
        .unwrap();
    assert_eq!(read(SignedInAs::Only("personal")).last(), Some(&100.0));
    assert_eq!(read(SignedInAs::Own(Some("team"))).last(), Some(&6.0));
}

#[test]
fn the_step_is_safe_to_run_again_and_keeps_what_was_set() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let id = {
        let storage = Storage::open(&path).unwrap();
        let id = storage
            .apps()
            .upsert_desktop("claude-code", "claude-app", Some("/u/aij-test"), 1)
            .unwrap();
        let personal = storage
            .subscriptions()
            .upsert("anthropic", "personal", 1)
            .unwrap();
        storage.apps().set_account(&id, Some(&personal)).unwrap();
        id
    };
    Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", i64::try_from(SCHEMA_BEFORE).unwrap())
        .unwrap();
    let storage = Storage::open(&path).unwrap();
    assert!(storage.apps().accounts().unwrap()[&id].is_some());
}
