//! ADR-0031 (D-93): **Move to account**, a switch by move. The adapter
//! plans ([`juggler_adapter::moving::InPlaceMove`]); the engine applies
//! (`juggler_handover::moving`); this module does the process side, the
//! same for every tool and app:
//!
//! - **The offer** ([`offer`]): both sides resolved (the account's tool
//!   folder and, when the conversation lives in a desktop app profile,
//!   that profile and the target's), the adapter's read-only plan, the
//!   engine's preflight, and busy work in either profile: what the switch
//!   screen shows before anything happens.
//! - **The move** ([`execute`]): one move of these folders at a time
//!   (a lock every AI Juggler on the Mac shares), keep alive and the
//!   automation watch held off, both profiles quit by their exact process
//!   (pid and start time; never by the app's name, never forced), the plan
//!   made again after they quit and refused if the family changed, the
//!   engine's backup, apply and checks, then both profiles started again
//!   with their own folders, and the conversation opened in the target by
//!   a link sent to the target profile's own process. A failure before the
//!   apps start again is undone at once; nothing is left half moved.
//! - **Undo** ([`undo`]): the same stop, the engine's Undo (backups checked
//!   first, work done since kept aside), the same start, opened in the
//!   source.
//!
//! What it reports keeps four outcomes apart: the conversation's files,
//! the app's list of it, that it opened in the target profile, and a
//! reply from the model on the target account, which only the user can
//! check (the move never sends a prompt).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::moving::{MemberSide, MovePlan, MoveRequest, MoveSide, NotesChoice};
use juggler_core::model::Account;
use juggler_handover::moving::{Lock, MoveError, Phase, Recovery, Stopped, Transaction};

use crate::error::{AppError, AppResult};
use crate::features::places::book::KnownPlace;
use crate::state::AppState;

/// How long a profile is given to quit.
const QUIT_WAIT: Duration = Duration::from_secs(30);
/// How long a started profile is given to come up.
const START_WAIT: Duration = Duration::from_secs(30);
/// How long the target app is given to settle after the link (it adds
/// its bookkeeping to the history when it opens it).
const SETTLE: Duration = Duration::from_secs(6);
/// Between looks.
const EVERY: Duration = Duration::from_millis(250);

/// One conversation of the family, as the switch screen lists it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveMemberInfo {
    /// The app's title, else the conversation's id.
    pub title: String,
    /// Archived in the app (it stays archived).
    pub archived: bool,
    /// It moves (`moves`), or it is already in the target and is linked
    /// to the moved one (`relinked`).
    pub side: String,
    /// Its working folder is gone (its history is kept).
    pub folder_missing: bool,
}

/// Whether and how a conversation can be moved, for the switch screen.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveOfferInfo {
    /// The tool can move between these two accounts at all (both sides of
    /// a kind it proved): Move is shown.
    pub applies: bool,
    /// It can move now: Move is the default.
    pub offered: bool,
    /// Why not, in words (empty when offered).
    pub why_not: Vec<String>,
    /// The family, the moved conversation first.
    pub members: Vec<MoveMemberInfo>,
    /// The profiles that quit and start again ("Claude app · Work").
    pub restarts: Vec<String>,
    /// What the user should know (a kept branch, a missing folder).
    pub notes: Vec<String>,
    /// The project's notes differ in the two accounts (the user may take
    /// the conversation's).
    pub notes_conflict: Option<String>,
    /// Output folders that stay where old messages name them.
    pub kept_outputs: usize,
    /// The family's bytes (moved, not copied: no extra room).
    #[ts(type = "number")]
    pub bytes: u64,
    /// Desktop app profiles (true) or tool folders in a terminal (false).
    pub app: bool,
}

/// One of a move's four outcomes.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveOutcomeInfo {
    /// `verified`, `failed`, or `pending` (not something AI Juggler can
    /// check by itself).
    pub state: String,
    /// In words.
    pub detail: String,
}

/// What a move did.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveDoneInfo {
    /// The move's id (for Undo).
    pub move_id: String,
    /// The conversation's files: every byte where the plan put it, before
    /// and after the apps started again.
    pub payload: MoveOutcomeInfo,
    /// The app's list: every member listed under the target account,
    /// linked as it was, no record left on the source.
    pub catalog: MoveOutcomeInfo,
    /// Opened in the target profile, by its own process.
    pub opened: MoveOutcomeInfo,
    /// A reply from the model on the target account: only the user can
    /// check it (a move never sends a prompt).
    pub model: MoveOutcomeInfo,
    /// What the user should know.
    pub notes: Vec<String>,
}

