//! AJ-E706 integration: two app instances (two data dirs, one process)
//! over loopback, each with a stub `claude`. Proves: cross-machine
//! targets carry the machine, an `Unvalidated` badge and their
//! preconditions; no handover bytes flow before the click; T2 runs only
//! with the same harness version and the same checkout path (or a
//! mapping the user confirmed), otherwise it is blocked with reasons and
//! T3 replay is offered — never a silent downgrade; the destination
//! re-checks at execution; both machines record the same lineage edge;
//! the source transcript never changes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::crossmachine::{self, FleetSwitchRequest};
use juggler_app::state::AppState;
use juggler_app::{commands, fleet, link};
use juggler_link::Permission;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn claude_stub(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("claude-stub");
    stand_in::write(
        &path,
        // Written whole, then moved into place: a reader never sees it
        // half-written (the same fix as parked_reset's stub).
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/last-args.tmp\" && mv \"$CLAUDE_CONFIG_DIR/last-args.tmp\" \"$CLAUDE_CONFIG_DIR/last-args\"\n",
    );
    path
}

fn app_state(dir: &Path, version: &str) -> Arc<AppState> {
    let stub = claude_stub(dir);
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    let state = Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap());
    link::link_disable_discovery_for_test(&state);
    crossmachine::set_harness_versions_for_test(&state, &[(CLAUDE_CODE_ID, version)]);
    state
}

fn wait_for<T>(what: &str, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn phase(info: link::PairingStateInfo, phase: &str) -> Option<()> {
    (info.phase == phase).then_some(())
}

/// `host` grants `joiner` `grant` (the joiner then acts as a console of
/// the host) and the joiner grants the host `back`.
fn pair(host: &Arc<AppState>, joiner: &Arc<AppState>, grant: Permission, back: Permission) {
    let code = link::pairing_host_open_impl(host).unwrap().code.unwrap();
    let address = format!("127.0.0.1:{}", link::link_status_impl(host).port.unwrap());
    link::pairing_join_start_impl(joiner, None, Some(&address), &code).unwrap();
    wait_for("joiner confirm", || {
        phase(link::pairing_join_state_impl(joiner).unwrap(), "confirm")
    });
    wait_for("host confirm", || {
        phase(link::pairing_host_state_impl(host).unwrap(), "confirm")
    });
    link::pairing_join_decide_impl(joiner, true, back).unwrap();
    link::pairing_host_decide_impl(host, true, grant).unwrap();
    wait_for("paired", || {
        (phase(link::pairing_join_state_impl(joiner).unwrap(), "paired").is_some()
            && phase(link::pairing_host_state_impl(host).unwrap(), "paired").is_some())
        .then_some(())
    });
}

fn append_turn(path: &Path, uuid: &str, role: &str, text: &str, cwd: &str) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    writeln!(
        file,
        "{{\"type\":\"{role}\",\"uuid\":\"{uuid}\",\"sessionId\":\"native-5\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"{role}\",\"content\":\"{text}\"}}}}"
    )
    .unwrap();
}

/// Two Macs: the laptop (source, console of the desktop) and the desktop
/// (destination). Returns the laptop's source Instance id.
struct Pair {
    _work: tempfile::TempDir,
    laptop: Arc<AppState>,
    desktop: Arc<AppState>,
    source: String,
    source_transcript: PathBuf,
    desktop_recipe: String,
    desktop_project: String,
    desktop_root: PathBuf,
    desktop_account: String,
    laptop_account: String,
    laptop_cwd: String,
}

fn setup(desktop_version: &str, same_path: bool, grant: Permission) -> Pair {
    setup_both(desktop_version, same_path, grant, Permission::View)
}

