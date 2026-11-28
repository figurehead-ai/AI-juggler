//! Bot's commands (D-136): ask, read the conversation a page at a time,
//! find in it, start a New topic, clear it, and say "This is wrong" about
//! a written answer. Asking writes only the turn to this Mac's own
//! conversation; nothing is sent, switched, stopped or approved. Watches
//! are written only by the Watches commands, on the user's click on a
//! confirm card (AJ-E3815).

use std::sync::Arc;

use juggler_core::storage::memory::{Actor, NewEvent, NewTurn, SecretScan, Stream};

use super::dto::{
    BotAskInput, BotCountInfo, BotFoundInfo, BotOutcome, BotPageInfo, BotPastReminder,
    BotTopicInfo, BotTopicUsageInfo, BotTurnInfo,
};
use super::history::{Meta, cites_json, shown_text, turn_info};
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

/// The current topic's turns, newest first (at most `limit`): none when
/// a New topic was started after the newest turn.
fn topic_turns(
    state: &AppState,
    limit: u32,
) -> AppResult<Vec<juggler_core::storage::memory::Turn>> {
    let page = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage.chat().page(None, limit).map_err(memory_error)?
    };
    let Some(newest) = page.turns.first() else {
        return Ok(Vec::new());
    };
    let latest_topic = page.topics.iter().max_by_key(|t| t.started_ms);
    if latest_topic.is_some_and(|t| t.id != newest.topic && t.started_ms >= newest.at_ms) {
        return Ok(Vec::new());
    }
    let topic = newest.topic.clone();
    Ok(page
        .turns
        .into_iter()
        .take_while(|turn| turn.topic == topic)
        .collect())
}

/// 0.5.2 G31: the scope the topic's newest turn was asked with (`None`
/// in a New topic, or when it was asked about everything).
fn last_scope(state: &AppState) -> Option<super::dto::BotScope> {
    let newest = topic_turns(state, 1).ok()?.into_iter().next()?;
    serde_json::from_str::<Meta>(&newest.entities).ok()?.scope
}

/// The current topic: a New topic started after the newest turn, else
/// the newest turn's (`None` before the first question).
fn current_topic(state: &AppState) -> Option<String> {
    let storage = state.storage.lock().ok()?;
    let page = storage.chat().page(None, 1).ok()?;
    let newest = page.turns.first();
    let started = page
        .topics
        .iter()
        .filter(|t| newest.is_none_or(|n| t.started_ms >= n.at_ms))
        .max_by_key(|t| t.started_ms)
        .map(|t| t.id.clone());
    started.or_else(|| newest.map(|t| t.topic.clone()))
}

/// The topic's last two exchanges Bot answered, oldest first (ADR-0034
/// §6.7): never a redacted turn, never an answer marked wrong
/// (AJ-E1548 #2).
fn history(state: &AppState) -> AppResult<Vec<(String, String)>> {
    let mut out: Vec<(String, String)> = topic_turns(state, 12)?
        .into_iter()
        .filter(|turn| !turn.redacted)
        .filter_map(|turn| {
            let meta: Meta = serde_json::from_str(&turn.entities).unwrap_or_default();
            let shown = turn.shown.filter(|s| !s.is_empty())?;
            (!meta.flagged).then_some((turn.question, shown))
        })
        .take(2)
        .collect();
    out.reverse();
    Ok(out)
}

/// Asks Bot a question (D-144 in full, AJ-E3825): the model answers every
/// question ([`super::answer`]), through the app's started engine.
///
/// # Errors
///
/// An empty question; storage failures.
pub fn bot_ask_impl(state: &Arc<AppState>, input: &BotAskInput) -> AppResult<BotTurnInfo> {
    let live = super::model::Live {
        service: crate::features::ai::service::shared(),
        state,
    };
    bot_ask_with(state, input, &live)
}

