//! The docs fact-check (fix batch 4): what macOS needs in the app bundle
//! for the access AI Juggler asks for. Without these, macOS 15+ fails
//! Fleet's local network silently, and the hardened runtime denies Apple
//! Events (jump to a session's window) with no prompt.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;

fn read(relative: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

/// The value after `<key>name</key>`, up to its closing tag.
fn value_after<'a>(plist: &'a str, key: &str) -> &'a str {
    let at = plist
        .find(&format!("<key>{key}</key>"))
        .unwrap_or_else(|| panic!("{key} is missing"));
    let rest = &plist[at..];
    let end = rest.find("</string>").map_or(rest.len(), |end| end);
    &rest[..end]
}

#[test]
fn fleet_can_reach_the_local_network() {
    let plist = read("Info.plist");
    let why = value_after(&plist, "NSLocalNetworkUsageDescription");
    assert!(why.contains("Fleet"), "says what it's for: {why}");
    // The Bonjour type is the one Fleet advertises and browses.
    let service = juggler_link::discovery::SERVICE_TYPE
        .trim_end_matches('.')
        .trim_end_matches(".local");
    let services = &plist[plist
        .find("<key>NSBonjourServices</key>")
        .expect("NSBonjourServices")..];
    let array = &services[..services.find("</array>").unwrap()];
    assert!(
        array.contains(&format!("<string>{service}</string>")),
        "{service} in {array}"
    );
}

#[test]
fn jumping_to_a_window_may_send_apple_events() {
    let plist = read("Info.plist");
    let why = value_after(&plist, "NSAppleEventsUsageDescription");
    assert!(why.contains("System Events"), "{why}");
    let entitlements = read("entitlements.plist");
    let key = "<key>com.apple.security.automation.apple-events</key>";
    let at = entitlements
        .find(key)
        .expect("the Apple Events entitlement");
    assert!(
        entitlements[at + key.len()..]
            .trim_start()
            .starts_with("<true/>")
    );
    // Every build signs with it: the app's config and the release workflow.
    let config: serde_json::Value = serde_json::from_str(&read("tauri.conf.json")).unwrap();
    assert_eq!(
        config["bundle"]["macOS"]["entitlements"],
        "entitlements.plist"
    );
    let workflow = read("../../.github/workflows/build-macos.yml");
    assert!(workflow.contains(r#""entitlements": "entitlements.plist""#));
}
