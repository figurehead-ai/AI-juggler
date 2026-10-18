//! AJ-E414 real-window evidence: two REAL TextEdit windows (distinct
//! sacrificial temp-dir documents), focused through the production
//! tier service and the REAL `OsascriptRunner`. With the Accessibility
//! grant present this records 10/10 correct-window focus (alternating
//! targets, minimised included, AX-verified frontmost). Without the
//! grant it records the DEGRADED tier exactly as shipped (activation +
//! honest outcome + permission flag — Req.3's path). Ignored in CI (no
//! GUI): run locally with
//! `cargo test -p juggler-app --test focus_real -- --ignored
//! --nocapture` and store the output as the evidence record.
//!
//! Safety: only temp-dir files this test creates and TextEdit windows
//! it opens are touched; if TextEdit was already running for the user,
//! only our two windows are closed and the app is NOT quit (Apple
//! Events `close window`, a separate grant from Accessibility).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::sync::Arc;

use juggler_app::focus::{self, FocusAttempt, OsascriptRunner, ScriptRunner};
use juggler_core::model::WindowIdentity;

const BUNDLE: &str = "com.apple.TextEdit";

fn osa(source: &str) -> Result<String, String> {
    OsascriptRunner.osascript(source)
}

/// Apple Events (a separate grant from AX): window counts + cleanup.
fn textedit_window_count() -> usize {
    osa(&format!(
        "tell application id \"{BUNDLE}\" to count windows"
    ))
    .unwrap_or_default()
    .trim()
    .parse()
    .unwrap_or(0)
}

fn frontmost_title() -> Option<String> {
    osa("tell application \"System Events\" to get name of frontmost window of (first application process whose bundle identifier is \"com.apple.TextEdit\")")
        .ok()
}

fn set_minimised(title: &str, minimised: bool) {
    let _ = osa(&format!(
        "tell application \"System Events\" to tell (first application process whose bundle identifier is \"{BUNDLE}\") to set value of attribute \"AXMinimized\" of window \"{title}\" to {minimised}"
    ));
}

#[test]
#[ignore = "drives real TextEdit windows; local evidence run only"]
fn real_two_window_focus_record() {
    let dir = std::env::temp_dir();
    let doc_a = dir.join("aij-e414-a.txt");
    let doc_b = dir.join("aij-e414-b.txt");
    std::fs::write(&doc_a, "AJ-E414 sacrificial window A\n").unwrap();
    std::fs::write(&doc_b, "AJ-E414 sacrificial window B\n").unwrap();
    // Pre-clean: a previously interrupted run may have left our titled
    // windows behind (only windows with OUR titles are ever touched).
    for title in ["aij-e414-a.txt", "aij-e414-b.txt"] {
        let _ = osa(&format!(
            "tell application id \"{BUNDLE}\" to close window \"{title}\""
        ));
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    let windows_before = textedit_window_count();
    println!("[record] TextEdit windows before: {windows_before}");

    // Open both documents (LaunchServices; distinct window titles).
    for doc in [&doc_a, &doc_b] {
        let status = std::process::Command::new("/usr/bin/open")
            .arg("-a")
            .arg("TextEdit")
            .arg(doc)
            .status()
            .expect("open TextEdit");
        assert!(status.success(), "open {doc:?} failed");
    }
    // Wait until TextEdit reports BOTH new windows (Apple Events).
    for _ in 0..100 {
        if textedit_window_count() >= windows_before + 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let windows_after = textedit_window_count();
    println!("[record] TextEdit windows after open: {windows_after}");
    assert!(
        windows_after >= windows_before + 2,
        "TextEdit did not open both sacrificial windows"
    );

    let title_a = "aij-e414-a.txt";
    let title_b = "aij-e414-b.txt";
    let identity_for = |title: &str| WindowIdentity {
        pid: 0, // pre-v10-style identity: forces bundle re-discovery
        bundle_id: Some(BUNDLE.to_string()),
        title: Some(title.to_string()),
        cgwindow_id: None,
    };

    let runner: Arc<dyn ScriptRunner> = Arc::new(OsascriptRunner);
    let mut focused = 0usize;
    let mut degraded = 0usize;
    let mut first: Option<FocusAttempt> = None;
    for attempt in 1..=10 {
        let target = if attempt % 2 == 1 { title_a } else { title_b };
        let started = std::time::Instant::now();
        let result = focus::focus(runner.clone(), &identity_for(target));
        let elapsed = started.elapsed();
        println!(
            "[record] attempt {attempt:2} target={target:18} outcome={} perm_missing={} in {elapsed:?}",
            result.outcome, result.permission_missing
        );
        if first.is_none() {
            first = Some(result.clone());
        }
        if result.permission_missing {
            assert_eq!(
                result.outcome,
                focus::OUTCOME_WINDOW_NOT_FOUND,
                "degraded tier must report the honest activation-only outcome"
            );
            degraded += 1;
        } else {
            assert_eq!(
                result.outcome,
                focus::OUTCOME_FOCUSED_WINDOW,
                "attempt {attempt} did not focus {target}"
            );
            if let Some(frontmost) = frontmost_title() {
                println!("[record]          frontmost=\"{frontmost}\"");
                assert_eq!(frontmost, target, "wrong window frontmost after focus");
            }
            // Re-minimise A between odd attempts so the minimised case
            // is exercised every time (AXRaise on a minimised window
            // is a silent no-op — the service must clear it first).
            if attempt % 2 == 1 {
                set_minimised(title_a, true);
            }
            focused += 1;
        }
    }

    // Cleanup: only OUR windows (Apple Events); the app is left
    // running whatever it was doing before.
    for title in [title_a, title_b] {
        let _ = osa(&format!(
            "tell application id \"{BUNDLE}\" to close window \"{title}\""
        ));
    }
    let _ = std::fs::remove_file(&doc_a);
    let _ = std::fs::remove_file(&doc_b);
    println!(
        "[record] windows after cleanup: {}",
        textedit_window_count()
    );

    if degraded > 0 {
        println!(
            "[record] DEGRADED {degraded}/10 (permission_missing): activation-only tier, \
             outcome \"{}\" — grant Accessibility (System Settings → Privacy & Security → \
             Accessibility) and re-run for the focused-window record; the 10/10 \
             correct-window gate is covered by the scripted fixture suite.",
            first.unwrap().outcome
        );
    } else {
        println!(
            "[record] focused correctly {focused}/10 (alternating targets, minimised included)"
        );
    }
}
