//! `fake-harness run --hooks-config FILE` (AJ-E2306): the fake tool runs
//! the command FILE names as its hooks, the way Claude Code runs the hooks
//! of its `--settings` file: `<command> hook <event>` with the event's JSON
//! on stdin, waiting for it (at most 5 s) before going on.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Runs the configured hook for `event` with `input`, if one is configured.
pub fn fire(config: Option<&Path>, event: &str, input: &Value) {
    let Some(config) = config else { return };
    let Some(command) = std::fs::read_to_string(config)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.get("command")?.as_str().map(str::to_owned))
    else {
        return;
    };
    let Ok(mut child) = Command::new(command)
        .args(["hook", event])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.to_string().as_bytes());
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        // poll: the fake's hook ends within its 5 s deadline, then stops.
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
}
