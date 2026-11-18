//! The store's vocabulary (ADR-0037 §1.1–§1.3): kinds, authority, scope,
//! tiers, the writer capability, the secret seam and the refusals.

use crate::error::CoreError;

/// Text-coded enums: one `as_str` and its parse, so the columns stay text
/// (0.6 adds values without a table rebuild, ADR-0037 §1.7).
macro_rules! coded {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident = $code:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name { $($(#[$vmeta])* $variant),+ }
        impl $name {
            /// The stored code.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $code),+ }
            }
            /// The value a stored code names, if any.
            #[must_use]
            pub fn parse(code: &str) -> Option<Self> {
                match code { $($code => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

coded! {
    /// What a record holds (ADR-0037 §1.1). There is no "fact" kind: live
    /// facts are read live (§3).
    Kind {
        /// A stated preference from a closed set of slots (`alias`, `focus`).
        Preference = "preference",
        /// The user's own words after "remember…".
        Note = "note",
        /// A confirmed watch (ADR-0036's `Watch`).
        StandingRequest = "standing_request",
        /// A question the user asked, with the entities it resolved to.
        Asked = "asked",
        /// What was shown for a question, as it was then.
        Answer = "answer",
        /// A digest (ADR-0036 §1.9).
        Digest = "digest",
        /// 0.6, reserved: a rule the user set for the bot.
        Policy = "policy",
        /// 0.6, reserved: a bot decision.
        Decision = "decision",
        /// 0.6, reserved: an act the bot took.
        Action = "action",
        /// 0.6, reserved: what came of an action.
        Outcome = "outcome",
    }
}

impl Kind {
    /// Built in 0.5 (the reserved kinds are refused, here and by a trigger).
    #[must_use]
    pub const fn is_built(self) -> bool {
        !matches!(
            self,
            Self::Policy | Self::Decision | Self::Action | Self::Outcome
        )
    }

    /// Recency half-life in days for retrieval (ADR-0037 §4.2); `long`
    /// says whether a watch is still active (long tier).
    #[must_use]
    pub const fn half_life_days(self, long: bool) -> f64 {
        match self {
            Self::Note | Self::Preference => 90.0,
            Self::StandingRequest if long => 90.0,
            Self::StandingRequest => 30.0,
            _ => 14.0,
        }
    }
}

coded! {
    /// Who a record's content comes from (ADR-0037 §1.3: user > app >
    /// model > agent; `bot` is reserved for 0.6).
    Authority {
        /// Said by the user.
        User = "user",
        /// Recorded by AI Juggler's code.
        App = "app",
        /// Inferred by a model.
        Model = "model",
        /// Agent-written text (always untrusted).
        Agent = "agent",
        /// 0.6, reserved.
        Bot = "bot",
    }
}

impl Authority {
    /// The supersede order: a write may replace the current row only at
    /// the same rank or higher.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::User => 4,
            Self::App => 3,
            Self::Model | Self::Bot => 2,
            Self::Agent => 1,
        }
    }
}

coded! {
    /// How long a record lives (ADR-0037 §2.1).
    Tier {
        /// The conversation; model-noted rows not yet confirmed.
        Short = "short",
        /// What was asked and answered; digests.
        Mid = "mid",
        /// What the user stated; active watches; promoted rows.
        Long = "long",
    }
}

coded! {
    /// Where a record came from (ADR-0037 §1.2 `source_kind`).
    SourceKind {
        /// A Chat turn.
        ChatTurn = "chat_turn",
        /// The Watches view.
        WatchesView = "watches_view",
        /// Settings › AI › Memory.
        Settings = "settings",
        /// The digest job.
        DigestJob = "digest_job",
        /// The (Beta) note job.
        NoteJob = "note_job",
        /// Maintenance.
        Maintenance = "maintenance",
    }
}

coded! {
    /// The entity kinds a record may name.
    EntityKind {
        /// A session.
        Session = "session",
        /// An App (desktop or CLI app, or a profile).
        App = "app",
        /// A project.
        Project = "project",
        /// A pool.
        Pool = "pool",
        /// An account.
        Account = "account",
    }
}

/// Priority, 0 low … 3 pinned (ADR-0037 §1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    /// Decayed.
    Low = 0,
    /// The default.
    Normal = 1,
    /// Promoted.
    High = 2,
    /// The user pinned it.
    Pinned = 3,
}

impl Priority {
    /// The stored number.
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self as i64
    }

    /// The priority a stored number names (out of range reads as normal).
    #[must_use]
    pub const fn from_i64(value: i64) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::High,
            3 => Self::Pinned,
            _ => Self::Normal,
        }
    }

    /// Retrieval's weight (ADR-0037 §4.2).
    #[must_use]
    pub const fn weight(self) -> f64 {
        match self {
            Self::Low => 0.5,
            Self::Normal => 1.0,
            Self::High => 1.5,
            Self::Pinned => 2.0,
        }
    }
}

/// Where a record applies.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Everywhere.
    Global,
    /// One project.
    Project(String),
    /// One pool.
    Pool(String),
    /// One session.
    Session(String),
}

