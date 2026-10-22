//! AJ-E1530 Req.1 (ADR-0018 amendment, opt-in): measuring limits for
//! sessions started outside AI Juggler on an account used where it is.
//! Only `statusLine` changes, spliced so every other byte stays; turning
//! it off puts the file back exactly, or, after the user's own edits,
//! takes out only AI Juggler's value. Unsafe states are refused.

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
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::quota::capture::{
    preview_impl, status_impl, turn_off_impl, turn_on_impl,
};
use juggler_app::state::AppState;

fn app_state(dir: &Path) -> Arc<AppState> {
    let data = dir.join("app-data");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ClaudeCodeAdapter::with_bin("/bin/true").with_overlays_root(data.join("overlays")),
    ));
    Arc::new(AppState::init_with_registry(data, registry).unwrap())
}

/// A setup with `settings` (or none), brought in where it is.
fn setup(root: &Path, settings: Option<&str>, in_place: bool) -> (Arc<AppState>, String, PathBuf) {
    let dir = root.join("ClaudeCodeProfiles/work-a");
    std::fs::create_dir_all(dir.join("projects")).unwrap();
    std::fs::write(dir.join(".credentials.json"), "{}").unwrap();
    if let Some(settings) = settings {
        std::fs::write(dir.join("settings.json"), settings).unwrap();
    }
    let state = app_state(root);
    let account = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        dir.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap()
    .account;
    (state, account.id.to_string(), dir)
}

/// Every name in the user's folder (it must gain nothing).
fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

const FIXTURES: [&str; 4] = [
    // Comments and an unusual key order.
    "// mine\n{\n  \"zeta\": true,\n  /* hooks */ \"hooks\": {\"Stop\": []},\n  \"alpha\": \"a\" // trailing\n}\n",
    // CRLF.
    "{\r\n\t\"model\": \"sonnet\",\r\n\t\"permissions\": {\"allow\": []}\r\n}\r\n",
    // Their own status line.
    "{\n  \"statusLine\": {\"type\": \"command\", \"command\": \"~/bin/my-line.sh\", \"padding\": 0},\n  \"model\": \"opus\"\n}\n",
    // Nothing yet.
    "{}",
];

#[test]
fn turning_it_on_changes_only_the_status_line_and_off_puts_every_byte_back() {
    for fixture in FIXTURES {
        let work = tempfile::tempdir().unwrap();
        let (state, account, dir) =
            setup(&work.path().canonicalize().unwrap(), Some(fixture), true);
        let file = dir.join("settings.json");
        assert_eq!(status_impl(&state, &account).unwrap().state, "off");
        let before_names = names(&dir);
        let preview = preview_impl(&state, &account).unwrap();
        assert!(
            preview.after.starts_with("\"statusLine\": "),
            "{}",
            preview.after
        );
        assert!(preview.after.contains("juggler-statusline.sh"));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            fixture,
            "a preview writes nothing"
        );

        turn_on_impl(&state, &account, &preview.fingerprint).unwrap();
        let on = std::fs::read_to_string(&file).unwrap();
        assert_eq!(status_impl(&state, &account).unwrap().state, "on");
        // Only statusLine differs: putting the old value back gives the
        // file as it was.
        let ours = juggler_adapter::jsonc::value_text(&on, "statusLine")
            .unwrap()
            .unwrap();
        let original = juggler_adapter::jsonc::value_text(fixture, "statusLine").unwrap();
        assert_eq!(
            juggler_adapter::jsonc::unset(&on, "statusLine", &ours, original.as_deref())
                .unwrap()
                .unwrap(),
            fixture
        );
        assert_eq!(
            names(&dir),
            before_names,
            "nothing new in the user's folder"
        );
        // Their own line is chained: the wrapper runs it after capturing.
        let wrapper = std::fs::read_to_string(
            state
                .overlays_dir()
                .join(&account)
                .join("juggler-statusline.sh"),
        )
        .unwrap();
        if fixture.contains("my-line.sh") {
            assert!(wrapper.contains("~/bin/my-line.sh"), "{wrapper}");
            assert!(
                ours.contains("\"padding\":0"),
                "its other keys kept: {ours}"
            );
        }

        assert_eq!(turn_off_impl(&state, &account).unwrap().outcome, "restored");
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            fixture,
            "byte for byte"
        );
        assert_eq!(status_impl(&state, &account).unwrap().state, "off");
    }
}

