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
    let end = rest.find("</string>").unwrap_or(rest.len());
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

/// AJ-E1704: the agent bridge ships inside the signed, notarized bundle.
/// The release workflow builds it for both chips, makes it Tauri's
/// `externalBin` (so it lands in `Contents/MacOS` beside the app), and
/// checks it is there; the sign script names it as the bundle's one
/// helper, signs it on its own before the app (hardened runtime, no
/// entitlements), and refuses a bundle without it.
#[test]
fn the_agent_bridge_ships_signed_inside_the_bundle() {
    let program = juggler_agentbridge::wire::PROGRAM;
    let workflow = read("../../.github/workflows/build-macos.yml");
    assert!(workflow.contains(&format!("--bin {program} --target \"$arch\"")));
    assert!(workflow.contains(&format!("binaries/{program}-universal-apple-darwin")));
    assert!(workflow.contains(&format!(r#""externalBin": ["binaries/{program}"]"#)));
    assert!(workflow.contains(&format!("Contents/MacOS/{program}")));
    let sign = read("../../scripts/sign_macos.sh");
    assert!(sign.contains(&format!("helpers=({program})")));
    let helpers_signed = sign.find(r#"sign "$app/Contents/MacOS/$helper""#).unwrap();
    let app_signed = sign.find("\nsign \"$app\"\n").unwrap();
    assert!(
        helpers_signed < app_signed,
        "inside out: the helper before the app"
    );
    assert!(sign.contains("the bundle lacks its helper"));
    assert!(!sign.contains("codesign --force --deep"));
    let ignored = read("../../.gitignore");
    assert!(
        ignored.contains("/crates/juggler-app/binaries/"),
        "never committed"
    );
}