/// [`bot_ask_impl`] through `model` (a test's fake, or AJ-E3829's real
/// check on a service of its own).
///
/// # Errors
///
/// An empty question; storage failures.
#[expect(
    clippy::too_many_lines,
    reason = "one turn in order: read, remember or answer, keep, link (AJ-E3832 added the world model's two steps)"
)]
pub fn bot_ask_with(
    state: &Arc<AppState>,
    input: &BotAskInput,
    model: &dyn super::model::BotModel,
) -> AppResult<BotTurnInfo> {
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
    if let Some(drafted) =
        super::watch_draft::drafted(state, &world, &question, input.scope.as_ref())
    {
        return watch_turn(state, &world, &question, drafted);
    }
    // 0.5.2 G59: from a post, the user's own words are asked (the turn
    // keeps all it said); the post's record is the chip's, by its key.
    let asked = super::question::after_post(&question);
    let (read_as, used) = super::memory::with_aliases(state, &world, asked);
    let reading = read(&read_as);
    // 0.5.2 G31: the chip is the ask's own (the window drops it once
    // asked); a question that refers back keeps the topic's last scope
    // ("and why did it fail?"), one that doesn't asks about everything.
    let scope = input.scope.clone().or_else(|| {
        super::question::refers_back(asked)
            .then(|| last_scope(state))
            .flatten()
    });
    let mut meta = Meta {
        scope: scope.clone(),
        lang: reading.lang,
        used: used.iter().map(|(info, _)| info.key.clone()).collect(),
        ..Meta::default()
    };
    // 0.5.2 G29: "call api the payments agent" is a Remember too.
    let alias = (reading.remember.is_none() && super::memory::names_alias(&world, asked))
        .then(|| asked.to_owned());
    if let Some(words) = reading.remember.as_deref().or(alias.as_deref()) {
        return remember_turn(state, &world, &question, words, secret, meta);
    }
    let memory: Vec<_> = used.iter().map(|(info, _)| info.clone()).collect();
    // 0.5.2 G16: the topic's last exchanges, and the topic's seed for the
    // world model, go only with a question that refers back to them; one
    // that doesn't is read on its own, so a refusal or a grounded answer
    // before it never makes it grounded ("3+5" after "what needs me").
    let follow_up = super::question::refers_back(asked);
    let history = if follow_up {
        history(state)?
    } else {
        Vec::new()
    };
    // ADR-0038 §5: the world model's section, rebuilt for this question,
    // seeded from what the look-up finds, the words and (for a follow-up)
    // this topic.
    let topic = current_topic(state).filter(|_| follow_up);
    let looked = super::lookup::look_up(&reading, &world, scope.as_ref());
    // 0.5.2 G53 (RW1 W4): only what the question names seeds it. A
    // question that names nothing gets every open record from the look-up
    // (those aren't matches, as `answer` reads them); seeded with them, a
    // plain "what is 3+5" carried an open session into its prompt and the
    // model took it for a question about the user's work.
    let seeds: &[super::dto::BotRowInfo] = if scope.is_some() || reading.names_work() {
        &looked.rows
    } else {
        &[]
    };
    let connected = crate::features::ai::world::assemble::assemble(
        state,
        &crate::features::ai::world::assemble::Ask {
            question: asked,
            world: &world,
            rows: seeds,
            topic: topic.as_deref(),
            recent: &history,
        },
    );
    // AJ-E3832 G9: a question about what the user told Bot gets those
    // memories as records it can cite (the ones the world model linked to
    // it, else the newest), read through ADR-0037's store.
    let memories = if super::memory::asks_about_memory(asked) {
        let keys: Vec<String> = connected
            .memory
            .iter()
            .map(|(_, key, _)| key.clone())
            .chain(memory.iter().map(|m| m.key.clone()))
            .collect();
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        super::memory::rows_for(&storage, &keys)
    } else {
        Vec::new()
    };
    let answered = super::answer::answer(
        model,
        &super::answer::Asked {
            question: asked,
            reading: &reading,
            world: &world,
            scope: scope.as_ref(),
            again: marked_wrong(state, input.again.as_deref()),
            memory: &memory,
            history: &history,
            connected: &connected,
            memories: &memories,
        },
    );
    let outcome = answered.outcome.unwrap_or(BotOutcome::Failed);
    let rows = answered
        .found
        .as_ref()
        .map(|found| found.rows.clone())
        .unwrap_or_default();
    keep_in_meta(&mut meta, &answered);
    let written = answered.written.clone();
    let general = answered.general.clone();
    if let Some(details) = &answered.details {
        crate::features::ai::run::note_usage(state, details.tokens_in, details.tokens_out);
    }
    let entities = serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_owned());
    // A general answer's rows aren't its grounding: none are kept with it.
    let rows = if outcome == BotOutcome::Answered || outcome == BotOutcome::DontKnow {
        rows
    } else {
        Vec::new()
    };
    let cites = cites_json(&rows);
    let shown = shown_text(written.as_ref()).or_else(|| general.as_ref().map(|g| g.text.clone()));
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
        event.payload = serde_json::json!({
            "outcome": outcome.code(),
            "rows": rows.len(),
            "calls": answered.calls,
            "failure": answered.failure.map(crate::features::ai::run::AiFailure::code),
            "rerouted": answered.rerouted,
        });
        let _ = storage.ai_log().append(&event, None);
        // Each memory the question used is linked to the turn, so its
        // forget reaches this turn too (ADR-0037 §1.5 step 3).
        for (_, subject) in &used {
            storage
                .memory()
                .link_turn(&kept.id, subject)
                .map_err(memory_error)?;
        }
        // A memory shown as a record is linked too: its forget redacts the
        // turn that showed it.
        for row in rows
            .iter()
            .filter(|r| r.kind == super::dto::BotRowKind::Memory)
        {
            storage
                .memory()
                .link_turn(&kept.id, &row.id)
                .map_err(memory_error)?;
        }
        // The turn in the world model, part of its topic, about the records
        // its answer rested on (ADR-0038 §5); then the uses its context
        // queued, in this same hold.
        let about: Vec<(super::dto::BotRowKind, String)> =
            rows.iter().map(|r| (r.kind, r.id.clone())).collect();
        let _ = state.world.write(
            state,
            &storage,
            &crate::features::ai::world::link::turn_batch(
                &kept.id,
                &kept.topic,
                &about,
                world.now_ms,
            ),
            world.now_ms,
        );
        state.world.flush_uses(&storage, world.now_ms);
        kept
    };
    let mut info = turn_info(&kept, None, &|_| None);
    info.used = used.into_iter().map(|(info, _)| info).collect();
    // The turn just asked shows its rows as found, agent words included.
    info.rows = rows;
    info.written = written;
    info.general = general;
    info.live = true;
    Ok(info)
}

