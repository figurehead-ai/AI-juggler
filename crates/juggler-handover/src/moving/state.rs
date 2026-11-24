//! What a path was before a move (its baseline), read with the writers
//! stopped, and how to compare what is there now with it.

use std::path::{Path, PathBuf};

use juggler_platform::transfer::FileId;

/// One file's state.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileState {
    /// Its bytes.
    pub size: u64,
    /// The SHA-256 of all of them.
    pub sha256: String,
    /// What it is on disk.
    pub id: FileId,
    /// AJ-E2324: when it was last written (nanoseconds since the epoch),
    /// for the cold check by identity, size and this, without reading its
    /// bytes again. `None` in a move made before 0.5.1 (its check reads).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime_ns: Option<u64>,
}

/// A folder's state: every plain file below it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirState {
    /// The folder itself on disk.
    pub id: FileId,
    /// Every file, by its path below the folder.
    pub files: Vec<(PathBuf, FileState)>,
    /// Every folder below it, parents first.
    pub dirs: Vec<PathBuf>,
}

/// What a path was.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum State {
    /// Nothing was there.
    Absent,
    /// A plain file.
    File(FileState),
    /// A folder.
    Dir(DirState),
}

impl State {
    /// Whether something was there.
    #[must_use]
    pub fn present(&self) -> bool {
        !matches!(self, Self::Absent)
    }

    /// Its bytes in all.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        match self {
            Self::Absent => 0,
            Self::File(file) => file.size,
            Self::Dir(dir) => dir.files.iter().map(|(_, f)| f.size).sum(),
        }
    }

    /// What it is on disk, when something was there.
    #[must_use]
    pub fn id(&self) -> Option<FileId> {
        match self {
            Self::Absent => None,
            Self::File(file) => Some(file.id),
            Self::Dir(dir) => Some(dir.id),
        }
    }
}

/// Reads `path`'s state (hashing every file; links refused).
///
/// # Errors
///
/// It can't be read, or holds a link.
pub fn read(path: &Path) -> std::io::Result<State> {
    read_with(path, &mut |_| {})
}

/// [`read`], telling `tick` the bytes of each piece as it is read (a
/// long backup says how far it is, 0.4.9).
///
/// # Errors
///
/// It can't be read, or holds a link.
pub fn read_with(path: &Path, tick: &mut dyn FnMut(u64)) -> std::io::Result<State> {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Ok(State::Absent);
    };
    if meta.is_file() {
        return Ok(State::File(file_state_with(path, tick)?));
    }
    if meta.is_dir() {
        let (files, dirs) = juggler_platform::transfer::tree(path)?;
        let mut states = Vec::with_capacity(files.len());
        for rel in files {
            let state = file_state_with(&path.join(&rel), tick)?;
            states.push((rel, state));
        }
        return Ok(State::Dir(DirState {
            id: juggler_platform::transfer::file_id(path)?,
            files: states,
            dirs,
        }));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("{} is a link or a special file", path.display()),
    ))
}

/// One file's state.
///
/// # Errors
///
/// It can't be read.
pub fn file_state(path: &Path) -> std::io::Result<FileState> {
    file_state_with(path, &mut |_| {})
}

/// [`file_state`], telling `tick` each piece read.
fn file_state_with(path: &Path, tick: &mut dyn FnMut(u64)) -> std::io::Result<FileState> {
    let reader = Counted {
        inner: std::fs::File::open(path)?,
        tick,
    };
    let (sha256, size) = juggler_adapter::handover::sha256_reader(reader, None)?;
    Ok(FileState {
        size,
        sha256,
        id: juggler_platform::transfer::file_id(path)?,
        mtime_ns: mtime_ns(&std::fs::symlink_metadata(path)?),
    })
}

/// When `meta`'s file was last written, in nanoseconds since the epoch.
fn mtime_ns(meta: &std::fs::Metadata) -> Option<u64> {
    let since = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    u64::try_from(since.as_nanos()).ok()
}

