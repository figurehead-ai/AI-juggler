//! The conversations feature's commands.

use std::collections::HashMap;
use std::sync::Arc;

use super::dto::{ConversationInfo, GoToOutcomeInfo, PlaceInfo, StartedSessionInfo};
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// Every account's conversations, newest first: each with its account,
/// project (worktrees folded into their repository), title, tokens,
/// whether a running session is on it, and the AI Juggler session it is
/// when AI Juggler started it.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn list_impl(state: &Arc<AppState>) -> AppResult<Vec<ConversationInfo>> {
    // The first ask after a start reads the files itself, unless the
    // background pass already is.
    super::read_if_cold(state)?;
    let rows = list_indexed(state)?;
    Ok(crate::features::subagents::count::counted_conversations(
        state, rows,
    ))
}

/// [`list_impl`]'s rows as the index has them now, without reading any
/// file or counting subagents (AJ-E3102: the relation counts are made
/// from these, so they and the lists agree).
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn list_indexed(state: &Arc<AppState>) -> AppResult<Vec<ConversationInfo>> {
    let (labels, folders, projects, sessions, homes) = {
        let storage = lock_storage_arc(state)?;
        let homes = storage.projects().worktree_homes()?;
        let accounts = storage.accounts().list()?;
        let folders: HashMap<std::path::PathBuf, String> = accounts
            .iter()
            .map(|account| {
                (
                    std::fs::canonicalize(&account.state_dir)
                        .unwrap_or_else(|_| account.state_dir.clone()),
                    account.label.clone(),
                )
            })
            .collect();
        let labels: HashMap<String, String> = accounts
            .into_iter()
            .map(|account| (account.id.to_string(), account.label))
            .collect();
        let projects =
            crate::features::projects::by_root(&storage, |project| project.id.to_string())?;
        // The newest session on each account's conversation (a later one
        // replaces an earlier in the map).
        let sessions: HashMap<(String, String), String> = storage
            .sessions()
            .conversation_links()?
            .into_iter()
            .map(|(account, conversation, session)| ((account, conversation), session))
            .collect();
        (labels, folders, projects, sessions, homes)
    };
    // One project lookup per folder: many conversations share one.
    let mut roots: HashMap<std::path::PathBuf, std::path::PathBuf> = HashMap::new();
    let marks = Marks::read(state);
    let rows = state
        .conversations
        .all()?
        .into_iter()
        .map(|conversation| {
            let summary = &conversation.summary;
            let root = conversation.folder.as_deref().map(|folder| {
                roots
                    .entry(folder.to_path_buf())
                    .or_insert_with(|| juggler_core::storage::project_root_with(folder, &homes))
                    .clone()
            });
            let mut info = ConversationInfo {
                live: false,
                places: Vec::new(),
                pid: None,
                state: None,
                probably: false,
                viewed_ms: None,
                background: false,
                inactive: state.conversations.is_inactive(
                    &conversation.account_id,
                    &conversation.tool,
                    &summary.id,
                ),
                session_id: sessions
                    .get(&(conversation.account_id.clone(), summary.id.clone()))
                    .cloned(),
                project_id: root.and_then(|root| projects.get(&root).cloned()),
                account_label: labels
                    .get(&conversation.account_id)
                    .cloned()
                    .unwrap_or_default(),
                id: summary.id.clone(),
                tool: conversation.tool.clone(),
                account_id: conversation.account_id.clone(),
                cwd: summary
                    .cwd
                    .as_ref()
                    .map(|cwd| cwd.to_string_lossy().into_owned()),
                title: row_title(state, &conversation.account_id, &conversation.tool, summary),
                git_branch: summary.git_branch.clone(),
                started_in: summary.started_in.clone(),
                started_from: marks.parent(&conversation.tool, &summary.id),
                started_ms: summary.started_ms,
                last_activity_ms: summary.last_ms,
                tokens: crate::features::sessions::read_model::tokens_info(summary.tokens),
                // AJ-E1520: `features::subagents::count` fills it; null until read.
                subtask_count: None,
                app_archived: marks.app_archived(&conversation.tool, &summary.id),
                archived_here: marks.here(&conversation.account_id, &summary.id),
            };
            if let Some(entry) = state.conversations.live_entry(
                &conversation.account_id,
                &conversation.tool,
                &summary.id,
            ) {
                with_live(&mut info, &entry, &folders);
            }
            info
        })
        .collect();
    Ok(rows)
}

