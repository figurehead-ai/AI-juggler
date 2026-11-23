//! AJ-E2317 (the review's A2.2, A2.3, E5 and E6): a move's two sides, by
//! evidence and by the user's pick, never by account. Before, both were
//! the account's "running profile, else the one seen last": with two app
//! profiles on one account, a move quit, wrote into and opened the wrong
//! one.
//!
//! - **The source** is the app profile whose own records list the
//!   conversation (its holder), whatever profile runs on the account. Two
//!   profiles listing it can't be told apart: the move is refused, in
//!   words (a Bug class once AJ-E2319 types refusals). None: the
//!   conversation lives in a terminal's tool folder.
//! - **The target** is the App the user picked in the switch list, by its
//!   place id, carried from the click through `switch_move_execute` to
//!   every step after (actions target by id), never found again from the
//!   account.
//! - **Each profile's folder link** is checked against what its own
//!   sessions show (AJ-E2327's evidence) before the move and before each
//!   of its restarts ([`link_contradiction`]): a contradiction refuses up
//!   front, naming the folder each side uses, and a profile is never
//!   started again on a folder its sessions don't use.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::live::HostApp;
use juggler_adapter::moving::{MoveRequest, MoveSide, NotesChoice, Refusal};
use juggler_core::model::Account;

use super::{MoveSystem, Sides, place_name, refused, shown, tool_version};
use crate::error::{AppError, AppResult};
use crate::features::conversations::places::HostRegistry;
use crate::features::ownership::InstanceKey;
use crate::features::places::book::KnownPlace;
use crate::features::places::with_book;
use crate::state::AppState;

/// Whether `host` takes a session handed to it (a link, and a record of
/// it): an app a move can quit, write into and open in.
fn takes_sessions(host: &HostApp) -> bool {
    host.handoff_link.is_some() && host.handoff_record.is_some()
}

/// The remembered app profiles whose own records (`lists`: the ids an
/// instance folder's records name) list `conversation`, each once however
/// its folder is spelled, in the book's order.
pub(super) fn holders(
    state: &Arc<AppState>,
    conversation: &str,
    lists: impl Fn(&Path) -> Vec<String>,
) -> Vec<(String, KnownPlace)> {
    let hosts = HostRegistry::of(&state.registry);
    let places: Vec<(String, KnownPlace)> = with_book(state, |book| {
        book.places
            .iter()
            .filter(|(_, place)| hosts.by_id(&place.app_id).is_some_and(takes_sessions))
            .map(|(id, place)| (id.clone(), place.clone()))
            .collect()
    })
    .unwrap_or_default();
    let mut seen: Vec<InstanceKey> = Vec::new();
    let mut out = Vec::new();
    for (id, place) in places {
        let Some(dir) = place.instance_dir.as_deref() else {
            continue;
        };
        let key = InstanceKey::of(dir);
        if seen.contains(&key) || !lists(dir).iter().any(|one| one == conversation) {
            continue;
        }
        seen.push(key);
        out.push((id, place));
    }
    out
}

/// Names in words: "a", "a and b", "a, b and c".
fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The source profile of a conversation on `account`, of its `holders`:
/// none (a terminal's folder), the one, or why the move can't tell.
pub(super) fn source_of(
    state: &Arc<AppState>,
    holders: Vec<(String, KnownPlace)>,
    account: &Account,
) -> Result<Option<(String, KnownPlace)>, Refusal> {
    let names: Vec<String> = holders
        .iter()
        .map(|(_, place)| place_name(state, place))
        .collect();
    let mut holders = holders.into_iter();
    match (holders.next(), holders.next()) {
        (None, _) => Ok(None),
        (Some(one), None) if one.1.account_id.as_deref() == Some(account.id.as_str()) => {
            Ok(Some(one))
        }
        (Some(_), None) => Err(Refusal::user_can_clear(format!(
            "{} lists this conversation, but AI Juggler doesn't have it on {}; set its folder in Settings › Desktop apps, and Move becomes available",
            and_list(&names),
            account.label
        ))),
        (Some(_), Some(_)) => Err(Refusal::setup(format!(
            "{} all list this conversation, so AI Juggler can't tell which one it is in, and moves nothing; Copy carries it",
            and_list(&names)
        ))),
    }
}

/// The target of a move to `account`: the App the user picked (`picked`,
/// its place id), when the conversation is in an app profile (`held`);
/// none for a terminal's folder, or when no App was picked (the planner
/// then says a move between an app and a terminal isn't built).
pub(super) fn target_of(
    state: &Arc<AppState>,
    picked: Option<&str>,
    account: &Account,
    held: bool,
) -> Result<Option<(String, KnownPlace)>, Refusal> {
    let Some(id) = picked.filter(|_| held) else {
        return Ok(None);
    };
    let place = with_book(state, |book| book.places.get(id).cloned())
        .ok()
        .flatten()
        .ok_or_else(|| {
            Refusal::setup("AI Juggler doesn't know the App you picked any more; pick it again")
        })?;
    let name = place_name(state, &place);
    let hosts = HostRegistry::of(&state.registry);
    if !hosts.by_id(&place.app_id).is_some_and(takes_sessions) {
        return Err(Refusal::setup(format!(
            "{name} can't take a moved conversation"
        )));
    }
    if place.account_id.as_deref() != Some(account.id.as_str()) {
        return Err(Refusal::setup(format!(
            "{name} isn't on {}, so AI Juggler moves nothing into it",
            account.label
        )));
    }
    Ok(Some((id.to_owned(), place)))
}

