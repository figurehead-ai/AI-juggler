//! Discovery reports (spec §4): what is installed, and is it a version the
//! adapter was verified against?

use std::path::PathBuf;

use crate::model::HarnessId;

/// Outcome of probing the machine for a harness installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallStatus {
    /// The harness is installed; carries its reported version string.
    Installed {
        /// Version exactly as reported by the harness.
        version: String,
    },
    /// No installation found.
    NotFound,
    /// Probing failed (e.g. the binary exists but `--version` errored).
    Failed {
        /// What went wrong.
        message: String,
    },
}

/// Whether the discovered version is covered by this adapter's verified set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationStatus {
    /// The installed version is in `verified_against`.
    Verified,
    /// Installed but not listed as verified — features may still work;
    /// surfaces as a UI warning, never a hard failure (spec §4 rules).
    Unverified,
}

/// Everything [`crate::HarnessAdapter::discover`] reports about one harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryReport {
    /// The adapter's harness id.
    pub adapter: HarnessId,
    /// Install probe result.
    pub install: InstallStatus,
    /// Version verification result.
    pub verification: VerificationStatus,
    /// Resolved binary path, when found.
    pub binary: Option<PathBuf>,
    /// Human-facing notes (warnings, drift details).
    pub notes: Vec<String>,
}

impl DiscoveryReport {
    /// True when the harness is installed (regardless of verification).
    #[must_use]
    pub fn is_installed(&self) -> bool {
        matches!(self.install, InstallStatus::Installed { .. })
    }
}
