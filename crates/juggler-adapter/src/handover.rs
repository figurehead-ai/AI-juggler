//! Handover support contract (AJ-E204; deep/09 §H1, H-1).
//!
//! An OPTIONAL sub-trait of [`crate::adapter::HarnessAdapter`] so the
//! semver-protected main trait is unchanged (ADR-0002) and adapters
//! opt in incrementally. A provider without `HandoverSupport` is
//! replay/pack-only (T3/T4). Alongside the trait live the pure
//! transcript-format helpers (sibling-set resolution, tail
//! verification, session-id forking, embedded-`cwd` reading) —
//! byte-level logic shared by every implementing adapter and unit
//! tested here, per the H-2/H-3 mandates: resolve via
//! `SessionRecord.transcript_ref` (never newest-mtime), copy the full
//! sibling set (never one file), and discover — never compute — the
//! target directory from `cwd`.

use std::collections::BTreeSet;
use std::io::{BufRead as _, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::adapter::HarnessAdapter;
use crate::error::AdapterError;
use crate::model::{Account, HarnessId};
use juggler_core::model::{Timestamp, TranscriptRef};

/// sha256 hex of `bytes` (the hash every manifest row and
/// copy-never-move invariant uses).
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex(&hasher.finalize())
}

/// sha256 hex of `path`'s first `limit` bytes (all of it with `None`),
/// streamed, and how many bytes it hashed. A transcript only grows at its
/// end, so "the source still starts with what was carried" is this over
/// the carried length (0.3.9: a conversation still open in its app keeps
/// writing while it is switched).
///
/// # Errors
///
/// The file can't be read.
pub fn sha256_file(path: &Path, limit: Option<u64>) -> std::io::Result<(String, u64)> {
    sha256_reader(std::fs::File::open(path)?, limit)
}

/// [`sha256_file`] of any reader (a decoded stream, say): its hash and
/// how many bytes it read, up to `limit`.
///
/// # Errors
///
/// The read fails.
pub fn sha256_reader(
    reader: impl std::io::Read,
    limit: Option<u64>,
) -> std::io::Result<(String, u64)> {
    use std::io::Read as _;
    let mut reader: Box<dyn std::io::Read> = match limit {
        Some(limit) => Box::new(reader.take(limit)),
        None => Box::new(reader),
    };
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut total = 0_u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok((hex(&hasher.finalize()), total))
}

fn hex(digest: &[u8]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Above this many bytes a file travels as a frozen copy on disk, never
/// in memory (0.3.9: a long Claude app conversation's transcript passed
/// 800 MB, and a cap that refused it made the switch fail every time).
pub const INLINE_MAX: u64 = 32 * 1024 * 1024;

/// How much of a large transcript's start is read for what its first
/// lines say (its `cwd`, a rollout's `session_meta`).
pub const HEAD_BYTES: usize = 8 * 1024 * 1024;

/// What a switch leaves free on any disk it writes to: it stops before it
/// would fill one (a full startup disk can stop the whole computer).
pub const DISK_RESERVE: u64 = 4_000_000_000;

/// [`DISK_RESERVE`] in words, as disks count it (decimal, as Finder).
pub const DISK_RESERVE_WORDS: &str = "4 GB";

/// Refuses a write of `bytes` into `dir` that would leave its disk with
/// less than [`DISK_RESERVE`] free.
///
/// # Errors
///
/// [`std::io::ErrorKind::StorageFull`], saying how much room it needs.
pub fn ensure_room(dir: &Path, bytes: u64) -> std::io::Result<()> {
    // A system that can't say (not yet on Windows): the guarded writer's
    // own checks are all there is.
    let Ok(free) = juggler_platform::room::free_space(dir) else {
        return Ok(());
    };
    let needed = bytes.saturating_add(DISK_RESERVE);
    if free >= needed {
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::StorageFull,
        format!(
            "not enough free space: needs {} MB more",
            (needed - free).div_ceil(1_000_000)
        ),
    ))
}

/// A writer that stops while [`DISK_RESERVE`] is still free on its disk,
/// checking as it goes (every 64 MB): something else may be filling the
/// disk at the same time.
pub struct RoomWriter<W> {
    inner: W,
    dir: PathBuf,
    since_check: u64,
}

impl<W: std::io::Write> RoomWriter<W> {
    /// `inner`, writing into the folder `dir`.
    pub fn new(inner: W, dir: &Path) -> Self {
        Self {
            inner,
            dir: dir.to_path_buf(),
            since_check: 0,
        }
    }
}

