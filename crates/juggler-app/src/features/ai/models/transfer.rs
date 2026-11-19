//! AJ-E3813 (ADR-0035 §11.8, ADR-0034 §5.1–5.2, ADR-0033 §5): one model
//! file's transfer, from the user's click to a verified `<sha256>.gguf`.
//!
//! **What reaches the network.** Only the entry's pinned URL, or its
//! mirror once the pinned one failed, each checked again by the one URL
//! parser ([`super::source::pinned`]) before the first request; and only
//! redirect hops [`super::source::redirect_allowed`] lets through, at most
//! [`super::source::MAX_REDIRECTS`] of them, each checked. A plain GET:
//! the generic user agent and a `Range` when resuming, nothing else
//! ([`juggler_account::UreqTransport::get_stream`]).
//!
//! **What reaches the disk.** `<sha256>-<n>.part` while it arrives, then
//! `<sha256>.gguf` by an atomic rename, only after:
//! - the size matched at every step: a `Content-Length` or a
//!   `Content-Range` total other than the entry's size stops it before a
//!   byte is written, a byte past the size aborts it, and a final length
//!   other than the size deletes it;
//! - the SHA-256 of every byte, taken as it arrived, equals the entry's.
//!
//! No name from the section or the URL becomes a path: the file name is
//! the entry's hash, which the record check holds to 64 lower-case hex.
//! A cancelled or failed transfer deletes its part file.
//!
//! **While it runs.** Before each request and about once a second between
//! chunks it reads this Mac ([`Machine`]): Low Power Mode or a metered
//! link pauses it (the part kept, the connection closed); free disk below
//! the bytes still to fetch plus the margin pauses it; free disk below
//! the local cache's low-disk line stops it and deletes the part
//! (ADR-0034 §5.2 rule 5). A pause waits on a condition variable, woken
//! by a cancel, a nudge or [`Tuning::pause_poll`], so it costs no CPU. It
//! goes on by itself with a `Range` request from the pinned URL again,
//! since a host's signed redirect expires. A dropped connection resumes
//! the same way, up to [`Tuning::attempts`] failures in a row.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use juggler_platform::room::DiskSpace;
use sha2::{Digest as _, Sha256};

use super::ModelEntry;
use super::source::{self, Source};

/// A finished model file's extension.
pub const MODEL_EXTENSION: &str = "gguf";

/// A file still arriving.
pub const PART_EXTENSION: &str = "part";

/// One answer to a GET: its status, its headers (lower-case names), and
/// its body as it arrives.
pub struct Answer {
    /// The status code.
    pub status: u16,
    /// The headers, lower-case names.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Box<dyn Read + Send>,
}

impl Answer {
    /// A header by (lower-case) name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

impl std::fmt::Debug for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answer")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

/// Sends the transfer's GETs. The URL is checked by the caller.
pub trait Fetch: Send + Sync {
    /// A plain GET of `url`, from byte `from` on when `Some`.
    ///
    /// # Errors
    ///
    /// Why no answer arrived.
    fn get(&self, url: &str, from: Option<u64>) -> Result<Answer, String>;
}

/// The production fetch: the app's own HTTPS client (the account
/// client's ureq + rustls with the system's trust store).
pub struct HttpsFetch(juggler_account::UreqTransport);

impl std::fmt::Debug for HttpsFetch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpsFetch").finish_non_exhaustive()
    }
}

impl HttpsFetch {
    /// The client, identifying only as `AIJuggler/<version>`.
    #[must_use]
    pub fn new() -> Self {
        Self(juggler_account::UreqTransport::new(env!(
            "CARGO_PKG_VERSION"
        )))
    }
}

