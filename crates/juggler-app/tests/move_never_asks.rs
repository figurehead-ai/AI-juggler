//! 0.5.1 F93 (PAT-OSP): a move never asks with its Apps closed. Its
//! system prompts (the link's Automation sheet, `automation-app-links`)
//! are asked on the Move screen and by each Undo button, before anything
//! quits, from the offer's and the move record's `prompts`
//! (`moving_prompts.rs`). The move's own path, `execute` and `undo` and
//! everything they call in `features/handover/moving*.rs`, only ever
//! sends in the background: it never calls the prompt gate's asking side
//! (`os_prompts::request_impl`, a `prime…` call, `Presence::User` or
//! `Presence::Primed`, the primer). In the PL1 walk a sheet asked inside
//! the move held one with both Apps closed for 600 s.
//!
//! The move's own tests prove the order; this scan catches an ask put
//! back inside the move. It reads the product source (not test files,
//! not `#[cfg(test)]` modules, not comments or strings).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test"
)]

use std::path::{Path, PathBuf};

mod common;
use common::source::{code_only, product};

/// The one file of a move that may name the asking side: the offer's and
/// Undo's prompt list, and the background send.
const PROMPTS_FILE: &str = "moving_prompts.rs";

/// Names that ask (or prime without asking): the gate's asking calls in
/// `crate::os_prompts` and `juggler_platform::os_prompt`.
const ASKING: &[&str] = &[
    "request_impl",
    "prime_call",
    "os_prompt_prime",
    "os_prompt_request",
    "primer",
    "Primer",
    "set_primed",
    "ask_accessibility",
    "ask_screen_recording",
];

/// The move's product files: `features/handover/moving*.rs` without the
/// prompts file, test files and the test fixtures.
fn move_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/features/handover");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            name.starts_with("moving")
                && name.ends_with(".rs")
                && name != PROMPTS_FILE
                && name != "moving_fixtures.rs"
                && !name.ends_with("_tests.rs")
        })
        .collect();
    files.sort();
    files
}

/// Each path in `code` (`a::b::c`, spaces and line breaks around `::`
/// allowed) with its 1-based line.
fn paths(code: &str) -> Vec<(usize, Vec<String>)> {
    let bytes = code.as_bytes();
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut out: Vec<(usize, Vec<String>)> = Vec::new();
    let mut open = false; // a path is being read
    let mut joined = false; // and its last thing was `::`
    let mut line = 1;
    let mut at = 0;
    while at < bytes.len() {
        let b = bytes[at];
        if b == b'\n' {
            line += 1;
            at += 1;
        } else if b.is_ascii_whitespace() {
            at += 1;
        } else if b == b':' && bytes.get(at + 1) == Some(&b':') {
            joined = open;
            at += 2;
        } else if word(b) {
            let start = at;
            while at < bytes.len() && word(bytes[at]) {
                at += 1;
            }
            let name = code[start..at].to_owned();
            match out.last_mut() {
                Some((_, path)) if open && joined => path.push(name),
                _ => out.push((line, vec![name])),
            }
            open = true;
            joined = false;
        } else {
            // Anything else ends a path.
            open = false;
            joined = false;
            at += 1;
        }
    }
    out
}

/// Every ask in one file's text, as `line: path`.
fn asks(text: &str) -> Vec<String> {
    let code = code_only(&product(text));
    let mut found = Vec::new();
    for (line, path) in paths(&code) {
        let pair = |a: &str, b: &str| path.windows(2).any(|w| w[0] == a && w[1] == b);
        let asking = path.iter().any(|segment| {
            ASKING.contains(&segment.as_str())
                || (segment.starts_with("prime") && segment != "primed")
        }) || pair("Presence", "User")
            || pair("Presence", "Primed")
            || pair("os_prompt", "user")
            || path.windows(2).any(|w| {
                w[0] == "os_prompts"
                    && (w[1].starts_with("request") || w[1].starts_with("os_prompt_"))
            });
        if asking {
            found.push(format!("{line}: {}", path.join("::")));
        }
    }
    found
}

/// The scan sees code only: each ask is caught, and none of the decoys.
#[test]
fn the_scan_finds_asks_in_code_not_in_words() {
    let caught = [
        "fn execute() { prime_links(state); }",
        "fn undo() { let _ = crate::os_prompts::request_impl(state, entry); }",
        "fn f() { let _ = crate::os_prompts::\n    request_impl(state, entry); }",
        "fn f() { os_prompts::prime_call(state, entry, presence); }",
        "fn f() { send_link(p, Presence::User(os_prompt::primer()), pid, link); }",
        "fn f() { send_link(p, Presence::Primed, pid, link); }",
        "fn f() { send_link(p, os_prompt::user(), pid, link); }",
        "fn f() { gate.set_primed(id); }",
        "use crate::os_prompts::{registry, request_impl as ask};",
    ];
    for src in caught {
        assert!(!asks(src).is_empty(), "missed: {src}");
    }
    let decoys = [
        "// prime_links(state);\nfn f() {}",
        "/// Asks with `os_prompts::request_impl` on the Move screen.\nfn f() {}",
        "fn f() { let _ = \"prime_links(state)\"; }",
        "fn f() { send_link(APP_LINKS, Presence::Background, pid, link); }",
        "fn f() { let _ = prompts::unprimed_now(state, true); gate.primed(id); }",
        "fn f() { sides.request.notes = notes; }",
        "#[cfg(test)]\nmod tests {\n    fn t() { prime_links(state); }\n}\nfn f() {}",
    ];
    for src in decoys {
        assert_eq!(asks(src), Vec::<String>::new(), "{src}");
    }
}

/// PAT-OSP, F93: never ask with the Apps closed. A move's `execute` and
/// `undo` paths (`features/handover/moving*.rs` but `moving_prompts.rs`)
/// call nothing that asks; its prompts are asked on the Move screen and
/// by Undo's button before anything quits.
#[test]
fn a_move_never_asks_with_its_apps_closed() {
    let files = move_files();
    let names: Vec<String> = files
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    for needed in ["moving.rs", "moving_undo.rs", "moving_real.rs"] {
        assert!(
            names.iter().any(|name| name == needed),
            "{needed} moved: point this scan at the move's execute and undo files ({names:?})"
        );
    }
    let mut found = Vec::new();
    for (path, name) in files.iter().zip(&names) {
        let text = std::fs::read_to_string(path).unwrap();
        found.extend(asks(&text).into_iter().map(|ask| format!("{name}:{ask}")));
    }
    assert!(
        found.is_empty(),
        "PAT-OSP, F93: never ask with the Apps closed. A move's execute and undo \
         paths call the system-prompt gate's asking side here:\n  {}\n\
         Ask on the Move screen or Undo's button before anything quits: list the \
         prompt in the offer's or the move record's `prompts` ({PROMPTS_FILE}), and \
         send in the background only (`Presence::Background`).",
        found.join("\n  ")
    );
}