/// A move AI Juggler knows of, for Undo and for one left part way.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveRecordInfo {
    /// Its id.
    pub move_id: String,
    /// The moved conversation's id in its tool.
    pub conversation: String,
    /// `committed`, `undone`, or `interrupted`.
    pub state: String,
    /// When (epoch ms).
    #[ts(type = "number")]
    pub at_ms: i64,
    /// For an interrupted one: what recovery found, in words.
    pub detail: Option<String>,
    /// What it keeps where it was on purpose (output folders old messages
    /// name by their full path) and its own folder (backups, kept
    /// branches, work kept aside by Undo), `~`-shortened: cleanup keeps
    /// each while the move is listed.
    pub kept: Vec<String>,
}

/// The two sides of a move, resolved.
struct Sides {
    request: MoveRequest,
    tool: String,
    adapter: juggler_adapter::adapter::SharedAdapter,
    /// The app profiles that quit and start again: (place id, place).
    source_place: Option<(String, KnownPlace)>,
    target_place: Option<(String, KnownPlace)>,
    /// The two accounts' names.
    source_label: String,
    target_label: String,
    /// The source conversation is running in a session AI Juggler sees.
    source_live: bool,
    /// Both accounts' ids (their sessions outside the two profiles must be
    /// quiet: they write the same folders).
    accounts: Vec<String>,
}

/// The process side of a move (a seam: the tests script it).
pub trait MoveSystem: Send + Sync {
    /// The profile's main process now (pid, start).
    fn running(&self, state: &Arc<AppState>, place_id: &str) -> Option<(u32, Option<i64>)>;
    /// Asks `pid` to quit (as a normal quit).
    ///
    /// # Errors
    ///
    /// The system's reason.
    fn quit(&self, pid: u32) -> Result<(), String>;
    /// Whether `pid` is still the process that started at `start`.
    fn alive(&self, pid: u32, start: Option<i64>) -> bool;
    /// Starts the profile on its own folders.
    ///
    /// # Errors
    ///
    /// Why it didn't start.
    fn start(&self, state: &Arc<AppState>, place: &KnownPlace) -> Result<(), String>;
    /// Sends `link` to process `pid` alone.
    ///
    /// # Errors
    ///
    /// Why it couldn't.
    fn send(&self, pid: u32, link: &str) -> Result<(), String>;
    /// Sessions working (a turn running) in the profile whose process is
    /// `pid`.
    fn working(&self, state: &Arc<AppState>, pid: u32) -> usize;
    /// Sessions open on any of `accounts` that run outside the app
    /// processes `hosts` (a terminal on the same folder): another writer.
    fn open_elsewhere(&self, state: &Arc<AppState>, accounts: &[String], hosts: &[u32]) -> usize;
    /// Waits `for_` (a test doesn't).
    fn settle(&self, for_: Duration) {
        std::thread::sleep(for_);
    }
    /// How long a profile is given to quit.
    fn quit_wait(&self) -> Duration {
        QUIT_WAIT
    }
}

/// The real system.
pub struct Real;

impl MoveSystem for Real {
    fn running(&self, state: &Arc<AppState>, place_id: &str) -> Option<(u32, Option<i64>)> {
        crate::features::places::running_now(state, place_id)
    }

    fn quit(&self, pid: u32) -> Result<(), String> {
        use crate::features::places::start::Starter as _;
        crate::features::places::start::System.quit(pid)
    }

    fn alive(&self, pid: u32, start: Option<i64>) -> bool {
        use crate::features::places::start::Starter as _;
        crate::features::places::start::System.alive(pid, start)
    }

    fn start(&self, state: &Arc<AppState>, place: &KnownPlace) -> Result<(), String> {
        let hosts = crate::features::conversations::places::HostRegistry::of(&state.registry);
        crate::features::places::start_place(
            state,
            place,
            &hosts,
            &crate::features::places::start::System,
            false,
        )
    }

    fn send(&self, pid: u32, link: &str) -> Result<(), String> {
        juggler_platform::apps::send_link(pid, link).map_err(|error| error.to_string())
    }

    fn working(&self, state: &Arc<AppState>, pid: u32) -> usize {
        crate::features::places::working_in(state, pid)
    }

    fn open_elsewhere(&self, state: &Arc<AppState>, accounts: &[String], hosts: &[u32]) -> usize {
        state
            .conversations
            .live_model()
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .account_id
                    .as_ref()
                    .is_some_and(|a| accounts.contains(a))
            })
            .filter(|entry| {
                !entry.places.iter().any(|place| {
                    place
                        .host
                        .as_ref()
                        .is_some_and(|host| hosts.contains(&host.pid))
                })
            })
            .count()
    }
}

fn moves_dir(state: &Arc<AppState>) -> PathBuf {
    state.data_dir.join(juggler_handover::moving::FOLDER)
}

fn refused(why: impl Into<String>) -> AppError {
    AppError::Refused(why.into())
}

