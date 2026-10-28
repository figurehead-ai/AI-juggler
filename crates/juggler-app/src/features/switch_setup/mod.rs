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
//!
//! What comes with the source's account (a claude.ai skill, an `OpenAI`
//! plugin) is never copied: the target adapter says how to turn it on
//! for the target ([`SetUp::TurnOn`]), and a switch remembers it until
//! the target has it ([`turn_ons`]). The source adapter says what the
//! conversation used ([`juggler_adapter::HarnessAdapter::used_in`]), so
//! those lead and the rest wait under one collapsed line
//! (switch-setup-items.md).

pub mod turn_ons;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::inventory::{Applied, GetIt, Item, ItemKind, Origin, SetUp, Undo};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The file the set-ups are recorded in, in AI Juggler's data folder.
const RECORD: &str = "switch-setups.json";

/// Held while the record file is read and written again: commands run in
/// parallel, and two saves at once would drop one record (review of
/// AJ-E1507, M3).
static RECORDS: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    /// `copy`, `translate`, `needs-you`, `impossible`, `turn-on` (the
    /// other account turns it on itself: `get_it` says how).
    pub verdict: String,
    /// What the verdict means for this one, in a sentence.
    pub note: Option<String>,
    /// What it holds, in a few words, for a setting ("2 allowed, 1
    /// denied"; AJ-E1508).
    pub about: Option<String>,
    /// Ticked to start with: set up as it is, or translated, and used in
    /// this conversation (or nothing says).
    pub suggested: bool,
    /// Whether this conversation used it; `null` when the tool's
    /// transcripts don't say, or it isn't something a conversation uses
    /// (rules). Never read `null` as "not used".
    pub used: Option<bool>,
    /// Where it came from: `own`, `account` (synced by the account, whose
    /// exactly can't be told), `provided`, `unknown`.
    pub origin: String,
    /// Who provides it, for `provided` (`Anthropic`, `OpenAI`).
    pub provided_by: Option<String>,
    /// How the user finishes it themselves, from the target tool's
    /// adapter (never written here): for `turn-on`, how the target
    /// account turns it on; for `needs-you`, how to give it the secret
    /// that is never copied (the steps and the command, a placeholder in
    /// place of the value). `null` when there are no steps.
    pub get_it: Option<GetItInfo>,
    /// For `turn-on`: whether it may be copied instead when it's the
    /// user's own (ticking it copies it, with an undo).
    pub can_copy: bool,
}

/// How an account gets an item it can't be given by a copy: the steps for
/// it, by name.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct GetItInfo {
    /// Short steps, in order.
    pub steps: Vec<String>,
    /// The one command those steps say to run, exactly as it's run (a
    /// secret in it is a placeholder, never a value).
    #[serde(default)]
    pub command: Option<String>,
    /// The page that does it (https), opened with `switch_setup_open`.
    pub open_url: Option<String>,
    /// What has to be true first, a sentence each.
    pub needs: Vec<String>,
    /// What happens after the last step.
    pub then: Option<String>,
}

impl GetItInfo {
    /// `get_it`, for the account named `account`.
    pub(crate) fn of(get_it: &GetIt, account: &str) -> Self {
        let get_it = get_it.for_account(account);
        if let Some(url) = &get_it.open_url {
            turn_ons::may_open(url);
        }
        Self {
            steps: get_it.steps,
            command: get_it.command,
            open_url: get_it.open_url,
            needs: get_it
                .needs
                .iter()
                .map(|need| need.text().to_owned())
                .collect(),
            then: get_it.then,
        }
    }
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
    /// What the user should know about what was done, when anything.
    pub note: Option<String>,
}

/// One set-up still in place, for the list that outlives the switch
/// screen (review of #132, M3).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SetUpInfo {
    /// The record to undo it with.
    pub record: String,
    /// Where it was set up: the account's name, or the project's for a
    /// change to a project's own files (AJ-E1507).
    pub place: String,
    /// What it is ("the skill release-notes"), or what was done, for a
    /// record made before that was kept.
    pub what: String,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// A recorded set-up (kept until undone).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
