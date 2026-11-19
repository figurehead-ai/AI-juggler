//! Watches on ADR-0037's store (ADR-0036 §2.7): each a `standing_request`
//! record, said by the user, written only through its commands with the
//! watches' own [`KindWriter`] (create, edit, pause, resume, remove), each
//! change and its `watch` log event in one transaction. An edit is a new
//! version that supersedes the old. What the engine says (a fire, an
//! ending) goes to the log only: the engine never writes a watch, and
//! the store reads a watch's ending back from its log.
//!
//! The store's tables are V54 (ADR-0037 §1.7), listed right after
//! 0.4.6's V53; on a database without them (one restored from before)
//! [`available`] is false and every command says so.

use juggler_core::storage::Storage;
use juggler_core::storage::memory::{
    self, Actor, AiLog, Authority, EntityKind, EntityRef, Kind, KindWriter, MemoryError, NewEvent,
    NewRecord, Priority, RecordText, Scope, SecretScan, SourceKind, Stream, Tier, code_like,
};
use serde::{Deserialize, Serialize};

use super::conditions::Registry;
use super::echo::{self, LocalClock, Unnamed};
use super::engine::{EndReason, Expiry, Fire};
use super::model::{MAX_ACTIVE, Watch, WatchError, WatchId, WatchTarget};
use super::words::Lang;

/// The log's subject kind for a watch.
pub const SUBJECT: &str = "watch";

/// The store's subject key for a watch (ADR-0037 §1.1).
#[must_use]
pub fn subject_key(id: &WatchId) -> String {
    format!("watch:{}", id.0)
}

/// Where a change came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// The Watches view (or a session row's Watch…).
    WatchesView,
    /// A Chat draft the user confirmed, by its turn id.
    ChatTurn(String),
}

impl Origin {
    fn source(&self) -> (SourceKind, Option<String>) {
        match self {
            Self::WatchesView => (SourceKind::WatchesView, None),
            Self::ChatTurn(turn) => (SourceKind::ChatTurn, Some(turn.clone())),
        }
    }
}

/// A stored watch's payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Payload {
    watch: Watch,
    #[serde(default)]
    removed: bool,
}

/// A stored row's place in the list: its version's time and record id.
pub type ListKey = (i64, String);

/// A watch as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stored {
    /// The watch.
    pub watch: Watch,
    /// The user removed it.
    pub removed: bool,
    /// Its version (an edit, a pause, a resume: each one more).
    pub version: i64,
    /// When this version was written.
    pub updated_ms: i64,
    /// Its place in the list: this version's time and record id.
    pub key: ListKey,
}

/// Why a store step failed.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The memory store's tables aren't in this database yet (V54).
    #[error("unavailable")]
    Unavailable,
    /// No such watch (or it was removed).
    #[error("not_found")]
    NotFound,
    /// The watch already exists.
    #[error("exists")]
    Exists,
    /// The watch was refused.
    #[error("{}", .0.code())]
    Invalid(WatchError),
    /// The store's gate or storage.
    #[error(transparent)]
    Memory(#[from] MemoryError),
}

/// The store's tables are in this database.
#[must_use]
pub fn available(storage: &Storage) -> bool {
    memory::installed(storage.raw_conn()).unwrap_or(false)
}

fn scope_of(watch: &Watch) -> Scope {
    match watch.targets.as_slice() {
        [WatchTarget::Session(id)] => Scope::Session(id.clone()),
        [WatchTarget::Project(id)] => Scope::Project(id.clone()),
        [WatchTarget::Pool(id)] => Scope::Pool(id.clone()),
        _ => Scope::Global,
    }
}

fn entities_of(watch: &Watch) -> Vec<EntityRef> {
    watch
        .targets
        .iter()
        .filter_map(|target| {
            let kind = match target {
                WatchTarget::Session(_) => EntityKind::Session,
                WatchTarget::App(_) => EntityKind::App,
                WatchTarget::Project(_) => EntityKind::Project,
                WatchTarget::Pool(_) => EntityKind::Pool,
                WatchTarget::Account(_) => EntityKind::Account,
                WatchTarget::Any => return None,
            };
            Some(EntityRef {
                kind,
                id: target.id()?.to_owned(),
            })
        })
        .collect()
}

