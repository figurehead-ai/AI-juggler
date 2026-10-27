//! AJ-E2502, D-49 — Needs you, worked out in the backend: the one list
//! every view reads (the rail, Home, the bell, the app badge, the menu-bar
//! item, the system notifications), on the backend's own tick, so it holds
//! with no window open.
//!
//! What needs the user:
//! - AI Juggler's sessions that wait, may be stuck, hit their limit or
//!   stopped with an error (not parked, archived or in a forgotten project);
//! - sessions other apps run that ask, or finished something not yet looked
//!   at (the seen marks AI Juggler keeps, `conversation_seen`, and the
//!   app's own last view), in apps not turned off in Settings;
//! - an account that needs its sign-in;
//! - this computer under pressure (its alerts), a session working it hard,
//!   processes left running;
//! - a monthly cap reached, an agent's message or task that waits;
//! - (the model's own) an account almost out of room.
//!
//! A snoozed subject leaves the count until its snooze ends; the tick ends
//! it, and the views follow ([`super::changed`]).

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::centre::{self, NeedInput, NoticeTarget};
use super::views::{OpenApp, OpenNowInput};
use crate::commands::{AccountInfo, InstanceInfo, NeedsAttention};
use crate::features::conversations::dto::ConversationInfo;
use crate::features::messages::dto::MessageNeedInfo;
use crate::features::tasks::dto::TaskNeedInfo;
use crate::machine_health::ThisMacInfo;
use crate::state::AppState;

/// AI Juggler's own place, as the rail names it.
pub const OWN_APP: &str = "AI Juggler";
/// The seen marks (AJ-E2107), as the window keeps them.
const SEEN_KEY: &str = "conversation_seen";
/// How long an account's sign-in check is reused (a Keychain lookup each).
const SIGN_INS_FRESH_MS: i64 = 30_000;

/// What the needs are worked out from, read once per tick.
#[derive(Default)]
pub struct NeedSources {
    /// AI Juggler's working set of sessions.
    pub sessions: Vec<InstanceInfo>,
    /// Other apps' sessions (the working set).
    pub conversations: Vec<ConversationInfo>,
    /// The accounts in use.
    pub accounts: Vec<AccountInfo>,
    /// Project id → name, and the forgotten ones.
    pub projects: HashMap<String, String>,
    /// Projects the user forgot (their sessions leave the list).
    pub forgotten: HashSet<String>,
    /// Apps turned off in Settings › Desktop apps (their ids).
    pub hidden_apps: HashSet<String>,
    /// When each other app's session was last looked at here.
    pub seen: HashMap<String, i64>,
    /// This computer, as last sampled.
    pub this_mac: Option<ThisMacInfo>,
    /// Monthly caps that need a look.
    pub caps: Vec<crate::features::budgets::BudgetInfo>,
    /// Agents' messages and tasks that wait on the user.
    pub messages: Vec<MessageNeedInfo>,
    /// …and their tasks.
    pub tasks: Vec<TaskNeedInfo>,
    /// The words a tool uses for its own sign-in, by tool id.
    pub sign_in_words: HashMap<String, String>,
    /// This system's name for the computer ("Mac").
    pub computer: &'static str,
}

fn folder(cwd: Option<&str>) -> Option<String> {
    let name = std::path::Path::new(cwd?)
        .file_name()?
        .to_string_lossy()
        .into_owned();
    (!name.is_empty()).then_some(name)
}

/// A branch worth naming (never a bare "main").
fn telling(branch: Option<&str>) -> Option<&str> {
    branch.filter(|branch| {
        !matches!(
            branch.to_lowercase().as_str(),
            "main" | "master" | "trunk" | "develop" | "dev"
        )
    })
}

