//! AJ-E2016: reading JSONL (and any line format) one bounded line at a
//! time, whatever the tool: a transcript's memory is one line's, whatever
//! its size. `transcript` opens session files (compressed or not) on it.

/// The longest line a streamed read keeps. A longer one is skipped whole,
/// never held: a transcript's memory is one line's, whatever its size.
pub const LINE_MAX: usize = 32 * 1024 * 1024;

/// What a streamed read went through.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LinesRead {
    /// Lines handed to the caller.
    pub lines: usize,
    /// Lines longer than [`LINE_MAX`], skipped.
    pub skipped: usize,
}

/// A reader's lines, each at most `max` bytes: a longer line is skipped
/// whole without ever being held. A read error ends the lines.
pub struct BoundedLines {
    reader: Box<dyn std::io::BufRead + Send>,
    max: usize,
    index: usize,
    done: bool,
    /// What it has gone through so far.
    pub read: LinesRead,
}

impl BoundedLines {
    /// `reader`'s lines, each at most `max` bytes.
    #[must_use]
    pub fn new(reader: Box<dyn std::io::BufRead + Send>, max: usize) -> Self {
        Self {
            reader,
            max,
            index: 0,
            done: false,
            read: LinesRead::default(),
        }
    }
}

impl Iterator for BoundedLines {
    type Item = (usize, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = Vec::new();
        let mut too_long = false;
        while !self.done {
            let Ok(buf) = self.reader.fill_buf() else {
                self.done = true;
                break;
            };
            if buf.is_empty() {
                self.done = true;
                // The last line, when the file doesn't end with a newline.
                if too_long {
                    self.read.skipped += 1;
                    return None;
                }
                if line.is_empty() {
                    return None;
                }
                self.read.lines += 1;
                return Some((self.index, line));
            }
            let (part, ends) = match buf.iter().position(|byte| *byte == b'\n') {
                Some(at) => (&buf[..at], Some(at + 1)),
                None => (buf, None),
            };
            if !too_long {
                if line.len() + part.len() <= self.max {
                    line.extend_from_slice(part);
                } else {
                    too_long = true;
                    line = Vec::new();
                }
            }
            let used = ends.unwrap_or(buf.len());
            self.reader.consume(used);
            if ends.is_some() {
                let index = self.index;
                self.index += 1;
                if too_long {
                    self.read.skipped += 1;
                    too_long = false;
                    continue;
                }
                self.read.lines += 1;
                return Some((index, line));
            }
        }
        None
    }
}
