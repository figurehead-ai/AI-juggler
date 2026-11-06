//! AJ-E2912 (D-75): debug mode, a detailed local log of switches and
//! launches, and the last flow's steps for Report a bug.
//!
//! - The switch and launch code (here and in the libraries) emit `tracing`
//!   events with the target [`TARGET`]: a flow (`switch`, `launch`), a
//!   step, and numbers and closed words only (sizes, counts, durations,
//!   codes, the choices taken). Never a message's text, a prompt, a file's
//!   contents, a credential or an environment value: the fields are ours,
//!   and a test switches a conversation with marked text and scans the log.
//! - [`Layer`] keeps the last [`RECENT_LINES`] lines in memory always
//!   (Report a bug shows the last switch's steps), and writes them to
//!   `logs/debug.log` only while debug mode is on.
//! - Bounded: past [`MAX_BYTES`] the file becomes `debug.1.log` (the older
//!   one dropped), so at most twice that is kept. Debug mode turns itself
//!   off [`DAYS_ON`] days after it was turned on, and says so in a notice
//!   ([`stop_if_due`]: at start, and on the backend's tick while it runs);
//!   past its days nothing more is written. Every line is scrubbed
//!   (home folder, user name, paths outside home, links, keys) before it's
//!   kept, the file is the owner's alone, and it's scrubbed again (names)
//!   whenever it's shown.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

use juggler_core::storage::Storage;

use crate::error::AppResult;

/// The events' target.
pub const TARGET: &str = "aij::flow";
/// When debug mode was turned on (epoch ms); absent or empty: off.
pub const SINCE_KEY: &str = "debug.mode_since";
/// The log, inside the data folder.
pub const RELATIVE: &str = "logs/debug.log";
/// The older log.
pub const OLDER: &str = "logs/debug.1.log";
/// The most one file holds.
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;
/// Days debug mode stays on.
pub const DAYS_ON: i64 = 7;
/// Lines kept in memory.
pub const RECENT_LINES: usize = 400;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

static PATH: OnceLock<PathBuf> = OnceLock::new();
static ON: AtomicBool = AtomicBool::new(false);
static UNTIL_MS: AtomicI64 = AtomicI64::new(0);
static RECENT: Mutex<VecDeque<Line>> = Mutex::new(VecDeque::new());
static WRITE: Mutex<()> = Mutex::new(());
/// Tests that touch debug mode's process-wide state (its flag, the lines
/// kept in memory) take turns.
#[cfg(test)]
pub(super) static TEST_SERIAL: Mutex<()> = Mutex::new(());

/// One step, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// When, epoch ms.
    pub at_ms: i64,
    /// Its flow (`switch`, `launch`).
    pub flow: String,
    /// The line as written.
    pub text: String,
}

/// Debug mode as Settings shows it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DebugModeInfo {
    /// On now.
    pub on: bool,
    /// When it turns itself off (epoch ms), while on.
    #[ts(type = "number | null")]
    pub until_ms: Option<i64>,
    /// The log's size on disk (both files).
    #[ts(type = "number")]
    pub bytes: u64,
    /// Lines in the log on disk.
    #[ts(type = "number")]
    pub lines: u64,
}

/// Collects an event's fields as `name=value` words.
#[derive(Default)]
struct Fields {
    flow: String,
    step: String,
    rest: String,
}

impl tracing::field::Visit for Fields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "flow" => value.clone_into(&mut self.flow),
            "step" | "message" if self.step.is_empty() => value.clone_into(&mut self.step),
            name => {
                let _ = write!(self.rest, " {name}={}", one_line(value));
            }
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let text = format!("{value:?}");
        self.record_str(field, text.trim_matches('"'));
    }
}

fn one_line(value: &str) -> String {
    let flat: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(300)
        .collect();
    if flat.contains(' ') {
        format!("\"{flat}\"")
    } else {
        flat
    }
}

/// The layer: in memory always, on disk while debug mode is on.
#[derive(Debug, Default)]
pub struct Layer;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Layer {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        keep(juggler_core::model::Timestamp::now().millis(), &fields);
    }
}

/// Whether a step at `at_ms` goes to the file: debug mode on, within its
/// days.
fn writing(at_ms: i64) -> bool {
    ON.load(Ordering::Relaxed) && at_ms <= UNTIL_MS.load(Ordering::Relaxed)
}

