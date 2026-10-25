//! AJ-E2302: the neutral events and answers of a driven session. Each
//! adapter's codec turns its tool's messages into these; core, the
//! supervisor and the UI see nothing else (ADR-0019).
//!
//! Text the tool sends (its messages, a command it wants to run, a
//! question) is held only to show it: nothing here is stored, and every
//! text field is bounded ([`bounded`]).

use serde::{Deserialize, Serialize};

use crate::live::LiveStatus;

/// The longest text one event carries (a summary, a delta, a question):
/// the rest is cut on a character boundary, never mid-way.
pub const TEXT_MAX: usize = 4 * 1024;

/// `text` cut to [`TEXT_MAX`] bytes on a character boundary.
#[must_use]
pub fn bounded(text: &str) -> String {
    if text.len() <= TEXT_MAX {
        return text.to_owned();
    }
    let mut end = TEXT_MAX;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// The tool's own id for one pending ask (an approval or a question).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AskId(pub String);

impl AskId {
    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Who started a turn: the person, or a message delivered into the
/// session (E18, D-29: labelled with its sender; the label itself is the
/// delivery's, the runtime carries the text unchanged).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TurnOrigin {
    /// The user, by a click or a reply in AI Juggler.
    User,
    /// A message AI Juggler delivered between turns.
    Delivered {
        /// Who sent it (a session's name or a pool's member).
        sender: String,
    },
}

/// One user turn for a driven session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnInput {
    /// The turn's text, exactly as the tool receives it.
    pub text: String,
    /// Who it is from.
    pub origin: TurnOrigin,
}

impl TurnInput {
    /// A turn the user typed.
    #[must_use]
    pub fn from_user(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            origin: TurnOrigin::User,
        }
    }
}

/// How a turn ended, as the tool says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnEnd {
    /// It finished.
    Completed,
    /// It was interrupted (by the user, or a "Deny and stop").
    Interrupted,
    /// It failed (see the error event before it).
    Failed,
}

/// Tokens, as the tool reports them for a turn or the session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnUsage {
    /// Input tokens.
    pub input: u64,
    /// Of which read from the cache.
    pub cached_input: u64,
    /// Output tokens.
    pub output: u64,
    /// Of which reasoning.
    pub reasoning: u64,
    /// The model's context window, when the tool says.
    pub context_window: Option<u64>,
}

/// One limit window, as the tool reports it: never one assumed for the
/// tool or the plan (P13).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LimitWindow {
    /// The tool's own name for it (`primary`, `five_hour`, …).
    pub id: String,
    /// Its length in minutes, when the tool says.
    pub window_mins: Option<u32>,
    /// How much of it is used, 0–100.
    pub used_pct: f64,
    /// When it resets, epoch ms, when the tool says.
    pub resets_at_ms: Option<i64>,
}

/// What the tool reported about its account's limits.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LimitsSnapshot {
    /// The windows, in the tool's order.
    pub windows: Vec<LimitWindow>,
    /// The plan the tool named, when it did.
    pub plan: Option<String>,
    /// The tool says a limit is reached now.
    pub reached: bool,
}

/// What an approval is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKind {
    /// Running a command.
    Command,
    /// Changing files.
    FileChange,
    /// More permissions (folders, network).
    Permissions,
    /// Using a tool (an MCP tool, a web fetch, …).
    Tool,
}

/// What choosing an option does, in neutral terms (the label is the
/// tool's own words).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OptionEffect {
    /// Allow this once.
    AllowOnce,
    /// Allow, and the tool remembers a rule of its own (D-28 as amended:
    /// "don't ask again" is the tool's rule, visible and revocable there).
    AllowRule {
        /// What the tool will remember, in its words.
        rule: String,
    },
    /// Deny; the turn goes on.
    Deny,
    /// Deny and stop the turn.
    DenyAndStop,
}

/// One option an ask offers: only ever one the tool itself offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskOption {
    /// The codec's id for it (what [`AskAnswer::Choose`] names).
    pub id: String,
    /// Its label, in the tool's words.
    pub label: String,
    /// What it does.
    pub effect: OptionEffect,
}

/// A tool asking to do something.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    /// The ask's id.
    pub id: AskId,
    /// What for.
    pub kind: ApprovalKind,
    /// What it asks, in one line (the command, the file, the tool).
    pub summary: String,
    /// Why, when the tool says.
    pub reason: Option<String>,
    /// The options the tool offers, in its order.
    pub options: Vec<AskOption>,
}

/// One choice a question offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    /// Its label (also what an answer sends).
    pub label: String,
    /// More about it, when the tool says.
    pub description: Option<String>,
}

/// One part of a question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionPart {
    /// The tool's id for this part.
    pub id: String,
    /// A short header.
    pub header: String,
    /// The question.
    pub question: String,
    /// The tool's choices (empty: a free reply).
    pub choices: Vec<Choice>,
    /// More than one choice may be picked.
    pub multi: bool,
    /// A free reply is allowed next to the choices.
    pub free_text: bool,
    /// The answer is a secret: it is never shown again or logged.
    pub secret: bool,
}

/// A tool asking the user something.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    /// The ask's id.
    pub id: AskId,
    /// Its parts, in order.
    pub parts: Vec<QuestionPart>,
}

/// One part's answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartAnswer {
    /// The part's id.
    pub part: String,
    /// The chosen labels, or the free reply.
    pub answers: Vec<String>,
}

