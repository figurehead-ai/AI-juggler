//! AJ-E1510: system notifications (ADR-0019 §3) — what needs the user
//! reaches them outside the window, when they asked for it.
//!
//! The state changes come in where they're made (a heuristic's waiting
//! or stuck, a process's finish or usage limit, This Mac's room); the
//! [`engine`] decides what is news; Settings › Notifications says which
//! kinds may interrupt, and its quiet hours and accounts left out apply
//! here as they do to the digest. A click brings AI Juggler forward on
//! that session (or This Mac).
//!
//! AJ-E3712, AJ-E3817 (ADR-0036 §1.7, §2.5): what is due goes through
//! one [`batch`] in front of the sink: the 60-second burst, several items
//! as one notification, quiet hours released as one. The watches' alerts
//! and the agent digest come the same way ([`hand_over`]), under the same
//! rules: no second notification channel.

pub mod batch;
pub mod centre;
pub mod commands;
#[cfg(test)]
mod consolidation_tests;
pub mod engine;
pub mod needs;
pub mod prefs;
pub mod sources;
pub mod tray;
pub mod views;
pub mod watch_alerts;

use crate::events::{AppEvent, EmitEvent as _};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use engine::{Due, Engine, Gate, Kind, Target};

use crate::state::AppState;

/// Asks the UI to show a session or This Mac (a notification's click).
pub const EVENT_OPEN: &str = AppEvent::AppOpen.name();

/// What a click asks the UI to open.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct OpenRequest {
    /// The session to open, when it's one.
    pub session_id: Option<String>,
    /// `this_mac` for the This Mac page, `notifications` for the centre;
    /// from the Floor (AJ-E3510) also `app`, `conversation`, `subagents`,
    /// `usage` and `pool`, each with its [`Self::id`]; from its View menu
    /// (AJ-E3519) `settings` with the pane (`the-floor`).
    pub page: Option<String>,
    /// AJ-E1515: the paired Mac the session runs on (`None`: this one).
    #[serde(default)]
    pub machine_id: Option<String>,
    /// What `page` opens: an App's id, another app's session
    /// (`<account>/<id>`), a session's subagents (the rail's key) or an
    /// Account's id.
    #[serde(default)]
    pub id: Option<String>,
}

/// One notification as the system shows it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Outgoing {
    /// The id a click comes back with.
    pub id: i32,
    /// What it's about (a session's name, or This Mac).
    pub title: String,
    /// Its state, where and on which account.
    pub body: String,
}

type Sink = Box<dyn Fn(&Outgoing) + Send + Sync>;
/// AJ-E2501: what follows a change in the model (the badge, the menu-bar
/// item, the window).
type OnChange = Box<dyn Fn() + Send + Sync>;

/// AJ-E3712: away after AI Juggler's window has been out of the front
/// this long (D-118's two hours).
pub const AWAY_MS: i64 = 2 * 3_600_000;

/// No offset known yet (the UI tells it within its first tick).
const UNKNOWN_OFFSET: i32 = i32::MIN;
/// Clicks remembered (the oldest go first).
const CLICKS_KEPT: usize = 200;

