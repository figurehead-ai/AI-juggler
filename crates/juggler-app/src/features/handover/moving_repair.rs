//! AJ-E2328 req 7: the **ownership check** and its **Repair**, beside the
//! move whose path they reuse.
//!
//! A move before 0.5.1 carried a conversation's record but not the app's
//! ownership of its worktree: the old profile kept it, so the new one made
//! a fresh worktree, the old one's cleanup could remove that, and the
//! moved session couldn't write. The check ([`findings`]) asks the tool's
//! adapter for such conversations among the App profiles AI Juggler knows
//! ([`juggler_adapter::moving::InPlaceMove::splits`]) and for worktrees
//! another profile's cleanup has seen (`also_tidied`), and says each in
//! plain words. Repair ([`repair`]) is the move's own path: the folders
//! quiet first, the two profiles quit by their exact process (never by
//! name, never forced), the adapter's plan made again once they closed,
//! the engine's backup, apply and checks, both started again, and the
//! check run again to prove it held. It never writes inside a worktree;
//! a failure before the apps start again is undone at once.
//!
//! The maintainer's ruling (2026-11-23) on one conversation with two
//! worktrees (the app made a fresh one): the stale one's ownership moves
//! too, never dropped (a folder no profile owns is removed by a cleanup,
//! and may hold unpushed work); the row and the result say both are kept.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::adapter::SharedAdapter;
use juggler_adapter::moving::{
    Appended, MoveCheck, MovePlan, MoveRequest, MoveSide, NotesChoice, Split,
};
use juggler_handover::moving::{Phase, Stopped, Transaction};

use super::{
    KnownPlace, MoveSystem, Writing, engine, hold_still, lock, moves_dir, new_id, pause,
    place_name, quiet, refused, start_again, still_down, stop_places,
};
use crate::error::{AppError, AppResult, MoveStop, StopClass};
use crate::features::ownership::InstanceKey;
use crate::state::AppState;

/// What the ownership check found, for Settings › Desktop apps.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OwnershipInfo {
    /// Each app whose conversations move: its version and whether moves
    /// were checked on it.
    pub apps: Vec<MovesCheckedInfo>,
    /// Conversations whose worktree another profile owns (or whose branch
    /// the app has wrong): each one Repair puts right.
    pub findings: Vec<OwnershipFindingInfo>,
    /// Worktrees another profile's cleanup may remove, in words.
    pub tidied: Vec<TidiedInfo>,
    /// Why the check couldn't tell, in words (a format AI Juggler doesn't
    /// know yet); `null` when it could.
    pub unreadable: Option<String>,
}

/// "Claude app 5.11492.5 · moves checked".
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MovesCheckedInfo {
    /// The host registry's id for the app (its icon).
    pub app_id: String,
    /// The app in words (`Claude app`).
    pub app: String,
    /// Its installed version, when it can be read.
    pub version: Option<String>,
    /// Moves (and Repair) were checked on that version.
    pub checked: bool,
}

/// One conversation Repair puts right.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OwnershipFindingInfo {
    /// What Repair takes (the finding's key).
    pub key: String,
    /// The conversation's title, or "A conversation".
    pub title: String,
    /// What is wrong, in plain words.
    pub words: String,
    /// The worktree's folder name.
    pub worktree: String,
    /// The App that holds the conversation (owns the worktree after).
    pub holder: String,
    /// The Apps Repair quits and starts again (those running now).
    pub quits: Vec<String>,
    /// What else to know (two worktrees, another profile's cleanup).
    pub notes: Vec<String>,
}

/// A worktree another profile's cleanup has seen.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TidiedInfo {
    /// The worktree's folder name.
    pub worktree: String,
    /// What to do, in words.
    pub words: String,
}

/// What a Repair did.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OwnershipRepairedInfo {
    /// The outcome, in words.
    pub words: String,
    /// What else to know.
    pub notes: Vec<String>,
}

/// The App profiles of one tool that the check reads: (place id, place),
/// each instance folder once.
struct Hosts {
    adapter: SharedAdapter,
    places: Vec<(String, KnownPlace)>,
}

impl Hosts {
    fn dirs(&self) -> Vec<PathBuf> {
        self.places
            .iter()
            .filter_map(|(_, p)| p.instance_dir.clone())
            .collect()
    }

