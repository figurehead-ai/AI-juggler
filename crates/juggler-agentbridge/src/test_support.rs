//! Tests' fake AI Juggler: a socket where the app's would be, answering.

use std::io::{BufRead, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

/// A socket listening in a folder only this user may open (`agent/`,
/// 0700, under `root`), as the app's is.
pub(crate) fn agent_socket(root: &Path) -> (PathBuf, UnixListener) {
    let dir = root.join("agent");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let socket = dir.join("bridge.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    (socket, listener)
}

/// Answers each line sent to `listener` with `answer`, on a thread of its
/// own, for as long as the test runs.
pub(crate) fn answer_each(listener: UnixListener, answer: &'static [u8]) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            let _ = std::io::BufReader::new(&stream).read_line(&mut line);
            let mut stream = stream;
            let _ = stream.write_all(answer);
        }
    });
}
