//! AJ-E1520 (0.3): one subagent's own page — what it was given, what it
//! answered, what it did, what it used and cost, the processes it
//! probably started, and how to stop it.
//!
//! Its words are redacted (D-14); its prompt, result and the message that
//! started it only while last messages are on. Its tokens are a split
//! of the session's (a share, never on top); its cost an estimate from
//! the price list. Its processes are the Processes view's own rows that
//! view matched to it ("probably": a subagent has no process of its
//! own). Nothing here stops anything: where its tool has a way, the page
//! says it.

use std::sync::Arc;

use juggler_adapter::subagents::{SubagentSupport, SubagentTokens};
use juggler_core::model::TokenCounts;

use super::dto::{
    SubagentCrumbInfo, SubagentDetailInfo, SubagentLinesInfo, SubagentProcessInfo,
    SubagentToolsInfo,
};
use super::view::{Found, Tree, counted_word, share};
use super::{Missing, TAB_BUDGET, found};
use crate::commands::TokenCountsInfo;
use crate::error::AppResult;
use crate::features::conversations::last_messages;
use crate::state::AppState;

/// The most of its subagents listed on its page.
const CHILDREN_SHOWN: usize = 50;

/// What a subagent's page needs from elsewhere: prices and processes.
pub struct Around<'a> {
    /// An estimate of `tokens` on `model` at `at_ms` (USD).
    pub price: &'a dyn Fn(&str, i64, SubagentTokens) -> Option<f64>,
    /// The processes it probably started.
    pub processes: Vec<SubagentProcessInfo>,
    /// The Processes view's id for the session.
    pub process_session: Option<String>,
    /// Last messages are on.
    pub messages: bool,
}

/// `account`'s conversation `id`'s subagent `subagent`, on its own page.
///
/// # Errors
///
/// None today; the answer says what it couldn't read.
pub fn detail_impl(
    state: &Arc<AppState>,
    account: &str,
    id: &str,
    subagent: &str,
) -> AppResult<SubagentDetailInfo> {
    let found = match found(state, account, id, TAB_BUDGET) {
        Ok(found) => found,
        Err(missing) => return Ok(missing_detail(missing)),
    };
    let catalog = state
        .storage
        .lock()
        .ok()
        .map(|storage| crate::commands::pricing_catalog(&storage));
    let price = |model: &str, at: i64, tokens: SubagentTokens| {
        catalog.as_ref()?.price_tokens(
            model,
            at,
            TokenCounts {
                input: tokens.input,
                output: tokens.output,
                cache_read: tokens.cache_read,
                cache_write: tokens.cache_write,
                reasoning: 0,
            },
        )
    };
    let process_session = process_session(state, account, id);
    let processes = process_session
        .as_deref()
        .map(|session| processes_of(state, session, subagent))
        .unwrap_or_default();
    Ok(detail_of(
        &found,
        subagent,
        &Around {
            price: &price,
            processes,
            process_session,
            messages: last_messages::enabled(state),
        },
    ))
}