    fn place_of(&self, dir: &Path) -> Option<&(String, KnownPlace)> {
        let key = InstanceKey::of(dir);
        self.places.iter().find(|(_, p)| {
            p.instance_dir
                .as_deref()
                .is_some_and(|d| InstanceKey::of(d) == key)
        })
    }
}

/// Every tool that moves in place and has desktop apps, with its App
/// profiles AI Juggler knows.
fn hosts_of(state: &Arc<AppState>) -> Vec<Hosts> {
    let registry = &state.registry;
    let book: Vec<(String, KnownPlace)> = crate::features::places::with_book(state, |book| {
        book.places
            .iter()
            .map(|(id, p)| (id.clone(), p.clone()))
            .collect()
    })
    .unwrap_or_default();
    let mut out = Vec::new();
    for id in registry.ids() {
        let Some(adapter) = registry.get(&id) else {
            continue;
        };
        if adapter.in_place_move().is_none() {
            continue;
        }
        let apps: Vec<String> = adapter.host_apps().into_iter().map(|h| h.id).collect();
        let mut seen: Vec<InstanceKey> = Vec::new();
        let mut places = Vec::new();
        for (pid, place) in &book {
            let Some(dir) = place.instance_dir.as_deref() else {
                continue;
            };
            let key = InstanceKey::of(dir);
            if !apps.contains(&place.app_id) || seen.contains(&key) {
                continue;
            }
            seen.push(key);
            places.push((pid.clone(), place.clone()));
        }
        if !places.is_empty() {
            out.push(Hosts { adapter, places });
        }
    }
    out
}

/// The app's word alone (`Claude app`).
fn app_word(state: &Arc<AppState>, place: &KnownPlace) -> String {
    place_name(
        state,
        &KnownPlace {
            instance: None,
            account_id: None,
            ..place.clone()
        },
    )
}

/// The version of the app `place` runs, as the move's gate reads it.
fn version_of(place: &KnownPlace) -> Option<String> {
    juggler_platform::apps::bundle_version(&place.bundle)
}

fn name_of(state: &Arc<AppState>, hosts: &Hosts, dir: &Path) -> String {
    hosts.place_of(dir).map_or_else(
        || "another profile".to_owned(),
        |(_, p)| place_name(state, p),
    )
}

/// Names in words: "a", "a and b", "a, b and c".
fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// "<profile> also tidies this folder; keep <App> running in it or commit
/// your work" (the maintainer's words).
fn tidied_words(state: &Arc<AppState>, hosts: &Hosts, by: &[PathBuf], owner: &Path) -> String {
    let names: Vec<String> = by.iter().map(|d| name_of(state, hosts, d)).collect();
    format!(
        "{} also {} this folder; keep {} running in it or commit your work",
        and_list(&names),
        if names.len() == 1 { "tidies" } else { "tidy" },
        name_of(state, hosts, owner)
    )
}

/// What is wrong with `split`, in plain words.
fn finding_words(state: &Arc<AppState>, hosts: &Hosts, split: &Split) -> String {
    let holder = name_of(state, hosts, &split.holder);
    let name = &split.worktree;
    let mut words = if split.owner == split.holder {
        format!("{holder} has another branch for its worktree {name} than the one it's on")
    } else {
        format!(
            "Its worktree {name} still belongs to {}, so {holder} can't write there and may make a fresh one",
            name_of(state, hosts, &split.owner)
        )
    };
    if let (Some(branch), true) = (&split.branch, split.branch_differs) {
        if split.owner == split.holder {
            words = format!("{words} ({branch})");
        } else {
            words = format!("{words}; it's on {branch} now");
        }
    }
    words
}

/// The profiles `split` involves: (place id, place), the owner first,
/// each once.
fn involved(hosts: &Hosts, split: &Split) -> Vec<(String, KnownPlace)> {
    let mut out: Vec<(String, KnownPlace)> = Vec::new();
    for dir in [&split.owner, &split.holder] {
        if let Some(one) = hosts.place_of(dir)
            && !out.iter().any(|(id, _)| *id == one.0)
        {
            out.push(one.clone());
        }
    }
    out
}

