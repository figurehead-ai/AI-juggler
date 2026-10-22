//! Writes a test's stand-in executable (a fake tool, a stub CLI) from a
//! child process, never from the test process. A file this process held
//! open for writing while another test in the binary forked stays "busy"
//! (ETXTBSY) until that child execs, and running it fails at once — a CI
//! flake, 2026-10-21 (AJ-E806). A `sh` that writes it and exits leaves no
//! writer behind. Shared by every crate's tests:
//! `#[path = "…/juggler-supervisor/tests/fixtures/stand_in.rs"] mod stand_in;`

#![allow(dead_code)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Writes `body` to `path` as an executable (mode 755).
pub fn write(path: impl AsRef<Path>, body: impl AsRef<[u8]>) {
    let path = path.as_ref();
    let mut writer = Command::new("/bin/sh")
        .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "sh"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("sh to write a stand-in");
    writer
        .stdin
        .take()
        .expect("its stdin")
        .write_all(body.as_ref())
        .expect("the stand-in's body");
    assert!(
        writer.wait().expect("sh to finish").success(),
        "wrote {}",
        path.display()
    );
}