/// A change's log event: codes only (the conditions, the lifetime, the
/// quiet switch), never words.
fn change_event(watch: &Watch, verb: &'static str, now_ms: i64) -> NewEvent {
    let mut event = NewEvent::new(now_ms, Stream::Watch, verb, Actor::User);
    event.subject = Some((SUBJECT, watch.id.0.clone()));
    let lifetime = match watch.lifetime {
        super::model::Lifetime::Once => "once",
        super::model::Lifetime::UntilEnds => "until_ends",
        super::model::Lifetime::Until { .. } => "until",
        super::model::Lifetime::Always => "always",
    };
    event.payload = serde_json::json!({
        "targets": watch.targets.len(),
        "conditions": watch.conditions.iter().map(|c| c.code.clone()).collect::<Vec<_>>(),
        "lifetime": lifetime,
        "all": watch.matching == super::model::Matching::All,
        "quiet": watch.even_in_quiet_hours,
        "paused": watch.paused,
    });
    event
}

/// The label kept with the record: the watch's echo with each target by
/// its kind, so it holds no agent-written name (names are read live).
fn label(registry: &Registry, watch: &Watch, now_ms: i64) -> String {
    let words = echo::echo(
        registry,
        watch,
        &Unnamed,
        Lang::En,
        LocalClock {
            now_ms,
            offset_hours: 0,
        },
    );
    words.chars().take(memory::TEXT_MAX_CHARS).collect()
}

/// The watch store over one storage hold.
pub struct WatchStore<'a> {
    storage: &'a Storage,
    registry: &'a Registry,
}

impl<'a> WatchStore<'a> {
    /// The store on `storage`.
    #[must_use]
    pub const fn new(storage: &'a Storage, registry: &'a Registry) -> Self {
        Self { storage, registry }
    }

    fn ready(&self) -> Result<(), StoreError> {
        if available(self.storage) {
            Ok(())
        } else {
            Err(StoreError::Unavailable)
        }
    }

