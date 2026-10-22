//! The read-only test bridge (`docs/qa/DEV-BRIDGE.md`) never ships: the
//! `dev-bridge` feature is no default feature, so the builds every
//! release starts from leave it out. (`scripts/dev_bridge_check.sh` checks
//! the feature graph and the release workflows, and that a release build
//! with the feature doesn't compile.)
#![allow(clippy::expect_used, reason = "test code")]

#[test]
fn the_dev_bridge_is_not_a_default_feature() {
    let manifest = include_str!("../Cargo.toml");
    let default = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("default ="))
        .expect("juggler-app declares its default features");
    assert!(!default.contains("dev-bridge"), "{default}");
    assert!(
        manifest.contains("\ndev-bridge = ["),
        "the feature this test guards was renamed"
    );
}
