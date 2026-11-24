//! 0.5.1 F98 (F-PL1-15 a): Quit…, a running desktop App closed from AI
//! Juggler and left closed. Before, Restart… quit it and started it at
//! once and Forget asked the user to quit it first, so a Claude app
//! profile or the Codex app couldn't be closed from here.
//!
//! One quit for every caller: `ask_to_quit` is Restart's quit too
//! (`places::restart`), and it signals as Move does: the very process
//! looked at, by its pid and start, checked again right before the signal
//! (`ownership::signal::quit_checked`, AJ-E2320), through the launcher's
//! [`Starter::quit`](super::start::Starter::quit)
//! (`juggler_platform::apps::quit`, the call Move's system makes).
//!
//! Keep alive never starts an App the user just quit: the quit pauses it
//! for that App until its next Open or Start, as a Stop from AI Juggler
//! does (`places::stopped_here`).

use std::sync::Arc;

use super::{
    QuitWait, Restarting, added_and_free, changed, running_now, start, wait_until, with_book,
};
use crate::features::ownership;
use crate::state::AppState;

/// Keep alive's pause after a quit here, in the App's line's words
/// ("Keep alive paused: you quit it in AI Juggler").
pub const QUIT_HERE: &str = "you quit it in AI Juggler";

/// Asks `label`'s process `pid` (started at `since`) to quit, through its
/// identity checked right before the signal (AJ-E2320, as Move quits),
/// calling `before` once it is that process and before the signal, then
/// waits for it to end. `Ok(true)`: it ended; `Ok(false)`: it still runs
/// after `wait`.
///
/// # Errors
///
/// In words: it isn't the process it was a moment ago (nothing was
/// signalled), or the system refused the signal.
pub(super) fn ask_to_quit(
    starter: &dyn start::Starter,
    label: &str,
    pid: u32,
    since: Option<i64>,
    wait: QuitWait,
    before: impl FnOnce(),
) -> Result<bool, String> {
    let changed =
        || format!("{label} isn't the process it was a moment ago, so nothing was quit. Try again");
    let looked = starter
        .identify(pid)
        .filter(|looked| looked.started_in(since))
        .ok_or_else(changed)?;
    before();
    ownership::signal::quit_checked(looked, &|pid| starter.identify(pid), &|pid| {
        starter.quit(pid)
    })
    .map_err(|why| match why {
        ownership::signal::NotAsked::NotThatProcess => changed(),
        ownership::signal::NotAsked::Refused(why) => format!("Couldn't quit {label}: {why}"),
    })?;
    Ok(wait_until(wait, || !starter.alive(pid, since)))
}

/// Quits `id`, a running app or profile, after the user confirmed it (its
/// confirm says what stops), and leaves it closed: keep alive pauses for
/// it until its next Open or Start. Never while a move holds it or a
/// restart of it runs.
///
/// Returns what it did, in words ("Quit Claude app · Work").
///
/// # Errors
///
/// In words: it isn't remembered, a move holds it, it is restarting or
/// quitting already, it isn't the process it was, the system refused, or
/// it didn't quit within `wait` (it may be asking something of its own).
pub fn quit(
    state: &Arc<AppState>,
    id: &str,
    starter: &dyn start::Starter,
    wait: QuitWait,
) -> Result<String, String> {
    let (place, label) = added_and_free(state, id, |_| ())?;
    let Some(_guard) = Restarting::begin(state, id) else {
        return Err(format!("{label} is restarting or quitting already"));
    };
    let Some((pid, since)) = running_now(state, id) else {
        // Closed already: it stays closed all the same.
        pause_keep_alive(state, id);
        changed(state);
        return Ok(format!("{label} is closed"));
    };
    let before = place.paused.clone();
    let ended = ask_to_quit(starter, &label, pid, since, wait, || {
        pause_keep_alive(state, id);
    });
    let said = match ended {
        Ok(true) => Ok(format!("Quit {label}")),
        // Asked: it may still close once its own question is answered, so
        // keep alive stays paused.
        Ok(false) => Err(format!(
            "{label} didn't quit within {} seconds: it may be asking you something. Look at it, or quit it in Activity Monitor",
            wait.limit.as_secs()
        )),
        // Nothing was signalled: keep alive as it was.
        Err(why) => {
            let _ = with_book(state, |book| {
                if let Some(place) = book.places.get_mut(id) {
                    place.paused.clone_from(&before);
                }
            });
            Err(why)
        }
    };
    changed(state);
    said
}

/// Keep alive leaves `id` closed until its next Open or Start.
fn pause_keep_alive(state: &Arc<AppState>, id: &str) {
    let _ = with_book(state, |book| {
        if let Some(place) = book.places.get_mut(id).filter(|place| place.keep_alive) {
            place.paused = Some(QUIT_HERE.to_owned());
        }
    });
}

#[cfg(test)]
#[path = "quit_tests.rs"]
mod tests;