fn keep(at_ms: i64, fields: &Fields) {
    keep_at(PATH.get().map(PathBuf::as_path), at_ms, fields);
}

/// [`keep`], writing to `path` (the log's, once installed).
fn keep_at(path: Option<&Path>, at_ms: i64, fields: &Fields) {
    // Scrubbed before it's kept: a tool's own error names its paths (the
    // home folder as `~`, a path outside it as `[path]`), keys stripped.
    let scrub = super::Scrub::here();
    let text = scrub.text(&crate::error_log::without_keys(&format!(
        "{at_ms} {} {}{}",
        if fields.flow.is_empty() {
            "-"
        } else {
            &fields.flow
        },
        if fields.step.is_empty() {
            "-"
        } else {
            &fields.step
        },
        fields.rest
    )));
    let line = Line {
        at_ms,
        flow: fields.flow.clone(),
        text,
    };
    // Off (or past its days: the tick turns it off and says so), nothing
    // is written.
    if writing(at_ms)
        && let Some(path) = path
    {
        let _ = append(path, &line.text);
    }
    if let Ok(mut recent) = RECENT.lock() {
        if recent.len() >= RECENT_LINES {
            recent.pop_front();
        }
        recent.push_back(line);
    }
}

fn append(path: &Path, text: &str) -> std::io::Result<()> {
    let _guard = WRITE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        std::fs::rename(path, older_of(path))?;
    }
    let mut file = juggler_platform::private_fs::append_private(path)?;
    writeln!(file, "{text}")
}

fn older_of(path: &Path) -> PathBuf {
    path.with_file_name("debug.1.log")
}

/// What this layer keeps: the switch and launch steps only.
#[must_use]
pub fn steps() -> tracing_subscriber::filter::FilterFn<impl Fn(&tracing::Metadata<'_>) -> bool> {
    tracing_subscriber::filter::filter_fn(|metadata| metadata.target() == TARGET)
}

/// Starts the layer and reads whether debug mode is on (at start; turns
/// it off when its days are up).
pub fn install(data_dir: &Path, storage: &Storage) {
    use tracing_subscriber::Layer as _;
    use tracing_subscriber::layer::SubscriberExt as _;
    let _ = PATH.set(data_dir.join(RELATIVE));
    super::app_log::set_folder(data_dir);
    // Each layer filters for itself (a layer's own `enabled` would filter
    // for every layer): the steps here, the rest in the app log.
    let _ = tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(Layer.with_filter(steps()))
            .with(super::app_log::Layer.with_filter(super::app_log::filter())),
    );
    let now = juggler_core::model::Timestamp::now().millis();
    if stop_if_due(storage, now).is_none()
        && let Some(since) = since(storage)
    {
        arm(since);
    }
}

/// Turns debug mode off once its [`DAYS_ON`] days are up, and says so in
/// a notice (once per time it was on). `Some(since)`: it stopped now.
pub fn stop_if_due(storage: &Storage, now: i64) -> Option<i64> {
    let since = since(storage)?;
    if now < since + DAYS_ON * DAY_MS {
        return None;
    }
    let _ = storage.app_settings().set(SINCE_KEY, "");
    ON.store(false, Ordering::Relaxed);
    crate::features::notifications::sources::debug_mode_stopped(storage, since, now);
    Some(since)
}

/// The backend's tick: debug mode on past its days stops, and the views
/// hear of its notice. Costs two atomic reads otherwise.
pub fn tick(state: &std::sync::Arc<crate::state::AppState>) {
    let now = juggler_core::model::Timestamp::now().millis();
    if !ON.load(Ordering::Relaxed) || now <= UNTIL_MS.load(Ordering::Relaxed) {
        return;
    }
    let stopped = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| stop_if_due(&storage, now));
    if stopped.is_some() {
        crate::features::notifications::changed(state);
    }
}

fn arm(since: i64) {
    UNTIL_MS.store(since + DAYS_ON * DAY_MS, Ordering::Relaxed);
    ON.store(true, Ordering::Relaxed);
}

fn since(storage: &Storage) -> Option<i64> {
    storage
        .app_settings()
        .get(SINCE_KEY)
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
}

/// Turns debug mode on (from now, for [`DAYS_ON`] days) or off; the log
/// stays until it's cleared.
///
/// # Errors
///
/// Storage errors.
pub fn set(storage: &Storage, on: bool) -> AppResult<()> {
    if on {
        let now = juggler_core::model::Timestamp::now().millis();
        storage.app_settings().set(SINCE_KEY, &now.to_string())?;
        arm(now);
    } else {
        storage.app_settings().set(SINCE_KEY, "")?;
        ON.store(false, Ordering::Relaxed);
    }
    Ok(())
}

