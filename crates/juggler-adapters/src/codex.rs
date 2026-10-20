//! Codex adapter (spec §4.2) — `OpenAI`'s Codex CLI as the second Provider.
//!
//! Isolation model (research `docs/research/codex.md` §3 and the §10
//! addendum, verified against codex-cli 0.163.0 source and binary for
//! AJ-E1301): every managed account gets its own `CODEX_HOME`. Codex keeps
//! `config.toml`, `auth.json`, `sessions/`, its SQLite state and logs under
//! that directory and refuses to start when it does not exist, so
//! [`HarnessAdapter::prepare_account`] creates it. `CODEX_SQLITE_HOME`
//! would move the SQLite state elsewhere — shared between accounts — so
//! every launch pins it to the account's own directory too.
//!
//! Every terminal launch passes `--no-daemon`: since 0.157 the TUI starts
//! (or attaches to) a background app-server per `CODEX_HOME` that
//! outlives the TUI — a process Juggler did not spawn and could not stop
//! or clean up with the Instance (ADR-0010). The in-process server keeps
//! the whole session inside the tree Juggler owns.
//!
//! Guardrails (ADR-0003): sign-in detection reads the NAMES and shape of
//! `auth.json` fields (`auth_mode`, is there a token) — never a value;
//! with Keychain storage it checks the entry's existence only. Login
//! always runs through the vendor's own `codex login` in a guided
//! terminal.

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{DiscoveredInstall, ImportOutcome, LoginProbe};
use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, HarnessAdapter, InstallStatus, LaunchMode,
    LaunchSpec, SessionWatcher, VerificationStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

use crate::claude_code::{
    LoginState, existing, find_on_path, home_dir, home_relative, prepare_managed_dir_for,
};

/// Provider id: `codex`.
pub const CODEX_ID: &str = "codex";

/// Versions this adapter's launch/login behaviour was verified against
/// (codex-cli 0.163.0, 2026-10-20 — AJ-E1301 evidence). Other versions
/// run with a drift note, never a hard failure (spec §4 rules).
const VERIFIED_AGAINST: &[&str] = &["0.163.0"];

/// Environment variable naming the isolated state dir.
pub const ENV_CODEX_HOME: &str = "CODEX_HOME";

/// Would relocate the SQLite state out of `CODEX_HOME`; pinned per account.
const ENV_SQLITE_HOME: &str = "CODEX_SQLITE_HOME";

/// Optional test/dev override for the Codex binary path.
const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_CODEX_BIN";

/// Variables that would sign a launch in as someone else (an API key
/// bills that key, whatever the account signed in as).
const INHERITED_KEYS: &[&str] = &["CODEX_API_KEY", "OPENAI_API_KEY"];

/// Sets every [`INHERITED_KEYS`] entry to the env-unset marker.
fn unset_inherited_keys(env: &mut EnvMap) {
    for key in INHERITED_KEYS {
        env.insert(
            (*key).to_owned(),
            juggler_adapter::launch::ENV_UNSET.to_owned(),
        );
    }
}

/// Keeps the TUI off the background app-server (see module docs).
const NO_DAEMON: &str = "--no-daemon";

/// Adapter for the `OpenAI` Codex CLI.
#[derive(Debug, Clone, Default)]
pub struct CodexAdapter {
    bin: Option<PathBuf>,
    /// The user's home for the shell-file scan (tests/fixtures; `None`
    /// reads `$HOME`).
    home: Option<PathBuf>,
}

impl CodexAdapter {
    /// Creates the adapter, resolving the binary at call time.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates the adapter pinned to a specific binary (tests/fixtures).
    #[must_use]
    pub fn with_bin(bin: impl Into<PathBuf>) -> Self {
        Self {
            bin: Some(bin.into()),
            home: None,
        }
    }

