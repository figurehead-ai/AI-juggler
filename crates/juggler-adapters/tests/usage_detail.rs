//! 0.3.7, the usage audit: what each API call says beyond its token
//! classes, read from the tools' own records and priced per call. Claude's
//! 1-hour cache writes, a long-context Codex call, and Codex's automatic
//! reviews and subagents counted under the conversation that started
//! them. Lines are shaped like the real ones (every key the audit's
//! probe saw that matters here); values are invented.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_adapters::quota_ingest::{rollout_events, transcript_events};
use juggler_core::cost_ledger::PricingCatalog;
use juggler_core::model::TokenCounts;
use juggler_core::quota::{CallKind, UsageDetail};
use juggler_core::storage::Storage;

/// One Claude Code transcript line: an assistant message's usage, with
/// the cache writes split between the 5-minute and the 1-hour cache.
fn claude_line(id: &str, sidechain: bool, five_minute: u64, one_hour: u64, extra: &str) -> String {
    format!(
        r#"{{"type":"assistant","sessionId":"conv-1","isSidechain":{sidechain},"timestamp":"2026-11-01T10:00:00.000Z","requestId":"req-{id}","uuid":"u-{id}","message":{{"id":"msg-{id}","model":"claude-opus-5-5","usage":{{"input_tokens":2,"output_tokens":1000,"cache_read_input_tokens":50000,"cache_creation_input_tokens":{},"cache_creation":{{"ephemeral_5m_input_tokens":{five_minute},"ephemeral_1h_input_tokens":{one_hour}}}{extra}}}}}}}"#,
        five_minute + one_hour
    ) + "\n"
}

fn codex_line(kind: &str, minute: u32, body: &str) -> String {
    format!(
        "{{\"timestamp\":\"2026-11-01T10:{minute:02}:00.000Z\",\"type\":\"{kind}\",\"payload\":{body}}}\n"
    )
}

/// A Codex thread's `session_meta`: `source` as Codex writes it, and the
/// conversation it belongs to (`session_id`).
fn meta(thread: &str, source: &str, session: &str) -> String {
    codex_line(
        "session_meta",
        0,
        &format!(
            r#"{{"id":"{thread}","session_id":"{session}","parent_thread_id":null,"cwd":"/w/api","source":{source},"thread_source":"user"}}"#
        ),
    )
}