impl<W: std::io::Write> std::io::Write for RoomWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        const EVERY: u64 = 64 * 1024 * 1024;
        if self.since_check >= EVERY {
            ensure_room(&self.dir, EVERY)?;
            self.since_check = 0;
        }
        let wrote = self.inner.write(buf)?;
        self.since_check += wrote as u64;
        Ok(wrote)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Copies `from` to the new file `to`: a clone where the disk can (no
/// time, no room), else every byte through a [`RoomWriter`], after
/// checking there's room for it.
///
/// # Errors
///
/// No room, `to` exists, or the copy fails.
pub fn copy_file(from: &Path, to: &Path) -> std::io::Result<()> {
    let dir = to.parent().unwrap_or_else(|| Path::new("."));
    if juggler_platform::room::copy_is_a_clone(from, dir).unwrap_or(false) {
        std::fs::copy(from, to)?;
        return Ok(());
    }
    let mut source = std::fs::File::open(from)?;
    ensure_room(dir, source.metadata()?.len())?;
    let mut out = RoomWriter::new(
        std::io::BufWriter::with_capacity(
            1 << 20,
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(to)?,
        ),
        dir,
    );
    std::io::copy(&mut source, &mut out)?;
    out.flush()
}

/// One file inside a [`SessionBundle`]: its bytes in memory, or (a large
/// one) a frozen copy on disk taken when the bundle was made.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BundleFile {
    /// File name (not a deep path — the sibling set is flat).
    pub name: String,
    /// The file's bytes; empty when it is [`Self::on_disk`].
    pub bytes: Vec<u8>,
    /// A large file's frozen copy (a clone in the bundle's scratch
    /// folder, made at bundle time so a source still being written can't
    /// change under the switch).
    pub on_disk: Option<PathBuf>,
    /// D-76: this is a cut of the source (its start left out, on the
    /// user's request); here is the frozen whole source, which the
    /// copy-never-move check reads instead.
    pub cut_from: Option<PathBuf>,
}

impl BundleFile {
    /// A file that travels in memory.
    #[must_use]
    pub fn inline(name: String, bytes: Vec<u8>) -> Self {
        Self {
            name,
            bytes,
            on_disk: None,
            cut_from: None,
        }
    }

    /// Its size in bytes.
    ///
    /// # Errors
    ///
    /// The frozen copy can't be read.
    pub fn len(&self) -> std::io::Result<u64> {
        match &self.on_disk {
            Some(path) => Ok(std::fs::metadata(path)?.len()),
            None => Ok(self.bytes.len() as u64),
        }
    }

    /// Whether it holds nothing.
    ///
    /// # Errors
    ///
    /// The frozen copy can't be read.
    pub fn is_empty(&self) -> std::io::Result<bool> {
        Ok(self.len()? == 0)
    }

    /// sha256 hex of its content, streamed for a large one.
    ///
    /// # Errors
    ///
    /// The frozen copy can't be read.
    pub fn sha256(&self) -> std::io::Result<String> {
        match &self.on_disk {
            Some(path) => sha256_file(path, None).map(|(hash, _)| hash),
            None => Ok(sha256_hex(&self.bytes)),
        }
    }

    /// Its first `limit` bytes (what the embedded `cwd` is read from).
    ///
    /// # Errors
    ///
    /// The frozen copy can't be read.
    pub fn head(&self, limit: usize) -> std::io::Result<Vec<u8>> {
        let Some(path) = &self.on_disk else {
            return Ok(self.bytes[..self.bytes.len().min(limit)].to_vec());
        };
        let mut out = Vec::with_capacity(limit);
        std::fs::File::open(path)?
            .take(limit as u64)
            .read_to_end(&mut out)?;
        Ok(out)
    }

    /// Its last `limit` bytes (what the torn-tail check reads).
    ///
    /// # Errors
    ///
    /// The frozen copy can't be read.
    pub fn tail(&self, limit: usize) -> std::io::Result<Vec<u8>> {
        let Some(path) = &self.on_disk else {
            return Ok(self.bytes[self.bytes.len().saturating_sub(limit)..].to_vec());
        };
        let mut file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        file.seek(SeekFrom::Start(len.saturating_sub(limit as u64)))?;
        let mut out = Vec::with_capacity(limit);
        file.read_to_end(&mut out)?;
        Ok(out)
    }

    /// Its whole content in memory: for a caller that must hold it (a
    /// bundle sent to another Mac); `max` refuses one larger than that.
    ///
    /// # Errors
    ///
    /// Larger than `max`, or the frozen copy can't be read.
    pub fn load(&self, max: u64) -> std::io::Result<Vec<u8>> {
        let Some(path) = &self.on_disk else {
            return Ok(self.bytes.clone());
        };
        if std::fs::metadata(path)?.len() > max {
            return Err(std::io::Error::other("too large to send in one piece"));
        }
        std::fs::read(path)
    }

    /// Whether its last non-empty line is whole JSON (F-H-02: a source
    /// still writing, or torn), read from its end: a window that grows
    /// until it holds that line, so a very long last line (a big tool
    /// output) is read whole, never cut.
    ///
    /// # Errors
    ///
    /// The frozen copy can't be read.
    pub fn last_line_whole(&self) -> std::io::Result<bool> {
        let len = usize::try_from(self.len()?).unwrap_or(usize::MAX);
        let mut window = 1024 * 1024;
        loop {
            let tail = self.tail(window)?;
            let end = tail
                .iter()
                .rposition(|byte| !byte.is_ascii_whitespace())
                .map_or(0, |at| at + 1);
            let body = &tail[..end];
            let start = body.iter().rposition(|byte| *byte == b'\n');
            if start.is_some() || window >= len {
                return Ok(tail_parses(&body[start.map_or(0, |at| at + 1)..]));
            }
            window = window.saturating_mul(2);
        }
    }

