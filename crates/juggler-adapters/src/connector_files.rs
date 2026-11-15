//! AJ-E3013 (D-116): one writer for every desktop App's connector.
//!
//! Each desktop adapter says what its connector is, as data
//! ([`juggler_adapter::connector::DesktopConnector::plan`]); this writes
//! it into that App's own config folder, and nowhere else:
//!
//! - **Only inside the folder.** Every part's path is relative, with no
//!   `..` ([`juggler_adapter::connector::stays_inside`]); the folder must
//!   be a real folder, and no folder on the way, nor the file itself, may
//!   be a link (never followed, never written through).
//! - **Preview first** ([`preview`]): each file as it is and as it would
//!   be, and whether Add would refuse.
//! - **Add records what it wrote** ([`add`], [`ConnectorRecord`]): each
//!   whole file's sha256, or a merged table's fingerprint (its values, its
//!   layout aside), and the folders it made. Something already there that
//!   AI Juggler didn't write (or that was changed since) is never
//!   replaced: Add refuses and names it. A file of AI Juggler's own from
//!   an older edition, still as written, is replaced.
//! - **Remove is exact** ([`remove`]): it deletes only what still matches
//!   its record (a merged table: that table alone, the rest of the file
//!   byte for byte), and names what changed, which stays.
//! - **Status** ([`status`]): absent, AI Juggler's and current, AI
//!   Juggler's but outdated (an older edition's), or changed by someone.
//! - Hidden files (`.claude-plugin/`) are written like any other: unlike
//!   a copied skill (`inventory_files::copy_tree`), every file here is AI
//!   Juggler's own and holds no secret.

use std::path::{Path, PathBuf};

use juggler_adapter::connector::{ConnectorPart, ConnectorPlan, stays_inside};
use juggler_adapter::handover::sha256_hex;
use juggler_adapter::safe_fs::{Dir, Expect};

/// What Add wrote, kept by the app so Remove is exact.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConnectorRecord {
    /// Each part as written.
    pub parts: Vec<RecordedPart>,
    /// The folders Add made (relative to the config folder), removed again
    /// when empty.
    pub dirs: Vec<PathBuf>,
}

/// One part as written.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RecordedPart {
    /// A whole file.
    File {
        /// Relative to the config folder.
        path: PathBuf,
        /// Its sha256 as written.
        sha256: String,
    },
    /// A table merged into a TOML file.
    TomlTable {
        /// The file, relative to the config folder.
        path: PathBuf,
        /// The table's dotted path.
        table: Vec<String>,
        /// Its values' fingerprint as written (its layout aside).
        fingerprint: String,
        /// Whether Add made the file (Remove takes an empty one away).
        created_file: bool,
    },
}

impl RecordedPart {
    /// Its file, relative to the config folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::File { path, .. } | Self::TomlTable { path, .. } => path,
        }
    }

    fn fingerprint(&self) -> &str {
        match self {
            Self::File { sha256, .. } => sha256,
            Self::TomlTable { fingerprint, .. } => fingerprint,
        }
    }

    fn same_part(&self, part: &ConnectorPart) -> bool {
        match (self, part) {
            (Self::File { path, .. }, ConnectorPart::File { path: planned, .. }) => path == planned,
            (
                Self::TomlTable { path, table, .. },
                ConnectorPart::TomlTable {
                    path: planned,
                    table: planned_table,
                    ..
                },
            ) => path == planned && table == planned_table,
            _ => false,
        }
    }
}

/// Where the connector is in an App.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorState {
    /// Not there.
    Absent,
    /// AI Juggler's, as this edition writes it.
    Current,
    /// AI Juggler's, as an older edition wrote it (Add updates it).
    Outdated,
    /// Changed by someone since AI Juggler wrote it, or something of the
    /// same name that AI Juggler didn't write.
    Changed,
}

/// What [`status`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// The state.
    pub state: ConnectorState,
    /// The files changed by someone (relative), for [`ConnectorState::Changed`].
    pub changed: Vec<PathBuf>,
}

