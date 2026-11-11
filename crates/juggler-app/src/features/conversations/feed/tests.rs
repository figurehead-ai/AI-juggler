//! The feeds (ADR-0024): follow, changes, sharing, leases, eviction,
//! resets, redaction and the batching of changes. Synthetic transcripts
//! only.

use std::io::Write as _;
use std::sync::Mutex;

use super::watch::{Debounce, SETTLE_MAX};
use super::*;

#[derive(Default)]
struct Collect {
    got: Mutex<Vec<ConversationDeltaInfo>>,
    closed: bool,
}

impl DeltaSink for Collect {
    fn send(&self, delta: ConversationDeltaInfo) -> bool {
        self.got.lock().unwrap().push(delta);
        !self.closed
    }
}

impl Collect {
    fn take(&self) -> Vec<ConversationDeltaInfo> {
        std::mem::take(&mut *self.got.lock().unwrap())
    }
}

fn line(kind: &str, words: &str) -> String {
    serde_json::json!({
        "type": kind, "timestamp": "2026-10-24T10:00:00Z",
        "message": {"role": kind, "content": [{"type": "text", "text": words}]},
    })
    .to_string()
}

fn write(path: &Path, turns: usize) {
    let lines: Vec<String> = (0..turns)
        .flat_map(|n| {
            [
                line("user", &format!("ask {n}")),
                line("assistant", &format!("answer {n}")),
            ]
        })
        .collect();
    std::fs::write(path, lines.join("\n") + "\n").unwrap();
}

fn append(path: &Path, lines: &[String]) {
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    file.write_all((lines.join("\n") + "\n").as_bytes())
        .unwrap();
}

fn key(id: &str) -> FeedKey {
    FeedKey {
        account: "acc".to_owned(),
        conversation: id.to_owned(),
        subagent: None,
    }
}

fn same(path: &Path) -> impl Fn(&FeedKey, &Path) -> Option<PathBuf> + '_ {
    move |_, _| Some(path.to_path_buf())
}

struct Setup {
    _dir: tempfile::TempDir,
    path: PathBuf,
    feeds: Feeds,
    sink: Arc<Collect>,
    snapshot: ConversationSnapshotInfo,
    now: Instant,
}

fn setup(turns: usize) -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    write(&path, turns);
    let feeds = Feeds::default();
    let sink = Arc::new(Collect::default());
    let now = Instant::now();
    let snapshot = feeds.follow(key("c1"), &path, 30, sink.clone(), 8, now);
    Setup {
        _dir: dir,
        path,
        feeds,
        sink,
        snapshot,
        now,
    }
}

#[test]
fn a_follow_shows_the_latest_turns_with_ids_then_only_what_changes() {
    let s = setup(20);
    assert_eq!(s.snapshot.state, "shown");
    assert_eq!(s.snapshot.turns.len(), 30);
    assert!(s.snapshot.subscription.is_some());
    assert_eq!(s.snapshot.layout, "stretches");
    let last = s.snapshot.turns.last().unwrap().clone();
    assert_eq!(
        last.id,
        format!("{}:{}", s.snapshot.generation, last.offset)
    );
    assert!(last.open);
    assert_eq!(s.snapshot.earlier, Some(s.snapshot.turns[0].offset));
    // Nothing new: nothing sent.
    s.feeds.pass(s.now, true, &same(&s.path));
    assert!(s.sink.take().is_empty());
    append(&s.path, &[line("assistant", "and one more thing")]);
    s.feeds.pass(s.now, true, &same(&s.path));
    let got = s.sink.take();
    assert_eq!(got.len(), 1);
    let delta = &got[0];
    assert_eq!(delta.kind, "append");
    assert_eq!(delta.revision, s.snapshot.revision + 1);
    assert_eq!(delta.subscription, s.snapshot.subscription.unwrap());
    assert_eq!(delta.upserts.len(), 1);
    assert_eq!(
        delta.upserts[0].id, last.id,
        "the open turn grew under its id"
    );
    assert!(delta.upserts[0].revision > last.revision);
}

