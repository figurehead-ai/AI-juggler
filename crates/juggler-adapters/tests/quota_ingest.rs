//! AJ-E607 ingest fixture corpus + gates: every source parses its
//! captured shape with zero dropped valid lines; the statusline
//! wrapper CHAINS a user statusline byte-for-byte; install/uninstall
//! is exact-restore; tails are idempotent (store-level COUNT proof);
//! the no-HTTP law is grep-pinned against the crate's sources.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

use juggler_adapters::quota_ingest::{
    self, SPOOL_ROTATE_BYTES, gemini_events, install_statusline, iso_to_epoch_ms, rollout_events,
    spool_path, statusline_samples, tail_rollout, tail_spool, tail_transcript,
    uninstall_statusline, zcode_events, zcode_is_peak,
};
use juggler_core::model::Confidence;
use juggler_core::quota::{UsageSource, WindowKind};
use juggler_core::storage::Storage;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quota");

fn fixture(name: &str) -> PathBuf {
    Path::new(FIXTURES).join(name)
}

// ---------------------------------------------------------------------------
// Source: Claude statusline spool
// ---------------------------------------------------------------------------

#[test]
fn spool_parses_with_zero_dropped_valid_lines() {
    let bytes = std::fs::read(fixture("claude-statusline.spool")).unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    // 4 lines: 2 with rate_limits, 1 without (cost only — valid, no
    // samples), 1 malformed JSON. Valid lines dropped: 0.
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    let mut samples = Vec::new();
    let mut errors = 0u64;
    for line in &lines {
        let tab = line.find('\t').unwrap();
        let ts: i64 = line[..tab].parse().unwrap();
        match serde_json::from_str::<serde_json::Value>(&line[tab + 1..]) {
            Ok(payload) => samples.extend(statusline_samples(ts, &payload)),
            Err(_) => errors += 1,
        }
    }
    assert_eq!(errors, 1, "exactly the malformed line is dropped");
    assert_eq!(samples.len(), 4, "two payloads × two windows");
    let (kind, first) = &samples[0];
    assert_eq!(*kind, WindowKind::FiveHour);
    assert!((first.used_pct - 42.5).abs() < 1e-9);
    assert_eq!(first.resets_at, Some(1_760_006_000_000));
    assert_eq!(first.confidence, Confidence::Measured);
    assert!(matches!(
        samples[1].0,
        WindowKind::Weekly { anchor_ms: None }
    ));
    // The second payload's five_hour sample follows in file order.
    assert!(matches!(samples[2].0, WindowKind::FiveHour));
    assert!((samples[2].1.used_pct - 50.0).abs() < 1e-9);
}

#[test]
fn spool_tail_is_offset_incremental_and_truncation_aware() {
    let dir = tempfile::tempdir().unwrap();
    let spool = dir.path().join("s.jsonl");
    std::fs::write(&spool, "100\t{\"a\":1}\n").unwrap();
    let first = tail_spool(&spool, 0);
    assert_eq!(
        first.offset,
        std::fs::read_to_string(&spool).unwrap().len() as u64
    );
    // No new data: same offset, nothing parsed.
    let second = tail_spool(&spool, first.offset);
    assert_eq!(second.offset, first.offset);
    // Append: only the new line is consumed.
    std::fs::write(&spool, "100\t{\"a\":1}\n200\t{\"b\":2}\n").unwrap();
    let third = tail_spool(&spool, first.offset);
    assert_eq!(third.parse_errors, 0);
    // Truncation resets to zero (full reparse).
    std::fs::write(&spool, "300\t{\"c\":3}\n").unwrap();
    let fourth = tail_spool(&spool, first.offset);
    assert_eq!(
        fourth.offset,
        std::fs::read_to_string(&spool).unwrap().len() as u64
    );
}

#[test]
fn spool_rotates_at_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let spool = dir.path().join("s.jsonl");
    // Sparse file over the threshold (fast, no 10 MB of disk).
    let file = std::fs::File::create(&spool).unwrap();
    use std::io::{Seek, SeekFrom, Write as _};
    let mut file = file;
    file.seek(SeekFrom::Start(SPOOL_ROTATE_BYTES)).unwrap();
    file.write_all(b"x").unwrap();
    drop(file);
    quota_ingest::rotate_spool_if_needed(&spool);
    assert!(
        spool.with_extension("jsonl.1").exists(),
        "previous generation kept"
    );
    assert!(
        !spool.exists(),
        "the live spool starts fresh (next append recreates)"
    );
    // Under the threshold: no-op.
    std::fs::write(&spool, b"small").unwrap();
    quota_ingest::rotate_spool_if_needed(&spool);
    assert!(spool.exists());
}

// ---------------------------------------------------------------------------
// Source: Claude transcript
// ---------------------------------------------------------------------------

