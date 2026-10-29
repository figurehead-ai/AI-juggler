//! What the other tool was told (0.3): after a switch that wrote a
//! handover, the switch's done screen and its edge in the Switch Graph say
//! in one line what was hidden and what was left out, and open the
//! handover read-only, in the sheet "Start from a saved handover…" shows a
//! saved one in (the same rows and words). A switch that copies the whole
//! session (same tool) writes no handover, so there is nothing to show.
//! "Save handover…" on a session's page saves one, ending with the same
//! line and Reveal in Finder. Kinds and counts only: a secret's value is
//! never read back, shown or stored here.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_core::model::SwitchEdgeId;

use super::{FILE, FOLDER, HandoverState, STATE};
use crate::error::{AppError, AppResult};
use crate::features::packs::{WithheldInfo, tool_name, withheld_rows};
use crate::features::quota::capture::home_relative;
use crate::state::AppState;

/// A switch's handover, in one line, for its done screen and its edge.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct HandoverSummaryInfo {
    /// The tool it was written for (its name: "Codex").
    pub to_tool: String,
    /// What was hidden from it and what was left out, in one line;
    /// `None` when there's nothing to say.
    pub line: Option<String>,
}

/// A switch's handover, as the read-only sheet shows it: the rows
/// "Start from a saved handover…" shows a saved one with.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct HandoverSheetInfo {
    /// The tool it came from (its name).
    pub from_tool: String,
    /// The account it came from.
    pub from_account: String,
    /// The tool it was written for (its name).
    pub to_tool: String,
    /// The account it was written for.
    pub to_account: String,
    /// When it was written (epoch ms).
    #[ts(type = "number")]
    pub created_at_ms: i64,
    /// The folder the work runs in (home-relative).
    pub project_root: String,
    /// What carried, in the switch's words ("Full story, fresh start").
    pub fidelity: Option<String>,
    /// What it carries, in words.
    pub carries: Vec<String>,
    /// What it left out, and why.
    pub not_carried: Vec<String>,
    /// What the keys check hid from it; `None` for a handover written
    /// before that was recorded.
    pub withheld: Option<Vec<WithheldInfo>>,
    /// What it found and left as it was, as the user chose.
    pub kept: Vec<WithheldInfo>,
    /// The handover itself (`HANDOVER.md`), to read in full.
    pub brief: String,
    /// The one line, as the done screen shows it.
    pub line: Option<String>,
}

/// A handover saved from a session's page (or the Switch Graph).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SavedHandoverInfo {
    /// Its folder: what the sheet opens and Reveal in Finder shows.
    pub path: String,
    /// Its folder, home-relative, to show.
    pub shown: String,
    /// What was hidden and left out, in one line; `None` when there's
    /// nothing to say.
    pub line: Option<String>,
}

