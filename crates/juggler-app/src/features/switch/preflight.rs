//! AJ-E205: the preflight verdict for one switch, and the door counts.

use std::sync::Arc;

use juggler_core::model::{Account, Timestamp};
use juggler_core::storage::Storage;

use crate::commands::{find_profile_with_account_arc, lock_storage_arc};
use crate::error::{AppError, AppResult};
use crate::features::handover::source::handover_source;
use crate::features::handover::stays_behind;
use crate::features::providers::installed_versions_for;
use crate::features::quota::quota_display_inner;
use crate::features::redaction::dto::{RedactionItemInfo, RedactionPolicyInfo};
use crate::features::redaction::{redaction_global_of, redaction_items_of, redaction_override_of};
use crate::features::switch::dto::{CarryListInfo, PreflightInfo};
use crate::features::switch::edges::source_row;
use crate::features::switch::execute::{switch_destinations, user_warnings};
use crate::features::validation::{dogfood_cells_of, dogfood_key};
use crate::state::AppState;

/// AJ-E206 §8 telemetry: local-only door counters (never networked) —
/// `switch_oneclick` vs `switch_wizard{steps_shown}` proves the fast
/// path is earned, not assumed.
const SWITCH_DOOR_COUNTERS_KEY: &str = "switch_door_counters";

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on resolution failures or hard
/// preflight stops.
pub fn switch_preflight_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<PreflightInfo> {
    preflight_with(state, session_id, target_profile_id)
}

