//! Embeds the helper's Info.plist in its `__TEXT,__info_plist` section on
//! macOS: its bundle identifier is what it is signed as (and what the app
//! requires, `juggler_local_ai_protocol::IDENTIFIER`), and Metal names the
//! helper's shader cache after it. (Its sandbox is the profile the app
//! applies at start, not the App Sandbox: it has no entitlements.)

#![allow(
    clippy::print_stdout,
    reason = "cargo reads a build script's directives from its stdout"
)]

fn main() {
    println!("cargo:rerun-if-changed=Info.plist");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        println!("cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{dir}/Info.plist");
    }
}
