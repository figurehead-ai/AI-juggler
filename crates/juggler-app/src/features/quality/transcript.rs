//! A session's quality signals, read from its transcript.

use std::sync::Arc;

use juggler_core::model::{SessionId, SessionRecord, Timestamp};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::sessions::dto::{AttentionInfo, QualitySignalsInfo, TokenCountsInfo};
use crate::state::AppState;

/// The model a transcript line records for its turn — top-level
/// `model` or nested `message.model` (both shapes ship in the wild), or
/// a Codex rollout's `turn_context` payload (AJ-E1303).
pub(crate) fn line_model(value: &serde_json::Value) -> Option<String> {
    value
        .get("model")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            value
                .get("message")
                .and_then(|message| message.get("model"))
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| {
            (value.get("type").and_then(serde_json::Value::as_str) == Some("turn_context"))
                .then(|| value.get("payload")?.get("model")?.as_str())
                .flatten()
        })
        .map(str::to_string)
}

/// AJ-E1303: one Codex rollout line's contribution to the quality scan —
/// `token_usage_record` rows are turns with tokens, `token_count` events
/// carry the context pair, `error` events count as errors.
fn scan_rollout_line(
    value: &serde_json::Value,
    payload: &serde_json::Value,
    model: &mut Option<String>,
    scan: &mut TranscriptScan,
    now: i64,
) {
    let kind = value.get("type").and_then(serde_json::Value::as_str);
    let inner = payload.get("type").and_then(serde_json::Value::as_str);
    match (kind, inner) {
        (Some("turn_context"), _) => {
            if let Some(name) = payload.get("model").and_then(serde_json::Value::as_str) {
                *model = Some(name.to_owned());
            }
        }
        (Some("token_usage_record"), _) => {
            // Once per response, as the parser and the usage pipeline count.
            let key = ["thread_id", "turn_id", "response_id"]
                .map(|field| payload[field].as_str().unwrap_or_default())
                .join("/");
            if !scan.rollout_seen.insert(key) {
                return;
            }
            let usage = &payload["usage"];
            scan.turns += 1;
            scan.events.push(juggler_core::quality::UsageEvent {
                at_ms: value["timestamp"]
                    .as_str()
                    .and_then(juggler_core::time::timestamp_ms)
                    .unwrap_or(now),
                // Disjoint classes, split as the usage pipeline does.
                tokens: juggler_adapters::quota_ingest::codex_token_counts(usage),
                model: model.clone(),
            });
        }
        (Some("event_msg"), Some("token_count")) => {
            let info = &payload["info"];
            if let Some(used) = info["last_token_usage"]["total_tokens"].as_u64() {
                scan.context_used = Some(used);
                scan.context_window = info["model_context_window"].as_u64();
            }
        }
        (Some("event_msg"), Some("error" | "stream_error")) => scan.errors += 1,
        _ => {}
    }
}

/// Aggregated transcript facts feeding the quality signals.
pub(crate) struct TranscriptScan {
    events: Vec<juggler_core::quality::UsageEvent>,
    pub(crate) errors: u64,
    pub(crate) turns: u64,
    pub(crate) context_used: Option<u64>,
    pub(crate) context_window: Option<u64>,
    /// Codex usage rows already counted (thread/turn/response): a forked
    /// or resumed rollout repeats its parent's rows.
    rollout_seen: std::collections::BTreeSet<String>,
}

/// Scans the native transcript JSONL for per-turn usage events,
/// timestamps, error markers and the last reported context pair.
/// Fixture-compatible shapes; Claude transcripts carry `timestamp` +
/// `usage` on assistant lines.
pub(crate) fn scan_transcript(record: &SessionRecord, now: i64) -> TranscriptScan {
    let mut scan = TranscriptScan {
        events: Vec::new(),
        errors: 0,
        turns: 0,
        context_used: None,
        context_window: None,
        rollout_seen: std::collections::BTreeSet::new(),
    };
    // A line at a time: the session page asks often, and a transcript can
    // be gigabytes.
    let lines = record
        .transcript_ref
        .as_ref()
        .and_then(|transcript| juggler_adapters::session_lines(&transcript.path).ok());
    let Some(lines) = lines else {
        return scan;
    };
    let mut rollout_model = None;
    for (_, line) in lines {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) else {
            continue;
        };
        if let Some(payload) = value.get("payload") {
            scan_rollout_line(&value, payload, &mut rollout_model, &mut scan, now);
            continue;
        }
        if value.get("usage").is_none() {
            continue;
        }
        scan.turns += 1;
        let usage = &value["usage"];
        let tokens = juggler_core::model::TokenCounts {
            input: usage["input_tokens"].as_u64().unwrap_or(0),
            output: usage["output_tokens"].as_u64().unwrap_or(0),
            cache_read: 0,
            cache_write: 0,
            reasoning: 0,
        };
        // Wall clock: RFC3339 `timestamp` when present, else `now`
        // (honest single-point fallback → the per-minute bars degrade
        // to one bucket, never fabricated).
        let at_ms = value["timestamp"]
            .as_str()
            .and_then(juggler_core::time::timestamp_ms)
            .unwrap_or(now);
        scan.events.push(juggler_core::quality::UsageEvent {
            at_ms,
            tokens,
            model: line_model(&value),
        });
        if value.get("error").is_some() {
            scan.errors += 1;
        }
        if let Some(used) = value["context_tokens"].as_u64() {
            scan.context_used = Some(used);
            scan.context_window = value["context_window"].as_u64();
        }
    }
    scan
}

