//! The conversation feed (0.3.5, ADR-0024): a transcript's turns, built a
//! line at a time from whole lines and where each starts in the file.
//!
//! One builder, [`TurnLog`], serves every read of a conversation: the
//! page opening (fed forward from a turn boundary the backward scan
//! found, [`crate::last_turns::scan_back`]), following it as it grows
//! (fed what a [`crate::transcript_tail::TranscriptTail`] appends) and
//! "Show earlier" (another scan, the same builder). So turn boundaries
//! never disagree, and a turn's id, the offset of its first line, is the
//! same in every read: an appended tool call extends the open turn under
//! its id ([`FeedTurn::revision`] goes up) instead of making a new list.
//!
//! The rules, in one place: who spoke (`conversation_lines`);
//! consecutive messages of one speaker are one turn, its parts in the
//! order they came; a line the tool put in the conversation (a task's
//! notification, a command's output: a `<tag>` line of yours) ends the
//! turn before it; a message the person sent mid-turn is theirs and the
//! work after it a new turn; a subagent's lines are left out of a
//! session's conversation. A turn's parts are words, narration (thinking
//! the tool marks to be shown like words) and runs of work: the tool
//! calls between two texts, with the thinking among them, as one
//! [`StepRun`] (the Claude app's fold; its title from the tool's own
//! rules, [`fold_title`]).
//!
//! Bounded: a line longer than [`LINE_HOLD_MAX`] is passed over and
//! counted; text keeps its end ([`BLOCK_CHARS`]), a run its latest calls
//! ([`STEPS_MAX`]) and thoughts, a turn its latest blocks
//! ([`BLOCKS_MAX`]); the log keeps at most [`Limits`] (turns and bytes),
//! dropping its oldest turns first (they stay a "Show earlier" away).
//! With a [`Scrub`], text is redacted as it is read, so what the log
//! holds was never raw past one line.

use std::collections::{BTreeSet, HashSet, VecDeque};
use std::sync::Arc;

use serde_json::Value;

use crate::conversation_lines::{self, Call, CallResult, Message, Part, Said};
use crate::transcript::{TranscriptFormat, line_format};

/// The most characters one text block keeps (its end).
pub const BLOCK_CHARS: usize = 16_000;

/// The most characters one thought keeps.
pub const THOUGHT_CHARS: usize = 2_000;

/// The most thoughts one run keeps (the latest).
pub const THOUGHTS_MAX: usize = 8;

/// The most tool calls one run of them keeps (the latest).
pub const STEPS_MAX: usize = 40;

/// The most blocks one turn keeps (its latest).
pub const BLOCKS_MAX: usize = 200;

/// The longest line read; a longer one (a huge tool output or image) is
/// passed over and counted ([`TurnLog::skipped`]).
pub const LINE_HOLD_MAX: usize = 8 * 1024 * 1024;

/// How many line ids are remembered to read a repeated line once.
const SEEN_MAX: usize = 4_096;

/// How many turns back a call's result is looked for.
const RESULT_TURNS: usize = 3;

/// Who spoke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaker {
    /// The person (or, in a subagent's conversation, the session that
    /// asked it).
    You,
    /// The tool's model.
    Assistant,
}

/// Which lines are the conversation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lines {
    /// A session's: a subagent's lines are left out.
    #[default]
    Session,
    /// A subagent's own transcript: its lines are the conversation.
    Subagent,
}

/// How the tool's own app draws a turn, so the page can draw it alike
/// without knowing which tool it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Every text in full and each run of work as one titled line (the
    /// Claude app's Code tab).
    Stretches,
    /// A finished turn's work as one "Worked for" row, then its reply
    /// (`ChatGPT`'s Codex).
    Folded,
}

/// The inputs of a call a title needs (raw from the transcript, then
/// redacted).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallInput {
    /// The call's own short description (a command's, a subagent's task).
    pub description: Option<String>,
    /// A command.
    pub command: Option<String>,
    /// A file.
    pub file_path: Option<String>,
    /// A folder.
    pub path: Option<String>,
    /// What a search looks for.
    pub pattern: Option<String>,
    /// A web search's words.
    pub query: Option<String>,
    /// A subagent's kind.
    pub subagent_type: Option<String>,
    /// What a call does among its kinds (`left_click`), when it says: a
    /// plain identifier only, never words.
    pub action: Option<String>,
    /// A batch's actions in order, each its name and its own action
    /// (plain identifiers, else empty).
    pub batch: Vec<(String, String)>,
    /// It opens a page by its address alone (no server's name).
    pub url_without_name: bool,
}

