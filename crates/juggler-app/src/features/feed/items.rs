//! Each journal's rows as feed items (D-118): their type, level, filter
//! chip, and the keys that coalesce and roll them up (`events.md` §1–6,
//! the proposal's §3). The level comes from the one table
//! ([`attention_level`]) where a need's kind decides it; the feed adds no
//! level of its own for those.

use juggler_core::model::ending_of;
use juggler_core::storage::feed::{
    FeedLifecycle, FeedMessage, FeedNotice, FeedSession, FeedSwitch, FeedTaskEvent,
};

use super::dto::{FeedAuthorInfo, FeedFactsInfo, FeedItemInfo};
use crate::features::notifications::centre::{
    LevelFacts, NoticeFacts, NoticeTarget, attention_level,
};

/// Same verb, App and project (the proposal §3.2).
pub const SAME_VERB_MS: i64 = 2 * 3_600_000;
/// A pool thread (§3.3).
pub const THREAD_MS: i64 = 30 * 60_000;
/// A pool's tasks (§3.4).
pub const TASKS_MS: i64 = 3_600_000;
/// An App started again by keep alive (M4: per App per day).
pub const DAY_MS: i64 = 24 * 3_600_000;

/// The chip each kind is under.
#[must_use]
pub fn category_of(kind: &str) -> &'static str {
    match kind {
        "failed" | "crashed" | "restarted" | "answered" | "switch_failed" => "needs",
        "finished" | "ended" | "stopped" | "reviewed" | "archived" => "finished",
        "switch" | "quota" | "reset" | "renewal" | "credits" => "switches",
        "task_done" | "task_review" | "task_blocked" | "message" => "pools",
        "this_mac" | "app_started" | "keep_alive_paused" | "update" | "found" => "mac",
        _ => "other",
    }
}

/// The notices kept as something that happened, by the feed's kind for
/// each (`credits` is a `quota` notice with credits in its facts).
pub const EARLIER_KINDS: &[(&str, &str)] = &[
    ("finished", "finished"),
    ("restarted", "restarted"),
    ("quota", "quota"),
    ("reset", "reset"),
    ("renewal", "renewal"),
    ("this-mac", "this_mac"),
    ("app-started", "app_started"),
    ("keep-alive-paused", "keep_alive_paused"),
    ("update", "update"),
    ("found", "found"),
    ("import", "import"),
];

/// The needs whose resolution is a quiet history line ("…no longer asks
/// you"). A review is the user's own mark; leftovers and room come and go
/// with This Mac's own items.
pub const ANSWERED_NEEDS: &[&str] = &[
    "asks", "stuck", "limit", "failed", "signin", "heavy", "cap", "message", "task",
];

/// Session endings, by what the lifecycle event says, and the user's marks.
pub const ENDINGS: &[(&str, &str)] = &[
    ("ended: finished", "finished"),
    ("ended: error", "failed"),
    ("ended: crashed", "crashed"),
    ("ended: ended with AI Juggler", "ended"),
];
/// See [`ENDINGS`]: the user's own acts.
pub const MINE: &[(&str, &str)] = &[
    ("ended: stopped by you", "stopped"),
    ("reviewed", "reviewed"),
    ("archived", "archived"),
];

/// Task changes the feed tells, by the word the log keeps for the status
/// it went to.
pub const TASK_CHANGES: &[(&str, &str)] = &[
    ("Done", "task_done"),
    ("In review", "task_review"),
    ("Blocked", "task_blocked"),
];

/// Agents' message kinds the feed shows (a note is chatter: only counted,
/// in its thread's count).
pub const MESSAGE_KINDS: &[&str] = &["handoff", "review_request", "question", "status"];

/// Of `table`'s `(stored, kind)` pairs, the stored words whose kind is
/// under `category` (all of them for none).
#[must_use]
pub fn stored_for(table: &[(&str, &str)], category: Option<&str>) -> Vec<String> {
    table
        .iter()
        .filter(|(_, kind)| category.is_none_or(|chip| category_of(kind) == chip))
        .map(|(stored, _)| (*stored).to_owned())
        .collect()
}

fn session_author(session: &FeedSession) -> FeedAuthorInfo {
    FeedAuthorInfo {
        kind: "app".to_owned(),
        name: None,
        provider: session.provider.clone(),
        folder: session.folder.clone(),
        folder_label: session.folder_label.clone(),
        desktop: session.kind.as_deref() == Some("desktop"),
    }
}

