//! AJ-E1520 (0.3): the Subagents tab's rows — what a session's (or a
//! project's) subagents look like for one query: grouped under the
//! user's message that started them, each group a tree of who started
//! whom (real tree lines), filtered by a view (Running, Needs attention,
//! All), a search and a type, in the order asked (newest, most tokens,
//! longest), paged.
//!
//! Pure: it takes what the readers listed and the words already redacted
//! (`clean.rs`), and reads nothing. A parent's tokens are its own and
//! its children's together (its own are said apart). When a filter is on,
//! a match's ancestors are shown too, dimmed (`context`), so the tree
//! still says who started it. Tokens are a split of the session's: their
//! share is of the session's own and theirs together, never added on top.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use juggler_adapter::subagents::{
    Subagent, SubagentStatus, SubagentSupport, SubagentTokens, TokensCounted,
};

use super::clean::Clean;
use super::dto::{
    SubagentGroupInfo, SubagentInfo, SubagentKindInfo, SubagentRowInfo, SubagentSessionInfo,
    SubagentsInfo, SubagentsQuery, SubagentsSummaryInfo,
};
use crate::error::AppResult;
use crate::paging::{PageRequest, SortKey};

/// Deeper than this, a loop in the records is assumed.
const DEEPEST: usize = 32;

/// What the readers found, ready to show.
pub struct Found {
    /// What its tool's adapter answered.
    pub support: SubagentSupport,
    /// The session is open now.
    pub live: bool,
    /// A long session is still being read.
    pub reading: bool,
    /// Every subagent read.
    pub agents: Vec<Subagent>,
    /// Their words, redacted, by id.
    pub cleans: HashMap<String, Arc<Clean>>,
    /// The session's own tokens (without its subagents').
    pub session_tokens: Option<SubagentTokens>,
    /// How its subagents' tokens relate to the session's.
    pub counted: TokensCounted,
    /// The tool's name.
    pub tool: String,
    /// Now, epoch ms.
    pub now_ms: i64,
    /// The session each subagent is in, by its id (a project's view).
    pub sessions: HashMap<String, SubagentSessionInfo>,
}

/// Where a status stands on the page: running only while the session is
/// open (D-06).
#[must_use]
pub fn status_word(status: SubagentStatus, live: bool) -> &'static str {
    match status {
        SubagentStatus::Running if live => "running",
        SubagentStatus::Running | SubagentStatus::Unknown => "unknown",
        SubagentStatus::Done => "done",
        SubagentStatus::Failed => "failed",
        SubagentStatus::Stopped => "stopped",
    }
}

/// One subagent in the tree.
pub struct Node<'a> {
    /// Its record.
    pub agent: &'a Subagent,
    /// Its words, redacted.
    pub clean: Arc<Clean>,
    /// Its status word.
    pub status: &'static str,
    /// The subagent that started it.
    pub parent: Option<usize>,
    /// The ones it started.
    pub children: Vec<usize>,
    /// Its own tokens.
    pub own: Option<u64>,
    /// Its tokens with its children's.
    pub subtree: Option<u64>,
    /// How long it ran (or has run), ms.
    pub duration: Option<i64>,
    /// Its group (by its first ancestor's message).
    pub group: String,
}

/// Every subagent as a tree, by id.
pub struct Tree<'a> {
    /// Every subagent.
    pub nodes: Vec<Node<'a>>,
    /// Each one's place in `nodes`, by id.
    pub by_id: HashMap<&'a str, usize>,
    /// The ones the session started.
    pub roots: Vec<usize>,
    sessions: &'a HashMap<String, SubagentSessionInfo>,
}