impl CallInput {
    fn pieces_mut(&mut self) -> [&mut Option<String>; 7] {
        [
            &mut self.description,
            &mut self.command,
            &mut self.file_path,
            &mut self.path,
            &mut self.pattern,
            &mut self.query,
            &mut self.subagent_type,
        ]
    }

    fn bytes(&self) -> usize {
        [
            &self.description,
            &self.command,
            &self.file_path,
            &self.path,
            &self.pattern,
            &self.query,
            &self.subagent_type,
        ]
        .iter()
        .map(|piece| piece.as_ref().map_or(0, String::len))
        .sum::<usize>()
            + self.action.as_ref().map_or(0, String::len)
            + self
                .batch
                .iter()
                .map(|(name, action)| name.len() + action.len())
                .sum::<usize>()
    }
}

/// How a call ended, once its result came.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepResult {
    /// It failed.
    pub failed: bool,
    /// What it did to a file (`create`, `update`), when it says.
    pub kind: Option<String>,
}

/// One tool call in a run of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// What it did in a few words (`ran "pnpm test"`, `Edit a.ts`).
    pub line: String,
    /// The tool's name as the transcript gives it.
    pub tool: String,
    /// Its id (its result names it).
    pub call_id: Option<String>,
    /// The inputs a title needs.
    pub input: CallInput,
    /// When it was made (epoch ms), when the line says.
    pub at_ms: Option<i64>,
    /// How it ended; `None` while it runs (or its result was not read).
    pub result: Option<StepResult>,
}

/// Words, at most `max` characters (their end).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextBlock {
    kept: String,
    cut: bool,
    /// It began right after the model thought.
    pub after_thinking: bool,
    /// The id of the message it began with.
    pub message_id: Option<String>,
    /// When its latest words were written (epoch ms).
    pub at_ms: Option<i64>,
}

impl TextBlock {
    /// The words, after `…` when their start was cut.
    #[must_use]
    pub fn text(&self) -> String {
        if self.cut {
            format!("…{}", self.kept.trim_start())
        } else {
            self.kept.clone()
        }
    }

    fn push(&mut self, words: &str, max: usize) {
        if !self.kept.is_empty() {
            self.kept.push_str("\n\n");
        }
        self.kept.push_str(words);
        let count = self.kept.chars().count();
        if count > max {
            self.kept = self.kept.chars().skip(count - max).collect();
            self.cut = true;
        }
    }
}

/// A run of work between two texts: its tool calls and the thinking
/// among them (one fold line in the Claude app).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepRun {
    /// The calls, the latest [`STEPS_MAX`].
    pub steps: Vec<Step>,
    /// Earlier calls in this run not kept.
    pub hidden: usize,
    /// What the model thought in it, the latest [`THOUGHTS_MAX`] (words
    /// only when the transcript keeps them).
    pub thoughts: Vec<String>,
}

/// A part of a turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// Words.
    Text(TextBlock),
    /// Thinking the tool marks to be shown like words.
    Narration(TextBlock),
    /// A run of work.
    Steps(StepRun),
}

impl Block {
    fn bytes(&self) -> usize {
        match self {
            Self::Text(text) | Self::Narration(text) => text.kept.len(),
            Self::Steps(run) => {
                run.steps
                    .iter()
                    .map(|step| step.line.len() + step.tool.len() + step.input.bytes())
                    .sum::<usize>()
                    + run.thoughts.iter().map(String::len).sum::<usize>()
            }
        }
    }
}

/// One turn: who spoke, what they wrote and did, and where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedTurn {
    /// Who spoke.
    pub speaker: Speaker,
    /// Its parts, in order.
    pub blocks: Vec<Block>,
    /// Earlier blocks of a very long turn not kept.
    pub blocks_hidden: usize,
    /// When its last message was written (epoch ms), when known.
    pub at_ms: Option<i64>,
    /// The model an assistant turn ran on, when the transcript says.
    pub model: Option<String>,
    /// Where its first line starts in the file: its id.
    pub offset: u64,
    /// Where its last line ends.
    pub end: u64,
    /// How many times it was written to (an open turn grows).
    pub revision: u32,
    /// Keys or tokens taken out of it.
    pub redacted: u32,
    bytes: usize,
}