    /// Its lines that hold any of `needles`, one after another (what a
    /// marker check parses instead of the whole file; streamed for a
    /// large one).
    ///
    /// # Errors
    ///
    /// The frozen copy can't be read.
    pub fn lines_with(&self, needles: &[&str]) -> std::io::Result<Vec<u8>> {
        let finders: Vec<_> = needles
            .iter()
            .map(|needle| memchr::memmem::Finder::new(needle.as_bytes()))
            .collect();
        let mut out = Vec::new();
        self.each_line(|line| {
            if finders.iter().any(|finder| finder.find(line).is_some()) {
                out.extend_from_slice(line);
                if !line.ends_with(b"\n") {
                    out.push(b'\n');
                }
            }
            Ok(())
        })?;
        Ok(out)
    }

    /// Writes it to `to` a line at a time, each line through `rewrite`
    /// (a forked session id; streamed for a large one). `to` must not
    /// exist yet.
    ///
    /// # Errors
    ///
    /// `to` exists, or a read or write fails.
    pub fn rewrite_lines_to(
        &self,
        to: &Path,
        mut rewrite: impl FnMut(&[u8]) -> Vec<u8>,
    ) -> std::io::Result<()> {
        // A rewrite writes every byte again: room for it first.
        let dir = to.parent().unwrap_or_else(|| Path::new("."));
        ensure_room(dir, self.len()?)?;
        let mut writer = RoomWriter::new(
            std::io::BufWriter::with_capacity(
                1 << 20,
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(to)?,
            ),
            dir,
        );
        self.each_line(|line| writer.write_all(&rewrite(line)))?;
        writer.flush()
    }

    /// Each line, its newline kept (the last one may have none).
    fn each_line(&self, mut each: impl FnMut(&[u8]) -> std::io::Result<()>) -> std::io::Result<()> {
        let Some(path) = &self.on_disk else {
            for line in self.bytes.split_inclusive(|byte| *byte == b'\n') {
                each(line)?;
            }
            return Ok(());
        };
        let mut reader = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path)?);
        let mut line = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                return Ok(());
            }
            each(&line)?;
        }
    }

    /// Writes its content to `to` (a clone where the file system can, so
    /// even a very large one takes no time or extra space).
    ///
    /// # Errors
    ///
    /// The copy or the write fails.
    pub fn write_to(&self, to: &Path) -> std::io::Result<()> {
        match &self.on_disk {
            Some(path) => copy_file(path, to),
            None => std::fs::write(to, &self.bytes),
        }
    }
}

/// A bundle's scratch folder for its large files' frozen copies, removed
/// with the last bundle that holds it.
#[derive(Debug)]
pub struct BundleScratch(PathBuf);

impl BundleScratch {
    /// A new, empty, owner-only scratch folder under the system's temp
    /// folder (on the same disk as the user's files, so copies clone).
    ///
    /// # Errors
    ///
    /// It can't be made.
    pub fn new() -> std::io::Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ai-juggler-bundle-{}-{}-{}",
            std::process::id(),
            Timestamp::now().millis(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        juggler_platform::private_fs::create_private_dir(&dir)?;
        Ok(Self(dir))
    }

    /// Freezes `source` as entry `index` (a clone where it can).
    ///
    /// # Errors
    ///
    /// The copy fails.
    pub fn freeze(&self, source: &Path, index: usize) -> std::io::Result<PathBuf> {
        let to = self.entry(index);
        copy_file(source, &to)?;
        Ok(to)
    }

    /// Where entry `index` goes (for a copy made another way: a
    /// decompressed one).
    #[must_use]
    pub fn entry(&self, index: usize) -> PathBuf {
        self.0.join(index.to_string())
    }

    /// The folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// Carries a bundle's files: each read into memory while the bundle's
/// total stays within [`INLINE_MAX`], the rest frozen on disk (0.3.9: a
/// conversation's folder of 1,719 small files held 460 MB at once).
#[derive(Debug, Default)]
pub struct Carrier {
    scratch: Option<std::sync::Arc<BundleScratch>>,
    in_memory: u64,
}

impl Carrier {
    /// Nothing carried yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Carries `source` as `name`, entry `index` of the bundle.
    ///
    /// # Errors
    ///
    /// The read, the scratch folder or the copy fails.
    pub fn carry(
        &mut self,
        name: String,
        source: &Path,
        index: usize,
    ) -> std::io::Result<BundleFile> {
        if self.fits(std::fs::metadata(source)?.len()) {
            return Ok(BundleFile::inline(name, std::fs::read(source)?));
        }
        let frozen = self.held()?.freeze(source, index)?;
        Ok(BundleFile {
            name,
            bytes: Vec::new(),
            on_disk: Some(frozen),
            cut_from: None,
        })
    }

    /// Whether `len` more bytes fit in memory (counted when they do).
    pub fn fits(&mut self, len: u64) -> bool {
        let total = self.in_memory.saturating_add(len);
        if total > INLINE_MAX {
            return false;
        }
        self.in_memory = total;
        true
    }