impl<'a> Tree<'a> {
    #[must_use]
    /// The sessions each is in (a project's view).
    pub fn new(found: &'a Found) -> Self {
        let by_id: HashMap<&str, usize> = found
            .agents
            .iter()
            .enumerate()
            .map(|(n, agent)| (agent.id.as_str(), n))
            .collect();
        let mut nodes: Vec<Node<'a>> = found
            .agents
            .iter()
            .map(|agent| {
                let status = status_word(agent.status, found.live);
                let duration = match (agent.started_ms, agent.ended_ms) {
                    (Some(start), Some(end)) if agent.status.is_final() => Some(end - start),
                    (Some(start), _) if status == "running" => Some(found.now_ms - start),
                    _ => agent.details.duration_ms,
                }
                .map(|ms| ms.max(0));
                Node {
                    agent,
                    clean: found.cleans.get(&agent.id).cloned().unwrap_or_default(),
                    status,
                    parent: None,
                    children: Vec::new(),
                    own: agent.details.tokens.map(SubagentTokens::total),
                    subtree: None,
                    duration,
                    group: String::new(),
                }
            })
            .collect();
        // Parents, with a loop in the records cut where it closes.
        for n in 0..nodes.len() {
            let Some(&parent) = nodes[n].agent.parent.as_deref().and_then(|p| by_id.get(p)) else {
                continue;
            };
            let mut at = Some(parent);
            let mut steps = 0;
            let mut cycle = false;
            while let Some(up) = at {
                if up == n || steps > DEEPEST {
                    cycle = true;
                    break;
                }
                steps += 1;
                at = nodes[up].parent;
            }
            if !cycle {
                nodes[n].parent = Some(parent);
                nodes[parent].children.push(n);
            }
        }
        let roots: Vec<usize> = (0..nodes.len())
            .filter(|n| nodes[*n].parent.is_none())
            .collect();
        for &root in &roots {
            let key = nodes[root]
                .agent
                .details
                .asked
                .as_ref()
                .map_or_else(|| "none".to_owned(), |asked| asked.key.clone());
            let session = found
                .sessions
                .get(&nodes[root].agent.id)
                .map(|s| format!("{}\u{1f}{}\u{1f}", s.account_id, s.conversation_id))
                .unwrap_or_default();
            let group = format!("{session}{key}");
            let mut stack = vec![root];
            while let Some(n) = stack.pop() {
                nodes[n].group.clone_from(&group);
                stack.extend(nodes[n].children.iter().copied());
            }
        }
        let mut tree = Self {
            nodes,
            by_id,
            roots,
            sessions: &found.sessions,
        };
        for root in tree.roots.clone() {
            tree.total(root, 0);
        }
        tree
    }

    /// A node's tokens with its children's.
    fn total(&mut self, n: usize, depth: usize) -> Option<u64> {
        let mut sum = self.nodes[n].own;
        if depth < DEEPEST {
            for child in self.nodes[n].children.clone() {
                if let Some(tokens) = self.total(child, depth + 1) {
                    sum = Some(sum.unwrap_or(0).saturating_add(tokens));
                }
            }
        }
        self.nodes[n].subtree = sum;
        sum
    }

    /// `n`'s ancestors, nearest first.
    #[must_use]
    pub fn ancestors(&self, n: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut at = self.nodes[n].parent;
        while let Some(up) = at {
            if out.len() > DEEPEST {
                break;
            }
            out.push(up);
            at = self.nodes[up].parent;
        }
        out
    }

    /// The row a node shows as.
    #[must_use]
    pub fn info(&self, n: usize, conversations: bool) -> SubagentInfo {
        let node = &self.nodes[n];
        let agent = node.agent;
        SubagentInfo {
            id: agent.id.clone(),
            kind: agent.kind.clone(),
            task: node.clean.task.clone(),
            status: node.status.to_owned(),
            started_ms: agent.started_ms,
            ended_ms: agent.ended_ms.filter(|_| agent.status.is_final()),
            duration_ms: node.duration,
            last_activity_ms: agent.details.last_activity_ms,
            parent: node.parent.map(|p| self.nodes[p].agent.id.clone()),
            depth: agent.depth.max(1),
            background: agent.background,
            has_conversation: conversations && agent.transcript.is_some(),
            tokens: node.subtree,
            own_tokens: node.own,
            model: agent.details.model.clone(),
            children: u32::try_from(node.children.len()).unwrap_or(u32::MAX),
            session: self.sessions.get(agent.id.as_str()).cloned(),
        }
    }
}