    /// The adapter scanning `home`'s shell files instead of `$HOME`'s
    /// (tests/fixtures).
    #[must_use]
    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }

    fn resolve_bin(&self) -> Option<PathBuf> {
        if let Some(bin) = &self.bin {
            return Some(bin.clone());
        }
        if let Some(override_path) = std::env::var_os(ENV_BIN_OVERRIDE) {
            let path = PathBuf::from(override_path);
            if path.is_file() {
                return Some(path);
            }
        }
        find_on_path("codex").or_else(|| {
            // GUI launches don't inherit the shell PATH: cover Homebrew
            // (cask and npm global), the standalone installer and a
            // user-level npm prefix.
            existing(&["/opt/homebrew/bin/codex", "/usr/local/bin/codex"])
                .or_else(|| home_relative(".local/bin/codex").filter(|p| p.is_file()))
                .or_else(|| home_relative(".npm-global/bin/codex").filter(|p| p.is_file()))
        })
    }

    fn not_found(&self, note: String) -> DiscoveryReport {
        DiscoveryReport {
            adapter: self.id(),
            install: InstallStatus::NotFound,
            verification: VerificationStatus::Unverified,
            binary: None,
            notes: vec![note],
        }
    }

    fn failed(&self, bin: PathBuf, message: String) -> DiscoveryReport {
        DiscoveryReport {
            adapter: self.id(),
            install: InstallStatus::Failed { message },
            verification: VerificationStatus::Unverified,
            binary: Some(bin),
            notes: Vec::new(),
        }
    }

    /// Probes whether the account in `state_dir` is signed in.
    ///
    /// Reads `auth.json` only for the presence and shape of its fields
    /// (`auth_mode`, whether a token or key is set) — values are never
    /// logged, compared or returned (ADR-0003). `codex login status`
    /// answers from the same file (verified on 0.163.0 with synthetic
    /// files, AJ-E1301); `file` is Codex's default store on macOS too.
    #[must_use]
    pub fn login_state(&self, state_dir: &Path) -> LoginState {
        codex_login_state(state_dir)
    }

    /// The guided-terminal sign-in: the vendor's own `codex login`
    /// (browser sign-in with `ChatGPT`; the user may pick an API key or
    /// device code instead inside the vendor flow).
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] when no codex binary resolves.
    pub fn login_spec_inherent(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        let bin = self.require_bin()?;
        let mut env = home_env(state_dir);
        // The vendor sign-in signs THIS account in, never a key's owner.
        unset_inherited_keys(&mut env);
        Ok(LaunchSpec {
            program: bin,
            args: vec!["login".into()],
            env,
            cwd: state_dir.to_path_buf(),
            needs_pty: true,
        })
    }

    fn require_bin(&self) -> Result<PathBuf, AdapterError> {
        self.resolve_bin().ok_or_else(|| {
            AdapterError::InvalidLaunch("codex binary not found; run discovery for details".into())
        })
    }
}

/// A native session id is a UUID or a name; one starting with "-" would
/// be read as an option (`--last` resumes something else entirely).
///
/// # Errors
///
/// [`AdapterError::InvalidLaunch`] for an id that starts with "-".
pub fn refuse_option_like(session_id: &str) -> Result<(), AdapterError> {
    if session_id.is_empty() || session_id.starts_with('-') {
        return Err(AdapterError::InvalidLaunch(
            "that session id can't be resumed".into(),
        ));
    }
    Ok(())
}

/// `CODEX_HOME=` assignments in the shell rc files under `home`.
#[must_use]
pub fn codex_aliases(home: &Path) -> Vec<juggler_adapter::environment::AliasHint> {
    let (hints, _) = crate::scan_signals::scan_rc_assignments(home);
    hints
        .into_iter()
        .filter(|hint| hint.variable == ENV_CODEX_HOME)
        .collect()
}

/// `codex --version` prints `codex-cli 0.163.0`; older builds printed the
/// bare number. The version is the last token of the first line.
#[must_use]
pub fn parse_version(stdout: &str) -> Option<String> {
    let token = stdout.lines().next()?.split_whitespace().last()?;
    token
        .starts_with(|c: char| c.is_ascii_digit())
        .then(|| token.to_owned())
}

