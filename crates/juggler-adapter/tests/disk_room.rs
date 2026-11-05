//! 0.3.9: a switch never fills a disk. What it writes in full (a copy
//! that can't be a clone, a fork, an unpacked rollout) checks the room
//! first and again as it goes, and stops while 4 GB is still free.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use juggler_adapter::handover::{DISK_RESERVE, RoomWriter, ensure_room};
use juggler_platform::room::set_free_space_on_this_thread;

const GB: u64 = 1_000_000_000;

#[test]
fn a_write_that_would_leave_less_than_the_reserve_is_refused_saying_how_much() {
    let dir = tempfile::tempdir().unwrap();
    set_free_space_on_this_thread(Some(|_: &Path| 10 * GB));
    let refused =
        ensure_room(dir.path(), 8 * GB).expect_err("8 GB into 10 GB free leaves under 4 GB");
    assert_eq!(refused.kind(), std::io::ErrorKind::StorageFull);
    assert_eq!(
        refused.to_string(),
        "not enough free space: needs 2000 MB more"
    );
    assert!(
        ensure_room(dir.path(), 6 * GB).is_ok(),
        "6 GB leaves exactly the reserve"
    );
    assert!(ensure_room(dir.path(), 6 * GB + 1).is_err());
    set_free_space_on_this_thread(None);
}

/// Free space that falls as something else fills the disk.
static FREE: AtomicU64 = AtomicU64::new(0);

#[test]
fn a_copy_stops_while_the_reserve_is_still_free_when_the_disk_fills_meanwhile() {
    let dir = tempfile::tempdir().unwrap();
    FREE.store(DISK_RESERVE + 200 * 1024 * 1024, Ordering::SeqCst);
    set_free_space_on_this_thread(Some(|_: &Path| {
        // Every check sees 100 MB less: another program is writing.
        FREE.fetch_sub(100 * 1024 * 1024, Ordering::SeqCst)
    }));
    let mut out = RoomWriter::new(std::io::sink(), dir.path());
    let chunk = vec![0u8; 1024 * 1024];
    let mut wrote = 0u64;
    let stopped = loop {
        match out.write_all(&chunk) {
            Ok(()) => wrote += chunk.len() as u64,
            Err(error) => break error,
        }
        assert!(wrote < 10 * GB, "it never stopped");
    };
    set_free_space_on_this_thread(None);
    assert_eq!(stopped.kind(), std::io::ErrorKind::StorageFull);
    assert!(
        wrote <= 256 * 1024 * 1024,
        "it stopped within a few checks of the disk running low: {wrote}"
    );
}