/// A session's place as This Mac names it: its project (else its folder),
/// and its branch and account.
fn place_of(row: &InstanceInfo, projects: &HashMap<String, String>) -> (String, String) {
    let title = row
        .project_id
        .as_ref()
        .and_then(|id| projects.get(id).cloned())
        .or_else(|| folder(Some(&row.cwd)))
        .unwrap_or_else(|| row.account_label.clone());
    let detail = join([row.git_branch.clone(), Some(row.account_label.clone())]);
    (title, detail)
}

/// A session's name as its rail row says it.
fn session_name(row: &InstanceInfo) -> String {
    row.label
        .clone()
        .or_else(|| row.git_branch.clone())
        .or_else(|| folder(Some(&row.cwd)))
        .unwrap_or_else(|| row.account_label.clone())
}

/// Another app's session's name as its rail row says it.
fn conversation_name(c: &ConversationInfo) -> String {
    if let Some(title) = &c.title {
        return title.clone();
    }
    if let Some(branch) = telling(c.git_branch.as_deref()) {
        return branch.to_owned();
    }
    folder(c.cwd.as_deref()).map_or_else(
        || "Untitled session".to_owned(),
        |folder| format!("Untitled session · {folder}"),
    )
}

fn quoted(kind: &str, name: &str) -> String {
    match kind {
        "asks" => format!("“{name}” needs your reply"),
        "stuck" => format!("“{name}” may be stuck"),
        "limit" => format!("“{name}” hit its usage limit"),
        "failed" => format!("“{name}” stopped with an error"),
        _ => format!("“{name}” is done — review it"),
    }
}

fn join(parts: impl IntoIterator<Item = Option<String>>) -> String {
    parts
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// An own session's need: its attention, else a status that is one.
fn session_kind(row: &InstanceInfo) -> Option<&'static str> {
    let attention = row.needs_attention.or(match row.status.as_str() {
        "waiting_user" => Some(NeedsAttention::WaitingUser),
        "stuck" => Some(NeedsAttention::Stuck),
        "quota_blocked" => Some(NeedsAttention::QuotaBlocked),
        "failed" => Some(NeedsAttention::Failed),
        _ => None,
    })?;
    Some(match attention {
        NeedsAttention::WaitingUser => "asks",
        NeedsAttention::Stuck => "stuck",
        NeedsAttention::QuotaBlocked => "limit",
        NeedsAttention::Failed => "failed",
    })
}

/// Another app's session's need: it asks, or it finished something not
/// looked at since.
fn conversation_kind(c: &ConversationInfo, seen: Option<i64>) -> Option<&'static str> {
    if !c.live || c.session_id.is_some() {
        return None;
    }
    match c.state.as_deref() {
        Some("asks_you") => Some("asks"),
        Some("done_review") => {
            let looked = seen.unwrap_or(0).max(c.viewed_ms.unwrap_or(0));
            (looked < c.last_activity_ms.unwrap_or(0)).then_some("review")
        }
        _ => None,
    }
}

fn hidden(c: &ConversationInfo, apps: &HashSet<String>) -> bool {
    c.places
        .first()
        .and_then(|place| place.app_id.as_ref())
        .is_some_and(|id| apps.contains(id))
}

