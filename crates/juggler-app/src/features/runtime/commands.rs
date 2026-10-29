//! The runtime's commands (AJ-E2303..E2307). Only the app's own window
//! calls them; an agent can't (the bridge has no path here).
//!
//! [`runtime_answer`] is the one way an ask is answered: the user's click
//! in Needs you or on the session page, carried to the tool unchanged and
//! written to the account's audit (the option, never a reply's text).

use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::runtime::{
    AskAnswer, AskId, PartAnswer, Release, Runtime, TerminalView, TurnInput, ViewOf,
};
use juggler_core::model::SessionId;

use super::dto::{AnswerInput, AskInfo, ProfileRuntimeInfo, RuntimeChoicesInfo, RuntimeViewInfo};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The most asks one page holds.
pub const ASKS_PAGE_MAX: usize = 100;
/// The longest turn text AI Juggler sends.
pub const TURN_MAX: usize = 64 * 1024;

fn driver(state: &Arc<AppState>, session: &str) -> AppResult<Arc<super::driver::Driver>> {
    state
        .runtime
        .driver(session)
        .ok_or_else(|| AppError::Refused("this session isn't run from AI Juggler".into()))
}

/// A driven session's page, or `None` for a session in a terminal.
///
/// # Errors
///
/// None today (kept for the IPC shape).
pub fn get_impl(state: &Arc<AppState>, session: &str) -> AppResult<Option<RuntimeViewInfo>> {
    Ok(state.runtime.driver(session).map(|driver| driver.view()))
}

/// The open asks of every driven session, oldest first, one page.
#[must_use]
pub fn asks_impl(state: &Arc<AppState>, offset: usize, limit: usize) -> Vec<AskInfo> {
    let mut asks: Vec<AskInfo> = state
        .runtime
        .drivers()
        .iter()
        .flat_map(|driver| driver.info().asks)
        .collect();
    asks.sort_by_key(|ask| ask.asked_at_ms);
    asks.into_iter()
        .skip(offset)
        .take(limit.clamp(1, ASKS_PAGE_MAX))
        .collect()
}

/// The user's answer as the runtime's, checked for shape (the codec
/// checks it against the open ask).
fn answer_of(input: AnswerInput) -> AppResult<AskAnswer> {
    match (input.option, input.replies) {
        (Some(option), None) => Ok(AskAnswer::Choose { option }),
        (None, Some(replies)) if !replies.is_empty() => Ok(AskAnswer::Reply {
            parts: replies
                .into_iter()
                .map(|reply| PartAnswer {
                    part: reply.part,
                    answers: reply
                        .answers
                        .into_iter()
                        .map(|answer| answer.chars().take(TURN_MAX).collect())
                        .collect(),
                })
                .collect(),
        }),
        _ => Err(AppError::Validation {
            subject: "answer",
            message: "an answer is one option, or replies to the question".into(),
        }),
    }
}

/// Carries the user's own answer to an open ask, and audits it.
///
/// # Errors
///
/// [`AppError::Refused`] for a session that isn't driven, an ask that
/// isn't open, or an option the tool didn't offer.
pub fn answer_impl(
    state: &Arc<AppState>,
    session: &str,
    ask: &str,
    input: AnswerInput,
) -> AppResult<()> {
    let driver = driver(state, session)?;
    let open = driver
        .info()
        .asks
        .into_iter()
        .find(|open| open.id == ask)
        .ok_or_else(|| AppError::Refused("that question isn't open any more".into()))?;
    let answer = answer_of(input)?;
    let chosen = match &answer {
        AskAnswer::Choose { option } => open
            .options
            .iter()
            .find(|offered| &offered.id == option)
            .map_or_else(|| option.clone(), |offered| offered.label.clone()),
        AskAnswer::Reply { .. } => "replied".to_owned(),
    };
    driver
        .answer(AskId(ask.to_owned()), answer)
        .map_err(AppError::Refused)?;
    audit(state, session, &open, &chosen);
    Ok(())
}

/// The account's audit row for an answer: the session, what it asked (its
/// kind), the option; never a reply's text.
fn audit(state: &Arc<AppState>, session: &str, ask: &AskInfo, chosen: &str) {
    let Ok(storage) = state.storage.lock() else {
        return;
    };
    let Ok(record) = storage.sessions().get(&SessionId::new(session)) else {
        return;
    };
    let Ok(profile) = storage.profiles().get(&record.profile) else {
        return;
    };
    let kind = ask.approval_kind.as_deref().unwrap_or(ask.kind.as_str());
    let detail = format!("{session} · {kind} · {chosen}");
    let _ = storage.account_audit().record(
        &profile.account,
        "answered",
        Some(&detail),
        crate::manage::ACTOR_USER,
    );
}