/// The share `part` is of `whole`, 0–100.
#[must_use]
#[allow(clippy::cast_precision_loss)] // a percentage
pub fn share(part: u64, whole: u64) -> Option<f64> {
    (whole > 0).then(|| (part as f64 / whole as f64 * 100.0).clamp(0.0, 100.0))
}

/// Counts over every subagent.
#[must_use]
pub fn summary(found: &Found, tree: &Tree<'_>) -> SubagentsSummaryInfo {
    let count = |word: &str| tree.nodes.iter().filter(|n| n.status == word).count();
    let tokens: Option<u64> = tree
        .nodes
        .iter()
        .filter_map(|n| n.own)
        .reduce(u64::saturating_add);
    let whole =
        tokens.map(|t| t.saturating_add(found.session_tokens.map_or(0, SubagentTokens::total)));
    SubagentsSummaryInfo {
        total: u32::try_from(tree.nodes.len()).unwrap_or(u32::MAX),
        running: u32::try_from(count("running")).unwrap_or(u32::MAX),
        failed: u32::try_from(count("failed")).unwrap_or(u32::MAX),
        attention: u32::try_from(count("failed") + count("stopped") + count("unknown"))
            .unwrap_or(u32::MAX),
        tokens,
        share_pct: match (tokens, whole, found.session_tokens) {
            (Some(part), Some(whole), Some(_)) => share(part, whole),
            _ => None,
        },
        counted: counted_word(found.counted).to_owned(),
    }
}

/// How a tool's subagents' tokens relate to the session's, as a word.
#[must_use]
pub fn counted_word(counted: TokensCounted) -> &'static str {
    match counted {
        TokensCounted::InSession => "in_session",
        TokensCounted::Separate => "separate",
        TokensCounted::NotRecorded => "none",
    }
}

/// What a query asks of each subagent.
struct Filter {
    view: String,
    kind: Option<String>,
    words: Vec<String>,
    kinds: Vec<String>,
    files: Vec<String>,
}

impl Filter {
    fn of(query: &SubagentsQuery) -> Self {
        let mut out = Self {
            view: query.view.clone().unwrap_or_else(|| "all".to_owned()),
            kind: query.kind.clone().filter(|kind| !kind.is_empty()),
            words: Vec::new(),
            kinds: Vec::new(),
            files: Vec::new(),
        };
        for term in query
            .search
            .as_deref()
            .unwrap_or_default()
            .split_whitespace()
        {
            let term = term.to_lowercase();
            if let Some(kind) = term.strip_prefix("type:") {
                out.kinds.push(kind.to_owned());
            } else if let Some(file) = term.strip_prefix("file:") {
                out.files.push(file.to_owned());
            } else {
                out.words.push(term);
            }
        }
        out.kinds.retain(|k| !k.is_empty());
        out.files.retain(|f| !f.is_empty());
        out
    }

    fn active(&self) -> bool {
        self.view != "all"
            || self.kind.is_some()
            || !self.words.is_empty()
            || !self.kinds.is_empty()
            || !self.files.is_empty()
    }

    fn matches(&self, node: &Node<'_>) -> bool {
        let view = match self.view.as_str() {
            "running" => node.status == "running",
            "attention" => matches!(node.status, "failed" | "stopped" | "unknown"),
            _ => true,
        };
        view && self
            .kind
            .as_deref()
            .is_none_or(|kind| node.agent.kind.as_deref() == Some(kind))
            && self
                .words
                .iter()
                .all(|w| node.clean.hay.contains(w.as_str()))
            && self
                .kinds
                .iter()
                .all(|k| node.clean.kind_lc.contains(k.as_str()))
            && self
                .files
                .iter()
                .all(|f| node.clean.files_lc.contains(f.as_str()))
    }
}

/// The order siblings and groups take.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sort {
    Newest,
    Tokens,
    Longest,
}

impl Sort {
    fn of(word: Option<&str>) -> Self {
        match word {
            Some("tokens") => Self::Tokens,
            Some("longest") => Self::Longest,
            _ => Self::Newest,
        }
    }

