//! AJ-E1542: before a switch, what the target account lacks, and setting
//! it up on the user's say-so.
//!
//! The source and target adapters each read what their account has set up
//! (`HarnessAdapter::inventory`: MCP servers, skills, commands, subagents,
//! user rules, plugins). Everything the source has and the target doesn't
//! becomes an offer, with the target tool's verdict: set up as it is,
//! translated, needs you (a login, a value to provide), or can't be set up
//! there and why. Nothing is written until the user picks offers and
//! applies them; each set-up is recorded with how to undo it, and undone
//! from the same place. Matching is by kind and name, and an account's own
//! rules are one item whatever the tool calls the file (`CLAUDE.md`,
//! `AGENTS.md`), so a target that has its own rules is never given more.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::inventory::{Applied, Item, ItemKind, SetUp, Undo};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The file the set-ups are recorded in, in AI Juggler's data folder.
const RECORD: &str = "switch-setups.json";

/// One thing the target could get.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OfferInfo {
    /// Pass back to apply it.
    pub id: String,
    /// `mcp-server`, `skill`, `command`, `subagent`, `rules`, `plugin`.
    pub kind: String,
    /// Its name.
    pub name: String,
    /// `user` or `project-private`.
    pub scope: String,
    /// `copy`, `translate`, `needs-you`, `impossible`.
    pub verdict: String,
    /// What the verdict means for this one, in a sentence.
    pub note: Option<String>,
    /// Ticked to start with: set up as it is, or translated.
    pub suggested: bool,
}

/// What the target lacks, for one switch.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SetupCheckInfo {
    /// The source account's name.
    pub from: String,
    /// The target account's name.
    pub to: String,
    /// What the target lacks, the ones that can be set up first.
    pub offers: Vec<OfferInfo>,
    /// How many items the source has that the target already has.
    pub already_there: u32,
}

/// What applying one offer did.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AppliedInfo {
    /// The offer.
    pub offer: String,
    /// The record to undo it with, when it worked.
    pub record: Option<String>,
    /// What was done, or why it wasn't.
    pub message: String,
}

/// A recorded set-up (kept until undone).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
struct Record {
    id: String,
    account: String,
    at_ms: i64,
    applied: Applied,
    /// What it is, in words ("skill release-notes"); empty in records
    /// made before it was kept.
    #[serde(default)]
    what: String,
}

/// The two sides of a switch, for the setup check.
struct Sides {
    source: juggler_core::model::Account,
    target: juggler_core::model::Account,
    project: Option<PathBuf>,
}

fn sides(state: &Arc<AppState>, session_id: &str, target_profile_id: &str) -> AppResult<Sides> {
    let (_, target) = crate::commands::find_profile_with_account_arc(state, target_profile_id)?;
    let session = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(session_id))
            .map_err(|_| AppError::Refused("that session isn't there any more".to_owned()))?
    };
    let source = {
        let storage = crate::commands::lock_storage_arc(state)?;
        let profile = storage.profiles().get(&session.profile)?;
        storage.accounts().get(&profile.account)?
    };
    Ok(Sides {
        source,
        target,
        project: Some(session.cwd).filter(|cwd| cwd.is_dir()),
    })
}

/// What each side has.
fn inventories(state: &Arc<AppState>, sides: &Sides) -> AppResult<(Vec<Item>, Vec<Item>)> {
    let adapter = |account: &juggler_core::model::Account| {
        state
            .registry
            .get(&account.harness)
            .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))
    };
    let project = sides.project.as_deref();
    Ok((
        adapter(&sides.source)?.inventory(&sides.source.state_dir, project),
        adapter(&sides.target)?.inventory(&sides.target.state_dir, project),
    ))
}

/// How items match across accounts and tools: by kind and name, with an
/// account's main rules file one item under either tool's name.
fn key(item: &Item) -> (ItemKind, String) {
    let name = if item.kind == ItemKind::Rules
        && matches!(item.name.as_str(), "CLAUDE.md" | "AGENTS.md")
    {
        "main rules".to_owned()
    } else {
        item.name.clone()
    };
    (item.kind, name)
}

/// An enum's wire name (`mcp-server`, `project-private`).
fn kebab<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// A stable id for an offer: its kind, name, scope and source tool.
fn offer_id(item: &Item) -> String {
    juggler_adapter::handover::sha256_hex(
        format!(
            "{:?}|{}|{:?}|{}",
            item.kind, item.name, item.scope, item.tool
        )
        .as_bytes(),
    )[..16]
        .to_owned()
}

