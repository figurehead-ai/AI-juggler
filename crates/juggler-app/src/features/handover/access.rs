//! 0.3.2 — a switch checks it can do everything before it changes
//! anything (the maintainer's walk: macOS said "Data Access Blocked" in
//! the middle of a switch, the new session opened on Claude Code's setup
//! screen, and AI Juggler still said "switched"). It reads the
//! conversation it will carry, and asks the target's tool whether a launch
//! there can work ([`juggler_adapter::readiness`]). Nothing is written,
//! but for one step the user's switch takes (0.3.5): a target whose tool is
//! signed in but hasn't done its own first run has it finished, as the tool
//! itself would ([`finish_first_run`]).

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::readiness::{NotReady, probe_read};
use juggler_core::model::Account;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Checks the switch of the conversation in `transcript` into `target`.
///
/// # Errors
///
/// The first thing that would stop it, in words that say what to do.
pub fn check_switch_access(
    state: &Arc<AppState>,
    transcript: &Path,
    target: &Account,
) -> AppResult<()> {
    probe_read(transcript).map_err(|not| refusal(&not, &target.label))?;
    if let Some(adapter) = state.registry.get(&target.harness) {
        match adapter.launch_ready(&target.state_dir) {
            // 0.3.5: the switch finishes it as it goes (`finish_first_run`).
            Ok(()) | Err(NotReady::FirstRun) => {}
            Err(not) => return Err(refusal(&not, &target.label)),
        }
    }
    Ok(())
}

/// 0.3.5: a switch the user started into `target` finishes its tool's
/// first-run setup when that's all that's left (a Claude app profile's
/// folder), as the tool itself would. Whether it did.
///
/// # Errors
///
/// The tool couldn't mark it done; nothing else was changed.
pub fn finish_first_run(state: &Arc<AppState>, target: &Account) -> AppResult<bool> {
    let Some(adapter) = state.registry.get(&target.harness) else {
        return Ok(false);
    };
    if adapter.launch_ready(&target.state_dir) != Err(NotReady::FirstRun) {
        return Ok(false);
    }
    adapter
        .complete_first_run(&target.state_dir)
        .map_err(|error| AppError::Io {
            context: "switch",
            message: format!(
                "Can't continue on {} yet: AI Juggler couldn't finish its first-run setup ({error}). Nothing else was changed.",
                target.label
            ),
        })?;
    Ok(true)
}

/// 0.3.4: what would stop a launch on `target` (its tool's own check),
/// for the switch's target list, so a target a switch would refuse is
/// never offered as ready (the maintainer's walk: personal-max read
/// "Ready", then the switch said it wasn't set up). Read-only.
#[must_use]
pub fn launch_blocker(state: &Arc<AppState>, target: &Account) -> Option<NotReady> {
    state
        .registry
        .get(&target.harness)?
        .launch_ready(&target.state_dir)
        .err()
}

/// What stops it, in a target list's words: what to do, without the
/// switch's "Nothing was changed".
#[must_use]
pub fn blocker_words(not: &NotReady) -> String {
    match not {
        NotReady::Blocked(path) => format!(
            "The system isn't letting AI Juggler read {} (another app's data). Allow AI Juggler in System Settings › Privacy & Security › Files & Folders (or Full Disk Access), then try again.",
            home_short(path)
        ),
        NotReady::NotSetUp(why) => why.clone(),
        NotReady::FirstRun => FIRST_RUN_WORDS.to_owned(),
    }
}

/// 0.3.5: a target whose tool hasn't done its first run, in its row.
pub const FIRST_RUN_WORDS: &str =
    "Its first-run setup isn't done yet; AI Juggler finishes it as it switches, as the tool would.";

/// The words for what stops it.
#[must_use]
pub fn refusal_words(not: &NotReady, target: &str) -> String {
    match not {
        NotReady::Blocked(_) => format!("{} Nothing was changed.", blocker_words(not)),
        NotReady::NotSetUp(why) => {
            format!("Can't continue on {target} yet. {why} Nothing was changed.")
        }
        NotReady::FirstRun => format!("{target}: {FIRST_RUN_WORDS}"),
    }
}

fn refusal(not: &NotReady, target: &str) -> AppError {
    AppError::Io {
        context: "switch",
        message: refusal_words(not, target),
    }
}

/// A path with the home folder said as `~`.
fn home_short(path: &Path) -> String {
    let text = path.display().to_string();
    match juggler_platform::user::home() {
        Some(home) if path.starts_with(&home) => {
            format!(
                "~/{}",
                path.strip_prefix(&home)
                    .map(|rest| rest.display().to_string())
                    .unwrap_or_default()
            )
        }
        _ => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_say_what_to_allow_and_that_nothing_changed() {
        let blocked = refusal_words(
            &NotReady::Blocked("/x/Library/Application Support/Claude/a.jsonl".into()),
            "Personal",
        );
        assert!(
            blocked.contains("Privacy & Security › Files & Folders"),
            "{blocked}"
        );
        assert!(blocked.ends_with("Nothing was changed."));
        let set_up = refusal_words(
            &NotReady::NotSetUp("Claude Code hasn't been set up on this account yet.".into()),
            "Personal",
        );
        assert_eq!(
            set_up,
            "Can't continue on Personal yet. Claude Code hasn't been set up on this account yet. Nothing was changed."
        );
    }
}
