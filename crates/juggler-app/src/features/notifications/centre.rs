//! AJ-E2501 — the one notification model. Every source writes here: the
//! Needs you list (kept in step by the window, [`sync_needs`]), and AI
//! Juggler's own sources ([`super::sources`]: an account almost out of
//! room, a reset, a finish, a crash and its restart, an update). The
//! centre, the banners, the bell, the app badge, the menu-bar item and the
//! system notifications are views of it.
//!
//! Rules (the approved design, 2026-10-27, and D-49):
//! - **One count.** The number on the bell, the badge, the menu-bar item,
//!   the rail and Home is the open needs that aren't snoozed: the Needs
//!   you count. A snoozed need leaves the count until its snooze ends (it
//!   comes back by itself, on the backend's tick); a muted one is still
//!   counted and only stops interrupting.
//! - **Told once.** What happened is written once per key; a need is open
//!   once per key ([`juggler_core::storage::NoticesRepo`]).
//! - **Nothing acts by itself.** A notice says what happened and offers
//!   one action; only the user's click runs it (the UI does, never here).
//! - **Kept 30 days and paged** ([`page`]).
//!
//! Pure over [`Storage`] and the time: the Tauri glue is in
//! [`super::views`] and [`super::commands`].

use juggler_core::storage::{NOTICE_KEEP_MS, NewNotice, Notice, NoticeMute, Storage, Written};

use crate::error::{AppError, AppResult};
use crate::paging::{Page, PageRequest, SortKey, page_of_fetched};

/// The Needs you list (kept in step by the window).
pub const ORIGIN_NEEDS: &str = "needs";
/// AI Juggler's own sources.
pub const ORIGIN_APP: &str = "app";

/// What may be written from the Needs you list, and how each kind is
/// named in Settings (Mute this kind). The words are the app's own.
pub const NEED_KINDS: &[(&str, &str)] = &[
    ("asks", "Sessions that ask you"),
    ("stuck", "Sessions that may be stuck"),
    ("limit", "Usage limits reached"),
    ("failed", "Sessions that stopped with an error"),
    ("review", "Finished work to review"),
    ("signin", "Sign-ins"),
    ("room", "Room on this computer"),
    ("heavy", "Sessions working hard"),
    ("leftovers", "Processes left running"),
    ("cap", "Monthly caps"),
    ("message", "Messages from agents"),
    ("task", "Agent tasks"),
];

/// AI Juggler's own kinds, and their names.
pub const APP_KINDS: &[(&str, &str)] = &[
    ("quota", "Accounts running out of room"),
    ("reset", "Resets"),
    ("finished", "Finished sessions"),
    ("restarted", "Crashes and restarts"),
    ("update", "Updates"),
    ("renewal", "Plan renewals"),
];

/// The kinds that come up as a banner while they need the user (at most
/// [`BANNERS_SHOWN`], one per account).
const BANNER_KINDS: &[&str] = &["quota"];
/// Banners stacked at most.
pub const BANNERS_SHOWN: usize = 3;

/// How a kind is named (Mute this kind; Settings).
#[must_use]
pub fn kind_label(kind: &str) -> Option<&'static str> {
    NEED_KINDS
        .iter()
        .chain(APP_KINDS)
        .find(|(id, _)| *id == kind)
        .map(|(_, label)| *label)
}

/// What a click opens. Every field is optional; the UI opens the first
/// it can (a session's page, a page, an account, a project).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NoticeTarget {
    /// A session (AI Juggler's), by id.
    #[serde(default)]
    pub session_id: Option<String>,
    /// A session in another app (its conversation id).
    #[serde(default)]
    pub conversation_id: Option<String>,
    /// That conversation's account.
    #[serde(default)]
    pub account_id: Option<String>,
    /// A project, by id.
    #[serde(default)]
    pub project_id: Option<String>,
    /// A page: `this_mac` | `processes` | `settings` | `updates` | `accounts`.
    #[serde(default)]
    pub page: Option<String>,
    /// A task (a pool's), by id.
    #[serde(default)]
    pub task_id: Option<String>,
    /// A pool, by id.
    #[serde(default)]
    pub pool_id: Option<String>,
}