fn setup_both(desktop_version: &str, same_path: bool, grant: Permission, back: Permission) -> Pair {
    let work = tempfile::tempdir().unwrap();
    let laptop = app_state(&work.path().join("laptop"), "2.1.0");
    let desktop = app_state(&work.path().join("desktop"), desktop_version);
    fleet::fleet_set_label_impl(&desktop, "Studio Mac").unwrap();
    fleet::fleet_set_label_impl(&laptop, "Laptop").unwrap();

    // The laptop's work in `storefront`.
    let project_dir = work.path().join("storefront");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let account = commands::create_account(&laptop, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let profile = commands::create_profile(&laptop, &account.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&laptop, &profile.id, Some(cwd.clone())).unwrap();
    let transcript = laptop
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(&account.id)
        .join("projects")
        .join(cwd.replace('/', "-"))
        .join("native-5.jsonl");
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    append_turn(&transcript, "u1", "user", "add the checkout page", &cwd);
    append_turn(
        &transcript,
        "u2",
        "assistant",
        "checkout scaffolded; key sk-ant-api03-SECRETSECRETSECRETSECRET1234 set",
        &cwd,
    );
    {
        let storage = laptop.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.clone(),
            ))
            .unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: transcript.clone(),
            native_id: Some("native-5".to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }

    // The desktop: its own Account and Launch setting, and the project
    // — at the same path, or as its own copy elsewhere.
    let desktop_account =
        commands::create_account(&desktop, CLAUDE_CODE_ID, "Desktop Max", None).unwrap();
    let desktop_profile =
        commands::create_profile(&desktop, &desktop_account.id, "Main", None, None).unwrap();
    let desktop_root = if same_path {
        project_dir.canonicalize().unwrap()
    } else {
        let copy = work.path().join("desktop-src").join("storefront");
        std::fs::create_dir_all(&copy).unwrap();
        copy.canonicalize().unwrap()
    };
    let desktop_project = {
        let storage = desktop.storage.lock().unwrap();
        storage
            .projects()
            .touch(&desktop_root, None)
            .unwrap()
            .id
            .to_string()
    };

    link::link_set_enabled_impl(&laptop, true).unwrap();
    link::link_set_enabled_impl(&desktop, true).unwrap();
    // The desktop grants the laptop `grant` (the laptop is its console).
    pair(&desktop, &laptop, grant, back);
    Pair {
        _work: work,
        laptop,
        desktop,
        source: launched.session_id,
        source_transcript: transcript,
        desktop_recipe: desktop_profile.id,
        desktop_project,
        desktop_root,
        desktop_account: desktop_account.id,
        laptop_account: account.id,
        laptop_cwd: cwd,
    }
}

fn target(pair: &Pair, readiness: &str) -> crossmachine::RemoteSwitchTargetInfo {
    wait_for(&format!("a {readiness} target"), || {
        crossmachine::fleet_switch_targets_impl(&pair.laptop, &pair.source)
            .unwrap()
            .into_iter()
            .find(|row| row.recipe_id == pair.desktop_recipe && row.readiness == readiness)
    })
}

fn request(pair: &Pair, mechanism: &str) -> FleetSwitchRequest {
    FleetSwitchRequest {
        session_id: pair.source.clone(),
        machine_id: fleet::fleet_self_impl(&pair.desktop).unwrap().machine_id,
        recipe_id: pair.desktop_recipe.clone(),
        mechanism: mechanism.to_owned(),
        project_id: None,
        mapping_confirmed: false,
        freeze_choice: "finish-turn".to_owned(),
        redaction_choice: None,
    }
}

fn execute(
    pair: &Pair,
    request: FleetSwitchRequest,
) -> (
    juggler_app::error::AppResult<crossmachine::FleetSwitchOutcomeInfo>,
    Vec<crossmachine::FleetSwitchProgressInfo>,
) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut progress = Vec::new();
    let outcome = runtime.block_on(crossmachine::fleet_switch_execute_impl(
        Arc::clone(&pair.laptop),
        request,
        |update| progress.push(update),
    ));
    (outcome, progress)
}

