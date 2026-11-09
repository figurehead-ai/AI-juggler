//! AJ-E206: executing a switch: the continuation session, freezing the
//! source, and where it lands.

use std::sync::Arc;

use juggler_core::model::{Account, SessionId, Timestamp};

use crate::commands::find_profile_with_account_arc;
use crate::error::{AppError, AppResult};
use crate::features::account::login_state_of;
use crate::features::handover::source::handover_source_linked;
use crate::features::switch::dto::SwitchExecutedInfo;
use crate::features::switch::edges::{
    EdgeCtx, make_launch_hook, record_edge_failed, record_edge_staged, record_edge_succeeded,
};
use crate::features::switch::replay::switch_replay;
use crate::features::switch::undo::record_redaction_decision;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// AJ-E1523: a switch away from a session that had reached its usage
/// limit is counted as the limit's (`quota_wall`), however the switch
/// screen was opened. A successful one is a juggling loop: the limit hit,
/// and the work went on elsewhere. `source` is the session as it was
/// before the switch stopped it.
pub(crate) fn loop_trigger(
    source: &juggler_core::model::InstanceRecord,
    trigger: &'static str,
) -> &'static str {
    if source.status == juggler_core::model::SessionStatus::QuotaBlocked {
        "quota_wall"
    } else {
        trigger
    }
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on any failure.
pub fn switch_execute_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    freeze_choice: &str,
    redaction_choice: Option<&str>,
    redaction_unredact: &[u32],
) -> AppResult<SwitchExecutedInfo> {
    switch_execute_with(
        state,
        session_id,
        target_profile_id,
        freeze_choice,
        redaction_choice,
        redaction_unredact,
        &[],
    )
}

/// 0.3.5: [`switch_execute_impl`] continuing in `destination` (`desktop`
/// or `cli`), as the confirm step's choice sends it (walks and tests).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on any failure.
pub fn switch_execute_to(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    freeze_choice: &str,
    destination: &str,
) -> AppResult<SwitchExecutedInfo> {
    switch_execute_core(
        state,
        session_id,
        target_profile_id,
        freeze_choice,
        None,
        &[],
        "manual",
        &[],
        Some(destination),
        None,
    )
}

/// D-76: [`switch_execute_impl`] with the start of the conversation left
/// out as `cut` says (the lab's and tests' way in).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on any failure.
pub fn switch_execute_cut(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    freeze_choice: &str,
    cut: juggler_adapter::handover::Cut,
) -> AppResult<SwitchExecutedInfo> {
    switch_execute_core(
        state,
        session_id,
        target_profile_id,
        freeze_choice,
        None,
        &[],
        "manual",
        &[],
        None,
        Some(cut),
    )
}

/// [`switch_execute_impl`] with the offers the user ticked that the switch
/// itself carries out (see [`switch_execute`](crate::features::switch::commands::switch_execute)).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on any failure.
pub fn switch_execute_with(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    freeze_choice: &str,
    redaction_choice: Option<&str>,
    redaction_unredact: &[u32],
    approved: &[String],
) -> AppResult<SwitchExecutedInfo> {
    switch_execute_core(
        state,
        session_id,
        target_profile_id,
        freeze_choice,
        redaction_choice,
        redaction_unredact,
        "manual",
        approved,
        None,
        None,
    )
}

/// [`switch_execute_impl`] with the switch's trigger (AJ-E1103).
/// A session's switch in progress: held for the whole switch, released
/// however it ends.
pub(crate) struct SwitchSlot<'a> {
    state: &'a Arc<AppState>,
    session: String,
}

impl<'a> SwitchSlot<'a> {
    /// Takes `session`'s slot, refusing while another switch of it runs.
    pub(crate) fn take(state: &'a Arc<AppState>, session: &str) -> AppResult<Self> {
        let mut running = state
            .switching
            .lock()
            .map_err(|_| AppError::Refused("switching is unavailable right now".to_owned()))?;
        if !running.insert(session.to_owned()) {
            return Err(AppError::Refused(
                "This session is already being switched: wait for that to finish".to_owned(),
            ));
        }
        Ok(Self {
            state,
            session: session.to_owned(),
        })
    }
}