/// AJ-E2324 (the review's C3): `source`'s state read from `clone`, its
/// clone made while nothing wrote it (the caller compares `source`'s
/// [`stamps`] before and after the clone): the bytes hashed are the
/// clone's (private, never written again), the identity and time of last
/// write are `source`'s. One read of every byte, not two.
///
/// # Errors
///
/// Either can't be read.
pub fn read_clone_with(
    source: &Path,
    clone: &Path,
    tick: &mut dyn FnMut(u64),
) -> std::io::Result<State> {
    let one = |rel: Option<&Path>, tick: &mut dyn FnMut(u64)| -> std::io::Result<FileState> {
        let (from, copy) = match rel {
            Some(rel) => (source.join(rel), clone.join(rel)),
            None => (source.to_path_buf(), clone.to_path_buf()),
        };
        let reader = Counted {
            inner: std::fs::File::open(&copy)?,
            tick,
        };
        let (sha256, size) = juggler_adapter::handover::sha256_reader(reader, None)?;
        Ok(FileState {
            size,
            sha256,
            id: juggler_platform::transfer::file_id(&from)?,
            mtime_ns: mtime_ns(&std::fs::symlink_metadata(&from)?),
        })
    };
    let meta = std::fs::symlink_metadata(source)?;
    if meta.is_file() {
        return Ok(State::File(one(None, tick)?));
    }
    if meta.is_dir() {
        let (files, dirs) = juggler_platform::transfer::tree(source)?;
        let mut states = Vec::with_capacity(files.len());
        for rel in files {
            let state = one(Some(&rel), tick)?;
            states.push((rel, state));
        }
        return Ok(State::Dir(DirState {
            id: juggler_platform::transfer::file_id(source)?,
            files: states,
            dirs,
        }));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("{} is a link or a special file", source.display()),
    ))
}

/// What a write changes, read without reading a byte: every file at or
/// under `path` (itself, when a file) by its identity, size and time of
/// last write. Equal stamps before and after a clone: nothing wrote it
/// meanwhile. Empty: nothing is there.
#[must_use]
pub fn stamps(path: &Path) -> Vec<(PathBuf, FileId, u64, Option<u64>)> {
    let stamp = |file: &Path| {
        let meta = std::fs::symlink_metadata(file).ok()?;
        let id = juggler_platform::transfer::file_id(file).ok()?;
        Some((file.to_path_buf(), id, meta.len(), mtime_ns(&meta)))
    };
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Vec::new();
    };
    if meta.is_dir() {
        let Ok((files, _)) = juggler_platform::transfer::tree(path) else {
            return Vec::new();
        };
        let mut out: Vec<_> = stamp(path).into_iter().collect();
        out.extend(files.iter().filter_map(|rel| stamp(&path.join(rel))));
        out
    } else {
        stamp(path).into_iter().collect()
    }
}

/// AJ-E2324: the cold check, without reading a byte: what is at `path`
/// now is `before`'s very files (identity), each with its size and time
/// of last write (a rename keeps all three; any write changes one). A
/// state recorded before 0.5.1 (no times) is checked by its bytes.
#[must_use]
pub fn unchanged(path: &Path, before: &State) -> bool {
    let same = |file: &Path, was: &FileState| {
        let when = was.mtime_ns?;
        let Ok(meta) = std::fs::symlink_metadata(file) else {
            return Some(false);
        };
        Some(
            meta.is_file()
                && meta.len() == was.size
                && mtime_ns(&meta) == Some(when)
                && juggler_platform::transfer::file_id(file).ok() == Some(was.id),
        )
    };
    let checked = match before {
        State::Absent => Some(std::fs::symlink_metadata(path).is_err()),
        State::File(was) => same(path, was),
        State::Dir(was) => {
            let now = juggler_platform::transfer::tree(path).ok();
            match now {
                Some((files, _)) if files.len() == was.files.len() => was
                    .files
                    .iter()
                    .map(|(rel, f)| same(&path.join(rel), f))
                    .try_fold(true, |all, one| one.map(|ok| all && ok)),
                _ => Some(false),
            }
        }
    };
    checked.unwrap_or_else(|| same_bytes(path, before))
}