/// [`switch_preflight_impl`] (AJ-E1513: staging checks the switch too).
pub(crate) fn preflight_with(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<PreflightInfo> {
    use juggler_handover::pipeline;

    let (_profile_b, account_b) = find_profile_with_account_arc(state, target_profile_id)?;
    let (source, account_a) = handover_source(state, session_id)?;
    let reference =
        crate::features::switch::execute::switch_reference(state, &source, &account_a, &account_b)?;
    // (No compaction scan: a compacted source loses nothing in a switch —
    // D-03 amendment, 2026-10-27.)

    // Target quota gate + the usual-account test (one storage pass).
    let now = Timestamp::now().millis();
    let storage = lock_storage_arc(state)?;
    let (exhausted, tight) = quota_gate_of(&storage, &account_b, now);
    // AJ-E206 §8: the "usual account" test — the project's dominant
    // account by session count (a tie is not a habit).
    let usual_account = source
        .project
        .as_ref()
        .is_some_and(|project| usual_account_for_project(&storage, project, &account_b));
    drop(storage);

    // One disposition pass feeds both the fidelity engine and the
    // door's carry-list (same names, same truth).
    let mut dispositions = juggler_handover::fidelity::base_dispositions(
        account_a.harness.as_str(),
        account_b.harness.as_str(),
    );
    let carry = carry_lists_of(&dispositions);
    // AJ-E808: the door shows the same validation verdict as the rows, and
    // the fidelity engine caps an unproven pair from the same cell (QA P23:
    // it used to treat every pair as unproven).
    // AJ-E1601: a profile through another service is its own side, never
    // a proven switch.
    let validated = pair_validated_now(
        state,
        &crate::features::service_accounts::switch_side(state, &account_a)?,
        &crate::features::service_accounts::switch_side(state, &account_b)?,
    )?;
    dispositions.flags.validated = validated;

    let input = pipeline::PreflightInput {
        fidelity: dispositions.clone(),
        target_account: account_b.label.clone(),
        gates: pipeline::PreflightGates {
            target_exhausted_no_reset: exhausted,
            version_drift: false, // per-version pinning rides E210's matrix
        },
        target_quota_tight: tight,
        git: pipeline::GitState::default(),
        hop_count: 0,
        replay_tokens_spent: 0,
    };
    let report = pipeline::preflight(&input)
        .map_err(|failure| AppError::switch(failure, &account_a.label, &account_b.label))?;
    let warnings = user_warnings(&report);
    let source_live = matches!(source.status.as_str(), "running" | "starting");
    let stays = source_stays(state, &source, &reference, &account_a, source_live);

    // AJ-E206 §8: choose the door from the same preflight facts.
    // Headroom ≥ Tight holds unless the tight warn fired (exhausted
    // already stopped above at the hard gate); conflicts arrive with
    // E207's reconciliation and rules divergence has no source yet.
    let door_inputs = juggler_handover::door::DoorInputs {
        mechanism: Some(report.mechanism),
        fidelity: Some(report.pair.label),
        conflicts: false,
        drops_listed: !carry.dropped.is_empty(),
        target_headroom_ok: tight.is_none(),
        rules_diverged: false,
    };
    let (door, wizard_steps, wizard_reasons) = door_verdict(&door_inputs, &carry, validated);
    let (redaction, (redaction_items, messages)) = redaction_for(state, &account_b, &source)?;
    let tool_b = crate::features::tools::display_name(&state.registry, account_b.harness.as_str());
    let words = crate::features::handover::switch_words::preflight_words(
        state,
        &account_b,
        report.mechanism,
        &input.fidelity,
        messages,
    );
    let named = stays_behind::of(state, &reference, &account_a, &account_b);
    Ok(PreflightInfo {
        mechanism: report.mechanism.as_wire().to_owned(),
        fidelity: report.pair.label.as_str().to_owned(),
        // What the move does; "not proven yet" is said beside it (P23).
        fidelity_words: report.pair.described.user_label().to_owned(),
        score: report.pair.score,
        warnings,
        zombies: report.zombie_children.clone(),
        diff_snapshotted: report.diff_snapshotted,
        source_live,
        moving: named.moving(&account_a),
        source_keeps_running: stays.keeps_running,
        source_outside: stays.outside,
        source_app: stays.app,
        experimental_copy: move_copy(
            report.mechanism,
            validated,
            &account_a.label,
            &account_b.label,
            &tool_b,
        ),
        door,
        wizard_steps,
        wizard_reasons,
        usual_account,
        carry,
        redaction,
        redaction_items,
        validated,
        carry_items: [words.carry_items, named.stays].concat(),
        move_line: words.move_line,
        secrets: words.secrets,
        target_tool: account_b.harness.to_string(),
        target_account: account_b.id.to_string(),
        destinations: switch_destinations(state, &account_a, &account_b, &reference),
    })
}

/// The target's keys policy, and the source conversation's detected
/// secrets (masked) with its message count.
fn redaction_for(
    state: &Arc<AppState>,
    account_b: &Account,
    source: &juggler_core::model::InstanceRecord,
) -> AppResult<(RedactionPolicyInfo, (Vec<RedactionItemInfo>, usize))> {
    let storage = lock_storage_arc(state)?;
    let policy = RedactionPolicyInfo::from_parts(
        &crate::features::tools::display_name(&state.registry, account_b.harness.as_str()),
        redaction_global_of(&storage),
        redaction_override_of(&storage, account_b.harness.as_str()),
    );
    drop(storage);
    Ok((policy, redaction_items_of(source)))
}

/// What the switch leaves of the source, for the switch screen.
struct SourceStays {
    /// It keeps running: the same rule `freeze_source` follows (only a
    /// session AI Juggler supervises is stopped).
    keeps_running: bool,
    /// AJ-E2108: it is a session another app runs.
    outside: bool,
    /// The app that one is open in now, when it is.
    app: Option<String>,
}

/// [`SourceStays`] for `source` (`live` when it runs now).
fn source_stays(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    reference: &juggler_core::model::TranscriptRef,
    account: &Account,
    live: bool,
) -> SourceStays {
    let keeps_running = live && !state.supervisor.is_supervised(&source.id);
    let outside = source_row(source).is_none();
    let app = outside
        .then(|| {
            reference.native_id.as_deref().and_then(|native| {
                crate::features::conversations::switch_source::open_in(
                    state,
                    account.id.as_str(),
                    account.harness.as_str(),
                    native,
                )
            })
        })
        .flatten();
    SourceStays {
        keeps_running,
        outside,
        app,
    }
}

/// AJ-E808: whether `from → to` is validated right now — the recorded
/// live cell, the fidelity register, and the installed versions (strict).
fn pair_validated_now(state: &Arc<AppState>, from: &str, to: &str) -> AppResult<bool> {
    let versions = installed_versions_for(state, from, to);
    let installed = |provider: &str| versions.get(provider).cloned();
    let cell = {
        let storage = lock_storage_arc(state)?;
        dogfood_cells_of(&storage)
            .get(&dogfood_key(from, to))
            .cloned()
    };
    Ok(juggler_handover::validation::pair_cell(from, to)
        .map(|row| match cell {
            Some(record) => juggler_handover::validation::with_dogfood(row, record),
            None => row,
        })
        .is_some_and(|row| row.validated_for(&installed, true)))
}

/// Target quota gate from the E606/E610 engine (hard filter =
/// exhausted with no reset; tight = the warn copy).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "display-copy rounding, clamped to [0, u32::MAX) before the cast"
)]
fn quota_gate_of(storage: &Storage, account: &Account, now: i64) -> (bool, Option<(u32, u32)>) {
    let target_display = quota_display_inner(storage, account, now);
    match target_display.five_hour.as_ref().map(|w| w.state.as_str()) {
        Some("exhausted") => (target_display.using_credits().is_none(), None), // 0.3.2: or credits
        Some(_) => {
            let used = target_display
                .five_hour
                .as_ref()
                .and_then(crate::features::quota::WindowDisplay::best_estimate)
                .unwrap_or(0.0);
            let tight = target_display
                .forecast
                .tte_min
                .filter(|tte| *tte < 45.0)
                .map(|tte| {
                    (
                        (100.0 - used).round().clamp(0.0, 99.0) as u32,
                        tte.round().clamp(0.0, 999.0) as u32,
                    )
                });
            (false, tight)
        }
        None => (false, None),
    }
}

