//! ADR-0031 (D-93): the transaction engine of a switch by move, the same
//! for every tool. The adapter plans
//! ([`juggler_adapter::moving::InPlaceMove`]); this engine applies the
//! plan so that nothing can be lost:
//!
//! 1. **Baseline, with the writers stopped.** Every path a step touches is
//!    read (bytes hashed, identity on disk), and everything there is
//!    cloned into the move's folder and the clones checked against it.
//!    Nothing changes until every backup checks.
//! 2. **A journal.** Each step writes its intent before it acts and its
//!    completion after, synced, so a crash leaves a state that
//!    [`Transaction::recover`] reads back exactly: done, not done, or (never
//!    guessed) needing a person.
//! 3. **Steps that can't lose.** Renames stay on one disk and never replace
//!    a file; what a step would replace is put aside in the move's holding
//!    place first; records are written whole and atomically.
//! 4. **Checks before the apps start again** ([`Transaction::verify_cold`]):
//!    every moved file is the same file with the same bytes, and the
//!    adapter's own checks of its records pass. **And after**
//!    ([`Transaction::verify_opened`]): a history the app added to still
//!    starts with every byte it had, and what was added is the app's
//!    bookkeeping, never a turn the move wrote.
//! 5. **Undo** ([`Transaction::undo`]): every backup checked first; what is
//!    there now (work done since included) is put aside, never deleted;
//!    then the baseline is restored from the clones and checked.
//!
//! The process side (which apps are busy, quitting the two exact profiles,
//! starting them again, the resume link to the exact process) is the
//! caller's: this engine only touches files, and refuses to start without
//! the caller's word that the writers are stopped ([`Stopped`]).

pub mod state;

use std::io::Write as _;
use std::path::{Path, PathBuf};

use juggler_adapter::moving::{Appended, MoveCheck, MovePlan, MoveRequest, MoveStep};
use juggler_platform::transfer;

use state::State;

/// What `manifest.json` calls its layout.
pub const FORMAT: &str = "ai-juggler-move";
/// This layout's version.
pub const VERSION: u32 = 1;
/// Where moves live in AI Juggler's data folder.
pub const FOLDER: &str = "moves";
/// The manifest's name in a move's folder.
const MANIFEST: &str = "manifest.json";
/// The journal's name.
const JOURNAL: &str = "journal.jsonl";
/// The room a move keeps free beyond its own writes.
const ROOM_MARGIN: u64 = 256 * 1024 * 1024;

/// A move's error, in words a user can read.
#[derive(Debug, thiserror::Error)]
pub enum MoveError {
    /// The move can't start or go on, and nothing it would touch changed.
    #[error("{0}")]
    Refused(String),
    /// A step failed; the journal says how far it got.
    #[error("{what}: {source}")]
    Io {
        /// What was being done.
        what: String,
        /// The system's error.
        source: std::io::Error,
    },
}

fn io(what: impl Into<String>) -> impl FnOnce(std::io::Error) -> MoveError {
    let what = what.into();
    move |source| MoveError::Io { what, source }
}

/// Where a move is.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Planned; nothing read or written yet.
    Planned,
    /// The baseline is read and every backup checked.
    BackedUp,
    /// Steps are being applied (the journal says which).
    Applying,
    /// Every step applied.
    Applied,
    /// The checks before the apps start again passed.
    Verified,
    /// The apps started again and the checks after passed: done.
    Committed,
    /// Being undone (the journal says how far).
    Undoing,
    /// Undone: the baseline is back, what was there kept aside.
    Undone,
}

/// One path the move touches, as it was.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    /// The path.
    pub path: PathBuf,
    /// What was there (read with the writers stopped).
    pub before: State,
    /// Its clone in the move's `backups/`, when something was there.
    pub backup: Option<PathBuf>,
    /// The plan made the folder (nothing was there, nothing of its own is
    /// kept): Undo removes it only when it is empty.
    pub made: bool,
}