impl Scope {
    /// `(scope_kind, scope_id)` as stored.
    #[must_use]
    pub fn columns(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::Global => ("global", None),
            Self::Project(id) => ("project", Some(id)),
            Self::Pool(id) => ("pool", Some(id)),
            Self::Session(id) => ("session", Some(id)),
        }
    }

    /// The scope stored columns name.
    #[must_use]
    pub fn from_columns(kind: &str, id: Option<String>) -> Option<Self> {
        match (kind, id) {
            ("global", None) => Some(Self::Global),
            ("project", Some(id)) => Some(Self::Project(id)),
            ("pool", Some(id)) => Some(Self::Pool(id)),
            ("session", Some(id)) => Some(Self::Session(id)),
            _ => None,
        }
    }
}

/// An entity a record names (for recall and "forget everything about web").
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityRef {
    /// Its kind.
    pub kind: EntityKind,
    /// Its id.
    pub id: String,
}

/// Agent-written text inside a payload. It serialises as
/// `{"$untrusted": …}`, and the write gate marks any record whose payload
/// holds one `untrusted` (ADR-0037 §1.2: set by type, not by care).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Untrusted<T> {
    /// The agent's words.
    #[serde(rename = "$untrusted")]
    pub value: T,
}

/// The marker key [`Untrusted`] writes.
pub const UNTRUSTED_KEY: &str = "$untrusted";

/// The capability to write some kinds (ADR-0037 §1.1: each kind has one
/// writer), with the authorities and sources that writer may claim. It
/// can't be made outside this module: [`Writers::issue`] hands every
/// owner its one writer, once per process, at start. The gate refuses a
/// kind, authority or source the writer doesn't cover, so a module can't
/// declare itself the user (the security review's M2).
#[derive(Debug)]
pub struct KindWriter {
    kinds: &'static [Kind],
    authorities: &'static [Authority],
    sources: &'static [SourceKind],
}

impl KindWriter {
    /// It may write `kind`, as `authority`, from `source`.
    #[must_use]
    pub fn covers(&self, kind: Kind, authority: Authority, source: SourceKind) -> bool {
        self.kinds.contains(&kind)
            && self.authorities.contains(&authority)
            && self.sources.contains(&source)
    }
    /// A writer covering everything, for the gate's own tests.
    #[cfg(test)]
    pub(crate) const fn any() -> Self {
        Self {
            kinds: &[
                Kind::Preference,
                Kind::Note,
                Kind::StandingRequest,
                Kind::Asked,
                Kind::Answer,
                Kind::Digest,
                Kind::Policy,
            ],
            authorities: &[
                Authority::User,
                Authority::App,
                Authority::Model,
                Authority::Agent,
                Authority::Bot,
            ],
            sources: &[
                SourceKind::ChatTurn,
                SourceKind::WatchesView,
                SourceKind::Settings,
                SourceKind::DigestJob,
                SourceKind::NoteJob,
                SourceKind::Maintenance,
            ],
        }
    }
}

/// Every writer of the store, one per owning module (ADR-0037 §1.1's
/// table). Issued once per process; the app hands each module its own.
#[derive(Debug)]
pub struct Writers {
    /// `features/ai/memory`: Chat's remember flow and Settings write the
    /// user's notes and preferences; the (Beta) note job proposes them.
    pub memory: KindWriter,
    /// `features/watches`: a watch, only through its three commands.
    pub watches: KindWriter,
    /// `features/ai/chat`: what was asked and what was shown.
    pub chat: KindWriter,
    /// `features/digest`: digests (the rollup by the app, items by a
    /// model, quotes as agent text).
    pub digest: KindWriter,
}

static ISSUED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

impl Writers {
    /// The writers, the first time only (`None` after): the composition
    /// root takes them at start and hands each module its own.
    #[must_use]
    pub fn issue() -> Option<Self> {
        (!ISSUED.swap(true, std::sync::atomic::Ordering::SeqCst)).then(Self::make)
    }

