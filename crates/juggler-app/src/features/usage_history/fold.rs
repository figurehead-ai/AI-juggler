//! Folding the hourly sums into what the charts draw: series per bucket,
//! shares, the top sessions and the activity counts. Pure: the rows, the
//! names and the prices come in, the summary goes out.

use std::collections::{BTreeSet, HashMap, HashSet};

use juggler_core::model::TokenCounts;
use juggler_core::storage::{HourRow, add_tokens as add};

use super::dto::{
    UsagePartInfo, UsageSeriesInfo, UsageSessionInfo, UsageShareInfo, UsageTotalsInfo,
};
use crate::commands::TokenCountsInfo;

/// At most this many series; the rest fold into "Other" (a ninth colour
/// would be a guess).
pub const MAX_SERIES: usize = 8;
/// The key of the folded series.
pub const OTHER: &str = "__other";
/// At most this many projects in `by_project`.
pub const MAX_PROJECTS: usize = 200;

/// What the summary needs to know about a profile.
#[derive(Clone, Debug, Default)]
pub struct ProfileFacts {
    /// Its name.
    pub label: String,
    /// Its tool's id.
    pub tool: String,
}

/// What the summary needs to know about a session.
#[derive(Clone, Debug, Default)]
pub struct SessionFacts {
    /// Its project.
    pub project: Option<String>,
    /// Its tool, when the conversation says.
    pub tool: Option<String>,
    /// Its title.
    pub title: Option<String>,
    /// AI Juggler's session, when AI Juggler started it.
    pub juggler: Option<String>,
}

/// The names the summary labels its groups with.
#[derive(Clone, Debug, Default)]
pub struct Lookups {
    /// Profiles by id.
    pub profiles: HashMap<String, ProfileFacts>,
    /// Sessions by (profile, conversation id).
    pub sessions: HashMap<(String, String), SessionFacts>,
    /// Project names by id.
    pub projects: HashMap<String, String>,
}

/// How a row is priced: its model, its hour and its tokens.
pub type Price<'a> = &'a dyn Fn(Option<&str>, i64, TokenCounts) -> Option<f64>;

/// The folded summary of one range.
#[derive(Clone, Debug, Default)]
pub struct Folded {
    /// One series per group, biggest first.
    pub series: Vec<UsageSeriesInfo>,
    /// Totals.
    pub totals: UsageTotalsInfo,
    /// Shares.
    pub by_profile: Vec<UsageShareInfo>,
    /// Shares.
    pub by_tool: Vec<UsageShareInfo>,
    /// Shares.
    pub by_model: Vec<UsageShareInfo>,
    /// Shares.
    pub by_project: Vec<UsageShareInfo>,
    /// Every session with usage, biggest first.
    pub sessions: Vec<UsageSessionInfo>,
    /// Sessions with usage, per bucket.
    pub sessions_per_bucket: Vec<u32>,
    /// Hours with usage, ascending.
    pub active_hours: Vec<i64>,
}

/// The bucket an instant falls in, if it's inside the range.
#[must_use]
pub fn bucket_of(edges: &[i64], at_ms: i64) -> Option<usize> {
    let (first, last) = (*edges.first()?, *edges.last()?);
    if at_ms < first || at_ms >= last {
        return None;
    }
    Some(edges.partition_point(|edge| *edge <= at_ms) - 1)
}

/// Input and output tokens: what the charts count.
#[must_use]
pub fn io(tokens: &TokenCounts) -> u64 {
    tokens.input.saturating_add(tokens.output)
}

fn cache(tokens: &TokenCounts) -> u64 {
    tokens.cache_read.saturating_add(tokens.cache_write)
}

#[derive(Default)]
struct Share {
    io: u64,
    cache: u64,
    usd: f64,
    priced: bool,
    sessions: HashSet<(String, String)>,
    parts: HashMap<String, u64>,
}

impl Share {
    fn take(
        &mut self,
        profile: &str,
        session: &(String, String),
        tokens: &TokenCounts,
        usd: Option<f64>,
    ) {
        self.io = self.io.saturating_add(io(tokens));
        self.cache = self.cache.saturating_add(cache(tokens));
        if let Some(usd) = usd {
            self.usd += usd;
            self.priced = true;
        }
        if !session.1.is_empty() {
            self.sessions.insert(session.clone());
        }
        *self.parts.entry(profile.to_owned()).or_default() += io(tokens);
    }

    fn info(self, key: String, label: String) -> UsageShareInfo {
        let mut parts: Vec<UsagePartInfo> = self
            .parts
            .into_iter()
            .map(|(profile, io)| UsagePartInfo { profile, io })
            .collect();
        parts.sort_by(|a, b| b.io.cmp(&a.io).then_with(|| a.profile.cmp(&b.profile)));
        parts.truncate(6);
        UsageShareInfo {
            key,
            label,
            io: self.io,
            cache: self.cache,
            usd: self.priced.then_some(self.usd),
            sessions: u32::try_from(self.sessions.len()).unwrap_or(u32::MAX),
            profiles: parts,
        }
    }
}

