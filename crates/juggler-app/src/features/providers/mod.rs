//! AJ-E301: the wizard's install gate: one row per Provider from its
//! adapter's own discovery, probed in parallel.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::providers::dto::{ProviderCapabilityInfo, ProviderInstallInfo};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// AJ-E301: the wizard's Step-0 install gate — one row per Provider
// from the adapters' own `discover()`, probed in PARALLEL off the UI
// thread; a missing binary is caught before any account is created.

/// Vendor display metadata for one Provider, from its adapter's spec
/// (ADR-0019): the name, and the install page ("" when there is none).
pub(crate) fn provider_display(
    registry: &juggler_adapter::AdapterRegistry,
    provider: &str,
) -> (String, String) {
    (
        crate::features::tools::display_name(registry, provider),
        crate::features::tools::install_url(registry, provider).unwrap_or_default(),
    )
}

/// Core implementation shared with integration tests.
#[must_use]
pub fn providers_capabilities_impl(state: &Arc<AppState>) -> Vec<ProviderCapabilityInfo> {
    state
        .registry
        .ids()
        .into_iter()
        .filter_map(|id| {
            state
                .registry
                .get(&id)
                .map(|adapter| ProviderCapabilityInfo {
                    provider: id.to_string(),
                    can_use_in_place: adapter.can_use_in_place(),
                    reports_limits: adapter
                        .capabilities()
                        .contains(juggler_adapter::Capabilities::USAGE_QUOTA_FEED),
                })
        })
        .collect()
}

/// Core implementation shared with integration tests (the no-binary
/// Step-0 fixture).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per the command docs.
pub fn providers_discover_impl(state: &Arc<AppState>) -> AppResult<Vec<ProviderInstallInfo>> {
    let rows = providers_discover_rows(state);
    // AJ-E808: remember what is installed, for the register's reset.
    if let Ok(mut versions) = state.provider_versions.lock() {
        for row in &rows {
            if let Some(version) = &row.version {
                versions.insert(row.provider.clone(), version.clone());
            }
        }
    }
    Ok(rows)
}

/// AJ-E808: the installed versions the fidelity register needs to
/// confirm a PASS for `from → to`. Discovery runs (once per launch)
/// only when the register holds a decided cell for that pair — while
/// every cell is UNVALIDATED there is nothing to confirm.
pub(crate) fn installed_versions_for(
    state: &Arc<AppState>,
    from: &str,
    to: &str,
) -> std::collections::HashMap<String, String> {
    let decided = juggler_handover::register::embedded()
        .pair(from, to)
        .is_some_and(|pair| {
            pair.cells
                .iter()
                .any(|cell| cell.verdict != juggler_handover::register::Verdict::Unvalidated)
        });
    if !decided {
        return std::collections::HashMap::new();
    }
    let cached = state
        .provider_versions
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();
    if !cached.is_empty() {
        return cached;
    }
    let _ = providers_discover_impl(state);
    state
        .provider_versions
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default()
}

/// Runs discovery once per launch if any register cell is decided and
/// nothing is cached yet, so later lookups are cache hits.
pub(crate) fn warm_provider_versions(state: &Arc<AppState>) {
    let decided = juggler_handover::register::embedded()
        .pairs
        .iter()
        .any(|pair| {
            pair.cells
                .iter()
                .any(|cell| cell.verdict != juggler_handover::register::Verdict::Unvalidated)
        });
    let empty = state
        .provider_versions
        .lock()
        .map_or(true, |v| v.is_empty());
    if decided && empty {
        let _ = providers_discover_impl(state);
    }
}

fn providers_discover_rows(state: &Arc<AppState>) -> Vec<ProviderInstallInfo> {
    // Commands run this off the main thread (`crate::ipc::blocking`); the per-adapter
    // probes additionally run on their OWN threads (parallel, one
    // subprocess each — the ≤800 ms p95 budget for 3 Providers).
    {
        let reports: Vec<juggler_adapter::discovery::DiscoveryReport> =
            crate::features::tools::per_adapter(&state.registry, |id, adapter| {
                adapter.map_or_else(
                    || juggler_adapter::discovery::DiscoveryReport {
                        adapter: id.clone(),
                        install: juggler_adapter::discovery::InstallStatus::NotFound,
                        verification: juggler_adapter::discovery::VerificationStatus::Unverified,
                        binary: None,
                        notes: Vec::new(),
                    },
                    |adapter| crate::features::tools::discover(state, adapter.as_ref()),
                )
            });
        reports
            .into_iter()
            .map(|report| {
                let (display_name, install_url) =
                    provider_display(&state.registry, report.adapter.as_str());
                let (status, version) = match &report.install {
                    juggler_adapter::discovery::InstallStatus::Installed { version } => {
                        let verified = matches!(
                            report.verification,
                            juggler_adapter::discovery::VerificationStatus::Verified
                        );
                        (
                            if verified { "installed" } else { "unverified" },
                            Some(version.clone()),
                        )
                    }
                    juggler_adapter::discovery::InstallStatus::NotFound => ("not_installed", None),
                    // F19: found from its files; this run starts no tool.
                    juggler_adapter::discovery::InstallStatus::NotAsked => ("found", None),
                    juggler_adapter::discovery::InstallStatus::Failed { .. } => {
                        ("probe_failed", None)
                    }
                };
                let adapter_meta = state.registry.get(&juggler_core::model::HarnessId::new(
                    report.adapter.as_str(),
                ));
                let (verified_against, kind, single_instance_note) = match adapter_meta {
                    Some(adapter) => (
                        adapter
                            .verified_against()
                            .iter()
                            .map(|v| (*v).to_owned())
                            .collect(),
                        if adapter.desktop_isolation_label().is_some() {
                            "desktop".to_owned()
                        } else {
                            "terminal".to_owned()
                        },
                        adapter.desktop_isolation_label().map(str::to_owned),
                    ),
                    None => (Vec::new(), "terminal".to_owned(), None),
                };
                ProviderInstallInfo {
                    provider: report.adapter.to_string(),
                    display_name: display_name.clone(),
                    install_url,
                    status: status.to_owned(),
                    version,
                    binary: report
                        .binary
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned()),
                    notes: report.notes,
                    verified_against,
                    kind,
                    single_instance_note,
                }
            })
            .collect()
    }
}