struct Record {
    id: String,
    /// The account it was set up on; `None` for a change to a project's
    /// own files (AJ-E1507). Older records always have one.
    account: Option<String>,
    at_ms: i64,
    applied: Applied,
    /// What it is, in words ("skill release-notes"); empty in records
    /// made before it was kept.
    #[serde(default)]
    what: String,
    /// The project it changed, for a change to a project's own files
    /// (AJ-E1507); then `account` is empty.
    #[serde(default)]
    project: Option<String>,
}

/// The two sides of a set-up: a switch's, or any two accounts in a
/// project (AJ-E1507's server copy).
pub(crate) struct Sides {
    pub(crate) source: juggler_core::model::Account,
    pub(crate) target: juggler_core::model::Account,
    pub(crate) project: Option<PathBuf>,
    /// The switching conversation's transcript, for what it used; `None`
    /// outside a switch.
    pub(crate) transcript: Option<PathBuf>,
}

fn sides(state: &Arc<AppState>, session_id: &str, target_profile_id: &str) -> AppResult<Sides> {
    let (_, target) = crate::commands::find_profile_with_account_arc(state, target_profile_id)?;
    // AJ-E2108: a session another app runs is read from the index, as the
    // switch reads it; a session AI Juggler runs from its row.
    let (session, source) = if let Some((account, conversation)) =
        crate::features::conversations::switch_source::parse_source_key(session_id)
    {
        crate::features::conversations::switch_source::as_switch_source(
            state,
            account,
            conversation,
            false,
        )?
    } else {
        let storage = crate::commands::lock_storage_arc(state)?;
        let session = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(session_id))
            .map_err(|_| AppError::Refused("that session isn't there any more".to_owned()))?;
        let profile = storage.profiles().get(&session.profile)?;
        let source = storage.accounts().get(&profile.account)?;
        (session, source)
    };
    Ok(Sides {
        source,
        target,
        project: Some(session.cwd).filter(|cwd| cwd.is_dir()),
        transcript: session.transcript_ref.map(|reference| reference.path),
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

/// How items match across accounts and tools: by kind and name; an
/// account has one main rules file, whatever its tool calls it.
fn key(item: &Item) -> (ItemKind, String) {
    let name = if item.kind == ItemKind::MainRules {
        String::new()
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
    check_sides(
        state,
        &sides(state, session_id, target_profile_id)?,
        &for_a_switch,
    )
}

/// What a switch offers: everything but the account's own settings, which
/// the target keeps (AJ-E1508 offers those when an account is added).
fn for_a_switch(kind: ItemKind) -> bool {
    !kind.is_account_setting()
}

/// What `sides.target` lacks of `sides.source`'s items of the kinds
/// `wanted` takes, each with the target tool's verdict. Read-only.
pub(crate) fn check_sides(
    state: &Arc<AppState>,
    sides: &Sides,
    wanted: &dyn Fn(ItemKind) -> bool,
) -> AppResult<SetupCheckInfo> {
    let (from, to) = inventories(state, sides)?;
    let from: Vec<Item> = from.into_iter().filter(|item| wanted(item.kind)).collect();
    let target_adapter = state
        .registry
        .get(&sides.target.harness)
        .ok_or_else(|| AppError::UnknownHarness(sides.target.harness.to_string()))?;
    // What the conversation used, by the source tool's own reading.
    let used = sides.transcript.as_deref().and_then(|transcript| {
        state
            .registry
            .get(&sides.source.harness)
            .and_then(|adapter| adapter.used_in(transcript))
    });
    let (offers, already_there) = offers(
        &sides.target,
        target_adapter.as_ref(),
        &from,
        &to,
        used.as_deref(),
        &|tool| crate::features::tools::display_name(&state.registry, tool),
    );
    Ok(SetupCheckInfo {
        from: sides.source.label.clone(),
        to: sides.target.label.clone(),
        offers,
        already_there,
    })
}

/// Whether the conversation used `item`, by what the source tool read
/// from it (`used`): `None` when it doesn't say, or when `item` isn't
/// something a conversation uses (rules, settings). A plugin matches by
/// its name without the marketplace.
fn used_of(item: &Item, used: Option<&[(ItemKind, String)]>) -> Option<bool> {
    let used = used?;
    if !matches!(
        item.kind,
        ItemKind::Skill
            | ItemKind::Command
            | ItemKind::Subagent
            | ItemKind::McpServer
            | ItemKind::Plugin
    ) {
        return None;
    }
    let name = if item.kind == ItemKind::Plugin {
        item.name.split('@').next().unwrap_or_default()
    } else {
        item.name.as_str()
    };
    Some(
        used.iter()
            .any(|(kind, used)| *kind == item.kind && used == name),
    )
}

/// Where it came from, on the wire.
fn origin_words(origin: &Origin) -> (&'static str, Option<String>) {
    match origin {
        Origin::Own => ("own", None),
        Origin::Account => ("account", None),
        Origin::Provided { by } => ("provided", Some(by.clone())),
        Origin::Unknown => ("unknown", None),
    }
}

/// How a verdict sorts: what can be set up, then what needs the user,
/// then what the other account turns on, then the rest.
fn verdict_rank(verdict: &str) -> u8 {
    match verdict {
        "copy" | "translate" => 0,
        "needs-you" => 1,
        "turn-on" => 2,
        _ => 3,
    }
}

/// What `target` (its items `to`, its tool `target_adapter`) lacks of the
/// items `from`, each with the target tool's verdict; and how many of
/// `from` it already has. What the conversation `used` comes first, then
/// what can be set up; `tool_name` names a tool in a note (`{tool}`).
pub(crate) fn offers(
    target: &juggler_core::model::Account,
    target_adapter: &dyn juggler_adapter::HarnessAdapter,
    from: &[Item],
    to: &[Item],
    used: Option<&[(ItemKind, String)]>,
    tool_name: &dyn Fn(&str) -> String,
) -> (Vec<OfferInfo>, u32) {
    let have: std::collections::HashSet<(ItemKind, String)> = to.iter().map(key).collect();
    let mut already_there: u32 = 0;
    let mut offers = Vec::new();
    // QA P24: ticks are keyed by an offer's id, so an item is offered
    // (or counted as there) once, whatever an inventory listed twice.
    let mut seen = std::collections::HashSet::new();
    for item in from {
        if !seen.insert(offer_id(item)) {
            continue;
        }
        if have.contains(&key(item)) {
            already_there += 1;
            continue;
        }
        let (verdict, note, get_it, can_copy) = match target_adapter.can_set_up(item) {
            SetUp::Copy => ("copy", None, None, false),
            SetUp::Translate(note) => ("translate", Some(note), None, false),
            // The user's step, in the target tool's words (AJ-E1543).
            SetUp::NeedsYou(note) => {
                let how = target_adapter.how_to_get(item, &target.state_dir);
                let get_it = (!how.steps.is_empty()).then(|| GetItInfo::of(&how, &target.label));
                ("needs-you", Some(note), get_it, false)
            }
            SetUp::Impossible(note) => ("impossible", Some(note), None, false),
            SetUp::TurnOn { get_it, can_copy } => (
                "turn-on",
                None,
                Some(GetItInfo::of(&get_it, &target.label)),
                can_copy,
            ),
        };
        let note = note.map(|note| note.replace("{tool}", &tool_name(&item.tool)));
        let used = used_of(item, used);
        let (origin, provided_by) = origin_words(&item.origin);
        offers.push(OfferInfo {
            id: offer_id(item),
            kind: kebab(&item.kind),
            name: item.name.clone(),
            scope: kebab(&item.scope),
            verdict: verdict.to_owned(),
            note,
            about: match &item.detail {
                juggler_adapter::inventory::Detail::Setting { about, .. } => about.clone(),
                _ => None,
            },
            // A project's own item that would apply to every project on
            // the target is offered, not pre-ticked (re-review of #132).
            suggested: matches!(verdict, "copy" | "translate")
                && used != Some(false)
                && !(item.scope == juggler_adapter::inventory::Scope::ProjectPrivate
                    && item.tool != target.harness.as_str()),
            used,
            origin: origin.to_owned(),
            provided_by,
            get_it,
            can_copy,
        });
    }
    offers.sort_by_key(|offer| {
        (
            offer.used != Some(true),
            verdict_rank(&offer.verdict),
            offer.kind.clone(),
            offer.name.clone(),
        )
    });
    (offers, already_there)
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
    let applied = apply_sides(state, &sides, picked, &for_a_switch)?;
    // What the target turns on itself is kept as a reminder until it has
    // it; a reminder that can't be kept never stops the switch.
    let _ = turn_ons::remember(state, &sides, picked);
    Ok(applied)
}

/// Sets up the picked offers of `sides.source` on `sides.target`, of the
/// kinds `wanted` takes only (the project sync copies servers and nothing
/// else), each recorded so it can be undone; see [`apply_impl`].
pub(crate) fn apply_sides(
    state: &Arc<AppState>,
    sides: &Sides,
    picked: &[String],
    wanted: &dyn Fn(ItemKind) -> bool,
) -> AppResult<Vec<AppliedInfo>> {
    let (from, to) = inventories(state, sides)?;
    let from: Vec<Item> = from.into_iter().filter(|item| wanted(item.kind)).collect();
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
                note: None,
            });
            continue;
        };
        if have.contains(&key(item)) {
            out.push(AppliedInfo {
                offer: id.clone(),
                record: None,
                message: format!("{} already has {}", sides.target.label, item.name),
                note: None,
            });
            continue;
        }
        // What comes with an account is turned on there, never copied,
        // whatever id is sent; only what may be the user's own can be.
        if let SetUp::TurnOn {
            can_copy: false, ..
        } = target_adapter.can_set_up(item)
        {
            out.push(AppliedInfo {
                offer: id.clone(),
                record: None,
                message: format!(
                    "{} isn't copied: turn it on for {} instead",
                    item.name, sides.target.label
                ),
                note: None,
            });
            continue;
        }
        match target_adapter.set_up(&sides.target.state_dir, sides.project.as_deref(), item) {
            Ok(applied) => {
                // One of a kind (an account's permissions) is named by its
                // kind alone: "the permissions", not "the permissions
                // permissions" (review of AJ-E1508).
                let what = if item.name == item.kind.words() {
                    item.name.clone()
                } else {
                    format!("{} {}", item.kind.words(), item.name)
                };
                let note = applied.note.clone();
                let record = record(
                    state,
                    Some(sides.target.id.to_string()),
                    None,
                    what.clone(),
                    applied,
                )?;
                out.push(AppliedInfo {
                    offer: id.clone(),
                    record: Some(record),
                    message: match &note {
                        Some(note) => format!("Set up the {what} ({note})"),
                        None => format!("Set up the {what}"),
                    },
                    note,
                });
            }
            Err(error) => out.push(AppliedInfo {
                offer: id.clone(),
                record: None,
                message: error.to_string(),
                note: None,
            }),
        }
    }
    Ok(out)
}