#[test]
fn transcript_events_skip_unkeyed_and_double_usage() {
    let outcome = juggler_adapters::tail_transcript(&fixture("claude-transcript.jsonl"), "acc", 0);
    assert_eq!(outcome.parse_errors, 0);
    // a-1, a-2 kept; the no-uuid usage line skipped (cannot dedupe);
    // the repeated a-1 line (no message id: keyed by its uuid) is one event.
    assert_eq!(outcome.events.len(), 2);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|e| e.msg_uuid.as_deref() == Some("a-1"))
            .count(),
        1
    );
    let a2 = outcome
        .events
        .iter()
        .find(|e| e.msg_uuid.as_deref() == Some("a-2"))
        .unwrap();
    assert_eq!(a2.tokens.input, 200);
    assert_eq!(a2.tokens.cache_write, 2);
    assert_eq!(a2.ts, iso_to_epoch_ms("2026-10-15T20:00:02.456Z").unwrap());
    assert_eq!(a2.source, UsageSource::Transcript);
    assert_eq!(a2.confidence, Confidence::Measured);
    assert_eq!(a2.session.as_deref(), Some("sess-t"));
    assert_eq!(a2.model.as_deref(), Some("claude-opus-4"));
}

/// P3 (0.0.12 QA walk: Claude tokens ×1.8–2.5 the transcripts'). Claude
/// Code writes one line per content block — thinking, text, each tool
/// call — each with its own `uuid` but the same `message.id`,
/// `requestId` and `usage`. One API response is one event, whatever its
/// blocks; a later block's larger figure (streamed output) wins.
#[test]
fn one_response_written_as_several_content_blocks_counts_once() {
    let outcome = juggler_adapters::tail_transcript(
        &fixture("claude-transcript-content-blocks.jsonl"),
        "acc",
        0,
    );
    assert_eq!(outcome.parse_errors, 0);
    let keys: Vec<_> = outcome
        .events
        .iter()
        .map(|event| event.msg_uuid.as_deref().unwrap())
        .collect();
    assert_eq!(keys, ["msg_A/req_A", "msg_B/req_B"]);
    let sum = |class: fn(&juggler_core::model::TokenCounts) -> u64| {
        outcome
            .events
            .iter()
            .map(|event| class(&event.tokens))
            .sum::<u64>()
    };
    assert_eq!(sum(|t| t.input), 4 + 6);
    assert_eq!(sum(|t| t.output), 310 + 52);
    assert_eq!(sum(|t| t.cache_read), 12_000 + 12_800);
    assert_eq!(sum(|t| t.cache_write), 800 + 90);
    // The response's first line gives its time.
    assert_eq!(
        outcome.events[1].ts,
        iso_to_epoch_ms("2026-10-20T09:00:04.000Z").unwrap()
    );
}

/// P3, across passes and files: a response read again (a re-tail, a
/// resumed session's replayed history) is stored once.
#[test]
fn a_response_read_twice_is_stored_once() {
    let storage = Storage::open_in_memory().unwrap();
    let path = fixture("claude-transcript-content-blocks.jsonl");
    for _ in 0..2 {
        let outcome = juggler_adapters::tail_transcript(&path, "acc", 0);
        storage.usage().insert_events(&outcome.events).unwrap();
    }
    let stored = storage.usage().events_for_account("acc", 0).unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(
        stored.iter().map(|event| event.tokens.output).sum::<u64>(),
        362
    );
}

/// P3, across two tail passes: a response's first block lands in one pass
/// (output 40) and its last block in the next (output 52). The store keeps
/// the larger figure, and the second write adds only the difference.
#[test]
fn a_response_split_across_two_passes_keeps_its_final_figure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.jsonl");
    let lines: Vec<String> =
        std::fs::read_to_string(fixture("claude-transcript-content-blocks.jsonl"))
            .unwrap()
            .lines()
            .map(|line| format!("{line}\n"))
            .collect();
    // The first pass ends after b-4 (msg_B's first block, output 40).
    let split = lines
        .iter()
        .position(|line| line.contains("\"b-5\""))
        .unwrap();
    std::fs::write(&path, lines[..split].concat()).unwrap();
    let storage = Storage::open_in_memory().unwrap();
    let first = tail_transcript(&path, "acc", 0);
    storage.usage().store_events(&first.events).unwrap();
    std::fs::write(&path, lines.concat()).unwrap();
    let second = tail_transcript(&path, "acc", first.offset);
    let stored = storage.usage().store_events(&second.events).unwrap();
    assert_eq!(stored.len(), 1);
    assert!(!stored[0].new);
    assert_eq!(stored[0].added.output, 12, "only the increase is new use");
    let rows = storage.usage().events_for_account("acc", 0).unwrap();
    let msg_b = rows
        .iter()
        .find(|row| row.msg_uuid.as_deref() == Some("msg_B/req_B"))
        .unwrap();
    assert_eq!(msg_b.tokens.output, 52);
    assert_eq!(rows.iter().map(|row| row.tokens.output).sum::<u64>(), 362);
    // The same lines again change nothing.
    let again = storage.usage().store_events(&second.events).unwrap();
    assert_eq!(again[0].added, juggler_core::model::TokenCounts::default());
}