/// AJ-E206 §8: chooses the door from the preflight facts. Returns the
/// wire name, the wizard step titles (empty for 1-click — never phantom
/// steps), and the wizard's opening lines. Counting the door is the
/// switch screen's ([`switch_door_count`]): the preflight only reads.
fn door_verdict(
    inputs: &juggler_handover::door::DoorInputs,
    carry: &CarryListInfo,
    validated: bool,
) -> (String, Vec<String>, Vec<String>) {
    let door = juggler_handover::door::choose_door(inputs);
    match &door {
        juggler_handover::door::Door::OneClick => ("one-click".to_owned(), Vec::new(), Vec::new()),
        juggler_handover::door::Door::Wizard { steps } => (
            "wizard".to_owned(),
            steps.iter().map(|step| step.title().to_owned()).collect(),
            wizard_reasons_of(inputs, carry, validated),
        ),
    }
}

/// QA P23: the Beta line under the switch, for what this move does: a
/// transplant moves the whole session; a replay starts fresh from the
/// story; a pack is read by the new session.
fn move_copy(
    mechanism: juggler_handover::pipeline::Mechanism,
    validated: bool,
    account_a: &str,
    account_b: &str,
    tool_b: &str,
) -> String {
    use juggler_handover::pipeline::{CopySituation, Mechanism, user_copy};
    let unproven = |words: String| {
        if validated {
            words
        } else {
            format!("Beta, not proven yet: {words}")
        }
    };
    match mechanism {
        Mechanism::NativeResume | Mechanism::Transplant if validated => {
            user_copy(CopySituation::TransplantProven {
                account_b: account_b.to_owned(),
                account_a: account_a.to_owned(),
            })
        }
        Mechanism::NativeResume | Mechanism::Transplant => {
            user_copy(CopySituation::TransplantUnproven {
                account_a: account_a.to_owned(),
            })
        }
        Mechanism::Replay => unproven(format!(
            "a fresh start on {tool_b}, which reads the conversation, what each tool did, the task list and the project's changes. Tool state and subagents don't carry. {account_a} stays resumable."
        )),
        Mechanism::PackOnly => unproven(format!(
            "{tool_b} starts fresh from notes about the work: what it is, what was decided and what's left. {account_a} stays resumable."
        )),
    }
}

/// AJ-E206 §8: whether `target` is the project's dominant account —
/// strictly the unique maximum by occurrence count (live session
/// records by account, plus one occurrence per imported-history
/// label). No history ⇒ nobody is usual ⇒ the wizard opens.
fn usual_account_for_project(
    storage: &Storage,
    project: &juggler_core::model::ProjectId,
    target: &Account,
) -> bool {
    let counts = project_account_counts(storage, project);
    let Some(target_count) = counts.get(&target.id).copied().filter(|count| *count > 0) else {
        return false;
    };
    let others_max = counts
        .iter()
        .filter(|(id, _)| *id != &target.id)
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(0);
    target_count > others_max
}

