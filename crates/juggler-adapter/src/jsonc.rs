//! AJ-E1530 Req.1 (ADR-0018 amendment): one top-level member of a JSON
//! document, found, set and removed in its text, so that every other
//! byte stays as it was: comments, key order, spacing, line endings and
//! trailing commas included. JSON with `//` and `/* */` comments and
//! trailing commas is read (tools accept it in their settings files); a
//! document that isn't that is refused, never guessed at.

use std::ops::Range;

/// One top-level member: where it starts (its key's opening quote) and
/// its value's span.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Member {
    key: String,
    start: usize,
    value: Range<usize>,
}

/// The root object: its braces and members, in order.
#[derive(Debug)]
struct Root {
    open: usize,
    members: Vec<Member>,
}

/// Reads `text` as a JSON object (comments and trailing commas allowed).
fn root(text: &str) -> Result<Root, String> {
    let mut scan = Scan {
        bytes: text.as_bytes(),
        at: 0,
    };
    scan.skip()?;
    if scan.peek() != Some(b'{') {
        return Err("it isn't a JSON object".into());
    }
    let open = scan.at;
    scan.at += 1;
    let mut members = Vec::new();
    loop {
        scan.skip()?;
        match scan.peek() {
            Some(b'}') => {
                scan.at += 1;
                break;
            }
            Some(b'"') => {
                let start = scan.at;
                let key = scan.string()?;
                scan.skip()?;
                if scan.peek() != Some(b':') {
                    return Err("a key without its value".into());
                }
                scan.at += 1;
                scan.skip()?;
                let value_start = scan.at;
                scan.value()?;
                members.push(Member {
                    key,
                    start,
                    value: value_start..scan.at,
                });
                scan.skip()?;
                match scan.peek() {
                    Some(b',') => scan.at += 1,
                    Some(b'}') => {}
                    _ => return Err("a member not followed by a comma or the end".into()),
                }
            }
            _ => return Err("not a JSON object".into()),
        }
    }
    scan.skip()?;
    if scan.at != scan.bytes.len() {
        return Err("there's more after the object".into());
    }
    Ok(Root { open, members })
}

