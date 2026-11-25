use super::*;
use std::collections::HashSet;
use std::path::Path;

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn every_entry_is_complete_with_a_short_stable_id() {
    let mut ids = HashSet::new();
    for entry in registry::ENTRIES {
        let id = entry.prompt.id;
        assert!(ids.insert(id), "{id}: one entry per id");
        assert!(
            !id.is_empty()
                && id.len() <= 28
                && id.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'),
            "{id}: short, lowercase, dashes only (the walks' log names it)"
        );
        for (what, text) in [
            ("title", entry.title),
            ("why", entry.why),
            ("if_declined", entry.if_declined),
            ("offered_again", entry.offered_again),
            ("background", entry.background),
        ] {
            assert!(!text.trim().is_empty(), "{id}: {what} is empty");
        }
        assert!(!entry.triggers.is_empty(), "{id}: names its code paths");
        if cfg!(target_os = "macos") {
            let dialog = juggler_platform::os_prompt::dialog(entry.prompt.kind, "AI Juggler");
            assert!(dialog.is_some(), "{id}: the system's words");
        }
    }
}

/// Every gated path is named, and names a function its file has.
#[test]
fn every_trigger_names_a_function_in_its_file() {
    for entry in registry::ENTRIES {
        for trigger in entry.triggers {
            let parts = trigger.split_once(':');
            assert!(parts.is_some(), "{trigger}: file:function");
            let (file, function) = parts.unwrap();
            let text = std::fs::read_to_string(root().join(file));
            assert!(text.is_ok(), "{}: {file} is there", entry.prompt.id);
            let text = text.unwrap();
            assert!(
                text.contains(&format!("fn {function}("))
                    || text.contains(&format!("fn {function}<")),
                "{}: {file} has fn {function}",
                entry.prompt.id
            );
        }
    }
}

/// The committed registry (`docs/design/os-prompts.md`, PAT-OSP-1) lists
/// every entry with every trigger: the code and the doc never drift.
#[test]
fn the_design_doc_lists_every_entry_and_its_triggers() {
    let doc = std::fs::read_to_string(root().join("docs/design/os-prompts.md")).unwrap();
    for entry in registry::ENTRIES {
        let id = entry.prompt.id;
        assert!(doc.contains(&format!("`{id}`")), "{id} in the registry doc");
        for trigger in entry.triggers {
            assert!(doc.contains(trigger), "{id}: {trigger} in the registry doc");
        }
    }
}

/// Background work never prompts: a session that waits (or a watch, on
/// from the start) shows no notification until the entry is primed.
#[test]
fn background_notifications_wait_until_primed() {
    let ledger = juggler_platform::os_prompt::MemoryLedger::default();
    let shown = Mutex::new(0);
    let show = |_: &crate::features::notifications::Outgoing| *shown.lock().unwrap() += 1;
    let outgoing = crate::features::notifications::Outgoing {
        id: 1,
        title: "payments-api".to_owned(),
        body: "Waiting for you".to_owned(),
    };
    let gate = juggler_platform::os_prompt::Gate::new(&ledger);
    assert!(!notify::deliver(gate, &outgoing, &show));
    assert_eq!(*shown.lock().unwrap(), 0, "nothing shown, so nothing asked");
    ledger.set_primed(registry::NOTIFICATIONS.id);
    assert!(notify::deliver(gate, &outgoing, &show));
    assert_eq!(*shown.lock().unwrap(), 1);
}

#[test]
fn a_program_that_isnt_ours_starts_without_the_gate() {
    let command = std::process::Command::new("/bin/echo");
    assert!(helper_may_start(&command));
}

#[test]
fn the_click_words_say_the_button_and_the_switch() {
    assert_eq!(click_words("Allow", None), "Click Allow.");
    assert_eq!(click_words("", None), "Nothing: it's a notice.");
    let to_settings = click_words(
        "Open System Settings",
        Some("System Settings › Privacy & Security › Accessibility"),
    );
    assert!(
        to_settings.starts_with("Click Open System Settings, then turn on "),
        "{to_settings}"
    );
}

#[test]
fn the_list_offers_only_what_this_setup_can_use_and_never_a_built_in_silence() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(dir.path().to_path_buf());
    let list = list_impl(&state);
    assert_eq!(list.len(), registry::ENTRIES.len());
    for info in &list {
        assert!(
            ["allowed", "not_allowed", "not_asked", "asked"].contains(&info.status.as_str()),
            "{}: {}",
            info.id,
            info.status
        );
        if info.never_shown {
            assert!(!info.needed, "{}: never offered", info.id);
        }
    }
    // No project in a protected folder, no Link, no desktop app: none of
    // their prompts is offered (D-127).
    for id in [
        "documents",
        "desktop",
        "downloads",
        "app-data",
        "local-network",
        "automation-app-links",
        "automation-terminal",
    ] {
        let info = list.iter().find(|info| info.id == id).unwrap();
        assert!(!info.needed, "{id} isn't needed by an empty setup");
    }
}

#[test]
fn an_entry_built_never_to_be_shown_cant_be_primed() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(dir.path().to_path_buf());
    for id in ["keychain", "first-open"] {
        let entry = registry::entry(id).unwrap();
        assert!(prime_call(&state, entry, Presence::Primed).is_err(), "{id}");
    }
}