struct SessionSum {
    tokens: TokenCounts,
    usd: f64,
    priced: bool,
    first: i64,
    last: i64,
    messages: u32,
}

/// The group a row belongs to under `group_by`.
fn group_key(
    group_by: &str,
    profile: &str,
    row: &HourRow,
    facts: Option<&SessionFacts>,
    lookups: &Lookups,
) -> String {
    match group_by {
        "tool" => facts
            .and_then(|facts| facts.tool.clone())
            .or_else(|| lookups.profiles.get(profile).map(|p| p.tool.clone()))
            .unwrap_or_default(),
        "model" => row.model.clone().unwrap_or_default(),
        "project" => facts
            .and_then(|facts| facts.project.clone())
            .unwrap_or_default(),
        _ => profile.to_owned(),
    }
}

/// A group's name.
#[must_use]
pub fn label_of(group_by: &str, key: &str, lookups: &Lookups) -> String {
    match (group_by, key) {
        (_, OTHER) => "Other".to_owned(),
        ("model", "") => "Model not recorded".to_owned(),
        ("project", "") => "No project".to_owned(),
        ("tool", "") => "Unknown tool".to_owned(),
        ("project", id) => lookups
            .projects
            .get(id)
            .cloned()
            .unwrap_or_else(|| "A removed project".to_owned()),
        ("profile", id) => lookups
            .profiles
            .get(id)
            .map_or_else(|| "A removed profile".to_owned(), |p| p.label.clone()),
        (_, other) => other.to_owned(),
    }
}

/// Folds one range's hourly rows (each with its profile) into the summary.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn fold(
    rows: &[(String, HourRow)],
    edges: &[i64],
    group_by: &str,
    lookups: &Lookups,
    price: Price<'_>,
) -> Folded {
    let buckets = edges.len().saturating_sub(1);
    let mut series: HashMap<String, (Vec<u64>, Vec<f64>)> = HashMap::new();
    let mut shares: [HashMap<String, Share>; 4] = Default::default();
    let mut sessions: HashMap<(String, String), SessionSum> = HashMap::new();
    let mut per_bucket: Vec<HashSet<(String, String)>> = vec![HashSet::new(); buckets];
    let mut hours: BTreeSet<i64> = BTreeSet::new();
    let mut totals = UsageTotalsInfo::default();
    let mut tokens = TokenCounts::default();
    let mut usd_total = 0.0;
    let mut priced_any = false;
    let mut messages: u32 = 0;
    let none = String::new();
    for (profile, row) in rows {
        let Some(bucket) = bucket_of(edges, row.hour_ms) else {
            continue;
        };
        let conversation = row.session.clone().unwrap_or_default();
        let key = (profile.clone(), conversation);
        let facts = lookups.sessions.get(&key);
        let usd = price(row.model.as_deref(), row.hour_ms, row.tokens);
        add(&mut tokens, row.tokens);
        messages = messages.saturating_add(row.messages);
        match usd {
            Some(value) => {
                usd_total += value;
                priced_any = true;
            }
            None => totals.unpriced = true,
        }
        hours.insert(row.hour_ms);
        let group = group_key(group_by, profile, row, facts, lookups);
        let entry = series
            .entry(group)
            .or_insert_with(|| (vec![0; buckets], vec![0.0; buckets]));
        entry.0[bucket] = entry.0[bucket].saturating_add(io(&row.tokens));
        entry.1[bucket] += usd.unwrap_or(0.0);
        let tool = facts
            .and_then(|facts| facts.tool.clone())
            .or_else(|| lookups.profiles.get(profile).map(|p| p.tool.clone()))
            .unwrap_or_default();
        let project = facts
            .and_then(|facts| facts.project.clone())
            .unwrap_or_default();
        let model = row.model.clone().unwrap_or_default();
        for (index, group) in [profile, &tool, &model, &project].into_iter().enumerate() {
            shares[index]
                .entry(group.clone())
                .or_default()
                .take(profile, &key, &row.tokens, usd);
        }
        if key.1 != none {
            per_bucket[bucket].insert(key.clone());
            let sum = sessions.entry(key).or_insert(SessionSum {
                tokens: TokenCounts::default(),
                usd: 0.0,
                priced: false,
                first: row.hour_ms,
                last: row.hour_ms,
                messages: 0,
            });
            add(&mut sum.tokens, row.tokens);
            if let Some(value) = usd {
                sum.usd += value;
                sum.priced = true;
            }
            sum.first = sum.first.min(row.hour_ms);
            sum.last = sum.last.max(row.hour_ms);
            sum.messages = sum.messages.saturating_add(row.messages);
        }
    }
    totals.tokens = TokenCountsInfo {
        input: tokens.input,
        output: tokens.output,
        cache_read: tokens.cache_read,
        cache_write: tokens.cache_write,
        reasoning: tokens.reasoning,
    };
    totals.usd = priced_any.then_some(usd_total);
    totals.sessions = u32::try_from(sessions.len()).unwrap_or(u32::MAX);
    totals.active_hours = u32::try_from(hours.len()).unwrap_or(u32::MAX);
    totals.messages = messages;

    let [profiles, tools, models, projects] = shares;
    let ranked = |map: HashMap<String, Share>, group_by: &str, cap: usize| -> Vec<UsageShareInfo> {
        let mut list: Vec<UsageShareInfo> = map
            .into_iter()
            .map(|(key, share)| {
                let label = label_of(group_by, &key, lookups);
                share.info(key, label)
            })
            .collect();
        list.sort_by(|a, b| b.io.cmp(&a.io).then_with(|| a.key.cmp(&b.key)));
        list.truncate(cap);
        list
    };
    let mut sessions_list: Vec<UsageSessionInfo> = sessions
        .into_iter()
        .map(|((profile, id), sum)| {
            let facts = lookups.sessions.get(&(profile.clone(), id.clone()));
            UsageSessionInfo {
                tool: facts
                    .and_then(|facts| facts.tool.clone())
                    .or_else(|| lookups.profiles.get(&profile).map(|p| p.tool.clone()))
                    .unwrap_or_default(),
                title: facts.and_then(|facts| facts.title.clone()),
                project_id: facts.and_then(|facts| facts.project.clone()),
                session_id: facts.and_then(|facts| facts.juggler.clone()),
                io: io(&sum.tokens),
                cache: cache(&sum.tokens),
                usd: sum.priced.then_some(sum.usd),
                first_ms: sum.first,
                last_ms: sum.last,
                messages: sum.messages,
                profile,
                id,
            }
        })
        .collect();
    sessions_list.sort_by(|a, b| {
        b.io.cmp(&a.io)
            .then_with(|| b.last_ms.cmp(&a.last_ms))
            .then_with(|| a.id.cmp(&b.id))
    });

    Folded {
        series: ranked_series(series, group_by, lookups),
        totals,
        by_profile: ranked(profiles, "profile", usize::MAX),
        by_tool: ranked(tools, "tool", usize::MAX),
        by_model: ranked(models, "model", usize::MAX),
        by_project: ranked(projects, "project", MAX_PROJECTS),
        sessions: sessions_list,
        sessions_per_bucket: per_bucket
            .into_iter()
            .map(|set| u32::try_from(set.len()).unwrap_or(u32::MAX))
            .collect(),
        active_hours: hours.into_iter().collect(),
    }
}

