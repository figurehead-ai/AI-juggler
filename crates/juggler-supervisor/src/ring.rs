//! Bounded output ring buffer: captures the tail of a session's output
//! without unbounded memory growth (spec §5.1 output fan-out).

use std::collections::VecDeque;

use juggler_core::model::Timestamp;

/// Fixed-capacity byte buffer keeping the most recent output, with
/// optional live listeners (the embedded-terminal stream, spec §5.1).
#[derive(Debug)]
pub struct OutputRing {
    buffer: VecDeque<u8>,
    capacity: usize,
    total_written: u64,
    last_write: Option<Timestamp>,
    listeners: Vec<std::sync::mpsc::Sender<Vec<u8>>>,
}

impl OutputRing {
    /// Creates a ring with the given byte capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity.min(64 * 1024)),
            capacity,
            total_written: 0,
            last_write: None,
            listeners: Vec::new(),
        }
    }

    /// Appends bytes, discarding the oldest when over capacity.
    pub fn push(&mut self, bytes: &[u8]) {
        if bytes.is_empty() || self.capacity == 0 {
            return;
        }
        self.total_written = self
            .total_written
            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        self.last_write = Some(Timestamp::now());
        self.forward(bytes);
        for chunk in bytes {
            if self.buffer.len() >= self.capacity {
                self.buffer.pop_front();
            }
            self.buffer.push_back(*chunk);
        }
    }

    /// Copies the currently retained bytes.
    #[must_use]
    pub fn snapshot(&self) -> Vec<u8> {
        self.buffer.iter().copied().collect()
    }

    /// Total bytes ever written (retained + discarded).
    #[must_use]
    pub fn total_written(&self) -> u64 {
        self.total_written
    }

    /// When output was last appended.
    #[must_use]
    pub fn last_write(&self) -> Option<Timestamp> {
        self.last_write
    }

    /// Registers a live-output listener; every pushed chunk is forwarded
    /// as its own `Vec<u8>`. Dead listeners (dropped receivers) are
    /// pruned on push.
    pub fn add_listener(&mut self, sender: std::sync::mpsc::Sender<Vec<u8>>) {
        self.listeners.push(sender);
    }

    /// Test hook: whether any listener remains registered.
    #[cfg(test)]
    fn listeners_is_empty_for_test(&self) -> bool {
        self.listeners.is_empty()
    }

    fn forward(&mut self, bytes: &[u8]) {
        if self.listeners.is_empty() {
            return;
        }
        let chunk = bytes.to_vec();
        self.listeners
            .retain(|sender| sender.send(chunk.clone()).is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::OutputRing;

    #[test]
    fn ring_keeps_tail_and_counts_totals() {
        let mut ring = OutputRing::new(4);
        ring.push(b"abcdef");
        assert_eq!(ring.snapshot(), b"cdef");
        assert_eq!(ring.total_written(), 6);
        assert!(ring.last_write().is_some());
    }

    #[test]
    fn listeners_receive_pushed_chunks_and_prune_when_dropped() {
        let mut ring = OutputRing::new(64);
        let (tx, rx) = std::sync::mpsc::channel();
        ring.add_listener(tx);
        ring.push(b"hello");
        ring.push(b" world");
        assert_eq!(rx.try_recv().unwrap(), b"hello".to_vec());
        assert_eq!(rx.try_recv().unwrap(), b" world".to_vec());
        assert!(rx.try_recv().is_err());

        // A dropped receiver is pruned on the next push, not an error.
        drop(rx);
        ring.push(b"more");
        assert!(ring.listeners_is_empty_for_test());
    }

    #[test]
    fn empty_push_is_a_noop() {
        let mut ring = OutputRing::new(4);
        ring.push(b"");
        assert_eq!(ring.snapshot(), b"");
        assert_eq!(ring.total_written(), 0);
        assert!(ring.last_write().is_none());
    }
}
