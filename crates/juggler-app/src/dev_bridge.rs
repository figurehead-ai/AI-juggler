//! The read-only test bridge (`docs/qa/DEV-BRIDGE.md`): the real UI, in
//! a normal browser, against this backend and its data — so QA can walk
//! a real setup without granting the app Accessibility.
//!
//! Compiled only into debug builds with the `dev-bridge` feature (a
//! release build with the feature fails to compile), and started only
//! when `AIJ_DEV_BRIDGE=1` is set at launch, in the Dev edition.
//!
//! - It listens on 127.0.0.1, on a random port.
//! - Every request needs `Authorization: Bearer <token>`, a 256-bit
//!   token made per run and kept in a `0600` file in the Dev data folder.
//! - The `Host` must be loopback, and the `Origin` the one allowed dev
//!   origin.
//! - Requests are size- and rate-limited.
//! - It serves only the commands [`routes`] names; a test holds each of
//!   them to [`crate::CommandAccess::Read`] in the command registry.
//! - The refresh events go out over SSE, behind the same token.

mod routes;
mod server;
pub mod walk_data;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use zeroize::Zeroizing;

pub use routes::{READ_ROUTES, ReadRoute};

/// The launch switch: the bridge starts only when this is exactly `1`.
pub const ENV_ENABLE: &str = "AIJ_DEV_BRIDGE";

/// Overrides the one browser origin the bridge answers (CORS and the
/// `Origin` check). Only `http://localhost:<port>` or
/// `http://127.0.0.1:<port>`.
pub const ENV_ORIGIN: &str = "AIJ_DEV_BRIDGE_ORIGIN";

/// The Vite dev server (`ui/vite.config.ts`).
pub const DEFAULT_ORIGIN: &str = "http://localhost:5173";

/// The port and token, in the Dev data folder (`0600`).
pub const TOKEN_FILE: &str = "dev-bridge.json";

/// The events the UI refreshes on. Nothing that carries terminal output,
/// screen frames or the progress of a write goes out.
pub const FORWARDED_EVENTS: &[&str] = &[
    crate::EVENT_APP_READY,
    crate::EVENT_SESSIONS_UPDATED,
    crate::EVENT_SESSIONS_RESOURCES,
    crate::EVENT_SESSIONS_QUOTA,
    crate::EVENT_THIS_MAC,
    "update://status",
    // AJ-E1529: the rail reads sessions started outside AI Juggler again.
    crate::features::conversations::EVENT_CHANGED,
    // The 0.2 walk (F4, F19): Needs you, the bell and Home read again
    // when the backend's count changes; without it a bridged page's
    // counts went stale.
    crate::features::notifications::views::EVENT_CHANGED,
    crate::features::processes::EVENT_CHANGED,
    crate::features::runtime::EVENT_CHANGED,
    crate::features::tasks::EVENT_CHANGED,
    crate::features::messages::EVENT_CHANGED,
];

/// Events the UI listens to that the bridge never forwards, each with its
/// reason (a test holds every event the UI names to one list or the
/// other).
pub const NOT_FORWARDED_EVENTS: &[(&str, &str)] = &[
    ("terminal://output", "a terminal's output"),
    (
        "terminal://exit",
        "a terminal's own lifecycle, for its view only",
    ),
    (
        "terminal://windows",
        "the native terminal windows, window-only",
    ),
    ("preview://frame", "screen frames"),
    ("preview://state", "the window preview, window-only"),
    ("fleet://switch-progress", "the progress of a write"),
    ("backup://progress", "the progress of a write"),
    ("app://quit-requested", "the native window's own quit"),
    ("app://go-to", "the native menu asking the window to act"),
    ("app://open", "the menu-bar item asking the window to act"),
    (
        "app://new-session",
        "the menu-bar item asking the window to act",
    ),
    (
        "notifications://open",
        "the menu-bar item asking the window to act",
    ),
];

/// The per-run bearer token: 32 random bytes, hex. Its `Debug` never
/// shows it, and it is wiped on drop.
pub struct Token(Zeroizing<String>);

impl Token {
    /// A fresh token from the OS random source.
    ///
    /// # Errors
    ///
    /// The OS random source failed.
    pub fn generate() -> Result<Self, String> {
        let mut bytes = Zeroizing::new([0_u8; 32]);
        getrandom::fill(bytes.as_mut()).map_err(|error| error.to_string())?;
        let mut hex = Zeroizing::new(String::with_capacity(64));
        for byte in bytes.iter() {
            use std::fmt::Write as _;
            let _ = write!(hex, "{byte:02x}");
        }
        Ok(Self(hex))
    }

