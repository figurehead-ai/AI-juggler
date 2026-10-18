//! `fake-desktop` — the scriptable stand-in desktop app used by the
//! AJ-E413 lifecycle fixtures.
//!
//! It behaves like a minimal windowed harness at the PROCESS level
//! (pid + long-running + clean TERM handling) without drawing a real
//! window (true window embedding is impossible on macOS anyway — the
//! panel is the surface):
//!
//! ```text
//! fake-desktop [--marker FILE] [--exit-after-secs N] [--crash]
//!              [--spawn-child PROG --child-args "A B"]
//! ```
//!
//! `--marker FILE` writes `running <pid>` once up (the tracked evidence);
//! `--exit-after-secs` exits 0; `--crash` exits 7 after the window;
//! TERM is handled cleanly (exit 0) — the crash fixture uses a code.

use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut marker: Option<PathBuf> = None;
    let mut exit_after = 3600_u64;
    let mut crash = false;
    let mut child: Option<String> = None;
    let mut child_args = String::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        let value = || args.get(i + 1).cloned().unwrap_or_default();
        match arg.as_str() {
            "--marker" => {
                marker = Some(PathBuf::from(value()));
                i += 2;
            }
            "--exit-after-secs" => {
                exit_after = value().parse().unwrap_or(3600);
                i += 2;
            }
            "--crash" => {
                crash = true;
                i += 1;
            }
            "--spawn-child" => {
                child = Some(value());
                i += 2;
            }
            "--child-args" => {
                child_args = value();
                i += 2;
            }
            _ => i += 1,
        }
    }

    if let Some(path) = &marker
        && let Ok(mut file) = std::fs::File::create(path)
    {
        let _ = writeln!(file, "running {}", std::process::id());
    }

    // A leftover child fixture (the orphan case for desktop apps).
    if let Some(program) = child {
        let _ = std::process::Command::new(program)
            .args(child_args.split_whitespace())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    let deadline = Instant::now() + Duration::from_secs(exit_after);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(100));
    }
    std::process::exit(if crash { 7 } else { 0 });
}
