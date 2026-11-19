//! AJ-E3803: the disk rules, the index, Remove, approvals and declines,
//! and the registered cache that Clear cache never empties. Model files
//! here are small stand-ins of their entry's exact size.

use std::path::Path;
use std::sync::Arc;

use juggler_platform::room::DiskSpace;

use super::store::{Refusal, Store, cache_of};
use super::transfer_tests::stub::{bytes, entry, entry_sized, files, sha256};
use super::{InstalledState, Job, Listing, ModelEntry, Verdict, installed_state, read_untargeted};
use crate::features::cache::{Limit, Manager};

const GB: u64 = 1_000_000_000;

/// Puts `file` into the store as `entry`'s verified model.
fn install(store: &Store, entry: &ModelEntry, file: &[u8]) {
    std::fs::create_dir_all(store.dir()).expect("dir");
    let sha = &entry.download.as_ref().expect("file").sha256;
    let path = store.file_of(sha);
    std::fs::write(&path, file).expect("write");
    store.install(entry, &path).expect("installed");
}

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("dir");
    let store = Store::in_data_dir(dir.path());
    (dir, store)
}

#[test]
fn the_free_disk_margin_is_the_size_plus_10_gb_to_the_byte() {
    let (_dir, store) = store();
    let size = 1_834_426_016;
    let model = entry_sized("qwen", &"a".repeat(64), size);
    assert_eq!(model.free_disk_needed(), size + 10 * GB);
    assert_eq!(
        store.check_start(&model, Some(size + 10 * GB), false, None),
        Ok(())
    );
    assert_eq!(
        store.check_start(&model, Some(size + 10 * GB - 1), false, None),
        Err(Refusal::NotEnoughDisk {
            needed_bytes: size + 10 * GB,
            free_bytes: size + 10 * GB - 1,
        })
    );
    assert_eq!(
        store.check_start(&model, None, false, None),
        Err(Refusal::DiskUnknown)
    );
}

#[test]
fn an_entry_s_own_free_disk_only_tightens_the_margin() {
    let mut model = entry_sized("qwen", &"a".repeat(64), 2 * GB);
    model.min_free_gb = Some(4);
    assert_eq!(model.free_disk_needed(), 12 * GB);
    model.min_free_gb = Some(25);
    assert_eq!(model.free_disk_needed(), 27 * GB);
}

#[test]
fn one_model_is_the_default_and_each_further_one_needs_its_own_approval_each_time() {
    let (_dir, store) = store();
    let first = bytes(4_000, 1);
    let a = entry("model-a", &first);
    let b = entry_sized("model-b", &"b".repeat(64), 3 * GB);
    let c = entry_sized("model-c", &"c".repeat(64), 2 * GB);
    let roomy = Some(100 * GB);
    // The first needs no approval.
    assert_eq!(store.check_start(&a, roomy, false, None), Ok(()));
    install(&store, &a, &first);
    // The second does, showing its size and the new total.
    let needs = Err(Refusal::NeedsApproval {
        size_bytes: 3 * GB,
        total_after_bytes: 4_000 + 3 * GB,
    });
    assert_eq!(store.check_start(&b, roomy, false, None), needs);
    assert_eq!(store.check_start(&b, roomy, false, Some("guess")), needs);
    // One approval covers one start of that model only.
    let token = store.approve("model-b");
    assert_eq!(token.len(), 32);
    assert!(store.check_start(&c, roomy, false, Some(&token)).is_err());
    assert_eq!(store.check_start(&b, roomy, false, Some(&token)), Ok(()));
    assert_eq!(store.check_start(&b, roomy, false, Some(&token)), needs);
    // An approval isn't used up by a start another rule refuses.
    let token = store.approve("model-b");
    assert!(matches!(
        store.check_start(&b, Some(GB), false, Some(&token)),
        Err(Refusal::NotEnoughDisk { .. })
    ));
    assert_eq!(store.check_start(&b, roomy, false, Some(&token)), Ok(()));
    // A newer approval replaces an older one of the same model.
    let old = store.approve("model-b");
    let new = store.approve("model-b");
    assert_eq!(store.check_start(&b, roomy, false, Some(&old)), needs);
    assert_eq!(store.check_start(&b, roomy, false, Some(&new)), Ok(()));
}