/// The notifier, held by [`AppState`].
pub struct Notifier {
    engine: Mutex<Engine>,
    sink: OnceLock<Sink>,
    clicks: Mutex<HashMap<i32, batch::Opens>>,
    /// AJ-E3712: consolidation in front of the sink.
    batch: Mutex<batch::Batcher>,
    /// AJ-E3817, AJ-E3712: the watches' alerts and the digest's item,
    /// waiting for [`show_due`] to put them through the same rules.
    incoming: Mutex<Vec<batch::Item>>,
    /// When the open burst's end is already awaited.
    flush_at: Mutex<Option<i64>>,
    next_id: AtomicI32,
    /// AI Juggler's main window has focus.
    in_front: AtomicBool,
    /// When it last left the front (0: in front, or never left).
    left_front_ms: AtomicI64,
    /// Local time minus UTC, in whole hours (the webview knows the
    /// local hour; the backend keeps no time zone).
    utc_offset_hours: AtomicI32,
    /// This Mac's room in words, as last seen (title, reason).
    mac_words: Mutex<Option<(String, String)>>,
    /// AJ-E2405: how many processes each session left running, as told.
    left: Mutex<HashMap<String, u32>>,
    /// AJ-E2501: the model's own words for what it tells (an account
    /// almost out of room, a crash restarted, a reset).
    told: Mutex<HashMap<(Target, Kind), (String, String)>>,
    /// AJ-E2501: the views of the model, once the app has them.
    on_change: OnceLock<OnChange>,
    /// AJ-E2501: what is open now, as the backend last worked it out (the
    /// menu-bar item lists it).
    open_now: Mutex<views::OpenNowInput>,
    /// D-49: This Mac as last sampled (the tick's Needs you reads it).
    this_mac: Mutex<Option<crate::machine_health::ThisMacInfo>>,
    /// D-49: the accounts' sign-ins as last checked, and when.
    sign_ins: Mutex<Option<(i64, Vec<crate::features::account::dto::AccountInfo>)>>,
    /// D-49: what Needs you was last time (a hash: the model is written
    /// only when it changes).
    needs_signature: Mutex<Option<u64>>,
    /// D-49, AJ-E805: AI Juggler's session rows as Needs you last read
    /// them from storage ([`needs::KeptRows`]).
    kept_rows: Mutex<Option<needs::KeptRows>>,
    /// How many times Needs you read those rows from storage.
    rows_read: std::sync::atomic::AtomicU64,
    /// NY1 (0.4.6 rc3a): the views are told one change at a time
    /// ([`views::refresh_on`]), in the order each was worked out.
    telling: Mutex<()>,
}

impl Default for Notifier {
    fn default() -> Self {
        Self {
            engine: Mutex::new(Engine::default()),
            sink: OnceLock::new(),
            clicks: Mutex::new(HashMap::new()),
            batch: Mutex::new(batch::Batcher::default()),
            incoming: Mutex::new(Vec::new()),
            flush_at: Mutex::new(None),
            next_id: AtomicI32::new(1),
            in_front: AtomicBool::new(false),
            left_front_ms: AtomicI64::new(0),
            utc_offset_hours: AtomicI32::new(UNKNOWN_OFFSET),
            mac_words: Mutex::new(None),
            left: Mutex::new(HashMap::new()),
            told: Mutex::new(HashMap::new()),
            on_change: OnceLock::new(),
            open_now: Mutex::new(views::OpenNowInput::default()),
            this_mac: Mutex::new(None),
            sign_ins: Mutex::new(None),
            needs_signature: Mutex::new(None),
            kept_rows: Mutex::new(None),
            rows_read: std::sync::atomic::AtomicU64::new(0),
            telling: Mutex::new(()),
        }
    }
}

impl Notifier {
    /// Where notifications go: the system in the app, a list in tests.
    /// Set once; until then nothing is shown.
    pub fn set_sink(&self, sink: Sink) {
        let _ = self.sink.set(sink);
    }

    /// AJ-E2501: what follows a change in the model. Set once; until then
    /// (and in tests) a change only changes the model.
    pub fn set_on_change(&self, on_change: OnChange) {
        let _ = self.on_change.set(on_change);
    }

    /// The UTC offset learned from the window, when known.
    #[must_use]
    pub fn utc_offset(&self) -> Option<i64> {
        let offset = self.utc_offset_hours.load(Ordering::Relaxed);
        (offset != UNKNOWN_OFFSET).then_some(i64::from(offset))
    }

    /// AJ-E2501: the model's clock now (the time, and the local hour's offset).
    #[must_use]
    pub fn clock(&self) -> centre::Clock {
        centre::Clock {
            now: crate::commands::now_millis(),
            utc_offset_hours: self.utc_offset(),
        }
    }

    /// AJ-E2501: what is open now, as the window last said.
    #[must_use]
    pub fn open_now(&self) -> views::OpenNowInput {
        self.open_now
            .lock()
            .map(|open| open.clone())
            .unwrap_or_default()
    }

    /// AJ-E2501: what is open now, as the backend worked it out.
    pub fn set_open_now(&self, open: views::OpenNowInput) {
        if let Ok(mut now) = self.open_now.lock() {
            *now = open;
        }
    }

