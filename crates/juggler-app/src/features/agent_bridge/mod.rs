//! AJ-E1704 (ADR-0020, D-28, D-30, D-33): the agent bridge's app side.
//!
//! - **Identity.** Each launch AI Juggler starts with the bridge on gets
//!   a fresh random key (32 bytes), in the launch's environment only
//!   ([`prepare`]); a switch or a resume is a new launch, so a new key.
//!   The app keeps only each key's SHA-256 and its session (in memory,
//!   and in a `0600` file in its own `0700` folder so sessions that
//!   outlive a restart keep their tools). It never reads another
//!   process's environment (D-37): it wrote the key into the launch.
//! - **Injection** (D-30). On only after the user said yes once, and not
//!   for a project or profile they switched off ([`settings`]). What a
//!   tool takes is its adapter's ([`juggler_adapter::HarnessAdapter::agent_bridge`]);
//!   the files go in AI Juggler's overlay folder, never the user's setup.
//! - **Calls** ([`calls`]): over a `0600` socket in a `0700` folder of AI
//!   Juggler's own ([`server`]), only from this user, each through
//!   `allowed(pool, action)` and so audited; read-only, plus the agent's
//!   own status line.
//!
//! Nothing here starts, stops or types into a session.

pub mod calls;
pub mod commands;
pub mod dto;
pub mod server;
pub mod settings;

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use juggler_adapter::{BridgeLaunch, HarnessAdapter, LaunchSpec};
use juggler_agentbridge::wire;
use juggler_core::model::SessionId;
use sha2::{Digest, Sha256};

use crate::state::AppState;

/// The bridge's folder in the app's data folder: the socket and the
/// keys' hashes (`0700`).
pub const DIR: &str = "agent";
/// The socket's name in [`DIR`].
pub const SOCKET: &str = "bridge.sock";
/// The keys' hashes, by session.
const EPOCHS: &str = "epochs.json";
/// Each launch's files: `overlays/agent/<session>/` (ADR-0018's folder).
pub const OVERLAY: &str = "agent";
/// A socket path must fit `sun_path` (104 bytes on macOS).
const MAX_SOCKET_PATH: usize = 100;
/// A key younger than this is kept even before its session is recorded
/// (the record is written just after the spawn).
const YOUNG_MS: i64 = 60_000;

/// A status line an agent set, and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    /// The line (one line, at most 140 characters).
    pub text: String,
    /// When, epoch ms.
    pub at: i64,
}

/// One key's session.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Epoch {
    session: String,
    minted_at: i64,
    /// What the key may do: empty for the agent tools, [`REPORTS`] for a
    /// tool's hooks reporting its session's state (AJ-E2306). A key of one
    /// never works for the other.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    scope: String,
}

/// The scope of a report key (AJ-E2306).
pub const REPORTS: &str = "reports";

/// The bridge's state in the app: the keys' hashes, the agents' status
/// lines, the call counts, and the socket once it's served.
#[derive(Debug, Default)]
pub struct Bridge {
    /// SHA-256 of each key (hex) → its session. `None` until first read.
    epochs: Mutex<Option<HashMap<String, Epoch>>>,
    /// Each member's status line, by member (`work:…` or `session:…`).
    statuses: Mutex<HashMap<String, Said>>,
    /// Each session's recent calls (the rate limit).
    pub(crate) calls: Mutex<HashMap<String, VecDeque<Instant>>>,
    /// The socket, once served.
    pub(crate) serving: Mutex<Option<PathBuf>>,
    /// The program, when set (tests); else beside AI Juggler's own.
    program: Mutex<Option<PathBuf>>,
}

/// The socket for the app's data folder (per edition: each edition has
/// its own), or `None` when the path wouldn't fit a socket address.
#[must_use]
pub fn socket_path(data_dir: &Path) -> Option<PathBuf> {
    let path = data_dir.join(DIR).join(SOCKET);
    (path.as_os_str().len() < MAX_SOCKET_PATH).then_some(path)
}