impl FeedTurn {
    /// The turn's words, its text blocks joined.
    #[must_use]
    pub fn text(&self) -> String {
        self.blocks
            .iter()
            .filter_map(|block| match block {
                Block::Text(text) => Some(text.text()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// What changed since the last [`TurnLog::take_delta`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TurnDelta {
    /// The turns written to (new, grown, or closed), oldest first.
    pub upserts: Vec<FeedTurn>,
    /// Turns before this offset were dropped (the log's bound).
    pub dropped_before: Option<u64>,
}

impl TurnDelta {
    /// Nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.upserts.is_empty() && self.dropped_before.is_none()
    }
}

/// How much a log keeps; past either, its oldest turns go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Turns.
    pub turns: usize,
    /// Bytes of text, thoughts and call inputs.
    pub bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            turns: 300,
            bytes: 4 * 1024 * 1024,
        }
    }
}

/// Takes keys and tokens out of pieces of text (the app's D-14
/// patterns).
pub trait Scrub: Send + Sync {
    /// Cleans `pieces` in place; how many it took out.
    fn scrub(&self, pieces: &mut [String]) -> u32;
}

/// A run's title as the tool's own app writes it: done, and while it
/// runs. `None` for a tool whose app draws no such line.
#[must_use]
pub fn fold_title(layout: Layout, run: &StepRun) -> Option<(String, String)> {
    match layout {
        Layout::Stretches => Some((
            crate::claude_folds::title(run, false),
            crate::claude_folds::title(run, true),
        )),
        Layout::Folded => None,
    }
}

/// A conversation's turns, built from whole lines (see the module docs).
#[derive(Clone, Default)]
pub struct TurnLog {
    lines: Lines,
    format: Option<TranscriptFormat>,
    turns: VecDeque<FeedTurn>,
    /// A line ended the last turn: the next message starts another.
    ended: bool,
    /// The model thought since the last words or call.
    thinking: bool,
    /// The model a rollout's last `turn_context` named.
    model: Option<String>,
    seen: VecDeque<String>,
    seen_set: HashSet<String>,
    changed: BTreeSet<u64>,
    dropped: Option<u64>,
    /// Turns were dropped from the front (bound, page or scan).
    trimmed: bool,
    /// Where the first line read starts.
    start: Option<u64>,
    /// Where the next bytes [`TailLog::feed`] gets start.
    next: u64,
    skipped: u32,
    limits: Limits,
    held: usize,
    scrub: Option<Arc<dyn Scrub>>,
}

impl std::fmt::Debug for TurnLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TurnLog")
            .field("lines", &self.lines)
            .field("turns", &self.turns.len())
            .field("start", &self.start)
            .field("skipped", &self.skipped)
            .finish_non_exhaustive()
    }
}

impl TurnLog {
    /// An empty log of `lines`.
    #[must_use]
    pub fn new(lines: Lines) -> Self {
        Self {
            lines,
            ..Self::default()
        }
    }

    /// The same, redacting what it reads with `scrub`.
    #[must_use]
    pub fn with_scrub(mut self, scrub: Arc<dyn Scrub>) -> Self {
        self.scrub = Some(scrub);
        self
    }

    /// The same, keeping at most `limits`.
    #[must_use]
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// A new empty log read the same way (its lines, limits and scrub).
    #[must_use]
    pub fn fresh(&self) -> Self {
        Self {
            lines: self.lines,
            limits: self.limits,
            scrub: self.scrub.clone(),
            ..Self::default()
        }
    }

    /// Which lines it reads.
    #[must_use]
    pub fn lines(&self) -> Lines {
        self.lines
    }

    /// How the tool's own app draws this conversation's turns (known
    /// once a line was read; Claude's until then).
    #[must_use]
    pub fn layout(&self) -> Layout {
        match self.format {
            Some(TranscriptFormat::CodexRollout) => Layout::Folded,
            _ => Layout::Stretches,
        }
    }

    /// The turns it keeps, oldest first.
    #[must_use]
    pub fn turns(&self) -> &VecDeque<FeedTurn> {
        &self.turns
    }

