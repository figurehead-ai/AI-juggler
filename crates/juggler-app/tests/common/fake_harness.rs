//! Where the `fake-harness` test binary is (built with the workspace;
//! `CARGO_BIN_EXE_fake-harness` only exists inside its own crate).

use std::path::PathBuf;

/// The built `fake-harness`, debug before release.
#[must_use]
pub fn bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_fake-harness") {
        return PathBuf::from(path);
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join("fake-harness");
        if candidate.exists() {
            return candidate;
        }
    }
    unreachable!("fake-harness binary not found under {}", target.display());
}