/// The user's own answer to an ask, carried to the tool unchanged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AskAnswer {
    /// One of an approval's options, by its id.
    Choose {
        /// [`AskOption::id`].
        option: String,
    },
    /// A question's answers.
    Reply {
        /// One per part answered.
        parts: Vec<PartAnswer>,
    },
}

/// One model a tool offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelChoice {
    /// What `set_model` takes.
    pub id: String,
    /// Its name, as the tool shows it.
    pub name: String,
    /// The tool's default.
    pub default: bool,
}

/// What a tool's item is (for the conversation view; never stored).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// The user's message.
    UserMessage,
    /// The agent's message.
    AgentMessage,
    /// A command it ran.
    Command,
    /// Files it changed.
    FileChange,
    /// A tool call.
    Tool,
    /// Anything else.
    Other,
}

/// One finished item of a turn, in a line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemSummary {
    /// What it is.
    pub kind: ItemKind,
    /// Its text, bounded.
    pub text: String,
}

/// Why something went wrong, in the categories ADR-0003 point 3 needs (a
/// quota or sign-in stop is never restarted like a crash).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// A usage or rate limit.
    RateLimit,
    /// Billing or credits.
    Billing,
    /// Sign-in.
    SignIn,
    /// The tool or its server crashed or stopped answering.
    Crash,
    /// Anything else.
    Other,
}

/// What a driven session says, in neutral terms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum RuntimeEvent {
    /// The conversation is ready: the tool's own id for it (what a resume
    /// takes) and its model.
    Started {
        /// The tool's id.
        native_id: String,
        /// The model, when the tool says.
        model: Option<String>,
    },
    /// Its state changed (live.rs's vocabulary: busy, idle, waiting).
    State {
        /// The new state.
        status: LiveStatusWord,
    },
    /// A turn started.
    TurnStarted {
        /// The tool's id for the turn, when it has one.
        turn: Option<String>,
        /// Who started it, when AI Juggler did.
        origin: Option<TurnOrigin>,
    },
    /// A turn ended: the exact turn boundary.
    TurnEnded {
        /// How.
        outcome: TurnEnd,
        /// Its tokens, when the tool says.
        usage: Option<TurnUsage>,
    },
    /// Text the agent is writing (bounded).
    TextDelta {
        /// The new text.
        text: String,
    },
    /// Output of a tool or command it runs (bounded).
    ToolDelta {
        /// The tool's id for the item.
        item: String,
        /// The new output.
        text: String,
    },
    /// An item finished.
    Item {
        /// What it was.
        item: ItemSummary,
    },
    /// The tool asks to do something; only the user answers it.
    ApprovalRequested {
        /// The ask.
        approval: Approval,
    },
    /// The tool asks the user something.
    QuestionAsked {
        /// The question.
        question: Question,
    },
    /// An ask is no longer open (answered here, in an attached terminal,
    /// or withdrawn by the tool).
    AskResolved {
        /// Which.
        id: AskId,
    },
    /// The account's limits, as the tool reported them.
    LimitsUpdated {
        /// The snapshot.
        limits: LimitsSnapshot,
    },
    /// The session's running token total.
    Usage {
        /// The total.
        total: TurnUsage,
    },
    /// The conversation was compacted.
    Compacted,
    /// The model changed (from the next turn, for a tool that takes the
    /// model per turn).
    ModelChanged {
        /// The new model.
        model: String,
    },
    /// The models the tool offers, as it lists them.
    Models {
        /// In the tool's order.
        models: Vec<ModelChoice>,
    },
    /// Something went wrong.
    Error {
        /// Its category.
        category: ErrorCategory,
        /// The tool's words (bounded).
        message: String,
        /// The tool retries on its own.
        will_retry: bool,
    },
    /// The tool's process or connection is gone.
    Exited,
}

/// [`LiveStatus`] in a form events carry (serializable, no `Unknown`
/// guess: a tool that didn't say sends no state event).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveStatusWord {
    /// A turn is running.
    Busy,
    /// Waiting for the user's next turn.
    Idle,
    /// Waiting on the user's answer (an approval, a question).
    Waiting,
}

impl From<LiveStatusWord> for LiveStatus {
    fn from(word: LiveStatusWord) -> Self {
        match word {
            LiveStatusWord::Busy => Self::Busy,
            LiveStatusWord::Idle => Self::Idle,
            LiveStatusWord::Waiting => Self::Waiting,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_bounded_on_a_character() {
        let long = "é".repeat(TEXT_MAX);
        let cut = bounded(&long);
        assert!(cut.len() <= TEXT_MAX + "…".len());
        assert!(cut.ends_with('…'));
        assert_eq!(bounded("short"), "short");
    }

    #[test]
    fn events_round_trip_as_tagged_json() {
        let event = RuntimeEvent::ApprovalRequested {
            approval: Approval {
                id: AskId("7".into()),
                kind: ApprovalKind::Command,
                summary: "npm test".into(),
                reason: None,
                options: vec![AskOption {
                    id: "accept".into(),
                    label: "Allow once".into(),
                    effect: OptionEffect::AllowOnce,
                }],
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""event":"approval_requested""#), "{json}");
        assert_eq!(serde_json::from_str::<RuntimeEvent>(&json).unwrap(), event);
        let answer = AskAnswer::Reply {
            parts: vec![PartAnswer {
                part: "db".into(),
                answers: vec!["Postgres".into()],
            }],
        };
        let json = serde_json::to_string(&answer).unwrap();
        assert_eq!(serde_json::from_str::<AskAnswer>(&json).unwrap(), answer);
    }
}