/// [`instance_quality`](crate::features::quality::commands::instance_quality)'s work. Storage is held only to read the
/// session and its tool; the transcript (it can be gigabytes) is read
/// after, so nothing else waits on it.
///
/// # Errors
///
/// As [`instance_quality`](crate::features::quality::commands::instance_quality).
pub fn instance_quality_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<QualitySignalsInfo> {
    let (record, provider, stuck_episodes) = {
        let storage = lock_storage_arc(state)?;
        let record = storage
            .sessions()
            .get(&SessionId::new(session_id))
            .map_err(AppError::from)?;
        let stuck_episodes = storage
            .sessions()
            .transitions(&record.id)
            .map_or(0, |rows| rows.iter().filter(|row| row.1 == "stuck").count());
        let recipe = storage.profiles().get(&record.profile).ok();
        let provider = storage
            .accounts()
            .list()
            .ok()
            .and_then(|accounts| {
                accounts
                    .iter()
                    .find(|a| recipe.as_ref().is_some_and(|r| r.account == a.id))
                    .map(|a| a.harness.to_string())
            })
            .unwrap_or_default();
        (record, provider, stuck_episodes)
    };
    let now = Timestamp::now().millis();
    let scan = scan_transcript(&record, now);

    let tpm = juggler_core::quality::signals::tokens_per_min(&scan.events, now);
    let error_label = if scan.turns == 0 {
        "Not measurable (interactive)".to_string()
    } else {
        juggler_core::quality::ErrorRate::words(scan.errors, scan.turns)
    };
    let table = juggler_core::quality::load_pricing_table();
    // AJ-E412 Req.4: price per turn where the transcript records the
    // model; turns without one stay in the token totals but are
    // excluded from $. Falls back to tokens-only when no turn was
    // priceable (never to the launch overlay — that is intent, not
    // telemetry).
    let (cost, priced_model) = match juggler_core::quality::pricing::session_cost_per_turn(
        &table,
        &provider,
        &scan.events,
    ) {
        Some((total, models)) => (Some(total), Some(models.join(", "))),
        None => (None, None),
    };
    // Forecast: from quota deltas — none are observable yet, so idle
    // honestly until the usage engine (AJ-E606+) feeds burn rates.
    let forecast = juggler_core::quality::quota_forecast_label(
        record
            .usage
            .window
            .as_ref()
            .map_or(0.0, |w| w.used_percentage),
        0.0,
    );
    Ok(QualitySignalsInfo {
        tokens_bars: tpm.bars.map(|bars| bars.to_vec()),
        avg_tokens_per_min: tpm.avg_per_min,
        error_label,
        errors_not_measurable: scan.turns == 0,
        context_used: scan.context_used,
        context_window: scan.context_window,
        cumulative: TokenCountsInfo {
            input: record.usage.tokens.input,
            output: record.usage.tokens.output,
            cache_read: record.usage.tokens.cache_read,
            cache_write: record.usage.tokens.cache_write,
            reasoning: record.usage.tokens.reasoning,
        },
        session_cost_usd: cost,
        priced_model: priced_model.filter(|_| cost.is_some()),
        quota_forecast: forecast,
        stuck: record.attention.as_ref().map(|reason| AttentionInfo {
            rule: reason.rule.clone(),
            inputs: reason.inputs.clone(),
            since_ms: reason.since_ms,
        }),
        stuck_episodes: u32::try_from(stuck_episodes).unwrap_or(u32::MAX),
        end_context: record
            .usage
            .at_end
            .and_then(|end| end.context_used.zip(end.context_window)),
    })
}

#[cfg(test)]
mod rollout_scan_tests {
    use super::{TranscriptScan, scan_rollout_line};

    /// An empty scan, and one Codex usage row: 100 in (40 cached), 10 out
    /// (4 of it reasoning), 110 in all.
    fn empty_scan_and_usage_row() -> (TranscriptScan, serde_json::Value) {
        let scan = TranscriptScan {
            events: Vec::new(),
            errors: 0,
            turns: 0,
            context_used: None,
            context_window: None,
            rollout_seen: std::collections::BTreeSet::new(),
        };
        let row = serde_json::json!({
            "timestamp": "2026-10-20T08:00:00.000Z",
            "type": "token_usage_record",
            "payload": {"thread_id": "t", "turn_id": "1", "response_id": "r",
                        "usage": {"input_tokens": 100, "cached_input_tokens": 40, "output_tokens": 10,
                                  "reasoning_output_tokens": 4, "total_tokens": 110}}
        });
        (scan, row)
    }

    #[test]
    fn a_repeated_usage_row_counts_once_in_the_quality_scan() {
        let (mut scan, row) = empty_scan_and_usage_row();
        let mut model = None;
        for _ in 0..3 {
            scan_rollout_line(&row, &row["payload"], &mut model, &mut scan, 0);
        }
        assert_eq!(
            scan.turns, 1,
            "a forked/resumed rollout's repeat is not a turn"
        );
        assert_eq!(scan.events.len(), 1);
        assert_eq!(scan.events[0].tokens.input, 60);
    }

    /// P14 (0.0.12 QA): Codex's output includes its reasoning; the scan
    /// counted the reasoning twice. Every token once: the classes sum to
    /// Codex's own `total_tokens`.
    #[test]
    fn the_quality_scan_counts_each_codex_token_once() {
        let (mut scan, row) = empty_scan_and_usage_row();
        scan_rollout_line(&row, &row["payload"], &mut None, &mut scan, 0);
        let tokens = &scan.events[0].tokens;
        assert_eq!((tokens.output, tokens.reasoning), (6, 4));
        assert_eq!(
            tokens.input + tokens.cache_read + tokens.output + tokens.reasoning,
            110
        );
    }
}
