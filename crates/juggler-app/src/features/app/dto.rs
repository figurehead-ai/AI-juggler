//! The app's info on the wire.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Static application information served to the UI at startup.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AppInfo {
    /// Application version from the crate manifest.
    pub version: String,
    /// The app's name: the edition's (AI Juggler, AI Juggler Nightly,
    /// AI Juggler Dev — D-23).
    pub name: String,
    /// The edition: `main` | `nightly` | `dev`.
    pub edition: String,
}

impl AppInfo {
    /// Builds the info from compile-time crate metadata.
    #[must_use]
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            name: crate::edition::current().app_name().to_string(),
            edition: crate::edition::current().as_str().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppInfo;

    #[test]
    fn app_info_is_populated() {
        let info = AppInfo::current();
        assert_eq!(info.name, "AI Juggler");
        assert_eq!(info.edition, "main");
        assert!(!info.version.is_empty());
    }
}