/// Every set-up still in place, newest first. Read-only.
///
/// # Errors
///
/// Storage errors.
pub fn list_impl(state: &Arc<AppState>) -> AppResult<Vec<SetUpInfo>> {
    let labels: std::collections::HashMap<String, String> = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .accounts()
            .list()?
            .into_iter()
            .map(|account| (account.id.to_string(), account.label))
            .collect()
    };
    let mut out: Vec<SetUpInfo> = load(state)
        .into_iter()
        .map(|record| SetUpInfo {
            place: match &record.project {
                Some(project) => project.clone(),
                None => record
                    .account
                    .as_ref()
                    .and_then(|account| labels.get(account))
                    .cloned()
                    .unwrap_or_else(|| "an account no longer here".to_owned()),
            },
            what: if record.what.is_empty() {
                record.applied.summary.clone()
            } else {
                format!("the {}", record.what)
            },
            at_ms: record.at_ms,
            record: record.id,
        })
        .collect();
    out.sort_by_key(|info| std::cmp::Reverse(info.at_ms));
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
    let _records = RECORDS
        .lock()
        // A panic elsewhere mid-save leaves the file as it was: carry on.
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    } else if matches!(record.applied.undo, Undo::Restore { .. }) {
        format!("Put back the {} as it was", record.what)
    } else {
        format!("Removed the {}", record.what)
    })
}

