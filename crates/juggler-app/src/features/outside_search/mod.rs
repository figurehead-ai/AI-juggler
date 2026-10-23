//! AJ-E1538: searching the words of sessions started outside AI Juggler,
//! for people who turn it on (Settings › Privacy; off by default).
//!
//! - **Its own file.** The index is `outside-search.sqlite` in AI
//!   Juggler's data folder, readable by the user only, with the main
//!   store's index tables ([`juggler_core::storage::SearchIndexFile`]).
//!   Turning it off deletes the file, so nothing of it is left behind.
//! - **Built in the background**, from each tool's own files (read-only),
//!   newest first and incrementally: a pass works for at most
//!   [`PASS_BUDGET`], never while the conversations index is reading, so
//!   that index's first pass is as fast with this on.
//! - **Terms, not text.** The index is contentless: it holds the search
//!   terms and where each line is; a hit's words are read from the
//!   tool's own file. A conversation the tool deletes leaves the index
//!   on the next pass.
//!
//! AI Juggler's own sessions are indexed by AJ-E1007's index already;
//! this one covers only the conversations no AI Juggler session owns.

pub mod commands;
pub mod dto;

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_core::model::SessionId;
use juggler_core::storage::SearchIndexFile;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
pub use dto::{OutsideSearchEstimateInfo, OutsideSearchInfo};

/// Where the switch is kept.
const SETTING: &str = "outside_search";
/// The index's file, in AI Juggler's data folder.
const FILE: &str = "outside-search.sqlite";
/// The most one background pass works before it rests.
pub const PASS_BUDGET: Duration = Duration::from_secs(2);
/// How much of the sessions' files the size estimate indexes to measure.
const SAMPLE_BYTES: u64 = 32 * 1024 * 1024;
/// Lines written per transaction for a compressed file.
const LINES_PER_WRITE: usize = 2_000;

/// The index, while it is on.
#[derive(Debug, Default)]
pub struct OutsideSearch {
    index: Mutex<Option<SearchIndexFile>>,
    /// The last estimate, for the sessions it was made from.
    estimate: Mutex<Option<((u32, u64), OutsideSearchEstimateInfo)>>,
    /// Set when the switch is turned on: the builder starts at once.
    kick: AtomicBool,
}

fn index_path(state: &Arc<AppState>) -> PathBuf {
    state.data_dir.join(FILE)
}

/// The user turned it on.
///
/// # Errors
///
/// Storage errors.
pub fn enabled(state: &Arc<AppState>) -> AppResult<bool> {
    Ok(lock_storage_arc(state)?
        .app_settings()
        .get(SETTING)?
        .is_some_and(|value| value == "on"))
}

/// One outside conversation's file.
struct Source {
    key: SessionId,
    path: PathBuf,
    size: u64,
    compressed: bool,
}

/// The conversations no AI Juggler session owns, newest first.
fn sources(state: &Arc<AppState>) -> AppResult<Vec<Source>> {
    let owned: HashSet<String> = lock_storage_arc(state)?
        .sessions()
        .list(None)?
        .into_iter()
        .filter_map(|record| Some(record.conversation?.as_str().to_owned()))
        .collect();
    let mut conversations: Vec<_> = state
        .conversations
        .all()?
        .into_iter()
        .filter(|conversation| !owned.contains(&conversation.summary.id))
        .collect();
    conversations.sort_by_key(|conversation| std::cmp::Reverse(conversation.summary.last_ms));
    Ok(conversations
        .into_iter()
        .filter_map(|conversation| {
            let size = std::fs::metadata(&conversation.path).ok()?.len();
            Some(Source {
                key: SessionId::new(format!(
                    "{}:{}",
                    conversation.account_id, conversation.summary.id
                )),
                compressed: juggler_adapters::is_compressed(&conversation.path),
                path: conversation.path,
                size,
            })
        })
        .collect())
}

