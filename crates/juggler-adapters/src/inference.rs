//! AJ-E303 — Found/Likely/Guessed inference (pass2/03 §4.1):
//! deterministic rules only (no model call, works offline),
//! evaluated in order, first match wins per field. Every inferred
//! field renders its confidence as WORDS — numeric scores are never
//! shown. Plan tier is deliberately NOT inferred (usage volume can
//! never manufacture quota precision — D-12/quota honesty).

use std::path::Path;

/// The confidence words (Req.3 — words, never scores).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// A direct probe observed this (login metadata, managed marker).
    Found,
    /// Name/pattern inference (`~/.claude-work` → Work).
    Likely,
    /// A weak signal — kept verbatim, always editable.
    Guessed,
}

impl Confidence {
    /// The wire name (`found`/`likely`/`guessed`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::Likely => "likely",
            Self::Guessed => "guessed",
        }
    }

    /// The display word (`Found`/`Likely`/`Guessed`).
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Found => "Found",
            Self::Likely => "Likely",
            Self::Guessed => "Guessed",
        }
    }
}

/// The activity tier (S8-driven, Req.2): Active ≤7 d, Dormant ≤90 d,
/// else an Archive candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityTier {
    /// Used within 7 days.
    Active,
    /// Used within 90 days.
    Dormant {
        /// Days since last use (for "Dormant 40 d").
        days: u32,
    },
    /// Older than 90 days.
    Archive {
        /// Days since last use.
        days: u32,
    },
}

impl ActivityTier {
    /// The wire name (`active`/`dormant`/`archive`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Dormant { .. } => "dormant",
            Self::Archive { .. } => "archive",
        }
    }

    /// The display words ("Active this week" / "Dormant 40 d" /
    /// "Archive candidate · 240 d").
    #[must_use]
    pub fn words(self) -> String {
        match self {
            Self::Active => "Active this week".to_owned(),
            Self::Dormant { days } => format!("Dormant {days} d"),
            Self::Archive { days } => format!("Archive candidate · {days} d"),
        }
    }
}

/// One inferred login state with its honesty tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginInference {
    /// Credentials present and non-empty (direct probe).
    SignedIn,
    /// The credentials file exists but is empty — may have expired.
    MaybeExpired,
    /// No credentials found (direct probe).
    NotSignedIn,
}

impl LoginInference {
    /// The wire name.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SignedIn => "signed-in",
            Self::MaybeExpired => "maybe-expired",
            Self::NotSignedIn => "not-signed-in",
        }
    }

    /// The display words.
    #[must_use]
    pub fn words(&self) -> &'static str {
        match self {
            Self::SignedIn => "Signed in",
            Self::MaybeExpired => "May be expired",
            Self::NotSignedIn => "Not signed in",
        }
    }
}

/// One field's inference: value + confidence word (Req.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inferred<T> {
    /// The inferred value.
    pub value: T,
    /// How sure the rule is (Found/Likely/Guessed).
    pub confidence: Confidence,
}

/// The full inference for one candidate (Req.2), deterministic from
/// the scan's own metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateInference {
    /// Account purpose ("Work" / "Personal" / humanised basename).
    pub purpose: Inferred<String>,
    /// Login state + how it was observed.
    pub login: Inferred<LoginInference>,
    /// Activity tier.
    pub activity: Inferred<ActivityTier>,
    /// True when the dir carries the AI Juggler managed marker —
    /// renders disabled "Already in AI Juggler" at Detect (Req.6).
    pub already_imported: bool,
    /// True when the login lives in the OS keychain (no credentials
    /// file): the re-login caveat copy applies after import.
    pub keychain_bound: bool,
}

