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
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Ok(State::Absent);
    };
    if meta.is_file() {
        return Ok(State::File(file_state(path)?));
    }
    if meta.is_dir() {
        let (files, dirs) = juggler_platform::transfer::tree(path)?;
        let mut states = Vec::with_capacity(files.len());
        for rel in files {
            let state = file_state(&path.join(&rel))?;
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
    let (sha256, size) = juggler_adapter::handover::sha256_file(path, None)?;
    Ok(FileState {
        size,
        sha256,
        id: juggler_platform::transfer::file_id(path)?,
    })
}

/// Whether what is at `path` now holds exactly `before`'s bytes (its
/// identity aside: a restored clone is another file with the same bytes).
#[must_use]
pub fn same_bytes(path: &Path, before: &State) -> bool {
    match (before, read(path)) {
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
