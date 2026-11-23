//! AJ-E1553 R1/R2: one sign-in, watched while it runs: its output read by
//! its adapter for the page and code it shows, its folder by the adapter's
//! "signed in?" signal (a file event there, or every few seconds in case
//! one is lost, then the adapter's own check), and its end.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::AppHandle;

use juggler_adapter::sign_in_page::{SignInPage, SignInWatch, last_lines, plain_lines};
use juggler_adapters::folder_wake::FolderWake;
use juggler_core::model::{Account, SessionId};

use super::{LAST_LINES, SignInState, host_of, tell};
use crate::state::AppState;

/// How long one wait for output lasts.
const TICK: Duration = Duration::from_millis(400);
/// The adapter's own check runs at least this often (file events can be
/// late or lost: `folder_wake`'s rule).
const CHECK_EVERY: Duration = Duration::from_secs(3);
/// The most output kept to read (its tail).
const KEPT_BYTES: usize = 64 * 1024;

/// What a sign-in came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Its App reads signed in.
    SignedIn,
    /// Its process ended first: its last lines.
    Ended(Vec<String>),
}

/// R1/R2: what a sign-in came to, by whether its folder reads done and
/// whether its process ended: done wins even as it ends (a tool writes
/// its sign-in, then exits); ended without it keeps the last lines.
pub(super) fn outcome(done: bool, exited: bool, output: &[u8]) -> Option<Outcome> {
    if done {
        return Some(Outcome::SignedIn);
    }
    exited.then(|| Outcome::Ended(kept_lines(output)))
}

/// An ended sign-in's last lines: plain, made safe, keys taken out.
pub(super) fn kept_lines(output: &[u8]) -> Vec<String> {
    last_lines(&plain_lines(output), LAST_LINES)
        .into_iter()
        .map(|line| crate::error_log::without_keys(&line))
        .collect()
}

/// The tail of a sign-in's output and the page read from it.
#[derive(Debug, Default)]
pub(super) struct Output {
    pub(super) bytes: Vec<u8>,
    page: Option<SignInPage>,
}

impl Output {
    pub(super) fn push(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);
        if self.bytes.len() > KEPT_BYTES {
            let cut = self.bytes.len() - KEPT_BYTES;
            self.bytes.drain(..cut);
        }
    }

    /// The page `read` finds now, when it's new. One found stays (a
    /// screen redrawn past it still shows it on the card).
    pub(super) fn new_page(
        &mut self,
        read: impl Fn(&str) -> Option<SignInPage>,
    ) -> Option<SignInPage> {
        let found = read(&plain_lines(&self.bytes))?;
        if self.page.as_ref() == Some(&found) {
            return None;
        }
        // A code seen once isn't lost when a later read has none.
        let found = match (&self.page, found) {
            (Some(was), mut now) if now.code.is_none() && was.url == now.url => {
                now.code.clone_from(&was.code);
                if self.page.as_ref() == Some(&now) {
                    return None;
                }
                now
            }
            (_, now) => now,
        };
        self.page = Some(found.clone());
        Some(found)
    }
}

/// Watches sign-in `id` for `account` on its own thread.
pub(super) fn spawn(
    app: AppHandle,
    state: Arc<AppState>,
    id: SessionId,
    account: Account,
    watch: SignInWatch,
) {
    let _ = std::thread::Builder::new()
        .name("sign-in watch".into())
        .spawn(move || run(&app, &state, &id, &account, watch));
}

