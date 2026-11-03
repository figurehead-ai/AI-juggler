//! D-62 (AJ-E3202 req. 4) storage: plan, renewal and fee move to the
//! Account where its folders agree; where they disagree each folder keeps
//! its own until the user chooses once; a chosen term is never asked again
//! and the folders' own values are kept, not written.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use juggler_core::model::{Account, AccountId, HarnessId, PlanTier};
use juggler_core::storage::{Storage, Term, Terms, fee_key};
use rusqlite::Connection;

fn folder(id: &str, plan: Option<&str>, renewal: Option<&str>) -> Account {
    Account {
        id: AccountId::new(id),
        harness: HarnessId::new("claude-code"),
        label: id.into(),
        plan: plan.map(PlanTier::new),
        state_dir: format!("/tmp/aij-test/{id}").into(),
        cleanup_opt_in: false,
        renewal_date: renewal.map(str::to_owned),
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

/// A folder: its id, plan, renewal and fee.
type Seed<'a> = (&'a str, Option<&'a str>, Option<&'a str>, Option<f64>);

/// Folders (each with its plan, renewal and fee) signed in to one
/// Account; its id.
fn on_one_account(storage: &Storage, folders: &[Seed<'_>]) -> String {
    let account = storage
        .subscriptions()
        .upsert("anthropic", "aaaa", 1)
        .unwrap();
    for (id, plan, renewal, fee) in folders {
        storage
            .accounts()
            .insert(&folder(id, *plan, *renewal))
            .unwrap();
        if let Some(fee) = fee {
            storage
                .app_settings()
                .set(&fee_key(id), &fee.to_string())
                .unwrap();
        }
        storage
            .subscriptions()
            .set_folder(id, Some(&account))
            .unwrap();
    }
    account
}

#[test]
fn v41_adds_one_column_and_changes_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    {
        let storage = Storage::open(&db).unwrap();
        storage
            .accounts()
            .insert(&folder("work", Some("max"), Some("2026-11-12")))
            .unwrap();
        storage.app_settings().set(&fee_key("work"), "200").unwrap();
    }
    // Back to v40, as an upgrade finds it: v41 (and v42) run on the next
    // open.
    Connection::open(&db)
        .unwrap()
        .execute_batch(
            "ALTER TABLE accounts DROP COLUMN terms_moved;
             DROP TRIGGER apps_colour_slot;
             DROP TRIGGER subscriptions_colour_slot;
             ALTER TABLE apps DROP COLUMN colour_slot;
             ALTER TABLE subscriptions DROP COLUMN colour_slot;
             PRAGMA user_version = 40;",
        )
        .unwrap();
    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    assert_eq!(storage.user_version().unwrap(), 42);
    // Nothing moved by the migration: the folder reads as it did.
    assert_eq!(
        storage.terms().effective("work").unwrap(),
        Terms {
            plan: Some("max".into()),
            renewal_date: Some("2026-11-12".into()),
            fee_usd: Some(200.0),
        }
    );
    let account = storage.accounts().get(&AccountId::new("work")).unwrap();
    assert_eq!(account.renewal_date.as_deref(), Some("2026-11-12"));
}

#[test]
fn where_the_folders_agree_the_terms_move_to_the_account() {
    let storage = Storage::open_in_memory().unwrap();
    let account = on_one_account(
        &storage,
        &[
            ("work", Some("max-20x"), Some("2026-11-12"), Some(200.0)),
            // Says the same plan in other words; says no renewal or fee.
            ("work-2", Some("Max 20x"), None, None),
        ],
    );
    assert!(storage.terms().settle(&account).unwrap().is_empty());
    let terms = storage.terms().account(&account).unwrap().unwrap();
    assert_eq!(terms.plan.as_deref(), Some("max-20x"));
    assert_eq!(terms.renewal_date.as_deref(), Some("2026-11-12"));
    assert_eq!(terms.fee_usd, Some(200.0));
    // Both folders read the Account's now.
    assert_eq!(storage.terms().effective("work-2").unwrap(), terms);
    // Nothing is lost: each folder keeps its own value.
    assert_eq!(
        storage
            .terms()
            .own("work-2")
            .unwrap()
            .unwrap()
            .plan
            .as_deref(),
        Some("Max 20x")
    );
    // Moved once: set on the Account later, a second read doesn't undo it.
    storage.terms().set(&account, Term::Renewal, None).unwrap();
    assert!(storage.terms().settle(&account).unwrap().is_empty());
    assert_eq!(
        storage.terms().effective("work").unwrap().renewal_date,
        None
    );
}

#[test]
fn where_they_disagree_each_keeps_its_own_until_the_user_chooses_once() {
    let storage = Storage::open_in_memory().unwrap();
    let account = on_one_account(
        &storage,
        &[
            ("work", Some("max"), Some("2026-11-12"), Some(200.0)),
            ("work-2", Some("max"), Some("2026-12-03"), Some(200.0)),
        ],
    );
    let choices = storage.terms().settle(&account).unwrap();
    assert_eq!(choices.len(), 1, "only the renewal differs");
    let renewal = &choices[0];
    assert_eq!(renewal.term, Term::Renewal);
    let said: Vec<(&str, Vec<&str>)> = renewal
        .options
        .iter()
        .map(|option| {
            (
                option.value.as_str(),
                option.folders.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        said,
        [("2026-11-12", vec!["work"]), ("2026-12-03", vec!["work-2"])]
    );
    // The rest moved; the renewal didn't: each folder still reads its own.
    let terms = storage.terms().account(&account).unwrap().unwrap();
    assert_eq!(
        (terms.plan.as_deref(), terms.renewal_date),
        (Some("max"), None)
    );
    assert_eq!(
        storage
            .terms()
            .effective("work-2")
            .unwrap()
            .renewal_date
            .as_deref(),
        Some("2026-12-03")
    );
    // Asked again on every read until chosen.
    assert_eq!(storage.terms().settle(&account).unwrap(), choices);

    // Chosen once: stored on the Account, every folder reads it, never asked again.
    storage
        .terms()
        .set(&account, Term::Renewal, Some("2026-12-03"))
        .unwrap();
    assert!(storage.terms().settle(&account).unwrap().is_empty());
    for id in ["work", "work-2"] {
        assert_eq!(
            storage
                .terms()
                .effective(id)
                .unwrap()
                .renewal_date
                .as_deref(),
            Some("2026-12-03")
        );
    }
    // The folders' own values are kept as they were, and no longer written:
    // setting through a folder sets the Account.
    storage
        .terms()
        .set_for_folder("work", Term::Fee, Some("100"))
        .unwrap();
    assert_eq!(
        storage.terms().account(&account).unwrap().unwrap().fee_usd,
        Some(100.0)
    );
    assert_eq!(
        storage.terms().own("work").unwrap().unwrap().fee_usd,
        Some(200.0)
    );
    assert_eq!(
        storage
            .terms()
            .own("work")
            .unwrap()
            .unwrap()
            .renewal_date
            .as_deref(),
        Some("2026-11-12")
    );
    assert_eq!(
        storage.terms().effective("work-2").unwrap().fee_usd,
        Some(100.0)
    );
}

#[test]
fn a_folder_that_joins_later_with_another_value_asks_again() {
    let storage = Storage::open_in_memory().unwrap();
    let account = on_one_account(&storage, &[("work", None, Some("2026-11-12"), None)]);
    assert!(storage.terms().settle(&account).unwrap().is_empty());
    storage
        .accounts()
        .insert(&folder("late", None, Some("2026-11-20")))
        .unwrap();
    storage
        .subscriptions()
        .set_folder("late", Some(&account))
        .unwrap();
    let choices = storage.terms().settle(&account).unwrap();
    assert_eq!(choices.len(), 1);
    assert!(
        choices[0].options[0].account,
        "the Account's own value first"
    );
    assert_eq!(choices[0].options[0].value, "2026-11-12");
    assert_eq!(choices[0].options[1].folders, ["late"]);
    // Until chosen, the newcomer keeps its date; the first folder the Account's.
    assert_eq!(
        storage
            .terms()
            .effective("late")
            .unwrap()
            .renewal_date
            .as_deref(),
        Some("2026-11-20")
    );
}

#[test]
fn a_folder_with_no_account_keeps_its_own_terms_and_is_written_itself() {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .accounts()
        .insert(&folder("out", Some("pro"), None))
        .unwrap();
    storage
        .terms()
        .set_for_folder("out", Term::Renewal, Some("2026-11-30"))
        .unwrap();
    storage
        .terms()
        .set_for_folder("out", Term::Fee, Some("20"))
        .unwrap();
    assert_eq!(
        storage.terms().effective("out").unwrap(),
        Terms {
            plan: Some("pro".into()),
            renewal_date: Some("2026-11-30".into()),
            fee_usd: Some(20.0),
        }
    );
    storage
        .terms()
        .set_for_folder("out", Term::Fee, None)
        .unwrap();
    assert_eq!(storage.terms().effective("out").unwrap().fee_usd, None);
    assert_eq!(storage.terms().effective("nope").unwrap(), Terms::default());
}
