//! AJ-E1003: the quota-reset queue — park a quota-blocked Instance, and
//! be offered its continuation (on its own Account) when the window
//! resets.
//!
//! Passive by design: nothing ever resumes on its own. At the reset the
//! user is offered ONE "resume where you parked it" (an in-app offer and
//! a notification); dismissing it snoozes once, a second dismissal leaves
//! the Instance parked quietly. Resuming continues the original native
//! session on the original Account (D-05; AJ-E1001 identity first). When
//! the reset time is Estimated rather than Measured, the copy says so and
//! offers "check now" instead of a countdown.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;
use std::sync::Arc;

use juggler_core::model::{SessionId, SessionStatus, Timestamp, TranscriptRef};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The app setting holding the parked Instances.
const PARKED_KEY: &str = "parked_instances";
/// A snoozed offer comes back after this long — once.
const SNOOZE_MS: i64 = 30 * 60 * 1000;

/// One parked Instance (persisted).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
struct ParkedRow {
    account_id: String,
    account_label: String,
    parked_at_ms: i64,
    /// The vendor reset instant, when known.
    resets_at_ms: Option<i64>,
    /// `measured` | `estimated` | `unknown`.
    confidence: String,
    /// The staged portable pack.
    pack_dir: String,
    /// Offers shown so far (the reset one, then at most the snoozed one).
    offers: u8,
    /// A snoozed offer returns at this instant.
    snoozed_until_ms: Option<i64>,
    /// The user dismissed it twice: stay parked, quietly.
    quiet: bool,
}

fn load(state: &Arc<AppState>) -> BTreeMap<String, ParkedRow> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(PARKED_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save(state: &Arc<AppState>, parked: &BTreeMap<String, ParkedRow>) -> AppResult<()> {
    let raw = serde_json::to_string(parked).map_err(|e| AppError::Io {
        context: "parked instances",
        message: e.to_string(),
    })?;
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(PARKED_KEY, &raw)?;
    Ok(())
}

/// Parked Instances straight from storage (the read model's join):
/// session id → (reset instant, confidence).
pub(crate) fn parked_index(
    storage: &juggler_core::storage::Storage,
) -> BTreeMap<String, (Option<i64>, String)> {
    storage
        .app_settings()
        .get(PARKED_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<BTreeMap<String, ParkedRow>>(&raw).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|(id, row)| (id, (row.resets_at_ms, row.confidence)))
        .collect()
}

/// A parked Instance, as the rail and inspector show it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ParkedInfo {
    /// The Instance.
    pub session_id: String,
    /// Its Account.
    pub account_label: String,
    /// When it was parked, epoch ms.
    #[ts(type = "number")]
    pub parked_at_ms: i64,
    /// The window reset, epoch ms — `null` when not known.
    #[ts(type = "number | null")]
    pub resets_at_ms: Option<i64>,
    /// `measured` | `estimated` | `unknown` — only `measured` gets a
    /// countdown.
    pub confidence: String,
    /// The reset has passed.
    pub due: bool,
    /// The staged pack (where the context waits).
    pub pack_dir: String,
}

fn info(session_id: &str, row: &ParkedRow, now: i64) -> ParkedInfo {
    ParkedInfo {
        session_id: session_id.to_owned(),
        account_label: row.account_label.clone(),
        parked_at_ms: row.parked_at_ms,
        resets_at_ms: row.resets_at_ms,
        confidence: row.confidence.clone(),
        due: row.resets_at_ms.is_some_and(|at| now >= at),
        pack_dir: row.pack_dir.clone(),
    }
}

/// The reset the Instance waits for: the latest reset among the
/// Account's exhausted windows (both must reset to continue), else the
/// earliest known one; with how much it can be trusted.
fn reset_of(
    state: &Arc<AppState>,
    account: &juggler_core::model::Account,
) -> (Option<i64>, String) {
    let Ok(storage) = state.storage.lock() else {
        return (None, "unknown".to_owned());
    };
    let display =
        crate::commands::quota_display_inner(&storage, account, Timestamp::now().millis());
    let windows: Vec<&crate::commands::WindowDisplay> =
        [display.five_hour.as_ref(), display.weekly.as_ref()]
            .into_iter()
            .flatten()
            .filter(|window| window.resets_at_ms.is_some())
            .collect();
    let exhausted: Vec<&&crate::commands::WindowDisplay> = windows
        .iter()
        .filter(|window| window.state == "exhausted")
        .collect();
    let chosen = if exhausted.is_empty() {
        windows.iter().min_by_key(|window| window.resets_at_ms)
    } else {
        exhausted
            .into_iter()
            .max_by_key(|window| window.resets_at_ms)
    };
    match chosen {
        Some(window) => {
            let confidence = if window
                .badge
                .as_deref()
                .is_some_and(|badge| badge.eq_ignore_ascii_case("measured"))
            {
                "measured"
            } else {
                "estimated"
            };
            (window.resets_at_ms, confidence.to_owned())
        }
        None => (None, "unknown".to_owned()),
    }
}