    /// Where entry `index` goes on disk (for a copy made another way: a
    /// decompressed one), the scratch folder made on first need.
    ///
    /// # Errors
    ///
    /// The folder can't be made.
    pub fn entry(&mut self, index: usize) -> std::io::Result<PathBuf> {
        Ok(self.held()?.entry(index))
    }

    /// The scratch folder the bundle keeps, if any file needed it.
    #[must_use]
    pub fn into_scratch(self) -> Option<std::sync::Arc<BundleScratch>> {
        self.scratch
    }

    fn held(&mut self) -> std::io::Result<std::sync::Arc<BundleScratch>> {
        if let Some(held) = &self.scratch {
            return Ok(held.clone());
        }
        let made = std::sync::Arc::new(BundleScratch::new()?);
        self.scratch = Some(made.clone());
        Ok(made)
    }
}

impl Drop for BundleScratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl PartialEq for BundleScratch {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for BundleScratch {}

/// The bundle manifest: identity + fidelity evidence for what was
/// copied (deep/09 §H1 shape).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BundleManifest {
    /// Manifest schema (1).
    pub schema: u32,
    /// The harness this bundle belongs to.
    pub harness: HarnessId,
    /// The vendor-native session id.
    pub native_id: String,
    /// The working directory the session ran in (read from the
    /// transcript's embedded `cwd` — the discovery key).
    pub cwd: PathBuf,
    /// Harness version observed at bundle time, when known.
    pub harness_version: Option<String>,
    /// When the bundle was taken.
    pub created_at: Timestamp,
    /// The fidelity honestly attributable to this bundle.
    pub fidelity: Fidelity,
    /// Whether any redaction was applied (pack policy, AJ-E208).
    pub redacted: bool,
}

/// Fidelity evidence carried on a bundle.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Fidelity {
    /// The F-label (`F0`–`F4`).
    pub level: String,
    /// Caps applied (e.g. `unvalidated-pair`, `target-dir-fallback`).
    pub caps_applied: Vec<String>,
}

/// A full session bundle: the main transcript plus every sibling
/// file (subagent transcripts, spill, file-history inventory) — a
/// single-file bundle is not constructible through the honest path.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SessionBundle {
    /// Identity + fidelity.
    pub manifest: BundleManifest,
    /// The `<session>*` sibling set (main file included).
    pub files: Vec<BundleFile>,
    /// Where its large files' frozen copies are kept, while it lives.
    pub scratch: Option<std::sync::Arc<BundleScratch>>,
}

/// The prime-prompt input: distilled turns (T3 replay).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TurnText {
    /// Message uuid, when the source carried one.
    pub uuid: Option<String>,
    /// Who spoke (`user` / `assistant`).
    pub role: &'static str,
    /// The distilled text.
    pub text: String,
}

impl TurnText {
    /// This turn as it goes into a replay brief: a `[role]:` line, then
    /// the text quoted line by line with `> `, so nothing inside a turn
    /// can pose as another speaker's turn. Every line break a model may
    /// read as one (`\r`, NEL, the Unicode line and paragraph
    /// separators) starts a new quoted line.
    #[must_use]
    pub fn quoted(&self) -> String {
        let mut out = format!("[{}]:\n", self.role);
        let text = self
            .text
            .replace("\r\n", "\n")
            .replace(['\r', '\u{85}', '\u{2028}', '\u{2029}'], "\n");
        for line in text.split('\n') {
            out.push_str("> ");
            out.push_str(line);
            out.push('\n');
        }
        out
    }
}

/// The install result: where the session landed plus the honesty
/// flags the manifest/UI must carry (H-3's flagged fallback, F-H-03's
/// fork notice).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InstallOutcome {
    /// The installed transcript's reference.
    pub reference: TranscriptRef,
    /// The native id forked (collision on target).
    pub forked: bool,
    /// The target dir came from the flagged mangle fallback, NOT
    /// discovery — surfaced, never hidden.
    pub target_dir_fallback: bool,
    /// ADR-0028: where each file of the bundle was placed, by its name
    /// in the bundle, so the switch checks each where it truly is (a
    /// family's members can live in other folders, with other ids).
    /// Empty: beside the transcript, as the tool's own layout has it.
    pub placed: Vec<(String, PathBuf)>,
}

/// D-76 (AJ-E211): what a switch leaves out of a very large
/// conversation, when the user asks (the default carries everything).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cut {
    /// The turns before this time (epoch ms).
    Before(i64),
    /// All but the last this many turns.
    KeepLast(usize),
}

/// What a cut would leave out and carry, before the switch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CutPreview {
    /// The person's turns in the conversation.
    pub turns: usize,
    /// When its first and last turns were written (epoch ms).
    pub first_ms: Option<i64>,
    /// When its last turn was written.
    pub last_ms: Option<i64>,
    /// The conversation's size (its main transcript).
    pub bytes: u64,
    /// With the cut: the turns left out (0: nothing would be).
    pub turns_left_out: usize,
    /// The bytes left out.
    pub bytes_left_out: u64,
    /// When the first kept line was written.
    pub from_ms: Option<i64>,
    /// It lands on the tool's own compaction (its summary of before).
    pub at_compaction: bool,
}

