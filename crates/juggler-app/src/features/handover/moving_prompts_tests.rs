//! 0.5.1 F93 (PAT-OSP): a move's system prompts are asked on the Move
//! screen before anything quits, never inside the move; declined, the
//! move still runs and never waits with its Apps closed. Scripted
//! processes over the move lab's real folders, and a prompt gate of our
//! own (a ledger in memory, a sheet that records when it was asked).

use juggler_platform::os_prompt::{
    Answer, Gate, MemoryLedger, Outcome, Presence, Primer, Prompt, Status,
};

use super::*;
use crate::os_prompts::registry;

/// The sheet: answers `go`, and records how many Apps the move had
/// asked to quit when it was shown.
struct Sheet<'a> {
    system: &'a Scripted,
    go: bool,
    quits_when_asked: Mutex<Vec<usize>>,
}

impl Primer for Sheet<'_> {
    fn ask(&self, _: Prompt) -> Answer {
        let quits = self.system.quits.lock().unwrap().len();
        self.quits_when_asked.lock().unwrap().push(quits);
        if self.go {
            Answer::Continue
        } else {
            Answer::NotNow
        }
    }

    fn told(&self, _: Prompt, _: &Outcome) {}
}

impl Sheet<'_> {
    fn asked(&self) -> Vec<usize> {
        self.quits_when_asked.lock().unwrap().clone()
    }
}

/// The Move button's step (the window's `os_prompt_request` for each of
/// the offer's `prompts`, a priming-only entry): the sheet, then primed
/// on Continue.
fn answered(ledger: &MemoryLedger, sheet: &Sheet<'_>, ids: &[String]) {
    for id in ids {
        let prompt = registry::entry(id).unwrap().prompt;
        let _ = Gate::new(ledger).through(prompt, Presence::User(sheet), &|| None, || ());
    }
}

/// Both profiles running; links go through the move's own gate.
fn gated(lab: &Lab) -> Scripted {
    Scripted {
        links: Some(MemoryLedger::default()),
        ..running_both(lab)
    }
}

/// The offer names the link's prompt while it isn't primed or decided,
/// and only where Apps are involved.
#[test]
fn the_offer_names_each_prompt_a_move_can_cause_until_it_is_primed() {
    let links = registry::APP_LINKS.id.to_owned();
    let ledger = MemoryLedger::default();
    let gate = Gate::new(&ledger);
    let unknown = |_: Prompt| Status::NotAsked;
    assert_eq!(prompts::unprimed(gate, &unknown, true), vec![links.clone()]);
    assert!(
        prompts::unprimed(gate, &unknown, false).is_empty(),
        "a terminal move sends no link"
    );
    for decided in [Status::Allowed, Status::NotAllowed] {
        assert!(
            prompts::unprimed(gate, &|_| decided, true).is_empty(),
            "the system decided"
        );
    }
    juggler_platform::os_prompt::Ledger::set_primed(&ledger, &links);
    assert!(
        prompts::unprimed(gate, &unknown, true).is_empty(),
        "primed once"
    );
    // The real offer, on a computer where nothing was primed.
    let lab = lab();
    let offer = offer_of(&lab.state, &lab.sides, &Scripted::default());
    assert_eq!(offer.prompts, vec![links]);
}

/// The coordinator's rule: an unprimed move never quits an App before
/// its prompts are answered. The sheet is shown before the first quit,
/// and the move itself asks nothing (before 0.5.1 F93 the sheet waited
/// inside the move; PL1 saw it hold both Apps closed for 600 s).
#[test]
fn an_unprimed_move_never_quits_an_app_before_its_prompts_are_answered() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = gated(&lab);
    let sheet = Sheet {
        system: &system,
        go: true,
        quits_when_asked: Mutex::new(Vec::new()),
    };
    let offer = offer_of(&lab.state, &lab.sides, &system);
    assert_eq!(offer.prompts, vec![registry::APP_LINKS.id.to_owned()]);
    answered(system.links.as_ref().unwrap(), &sheet, &offer.prompts);
    assert_eq!(sheet.asked(), vec![0], "asked once, with nothing quit");
    let done = run_once_more(&lab.state, &lab.sides, &system).unwrap();
    assert_eq!(sheet.asked(), vec![0], "the move itself asked nothing");
    assert_eq!(
        system.quits.lock().unwrap().len(),
        2,
        "both Apps quit, after"
    );
    assert_eq!(
        system.sent.lock().unwrap().len(),
        1,
        "primed: the link went"
    );
    assert_eq!(done.opened.state, "verified", "{}", done.opened.detail);
}

/// Declined (Not now): the move still runs, both Apps start again, no
/// link is sent and nothing waits for the conversation to open; it says
/// how to open it, and offers Open in the App it moved to.
#[test]
fn a_declined_prompt_still_moves_and_says_how_to_open_it_without_waiting() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = gated(&lab);
    let sheet = Sheet {
        system: &system,
        go: false,
        quits_when_asked: Mutex::new(Vec::new()),
    };
    let offer = offer_of(&lab.state, &lab.sides, &system);
    answered(system.links.as_ref().unwrap(), &sheet, &offer.prompts);
    let started = std::time::Instant::now();
    let done = moved_with(&lab, &system);
    assert!(
        started.elapsed() < timeline::OPEN_CAP,
        "never waited to see it open"
    );
    assert_eq!(sheet.asked(), vec![0], "asked once, before; never again");
    assert!(
        system.sent.lock().unwrap().is_empty(),
        "no link without a yes"
    );
    assert!(!system.stages().contains(&"opening".to_owned()));
    for id in place_ids(&lab) {
        assert!(
            system.running(&lab.state, &id).is_some(),
            "{id} started again"
        );
    }
    assert_eq!(done.opened.state, "pending", "{}", done.opened.detail);
    assert!(
        done.opened.detail.contains("sidebar") && done.opened.detail.contains("Open"),
        "{}",
        done.opened.detail
    );
    assert_eq!(done.open_place_id, Some(target_id(&lab)));
    // Its Undo asks nothing either: back where it was, how to open it said.
    let undone = undo_with(&lab.state, &done.move_id, &system).unwrap();
    assert_eq!(sheet.asked(), vec![0]);
    assert!(system.sent.lock().unwrap().is_empty());
    assert!(undone.words.contains("Open it from"), "{}", undone.words);
}
