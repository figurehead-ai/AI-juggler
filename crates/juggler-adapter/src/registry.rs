//! Adapter registry: id → adapter resolution and fleet-wide discovery.

use std::sync::Arc;

use crate::adapter::{HarnessAdapter, SharedAdapter};
use crate::discovery::DiscoveryReport;
use crate::handover::HandoverSupport;
use crate::model::HarnessId;

/// A shared handover-capable adapter handle.
pub type SharedHandover = Arc<dyn HandoverSupport>;

/// Holds every registered adapter. Registration happens at app startup
/// (built-ins) and later via manifests (post-v1, spec §4).
#[derive(Default)]
pub struct AdapterRegistry {
    adapters: Vec<SharedAdapter>,
    handovers: Vec<SharedHandover>,
}

impl AdapterRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an adapter; a duplicate id replaces the previous entry
    /// (last registration wins, which keeps manifest reloads simple).
    pub fn register(&mut self, adapter: Arc<dyn HarnessAdapter>) {
        self.adapters
            .retain(|existing| existing.id() != adapter.id());
        self.adapters.push(adapter);
    }

    /// Resolves an adapter by harness id.
    #[must_use]
    pub fn get(&self, id: &HarnessId) -> Option<SharedAdapter> {
        self.adapters
            .iter()
            .find(|adapter| adapter.id() == *id)
            .cloned()
    }

    /// Registers the handover capability for an adapter (the same
    /// `Arc` registered via [`Self::register`] coerced to the
    /// sub-trait). Providers without a registration are replay/pack
    /// only (T3/T4).
    pub fn register_handover(&mut self, support: SharedHandover) {
        self.handovers
            .retain(|existing| existing.id() != support.id());
        self.handovers.push(support);
    }

    /// Resolves the handover support for a harness id, when the
    /// adapter opted in.
    #[must_use]
    pub fn handover(&self, id: &HarnessId) -> Option<SharedHandover> {
        self.handovers
            .iter()
            .find(|support| support.id() == *id)
            .cloned()
    }

    /// All registered harness ids, in registration order.
    #[must_use]
    pub fn ids(&self) -> Vec<HarnessId> {
        self.adapters.iter().map(|a| a.id()).collect()
    }

    /// Runs discovery across every registered adapter.
    #[must_use]
    pub fn discover_all(&self) -> Vec<DiscoveryReport> {
        self.adapters.iter().map(|a| a.discover()).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use super::AdapterRegistry;
    use crate::adapter::HarnessAdapter;
    use crate::capabilities::Capabilities;
    use crate::discovery::{DiscoveryReport, InstallStatus, VerificationStatus};
    use crate::error::AdapterError;
    use crate::launch::{EnvMap, LaunchMode, LaunchSpec};
    use crate::model::{Account, HarnessId, Profile};
    use crate::watcher::SessionWatcher;
    use std::path::PathBuf;

    /// Minimal in-test adapter used to exercise the registry.
    struct StubAdapter {
        id: HarnessId,
    }

    impl HarnessAdapter for StubAdapter {
        fn id(&self) -> HarnessId {
            self.id.clone()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::HEADLESS
        }
        fn verified_against(&self) -> &'static [&'static str] {
            &["1.0.0"]
        }
        fn discover(&self) -> DiscoveryReport {
            DiscoveryReport {
                adapter: self.id.clone(),
                install: InstallStatus::Installed {
                    version: "1.0.0".into(),
                },
                verification: VerificationStatus::Verified,
                binary: Some(PathBuf::from("/usr/bin/true")),
                notes: Vec::new(),
            }
        }
        fn prepare_account(&self, _state_dir: &Path) -> Result<EnvMap, AdapterError> {
            Ok(EnvMap::new())
        }
        fn launch_spec(
            &self,
            _account: &Account,
            _profile: &Profile,
            cwd: &Path,
            _mode: &LaunchMode,
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
            Err(AdapterError::Watch("stub has no watcher".into()))
        }
    }

    #[test]
    fn registry_resolves_and_replaces() {
        let mut registry = AdapterRegistry::new();
        let stub = HarnessId::new("stub");
        registry.register(Arc::new(StubAdapter { id: stub.clone() }));
        assert!(registry.get(&stub).is_some());
        assert_eq!(registry.ids(), vec![stub.clone()]);

        // Re-registering replaces, not duplicates.
        registry.register(Arc::new(StubAdapter { id: stub.clone() }));
        assert_eq!(registry.ids().len(), 1);
        assert_eq!(registry.discover_all().len(), 1);

        // Unknown ids resolve to None.
        let ghost = HarnessId::new("ghost");
        assert!(registry.get(&ghost).is_none());
        assert!(!registry.ids().contains(&ghost));

        // The trait object is fully usable through the registry.
        let adapter = registry.get(&stub).unwrap();
        let state = std::env::temp_dir().join("juggler-registry-stub");
        assert!(adapter.prepare_account(&state).is_ok());
        let account = juggler_core::model::Account {
            id: juggler_core::model::AccountId::new("a"),
            harness: stub.clone(),
            label: "Stub".into(),
            plan: None,
            state_dir: state.clone(),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
        };
        let profile = juggler_core::model::Profile {
            id: juggler_core::model::ProfileId::new("p"),
            name: "stub profile".into(),
            account: account.id.clone(),
            settings_overlay: juggler_core::model::SettingsOverlay::default(),
            default_cwd: None,
        };
        let spec = adapter.launch_spec(
            &account,
            &profile,
            Path::new("/tmp"),
            &LaunchMode::Headless {
                prompt: "hi".into(),
            },
        );
        assert!(spec.is_ok());
        assert!(adapter.watcher(&state).is_err()); // stub has none, by design
    }
}
