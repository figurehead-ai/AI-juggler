//! The schema is frozen once shipped: a host applies it as one of its own
//! migrations, and a migration's text never changes. A change here is a
//! new `SCHEMA_V2` batch, never an edit of V1.

#![allow(
    missing_docs,
    clippy::expect_used,
    reason = "a test: a failed step fails the test"
)]

fn fnv(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[test]
fn schema_v1_is_frozen() {
    assert_eq!(
        format!("{:016x}", fnv(gyrus::SCHEMA_V1)),
        "ad1e730059eb32b8",
        "SCHEMA_V1 changed: add a new batch instead"
    );
}

#[test]
fn schema_v1_installs_twice() {
    #[allow(clippy::disallowed_methods, reason = "gyrus's own in-memory database")]
    let conn = rusqlite::Connection::open_in_memory().expect("open");
    gyrus::schema::install(&conn).expect("first");
    gyrus::schema::install(&conn).expect("again");
    assert!(gyrus::schema::installed(&conn).expect("installed"));
    let buckets: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name LIKE 'gyrus_fts___'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(buckets, gyrus::FTS_BUCKETS);
}
