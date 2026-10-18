//! AJ-E303 evidence: the Found/Likely/Guessed rule table as golden
//! fixtures — every rule has ≥ 1 positive AND ≥ 1 negative case
//! (first match wins per field), plus the ≤ 200-candidate budget
//! shape. Deterministic rules only; no I/O beyond names.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_adapters::inference::{
    ActivityTier, Confidence, InferInput, LoginInference, infer_activity, infer_candidate,
    infer_login, infer_purpose,
};
use std::path::PathBuf;

fn leak(path: &str) -> &'static std::path::Path {
    Box::leak(PathBuf::from(path).into_boxed_path())
}

fn base(path: &str) -> InferInput<'static> {
    InferInput {
        path: leak(path),
        manager_name: None,
        probe_logged_in: false,
        credentials_len: None,
        idle_secs: None,
        managed: false,
    }
}

/// Rule 1 (purpose): keyword Work (positive) vs neutral (negative).
#[test]
fn purpose_work_rule() {
    let positive = infer_purpose(&base("/Users/dev/.claude-work"));
    assert_eq!(
        (positive.value.as_str(), positive.confidence),
        ("Work", Confidence::Likely)
    );
    let negative = infer_purpose(&base("/Users/dev/.claude-staging"));
    assert_ne!(negative.value, "Work");
    assert_eq!(negative.confidence, Confidence::Guessed);
}

/// Rule 1 (purpose): keyword Personal (positive) vs Work-keyword
/// precedence when both appear (first match wins).
#[test]
fn purpose_personal_rule_and_first_match() {
    let positive = infer_purpose(&base("/Users/dev/.claude-personal"));
    assert_eq!(
        (positive.value.as_str(), positive.confidence),
        ("Personal", Confidence::Likely)
    );
    // "work" is checked first: a name with BOTH keywords resolves to
    // Work — deterministic order, never a coin flip.
    let both = infer_purpose(&base("/Users/dev/.claude-work-personal"));
    assert_eq!(both.value, "Work");
}

/// Rule 2 (purpose): manager names ride verbatim, Guessed (positive
/// = a manager name survives untouched; negative = without a
/// manager, the basename humanises instead).
#[test]
fn purpose_manager_name_rule() {
    let managed = InferInput {
        manager_name: Some("acct-2"),
        ..base("/x/ClaudeCodeProfiles/acct-2")
    };
    let positive = infer_purpose(&managed);
    assert_eq!(
        (positive.value.as_str(), positive.confidence),
        ("acct-2", Confidence::Guessed)
    );
    let negative = infer_purpose(&base("/x/ClaudeCodeProfiles/acct-2"));
    assert_ne!(negative.value, "acct-2");
}

/// Rule (login): keychain-bound signed-in (positive: caveat flag)
/// vs file-backed signed-in (negative: no caveat).
#[test]
fn login_keychain_bound_rule() {
    let (keychain, bound) = infer_login(&InferInput {
        probe_logged_in: true,
        credentials_len: None,
        ..base("/a/.claude")
    });
    assert_eq!(keychain.value, LoginInference::SignedIn);
    assert!(bound);
    let (file_backed, bound) = infer_login(&InferInput {
        probe_logged_in: true,
        credentials_len: Some(96),
        ..base("/a/.claude")
    });
    assert_eq!(file_backed.value, LoginInference::SignedIn);
    assert!(!bound);
}

/// Rule (login): empty credentials file → May be expired (positive)
/// vs absent file → Not signed in (negative — never "expired"
/// without a file).
#[test]
fn login_maybe_expired_rule() {
    let positive = infer_login(&InferInput {
        credentials_len: Some(0),
        ..base("/a/.claude")
    });
    assert_eq!(positive.0.value, LoginInference::MaybeExpired);
    assert_eq!(positive.0.confidence, Confidence::Likely);
    let negative = infer_login(&base("/a/.claude"));
    assert_eq!(negative.0.value, LoginInference::NotSignedIn);
}

/// Rule (activity): the 7-day Active bound (positive day 7 /
/// negative day 8) and the 90-day Dormant bound.
#[test]
fn activity_bounds_positive_and_negative() {
    assert_eq!(infer_activity(Some(7 * 86_400)).value, ActivityTier::Active);
    assert_eq!(
        infer_activity(Some(8 * 86_400)).value,
        ActivityTier::Dormant { days: 8 },
        "day 8 is Dormant, not Active"
    );
    assert_eq!(
        infer_activity(Some(90 * 86_400)).value,
        ActivityTier::Dormant { days: 90 }
    );
    assert_eq!(
        infer_activity(Some(91 * 86_400)).value,
        ActivityTier::Archive { days: 91 }
    );
}

/// Rule (already-imported): the managed marker rides through
/// verbatim (positive/negative).
#[test]
fn already_imported_rule() {
    let imported = infer_candidate(&InferInput {
        managed: true,
        ..base("/a/.claude")
    });
    assert!(imported.already_imported);
    let fresh = infer_candidate(&base("/a/.claude"));
    assert!(!fresh.already_imported);
}

/// The plan tier is NEVER inferred: the inference exposes no plan
/// field at all (compile-time guarantee — this test pins the struct's
/// field list so a future plan field must justify itself here).
#[test]
fn plan_tier_is_never_inferred() {
    let inference = infer_candidate(&InferInput {
        idle_secs: Some(0),
        ..base("/a/.claude-max")
    });
    let debug = format!("{inference:?}");
    assert!(
        !debug.to_lowercase().contains("plan"),
        "no plan field exists: {debug}"
    );
}

/// Budget shape: 200 candidates infer in well under 50 ms (rules are
/// string matches — no I/O per candidate beyond names).
#[test]
fn two_hundred_candidates_within_budget() {
    let paths: Vec<InferInput<'static>> = (0..200)
        .map(|index| InferInput {
            path: leak(&format!("/Users/dev/.claude-{index}-work")),
            probe_logged_in: index % 2 == 0,
            credentials_len: Some(64),
            idle_secs: Some(index as u64 * 86_400),
            managed: false,
            manager_name: None,
        })
        .collect();
    let start = std::time::Instant::now();
    let count = paths
        .iter()
        .filter(|input| infer_candidate(input).purpose.value == "Work")
        .count();
    let elapsed = start.elapsed();
    assert_eq!(count, 200);
    assert!(
        elapsed.as_millis() < 50,
        "200 candidates in {:?} (budget 50 ms)",
        elapsed
    );
}
