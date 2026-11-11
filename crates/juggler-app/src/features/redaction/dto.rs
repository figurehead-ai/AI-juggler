//! The redaction types on the wire (exported to TypeScript).

use crate::features::redaction::redaction_line;

/// The resolved policy a switch surface shows (the confirm line is
/// verbatim per Req.2).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RedactionPolicyInfo {
    /// The user's global choice wire name (`strip|keep|ask`).
    pub global: Option<String>,
    /// This Provider's override wire name, when set.
    pub r#override: Option<String>,
    /// override-if-set else global; `null` = unset (the first pack
    /// is blocked until the user chooses).
    pub effective: Option<String>,
    /// `global` | `override` | `unset`.
    #[ts(type = "\"global\" | \"override\" | \"unset\"")]
    pub source: String,
    /// The verbatim confirm line ("Keys and passwords for {Provider}: …").
    pub line: String,
}

impl RedactionPolicyInfo {
    /// Builds the info block (line verbatim per Req.2).
    #[must_use]
    pub fn from_parts(
        provider_name: &str,
        global: Option<juggler_handover::redaction::RedactionPolicy>,
        r#override: Option<juggler_handover::redaction::RedactionPolicy>,
    ) -> Self {
        let effective = juggler_handover::redaction::resolve_policy(global, r#override);
        Self {
            global: global.map(|policy| policy.as_str().to_owned()),
            r#override: r#override.map(|policy| policy.as_str().to_owned()),
            effective: effective.map(|policy| policy.as_str().to_owned()),
            source: match (&global, &r#override) {
                (_, Some(_)) => "override".to_owned(),
                (Some(_), None) => "global".to_owned(),
                (None, None) => "unset".to_owned(),
            },
            line: redaction_line(provider_name, global, r#override),
        }
    }
}

/// One listed secret for the preview UI (masked — values never
/// cross the bridge).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RedactionItemInfo {
    /// Detection kind (`sk-key`, `bearer`, `env-value`,
    /// `private-key`, `zcode-rollout`).
    pub kind: String,
    /// Masked preview (first char + `•••` + last 4).
    pub preview: String,
    /// Segment ordinal (conversation turns first).
    pub segment: u32,
}