/// Sends the user's text: a new turn between turns, or into the running
/// turn where the tool takes that.
///
/// # Errors
///
/// [`AppError::Refused`] in the tool's words when it can't take it now.
pub fn send_impl(state: &Arc<AppState>, session: &str, text: &str) -> AppResult<()> {
    let text = text.trim();
    if text.is_empty() || text.len() > TURN_MAX {
        return Err(AppError::Validation {
            subject: "message",
            message: "a message is 1 to 64 KB of text".into(),
        });
    }
    let driver = driver(state, session)?;
    let input = TurnInput::from_user(text);
    if driver.info().can_steer {
        driver.steer(input)
    } else {
        driver.turn(input)
    }
    .map_err(AppError::Refused)
}

/// Interrupts the running turn.
///
/// # Errors
///
/// [`AppError::Refused`] for a session that isn't driven.
pub fn interrupt_impl(state: &Arc<AppState>, session: &str) -> AppResult<()> {
    driver(state, session)?
        .interrupt()
        .map_err(AppError::Refused)
}

/// Where a hand-over goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    /// The tool's own terminal UI ("Open in terminal").
    Terminal,
    /// Back to AI Juggler ("Run in AI Juggler").
    Driven,
}

/// What a hand-over started: the session that now holds the conversation,
/// and the terminal to show when one was opened beside it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct HandoverInfo {
    /// The session to show now.
    pub session_id: String,
}

/// The session, its launch settings and its account.
pub(super) fn owner(
    state: &Arc<AppState>,
    session: &str,
) -> AppResult<(
    juggler_core::model::SessionRecord,
    juggler_core::model::Profile,
    juggler_core::model::Account,
)> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let record = storage.sessions().get(&SessionId::new(session))?;
    let profile = storage.profiles().get(&record.profile)?;
    let account = storage.accounts().get(&profile.account)?;
    Ok((record, profile, account))
}

/// Continues `native_id` as a new session of `from`'s, in `runtime`: the
/// old one is marked handed over, the new one recorded (never two writers:
/// the old one's process has ended first).
fn continue_as(
    state: &Arc<AppState>,
    from: &juggler_core::model::SessionRecord,
    profile: &juggler_core::model::Profile,
    account: &juggler_core::model::Account,
    native_id: &str,
    runtime: Runtime,
) -> AppResult<HandoverInfo> {
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let terminal = adapter.launch_spec(
        account,
        profile,
        &from.cwd,
        &juggler_adapter::LaunchMode::Resume {
            session_id: native_id.to_owned(),
            delta_prompt: None,
        },
    )?;
    // The one path: its row first, then the runtime asked for.
    let session = SessionId::new(format!("sess-{}", crate::commands::unique_suffix()));
    let started = crate::features::sessions::spawn::start_session(
        state,
        &crate::features::sessions::spawn::NewSession {
            id: &session,
            account,
            profile: &profile.id,
            project: from.project.clone(),
            cwd: from.cwd.clone(),
            conversation: Some(juggler_core::model::ConversationId::new(native_id)),
            transcript_ref: from.transcript_ref.clone(),
            origin: "resume",
        },
        crate::features::sessions::spawn::Runs::Planned {
            profile,
            resume: Some(native_id.to_owned()),
            requested: Some(runtime),
            terminal,
        },
    )?;
    record_continuation(state, from)?;
    Ok(HandoverInfo {
        session_id: started.session.to_string(),
    })
}

/// A new session holds `from`'s conversation now (its row was written
/// by the one path before it started): `from` is marked handed over.
pub(super) fn record_continuation(
    state: &Arc<AppState>,
    from: &juggler_core::model::SessionRecord,
) -> AppResult<()> {
    use juggler_core::model::SessionStatus;
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    if let Ok(mut old) = storage.sessions().get(&from.id) {
        old.status = SessionStatus::HandedOver;
        old.attention = None;
        storage.sessions().update(&old)?;
    }
    Ok(())
}