    /// The token as sent in the `Authorization` header.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Constant-time comparison with what a request presented.
    #[must_use]
    pub fn matches(&self, presented: &[u8]) -> bool {
        use subtle::ConstantTimeEq as _;
        bool::from(self.0.as_bytes().ct_eq(presented))
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

/// What a running bridge was started with.
#[derive(Debug)]
pub struct Config {
    /// The port it listens on (127.0.0.1).
    pub port: u16,
    /// The one browser origin it answers.
    pub origin: String,
    /// The bearer token.
    pub token: Token,
}

/// Starts the bridge if `AIJ_DEV_BRIDGE=1`, in the Dev edition only.
/// Anything wrong is said on stderr and the app runs on without it.
pub fn start_if_enabled<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if std::env::var(ENV_ENABLE).as_deref() != Ok("1") {
        return;
    }
    if crate::edition::current() != crate::edition::Edition::Dev {
        eprintln!("ai-juggler: dev bridge: Dev edition only; not started");
        return;
    }
    // The bridge starts no vendor program (DEV-BRIDGE.md): from here on,
    // what's installed is read from files only, the version not asked
    // (the 0.2 walk, F19: `switch_targets` and Settings ran `--version`).
    {
        use tauri::Manager as _;
        app.state::<Arc<crate::state::AppState>>()
            .tool_programs
            .stop();
    }
    let origin = match std::env::var(ENV_ORIGIN) {
        Ok(origin) => origin,
        Err(_) => DEFAULT_ORIGIN.to_string(),
    };
    if let Err(message) = start(app, &origin) {
        eprintln!("ai-juggler: dev bridge not started: {message}");
    }
}

fn start<R: tauri::Runtime>(app: &tauri::AppHandle<R>, origin: &str) -> Result<(), String> {
    use tauri::Manager as _;
    let origin = checked_origin(origin)?;
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| format!("bind 127.0.0.1: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let config = Arc::new(Config {
        port,
        origin,
        token: Token::generate()?,
    });
    let data_dir = app.state::<Arc<crate::state::AppState>>().data_dir.clone();
    let file = write_token_file(&data_dir, &config)?;
    server::spawn(app.clone(), listener, Arc::clone(&config))?;
    // Never the token itself: where to read it.
    eprintln!(
        "ai-juggler: read-only dev bridge on 127.0.0.1:{port} for {}; token in {}",
        config.origin,
        file.display()
    );
    Ok(())
}

/// Removes the token file (on exit); a missing file is fine.
pub fn remove_token_file(data_dir: &Path) {
    let _ = std::fs::remove_file(data_dir.join(TOKEN_FILE));
}

/// Only a loopback `http` origin with a port: the bridge serves a page on
/// this Mac, never a site.
///
/// # Errors
///
/// The origin isn't `http://localhost:<port>` or `http://127.0.0.1:<port>`.
pub fn checked_origin(origin: &str) -> Result<String, String> {
    let refuse =
        || format!("{ENV_ORIGIN} must be http://localhost:<port> or http://127.0.0.1:<port>");
    let rest = origin.strip_prefix("http://").ok_or_else(refuse)?;
    let (host, port) = rest.split_once(':').ok_or_else(refuse)?;
    if !matches!(host, "localhost" | "127.0.0.1")
        || port.is_empty()
        || port.len() > 5
        || !port.bytes().all(|byte| byte.is_ascii_digit())
        || port.parse::<u16>().map_or(true, |port| port == 0)
    {
        return Err(refuse());
    }
    Ok(origin.to_string())
}

/// Writes `{port, token, origin}` owner-only and atomically: a fresh
/// `0600` file, then a rename over the last run's.
fn write_token_file(data_dir: &Path, config: &Config) -> Result<PathBuf, String> {
    use std::io::Write as _;
    let body = Zeroizing::new(
        serde_json::json!({
            "port": config.port,
            "token": config.token.expose(),
            "origin": config.origin,
        })
        .to_string(),
    );
    let path = data_dir.join(TOKEN_FILE);
    let staging = data_dir.join(format!("{TOKEN_FILE}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&staging);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let written = options
        .open(&staging)
        .and_then(|mut file| {
            file.write_all(body.as_bytes())
                .and_then(|()| file.sync_all())
        })
        .and_then(|()| std::fs::rename(&staging, &path));
    if let Err(error) = written {
        let _ = std::fs::remove_file(&staging);
        return Err(format!("write {}: {error}", path.display()));
    }
    Ok(path)
}

#[cfg(test)]
mod tests;