impl Drop for SwitchSlot<'_> {
    fn drop(&mut self) {
        if let Ok(mut running) = self.state.switching.lock() {
            running.remove(&self.session);
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the switch screen's answers, each its own"
)]
#[allow(
    clippy::too_many_lines,
    reason = "one switch, stage by stage: freeze, record, stage, launch, record"
)]
pub(crate) fn switch_execute_core(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    freeze_choice: &str,
    redaction_choice: Option<&str>,
    redaction_unredact: &[u32],
    trigger: &'static str,
    approved: &[String],
    destination: Option<&str>,
    cut: Option<juggler_adapter::handover::Cut>,
) -> AppResult<SwitchExecutedInfo> {
    use juggler_handover::pipeline::{self, Mechanism};
    let _slot = SwitchSlot::take(state, session_id)?;

    let freeze = freeze_of(freeze_choice);
    let sides = switch_sides(state, session_id, target_profile_id)?;
    // 0.3.5: Desktop continues in the target account's app profile; CLI
    // (the default) in a terminal.
    let desktop = desktop_place(state, &sides, destination)?;
    // AJ-E1304 (D-11): across Providers the conversation REPLAYS into a
    // primed session — one vendor's file is never installed into the
    // other's home.
    if sides.account_a.harness != sides.account_b.harness {
        return switch_replay(
            state,
            &sides,
            freeze,
            redaction_choice,
            redaction_unredact,
            trigger,
            approved,
        );
    }
    let SwitchSides {
        source,
        reference,
        account_a,
        profile_b,
        account_b,
    } = sides;
    let support = handover_support(state, &account_b)?;

    // AJ-E1001: the target Account must be who it was, before anything
    // moves.
    let assertion = crate::identity::assert_identity(state, &account_b)?;
    // AJ-E1540: the source stops (after its turn, or now) before its file
    // is copied, so the conversation never has two writers.
    freeze_source(state, &source, &reference, freeze)?;

    // Record (E201): the Staged edge exists BEFORE anything moves;
    // failures transition it, history is never rewritten.
    let (work_id, from_epoch, edge_id) = record_edge_staged(
        state,
        &source,
        &reference,
        &account_a,
        &account_b,
        juggler_core::model::SwitchMechanism::Transplant,
    )?;
    let switch = crate::usage_stats::Switch::started(
        account_a.harness.as_str(),
        account_b.harness.as_str(),
        "transplant",
        loop_trigger(&source, trigger),
    );

    // The pipeline: snapshot → stage → verify → launch (the app's
    // supervisor hook) → the copy-never-move invariant.
    let adapter = state
        .registry
        .get(&account_b.harness)
        .ok_or_else(|| AppError::UnknownHarness(account_b.harness.to_string()))?;
    let new_session = SessionId::new(format!("sess-{}", unique_suffix()));
    let mut launch_hook = make_launch_hook(
        state,
        adapter,
        &account_b,
        &profile_b,
        &source,
        &new_session,
        desktop.as_deref(),
    );
    // ADR-0028: the sessions started from it move with it.
    let family = crate::features::handover::family::find(state, &account_a, &reference);
    // D-76: the start left out, when the user asked (by default all of it
    // travels).
    let outcome = pipeline::execute_family(
        support.as_ref(),
        &reference,
        &family.refs,
        &account_b,
        freeze,
        Mechanism::Transplant,
        cut,
        &mut launch_hook,
    );
    let (record, pid) = match outcome {
        Ok(value) => value,
        Err(failure) => {
            switch.failed(failure.stage());
            let error = AppError::switch(failure, &account_a.label, &account_b.label);
            record_edge_failed(state, &edge_id, &error);
            return Err(error);
        }
    };
    // In Desktop the session is the app's: kept by its account and id, as
    // every session another app runs is.
    let conversation_key = desktop.as_ref().map(|_| landed_key(&account_b, &record));
    let new_session = conversation_key
        .as_ref()
        .map_or_else(|| new_session.clone(), |key| SessionId::new(key.clone()));
    record_edge_succeeded(
        state,
        &source,
        &record,
        &account_b,
        &EdgeCtx {
            work: work_id,
            from_epoch,
            edge: edge_id.clone(),
            session: new_session.clone(),
        },
    )?;

    // 0.4.1 (the rc5 walk): a Desktop copy stays in the original's pools,
    // watched, as the confirm's pool lines said.
    if let Some((folder, id)) = conversation_key
        .as_deref()
        .and_then(crate::features::conversations::switch_source::parse_source_key)
        && let Err(error) =
            crate::features::pools::switching::keep_watched(state, source.id.as_str(), folder, id)
    {
        crate::error_log::record("switch_pools", &error.to_string());
    }
    crate::identity::record(state, &new_session, &account_b, &assertion, "switch");
    // The app's own record already proved a Desktop hand-off arrived.
    if desktop.is_none() {
        crate::features::handover::arrival::watch_switch(
            state,
            &account_b,
            &new_session,
            pid,
            &record,
        );
    }
    switch.succeeded();
    tasks_after_switch(state, &new_session, &account_a, &account_b);
    let (family_nests, family_place) = family_after(
        state,
        &account_b,
        &reference,
        &record,
        &family,
        desktop.as_deref(),
    );
    // ADR-0028: what wakes it by itself moves to the copy and is turned
    // off on the original (never two copies at work).
    let automation = crate::features::handover::automation::hand_over(
        state,
        &account_a,
        &account_b,
        &reference,
        &family.refs,
        &renamed_of(&reference, &record, &family),
        desktop.as_deref(),
        edge_id.as_str(),
    );

    // AJ-E208: the switch's redaction decision rides the edge.
    record_redaction_decision(
        state,
        &edge_id,
        &account_b.harness,
        redaction_choice,
        redaction_unredact,
    );

    Ok(SwitchExecutedInfo {
        session_id: new_session.to_string(),
        pid,
        edge_id: edge_id.to_string(),
        source_session_id: source.id.to_string(),
        continued_in: if desktop.is_some() { "desktop" } else { "cli" }.to_owned(),
        conversation_key,
        family_moved: u32::try_from(record.family.len()).unwrap_or(u32::MAX),
        family_nests,
        family_place,
        automation_moved: automation.moved,
        automation_copy_place: automation.copy_waits,
        automation_original_waits: automation.original_waits,
        automation_kept: automation.kept,
    })
}

