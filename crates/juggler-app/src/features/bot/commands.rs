//! Bot's commands (D-136): ask, read the conversation a page at a time,
//! find in it, start a New topic, clear it, and say "This is wrong" about
//! a written answer. Asking writes only the turn to this Mac's own
//! conversation; nothing is sent, switched, stopped or approved. Watches
//! are written only by the Watches commands, on the user's click on a
//! confirm card (AJ-E3815).

use std::sync::Arc;

use juggler_core::storage::memory::{Actor, NewEvent, NewTurn, SecretScan, Stream};

use super::dto::{
    BotAskInput, BotCountInfo, BotFoundInfo, BotOutcome, BotPageInfo, BotTopicInfo, BotTurnInfo,
};
use super::history::{Meta, cites_json, shown_text, turn_info};
use super::lookup::look_up;
use super::question::read;
use super::world::{World, gather};
use crate::error::{AppError, AppResult};
use crate::features::ai::dto::{AiJob, AiJobState};
use crate::features::ai::memory::RedactionSecrets;
use crate::state::AppState;

/// The longest question taken, in characters (ADR-0037 §1.6).
pub const QUESTION_MAX: usize = 600;

fn memory_error(error: juggler_core::storage::memory::MemoryError) -> AppError {
    match error {
        juggler_core::storage::memory::MemoryError::Core(core) => AppError::from(core),
        other => AppError::Validation {
            subject: "bot",
            message: other.to_string(),
        },
    }
}

/// Whether Bot's written answers run now: the router says Bot is ready,
/// which needs the user's Beta switch on and a model that passed.
fn written_on(state: &AppState) -> bool {
    crate::features::ai::service::jobs_now(state)
        .ok()
        .and_then(|(jobs, _)| jobs.into_iter().find(|info| info.job == AiJob::Bot))
        .is_some_and(|info| info.state == AiJobState::Ready)
}

/// Asks Bot a question: read by code, looked up in AI Juggler's records,
/// kept in this Mac's conversation; a Beta written answer above the rows
/// only when it passes the check.
///
/// # Errors
///
/// An empty question; storage failures.
pub fn bot_ask_impl(state: &Arc<AppState>, input: &BotAskInput) -> AppResult<BotTurnInfo> {
    let question: String = input.question.trim().chars().take(QUESTION_MAX).collect();
    if question.is_empty() {
        return Err(AppError::Validation {
            subject: "bot",
            message: "empty".to_owned(),
        });
    }
    // A key in "Remember …" is refused on the user's own words, before
    // they are masked (the Bot review's L1: the mask hid it from the check).
    let secret = read(&question)
        .remember
        .is_some_and(|words| RedactionSecrets.finds_secret(&words));
    let question = RedactionSecrets.mask(&question);
    let world = gather(state)?;
    let (read_as, used) = super::memory::with_aliases(state, &world, &question);
    let reading = read(&read_as);
    let scope = input.scope.clone();
    let mut meta = Meta {
        scope: scope.clone(),
        lang: reading.lang,
        used: used.iter().map(|(info, _)| info.key.clone()).collect(),
        ..Meta::default()
    };
    if let Some(words) = reading.remember.as_deref() {
        return remember_turn(state, &world, &question, words, secret, meta);
    }
    let (outcome, rows, written) = if reading.help {
        (BotOutcome::Help, Vec::new(), None)
    } else if reading.out_of_scope {
        (BotOutcome::OutOfScope, Vec::new(), None)
    } else {
        let found = look_up(&reading, &world, scope.as_ref());
        meta.unknown = found.unknown;
        meta.shortcut = found.shortcut;
        meta.looked = Some(super::dto::BotLooked {
            memories: u32::try_from(meta.used.len()).unwrap_or(0),
            ..found.looked
        });
        meta.total = found.total;
        if found.unknown.is_some() {
            (BotOutcome::DontKnow, found.rows, None)
        } else if !found.rows.is_empty() && written_on(state) {
            let (written, _why) =
                super::written::answer(state, &question, reading.lang, &found.rows, &world.removed);
            let outcome = if written.is_some() {
                BotOutcome::Answered
            } else {
                BotOutcome::Rows
            };
            (outcome, found.rows, written)
        } else {
            (BotOutcome::Rows, found.rows, None)
        }
    };
    if let Some(written) = &written {
        meta.written_model = Some(written.model_name.clone());
        meta.written_cites = written.parts.iter().map(|p| p.cites.clone()).collect();
    }
    let entities = serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_owned());
    let cites = cites_json(&rows);
    let shown = shown_text(written.as_ref());
    let kept = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let kept = storage
            .chat()
            .append(&NewTurn {
                at_ms: world.now_ms,
                question: &question,
                outcome: outcome.code(),
                shown: shown.as_deref(),
                cites: &cites,
                entities: &entities,
                has_agent_words: rows.iter().any(|r| r.quote.is_some()),
            })
            .map_err(memory_error)?;
        let mut event = NewEvent::new(world.now_ms, Stream::Chat, "turn", Actor::User);
        event.subject = Some(("chat_turn", kept.id.clone()));
        event.payload = serde_json::json!({ "outcome": outcome.code(), "rows": rows.len() });
        let _ = storage.ai_log().append(&event, None);
        // Each memory the question used is linked to the turn, so its
        // forget reaches this turn too (ADR-0037 §1.5 step 3).
        for (_, subject) in &used {
            storage
                .memory()
                .link_turn(&kept.id, subject)
                .map_err(memory_error)?;
        }
        kept
    };
    let mut info = turn_info(&kept, None, &|_| None);
    info.used = used.into_iter().map(|(info, _)| info).collect();
    // The turn just asked shows its rows as found, agent words included.
    info.rows = rows;
    info.written = written;
    info.live = true;
    Ok(info)
}

