//! The usage history's wire types (0.1.5). Every list is bounded: a page
//! draws a few rows per chart, and a long list is read a page at a time.

use crate::commands::TokenCountsInfo;

/// What a page asks for: a range cut into buckets, what to split the
/// usage by, and whose usage.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageQueryInput {
    /// The buckets' edges, epoch ms, ascending: the first is the range's
    /// start and the last its end (2 to 401 of them). The screen works
    /// them out in the user's time zone, so a day is the user's day.
    #[ts(type = "number[]")]
    pub edges_ms: Vec<i64>,
    /// What the series split by: `profile`, `tool`, `model` or `project`.
    pub group_by: String,
    /// Only this profile's usage.
    pub profile: Option<String>,
    /// Only this project's usage.
    pub project: Option<String>,
    /// How many of the top sessions to list (at most 50; 10 when absent).
    pub top: Option<u32>,
}

/// A range's usage, summarised for the charts.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageHistoryInfo {
    /// The buckets' edges, as asked.
    #[ts(type = "number[]")]
    pub edges_ms: Vec<i64>,
    /// What the series split by.
    pub group_by: String,
    /// One series per group, the biggest first, each with a value per
    /// bucket.
    pub series: Vec<UsageSeriesInfo>,
    /// The range's totals.
    pub totals: UsageTotalsInfo,
    /// The same span just before the range (the change on each tile).
    pub previous: UsageTotalsInfo,
    /// The range's usage by profile, the biggest first.
    pub by_profile: Vec<UsageShareInfo>,
    /// By tool.
    pub by_tool: Vec<UsageShareInfo>,
    /// By model.
    pub by_model: Vec<UsageShareInfo>,
    /// By project (at most 200; sessions in no project together).
    pub by_project: Vec<UsageShareInfo>,
    /// The sessions that used the most, the biggest first.
    pub top_sessions: Vec<UsageSessionInfo>,
    /// How many sessions used anything in the range.
    pub session_count: u32,
    /// Sessions with usage, per bucket.
    pub sessions_per_bucket: Vec<u32>,
    /// Sessions that started, per bucket (where the start is known).
    pub started_per_bucket: Vec<u32>,
    /// The start of every hour with usage, epoch ms, ascending (the
    /// heatmap folds them into the user's weekdays and hours).
    #[ts(type = "number[]")]
    pub active_hours: Vec<i64>,
    /// Moves of work between accounts in the range.
    pub switches: Vec<UsageSwitchInfo>,
    /// Sessions in scope whose usage isn't measured (their tool keeps no
    /// token counts on this Mac): counted, never guessed.
    pub unmeasured_sessions: u32,
    /// The range held more than a summary carries; the oldest hours were
    /// left out.
    pub truncated: bool,
    /// While the first read of the tools' history runs: how many
    /// conversations it has read so far (the figures are still coming:
    /// the 0.2 walk, F6). `None` once it's done.
    #[ts(type = "number | null")]
    pub reading: Option<u64>,
}

/// One group's usage over time.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageSeriesInfo {
    /// The group's id: a profile, tool, model or project id (`""` for
    /// none).
    pub key: String,
    /// Its name.
    pub label: String,
    /// Input and output tokens per bucket.
    #[ts(type = "number[]")]
    pub io: Vec<u64>,
    /// Estimated USD per bucket (what could be priced).
    pub usd: Vec<f64>,
}

/// A range's totals.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageTotalsInfo {
    /// The tokens, each message counted once.
    pub tokens: TokenCountsInfo,
    /// Estimated USD at list prices; `None` when none of it could be
    /// priced.
    pub usd: Option<f64>,
    /// Some usage couldn't be priced, so the estimate is low.
    pub unpriced: bool,
    /// Sessions with usage.
    pub sessions: u32,
    /// Hours with any usage.
    pub active_hours: u32,
    /// Messages.
    pub messages: u32,
}

/// One group's share of a range.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageShareInfo {
    /// The group's id (`""` for none).
    pub key: String,
    /// Its name.
    pub label: String,
    /// Input and output tokens.
    #[ts(type = "number")]
    pub io: u64,
    /// Cache tokens read and written.
    #[ts(type = "number")]
    pub cache: u64,
    /// Estimated USD; `None` when none of it could be priced.
    pub usd: Option<f64>,
    /// Sessions with usage.
    pub sessions: u32,
    /// The profiles that did the work, the biggest first (at most 6).
    pub profiles: Vec<UsagePartInfo>,
}