/// Without the window's sheet installed (a test run), a click's call is
/// Not now: nothing asks, nothing is marked.
#[test]
fn not_now_without_a_window_asks_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(dir.path().to_path_buf());
    let entry = registry::entry("automation-app-links").unwrap();
    let outcome = prime_call(&state, entry, juggler_platform::os_prompt::user()).unwrap();
    assert_eq!(outcome.result, "not_now");
    assert!(outcome.words.contains("ask again"), "{}", outcome.words);
}

/// 0.5.1 F28: after Not now the sheet says where (or when) it's offered
/// again: one sentence, a Settings place or the next use that asks.
#[test]
fn not_now_says_where_it_is_offered_again() {
    for entry in registry::ENTRIES.iter().filter(|entry| !entry.never_shown) {
        let again = entry.offered_again;
        assert!(again.ends_with('.'), "{}: one sentence", entry.prompt.id);
        assert!(
            again.contains("Settings ›")
                || again.starts_with("It asks again the next time")
                || again.starts_with("Open the app yourself"),
            "{}: a place in Settings or the next use that asks: {again}",
            entry.prompt.id
        );
    }
    assert!(
        registry::entry("notifications")
            .unwrap()
            .offered_again
            .contains("Settings › Notifications")
    );
    assert!(
        registry::entry("helper")
            .unwrap()
            .offered_again
            .contains("Settings › AI › Models")
    );
}

/// AJ-E1555 (was AJ-E3822's rename): a helper set up before whose
/// signature changed doesn't start unannounced: its next start waits for
/// the `helper-changed` sheet, once, and priming that is enough.
#[test]
fn a_changed_helper_waits_for_its_one_sheet() {
    use juggler_platform::os_prompt::{Gate, Ledger, MemoryLedger};
    let ledger = MemoryLedger::default();
    ledger.set_primed(registry::HELPER.id);
    let gate = Gate::new(&ledger);
    assert!(helper_start_through(gate, false), "unchanged: as before");
    assert!(
        !helper_start_through(gate, true),
        "changed: the helper waits for the user to be told"
    );
    assert_eq!(helper_prompt(gate, true).id, registry::HELPER_CHANGED.id);
    ledger.set_primed(registry::HELPER_CHANGED.id);
    assert!(helper_start_through(gate, true), "told once: it starts");
    assert_eq!(helper_prompt(gate, true).id, registry::HELPER.id);
}

/// AJ-E1555 R1: the "differs" prompt is needed only when the helper's
/// signature on disk differs from the one last opened: never for a
/// helper never opened here, never because a Dev build's data folder sits
/// beside ours, and no longer once the changed one opened.
#[test]
fn helper_changed_only_when_its_signature_on_disk_differs() {
    assert!(
        !signature_changed(None, Some("team:A")),
        "never opened here"
    );
    assert!(!signature_changed(Some("team:A"), Some("team:A")));
    assert!(
        !signature_changed(Some("team:A"), None),
        "unreadable: not claimed"
    );
    assert!(signature_changed(Some("team:A"), Some("file:00ff")));

    let dir = tempfile::tempdir().unwrap();
    // A Dev build's data folder beside ours changes nothing.
    std::fs::create_dir_all(
        dir.path()
            .join(crate::edition::Edition::Dev.data_dir_name()),
    )
    .unwrap();
    let state = crate::test_support::bare_state(dir.path().join("AI Juggler Nightly"));
    let helper = dir.path().join("juggler-helper-fixture");
    std::fs::write(&helper, b"build one").unwrap();
    assert!(
        !helper_changed_at(&state, &helper),
        "never opened: the first-start entry covers it"
    );
    note_opened(&state, &helper);
    assert!(!helper_changed_at(&state, &helper), "the one last opened");
    std::fs::write(&helper, b"build two").unwrap();
    assert!(helper_changed_at(&state, &helper), "an update replaced it");
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set("os_prompt_primed:helper-changed", "1")
        .unwrap();
    note_opened(&state, &helper);
    assert!(
        !helper_changed_at(&state, &helper),
        "opened: the same again"
    );
    assert_eq!(
        state
            .app_setting("os_prompt_primed:helper-changed")
            .as_deref(),
        Some("0"),
        "the next change is told again"
    );
}

/// AJ-E1555 R2: every entry's sheet words are real: its title isn't
/// repeated as its why, and no outcome body is a placeholder or a state
/// word, whatever the system says.
#[test]
fn every_sheet_says_something_real() {
    use juggler_platform::os_prompt::Status;
    const PLACEHOLDERS: &[&str] = &[
        "",
        "Asked.",
        "Asked",
        "Allowed",
        "Allowed.",
        "Not allowed.",
        "Done.",
    ];
    for entry in registry::ENTRIES {
        let id = entry.prompt.id;
        assert_ne!(entry.title.trim(), entry.why.trim(), "{id}: title repeated");
        for words in [
            entry.title,
            entry.why,
            entry.if_declined,
            entry.offered_again,
            entry.after,
        ] {
            assert!(!PLACEHOLDERS.contains(&words.trim()), "{id}: {words:?}");
        }
        for status in [
            Status::Allowed,
            Status::NotAllowed,
            Status::NotAsked,
            Status::Asked,
        ] {
            let body = outcome_words(entry, status);
            assert!(
                !PLACEHOLDERS.contains(&body.trim()),
                "{id} {status:?}: {body:?}"
            );
            assert!(
                body.split_whitespace().count() >= 4,
                "{id} {status:?}: {body:?}"
            );
            assert_ne!(
                body.trim(),
                entry.title.trim(),
                "{id}: the title as its body"
            );
        }
    }
}