/// The name-shape rule inputs kept pure (no I/O here): everything
/// comes from the scan's metadata so tests are golden-file stable.
pub struct InferInput<'a> {
    /// The candidate's config dir path (name rules read the basename).
    pub path: &'a Path,
    /// The dir's human name when it came from another manager
    /// (cc-switch provider name / t3 folder name) — None otherwise.
    pub manager_name: Option<&'a str>,
    /// The S7 login probe result (LoggedOut/LoggedIn from the scan).
    pub probe_logged_in: bool,
    /// `.credentials.json` file size, when the file exists.
    pub credentials_len: Option<u64>,
    /// Seconds since the newest transcript mtime (S8), when known.
    pub idle_secs: Option<u64>,
    /// The managed marker was present.
    pub managed: bool,
}

/// Humanises a config-dir basename: `claude-work` → "Claude work",
/// dots/dashes to spaces, first letter capitalised, bare `~/.claude`
/// → "Main".
#[must_use]
pub fn humanise_basename(name: &str) -> String {
    let trimmed = name
        .trim_start_matches('.')
        .trim_start_matches("claude")
        .trim_start_matches(['-', '_'])
        .replace(['-', '_'], " ");
    let trimmed = trimmed.trim();
    if trimmed.is_empty() {
        return "Main".to_owned();
    }
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Main".to_owned(),
    }
}

/// The purpose rule (§4.1 row 1–2, first match wins): manager name
/// verbatim (Guessed) → name keywords (Likely) → humanised basename
/// (Guessed).
#[must_use]
pub fn infer_purpose(input: &InferInput) -> Inferred<String> {
    if let Some(name) = input.manager_name {
        return Inferred {
            value: name.to_owned(),
            confidence: Confidence::Guessed,
        };
    }
    let lower = input
        .path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ["work", "corp", "acme"]
        .iter()
        .any(|key| lower.contains(key))
    {
        return Inferred {
            value: "Work".to_owned(),
            confidence: Confidence::Likely,
        };
    }
    if ["personal", "home", "side"]
        .iter()
        .any(|key| lower.contains(key))
    {
        return Inferred {
            value: "Personal".to_owned(),
            confidence: Confidence::Likely,
        };
    }
    Inferred {
        value: humanise_basename(&lower),
        confidence: Confidence::Guessed,
    }
}

/// The login rule (§4.1): probe non-empty credentials → Signed in
/// (Found); credentials file exists but EMPTY → May be expired
/// (Likely); signed in with NO credentials file → keychain-bound
/// (Signed in, Found, with the re-login caveat); else Not signed in
/// (Found).
#[must_use]
pub fn infer_login(input: &InferInput) -> (Inferred<LoginInference>, bool) {
    let keychain_bound = input.probe_logged_in && input.credentials_len.is_none();
    let inference = if input.probe_logged_in {
        LoginInference::SignedIn
    } else if input.credentials_len == Some(0) {
        LoginInference::MaybeExpired
    } else {
        LoginInference::NotSignedIn
    };
    // The probe observed the truth directly; "may be expired" is the
    // one inference (empty file) → Likely.
    let confidence = if matches!(inference, LoginInference::MaybeExpired) {
        Confidence::Likely
    } else {
        Confidence::Found
    };
    (
        Inferred {
            value: inference,
            confidence,
        },
        keychain_bound,
    )
}

/// The activity rule (§4.1): ≤7 d Active, ≤90 d Dormant, else
/// Archive — Found when the recency probe knew, Guessed otherwise
/// (no transcripts at all).
#[must_use]
pub fn infer_activity(idle_secs: Option<u64>) -> Inferred<ActivityTier> {
    match idle_secs {
        None => Inferred {
            value: ActivityTier::Archive { days: 0 },
            confidence: Confidence::Guessed,
        },
        Some(secs) => {
            let days = u32::try_from(secs / 86_400).unwrap_or_default();
            let tier = if days <= 7 {
                ActivityTier::Active
            } else if days <= 90 {
                ActivityTier::Dormant { days }
            } else {
                ActivityTier::Archive { days }
            };
            Inferred {
                value: tier,
                confidence: Confidence::Found,
            }
        }
    }
}

