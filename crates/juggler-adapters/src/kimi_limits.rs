//! AJ-E1603 R3 (D-106, keyless): a Kimi Code turn that stopped at the
//! plan's limit, from the session's own event log (`wire.jsonl`, protocol
//! 1.5, read by [`crate::kimi_sessions::wire`]).
//!
//! Kimi Code ends such a turn with `turn.ended` `reason: "failed"` and
//! its error payload (`{code, message, name, details, retryable}`): the
//! research pass of 2026-11-13 read it in the MIT `MoonshotAI/kimi-code`
//! source at `30e3ce44` (2.6.1): `agent/loop/journal.ts` writes the
//! payload of `_base/errors/serialize.ts`; Kimi's quota error
//! (`exceeded_current_quota_error`, HTTP 429, `provider/kimi/errors.ts`)
//! becomes `APIProviderQuotaExhaustedError` (`llm-adapter/contract/
//! errors.ts`), whose message is the vendor's own. Which window ran out is
//! only in that message, whose fixed English openings the vendor's error
//! reference lists (`kimi.com/code/docs/en/kimi-code/error-reference.html`):
//!
//! - "You've reached your 5-hour usage limit" — the 5-hour window;
//! - "You've reached your weekly (7-day) usage limit" — the week;
//! - "You've reached your monthly usage limit" — the month.
//!
//! A hit is the quota error by name (its window from the message, or
//! unknown), or a 429 `provider.api_error` / `provider.rate_limit` whose
//! message opens with one of those (an older build's 429). Anything else —
//! another 429 (too many requests at once), a localized or unknown message
//! on a plain 429 — is no hit, and a window is never guessed. The message
//! is matched as it is parsed and dropped: only the window is kept, never
//! its text. The vendor's reset time isn't in the record, so none is
//! claimed.

use juggler_core::quota::WindowKind;

/// The quota error's name in the payload.
const QUOTA_ERROR: &str = "APIProviderQuotaExhaustedError";
/// The payload codes a 429 limit message can come with.
const LIMIT_CODES: &[&str] = &["provider.api_error", "provider.rate_limit"];
/// HTTP 429, the status Kimi's quota errors carry.
const TOO_MANY: u64 = 429;

/// The vendor's messages, by the window each names.
const OPENINGS: &[(&str, WindowKind)] = &[
    (
        "You've reached your 5-hour usage limit",
        WindowKind::FiveHour,
    ),
    (
        "You've reached your weekly (7-day) usage limit",
        WindowKind::Weekly { anchor_ms: None },
    ),
    (
        "You've reached your monthly usage limit",
        WindowKind::Monthly,
    ),
];

/// The window a message names: matched and dropped as it is parsed, so
/// the text is never kept. `None`: it names none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Named(Option<WindowKind>);

/// The window a message opens with, when it is one of the vendor's openings.
#[must_use]
pub fn window_named(message: &str) -> Option<WindowKind> {
    OPENINGS
        .iter()
        .find(|(opening, _)| message.starts_with(opening))
        .map(|(_, window)| *window)
}

impl<'de> serde::Deserialize<'de> for Named {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visit;
        impl serde::de::Visitor<'_> for Visit {
            type Value = Named;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a message")
            }
            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Named, E> {
                Ok(Named(window_named(text)))
            }
        }
        deserializer.deserialize_str(Visit)
    }
}

/// The error payload's fields that are read (`details` only for its
/// status; everything else is skipped unkept).
#[derive(serde::Deserialize)]
struct Payload {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<Named>,
    #[serde(default)]
    details: Option<Details>,
}

#[derive(serde::Deserialize)]
struct Details {
    #[serde(rename = "statusCode", default)]
    status_code: Option<serde_json::Value>,
}

/// The line's error, and nothing else of it.
#[derive(serde::Deserialize)]
struct Ended {
    #[serde(default)]
    error: Option<Payload>,
}

