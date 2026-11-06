//! D-76 (AJ-E211): leaving out the start of a very large conversation
//! when switching — what every tool shares: where a cut may land (its
//! turns and compaction boundaries, found by each tool's own reader), the
//! plan for a choice, and a preview of it kept while the file is unchanged.
//! Each tool writes its own cut (`claude_cut`, `codex_cut`).

use std::path::Path;
use std::sync::Mutex;

pub use juggler_adapter::handover::Cut;

/// A place a cut may land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnMark {
    /// The line's number (0-based).
    pub line: usize,
    /// Its first byte.
    pub offset: u64,
    /// When it was written (epoch ms), when it says.
    pub at_ms: Option<i64>,
    /// A compaction boundary (else a person's turn).
    pub compaction: bool,
}

/// Where the cut lands, and what it leaves out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutPlan {
    /// The first kept line.
    pub from_line: usize,
    /// Its first byte.
    pub from_offset: u64,
    /// The person's turns left out.
    pub turns_left_out: usize,
    /// The person's turns kept.
    pub turns_kept: usize,
    /// The bytes left out (of the main transcript).
    pub bytes_left_out: u64,
    /// When the first kept line was written, when it says.
    pub from_ms: Option<i64>,
    /// It lands on a compaction boundary.
    pub at_compaction: bool,
}

/// What a cut wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutWritten {
    /// Lines written.
    pub lines: usize,
    /// Bytes written.
    pub bytes: u64,
    /// Lines whose parent was left out, now roots.
    pub rerooted: usize,
}

/// Where `cut` lands in a transcript of `total` bytes with `marks`;
/// `None` when it leaves nothing out (or would leave out every turn).
#[must_use]
pub fn plan(marks: &[TurnMark], total: u64, cut: Cut) -> Option<CutPlan> {
    let turns: Vec<&TurnMark> = marks.iter().filter(|m| !m.compaction).collect();
    let at = match cut {
        Cut::Before(ms) => turns
            .iter()
            .position(|m| m.at_ms.is_some_and(|at| at >= ms))?,
        Cut::KeepLast(n) => turns.len().checked_sub(n.max(1))?,
    };
    if at == 0 {
        return None;
    }
    let mut start = *turns[at];
    // A compaction boundary near before the cut: Claude's own fresh start,
    // with its summary of what came before.
    let near = total / 10;
    if let Some(boundary) = marks
        .iter()
        .rev()
        .find(|m| m.compaction && m.offset <= start.offset && start.offset - m.offset <= near)
    {
        start = *boundary;
    }
    let left_out = turns.iter().filter(|m| m.offset < start.offset).count();
    Some(CutPlan {
        from_line: start.line,
        from_offset: start.offset,
        turns_left_out: left_out,
        turns_kept: turns.len() - left_out,
        bytes_left_out: start.offset,
        from_ms: start.at_ms,
        at_compaction: start.compaction,
    })
}

/// The last transcript read: its path, length and change time, and its
/// marks (a choice's size is shown at once after the first read).
type Read1 = (
    std::path::PathBuf,
    u64,
    Option<std::time::SystemTime>,
    Vec<TurnMark>,
    u64,
);
static LAST: Mutex<Option<Read1>> = Mutex::new(None);

/// What `cut` would leave out of the transcript at `path` (its marks read
/// once, then kept while the file is unchanged).
///
/// # Errors
///
/// The read's IO error.
pub fn preview(
    path: &Path,
    cut: Cut,
    marks_of: fn(std::fs::File) -> std::io::Result<(Vec<TurnMark>, u64)>,
) -> std::io::Result<juggler_adapter::handover::CutPreview> {
    let meta = std::fs::metadata(path)?;
    let key = (path.to_path_buf(), meta.len(), meta.modified().ok());
    let kept = LAST.lock().ok().and_then(|last| {
        last.as_ref()
            .filter(|(p, n, t, ..)| (p, n, t) == (&key.0, &key.1, &key.2))
            .map(|(.., marks, total)| (marks.clone(), *total))
    });
    let (marks, total) = if let Some(read) = kept {
        read
    } else {
        let read = marks_of(std::fs::File::open(path)?)?;
        if let Ok(mut last) = LAST.lock() {
            *last = Some((key.0, key.1, key.2, read.0.clone(), read.1));
        }
        read
    };
    let turns: Vec<&TurnMark> = marks.iter().filter(|m| !m.compaction).collect();
    let planned = plan(&marks, total, cut);
    Ok(juggler_adapter::handover::CutPreview {
        turns: turns.len(),
        first_ms: turns.first().and_then(|m| m.at_ms),
        last_ms: turns.last().and_then(|m| m.at_ms),
        bytes: total,
        turns_left_out: planned.map_or(0, |p| p.turns_left_out),
        bytes_left_out: planned.map_or(0, |p| p.bytes_left_out),
        from_ms: planned.and_then(|p| p.from_ms),
        at_compaction: planned.is_some_and(|p| p.at_compaction),
    })
}

/// A tool's reader of marks.
pub type MarksOf = fn(std::fs::File) -> std::io::Result<(Vec<TurnMark>, u64)>;
/// A tool's writer of a cut.
pub type WriteCut = fn(&Path, &CutPlan, &Path) -> std::io::Result<CutWritten>;

/// Replaces `bundle`'s file `main_name` with its cut (in the bundle's
/// scratch folder), keeping the frozen whole source for the
/// copy-never-move check, and lowers the fidelity as a compaction does.
///
/// # Errors
///
/// [`juggler_adapter::error::AdapterError::Unsupported`] when the choice
/// leaves nothing out; the IO errors.
pub fn cut_bundle(
    bundle: &mut juggler_adapter::handover::SessionBundle,
    main_name: &str,
    cut: Cut,
    marks_of: MarksOf,
    write: WriteCut,
) -> Result<CutPlan, juggler_adapter::error::AdapterError> {
    use juggler_adapter::error::AdapterError;
    let scratch = if let Some(held) = &bundle.scratch {
        held.clone()
    } else {
        let made = std::sync::Arc::new(juggler_adapter::handover::BundleScratch::new()?);
        bundle.scratch = Some(made.clone());
        made
    };
    let index = bundle.files.len() + 1_000;
    let main = bundle
        .files
        .iter_mut()
        .find(|file| file.name == main_name)
        .ok_or_else(|| {
            AdapterError::Unsupported("the conversation's main file is missing".into())
        })?;
    // The frozen whole source: what the cut reads, and what the
    // copy-never-move check compares the source with.
    let whole = if let Some(path) = &main.on_disk {
        path.clone()
    } else {
        let path = scratch.entry(index);
        std::fs::write(&path, &main.bytes)?;
        path
    };
    let (marks, total) = marks_of(std::fs::File::open(&whole)?)?;
    let planned = plan(&marks, total, cut).ok_or_else(|| {
        AdapterError::Unsupported("that choice leaves nothing out of this conversation".into())
    })?;
    let cut_path = scratch.entry(index + 1);
    write(&whole, &planned, &cut_path)?;
    main.bytes = Vec::new();
    main.on_disk = Some(cut_path);
    main.cut_from = Some(whole);
    // As a compaction lowers it: the turns before the cut don't travel.
    let fidelity = &mut bundle.manifest.fidelity;
    "F2".clone_into(&mut fidelity.level);
    if !fidelity.caps_applied.iter().any(|c| c == "cut-start") {
        fidelity.caps_applied.insert(0, "cut-start".to_owned());
    }
    Ok(planned)
}
