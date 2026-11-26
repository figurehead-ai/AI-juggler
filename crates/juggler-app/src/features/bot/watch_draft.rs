//! AJ-E3830 (D-149): Bot's watch request fills the Watches form. When the
//! words ask to be told and the router serves the Watches job (its
//! verdict), the draft is made in two small calls
//! ([`crate::features::watches::draft`]) and the turn carries it: the
//! window opens the form filled in, and only the user's click adds the
//! watch. Otherwise Bot answers as it would.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::dto::{BotRowKind, BotTurnInfo};
use super::world::World;
use crate::features::ai::dto::AiJob;
use crate::features::ai::engine::Stop;
use crate::features::ai::run::LiveJob;
use crate::features::watches::commands::{watches_catalogue_impl, watches_preview_impl};
use crate::features::watches::draft::{self, Ask, Candidate, Outcome, WatchDrafted};
use crate::features::watches::model::WatchTarget;
use crate::features::watches::words::Lang;
use crate::state::AppState;

/// How long the two calls may take, retries included.
const WAIT: Duration = Duration::from_secs(45);
/// The most sessions offered as targets (the newest first).
const SESSIONS_MAX: usize = 24;

fn offered(state: &Arc<AppState>, target: &WatchTarget) -> Vec<String> {
    watches_catalogue_impl(state, target, Lang::En)
        .map(|entries| entries.into_iter().map(|e| e.code).collect())
        .unwrap_or_default()
}

/// The targets the request may be about: the newest sessions, their
/// projects and Apps, and the Accounts.
fn candidates(state: &Arc<AppState>, world: &World) -> Vec<Candidate> {
    let mut sessions: Vec<_> = world
        .sessions
        .iter()
        .filter(|row| row.kind == BotRowKind::Session && !row.gone && !row.name_hidden)
        .collect();
    sessions.sort_by_key(|row| std::cmp::Reverse(row.at_ms.unwrap_or(0)));
    sessions.truncate(SESSIONS_MAX);
    let mut out = Vec::new();
    let mut projects: Vec<(String, String)> = Vec::new();
    let mut apps: Vec<(String, String)> = Vec::new();
    for row in &sessions {
        let target = WatchTarget::Session(row.id.clone());
        out.push(Candidate {
            offered: offered(state, &target),
            target,
            name: row.title.clone(),
            aliases: Vec::new(),
            app: row.app_label.clone(),
            project: row.project.clone(),
        });
        if let (Some(id), Some(name)) = (&row.project_id, &row.project)
            && !projects.iter().any(|(p, _)| p == id)
        {
            projects.push((id.clone(), name.clone()));
        }
        if let (Some(id), Some(name)) = (&row.app_id, &row.app_label)
            && !apps.iter().any(|(a, _)| a == id)
        {
            apps.push((id.clone(), name.clone()));
        }
    }
    for (id, name) in apps {
        let target = WatchTarget::App(id);
        out.push(Candidate {
            offered: offered(state, &target),
            target,
            name,
            app: None,
            project: None,
            aliases: Vec::new(),
        });
    }
    for (id, name) in projects {
        let target = WatchTarget::Project(id);
        out.push(Candidate {
            offered: offered(state, &target),
            target,
            name,
            app: None,
            project: None,
            aliases: Vec::new(),
        });
    }
    for row in world
        .accounts
        .iter()
        .filter(|row| row.kind == BotRowKind::Account)
    {
        let target = WatchTarget::Account(row.id.clone());
        out.push(Candidate {
            offered: offered(state, &target),
            target,
            name: row.title.clone(),
            app: None,
            project: None,
            aliases: Vec::new(),
        });
    }
    out
}

/// The form to open for `question`, or `None` when Bot answers as usual
/// (not a watch request, or the Watches job isn't served).
pub fn drafted(state: &Arc<AppState>, world: &World, question: &str) -> Option<WatchDrafted> {
    if !draft::is_watch_request(question) {
        return None;
    }
    let live = LiveJob {
        service: crate::features::ai::service::shared(),
        state,
        job: AiJob::Watches,
    };
    if !live.served() {
        return None;
    }
    drafted_with(&live, state, world, question)
}

/// [`drafted`] through `model` (a test's fake).
pub fn drafted_with(
    model: &dyn crate::features::ai::run::JobModel,
    state: &Arc<AppState>,
    world: &World,
    question: &str,
) -> Option<WatchDrafted> {
    let cands = candidates(state, world);
    let day = 86_400_000;
    let local = world.now_ms + world.offset_hours * 3_600_000;
    let ask = Ask {
        request: question,
        candidates: &cands,
        midnight_ms: world.now_ms - local.rem_euclid(day),
        now_ms: world.now_ms,
    };
    let check = |d: &crate::features::watches::commands::WatchDraftInfo| {
        watches_preview_impl(state, d, Lang::En)
            .ok()
            .and_then(|p| p.problem)
    };
    let stop = Stop::at(Instant::now() + WAIT);
    match draft::draft(model, &ask, &stop, &check).0 {
        Outcome::NotAWatch => None,
        Outcome::Drafted(d) => Some(d),
    }
}

/// The turn's one line, in code's words (the window says it in the
/// user's language from the same kind).
#[must_use]
pub fn shown(d: &WatchDrafted) -> String {
    match d {
        WatchDrafted::Filled { .. } => {
            "I filled in the Watches form. Check it and click to add the watch.".to_owned()
        }
        WatchDrafted::Pick { .. } => {
            "Two match that name: the Watches form asks which one.".to_owned()
        }
        WatchDrafted::CantWatch { asked, draft } => {
            if draft.is_some() {
                format!(
                    "I can't watch for “{asked}”. The Watches form is open with the nearest I can."
                )
            } else {
                format!("I can't watch for “{asked}”. The Watches form is open.")
            }
        }
        WatchDrafted::Form => {
            "I couldn't fill the form from your words. The Watches form is open.".to_owned()
        }
    }
}

/// Sets the turn id on the draft (the watch records which Bot turn it
/// came from once the user adds it) and puts it on the turn.
pub fn attach(info: &mut BotTurnInfo, mut d: WatchDrafted) {
    let turn = info.id.clone();
    match &mut d {
        WatchDrafted::Filled { draft } | WatchDrafted::Pick { draft } => {
            draft.chat_turn = Some(turn);
        }
        WatchDrafted::CantWatch {
            draft: Some(draft), ..
        } => draft.chat_turn = Some(turn),
        WatchDrafted::CantWatch { draft: None, .. } | WatchDrafted::Form => {}
    }
    info.watch_draft = Some(d);
}

#[cfg(test)]
#[path = "watch_draft_tests.rs"]
mod tests;