/// See [`CodexAdapter::login_state`].
#[must_use]
pub fn codex_login_state(state_dir: &Path) -> LoginState {
    if std::fs::read_dir(state_dir).is_err() {
        return LoginState::Unknown; // probe failure ≠ logged out (AJ-E308)
    }
    let path = state_dir.join("auth.json");
    let Ok(meta) = std::fs::metadata(&path) else {
        // Keychain storage (`cli_auth_credentials_store = "keyring"`)
        // leaves no file: the entry's existence is the answer.
        #[cfg(target_os = "macos")]
        if keychain_entry_exists(state_dir) {
            return LoginState::SignedIn;
        }
        return LoginState::LoggedOut;
    };
    if meta.len() == 0 {
        return LoginState::Partial; // a sign-in abandoned mid-write
    }
    // ADR-0003: the file is streamed into `AuthShape`, which keeps only
    // whether each field is there — every value (tokens, keys) is skipped
    // while parsing and never stored, logged or returned.
    let Ok(file) = std::fs::File::open(&path) else {
        return LoginState::Unknown;
    };
    match serde_json::from_reader::<_, AuthShape>(std::io::BufReader::new(file)) {
        Ok(shape) => classify_auth(&shape),
        Err(_) => LoginState::Unknown,
    }
}

/// Whether a field holds something — never what. A string counts when
/// non-empty, an object when it has entries; the value is consumed by
/// the parser and dropped on the spot (ADR-0003).
#[derive(Debug, Default, Clone, Copy)]
struct Present(bool);

