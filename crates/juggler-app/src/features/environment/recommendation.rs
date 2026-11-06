//! AJ-E304: the recommendation read side: the scan inventory becomes one set-
//! level proposal. Staging only: no account, sign-in or copy happens here.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::environment::scan::environment_scan_full_impl;
use crate::features::providers::providers_discover_impl;
use crate::state::AppState;

/// Core implementation shared with integration tests (the
/// zero-writes-before-accept guardrail).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per the command docs.
pub fn recommendation_for_impl(
    state: &Arc<AppState>,
) -> AppResult<juggler_core::recommendation::Recommendation> {
    let report = environment_scan_full_impl(state, None)?;
    let candidates: Vec<juggler_core::recommendation::RecCandidate> = report
        .installs
        .iter()
        .map(|install| juggler_core::recommendation::RecCandidate {
            provider: install.harness.clone(),
            purpose: install.inference.purpose.clone(),
            signed_in: install.login == "logged_in",
            dormant: matches!(install.inference.activity.as_str(), "dormant" | "archive"),
            managed: install.managed,
            source_path: Some(install.path.clone()),
        })
        .collect();
    // Installed-but-unconfigured Providers: discover says installed,
    // the scan found no candidate there.
    let configured: std::collections::BTreeSet<String> = report
        .installs
        .iter()
        .map(|install| install.harness.clone())
        .collect();
    let gaps = providers_discover_impl(state)?
        .into_iter()
        .filter(|row| row.status != "not_installed" && !configured.contains(&row.provider))
        .map(|row| juggler_core::recommendation::RecProviderGap {
            provider: row.provider,
            display_name: row.display_name,
        })
        .collect::<Vec<_>>();
    Ok(juggler_core::recommendation::recommend(
        &candidates,
        &gaps,
        report.seeds.first().cloned(),
    ))
}
