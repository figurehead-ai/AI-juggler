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
