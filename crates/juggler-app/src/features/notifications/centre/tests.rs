use super::*;
use juggler_core::storage::Storage;

const MIN: i64 = 60_000;
const NOW: i64 = 1_800_000_000_000;

fn storage() -> (tempfile::TempDir, Storage) {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    (dir, storage)
}

fn need(key: &str, kind: &str, title: &str) -> NeedInput {
    NeedInput {
        key: key.to_owned(),
        kind: kind.to_owned(),
        title: title.to_owned(),
        detail: "docs-site · Claude app".to_owned(),
        project_id: Some("proj-docs".to_owned()),
        project: Some("docs-site".to_owned()),
        target: NoticeTarget {
            session_id: Some(format!("sess-{key}")),
            ..NoticeTarget::default()
        },
        action: Some("open".to_owned()),
    }
}

fn quota(account: &str, left: i64, resets: i64) -> NewNotice {
    super::super::sources::low_notice(
        account,
        account,
        "5-hour",
        Some((100 - left) as f64),
        Some(resets),
        2,
        false,
    )
}

fn happened(key: &str, kind: &str) -> NewNotice {
    NewNotice {
        key: key.to_owned(),
        kind: kind.to_owned(),
        title: format!("{kind} happened"),
        ..NewNotice::default()
    }
}

fn clock(now: i64) -> Clock {
    Clock {
        now,
        utc_offset_hours: Some(0),
    }
}

fn first() -> PageRequest {
    PageRequest::first(50)
}

fn keys(page: &Page<NoticeInfo>) -> Vec<String> {
    page.items.iter().map(|row| row.title.clone()).collect()
}

#[test]
fn the_needs_you_list_is_kept_in_step_and_each_need_is_told_once() {
    let (_dir, storage) = storage();
    let fresh = sync_needs(
        &storage,
        &[need("a", "asks", "“Docs bot” needs your reply")],
        NOW,
    )
    .unwrap();
    assert_eq!(fresh.len(), 1);
    assert!(fresh[0].unread);
    // The same list again: nothing new to tell.
    assert!(
        sync_needs(
            &storage,
            &[need("a", "asks", "“Docs bot” needs your reply")],
            NOW + MIN
        )
        .unwrap()
        .is_empty()
    );
    // A second item joins; the first goes.
    let fresh = sync_needs(
        &storage,
        &[need("b", "review", "“Site deployment” is done — review it")],
        NOW + 2 * MIN,
    )
    .unwrap();
    assert_eq!(fresh.len(), 1);
    let open = page(&storage, "needs", &first(), NOW + 2 * MIN).unwrap();
    assert_eq!(keys(&open), ["“Site deployment” is done — review it"]);
    assert_eq!(summary(&storage, clock(NOW + 2 * MIN)).unwrap().count, 1);
}

#[test]
fn the_window_cant_write_what_it_doesnt_know() {
    let (_dir, storage) = storage();
    let mut odd = need("a", "asks", "A");
    odd.kind = "quota".to_owned();
    assert!(
        sync_needs(&storage, &[odd], NOW).is_err(),
        "an app kind from the window"
    );
    let mut keyless = need("a", "asks", "A");
    keyless.key = String::new();
    assert!(sync_needs(&storage, &[keyless], NOW).is_err());
    let many: Vec<NeedInput> = (0..501)
        .map(|n| need(&n.to_string(), "asks", "A"))
        .collect();
    assert!(sync_needs(&storage, &many, NOW).is_err());
    let mut long = need("a", "asks", &"x".repeat(1_000));
    long.detail = "y".repeat(2_000);
    let fresh = sync_needs(&storage, &[long], NOW).unwrap();
    assert_eq!(fresh[0].title.chars().count(), 300);
    assert_eq!(fresh[0].detail.chars().count(), 600);
}

