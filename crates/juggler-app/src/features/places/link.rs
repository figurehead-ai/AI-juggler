//! Linking a desktop App to the tool folder its sessions use, by
//! evidence (AJ-E2327, through the shared ownership module): a profile's
//! running sessions' pid files under its own main process (AJ-E2315,
//! [`folder_of_its_sessions`]: the folder its Code child writes in, which
//! is the folder it was given, read from the child's own file and never
//! from any process's environment, D-37), then its own records' sessions
//! found as transcripts in one folder, then the folder its app declares
//! for its one instance (ADR-0032, the Codex app's `~/.codex`), then AI
//! Juggler's own record (its link so far, or the folder the user chose),
//! and only then, as a fallback the evidence confirms or corrects later,
//! the folder named like the profile ([`ownership::sessions::decide`]).
//! What linked it, or why nothing could, is kept for the App's row
//! ([`LinkNote`]). Moved out of `places` for its size.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::environment::DiscoveredInstall;

use super::book::KnownPlace;
use super::with_book;
use crate::features::ownership;
use crate::features::ownership::sessions::{Evidence, LinkSource};
use crate::state::AppState;

/// How long a look that found nothing waits before the next, while the
/// profile runs (a Code session may open any moment).
const RETRY_RUNNING: Duration = Duration::from_secs(60);
/// The same while it's closed: nothing new tells until it runs (a start
/// looks again at once) or its app records a session.
const RETRY_CLOSED: Duration = Duration::from_mins(10);
/// The stored record of what linked each App (`<prefix><place id>` →
/// the [`LinkSource`]'s word), so its row says it after a restart.
const SOURCE_KEY: &str = "places.folder_by:";

/// AJ-E2327: what the last look for a desktop App's tool folder found,
/// for its row: the folder and what told, or why nothing did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkNote {
    /// Linked to `folder` by `by`.
    Linked {
        /// The tool folder, where it is.
        folder: PathBuf,
        /// What told.
        by: LinkSource,
    },
    /// Not linked: why, in plain words.
    Unlinked(String),
}

/// AJ-E2327: what Add (or Choose its folder…) did with the App's folder,
/// for its confirmation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Linked {
    /// The folder it uses now, where it is (never copied).
    pub folder: Option<PathBuf>,
    /// What told ([`LinkSource::word`]).
    pub by: Option<LinkSource>,
    /// Why no folder is linked, when none is.
    pub why: Option<String>,
}

/// What a look gathered for one App: its adapter's tool, the folders
/// found, and the evidence.
struct Look {
    harness: juggler_core::model::HarnessId,
    installs: Vec<DiscoveredInstall>,
    running: Option<PathBuf>,
    records: Option<PathBuf>,
    recorded: Option<PathBuf>,
    declared: Option<PathBuf>,
    named: Option<PathBuf>,
    /// How many conversations its own records list (`None`: its app
    /// keeps no such list, or it has no instance folder).
    listed: Option<usize>,
    /// Its records' conversations sit in two folders alike.
    tied: bool,
    /// It runs now.
    runs: bool,
    /// Its tool's name (`Claude Code`), for the words.
    tool: String,
}

impl Look {
    fn evidence(&self) -> Evidence<'_> {
        Evidence {
            running: self.running.as_deref(),
            records: self.records.as_deref(),
            recorded: self.recorded.as_deref(),
            declared: self.declared.as_deref(),
            named: self.named.as_deref(),
        }
    }

    /// Why nothing tells, in plain words (the App's row says it).
    fn why_none(&self) -> String {
        let tool = &self.tool;
        if self.tied {
            return format!(
                "Its sessions are in more than one {tool} folder, so AI Juggler won't guess which one is its own."
            );
        }
        match (self.runs, self.listed.unwrap_or(0)) {
            (true, 0) => format!(
                "None of its sessions is open and its app lists none yet, so AI Juggler can't tell which {tool} folder it uses."
            ),
            (true, _) => format!(
                "None of its sessions is open, and the sessions its app lists aren't in any {tool} folder AI Juggler can find."
            ),
            (false, 0) => format!(
                "It isn't running and its app lists no sessions yet, so AI Juggler can't tell which {tool} folder it uses."
            ),
            (false, _) => format!(
                "The sessions its app lists aren't in any {tool} folder AI Juggler can find."
            ),
        }
    }
}

/// The tool folders discovery finds for `adapter` (a test's made-up Mac
/// has its own).
fn installs_of(
    state: &Arc<AppState>,
    adapter: &juggler_adapter::SharedAdapter,
) -> Vec<DiscoveredInstall> {
    state
        .places
        .made_up
        .get()
        .and_then(|mac| mac.installs.clone())
        .unwrap_or_else(|| adapter.scan_environment())
}