/// A resumed copy repeats its original's history under its own session id
/// (QA: 58% of one session's messages also sat in its copy). Each shared
/// message belongs to the original, whichever file is read first: the
/// same per-session split both ways, and the account total unchanged.
#[test]
fn a_shared_message_belongs_to_the_session_that_came_first() {
    let line = |session: &str, id: &str, ts: &str, output: u64| {
        format!(
            "{{\"type\":\"assistant\",\"uuid\":\"{session}-{id}\",\"requestId\":\"req_{id}\",\"sessionId\":\"{session}\",\"timestamp\":\"{ts}\",\"message\":{{\"id\":\"msg_{id}\",\"role\":\"assistant\",\"model\":\"claude-opus-4\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":{output},\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}}}}}}\n"
        )
    };
    // The original, and its copy: the same two responses, with their
    // original times, then one of its own. The copy's id sorts first, so
    // the id alone would credit it.
    let original = [
        line("sess-a", "1", "2026-10-20T09:00:00.000Z", 100),
        line("sess-a", "2", "2026-10-20T09:05:00.000Z", 200),
    ]
    .concat();
    let copy = [
        line("sess-0", "1", "2026-10-20T09:00:00.000Z", 100),
        line("sess-0", "2", "2026-10-20T09:05:00.000Z", 200),
        line("sess-0", "3", "2026-10-21T10:00:00.000Z", 400),
    ]
    .concat();
    let split = |first: &str, second: &str| {
        let storage = Storage::open_in_memory().unwrap();
        for bytes in [first, second] {
            let outcome =
                juggler_adapters::quota_ingest::transcript_events("acc", bytes.as_bytes());
            storage.usage().store_events(&outcome.events).unwrap();
        }
        let mut by_session = std::collections::BTreeMap::<String, u64>::new();
        for row in storage.usage().events_for_account("acc", 0).unwrap() {
            *by_session.entry(row.session.unwrap()).or_default() += row.tokens.output;
        }
        by_session
    };
    let original_first = split(&original, &copy);
    let copy_first = split(&copy, &original);
    assert_eq!(
        original_first, copy_first,
        "the reading order never changes the answer"
    );
    assert_eq!(
        original_first.get("sess-a"),
        Some(&300),
        "the history stays with the original"
    );
    assert_eq!(
        original_first.get("sess-0"),
        Some(&400),
        "the copy keeps only its own"
    );
    assert_eq!(
        original_first.values().sum::<u64>(),
        700,
        "the account total is unchanged"
    );
}

/// Rollout lines shaped like QA's (P17): `compacted`, then one
/// `token_count` before the next turn, which no record covers. Values are
/// invented.
fn codex_line(kind: &str, ts: &str, body: &str) -> String {
    format!(
        "{{\"timestamp\":\"2026-10-21T10:{ts}.000Z\",\"type\":\"{kind}\",\"payload\":{body}}}\n"
    )
}

fn count(ts: &str, last: u64, total: u64) -> String {
    codex_line(
        "event_msg",
        ts,
        &format!(
            r#"{{"type":"token_count","info":{{"last_token_usage":{{"input_tokens":{last},"cached_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0,"total_tokens":{last}}},"total_token_usage":{{"input_tokens":{total},"cached_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0,"total_tokens":{total}}}}},"rate_limits":null}}"#
        ),
    )
}

fn record(ts: &str, turn: &str, input: u64) -> String {
    codex_line(
        "token_usage_record",
        ts,
        &format!(
            r#"{{"thread_id":"th-sub","turn_id":"{turn}","response_id":"r-{turn}","usage":{{"input_tokens":{input},"cached_input_tokens":0,"cache_write_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0,"total_tokens":{input}}}}}"#
        ),
    )
}