fn audit_kinds(state: &Arc<AppState>) -> Vec<String> {
    fleet::verify_audit(state)
        .unwrap()
        .into_iter()
        .map(|entry| entry["kind"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn last_args(state: &Arc<AppState>, account: &str) -> String {
    let path = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(account)
        .join("last-args");
    wait_for("the destination launched", || {
        std::fs::read_to_string(&path).ok()
    })
}

fn sha(path: &Path) -> String {
    juggler_adapter::handover::sha256_hex(&std::fs::read(path).unwrap())
}

#[test]
fn t2_at_the_same_path_switches_to_the_other_mac_with_shared_lineage() {
    let pair = setup("2.1.0", true, Permission::Control);
    let row = target(&pair, "t2_ready");
    assert_eq!(row.machine_label, "Studio Mac");
    assert_eq!(row.account_label, "Desktop Max");
    assert_eq!(row.mechanism, "transplant");
    assert!(row.badge.ends_with("· Unvalidated"), "{}", row.badge);
    assert!(!row.validated);
    assert_eq!(
        row.same_path_project.as_deref(),
        Some(pair.desktop_project.as_str())
    );

    // Target list with preconditions: in-memory (published catalogs +
    // the cached version probe) — far inside the 1 s budget.
    let started = Instant::now();
    let _ = crossmachine::fleet_switch_targets_impl(&pair.laptop, &pair.source).unwrap();
    let took = started.elapsed();
    eprintln!("[perf] cross-machine target list with preconditions: {took:?}");
    assert!(took < Duration::from_millis(500), "{took:?}");
    // Browsing targets moved nothing: no handover on either side.
    assert!(
        !audit_kinds(&pair.desktop)
            .iter()
            .any(|k| k.starts_with("handover"))
    );
    assert!(
        !audit_kinds(&pair.laptop)
            .iter()
            .any(|k| k.starts_with("handover"))
    );
    let before = sha(&pair.source_transcript);

    let (outcome, progress) = execute(&pair, request(&pair, "transplant"));
    let outcome = outcome.unwrap();
    assert!(outcome.accepted, "{outcome:?}");
    assert!(progress.iter().any(|update| update.phase == "preparing"));
    assert!(progress.iter().any(|update| update.phase == "done"));
    let instance = outcome.instance_id.clone().unwrap();
    let edge = outcome.edge_id.clone().unwrap();

    // The desktop resumed the transplanted native session in its project.
    let args = last_args(&pair.desktop, &pair.desktop_account);
    assert!(
        args.contains("--resume") && args.contains("native-5"),
        "{args}"
    );
    let desktop_row = {
        let storage = pair.desktop.storage.lock().unwrap();
        storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(instance.clone()))
            .unwrap()
    };
    let installed = desktop_row.transcript_ref.clone().unwrap().path;
    assert!(installed.is_file());
    assert!(installed.starts_with(pair.desktop.accounts_dir()));

    // Copy-never-move: the laptop's transcript is byte-identical.
    assert_eq!(sha(&pair.source_transcript), before);

    // The same edge on both machines, each with the other's epoch.
    let laptop_id = fleet::fleet_self_impl(&pair.laptop).unwrap().machine_id;
    let desktop_id = fleet::fleet_self_impl(&pair.desktop).unwrap().machine_id;
    for (state, other) in [(&pair.laptop, &desktop_id), (&pair.desktop, &laptop_id)] {
        let storage = state.storage.lock().unwrap();
        let graph = storage.switch_graph();
        let found = graph
            .edge(&juggler_core::model::SwitchEdgeId::new(edge.clone()))
            .unwrap();
        assert!(
            matches!(found.state, juggler_core::model::EdgeState::Confirmed),
            "{:?}",
            found.state
        );
        let snapshot = graph.graph(&found.work).unwrap();
        assert!(
            snapshot
                .epochs
                .iter()
                .any(|epoch| epoch.machine.as_ref().map(|m| m.as_str()) == Some(other.as_str())),
            "a mirror of the other machine's epoch"
        );
    }
    {
        let storage = pair.laptop.storage.lock().unwrap();
        let found = storage
            .switch_graph()
            .edge(&juggler_core::model::SwitchEdgeId::new(edge.clone()))
            .unwrap();
        assert_eq!(found.to_machine.unwrap().as_str(), desktop_id);
        let source = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(pair.source.clone()))
            .unwrap();
        assert_eq!(
            source.status,
            juggler_core::model::SessionStatus::HandedOver
        );
    }
    // Audited on both machines.
    assert!(audit_kinds(&pair.laptop).contains(&"handover_sent".to_owned()));
    assert!(audit_kinds(&pair.desktop).contains(&"handover_accepted".to_owned()));
}

#[test]
fn another_checkout_path_needs_a_confirmed_mapping_and_the_session_is_remapped() {
    let pair = setup("2.1.0", false, Permission::Control);
    let row = target(&pair, "needs_mapping");
    assert!(
        row.blockers[0].contains("isn't at the same path"),
        "{:?}",
        row.blockers
    );
    let copy = row
        .projects
        .iter()
        .find(|project| project.project_id == pair.desktop_project)
        .unwrap();
    assert!(
        copy.same_name && !copy.same_path,
        "a suggestion, never assumed"
    );

    // Unmapped: T2 is blocked with an explanation.
    let (refused, _) = execute(&pair, request(&pair, "transplant"));
    assert!(refused.is_err());
    // Mapped but not confirmed: still blocked.
    let mut unconfirmed = request(&pair, "transplant");
    unconfirmed.project_id = Some(pair.desktop_project.clone());
    assert!(execute(&pair, unconfirmed).0.is_err());
    assert!(
        !audit_kinds(&pair.desktop)
            .iter()
            .any(|k| k.starts_with("handover"))
    );

    let mut confirmed = request(&pair, "transplant");
    confirmed.project_id = Some(pair.desktop_project.clone());
    confirmed.mapping_confirmed = true;
    let outcome = execute(&pair, confirmed).0.unwrap();
    assert!(outcome.accepted, "{outcome:?}");
    let installed = {
        let storage = pair.desktop.storage.lock().unwrap();
        storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                outcome.instance_id.unwrap(),
            ))
            .unwrap()
            .transcript_ref
            .unwrap()
            .path
    };
    let text = std::fs::read_to_string(installed).unwrap();
    let target_root = pair.desktop_root.to_string_lossy().into_owned();
    assert!(
        text.contains(&format!("\"cwd\":\"{target_root}\"")),
        "{text}"
    );
    assert!(
        !text.contains(&format!("\"cwd\":\"{}\"", pair.laptop_cwd)),
        "every working directory follows the mapping"
    );
}

