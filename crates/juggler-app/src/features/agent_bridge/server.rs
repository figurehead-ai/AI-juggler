//! The bridge's socket (SECURITY-GUIDELINES C7, ADR-0020): a Unix socket,
//! never a TCP or HTTP listener.
//!
//! - It sits in AI Juggler's own data folder, per edition, in a `0700`
//!   folder of ours (`agent/`), and is made `0600`.
//! - Every connection's peer user is checked (`getpeereid`, through
//!   tokio's `peer_cred`); anything that isn't this user is dropped
//!   unanswered. Then the key decides who is calling.
//! - One request per connection, at most [`wire::MAX_REQUEST`] bytes,
//!   read within [`READ_WAIT`]; at most [`MAX_OPEN`] at once.
//! - A stale socket is removed only when `lstat` shows it is a socket we
//!   own; anything else at the path is refused, never followed.
//!
//! Served lazily: the first launch that gets the bridge starts it, or
//! the app's start when sessions still hold keys.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use juggler_agentbridge::wire;

use crate::state::AppState;

/// How long a caller has to send its request.
pub const READ_WAIT: Duration = Duration::from_secs(2);
/// How many calls are answered at once.
pub const MAX_OPEN: usize = 16;

/// Serves the socket if it isn't served yet; its path.
///
/// # Errors
///
/// When the path doesn't fit, the folder or a file at the path isn't
/// ours, or the socket can't be made.
pub fn ensure_serving(state: &Arc<AppState>) -> std::io::Result<PathBuf> {
    let mut serving = state
        .agent_bridge
        .serving
        .lock()
        .map_err(|_| std::io::Error::other("the bridge's lock"))?;
    if let Some(path) = serving.as_ref() {
        return Ok(path.clone());
    }
    let socket = super::socket_path(&state.data_dir)
        .ok_or_else(|| std::io::Error::other("the data folder's path is too long for a socket"))?;
    let listener = bind(&socket)?;
    let weak = Arc::downgrade(state);
    tauri::async_runtime::spawn(async move {
        let Ok(listener) = tokio::net::UnixListener::from_std(listener) else {
            return;
        };
        let open = Arc::new(tokio::sync::Semaphore::new(MAX_OPEN));
        let me = juggler_adapter::file_trust::me();
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            let Some(state) = weak.upgrade() else {
                return; // the app is gone
            };
            let Ok(permit) = Arc::clone(&open).try_acquire_owned() else {
                continue; // too many at once: dropped, the caller retries
            };
            tauri::async_runtime::spawn(async move {
                answer(stream, me, state).await;
                drop(permit);
            });
        }
    });
    *serving = Some(socket.clone());
    Ok(socket)
}

/// Makes the socket: our `0700` folder, a stale socket of ours removed,
/// bound, then `0600`.
fn bind(socket: &Path) -> std::io::Result<std::os::unix::net::UnixListener> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    let dir = socket
        .parent()
        .ok_or_else(|| std::io::Error::other("no folder"))?;
    super::own_dir(dir)?;
    let me = juggler_adapter::file_trust::me();
    match std::fs::symlink_metadata(socket) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
        Ok(meta) if meta.file_type().is_socket() && meta.uid() == me => {
            std::fs::remove_file(socket)?;
        }
        Ok(_) => {
            return Err(std::io::Error::other(format!(
                "{} isn't AI Juggler's socket",
                socket.display()
            )));
        }
    }
    let listener = std::os::unix::net::UnixListener::bind(socket)?;
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

/// One connection: only this user, one request, one answer.
async fn answer(mut stream: tokio::net::UnixStream, me: u32, state: Arc<AppState>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    if stream.peer_cred().ok().map(|cred| cred.uid()) != Some(me) {
        return;
    }
    let mut request = Vec::new();
    let mut chunk = [0_u8; 4096];
    let read = tokio::time::timeout(READ_WAIT, async {
        loop {
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                return Ok::<bool, std::io::Error>(true);
            }
            request.extend_from_slice(&chunk[..n]);
            if request.len() > wire::MAX_REQUEST {
                return Ok(false);
            }
            if request.contains(&b'\n') {
                return Ok(true);
            }
        }
    })
    .await;
    let response = match read {
        Ok(Ok(true)) => {
            let line = String::from_utf8_lossy(&request).into_owned();
            tauri::async_runtime::spawn_blocking(move || super::calls::handle_line(&state, &line))
                .await
                .unwrap_or_else(|_| {
                    wire::Response::refused(wire::Refusal::Bad, "AI Juggler couldn't answer that")
                })
        }
        Ok(Ok(false)) => wire::Response::refused(wire::Refusal::Limit, "That request is too long."),
        _ => return,
    };
    let mut line = serde_json::to_string(&response).unwrap_or_default();
    line.push('\n');
    let _ = stream.write_all(line.as_bytes()).await;
    let _ = stream.shutdown().await;
}
