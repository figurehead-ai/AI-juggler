//! OpenCode 2's background service, kept out of managed accounts.
//!
//! OpenCode 2 runs one shared background server per user: a client finds
//! it through `$XDG_STATE_HOME/opencode/service.json` (its url, pid,
//! version and password) or starts it, on port 49374 unless
//! `$XDG_CONFIG_HOME/opencode/service.json` names another; the server
//! outlives the terminal that started it (research
//! `docs/research/capabilities/opencode.md` §A1, the 2.2.0 source
//! `packages/cli/src/services/{server-connection,service-config}.ts`, read
//! 2026-10-27). A managed account has its own XDG folders, so each would
//! look for its own service on that same port: it fails while the user's
//! own OpenCode or its Desktop runs, or, started first, blocks them.
//!
//! So every launch of a managed account on OpenCode 2 passes
//! `--standalone`, the vendor's own option for "a private server instead of
//! the background service". Its server (`serve --stdio --port 0`) takes a
//! free port the system hands out and a password made for it, kept in
//! memory, and it ends when the terminal does: it treats the end of its
//! input pipe as the end of its lease, which the system closes even when
//! the terminal is killed (`packages/cli/src/services/standalone.ts`).
//! Chosen over a `service.json` of the account's own (a port and a
//! password AI Juggler would make):
//!
//! - nothing outlives the session, so there's no server for AI Juggler to
//!   stop, and none left behind by a crash;
//! - no port is picked (none can collide) and no password is written to
//!   or read from any file; the user's default `service.json` is never
//!   opened;
//! - nothing is written to the account's folder (per-launch settings go in
//!   as flags, ADR-0018 §3), so every existing account is covered.
//!
//! What the account gives up is a server other clients could join — no
//! other client shares its folders anyway. The default folder used in
//! place keeps the shared service: it runs as the user's own `opencode`
//! does, beside their Desktop.
//!
//! Verified from the 2.2.0 source: the option, and what its server does.
//! Inferred, on the live checklist (AJ-E1612): that `run` and `auth login`
//! take it after their own names (they declare it as a global option),
//! and every 2.x before 2.2.0 had it.
//!
//! The line is told by the installed binary's version (`--version`), never
//! by its name: 1.x has no service, and its launches don't change. A
//! version that can't be read leaves the launch as 1.x's.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use super::{Line, line_of, parse_version};

/// The option that gives a launch a private server of its own.
pub const STANDALONE: &str = "--standalone";

/// A binary as last seen (its size and time) and the version it printed.
type Seen = (u64, Option<SystemTime>, String);

/// The version each binary printed, by path, while it is unchanged.
static VERSIONS: LazyLock<Mutex<HashMap<PathBuf, Seen>>> = LazyLock::new(Mutex::default);

fn stamp(bin: &Path) -> Option<(u64, Option<SystemTime>)> {
    let meta = std::fs::metadata(bin).ok()?;
    Some((meta.len(), meta.modified().ok()))
}

/// Records what `bin --version` printed (discovery reads it anyway).
pub fn remember(bin: &Path, version: &str) {
    let Some((len, modified)) = stamp(bin) else {
        return;
    };
    if let Ok(mut versions) = VERSIONS.lock() {
        versions.insert(bin.to_path_buf(), (len, modified, version.to_owned()));
    }
}

/// The line of the OpenCode at `bin`: what it printed last while it is
/// unchanged, else what `bin --version` prints now (an update replaces the
/// file, so it is asked again). `None` when it can't be told.
#[must_use]
pub fn installed_line(bin: &Path) -> Option<Line> {
    let (len, modified) = stamp(bin)?;
    if let Ok(versions) = VERSIONS.lock()
        && let Some((known_len, known_modified, version)) = versions.get(bin)
        && *known_len == len
        && *known_modified == modified
    {
        return line_of(version);
    }
    // With the user's PATH: the npm package runs on `node`.
    let output = juggler_adapter::user_path::command(bin)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let version = parse_version(&String::from_utf8_lossy(&output.stdout))?;
    remember(bin, &version);
    line_of(&version)
}

/// Whether a launch of the account folder needs a private server: only a
/// managed account (`managed`), only on OpenCode 2.
#[must_use]
pub fn needs_standalone(bin: &Path, managed: bool) -> bool {
    managed && installed_line(bin) == Some(Line::V2)
}