/// Hands a session's conversation over at a turn boundary (D-40, one
/// writer at a time): a driven session to the tool's own terminal ("Open
/// in terminal": the driver lets go when the turn ends, its process ends,
/// and the terminal resumes the conversation as a new session), or a
/// session whose terminal has ended back to AI Juggler ("Run in AI
/// Juggler"). A tool that attaches its terminal live gets it beside the
/// driver instead.
///
/// # Errors
///
/// [`AppError::Refused`] in words: a terminal still open, a conversation
/// not started yet, a tool with one runtime.
pub fn handover_impl(
    app: Option<&tauri::AppHandle>,
    state: &Arc<AppState>,
    session: &str,
    toward: Toward,
) -> AppResult<HandoverInfo> {
    let (record, profile, account) = owner(state, session)?;
    let id = SessionId::new(session);
    if toward == Toward::Driven {
        if state.runtime.is_driven(&id) {
            return Err(AppError::Refused("it already runs in AI Juggler".into()));
        }
        if !state.supervisor.is_finished(&id) {
            return Err(AppError::Refused(
                "it's open in its terminal: exit it there first (one writer at a time)".into(),
            ));
        }
        let native_id = record
            .conversation
            .as_ref()
            .map(ToString::to_string)
            .or_else(|| {
                record
                    .transcript_ref
                    .as_ref()
                    .and_then(|t| t.native_id.clone())
            })
            .ok_or_else(|| AppError::Refused("its conversation isn't known yet".into()))?;
        return continue_as(
            state,
            &record,
            &profile,
            &account,
            &native_id,
            Runtime::Driven,
        );
    }
    let driver = driver(state, session)?;
    let native_id = driver
        .snapshot()
        .native_id
        .ok_or_else(|| AppError::Refused("its conversation hasn't started yet".into()))?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let tool_runtime = adapter
        .driven()
        .ok_or_else(|| AppError::Refused("this tool runs in a terminal only".into()))?;
    let view = tool_runtime.terminal_view(
        &account,
        &profile,
        &record.cwd,
        &ViewOf {
            native_id: native_id.clone(),
            socket: driver.socket.clone(),
        },
    )?;
    match view {
        TerminalView::Attach(spec) => {
            let app = app.ok_or_else(|| AppError::Refused("no window to show it in".into()))?;
            crate::terminal::spawn_view_terminal(app, state, &account, &spec)?;
            Ok(HandoverInfo {
                session_id: session.to_owned(),
            })
        }
        TerminalView::AfterRelease(_) => {
            driver
                .release(Release::AfterTurn(Duration::from_secs(600)))
                .map_err(AppError::Refused)?;
            // Its input closed, the tool's process ends on its own; stopped
            // if it lingers, so the terminal is the only writer.
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while !state.supervisor.is_finished(&id) && std::time::Instant::now() < deadline {
                // poll: the tool exits within 5 s of its input closing.
                std::thread::sleep(Duration::from_millis(20));
            }
            if !state.supervisor.is_finished(&id) {
                let _ = state.supervisor.stop(&id, Duration::from_secs(2));
            }
            state.runtime.forget(session);
            continue_as(
                state,
                &record,
                &profile,
                &account,
                &native_id,
                Runtime::Terminal,
            )
        }
    }
}

/// The profiles' runtime choices and the Beta switch.
///
/// # Errors
///
/// Storage failures.
pub fn choices_impl(state: &Arc<AppState>) -> AppResult<RuntimeChoicesInfo> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let chosen = super::settings::profile_choices(&storage);
    let mut profiles = Vec::new();
    for account in storage.accounts().list()? {
        let Some(driven) = state.registry.get(&account.harness).and_then(|a| {
            a.driven().map(|d| {
                (
                    d.label(),
                    d.default_runtime(),
                    d.maturity() == juggler_adapter::runtime::Maturity::Beta,
                )
            })
        }) else {
            continue;
        };
        for profile in storage.profiles().list_by_account(&account.id)? {
            profiles.push(ProfileRuntimeInfo {
                profile_id: profile.id.to_string(),
                account_label: account.label.clone(),
                name: profile.name.clone(),
                label: driven.0.to_owned(),
                default: driven.1.as_str().to_owned(),
                chosen: chosen
                    .get(profile.id.as_str())
                    .map(|r| r.as_str().to_owned()),
                beta: driven.2,
                unavailable: None,
            });
        }
    }
    Ok(RuntimeChoicesInfo {
        beta_on: super::settings::beta_on(&storage),
        profiles,
    })
}

