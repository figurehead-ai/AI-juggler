//! AJ-E1001: per-Instance identity assertion — the wrong-account billing
//! guard.
//!
//! Before an Instance launches (and again on resume, switch and
//! switch-back), the Account's state dir is probed for who it is signed
//! in as — metadata only, through the adapter (`probe_identity`), never
//! credential content. The first identity an Account is seen with is
//! remembered (shown as "first seen" so a wrong first sign-in is
//! noticed); a later probe that names a different identity STOPS the
//! launch with expected vs detected. There is no "continue anyway": the
//! user switches to the right Account, checks again, or cancels.
//! Providers without identity metadata show `Identity unverified`.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;
use std::sync::Arc;

use juggler_adapter::IdentityProbe;
use juggler_core::model::{Account, SessionId};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The app setting holding each Account's remembered identity.
const PINS_KEY: &str = "identity_pins";

/// The identity an Account was first seen with.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
struct Pin {
    /// The vendor's stable account id (compared, never shown).
    subject: String,
    /// The masked label at the time.
    label: String,
    /// When, epoch ms.
    pinned_at_ms: i64,
}

fn pins(state: &Arc<AppState>) -> BTreeMap<String, Pin> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(PINS_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_pins(state: &Arc<AppState>, pins: &BTreeMap<String, Pin>) -> AppResult<()> {
    let raw = serde_json::to_string(pins).map_err(|e| AppError::Io {
        context: "identity",
        message: e.to_string(),
    })?;
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(PINS_KEY, &raw)?;
    Ok(())
}

/// Another Launch setting whose Account is signed in as the detected
/// identity (the "switch selection" action).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct IdentityAlternativeInfo {
    /// The Launch setting.
    pub recipe_id: String,
    /// Its name.
    pub recipe_name: String,
    /// Its Account's label.
    pub account_label: String,
}

/// What an identity check found, before anything launches.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct IdentityCheckInfo {
    /// `verified` | `first_seen` (nothing remembered yet — this becomes
    /// the expected identity) | `mismatch` | `signed_out` | `unverified`.
    pub state: String,
    /// Provider id.
    pub provider: String,
    /// The selected Account.
    pub account_id: String,
    /// Its label.
    pub account_label: String,
    /// The model the launch will use, when the Launch settings set one.
    pub model: Option<String>,
    /// The identity this Account is expected to be (masked).
    pub expected: Option<String>,
    /// The identity detected now (masked).
    pub detected: Option<String>,
    /// Why it couldn't be verified, when it couldn't.
    pub reason: Option<String>,
    /// Launch settings whose Account IS the detected identity.
    pub alternatives: Vec<IdentityAlternativeInfo>,
}

/// One assertion's verdict, as launches use it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Assertion {
    pub state: String,
    pub label: Option<String>,
}

fn probe(state: &Arc<AppState>, account: &Account) -> IdentityProbe {
    state.registry.get(&account.harness).map_or_else(
        || IdentityProbe::Unverifiable {
            reason: "this Provider isn't installed here".into(),
        },
        |adapter| adapter.probe_identity(account),
    )
}

/// Compares a probe with the remembered pin (pure).
fn verdict(
    probe: &IdentityProbe,
    pin: Option<&Pin>,
) -> (&'static str, Option<String>, Option<String>) {
    match probe {
        IdentityProbe::SignedIn { subject, label } => match pin {
            Some(pin) if &pin.subject == subject => ("verified", Some(label.clone()), None),
            Some(_) => ("mismatch", Some(label.clone()), None),
            None => ("first_seen", Some(label.clone()), None),
        },
        IdentityProbe::SignedOut => ("signed_out", None, None),
        IdentityProbe::Unverifiable { reason } => ("unverified", None, Some(reason.clone())),
    }
}

/// Core: the identity check for a Launch setting — read-only (nothing is
/// remembered or launched).
///
/// # Errors
///
/// Returns [`AppError::NotFound`] for an unknown Launch setting.
pub fn identity_check_impl(state: &Arc<AppState>, recipe_id: &str) -> AppResult<IdentityCheckInfo> {
    let (profile, account) = crate::commands::find_profile_with_account_arc(state, recipe_id)?;
    let found = probe(state, &account);
    let pins = pins(state);
    let pin = pins.get(account.id.as_str());
    let (verdict, detected, reason) = verdict(&found, pin);
    let alternatives = match (&found, verdict) {
        (IdentityProbe::SignedIn { subject, .. }, "mismatch") => {
            alternatives_for(state, &pins, subject)
        }
        _ => Vec::new(),
    };
    Ok(IdentityCheckInfo {
        state: verdict.to_owned(),
        provider: account.harness.to_string(),
        account_id: account.id.to_string(),
        account_label: account.label.clone(),
        model: profile.settings_overlay.model.clone(),
        expected: pin.map(|pin| pin.label.clone()),
        detected,
        reason,
        alternatives,
    })
}