/// The full candidate inference (first-match rules, all fields).
#[must_use]
pub fn infer_candidate(input: &InferInput) -> CandidateInference {
    let (login, keychain_bound) = infer_login(input);
    CandidateInference {
        purpose: infer_purpose(input),
        login,
        activity: infer_activity(input.idle_secs),
        already_imported: input.managed,
        keychain_bound,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn input(path: &str) -> InferInput<'static> {
        InferInput {
            path: Box::leak(PathBuf::from(path).into_boxed_path()),
            manager_name: None,
            probe_logged_in: false,
            credentials_len: None,
            idle_secs: None,
            managed: false,
        }
    }

    #[test]
    fn purpose_rules_first_match_wins() {
        // Positive: name keyword → Likely Work/Personal.
        let work = infer_purpose(&input("/Users/dev/.claude-work"));
        assert_eq!(work.value, "Work");
        assert_eq!(work.confidence, Confidence::Likely);
        let personal = infer_purpose(&input("/Users/dev/.claude-personal"));
        assert_eq!(personal.value, "Personal");
        // Negative: a neutral name falls to the humanised basename,
        // Guessed — never a keyword guess.
        let neutral = infer_purpose(&input("/Users/dev/.claude-staging"));
        assert_eq!(neutral.value, "Staging", "the claude prefix is stripped");
        assert_eq!(neutral.confidence, Confidence::Guessed);
        // The vendor default humanises to Main.
        let main = infer_purpose(&input("/Users/dev/.claude"));
        assert_eq!(main.value, "Main");
        // Manager names ride verbatim, Guessed.
        let managed = InferInput {
            manager_name: Some("acct-2"),
            ..input("/x/y")
        };
        assert_eq!(infer_purpose(&managed).value, "acct-2");
        assert_eq!(infer_purpose(&managed).confidence, Confidence::Guessed);
    }

    #[test]
    fn login_rules_cover_all_four_states() {
        // Signed in from the file: Found, not keychain-bound.
        let (signed, key) = infer_login(&InferInput {
            probe_logged_in: true,
            credentials_len: Some(120),
            ..input("/a/.claude")
        });
        assert_eq!(signed.value, LoginInference::SignedIn);
        assert_eq!(signed.confidence, Confidence::Found);
        assert!(!key);
        // Signed in with NO file: keychain-bound (caveat copy applies).
        let (keychain, key) = infer_login(&InferInput {
            probe_logged_in: true,
            credentials_len: None,
            ..input("/a/.claude")
        });
        assert_eq!(keychain.value, LoginInference::SignedIn);
        assert!(key, "keychain caveat");
        // Empty credentials file: May be expired, Likely.
        let (stale, _) = infer_login(&InferInput {
            probe_logged_in: false,
            credentials_len: Some(0),
            ..input("/a/.claude")
        });
        assert_eq!(stale.value, LoginInference::MaybeExpired);
        assert_eq!(stale.confidence, Confidence::Likely);
        // Nothing: Not signed in, Found.
        let (out, _) = infer_login(&input("/a/.claude"));
        assert_eq!(out.value, LoginInference::NotSignedIn);
        assert_eq!(out.confidence, Confidence::Found);
    }

    #[test]
    fn activity_tiers_bound_at_7_and_90_days() {
        assert_eq!(infer_activity(Some(0)).value, ActivityTier::Active);
        assert_eq!(
            infer_activity(Some(7 * 86_400)).value,
            ActivityTier::Active,
            "day 7 is still Active"
        );
        assert_eq!(
            infer_activity(Some(40 * 86_400)).value,
            ActivityTier::Dormant { days: 40 }
        );
        assert_eq!(
            infer_activity(Some(240 * 86_400)).value,
            ActivityTier::Archive { days: 240 }
        );
        // No transcripts at all: Guessed, never a fabricated tier.
        let none = infer_activity(None);
        assert_eq!(none.confidence, Confidence::Guessed);
    }
}
