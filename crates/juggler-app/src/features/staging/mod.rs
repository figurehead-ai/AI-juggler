//! AJ-E1513: the switch, prepared before the limit. Once a live
//! session's account crosses the quota warning (a window at 80 % or
//! more, or a forecast under 45 minutes), the background quota tick runs
//! the switch check for the
//! account the limit dialog would suggest and writes a redacted pack of
//! the conversation under `data_dir/staged/<session>/` — so the dialog
//! can say "Ready to continue on Work" and the switch has nothing left
//! to work out.
//!
//! Nothing here switches. It runs the switch check (a pure read since
//! #153) and writes the pack; it never freezes, launches or executes a
//! switch (`tests/trust.rs`'s `nothing_switches_on_its_own` holds it to
//! that). The pack holds conversation text, so its folder and files are
//! this user's only (0700 / 0600, as the handover's), and it goes when
//! the session ends or its window resets — and at startup, for anything
//! a crash left behind.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_core::model::{Account, InstanceRecord, SessionStatus};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The folder under `data_dir` that holds staged switches.
pub const FOLDER: &str = "staged";
/// A staged switch is refreshed at most this often, and only when its
/// transcript changed.
pub const REFRESH_MS: i64 = 5 * 60 * 1000;
/// A window this full has crossed the warning.
pub const WARN_PCT: f64 = 80.0;
/// The largest transcript staged in the background: a pack's own cap
/// on one file. A larger one is left to the switch itself.
pub const TRANSCRIPT_MAX: u64 = juggler_handover::pack_read::MAX_FILE;

const META: &str = "staged.json";
const PACK: &str = "pack";
const NEXT: &str = ".next";

/// A staged switch, as the limit dialog shows it.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StagedSwitchInfo {
    /// The session it was prepared for.
    pub session_id: String,
    /// The launch setting it would continue on.
    pub target_recipe_id: String,
    /// That launch setting's account.
    pub target_account_id: String,
    /// The account's name ("Work").
    pub target_account_label: String,
    /// When it was prepared, epoch ms.
    #[ts(type = "number")]
    pub prepared_at_ms: i64,
    /// What the switch carries, in words.
    pub fidelity_words: String,
}

/// The transcript as it was when the pack was made.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
struct Fingerprint {
    len: u64,
    modified_ms: i64,
}

/// What `staged.json` keeps beside the pack (no conversation text).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Meta {
    ready: StagedSwitchInfo,
    transcript: Fingerprint,
    /// The reset of the window that crossed: the pack goes after it.
    window_resets_at_ms: Option<i64>,
}

/// The folder staged switches live in.
#[must_use]
pub fn root(state: &AppState) -> PathBuf {
    state.data_dir.join(FOLDER)
}

/// Whether a session in `status` keeps its staged switch: it is live,
/// or stopped by its limit (exactly when the switch is wanted).
#[must_use]
pub fn keeps(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Starting
            | SessionStatus::Running
            | SessionStatus::WaitingUser
            | SessionStatus::Stuck
            | SessionStatus::QuotaBlocked
    )
}

/// A session id that is one plain path segment.
fn plain(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// A crossed quota warning.
#[derive(Clone, Copy)]
struct Crossed {
    /// When the crossed window resets (the soonest window's reset when
    /// the crossing was the forecast or the limit itself), if known.
    resets_at_ms: Option<i64>,
}

/// Whether `account`'s quota warning is crossed. A session stopped by
/// its limit crosses it through its window (100 %); once that window
/// resets, nothing is crossed and nothing is staged again.
fn warning(
    storage: &juggler_core::storage::Storage,
    account: &Account,
    now_ms: i64,
) -> Option<Crossed> {
    let display = crate::features::quota::quota_display_inner(storage, account, now_ms);
    let windows: Vec<&crate::features::quota::WindowDisplay> =
        [display.five_hour.as_ref(), display.weekly.as_ref()]
            .into_iter()
            .flatten()
            .collect();
    let full: Vec<&&crate::features::quota::WindowDisplay> = windows
        .iter()
        .filter(|window| window.percentage.is_some_and(|pct| pct >= WARN_PCT))
        .collect();
    let running_out = display
        .forecast
        .tte_min
        .is_some_and(|tte| tte < juggler_core::alerts::TTE_TRAY_MIN);
    if full.is_empty() && !running_out {
        return None;
    }
    let soonest = |of: &mut dyn Iterator<Item = &crate::features::quota::WindowDisplay>| {
        of.filter_map(|window| window.resets_at_ms).min()
    };
    Some(Crossed {
        resets_at_ms: soonest(&mut full.iter().map(|window| **window))
            .or_else(|| soonest(&mut windows.iter().copied())),
    })
}

fn fingerprint(path: &Path) -> Option<Fingerprint> {
    let meta = std::fs::metadata(path).ok()?;
    let modified_ms = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|age| i64::try_from(age.as_millis()).ok())?;
    Some(Fingerprint {
        len: meta.len(),
        modified_ms,
    })
}