/// What the target lacks: the source's items it doesn't have, each with
/// the target tool's verdict. Read-only.
///
/// # Errors
///
/// [`AppError::Refused`] when the session is gone; lookups' errors.
pub fn check_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<SetupCheckInfo> {
    let sides = sides(state, session_id, target_profile_id)?;
    let (from, to) = inventories(state, &sides)?;
    let target_adapter = state
        .registry
        .get(&sides.target.harness)
        .ok_or_else(|| AppError::UnknownHarness(sides.target.harness.to_string()))?;
    let have: std::collections::HashSet<(ItemKind, String)> = to.iter().map(key).collect();
    let mut already_there: u32 = 0;
    let mut offers = Vec::new();
    for item in &from {
        if have.contains(&key(item)) {
            already_there += 1;
            continue;
        }
        let (verdict, note) = match target_adapter.can_set_up(item) {
            SetUp::Copy => ("copy", None),
            SetUp::Translate(note) => ("translate", Some(note)),
            SetUp::NeedsYou(note) => ("needs-you", Some(note)),
            SetUp::Impossible(note) => ("impossible", Some(note)),
        };
        offers.push(OfferInfo {
            id: offer_id(item),
            kind: kebab(&item.kind),
            name: item.name.clone(),
            scope: kebab(&item.scope),
            verdict: verdict.to_owned(),
            note,
            suggested: matches!(verdict, "copy" | "translate"),
        });
    }
    offers.sort_by_key(|offer| {
        (
            offer.verdict == "impossible",
            offer.kind.clone(),
            offer.name.clone(),
        )
    });
    Ok(SetupCheckInfo {
        from: sides.source.label.clone(),
        to: sides.target.label.clone(),
        offers,
        already_there,
    })
}

/// Sets up the offers the user picked, each recorded so it can be undone.
/// Anything that can't be set up, or that the target has meanwhile, is
/// reported and skipped; nothing the target has is ever replaced.
///
/// # Errors
///
/// As [`check_impl`]; a failed record write.
pub fn apply_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    picked: &[String],
) -> AppResult<Vec<AppliedInfo>> {
    let sides = sides(state, session_id, target_profile_id)?;
    let (from, to) = inventories(state, &sides)?;
    let target_adapter = state
        .registry
        .get(&sides.target.harness)
        .ok_or_else(|| AppError::UnknownHarness(sides.target.harness.to_string()))?;
    let have: std::collections::HashSet<(ItemKind, String)> = to.iter().map(key).collect();
    let mut out = Vec::new();
    for id in picked {
        let Some(item) = from.iter().find(|item| &offer_id(item) == id) else {
            out.push(AppliedInfo {
                offer: id.clone(),
                record: None,
                message: "It isn't on the source account any more".to_owned(),
            });
            continue;
        };
        if have.contains(&key(item)) {
            out.push(AppliedInfo {
                offer: id.clone(),
                record: None,
                message: format!("{} already has {}", sides.target.label, item.name),
            });
            continue;
        }
        match target_adapter.set_up(&sides.target.state_dir, sides.project.as_deref(), item) {
            Ok(applied) => {
                let what = format!("{} {}", item.kind.words(), item.name);
                let record = Record {
                    id: format!("setup-{}", crate::commands::unique_suffix()),
                    account: sides.target.id.to_string(),
                    at_ms: juggler_core::model::Timestamp::now().millis(),
                    applied,
                    what: what.clone(),
                };
                save(state, &record)?;
                out.push(AppliedInfo {
                    offer: id.clone(),
                    record: Some(record.id),
                    message: format!("Set up the {what}"),
                });
            }
            Err(error) => out.push(AppliedInfo {
                offer: id.clone(),
                record: None,
                message: error.to_string(),
            }),
        }
    }
    Ok(out)
}

/// Undoes one recorded set-up: removes what it created, if still exactly
/// as it was made, or runs the tool's own removal.
///
/// # Errors
///
/// [`AppError::Refused`] when there's no such record, or what was made has
/// changed since (then nothing is removed); the undo's error.
pub fn undo_impl(state: &Arc<AppState>, record_id: &str) -> AppResult<String> {
    let mut records = load(state);
    let at = records
        .iter()
        .position(|record| record.id == record_id)
        .ok_or_else(|| AppError::Refused("there's nothing to undo".to_owned()))?;
    undo(&records[at].applied.undo)?;
    let record = records.remove(at);
    write(state, &records)?;
    // The tool's backup goes with the last set-up that needed it.
    if let Undo::RemoveAppended {
        backup: Some(backup),
        ..
    } = &record.applied.undo
    {
        let still_needed = records.iter().any(|other| {
            matches!(&other.applied.undo, Undo::RemoveAppended { backup: Some(b), .. } if b == backup)
        });
        if !still_needed {
            let _ = std::fs::remove_file(backup);
        }
    }
    Ok(if record.what.is_empty() {
        format!("Undid: {}", record.applied.summary)
    } else {
        format!("Removed the {}", record.what)
    })
}

