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
    MESSAGING_SOCKET_ENV, MESSAGING_TOKEN_ENV, Refusal, Response, VERSION, WAIT_MAX_SECS, Wait,
};

/// The longest notice posted.
pub const NOTICE_MAX: usize = 300;

/// The shortest and longest pause after AI Juggler didn't answer or
/// refused (limited, not attested, can't be woken); a [`Refusal::Busy`]
/// is tried again after [`MIN_PAUSE`] instead. At most a minute (the A9
/// lab walk, 2026-11-15: five minutes kept a member's wake away long after
/// the refusal had passed).
const BACKOFF: (Duration, Duration) = (Duration::from_secs(15), Duration::from_secs(60));

/// AJ-E3013 (the A9 lab walk): how long after the waker starts a "not
/// attested" is tried again after [`MIN_PAUSE`], with no back-off: a
/// Code-tab session's records come a few seconds after it starts (the
/// start race), and a refusal then is no reason to wait minutes.
pub const START_GRACE: Duration = Duration::from_secs(120);

/// M2: the least time between two waits, whatever the answer: an answer
/// that comes back at once (a wake, or none) never makes a busy loop.
pub const MIN_PAUSE: Duration = Duration::from_secs(5);

/// How the waker paces itself.
#[derive(Clone, Copy, Debug)]
pub struct Pace {
    /// The least time between two waits.
    pub min: Duration,
    /// The first and the longest pause after a refusal.
    pub backoff: (Duration, Duration),
    /// How long after it starts, until a wait is first answered, a "not
    /// attested" is tried again after `min`.
    pub start_grace: Duration,
}

impl Default for Pace {
    fn default() -> Self {
        Self {
            min: MIN_PAUSE,
            backoff: BACKOFF,
            start_grace: START_GRACE,
        }
    }
}

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
    run_until(app, inbox, Pace::default(), &|| true);
}

/// [`run`] while `go_on` says so; how many waits it asked for.
pub fn run_until(app: &Path, inbox: &Inbox, pace: Pace, go_on: &dyn Fn() -> bool) -> usize {
    let mut pause = pace.backoff.0;
    let mut asked = 0;
    let began = std::time::Instant::now();
    let mut answered = false;
    while go_on() {
        let started = std::time::Instant::now();
        asked += 1;
        let answer = wait_once(app);
        let busy = matches!(&answer, Err(CallError::Refused(Refusal::Busy, _)));
        // The start race: not attested yet, early on, is asked again soon.
        let racing = !answered
            && began.elapsed() < pace.start_grace
            && matches!(&answer, Err(CallError::Refused(Refusal::Unattested, _)));
        if let Ok(found) = answer {
            answered = true;
            if let Some(notice) = found {
                let _ = post(inbox, &notice);
            }
            pause = pace.backoff.0;
        } else if !busy && !racing {
            std::thread::sleep(pause);
            pause = (pause * 2).min(pace.backoff.1);
            continue;
        }
        // M2: never sooner than the least pause, however fast the answer
        // came; the global cap's Busy is tried again after it too, with
        // no back-off (a flood of others' mustn't push a member's wake
        // minutes away).
        if let Some(rest) = pace.min.checked_sub(started.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    asked
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

    /// M2: an app that answers at once (no wake, or a wake) never makes
    /// the waker loop: at least the least pause between two waits.
    #[cfg(unix)]
    #[test]
    fn an_answer_at_once_never_makes_a_busy_loop() {
        let root = tempfile::tempdir().unwrap();
        let (socket, listener) = crate::test_support::agent_socket(root.path());
        crate::test_support::answer_each(listener, b"{\"result\":{\"wake\":null}}\n");
        let inbox = Inbox {
            socket: root.path().join("no-inbox.sock"),
            token: None,
        };
        let pace = Pace {
            min: Duration::from_millis(200),
            backoff: (Duration::from_millis(200), Duration::from_millis(400)),
            start_grace: Duration::ZERO,
        };
        let until = std::time::Instant::now() + Duration::from_millis(1_000);
        let asked = run_until(&socket, &inbox, pace, &|| std::time::Instant::now() < until);
        assert!((1..=6).contains(&asked), "{asked} waits in a second");
    }

    /// The review's global-cap finding: a Busy answer (every keyless line
    /// together at its cap) is tried again after the least pause, never
    /// backed off; any other refusal is.
    #[cfg(unix)]
    #[test]
    fn busy_is_tried_again_after_the_least_pause_and_other_refusals_back_off() {
        let asked_in_a_second = |answer: &'static [u8], start_grace: Duration| {
            let root = tempfile::tempdir().unwrap();
            let (socket, listener) = crate::test_support::agent_socket(root.path());
            crate::test_support::answer_each(listener, answer);
            let inbox = Inbox {
                socket: root.path().join("no-inbox.sock"),
                token: None,
            };
            let pace = Pace {
                min: Duration::from_millis(100),
                backoff: (Duration::from_millis(600), Duration::from_millis(2_000)),
                start_grace,
            };
            let until = std::time::Instant::now() + Duration::from_millis(1_000);
            run_until(&socket, &inbox, pace, &|| std::time::Instant::now() < until)
        };
        let busy = asked_in_a_second(
            b"{\"refused\":\"busy\",\"reason\":\"busy\"}\n",
            Duration::ZERO,
        );
        assert!(
            (6..=11).contains(&busy),
            "{busy} waits in a second when busy"
        );
        let limited = asked_in_a_second(
            b"{\"refused\":\"limit\",\"reason\":\"limit\"}\n",
            Duration::ZERO,
        );
        assert!(
            (1..=2).contains(&limited),
            "{limited} waits in a second when limited"
        );
        // AJ-E3013 (the A9 lab walk): "not attested" in the start race is
        // asked again after the least pause, never backed off; once the
        // grace is over, it backs off as any refusal does.
        let unattested = b"{\"refused\":\"unattested\",\"reason\":\"not yet\"}\n";
        let racing = asked_in_a_second(unattested, Duration::from_secs(60));
        assert!(
            (6..=11).contains(&racing),
            "{racing} waits in a second in the start race"
        );
        let after = asked_in_a_second(unattested, Duration::ZERO);
        assert!(
            (1..=2).contains(&after),
            "{after} waits in a second after the grace"
        );
    }
}
