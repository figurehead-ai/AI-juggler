//! F94 (the AJ-E2317 class): a switch's source is the App the conversation
//! is in, by its id, and the targets are each App by its own id. Fixtures
//! only: temp folders, `true` standing in for the tool, a made-up desktop
//! App profile; nothing runs and no real profile is read.

use std::path::Path;
use std::sync::Arc;

use crate::features::conversations::switch_source::source_key;
use crate::state::AppState;

const NATIVE: &str = "6f1c2d3e-4a5b-4c6d-8e7f-00112233aabb";

struct Lab {
    _dir: tempfile::TempDir,
    root: std::path::PathBuf,
    state: Arc<AppState>,
    work: String,
    work_recipe: String,
    home_recipe: String,
}

/// Two Claude accounts, Work and Home (Home set up), beta switches on,
/// and one closed conversation on Work.
fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let mut registry = juggler_adapter::AdapterRegistry::new();
    let claude = Arc::new(juggler_adapters::ClaudeCodeAdapter::with_bin(
        "/usr/bin/true",
    ));
    registry.register(claude.clone());
    registry.register_handover(claude);
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    crate::settings_policy::app_setting_set_impl(
        &state,
        crate::features::validation::BETA_SWITCHES_KEY,
        "1",
    )
    .unwrap();
    let tool = juggler_adapters::CLAUDE_CODE_ID;
    let work = crate::features::account::create_account(&state, tool, "Work", None).unwrap();
    let work_recipe =
        crate::features::profile::create_profile(&state, &work.id, "Main", None, None).unwrap();
    let home = crate::features::account::create_account(&state, tool, "Home", None).unwrap();
    let home_recipe =
        crate::features::profile::create_profile(&state, &home.id, "Main", None, None).unwrap();
    let home_dir = state.accounts_dir().join(tool).join(&home.id);
    std::fs::create_dir_all(&home_dir).unwrap();
    std::fs::write(
        home_dir.join(".claude.json"),
        r#"{"hasCompletedOnboarding":true}"#,
    )
    .unwrap();
    write_conversation(&state, &root, &work.id);
    crate::features::conversations::commands::list_impl(&state).unwrap();
    Lab {
        _dir: dir,
        root,
        state,
        work: work.id,
        work_recipe: work_recipe.id,
        home_recipe: home_recipe.id,
    }
}

fn write_conversation(state: &Arc<AppState>, root: &Path, account: &str) {
    let tool = juggler_adapters::CLAUDE_CODE_ID;
    let folder = root.join("site");
    std::fs::create_dir_all(&folder).unwrap();
    let transcripts = state
        .accounts_dir()
        .join(tool)
        .join(account)
        .join("projects")
        .join(folder.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&transcripts).unwrap();
    let cwd = folder.display();
    std::fs::write(
        transcripts.join(format!("{NATIVE}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"uuid\":\"u1\",\"sessionId\":\"{NATIVE}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"user\",\"content\":\"Refactor the settings screen\"}}}}\n\
             {{\"type\":\"assistant\",\"uuid\":\"u2\",\"parentUuid\":\"u1\",\"sessionId\":\"{NATIVE}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"Done.\"}}]}}}}\n"
        ),
    )
    .unwrap();
}

/// A Claude app profile on Work; `holds`: its own records list the
/// conversation (a conversation of its Code tab).
fn desktop_app_on_work(lab: &Lab, holds: bool) -> String {
    let (id, _, _) = crate::features::places::remember_test_profile(
        &lab.state,
        &lab.root,
        "Work",
        Some(&lab.work),
    );
    if holds {
        crate::features::places::with_book(&lab.state, |book| {
            book.places.get_mut(&id).unwrap().conversations =
                vec![crate::features::places::book::conversation_key(
                    juggler_adapters::CLAUDE_CODE_ID,
                    NATIVE,
                )];
        })
        .unwrap();
    }
    id
}

/// F94: opened from the desktop App's row, the dialog's source is that
/// desktop App (its id and its name), never "Claude Code CLI · Work".
#[test]
fn a_desktop_apps_code_tab_conversation_has_that_app_as_its_source() {
    let lab = lab();
    let app = desktop_app_on_work(&lab, true);
    let key = source_key(&lab.work, NATIVE);

    let preflight = crate::features::switch::preflight::switch_preflight_impl(
        &lab.state,
        &key,
        &lab.home_recipe,
    )
    .unwrap();
    assert_eq!(preflight.source_app_id, app, "the App that holds it, by id");
    assert_eq!(preflight.source_app_name, "Claude app · Work");

    // Its targets: never the App it is in, nor Work in a terminal (its own
    // launch settings, Continue in AI Juggler); Home is offered.
    let targets = crate::features::switch::targets::switch_targets_impl(&lab.state, &key).unwrap();
    assert!(
        targets.iter().all(|row| row.app_id != app),
        "the source App is no target: {targets:?}"
    );
    assert!(
        !targets
            .iter()
            .any(|row| row.recipe_id == lab.work_recipe && row.place_id.is_none()),
        "its own folder in a terminal is no switch"
    );
    assert!(targets.iter().any(|row| row.recipe_id == lab.home_recipe));
}

/// F94: a terminal conversation on Work is its CLI App's, and the desktop
/// App on Work that takes sessions is a target, by its own id (before, the
/// whole account was left out with its launch settings).
#[test]
fn a_terminal_conversation_is_offered_its_folders_desktop_app_by_its_id() {
    let lab = lab();
    let app = desktop_app_on_work(&lab, false);
    let key = source_key(&lab.work, NATIVE);

    let preflight = crate::features::switch::preflight::switch_preflight_impl(
        &lab.state,
        &key,
        &lab.home_recipe,
    )
    .unwrap();
    assert_eq!(
        preflight.source_app_id,
        juggler_core::storage::cli_app_id(&lab.work)
    );
    assert_eq!(preflight.source_app_name, "Claude Code CLI · Work");

    let targets = crate::features::switch::targets::switch_targets_impl(&lab.state, &key).unwrap();
    let desktop = targets
        .iter()
        .find(|row| row.place_id.as_deref() == Some(app.as_str()));
    assert!(
        desktop.is_some(),
        "Claude app · Work is offered: {targets:?}"
    );
    let desktop = desktop.unwrap();
    assert_eq!(desktop.app_id, app, "each row is its App by its own id");
    assert_eq!(desktop.recipe_id, lab.work_recipe);
    assert!(
        targets
            .iter()
            .all(|row| row.app_id != preflight.source_app_id),
        "the source's own CLI App is no target"
    );
    // Every other row names its App: Home's CLI App by its folder.
    let home = targets
        .iter()
        .find(|row| row.recipe_id == lab.home_recipe && row.place_id.is_none())
        .unwrap();
    assert_eq!(home.app_id, format!("cli:{}", home.account_id));
}