#[test]
fn after_the_users_own_edits_only_ai_jugglers_value_comes_out() {
    let work = tempfile::tempdir().unwrap();
    let fixture = FIXTURES[2];
    let (state, account, dir) = setup(&work.path().canonicalize().unwrap(), Some(fixture), true);
    let file = dir.join("settings.json");
    let preview = preview_impl(&state, &account).unwrap();
    turn_on_impl(&state, &account, &preview.fingerprint).unwrap();
    // The user edits another key since.
    let edited = std::fs::read_to_string(&file)
        .unwrap()
        .replace("\"opus\"", "\"sonnet\"");
    std::fs::write(&file, &edited).unwrap();
    assert_eq!(turn_off_impl(&state, &account).unwrap().outcome, "spliced");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        fixture.replace("\"opus\"", "\"sonnet\""),
        "their status line back, their edit kept"
    );
}

#[test]
fn a_status_line_the_user_changed_since_is_theirs_and_left_alone() {
    let work = tempfile::tempdir().unwrap();
    let (state, account, dir) = setup(
        &work.path().canonicalize().unwrap(),
        Some(FIXTURES[1]),
        true,
    );
    let file = dir.join("settings.json");
    let preview = preview_impl(&state, &account).unwrap();
    turn_on_impl(&state, &account, &preview.fingerprint).unwrap();
    let on = std::fs::read_to_string(&file).unwrap();
    let ours = juggler_adapter::jsonc::value_text(&on, "statusLine")
        .unwrap()
        .unwrap();
    let theirs = on.replace(&ours, "{\"type\": \"command\", \"command\": \"new-line\"}");
    std::fs::write(&file, &theirs).unwrap();
    assert_eq!(status_impl(&state, &account).unwrap().state, "changed");
    assert_eq!(turn_off_impl(&state, &account).unwrap().outcome, "theirs");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), theirs, "untouched");
}

#[test]
fn unsafe_states_are_refused_in_words() {
    // No settings.json.
    let work = tempfile::tempdir().unwrap();
    let (state, account, _) = setup(&work.path().canonicalize().unwrap(), None, true);
    let error = preview_impl(&state, &account).unwrap_err().to_string();
    assert!(error.contains("isn't there"), "{error}");

    // Not JSON.
    let work = tempfile::tempdir().unwrap();
    let (state, account, dir) = setup(
        &work.path().canonicalize().unwrap(),
        Some("{\"model\": "),
        true,
    );
    let error = preview_impl(&state, &account).unwrap_err().to_string();
    assert!(error.contains("isn't valid JSON"), "{error}");
    assert_eq!(
        std::fs::read_to_string(dir.join("settings.json")).unwrap(),
        "{\"model\": "
    );

    // Changed between the preview and the change.
    let work = tempfile::tempdir().unwrap();
    let (state, account, dir) = setup(
        &work.path().canonicalize().unwrap(),
        Some(FIXTURES[3]),
        true,
    );
    let preview = preview_impl(&state, &account).unwrap();
    std::fs::write(dir.join("settings.json"), "{\"model\": \"opus\"}").unwrap();
    let error = turn_on_impl(&state, &account, &preview.fingerprint)
        .unwrap_err()
        .to_string();
    assert!(error.contains("changed since the preview"), "{error}");
    assert_eq!(
        std::fs::read_to_string(dir.join("settings.json")).unwrap(),
        "{\"model\": \"opus\"}"
    );
    assert_eq!(status_impl(&state, &account).unwrap().state, "off");

    // A copy isn't offered: only AI Juggler runs sessions on it.
    let work = tempfile::tempdir().unwrap();
    let (state, account, _) = setup(
        &work.path().canonicalize().unwrap(),
        Some(FIXTURES[3]),
        false,
    );
    let status = status_impl(&state, &account).unwrap();
    assert!(!status.offered);
    assert!(preview_impl(&state, &account).is_err());
}

