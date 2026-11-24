//! AJ-E2322 (the review's A3.7, A3.8, B7 and B9 12 and 15): a move says
//! "opened" only when the conversation really is open in the target app,
//! and keeps a timeline of every phase.
//!
//! - **Opened** is verified only by the adapter's evidence after the
//!   link: a session of the conversation running in the target profile
//!   (the writers hook: Claude Code's pid file or `--resume`), or the
//!   target app adding to the conversation's history. A link delivered is
//!   not "opened": without that evidence the move says so plainly and
//!   offers Open.
//! - **The wait** for it is by file events on the moved files' folders
//!   (a look every [`LOOK`] in case an event is lost, `folder_wake`'s
//!   rule), capped at [`OPEN_CAP`], then a short quiet window so the
//!   app's rewrite of its records is done before the checks after: never
//!   a fixed settle (a slow start gave false results).
//! - **The timeline**: each phase in the journal with how long it took
//!   (ms), each wait's set (program, reason, evidence) and what remained.
//!   Copy details carries it with the home folder as `~`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use juggler_adapter::moving::{Writer, WriterEvidence};
use juggler_handover::moving::Transaction;

use super::quit::{Writing, writers_now};
use super::{KnownPlace, MoveSystem};
use crate::state::AppState;

/// The longest a move waits for the conversation to show open.
pub(super) const OPEN_CAP: Duration = Duration::from_secs(15);
/// A look without an event, in case one is lost.
pub(super) const LOOK: Duration = Duration::from_secs(1);
/// Once open, how long the files must go without a change (the app's
/// rewrite of its records done) before the checks after.
const QUIET: Duration = Duration::from_millis(300);

/// Waits until `seen` finds the conversation open, looking again at each
/// wake on `wakes` (a file event) or every [`LOOK`], at most `cap`; then
/// until the files go [`QUIET`] (still within `cap`). Returns what `seen`
/// found; `None`: not within `cap`.
pub(super) fn wait_on(
    wakes: &Receiver<()>,
    cap: Duration,
    seen: &mut dyn FnMut() -> Option<String>,
) -> Option<String> {
    let until = Instant::now() + cap;
    let next = |step: Duration| {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        match wakes.recv_timeout(left.min(step)) {
            Ok(()) => Some(true),
            Err(RecvTimeoutError::Timeout) => Some(false),
            Err(RecvTimeoutError::Disconnected) => {
                // No events to hear: looks only.
                std::thread::sleep(left.min(step));
                Some(false)
            }
        }
    };
    let found = loop {
        if let Some(found) = seen() {
            break found;
        }
        next(LOOK)?;
    };
    while next(QUIET) == Some(true) {}
    Some(found)
}

/// The folders whose changes wake the wait: each moved file's.
pub(super) fn folders_of(txn: &Transaction) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = txn
        .touched()
        .into_iter()
        .filter_map(Path::parent)
        .map(Path::to_path_buf)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The adapter's after-open check: the conversation open in the target
/// profile (main process `pid`, `place`) now, as evidence in words; `None`:
/// nothing shows it open yet. `history`: the conversation's history in the
/// target and its length when the link went.
pub(super) fn seen_open(
    state: &Arc<AppState>,
    (pid, place): (u32, &KnownPlace),
    writing: &Writing<'_>,
    history: Option<&(PathBuf, u64)>,
    system: &dyn MoveSystem,
) -> Option<String> {
    let table = system.table(state);
    let writers = writers_now(&table, writing);
    if !writers.is_empty() {
        let tree = system.tree(state, place, &[pid]);
        let ours = |w: &&Writer| w.pid == pid || tree.iter().any(|m| m.pid == w.pid);
        if let Some(writer) = writers.iter().find(ours) {
            let name = tree
                .iter()
                .find(|m| m.pid == writer.pid)
                .map_or("its session", |m| m.name.as_str());
            return Some(format!(
                "{name} (process {}) has it open: {}",
                writer.pid,
                evidence_words(writer.evidence)
            ));
        }
    }
    let (path, was) = history?;
    let now = std::fs::metadata(path).map(|meta| meta.len()).ok()?;
    (now > *was).then(|| "the app added to its history after the link".to_owned())
}

/// How a writer is known, in words.
pub(super) fn evidence_words(evidence: WriterEvidence) -> &'static str {
    match evidence {
        WriterEvidence::SessionFile => "its session file",
        WriterEvidence::ResumeArg => "started to resume it",
        WriterEvidence::HeldLock => "it holds the conversation's lock",
        WriterEvidence::OpenFile => "it has a moved file open",
    }
}

/// The move's phases as they end, into its journal (the review's B7).
pub(super) struct Timeline {
    last: Instant,
}

impl Timeline {
    pub(super) fn start() -> Self {
        Self {
            last: Instant::now(),
        }
    }

