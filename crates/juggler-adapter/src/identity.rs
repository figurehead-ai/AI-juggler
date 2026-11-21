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

/// What a tool with no account metadata says where its account would be
/// ([`IdentityProbe::Unverifiable`], and its key sign-ins'), by the tool's
/// own name (SK48-4: a session page said "Account unknown · this Provider
/// exposes no account metadata", the tool unnamed and an internal word).
#[must_use]
pub fn no_account_metadata(tool: &str) -> String {
    format!("{tool} doesn't say which account it's signed in to")
}

/// [`IdentityProbe::SignedInWithKey`] for a tool whose sign-in state says
/// it runs on an API key, else what a tool with no account metadata says
/// ([`no_account_metadata`], `tool` its name as people see it).
#[must_use]
pub fn key_or_no_metadata(on_key: bool, tool: &str) -> IdentityProbe {
    let reason = no_account_metadata(tool);
    if on_key {
        IdentityProbe::SignedInWithKey { reason }
    } else {
        IdentityProbe::Unverifiable { reason }
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

/// Masks every email inside a text as [`mask_email`] does, leaving the
/// rest as it is: an organisation a tool names after its owner's email
/// ("sam@example.com's Organization") never shows the address in full
/// beside its masked form (SK-16). A masked email stays as it is, so a
/// label can be masked again safely.
#[must_use]
pub fn mask_emails_in(text: &str) -> String {
    let local = |c: char| c.is_alphanumeric() || "._%+-".contains(c);
    let domain = |c: char| c.is_alphanumeric() || ".-".contains(c);
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if !local(chars[i]) {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && local(chars[i]) {
            i += 1;
        }
        let at = i;
        let mut end = at + 1;
        while end < chars.len() && domain(chars[end]) {
            end += 1;
        }
        // A domain is a dotted name: `example.com`, not `x` or `.com`.
        let host: String = chars.get(at + 1..end).unwrap_or_default().iter().collect();
        let host = host.trim_end_matches('.');
        if chars.get(at) == Some(&'@') && host.contains('.') && !host.starts_with('.') {
            out.push(chars[start]);
            out.push('…');
            out.push('@');
            out.push_str(host);
            i = at + 1 + host.chars().count();
        } else {
            out.extend(&chars[start..at]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SK48-4: a tool with no account metadata is named, in plain words
    /// ("this Provider exposes no account metadata" before).
    #[test]
    fn no_account_metadata_names_the_tool_in_plain_words() {
        let probe = key_or_no_metadata(false, "Kimi Code");
        assert_eq!(
            probe,
            IdentityProbe::Unverifiable {
                reason: "Kimi Code doesn't say which account it's signed in to".into()
            }
        );
        assert_eq!(
            key_or_no_metadata(true, "Grok Build"),
            IdentityProbe::SignedInWithKey {
                reason: "Grok Build doesn't say which account it's signed in to".into()
            }
        );
    }

    #[test]
    fn emails_are_masked_but_recognisable() {
        assert_eq!(mask_email("sam@example.com"), "s…@example.com");
        assert_eq!(mask_email("@example.com"), "…");
        assert_eq!(mask_email("nobody"), "…");
    }

    /// SK-16: a personal Claude organisation is named after its email.
    #[test]
    fn every_email_inside_a_text_is_masked_and_nothing_else() {
        assert_eq!(
            mask_emails_in("sam.lee+ai@example.com's Organization"),
            "s…@example.com's Organization"
        );
        assert_eq!(
            mask_emails_in("s…@example.com · sam@example.com's Organization"),
            "s…@example.com · s…@example.com's Organization"
        );
        assert_eq!(
            mask_emails_in("a@example.com and c@example.com."),
            "a…@example.com and c…@example.com."
        );
        assert_eq!(mask_emails_in("Acme · Team"), "Acme · Team");
        assert_eq!(
            mask_emails_in("ops@localhost, @handle, x@.com"),
            "ops@localhost, @handle, x@.com"
        );
        assert_eq!(mask_emails_in("Zoë@bücher.de"), "Z…@bücher.de");
        let once = mask_emails_in("sam@example.com");
        assert_eq!(mask_emails_in(&once), once, "masking again changes nothing");
    }
}