/// A row's title: a desktop profile's own copy goes by the app's title
/// for it (what its sidebar shows, as the live view gives an open one;
/// the go sitting's rerun named rows by their first prompt), else the
/// transcript's.
fn row_title(
    state: &Arc<AppState>,
    folder: &str,
    tool: &str,
    summary: &juggler_adapter::conversation::ConversationSummary,
) -> Option<String> {
    let key = crate::features::places::book::conversation_key(tool, &summary.id);
    crate::features::places::record_title(state, folder, &key).or_else(|| summary.title.clone())
}

/// What a row carries besides its conversation, read once for the list:
/// who started it, whether its app archived it, whether it's archived
/// here (0.4.1).
struct Marks {
    parents: HashMap<(String, String), String>,
    app_archived: std::collections::HashSet<(String, String)>,
    here: std::collections::HashSet<String>,
}

impl Marks {
    fn read(state: &Arc<AppState>) -> Self {
        Self {
            parents: started_from(state),
            app_archived: state
                .conversations
                .live_model()
                .archived
                .into_iter()
                .collect(),
            here: super::archive::archived(state),
        }
    }

    fn parent(&self, tool: &str, id: &str) -> Option<String> {
        self.parents.get(&(tool.to_owned(), id.to_owned())).cloned()
    }

    fn app_archived(&self, tool: &str, id: &str) -> bool {
        self.app_archived
            .contains(&(tool.to_owned(), id.to_owned()))
    }

    fn here(&self, account: &str, id: &str) -> bool {
        self.here.contains(&super::archive::key_of(account, id))
    }
}

/// 0.3.9: who started whom, as the running apps record it (a Claude app
/// side session's parent), `(tool, conversation)` → the one it was started
/// from: read once for every row.
pub(crate) fn started_from(state: &Arc<AppState>) -> HashMap<(String, String), String> {
    state
        .conversations
        .live_model()
        .started
        .into_iter()
        .map(|link| ((link.tool, link.session_id), link.parent))
        .collect()
}

/// AJ-E2101: what the live view says of an open conversation: where it's
/// open, its state, when it was last viewed, and the app's own title (what
/// its sidebar shows).
fn with_live(
    info: &mut ConversationInfo,
    entry: &super::scan::LiveEntry,
    folders: &HashMap<std::path::PathBuf, String>,
) {
    info.live = true;
    info.inactive = false;
    info.places = entry
        .places
        .iter()
        .map(|place| place_info(place, folders))
        .collect();
    info.pid = info.places.first().map(|place| place.pid);
    info.state = entry.state.map(|state| state.word().to_owned());
    info.probably = entry.probably && info.state.is_some();
    info.viewed_ms = entry.viewed_ms;
    info.background = entry.background;
    if let Some(title) = &entry.app_title {
        info.title = Some(title.clone());
    }
}

