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
//!
//! 0.4.9 (a 406 MB move refused: "the backup of … doesn't match it"):
//! a profile is quit as its whole process tree ([`places::tree`]: its
//! helpers and the Code tab's Claude Code outlive its main process and
//! wrote the history's last lines after it ended), then every file the
//! move touches must hold still before it is backed up, a backup that
//! still caught a write is tried once more, a refusal says why in plain
//! words (the paths only in Copy details), the switch screen is told how
//! far the move is, and an undone move's backups go.
//!
//! [`places::tree`]: crate::features::places::tree

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use door::shown;
use juggler_adapter::moving::{
    MemberSide, MovePlan, MoveRequest, NotesChoice, Refusal, RefusalKind,
};
use juggler_core::model::Account;
use juggler_handover::moving::{Lock, MoveError, Phase, Stopped, Transaction};

use crate::error::{AppError, AppResult, MoveHolder, MoveStop, StopClass};
use crate::features::conversations::procs::Table;
use crate::features::ownership::ProcId;
use crate::features::places::book::KnownPlace;
use crate::features::places::hold;
use crate::features::places::tree::{self, Member};
use crate::state::AppState;

/// How long a profile's app is given to quit.
const QUIT_WAIT: Duration = Duration::from_secs(30);
/// AJ-E2318: how long its sessions that write the moving family are
/// given to end once the app closed (the Code tab's Claude Code writes its
/// last lines and ends within a second in the live trace).
const WRITER_WAIT: Duration = Duration::from_secs(10);
/// How long a started profile is given to come up.
const START_WAIT: Duration = Duration::from_secs(30);
/// Between looks.
const EVERY: Duration = Duration::from_millis(250);
/// AJ-E2319: how long a transient stop waits before its one try more.
const AGAIN: Duration = Duration::from_secs(3);

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
    /// AJ-E2317: why the sides can't be told, whatever happens next (two
    /// profiles list the conversation, the picked App isn't on the
    /// account); nothing is planned or quit then.
    stop: Option<Refusal>,
}

/// The process side of a move (a seam: the tests script it).
pub trait MoveSystem: Send + Sync {
    /// The profile's main process now (pid, start).
    fn running(&self, state: &Arc<AppState>, place_id: &str) -> Option<(u32, Option<i64>)>;
    /// Asks `pid` to quit (as a normal quit). Called only through the
    /// process's [`ProcId`], checked right before
    /// ([`crate::features::ownership::signal::quit_checked`]).
    ///
    /// # Errors
    ///
    /// The system's reason.
    fn quit(&self, pid: u32) -> Result<(), String>;
    /// AJ-E2318: the identity of each of `pids` running now (pid and
    /// start, to the microsecond; a gone pid is absent).
    fn identify(&self, pids: &[u32]) -> HashMap<u32, ProcId>;
    /// AJ-E2318: the process table now (the writers hook reads it).
    fn table(&self, state: &Arc<AppState>) -> Table;
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
    /// AJ-E2322: waits until `seen` finds the conversation open, woken by
    /// a change in `folders` (the real system's file events), at most
    /// `cap`. Returns what `seen` found; `None`: not within `cap`. By
    /// default it looks every `timeline::LOOK` through [`Self::settle`].
    fn wait_open(
        &self,
        folders: &[PathBuf],
        cap: Duration,
        seen: &mut dyn FnMut() -> Option<String>,
    ) -> Option<String> {
        let _ = folders;
        let looks = (cap.as_millis() / timeline::LOOK.as_millis()).max(1);
        for _ in 0..looks {
            if let Some(found) = seen() {
                return Some(found);
            }
            self.settle(timeline::LOOK);
        }
        seen()
    }
    /// AJ-E2324: waits until one of `pids` ends, at most `cap` (the real
    /// system is woken by the kernel; by default a look every `EVERY`).
    fn wait_exit(&self, pids: &[u32], cap: Duration) {
        let _ = pids;
        std::thread::sleep(EVERY.min(cap));
    }
    /// AJ-E2324: waits until `window` passes with no change to `paths`
    /// (the real system's file events), at most `cap`; whether it did. By
    /// default it settles `window` and says so: the sizes and times of
    /// last write looked at after decide (`still::hold_still`).
    fn quiet_for(&self, paths: &[PathBuf], window: Duration, cap: Duration) -> bool {
        let _ = (paths, cap);
        self.settle(window);
        true
    }
    /// AJ-E2324 R2: the programs (pid, name) that have a file at or under
    /// `paths` open to write it.
    fn open_writers(&self, _paths: &[PathBuf]) -> Vec<(u32, String)> {
        Vec::new()
    }
    /// How long a profile's app (its main process) is given to quit.
    fn quit_wait(&self) -> Duration {
        QUIT_WAIT
    }
    /// AJ-E2318: how long the sessions writing the family are given to
    /// end once the apps closed.
    fn writer_wait(&self) -> Duration {
        WRITER_WAIT
    }
    /// Every process of `place` now ([`tree::now`]): `roots` (its main
    /// process, while it runs) and below, its helpers, the tools it runs
    /// from its own folder, and theirs. What the move closes with it:
    /// reported, waited on only where a writer.
    fn tree(&self, state: &Arc<AppState>, place: &KnownPlace, roots: &[u32]) -> Vec<Member>;
    /// Tells the switch screen how the move is going.
    fn tell(&self, _progress: &MoveProgressInfo) {}
    /// AJ-E2315 (the self rule): why closing `place` (main process `pid`)
    /// for `action` would close an AI Juggler, in words; `None`: it wouldn't.
    fn quits_juggler(&self, _: &Arc<AppState>, _: &KnownPlace, _: u32, _: &str) -> Option<String> {
        None
    }
    /// AJ-E2317: the tool folder `place`'s own sessions show it uses, when
    /// that contradicts `linked`, the folder AI Juggler has it on
    /// (AJ-E2327's evidence, `places::link::folder_mismatch`); `None`:
    /// nothing contradicts it.
    fn link_mismatch(
        &self,
        state: &Arc<AppState>,
        place_id: &str,
        place: &KnownPlace,
        linked: &Path,
    ) -> Option<PathBuf>;
}

