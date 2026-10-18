//! The `HarnessAdapter` contract (spec §4, ADR-0002).

use std::path::Path;
use std::sync::Arc;

use crate::capabilities::Capabilities;
use crate::discovery::DiscoveryReport;
use crate::environment::{DiscoveredInstall, ImportOutcome, LoginUrl};
use crate::error::AdapterError;
use crate::launch::{DesktopLaunch, EnvMap, LaunchMode, LaunchSpec};
use crate::model::{Account, HarnessId, Profile};
use crate::watcher::SessionWatcher;

/// The only place harness specifics are allowed to live.
///
/// Implementations:
/// - are **read-only** toward harness state except inside
///   [`HarnessAdapter::prepare_account`], the single writer to a managed
///   account directory;
/// - declare `verified_against` versions and capability-probe at runtime —
///   never assume unadvertised capabilities;
/// - never touch vendor credentials' contents.
///
/// The trait is public, semver-protected API (guarded by
/// `cargo-semver-checks` in CI once the first stable release ships).
pub trait HarnessAdapter: Send + Sync {
    /// The harness this adapter integrates, e.g. `claude-code`.
    fn id(&self) -> HarnessId;

    /// What this harness integration supports.
    fn capabilities(&self) -> Capabilities;

    /// Harness versions this adapter was validated against (spec §4
    /// version-pinning rule). Discovery compares against this list.
    fn verified_against(&self) -> &'static [&'static str];

    /// Probes the machine: is the harness installed, at what version, and is
    /// that version verified? Never fails — problems are carried in the
    /// report.
    #[must_use]
    fn discover(&self) -> DiscoveryReport;

    /// Prepares (idempotently) an isolated account state directory and
    /// returns the environment overlay that points the harness at it.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::AccountDir`] when the directory cannot be
    /// created or initialized.
    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError>;

    /// Builds the launch specification for one session. The account carries
    /// the managed state directory the adapter itself prepared earlier via
    /// [`HarnessAdapter::prepare_account`].
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::InvalidLaunch`] for unsupported
    /// profile/mode combinations.
    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError>;

    /// Builds a desktop launch for the account (AJ-E413): the
    /// windowed app tracked as an Instance. Providers without a
    /// desktop app keep the default — `Unsupported`, and the core
    /// never offers the launch (capability-gated).
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Unsupported`] when the provider ships no
    /// desktop app, [`AdapterError::InvalidLaunch`] for a missing binary.
    fn desktop_launch(&self, account: &Account) -> Result<DesktopLaunch, AdapterError> {
        let _ = account;
        Err(AdapterError::Unsupported(
            "this provider ships no desktop app".into(),
        ))
    }

    /// The per-Provider desktop isolation claim (AJ-E413 Req.1), shown
    /// verbatim on every desktop Instance of this provider. `None` for
    /// providers without a desktop app; single-account apps carry
    /// `Single-account app — Juggler tracks, cannot isolate` — never an
    /// implied isolation the vendor does not provide.
    fn desktop_isolation_label(&self) -> Option<&'static str> {
        None
    }

    /// Creates a watcher over the account's transcript root.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Watch`] when the root cannot be observed.
    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError>;

    /// Scans the machine for existing installations of this harness
    /// (smart onboarding, spec §12). Metadata-only: presence, size,
    /// timestamps — never file contents. Adapters that cannot scan return
    /// an empty list; scanning never fails.
    #[must_use]
    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        Vec::new()
    }

    /// Shell rc assignments pointing at this harness's config dirs
    /// (AJ-E302 S5): path values only, source file + line recorded.
    /// Default: none — adapters opt in.
    fn scan_shell_aliases(&self) -> Vec<crate::environment::AliasHint> {
        Vec::new()
    }

    /// Other manager tools holding this harness's setups (AJ-E302
    /// S3/S4): metadata only (existence, counts, names — never
    /// tokens). Default: none — adapters opt in.
    fn scan_managers(&self) -> Vec<crate::environment::ManagerHint> {
        Vec::new()
    }

    /// Copy-imports an existing config dir (found via
    /// [`HarnessAdapter::scan_environment`]) into a fresh managed state
    /// dir. Read-only toward `source`: a byte copy with rebuildable caches
    /// skipped; the source is never written to or marked.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Unsupported`] when this adapter cannot
    /// import, [`AdapterError::AccountDir`] for path/size problems and
    /// copy failures (message carries the plain-language reason).
    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        let _ = (source, dest);
        Err(AdapterError::Unsupported(
            "this adapter cannot import existing configurations".into(),
        ))
    }

    /// AJ-E307 Req.3: the pre-copy source inspection — metadata-only;
    /// the Confirm-source step's data (sizes, breakdown, evidence).
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] by default; adapters with a copy
    /// import surface their own typed failures.
    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<crate::environment::SourceInspect, AdapterError> {
        let _ = source;
        Err(AdapterError::Unsupported(
            "this adapter cannot inspect configurations".into(),
        ))
    }

    /// AJ-E307 Req.4: the settings-only degraded import — history
    /// dirs skipped, each skip recorded with its size.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] by default; otherwise the same
    /// conditions as [`HarnessAdapter::import_account`].
    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        let _ = (source, dest);
        Err(AdapterError::Unsupported(
            "this adapter has no degraded import".into(),
        ))
    }

    /// The vendor's own web sign-in page, when one exists (the maximum
    /// login aid: we open the page, the user completes the vendor's flow).
    #[must_use]
    fn login_url(&self) -> Option<LoginUrl> {
        None
    }

    /// Builds the launch spec for the vendor's own interactive login flow
    /// against a config dir — the guided-terminal login primitive (spec
    /// §4.1): AI Juggler spawns it in an embedded terminal; the user
    /// completes the OAuth flow; nothing is automated.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Unsupported`] when the harness has no
    /// terminal login flow, [`AdapterError::InvalidLaunch`] when the
    /// binary cannot be resolved.
    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        let _ = state_dir;
        Err(AdapterError::Unsupported(
            "this harness has no terminal login flow".into(),
        ))
    }

    /// The environment variables that fence an Account's isolated
    /// state for this Provider (AJ-E309). Launch-settings overrides
    /// may never set them — an override that repointed the config
    /// dir would silently break isolation. The FIRST key is the one
    /// that points the Provider at the state dir (the snippet sets
    /// it); later keys are reserved but never emitted.
    #[must_use]
    fn isolation_env_keys(&self) -> &'static [&'static str] {
        &[]
    }

    /// The command a user types to start this Provider against
    /// `state_dir` by hand — the Expert-setup snippet and the
    /// external-terminal login fallback (AJ-E309 Req.5). `None` when
    /// the Provider has no terminal entry point.
    #[must_use]
    fn shell_command(&self) -> Option<&'static str> {
        None
    }
}

