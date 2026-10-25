//! AJ-E2303 (D-40): the app-server protocol AI Juggler speaks, pinned.
//!
//! - [`DRIVEN_FROM`]: the oldest Codex AI Juggler drives (the version its
//!   surface was recorded from, `tests/fixtures/codex-app-server/`). An
//!   older Codex starts in the terminal.
//! - [`SURFACE`]: every method the codec sends or reads. A newer Codex is
//!   driven only when its own generated schema (`codex app-server
//!   generate-json-schema`, run offline with a scratch `CODEX_HOME`)
//!   still has every one of them; otherwise it starts in the terminal with
//!   the missing names as the reason, never a guess.
//! - Stable methods only: `initialize` never asks for `experimentalApi`.

use std::collections::BTreeSet;
use std::path::Path;

/// The oldest Codex driven through app-server (verified 2026-10-25).
pub const DRIVEN_FROM: &str = "0.163.0";

/// Who sends a method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// AI Juggler's requests.
    ClientRequest,
    /// The server's requests (approvals, questions).
    ServerRequest,
    /// The server's notifications.
    Notification,
}

impl Kind {
    /// The schema file that lists this kind's methods.
    #[must_use]
    pub fn schema_file(self) -> &'static str {
        match self {
            Self::ClientRequest => "ClientRequest.json",
            Self::ServerRequest => "ServerRequest.json",
            Self::Notification => "ServerNotification.json",
        }
    }
}

/// Every method the codec uses.
pub const SURFACE: &[(Kind, &str)] = &[
    (Kind::ClientRequest, "initialize"),
    (Kind::ClientRequest, "thread/start"),
    (Kind::ClientRequest, "thread/resume"),
    (Kind::ClientRequest, "turn/start"),
    (Kind::ClientRequest, "turn/steer"),
    (Kind::ClientRequest, "turn/interrupt"),
    (Kind::ClientRequest, "thread/compact/start"),
    (Kind::ClientRequest, "model/list"),
    (Kind::ClientRequest, "account/rateLimits/read"),
    (Kind::ServerRequest, "item/commandExecution/requestApproval"),
    (Kind::ServerRequest, "item/fileChange/requestApproval"),
    (Kind::ServerRequest, "item/permissions/requestApproval"),
    (Kind::ServerRequest, "item/tool/requestUserInput"),
    (Kind::ServerRequest, "mcpServer/elicitation/request"),
    (Kind::Notification, "thread/started"),
    (Kind::Notification, "thread/status/changed"),
    (Kind::Notification, "turn/started"),
    (Kind::Notification, "turn/completed"),
    (Kind::Notification, "item/completed"),
    (Kind::Notification, "item/agentMessage/delta"),
    (Kind::Notification, "item/commandExecution/outputDelta"),
    (Kind::Notification, "serverRequest/resolved"),
    (Kind::Notification, "account/rateLimits/updated"),
    (Kind::Notification, "thread/tokenUsage/updated"),
    (Kind::Notification, "thread/compacted"),
    (Kind::Notification, "error"),
];

/// A version's numbers (`0.163.0` → `[0, 163, 0]`); `None` for anything
/// else.
#[must_use]
pub fn numbers(version: &str) -> Option<Vec<u64>> {
    let core = version.split(['-', '+']).next()?;
    core.split('.').map(|part| part.parse().ok()).collect()
}

/// Whether `version` is at least [`DRIVEN_FROM`].
#[must_use]
pub fn new_enough(version: &str) -> bool {
    match (numbers(version), numbers(DRIVEN_FROM)) {
        (Some(have), Some(need)) => have >= need,
        _ => false,
    }
}

/// The methods a schema file lists.
fn listed(dir: &Path, kind: Kind) -> BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(dir.join(kind.schema_file())) else {
        return BTreeSet::new();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
        return BTreeSet::new();
    };
    doc.get("oneOf")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|variant| {
            variant
                .pointer("/properties/method/enum/0")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

/// The [`SURFACE`] methods a generated schema (the folder
/// `generate-json-schema` wrote) lacks; empty when it has them all.
#[must_use]
pub fn missing_in(dir: &Path) -> Vec<&'static str> {
    let lists = [
        (Kind::ClientRequest, listed(dir, Kind::ClientRequest)),
        (Kind::ServerRequest, listed(dir, Kind::ServerRequest)),
        (Kind::Notification, listed(dir, Kind::Notification)),
    ];
    SURFACE
        .iter()
        .filter(|(kind, method)| {
            !lists
                .iter()
                .any(|(listed_kind, methods)| listed_kind == kind && methods.contains(*method))
        })
        .map(|(_, method)| *method)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(new_enough("0.163.0"));
        assert!(new_enough("0.164.0"));
        assert!(new_enough("1.0.0"));
        assert!(!new_enough("0.157.2"));
        assert!(!new_enough("0.99.0"));
        assert!(!new_enough("nightly"));
        assert_eq!(numbers("0.164.0-alpha.1"), Some(vec![0, 164, 0]));
    }

    #[test]
    fn a_schema_without_a_method_is_named() {
        let dir = tempfile::tempdir().unwrap();
        let write = |file: &str, methods: &[&str]| {
            let variants: Vec<serde_json::Value> = methods
                .iter()
                .map(|m| serde_json::json!({"properties": {"method": {"enum": [m]}}}))
                .collect();
            std::fs::write(
                dir.path().join(file),
                serde_json::json!({ "oneOf": variants }).to_string(),
            )
            .unwrap();
        };
        let of = |kind: Kind| -> Vec<&str> {
            SURFACE
                .iter()
                .filter(|(k, _)| *k == kind)
                .map(|(_, m)| *m)
                .collect()
        };
        write("ClientRequest.json", &of(Kind::ClientRequest));
        write("ServerRequest.json", &of(Kind::ServerRequest));
        write("ServerNotification.json", &of(Kind::Notification));
        assert!(missing_in(dir.path()).is_empty());
        let mut fewer = of(Kind::ServerRequest);
        fewer.retain(|m| *m != "item/tool/requestUserInput");
        write("ServerRequest.json", &fewer);
        assert_eq!(missing_in(dir.path()), vec!["item/tool/requestUserInput"]);
        assert_eq!(missing_in(&dir.path().join("none")).len(), SURFACE.len());
    }
}