#[allow(clippy::needless_pass_by_value, reason = "used as `map_err(engine)`")]
fn engine(error: MoveError) -> AppError {
    refused(error.to_string())
}

/// The installed version of `account`'s tool (Claude Code's), as AI
/// Juggler's tool check found it.
fn tool_version(state: &Arc<AppState>, account: &Account) -> Option<String> {
    crate::features::providers::warm_provider_versions(state);
    state
        .provider_versions
        .lock()
        .ok()
        .and_then(|versions| versions.get(account.harness.as_str()).cloned())
}

/// Resolves both sides of moving `session_id` to `target_profile_id`.
fn sides(state: &Arc<AppState>, session_id: &str, target_profile_id: &str) -> AppResult<Sides> {
    let (_profile_b, account_b) =
        crate::commands::find_profile_with_account_arc(state, target_profile_id)?;
    let (source, account_a) = super::source::handover_source(state, session_id)?;
    let conversation = source
        .transcript_ref
        .as_ref()
        .and_then(|reference| reference.native_id.clone())
        .ok_or_else(|| refused("AI Juggler couldn't tell which conversation this is"))?;
    if account_a.harness != account_b.harness {
        return Err(refused(
            "a move stays within one tool; Copy carries it to another",
        ));
    }
    let adapter = state
        .registry
        .get(&account_a.harness)
        .ok_or_else(|| AppError::UnknownHarness(account_a.harness.as_str().to_owned()))?;
    // The conversation lives in a desktop app profile when that profile
    // keeps a record of it; then the target's profile is the other side.
    let source_place = crate::features::places::handoff::profile_for(state, account_a.id.as_str())
        .filter(|(_, place)| {
            let (Some(dir), Some(family)) = (place.instance_dir.as_deref(), adapter.host_family())
            else {
                return false;
            };
            family
                .conversations(dir)
                .iter()
                .any(|r| r.id == conversation)
        });
    let target_place = source_place
        .as_ref()
        .and_then(|_| crate::features::places::handoff::profile_for(state, account_b.id.as_str()));
    let side = |account: &Account, place: Option<&(String, KnownPlace)>| MoveSide {
        tool_dir: account.state_dir.clone(),
        host_dir: place.and_then(|(_, p)| p.instance_dir.clone()),
        version: match place {
            Some((_, p)) => juggler_platform::apps::bundle_version(&p.bundle),
            None => tool_version(state, account),
        },
    };
    let request = MoveRequest {
        conversation,
        source: side(&account_a, source_place.as_ref()),
        target: side(&account_b, target_place.as_ref()),
        notes: NotesChoice::KeepTarget,
    };
    Ok(Sides {
        request,
        tool: account_a.harness.as_str().to_owned(),
        adapter,
        source_live: matches!(source.status.as_str(), "running" | "starting"),
        source_place,
        target_place,
        source_label: account_a.label.clone(),
        target_label: account_b.label.clone(),
        accounts: vec![
            account_a.id.as_str().to_owned(),
            account_b.id.as_str().to_owned(),
        ],
    })
}

fn place_name(state: &Arc<AppState>, place: &KnownPlace) -> String {
    crate::features::places::named(state, place)
}

/// Why the move can't happen now because something is working: a turn
/// running in either profile, or (terminal folders) the conversation
/// open in a session.
fn busy(state: &Arc<AppState>, sides: &Sides, system: &dyn MoveSystem) -> Vec<String> {
    let places: Vec<(String, KnownPlace)> = [&sides.source_place, &sides.target_place]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let mut out = quiet(state, &places, &sides.accounts, system);
    if sides.source_place.is_none() && sides.source_live && out.is_empty() {
        out.push("the conversation is open in a session: end it first, then move it".to_owned());
    }
    out
}

/// Why the folders of `places` and `accounts` aren't quiet enough to
/// change: a turn running in either profile, or a session of either
/// account open outside them (a terminal writes the same folder).
fn quiet(
    state: &Arc<AppState>,
    places: &[(String, KnownPlace)],
    accounts: &[String],
    system: &dyn MoveSystem,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut hosts = Vec::new();
    for (id, place) in places {
        if let Some((pid, _)) = system.running(state, id) {
            hosts.push(pid);
            let working = system.working(state, pid);
            if working > 0 {
                out.push(format!(
                    "{} has {working} session{} working: a move restarts it, which would stop {}. Let {} finish first",
                    place_name(state, place),
                    if working == 1 { "" } else { "s" },
                    if working == 1 { "it" } else { "them" },
                    if working == 1 { "it" } else { "them" },
                ));
            }
        }
    }
    let elsewhere = system.open_elsewhere(state, accounts, &hosts);
    if elsewhere > 0 {
        out.push(format!(
            "{elsewhere} session{} of these accounts {} open outside the app (in a terminal, say), and {} the same folders: end {} first",
            if elsewhere == 1 { "" } else { "s" },
            if elsewhere == 1 { "is" } else { "are" },
            if elsewhere == 1 { "writes" } else { "write" },
            if elsewhere == 1 { "it" } else { "them" },
        ));
    }
    out
}