/// A reader that tells `tick` how many bytes each read gave.
struct Counted<'a, R> {
    inner: R,
    tick: &'a mut dyn FnMut(u64),
}

impl<R: std::io::Read> std::io::Read for Counted<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        (self.tick)(read as u64);
        counted(read as u64);
        Ok(read)
    }
}

#[cfg(test)]
thread_local! {
    /// The bytes of moved files this test thread read (AJ-E2324: two full
    /// reads a move, not four).
    pub(super) static READ: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Counts `bytes` read, in tests.
fn counted(bytes: u64) {
    #[cfg(test)]
    READ.with(|read| read.set(read.get() + bytes));
    #[cfg(not(test))]
    let _ = bytes;
}

/// Whether what is at `path` now holds exactly `before`'s bytes (its
/// identity aside: a restored clone is another file with the same bytes).
#[must_use]
pub fn same_bytes(path: &Path, before: &State) -> bool {
    same_bytes_with(path, before, &mut |_| {})
}

/// [`same_bytes`], telling `tick` each piece read.
#[must_use]
pub fn same_bytes_with(path: &Path, before: &State, tick: &mut dyn FnMut(u64)) -> bool {
    match (before, read_with(path, tick)) {
        (State::Absent, Ok(State::Absent)) => true,
        (State::File(was), Ok(State::File(is))) => was.sha256 == is.sha256 && was.size == is.size,
        (State::Dir(was), Ok(State::Dir(is))) => {
            was.files.len() == is.files.len()
                && was
                    .files
                    .iter()
                    .zip(&is.files)
                    .all(|((a, fa), (b, fb))| a == b && fa.sha256 == fb.sha256)
        }
        _ => false,
    }
}

/// Whether the file at `path` still starts with exactly `before`'s bytes
/// (what an app may do to a history is add to its end). Returns the bytes
/// added after them, when it does.
#[must_use]
pub fn starts_with(path: &Path, before: &FileState) -> Option<Vec<u8>> {
    use std::io::{Read as _, Seek as _};
    let (sha, read) = juggler_adapter::handover::sha256_file(path, Some(before.size)).ok()?;
    counted(read);
    if read != before.size || sha != before.sha256 {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?;
    file.seek(std::io::SeekFrom::Start(before.size)).ok()?;
    let mut added = Vec::new();
    // The added part is a few lines; a megabyte more is read at most.
    file.take(1 << 20).read_to_end(&mut added).ok()?;
    Some(added)
}

/// What a writer changes, read without reading any bytes: every file at
/// or under each of `paths`, with its size and when it was last written
/// (0.4.9: a move waits for these to hold still before it backs them up).
pub type Marks = std::collections::BTreeMap<PathBuf, (u64, Option<std::time::SystemTime>)>;

/// The [`Marks`] of `paths` now (what can't be read is left out: a file
/// that comes or goes reads as a change).
#[must_use]
pub fn marks(paths: &[PathBuf]) -> Marks {
    let mut out = Marks::new();
    for path in paths {
        let Ok(meta) = std::fs::symlink_metadata(path) else {
            continue;
        };
        if meta.is_file() {
            out.insert(path.clone(), (meta.len(), meta.modified().ok()));
        } else if meta.is_dir()
            && let Ok((files, _)) = juggler_platform::transfer::tree(path)
        {
            for rel in files {
                let file = path.join(rel);
                if let Ok(meta) = std::fs::symlink_metadata(&file) {
                    out.insert(file, (meta.len(), meta.modified().ok()));
                }
            }
        }
    }
    out
}

/// The files whose marks differ between `before` and `now` (changed,
/// come or gone).
#[must_use]
pub fn changed(before: &Marks, now: &Marks) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = now
        .iter()
        .filter(|(path, mark)| before.get(*path) != Some(*mark))
        .map(|(path, _)| path.clone())
        .collect();
    out.extend(
        before
            .keys()
            .filter(|path| !now.contains_key(*path))
            .cloned(),
    );
    out
}
