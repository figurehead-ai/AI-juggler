//! The conversation feed's wire types (ADR-0024): a turn with its id,
//! what a page is given when it starts following, each change after, and
//! a page of earlier turns. Built from the adapters' turns, which the
//! feed's log already redacted.

use juggler_adapters::conversation_feed::{Block, FeedTurn, Layout, Speaker, Step, StepRun};

/// One tool call in a run of work: what a renderer needs to title and
/// draw it (redacted).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TurnCallInfo {
    /// The tool's name as the transcript gives it.
    pub tool: String,
    /// What it did in a few words (`ran "pnpm test"`, `Edit a.ts`).
    pub line: String,
    /// The call's own short description, when it has one.
    pub description: Option<String>,
    /// A command it ran.
    pub command: Option<String>,
    /// A file it read or wrote.
    pub file_path: Option<String>,
    /// A folder it looked in.
    pub path: Option<String>,
    /// What a search looked for.
    pub pattern: Option<String>,
    /// A web search's words.
    pub query: Option<String>,
    /// A subagent's kind.
    pub subagent_type: Option<String>,
    /// Its result came.
    pub done: bool,
    /// It failed.
    pub failed: bool,
    /// What it did to a file (`create`, `update`), when it says.
    pub result_kind: Option<String>,
    /// When it was made (epoch ms).
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
}

/// A part of a turn.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TurnBlockInfo {
    /// `text` (words), `narration` (thinking its tool shows like words)
    /// or `steps` (a run of work: tool calls and the thinking among them).
    pub kind: String,
    /// The words (markdown as written), redacted; empty for `steps`.
    pub text: String,
    /// One line per call (`ran "pnpm test"`, `Edit a.ts`), redacted;
    /// empty unless `steps`.
    pub steps: Vec<String>,
    /// Earlier calls in this run not kept.
    pub hidden: u32,
    /// The calls, one per line of `steps`.
    pub calls: Vec<TurnCallInfo>,
    /// What the model thought in this run, when the transcript keeps it.
    pub thoughts: Vec<String>,
    /// The run's title as the tool's own app writes it, when its app
    /// draws one (`Ran 3 commands`, `Recorded the case`).
    pub title: Option<String>,
    /// The same while the run is going (`Recording the case`).
    pub title_running: Option<String>,
    /// It began right after the model thought.
    pub after_thinking: bool,
    /// The message it began with.
    pub message_id: Option<String>,
    /// When it was last written (epoch ms).
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
}

/// One turn, redacted, with its id.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TurnInfo {
    /// `"{generation}:{offset}"`: the same in every read of this file.
    pub id: String,
    /// Grows each time the turn is written to.
    pub revision: u32,
    /// Where it starts in its transcript (pages back from here).
    #[ts(type = "number")]
    pub offset: u64,
    /// The last turn, and nothing has ended it: it may still grow.
    pub open: bool,
    /// `you` or `assistant`.
    pub speaker: String,
    /// Its parts, in order.
    pub blocks: Vec<TurnBlockInfo>,
    /// Earlier parts of a very long turn not kept.
    pub blocks_hidden: u32,
    /// When its last message was written (epoch ms), when known.
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
    /// The model an assistant turn ran on, when the transcript says.
    pub model: Option<String>,
    /// Keys or tokens taken out of it.
    pub redacted: u32,
}

/// What a page is given when it starts following a conversation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ConversationSnapshotInfo {
    /// `shown`; or why there's nothing: `off` (the setting), `none` (no
    /// transcript on this Mac), `compressed` (a finished rollout stored
    /// compressed), `unreadable`.
    pub state: String,
    /// What to unfollow with; changes come on the page's channel under
    /// it. `null` unless `shown`.
    #[ts(type = "number | null")]
    pub subscription: Option<u64>,
    /// The transcript read: a change of file or a rewrite is another.
    pub generation: u32,
    /// The feed's revision now; each change after is one more.
    #[ts(type = "number")]
    pub revision: u64,
    /// `stretches` (every text, each run of work one titled line: the
    /// Claude app) or `folded` (a turn's work in one row: `ChatGPT`'s Codex).
    pub layout: String,
    /// The latest turns, oldest first.
    pub turns: Vec<TurnInfo>,
    /// Where they start: read before it for earlier ones; `null` at the
    /// conversation's start.
    #[ts(type = "number | null")]
    pub earlier: Option<u64>,
    /// `ended` (stored compressed now: no more changes), `gone` (its
    /// file is gone) or `unreadable` (tried again): said over the turns.
    pub status: Option<String>,
    /// Lines too long to read (a huge tool output) passed over.
    pub skipped: u32,
}