#[test]
fn a_version_mismatch_blocks_t2_offers_t3_and_never_downgrades_silently() {
    let pair = setup("2.0.9", true, Permission::Control);
    let row = target(&pair, "t2_blocked");
    assert_eq!(row.mechanism, "replay", "T3 offered instead");
    assert!(row.replay_available);
    assert!(
        row.blockers
            .iter()
            .any(|reason| reason.contains("2.1.0 here, 2.0.9 on Studio Mac")),
        "{:?}",
        row.blockers
    );
    // Asking for T2 anyway is refused — never quietly replayed.
    assert!(execute(&pair, request(&pair, "transplant")).0.is_err());
    // T3 without a secret choice is refused too (D-14: unset blocks).
    assert!(execute(&pair, request(&pair, "replay")).0.is_err());

    let mut replay = request(&pair, "replay");
    replay.redaction_choice = Some("strip".to_owned());
    let outcome = execute(&pair, replay).0.unwrap();
    assert!(outcome.accepted, "{outcome:?}");
    let args = last_args(&pair.desktop, &pair.desktop_account);
    assert!(
        args.contains("add the checkout page"),
        "primed with the turns: {args}"
    );
    assert!(
        !args.contains("SECRETSECRET"),
        "the secret never reached the other Mac: {args}"
    );
}

#[test]
fn a_view_only_pairing_lists_the_machine_but_cannot_switch_to_it() {
    let pair = setup("2.1.0", true, Permission::View);
    let row = target(&pair, "unavailable");
    assert!(
        row.blockers
            .iter()
            .any(|reason| reason.contains("view only")),
        "{:?}",
        row.blockers
    );
    assert!(execute(&pair, request(&pair, "transplant")).0.is_err());
    assert!(
        !audit_kinds(&pair.desktop)
            .iter()
            .any(|k| k.starts_with("handover"))
    );
}