fn undo(how: &Undo) -> AppResult<()> {
    match how {
        Undo::RemoveCreated { files, dirs } => {
            let changed: Vec<&PathBuf> = files
                .iter()
                .filter(|(path, sha)| {
                    std::fs::read(path)
                        .is_ok_and(|bytes| juggler_adapter::handover::sha256_hex(&bytes) != *sha)
                })
                .map(|(path, _)| path)
                .collect();
            if let Some(first) = changed.first() {
                return Err(AppError::Refused(format!(
                    "{} has changed since it was set up, so it was left as it is",
                    first.display()
                )));
            }
            for (path, _) in files {
                let _ = std::fs::remove_file(path);
            }
            for dir in dirs.iter().rev() {
                let _ = std::fs::remove_dir(dir);
            }
            Ok(())
        }
        Undo::RemoveAppended { file, text, .. } => {
            let now =
                std::fs::read_to_string(file).map_err(|e| AppError::io("undo the set-up", &e))?;
            let Some(at) = now.find(text.as_str()) else {
                return Err(AppError::Refused(format!(
                    "{} doesn't have that change any more, so it was left as it is",
                    file.display()
                )));
            };
            let mut restored = now.clone();
            restored.replace_range(at..at + text.len(), "");
            atomic_write(file, &restored)
        }
        Undo::Command { argv, env } => {
            let (program, args) = argv
                .split_first()
                .ok_or_else(|| AppError::Refused("nothing to run".to_owned()))?;
            let mut command = std::process::Command::new(program);
            command.args(args).stdin(std::process::Stdio::null());
            if env.is_empty() {
                command.env_remove("CLAUDE_CONFIG_DIR");
            }
            for (key, value) in env {
                command.env(key, value);
            }
            let output = command
                .output()
                .map_err(|e| AppError::io("undo the set-up", &e))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(AppError::Refused(
                    String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                ))
            }
        }
    }
}

fn atomic_write(path: &Path, text: &str) -> AppResult<()> {
    let staged = path.with_extension(format!("ai-juggler-{}", std::process::id()));
    std::fs::write(&staged, text).map_err(|e| AppError::io("undo the set-up", &e))?;
    std::fs::rename(&staged, path).map_err(|e| AppError::io("undo the set-up", &e))
}

fn load(state: &Arc<AppState>) -> Vec<Record> {
    std::fs::read(state.data_dir.join(RECORD))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save(state: &Arc<AppState>, record: &Record) -> AppResult<()> {
    let mut records = load(state);
    records.push(record.clone());
    write(state, &records)
}

fn write(state: &Arc<AppState>, records: &[Record]) -> AppResult<()> {
    let text = serde_json::to_string_pretty(records).unwrap_or_default();
    atomic_write(&state.data_dir.join(RECORD), &text)
}

/// What the target lacks before a switch (read-only).
///
/// # Errors
///
/// As [`check_impl`].
#[tauri::command]
pub async fn switch_setup_check(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
) -> AppResult<SetupCheckInfo> {
    crate::ipc::blocking(state.inner(), "check what the account has", move |state| {
        check_impl(state, &session_id, &target_profile_id)
    })
    .await
}

/// Sets up the picked offers on the target.
///
/// # Errors
///
/// As [`apply_impl`].
#[tauri::command]
pub async fn switch_setup_apply(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
    offers: Vec<String>,
) -> AppResult<Vec<AppliedInfo>> {
    crate::ipc::blocking(state.inner(), "set it up", move |state| {
        apply_impl(state, &session_id, &target_profile_id, &offers)
    })
    .await
}

/// Undoes one set-up.
///
/// # Errors
///
/// As [`undo_impl`].
#[tauri::command]
pub async fn switch_setup_undo(
    state: tauri::State<'_, Arc<AppState>>,
    record: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "undo the set-up", move |state| {
        undo_impl(state, &record)
    })
    .await
}