/// A profile's part of a group.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsagePartInfo {
    /// The profile.
    pub profile: String,
    /// Its input and output tokens in the group.
    #[ts(type = "number")]
    pub io: u64,
}

/// One session's usage in a range.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageSessionInfo {
    /// The profile it ran on.
    pub profile: String,
    /// The tool's id for it (its conversation).
    pub id: String,
    /// AI Juggler's session, when AI Juggler started it.
    pub session_id: Option<String>,
    /// Its tool.
    pub tool: String,
    /// Its title, when the tool recorded one.
    pub title: Option<String>,
    /// Its project, when AI Juggler knows it.
    pub project_id: Option<String>,
    /// Input and output tokens in the range.
    #[ts(type = "number")]
    pub io: u64,
    /// Cache tokens in the range.
    #[ts(type = "number")]
    pub cache: u64,
    /// Estimated USD in the range.
    pub usd: Option<f64>,
    /// Its first and last usage in the range, epoch ms.
    #[ts(type = "number")]
    pub first_ms: i64,
    /// See `first_ms`.
    #[ts(type = "number")]
    pub last_ms: i64,
    /// Messages in the range.
    pub messages: u32,
}

/// A move of work between accounts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageSwitchInfo {
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// The project.
    pub project_id: String,
    /// The profile it left.
    pub from_profile: String,
    /// The profile it went to.
    pub to_profile: String,
    /// `complete`, `failed`, … as recorded.
    pub state: String,
}

/// A profile's limit windows over a range.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageLimitsInfo {
    /// The profile.
    pub profile: String,
    /// The 5-hour window.
    pub five_hour: UsageWindowHistoryInfo,
    /// The weekly window.
    pub weekly: UsageWindowHistoryInfo,
}

/// One window's readings over a range.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageWindowHistoryInfo {
    /// The readings, oldest first: `[epoch ms, percent used]` (at most
    /// 400, thinned evenly).
    #[ts(type = "Array<[number, number]>")]
    pub points: Vec<(i64, f64)>,
    /// Each window the readings saw, oldest first, with its peak.
    pub windows: Vec<UsageWindowPeakInfo>,
    /// Readings in the range before thinning.
    pub samples: u32,
}

/// One window (one reset) and how full it got.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageWindowPeakInfo {
    /// Its first reading, epoch ms.
    #[ts(type = "number")]
    pub first_ms: i64,
    /// When it reset (or resets), epoch ms; `None` when the vendor didn't
    /// say.
    #[ts(type = "number | null")]
    pub resets_ms: Option<i64>,
    /// The highest percent used it was read at.
    pub peak_pct: f64,
}

/// One session's usage, turn by turn.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageSessionDetailInfo {
    /// Its messages, oldest first (the newest 2,000).
    pub turns: Vec<UsageTurnInfo>,
    /// Older messages were left out of `turns` (the totals count them).
    pub truncated: bool,
    /// Its totals.
    pub totals: UsageTotalsInfo,
    /// By model.
    pub by_model: Vec<UsageShareInfo>,
    /// Its share of the profile's windows now.
    pub windows: Vec<UsageWindowShareInfo>,
}

/// One message.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageTurnInfo {
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// The model.
    pub model: Option<String>,
    /// Its tokens.
    pub tokens: TokenCountsInfo,
    /// Its estimated USD.
    pub usd: Option<f64>,
}

/// How much of a window's usage one session is.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageWindowShareInfo {
    /// `five_hour` or `weekly`.
    pub window: String,
    /// The window's start, epoch ms.
    #[ts(type = "number")]
    pub since_ms: i64,
    /// `window`: from the vendor's reset time; `rolling`: no reset is
    /// known, so the last 5 hours or 7 days.
    pub basis: String,
    /// The session's input and output tokens in it.
    #[ts(type = "number")]
    pub session_io: u64,
    /// The profile's input and output tokens in it.
    #[ts(type = "number")]
    pub profile_io: u64,
}