    fn log(&self) -> AiLog<'a> {
        self.storage.ai_log()
    }

    /// The current stored version of a watch, removed or not.
    ///
    /// # Errors
    ///
    /// The store isn't there; storage failures.
    pub fn get(&self, id: &WatchId) -> Result<Option<Stored>, StoreError> {
        self.ready()?;
        let Some(record) = self.storage.memory().current(&subject_key(id))? else {
            return Ok(None);
        };
        Ok(serde_json::from_value::<Payload>(record.payload)
            .ok()
            .map(|payload| Stored {
                watch: payload.watch,
                removed: payload.removed,
                version: record.version,
                updated_ms: record.created_ms,
                key: (record.created_ms, record.id),
            }))
    }

    /// One keyset page of the watches the user hasn't removed, newest
    /// change first (active and ended alike; [`Self::ended`] tells them
    /// apart): up to `limit` stored rows after `after` (a row's
    /// [`Stored::key`]), and whether more follow. A removed watch's row
    /// counts toward the page but isn't in it.
    ///
    /// # Errors
    ///
    /// The store isn't there; storage failures.
    pub fn page(
        &self,
        after: Option<&ListKey>,
        limit: u32,
    ) -> Result<(Vec<Stored>, Option<ListKey>), StoreError> {
        self.ready()?;
        let after = after.map(|(at, id)| (*at, id.as_str()));
        let mut records = self.storage.memory().current_of_kind(
            Kind::StandingRequest,
            after,
            limit.saturating_add(1),
        )?;
        let more = records.len() > usize::try_from(limit).unwrap_or(usize::MAX);
        records.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        let next = more
            .then(|| records.last().map(|r| (r.created_ms, r.id.clone())))
            .flatten();
        let rows = records
            .into_iter()
            .filter_map(|record| {
                let payload: Payload = serde_json::from_value(record.payload).ok()?;
                (!payload.removed).then_some(Stored {
                    watch: payload.watch,
                    removed: false,
                    version: record.version,
                    updated_ms: record.created_ms,
                    key: (record.created_ms, record.id),
                })
            })
            .collect();
        Ok((rows, next))
    }

    /// Every watch the user hasn't removed, newest change first, read a
    /// page at a time.
    ///
    /// # Errors
    ///
    /// The store isn't there; storage failures.
    pub fn list(&self) -> Result<Vec<Stored>, StoreError> {
        let mut out = Vec::new();
        let mut after = None;
        loop {
            let (rows, next) = self.page(after.as_ref(), 200)?;
            out.extend(rows);
            match next {
                Some(key) => after = Some(key),
                None => return Ok(out),
            }
        }
    }

    /// How a watch ended, from its log: the reason and when; `None`
    /// while it is active (a newer edit or resume after an ending makes
    /// it active again).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn ended(&self, id: &WatchId) -> Result<Option<(EndReason, i64)>, StoreError> {
        for event in self.log().of_subject(SUBJECT, &id.0, 50)? {
            match event.verb.as_str() {
                "expired" | "ended" => {
                    let reason = event
                        .payload
                        .get("reason")
                        .and_then(serde_json::Value::as_str)
                        .and_then(reason_of)
                        .unwrap_or(EndReason::TimeReached);
                    return Ok(Some((reason, event.at_ms)));
                }
                "created" | "edited" | "resumed" => return Ok(None),
                _ => {}
            }
        }
        Ok(None)
    }

    /// How many times a watch told the user.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn fired_count(&self, id: &WatchId) -> Result<u64, StoreError> {
        Ok(self.log().count_of_subject(SUBJECT, &id.0, "fired")?)
    }

    /// The watches that are neither removed nor ended: what the engine
    /// hears for.
    ///
    /// # Errors
    ///
    /// The store isn't there; storage failures.
    pub fn active(&self) -> Result<Vec<Stored>, StoreError> {
        let mut out = Vec::new();
        for stored in self.list()? {
            if self.ended(&stored.watch.id)?.is_none() {
                out.push(stored);
            }
        }
        Ok(out)
    }

    fn write(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        payload: &Payload,
        origin: &Origin,
        verb: &'static str,
        now_ms: i64,
    ) -> Result<Stored, StoreError> {
        let watch = &payload.watch;
        let (source, source_id) = origin.source();
        let record = NewRecord {
            subject_key: subject_key(&watch.id),
            kind: Kind::StandingRequest,
            scope: scope_of(watch),
            // ADR-0037 §2.2: long while active; a removed one ages as mid.
            tier: if payload.removed {
                Tier::Mid
            } else {
                Tier::Long
            },
            priority: Priority::Normal,
            authority: Authority::User,
            source,
            source_id,
            engine: None,
            confidence: 1.0,
            as_of_ms: now_ms,
            source_rev: None,
            text: RecordText::Trusted(label(self.registry, watch, now_ms)),
            payload: serde_json::to_value(payload).map_err(MemoryError::from)?,
            expires_ms: None,
            entities: entities_of(watch),
        };
        let event = change_event(watch, verb, now_ms);
        let written = self
            .storage
            .memory()
            .remember_logged(writer, secrets, &record, &event, now_ms)?;
        Ok(Stored {
            watch: watch.clone(),
            removed: payload.removed,
            version: written.version,
            updated_ms: now_ms,
            key: (now_ms, written.id),
        })
    }

    /// Creates a watch the user confirmed (validated against the closed
    /// set; at most [`MAX_ACTIVE`] active).
    ///
    /// # Errors
    ///
    /// Refused ([`StoreError::Invalid`], [`StoreError::Exists`]); the
    /// store isn't there; the gate or storage.
    pub fn create(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        watch: &Watch,
        origin: &Origin,
        now_ms: i64,
    ) -> Result<Stored, StoreError> {
        self.ready()?;
        watch
            .validate(self.registry, now_ms)
            .map_err(StoreError::Invalid)?;
        if self.get(&watch.id)?.is_some() {
            return Err(StoreError::Exists);
        }
        if self.active()?.len() >= MAX_ACTIVE {
            return Err(StoreError::Invalid(WatchError::TooMany));
        }
        let payload = Payload {
            watch: watch.clone(),
            removed: false,
        };
        self.write(writer, secrets, &payload, origin, "created", now_ms)
    }

    /// Changes a watch: a new version superseding the old.
    ///
    /// # Errors
    ///
    /// As [`Self::create`]; [`StoreError::NotFound`].
    pub fn update(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        watch: &Watch,
        origin: &Origin,
        now_ms: i64,
    ) -> Result<Stored, StoreError> {
        self.ready()?;
        let current = self.current_live(&watch.id)?;
        watch
            .validate(self.registry, now_ms)
            .map_err(StoreError::Invalid)?;
        let mut next = watch.clone();
        next.created_ms = current.watch.created_ms;
        let payload = Payload {
            watch: next,
            removed: false,
        };
        self.write(writer, secrets, &payload, origin, "edited", now_ms)
    }

    /// Pauses or resumes a watch.
    ///
    /// # Errors
    ///
    /// The store isn't there; [`StoreError::NotFound`]; the gate or storage.
    pub fn set_paused(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        id: &WatchId,
        paused: bool,
        origin: &Origin,
        now_ms: i64,
    ) -> Result<Stored, StoreError> {
        self.ready()?;
        let mut watch = self.current_live(id)?.watch;
        watch.paused = paused;
        let payload = Payload {
            watch,
            removed: false,
        };
        let verb = if paused { "paused" } else { "resumed" };
        self.write(writer, secrets, &payload, origin, verb, now_ms)
    }

    /// Removes a watch: a last version marked removed (kept for "what did
    /// I ask you to watch", ADR-0037 §2.2; Settings' forget erases it).
    ///
    /// # Errors
    ///
    /// The store isn't there; [`StoreError::NotFound`]; the gate or storage.
    pub fn remove(
        &self,
        writer: &KindWriter,
        secrets: &dyn SecretScan,
        id: &WatchId,
        origin: &Origin,
        now_ms: i64,
    ) -> Result<Stored, StoreError> {
        self.ready()?;
        let watch = self.current_live(id)?.watch;
        let payload = Payload {
            watch,
            removed: true,
        };
        self.write(writer, secrets, &payload, origin, "removed", now_ms)
    }

    fn current_live(&self, id: &WatchId) -> Result<Stored, StoreError> {
        match self.get(id)? {
            Some(stored) if !stored.removed => Ok(stored),
            _ => Err(StoreError::NotFound),
        }
    }

    /// Logs a fire (the app's own record of what the engine said; no
    /// watch is written).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn log_fire(&self, fire: &Fire) -> Result<i64, StoreError> {
        let mut event = NewEvent::new(fire.at_ms, Stream::Watch, "fired", Actor::App);
        event.subject = Some((SUBJECT, fire.watch.0.clone()));
        let mut payload = serde_json::json!({
            "condition": fire.condition,
            "cause": fire.cause,
            "replayed": fire.replayed,
            "quiet": fire.even_in_quiet_hours,
        });
        if let Some(session) = fire.session.as_deref().filter(|s| code_like(s, 96)) {
            payload["session"] = serde_json::Value::from(session);
        }
        event.payload = payload;
        Ok(self.log().append(&event, None)?)
    }

    /// Logs a watch's ending: `expired` for its own lifetime, `ended` when
    /// what it watched is gone or no longer offered.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn log_end(&self, expiry: &Expiry) -> Result<i64, StoreError> {
        let verb = match expiry.reason {
            EndReason::TargetGone | EndReason::NoLongerOffered => "ended",
            EndReason::FirstFire | EndReason::TargetsEnded | EndReason::TimeReached => "expired",
        };
        let mut event = NewEvent::new(expiry.at_ms, Stream::Watch, verb, Actor::App);
        event.subject = Some((SUBJECT, expiry.watch.0.clone()));
        event.payload = serde_json::json!({ "reason": expiry.reason.code() });
        Ok(self.log().append(&event, None)?)
    }
}

fn reason_of(code: &str) -> Option<EndReason> {
    [
        EndReason::FirstFire,
        EndReason::TargetsEnded,
        EndReason::TimeReached,
        EndReason::TargetGone,
        EndReason::NoLongerOffered,
    ]
    .into_iter()
    .find(|reason| reason.code() == code)
}
