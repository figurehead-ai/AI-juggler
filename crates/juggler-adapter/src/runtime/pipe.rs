//! The plumbing every line-protocol driven session shares: the tool's
//! lines in, JSON lines out, the events read and not yet handed out, and
//! whether the tool has gone. Each codec keeps what the lines mean.

use std::collections::VecDeque;
use std::io::Write;
use std::time::Duration;

use serde_json::Value;

use super::lines::{Lines, Next, write_json};
use super::{Connection, RuntimeEvent};
use crate::error::AdapterError;

/// One connection's lines, writer and event queue.
pub struct Pipe {
    lines: Lines,
    writer: Box<dyn Write + Send>,
    /// Events read and not yet handed out, oldest first.
    pub queue: VecDeque<RuntimeEvent>,
    exited: bool,
}

impl std::fmt::Debug for Pipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipe")
            .field("queued", &self.queue.len())
            .field("exited", &self.exited)
            .finish_non_exhaustive()
    }
}

impl Pipe {
    /// Starts reading `io`'s lines on their own thread.
    #[must_use]
    pub fn new(io: Connection) -> Self {
        Self {
            lines: Lines::spawn(io.reader),
            writer: io.writer,
            queue: VecDeque::new(),
            exited: false,
        }
    }

    /// Writes one JSON line to the tool.
    ///
    /// # Errors
    ///
    /// The write's error.
    pub fn send(&mut self, value: &Value) -> Result<(), AdapterError> {
        write_json(self.writer.as_mut(), value).map_err(AdapterError::Io)
    }

    /// The tool's stream has ended.
    #[must_use]
    pub fn exited(&self) -> bool {
        self.exited
    }

    /// The next line, waiting at most `timeout`: `None` on a timeout or
    /// once the stream has ended (its end queues one
    /// [`RuntimeEvent::Exited`]).
    pub fn read(&mut self, timeout: Duration) -> Option<String> {
        if self.exited {
            return None;
        }
        match self.lines.next(timeout) {
            Next::Line(line) => Some(line),
            Next::Timeout => None,
            Next::Closed => {
                self.exited = true;
                self.queue.push_back(RuntimeEvent::Exited);
                None
            }
        }
    }

    /// Sends a codec's `replies` to the tool, then queues its `events`.
    ///
    /// # Errors
    ///
    /// A reply's write error (the events are then not queued).
    pub fn take(
        &mut self,
        events: Vec<RuntimeEvent>,
        replies: &[Value],
    ) -> Result<(), AdapterError> {
        for reply in replies {
            self.send(reply)?;
        }
        self.queue.extend(events);
        Ok(())
    }

    /// The message of the first error queued.
    #[must_use]
    pub fn queued_error(&self) -> Option<String> {
        self.queue.iter().find_map(|event| match event {
            RuntimeEvent::Error { message, .. } => Some(message.clone()),
            _ => None,
        })
    }

    /// Hands out everything queued (a release's tail starts with it).
    pub fn drain(&mut self) -> Vec<RuntimeEvent> {
        self.queue.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::events::ErrorCategory;
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("lock").extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn lines_in_replies_out_and_the_end_queued_once() {
        let written = Shared::default();
        let mut pipe = Pipe::new(Connection {
            reader: Box::new(Cursor::new(b"one\n".to_vec())),
            writer: Box::new(written.clone()),
        });
        assert_eq!(pipe.read(Duration::from_secs(5)).as_deref(), Some("one"));
        let error = RuntimeEvent::Error {
            category: ErrorCategory::Other,
            message: "no".into(),
            will_retry: false,
        };
        pipe.take(vec![error], &[serde_json::json!({"ok": true})])
            .expect("take");
        assert_eq!(
            String::from_utf8(written.0.lock().expect("lock").clone()).expect("utf-8"),
            "{\"ok\":true}\n"
        );
        assert_eq!(pipe.queued_error().as_deref(), Some("no"));
        assert_eq!(pipe.read(Duration::from_secs(5)), None, "the end");
        assert!(pipe.exited());
        assert_eq!(pipe.read(Duration::from_millis(10)), None);
        let queued = pipe.drain();
        assert_eq!(queued.len(), 2);
        assert_eq!(queued[1], RuntimeEvent::Exited, "queued once");
        assert!(pipe.queue.is_empty());
    }
}