fn read_meta(dir: &Path) -> Option<Meta> {
    let raw = std::fs::read_to_string(dir.join(META)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn discard(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
fn private(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)] // the same shape as the Unix one
fn private(_path: &Path, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

/// Makes `dir` and everything in it this user's only: folders 0700,
/// files 0600. Links are left alone (a pack never holds one).
fn lock_down(dir: &Path) -> std::io::Result<()> {
    private(dir, 0o700)?;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            lock_down(&entry.path())?;
        } else if kind.is_file() {
            private(&entry.path(), 0o600)?;
        }
    }
    Ok(())
}

/// A private folder, created when missing.
fn private_folder(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    private(dir, 0o700)
}

/// The dialog's pick: the suggested target on another account, else the
/// best one (the same rule as the limit dialog's `pickTarget`).
fn pick<'a>(
    targets: &'a [crate::commands::SwitchTargetInfo],
    from_account: &str,
) -> Option<&'a crate::commands::SwitchTargetInfo> {
    let usable: Vec<&crate::commands::SwitchTargetInfo> = targets
        .iter()
        .filter(|target| target.account_id != from_account && target.filtered_reason.is_none())
        .collect();
    usable
        .iter()
        .find(|target| target.suggested)
        .or_else(|| usable.first())
        .copied()
}

fn io(error: &std::io::Error) -> AppError {
    AppError::Io {
        context: "prepare the switch",
        message: error.to_string(),
    }
}

/// Removes every staged switch whose session no longer keeps one (`live`
/// says which do) or whose window has reset by `now_ms`.
fn sweep(root: &Path, live: &dyn Fn(&str) -> bool, now_ms: i64) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let reset = read_meta(&dir)
            .and_then(|meta| meta.window_resets_at_ms)
            .is_some_and(|at| now_ms >= at);
        if !live(&name) || reset {
            discard(&dir);
        }
    }
}

fn sessions(state: &AppState) -> Vec<InstanceRecord> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.sessions().list(None).ok())
        .unwrap_or_default()
}

/// At startup: drops what a crash left for sessions that are no longer
/// live.
pub fn sweep_at_startup(state: &AppState) {
    let live: std::collections::BTreeSet<String> = sessions(state)
        .into_iter()
        .filter(|record| keeps(record.status))
        .map(|record| record.id.as_str().to_owned())
        .collect();
    sweep(
        &root(state),
        &|id| live.contains(id),
        crate::commands::now_millis(),
    );
}

/// One pass, from the background quota tick: drops what ended or reset,
/// then stages (or refreshes) a switch for every live session whose
/// account crossed the warning. Returns how many it staged. Failures
/// only mean nothing is ready; the switch itself still works.
pub fn tick(state: &Arc<AppState>, now_ms: i64) -> usize {
    let live: Vec<InstanceRecord> = sessions(state)
        .into_iter()
        .filter(|record| keeps(record.status))
        .collect();
    let root = root(state);
    sweep(
        &root,
        &|id| live.iter().any(|record| record.id.as_str() == id),
        now_ms,
    );
    crossed(state, &live, now_ms)
        .into_iter()
        .filter(|(record, crossed)| {
            matches!(stage_one(state, &root, record, *crossed, now_ms), Ok(true))
        })
        .count()
}

/// The live sessions whose account crossed the warning: each account's
/// quota is read once, and nothing else is looked at for the rest.
fn crossed<'a>(
    state: &AppState,
    live: &'a [InstanceRecord],
    now_ms: i64,
) -> Vec<(&'a InstanceRecord, Crossed)> {
    let Ok(storage) = state.storage.lock() else {
        return Vec::new();
    };
    let mut by_account: std::collections::BTreeMap<String, Option<Crossed>> =
        std::collections::BTreeMap::new();
    live.iter()
        .filter_map(|record| {
            let profile = storage.profiles().get(&record.profile).ok()?;
            let key = profile.account.as_str().to_owned();
            let crossed = if let Some(known) = by_account.get(&key) {
                *known
            } else {
                let account = storage.accounts().get(&profile.account).ok()?;
                let crossed = warning(&storage, &account, now_ms);
                by_account.insert(key, crossed);
                crossed
            };
            crossed.map(|crossed| (record, crossed))
        })
        .collect()
}