#[cfg(test)]
#[path = "moving_fixtures.rs"]
mod fixtures;
#[path = "moving_info.rs"]
mod info;
pub use info::{
    MoveBlockInfo, MoveBlockKind, MoveDoneInfo, MoveMemberInfo, MoveOfferInfo, MoveOutcomeInfo,
    MoveProgressInfo, MoveUndoneInfo,
};
#[path = "moving_door.rs"]
mod door;
#[path = "moving_pick.rs"]
mod pick;
#[path = "moving_quit.rs"]
mod quit;
#[path = "moving_real.rs"]
mod real;
#[path = "moving_recover.rs"]
pub mod recover;
#[path = "moving_reopen.rs"]
mod reopen;
#[path = "moving_repair.rs"]
pub mod repair;
#[path = "moving_retention.rs"]
pub mod retention;
#[path = "moving_still.rs"]
mod still;
#[path = "moving_timeline.rs"]
mod timeline;
pub use door::{MoveRecordInfo, moves};
use pick::{links_hold, sides, start_checked};
use quit::{Writing, stop_places};
pub use real::{Real, Tell};
use still::{hold_still, size_words, still_writing};

/// AJ-E2320: new moves' journals, in the move area every AI Juggler on
/// this Mac shares ([`hold::journals`]).
fn moves_dir(state: &Arc<AppState>) -> PathBuf {
    hold::journals(&hold::area_at(&state.data_dir))
}

/// AJ-E2320: the lock on `roots`, in the shared move area.
fn lock(state: &Arc<AppState>, roots: &[PathBuf]) -> AppResult<Lock> {
    Lock::take_in(&hold::locks(&hold::area_at(&state.data_dir)), roots).map_err(engine)
}

fn refused(why: impl Into<String>) -> AppError {
    AppError::Refused(why.into())
}

#[allow(clippy::needless_pass_by_value, reason = "used as `map_err(engine)`")]
fn engine(error: MoveError) -> AppError {
    refused(error.to_string())
}

/// AJ-E2319: `error` as a typed stop: a typed one as it is; a refusal (or
/// what isn't found) of `class`; anything else is a bug.
fn typed(error: AppError, class: StopClass) -> AppError {
    match error {
        AppError::Move(_) => error,
        AppError::Refused(words) => MoveStop::new(class, words.clone(), words).into(),
        other @ (AppError::NotFound { .. } | AppError::Validation { .. }) => MoveStop::new(
            class,
            crate::error_words::user_words(&other),
            other.to_string(),
        )
        .into(),
        other => MoveStop::new(
            StopClass::Bug,
            crate::error_words::user_words(&other),
            other.to_string(),
        )
        .into(),
    }
}

/// AJ-E2319: what stands in Move's way, as one stop: setup when anything
/// the person can't clear is among it, else theirs to clear.
fn refusals(why_not: &[Refusal]) -> AppError {
    let class = if why_not.iter().any(|r| matches!(r.kind, RefusalKind::Setup)) {
        StopClass::Setup
    } else {
        StopClass::NeedsYou
    };
    let words = juggler_handover::moving::words(why_not);
    MoveStop::new(class, words.clone(), words).into()
}

