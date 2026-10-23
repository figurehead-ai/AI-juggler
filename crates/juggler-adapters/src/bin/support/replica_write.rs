//! `fake-harness write` — a live session for the replica (AJ-E1531).
//!
//! Copied into a replica HOME under the names the app's process reader
//! knows (`claude`, `codex`), it appends real-shaped lines to one
//! transcript while a walk runs, so the app sees a live session started
//! outside it:
//!
//! ```text
//! fake-harness write --format claude|codex --file PATH --cwd DIR
//!                    --session-id ID [--every-ms MS] [--used FIVE WEEK]
//!                    [--idle]
//! ```
//!
//! `--idle` runs without writing: a resumed session left waiting, whose
//! conversation only its `--resume` id names (nothing newer is written).
//!
//! `claude` writes Claude Code's `projects/<cwd>/<id>.jsonl` lines (user
//! and assistant turns, with `message.usage`); `codex` writes a rollout
//! (`session_meta`, then `turn_context`, `token_usage_record` and
//! `token_count` with `rate_limits`, from `--used`). The content is
//! placeholder text, never real. It exits when whoever started it goes
//! away, so a walk never leaves one behind.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct WriteOptions {
    format: String,
    file: PathBuf,
    cwd: String,
    session_id: String,
    every_ms: u64,
    used: (f64, f64),
    /// The first thing the user asks (a fresh session's title).
    ask: Option<String>,
    /// Running, writing nothing.
    idle: bool,
    /// When this run started (epoch s): a resumed thread's new responses
    /// get ids of their own.
    started: u64,
}

