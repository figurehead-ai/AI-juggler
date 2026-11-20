//! AJ-E3013: keyless lines under load: a looping caller limited by its
//! peer (L7), the keyless lines' common cap (M2), a flood that can't
//! starve members (the global-cap finding), and an attested session's one
//! allowance (the fifth finding). Split from `attest_tests` (its size
//! budget); its lab and helpers are shared.

use std::sync::Arc;

use juggler_agentbridge::wire::{Args, Call, Refusal, Request};

use super::attest_tests::{caller, conversation, full_lab, lab, message_for, peer, wait_of};

/// L7: keyless lines are limited by their peer before any attestation:
/// past 30 in 10 s the answer is Limit, not another attempt.
#[test]
fn a_looping_keyless_caller_is_limited_before_it_is_attested() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = lab(dir.path());
    let peer = peer(4_000_000);
    let line =
        serde_json::to_string(&Request::new(String::new(), Call::Whoami, Args::default())).unwrap();
    let answers: Vec<_> = (0..31)
        .map(|_| super::calls::handle_line_from(&state, &line, Some(peer)).refused)
        .collect();
    assert!(
        answers[..30]
            .iter()
            .all(|refusal| *refusal == Some(Refusal::Unattested))
    );
    assert_eq!(
        answers[30],
        Some(Refusal::Limit),
        "the 31st isn't attested at all"
    );
    // Another peer has its own allowance.
    let other = juggler_platform::local_ipc::Peer {
        pid: Some(4_000_001),
        ..peer
    };
    assert_eq!(
        super::calls::handle_line_from(&state, &line, Some(other)).refused,
        Some(Refusal::Unattested)
    );
    // A connector hook's keyless report is limited the same way.
    let report = serde_json::to_string(&juggler_agentbridge::wire::Report {
        v: juggler_agentbridge::wire::VERSION,
        token: String::new(),
        hook: juggler_agentbridge::wire::HookReport {
            event: "UserPromptSubmit".into(),
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(
        super::calls::handle_line_from(&state, &report, Some(peer)).refused,
        Some(Refusal::Limit)
    );
}

/// M2: 40 connectors wait at once; a hostile caller loops (unattested,
/// and as a session that already waits): its work stays bounded by the
/// peer limit and one wait a session, no connector is refused, and a
/// member a message is for still wakes.
#[test]
fn forty_connectors_and_a_hostile_loop_stay_bounded_and_members_still_wake() {
    use juggler_agentbridge::wire::Refusal;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = full_lab(dir.path(), 0..40);
    let asked: Arc<Mutex<HashMap<u32, usize>>> = Arc::default();
    let attestor: super::desktop::Attestor = {
        let asked = Arc::clone(&asked);
        Arc::new(move |_state, peer| {
            let pid = peer.pid.unwrap_or_default();
            *asked.lock().unwrap().entry(pid).or_default() += 1;
            if !(1000..1040).contains(&pid) {
                return Err("not a desktop session".to_owned());
            }
            let n = pid - 1000;
            Ok(caller(
                n,
                1,
                juggler_core::model::Timestamp::now().millis() / 1_000,
            ))
        })
    };
    let wait = wait_of(3);
    let look = Duration::from_millis(50);
    tauri::async_runtime::block_on(async {
        let waiting: Vec<_> = (1000..1040)
            .map(|pid| {
                tauri::async_runtime::spawn(super::desktop::wait_with(
                    Arc::clone(&state),
                    wait.clone(),
                    peer(pid),
                    Arc::clone(&attestor),
                    look,
                ))
            })
            .collect();
        tokio::time::sleep(Duration::from_millis(300)).await;
        // A hostile loop, unattested: limited by its peer before more
        // attestations than the limit allows.
        let mut limited = 0;
        for _ in 0..200 {
            let answer = super::desktop::wait_with(
                Arc::clone(&state),
                wait.clone(),
                peer(9_999),
                Arc::clone(&attestor),
                look,
            )
            .await;
            assert!(answer.result.is_none(), "{answer:?}");
            limited += usize::from(answer.refused == Some(Refusal::Limit));
        }
        assert_eq!(limited, 170, "past 30 in 10 s: Limit");
        assert_eq!(asked.lock().unwrap()[&9_999], 30, "at most 30 attestations");
        // The same, as a session that already waits: never a second wait.
        for _ in 0..50 {
            let again = super::desktop::wait_with(
                Arc::clone(&state),
                wait.clone(),
                peer(1_007),
                Arc::clone(&attestor),
                look,
            )
            .await;
            assert_eq!(again.refused, Some(Refusal::Limit));
        }
        // A message for member 7 wakes it while the others wait on.
        message_for(&state, &pool, "m-7", &conversation(7));
        let mut answers = Vec::new();
        for handle in waiting {
            answers.push(handle.await.unwrap());
        }
        for (n, answer) in answers.iter().enumerate() {
            assert!(
                answer.refused.is_none(),
                "connector {n} refused: {answer:?}"
            );
            let wake = &answer.result.as_ref().unwrap()["wake"];
            if n == 7 {
                assert!(wake.as_str().unwrap().contains("Planner"), "{wake}");
            } else {
                assert!(wake.is_null(), "connector {n}: {wake}");
            }
        }
    });
}

/// The review's global-cap finding, as one decision: a flood of new
/// processes fills the common cap; a process not attested lately is then
/// told Busy (its own allowance not spent), a member attested lately is
/// served, and right after the flood's window the newcomer has its whole
/// allowance.
#[test]
fn a_flood_fills_the_common_cap_but_never_a_members_lines_or_a_newcomers_allowance() {
    use super::calls::{Gate, KEYLESS_MAX, MAX_CALLS, WINDOW, gate};
    use std::collections::HashMap;
    use std::time::{Duration, Instant};
    let mut calls = HashMap::new();
    let start = Instant::now();
    for pid in 0..KEYLESS_MAX {
        assert_eq!(
            gate(&mut calls, &format!("peer:{pid}"), false, start),
            Gate::Served
        );
    }
    for _ in 0..MAX_CALLS * 2 {
        assert_eq!(gate(&mut calls, "peer:newcomer", false, start), Gate::Busy);
    }
    for _ in 0..MAX_CALLS {
        assert_eq!(
            gate(&mut calls, "peer:member", true, start),
            Gate::Served,
            "attested lately: out of the common cap"
        );
    }
    assert_eq!(
        gate(&mut calls, "peer:member", true, start),
        Gate::Own,
        "its own allowance still holds"
    );
    let after = start + WINDOW + Duration::from_millis(1);
    for _ in 0..MAX_CALLS {
        assert_eq!(
            gate(&mut calls, "peer:newcomer", false, after),
            Gate::Served,
            "Busy spent none of it"
        );
    }
    assert_eq!(gate(&mut calls, "peer:newcomer", false, after), Gate::Own);
}

/// The review's global-cap finding, end to end: members' connectors wait
/// (attested); a flood of new processes' waits fills the common cap
/// (Busy, its own reason); a member's next wait is still served and wakes
/// at once for its message, during the flood and right after it, while a
/// connector never attested is told Busy.
#[test]
fn members_still_wake_promptly_during_and_right_after_a_flood() {
    use std::time::{Duration, Instant};
    let dir = tempfile::tempdir().unwrap();
    let (state, pool) = full_lab(dir.path(), [50, 51, 52]);
    let attestor: super::desktop::Attestor = Arc::new(|_state, peer| {
        let pid = peer.pid.unwrap_or_default();
        if !(1050..1053).contains(&pid) {
            return Err("not a desktop session".to_owned());
        }
        let n = pid - 1000;
        Ok(caller(
            n,
            u32::MAX,
            juggler_core::model::Timestamp::now().millis() / 1_000,
        ))
    });
    let look = Duration::from_millis(50);
    let wait_as = |pid: u32, secs: u32| {
        super::desktop::wait_with(
            Arc::clone(&state),
            wait_of(secs),
            peer(pid),
            Arc::clone(&attestor),
            look,
        )
    };
    tauri::async_runtime::block_on(async {
        // Members 50 and 51 have waited before (attested).
        for pid in [1050, 1051] {
            let first = wait_as(pid, 0).await;
            assert!(first.refused.is_none(), "{first:?}");
        }
        // The flood: 300 new processes, one wait each.
        let mut busy = 0;
        for pid in 0..300 {
            let answer = wait_as(5_000 + pid, 0).await;
            busy += usize::from(answer.refused == Some(Refusal::Busy));
        }
        assert!(busy >= 300 - super::calls::KEYLESS_MAX, "{busy} busy");
        // A connector never attested is told Busy, its own reason.
        assert_eq!(wait_as(1052, 3).await.refused, Some(Refusal::Busy));
        // During the flood, member 50's wait is served and wakes at once;
        // right after it (its lines still in the window), member 51's too.
        for n in [50, 51] {
            let started = Instant::now();
            let waiting = tauri::async_runtime::spawn(wait_as(1000 + n, 3));
            tokio::time::sleep(Duration::from_millis(100)).await;
            message_for(&state, &pool, &format!("m-{n}"), &conversation(n));
            let woke = waiting.await.unwrap();
            assert!(woke.refused.is_none(), "{n}: {woke:?}");
            assert!(
                woke.result.unwrap()["wake"]
                    .as_str()
                    .unwrap()
                    .contains("Planner")
            );
            assert!(started.elapsed() < Duration::from_secs(1), "{n} promptly");
        }
    });
}

/// The fifth finding: the lines of one attested session's processes (its
/// connector, its hooks, anything it runs) share one allowance, keyed by
/// its tool's pid and version, not one each; a process never attested has
/// its own.
#[test]
fn an_attested_sessions_processes_share_one_allowance() {
    use super::calls::{Counted, MAX_CALLS};
    let dir = tempfile::tempdir().unwrap();
    let (state, _) = lab(dir.path());
    // Its tool: this test's own process (a version the kernel gives).
    let tool = std::process::id();
    super::calls::note_attested(&state, peer(4_100), tool);
    super::calls::note_attested(&state, peer(4_101), tool);
    let line = |pid: u32| {
        super::calls::keyless_refused(
            &state,
            Counted {
                pid: Some(pid),
                version: Some(1),
            },
            "own",
        )
    };
    let served = (0..MAX_CALLS)
        .filter(|n| line(if n % 2 == 0 { 4_100 } else { 4_101 }).is_none())
        .count();
    assert_eq!(served, MAX_CALLS, "together, up to the one allowance");
    assert_eq!(
        line(4_101).map(|refused| refused.refused),
        Some(Some(Refusal::Limit)),
        "the session's allowance is spent, whichever process asks"
    );
    assert!(line(4_102).is_none(), "a process never attested: its own");
}

/// The fifth finding: a full table of attested processes makes room by
/// dropping the oldest one, never every member's exemption at once.
#[test]
fn a_full_table_of_attested_processes_drops_the_oldest_only() {
    use super::calls::{Kept, make_room};
    use std::collections::HashMap;
    use std::time::{Duration, Instant};
    let base = Instant::now();
    let mut kept: HashMap<u32, Kept> = (0..1_024_u32)
        .map(|pid| {
            (
                pid,
                Kept {
                    version: 1,
                    at: base + Duration::from_millis(u64::from(pid)),
                    session: (pid, 1),
                },
            )
        })
        .collect();
    make_room(&mut kept, 1_024, base + Duration::from_secs(2));
    assert_eq!(kept.len(), 1_023);
    assert!(!kept.contains_key(&0), "the oldest went");
    assert!(
        kept.contains_key(&1) && kept.contains_key(&1_023),
        "the rest stay"
    );
    // Those past their time go first, all of them.
    make_room(
        &mut kept,
        1_023,
        base + super::calls::ATTESTED_FOR + Duration::from_millis(512),
    );
    assert!(kept.len() < 1_023 && kept.contains_key(&1_023));
    assert!(
        kept.values()
            .all(|one| one.at >= base + Duration::from_millis(512))
    );
}