/// ADR-0028: the conversation ids the switch changed (a fork on a
/// collision): original → copy, for the conversation and its family.
fn renamed_of(
    reference: &juggler_core::model::TranscriptRef,
    record: &juggler_handover::pipeline::SwitchRecord,
    family: &crate::features::handover::family::Family,
) -> Vec<(String, String)> {
    let installed = |install: &juggler_adapter::handover::InstallOutcome| {
        install.reference.native_id.clone().unwrap_or_default()
    };
    std::iter::once((reference, &record.install))
        .chain(family.refs.iter().zip(&record.family))
        .filter_map(|(original, install)| {
            let old = original.native_id.clone()?;
            let new = installed(install);
            (!new.is_empty() && new != old).then_some((old, new))
        })
        .collect()
}

/// ADR-0028: after a switch into a desktop app profile, its family's
/// records go there beside the conversation's: at once when the app isn't
/// running, else at its next start. Returns where they nest (`now`,
/// `restart`, or empty) and the profile.
fn family_after(
    state: &Arc<AppState>,
    account_b: &Account,
    reference: &juggler_core::model::TranscriptRef,
    record: &juggler_handover::pipeline::SwitchRecord,
    family: &crate::features::handover::family::Family,
    place: Option<&str>,
) -> (String, Option<String>) {
    use crate::features::handover::family;
    let (Some(place), false) = (place, record.family.is_empty()) else {
        return (String::new(), None);
    };
    let renamed = renamed_of(reference, record, family);
    let installed = |install: &juggler_adapter::handover::InstallOutcome| {
        install.reference.native_id.clone().unwrap_or_default()
    };
    let pending = family::Pending {
        place: place.to_owned(),
        tool: account_b.harness.to_string(),
        parent: installed(&record.install),
        members: family.members.iter().map(Into::into).collect(),
        renamed,
    };
    if let Err(error) = family::wait(state, &pending) {
        tracing::warn!(target: "aij::flow", flow = "family", step = "wait", %error);
        return (String::new(), None);
    }
    if crate::features::places::running_now(state, place).is_none()
        && let Some(dir) = crate::features::places::instance_dir_of(state, place)
    {
        family::place_waiting(state, place, &dir);
        return ("now".to_owned(), Some(place.to_owned()));
    }
    ("restart".to_owned(), Some(place.to_owned()))
}