/// Whether every profile in `stopped` is still closed (the user may start
/// one from the Dock while the move runs).
fn still_down(
    state: &Arc<AppState>,
    stopped: &[StoppedPlace],
    system: &dyn MoveSystem,
) -> Result<(), String> {
    match stopped
        .iter()
        .find(|p| system.running(state, &p.id).is_some())
    {
        Some(place) => Err(format!(
            "{} was started while the move ran",
            place_name(state, &place.place)
        )),
        None => Ok(()),
    }
}

/// Keeps keep alive off `stopped` (a move left them for Undo to finish).
fn pause(state: &Arc<AppState>, stopped: &[StoppedPlace]) {
    let _ = crate::features::places::with_book(state, |book| {
        for place in stopped {
            if let Some(known) = book.places.get_mut(&place.id) {
                known.paused = Some(
                    "a move stopped part way: undo it from the switch screen first".to_owned(),
                );
            }
        }
    });
}

fn plan_of(sides: &Sides) -> AppResult<MovePlan> {
    let mover = sides
        .adapter
        .in_place_move()
        .ok_or_else(|| refused("this tool's conversations travel as a copy"))?;
    mover
        .plan(&sides.request)
        .map_err(|error| refused(error.to_string()))
}

/// Whether the plan's refusals are about the kind of sides (Move doesn't
/// apply here at all) rather than about now.
fn applies(plan: &MovePlan) -> bool {
    !plan
        .refusals
        .iter()
        .any(|r| r.contains("isn't proven yet") || r.contains("same Claude Code folder"))
}