/// The facts a line is drawn from, so the window can say them in the
/// user's own time and words.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NoticeFacts {
    /// The usage window: `5-hour` | `weekly`.
    #[serde(default)]
    pub window: Option<String>,
    /// When it resets, epoch ms.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub resets_at_ms: Option<i64>,
    /// Live sessions on the account.
    #[serde(default)]
    pub sessions: Option<u32>,
    /// A version (an update).
    #[serde(default)]
    pub version: Option<String>,
    /// Its channel (`stable` | `beta` | `nightly`).
    #[serde(default)]
    pub channel: Option<String>,
    /// The try a restart was (1 of [`NoticeFacts::tries`]).
    #[serde(default)]
    pub attempt: Option<u32>,
    /// Tries in all.
    #[serde(default)]
    pub tries: Option<u32>,
    /// The account's name.
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct NoticeData {
    #[serde(default)]
    target: NoticeTarget,
    #[serde(default)]
    facts: NoticeFacts,
}

/// A notice's `data` column.
#[must_use]
pub fn data_of(target: &NoticeTarget, facts: &NoticeFacts) -> String {
    serde_json::to_string(&NoticeData {
        target: target.clone(),
        facts: facts.clone(),
    })
    .unwrap_or_else(|_| "{}".to_owned())
}

/// One notice as the window shows it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[allow(clippy::struct_excessive_bools)] // the user's marks: each its own fact
pub struct NoticeInfo {
    /// Row id.
    #[ts(type = "number")]
    pub id: i64,
    /// What it is about (`asks`, `quota`, `finished`, …).
    pub kind: String,
    /// It needs the user (else it happened: Earlier).
    pub needs: bool,
    /// The line, in plain words.
    pub title: String,
    /// The second line.
    pub detail: String,
    /// Its project, when it has one (Mute this project).
    pub project_id: Option<String>,
    /// That project's name.
    pub project: Option<String>,
    /// What a click opens.
    pub target: NoticeTarget,
    /// The facts its line is drawn from.
    pub facts: NoticeFacts,
    /// The one primary action's id (the window names and runs it).
    pub action: Option<String>,
    /// A meter, in percent (room left).
    #[ts(type = "number | null")]
    pub meter: Option<i64>,
    /// First written, epoch ms.
    #[ts(type = "number")]
    pub created_ms: i64,
    /// Not read yet, and allowed to ask for attention (not muted or snoozed).
    pub unread: bool,
    /// What a snooze holds it by (`s:<session>`, `c:<account>/<conversation>`,
    /// `signin:<account>`, `quota:<account>`…).
    pub subject: String,
    /// Snoozed now: out of the count until the snooze ends.
    pub snoozed: bool,
    /// When the snooze ends (epoch ms); none while snoozed until woken.
    #[ts(type = "number | null")]
    pub snoozed_until_ms: Option<i64>,
    /// Its project or kind is muted.
    pub muted: bool,
    /// Its banner was closed.
    pub dismissed: bool,
}

/// The day so far, in counts (the centre's digest line).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NoticeDigestInfo {
    /// Sessions that finished today.
    pub finished: u32,
    /// Crashes restarted today.
    pub restarted: u32,
    /// Accounts that got their room back today.
    pub resets: u32,
    /// Updates ready today.
    pub updates: u32,
    /// The next reset of an account that is almost out of room: its name.
    pub next_reset_account: Option<String>,
    /// …and when, epoch ms.
    #[ts(type = "number | null")]
    pub next_reset_ms: Option<i64>,
}

/// Where the model stands: what every view shows at a glance.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NoticeSummaryInfo {
    /// The one count: what needs the user (= Needs you).
    pub count: u32,
    /// Notices with an unread dot.
    pub unread: u32,
    /// Needs snoozed now (out of the count until they wake).
    pub snoozed: u32,
    /// Needs whose project or kind is muted (still counted).
    pub muted: u32,
    /// Inside the quiet hours now: nothing interrupts.
    pub quiet: bool,
    /// When the quiet hours end (`07:00`), while inside them.
    pub quiet_until: Option<String>,
    /// Banners due now (at most [`BANNERS_SHOWN`]).
    pub banners: u32,
    /// The day so far.
    pub digest: NoticeDigestInfo,
    /// Changes whenever any list does: the window reads its lists again.
    #[ts(type = "number")]
    pub version: i64,
}