#[test]
fn a_key_in_an_appended_turn_arrives_redacted_and_counted() {
    let s = setup(2);
    append(
        &s.path,
        &[line(
            "user",
            "use the key sk-ant-api03-abcdefghijklmnopqrstu to call it",
        )],
    );
    s.feeds.pass(s.now, true, &same(&s.path));
    let got = s.sink.take();
    let turn = got[0].upserts.last().unwrap();
    let words = &turn.blocks[0].text;
    assert!(!words.contains("abcdefghijklmnop"), "a test key leaked");
    assert!(words.contains("•••"));
    assert!(turn.redacted >= 1);
}

#[test]
fn two_pages_share_one_feed_and_each_hears_the_change() {
    let s = setup(3);
    let other = Arc::new(Collect::default());
    let second = s
        .feeds
        .follow(key("c1"), &s.path, 10, other.clone(), 8, s.now);
    assert_eq!(s.feeds.len(), 1);
    assert_eq!(second.turns.len(), 6);
    assert_ne!(second.subscription, s.snapshot.subscription);
    append(&s.path, &[line("user", "next")]);
    s.feeds.pass(s.now, true, &same(&s.path));
    assert_eq!(s.sink.take().len(), 1);
    let theirs = other.take();
    assert_eq!(theirs.len(), 1);
    assert_eq!(theirs[0].subscription, second.subscription.unwrap());
}

#[test]
fn an_unfollowed_feed_is_kept_a_minute_then_let_go() {
    let s = setup(3);
    s.feeds.unfollow(s.snapshot.subscription.unwrap(), s.now);
    s.feeds
        .pass(s.now + Duration::from_secs(30), true, &same(&s.path));
    assert_eq!(s.feeds.len(), 1, "kept: the page may come back");
    s.feeds.pass(
        s.now + IDLE_KEEP + Duration::from_secs(1),
        true,
        &same(&s.path),
    );
    assert!(s.feeds.is_empty());
}

#[test]
fn a_follow_lasts_while_the_page_says_so() {
    let s = setup(3);
    let id = s.snapshot.subscription.unwrap();
    let later = s.now + LEASE - Duration::from_secs(1);
    assert!(s.feeds.keep(&[id, 9_999], later).contains(&9_999));
    // Renewed at `later`: still followed past the first lease.
    s.feeds
        .pass(s.now + LEASE + Duration::from_secs(1), true, &same(&s.path));
    assert_eq!(s.feeds.followed_paths().len(), 1);
    // Not renewed again: let go.
    s.feeds
        .pass(later + LEASE + Duration::from_secs(1), true, &same(&s.path));
    assert!(s.feeds.followed_paths().is_empty());
    assert_eq!(s.feeds.keep(&[id], later), vec![id]);
}

#[test]
fn a_page_that_cant_be_reached_is_dropped() {
    let s = setup(2);
    let gone = Arc::new(Collect {
        closed: true,
        ..Collect::default()
    });
    s.feeds.follow(key("c1"), &s.path, 10, gone, 8, s.now);
    append(&s.path, &[line("user", "next")]);
    s.feeds.pass(s.now, true, &same(&s.path));
    append(&s.path, &[line("assistant", "ok")]);
    s.feeds.pass(s.now, true, &same(&s.path));
    assert_eq!(s.sink.take().len(), 2, "the reachable page hears both");
}

#[test]
fn unfollowed_feeds_past_the_tier_go_longest_idle_first() {
    let dir = tempfile::tempdir().unwrap();
    let feeds = Feeds::default();
    let now = Instant::now();
    for n in 0..4u64 {
        let path = dir.path().join(format!("{n}.jsonl"));
        write(&path, 2);
        let snapshot = feeds.follow(
            key(&n.to_string()),
            &path,
            10,
            Arc::new(Collect::default()),
            2,
            now,
        );
        feeds.unfollow(snapshot.subscription.unwrap(), now + Duration::from_secs(n));
    }
    let path = dir.path().join("new.jsonl");
    write(&path, 2);
    feeds.follow(key("new"), &path, 10, Arc::new(Collect::default()), 2, now);
    assert_eq!(feeds.len(), 3, "the followed one and the two kept");
}