/// One place in words: the app, then its instance or the profile.
fn place_info(
    place: &super::scan::Place,
    folders: &HashMap<std::path::PathBuf, String>,
) -> PlaceInfo {
    let profile = place.profile.as_ref().map(|dir| {
        folders.get(dir).cloned().unwrap_or_else(|| {
            dir.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    });
    let app = place
        .host
        .as_ref()
        .map(|host| host.name.clone())
        .unwrap_or_default();
    let shown = place
        .host
        .as_ref()
        .map(|host| host.shown.clone())
        .unwrap_or_default();
    let instance = place.host.as_ref().and_then(|host| host.instance.clone());
    let label = [Some(shown.clone()), instance.clone()]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    PlaceInfo {
        app_id: place.host.as_ref().and_then(|host| host.app_id.clone()),
        app,
        shown,
        profile,
        instance,
        pid: place.pid,
        kind: place.host.as_ref().and_then(|host| host.kind).map(|kind| {
            match kind {
                juggler_adapter::live::HostKind::Desktop => "desktop",
                juggler_adapter::live::HostKind::Terminal => "terminal",
                juggler_adapter::live::HostKind::Editor => "editor",
            }
            .to_owned()
        }),
        app_pid: place.host.as_ref().map(|host| host.pid),
        label,
    }
}

/// Whether the conversation list is still being built for the first time
/// ([`super::building`]): the window shows a quiet line until it isn't.
///
/// # Errors
///
/// None: an `AppResult` for the IPC's shape.
#[tauri::command]
pub async fn conversations_building(state: tauri::State<'_, Arc<AppState>>) -> AppResult<bool> {
    Ok(super::building(state.inner()))
}

/// The conversations the window holds (AJ-E1529: the rail, a project's
/// page, Accounts' running counts, the go-to palette): every one open now
/// or in an app's sidebar, and the newest [`RECENT`] of the last week
/// (0.1.5 review: it sent every conversation ever had, every few
/// seconds). History and the Sessions tabs page the rest
/// (`sessions_page`).
///
/// # Errors
///
/// As [`list_impl`].
#[tauri::command]
pub async fn conversations_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ConversationInfo>> {
    crate::ipc::blocking(state.inner(), "the conversations", |state| {
        Ok(working(list_impl(state)?, crate::commands::now_millis()))
    })
    .await
}

/// One conversation, by its account and id, as [`conversations_list`]
/// has it: a page opened for one past the window's working set (an old
/// one, from History or a project's Sessions). `None` when it's gone.
///
/// # Errors
///
/// As [`list_impl`].
#[tauri::command]
pub async fn conversations_get(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
) -> AppResult<Option<ConversationInfo>> {
    crate::ipc::blocking(state.inner(), "the conversation", move |state| {
        Ok(list_impl(state)?
            .into_iter()
            .find(|c| c.account_id == account_id && c.id == conversation_id))
    })
    .await
}

/// 0.3.4: the sessions `conversation_id` (of `tool`) started, newest
/// first, as a running app records them; none when no app does (a tool
/// that records no such link, or its app isn't open). Read from what the
/// live refresher last saw: no file is read here.
#[must_use]
pub fn started_impl(
    state: &Arc<AppState>,
    tool: &str,
    conversation_id: &str,
) -> Vec<StartedSessionInfo> {
    let mut started: Vec<StartedSessionInfo> = state
        .conversations
        .live_model()
        .started
        .into_iter()
        .filter(|link| link.tool == tool && link.parent == conversation_id)
        .map(|link| StartedSessionInfo {
            tool: link.tool,
            conversation_id: link.session_id,
            title: link.title,
            last_activity_ms: link.last_activity_ms,
        })
        .collect();
    newest_started(&mut started);
    started
}

/// AJ-E3106 R5: the most sessions one session's list holds (the page
/// shows them ten at a time).
pub const STARTED_MAX: usize = 100;

/// The newest first (no activity yet last), at most [`STARTED_MAX`].
fn newest_started(started: &mut Vec<StartedSessionInfo>) {
    started.sort_by_key(|row| std::cmp::Reverse(row.last_activity_ms));
    started.truncate(STARTED_MAX);
}

#[cfg(test)]
mod started_tests {
    use super::{STARTED_MAX, StartedSessionInfo, newest_started};

    #[test]
    fn the_sessions_it_started_come_newest_first_and_bounded() {
        let mut started: Vec<StartedSessionInfo> = (0..(STARTED_MAX + 20))
            .map(|n| StartedSessionInfo {
                tool: "claude-desktop".into(),
                conversation_id: format!("c{n}"),
                title: Some(format!("Session {n}")),
                last_activity_ms: (n % 7 != 0).then_some(i64::try_from(n).unwrap()),
            })
            .collect();
        newest_started(&mut started);
        assert_eq!(started.len(), STARTED_MAX);
        let newest = (0..(STARTED_MAX + 20)).rev().find(|n| n % 7 != 0).unwrap();
        assert_eq!(started[0].conversation_id, format!("c{newest}"));
        let times: Vec<Option<i64>> = started.iter().map(|s| s.last_activity_ms).collect();
        assert!(
            times.windows(2).all(|pair| pair[0] >= pair[1]),
            "newest first"
        );
    }
}

/// The sessions a session started ([`started_impl`]).
///
/// # Errors
///
/// None today; a `Result` for the IPC contract.
#[tauri::command]
pub async fn conversations_started(
    state: tauri::State<'_, Arc<AppState>>,
    tool: String,
    conversation_id: String,
) -> AppResult<Vec<StartedSessionInfo>> {
    Ok(started_impl(state.inner(), &tool, &conversation_id))
}

/// How many recent conversations (not open) the window holds.
pub const RECENT: usize = 200;
/// How far back they go.
pub const RECENT_MS: i64 = 7 * 24 * 3_600_000;

/// The working set of `all` (newest first, as [`list_impl`] gives it):
/// open or in a sidebar, and the newest [`RECENT`] of the last week.
#[must_use]
pub fn working(all: Vec<ConversationInfo>, now_ms: i64) -> Vec<ConversationInfo> {
    let mut recent = 0;
    all.into_iter()
        .filter(|conversation| {
            if conversation.live || conversation.inactive {
                return true;
            }
            let at = conversation.last_activity_ms.or(conversation.started_ms);
            // Every one of the last day: the rail's "ended today" stays true.
            if at.is_some_and(|at| at >= now_ms - crate::features::sessions::read_model::DAY_MS) {
                return true;
            }
            let fresh = at.is_some_and(|at| at >= now_ms - RECENT_MS);
            if fresh && recent < RECENT {
                recent += 1;
                return true;
            }
            false
        })
        .collect()
}

/// The copyable resume line for a conversation started outside AI Juggler
/// (its account's and its own ids; nothing else from the UI).
///
/// # Errors
///
/// As [`super::resume_command_impl`].
#[tauri::command]
pub async fn conversations_resume_command(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "the resume command", move |state| {
        super::resume_command_impl(state, &account_id, &conversation_id)
    })
    .await
}

/// "Continue in AI Juggler" on a conversation started outside it.
///
/// # Errors
///
/// As [`super::continue_impl`].
#[tauri::command]
pub async fn conversations_continue(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
) -> AppResult<crate::features::launch::dto::LaunchResult> {
    let result = crate::ipc::blocking(state.inner(), "continuing the conversation", move |state| {
        super::continue_impl(state, &account_id, &conversation_id)
    })
    .await?;
    crate::features::sessions::read_model::emit_sessions(&app, state.inner())?;
    Ok(result)
}

/// AJ-E2102: Go to it, for any session open now (AI Juggler's or not):
/// brings its app forward by the app's own process, then what the app
/// allows, within 1.5 s, off the main thread. Never raises a permission
/// prompt, except with `ask`: the user's own "Open the session itself",
/// after a Go to it said macOS would ask once (0.4.1). `pid` picks one
/// place when it's open in several.
///
/// # Errors
///
/// As [`super::goto::go_to_impl`].
#[tauri::command]
pub async fn conversations_go_to(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
    pid: Option<u32>,
    ask: Option<bool>,
) -> AppResult<GoToOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "going to the session", move |state| {
        super::goto::go_to_impl(
            state,
            &account_id,
            &conversation_id,
            pid,
            ask.unwrap_or(false),
            Arc::new(super::goto::MacDesktop),
        )
    })
    .await
}

