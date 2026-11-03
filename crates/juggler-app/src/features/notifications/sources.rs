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

/// Who a quota notice is about (D-62): limits belong to the Account, so
/// the folders signed in to one share one notice, named by the Account
/// ([`crate::features::subscriptions::SubscriptionInfo::display_name`]).
/// A folder no Account lists (not signed in, a sign-in that can't be
/// read) stands for itself, under its own name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotaSubject {
    /// What the notice is keyed by: the stored Account's id, or the lone
    /// folder's.
    pub id: String,
    /// Its name: the Account's, or the folder's own.
    pub name: String,
    /// The folder its actions use (a hand-over needs a folder to continue
    /// from; a switch back, one to return to): see [`acting_folder`]. The
    /// notice's `target.account_id`.
    pub folder: String,
}

/// The notice for an account running out of room, from its quota now.
#[must_use]
pub fn low_notice(
    subject: &QuotaSubject,
    window: &str,
    used: Option<f64>,
    resets_at_ms: Option<i64>,
    sessions: u32,
    run_out: bool,
) -> NewNotice {
    let label = &subject.name;
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
        key: format!("quota:{}", subject.id),
        kind: "quota".to_owned(),
        title,
        detail,
        data: centre::data_of(
            &NoticeTarget {
                account_id: Some(subject.folder.clone()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                window: Some(window.to_owned()),
                resets_at_ms,
                sessions: Some(sessions),
                account: Some(label.clone()),
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
    subject: &QuotaSubject,
    window: &str,
    resets_at_ms: Option<i64>,
    sessions: u32,
    cover: crate::features::quota::CreditsCover,
) -> NewNotice {
    let label = &subject.name;
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
            "credits:{}:{}",
            subject.id,
            resets_at_ms.map_or(0, |at| at / 60_000)
        ),
        kind: "quota".to_owned(),
        title,
        detail,
        data: centre::data_of(
            &NoticeTarget {
                account_id: Some(subject.folder.clone()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                window: Some(window.to_owned()),
                resets_at_ms,
                sessions: Some(sessions),
                account: Some(label.clone()),
                credits: Some(left.unwrap_or_default()),
                ..NoticeFacts::default()
            },
        ),
        action: Some("hand-over".to_owned()),
        meter: None,
        ..NewNotice::default()
    }
}

/// One subject's folders (D-62): an Account's, in the order AI Juggler
/// lists them, or a lone folder.
struct Group<'a> {
    id: String,
    name: String,
    folders: Vec<&'a juggler_core::model::Account>,
}

/// The folders, grouped by the Account each is signed in to (`accounts`,
/// by folder); a folder in none is a group of its own.
fn groups<'a>(
    folders: &'a [juggler_core::model::Account],
    accounts: &std::collections::BTreeMap<String, crate::features::subscriptions::FolderAccount>,
) -> Vec<Group<'a>> {
    let mut out: Vec<Group<'a>> = Vec::new();
    for folder in folders {
        let Some(account) = accounts.get(folder.id.as_str()) else {
            out.push(Group {
                id: folder.id.to_string(),
                name: folder.label.clone(),
                folders: vec![folder],
            });
            continue;
        };
        match out.iter_mut().find(|group| group.id == account.id) {
            Some(group) => group.folders.push(folder),
            None => out.push(Group {
                id: account.id.clone(),
                name: account.name.clone(),
                folders: vec![folder],
            }),
        }
    }
    out
}

/// True when `display` carries a real reading (a window with its
/// percentage), as the UI's runway reads one.
fn has_reading(display: &crate::features::quota::QuotaDisplayInfo) -> bool {
    display.unavailable_reason.is_none()
        && [&display.five_hour, &display.weekly]
            .into_iter()
            .flatten()
            .any(|window| {
                window.percentage.is_some_and(f64::is_finite)
                    && !matches!(
                        window.state.as_str(),
                        "unknown" | "learning" | "unmeasured" | "reset"
                    )
            })
}

/// Which of an Account's folders' readings is its own now (one runway for
/// all of them, as the UI's `accountDisplay` picks): the newest real
/// reading; with none, the first folder's.
#[must_use]
pub fn newest_reading(displays: &[crate::features::quota::QuotaDisplayInfo]) -> usize {
    let when = |display: &crate::features::quota::QuotaDisplayInfo| {
        [&display.five_hour, &display.weekly]
            .into_iter()
            .flatten()
            .filter_map(|window| window.measured_at_ms)
            .max()
            .unwrap_or(0)
    };
    displays
        .iter()
        .enumerate()
        .filter(|(_, display)| has_reading(display))
        .max_by_key(|(at, display)| (when(display), std::cmp::Reverse(*at)))
        .map_or(0, |(at, _)| at)
}

