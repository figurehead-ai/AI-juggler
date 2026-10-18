//! The fake desktop adapter: drives the real `fake-desktop` binary
//! through the AJ-E413 desktop launch contract. It is the reference
//! desktop implementation adapters mirror, and the lifecycle fixture
//! the supervisor/reconciliation suites run against.
//!
//! The fake desktop app is SINGLE-ACCOUNT by design — it carries the
//! honest isolation label (`Single-account app — Juggler tracks,
//! cannot isolate`) so the UI state is exercised end to end.

use std::path::{Path, PathBuf};

use juggler_adapter::{
    AdapterError, Capabilities, DesktopLaunch, DiscoveryReport, HarnessAdapter, InstallStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

/// Harness id of the fake desktop adapter.
pub const FAKE_DESKTOP_ID: &str = "fake-desktop";

/// The honest label for single-account desktop apps (Req.1 copy exact).
pub const SINGLE_ACCOUNT_LABEL: &str = "Single-account app — Juggler tracks, cannot isolate";

/// Adapter over the `fake-desktop` binary (see `src/bin/fake_desktop.rs`).
#[derive(Debug, Clone)]
pub struct FakeDesktopAdapter {
    bin: PathBuf,
}

impl FakeDesktopAdapter {
    /// Creates an adapter pointing at the fake desktop binary.
    #[must_use]
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self { bin: bin.into() }
    }

    /// The binary this adapter launches.
    #[must_use]
    pub fn bin(&self) -> &Path {
        &self.bin
    }
}

impl HarnessAdapter for FakeDesktopAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(FAKE_DESKTOP_ID)
    }

    fn capabilities(&self) -> Capabilities {
        // Desktop app + single-account: the honest limits pair (Req.1).
        Capabilities::DESKTOP_APP | Capabilities::DESKTOP_SINGLE_ACCOUNT
    }

    fn verified_against(&self) -> &'static [&'static str] {
        &["0.1.0"]
    }

    fn discover(&self) -> DiscoveryReport {
        let installed = self.bin.exists();
        DiscoveryReport {
            adapter: self.id(),
            install: if installed {
                InstallStatus::Installed {
                    version: "0.1.0".to_string(),
                }
            } else {
                InstallStatus::NotFound
            },
            verification: juggler_adapter::VerificationStatus::Verified,
            binary: installed.then(|| self.bin.clone()),
            notes: if installed {
                Vec::new()
            } else {
                vec!["fake-desktop binary not built".to_string()]
            },
        }
    }

    fn prepare_account(&self, state_dir: &Path) -> Result<juggler_adapter::EnvMap, AdapterError> {
        std::fs::create_dir_all(state_dir).map_err(|error| AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: error.to_string(),
        })?;
        // Single-account: there is nothing to isolate — the env overlay
        // is empty and the label says so.
        Ok(juggler_adapter::EnvMap::new())
    }

    fn launch_spec(
        &self,
        _account: &Account,
        _profile: &Profile,
        cwd: &Path,
        _mode: &juggler_adapter::LaunchMode,
    ) -> Result<juggler_adapter::LaunchSpec, AdapterError> {
        // The fake desktop ships no CLI engine: terminal sessions are
        // not offered (Req.4's either/or resolved to desktop-only).
        let _ = cwd;
        Err(AdapterError::InvalidLaunch(
            "the fake desktop app has no CLI engine".into(),
        ))
    }

    fn desktop_launch(&self, account: &Account) -> Result<DesktopLaunch, AdapterError> {
        if !self.bin.exists() {
            return Err(AdapterError::InvalidLaunch(format!(
                "desktop binary missing: {}",
                self.bin.display()
            )));
        }
        Ok(DesktopLaunch {
            program: self.bin.clone(),
            bundle_id: Some("dev.aijuggler.fake-desktop".to_string()),
            args: Vec::new(),
            env: juggler_adapter::EnvMap::new(),
            cwd: account.state_dir.clone(),
            isolation_label: SINGLE_ACCOUNT_LABEL,
        })
    }

    fn desktop_isolation_label(&self) -> Option<&'static str> {
        Some(SINGLE_ACCOUNT_LABEL)
    }

    fn watcher(
        &self,
        _transcript_root: &Path,
    ) -> Result<Box<dyn juggler_adapter::SessionWatcher>, AdapterError> {
        // Desktop apps ship no transcripts in this fixture.
        Ok(Box::new(NoDesktopWatcher))
    }
}

/// The no-op watcher: a desktop app without transcripts never yields
/// events (read-only, never fails the system).
struct NoDesktopWatcher;

impl juggler_adapter::SessionWatcher for NoDesktopWatcher {
    fn poll(
        &mut self,
        _timeout: std::time::Duration,
    ) -> Result<Option<juggler_adapter::SessionEvent>, AdapterError> {
        Ok(None)
    }
}