/// What a turn keeps of its answer beside its question (no words: the
/// shown answer is the `shown` column).
fn keep_in_meta(meta: &mut Meta, answered: &super::answer::Answer) {
    if let Some(found) = &answered.found {
        meta.shortcut = found.shortcut;
        meta.looked = Some(super::dto::BotLooked {
            memories: u32::try_from(meta.used.len()).unwrap_or(0),
            ..found.looked.clone()
        });
        meta.total = found.total;
        meta.tallies.clone_from(&found.tallies);
    }
    meta.unknown = answered.unknown;
    meta.failure = answered.failure;
    meta.details.clone_from(&answered.details);
    if let Some(written) = &answered.written {
        meta.written_model = Some(written.model_name.clone());
        meta.written_cites = written.parts.iter().map(|p| p.cites.clone()).collect();
    }
    if let Some(general) = &answered.general {
        meta.general_model = Some(general.model_name.clone());
        meta.off_topic = general.off_topic;
    }
}

/// The topic's running totals (AJ-E3825 R8): tokens this topic, and the
/// latest answer's context and memory.
///
/// # Errors
///
/// Storage failures.
pub fn bot_topic_usage_impl(state: &Arc<AppState>) -> AppResult<BotTopicUsageInfo> {
    let mut usage = BotTopicUsageInfo::default();
    for turn in topic_turns(state, 500)? {
        let meta: Meta = serde_json::from_str(&turn.entities).unwrap_or_default();
        if let Some(details) = meta.details {
            usage.answers += 1;
            usage.tokens_in = usage.tokens_in.plus(details.tokens_in);
            usage.tokens_out = usage.tokens_out.plus(details.tokens_out);
            if usage.latest.is_none() {
                usage.latest = Some(details);
            }
        }
    }
    Ok(usage)
}

/// A turn with no rows kept as it is (a watch draft, a memory echo).
fn keep_turn(
    state: &Arc<AppState>,
    world: &World,
    question: &str,
    outcome: BotOutcome,
    shown: Option<&str>,
    meta: &Meta,
) -> AppResult<juggler_core::storage::memory::Turn> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    storage
        .chat()
        .append(&NewTurn {
            at_ms: world.now_ms,
            question,
            outcome: outcome.code(),
            shown,
            cites: "[]",
            entities: &serde_json::to_string(meta).unwrap_or_else(|_| "{}".to_owned()),
            has_agent_words: false,
        })
        .map_err(memory_error)
}

