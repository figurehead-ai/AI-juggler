//! AJ-E2302: the one way a codec reads its tool: newline-delimited text,
//! read on its own thread so [`super::DrivenSession::next_event`] can
//! wait with a timeout on pipes and sockets alike. Shared by every
//! adapter's codec (no copy per tool).
//!
//! A line longer than [`LINE_MAX`] is skipped and counted, never kept or
//! a crash; the reader stops at the end of the stream.

use std::io::{BufRead, Write};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::time::Duration;

/// The longest line a codec reads (a tool's aggregated command output
/// can be large; anything longer is skipped).
pub const LINE_MAX: usize = 16 * 1024 * 1024;

/// How many lines may wait unread before the reader waits too.
const QUEUE: usize = 1024;

/// What waiting for a line gave.
#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    /// A line, without its newline.
    Line(String),
    /// Nothing within the timeout.
    Timeout,
    /// The stream ended (the tool exited or closed the connection).
    Closed,
}

enum Item {
    Line(String),
    TooLong,
}

/// Lines from a tool, read on a thread of their own.
pub struct Lines {
    rx: Receiver<Item>,
    skipped: usize,
}

impl std::fmt::Debug for Lines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lines")
            .field("skipped", &self.skipped)
            .finish_non_exhaustive()
    }
}

impl Lines {
    /// Starts reading `reader` on its own thread.
    #[must_use]
    pub fn spawn(reader: Box<dyn BufRead + Send>) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel(QUEUE);
        std::thread::spawn(move || read_all(reader, &tx));
        Self { rx, skipped: 0 }
    }

    /// The next line, waiting at most `timeout`.
    pub fn next(&mut self, timeout: Duration) -> Next {
        loop {
            match self.rx.recv_timeout(timeout) {
                Ok(Item::Line(line)) => return Next::Line(line),
                Ok(Item::TooLong) => self.skipped += 1,
                Err(RecvTimeoutError::Timeout) => return Next::Timeout,
                Err(RecvTimeoutError::Disconnected) => return Next::Closed,
            }
        }
    }

    /// How many lines were too long to read.
    #[must_use]
    pub fn skipped(&self) -> usize {
        self.skipped
    }
}

fn send(tx: &SyncSender<Item>, item: Item) -> bool {
    match tx.try_send(item) {
        Ok(()) => true,
        Err(TrySendError::Full(item)) => tx.send(item).is_ok(),
        Err(TrySendError::Disconnected(_)) => false,
    }
}

fn read_all(mut reader: Box<dyn BufRead + Send>, tx: &SyncSender<Item>) {
    let mut line: Vec<u8> = Vec::new();
    let mut too_long = false;
    loop {
        let (consumed, ended) = {
            let Ok(buf) = reader.fill_buf() else { return };
            if buf.is_empty() {
                return; // the end; a last line without a newline is dropped
            }
            let newline = buf.iter().position(|b| *b == b'\n');
            let take = newline.unwrap_or(buf.len());
            if !too_long && line.len() + take <= LINE_MAX {
                line.extend_from_slice(&buf[..take]);
            } else {
                too_long = true;
                line.clear();
            }
            newline.map_or((buf.len(), false), |at| (at + 1, true))
        };
        reader.consume(consumed);
        if ended {
            let item = if too_long {
                Item::TooLong
            } else {
                let text = String::from_utf8_lossy(&line);
                Item::Line(text.trim_end_matches('\r').to_owned())
            };
            if !send(tx, item) {
                return;
            }
            line.clear();
            too_long = false;
        }
    }
}

/// Writes one JSON value as a line and flushes it.
///
/// # Errors
///
/// The write's error.
pub fn write_json(writer: &mut dyn Write, value: &serde_json::Value) -> std::io::Result<()> {
    let mut text = serde_json::to_string(value).map_err(std::io::Error::other)?;
    text.push('\n');
    writer.write_all(text.as_bytes())?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::{LINE_MAX, Lines, Next};
    use std::time::Duration;

    #[test]
    fn lines_arrive_in_order_and_the_end_closes() {
        let input = b"one\r\ntwo\n\nthree".to_vec();
        let mut lines = Lines::spawn(Box::new(std::io::Cursor::new(input)));
        let wait = Duration::from_secs(2);
        assert_eq!(lines.next(wait), Next::Line("one".into()));
        assert_eq!(lines.next(wait), Next::Line("two".into()));
        assert_eq!(lines.next(wait), Next::Line(String::new()));
        assert_eq!(
            lines.next(wait),
            Next::Closed,
            "an unfinished line is dropped"
        );
    }

    #[test]
    fn an_overlong_line_is_skipped_and_counted() {
        let mut input = vec![b'x'; LINE_MAX + 10];
        input.extend_from_slice(b"\nafter\n");
        let mut lines = Lines::spawn(Box::new(std::io::Cursor::new(input)));
        assert_eq!(
            lines.next(Duration::from_secs(5)),
            Next::Line("after".into())
        );
        assert_eq!(lines.skipped(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn waiting_times_out() {
        let (reader, _writer) = std::os::unix::net::UnixStream::pair().unwrap();
        let mut lines = Lines::spawn(Box::new(std::io::BufReader::new(reader)));
        assert_eq!(lines.next(Duration::from_millis(20)), Next::Timeout);
    }
}
