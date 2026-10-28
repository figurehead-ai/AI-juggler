//! AJ-E2501 — AI Juggler's own sources of notices: an account almost out
//! of room (and its reset), a session that finished, a crash and its
//! restart, an update. Each writes through the model ([`super::centre`]);
//! the views follow ([`super::changed`]). The words are plain: an
//! account's own name, a window as "5-hour" or "weekly", never an id, a
//! code or a confidence badge.
//!
//! The Needs you list comes from the window ([`super::centre::sync_needs`]);
//! nothing here duplicates it.

use std::sync::Arc;

use juggler_core::storage::{NewNotice, Storage};

use super::centre::{self, NoticeFacts, NoticeTarget};
use super::engine::{Kind, Target};
use crate::state::AppState;

/// The alert kinds that say an account is running out of room.
const LOW_KINDS: &[&str] = &["tray-amber", "actionable", "exhausted"];
/// Below this much used, an account that was almost out has room again
/// (its window reset).
const ROOM_AGAIN_USED_PCT: f64 = 50.0;

/// The words for a window, as the user reads it.
fn window_words(slot: &str) -> &'static str {
    if slot == "weekly" { "weekly" } else { "5-hour" }
}

/// What has reset, as the user reads it: "its week", "its 5-hour window".
fn reset_words(window: &str) -> &'static str {
    if window == "weekly" {
        "its week"
    } else {
        "its 5-hour window"
    }
}

/// The fullest window of an account now: `(window, used %, resets at)`.
fn fullest(
    display: &crate::features::quota::QuotaDisplayInfo,
) -> Option<(&'static str, Option<f64>, Option<i64>)> {
    [
        ("five_hour", &display.five_hour),
        ("weekly", &display.weekly),
    ]
    .into_iter()
    .filter_map(|(slot, window)| window.as_ref().map(|window| (slot, window)))
    .max_by(|a, b| {
        let used = |w: &crate::features::quota::WindowDisplay| w.percentage.unwrap_or(-1.0);
        used(a.1).total_cmp(&used(b.1))
    })
    .map(|(slot, window)| (window_words(slot), window.percentage, window.resets_at_ms))
}

fn live_sessions_on(storage: &Storage, account_id: &str) -> u32 {
    let live = storage
        .sessions()
        .live()
        .unwrap_or_default()
        .into_iter()
        .filter(|record| {
            storage
                .profiles()
                .get(&record.profile)
                .is_ok_and(|profile| profile.account.as_str() == account_id)
        })
        .count();
    u32::try_from(live).unwrap_or(u32::MAX)
}

