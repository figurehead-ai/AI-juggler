//! AJ-E3004 R1: pool membership resolves from maps kept in memory and
//! rebuilt only when what it's resolved from changes (the `memberships`
//! stamp, v48), never by re-reading on every call.

#![allow(clippy::unwrap_used, reason = "a test")]

use std::sync::Arc;

use juggler_core::storage::{MemberRef, PoolRole, Storage};

#[test]
fn the_maps_are_kept_until_a_pool_or_member_changes() {
    let storage = Storage::open_in_memory().unwrap();
    let pool = storage.pools().create("Checkout", None).unwrap();
    let first = storage.memberships().unwrap();
    let again = storage.memberships().unwrap();
    assert!(
        Arc::ptr_eq(&first, &again),
        "nothing changed: the same maps"
    );

    // A member joins: rebuilt.
    storage
        .pools()
        .join(
            &pool.id,
            &MemberRef::Tool("codex".into()),
            &[],
            PoolRole::Member,
            None,
        )
        .unwrap();
    let joined = storage.memberships().unwrap();
    assert!(!Arc::ptr_eq(&again, &joined), "a member joined");
    assert!(Arc::ptr_eq(&joined, &storage.memberships().unwrap()));

    // Sealed: the `pools` stamp doesn't move for it, this one does.
    storage.pools().set_sealed(&pool.id, true).unwrap();
    let sealed = storage.memberships().unwrap();
    assert!(
        !Arc::ptr_eq(&joined, &sealed),
        "a pool sealed changes who is in"
    );

    // A rename changes no membership but rebuilds (correct, never stale).
    storage.pools().rename(&pool.id, "Payments").unwrap();
    assert!(!Arc::ptr_eq(&sealed, &storage.memberships().unwrap()));
}
