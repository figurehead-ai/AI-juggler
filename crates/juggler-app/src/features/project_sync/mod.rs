//! AJ-E1507: a project's rules and MCP servers, made the same across tools
//! and accounts in one reviewed click.
//!
//! Rules: each tool reads its own rules file in the project (the adapters'
//! `ToolSpec::rules_files`). The user picks the master; every other file
//! shows how it stands and, on request, the diff applying the master would
//! make. A file is written only when the user applies it, and only the
//! project's rules files are ever written: never a link, never a file that
//! changed since it was shown. The file from before is kept in AI
//! Juggler's data folder, so Settings can undo the change.
//!
//! Servers: each account's MCP servers, this project's own included, and
//! what one account could get from another, with the target tool's verdict
//! (the switch set-up's, AJ-E1542): copied as it is, translated, or why it
//! can't be. Secrets never move.

pub mod commands;
pub mod dto;

pub use dto::{
    DiffLineInfo, RulesDiffInfo, RulesFileInfo, RulesSyncInfo, ServerAccountInfo, ServerCopyInfo,
    ServersSyncInfo,
};

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::ToolSpec;
use juggler_adapter::inventory::{Applied, Item, ItemKind, Undo};
use juggler_core::model::{Account, ProjectId};
use juggler_handover::sync::{self, LineChange, RulesFile, Standing};

use crate::error::{AppError, AppResult};
use crate::features::switch_setup::{self, AppliedInfo, Sides};
use crate::state::AppState;

/// Rules files bigger than this aren't compared.
const MAX_RULES_BYTES: u64 = 2 * 1024 * 1024;

/// Where the files from before a sync are kept, in AI Juggler's data folder.
const BACKUPS: &str = "sync-backups";

/// The project, as far as the sync needs it.
struct Project {
    name: String,
    root: PathBuf,
    /// The tools that have worked in it (a session, or the history from
    /// before AI Juggler).
    tools_used: HashSet<String>,
}

fn project(state: &Arc<AppState>, project_id: &str) -> AppResult<Project> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let record = storage
        .projects()
        .get(&ProjectId::new(project_id))
        .map_err(|_| AppError::Refused("that project isn't there any more".to_owned()))?;
    if !record.root.is_dir() {
        return Err(AppError::Refused(format!(
            "{} isn't on this Mac any more",
            record.root.display()
        )));
    }
    let accounts = storage.accounts().list()?;
    let mut tools_used: HashSet<String> = accounts
        .iter()
        .filter(|account| record.history.accounts.contains(&account.label))
        .map(|account| account.harness.to_string())
        .collect();
    // Each launch setting's tool, through its account.
    let mut harness_of: HashMap<String, String> = HashMap::new();
    for account in &accounts {
        for profile in storage.profiles().list_by_account(&account.id)? {
            harness_of.insert(profile.id.to_string(), account.harness.to_string());
        }
    }
    for session in storage.sessions().list(None)? {
        if let Some(harness) = harness_of
            .get(session.profile.as_str())
            .filter(|_| session.project.as_ref() == Some(&record.id))
        {
            tools_used.insert(harness.clone());
        }
    }
    Ok(Project {
        name: record.name,
        root: record.root,
        tools_used,
    })
}

/// The specs of the tools the user has an account for.
fn specs_in_play(state: &Arc<AppState>) -> AppResult<Vec<ToolSpec>> {
    let harnesses: BTreeSet<String> = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .accounts()
            .list()?
            .into_iter()
            .map(|account| account.harness.to_string())
            .collect()
    };
    Ok(harnesses
        .iter()
        .filter_map(|id| {
            state
                .registry
                .get(&juggler_core::model::HarnessId::new(id.as_str()))
        })
        .map(|adapter| adapter.spec())
        .collect())
}

/// A rules file as read, with what the comparison doesn't carry.
struct Read {
    file: RulesFile,
    path: PathBuf,
    /// Not text, or too big to compare.
    unreadable: bool,
    /// It's a link (to the master or anywhere else): never written.
    is_link: bool,
    sha256: Option<String>,
}

