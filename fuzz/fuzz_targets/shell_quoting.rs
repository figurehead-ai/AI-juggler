#![no_main]
//! Fuzz target: shell quoting (AJ-E2008). Any value quoted for a POSIX
//! shell reads back as exactly that value: one single-quoted word, with
//! each `'` written `'\''`. Run: `cargo +nightly fuzz run shell_quoting`.

use libfuzzer_sys::fuzz_target;

/// How a POSIX shell reads a word made of '…' runs joined by `\'`.
fn shell_reads(word: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = word;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("\\'") {
            out.push('\'');
            rest = after;
        } else if let Some(body) = rest.strip_prefix('\'') {
            let end = body.find('\'')?;
            out.push_str(&body[..end]);
            rest = &body[end + 1..];
        } else {
            return None;
        }
    }
    Some(out)
}

fuzz_target!(|data: &[u8]| {
    let Ok(value) = std::str::from_utf8(data) else { return };
    let quoted = juggler_adapter::adapter::shell_quote(value);
    assert_eq!(shell_reads(&quoted).as_deref(), Some(value), "{quoted}");
});