/// Single-quotes `value` for a POSIX shell (`'` becomes `'\''`), so a
/// copied snippet survives spaces and quotes in managed paths.
#[must_use]
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Builds the copyable shell line for `adapter` against `state_dir`
/// plus any extra environment (Launch-settings overrides), isolation
/// keys first. `None` when the Provider has no terminal entry point.
#[must_use]
pub fn shell_snippet(
    adapter: &dyn HarnessAdapter,
    state_dir: &Path,
    extra_env: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    let command = adapter.shell_command()?;
    let dir = state_dir.to_string_lossy();
    let mut parts: Vec<String> = adapter
        .isolation_env_keys()
        .iter()
        .take(1)
        .map(|key| format!("{key}={}", shell_quote(&dir)))
        .collect();
    for (key, value) in extra_env {
        if !adapter.isolation_env_keys().contains(&key.as_str()) {
            parts.push(format!("{key}={}", shell_quote(value)));
        }
    }
    parts.push(command.to_owned());
    Some(parts.join(" "))
}

/// Shared adapter handle type used by the registry and supervisor.
pub type SharedAdapter = Arc<dyn HarnessAdapter>;

/// ADR-0008 vocabulary alias: adapters integrate one *Provider*.
pub type ProviderAdapter = dyn HarnessAdapter;
