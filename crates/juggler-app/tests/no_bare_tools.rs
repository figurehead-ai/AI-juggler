//! SECURITY-GUIDELINES C2 (AJ-E2010): no system tool is run by bare name.
//! The app runs with the user's `PATH` in front, so a tool run by name could
//! be a stand-in earlier on it. Every system tool is named by absolute path
//! in `juggler_adapter::system_tools`. This scans the product source (not
//! tests, not `#[cfg(test)]` modules) and fails on:
//! - `Command::new("<bare name>")`;
//! - in a file that spawns, a known system tool's name on a line with
//!   `Command`, or alone on a line (the `let opener = if … { "open" }` shape);
//! - a known system tool looked up on `PATH` (`user_path::find("git")`).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};

/// Names that must never be spawned bare.
const TOOLS: &[&str] = &[
    "security",
    "open",
    "xdg-open",
    "osascript",
    "git",
    "ps",
    "vm_stat",
    "sysctl",
    "lsof",
    "pmset",
    "launchctl",
    "defaults",
    "df",
    "diskutil",
    "ioreg",
    "pgrep",
    "pkill",
    "kill",
    "codesign",
    "ditto",
    "tar",
    "hdiutil",
    "plutil",
    "sh",
    "zsh",
    "bash",
    "env",
    "which",
    "uname",
    "sw_vers",
    "explorer",
];

/// Bare spawns that are allowed, by file and name: test fakes that spawn
/// what their caller names.
const ALLOWED: &[(&str, &str)] = &[];

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
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
fn product(text: &str) -> String {
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
fn block_end(text: &str, open: usize) -> usize {
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
            b'r' if matches!(bytes.get(at + 1), Some(b'"' | b'#')) && !ident(bytes, at) => {
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

/// Whether the byte before `at` continues an identifier (`for` in
/// `for"…"` isn't a raw string).
fn ident(bytes: &[u8], at: usize) -> bool {
    at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_')
}

/// A test module in the middle of a file no longer hides the code after
/// it (#126 review).
#[test]
fn the_scan_reads_past_a_test_module() {
    let text = "fn a() {}\n#[cfg(test)]\nmod tests {\n    fn t() { let s = \"}\"; let c = '}'; let r = r#\"}\"#; }\n    // }\n}\nfn b() { Command::new(\"open\"); }\n";
    let code = product(text);
    assert!(code.contains("fn a()"));
    assert!(!code.contains("fn t()"));
    assert!(code.contains("Command::new(\"open\")"), "{code}");
}

#[test]
fn no_system_tool_is_spawned_by_bare_name() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap().flatten() {
        let src = krate.path().join("src");
        if src.is_dir() {
            sources(&src, &mut files);
        }
    }
    assert!(files.len() > 50, "the scan found the sources");
    let bare = regex_lite_bare();
    let mut found = Vec::new();
    for file in files {
        let name = file.to_string_lossy().replace('\\', "/");
        if name.ends_with("system_tools.rs") || name.contains("/bin/fake_") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        let code = product(&text);
        let spawns = code.contains("Command::new(");
        for tool in TOOLS {
            if code.contains(&format!("find(\"{tool}\")")) {
                found.push(format!("{name}: {tool} looked up on PATH"));
            }
        }
        for (number, line) in code.lines().enumerate() {
            let at = format!("{name}:{}", number + 1);
            if let Some(program) = bare(line) {
                if !ALLOWED
                    .iter()
                    .any(|(f, p)| name.ends_with(f) && *p == program)
                {
                    found.push(format!("{at}: Command::new(\"{program}\")"));
                }
                continue;
            }
            if !spawns {
                continue;
            }
            let trimmed = line.trim();
            for tool in TOOLS {
                let quoted = format!("\"{tool}\"");
                if (line.contains("Command") && line.contains(&quoted)) || trimmed == quoted {
                    found.push(format!("{at}: {trimmed}"));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "run these by absolute path (juggler_adapter::system_tools):\n{}",
        found.join("\n")
    );
}

/// `Command::new("<name>")` with a name that isn't an absolute path.
fn regex_lite_bare() -> impl Fn(&str) -> Option<String> {
    |line: &str| {
        let start = line.find("Command::new(\"")? + "Command::new(\"".len();
        let rest = &line[start..];
        let end = rest.find('"')?;
        let program = &rest[..end];
        (!program.starts_with('/')).then(|| program.to_owned())
    }
}

/// On Linux, sysinfo lists every thread as a process unless told not to
/// (`ProcessRefreshKind::nothing()` keeps tasks): a dev server's serving
/// thread read as a second orphan (#116's CI). Every process refresh in
/// the product leaves threads out.
#[test]
fn every_process_refresh_leaves_out_threads() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap().flatten() {
        let src = krate.path().join("src");
        if src.is_dir() {
            sources(&src, &mut files);
        }
    }
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let code: String = product(&text).split_whitespace().collect();
        let mut rest = code.as_str();
        while let Some(at) = rest.find("ProcessRefreshKind::nothing()") {
            let after = &rest[at + "ProcessRefreshKind::nothing()".len()..];
            if !after.starts_with(".without_tasks()") {
                found.push(file.to_string_lossy().into_owned());
            }
            rest = after;
        }
    }
    assert!(
        found.is_empty(),
        "add .without_tasks():\n{}",
        found.join("\n")
    );
}