impl Default for HttpsFetch {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetch for HttpsFetch {
    fn get(&self, url: &str, from: Option<u64>) -> Result<Answer, String> {
        let answer = self.0.get_stream(url, from).map_err(|error| error.0)?;
        Ok(Answer {
            status: answer.status,
            headers: answer.headers,
            body: answer.body,
        })
    }
}

/// What the transfer reads of this Mac, through the OS seams (D-52).
pub trait Machine: Send + Sync {
    /// Low Power Mode is on (unknown: no).
    fn low_power(&self) -> bool;
    /// The link is metered: a hotspot, cellular, Low Data Mode (unknown:
    /// no, since the user asked for this one file).
    fn metered(&self) -> bool;
    /// Free and total space of the disk `dir` is on; `None` when it
    /// can't be read (the transfer then waits: unknown fails closed).
    fn disk(&self, dir: &Path) -> Option<DiskSpace>;
}

/// This Mac, read through the seams that exist: Low Power Mode from the
/// machine readings (`juggler-supervisor`), free disk from
/// [`juggler_platform::room`]. The metered flag is the Apple helper's
/// `NWPathMonitor` reading (ADR-0033 §10), which the helper doesn't
/// carry yet: until it does, a link is never taken for metered. Other
/// systems read no Low Power Mode, and no disk off Unix (D-52 stubs).
#[derive(Debug, Default, Clone, Copy)]
pub struct HostMachine;

impl Machine for HostMachine {
    fn low_power(&self) -> bool {
        use juggler_supervisor::machine::OsReadings as _;
        juggler_supervisor::machine::HostReadings
            .power()
            .is_some_and(|power| power.low_power_mode)
    }

    fn metered(&self) -> bool {
        false
    }

    fn disk(&self, dir: &Path) -> Option<DiskSpace> {
        // The folder may not exist before the first download: ask its
        // nearest existing parent (the same disk).
        let mut at = Some(dir);
        while let Some(path) = at {
            if path.exists() {
                return juggler_platform::room::disk_space(path).ok();
            }
            at = path.parent();
        }
        None
    }
}

/// Why a transfer waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(
    export,
    rename = "DownloadWait",
    export_to = "../../../ui/src/types/generated/"
)]
pub enum Wait {
    /// Low Power Mode is on.
    LowPower,
    /// The link is metered.
    Metered,
    /// Free disk is below the bytes still to fetch plus the margin.
    DiskShort {
        /// The free disk it needs.
        #[ts(type = "number")]
        needed: u64,
        /// The free disk now.
        #[ts(type = "number")]
        free: u64,
    },
    /// The disk's free space can't be read.
    DiskUnknown,
}

/// Whether the transfer may go on now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// Go on.
    Go,
    /// Wait, keeping the part file.
    Wait(Wait),
    /// Stop and delete the part file: the disk is below the local cache's
    /// low-disk line.
    Stop,
}

/// Whether a transfer with `remaining` bytes still to fetch into `dir`
/// may go on, keeping `margin` free on top (pure but for the readings).
#[must_use]
pub fn hold(machine: &dyn Machine, dir: &Path, remaining: u64, margin: u64) -> Hold {
    if machine.low_power() {
        return Hold::Wait(Wait::LowPower);
    }
    if machine.metered() {
        return Hold::Wait(Wait::Metered);
    }
    let Some(disk) = machine.disk(dir) else {
        return Hold::Wait(Wait::DiskUnknown);
    };
    if disk.free < crate::features::cache::budget::low_disk_line(disk) {
        return Hold::Stop;
    }
    let needed = remaining.saturating_add(margin);
    if disk.free < needed {
        return Hold::Wait(Wait::DiskShort {
            needed,
            free: disk.free,
        });
    }
    Hold::Go
}

/// Why a transfer ended without a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The user cancelled it.
    Cancelled,
    /// The entry's URL breaks the URL rules (never sent).
    UrlRefused(&'static str),
    /// A redirect to a host off the source's list, or one that isn't a
    /// plain https URL (never followed).
    RedirectRefused,
    /// More than [`source::MAX_REDIRECTS`] redirects.
    TooManyRedirects,
    /// A `Content-Length` or `Content-Range` other than the entry's size.
    SizeMismatch,
    /// A byte past the entry's size.
    TooLong,
    /// The bytes' SHA-256 isn't the entry's.
    HashMismatch,
    /// The disk fell below the low-disk line while it ran.
    DiskFull,
    /// No answer, a dropped connection or an error status, too often.
    Network(String),
    /// The file couldn't be written or renamed.
    Disk(String),
}