fn started(ts: &str) -> String {
    codex_line("event_msg", ts, r#"{"type":"task_started"}"#)
}

/// P17: a subagent's rollout opens with `compacted`, then one token_count
/// whose total carries its parent's tokens (13.9× its own here) and whose
/// own usage no record covers. Its own usage counts, once, under the
/// thread the next record names; the parent's total never does.
#[test]
fn a_compactions_own_usage_counts_once_never_the_starting_total() {
    let bytes = [
        codex_line("compacted", "00:00", "{}"),
        count("00:01", 160, 2_224),
        started("00:02"),
        record("00:03", "t1", 100),
        count("00:04", 100, 2_324),
    ]
    .concat();
    let outcome = rollout_events("acc", bytes.as_bytes());
    assert_eq!(outcome.hold_back, None);
    let keys: Vec<_> = outcome
        .events
        .iter()
        .map(|e| e.msg_uuid.clone().unwrap())
        .collect();
    assert_eq!(keys, ["th-sub/compacted/1792576801000", "th-sub/t1/r-t1"]);
    assert_eq!(
        outcome.events.iter().map(|e| e.tokens.input).sum::<u64>(),
        260,
        "each turn's own usage: 160 + 100, never the 2,224 it started with"
    );
}

/// A compaction mid-thread whose token_count only repeats the total
/// before it adds nothing.
#[test]
fn a_compaction_that_repeats_the_total_adds_nothing() {
    let bytes = [
        started("00:00"),
        record("00:01", "t1", 100),
        count("00:02", 100, 500),
        codex_line("compacted", "00:03", "{}"),
        count("00:04", 100, 500),
        started("00:05"),
        record("00:06", "t2", 50),
    ]
    .concat();
    let outcome = rollout_events("acc", bytes.as_bytes());
    assert_eq!(outcome.events.len(), 2, "the two records, no compaction");
}

/// A pass that ends before the compaction's thread is known holds those
/// lines back; the next pass reads them again and counts the usage once.
#[test]
fn a_compaction_split_across_passes_counts_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollout.jsonl");
    let first = [
        codex_line("compacted", "00:00", "{}"),
        count("00:01", 160, 2_224),
    ]
    .concat();
    std::fs::write(&path, &first).unwrap();
    let storage = Storage::open_in_memory().unwrap();
    let pass1 = tail_rollout(&path, "acc", 0);
    assert!(pass1.events.is_empty());
    assert_eq!(pass1.offset, 0, "held back to the compaction");
    storage.usage().store_events(&pass1.events).unwrap();
    std::fs::write(
        &path,
        [first, started("00:02"), record("00:03", "t1", 100)].concat(),
    )
    .unwrap();
    let pass2 = tail_rollout(&path, "acc", pass1.offset);
    storage.usage().store_events(&pass2.events).unwrap();
    let pass3 = tail_rollout(&path, "acc", pass2.offset);
    assert!(pass3.events.is_empty(), "nothing read twice");
    let rows = storage.usage().events_for_account("acc", 0).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows.iter().map(|row| row.tokens.input).sum::<u64>(), 260);
}