/// The switch screen's Move offer for `session_id` to `target_profile_id`.
/// Reads only.
///
/// # Errors
///
/// The session or account can't be resolved.
pub fn offer(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<MoveOfferInfo> {
    Ok(offer_with(state, session_id, target_profile_id, &Real))
}

pub(crate) fn offer_with(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    system: &dyn MoveSystem,
) -> MoveOfferInfo {
    let Ok(sides) = sides(state, session_id, target_profile_id) else {
        return MoveOfferInfo::default();
    };
    if sides.adapter.in_place_move().is_none() {
        return MoveOfferInfo::default();
    }
    let plan = match plan_of(&sides) {
        Ok(plan) => plan,
        Err(error) => {
            return MoveOfferInfo {
                applies: true,
                why_not: vec![error.to_string()],
                ..MoveOfferInfo::default()
            };
        }
    };
    let mut why_not = juggler_handover::moving::preflight(&plan, &moves_dir(state));
    why_not.extend(busy(state, &sides, system));
    let restarts = [&sides.source_place, &sides.target_place]
        .into_iter()
        .flatten()
        .map(|(_, place)| place_name(state, place))
        .collect();
    MoveOfferInfo {
        applies: applies(&plan),
        offered: why_not.is_empty(),
        why_not,
        members: plan
            .members
            .iter()
            .map(|m| MoveMemberInfo {
                title: m.title.clone().unwrap_or_else(|| m.conversation.clone()),
                archived: m.archived,
                side: match m.side {
                    MemberSide::Source => "moves",
                    MemberSide::Target => "relinked",
                }
                .to_owned(),
                folder_missing: m.folder_missing,
            })
            .collect(),
        restarts,
        notes: plan.notes.clone(),
        notes_conflict: plan.notes_conflict.clone(),
        kept_outputs: plan.retained.len(),
        bytes: plan.members.iter().map(|m| m.bytes).sum(),
        app: sides.source_place.is_some(),
    }
}

/// The profiles a move stopped: (place id, place, pid, start, it was
/// running before).
struct StoppedPlace {
    id: String,
    place: KnownPlace,
    was_running: bool,
}

/// Quits each of `places` by its exact process and waits for it to end.
/// On a refusal, the ones already quit are started again.
fn stop_places(
    state: &Arc<AppState>,
    places: &[(String, KnownPlace)],
    system: &dyn MoveSystem,
) -> AppResult<Vec<StoppedPlace>> {
    let mut stopped: Vec<StoppedPlace> = Vec::new();
    for (id, place) in places {
        let Some((pid, start)) = system.running(state, id) else {
            stopped.push(StoppedPlace {
                id: id.clone(),
                place: place.clone(),
                was_running: false,
            });
            continue;
        };
        // The pid is this profile's process still (not one the system
        // gave another program since): checked by its start time.
        if start.is_none() || !system.alive(pid, start) {
            start_again(state, &stopped, system);
            return Err(refused(format!(
                "AI Juggler couldn't be sure which process is {}, so nothing was quit or moved",
                place_name(state, place)
            )));
        }
        if let Err(why) = system.quit(pid) {
            start_again(state, &stopped, system);
            return Err(refused(format!(
                "couldn't quit {}: {why}",
                place_name(state, place)
            )));
        }
        let wait = system.quit_wait();
        let until = Instant::now() + wait;
        while system.alive(pid, start) {
            if Instant::now() >= until {
                start_again(state, &stopped, system);
                return Err(refused(format!(
                    "{} didn't quit within {} seconds, so nothing was moved. It may be asking something: look at its window",
                    place_name(state, place),
                    wait.as_secs().max(1)
                )));
            }
            std::thread::sleep(EVERY);
        }
        stopped.push(StoppedPlace {
            id: id.clone(),
            place: place.clone(),
            was_running: true,
        });
    }
    Ok(stopped)
}

/// Starts again the profiles that were running.
fn start_again(state: &Arc<AppState>, stopped: &[StoppedPlace], system: &dyn MoveSystem) {
    for place in stopped.iter().filter(|p| p.was_running) {
        if let Err(why) = system.start(state, &place.place) {
            tracing::warn!(target: "juggler::move", "a profile didn't start again after a move: {why}");
        }
    }
}

/// Starts `place` (when it isn't running) and sends `link` to its own
/// process. Returns that process.
fn open_in(
    state: &Arc<AppState>,
    id: &str,
    place: &KnownPlace,
    link: &str,
    system: &dyn MoveSystem,
) -> Result<u32, String> {
    if system.running(state, id).is_none() {
        system.start(state, place)?;
    }
    let until = Instant::now() + START_WAIT;
    let pid = loop {
        if let Some((pid, _)) = system.running(state, id) {
            break pid;
        }
        if Instant::now() >= until {
            return Err(format!(
                "{} didn't finish starting",
                place_name(state, place)
            ));
        }
        std::thread::sleep(EVERY);
    };
    system.send(pid, link)?;
    Ok(pid)
}

fn outcome(pass: bool, ok: &str, failed: String) -> MoveOutcomeInfo {
    MoveOutcomeInfo {
        state: if pass { "verified" } else { "failed" }.to_owned(),
        detail: if pass { ok.to_owned() } else { failed },
    }
}

/// A new move's id: when, and a random tail.
fn new_id(now_ms: i64) -> String {
    format!("{now_ms}-{:08x}", crate::ids::unique_suffix() & 0xffff_ffff)
}

/// Moves `session_id`'s conversation and its family to
/// `target_profile_id`'s account.
///
/// # Errors
///
/// Refused before anything changed (with why), or a failure after which
/// the move was undone (with why).
pub fn execute(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    notes: NotesChoice,
) -> AppResult<MoveDoneInfo> {
    execute_with(state, session_id, target_profile_id, notes, &Real)
}

pub(crate) fn execute_with(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    notes: NotesChoice,
    system: &dyn MoveSystem,
) -> AppResult<MoveDoneInfo> {
    let mut sides = sides(state, session_id, target_profile_id)?;
    sides.request.notes = notes;
    run(state, &sides, system)
}

/// The move of resolved `sides`.
#[allow(
    clippy::too_many_lines,
    reason = "the move, step by step, in one place"
)]
fn run(state: &Arc<AppState>, sides: &Sides, system: &dyn MoveSystem) -> AppResult<MoveDoneInfo> {
    let plan = plan_of(sides)?;
    let folder = moves_dir(state);
    let mut why_not = juggler_handover::moving::preflight(&plan, &folder);
    why_not.extend(busy(state, sides, system));
    if !why_not.is_empty() {
        return Err(refused(why_not.join("; ")));
    }
    let places: Vec<(String, KnownPlace)> = [&sides.source_place, &sides.target_place]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let _lock = Lock::take(&plan.roots).map_err(engine)?;
    let ids: Vec<String> = places.iter().map(|(id, _)| id.clone()).collect();
    let _hold = crate::features::places::MoveHold::take(state, &ids)
        .ok_or_else(|| refused("another move of these profiles is running"))?;
    let now = juggler_core::model::Timestamp::now().millis();
    let previous = Transaction::all(&folder)
        .into_iter()
        .rev()
        .find(|txn| {
            txn.manifest.phase != Phase::Undone
                && txn.manifest.request.conversation == sides.request.conversation
        })
        .map(|txn| txn.manifest.id.clone());
    let mut txn = Transaction::begin(
        &folder,
        &new_id(now),
        &sides.tool,
        sides.request.clone(),
        plan,
        previous,
        now,
    )
    .map_err(engine)?;
    txn.remember_places(ids, sides.accounts.clone())
        .map_err(engine)?;

    // The single-writer phase.
    let stopped_places = stop_places(state, &places, system)?;
    let stopped = Stopped::checked();
    let fail = |txn: &mut Transaction, why: String| -> AppError {
        // Never undo under a running app: it would write over the restore.
        if let Err(started) = still_down(state, &stopped_places, system) {
            let _ = txn.stop_with(&format!("{why}; {started}"));
            return refused(format!(
                "{why}, and {started}, so AI Juggler left both accounts as they are now. Quit it, then Undo the move here"
            ));
        }
        match txn.undo(&stopped, juggler_core::model::Timestamp::now().millis()) {
            Ok(_) => {
                start_again(state, &stopped_places, system);
                refused(format!(
                    "{why}. Nothing was moved: both accounts are as they were"
                ))
            }
            Err(error) => {
                // Left closed: an app writing into a half-undone folder
                // would make the next Undo reconcile its writes.
                let _ = txn.stop_with(&format!("{why}; undo: {error}"));
                pause(state, &stopped_places);
                refused(format!(
                    "{why}. Undoing it didn't finish ({error}); both profiles stay closed, the move's journal and backups are kept, and Undo the move here finishes it"
                ))
            }
        }
    };
    // The apps write their records when they quit: plan again, refuse a
    // changed family.
    let again = plan_of(sides).and_then(|plan| txn.replan(plan).map_err(engine));
    if let Err(error) = again {
        return Err(fail(&mut txn, error.to_string()));
    }
    let plan = txn.manifest.plan.clone();
    let mover = sides
        .adapter
        .in_place_move()
        .ok_or_else(|| refused("no mover"))?;
    if let Err(started) = still_down(state, &stopped_places, system) {
        return Err(fail(&mut txn, started));
    }
    if let Err(error) = txn.back_up(&stopped) {
        return Err(fail(&mut txn, error.to_string()));
    }
    if let Err(started) = still_down(state, &stopped_places, system) {
        return Err(fail(&mut txn, started));
    }
    if let Err(error) = txn.apply(&stopped, 0) {
        return Err(fail(&mut txn, error.to_string()));
    }
    let cold = match txn.verify_cold(mover.verify(&sides.request, &plan)) {
        Ok(report) => report,
        Err(error) => return Err(fail(&mut txn, error.to_string())),
    };
    if !cold.passed() {
        let why = format!(
            "a check before reopening failed: {}",
            cold.failures().join("; ")
        );
        return Err(fail(&mut txn, why));
    }

    // Both profiles again, each on its own folders; the conversation
    // opened in the target by its own process.
    let source_id = sides.source_place.as_ref().map(|(id, _)| id);
    if let Some(source) = stopped_places.iter().find(|p| Some(&p.id) == source_id)
        && source.was_running
        && let Err(why) = system.start(state, &source.place)
    {
        tracing::warn!(target: "juggler::move", "the source profile didn't start again: {why}");
    }
    let link = mover.resume_link(&plan.resume);
    let opened = match (&sides.target_place, &link) {
        (Some((id, place)), Some(link)) => {
            Some(open_in(state, id, place, link, system).map(|pid| (pid, place_name(state, place))))
        }
        _ => None,
    };
    system.settle(SETTLE);
    // The move happened: a check that can't be written is a failed check,
    // never an error that hides the move and its Undo.
    let after = txn
        .verify_opened(
            &|line| mover.appended(line),
            mover.verify(&sides.request, &plan),
        )
        .unwrap_or_else(|error| juggler_handover::moving::Report {
            checks: vec![juggler_adapter::moving::MoveCheck {
                what: format!("the checks after reopening couldn't be recorded: {error}"),
                pass: false,
            }],
            ..juggler_handover::moving::Report::default()
        });
    let payload_failures: Vec<String> = after
        .checks
        .iter()
        .filter(|c| !c.pass && c.what.contains("starts with"))
        .map(|c| c.what.clone())
        .collect();
    let catalog_failures: Vec<String> = after
        .checks
        .iter()
        .filter(|c| !c.pass && !c.what.contains("starts with"))
        .map(|c| c.what.clone())
        .collect();
    let target_name = sides
        .target_place
        .as_ref()
        .map_or_else(|| sides.target_label.clone(), |(_, p)| place_name(state, p));
    let members = plan.members.len();
    let many = |n: usize, one: &str, more: &str| format!("{n} {}", if n == 1 { one } else { more });
    let mut notes = plan.notes.clone();
    if after.added.turns > 0 {
        notes.push(format!(
            "{} appeared in the moved history after it opened; AI Juggler sent none",
            many(after.added.turns, "new turn", "new turns")
        ));
    }
    if !plan.retained.is_empty() {
        notes.push(format!(
            "{} of tool output stay with {}, where old messages name them; cleanup keeps them",
            many(plan.retained.len(), "folder", "folders"),
            sides.source_label
        ));
    }
    Ok(MoveDoneInfo {
        move_id: txn.manifest.id.clone(),
        payload: outcome(
            payload_failures.is_empty(),
            &format!(
                "every byte of {} is in {}, checked before and after it opened",
                many(members, "conversation", "conversations"),
                sides.target_label
            ),
            format!("changed after reopening: {}", payload_failures.join("; ")),
        ),
        catalog: outcome(
            catalog_failures.is_empty(),
            &format!("{target_name} lists the conversation and its family as they were nested"),
            format!("not as planned: {}", catalog_failures.join("; ")),
        ),
        opened: match opened {
            Some(Ok((pid, name))) => MoveOutcomeInfo {
                state: "verified".into(),
                detail: format!("opened in {name} (process {pid}), signed in as that account"),
            },
            Some(Err(why)) => MoveOutcomeInfo {
                state: "failed".into(),
                detail: format!("moved, but not opened: {why}. Open it from the session list"),
            },
            None => MoveOutcomeInfo {
                state: "pending".into(),
                detail: format!("resume it in a terminal on {}", sides.target_label),
            },
        },
        model: MoveOutcomeInfo {
            state: "pending".into(),
            detail: format!(
                "send a message to check it answers on {}; AI Juggler never sends one for you",
                sides.target_label
            ),
        },
        notes,
    })
}