#[test]
fn the_other_rules_come_first() {
    let (_dir, store) = store();
    let file = bytes(1_000, 2);
    let a = entry("model-a", &file);
    install(&store, &a, &file);
    let roomy = Some(100 * GB);
    assert_eq!(
        store.check_start(&a, roomy, false, None),
        Err(Refusal::AlreadyInstalled)
    );
    let b = entry_sized("model-b", &"b".repeat(64), GB);
    assert_eq!(store.check_start(&b, roomy, true, None), Err(Refusal::Busy));
    let mut apple = b.clone();
    apple.download = None;
    assert_eq!(
        store.check_start(&apple, roomy, false, None),
        Err(Refusal::NothingToFetch)
    );
}

#[test]
fn a_decline_is_remembered_until_the_user_approves() {
    let dir = tempfile::tempdir().expect("dir");
    let store = Store::in_data_dir(dir.path());
    let token = store.approve("model-b");
    store.decline("model-b");
    assert_eq!(store.declined(), vec!["model-b".to_owned()]);
    // A decline drops a pending approval.
    let b = entry_sized("model-b", &"b".repeat(64), GB);
    let first = bytes(100, 3);
    install(&store, &entry("model-a", &first), &first);
    assert!(
        store
            .check_start(&b, Some(100 * GB), false, Some(&token))
            .is_err()
    );
    // Remembered across a restart.
    let again = Store::in_data_dir(dir.path());
    assert_eq!(again.declined(), vec!["model-b".to_owned()]);
    let _ = again.approve("model-b");
    assert!(again.declined().is_empty());
    assert!(Store::in_data_dir(dir.path()).declined().is_empty());
}

#[test]
fn remove_deletes_the_file_and_its_copy_and_the_total_follows() {
    let (_dir, store) = store();
    let one = bytes(5_000, 4);
    let two = bytes(7_000, 5);
    let a = entry("model-a", &one);
    let b = entry("model-b", &two);
    install(&store, &a, &one);
    install(&store, &b, &two);
    assert_eq!(store.total_bytes(), 12_000);
    assert_eq!(store.installed_ids(), vec!["model-a", "model-b"]);
    assert_eq!(store.remove("model-a").expect("removed"), Some(5_000));
    assert_eq!(store.total_bytes(), 7_000);
    let sha_b = sha256(&two);
    assert_eq!(
        files(store.dir()),
        vec![format!("{sha_b}.gguf"), format!("{sha_b}.json")]
    );
    assert_eq!(store.remove("model-a").expect("nothing"), None);
}

#[test]
fn the_total_comes_from_the_index_not_a_walk() {
    let (dir, store) = store();
    let one = bytes(5_000, 6);
    let a = entry("model-a", &one);
    install(&store, &a, &one);
    // A file changed behind the store's back doesn't move the total: it
    // is read from the index, never by walking the folder.
    std::fs::write(store.file_of(&sha256(&one)), b"x").expect("write");
    assert_eq!(store.total_bytes(), 5_000);
    // A new store (the next start) reads the copies again and drops the
    // one whose file no longer matches.
    let again = Store::in_data_dir(dir.path());
    assert_eq!(again.total_bytes(), 0);
    assert_eq!(files(again.dir()), Vec::<String>::new());
}