/// Resolves both sides of moving `session_id` to `target_profile_id`'s
/// account, into the App `target_place_id` the user picked.
pub(super) fn sides(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
    target_place_id: Option<&str>,
) -> AppResult<Sides> {
    let (_profile_b, account_b) =
        crate::commands::find_profile_with_account_arc(state, target_profile_id)?;
    let (source, account_a) = super::super::source::handover_source(state, session_id)?;
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
    let held = holders(state, &conversation, |dir| {
        adapter
            .host_family()
            .and_then(|family| family.conversations(dir))
            .unwrap_or_default()
            .into_iter()
            .map(|record| record.id)
            .collect()
    });
    let picked = source_of(state, held, &account_a).and_then(|source_place| {
        let target = target_of(state, target_place_id, &account_b, source_place.is_some())?;
        Ok((source_place, target))
    });
    let (source_place, target_place, stop) = match picked {
        Ok((source, target)) => (source, target, None),
        Err(stop) => (None, None, Some(stop)),
    };
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
        source_live: crate::features::switch::execute::source_is_live(state, &source),
        source_place,
        target_place,
        source_label: account_a.label.clone(),
        target_label: account_b.label.clone(),
        accounts: vec![
            account_a.id.as_str().to_owned(),
            account_b.id.as_str().to_owned(),
        ],
        stop,
    })
}

/// AJ-E2317 → AJ-E2327: the evidence-first link check
/// ([`crate::features::places::link::folder_mismatch`]): the tool folder
/// profile `place_id`'s own evidence shows it uses (its running sessions'
/// pid files, its records, its app's declared folder), when that isn't
/// the folder it's linked to. `None`: nothing proves otherwise. Called
/// before a move, before each of its restarts, and by Undo.
#[must_use]
pub(super) fn link_contradiction(state: &Arc<AppState>, place_id: &str) -> Option<PathBuf> {
    crate::features::places::link::folder_mismatch(state, place_id).map(|(folder, _)| folder)
}

/// The tool folder of `request`'s side whose app profile is `place`.
fn linked_dir<'r>(place: &KnownPlace, request: &'r MoveRequest) -> Option<&'r Path> {
    let dir = InstanceKey::of(place.instance_dir.as_deref()?);
    [&request.source, &request.target]
        .into_iter()
        .find(|side| {
            side.host_dir
                .as_deref()
                .is_some_and(|host| InstanceKey::of(host) == dir)
        })
        .map(|side| side.tool_dir.as_path())
}

/// Why `place`'s folder link can't be trusted, in words naming the folder
/// each side uses; `None`: nothing contradicts it.
fn mismatch(
    state: &Arc<AppState>,
    id: &str,
    place: &KnownPlace,
    request: &MoveRequest,
    system: &dyn MoveSystem,
) -> Option<String> {
    let linked = linked_dir(place, request)?;
    let uses = system.link_mismatch(state, id, place, linked)?;
    Some(format!(
        "{}'s sessions use {}, but AI Juggler has it on {}",
        place_name(state, place),
        shown(&uses),
        shown(linked)
    ))
}

/// Why `places`' folder links stop `action` (Move, Undo) before anything
/// quits: each one its own sessions contradict needs the user (its folder
/// set in Settings › Desktop apps).
pub(super) fn links_hold(
    state: &Arc<AppState>,
    places: &[(String, KnownPlace)],
    request: &MoveRequest,
    system: &dyn MoveSystem,
    action: &str,
) -> Vec<Refusal> {
    places
        .iter()
        .filter_map(|(id, place)| mismatch(state, id, place, request, system))
        .map(|why| {
            Refusal::user_can_clear(format!(
                "{why}; set its folder in Settings › Desktop apps, and {action} becomes available"
            ))
        })
        .collect()
}

/// Starts `place` (`id`) again on its own folders, unless its sessions
/// now contradict its folder link: then it stays closed, and says why.
pub(super) fn start_checked(
    state: &Arc<AppState>,
    id: &str,
    place: &KnownPlace,
    request: &MoveRequest,
    system: &dyn MoveSystem,
) -> Result<(), String> {
    if let Some(why) = mismatch(state, id, place, request, system) {
        return Err(format!(
            "{why}, so AI Juggler left it closed; set its folder in Settings › Desktop apps, then open it"
        ));
    }
    system.start(state, place)
}