/// What Add would do to one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// A new file.
    Create,
    /// AI Juggler's own file, rewritten (an older edition's).
    Replace,
    /// One table merged into the tool's file; the rest kept.
    Merge,
    /// Already as it would be: nothing written.
    Same,
    /// Something AI Juggler didn't write is there: Add refuses.
    Refuse,
}

/// One file in the preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewFile {
    /// Relative to the config folder.
    pub path: PathBuf,
    /// What Add would do.
    pub action: Action,
    /// AI Juggler's part as it is now: a whole file's text, or a merged
    /// table's own lines (`None`: not there, or not AI Juggler's: someone
    /// else's file is never shown).
    pub before: Option<String>,
    /// AI Juggler's part as Add writes it (a merged table: its own lines
    /// only; the rest of the tool's file is never shown, and stays as it is).
    pub after: String,
}

/// Why the connector couldn't be written or taken out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectorError {
    /// Refused before anything was written, and why.
    Refused(String),
    /// The system's error (anything written is taken back).
    Io(String),
}

impl std::fmt::Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(why) | Self::Io(why) => f.write_str(why),
        }
    }
}

impl From<std::io::Error> for ConnectorError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// What [`remove`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Removal {
    /// The files it took AI Juggler's parts out of (relative).
    pub removed: Vec<PathBuf>,
    /// The files changed since Add, left as they are (relative).
    pub kept: Vec<PathBuf>,
    /// The record of what's left (the changed parts), `None` when nothing is.
    pub left: Option<ConnectorRecord>,
}

fn refused(why: impl Into<String>) -> ConnectorError {
    ConnectorError::Refused(why.into())
}

/// The most bytes a file of the connector, or a tool config file it
/// merges into, may hold.
const FILE_MAX: u64 = 4 * 1024 * 1024;