/// Stages one session's switch (its warning is crossed) when nothing
/// fresh is staged. `Ok(false)`: nothing to do.
fn stage_one(
    state: &Arc<AppState>,
    root: &Path,
    record: &InstanceRecord,
    crossed: Crossed,
    now_ms: i64,
) -> AppResult<bool> {
    let id = record.id.as_str();
    if !plain(id) {
        return Ok(false);
    }
    let dir = root.join(id);
    let previous = read_meta(&dir);
    if previous
        .as_ref()
        .is_some_and(|meta| now_ms - meta.ready.prepared_at_ms < REFRESH_MS)
    {
        return Ok(false);
    }
    let (source, account) = crate::commands::handover_source(state, id)?;
    let window_resets_at_ms = crossed.resets_at_ms;
    let Some(reference) = source.transcript_ref.clone() else {
        return Ok(false);
    };
    let Some(print) = fingerprint(&reference.path) else {
        return Ok(false);
    };
    if print.len > TRANSCRIPT_MAX {
        discard(&dir);
        return Ok(false);
    }
    let targets = crate::commands::switch_targets_impl(state, id)?;
    let Some(target) = pick(&targets, account.id.as_str()) else {
        discard(&dir);
        return Ok(false);
    };
    if previous.is_some_and(|meta| {
        meta.transcript == print && meta.ready.target_recipe_id == target.recipe_id
    }) {
        return Ok(false);
    }

    // One read and one parse for the pack's turns.
    let bytes = juggler_adapters::read_session_file(&reference.path).map_err(|e| io(&e))?;
    let parsed = juggler_adapters::parse_any(&bytes);
    drop(bytes);
    let (_, turns) = crate::commands::turns_of(parsed);
    let preflight = match crate::commands::preflight_with(state, id, &target.recipe_id) {
        Ok(preflight) => preflight,
        Err(error) => {
            // A switch that would stop isn't ready.
            discard(&dir);
            return Err(error);
        }
    };

    private_folder(root).map_err(|e| io(&e))?;
    private_folder(&dir).map_err(|e| io(&e))?;
    let next = dir.join(NEXT);
    discard(&next);
    let ids = crate::commands::PackIds {
        work: String::new(),
        epoch: String::new(),
        native: reference.native_id.clone().unwrap_or_default(),
    };
    let written =
        crate::commands::write_session_pack(state, &source, &account, &ids, &next, Some(turns))
            .and_then(|()| lock_down(&next).map_err(|e| io(&e)));
    if let Err(error) = written {
        discard(&next);
        return Err(error);
    }
    let pack = dir.join(PACK);
    discard(&pack);
    std::fs::rename(&next, &pack).map_err(|e| io(&e))?;
    let meta = Meta {
        ready: StagedSwitchInfo {
            session_id: id.to_owned(),
            target_recipe_id: target.recipe_id.clone(),
            target_account_id: target.account_id.clone(),
            target_account_label: target.account_label.clone(),
            prepared_at_ms: now_ms,
            fidelity_words: preflight.fidelity_words,
        },
        transcript: print,
        window_resets_at_ms,
    };
    let path = dir.join(META);
    std::fs::write(&path, serde_json::to_vec(&meta).unwrap_or_default()).map_err(|e| io(&e))?;
    private(&path, 0o600).map_err(|e| io(&e))?;
    Ok(true)
}

/// The staged switch for `session_id`, when one is ready.
#[must_use]
pub fn ready_impl(state: &AppState, session_id: &str, now_ms: i64) -> Option<StagedSwitchInfo> {
    if !plain(session_id) {
        return None;
    }
    let dir = root(state).join(session_id);
    let meta = read_meta(&dir)?;
    let reset = meta.window_resets_at_ms.is_some_and(|at| now_ms >= at);
    (!reset && dir.join(PACK).is_dir()).then_some(meta.ready)
}

/// The staged switch for a session, when one is ready (the limit
/// dialog's "Ready to continue on …").
///
/// # Errors
///
/// None in practice: a missing or unreadable staged switch is `None`.
#[tauri::command]
pub async fn staging_ready(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Option<StagedSwitchInfo>> {
    crate::ipc::blocking(state.inner(), "read the prepared switch", move |state| {
        Ok(ready_impl(
            state,
            &session_id,
            crate::commands::now_millis(),
        ))
    })
    .await
}