/// A change to a followed conversation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ConversationDeltaInfo {
    /// The follow it's for.
    #[ts(type = "number")]
    pub subscription: u64,
    /// The transcript read (a `reset` brings another).
    pub generation: u32,
    /// One more than the last change: a gap means one was missed.
    #[ts(type = "number")]
    pub revision: u64,
    /// `append` (turns new or grown), `reset` (the conversation read
    /// again: these turns replace all) or `state` (its status changed).
    pub kind: String,
    /// The turns new or changed, oldest first (a `reset`: the latest).
    pub upserts: Vec<TurnInfo>,
    /// Turns before this offset were let go (the feed's bound).
    #[ts(type = "number | null")]
    pub dropped_before: Option<u64>,
    /// After a `reset`: where its turns start.
    #[ts(type = "number | null")]
    pub earlier: Option<u64>,
    /// As the snapshot's.
    pub layout: String,
    /// `ended`, `gone`, `unreadable`, `off`; `null`: read as usual.
    pub status: Option<String>,
    /// Lines passed over as too long, so far.
    pub skipped: u32,
}

/// A page of earlier turns.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct EarlierPageInfo {
    /// `shown`; `gone` (the follow ended: follow again); `unreadable`.
    pub state: String,
    /// The transcript read: turns of another are dropped.
    pub generation: u32,
    /// The turns, oldest first; none with `earlier` set means a long
    /// stretch with no turn was passed: read on from `earlier`.
    pub turns: Vec<TurnInfo>,
    /// Where these start; `null` at the conversation's start.
    #[ts(type = "number | null")]
    pub earlier: Option<u64>,
}

impl EarlierPageInfo {
    pub(crate) fn without(state: &str) -> Self {
        Self {
            state: state.to_owned(),
            generation: 0,
            turns: Vec::new(),
            earlier: None,
        }
    }
}

/// The wire name of a layout.
#[must_use]
pub fn layout_name(layout: Layout) -> &'static str {
    match layout {
        Layout::Stretches => "stretches",
        Layout::Folded => "folded",
    }
}

/// A turn on the wire.
#[must_use]
pub fn turn_info(turn: &FeedTurn, generation: u32, open: bool, layout: Layout) -> TurnInfo {
    TurnInfo {
        id: format!("{generation}:{}", turn.offset),
        revision: turn.revision,
        offset: turn.offset,
        open,
        speaker: match turn.speaker {
            Speaker::You => "you",
            Speaker::Assistant => "assistant",
        }
        .to_owned(),
        blocks: turn
            .blocks
            .iter()
            .map(|block| block_info(block, layout))
            .collect(),
        blocks_hidden: count(turn.blocks_hidden),
        at_ms: turn.at_ms,
        model: turn.model.clone(),
        redacted: turn.redacted,
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn block_info(block: &Block, layout: Layout) -> TurnBlockInfo {
    let empty = TurnBlockInfo {
        kind: String::new(),
        text: String::new(),
        steps: Vec::new(),
        hidden: 0,
        calls: Vec::new(),
        thoughts: Vec::new(),
        title: None,
        title_running: None,
        after_thinking: false,
        message_id: None,
        at_ms: None,
    };
    match block {
        Block::Text(text) | Block::Narration(text) => TurnBlockInfo {
            kind: if matches!(block, Block::Text(_)) {
                "text"
            } else {
                "narration"
            }
            .to_owned(),
            text: text.text(),
            after_thinking: text.after_thinking,
            message_id: text.message_id.clone(),
            at_ms: text.at_ms,
            ..empty
        },
        Block::Steps(run) => steps_info(run, layout, empty),
    }
}

fn steps_info(run: &StepRun, layout: Layout, empty: TurnBlockInfo) -> TurnBlockInfo {
    let titles = juggler_adapters::conversation_feed::fold_title(layout, run);
    TurnBlockInfo {
        kind: "steps".to_owned(),
        steps: run.steps.iter().map(|step| step.line.clone()).collect(),
        hidden: count(run.hidden),
        calls: run.steps.iter().map(call_info).collect(),
        thoughts: run.thoughts.clone(),
        at_ms: run.steps.last().and_then(|step| step.at_ms),
        title: titles.as_ref().map(|(done, _)| done.clone()),
        title_running: titles.map(|(_, going)| going),
        ..empty
    }
}

fn call_info(step: &Step) -> TurnCallInfo {
    let input = &step.input;
    TurnCallInfo {
        tool: step.tool.clone(),
        line: step.line.clone(),
        description: input.description.clone(),
        command: input.command.clone(),
        file_path: input.file_path.clone(),
        path: input.path.clone(),
        pattern: input.pattern.clone(),
        query: input.query.clone(),
        subagent_type: input.subagent_type.clone(),
        done: step.result.is_some(),
        failed: step.result.as_ref().is_some_and(|result| result.failed),
        result_kind: step.result.as_ref().and_then(|result| result.kind.clone()),
        at_ms: step.at_ms,
    }
}
