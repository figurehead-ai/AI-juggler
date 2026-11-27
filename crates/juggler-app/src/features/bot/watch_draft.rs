//! AJ-E3830 (D-149; 0.5.2's ruling): Bot pre-fills the Watches form; the
//! user saves. Every request Bot reads as a watch request opens the form
//! with what code verified, from code's reading and the model's labels as
//! code checks them ([`crate::features::watches::draft::prefill`]); a
//! field nothing verified stays empty for the user, and only the user's
//! click adds the watch. With every field verified and the user's
//! Watches Beta on (Settings › AI › Models), Bot shows its confirm card
//! instead. The labels ride Bot's own model; with no model served, code
//! alone fills what it reads.
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::dto::{BotRowKind, BotTurnInfo};
use super::world::World;
use crate::features::ai::dto::AiJob;
use crate::features::ai::engine::Stop;
use crate::features::ai::jobs;
use crate::features::ai::run::LiveJob;
use crate::features::schedule::remind_draft;
use crate::features::watches::commands::{watches_catalogue_impl, watches_preview_impl};
use crate::features::watches::draft::prefill::{self, PrefillField, PrefillWhy, WatchPrefill};
use crate::features::watches::draft::{self, Ask, Candidate, WatchDrafted};
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
/// (not a watch request, a question about when something happened, or the
/// Watches job switched off by an update).
pub fn drafted(state: &Arc<AppState>, world: &World, question: &str) -> Option<WatchDrafted> {
    // AJ-E2027: a reminder is code's alone (no model call): its time and
    // words from the request, what it is about from the user's names.
    if remind_draft::is_reminder_request(question) && state.schedules.registry().is_some() {
        return Some(reminder(state, world, question));
    }
    if !draft::is_watch_request(question)
        || asks_when_it_happened(question)
        || !crate::features::flags::is_on(&jobs::WATCHES_ON)
    {
        return None;
    }
    let live = LiveJob {
        service: crate::features::ai::service::shared(),
        state,
        job: AiJob::Bot,
    };
    let card = card_on(state);
    if live.served() {
        drafted_with(&live, state, world, question, card)
    } else {
        drafted_with(&NoModel, state, world, question, card)
    }
}

/// No model: every call fails, so code alone fills what it reads.
struct NoModel;

impl crate::features::ai::run::JobModel for NoModel {
    fn generate(
        &self,
        _request: &crate::features::ai::engine::Request,
        _stop: &Stop,
    ) -> Result<crate::features::ai::engine::Draft, crate::features::ai::run::AiFailure> {
        Err(crate::features::ai::run::AiFailure::Refused)
    }
}

/// The user's Watches Beta: Bot's confirm card for a request code fully
/// verified (off unless turned on in Settings › AI › Models).
fn card_on(state: &AppState) -> bool {
    state.storage.lock().is_ok_and(|storage| {
        storage
            .app_settings()
            .get(&jobs::beta_key(crate::features::ai::models::Job::Watches))
            .ok()
            .flatten()
            .as_deref()
            == Some("1")
    })
}

/// "When did api finish?", "tell me when it was started": a question about
/// the past, answered as a question, never a watch.
#[must_use]
pub fn asks_when_it_happened(question: &str) -> bool {
    let w = draft::words(question);
    let pair = |a: &str, b: &[&str]| {
        w.windows(2)
            .any(|p| p[0] == a && b.contains(&p[1].as_str()))
    };
    pair("when", &["did", "was", "were", "had", "has", "have"])
        || pair(
            "wann",
            &["hat", "war", "waren", "wurde", "wurden", "ist", "hatte"],
        )
        || pair("what", &["time"]) && w.iter().any(|x| x == "did")
}

/// The reminder draft for `question`: what it is about when the request
/// names one target the resolver settles.
pub fn reminder(state: &Arc<AppState>, world: &World, question: &str) -> WatchDrafted {
    let cands = candidates(state, world);
    let found = crate::features::watches::resolve::resolve(question, &cands);
    let about = match found.candidates.as_slice() {
        [one] if found.picker.is_empty() => {
            cands.get(*one).map(|c| (c.target.clone(), c.name.clone()))
        }
        _ => None,
    };
    WatchDrafted::Remind {
        reminder: remind_draft::draft(
            question,
            world.now_ms,
            &crate::features::schedule::zone::local(),
            about,
        ),
    }
}

/// [`drafted`] through `model` (a test's fake); `card`: the user's
/// Watches Beta.
pub fn drafted_with(
    model: &dyn crate::features::ai::run::JobModel,
    state: &Arc<AppState>,
    world: &World,
    question: &str,
    card: bool,
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
    let (mut prefill, _) = prefill::prefill(model, &ask, &stop, &check)?;
    prefill.card &= card;
    Some(WatchDrafted::Prefilled { prefill })
}

/// The fields left, in words ("how long and quiet hours").
fn left_words(left: &[PrefillField]) -> String {
    let names: Vec<&str> = left
        .iter()
        .map(|f| match f {
            PrefillField::Target => "what to watch",
            PrefillField::Condition => "what to watch for",
            PrefillField::Lifetime => "how long",
            PrefillField::QuietHours => "quiet hours",
        })
        .collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The pre-fill's one line, in code's words (the window says it in the
/// user's language from the same fields).
#[must_use]
pub fn prefill_line(p: &WatchPrefill) -> String {
    let filled = p.left.len() < 4
        && !(p.left.contains(&PrefillField::Target) && p.left.contains(&PrefillField::Condition));
    let left = left_words(&p.left);
    match (p.why, p.asked.as_deref()) {
        (PrefillWhy::Verified, _) if p.card => {
            "Here is the watch. Check it, then click Watch.".to_owned()
        }
        (PrefillWhy::Verified, _) => {
            "I filled in the Watches form. Check it and click Watch.".to_owned()
        }
        (PrefillWhy::Several, _) => format!(
            "More than one matches that name, so the Watches form asks which one{}.",
            if p.left.len() > 1 { format!("; I left {left} for you too") } else { String::new() }
        ),
        (PrefillWhy::CantWatch, Some(asked)) => {
            let near = if p.nearest.is_some() {
                " The nearest it can watch is marked."
            } else {
                ""
            };
            format!("I can't watch for “{asked}”. The Watches form is open with what I could fill in.{near}")
        }
        (PrefillWhy::NoCondition, _) if filled => {
            "Your words don't say what to watch for. The Watches form is open with the rest filled in.".to_owned()
        }
        _ if !filled => {
            "I couldn't fill the form from your words. The Watches form is open.".to_owned()
        }
        (_, Some(asked)) => {
            format!("I couldn't read “{asked}”, so I left {left} for you. The Watches form is open.")
        }
        _ => format!("I left {left} for you. The Watches form is open."),
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
        WatchDrafted::Prefilled { prefill } => prefill_line(prefill),
        WatchDrafted::Remind { reminder } => {
            if reminder.at_ms.is_some() {
                "I drafted the reminder. Check the time, then set it.".to_owned()
            } else {
                "I drafted the reminder. Pick a time, then set it.".to_owned()
            }
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
        WatchDrafted::Remind { reminder } => reminder.chat_turn = Some(turn),
        WatchDrafted::Prefilled { prefill } => prefill.draft.chat_turn = Some(turn),
        WatchDrafted::CantWatch { draft: None, .. } | WatchDrafted::Form => {}
    }
    info.watch_draft = Some(d);
}

#[cfg(test)]
#[path = "watch_draft_tests.rs"]
mod tests;