/// AJ-E2319: `error` names the move it stopped.
fn of_move(error: AppError, move_id: &str) -> AppError {
    match error {
        AppError::Move(mut stop) => {
            stop.move_id = Some(move_id.to_owned());
            AppError::Move(stop)
        }
        other => other,
    }
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

fn place_name(state: &Arc<AppState>, place: &KnownPlace) -> String {
    crate::features::places::named(state, place)
}

/// Why the move can't happen now because something is working: a turn
/// running in either profile, or (terminal folders) the conversation
/// open in a session. Each the user's to clear (D-127).
fn busy(
    state: &Arc<AppState>,
    sides: &Sides,
    system: &dyn MoveSystem,
    writing: &Writing<'_>,
) -> Vec<Refusal> {
    let places: Vec<(String, KnownPlace)> = [&sides.source_place, &sides.target_place]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let both = (&places[..], &sides.request);
    let mut out = quiet(state, both, &sides.accounts, system, "Move", writing);
    if sides.source_place.is_none() && sides.source_live && out.is_empty() {
        out.push(Refusal::user_can_clear(
            "the conversation is open in a session; Move becomes available once you end it",
        ));
    }
    out
}

/// Why the folders of `places` and `accounts` aren't quiet enough for
/// `action` (Move, Undo) to change them: AI Juggler itself in a profile
/// (the self rule), a turn running in either profile, a writer of the
/// family (`writing`) that no profile the move closes runs (AJ-E2318),
/// or a session of either account open outside them (a terminal writes
/// the same folder). Each says how it clears. Decided before anything
/// quits.
fn quiet(
    state: &Arc<AppState>,
    (places, request): (&[(String, KnownPlace)], &MoveRequest),
    accounts: &[String],
    system: &dyn MoveSystem,
    action: &str,
    writing: &Writing<'_>,
) -> Vec<Refusal> {
    // AJ-E2317: a profile whose sessions contradict its folder link needs
    // the user first.
    let mut out = links_hold(state, places, request, system, action);
    let mut hosts = Vec::new();
    let mut running = Vec::new();
    for (id, place) in places {
        if let Some((pid, _)) = system.running(state, id) {
            hosts.push(pid);
            running.push((place.clone(), pid));
            // Before anything quits: never AI Juggler (ours or another).
            let juggler = system.quits_juggler(state, place, pid, action);
            out.extend(juggler.map(Refusal::user_can_clear));
            let working = system.working(state, pid);
            if working > 0 {
                let one = working == 1;
                out.push(Refusal::user_can_clear(format!(
                    "{working} session{} {} working in {}; {action} becomes available once {} {}",
                    if one { "" } else { "s" },
                    if one { "is" } else { "are" },
                    place_name(state, place),
                    if one { "it" } else { "they" },
                    if one { "finishes" } else { "finish" },
                )));
            }
        }
    }
    // The named writers say it; the live view's count would say it again.
    let outside = quit::outside_now(state, system, &running, writing);
    if !outside.is_empty() {
        out.push(Refusal::user_can_clear(quit::outside_clears(
            &outside,
            !places.is_empty(),
            action,
        )));
        return out;
    }
    let elsewhere = system.open_elsewhere(state, accounts, &hosts);
    if elsewhere > 0 {
        let one = elsewhere == 1;
        out.push(Refusal::user_can_clear(format!(
            "{elsewhere} session{} of these accounts {} open outside the app (in a terminal, say); {action} becomes available once you end {}",
            if one { "" } else { "s" },
            if one { "is" } else { "are" },
            if one { "it" } else { "them" },
        )));
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
    still_down_held(state, stopped, system).map_err(|(words, _)| words)
}

/// [`still_down`], with the profile that started as the holder
/// (AJ-E2319).
fn still_down_held(
    state: &Arc<AppState>,
    stopped: &[StoppedPlace],
    system: &dyn MoveSystem,
) -> Result<(), (String, MoveHolder)> {
    match stopped
        .iter()
        .find(|p| system.running(state, &p.id).is_some())
    {
        Some(place) => {
            let name = place_name(state, &place.place);
            Err((
                format!("{name} was started while the move ran"),
                MoveHolder {
                    name,
                    place_id: Some(place.id.clone()),
                },
            ))
        }
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

/// Where AI Juggler keeps what a shape check found (D-128), for the gate
/// watch (`scripts/gate_watch.py`) to list.
pub const SHAPE_FILE: &str = "compat/shape-mismatches.json";
/// How many it keeps.
const SHAPE_KEPT: usize = 64;

/// D-128: keeps what didn't fit the move's shape check, with the tool and
/// its version, one entry per tool, version and what (the newest kept).
/// Best effort: a note that can't be written changes nothing else.
fn note_shape(state: &Arc<AppState>, sides: &Sides, plan: &MovePlan) {
    if plan.shape.is_empty() {
        return;
    }
    let path = state.data_dir.join(SHAPE_FILE);
    let mut seen: Vec<serde_json::Value> = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|v| v.get("seen").and_then(serde_json::Value::as_array).cloned())
        .unwrap_or_default();
    let now = juggler_core::model::Timestamp::now().millis();
    let app = sides.source_place.is_some();
    let mut versions: Vec<String> = [&sides.request.source, &sides.request.target]
        .into_iter()
        .map(|side| side.version.clone().unwrap_or_default())
        .collect();
    versions.dedup();
    for version in versions {
        for what in &plan.shape {
            seen.retain(|entry| {
                !(entry["tool"] == sides.tool.as_str()
                    && entry["app"] == app
                    && entry["version"] == version.as_str()
                    && entry["what"] == what.as_str())
            });
            seen.push(serde_json::json!({
                "tool": sides.tool, "app": app, "version": version,
                "feature": "move", "what": what, "at_ms": now,
            }));
        }
    }
    let excess = seen.len().saturating_sub(SHAPE_KEPT);
    seen.drain(..excess);
    let body = serde_json::json!({"v": 1, "seen": seen});
    if let Some(dir) = path.parent()
        && std::fs::create_dir_all(dir).is_ok()
    {
        let tmp = path.with_extension("json.part");
        if std::fs::write(&tmp, body.to_string()).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

/// The switch screen's Move offer for `session_id` to `target_profile_id`'s
/// account, into the App `target_place_id` the user picked (AJ-E2317).
/// Reads only.
///
/// # Errors
///
/// The session or account can't be resolved.
pub fn offer(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    target_place_id: Option<&str>,
) -> AppResult<MoveOfferInfo> {
    let Ok(sides) = sides(state, session_id, target_profile_id, target_place_id) else {
        return Ok(MoveOfferInfo::default());
    };
    Ok(offer_of(state, &sides, &Real::quiet()))
}

/// The offer for resolved `sides` (D-127: each block says whether the
/// user can clear it; D-128: what didn't fit is kept for the gate watch).
fn offer_of(state: &Arc<AppState>, sides: &Sides, system: &dyn MoveSystem) -> MoveOfferInfo {
    if sides.adapter.in_place_move().is_none() {
        return MoveOfferInfo::default();
    }
    if let Some(stop) = &sides.stop {
        return MoveOfferInfo {
            applies: true,
            blocks: vec![MoveBlockInfo::from(stop.clone())],
            ..MoveOfferInfo::default()
        };
    }
    let plan = match plan_of(sides) {
        Ok(plan) => plan,
        Err(error) => {
            return MoveOfferInfo {
                applies: true,
                blocks: vec![MoveBlockInfo {
                    kind: MoveBlockKind::Setup,
                    words: error.to_string(),
                }],
                ..MoveOfferInfo::default()
            };
        }
    };
    note_shape(state, sides, &plan);
    let folder = moves_dir(state);
    let mut blocks = juggler_handover::moving::preflight(&plan, &folder);
    // AJ-E2325: a backup that won't fit brings the offer to clear old ones.
    let disk = folder
        .ancestors()
        .find(|dir| dir.is_dir())
        .unwrap_or(&folder);
    if juggler_platform::room::free_space(disk)
        .is_ok_and(|free| free < juggler_handover::moving::room_needed(&plan))
    {
        retention::note_room_short(state, juggler_core::model::Timestamp::now().millis());
    }
    if let Some(writing) = writing_of(sides, &plan) {
        blocks.extend(busy(state, sides, system, &writing));
    }
    let restarts = [&sides.source_place, &sides.target_place]
        .into_iter()
        .flatten()
        .map(|(_, place)| place_name(state, place))
        .collect();
    MoveOfferInfo {
        applies: true,
        blocks: blocks.into_iter().map(MoveBlockInfo::from).collect(),
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
        carried: carried_words(
            &plan,
            &sides.target_place.as_ref().map_or_else(
                || sides.target_label.clone(),
                |(_, place)| place_name(state, place),
            ),
        ),
    }
}

/// AJ-E2328: what `plan` carries beside the family, in words, `target`
/// the App that has it after the move.
fn carried_words(plan: &MovePlan, target: &str) -> Vec<String> {
    use juggler_adapter::moving::Carried;
    plan.carried
        .iter()
        .map(|carried| match carried {
            Carried::Worktree { name, branch } => match branch {
                Some(branch) => format!(
                    "Its worktree {name} (branch {branch}): {target} owns it after the move; its files stay where they are"
                ),
                None => format!(
                    "Its worktree {name}: {target} owns it after the move; its files stay where they are"
                ),
            },
            Carried::Notes { of, files } => format!(
                "The side-session notes of “{of}” ({files} file{})",
                if *files == 1 { "" } else { "s" }
            ),
        })
        .collect()
}

/// The profiles a move stopped: (place id, place, pid, start, it was
/// running before).
struct StoppedPlace {
    id: String,
    place: KnownPlace,
    was_running: bool,
}

/// [`writing_of`] for a recorded move `txn` of `adapter`'s tool (Undo).
fn writing_of_move<'a>(
    adapter: &'a juggler_adapter::adapter::SharedAdapter,
    txn: &Transaction,
) -> AppResult<Writing<'a>> {
    let manifest = &txn.manifest;
    Ok(Writing {
        mover: adapter
            .in_place_move()
            .ok_or_else(|| refused("this tool's conversations travel as a copy"))?,
        dirs: vec![
            manifest.request.source.tool_dir.clone(),
            manifest.request.target.tool_dir.clone(),
        ],
        conversations: manifest
            .plan
            .members
            .iter()
            .map(|m| m.conversation.clone())
            .collect(),
    })
}

/// What the writers hook is asked for `sides`' move of `plan`'s family
/// (AJ-E2318): both accounts' folders, whatever the place.
fn writing_of<'a>(sides: &'a Sides, plan: &MovePlan) -> Option<Writing<'a>> {
    Some(Writing {
        mover: sides.adapter.in_place_move()?,
        dirs: vec![
            sides.request.source.tool_dir.clone(),
            sides.request.target.tool_dir.clone(),
        ],
        conversations: plan
            .members
            .iter()
            .map(|m| m.conversation.clone())
            .collect(),
    })
}

/// Starts again the profiles that were running (and aren't again: one the
/// user started meanwhile is left as it is), each on the folder `request`
/// gives it, never one its sessions contradict (AJ-E2317).
fn start_again(
    state: &Arc<AppState>,
    stopped: &[StoppedPlace],
    request: &MoveRequest,
    system: &dyn MoveSystem,
) {
    for place in stopped.iter().filter(|p| p.was_running) {
        if system.running(state, &place.id).is_some() {
            continue;
        }
        if let Err(why) = start_checked(state, &place.id, &place.place, request, system) {
            tracing::warn!(target: "juggler::move", "a profile didn't start again after a move: {why}");
        }
    }
}

/// Starts `place` (when it isn't running, on the folder `request` gives
/// it) and sends `link` to its own process. Returns that process.
fn open_in(
    state: &Arc<AppState>,
    (id, place): (&str, &KnownPlace),
    link: &str,
    request: &MoveRequest,
    system: &dyn MoveSystem,
) -> Result<u32, String> {
    if system.running(state, id).is_none() {
        start_checked(state, id, place, request, system)?;
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

/// 0.5.1 F69: a Copy switch's "opened", by Move's own check (AJ-E2322):
/// whether the app profile `place_id` (its main process `pid`) shows
/// `conversation` open after the link: a session of it running in that
/// app (the writers hook, asked of `tool_dir`), or the app adding to its
/// history (`history`: the path and its length as the link went), waited
/// for by file events at most [`timeline::OPEN_CAP`]. In words, the
/// evidence; `None`: not seen open (the switch says it was sent, and
/// offers Open).
pub fn seen_open_after_link(
    state: &Arc<AppState>,
    (place_id, pid): (&str, u32),
    mover: &dyn juggler_adapter::moving::InPlaceMove,
    (tool_dir, conversation): (&Path, &str),
    history: Option<&(PathBuf, u64)>,
    system: &dyn MoveSystem,
) -> Option<String> {
    let place =
        crate::features::places::with_book(state, |book| book.places.get(place_id).cloned())
            .ok()
            .flatten()?;
    let writing = Writing {
        mover,
        dirs: vec![tool_dir.to_path_buf()],
        conversations: vec![conversation.to_owned()],
    };
    let folders: Vec<PathBuf> = history
        .and_then(|(path, _)| path.parent())
        .map(Path::to_path_buf)
        .into_iter()
        .collect();
    system.wait_open(&folders, timeline::OPEN_CAP, &mut || {
        timeline::seen_open(state, (pid, &place), &writing, history, system)
    })
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
/// `target_profile_id`'s account, into the App `target_place_id` the user
/// picked (AJ-E2317: by its id, never the account's).
///
/// # Errors
///
/// Refused before anything changed (with why), or a failure after which
/// the move was undone (with why).
pub fn execute(
    state: &Arc<AppState>,
    (session_id, target_profile_id, target_place_id): (&str, &str, Option<&str>),
    notes: NotesChoice,
    tell: Tell,
) -> AppResult<MoveDoneInfo> {
    let mut sides = sides(state, session_id, target_profile_id, target_place_id)
        .map_err(|error| typed(error, StopClass::NeedsYou))?;
    sides.request.notes = notes;
    let done = run_once_more(state, &sides, &Real::telling(tell));
    // AJ-E2325: a refused attempt's folder goes at once.
    if done.is_err() {
        retention::drop_refused(state);
    }
    done
}

/// AJ-E2319: the move, typed; a transient stop that changed nothing is
/// tried once more by itself, after a pause.
fn run_once_more(
    state: &Arc<AppState>,
    sides: &Sides,
    system: &dyn MoveSystem,
) -> AppResult<MoveDoneInfo> {
    let once = |system: &dyn MoveSystem| {
        run(state, sides, system).map_err(|error| typed(error, StopClass::NeedsYou))
    };
    match once(system) {
        Err(AppError::Move(stop)) if stop.retryable() => {
            tracing::info!(target: "juggler::move", "a transient stop, tried once more: {}", stop.detail);
            system.tell(&MoveProgressInfo::step(
                "closing",
                "Trying once more…".to_owned(),
            ));
            system.settle(AGAIN);
            once(system)
        }
        done => done,
    }
}

/// The move of resolved `sides`.
#[allow(
    clippy::too_many_lines,
    reason = "the move, step by step, in one place"
)]
fn run(state: &Arc<AppState>, sides: &Sides, system: &dyn MoveSystem) -> AppResult<MoveDoneInfo> {
    if let Some(stop) = &sides.stop {
        return Err(refusals(std::slice::from_ref(stop)));
    }
    // AJ-E2319: a tool that can't plan this move is the setup's.
    let plan = plan_of(sides).map_err(|error| typed(error, StopClass::Setup))?;
    let writing =
        writing_of(sides, &plan).ok_or_else(|| typed(refused("no mover"), StopClass::Setup))?;
    let folder = moves_dir(state);
    let mut why_not = juggler_handover::moving::preflight(&plan, &folder);
    why_not.extend(busy(state, sides, system, &writing));
    if !why_not.is_empty() {
        return Err(refusals(&why_not));
    }
    let places: Vec<(String, KnownPlace)> = [&sides.source_place, &sides.target_place]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    // AJ-E2319: another move on these files ends by itself.
    let _lock = lock(state, &plan.roots).map_err(|error| typed(error, StopClass::Transient))?;
    let ids: Vec<String> = places.iter().map(|(id, _)| id.clone()).collect();
    let now = juggler_core::model::Timestamp::now().millis();
    let move_id = new_id(now);
    // AJ-E2320: held in every AI Juggler before anything quits.
    let _hold =
        crate::features::places::MoveHold::take(state, &ids, &move_id).map_err(|words| {
            // AJ-E2319: a running move ends by itself; a crashed one is the
            // person's to undo; anything else is never expected.
            let class = match words.as_str() {
                hold::BUSY => StopClass::Transient,
                hold::UNFINISHED => StopClass::NeedsYou,
                _ => StopClass::Bug,
            };
            typed(refused(words), class)
        })?;
    // AJ-E2326 R1: no start path continues the conversation meanwhile.
    let _held = crate::features::places::ConversationHold::take(
        state,
        &sides.request.conversation,
        &move_id,
    );
    let previous = hold::all_moves(&state.data_dir)
        .into_iter()
        .rev()
        .find(|txn| {
            txn.manifest.phase != Phase::Undone
                && txn.manifest.request.conversation == sides.request.conversation
        })
        .map(|txn| txn.manifest.id.clone());
    let mut txn = Transaction::begin(
        &folder,
        &move_id,
        &sides.tool,
        sides.request.clone(),
        plan,
        previous,
        now,
    )
    .map_err(|error| typed(engine(error), StopClass::Bug))?;
    txn.remember_places(ids, sides.accounts.clone())
        .map_err(|error| of_move(typed(engine(error), StopClass::Bug), &move_id))?;
    let mut timeline = timeline::Timeline::start();
    // AJ-E2326 R2: the conversation open again where it was, checked, once
    // the move is refused after its app was quit (it was open there).
    let reopen_source = |txn: &Transaction, ran: bool| {
        if let (true, Some((id, place)), Some(mover)) =
            (ran, &sides.source_place, sides.adapter.in_place_move())
        {
            let seen =
                reopen::reopen_where_it_was(state, (id, place), &sides.request, mover, system);
            let _ = txn.timeline("reopened", 0, serde_json::json!({ "seen": seen }));
        }
    };
    let source_was = sides
        .source_place
        .as_ref()
        .and_then(|(id, _)| system.running(state, id));

    // The single-writer phase.
    let (stopped_places, kept_running) =
        match stop_places(state, (&places, &sides.request), system, "moved", &writing) {
            Ok(stopping) => {
                timeline.mark(
                    &txn,
                    "closing",
                    serde_json::json!({"waited": stopping.waited, "remained": stopping.remained}),
                );
                (stopping.stopped, stopping.notes)
            }
            Err(error) => {
                let remained: Vec<String> = match &error {
                    AppError::Move(stop) => stop.holders.iter().map(|h| h.name.clone()).collect(),
                    _ => Vec::new(),
                };
                timeline.mark(&txn, "closing", serde_json::json!({"remained": remained}));
                // Planned only: its undo touches no file. The journal reads
                // planned → undone, with nothing kept.
                let _ = txn.undo(
                    &Stopped::checked(),
                    juggler_core::model::Timestamp::now().millis(),
                );
                let _ = txn.drop_backups();
                // Quit, then started again by the refusal: open it there.
                let source_now = sides
                    .source_place
                    .as_ref()
                    .and_then(|(id, _)| system.running(state, id));
                reopen_source(&txn, source_was.is_some() && source_now != source_was);
                return Err(of_move(error, &move_id));
            }
        };
    let stopped = Stopped::checked();
    // `stop`'s words say why in plain words (no path), its detail is the
    // program's own, for Copy details; its class and holders say what
    // comes next (AJ-E2319). It names this move, for Undo by its id.
    let fail = |txn: &mut Transaction, mut stop: MoveStop| -> AppError {
        stop.move_id = Some(txn.manifest.id.clone());
        let (words, detail) = (stop.words.clone(), stop.detail.clone());
        let untouched = txn.manifest.phase == Phase::Planned;
        // Never undo under a running app: it would write over the restore
        // (a move only planned has nothing to restore).
        if !untouched
            && let Err((started, holder)) = still_down_held(state, &stopped_places, system)
        {
            let _ = txn.stop_with(&format!("{detail}; {started}"));
            stop.words = format!(
                "{words}, and {started}, so AI Juggler left both accounts as they are now. Quit it, then Undo the move here."
            );
            stop.class = StopClass::NeedsYou;
            stop.holders = vec![holder];
            stop.unchanged = false;
            return stop.into();
        }
        match txn.undo(&stopped, juggler_core::model::Timestamp::now().millis()) {
            Ok(_) => {
                // Nothing is left to restore from them.
                let _ = txn.drop_backups();
                start_again(state, &stopped_places, &sides.request, system);
                let source = sides.source_place.as_ref().map(|(id, _)| id);
                let ran = stopped_places
                    .iter()
                    .any(|p| Some(&p.id) == source && p.was_running);
                reopen_source(txn, ran);
                // Only a stop that can clear by itself says try again.
                stop.words = if stop.class == StopClass::Transient {
                    format!("{words}, so nothing was moved. Try again in a moment.")
                } else {
                    format!("{words}, so nothing was moved.")
                };
                stop.into()
            }
            Err(error) => {
                // Left closed: an app writing into a half-undone folder
                // would make the next Undo reconcile its writes.
                let _ = txn.stop_with(&format!("{detail}; undo: {error}"));
                pause(state, &stopped_places);
                stop.words = format!(
                    "{words}. Undoing it didn't finish; both profiles stay closed, the move's journal and backups are kept, and Undo the move here finishes it."
                );
                stop.detail = format!("{detail}; undo: {error}");
                stop.class = StopClass::Bug;
                stop.unchanged = false;
                stop.into()
            }
        }
    };
    // A profile started again while it ran: the person's (it names the
    // profile, with its door).
    let started_again = |(started, holder): (String, MoveHolder)| {
        MoveStop::new(StopClass::NeedsYou, started.clone(), started).held_by(vec![holder])
    };
    // The apps write their records when they quit: plan again, refuse a
    // changed family.
    let again = plan_of(sides).and_then(|plan| txn.replan(plan).map_err(engine));
    if let Err(error) = again {
        let words = "the conversation's family or files changed while the apps closed".to_owned();
        let stop = MoveStop::new(StopClass::Transient, words, error.to_string());
        return Err(fail(&mut txn, stop));
    }
    let plan = txn.manifest.plan.clone();
    let mover = sides
        .adapter
        .in_place_move()
        .ok_or_else(|| of_move(typed(refused("no mover"), StopClass::Bug), &move_id))?;
    if let Err(started) = still_down_held(state, &stopped_places, system) {
        return Err(fail(&mut txn, started_again(started)));
    }
    // 0.4.9: every file it touches holds still before it is backed up.
    let watched: Vec<PathBuf> = txn.touched().into_iter().map(Path::to_path_buf).collect();
    system.tell(&MoveProgressInfo::step(
        "still",
        "Waiting for the conversation's files to hold still…".to_owned(),
    ));
    let still = hold_still(&watched, system);
    let changing: Vec<String> = still
        .as_ref()
        .err()
        .into_iter()
        .flatten()
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    timeline.mark(&txn, "still", serde_json::json!({"remained": changing}));
    if let Err(moving) = still {
        let first = moving.first().cloned().unwrap_or_default();
        let detail = format!(
            "still changing after {} s: {}",
            still::STILL_CAP.as_secs(),
            moving
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        // A writer still finishing: tried once more (AJ-E2319).
        let (words, holders) = still_writing(state, sides, &first, system, &writing);
        let stop = MoveStop::new(StopClass::Transient, words, detail).held_by(holders);
        return Err(fail(&mut txn, stop));
    }
    // AJ-E2324 R2: no program has them open to write them.
    if let Some(stop) = still::open_writers(&watched, system) {
        return Err(fail(&mut txn, stop));
    }
    let mut told = 0_u64;
    let mut progress = |done: u64, total: u64| {
        if done < told {
            told = 0; // a second try reads from the start
        }
        if done == total || (done - told).saturating_mul(100) >= total {
            told = done;
            system.tell(&MoveProgressInfo {
                stage: "backing_up".to_owned(),
                words: format!("Backing up the conversation ({})…", size_words(total)),
                done,
                total,
            });
        }
    };
    let backed_up = txn.back_up_with(&stopped, &mut progress, &mut || {
        let _ = hold_still(&watched, system);
    });
    if let Err(error) = backed_up {
        let stop = match &error {
            MoveError::Changed(path) => {
                let (words, holders) = still_writing(state, sides, path, system, &writing);
                MoveStop::new(StopClass::Transient, words, error.to_string()).held_by(holders)
            }
            _ => MoveStop::new(
                StopClass::Bug,
                "AI Juggler couldn't back up the conversation's files",
                error.to_string(),
            ),
        };
        return Err(fail(&mut txn, stop));
    }
    timeline.mark(&txn, "backing_up", serde_json::json!({}));
    if let Err(started) = still_down_held(state, &stopped_places, system) {
        return Err(fail(&mut txn, started_again(started)));
    }
    if let Some(stop) = still::open_writers(&watched, system) {
        return Err(fail(&mut txn, stop));
    }
    system.tell(&MoveProgressInfo::step(
        "moving",
        "Moving the conversation…".to_owned(),
    ));
    if let Err(error) = txn.apply(&stopped, 0) {
        let words = "the conversation's files couldn't be moved";
        return Err(fail(
            &mut txn,
            MoveStop::new(StopClass::Bug, words, error.to_string()),
        ));
    }
    timeline.mark(&txn, "moving", serde_json::json!({}));
    let cold = match txn.verify_cold(mover.verify(&sides.request, &plan)) {
        Ok(report) => report,
        Err(error) => {
            let words = "AI Juggler couldn't record its checks of the move";
            return Err(fail(
                &mut txn,
                MoveStop::new(StopClass::Bug, words, error.to_string()),
            ));
        }
    };
    if !cold.passed() {
        let words = "a check before reopening failed";
        return Err(fail(
            &mut txn,
            MoveStop::new(StopClass::Bug, words, cold.failures().join("; ")),
        ));
    }

    timeline.mark(&txn, "checking_before", serde_json::json!({}));
    // Both profiles again, each on its own folders; the conversation
    // opened in the target by its own process.
    if !places.is_empty() {
        let names: Vec<String> = places.iter().map(|(_, p)| place_name(state, p)).collect();
        system.tell(&MoveProgressInfo::step(
            "starting",
            format!("Starting {} again…", names.join(" and ")),
        ));
    }
    let source_id = sides.source_place.as_ref().map(|(id, _)| id);
    if let Some(source) = stopped_places.iter().find(|p| Some(&p.id) == source_id)
        && source.was_running
        && let Err(why) = start_checked(state, &source.id, &source.place, &sides.request, system)
    {
        tracing::warn!(target: "juggler::move", "the source profile didn't start again: {why}");
    }
    let link = mover.resume_link(&plan.resume);
    // AJ-E2322: the conversation's history in the target, and its length
    // as the link goes (the app adding to it is evidence it opened).
    let history = plan
        .members
        .iter()
        .find(|m| m.conversation == sides.request.conversation)
        .and_then(|m| m.transcript_to.clone())
        .map(|path| {
            let len = std::fs::metadata(&path).map_or(0, |meta| meta.len());
            (path, len)
        });
    let opened = match (&sides.target_place, &link) {
        (Some((id, place)), Some(link)) => Some(
            open_in(state, (id, place), link, &sides.request, system)
                .map(|pid| (pid, place_name(state, place))),
        ),
        _ => None,
    };
    let target_name = sides
        .target_place
        .as_ref()
        .map_or_else(|| sides.target_label.clone(), |(_, p)| place_name(state, p));
    timeline.mark(&txn, "starting", serde_json::json!({}));
    // AJ-E2322 R1/R2: "opened" only when the adapter's check sees it open,
    // waited for by file events, capped; never a fixed settle.
    let seen = match (&opened, &sides.target_place) {
        (Some(Ok((pid, _))), Some((_, place))) => {
            system.tell(&MoveProgressInfo::step(
                "opening",
                format!("Opening the conversation in {target_name}…"),
            ));
            let target = Writing {
                mover,
                dirs: vec![sides.request.target.tool_dir.clone()],
                conversations: vec![sides.request.conversation.clone()],
            };
            let found =
                system.wait_open(&timeline::folders_of(&txn), timeline::OPEN_CAP, &mut || {
                    timeline::seen_open(state, (*pid, place), &target, history.as_ref(), system)
                });
            timeline.mark(
                &txn,
                "opening",
                serde_json::json!({"seen": found, "remained": if found.is_some() { Vec::<String>::new() } else { vec!["not seen open".to_owned()] }}),
            );
            found
        }
        _ => None,
    };
    system.tell(&MoveProgressInfo::step(
        "checking",
        format!("Checking the conversation in {target_name}…"),
    ));
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
    timeline.mark(
        &txn,
        "checking_after",
        serde_json::json!({"committed": after.passed()}),
    );
    // R1: open only by the adapter's evidence: a session of it running in
    // the target (seen while waiting), or the target app adding to its
    // history (each line sorted by the adapter).
    let is_open = matches!(opened, Some(Ok(_)))
        && (seen.is_some()
            || after.added.bookkeeping + after.added.notices + after.added.turns > 0);
    let members = plan.members.len();
    let many = |n: usize, one: &str, more: &str| format!("{n} {}", if n == 1 { one } else { more });
    let mut notes = plan.notes.clone();
    notes.extend(kept_running);
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
        opened: timeline::opened_outcome(opened, is_open, &sides.target_label),
        // Offered Open where it isn't seen open, in the App it moved to.
        open_place_id: sides
            .target_place
            .as_ref()
            .filter(|_| !is_open)
            .map(|(id, _)| id.clone()),
        conversation: sides.request.conversation.clone(),
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
pub fn undo(state: &Arc<AppState>, move_id: &str) -> AppResult<MoveUndoneInfo> {
    undo_with(state, move_id, &Real::quiet())
}

pub(crate) fn undo_with(
    state: &Arc<AppState>,
    move_id: &str,
    system: &dyn MoveSystem,
) -> AppResult<MoveUndoneInfo> {
    undo_as(state, move_id, system, true)
}

/// The recorded move `move_id` (its id checked first: never a path).
fn open_by_id(state: &Arc<AppState>, move_id: &str) -> AppResult<Transaction> {
    if move_id.is_empty()
        || !move_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(refused("that isn't a move's id"));
    }
    hold::open_move(&state.data_dir, move_id).map_err(engine)
}

/// What Undo and Finish take before they touch a recorded move `txn`: the
/// lock of its files, its hold in every AI Juggler, and its tool's adapter.
fn take_move(
    state: &Arc<AppState>,
    txn: &Transaction,
) -> AppResult<(
    Lock,
    crate::features::places::MoveHold,
    juggler_adapter::adapter::SharedAdapter,
)> {
    let lock = lock(state, &txn.manifest.plan.roots)?;
    let hold =
        crate::features::places::MoveHold::take(state, &txn.manifest.places, &txn.manifest.id)
            .map_err(refused)?;
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(
            txn.manifest.tool.as_str(),
        ))
        .ok_or_else(|| refused("this tool's conversations travel as a copy"))?;
    Ok((lock, hold, adapter))
}

/// [`undo_with`]; `reopen`: the conversation opened where it was after
/// (the user's Undo), not by startup recovery (AJ-E2321).
pub(crate) fn undo_as(
    state: &Arc<AppState>,
    move_id: &str,
    system: &dyn MoveSystem,
    reopen: bool,
) -> AppResult<MoveUndoneInfo> {
    let mut txn = open_by_id(state, move_id)?;
    // A later move (in any state but undone) that touches the same files
    // owns them now: undoing this one would put old copies over it.
    if door::moved_again(&hold::all_moves(&state.data_dir), &txn) {
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
    let (_lock, _hold, adapter) = take_move(state, &txn)?;
    let _held = crate::features::places::ConversationHold::take(
        state,
        &txn.manifest.request.conversation,
        move_id,
    );
    let writing = writing_of_move(&adapter, &txn)?;
    let both = (&places[..], &txn.manifest.request);
    let why_not = quiet(
        state,
        both,
        &txn.manifest.accounts,
        system,
        "Undo",
        &writing,
    );
    if !why_not.is_empty() {
        return Err(refused(juggler_handover::moving::words(&why_not)));
    }
    // R4: a refused Undo says "nothing was undone", not the Move's words.
    // AJ-E2319 R5: what kept running is told, as the move tells it.
    let stopping = stop_places(state, both, system, "undone", &writing)?;
    let (stopped_places, kept_running) = (stopping.stopped, stopping.notes);
    if let Err(started) = still_down(state, &stopped_places, system) {
        start_again(state, &stopped_places, &txn.manifest.request, system);
        return Err(refused(format!("{started}, so nothing was undone")));
    }
    let result = txn.undo(
        &Stopped::checked(),
        juggler_core::model::Timestamp::now().millis(),
    );
    let kept = match result {
        Ok(kept) => {
            // Nothing is left to restore from them (0.4.9).
            let _ = txn.drop_backups();
            start_again(state, &stopped_places, &txn.manifest.request, system);
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
    // Opened where it was, and seen open there (AJ-E2326 R2).
    let mut words = "Undone: the conversation is back where it was. Anything done since the move is kept aside; Copy details says where.".to_owned();
    if let (true, Some((id, place))) = (reopen, places.first()) {
        let seen = reopen::reopen_where_it_was(
            state,
            (id, place),
            &txn.manifest.request,
            writing.mover,
            system,
        );
        let _ = txn.timeline("reopened", 0, serde_json::json!({ "seen": seen }));
        if seen.is_none() {
            let name = place_name(state, place);
            words = format!(
                "Undone: the conversation is back where it was, but AI Juggler couldn't see it open in {name}. Open it from {name}. Anything done since the move is kept aside; Copy details says where."
            );
        }
    }
    // R4: no path in the words (Copy details has it).
    Ok(MoveUndoneInfo {
        words,
        kept: kept.display().to_string(),
        notes: kept_running,
    })
}

/// At start, before keep alive (AJ-E2321): what a crash left part way is
/// held and settled ([`recover`]); in the background, undone moves'
/// backups go (0.4.9). Committed moves' retention is AJ-E2325's policy.
pub fn sweep_at_startup(state: &Arc<AppState>) {
    recover::at_startup(state);
}

/// What moves keep where it was on purpose (out-of-line outputs old
/// messages name) and their own folders (backups, kept branches, work
/// kept aside by Undo): cleanup must leave each.
#[must_use]
pub fn kept_paths(state: &Arc<AppState>) -> Vec<PathBuf> {
    hold::all_moves(&state.data_dir)
        .iter()
        .flat_map(Transaction::dependencies)
        .collect()
}

#[cfg(test)]
#[path = "moving_tests.rs"]
mod tests;
