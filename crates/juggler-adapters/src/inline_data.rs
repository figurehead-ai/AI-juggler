//! Inline binary data left out of a transcript line as it is held
//! (ADR-0024): a screenshot pasted into a conversation is kept in its
//! line as base64 (Claude Code: an `image` or `document` part whose
//! `source` has `"data": "<base64>"`; Codex: an `input_image` part whose
//! `image_url` is a `data:image/png;base64,…` URL), so one message can
//! pass the longest line a reader holds and vanish, its words with it.
//! Nothing that reads these lines draws the bytes (images are counted by
//! their parts), so they are left out as the line is held.
//!
//! [`InlineData`] reads a line's bytes as they come, in pieces split
//! anywhere (inside a string, an escape or a key), and copies them on,
//! except the content of a string value that is inline data once it
//! passes [`KEEP_MAX`] bytes: the value of a `data` key that is all
//! base64 is left empty (`"data":""`), a `data:` URL keeps its header
//! only (`"data:image/png;base64,"`). The part stays, with its type and
//! media type, so it is still counted and the line still parses. Any
//! other string, however long, is kept: what is too long then is the
//! reader's to pass over (huge text, huge tool output).
//!
//! The decision is made once, on a string's first [`KEEP_MAX`] bytes;
//! after it the rest of that string is left out up to its closing quote,
//! whatever it holds. So what a line holds of one string is at most
//! [`KEEP_MAX`] bytes past what it keeps, and the image's bytes are
//! never held whole. The state is a few flags and one index: a line read
//! in any number of pieces is filtered as one read whole.

use serde::{Deserialize, Serialize};

/// The longest string content kept before it is judged: past this, a
/// string that is inline data has its content left out.
pub(crate) const KEEP_MAX: usize = 64 * 1024;

/// How far into a string a `data:` URL's `;base64,` is looked for.
const HEADER_MAX: usize = 256;

/// A line being filtered: where its reading is (inside a string, after
/// an escape, after a key) and what the string being read is.
#[allow(clippy::struct_excessive_bools)] // a lexer's flags, one state
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InlineData {
    /// Inside a string.
    in_string: bool,
    /// Inside a string, right after a `\`.
    escaped: bool,
    /// Outside strings, the last byte read (past whitespace) was `:`.
    after_colon: bool,
    /// The last string read was `data` and nothing but whitespace or `:`
    /// came after it: the next string may be its value.
    after_data_key: bool,
    /// The string being read is the value of a `data` key.
    data_value: bool,
    /// Where the string being read starts in what is held (after its
    /// opening quote).
    from: usize,
    /// The string being read was judged (its first [`KEEP_MAX`] bytes).
    judged: bool,
    /// The rest of the string being read is left out.
    dropping: bool,
}

/// What [`InlineData::pass`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Held {
    /// The bytes were taken in.
    Within,
    /// What is held would pass the bound: the line is too long to hold.
    TooLong,
}

impl InlineData {
    /// Appends `input`, the next bytes of a line, to `out` (what is held
    /// of it so far, read by this filter from the line's start), leaving
    /// out inline data. [`Held::TooLong`] when `out` would pass `cap`
    /// bytes: nothing past it was added.
    pub(crate) fn pass(&mut self, input: &[u8], out: &mut Vec<u8>, cap: usize) -> Held {
        let mut rest = input;
        while let Some(&byte) = rest.first() {
            let took = if !self.in_string {
                self.outside(rest, out, cap)
            } else if self.escaped {
                self.escaped = false;
                self.content(&rest[..1], out, cap)
            } else if byte == b'\\' {
                self.escaped = true;
                self.content(&rest[..1], out, cap)
            } else if byte == b'"' {
                self.close(out, cap)
            } else {
                let run = rest
                    .iter()
                    .position(|b| *b == b'"' || *b == b'\\')
                    .unwrap_or(rest.len());
                // Up to the judging point first, so a string is judged
                // on exactly its first bytes past the bound.
                let run = if self.judged || self.dropping {
                    run
                } else {
                    run.min(KEEP_MAX + 1 - (out.len() - self.from).min(KEEP_MAX))
                };
                self.content(&rest[..run], out, cap)
            };
            match took {
                Some(count) => rest = &rest[count..],
                None => return Held::TooLong,
            }
        }
        Held::Within
    }

