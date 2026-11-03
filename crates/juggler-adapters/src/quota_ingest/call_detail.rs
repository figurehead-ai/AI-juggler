//! What one API call says beyond its token classes (0.3.6, the usage
//! audit): Claude's 1-hour cache writes, prompt size, speed and inference
//! place, and which kind of thread a Codex record belongs to and the
//! conversation that started it.

use juggler_core::model::TokenCounts;
use juggler_core::quota::{CallKind, UsageDetail};

use super::RolloutCarry;

/// One Claude response's facts beyond its classes (0.3.6, the usage
/// audit): the cache writes that went to the 1-hour cache
/// (`cache_creation.ephemeral_1h_input_tokens`, priced at 2× base input
/// against 1.25× for the 5-minute ones), the whole prompt (input, cache
/// reads and writes: what Claude Haiku 5.5's price tier counts), the speed
/// and inference place when they aren't the default, and whether a
/// subagent made it (`isSidechain`). Its session is its conversation.
pub(super) fn claude_detail(
    value: &serde_json::Value,
    usage: &serde_json::Value,
    tokens: &TokenCounts,
) -> UsageDetail {
    let word = |key: &str, default: &str| {
        usage
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|word| *word != default && !word.is_empty() && word.len() <= 16)
            .map(str::to_owned)
    };
    UsageDetail {
        cache_write_1h: usage
            .get("cache_creation")
            .and_then(|split| split.get("ephemeral_1h_input_tokens"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default()
            .min(tokens.cache_write),
        prompt_tokens: tokens
            .input
            .saturating_add(tokens.cache_read)
            .saturating_add(tokens.cache_write),
        speed: word("speed", "standard"),
        geo: word("inference_geo", "global"),
        kind: if value
            .get("isSidechain")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            CallKind::Subagent
        } else {
            CallKind::Main
        },
        root_session: None,
    }
}

/// What a Codex thread is, as its `session_meta` says (0.3.6, the usage
/// audit: 2,285 of 2,779 "sessions" on one Mac were automatic reviews).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum ThreadKind {
    /// A conversation (`source` is `cli`, `vscode`, `exec`, …).
    #[default]
    Conversation,
    /// A subagent a conversation spawned (`subagent.thread_spawn`).
    Subagent,
    /// An automatic review a conversation ran (`subagent.other: guardian`,
    /// or `subagent: review`).
    Review,
}

impl RolloutCarry {
    /// Learns the file's thread from its first `session_meta`.
    pub(super) fn meta(&mut self, payload: &serde_json::Value) {
        if self.thread.is_some() {
            return; // a copied parent's meta later in a fork's file
        }
        let Some(id) = payload.get("id").and_then(serde_json::Value::as_str) else {
            return;
        };
        let subagent = payload
            .get("source")
            .and_then(|source| source.get("subagent"));
        self.kind = match subagent {
            None => ThreadKind::Conversation,
            Some(serde_json::Value::String(word)) if word == "review" => ThreadKind::Review,
            Some(serde_json::Value::Object(map))
                if map.get("other").and_then(serde_json::Value::as_str) == Some("guardian") =>
            {
                ThreadKind::Review
            }
            Some(_) => ThreadKind::Subagent,
        };
        let text = |key: &str| {
            payload
                .get(key)
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty() && *value != id)
                .map(str::to_owned)
        };
        self.root = text("session_id").or_else(|| text("parent_thread_id"));
        self.thread = Some(id.to_owned());
    }

    /// A record's kind and conversation, by the thread it names: the
    /// file's own thread is what its meta says; another one is a record
    /// copied from the history a forked thread opens with (the
    /// conversation's own, or another subagent's).
    pub(super) fn thread_of(&self, thread: &str) -> (CallKind, Option<String>) {
        let root = self.root.clone().filter(|root| root != thread);
        if self.thread.as_deref() == Some(thread) {
            let kind = match self.kind {
                ThreadKind::Conversation => CallKind::Main,
                ThreadKind::Subagent => CallKind::Subagent,
                ThreadKind::Review => CallKind::Review,
            };
            return (kind, root);
        }
        match root {
            None => (CallKind::Main, None),
            Some(root) => (CallKind::Subagent, Some(root)),
        }
    }
}