    /// Whether its last turn can still grow (no line has ended it).
    #[must_use]
    pub fn last_open(&self) -> bool {
        !self.ended && !self.turns.is_empty()
    }

    /// Where the turns it keeps start, when there may be earlier ones
    /// (read before it for them); `None` when it holds the
    /// conversation's start.
    #[must_use]
    pub fn earlier(&self) -> Option<u64> {
        if self.start.unwrap_or(0) == 0 && !self.trimmed {
            return None;
        }
        self.turns
            .front()
            .map(|turn| turn.offset)
            .or(self.start)
            .filter(|at| *at > 0)
    }

    /// Lines passed over as longer than [`LINE_HOLD_MAX`].
    #[must_use]
    pub fn skipped(&self) -> u32 {
        self.skipped
    }

    /// The turns written to since the last call, and where turns were
    /// dropped.
    pub fn take_delta(&mut self) -> TurnDelta {
        let changed = std::mem::take(&mut self.changed);
        TurnDelta {
            upserts: self
                .turns
                .iter()
                .filter(|turn| changed.contains(&turn.offset))
                .cloned()
                .collect(),
            dropped_before: self.dropped.take(),
        }
    }

    /// Notes that reading starts at `at` (before any line is fed): a
    /// read that finds no whole line there still knows where it began.
    pub fn begin_at(&mut self, at: u64) {
        self.start.get_or_insert(at);
    }

    /// Forgets what changed: what it holds now is what the page was
    /// given (a snapshot), not news.
    pub fn settle(&mut self) {
        self.changed.clear();
        self.dropped = None;
    }

    /// Drops its first turn: one a read began in the middle of.
    pub fn drop_first(&mut self) {
        if let Some(turn) = self.turns.pop_front() {
            self.held = self.held.saturating_sub(turn.bytes);
            self.changed.remove(&turn.offset);
            self.trimmed = true;
            self.dropped = self.turns.front().map(|first| first.offset);
        }
    }

    /// Keeps only its last `count` turns.
    pub fn keep_last(&mut self, count: usize) {
        while self.turns.len() > count {
            self.drop_first();
        }
    }

    /// One whole line (without its `\n`) that starts at `at` and ends at
    /// `end` (after its `\n`).
    pub fn line(&mut self, at: u64, end: u64, bytes: &[u8]) {
        if bytes.len() > LINE_HOLD_MAX {
            self.skipped += 1;
            return;
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
            return;
        };
        let format = *self.format.get_or_insert_with(|| line_format(&value));
        let said = match format {
            TranscriptFormat::ClaudeCode => conversation_lines::claude(&value),
            TranscriptFormat::CodexRollout => {
                conversation_lines::codex(&value, self.model.as_deref())
            }
        };
        match said {
            Said::Nothing => {}
            Said::Model(model) => self.model = Some(model),
            Said::Message(message) => self.message(at, end, message),
            Said::Results { results, sidechain } => {
                if !(sidechain && self.lines == Lines::Session) {
                    self.results(&results);
                }
            }
        }
    }