/// AJ-E3830: the turn that opens the Watches form filled in (nothing is
/// added: the user's click in the form adds the watch).
fn watch_turn(
    state: &Arc<AppState>,
    world: &World,
    question: &str,
    drafted: crate::features::watches::draft::WatchDrafted,
) -> AppResult<BotTurnInfo> {
    let shown = super::watch_draft::shown(&drafted);
    let meta = Meta {
        lang: read(question).lang,
        // 0.5.2 G30: read back, it says it drafted (or set) a reminder.
        remind: matches!(
            drafted,
            crate::features::watches::draft::WatchDrafted::Remind { .. }
        )
        .then(BotPastReminder::default),
        ..Meta::default()
    };
    let kept = keep_turn(
        state,
        world,
        question,
        BotOutcome::WatchDraft,
        Some(&shown),
        &meta,
    )?;
    let mut info = turn_info(&kept, None, &|_| None);
    info.live = true;
    super::watch_draft::attach(&mut info, drafted);
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
    let kept = keep_turn(state, world, question, outcome, None, &meta)?;
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

/// The watches the user has, as the Watches page counts them.
pub(super) fn kept_watches(storage: &juggler_core::storage::Storage) -> u32 {
    let registry = crate::features::watches::conditions::Registry::shipped();
    crate::features::watches::store::WatchStore::new(storage, &registry).kept()
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
        watches: kept_watches(&storage),
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

/// AJ-E1548 #2: whether Ask again's `turn` has its answer marked wrong
/// now (the request says so only then).
fn marked_wrong(state: &AppState, turn: Option<&str>) -> bool {
    let (Some(turn), Ok(storage)) = (turn, state.storage.lock()) else {
        return false;
    };
    storage
        .chat()
        .turn(turn)
        .ok()
        .flatten()
        .and_then(|kept| serde_json::from_str::<Meta>(&kept.entities).ok())
        .is_some_and(|meta| meta.flagged)
}

/// "This is wrong" about a turn's written answer (`wrong`), or its Undo:
/// kept on this Mac only, in the turn and the AI log (ADR-0036 §1.8).
/// AJ-E1548 #2: an answer marked wrong is never sent back to the AI (no
/// earlier answer is); Ask again re-asks its question with a note.
///
/// # Errors
///
/// No such turn; storage failures.
pub fn bot_wrong_impl(state: &Arc<AppState>, turn: &str, wrong: bool) -> AppResult<()> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let chat = storage.chat();
    let Some(kept) = chat.turn(turn).map_err(memory_error)? else {
        return Err(AppError::Validation {
            subject: "bot",
            message: "no_such_turn".to_owned(),
        });
    };
    let mut meta: Meta = serde_json::from_str(&kept.entities).unwrap_or_default();
    meta.flagged = wrong;
    chat.annotate(
        turn,
        &serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_owned()),
    )
    .map_err(memory_error)?;
    let now = state.notifier.clock().now;
    let what = if wrong { "wrong" } else { "wrong_undone" };
    let mut event = NewEvent::new(now, Stream::Feedback, what, Actor::User);
    event.subject = Some(("chat_turn", turn.to_owned()));
    storage
        .ai_log()
        .append(&event, None)
        .map_err(memory_error)?;
    Ok(())
}

/// 0.5.2 G30: the user set the reminder Bot's turn `turn` drafted: the
/// turn keeps the reminder's id, so read back it says it set one and opens
/// it. A turn that drafted none (or is gone) is left as it is.
pub fn reminder_set(state: &Arc<AppState>, turn: &str, schedule: &str) {
    let Ok(storage) = state.storage.lock() else {
        return;
    };
    let chat = storage.chat();
    let Ok(Some(kept)) = chat.turn(turn) else {
        return;
    };
    let mut meta: Meta = serde_json::from_str(&kept.entities).unwrap_or_default();
    if meta.remind.is_none() {
        return;
    }
    meta.remind = Some(BotPastReminder {
        set: Some(schedule.to_owned()),
    });
    let _ = chat.annotate(
        turn,
        &serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_owned()),
    );
}

/// D-144: Bot is AI and conversation only. A question is taken only
/// while Bot's AI is ready; with none, the window shows "Set up Bot" and
/// no question box, and this refuses anything that reaches it anyway.
///
/// # Errors
///
/// Bot's AI isn't ready.
pub fn ai_ready_or_refuse(state: &AppState) -> AppResult<()> {
    if written_on(state) {
        Ok(())
    } else {
        Err(AppError::Validation {
            subject: "bot",
            message: "Bot's AI isn't set up yet".to_owned(),
        })
    }
}

/// Asks Bot (see [`bot_ask_impl`]), only while its AI is ready (D-144).
///
/// # Errors
///
/// As [`ai_ready_or_refuse`] and [`bot_ask_impl`].
#[tauri::command]
pub async fn bot_ask(
    state: tauri::State<'_, Arc<AppState>>,
    input: BotAskInput,
) -> AppResult<BotTurnInfo> {
    crate::ipc::blocking(state.inner(), "Bot", move |state| {
        ai_ready_or_refuse(state)?;
        bot_ask_impl(state, &input)
    })
    .await
}

/// The topic's running totals (see [`bot_topic_usage_impl`]).
///
/// # Errors
///
/// As [`bot_topic_usage_impl`].
#[tauri::command]
pub async fn bot_topic_usage(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<BotTopicUsageInfo> {
    crate::ipc::blocking(state.inner(), "Bot", bot_topic_usage_impl).await
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

/// "This is wrong", or its Undo with `wrong: false` (see
/// [`bot_wrong_impl`]).
///
/// # Errors
///
/// As [`bot_wrong_impl`].
#[tauri::command]
pub async fn bot_wrong(
    state: tauri::State<'_, Arc<AppState>>,
    turn: String,
    wrong: Option<bool>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "Bot", move |state| {
        bot_wrong_impl(state, &turn, wrong.unwrap_or(true))
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
