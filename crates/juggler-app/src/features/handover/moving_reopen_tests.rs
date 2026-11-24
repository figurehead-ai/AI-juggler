//! AJ-E2326 R1/R2: a refused or undone move leaves the conversation where
//! it was, open, and no start path continues it here while the move holds
//! it. Scripted processes over the move lab's real folders.

use super::*;
use crate::features::places::{CONVERSATION_MOVING, CONVERSATION_SETTLING, conversation_held};

/// The link sent to each process, and whether the source's new process
/// got one.
fn sent_to(system: &Scripted, pid: u32) -> bool {
    system.sent.lock().unwrap().iter().any(|(to, _)| *to == pid)
}

/// The source's process now.
fn source_pid(lab: &Lab, system: &Scripted) -> u32 {
    system.running(&lab.state, &source_id(lab)).unwrap().0
}

/// What the journal says of the reopen: its line's `seen`.
fn reopened(lab: &Lab) -> Option<serde_json::Value> {
    hold::all_moves(&lab.state.data_dir)
        .last()?
        .journal()
        .into_iter()
        .find(|line| line["event"] == "timeline" && line["phase"] == "reopened")
        .map(|line| line["seen"].clone())
}

/// R2: refused after the backup (the files kept changing): undone, the
/// source started again and the conversation opened in it by the move's
/// own link, seen open there; the conversation settles (no start here).
#[test]
fn a_refused_move_reopens_the_conversation_where_it_was_and_checks_it() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = still_writing(&lab);
    let before = source_pid(&lab, &system);
    run(&lab.state, &lab.sides, &system).unwrap_err();
    let now = source_pid(&lab, &system);
    assert_ne!(now, before, "started again");
    assert!(sent_to(&system, now), "opened in the source");
    let seen = reopened(&lab).expect("the reopen is in the journal");
    assert!(seen.is_string(), "seen open there: {seen}");
    assert!(unchanged(&lab));
    assert_eq!(
        conversation_held(&lab.state, &lab.sides.request.conversation),
        Some(CONVERSATION_SETTLING)
    );
}

/// R2: refused while closing (the target never quit): the source was quit
/// and started again, so the conversation is opened in it.
#[test]
fn a_move_refused_while_closing_reopens_it_in_the_source() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = stubborn_target(&lab);
    run(&lab.state, &lab.sides, &system).unwrap_err();
    let now = source_pid(&lab, &system);
    assert!(sent_to(&system, now), "opened in the source");
    assert!(reopened(&lab).is_some_and(|seen| seen.is_string()));
}

/// R2: refused before anything quit (a terminal session writes it): it
/// is still open where it was, so nothing is sent.
#[test]
fn a_move_refused_before_any_quit_sends_nothing() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = Scripted {
        elsewhere: 1,
        ..running_both(&lab)
    };
    run(&lab.state, &lab.sides, &system).unwrap_err();
    assert!(system.sent.lock().unwrap().is_empty());
}

/// R1: a move that succeeded lets the conversation go at once.
#[test]
fn a_done_move_holds_the_conversation_no_longer() {
    let Some((lab, _, _)) = moved() else {
        return;
    };
    assert_eq!(
        conversation_held(&lab.state, &lab.sides.request.conversation),
        None
    );
}

/// R1: held while the move runs (as the apps quit), for every AI Juggler
/// (the shared area's mark).
#[test]
fn the_conversation_is_held_while_its_move_runs() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let conversation = lab.sides.request.conversation.clone();
    let hold = crate::features::places::ConversationHold::take(&lab.state, &conversation, "m-1");
    assert_eq!(
        conversation_held(&lab.state, &conversation),
        Some(CONVERSATION_MOVING)
    );
    drop(hold);
    // No journal: a move that never began holds nothing.
    assert_eq!(conversation_held(&lab.state, &conversation), None);
}

/// R1: the settle window ends by itself.
#[test]
fn the_settle_window_ends_by_itself() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let conversation = lab.sides.request.conversation.clone();
    let now = juggler_core::model::Timestamp::now().millis();
    crate::features::places::settle_conversation(&lab.state, &conversation, "m-1", now + 60_000);
    assert!(conversation_held(&lab.state, &conversation).is_some());
    crate::features::places::settle_conversation(&lab.state, &conversation, "m-1", now - 1);
    assert_eq!(conversation_held(&lab.state, &conversation), None);
}

/// R2: Undo opens it where it was and checks it.
#[test]
fn undo_reopens_it_where_it_was_and_checks_it() {
    let Some((lab, system, done)) = moved() else {
        return;
    };
    let sent = system.sent.lock().unwrap().len();
    let undone = undo_with(&lab.state, &done.move_id, &system).unwrap();
    assert!(system.sent.lock().unwrap().len() > sent, "a link went");
    assert!(
        undone
            .words
            .starts_with("Undone: the conversation is back where it was.")
    );
    assert!(unchanged(&lab));
    assert_eq!(
        conversation_held(&lab.state, &lab.sides.request.conversation),
        Some(CONVERSATION_SETTLING)
    );
}