/// The move's record of itself, rewritten whole at every phase.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    /// [`FORMAT`].
    pub format: String,
    /// [`VERSION`].
    pub version: u32,
    /// The move's id (its folder's name).
    pub id: String,
    /// The tool's id.
    pub tool: String,
    /// When it was planned (epoch ms).
    pub created_ms: i64,
    /// The request the adapter planned.
    pub request: MoveRequest,
    /// The plan (made again after the apps quit).
    pub plan: MovePlan,
    /// Where it is.
    pub phase: Phase,
    /// Every path a step touches, as it was.
    pub entries: Vec<Entry>,
    /// The move this one moves back or on (a switch again), when any.
    pub previous: Option<String>,
    /// Why it stopped, when it did.
    pub problem: Option<String>,
    /// The caller's own names for what it stopped and starts again (app
    /// profiles), so Undo and recovery stop the same ones.
    #[serde(default)]
    pub places: Vec<String>,
    /// The caller's ids of the accounts on both sides (their sessions must
    /// be quiet before the move or its Undo touches their folders).
    #[serde(default)]
    pub accounts: Vec<String>,
    /// Where Undo keeps what was there (its folder in `preserved/`).
    #[serde(default)]
    pub preserved: Option<String>,
}

/// The caller's word that the writers of both sides are stopped and stay
/// stopped while the value lives (the two app profiles quit, checked by
/// process and start time; AI Juggler's own sessions of both accounts
/// stopped). Made only by the caller that stopped them.
pub struct Stopped {
    _private: (),
}

impl Stopped {
    /// The writers are stopped: the caller checked it.
    #[must_use]
    pub fn checked() -> Self {
        Self { _private: () }
    }
}

/// A lock on the folders a move touches, held while the move runs, shared
/// by every AI Juggler on this Mac (the Dev build and the installed one
/// keep different data folders, so the lock is in the user's own temporary
/// folder, named by the folders it guards).
pub struct Lock {
    _files: Vec<std::fs::File>,
}

impl Lock {
    /// Takes the lock on every one of `roots`, or none.
    ///
    /// # Errors
    ///
    /// Another move holds one of them.
    pub fn take(roots: &[PathBuf]) -> Result<Self, MoveError> {
        let mut files = Vec::new();
        let mut roots = roots.to_vec();
        roots.sort();
        roots.dedup();
        for root in roots {
            let digest =
                juggler_adapter::handover::sha256_reader(root.to_string_lossy().as_bytes(), None)
                    .map_err(io("name the lock"))?
                    .0;
            let path = std::env::temp_dir().join(format!("ai-juggler-move-{}.lock", &digest[..16]));
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&path)
                .map_err(io("open the lock"))?;
            if file.try_lock().is_err() {
                return Err(MoveError::Refused(
                    "another move of these accounts is running; wait for it to finish".into(),
                ));
            }
            files.push(file);
        }
        Ok(Self { _files: files })
    }
}

/// What the checks found.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Report {
    /// Every check, in words.
    pub checks: Vec<MoveCheck>,
    /// Lines the app added to moved histories after it started, by kind.
    pub added: AddedLines,
}

impl Report {
    /// Whether every check holds.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.pass)
    }

    /// The checks that failed, in words.
    #[must_use]
    pub fn failures(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter(|c| !c.pass)
            .map(|c| c.what.clone())
            .collect()
    }

    fn check(&mut self, what: impl Into<String>, pass: bool) {
        self.checks.push(MoveCheck {
            what: what.into(),
            pass,
        });
    }
}

/// What the app added to moved histories after it started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AddedLines {
    /// Its own bookkeeping.
    pub bookkeeping: usize,
    /// Notices it wrote into a history only (not turns).
    pub notices: usize,
    /// Turns (someone typed, or the model answered).
    pub turns: usize,
    /// Lines of a shape the adapter doesn't know.
    pub unknown: usize,
}

/// What [`Transaction::recover`] found of an interrupted move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// Nothing to recover: the move is planned, done or undone.
    Settled(Phase),
    /// Interrupted while applying: steps before `next` are done; it can go
    /// on from `next` or be undone.
    Applying {
        /// The first step not done.
        next: usize,
    },
    /// Interrupted while being undone: undo again (it is safe to repeat).
    Undoing,
    /// A step's state can't be told from the disk: a person decides (the
    /// journal and both sides are kept as they are).
    Unclear(String),
}

/// A move.
pub struct Transaction {
    dir: PathBuf,
    /// Its manifest.
    pub manifest: Manifest,
}