impl<'de> serde::Deserialize<'de> for Present {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Shape;
        impl<'de> serde::de::Visitor<'de> for Shape {
            type Value = Present;
            fn expecting(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                out.write_str("any JSON value")
            }
            fn visit_str<E>(self, value: &str) -> Result<Present, E> {
                Ok(Present(!value.is_empty()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Present, A::Error> {
                let mut any = false;
                while map
                    .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                    .is_some()
                {
                    any = true;
                }
                Ok(Present(any))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Present, A::Error> {
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
                Ok(Present(false))
            }
            fn visit_unit<E>(self) -> Result<Present, E> {
                Ok(Present(false))
            }
            fn visit_bool<E>(self, _: bool) -> Result<Present, E> {
                Ok(Present(false))
            }
            fn visit_i64<E>(self, _: i64) -> Result<Present, E> {
                Ok(Present(false))
            }
            fn visit_u64<E>(self, _: u64) -> Result<Present, E> {
                Ok(Present(false))
            }
            fn visit_f64<E>(self, _: f64) -> Result<Present, E> {
                Ok(Present(false))
            }
        }
        deserializer.deserialize_any(Shape)
    }
}

/// The shape of `auth.json` the probe needs — field names and presence;
/// `auth_mode` is a mode name, not a secret.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct AuthShape {
    auth_mode: Option<String>,
    #[serde(rename = "OPENAI_API_KEY")]
    api_key: Present,
    tokens: Option<TokenShape>,
    personal_access_token: Present,
    bedrock_api_key: Present,
    bedrock_access_keys: Present,
}

/// The `tokens` object's shape (presence only).
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct TokenShape {
    access_token: Present,
    refresh_token: Present,
}

/// Classifies an `auth.json` shape by field names and presence only.
/// `auth_mode` decides when present — a `ChatGPT` sign-in may ALSO carry an
/// exchanged API key, so `OPENAI_API_KEY` alone never means an API-key
/// account (codex-rs `login/src/auth/manager.rs` `resolved_mode`).
fn classify_auth(shape: &AuthShape) -> LoginState {
    let has_tokens = shape
        .tokens
        .as_ref()
        .is_some_and(|t| t.refresh_token.0 || t.access_token.0);
    let has_key = shape.api_key.0;
    let has_pat = shape.personal_access_token.0;
    let has_bedrock = shape.bedrock_api_key.0 || shape.bedrock_access_keys.0;
    let mode = shape.auth_mode.as_deref();
    // Files written before `auth_mode` existed follow Codex's own
    // fallback order: personal access token, Bedrock, API key, tokens.
    let api_key = match mode {
        Some("apikey") => has_key,
        Some(_) => false,
        None => !has_pat && (has_bedrock || has_key),
    };
    let signed_in = match mode {
        Some("chatgpt" | "chatgptAuthTokens") => has_tokens,
        Some("personalAccessToken") => has_pat,
        Some(_) => false,
        None => has_pat || has_tokens,
    };
    if api_key {
        LoginState::ApiKey
    } else if signed_in {
        LoginState::SignedIn
    } else {
        LoginState::LoggedOut
    }
}

/// Codex's Keychain item for a `CODEX_HOME`: service `Codex Auth`,
/// account `cli|<first 16 hex of sha256(canonical home)>` (codex-rs
/// `login/src/auth/storage.rs`). Existence only — `-w` is never passed.
#[cfg(target_os = "macos")]
fn keychain_entry_exists(state_dir: &Path) -> bool {
    let Some(account) = keychain_account(state_dir) else {
        return false;
    };
    std::process::Command::new("security")
        .args(["find-generic-password", "-s", "Codex Auth", "-a", &account])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The Keychain account name Codex derives from a home directory.
#[must_use]
pub fn keychain_account(state_dir: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let canonical = state_dir.canonicalize().ok()?;
    let digest = Sha256::digest(canonical.to_string_lossy().as_bytes());
    let hex = digest.iter().take(8).fold(String::new(), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    });
    Some(format!("cli|{hex}"))
}

/// The vendor session id of a rollout file: the thread UUID in its name
/// (falls back to the stem for anything unexpected).
fn rollout_native_id(path: &Path) -> String {
    path.file_name()
        .and_then(|name| crate::codex_transcript::thread_id_from_name(&name.to_string_lossy()))
        .unwrap_or_else(|| crate::claude_code::file_stem(path))
}

/// See [`CodexAdapter::probe_identity`]: field names and the account id
/// only; API-key and Keychain sign-ins cannot be told apart and say so.
#[must_use]
pub fn probe_codex_identity(state_dir: &Path) -> juggler_adapter::IdentityProbe {
    use juggler_adapter::IdentityProbe;
    match codex_login_state(state_dir) {
        LoginState::SignedIn => {}
        LoginState::ApiKey => {
            return IdentityProbe::Unverifiable {
                reason: "API-key sign-ins carry no account id".into(),
            };
        }
        LoginState::Unknown => {
            return IdentityProbe::Unverifiable {
                reason: "Codex's sign-in file couldn't be read".into(),
            };
        }
        LoginState::Partial
        | LoginState::Expired
        | LoginState::SsoPending
        | LoginState::LoggedOut => {
            return IdentityProbe::SignedOut;
        }
    }
    // Streamed into a shape that keeps ONLY `tokens.account_id` (an id,
    // not a credential); every token value is skipped (ADR-0003).
    let account_id = std::fs::File::open(state_dir.join("auth.json"))
        .ok()
        .and_then(|file| serde_json::from_reader::<_, IdShape>(std::io::BufReader::new(file)).ok())
        .and_then(|shape| shape.tokens?.account_id)
        .filter(|id| !id.is_empty());
    match account_id {
        Some(subject) => {
            let tail: String = subject
                .chars()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            IdentityProbe::SignedIn {
                label: format!("ChatGPT account …{tail}"),
                subject,
            }
        }
        // Signed in through the Keychain, or a file without the id.
        None => IdentityProbe::Unverifiable {
            reason: "this Codex sign-in records no account id Juggler can read".into(),
        },
    }
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

/// The scan-side sign-in axis stays binary (the inference vocabulary);
/// the account wire carries the full taxonomy (AJ-E308), as for Claude.
fn scan_probe(path: &Path) -> LoginProbe {
    match codex_login_state(path) {
        LoginState::SignedIn => LoginProbe::LoggedIn,
        LoginState::Partial
        | LoginState::Expired
        | LoginState::SsoPending
        | LoginState::ApiKey
        | LoginState::LoggedOut
        | LoginState::Unknown => LoginProbe::LoggedOut,
    }
}

/// The isolation overlay: the home, and the SQLite home pinned inside it
/// so an inherited `CODEX_SQLITE_HOME` can never join two accounts.
fn home_env(state_dir: &Path) -> EnvMap {
    let dir = state_dir.to_string_lossy().into_owned();
    let mut env = EnvMap::new();
    env.insert(ENV_CODEX_HOME.to_string(), dir.clone());
    env.insert(ENV_SQLITE_HOME.to_string(), dir);
    env
}

impl HarnessAdapter for CodexAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(CODEX_ID)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::HEADLESS
            | Capabilities::RESUME
            | Capabilities::FORK
            | Capabilities::TRANSCRIPT_WATCH
            | Capabilities::USAGE_TOKENS
    }