#[test]
fn the_destination_rechecks_at_execution_and_the_edge_records_the_failure() {
    let pair = setup("2.1.0", true, Permission::Control);
    let _ = target(&pair, "t2_ready");
    // The desktop's Claude updates after the console looked.
    crossmachine::set_harness_versions_for_test(&pair.desktop, &[(CLAUDE_CODE_ID, "2.2.0")]);
    let outcome = execute(&pair, request(&pair, "transplant")).0.unwrap();
    assert!(!outcome.accepted);
    assert_eq!(outcome.reason_code.as_deref(), Some("version_mismatch"));
    let storage = pair.laptop.storage.lock().unwrap();
    let edge = storage
        .switch_graph()
        .edge(&juggler_core::model::SwitchEdgeId::new(
            outcome.edge_id.unwrap(),
        ))
        .unwrap();
    assert!(
        matches!(edge.state, juggler_core::model::EdgeState::Failed { .. }),
        "{:?}",
        edge.state
    );
    let source = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(pair.source.clone()))
        .unwrap();
    assert_ne!(
        source.status,
        juggler_core::model::SessionStatus::HandedOver
    );
}

// ------------------------------------------------- switch-back (D-05)

/// A → B by T2, then the away session on B gains two turns.
fn switched_and_worked(pair: &Pair) -> String {
    let _ = target(pair, "t2_ready");
    let outcome = execute(pair, request(pair, "transplant")).0.unwrap();
    assert!(outcome.accepted, "{outcome:?}");
    let away = outcome.instance_id.unwrap();
    let installed = {
        let storage = pair.desktop.storage.lock().unwrap();
        storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(away.clone()))
            .unwrap()
            .transcript_ref
            .unwrap()
            .path
    };
    let cwd = pair.desktop_root.to_string_lossy().into_owned();
    append_turn(&installed, "u3", "user", "now the payment form", &cwd);
    append_turn(&installed, "u4", "assistant", "payment form added", &cwd);
    away
}

fn back_plan(pair: &Pair, away: &str) -> crossmachine::switch_back::FleetSwitchBackPlanInfo {
    wait_for("a switch-back plan", || {
        crossmachine::switch_back::fleet_switch_back_plan_impl(&pair.desktop, away)
            .unwrap()
            .filter(|plan| plan.blockers.is_empty())
    })
}

fn switch_back(pair: &Pair, away: &str, choice: &str) -> crossmachine::FleetSwitchOutcomeInfo {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime
        .block_on(crossmachine::switch_back::fleet_switch_back_execute_impl(
            Arc::clone(&pair.desktop),
            away.to_owned(),
            choice.to_owned(),
            false,
            |_| {},
        ))
        .unwrap()
}

fn clear_args(state: &Arc<AppState>, account: &str) {
    let _ = std::fs::remove_file(
        state
            .accounts_dir()
            .join(CLAUDE_CODE_ID)
            .join(account)
            .join("last-args"),
    );
}

