//! AJ-E1525 (ADR-0019 migration step 1): the tools catalog. Each adapter
//! describes its tool in a `ToolSpec`; the catalog composes that with the
//! adapter's capabilities and isolation keys and serves it to the UI, so
//! no screen and no command branches on a tool id.

pub mod commands;
pub mod dto;

pub use dto::{SetupGroupInfo, ToolFlowInfo, ToolSpecInfo, ToolSupportsInfo, ToolTermsInfo};

use std::sync::Arc;

use juggler_adapter::{AdapterRegistry, Capabilities, HarnessAdapter, ToolKind};

/// Whether this run may start a tool's program to ask its version (the
/// 0.2 walk, F19): yes by default; never under the read-only test bridge,
/// which promises to start no vendor program at all. Every discovery in
/// the app goes through [`discover`], so this is the one place it's
/// decided.
#[derive(Debug)]
pub struct ToolPrograms(std::sync::atomic::AtomicBool);

impl Default for ToolPrograms {
    fn default() -> Self {
        Self(std::sync::atomic::AtomicBool::new(true))
    }
}

impl ToolPrograms {
    /// From now on, discovery reads files only (the bridge's promise).
    pub fn stop(&self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Whether a tool may be started to ask its version.
    #[must_use]
    pub fn allowed(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// What's installed of `adapter`'s tool: its own `discover` (which runs
/// `--version`), or, when this run starts no tool, what its files say
/// (found with its version not asked, or not found). The one seam every
/// discovery in the app goes through.
#[must_use]
pub fn discover(
    state: &crate::state::AppState,
    adapter: &dyn HarnessAdapter,
) -> juggler_adapter::DiscoveryReport {
    if state.tool_programs.allowed() {
        adapter.discover()
    } else {
        juggler_adapter::DiscoveryReport::located(adapter.id(), adapter.locate())
    }
}

/// One registered tool, as the UI sees it.
#[must_use]
pub fn spec_info(adapter: &dyn HarnessAdapter) -> ToolSpecInfo {
    let spec = adapter.spec();
    let app_meter = spec.app_meter && spec.desktop_app.is_some();
    let thread_link = adapter.host_apps().iter().any(|app| {
        app.kind == juggler_adapter::live::HostKind::Desktop && app.thread_link.is_some()
    });
    ToolSpecInfo {
        id: spec.id,
        display_name: spec.display_name,
        short_name: spec.short_name,
        vendor: spec.vendor,
        mark: spec.mark,
        vendor_mark: spec.vendor_mark,
        install_url: spec.install_url,
        install_command: spec.install_command,
        sign_in_own: spec.sign_in.own,
        sign_in_short: spec.sign_in.short,
        same_as: spec.same_as,
        desktop_app: spec.desktop_app,
        host_app: adapter.host_apps().into_iter().next().map(|app| app.id),
        kinds: spec
            .kinds
            .iter()
            .map(|kind| match kind {
                ToolKind::Cli => "cli".to_owned(),
                ToolKind::Desktop => "desktop".to_owned(),
            })
            .collect(),
        rules_files: spec.rules_files,
        scan_hint: spec.scan_hint,
        setup_groups: spec
            .setup_groups
            .into_iter()
            .map(|group| SetupGroupInfo {
                id: group.id,
                title: group.title,
                path_contains: group.path_contains,
                origin: group.origin,
                shows_install: group.shows_install,
            })
            .collect(),
        default_label: spec.default_label,
        flow: ToolFlowInfo {
            first_run: spec.flags.first_run,
            primary: spec.flags.primary,
        },
        terms: spec.terms.map(|terms| ToolTermsInfo {
            name: terms.name,
            url: terms.url,
            accounts: terms.accounts,
            checked: terms.checked,
        }),
        home_env: adapter
            .isolation_env_keys()
            .iter()
            .map(|key| (*key).to_owned())
            .collect(),
        supports: ToolSupportsInfo {
            in_place: adapter.can_use_in_place(),
            limits: adapter
                .capabilities()
                .contains(Capabilities::USAGE_QUOTA_FEED),
            app_meter,
            thread_link,
        },
        runtime: adapter.driven().map(|driven| dto::ToolRuntimeInfo {
            label: driven.label().to_owned(),
            beta: driven.maturity() == juggler_adapter::runtime::Maturity::Beta,
            terms_note: driven.terms_note().map(str::to_owned),
        }),
    }
}

/// Every registered tool, the primary one first, then in registration
/// order.
#[must_use]
pub fn catalog(registry: &AdapterRegistry) -> Vec<ToolSpecInfo> {
    let mut rows: Vec<ToolSpecInfo> = registry
        .ids()
        .iter()
        .filter_map(|id| registry.get(id))
        .map(|adapter| spec_info(adapter.as_ref()))
        .collect();
    rows.sort_by_key(|row| !row.flow.primary);
    rows
}

/// The display name of tool `id`: its spec's, or the id itself for a
/// tool no adapter describes (never a guessed product name).
#[must_use]
pub fn display_name(registry: &AdapterRegistry, id: &str) -> String {
    registry
        .get(&juggler_core::model::HarnessId::new(id))
        .map_or_else(|| id.to_owned(), |adapter| adapter.spec().display_name)
}

/// The install page of tool `id`, when it has one.
#[must_use]
pub fn install_url(registry: &AdapterRegistry, id: &str) -> Option<String> {
    registry
        .get(&juggler_core::model::HarnessId::new(id))
        .and_then(|adapter| adapter.spec().install_url)
}

/// The catalog for the app's registry.
#[must_use]
pub fn catalog_impl(state: &Arc<crate::state::AppState>) -> Vec<ToolSpecInfo> {
    catalog(&state.registry)
}