    /// D-49: This Mac as just sampled.
    pub fn set_this_mac(&self, info: crate::machine_health::ThisMacInfo) {
        if let Ok(mut this_mac) = self.this_mac.lock() {
            *this_mac = Some(info);
        }
    }

    /// D-49: This Mac as last sampled.
    #[must_use]
    pub fn this_mac(&self) -> Option<crate::machine_health::ThisMacInfo> {
        self.this_mac
            .lock()
            .ok()
            .and_then(|this_mac| this_mac.clone())
    }

    /// D-49: the accounts with their sign-ins, checked again (`check`)
    /// only when the last check is older than `fresh_ms`.
    pub fn sign_ins(
        &self,
        now: i64,
        fresh_ms: i64,
        check: impl FnOnce() -> Vec<crate::features::account::dto::AccountInfo>,
    ) -> Vec<crate::features::account::dto::AccountInfo> {
        if let Ok(cached) = self.sign_ins.lock()
            && let Some((at, accounts)) = cached.as_ref()
            && now - at < fresh_ms
        {
            return accounts.clone();
        }
        let accounts = check();
        if let Ok(mut cached) = self.sign_ins.lock() {
            *cached = Some((now, accounts.clone()));
        }
        accounts
    }

    /// AJ-E1612: forgets the accounts' sign-ins as last checked, so the
    /// next [`Self::sign_ins`] checks them again (a sign-in just ended).
    pub fn forget_sign_ins(&self) {
        if let Ok(mut cached) = self.sign_ins.lock() {
            *cached = None;
        }
    }

    /// D-49: the session rows Needs you kept, when still `fresh`; else
    /// `read` reads them, and they are kept.
    pub fn session_rows(
        &self,
        fresh: impl FnOnce(&needs::KeptRows) -> bool,
        read: impl FnOnce() -> Option<needs::KeptRows>,
    ) -> Option<Vec<crate::features::sessions::dto::InstanceInfo>> {
        if let Ok(kept) = self.kept_rows.lock()
            && let Some(kept) = kept.as_ref().filter(|kept| fresh(kept))
        {
            return Some(kept.rows.clone());
        }
        let read = read()?;
        self.rows_read.fetch_add(1, Ordering::Relaxed);
        let rows = read.rows.clone();
        if let Ok(mut kept) = self.kept_rows.lock() {
            *kept = Some(read);
        }
        Some(rows)
    }

    /// How many times Needs you read AI Juggler's session rows from
    /// storage (a cached tick reads none).
    #[must_use]
    pub fn session_rows_read(&self) -> u64 {
        self.rows_read.load(Ordering::Relaxed)
    }

    /// NY1 (0.4.6 rc3a): held while the views are told of a change, so
    /// two told at once (a command's, the tick's) reach them in the order
    /// they were worked out. A poisoned one still tells.
    pub(crate) fn telling(&self) -> std::sync::MutexGuard<'_, ()> {
        self.telling
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// D-49: records what Needs you is now; `true` when it is what it was.
    pub fn needs_signature(&self, signature: u64) -> bool {
        self.needs_signature
            .lock()
            .is_ok_and(|mut last| last.replace(signature) == Some(signature))
    }

    /// A burst's length (ADR-0036 §1.7's 60 seconds). Tests of the rules
    /// before the batch set 0: each item goes out as it falls due.
    pub fn set_batch_ms(&self, window_ms: i64) {
        if let Ok(mut batch) = self.batch.lock() {
            *batch = batch::Batcher::with_window(window_ms);
        }
    }

    /// AI Juggler's main window gained or lost focus.
    pub fn set_in_front(&self, in_front: bool) {
        let was = self.in_front.swap(in_front, Ordering::Relaxed);
        if was && !in_front {
            self.left_front_ms
                .store(crate::commands::now_millis(), Ordering::Relaxed);
        }
        if in_front {
            self.left_front_ms.store(0, Ordering::Relaxed);
        }
    }

