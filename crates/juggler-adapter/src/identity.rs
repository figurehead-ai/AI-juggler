//! AJ-E1001: who an Account's state directory is signed in as.
//!
//! Metadata only: a stable account id and a display label — never a
//! token, a password or any credential file's content. Each adapter
//! documents what it reads; a Provider with no identity metadata says so
//! (`Unverifiable`), and the Instance then shows `Identity unverified`
//! rather than pretending.

/// The outcome of an identity probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityProbe {
    /// The state dir is signed in.
    SignedIn {
        /// A stable vendor account id (compared, never shown).
        subject: String,
        /// A recognisable, masked label (`o…@example.com · Acme`).
        label: String,
        /// D-35: the organisation this sign-in is a seat in, when the
        /// tool records one (compared, never shown). One login can be a
        /// personal plan and a team seat: the same `subject`, two
        /// organisations, two accounts. The wrong-account guard compares
        /// `subject` alone.
        org: Option<String>,
    },
    /// No sign-in is recorded in this state dir yet.
    SignedOut,
    /// 0.4.1: signed in with an API key: a definite answer, but one that
    /// names no account (a key's owner is never read). It is no
    /// subscription's sign-in, so a folder that had one leaves it.
    SignedInWithKey {
        /// Plain words, said where its account would be.
        reason: String,
    },
    /// This Provider exposes no identity metadata, or this read couldn't
    /// tell (a file mid-write, a timeout, a tool not installed). Never a
    /// sign-out: a folder keeps the Account it was last read as.
    Unverifiable {
        /// Plain words.
        reason: String,
    },
}

/// What a Provider with no account metadata says where its account would
/// be ([`IdentityProbe::Unverifiable`], and its key sign-ins').
pub const NO_ACCOUNT_METADATA: &str = "this Provider exposes no account metadata";

/// [`IdentityProbe::SignedInWithKey`] for a tool whose sign-in state says
/// it runs on an API key, else what a Provider with no account metadata
/// says.
#[must_use]
pub fn key_or_no_metadata(on_key: bool) -> IdentityProbe {
    if on_key {
        IdentityProbe::SignedInWithKey {
            reason: NO_ACCOUNT_METADATA.into(),
        }
    } else {
        IdentityProbe::Unverifiable {
            reason: NO_ACCOUNT_METADATA.into(),
        }
    }
}

/// Masks an email for display: first character, `…`, then the domain
/// (`sam@example.com` → `s…@example.com`). Recognisable, never the
/// full address.
#[must_use]
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) if !local.is_empty() => {
            let first: String = local.chars().take(1).collect();
            format!("{first}…@{domain}")
        }
        _ => "…".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_are_masked_but_recognisable() {
        assert_eq!(mask_email("sam@example.com"), "s…@example.com");
        assert_eq!(mask_email("@example.com"), "…");
        assert_eq!(mask_email("nobody"), "…");
    }
}