/// Launch settings whose Account is remembered as `subject`.
fn alternatives_for(
    state: &Arc<AppState>,
    pins: &BTreeMap<String, Pin>,
    subject: &str,
) -> Vec<IdentityAlternativeInfo> {
    let Ok(storage) = state.storage.lock() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for account in storage.accounts().list().unwrap_or_default() {
        if pins
            .get(account.id.as_str())
            .is_none_or(|pin| pin.subject != subject)
        {
            continue;
        }
        for recipe in storage
            .profiles()
            .list_by_account(&account.id)
            .unwrap_or_default()
        {
            out.push(IdentityAlternativeInfo {
                recipe_id: recipe.id.to_string(),
                recipe_name: recipe.name.clone(),
                account_label: account.label.clone(),
            });
        }
    }
    out
}

/// Asserts `account`'s identity before a launch, resume or switch: a
/// mismatch is refused (typed, with expected vs detected); a first
/// sighting is remembered.
///
/// # Errors
///
/// Returns [`AppError::IdentityMismatch`] when the state dir is signed in
/// as someone other than the Account's remembered identity.
pub(crate) fn assert_identity(state: &Arc<AppState>, account: &Account) -> AppResult<Assertion> {
    let found = probe(state, account);
    let mut pins = pins(state);
    let pin = pins.get(account.id.as_str()).cloned();
    let (verdict, detected, _) = verdict(&found, pin.as_ref());
    match (verdict, &found) {
        ("mismatch", _) => Err(AppError::IdentityMismatch {
            account: account.label.clone(),
            expected: pin.map(|pin| pin.label).unwrap_or_default(),
            detected: detected.unwrap_or_default(),
        }),
        ("first_seen", IdentityProbe::SignedIn { subject, label }) => {
            pins.insert(
                account.id.to_string(),
                Pin {
                    subject: subject.clone(),
                    label: label.clone(),
                    pinned_at_ms: juggler_core::model::Timestamp::now().millis(),
                },
            );
            save_pins(state, &pins)?;
            Ok(Assertion {
                state: "first_seen".to_owned(),
                label: Some(label.clone()),
            })
        }
        _ => Ok(Assertion {
            state: verdict.to_owned(),
            label: detected,
        }),
    }
}

/// Records an assertion in the Instance's history (who ran where, when).
pub(crate) fn record(
    state: &Arc<AppState>,
    session: &SessionId,
    account: &Account,
    assertion: &Assertion,
    moment: &str,
) {
    if let Ok(storage) = state.storage.lock() {
        let _ = storage
            .identity()
            .record(&juggler_core::storage::IdentityAssertion {
                session_id: session.to_string(),
                account_id: account.id.to_string(),
                state: assertion.state.clone(),
                label: assertion.label.clone(),
                at_ms: juggler_core::model::Timestamp::now().millis(),
                moment: moment.to_owned(),
            });
    }
}

/// Core: forgets the identity remembered for an Account (it is learned
/// again at its next launch, shown as "first seen").
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn account_forget_identity_impl(state: &Arc<AppState>, account_id: &str) -> AppResult<()> {
    let mut pins = pins(state);
    if pins.remove(account_id).is_some() {
        save_pins(state, &pins)?;
    }
    Ok(())
}

/// AJ-E1001: the identity check for a Launch setting, before launching.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`identity_check_impl`].
#[tauri::command]
pub fn identity_check(
    state: tauri::State<'_, Arc<AppState>>,
    recipe_id: String,
) -> AppResult<IdentityCheckInfo> {
    identity_check_impl(state.inner(), &recipe_id)
}

/// AJ-E1001: forgets an Account's remembered identity.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`account_forget_identity_impl`].
#[tauri::command]
pub fn account_forget_identity(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<()> {
    account_forget_identity_impl(state.inner(), &account_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed_in(subject: &str) -> IdentityProbe {
        IdentityProbe::SignedIn {
            subject: subject.into(),
            label: format!("{subject}-label"),
        }
    }

    #[test]
    fn the_verdict_never_passes_a_different_identity() {
        let pin = Pin {
            subject: "acct-1".into(),
            label: "o…@work.com".into(),
            pinned_at_ms: 1,
        };
        assert_eq!(verdict(&signed_in("acct-1"), Some(&pin)).0, "verified");
        assert_eq!(verdict(&signed_in("acct-2"), Some(&pin)).0, "mismatch");
        assert_eq!(verdict(&signed_in("acct-2"), None).0, "first_seen");
        assert_eq!(
            verdict(&IdentityProbe::SignedOut, Some(&pin)).0,
            "signed_out"
        );
        assert_eq!(
            verdict(
                &IdentityProbe::Unverifiable {
                    reason: "n/a".into()
                },
                Some(&pin)
            )
            .0,
            "unverified"
        );
    }
}