/// Runs `with` on the index, opening its file if the switch is on;
/// `None` while it is off. The switch is read under the index's lock,
/// which turning it off holds too, so a pass can never open the file
/// again after it was deleted.
fn with_index<T>(
    state: &Arc<AppState>,
    with: impl FnOnce(&SearchIndexFile) -> AppResult<T>,
) -> AppResult<Option<T>> {
    let mut index = state
        .outside_search
        .index
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    if !enabled(state)? {
        return Ok(None);
    }
    if index.is_none() {
        *index = Some(SearchIndexFile::open(&index_path(state))?);
    }
    index.as_ref().map(with).transpose()
}

/// Turns it on (building starts in the background) or off (the index's
/// file is deleted).
///
/// # Errors
///
/// Storage errors; [`AppError::Io`] when the file can't be deleted.
pub fn set_impl(state: &Arc<AppState>, on: bool) -> AppResult<OutsideSearchInfo> {
    {
        let mut index = state
            .outside_search
            .index
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        lock_storage_arc(state)?
            .app_settings()
            .set(SETTING, if on { "on" } else { "" })?;
        if on {
            if index.is_none() {
                *index = Some(SearchIndexFile::open(&index_path(state))?);
            }
        } else {
            drop(index.take()); // closes the file before it goes
            SearchIndexFile::remove(&index_path(state)).map_err(|error| AppError::Io {
                context: "delete the search index",
                message: error.to_string(),
            })?;
        }
    }
    if on {
        state.outside_search.kick.store(true, Ordering::Release);
    }
    status_impl(state)
}

/// Whether it is on, and how far the index has got.
///
/// # Errors
///
/// Storage errors.
pub fn status_impl(state: &Arc<AppState>) -> AppResult<OutsideSearchInfo> {
    let sources = sources(state)?;
    let sessions = u32::try_from(sources.len()).unwrap_or(u32::MAX);
    let built = with_index(state, |index| {
        let states = index.repo().states()?;
        let indexed = sources
            .iter()
            .filter(|source| {
                states.get(source.key.as_str()).is_some_and(|done| {
                    done.path == source.path.to_string_lossy() && done.indexed_bytes == source.size
                })
            })
            .count();
        Ok((
            index.size_bytes()?,
            u32::try_from(indexed).unwrap_or(u32::MAX),
        ))
    })?;
    Ok(OutsideSearchInfo {
        enabled: built.is_some(),
        size_bytes: built.map(|(size, _)| size),
        sessions,
        indexed: built.map_or(0, |(_, indexed)| indexed),
    })
}

/// What turning it on would take: the index's likely size, measured by
/// indexing a spread of the sessions' files in memory (all of them, when
/// they come to less than 32 MB) and scaling by their bytes,
/// compressed and plain files apart.
///
/// # Errors
///
/// Storage errors.
pub fn estimate_impl(state: &Arc<AppState>) -> AppResult<OutsideSearchEstimateInfo> {
    estimate_with(state, SAMPLE_BYTES)
}

