//! AJ-E3831 (D-154): a finished turn's AI summary in the feed, what the
//! turn came to against what was asked ([`super::line`]), written by the
//! model the router serves the feed job on. An opt-in Beta (v14's
//! re-measure, 2026-11-27: preferred in 69 % of blind pairs, but 19 % of
//! its lines carried a critical error, over D-130's 1 %), so the user
//! turns it on in Settings › AI › Models; off, nothing here runs.
//!
//! The agent's own words (the plain excerpt, [`super::line::plain`]) come
//! with every answer, so the post shows them one tap away, and in place
//! of the line when every draft was refused. "This is wrong" records the
//! line on this Mac only ([`crate::features::review::commands`]).
//!
//! The turn is the review's: the same gates (Last messages on, an App
//! whose turn ends AI Juggler sees) and the same transcript.

use std::sync::{Arc, LazyLock};
use std::time::Instant;

use serde::Serialize;

use super::line::{self, Options};
use crate::error::AppResult;
use crate::features::ai::{
    dto::AiJob, engine::Stop, kept::Kept, models::Job, run::LiveJob, service,
};
use crate::features::conversations::last_messages;
use crate::features::review::commands::{LINES_WAIT, record_wrong};
use crate::features::review::dto::ReviewAsk;
use crate::features::review::read::{locate, sees_turn_ends};
use crate::state::AppState;

/// Answers kept (a turn's line is written once).
const KEPT_MAX: usize = 128;

/// A turn's summary for its feed post.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FeedLineInfo {
    /// `shown` (a line), `plain` (every draft refused or no engine: the
    /// agent's words only), or why there is neither: `off` (Last messages
    /// off, or the job not served), `none`, `unseen`, `unreadable`.
    pub state: String,
    /// The turn's key ("This is wrong" names it).
    pub turn: String,
    /// The model's line.
    pub line: Option<String>,
    /// The agent's own words (the plain excerpt), always when read.
    pub excerpt: Option<String>,
    /// The model that wrote the line.
    pub model: Option<String>,
}

impl FeedLineInfo {
    fn without(state: &str) -> Self {
        Self {
            state: state.to_owned(),
            ..Self::default()
        }
    }
}

/// What the model wrote for a turn, by its key: the line and the model's
/// name, or `None` when every draft was refused.
static KEPT: LazyLock<Kept<String, Option<(String, String)>>> =
    LazyLock::new(|| Kept::new(KEPT_MAX));

fn key_of(path: &std::path::Path, ask: &ReviewAsk, reply: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .chain(reply.as_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}:t{}", ask.until_ms.unwrap_or(0))
}

/// The summary of the turn `ask` names: the agent's words at once, the
/// model's line when the router serves the feed job (its flag, a `beta`
/// verdict and its Beta on, an engine ready); a refused line leaves the
/// agent's words alone (`plain`).
#[must_use]
pub fn feed_line_impl(state: &Arc<AppState>, ask: &ReviewAsk) -> FeedLineInfo {
    let asked = Instant::now();
    if !last_messages::enabled(state) {
        return FeedLineInfo::without("off");
    }
    let Some((path, tool, desktop)) = locate(state, ask) else {
        return FeedLineInfo::without("none");
    };
    if !sees_turn_ends(state, &tool, desktop) {
        return FeedLineInfo::without("unseen");
    }
    let Some(view) = line::read_at(&path, ask.until_ms, ask.ended.as_deref(), None) else {
        return FeedLineInfo::without("unreadable");
    };
    let turn = key_of(&path, ask, view.reply());
    let mut info = FeedLineInfo {
        state: "plain".to_owned(),
        turn: turn.clone(),
        excerpt: line::plain(&view),
        ..FeedLineInfo::default()
    };
    let service = service::shared();
    let Ok(routed) = service::route_on(service, state, Job::Feed) else {
        "off".clone_into(&mut info.state);
        return info;
    };
    let kept = KEPT.get(&turn);
    let done = kept.unwrap_or_else(|| {
        let live = LiveJob {
            service,
            state,
            job: AiJob::Feed,
        };
        // The prompt each engine was measured with (harness v14): the
        // built-in model's, or a downloaded model's with its JSON tail.
        let options = if routed.engine.is_some_and(|engine| !engine.downloads()) {
            Options::APPLE
        } else {
            Options::LOCAL
        };
        let (shown, attempts) = line::line(&live, &view, options, &Stop::at(asked + LINES_WAIT));
        for (n, attempt) in attempts.iter().enumerate() {
            tracing::info!(
                target: "feed",
                attempt = n,
                verdict = attempt.refused.as_deref().unwrap_or("kept"),
                "feed summary"
            );
        }
        let done = shown.map(|shown| (shown, routed.name.clone()));
        KEPT.keep(turn, done.clone());
        done
    });
    if let Some((shown, model)) = done {
        "shown".clone_into(&mut info.state);
        info.line = Some(shown);
        info.model = Some(model);
    }
    info
}

/// The summary (see [`feed_line_impl`]).
///
/// # Errors
///
/// None of its own; a task that can't finish.
#[tauri::command]
pub async fn feed_line(
    state: tauri::State<'_, Arc<AppState>>,
    ask: ReviewAsk,
) -> AppResult<FeedLineInfo> {
    // Nothing waits on it: the post shows first, the line after.
    crate::ipc::blocking_with(
        state.inner(),
        "feed summary",
        crate::ipc::Wait::Job(LINES_WAIT),
        move |state| Ok(feed_line_impl(state, &ask)),
    )
    .await
}

/// "This is wrong" about a feed summary, or its Undo: kept on this Mac
/// only, in the AI log.
///
/// # Errors
///
/// Storage failures.
#[tauri::command]
pub async fn feed_line_wrong(
    state: tauri::State<'_, Arc<AppState>>,
    turn: String,
    line: String,
    wrong: Option<bool>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "feed summary", move |state| {
        record_wrong(state, "feed_line", &turn, &line, wrong.unwrap_or(true))
    })
    .await
}
