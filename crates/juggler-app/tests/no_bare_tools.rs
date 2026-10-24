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

use std::path::Path;

mod common;
use common::source::{functions, product, sources, without_comments};

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

/// The files a tool keeps its sign-in secrets in (ADR-0003, SECURITY):
/// named only in `juggler-adapters/src/sign_in.rs`.
const SIGN_IN_FILES: &[&str] = &[
    "\".credentials.json\"",
    "\"auth.json\"",
    "\"mcp-auth.json\"",
    "\"kimi-code.json\"",
    "\"mcp_credentials.json\"",
];

/// The module that owns them, and its name constants.
const SIGN_IN_MODULE: &str = "juggler-adapters/src/sign_in.rs";
const SIGN_IN_CONSTANTS: &[&str] = &[
    "CLAUDE_CREDENTIALS",
    "CODEX_AUTH",
    "OPENCODE_AUTH",
    "OPENCODE_MCP_AUTH",
    "KIMI_CREDENTIALS",
    "GROK_AUTH",
    "GROK_MCP_AUTH",
    "KIMI_MEMBERSHIP",
];

/// Its reads: each streams `auth.json` into a presence-only shape (for
/// OpenCode, each entry's kind).
const SIGN_IN_READS: &[&str] = &[
    "codex_auth",
    "codex_account_id",
    "opencode_auth",
    "grok_auth",
];

/// Claude's `settings.json` can hold an API key (`apiKey`, `primaryApiKey`,
/// `env.ANTHROPIC_API_KEY`): read only here, each for a reason.
const SETTINGS_FILES: &[&str] = &["\"settings.json\""];
const SETTINGS_READS: &[(&str, &str)] = &[
    // Presence-only shape: whether an API-key field is filled.
    (
        "juggler-adapters/src/claude_code.rs",
        "settings_name_an_api_key",
    ),
    // Narrow shape: `statusLine.command` only.
    (
        "juggler-adapters/src/quota_ingest.rs",
        "ensure_launch_overlay",
    ),
    // Rewritten whole with only `statusLine` changed (atomic); the other
    // values pass through, never stored or logged.
    // Narrow shape: `enabledPlugins` only (the switch's set-up check,
    // AJ-E1542).
    ("juggler-adapters/src/claude_inventory.rs", "inventory"),
    // Key by key (`jsonc::value_text`): `permissions` and `statusLine`
    // only, a value with a secret written in it never kept (AJ-E1508).
    ("juggler-adapters/src/claude_inventory.rs", "own_settings"),
    ("juggler-adapters/src/quota_ingest.rs", "install_statusline"),
    (
        "juggler-adapters/src/quota_ingest.rs",
        "uninstall_statusline",
    ),
];

/// What opens a file for its contents (a copy, or `cat`, reads it too).
const READS: &[&str] = &[
    "read_to_string(",
    "fs::read(",
    "File::open(",
    "OpenOptions",
    "from_reader(",
    "fs::copy(",
    "io::copy(",
    "tokio::fs",
    "Command::new(",
];

/// The functions a body hands a sign-in file's path to: `let x = ….join(
/// "<secret>")` and then `f(&x` / `f(x`.
fn handed_the_path(body: &str, files: &[&str]) -> Vec<String> {
    let mut callees = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        let Some(binding) = line.strip_prefix("let ") else {
            continue;
        };
        if !files.iter().any(|file| line.contains(file)) {
            continue;
        }
        let name: String = binding
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        for pattern in [
            format!("(&{name}"),
            format!("({name})"),
            format!("({name},"),
        ] {
            let mut rest = body;
            while let Some(at) = rest.find(&pattern) {
                let callee: String = rest[..at]
                    .chars()
                    .rev()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if !callee.is_empty() {
                    callees.push(callee);
                }
                rest = &rest[at + pattern.len()..];
            }
        }
    }
    callees
}

/// The docs fact-check (fix batch 4) and #130's review: AI Juggler never
/// opens a sign-in file for its contents (Claude's `.credentials.json` was
/// read for an expiry). Structural: the names live in one module that
/// hands out no path, and this fails when
/// - a sign-in file's name appears anywhere else in the product;
/// - a function outside that module uses its name constants (they're for
///   the copy rules' lists);
/// - that module reads other than through its presence-only shapes;
/// - a function names `settings.json`, or is handed its path, and reads
///   it, other than the ones listed with their reasons.
#[test]
fn no_sign_in_file_is_read() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap().flatten() {
        let src = krate.path().join("src");
        if src.is_dir() {
            sources(&src, &mut files);
        }
    }
    let reads = |body: &str| READS.iter().any(|read| body.contains(read));
    let mut found = Vec::new();
    for file in files {
        let name = file.to_string_lossy().replace('\\', "/");
        let code = without_comments(&product(&std::fs::read_to_string(&file).unwrap()));
        let owner = name.ends_with(SIGN_IN_MODULE);
        if !owner && SIGN_IN_FILES.iter().any(|secret| code.contains(secret)) {
            found.push(format!(
                "{name}: names a sign-in file (only {SIGN_IN_MODULE} may)"
            ));
        }
        let all = functions(&code);
        for (function, body) in &all {
            if !owner
                && SIGN_IN_CONSTANTS
                    .iter()
                    .any(|constant| body.contains(constant))
            {
                found.push(format!(
                    "{name}: fn {function} uses a sign-in name constant"
                ));
            }
            if owner && reads(body) && !SIGN_IN_READS.contains(&function.as_str()) {
                found.push(format!("{name}: fn {function} reads in the sign-in module"));
            }
            let settings_allowed = |func: &str| {
                SETTINGS_READS
                    .iter()
                    .any(|(f, allowed)| name.ends_with(f) && *allowed == func)
            };
            if SETTINGS_FILES.iter().any(|file| body.contains(file))
                && reads(body)
                && !settings_allowed(function)
            {
                found.push(format!("{name}: fn {function} reads settings.json"));
            }
            for callee in handed_the_path(body, SETTINGS_FILES) {
                if let Some((_, callee_body)) = all.iter().find(|(f, _)| *f == callee)
                    && reads(callee_body)
                    && !settings_allowed(&callee)
                {
                    found.push(format!(
                        "{name}: fn {callee} reads settings.json (handed it by {function})"
                    ));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "sign-in files and settings.json are read only where listed (ADR-0003):\n{}",
        found.join("\n")
    );
}