/// How often each account ran in `project`: its session records, plus one
/// per imported-history label. The one count behind "used most here" in
/// the rankings and the usual-account test.
pub(crate) fn project_account_counts(
    storage: &Storage,
    project: &juggler_core::model::ProjectId,
) -> std::collections::BTreeMap<juggler_core::model::AccountId, u64> {
    let mut counts: std::collections::BTreeMap<juggler_core::model::AccountId, u64> =
        std::collections::BTreeMap::new();
    let sessions = storage.sessions().list(None).unwrap_or_default();
    for record in sessions
        .iter()
        .filter(|row| row.project.as_ref() == Some(project))
    {
        if let Ok(recipe) = storage.profiles().get(&record.profile) {
            *counts.entry(recipe.account).or_default() += 1;
        }
    }
    // Imported history speaks in labels, not ids (ADR-0008 E1 rename
    // kept `ProjectHistory.accounts` as labels) — resolve via accounts.
    if let Ok(record) = storage.projects().get(project) {
        let accounts = storage.accounts().list().unwrap_or_default();
        for label in &record.history.accounts {
            if let Some(account) = accounts.iter().find(|row| &row.label == label) {
                *counts.entry(account.id.clone()).or_default() += 1;
            }
        }
    }
    counts
}

/// Groups the pair's dispositions into the door's carry-list names.
pub(crate) fn carry_lists_of(input: &juggler_handover::fidelity::FidelityInput) -> CarryListInfo {
    use juggler_handover::fidelity::{Disposition, FidelityInput};
    let FidelityInput { dispositions, .. } = input;
    let mut carry = CarryListInfo::default();
    for (kind, disposition) in dispositions {
        match disposition {
            Disposition::Copy => carry.carried.push(kind.label().to_owned()),
            Disposition::Regenerate => carry.regenerated.push(kind.label().to_owned()),
            // A translate without a written mapping (or with dropped
            // keys) is honestly a Drop (§H2) — it carries nothing the
            // user can rely on, so it lists on the drop side.
            Disposition::Translate {
                untranslated,
                keys_dropped,
            } => {
                if *untranslated || *keys_dropped {
                    carry.dropped.push(kind.label().to_owned());
                } else {
                    carry.carried.push(kind.label().to_owned());
                }
            }
            Disposition::Drop => carry.dropped.push(kind.label().to_owned()),
        }
    }
    carry
}

/// AJ-E206 §5: the wizard's opening lines — one human sentence per
/// failed §8 condition (never a generic "some context may be lost").
fn wizard_reasons_of(
    inputs: &juggler_handover::door::DoorInputs,
    carry: &CarryListInfo,
    validated: bool,
) -> Vec<String> {
    use juggler_handover::pipeline::Mechanism;
    // D-03 amendment (2026-10-27): only real, actionable risks — another
    // account is the point of a switch, and a compacted source loses
    // nothing, so neither is a reason.
    let mut reasons = Vec::new();
    match inputs.mechanism {
        Some(Mechanism::Replay) => {
            reasons.push(
                "The other tool can't pick up this conversation where it stopped: it starts fresh and reads it first.".to_owned(),
            );
        }
        Some(Mechanism::PackOnly) => {
            reasons.push("The other tool can't continue this conversation: it starts from notes about the work.".to_owned());
        }
        _ => {}
    }
    // QA P23: no F-number. A lower label always has its own, plainer
    // reason here (it replays, or leaves something behind);
    // "not proven" is said as itself.
    if !validated {
        reasons.push("This switch hasn't been proven on a real setup yet.".to_owned());
    }
    if !carry.dropped.is_empty() {
        reasons.push(format!("Doesn't carry: {}.", carry.dropped.join(", ")));
    }
    if !inputs.target_headroom_ok {
        reasons.push("At this pace the other account runs out of room soon.".to_owned());
    }
    reasons
}

/// [`switch_door_count`](crate::features::switch::commands::switch_door_count)'s work, for tests.
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown door or step.
pub fn switch_door_count_impl(
    state: &Arc<AppState>,
    door: &str,
    steps: &[String],
) -> AppResult<()> {
    let door =
        juggler_handover::door::Door::from_wire(door, steps).ok_or(AppError::Validation {
            subject: "door",
            message: "That isn't a switch screen.".to_owned(),
        })?;
    record_door_counter(state, &door);
    Ok(())
}

/// Counts `door` in the local KV: a failed read or write never blocks
/// the switch.
fn record_door_counter(state: &Arc<AppState>, door: &juggler_handover::door::Door) {
    let Ok(storage) = lock_storage_arc(state) else {
        return;
    };
    let settings = storage.app_settings();
    let mut counters: std::collections::BTreeMap<String, u64> = settings
        .get(SWITCH_DOOR_COUNTERS_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    *counters.entry(door.telemetry_kind()).or_insert(0) += 1;
    if let Ok(blob) = serde_json::to_string(&counters) {
        let _ = settings.set(SWITCH_DOOR_COUNTERS_KEY, &blob);
    }
}