fn juggler() -> FeedAuthorInfo {
    FeedAuthorInfo {
        kind: "juggler".to_owned(),
        ..FeedAuthorInfo::default()
    }
}

/// A session's subject: the name the user gave it, else its folder's
/// name (the window names a session it holds by its own rule).
fn session_name(session: &FeedSession) -> String {
    session
        .label
        .clone()
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| folder_name(session.cwd.as_deref()))
}

fn folder_name(cwd: Option<&str>) -> String {
    cwd.and_then(|cwd| std::path::Path::new(cwd).file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "A session".to_owned())
}

fn session_facts(session: &FeedSession) -> FeedFactsInfo {
    FeedFactsInfo {
        project: session.project.clone(),
        project_id: session.project_id.clone(),
        cwd: session.cwd.clone(),
        ..FeedFactsInfo::default()
    }
}

/// A key for "same verb, App and project": its level inside, so a rollup
/// key never crosses a level boundary.
fn verb_key(verb: &str, folder: Option<&str>, project: Option<&str>, level: u8) -> String {
    format!(
        "v:{verb}:{}:{}:L{level}",
        folder.unwrap_or("-"),
        project.unwrap_or("-")
    )
}

fn item(
    id: String,
    at_ms: i64,
    kind: &str,
    level: u8,
    author: FeedAuthorInfo,
    subject: String,
) -> FeedItemInfo {
    FeedItemInfo {
        id,
        at_ms,
        kind: kind.to_owned(),
        category: category_of(kind).to_owned(),
        level,
        mine: false,
        coalesce: None,
        rollup: None,
        rollup_window_ms: 0,
        scope: None,
        author,
        subject,
        facts: FeedFactsInfo::default(),
        target: NoticeTarget::default(),
    }
}

/// A session ending, or the user's own mark on one.
#[must_use]
pub fn of_lifecycle(row: &FeedLifecycle) -> Option<FeedItemInfo> {
    let session = &row.session;
    let kind = ENDINGS
        .iter()
        .chain(MINE)
        .find(|(prefix, _)| row.event.starts_with(prefix))
        .map(|(_, kind)| *kind)?;
    let mine = MINE.iter().any(|(_, mine)| *mine == kind);
    let level = match kind {
        "failed" | "crashed" => 3,
        "finished" | "ended" => 1,
        _ => 0,
    };
    let author = if mine {
        FeedAuthorInfo {
            kind: "you".to_owned(),
            ..FeedAuthorInfo::default()
        }
    } else {
        session_author(session)
    };
    let mut out = item(
        format!("l:{}", row.key.rowid),
        row.key.at_ms,
        kind,
        level,
        author,
        session_name(session),
    );
    out.mine = mine;
    out.facts = session_facts(session);
    out.facts.detail = ending_of(&row.event).and_then(|(_, detail)| detail);
    out.facts.duration_ms = session
        .started_ms
        .map(|start| row.key.at_ms - start)
        .filter(|ms| *ms > 0);
    out.target.session_id = Some(session.id.clone());
    out.target.project_id.clone_from(&session.project_id);
    out.scope = session.folder.as_ref().map(|folder| format!("f:{folder}"));
    if mine {
        out.coalesce = Some(format!("mine:{}:{kind}", session.id));
    } else {
        out.coalesce = Some(format!("s:{}", session.id));
        if matches!(kind, "finished" | "ended") {
            out.rollup = Some(verb_key(
                kind,
                session.folder.as_deref(),
                session.project_id.as_deref(),
                level,
            ));
            out.rollup_window_ms = SAME_VERB_MS;
        }
    }
    Some(out)
}

#[derive(Default, serde::Deserialize)]
struct Data {
    #[serde(default)]
    target: NoticeTarget,
    #[serde(default)]
    facts: NoticeFacts,
}