#[test]
fn a_truncated_or_replaced_file_is_read_again_as_a_reset() {
    let s = setup(5);
    let first = s.snapshot.generation;
    // Rewritten shorter.
    write(&s.path, 2);
    s.feeds.pass(s.now, true, &same(&s.path));
    let got = s.sink.take();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].kind, "reset");
    assert_ne!(got[0].generation, first);
    assert_eq!(got[0].upserts.len(), 4);
    assert_eq!(got[0].earlier, None);
    // Replaced by another file at the same path, longer.
    let other = s.path.with_extension("tmp");
    write(&other, 6);
    std::fs::rename(&other, &s.path).unwrap();
    s.feeds.pass(s.now, true, &same(&s.path));
    let got = s.sink.take();
    assert_eq!(got[0].kind, "reset");
    assert_eq!(got[0].upserts.len(), 12);
}

#[test]
fn another_file_for_the_conversation_is_a_reset() {
    let s = setup(3);
    let resumed = s.path.with_file_name("resumed.jsonl");
    write(&resumed, 4);
    s.feeds.pass(s.now, true, &move |_: &FeedKey, _: &Path| {
        Some(resumed.clone())
    });
    let got = s.sink.take();
    assert_eq!(got[0].kind, "reset");
    assert_eq!(got[0].upserts.len(), 8);
    assert_ne!(got[0].generation, s.snapshot.generation);
}

#[test]
fn a_gone_or_compressed_file_keeps_the_turns_and_says_so() {
    let s = setup(3);
    std::fs::remove_file(&s.path).unwrap();
    s.feeds.pass(s.now, true, &same(&s.path));
    let got = s.sink.take();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].kind, "state");
    assert_eq!(got[0].status.as_deref(), Some("gone"));
    assert!(got[0].upserts.is_empty(), "nothing replaced");
    // Said once.
    s.feeds.pass(s.now, true, &same(&s.path));
    assert!(s.sink.take().is_empty());
    // Back: read as usual again.
    write(&s.path, 3);
    s.feeds.pass(s.now, true, &same(&s.path));
    let back = s.sink.take();
    assert_eq!(back[0].status, None);
    // Stored compressed now: ended, not read again.
    let compressed = s.path.with_extension("jsonl.zst");
    std::fs::write(&compressed, b"x").unwrap();
    s.feeds.pass(s.now, true, &move |_: &FeedKey, _: &Path| {
        Some(compressed.clone())
    });
    let ended = s.sink.take();
    assert_eq!(ended.last().unwrap().status.as_deref(), Some("ended"));
}

#[test]
fn many_new_turns_at_once_come_as_a_reset_of_the_latest() {
    let s = setup(2);
    let lines: Vec<String> = (0..60)
        .flat_map(|n| {
            [
                line("user", &format!("more {n}")),
                line("assistant", &format!("reply {n}")),
            ]
        })
        .collect();
    append(&s.path, &lines);
    s.feeds.pass(s.now, true, &same(&s.path));
    let got = s.sink.take();
    assert_eq!(got[0].kind, "reset");
    assert_eq!(got[0].upserts.len(), PAGE_MAX);
    assert_eq!(
        got[0].generation, s.snapshot.generation,
        "same file: same ids"
    );
}

#[test]
fn turned_off_every_page_hears_it_and_all_is_let_go() {
    let s = setup(2);
    s.feeds.pass(s.now, false, &same(&s.path));
    let got = s.sink.take();
    assert_eq!(got[0].status.as_deref(), Some("off"));
    assert!(s.feeds.is_empty());
}