/// The transfer's knobs; [`Tuning::default`] is production's.
#[derive(Debug, Clone)]
pub struct Tuning {
    /// The bytes read at a time.
    pub chunk: usize,
    /// The most bytes a second (ADR-0033 §5: 4 MB/s, so it never competes
    /// with an agent's work); `None`: no cap.
    pub rate: Option<u64>,
    /// How often a paused transfer reads this Mac again.
    pub pause_poll: Duration,
    /// How often between chunks it reads this Mac.
    pub check_every: Duration,
    /// How often it reports its progress.
    pub report_every: Duration,
    /// The first wait after a failure; each next one doubles, up to
    /// [`Tuning::longest_wait`].
    pub first_wait: Duration,
    /// The longest wait after a failure.
    pub longest_wait: Duration,
    /// The failures in a row (no byte gained between them) that end it.
    pub attempts: u32,
    /// The failures in a row after which the mirror is tried.
    pub mirror_after: u32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            chunk: 1024 * 1024,
            rate: Some(4_000_000),
            pause_poll: Duration::from_secs(10),
            check_every: Duration::from_secs(1),
            report_every: Duration::from_millis(500),
            first_wait: Duration::from_secs(2),
            longest_wait: Duration::from_secs(60),
            attempts: 6,
            mirror_after: 2,
        }
    }
}

/// A running transfer's cancel and wake-up, shared with whoever asked
/// for it.
#[derive(Debug, Default)]
pub struct Control {
    cancelled: Mutex<bool>,
    bell: Condvar,
    waits: AtomicU64,
}

impl Control {
    /// Cancels it: the transfer stops at its next read or wait, and its
    /// part file goes.
    pub fn cancel(&self) {
        if let Ok(mut cancelled) = self.cancelled.lock() {
            *cancelled = true;
        }
        self.bell.notify_all();
    }

    /// Whether it was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.lock().map_or(true, |cancelled| *cancelled)
    }

    /// Wakes a waiting transfer to read this Mac again now (power or
    /// network changed).
    pub fn nudge(&self) {
        self.bell.notify_all();
    }

    /// How many times it has waited (a paused transfer waits once per
    /// [`Tuning::pause_poll`], so this proves it isn't spinning).
    #[must_use]
    pub fn waits(&self) -> u64 {
        self.waits.load(Ordering::Relaxed)
    }

    /// Waits up to `limit`, or until cancelled or nudged; true when
    /// cancelled.
    fn wait(&self, limit: Duration) -> bool {
        self.waits.fetch_add(1, Ordering::Relaxed);
        let Ok(cancelled) = self.cancelled.lock() else {
            return true;
        };
        if *cancelled {
            return true;
        }
        self.bell
            .wait_timeout(cancelled, limit)
            .map_or(true, |(cancelled, _)| *cancelled)
    }
}

/// Where a transfer stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Bytes are arriving.
    Running,
    /// Waiting for this Mac.
    Waiting(Wait),
    /// Every byte is in; checking the hash.
    Verifying,
}

/// A progress report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Bytes on disk.
    pub done: u64,
    /// The file's size.
    pub size: u64,
    /// Where it stands.
    pub phase: Phase,
}

/// What a transfer fetches: the file's hash and size, the margin to
/// keep free, and its sources, the pinned URL first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The file's SHA-256, lower-case hex (its name on disk).
    pub sha256: String,
    /// Its exact size.
    pub size_bytes: u64,
    /// The free disk kept on top of the bytes still to fetch.
    pub margin_bytes: u64,
    /// The pinned URL and its source, then the mirror's.
    pub sources: Vec<(String, Source)>,
}

impl Plan {
    /// The plan for `entry`'s file, every URL checked again by the one
    /// parser.
    ///
    /// # Errors
    ///
    /// [`Failure::UrlRefused`]: no file (Apple's model), a hash that
    /// isn't 64 lower-case hex, or a URL off the rules.
    pub fn of(entry: &ModelEntry) -> Result<Self, Failure> {
        let download = entry
            .download
            .as_ref()
            .ok_or(Failure::UrlRefused("no file to fetch"))?;
        let sha = &download.sha256;
        let hash_ok = sha.len() == 64
            && sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !hash_ok {
            return Err(Failure::UrlRefused("not a SHA-256"));
        }
        let mut sources = vec![(
            download.url.clone(),
            source::pinned(&download.url, sha).map_err(Failure::UrlRefused)?,
        )];
        if let Some(mirror) = &download.mirror_url {
            sources.push((
                mirror.clone(),
                source::pinned(mirror, sha).map_err(Failure::UrlRefused)?,
            ));
        }
        Ok(Self {
            sha256: sha.clone(),
            size_bytes: download.size_bytes,
            margin_bytes: entry.disk_margin(),
            sources,
        })
    }

