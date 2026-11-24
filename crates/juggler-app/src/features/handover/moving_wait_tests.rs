//! AJ-E2318: what a move waits on (each profile's main process and the
//! writers the tool's adapter names), what it only reports, what it
//! refuses before anything quits, and how it quits (all at once, each
//! process by its identity, checked right before the signal). Scripted
//! processes over the move lab's real folders; no app is quit or started.

use super::*;

/// `secs` after the epoch as Claude Code records a process's start
/// (`procStart`, a ctime in UTC); under an hour only.
fn ctime(secs: i64) -> String {
    format!("Thu Jan  1 00:{:02}:{:02} 1970", secs / 60, secs % 60)
}

/// Claude Code's pid file in the source's code folder: session `session`
/// runs in `pid`, started at `started` (epoch seconds).
fn pid_file(lab: &Lab, pid: u32, session: &str, started: i64) {
    let dir = lab.a_code.join("sessions");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{pid}.json")),
        format!(
            r#"{{"pid":{pid},"sessionId":"{session}","status":"idle","procStart":"{}"}}"#,
            ctime(started)
        ),
    )
    .unwrap();
}

/// A move run with `kids` and both profiles running.
fn run_with(lab: &Lab, kids: Vec<Kid>) -> (Scripted, AppResult<MoveDoneInfo>) {
    let system = Scripted {
        kids,
        ..running_both(lab)
    };
    let done = run(&lab.state, &lab.sides, &system);
    (system, done)
}

/// A dev server the profile's session started in the background (it
/// outlives the app): reported in plain words, never waited on. Before,
/// it refused every move after both apps were quit.
#[test]
fn a_dev_server_under_the_session_is_reported_not_waited_on() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let (_, done) = run_with(&lab, vec![kid(&lab, 710, "node", &["server.js"], true)]);
    let done = done.unwrap();
    assert_eq!(done.payload.state, "verified", "{}", done.payload.detail);
    let source = place_name(&lab.state, &lab.sides.source_place.as_ref().unwrap().1);
    assert!(
        done.notes.contains(&format!(
            "node that {source} started is still running; the move doesn't need it closed"
        )),
        "{:?}",
        done.notes
    );
}

/// Electron's crash reporter (launchd's child, naming the profile's
/// Crashpad folder) and the app's helpers never hold a move back.
#[test]
fn crashpad_and_helpers_never_block() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let (_, done) = run_with(
        &lab,
        vec![
            kid(
                &lab,
                711,
                "chrome_crashpad_handler",
                &["--database=/Users/you/Library/Application Support/ClaudeProfiles/work/Crashpad"],
                true,
            ),
            kid(
                &lab,
                712,
                "Claude Helper (Renderer)",
                &[
                    "--type=renderer",
                    "--user-data-dir=/Users/you/Library/Application Support/ClaudeProfiles/work",
                ],
                true,
            ),
        ],
    );
    let done = done.unwrap();
    assert_eq!(done.payload.state, "verified", "{}", done.payload.detail);
    assert!(
        done.notes
            .iter()
            .any(|n| n.contains("Claude Helper") && n.contains("chrome_crashpad_handler")),
        "{:?}",
        done.notes
    );
}

/// AJ-E2315's shape: another profile's `disclaimer` wrapper, its
/// `--add-dir=` inside the moving profile, counted in its tree: it writes
/// none of the family, so it never blocks (before: refused on every try).
#[test]
fn another_profiles_wrapper_never_blocks() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let (_, done) = run_with(
        &lab,
        vec![kid(
            &lab,
            713,
            "disclaimer",
            &[
                "--pgroup",
                "--",
                "/Users/you/Library/Application Support/ClaudeProfiles/team-a/claude-code/2.1/claude.app/Contents/MacOS/claude",
                "--resume=other",
                "--add-dir=/Users/you/Library/Application Support/ClaudeProfiles/work/local-agent-mode-sessions",
            ],
            true,
        )],
    );
    assert_eq!(done.unwrap().payload.state, "verified");
}

/// A session of the profile resuming a family member (`--resume <id>`)
/// writes it: the move waits for it, and refuses calmly when it doesn't
/// end, nothing moved.
#[test]
fn a_resume_writer_blocks() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let (system, done) = run_with(
        &lab,
        vec![kid(&lab, 714, "claude", &["--resume", "kid"], true)],
    );
    let words = done.unwrap_err().wire().user_message;
    assert!(
        words.contains("claude still runs, so nothing was moved"),
        "{words}"
    );
    assert_eq!(*system.quits.lock().unwrap(), [500, 501]);
    assert!(unchanged(&lab));
}