fn parse(args: &[String]) -> Result<WriteOptions, String> {
    let mut opts = WriteOptions {
        format: String::new(),
        file: PathBuf::new(),
        cwd: String::new(),
        session_id: String::new(),
        every_ms: 4000,
        used: (0.0, 0.0),
        ask: None,
        idle: false,
        started: now_secs(),
    };
    let mut i = 0;
    while i < args.len() {
        let value = |at: usize| {
            args.get(at)
                .cloned()
                .ok_or_else(|| format!("missing value for {}", args[i]))
        };
        if args[i] == "--idle" {
            opts.idle = true;
            i += 1;
            continue;
        }
        match args[i].as_str() {
            "--format" => opts.format = value(i + 1)?,
            "--file" => opts.file = PathBuf::from(value(i + 1)?),
            "--cwd" => opts.cwd = value(i + 1)?,
            "--session-id" => opts.session_id = value(i + 1)?,
            "--ask" => opts.ask = Some(value(i + 1)?),
            "--every-ms" => {
                opts.every_ms = value(i + 1)?
                    .parse()
                    .map_err(|e| format!("--every-ms: {e}"))?;
            }
            "--used" => {
                let five = value(i + 1)?.parse().map_err(|e| format!("--used: {e}"))?;
                let week = value(i + 2)?.parse().map_err(|e| format!("--used: {e}"))?;
                opts.used = (five, week);
                i += 1;
            }
            other => return Err(format!("unknown option {other}")),
        }
        i += 2;
    }
    if !matches!(opts.format.as_str(), "claude" | "codex") {
        return Err("--format is claude or codex".into());
    }
    if opts.file.as_os_str().is_empty() || opts.session_id.is_empty() {
        return Err("--file and --session-id are required".into());
    }
    Ok(opts)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// RFC 3339 in UTC with milliseconds, from epoch seconds (no date crate:
/// civil-from-days, Howard Hinnant's algorithm).
fn stamp(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.000Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn claude_lines(
    opts: &WriteOptions,
    turn: u64,
    parent: &mut Option<String>,
) -> Vec<serde_json::Value> {
    let at = stamp(now_secs());
    let mut out = Vec::new();
    let ask = if turn == 0 { opts.ask.as_deref() } else { None };
    for (kind, text) in [
        ("user", ask.unwrap_or("Keep going (replica).")),
        ("assistant", "Done (replica)."),
    ] {
        let uuid = format!("{}-{turn}-{kind}", opts.session_id);
        let mut line = serde_json::json!({
            "parentUuid": parent, "isSidechain": false, "userType": "external",
            "cwd": opts.cwd, "sessionId": opts.session_id, "version": "2.1.296",
            "gitBranch": "main", "uuid": uuid, "timestamp": at, "type": kind,
        });
        line["message"] = if kind == "user" {
            serde_json::json!({"role": "user", "content": text})
        } else {
            serde_json::json!({
                "id": format!("msg_{turn}"), "type": "message", "role": "assistant",
                "model": "claude-opus-4-6", "content": [{"type": "text", "text": text}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1200 + turn * 7, "cache_creation_input_tokens": 0,
                          "cache_read_input_tokens": 8000, "output_tokens": 300, "service_tier": "standard"},
            })
        };
        *parent = Some(uuid);
        out.push(line);
    }
    out
}

fn codex_lines(opts: &WriteOptions, turn: u64) -> Vec<serde_json::Value> {
    let secs = now_secs();
    let at = stamp(secs);
    let mut out = Vec::new();
    if turn == 0 {
        out.push(
            serde_json::json!({"timestamp": at, "type": "session_meta", "payload": {
            "id": opts.session_id, "cwd": opts.cwd, "cli_version": "0.160.0",
            "originator": "codex_cli_rs", "history_mode": "paginated"}}),
        );
    }
    let usage = serde_json::json!({"input_tokens": 4000 + turn * 11, "cached_input_tokens": 1200,
        "output_tokens": 250, "reasoning_output_tokens": 60, "total_tokens": 4250 + turn * 11});
    out.push(serde_json::json!({"timestamp": at, "type": "turn_context",
        "payload": {"cwd": opts.cwd, "model": "gpt-5-codex"}}));
    if turn == 0
        && let Some(ask) = &opts.ask
    {
        out.push(
            serde_json::json!({"timestamp": at, "type": "response_item", "payload": {
            "type": "message", "role": "user", "content": [{"type": "input_text", "text": ask}]}}),
        );
    }
    // As Codex writes it: each response's usage once, keyed by its ids.
    out.push(serde_json::json!({"timestamp": at, "type": "token_usage_record", "payload": {
        "thread_id": opts.session_id, "turn_id": turn.to_string(),
        "response_id": format!("resp_{}_{}_{turn}", opts.session_id, opts.started), "usage": usage}}));
    out.push(serde_json::json!({"timestamp": at, "type": "event_msg", "payload": {
        "type": "token_count", "info": {"total_token_usage": usage, "last_token_usage": usage},
        "rate_limits": {
            "primary": {"used_percent": opts.used.0, "window_minutes": 300, "resets_at": secs + 3 * 3600},
            "secondary": {"used_percent": opts.used.1, "window_minutes": 10_080, "resets_at": secs + 4 * 86_400},
        }}}));
    out
}

/// Runs the writer until its parent goes away (or it is signalled).
pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    #[cfg(unix)]
    let parent = std::os::unix::process::parent_id();
    if let Some(dir) = opts.file.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut previous: Option<String> = None;
    let mut turn: u64 = 0;
    loop {
        let lines = if opts.idle {
            Vec::new()
        } else if opts.format == "claude" {
            claude_lines(&opts, turn, &mut previous)
        } else {
            codex_lines(&opts, turn)
        };
        if !lines.is_empty() {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&opts.file)
                .map_err(|e| format!("{}: {e}", opts.file.display()))?;
            for line in lines {
                writeln!(file, "{line}").map_err(|e| e.to_string())?;
            }
        }
        thread::sleep(Duration::from_millis(opts.every_ms));
        #[cfg(unix)]
        if std::os::unix::process::parent_id() != parent {
            return Ok(()); // whoever started it is gone
        }
        turn += 1;
    }
}