/// Stops keeping a set-up that can't be undone any more (what it made has
/// changed, or is gone). Nothing on disk changes: what it set up stays,
/// and so does the tool's backup, which may be the only copy of how its
/// settings were before.
///
/// # Errors
///
/// [`AppError::Refused`] when there's no such record; storage errors.
pub fn forget_impl(state: &Arc<AppState>, record_id: &str) -> AppResult<String> {
    let _records = RECORDS
        .lock()
        // A panic elsewhere mid-save leaves the file as it was: carry on.
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut records = load(state);
    let at = records
        .iter()
        .position(|record| record.id == record_id)
        .ok_or_else(|| AppError::Refused("there's nothing to forget".to_owned()))?;
    let record = records.remove(at);
    write(state, &records)?;
    let what = if record.what.is_empty() {
        record.applied.summary
    } else {
        format!("the {}", record.what)
    };
    Ok(format!("Forgot {what}. It stays as it is now."))
}

/// Undoes `how` straight away, for a change that couldn't be recorded.
pub(crate) fn undo_now(how: &Undo) -> AppResult<()> {
    undo(how)
}

fn undo(how: &Undo) -> AppResult<()> {
    match how {
        Undo::RemoveCreated { files, dirs } => {
            // A file the user has since replaced with a link is theirs now,
            // whatever the link points to.
            let changed: Vec<&PathBuf> = files
                .iter()
                .filter(|(path, sha)| {
                    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
                        || std::fs::read(path).is_ok_and(|bytes| {
                            juggler_adapter::handover::sha256_hex(&bytes) != *sha
                        })
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
        Undo::RemoveAppended { file, text, .. } => remove_appended(file, text),
        Undo::Command {
            argv,
            env,
            unset,
            cwd,
            unless_changed,
        } => {
            if let Some(entry) = unless_changed {
                match juggler_adapter::inventory::JsonEntry::fingerprint(&entry.file, &entry.keys) {
                    // Already removed by the user: nothing left to undo.
                    None => return Ok(()),
                    Some(now) if now != entry.sha256 => {
                        return Err(AppError::Refused(
                            "it has changed since it was set up, so it was left as it is"
                                .to_owned(),
                        ));
                    }
                    Some(_) => {}
                }
            }
            let (program, args) = argv
                .split_first()
                .ok_or_else(|| AppError::Refused("nothing to run".to_owned()))?;
            let mut command = std::process::Command::new(program);
            command.args(args).stdin(std::process::Stdio::null());
            if let Some(cwd) = cwd {
                command.current_dir(cwd);
            }
            for name in unset {
                command.env_remove(name);
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
        Undo::Restore {
            file,
            backup,
            sha256,
        } => restore(file, backup, sha256),
        Undo::RemoveSetting {
            file,
            format,
            entries,
        } => juggler_adapters::config_edit::remove(file, *format, entries)
            .map(|_| ())
            .map_err(|why| {
                AppError::Refused(format!(
                    "{}: {why}, so it was left as it is",
                    file.display()
                ))
            }),
    }
}

/// Takes out a block of text that was appended, if it's still there as it
/// was and nothing was added under it.
fn remove_appended(file: &Path, text: &str) -> AppResult<()> {
    let now = std::fs::read_to_string(file).map_err(|e| AppError::io("undo the set-up", &e))?;
    let Some(at) = now.find(text) else {
        return Err(AppError::Refused(format!(
            "{} doesn't have that change any more, so it was left as it is",
            file.display()
        )));
    };
    // Lines the user added under our table since belong to it: taking
    // the table out would leave them under the one before.
    let added_under = now[at + text.len()..]
        .lines()
        .take_while(|line| !line.trim_start().starts_with('['))
        .any(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'));
    if added_under {
        return Err(AppError::Refused(format!(
            "{} has settings added to it since, so it was left as it is",
            file.display()
        )));
    }
    let mut restored = now.clone();
    restored.replace_range(at..at + text.len(), "");
    juggler_adapter::inventory::rewrite_in_place(file, &restored)
        .map_err(|e| AppError::io("undo the set-up", &e))
}

/// Puts back a file that was replaced, while it's still a plain file with
/// exactly what was written (AJ-E1507). One replaced by a link since is left
/// alone: the write would go through the link to another file, the master
/// above all (review of AJ-E1507, H1). A hard link is safe: the write
/// replaces the name, it doesn't write through it.
fn restore(file: &Path, backup: &Path, sha256: &str) -> AppResult<()> {
    if std::fs::symlink_metadata(file).map_or(true, |meta| !meta.file_type().is_file()) {
        return Err(AppError::Refused(format!(
            "{} has been replaced since, so it was left as it is",
            file.display()
        )));
    }
    let now = std::fs::read(file).map_err(|e| AppError::io("undo the change", &e))?;
    if juggler_adapter::handover::sha256_hex(&now) != sha256 {
        return Err(AppError::Refused(format!(
            "{} has changed since, so it was left as it is",
            file.display()
        )));
    }
    let before =
        std::fs::read_to_string(backup).map_err(|e| AppError::io("undo the change", &e))?;
    juggler_adapter::inventory::rewrite_in_place(file, &before)
        .map_err(|e| AppError::io("undo the change", &e))?;
    let _ = std::fs::remove_file(backup);
    Ok(())
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

/// Records a set-up so it can be undone from Settings: made on `account`,
/// or to `project`'s own files. Returns the record's id.
pub(crate) fn record(
    state: &Arc<AppState>,
    account: Option<String>,
    project: Option<String>,
    what: String,
    applied: Applied,
) -> AppResult<String> {
    let record = Record {
        id: format!("setup-{}", crate::commands::unique_suffix()),
        account,
        at_ms: juggler_core::model::Timestamp::now().millis(),
        applied,
        what,
        project,
    };
    save(state, &record)?;
    Ok(record.id)
}

fn save(state: &Arc<AppState>, record: &Record) -> AppResult<()> {
    let _records = RECORDS
        .lock()
        // A panic elsewhere mid-save leaves the file as it was: carry on.
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut records = load(state);
    records.push(record.clone());
    write(state, &records)
}

fn write(state: &Arc<AppState>, records: &[Record]) -> AppResult<()> {
    let text = serde_json::to_string_pretty(records).unwrap_or_default();
    atomic_write(&state.data_dir.join(RECORD), &text)
}

/// Every set-up still in place (Settings), newest first.
///
/// # Errors
///
/// As [`list_impl`].
#[tauri::command]
pub async fn switch_setup_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<SetUpInfo>> {
    crate::ipc::blocking(state.inner(), "list the set-ups", list_impl).await
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

/// Stops keeping a set-up (Settings), leaving everything as it is.
///
/// # Errors
///
/// As [`forget_impl`].
#[tauri::command]
pub async fn switch_setup_forget(
    state: tauri::State<'_, Arc<AppState>>,
    record: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "forget the set-up", move |state| {
        forget_impl(state, &record)
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

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::inventory::{Detail, Scope};

    /// QA P24: an item an inventory lists twice is offered once, so its
    /// tick (keyed by the offer's id) is one tick.
    #[test]
    fn an_item_listed_twice_is_offered_once() {
        let skill = |name: &str| Item {
            tool: "claude-code".to_owned(),
            kind: ItemKind::Skill,
            name: name.to_owned(),
            scope: Scope::User,
            detail: Detail::FromTheAccount {
                source: None,
                files: None,
                requires: Vec::new(),
            },
            origin: juggler_adapter::inventory::Origin::Unknown,
        };
        let target = juggler_core::model::Account {
            id: juggler_core::model::AccountId::new("codex-1"),
            harness: juggler_core::model::HarnessId::new("fake"),
            label: "Codex".into(),
            plan: None,
            state_dir: std::path::PathBuf::from("/nonexistent"),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        };
        let adapter = juggler_adapters::fake::FakeHarnessAdapter::new("/bin/true");
        let from = [skill("pdf"), skill("pdf"), skill("docs")];
        let (offers, there) = offers(
            &target,
            &adapter,
            &from,
            &[skill("docs"), skill("docs")],
            None,
            &str::to_owned,
        );
        assert_eq!(offers.len(), 1, "{offers:?}");
        assert_eq!(there, 1);
    }
}
