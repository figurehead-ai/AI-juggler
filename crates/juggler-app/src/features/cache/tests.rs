#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use juggler_platform::room::DiskSpace;

use super::folder::FolderCache;
use super::stamp::{self, Stamp};
use super::*;

const GB: u64 = 1_000_000_000;

fn about(id: &'static str, trim: Trim) -> About {
    About {
        id,
        name: "A cache",
        speeds_up: "Something.",
        slower: "Something else.",
        trim,
        own_switch: None,
        max_entries: None,
        shaped_by: &["the_setting"],
    }
}

fn folder(id: &'static str, dir: &Path) -> Arc<dyn LocalCache> {
    Arc::new(FolderCache::new(
        about(id, Trim::Partly),
        dir.to_path_buf(),
        "c",
        &["old"],
        Box::new(|| (1, 7)),
    ))
}

fn stamp_of(id: &str) -> Stamp<'_> {
    Stamp {
        cache: id,
        format: 1,
        settings: 7,
        source: "src",
    }
}

/// An entry of `bytes` bytes in `dir`, last used `age` seconds ago.
fn entry(gate: &Gate, dir: &Path, id: &str, name: &str, bytes: usize, age: u64) -> PathBuf {
    let path = dir.join(format!("{name}.c"));
    assert!(stamp::write(gate, &path, &stamp_of(id), &vec![b'x'; bytes]).unwrap());
    std::fs::File::options()
        .append(true)
        .open(&path)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(age))
        .unwrap();
    path
}

fn plenty() -> Option<DiskSpace> {
    Some(DiskSpace {
        free: 500 * GB,
        total: 1000 * GB,
    })
}

fn nearly_full() -> Option<DiskSpace> {
    Some(DiskSpace {
        free: GB,
        total: 1000 * GB,
    })
}

/// A whole-only cache: one file, removed only whole.
struct Whole {
    file: PathBuf,
    about: About,
}

impl LocalCache for Whole {
    fn about(&self) -> &About {
        &self.about
    }
    fn location(&self) -> PathBuf {
        self.file.clone()
    }
    fn items(&self, _limit: usize) -> Vec<Item> {
        std::fs::metadata(&self.file)
            .map(|meta| Item {
                path: self.file.clone(),
                bytes: meta.len(),
                used: SystemTime::UNIX_EPOCH,
            })
            .into_iter()
            .collect()
    }
    fn remove(&self, item: &Item) -> u64 {
        if self.clear().is_ok() { item.bytes } else { 0 }
    }
    fn clear(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.file) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

fn whole(dir: &Path, bytes: u64) -> Arc<dyn LocalCache> {
    let file = dir.join("index.sqlite");
    // Sparse: its size without the disk.
    std::fs::File::create(&file)
        .unwrap()
        .set_len(bytes)
        .unwrap();
    Arc::new(Whole {
        file,
        about: About {
            own_switch: Some(OwnSwitch {
                name: "its own switch",
                pane: "Privacy",
                section: "search",
            }),
            ..about("whole", Trim::WholeOnly)
        },
    })
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn an_entry_is_dropped_when_its_source_format_or_settings_change() {
    let tmp = tempfile::tempdir().unwrap();
    let gate = Gate::default();
    let path = tmp.path().join("cache/one.c");
    let made = Stamp {
        cache: "c",
        format: 1,
        settings: stamp::settings_print(&["messages"]),
        source: "file-100-1700000000",
    };
    let changed = [
        Stamp {
            source: "file-120-1700000500",
            ..made
        },
        Stamp { format: 2, ..made },
        Stamp {
            settings: stamp::settings_print(&["no messages"]),
            ..made
        },
        Stamp {
            cache: "other",
            ..made
        },
    ];
    for stale in changed {
        assert!(stamp::write(&gate, &path, &made, b"words").unwrap());
        assert_eq!(
            stamp::read(&gate, &path, &made, 1024).as_deref(),
            Some(&b"words"[..])
        );
        assert_eq!(stamp::read(&gate, &path, &stale, 1024), None);
        assert!(!path.exists(), "a changed entry is dropped, never shown");
    }
    // Corrupt, or too big: dropped too.
    std::fs::write(&path, b"no head line").unwrap();
    assert_eq!(stamp::read(&gate, &path, &made, 1024), None);
    assert!(!path.exists());
    assert!(stamp::write(&gate, &path, &made, &[b'x'; 2048]).unwrap());
    assert_eq!(stamp::read(&gate, &path, &made, 1024), None);
    assert!(!path.exists());
    // The print tells the words apart, not just their letters.
    assert_ne!(
        stamp::settings_print(&["ab", "c"]),
        stamp::settings_print(&["a", "bc"])
    );
}

#[test]
fn entries_are_owner_only_and_written_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let gate = Gate::default();
    let dir = tmp.path().join("new-cache");
    let path = dir.join("one.c");
    assert!(stamp::write(&gate, &path, &stamp_of("c"), b"words").unwrap());
    assert_eq!(names(&dir), vec!["one.c"], "no temporary file is left");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let file = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(file & 0o077, 0, "the file is this user's only");
        let folder = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(folder & 0o077, 0, "a new folder is this user's only");
    }
    assert_eq!(
        gate.written.load(Ordering::Acquire),
        std::fs::metadata(&path).unwrap().len()
    );
}

