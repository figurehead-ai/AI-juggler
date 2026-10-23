//! AJ-E1509 Req.2–3 on this Mac: keeping it awake is a real power
//! assertion, named, taken, renamed and released as the plan says — read
//! back from `pmset -g assertions` (macOS only). Holding off App Nap
//! allows idle sleep, so it is no power assertion: it must never show as
//! one (the keeper's unit tests follow its state).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(target_os = "macos")]

use juggler_supervisor::awake::{AwakeKeeper, AwakePlan, awake_reason};

/// This process's assertions, as `pmset` lists them.
fn ours() -> Vec<String> {
    let out = std::process::Command::new("/usr/bin/pmset")
        .args(["-g", "assertions"])
        .output()
        .unwrap();
    let me = format!("pid {}(", std::process::id());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| line.contains(&me))
        .map(str::to_owned)
        .collect()
}

fn has(lines: &[String], kind: &str, name: &str) -> bool {
    lines
        .iter()
        .any(|line| line.contains(kind) && line.contains(&format!("named: \"{name}\"")))
}

#[test]
fn the_assertions_are_named_taken_renamed_and_released() {
    let mut keeper = AwakeKeeper::default();
    keeper.apply(AwakePlan {
        working: 2,
        supervised: 2,
        keep_awake: true,
    });
    let held = ours();
    assert!(
        has(&held, "PreventUserIdleSystemSleep", &awake_reason(2)),
        "{held:#?}"
    );

    // A third session starts working: the name follows.
    keeper.apply(AwakePlan {
        working: 3,
        supervised: 3,
        keep_awake: true,
    });
    let held = ours();
    assert!(
        has(&held, "PreventUserIdleSystemSleep", &awake_reason(3)),
        "{held:#?}"
    );
    assert!(!has(&held, "", &awake_reason(2)), "the old name is gone");

    // Nothing works now (all waiting for the user): the Mac may sleep.
    keeper.apply(AwakePlan {
        working: 0,
        supervised: 3,
        keep_awake: true,
    });
    let held = ours();
    assert!(
        !held
            .iter()
            .any(|line| line.contains("PreventUserIdleSystemSleep")),
        "{held:#?}"
    );
    assert!(keeper.holding_off_app_nap(), "App Nap stays held off");

    // Nothing supervised: nothing held at all.
    keeper.apply(AwakePlan::default());
    let held = ours();
    assert!(
        !held.iter().any(|line| line.contains("AI Juggler")),
        "{held:#?}"
    );
}