#[test]
fn switching_back_resumes_the_original_with_one_delta_turn_and_never_copies_back() {
    let pair = setup_both("2.1.0", true, Permission::Control, Permission::Control);
    let away = switched_and_worked(&pair);
    let original_before = sha(&pair.source_transcript);
    let plan = back_plan(&pair, &away);
    assert_eq!(plan.original_machine_label, "Laptop");
    assert_eq!(plan.new_turns, 2, "only the turns added while away");
    assert!(
        plan.delta_preview.contains("payment form"),
        "{}",
        plan.delta_preview
    );
    assert!(
        !plan.delta_preview.contains("add the checkout page"),
        "turns the original already has are never re-sent"
    );
    clear_args(&pair.laptop, &pair.laptop_account);
    let outcome = switch_back(&pair, &away, "");
    assert!(outcome.accepted, "{outcome:?}");
    // The laptop resumed the ORIGINAL native session with the one turn.
    let args = last_args(&pair.laptop, &pair.laptop_account);
    assert!(
        args.contains("--resume") && args.contains("native-5"),
        "{args}"
    );
    assert!(args.contains("While you were away"), "{args}");
    // The copy-back ban: the original's bytes never changed.
    assert_eq!(sha(&pair.source_transcript), original_before);
    // The same switch-back edge on both machines, confirmed.
    let edge = juggler_core::model::SwitchEdgeId::new(outcome.edge_id.unwrap());
    for state in [&pair.laptop, &pair.desktop] {
        let storage = state.storage.lock().unwrap();
        let found = storage.switch_graph().edge(&edge).unwrap();
        assert!(matches!(
            found.state,
            juggler_core::model::EdgeState::Confirmed
        ));
        assert_eq!(
            found.mechanism,
            juggler_core::model::SwitchMechanism::NativeResume
        );
    }
    let storage = pair.desktop.storage.lock().unwrap();
    let away_row = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(away))
        .unwrap();
    assert_eq!(
        away_row.status,
        juggler_core::model::SessionStatus::HandedOver
    );
}

#[test]
fn a_diverged_original_blocks_switch_back_until_the_user_chooses() {
    let pair = setup_both("2.1.0", true, Permission::Control, Permission::Control);
    let away = switched_and_worked(&pair);
    // Meanwhile the laptop's original moved on too.
    append_turn(
        &pair.source_transcript,
        "u9",
        "user",
        "quick fix on the laptop",
        &pair.laptop_cwd,
    );
    let original_after = sha(&pair.source_transcript);
    let _ = back_plan(&pair, &away);
    clear_args(&pair.laptop, &pair.laptop_account);

    let started = Instant::now();
    let refused = switch_back(&pair, &away, "");
    let took = started.elapsed();
    eprintln!("[perf] diverged refusal round trip (detect + report): {took:?}");
    assert!(took < Duration::from_secs(2), "{took:?}");
    assert!(!refused.accepted);
    assert_eq!(refused.reason_code.as_deref(), Some("diverged"));
    let divergence = refused.divergence.unwrap();
    assert_eq!(divergence.turns_since, 1, "the exact count, never 'some'");
    assert!(divergence.last_activity_ms.is_some());
    // The divergence waits for the user (the sync chip's "needs you"),
    // and blocks destructive actions on the Instance meanwhile.
    let pending = crossmachine::switch_back::fleet_divergences_impl(&pair.desktop);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].machine_label, "Laptop");
    assert!(crossmachine::switch_back::is_diverged(&pair.desktop, &away));
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !pair
            .laptop
            .accounts_dir()
            .join(CLAUDE_CODE_ID)
            .join(&pair.laptop_account)
            .join("last-args")
            .exists(),
        "nothing ran on the laptop"
    );
    {
        let storage = pair.desktop.storage.lock().unwrap();
        let edge = storage
            .switch_graph()
            .edge(&juggler_core::model::SwitchEdgeId::new(
                refused.edge_id.unwrap(),
            ))
            .unwrap();
        assert!(matches!(
            edge.state,
            juggler_core::model::EdgeState::Failed { .. }
        ));
    }

    // Keep both: a copy of the original resumes; the original stays.
    let both = switch_back(&pair, &away, "keep_both");
    assert!(both.accepted, "{both:?}");
    assert!(
        crossmachine::switch_back::fleet_divergences_impl(&pair.desktop).is_empty(),
        "resolved by the choice"
    );
    let args = last_args(&pair.laptop, &pair.laptop_account);
    assert!(
        args.contains("--resume") && !args.contains("native-5"),
        "a fork id: {args}"
    );
    assert_eq!(
        sha(&pair.source_transcript),
        original_after,
        "the original is untouched"
    );
    let forks: Vec<_> = std::fs::read_dir(pair.source_transcript.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".jsonl"))
        .collect();
    assert_eq!(forks.len(), 2, "the original plus its fork");
}