    fn message(&mut self, at: u64, end: u64, message: Message) {
        if message.sidechain && self.lines == Lines::Session {
            return;
        }
        if let Some(uuid) = &message.uuid
            && !self.first_sight(uuid)
        {
            return;
        }
        // A tool's own line in the conversation (`<command-name>`, a
        // task's notification) isn't shown, but ends the turn before it.
        if message.speaker == Speaker::You && message.text().starts_with('<') {
            if !self.ended
                && let Some(last) = self.turns.back()
            {
                // It closes: said again so the page knows.
                self.changed.insert(last.offset);
            }
            self.ended = true;
            return;
        }
        // Thinking kept in no words (a rollout's reasoning) is a mark only.
        let shown = message.parts.iter().any(|part| match part {
            Part::Thought { text, .. } => !text.is_empty(),
            Part::Text(_) | Part::Call(_) => true,
        });
        if !shown {
            self.thinking = true;
            return;
        }
        let Message {
            speaker,
            parts,
            message_id,
            model,
            at_ms,
            ..
        } = message;
        let (parts, redacted) = self.scrubbed(parts);
        let new_turn = std::mem::take(&mut self.ended)
            || self.turns.back().is_none_or(|last| last.speaker != speaker);
        if new_turn {
            if let Some(last) = self.turns.back() {
                self.changed.insert(last.offset);
            }
            self.turns.push_back(FeedTurn {
                speaker,
                blocks: Vec::new(),
                blocks_hidden: 0,
                at_ms: None,
                model: None,
                offset: at,
                end,
                revision: 0,
                redacted: 0,
                bytes: 0,
            });
        }
        let Some(turn) = self.turns.back_mut() else {
            return;
        };
        turn.end = end;
        turn.at_ms = at_ms.or(turn.at_ms);
        if model.is_some() {
            turn.model = model;
        }
        let said = Origin { message_id, at_ms };
        for part in parts {
            match part {
                Part::Text(text) => {
                    push_text(turn, &text, self.thinking, &said);
                    self.thinking = false;
                }
                Part::Thought { text, narration } => {
                    if narration && !text.is_empty() {
                        push_narration(turn, &text, &said);
                    } else if !text.is_empty() {
                        push_thought(turn, &text);
                    }
                    self.thinking = true;
                }
                Part::Call(call) => {
                    push_step(turn, call, at_ms);
                    self.thinking = false;
                }
            }
        }
        if turn.blocks.len() > BLOCKS_MAX {
            let cut = turn.blocks.len() - BLOCKS_MAX;
            turn.blocks.drain(..cut);
            turn.blocks_hidden += cut;
        }
        turn.redacted = turn.redacted.saturating_add(redacted);
        self.touched_last();
    }

    /// The calls `results` answer, marked done (looked for in the last
    /// few turns, where their calls are).
    fn results(&mut self, results: &[CallResult]) {
        for result in results {
            let found =
                self.turns
                    .iter_mut()
                    .rev()
                    .take(RESULT_TURNS)
                    .find_map(|turn| {
                        let step = turn.blocks.iter_mut().rev().find_map(|block| match block {
                            Block::Steps(run) => run.steps.iter_mut().rev().find(|step| {
                                step.call_id.as_deref() == Some(result.call_id.as_str())
                            }),
                            _ => None,
                        })?;
                        step.result = Some(StepResult {
                            failed: result.failed,
                            kind: result.kind.clone(),
                        });
                        turn.revision = turn.revision.saturating_add(1);
                        Some(turn.offset)
                    });
            if let Some(offset) = found {
                self.changed.insert(offset);
            }
        }
    }

    /// The last turn was written to: its revision, its size, the bound.
    fn touched_last(&mut self) {
        let Some(turn) = self.turns.back_mut() else {
            return;
        };
        turn.revision = turn.revision.saturating_add(1);
        let bytes: usize = turn.blocks.iter().map(Block::bytes).sum();
        self.held = self.held.saturating_sub(turn.bytes) + bytes;
        turn.bytes = bytes;
        self.changed.insert(turn.offset);
        while self.turns.len() > 1
            && (self.turns.len() > self.limits.turns || self.held > self.limits.bytes)
        {
            self.drop_first();
        }
    }

    /// `parts`, each cut to what a block keeps and redacted when the log
    /// scrubs; how many keys or tokens were taken out.
    fn scrubbed(&self, parts: Vec<Part>) -> (Vec<Part>, u32) {
        // A margin past what a block keeps: a key at the cut stays whole
        // enough to be found.
        let mut parts: Vec<Part> = parts
            .into_iter()
            .map(|part| match part {
                Part::Text(text) => Part::Text(end_of(&text, BLOCK_CHARS + 1_024)),
                Part::Thought { text, narration } => Part::Thought {
                    text: end_of(&text, THOUGHT_CHARS + 1_024),
                    narration,
                },
                Part::Call(call) => Part::Call(call),
            })
            .collect();
        let Some(scrub) = &self.scrub else {
            return (parts, 0);
        };
        let mut pieces: Vec<String> = Vec::new();
        for part in &mut parts {
            match part {
                Part::Text(text) | Part::Thought { text, .. } => pieces.push(std::mem::take(text)),
                Part::Call(call) => {
                    pieces.push(std::mem::take(&mut call.line));
                    for piece in call.input.pieces_mut() {
                        pieces.push(piece.take().unwrap_or_default());
                    }
                }
            }
        }
        let redacted = scrub.scrub(&mut pieces);
        let mut pieces = pieces.into_iter();
        let mut next = || pieces.next().unwrap_or_default();
        for part in &mut parts {
            match part {
                Part::Text(text) | Part::Thought { text, .. } => *text = next(),
                Part::Call(call) => {
                    call.line = next();
                    for piece in call.input.pieces_mut() {
                        let words = next();
                        *piece = (!words.is_empty()).then_some(words);
                    }
                }
            }
        }
        (parts, redacted)
    }