/// Bytes as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// The SHA-256 of a key, hex.
#[must_use]
pub fn key_hash(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

impl Bridge {
    /// Uses `program` as the bridge (tests; `None` goes back to the one
    /// beside AI Juggler).
    pub fn set_program(&self, program: Option<PathBuf>) {
        if let Ok(mut slot) = self.program.lock() {
            *slot = program;
        }
    }

    /// The bridge program: beside AI Juggler's own (`Contents/MacOS` in
    /// the app), when it's there and safe to run (ours or root's, and no
    /// one else can write it or a folder above it).
    #[must_use]
    pub fn program(&self) -> Option<PathBuf> {
        let chosen = self.program.lock().ok().and_then(|slot| slot.clone());
        let path = chosen.or_else(|| {
            std::env::current_exe()
                .ok()?
                .parent()
                .map(|dir| dir.join(wire::PROGRAM))
        })?;
        juggler_adapter::file_trust::trusted_executable(&path, juggler_adapter::file_trust::me())
    }

    /// The status line a member set, if it did.
    #[must_use]
    pub fn status_of(&self, member: &str) -> Option<Said> {
        self.statuses.lock().ok()?.get(member).cloned()
    }

    /// Keeps a member's status line (empty clears it).
    pub(crate) fn set_status(&self, member: String, said: Option<Said>) {
        if let Ok(mut statuses) = self.statuses.lock() {
            match said {
                Some(said) => statuses.insert(member, said),
                None => statuses.remove(&member),
            };
        }
    }

    /// The session an agent tools' key stands for, if it is current.
    #[must_use]
    pub fn session_of(&self, data_dir: &Path, token: &str) -> Option<SessionId> {
        self.session_of_scope(data_dir, token, "")
    }

    /// The session a report key stands for, if it is current (AJ-E2306).
    #[must_use]
    pub fn session_of_reports(&self, data_dir: &Path, token: &str) -> Option<SessionId> {
        self.session_of_scope(data_dir, token, REPORTS)
    }

    fn session_of_scope(&self, data_dir: &Path, token: &str, scope: &str) -> Option<SessionId> {
        let mut epochs = self.epochs.lock().ok()?;
        let epochs = epochs.get_or_insert_with(|| load_epochs(data_dir));
        epochs
            .get(&key_hash(token))
            .filter(|epoch| epoch.scope == scope)
            .map(|epoch| SessionId::new(epoch.session.as_str()))
    }

    /// A new key for `session`: every older key of that session stops
    /// working, and so does every key of a session `ended` says is over
    /// (unless it is too young to have been recorded yet).
    fn mint(
        &self,
        data_dir: &Path,
        session: &SessionId,
        ended: &dyn Fn(&str) -> bool,
    ) -> Option<String> {
        self.mint_scoped(data_dir, session, ended, "")
    }

    /// A new report key for `session` (AJ-E2306): as [`Self::mint`], for
    /// the report scope only.
    pub(crate) fn mint_reports(
        &self,
        data_dir: &Path,
        session: &SessionId,
        ended: &dyn Fn(&str) -> bool,
    ) -> Option<String> {
        self.mint_scoped(data_dir, session, ended, REPORTS)
    }

    fn mint_scoped(
        &self,
        data_dir: &Path,
        session: &SessionId,
        ended: &dyn Fn(&str) -> bool,
        scope: &str,
    ) -> Option<String> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).ok()?;
        let token = hex(&bytes);
        let now = juggler_core::model::Timestamp::now().millis();
        let mut epochs = self.epochs.lock().ok()?;
        let epochs = epochs.get_or_insert_with(|| load_epochs(data_dir));
        epochs.retain(|_, epoch| {
            !(epoch.session == session.as_str() && epoch.scope == scope)
                && (now - epoch.minted_at < YOUNG_MS || !ended(&epoch.session))
        });
        epochs.insert(
            key_hash(&token),
            Epoch {
                session: session.as_str().to_owned(),
                minted_at: now,
                scope: scope.to_owned(),
            },
        );
        save_epochs(data_dir, epochs).ok()?;
        Some(token)
    }

    /// Whether `session` holds a key for the agent tools now (E18: only
    /// then is it told about its messages).
    #[must_use]
    pub fn has_tools_key(&self, data_dir: &Path, session: &str) -> bool {
        let Ok(mut epochs) = self.epochs.lock() else {
            return false;
        };
        let epochs = epochs.get_or_insert_with(|| load_epochs(data_dir));
        epochs
            .values()
            .any(|epoch| epoch.session == session && epoch.scope.is_empty())
    }

    /// The sessions that hold a key now.
    #[must_use]
    pub fn sessions(&self, data_dir: &Path) -> Vec<String> {
        let Ok(mut epochs) = self.epochs.lock() else {
            return Vec::new();
        };
        let epochs = epochs.get_or_insert_with(|| load_epochs(data_dir));
        epochs.values().map(|epoch| epoch.session.clone()).collect()
    }
}