/// Undoes move `move_id`: both profiles quit, the engine's Undo (backups
/// checked first, the work done since kept aside), both started again, the
/// conversation opened where it was. Returns where the work done since
/// was kept.
///
/// # Errors
///
/// A backup doesn't match (nothing changed), a profile wouldn't quit, or
/// the restore failed (the journal says how far).
pub fn undo(state: &Arc<AppState>, move_id: &str) -> AppResult<String> {
    undo_with(state, move_id, &Real)
}

pub(crate) fn undo_with(
    state: &Arc<AppState>,
    move_id: &str,
    system: &dyn MoveSystem,
) -> AppResult<String> {
    if move_id.is_empty()
        || !move_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(refused("that isn't a move's id"));
    }
    let moves = moves_dir(state);
    let mut txn = Transaction::open(&moves.join(move_id)).map_err(engine)?;
    // A later move (in any state but undone) that touches the same files
    // owns them now: undoing this one would put old copies over it.
    let later = Transaction::all(&moves).into_iter().any(|other| {
        other.manifest.id != txn.manifest.id
            && other.manifest.created_ms >= txn.manifest.created_ms
            && other.manifest.phase != Phase::Undone
            && other.overlaps(&txn)
    });
    if later {
        return Err(refused(
            "these conversations were moved again since; undo the later move first, or move them back",
        ));
    }
    // Every profile it stopped must still be one AI Juggler knows, to stop
    // it again: one it can't name might be running and writing.
    let mut places: Vec<(String, KnownPlace)> = Vec::new();
    for id in &txn.manifest.places {
        let known = crate::features::places::with_book(state, |book| book.places.get(id).cloned())
            .ok()
            .flatten()
            .ok_or_else(|| {
                refused("an app profile of this move isn't one AI Juggler knows any more; add it again (Settings › Desktop apps), then Undo")
            })?;
        places.push((id.clone(), known));
    }
    let _lock = Lock::take(&txn.manifest.plan.roots).map_err(engine)?;
    let _hold = crate::features::places::MoveHold::take(state, &txn.manifest.places)
        .ok_or_else(|| refused("another move of these profiles is running"))?;
    let why_not = quiet(state, &places, &txn.manifest.accounts, system);
    if !why_not.is_empty() {
        return Err(refused(why_not.join("; ")));
    }
    let stopped_places = stop_places(state, &places, system)?;
    if let Err(started) = still_down(state, &stopped_places, system) {
        start_again(state, &stopped_places, system);
        return Err(refused(format!("{started}, so nothing was undone")));
    }
    let result = txn.undo(
        &Stopped::checked(),
        juggler_core::model::Timestamp::now().millis(),
    );
    let kept = match result {
        Ok(kept) => {
            start_again(state, &stopped_places, system);
            kept
        }
        Err(error) => {
            // Left closed: Undo again finishes it on a quiet folder.
            pause(state, &stopped_places);
            return Err(refused(format!(
                "{error}. Both profiles stay closed; Undo the move again to finish"
            )));
        }
    };
    // Keep alive, paused by a move left part way, watches again.
    let _ = crate::features::places::with_book(state, |book| {
        for place in &stopped_places {
            if let Some(known) = book.places.get_mut(&place.id)
                && known
                    .paused
                    .as_deref()
                    .is_some_and(|why| why.starts_with("a move stopped"))
            {
                known.paused = None;
            }
        }
    });
    // Opened where it was.
    let adapter = state.registry.get(&juggler_core::model::HarnessId::new(
        txn.manifest.tool.as_str(),
    ));
    let link = adapter
        .as_ref()
        .and_then(|adapter| adapter.in_place_move())
        .and_then(|mover| mover.resume_link(&txn.manifest.request.conversation));
    if let (Some((id, place)), Some(link)) = (places.first(), link) {
        let _ = open_in(state, id, place, &link, system);
    }
    Ok(format!(
        "Undone: the conversation is back where it was. Anything done since the move is kept in {}",
        kept.display()
    ))
}