/// "Remember …": the turn kept, then the memory written from it (so a
/// forget clears the message it came from), then its echo noted.
fn remember_turn(
    state: &Arc<AppState>,
    world: &World,
    question: &str,
    words: &str,
    secret: bool,
    mut meta: Meta,
) -> AppResult<BotTurnInfo> {
    let secret = secret || RedactionSecrets.finds_secret(words);
    let outcome = if secret {
        BotOutcome::Refused
    } else {
        BotOutcome::Remembered
    };
    let kept = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .chat()
            .append(&NewTurn {
                at_ms: world.now_ms,
                question,
                outcome: outcome.code(),
                shown: None,
                cites: "[]",
                entities: &serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_owned()),
                has_agent_words: false,
            })
            .map_err(memory_error)?
    };
    let mut kept_memory = None;
    if !secret {
        kept_memory = super::memory::remember(state, world, words, &kept.id)?;
        meta.memory = kept_memory.as_ref().map(|m| m.key.clone());
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .chat()
            .annotate(
                &kept.id,
                &serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_owned()),
            )
            .map_err(memory_error)?;
    }
    let mut info = turn_info(&kept, None, &|_| None);
    info.memory = kept_memory;
    Ok(info)
}

/// A page of the conversation, newest first, each row looked up again.
///
/// # Errors
///
/// Storage failures.
pub fn bot_page_impl(
    state: &Arc<AppState>,
    before: Option<(i64, &str)>,
    limit: u32,
) -> AppResult<BotPageInfo> {
    let page = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage.chat().page(before, limit).map_err(memory_error)?
    };
    let world: Option<World> = if page.turns.is_empty() {
        None
    } else {
        Some(gather(state)?)
    };
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let memory = |key: &str| super::memory::current(&storage, key);
    Ok(BotPageInfo {
        turns: page
            .turns
            .iter()
            .map(|turn| turn_info(turn, world.as_ref(), &memory))
            .collect(),
        topics: page
            .topics
            .into_iter()
            .map(|t| BotTopicInfo {
                id: t.id,
                started_ms: t.started_ms,
                first: t.first,
            })
            .collect(),
        more: page.more,
    })
}

/// Starts a New topic.
///
/// # Errors
///
/// Storage failures.
pub fn bot_new_topic_impl(state: &Arc<AppState>) -> AppResult<BotTopicInfo> {
    let now = state.notifier.clock().now;
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let topic = storage.chat().new_topic(now).map_err(memory_error)?;
    Ok(BotTopicInfo {
        id: topic.id,
        started_ms: topic.started_ms,
        first: topic.first,
    })
}

/// The turns holding `words`, oldest first.
///
/// # Errors
///
/// Storage failures.
pub fn bot_find_impl(state: &Arc<AppState>, words: &str) -> AppResult<Vec<BotFoundInfo>> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    Ok(storage
        .chat()
        .find(words)
        .map_err(memory_error)?
        .into_iter()
        .map(|(id, at_ms)| BotFoundInfo { id, at_ms })
        .collect())
}

/// What the clear sheet says: the questions that go, the notes and
/// watches that stay.
///
/// # Errors
///
/// Storage failures.
pub fn bot_count_impl(state: &Arc<AppState>) -> AppResult<BotCountInfo> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let count = storage.chat().count().map_err(memory_error)?;
    let memory = storage.memory();
    let of = |kind: juggler_core::storage::memory::Kind| {
        memory
            .current_of_kind(kind, None, 10_000)
            .map_or(0, |rows| u32::try_from(rows.len()).unwrap_or(u32::MAX))
    };
    Ok(BotCountInfo {
        questions: count.turns,
        since_ms: count.since_ms,
        memories: of(juggler_core::storage::memory::Kind::Preference)
            + of(juggler_core::storage::memory::Kind::Note),
        watches: of(juggler_core::storage::memory::Kind::StandingRequest),
    })
}