/// [`estimate_impl`], indexing at most `sample_bytes` of each kind of
/// file to measure (the evidence drives a small sample over many files).
///
/// # Errors
///
/// Storage errors.
pub fn estimate_with(
    state: &Arc<AppState>,
    sample_bytes: u64,
) -> AppResult<OutsideSearchEstimateInfo> {
    let sources = sources(state)?;
    let signature = (
        u32::try_from(sources.len()).unwrap_or(u32::MAX),
        sources.iter().map(|source| source.size).sum::<u64>(),
    );
    if sample_bytes == SAMPLE_BYTES
        && let Ok(cached) = state.outside_search.estimate.lock()
        && let Some((seen, estimate)) = cached.as_ref()
        && *seen == signature
    {
        return Ok(estimate.clone());
    }
    let empty = SearchIndexFile::in_memory()?.size_bytes()?;
    let mut size_bytes = empty;
    for compressed in [false, true] {
        let mut group: Vec<&Source> = sources
            .iter()
            .filter(|source| source.compressed == compressed && source.size > 0)
            .collect();
        let total: u64 = group.iter().map(|source| source.size).sum();
        if total == 0 {
            continue;
        }
        // Points spread evenly over the group's bytes, so a big file is
        // sampled as often as its share of the bytes (sampling by file
        // count only ever saw the small ones, which index densest). The
        // same points on every run.
        group.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));
        let sample = SearchIndexFile::in_memory()?;
        size_bytes += if total <= sample_bytes {
            // Small enough to index all of it: the size itself.
            let covered: u64 = group
                .iter()
                .filter(|source| index_whole(&sample, source).is_ok())
                .map(|source| source.size)
                .sum();
            sample.optimize()?;
            scaled(sample.size_bytes()?.saturating_sub(empty), covered, total)
        } else if compressed {
            let covered = sample_whole(&sample, &group, total, sample_bytes);
            sample.optimize()?;
            scaled(sample.size_bytes()?.saturating_sub(empty), covered, total)
        } else {
            windows_estimate(&sample, &group, total, sample_bytes, empty)?
        };
    }
    let estimate = OutsideSearchEstimateInfo {
        size_bytes,
        sessions: signature.0,
    };
    if sample_bytes == SAMPLE_BYTES
        && let Ok(mut cached) = state.outside_search.estimate.lock()
    {
        *cached = Some((signature, estimate.clone()));
    }
    Ok(estimate)
}

/// How many points the estimate reads a group at: enough that one odd
/// file doesn't sway it.
const POINTS: u64 = 64;

/// The bytes of whole lines read at each point: the sample shared by the
/// points (512 KB each on a real setup, never under 4 KB).
fn window_bytes(sample_bytes: u64) -> u64 {
    (sample_bytes / POINTS).clamp(4 * 1024, 512 * 1024)
}

/// The files at `points` evenly spaced points over `group`'s `total`
/// bytes, and each point's offset in its file.
fn at_points<'a>(group: &[&'a Source], total: u64, points: u64) -> Vec<(&'a Source, u64)> {
    let mut picked = Vec::new();
    let mut files = group.iter().peekable();
    let mut before = 0_u64;
    for point in 0..points {
        // The middle of each of `points` equal stretches of the bytes.
        let at = u64::try_from(
            (u128::from(total) * (2 * u128::from(point) + 1)) / (2 * u128::from(points)),
        )
        .unwrap_or(u64::MAX);
        while let Some(file) = files.peek() {
            if at < before + file.size {
                picked.push((**file, at - before));
                break;
            }
            before += file.size;
            files.next();
        }
    }
    picked
}

/// The index a group of plain files would take, from a window of whole
/// lines at each of its points. Each point is an equal share of the bytes,
/// however long the lines it lands in, so the group's words are its bytes
/// times the words per byte averaged over the points (weighting by the
/// bytes read let long lines count for more than their share); the index
/// is those words at the sample's own index bytes per word.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn windows_estimate(
    sample: &SearchIndexFile,
    group: &[&Source],
    total: u64,
    sample_bytes: u64,
    empty: u64,
) -> AppResult<u64> {
    let window = window_bytes(sample_bytes);
    let points = (sample_bytes / window).max(1);
    let mut per_byte = Vec::new();
    let mut words = 0_u64;
    for (n, (source, at)) in at_points(group, total, points).into_iter().enumerate() {
        // Whole lines from the one the point falls in.
        let Ok((lines, bytes)) = crate::transcript_text::window_lines(&source.path, at, window)
        else {
            continue;
        };
        if bytes == 0 {
            continue;
        }
        let key = SessionId::new(format!("{}#{n}", source.key.as_str()));
        let path = source.path.to_string_lossy().into_owned();
        if sample.repo().append(&key, &path, &lines, bytes).is_ok() {
            let here: u64 = lines.iter().map(|(_, _, text)| word_count(text)).sum();
            words += here;
            per_byte.push(here as f64 / bytes as f64);
        }
    }
    if per_byte.is_empty() || words == 0 {
        return Ok(0);
    }
    sample.optimize()?;
    let per_word = sample.size_bytes()?.saturating_sub(empty) as f64 / words as f64;
    let density = per_byte.iter().sum::<f64>() / per_byte.len() as f64;
    Ok((total as f64 * density * per_word) as u64)
}