fn run(
    app: &AppHandle,
    state: &Arc<AppState>,
    id: &SessionId,
    account: &Account,
    watch: SignInWatch,
) {
    let Some(adapter) = state.registry.get(&account.harness) else {
        return;
    };
    let Ok((backlog, rx)) = state.supervisor.tap_with_backlog(id) else {
        return;
    };
    let mut output = Output::default();
    output.push(&backlog);
    let woke = Arc::new(AtomicBool::new(false));
    let mut folder = {
        let woke = Arc::clone(&woke);
        FolderWake::trees(move |_| woke.store(true, Ordering::SeqCst))
    };
    folder.watch(&BTreeSet::from([account.state_dir.clone()]));
    let mut checked = Instant::now();
    let mut exited = false;
    loop {
        match rx.recv_timeout(TICK) {
            Ok(chunk) => {
                output.push(&chunk);
                while let Ok(more) = rx.try_recv() {
                    output.push(&more);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => exited = true,
        }
        exited = exited || state.supervisor.is_finished(id);
        if let Some(page) = output.new_page(|text| adapter.sign_in_page(text)) {
            let listed = state
                .sign_ins
                .change(&state.data_dir, id.as_str(), |entry| {
                    entry.info.page_host = host_of(&page.url);
                    entry.info.code.clone_from(&page.code);
                    entry.info.pastes_code = page.pastes_code;
                    entry.url = Some(page.url.clone());
                });
            if listed {
                tell(app, state);
            }
        }
        if !state.sign_ins.contains(id.as_str()) {
            return; // dismissed: its process was ended there
        }
        if !(exited || woke.swap(false, Ordering::SeqCst) || checked.elapsed() >= CHECK_EVERY) {
            continue;
        }
        checked = Instant::now();
        let done = watch.done(&*adapter, &account.state_dir);
        match outcome(done, exited, &output.bytes) {
            Some(Outcome::SignedIn) => {
                state
                    .sign_ins
                    .change(&state.data_dir, id.as_str(), |entry| {
                        entry.info.state = SignInState::SignedIn;
                        entry.info.last_lines.clear();
                        entry.url = None;
                    });
                // R1: its process ends; its App's sign-in is read again at
                // once (Needs you drops it), and the window reads its lists.
                if !exited {
                    let _ = state.supervisor.stop(id, Duration::from_secs(3));
                }
                crate::features::notifications::needs::sign_in_ended(state);
                tell(app, state);
                return;
            }
            Some(Outcome::Ended(lines)) => {
                state
                    .sign_ins
                    .change(&state.data_dir, id.as_str(), |entry| {
                        entry.info.state = SignInState::Ended;
                        entry.info.last_lines = lines;
                        entry.url = None;
                    });
                tell(app, state);
                return;
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R1/R2: done wins, even as the process ends; an end without it keeps
    /// the last lines (made safe); neither: still waiting.
    #[test]
    fn a_sign_in_comes_to_signed_in_or_ended_with_its_last_lines() {
        let output = b"\x1b[1mSign in\x1b[0m\r\nOpen https://auth.example.com/d?state=abc\r\nError: the code expired\r\n";
        assert_eq!(outcome(true, true, output), Some(Outcome::SignedIn));
        assert_eq!(outcome(true, false, output), Some(Outcome::SignedIn));
        assert_eq!(outcome(false, false, output), None);
        let Some(Outcome::Ended(lines)) = outcome(false, true, output) else {
            panic!("an early end must keep its lines");
        };
        assert_eq!(
            lines,
            [
                "Sign in",
                "Open https://auth.example.com/d",
                "Error: the code expired"
            ]
        );
        assert_eq!(outcome(false, true, b""), Some(Outcome::Ended(Vec::new())));
    }

    #[test]
    fn a_page_is_told_once_and_its_code_kept() {
        let read = |text: &str| {
            juggler_adapter::sign_in_page::read(
                text,
                &juggler_adapter::PageRule {
                    hosts: &["auth.example.com"],
                    device_codes: true,
                    paste_words: &[],
                },
            )
        };
        let mut output = Output::default();
        output.push(b"Starting...\r\n");
        assert_eq!(output.new_page(read), None, "nothing to read yet");
        output.push(b"Code: WDJB-MJHT\r\nOpen https://auth.example.com/device\r\n");
        let page = output.new_page(read).unwrap();
        assert_eq!(page.code.as_deref(), Some("WDJB-MJHT"));
        output.push(b"Waiting...\r\n");
        assert_eq!(
            output.new_page(read),
            None,
            "the same page isn't told twice"
        );
        // The code scrolled out of the kept tail: the card keeps it.
        let mut later = Output {
            bytes: Vec::new(),
            page: Some(page),
        };
        later.push(b"Open https://auth.example.com/device\r\n");
        assert_eq!(later.new_page(read), None);
    }

    #[test]
    fn only_the_tail_of_a_long_output_is_kept() {
        let mut output = Output::default();
        output.push(&vec![b'a'; KEPT_BYTES]);
        output.push(b"the end");
        assert_eq!(output.bytes.len(), KEPT_BYTES);
        assert!(output.bytes.ends_with(b"the end"));
    }
}
