//! AJ-E1525 (ADR-0019 §1): the tools catalog. Every registered tool is
//! served as data, the primary tool first; a tool no adapter describes
//! beyond its id gets the neutral spec (its id as its name, the letter
//! mark, one group), so a new tool needs no change outside its adapter.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::discovery::{DiscoveryReport, InstallStatus, VerificationStatus};
use juggler_adapter::launch::{EnvMap, LaunchMode, LaunchSpec};
use juggler_adapter::model::{Account, HarnessId, Profile};
use juggler_adapter::watcher::SessionWatcher;
use juggler_adapter::{AdapterError, AdapterRegistry, Capabilities, HarnessAdapter};
use juggler_app::features::tools;

/// A tool only its id describes.
struct Acme;

impl HarnessAdapter for Acme {
    fn id(&self) -> HarnessId {
        HarnessId::new("acme")
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::HEADLESS
    }
    fn verified_against(&self) -> &'static [&'static str] {
        &["1.0.0"]
    }
    fn discover(&self) -> DiscoveryReport {
        DiscoveryReport {
            adapter: self.id(),
            install: InstallStatus::NotFound,
            verification: VerificationStatus::Unverified,
            binary: None,
            notes: Vec::new(),
        }
    }
    fn prepare_account(&self, _state_dir: &Path) -> Result<EnvMap, AdapterError> {
        Ok(EnvMap::new())
    }
    fn launch_spec(
        &self,
        _: &Account,
        _: &Profile,
        cwd: &Path,
        _: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        Ok(LaunchSpec {
            program: PathBuf::from("/usr/bin/true"),
            args: Vec::new(),
            env: EnvMap::new(),
            cwd: cwd.to_path_buf(),
            needs_pty: false,
        })
    }
    fn watcher(&self, _root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        Err(AdapterError::Watch("acme has no watcher".into()))
    }
}

#[test]
fn the_catalog_serves_every_tool_as_data_the_primary_first() {
    let catalog = tools::catalog(&juggler_app::state::default_registry());
    let ids: Vec<&str> = catalog.iter().map(|tool| tool.id.as_str()).collect();
    assert_eq!(ids.len(), 2, "{ids:?}");
    let primary = &catalog[0];
    assert!(primary.flow.primary && primary.flow.first_run);
    assert_eq!(
        catalog.iter().filter(|tool| tool.flow.primary).count(),
        1,
        "one primary tool"
    );
    for tool in &catalog {
        assert!(
            !tool.display_name.is_empty() && tool.display_name != tool.id,
            "{} has a name",
            tool.id
        );
        assert!(!tool.vendor.is_empty(), "{} names its vendor", tool.id);
        assert!(
            tool.install_url
                .as_deref()
                .is_some_and(|url| url.starts_with("https://")),
            "{}",
            tool.id
        );
        assert!(
            !tool.setup_groups.is_empty(),
            "{} has first run's groups",
            tool.id
        );
        assert!(
            !tool.home_env.is_empty(),
            "{} is isolated by its own keys",
            tool.id
        );
        assert!(
            !tool.rules_files.is_empty(),
            "{} names its rules files",
            tool.id
        );
        assert!(tool.sign_in_own.contains("own sign-in"), "{}", tool.id);
    }
}

#[test]
fn a_tool_only_its_id_describes_gets_the_neutral_spec() {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(Acme));
    let catalog = tools::catalog(&registry);
    assert_eq!(catalog.len(), 1);
    let acme = &catalog[0];
    assert_eq!(acme.display_name, "acme");
    assert_eq!(acme.mark, None, "the letter mark, never a guessed logo");
    assert_eq!(acme.install_url, None);
    assert_eq!(acme.setup_groups.len(), 1);
    assert!(!acme.flow.first_run && !acme.flow.primary);
    assert!(!acme.supports.limits);
    assert_eq!(tools::display_name(&registry, "acme"), "acme");
    assert_eq!(
        tools::display_name(&registry, "unknown"),
        "unknown",
        "never a guessed product name"
    );
}