/// The preflight's warnings in the screen's words (§7.1).
pub(crate) fn user_warnings(report: &juggler_handover::pipeline::PreflightReport) -> Vec<String> {
    report
        .warnings
        .iter()
        .map(|situation| juggler_handover::pipeline::user_copy(situation.clone()))
        .collect()
}

/// 0.3.5: the preflight's two choices. A session the desktop app started
/// (its tool says so) stays in the app by default; across tools only CLI.
pub(crate) fn switch_destinations(
    state: &Arc<AppState>,
    account_a: &Account,
    account_b: &Account,
    reference: &juggler_core::model::TranscriptRef,
) -> crate::features::places::handoff::DestinationsInfo {
    use crate::features::places::handoff::{DestinationsInfo, destinations};
    if account_a.harness != account_b.harness {
        return DestinationsInfo {
            suggested: "cli".to_owned(),
            desktop: None,
            desktop_why_not: Some(
                "Desktop takes a switch between two accounts of the same tool for now".to_owned(),
            ),
            cli_why_not: None,
        };
    }
    let ran_in_desktop = reference.native_id.as_deref().is_some_and(|native| {
        state
            .conversations
            .conversation_of(account_a.id.as_str(), native)
            .is_some_and(|conversation| conversation.summary.started_in.is_some())
    });
    destinations(
        state,
        account_b.id.as_str(),
        &account_b.label,
        ran_in_desktop,
        terminal_signed_in(state, account_b),
    )
}

/// 0.4: whether `account`'s tool in a terminal is signed in (an unknown
/// answer isn't held against it).
pub(crate) fn terminal_signed_in(state: &Arc<AppState>, account: &Account) -> bool {
    login_state_of(&state.registry, account) != juggler_adapters::LoginState::LoggedOut
}

/// 0.3.5: the key of the conversation a Desktop switch handed over (the
/// app's session, kept by its account and id).
fn landed_key(account: &Account, record: &juggler_handover::pipeline::SwitchRecord) -> String {
    crate::features::conversations::switch_source::source_key(
        account.id.as_str(),
        record
            .install
            .reference
            .native_id
            .as_deref()
            .unwrap_or_default(),
    )
}

/// 0.3.5: the profile a switch continues in when the user picked Desktop
/// (`None` for CLI); refused in words when there's none to pick, or the
/// switch crosses tools (a replay starts a new session in a terminal).
fn desktop_place(
    state: &Arc<AppState>,
    sides: &SwitchSides,
    destination: Option<&str>,
) -> AppResult<Option<String>> {
    if destination == Some("cli")
        && sides.account_a.harness == sides.account_b.harness
        && !terminal_signed_in(state, &sides.account_b)
        && crate::features::places::handoff::signs_in_through_app(
            state,
            sides.account_b.id.as_str(),
        )
    {
        // 0.4: never a terminal that opens on a sign-in for an app that's
        // signed in through its desktop profile.
        let offered = crate::features::places::handoff::destinations(
            state,
            sides.account_b.id.as_str(),
            &sides.account_b.label,
            true,
            false,
        );
        return Err(AppError::Refused(offered.cli_why_not.unwrap_or_else(
            || "It signs in through its desktop app: continue there".to_owned(),
        )));
    }
    if destination != Some("desktop") {
        return Ok(None);
    }
    if sides.account_a.harness != sides.account_b.harness {
        return Err(AppError::Refused(
            "Desktop takes a switch between two accounts of the same tool for now: pick CLI"
                .to_owned(),
        ));
    }
    let offered = crate::features::places::handoff::destinations(
        state,
        sides.account_b.id.as_str(),
        &sides.account_b.label,
        true,
        true,
    );
    match offered.desktop {
        Some(desktop) => Ok(Some(desktop.place_id)),
        None => Err(AppError::Refused(offered.desktop_why_not.unwrap_or_else(
            || "There's no desktop app profile to continue in".to_owned(),
        ))),
    }
}