    /// The finished file's path in `dir`.
    #[must_use]
    pub fn file_in(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.{MODEL_EXTENSION}", self.sha256))
    }

    /// The part file's path in `dir` for download number `job` (each
    /// download has its own, so a cancelled one that is still winding
    /// down never touches the next one's).
    #[must_use]
    pub fn part_in(&self, dir: &Path, job: u64) -> PathBuf {
        dir.join(format!("{}-{job}.{PART_EXTENSION}", self.sha256))
    }
}

/// What a transfer works with.
pub struct Run<'a> {
    /// What it fetches.
    pub plan: &'a Plan,
    /// The models' folder.
    pub dir: &'a Path,
    /// The part file, in that folder.
    pub part: &'a Path,
    /// The network.
    pub fetch: &'a dyn Fetch,
    /// This Mac.
    pub machine: &'a dyn Machine,
    /// The cancel and wake-up.
    pub control: &'a Control,
    /// The knobs.
    pub tuning: &'a Tuning,
}

/// Fetches `run.plan`'s file into `run.dir` as `<sha256>.gguf`, reporting
/// progress to `report`; the file's path once verified.
///
/// # Errors
///
/// Why it ended without a file; the part file is gone then.
pub fn run(run: &Run<'_>, report: &mut dyn FnMut(Progress)) -> Result<PathBuf, Failure> {
    let result = transfer(run, run.part, report);
    if result.is_err() {
        let _ = std::fs::remove_file(run.part);
    }
    result
}

/// The part file, the hash of what is in it, and its length.
struct Partial {
    file: File,
    hasher: Sha256,
    done: u64,
}

impl Partial {
    fn create(path: &Path) -> Result<Self, Failure> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .map_err(|error| Failure::Disk(error.to_string()))?;
        Ok(Self {
            file,
            hasher: Sha256::new(),
            done: 0,
        })
    }

    /// Back to nothing: the host sent the whole file again.
    fn restart(&mut self) -> Result<(), Failure> {
        self.file
            .set_len(0)
            .and_then(|()| self.file.seek(SeekFrom::Start(0)).map(|_| ()))
            .map_err(|error| Failure::Disk(error.to_string()))?;
        self.hasher = Sha256::new();
        self.done = 0;
        Ok(())
    }
}

/// How one response's body ended.
enum Streamed {
    /// The body ended.
    Ended,
    /// This Mac said wait: the connection is dropped, the part kept.
    Paused,
}

fn transfer(
    run: &Run<'_>,
    part: &Path,
    report: &mut dyn FnMut(Progress),
) -> Result<PathBuf, Failure> {
    let size = run.plan.size_bytes;
    std::fs::create_dir_all(run.dir).map_err(|error| Failure::Disk(error.to_string()))?;
    let mut partial = Partial::create(part)?;
    let mut failures: u32 = 0;
    let mut done_at_failure = 0;
    loop {
        if run.control.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if partial.done == size {
            break;
        }
        match hold(
            run.machine,
            run.dir,
            size - partial.done,
            run.plan.margin_bytes,
        ) {
            Hold::Go => {}
            Hold::Stop => return Err(Failure::DiskFull),
            Hold::Wait(why) => {
                report(Progress {
                    done: partial.done,
                    size,
                    phase: Phase::Waiting(why),
                });
                if run.control.wait(run.tuning.pause_poll) {
                    return Err(Failure::Cancelled);
                }
                continue;
            }
        }
        let use_mirror = failures >= run.tuning.mirror_after && run.plan.sources.len() > 1;
        let (url, source) = &run.plan.sources[usize::from(use_mirror)];
        let outcome = open(run, url, *source, partial.done).and_then(|opened| {
            if opened.from != partial.done {
                partial.restart()?;
            }
            stream(run, opened.body, &mut partial, report)
        });
        match outcome {
            Ok(Streamed::Paused) => {}
            Ok(Streamed::Ended) if partial.done == size => break,
            Ok(Streamed::Ended) => {
                failure(
                    run,
                    &mut failures,
                    &mut done_at_failure,
                    partial.done,
                    "the connection ended early",
                )?;
            }
            Err(Failure::Network(why)) => {
                failure(run, &mut failures, &mut done_at_failure, partial.done, &why)?;
            }
            Err(other) => return Err(other),
        }
    }
    report(Progress {
        done: size,
        size,
        phase: Phase::Verifying,
    });
    finish(run, part, partial)
}