/// AJ-E2104: a place's Open: brings the app whose process is `app_pid`
/// forward when an open session runs in it (never launches it), off the
/// main thread, within 1.5 s.
///
/// # Errors
///
/// As [`super::goto::bring_forward_impl`].
#[tauri::command]
pub async fn conversations_bring_forward(
    state: tauri::State<'_, Arc<AppState>>,
    app_pid: u32,
) -> AppResult<GoToOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "bringing the app forward", move |state| {
        super::goto::bring_forward_impl(state, app_pid, Arc::new(super::goto::MacDesktop))
    })
    .await
}

/// AJ-E2108: a session's last messages (up to 30 turns), read from the
/// end of its transcript while its page shows, with keys and tokens taken
/// out; kept nowhere. Deprecated in 0.3.5 for [`conversations_follow`]
/// (ADR-0024); kept for the read-only preview callers until its removal
/// in 0.4.0. `before` pages back, `from` follows along, `known`
/// (the stamp the page shows) answers `unchanged` without a read. Nothing
/// is read when the setting is off.
///
/// # Errors
///
/// As [`super::last_messages::last_messages_impl`].
#[tauri::command]
pub async fn conversations_last_messages(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
    known: Option<String>,
    before: Option<u64>,
    from: Option<u64>,
    max: Option<u32>,
) -> AppResult<super::last_messages::LastMessagesInfo> {
    let ask = super::last_messages::Ask {
        known,
        before,
        from,
        max: max.and_then(|max| usize::try_from(max).ok()),
    };
    crate::ipc::blocking(state.inner(), "the last messages", move |state| {
        super::last_messages::last_messages_impl(state, &account_id, &conversation_id, &ask)
    })
    .await
}