/// Debug mode now.
#[must_use]
pub fn info(data_dir: &Path) -> DebugModeInfo {
    let on = ON.load(Ordering::Relaxed)
        && juggler_core::model::Timestamp::now().millis() <= UNTIL_MS.load(Ordering::Relaxed);
    let (mut bytes, mut lines) = (0, 0);
    for path in [data_dir.join(OLDER), data_dir.join(RELATIVE)] {
        if let Ok(text) = std::fs::read(&path) {
            bytes += text.len() as u64;
            lines += text.split(|b| *b == b'\n').count().saturating_sub(1) as u64;
        }
    }
    DebugModeInfo {
        on,
        until_ms: on.then(|| UNTIL_MS.load(Ordering::Relaxed)),
        bytes,
        lines,
    }
}

/// Removes the log (both files, and what's kept in memory).
///
/// # Errors
///
/// The IO error, when a file can't be removed.
pub fn clear(data_dir: &Path) -> std::io::Result<()> {
    let _guard = WRITE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for path in [data_dir.join(OLDER), data_dir.join(RELATIVE)] {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    if let Ok(mut recent) = RECENT.lock() {
        recent.clear();
    }
    Ok(())
}

/// The log on disk, oldest first, its newest `max` bytes from a line's
/// start (not scrubbed: the caller scrubs what it shows).
#[must_use]
pub fn text(data_dir: &Path, max: usize) -> String {
    let mut all = Vec::new();
    for path in [data_dir.join(OLDER), data_dir.join(RELATIVE)] {
        if let Ok(bytes) = std::fs::read(path) {
            all.extend_from_slice(&bytes);
        }
    }
    let from = all.len().saturating_sub(max);
    let text = String::from_utf8_lossy(&all[from..]).into_owned();
    match (from, text.find('\n')) {
        (0, _) | (_, None) => text,
        (_, Some(at)) => text[at + 1..].to_owned(),
    }
}

/// Keeps one step of `flow` at `at_ms`, as the layer does (the tests of
/// what reads the steps).
#[cfg(test)]
pub(super) fn keep_step(at_ms: i64, flow: &str, step: &str) {
    keep_at(
        None,
        at_ms,
        &Fields {
            flow: flow.to_owned(),
            step: step.to_owned(),
            rest: String::new(),
        },
    );
}

/// The last run of `flow` kept in memory, if it ended within `within_ms`:
/// its lines from its first step (the step named `start`) on.
#[must_use]
pub fn last_flow(flow: &str, now_ms: i64, within_ms: i64) -> Vec<String> {
    let Ok(recent) = RECENT.lock() else {
        return Vec::new();
    };
    let mine: Vec<&Line> = recent.iter().filter(|l| l.flow == flow).collect();
    let Some(last) = mine.last() else {
        return Vec::new();
    };
    if now_ms - last.at_ms > within_ms {
        return Vec::new();
    }
    let start = mine
        .iter()
        .rposition(|l| l.text.split(' ').nth(2) == Some("start"))
        .unwrap_or(0);
    mine[start..].iter().map(|l| l.text.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(flow: &str, step: &str, rest: &str) -> Fields {
        Fields {
            flow: flow.to_owned(),
            step: step.to_owned(),
            rest: rest.to_owned(),
        }
    }

    #[test]
    fn the_last_flow_starts_at_its_start_and_ages_out() {
        let _serial = TEST_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        keep(1_000, &fields("switch", "start", " pair=claude>codex"));
        keep(1_100, &fields("switch", "stage", " bytes=10"));
        keep(1_150, &fields("launch", "start", ""));
        keep(2_000, &fields("switch", "start", " pair=claude>claude"));
        keep(2_400, &fields("switch", "done", " ms=400"));
        let lines = last_flow("switch", 3_000, 60_000);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].contains("pair=claude>claude"));
        assert!(
            last_flow("switch", 2_400 + 60_001, 60_000).is_empty(),
            "too old"
        );
    }

    #[test]
    fn the_file_rolls_over_and_keys_never_reach_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(RELATIVE);
        append(
            &path,
            &crate::error_log::without_keys("x key=sk-FAKE0000NOTAREALKEY111"),
        )
        .unwrap();
        assert!(
            !std::fs::read_to_string(&path)
                .unwrap()
                .contains("NOTAREALKEY")
        );
        let long = "y".repeat(64 * 1024);
        for _ in 0..300 {
            append(&path, &long).unwrap();
        }
        let total = std::fs::metadata(&path).unwrap().len()
            + std::fs::metadata(older_of(&path)).map_or(0, |m| m.len());
        assert!(
            total <= 2 * MAX_BYTES + 128 * 1024,
            "at most two files: {total}"
        );
        clear(dir.path()).unwrap();
        assert!(!path.exists() && !older_of(&path).exists());
    }

    fn lines_in(path: &Path) -> usize {
        std::fs::read_to_string(path).map_or(0, |text| text.lines().count())
    }

    /// R4: off, nothing is written; turned off, the log stays until it's
    /// cleared.
    #[test]
    fn off_writes_nothing_and_turning_it_off_keeps_the_log() {
        let _serial = TEST_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(RELATIVE);
        let storage = Storage::open_in_memory().unwrap();
        let now = || juggler_core::model::Timestamp::now().millis();
        set(&storage, false).unwrap();
        keep_at(Some(&path), now(), &fields("switch", "start", " files=3"));
        assert!(!path.exists(), "off: nothing is written");
        assert_eq!(info(dir.path()).lines, 0);

        set(&storage, true).unwrap();
        keep_at(Some(&path), now(), &fields("switch", "start", " files=3"));
        keep_at(Some(&path), now(), &fields("switch", "done", " ms=40"));
        assert_eq!(lines_in(&path), 2, "on: each step is written");

        set(&storage, false).unwrap();
        assert!(!info(dir.path()).on);
        keep_at(Some(&path), now(), &fields("launch", "spawned", " args=2"));
        assert_eq!(lines_in(&path), 2, "off again: nothing more is written");
        assert_eq!(info(dir.path()).lines, 2, "the log stays");
        assert_eq!(since(&storage), None);
        clear(dir.path()).unwrap();
        assert!(!path.exists(), "until it's cleared");
        assert_eq!(info(dir.path()).lines, 0);
    }

    /// R3: seven days after it was turned on, debug mode writes nothing
    /// more and stops by itself, once, with a notice; within its days it
    /// stays on.
    #[test]
    fn it_stops_itself_after_seven_days_and_says_so() {
        use crate::features::notifications::centre;
        let _serial = TEST_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(RELATIVE);
        let storage = Storage::open_in_memory().unwrap();
        let now = juggler_core::model::Timestamp::now().millis();

        set(&storage, true).unwrap();
        let since = super::since(&storage).unwrap();
        assert_eq!(
            stop_if_due(&storage, since + DAYS_ON * DAY_MS - 1),
            None,
            "within its seven days it stays on"
        );
        assert!(info(dir.path()).on);
        assert_eq!(info(dir.path()).until_ms, Some(since + DAYS_ON * DAY_MS));

        // Turned on eight days ago: past its days, nothing is written.
        let long_ago = now - 8 * DAY_MS;
        storage
            .app_settings()
            .set(SINCE_KEY, &long_ago.to_string())
            .unwrap();
        arm(long_ago);
        keep_at(Some(&path), now, &fields("switch", "start", ""));
        assert!(!path.exists(), "past its days nothing is written");
        assert!(!info(dir.path()).on);

        assert_eq!(stop_if_due(&storage, now), Some(long_ago));
        assert_eq!(super::since(&storage), None, "off in Settings");
        assert!(!ON.load(Ordering::Relaxed));
        assert_eq!(stop_if_due(&storage, now + 1), None, "told once");
        let earlier = centre::page(
            &storage,
            "earlier",
            &crate::paging::PageRequest::first(10),
            now,
        )
        .unwrap()
        .items;
        assert_eq!(earlier.len(), 1, "{earlier:?}");
        assert_eq!(earlier[0].title, "Debug mode turned itself off");
        assert!(earlier[0].detail.starts_with("It was on for 7 days."));
        assert_eq!(earlier[0].target.page.as_deref(), Some("diagnostics"));
        assert!(!earlier[0].needs, "it informs; nothing needs the user");
    }

    #[test]
    fn a_value_is_one_line_and_bounded() {
        assert_eq!(one_line("a\nb"), "\"a b\"");
        assert_eq!(one_line(&"z".repeat(400)).len(), 300);
    }
}