/// Counts a failure that may be retried: ends the transfer after too
/// many in a row with no byte gained, else waits before the next try.
fn failure(
    run: &Run<'_>,
    failures: &mut u32,
    done_at_failure: &mut u64,
    done: u64,
    why: &str,
) -> Result<(), Failure> {
    if done > *done_at_failure {
        *failures = 0;
    }
    *done_at_failure = done;
    *failures += 1;
    if *failures >= run.tuning.attempts {
        return Err(Failure::Network(why.to_owned()));
    }
    let doubling = 1u32 << (*failures - 1).min(16);
    let wait = run
        .tuning
        .first_wait
        .saturating_mul(doubling)
        .min(run.tuning.longest_wait);
    if run.control.wait(wait) {
        return Err(Failure::Cancelled);
    }
    Ok(())
}

/// An opened body and the byte it starts at.
struct Opened {
    from: u64,
    body: Box<dyn Read + Send>,
}

/// The first byte, the last byte and the total of a `Content-Range`.
fn content_range(value: &str) -> Option<(u64, u64, u64)> {
    let rest = value.trim().strip_prefix("bytes ")?;
    let (span, total) = rest.split_once('/')?;
    let (first, last) = span.split_once('-')?;
    Some((
        first.trim().parse().ok()?,
        last.trim().parse().ok()?,
        total.trim().parse().ok()?,
    ))
}

fn content_length(answer: &Answer) -> Option<Result<u64, Failure>> {
    answer
        .header("content-length")
        .map(|value| value.trim().parse().map_err(|_| Failure::SizeMismatch))
}

/// GETs `url` from byte `from`, following at most
/// [`source::MAX_REDIRECTS`] redirects, each checked against `source`'s
/// hosts; the body and the byte it starts at, after its size is checked.
fn open(run: &Run<'_>, url: &str, source: Source, from: u64) -> Result<Opened, Failure> {
    let size = run.plan.size_bytes;
    // The pinned URL is checked again right before it is sent.
    source::pinned(url, &run.plan.sha256).map_err(Failure::UrlRefused)?;
    let range = (from > 0).then_some(from);
    let mut at = url.to_owned();
    let mut hops = 0;
    loop {
        if run.control.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        let answer = run.fetch.get(&at, range).map_err(Failure::Network)?;
        match answer.status {
            301 | 302 | 303 | 307 | 308 => {
                hops += 1;
                if hops > source::MAX_REDIRECTS {
                    return Err(Failure::TooManyRedirects);
                }
                let next = answer.header("location").ok_or(Failure::RedirectRefused)?;
                if !source::redirect_allowed(source, next) {
                    return Err(Failure::RedirectRefused);
                }
                next.clone_into(&mut at);
            }
            200 => {
                if let Some(length) = content_length(&answer)
                    && length? != size
                {
                    return Err(Failure::SizeMismatch);
                }
                return Ok(Opened {
                    from: 0,
                    body: answer.body,
                });
            }
            206 => {
                let (first, last, total) = answer
                    .header("content-range")
                    .and_then(content_range)
                    .ok_or(Failure::SizeMismatch)?;
                if total != size {
                    return Err(Failure::SizeMismatch);
                }
                if first != from || last.checked_add(1) != Some(size) {
                    // Not the bytes asked for: start again from 0.
                    return restart_from_zero(
                        run,
                        url,
                        source,
                        from,
                        Failure::Network("an unexpected range".to_owned()),
                    );
                }
                if let Some(length) = content_length(&answer)
                    && length? != size - from
                {
                    return Err(Failure::SizeMismatch);
                }
                return Ok(Opened {
                    from,
                    body: answer.body,
                });
            }
            416 if from > 0 => {
                return restart_from_zero(
                    run,
                    url,
                    source,
                    from,
                    Failure::Network("the range was refused".to_owned()),
                );
            }
            status => return Err(Failure::Network(format!("status {status}"))),
        }
    }
}