/// A notice's feed kind and level: an earlier one by its kind (credits
/// are a quota notice with credits), a resolved need as a quiet history
/// line; `None` for one the feed doesn't tell.
fn notice_kind(notice: &juggler_core::storage::Notice, data: &Data) -> Option<(&'static str, u8)> {
    if notice.needs {
        return ANSWERED_NEEDS
            .contains(&notice.kind.as_str())
            .then_some(("answered", 0));
    }
    let kind = EARLIER_KINDS
        .iter()
        .find(|(stored, _)| *stored == notice.kind)?
        .1;
    let kind = if kind == "quota" && data.facts.credits.is_some() {
        "credits"
    } else {
        kind
    };
    let level = match kind {
        "quota" => attention_level(
            "quota",
            LevelFacts {
                left: notice.meter,
                live_sessions: data.facts.sessions,
                ..LevelFacts::default()
            },
        ),
        "restarted" => 3,
        "keep_alive_paused" => 2,
        "this_mac" | "import" => 0,
        _ => 1,
    };
    Some((kind, level))
}

const ACCOUNT_KINDS: [&str; 4] = ["quota", "credits", "reset", "renewal"];

/// Who a notice's item is by, and what it is about.
fn notice_who(kind: &str, session: Option<&FeedSession>, data: &Data) -> (FeedAuthorInfo, String) {
    if ACCOUNT_KINDS.contains(&kind) {
        let author = FeedAuthorInfo {
            kind: "account".to_owned(),
            name: data.facts.account.clone(),
            folder: data.target.account_id.clone(),
            ..FeedAuthorInfo::default()
        };
        return (author, data.facts.account.clone().unwrap_or_default());
    }
    let author = match session {
        Some(session) if matches!(kind, "finished" | "answered") => session_author(session),
        _ => juggler(),
    };
    (author, session.map(session_name).unwrap_or_default())
}

/// A notice item's coalesce and rollup keys (one session one card; one
/// Account's window; one version; an App started again, per day).
fn notice_keys(
    out: &mut FeedItemInfo,
    notice: &juggler_core::storage::Notice,
    session: Option<&FeedSession>,
    data: &Data,
) {
    let account = out
        .target
        .account_id
        .clone()
        .or_else(|| data.facts.account.clone())
        .unwrap_or_default();
    let second = notice.key.split(':').nth(1);
    out.coalesce = match out.kind.as_str() {
        // The crash a restart follows, by the session its key names.
        "finished" => session.map(|s| format!("s:{}", s.id)),
        "restarted" => second.map(|crashed| format!("s:{crashed}")),
        "quota" | "credits" => Some(format!(
            "q:{account}:{}",
            data.facts.window.as_deref().unwrap_or("-")
        )),
        "renewal" => Some(format!("renewal:{account}")),
        "update" => data
            .facts
            .version
            .as_ref()
            .map(|version| format!("u:{version}")),
        "app_started" => None,
        _ => Some(format!("n:{}", notice.key)),
    };
    match out.kind.as_str() {
        "app_started" => {
            out.rollup = Some(format!("as:{}:L{}", second.unwrap_or("-"), out.level));
            out.rollup_window_ms = DAY_MS;
        }
        "finished" => {
            out.rollup = session.map(|s| {
                verb_key(
                    "finished",
                    s.folder.as_deref(),
                    s.project_id.as_deref(),
                    out.level,
                )
            });
            out.rollup_window_ms = SAME_VERB_MS;
        }
        _ => {}
    }
}

/// A notice: what happened (Earlier), or a need once resolved.
#[must_use]
pub fn of_notice(row: &FeedNotice) -> Option<FeedItemInfo> {
    let notice = &row.notice;
    let data: Data = serde_json::from_str(&notice.data).unwrap_or_default();
    let session = row.session.as_ref();
    let (kind, level) = notice_kind(notice, &data)?;
    let (author, subject) = notice_who(kind, session, &data);
    let mut out = item(
        format!("n:{}", notice.id),
        row.key.at_ms,
        kind,
        level,
        author,
        subject,
    );
    let mut facts = session.map(session_facts).unwrap_or_default();
    facts.project = facts.project.or_else(|| notice.project.clone());
    facts.project_id = facts.project_id.or_else(|| notice.project_id.clone());
    facts.attempt = data.facts.attempt;
    facts.tries = data.facts.tries;
    facts.window.clone_from(&data.facts.window);
    facts.resets_at_ms = data.facts.resets_at_ms;
    facts.sessions = data.facts.sessions;
    facts.credits.clone_from(&data.facts.credits);
    facts.version.clone_from(&data.facts.version);
    facts.meter = notice.meter;
    facts.line = Some(notice.title.clone());
    facts.need = notice.needs.then(|| notice.kind.clone());
    out.facts = facts;
    out.target = data.target.clone();
    out.scope = session
        .and_then(|s| s.folder.as_ref())
        .map(|folder| format!("f:{folder}"));
    notice_keys(&mut out, notice, session, &data);
    Some(out)
}