/// The words the index's tokenizer makes of `text`, near enough: runs of
/// letters and digits.
fn word_count(text: &str) -> u64 {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .count() as u64
}

/// Indexes the compressed files at evenly spaced points over their bytes,
/// each whole (a compressed file is read whole), until about
/// `sample_bytes` of them; the compressed bytes they hold.
fn sample_whole(sample: &SearchIndexFile, group: &[&Source], total: u64, sample_bytes: u64) -> u64 {
    let mut seen = HashSet::new();
    let mut covered = 0_u64;
    for (source, _) in at_points(group, total, POINTS) {
        if covered >= sample_bytes / 8 {
            break;
        }
        if seen.insert(source.key.as_str()) && index_whole(sample, source).is_ok() {
            covered += source.size;
        }
    }
    covered
}

/// The index for `total` bytes of files, from a sample of `sampled` bytes
/// that built `built`: in proportion to the bytes.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn scaled(built: u64, sampled: u64, total: u64) -> u64 {
    match sampled {
        0 => 0,
        _ if sampled >= total => built,
        _ => (built as f64 * total as f64 / sampled as f64) as u64,
    }
}

/// Indexes all of `source` into `index` (the estimate's sample).
fn index_whole(index: &SearchIndexFile, source: &Source) -> AppResult<()> {
    let path = source.path.to_string_lossy().into_owned();
    let io = |error: std::io::Error| AppError::Io {
        context: "read a session's file",
        message: error.to_string(),
    };
    if source.compressed {
        let lines = crate::transcript_text::compressed_lines(&source.path).map_err(io)?;
        index
            .repo()
            .append(&source.key, &path, &lines, source.size)?;
        return Ok(());
    }
    let mut start = 0;
    while start < source.size {
        let (lines, next) =
            crate::transcript_text::read_lines(&source.path, start, source.size).map_err(io)?;
        if next == start {
            break;
        }
        index.repo().append(&source.key, &path, &lines, next)?;
        start = next;
    }
    Ok(())
}

/// One background pass: brings the index up to date with the outside
/// sessions' files, newest first, for at most `budget`; forgets the
/// sessions whose files are gone. `true` when there is more to do.
///
/// # Errors
///
/// Storage errors (an unreadable file is skipped).
pub fn pass(state: &Arc<AppState>, budget: Duration) -> AppResult<bool> {
    // Never while the conversations index reads (its first pass stays as
    // fast with this on), and not before it has read anything.
    if state.conversations.is_reading() {
        return Ok(true);
    }
    if !enabled(state)? || state.conversations.all()?.is_empty() {
        return Ok(false);
    }
    let sources = sources(state)?;
    let Some(states) = with_index(state, |index| Ok(index.repo().states()?))? else {
        return Ok(false);
    };
    let live: HashSet<&str> = sources.iter().map(|source| source.key.as_str()).collect();
    for gone in states.keys().filter(|key| !live.contains(key.as_str())) {
        with_index(state, |index| {
            Ok(index.repo().forget(&SessionId::new(gone.as_str()))?)
        })?;
    }
    let deadline = Instant::now() + budget;
    let mut wrote = false;
    for source in &sources {
        if Instant::now() >= deadline {
            return Ok(true);
        }
        wrote |= index_more(state, source, &states, deadline)?;
    }
    if wrote {
        // Caught up: one segment, so the file is as small as it can be.
        with_index(state, |index| Ok(index.optimize()?))?;
    }
    Ok(false)
}