/// A resume the host didn't honour: once, the whole file from the pinned
/// URL again (the caller empties the part file when `from` is 0).
fn restart_from_zero(
    run: &Run<'_>,
    url: &str,
    source: Source,
    from: u64,
    failure: Failure,
) -> Result<Opened, Failure> {
    if from == 0 {
        return Err(failure);
    }
    open(run, url, source, 0)
}

/// Writes `body` into the part file, hashing each byte, until it ends,
/// this Mac says wait, or a rule breaks.
fn stream(
    run: &Run<'_>,
    mut body: Box<dyn Read + Send>,
    partial: &mut Partial,
    report: &mut dyn FnMut(Progress),
) -> Result<Streamed, Failure> {
    let size = run.plan.size_bytes;
    let mut buffer = vec![0u8; run.tuning.chunk.max(1)];
    let started = Instant::now();
    let mut this_stream: u64 = 0;
    let mut checked = Instant::now();
    let mut reported = Instant::now();
    report(Progress {
        done: partial.done,
        size,
        phase: Phase::Running,
    });
    loop {
        let read = match body.read(&mut buffer) {
            Ok(0) => return Ok(Streamed::Ended),
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(Failure::Network(error.to_string())),
        };
        let bytes = &buffer[..read];
        let after = partial.done.saturating_add(read as u64);
        if after > size {
            return Err(Failure::TooLong);
        }
        partial
            .file
            .write_all(bytes)
            .map_err(|error| Failure::Disk(error.to_string()))?;
        partial.hasher.update(bytes);
        partial.done = after;
        this_stream = this_stream.saturating_add(read as u64);
        if run.control.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if checked.elapsed() >= run.tuning.check_every {
            checked = Instant::now();
            match hold(
                run.machine,
                run.dir,
                size - partial.done,
                run.plan.margin_bytes,
            ) {
                Hold::Go => {}
                Hold::Stop => return Err(Failure::DiskFull),
                Hold::Wait(_) => return Ok(Streamed::Paused),
            }
        }
        if reported.elapsed() >= run.tuning.report_every {
            reported = Instant::now();
            report(Progress {
                done: partial.done,
                size,
                phase: Phase::Running,
            });
        }
        if let Some(rate) = run.tuning.rate.filter(|rate| *rate > 0) {
            #[allow(
                clippy::cast_precision_loss,
                reason = "a rate cap: a few bytes off on a multi-GB count is nothing"
            )]
            let due = Duration::from_secs_f64(this_stream as f64 / rate as f64);
            let elapsed = started.elapsed();
            if let Some(ahead) = due.checked_sub(elapsed)
                && run.control.wait(ahead)
            {
                return Err(Failure::Cancelled);
            }
        }
    }
}

/// The last checks, then the atomic rename into place.
fn finish(run: &Run<'_>, part: &Path, partial: Partial) -> Result<PathBuf, Failure> {
    let Partial { file, hasher, .. } = partial;
    file.sync_all()
        .map_err(|error| Failure::Disk(error.to_string()))?;
    drop(file);
    let length = std::fs::metadata(part)
        .map_err(|error| Failure::Disk(error.to_string()))?
        .len();
    if length != run.plan.size_bytes {
        return Err(Failure::SizeMismatch);
    }
    let digest = hasher.finalize();
    if crate::crash::hex(&digest) != run.plan.sha256 {
        return Err(Failure::HashMismatch);
    }
    if run.control.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let file = run.plan.file_in(run.dir);
    std::fs::rename(part, &file).map_err(|error| Failure::Disk(error.to_string()))?;
    Ok(file)
}