/// A switch that finished or failed.
#[must_use]
pub fn of_switch(row: &FeedSwitch) -> FeedItemInfo {
    let failed = row.state == "failed";
    let kind = if failed { "switch_failed" } else { "switch" };
    let mut out = item(
        format!("w:{}", row.key.rowid),
        row.key.at_ms,
        kind,
        if failed { 2 } else { 1 },
        juggler(),
        row.work_title.clone(),
    );
    out.facts = FeedFactsInfo {
        project: row.project.clone(),
        project_id: row.project_id.clone(),
        from_label: row.from_label.clone(),
        from_provider: Some(row.from_provider.clone()),
        to_label: row.to_label.clone(),
        to_provider: Some(row.to_provider.clone()),
        mechanism: Some(row.mechanism.clone()),
        fidelity: row.fidelity.clone(),
        detail: row.fail_reason.clone(),
        ..FeedFactsInfo::default()
    };
    out.target.project_id.clone_from(&row.project_id);
    out.coalesce = Some(format!("w:{}", row.id));
    out
}

/// A task's status change.
#[must_use]
pub fn of_task(row: &FeedTaskEvent) -> Option<FeedItemInfo> {
    let to = row.detail.as_deref()?.rsplit("→ ").next()?;
    let kind = TASK_CHANGES.iter().find(|(word, _)| *word == to)?.1;
    let level = u8::from(kind != "task_done");
    let author = FeedAuthorInfo {
        kind: "agent".to_owned(),
        name: Some(row.actor_name.clone()),
        folder: None,
        ..FeedAuthorInfo::default()
    };
    let mut out = item(
        format!("t:{}", row.key.rowid),
        row.key.at_ms,
        kind,
        level,
        author,
        row.title.clone().unwrap_or_default(),
    );
    out.facts.pool.clone_from(&row.pool);
    out.facts.pool_id = Some(row.pool_id.clone());
    out.facts.change.clone_from(&row.detail);
    out.target.task_id = Some(row.task_id.clone());
    out.target.pool_id = Some(row.pool_id.clone());
    out.scope = Some(format!("p:{}", row.pool_id));
    out.rollup = Some(format!("t:{}:{kind}:L{level}", row.pool_id));
    out.rollup_window_ms = TASKS_MS;
    Some(out)
}

/// The most of an agent's words a feed item quotes.
const QUOTE_CHARS: usize = 280;

/// An agent's message of a highlight kind.
#[must_use]
pub fn of_message(row: &FeedMessage) -> FeedItemInfo {
    let level = u8::from(row.kind == "review_request");
    let author = FeedAuthorInfo {
        kind: if row.sender_kind == "user" {
            "you"
        } else {
            "agent"
        }
        .to_owned(),
        name: Some(row.sender_name.clone()),
        ..FeedAuthorInfo::default()
    };
    let mut out = item(
        format!("m:{}", row.key.rowid),
        row.key.at_ms,
        "message",
        level,
        author,
        row.thread.clone(),
    );
    out.mine = row.sender_kind == "user";
    let quote: String = row.body.chars().take(QUOTE_CHARS).collect();
    out.facts.quote = Some(if quote.len() < row.body.len() {
        format!("{}…", quote.trim_end())
    } else {
        quote
    });
    out.facts.pool.clone_from(&row.pool);
    out.facts.pool_id = Some(row.pool_id.clone());
    out.facts.thread = Some(row.thread.clone());
    out.facts.thread_count = Some(row.thread_count);
    out.facts.change = Some(row.kind.clone());
    out.target.pool_id = Some(row.pool_id.clone());
    out.target.thread_id = Some(row.thread_id.clone());
    out.scope = Some(format!("p:{}", row.pool_id));
    out.rollup = Some(format!("m:{}:L{level}", row.thread_id));
    out.rollup_window_ms = THREAD_MS;
    out
}
