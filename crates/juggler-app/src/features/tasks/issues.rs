//! AJ-E1906: the issue a task comes from — its title and state, looked
//! up only when the user clicks, through the user's own GitHub CLI (`gh`,
//! with its own sign-in: AI Juggler never reads it). No sync: the title
//! and state are kept as last looked up, with when.
//!
//! - Only a GitHub issue or pull request link is looked up; a Linear link
//!   shows its id from the link itself; any other `https` link opens as
//!   it is.
//! - Offline, or with `gh` missing or signed out, the task keeps its link
//!   and says it couldn't look it up. Nothing is retried by itself.
//! - AI Juggler itself makes no network request here: `gh` does, as the
//!   user's tool, with the owner, the repository and the number only.

use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::dto::TaskDetail;
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// How long a look-up may take.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// Where `gh` usually is on a Mac.
const PLACES: [&str; 3] = ["/opt/homebrew/bin/gh", "/usr/local/bin/gh", "/usr/bin/gh"];

/// Said when there's no `gh` at all.
pub const NO_GH: &str =
    "Looking it up needs GitHub's own command line, gh, on this computer; the link still opens";

/// The user's own `gh`, when it's there and safe to run (ours or root's,
/// and no one but us and the Mac's administrators can change it:
/// [`juggler_platform::file_trust::entry_trusted`]). Tests point it
/// elsewhere ([`super::Tasks::set_gh`]).
#[must_use]
pub fn gh(state: &AppState) -> Option<PathBuf> {
    gh_or_why(state).ok()
}

/// [`gh`], or why it can't be used, in words: missing ([`NO_GH`]), or
/// there but not safe to run, naming why. A `gh` that's there is never
/// called missing.
///
/// # Errors
///
/// The words.
pub fn gh_or_why(state: &AppState) -> Result<PathBuf, String> {
    use juggler_platform::file_trust::{Untrusted, executable_trust};
    // No one to trust it for (the system can't say who we are): no gh.
    let me = juggler_platform::user::current().map_err(|_| NO_GH.to_owned())?;
    let chosen = state.tasks.gh.lock().ok().and_then(|slot| slot.clone());
    let candidates: Vec<PathBuf> = match chosen {
        Some(path) => vec![path],
        None => PLACES.iter().map(PathBuf::from).collect(),
    };
    let mut refused = None;
    for path in candidates {
        match executable_trust(&path, me) {
            Ok(resolved) => return Ok(resolved),
            Err(Untrusted::Missing) => {}
            Err(why) => {
                refused.get_or_insert_with(|| {
                    format!(
                        "gh is at {}, but AI Juggler won't run it: {}. The link still opens",
                        path.display(),
                        why.words()
                    )
                });
            }
        }
    }
    Err(refused.unwrap_or_else(|| NO_GH.to_owned()))
}

/// Whether a word is a safe GitHub owner or repository name.
fn plain(word: &str) -> bool {
    !word.is_empty()
        && word.len() <= 100
        && !word.starts_with('-')
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Runs `gh api repos/<owner>/<repo>/issues/<n>` for the title and state;
/// `None` when it can't (offline, signed out, no `gh`, too slow).
fn ask_gh(gh: &PathBuf, owner: &str, repo: &str, number: u64) -> Option<(String, String)> {
    let mut child = std::process::Command::new(gh)
        .args([
            "api",
            &format!("repos/{owner}/{repo}/issues/{number}"),
            "--jq",
            "[.title, (if .pull_request.merged_at then \"merged\" else .state end)] | @tsv",
        ])
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                // poll: a look-up the user clicked, bounded by TIMEOUT.
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    let mut out = String::new();
    child
        .stdout
        .take()?
        .take(16 * 1024)
        .read_to_string(&mut out)
        .ok()?;
    let (title, state) = out.trim_end().split_once('\t')?;
    let title = juggler_core::messaging::one_line(title, 200);
    let state = match state.trim() {
        "open" => "open",
        "closed" => "closed",
        "merged" => "merged",
        _ => return None,
    };
    (!title.is_empty()).then(|| (title, state.to_owned()))
}

/// The user's click: looks the task's issue up, keeps its title and
/// state, and logs it.
///
/// # Errors
///
/// [`AppError::Refused`] in words: no link, not a GitHub one, no `gh`,
/// or it couldn't look it up (offline or signed out).
pub fn look_up(state: &Arc<AppState>, task_id: &str) -> AppResult<TaskDetail> {
    let task = lock_storage_arc(state)?
        .tasks()
        .get(task_id)?
        .ok_or_else(|| AppError::Refused("That task isn't there any more".to_owned()))?;
    let link = task
        .external
        .as_ref()
        .and_then(|x| juggler_core::tasks::external_link(&x.url).ok())
        .ok_or_else(|| AppError::Refused("This task has no link to look up".to_owned()))?;
    let Some((owner, repo, number)) = link.github.filter(|(o, r, _)| plain(o) && plain(r)) else {
        return Err(AppError::Refused(
            "Only GitHub issues and pull requests are looked up; the link still opens".to_owned(),
        ));
    };
    let gh = gh_or_why(state).map_err(AppError::Refused)?;
    let (title, issue_state) = ask_gh(&gh, &owner, &repo, number).ok_or_else(|| {
        AppError::Refused(
            "Couldn't look it up: offline, or gh isn't signed in to it. The link still opens"
                .to_owned(),
        )
    })?;
    {
        let storage = lock_storage_arc(state)?;
        storage
            .tasks()
            .set_external(&task.id, Some(&title), Some(&issue_state))?;
        storage.tasks().log(
            &task.pool_id,
            &task.id,
            super::Actor::user().who(),
            "issue_looked_up",
            Some(&format!("{issue_state} · {title}")),
        )?;
    }
    state.tasks.notify(&task.pool_id);
    super::views::detail(state, &task.id, None)
}

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn only_plain_names_reach_gh() {
        assert!(plain("figurehead") && plain("ai-juggler.app") && plain("my_repo"));
        for bad in ["", "-x", "a/b", "a b", "$(x)", "a;b"] {
            assert!(!plain(bad), "{bad}");
        }
    }
}