/// Core: parks a quota-blocked Instance — its pack staged, the reset
/// recorded.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when the Instance isn't quota-blocked
/// or packs have no secret policy yet; storage/pack errors otherwise.
pub fn instance_park_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<ParkedInfo> {
    let (session, account) = crate::commands::handover_source(state, session_id)?;
    if session.status != SessionStatus::QuotaBlocked {
        return Err(AppError::Validation {
            subject: "park",
            message: "only a session blocked by its quota can be parked until the reset"
                .to_owned(),
        });
    }
    let dir = state.data_dir.join("parked").join(session_id);
    let _ = std::fs::remove_dir_all(&dir);
    let native = session
        .transcript_ref
        .as_ref()
        .and_then(|reference| reference.native_id.clone())
        .unwrap_or_default();
    crate::commands::stage_session_pack(
        state,
        &session,
        &account,
        &crate::commands::PackIds {
            work: String::new(),
            epoch: String::new(),
            native,
        },
        &dir,
    )?;
    let (resets_at_ms, confidence) = reset_of(state, &account);
    let now = Timestamp::now().millis();
    let row = ParkedRow {
        account_id: account.id.to_string(),
        account_label: account.label.clone(),
        parked_at_ms: now,
        resets_at_ms,
        confidence,
        pack_dir: dir.to_string_lossy().into_owned(),
        offers: 0,
        snoozed_until_ms: None,
        quiet: false,
    };
    let mut parked = load(state);
    parked.insert(session_id.to_owned(), row.clone());
    save(state, &parked)?;
    Ok(info(session_id, &row, now))
}

/// Core: every parked Instance (the rail's and inspector's view).
#[must_use]
pub fn parked_list_impl(state: &Arc<AppState>) -> Vec<ParkedInfo> {
    let now = Timestamp::now().millis();
    load(state)
        .iter()
        .map(|(id, row)| info(id, row, now))
        .collect()
}

/// A resume offer at the actionable moment.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ParkedOfferInfo {
    /// The parked Instance.
    pub session_id: String,
    /// Its Account.
    pub account_label: String,
    /// The offer copy.
    pub text: String,
    /// This is the snoozed (second, last) offer.
    pub last: bool,
}

/// Core: the resume offers due now — evaluated with the alert/quota poll
/// (no timer of its own). An offer appears once at the reset; a snoozed
/// one once more; never again after that.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn parked_offers_impl(state: &Arc<AppState>) -> AppResult<Vec<ParkedOfferInfo>> {
    let now = Timestamp::now().millis();
    let mut parked = load(state);
    let mut out = Vec::new();
    let mut changed = false;
    for (id, row) in &mut parked {
        let due = row.resets_at_ms.is_some_and(|at| now >= at);
        let snooze_over = row.snoozed_until_ms.is_none_or(|until| now >= until);
        if !due || row.quiet || !snooze_over {
            continue;
        }
        if row.offers == 0 || (row.snoozed_until_ms.is_some() && row.offers == 1) {
            row.offers += 1;
            changed = true;
            notify_log(state, id, row);
        }
        out.push(ParkedOfferInfo {
            session_id: id.clone(),
            account_label: row.account_label.clone(),
            text: format!(
                "{} has reset. Resume the work you parked — the same conversation, on the same Account.",
                row.account_label
            ),
            last: row.offers >= 2,
        });
    }
    if changed {
        save(state, &parked)?;
    }
    Ok(out)
}

/// The notification log: one line per offer actually raised.
fn notify_log(state: &Arc<AppState>, session_id: &str, row: &ParkedRow) {
    let line = serde_json::json!({
        "at_ms": Timestamp::now().millis(),
        "kind": "parked_reset_offer",
        "session_id": session_id,
        "account": row.account_label,
        "offer": row.offers,
    });
    let path = state.data_dir.join("parked").join("offers.jsonl");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write as _;
        let _ = writeln!(file, "{line}");
    }
}

/// Core: "Not now" — the first dismissal snoozes once (30 min); the
/// second leaves the Instance parked without further offers.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] for an Instance that isn't parked.
pub fn parked_dismiss_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<()> {
    let mut parked = load(state);
    let row = parked
        .get_mut(session_id)
        .ok_or_else(|| AppError::NotFound {
            entity: "parked session",
            id: session_id.to_owned(),
        })?;
    if row.snoozed_until_ms.is_none() {
        row.snoozed_until_ms = Some(Timestamp::now().millis() + SNOOZE_MS);
    } else {
        row.quiet = true;
    }
    save(state, &parked)
}

