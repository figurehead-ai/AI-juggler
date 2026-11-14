//! 0.4.5 (AJ-E805): one reader per transcript, shared. The conversation
//! list and the usage reading both take from it: each transcript is read
//! once, as it grows, by its tool's reader
//! ([`juggler_adapter::ToolTranscripts::transcript_reader`]), never whole
//! again. Before, the conversation list's 15 s pass read every changed
//! transcript whole again for its tokens while the usage reading tailed the
//! same file, so a working session cost more the longer it ran.
//!
//! Each file keeps where its next read starts, its reader (what the file
//! says so far, its tokens) and the usage read since the usage reading last
//! took it. A file that got shorter, or was replaced, is read again from
//! its start with a new reader; a compressed one (written once) is read
//! whole again whenever its size changes. Nothing here is saved: after a
//! start, a file is read from its start the first time it is needed (the
//! usage reading drops what it had stored before, by offset).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use juggler_adapter::conversation::ConversationSummary;
use juggler_adapter::transcript_read::{ConversationFile, TranscriptReader, UsageRead};
use juggler_adapters::transcript_follow::{Step, Void, follow};

/// One transcript as read so far.
struct Tracked {
    /// Where the next read starts (a compressed file's: its size).
    offset: u64,
    /// The bytes just before `offset` (up to [`SEAM`]): a file rewritten
    /// rather than appended to shows others there, and is read again from
    /// its start.
    seam: Vec<u8>,
    /// Its reader, with its tokens.
    file: ConversationFile,
    /// Usage read since the usage reading last took it, each slice with
    /// the offset it ended at (`u64::MAX`: a compressed file's, whole).
    usage: Vec<(u64, UsageRead)>,
    /// Read again from its start since the usage reading last took it:
    /// everything queued is new to it, whatever it had stored.
    restarted: bool,
}

/// Every transcript's reader (one per app, in `AppState`).
#[derive(Default)]
pub struct Readers {
    files: Mutex<HashMap<Key, Shared>>,
}

/// A file's reader by its account and path.
type Key = (String, PathBuf);

/// One file's reader, held by whichever pass reads it.
type Shared = Arc<Mutex<Tracked>>;

impl std::fmt::Debug for Readers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Readers").finish_non_exhaustive()
    }
}

/// The reader a tool gives for one of its files, when it gives one.
type Make<'a> = &'a dyn Fn() -> Option<Box<dyn TranscriptReader>>;

impl Readers {
    /// `path` (of `account`, whose tool is `adapter`) read on to its end:
    /// what it says about itself, `Some(None)` when that isn't a
    /// conversation. `None`: its tool reads it whole instead (no reader).
    pub fn summary(
        &self,
        adapter: &dyn juggler_adapter::HarnessAdapter,
        account: &str,
        path: &Path,
    ) -> Option<Option<ConversationSummary>> {
        let tracked = self.advance(adapter, account, path)?;
        let tracked = tracked.lock().ok()?;
        Some(tracked.file.summary(path))
    }

    /// `path` read on to its end, and the usage read from it since the last
    /// take: where it was read to, and the usage, without the slices that
    /// ended at or before `stored` (what the caller stored already) unless
    /// the file was read again from its start since. `None`: no reader.
    pub fn take_usage(
        &self,
        adapter: &dyn juggler_adapter::HarnessAdapter,
        account: &str,
        path: &Path,
        stored: u64,
    ) -> Option<(u64, UsageRead)> {
        let tracked = self.advance(adapter, account, path)?;
        let mut tracked = tracked.lock().ok()?;
        let restarted = std::mem::take(&mut tracked.restarted);
        let mut usage = UsageRead::default();
        for (end, slice) in std::mem::take(&mut tracked.usage) {
            if restarted || end > stored {
                usage.append(slice);
            }
        }
        Some((tracked.offset, usage))
    }

    /// Whether `path` was read before (a read now takes only what was
    /// written since).
    #[must_use]
    pub fn is_read(&self, account: &str, path: &Path) -> bool {
        self.files
            .lock()
            .is_ok_and(|files| files.contains_key(&(account.to_owned(), path.to_path_buf())))
    }

    /// Forgets `path`'s reader (the file is gone).
    pub fn forget(&self, account: &str, path: &Path) {
        if let Ok(mut files) = self.files.lock() {
            files.remove(&(account.to_owned(), path.to_path_buf()));
        }
    }

    /// Keeps only the readers whose account and file `keep` holds.
    pub fn retain(&self, keep: impl Fn(&str, &Path) -> bool) {
        if let Ok(mut files) = self.files.lock() {
            files.retain(|(account, path), _| keep(account, path));
        }
    }

