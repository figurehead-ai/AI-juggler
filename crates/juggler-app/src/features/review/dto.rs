//! The turn review's wire types (AJ-E3820). Facts and the agent's own
//! words only: every sentence around them is the window's
//! (`ui/src/lib/review-words.ts`), so the words are written and tested in
//! one place.

/// Which turn to review: a session's or another app's conversation, and
/// when the turn ended (the feed item's time, or the session page's last
/// turn's).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReviewAsk {
    /// An AI Juggler session.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub session_id: Option<String>,
    /// The Account folder of a conversation (with `conversation_id`).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub account_id: Option<String>,
    /// The conversation's id in its tool.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub conversation_id: Option<String>,
    /// When the turn ended, epoch ms; none: the transcript's last turn.
    #[serde(default)]
    #[ts(optional = nullable, type = "number | null")]
    pub until_ms: Option<i64>,
    /// How it ended, as the feed says it (`failed` and `crashed` are a
    /// failed turn).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub ended: Option<String>,
}

/// A changed file in the review.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReviewFileInfo {
    /// Its name (a path's last two parts, as the feed's diff stat).
    pub name: String,
    /// Lines added; `None` when a call doesn't say (files only).
    pub added: Option<u32>,
    /// Lines removed; see `added`.
    pub removed: Option<u32>,
    /// The model's short description (the Beta), checked; none in the
    /// code-only review.
    pub about: Option<String>,
}

/// The review of one turn.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TurnReviewInfo {
    /// `shown`; or why there is none: `off` (Last messages off: the
    /// agent's words aren't read), `unseen` (its App's turn ends aren't
    /// seen, R5), `none` (no transcript on this Mac), `unreadable`.
    pub state: String,
    /// The turn, as a key: the same turn read twice is one turn.
    pub turn: String,
    /// The files its edit and write calls touched, reverted ones too: the
    /// post's diff stat's count (0.5.2 G25: one count for both).
    pub files: u32,
    /// The files its edit and write calls changed, reverts left out.
    pub changed: Vec<ReviewFileInfo>,
    /// The files its own words say it undid.
    pub reverted: Vec<String>,
    /// It says it undid part of its work without naming which file: the
    /// changed files are said as edited, not as changed.
    pub undone_unclear: bool,
    /// The agent's own sentence saying what it undid.
    pub reverted_words: Option<String>,
    /// The agent's own sentences about what failed.
    pub failures: Vec<String>,
    /// The agent's own question to you.
    pub question: Option<String>,
    /// Its tests, as its own words say: `passed`, `failed` or `not_run`;
    /// none when they don't say.
    pub tests: Option<String>,
    /// The agent's own sentence about its tests.
    pub tests_words: Option<String>,
    /// `low`, `medium` or `high`, from the facts (never low on a failed or
    /// untested turn).
    pub risk: String,
    /// Why, as keys (`failed`, `tests_failed`, `failures`, `tests_not_run`,
    /// `untested`, `asks`, `reverted`, `large`), most serious first.
    pub risk_why: Vec<String>,
    /// The model's lines, when the Beta wrote them (they replace nothing:
    /// each file's `about`, and this).
    pub ai: Option<ReviewAiInfo>,
}

/// What the model added (the Beta, D-149), after the shared guard and
/// check.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReviewAiInfo {
    /// The model's display name.
    pub model: String,
    /// Its risk, when higher than the facts' (it may only raise it).
    pub risk: Option<String>,
    /// Its reason, in the agent's words, checked.
    pub risk_reason: Option<String>,
    /// The source the lines were checked against: the agent's reply after
    /// the guard, cut to [`SOURCE_MAX`] characters (one tap shows it,
    /// ADR-0033's Beta).
    pub source: String,
}

/// The most of the agent's reply a review's source shows.
pub const SOURCE_MAX: usize = 1_500;
