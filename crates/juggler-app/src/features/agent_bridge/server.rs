//! The bridge's socket (SECURITY-GUIDELINES C7, ADR-0020): a local
//! connection only this user can make, never a TCP or HTTP listener
//! ([`juggler_platform::local_ipc`]).
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
//! the app's start when sessions still hold keys. Where the system has
//! no such connection yet (Windows), nothing is served and the bridge
//! stays off.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use juggler_agentbridge::wire;
use juggler_platform::local_ipc;

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
/// ours, the socket can't be made, or this system has no local
/// connections yet.
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
    let bound = local_ipc::bind(&socket, juggler_platform::user::current()?)?;
    let weak = Arc::downgrade(state);
    tauri::async_runtime::spawn(async move {
        let Ok(listener) = bound.into_async() else {
            return;
        };
        let open = Arc::new(tokio::sync::Semaphore::new(MAX_OPEN));
        loop {
            // Anyone but this user is dropped unanswered (`None`).
            let Ok(Some(stream)) = listener.accept().await else {
                continue;
            };
            let Some(state) = weak.upgrade() else {
                return; // the app is gone
            };
            let Ok(permit) = Arc::clone(&open).try_acquire_owned() else {
                continue; // too many at once: dropped, the caller retries
            };
            tauri::async_runtime::spawn(async move {
                answer(stream, state).await;
                drop(permit);
            });
        }
    });
    *serving = Some(socket.clone());
    Ok(socket)
}

/// One connection (already known to be this user's): one request, one
/// answer.
async fn answer(mut stream: local_ipc::Stream, state: Arc<AppState>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
