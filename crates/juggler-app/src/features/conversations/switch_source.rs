//! AJ-E2108: a session another app runs (the Claude app, Codex in the
//! `ChatGPT` app, a CLI in a terminal) as the source of a switch.
//!
//! The switch reads a source session: its account, launch settings,
//! folder, project and conversation file. A session AI Juggler runs has a
//! row with all of that; one another app runs has its conversation in the
//! index instead. Its key (`conversation:<account>:<id>`) stands where a
//! session id goes, so the switch's targets, checks, preflight and the
//! switch itself are the same code for both (no second switch).
//!
//! Nothing here writes to the other app's session or its files (D-28,
//! D-37): the conversation is only read, and the switch continues it as a
//! new session on the profile the user picked. The source gets no row of
//! its own: it isn't a session AI Juggler runs, and its own row stays the
//! other app's in every list. The switch graph records it by its account
//! and the tool's id for it.

use std::sync::Arc;

use juggler_core::model::{
    Account, ConversationId, InstanceKind, InstanceRecord, SessionId, SessionStatus, Timestamp,
    TranscriptRef, UsageSnapshot,
};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// What a source key starts with (session ids start `sess-`).
const PREFIX: &str = "conversation:";

/// The key a switch takes for `account_id`'s conversation `id`, where a
/// session id goes. The UI builds the same (`switchSourceKey`).
#[must_use]
pub fn source_key(account_id: &str, id: &str) -> String {
    format!("{PREFIX}{account_id}:{id}")
}

/// The account and conversation of a source key; `None` for a session id.
/// Account ids hold no colon, so the first one after the prefix ends it.
#[must_use]
pub fn parse_source_key(key: &str) -> Option<(&str, &str)> {
    let (account, id) = key.strip_prefix(PREFIX)?.split_once(':')?;
    (!account.is_empty() && !id.is_empty()).then_some((account, id))
}

/// Whether `id` is a session another app runs (a source key), not a row.
#[must_use]
pub fn is_source_key(id: &str) -> bool {
    parse_source_key(id).is_some()
}

/// The app `account_id`'s conversation `id` is open in now (`Claude`,
/// `ChatGPT`, `Terminal`), when it is open and its app is known.
#[must_use]
pub fn open_in(state: &Arc<AppState>, account_id: &str, tool: &str, id: &str) -> Option<String> {
    state
        .conversations
        .live_entry(account_id, tool, id)?
        .places
        .iter()
        .find_map(|place| place.host.as_ref().map(|host| host.shown.clone()))
        .filter(|name| !name.is_empty())
}

