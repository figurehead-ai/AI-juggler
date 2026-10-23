//! Build script: generates Tauri context artifacts (window/capability
//! schemas) for the application crate.

fn main() {
    tauri_build::build();
    crash_sender();
}

/// AJ-E1516: the crash report sender (`src/crash_send.rs`) is built only
/// into a release build given the reporting service's address. No
/// workflow sets it, so no build has a sender until the service exists.
fn crash_sender() {
    println!("cargo::rustc-check-cfg=cfg(crash_sender)");
    println!("cargo::rerun-if-env-changed=AI_JUGGLER_CRASH_ENDPOINT");
    let address = std::env::var("AI_JUGGLER_CRASH_ENDPOINT").unwrap_or_default();
    let release = std::env::var("PROFILE").is_ok_and(|profile| profile == "release");
    if release && address.starts_with("https://") {
        println!("cargo::rustc-cfg=crash_sender");
    }
}