/// A rules file name is a plain relative path inside the project.
fn inside(name: &str) -> bool {
    let path = Path::new(name);
    !name.is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Every rules file the tools in play read, once each.
fn read_files(root: &Path, specs: &[ToolSpec]) -> Vec<Read> {
    let mut names: Vec<(String, Vec<String>)> = Vec::new();
    for spec in specs {
        for name in spec.rules_files.iter().filter(|name| inside(name)) {
            match names.iter_mut().find(|(known, _)| known == name) {
                Some((_, tools)) => tools.push(spec.id.clone()),
                None => names.push((name.clone(), vec![spec.id.clone()])),
            }
        }
    }
    // Which file each name is: a link, a hard link or a case-insensitive
    // disk can make two names one file, and it's then compared as a link
    // and never written (review: a "read" apply would replace the master).
    let ids: Vec<Option<String>> = names
        .iter()
        .map(|(name, _)| juggler_adapter::inventory::file_identity(&root.join(name)))
        .collect();
    let all: Vec<String> = names.iter().map(|(name, _)| name.clone()).collect();
    let symlinks: Vec<bool> = names
        .iter()
        .map(|(name, _)| {
            std::fs::symlink_metadata(root.join(name))
                .is_ok_and(|meta| meta.file_type().is_symlink())
        })
        .collect();
    names
        .into_iter()
        .enumerate()
        .map(|(at, (name, tools))| {
            let path = root.join(&name);
            let is_symlink = symlinks[at];
            // A link names the file it points to, a plain file. Of two plain
            // names of one file (a hard link, a case-insensitive disk), the
            // later one is the link. A plain file is never a link to a
            // symlink pointing at it (re-review: CLAUDE.md → AGENTS.md made
            // both links, and nothing could be compared).
            let link_to = ids[at].as_ref().and_then(|id| {
                ids.iter()
                    .enumerate()
                    .find(|(other, other_id)| {
                        *other != at
                            && other_id.as_ref() == Some(id)
                            && !symlinks[*other]
                            && (is_symlink || *other < at)
                    })
                    .map(|(other, _)| all[other].clone())
            });
            let found = std::fs::metadata(&path).ok();
            // Something by that name that isn't a file (a folder) can't be
            // compared or written.
            let not_a_file = found.as_ref().is_some_and(|meta| !meta.is_file());
            let meta = found.filter(std::fs::Metadata::is_file);
            let modified_ms = meta.as_ref().and_then(|meta| {
                meta.modified()
                    .ok()
                    .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                    .and_then(|at| i64::try_from(at.as_millis()).ok())
            });
            let bytes = meta
                .as_ref()
                .filter(|meta| meta.len() <= MAX_RULES_BYTES)
                .and_then(|_| std::fs::read(&path).ok());
            let sha256 = bytes.as_deref().map(juggler_adapter::handover::sha256_hex);
            let text = bytes.and_then(|bytes| String::from_utf8(bytes).ok());
            let unreadable = not_a_file || (meta.is_some() && text.is_none());
            Read {
                file: RulesFile {
                    name,
                    tools,
                    modified_ms: text.as_ref().and(modified_ms),
                    text,
                    link_to: link_to.clone(),
                },
                path,
                unreadable,
                is_link: is_symlink || link_to.is_some(),
                sha256,
            }
        })
        .collect()
}

/// The comparison's files: unreadable ones take no part.
fn comparable(reads: &[Read]) -> Vec<RulesFile> {
    reads
        .iter()
        .filter(|read| !read.unreadable)
        .map(|read| read.file.clone())
        .collect()
}

/// The master: `picked` when it can be one, or the default.
fn master_of(picked: Option<&str>, files: &[RulesFile], specs: &[ToolSpec]) -> Option<String> {
    picked
        .and_then(|name| {
            files
                .iter()
                .find(|file| file.name == name && sync::can_be_master(file, files, specs))
        })
        .map(|file| file.name.clone())
        .or_else(|| sync::default_master(files, specs))
}

fn tool_names(state: &Arc<AppState>, tools: &[String]) -> Vec<String> {
    tools
        .iter()
        .map(|tool| crate::features::tools::display_name(&state.registry, tool))
        .collect()
}

/// The project's rules files and how each stands beside the master
/// (`picked`, or the newest file with its own text). Read-only.
///
/// # Errors
///
/// [`AppError::Refused`] when the project is gone; storage errors.
pub fn rules_impl(
    state: &Arc<AppState>,
    project_id: &str,
    picked: Option<&str>,
) -> AppResult<RulesSyncInfo> {
    let project = project(state, project_id)?;
    let specs = specs_in_play(state)?;
    let reads = read_files(&project.root, &specs);
    let files = comparable(&reads);
    let master = master_of(picked, &files, &specs);
    let master_file = master
        .as_ref()
        .and_then(|name| files.iter().find(|file| &file.name == name));
    let mut out = Vec::new();
    let mut differ = 0usize;
    for read in &reads {
        let file = &read.file;
        let broken = sync::reads_missing(file, &files, &specs);
        let (standing, words) = if read.unreadable {
            (
                "unreadable",
                "Not a text file, or too big to compare".to_owned(),
            )
        } else if let Some(other) = &broken {
            // Its tools see none of those rules (review: it read as "same").
            ("broken", format!("Reads {other}, which isn't there"))
        } else {
            match master_file.map(|master| (master, sync::standing(file, master, &specs))) {
                None if file.text.is_none() => ("missing", "Not there".to_owned()),
                None => ("same", "Reads another rules file".to_owned()),
                Some((_, Standing::Master)) => ("master", "The master".to_owned()),
                Some((master, Standing::Same)) => ("same", format!("Same as {}", master.name)),
                Some((master, Standing::Reads)) => ("reads", format!("Reads {}", master.name)),
                Some((master, Standing::Link)) => ("link", format!("A link to {}", master.name)),
                Some((master, Standing::Differs { .. })) => {
                    ("differs", format!("Differs from {}", master.name))
                }
                Some((_, Standing::Missing)) => ("missing", "Not there".to_owned()),
            }
        };
        let can_apply = !read.unreadable && !read.is_link && master.as_deref() != Some(&file.name);
        // What the summary counts, and what the sheet opens on. A missing
        // file counts only for a tool that works here: a Codex account
        // doesn't make every project without AGENTS.md "differ". A link to
        // somewhere else can't be applied, so it isn't counted for ever.
        let counts = match standing {
            "broken" => true,
            "differs" => can_apply,
            // Only beside a master: a project with no rules file at all has
            // nothing to differ from (re-review: it warned on every one).
            "missing" => {
                master.is_some()
                    && file
                        .tools
                        .iter()
                        .any(|tool| project.tools_used.contains(tool))
            }
            _ => false,
        };
        differ += usize::from(counts);
        let words = if read.is_link && standing == "differs" {
            "A link to another file, so it's left as it is".to_owned()
        } else {
            words
        };
        out.push(RulesFileInfo {
            name: file.name.clone(),
            tools: tool_names(state, &file.tools),
            standing: standing.to_owned(),
            words,
            modified_ms: file.modified_ms,
            can_be_master: !read.unreadable && sync::can_be_master(file, &files, &specs),
            can_apply,
            counts,
            import_line: master
                .as_deref()
                .filter(|master| *master != file.name)
                .and_then(|master| sync::import_line(file, master, &specs)),
            sha256: read.sha256.clone(),
        });
    }
    let summary = (master.is_some() || differ > 0).then(|| match differ {
        0 => "Rules in sync".to_owned(),
        1 => "1 rules file differs".to_owned(),
        n => format!("{n} rules files differ"),
    });
    Ok(RulesSyncInfo {
        master,
        in_sync: differ == 0,
        summary,
        files: out,
    })
}

/// The text `file` would get: the master's (`copy`), or the line that
/// makes it read the master (`read`).
fn new_text(
    how: &str,
    file: &RulesFile,
    master: &RulesFile,
    specs: &[ToolSpec],
) -> AppResult<String> {
    match how {
        "copy" => Ok(master.text.clone().unwrap_or_default()),
        "read" => sync::import_line(file, &master.name, specs)
            .map(|line| format!("{line}\n"))
            .ok_or_else(|| AppError::Refused(format!("{} can't read another file", file.name))),
        _ => Err(AppError::Refused(format!("unknown way to apply: {how}"))),
    }
}

/// The project's files, the master and `file`, checked for a diff or an
/// apply.
struct Pair {
    project: Project,
    specs: Vec<ToolSpec>,
    reads: Vec<Read>,
    /// Where `file` and the master are in `reads`.
    target: usize,
    master: usize,
}

fn pair(state: &Arc<AppState>, project_id: &str, master: &str, file: &str) -> AppResult<Pair> {
    let project = project(state, project_id)?;
    let specs = specs_in_play(state)?;
    let reads = read_files(&project.root, &specs);
    let files = comparable(&reads);
    let master_at = reads
        .iter()
        .position(|read| {
            read.file.name == master
                && !read.unreadable
                && sync::can_be_master(&read.file, &files, &specs)
        })
        .ok_or_else(|| AppError::Refused(format!("{master} can't be the master")))?;
    let target_at = reads
        .iter()
        .position(|read| read.file.name == file)
        .filter(|at| *at != master_at)
        .ok_or_else(|| {
            AppError::Refused(format!(
                "{file} isn't a rules file AI Juggler can change here"
            ))
        })?;
    Ok(Pair {
        project,
        specs,
        reads,
        target: target_at,
        master: master_at,
    })
}

impl Pair {
    fn target(&self) -> &Read {
        &self.reads[self.target]
    }

    fn master(&self) -> &Read {
        &self.reads[self.master]
    }
}

/// What applying the master to `file` would change (`how`: `copy` or
/// `read`). Read-only.
///
/// # Errors
///
/// [`AppError::Refused`] when `master` can't be the master, `file` isn't
/// one of the project's rules files, or `how` isn't possible for it.
pub fn diff_impl(
    state: &Arc<AppState>,
    project_id: &str,
    master: &str,
    file: &str,
    how: &str,
) -> AppResult<RulesDiffInfo> {
    let pair = pair(state, project_id, master, file)?;
    let target = &pair.target().file;
    let new = new_text(how, target, &pair.master().file, &pair.specs)?;
    let old = target.text.clone().unwrap_or_default();
    let mut added = 0u32;
    let mut removed = 0u32;
    let groups = sync::diff(&old, &new)
        .into_iter()
        .map(|group| {
            group
                .into_iter()
                .map(|line| {
                    let change = match line.change {
                        LineChange::Kept => "kept",
                        LineChange::Added => {
                            added += 1;
                            "added"
                        }
                        LineChange::Removed => {
                            removed += 1;
                            "removed"
                        }
                    };
                    DiffLineInfo {
                        change: change.to_owned(),
                        text: line.text,
                        old_number: line.old_number.and_then(|n| u32::try_from(n).ok()),
                        new_number: line.new_number.and_then(|n| u32::try_from(n).ok()),
                    }
                })
                .collect()
        })
        .collect();
    Ok(RulesDiffInfo {
        file: file.to_owned(),
        master: master.to_owned(),
        how: how.to_owned(),
        groups,
        added,
        removed,
    })
}

/// Applies the master to `file`, against exactly what was shown: `file_sha`
/// (`None` when it wasn't there) and `master_sha` must still be the files'.
/// The file from before is kept so Settings can undo it.
///
/// # Errors
///
/// [`AppError::Refused`] when either file changed since, `file` is a link
/// or isn't text, or `how` isn't possible; the write's error.
pub fn apply_impl(
    state: &Arc<AppState>,
    project_id: &str,
    master: &str,
    file: &str,
    how: &str,
    file_sha: Option<&str>,
    master_sha: &str,
) -> AppResult<AppliedInfo> {
    let pair = pair(state, project_id, master, file)?;
    let target = pair.target();
    if target.is_link || target.unreadable {
        return Err(AppError::Refused(format!(
            "{file} is a link or isn't text, so it's left as it is"
        )));
    }
    if target.sha256.as_deref() != file_sha || pair.master().sha256.as_deref() != Some(master_sha) {
        return Err(AppError::Refused(format!(
            "{file} or {master} changed since you looked: look again before applying"
        )));
    }
    let new = new_text(how, &target.file, &pair.master().file, &pair.specs)?;
    let written = juggler_adapter::handover::sha256_hex(new.as_bytes());
    let undo = if let Some(before) = &target.file.text {
        let backups = state.data_dir.join(BACKUPS);
        std::fs::create_dir_all(&backups)
            .map_err(|e| AppError::io("keep the file from before", &e))?;
        let backup = backups.join(format!(
            "{}-{}",
            crate::commands::unique_suffix(),
            file.replace(['/', '\\'], "_")
        ));
        create_new(&backup, before).map_err(|e| AppError::io("keep the file from before", &e))?;
        if let Err(error) = juggler_adapter::inventory::rewrite_in_place(&target.path, &new) {
            let _ = std::fs::remove_file(&backup);
            return Err(AppError::io("apply the rules", &error));
        }
        Undo::Restore {
            file: target.path.clone(),
            backup,
            sha256: written,
        }
    } else {
        create_new(&target.path, &new).map_err(|e| AppError::io("apply the rules", &e))?;
        Undo::RemoveCreated {
            files: vec![(target.path.clone(), written)],
            dirs: Vec::new(),
        }
    };
    let message = if how == "read" {
        format!("{file} now reads {master}")
    } else {
        format!("{file} now has the rules from {master}")
    };
    let applied = Applied {
        summary: message.clone(),
        undo,
        note: None,
    };
    let recorded = switch_setup::record(
        state,
        None,
        Some(pair.project.name.clone()),
        format!("rules in {file}"),
        applied.clone(),
    );
    let record = match recorded {
        Ok(record) => record,
        Err(error) => {
            // Without a record there'd be no way to undo it: put the file
            // back as it was (review of AJ-E1507, M1).
            let _ = switch_setup::undo_now(&applied.undo);
            return Err(error);
        }
    };
    Ok(AppliedInfo {
        offer: file.to_owned(),
        record: Some(record),
        message,
    })
}

/// Writes a new file, refusing one that's already there.
fn create_new(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut created| created.write_all(text.as_bytes()))
}