/// AJ-E2328 req 7, req 9 (c): what the ownership check finds now, with
/// each app's version and whether moves were checked on it. Reads only.
#[must_use]
pub fn findings(state: &Arc<AppState>, system: &dyn MoveSystem) -> OwnershipInfo {
    let mut info = OwnershipInfo::default();
    for hosts in hosts_of(state) {
        let Some(mover) = hosts.adapter.in_place_move() else {
            continue;
        };
        // One row per app: the version its profiles run.
        for (_, place) in &hosts.places {
            if info.apps.iter().any(|a| a.app_id == place.app_id) {
                continue;
            }
            let version = version_of(place);
            if let Some(checked) = mover.moves_checked(version.as_deref()) {
                info.apps.push(MovesCheckedInfo {
                    app_id: place.app_id.clone(),
                    app: app_word(state, place),
                    version,
                    checked,
                });
            }
        }
        let dirs = hosts.dirs();
        match mover.splits(&dirs) {
            Ok(splits) => {
                for split in splits {
                    let mut notes = Vec::new();
                    if split.other_worktree.is_some() {
                        notes.push(TWO_WORKTREES.to_owned());
                    }
                    if !split.also_swept_by.is_empty() {
                        notes.push(tidied_words(
                            state,
                            &hosts,
                            &split.also_swept_by,
                            &split.holder,
                        ));
                    }
                    let quits = involved(&hosts, &split)
                        .iter()
                        .filter(|(id, _)| system.running(state, id).is_some())
                        .map(|(_, p)| place_name(state, p))
                        .collect();
                    info.findings.push(OwnershipFindingInfo {
                        key: split.key.clone(),
                        title: split
                            .title
                            .clone()
                            .unwrap_or_else(|| "A conversation".to_owned()),
                        words: finding_words(state, &hosts, &split),
                        worktree: split.worktree.clone(),
                        holder: name_of(state, &hosts, &split.holder),
                        quits,
                        notes,
                    });
                }
            }
            Err(why) => info.unreadable = Some(format!("{why}; update AI Juggler to check it")),
        }
        if let Ok(tidied) = mover.also_tidied(&dirs) {
            for one in tidied {
                info.tidied.push(TidiedInfo {
                    words: tidied_words(state, &hosts, &one.by, &one.owner),
                    worktree: one.worktree,
                });
            }
        }
    }
    info
}

/// The repair's note when a conversation has two worktrees.
const TWO_WORKTREES: &str = "This conversation has two worktrees; both are kept";

/// A repair, resolved: the split as found now, the profiles it involves,
/// and the request the engine records (the owner as the source, the
/// holder as the target).
pub(super) struct Repairing {
    pub split: Split,
    pub adapter: SharedAdapter,
    pub places: Vec<(String, KnownPlace)>,
    pub request: MoveRequest,
    pub accounts: Vec<String>,
    pub version: Option<String>,
}

/// Finds the split `key` among the profiles AI Juggler knows now.
fn resolve(state: &Arc<AppState>, key: &str) -> AppResult<Repairing> {
    for hosts in hosts_of(state) {
        let Some(mover) = hosts.adapter.in_place_move() else {
            continue;
        };
        let splits = mover.splits(&hosts.dirs()).map_err(refused)?;
        let Some(split) = splits.into_iter().find(|s| s.key == key) else {
            continue;
        };
        let places = involved(&hosts, &split);
        let side = |dir: &Path| {
            let place = hosts.place_of(dir).map(|(_, p)| p);
            MoveSide {
                tool_dir: place
                    .and_then(|p| crate::features::places::tool_folder(state, p))
                    .unwrap_or_default(),
                host_dir: Some(dir.to_path_buf()),
                version: place.and_then(version_of),
            }
        };
        let request = MoveRequest {
            conversation: split.conversation.clone(),
            source: side(&split.owner),
            target: side(&split.holder),
            notes: NotesChoice::KeepTarget,
        };
        let version = request.target.version.clone();
        let mut accounts: Vec<String> = places
            .iter()
            .filter_map(|(_, p)| p.account_id.clone())
            .collect();
        accounts.dedup();
        return Ok(Repairing {
            split,
            adapter: hosts.adapter.clone(),
            places,
            request,
            accounts,
            version,
        });
    }
    Err(refused(
        "this worktree's ownership is already right; nothing to repair",
    ))
}