    /// A fresh set for a test (each test its own store).
    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self::make()
    }

    const fn make() -> Self {
        Self {
            memory: KindWriter {
                kinds: &[Kind::Preference, Kind::Note],
                authorities: &[Authority::User, Authority::Model],
                sources: &[
                    SourceKind::ChatTurn,
                    SourceKind::Settings,
                    SourceKind::NoteJob,
                ],
            },
            watches: KindWriter {
                kinds: &[Kind::StandingRequest],
                authorities: &[Authority::User],
                sources: &[SourceKind::WatchesView, SourceKind::ChatTurn],
            },
            chat: KindWriter {
                kinds: &[Kind::Asked, Kind::Answer],
                authorities: &[Authority::App],
                sources: &[SourceKind::ChatTurn],
            },
            digest: KindWriter {
                kinds: &[Kind::Digest],
                authorities: &[Authority::App, Authority::Model, Authority::Agent],
                sources: &[SourceKind::DigestJob],
            },
        }
    }
}

/// A record's label, typed by whose words it holds (ADR-0037 §1.2: set by
/// type, not by care). Agent-written words are `Untrusted`, and the record
/// is marked so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordText {
    /// The user's words, or text the app or a gated model wrote.
    Trusted(String),
    /// Agent-written words.
    Untrusted(String),
}

impl RecordText {
    /// The words.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Trusted(text) | Self::Untrusted(text) => text,
        }
    }

    /// Agent-written.
    #[must_use]
    pub const fn is_untrusted(&self) -> bool {
        matches!(self, Self::Untrusted(_))
    }
}

/// The D-14 secret-hiding pass, handed in by the app (the detector lives
/// in `juggler-handover`, which this crate must not know).
pub trait SecretScan {
    /// The text holds something that looks like a key or password.
    fn finds_secret(&self, text: &str) -> bool;
    /// The text with each secret look-alike replaced.
    fn mask(&self, text: &str) -> String;
}

/// Why the gate refused a write. Codes only, never the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The writer doesn't cover the kind, the authority or the source.
    WrongWriter,
    /// A 0.6 kind, or the `bot` authority.
    Reserved,
    /// A lower authority against a higher current row.
    LowerAuthority,
    /// It looks like a key or password.
    Secret,
    /// Empty text, or over 280 characters.
    TextBounds,
    /// The payload isn't a JSON object, or is over 16 KB.
    PayloadBounds,
    /// Agent-authored but not untrusted.
    AgentNotUntrusted,
    /// Confidence outside 0..1, below 1 for user or app, or below the
    /// gate (0.8) for a model.
    Confidence,
    /// A log payload or code holds free text.
    NotTextFree,
    /// The subject key is empty, too long or not code-like.
    SubjectKey,
    /// A preference that isn't a setting's shape: it names no entity, or
    /// its label is over [`PREFERENCE_MAX_CHARS`]. The caller keeps such
    /// words as a note (local only).
    PreferenceShape,
}

/// The longest label a preference may have (an alias, a focus's name).
pub const PREFERENCE_MAX_CHARS: usize = 60;

impl Refusal {
    /// The code logged with a refusal.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::WrongWriter => "wrong_writer",
            Self::Reserved => "reserved",
            Self::LowerAuthority => "lower_authority",
            Self::Secret => "secret",
            Self::TextBounds => "text_bounds",
            Self::PayloadBounds => "payload_bounds",
            Self::AgentNotUntrusted => "agent_not_untrusted",
            Self::Confidence => "confidence",
            Self::NotTextFree => "not_text_free",
            Self::SubjectKey => "subject_key",
            Self::PreferenceShape => "preference_shape",
        }
    }
}

/// A memory store or log failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MemoryError {
    /// The write gate refused it (nothing was written but the refusal's
    /// log event).
    #[error("refused: {}", .0.code())]
    Refused(Refusal),
    /// A storage failure.
    #[error(transparent)]
    Core(#[from] CoreError),
}

impl From<rusqlite::Error> for MemoryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Core(CoreError::Storage(error))
    }
}

impl From<serde_json::Error> for MemoryError {
    fn from(error: serde_json::Error) -> Self {
        Self::Core(CoreError::Serialization(error))
    }
}

/// A code-like string: ids, enum codes, engine ids, subject keys. Never
/// free text (no spaces), at most `max` bytes.
#[must_use]
pub fn code_like(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':' | b',' | b'$')
        })
}