    /// AJ-E3712 (ADR-0036 §1.6): the user is away: AI Juggler's window
    /// left the front [`AWAY_MS`] ago or more and hasn't come back. Its
    /// window's focus is the presence the app knows; a window never in
    /// front since start isn't counted away.
    #[must_use]
    pub fn away(&self, now_ms: i64) -> bool {
        let left = self.left_front_ms.load(Ordering::Relaxed);
        !self.in_front.load(Ordering::Relaxed) && left > 0 && now_ms - left >= AWAY_MS
    }

    /// The UI's local hour, as it ticks: the offset from UTC follows.
    pub fn local_hour_seen(&self, local_hour: Option<u32>, now_ms: i64) {
        let Some(local) = local_hour.filter(|hour| *hour < 24) else {
            return;
        };
        let offset = (i64::from(local) - utc_hour(now_ms)).rem_euclid(24);
        self.utc_offset_hours
            .store(i32::try_from(offset).unwrap_or(0), Ordering::Relaxed);
    }

    /// The local hour now, when known.
    fn local_hour(&self, now_ms: i64) -> Option<u32> {
        let offset = self.utc_offset_hours.load(Ordering::Relaxed);
        (offset != UNKNOWN_OFFSET)
            .then(|| u32::try_from((utc_hour(now_ms) + i64::from(offset)).rem_euclid(24)).ok())
            .flatten()
    }

    /// Where a click goes.
    #[must_use]
    pub fn target_of(&self, id: i32) -> Option<batch::Opens> {
        self.clicks.lock().ok()?.get(&id).cloned()
    }

    fn remember(&self, target: batch::Opens) -> i32 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut clicks) = self.clicks.lock() {
            if clicks.len() >= CLICKS_KEPT
                && let Some(oldest) = clicks.keys().min().copied()
            {
                clicks.remove(&oldest);
            }
            clicks.insert(id, target);
        }
        id
    }
}

fn utc_hour(now_ms: i64) -> i64 {
    now_ms.div_euclid(3_600_000).rem_euclid(24)
}

/// A session's status now (`SessionStatus::as_str`); shows what is due.
pub fn session_status(state: &Arc<AppState>, session_id: &str, status: &str) {
    // AJ-E3814: the observation bus hears every status a session enters.
    crate::features::observe::publish_for_session(
        state,
        session_id,
        crate::features::observe::from_status(status),
        crate::commands::now_millis(),
    );
    // AJ-E2501: a finish is told once in the centre, whatever the
    // system notifications allow.
    if status == "finished" {
        sources::finished(state, session_id);
    }
    observe(
        state,
        Target::Session(session_id.to_owned()),
        Kind::of_status(status),
    );
}

/// This Mac's room now (its summary: `plenty` | `tight` | `pressure`).
pub fn this_mac_room(state: &Arc<AppState>, room: &crate::machine_health::RoomSummaryInfo) {
    let tight = matches!(room.level.as_str(), "tight" | "pressure");
    if tight && let Ok(mut words) = state.notifier.mac_words.lock() {
        *words = Some((
            format!("This Mac: {}", room.words.to_lowercase()),
            room.reasons
                .first()
                .map_or_else(String::new, |reason| reason.words.clone()),
        ));
    }
    observe(state, Target::ThisMac, tight.then_some(Kind::MacTight));
}

/// AJ-E2405: a session ended and left `count` processes running (told
/// once; a click opens the session's page).
pub fn left_behind(state: &Arc<AppState>, session_id: &str, count: u32) {
    crate::features::observe::publish_for_session(
        state,
        session_id,
        vec![crate::features::observe::Observed::ProcessesLeft { count }],
        crate::commands::now_millis(),
    );
    if let Ok(mut left) = state.notifier.left.lock() {
        left.insert(session_id.to_owned(), count);
    }
    let target = Target::Session(session_id.to_owned());
    observe(state, target.clone(), Some(Kind::LeftBehind));
    // Told (or held for quiet hours): the next ending can be told again.
    let now = crate::commands::now_millis();
    if let Ok(mut engine) = state.notifier.engine.lock()
        && !engine.pending()
    {
        engine.observe(target, None, now);
    }
}