/// Sets a profile's runtime (`terminal`, `driven`, or `None` for the
/// tool's default).
///
/// # Errors
///
/// [`AppError::Validation`] for another word.
pub fn choose_impl(
    state: &Arc<AppState>,
    profile: &str,
    runtime: Option<&str>,
) -> AppResult<RuntimeChoicesInfo> {
    let runtime = match runtime {
        None => None,
        Some(word) => Some(
            Runtime::from_word(word).ok_or_else(|| AppError::Validation {
                subject: "runtime",
                message: format!("{word} isn't a runtime"),
            })?,
        ),
    };
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .profiles()
            .get(&juggler_core::model::ProfileId::new(profile))?;
        super::settings::set_profile_choice(&storage, profile, runtime)?;
    }
    choices_impl(state)
}

/// Turns "Run without a terminal (Beta)" on or off.
///
/// # Errors
///
/// Storage failures.
pub fn beta_impl(state: &Arc<AppState>, on: bool) -> AppResult<RuntimeChoicesInfo> {
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        super::settings::set_beta(&storage, on)?;
    }
    choices_impl(state)
}

/// A driven session's page.
///
/// # Errors
///
/// As [`get_impl`].
#[tauri::command]
pub async fn runtime_get(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Option<RuntimeViewInfo>> {
    crate::ipc::blocking(state.inner(), "the session's runtime", move |state| {
        get_impl(state, &session_id)
    })
    .await
}

/// The open asks across driven sessions, one page.
///
/// # Errors
///
/// None (the IPC task's own).
#[tauri::command]
pub async fn runtime_asks(
    state: tauri::State<'_, Arc<AppState>>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> AppResult<Vec<AskInfo>> {
    crate::ipc::blocking(state.inner(), "the open questions", move |state| {
        let offset = usize::try_from(offset.unwrap_or(0)).unwrap_or(0);
        let limit = usize::try_from(limit.unwrap_or(50)).unwrap_or(50);
        Ok(asks_impl(state, offset, limit))
    })
    .await
}

/// The user's answer to an ask (their click).
///
/// # Errors
///
/// As [`answer_impl`].
#[tauri::command]
pub async fn runtime_answer(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    ask_id: String,
    answer: AnswerInput,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the answer", move |state| {
        answer_impl(state, &session_id, &ask_id, answer)
    })
    .await
}

/// The user's message to a driven session.
///
/// # Errors
///
/// As [`send_impl`].
#[tauri::command]
pub async fn runtime_send(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    text: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the message", move |state| {
        send_impl(state, &session_id, &text)
    })
    .await
}

/// Interrupts a driven session's turn.
///
/// # Errors
///
/// As [`interrupt_impl`].
#[tauri::command]
pub async fn runtime_interrupt(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the interrupt", move |state| {
        interrupt_impl(state, &session_id)
    })
    .await
}

/// Hands a session over: `toward` is `terminal` or `driven`.
///
/// # Errors
///
/// As [`handover_impl`].
#[tauri::command]
pub async fn runtime_handover(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    toward: String,
) -> AppResult<HandoverInfo> {
    let toward = match toward.as_str() {
        "terminal" => Toward::Terminal,
        "driven" => Toward::Driven,
        other => {
            return Err(AppError::Validation {
                subject: "hand-over",
                message: format!("{other} isn't where a session can go"),
            });
        }
    };
    crate::ipc::blocking(state.inner(), "the hand-over", move |state| {
        handover_impl(Some(&app), state, &session_id, toward)
    })
    .await
}

/// The profiles' runtime choices.
///
/// # Errors
///
/// As [`choices_impl`].
#[tauri::command]
pub async fn runtime_choices(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<RuntimeChoicesInfo> {
    crate::ipc::blocking(state.inner(), "the runtime choices", choices_impl).await
}

/// Sets a profile's runtime.
///
/// # Errors
///
/// As [`choose_impl`].
#[tauri::command]
pub async fn runtime_choose(
    state: tauri::State<'_, Arc<AppState>>,
    profile_id: String,
    runtime: Option<String>,
) -> AppResult<RuntimeChoicesInfo> {
    crate::ipc::blocking(state.inner(), "the runtime choice", move |state| {
        choose_impl(state, &profile_id, runtime.as_deref())
    })
    .await
}

/// The Beta switch.
///
/// # Errors
///
/// As [`beta_impl`].
#[tauri::command]
pub async fn runtime_beta(
    state: tauri::State<'_, Arc<AppState>>,
    on: bool,
) -> AppResult<RuntimeChoicesInfo> {
    crate::ipc::blocking(state.inner(), "the Beta switch", move |state| {
        beta_impl(state, on)
    })
    .await
}
