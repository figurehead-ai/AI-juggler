//! Kimi Code's event log, `agents/<agent>/wire.jsonl` in each session's
//! folder (research `docs/research/capabilities/kimi-grok.md` §4, the MIT
//! `MoonshotAI/kimi-code` source at e2ad1dcf, read 2026-10-27): what a
//! session is doing now, and the exact tokens of each model call.
//!
//! The log is versioned. Its first line is
//! `{"type":"metadata","protocol_version":"1.5","created_at":<ms>}`; each
//! later line is one record, `{"type", …its fields, "time": <ms>}`
//! (`packages/agent-core-v2/src/wire/record.ts`, `wireService.ts`,
//! `docs/wire-manifest.d.ts`). Only the protocol versions in
//! [`KNOWN_PROTOCOLS`] are read: a log of another version, or one that
//! doesn't open with its version, says nothing (D-06: unknown, never a
//! guess). Of the 67 record types, six are read, field by field:
//!
//! - `turn.prompt` opens a turn and `turn.ended` (completed, cancelled,
//!   failed or blocked) closes it: working, then idle;
//! - `interaction.request` (an approval, a question, a tool the user
//!   runs) waits on the user until the `interaction.resolved` with its
//!   `id`: needs you;
//! - `usage.record` is one model call's tokens (`inputOther`, `output`,
//!   `inputCacheRead`, `inputCacheCreation`) and its model.
//!
//! What the lines say is never kept: no prompt, message or answer text is
//! read into memory as a value, only types, ids, times and counts.
//!
//! Read-only and incremental: each file is read from where the last read
//! stopped ([`crate::transcript_tail::TranscriptTail`]), at most 16 MiB a
//! call, and a replaced or shorter file is read again from its start.
//!
//! Which sessions are open comes from Kimi Code's own session locks,
//! `locks/sessions/<id>.lock` in the home (`{version, token, pid,
//! hostname, startedAt, clientIdentity}`, written while a CLI, Desktop or
//! web client holds the session and removed when it lets go:
//! `persistence/backends/node-fs/sessionLockService.ts`). Inferred, on the
//! live checklist (AJ-E1612): that a turn or a request older than the lock
//! belongs to an earlier process (a crash leaves a turn open), and that a
//! turn's end settles what it asked.
//!
//! Which reader, and why: Kimi Code's documented ways in are its ACP
//! server (only for sessions the client itself drives) and `kimi web`'s
//! local server (it can't reach a session a terminal holds, and reading it
//! needs a token from the home: outside what AI Juggler reads, the
//! maintainer's ruling). Neither sees the sessions the user runs, so this
//! reads the files Kimi Code writes in its home: their place is documented
//! (`configuration/data-locations.md`), their records are the vendor's own
//! generated schema; the locks are known from its source only.
//!
//! Observation only, to add to the user's own Kimi Code, never to stand in
//! for it: their unmodified `kimi` runs with their own sign-in, and AI
//! Juggler reads what it writes. Kimi Code's terms say "Kimi Code
//! subscriptions are for interactive use only" and "Don't use Kimi Code
//! for non-interactive automation": nothing here sends anything to a
//! session, answers for the user, or starts one.
//!
//! Paths are joined part by part from the home AI Juggler already knows
//! (`KIMI_CODE_HOME`, or the default Kimi Code resolves), on every system.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use juggler_adapter::live::{LiveSession, LiveStatus};
use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{UsageEvent, UsageSource};

use crate::transcript_tail::{TailLog, TranscriptTail};

/// A session's agents' folder (`<session>/agents/<agent>/`).
pub const AGENTS_DIR: &str = "agents";
/// The session's own agent (subagents have folders beside it).
pub const MAIN_AGENT: &str = "main";
/// Each agent's event log.
pub const WIRE_FILE: &str = "wire.jsonl";
/// Kimi Code's session locks, in the home: `locks/sessions/`.
pub const LOCKS_DIR: [&str; 2] = ["locks", "sessions"];

