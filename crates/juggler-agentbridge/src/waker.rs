//! D-119 (AJ-E3013): waking an idle desktop Code-tab session for a pool
//! message, through Claude Code's own inbox.
//!
//! - Only in AI Juggler's connector (`--mcp --plugin`, keyless) and only
//!   when this process's own environment names its session's inbox socket
//!   ([`MESSAGING_SOCKET_ENV`]): Claude Code gives it to the processes it
//!   starts. Nothing is woken anywhere else.
//! - It asks the app to be told ([`Wait`], held at most
//!   [`WAIT_MAX_SECS`]). The app attests it, and answers with a notice only
//!   when the pool's waking rule allows it (the pool at Full, or a rule the
//!   user made: the same gate and hop limit as a terminal member's) and
//!   within its wake budget; else nothing.
//! - It posts that notice, facts only ("AI Juggler: 1 unread message from
//!   Planner."), as one user line to the session's own socket, as the
//!   session's own child (with the session's token in the auth line when it
//!   was given one). Claude Code treats it as a peer message: it can't
//!   approve or change anything, and Claude Code rate-limits it too.
//! - The token is read from this process's environment only and sent only
//!   to that socket; it never goes to AI Juggler and is never written.
//! - When AI Juggler isn't answering, it waits longer between tries, and
//!   it ends with the process.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::client::{CallError, post_lines, send_line};
use crate::wire::{
    MESSAGING_SOCKET_ENV, MESSAGING_TOKEN_ENV, Response, VERSION, WAIT_MAX_SECS, Wait,
};

/// The longest notice posted.
pub const NOTICE_MAX: usize = 300;

/// The shortest and longest pause after AI Juggler didn't answer.
const BACKOFF: (Duration, Duration) = (Duration::from_secs(15), Duration::from_secs(300));

/// Where a wake goes: the session's inbox socket, and its token when it
/// gave one.
#[derive(Clone, Debug)]
pub struct Inbox {
    /// The socket.
    pub socket: PathBuf,
    /// The auth line's token.
    pub token: Option<String>,
}

impl Inbox {
    /// From this process's own environment; `None` when it names no inbox.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let socket = std::env::var_os(MESSAGING_SOCKET_ENV)
            .filter(|socket| !socket.is_empty())
            .map(PathBuf::from)?;
        let token = std::env::var(MESSAGING_TOKEN_ENV)
            .ok()
            .filter(|token| !token.is_empty());
        Some(Self { socket, token })
    }
}

/// The notice as posted: one line, no control characters, bounded.
#[must_use]
pub fn clean_notice(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(NOTICE_MAX)
        .collect::<String>()
        .trim()
        .to_owned()
}

/// The lines posted to the session's inbox for `notice`.
#[must_use]
pub fn inbox_lines(inbox: &Inbox, notice: &str) -> String {
    let mut lines = String::new();
    if let Some(token) = &inbox.token {
        lines.push_str(&serde_json::json!({"type": "auth", "token": token}).to_string());
        lines.push('\n');
    }
    lines.push_str(
        &serde_json::json!({
            "type": "user",
            "message": {"role": "user", "content": clean_notice(notice)}
        })
        .to_string(),
    );
    lines.push('\n');
    lines
}

/// Posts `notice` to the session's inbox (a socket of this user's, never
/// a link).
///
/// # Errors
///
/// When the socket isn't trustworthy or the post fails.
pub fn post(inbox: &Inbox, notice: &str) -> Result<(), CallError> {
    post_lines(
        &inbox.socket,
        &inbox_lines(inbox, notice),
        Duration::from_secs(2),
    )
}

/// One wait: the notice the app sent, if any.
///
/// # Errors
///
/// When the app can't be reached or refuses (the caller isn't attested,
/// or its app can't be woken).
pub fn wait_once(app: &Path) -> Result<Option<String>, CallError> {
    let request = Wait {
        v: VERSION,
        token: String::new(),
        wait_secs: WAIT_MAX_SECS,
    };
    let line =
        serde_json::to_string(&request).map_err(|error| CallError::Garbled(error.to_string()))?;
    let answer = send_line(
        app,
        &line,
        Duration::from_secs(u64::from(WAIT_MAX_SECS) + 10),
    )?;
    let response: Response = serde_json::from_str(answer.trim())
        .map_err(|error| CallError::Garbled(error.to_string()))?;
    if let Some(refusal) = response.refused {
        return Err(CallError::Refused(
            refusal,
            response.reason.unwrap_or_default(),
        ));
    }
    Ok(response
        .result
        .and_then(|result| result.get("wake")?.as_str().map(clean_notice))
        .filter(|notice| !notice.is_empty()))
}

/// Runs the waker until the process ends: waits, posts, waits again.
pub fn run(app: &Path, inbox: &Inbox) {
    let mut pause = BACKOFF.0;
    loop {
        match wait_once(app) {
            Ok(Some(notice)) => {
                let _ = post(inbox, &notice);
                pause = BACKOFF.0;
            }
            Ok(None) => pause = BACKOFF.0,
            Err(_) => {
                std::thread::sleep(pause);
                pause = (pause * 2).min(BACKOFF.1);
            }
        }
    }
}

/// Starts the waker beside the connector's MCP server, when this process
/// is one (keyless, with the app's socket) and its session has an inbox.
pub fn start(app: Option<PathBuf>, inbox: Option<Inbox>) {
    let (Some(app), Some(inbox)) = (app, inbox) else {
        return;
    };
    let _ = std::thread::Builder::new()
        .name("juggler-waker".into())
        .spawn(move || run(&app, &inbox));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_notice_is_one_bounded_line_and_the_token_only_in_the_auth_line() {
        let inbox = Inbox {
            socket: PathBuf::from("/x"),
            token: Some("t0k".into()),
        };
        let lines = inbox_lines(&inbox, "AI Juggler: 2 unread\nmessages\u{1b} from Planner.");
        let mut parts = lines.lines();
        let auth: serde_json::Value = serde_json::from_str(parts.next().unwrap()).unwrap();
        assert_eq!(auth, serde_json::json!({"type": "auth", "token": "t0k"}));
        let user: serde_json::Value = serde_json::from_str(parts.next().unwrap()).unwrap();
        assert_eq!(
            user["message"]["content"],
            "AI Juggler: 2 unread messages  from Planner."
        );
        assert!(parts.next().is_none());
        let without = inbox_lines(
            &Inbox {
                token: None,
                ..inbox
            },
            "x",
        );
        assert_eq!(without.lines().count(), 1);
        assert_eq!(
            clean_notice(&"é".repeat(NOTICE_MAX + 5)).chars().count(),
            NOTICE_MAX
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_wake_is_posted_to_a_socket_of_this_users_only() {
        use std::io::Read;
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("inbox");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket = dir.join("s.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let session = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut text = String::new();
            stream.read_to_string(&mut text).unwrap();
            text
        });
        let inbox = Inbox {
            socket: socket.clone(),
            token: None,
        };
        post(&inbox, "AI Juggler: 1 unread message from Planner.").unwrap();
        let got = session.join().unwrap();
        assert!(got.contains("\"role\":\"user\"") && got.contains("from Planner"));
        let link = dir.join("link.sock");
        std::os::unix::fs::symlink(&socket, &link).unwrap();
        assert!(
            post(
                &Inbox {
                    socket: link,
                    token: None
                },
                "x"
            )
            .is_err(),
            "never through a link"
        );
    }

    #[test]
    fn with_no_ai_juggler_a_wait_fails_quietly() {
        let root = tempfile::tempdir().unwrap();
        assert!(wait_once(&root.path().join("bridge.sock")).is_err());
    }
}
