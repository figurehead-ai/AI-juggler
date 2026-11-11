//! AJ-E208: the pack redaction policy: a global choice and a per-Provider
//! override (D-14). Unset blocks pack generation.

pub mod commands;
pub mod dto;

use std::sync::Arc;

use juggler_core::storage::Storage;

use crate::features::redaction::dto::{RedactionItemInfo, RedactionPolicyInfo};
use crate::features::switch::back::transcript_side;
use crate::state::AppState;
use crate::{
    commands::lock_storage_arc,
    error::{AppError, AppResult},
};

// ---------------------------------------------------------------------------
// AJ-E208: pack redaction policy — global choice + per-Provider
// override (D-14). Always the user's remembered choice; unset BLOCKS
// pack generation (no code path applies an unchosen default).

/// Settings KV keys (schema-less strings — persisted across
/// restarts; round-trip pinned by test).
const REDACTION_GLOBAL_KEY: &str = "redaction_global";

const REDACTION_OVERRIDES_KEY: &str = "redaction_overrides";

/// The stored global choice.
pub(crate) fn redaction_global_of(
    storage: &Storage,
) -> Option<juggler_handover::redaction::RedactionPolicy> {
    storage
        .app_settings()
        .get(REDACTION_GLOBAL_KEY)
        .ok()
        .flatten()
        .and_then(|wire| juggler_handover::redaction::RedactionPolicy::from_wire(&wire))
}

/// The stored override for one Provider.
pub(crate) fn redaction_override_of(
    storage: &Storage,
    provider: &str,
) -> Option<juggler_handover::redaction::RedactionPolicy> {
    let overrides: std::collections::BTreeMap<String, String> = storage
        .app_settings()
        .get(REDACTION_OVERRIDES_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    overrides
        .get(provider)
        .and_then(|wire| juggler_handover::redaction::RedactionPolicy::from_wire(wire))
}

/// Builds the confirm line verbatim (Req.2) — "(your global
/// choice)" vs "(override)"; empty when unset (the UI shows the
/// first-run chooser instead).
#[must_use]
pub(crate) fn redaction_line(
    provider: &str,
    global: Option<juggler_handover::redaction::RedactionPolicy>,
    r#override: Option<juggler_handover::redaction::RedactionPolicy>,
) -> String {
    let effective = juggler_handover::redaction::resolve_policy(global, r#override);
    let Some(policy) = effective else {
        return String::new();
    };
    let source = if r#override.is_some() {
        "override"
    } else {
        "your global choice"
    };
    format!(
        "Keys and passwords for {provider}: {} ({source})",
        strip_label(policy)
    )
}

/// The Strip label words the picker uses ("replace keys with •••");
/// Keep/Ask use their own plain words.
fn strip_label(policy: juggler_handover::redaction::RedactionPolicy) -> String {
    match policy {
        juggler_handover::redaction::RedactionPolicy::Strip => "hidden".to_owned(),
        juggler_handover::redaction::RedactionPolicy::Keep => "left in".to_owned(),
        juggler_handover::redaction::RedactionPolicy::AskEachTime => {
            "asked on every switch".to_owned()
        }
    }
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage reads fail.
pub fn redaction_policy_get_impl(
    state: &Arc<AppState>,
    provider: &str,
) -> AppResult<RedactionPolicyInfo> {
    let storage = lock_storage_arc(state)?;
    Ok(RedactionPolicyInfo::from_parts(
        &crate::features::tools::display_name(&state.registry, provider),
        redaction_global_of(&storage),
        redaction_override_of(&storage, provider),
    ))
}

/// A policy's wire name, or none to unset it; an unknown name is refused,
/// never guessed.
fn parsed_policy(
    policy: Option<&str>,
) -> AppResult<Option<juggler_handover::redaction::RedactionPolicy>> {
    let parsed = policy.and_then(juggler_handover::redaction::RedactionPolicy::from_wire);
    if policy.is_some() && parsed.is_none() {
        return Err(AppError::Io {
            context: "redaction policy",
            message: format!("Unknown policy {policy:?} — strip, keep, or ask"),
        });
    }
    Ok(parsed)
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] on an unknown wire name or a
/// storage write failure (an unknown name is refused, never
/// guessed).
pub fn redaction_policy_set_global_impl(
    state: &Arc<AppState>,
    policy: Option<&str>,
) -> AppResult<()> {
    let parsed = parsed_policy(policy)?;
    let storage = lock_storage_arc(state)?;
    let settings = storage.app_settings();
    match parsed {
        Some(parsed) => {
            settings.set(REDACTION_GLOBAL_KEY, parsed.as_str())?;
        }
        None => {
            settings.set(REDACTION_GLOBAL_KEY, "")?;
        }
    }
    Ok(())
}

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn redaction_policy_set_override_impl(
    state: &Arc<AppState>,
    provider: &str,
    policy: Option<&str>,
) -> AppResult<()> {
    let parsed = parsed_policy(policy)?;
    let storage = lock_storage_arc(state)?;
    let settings = storage.app_settings();
    let mut overrides: std::collections::BTreeMap<String, String> = settings
        .get(REDACTION_OVERRIDES_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default();
    match parsed {
        Some(parsed) => {
            overrides.insert(provider.to_owned(), parsed.as_str().to_owned());
        }
        None => {
            overrides.remove(provider);
        }
    }
    let blob = serde_json::to_string(&overrides).map_err(|error| AppError::Io {
        context: "redaction policy",
        message: error.to_string(),
    })?;
    settings.set(REDACTION_OVERRIDES_KEY, &blob)?;
    Ok(())
}

/// Lists the source conversation's secrets (masked) for the
/// preview; the un-redact indices the user picks address these.
pub(crate) fn redaction_items_of(
    source: &juggler_core::model::InstanceRecord,
) -> (Vec<RedactionItemInfo>, usize) {
    let Some(reference) = source.transcript_ref.as_ref() else {
        return (Vec::new(), 0);
    };
    let Some((_uuids, turns)) = transcript_side(&reference.path) else {
        return (Vec::new(), 0);
    };
    let segments: Vec<String> = turns.iter().map(|turn| turn.text.clone()).collect();
    let refs: Vec<&str> = segments.iter().map(String::as_str).collect();
    let items = juggler_handover::redaction::list_segments(&refs)
        .into_iter()
        .map(|listed| RedactionItemInfo {
            kind: listed.kind.to_owned(),
            preview: listed.preview,
            segment: u32::try_from(listed.segment).unwrap_or(u32::MAX),
        })
        .collect();
    (items, turns.len())
}