    /// Journals `phase` as ended now: how long it took (ms) and `detail`.
    /// A journal that can't be written loses the line, never the move.
    pub(super) fn mark(&mut self, txn: &Transaction, phase: &str, detail: serde_json::Value) {
        let took = u64::try_from(self.last.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.last = Instant::now();
        if let Err(error) = txn.timeline(phase, took, detail) {
            tracing::warn!(target: "juggler::move", "a timeline line wasn't written: {error}");
        }
    }
}

/// One journal line for Copy details: its event, then what it names, with
/// the home folder as `~` and times from the move's start (`created_ms`).
pub(super) fn line_words(value: &serde_json::Value, created_ms: i64) -> String {
    let Some(fields) = value.as_object() else {
        return value.to_string();
    };
    let event = fields
        .get("event")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("note");
    let rest: Vec<String> = fields
        .iter()
        .filter(|(key, _)| key.as_str() != "event")
        .map(|(key, value)| match value {
            serde_json::Value::String(text) if text.starts_with('/') => {
                format!("{key} {}", super::shown(Path::new(text)))
            }
            serde_json::Value::String(text) => format!("{key} {text}"),
            serde_json::Value::Number(n) if key == "step" => {
                format!("step {}", n.as_u64().map_or(0, |n| n + 1))
            }
            serde_json::Value::Number(n) if key == "at_ms" => {
                format!("at +{} ms", n.as_i64().map_or(0, |at| at - created_ms))
            }
            serde_json::Value::Number(n) if key == "took_ms" => format!("took {n} ms"),
            serde_json::Value::Array(items) => format!("{key} {}", list_words(items)),
            other => format!("{key} {other}"),
        })
        .collect();
    if rest.is_empty() {
        event.to_owned()
    } else {
        format!("{event}: {}", rest.join(", "))
    }
}

/// A wait set or what remained, in words: "claude (process 42, writes it:
/// its session file)", or "none".
fn list_words(items: &[serde_json::Value]) -> String {
    if items.is_empty() {
        return "none".to_owned();
    }
    let field = |item: &serde_json::Value, key: &str| {
        item.get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    items
        .iter()
        .map(|item| match item {
            serde_json::Value::String(text) => text.clone(),
            serde_json::Value::Object(_) => {
                let program = field(item, "program").unwrap_or_else(|| "a process".into());
                let pid = item.get("pid").and_then(serde_json::Value::as_u64);
                let how: Vec<String> = [field(item, "reason"), field(item, "evidence")]
                    .into_iter()
                    .flatten()
                    .collect();
                let pid = pid.map_or_else(String::new, |pid| format!("process {pid}"));
                let inner: Vec<String> = [pid, how.join(": ")]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect();
                format!("{program} ({})", inner.join(", "))
            }
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    /// R2: a file event wakes the wait at once (no fixed settle, no
    /// full look interval).
    #[test]
    fn the_open_wait_wakes_on_a_file_event_not_a_fixed_settle() {
        let (tx, rx) = mpsc::channel();
        let open = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&open);
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            tx.send(()).unwrap();
            tx
        });
        let started = Instant::now();
        let found = wait_on(&rx, OPEN_CAP, &mut || {
            open.load(std::sync::atomic::Ordering::SeqCst)
                .then(|| "seen".to_owned())
        });
        assert_eq!(found.as_deref(), Some("seen"));
        assert!(started.elapsed() < LOOK, "{:?}", started.elapsed());
        drop(writer.join().unwrap());
    }

    /// R2: never past its cap; nothing seen is `None` (pending, not
    /// verified).
    #[test]
    fn the_open_wait_ends_at_its_cap_with_nothing_seen() {
        let (_tx, rx) = mpsc::channel::<()>();
        let started = Instant::now();
        let found = wait_on(&rx, Duration::from_millis(80), &mut || None);
        assert_eq!(found, None);
        assert!(started.elapsed() < Duration::from_millis(600));
    }

    /// R2: once seen, it waits for the files to go quiet (the records'
    /// rewrite), still within the cap.
    #[test]
    fn once_open_the_wait_lets_the_records_rewrite_finish() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..3 {
            tx.send(()).unwrap();
        }
        let mut looks = 0;
        let found = wait_on(&rx, Duration::from_secs(2), &mut || {
            looks += 1;
            Some("seen".to_owned())
        });
        assert_eq!(found.as_deref(), Some("seen"));
        assert_eq!(looks, 1);
        assert!(rx.try_recv().is_err(), "every queued change was waited out");
    }

    /// R3: Copy details: each wait's set and what remained in words, times
    /// from the move's start, the home folder never spelled out.
    #[test]
    fn copy_details_says_the_timeline_in_words_without_the_home_folder() {
        let line = serde_json::json!({
            "event": "timeline", "phase": "closing", "at_ms": 1_500, "took_ms": 420,
            "waited": [{"program": "claude", "pid": 42, "reason": "writes it", "evidence": "its session file"}],
            "remained": [],
        });
        let words = line_words(&line, 1_000);
        assert!(words.starts_with("timeline: "), "{words}");
        assert!(words.contains("at +500 ms"), "{words}");
        assert!(words.contains("took 420 ms"), "{words}");
        assert!(
            words.contains("waited claude (process 42, writes it: its session file)"),
            "{words}"
        );
        assert!(words.contains("remained none"), "{words}");
        if let Some(home) = juggler_platform::user::home() {
            let path = home.join("Library/x.json");
            let line = serde_json::json!({"event": "restore", "path": path});
            let words = line_words(&line, 0);
            assert!(!words.contains(&home.display().to_string()), "{words}");
            assert!(words.contains("~/Library/x.json"), "{words}");
        }
    }
}