    fn verified_against(&self) -> &'static [&'static str] {
        VERIFIED_AGAINST
    }

    fn discover(&self) -> DiscoveryReport {
        let Some(bin) = self.resolve_bin() else {
            return self.not_found(
                "looked on PATH, /opt/homebrew/bin, /usr/local/bin, ~/.local/bin, ~/.npm-global/bin"
                    .into(),
            );
        };
        if !bin.is_file() {
            return self.not_found(format!(
                "resolved {} but the file does not exist",
                bin.display()
            ));
        }
        match std::process::Command::new(&bin).arg("--version").output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let Some(version) = parse_version(&stdout) else {
                    return self.failed(bin, format!("unrecognised --version output: {stdout:?}"));
                };
                let verified = VERIFIED_AGAINST.contains(&version.as_str());
                DiscoveryReport {
                    adapter: self.id(),
                    install: InstallStatus::Installed {
                        version: version.clone(),
                    },
                    verification: if verified {
                        VerificationStatus::Verified
                    } else {
                        VerificationStatus::Unverified
                    },
                    binary: Some(bin),
                    notes: if verified {
                        Vec::new()
                    } else {
                        vec![format!(
                            "version drift: {version}; this adapter was verified against {VERIFIED_AGAINST:?}"
                        )]
                    },
                }
            }
            Ok(output) => self.failed(
                bin,
                format!("--version exited with {status}", status = output.status),
            ),
            Err(error) => self.failed(bin, error.to_string()),
        }
    }

    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        // Codex refuses a CODEX_HOME that does not exist (verified on
        // 0.163.0) — the directory itself is the whole preparation.
        prepare_managed_dir_for(state_dir, CODEX_ID)?;
        Ok(home_env(state_dir))
    }

    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        let bin = self.require_bin()?;
        let effective_cwd = profile
            .default_cwd
            .clone()
            .unwrap_or_else(|| cwd.to_path_buf());
        // Launch-settings overrides first, isolation LAST (AJ-E309).
        let mut env: EnvMap = profile.settings_overlay.env.clone();
        env.extend(home_env(&account.state_dir));
        // An API key in the environment would bill every managed account
        // to that key, whatever it signed in as: unset for the launch —
        // from Launch settings and from what the app itself inherited
        // (the supervisor removes a variable set to `ENV_UNSET`).
        unset_inherited_keys(&mut env);
        if let LaunchMode::Resume { session_id, .. } = mode {
            refuse_option_like(session_id)?;
        }
        let mut args: Vec<String> = Vec::new();
        let needs_pty = !matches!(mode, LaunchMode::Headless { .. });
        match mode {
            LaunchMode::Headless { .. } => args.extend(["exec".into(), "--json".into()]),
            LaunchMode::Resume { session_id, .. } => {
                // `-C` answers the TUI's "resume here or in the session's
                // folder?" question up front: here.
                args.extend([
                    "resume".into(),
                    session_id.clone(),
                    "-C".into(),
                    effective_cwd.to_string_lossy().into_owned(),
                ]);
            }
            LaunchMode::Interactive | LaunchMode::Primed { .. } => {}
        }
        if needs_pty {
            args.push(NO_DAEMON.into());
        }
        if let Some(model) = &profile.settings_overlay.model {
            args.extend(["--model".into(), model.clone()]);
        }
        // The prompt goes last, after `--`: user text that starts with
        // "-" (a markdown bullet, or a flag) is never read as an option
        // (verified on 0.163.0: `codex -- "- fix X"` parses, without `--`
        // it is an "unexpected argument").
        match mode {
            LaunchMode::Headless { prompt } | LaunchMode::Primed { prompt } => {
                args.extend(["--".into(), prompt.clone()]);
            }
            LaunchMode::Resume {
                delta_prompt: Some(delta),
                ..
            } => args.extend(["--".into(), delta.clone()]),
            LaunchMode::Resume { .. } | LaunchMode::Interactive => {}
        }
        Ok(LaunchSpec {
            program: bin,
            args,
            env,
            cwd: effective_cwd,
            needs_pty,
        })
    }

    fn login_state(&self, state_dir: &Path) -> LoginState {
        codex_login_state(state_dir)
    }

    /// AJ-E1303: watches `<home>/sessions`; the native id is the thread
    /// UUID `codex resume` takes (from the rollout's name).
    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        crate::claude_code::ClaudeTranscriptWatcher::with_ids(transcript_root, rollout_native_id)
            .map(|watcher| Box::new(watcher) as Box<dyn SessionWatcher>)
    }

    /// AJ-E1303: the rollout under `sessions/YYYY/MM/DD/` whose
    /// `session_meta` cwd is `cwd`, created at or after `since_ms`.
    fn locate_transcript(
        &self,
        state_dir: &Path,
        cwd: &Path,
        since_ms: i64,
        taken: &[PathBuf],
    ) -> Option<juggler_core::model::TranscriptRef> {
        let mut files = Vec::new();
        crate::claude_code::collect_jsonl(&state_dir.join("sessions"), &mut files, 0);
        files.retain(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("rollout-"))
        });
        crate::claude_code::only_since(files, since_ms, taken, |bytes| {
            crate::codex_transcript::rollout_cwd(bytes)
                .is_some_and(|found| crate::claude_code::same_folder(&found, cwd))
        })
        .map(|path| juggler_core::model::TranscriptRef {
            native_id: Some(rollout_native_id(&path)),
            path,
        })
    }

    /// AJ-E1303: the `tokens.account_id` field of `auth.json` is the
    /// subject; the label is that id masked. No token is decoded — the
    /// email and plan exist only inside the `id_token`, a credential.
    fn probe_identity(&self, account: &Account) -> juggler_adapter::IdentityProbe {
        probe_codex_identity(&account.state_dir)
    }

    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        self.login_spec_inherent(state_dir)
    }

    /// AJ-E1302: `~/.codex`, `$CODEX_HOME` and `~/.codex*` homes —
    /// metadata only (the sign-in probe reads field names, never values).
    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        let Some(home) = self.home.clone().or_else(home_dir) else {
            return Vec::new();
        };
        let env_override = std::env::var_os(ENV_CODEX_HOME)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        crate::codex_environment::scan_codex_environment(
            &home,
            env_override.as_deref(),
            &scan_probe,
        )
    }

    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::codex_environment::import_codex_home(source, dest, &scan_probe)
    }

    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        crate::codex_environment::inspect_codex_source(source, &scan_probe)
    }

    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        crate::codex_environment::import_codex_settings_only(source, dest, &scan_probe)
    }

    fn isolation_env_keys(&self) -> &'static [&'static str] {
        &[ENV_CODEX_HOME, ENV_SQLITE_HOME, ENV_BIN_OVERRIDE]
    }

    /// `CODEX_API_KEY` / `OPENAI_API_KEY`: unset for every launch and in
    /// the copyable snippet (an inherited key would bill every account).
    fn launch_unset_keys(&self) -> &'static [&'static str] {
        INHERITED_KEYS
    }

    /// Both the home and the SQLite home point at the Account's folder —
    /// the snippet sets both, so an exported `CODEX_SQLITE_HOME` never
    /// joins two accounts.
    fn pinned_env_keys(&self) -> &'static [&'static str] {
        &[ENV_CODEX_HOME, ENV_SQLITE_HOME]
    }

    /// AJ-E302 S5: rc assignments of `CODEX_HOME` only (`CLAUDE_CONFIG_DIR`
    /// belongs to the Claude adapter).
    fn scan_shell_aliases(&self) -> Vec<juggler_adapter::environment::AliasHint> {
        self.home
            .clone()
            .or_else(crate::claude_code::home_dir)
            .map_or_else(Vec::new, |home| codex_aliases(&home))
    }

    /// Plain `codex`: it runs on every version (`--no-daemon` needs 0.156
    /// or later), and a session started by hand is the user's own process,
    /// not one Juggler tracks — the daemon reasoning applies to launches.
    fn shell_command(&self) -> Option<&'static str> {
        Some("codex")
    }
}