/// A session whose pid file names a family member, its start matching
/// the process's: a writer, waited on.
#[test]
fn a_session_pid_file_blocks() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    pid_file(&lab, 720, "kid", 720);
    let (_, done) = run_with(&lab, vec![kid(&lab, 720, "claude", &[], true)]);
    let words = done.unwrap_err().wire().user_message;
    assert!(
        words.contains("claude still runs, so nothing was moved"),
        "{words}"
    );
    assert!(unchanged(&lab));
}

/// A pid file left behind, its pid now a process that started later (a
/// start mismatch): stale, never a writer.
#[test]
fn a_stale_pid_file_never_blocks() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    pid_file(&lab, 721, "kid", 3000);
    let (_, done) = run_with(&lab, vec![kid(&lab, 721, "claude", &[], true)]);
    assert_eq!(done.unwrap().payload.state, "verified");
}

/// A writer no profile the move closes runs (a terminal `claude
/// --resume`): the offer says so, naming it, and the move refuses before
/// anything quits (before: both apps were quit first).
#[test]
fn the_offer_refuses_an_outside_writer_before_any_quit() {
    let lab = lab();
    let system = Scripted {
        kids: vec![kid(&lab, 730, "claude", &["--resume=main"], false)],
        ..running_both(&lab)
    };
    let said = "claude (process 730) has this conversation open outside the apps the move closes (in a terminal, say)";
    let offer = offer_of(&lab.state, &lab.sides, &system);
    let waits: Vec<&str> = offer
        .blocks
        .iter()
        .filter(|b| b.kind == MoveBlockKind::UserCanClear)
        .map(|b| b.words.as_str())
        .collect();
    let block = format!("{said}; Move becomes available once you end it");
    assert!(waits.contains(&block.as_str()), "{waits:?}");
    let words = run(&lab.state, &lab.sides, &system)
        .unwrap_err()
        .wire()
        .user_message;
    assert!(words.contains(said), "{words}");
    assert!(system.quits.lock().unwrap().is_empty(), "nothing quit");
    assert!(system.starts.lock().unwrap().is_empty(), "nothing started");
    assert_nothing_begun(&lab);
}

/// Both profiles are asked to quit at once (each still running when the
/// other is asked), then waited on together.
#[test]
fn both_profiles_quit_in_parallel() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = Scripted {
        slow: Some(Duration::from_millis(150)),
        wait: Some(Duration::from_secs(5)),
        ..running_both(&lab)
    };
    moved_with(&lab, &system);
    assert_eq!(*system.quits.lock().unwrap(), [500, 501]);
    assert_eq!(
        *system.running_at_quit.lock().unwrap(),
        [2, 2],
        "the second asked while the first still ran"
    );
}

/// A pid given to another program between the move's look and its
/// signal is never signalled: nothing quits, the move refuses.
#[test]
fn a_pid_reused_between_read_and_signal_is_never_signalled() {
    let lab = lab();
    let system = Scripted {
        reused_after_read: vec![source_id(&lab)],
        ..running_both(&lab)
    };
    let words = run(&lab.state, &lab.sides, &system)
        .unwrap_err()
        .wire()
        .user_message;
    assert!(words.contains("couldn't be sure which process"), "{words}");
    assert!(system.quits.lock().unwrap().is_empty(), "never signalled");
    assert!(unchanged(&lab));
}

/// The start/move race: a start that comes while a move holds the
/// profile sees the hold (checked and started under the hold's lock), and
/// starts once it lets go.
#[test]
fn a_start_racing_a_move_sees_its_hold() {
    let lab = lab();
    let (id, place) = lab.sides.target_place.clone().unwrap();
    let hosts = crate::features::conversations::places::HostRegistry::of(&lab.state.registry);
    let starter = crate::features::places::fakes::Recorder::default();
    let hold =
        crate::features::places::MoveHold::take(&lab.state, std::slice::from_ref(&id), "m-1")
            .unwrap();
    let refused = crate::features::places::start_place(&lab.state, &place, &hosts, &starter, true);
    assert!(refused.is_err(), "{refused:?}");
    assert!(starter.starts.lock().unwrap().is_empty(), "nothing started");
    drop(hold);
    crate::features::places::start_place(&lab.state, &place, &hosts, &starter, true).unwrap();
    assert_eq!(starter.starts.lock().unwrap().len(), 1);
}
