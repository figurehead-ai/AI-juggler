//! The agent labs' small helpers: waiting, terminal text, the lab's own
//! processes (pid and name only), and versions.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Polls `done` every 250 ms until it holds or `cap` passes.
pub fn wait_for(cap: Duration, mut done: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    loop {
        if done() {
            return true;
        }
        if started.elapsed() >= cap {
            return false;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Text with terminal escape sequences removed.
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    // CSI: parameters, then one final byte in @..~.
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC: up to BEL or ESC \.
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {
                    chars.next();
                }
            }
            continue;
        }
        if c == '\r' || (c.is_control() && c != '\n' && c != '\t') {
            out.push(' ');
            continue;
        }
        out.push(c);
    }
    out
}

/// Lowercase text without whitespace (a TUI places words by cursor
/// moves, so spaces can be missing or doubled).
pub fn squash(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether `text` mentions `words`, spacing aside.
pub fn says(text: &str, words: &str) -> bool {
    squash(text).contains(&squash(words))
}

/// The processes this test started that still run, as pid and program
/// name only (never a command line or an environment).
pub fn own_processes() -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let me = sysinfo::Pid::from_u32(std::process::id());
    let mut out = Vec::new();
    for (pid, process) in system.processes() {
        let mut parent = process.parent();
        let mut hops = 0;
        while let Some(up) = parent {
            if up == me {
                out.push(format!("{pid} {}", process.name().to_string_lossy()));
                break;
            }
            hops += 1;
            if hops > 32 {
                break;
            }
            parent = system.process(up).and_then(sysinfo::Process::parent);
        }
    }
    out.sort();
    out
}

/// Processes whose command line holds any of `needles` (pid and name only
/// are kept: `pgrep -f` selects, `ps -o pid=,comm=` prints).
pub fn matching_processes(needles: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for needle in needles {
        let Ok(found) = std::process::Command::new("pgrep")
            .env("LC_ALL", "C")
            .args(["-f", needle])
            .output()
        else {
            continue;
        };
        for pid in String::from_utf8_lossy(&found.stdout).split_whitespace() {
            if pid == std::process::id().to_string() {
                continue;
            }
            if let Ok(line) = std::process::Command::new("ps")
                .env("LC_ALL", "C")
                .args(["-o", "pid=,comm=", "-p", pid])
                .output()
            {
                let line = String::from_utf8_lossy(&line.stdout).trim().to_owned();
                if !line.is_empty() {
                    out.push(line);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// How often each of `needles` occurs in one of this test's own
/// children's command line (counts only; the line itself isn't kept).
pub fn argv_counts(pid: u32, needles: &[&str]) -> Value {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut system = System::new();
    let pid = Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    let line = system
        .process(pid)
        .map(|process| {
            process
                .cmd()
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let counts: serde_json::Map<String, Value> = needles
        .iter()
        .map(|needle| ((*needle).to_owned(), json!(line.matches(needle).count())))
        .collect();
    Value::Object(counts)
}

pub fn version(tool: &str) -> String {
    std::process::Command::new(tool)
        .arg("--version")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_default()
}

pub fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// PASS or FAIL.
pub fn verdict(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}