/// The folder of the session locks in `home`.
fn locks_dir(home: &Path) -> PathBuf {
    LOCKS_DIR
        .iter()
        .fold(home.to_path_buf(), |dir, part| dir.join(part))
}

/// The protocol versions whose records this reader was built from.
pub const KNOWN_PROTOCOLS: &[&str] = &["1.5"];

/// The most bytes read from a lock file.
const MAX_LOCK: u64 = 64 * 1024;

/// How long usage records stay in memory after a read asked for them:
/// a later read asks again from a minute before the newest one it got.
const KEEP_MS: i64 = 60 * 60 * 1000;

/// The record types read; every other line is skipped unparsed.
const READ_TYPES: &[&str] = &[
    "metadata",
    "turn.prompt",
    "turn.ended",
    "interaction.request",
    "interaction.resolved",
    "usage.record",
];

/// What a log's first line said of its version.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum Protocol {
    /// Nothing read yet.
    #[default]
    Unread,
    /// A version in [`KNOWN_PROTOCOLS`].
    Known,
    /// Another version, or no version line: nothing in it is read.
    Unknown,
}

/// One model call, as a `usage.record` gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireUsage {
    /// When it was recorded, epoch ms.
    pub ts: i64,
    /// The model it names.
    pub model: Option<String>,
    /// Its tokens.
    pub tokens: TokenCounts,
}

/// What one log says so far.
#[derive(Debug, Default)]
pub struct WireLog {
    protocol: Protocol,
    /// The running turn, by the time it was prompted.
    turn: Option<i64>,
    /// Requests waiting on the user, by id, with when they were made.
    open: BTreeMap<String, i64>,
    /// The last time a read record carried (a record without one takes it).
    last_time: Option<i64>,
    /// Usage read since it was last taken out.
    usage: Vec<WireUsage>,
}

/// A record's fields that are read; the rest of it is ignored.
#[derive(serde::Deserialize)]
struct Record {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    time: Option<f64>,
    #[serde(default)]
    protocol_version: Option<String>,
    #[serde(default)]
    created_at: Option<f64>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Tokens>,
}

/// `TokenUsage` (`packages/agent-core-v2/src/human/llm/usage.ts`).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tokens {
    #[serde(default)]
    input_other: u64,
    #[serde(default)]
    output: u64,
    #[serde(default)]
    input_cache_read: u64,
    #[serde(default)]
    input_cache_creation: u64,
}

/// A record's time (epoch ms, written as a number), whole.
#[allow(clippy::cast_possible_truncation)]
fn ms(value: f64) -> Option<i64> {
    (value.is_finite() && (0.0..1e15).contains(&value)).then_some(value as i64)
}

/// The record type a line names, cheaply, when it opens with it (the
/// writer puts `type` first); `None` when it doesn't, and the line is
/// read whole to find out.
fn leading_type(line: &[u8]) -> Option<&str> {
    let rest = line.strip_prefix(b"{\"type\":\"")?;
    let end = rest.iter().position(|b| *b == b'"')?;
    std::str::from_utf8(&rest[..end]).ok()
}