/// Whether a failed turn's line (`turn.ended`, `reason: "failed"`) is a
/// limit hit: `Some` with the window it names (`None` inside: the quota
/// error, its window unknown); `None` when it is no hit.
#[must_use]
pub fn hit_in(line: &[u8]) -> Option<Option<WindowKind>> {
    let error = serde_json::from_slice::<Ended>(line).ok()?.error?;
    let window = error.message.and_then(|named| named.0);
    if error.name.as_deref() == Some(QUOTA_ERROR) {
        return Some(window);
    }
    let status = error
        .details
        .and_then(|details| details.status_code)
        .and_then(|status| status.as_u64());
    let limit_code = error
        .code
        .as_deref()
        .is_some_and(|code| LIMIT_CODES.contains(&code));
    (status == Some(TOO_MANY) && limit_code)
        .then_some(window)
        .flatten()
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ended(error: &str) -> Vec<u8> {
        format!(
            r#"{{"type":"turn.ended","agentId":"main","turnId":3,"reason":"failed","error":{error},"time":5000}}"#
        )
        .into_bytes()
    }

    #[test]
    fn the_quota_error_names_its_window_by_the_vendors_words() {
        for (message, window) in [
            (
                "You've reached your 5-hour usage limit. It resets soon.",
                WindowKind::FiveHour,
            ),
            (
                "You've reached your weekly (7-day) usage limit.",
                WindowKind::Weekly { anchor_ms: None },
            ),
            (
                "You've reached your monthly usage limit.",
                WindowKind::Monthly,
            ),
        ] {
            let line = ended(&format!(
                r#"{{"code":"provider.api_error","message":"{message}","name":"APIProviderQuotaExhaustedError","details":{{"statusCode":429}},"retryable":false}}"#
            ));
            assert_eq!(hit_in(&line), Some(Some(window)), "{message}");
        }
    }

    #[test]
    fn a_plain_429_counts_only_with_a_limit_message() {
        let five = ended(
            r#"{"code":"provider.rate_limit","message":"You've reached your 5-hour usage limit.","name":"APIProviderRateLimitError","details":{"statusCode":429},"retryable":true}"#,
        );
        assert_eq!(hit_in(&five), Some(Some(WindowKind::FiveHour)));
        // Too many requests at once is no hit.
        let busy = ended(
            r#"{"code":"provider.rate_limit","message":"Too many concurrent requests, please try again later.","name":"APIProviderRateLimitError","details":{"statusCode":429},"retryable":true}"#,
        );
        assert_eq!(hit_in(&busy), None);
        // A localized message on a plain 429: never guessed.
        let other = ended(
            r#"{"code":"provider.rate_limit","message":"您已达到 5 小时使用上限","name":"APIProviderRateLimitError","details":{"statusCode":429},"retryable":true}"#,
        );
        assert_eq!(hit_in(&other), None);
        // The right words with another status, or another code: no hit.
        let status = ended(
            r#"{"code":"provider.rate_limit","message":"You've reached your 5-hour usage limit.","details":{"statusCode":403},"retryable":false}"#,
        );
        assert_eq!(hit_in(&status), None);
        let code = ended(
            r#"{"code":"provider.auth_error","message":"You've reached your 5-hour usage limit.","details":{"statusCode":429},"retryable":false}"#,
        );
        assert_eq!(hit_in(&code), None);
    }

    #[test]
    fn the_quota_error_with_other_words_is_a_hit_on_no_named_window() {
        for message in [
            "Your credit balance is used up.",
            "您的额度已用完",
            "You have reached your usage limit",
        ] {
            let line = ended(&format!(
                r#"{{"code":"provider.api_error","message":"{message}","name":"APIProviderQuotaExhaustedError","details":{{"statusCode":429}},"retryable":false}}"#
            ));
            assert_eq!(hit_in(&line), Some(None), "{message}");
        }
    }

    #[test]
    fn a_line_of_another_shape_is_no_hit() {
        assert_eq!(hit_in(b"not json"), None);
        assert_eq!(hit_in(br#"{"type":"turn.ended","reason":"failed"}"#), None);
        assert_eq!(hit_in(&ended(r#""a string""#)), None);
        assert_eq!(
            hit_in(&ended(
                r#"{"code":"provider.api_error","message":42,"name":"APIProviderQuotaExhaustedError"}"#
            )),
            None
        );
    }
}