/// The folder an Account's notice acts on (`live`: sessions open on each
/// folder; `reading`: the folder its reading came from): the one with the
/// most sessions open, since a hand-over continues one of them; the
/// reading's on a tie or with none open.
#[must_use]
pub fn acting_folder(live: &[u32], reading: usize) -> usize {
    let most = live.iter().copied().max().unwrap_or(0);
    if most == 0 || live.get(reading) == Some(&most) {
        return reading;
    }
    live.iter()
        .position(|open| *open == most)
        .unwrap_or(reading)
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

/// A subject with room again, and its window, told once the work that can
/// switch back to it is known.
struct RoomAgain {
    subject: QuotaSubject,
    folders: Vec<String>,
    window: &'static str,
}

/// The keys subject `id`'s room need may be open under: its own; and,
/// before 0.4 (D-62) each folder had a need of its own, each of its
/// folders' (an Account's folders' needs are its own now).
fn need_keys(id: &str, folders: &[&str]) -> Vec<String> {
    std::iter::once(id)
        .chain(folders.iter().copied().filter(|folder| *folder != id))
        .map(|key| format!("quota:{key}"))
        .collect()
}

/// Whether subject `id` carries on with its credits: kept under its id
/// (before 0.4, under a folder's: moved to the Account's now).
fn carrying_on(
    on_credits: &mut std::collections::BTreeSet<String>,
    id: &str,
    folders: &[&str],
) -> bool {
    let mut carrying = on_credits.contains(id);
    for folder in folders.iter().filter(|folder| **folder != id) {
        carrying |= on_credits.remove(*folder);
    }
    if carrying {
        on_credits.insert(id.to_owned());
    }
    carrying
}

impl AccountPass<'_> {
    fn run(
        &self,
        group: &Group<'_>,
        on_credits: &mut std::collections::BTreeSet<String>,
        told: &mut Told,
        room_again: &mut Vec<RoomAgain>,
    ) {
        let id = group.id.as_str();
        let folders: Vec<&str> = group.folders.iter().map(|f| f.id.as_str()).collect();
        let own_key = format!("quota:{id}");
        let keys = need_keys(id, &folders);
        let low_now = self.emitted.iter().rfind(|event| {
            folders.contains(&event.account.as_str()) && LOW_KINDS.contains(&event.kind.as_str())
        });
        let is_open = keys.iter().any(|key| self.open.contains(key));
        let carrying = carrying_on(on_credits, id, &folders);
        if low_now.is_none() && !is_open && !carrying {
            return;
        }
        let mut displays: Vec<_> = group
            .folders
            .iter()
            .map(|folder| {
                crate::features::quota::quota_display_inner(self.storage, folder, self.now)
            })
            .collect();
        let reading = newest_reading(&displays);
        let display = displays.swap_remove(reading);
        let Some((window, used, resets_at)) = fullest(&display) else {
            return;
        };
        let live: Vec<u32> = folders
            .iter()
            .map(|folder| live_sessions_on(self.storage, folder))
            .collect();
        let sessions = live
            .iter()
            .fold(0_u32, |sum, open| sum.saturating_add(*open));
        let subject = QuotaSubject {
            id: id.to_owned(),
            name: group.name.clone(),
            folder: folders[acting_folder(&live, reading)].to_owned(),
        };
        if carrying && !used.is_some_and(crate::features::quota::used_up) {
            // Its window reset: no longer on its credits.
            on_credits.remove(id);
            if low_now.is_none() && !is_open {
                return;
            }
        }
        if is_open && low_now.is_none() && used.is_some_and(|used| used < ROOM_AGAIN_USED_PCT) {
            let mut ended = false;
            for key in &keys {
                ended |= centre::end_need(self.storage, key, self.now).unwrap_or(false);
            }
            if ended {
                room_again.push(RoomAgain {
                    subject,
                    folders: folders.iter().map(|folder| (*folder).to_owned()).collect(),
                    window,
                });
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
                for key in &keys {
                    let _ = centre::end_need(self.storage, key, self.now);
                }
            }
            let notice = credits_notice(&subject, window, resets_at, sessions, cover);
            if let Ok(Some(info)) = centre::record(self.storage, &notice, self.now) {
                told.push((Target::Account(subject.folder.clone()), Kind::Quota, info));
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
        // One need per Account: a folder's own (from before 0.4) gives way.
        for key in keys.iter().filter(|key| **key != own_key) {
            if self.open.contains(key) {
                let _ = centre::end_need(self.storage, key, self.now);
            }
        }
        let notice = low_notice(&subject, window, used, resets_at, sessions, run_out);
        if let Ok(Some(info)) = centre::open_need(self.storage, &notice, self.now) {
            told.push((Target::Account(subject.folder.clone()), Kind::Quota, info));
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
    subject: &QuotaSubject,
    window: &str,
    at_ms: i64,
    back: Option<&SwitchBack>,
) -> NewNotice {
    let label = &subject.name;
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
        key: format!("reset:{}:{}", subject.id, at_ms / 60_000),
        kind: "reset".to_owned(),
        title: format!("{label} has room again — {} reset", reset_words(window)),
        detail,
        data: centre::data_of(
            &NoticeTarget {
                account_id: Some(subject.folder.clone()),
                session_id: back.map(|back| back.session_id.clone()),
                ..NoticeTarget::default()
            },
            &NoticeFacts {
                window: Some(window.to_owned()),
                account: Some(label.clone()),
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

/// The work that can switch back to each subject in `subjects` (its id and
/// folders), from the switch-back suggestions (whose folder has room
/// again): the newest session first, by its last activity.
fn switch_backs(
    state: &Arc<AppState>,
    subjects: &[(String, Vec<String>)],
) -> Vec<(String, SwitchBack)> {
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
    for (subject, folders) in subjects {
        let mut returns: Vec<(i64, &str)> = suggestions
            .iter()
            .filter(|suggestion| folders.contains(&suggestion.original_account_id))
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
            subject.clone(),
            SwitchBack {
                session_id: (*newest).to_owned(),
                name: super::describe(state, newest).name,
                count: u32::try_from(returns.len()).unwrap_or(u32::MAX),
            },
        ));
    }
    out
}

/// A plan renewing soon (the digest's line), as a notice: `account_id`
/// is the folder it was read for (its Open goes there), `label` what it
/// is called (D-62: the renewal is the Account's, so its name; a folder
/// no Account lists, its own).
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
///
/// D-62: the alerts are per folder, the notices per Account (its folders
/// share its limits): one notice for all its folders, named by it, from
/// its newest reading ([`newest_reading`]), acting on the folder with its
/// open sessions ([`acting_folder`]). A renewal is the Account's too,
/// so its line names it. The Accounts are read only when there is
/// something to tell.
pub fn quota_pass(state: &Arc<AppState>, emitted: &[crate::commands::AlertEventInfo]) {
    let now = crate::commands::now_millis();
    let mut told = Vec::new();
    let renewals = emitted.iter().any(|event| event.kind == "digest");
    let accounts = if renewals || quota_work(state, emitted) {
        crate::features::subscriptions::folder_accounts(state)
    } else {
        std::collections::BTreeMap::new()
    };
    // Subjects with room again, told once the work that can switch back to
    // each is known (it reads storage itself).
    let mut room_again: Vec<RoomAgain> = Vec::new();
    {
        let Ok(storage) = state.storage.lock() else {
            return;
        };
        let folders = storage.accounts().list().unwrap_or_default();
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
            let own = folders
                .iter()
                .find(|account| account.id.as_str() == event.account)
                .map_or("An account", |account| account.label.as_str());
            let label = crate::features::subscriptions::folder_name(&accounts, &event.account, own);
            let _ = centre::record(
                &storage,
                &renewal_notice(&event.account, &label, &event.text),
                now,
            );
        }
        // 0.3.2: accounts carrying on with their credits, watched each
        // pass: the alert that said "used up" fires once, so credits that
        // run out mid-window would otherwise go unsaid.
        let mut on_credits = on_credits_of(&storage);
        let before = on_credits.clone();
        let pass = AccountPass {
            storage: &storage,
            emitted,
            open: &open,
            now,
        };
        for group in groups(&folders, &accounts) {
            pass.run(&group, &mut on_credits, &mut told, &mut room_again);
        }
        if on_credits != before {
            save_on_credits(&storage, &on_credits);
        }
    }
    // 0.3: the work that moved away from an account that has room again
    // can switch back (D-05), from the notice itself.
    if !room_again.is_empty() {
        let subjects: Vec<(String, Vec<String>)> = room_again
            .iter()
            .map(|room| (room.subject.id.clone(), room.folders.clone()))
            .collect();
        let backs = switch_backs(state, &subjects);
        for room in &room_again {
            let back = backs
                .iter()
                .find(|(subject, _)| *subject == room.subject.id)
                .map(|(_, back)| back);
            let notice = reset_notice(&room.subject, room.window, now, back);
            if let Ok(storage) = state.storage.lock()
                && let Ok(Some(info)) = centre::record(&storage, &notice, now)
            {
                told.push((
                    Target::Account(room.subject.folder.clone()),
                    Kind::Reset,
                    info,
                ));
            }
        }
    }
    for (target, kind, info) in told {
        super::model_event(state, target, kind, &info);
    }
    super::changed(state);
}

/// Whether the quota pass has anything to look at: an alert that an
/// account runs low, a room need open, or an account on its credits.
fn quota_work(state: &Arc<AppState>, emitted: &[crate::commands::AlertEventInfo]) -> bool {
    if emitted
        .iter()
        .any(|event| LOW_KINDS.contains(&event.kind.as_str()))
    {
        return true;
    }
    let Ok(storage) = state.storage.lock() else {
        return false;
    };
    !on_credits_of(&storage).is_empty()
        || storage
            .notices()
            .open_needs()
            .unwrap_or_default()
            .iter()
            .any(|row| row.kind == "quota")
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

/// 0.3.3: an app or profile kept alive stopped and was started again
/// (`attempt` of the tries keep alive allows): told once per start, under
/// Earlier; its action opens Settings › Desktop apps, where keep alive is
/// turned off.
pub fn app_started_again(state: &Arc<AppState>, place_id: &str, label: &str, attempt: usize) {
    let now = crate::commands::now_millis();
    let attempt = u32::try_from(attempt).unwrap_or(u32::MAX);
    let tries = u32::try_from(crate::features::sessions::restart::TRIES).unwrap_or(u32::MAX);
    keep_alive_notice(
        state,
        &NewNotice {
            key: format!("app-started:{place_id}:{now}"),
            kind: "app-started".to_owned(),
            title: format!("{label} stopped and was started again"),
            detail: format!("Keep alive · try {attempt} of {tries} in 10 minutes"),
            data: centre::data_of(
                &NoticeTarget {
                    page: Some("desktop_apps".to_owned()),
                    ..NoticeTarget::default()
                },
                &NoticeFacts {
                    attempt: Some(attempt),
                    tries: Some(tries),
                    ..NoticeFacts::default()
                },
            ),
            action: Some("keep-alive".to_owned()),
            ..NewNotice::default()
        },
        now,
    );
}

/// 0.3.3: keep alive stopped starting an app or profile, and why (it
/// stopped too often, it couldn't start, the user stopped it here).
pub fn keep_alive_paused(state: &Arc<AppState>, place_id: &str, label: &str, why: &str) {
    let now = crate::commands::now_millis();
    keep_alive_notice(
        state,
        &NewNotice {
            key: format!("keep-alive-paused:{place_id}:{now}"),
            kind: "keep-alive-paused".to_owned(),
            title: format!("Keep alive paused for {label}"),
            detail: format!(
                "Not started again: {why}. Open it to start it and keep it alive again."
            ),
            data: centre::data_of(
                &NoticeTarget {
                    page: Some("desktop_apps".to_owned()),
                    ..NoticeTarget::default()
                },
                &NoticeFacts::default(),
            ),
            action: Some("keep-alive".to_owned()),
            ..NewNotice::default()
        },
        now,
    );
}

fn keep_alive_notice(state: &Arc<AppState>, notice: &NewNotice, now: i64) {
    let recorded = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| centre::record(&storage, notice, now).ok().flatten());
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

    /// A folder no Account lists: it stands for itself.
    fn lone(id: &str, name: &str) -> QuotaSubject {
        QuotaSubject {
            id: id.to_owned(),
            name: name.to_owned(),
            folder: id.to_owned(),
        }
    }

    /// D-62: an Account's notice is keyed and named by the Account, and
    /// acts on one of its folders.
    #[test]
    fn an_accounts_notice_is_named_and_keyed_by_it_and_acts_on_a_folder() {
        let subject = QuotaSubject {
            id: "acct:anthropic:1".to_owned(),
            name: "Claude Max 20x".to_owned(),
            folder: "claude-work".to_owned(),
        };
        let low = low_notice(&subject, "5-hour", Some(93.0), Some(5), 3, false);
        assert_eq!(low.title, "Claude Max 20x is almost out of room");
        assert_eq!(low.key, "quota:acct:anthropic:1");
        let data: serde_json::Value = serde_json::from_str(&low.data).unwrap();
        assert_eq!(data["target"]["account_id"], "claude-work");
        assert_eq!(data["facts"]["account"], "Claude Max 20x");
        assert_eq!(data["facts"]["sessions"], 3);
        let reset = reset_notice(&subject, "weekly", 0, None);
        assert_eq!(
            reset.title,
            "Claude Max 20x has room again — its week reset"
        );
        assert!(reset.key.starts_with("reset:acct:anthropic:1:"));
        let credits = credits_notice(
            &subject,
            "weekly",
            None,
            1,
            crate::features::quota::CreditsCover::Probably,
        );
        assert_eq!(
            credits.title,
            "Claude Max 20x is probably using its credits"
        );
        assert!(credits.key.starts_with("credits:acct:anthropic:1:"));
    }

    #[test]
    fn the_acting_folder_is_the_one_with_open_sessions_else_the_readings() {
        assert_eq!(acting_folder(&[0, 0], 1), 1, "none open: the reading's");
        assert_eq!(acting_folder(&[0, 2], 0), 1, "the one with sessions open");
        assert_eq!(acting_folder(&[2, 2, 1], 1), 1, "a tie: the reading's");
        assert_eq!(acting_folder(&[1, 3, 3], 0), 1, "the first with the most");
        assert_eq!(acting_folder(&[], 0), 0);
    }

    #[test]
    fn an_account_running_low_is_said_in_plain_words() {
        let notice = low_notice(
            &lone("acct-1", "Work Max"),
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
        let out = low_notice(
            &lone("acct-1", "Work Max"),
            "weekly",
            Some(100.0),
            None,
            0,
            true,
        );
        assert_eq!(out.title, "Work Max has run out of room");
        assert_eq!(out.detail, "0% of its weekly window left");
        let unknown = low_notice(&lone("acct-1", "Work Max"), "5-hour", None, None, 0, false);
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
            &lone("acct-c", "Codex"),
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
        let notice = credits_notice(&lone("acct-c", "Codex"), "5-hour", None, 0, probably);
        assert_eq!(notice.title, "Codex is probably using its credits");
        assert_eq!(notice.detail, "Its 5-hour window is used up");
        let data: serde_json::Value = serde_json::from_str(&notice.data).unwrap();
        assert_eq!(data["facts"]["credits"], "");

        // Unlimited.
        let unlimited =
            credits_carry_on(Some(100.0), true, Some(&credits(false, true, None))).unwrap();
        assert_eq!(
            credits_notice(&lone("a", "Codex"), "weekly", None, 0, unlimited).detail,
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
        let out = low_notice(
            &lone("acct-1", "Codex"),
            "weekly",
            Some(100.0),
            None,
            0,
            true,
        );
        assert_eq!(out.title, "Codex has run out of room");
        assert_eq!(out.detail, "0% of its weekly window left");
        assert_eq!(out.meter, Some(0));
    }

    #[test]
    fn a_reset_names_the_window_it_was() {
        assert_eq!(
            reset_notice(&lone("a", "Personal Max"), "weekly", 0, None).title,
            "Personal Max has room again — its week reset"
        );
        let plain = reset_notice(&lone("a", "Work Max"), "5-hour", 0, None);
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
        let notice = reset_notice(&lone("acct-w", "Work Max"), "5-hour", 0, Some(&one));
        assert_eq!(notice.action.as_deref(), Some("switch-back"));
        assert_eq!(notice.detail, "“fix-auth-tests” can switch back to it");
        let data: serde_json::Value = serde_json::from_str(&notice.data).unwrap();
        assert_eq!(data["target"]["session_id"], "s-away");
        assert_eq!(data["target"]["account_id"], "acct-w");
        assert_eq!(data["facts"]["sessions"], 1);
        let three = SwitchBack { count: 3, ..one };
        assert_eq!(
            reset_notice(&lone("acct-w", "Work Max"), "5-hour", 0, Some(&three)).detail,
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