/// The bridge's own folder: made `0700`; an existing one must be ours and
/// closed (never chmod'ed into shape).
///
/// # Errors
///
/// When it can't be made, or is someone else's or open.
pub fn own_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    match std::fs::DirBuilder::new()
        .mode(0o700)
        .recursive(true)
        .create(dir)
    {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    if juggler_adapter::file_trust::dir_is_ours(dir, juggler_adapter::file_trust::me()) {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "{} isn't AI Juggler's own",
            dir.display()
        )))
    }
}

/// Writes `contents` to `path`, readable by this user only.
///
/// # Errors
///
/// The write's error.
pub(crate) fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let partial = path.with_extension("partial");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&partial)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    std::fs::rename(&partial, path)
}

fn load_epochs(data_dir: &Path) -> HashMap<String, Epoch> {
    std::fs::read_to_string(data_dir.join(DIR).join(EPOCHS))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_epochs(data_dir: &Path, epochs: &HashMap<String, Epoch>) -> std::io::Result<()> {
    let dir = data_dir.join(DIR);
    own_dir(&dir)?;
    let text = serde_json::to_string(epochs).map_err(std::io::Error::other)?;
    write_private(&dir.join(EPOCHS), &text)
}

/// What [`prepare`] did to a launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prepared {
    /// The tool gets the MCP server, pre-allowed, and the command line.
    Added,
    /// The tool can't take an MCP server by its launch options: it gets
    /// the command line (the key, the socket and the program in its
    /// environment).
    CommandLine,
    /// Nothing added, and why.
    Not(&'static str),
}

/// Adds the bridge to a launch AI Juggler is about to start, when the
/// user has it on for this project and profile: a new key in the
/// launch's environment (never its arguments), the socket served, and
/// the tool's own options and files from its adapter. Never blocks a
/// launch: anything that fails leaves it as it was.
pub fn prepare(
    state: &Arc<AppState>,
    adapter: &dyn HarnessAdapter,
    spec: &mut LaunchSpec,
    session: &SessionId,
    project: Option<&str>,
    account: &str,
) -> Prepared {
    if let Err(why) = settings::enabled_for(state, project, account) {
        return Prepared::Not(why);
    }
    let Some(program) = state.agent_bridge.program() else {
        return Prepared::Not("the agent tools aren't in this build");
    };
    let Ok(socket) = server::ensure_serving(state) else {
        return Prepared::Not("the agent tools' socket couldn't be opened");
    };
    let dir = state.overlays_dir().join(OVERLAY).join(session.as_str());
    let bridge = BridgeLaunch {
        program: program.clone(),
        server: wire::SERVER_NAME.to_owned(),
        tools: wire::TOOLS.iter().map(|tool| (*tool).to_owned()).collect(),
        env_names: vec![wire::TOKEN_ENV.to_owned(), wire::SOCKET_ENV.to_owned()],
        dir: dir.clone(),
    };
    let injection = adapter.agent_bridge(&bridge, &spec.args);
    if let Some(injection) = &injection {
        let written = own_dir(&state.overlays_dir().join(OVERLAY))
            .and_then(|()| own_dir(&dir))
            .and_then(|()| {
                injection.files.iter().try_for_each(|file| {
                    if file.name.contains(['/', '\\']) || file.name.starts_with('.') {
                        return Err(std::io::Error::other("a file name with a folder"));
                    }
                    write_private(&dir.join(&file.name), &file.contents)
                })
            });
        if written.is_err() {
            return Prepared::Not("the launch's files couldn't be written");
        }
    }
    let ended = |id: &str| session_ended(state, id);
    let Some(token) = state.agent_bridge.mint(&state.data_dir, session, &ended) else {
        return Prepared::Not("no key could be made");
    };
    prune_overlays(state);
    spec.env.insert(wire::TOKEN_ENV.to_owned(), token);
    spec.env.insert(
        wire::SOCKET_ENV.to_owned(),
        socket.to_string_lossy().into_owned(),
    );
    spec.env.insert(
        wire::PROGRAM_ENV.to_owned(),
        program.to_string_lossy().into_owned(),
    );
    match injection {
        Some(injection) => {
            spec.args = injection.applied(&spec.args);
            Prepared::Added
        }
        None => Prepared::CommandLine,
    }
}

/// At the app's start: serves the socket when the tools are on and
/// sessions that outlived the last run still hold keys; otherwise the
/// first launch that gets the bridge serves it.
pub fn resume(state: &Arc<AppState>) {
    let on = state
        .storage
        .lock()
        .is_ok_and(|storage| settings::read(&storage).on == Some(true));
    if on && !state.agent_bridge.sessions(&state.data_dir).is_empty() {
        let _ = server::ensure_serving(state);
    }
}

/// Whether a session is over (or unknown): its key no longer works.
pub(crate) fn session_ended(state: &Arc<AppState>, session: &str) -> bool {
    use juggler_core::model::SessionStatus;
    let Ok(storage) = state.storage.lock() else {
        return false;
    };
    storage
        .sessions()
        .get(&SessionId::new(session))
        .map_or(true, |record| {
            !matches!(
                record.status,
                SessionStatus::Starting
                    | SessionStatus::Running
                    | SessionStatus::WaitingUser
                    | SessionStatus::Stuck
            )
        })
}

/// Removes the launch folders of sessions that no longer hold a key.
fn prune_overlays(state: &Arc<AppState>) {
    let root = state.overlays_dir().join(OVERLAY);
    let keep: std::collections::HashSet<String> = state
        .agent_bridge
        .sessions(&state.data_dir)
        .into_iter()
        .collect();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if is_dir && !keep.contains(&name) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bridge, key_hash, socket_path};
    use juggler_core::model::SessionId;
    use std::path::Path;

    #[test]
    fn a_report_key_and_a_tools_key_never_stand_in_for_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let bridge = Bridge::default();
        let session = SessionId::new("sess-1");
        let never = |_: &str| false;
        let tools = bridge.mint(dir.path(), &session, &never).unwrap();
        let reports = bridge.mint_reports(dir.path(), &session, &never).unwrap();
        assert_ne!(tools, reports);
        assert_eq!(bridge.session_of(dir.path(), &tools), Some(session.clone()));
        assert_eq!(
            bridge.session_of(dir.path(), &reports),
            None,
            "reports only"
        );
        assert_eq!(
            bridge.session_of_reports(dir.path(), &reports),
            Some(session.clone())
        );
        assert_eq!(
            bridge.session_of_reports(dir.path(), &tools),
            None,
            "tools only"
        );
        // A new key of one scope leaves the other's working.
        let tools_again = bridge.mint(dir.path(), &session, &never).unwrap();
        assert_eq!(
            bridge.session_of(dir.path(), &tools),
            None,
            "the old one stops"
        );
        assert_eq!(
            bridge.session_of(dir.path(), &tools_again),
            Some(session.clone())
        );
        assert_eq!(
            bridge.session_of_reports(dir.path(), &reports),
            Some(session)
        );
    }

    #[test]
    fn the_socket_is_per_data_folder_and_fits() {
        let path = socket_path(Path::new(
            "/Users/me/Library/Application Support/ai.figurehead.juggler",
        ))
        .unwrap();
        assert!(path.ends_with("agent/bridge.sock"));
        assert!(socket_path(&Path::new("/x").join("y".repeat(120))).is_none());
    }

    #[test]
    fn a_key_is_kept_only_as_its_hash() {
        let hash = key_hash("secret");
        assert_eq!(hash.len(), 64);
        assert!(!hash.contains("secret"));
    }
}
