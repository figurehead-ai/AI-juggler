//! AJ-E309: the checks on a Launch settings' environment overrides.

use crate::error::{AppError, AppResult};

/// `_`-delimited key segments that mark an environment key as
/// credential-shaped (`ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`)
/// — whole segments, so `CLAUDE_CODE_MAX_OUTPUT_TOKENS` stays allowed.
const CREDENTIAL_KEY_SEGMENTS: &[&str] = &[
    "KEY",
    "APIKEY",
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "CREDENTIALS",
    "AUTH",
    "OAUTH",
    "BEARER",
    "COOKIE",
];

/// The most overrides one Launch settings may carry.
const ENV_OVERLAY_MAX: usize = 32;

/// AJ-E309: validates a Launch-settings environment overlay. Keys are
/// shell-style (`[A-Z_][A-Z0-9_]*`, ≤ 64); values are one line
/// (≤ 1024 bytes). Isolation keys are refused (they would repoint the
/// Account's folder), and so are credential-shaped keys — vendor
/// credentials are never stored by AI Juggler; the vendor's own
/// sign-in owns them.
///
/// # Errors
///
/// Returns [`AppError::Validation`] naming the first offending key.
pub fn validate_env_overlay(
    protected: &[&str],
    env: &std::collections::BTreeMap<String, String>,
) -> AppResult<()> {
    let refuse = |message: String| AppError::Validation {
        subject: "environment override",
        message,
    };
    if env.len() > ENV_OVERLAY_MAX {
        return Err(refuse(format!(
            "at most {ENV_OVERLAY_MAX} overrides per Launch settings"
        )));
    }
    for (key, value) in env {
        let mut bytes = key.bytes();
        let shaped = matches!(bytes.next(), Some(b'A'..=b'Z' | b'_'))
            && key.len() <= 64
            && bytes.all(|b| matches!(b, b'A'..=b'Z' | b'0'..=b'9' | b'_'));
        if !shaped {
            return Err(refuse(format!(
                "“{key}” isn't a valid variable name — use capitals, digits and _"
            )));
        }
        if protected.contains(&key.as_str()) {
            return Err(refuse(format!(
                "{key} is managed by AI Juggler — it keeps this account isolated"
            )));
        }
        if key
            .split('_')
            .any(|segment| CREDENTIAL_KEY_SEGMENTS.contains(&segment))
        {
            return Err(refuse(format!(
                "{key} looks like a credential — sign in with the vendor's own flow instead; \
                 AI Juggler never stores credentials"
            )));
        }
        if value.len() > 1024 || value.contains(['\0', '\n', '\r']) {
            return Err(refuse(format!(
                "the value for {key} must be a single line under 1 KB"
            )));
        }
    }
    Ok(())
}
