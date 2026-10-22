//! The files a tool keeps its sign-in in (ADR-0003): the one place their
//! names live. Outside this module they're seen only through what it hands
//! out, never a path to open: a file's metadata (presence, size), and the
//! presence-only shape of Codex's `auth.json` (which fields are filled, its
//! mode and account id; every token is skipped while parsing and never
//! stored). The name constants are the crate's own, for the copy rules
//! that keep these files back. `juggler-app/tests/no_bare_tools.rs` fails
//! when the names appear anywhere else, when a function outside the lists
//! uses the constants, or when this module reads any other way.

use std::path::Path;

use crate::presence::Present;

/// Claude Code's file-based sign-in (its OAuth tokens).
pub(crate) const CLAUDE_CREDENTIALS: &str = ".credentials.json";
/// Codex's sign-in (its tokens, or an API key).
pub(crate) const CODEX_AUTH: &str = "auth.json";

/// A provider's sign-in file in a config folder, when it's file-based:
/// presence and size only, never its contents.
#[must_use]
pub fn metadata(provider: &str, dir: &Path) -> Option<std::fs::Metadata> {
    let name = if provider == crate::codex::CODEX_ID {
        CODEX_AUTH
    } else {
        CLAUDE_CREDENTIALS
    };
    std::fs::metadata(dir.join(name)).ok()
}

/// What Codex's `auth.json` says, by shape.
pub(crate) enum CodexAuth {
    /// No file (the Keychain may hold the sign-in).
    Missing,
    /// An empty file: a sign-in abandoned mid-write.
    Empty,
    /// Which fields are filled.
    Shape(AuthShape),
    /// There, but not readable as JSON.
    Unreadable,
}

/// Codex's `auth.json`, streamed into [`AuthShape`]: every value (tokens,
/// keys) is skipped while parsing and never stored, logged or returned.
pub(crate) fn codex_auth(dir: &Path) -> CodexAuth {
    let path = dir.join(CODEX_AUTH);
    let Ok(meta) = std::fs::metadata(&path) else {
        return CodexAuth::Missing;
    };
    if meta.len() == 0 {
        return CodexAuth::Empty;
    }
    let Ok(file) = std::fs::File::open(&path) else {
        return CodexAuth::Unreadable;
    };
    serde_json::from_reader::<_, AuthShape>(std::io::BufReader::new(file))
        .map_or(CodexAuth::Unreadable, CodexAuth::Shape)
}

/// Codex's `tokens.account_id` (an id, not a credential), streamed so
/// every token value is skipped.
pub(crate) fn codex_account_id(dir: &Path) -> Option<String> {
    let file = std::fs::File::open(dir.join(CODEX_AUTH)).ok()?;
    serde_json::from_reader::<_, IdShape>(std::io::BufReader::new(file))
        .ok()
        .and_then(|shape| shape.tokens?.account_id)
        .filter(|id| !id.is_empty())
}

/// The shape of `auth.json` the probe needs — field names and presence;
/// `auth_mode` is a mode name, not a secret.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct AuthShape {
    pub(crate) auth_mode: Option<String>,
    #[serde(rename = "OPENAI_API_KEY")]
    pub(crate) api_key: Present,
    pub(crate) tokens: Option<TokenShape>,
    pub(crate) personal_access_token: Present,
    pub(crate) bedrock_api_key: Present,
    pub(crate) bedrock_access_keys: Present,
}

/// The `tokens` object's shape (presence only).
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct TokenShape {
    pub(crate) access_token: Present,
    pub(crate) refresh_token: Present,
}

/// The part of `auth.json` the identity probe keeps: the account id.
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct IdShape {
    tokens: Option<IdTokens>,
}

/// `tokens.account_id` only — the token values are skipped.
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct IdTokens {
    account_id: Option<String>,
}