/// Core: "check now" for an Estimated reset — re-reads the quota engine.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] for an Instance that isn't parked.
pub fn parked_check_now_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<ParkedInfo> {
    let (_, account) = crate::commands::handover_source(state, session_id)?;
    let (resets_at_ms, confidence) = reset_of(state, &account);
    let mut parked = load(state);
    let row = parked
        .get_mut(session_id)
        .ok_or_else(|| AppError::NotFound {
            entity: "parked session",
            id: session_id.to_owned(),
        })?;
    row.resets_at_ms = resets_at_ms;
    row.confidence = confidence;
    let out = info(session_id, row, Timestamp::now().millis());
    save(state, &parked)?;
    Ok(out)
}

/// Core: resumes a parked Instance — the original native session on the
/// original Account (D-05), identity asserted first (AJ-E1001). Only on
/// the user's click; it is never called by a timer.
///
/// # Errors
///
/// Returns the typed [`AppError`] when it isn't parked, has no native
/// session or project, or the launch is refused.
pub fn parked_resume_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<crate::commands::LaunchResult> {
    let parked = load(state);
    if !parked.contains_key(session_id) {
        return Err(AppError::NotFound {
            entity: "parked session",
            id: session_id.to_owned(),
        });
    }
    let (session, account) = crate::commands::handover_source(state, session_id)?;
    let reference: TranscriptRef =
        session
            .transcript_ref
            .clone()
            .ok_or_else(|| AppError::Validation {
                subject: "resume",
                message: "the parked session has no conversation to resume".to_owned(),
            })?;
    let native = reference.native_id.clone().unwrap_or_default();
    let project_id = session
        .project
        .clone()
        .ok_or_else(|| AppError::Validation {
            subject: "resume",
            message: "the parked session isn't linked to a project".to_owned(),
        })?;
    let (profile, project) = {
        let storage = crate::commands::lock_storage_arc(state)?;
        (
            storage.profiles().get(&session.profile)?,
            storage.projects().get(&project_id)?,
        )
    };
    let (instance, pid) = crate::crossmachine::launch_here(
        state,
        &profile,
        &account,
        &project,
        &juggler_adapter::LaunchMode::Resume {
            session_id: native,
            delta_prompt: None,
        },
        Some(reference),
        "resume",
    )?;
    let mut parked = load(state);
    parked.remove(session_id);
    save(state, &parked)?;
    {
        let storage = crate::commands::lock_storage_arc(state)?;
        if let Ok(mut old) = storage.sessions().get(&SessionId::new(session_id)) {
            old.status = SessionStatus::HandedOver;
            storage.sessions().update(&old)?;
        }
    }
    Ok(crate::commands::LaunchResult {
        session_id: instance.to_string(),
        pid,
    })
}

/// Test seam: moves a parked Instance's reset instant (simulates the
/// window resetting).
#[doc(hidden)]
pub fn parked_set_reset_for_test(state: &Arc<AppState>, session_id: &str, at_ms: i64) {
    let mut parked = load(state);
    if let Some(row) = parked.get_mut(session_id) {
        row.resets_at_ms = Some(at_ms);
        "measured".clone_into(&mut row.confidence);
    }
    let _ = save(state, &parked);
}

/// Test seam: ends a snooze now.
#[doc(hidden)]
pub fn parked_end_snooze_for_test(state: &Arc<AppState>, session_id: &str) {
    let mut parked = load(state);
    if let Some(row) = parked.get_mut(session_id)
        && row.snoozed_until_ms.is_some()
    {
        row.snoozed_until_ms = Some(0);
    }
    let _ = save(state, &parked);
}

// ---------------------------------------------------------- the commands

/// AJ-E1003: park a quota-blocked Instance until its reset.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`instance_park_impl`].
#[tauri::command]
pub fn instance_park(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<ParkedInfo> {
    instance_park_impl(state.inner(), &session_id)
}

/// AJ-E1003: every parked Instance.
#[tauri::command]
#[must_use]
pub fn parked_list(state: tauri::State<'_, Arc<AppState>>) -> Vec<ParkedInfo> {
    parked_list_impl(state.inner())
}

/// AJ-E1003: the resume offers due now.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`parked_offers_impl`].
#[tauri::command]
pub fn parked_offers(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<ParkedOfferInfo>> {
    parked_offers_impl(state.inner())
}

/// AJ-E1003: "Not now" (snooze once, then quiet).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`parked_dismiss_impl`].
#[tauri::command]
pub fn parked_dismiss(state: tauri::State<'_, Arc<AppState>>, session_id: String) -> AppResult<()> {
    parked_dismiss_impl(state.inner(), &session_id)
}

/// AJ-E1003: re-read an Estimated reset.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`parked_check_now_impl`].
#[tauri::command]
pub fn parked_check_now(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<ParkedInfo> {
    parked_check_now_impl(state.inner(), &session_id)
}

/// AJ-E1003: resume a parked Instance (the user's click).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`parked_resume_impl`].
#[tauri::command]
pub fn parked_resume(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<crate::commands::LaunchResult> {
    parked_resume_impl(state.inner(), &session_id)
}
