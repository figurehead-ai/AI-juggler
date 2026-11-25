//! Embeds the helper's Info.plist in its `__TEXT,__info_plist` section on
//! macOS: its bundle identifier is what it is signed as (and what the app
//! requires, the app's `engines::local::identifier()`), and Metal names the
//! helper's shader cache after it. (Its sandbox is the profile the app
//! applies at start, not the App Sandbox: it has no entitlements.)
//!
//! AJ-E3822: the identifier is the build's identity's, never one fixed
//! name. `scripts/build_local_ai_helper.sh` passes it in
//! `AIJ_HELPER_IDENTIFIER` (from `scripts/helper_identity.sh`: the release
//! workflow's AI Juggler and Nightly share `ai.figurehead.juggler`'s, Dev
//! and lab builds carry the Dev prefix). A plain `cargo build` or `cargo
//! test` here names no identity: it is a development build, so the Dev
//! identity's, as an unnamed build is everywhere else
//! (`Edition::from_identifier`). The plist's names (`CFBundleName`,
//! `CFBundleDisplayName`) are what a system prompt shows for it.

#![allow(
    clippy::print_stdout,
    clippy::panic,
    reason = "cargo reads a build script's directives from its stdout, and a bad identifier stops the build"
)]

/// The Dev identity's prefix (the app's `Edition::Dev.identifier()`;
/// `scripts/helper_identity_lint.py` keeps the two equal).
const DEV_PREFIX: &str = "ai.figurehead.juggler.dev";

/// The last part of the helper's identifier.
const SUFFIX: &str = "local-ai";

/// Where the template has the identifier written in.
const PLACEHOLDER: &str = "<string>@HELPER_IDENTIFIER@</string>";

fn main() {
    println!("cargo:rerun-if-changed=Info.plist");
    println!("cargo:rerun-if-env-changed=AIJ_HELPER_IDENTIFIER");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let identifier = std::env::var("AIJ_HELPER_IDENTIFIER")
        .ok()
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| format!("{DEV_PREFIX}.{SUFFIX}"));
    // Quoted into a plist and, by the app, a code requirement.
    assert!(
        identifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            && identifier.ends_with(&format!(".{SUFFIX}")),
        "AIJ_HELPER_IDENTIFIER {identifier:?} isn't a local-ai helper's identifier"
    );
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let template = std::fs::read_to_string(format!("{dir}/Info.plist"))
        .unwrap_or_else(|e| panic!("Info.plist: {e}"));
    assert!(
        template.contains(PLACEHOLDER),
        "Info.plist has no {PLACEHOLDER}"
    );
    let plist = template.replace(PLACEHOLDER, &format!("<string>{identifier}</string>"));
    let out = std::env::var("OUT_DIR").unwrap_or_default();
    let path = format!("{out}/Info.plist");
    std::fs::write(&path, plist).unwrap_or_else(|e| panic!("{path}: {e}"));
    println!("cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{path}");
}