#[test]
fn the_first_read_tidies_what_a_crash_left() {
    let (dir, store) = store();
    let one = bytes(3_000, 7);
    let a = entry("model-a", &one);
    install(&store, &a, &one);
    let folder = store.dir().to_path_buf();
    let sha = sha256(&one);
    let stray_sha = "e".repeat(64);
    let tamper_sha = "f".repeat(64);
    std::fs::write(folder.join(format!("{sha}-3.part")), b"half").expect("part");
    std::fs::write(folder.join(format!("{stray_sha}.gguf")), b"no copy").expect("stray");
    // A copy that names another file than its own name.
    std::fs::copy(
        folder.join(format!("{sha}.json")),
        folder.join(format!("{tamper_sha}.json")),
    )
    .expect("copy");
    std::fs::write(folder.join(format!("{tamper_sha}.gguf")), &*one).expect("file");
    std::fs::write(folder.join("notes.txt"), b"not ours").expect("other");
    let again = Store::in_data_dir(dir.path());
    assert_eq!(again.installed_ids(), vec!["model-a"]);
    assert_eq!(
        files(&folder),
        vec![
            format!("{sha}.gguf"),
            format!("{sha}.json"),
            "notes.txt".to_owned()
        ]
    );
}

#[test]
fn the_installed_copy_reads_back_through_the_section_s_reader() {
    let (dir, store) = store();
    let one = bytes(2_000, 8);
    let mut a = entry("model-a", &one);
    a.min_free_gb = Some(12);
    a.sampling.context_tokens = Some(8_192);
    a.sampling.datamark = Some(true);
    install(&store, &a, &one);
    let again = Store::in_data_dir(dir.path());
    let copy = again.installed().pop().expect("one");
    assert_eq!(copy.entry, a);
    assert_eq!(copy.size_bytes, 2_000);
    // The record is the file's own shape.
    let record = a.to_record();
    let read =
        read_untargeted(&serde_json::json!({ "schema": 1, "items": [record] })).expect("reads");
    assert_eq!(read, vec![a]);
}

#[test]
fn a_copy_follows_its_entry_while_the_file_is_the_same() {
    let (_dir, store) = store();
    let one = bytes(2_000, 9);
    let a = entry("model-a", &one);
    install(&store, &a, &one);
    // Same file, a widened verdict: the copy follows.
    let mut wider = a.clone();
    if let Some(chat) = wider.verdicts.get_mut(&Job::Chat) {
        chat.verdict = Verdict::Ships;
    }
    assert!(store.refresh(&Listing::Read(vec![wider.clone()])));
    assert_eq!(store.installed()[0].entry, wider);
    // Another file under the same id: ignored, nothing replaced.
    let mut other = wider.clone();
    other.download.as_mut().expect("file").sha256 = "d".repeat(64);
    assert!(!store.refresh(&Listing::Read(vec![other.clone()])));
    assert_eq!(store.installed()[0].entry, wider);
    assert!(matches!(
        installed_state(&store.installed()[0].entry, &Listing::Read(vec![other])),
        InstalledState::Unknown(_)
    ));
    // An unread section changes nothing.
    assert!(!store.refresh(&Listing::Unread));
}

fn manager_with(
    store: &Arc<Store>,
    data_dir: &Path,
    free: fn(&Path) -> Option<DiskSpace>,
) -> Manager {
    let manager =
        Manager::new(data_dir.to_path_buf(), true, Limit::Automatic, None).with_disk(free);
    manager.register(cache_of(Arc::clone(store)));
    manager
}

#[test]
fn clear_cache_the_cache_switch_and_a_full_disk_never_remove_a_model() {
    let dir = tempfile::tempdir().expect("dir");
    let store = Arc::new(Store::in_data_dir(dir.path()));
    let one = bytes(6_000, 10);
    let a = entry("model-a", &one);
    install(&store, &a, &one);
    let nearly_full = |_: &Path| {
        Some(DiskSpace {
            free: 1_000_000,
            total: 256 * GB,
        })
    };
    let manager = manager_with(&store, dir.path(), nearly_full);
    manager.clear().expect("cleared");
    manager.set_on(false).expect("off");
    manager.set_on(true).expect("on");
    let pass = manager.pass(manager.disk_space());
    assert!(pass.walked);
    assert_eq!(store.installed_ids(), vec!["model-a"]);
    let sha = sha256(&one);
    assert_eq!(
        files(store.dir()),
        vec![format!("{sha}.gguf"), format!("{sha}.json")]
    );
    // And a fresh store still finds it.
    assert_eq!(Store::in_data_dir(dir.path()).total_bytes(), 6_000);
}
