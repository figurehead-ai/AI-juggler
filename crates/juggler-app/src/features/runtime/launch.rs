//! Starting a session in its runtime (ADR-0022 decision 1): driven when
//! the plan says so and the tool's protocol answers, else the terminal,
//! with the reason. Both are owned by the supervisor (ADR-0010): a driven
//! session's process is a ledger child, piped or serving a socket in AI
//! Juggler's `runtime/` folder (`0700`; the socket made `0600`).

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::runtime::{Connection, DrivenLaunch, DrivenRuntime, Runtime, Transport};
use juggler_adapter::{HarnessAdapter, LaunchSpec};
use juggler_core::model::{Profile, SessionId};
use sha2::{Digest, Sha256};

use super::driver::{Driver, Meta};
use super::settings;
use crate::error::AppError;
use crate::features::sessions::spawn::{Io, NewSession, Started, Unstarted, spawn_process};
use crate::state::AppState;

/// The runtime folder in the app's data folder (`0700`).
pub const DIR: &str = "runtime";
/// A socket path must fit `sun_path` (104 bytes on macOS).
const MAX_SOCKET_PATH: usize = 100;
/// How long a server has to make its socket.
const SOCKET_WAIT: Duration = Duration::from_secs(5);

/// What a launch asks of its runtime (its account, project and folder
/// are its row's: [`NewSession`]).
pub struct Plan<'a> {
    /// Its launch settings.
    pub profile: &'a Profile,
    /// The tool's conversation to continue, when resuming.
    pub resume: Option<String>,
    /// The user's choice for this session, when they made one.
    pub requested: Option<Runtime>,
}

/// The socket for `session`: short, in the runtime folder, or `None` when
/// the data folder's path is too long for one.
#[must_use]
pub fn socket_path(data_dir: &Path, session: &SessionId) -> Option<PathBuf> {
    let digest = Sha256::digest(session.as_str().as_bytes());
    let name = hex(&digest[..6]);
    let path = data_dir.join(DIR).join(format!("{name}.sock"));
    (path.as_os_str().len() < MAX_SOCKET_PATH).then_some(path)
}

