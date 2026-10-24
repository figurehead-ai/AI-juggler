//! The files a tool keeps its sign-in in (ADR-0003): the one place their
//! names live. Outside this module they're seen only through what it hands
//! out, never a path to open: a file's metadata (presence, size), the
//! presence-only shape of Codex's `auth.json` (which fields are filled, its
//! mode and account id; every token is skipped while parsing and never
//! stored), the kinds of OpenCode's sign-ins (`oauth`, `api`), Grok
//! Build's modes (`web_login`, `api_key`), and which of Kimi Code's
//! sign-in files are there. The name constants are the crate's own, for the copy rules
//! that keep these files back. `juggler-app/tests/no_bare_tools.rs` fails
//! when the names appear anywhere else, when a function outside the lists
//! uses the constants, or when this module reads any other way.

use std::path::Path;

use crate::presence::Present;

/// Claude Code's file-based sign-in (its OAuth tokens).
pub(crate) const CLAUDE_CREDENTIALS: &str = ".credentials.json";
/// Codex's sign-in (its tokens, or an API key).
pub(crate) const CODEX_AUTH: &str = "auth.json";
/// OpenCode's sign-ins (AJ-E1602): one entry per provider, in its data
/// folder (v1; v2 imports it into its database).
pub(crate) const OPENCODE_AUTH: &str = "auth.json";
/// OpenCode's MCP servers' sign-ins.
pub(crate) const OPENCODE_MCP_AUTH: &str = "mcp-auth.json";
/// Grok Build's sign-ins (AJ-E1604): one entry per scope, each with its
/// mode, in its home.
pub(crate) const GROK_AUTH: &str = "auth.json";
/// Grok Build's MCP servers' sign-ins.
pub(crate) const GROK_MCP_AUTH: &str = "mcp_credentials.json";
/// Kimi Code's sign-ins folder (AJ-E1603): one file per sign-in, owner
/// only, and its MCP servers' sign-ins under `mcp/`.
pub(crate) const KIMI_CREDENTIALS: &str = "credentials";
/// Kimi Code's membership (OAuth) sign-in, in that folder.
pub(crate) const KIMI_MEMBERSHIP: &str = "kimi-code.json";

/// A provider's sign-in file in a config folder, when it's file-based:
/// presence and size only, never its contents.
#[must_use]
pub fn metadata(provider: &str, dir: &Path) -> Option<std::fs::Metadata> {
    let name = match provider {
        crate::codex::CODEX_ID => CODEX_AUTH,
        crate::opencode::OPENCODE_ID => OPENCODE_AUTH,
        crate::grok_build::GROK_BUILD_ID => GROK_AUTH,
        crate::kimi_code::KIMI_CODE_ID => {
            return std::fs::metadata(dir.join(KIMI_CREDENTIALS).join(KIMI_MEMBERSHIP)).ok();
        }
        _ => CLAUDE_CREDENTIALS,
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

/// What OpenCode's `auth.json` says, by shape.
pub(crate) enum OpenCodeAuth {
    /// No file.
    Missing,
    /// An empty file: a sign-in abandoned mid-write.
    Empty,
    /// Each entry's kind (`oauth`, `api`, `wellknown`: a kind, not a
    /// secret), in the file's order.
    Kinds(Vec<String>),
    /// There, but not readable as JSON.
    Unreadable,
}

/// OpenCode's `auth.json` in the data folder `dir`, streamed into each
/// entry's `type`: every other field (tokens, keys) is skipped while
/// parsing and never stored, logged or returned.
pub(crate) fn opencode_auth(dir: &Path) -> OpenCodeAuth {
    /// One entry: its kind only.
    #[derive(serde::Deserialize)]
    struct Entry {
        #[serde(default, rename = "type")]
        kind: String,
    }
    let path = dir.join(OPENCODE_AUTH);
    let Ok(meta) = std::fs::metadata(&path) else {
        return OpenCodeAuth::Missing;
    };
    if meta.len() == 0 {
        return OpenCodeAuth::Empty;
    }
    let Ok(file) = std::fs::File::open(&path) else {
        return OpenCodeAuth::Unreadable;
    };
    serde_json::from_reader::<_, std::collections::BTreeMap<String, Entry>>(
        std::io::BufReader::new(file),
    )
    .map_or(OpenCodeAuth::Unreadable, |entries| {
        OpenCodeAuth::Kinds(entries.into_values().map(|entry| entry.kind).collect())
    })
}

/// What Grok Build's `auth.json` says, by shape.
pub(crate) enum GrokAuth {
    /// No file.
    Missing,
    /// An empty file: a sign-in abandoned mid-write.
    Empty,
    /// Each entry's mode (`web_login`, `oidc`, `external`, `api_key`: a
    /// kind, not a secret).
    Modes(Vec<String>),
    /// There, but not readable as JSON.
    Unreadable,
}

/// Grok Build's `auth.json` in the home `dir`, streamed into each entry's
/// `auth_mode`: every other field (keys, tokens, email) is skipped while
/// parsing and never stored, logged or returned.
pub(crate) fn grok_auth(dir: &Path) -> GrokAuth {
    /// One entry: its mode only.
    #[derive(serde::Deserialize)]
    struct Entry {
        #[serde(default)]
        auth_mode: String,
    }
    let path = dir.join(GROK_AUTH);
    let Ok(meta) = std::fs::metadata(&path) else {
        return GrokAuth::Missing;
    };
    if meta.len() == 0 {
        return GrokAuth::Empty;
    }
    let Ok(file) = std::fs::File::open(&path) else {
        return GrokAuth::Unreadable;
    };
    serde_json::from_reader::<_, std::collections::BTreeMap<String, Entry>>(
        std::io::BufReader::new(file),
    )
    .map_or(GrokAuth::Unreadable, |entries| {
        GrokAuth::Modes(entries.into_values().map(|entry| entry.auth_mode).collect())
    })
}

/// Which sign-ins a Kimi Code home holds, by the files' presence only:
/// none is ever opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KimiSignIn {
    /// The membership's own sign-in (`kimi-code.json`): a subscription.
    Membership,
    /// Another sign-in file, and no membership one: a Kimi Platform API
    /// key [Inferred from the layout; on the live checklist].
    Other,
    /// No sign-in file.
    None,
}

/// See [`KimiSignIn`]. `None` when the home can't be listed.
pub(crate) fn kimi_sign_in(home: &Path) -> Option<KimiSignIn> {
    std::fs::read_dir(home).ok()?;
    let folder = home.join(KIMI_CREDENTIALS);
    let filled =
        |path: &Path| std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() > 0);
    if filled(&folder.join(KIMI_MEMBERSHIP)) {
        return Some(KimiSignIn::Membership);
    }
    let other = std::fs::read_dir(&folder).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let path = entry.path();
            path.extension().is_some_and(|ext| ext == "json") && filled(&path)
        })
    });
    Some(if other {
        KimiSignIn::Other
    } else {
        KimiSignIn::None
    })
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