/// One item of the Needs you list, as the backend works it out
/// ([`super::needs`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NeedInput {
    /// The item's own key (stable while it needs the user).
    pub key: String,
    /// One of [`NEED_KINDS`].
    pub kind: String,
    /// The line, in plain words.
    pub title: String,
    /// The second line.
    pub detail: String,
    /// Its project.
    pub project_id: Option<String>,
    /// That project's name.
    pub project: Option<String>,
    /// What a click opens.
    pub target: NoticeTarget,
    /// The primary action's id.
    pub action: Option<String>,
}

/// A snooze on now, as the rail reads it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NeedSnoozeInfo {
    /// What is snoozed (`s:<session>`, …).
    pub subject: String,
    /// Until then (epoch ms); none: until woken.
    #[ts(type = "number | null")]
    pub until_ms: Option<i64>,
}

/// A mute, as Settings lists it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NoticeMuteInfo {
    /// `project` | `kind`.
    pub scope: String,
    /// The project's id, or the kind.
    pub value: String,
    /// Its name.
    pub label: String,
}

/// Needs the window may send at once (far beyond any real list).
const MAX_NEEDS: usize = 500;
const MAX_KEY: usize = 200;
const MAX_TITLE: usize = 300;
const MAX_DETAIL: usize = 600;