#[test]
fn iso_timestamps_parse_strictly() {
    assert_eq!(
        iso_to_epoch_ms("2026-10-15T20:00:02.456Z"),
        Some(1_792_094_402_456)
    );
    assert_eq!(iso_to_epoch_ms("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(iso_to_epoch_ms("2026-13-01T00:00:00Z"), None);
    assert_eq!(iso_to_epoch_ms("not-a-date"), None);
}

// ---------------------------------------------------------------------------
// Source: Codex rollout
// ---------------------------------------------------------------------------

#[test]
fn rollout_events_read_only_the_response_level_usage() {
    let outcome = rollout_events(
        "acc",
        &std::fs::read(fixture("codex-rollout.jsonl")).unwrap(),
    );
    assert_eq!(outcome.parse_errors, 0);
    // Only the two token_usage_record lines; meta + event_msg skipped.
    assert_eq!(outcome.events.len(), 2);
    let first = &outcome.events[0];
    assert_eq!(
        first.msg_uuid.as_deref(),
        Some("th-1/tu-1/re-1"),
        "dedupe key survives re-tails and resumed rollouts"
    );
    // Disjoint classes: input without its cached part, output without
    // its reasoning (Codex nests them).
    assert_eq!(first.tokens.input, 52_381 - 5_888);
    assert_eq!(first.tokens.cache_read, 5_888); // cached_input_tokens
    assert_eq!(first.tokens.reasoning, 136); // reasoning_output_tokens
    assert_eq!(first.session.as_deref(), Some("th-1"));
    assert_eq!(first.source, UsageSource::Rollout);
    // One response-level read only — rollup fields never re-counted.
    assert_eq!(first.tokens.output, 200 - 136);
}

/// P14 (0.0.12 QA walk: Codex tokens exactly ×2). Codex's
/// `input_tokens` includes `cached_input_tokens` and its `output_tokens`
/// includes `reasoning_output_tokens`; storing all four counted each
/// token twice. Every token once: the stored classes sum to Codex's own
/// `total_tokens`.
#[test]
fn codex_tokens_are_counted_once() {
    let outcome = rollout_events(
        "acc",
        &std::fs::read(fixture("codex-rollout.jsonl")).unwrap(),
    );
    // Every stored class, cache writes included (the second line has 50).
    assert_eq!(outcome.events[1].tokens.cache_write, 50);
    for (event, total) in outcome.events.iter().zip([52_581_u64, 1_280]) {
        let t = &event.tokens;
        assert_eq!(
            t.input + t.cache_read + t.cache_write + t.output + t.reasoning,
            total
        );
    }
}

/// P15(a): a rollout's usage lines carry no model; the turn's
/// `turn_context` does. Usage before any `turn_context` in the slice has
/// none here (the pipeline carries the previous slice's).
#[test]
fn codex_usage_takes_the_model_its_turn_context_names() {
    let outcome = rollout_events(
        "acc",
        &std::fs::read(fixture("codex-rollout.jsonl")).unwrap(),
    );
    let models: Vec<_> = outcome
        .events
        .iter()
        .map(|event| event.model.as_deref())
        .collect();
    assert_eq!(models, [None, Some("gpt-5-codex")]);
    assert_eq!(outcome.model.as_deref(), Some("gpt-5-codex"));
}

/// The 0.0.12 report: Codex's rollouts carry its own limit readings on
/// `event_msg` `token_count` lines (`rate_limits`: primary, secondary,
/// credits). The fixture is shaped like the real line; every value is
/// invented.
#[test]
fn rollout_token_counts_become_measured_samples_and_the_credits() {
    let outcome = rollout_events(
        "acc",
        &std::fs::read(fixture("codex-rollout-rate-limits.jsonl")).unwrap(),
    );
    // A line of an unknown window width yields nothing, never guessed
    // into a pool; it is an odd reading, not a broken line. A null
    // rate_limits is just skipped.
    assert_eq!(outcome.parse_errors, 0);
    assert_eq!(
        outcome.events.len(),
        1,
        "tokens still come from token_usage_record"
    );
    let first_ts = iso_to_epoch_ms("2026-10-21T10:01:00.000Z").unwrap();
    let later_ts = iso_to_epoch_ms("2026-10-21T10:30:00.000Z").unwrap();
    let samples: Vec<(WindowKind, f64, Option<i64>, i64)> = outcome
        .samples
        .iter()
        .map(|(kind, sample)| (*kind, sample.used_pct, sample.resets_at, sample.ts))
        .collect();
    assert_eq!(
        samples,
        vec![
            (
                WindowKind::FiveHour,
                12.5,
                Some(1_792_580_400_000),
                first_ts
            ),
            (
                WindowKind::Weekly { anchor_ms: None },
                40.0,
                Some(1_792_990_000_000),
                first_ts
            ),
            // resets_in_seconds (older builds) counts from the line.
            (
                WindowKind::FiveHour,
                20.0,
                Some(later_ts + 3_600_000),
                later_ts
            ),
        ]
    );
    assert!(
        outcome
            .samples
            .iter()
            .all(|(_, sample)| sample.confidence == Confidence::Measured)
    );
    // The newest credits snapshot wins.
    let credits = outcome.credits.expect("credits");
    assert_eq!(credits.ts, later_ts);
    assert!(credits.has_credits && !credits.unlimited);
    assert_eq!(credits.balance.as_deref(), Some("16.25"));
}

#[test]
fn codex_rate_limits_take_seconds_or_milliseconds_and_skip_odd_widths() {
    let ts = 1_792_500_000_000;
    let (samples, credits) = quota_ingest::codex_rate_limits(
        ts,
        &serde_json::json!({
            "primary": {"used_percent": 150.0, "window_minutes": 300, "resets_at": 1_792_580_400_000_i64},
            "secondary": {"used_percent": 1.0, "window_minutes": 1440},
            "credits": {"has_credits": true, "unlimited": true, "balance": null}
        }),
    );
    assert_eq!(
        samples.len(),
        1,
        "a 1-day window isn't a pool AI Juggler knows"
    );
    assert_eq!(samples[0].1.used_pct, 100.0, "clamped");
    assert_eq!(
        samples[0].1.resets_at,
        Some(1_792_580_400_000),
        "ms passes through"
    );
    let credits = credits.unwrap();
    assert!(credits.unlimited);
    assert_eq!(credits.balance, None);
}

/// QA's 30 days of the real Codex account (2026-10-21): only `primary`,
/// weekly (`window_minutes` 10080); `secondary` never appears; the plan
/// can be null; odd readings carry `window_minutes` 0 or a null
/// `primary`. Odd readings give no sample; a null plan is fine; the
/// credits still count. Shapes from that analysis, values invented.
#[test]
fn codex_odd_readings_are_ignored_and_a_null_plan_is_fine() {
    let line = |ts: &str, limits: &str| {
        format!(
            "{{\"timestamp\":\"{ts}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":null,\"rate_limits\":{limits}}}}}\n"
        )
    };
    let credits = r#""credits":{"has_credits":true,"unlimited":false,"balance":"12.50"}"#;
    let bytes = [
        line(
            "2026-10-21T10:00:00.000Z",
            &format!(r#"{{"limit_id":"codex","plan_type":null,"primary":{{"used_percent":64,"window_minutes":10080,"resets_at":1793000000}},"secondary":null,{credits}}}"#),
        ),
        line(
            "2026-10-21T10:00:14.000Z",
            &format!(r#"{{"limit_id":"codex","plan_type":"pro","primary":{{"used_percent":0,"window_minutes":0,"resets_at":1793000000}},"secondary":null,{credits}}}"#),
        ),
        line(
            "2026-10-21T10:00:28.000Z",
            &format!(r#"{{"limit_id":"codex","plan_type":"pro","primary":null,"secondary":null,{credits}}}"#),
        ),
    ]
    .concat();
    let outcome = rollout_events("acc", bytes.as_bytes());
    assert_eq!(
        outcome.samples.len(),
        1,
        "one reading: the odd ones give none"
    );
    let (kind, sample) = &outcome.samples[0];
    assert_eq!(*kind, WindowKind::Weekly { anchor_ms: None });
    assert_eq!(sample.used_pct, 64.0);
    assert_eq!(sample.resets_at, Some(1_793_000_000_000));
    assert_eq!(
        outcome.parse_errors, 0,
        "an odd reading is not a broken line"
    );
    // Not even with no credits beside it; a window without its figure is.
    let odd = line(
        "2026-10-21T10:00:42.000Z",
        r#"{"limit_id":"codex","primary":{"used_percent":0,"window_minutes":0},"secondary":null,"credits":null}"#,
    );
    assert_eq!(rollout_events("acc", odd.as_bytes()).parse_errors, 0);
    let broken = line(
        "2026-10-21T10:00:56.000Z",
        r#"{"limit_id":"codex","primary":{"window_minutes":10080},"secondary":null,"credits":null}"#,
    );
    assert_eq!(rollout_events("acc", broken.as_bytes()).parse_errors, 1);
    assert_eq!(
        outcome.credits.as_ref().and_then(|c| c.balance.as_deref()),
        Some("12.50")
    );
}

// ---------------------------------------------------------------------------
// Source: ZCode SQLite (read-only)
// ---------------------------------------------------------------------------

fn zcode_fixture_db() -> PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db.sqlite");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(&std::fs::read_to_string(fixture("zcode-model-usage.sql")).unwrap())
        .unwrap();
    drop(conn);
    tempfile_dir_keep(dir.keep());
    db
}

/// Keeps the tempdir alive by leaking it (fixture DBs must outlive the
/// helper; test-scoped, OS-cleaned).
fn tempfile_dir_keep(path: PathBuf) {
    Box::leak(Box::new(path));
}

#[test]
fn zcode_events_compute_credits_with_peak_and_offpeak() {
    let db = zcode_fixture_db();
    let events = zcode_events("acc", &db).unwrap();
    // 3 completed rows (peak, off-peak, unknown-model); the failed row
    // is excluded.
    assert_eq!(events.len(), 3);
    let peak = events
        .iter()
        .find(|e| e.tokens.input == 480_347)
        .expect("peak row");
    assert!(zcode_is_peak(peak.ts));
    // fresh=923, cached=479424, out=479 (GLM-5.3 multipliers).
    let expected = (923.0 * 6.9 + 479_424.0 * 1.7 + 479.0 * 24.0) / 10_000.0;
    assert!((peak.credits.unwrap() - expected).abs() < 1e-9);
    assert_eq!(peak.confidence, Confidence::Computed);
    assert_eq!(peak.source, UsageSource::Sqlite);
    assert!(
        peak.msg_uuid
            .as_deref()
            .is_some_and(|uuid| uuid.starts_with("zcode:"))
    );

    let offpeak = events.iter().find(|e| e.tokens.input == 1_000).unwrap();
    assert!(!zcode_is_peak(offpeak.ts));
    let expected_off = (1_000.0 * 6.9 + 0.0 + 100.0 * 24.0) / 10_000.0 * 0.5;
    assert!((offpeak.credits.unwrap() - expected_off).abs() < 1e-9);

    let unknown = events
        .iter()
        .find(|e| e.model.as_deref() == Some("GLM-6-X"))
        .unwrap();
    assert!(
        unknown.credits.is_none(),
        "no published multiplier ⇒ no credits"
    );
    assert_eq!(unknown.confidence, Confidence::Estimated);
}

// ---------------------------------------------------------------------------
// Gemini: the typed absence
// ---------------------------------------------------------------------------

#[test]
fn gemini_has_no_emitter_by_design() {
    assert!(gemini_events().is_empty());
}

// ---------------------------------------------------------------------------
// Statusline install/uninstall + chaining (Req.1)
// ---------------------------------------------------------------------------

fn write_settings(dir: &Path, statusline: Option<serde_json::Value>) {
    let mut settings = serde_json::json!({"model": "claude-opus-4"});
    if let Some(value) = statusline {
        settings["statusLine"] = value;
    }
    std::fs::write(
        dir.join("settings.json"),
        serde_json::to_vec_pretty(&settings).unwrap(),
    )
    .unwrap();
}

#[test]
fn install_chains_and_uninstall_restores_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let user_command = format!("{} marker", fixture("user-statusline.sh").to_string_lossy());
    let original = serde_json::json!({
        "type": "command",
        "command": user_command,
        "padding": 12,
    });
    write_settings(dir.path(), Some(original.clone()));

    assert!(
        install_statusline(dir.path()).unwrap(),
        "first install acts"
    );
    assert!(
        !install_statusline(dir.path()).unwrap(),
        "second install is a no-op"
    );
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap()).unwrap();
    let wrapper = dir.path().join("juggler-statusline.sh");
    assert_eq!(
        settings["statusLine"]["command"].as_str().unwrap(),
        format!("'{}'", wrapper.to_string_lossy()),
        "one shell word"
    );
    assert!(dir.path().join("juggler-statusline-backup.json").exists());
    // The wrapper embeds the user's command as the chain.
    let script = std::fs::read_to_string(&wrapper).unwrap();
    assert!(script.contains("user-statusline.sh"), "chain is embedded");

    assert!(uninstall_statusline(dir.path()).unwrap());
    let restored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        restored["statusLine"], original,
        "byte-for-byte value restore"
    );
    assert!(!wrapper.exists());
    assert!(!dir.path().join("juggler-statusline-backup.json").exists());
    assert!(
        !uninstall_statusline(dir.path()).unwrap(),
        "second uninstall is a no-op"
    );
}

#[test]
fn install_over_nothing_uninstalls_to_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write_settings(dir.path(), None);
    assert!(install_statusline(dir.path()).unwrap());
    assert!(uninstall_statusline(dir.path()).unwrap());
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap()).unwrap();
    assert!(
        settings.get("statusLine").is_none(),
        "the key is removed, exactly as before"
    );
}

