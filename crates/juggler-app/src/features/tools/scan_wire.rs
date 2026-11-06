//! The scan's wire forms (AJ-E303, AJ-E2906): one discovered setup as
//! the found step reads it, with the inference block — each field's
//! value, its confidence word and the rule that decided it.

use serde::Serialize;
use ts_rs::TS;

use crate::features::account::login_probe_name;

/// An existing harness setup found by the onboarding environment scan
/// (spec §12). Metadata-only discovery; the wizard shows `notes`
/// verbatim.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DiscoveredInfo {
    /// Harness id.
    pub harness: String,
    /// Config dir path.
    pub path: String,
    /// `vendor-default` | `env-override` | `home-pattern` |
    /// `shell-alias` | `other-manager`.
    pub origin: String,
    /// AJ-E2906: the other app whose settings point at this folder
    /// (`other-manager`), for "named by …".
    pub set_by: Option<String>,
    /// `logged_in` | `logged_out`.
    pub login: String,
    /// Approximate bytes on disk (caches excluded).
    #[ts(type = "number")]
    pub bytes: u64,
    /// Most recent activity, unix seconds (when known).
    #[ts(type = "number")]
    pub last_used_secs: Option<u64>,
    /// True when already managed by AI Juggler.
    pub managed: bool,
    /// Human-facing hints shown by the wizard.
    pub notes: Vec<String>,
    /// AJ-E303: the inferred identity (purpose/login/activity with
    /// Found/Likely/Guessed confidence words — never scores).
    pub inference: InferenceInfo,
    /// AJ-E303: every signal that found this dir (merged duplicates:
    /// alias + home-pattern → ONE candidate with both signals).
    pub signals: Vec<String>,
    /// Where this setup is open right now: `desktop` (the Claude desktop
    /// app) and/or `terminal` (`claude` outside it), from the running
    /// processes ([`crate::running_setups`]). Empty = not open now.
    pub in_use: Vec<String>,
}

/// AJ-E303: the inference block on one candidate (rule-table fields,
/// §4.1 — deterministic, offline, first match wins per field).
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InferenceInfo {
    /// Inferred account purpose ("Work", "Personal", humanised name).
    pub purpose: String,
    /// found | likely | guessed.
    pub purpose_confidence: String,
    /// signed-in | maybe-expired | not-signed-in.
    pub login: String,
    /// found | likely | guessed.
    pub login_confidence: String,
    /// active | dormant | archive.
    pub activity: String,
    /// found | likely | guessed.
    pub activity_confidence: String,
    /// Human words for the activity tier ("Dormant 40 d").
    pub activity_words: String,
    /// True when the login lives in the OS keychain (the re-login
    /// caveat copy applies after import).
    pub keychain_bound: bool,
    /// AJ-E2906: the rule that decided the purpose: `usual-folder` |
    /// `other-app` | `name-word` | `folder-name` (words in the UI).
    pub purpose_rule: String,
    /// `sign-in-present` | `system-keychain` | `sign-in-empty` |
    /// `no-sign-in`.
    pub login_rule: String,
    /// `newest-conversation` | `no-conversations`.
    pub activity_rule: String,
}

/// The wire form of one discovered install.
pub(crate) fn discovered_info_of(
    found: juggler_adapter::environment::DiscoveredInstall,
) -> DiscoveredInfo {
    let credentials_len = juggler_adapters::sign_in::metadata(found.harness.as_str(), &found.path)
        .map(|meta| meta.len());
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or_default();
    let idle_secs = found
        .last_used_secs
        .map(|last| now_secs.saturating_sub(last));
    let usual_folder = matches!(
        found.origin,
        juggler_adapter::environment::InstallOrigin::VendorDefault
    );
    DiscoveredInfo {
        inference: inference_info_of(
            &found.path,
            matches!(
                found.login,
                juggler_adapter::environment::LoginProbe::LoggedIn
            ),
            credentials_len,
            idle_secs,
            (found.managed, usual_folder),
        ),
        set_by: None,
        signals: vec![found.origin.as_str().to_owned()],
        harness: found.harness.as_str().to_string(),
        path: found.path.to_string_lossy().into_owned(),
        origin: found.origin.as_str().to_string(),
        login: login_probe_name(found.login),
        bytes: found.bytes,
        last_used_secs: found.last_used_secs,
        managed: found.managed,
        in_use: Vec::new(),
        notes: found.notes,
    }
}

/// The wire inference block for one candidate (the one extra read is
/// a credentials-file SIZE — metadata, never contents).
pub(crate) fn inference_info_of(
    path: &std::path::Path,
    probe_logged_in: bool,
    credentials_len: Option<u64>,
    idle_secs: Option<u64>,
    (managed, usual_folder): (bool, bool),
) -> InferenceInfo {
    let inference =
        juggler_adapters::inference::infer_candidate(&juggler_adapters::inference::InferInput {
            path,
            manager_name: None,
            probe_logged_in,
            credentials_len,
            idle_secs,
            managed,
            usual_folder,
        });
    InferenceInfo {
        purpose: inference.purpose.value,
        purpose_confidence: inference.purpose.confidence.as_str().to_owned(),
        login: inference.login.value.as_str().to_owned(),
        login_confidence: inference.login.confidence.as_str().to_owned(),
        activity: inference.activity.value.as_str().to_owned(),
        activity_confidence: inference.activity.confidence.as_str().to_owned(),
        activity_words: inference.activity.value.words(),
        keychain_bound: inference.keychain_bound,
        purpose_rule: inference.purpose.rule.as_str().to_owned(),
        login_rule: inference.login.rule.as_str().to_owned(),
        activity_rule: inference.activity.rule.as_str().to_owned(),
    }
}
