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
    },
    /// No sign-in is recorded in this state dir yet.
    SignedOut,
    /// This Provider exposes no identity metadata.
    Unverifiable {
        /// Plain words.
        reason: String,
    },
}

/// Masks an email for display: first character, `…`, then the domain
/// (`olivia@example.com` → `o…@example.com`). Recognisable, never the
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
        assert_eq!(mask_email("olivia@example.com"), "o…@example.com");
        assert_eq!(mask_email("@example.com"), "…");
        assert_eq!(mask_email("nobody"), "…");
    }
}
