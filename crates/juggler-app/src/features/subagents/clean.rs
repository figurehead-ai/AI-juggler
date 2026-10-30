//! AJ-E1520 (0.3): each subagent's words as the tab shows and searches
//! them — redacted (D-14) once, and kept while they don't change, so a
//! session with a thousand subagents isn't redacted again on every page.
//! With last messages off, no prompt, result or message is kept or
//! searched: only the task, its type and its files.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use juggler_adapter::subagents::{Subagent, SubagentText};

/// One subagent's words, redacted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clean {
    /// What it was asked to do.
    pub task: Option<String>,
    /// The prompt it was given.
    pub prompt: Option<String>,
    /// The start of its answer.
    pub result: Option<String>,
    /// Why it failed.
    pub failure: Option<String>,
    /// The message its turn began with.
    pub asked: Option<String>,
    /// The files it edited.
    pub files: Vec<String>,
    /// Its commands, each with when it was asked for.
    pub commands: Vec<(Option<i64>, String)>,
    /// Everything searched, lower-cased: task, type, prompt, result,
    /// files.
    pub hay: String,
    /// Its files, lower-cased (`file:`).
    pub files_lc: String,
    /// Its type, lower-cased (`type:`).
    pub kind_lc: String,
}

/// `text` redacted (D-14).
#[must_use]
pub fn redact(text: &str) -> String {
    let (mut clean, _) =
        juggler_handover::redaction::strip_segments(&[text.to_owned()], true, &BTreeSet::new());
    clean.pop().unwrap_or_default()
}

/// What a checkpoint keeps of a text: redacted; a prompt, result or
/// message only while last messages are on.
#[must_use]
pub fn kept(kind: SubagentText, text: &str, messages: bool) -> Option<String> {
    match kind {
        SubagentText::Prompt | SubagentText::Result | SubagentText::Asked if !messages => None,
        _ => Some(redact(text)),
    }
}

/// The redacted words kept, by subagent.
#[derive(Debug, Default)]
pub struct Cleaned {
    messages: bool,
    by_id: HashMap<String, (u64, Arc<Clean>)>,
}

impl Cleaned {
    /// Each of `agents`' words, redacting only those that changed.
    pub fn all(&mut self, agents: &[Subagent], messages: bool) -> HashMap<String, Arc<Clean>> {
        if messages != self.messages {
            self.by_id.clear();
            self.messages = messages;
        }
        let mut out = HashMap::with_capacity(agents.len());
        for agent in agents {
            let print = fingerprint(agent);
            let clean = match self.by_id.get(&agent.id) {
                Some((kept, clean)) if *kept == print => Arc::clone(clean),
                _ => {
                    let clean = Arc::new(clean_of(agent, messages));
                    self.by_id
                        .insert(agent.id.clone(), (print, Arc::clone(&clean)));
                    clean
                }
            };
            out.insert(agent.id.clone(), clean);
        }
        // Gone ones go.
        if self.by_id.len() > out.len() {
            self.by_id.retain(|id, _| out.contains_key(id));
        }
        out
    }
}

/// `agent`'s words redacted (one call: every text at once).
#[must_use]
pub fn clean_of(agent: &Subagent, messages: bool) -> Clean {
    let d = &agent.details;
    let pick = |text: &Option<String>, message: bool| -> Option<String> {
        text.clone().filter(|_| messages || !message)
    };
    let fields = [
        pick(&agent.task, false),
        pick(&d.prompt, true),
        pick(&d.result, true),
        pick(&d.failure, true),
        pick(&d.asked.as_ref().map(|asked| asked.text.clone()), true),
    ];
    let mut segments: Vec<String> = fields
        .iter()
        .map(|f| f.clone().unwrap_or_default())
        .collect();
    segments.extend(d.files.iter().cloned());
    segments.extend(d.commands.iter().map(|c| c.command.clone()));
    let (mut clean, _) =
        juggler_handover::redaction::strip_segments(&segments, true, &BTreeSet::new());
    let commands: Vec<String> = clean.split_off(fields.len() + d.files.len());
    let files: Vec<String> = clean.split_off(fields.len());
    let mut texts = clean
        .into_iter()
        .zip(fields.iter())
        .map(|(text, raw)| raw.as_ref().map(|_| text));
    let task = texts.next().flatten();
    let prompt = texts.next().flatten();
    let result = texts.next().flatten();
    let failure = texts.next().flatten();
    let asked = texts.next().flatten();
    let kind_lc = agent.kind.clone().unwrap_or_default().to_lowercase();
    let files_lc = files.join("\n").to_lowercase();
    let hay = [
        task.as_deref(),
        Some(kind_lc.as_str()),
        prompt.as_deref(),
        result.as_deref(),
        failure.as_deref(),
        Some(files_lc.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
    .to_lowercase();
    Clean {
        commands: d.commands.iter().map(|c| c.at_ms).zip(commands).collect(),
        task,
        prompt,
        result,
        failure,
        asked,
        files,
        hay,
        files_lc,
        kind_lc,
    }
}

/// A cheap print of every word a subagent has (FNV-1a over them).
fn fingerprint(agent: &Subagent) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |text: &str| {
        for byte in text.bytes().chain(std::iter::once(0)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    let d = &agent.details;
    for text in [
        agent.task.as_deref(),
        agent.kind.as_deref(),
        d.prompt.as_deref(),
        d.result.as_deref(),
        d.failure.as_deref(),
        d.asked.as_ref().map(|asked| asked.text.as_str()),
    ] {
        eat(text.unwrap_or_default());
    }
    for file in &d.files {
        eat(file);
    }
    for command in &d.commands {
        eat(&command.command);
    }
    hash
}
