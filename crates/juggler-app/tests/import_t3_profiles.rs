//! Operational runner (not a CI test): imports the user's real t3.codes
//! Claude profiles into AI Juggler's app data dir as read-only copies
//! (ADR-0006). Run explicitly:
//!
//! ```text
//! cargo test -p juggler-app --test import_t3_profiles -- --ignored --nocapture
//! ```

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(target_os = "macos")]

use std::sync::Arc;

use juggler_app::commands;
use juggler_app::state::AppState;

/// The t3.codes profile root (the manager tool's default location).
const T3_ROOT: &str = "~/Library/Application Support/ClaudeCodeProfiles";

fn expand(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").expect("HOME");
        std::path::Path::new(&home).join(rest)
    } else {
        std::path::PathBuf::from(path)
    }
}

#[test]
#[ignore = "operational: imports the user's real profiles on demand"]
fn import_real_t3_profiles() {
    let root = expand(T3_ROOT);
    // Every profile folder under the manager root (names are the
    // user's own — never hard-coded here). Plans stay unset: the
    // user picks them in the app (plan tier is never inferred).
    let mut profiles: Vec<(String, Option<&str>)> = std::fs::read_dir(&root)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .map(|entry| (entry.file_name().to_string_lossy().into_owned(), None))
                .collect()
        })
        .unwrap_or_default();
    profiles.sort();

    let data_dir = expand("~/Library/Application Support/AI Juggler");
    let state = Arc::new(AppState::init(data_dir).expect("app state"));

    for (name, plan) in profiles {
        let source = root.join(&name);
        if !source.is_dir() {
            eprintln!("SKIP {name}: not found at {}", source.display());
            continue;
        }
        match commands::account_import_impl(
            &state,
            "claude-code",
            &source.to_string_lossy(),
            &name,
            plan.map(std::string::ToString::to_string),
            Some("home-pattern".to_string()),
            false,
        ) {
            Ok(result) => {
                println!(
                    "IMPORTED {} → account {} | {} files, {} bytes | login: {} | skipped: {} | note: {}",
                    name,
                    result.account.id,
                    result.files_copied,
                    result.bytes_copied,
                    result.login,
                    result.skipped.len(),
                    result.login_note.as_deref().unwrap_or("-"),
                );
            }
            Err(error) => {
                eprintln!("FAILED {name}: {error}");
            }
        }
    }

    let accounts = {
        let storage = state.storage.lock().unwrap();
        storage.accounts().list().unwrap()
    };
    println!("TOTAL ACCOUNTS NOW: {}", accounts.len());
    for account in accounts {
        println!("  {} | {}", account.id, account.label);
    }
}

#[test]
#[ignore = "operational: scans the real machine, prints findings"]
fn scan_real_machine() {
    let data_dir = expand("~/Library/Application Support/AI Juggler");
    let state = Arc::new(AppState::init(data_dir).expect("app state"));
    for found in commands::environment_scan_impl(&state) {
        println!(
            "{} | {} | login: {} | {} bytes | managed: {}",
            found.origin, found.path, found.login, found.bytes, found.managed
        );
        for note in &found.notes {
            println!("    note: {note}");
        }
    }
}
