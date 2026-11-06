//! One folder is one project, however it's spelled: a path through a
//! symlink (macOS's /tmp → /private/tmp, a linked ~/Dev) finds the project
//! the other spelling made (the 0.4 P3 walk: a launch's resolved folder
//! made a second "web" beside the one the user added).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use juggler_core::storage::Storage;

#[test]
fn a_folder_reached_through_a_link_is_the_same_project() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("repos/web");
    std::fs::create_dir_all(&real).unwrap();
    let link = dir.path().join("linked");
    std::os::unix::fs::symlink(dir.path().join("repos"), &link).unwrap();
    let aliased = link.join("web");

    let storage = Storage::open_in_memory().unwrap();
    let projects = storage.projects();
    let made = projects.touch(&aliased, None).unwrap();
    // Found by the resolved spelling, and touching it there makes no other.
    assert_eq!(
        projects.by_root(&real).unwrap().map(|p| p.id),
        Some(made.id.clone())
    );
    assert_eq!(projects.touch(&real, None).unwrap().id, made.id);
    assert_eq!(projects.list().unwrap().len(), 1);
    // A folder that isn't there is matched by its spelling only.
    assert!(
        projects
            .by_root(&dir.path().join("gone"))
            .unwrap()
            .is_none()
    );
}