/// 0.3.5 (ADR-0024): follows a conversation while its page shows it:
/// its latest turns now (up to `max`, at most 60), redacted, then each
/// change on `channel` (turns new or grown, under stable ids), until the
/// page unfollows or stops saying it follows ([`conversations_keep`]).
/// Kept in memory only. Nothing is read when the setting is off.
///
/// # Errors
///
/// None today; the snapshot says what it couldn't read.
#[tauri::command]
pub async fn conversations_follow(
    state: tauri::State<'_, Arc<AppState>>,
    feeds: tauri::State<'_, Arc<super::feed::Feeds>>,
    account_id: String,
    conversation_id: String,
    max: Option<u32>,
    channel: tauri::ipc::Channel<super::feed::dto::ConversationDeltaInfo>,
) -> AppResult<super::feed::dto::ConversationSnapshotInfo> {
    let feeds = Arc::clone(feeds.inner());
    crate::ipc::blocking(state.inner(), "the conversation", move |state| {
        let path = state.conversations.path_of(&account_id, &conversation_id);
        let key = super::feed::FeedKey {
            account: account_id,
            conversation: conversation_id,
            subagent: None,
        };
        Ok(super::feed::follow_impl(
            state,
            &feeds,
            key,
            path,
            max,
            Arc::new(channel),
        ))
    })
    .await
}

/// 0.3.5: stops following (the page closed or was hidden).
///
/// # Errors
///
/// None today.
#[tauri::command]
pub async fn conversations_unfollow(
    state: tauri::State<'_, Arc<AppState>>,
    feeds: tauri::State<'_, Arc<super::feed::Feeds>>,
    subscription: u64,
) -> AppResult<()> {
    let feeds = Arc::clone(feeds.inner());
    crate::ipc::blocking(state.inner(), "the conversation", move |_| {
        super::feed::unfollow_impl(&feeds, subscription);
        Ok(())
    })
    .await
}

/// 0.3.5: the page still follows `subscriptions` (every half minute):
/// their leases go on. Answers those that ended, to follow again.
///
/// # Errors
///
/// None today.
#[tauri::command]
pub async fn conversations_keep(
    state: tauri::State<'_, Arc<AppState>>,
    feeds: tauri::State<'_, Arc<super::feed::Feeds>>,
    subscriptions: Vec<u64>,
) -> AppResult<Vec<u64>> {
    let feeds = Arc::clone(feeds.inner());
    crate::ipc::blocking(state.inner(), "the conversation", move |_| {
        let asked: Vec<u64> = subscriptions.into_iter().take(64).collect();
        Ok(feeds.keep(&asked, std::time::Instant::now()))
    })
    .await
}

