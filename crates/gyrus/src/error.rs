//! Errors.

/// What can go wrong.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// SQLite.
    #[error("storage: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A pack that doesn't hold together.
    #[error("pack: {0}")]
    Pack(String),
    /// A write the pack doesn't declare, or one out of its bounds.
    #[error("refused: {0}")]
    Refused(Refusal),
    /// The index isn't built yet (it builds lazily).
    #[error("the index is warming up")]
    WarmingUp,
}

/// Why a write was refused. Each is a code, never the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A node kind the pack doesn't declare.
    UndeclaredKind,
    /// A link kind the pack doesn't declare, or between kinds it doesn't
    /// allow.
    UndeclaredLink,
    /// A link to or from a key that doesn't exist.
    NoSuchNode,
    /// A key that isn't a code.
    Key,
    /// A label, alias or props out of bounds.
    Bounds,
    /// An immediate-tier node (held in memory only, never stored).
    Immediate,
    /// A change by a lower authority than the node's.
    LowerAuthority,
    /// An agent's node not marked untrusted.
    AgentNotUntrusted,
    /// A weight, confidence, salience or priority out of range.
    Range,
    /// More links of a kind than the source kind allows.
    TooManyLinks,
}

impl Refusal {
    /// The code (for a log).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UndeclaredKind => "undeclared_kind",
            Self::UndeclaredLink => "undeclared_link",
            Self::NoSuchNode => "no_such_node",
            Self::Key => "key",
            Self::Bounds => "bounds",
            Self::Immediate => "immediate",
            Self::LowerAuthority => "lower_authority",
            Self::AgentNotUntrusted => "agent_not_untrusted",
            Self::Range => "range",
            Self::TooManyLinks => "too_many_links",
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

/// gyrus's result.
pub type Result<T> = std::result::Result<T, Error>;