/// Every move AI Juggler knows of, newest first, with any left part way
/// read back from its journal.
#[must_use]
pub fn moves(state: &Arc<AppState>) -> Vec<MoveRecordInfo> {
    let mut out: Vec<MoveRecordInfo> = Transaction::all(&moves_dir(state))
        .into_iter()
        .map(|txn| {
            let (label, detail) = match txn.recover() {
                Ok(Recovery::Settled(Phase::Committed)) => ("committed", None),
                Ok(Recovery::Settled(Phase::Undone)) => ("undone", None),
                Ok(Recovery::Settled(phase)) if phase < Phase::BackedUp => ("undone", None),
                Ok(Recovery::Settled(_)) => (
                    "interrupted",
                    Some("it stopped after its changes and before its last check: Undo puts both accounts back".to_owned()),
                ),
                Ok(Recovery::Applying { next }) => (
                    "interrupted",
                    Some(format!("it stopped at step {} of {}: Undo puts both accounts back", next + 1, txn.manifest.plan.steps.len())),
                ),
                Ok(Recovery::Undoing) => (
                    "interrupted",
                    Some("Undo stopped part way: Undo again finishes it".to_owned()),
                ),
                Ok(Recovery::Unclear(why)) => ("interrupted", Some(why)),
                Err(error) => ("interrupted", Some(error.to_string())),
            };
            MoveRecordInfo {
                move_id: txn.manifest.id.clone(),
                conversation: txn.manifest.request.conversation.clone(),
                state: label.to_owned(),
                at_ms: txn.manifest.created_ms,
                detail: detail.or(txn.manifest.problem.clone()),
                kept: txn.dependencies().iter().map(|path| shown(path)).collect(),
            }
        })
        .collect();
    out.reverse();
    out
}