/// Bytes as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// A random UUID (v4), for a tool that lets the caller name a new
/// conversation.
fn fresh_uuid() -> String {
    let mut bytes = [0_u8; 16];
    let _ = getrandom::fill(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = hex(&bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// Makes a socket the tool created ours alone: a socket (never a link)
/// owned by this user, then `0600`.
fn harden(socket: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    let meta = std::fs::symlink_metadata(socket)?;
    if !meta.file_type().is_socket() || meta.uid() != juggler_adapter::file_trust::me() {
        return Err(std::io::Error::other("it isn't the session's own socket"));
    }
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
}

/// Removes a socket left from before (or after a driven session ended),
/// only when it is a socket of ours.
pub(super) fn clear_stale(socket: &Path) {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    if let Ok(meta) = std::fs::symlink_metadata(socket)
        && meta.file_type().is_socket()
        && meta.uid() == juggler_adapter::file_trust::me()
    {
        let _ = std::fs::remove_file(socket);
    }
}

/// Why a driven start didn't happen.
enum NotDriven {
    /// Nothing was spawned: the terminal starts under the same id.
    Before(String),
    /// A process was spawned and stopped: its id is spent.
    After(String),
}

fn connect_socket(
    state: &Arc<AppState>,
    session: &SessionId,
    socket: &Path,
) -> Result<Connection, String> {
    let deadline = Instant::now() + SOCKET_WAIT;
    while !socket.exists() {
        if Instant::now() > deadline || state.supervisor.is_finished(session) {
            return Err("its server didn't open its socket".into());
        }
        // poll: the server makes its socket within SOCKET_WAIT (5 s), or
        // the start falls back to the terminal.
        std::thread::sleep(Duration::from_millis(20));
    }
    harden(socket).map_err(|e| e.to_string())?;
    let stream = std::os::unix::net::UnixStream::connect(socket).map_err(|e| e.to_string())?;
    let reader = stream.try_clone().map_err(|e| e.to_string())?;
    Ok(Connection {
        reader: Box::new(BufReader::new(reader)),
        writer: Box::new(stream),
    })
}

fn try_driven(
    state: &Arc<AppState>,
    adapter: &dyn HarnessAdapter,
    driven: &dyn DrivenRuntime,
    row: &NewSession<'_>,
    plan: &Plan<'_>,
    cwd: &Path,
    trust: Option<bool>,
) -> Result<Started, NotDriven> {
    let session = row.id;
    let socket = socket_path(&state.data_dir, session).ok_or_else(|| {
        NotDriven::Before("the data folder's path is too long for a socket".into())
    })?;
    let dir = socket.parent().unwrap_or(&state.data_dir);
    crate::features::agent_bridge::own_dir(dir).map_err(|e| NotDriven::Before(e.to_string()))?;
    clear_stale(&socket);
    let launch = DrivenLaunch {
        socket: socket.clone(),
        resume: plan.resume.clone(),
        fresh_id: fresh_uuid(),
    };
    let mut spec = driven
        .driven_spec(row.account, plan.profile, cwd, &launch)
        .map_err(|e| NotDriven::Before(e.to_string()))?;
    // Lab 2: the folder's trust, in memory (never the tool's own config).
    if let Some(trusted) = trust {
        adapter.apply_folder_trust(&mut spec.launch.args, cwd, trusted);
    }
    // The one spawn path: the bridge and the account's key go into this
    // launch, under the supervisor.
    let io = match &spec.transport {
        Transport::Socket(_) => Io::Owned,
        Transport::Stdio => Io::Driver,
    };
    let (spawned, pipes) = spawn_process(state, adapter, row, session, &mut spec.launch, io, false)
        .map_err(|e| NotDriven::Before(e.to_string()))?;
    let io = match (&spec.transport, pipes) {
        (Transport::Socket(path), _) => connect_socket(state, session, path),
        (Transport::Stdio, Some(pipes)) => Ok(Connection {
            reader: Box::new(BufReader::new(pipes.stdout)),
            writer: Box::new(pipes.stdin),
        }),
        (Transport::Stdio, None) => Err("the session's pipes couldn't be opened".to_owned()),
    };
    let connected = io.and_then(|io| {
        driven
            .connect(io, &spec, row.account)
            .map_err(|e| e.to_string())
    });
    let session_box = match connected {
        Ok(session_box) => session_box,
        Err(why) => {
            let _ = state.supervisor.stop(session, Duration::from_secs(2));
            clear_stale(&socket);
            return Err(NotDriven::After(why));
        }
    };
    let socket = matches!(spec.transport, Transport::Socket(_)).then_some(socket);
    let held = Driver::spawn(
        state,
        session.clone(),
        session_box,
        Meta {
            label: driven.label(),
            beta: driven.maturity() == juggler_adapter::runtime::Maturity::Beta,
            socket,
            can_steer: driven.steers(),
        },
    );
    state.runtime.insert(held);
    Ok(Started {
        session: session.clone(),
        spawned,
        runtime: Runtime::Driven,
    })
}

/// Starts `row`'s session in the runtime its plan picks; the terminal
/// launch `terminal` is the fallback (and what runs for a terminal plan).
/// Called by the one path,
/// [`crate::features::sessions::spawn::start_session`], which wrote the
/// row first. A driven start that fails after its process ran leaves that
/// id spent: the terminal's own row is written before it starts.
///
/// # Errors
///
/// The terminal launch's own spawn errors, with the session whose row
/// they end.
pub(crate) fn start(
    state: &Arc<AppState>,
    adapter: &dyn HarnessAdapter,
    row: &NewSession<'_>,
    plan: &Plan<'_>,
    mut terminal: LaunchSpec,
) -> Result<Started, Unstarted> {
    let (profile_choice, beta, trust) = {
        let storage = state
            .storage
            .lock()
            .map_err(|_| Unstarted::of(row.id, AppError::StatePoisoned))?;
        (
            settings::profile_choices(&storage)
                .get(plan.profile.id.as_str())
                .copied(),
            settings::beta_on(&storage),
            super::trust::plan(&storage, adapter, row.account, &terminal.cwd),
        )
    };
    if let Some(trusted) = trust {
        adapter.apply_folder_trust(&mut terminal.args, &terminal.cwd, trusted);
    }
    let mut session = row.id.clone();
    let fell_back = match settings::plan(adapter.driven(), plan.requested, profile_choice, beta) {
        settings::Plan::Terminal(why) => why,
        settings::Plan::Driven => {
            let driven = adapter.driven().ok_or_else(|| {
                Unstarted::of(row.id, AppError::Refused("no driven runtime".into()))
            })?;
            match try_driven(state, adapter, driven, row, plan, &terminal.cwd, trust) {
                Ok(started) => return Ok(started),
                Err(NotDriven::Before(why)) => Some(why),
                Err(NotDriven::After(why)) => {
                    session = SessionId::new(format!("{}-t", session.as_str()));
                    spent(state, row, &session, &why)?;
                    Some(why)
                }
            }
        }
    };
    let row = NewSession {
        id: &session,
        ..row.clone()
    };
    // AJ-E2306: the tool's own hooks report its exact state (`true`).
    let (spawned, _) = spawn_process(
        state,
        adapter,
        &row,
        &session,
        &mut terminal,
        Io::Owned,
        true,
    )
    .map_err(|error| Unstarted::of(&session, error))?;
    if let Some(why) = fell_back {
        state.runtime.fell_back(&session, why);
    }
    Ok(Started {
        session,
        spawned,
        runtime: Runtime::Terminal,
    })
}

/// A driven start failed after its process ran (and was stopped): its id
/// is spent. Its history says why, and the terminal's row, `next`, is
/// written before the terminal starts (intent before spawn).
fn spent(
    state: &Arc<AppState>,
    row: &NewSession<'_>,
    next: &SessionId,
    why: &str,
) -> Result<(), Unstarted> {
    if let Ok(storage) = state.storage.lock() {
        let _ = storage.sessions().record_lifecycle(
            row.id,
            &format!("its driven start didn't answer ({why}): it started in its terminal"),
        );
    }
    let next_row = NewSession {
        id: next,
        ..row.clone()
    };
    crate::features::sessions::spawn::write_starting(
        state,
        &next_row,
        juggler_core::model::InstanceKind::Terminal,
    )
    .map_err(|error| Unstarted::of(row.id, error))
}

#[cfg(test)]
mod tests {
    use super::{fresh_uuid, socket_path};
    use juggler_core::model::SessionId;
    use std::path::Path;

    #[test]
    fn sockets_are_short_and_per_session() {
        let dir = Path::new("/Users/someone/Library/Application Support/AI Juggler Dev");
        let a = socket_path(dir, &SessionId::new("sess-1")).unwrap();
        let b = socket_path(dir, &SessionId::new("sess-2")).unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with(dir.join("runtime")));
        assert!(a.as_os_str().len() < 100);
        let long = Path::new("/").join("x".repeat(120));
        assert_eq!(socket_path(&long, &SessionId::new("s")), None);
    }

    #[test]
    fn fresh_ids_are_v4_uuids() {
        let id = fresh_uuid();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert_ne!(id, fresh_uuid());
    }
}