    /// Outside strings: structure, numbers and literals up to the next
    /// string, which it opens. How many bytes it took.
    fn outside(&mut self, rest: &[u8], out: &mut Vec<u8>, cap: usize) -> Option<usize> {
        let run = rest.iter().position(|b| *b == b'"');
        let plain = &rest[..run.unwrap_or(rest.len())];
        for byte in plain {
            match byte {
                b':' => self.after_colon = true,
                b' ' | b'\t' | b'\r' | b'\n' => {}
                _ => {
                    self.after_colon = false;
                    self.after_data_key = false;
                }
            }
        }
        keep(out, plain, cap)?;
        if run.is_none() {
            return Some(plain.len());
        }
        keep(out, b"\"", cap)?;
        self.data_value = self.after_colon && self.after_data_key;
        self.after_colon = false;
        self.after_data_key = false;
        self.in_string = true;
        self.judged = false;
        self.dropping = false;
        self.from = out.len();
        Some(plain.len() + 1)
    }

    /// Bytes of a string's content: kept, or left out once it is judged
    /// inline data. How many bytes it took.
    fn content(&mut self, bytes: &[u8], out: &mut Vec<u8>, cap: usize) -> Option<usize> {
        if self.dropping {
            return Some(bytes.len());
        }
        keep(out, bytes, cap)?;
        if !self.judged && out.len() - self.from > KEEP_MAX {
            self.judged = true;
            if let Some(kept) = inline_data(&out[self.from..], self.data_value) {
                out.truncate(self.from + kept);
                self.dropping = true;
            }
        }
        Some(bytes.len())
    }

    /// A string's closing quote.
    fn close(&mut self, out: &mut Vec<u8>, cap: usize) -> Option<usize> {
        self.after_data_key = !self.dropping && out.get(self.from..) == Some(b"data".as_slice());
        keep(out, b"\"", cap)?;
        self.in_string = false;
        self.data_value = false;
        self.dropping = false;
        Some(1)
    }
}

/// Appends `bytes` to `out` unless that passes `cap`.
fn keep(out: &mut Vec<u8>, bytes: &[u8], cap: usize) -> Option<()> {
    if out.len() + bytes.len() > cap {
        return None;
    }
    out.extend_from_slice(bytes);
    Some(())
}

/// How much of `content`, a string's first bytes (raw, escapes as
/// written), to keep when it is inline data: the header of a `data:`
/// URL whose payload is base64, nothing of a `data` key's base64 value.
/// `None`: it isn't inline data.
fn inline_data(content: &[u8], data_value: bool) -> Option<usize> {
    if let Some(after) = content.strip_prefix(b"data:") {
        let window = &after[..after.len().min(HEADER_MAX)];
        let header = window.windows(8).position(|w| w == b";base64,")?;
        let kept = 5 + header + 8;
        return is_base64(&content[kept..]).then_some(kept);
    }
    (data_value && is_base64(content)).then_some(0)
}

/// Whether `content` (raw JSON string content) is base64: its alphabet,
/// line breaks and `\/` (an escaped `/`) only. A `\` at its very end is
/// the start of an escape still to come.
fn is_base64(content: &[u8]) -> bool {
    let mut bytes = content.iter();
    while let Some(byte) = bytes.next() {
        let fine = match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=' => true,
            b'\\' => matches!(bytes.next(), None | Some(b'/' | b'n' | b'r')),
            _ => false,
        };
        if !fine {
            return false;
        }
    }
    true
}