/// `path` with the home folder as `~`.
fn shown(path: &std::path::Path) -> String {
    match dirs::home_dir().and_then(|home| {
        path.strip_prefix(home)
            .ok()
            .map(std::path::Path::to_path_buf)
    }) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// What moves keep where it was on purpose (out-of-line outputs old
/// messages name) and their own folders (backups, kept branches, work
/// kept aside by Undo): cleanup must leave each.
#[must_use]
pub fn kept_paths(state: &Arc<AppState>) -> Vec<PathBuf> {
    Transaction::all(&moves_dir(state))
        .iter()
        .flat_map(Transaction::dependencies)
        .collect()
}

/// Tauri: the Move offer for the switch screen.
///
/// # Errors
///
/// As [`offer`].
#[tauri::command]
pub async fn switch_move_offer(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
) -> AppResult<MoveOfferInfo> {
    crate::ipc::blocking(state.inner(), "check the move", move |state| {
        offer(state, &session_id, &target_profile_id)
    })
    .await
}

/// Tauri: moves the conversation (the user's click on Move).
///
/// # Errors
///
/// As [`execute`].
#[tauri::command]
pub async fn switch_move_execute(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
    take_notes: Option<bool>,
) -> AppResult<MoveDoneInfo> {
    let notes = if take_notes == Some(true) {
        NotesChoice::TakeSource
    } else {
        NotesChoice::KeepTarget
    };
    crate::ipc::blocking(state.inner(), "move", move |state| {
        let done = execute(state, &session_id, &target_profile_id, notes)?;
        let _ = crate::features::sessions::read_model::emit_sessions(&app, state);
        Ok(done)
    })
    .await
}

/// Tauri: undoes a move (the user's click on Undo).
///
/// # Errors
///
/// As [`undo`].
#[tauri::command]
pub async fn switch_move_undo(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    move_id: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "undo the move", move |state| {
        let said = undo(state, &move_id)?;
        let _ = crate::features::sessions::read_model::emit_sessions(&app, state);
        Ok(said)
    })
    .await
}

/// Tauri: the moves AI Juggler knows of (Undo, and one left part way).
///
/// # Errors
///
/// Never.
#[tauri::command]
pub async fn switch_moves(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<MoveRecordInfo>> {
    crate::ipc::blocking(state.inner(), "list moves", |state| Ok(moves(state))).await
}

#[cfg(test)]
#[path = "moving_tests.rs"]
mod tests;