#[test]
fn app_needs_are_not_ended_by_the_windows_list() {
    let (_dir, storage) = storage();
    assert!(
        open_need(&storage, &quota("Work Max", 7, NOW + 60 * MIN), NOW)
            .unwrap()
            .is_some()
    );
    sync_needs(&storage, &[], NOW).unwrap();
    let summary_now = summary(&storage, clock(NOW)).unwrap();
    assert_eq!(
        summary_now.count, 1,
        "the account almost out of room still needs you"
    );
    assert_eq!(summary_now.banners, 1);
    assert!(end_need(&storage, "quota:Work Max", NOW + MIN).unwrap());
    assert_eq!(summary(&storage, clock(NOW + MIN)).unwrap().count, 0);
}

/// D-49 (the maintainer's ruling): a snoozed need leaves the count until
/// its snooze ends; a muted one stays counted and only stops interrupting.
#[test]
fn snoozed_needs_leave_the_count_until_they_wake_muted_ones_stay() {
    let (_dir, storage) = storage();
    sync_needs(
        &storage,
        &[
            need("a", "asks", "A"),
            need("b", "stuck", "B"),
            need("c", "review", "C"),
        ],
        NOW,
    )
    .unwrap();
    snooze(&storage, "a", Snooze::Until(NOW + 60 * MIN), NOW).unwrap();
    snooze(&storage, "s:sess-a", Snooze::Until(NOW + 60 * MIN), NOW).unwrap();
    mute(&storage, "kind", "stuck", "", NOW).unwrap();
    let now = summary(&storage, clock(NOW)).unwrap();
    assert_eq!(now.count, 2, "the snoozed need is out of the count");
    assert_eq!(now.snoozed, 1);
    assert_eq!(now.muted, 1, "the muted need is still counted");
    assert_eq!(now.unread, 1, "only the review has a dot");
    let rows = page(&storage, "needs", &first(), NOW).unwrap().items;
    let snoozed = rows.iter().find(|row| row.subject == "a").unwrap();
    assert!(snoozed.snoozed && !snoozed.unread);
    assert_eq!(snoozed.snoozed_until_ms, Some(NOW + 60 * MIN));
    assert_eq!(open_needs(&storage, NOW, 10).len(), 2, "the menu-bar item lists what counts");
    assert!(!may_interrupt(&storage, "stuck", None));
    assert!(may_interrupt(&storage, "asks", None));
    assert!(session_snoozed(&storage, "sess-a", NOW));
    assert!(
        !session_snoozed(&storage, "sess-a", NOW + 61 * MIN),
        "a snooze ends"
    );
    // Its time come, it is back in the count by itself.
    assert_eq!(summary(&storage, clock(NOW + 61 * MIN)).unwrap().count, 3);
    // Snoozed until woken: out until the user wakes it.
    snooze(&storage, "b", Snooze::UntilWoken, NOW).unwrap();
    assert_eq!(summary(&storage, clock(NOW + 61 * MIN)).unwrap().count, 2);
    snooze(&storage, "b", Snooze::Wake, NOW).unwrap();
    assert_eq!(summary(&storage, clock(NOW + 61 * MIN)).unwrap().count, 3);
    // Mute this project: the project's needs stop interrupting.
    mute(&storage, "project", "proj-docs", "docs-site", NOW).unwrap();
    assert!(!may_interrupt(&storage, "review", Some("proj-docs")));
    assert_eq!(summary(&storage, clock(NOW)).unwrap().unread, 0);
    unmute(&storage, "project", "proj-docs").unwrap();
    unmute(&storage, "kind", "stuck").unwrap();
    assert_eq!(summary(&storage, clock(NOW)).unwrap().muted, 0);
    // A snooze must end in the future, and be of something.
    assert!(snooze(&storage, "b", Snooze::Until(NOW - 1), NOW).is_err());
    assert!(snooze(&storage, "", Snooze::Until(NOW + MIN), NOW).is_err());
    // An unknown kind or scope can't be muted.
    assert!(mute(&storage, "kind", "everything", "", NOW).is_err());
    assert!(mute(&storage, "account", "x", "", NOW).is_err());
}