/// Repairs the ownership `key` names (the user's click, after the sheet
/// that names the profiles it quits).
///
/// # Errors
///
/// Refused before anything changed (with why), or a failure after which
/// the repair was undone (with why).
pub fn repair(state: &Arc<AppState>, key: &str) -> AppResult<OwnershipRepairedInfo> {
    let repairing = resolve(state, key)?;
    run_repair(state, &repairing, &super::Real::quiet())
}

fn plan_again(repairing: &Repairing) -> AppResult<MovePlan> {
    let mover = repairing
        .adapter
        .in_place_move()
        .ok_or_else(|| refused("this tool has nothing to repair"))?;
    let plan = mover.repair(&repairing.split, repairing.version.as_deref());
    if plan.can_move() {
        Ok(plan)
    } else {
        Err(refused(juggler_handover::moving::words(&plan.refusals)))
    }
}

/// The check, run again: the split is gone.
fn held(repairing: &Repairing) -> MoveCheck {
    let hosts = [
        repairing.split.owner.clone(),
        repairing.split.holder.clone(),
    ];
    let found = repairing.adapter.in_place_move().map(|m| m.splits(&hosts));
    let pass =
        matches!(&found, Some(Ok(splits)) if !splits.iter().any(|s| s.key == repairing.split.key));
    MoveCheck {
        what: format!(
            "the worktree {} belongs to the profile that holds its conversation",
            repairing.split.worktree
        ),
        pass,
    }
}