impl WireLog {
    fn line(&mut self, line: &[u8]) {
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        let first = self.protocol == Protocol::Unread;
        if self.protocol == Protocol::Unknown {
            return;
        }
        if !first && leading_type(line).is_some_and(|kind| !READ_TYPES.contains(&kind)) {
            return;
        }
        let Ok(record) = serde_json::from_slice::<Record>(line) else {
            if first {
                self.protocol = Protocol::Unknown;
            }
            return;
        };
        if first {
            let known = record.kind == "metadata"
                && record
                    .protocol_version
                    .as_deref()
                    .is_some_and(|version| KNOWN_PROTOCOLS.contains(&version));
            self.protocol = if known {
                Protocol::Known
            } else {
                Protocol::Unknown
            };
            self.last_time = record.created_at.and_then(ms);
            return;
        }
        let own = record.time.and_then(ms);
        let time = own.or(self.last_time);
        if own.is_some() {
            self.last_time = own;
        }
        let at = time.unwrap_or_default();
        match record.kind.as_str() {
            "turn.prompt" => self.turn = Some(at),
            "turn.ended" => {
                self.turn = None;
                // A turn that ended waits on nothing it asked (inferred).
                self.open.clear();
            }
            "interaction.request" => {
                if let Some(id) = record.id {
                    self.open.insert(id, at);
                }
            }
            "interaction.resolved" => {
                if let Some(id) = record.id {
                    self.open.remove(&id);
                }
            }
            "usage.record" => {
                if let (Some(tokens), Some(ts)) = (record.usage, time) {
                    let tokens = TokenCounts {
                        input: tokens.input_other,
                        output: tokens.output,
                        cache_read: tokens.input_cache_read,
                        cache_write: tokens.input_cache_creation,
                        reasoning: 0,
                    };
                    if tokens != TokenCounts::default() {
                        self.usage.push(WireUsage {
                            ts,
                            model: record.model.filter(|model| !model.is_empty()),
                            tokens,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    /// What the log says the session is doing, for a session held since
    /// `held_since` (epoch ms): a turn or a request from before then was
    /// an earlier process's.
    #[must_use]
    pub fn status(&self, held_since: Option<i64>) -> LiveStatus {
        if self.protocol != Protocol::Known {
            return LiveStatus::Unknown;
        }
        let current = |at: i64| held_since.is_none_or(|since| at >= since);
        if self.open.values().any(|at| current(*at)) {
            LiveStatus::Waiting
        } else if self.turn.is_some_and(current) {
            LiveStatus::Busy
        } else {
            LiveStatus::Idle
        }
    }
}

impl TailLog for WireLog {
    fn feed(&mut self, bytes: &[u8]) {
        for line in bytes.split(|b| *b == b'\n') {
            self.line(line);
        }
    }
}

/// One log as read so far, and the usage taken out of it.
#[derive(Debug, Default)]
struct WireFile {
    tail: TranscriptTail<WireLog>,
    /// Usage read and not yet dropped, oldest first.
    kept: Vec<WireUsage>,
    /// Usage older than this was dropped: a read from before it reads the
    /// file again from its start.
    floor_ms: Option<i64>,
}

impl WireFile {
    /// Reads what was appended; `false` when the file couldn't be read.
    fn catch_up(&mut self, path: &Path) -> bool {
        if self.tail.read_all(path).is_err() {
            return false;
        }
        let fresh = std::mem::take(&mut self.tail.log_mut().usage);
        self.kept.extend(fresh);
        true
    }
}

/// Every log read so far, by path (one cache for the app, as the Codex
/// titles' is).
static FILES: LazyLock<Mutex<HashMap<PathBuf, WireFile>>> = LazyLock::new(Mutex::default);

/// A session id from a file name: never empty, hidden, or a path.
fn plain_id(name: &str) -> Option<&str> {
    (!name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\'])).then_some(name)
}

/// The folder of session `id` in `home` (`sessions/<key>/<id>`), found by
/// listing the folder keys (no file is read).
fn session_dir(home: &Path, id: &str) -> Option<PathBuf> {
    std::fs::read_dir(home.join(super::SESSIONS_DIR))
        .ok()?
        .flatten()
        .map(|key| key.path().join(id))
        .find(|dir| dir.join(AGENTS_DIR).is_dir())
}

/// The main agent's log of the session in `dir`.
#[must_use]
pub fn main_log(dir: &Path) -> PathBuf {
    dir.join(AGENTS_DIR).join(MAIN_AGENT).join(WIRE_FILE)
}

/// The sessions open in the home at `home`, one file each: its main log
/// when there is one (so the session is read again whenever its log
/// grows), else its lock. Lists only; reads nothing.
#[must_use]
pub fn live_files(home: &Path) -> Vec<PathBuf> {
    let Ok(locks) = std::fs::read_dir(locks_dir(home)) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = locks
        .flatten()
        .filter_map(|entry| {
            let lock = entry.path();
            // `<id>.lock`; never the `.tmp-…` or `.reclaim` files a claim
            // makes beside it.
            let name = lock.file_name()?.to_str()?;
            let id = plain_id(name.strip_suffix(".lock")?)?;
            let log = session_dir(home, id).map(|dir| main_log(&dir));
            Some(log.filter(|log| log.is_file()).unwrap_or(lock))
        })
        .collect();
    out.sort();
    out
}

/// A session lock's fields that are read.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Lock {
    #[serde(default)]
    pid: Option<u32>,
    #[serde(default)]
    started_at: Option<i64>,
    #[serde(default)]
    client_identity: Option<String>,
}

fn read_lock(path: &Path) -> Option<Lock> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_LOCK {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// One of [`live_files`]: the open session, its process and what its log
/// says it is doing. `None` for any other file, or a lock gone or naming
/// no process.
#[must_use]
pub fn read_live(path: &Path) -> Option<LiveSession> {
    let name = path.file_name()?.to_str()?;
    let (home, id, log) = if name == WIRE_FILE {
        // <home>/sessions/<key>/<id>/agents/main/wire.jsonl
        let agent = path.parent()?;
        if agent.file_name()? != MAIN_AGENT {
            return None;
        }
        let dir = agent.parent()?.parent()?;
        let id = plain_id(dir.file_name()?.to_str()?)?.to_owned();
        let home = dir.parent()?.parent()?.parent()?.to_path_buf();
        (home, id, Some(path.to_path_buf()))
    } else {
        // <home>/locks/sessions/<id>.lock
        let id = plain_id(name.strip_suffix(".lock")?)?.to_owned();
        let home = path.parent()?.parent()?.parent()?.to_path_buf();
        (home, id, None)
    };
    let lock = read_lock(&locks_dir(&home).join(format!("{id}.lock")))?;
    let pid = lock.pid.filter(|pid| *pid > 0)?;
    let status = log.map_or(LiveStatus::Unknown, |log| status_of(&log, lock.started_at));
    Some(LiveSession {
        session_id: id,
        pid: Some(pid),
        // The lock's time is when the session was claimed, not when its
        // process started: the pid alone is matched.
        started_secs: None,
        cwd: None,
        status,
        host_session_id: None,
        entrypoint: lock.client_identity.filter(|client| !client.is_empty()),
        name: None,
        background: false,
    })
}

/// What the log at `path` says its session is doing, read up to its end.
#[must_use]
pub fn status_of(path: &Path, held_since: Option<i64>) -> LiveStatus {
    let Ok(taken) = FILES.lock().map(|mut files| files.remove(path)) else {
        return LiveStatus::Unknown;
    };
    let mut file = taken.unwrap_or_default();
    if !file.catch_up(path) {
        return LiveStatus::Unknown;
    }
    let status = file.tail.log().status(held_since);
    if let Ok(mut files) = FILES.lock() {
        files.insert(path.to_path_buf(), file);
    }
    status
}

/// Every agent's log of every session in `home`.
#[must_use]
pub fn logs(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in super::conversation_files(home)
        .iter()
        .filter_map(|state| state.parent())
    {
        let Ok(agents) = std::fs::read_dir(dir.join(AGENTS_DIR)) else {
            continue;
        };
        for agent in agents.flatten() {
            let log = agent.path().join(WIRE_FILE);
            if log.is_file() {
                out.push(log);
            }
        }
    }
    out.sort();
    out
}

/// When a file last changed, epoch ms.
fn modified_ms(path: &Path) -> Option<i64> {
    let at = std::fs::metadata(path).ok()?.modified().ok()?;
    i64::try_from(at.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis()).ok()
}

/// The key of one model call's usage. A record copied into another
/// session (a fork) keeps its time, model and counts, so it keys the
/// same and is counted once.
fn usage_key(usage: &WireUsage) -> String {
    let tokens = &usage.tokens;
    format!(
        "kimi-code:{}:{}:{}:{}:{}:{}",
        usage.ts,
        usage.model.as_deref().unwrap_or_default(),
        tokens.input,
        tokens.output,
        tokens.cache_read,
        tokens.cache_write
    )
}

/// The session a log belongs to: `<session>/agents/<agent>/wire.jsonl`.
fn session_of(log: &Path) -> Option<String> {
    let dir = log.parent()?.parent()?.parent()?;
    Some(dir.file_name()?.to_string_lossy().into_owned())
}

/// Every model call recorded in `home` at or after `since_ms`, for the
/// account `account`: exact counts (Kimi Code's own), each keyed so a call
/// read twice counts once. A log last written before `since_ms` isn't
/// opened.
#[must_use]
pub fn usage_since(account: &str, home: &Path, since_ms: i64) -> Vec<UsageEvent> {
    let logs = logs(home);
    // A log that went away is forgotten.
    if let Ok(mut files) = FILES.lock() {
        let listed: HashSet<&PathBuf> = logs.iter().collect();
        files.retain(|path, _| !path.starts_with(home) || listed.contains(path));
    }
    let mut events = Vec::new();
    for log in &logs {
        // Each log is taken out of the cache while it is read, so the live
        // refresher never waits behind a whole pass.
        let Ok(taken) = FILES.lock().map(|mut files| files.remove(log)) else {
            return events;
        };
        if taken.is_none() && modified_ms(log).is_some_and(|at| at < since_ms) {
            continue;
        }
        let mut file = taken.unwrap_or_default();
        if file.floor_ms.is_some_and(|floor| since_ms < floor) {
            file = WireFile::default();
        }
        if !file.catch_up(log) {
            continue;
        }
        let session = session_of(log);
        events.extend(
            file.kept
                .iter()
                .filter(|usage| usage.ts >= since_ms)
                .map(|usage| UsageEvent {
                    account: account.to_owned(),
                    session: session.clone(),
                    ts: usage.ts,
                    model: usage.model.clone(),
                    tokens: usage.tokens,
                    credits: None,
                    usd_est: None,
                    source: UsageSource::Transcript,
                    // Kimi Code's own counts, as the provider reported them.
                    confidence: Confidence::Measured,
                    msg_uuid: Some(usage_key(usage)),
                }),
        );
        // Only the last hour before what was asked for stays in memory.
        let keep_from = since_ms.saturating_sub(KEEP_MS);
        if file.kept.iter().any(|usage| usage.ts < keep_from) {
            file.kept.retain(|usage| usage.ts >= keep_from);
            file.floor_ms = Some(
                file.floor_ms
                    .map_or(keep_from, |floor| floor.max(keep_from)),
            );
        }
        if let Ok(mut files) = FILES.lock() {
            files.insert(log.clone(), file);
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(lines: &[&str]) -> WireLog {
        let mut log = WireLog::default();
        log.feed(format!("{}\n", lines.join("\n")).as_bytes());
        log
    }

    const META: &str = r#"{"type":"metadata","protocol_version":"1.5","created_at":1000}"#;

    #[test]
    fn a_turn_works_then_waits_on_the_user_then_idles() {
        let mut wire = log(&[META]);
        assert_eq!(wire.status(None), LiveStatus::Idle);
        wire.feed(
            br#"{"type":"turn.prompt","agentId":"main","input":[{"type":"text","text":"hi"}],"origin":"user","time":2000}
"#,
        );
        assert_eq!(wire.status(None), LiveStatus::Busy);
        wire.feed(
            br#"{"type":"interaction.request","agentId":"main","id":"ask-1","kind":"approval","request":{},"time":2100}
"#,
        );
        assert_eq!(wire.status(None), LiveStatus::Waiting);
        wire.feed(
            br#"{"type":"interaction.resolved","agentId":"main","id":"ask-1","response":{},"time":2200}
"#,
        );
        assert_eq!(wire.status(None), LiveStatus::Busy);
        wire.feed(
            br#"{"type":"turn.ended","agentId":"main","reason":"completed","time":2300}
"#,
        );
        assert_eq!(wire.status(None), LiveStatus::Idle);
    }

    #[test]
    fn a_turn_or_request_from_before_the_lock_was_another_process() {
        let wire = log(&[
            META,
            r#"{"type":"turn.prompt","agentId":"main","input":[],"origin":"user","time":2000}"#,
            r#"{"type":"interaction.request","agentId":"main","id":"q","kind":"question","request":{},"time":2100}"#,
        ]);
        assert_eq!(wire.status(Some(1500)), LiveStatus::Waiting);
        assert_eq!(wire.status(Some(5000)), LiveStatus::Idle);
    }

    #[test]
    fn an_unknown_or_missing_version_says_nothing() {
        for first in [
            r#"{"type":"metadata","protocol_version":"9.0","created_at":1}"#,
            r#"{"type":"metadata","created_at":1}"#,
            r#"{"type":"turn.prompt","time":2}"#,
            "not json",
        ] {
            let wire = log(&[
                first,
                r#"{"type":"turn.prompt","agentId":"main","input":[],"time":2000}"#,
                r#"{"type":"usage.record","agentId":"main","model":"k","usage":{"inputOther":1,"output":1,"inputCacheRead":0,"inputCacheCreation":0},"time":2001}"#,
            ]);
            assert_eq!(wire.status(None), LiveStatus::Unknown);
            assert!(wire.usage.is_empty(), "an unknown version gave usage");
        }
    }

    #[test]
    fn usage_records_carry_exact_counts_and_other_lines_are_skipped() {
        let wire = log(&[
            META,
            r#"{"type":"context.append_message","agentId":"main","message":{"role":"user","content":"usage.record turn.prompt"},"time":1500}"#,
            r#"{"type":"usage.record","agentId":"main","model":"kimi-k3","usage":{"inputOther":120,"output":40,"inputCacheRead":900,"inputCacheCreation":10},"usageScope":"turn","time":2000}"#,
            r#"{"type":"usage.record","agentId":"main","model":"kimi-k3","usage":{"inputOther":0,"output":0,"inputCacheRead":0,"inputCacheCreation":0},"time":2001}"#,
            r#"{"type":"usage.record","agentId":"main","model":"kimi-k3","usage":{"inputOther":5,"output":6,"inputCacheRead":0,"inputCacheCreation":0}}"#,
        ]);
        assert_eq!(wire.status(None), LiveStatus::Idle);
        assert_eq!(
            wire.usage,
            vec![
                WireUsage {
                    ts: 2000,
                    model: Some("kimi-k3".into()),
                    tokens: TokenCounts {
                        input: 120,
                        output: 40,
                        cache_read: 900,
                        cache_write: 10,
                        reasoning: 0,
                    },
                },
                // No time of its own: the last one read.
                WireUsage {
                    ts: 2001,
                    model: Some("kimi-k3".into()),
                    tokens: TokenCounts {
                        input: 5,
                        output: 6,
                        ..TokenCounts::default()
                    },
                },
            ]
        );
    }

    #[test]
    fn a_lock_file_name_is_a_plain_id_only() {
        assert_eq!(plain_id("ses-1"), Some("ses-1"));
        assert_eq!(plain_id(""), None);
        assert_eq!(plain_id(".hidden"), None);
        assert_eq!(plain_id("a/b"), None);
    }
}