    /// A node's rank (larger first).
    fn rank(self, node: &Node<'_>) -> i64 {
        match self {
            Self::Newest => node.agent.started_ms.unwrap_or(i64::MIN),
            Self::Tokens => node
                .subtree
                .map_or(-1, |t| i64::try_from(t).unwrap_or(i64::MAX)),
            Self::Longest => node.duration.unwrap_or(-1),
        }
    }
}

/// One group being built.
struct Group {
    key: String,
    root: usize,
    members: Vec<usize>,
    rank: i64,
}

/// The rows `query` asks for, and how many subagents match.
#[must_use]
pub fn rows(
    found: &Found,
    tree: &Tree<'_>,
    query: &SubagentsQuery,
    conversations: bool,
) -> (Vec<SubagentRowInfo>, u32) {
    let filter = Filter::of(query);
    let sort = Sort::of(query.sort.as_deref());
    let active = filter.active();
    let matched: HashSet<usize> = (0..tree.nodes.len())
        .filter(|&n| filter.matches(&tree.nodes[n]))
        .collect();
    let mut shown: HashSet<usize> = matched.clone();
    if active {
        for &n in &matched {
            shown.extend(tree.ancestors(n));
        }
    }
    // Groups, by the message their first subagent was started with.
    let mut groups: Vec<Group> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for &root in &tree.roots {
        let mut members = Vec::new();
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            members.push(n);
            stack.extend(tree.nodes[n].children.iter().copied());
        }
        let key = tree.nodes[root].group.as_str();
        let at = *index.entry(key).or_insert_with(|| {
            groups.push(Group {
                key: key.to_owned(),
                root,
                members: Vec::new(),
                rank: i64::MIN,
            });
            groups.len() - 1
        });
        groups[at].members.extend(members);
    }
    for group in &mut groups {
        group.rank = match sort {
            Sort::Newest => group_at(tree, group).unwrap_or(i64::MIN),
            Sort::Tokens => group
                .members
                .iter()
                .filter_map(|&n| tree.nodes[n].own)
                .fold(0_u64, u64::saturating_add)
                .try_into()
                .unwrap_or(i64::MAX),
            Sort::Longest => group
                .members
                .iter()
                .filter_map(|&n| tree.nodes[n].duration)
                .max()
                .unwrap_or(-1),
        };
    }
    groups.retain(|group| group.members.iter().any(|n| shown.contains(n)));
    groups.sort_by(|a, b| b.rank.cmp(&a.rank).then_with(|| a.key.cmp(&b.key)));
    let mut out = Vec::new();
    for (place, group) in groups.iter().enumerate() {
        let open = if query.closed.contains(&group.key) {
            false
        } else {
            query.open.contains(&group.key) || active || place == 0
        };
        out.push(group_row(found, tree, group, open));
        if !open {
            continue;
        }
        let mut roots: Vec<usize> = group
            .members
            .iter()
            .copied()
            .filter(|&n| tree.nodes[n].parent.is_none() && shown.contains(&n))
            .collect();
        order(tree, sort, &mut roots);
        let last = roots.len().saturating_sub(1);
        for (i, root) in roots.into_iter().enumerate() {
            walk(
                tree,
                root,
                &[],
                i == last,
                &shown,
                &matched,
                sort,
                conversations,
                &mut out,
                0,
            );
        }
    }
    (out, u32::try_from(matched.len()).unwrap_or(u32::MAX))
}

/// When a group's message was sent, else its first subagent's start.
fn group_at(tree: &Tree<'_>, group: &Group) -> Option<i64> {
    tree.nodes[group.root]
        .agent
        .details
        .asked
        .as_ref()
        .and_then(|asked| asked.at_ms)
        .or_else(|| {
            group
                .members
                .iter()
                .filter_map(|&n| tree.nodes[n].agent.started_ms)
                .min()
        })
}