/// The repair of resolved `repairing` (the move's path).
#[allow(
    clippy::too_many_lines,
    reason = "the repair, step by step, as the move's"
)]
pub(super) fn run_repair(
    state: &Arc<AppState>,
    repairing: &Repairing,
    system: &dyn MoveSystem,
) -> AppResult<OwnershipRepairedInfo> {
    let plan = plan_again(repairing)?;
    let mover = repairing
        .adapter
        .in_place_move()
        .ok_or_else(|| refused("this tool has nothing to repair"))?;
    let writing = Writing {
        mover,
        dirs: [&repairing.request.source, &repairing.request.target]
            .into_iter()
            .map(|s| s.tool_dir.clone())
            .filter(|d| !d.as_os_str().is_empty())
            .collect(),
        conversations: vec![repairing.split.conversation.clone()],
    };
    let folder = moves_dir(state);
    let places = &repairing.places;
    let mut why_not = juggler_handover::moving::preflight(&plan, &folder);
    why_not.extend(quiet(
        state,
        (places, &repairing.request),
        &repairing.accounts,
        system,
        "Repair",
        &writing,
    ));
    if !why_not.is_empty() {
        return Err(refused(juggler_handover::moving::words(&why_not)));
    }
    let _lock = lock(state, &plan.roots)?;
    let ids: Vec<String> = places.iter().map(|(id, _)| id.clone()).collect();
    let now = juggler_core::model::Timestamp::now().millis();
    let move_id = new_id(now);
    // Held in every AI Juggler before anything quits (AJ-E2320).
    let _hold = crate::features::places::MoveHold::take(state, &ids, &move_id).map_err(refused)?;
    let mut txn = Transaction::begin(
        &folder,
        &move_id,
        repairing.adapter.id().as_str(),
        repairing.request.clone(),
        plan,
        None,
        now,
    )
    .map_err(engine)?;
    txn.remember_places(ids, repairing.accounts.clone())
        .map_err(engine)?;
    let stopped_places = match stop_places(
        state,
        (places, &repairing.request),
        system,
        "repaired",
        &writing,
    ) {
        Ok(stopping) => stopping.stopped,
        Err(error) => {
            let _ = txn.undo(
                &Stopped::checked(),
                juggler_core::model::Timestamp::now().millis(),
            );
            let _ = txn.drop_backups();
            return Err(error);
        }
    };
    let stopped = Stopped::checked();
    let fail = |txn: &mut Transaction, words: String, detail: String| -> AppError {
        let untouched = txn.manifest.phase == Phase::Planned;
        if !untouched && let Err(started) = still_down(state, &stopped_places, system) {
            let _ = txn.stop_with(&format!("{detail}; {started}"));
            // AJ-E2319: typed, naming the repair for Undo by its id.
            let mut stop = MoveStop::new(
                StopClass::NeedsYou,
                format!(
                    "{words}, and {started}, so AI Juggler left both profiles as they are now. Quit it, then Undo the repair from the switch screen."
                ),
                detail,
            );
            stop.move_id = Some(txn.manifest.id.clone());
            stop.unchanged = false;
            return stop.into();
        }
        match txn.undo(&stopped, juggler_core::model::Timestamp::now().millis()) {
            Ok(_) => {
                let _ = txn.drop_backups();
                start_again(state, &stopped_places, &repairing.request, system);
                let mut stop = MoveStop::new(
                    StopClass::Transient,
                    format!("{words}, so nothing was changed. Try again in a moment."),
                    detail,
                );
                stop.move_id = Some(txn.manifest.id.clone());
                stop.into()
            }
            Err(error) => {
                let _ = txn.stop_with(&format!("{detail}; undo: {error}"));
                pause(state, &stopped_places);
                let mut stop = MoveStop::new(
                    StopClass::Bug,
                    format!(
                        "{words}. Undoing it didn't finish; both profiles stay closed, and Undo from the switch screen finishes it."
                    ),
                    format!("{detail}; undo: {error}"),
                );
                stop.move_id = Some(txn.manifest.id.clone());
                stop.unchanged = false;
                stop.into()
            }
        }
    };
    // The apps write their files when they quit: plan again.
    let again = plan_again(repairing).and_then(|plan| txn.replan(plan).map_err(engine));
    if let Err(error) = again {
        let words = "the worktree's files changed while the apps closed".to_owned();
        return Err(fail(&mut txn, words, error.to_string()));
    }
    if let Err(started) = still_down(state, &stopped_places, system) {
        return Err(fail(&mut txn, started.clone(), started));
    }
    let watched: Vec<PathBuf> = txn.touched().into_iter().map(Path::to_path_buf).collect();
    if let Err(moving) = hold_still(&watched, system) {
        let detail = moving
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let words = "the app's files kept changing after it closed".to_owned();
        return Err(fail(&mut txn, words, detail));
    }
    if let Err(error) = txn.back_up(&stopped) {
        let words = "AI Juggler couldn't back up the app's files".to_owned();
        return Err(fail(&mut txn, words, error.to_string()));
    }
    if let Err(started) = still_down(state, &stopped_places, system) {
        return Err(fail(&mut txn, started.clone(), started));
    }
    if let Err(error) = txn.apply(&stopped, 0) {
        let words = "the worktree's ownership couldn't be moved".to_owned();
        return Err(fail(&mut txn, words, error.to_string()));
    }
    let cold = match txn.verify_cold(vec![held(repairing)]) {
        Ok(report) => report,
        Err(error) => {
            let words = "AI Juggler couldn't record its checks of the repair".to_owned();
            return Err(fail(&mut txn, words, error.to_string()));
        }
    };
    if !cold.passed() {
        let words = "a check before reopening failed".to_owned();
        return Err(fail(&mut txn, words, cold.failures().join("; ")));
    }
    start_again(state, &stopped_places, &repairing.request, system);
    // Started again: the app may rewrite its files as it starts; the check
    // runs once more.
    let after = txn
        .verify_opened(&|_| Appended::Bookkeeping, vec![held(repairing)])
        .is_ok_and(|report| report.passed());
    let holder = places
        .iter()
        .find(|(_, p)| {
            p.instance_dir
                .as_deref()
                .is_some_and(|d| InstanceKey::of(d) == InstanceKey::of(&repairing.split.holder))
        })
        .map_or_else(|| "its profile".to_owned(), |(_, p)| place_name(state, p));
    let words = if after {
        format!(
            "Repaired: the worktree {} belongs to {holder} now. Its files weren't touched.",
            repairing.split.worktree
        )
    } else {
        format!(
            "Repaired, but the check after {holder} started again didn't hold; open Settings › Desktop apps to look again."
        )
    };
    Ok(OwnershipRepairedInfo {
        words,
        notes: txn.manifest.plan.notes.clone(),
    })
}

#[cfg(test)]
#[path = "moving_repair_tests.rs"]
mod tests;