fn observe(state: &Arc<AppState>, target: Target, kind: Option<Kind>) {
    let now = crate::commands::now_millis();
    if let Ok(mut engine) = state.notifier.engine.lock() {
        engine.observe(target, kind, now);
    }
    show_due(state);
}

/// AJ-E2501: the model changed — the views follow (the badge, the
/// menu-bar item, the window).
pub fn changed(state: &Arc<AppState>) {
    if let Some(on_change) = state.notifier.on_change.get() {
        on_change();
    }
}

/// AJ-E2501: something the model wrote may become a system notification
/// (its kind on in Settings, its project and kind not muted, and the
/// rules every notification keeps: told once, quiet hours, never while
/// AI Juggler is in front). One that happened (a crash, a reset) can be
/// told again the next time; one that needs the user while it holds (an
/// account almost out of room) once while it holds.
pub fn model_event(state: &Arc<AppState>, target: Target, kind: Kind, info: &centre::NoticeInfo) {
    // AJ-E3814: what happened reaches the watches whatever the user's
    // notification settings (a watch is its own ask).
    publish_model_event(state, &target, kind);
    // R7: a need whose subject is snoozed (an account's room need snoozed
    // until woken, opening again) waits in the centre; it doesn't
    // interrupt.
    if info.snoozed {
        return;
    }
    let muted = state.storage.lock().is_ok_and(|storage| {
        !centre::may_interrupt_in(
            &storage,
            &info.kind,
            info.project_id.as_deref(),
            info.target.pool_id.as_deref(),
        )
    });
    if muted {
        return;
    }
    if let Ok(mut told) = state.notifier.told.lock() {
        told.insert(
            (target.clone(), kind),
            (info.title.clone(), info.detail.clone()),
        );
    }
    observe(state, target.clone(), Some(kind));
    // A quota notice that only informs (an account carrying on with its
    // credits, 0.3.2) happened: the next one can be told again.
    if kind != Kind::Quota || !info.needs {
        let now = crate::commands::now_millis();
        if let Ok(mut engine) = state.notifier.engine.lock()
            && !engine.pending()
        {
            engine.observe(target, None, now);
        }
    }
}

/// AJ-E3814: a crash's restart and an account's reset, for the
/// observation bus. An account's share left waits for AJ-E3704's runway
/// crossings.
fn publish_model_event(state: &Arc<AppState>, target: &Target, kind: Kind) {
    use crate::features::observe::{self, Observed};
    let now = crate::commands::now_millis();
    match (kind, target) {
        (Kind::Crashed, Target::Session(session)) => {
            observe::publish_for_session(state, session, vec![Observed::Restarted], now);
        }
        (Kind::Reset, Target::Account(account)) => {
            observe::publish_for_account(state, account, Observed::RunwayReset, now);
        }
        _ => {}
    }
}

/// Shows what is due now: called on each change, and on the heuristic
/// tick so what quiet hours held comes once they end. A burst still
/// gathering is awaited by one short-lived timer, so its one
/// notification goes out when its minute is up.
pub fn show_due(state: &Arc<AppState>) {
    if let Some(at) = show_due_at(state, crate::commands::now_millis()) {
        await_flush(state, at);
    }
}

