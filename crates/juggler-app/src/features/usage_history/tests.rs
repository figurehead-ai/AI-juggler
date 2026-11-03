//! The usage history's folding and its input checks.

use juggler_core::model::TokenCounts;
use juggler_core::quota::CallKind;
use juggler_core::storage::{GroupCost, HourRow};

use super::fold::{self, Lookups, MAX_SERIES, OTHER, ProfileFacts, SessionFacts};
use super::{UsageQueryInput, checked};

const HOUR: i64 = 3_600_000;

/// An hour's row with no stored cost (priced from its sum by `price`).
fn row(hour: i64, session: &str, model: &str, input: u64, output: u64) -> HourRow {
    let tokens = TokenCounts {
        input,
        output,
        cache_read: 1_000,
        ..TokenCounts::default()
    };
    HourRow {
        hour_ms: hour * HOUR,
        session: Some(session.to_owned()),
        root: None,
        kind: CallKind::Main,
        model: Some(model.to_owned()),
        tokens,
        messages: 2,
        cost: GroupCost {
            stored_usd: 0.0,
            unstored: tokens,
        },
    }
}

fn lookups() -> Lookups {
    let mut lookups = Lookups::default();
    for (id, label, tool) in [
        ("p-work", "Work", "claude-code"),
        ("p-codex", "Codex Main", "codex"),
    ] {
        lookups.profiles.insert(
            id.to_owned(),
            ProfileFacts {
                label: label.to_owned(),
                tool: tool.to_owned(),
            },
        );
    }
    lookups
        .projects
        .insert("proj-web".to_owned(), "web-app".to_owned());
    lookups.sessions.insert(
        ("p-work".to_owned(), "s-1".to_owned()),
        SessionFacts {
            project: Some("proj-web".to_owned()),
            tool: Some("claude-code".to_owned()),
            title: Some("Refactor the billing module".to_owned()),
            juggler: Some("sess-1".to_owned()),
        },
    );
    lookups
}

fn price(model: Option<&str>, _at: i64, tokens: TokenCounts) -> Option<f64> {
    // A dollar a million tokens in and out; "mystery" has no price.
    (model? != "mystery").then(|| (tokens.input + tokens.output) as f64 / 1e6)
}

#[test]
fn an_instant_falls_in_the_bucket_whose_edges_hold_it() {
    let edges = [0, 10, 20, 30];
    assert_eq!(fold::bucket_of(&edges, -1), None);
    assert_eq!(fold::bucket_of(&edges, 0), Some(0));
    assert_eq!(fold::bucket_of(&edges, 9), Some(0));
    assert_eq!(fold::bucket_of(&edges, 10), Some(1));
    assert_eq!(fold::bucket_of(&edges, 29), Some(2));
    assert_eq!(fold::bucket_of(&edges, 30), None);
}

#[test]
fn rows_fold_into_series_shares_sessions_and_activity() {
    let rows = vec![
        (
            "p-work".to_owned(),
            row(0, "s-1", "opus", 1_000_000, 500_000),
        ),
        ("p-work".to_owned(), row(1, "s-1", "opus", 2_000_000, 0)),
        (
            "p-codex".to_owned(),
            row(25, "s-9", "gpt-5", 400_000, 100_000),
        ),
        ("p-codex".to_owned(), row(26, "s-9", "mystery", 10, 0)),
        // Outside the range: left out of everything.
        ("p-codex".to_owned(), row(48, "s-9", "gpt-5", 9_999_999, 0)),
    ];
    let edges = [0, 24 * HOUR, 48 * HOUR];
    let price_fn = price;
    let folded = fold::fold(&rows, &edges, "profile", &lookups(), &price_fn);

    assert_eq!(folded.series.len(), 2);
    assert_eq!(folded.series[0].key, "p-work");
    assert_eq!(folded.series[0].label, "Work");
    assert_eq!(folded.series[0].io, vec![3_500_000, 0]);
    assert_eq!(folded.series[1].io, vec![0, 500_010]);
    assert!((folded.series[0].usd[0] - 3.5).abs() < 1e-9);

    assert_eq!(folded.totals.tokens.input, 3_400_010);
    assert_eq!(folded.totals.tokens.cache_read, 4_000);
    assert_eq!(folded.totals.sessions, 2);
    assert_eq!(folded.totals.active_hours, 4);
    assert_eq!(folded.totals.messages, 8);
    assert!(folded.totals.unpriced, "the mystery model has no price");

    assert_eq!(folded.by_project[0].key, "proj-web");
    assert_eq!(folded.by_project[0].label, "web-app");
    assert_eq!(folded.by_project[1].label, "No project");
    assert_eq!(folded.by_tool[0].key, "claude-code");
    assert_eq!(folded.by_model[0].key, "opus");
    assert_eq!(folded.by_project[0].profiles[0].profile, "p-work");

    assert_eq!(folded.sessions[0].id, "s-1");
    assert_eq!(
        folded.sessions[0].title.as_deref(),
        Some("Refactor the billing module")
    );
    assert_eq!(folded.sessions[0].session_id.as_deref(), Some("sess-1"));
    assert_eq!(folded.sessions[0].first_ms, 0);
    assert_eq!(folded.sessions[0].last_ms, HOUR);
    assert_eq!(
        folded.sessions[1].tool, "codex",
        "the profile's tool when the conversation isn't known"
    );
    assert_eq!(folded.sessions_per_bucket, vec![1, 1]);
    assert_eq!(folded.active_hours, vec![0, HOUR, 25 * HOUR, 26 * HOUR]);
}

