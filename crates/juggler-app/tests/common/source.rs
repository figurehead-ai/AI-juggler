//! Reading the workspace's own Rust source in tests: the files, the code
//! without its test modules or comments, and its functions. Shared by the
//! source-scanning guardrails (`no_bare_tools`, `trust`).

use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, appended to `out`.
pub fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The product part of a source file: every `#[cfg(test)] mod … { … }`
/// cut out, wherever it is, and the code after it kept. Braces inside
/// strings, chars and comments don't count.
pub fn product(text: &str) -> String {
    const MARK: &str = "#[cfg(test)]\nmod ";
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(MARK) {
        out.push_str(&rest[..at]);
        let module = &rest[at..];
        let Some(open) = module.find('{') else {
            return out; // `mod tests;`-style: nothing inline to skip
        };
        rest = &module[block_end(module, open)..];
    }
    out.push_str(rest);
    out
}

/// The byte just past the block whose `{` is at `open`.
pub fn block_end(text: &str, open: usize) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut at = open;
    while at < bytes.len() {
        match bytes[at] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1;
                }
            }
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                at = text[at..].find('\n').map_or(bytes.len(), |end| at + end);
            }
            b'r' if matches!(bytes.get(at + 1), Some(b'"' | b'#'))
                && !ident(bytes, at)
                && raw_string_at(bytes, at) =>
            {
                let hashes = bytes[at + 1..].iter().take_while(|b| **b == b'#').count();
                let close = format!("\"{}", "#".repeat(hashes));
                let body = at + 2 + hashes;
                at = text[body..]
                    .find(&close)
                    .map_or(bytes.len(), |end| body + end + close.len())
                    - 1;
            }
            b'"' => {
                at += 1;
                while at < bytes.len() && bytes[at] != b'"' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
            }
            b'\'' if bytes.get(at + 2) == Some(&b'\'') => at += 2,
            b'\'' if bytes.get(at + 1) == Some(&b'\\') => {
                at = text[at + 2..]
                    .find('\'')
                    .map_or(bytes.len(), |end| at + 2 + end);
            }
            _ => {}
        }
        at += 1;
    }
    bytes.len()
}

/// Whether `r` at `at` opens a raw string (`r"…"`, `r#"…"#`), not a raw
/// identifier (`r#override`).
pub fn raw_string_at(bytes: &[u8], at: usize) -> bool {
    let hashes = bytes[at + 1..].iter().take_while(|b| **b == b'#').count();
    bytes.get(at + 1 + hashes) == Some(&b'"')
}

/// Whether the byte before `at` continues an identifier (`for` in
/// `for"…"` isn't a raw string).
pub fn ident(bytes: &[u8], at: usize) -> bool {
    at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_')
}

/// The code without its comments (a name in a comment isn't a use).
pub fn without_comments(code: &str) -> String {
    code.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                ""
            } else {
                line.split(" // ").next().unwrap_or(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A file's functions: name and body (bodiless declarations skipped).
pub fn functions(code: &str) -> Vec<(String, &str)> {
    let mut out = Vec::new();
    let mut rest = code;
    while let Some(at) = rest.find("fn ") {
        let after = &rest[at + 3..];
        if at > 0 && ident(rest.as_bytes(), at) {
            rest = after;
            continue; // part of a longer word
        }
        let function: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        // The signature ends at its body's `{`, or at `;` when it has none
        // (a trait's declaration).
        let Some(open) = after.find(['{', ';']) else {
            break;
        };
        if after.as_bytes()[open] == b';' {
            rest = &after[open + 1..];
            continue;
        }
        let end = block_end(after, open);
        if !function.is_empty() {
            out.push((function, &after[..end]));
        }
        rest = &after[open + 1..];
    }
    out
}

/// `code` with every comment and the inside of every string, raw string
/// and char blanked to spaces, so a scan for calls or names sees only
/// code. Lengths and line breaks stay, so offsets still line up.
pub fn code_only(code: &str) -> String {
    let bytes = code.as_bytes();
    let mut out = bytes.to_vec();
    let mut at = 0;
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in &mut out[from..to.min(bytes.len())] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    while at < bytes.len() {
        match bytes[at] {
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                let end = code[at..].find('\n').map_or(bytes.len(), |end| at + end);
                blank(&mut out, at, end);
                at = end;
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                let end = code[at + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |end| at + 2 + end + 2);
                blank(&mut out, at, end);
                at = end;
            }
            b'r' if matches!(bytes.get(at + 1), Some(b'"' | b'#'))
                && !ident(bytes, at)
                && raw_string_at(bytes, at) =>
            {
                let hashes = bytes[at + 1..].iter().take_while(|b| **b == b'#').count();
                let close = format!("\"{}", "#".repeat(hashes));
                let body = at + 2 + hashes;
                let end = code[body..]
                    .find(&close)
                    .map_or(bytes.len(), |end| body + end);
                blank(&mut out, body, end);
                at = end + close.len();
            }
            b'"' => {
                let mut end = at + 1;
                while end < bytes.len() && bytes[end] != b'"' {
                    end += if bytes[end] == b'\\' { 2 } else { 1 };
                }
                blank(&mut out, at + 1, end);
                at = end + 1;
            }
            b'\'' if bytes.get(at + 2) == Some(&b'\'') => {
                blank(&mut out, at + 1, at + 2);
                at += 3;
            }
            _ => at += 1,
        }
    }
    String::from_utf8(out).unwrap_or_default()
}