/// [`show_due`] at `now` (a test's clock): sends what is due through the
/// batch; returns when the open burst's minute is up, if one is open.
pub(crate) fn show_due_at(state: &Arc<AppState>, now: i64) -> Option<i64> {
    let notifier = &state.notifier;
    let sink = notifier.sink.get()?;
    let incoming = notifier
        .incoming
        .lock()
        .map(|mut incoming| std::mem::take(&mut *incoming))
        .unwrap_or_default();
    let engine_pending = notifier.engine.lock().is_ok_and(|engine| engine.pending());
    let waiting = notifier.batch.lock().is_ok_and(|batch| batch.waiting(now));
    if !engine_pending && incoming.is_empty() && !waiting {
        return None;
    }
    let in_front = notifier.in_front.load(Ordering::Relaxed);
    let (due, incoming, quiet, carried) = {
        let storage = state.storage.lock().ok()?;
        let kinds = prefs::read(&storage);
        let quiet = notifier
            .local_hour(now)
            .is_none_or(|hour| crate::settings_policy::quiet_hours(&storage).contains(hour));
        let left_out = crate::settings_policy::muted_accounts(&storage);
        // AJ-E2501: a muted kind doesn't interrupt either.
        let allowed = |kind: Kind| {
            kinds.allows(kind) && centre::may_interrupt(&storage, model_kind(kind), None)
        };
        // AJ-E2501: nor does a snoozed need or a muted project (the
        // model's marks; the count still holds them).
        let muted = |target: &Target| match target {
            Target::Session(id) => {
                account_of(&storage, id).is_some_and(|account| left_out.contains(&account))
                    || centre::session_snoozed(&storage, id, now)
                    || !centre::may_interrupt(&storage, "", project_of(&storage, id).as_deref())
            }
            Target::ThisMac | Target::Centre | Target::Feed => false,
            Target::Account(id) => left_out.contains(id),
        };
        let gate = Gate {
            allowed: &allowed,
            quiet,
            in_front,
            muted: &muted,
            wait_after_ms: crate::settings_policy::wait_after_ms(&storage),
        };
        let due = if engine_pending {
            notifier
                .engine
                .lock()
                .map(|mut engine| engine.due(&gate, now))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        // The watches' alerts and the digest keep every rule a need does:
        // their kind on, not muted, never while AI Juggler is in front.
        let incoming: Vec<batch::Item> = incoming
            .into_iter()
            .filter(|item| !in_front && allowed(item.kind) && !muted(&item.target))
            .collect();
        // ADR-0036 §1.7: with the digest's notification on, the finishes
        // are told in it rather than one each.
        let carried = kinds.digest
            && kinds.finished
            && crate::features::digest::carries_finishes(state, &storage);
        (due, incoming, quiet, carried)
    };
    let mut items: Vec<batch::Item> = due
        .into_iter()
        .filter(|due| !(carried && due.kind == Kind::Finished))
        .map(|due| item_of(state, due, now))
        .collect();
    items.extend(incoming);
    let (groups, next) = {
        let mut batch = notifier.batch.lock().ok()?;
        if in_front {
            batch.drop_waiting();
        }
        let groups = batch.offer(items, quiet, now);
        (groups, batch.next_flush())
    };
    for group in groups {
        let words = group.words();
        let id = notifier.remember(words.opens);
        sink(&Outgoing {
            id,
            title: words.title,
            body: words.body,
        });
    }
    next
}

/// Awaits the open burst's end once: a short-lived timer that sends its
/// one notification then (the heuristic tick would, up to 15 s later).
fn await_flush(state: &Arc<AppState>, at: i64) {
    {
        let Ok(mut armed) = state.notifier.flush_at.lock() else {
            return;
        };
        if *armed == Some(at) {
            return;
        }
        *armed = Some(at);
    }
    let weak = Arc::downgrade(state);
    let _ = std::thread::Builder::new()
        .name("notify-batch".into())
        .spawn(move || {
            let wait = at - crate::commands::now_millis();
            if wait > 0 {
                std::thread::sleep(std::time::Duration::from_millis(
                    u64::try_from(wait).unwrap_or(0) + 50,
                ));
            }
            if let Some(state) = weak.upgrade() {
                if let Ok(mut armed) = state.notifier.flush_at.lock()
                    && *armed == Some(at)
                {
                    *armed = None;
                }
                show_due(&state);
            }
        });
}

/// A due state as a batch item: its words, its level (D-117) and what it
/// is called among others ("web" "asks you").
fn item_of(state: &Arc<AppState>, due: Due, now: i64) -> batch::Item {
    let (title, body) = words(state, &due);
    let (phrase, many) = match (&due.target, due.kind) {
        (Target::Session(_), Kind::Waiting) => ("waits for you", "wait for you"),
        (Target::Session(_), Kind::Stuck) => ("looks stuck", "look stuck"),
        (Target::Session(_), Kind::Limit) => ("hit its usage limit", "hit their usage limits"),
        (Target::Session(_), Kind::Finished) => ("finished", "finished"),
        (Target::Session(_), Kind::LeftBehind) => {
            ("left processes running", "left processes running")
        }
        // The model's own words say it whole (a crash restarted, an
        // account's room): the title is the line.
        _ => ("", ""),
    };
    let level = match due.kind {
        Kind::Waiting | Kind::Stuck | Kind::Limit | Kind::MacTight | Kind::Quota => 2,
        Kind::Finished
        | Kind::LeftBehind
        | Kind::Crashed
        | Kind::Reset
        | Kind::Watch
        | Kind::Digest => 1,
    };
    batch::Item {
        name: title.clone(),
        phrase: phrase.to_owned(),
        phrase_many: many.to_owned(),
        title,
        body,
        held: due.held,
        ..batch::Item::new(
            format!("e:{}:{:?}:{now}", model_kind(due.kind), due.target),
            due.kind,
            due.target,
            level,
        )
    }
}

/// Hands items (the watches' alerts, the digest's) to the notification
/// path: they go through [`show_due`]'s rules and the batch.
pub fn hand_over(state: &Arc<AppState>, items: Vec<batch::Item>) {
    if items.is_empty() {
        return;
    }
    if let Ok(mut incoming) = state.notifier.incoming.lock() {
        incoming.extend(items);
    }
    show_due(state);
}

/// AJ-E2904: the id a test notification carries (a click brings AI
/// Juggler forward, as an unknown id does).
const TEST_ID: i32 = 2_904;

/// Sends one system notification now, through the same way real ones go,
/// whatever the quiet hours and with AI Juggler in front (the user presses
/// it there); says what to check if none appeared.
///
/// # Errors
///
/// [`crate::error::AppError::Refused`] before notifications are connected.
pub fn send_test(state: &Arc<AppState>) -> crate::error::AppResult<String> {
    let sink = state.notifier.sink.get().ok_or_else(|| {
        crate::error::AppError::Refused(
            "Notifications aren't ready yet. Try again in a moment.".into(),
        )
    })?;
    sink(&Outgoing {
        id: TEST_ID,
        title: format!("{} test", crate::edition::current().app_name()),
        body: "Notifications reach you.".to_owned(),
    });
    Ok(format!(
        "Sent. If nothing appeared, allow {} in {}.",
        crate::edition::current().app_name(),
        juggler_platform::system::notification_settings_words()
    ))
}

/// The model's kind for a system notification's kind (Mute this kind).
const fn model_kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Waiting => "asks",
        Kind::Stuck => "stuck",
        Kind::Limit => "limit",
        Kind::Finished => "finished",
        Kind::MacTight => "room",
        Kind::LeftBehind => "leftovers",
        Kind::Quota => "quota",
        Kind::Crashed => "restarted",
        Kind::Reset => "reset",
        Kind::Watch => "watch",
        Kind::Digest => "digest",
    }
}