/// Indexes what `source` gained since `states` was read, until `deadline`.
fn index_more(
    state: &Arc<AppState>,
    source: &Source,
    states: &BTreeMap<String, juggler_core::storage::IndexState>,
    deadline: Instant,
) -> AppResult<bool> {
    let path = source.path.to_string_lossy().into_owned();
    let done = states.get(source.key.as_str());
    let same = done.is_some_and(|done| done.path == path && done.indexed_bytes <= source.size);
    if source.compressed
        && done.is_some_and(|done| done.path == path && done.indexed_bytes == source.size)
    {
        return Ok(false); // a finished rollout doesn't grow
    }
    let mut start = if same && !source.compressed {
        done.map_or(0, |done| done.indexed_bytes)
    } else {
        if done.is_some() {
            with_index(state, |index| Ok(index.repo().forget(&source.key)?))?;
        }
        0
    };
    if source.compressed {
        let Ok(lines) = crate::transcript_text::compressed_lines(&source.path) else {
            return Ok(false);
        };
        let batches: Vec<&[(u64, u64, String)]> = lines.chunks(LINES_PER_WRITE).collect();
        let last = batches.len().saturating_sub(1);
        for (at, batch) in batches.iter().enumerate() {
            // Only the last write marks the file done: an interrupted one
            // is indexed again from the start.
            let indexed = if at == last { source.size } else { 0 };
            with_index(state, |index| {
                Ok(index.repo().append(&source.key, &path, batch, indexed)?)
            })?;
        }
        if lines.is_empty() {
            with_index(state, |index| {
                Ok(index.repo().append(&source.key, &path, &[], source.size)?)
            })?;
        }
        return Ok(true);
    }
    let mut wrote = false;
    while start < source.size && Instant::now() < deadline {
        let Ok((lines, next)) =
            crate::transcript_text::read_lines(&source.path, start, source.size)
        else {
            return Ok(wrote);
        };
        if next == start {
            break; // a partial line; wait for the rest
        }
        with_index(state, |index| {
            Ok(index.repo().append(&source.key, &path, &lines, next)?)
        })?;
        wrote = true;
        start = next;
    }
    Ok(wrote)
}

/// A hit in an outside session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutsideHit {
    /// The conversation's account.
    pub account_id: String,
    /// The tool's id for the conversation.
    pub conversation_id: String,
    /// The words around the match, read from the tool's own file.
    pub snippet: String,
}

/// Searches the outside sessions' words: nothing while it is off.
///
/// # Errors
///
/// Storage errors.
pub fn search(state: &Arc<AppState>, query: &str, limit: usize) -> AppResult<Vec<OutsideHit>> {
    let Some((hits, paths)) = with_index(state, |index| {
        let hits = index.repo().search(query, limit)?;
        let paths: BTreeMap<String, String> = index
            .repo()
            .states()?
            .into_iter()
            .map(|(key, done)| (key, done.path))
            .collect();
        Ok((hits, paths))
    })?
    else {
        return Ok(Vec::new());
    };
    Ok(hits
        .into_iter()
        .filter_map(|hit| {
            let (account_id, conversation_id) = hit.session.split_once(':')?;
            let path = paths.get(&hit.session)?;
            Some(OutsideHit {
                account_id: account_id.to_owned(),
                conversation_id: conversation_id.to_owned(),
                snippet: crate::transcript_text::snippet(
                    std::path::Path::new(path),
                    hit.offset,
                    hit.len,
                    query,
                )?,
            })
        })
        .collect())
}

/// Starts the background builder: a pass while there is work, resting
/// between passes; a minute between checks once it has caught up, or at
/// once when the switch is turned on.
pub fn spawn(state: Arc<AppState>) {
    let _ = std::thread::Builder::new()
        .name("outside-search".to_owned())
        .spawn(move || {
            loop {
                let more = pass(&state, PASS_BUDGET).unwrap_or(false);
                let rest = if more { 3 } else { 60 };
                for _ in 0..rest {
                    if state.outside_search.kick.swap(false, Ordering::AcqRel) {
                        break;
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        });
}