#[test]
fn earlier_turns_join_the_shown_ones_without_a_seam() {
    let s = setup(40);
    let id = s.snapshot.subscription.unwrap();
    let before = s.snapshot.earlier.unwrap();
    let page = s.feeds.earlier(id, before, 30);
    assert_eq!(page.state, "shown");
    assert_eq!(page.generation, s.snapshot.generation);
    assert_eq!(page.turns.len(), 30);
    assert!(page.turns.iter().all(|turn| !turn.open));
    assert_eq!(page.turns.last().unwrap().blocks[0].text, "answer 24");
    assert_eq!(s.snapshot.turns[0].blocks[0].text, "ask 25");
    assert_eq!(s.feeds.earlier(12_345, before, 30).state, "gone");
}

#[test]
fn a_hundred_appends_in_a_second_are_a_few_batches_each_within_half_a_second() {
    let start = Instant::now();
    let mut debounce = Debounce::default();
    let mut fired: Vec<(Instant, Instant)> = Vec::new();
    let mut first_pending: Option<Instant> = None;
    // Every 10 ms for a second, then quiet; the thread's loop, on a
    // clock of its own.
    let mut now = start;
    let end = start + Duration::from_secs(2);
    let mut next_event = start;
    let mut events = 0;
    while now < end {
        if events < 100 && now >= next_event {
            debounce.changed(now);
            first_pending.get_or_insert(now);
            events += 1;
            next_event += Duration::from_millis(10);
        }
        if debounce.wait(now) == Some(Duration::ZERO) {
            debounce.fired();
            fired.push((first_pending.take().unwrap(), now));
        }
        now += Duration::from_millis(1);
    }
    assert!(fired.len() <= 4, "{} batches", fired.len());
    assert!(fired.len() >= 2);
    for (first, at) in fired {
        assert!(at.duration_since(first) <= SETTLE_MAX);
    }
}

/// The rc21 preflight (2026-11-09): the system's file events ran more than
/// 60 s late. The followed file's own event never comes here, while other
/// files in its folder (other sessions of the project) wake the thread
/// every 300 ms: the fallback still checks the file at least every
/// [`FALLBACK`] (it used to restart at each wake-up, and never ran), and
/// a rescan wake-up (the system dropped events) counts as a change. The
/// thread's loop, on a clock of its own.
#[test]
fn a_followed_files_change_is_checked_within_the_fallback_whatever_wakes_the_thread() {
    use super::watch::{FALLBACK, SETTLE, due_in};
    use juggler_adapters::folder_wake::Woke;
    let followed = |path: &Path| path.ends_with("mine.jsonl");
    let start = Instant::now();
    let other = Woke::Paths(vec![PathBuf::from("/p/other.jsonl")]);
    let wakes: Vec<(Instant, Woke)> = (1..=20)
        .map(|n| (start + Duration::from_millis(300) * n, other.clone()))
        .chain([(start + Duration::from_millis(6_100), Woke::Rescan)])
        .collect();
    let end = start + Duration::from_secs(7);
    let mut debounce = Debounce::default();
    let (mut now, mut passed_at) = (start, start);
    let mut passes: Vec<Instant> = Vec::new();
    let mut next = wakes.iter().peekable();
    while now < end {
        let wait = due_in(&debounce, Some(passed_at), now).unwrap();
        match next.peek() {
            Some((at, woke)) if *at < now + wait => {
                now = *at;
                if woke.concerns(followed) {
                    debounce.changed(now);
                }
                next.next();
                if due_in(&debounce, Some(passed_at), now) != Some(Duration::ZERO) {
                    continue;
                }
            }
            _ => now += wait,
        }
        debounce.fired();
        passed_at = now;
        passes.push(now);
    }
    let mut last = start;
    for at in &passes {
        assert!(
            at.duration_since(last) <= FALLBACK,
            "a check {:?} after the last",
            at.duration_since(last)
        );
        last = *at;
    }
    assert!(passes.len() >= 6, "{} checks in 7 s", passes.len());
    let rescan = start + Duration::from_millis(6_100);
    assert!(
        passes.contains(&(rescan + SETTLE)),
        "a rescan is a change: checked {SETTLE:?} after it"
    );
}