    /// `path`'s reader, read on to the file's end (made at its first read).
    fn advance(
        &self,
        adapter: &dyn juggler_adapter::HarnessAdapter,
        account: &str,
        path: &Path,
    ) -> Option<Shared> {
        let make = || adapter.transcript_reader(path, account);
        let tracked = {
            let mut files = self.files.lock().ok()?;
            let key = (account.to_owned(), path.to_path_buf());
            if let Some(tracked) = files.get(&key) {
                Arc::clone(tracked)
            } else {
                let tracked = Arc::new(Mutex::new(Tracked {
                    offset: 0,
                    seam: Vec::new(),
                    file: ConversationFile::new(make()?),
                    usage: Vec::new(),
                    restarted: false,
                }));
                files.insert(key, Arc::clone(&tracked));
                tracked
            }
        };
        if let Ok(mut held) = tracked.lock() {
            read_on(&mut held, path, &make);
        }
        Some(tracked)
    }
}

/// How many bytes before a file's offset are kept to tell an append from a
/// rewrite.
const SEAM: usize = 64;

/// Whether `path` still holds `seam` just before `offset`: lines were only
/// appended since. A tool writes its transcript only so; a file written
/// again whole (a move, a test) holds other bytes there.
fn continues(path: &Path, offset: u64, seam: &[u8]) -> bool {
    use std::io::{Read as _, Seek as _};
    if offset == 0 || seam.is_empty() {
        return true;
    }
    let mut held = vec![0_u8; seam.len()];
    std::fs::File::open(path).is_ok_and(|mut file| {
        file.seek(std::io::SeekFrom::Start(offset - seam.len() as u64))
            .and_then(|_| file.read_exact(&mut held))
            .is_ok()
            && held == seam
    })
}

/// Reads what was written to `path` since `tracked` last read it.
fn read_on(tracked: &mut Tracked, path: &Path, make: Make<'_>) {
    let compressed = juggler_adapters::transcript::is_compressed(path);
    if !compressed && !continues(path, tracked.offset, &tracked.seam) {
        // Written again, not appended to: read again from its start.
        if let Some(reader) = make() {
            tracked.file = ConversationFile::new(reader);
        }
        tracked.offset = 0;
        tracked.seam.clear();
        tracked.restarted = true;
    }
    let offset = tracked.offset;
    let mut seam = tracked.seam.clone();
    // A file read again from its start: into a new reader, kept only
    // when the read holds.
    let mut fresh: Option<ConversationFile> = None;
    let mut slices: Vec<(u64, UsageRead)> = Vec::new();
    let mut at = offset;
    let followed = follow(path, offset, |step| match step {
        Step::From(from) => {
            at = from;
            if from != offset || (compressed && offset != 0) {
                fresh = make().map(ConversationFile::new);
            }
        }
        Step::Lines(lines) => {
            at += lines.len() as u64;
            seam.extend_from_slice(&lines[lines.len().saturating_sub(SEAM)..]);
            seam.drain(..seam.len().saturating_sub(SEAM));
            let file = fresh.as_mut().unwrap_or(&mut tracked.file);
            let usage = file.read(lines);
            slices.push((if compressed { u64::MAX } else { at }, usage));
        }
    });
    match followed.void {
        None => {
            if let Some(fresh) = fresh {
                tracked.file = fresh;
                tracked.restarted = true;
            }
            tracked.usage.extend(slices);
        }
        // Not decodable yet: as it was, read again next time.
        Some(Void::NotReady) => return,
        Some(void) => {
            // What it handed on doesn't count; a reader that took part
            // of it starts over empty.
            if fresh.is_none()
                && let Some(reader) = make()
            {
                tracked.file = ConversationFile::new(reader);
            }
            let note = UsageRead {
                parse_errors: u64::from(void == Void::Damaged),
                unread: (void == Void::TooLarge)
                    .then_some("a compressed rollout too large to read"),
                ..UsageRead::default()
            };
            tracked.usage.push((u64::MAX, note));
        }
    }
    tracked.offset = followed.offset;
    // A line skipped for its length leaves nothing to check the next read
    // by.
    tracked.seam = if followed.skipped > 0 || compressed {
        Vec::new()
    } else {
        seam
    };
    if followed.skipped > 0 {
        tracked.usage.push((
            u64::MAX,
            UsageRead {
                parse_errors: followed.skipped,
                ..UsageRead::default()
            },
        ));
    }
}