/// The chaining gate (deep/09 §Q9): the wrapper runs the user's own
/// statusline with the SAME stdin bytes — its output is
/// byte-identical to running the user statusline directly, while the
/// spool ALSO captures the payload. Both outputs visible, no
/// duplication.
#[test]
fn statusline_wrapper_chains_user_script_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let user_command = fixture("user-statusline.sh").to_string_lossy().to_string();
    let spool = spool_path(dir.path());
    std::fs::create_dir_all(spool.parent().unwrap()).unwrap();
    let script = quota_ingest::statusline_script(&spool, Some(&user_command));
    let wrapper = dir.path().join("wrap.sh");
    std::fs::write(&wrapper, script).unwrap();

    let payload = r#"{"session_id":"s","rate_limits":{"five_hour":{"used_percentage":7.0}}}"#;
    let chained = std::process::Command::new("/bin/sh")
        .arg(&wrapper)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(payload.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .unwrap();
    let direct = std::process::Command::new("/bin/sh")
        .arg(fixture("user-statusline.sh"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(payload.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .unwrap();
    assert_eq!(
        chained.stdout, direct.stdout,
        "user statusline output is byte-for-byte"
    );
    let spool_text = std::fs::read_to_string(&spool).unwrap();
    assert!(
        spool_text.contains("\"used_percentage\":7.0"),
        "spool captured the payload"
    );
    assert!(spool_text.contains('\t'), "envelope carries the timestamp");
}

// ---------------------------------------------------------------------------
// Idempotency at the store level (Req.3)
// ---------------------------------------------------------------------------

#[test]
fn re_ingesting_the_same_fixtures_inserts_zero_new_rows() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open_in_memory().unwrap();
    let run_once = |path: &Path| {
        let outcome = tail_transcript(path, "acc", 0);
        storage.usage().insert_events(&outcome.events).unwrap();
    };
    run_once(&fixture("claude-transcript.jsonl"));
    let first = storage.usage().events_for_account("acc", 0).unwrap().len();
    run_once(&fixture("claude-transcript.jsonl"));
    let second = storage.usage().events_for_account("acc", 0).unwrap().len();
    assert_eq!(
        first, second,
        "the resumed-file duplicate is absorbed by uuid dedupe"
    );
    assert_eq!(first, 2, "a-1 and a-2 once each");
    let _ = dir;
}

// ---------------------------------------------------------------------------
// The no-HTTP law (Quality gate: grep + review checklist)
// ---------------------------------------------------------------------------

/// No ingest code path may talk to a Provider: the crate carries no
/// HTTP client dependency and no client construction exists anywhere
/// in its sources (doc URLs are fine; client APIs are not).
#[test]
fn no_http_client_anywhere_in_the_adapters_crate() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    for banned in ["reqwest", "ureq", "isahc", "curl", "hyper"] {
        assert!(
            !manifest.contains(banned),
            "Cargo.tomt must not depend on {banned}"
        );
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).unwrap();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_some_and(|extension| extension == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                for banned in [
                    "reqwest::",
                    "ureq::",
                    "isahc::",
                    "curl::",
                    "hyper::Client",
                    "ClientBuilder",
                    "TcpStream::connect",
                    "UdpSocket::",
                ] {
                    assert!(
                        !text.contains(banned),
                        "{} contains forbidden HTTP/network client: {banned}",
                        path.display()
                    );
                }
            }
        }
    }
}