fn bounded(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// A refusal in plain words (never an id or a code).
fn invalid(message: &str) -> AppError {
    AppError::Refused(message.to_owned())
}

/// What the user allows now: the hour, for the quiet hours and "today".
#[derive(Clone, Copy, Debug, Default)]
pub struct Clock {
    /// Now, epoch ms.
    pub now: i64,
    /// Local time minus UTC, in hours, when known.
    pub utc_offset_hours: Option<i64>,
}

impl Clock {
    /// The local hour now, when the offset is known.
    #[must_use]
    pub fn local_hour(self) -> Option<u32> {
        let offset = self.utc_offset_hours?;
        let hour = (self.now.div_euclid(3_600_000) + offset).rem_euclid(24);
        u32::try_from(hour).ok()
    }

    /// Local midnight today, epoch ms (UTC midnight while the offset is
    /// unknown).
    #[must_use]
    pub fn today_start(self) -> i64 {
        let offset_ms = self.utc_offset_hours.unwrap_or(0) * 3_600_000;
        let local = self.now + offset_ms;
        local - local.rem_euclid(86_400_000) - offset_ms
    }
}

/// The user's marks that shape a row: the mutes, and the snoozes on now.
struct Marks {
    projects: Vec<String>,
    kinds: Vec<String>,
    snoozes: std::collections::HashMap<String, Option<i64>>,
}

impl Marks {
    fn read(storage: &Storage, now: i64) -> Self {
        let all = storage.notices().mutes().unwrap_or_default();
        let of = |scope: &str| {
            all.iter()
                .filter(|mute| mute.scope == scope)
                .map(|mute| mute.value.clone())
                .collect()
        };
        Self {
            projects: of("project"),
            kinds: of("kind"),
            snoozes: storage
                .notices()
                .snoozes(now)
                .unwrap_or_default()
                .into_iter()
                .collect(),
        }
    }

    fn covers(&self, kind: &str, project_id: Option<&str>) -> bool {
        self.kinds.iter().any(|muted| muted == kind)
            || project_id.is_some_and(|project| self.projects.iter().any(|muted| muted == project))
    }
}

/// What a snooze holds a notice by: a need's key without its origin.
#[must_use]
pub fn subject_of(key: &str) -> &str {
    key.strip_prefix(ORIGIN_NEEDS)
        .and_then(|rest| rest.strip_prefix(':'))
        .unwrap_or(key)
}

fn info(row: Notice, marks: &Marks) -> NoticeInfo {
    let data: NoticeData = serde_json::from_str(&row.data).unwrap_or_default();
    let muted = marks.covers(&row.kind, row.project_id.as_deref());
    let subject = subject_of(&row.key).to_owned();
    let snooze = marks.snoozes.get(&subject).copied();
    let snoozed = row.needs && snooze.is_some();
    NoticeInfo {
        id: row.id,
        unread: row.read_at.is_none() && !muted && !snoozed,
        subject,
        snoozed,
        snoozed_until_ms: snooze.flatten(),
        kind: row.kind,
        needs: row.needs && row.resolved_at.is_none(),
        title: row.title,
        detail: row.detail,
        project_id: row.project_id,
        project: row.project,
        target: data.target,
        facts: data.facts,
        action: row.action,
        meter: row.meter,
        created_ms: row.created_at,
        muted,
        dismissed: row.dismissed_at.is_some(),
    }
}

/// The Needs you list now: writes what is new, brings up to date what
/// changed, ends what is gone. Returns the needs new since the last sync
/// (the ones that may interrupt).
///
/// # Errors
///
/// [`AppError::Refused`] for a list this app's window didn't make;
/// the typed storage error.
pub fn sync_needs(storage: &Storage, needs: &[NeedInput], now: i64) -> AppResult<Vec<NoticeInfo>> {
    if needs.len() > MAX_NEEDS {
        return Err(invalid("Too many items to keep in step."));
    }
    let repo = storage.notices();
    let mut keep = Vec::with_capacity(needs.len());
    let mut fresh = Vec::new();
    for need in needs {
        if need.key.is_empty() || need.key.len() > MAX_KEY {
            return Err(invalid("An item without a usable key."));
        }
        if !NEED_KINDS.iter().any(|(kind, _)| *kind == need.kind) {
            return Err(invalid("An item of a kind this app doesn't know."));
        }
        let key = format!("{ORIGIN_NEEDS}:{}", need.key);
        let written = repo.open(
            &NewNotice {
                key: key.clone(),
                kind: need.kind.clone(),
                origin: ORIGIN_NEEDS.to_owned(),
                needs: true,
                title: bounded(&need.title, MAX_TITLE),
                detail: bounded(&need.detail, MAX_DETAIL),
                project_id: need.project_id.clone(),
                project: need.project.clone(),
                data: data_of(&need.target, &NoticeFacts::default()),
                action: need.action.clone(),
                meter: None,
            },
            now,
        )?;
        if let Written::New(id) = written {
            fresh.push(id);
        }
        keep.push(key);
    }
    repo.resolve_missing(ORIGIN_NEEDS, &keep, now)?;
    repo.purge(now - NOTICE_KEEP_MS)?;
    let marks = Marks::read(storage, now);
    Ok(fresh
        .into_iter()
        .filter_map(|id| repo.get(id).ok().flatten())
        .map(|row| info(row, &marks))
        .collect())
}

/// Opens (or brings up to date) one of AI Juggler's own needs. Returns
/// the notice when it is new.
///
/// # Errors
///
/// The typed storage error.
pub fn open_need(storage: &Storage, notice: &NewNotice, now: i64) -> AppResult<Option<NoticeInfo>> {
    let notice = NewNotice {
        origin: ORIGIN_APP.to_owned(),
        needs: true,
        ..notice.clone()
    };
    let written = storage.notices().open(&notice, now)?;
    Ok(match written {
        Written::New(id) => storage
            .notices()
            .get(id)?
            .map(|row| info(row, &Marks::read(storage, now))),
        Written::Changed(_) | Written::Same(_) => None,
    })
}

/// Ends one of AI Juggler's own needs; `true` when it was open.
///
/// # Errors
///
/// The typed storage error.
pub fn end_need(storage: &Storage, key: &str, now: i64) -> AppResult<bool> {
    Ok(storage.notices().resolve(key, now)?)
}

/// Writes something that happened, once. Returns it when it is new.
///
/// # Errors
///
/// The typed storage error.
pub fn record(storage: &Storage, notice: &NewNotice, now: i64) -> AppResult<Option<NoticeInfo>> {
    let notice = NewNotice {
        origin: ORIGIN_APP.to_owned(),
        ..notice.clone()
    };
    let repo = storage.notices();
    let Some(id) = repo.record(&notice, now)? else {
        return Ok(None);
    };
    repo.purge(now - NOTICE_KEEP_MS)?;
    Ok(repo
        .get(id)?
        .map(|row| info(row, &Marks::read(storage, now))))
}

/// Whether a notice of `kind` about `project_id` may interrupt now: its
/// kind and project aren't muted. (Snoozes are per notice.)
#[must_use]
pub fn may_interrupt(storage: &Storage, kind: &str, project_id: Option<&str>) -> bool {
    !Marks::read(storage, 0).covers(kind, project_id)
}

/// Whether the open need about this session is snoozed now.
#[must_use]
pub fn session_snoozed(storage: &Storage, session_id: &str, now: i64) -> bool {
    let subject = format!("s:{session_id}");
    storage
        .notices()
        .snoozes(now)
        .is_ok_and(|on| on.iter().any(|(snoozed, _)| *snoozed == subject))
}

fn banner_due(row: &NoticeInfo) -> bool {
    row.needs
        && BANNER_KINDS.contains(&row.kind.as_str())
        && !row.dismissed
        && !row.muted
        && !row.snoozed
}

fn open_infos(storage: &Storage, now: i64) -> AppResult<Vec<NoticeInfo>> {
    let marks = Marks::read(storage, now);
    Ok(storage
        .notices()
        .open_needs()?
        .into_iter()
        .map(|row| info(row, &marks))
        .collect())
}

/// Where the model stands now.
///
/// # Errors
///
/// The typed storage error.
pub fn summary(storage: &Storage, clock: Clock) -> AppResult<NoticeSummaryInfo> {
    let now = clock.now;
    let open = open_infos(storage, now)?;
    let count_of = |test: &dyn Fn(&NoticeInfo) -> bool| {
        u32::try_from(open.iter().filter(|row| test(row)).count()).unwrap_or(u32::MAX)
    };
    let repo = storage.notices();
    let marks = Marks::read(storage, now);
    let earlier_unread = repo
        .earlier(None, 50)?
        .into_iter()
        .map(|row| info(row, &marks))
        .filter(|row| row.unread)
        .count();
    let quiet_hours = crate::settings_policy::quiet_hours(storage);
    let quiet = clock
        .local_hour()
        .is_some_and(|hour| quiet_hours.contains(hour));
    let mut digest = NoticeDigestInfo::default();
    for (kind, n) in repo.earlier_kinds_since(clock.today_start())? {
        let n = u32::try_from(n).unwrap_or(u32::MAX);
        match kind.as_str() {
            "finished" => digest.finished = n,
            "restarted" => digest.restarted = n,
            "reset" => digest.resets = n,
            "update" => digest.updates = n,
            _ => {}
        }
    }
    if let Some(next) = open
        .iter()
        .filter(|row| row.kind == "quota")
        .filter_map(|row| {
            row.facts
                .resets_at_ms
                .map(|at| (at, row.facts.account.clone()))
        })
        .filter(|(at, _)| *at > now)
        .min_by_key(|(at, _)| *at)
    {
        digest.next_reset_ms = Some(next.0);
        digest.next_reset_account = next.1;
    }
    let banners = open
        .iter()
        .filter(|row| banner_due(row))
        .count()
        .min(BANNERS_SHOWN);
    Ok(NoticeSummaryInfo {
        count: count_of(&|row| !row.snoozed),
        unread: count_of(&|row| row.unread)
            .saturating_add(u32::try_from(earlier_unread).unwrap_or(u32::MAX)),
        snoozed: count_of(&|row| row.snoozed),
        muted: count_of(&|row| row.muted),
        quiet,
        quiet_until: quiet.then(|| format!("{:02}:00", quiet_hours.end)),
        banners: u32::try_from(banners).unwrap_or(0),
        digest,
        version: version_of(storage, now),
    })
}

/// A number that changes whenever a notice, a mute or a snooze does (a
/// snooze ending included).
fn version_of(storage: &Storage, now: i64) -> i64 {
    let repo = storage.notices();
    let latest = repo
        .open_needs()
        .unwrap_or_default()
        .iter()
        .map(|row| row.updated_at.max(row.read_at.unwrap_or(0)))
        .max()
        .unwrap_or(0);
    let held = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        repo.snoozes(now).unwrap_or_default().hash(&mut hasher);
        i64::from_ne_bytes(hasher.finish().to_ne_bytes())
    };
    let earlier = repo
        .earlier(None, 1)
        .ok()
        .and_then(|rows| rows.first().map(|row| row.id))
        .unwrap_or(0);
    let mutes = repo.mutes().map_or(0, |all| all.len());
    latest
        .wrapping_mul(31)
        .wrapping_add(earlier)
        .wrapping_mul(31)
        .wrapping_add(i64::try_from(mutes).unwrap_or(0))
        .wrapping_add(i64::try_from(repo.open_count().unwrap_or(0)).unwrap_or(0))
        .wrapping_mul(31)
        .wrapping_add(held)
}