#[test]
fn mute_labels_are_the_apps_own_words_for_a_kind() {
    let (_dir, storage) = storage();
    mute(&storage, "kind", "finished", "<b>ignored</b>", NOW).unwrap();
    let mutes = mutes(&storage, &first()).unwrap();
    assert_eq!(mutes.items[0].label, "Finished sessions");
    for (kind, label) in NEED_KINDS.iter().chain(APP_KINDS) {
        assert!(
            !label.contains('_') && !label.is_empty(),
            "a kind's name is plain words"
        );
        assert_eq!(kind_label(kind), Some(*label));
    }
}

#[test]
fn banners_are_due_quota_needs_at_most_three_and_dismissed_ones_stay_in_the_centre() {
    let (_dir, storage) = storage();
    for (n, account) in ["A", "B", "C", "D"].iter().enumerate() {
        open_need(
            &storage,
            &quota(account, 5, NOW + 60 * MIN),
            NOW + i64::try_from(n).unwrap(),
        )
        .unwrap();
    }
    let banners = page(&storage, "banners", &first(), NOW + 10).unwrap();
    assert_eq!(banners.items.len(), BANNERS_SHOWN);
    assert_eq!(
        banners.items[0].title, "D is almost out of room",
        "newest first"
    );
    dismiss(&storage, banners.items[0].id, NOW + 11).unwrap();
    let after = page(&storage, "banners", &first(), NOW + 12).unwrap();
    assert!(
        after
            .items
            .iter()
            .all(|row| row.title != "D is almost out of room")
    );
    let centre = page(&storage, "needs", &first(), NOW + 12).unwrap();
    assert!(
        centre
            .items
            .iter()
            .any(|row| row.title == "D is almost out of room" && row.dismissed)
    );
}

#[test]
fn earlier_pages_walk_thirty_days_newest_first() {
    let (_dir, storage) = storage();
    for n in 0..7 {
        record(
            &storage,
            &happened(&format!("finished:{n}"), "finished"),
            NOW + n * MIN,
        )
        .unwrap();
    }
    // Told once: the same happening again is nothing new.
    assert!(
        record(&storage, &happened("finished:3", "finished"), NOW + 9 * MIN)
            .unwrap()
            .is_none()
    );
    let mut seen = Vec::new();
    let mut request = PageRequest::first(3);
    loop {
        let page = page(&storage, "earlier", &request, NOW + 10 * MIN).unwrap();
        assert_eq!(page.total, Some(7));
        seen.extend(page.items.iter().map(|row| row.created_ms));
        match page.next_cursor {
            Some(cursor) => request = PageRequest::after(&cursor, 3),
            None => break,
        }
    }
    let expected: Vec<i64> = (0..7).rev().map(|n| NOW + n * MIN).collect();
    assert_eq!(seen, expected);
    // Thirty days on, the oldest are gone.
    let later = NOW + juggler_core::storage::NOTICE_KEEP_MS + 3 * MIN + 1;
    record(&storage, &happened("update:x", "update"), later).unwrap();
    let kept = page(&storage, "earlier", &first(), later).unwrap();
    assert_eq!(kept.total, Some(4));
    assert!(page(&storage, "nowhere", &first(), NOW).is_err());
}

#[test]
fn read_marks_one_or_all() {
    let (_dir, storage) = storage();
    sync_needs(
        &storage,
        &[need("a", "asks", "A"), need("b", "asks", "B")],
        NOW,
    )
    .unwrap();
    record(&storage, &happened("update:1", "update"), NOW).unwrap();
    assert_eq!(summary(&storage, clock(NOW)).unwrap().unread, 3);
    let rows = page(&storage, "needs", &first(), NOW).unwrap().items;
    read(&storage, Some(rows[0].id), NOW).unwrap();
    assert_eq!(summary(&storage, clock(NOW)).unwrap().unread, 2);
    read(&storage, None, NOW).unwrap();
    let after = summary(&storage, clock(NOW)).unwrap();
    assert_eq!(after.unread, 0);
    assert_eq!(after.count, 2, "reading changes no count");
}