/// Runs a status line `command` the way Claude Code does — through the
/// shell — with `payload` on stdin.
fn run_status_command(command: &str, payload: &str) -> std::process::Output {
    use std::io::Write as _;
    let mut child = std::process::Command::new("/bin/sh")
        .args(["-c", command])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// Review of ADR-0018: AI Juggler's data folder is under "Application
/// Support". The installed command was the bare path, so the shell split
/// it at the space and the wrapper never ran — no quota was ever captured
/// on a real Mac. Installed into a folder with a space (and a quote), the
/// command now runs, captures, and still shows the user's own line.
#[test]
fn the_status_line_runs_from_a_folder_with_spaces() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("Application Support/AI Juggler/it's here");
    std::fs::create_dir_all(&dir).unwrap();
    let user_command = format!("{} marker", fixture("user-statusline.sh").to_string_lossy());
    write_settings(
        &dir,
        Some(serde_json::json!({ "type": "command", "command": user_command })),
    );
    install_statusline(&dir).unwrap();
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("settings.json")).unwrap()).unwrap();
    let command = settings["statusLine"]["command"].as_str().unwrap();
    let payload = r#"{"session_id":"s","rate_limits":{"five_hour":{"used_percentage":9.0}}}"#;
    let output = run_status_command(command, payload);
    assert!(output.status.success(), "{output:?}");
    let direct = run_status_command(&user_command, payload);
    assert_eq!(output.stdout, direct.stdout, "the user's line still shows");
    let spool = std::fs::read_to_string(spool_path(&dir)).unwrap();
    assert!(
        spool.contains("\"used_percentage\":9.0"),
        "captured: {spool}"
    );
}