/// Clears the whole conversation from this Mac (notes and watches stay).
///
/// # Errors
///
/// Storage failures.
pub fn bot_clear_impl(state: &Arc<AppState>) -> AppResult<u32> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let cleared = storage.chat().clear().map_err(memory_error)?;
    let now = state.notifier.clock().now;
    let mut event = NewEvent::new(now, Stream::Chat, "cleared", Actor::User);
    event.payload = serde_json::json!({ "turns": cleared.turns });
    let _ = storage.ai_log().append(&event, None);
    // A backup staged before the clear would bring the words back.
    crate::backup::drop_staged(&storage, &state.data_dir)?;
    Ok(cleared.turns)
}

/// "This is wrong" about a turn's written answer: kept on this Mac only,
/// in the AI log (ADR-0036 §1.8).
///
/// # Errors
///
/// No such turn; storage failures.
pub fn bot_wrong_impl(state: &Arc<AppState>, turn: &str) -> AppResult<()> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let chat = storage.chat();
    let Some(kept) = chat.turn(turn).map_err(memory_error)? else {
        return Err(AppError::Validation {
            subject: "bot",
            message: "no_such_turn".to_owned(),
        });
    };
    let mut meta: Meta = serde_json::from_str(&kept.entities).unwrap_or_default();
    meta.flagged = true;
    chat.annotate(
        turn,
        &serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_owned()),
    )
    .map_err(memory_error)?;
    let now = state.notifier.clock().now;
    let mut event = NewEvent::new(now, Stream::Feedback, "wrong", Actor::User);
    event.subject = Some(("chat_turn", turn.to_owned()));
    storage
        .ai_log()
        .append(&event, None)
        .map_err(memory_error)?;
    Ok(())
}

/// Asks Bot (see [`bot_ask_impl`]).
///
/// # Errors
///
/// As [`bot_ask_impl`].
#[tauri::command]
pub async fn bot_ask(
    state: tauri::State<'_, Arc<AppState>>,
    input: BotAskInput,
) -> AppResult<BotTurnInfo> {
    crate::ipc::blocking(state.inner(), "Bot", move |state| {
        bot_ask_impl(state, &input)
    })
    .await
}

/// A page of the conversation (see [`bot_page_impl`]).
///
/// # Errors
///
/// As [`bot_page_impl`].
#[tauri::command]
pub async fn bot_page(
    state: tauri::State<'_, Arc<AppState>>,
    before_ms: Option<i64>,
    before_id: Option<String>,
    limit: Option<u32>,
) -> AppResult<BotPageInfo> {
    crate::ipc::blocking(state.inner(), "Bot", move |state| {
        bot_page_impl(
            state,
            before_ms.zip(before_id.as_deref()),
            limit.unwrap_or(20),
        )
    })
    .await
}

/// Starts a New topic (see [`bot_new_topic_impl`]).
///
/// # Errors
///
/// As [`bot_new_topic_impl`].
#[tauri::command]
pub async fn bot_new_topic(state: tauri::State<'_, Arc<AppState>>) -> AppResult<BotTopicInfo> {
    crate::ipc::blocking(state.inner(), "Bot", bot_new_topic_impl).await
}

/// Finds words in the conversation (see [`bot_find_impl`]).
///
/// # Errors
///
/// As [`bot_find_impl`].
#[tauri::command]
pub async fn bot_find(
    state: tauri::State<'_, Arc<AppState>>,
    words: String,
) -> AppResult<Vec<BotFoundInfo>> {
    crate::ipc::blocking(state.inner(), "Bot", move |state| {
        bot_find_impl(state, &words)
    })
    .await
}

/// What the clear sheet names (see [`bot_count_impl`]).
///
/// # Errors
///
/// As [`bot_count_impl`].
#[tauri::command]
pub async fn bot_count(state: tauri::State<'_, Arc<AppState>>) -> AppResult<BotCountInfo> {
    crate::ipc::blocking(state.inner(), "Bot", bot_count_impl).await
}

/// Clears the conversation (see [`bot_clear_impl`]).
///
/// # Errors
///
/// As [`bot_clear_impl`].
#[tauri::command]
pub async fn bot_clear(state: tauri::State<'_, Arc<AppState>>) -> AppResult<u32> {
    crate::ipc::blocking(state.inner(), "Bot", bot_clear_impl).await
}

/// "This is wrong" (see [`bot_wrong_impl`]).
///
/// # Errors
///
/// As [`bot_wrong_impl`].
#[tauri::command]
pub async fn bot_wrong(state: tauri::State<'_, Arc<AppState>>, turn: String) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "Bot", move |state| {
        bot_wrong_impl(state, &turn)
    })
    .await
}

/// What Bot remembers (see [`super::memory::list`]).
///
/// # Errors
///
/// As [`super::memory::list`].
#[tauri::command]
pub async fn bot_memories(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<super::memory::BotMemoriesInfo> {
    crate::ipc::blocking(state.inner(), "Bot", super::memory::list).await
}

/// Forgets one memory (see [`super::memory::forget`]).
///
/// # Errors
///
/// As [`super::memory::forget`].
#[tauri::command]
pub async fn bot_forget(state: tauri::State<'_, Arc<AppState>>, key: String) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "Bot", move |state| {
        super::memory::forget(state, &key)
    })
    .await
}