#[test]
fn past_eight_series_the_smallest_fold_into_other() {
    let rows: Vec<(String, HourRow)> = (0..12u64)
        .map(|n| {
            (
                "p-work".to_owned(),
                row(
                    0,
                    &format!("s-{n}"),
                    &format!("model-{n:02}"),
                    1_000 * (n + 1),
                    0,
                ),
            )
        })
        .collect();
    let price_fn = price;
    let folded = fold::fold(&rows, &[0, HOUR], "model", &lookups(), &price_fn);
    assert_eq!(folded.series.len(), MAX_SERIES);
    let other = folded.series.last().unwrap();
    assert_eq!(other.key, OTHER);
    assert_eq!(other.label, "Other");
    // models 00..04 (1k..5k) are the five smallest.
    assert_eq!(other.io, vec![15_000]);
    let total: u64 = folded.series.iter().map(|s| s.io[0]).sum();
    assert_eq!(total, (1..=12u64).map(|n| n * 1_000).sum::<u64>());
}

#[test]
fn starts_count_in_their_bucket() {
    assert_eq!(
        fold::started_per_bucket(&[0, 10, 20], [0, 5, 15, 25, -3]),
        vec![2, 1]
    );
}

fn query(edges: Vec<i64>, group_by: &str) -> UsageQueryInput {
    UsageQueryInput {
        edges_ms: edges,
        group_by: group_by.to_owned(),
        profile: None,
        project: None,
        top: None,
    }
}

#[test]
fn only_ranges_the_screen_can_ask_for_pass() {
    assert!(checked(&query(vec![0, 10], "profile")).is_ok());
    assert!(
        checked(&query(vec![0], "profile")).is_err(),
        "one edge is no range"
    );
    assert!(
        checked(&query(vec![10, 0], "profile")).is_err(),
        "edges rise"
    );
    assert!(
        checked(&query(vec![0, 10, 10], "profile")).is_err(),
        "no empty bucket"
    );
    assert!(
        checked(&query((0..500).collect(), "profile")).is_err(),
        "at most 400 buckets"
    );
    assert!(
        checked(&query(vec![0, 10], "colour")).is_err(),
        "a known grouping"
    );
    assert!(
        checked(&query(vec![0, 30 * 366 * 86_400_000], "tool")).is_err(),
        "at most twenty years"
    );
}

/// 0.3.7, the usage audit: "tokens" are new tokens (input, cache writes,
/// output and reasoning; cache reads apart), a subagent's or an automatic
/// review's thread counts under its conversation, and a stored per-call
/// cost is used as it is.
#[test]
fn new_tokens_and_conversations_count_as_the_vendors_bill_them() {
    let claude = HourRow {
        tokens: TokenCounts {
            input: 2,
            output: 100,
            cache_read: 90_000,
            cache_write: 4_000,
            reasoning: 0,
        },
        cost: GroupCost {
            stored_usd: 0.25,
            unstored: TokenCounts::default(),
        },
        ..row(0, "s-1", "opus", 0, 0)
    };
    let main = HourRow {
        tokens: TokenCounts {
            input: 1_000,
            output: 60,
            cache_read: 0,
            cache_write: 0,
            reasoning: 40,
        },
        cost: GroupCost {
            stored_usd: 0.5,
            unstored: TokenCounts::default(),
        },
        ..row(1, "th-main", "gpt-5", 0, 0)
    };
    let review = HourRow {
        root: Some("th-main".to_owned()),
        kind: CallKind::Review,
        ..row(1, "th-review", "mystery", 300, 0)
    };
    let rows = vec![
        ("p-work".to_owned(), claude),
        ("p-codex".to_owned(), main),
        ("p-codex".to_owned(), review),
    ];
    let folded = fold::fold(&rows, &[0, 24 * HOUR], "tool", &lookups(), &price);

    assert_eq!(folded.totals.sessions, 2, "the review is not a session");
    let by_tool: Vec<_> = folded
        .by_tool
        .iter()
        .map(|share| (share.key.as_str(), share.io, share.cache))
        .collect();
    assert_eq!(
        by_tool,
        [("claude-code", 4_102, 90_000), ("codex", 1_400, 1_000)],
        "Claude's cache writes and Codex's reasoning are new tokens; reads apart"
    );
    let conversation = folded
        .sessions
        .iter()
        .find(|session| session.id == "th-main")
        .unwrap();
    assert_eq!(
        conversation.io,
        1_100 + 300,
        "the review's tokens, under it"
    );
    assert!(
        folded.totals.unpriced,
        "the review's model has no list price"
    );
    assert_eq!(folded.totals.usd, Some(0.75), "stored costs as they are");
}