fn project_of(storage: &juggler_core::storage::Storage, session_id: &str) -> Option<String> {
    storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session_id))
        .ok()?
        .project
        .map(|project| project.as_str().to_owned())
}

fn account_of(storage: &juggler_core::storage::Storage, session_id: &str) -> Option<String> {
    let record = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session_id))
        .ok()?;
    let recipe = storage.profiles().get(&record.profile).ok()?;
    Some(recipe.account.to_string())
}

/// The title and line a notification shows.
fn words(state: &Arc<AppState>, due: &Due) -> (String, String) {
    if let Some(told) = state
        .notifier
        .told
        .lock()
        .ok()
        .and_then(|mut told| told.remove(&(due.target.clone(), due.kind)))
    {
        return told;
    }
    match &due.target {
        Target::Account(_) => ("An account needs a look".to_owned(), String::new()),
        Target::Centre => ("Something needs a look".to_owned(), String::new()),
        Target::Feed => ("Digest".to_owned(), String::new()),
        Target::ThisMac => state
            .notifier
            .mac_words
            .lock()
            .ok()
            .and_then(|words| words.clone())
            .unwrap_or_else(|| ("This Mac is getting tight".to_owned(), String::new())),
        Target::Session(id) => {
            let about = describe(state, id);
            let state_words = match due.kind {
                Kind::Waiting => "Waiting for your reply",
                Kind::Stuck => "No output for a while",
                Kind::Limit => "Hit its usage limit",
                Kind::Finished => "Finished",
                Kind::MacTight
                | Kind::LeftBehind
                | Kind::Quota
                | Kind::Crashed
                | Kind::Reset
                | Kind::Watch
                | Kind::Digest => "",
            };
            if due.kind == Kind::LeftBehind {
                let count = state
                    .notifier
                    .left
                    .lock()
                    .ok()
                    .and_then(|left| left.get(id).copied())
                    .unwrap_or(1);
                let mut line = vec![if count == 1 {
                    "Left 1 process running".to_owned()
                } else {
                    format!("Left {count} processes running")
                }];
                line.extend(about.place);
                line.extend(about.account);
                return (about.name, line.join(" · "));
            }
            let mut line = vec![state_words.to_owned()];
            line.extend(about.place);
            line.extend(about.account);
            (about.name, line.join(" · "))
        }
    }
}