/// "2 API keys, 1 bearer token".
fn counted(rows: &[WithheldInfo]) -> String {
    rows.iter()
        .map(|row| format!("{} {}", row.count, row.what))
        .collect::<Vec<_>>()
        .join(", ")
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// The one line: what was hidden from `to` (or from the saved handover),
/// what was left as it was by the user's choice, and what was left out.
/// `None` when none of them has anything in it.
#[must_use]
pub fn told_line(
    to: Option<&str>,
    withheld: &[WithheldInfo],
    kept: &[WithheldInfo],
    left_out: &[String],
) -> Option<String> {
    let mut parts = Vec::new();
    if !withheld.is_empty() {
        parts.push(match to {
            Some(tool) => format!("Hidden from {tool}: {}", counted(withheld)),
            None => format!("Hidden: {}", counted(withheld)),
        });
    }
    if !kept.is_empty() {
        parts.push(match to {
            Some(tool) => format!("Sent to {tool} as you chose: {}", counted(kept)),
            None => format!("Left in as you chose: {}", counted(kept)),
        });
    }
    if !left_out.is_empty() {
        parts.push(format!("Left out: {}", left_out.join(", ")));
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// What a switch's handover left out, counted ("12 earlier turns",
/// "1 note").
fn left_out_counts(state: &HandoverState) -> Vec<String> {
    let mut out = Vec::new();
    if state.turns_left_out > 0 {
        out.push(plural(
            state.turns_left_out,
            "earlier turn",
            "earlier turns",
        ));
    }
    if !state.notes_left_out.is_empty() {
        out.push(plural(state.notes_left_out.len(), "note", "notes"));
    }
    out
}

/// The folder of edge `edge`'s handover, when the edge is this Mac's
/// (never a path the window sends) and its handover is there.
fn folder_of(state: &Arc<AppState>, edge: &str) -> AppResult<Option<PathBuf>> {
    let plain = !edge.is_empty()
        && Path::new(edge)
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
        && Path::new(edge).components().count() == 1;
    if !plain {
        return Ok(None);
    }
    let known = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .switch_graph()
            .edge(&SwitchEdgeId::new(edge.to_owned()))
            .is_ok()
    };
    let folder = state.data_dir.join(FOLDER).join(edge);
    Ok((known && folder.join(FILE).is_file() && folder.join(STATE).is_file()).then_some(folder))
}

fn read_state(folder: &Path) -> AppResult<HandoverState> {
    let text = std::fs::read_to_string(folder.join(STATE)).map_err(|error| AppError::Io {
        context: "the handover",
        message: error.to_string(),
    })?;
    serde_json::from_str(&text).map_err(|_| AppError::Io {
        context: "the handover",
        message: "AI Juggler couldn't read what this handover recorded".to_owned(),
    })
}

fn summary_of(state: &Arc<AppState>, saved: &HandoverState) -> HandoverSummaryInfo {
    let to_tool = tool_name(state, &saved.to.tool);
    let withheld = withheld_rows(saved.withheld.as_deref().unwrap_or_default());
    let line = told_line(
        Some(&to_tool),
        &withheld,
        &withheld_rows(&saved.kept),
        &left_out_counts(saved),
    );
    HandoverSummaryInfo { to_tool, line }
}

/// Core: edge `edge`'s handover in one line; `None` when it has none (a
/// same-tool switch copies the session and writes no handover).
///
/// # Errors
///
/// The storage error, or [`AppError::Io`] when its state can't be read.
pub fn summary_impl(state: &Arc<AppState>, edge: &str) -> AppResult<Option<HandoverSummaryInfo>> {
    match folder_of(state, edge)? {
        Some(folder) => Ok(Some(summary_of(state, &read_state(&folder)?))),
        None => Ok(None),
    }
}

/// The Switch Graph's line for an edge, when it has a handover: as
/// [`summary_impl`], quiet on any error (the graph still draws).
pub(crate) fn edge_summary(state: &Arc<AppState>, edge: &str) -> Option<HandoverSummaryInfo> {
    summary_impl(state, edge).ok().flatten()
}

/// What a switch's handover carries and doesn't, in words.
fn carried(saved: &HandoverState) -> (Vec<String>, Vec<String>) {
    let mut carries = vec![format!(
        "the conversation ({})",
        plural(saved.turns_included, "turn", "turns")
    )];
    if !saved.rules_files.is_empty() {
        carries.push(plural(saved.rules_files.len(), "rules file", "rules files"));
    }
    if !saved.notes_quoted.is_empty() {
        carries.push(plural(
            saved.notes_quoted.len(),
            "note for this project",
            "notes for this project",
        ));
    }
    let mut not_carried = Vec::new();
    if saved.turns_left_out > 0 {
        not_carried.push(format!(
            "{}: they didn't fit, so the handover names what was asked in them",
            plural(saved.turns_left_out, "earlier turn", "earlier turns")
        ));
    }
    if !saved.notes_left_out.is_empty() {
        let why = if saved.notes_left_out.len() == 1 {
            "a key or password was found in it, or it didn't fit"
        } else {
            "a key or password was found in them, or they didn't fit"
        };
        not_carried.push(format!("{}: {why}", saved.notes_left_out.join(", ")));
    }
    (carries, not_carried)
}

/// Core: edge `edge`'s handover, to read.
///
/// # Errors
///
/// [`AppError::Validation`] when the edge has no handover here; the
/// storage error, or [`AppError::Io`] when it can't be read.
pub fn open_impl(state: &Arc<AppState>, edge: &str) -> AppResult<HandoverSheetInfo> {
    let folder = folder_of(state, edge)?.ok_or_else(|| AppError::Validation {
        subject: "handover",
        message: "this switch has no handover on this computer".to_owned(),
    })?;
    let saved = read_state(&folder)?;
    let brief = std::fs::read_to_string(folder.join(FILE)).map_err(|error| AppError::Io {
        context: "the handover",
        message: error.to_string(),
    })?;
    let fidelity = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .switch_graph()
            .edge(&SwitchEdgeId::new(edge.to_owned()))
            .ok()
            .map(|row| crate::commands::edge_carried_words(&row).to_owned())
    };
    let (carries, not_carried) = carried(&saved);
    let summary = summary_of(state, &saved);
    Ok(HandoverSheetInfo {
        from_tool: tool_name(state, &saved.from.tool),
        from_account: saved.from.account.clone(),
        to_tool: summary.to_tool,
        to_account: saved.to.account.clone(),
        created_at_ms: saved.written_ms,
        project_root: home_relative(Path::new(&saved.folder)),
        fidelity,
        carries,
        not_carried,
        withheld: saved.withheld.as_deref().map(withheld_rows),
        kept: withheld_rows(&saved.kept),
        brief,
        line: summary.line,
    })
}

/// A saved handover's folder, described: what the sheet opens, and the
/// one line.
///
/// # Errors
///
/// As [`crate::features::packs::preview_impl`].
pub fn saved_info(state: &Arc<AppState>, dir: &Path) -> AppResult<SavedHandoverInfo> {
    let pack = crate::features::packs::preview_impl(state, dir)?;
    Ok(SavedHandoverInfo {
        path: dir.display().to_string(),
        shown: home_relative(dir),
        line: told_line(None, &pack.withheld, &pack.kept, &[]),
    })
}

/// Core: saves the handover of the session `session` (AI Juggler's, or
/// one another app runs, by its key) under `<app-data>/exports/`, keys
/// and passwords handled as the user chose (unset refuses).
///
/// # Errors
///
/// The typed error when the session can't be found, the choice is unset,
/// it has no conversation, or the pack can't be written.
pub fn save_impl(state: &Arc<AppState>, session: &str) -> AppResult<SavedHandoverInfo> {
    let (source, account) = crate::commands::handover_source(state, session)?;
    let dir = crate::commands::export_dir(state);
    let native = source
        .transcript_ref
        .as_ref()
        .and_then(|reference| reference.native_id.clone())
        .unwrap_or_default();
    crate::commands::stage_session_pack(
        state,
        &source,
        &account,
        &crate::commands::PackIds {
            work: String::new(),
            epoch: String::new(),
            native,
        },
        &dir,
    )?;
    saved_info(state, &dir)
}

/// The folder Reveal in Finder shows for `path`: a saved handover or a
/// switch's, inside AI Juggler's own folders only (never another path
/// the window sends).
///
/// # Errors
///
/// [`AppError::Validation`] for any other path.
pub fn reveal_target(state: &Arc<AppState>, path: &str) -> AppResult<PathBuf> {
    let refused = || AppError::Validation {
        subject: "handover",
        message: "that isn't a handover AI Juggler saved".to_owned(),
    };
    let wanted = std::fs::canonicalize(path).map_err(|_| refused())?;
    let inside = ["exports", FOLDER].iter().any(|folder| {
        std::fs::canonicalize(state.data_dir.join(folder))
            .is_ok_and(|root| wanted.starts_with(&root) && wanted != root)
    });
    if inside { Ok(wanted) } else { Err(refused()) }
}

/// A switch's handover in one line (`None`: it has none).
///
/// # Errors
///
/// As [`summary_impl`].
#[tauri::command]
pub async fn handover_summary(
    state: tauri::State<'_, Arc<AppState>>,
    edge_id: String,
) -> AppResult<Option<HandoverSummaryInfo>> {
    crate::ipc::blocking(state.inner(), "the handover", move |state| {
        summary_impl(state, &edge_id)
    })
    .await
}

/// A switch's handover, to read.
///
/// # Errors
///
/// As [`open_impl`].
#[tauri::command]
pub async fn handover_open(
    state: tauri::State<'_, Arc<AppState>>,
    edge_id: String,
) -> AppResult<HandoverSheetInfo> {
    crate::ipc::blocking(state.inner(), "the handover", move |state| {
        open_impl(state, &edge_id)
    })
    .await
}

/// Saves a session's handover.
///
/// # Errors
///
/// As [`save_impl`].
#[tauri::command]
pub async fn handover_save(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<SavedHandoverInfo> {
    crate::ipc::blocking(state.inner(), "saving the handover", move |state| {
        save_impl(state, &session_id)
    })
    .await
}

/// Reveal in Finder: a saved handover's folder, through the system's
/// file manager (the opener, as everywhere else).
///
/// # Errors
///
/// As [`reveal_target`], or the opener's error.
#[tauri::command]
pub async fn handover_reveal(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let target = crate::ipc::blocking(state.inner(), "the handover", move |state| {
        reveal_target(state, &path)
    })
    .await?;
    app.opener()
        .reveal_item_in_dir(&target)
        .map_err(|error| AppError::Io {
            context: "reveal in Finder",
            message: error.to_string(),
        })
}