/// The page for `subagent` from what was found.
#[must_use]
#[allow(clippy::too_many_lines)] // one page's fields, each from its own part
pub fn detail_of(found: &Found, subagent: &str, around: &Around<'_>) -> SubagentDetailInfo {
    let tree = Tree::new(found);
    let Some(&n) = tree.by_id.get(subagent) else {
        return missing_detail(Missing::None);
    };
    let conversations = !matches!(
        found.support,
        SubagentSupport::Supported {
            conversations: juggler_adapter::subagents::SubagentConversations::NotRead(_),
            ..
        }
    );
    let node = &tree.nodes[n];
    let agent = node.agent;
    let d = &agent.details;
    let clean = &node.clean;
    let mut chain: Vec<SubagentCrumbInfo> = tree
        .ancestors(n)
        .into_iter()
        .map(|up| SubagentCrumbInfo {
            id: tree.nodes[up].agent.id.clone(),
            title: tree.nodes[up]
                .clean
                .task
                .clone()
                .or_else(|| tree.nodes[up].agent.kind.clone()),
        })
        .collect();
    chain.reverse();
    // The message that started it: its own, else its first ancestor's.
    let root = tree.ancestors(n).last().copied().unwrap_or(n);
    let mut kids: Vec<usize> = node.children.clone();
    kids.sort_by_key(|&k| std::cmp::Reverse(tree.nodes[k].agent.started_ms));
    let all_tokens: u64 = tree
        .nodes
        .iter()
        .filter_map(|n| n.own)
        .fold(0, u64::saturating_add);
    let whole = found
        .session_tokens
        .map(|session| session.total().saturating_add(all_tokens));
    let at = agent.started_ms.unwrap_or(found.now_ms);
    let cost = match (d.model.as_deref(), d.tokens) {
        (Some(model), Some(tokens)) => (around.price)(model, at, tokens),
        _ => None,
    };
    SubagentDetailInfo {
        state: "shown".to_owned(),
        reason: None,
        agent: Some(tree.info(n, conversations)),
        chain,
        prompt: clean.prompt.clone().filter(|_| around.messages),
        result: clean.result.clone().filter(|_| around.messages),
        failure: clean.failure.clone().filter(|_| around.messages),
        messages_off: !around.messages,
        asked: tree.nodes[root]
            .clean
            .asked
            .clone()
            .filter(|_| around.messages),
        tools: d.tools.map(|t| SubagentToolsInfo {
            total: t.total,
            reads: t.reads,
            searches: t.searches,
            commands: t.commands,
            edits: t.edits,
            other: t.other,
        }),
        lines: d.lines.map(|l| SubagentLinesInfo {
            added: l.added,
            removed: l.removed,
        }),
        files: clean.files.clone(),
        files_total: d.files_total,
        children: kids
            .iter()
            .take(CHILDREN_SHOWN)
            .map(|&k| tree.info(k, conversations))
            .collect(),
        children_total: u32::try_from(kids.len()).unwrap_or(u32::MAX),
        tokens: d.tokens.map(|t| TokenCountsInfo {
            input: t.input,
            output: t.output,
            cache_read: t.cache_read,
            cache_write: t.cache_write,
            reasoning: 0,
        }),
        counted: counted_word(found.counted).to_owned(),
        cost_usd: cost,
        share_pct: match (node.subtree, whole) {
            (Some(part), Some(whole)) => share(part, whole),
            _ => None,
        },
        processes: around.processes.clone(),
        process_session: around.process_session.clone(),
        commands: u32::try_from(d.commands.len()).unwrap_or(u32::MAX),
        stop_hint: match found.support {
            SubagentSupport::Supported { stop_hint, .. } => stop_hint.map(str::to_owned),
            _ => None,
        },
        worktree: d.worktree.clone(),
        branch: d.branch.clone(),
        stopped_by_user: d.stopped_by_user,
        tool: found.tool.clone(),
    }
}

/// The page when nothing can be shown.
#[must_use]
pub fn missing_detail(missing: Missing) -> SubagentDetailInfo {
    let (state, reason) = match missing {
        Missing::None => ("none", None),
        Missing::Unsupported(reason) => ("unsupported", Some(reason.to_owned())),
        Missing::Unreadable => (
            "unreadable",
            Some("Its record couldn't be read just now.".to_owned()),
        ),
    };
    SubagentDetailInfo {
        state: state.to_owned(),
        reason,
        agent: None,
        chain: Vec::new(),
        prompt: None,
        result: None,
        failure: None,
        messages_off: false,
        asked: None,
        tools: None,
        lines: None,
        files: Vec::new(),
        files_total: 0,
        children: Vec::new(),
        children_total: 0,
        tokens: None,
        counted: "none".to_owned(),
        cost_usd: None,
        share_pct: None,
        processes: Vec::new(),
        process_session: None,
        commands: 0,
        stop_hint: None,
        worktree: None,
        branch: None,
        stopped_by_user: None,
        tool: String::new(),
    }
}

/// The Processes view's id for `account`'s conversation `id`: AI
/// Juggler's session on it, else the conversation itself (a session
/// another app runs).
fn process_session(state: &Arc<AppState>, account: &str, id: &str) -> Option<String> {
    let storage = state.storage.lock().ok()?;
    let links = storage.sessions().conversation_links().ok()?;
    Some(
        links
            .into_iter()
            .filter(|(a, c, _)| a == account && c == id)
            .map(|(_, _, session)| session)
            .next_back()
            .unwrap_or_else(|| id.to_owned()),
    )
}

/// The processes the Processes view matched to `subagent` in `session`.
fn processes_of(state: &Arc<AppState>, session: &str, subagent: &str) -> Vec<SubagentProcessInfo> {
    let graph = state.processes.graph();
    let mut out = Vec::new();
    for app in &graph.apps {
        for node in app.sessions.iter().filter(|s| s.session_id == session) {
            for process in &node.processes {
                let Some((id, _)) = process.subagent.as_ref() else {
                    continue;
                };
                if id != subagent {
                    continue;
                }
                out.push(SubagentProcessInfo {
                    key: process.key.clone(),
                    pid: process.pid,
                    name: process.name.clone(),
                    command: process.label.clone().unwrap_or_default(),
                    rss_bytes: process.rss_bytes,
                    cpu_pct: process.cpu_pct,
                    alive: !matches!(
                        process.state,
                        crate::features::processes::model::ProcState::Ended
                    ),
                });
            }
        }
    }
    out
}
