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
    // Each architecture's half of a universal build wants its own copy (the
    // 0.2.0 rehearsal: "resource path `binaries/juggler-agent-aarch64-apple-
    // darwin` doesn't exist"), the bundler the universal one.
    assert!(workflow.contains(&format!("binaries/{program}-$arch")));
    assert!(workflow.contains(&format!("binaries/{program}-universal-apple-darwin")));
    // One list of helpers (8b3f587fa): `tauri.bundle.json`, which `just app`
    // and the workflow both read.
    let bundle: serde_json::Value = serde_json::from_str(&read("tauri.bundle.json")).unwrap();
    assert!(
        bundle["bundle"]["externalBin"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(format!("binaries/{program}")))
    );
    assert!(workflow.contains(
        r#"json.load(open("crates/juggler-app/tauri.bundle.json"))["bundle"]["externalBin"]"#
    ));
    // Every listed helper checked inside the built .app, beside the app's
    // executable (8c7d2bc72: one script, the same one `just app` runs).
    assert!(workflow.contains("python3 scripts/bundle_check.py \"$bundle\"/*.app"));
    let check = read("../../scripts/bundle_check.py");
    assert!(
        check.contains("Contents")
            && check.contains("MacOS")
            && check.contains("tauri.bundle.json")
    );
    let sign = read("../../scripts/sign_macos.sh");
    assert!(sign.contains(&format!("helpers=({program} ")));
    let helpers_signed = sign
        .find(r#"sign ${with[@]+"${with[@]}"} "$app/Contents/MacOS/$helper""#)
        .unwrap();
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

/// AJ-E3802 (ADR-0033 §4.1): the Apple model's helper ships inside the
/// signed, notarized bundle beside the agent bridge. Every build makes it
/// with the one script (arm64, its identifier from its embedded
/// Info.plist, hardened runtime, the App Sandbox and nothing else), the
/// bundle list names it, the sign script signs it with exactly its
/// entitlements before the app and checks it against the app's team, and
/// both the release candidate and the release check the bundled copy.
#[test]
fn the_apple_helper_ships_signed_and_sandboxed_inside_the_bundle() {
    use juggler_app::features::ai::engines::apple::{IDENTIFIER, PROGRAM};
    let plist = read("helpers/juggler-apple-ai.Info.plist");
    assert!(
        value_after(&plist, "CFBundleIdentifier").ends_with(&format!("<string>{IDENTIFIER}")),
        "signed as the identifier the app requires"
    );
    let entitlements = read("helpers/juggler-apple-ai.entitlements");
    let keys: Vec<&str> = entitlements
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<key>"))
        .collect();
    assert_eq!(
        keys,
        ["com.apple.security.app-sandbox</key>"],
        "the sandbox, no network, nothing else"
    );

    let build = read("../../scripts/build_apple_ai_helper.sh");
    assert!(build.contains("-target arm64-apple-macos14.0"));
    assert!(build.contains("__info_plist"));
    assert!(build.contains(&format!("identifier={IDENTIFIER}")));
    assert!(build.contains("--options runtime --entitlements \"$entitlements\""));
    assert!(build.contains("PrivateCloudCompute"));

    let bundle: serde_json::Value = serde_json::from_str(&read("tauri.bundle.json")).unwrap();
    assert!(
        bundle["bundle"]["externalBin"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(format!("binaries/{PROGRAM}")))
    );

    let sign = read("../../scripts/sign_macos.sh");
    assert!(sign.contains(&format!("helpers=(juggler-agent {PROGRAM} ")));
    assert!(sign.contains("crates/juggler-app/helpers/juggler-apple-ai.entitlements"));
    assert!(
        sign.contains("build_apple_ai_helper.sh\" check \"$app/Contents/MacOS/$helper\" \"$app\"")
    );

    let workflow = read("../../.github/workflows/build-macos.yml");
    assert!(workflow.contains("scripts/build_apple_ai_helper.sh release \"target/$ARCH/release\""));
    assert!(workflow.contains(&format!("binaries/{PROGRAM}-$ARCH")));
    assert!(workflow.contains(&format!("binaries/{PROGRAM}-universal-apple-darwin")));
    assert!(workflow.contains("juggler-app juggler-agent juggler-apple-ai"));
    assert!(workflow.contains("scripts/build_apple_ai_helper.sh check"));
    // The sign job's sparse checkout has what the check reads.
    assert_eq!(
        workflow
            .matches("            crates/juggler-app/helpers/juggler-apple-ai.entitlements")
            .count(),
        2
    );

    let dev = read("../../scripts/build_dev_app.sh");
    assert!(dev.contains("scripts/build_apple_ai_helper.sh \"$profile\""));
    assert!(dev.contains(&format!("binaries/{PROGRAM}-$triple")));
    assert!(dev.contains(
        "scripts/build_apple_ai_helper.sh check \"$app/Contents/MacOS/juggler-apple-ai\" \"$app\""
    ));
}

/// D-131 (ADR-0033 §4.2): the local model's helper ships the same way:
/// one script builds it offline and locked from its own workspace (arm64,
/// its identifier from its embedded Info.plist, hardened runtime, no
/// entitlements: its sandbox is the profile the app applies at start), the
/// bundle list names it, the sign script signs it and runs its check (no
/// network call imported), and the release candidate and the release
/// check the bundled copy.
#[test]
fn the_local_model_helper_ships_signed_and_sandboxed_inside_the_bundle() {
    use juggler_local_ai_protocol::{IDENTIFIER, PROGRAM};
    let plist = read("../juggler-local-ai/Info.plist");
    assert!(
        value_after(&plist, "CFBundleIdentifier").ends_with(&format!("<string>{IDENTIFIER}")),
        "signed as the identifier the app requires"
    );
    // Review M1: no entitlements file. Its sandbox is its own
    // deny-by-default profile, applied by the app at every start.
    assert!(
        !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../juggler-local-ai/juggler-local-ai.entitlements")
            .exists()
    );
    assert!(juggler_local_ai_protocol::SANDBOX_PROFILE.contains("(deny default)"));

    let manifest = read("../juggler-local-ai/Cargo.toml");
    assert!(manifest.contains("llama-cpp-2 = { version = \"=0.1.146\""));
    assert!(manifest.contains("unsafe_code = \"forbid\""));
    let build = read("../../scripts/build_local_ai_helper.sh");
    assert!(build.contains("--offline --locked --release --target aarch64-apple-darwin"));
    assert!(build.contains(&format!("identifier={IDENTIFIER}")));
    assert!(build.contains("codesign --force --options runtime --sign -"));
    assert!(build.contains("carries entitlements"));
    assert!(build.contains("imports a network call"));

    let bundle: serde_json::Value = serde_json::from_str(&read("tauri.bundle.json")).unwrap();
    assert!(
        bundle["bundle"]["externalBin"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(format!("binaries/{PROGRAM}")))
    );

    let sign = read("../../scripts/sign_macos.sh");
    assert!(sign.contains("helpers=(juggler-agent juggler-apple-ai juggler-local-ai)"));
    assert!(sign.contains(
        "juggler-local-ai) \"$here/build_local_ai_helper.sh\" check \"$app/Contents/MacOS/$helper\" \"$app\""
    ));

    let workflow = read("../../.github/workflows/build-macos.yml");
    assert!(workflow.contains("scripts/build_local_ai_helper.sh release \"target/$ARCH/release\""));
    assert!(workflow.contains("--manifest-path crates/juggler-local-ai/Cargo.toml"));
    assert!(workflow.contains(&format!("binaries/{PROGRAM}-$ARCH")));
    assert!(workflow.contains(&format!("binaries/{PROGRAM}-universal-apple-darwin")));
    assert!(workflow.contains("juggler-app juggler-agent juggler-apple-ai juggler-local-ai"));
    assert!(workflow.contains("scripts/build_local_ai_helper.sh check"));
    assert_eq!(
        workflow
            .matches("            scripts/build_local_ai_helper.sh\n")
            .count(),
        2,
        "the sign jobs' sparse checkout has the check"
    );

    let dev = read("../../scripts/build_dev_app.sh");
    assert!(dev.contains("scripts/build_local_ai_helper.sh \"$profile\""));
    assert!(dev.contains(&format!("binaries/{PROGRAM}-$triple")));
    assert!(dev.contains(
        "scripts/build_local_ai_helper.sh check \"$app/Contents/MacOS/juggler-local-ai\" \"$app\""
    ));
    let rc = read("../../scripts/qa/fetch_rc.sh");
    assert!(rc.contains("scripts/build_local_ai_helper.sh check"));
}