/// A section of the centre: `needs` (Needs action, oldest first),
/// `earlier` (what happened, newest first, 30 days) or `banners` (due
/// now, at most three).
///
/// # Errors
///
/// [`AppError::Refused`] for an unknown section, [`AppError::Validation`]
/// for a cursor this app didn't make; the typed
/// storage error.
pub fn page(
    storage: &Storage,
    section: &str,
    request: &PageRequest,
    now: i64,
) -> AppResult<Page<NoticeInfo>> {
    match section {
        "needs" => {
            let rows = open_infos(storage, now)?;
            crate::paging::page(
                rows,
                |row| SortKey::new([row.created_ms, row.id], row.id.to_string()),
                request,
            )
        }
        "banners" => {
            let mut rows: Vec<NoticeInfo> = open_infos(storage, now)?
                .into_iter()
                .filter(banner_due)
                .collect();
            rows.sort_by_key(|row| std::cmp::Reverse(row.created_ms));
            rows.truncate(BANNERS_SHOWN);
            crate::paging::page(
                rows,
                |row| SortKey::new([-row.created_ms, -row.id], row.id.to_string()),
                request,
            )
        }
        "earlier" => {
            let after = request.start_after()?.map(|key| {
                let at = key.ranks.first().map_or(i64::MAX, |rank| -rank);
                let id = key.ranks.get(1).map_or(i64::MAX, |rank| -rank);
                (at, id)
            });
            let limit = request.limit();
            let repo = storage.notices();
            let marks = Marks::read(storage, now);
            let fetched = repo
                .earlier(after, limit)?
                .into_iter()
                .map(|row| {
                    let key = SortKey::new([-row.created_at, -row.id], row.id.to_string());
                    (key, info(row, &marks))
                })
                .collect();
            Ok(page_of_fetched(fetched, limit, Some(repo.earlier_count()?)))
        }
        _ => Err(invalid("No such list.")),
    }
}

