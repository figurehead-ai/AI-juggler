//! AJ-E3306: the home folder has one way in, `juggler_platform::user::home`
//! (`$HOME` when set). No product code asks the system for it any other
//! way — `dirs::`, `$HOME`/`%USERPROFILE%` read by hand, the user record,
//! `NSHomeDirectory` — so a lab or replica on `HOME=<lab>` can't reach the
//! user's real folders through a path that forgot it. This scans the
//! product source of every crate (not tests, not `#[cfg(test)]` modules).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test"
)]

use std::path::Path;

mod common;
use common::source::{product, sources, without_comments};

/// Ways to the home folder that bypass the helper.
const OTHER_WAYS: &[&str] = &[
    "dirs::home_dir",
    "home::home_dir",
    "env::home_dir",
    "var_os(\"HOME\")",
    "var(\"HOME\")",
    "\"USERPROFILE\"",
    "record_home(",
    "getpwuid",
    "User::from_uid",
    "pw_dir",
    "NSHomeDirectory",
    "homeDirectory",
];

/// The places allowed one, by file and way: the helper itself; the
/// keychain guard, which compares raw `HOME` with the user record (an
/// unset HOME keeps the keychain off); the bridge's lab check, which
/// refuses a lab that is the user record's home; the user's display name
/// (the record's full name, never its home); and the data folder,
/// `dirs::data_dir` (on macOS `$HOME/Library/Application Support`).
const ALLOWED: &[(&str, &str)] = &[
    ("juggler-platform/src/user.rs", "*"),
    ("juggler-platform/src/keychain.rs", "var_os(\"HOME\")"),
    ("juggler-platform/src/keychain.rs", "record_home("),
    ("juggler-app/src/dev_bridge/lab.rs", "record_home("),
    ("juggler-platform/src/system.rs", "User::from_uid"),
];

fn allowed(file: &str, way: &str) -> bool {
    ALLOWED
        .iter()
        .any(|(f, w)| file.ends_with(f) && (*w == "*" || *w == way))
}

/// `dirs::` calls other than the data folder's.
fn other_dirs_call(line: &str) -> Option<String> {
    let at = line.find("dirs::")?;
    let call = &line[at..];
    let name: String = call["dirs::".len()..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    (name != "data_dir").then(|| format!("dirs::{name}"))
}

/// The bypasses in one file's text.
fn bypasses(file: &str, text: &str) -> Vec<String> {
    let code = without_comments(&product(text));
    let mut found = Vec::new();
    for (number, line) in code.lines().enumerate() {
        let at = format!("{file}:{}", number + 1);
        if OTHER_WAYS
            .iter()
            .any(|way| line.contains(way) && !allowed(file, way))
        {
            found.push(format!("{at}: {}", line.trim()));
            continue;
        }
        if let Some(call) = other_dirs_call(line)
            && !line.contains("juggler_adapter::")
            && !line.contains("_dirs::")
            && !allowed(file, &call)
        {
            found.push(format!("{at}: {}", line.trim()));
        }
    }
    found
}

#[test]
fn the_scan_finds_a_home_read_by_hand() {
    let text = "fn a() -> Option<PathBuf> {\n    dirs::home_dir()\n}\nfn b() { std::env::var_os(\"HOME\"); }\n#[cfg(test)]\nmod tests {\n    fn t() { dirs::home_dir(); }\n}\n";
    let found = bypasses("crates/x/src/a.rs", text);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(bypasses("crates/x/src/a.rs", "fn d() { dirs::data_dir(); }").is_empty());
    assert!(
        bypasses(
            "crates/x/src/a.rs",
            "fn h() { juggler_platform::user::home(); }"
        )
        .is_empty()
    );
}

#[test]
fn no_product_code_resolves_home_without_the_helper() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap().flatten() {
        let src = krate.path().join("src");
        if src.is_dir() {
            sources(&src, &mut files);
        }
    }
    assert!(files.len() > 50, "the scan found the sources");
    let mut found = Vec::new();
    for file in files {
        let name = file.to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&file).unwrap();
        found.extend(bypasses(&name, &text));
    }
    assert!(
        found.is_empty(),
        "resolve the home folder with juggler_platform::user::home():\n{}",
        found.join("\n")
    );
}
