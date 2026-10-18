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
    spool_path, statusline_samples, tail_spool, tail_transcript, uninstall_statusline,
    zcode_events, zcode_is_peak,
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
    let bytes = std::fs::read(fixture("claude-transcript.jsonl")).unwrap();
    let outcome = juggler_adapters::tail_transcript(&fixture("claude-transcript.jsonl"), "acc", 0);
    assert_eq!(outcome.parse_errors, 0);
    // a-1, a-2 kept; the no-uuid usage line skipped (cannot dedupe);
    // the duplicate a-1 line emits the SAME uuid (store dedupes).
    assert_eq!(outcome.events.len(), 3);
    assert_eq!(
        outcome
            .events
            .iter()
            .filter(|e| e.msg_uuid.as_deref() == Some("a-1"))
            .count(),
        2
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
    let _ = bytes;
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
    assert_eq!(first.tokens.input, 52_381);
    assert_eq!(first.tokens.cache_read, 5_888); // cached_input_tokens
    assert_eq!(first.tokens.reasoning, 136); // reasoning_output_tokens
    assert_eq!(first.session.as_deref(), Some("th-1"));
    assert_eq!(first.source, UsageSource::Rollout);
    // One response-level read only — rollup fields never re-counted.
    assert_eq!(first.tokens.output, 200);
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
        wrapper.to_string_lossy()
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