/// Marks one notice read (`Some(id)`) or all of them (`None`).
///
/// # Errors
///
/// The typed storage error.
pub fn read(storage: &Storage, id: Option<i64>, now: i64) -> AppResult<()> {
    match id {
        Some(id) => storage.notices().mark_read(id, now).map(|_| ())?,
        None => storage.notices().mark_all_read(now).map(|_| ())?,
    }
    Ok(())
}

/// How a subject is snoozed: until a time, until woken, or woken now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snooze {
    /// Until then (epoch ms, ahead).
    Until(i64),
    /// Until the user wakes it (the rail's Mute this session).
    UntilWoken,
    /// Back now.
    Wake,
}

/// Snoozes a subject (D-49): its needs leave the Needs you count and stop
/// interrupting until the snooze ends — by itself at its time, or when
/// woken. Its open needs are marked read.
///
/// # Errors
///
/// [`AppError::Refused`] for a time already past or a subject this app
/// didn't make; the typed storage error.
pub fn snooze(storage: &Storage, subject: &str, how: Snooze, now: i64) -> AppResult<()> {
    if subject.is_empty() || subject.len() > MAX_KEY {
        return Err(invalid("Nothing to snooze."));
    }
    let repo = storage.notices();
    match how {
        Snooze::Until(until) if until <= now => {
            return Err(invalid("Snooze until a time still ahead."));
        }
        Snooze::Until(until) => repo.snooze(subject, Some(until), now)?,
        Snooze::UntilWoken => repo.snooze(subject, None, now)?,
        Snooze::Wake => {
            repo.wake(subject)?;
            return Ok(());
        }
    }
    for row in repo.open_needs()? {
        if subject_of(&row.key) == subject {
            repo.mark_read(row.id, now)?;
        }
    }
    Ok(())
}