#[test]
fn keep_mine_resumes_the_diverged_original_and_a_running_original_is_refused() {
    let pair = setup_both("2.1.0", true, Permission::Control, Permission::Control);
    let away = switched_and_worked(&pair);
    append_turn(
        &pair.source_transcript,
        "u9",
        "user",
        "quick fix on the laptop",
        &pair.laptop_cwd,
    );
    let _ = back_plan(&pair, &away);
    // A plain switch-back records the divergence; Keep theirs lifts it
    // without sending anything.
    let refused = switch_back(&pair, &away, "");
    assert_eq!(refused.reason_code.as_deref(), Some("diverged"));
    crossmachine::switch_back::fleet_divergence_keep_theirs_impl(&pair.desktop, &away).unwrap();
    assert!(!crossmachine::switch_back::is_diverged(
        &pair.desktop,
        &away
    ));
    assert!(audit_kinds(&pair.desktop).contains(&"divergence_kept_theirs".to_owned()));
    // While the original runs on the laptop, nothing may resume it.
    {
        let storage = pair.laptop.storage.lock().unwrap();
        let mut row = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(pair.source.clone()))
            .unwrap();
        row.status = juggler_core::model::SessionStatus::Running;
        storage.sessions().update(&row).unwrap();
    }
    let busy = switch_back(&pair, &away, "keep_mine");
    assert_eq!(busy.reason_code.as_deref(), Some("busy"));
    {
        let storage = pair.laptop.storage.lock().unwrap();
        let mut row = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(pair.source.clone()))
            .unwrap();
        row.status = juggler_core::model::SessionStatus::HandedOver;
        storage.sessions().update(&row).unwrap();
    }
    clear_args(&pair.laptop, &pair.laptop_account);
    let mine = switch_back(&pair, &away, "keep_mine");
    assert!(mine.accepted, "{mine:?}");
    let args = last_args(&pair.laptop, &pair.laptop_account);
    assert!(
        args.contains("native-5") && args.contains("While you were away"),
        "{args}"
    );
}

// ------------------------------------------------ AJ-E710 guardrails

