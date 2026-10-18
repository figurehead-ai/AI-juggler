//! AJ-E415 real-capture evidence: drives the REAL `ScreenshotSource`
//! (`ScreenCaptureKit`) once. On a machine without the Screen Recording
//! grant this records the honest DENIED path end to end (no prompt:
//! content listing errors cleanly). With the grant (maintainer
//! machine), it captures one real window, prints frame sizes/timings
//! and the CPU delta — the live record. Ignored in CI (no GUI/no
//! grant): run locally with
//! `cargo test -p juggler-app --test preview_real -- --ignored
//! --nocapture`.

#![cfg(target_os = "macos")]
// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_app::preview::{
    CaptureError, CaptureSource, PreviewDriver, PreviewEvent, ScreenshotSource,
};

fn cpu_percent() -> Option<f32> {
    let out = std::process::Command::new("ps")
        .args(["-o", "%cpu=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f32>()
        .ok()
}

/// Launches TextEdit with the probe document and waits for the
/// window to settle — permission-free (a System Events window-name
/// poll needs Accessibility; `pgrep` + a settle delay does not).
/// Returns the doc path; cleanup quits TextEdit ONLY when this test
/// started it, else leaves the app as it was found.
fn launch_probe_window() -> Option<(std::path::PathBuf, bool, Option<u32>)> {
    let was_running = std::process::Command::new("pgrep")
        .arg("-x")
        .arg("TextEdit")
        .status()
        .is_ok_and(|code| code.success());
    let doc = std::env::temp_dir().join("aij-e415-probe");
    std::fs::write(&doc, "AJ-E415 real-capture probe\n").ok()?;
    let _ = std::process::Command::new("open").arg(&doc).status();
    // The titled window (named after the file) appears with the app.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let running = std::process::Command::new("pgrep")
            .arg("-x")
            .arg("TextEdit")
            .status()
            .is_ok_and(|code| code.success());
        if running {
            std::thread::sleep(Duration::from_millis(1_500));
            let pid = std::process::Command::new("pgrep")
                .arg("-x")
                .arg("TextEdit")
                .output()
                .ok()
                .and_then(|out| {
                    String::from_utf8_lossy(&out.stdout)
                        .trim()
                        .split('\n')
                        .next()
                        .and_then(|first| first.trim().parse::<u32>().ok())
                });
            return Some((doc, was_running, pid));
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    None
}

fn close_probe_window(doc: &std::path::Path, was_running: bool) {
    if was_running {
        // Best-effort close of just our document (Automation grant
        // permitting); a pre-existing instance is never quit.
        let _ = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"TextEdit\" to close (every document whose name is \"aij-e415-probe.txt\") saving no",
            ])
            .status();
    } else {
        let _ = std::process::Command::new("pkill")
            .arg("-x")
            .arg("TextEdit")
            .status();
    }
    let _ = std::fs::remove_file(doc);
}

#[test]
#[ignore = "drives real ScreenCaptureKit; local evidence run only"]
fn real_capture_record() {
    let source = ScreenshotSource;
    let probe = launch_probe_window();
    if let Some((_, _, Some(pid))) = &probe {
        println!("[record] probe window up: TextEdit pid {pid}");
    }
    if probe.is_none() {
        println!(
            "[record] probe window unavailable — continuing with the denied/idle enumeration probe"
        );
    }
    let probe_pid = match &probe {
        Some((_, _, pid)) => *pid,
        None => None,
    };
    let identity = juggler_core::model::WindowIdentity {
        // The REAL TextEdit pid when the probe is up (title disambiguates
        // its window from any user document); without the probe a pid of 0
        // can never match — the empty/denied enumeration signature.
        pid: probe_pid.unwrap_or(0),
        bundle_id: Some("com.apple.TextEdit".into()),
        title: Some("aij-e415-probe".into()),
        cgwindow_id: None,
    };
    let started = Instant::now();
    let attempt = source.capture(&identity);
    let elapsed = started.elapsed();
    match attempt {
        Err(CaptureError::PermissionDenied) => {
            println!(
                "[record] DENIED in {elapsed:?}: Screen Recording grant missing — \
                 the panel renders guidance + status data only (Req.3 path)"
            );
            // The driver over the real source: denied is terminal and
            // the loop ends without a frame (teardown by construction).
            let events: Arc<Mutex<Vec<PreviewEvent>>> = Arc::new(Mutex::new(Vec::new()));
            let sink = events.clone();
            let emit = move |event: PreviewEvent| sink.lock().unwrap().push(event);
            let stopped = AtomicBool::new(false);
            let driver = PreviewDriver {
                session_id: "sess-real",
                identity,
                source: &source,
                is_live: &|| true,
                emit: &emit,
                stopped: &stopped,
            };
            let loop_started = Instant::now();
            driver.run();
            assert!(
                loop_started.elapsed() < Duration::from_secs(1),
                "denied loop must end immediately"
            );
            let events = events.lock().unwrap().clone();
            assert!(matches!(
                events.as_slice(),
                [PreviewEvent::Phase { phase, .. }] if *phase == juggler_app::preview::PreviewPhase::Denied
            ));
            println!("[record] driver emitted exactly one Denied phase, no frames");
            if let Some((doc, was_running, _)) = &probe {
                close_probe_window(doc, *was_running);
            }
        }
        Err(other) => {
            if let Some((doc, was_running, _)) = &probe {
                close_probe_window(doc, *was_running);
            }
            panic!("unexpected capture error: {other:?}");
        }
        Ok(png) => {
            // The granted-machine live record: one real frame.
            assert!(
                png.len() > 8 && png.starts_with(&[0x89, b'P', b'N', b'G']),
                "not a PNG: {} bytes",
                png.len()
            );
            println!(
                "[record] LIVE frame: {} bytes PNG in {elapsed:?}; probing 2 s of polling…",
                png.len()
            );
            let before = cpu_percent();
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut frames = 0usize;
            while Instant::now() < deadline {
                if source.capture(&identity).is_ok() {
                    frames += 1;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            let during = cpu_percent();
            let after = cpu_percent();
            println!(
                "[record] 2 s polling: {frames} frames; cpu before={before:?}% during={during:?}% after={after:?}%"
            );
            if let Some((doc, was_running, _)) = &probe {
                close_probe_window(doc, *was_running);
            }
        }
    }
}
