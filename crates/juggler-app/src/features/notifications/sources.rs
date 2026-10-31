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

/// A used-up window that the account's credits carry on past (the
/// maintainer's report, 0.3.2: "has run out of room" while Codex kept
/// working on its credits): the cover when the window is used up (`run_out`
/// or 0% left) and the credits cover; `None` keeps the room notice.
#[must_use]
pub fn credits_carry_on(
    used: Option<f64>,
    run_out: bool,
    credits: Option<&crate::features::quota::CreditsDisplay>,
) -> Option<crate::features::quota::CreditsCover> {
    let used_up = run_out || used.is_some_and(crate::features::quota::used_up);
    if !used_up {
        return None;
    }
    credits.and_then(crate::features::quota::CreditsDisplay::cover)
}

/// The notice for an account whose window is used up while its credits
/// carry its work on. It informs (work isn't blocked): told once per used-up
/// window, under Earlier, never a need; no empty meter. "Continue on
/// another account…" stays its action.
#[must_use]
pub fn credits_notice(
    account_id: &str,
    label: &str,
    window: &str,
    resets_at_ms: Option<i64>,
    sessions: u32,
    cover: crate::features::quota::CreditsCover,
) -> NewNotice {
    let left = cover.left_words();
    let title = if left.is_some() {
        format!("{label} is using its credits")
    } else {
        format!("{label} is probably using its credits")
    };
    let detail = match &left {
        Some(left) => format!("Its {window} window is used up · {left}"),
        None => format!("Its {window} window is used up"),
    };
    NewNotice {
        key: format!(
            "credits:{account_id}:{}",
            resets_at_ms.map_or(0, |at| at / 60_000)
        ),
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
                credits: Some(left.unwrap_or_default()),
                ..NoticeFacts::default()
            },
        ),
        action: Some("hand-over".to_owned()),
        meter: None,
        ..NewNotice::default()
    }
}

/// One account's turn in the quota pass (0.3.2: split out when the
/// credits watch joined it).
struct AccountPass<'a> {
    storage: &'a Storage,
    emitted: &'a [crate::commands::AlertEventInfo],
    open: &'a [String],
    now: i64,
}

type Told = Vec<(Target, Kind, centre::NoticeInfo)>;

impl AccountPass<'_> {
    fn run(
        &self,
        account: &juggler_core::model::Account,
        on_credits: &mut std::collections::BTreeSet<String>,
        told: &mut Told,
        room_again: &mut Vec<(String, String, &'static str)>,
    ) {
        let id = account.id.as_str();
        let low_now = self
            .emitted
            .iter()
            .rfind(|event| event.account == id && LOW_KINDS.contains(&event.kind.as_str()));
        let is_open = self.open.iter().any(|key| *key == format!("quota:{id}"));
        let carrying = on_credits.contains(id);
        if low_now.is_none() && !is_open && !carrying {
            return;
        }
        let display = crate::features::quota::quota_display_inner(self.storage, account, self.now);
        let Some((window, used, resets_at)) = fullest(&display) else {
            return;
        };
        let sessions = live_sessions_on(self.storage, id);
        if carrying && !used.is_some_and(crate::features::quota::used_up) {
            // Its window reset: no longer on its credits.
            on_credits.remove(id);
            if low_now.is_none() && !is_open {
                return;
            }
        }
        if is_open && low_now.is_none() && used.is_some_and(|used| used < ROOM_AGAIN_USED_PCT) {
            if centre::end_need(self.storage, &format!("quota:{id}"), self.now).unwrap_or(false) {
                room_again.push((id.to_owned(), account.label.clone(), window));
            }
            return;
        }
        // Carrying on with credits that ran out: the window is used up,
        // so it has run out of room now.
        let credits_gone = on_credits.contains(id);
        let run_out = credits_gone || low_now.is_some_and(|event| event.kind == "exhausted");
        // 0.3.2: used up, but its credits carry the work on — said
        // once, under Earlier; any room need it had ends (work isn't
        // blocked, so nothing needs the user).
        if let Some(cover) = credits_carry_on(used, run_out, display.credits.as_ref()) {
            on_credits.insert(id.to_owned());
            if is_open {
                let _ = centre::end_need(self.storage, &format!("quota:{id}"), self.now);
            }
            let notice = credits_notice(id, &account.label, window, resets_at, sessions, cover);
            if let Ok(Some(info)) = centre::record(self.storage, &notice, self.now) {
                told.push((Target::Account(id.to_owned()), Kind::Quota, info));
            }
            return;
        }
        on_credits.remove(id);
        if credits_gone
            && low_now.is_none()
            && !is_open
            && !used.is_some_and(crate::features::quota::used_up)
        {
            return;
        }
        let notice = low_notice(
            id,
            &account.label,
            window,
            used,
            resets_at,
            sessions,
            run_out,
        );
        if let Ok(Some(info)) = centre::open_need(self.storage, &notice, self.now) {
            told.push((Target::Account(id.to_owned()), Kind::Quota, info));
        }
    }
}