#[test]
fn the_digest_counts_today_and_names_the_next_reset() {
    let (_dir, storage) = storage();
    let midnight = NOW - NOW.rem_euclid(86_400_000);
    record(
        &storage,
        &happened("finished:yesterday", "finished"),
        midnight - MIN,
    )
    .unwrap();
    for n in 0..4 {
        record(
            &storage,
            &happened(&format!("finished:{n}"), "finished"),
            midnight + n * MIN,
        )
        .unwrap();
    }
    record(
        &storage,
        &happened("restarted:x:1", "restarted"),
        midnight + 5 * MIN,
    )
    .unwrap();
    record(
        &storage,
        &happened("update:0.1.9", "update"),
        midnight + 6 * MIN,
    )
    .unwrap();
    open_need(
        &storage,
        &quota("Work Max", 7, midnight + 21 * 60 * MIN),
        midnight + 7 * MIN,
    )
    .unwrap();
    open_need(
        &storage,
        &quota("Personal", 9, midnight + 23 * 60 * MIN),
        midnight + 7 * MIN,
    )
    .unwrap();
    let digest = summary(&storage, clock(midnight + 8 * MIN)).unwrap().digest;
    assert_eq!(digest.finished, 4);
    assert_eq!(digest.restarted, 1);
    assert_eq!(digest.updates, 1);
    assert_eq!(digest.next_reset_account.as_deref(), Some("Work Max"));
    assert_eq!(digest.next_reset_ms, Some(midnight + 21 * 60 * MIN));
}

#[test]
fn the_quiet_hours_say_when_they_end() {
    let (_dir, storage) = storage();
    storage
        .app_settings()
        .set("alert_quiet_hours", "22-9")
        .unwrap();
    let midnight = NOW - NOW.rem_euclid(86_400_000);
    let at_night = summary(&storage, clock(midnight + 2 * 60 * MIN)).unwrap();
    let by_day = summary(&storage, clock(midnight + 12 * 60 * MIN)).unwrap();
    assert!(at_night.quiet);
    assert_eq!(at_night.quiet_until.as_deref(), Some("09:00"));
    assert!(!by_day.quiet);
    assert_eq!(by_day.quiet_until, None);
    let unknown = summary(
        &storage,
        Clock {
            now: midnight + 2 * 60 * MIN,
            utc_offset_hours: None,
        },
    )
    .unwrap();
    assert!(!unknown.quiet, "an unknown hour says nothing about quiet");
}

#[test]
fn the_version_moves_with_every_change() {
    let (_dir, storage) = storage();
    let start = summary(&storage, clock(NOW)).unwrap().version;
    sync_needs(&storage, &[need("a", "asks", "A")], NOW).unwrap();
    let one = summary(&storage, clock(NOW)).unwrap().version;
    assert_ne!(start, one);
    snooze(&storage, "a", Snooze::Until(NOW + MIN), NOW).unwrap();
    let two = summary(&storage, clock(NOW)).unwrap().version;
    assert_ne!(one, two);
    // The snooze ending moves it too (the views follow with no window).
    assert_ne!(two, summary(&storage, clock(NOW + 2 * MIN)).unwrap().version);
    assert_ne!(one, two);
    mute(&storage, "kind", "asks", "", NOW).unwrap();
    assert_ne!(two, summary(&storage, clock(NOW)).unwrap().version);
}

#[test]
fn local_midnight_follows_the_offset() {
    let day = 86_400_000;
    let utc_noon = 20_000 * day + 12 * 3_600_000;
    let tokyo = Clock {
        now: utc_noon,
        utc_offset_hours: Some(9),
    };
    assert_eq!(tokyo.local_hour(), Some(21));
    assert_eq!(tokyo.today_start(), 20_000 * day - 9 * 3_600_000);
    let unknown = Clock {
        now: utc_noon,
        utc_offset_hours: None,
    };
    assert_eq!(unknown.local_hour(), None);
    assert_eq!(unknown.today_start(), 20_000 * day);
}
