#![no_main]
//! Fuzz target: the Claude transcript parser and deduper must never panic
//! on arbitrary bytes (QUALITY.md §5 — parsers are the risk surface).
//! Run: `cargo +nightly fuzz run transcript_parse` from `fuzz/`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let parsed = juggler_adapters::claude_transcript::parse_transcript(data);
    let _ = juggler_adapters::claude_transcript::dedupe_messages(&parsed.messages);
});