/// Optional handover capability (T2 transplant + T3 prime). The
/// engine orchestrates; the adapter performs harness-specific I/O.
pub trait HandoverSupport: HarnessAdapter {
    /// Bundles the session at `reference` — the FULL sibling set,
    /// copy-never-move (the source stays byte-identical).
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] semantics for a missing file;
    /// other [`AdapterError`] variants for read failures or a torn
    /// tail (F-H-02).
    fn bundle_session(&self, reference: &TranscriptRef) -> Result<SessionBundle, AdapterError>;

    /// Installs a bundle under the target account, DISCOVERING the
    /// target project directory by embedded `cwd` (H-3; the naive
    /// mangle is a flagged fallback, never the primary). Returns the
    /// installed transcript's reference (forked id on collision,
    /// F-H-03).
    ///
    /// # Errors
    ///
    /// [`AdapterError::AccountDir`] for path/write failures.
    fn install_bundle(
        &self,
        bundle: &SessionBundle,
        target: &Account,
    ) -> Result<InstallOutcome, AdapterError>;

    /// Builds the prime prompt for T3 replay from distilled turns.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] when the harness has no prime
    /// path.
    fn prime_prompt(&self, turns: &[TurnText]) -> Result<String, AdapterError>;

    /// Where a file of the bundle [`HandoverSupport::bundle_session`]
    /// made for `reference` lives at the source, so the switch can check
    /// it was copied, never moved or changed. Default: beside the
    /// transcript. A tool that keeps some of a session's files elsewhere
    /// (in the [`FILE_HISTORY_AREA`]) says where.
    #[must_use]
    fn source_file(&self, reference: &TranscriptRef, name: &str) -> PathBuf {
        reference
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(name)
    }

    /// Whether the source of the bundle file `name` still starts with
    /// exactly the `bytes` bytes hashing to `sha256` that were carried
    /// (copy, never move; a source only grows). Default: the file at
    /// [`Self::source_file`], read that far. A tool whose source is
    /// stored otherwise (compressed, elsewhere) checks it its own way.
    #[must_use]
    fn source_unchanged(
        &self,
        reference: &TranscriptRef,
        name: &str,
        bytes: u64,
        sha256: &str,
    ) -> bool {
        matches!(
            sha256_file(&self.source_file(reference, name), Some(bytes)),
            Ok((hash, read)) if read == bytes && hash == sha256
        )
    }

    /// What a switch of `reference` to `target` will write, before it
    /// starts (0.3.9: the switch screen says how much disk space it uses).
    /// Reads sizes only. Default: the transcript alone, copied as
    /// [`room_of`] would.
    ///
    /// # Errors
    ///
    /// The transcript can't be read.
    fn switch_room(
        &self,
        reference: &TranscriptRef,
        target: &Account,
    ) -> Result<SwitchRoom, AdapterError> {
        Ok(room_of(
            &[(reference.path.clone(), false)],
            &target.state_dir,
        )?)
    }

    /// ADR-0028: the conversations the tool itself records as started
    /// from `reference`'s (and theirs), by id: the family its own bundle
    /// carries (Codex's subagent threads). A family kept by a desktop app
    /// is the host's ([`crate::live::HostFamily`]). Default: none.
    #[must_use]
    fn own_family(&self, reference: &TranscriptRef) -> Vec<String> {
        let _ = reference;
        Vec::new()
    }

    /// 0.4: the pages on the tool's own website the conversation links to
    /// (published or opened in it), by id, deduplicated. They belong to
    /// the account that made them, so on another account they may not
    /// open; the switch says so before anything moves. Reads the
    /// transcript as a stream. Default: this tool has none.
    ///
    /// # Errors
    ///
    /// The transcript can't be read.
    fn account_pages(&self, reference: &TranscriptRef) -> Result<Vec<String>, AdapterError> {
        let _ = reference;
        Ok(Vec::new())
    }

    /// D-76: what `cut` would leave out of `reference`'s conversation
    /// (reads it once; a tool caches what it read). Default: this tool
    /// can't leave part of a conversation out.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] where it can't; the read's error.
    fn cut_preview(&self, reference: &TranscriptRef, cut: Cut) -> Result<CutPreview, AdapterError> {
        let _ = (reference, cut);
        Err(AdapterError::Unsupported(
            "this tool's conversations travel whole".into(),
        ))
    }

    /// D-76: [`HandoverSupport::bundle_session`] with `cut`'s turns left
    /// out, at a turn the target resumes from; its fidelity lowered as a
    /// compaction lowers it. Default: unsupported.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] where it can't, or when `cut`
    /// leaves nothing out; as `bundle_session` otherwise.
    fn bundle_session_cut(
        &self,
        reference: &TranscriptRef,
        cut: Cut,
    ) -> Result<SessionBundle, AdapterError> {
        let _ = (reference, cut);
        Err(AdapterError::Unsupported(
            "this tool's conversations travel whole".into(),
        ))
    }
}

/// What a switch will write, worked out before it starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwitchRoom {
    /// The conversation's size: every file it carries.
    pub carried: u64,
    /// What it writes in full on disk (a clone writes nothing).
    pub writes: u64,
    /// Part of it unpacks as it's copied, to a size not known before:
    /// `writes` counts it at its packed size.
    pub unpacks: bool,
}

