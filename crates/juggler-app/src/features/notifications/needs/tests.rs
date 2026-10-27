#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn a_plain_branch_names_no_work() {
    assert_eq!(telling(Some("main")), None);
    assert_eq!(telling(Some("Master")), None);
    assert_eq!(telling(Some("fix/limits")), Some("fix/limits"));
    assert_eq!(telling(None), None);
}

#[test]
fn a_need_quotes_its_session_as_the_rail_does() {
    assert_eq!(quoted("asks", "payments-api"), "“payments-api” needs your reply");
    assert_eq!(quoted("stuck", "web"), "“web” may be stuck");
    assert_eq!(quoted("limit", "web"), "“web” hit its usage limit");
    assert_eq!(quoted("failed", "web"), "“web” stopped with an error");
    assert_eq!(quoted("review", "web"), "“web” is done — review it");
}

#[test]
fn reasons_read_as_one_sentence() {
    assert_eq!(
        reason_sentence(&["memory is low".to_owned(), "swap is busy".to_owned()]),
        "Memory is low; swap is busy"
    );
    assert_eq!(reason_sentence(&["macOS is compressing memory".to_owned()]), "macOS is compressing memory");
    assert_eq!(reason_sentence(&[]), "");
}

#[test]
fn the_month_starts_on_the_local_first() {
    let day = 86_400_000;
    // 2027-03-15 12:00 UTC: 20_892 days after 1970-01-01.
    let mid_march = 20_892 * day + 12 * 3_600_000;
    let first = month_start(mid_march, 0);
    assert_eq!(first, (20_892 - 14) * day);
    // Two hours ahead of UTC: local midnight is 22:00 UTC the day before.
    assert_eq!(month_start(mid_march, 2), (20_892 - 14) * day - 2 * 3_600_000);
}

#[test]
fn with_nothing_to_read_nothing_needs_the_user() {
    let (needs, open) = work_out(&NeedSources::default(), &HashSet::new());
    assert!(needs.is_empty());
    assert_eq!(open, OpenNowInput::default());
}
