//! AJ-E1501 storage: a removed account leaves the list but stays
//! readable by id (its sessions keep a name); defaults per project; the
//! audit rows are append-only and ordered.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use juggler_core::model::{Account, AccountId, HarnessId};
use juggler_core::storage::Storage;

fn account(id: &str, label: &str) -> Account {
    Account {
        id: AccountId::new(id),
        harness: HarnessId::new("claude-code"),
        label: label.into(),
        plan: None,
        state_dir: format!("/tmp/aij-test/{id}").into(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

#[test]
fn a_removed_account_leaves_the_list_but_keeps_its_row() {
    let storage = Storage::open_in_memory().unwrap();
    storage.accounts().insert(&account("a", "A")).unwrap();
    storage.accounts().insert(&account("b", "B")).unwrap();
    storage
        .accounts()
        .mark_removed(&AccountId::new("a"))
        .unwrap();

    let listed: Vec<String> = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .map(|a| a.label)
        .collect();
    assert_eq!(listed, ["B"]);
    assert_eq!(storage.accounts().list_all().unwrap().len(), 2);
    assert_eq!(
        storage.accounts().get(&AccountId::new("a")).unwrap().label,
        "A"
    );
    assert!(storage.accounts().is_removed(&AccountId::new("a")).unwrap());
    assert!(!storage.accounts().is_removed(&AccountId::new("b")).unwrap());
    assert_eq!(
        storage
            .accounts()
            .removed_ids()
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>(),
        ["a"]
    );
    // Twice is refused (nothing to forget), and an unknown id too.
    assert!(
        storage
            .accounts()
            .mark_removed(&AccountId::new("a"))
            .is_err()
    );
    assert!(
        storage
            .accounts()
            .is_removed(&AccountId::new("zz"))
            .is_err()
    );
}

#[test]
fn a_projects_default_account_is_set_cleared_and_follows_a_removal() {
    let storage = Storage::open_in_memory().unwrap();
    storage.accounts().insert(&account("a", "A")).unwrap();
    let project = storage
        .projects()
        .touch(std::path::Path::new("/tmp/aij-test/repo"), None)
        .unwrap();
    assert_eq!(project.default_account, None);
    storage
        .projects()
        .set_default_account(&project.id, Some(&AccountId::new("a")))
        .unwrap();
    assert_eq!(
        storage.projects().get(&project.id).unwrap().default_account,
        Some(AccountId::new("a"))
    );
    assert_eq!(
        storage
            .projects()
            .clear_default_account(&AccountId::new("a"))
            .unwrap(),
        1
    );
    assert_eq!(
        storage.projects().get(&project.id).unwrap().default_account,
        None
    );
}

#[test]
fn audit_rows_are_kept_in_order() {
    let storage = Storage::open_in_memory().unwrap();
    let id = AccountId::new("a");
    storage
        .account_audit()
        .record(&id, "rename", Some("«a» → «b»"), "user")
        .unwrap();
    storage
        .account_audit()
        .record(&id, "remove", None, "user")
        .unwrap();
    let rows = storage.account_audit().list_for(&id).unwrap();
    assert_eq!(
        rows.iter().map(|r| r.action.as_str()).collect::<Vec<_>>(),
        ["rename", "remove"]
    );
    assert_eq!(rows[0].detail.as_deref(), Some("«a» → «b»"));
    assert!(
        storage
            .account_audit()
            .list_for(&AccountId::new("other"))
            .unwrap()
            .is_empty()
    );
}