/// What carrying `files` to `target_dir` writes, as a bundle carries them
/// ([`Carrier`]): small ones read into memory and written (up to
/// [`INLINE_MAX`] in all), large ones cloned where the disks allow; a
/// file marked `true` is rewritten (a fork) and written in full.
///
/// # Errors
///
/// A file's size can't be read.
pub fn room_of(files: &[(PathBuf, bool)], target_dir: &Path) -> std::io::Result<SwitchRoom> {
    let scratch = std::env::temp_dir();
    let mut room = SwitchRoom::default();
    let mut carrier = Carrier::new();
    let existing = target_dir
        .ancestors()
        .find(|dir| dir.is_dir())
        .unwrap_or(target_dir);
    for (path, rewritten) in files {
        let len = std::fs::metadata(path)?.len();
        room.carried += len;
        let cloned = || {
            juggler_platform::room::copy_is_a_clone(path, &scratch).unwrap_or(false)
                && juggler_platform::room::copy_is_a_clone(path, existing).unwrap_or(false)
        };
        if *rewritten || carrier.fits(len) || !cloned() {
            room.writes += len;
        }
    }
    Ok(room)
}

/// AJ-E1540: the leading folder of a bundle file name that belongs in the
/// account's own `file-history/` rather than beside the transcript (a
/// name no tool gives a file: `~` never starts a session id).
pub const FILE_HISTORY_AREA: &str = "~file-history";

/// The live lab of 2026-10-28: the leading folder of a bundle file name
/// that belongs in the account's own `tasks/` (a session's task list,
/// kept outside its transcript).
pub const TASKS_AREA: &str = "~tasks";

/// Every reserved area of a bundle, with the account folder it stands for.
pub const ACCOUNT_AREAS: &[(&str, &str)] =
    &[(FILE_HISTORY_AREA, "file-history"), (TASKS_AREA, "tasks")];

/// Where the bundle file `name` lives in the account folder `account`,
/// when it is in one of the [`ACCOUNT_AREAS`]; `None` for a file that goes
/// beside the transcript.
#[must_use]
pub fn account_area_path(account: &Path, name: &str) -> Option<PathBuf> {
    let (area, rest) = name.split_once('/')?;
    ACCOUNT_AREAS
        .iter()
        .find(|(reserved, _)| *reserved == area)
        .map(|(_, folder)| account.join(folder).join(rest))
}

/// How deep a session's folder is walked (`<id>/subagents/…`).
const SESSION_TREE_DEPTH: usize = 6;

/// AJ-E1540: every file of one session in `dir`, as (bundle name, path):
/// the flat `<stem>*` files beside it (the transcript, older-style
/// subagent files) and everything under its `<stem>/` folder (current
/// Claude Code keeps `subagents/` and `tool-results/` there), named by
/// their path below `dir` with `/`. Links are skipped; sorted.
#[must_use]
pub fn session_files(dir: &Path, stem: &str) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    for path in sibling_paths(dir, stem) {
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if meta.is_file() {
            out.push((name, path));
        } else if meta.is_dir() && name == stem {
            tree_files(&path, &name, SESSION_TREE_DEPTH, &mut out);
        }
    }
    out.sort();
    out
}

/// The files below `dir`, named `prefix/…`, at most `depth` folders down.
pub fn tree_files(dir: &Path, prefix: &str, depth: usize, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        if meta.is_file() {
            out.push((name, path));
        } else if meta.is_dir() && depth > 0 {
            tree_files(&path, &name, depth - 1, out);
        }
    }
}

/// AJ-E1540: whether a bundle file name is a safe path to install: `/`
/// separated plain components (no `.`, `..`, hidden or empty ones, no `\`
/// or NUL), at most eight deep, a reserved area ([`ACCOUNT_AREAS`]) only
/// first. A bundle can come from a paired Mac, so its names are untrusted.
#[must_use]
pub fn is_safe_bundle_name(name: &str) -> bool {
    let parts: Vec<&str> = name.split('/').collect();
    !name.is_empty()
        && name.len() <= 1024
        && parts.len() <= 8
        && parts.iter().enumerate().all(|(index, part)| {
            (index == 0 && ACCOUNT_AREAS.iter().any(|(area, _)| area == part) && parts.len() > 1)
                || (!part.is_empty()
                    && part.len() <= 255
                    && !part.starts_with('.')
                    && !part.starts_with('~')
                    && !part.contains(['\\', '\0'])
                    && Path::new(part).components().count() == 1)
        })
}

/// Lists the `<stem>*` sibling set in `dir` (the main file plus
/// subagent transcripts, spill, file-history inventory) — F-H-04's
/// honest inventory. Sorted for determinism.
#[must_use]
pub fn sibling_paths(dir: &Path, stem: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(stem))
        })
        .collect();
    out.sort();
    out
}

/// Whether the last non-empty line parses as JSON — the F-H-02 torn
/// tail check (a source still appending produces a partial line).
#[must_use]
pub fn tail_parses(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
}