/// The snoozes on now, a page at a time (the rail reads them).
///
/// # Errors
///
/// [`AppError::Validation`] for a bad cursor; the typed storage error.
pub fn snoozes(
    storage: &Storage,
    request: &PageRequest,
    now: i64,
) -> AppResult<Page<NeedSnoozeInfo>> {
    let rows: Vec<NeedSnoozeInfo> = storage
        .notices()
        .snoozes(now)?
        .into_iter()
        .map(|(subject, until)| NeedSnoozeInfo {
            subject,
            until_ms: until,
        })
        .collect();
    crate::paging::page(rows, |row| SortKey::new([0], row.subject.clone()), request)
}

/// Closes a banner; the notice stays in the centre.
///
/// # Errors
///
/// The typed storage error.
pub fn dismiss(storage: &Storage, id: i64, now: i64) -> AppResult<()> {
    storage.notices().dismiss(id, now)?;
    storage.notices().mark_read(id, now)?;
    Ok(())
}

/// Mutes a project (`project`, its id, its name) or a kind (`kind`, one
/// of [`NEED_KINDS`] or [`APP_KINDS`]; its name is the app's own).
///
/// # Errors
///
/// [`AppError::Refused`] for an unknown scope or kind; the typed storage
/// error.
pub fn mute(storage: &Storage, scope: &str, value: &str, label: &str, now: i64) -> AppResult<()> {
    let label = match scope {
        "kind" => kind_label(value)
            .ok_or_else(|| invalid("A kind this app doesn't know."))?
            .to_owned(),
        "project" if !value.is_empty() && value.len() <= MAX_KEY => bounded(label, MAX_TITLE),
        _ => return Err(invalid("Mute a project or a kind.")),
    };
    storage.notices().mute(scope, value, &label, now)?;
    Ok(())
}

/// Lets a project or a kind interrupt again.
///
/// # Errors
///
/// The typed storage error.
pub fn unmute(storage: &Storage, scope: &str, value: &str) -> AppResult<()> {
    storage.notices().unmute(scope, value)?;
    Ok(())
}

/// The mutes, a page at a time.
///
/// # Errors
///
/// [`AppError::Validation`] for a bad cursor; the typed storage error.
pub fn mutes(storage: &Storage, request: &PageRequest) -> AppResult<Page<NoticeMuteInfo>> {
    let rows: Vec<(i64, NoticeMuteInfo)> = storage
        .notices()
        .mutes()?
        .into_iter()
        .map(
            |NoticeMute {
                 scope,
                 value,
                 label,
                 created_at,
             }| {
                (
                    created_at,
                    NoticeMuteInfo {
                        scope,
                        value,
                        label,
                    },
                )
            },
        )
        .collect();
    let page = crate::paging::page(
        rows,
        |(at, mute)| SortKey::new([*at], format!("{}:{}", mute.scope, mute.value)),
        request,
    )?;
    Ok(page.map(|(_, mute)| mute))
}

/// The open needs that count, for the views outside the window (the
/// menu-bar item): oldest first, at most `limit`; a snoozed one waits
/// until it wakes (D-49).
#[must_use]
pub fn open_needs(storage: &Storage, now: i64, limit: usize) -> Vec<NoticeInfo> {
    let mut rows = open_infos(storage, now).unwrap_or_default();
    rows.retain(|row| !row.snoozed);
    rows.truncate(limit);
    rows
}

#[cfg(test)]
mod tests;