/// A cursor over the document's bytes.
struct Scan<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Scan<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Past whitespace and comments.
    fn skip(&mut self) -> Result<(), String> {
        loop {
            match (self.peek(), self.bytes.get(self.at + 1)) {
                (Some(b' ' | b'\t' | b'\r' | b'\n'), _) => self.at += 1,
                (Some(b'/'), Some(b'/')) => {
                    while self.peek().is_some_and(|b| b != b'\n') {
                        self.at += 1;
                    }
                }
                (Some(b'/'), Some(b'*')) => {
                    self.at += 2;
                    loop {
                        match (self.peek(), self.bytes.get(self.at + 1)) {
                            (Some(b'*'), Some(b'/')) => {
                                self.at += 2;
                                break;
                            }
                            (Some(_), _) => self.at += 1,
                            (None, _) => return Err("a comment that never ends".into()),
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// A string, returned unescaped (keys are compared by their text).
    fn string(&mut self) -> Result<String, String> {
        let start = self.at;
        self.at += 1;
        loop {
            match self.peek() {
                Some(b'\\') => self.at += 2,
                Some(b'"') => {
                    self.at += 1;
                    let raw = std::str::from_utf8(&self.bytes[start..self.at])
                        .map_err(|_| "text that isn't UTF-8".to_owned())?;
                    return serde_json::from_str(raw).map_err(|_| "a string it can't read".into());
                }
                Some(b'\n') | None => return Err("a string that never ends".into()),
                Some(_) => self.at += 1,
            }
        }
    }

    /// Past one value of any kind.
    fn value(&mut self) -> Result<(), String> {
        match self.peek() {
            Some(b'"') => self.string().map(|_| ()),
            Some(open @ (b'{' | b'[')) => {
                let close = if open == b'{' { b'}' } else { b']' };
                self.at += 1;
                loop {
                    self.skip()?;
                    match self.peek() {
                        Some(b) if b == close => {
                            self.at += 1;
                            return Ok(());
                        }
                        Some(b',' | b':') => self.at += 1,
                        Some(_) => self.value()?,
                        None => return Err("an object or a list that never ends".into()),
                    }
                }
            }
            Some(b'-' | b'0'..=b'9' | b't' | b'f' | b'n') => {
                let start = self.at;
                while self
                    .peek()
                    .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'+' | b'.'))
                {
                    self.at += 1;
                }
                let word = &self.bytes[start..self.at];
                let literal = matches!(word, b"true" | b"false" | b"null");
                let number = std::str::from_utf8(word).is_ok_and(|w| w.parse::<f64>().is_ok());
                if literal || number {
                    Ok(())
                } else {
                    Err("a value it can't read".into())
                }
            }
            _ => Err("a value it can't read".into()),
        }
    }
}

/// Whether `text` reads as a JSON object (comments and trailing commas
/// allowed); the reason in words when it doesn't.
///
/// # Errors
///
/// What's wrong with it, in words.
pub fn check(text: &str) -> Result<(), String> {
    root(text).map(|_| ())
}

/// The text of top-level `key`'s value, as written; `None` when there's
/// no such key.
///
/// # Errors
///
/// When `text` isn't a JSON object.
pub fn value_text(text: &str, key: &str) -> Result<Option<String>, String> {
    Ok(root(text)?
        .members
        .into_iter()
        .find(|member| member.key == key)
        .map(|member| text[member.value].to_owned()))
}

/// The line ending the document uses.
fn newline(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// The indentation of the document's first member, else two spaces.
fn indent(text: &str, root: &Root) -> String {
    root.members
        .first()
        .and_then(|first| {
            let line_start = text[..first.start].rfind('\n').map_or(0, |at| at + 1);
            let lead = &text[line_start..first.start];
            lead.chars()
                .all(|c| c == ' ' || c == '\t')
                .then(|| lead.to_owned())
        })
        .filter(|lead| !lead.is_empty())
        .unwrap_or_else(|| "  ".to_owned())
}

/// Sets top-level `key` to `value` (JSON text). An existing value is
/// replaced in place; a missing key is added after the last member, in
/// the document's own indentation and line ending. Returns the new text
/// and the value it had before (its text as written).
///
/// # Errors
///
/// When `text` isn't a JSON object.
pub fn set(text: &str, key: &str, value: &str) -> Result<(String, Option<String>), String> {
    let root = root(text)?;
    if let Some(member) = root.members.iter().find(|member| member.key == key) {
        let old = text[member.value.clone()].to_owned();
        let mut out = String::with_capacity(text.len() + value.len());
        out.push_str(&text[..member.value.start]);
        out.push_str(value);
        out.push_str(&text[member.value.end..]);
        return Ok((out, Some(old)));
    }
    let nl = newline(text);
    let pad = indent(text, &root);
    let name = serde_json::to_string(key).map_err(|error| error.to_string())?;
    let (at, insert) = match root.members.last() {
        Some(last) => (last.value.end, format!(",{nl}{pad}{name}: {value}")),
        None => (root.open + 1, format!("{nl}{pad}{name}: {value}{nl}")),
    };
    let mut out = String::with_capacity(text.len() + insert.len());
    out.push_str(&text[..at]);
    out.push_str(&insert);
    out.push_str(&text[at..]);
    Ok((out, None))
}

/// Takes AI Juggler's `ours` out of top-level `key` when it's still
/// there: back to `original` (its text as it was), or the member removed
/// when there was none. `None` when `key` no longer holds `ours` (the
/// user changed it: it's theirs, and left alone).
///
/// # Errors
///
/// When `text` isn't a JSON object.
pub fn unset(
    text: &str,
    key: &str,
    ours: &str,
    original: Option<&str>,
) -> Result<Option<String>, String> {
    let root = root(text)?;
    let Some(index) = root.members.iter().position(|member| member.key == key) else {
        return Ok(None);
    };
    let member = &root.members[index];
    if text[member.value.clone()] != *ours {
        return Ok(None);
    }
    if let Some(original) = original {
        return Ok(Some(format!(
            "{}{original}{}",
            &text[..member.value.start],
            &text[member.value.end..]
        )));
    }
    // From the end of the member before it (its separator goes with it),
    // else from its own start to the next member.
    let cut = match (
        index.checked_sub(1).map(|i| &root.members[i]),
        root.members.get(index + 1),
    ) {
        (Some(before), _) => before.value.end..member.value.end,
        (None, Some(after)) => member.start..after.start,
        (None, None) => {
            // The only member: as `set` adds one to an empty object (its
            // own line), the whole line goes; else the member and a comma.
            let nl = newline(text);
            let lead = &text[root.open + 1..member.start];
            let own_line =
                lead.starts_with(nl) && lead[nl.len()..].chars().all(|c| c == ' ' || c == '\t');
            if own_line && text[member.value.end..].starts_with(nl) {
                root.open + 1..member.value.end + nl.len()
            } else {
                let mut scan = Scan {
                    bytes: text.as_bytes(),
                    at: member.value.end,
                };
                scan.skip()?;
                let end = if scan.peek() == Some(b',') {
                    scan.at + 1
                } else {
                    member.value.end
                };
                member.start..end
            }
        }
    };
    Ok(Some(format!("{}{}", &text[..cut.start], &text[cut.end..])))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const OURS: &str = r#"{"type": "command", "command": "'/data/w.sh'"}"#;

    fn round_trip(text: &str) {
        let (set_text, old) = set(text, "statusLine", OURS).unwrap();
        assert!(check(&set_text).is_ok(), "still JSON: {set_text}");
        assert_eq!(
            value_text(&set_text, "statusLine").unwrap().as_deref(),
            Some(OURS)
        );
        let back = unset(&set_text, "statusLine", OURS, old.as_deref())
            .unwrap()
            .unwrap();
        assert_eq!(back, text, "byte for byte");
    }

    #[test]
    fn set_and_unset_keep_every_other_byte() {
        // Comments, unusual key order, CRLF, an existing status line,
        // trailing commas, braces and comment marks inside strings.
        for text in [
            "{\n  \"model\": \"opus\"\n}\n",
            "{\r\n\t\"zeta\": 1,\r\n\t\"alpha\": [1, 2, {\"x\": \"}\"}]\r\n}\r\n",
            "// my settings\n{\n  /* hooks */ \"hooks\": {\"a\": \"//not a comment\"},\n  \"model\": \"opus\", // trailing\n}\n",
            "{\n    \"statusLine\": {\"type\": \"command\", \"command\": \"~/bin/line.sh\", \"padding\": 0},\n    \"model\": \"x\"\n}",
            "{}",
            "{ \"statusLine\" : \"weird\\\"escaped\" }",
        ] {
            round_trip(text);
        }
    }

    #[test]
    fn a_new_member_follows_the_documents_own_layout() {
        let (out, old) = set("{\r\n    \"model\": \"opus\"\r\n}\r\n", "statusLine", "1").unwrap();
        assert_eq!(old, None);
        assert_eq!(
            out,
            "{\r\n    \"model\": \"opus\",\r\n    \"statusLine\": 1\r\n}\r\n"
        );
        let (out, _) = set("{\n  \"a\": 1,\n}\n", "statusLine", "2").unwrap();
        assert_eq!(out, "{\n  \"a\": 1,\n  \"statusLine\": 2,\n}\n");
    }

    #[test]
    fn a_value_the_user_changed_is_left_alone() {
        let (text, _) = set("{\"model\": \"opus\"}", "statusLine", OURS).unwrap();
        let theirs = text.replace(OURS, "{\"type\": \"command\", \"command\": \"mine.sh\"}");
        assert_eq!(unset(&theirs, "statusLine", OURS, None).unwrap(), None);
        // Edited elsewhere: only our member goes.
        let edited = text.replace("\"opus\"", "\"sonnet\"");
        assert_eq!(
            unset(&edited, "statusLine", OURS, None).unwrap().unwrap(),
            "{\"model\": \"sonnet\"}"
        );
    }

    #[test]
    fn what_isnt_a_json_object_is_refused() {
        for bad in [
            "",
            "[1]",
            "{\"a\": }",
            "{\"a\": 1",
            "{\"a\": 1} x",
            "{\"a\": tru}",
            "{/* open",
        ] {
            assert!(check(bad).is_err(), "{bad}");
            assert!(set(bad, "statusLine", "1").is_err(), "{bad}");
        }
    }
}