fn group_row(found: &Found, tree: &Tree<'_>, group: &Group, open: bool) -> SubagentRowInfo {
    let count = |word: &str| {
        group
            .members
            .iter()
            .filter(|&&n| tree.nodes[n].status == word)
            .count()
    };
    let tokens = group
        .members
        .iter()
        .filter_map(|&n| tree.nodes[n].own)
        .reduce(u64::saturating_add);
    SubagentRowInfo {
        key: format!("g:{}", group.key),
        row: "group".to_owned(),
        group: Some(SubagentGroupInfo {
            key: group.key.clone(),
            asked: tree.nodes[group.root].clean.asked.clone(),
            at_ms: group_at(tree, group),
            count: u32::try_from(group.members.len()).unwrap_or(u32::MAX),
            running: u32::try_from(count("running")).unwrap_or(u32::MAX),
            failed: u32::try_from(count("failed")).unwrap_or(u32::MAX),
            tokens,
            open,
            session: found
                .sessions
                .get(&tree.nodes[group.root].agent.id)
                .cloned(),
        }),
        agent: None,
        guides: Vec::new(),
        last: false,
        context: false,
    }
}

fn order(tree: &Tree<'_>, sort: Sort, list: &mut [usize]) {
    list.sort_by(|&a, &b| {
        sort.rank(&tree.nodes[b])
            .cmp(&sort.rank(&tree.nodes[a]))
            .then_with(|| tree.nodes[a].agent.id.cmp(&tree.nodes[b].agent.id))
    });
}

#[allow(clippy::too_many_arguments)] // one tree walk's state
fn walk(
    tree: &Tree<'_>,
    n: usize,
    guides: &[bool],
    last: bool,
    shown: &HashSet<usize>,
    matched: &HashSet<usize>,
    sort: Sort,
    conversations: bool,
    out: &mut Vec<SubagentRowInfo>,
    depth: usize,
) {
    out.push(SubagentRowInfo {
        key: format!("a:{}", tree.nodes[n].agent.id),
        row: "agent".to_owned(),
        group: None,
        agent: Some(tree.info(n, conversations)),
        guides: guides.to_vec(),
        last,
        context: !matched.contains(&n),
    });
    if depth >= DEEPEST {
        return;
    }
    let mut kids: Vec<usize> = tree.nodes[n]
        .children
        .iter()
        .copied()
        .filter(|kid| shown.contains(kid))
        .collect();
    order(tree, sort, &mut kids);
    let mut below = guides.to_vec();
    below.push(!last);
    let end = kids.len().saturating_sub(1);
    for (i, kid) in kids.into_iter().enumerate() {
        walk(
            tree,
            kid,
            &below,
            i == end,
            shown,
            matched,
            sort,
            conversations,
            out,
            depth + 1,
        );
    }
}

/// Each type and how many.
#[must_use]
pub fn kinds(tree: &Tree<'_>) -> Vec<SubagentKindInfo> {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for node in &tree.nodes {
        if let Some(kind) = node.agent.kind.as_deref() {
            *counts.entry(kind).or_default() += 1;
        }
    }
    let mut out: Vec<SubagentKindInfo> = counts
        .into_iter()
        .map(|(kind, count)| SubagentKindInfo {
            kind: kind.to_owned(),
            count,
        })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.kind.cmp(&b.kind)));
    out
}

/// The page of rows `query` and `request` ask for.
///
/// # Errors
///
/// A cursor this app didn't make.
pub fn listed(
    found: &Found,
    query: &SubagentsQuery,
    request: &PageRequest,
) -> AppResult<SubagentsInfo> {
    let note = match found.support {
        SubagentSupport::Supported {
            conversations: juggler_adapter::subagents::SubagentConversations::NotRead(why),
            ..
        } => Some(why.to_owned()),
        _ => None,
    };
    let tree = Tree::new(found);
    let summary = summary(found, &tree);
    let (rows, matched) = rows(found, &tree, query, note.is_none());
    let indexed: Vec<(usize, SubagentRowInfo)> = rows.into_iter().enumerate().collect();
    let page = crate::paging::page(
        indexed,
        |(at, row)| SortKey::new([i64::try_from(*at).unwrap_or(i64::MAX)], row.key.clone()),
        request,
    )?
    .map(|(_, row)| row);
    Ok(SubagentsInfo {
        state: "listed".to_owned(),
        reason: None,
        page,
        running: summary.running,
        summary,
        kinds: kinds(&tree),
        matched,
        reading: found.reading,
        live: found.live,
        conversations_note: note,
        tool: found.tool.clone(),
    })
}
