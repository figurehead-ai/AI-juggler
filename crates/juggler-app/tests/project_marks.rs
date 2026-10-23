//! AJ-E1532: every project's mark comes with the project list, decided by
//! the backend — its own icon (found read-only; an oversized file and a
//! "logo" that isn't an image passed over), else letters on a colour.
//! Clones of one repository share a mark and are numbered in the order
//! they were first seen; the user's choice is kept for all of them and
//! can be undone; the UI reads an icon by project id only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use juggler_app::commands;
use juggler_app::error::AppError;
use juggler_app::features::projects::icon::icon_read_impl;
use juggler_app::features::projects::mark::{MarkChoice, MarkShows, mark_set_impl};
use juggler_app::state::AppState;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01";

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

/// A project folder, a checkout of `origin` when given, touched as a
/// project in that order.
fn folder(state: &Arc<AppState>, root: &Path, name: &str, origin: Option<&str>) -> String {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    if let Some(origin) = origin {
        git(&dir, &["init", "-q"]);
        git(&dir, &["remote", "add", "origin", origin]);
    }
    std::thread::sleep(std::time::Duration::from_millis(3));
    let record = state
        .storage
        .lock()
        .unwrap()
        .projects()
        .touch(&dir, None)
        .unwrap();
    record.id.to_string()
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn marks(
    state: &Arc<AppState>,
) -> HashMap<String, juggler_app::features::projects::mark::ProjectMarkInfo> {
    commands::projects_list_impl(state)
        .unwrap()
        .into_iter()
        .map(|p| (p.id, p.mark.expect("every listed project has a mark")))
        .collect()
}

#[test]
fn marks_come_with_the_list_icons_letters_colours_and_numbered_clones() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let state = Arc::new(AppState::init(root.join("app-data")).unwrap());
    let web = folder(
        &state,
        &root,
        "acme-web",
        Some("git@example.com:acme/acme-web.git"),
    );
    let payments = folder(&state, &root, "payments-api", None);
    let web2 = folder(
        &state,
        &root,
        "acme-web2",
        Some("https://example.com/acme/acme-web"),
    );
    let app = folder(&state, &root, "desk-app", None);
    let design = folder(&state, &root, "design-system", None);
    let cli = folder(&state, &root, "cli-tools", None);
    write(&root.join("desk-app/src-tauri/icons/icon.png"), PNG);
    write(
        &root.join("design-system/logo.png"),
        &[PNG, &vec![0; 600 * 1024]].concat(),
    );
    write(&root.join("cli-tools/logo.png"), b"this is not an image\n");
    write(
        &root.join("acme-web2/public/favicon.svg"),
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    );

    let marks = marks(&state);
    // Clones: one mark (letters, colour, the icon one of them has), numbered.
    assert_eq!(marks[&web].letters, "AW");
    assert_eq!(
        (marks[&web].letters.clone(), marks[&web].swatch),
        (marks[&web2].letters.clone(), marks[&web2].swatch)
    );
    assert_eq!((marks[&web].clone, marks[&web2].clone), (Some(1), Some(2)));
    assert_eq!(
        (marks[&web].shows, marks[&web2].shows),
        (MarkShows::Icon, MarkShows::Icon)
    );
    // Others: their own colours, in the order they were first seen.
    assert_eq!(marks[&web].swatch, 0);
    assert_eq!(marks[&payments].swatch, 1);
    assert_eq!(
        (marks[&payments].letters.as_str(), marks[&payments].clone),
        ("PA", None)
    );
    assert_eq!(marks[&app].shows, MarkShows::Icon);
    // The oversized logo and the fake one are passed over: letters.
    assert_eq!(
        (marks[&design].shows, marks[&design].icon.as_deref()),
        (MarkShows::Letters, None)
    );
    assert_eq!(
        (marks[&cli].shows, marks[&cli].icon.as_deref()),
        (MarkShows::Letters, None)
    );

    // The UI reads an icon by project id: a clone gets the one they share.
    assert!(
        icon_read_impl(&state, &app)
            .unwrap()
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
    assert!(
        icon_read_impl(&state, &web)
            .unwrap()
            .unwrap()
            .starts_with("data:image/svg+xml;base64,")
    );
    assert_eq!(icon_read_impl(&state, &design).unwrap(), None);
    assert_eq!(icon_read_impl(&state, "not-a-project").unwrap(), None);
}

#[test]
fn a_choice_is_kept_for_every_clone_refused_in_words_and_undone() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let state = Arc::new(AppState::init(root.join("app-data")).unwrap());
    let web = folder(
        &state,
        &root,
        "acme-web",
        Some("git@example.com:acme/acme-web.git"),
    );
    let web2 = folder(
        &state,
        &root,
        "acme-web2",
        Some("git@example.com:acme/acme-web.git"),
    );
    let other = folder(&state, &root, "infra", None);
    let before = marks(&state);

    let choice = MarkChoice {
        emoji: Some("🚀".into()),
        swatch: Some(9),
        letters: None,
    };
    mark_set_impl(&state, &web2, choice.clone()).unwrap();
    let after = marks(&state);
    for id in [&web, &web2] {
        assert_eq!(
            (
                after[id].emoji.as_deref(),
                after[id].swatch,
                after[id].shows
            ),
            (Some("🚀"), 9, MarkShows::Emoji)
        );
        assert_eq!(after[id].choice.as_ref(), Some(&choice));
    }
    assert_eq!(
        after[&other], before[&other],
        "another project's mark moved"
    );

    for (bad, words) in [
        (
            MarkChoice {
                letters: Some("ABC".into()),
                ..MarkChoice::default()
            },
            "one or two",
        ),
        (
            MarkChoice {
                swatch: Some(40),
                ..MarkChoice::default()
            },
            "twelve",
        ),
    ] {
        match mark_set_impl(&state, &web, bad) {
            Err(AppError::Refused(message)) => {
                assert!(message.to_lowercase().contains(words), "{message}")
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
    assert!(matches!(
        mark_set_impl(&state, "nope", MarkChoice::default()),
        Err(AppError::NotFound { .. })
    ));

    mark_set_impl(&state, &web, MarkChoice::default()).unwrap();
    assert_eq!(
        marks(&state),
        before,
        "Use its own brings back the mark it had"
    );
}