/// Checks a plan against the engine's rules before anything starts: every
/// step inside the plan's roots, the move's own folder on the same disk
/// as everything it puts aside or backs up, clones there, and room.
/// Returns the refusals, in words (empty: it may start).
#[must_use]
pub fn preflight(plan: &MovePlan, moves_dir: &Path) -> Vec<String> {
    let mut out = plan.refusals.clone();
    if plan.steps.is_empty() && out.is_empty() {
        out.push("there is nothing to move".into());
    }
    for step in &plan.steps {
        for path in step.paths() {
            if !juggler_adapter::moving::contained(path, &plan.roots) {
                out.push(format!(
                    "a step would touch {}, outside both accounts' folders",
                    path.display()
                ));
            }
        }
    }
    let device = transfer::device_of(moves_dir).ok();
    for step in &plan.steps {
        for path in step.paths() {
            if transfer::device_of(path).ok() != device {
                out.push(
                    "AI Juggler's data folder is on another disk than the accounts' folders, so it can't keep clone backups of them"
                        .into(),
                );
                break;
            }
        }
    }
    if !transfer::clones_supported(moves_dir).unwrap_or(false) {
        out.push(
            "the disk can't make clone backups (it needs a file system that makes clones)".into(),
        );
    }
    let existing = moves_dir
        .ancestors()
        .find(|dir| dir.is_dir())
        .unwrap_or(moves_dir);
    let room = juggler_platform::room::free_space(existing).unwrap_or(0);
    let writes: u64 = plan
        .steps
        .iter()
        .map(|step| match step {
            MoveStep::WriteRecord { value, .. } => value.to_string().len() as u64,
            _ => 0,
        })
        .sum();
    if room < writes + ROOM_MARGIN {
        out.push("the disk is nearly full; free some room first".into());
    }
    out.sort();
    out.dedup();
    out
}