/// Where the accounts carrying on with their credits are kept.
const ON_CREDITS_KEY: &str = "quota_on_credits";

/// The accounts carrying on with their credits (0.3.2).
fn on_credits_of(storage: &juggler_core::storage::Storage) -> std::collections::BTreeSet<String> {
    storage
        .app_settings()
        .get(ON_CREDITS_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_on_credits(
    storage: &juggler_core::storage::Storage,
    ids: &std::collections::BTreeSet<String>,
) {
    if let Ok(raw) = serde_json::to_string(ids) {
        let _ = storage.app_settings().set(ON_CREDITS_KEY, &raw);
    }
}

/// The work that moved away from an account and can come back to it now
/// (D-05's switch back, AJ-E207's suggestions): the newest such session,
/// by its row's name, and how many there are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchBack {
    /// The session the work went on in, on the other account (the one a
    /// switch back starts from).
    pub session_id: String,
    /// Its name, as its row says it.
    pub name: String,
    /// How many sessions can switch back (this one included).
    pub count: u32,
}

/// The notice for an account with room again. With work that moved away
/// from it (`back`), its one action is Switch back (the newest; the rest
/// keep their switch-back card on their own page); else it opens the
/// account.
#[must_use]
pub fn reset_notice(
    account_id: &str,
    label: &str,
    window: &str,
    at_ms: i64,
    back: Option<&SwitchBack>,
) -> NewNotice {
    let detail = match back {
        Some(back) if back.count > 1 => format!(
            "“{}” and {} more can switch back to it",
            back.name,
            back.count - 1
        ),
        Some(back) => format!("“{}” can switch back to it", back.name),
        None => "Sessions on it can carry on".to_owned(),
    };
    NewNotice {
        key: format!("reset:{account_id}:{}", at_ms / 60_000),
        kind: "reset".to_owned(),
        title: format!("{label} has room again — {} reset", reset_words(window)),
        detail,
        data: centre::data_of(
            &NoticeTarget {
                account_id: Some(account_id.to_owned()),
                session_id: back.map(|back| back.session_id.clone()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                window: Some(window.to_owned()),
                account: Some(label.to_owned()),
                sessions: back.map(|back| back.count),
                ..NoticeFacts::default()
            },
        ),
        action: Some(
            if back.is_some() {
                "switch-back"
            } else {
                "open"
            }
            .to_owned(),
        ),
        ..NewNotice::default()
    }
}

/// The work that can switch back to each account in `accounts`, from the
/// switch-back suggestions (whose account has room again): the newest
/// session first, by its last activity.
fn switch_backs(state: &Arc<AppState>, accounts: &[String]) -> Vec<(String, SwitchBack)> {
    let Ok(suggestions) = crate::commands::switch_back_suggestions_impl(state) else {
        return Vec::new();
    };
    let last_activity = |id: &str| -> i64 {
        state
            .storage
            .lock()
            .ok()
            .and_then(|storage| {
                storage
                    .sessions()
                    .get(&juggler_core::model::SessionId::new(id))
                    .ok()
            })
            .map_or(0, |record| {
                record.last_activity.unwrap_or(record.started_at).millis()
            })
    };
    let mut out = Vec::new();
    for account in accounts {
        let mut returns: Vec<(i64, &str)> = suggestions
            .iter()
            .filter(|suggestion| suggestion.original_account_id == *account)
            .map(|suggestion| {
                (
                    last_activity(&suggestion.session_id),
                    suggestion.session_id.as_str(),
                )
            })
            .collect();
        returns.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
        let mut seen = std::collections::HashSet::new();
        returns.retain(|(_, id)| seen.insert(*id));
        let Some((_, newest)) = returns.first() else {
            continue;
        };
        out.push((
            account.clone(),
            SwitchBack {
                session_id: (*newest).to_owned(),
                name: super::describe(state, newest).name,
                count: u32::try_from(returns.len()).unwrap_or(u32::MAX),
            },
        ));
    }
    out
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
    // Accounts with room again: (id, label, window), told once the work
    // that can switch back to each is known (it reads storage itself).
    let mut room_again: Vec<(String, String, &'static str)> = Vec::new();
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
        // 0.3.2: accounts carrying on with their credits, watched each
        // pass: the alert that said "used up" fires once, so credits that
        // run out mid-window would otherwise go unsaid.
        let mut on_credits = on_credits_of(&storage);
        let before = on_credits.clone();
        for account in &accounts {
            let pass = AccountPass {
                storage: &storage,
                emitted,
                open: &open,
                now,
            };
            pass.run(account, &mut on_credits, &mut told, &mut room_again);
        }
        if on_credits != before {
            save_on_credits(&storage, &on_credits);
        }
    }
    // 0.3: the work that moved away from an account that has room again
    // can switch back (D-05), from the notice itself.
    if !room_again.is_empty() {
        let ids: Vec<String> = room_again.iter().map(|(id, _, _)| id.clone()).collect();
        let backs = switch_backs(state, &ids);
        for (id, label, window) in &room_again {
            let back = backs
                .iter()
                .find(|(account, _)| account == id)
                .map(|(_, back)| back);
            let notice = reset_notice(id, label, window, now, back);
            if let Ok(storage) = state.storage.lock()
                && let Ok(Some(info)) = centre::record(&storage, &notice, now)
            {
                told.push((Target::Account(id.clone()), Kind::Reset, info));
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

    fn credits(
        has_credits: bool,
        unlimited: bool,
        balance: Option<&str>,
    ) -> crate::features::quota::CreditsDisplay {
        crate::features::quota::CreditsDisplay {
            has_credits,
            unlimited,
            balance: balance.map(str::to_owned),
            as_of_ms: 1_793_399_680_137,
            source: "A tool".to_owned(),
            stale: false,
        }
    }

    /// The maintainer's report (0.3.2): "has run out of room · 0% of its
    /// weekly window left" while the session kept working on its credits.
    #[test]
    fn a_used_up_window_on_credits_is_not_out_of_room() {
        use crate::features::quota::CreditsCover;
        // The maintainer's real stored value.
        let real = credits(true, false, Some("14150.1635025000"));
        let cover = credits_carry_on(Some(100.0), true, Some(&real)).unwrap();
        let notice = credits_notice(
            "acct-c",
            "Codex",
            "weekly",
            Some(1_793_500_000_000),
            1,
            cover,
        );
        assert_eq!(notice.title, "Codex is using its credits");
        assert_eq!(
            notice.detail,
            "Its weekly window is used up · 14,150 credits left"
        );
        assert_eq!(notice.meter, None, "no empty red meter");
        assert!(!notice.needs, "it informs: work isn't blocked");
        assert_eq!(notice.kind, "quota");
        assert_eq!(notice.action.as_deref(), Some("hand-over"));
        assert!(notice.key.starts_with("credits:acct-c:"));
        let data: serde_json::Value = serde_json::from_str(&notice.data).unwrap();
        assert_eq!(data["facts"]["credits"], "14,150 credits left");
        assert_eq!(data["facts"]["window"], "weekly");
        assert_eq!(data["target"]["account_id"], "acct-c");
        for words in [&notice.title, &notice.detail] {
            assert!(!words.contains("out of room") && !words.contains("acct-c"));
        }
        // 0% left counts as used up even without the exhausted alert.
        assert!(credits_carry_on(Some(100.0), false, Some(&real)).is_some());

        // Credits with no balance reported: probably.
        let probably = credits_carry_on(None, true, Some(&credits(true, false, None))).unwrap();
        assert_eq!(probably, CreditsCover::Probably);
        let notice = credits_notice("acct-c", "Codex", "5-hour", None, 0, probably);
        assert_eq!(notice.title, "Codex is probably using its credits");
        assert_eq!(notice.detail, "Its 5-hour window is used up");
        let data: serde_json::Value = serde_json::from_str(&notice.data).unwrap();
        assert_eq!(data["facts"]["credits"], "");

        // Unlimited.
        let unlimited =
            credits_carry_on(Some(100.0), true, Some(&credits(false, true, None))).unwrap();
        assert_eq!(
            credits_notice("a", "Codex", "weekly", None, 0, unlimited).detail,
            "Its weekly window is used up · unlimited credits"
        );
    }

    /// Credits that don't cover (none, or a balance of 0) keep today's
    /// words exactly; a window with room left is no credits notice.
    #[test]
    fn credits_that_dont_cover_keep_the_out_of_room_notice() {
        assert_eq!(
            credits_carry_on(Some(100.0), true, Some(&credits(true, false, Some("0")))),
            None,
            "a balance of 0"
        );
        assert_eq!(
            credits_carry_on(Some(100.0), true, Some(&credits(false, false, Some("12")))),
            None,
            "no credits"
        );
        assert_eq!(
            credits_carry_on(Some(100.0), true, None),
            None,
            "none reported"
        );
        assert_eq!(
            credits_carry_on(Some(93.0), false, Some(&credits(true, false, Some("500")))),
            None,
            "room left: the almost-out notice"
        );
        let out = low_notice("acct-1", "Codex", "weekly", Some(100.0), None, 0, true);
        assert_eq!(out.title, "Codex has run out of room");
        assert_eq!(out.detail, "0% of its weekly window left");
        assert_eq!(out.meter, Some(0));
    }

    #[test]
    fn a_reset_names_the_window_it_was() {
        assert_eq!(
            reset_notice("a", "Personal Max", "weekly", 0, None).title,
            "Personal Max has room again — its week reset"
        );
        let plain = reset_notice("a", "Work Max", "5-hour", 0, None);
        assert_eq!(
            plain.title,
            "Work Max has room again — its 5-hour window reset"
        );
        assert_eq!(plain.action.as_deref(), Some("open"));
        assert_eq!(plain.detail, "Sessions on it can carry on");
    }

    /// wf27: "Claude·Work quota reset — “fix-auth-tests” can switch back."
    #[test]
    fn a_reset_offers_the_work_that_moved_away_its_switch_back() {
        let one = SwitchBack {
            session_id: "s-away".to_owned(),
            name: "fix-auth-tests".to_owned(),
            count: 1,
        };
        let notice = reset_notice("acct-w", "Work Max", "5-hour", 0, Some(&one));
        assert_eq!(notice.action.as_deref(), Some("switch-back"));
        assert_eq!(notice.detail, "“fix-auth-tests” can switch back to it");
        let data: serde_json::Value = serde_json::from_str(&notice.data).unwrap();
        assert_eq!(data["target"]["session_id"], "s-away");
        assert_eq!(data["target"]["account_id"], "acct-w");
        assert_eq!(data["facts"]["sessions"], 1);
        let three = SwitchBack { count: 3, ..one };
        assert_eq!(
            reset_notice("acct-w", "Work Max", "5-hour", 0, Some(&three)).detail,
            "“fix-auth-tests” and 2 more can switch back to it"
        );
        for words in [&notice.title, &notice.detail] {
            assert!(
                !words.contains("s-away") && !words.contains("acct-w"),
                "an id reached the copy"
            );
        }
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