fn model(minute: u32, name: &str) -> String {
    codex_line("turn_context", minute, &format!(r#"{{"model":"{name}"}}"#))
}

/// A Codex response's record. Codex's input includes its cached input,
/// and its output its reasoning.
fn record(thread: &str, minute: u32, turn: &str, input: u64, cached: u64, output: u64) -> String {
    codex_line(
        "token_usage_record",
        minute,
        &format!(
            r#"{{"thread_id":"{thread}","turn_id":"{turn}","response_id":"r-{turn}","usage":{{"input_tokens":{input},"cached_input_tokens":{cached},"cache_write_input_tokens":0,"output_tokens":{output},"reasoning_output_tokens":0,"total_tokens":{}}}}}"#,
            input + output
        ),
    )
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn claude_one_hour_cache_writes_are_read_and_priced_at_twice_base_input() {
    let bytes = claude_line("a", false, 1_000, 2_000, "");
    let outcome = transcript_events("acc", bytes.as_bytes());
    let event = &outcome.events[0];
    assert_eq!(event.tokens.cache_write, 3_000);
    assert_eq!(event.detail.cache_write_1h, 2_000);
    assert_eq!(event.detail.prompt_tokens, 2 + 50_000 + 3_000);
    assert_eq!(event.detail.kind, CallKind::Main);
    // Opus 5.5: input $4, 5-minute writes $5, 1-hour writes $8, cache
    // hits $0.20, output $20 per million.
    let catalog = PricingCatalog::with_overrides(Vec::new());
    let usd = catalog
        .price_event("claude-opus-5-5", event.ts, event.tokens, &event.detail)
        .unwrap();
    let expected =
        (2.0 * 4.0 + 1_000.0 * 5.0 + 2_000.0 * 8.0 + 50_000.0 * 0.2 + 1_000.0 * 20.0) / 1e6;
    assert!(close(usd, expected), "{usd} vs {expected}");
    // At the 5-minute rate (before 0.3.7) it was $0.006 less.
    let flat = catalog
        .price_tokens("claude-opus-5-5", event.ts, event.tokens)
        .unwrap();
    assert!(close(usd - flat, 2_000.0 * (8.0 - 5.0) / 1e6));
}

#[test]
fn a_responses_lines_keep_the_largest_split_and_a_subagents_kind() {
    // Claude Code writes one line per content block, all with the same
    // usage; a later one may carry more.
    let bytes = [
        claude_line("a", true, 1_000, 0, ""),
        claude_line("a", true, 1_000, 2_000, ""),
    ]
    .concat();
    let outcome = transcript_events("acc", bytes.as_bytes());
    assert_eq!(outcome.events.len(), 1);
    assert_eq!(outcome.events[0].detail.cache_write_1h, 2_000);
    assert_eq!(outcome.events[0].detail.kind, CallKind::Subagent);
}

#[test]
fn fast_mode_and_us_only_inference_are_priced_at_their_rates() {
    let bytes = claude_line(
        "f",
        false,
        1_000,
        0,
        r#","speed":"fast","inference_geo":"us""#,
    );
    let event = &transcript_events("acc", bytes.as_bytes()).events[0];
    assert_eq!(event.detail.speed.as_deref(), Some("fast"));
    assert_eq!(event.detail.geo.as_deref(), Some("us"));
    let catalog = PricingCatalog::with_overrides(Vec::new());
    let usd = catalog
        .price_event("claude-opus-5-5", event.ts, event.tokens, &event.detail)
        .unwrap();
    // Fast: input $8, 5-minute writes $10, hits $0.40, output $40; ×1.1.
    let expected = (2.0 * 8.0 + 1_000.0 * 10.0 + 50_000.0 * 0.4 + 1_000.0 * 40.0) / 1e6 * 1.1;
    assert!(close(usd, expected), "{usd} vs {expected}");
    // The defaults are not stored as facts.
    let plain = claude_line(
        "g",
        false,
        1_000,
        0,
        r#","speed":"standard","inference_geo":"global""#,
    );
    let event = &transcript_events("acc", plain.as_bytes()).events[0];
    assert_eq!(
        (event.detail.speed.clone(), event.detail.geo.clone()),
        (None, None)
    );
}

#[test]
fn a_long_context_codex_call_pays_the_long_context_rates() {
    // 300K prompt (250K of it cached), over the 272K threshold; and a
    // 200K one under it.
    let bytes = [
        meta("th-main", r#""cli""#, "th-main"),
        model(0, "gpt-6.1-sol"),
        record("th-main", 1, "t1", 300_000, 250_000, 4_000),
        record("th-main", 2, "t2", 200_000, 150_000, 4_000),
    ]
    .concat();
    let outcome = rollout_events("acc", bytes.as_bytes());
    let [long, short] = &outcome.events[..] else {
        panic!("two calls")
    };
    assert_eq!(long.detail.prompt_tokens, 300_000);
    assert_eq!(long.tokens.input, 50_000, "input without the cached part");
    let catalog = PricingCatalog::with_overrides(Vec::new());
    let price = |event: &juggler_core::quota::UsageEvent| {
        catalog
            .price_event("gpt-6.1-sol", event.ts, event.tokens, &event.detail)
            .unwrap()
    };
    // gpt-6.1-sol long context: input $4, cached $0.20, output $15.
    assert!(close(
        price(long),
        (50_000.0 * 4.0 + 250_000.0 * 0.2 + 4_000.0 * 15.0) / 1e6
    ));
    // Short context: input $2, cached $0.10, output $10.
    assert!(close(
        price(short),
        (50_000.0 * 2.0 + 150_000.0 * 0.1 + 4_000.0 * 10.0) / 1e6
    ));
}

#[test]
fn claude_haiku_5_5_prices_a_prompt_over_100k_at_its_higher_rates() {
    let catalog = PricingCatalog::with_overrides(Vec::new());
    let tokens = TokenCounts {
        input: 1_000,
        output: 1_000,
        cache_read: 100_000,
        cache_write: 0,
        reasoning: 0,
    };
    // 101,000 prompt tokens, cache reads included: over the threshold.
    let usd = catalog
        .price_event("claude-haiku-5-5", 0, tokens, &UsageDetail::default())
        .unwrap();
    assert!(close(
        usd,
        (1_000.0 * 0.5 + 100_000.0 * 0.05 + 1_000.0 * 2.5) / 1e6
    ));
    let small = TokenCounts {
        cache_read: 10_000,
        ..tokens
    };
    let usd = catalog
        .price_event("claude-haiku-5-5", 0, small, &UsageDetail::default())
        .unwrap();
    assert!(close(
        usd,
        (1_000.0 * 0.1 + 10_000.0 * 0.01 + 1_000.0 * 0.5) / 1e6
    ));
}

/// Codex writes an automatic review (its "guardian") and a spawned
/// subagent as threads of their own, each naming the conversation in
/// `session_id`. The audit found 2,285 of 2,779 Codex "sessions" on one
/// Mac were such reviews.
fn three_threads() -> [String; 3] {
    let main = [
        meta("th-main", r#""vscode""#, "th-main"),
        model(0, "gpt-6.1-sol"),
        record("th-main", 1, "m1", 1_000, 0, 100),
    ]
    .concat();
    let review = [
        meta(
            "th-review",
            r#"{"subagent":{"other":"guardian"}}"#,
            "th-main",
        ),
        model(0, "codex-auto-review"),
        record("th-review", 2, "v1", 2_000, 0, 50),
    ]
    .concat();
    let subagent = [
        meta(
            "th-sub",
            r#"{"subagent":{"thread_spawn":{"parent_thread_id":"th-main","depth":1,"agent_path":"p","agent_nickname":"n","agent_role":null}}}"#,
            "th-main",
        ),
        model(0, "gpt-6.1-sol"),
        // A forked thread opens with its parent's history: that record is
        // the conversation's own call, not the subagent's.
        record("th-main", 1, "m1", 1_000, 0, 100),
        record("th-sub", 3, "s1", 3_000, 0, 10),
    ]
    .concat();
    [main, review, subagent]
}

#[test]
fn automatic_reviews_and_subagents_belong_to_their_conversation() {
    let [main, review, subagent] = three_threads();
    let main = rollout_events("acc", main.as_bytes());
    assert_eq!(main.events[0].detail.kind, CallKind::Main);
    assert_eq!(main.events[0].detail.root_session, None);
    let review = rollout_events("acc", review.as_bytes());
    assert_eq!(review.events[0].detail.kind, CallKind::Review);
    assert_eq!(
        review.events[0].detail.root_session.as_deref(),
        Some("th-main")
    );
    let subagent = rollout_events("acc", subagent.as_bytes());
    let kinds: Vec<_> = subagent
        .events
        .iter()
        .map(|event| (event.session.clone().unwrap(), event.detail.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("th-main".to_owned(), CallKind::Main),
            ("th-sub".to_owned(), CallKind::Subagent)
        ]
    );
    assert_eq!(
        subagent.events[1].detail.root_session.as_deref(),
        Some("th-main")
    );
}

#[test]
fn the_summaries_count_one_conversation_with_every_threads_tokens() {
    let storage = Storage::open_in_memory().unwrap();
    for file in three_threads() {
        let outcome = rollout_events("acc", file.as_bytes());
        storage.usage().store_events(&outcome.events).unwrap();
    }
    let (rows, _) = storage
        .usage_history()
        .hourly("acc", 0, i64::MAX, 1_000)
        .unwrap();
    let conversations: std::collections::BTreeSet<_> =
        rows.iter().filter_map(|row| row.conversation()).collect();
    assert_eq!(conversations.len(), 1, "one conversation: {rows:?}");
    let tokens: u64 = rows
        .iter()
        .map(|row| row.tokens.input + row.tokens.output)
        .sum();
    assert_eq!(tokens, 1_100 + 2_050 + 3_010, "every thread's tokens, once");
    let kinds: std::collections::BTreeSet<_> = rows.iter().map(|row| row.kind).collect();
    assert_eq!(
        kinds,
        [CallKind::Main, CallKind::Subagent, CallKind::Review]
            .into_iter()
            .collect()
    );
    // The conversation's own page holds its review's and subagent's calls.
    let (turns, _) = storage
        .usage_history()
        .session_turns("acc", "th-main", 100)
        .unwrap();
    assert_eq!(turns.len(), 3);
}

#[test]
fn a_stored_call_keeps_its_own_tier_in_the_hourly_sums() {
    let storage = Storage::open_in_memory().unwrap();
    let bytes = [
        meta("th-main", r#""cli""#, "th-main"),
        model(0, "gpt-6.1-sol"),
        record("th-main", 1, "t1", 300_000, 250_000, 4_000),
        record("th-main", 2, "t2", 200_000, 150_000, 4_000),
    ]
    .concat();
    let outcome = rollout_events("acc", bytes.as_bytes());
    storage.usage().store_events(&outcome.events).unwrap();
    let (rows, _) = storage
        .usage_history()
        .hourly("acc", 0, i64::MAX, 1_000)
        .unwrap();
    assert_eq!(rows.len(), 1, "one hour, one model");
    let cost = rows[0].cost;
    assert_eq!(cost.unstored, TokenCounts::default(), "every call priced");
    let expected = (50_000.0 * 4.0 + 250_000.0 * 0.2 + 4_000.0 * 15.0) / 1e6
        + (50_000.0 * 2.0 + 150_000.0 * 0.1 + 4_000.0 * 10.0) / 1e6;
    assert!(close(cost.stored_usd, expected), "{}", cost.stored_usd);
    // Priced from the hour's sum (the old way), the long call's tier is
    // lost: the hour reads low.
    let flat = PricingCatalog::with_overrides(Vec::new())
        .price_tokens("gpt-6.1-sol", 0, rows[0].tokens)
        .unwrap();
    assert!(flat < cost.stored_usd);
    // The cost reaches the per-call reads too.
    let events = storage.usage().events_for_account("acc", 0).unwrap();
    assert!(events.iter().all(|event| event.usd_est.is_some()));
}

#[test]
fn an_unpriced_model_stays_unpriced_never_zero() {
    let storage = Storage::open_in_memory().unwrap();
    let [_, review, _] = three_threads();
    let outcome = rollout_events("acc", review.as_bytes());
    storage.usage().store_events(&outcome.events).unwrap();
    let (rows, _) = storage
        .usage_history()
        .hourly("acc", 0, i64::MAX, 1_000)
        .unwrap();
    // `codex-auto-review` has no list price: its tokens wait for one.
    assert_eq!(rows[0].cost.stored_usd, 0.0);
    assert_eq!(rows[0].cost.unstored.input, 2_000);
    assert_eq!(rows[0].cost.total(|_| None), None);
}

#[test]
fn a_users_override_prices_every_stored_call_again() {
    let storage = Storage::open_in_memory().unwrap();
    let [_, review, _] = three_threads();
    let outcome = rollout_events("acc", review.as_bytes());
    storage.usage().store_events(&outcome.events).unwrap();
    let usage = storage.usage();
    assert!(!usage.prices_current().unwrap(), "a fresh store: unmarked");
    usage
        .pricing_override_upsert(&juggler_core::cost_ledger::LedgerPriceRow {
            model: "codex-auto-review".to_owned(),
            effective_from_ms: 0,
            input_per_1k: 0.001,
            output_per_1k: 0.002,
            cache_read_per_1k: 0.0,
            cache_write_per_1k: 0.0,
            cache_write_1h_per_1k: None,
            long_context: None,
            fast: None,
            us_multiplier: None,
            source_url: "test".to_owned(),
            captured: "2026-11-03".to_owned(),
        })
        .unwrap();
    let stamp = storage.usage_history().settled_stamp().unwrap();
    // In batches of one: a pass goes on where the last one stopped.
    let mut cursor = None;
    let mut batches = 0;
    loop {
        cursor = usage.reprice(cursor, 1).unwrap();
        batches += 1;
        if cursor.is_none() {
            break;
        }
    }
    assert!(batches >= 1);
    assert!(usage.prices_current().unwrap());
    assert_ne!(
        storage.usage_history().settled_stamp().unwrap(),
        stamp,
        "a settled hour's cost moved: the summaries kept in memory go"
    );
    let (rows, _) = storage
        .usage_history()
        .hourly("acc", 0, i64::MAX, 1_000)
        .unwrap();
    assert_eq!(rows[0].cost.unstored, TokenCounts::default());
    assert!(close(
        rows[0].cost.stored_usd,
        (2_000.0 * 1.0 + 50.0 * 2.0) / 1e6
    ));
}