impl Transaction {
    /// Starts a move of `plan` in a new folder under `moves_dir`. Nothing
    /// outside that folder is touched.
    ///
    /// # Errors
    ///
    /// [`preflight`]'s refusals; the folder can't be written.
    pub fn begin(
        moves_dir: &Path,
        id: &str,
        tool: &str,
        request: MoveRequest,
        plan: MovePlan,
        previous: Option<String>,
        now_ms: i64,
    ) -> Result<Self, MoveError> {
        let refusals = preflight(&plan, moves_dir);
        if !refusals.is_empty() {
            return Err(MoveError::Refused(refusals.join("; ")));
        }
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(MoveError::Refused(
                "a move's id is letters, digits and dashes".into(),
            ));
        }
        std::fs::create_dir_all(moves_dir).map_err(io("make the moves folder"))?;
        let dir = moves_dir.join(id);
        juggler_platform::private_fs::create_private_dir(&dir)
            .map_err(io("make the move's folder"))?;
        for sub in ["backups", "holding", "preserved"] {
            std::fs::create_dir(dir.join(sub)).map_err(io("make the move's folder"))?;
        }
        let manifest = Manifest {
            format: FORMAT.into(),
            version: VERSION,
            id: id.into(),
            tool: tool.into(),
            created_ms: now_ms,
            request,
            plan,
            phase: Phase::Planned,
            entries: Vec::new(),
            previous,
            problem: None,
            places: Vec::new(),
            accounts: Vec::new(),
            preserved: None,
        };
        let txn = Self { dir, manifest };
        txn.save()?;
        txn.log(&serde_json::json!({"event": "phase", "phase": Phase::Planned}))?;
        Ok(txn)
    }

    /// Opens the move in `dir`.
    ///
    /// # Errors
    ///
    /// No manifest there, or one of another format.
    pub fn open(dir: &Path) -> Result<Self, MoveError> {
        let bytes = std::fs::read(dir.join(MANIFEST)).map_err(io("read the move's manifest"))?;
        let manifest: Manifest = serde_json::from_slice(&bytes)
            .map_err(|e| MoveError::Refused(format!("the move's manifest can't be read: {e}")))?;
        if manifest.format != FORMAT || manifest.version > VERSION {
            return Err(MoveError::Refused(
                "the move was made by a newer AI Juggler; open it there".into(),
            ));
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            manifest,
        })
    }

    /// Every move under `moves_dir`, oldest first.
    #[must_use]
    pub fn all(moves_dir: &Path) -> Vec<Self> {
        let mut out: Vec<Self> = std::fs::read_dir(moves_dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|entry| Self::open(&entry.path()).ok())
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by_key(|txn| txn.manifest.created_ms);
        out
    }

    /// The move's folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn save(&self) -> Result<(), MoveError> {
        let bytes = serde_json::to_vec_pretty(&self.manifest)
            .map_err(|e| MoveError::Refused(format!("the manifest can't be written: {e}")))?;
        transfer::write_atomic(&self.dir.join(MANIFEST), &bytes).map_err(io("write the manifest"))
    }

    fn log(&self, line: &serde_json::Value) -> Result<(), MoveError> {
        let mut file = juggler_platform::private_fs::append_private(&self.dir.join(JOURNAL))
            .map_err(io("open the journal"))?;
        let mut text = line.to_string();
        text.push('\n');
        file.write_all(text.as_bytes())
            .map_err(io("write the journal"))?;
        file.sync_all().map_err(io("sync the journal"))
    }

    fn set_phase(&mut self, phase: Phase) -> Result<(), MoveError> {
        self.manifest.phase = phase;
        self.save()?;
        self.log(&serde_json::json!({"event": "phase", "phase": phase}))
    }

    /// Remembers the caller's names for what it stops and starts again.
    ///
    /// # Errors
    ///
    /// The manifest can't be written.
    pub fn remember_places(
        &mut self,
        places: Vec<String>,
        accounts: Vec<String>,
    ) -> Result<(), MoveError> {
        self.manifest.places = places;
        self.manifest.accounts = accounts;
        self.save()
    }

    /// Whether this move and `other` touch any path in common (one inside
    /// the other counts).
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        let mine: Vec<&Path> = self.touched();
        other.touched().iter().any(|theirs| {
            mine.iter()
                .any(|path| path.starts_with(theirs) || theirs.starts_with(path))
        })
    }

    /// Every path the plan's steps touch.
    #[must_use]
    pub fn touched(&self) -> Vec<&Path> {
        self.manifest
            .plan
            .steps
            .iter()
            .flat_map(MoveStep::paths)
            .collect()
    }

    /// Records why the move stopped.
    ///
    /// # Errors
    ///
    /// The manifest can't be written.
    pub fn stop_with(&mut self, problem: &str) -> Result<(), MoveError> {
        self.manifest.problem = Some(problem.to_owned());
        self.save()
    }

    /// Puts in a newer plan of the same shape (made again after the apps
    /// quit: they write their records when they quit). A plan of another
    /// shape refuses: the family or its files changed, so the user sees
    /// the move again before it happens.
    ///
    /// # Errors
    ///
    /// The shape changed, or the move is past planning.
    pub fn replan(&mut self, plan: MovePlan) -> Result<(), MoveError> {
        if self.manifest.phase != Phase::Planned {
            return Err(MoveError::Refused("the move is past planning".into()));
        }
        if plan.shape() != self.manifest.plan.shape() || !plan.can_move() {
            return Err(MoveError::Refused(
                "the conversation's family or files changed while the apps quit; nothing was moved, check the move again".into(),
            ));
        }
        self.manifest.plan = plan;
        self.save()
    }

    /// Reads the baseline of every path the plan touches and clones what
    /// is there into `backups/`, checking every clone against it. Nothing
    /// outside the move's folder changes.
    ///
    /// # Errors
    ///
    /// A path can't be read, a clone fails or doesn't match.
    pub fn back_up(&mut self, _stopped: &Stopped) -> Result<(), MoveError> {
        if self.manifest.phase != Phase::Planned {
            return Err(MoveError::Refused("the move is backed up already".into()));
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut entries = Vec::new();
        for step in &self.manifest.plan.steps {
            let made = matches!(step, MoveStep::MakeDir { .. });
            let touched: Vec<&Path> = match step {
                MoveStep::Clone { to, .. } => vec![to],
                other => other.paths(),
            };
            for path in touched {
                if !seen.insert(path.to_path_buf()) {
                    continue;
                }
                let before = state::read(path).map_err(io(format!("read {}", path.display())))?;
                entries.push(Entry {
                    path: path.to_path_buf(),
                    made: made && !before.present(),
                    before,
                    backup: None,
                });
            }
        }
        for (index, entry) in entries.iter_mut().enumerate() {
            if !entry.before.present() {
                continue;
            }
            let name = PathBuf::from(format!("{index:05}"));
            let to = self.dir.join("backups").join(&name);
            match &entry.before {
                State::File(_) => transfer::clone_file(&entry.path, &to),
                State::Dir(_) => transfer::clone_tree(&entry.path, &to),
                State::Absent => Ok(()),
            }
            .map_err(io(format!("back up {}", entry.path.display())))?;
            if !state::same_bytes(&to, &entry.before) {
                return Err(MoveError::Refused(format!(
                    "the backup of {} doesn't match it, so nothing was moved",
                    entry.path.display()
                )));
            }
            entry.backup = Some(name);
        }
        self.manifest.entries = entries;
        self.set_phase(Phase::BackedUp)
    }

    fn before(&self, path: &Path) -> Option<&State> {
        self.manifest
            .entries
            .iter()
            .find(|e| e.path == path)
            .map(|e| &e.before)
    }

    fn holding(&self, index: usize) -> PathBuf {
        self.dir.join("holding").join(format!("{index:05}"))
    }

    /// Applies the plan's steps from step `from` on (0 for a new move; a
    /// recovered move goes on where it stopped), journaling each.
    ///
    /// # Errors
    ///
    /// A step's precondition doesn't hold, or the step fails: the move
    /// stops there, and the journal says so.
    pub fn apply(&mut self, _stopped: &Stopped, from: usize) -> Result<(), MoveError> {
        match self.manifest.phase {
            Phase::BackedUp if from == 0 => {}
            Phase::Applying => {}
            _ => {
                return Err(MoveError::Refused(
                    "the move isn't ready to apply (it ran already, or isn't backed up)".into(),
                ));
            }
        }
        self.set_phase(Phase::Applying)?;
        let steps = self.manifest.plan.steps.clone();
        for (index, step) in steps.iter().enumerate().skip(from) {
            self.log(&serde_json::json!({"event": "intent", "step": index}))?;
            self.step(index, step)?;
            self.log(&serde_json::json!({"event": "done", "step": index}))?;
        }
        self.set_phase(Phase::Applied)
    }

    fn step(&self, index: usize, step: &MoveStep) -> Result<(), MoveError> {
        let fail = |what: &str, path: &Path| format!("{what} {}", path.display());
        match step {
            MoveStep::MakeDir { path } => {
                std::fs::create_dir(path).map_err(io(fail("make", path)))?;
                if let Some(parent) = path.parent() {
                    transfer::sync_dir(parent).map_err(io(fail("sync", parent)))?;
                }
            }
            MoveStep::Rename { from, to, .. } => {
                let expected = self.before(from).and_then(State::id);
                if transfer::file_id(from).ok() != expected {
                    return Err(MoveError::Refused(format!(
                        "{} changed since the backup, so the move stopped there",
                        from.display()
                    )));
                }
                transfer::rename_on_device(from, to).map_err(io(fail("move", from)))?;
                for dir in [from.parent(), to.parent()].into_iter().flatten() {
                    transfer::sync_dir(dir).map_err(io(fail("sync", dir)))?;
                }
            }
            MoveStep::Hold { path, .. } => {
                let expected = self.before(path).and_then(State::id);
                if expected.is_none() || transfer::file_id(path).ok() != expected {
                    return Err(MoveError::Refused(format!(
                        "{} changed since the backup, so the move stopped there",
                        path.display()
                    )));
                }
                let held = self.holding(index);
                transfer::rename_on_device(path, &held).map_err(io(fail("put aside", path)))?;
                if let Some(parent) = path.parent() {
                    transfer::sync_dir(parent).map_err(io(fail("sync", parent)))?;
                }
            }
            MoveStep::WriteRecord { path, value } => {
                // What must be there now: nothing, when an earlier step put
                // it aside; else exactly what the backup has.
                let held_before = self.manifest.plan.steps[..index]
                    .iter()
                    .any(|s| matches!(s, MoveStep::Hold { path: held, .. } if held == path));
                let expected = if held_before {
                    State::Absent
                } else {
                    self.before(path).cloned().unwrap_or(State::Absent)
                };
                if !state::same_bytes(path, &expected) {
                    return Err(MoveError::Refused(format!(
                        "{} changed since the backup, so the move stopped there",
                        path.display()
                    )));
                }
                let bytes = serde_json::to_vec(value)
                    .map_err(|e| MoveError::Refused(format!("a record can't be written: {e}")))?;
                transfer::write_atomic(path, &bytes).map_err(io(fail("write", path)))?;
            }
            MoveStep::Clone { from, to } => {
                let meta = std::fs::symlink_metadata(from).map_err(io(fail("read", from)))?;
                if meta.is_dir() {
                    transfer::clone_tree(from, to)
                } else {
                    transfer::clone_file(from, to)
                }
                .map_err(io(fail("clone", from)))?;
            }
        }
        Ok(())
    }

    /// The checks before the apps start again: every moved file is the
    /// same file with the same bytes where the plan put it, every put-aside
    /// thing is in the holding place, every record holds what the plan
    /// wrote, and `adapter`'s own checks pass. A passing report moves the
    /// phase to [`Phase::Verified`].
    ///
    /// # Errors
    ///
    /// The manifest can't be written.
    pub fn verify_cold(&mut self, adapter: Vec<MoveCheck>) -> Result<Report, MoveError> {
        let mut report = Report::default();
        for (index, step) in self.manifest.plan.steps.iter().enumerate() {
            match step {
                MoveStep::MakeDir { path } => {
                    report.check(format!("{} is a folder", path.display()), path.is_dir());
                }
                MoveStep::Rename { from, to, .. } => {
                    let before = self.before(from).cloned().unwrap_or(State::Absent);
                    let same_file = transfer::file_id(to).ok() == before.id();
                    report.check(
                        format!("{} arrived as the same file", to.display()),
                        same_file,
                    );
                    report.check(
                        format!("{} has every byte it had", to.display()),
                        state::same_bytes(to, &before),
                    );
                    report.check(
                        format!("{} left its old place", from.display()),
                        std::fs::symlink_metadata(from).is_err(),
                    );
                }
                MoveStep::Hold { path, .. } => {
                    let before = self.before(path).cloned().unwrap_or(State::Absent);
                    report.check(
                        format!("{} is kept aside", path.display()),
                        transfer::file_id(&self.holding(index)).ok() == before.id(),
                    );
                }
                MoveStep::WriteRecord { path, value } => {
                    let written = std::fs::read(path)
                        .ok()
                        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
                    report.check(
                        format!("{} holds the moved record", path.display()),
                        written.as_ref() == Some(value),
                    );
                }
                MoveStep::Clone { from, to } => {
                    let source = state::read(from).unwrap_or(State::Absent);
                    report.check(
                        format!("{} is a whole copy", to.display()),
                        source.present() && state::same_bytes(to, &source),
                    );
                }
            }
        }
        report.checks.extend(adapter);
        if report.passed() {
            self.set_phase(Phase::Verified)?;
        }
        Ok(report)
    }

    /// The checks after the apps started again: every moved history still
    /// starts with every byte it had (the app may add to its end), every
    /// other moved file is unchanged up to its length, and what was added
    /// is sorted by `appended`. A turn added is reported, never taken as
    /// the move's (the move writes none). `adapter`'s checks (its records
    /// again: an app rewrites them when it starts) are added. A passing
    /// report commits the move.
    ///
    /// # Errors
    ///
    /// The manifest can't be written.
    pub fn verify_opened(
        &mut self,
        appended: &dyn Fn(&[u8]) -> Appended,
        adapter: Vec<MoveCheck>,
    ) -> Result<Report, MoveError> {
        let mut report = Report::default();
        for step in &self.manifest.plan.steps {
            let MoveStep::Rename { from, to, history } = step else {
                continue;
            };
            let before = self.before(from).cloned().unwrap_or(State::Absent);
            let files: Vec<(PathBuf, state::FileState)> = match before {
                State::File(file) => vec![(to.clone(), file)],
                State::Dir(dir) => dir
                    .files
                    .into_iter()
                    .map(|(rel, f)| (to.join(rel), f))
                    .collect(),
                State::Absent => Vec::new(),
            };
            for (path, file) in files {
                let added = state::starts_with(&path, &file);
                report.check(
                    format!("{} still starts with every byte it had", path.display()),
                    added.is_some(),
                );
                if *history && let Some(added) = added {
                    for line in added.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
                        match appended(line) {
                            Appended::Bookkeeping => report.added.bookkeeping += 1,
                            Appended::SystemNotice => report.added.notices += 1,
                            Appended::Turn => report.added.turns += 1,
                            Appended::Unknown => report.added.unknown += 1,
                        }
                    }
                }
            }
        }
        report.checks.extend(adapter);
        if report.passed() && self.manifest.phase == Phase::Verified {
            self.set_phase(Phase::Committed)?;
        }
        Ok(report)
    }

    /// Reads back a move that stopped part way (AI Juggler quit or the
    /// Mac stopped): what the journal and the disk say together. Reads
    /// only: it changes nothing, so listing moves is always safe.
    ///
    /// # Errors
    ///
    /// The journal can't be read.
    pub fn recover(&self) -> Result<Recovery, MoveError> {
        match self.manifest.phase {
            Phase::Applying => {}
            Phase::Undoing => return Ok(Recovery::Undoing),
            phase => return Ok(Recovery::Settled(phase)),
        }
        let text =
            std::fs::read_to_string(self.dir.join(JOURNAL)).map_err(io("read the journal"))?;
        let mut done = std::collections::BTreeSet::new();
        let mut intent: Option<usize> = None;
        for line in text.lines() {
            // A torn last line is a write that never finished: ignored.
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let step = value
                .get("step")
                .and_then(serde_json::Value::as_u64)
                .and_then(|s| usize::try_from(s).ok());
            match (value.get("event").and_then(serde_json::Value::as_str), step) {
                (Some("intent"), Some(step)) => intent = Some(step),
                (Some("done"), Some(step)) => {
                    done.insert(step);
                    intent = None;
                }
                _ => {}
            }
        }
        let next = (0..self.manifest.plan.steps.len())
            .find(|step| !done.contains(step))
            .unwrap_or(self.manifest.plan.steps.len());
        let Some(open) = intent.filter(|step| !done.contains(step)) else {
            return Ok(Recovery::Applying { next });
        };
        // A step that started and may not have finished: the disk says.
        let step = &self.manifest.plan.steps[open];
        let happened = match step {
            MoveStep::MakeDir { path } => Some(path.is_dir()),
            MoveStep::Rename { from, to, .. } => self.renamed(from, to),
            MoveStep::Hold { path, .. } => self.renamed(path, &self.holding(open)),
            MoveStep::WriteRecord { path, value } => {
                let now = std::fs::read(path)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
                // The write is atomic (a whole new file renamed over it):
                // it happened, or it didn't.
                Some(now.as_ref() == Some(value))
            }
            // A clone that started may be whole or half made: nothing
            // there means it didn't happen; anything there is left as it
            // is for a person or Undo (which keeps it aside), never removed
            // by reading.
            MoveStep::Clone { to, .. } => std::fs::symlink_metadata(to).is_err().then_some(false),
        };
        match happened {
            Some(true) => Ok(Recovery::Applying { next: open + 1 }),
            Some(false) => Ok(Recovery::Applying { next: open }),
            None => Ok(Recovery::Unclear(format!(
                "step {} ({}) can't be told from the disk; both sides and the journal are kept as they are",
                open + 1,
                step.paths()
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(" → ")
            ))),
        }
    }

    /// Whether the rename of `from` (as it was backed up) to `to`
    /// happened: the same file at `to` and nothing at `from`, or the
    /// other way round; `None` when the disk shows neither.
    fn renamed(&self, from: &Path, to: &Path) -> Option<bool> {
        let id = self.before(from).and_then(State::id);
        match (transfer::file_id(from).ok(), transfer::file_id(to).ok()) {
            (None, Some(at)) if Some(at) == id => Some(true),
            (Some(at), None) if Some(at) == id => Some(false),
            _ => None,
        }
    }

    /// Undoes the move: every backup checked first (one that doesn't match
    /// stops it before anything changes); what is at each touched path now
    /// (work done since included) put aside in `preserved/<at>/`, never
    /// deleted; the baseline restored from the clones; then checked. Safe
    /// to repeat after an interruption. Returns where the work done since
    /// was kept.
    ///
    /// # Errors
    ///
    /// A backup doesn't match (nothing changed), or a restore fails (the
    /// journal says how far it got; undo again).
    pub fn undo(&mut self, _stopped: &Stopped, now_ms: i64) -> Result<PathBuf, MoveError> {
        if self.manifest.phase < Phase::BackedUp {
            self.set_phase(Phase::Undone)?;
            return Ok(self.dir.join("preserved"));
        }
        if self.manifest.phase == Phase::Undone {
            return Err(MoveError::Refused("the move is undone already".into()));
        }
        for entry in &self.manifest.entries {
            if let Some(backup) = &entry.backup
                && !state::same_bytes(&self.dir.join("backups").join(backup), &entry.before)
            {
                return Err(MoveError::Refused(format!(
                    "the backup of {} doesn't match it any more, so nothing was undone",
                    entry.path.display()
                )));
            }
        }
        self.set_phase(Phase::Undoing)?;
        // One preserved folder per undo, kept across an interrupted one.
        let name = self
            .manifest
            .preserved
            .clone()
            .unwrap_or_else(|| now_ms.to_string());
        if self.manifest.preserved.is_none() {
            self.manifest.preserved = Some(name.clone());
            self.save()?;
        }
        let kept = self.dir.join("preserved").join(name);
        std::fs::create_dir_all(&kept).map_err(io("make the preserved folder"))?;
        // Put aside what is there now, deepest first; a folder the move
        // made stays when anything else is in it (new conversations of
        // the project), and is removed when it's empty.
        let mut order: Vec<(usize, &Entry)> = self.manifest.entries.iter().enumerate().collect();
        order.sort_by_key(|(_, e)| std::cmp::Reverse(e.path.components().count()));
        for (index, entry) in &order {
            if entry.made || std::fs::symlink_metadata(&entry.path).is_err() {
                continue;
            }
            if entry.before.present() && state::same_bytes(&entry.path, &entry.before) {
                continue;
            }
            let to = kept.join(format!("{index:05}"));
            self.log(&serde_json::json!({"event": "preserve", "path": entry.path, "to": to}))?;
            transfer::rename_on_device(&entry.path, &to)
                .map_err(io(format!("keep {} aside", entry.path.display())))?;
        }
        for (_, entry) in &order {
            if entry.made && entry.path.is_dir() {
                // Only an empty folder goes.
                let _ = std::fs::remove_dir(&entry.path);
            }
        }
        // Restore, parents first.
        order.reverse();
        for (_, entry) in &order {
            let Some(backup) = &entry.backup else {
                continue;
            };
            if state::same_bytes(&entry.path, &entry.before) {
                continue;
            }
            let from = self.dir.join("backups").join(backup);
            self.log(&serde_json::json!({"event": "restore", "path": entry.path}))?;
            if let Some(parent) = entry.path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(io(format!("restore {}", parent.display())))?;
            }
            match &entry.before {
                State::File(_) => transfer::clone_file(&from, &entry.path),
                State::Dir(_) => transfer::clone_tree(&from, &entry.path),
                State::Absent => Ok(()),
            }
            .map_err(io(format!("restore {}", entry.path.display())))?;
        }
        let wrong: Vec<String> = self
            .manifest
            .entries
            .iter()
            .filter(|e| !e.made && !state::same_bytes(&e.path, &e.before))
            .map(|e| e.path.display().to_string())
            .collect();
        if !wrong.is_empty() {
            let problem = format!("these weren't restored exactly: {}", wrong.join(", "));
            self.stop_with(&problem)?;
            return Err(MoveError::Refused(problem));
        }
        self.manifest.problem = None;
        self.set_phase(Phase::Undone)?;
        Ok(kept)
    }

    /// What the move left where it was on purpose (cleanup must keep each
    /// while the moved conversation names it), and what it keeps itself
    /// (backups, held branches, work kept aside by Undo).
    #[must_use]
    pub fn dependencies(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = self
            .manifest
            .plan
            .retained
            .iter()
            .map(|r| r.path.clone())
            .collect();
        out.push(self.dir.clone());
        out
    }
}

#[cfg(test)]
mod tests;
