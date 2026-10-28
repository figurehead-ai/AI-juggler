//! D-51: a switch offers this project's notes (Claude Code's auto memory)
//! the way it offers MCP servers and skills: only when the other account
//! lacks some, never ticked for the user, copied only when picked, a note
//! that differs kept as it is, and undone from the same place. A tool that
//! keeps no notes gets no offer: the check says how many stay behind.
//! Temp folders only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::claude_memory::{INDEX, memory_folder};
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::features::switch_setup::{apply_impl, check_impl, list_impl, undo_impl};
use juggler_app::state::AppState;

struct Lab {
    state: Arc<AppState>,
    session: String,
    project: PathBuf,
    a: PathBuf,
    claude_b: (String, PathBuf),
    codex: String,
    _work: tempfile::TempDir,
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn lab() -> Lab {
    use std::os::unix::fs::PermissionsExt;
    let work = tempfile::tempdir().unwrap();
    let stub = work.path().join("tool-stub");
    std::fs::write(&stub, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    let codex = Arc::new(CodexAdapter::with_bin(&stub));
    registry.register(codex.clone());
    registry.register_handover(codex);
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());
    let project = work.path().join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let make = |tool: &str, label: &str| {
        let account = commands::create_account(&state, tool, label, None).unwrap();
        let profile = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
        let dir = state.accounts_dir().join(tool).join(&account.id);
        std::fs::create_dir_all(&dir).unwrap();
        (profile.id, dir)
    };
    let (profile_a, a) = make(CLAUDE_CODE_ID, "Work");
    let claude_b = make(CLAUDE_CODE_ID, "Personal");
    let (codex, _) = make(CODEX_ID, "Codex");

    // Work's notes for the shop: three, and the index.
    let folder = memory_folder(&a, &project);
    write(
        &folder.join(INDEX),
        "- [Codename](project-codename.md) — what it's called\n- [Tests](testing.md) — how to run them\n- [Style](style.md) — house style\n",
    );
    write(
        &folder.join("project-codename.md"),
        "The codename is Bluebird.\n",
    );
    write(&folder.join("testing.md"), "Run just test.\n");
    write(&folder.join("style.md"), "Short sentences.\n");

    let launched = commands::launch_profile(
        &state,
        &profile_a,
        Some(project.to_string_lossy().into_owned()),
    )
    .unwrap();
    Lab {
        state,
        session: launched.session_id,
        project,
        a,
        claude_b,
        codex,
        _work: work,
    }
}

#[test]
fn the_other_claude_account_is_offered_the_notes_it_lacks_and_gets_them_only_when_picked() {
    let lab = lab();
    let (profile_b, b) = &lab.claude_b;
    // Personal has its own codename note, with other words.
    let b_folder = memory_folder(b, &lab.project);
    write(
        &b_folder.join("project-codename.md"),
        "The codename is Kestrel.\n",
    );

    let check = check_impl(&lab.state, &lab.session, profile_b).unwrap();
    let offer = check
        .offers
        .iter()
        .find(|offer| offer.kind == "memory")
        .unwrap_or_else(|| panic!("no notes offer: {:?}", check.offers));
    assert_eq!(offer.count, Some(2), "two of the three are missing");
    assert_eq!(offer.verdict, "copy");
    assert!(!offer.suggested, "copied only when the user ticks it");
    let note = offer.note.as_deref().unwrap();
    assert!(
        note.contains("project-codename.md") && note.contains("stays as it is"),
        "{note}"
    );
    assert_eq!(check.notes_stay, None);

    // Not picked: nothing is copied.
    apply_impl(&lab.state, &lab.session, profile_b, &[]).unwrap();
    assert!(!b_folder.join("style.md").exists());

    // Picked: the two missing notes, and the index lines for them.
    let applied = apply_impl(
        &lab.state,
        &lab.session,
        profile_b,
        std::slice::from_ref(&offer.id),
    )
    .unwrap();
    assert_eq!(applied.len(), 1, "{applied:?}");
    let record = applied[0].record.clone().expect(&applied[0].message);
    assert!(
        applied[0]
            .message
            .starts_with("Copied the 2 notes for this project from Work"),
        "{}",
        applied[0].message
    );
    assert_eq!(
        std::fs::read_to_string(b_folder.join("project-codename.md")).unwrap(),
        "The codename is Kestrel.\n",
        "never overwritten"
    );
    assert!(b_folder.join("style.md").is_file() && b_folder.join("testing.md").is_file());
    let index = std::fs::read_to_string(b_folder.join(INDEX)).unwrap();
    assert!(
        !index.contains("project-codename.md"),
        "no line for Work's codename: {index}"
    );

    // Checked again: nothing missing, so no offer, and it counts as there.
    let again = check_impl(&lab.state, &lab.session, profile_b).unwrap();
    assert!(again.offers.iter().all(|offer| offer.kind != "memory"));

    // The set-up list shows it, and its undo takes out what was added.
    let listed = list_impl(&lab.state).unwrap();
    assert!(
        listed
            .iter()
            .any(|s| s.record == record && s.what == "the 2 notes for this project from Work")
    );
    assert_eq!(
        undo_impl(&lab.state, &record).unwrap(),
        "Took out the 2 notes for this project from Work"
    );
    assert!(!b_folder.join("style.md").exists() && !b_folder.join("testing.md").exists());
    assert!(
        !b_folder.join(INDEX).exists(),
        "the index the copy made is gone"
    );
    assert!(
        b_folder.join("project-codename.md").is_file(),
        "Personal's own stays"
    );
    // Work's notes were only read.
    assert_eq!(
        std::fs::read_to_string(memory_folder(&lab.a, &lab.project).join("style.md")).unwrap(),
        "Short sentences.\n"
    );
}

#[test]
fn a_tool_without_notes_gets_no_offer_and_the_notes_stay() {
    let lab = lab();
    let check = check_impl(&lab.state, &lab.session, &lab.codex).unwrap();
    assert!(check.offers.iter().all(|offer| offer.kind != "memory"));
    assert_eq!(check.notes_stay, Some(3));
}

#[test]
fn an_account_with_every_note_is_offered_nothing() {
    let lab = lab();
    let (profile_b, b) = &lab.claude_b;
    let from = memory_folder(&lab.a, &lab.project);
    let to = memory_folder(b, &lab.project);
    for name in ["project-codename.md", "testing.md", "style.md"] {
        write(
            &to.join(name),
            &std::fs::read_to_string(from.join(name)).unwrap(),
        );
    }
    let check = check_impl(&lab.state, &lab.session, profile_b).unwrap();
    assert!(
        check.offers.iter().all(|offer| offer.kind != "memory"),
        "{:?}",
        check.offers
    );
    assert_eq!(check.notes_stay, None);
}