/// Every file under `dir` (recursively), skipping `skip` subtrees.
fn files_under(dir: &Path, skip: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        if skip.iter().any(|skipped| next.starts_with(skipped)) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[test]
fn no_credential_or_secret_reaches_the_other_mac_or_any_log() {
    // T2 is blocked by the version, so this exercises the T3 path, the
    // one that sends conversation TEXT across, with Strip chosen.
    let pair = setup_both("2.0.9", true, Permission::Control, Permission::Control);
    // A vendor credential on the laptop, next to the Account's sessions.
    let credentials = pair
        .laptop
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(&pair.laptop_account)
        .join(".credentials.json");
    std::fs::write(
        &credentials,
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-CREDENTIALSECRETCREDENTIALSECRET"}}"#,
    )
    .unwrap();
    let _ = target(&pair, "t2_blocked");
    let mut replay = request(&pair, "replay");
    replay.redaction_choice = Some("strip".to_owned());
    let outcome = execute(&pair, replay).0.unwrap();
    assert!(outcome.accepted, "{outcome:?}");
    // Let a few telemetry ticks and the catalog flow both ways.
    std::thread::sleep(Duration::from_secs(3));

    let credential: &[u8] = b"CREDENTIALSECRET";
    let transcript_secret: &[u8] = b"SECRETSECRETSECRET";
    // Nothing of the laptop's credential or its stripped secret exists
    // ANYWHERE on the desktop — its sessions, database, logs.
    for file in files_under(&pair.desktop.data_dir, &[]) {
        let bytes = std::fs::read(&file).unwrap_or_default();
        assert!(
            !contains(&bytes, credential),
            "credential egressed into {file:?}"
        );
        assert!(
            !contains(&bytes, transcript_secret),
            "secret egressed into {file:?}"
        );
    }
    // No log or database on either machine holds a secret, a credential
    // or the identity private key (account folders legitimately hold
    // the vendor's own files and are excluded).
    for state in [&pair.laptop, &pair.desktop] {
        let key =
            std::fs::read(state.data_dir.join("fleet").join("identity.key")).unwrap_or_default();
        let accounts = state.accounts_dir();
        let identity_dir = state.data_dir.join("fleet").join("identity.key");
        for file in files_under(&state.data_dir, &[accounts, identity_dir.clone()]) {
            if file == identity_dir {
                continue;
            }
            let bytes = std::fs::read(&file).unwrap_or_default();
            assert!(!contains(&bytes, credential), "credential in {file:?}");
            assert!(!contains(&bytes, transcript_secret), "secret in {file:?}");
            assert!(!contains(&bytes, &key), "identity key bytes in {file:?}");
        }
    }
}

#[test]
fn link_off_returns_both_macs_to_fully_local_behaviour() {
    // AJ-E710 Req.6, tested: after real fleet use, Link off leaves no
    // fleet residue blocking a LOCAL switch or a local Clean.
    let pair = setup_both("2.1.0", true, Permission::Control, Permission::Control);
    let _ = switched_and_worked(&pair);
    link::link_set_enabled_impl(&pair.laptop, false).unwrap();
    link::link_set_enabled_impl(&pair.desktop, false).unwrap();
    assert!(
        crossmachine::fleet_switch_targets_impl(&pair.laptop, &pair.source)
            .unwrap()
            .iter()
            .all(|row| row.readiness == "unavailable"),
        "no machine is offered as a destination while Link is off"
    );
    // A local switch on the laptop still runs: a second Account there.
    let other = commands::create_account(&pair.laptop, CLAUDE_CODE_ID, "Side", None).unwrap();
    let side = commands::create_profile(&pair.laptop, &other.id, "Main", None, None).unwrap();
    let local =
        commands::launch_profile(&pair.laptop, &side.id, Some(pair.laptop_cwd.clone())).unwrap();
    let targets = commands::switch_targets_impl(&pair.laptop, &local.session_id).unwrap();
    assert!(!targets.is_empty(), "local destinations still listed");
    // A local Clean is never refused for fleet reasons: nothing on this
    // Mac is marked diverged.
    assert!(!crossmachine::switch_back::is_diverged(
        &pair.laptop,
        &local.session_id
    ));
    assert!(crossmachine::switch_back::fleet_divergences_impl(&pair.laptop).is_empty());
}

#[test]
fn forged_free_form_launches_are_refused_by_the_node() {
    // ADR-0013: a Control peer can only name what already exists on the
    // node — never a binary, a path or a working directory.
    let pair = setup_both("2.1.0", true, Permission::Control, Permission::Control);
    let _ = target(&pair, "t2_ready");
    let desktop_id = fleet::fleet_self_impl(&pair.desktop).unwrap().machine_id;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let forged = [
        juggler_link::proto::LaunchRecipe {
            recipe_id: "/bin/sh".into(),
            project_id: String::new(),
        },
        juggler_link::proto::LaunchRecipe {
            recipe_id: pair.desktop_recipe.clone(),
            project_id: "/tmp".into(),
        },
        juggler_link::proto::LaunchRecipe {
            recipe_id: "rec-; rm -rf ~".into(),
            project_id: "../../etc".into(),
        },
    ];
    for launch in forged {
        let outcome = wait_for("the node's verdict", || {
            let outcome = runtime
                .block_on(juggler_app::remote::send_action_for_test(
                    Arc::clone(&pair.laptop),
                    desktop_id.clone(),
                    juggler_link::proto::control_command::Action::LaunchRecipe(launch.clone()),
                ))
                .unwrap();
            (outcome.reason_code.as_deref() != Some("clock_unmeasured")).then_some(outcome)
        });
        assert!(!outcome.accepted, "{launch:?} → {outcome:?}");
        assert_eq!(
            outcome.reason_code.as_deref(),
            Some("unknown_record"),
            "{launch:?}"
        );
    }
    // Nothing ran on the desktop.
    assert!(
        !pair
            .desktop
            .accounts_dir()
            .join(CLAUDE_CODE_ID)
            .join(&pair.desktop_account)
            .join("last-args")
            .exists()
    );
}