/// The notice for an account running out of room, from its quota now.
#[must_use]
pub fn low_notice(
    account_id: &str,
    label: &str,
    window: &str,
    used: Option<f64>,
    resets_at_ms: Option<i64>,
    sessions: u32,
    run_out: bool,
) -> NewNotice {
    // Clamped to 0–100 first: the cast can't truncate.
    #[allow(clippy::cast_possible_truncation)]
    let left = used.map(|used| (100.0 - used).clamp(0.0, 100.0).round() as i64);
    let title = if run_out || left == Some(0) {
        format!("{label} has run out of room")
    } else {
        format!("{label} is almost out of room")
    };
    let detail = match left {
        Some(left) => format!("{left}% of its {window} window left"),
        None => format!("Its {window} window is running low"),
    };
    NewNotice {
        key: format!("quota:{account_id}"),
        kind: "quota".to_owned(),
        title,
        detail,
        data: centre::data_of(
            &NoticeTarget {
                account_id: Some(account_id.to_owned()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                window: Some(window.to_owned()),
                resets_at_ms,
                sessions: Some(sessions),
                account: Some(label.to_owned()),
                ..NoticeFacts::default()
            },
        ),
        action: Some("hand-over".to_owned()),
        meter: left,
        ..NewNotice::default()
    }
}

/// The notice for an account with room again.
#[must_use]
pub fn reset_notice(account_id: &str, label: &str, window: &str, at_ms: i64) -> NewNotice {
    NewNotice {
        key: format!("reset:{account_id}:{}", at_ms / 60_000),
        kind: "reset".to_owned(),
        title: format!("{label} has room again — {} reset", reset_words(window)),
        detail: "Sessions on it can carry on".to_owned(),
        data: centre::data_of(
            &NoticeTarget {
                account_id: Some(account_id.to_owned()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                window: Some(window.to_owned()),
                account: Some(label.to_owned()),
                ..NoticeFacts::default()
            },
        ),
        action: Some("open".to_owned()),
        ..NewNotice::default()
    }
}

/// A plan renewing soon (the digest's line), as a notice.
#[must_use]
pub fn renewal_notice(account_id: &str, label: &str, text: &str) -> NewNotice {
    NewNotice {
        key: format!("renewal:{account_id}:{text}"),
        kind: "renewal".to_owned(),
        title: format!("{label}: {text}"),
        data: centre::data_of(
            &NoticeTarget {
                account_id: Some(account_id.to_owned()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                account: Some(label.to_owned()),
                ..NoticeFacts::default()
            },
        ),
        action: Some("open".to_owned()),
        ..NewNotice::default()
    }
}

/// AJ-E611's alert pass, as notices: an account the pass found running
/// low opens (or refreshes) its need; one that has room again ends it and
/// says so. Called after each pass with what it emitted.
pub fn quota_pass(state: &Arc<AppState>, emitted: &[crate::commands::AlertEventInfo]) {
    let now = crate::commands::now_millis();
    let mut told = Vec::new();
    {
        let Ok(storage) = state.storage.lock() else {
            return;
        };
        let accounts = storage.accounts().list().unwrap_or_default();
        let open: Vec<String> = storage
            .notices()
            .open_needs()
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.kind == "quota")
            .map(|row| row.key)
            .collect();
        // The hourly digest's lines (a plan renewing soon): told once each,
        // under Earlier.
        for event in emitted.iter().filter(|event| event.kind == "digest") {
            let label = accounts
                .iter()
                .find(|account| account.id.as_str() == event.account)
                .map_or("An account", |account| account.label.as_str());
            let _ = centre::record(
                &storage,
                &renewal_notice(&event.account, label, &event.text),
                now,
            );
        }
        for account in &accounts {
            let id = account.id.as_str();
            let low_now = emitted
                .iter()
                .rfind(|event| event.account == id && LOW_KINDS.contains(&event.kind.as_str()));
            let is_open = open.iter().any(|key| *key == format!("quota:{id}"));
            if low_now.is_none() && !is_open {
                continue;
            }
            let display = crate::features::quota::quota_display_inner(&storage, account, now);
            let Some((window, used, resets_at)) = fullest(&display) else {
                continue;
            };
            let sessions = live_sessions_on(&storage, id);
            if is_open && low_now.is_none() && used.is_some_and(|used| used < ROOM_AGAIN_USED_PCT) {
                if centre::end_need(&storage, &format!("quota:{id}"), now).unwrap_or(false)
                    && let Ok(Some(info)) = centre::record(
                        &storage,
                        &reset_notice(id, &account.label, window, now),
                        now,
                    )
                {
                    told.push((Target::Account(id.to_owned()), Kind::Reset, info));
                }
                continue;
            }
            let run_out = low_now.is_some_and(|event| event.kind == "exhausted");
            let notice = low_notice(
                id,
                &account.label,
                window,
                used,
                resets_at,
                sessions,
                run_out,
            );
            if let Ok(Some(info)) = centre::open_need(&storage, &notice, now) {
                told.push((Target::Account(id.to_owned()), Kind::Quota, info));
            }
        }
    }
    for (target, kind, info) in told {
        super::model_event(state, target, kind, &info);
    }
    super::changed(state);
}

/// A session finished on its own: told once, under Earlier.
pub fn finished(state: &Arc<AppState>, session_id: &str) {
    let about = super::describe(state, session_id);
    let now = crate::commands::now_millis();
    let notice = NewNotice {
        key: format!("finished:{session_id}"),
        kind: "finished".to_owned(),
        title: format!("“{}” finished", about.name),
        detail: [about.place, about.account]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · "),
        project_id: about.project_id,
        data: centre::data_of(
            &NoticeTarget {
                session_id: Some(session_id.to_owned()),
                ..NoticeTarget::default()
            },
            &NoticeFacts::default(),
        ),
        action: Some("open".to_owned()),
        ..NewNotice::default()
    };
    let recorded = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| centre::record(&storage, &notice, now).ok().flatten());
    if recorded.is_some() {
        super::changed(state);
    }
}

/// A session crashed and was started again (`attempt` of `tries`): told
/// once per try, under Earlier; its action opens the new session.
pub fn restarted(
    state: &Arc<AppState>,
    crashed: &str,
    replacement: &str,
    attempt: usize,
    tries: usize,
) {
    let about = super::describe(state, crashed);
    let now = crate::commands::now_millis();
    let attempt = u32::try_from(attempt).unwrap_or(u32::MAX);
    let tries = u32::try_from(tries).unwrap_or(u32::MAX);
    let notice = NewNotice {
        key: format!("restarted:{crashed}:{attempt}"),
        kind: "restarted".to_owned(),
        title: format!(
            "“{}” crashed and was restarted (try {attempt} of {tries})",
            about.name
        ),
        detail: [about.place, about.account]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · "),
        project_id: about.project_id,
        data: centre::data_of(
            &NoticeTarget {
                session_id: Some(replacement.to_owned()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                attempt: Some(attempt),
                tries: Some(tries),
                ..NoticeFacts::default()
            },
        ),
        action: Some("see-why".to_owned()),
        ..NewNotice::default()
    };
    let recorded = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| centre::record(&storage, &notice, now).ok().flatten());
    if let Some(info) = recorded {
        super::model_event(
            state,
            Target::Session(replacement.to_owned()),
            Kind::Crashed,
            &info,
        );
        super::changed(state);
    }
}

/// The notice for an update, when the status is one worth telling: a new
/// version found, or one installed and ready.
#[must_use]
pub fn update_notice(status: &crate::updates::UpdateStatusInfo) -> Option<NewNotice> {
    let version = status.available.as_ref()?.version.clone();
    let (key, title) = match status.state.as_str() {
        "available" => (
            format!("update:{version}:available"),
            format!("AI Juggler {version} is available"),
        ),
        "installed" => (
            format!("update:{version}:installed"),
            format!("AI Juggler {version} is ready"),
        ),
        _ => return None,
    };
    Some(NewNotice {
        key,
        kind: "update".to_owned(),
        title,
        detail: if status.state == "installed" {
            "Restart when you're ready — nothing restarts by itself".to_owned()
        } else {
            "Nothing is installed until you choose".to_owned()
        },
        data: centre::data_of(
            &NoticeTarget {
                page: Some("updates".to_owned()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                version: Some(version),
                channel: Some(status.channel.clone()),
                ..NoticeFacts::default()
            },
        ),
        action: Some("whats-new".to_owned()),
        ..NewNotice::default()
    })
}

/// Where updating stands, as told to the window: a version found or
/// ready is told once, under Earlier.
pub fn update(state: &Arc<AppState>, status: &crate::updates::UpdateStatusInfo) {
    let Some(notice) = update_notice(status) else {
        return;
    };
    let now = crate::commands::now_millis();
    let recorded = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| centre::record(&storage, &notice, now).ok().flatten());
    if recorded.is_some() {
        super::changed(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_account_running_low_is_said_in_plain_words() {
        let notice = low_notice(
            "acct-1",
            "Work Max",
            "5-hour",
            Some(93.0),
            Some(5),
            2,
            false,
        );
        assert_eq!(notice.title, "Work Max is almost out of room");
        assert_eq!(notice.detail, "7% of its 5-hour window left");
        assert_eq!(notice.meter, Some(7));
        assert_eq!(notice.key, "quota:acct-1");
        assert_eq!(notice.action.as_deref(), Some("hand-over"));
        let out = low_notice("acct-1", "Work Max", "weekly", Some(100.0), None, 0, true);
        assert_eq!(out.title, "Work Max has run out of room");
        assert_eq!(out.detail, "0% of its weekly window left");
        let unknown = low_notice("acct-1", "Work Max", "5-hour", None, None, 0, false);
        assert_eq!(unknown.detail, "Its 5-hour window is running low");
        assert_eq!(unknown.meter, None);
        for words in [&notice.title, &notice.detail, &out.detail, &unknown.detail] {
            for internal in ["five_hour", "Measured", "Computed", "estimate", "acct-1"] {
                assert!(
                    !words.contains(internal),
                    "an internal word reached the copy"
                );
            }
        }
    }

    #[test]
    fn a_reset_names_the_window_it_was() {
        assert_eq!(
            reset_notice("a", "Personal Max", "weekly", 0).title,
            "Personal Max has room again — its week reset"
        );
        assert_eq!(
            reset_notice("a", "Work Max", "5-hour", 0).title,
            "Work Max has room again — its 5-hour window reset"
        );
    }

    #[test]
    fn an_update_is_told_when_found_and_when_ready_only() {
        let mut status = crate::updates::UpdateStatusInfo {
            current_version: "0.1.8".to_owned(),
            channel: "beta".to_owned(),
            edition: "main".to_owned(),
            app_name: "AI Juggler".to_owned(),
            channels: Vec::new(),
            configured: true,
            check_on_launch: true,
            state: "idle".to_owned(),
            available: None,
            progress: None,
            error: None,
            error_kind: None,
            error_detail: None,
            restart_wait: None,
            backup_path: None,
            checked_at_ms: None,
        };
        assert!(update_notice(&status).is_none());
        status.available = Some(crate::updates::AnnouncedInfo {
            version: "0.1.9".to_owned(),
            notes: String::new(),
            date_ms: None,
        });
        status.state = "available".to_owned();
        let found = update_notice(&status).unwrap();
        assert_eq!(found.title, "AI Juggler 0.1.9 is available");
        status.state = "installed".to_owned();
        let ready = update_notice(&status).unwrap();
        assert_eq!(ready.title, "AI Juggler 0.1.9 is ready");
        assert_ne!(found.key, ready.key);
        status.state = "downloading".to_owned();
        assert!(update_notice(&status).is_none());
    }
}