/// Reads the embedded `"cwd"` from the first lines of a Claude JSONL, or
/// the `payload.cwd` of a Codex rollout (the discovery key — decoding the
/// mangled directory name is ambiguous on dots/dashes, H-3).
#[must_use]
pub fn embedded_cwd(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    for line in text.lines().take(25) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(cwd) = value.get("cwd").and_then(serde_json::Value::as_str) {
            return Some(cwd.to_owned());
        }
        // A Codex rollout keeps it in the envelope's payload
        // (`session_meta` / `turn_context`, AJ-E1304).
        if let Some(cwd) = value
            .get("payload")
            .and_then(|payload| payload.get("cwd"))
            .and_then(serde_json::Value::as_str)
        {
            return Some(cwd.to_owned());
        }
    }
    None
}

/// Rewrites the top-level `sessionId` on every line to `new_id`
/// (F-H-03 collision fork): a NEW native id, the `uuid`/`parentUuid`
/// tree preserved line-by-line. Lines that fail to parse pass
/// through byte-identical (never dropped).
#[must_use]
pub fn fork_session_id(bytes: &[u8], new_id: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.trim().is_empty() {
            out.push('\n');
            continue;
        }
        if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "sessionId".to_owned(),
                    serde_json::Value::String(new_id.to_owned()),
                );
            }
            out.push_str(&value.to_string());
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.into_bytes()
}

/// Rewrites every top-level `"cwd"` (a Codex line's `payload.cwd`) that
/// is `from` (or inside it) to
/// the same place under `to` (AJ-E706: the user-confirmed path mapping
/// for a cross-machine transplant — never applied without that
/// confirmation). Lines that fail to parse, and paths outside `from`,
/// pass through byte-identical.
#[must_use]
pub fn remap_cwd(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    let from = from.trim_end_matches('/');
    let to = to.trim_end_matches('/');
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let remapped = serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .and_then(|mut value| {
                // Claude: top-level `cwd`; Codex: the envelope's
                // `payload.cwd` (AJ-E1304).
                let holder = if value.get("cwd").is_some() {
                    value.as_object_mut()?
                } else {
                    value.get_mut("payload")?.as_object_mut()?
                };
                let cwd = holder.get("cwd")?.as_str()?.to_owned();
                let rest = if cwd == from {
                    ""
                } else if cwd.starts_with(&format!("{from}/")) {
                    &cwd[from.len()..]
                } else {
                    return None;
                };
                holder.insert(
                    "cwd".to_owned(),
                    serde_json::Value::String(format!("{to}{rest}")),
                );
                Some(value.to_string())
            });
        out.push_str(remapped.as_deref().unwrap_or(line));
        out.push('\n');
    }
    out.into_bytes()
}

/// The set of message `uuid`s in a transcript (the first-wins dedupe
/// key — H2.1's cycle-safety).
#[must_use]
pub fn uuid_set(bytes: &[u8]) -> BTreeSet<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(uuid) = value.get("uuid").and_then(serde_json::Value::as_str) {
            out.insert(uuid.to_owned());
        }
    }
    out
}