/// `account_id`'s conversation `id` as a switch's source: the row the
/// switch reads, built from the index and never stored. Its launch
/// settings are the account's first (as "Continue in AI Juggler" takes);
/// it is `Running` while a process has it open, so the switch screen says
/// it keeps going there. `for_the_switch` is the switch itself running:
/// only then is its folder required and its project recorded (a look at
/// the targets or the preflight writes nothing).
///
/// # Errors
///
/// [`AppError::Refused`] in words: the conversation is gone, the account
/// has no launch settings, or (for the switch) its folder is gone; storage
/// errors.
pub(crate) fn as_switch_source(
    state: &Arc<AppState>,
    account_id: &str,
    id: &str,
    for_the_switch: bool,
) -> AppResult<(InstanceRecord, Account)> {
    let conversation = super::find(state, account_id, id)?;
    let account = crate::commands::find_account_arc(state, account_id)?;
    let folder = conversation.folder.clone().filter(|folder| folder.is_dir());
    if for_the_switch && folder.is_none() {
        return Err(AppError::Refused(
            "its folder isn't there any more".to_owned(),
        ));
    }
    let live = state
        .conversations
        .is_live(account_id, &conversation.tool, id);
    // The machine identity locks storage itself: resolved first.
    let machine = crate::commands::local_machine_id(state);
    let (profile, project) = {
        let storage = lock_storage_arc(state)?;
        let profile = storage
            .profiles()
            .list_by_account(&account.id)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                AppError::Refused(format!(
                    "{} has no launch settings yet: add them first",
                    account.label
                ))
            })?;
        let project = match &folder {
            Some(folder) => {
                let homes = storage.projects().worktree_homes()?;
                let root = juggler_core::storage::project_root_with(folder, &homes);
                if for_the_switch {
                    Some(storage.projects().touch(&root, None)?.id)
                } else {
                    storage
                        .projects()
                        .list()?
                        .into_iter()
                        .find(|project| project.root == root)
                        .map(|project| project.id)
                }
            }
            None => None,
        };
        (profile, project)
    };
    let summary = &conversation.summary;
    let record = InstanceRecord {
        id: SessionId::new(source_key(account_id, id)),
        conversation: Some(ConversationId::new(id)),
        profile: profile.id,
        machine,
        kind: InstanceKind::Terminal,
        project,
        cwd: folder.unwrap_or_default(),
        status: if live {
            SessionStatus::Running
        } else {
            SessionStatus::Finished
        },
        started_at: Timestamp::from_millis(summary.started_ms.unwrap_or_default()),
        last_activity: summary.last_ms.map(Timestamp::from_millis),
        usage: UsageSnapshot {
            tokens: summary.tokens,
            ..UsageSnapshot::default()
        },
        attention: None,
        running: None,
        window: None,
        transcript_ref: Some(TranscriptRef {
            path: conversation.path.clone(),
            native_id: Some(id.to_owned()),
        }),
    };
    Ok((record, account))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_key_reads_back_and_a_session_id_is_not_one() {
        let key = source_key("claude-code-1a2b", "0b5e0e5c-6c0b-4c55");
        assert_eq!(key, "conversation:claude-code-1a2b:0b5e0e5c-6c0b-4c55");
        assert_eq!(
            parse_source_key(&key),
            Some(("claude-code-1a2b", "0b5e0e5c-6c0b-4c55"))
        );
        // A conversation id with a colon in it stays whole.
        assert_eq!(
            parse_source_key("conversation:codex-9:thread:1"),
            Some(("codex-9", "thread:1"))
        );
        for not_one in [
            "sess-123",
            "conversation:",
            "conversation:a:",
            "conversation::x",
        ] {
            assert!(parse_source_key(not_one).is_none(), "not a source key");
        }
    }

    const NATIVE: &str = "6f1c2d3e-4a5b-4c6d-8e7f-001122334455";

    /// Two Claude accounts (`true` stands in for the tool: nothing real
    /// runs) and a conversation on the first, open in the Claude app now.
    fn open_in_the_claude_app(dir: &std::path::Path) -> (Arc<AppState>, String, String) {
        let root = dir.canonicalize().unwrap();
        let mut registry = juggler_adapter::AdapterRegistry::new();
        let claude = Arc::new(juggler_adapters::ClaudeCodeAdapter::with_bin(
            "/usr/bin/true",
        ));
        registry.register(claude.clone());
        registry.register_handover(claude);
        let state =
            Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
        let tool = juggler_adapters::CLAUDE_CODE_ID;
        let work = crate::commands::create_account(&state, tool, "Work", None).unwrap();
        crate::commands::create_profile(&state, &work.id, "Main", None, None).unwrap();
        let home = crate::commands::create_account(&state, tool, "Home", None).unwrap();
        let home_recipe =
            crate::commands::create_profile(&state, &home.id, "Main", None, None).unwrap();
        // Home is set up, as a signed-in account is (a switch checks, 0.3.2).
        let home_dir = state.accounts_dir().join(tool).join(&home.id);
        std::fs::create_dir_all(&home_dir).unwrap();
        std::fs::write(
            home_dir.join(".claude.json"),
            r#"{"hasCompletedOnboarding":true}"#,
        )
        .unwrap();
        let folder = root.join("site");
        std::fs::create_dir_all(&folder).unwrap();
        let transcripts = state
            .accounts_dir()
            .join(tool)
            .join(&work.id)
            .join("projects")
            .join(folder.to_string_lossy().replace(['/', '.'], "-"));
        std::fs::create_dir_all(&transcripts).unwrap();
        let cwd = folder.display();
        std::fs::write(
            transcripts.join(format!("{NATIVE}.jsonl")),
            format!(
                "{{\"type\":\"user\",\"uuid\":\"u1\",\"sessionId\":\"{NATIVE}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"user\",\"content\":\"Fix the footer\"}}}}\n\
                 {{\"type\":\"assistant\",\"uuid\":\"u2\",\"parentUuid\":\"u1\",\"sessionId\":\"{NATIVE}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"Fixed.\"}}]}}}}\n"
            ),
        )
        .unwrap();
        super::super::commands::list_impl(&state).unwrap();
        let app = super::super::places::Host {
            app_id: Some("claude".to_owned()),
            name: "Claude".to_owned(),
            shown: "Claude app".to_owned(),
            kind: Some(juggler_adapter::live::HostKind::Desktop),
            pid: 4242,
            start: None,
            instance_dir: None,
            instance: None,
        };
        state.conversations.live.lock().unwrap().insert(
            (work.id.clone(), tool.to_owned(), NATIVE.to_owned()),
            super::super::scan::LiveEntry {
                tool: tool.to_owned(),
                account_id: Some(work.id.clone()),
                session_id: NATIVE.to_owned(),
                places: vec![super::super::scan::Place {
                    pid: 4243,
                    start: None,
                    host: Some(app),
                    profile: None,
                }],
                state: Some(super::super::scan::State::Idle),
                probably: false,
                viewed_ms: None,
                app_title: None,
                background: false,
            },
        );
        (state, work.id, home_recipe.id)
    }

    /// AJ-E2108: a session open in its app is switched to another account
    /// while it stays open there; only what would write to it too (picking
    /// it up on its own account, going back to it) waits until it's closed.
    #[test]
    fn a_session_open_in_its_app_switches_and_stays_open_there() {
        let dir = tempfile::tempdir().unwrap();
        let (state, work, home_recipe) = open_in_the_claude_app(dir.path());
        let key = source_key(&work, NATIVE);

        let (source, _) = as_switch_source(&state, &work, NATIVE, false).unwrap();
        assert_eq!(source.status, SessionStatus::Running, "open in its app");
        let preflight = crate::commands::switch_preflight_impl(&state, &key, &home_recipe).unwrap();
        assert!(preflight.source_outside && preflight.source_live);
        assert!(
            preflight.source_keeps_running,
            "AI Juggler never stops another app's session"
        );
        assert_eq!(preflight.source_app.as_deref(), Some("Claude app"));

        let switched = crate::commands::switch_execute_impl(
            &state,
            &key,
            &home_recipe,
            "finish-turn",
            None,
            &[],
        )
        .expect("a switch to another account isn't blocked by the source being open");

        // Its own account would be a second writer: refused, in words.
        let carry_in = super::super::continue_impl(&state, &work, NATIVE).unwrap_err();
        assert!(carry_in.to_string().contains("close it there first"));
        let back =
            crate::commands::switch_back_execute_impl(&state, &switched.session_id, &[], false)
                .unwrap_err();
        assert!(
            back.to_string().contains("still open in its app"),
            "going back waits until it's closed there"
        );
    }
}
