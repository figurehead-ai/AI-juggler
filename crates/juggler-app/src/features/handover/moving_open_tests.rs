//! AJ-E2322: "opened" only when the conversation really is open in the
//! target app (the adapter's evidence after the link), waited for without
//! a fixed settle, and the move's timeline in its journal.

use super::*;

/// R1: the target app took the link but nothing shows the conversation
/// open there (no session of it runs, nothing added to its history):
/// pending, said plainly, with Open offered in that App.
#[test]
fn a_link_delivered_but_not_seen_open_is_pending_with_open_offered() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = Scripted {
        opens: false,
        ..running_both(&lab)
    };
    let done = moved_with(&lab, &system);
    assert_eq!(system.sent.lock().unwrap().len(), 1, "the link went");
    assert_eq!(done.opened.state, "pending", "{}", done.opened.detail);
    assert!(
        done.opened.detail.contains("couldn't see it open")
            && done.opened.detail.contains("Open it from"),
        "{}",
        done.opened.detail
    );
    let target_id = &lab.sides.target_place.as_ref().unwrap().0;
    assert_eq!(done.open_place_id.as_ref(), Some(target_id));
    assert_eq!(done.conversation, "main");
}

/// R1: the target's Claude Code resuming the conversation is the
/// adapter's evidence: verified, and no Open offered.
#[test]
fn a_session_of_it_running_in_the_target_is_opened() {
    let Some((_, system, done)) = moved() else {
        return;
    };
    assert_eq!(done.opened.state, "verified", "{}", done.opened.detail);
    assert!(
        done.opened.detail.starts_with("opened in "),
        "{}",
        done.opened.detail
    );
    assert_eq!(done.open_place_id, None);
    assert!(system.stages().contains(&"opening".to_owned()));
}

/// R2: no fixed settle after the link: the wait looks (by event in the
/// real system), at most its cap, and ends as soon as the conversation
/// shows open.
#[test]
fn the_checks_after_wait_for_the_open_not_a_fixed_settle() {
    let Some((_, system, _)) = moved() else {
        return;
    };
    let settles = system.settles.lock().unwrap();
    assert!(
        settles.iter().all(|for_| *for_ < Duration::from_secs(6)),
        "{settles:?}"
    );
    let lab = lab();
    let slow = Scripted {
        opens: false,
        ..running_both(&lab)
    };
    run(&lab.state, &lab.sides, &slow).unwrap();
    {
        let looks = slow
            .settles
            .lock()
            .unwrap()
            .iter()
            .filter(|for_| **for_ == timeline::LOOK)
            .count();
        assert_eq!(
            u128::try_from(looks).unwrap(),
            timeline::OPEN_CAP.as_millis() / timeline::LOOK.as_millis(),
            "capped at 15 s of looks"
        );
    }
}

/// R3: every phase in the journal with its ms, the closing wait's set
/// (program, reason, evidence) and what remained.
#[test]
fn the_journal_keeps_each_phase_its_wait_set_and_what_remained() {
    let Some((lab, _, done)) = moved() else {
        return;
    };
    let txn = hold::all_moves(&lab.state.data_dir)
        .into_iter()
        .find(|txn| txn.manifest.id == done.move_id)
        .unwrap();
    let journal = txn.journal();
    let phases: Vec<&str> = journal
        .iter()
        .filter(|line| line["event"] == "timeline")
        .filter_map(|line| line["phase"].as_str())
        .collect();
    assert_eq!(
        phases,
        [
            "closing",
            "still",
            "backing_up",
            "moving",
            "checking_before",
            "starting",
            "opening",
            "checking_after"
        ]
    );
    for line in journal.iter().filter(|line| line["event"] == "timeline") {
        assert!(line["took_ms"].is_u64() && line["at_ms"].is_i64(), "{line}");
    }
    let closing = &journal
        .iter()
        .find(|line| line["phase"] == "closing")
        .unwrap();
    let waited = closing["waited"].as_array().unwrap();
    assert_eq!(waited.len(), 2, "both apps' main processes: {closing}");
    assert!(
        waited.iter().all(|w| w["program"].is_string()
            && w["reason"] == "the app"
            && w["evidence"].is_string()),
        "{closing}"
    );
    assert!(closing["remained"].is_array());
    let opening = journal
        .iter()
        .find(|line| line["phase"] == "opening")
        .unwrap();
    assert!(
        opening["seen"]
            .as_str()
            .is_some_and(|s| s.contains("process 777")),
        "{opening}"
    );
    // Copy details: the same timeline, in words.
    let record = moves(&lab.state)
        .into_iter()
        .find(|m| m.move_id == done.move_id)
        .unwrap();
    assert!(
        record
            .timeline
            .iter()
            .any(|line| line.starts_with("timeline: ") && line.contains("phase opening")),
        "{:?}",
        record.timeline
    );
}

/// 0.5.1 F69: a Copy switch's "opened" uses this same check. A link the
/// app took with nothing showing it open (no session of it running, its
/// history unchanged) is not seen open, after the capped looks.
#[test]
fn a_copy_link_not_seen_open_says_none() {
    let lab = lab();
    let system = running_both(&lab);
    let history = lab.b_code.join("projects").join(SLUG).join("main.jsonl");
    std::fs::write(&history, "{\"type\":\"user\"}\n").unwrap();
    let len = std::fs::metadata(&history).unwrap().len();
    let (place, _) = lab.sides.target_place.as_ref().unwrap();
    let seen = seen_open_after_link(
        &lab.state,
        (place, 4242),
        lab.sides.adapter.in_place_move().unwrap(),
        (&lab.b_code, "main"),
        Some(&(history, len)),
        &system,
    );
    assert_eq!(seen, None);
    let looks = system
        .settles
        .lock()
        .unwrap()
        .iter()
        .filter(|for_| **for_ == timeline::LOOK)
        .count();
    assert_eq!(
        u128::try_from(looks).unwrap(),
        timeline::OPEN_CAP.as_millis() / timeline::LOOK.as_millis(),
        "capped, as Move's"
    );
}

/// 0.5.1 F69: the app adding to the copy's history after the link is the
/// evidence it opened it: seen open, said in words.
#[test]
fn a_copy_the_app_adds_to_is_seen_open() {
    let lab = lab();
    let history = lab.b_code.join("projects").join(SLUG).join("main.jsonl");
    std::fs::write(&history, "{\"type\":\"user\"}\n").unwrap();
    let len = std::fs::metadata(&history).unwrap().len();
    let system = Scripted {
        writer: Mutex::new(Some((history.clone(), b"{\"type\":\"system\"}\n", 1))),
        ..running_both(&lab)
    };
    let (place, _) = lab.sides.target_place.as_ref().unwrap();
    let seen = seen_open_after_link(
        &lab.state,
        (place, 4242),
        lab.sides.adapter.in_place_move().unwrap(),
        (&lab.b_code, "main"),
        Some(&(history, len)),
        &system,
    );
    assert_eq!(
        seen.as_deref(),
        Some("the app added to its history after the link")
    );
}