/// Claude Code's own name for a project's folder: every character that
/// isn't an ASCII letter or digit becomes `-` (`/Users/me/My App/.x` →
/// `-Users-me-My-App--x`, as on disk). Used only when discovery finds no
/// target folder (H-3). 0.3.2: it replaced only `/` and `\`, so a project
/// whose path held a space or a dot got a folder `--resume` never looked
/// in, and the switched session started fresh (the maintainer's walk).
///
/// Always one plain folder name: dots become dashes, so it can never
/// point at the config root or above it.
#[must_use]
pub fn mangled_dir(cwd: &str) -> String {
    let name: String = cwd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    if name.is_empty() {
        "-".to_owned()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_is_its_files_and_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let id = "abc-123";
        std::fs::write(dir.path().join(format!("{id}.jsonl")), "{}\n").unwrap();
        std::fs::write(dir.path().join(format!("{id}-agent-1.jsonl")), "{}\n").unwrap();
        std::fs::create_dir_all(dir.path().join(id).join("subagents")).unwrap();
        std::fs::write(
            dir.path().join(id).join("subagents").join("a.jsonl"),
            "{}\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join(id).join("tool-results")).unwrap();
        std::fs::write(
            dir.path().join(id).join("tool-results").join("r.txt"),
            "out",
        )
        .unwrap();
        std::fs::write(dir.path().join("other.jsonl"), "{}\n").unwrap();
        let names: Vec<String> = session_files(dir.path(), id)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(
            names,
            vec![
                format!("{id}-agent-1.jsonl"),
                format!("{id}.jsonl"),
                format!("{id}/subagents/a.jsonl"),
                format!("{id}/tool-results/r.txt"),
            ]
        );
    }

    #[test]
    fn bundle_names_never_climb_or_hide() {
        assert!(is_safe_bundle_name("abc.jsonl"));
        assert!(is_safe_bundle_name("abc/subagents/a.jsonl"));
        assert!(is_safe_bundle_name("~file-history/abc/f1@v2"));
        assert!(is_safe_bundle_name("~tasks/abc/1.json"));
        assert!(!is_safe_bundle_name("~other/abc/1.json"));
        assert_eq!(
            account_area_path(Path::new("/acct"), "~tasks/abc/1.json"),
            Some(PathBuf::from("/acct/tasks/abc/1.json"))
        );
        assert_eq!(account_area_path(Path::new("/acct"), "abc.jsonl"), None);
        for bad in [
            "",
            "../x",
            "abc/../x",
            "/abs",
            "abc//x",
            ".hidden",
            "abc/.git/config",
            "~file-history",
            "abc/~file-history/x",
            "~other/x",
            "a\\b",
            "a/b/c/d/e/f/g/h/i",
        ] {
            assert!(!is_safe_bundle_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_fallback_name_never_climbs_out() {
        assert_eq!(mangled_dir("/Users/dev/app"), "-Users-dev-app");
        assert_eq!(mangled_dir(".."), "--");
        assert_eq!(mangled_dir("."), "-");
        assert_eq!(mangled_dir(""), "-");
        assert_eq!(mangled_dir("..\\..\\x"), "------x");
    }

    fn line(uuid: &str, parent: Option<&str>, session: &str, cwd: &str) -> String {
        serde_json::json!({
            "uuid": uuid,
            "parentUuid": parent,
            "sessionId": session,
            "cwd": cwd,
            "type": "user",
            "message": {"role": "user", "content": "hi"},
        })
        .to_string()
    }

    #[test]
    fn remap_cwd_moves_only_paths_inside_the_mapped_root() {
        let body = format!(
            "{}\n{}\n{}\nnot json\n",
            line("u1", None, "s1", "/Users/a/Dev/shop"),
            line("u2", Some("u1"), "s1", "/Users/a/Dev/shop/web"),
            line("u3", Some("u2"), "s1", "/Users/a/Dev/shopping"),
        );
        let out = remap_cwd(body.as_bytes(), "/Users/a/Dev/shop", "/Volumes/work/shop/");
        let text = String::from_utf8(out).unwrap();
        let cwds: Vec<String> = text
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .map(|v| v["cwd"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            cwds,
            [
                "/Volumes/work/shop",
                "/Volumes/work/shop/web",
                "/Users/a/Dev/shopping"
            ],
            "a sibling folder sharing the prefix is never moved"
        );
        assert!(
            text.ends_with("not json\n"),
            "unparseable lines pass through"
        );
        assert_eq!(uuid_set(text.as_bytes()).len(), 3, "the tree is kept");
    }

    #[test]
    fn tail_parses_rejects_a_torn_line() {
        let whole = format!("{}\n", line("u1", None, "s1", "/w"));
        assert!(tail_parses(whole.as_bytes()));
        let torn = format!("{whole}{{\"uuid\": \"u2\", \"pare");
        assert!(!tail_parses(torn.as_bytes()));
    }

    #[test]
    fn embedded_cwd_reads_from_the_first_lines() {
        let bytes = format!("{}\n", line("u1", None, "s1", "/Users/dev/weird dir")).into_bytes();
        assert_eq!(
            embedded_cwd(&bytes),
            Some("/Users/dev/weird dir".to_owned())
        );
        assert_eq!(embedded_cwd(b"not json\n"), None);
    }

    #[test]
    fn fork_preserves_the_tree_and_rewrites_the_id() {
        let body = format!(
            "{}\n{}\n",
            line("u1", None, "s-old", "/w"),
            line("u2", Some("u1"), "s-old", "/w")
        );
        let forked = fork_session_id(body.as_bytes(), "s-new");
        let forked_text = String::from_utf8_lossy(&forked).into_owned();
        assert!(forked_text.contains("\"sessionId\":\"s-new\""));
        assert!(!forked_text.contains("s-old"));
        assert!(
            forked_text.contains("\"parentUuid\":\"u1\""),
            "tree preserved"
        );
        // Unparseable lines pass through byte-identical.
        let mixed = format!("{body}{{torn");
        let forked_mixed = fork_session_id(mixed.as_bytes(), "s-new");
        assert!(String::from_utf8_lossy(&forked_mixed).contains("{torn"));
    }

    #[test]
    fn uuid_set_collects_ids() {
        let body = format!(
            "{}\n{}\n",
            line("u1", None, "s", "/w"),
            line("u2", Some("u1"), "s", "/w")
        );
        let set = uuid_set(body.as_bytes());
        assert!(set.contains("u1") && set.contains("u2"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn mangle_is_the_flagged_fallback_only() {
        assert_eq!(mangled_dir("/a/b.c"), "-a-b-c");
        // As Claude Code names them on disk (0.3.2): spaces and dots too.
        assert_eq!(
            mangled_dir("/Users/me/Library/Application Support/AI Juggler/x"),
            "-Users-me-Library-Application-Support-AI-Juggler-x"
        );
        assert_eq!(
            mangled_dir("/Users/me/Dev/App/.claude/worktrees/rel-0.3"),
            "-Users-me-Dev-App--claude-worktrees-rel-0-3"
        );
        assert_eq!(mangled_dir(".."), "--");
        assert_eq!(mangled_dir(""), "-");
        // Fallback status is structural: discovery is the primary
        // path and flags its use — this helper never decides alone.
    }
}