#[test]
fn trimming_lets_the_least_recently_used_go_first_across_caches() {
    let tmp = tempfile::tempdir().unwrap();
    let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
    let manager = Manager::new(tmp.path().to_path_buf(), true, Limit::Fixed(2_000), None);
    manager.register(folder("a", &a));
    manager.register(folder("b", &b));
    let gate = manager.gate();
    // 6 entries of ~400 bytes, used at interleaved times.
    entry(&gate, &a, "a", "a-oldest", 400, 600);
    entry(&gate, &b, "b", "b-second", 400, 500);
    entry(&gate, &a, "a", "a-third", 400, 400);
    entry(&gate, &b, "b", "b-fourth", 400, 300);
    entry(&gate, &a, "a", "a-newest", 400, 10);
    entry(&gate, &b, "b", "b-newest", 400, 5);
    let pass = manager.pass(plenty());
    assert!(pass.walked);
    assert!(pass.total <= 2_000, "trimmed to the limit");
    assert_eq!(pass.removed, 2, "the two oldest, one in each cache");
    assert_eq!(names(&a), vec!["a-newest.c", "a-third.c"]);
    assert_eq!(names(&b), vec!["b-fourth.c", "b-newest.c"]);
    // A read makes an entry recent: it outlives older ones.
    let third = a.join("a-third.c");
    assert!(stamp::read(&gate, &third, &stamp_of("a"), 4096).is_some());
    // Four of ~450 bytes over a 1,400 limit: the oldest one goes, not the
    // one just read.
    manager.set_limit(Limit::Fixed(1_400));
    manager.pass(plenty());
    assert_eq!(names(&a), vec!["a-newest.c", "a-third.c"]);
    assert_eq!(names(&b), vec!["b-newest.c"]);
}

#[test]
fn a_whole_only_cache_is_never_trimmed_partly_or_for_the_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let parts = tmp.path().join("parts");
    let manager = Manager::new(tmp.path().to_path_buf(), true, Limit::Fixed(1_000), None);
    manager.register(folder("parts", &parts));
    manager.register(whole(tmp.path(), budget::FLOOR + 5_000));
    let gate = manager.gate();
    entry(&gate, &parts, "parts", "one", 300, 100);
    entry(&gate, &parts, "parts", "two", 300, 50);
    // Over the limit with the whole-only one alone: the trimmable go, it
    // stays (removing it would only build it again).
    manager.pass(plenty());
    assert!(names(&parts).is_empty());
    assert!(tmp.path().join("index.sqlite").exists());
    // Only a nearly full disk removes it, whole, when it's over the floor.
    let pass = manager.pass(nearly_full());
    assert!(pass.walked);
    assert!(!tmp.path().join("index.sqlite").exists());
    assert_eq!(pass.total, 0);
}

#[test]
fn a_nearly_full_disk_trims_to_the_floor_and_stops_writes_until_there_is_room() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("c");
    let manager = Manager::new(tmp.path().to_path_buf(), true, Limit::Automatic, None);
    manager.register(folder("c", &dir));
    let gate = manager.gate();
    entry(&gate, &dir, "c", "kept", 100, 10);
    manager.pass(nearly_full());
    assert!(!gate.may_write(), "no new writes on a nearly full disk");
    assert!(!gate.has_room());
    assert!(!stamp::write(&gate, &dir.join("new.c"), &stamp_of("c"), b"x").unwrap());
    assert_eq!(names(&dir), vec!["kept.c"], "under the floor: kept");
    let info = info_of(&manager, Duration::from_secs(60));
    assert!(info.low_disk);
    assert_eq!(info.budget_bytes, budget::FLOOR);
    // Room again: writes resume.
    manager.pass(plenty());
    assert!(gate.may_write());
    assert!(stamp::write(&gate, &dir.join("new.c"), &stamp_of("c"), b"x").unwrap());
}

#[test]
fn clear_removes_every_caches_files_and_they_are_rebuilt_as_used() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("c");
    let manager = Manager::new(tmp.path().to_path_buf(), true, Limit::Automatic, None);
    manager.register(folder("c", &dir));
    manager.register(whole(tmp.path(), 100));
    let gate = manager.gate();
    entry(&gate, &dir, "c", "one", 100, 10);
    manager.clear().unwrap();
    assert!(!dir.exists());
    assert!(!tmp.path().join("index.sqlite").exists(), "every cache");
    let info = info_of(&manager, Duration::ZERO);
    assert_eq!(info.used_bytes, 0);
    assert!(info.on, "clearing isn't turning it off");
    assert!(stamp::write(&gate, &dir.join("again.c"), &stamp_of("c"), b"x").unwrap());
}