/// AJ-E2315 (the review's E7): of `installs` (the tool folders discovery
/// found for `adapter`), the one a profile's sessions use, by evidence:
/// the one whose pid files name its sessions running under its own main
/// process ([`ownership::sessions`]), never the folder named like it (a
/// profile its user's own launcher started on another folder had its
/// history moved where it doesn't read). `None` while none of its
/// sessions runs.
pub(super) fn folder_of_its_sessions<'a>(
    state: &Arc<AppState>,
    adapter: &juggler_adapter::SharedAdapter,
    host: &juggler_adapter::live::HostApp,
    place: &KnownPlace,
    installs: &'a [DiscoveredInstall],
) -> Option<&'a DiscoveredInstall> {
    let key = ownership::InstanceKey::of(place.instance_dir.as_deref()?);
    let folders: Vec<_> = installs
        .iter()
        .map(|one| (one.path.clone(), adapter.live_sessions(&one.path)))
        .collect();
    let home = state.places.home();
    let under = ownership::sessions::Under {
        app: host,
        bundle: &place.bundle,
        home: &home,
        instance: &key,
    };
    let table = state.places.table();
    let found = ownership::sessions::folder_of_sessions(&table, &under, &folders)?;
    installs.iter().find(|one| one.path == found)
}

/// Every Account of `harness`: its id and folder.
fn accounts_of(state: &Arc<AppState>, harness: &str) -> Vec<(String, PathBuf)> {
    crate::commands::lock_storage_arc(state)
        .ok()
        .and_then(|storage| storage.accounts().list().ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|account| account.harness.as_str() == harness)
        .map(|account| (account.id.as_str().to_owned(), account.state_dir))
        .collect()
}

/// Gathers what tells which tool folder App `place` uses ([`Look`]):
/// `None` when no adapter knows its app.
fn look(state: &Arc<AppState>, id: &str, place: &KnownPlace) -> Option<Look> {
    let canonical = juggler_platform::real_path::canonical;
    for harness in state.registry.ids() {
        let Some(adapter) = state.registry.get(&harness) else {
            continue;
        };
        let Some(host) = adapter
            .host_apps()
            .into_iter()
            .find(|host| host.id == place.app_id)
        else {
            continue;
        };
        let installs = installs_of(state, &adapter);
        let runs = super::running_now(state, id).is_some();
        let mut look = Look {
            harness: harness.clone(),
            installs: Vec::new(),
            running: None,
            records: None,
            recorded: super::tool_folder(state, place),
            declared: None,
            named: None,
            listed: None,
            tied: false,
            runs,
            tool: adapter.spec().display_name.clone(),
        };
        let Some(dir) = place.instance_dir.as_deref() else {
            // ADR-0032: an app that runs one instance, on the folder it
            // declares.
            look.declared = host.default_folder.as_ref().and_then(|folder| {
                let want = canonical(&state.places.home().join(folder));
                installs
                    .iter()
                    .find(|one| canonical(&one.path) == want)
                    .map(|one| one.path.clone())
            });
            look.installs = installs;
            return Some(look);
        };
        // 1: its running sessions' pid files, under its own main process.
        look.running = runs
            .then(|| folder_of_its_sessions(state, &adapter, &host, place, &installs))
            .flatten()
            .map(|one| one.path.clone());
        // 2: its own records' conversations, as transcripts in one folder
        // (every folder found, and every Account's of its tool).
        let ids: Vec<String> = adapter
            .host_family()
            .and_then(|family| family.conversations(dir))
            .map(|records| records.into_iter().map(|record| record.id).collect())
            .unwrap_or_default();
        look.listed = adapter.host_family().map(|_| ids.len());
        // The folder named like it: a fallback only, and never once its
        // records name conversations (they'd have found it).
        let name = dir.file_name();
        let named = installs
            .iter()
            .find(|one| name.is_some() && one.path.file_name() == name)
            .map(|one| one.path.clone());
        if look.running.is_none() && !ids.is_empty() {
            let mut seen = HashSet::new();
            let folders: Vec<(PathBuf, HashSet<String>)> = installs
                .iter()
                .map(|one| one.path.clone())
                .chain(
                    accounts_of(state, harness.as_str())
                        .into_iter()
                        .map(|(_, dir)| dir),
                )
                .filter(|folder| seen.insert(canonical(folder)))
                .map(|folder| {
                    let held = adapter
                        .conversation_files(&folder)
                        .iter()
                        .filter_map(|path| {
                            path.file_stem().and_then(|s| s.to_str()).map(str::to_owned)
                        })
                        .collect();
                    (folder, held)
                })
                .collect();
            let prefer: Vec<&Path> = look
                .recorded
                .as_deref()
                .into_iter()
                .chain(named.as_deref())
                .collect();
            look.records = ownership::sessions::folder_of_records(&ids, &folders, &prefer)
                .map(Path::to_path_buf);
            let most = folders
                .iter()
                .map(|(_, held)| ids.iter().filter(|id| held.contains(*id)).count())
                .max()
                .unwrap_or(0);
            look.tied = look.records.is_none() && most > 0;
        }
        look.named = named.filter(|_| ids.is_empty());
        look.installs = installs;
        return Some(look);
    }
    None
}