fn reason_sentence(words: &[String]) -> String {
    let joined = words.join("; ");
    if joined.starts_with("macOS") {
        return joined;
    }
    let mut chars = joined.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// Who a machine alert is about, and what of theirs is high.
fn alert_subject(
    alert: &crate::machine_alerts::OverloadAlertInfo,
    name_of: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    use juggler_supervisor::machine::memory_words;
    if !alert.other_apps.is_empty() {
        let apps: Vec<String> = alert
            .other_apps
            .iter()
            .map(|app| {
                format!(
                    "{} {}{}",
                    app.name,
                    if app.processes > 1 { "about " } else { "" },
                    memory_words(app.rss_bytes)
                )
            })
            .collect();
        return Some(format!(
            "Other apps are using most of the memory: {}",
            apps.join(", ")
        ));
    }
    let who = match (&alert.session_id, &alert.place) {
        (Some(id), _) => Some(name_of(id).unwrap_or_else(|| "a session".to_owned())),
        (None, Some(_)) => alert.subject_name.clone(),
        (None, None) => None,
    }?;
    let figures: Vec<String> = [
        alert.subject_cpu.map(|cpu| format!("CPU {cpu:.0}%")),
        alert.subject_bytes.map(|bytes| {
            format!(
                "{}{}",
                if alert.subject_estimate { "about " } else { "" },
                memory_words(bytes)
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    if figures.is_empty() {
        return None;
    }
    let unit = if alert
        .place
        .as_deref()
        .is_some_and(|place| place.starts_with("app:"))
    {
        "sessions"
    } else {
        "windows"
    };
    let across = alert
        .subject_windows
        .filter(|count| *count > 1)
        .map_or_else(String::new, |count| format!(" across {count} {unit}"));
    Some(format!("{who} is using {}{across}", figures.join(" and ")))
}

/// This computer's own need: its overload alerts, or one already under
/// pressure.
fn room_need(
    mac: &ThisMacInfo,
    computer: &str,
    name_of: &dyn Fn(&str) -> Option<String>,
) -> Option<(String, String)> {
    if !mac.machine.sampled {
        return None;
    }
    let alerts: Vec<_> = mac
        .alerts
        .iter()
        .filter(|alert| alert.kind != "heavy")
        .collect();
    if let Some(first) = alerts.first() {
        let details: Vec<String> = alerts.iter().map(|alert| alert.detail.clone()).collect();
        let detail = [
            Some(reason_sentence(&details)),
            alert_subject(first, name_of),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(". ");
        return Some((first.title.clone(), detail));
    }
    (mac.machine.summary.level == "pressure").then(|| {
        let reasons: Vec<String> = mac
            .machine
            .summary
            .reasons
            .iter()
            .take(2)
            .map(|reason| reason.words.clone())
            .collect();
        (
            format!("Your {computer} is under pressure"),
            reason_sentence(&reasons),
        )
    })
}

/// Needs you, and what is open now, from `sources` (pure: the tests drive
/// it with no window and no tick).
#[must_use]
#[allow(clippy::too_many_lines)] // one list, kind by kind
pub fn work_out<S: std::hash::BuildHasher>(
    sources: &NeedSources,
    now_snoozed: &HashSet<String, S>,
) -> (Vec<NeedInput>, OpenNowInput) {
    let project = |id: Option<&String>| id.and_then(|id| sources.projects.get(id)).cloned();
    let mut needs = Vec::new();
    let mut listed_sessions = HashSet::new();
    let mut open = OpenNowInput::default();
    let mut by_app: HashMap<String, u32> = HashMap::new();
    for row in &sources.sessions {
        let gone = row.parked.is_some()
            || row.lifecycle.as_deref() == Some("archived")
            || row
                .project_id
                .as_ref()
                .is_some_and(|id| sources.forgotten.contains(id));
        if gone {
            continue;
        }
        let key = format!("s:{}", row.id);
        if let Some(kind) = session_kind(row) {
            listed_sessions.insert(row.id.clone());
            needs.push(NeedInput {
                kind: kind.to_owned(),
                title: quoted(kind, &session_name(row)),
                detail: join([
                    project(row.project_id.as_ref()),
                    Some(OWN_APP.to_owned()),
                    Some(row.account_label.clone()),
                ]),
                project_id: row.project_id.clone(),
                project: project(row.project_id.as_ref()),
                target: NoticeTarget {
                    session_id: Some(row.id.clone()),
                    project_id: row.project_id.clone(),
                    ..NoticeTarget::default()
                },
                action: Some(if kind == "limit" { "limit" } else { "open" }.to_owned()),
                key: key.clone(),
            });
            if !now_snoozed.contains(&key) {
                continue;
            }
        }
        if row.pinned == Some(true) || matches!(row.status.as_str(), "running" | "starting") {
            open.total += 1;
            open.working += u32::from(row.status == "running");
            *by_app.entry(OWN_APP.to_owned()).or_default() += 1;
        }
    }
    for c in &sources.conversations {
        if hidden(c, &sources.hidden_apps) || c.session_id.is_some() {
            continue;
        }
        let key = format!("c:{}/{}", c.account_id, c.id);
        let place = c.places.first();
        if let Some(kind) = conversation_kind(c, sources.seen.get(&key).copied()) {
            let profile =
                place.and_then(|place| place.profile.clone().or_else(|| place.instance.clone()));
            needs.push(NeedInput {
                kind: kind.to_owned(),
                title: quoted(kind, &conversation_name(c)),
                detail: join([
                    project(c.project_id.as_ref()).or_else(|| folder(c.cwd.as_deref())),
                    place.map(|place| place.app.clone()),
                    profile.clone(),
                    Some(c.account_label.clone()).filter(|label| Some(label) != profile.as_ref()),
                ]),
                project_id: c.project_id.clone(),
                project: project(c.project_id.as_ref()),
                target: NoticeTarget {
                    conversation_id: Some(c.id.clone()),
                    account_id: Some(c.account_id.clone()),
                    project_id: c.project_id.clone(),
                    ..NoticeTarget::default()
                },
                action: Some("open".to_owned()),
                key: key.clone(),
            });
            if !now_snoozed.contains(&key) {
                continue;
            }
        }
        if c.live && !c.background {
            open.total += 1;
            open.working += u32::from(c.state.as_deref() == Some("working"));
            *by_app
                .entry(place.map_or_else(|| "An app".to_owned(), |place| place.app.clone()))
                .or_default() += 1;
        }
    }
    for account in sources
        .accounts
        .iter()
        .filter(|account| !matches!(account.login_state.as_str(), "signed_in" | "logged_in"))
    {
        let own = sources
            .sign_in_words
            .get(&account.provider)
            .cloned()
            .unwrap_or_else(|| "Its own sign-in".to_owned());
        needs.push(NeedInput {
            key: format!("signin:{}", account.id),
            kind: "signin".to_owned(),
            title: format!("{} needs its sign-in", account.label),
            detail: format!("{own}, in a terminal here — about thirty seconds. AI Juggler never sees your password."),
            target: NoticeTarget {
                account_id: Some(account.id.clone()),
                ..NoticeTarget::default()
            },
            action: Some("sign-in".to_owned()),
            ..NeedInput::default()
        });
    }
    if let Some(mac) = &sources.this_mac {
        let names: HashMap<&str, String> = sources
            .sessions
            .iter()
            .map(|row| (row.id.as_str(), place_of(row, &sources.projects).0))
            .collect();
        let name_of = |id: &str| names.get(id).cloned();
        if let Some((title, detail)) = room_need(mac, sources.computer, &name_of) {
            needs.push(NeedInput {
                key: "room".to_owned(),
                kind: "room".to_owned(),
                title,
                detail,
                target: NoticeTarget {
                    page: Some("this_mac".to_owned()),
                    ..NoticeTarget::default()
                },
                action: Some("this-computer".to_owned()),
                ..NeedInput::default()
            });
        }
        for tree in &mac.heavy {
            let Some(id) = tree
                .session_id
                .as_ref()
                .filter(|id| !listed_sessions.contains(*id))
            else {
                continue;
            };
            let Some(row) = sources.sessions.iter().find(|row| row.id == *id) else {
                continue;
            };
            let helpers = match tree.helpers {
                0 => None,
                1 => Some("1 more process".to_owned()),
                n => Some(format!("{n} more processes")),
            };
            needs.push(NeedInput {
                key: format!("heavy:{id}"),
                kind: "heavy".to_owned(),
                title: format!(
                    "Working your {} hard — {}",
                    sources.computer,
                    tree.heavy_note
                        .clone()
                        .unwrap_or_else(|| "a lot of memory or CPU".to_owned())
                ),
                detail: {
                    let (title, detail) = place_of(row, &sources.projects);
                    join([Some(title), Some(detail), helpers])
                },
                project_id: row.project_id.clone(),
                project: project(row.project_id.as_ref()),
                target: NoticeTarget {
                    session_id: Some(id.clone()),
                    ..NoticeTarget::default()
                },
                action: Some("open".to_owned()),
            });
        }
        if mac.leftover_processes > 0 {
            let n = mac.leftover_processes;
            needs.push(NeedInput {
                key: "leftovers".to_owned(),
                kind: "leftovers".to_owned(),
                title: if n == 1 {
                    "1 process left running".to_owned()
                } else {
                    format!("{n} processes left running")
                },
                detail: "after their sessions closed".to_owned(),
                target: NoticeTarget {
                    page: Some("processes".to_owned()),
                    ..NoticeTarget::default()
                },
                action: Some("clean-up".to_owned()),
                ..NeedInput::default()
            });
        }
    }
    for cap in sources
        .caps
        .iter()
        .filter(|cap| cap.level != "under" && !cap.seen)
    {
        #[allow(clippy::cast_possible_truncation)] // a percentage, whole
        let percent = if cap.monthly_usd > 0.0 {
            (cap.spent_usd.unwrap_or(0.0) / cap.monthly_usd * 100.0).floor() as i64
        } else {
            0
        };
        needs.push(NeedInput {
            key: format!("cap:{}", cap.id),
            kind: "cap".to_owned(),
            title: if cap.level == "over" {
                format!("{} is over its monthly cap", cap.name)
            } else {
                format!("{} is at {percent}% of its monthly cap", cap.name)
            },
            target: NoticeTarget {
                page: Some("usage".to_owned()),
                ..NoticeTarget::default()
            },
            action: Some("usage".to_owned()),
            ..NeedInput::default()
        });
    }
    for message in &sources.messages {
        needs.push(NeedInput {
            key: format!("message:{}", message.key),
            kind: "message".to_owned(),
            title: message.title.clone(),
            detail: join([
                Some(message.pool_name.clone()),
                Some(message.detail.clone()),
            ]),
            target: NoticeTarget {
                pool_id: Some(message.pool_id.clone()),
                session_id: message.session_id.clone(),
                ..NoticeTarget::default()
            },
            action: Some("pool".to_owned()),
            ..NeedInput::default()
        });
    }
    for task in &sources.tasks {
        needs.push(NeedInput {
            key: format!("task:{}", task.id),
            kind: "task".to_owned(),
            title: task.title.clone(),
            detail: join([Some(task.pool_name.clone()), Some(task.detail.clone())]),
            project_id: task.project_id.clone(),
            target: NoticeTarget {
                pool_id: Some(task.pool_id.clone()),
                task_id: task.task_id.clone(),
                project_id: task.project_id.clone(),
                ..NoticeTarget::default()
            },
            action: Some("pool".to_owned()),
            ..NeedInput::default()
        });
    }
    let mut apps: Vec<OpenApp> = by_app
        .into_iter()
        .map(|(app, count)| OpenApp { app, count })
        .collect();
    apps.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.app.cmp(&b.app)));
    open.apps = apps;
    (needs, open)
}

/// The local first of this month, epoch ms (budgets count from it).
fn month_start(now: i64, offset_hours: i64) -> i64 {
    let offset = offset_hours * 3_600_000;
    let local = now + offset;
    let day = local.div_euclid(86_400_000);
    // Howard Hinnant's civil-from-days, for the day of the month.
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dom = doy - (153 * mp + 2) / 5; // 0-based
    (day - dom) * 86_400_000 - offset
}

/// Reads the sources now (the storage lock is held only to read).
fn read_sources(state: &Arc<AppState>, now: i64) -> NeedSources {
    let clock = state.notifier.clock();
    let mut sources = NeedSources {
        sessions: crate::commands::instances_shared_snapshot(state).unwrap_or_default(),
        conversations: crate::features::conversations::commands::list_impl(state)
            .map(|all| crate::features::conversations::commands::working(all, now))
            .unwrap_or_default(),
        accounts: state.notifier.sign_ins(now, SIGN_INS_FRESH_MS, || {
            crate::commands::accounts_info(state).unwrap_or_default()
        }),
        this_mac: state.notifier.this_mac(),
        caps: crate::features::budgets::list_impl(
            state,
            month_start(now, clock.utc_offset_hours.unwrap_or(0)),
        )
        .unwrap_or_default(),
        messages: crate::features::messages::needs::needs(state).unwrap_or_default(),
        tasks: crate::features::tasks::needs::needs(state).unwrap_or_default(),
        computer: juggler_platform::system::computer_noun().0,
        ..NeedSources::default()
    };
    for id in state.registry.ids() {
        if let Some(adapter) = state.registry.get(&id) {
            let spec = adapter.spec();
            sources.sign_in_words.insert(spec.id, spec.sign_in.own);
        }
    }
    if let Ok(storage) = state.storage.lock() {
        for project in storage.projects().list().unwrap_or_default() {
            let id = project.id.as_str().to_owned();
            if project.forgotten_at.is_some() {
                sources.forgotten.insert(id.clone());
            }
            sources.projects.insert(id, project.name.clone());
        }
        let settings = storage.app_settings();
        sources.seen = settings
            .get(SEEN_KEY)
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<HashMap<String, f64>>(&raw).ok())
            .map(|marks| {
                #[allow(clippy::cast_possible_truncation)] // epoch ms
                marks
                    .into_iter()
                    .map(|(key, at)| (key, at as i64))
                    .collect()
            })
            .unwrap_or_default();
        for (key, value) in settings.list().unwrap_or_default() {
            if let Some(app) = key.strip_prefix("show_sessions_from.")
                && value == "0"
            {
                sources.hidden_apps.insert(app.to_owned());
            }
        }
    }
    sources
}

/// The tick: works Needs you out, keeps the model in step, ends the
/// snoozes whose time has come, and tells the views when anything they
/// show changed (a snoozed need back in the count included). `true` when
/// it did.
pub fn refresh(state: &Arc<AppState>) -> bool {
    let now = crate::commands::now_millis();
    let sources = read_sources(state, now);
    refresh_from(state, &sources, now)
}

/// [`refresh`] from sources already read (the tests drive it).
pub fn refresh_from(state: &Arc<AppState>, sources: &NeedSources, now: i64) -> bool {
    let (changed, open) = {
        let Ok(storage) = state.storage.lock() else {
            return false;
        };
        let ended = storage.notices().end_snoozes(now).unwrap_or(0);
        let snoozed: HashSet<String> = storage
            .notices()
            .snoozes(now)
            .unwrap_or_default()
            .into_iter()
            .map(|(subject, _)| subject)
            .collect();
        let (needs, open) = work_out(sources, &snoozed);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for need in &needs {
            (&need.key, &need.kind, &need.title, &need.detail).hash(&mut hasher);
        }
        open.total.hash(&mut hasher);
        open.working.hash(&mut hasher);
        for app in &open.apps {
            (&app.app, app.count).hash(&mut hasher);
        }
        let signature = hasher.finish();
        let same = state.notifier.needs_signature(signature);
        if !same {
            let _ = centre::sync_needs(&storage, &needs, now);
        }
        (!same || ended > 0, open)
    };
    state.notifier.set_open_now(open);
    if changed {
        super::changed(state);
    }
    changed
}

#[cfg(test)]
mod tests;