/// The folder `rel`'s file is in, opened under `config_dir` (L5: the one
/// shared walk, `openat` with no link followed anywhere), and the file's
/// name; `None` when a folder on the way isn't there and nothing may be
/// set (`create_missing`). Folders made are added to `made` (relative to
/// `config_dir`).
fn folder_of(
    config_dir: &Path,
    rel: &Path,
    create_missing: bool,
    made: &mut Vec<PathBuf>,
) -> Result<Option<(Dir, std::ffi::OsString)>, ConnectorError> {
    if !stays_inside(rel) {
        return Err(refused(format!(
            "{} isn't inside the App's config folder",
            rel.display()
        )));
    }
    let name = rel
        .file_name()
        .ok_or_else(|| refused("a part names no file"))?
        .to_os_string();
    let root = Dir::root(config_dir)
        .map_err(|_| refused("the App's config folder is a link, a file, or isn't there"))?;
    let parent = rel.parent().unwrap_or_else(|| Path::new(""));
    match root.walk(parent, create_missing, made) {
        Ok(dir) => Ok(Some((dir, name))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !create_missing => Ok(None),
        Err(_) => Err(refused(format!(
            "{} can't be reached without going through a link",
            rel.display()
        ))),
    }
}

/// `rel`'s text under `config_dir`, read on the opened file (L5): `None`
/// when it isn't there.
///
/// # Errors
///
/// [`ConnectorError::Refused`] for a link, something not a plain file, or
/// a path outside the folder.
fn read_part(config_dir: &Path, rel: &Path) -> Result<Option<String>, ConnectorError> {
    let Some((dir, name)) = folder_of(config_dir, rel, false, &mut Vec::new())? else {
        return Ok(None);
    };
    let bytes = dir.read(&name, Expect::small(FILE_MAX)).map_err(|_| {
        refused(format!(
            "{} is a link, or isn't a plain file AI Juggler can read",
            rel.display()
        ))
    })?;
    bytes
        .map(|bytes| String::from_utf8(bytes).map_err(|_| refused("the file isn't text")))
        .transpose()
}

/// `rel` inside `config_dir`, when it's safe to read and write: relative,
/// no `..`, the folder a real folder, and no link anywhere on the way or
/// at the file itself (checked through the shared walk, L5).
///
/// # Errors
///
/// [`ConnectorError::Refused`] with why.
pub fn inside(config_dir: &Path, rel: &Path) -> Result<PathBuf, ConnectorError> {
    read_part(config_dir, rel)?;
    Ok(config_dir.join(rel))
}

/// A TOML value without its layout, keys sorted: what a fingerprint is of.
fn canonical(value: &toml::Value) -> String {
    match value {
        toml::Value::Table(table) => {
            let mut keys: Vec<&String> = table.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .into_iter()
                .map(|key| format!("{key:?}={}", canonical(&table[key])))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        toml::Value::Array(items) => format!(
            "[{}]",
            items.iter().map(canonical).collect::<Vec<_>>().join(",")
        ),
        other => other.to_string(),
    }
}

/// The table at `path` in the TOML `text` (`None`: not there, or the text
/// isn't TOML).
fn table_value(text: &str, path: &[String]) -> Option<toml::Value> {
    let parsed: toml::Table = text.parse().ok()?;
    let mut at = &toml::Value::Table(parsed);
    for key in path {
        at = at.as_table()?.get(key)?;
    }
    Some(at.clone())
}

/// The table at `path` in the TOML `text`, as its own lines (`None`: not
/// there).
fn table_text(text: &str, path: &[String]) -> Option<String> {
    let doc: toml_edit::DocumentMut = text.parse().ok()?;
    let mut at = doc.as_table();
    let (last, parents) = path.split_last()?;
    for key in parents {
        at = at.get(key)?.as_table()?;
    }
    let mut item = at.get(last)?.clone();
    unplace(&mut item);
    if let Some(table) = item.as_table_mut() {
        table.decor_mut().set_prefix("");
    }
    let mut only = toml_edit::DocumentMut::new();
    let mut into = only.as_table_mut();
    for key in parents {
        let mut made = toml_edit::Table::new();
        made.set_implicit(true);
        into.insert(key, toml_edit::Item::Table(made));
        into = into.get_mut(key)?.as_table_mut()?;
    }
    into.insert(last, item);
    Some(only.to_string())
}

/// The fingerprint of the table at `path` in `text`.
fn table_fingerprint(text: &str, path: &[String]) -> Option<String> {
    table_value(text, path).map(|value| sha256_hex(canonical(&value).as_bytes()))
}

/// One part, as it stands against its plan and its record.
struct Standing {
    /// Its file, relative to the App's config folder.
    rel: PathBuf,
    /// The file's text now.
    before: Option<String>,
    /// The part's fingerprint on disk (`None`: not there).
    disk: Option<String>,
    /// What this edition writes: its fingerprint.
    planned: String,
    /// The file as Add would leave it.
    after: String,
    /// What was recorded for it.
    recorded: Option<String>,
}

impl Standing {
    fn state(&self) -> ConnectorState {
        match (&self.disk, &self.recorded) {
            (None, _) => ConnectorState::Absent,
            (Some(disk), _) if *disk == self.planned => ConnectorState::Current,
            (Some(disk), Some(recorded)) if disk == recorded => ConnectorState::Outdated,
            (Some(_), _) => ConnectorState::Changed,
        }
    }
}

/// Drops the plan's positions from `item` and its tables, so they're
/// placed after the file's own.
fn unplace(item: &mut toml_edit::Item) {
    if let Some(table) = item.as_table_mut() {
        table.set_position(None);
        for (_, child) in table.iter_mut() {
            unplace(child);
        }
    }
}

/// Inserts (or replaces) the table at `path` in `doc` from `planned`.
fn merged(existing: &str, path: &[String], planned: &str) -> Result<String, ConnectorError> {
    let mut doc: toml_edit::DocumentMut = existing
        .parse()
        .map_err(|error: toml_edit::TomlError| refused(format!("the file isn't TOML: {error}")))?;
    let source: toml_edit::DocumentMut = planned
        .parse()
        .map_err(|error: toml_edit::TomlError| refused(format!("the plan isn't TOML: {error}")))?;
    let (last, parents) = path
        .split_last()
        .ok_or_else(|| refused("an empty table path"))?;
    let mut from = source.as_table();
    for key in parents {
        from = from
            .get(key)
            .and_then(toml_edit::Item::as_table)
            .ok_or_else(|| refused("the plan doesn't hold its table"))?;
    }
    let mut item = from
        .get(last)
        .cloned()
        .ok_or_else(|| refused("the plan doesn't hold its table"))?;
    // Placed after the file's own tables of the same parent (no position
    // of the plan's), with a blank line before it in a file that has any.
    unplace(&mut item);
    if let Some(table) = item.as_table_mut()
        && !existing.trim().is_empty()
    {
        table.decor_mut().set_prefix("\n");
    }
    let mut at = doc.as_table_mut();
    for key in parents {
        let entry = at.entry(key).or_insert_with(|| {
            let mut made = toml_edit::Table::new();
            made.set_implicit(true);
            toml_edit::Item::Table(made)
        });
        at = entry
            .as_table_mut()
            .ok_or_else(|| refused(format!("{key} in the file isn't a table")))?;
    }
    at.insert(last, item);
    Ok(doc.to_string())
}

/// Takes the table at `path` out of `table`, and the implicit tables above
/// it left empty.
fn take(table: &mut toml_edit::Table, path: &[String]) {
    match path {
        [] => {}
        [last] => {
            table.remove(last);
        }
        [first, rest @ ..] => {
            if let Some(inner) = table.get_mut(first).and_then(toml_edit::Item::as_table_mut) {
                take(inner, rest);
                if inner.is_empty() && inner.is_implicit() {
                    table.remove(first);
                }
            }
        }
    }
}

/// The file `text` without the table at `path` (and the implicit tables
/// above it that held only it).
fn unmerged(text: &str, path: &[String]) -> Result<String, ConnectorError> {
    let mut doc: toml_edit::DocumentMut = text
        .parse()
        .map_err(|error: toml_edit::TomlError| refused(format!("the file isn't TOML: {error}")))?;
    take(doc.as_table_mut(), path);
    Ok(doc.to_string())
}

fn standing(
    config_dir: &Path,
    part: &ConnectorPart,
    record: Option<&ConnectorRecord>,
) -> Result<Standing, ConnectorError> {
    let rel = part.path().to_path_buf();
    let before = read_part(config_dir, &rel)?;
    let recorded = record
        .and_then(|record| record.parts.iter().find(|done| done.same_part(part)))
        .map(|done| done.fingerprint().to_owned());
    Ok(match part {
        ConnectorPart::File { text, .. } => Standing {
            disk: before.as_ref().map(|now| sha256_hex(now.as_bytes())),
            planned: sha256_hex(text.as_bytes()),
            after: text.clone(),
            before,
            rel,
            recorded,
        },
        ConnectorPart::TomlTable { table, text, .. } => {
            let planned = table_fingerprint(text, table)
                .ok_or_else(|| refused("the plan doesn't hold its table"))?;
            let now = before.clone().unwrap_or_default();
            Standing {
                disk: table_fingerprint(&now, table),
                after: merged(&now, table, text)?,
                planned,
                before,
                rel,
                recorded,
            }
        }
    })
}

/// Each file as it is and as Add would leave it, with what Add would do.
///
/// # Errors
///
/// [`ConnectorError::Refused`] for a part outside the folder or through a
/// link; a read's error.
pub fn preview(
    config_dir: &Path,
    plan: &ConnectorPlan,
    record: Option<&ConnectorRecord>,
) -> Result<Vec<PreviewFile>, ConnectorError> {
    plan.parts
        .iter()
        .map(|part| {
            let found = standing(config_dir, part, record)?;
            let action = match (found.state(), part) {
                (ConnectorState::Current, _) => Action::Same,
                (ConnectorState::Changed, _) => Action::Refuse,
                (_, ConnectorPart::TomlTable { .. }) => Action::Merge,
                (ConnectorState::Absent, _) => Action::Create,
                _ => Action::Replace,
            };
            let ours = matches!(
                found.state(),
                ConnectorState::Current | ConnectorState::Outdated
            );
            let (before, after) = match part {
                ConnectorPart::File { text, .. } => (found.before.filter(|_| ours), text.clone()),
                ConnectorPart::TomlTable { table, text, .. } => (
                    found
                        .before
                        .as_deref()
                        .filter(|_| ours)
                        .and_then(|now| table_text(now, table)),
                    text.clone(),
                ),
            };
            Ok(PreviewFile {
                path: part.path().to_path_buf(),
                action,
                before,
                after,
            })
        })
        .collect()
}

/// Where the connector stands in the App at `config_dir`.
#[must_use]
pub fn status(config_dir: &Path, plan: &ConnectorPlan, record: Option<&ConnectorRecord>) -> Status {
    let mut states = Vec::new();
    let mut changed = Vec::new();
    for part in &plan.parts {
        // A link, or a folder where a file goes, is someone's.
        let state = standing(config_dir, part, record)
            .map_or(ConnectorState::Changed, |found| found.state());
        if state == ConnectorState::Changed {
            changed.push(part.path().to_path_buf());
        }
        states.push(state);
    }
    // A part an older edition wrote that this one doesn't.
    for done in record
        .map(|record| record.parts.as_slice())
        .unwrap_or_default()
    {
        if plan.parts.iter().any(|part| done.same_part(part)) {
            continue;
        }
        match on_disk(config_dir, done) {
            Some(now) if now == done.fingerprint() => states.push(ConnectorState::Outdated),
            Some(_) => {
                changed.push(done.path().to_path_buf());
                states.push(ConnectorState::Changed);
            }
            None => {}
        }
    }
    let state = if !changed.is_empty() {
        ConnectorState::Changed
    } else if states.iter().all(|state| *state == ConnectorState::Absent) {
        ConnectorState::Absent
    } else if states.iter().all(|state| *state == ConnectorState::Current) {
        ConnectorState::Current
    } else {
        ConnectorState::Outdated
    };
    Status { state, changed }
}

/// A recorded part's fingerprint on disk now (`None`: not there, or not
/// safe to read).
fn on_disk(config_dir: &Path, done: &RecordedPart) -> Option<String> {
    let text = read_part(config_dir, done.path()).ok()??;
    match done {
        RecordedPart::File { .. } => Some(sha256_hex(text.as_bytes())),
        RecordedPart::TomlTable { table, .. } => table_fingerprint(&text, table),
    }
}

/// Writes `text` as `rel` under `config_dir` (L5: through the shared
/// walk): a new file is created, never over one that appeared meanwhile;
/// an existing one is replaced whole by a rename, its permissions kept.
/// Folders made on the way are added to `made`.
fn write_part(
    config_dir: &Path,
    rel: &Path,
    text: &str,
    existed: bool,
    made: &mut Vec<PathBuf>,
) -> Result<(), ConnectorError> {
    let (dir, name) = folder_of(config_dir, rel, true, made)?
        .ok_or_else(|| refused("a folder on the way couldn't be made"))?;
    if existed {
        dir.replace(&name, text.as_bytes())?;
    } else {
        dir.create_new(&name, text.as_bytes())?;
    }
    Ok(())
}

/// Removes `rel` under `config_dir` (through the shared walk); `false`
/// when it wasn't there.
fn remove_part(config_dir: &Path, rel: &Path) -> Result<bool, ConnectorError> {
    let Some((dir, name)) = folder_of(config_dir, rel, false, &mut Vec::new())? else {
        return Ok(false);
    };
    Ok(dir.remove_file(&name)?)
}

/// Writes the connector, recording what it wrote. Nothing is written when
/// any part is refused.
///
/// # Errors
///
/// [`ConnectorError::Refused`] naming what's in the way (something AI
/// Juggler didn't write, or changed since; a link; a path outside the
/// folder); [`ConnectorError::Io`] when a write fails (what was written
/// is taken back).
pub fn add(
    config_dir: &Path,
    plan: &ConnectorPlan,
    record: Option<&ConnectorRecord>,
) -> Result<ConnectorRecord, ConnectorError> {
    let found: Vec<Standing> = plan
        .parts
        .iter()
        .map(|part| standing(config_dir, part, record))
        .collect::<Result<_, _>>()?;
    let in_the_way: Vec<String> = plan
        .parts
        .iter()
        .zip(&found)
        .filter(|(_, found)| found.state() == ConnectorState::Changed)
        .map(|(part, _)| part.path().display().to_string())
        .collect();
    if !in_the_way.is_empty() {
        return Err(refused(format!(
            "{} is already there and isn't AI Juggler's as it wrote it, so nothing was written",
            in_the_way.join(", ")
        )));
    }
    let mut made = record.map(|record| record.dirs.clone()).unwrap_or_default();
    // L6: Add is all or nothing. Each step is kept so a failure part-way
    // takes every one back: files made are removed, files replaced get
    // their old text again, folders made go when empty.
    let mut done: Vec<(PathBuf, Option<String>)> = Vec::new();
    let mut new_dirs: Vec<PathBuf> = Vec::new();
    let mut parts = Vec::new();
    for (part, found) in plan.parts.iter().zip(&found) {
        let existed = found.before.is_some();
        let wrote = if found.state() == ConnectorState::Current {
            Ok(()) // already as this edition writes it
        } else {
            write_part(config_dir, &found.rel, &found.after, existed, &mut new_dirs)
                .map(|()| done.push((found.rel.clone(), found.before.clone())))
        };
        if let Err(error) = wrote {
            take_back(config_dir, &done, &new_dirs);
            return Err(error);
        }
        parts.push(match part {
            ConnectorPart::File { path, .. } => RecordedPart::File {
                path: path.clone(),
                sha256: found.planned.clone(),
            },
            ConnectorPart::TomlTable { path, table, .. } => RecordedPart::TomlTable {
                path: path.clone(),
                table: table.clone(),
                fingerprint: found.planned.clone(),
                created_file: !existed
                    || record.is_some_and(|record| {
                        record.parts.iter().any(|done| {
                            matches!(
                                done,
                                RecordedPart::TomlTable {
                                    created_file: true,
                                    ..
                                }
                            ) && done.same_part(part)
                        })
                    }),
            },
        });
    }
    // An older edition's part this one doesn't write: taken out while
    // it's as written.
    if let Some(old) = record {
        let gone: Vec<RecordedPart> = old
            .parts
            .iter()
            .filter(|done| !plan.parts.iter().any(|part| done.same_part(part)))
            .cloned()
            .collect();
        if !gone.is_empty() {
            let _ = remove(
                config_dir,
                &ConnectorRecord {
                    parts: gone,
                    dirs: Vec::new(),
                },
            );
        }
    }
    made.extend(new_dirs);
    made.sort();
    made.dedup();
    Ok(ConnectorRecord { parts, dirs: made })
}

/// L6: undoes a part-way Add: each file written gets its old text back
/// (or goes, when Add made it), newest first, then the folders Add made
/// go when empty. Best effort: a step that fails leaves the rest going.
fn take_back(config_dir: &Path, done: &[(PathBuf, Option<String>)], made: &[PathBuf]) {
    for (rel, before) in done.iter().rev() {
        let _ = match before {
            Some(text) => write_part(config_dir, rel, text, true, &mut Vec::new()),
            None => remove_part(config_dir, rel).map(|_| ()),
        };
    }
    let mut made = made.to_vec();
    made.sort_by_key(|dir| std::cmp::Reverse(dir.components().count()));
    for dir in &made {
        if let Ok(Some((parent, name))) = folder_of(config_dir, dir, false, &mut Vec::new()) {
            let _ = parent.remove_dir(&name);
        }
    }
}

/// Takes the connector out: each part only while it's as recorded; what
/// changed stays, named.
///
/// # Errors
///
/// [`ConnectorError::Io`] when a file can't be rewritten or removed.
pub fn remove(config_dir: &Path, record: &ConnectorRecord) -> Result<Removal, ConnectorError> {
    let mut removed = Vec::new();
    let mut kept = Vec::new();
    let mut left = Vec::new();
    for done in &record.parts {
        let rel = done.path();
        let Ok(found) = read_part(config_dir, rel) else {
            kept.push(rel.to_path_buf());
            left.push(done.clone());
            continue;
        };
        let Some(text) = found else {
            continue; // already gone
        };
        match done {
            RecordedPart::File { sha256, .. } => {
                if sha256_hex(text.as_bytes()) == *sha256 {
                    remove_part(config_dir, rel)?;
                    removed.push(rel.to_path_buf());
                } else {
                    kept.push(done.path().to_path_buf());
                    left.push(done.clone());
                }
            }
            RecordedPart::TomlTable {
                table,
                fingerprint,
                created_file,
                ..
            } => match table_fingerprint(&text, table) {
                None => {} // the user took it out
                Some(now) if now == *fingerprint => {
                    let rest = unmerged(&text, table)?;
                    if *created_file && rest.trim().is_empty() {
                        remove_part(config_dir, rel)?;
                    } else {
                        write_part(config_dir, rel, &rest, true, &mut Vec::new())?;
                    }
                    removed.push(done.path().to_path_buf());
                }
                Some(_) => {
                    kept.push(done.path().to_path_buf());
                    left.push(done.clone());
                }
            },
        }
    }
    let mut dirs = record.dirs.clone();
    dirs.sort_by_key(|dir| std::cmp::Reverse(dir.components().count()));
    for dir in &dirs {
        // Only when empty, and never through a link (the shared walk).
        if let Ok(Some((parent, name))) = folder_of(config_dir, dir, false, &mut Vec::new()) {
            let _ = parent.remove_dir(&name);
        }
    }
    let left = (!left.is_empty()).then(|| ConnectorRecord {
        parts: left,
        dirs: record
            .dirs
            .iter()
            .filter(|dir| {
                Dir::root(config_dir)
                    .and_then(|root| root.walk(dir, false, &mut Vec::new()))
                    .is_ok()
            })
            .cloned()
            .collect(),
    });
    Ok(Removal {
        removed,
        kept,
        left,
    })
}

/// AJ-E3013: AI Juggler's connector as a tool's inventory lists it: a
/// plugin provided by AI Juggler (`id` is the tool's own name for it),
/// never a user's skill or server to copy between accounts.
#[must_use]
pub fn connector_item(tool: &str, id: &str) -> juggler_adapter::inventory::Item {
    use juggler_adapter::connector::{CONNECTOR_BY, CONNECTOR_ITEM};
    use juggler_adapter::inventory::{Detail, Item, ItemKind, Origin, Scope};
    Item {
        tool: tool.to_owned(),
        kind: ItemKind::Plugin,
        name: CONNECTOR_ITEM.to_owned(),
        scope: Scope::User,
        detail: Detail::Plugin { id: id.to_owned() },
        origin: Origin::Provided {
            by: CONNECTOR_BY.to_owned(),
        },
    }
}

/// Whether `item` is AI Juggler's connector (any tool's).
#[must_use]
pub fn is_connector(item: &juggler_adapter::inventory::Item) -> bool {
    use juggler_adapter::connector::{CONNECTOR_BY, CONNECTOR_ITEM};
    item.kind == juggler_adapter::inventory::ItemKind::Plugin
        && item.name == CONNECTOR_ITEM
        && matches!(&item.origin, juggler_adapter::inventory::Origin::Provided { by } if by == CONNECTOR_BY)
}

/// How another account gets AI Juggler's connector: AI Juggler adds it
/// there, on the user's say-so (never a copy).
#[must_use]
pub fn connector_turn_on() -> juggler_adapter::inventory::SetUp {
    juggler_adapter::inventory::SetUp::TurnOn {
        get_it: juggler_adapter::inventory::GetIt {
            steps: vec![
                "In AI Juggler, open the pool this session is in.".to_owned(),
                "On {account}'s card, click Add AI Juggler's connector and check what it writes."
                    .to_owned(),
            ],
            then: Some("Its sessions join the pool when they next start.".to_owned()),
            ..juggler_adapter::inventory::GetIt::default()
        },
        can_copy: false,
    }
}

#[cfg(test)]
#[path = "connector_files_tests.rs"]
mod tests;
