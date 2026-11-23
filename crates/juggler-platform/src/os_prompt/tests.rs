use super::*;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

#[derive(Default)]
struct Kept(Mutex<HashSet<String>>);

impl Ledger for Kept {
    fn primed(&self, id: &str) -> bool {
        self.0.lock().unwrap().contains(id)
    }

    fn set_primed(&self, id: &str) {
        self.0.lock().unwrap().insert(id.to_owned());
    }
}

/// A primer that answers `answer` and logs, in order, what happened.
struct Sheet {
    answer: Answer,
    log: Mutex<Vec<String>>,
}

impl Sheet {
    fn new(answer: Answer) -> Self {
        Self {
            answer,
            log: Mutex::new(Vec::new()),
        }
    }

    fn note(&self, line: String) {
        self.log.lock().unwrap().push(line);
    }

    fn log(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl Primer for Sheet {
    fn ask(&self, prompt: Prompt) -> Answer {
        self.note(format!("sheet {}", prompt.id));
        self.answer
    }

    fn told(&self, prompt: Prompt, outcome: &Outcome) {
        self.note(format!("told {} {}", prompt.id, outcome.status.as_str()));
    }
}

const ACCESS: Prompt = Prompt {
    id: "accessibility",
    kind: Kind::Accessibility,
};

#[test]
fn the_sheet_comes_before_the_call_and_the_outcome_after() {
    let ledger = Kept::default();
    let sheet = Sheet::new(Answer::Continue);
    let gated = Gate::new(&ledger).through(
        ACCESS,
        Presence::User(&sheet),
        &|| {
            (!sheet.log().iter().any(|l| l == "call"))
                .then_some(Status::NotAsked)
                .or(Some(Status::NotAllowed))
        },
        || sheet.note("call".to_owned()),
    );
    assert_eq!(
        sheet.log(),
        [
            "sheet accessibility",
            "call",
            "told accessibility not_allowed"
        ]
    );
    assert!(matches!(gated, Gated::Called { .. }));
    assert!(ledger.primed("accessibility"), "Continue primes the entry");
}

#[test]
fn not_now_never_calls_and_never_primes() {
    let ledger = Kept::default();
    let sheet = Sheet::new(Answer::NotNow);
    let gated = Gate::new(&ledger).through(ACCESS, Presence::User(&sheet), &|| None, || {
        sheet.note("call".to_owned());
    });
    assert_eq!(gated, Gated::NotNow);
    assert_eq!(sheet.log(), ["sheet accessibility"]);
    assert!(!ledger.primed("accessibility"));
}

#[test]
fn background_work_never_prompts_until_primed() {
    let ledger = Kept::default();
    let called = std::cell::Cell::new(0);
    let gate = Gate::new(&ledger);
    let first = gate.through(
        ACCESS,
        Presence::Background,
        &|| Some(Status::NotAsked),
        || {
            called.set(called.get() + 1);
        },
    );
    assert_eq!(first, Gated::Waiting);
    assert_eq!(called.get(), 0, "nothing was called without the user");
    // Primed in front of the user (the setup's card), background may go on.
    ledger.set_primed("accessibility");
    let later = gate.through(
        ACCESS,
        Presence::Background,
        &|| Some(Status::NotAsked),
        || {
            called.set(called.get() + 1);
        },
    );
    assert!(matches!(later, Gated::Called { .. }));
    assert_eq!(called.get(), 1);
}

#[test]
fn an_entry_the_system_already_allows_needs_no_sheet() {
    let ledger = Kept::default();
    let sheet = Sheet::new(Answer::NotNow);
    let gated = Gate::new(&ledger).through(
        ACCESS,
        Presence::User(&sheet),
        &|| Some(Status::Allowed),
        || 7,
    );
    assert_eq!(gated.value(), Some(7));
    assert!(sheet.log().is_empty(), "nothing will be asked, so no sheet");
    let background = Gate::new(&ledger).through(
        ACCESS,
        Presence::Background,
        &|| Some(Status::Allowed),
        || 8,
    );
    assert_eq!(background.value(), Some(8));
}

#[test]
fn an_entry_the_system_already_declined_gets_no_sheet_since_no_dialog_follows() {
    let ledger = Kept::default();
    let sheet = Sheet::new(Answer::Continue);
    let gated = Gate::new(&ledger).through(
        ACCESS,
        Presence::User(&sheet),
        &|| Some(Status::NotAllowed),
        || 3,
    );
    assert!(
        sheet.log().is_empty(),
        "a sheet with no dialog after it is never shown"
    );
    assert!(
        matches!(&gated, Gated::Called { value: 3, outcome } if outcome.status == Status::NotAllowed),
        "the call is made (it can't ask), and the outcome says not allowed"
    );
}

#[test]
fn not_now_is_never_a_decline_for_good_and_continue_is_never_asked_twice() {
    let ledger = Kept::default();
    let gate = Gate::new(&ledger);
    let no = Sheet::new(Answer::NotNow);
    assert_eq!(
        gate.through(ACCESS, Presence::User(&no), &|| None, || ()),
        Gated::NotNow
    );
    // The next click asks again.
    let yes = Sheet::new(Answer::Continue);
    assert!(matches!(
        gate.through(ACCESS, Presence::User(&yes), &|| None, || ()),
        Gated::Called { .. }
    ));
    assert_eq!(yes.log()[0], "sheet accessibility");
    // Once said yes to, the same sheet never shows again.
    let again = Sheet::new(Answer::NotNow);
    assert!(matches!(
        gate.through(ACCESS, Presence::User(&again), &|| None, || ()),
        Gated::Called { .. }
    ));
    assert!(again.log().is_empty());
}

#[test]
fn a_priming_card_primes_then_calls_without_a_second_sheet() {
    let ledger = Kept::default();
    let gated = Gate::new(&ledger).through(ACCESS, Presence::Primed, &|| None, || 1);
    assert_eq!(gated.value(), Some(1));
    assert!(ledger.primed("accessibility"));
}

#[test]
fn the_outcome_says_granted_or_declined_with_the_way_back() {
    assert_eq!(
        Outcome::of(Kind::Accessibility, Some(Status::Allowed)).words,
        "Allowed."
    );
    let declined = Outcome::of(Kind::Accessibility, Some(Status::NotAllowed));
    assert_eq!(declined.status, Status::NotAllowed);
    let unknown = Outcome::of(Kind::Notifications, None);
    assert_eq!(unknown.status, Status::Asked);
    if cfg!(target_os = "macos") {
        assert_eq!(
            declined.words,
            "Not allowed. Nothing else changes. You can change it later in System Settings › Privacy & Security › Accessibility."
        );
        assert!(
            unknown.words.ends_with("System Settings › Notifications."),
            "{}",
            unknown.words
        );
    }
}

#[test]
fn before_the_app_installs_its_ledger_everything_waits() {
    // No install in this test binary: nothing is primed, Not now is said.
    let gated = gate().through(ACCESS, Presence::Background, &|| None, || ());
    assert_eq!(gated, Gated::Waiting);
    let gated = gate().through(ACCESS, user(), &|| None, || ());
    assert_eq!(gated, Gated::NotNow);
}

#[test]
fn every_kind_has_the_systems_words_and_its_button() {
    let kinds = [
        Kind::Accessibility,
        Kind::Automation(Target {
            bundle_id: "com.apple.systemevents",
            name: "System Events",
        }),
        Kind::Notifications,
        Kind::LoginItem,
        Kind::ScreenRecording,
        Kind::LocalNetwork,
        Kind::Folder(Folder::Documents),
        Kind::AppData,
        Kind::Differs,
        Kind::Keychain,
        Kind::FirstOpen,
    ];
    for kind in kinds {
        let found = dialog(kind, "AI Juggler");
        assert_eq!(found.is_some(), cfg!(target_os = "macos"), "{kind:?}");
        let Some(dialog) = found else {
            continue;
        };
        assert!(!dialog.title.is_empty(), "{kind:?}");
        assert!(
            dialog.click.is_empty() && dialog.style == Style::Banner
                || dialog.buttons.contains(&dialog.click),
            "{kind:?}: the button to click is one of its buttons"
        );
    }
}

#[test]
fn protected_folders_and_other_apps_data_are_told_apart() {
    let home = Path::new("/Users/someone");
    let on_mac = cfg!(target_os = "macos");
    assert_eq!(
        protected_folder(&home.join("Documents/payments-api"), home),
        on_mac.then_some(Folder::Documents)
    );
    assert_eq!(
        protected_folder(&home.join("code/payments-api"), home),
        None
    );
    assert_eq!(protected_folder(&home.join("DocumentsOld"), home), None);
    assert_eq!(
        other_apps_data(&home.join("Library/Containers/com.example.app/Data"), home),
        on_mac
    );
    assert!(!other_apps_data(
        &home.join("Library/Application Support/Claude"),
        home
    ));
}

#[test]
fn a_download_never_approved_to_open_is_refused_not_opened() {
    // 0x0083: downloaded, quarantined, never opened; 0x00c3: opened once.
    assert!(first_open_pending(0x0083));
    assert!(!first_open_pending(0x00c3));
    assert!(!first_open_pending(0x01c1));
}