    /// Whether the line id `uuid` is new (it's remembered, bounded).
    fn first_sight(&mut self, uuid: &str) -> bool {
        if self.seen_set.contains(uuid) {
            return false;
        }
        if self.seen.len() >= SEEN_MAX
            && let Some(old) = self.seen.pop_front()
        {
            self.seen_set.remove(&old);
        }
        self.seen.push_back(uuid.to_owned());
        self.seen_set.insert(uuid.to_owned());
        true
    }
}

/// Where a message's words came from: its id and time.
struct Origin {
    message_id: Option<String>,
    at_ms: Option<i64>,
}

/// `text`'s end, at most `max` characters.
fn end_of(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_owned();
    }
    text.chars().skip(count - max).collect()
}

fn push_text(turn: &mut FeedTurn, text: &str, thought: bool, said: &Origin) {
    if text.is_empty() {
        return;
    }
    if let Some(Block::Text(last)) = turn.blocks.last_mut() {
        last.push(text, BLOCK_CHARS);
        last.at_ms = said.at_ms.or(last.at_ms);
        return;
    }
    let mut block = TextBlock {
        after_thinking: thought,
        message_id: said.message_id.clone(),
        at_ms: said.at_ms,
        ..TextBlock::default()
    };
    block.push(text, BLOCK_CHARS);
    turn.blocks.push(Block::Text(block));
}

fn push_narration(turn: &mut FeedTurn, text: &str, said: &Origin) {
    let mut block = TextBlock {
        after_thinking: true,
        message_id: said.message_id.clone(),
        at_ms: said.at_ms,
        ..TextBlock::default()
    };
    block.push(text, BLOCK_CHARS);
    turn.blocks.push(Block::Narration(block));
}

/// The run of work the next call or thought joins: the last block when
/// it is one, else a new one.
fn run_of(turn: &mut FeedTurn) -> Option<&mut StepRun> {
    if !matches!(turn.blocks.last(), Some(Block::Steps(_))) {
        turn.blocks.push(Block::Steps(StepRun::default()));
    }
    match turn.blocks.last_mut() {
        Some(Block::Steps(run)) => Some(run),
        _ => None,
    }
}

fn push_thought(turn: &mut FeedTurn, text: &str) {
    let Some(run) = run_of(turn) else {
        return;
    };
    run.thoughts.push(end_of(text, THOUGHT_CHARS));
    let cut = run.thoughts.len().saturating_sub(THOUGHTS_MAX);
    run.thoughts.drain(..cut);
}

fn push_step(turn: &mut FeedTurn, call: Call, at_ms: Option<i64>) {
    let Some(run) = run_of(turn) else {
        return;
    };
    run.steps.push(Step {
        line: call.line,
        tool: call.tool,
        call_id: call.call_id,
        input: call.input,
        at_ms,
        result: None,
    });
    let cut = run.steps.len().saturating_sub(STEPS_MAX);
    run.steps.drain(..cut);
    run.hidden += cut;
}

impl crate::transcript_tail::TailLog for TurnLog {
    const LINE_MAX: usize = LINE_HOLD_MAX;

    fn feed(&mut self, bytes: &[u8]) {
        let at = self.next;
        self.feed_at(bytes, at);
    }

    fn feed_at(&mut self, bytes: &[u8], base: u64) {
        self.start.get_or_insert(base);
        let mut at = base;
        for line in bytes.split(|b| *b == b'\n') {
            let end = at + line.len() as u64 + 1;
            if !line.is_empty() {
                self.line(at, end, line);
            }
            at = end;
        }
        self.next = base + bytes.len() as u64;
    }

    fn skipped_line(&mut self, at: u64) {
        self.start.get_or_insert(at);
        self.skipped += 1;
    }
}

#[cfg(test)]
#[path = "conversation_feed_tests.rs"]
mod tests;