#[test]
fn a_linked_settings_file_stays_a_link_and_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let (state, account, dir) = setup(&root, Some(FIXTURES[1]), true);
    // Their dotfiles manager keeps the real file elsewhere.
    let real = root.join("dotfiles/claude-settings.json");
    std::fs::create_dir_all(real.parent().unwrap()).unwrap();
    std::fs::rename(dir.join("settings.json"), &real).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::os::unix::fs::symlink(&real, dir.join("settings.json")).unwrap();
    let preview = preview_impl(&state, &account).unwrap();
    turn_on_impl(&state, &account, &preview.fingerprint).unwrap();
    assert!(
        std::fs::symlink_metadata(dir.join("settings.json"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        std::fs::read_to_string(&real)
            .unwrap()
            .contains("juggler-statusline.sh")
    );
    assert_eq!(
        std::fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o600
    );
    turn_off_impl(&state, &account).unwrap();
    assert_eq!(std::fs::read_to_string(&real).unwrap(), FIXTURES[1]);
}

#[test]
fn the_isolation_check_allows_exactly_the_key_turned_on() {
    let work = tempfile::tempdir().unwrap();
    let (state, account, dir) = setup(
        &work.path().canonicalize().unwrap(),
        Some(FIXTURES[0]),
        true,
    );
    let row = |id: &str| {
        commands::isolation_check_impl(&state, &account)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
    };
    assert!(row("in_place_settings_untouched").unwrap().pass);
    let preview = preview_impl(&state, &account).unwrap();
    turn_on_impl(&state, &account, &preview.fingerprint).unwrap();
    assert!(row("in_place_capture_only").unwrap().pass);
    assert!(row("in_place_no_juggler_files").unwrap().pass);
    // Any other write of AI Juggler's still fails the check.
    std::fs::write(dir.join("juggler-statusline.sh"), "#!/bin/sh\n").unwrap();
    assert!(!row("in_place_no_juggler_files").unwrap().pass);
}

/// The whole way through, as Claude runs it for a session started in the
/// Claude app or a terminal: the status line command from the user's
/// settings, given Claude's payload, prints the user's own line unchanged,
/// and the limits it carried become a measured sample for the account.
#[test]
fn a_session_started_elsewhere_reports_its_limits_and_still_shows_the_users_line() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let theirs = root.join("my-line.sh");
    std::fs::write(&theirs, "#!/bin/sh\ncat > /dev/null\nprintf 'MY LINE'\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&theirs, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let settings = format!(
        "{{\n  \"statusLine\": {{\"type\": \"command\", \"command\": \"{}\"}}\n}}\n",
        theirs.display()
    );
    let (state, account, dir) = setup(&root, Some(&settings), true);
    let preview = preview_impl(&state, &account).unwrap();
    turn_on_impl(&state, &account, &preview.fingerprint).unwrap();

    // What Claude runs: the command in the user's settings.json.
    let text = std::fs::read_to_string(dir.join("settings.json")).unwrap();
    let line: serde_json::Value = serde_json::from_str(
        &juggler_adapter::jsonc::value_text(&text, "statusLine")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let command = line["command"].as_str().unwrap().to_owned();
    let payload = r#"{"rate_limits":{"five_hour":{"used_percentage":41.0,"resets_at":"2099-01-01T00:00:00Z"}}}"#;
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(child.stdin.as_mut().unwrap(), payload.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "MY LINE",
        "their line, unchanged"
    );

    juggler_app::usage_pipeline::ingest_pass(&state, 4);
    let samples = state
        .storage
        .lock()
        .unwrap()
        .usage()
        .samples_for_account(&account, juggler_core::quota::WindowKind::FiveHour)
        .unwrap();
    assert_eq!(samples.len(), 1, "measured");
    assert!((samples[0].used_pct - 41.0).abs() < 1e-9);
}

/// Runs a status line command as Claude does (`sh -c`, JSON on stdin).
fn status_line(command: &str) -> String {
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(child.stdin.as_mut().unwrap(), b"{}").unwrap();
    String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).into_owned()
}