/// AJ-E1904: a switch from `from` to `to` that landed in `session` is a
/// line in the history of each task it carried.
pub(crate) fn tasks_after_switch(
    state: &Arc<AppState>,
    session: &SessionId,
    from: &Account,
    to: &Account,
) {
    crate::features::tasks::switched::after_switch(state, session, &from.id, &to.id);
}

/// How long the source's conversation and terminal must be quiet for its
/// turn to count as finished (AJ-E1540).
const TURN_QUIET: std::time::Duration = std::time::Duration::from_secs(3);

/// How long "Finish this turn" waits before saying the session is still
/// working (AJ-E1540).
const TURN_LIMIT: std::time::Duration = std::time::Duration::from_secs(120);

/// AJ-E1540: the freeze choice made real on a local switch. A session AI
/// Juggler started and that is still running stops before its file is
/// read: at once for "Stop now", or once its turn has finished (its
/// conversation file and its terminal quiet for [`TURN_QUIET`], the file's
/// last line whole) for "Finish this turn". A session started outside AI
/// Juggler isn't ours to stop: it keeps running, and the copy continues
/// from what it has written so far (the switch screen says so).
///
/// # Errors
///
/// [`AppError::Refused`] when the turn is still going after
/// [`TURN_LIMIT`], or the session won't stop.
pub(crate) fn freeze_source(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    reference: &juggler_core::model::TranscriptRef,
    freeze: juggler_handover::pipeline::FreezeChoice,
) -> AppResult<()> {
    use std::time::{Duration, Instant, SystemTime};
    let id = &source.id;
    if !state.supervisor.is_supervised(id) || state.supervisor.is_finished(id) {
        return Ok(());
    }
    // AJ-E2305: a driven session's turn boundary is its tool's own word.
    let finish = matches!(
        freeze,
        juggler_handover::pipeline::FreezeChoice::FinishTurnThenSwitch
    );
    if crate::features::runtime::release_for_switch(state, id, finish, TURN_LIMIT)? {
        return Ok(());
    }
    if matches!(
        freeze,
        juggler_handover::pipeline::FreezeChoice::FinishTurnThenSwitch
    ) {
        let started = Instant::now();
        loop {
            let file_quiet = std::fs::metadata(&reference.path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|at| SystemTime::now().duration_since(at).ok())
                .is_none_or(|idle| idle >= TURN_QUIET);
            let output_quiet = state.supervisor.last_output_ms(id).is_none_or(|at| {
                Timestamp::now().millis().saturating_sub(at)
                    >= i64::try_from(TURN_QUIET.as_millis()).unwrap_or(i64::MAX)
            });
            let whole = juggler_adapters::last_line_parses(&reference.path);
            if (file_quiet && output_quiet && whole) || state.supervisor.is_finished(id) {
                break;
            }
            if started.elapsed() >= TURN_LIMIT {
                return Err(AppError::Refused(
                    "The session is still working: choose Stop now, or try again once its turn \
                     is done"
                        .to_owned(),
                ));
            }
            // poll: the source's turn ending ("Finish this turn"); bounded
            // by TURN_LIMIT, then the user chooses Stop now.
            std::thread::sleep(Duration::from_millis(250));
        }
    }
    if state.supervisor.is_finished(id) {
        return Ok(());
    }
    state
        .supervisor
        .stop(id, Duration::from_secs(5))
        .map_err(|error| AppError::Refused(format!("The session didn't stop: {error}")))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !state.supervisor.is_finished(id) {
        if Instant::now() >= deadline {
            return Err(AppError::Refused(
                "The session didn't stop, so nothing was switched".to_owned(),
            ));
        }
        // poll: the stopped session's process ending; bounded by the
        // 10-second deadline above.
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

/// The target tool's transplant support, or why it has none.
fn handover_support(
    state: &Arc<AppState>,
    account: &juggler_core::model::Account,
) -> AppResult<Arc<dyn juggler_adapter::handover::HandoverSupport>> {
    state
        .registry
        .handover(&account.harness)
        .ok_or_else(|| AppError::Io {
            context: "hand over",
            message: format!(
                "{} can't take over this conversation — only a fresh start or handover notes",
                crate::features::tools::display_name(&state.registry, account.harness.as_str())
            ),
        })
}

/// The user's freeze choice (`stop-now`, else finish the turn first).
fn freeze_of(choice: &str) -> juggler_handover::pipeline::FreezeChoice {
    match choice {
        "stop-now" => juggler_handover::pipeline::FreezeChoice::StopNowAndSwitch,
        _ => juggler_handover::pipeline::FreezeChoice::FinishTurnThenSwitch,
    }
}

/// Both sides of a switch: the source session, its transcript and
/// account, and the target recipe and account.
pub(crate) struct SwitchSides {
    pub(crate) source: juggler_core::model::InstanceRecord,
    pub(crate) reference: juggler_core::model::TranscriptRef,
    pub(crate) account_a: Account,
    pub(crate) profile_b: juggler_core::model::LaunchRecipe,
    pub(crate) account_b: Account,
}

/// Resolves [`SwitchSides`]; a session without a project or a findable
/// transcript cannot switch.
fn switch_sides(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<SwitchSides> {
    let (profile_b, account_b) = find_profile_with_account_arc(state, target_profile_id)?;
    let (source, account_a) = handover_source_linked(state, session_id)?;
    if source.project.is_none() {
        return Err(AppError::Io {
            context: "hand over",
            message: "AI Juggler doesn't know which project this session belongs to, so it can't switch it".into(),
        });
    }
    let reference = switch_reference(state, &source, &account_b)?;
    // 0.3.5: the switch the user started finishes the target tool's own
    // first-run setup, the one thing left on a Claude app profile's folder.
    crate::features::handover::access::finish_first_run(state, &account_b)?;
    Ok(SwitchSides {
        source,
        reference,
        account_a,
        profile_b,
        account_b,
    })
}

/// The file a switch of `source` carries, once the switch may read it for
/// `target` (its folder's access checked).
///
/// # Errors
///
/// No transcript is known, or the access check refuses.
pub(crate) fn switch_reference(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    target: &juggler_core::model::Account,
) -> AppResult<juggler_core::model::TranscriptRef> {
    let reference = source.transcript_ref.clone().ok_or_else(|| AppError::Io {
        context: "hand over",
        message: "AI Juggler couldn't tell which file holds this conversation, so nothing moved"
            .into(),
    })?;
    crate::features::handover::access::check_switch_access(state, &reference.path, target)?;
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use juggler_core::model::{InstanceRecord, SessionStatus};

    use super::loop_trigger;

    fn record(id: &str, status: SessionStatus) -> InstanceRecord {
        crate::test_support::instance(id, "p1", status, std::env::temp_dir())
    }

    /// AJ-E1523: a switch away from a session at its usage limit counts
    /// as the limit's, however the screen was opened (a juggling loop when
    /// it succeeds); any other switch keeps its own reason.
    #[test]
    fn a_switch_from_a_session_at_its_limit_counts_as_a_juggling_loop() {
        let at_limit = record("s1", SessionStatus::QuotaBlocked);
        for trigger in ["manual", "suggestion", "quota_wall"] {
            assert_eq!(loop_trigger(&at_limit, trigger), "quota_wall");
        }
        for status in [
            SessionStatus::Running,
            SessionStatus::WaitingUser,
            SessionStatus::Stuck,
            SessionStatus::Finished,
            SessionStatus::Failed,
        ] {
            let other = record("s2", status);
            assert_eq!(loop_trigger(&other, "manual"), "manual", "{status:?}");
            assert_eq!(loop_trigger(&other, "suggestion"), "suggestion");
        }
    }
}