/// The series, biggest first; past [`MAX_SERIES`] the rest are summed as
/// "Other".
fn ranked_series(
    series: HashMap<String, (Vec<u64>, Vec<f64>)>,
    group_by: &str,
    lookups: &Lookups,
) -> Vec<UsageSeriesInfo> {
    let mut list: Vec<(String, Vec<u64>, Vec<f64>)> = series
        .into_iter()
        .map(|(key, (io, usd))| (key, io, usd))
        .collect();
    let total = |io: &[u64]| {
        io.iter()
            .fold(0u64, |sum, value| sum.saturating_add(*value))
    };
    list.sort_by(|a, b| total(&b.1).cmp(&total(&a.1)).then_with(|| a.0.cmp(&b.0)));
    if list.len() > MAX_SERIES {
        let rest = list.split_off(MAX_SERIES - 1);
        let width = rest.first().map_or(0, |entry| entry.1.len());
        let mut io = vec![0u64; width];
        let mut usd = vec![0.0; width];
        for (_, more_io, more_usd) in rest {
            for (index, value) in more_io.into_iter().enumerate() {
                io[index] = io[index].saturating_add(value);
            }
            for (index, value) in more_usd.into_iter().enumerate() {
                usd[index] += value;
            }
        }
        list.push((OTHER.to_owned(), io, usd));
    }
    list.into_iter()
        .map(|(key, io, usd)| UsageSeriesInfo {
            label: label_of(group_by, &key, lookups),
            key,
            io,
            usd,
        })
        .collect()
}

/// Sessions that started in each bucket.
#[must_use]
pub fn started_per_bucket(edges: &[i64], starts: impl IntoIterator<Item = i64>) -> Vec<u32> {
    let mut counts = vec![0u32; edges.len().saturating_sub(1)];
    for start in starts {
        if let Some(bucket) = bucket_of(edges, start) {
            counts[bucket] = counts[bucket].saturating_add(1);
        }
    }
    counts
}