/// The user's script printing their own line, and their settings using it.
fn their_line(root: &Path) -> String {
    let theirs = root.join("my line.sh");
    std::fs::write(&theirs, "#!/bin/sh\ncat > /dev/null\nprintf 'MY LINE'\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&theirs, std::fs::Permissions::from_mode(0o755)).unwrap();
    format!(
        "{{\n  \"statusLine\": {{\"type\": \"command\", \"command\": {}}}\n}}\n",
        serde_json::to_string(&format!("'{}'", theirs.display())).unwrap()
    )
}

/// The docs fact-check: with AI Juggler's data deleted (the app removed),
/// the line in the user's settings still prints their own status line.
#[test]
fn with_ai_jugglers_data_deleted_the_users_own_line_still_prints() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let (state, account, dir) = setup(&root, Some(&their_line(&root)), true);
    let preview = preview_impl(&state, &account).unwrap();
    assert!(
        preview.after.contains("aij-statusline"),
        "the preview shows the fail-safe line: {}",
        preview.after
    );
    turn_on_impl(&state, &account, &preview.fingerprint).unwrap();
    let text = std::fs::read_to_string(dir.join("settings.json")).unwrap();
    let line: serde_json::Value = serde_json::from_str(
        &juggler_adapter::jsonc::value_text(&text, "statusLine")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let command = line["command"].as_str().unwrap().to_owned();
    assert_eq!(
        status_line(&command),
        "MY LINE",
        "with AI Juggler: their line, through the wrapper"
    );
    std::fs::remove_dir_all(root.join("app-data")).unwrap();
    assert_eq!(
        status_line(&command),
        "MY LINE",
        "AI Juggler's data gone: still their line"
    );
}

/// The docs fact-check: removing the account puts the user's settings
/// back exactly as they were before measuring was turned on.
#[test]
fn removing_the_account_puts_the_users_settings_back() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let before = their_line(&root);
    let (state, account, dir) = setup(&root, Some(&before), true);
    let preview = preview_impl(&state, &account).unwrap();
    turn_on_impl(&state, &account, &preview.fingerprint).unwrap();
    assert_ne!(
        std::fs::read_to_string(dir.join("settings.json")).unwrap(),
        before
    );
    juggler_app::manage::account_remove_impl(
        &state,
        &account,
        juggler_app::manage::CopyFolder::Keep,
        &|_| Ok(()),
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("settings.json")).unwrap(),
        before,
        "every byte back"
    );
    assert!(
        juggler_adapters::quota_ingest::outside_capture_record(
            &root.join("app-data/overlays"),
            &account
        )
        .is_none(),
        "the record is gone with it"
    );
}

/// "Prepare to uninstall" puts every measured account's status line back,
/// deletes nothing, and lists what's left; run again, it has nothing to
/// put back.
#[test]
fn preparing_to_uninstall_puts_status_lines_back_and_lists_whats_left() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().canonicalize().unwrap();
    let before = their_line(&root);
    let (state, account, dir) = setup(&root, Some(&before), true);
    let preview = preview_impl(&state, &account).unwrap();
    turn_on_impl(&state, &account, &preview.fingerprint).unwrap();

    let prep = juggler_app::features::app::uninstall::prepare_impl(&state).unwrap();
    assert_eq!(prep.status_lines.len(), 1);
    assert_eq!(prep.status_lines[0].outcome, "restored");
    assert_eq!(
        std::fs::read_to_string(dir.join("settings.json")).unwrap(),
        before
    );
    assert!(root.join("app-data").is_dir(), "nothing deleted");
    assert!(!prep.data_folder.is_empty());
    assert_eq!(prep.keychain_services.len(), 2);
    assert!(status_impl(&state, &account).is_ok());

    let again = juggler_app::features::app::uninstall::prepare_impl(&state).unwrap();
    assert!(again.status_lines.is_empty(), "already back");
}
