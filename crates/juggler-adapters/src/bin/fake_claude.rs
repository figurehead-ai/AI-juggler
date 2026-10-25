//! `fake-claude` — a stand-in for Claude Code's headless stream-json mode
//! (AJ-E2308), so the Beta runtime is tested end to end without a real
//! Claude Code or account:
//!
//! ```text
//! fake-claude --version                          2.1.302 (Claude Code)
//! fake-claude -p --input-format stream-json --output-format stream-json
//!             --verbose --permission-prompt-tool stdio
//!             (--session-id ID | --resume ID) [--model M]
//! ```
//!
//! Like Claude Code 2.1.302 it refuses stream-json without `--verbose`
//! ("When using --print, --output-format=stream-json requires --verbose",
//! the lab of 2026-10-25), and it refuses `--bare` and the permission
//! bypasses outright, so a launch carrying one fails its test.
//!
//! On stdin: `control_request`s (`initialize`, `interrupt`, `set_model`),
//! `user` turns and `control_response`s. A turn whose text holds `ask:`
//! asks permission to run a command (with a suggested rule); `question:`
//! asks with `AskUserQuestion`; `slow:` runs until interrupted. Each turn
//! ends with a `rate_limit_event` and a `result`. The conversation is
//! written to `CLAUDE_CONFIG_DIR/projects/<folder>/<id>.jsonl` as Claude
//! Code writes it, so the readers find and resume it. It exits when stdin
//! closes.

#[path = "support/stream.rs"]
mod stream;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-v") {
        let version = std::env::var("FAKE_CLAUDE_VERSION").unwrap_or_else(|_| "2.1.302".to_owned());
        println!("{version} (Claude Code)");
        return ExitCode::SUCCESS;
    }
    let has = |word: &str| args.iter().any(|a| a == word);
    for refused in [
        "--bare",
        "--dangerously-skip-permissions",
        "bypassPermissions",
    ] {
        if args.iter().any(|a| a.contains(refused)) {
            eprintln!("fake-claude: AI Juggler must never pass {refused}");
            return ExitCode::from(3);
        }
    }
    if has("-p")
        && args
            .windows(2)
            .any(|w| w[0] == "--output-format" && w[1] == "stream-json")
        && !has("--verbose")
    {
        eprintln!("Error: When using --print, --output-format=stream-json requires --verbose");
        return ExitCode::from(1);
    }
    if !has("-p") {
        eprintln!("fake-claude: only the headless stream-json mode is faked");
        return ExitCode::from(2);
    }
    let value_of = |flag: &str| args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone());
    let id = value_of("--resume")
        .or_else(|| value_of("--session-id"))
        .unwrap_or_else(|| "fake-claude-session".to_owned());
    stream::run(&id, value_of("--model"));
    ExitCode::SUCCESS
}
