//! First run as the QA harnesses do it: the scan, then "Bring in N
//! accounts" with "Use them where they are", into a fresh data folder.

use std::path::Path;
use std::sync::Arc;

use juggler_app::features::account::import::{ImportRequest, account_import_mode_impl};
use juggler_app::features::environment::dto::ScanReportInfo;
use juggler_app::features::environment::scan::environment_scan_full_impl;
use juggler_app::features::tools::scan_wire::DiscoveredInfo;
use juggler_app::state::AppState;

/// The scan, and each setup it found brought in where it is: the default
/// `~/.claude` first, then the profiles by path, then Codex. Each is
/// named after its folder, or "Account A"… when `redacted`. Returns the
/// scan and one line per import.
///
/// # Panics
///
/// When the scan itself fails.
pub fn bring_in_all(
    state: &Arc<AppState>,
    home: &Path,
    redacted: bool,
) -> (ScanReportInfo, Vec<serde_json::Value>) {
    let scan = environment_scan_full_impl(state, None).unwrap();
    let mut order: Vec<&DiscoveredInfo> = scan.installs.iter().collect();
    order.sort_by_key(|found| {
        let path = Path::new(found.path.trim_end_matches('/'));
        (
            found.harness != "claude-code",
            path != home.join(".claude"),
            found.path.clone(),
        )
    });
    let mut imports = Vec::new();
    for (n, found) in order.into_iter().enumerate() {
        let label = if redacted {
            format!("Account {}", char::from(b'A' + u8::try_from(n).unwrap()))
        } else {
            Path::new(&found.path)
                .file_name()
                .map(|name| name.to_string_lossy().trim_start_matches('.').to_string())
                .unwrap_or_else(|| found.harness.clone())
        };
        let result = account_import_mode_impl(
            state,
            &found.harness,
            &found.path,
            &label,
            ImportRequest {
                origin: Some(found.origin.clone()),
                in_place: true,
                ..ImportRequest::default()
            },
        );
        imports.push(match result {
            Ok(done) => serde_json::json!({"path": found.path, "ok": true, "login": done.login,
                                           "account": done.account.id.as_str(), "in_place": done.account.in_place}),
            Err(error) => serde_json::json!({"path": found.path, "ok": false, "error": format!("{error:?}")}),
        });
    }
    (scan, imports)
}