/// Links App `id` to `folder` (`by` told): the Account that holds it, or
/// (with `may_bring_in`, the user's Add) the folder brought in where it
/// is, never copied. Returns whether the link changed, or why it can't
/// be made.
fn link_to(
    state: &Arc<AppState>,
    id: &str,
    place: &KnownPlace,
    look: &Look,
    folder: &Path,
    by: LinkSource,
    may_bring_in: bool,
) -> Result<bool, String> {
    let canonical = juggler_platform::real_path::canonical;
    let held = accounts_of(state, look.harness.as_str())
        .into_iter()
        .find(|(_, dir)| canonical(dir) == canonical(folder))
        .map(|(account, _)| account);
    let account = match held {
        Some(account) => account,
        None if !may_bring_in => {
            return Err(format!("Its {} folder isn't in AI Juggler yet.", look.tool));
        }
        None => {
            let name = place
                .instance_dir
                .as_deref()
                .and_then(Path::file_name)
                .or_else(|| folder.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let origin = look
                .installs
                .iter()
                .find(|one| canonical(&one.path) == canonical(folder))
                .map(|one| one.origin.as_str().to_owned());
            crate::features::account::import::account_import_mode_impl(
                state,
                look.harness.as_str(),
                &folder.to_string_lossy(),
                &name,
                crate::features::account::import::ImportRequest {
                    origin,
                    in_place: true,
                    ..crate::features::account::import::ImportRequest::default()
                },
            )
            .map_err(|why| {
                tracing::warn!(place = id, "the App's folder wasn't brought in: {why}");
                format!(
                    "Its {} folder {} couldn't be brought in: {why}",
                    look.tool,
                    folder_word(folder)
                )
            })?
            .account
            .id
            .as_str()
            .to_owned()
        }
    };
    let changed = with_book(state, |book| {
        book.places.get_mut(id).is_some_and(|place| {
            let new = place.account_id.as_deref() != Some(account.as_str());
            place.account_id = Some(account);
            // D-84, AJ-E1552 (`App::is_users`): a link alone never adds
            // an App, so `users` stays as its row says (added, named or
            // kept alive): the book and the Apps list read one rule, now
            // and after a restart alike.
            new
        })
    })
    .unwrap_or(false);
    if changed && place.account_id.is_some() {
        tracing::info!(
            place = id,
            by = by.word(),
            "an App's folder link corrected by evidence"
        );
    }
    Ok(changed)
}

/// A folder as its row names it: its own name.
fn folder_word(folder: &Path) -> String {
    folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Keeps what linked App `id` (or why nothing did), for its row, and
/// stores the source so it's said after a restart.
fn note(state: &Arc<AppState>, id: &str, note: LinkNote) {
    let word = match &note {
        LinkNote::Linked { by, .. } => by.word(),
        LinkNote::Unlinked(_) => "",
    };
    if let Ok(storage) = crate::commands::lock_storage_arc(state) {
        let key = format!("{SOURCE_KEY}{id}");
        if storage.app_settings().get(&key).ok().flatten().as_deref() != Some(word) {
            let _ = storage.app_settings().set(&key, word);
        }
    }
    if let Ok(mut notes) = state.places.link_notes.lock() {
        notes.insert(id.to_owned(), note);
    }
}

/// AJ-E2327: what App `id`'s row says of its folder: the last look's
/// note, else (after a restart) its stored source with its link, else
/// `None` (nothing looked yet, or an App with no folder to link).
pub(crate) fn link_note(state: &Arc<AppState>, id: &str) -> Option<LinkNote> {
    if let Some(found) = state
        .places
        .link_notes
        .lock()
        .ok()
        .and_then(|notes| notes.get(id).cloned())
    {
        return Some(found);
    }
    let place = with_book(state, |book| book.places.get(id).cloned()).ok()??;
    let folder = super::tool_folder(state, &place)?;
    let by = crate::commands::lock_storage_arc(state)
        .ok()?
        .app_settings()
        .get(&format!("{SOURCE_KEY}{id}"))
        .ok()
        .flatten()
        .and_then(|word| LinkSource::of_word(&word))
        .unwrap_or(LinkSource::Recorded);
    Some(LinkNote::Linked { folder, by })
}

/// Whether a background look at App `id` waits (it found nothing a
/// moment ago); a start, or the user's own Add, looks at once.
fn waits(state: &Arc<AppState>, id: &str, runs: bool) -> bool {
    state.places.link_retry.lock().is_ok_and(|retry| {
        retry
            .get(id)
            .is_some_and(|(at, ran)| *ran == runs && Instant::now() < *at)
    })
}

fn wait_after(state: &Arc<AppState>, id: &str, runs: bool) {
    if let Ok(mut retry) = state.places.link_retry.lock() {
        let gap = if runs { RETRY_RUNNING } else { RETRY_CLOSED };
        retry.insert(id.to_owned(), (Instant::now() + gap, runs));
    }
}

/// The go sitting (2026-11-07, finding 1), AJ-E2327: an added App lists
/// its sessions only once the tool folder they're written in is linked
/// to it. That folder is found by evidence ([`look`], in the module's
/// order), and brought in where it is (in place, never copied) unless an
/// Account holds it already; either way it's the App's folder from then
/// on. That link is what tells the folder's tool sign-in apart (the
/// profile signs in inside its app, never in a terminal) and gives the
/// App page its Account. A new folder is brought in only with the user's
/// consent: `may_bring_in` (Add's yes), or an App the user added (D-84:
/// their Add was it). Linked by a recalled or guessed folder, it's looked
/// at again (throttled), so evidence confirms or corrects it. A failure
/// never fails the add. Returns whether the link changed.
pub(super) fn link_its_folder(state: &Arc<AppState>, id: &str, may_bring_in: bool) -> bool {
    link_by_evidence(state, id, may_bring_in).0
}

/// [`link_its_folder`], with what it linked or why not ([`Linked`]).
pub(super) fn link_by_evidence(
    state: &Arc<AppState>,
    id: &str,
    may_bring_in: bool,
) -> (bool, Linked) {
    let Some(place) = with_book(state, |book| book.places.get(id).cloned())
        .ok()
        .flatten()
    else {
        return (false, Linked::default());
    };
    let runs = super::running_now(state, id).is_some();
    if !may_bring_in && waits(state, id, runs) {
        // Looked at again on a later read, once the wait is over.
        forget_looked(state, id);
        return (false, current(state, id));
    }
    let Some(look) = look(state, id, &place) else {
        return (false, Linked::default());
    };
    let consent = may_bring_in || place.users;
    let found = ownership::sessions::decide(&look.evidence());
    let Some((folder, by)) = found else {
        let why = look.why_none();
        note(state, id, LinkNote::Unlinked(why.clone()));
        wait_after(state, id, runs);
        forget_looked(state, id);
        return (
            false,
            Linked {
                why: Some(why),
                ..Linked::default()
            },
        );
    };
    let folder = folder.to_path_buf();
    match link_to(state, id, &place, &look, &folder, by, consent) {
        Ok(changed) => {
            note(
                state,
                id,
                LinkNote::Linked {
                    folder: folder.clone(),
                    by,
                },
            );
            // Only proof settles it; a recalled or guessed folder is
            // looked at again.
            if !by.is_evidence() {
                wait_after(state, id, runs);
                forget_looked(state, id);
            }
            (
                changed,
                Linked {
                    folder: Some(folder),
                    by: Some(by),
                    why: None,
                },
            )
        }
        Err(why) => {
            note(state, id, LinkNote::Unlinked(why.clone()));
            wait_after(state, id, runs);
            forget_looked(state, id);
            (
                false,
                Linked {
                    why: Some(why),
                    ..Linked::default()
                },
            )
        }
    }
}

/// Whether App `id`'s link is settled by proof (its running sessions,
/// its records, its app's declared folder): else a read looks again.
pub(super) fn proven(state: &Arc<AppState>, id: &str) -> bool {
    matches!(
        link_note(state, id),
        Some(LinkNote::Linked { by, .. }) if by.is_evidence()
    )
}

/// What App `id` is linked to now, as [`Linked`].
fn current(state: &Arc<AppState>, id: &str) -> Linked {
    match link_note(state, id) {
        Some(LinkNote::Linked { folder, by }) => Linked {
            folder: Some(folder),
            by: Some(by),
            why: None,
        },
        Some(LinkNote::Unlinked(why)) => Linked {
            why: Some(why),
            ..Linked::default()
        },
        None => Linked::default(),
    }
}

/// A later read looks for App `id`'s folder again.
fn forget_looked(state: &Arc<AppState>, id: &str) {
    if let Ok(mut looked) = state.places.looked.lock() {
        looked.remove(id);
    }
}

/// AJ-E2327, "Choose its folder…": the user picked `folder` as App
/// `id`'s tool folder (no evidence told). It's brought in where it is,
/// unless an Account holds it, and recorded as AI Juggler's own: the
/// folder it's started with from now on, until its own running sessions
/// or records say otherwise.
///
/// # Errors
///
/// Why it can't be its folder, in words.
pub(crate) fn choose_folder(
    state: &Arc<AppState>,
    id: &str,
    folder: &Path,
) -> Result<Linked, String> {
    let place = with_book(state, |book| book.places.get(id).cloned())
        .ok()
        .flatten()
        .filter(|place| place.users)
        .ok_or_else(|| super::NOT_ADDED.to_owned())?;
    let look = look(state, id, &place)
        .ok_or_else(|| "AI Juggler doesn't know that app's tool".to_owned())?;
    let adapter = state
        .registry
        .get(&look.harness)
        .ok_or_else(|| "AI Juggler doesn't know that app's tool".to_owned())?;
    let inspect = adapter
        .inspect_source(folder)
        .map_err(|why| format!("That folder can't be used: {why}"))?;
    if !inspect.is_config {
        return Err(format!(
            "That folder isn't a {} folder (no settings, sign-in or sessions in it).",
            look.tool
        ));
    }
    // Proof it uses another folder wins over a pick.
    if let Some((other, _)) = ownership::sessions::contradicts(&look.evidence(), folder) {
        return Err(format!(
            "Its sessions are in {}, not that folder: AI Juggler links that one.",
            folder_word(other)
        ));
    }
    link_to(state, id, &place, &look, folder, LinkSource::Recorded, true)?;
    let linked = LinkNote::Linked {
        folder: folder.to_path_buf(),
        by: LinkSource::Recorded,
    };
    note(state, id, linked);
    if let Ok(mut retry) = state.places.link_retry.lock() {
        retry.remove(id);
    }
    super::read_records(state);
    super::changed(state);
    Ok(Linked {
        folder: Some(folder.to_path_buf()),
        by: Some(LinkSource::Recorded),
        why: None,
    })
}

/// AJ-E2327, for AJ-E2317's Move-time refusal: when App `id`'s own
/// evidence (its running sessions, its records, its app's declared
/// folder) says it uses another folder than the one it's linked to, that
/// folder and what says so. `None` when it agrees, nothing proves
/// otherwise, or it isn't linked. Reads the folders and the process
/// table: call it off the window's thread.
#[must_use]
pub(crate) fn folder_mismatch(state: &Arc<AppState>, id: &str) -> Option<(PathBuf, LinkSource)> {
    let place = with_book(state, |book| book.places.get(id).cloned()).ok()??;
    let linked = super::tool_folder(state, &place)?;
    let look = look(state, id, &place)?;
    ownership::sessions::contradicts(&look.evidence(), &linked)
        .map(|(folder, by)| (folder.to_path_buf(), by))
}

/// AJ-E2327: the folder evidence pairs with profile `place` (not yet one
/// of the user's Apps: first run's "Your apps"), what told, and whether
/// it's certain (proof, not a recalled or guessed folder). Nothing is
/// brought in or linked.
#[must_use]
pub(crate) fn pairing(state: &Arc<AppState>, id: &str) -> Option<(PathBuf, LinkSource, bool)> {
    let place = with_book(state, |book| book.places.get(id).cloned())
        .ok()
        .flatten()?;
    let look = look(state, id, &place)?;
    let (folder, by) = ownership::sessions::decide(&look.evidence())?;
    Some((folder.to_path_buf(), by, by.is_evidence()))
}

/// 0.4.3 (D-102), AJ-E2327: a linked profile's link is checked once a run
/// against its evidence: its own records' sessions held in another folder
/// (or its running sessions writing in one) relink it, by the same order
/// as every look ([`link_by_evidence`]). The test profile was linked to
/// the default folder by sessions it once ran there, while its sessions
/// now run in its own folder: it wore the default Account's mark and
/// listed none of its 23. Returns whether the link changed.
pub(super) fn correct_by_records(state: &Arc<AppState>, id: &str) -> bool {
    link_by_evidence(state, id, false).0
}