/// An install from before the fix (the bare path) is re-pointed, and the
/// user's own status line — still in the backup — stays chained.
#[test]
fn an_unquoted_install_is_repaired_and_keeps_the_chain() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("Application Support/acct");
    std::fs::create_dir_all(&dir).unwrap();
    let user_command = format!("{} marker", fixture("user-statusline.sh").to_string_lossy());
    write_settings(
        &dir,
        Some(serde_json::json!({ "type": "command", "command": user_command })),
    );
    install_statusline(&dir).unwrap();
    // Make it look like the old install: the bare path in settings.json.
    let wrapper = dir.join("juggler-statusline.sh");
    write_settings(
        &dir,
        Some(serde_json::json!({ "type": "command", "command": wrapper.to_string_lossy() })),
    );
    assert!(install_statusline(&dir).unwrap(), "repaired");
    let script = std::fs::read_to_string(&wrapper).unwrap();
    assert!(
        script.contains("user-statusline.sh"),
        "the user's line is still chained: {script}"
    );
    assert!(uninstall_statusline(&dir).unwrap());
    let restored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        restored["statusLine"]["command"].as_str(),
        Some(user_command.as_str())
    );
}

/// Review of ADR-0018: a backup that holds one of our own wrappers (a
/// mixed-version install) must never be chained — the wrapper would run
/// inside itself on every refresh.
#[test]
fn our_own_wrapper_is_never_chained() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("Application Support/acct");
    std::fs::create_dir_all(&dir).unwrap();
    let wrapper = dir.join("juggler-statusline.sh");
    write_settings(
        &dir,
        Some(
            serde_json::json!({ "type": "command", "command": format!("'{}'", wrapper.display()) }),
        ),
    );
    // settings.json points at a quoted wrapper of ours that isn't the
    // expected command form (an older/newer build): treat it as ours.
    std::fs::write(
        dir.join("juggler-statusline-backup.json"),
        serde_json::to_vec(&serde_json::json!({ "statusLine": { "type": "command", "command": format!("'{}'", wrapper.display()) } })).unwrap(),
    )
    .unwrap();
    write_settings(
        &dir,
        Some(serde_json::json!({ "type": "command", "command": wrapper.to_string_lossy() })),
    );
    install_statusline(&dir).unwrap();
    let script = std::fs::read_to_string(&wrapper).unwrap();
    assert!(
        !script.contains("CHAIN="),
        "no chain to ourselves: {script}"
    );
}