#[test]
fn turning_it_off_stops_writes_and_reads_and_removes_its_files() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("c");
    let manager = Manager::new(tmp.path().to_path_buf(), true, Limit::Automatic, None);
    manager.register(folder("c", &dir));
    manager.register(whole(tmp.path(), 100));
    let gate = manager.gate();
    let path = entry(&gate, &dir, "c", "one", 100, 10);
    manager.set_on(false).unwrap();
    assert!(!dir.exists(), "its files are removed");
    assert!(
        tmp.path().join("index.sqlite").exists(),
        "a cache with its own switch keeps to it"
    );
    assert!(!gate.may_write());
    assert!(!stamp::write(&gate, &path, &stamp_of("c"), b"x").unwrap());
    assert!(!dir.exists(), "nothing is written while off");
    // A file that lands anyway (a write already under way) is read never
    // and removed by the next pass.
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&path, stamp::encode(&stamp_of("c"), b"x").unwrap()).unwrap();
    assert_eq!(stamp::read(&gate, &path, &stamp_of("c"), 1024), None);
    manager.pass(plenty());
    assert!(!dir.exists());
    let info = info_of(&manager, Duration::ZERO);
    assert!(!info.on);
    manager.set_on(true).unwrap();
    assert!(gate.may_write());
}

#[test]
fn a_setting_change_drops_the_entries_it_shaped_at_once() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("c");
    let other = tmp.path().join("o");
    let manager = Manager::new(tmp.path().to_path_buf(), true, Limit::Automatic, None);
    let settings = Arc::new(std::sync::atomic::AtomicU64::new(7));
    let now = Arc::clone(&settings);
    manager.register(Arc::new(FolderCache::new(
        about("c", Trim::Partly),
        dir.clone(),
        "c",
        &[],
        Box::new(move || (1, now.load(Ordering::Acquire))),
    )));
    manager.register(Arc::new(FolderCache::new(
        About {
            shaped_by: &[],
            ..about("o", Trim::Partly)
        },
        other.clone(),
        "c",
        &[],
        Box::new(|| (1, 1)),
    )));
    let gate = manager.gate();
    entry(&gate, &dir, "c", "words", 100, 10);
    entry(&gate, &other, "o", "kept", 100, 10);
    manager.setting_changed("another_setting");
    assert_eq!(names(&dir), vec!["words.c"], "not shaped by it");
    settings.store(8, Ordering::Release);
    manager.setting_changed("the_setting");
    assert!(names(&dir).is_empty(), "made with the old value: gone");
    assert_eq!(names(&other), vec!["kept.c"], "another cache: untouched");
}

#[test]
fn the_entry_cap_keeps_the_newest_and_old_formats_go_on_the_first_walk() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("c");
    let manager = Manager::new(tmp.path().to_path_buf(), true, Limit::Automatic, None);
    manager.register(Arc::new(FolderCache::new(
        About {
            max_entries: Some(2),
            ..about("c", Trim::Partly)
        },
        dir.clone(),
        "c",
        &["old"],
        Box::new(|| (1, 7)),
    )));
    let gate = manager.gate();
    entry(&gate, &dir, "c", "first", 10, 300);
    entry(&gate, &dir, "c", "second", 10, 200);
    entry(&gate, &dir, "c", "third", 10, 100);
    std::fs::write(dir.join("before.old"), b"an older format").unwrap();
    // Another format, never read again.
    let stale = Stamp {
        format: 0,
        ..stamp_of("c")
    };
    stamp::write(&gate, &dir.join("stale.c"), &stale, b"x").unwrap();
    let pass = manager.pass(plenty());
    assert!(pass.walked);
    assert_eq!(names(&dir), vec!["second.c", "third.c"]);
    // A quiet pass doesn't walk again.
    assert!(!manager.pass(plenty()).walked);
}

#[test]
fn settings_shows_each_cache_with_its_size_and_words() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("c");
    let manager = Manager::new(
        tmp.path().to_path_buf(),
        true,
        Limit::from_mb(512),
        Some(8 << 30),
    );
    manager.register(folder("c", &dir));
    manager.register(whole(tmp.path(), 1_000));
    let gate = manager.gate();
    assert_eq!(gate.tier(), Tier::Small);
    entry(&gate, &dir, "c", "one", 100, 10);
    let info = info_of(&manager, Duration::ZERO);
    assert!(info.on);
    assert!(!info.automatic);
    assert_eq!(info.limit_mb, Some(512));
    assert_eq!(info.caches.len(), 2);
    let row = &info.caches[0];
    assert_eq!(row.name, "A cache");
    assert_eq!(row.entries, 1);
    assert!(row.bytes > 100);
    assert!(info.caches[1].whole_only);
    assert_eq!(info.caches[1].own_switch.as_deref(), Some("its own switch"));
    assert_eq!(info.caches[1].own_switch_section.as_deref(), Some("search"));
    assert!(info.caches.iter().all(|row| row.on));
    assert_eq!(info.used_bytes, row.bytes + 1_000);
    // Off: the caches it governs say so; one with its own switch keeps to it.
    manager.set_on(false).unwrap();
    let info = info_of(&manager, Duration::ZERO);
    assert!(!info.caches[0].on);
    assert!(info.caches[1].on);
}
