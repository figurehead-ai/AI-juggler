//! 0.5.2 G27 (the RW1 walk's bell): a session is named as its rail row
//! names it, never by its folder; the folder is where it runs ("in web").

use super::about_of;
use super::needs::Named;

const NOW: i64 = 1_800_000_000_000;

#[test]
fn a_notice_names_a_session_by_its_title_its_folder_only_as_in_folder() {
    let titled = about_of(
        &Named {
            title: Some("Change the date format in the API responses to ISO 8601"),
            cwd: "/Users/someone/repos/api",
            started_ms: NOW - 60_000,
            ..Named::default()
        },
        NOW,
    );
    assert_eq!(
        titled.name,
        "Change the date format in the API responses to ISO 8601"
    );
    assert_eq!(titled.place.as_deref(), Some("in api"));
    // Named by the user: that name.
    let labelled = about_of(
        &Named {
            label: Some("payments agent"),
            cwd: "/Users/someone/repos/api",
            started_ms: NOW - 60_000,
            ..Named::default()
        },
        NOW,
    );
    assert_eq!(labelled.name, "payments agent");
    // Nothing named it yet: its facts, the folder in its name, said once.
    let untitled = about_of(
        &Named {
            cwd: "/Users/someone/repos/web",
            started_ms: NOW - 60_000,
            ..Named::default()
        },
        NOW,
    );
    assert!(
        untitled.name.starts_with("web · started "),
        "{}",
        untitled.name
    );
    assert_eq!(untitled.place, None);
}

/// 0.5.2 G57 (RW1 F-RW2-5): a finish told before the session had a name
/// ("“api · started 2:54 AM” finished") reads by its name once it has
/// one, its folder then said as where it ran; the stored line is kept.
#[test]
fn a_bell_entry_names_its_session_as_it_is_called_now() {
    #![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::seven_sessions(dir.path());
    super::sources::finished(&state, "s1");
    let read = || {
        let now = crate::commands::now_millis();
        let mut page = {
            let storage = state.storage.lock().unwrap();
            super::centre::page(
                &storage,
                "earlier",
                &crate::paging::PageRequest::first(10),
                now,
            )
            .unwrap()
        };
        super::renamed(&state, &mut page.items);
        page.items
            .into_iter()
            .find(|row| row.kind == "finished")
            .unwrap()
    };
    let before = read();
    assert!(
        before.title.starts_with("“api · started "),
        "{}",
        before.title
    );
    {
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .set_label(
                &juggler_core::model::SessionId::new("s1"),
                Some("Add retries to the sync client"),
            )
            .unwrap();
    }
    let after = read();
    assert_eq!(after.title, "“Add retries to the sync client” finished");
    assert!(after.detail.starts_with("in api"), "{}", after.detail);
    // The same entry, renamed (nothing new written).
    assert_eq!(after.id, before.id);
}