pub(super) struct About {
    pub(super) name: String,
    pub(super) place: Option<String>,
    pub(super) account: Option<String>,
    pub(super) project_id: Option<String>,
}

/// A session as its row names it: its conversation's title, else its
/// folder; where it runs, and on which account.
pub(super) fn describe(state: &Arc<AppState>, session_id: &str) -> About {
    let info = crate::features::conversations::titled_snapshot(state)
        .into_iter()
        .find(|row| row.id == session_id);
    let Some(info) = info else {
        return About {
            name: "A session".to_owned(),
            place: None,
            account: None,
            project_id: None,
        };
    };
    let folder = std::path::Path::new(&info.cwd)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty());
    let name = info
        .label
        .clone()
        .or_else(|| folder.clone())
        .unwrap_or_else(|| "A session".to_owned());
    About {
        place: folder.filter(|folder| *folder != name),
        account: Some(info.account_label).filter(|label| !label.is_empty()),
        project_id: info.project_id.clone(),
        name,
    }
}

/// A notification was clicked: AI Juggler comes forward on its session
/// (or This Mac). An unknown id still brings the window forward.
pub fn clicked<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &Arc<AppState>, id: i32) {
    let page = |page: &str| OpenRequest {
        session_id: None,
        page: Some(page.to_owned()),
        machine_id: None,
        id: None,
    };
    let open = match state.notifier.target_of(id) {
        Some(batch::Opens::Target(Target::Session(session_id))) => Some(OpenRequest {
            session_id: Some(session_id),
            page: None,
            machine_id: None,
            id: None,
        }),
        Some(batch::Opens::Target(Target::ThisMac)) => Some(page("this_mac")),
        // AJ-E2501: an account's notice opens the notification centre,
        // where its banner and its actions are; so does a consolidated
        // notification with a need among them (ADR-0036 §1.7).
        Some(batch::Opens::Target(Target::Account(_) | Target::Centre) | batch::Opens::Centre) => {
            Some(page("notifications"))
        }
        // AJ-E3712: several calm items open Home's feed, at the newest
        // digest.
        Some(batch::Opens::Feed | batch::Opens::Target(Target::Feed)) => Some(page("home")),
        None => None,
    };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        crate::main_window::bring_forward(&handle);
        if let Some(open) = open {
            handle.emit_event(AppEvent::AppOpen, open);
        }
    });
}

/// The system's own notifications (the Tauri plugin), clicks routed back.
pub fn connect(app: &tauri::AppHandle, state: &Arc<AppState>) {
    use tauri_plugin_notification::NotificationExt;
    let shower = app.clone();
    state
        .notifier
        .set_sink(Box::new(move |outgoing: &Outgoing| {
            let _ = shower
                .notification()
                .builder()
                .id(outgoing.id)
                .title(&outgoing.title)
                .body(&outgoing.body)
                .show();
        }));
    let clicks = app.clone();
    let click_state = Arc::clone(state);
    let _ = app.notification().on_action(move |performed| {
        if performed.action_id() != "tap" {
            return;
        }
        if let Some(id) = performed
            .notification()
            .map(tauri_plugin_notification::ActiveNotification::id)
        {
            clicked(&clicks, &click_state, id);
        }
    });
}