/// 0.3.5: a page of the turns before `before` (a shown turn's offset)
/// in a followed conversation (up to `max`, at most 60). An answer with
/// no turns and an `earlier` passed a long stretch with none: read on.
///
/// # Errors
///
/// None today; the page says what it couldn't read.
#[tauri::command]
pub async fn conversations_earlier(
    state: tauri::State<'_, Arc<AppState>>,
    feeds: tauri::State<'_, Arc<super::feed::Feeds>>,
    subscription: u64,
    before: u64,
    max: Option<u32>,
) -> AppResult<super::feed::dto::EarlierPageInfo> {
    let feeds = Arc::clone(feeds.inner());
    let max = max
        .and_then(|max| usize::try_from(max).ok())
        .unwrap_or(super::feed::PAGE_DEFAULT);
    crate::ipc::blocking(state.inner(), "earlier messages", move |_| {
        Ok(feeds.earlier(subscription, before, max))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conversation(id: &str, last_ms: i64, live: bool, inactive: bool) -> ConversationInfo {
        ConversationInfo {
            id: id.to_owned(),
            tool: "t".to_owned(),
            account_id: "a".to_owned(),
            account_label: "A".to_owned(),
            project_id: None,
            cwd: None,
            title: None,
            git_branch: None,
            started_in: None,
            started_from: None,
            started_ms: Some(last_ms),
            last_activity_ms: Some(last_ms),
            tokens: None,
            live,
            session_id: None,
            places: Vec::new(),
            pid: None,
            state: None,
            probably: false,
            viewed_ms: None,
            background: false,
            inactive,
            subtask_count: None,
            app_archived: false,
            archived_here: false,
        }
    }

    #[test]
    fn the_window_holds_what_is_open_and_the_newest_of_the_last_week() {
        let now = 100 * RECENT_MS;
        let mut all = vec![
            conversation("old-but-open", 0, true, false),
            conversation("in-a-sidebar", 0, false, true),
        ];
        // Newest first, as the index gives them: 10 today, 300 this week, older.
        let day = crate::features::sessions::read_model::DAY_MS;
        all.extend((0..10).map(|n| conversation(&format!("today-{n}"), now - n, false, false)));
        all.extend(
            (0..300).map(|n| conversation(&format!("week-{n}"), now - day - 1 - n, false, false)),
        );
        all.push(conversation(
            "last-month",
            now - 2 * RECENT_MS,
            false,
            false,
        ));
        let kept = working(all, now);
        let ids: Vec<&str> = kept.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"old-but-open") && ids.contains(&"in-a-sidebar"));
        assert_eq!(
            kept.len(),
            2 + 10 + RECENT,
            "all of today's, 200 of the week's"
        );
        assert!(
            ids.contains(&"week-0") && !ids.contains(&"week-250") && !ids.contains(&"last-month")
        );
    }

    /// 0.3.9 (the maintainer's walk of 2026-11-04): a Claude app side
    /// session showed as a sibling of the session that started it. Each
    /// row says what it was started from, as the app recorded it.
    #[test]
    fn a_session_started_from_another_says_which() {
        use crate::features::conversations::scan::{LiveModel, StartedLink};
        let work = tempfile::tempdir().unwrap();
        let root = work.path().canonicalize().unwrap();
        let repo = root.join("Dev/app");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        let config = root.join("setup");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("settings.json"), "{}").unwrap();
        let dir = config
            .join("projects")
            .join(repo.to_string_lossy().replace(['/', '.'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        for id in ["plan", "ws-a", "alone"] {
            std::fs::write(
                dir.join(format!("{id}.jsonl")),
                format!(
                    "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"{id}\"}}}}\n",
                    repo.display()
                ),
            )
            .unwrap();
        }
        let mut registry = juggler_adapter::AdapterRegistry::new();
        registry.register(Arc::new(juggler_adapters::ClaudeCodeAdapter::with_bin(
            "/bin/true",
        )));
        let state = Arc::new(
            crate::state::AppState::init_with_registry(root.join("app-data"), registry).unwrap(),
        );
        crate::features::account::import::account_import_mode_impl(
            &state,
            juggler_adapters::CLAUDE_CODE_ID,
            config.to_str().unwrap(),
            "Work",
            crate::features::account::import::ImportRequest {
                in_place: true,
                ..crate::features::account::import::ImportRequest::default()
            },
        )
        .unwrap();
        state.conversations.set_live_model(LiveModel {
            started: vec![StartedLink {
                tool: "claude-code".into(),
                parent: "plan".into(),
                session_id: "ws-a".into(),
                title: None,
                last_activity_ms: None,
            }],
            ..LiveModel::default()
        });
        let listed = list_impl(&state).unwrap();
        let from = |id: &str| {
            listed
                .iter()
                .find(|row| row.id == id)
                .unwrap()
                .started_from
                .clone()
        };
        assert_eq!(from("ws-a").as_deref(), Some("plan"));
        assert_eq!(from("plan"), None);
        assert_eq!(from("alone"), None);
    }
}