/// An account, its tool's adapter, and its MCP servers for the project.
struct Holder {
    account: Account,
    adapter: juggler_adapter::SharedAdapter,
    servers: Vec<Item>,
}

fn holders(state: &Arc<AppState>, root: &Path) -> AppResult<Vec<Holder>> {
    let accounts = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage.accounts().list()?
    };
    Ok(accounts
        .into_iter()
        .filter_map(|account| {
            let adapter = state.registry.get(&account.harness)?;
            let servers = adapter
                .inventory(&account.state_dir, Some(root))
                .into_iter()
                .filter(|item| item.kind == ItemKind::McpServer)
                .collect();
            Some(Holder {
                account,
                adapter,
                servers,
            })
        })
        .collect())
}

/// Each account's MCP servers, and what one could get from another: one
/// source per server and account, the same tool's first (copied as it
/// is). Read-only.
///
/// # Errors
///
/// [`AppError::Refused`] when the project is gone; storage errors.
pub fn servers_impl(state: &Arc<AppState>, project_id: &str) -> AppResult<ServersSyncInfo> {
    let project = project(state, project_id)?;
    let holders = holders(state, &project.root)?;
    let accounts = holders
        .iter()
        .map(|holder| {
            let mut servers: Vec<String> = holder
                .servers
                .iter()
                .map(|item| item.name.clone())
                .collect();
            servers.sort();
            servers.dedup();
            ServerAccountInfo {
                account: holder.account.id.to_string(),
                label: holder.account.label.clone(),
                tool: crate::features::tools::display_name(
                    &state.registry,
                    holder.account.harness.as_str(),
                ),
                servers,
            }
        })
        .collect();
    let mut copies: Vec<ServerCopyInfo> = Vec::new();
    for target in &holders {
        // The same tool's accounts first, so a server is copied as it is
        // where it can be.
        let mut sources: Vec<&Holder> = holders
            .iter()
            .filter(|source| source.account.id != target.account.id)
            .collect();
        sources.sort_by_key(|source| source.account.harness != target.account.harness);
        for source in sources {
            let (offers, _) = switch_setup::offers(
                &target.account,
                target.adapter.as_ref(),
                &source.servers,
                &target.servers,
            );
            for offer in offers {
                if copies
                    .iter()
                    .any(|copy| copy.to == target.account.id.as_str() && copy.server == offer.name)
                {
                    continue;
                }
                copies.push(ServerCopyInfo {
                    server: offer.name,
                    from: source.account.id.to_string(),
                    from_label: source.account.label.clone(),
                    to: target.account.id.to_string(),
                    to_label: target.account.label.clone(),
                    offer: offer.id,
                    verdict: offer.verdict,
                    note: offer.note,
                });
            }
        }
    }
    copies.sort_by(|a, b| {
        (a.verdict == "impossible", &a.to_label, &a.server).cmp(&(
            b.verdict == "impossible",
            &b.to_label,
            &b.server,
        ))
    });
    Ok(ServersSyncInfo { accounts, copies })
}

/// Copies one server (`offer`, from [`servers_impl`]) from account `from`
/// to account `to`, recorded so Settings can undo it.
///
/// # Errors
///
/// [`AppError::Refused`] when the project or an account is gone; the
/// set-up's errors.
pub fn servers_apply_impl(
    state: &Arc<AppState>,
    project_id: &str,
    from: &str,
    to: &str,
    offer: &str,
) -> AppResult<AppliedInfo> {
    let project = project(state, project_id)?;
    let account = |id: &str| -> AppResult<Account> {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .accounts()
            .get(&juggler_core::model::AccountId::new(id))
            .map_err(|_| AppError::Refused("that account isn't there any more".to_owned()))
    };
    let sides = Sides {
        source: account(from)?,
        target: account(to)?,
        project: Some(project.root),
    };
    switch_setup::apply_sides(
        state,
        &sides,
        &[offer.to_owned()],
        Some(ItemKind::McpServer),
    )?
    .into_iter()
    .next()
    .ok_or_else(|| AppError::Refused("nothing was copied".to_owned()))
}
