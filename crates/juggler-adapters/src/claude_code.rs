//! Claude Code adapter (spec §4.1) — the first real harness integration.
//!
//! Isolation model (research `docs/research/claude-code.md` §2–§3): each
//! managed account gets its own `CLAUDE_CONFIG_DIR`; since Claude Code
//! v2.1.56 the macOS keychain entry is namespaced per config dir, so
//! parallel accounts no longer clobber each other's OAuth tokens.
//!
//! Guardrails (ADR-0003): this adapter never reads credential *contents* —
//! login detection checks file presence/size and macOS keychain entry
//! existence only; login itself always runs through the vendor's own
//! interactive `claude` flow launched in a guided terminal.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::claude_environment;
use juggler_adapter::environment::{
    DiscoveredInstall, ImportOutcome, InstallOrigin, LoginProbe, LoginUrl,
};
use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, HarnessAdapter, InstallStatus, LaunchMode,
    LaunchSpec, SessionEvent, SessionWatcher, VerificationStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

/// Harness id: `claude-code`.
pub const CLAUDE_CODE_ID: &str = "claude-code";

/// Versions this adapter's launch/watch behavior was verified against
/// (research pinned 2.1.296, 2026-10-12). Other 2.1.x versions run with an
/// unverified-drift note — never a hard failure (spec §4 rules).
const VERIFIED_AGAINST: &[&str] = &["2.1.296"];

/// Marker file identifying a directory as managed by AI Juggler.
/// The managed-dir marker filename (AJ-E308 isolation row 2 reads it).
pub const MANAGED_MARKER: &str = ".ai-juggler-managed";

/// Environment variable naming the isolated config dir (spec §3).
const ENV_CONFIG_DIR: &str = "CLAUDE_CONFIG_DIR";

/// Optional test/dev override for the harness binary path.
const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_CLAUDE_BIN";

/// Adapter for Anthropic's Claude Code CLI.
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeAdapter {
    bin: Option<PathBuf>,
    /// ADR-0018: where in-place accounts' launch overlays live (AI
    /// Juggler's data). `None` = no status line capture for them.
    overlays_root: Option<PathBuf>,
}

impl ClaudeCodeAdapter {
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
            overlays_root: None,
        }
    }

    /// ADR-0018: in-place accounts get their status line capture from a
    /// `--settings` file under `root` instead of edits to their folder.
    #[must_use]
    pub fn with_overlays_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.overlays_root = Some(root.into());
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
        find_on_path("claude").or_else(|| {
            // GUI launches don't inherit the shell PATH (spec §4.1
            // discovery robustness); cover the installer's defaults.
            home_relative(".local/bin/claude")
                .filter(|p| p.is_file())
                .or_else(|| existing(&["/opt/homebrew/bin/claude", "/usr/local/bin/claude"]))
        })
    }

    fn report(&self, bin: PathBuf, version: String) -> DiscoveryReport {
        let verification = if VERIFIED_AGAINST.contains(&version.as_str()) {
            VerificationStatus::Verified
        } else {
            VerificationStatus::Unverified
        };
        let notes = if matches!(verification, VerificationStatus::Unverified) {
            vec![format!(
                "version drift: {version}; this adapter was verified against {VERIFIED_AGAINST:?}"
            )]
        } else {
            Vec::new()
        };
        DiscoveryReport {
            adapter: self.id(),
            install: InstallStatus::Installed { version },
            verification,
            binary: Some(bin),
            notes,
        }
    }
}

impl HarnessAdapter for ClaudeCodeAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(CLAUDE_CODE_ID)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::HEADLESS
            | Capabilities::RESUME
            | Capabilities::TRANSCRIPT_EXPORT
            | Capabilities::TRANSCRIPT_WATCH
            | Capabilities::USAGE_TOKENS
            | Capabilities::USAGE_QUOTA_FEED
            | Capabilities::FORK
    }

    fn verified_against(&self) -> &'static [&'static str] {
        VERIFIED_AGAINST
    }

    fn discover(&self) -> DiscoveryReport {
        let Some(bin) = self.resolve_bin() else {
            return DiscoveryReport {
                adapter: self.id(),
                install: InstallStatus::NotFound,
                verification: VerificationStatus::Unverified,
                binary: None,
                notes: vec![
                    "looked on PATH, ~/.local/bin, /opt/homebrew/bin, /usr/local/bin".into(),
                ],
            };
        };
        if !bin.is_file() {
            return DiscoveryReport {
                adapter: self.id(),
                install: InstallStatus::NotFound,
                verification: VerificationStatus::Unverified,
                binary: None,
                notes: vec![format!(
                    "resolved {} but the file does not exist",
                    bin.display()
                )],
            };
        }
        // With the user's PATH: an npm-installed CLI runs on `node`.
        match juggler_adapter::user_path::command(&bin)
            .arg("--version")
            .output()
        {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let version = stdout
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                if version.is_empty() {
                    DiscoveryReport {
                        adapter: self.id(),
                        install: InstallStatus::Failed {
                            message: format!("empty --version output: {stdout:?}"),
                        },
                        verification: VerificationStatus::Unverified,
                        binary: Some(bin),
                        notes: Vec::new(),
                    }
                } else {
                    self.report(bin, version)
                }
            }
            Ok(output) => DiscoveryReport {
                adapter: self.id(),
                install: InstallStatus::Failed {
                    message: format!("--version exited with {status}", status = output.status),
                },
                verification: VerificationStatus::Unverified,
                binary: Some(bin),
                notes: Vec::new(),
            },
            Err(error) => DiscoveryReport {
                adapter: self.id(),
                install: InstallStatus::Failed {
                    message: error.to_string(),
                },
                verification: VerificationStatus::Unverified,
                binary: Some(bin),
                notes: Vec::new(),
            },
        }
    }

    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        prepare_managed_dir(state_dir)?;
        let mut env = EnvMap::new();
        env.insert(
            ENV_CONFIG_DIR.to_string(),
            state_dir.to_string_lossy().into_owned(),
        );
        Ok(env)
    }

    /// AJ-E302 S5: rc assignments for THIS harness's variable only
    /// (`CODEX_HOME` belongs to the codex adapter).
    fn scan_shell_aliases(&self) -> Vec<juggler_adapter::environment::AliasHint> {
        let Some(home) = home_dir() else {
            return Vec::new();
        };
        let (hints, _) = crate::scan_signals::scan_rc_assignments(&home);
        hints
            .into_iter()
            .filter(|hint| hint.variable == ENV_CONFIG_DIR)
            .collect()
    }

    /// AJ-E302 S3/S4: t3 profile count + the cc-switch DB probe.
    fn scan_managers(&self) -> Vec<juggler_adapter::environment::ManagerHint> {
        let mut out = Vec::new();
        if let Some(home) = home_dir() {
            let t3 = home
                .join("Library")
                .join("Application Support")
                .join("ClaudeCodeProfiles");
            if t3.is_dir() {
                let count = std::fs::read_dir(&t3)
                    .map(|entries| entries.flatten().filter(|e| e.path().is_dir()).count())
                    .unwrap_or_default();
                out.push(juggler_adapter::environment::ManagerHint {
                    manager: "t3".to_owned(),
                    present: true,
                    detail: format!("{count} profile(s)"),
                    health: juggler_adapter::environment::ScanHealth::Ok,
                });
            }
            out.push(crate::scan_signals::scan_cc_switch(&home));
        }
        out
    }

    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        let env_override = std::env::var_os(ENV_CONFIG_DIR).map(PathBuf::from);
        let adapter = ClaudeCodeAdapter::new();
        #[cfg(target_os = "macos")]
        let legacy_shared_keychain = keychain_entry_exists("Claude Code-credentials");
        #[cfg(not(target_os = "macos"))]
        let legacy_shared_keychain = false;
        let probe = move |path: &Path| match adapter.login_state(path) {
            LoginState::SignedIn => LoginProbe::LoggedIn,
            LoginState::LoggedOut if legacy_shared_keychain => LoginProbe::LoggedIn,
            // Scan-side axis stays binary (inference vocabulary); the
            // account-state WIRE carries the full taxonomy (AJ-E308).
            _ => LoginProbe::LoggedOut,
        };
        let home = home_relative(".")
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        // t3.codes keeps whole Claude profile roots here (ADR-0006
        // "other manager tools" case, verified on a real machine).
        let manager_roots = [home.join("Library/Application Support/ClaudeCodeProfiles")];
        let mut found = claude_environment::scan_claude_environment(
            &home,
            env_override.as_deref(),
            &manager_roots,
            &probe,
        );
        if legacy_shared_keychain {
            for install in &mut found {
                if install.login == LoginProbe::LoggedIn
                    && install.origin == InstallOrigin::ManagerTool
                {
                    install.notes.push(
                        "Signed in via a shared (pre-2.1.56) keychain entry — after importing, plan on one quick re-login inside AI Juggler.".to_string(),
                    );
                }
            }
        }
        found
    }

    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        let adapter = ClaudeCodeAdapter::new();
        let probe = move |path: &Path| match adapter.login_state(path) {
            // Scan-side axis stays binary (the inference's
            // signed-in/not vocabulary); the account-state WIRE
            // carries the full taxonomy (AJ-E308).
            LoginState::SignedIn => LoginProbe::LoggedIn,
            LoginState::Partial
            | LoginState::Expired
            | LoginState::SsoPending
            | LoginState::ApiKey
            | LoginState::LoggedOut
            | LoginState::Unknown => LoginProbe::LoggedOut,
        };
        claude_environment::import_claude_config(source, dest, &probe)
    }

    fn can_use_in_place(&self) -> bool {
        true
    }

    fn vendor_default_dir(&self) -> Option<PathBuf> {
        home_dir().map(|home| home.join(".claude"))
    }

    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        let adapter = ClaudeCodeAdapter::new();
        let probe = move |path: &Path| match adapter.login_state(path) {
            LoginState::SignedIn => LoginProbe::LoggedIn,
            _ => LoginProbe::LoggedOut,
        };
        claude_environment::adopt_claude_config(source, &probe)
    }

    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        let adapter = ClaudeCodeAdapter::new();
        let probe = move |path: &Path| match adapter.login_state(path) {
            // Scan-side axis stays binary (the inference's
            // signed-in/not vocabulary); the account-state WIRE
            // carries the full taxonomy (AJ-E308).
            LoginState::SignedIn => LoginProbe::LoggedIn,
            LoginState::Partial
            | LoginState::Expired
            | LoginState::SsoPending
            | LoginState::ApiKey
            | LoginState::LoggedOut
            | LoginState::Unknown => LoginProbe::LoggedOut,
        };
        claude_environment::inspect_source(source, &probe)
    }

    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        let adapter = ClaudeCodeAdapter::new();
        let probe = move |path: &Path| match adapter.login_state(path) {
            // Scan-side axis stays binary (the inference's
            // signed-in/not vocabulary); the account-state WIRE
            // carries the full taxonomy (AJ-E308).
            LoginState::SignedIn => LoginProbe::LoggedIn,
            LoginState::Partial
            | LoginState::Expired
            | LoginState::SsoPending
            | LoginState::ApiKey
            | LoginState::LoggedOut
            | LoginState::Unknown => LoginProbe::LoggedOut,
        };
        claude_environment::import_settings_only(source, dest, &probe)
    }

    fn login_url(&self) -> Option<LoginUrl> {
        Some("https://claude.ai/login")
    }

    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        ClaudeCodeAdapter::login_spec_inherent(self, state_dir)
    }

    fn isolation_env_keys(&self) -> &'static [&'static str] {
        &[ENV_CONFIG_DIR, ENV_BIN_OVERRIDE]
    }

    fn launch_unset_keys(&self) -> &'static [&'static str] {
        &INHERITED_CREDENTIAL_KEYS
    }

    fn isolation_env_value(&self, state_dir: &Path) -> Option<String> {
        let value = config_dir_value(state_dir);
        (value != juggler_adapter::launch::ENV_UNSET).then_some(value)
    }

    fn shell_command(&self) -> Option<&'static str> {
        Some("claude")
    }

    fn resolved_binary(&self) -> Option<PathBuf> {
        self.resolve_bin()
    }

    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        let Some(bin) = self.resolve_bin() else {
            return Err(AdapterError::InvalidLaunch(
                "claude binary not found; run discovery for details".into(),
            ));
        };
        let effective_cwd = profile
            .default_cwd
            .clone()
            .unwrap_or_else(|| cwd.to_path_buf());
        // Launch-settings overrides first, isolation LAST: an
        // override can never repoint the config dir (AJ-E309 — the
        // env editor refuses these keys too; this is the backstop).
        let mut env: EnvMap = profile.settings_overlay.env.clone();
        env.insert(
            ENV_CONFIG_DIR.to_string(),
            config_dir_value(&account.state_dir),
        );
        unset_inherited_credentials(&mut env);
        let (mut args, needs_pty) = match mode {
            LaunchMode::Interactive => (Vec::new(), true),
            // `claude "<prompt>"` — an interactive session whose first
            // turn is the prompt (AJ-E706 T3 replay).
            LaunchMode::Primed { prompt } => (vec![prompt.clone()], true),
            LaunchMode::Headless { prompt } => (
                vec![
                    "-p".into(),
                    prompt.clone(),
                    "--output-format".into(),
                    "stream-json".into(),
                ],
                false,
            ),
            LaunchMode::Resume {
                session_id,
                delta_prompt,
            } => {
                // `--resume <id> "<delta>"` — the resumed conversation
                // receives the delta as its next user turn (E207). An id
                // that looks like an option is refused, never passed.
                crate::codex::refuse_option_like(session_id)?;
                let mut args = vec!["--resume".into(), session_id.clone()];
                if let Some(delta) = delta_prompt {
                    args.push(delta.clone());
                }
                (args, true)
            }
        };
        // Per-launch settings go in as flags, never as edits to the
        // folder's settings.json (ADR-0018 §3): the profile's model, and
        // for an in-place account the status line capture in AI Juggler's
        // own folder.
        let mut leading = Vec::new();
        if let Some(model) = &profile.settings_overlay.model {
            leading.extend(["--model".to_owned(), model.clone()]);
        }
        if account.in_place
            && let Some(root) = &self.overlays_root
            && let Ok(settings) = crate::quota_ingest::ensure_launch_overlay(
                root,
                account.id.as_str(),
                &account.state_dir,
            )
        {
            leading.extend([
                "--settings".to_owned(),
                settings.to_string_lossy().into_owned(),
            ]);
        }
        args.splice(0..0, leading);
        Ok(LaunchSpec {
            program: bin,
            args,
            env,
            cwd: effective_cwd,
            needs_pty,
        })
    }

    /// AJ-E1001: reads ONLY the `oauthAccount` object of the state
    /// dir's `.claude.json` — the account id, email and organisation
    /// name Claude Code records there. Credentials live elsewhere (the
    /// Keychain or `.credentials.json`) and are never opened.
    fn probe_identity(&self, account: &Account) -> juggler_adapter::IdentityProbe {
        probe_claude_identity(&account.state_dir)
    }

    fn login_state(&self, state_dir: &Path) -> LoginState {
        ClaudeCodeAdapter::login_state(self, state_dir)
    }

    /// Claude keeps `projects/<mangled cwd>/<session>.jsonl`; the cwd
    /// is read from the file (never decoded from the name, H-3).
    fn locate_transcript(
        &self,
        state_dir: &Path,
        cwd: &Path,
        since_ms: i64,
        taken: &[PathBuf],
    ) -> Option<juggler_core::model::TranscriptRef> {
        let mut candidates = Vec::new();
        for project in std::fs::read_dir(state_dir.join("projects"))
            .ok()?
            .flatten()
        {
            let Ok(files) = std::fs::read_dir(project.path()) else {
                continue;
            };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().is_some_and(|ext| ext == "jsonl") {
                    candidates.push(path);
                }
            }
        }
        only_since(candidates, since_ms, taken, |bytes| {
            juggler_adapter::handover::embedded_cwd(bytes)
                .is_some_and(|found| same_folder(&found, cwd))
        })
        .map(|path| juggler_core::model::TranscriptRef {
            native_id: Some(file_stem(&path)),
            path,
        })
    }

    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        ClaudeTranscriptWatcher::new(transcript_root)
            .map(|watcher| Box::new(watcher) as Box<dyn SessionWatcher>)
    }
}

/// The AJ-E308 sign-in taxonomy — moved to the adapter contract for
/// AJ-E1301 (re-exported here so existing paths keep working).
pub use juggler_adapter::environment::LoginState;

/// Current epoch seconds (std-only).
fn epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

impl ClaudeCodeAdapter {
    /// Probes whether the account in `state_dir` is logged in.
    ///
    /// Detection is metadata-only (ADR-0003): a non-empty
    /// `.credentials.json` inside the config dir, or — on macOS — the
    /// per-config-dir keychain entry (`Claude Code-credentials-{sha8}`,
    /// namespaced since Claude Code v2.1.56). A missing keychain entry
    /// simply reports `LoggedOut`; the user re-runs the vendor's own login.
    #[must_use]
    pub fn login_state(&self, state_dir: &Path) -> LoginState {
        // AJ-E308 Req.1: the seven-state classification — structure
        // and metadata only; credential CONTENTS are never read or
        // logged (ADR-0003).
        if std::fs::read_dir(state_dir).is_err() {
            return LoginState::Unknown; // probe failure ≠ logged out
        }
        if state_dir.join("claude-sso-session.json").exists() {
            return LoginState::SsoPending;
        }
        let credentials = state_dir.join(".credentials.json");
        if let Ok(meta) = std::fs::metadata(&credentials) {
            if meta.len() == 0 {
                return LoginState::Partial;
            }
            if Self::credential_expiry_passed(&credentials) {
                return LoginState::Expired;
            }
            return LoginState::SignedIn;
        }
        if Self::settings_name_an_api_key(&state_dir.join("settings.json")) {
            return LoginState::ApiKey;
        }
        #[cfg(target_os = "macos")]
        if keychain_entry_exists(&namespaced_service(state_dir)) {
            return LoginState::SignedIn;
        }
        // The vendor's default folder runs without CLAUDE_CONFIG_DIR, and
        // its sign-in has the un-namespaced name (ADR-0018 §3).
        #[cfg(target_os = "macos")]
        if is_vendor_default_dir(state_dir) && keychain_entry_exists(SHARED_SERVICE) {
            return LoginState::SignedIn;
        }
        LoginState::LoggedOut
    }

    /// True when the credentials file's expiry claim is in the past
    /// (the documented field only; the token is never read or logged).
    fn credential_expiry_passed(path: &Path) -> bool {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return false;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return false;
        };
        value
            .get("expires_at")
            .or_else(|| value.get("expiry"))
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|at| at < epoch_secs())
    }

    /// True when settings.json carries a non-empty API-key FIELD
    /// (existence only — the value is never read, ADR-0003).
    fn settings_name_an_api_key(path: &Path) -> bool {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return false;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return false;
        };
        value
            .get("primaryApiKey")
            .or_else(|| value.get("apiKey"))
            .or_else(|| value.get("api_key"))
            .and_then(|key| key.as_str())
            .is_some_and(|text| !text.is_empty())
    }

    /// Builds the launch spec for the vendor's own interactive login flow —
    /// the "guided terminal login" primitive (spec §4.1). AI Juggler spawns
    /// it; the user completes the OAuth flow; nothing is automated.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::InvalidLaunch`] when no claude binary is
    /// resolvable.
    pub fn login_spec_inherent(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        let Some(bin) = self.resolve_bin() else {
            return Err(AdapterError::InvalidLaunch(
                "claude binary not found; run discovery for details".into(),
            ));
        };
        let mut env = EnvMap::new();
        env.insert(ENV_CONFIG_DIR.to_string(), config_dir_value(state_dir));
        unset_inherited_credentials(&mut env);
        // Not the config folder itself: Claude would treat it as a project
        // (a trust prompt, a projects/ entry for the folder) — an empty
        // folder of AI Juggler's own, when there is one.
        let cwd = self
            .overlays_root
            .as_ref()
            .map(|root| root.join("sign-in"))
            .filter(|dir| std::fs::create_dir_all(dir).is_ok())
            .unwrap_or_else(|| state_dir.to_path_buf());
        Ok(LaunchSpec {
            program: bin,
            args: Vec::new(),
            env,
            cwd,
            needs_pty: true,
        })
    }
}

/// True when the directory carries the AI Juggler management marker.
#[must_use]
pub fn is_managed_dir(state_dir: &Path) -> bool {
    state_dir.join(MANAGED_MARKER).is_file()
}

/// Creates the managed account dir (0700) and writes the AI Juggler marker
/// (0600) — the single directory-initialization path shared by
/// `prepare_account` and config import.
///
/// # Errors
///
/// Returns [`AdapterError::AccountDir`] when the dir or marker cannot be
/// written.
pub(crate) fn prepare_managed_dir(state_dir: &Path) -> Result<(), AdapterError> {
    prepare_managed_dir_for(state_dir, CLAUDE_CODE_ID)
}

/// [`prepare_managed_dir`] for any Provider: the marker names the
/// Provider whose state the directory holds.
///
/// # Errors
///
/// Returns [`AdapterError::AccountDir`] when the dir or marker cannot be
/// written.
pub(crate) fn prepare_managed_dir_for(
    state_dir: &Path,
    provider: &str,
) -> Result<(), AdapterError> {
    // A failure after creating the folder removes it again — only when
    // this call created it (an existing folder is never removed).
    let created_here = !state_dir.exists();
    let undo = || {
        if created_here {
            let _ = std::fs::remove_dir_all(state_dir);
        }
    };
    create_private_dir(state_dir).map_err(|error| {
        undo();
        AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: error.to_string(),
        }
    })?;
    // Marker identifies the directory as ours — guardrail primitive for
    // "never touch unmanaged dirs" checks (ADR-0003).
    let marker = state_dir.join(MANAGED_MARKER);
    std::fs::write(
        &marker,
        format!("managed-by: ai-juggler\nharness: {provider}\n"),
    )
    .and_then(|()| restrict_to_owner(&marker))
    .map_err(|error| {
        undo();
        AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: format!("cannot write marker: {error}"),
        }
    })
}

/// notify-based watcher over the account's transcript tree
/// (`<config>/projects/…`). Filesystem events trigger a line-count diff per
/// `.jsonl` file; new files emit [`SessionEvent::Discovered`], growth emits
/// [`SessionEvent::MessageAppended`]. Content is never parsed here — the
/// transcript parser runs downstream on demand (spec §4.1).
pub(crate) struct ClaudeTranscriptWatcher {
    root: PathBuf,
    /// The vendor session id of a transcript file (Claude: the stem;
    /// Codex: the thread UUID inside the rollout name).
    id_of: fn(&Path) -> String,
    _watcher: notify::RecommendedWatcher,
    events: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    seen: BTreeMap<PathBuf, usize>,
    /// (size, mtime) at the last read: an unchanged file is not read
    /// again (Codex keeps every past rollout under `sessions/`).
    stamps: BTreeMap<PathBuf, (u64, Option<std::time::SystemTime>)>,
    pending: std::collections::VecDeque<SessionEvent>,
}

impl ClaudeTranscriptWatcher {
    fn new(root: &Path) -> Result<Self, AdapterError> {
        Self::with_ids(root, file_stem)
    }

    /// A watcher whose native ids come from `id_of` (shared by Codex).
    pub(crate) fn with_ids(root: &Path, id_of: fn(&Path) -> String) -> Result<Self, AdapterError> {
        use notify::Watcher;
        let (tx, events) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)
            .map_err(|error| AdapterError::Watch(format!("cannot create fs watcher: {error}")))?;
        watcher
            .watch(root, notify::RecursiveMode::Recursive)
            .map_err(|error| {
                AdapterError::Watch(format!("cannot watch {}: {error}", root.display()))
            })?;
        Ok(Self {
            root: root.to_path_buf(),
            id_of,
            _watcher: watcher,
            events,
            seen: BTreeMap::new(),
            stamps: BTreeMap::new(),
            pending: std::collections::VecDeque::new(),
        })
    }

    /// Waits up to `timeout` for a filesystem event (returning early when
    /// one arrives), then rescans. The scan is unconditional: fs backends
    /// can drop or delay events (sleep/wake, coalescing), so the bounded
    /// walk is the self-healing source of truth and notify is the wake-up
    /// signal (spec §5: watching degrades gracefully).
    fn wait_and_rescan(&mut self, timeout: std::time::Duration) {
        if !timeout.is_zero() {
            let _ = self.events.recv_timeout(timeout);
            // Coalesce bursts before scanning once.
            while self.events.try_recv().is_ok() {}
        }
        self.scan();
    }

    /// Walks the watched tree and queues Discovered/MessageAppended diffs.
    fn scan(&mut self) {
        let mut files = Vec::new();
        collect_jsonl(&self.root, &mut files, 0);
        files.sort();
        for path in files {
            let stamp = std::fs::metadata(&path)
                .map(|meta| (meta.len(), meta.modified().ok()))
                .unwrap_or_default();
            if self.stamps.get(&path) == Some(&stamp) {
                continue;
            }
            self.stamps.insert(path.clone(), stamp);
            let lines = std::fs::read(&path).map_or(0, |bytes| count_lines(&bytes));
            let known = self.seen.entry(path.clone()).or_insert(0);
            if *known == 0 && lines > 0 {
                self.pending.push_back(SessionEvent::Discovered {
                    native_id: (self.id_of)(&path),
                    transcript_path: path.clone(),
                });
            } else if lines > *known {
                self.pending.push_back(SessionEvent::MessageAppended {
                    native_id: (self.id_of)(&path),
                    transcript_path: path.clone(),
                });
            }
            *known = lines;
        }
    }
}

impl SessionWatcher for ClaudeTranscriptWatcher {
    fn poll(&mut self, timeout: std::time::Duration) -> Result<Option<SessionEvent>, AdapterError> {
        if self.pending.is_empty() {
            self.wait_and_rescan(timeout);
        }
        Ok(self.pending.pop_front())
    }
}

/// Bounded recursive collection of `.jsonl` files (depth cap guards against
/// pathological symlink loops).
pub(crate) fn collect_jsonl(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    const MAX_DEPTH: usize = 8;
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl(&path, out, depth + 1);
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            out.push(path);
        }
    }
}

/// The ONE file of `candidates` created at or after `since_ms` (2 s of
/// clock slack), not in `taken`, whose head satisfies `matches` — `None`
/// when there is none or more than one (never a guess between two).
pub(crate) fn only_since(
    candidates: Vec<PathBuf>,
    since_ms: i64,
    taken: &[PathBuf],
    matches: impl Fn(&[u8]) -> bool,
) -> Option<PathBuf> {
    let born_ms = |path: &Path| {
        let meta = std::fs::metadata(path).ok()?;
        let time = meta.created().or_else(|_| meta.modified()).ok()?;
        let since = time.duration_since(std::time::UNIX_EPOCH).ok()?;
        i64::try_from(since.as_millis()).ok()
    };
    let mut found = candidates
        .into_iter()
        .filter(|path| !taken.contains(path))
        .filter_map(|path| Some((born_ms(&path)?, path)))
        .filter(|(born, _)| *born >= since_ms.saturating_sub(2_000))
        .filter(|(_, path)| {
            // The head carries the cwd; a first line can be large (a
            // rollout's `session_meta` holds the base instructions), so
            // the bound is generous but finite.
            use std::io::Read as _;
            let mut head = Vec::new();
            std::fs::File::open(path)
                .and_then(|file| file.take(HEAD_LIMIT).read_to_end(&mut head))
                .is_ok_and(|_| matches(&head))
        })
        .map(|(_, path)| path);
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// How much of a transcript's head the cwd match reads.
const HEAD_LIMIT: u64 = 8 * 1024 * 1024;

/// The same folder, symlinks resolved when both sides exist (`/var` vs
/// `/private/var`), else the exact string.
pub(crate) fn same_folder(recorded: &str, cwd: &Path) -> bool {
    let recorded = Path::new(recorded);
    match (recorded.canonicalize(), cwd.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => recorded == cwd,
    }
}

pub(crate) fn count_lines(bytes: &[u8]) -> usize {
    bytes.split(|b| *b == b'\n').count().saturating_sub(1)
}

pub(crate) fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// ---- helpers -------------------------------------------------------------

/// The first `program` on the user's `PATH` (the login shell's, not the
/// one an app opened from Finder inherits — see `user_path`).
pub(crate) fn find_on_path(program: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_string()
    };
    juggler_adapter::user_path::find(&exe)
}

/// The user's home dir (`HOME` / `USERPROFILE`).
/// The un-namespaced Keychain service Claude Code uses when
/// `CLAUDE_CONFIG_DIR` is unset (the vendor's default folder).
#[cfg(target_os = "macos")]
const SHARED_SERVICE: &str = "Claude Code-credentials";

/// ADR-0018: true for the vendor's own default folder, `~/.claude`.
/// Only an in-place account can have it (copies live in AI Juggler's
/// data), and it is launched as the vendor runs it: with
/// `CLAUDE_CONFIG_DIR` unset, so its sign-in and state are the same.
#[must_use]
pub fn is_vendor_default_dir(dir: &Path) -> bool {
    home_dir().is_some_and(|home| is_vendor_default_dir_in(dir, &home))
}

/// [`is_vendor_default_dir`] for an explicit home folder (pure, testable).
/// The exact path only — not a link to it: a launch through a link sets
/// `CLAUDE_CONFIG_DIR` to the link, and that sign-in is namespaced.
#[must_use]
pub fn is_vendor_default_dir_in(dir: &Path, home: &Path) -> bool {
    let trimmed = PathBuf::from(dir.to_string_lossy().trim_end_matches('/'));
    trimmed == home.join(".claude")
}

/// Credentials Claude Code takes from the environment ahead of the
/// folder's own sign-in. Started from a terminal that exports one, AI
/// Juggler would otherwise run EVERY account on that one key.
pub const INHERITED_CREDENTIAL_KEYS: [&str; 3] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
];

/// Each account signs in as itself: remove [`INHERITED_CREDENTIAL_KEYS`]
/// from what the launch inherits (an account set up with a key keeps it
/// in its own settings, which this doesn't touch).
fn unset_inherited_credentials(env: &mut EnvMap) {
    for key in INHERITED_CREDENTIAL_KEYS {
        env.insert(
            key.to_owned(),
            juggler_adapter::launch::ENV_UNSET.to_owned(),
        );
    }
}

/// The `CLAUDE_CONFIG_DIR` value for a folder: its exact path (Claude's
/// Keychain name hashes that string), or unset for the default folder.
fn config_dir_value(dir: &Path) -> String {
    home_dir().map_or_else(
        || dir.to_string_lossy().into_owned(),
        |home| config_dir_value_in(dir, &home),
    )
}

/// The `CLAUDE_CONFIG_DIR` value for an explicit home folder (pure, testable).
#[must_use]
pub fn config_dir_value_in(dir: &Path, home: &Path) -> String {
    if is_vendor_default_dir_in(dir, home) {
        juggler_adapter::launch::ENV_UNSET.to_owned()
    } else {
        dir.to_string_lossy().into_owned()
    }
}

/// Claude Code's state file for a folder: inside it, or `~/.claude.json`
/// for the vendor's default folder (where Claude keeps it when
/// `CLAUDE_CONFIG_DIR` is unset).
#[must_use]
pub fn claude_state_file(dir: &Path) -> PathBuf {
    home_dir().map_or_else(
        || dir.join(".claude.json"),
        |home| claude_state_file_in(dir, &home),
    )
}

/// [`claude_state_file`] for an explicit home folder (pure, testable).
#[must_use]
pub fn claude_state_file_in(dir: &Path, home: &Path) -> PathBuf {
    // Unset, Claude keeps it at ~/.claude.json — prefer that over any
    // stale ~/.claude/.claude.json.
    if is_vendor_default_dir_in(dir, home) && home.join(".claude.json").exists() {
        return home.join(".claude.json");
    }
    dir.join(".claude.json")
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub(crate) fn home_relative(relative: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(relative))
}

pub(crate) fn existing(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.is_file())
}

pub(crate) fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::fs::DirBuilder;
        use std::os::unix::fs::DirBuilderExt;
        DirBuilder::new().recursive(true).mode(0o700).create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

pub(crate) fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// The macOS keychain service name Claude Code namespaces per config dir
/// (since v2.1.56): `Claude Code-credentials-{first 8 hex of sha256(path)}`.
#[cfg(target_os = "macos")]
fn namespaced_service(state_dir: &Path) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(state_dir.to_string_lossy().as_bytes());
    let hex = digest.iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    });
    format!("Claude Code-credentials-{}", &hex[..8])
}

/// Probes keychain entry existence via `security find-generic-password`
/// **without** extracting any secret (`-w` deliberately absent).
#[cfg(target_os = "macos")]
fn keychain_entry_exists(service: &str) -> bool {
    std::process::Command::new(juggler_adapter::system_tools::SECURITY)
        .args(["find-generic-password", "-s", service])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Writes the managed `settings.json` for a config dir from a profile's
/// settings overlay (model preference). Atomic, and confined to the managed
/// dir by construction.
///
/// # Errors
///
/// Returns [`AdapterError::AccountDir`] when the file cannot be written.
pub fn write_settings_overlay(
    state_dir: &Path,
    overlay: &juggler_core::model::SettingsOverlay,
) -> Result<(), AdapterError> {
    let Some(model) = &overlay.model else {
        return Ok(()); // nothing to overlay — no file written
    };
    let settings = serde_json::json!({ "model": model });
    let bytes = serde_json::to_vec_pretty(&settings).map_err(|error| AdapterError::AccountDir {
        path: state_dir.to_path_buf(),
        message: error.to_string(),
    })?;
    juggler_core::fs::atomic_write(&state_dir.join("settings.json"), &bytes).map_err(|error| {
        AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: error.to_string(),
        }
    })
}

/// AJ-E1001: Claude Code's sign-in metadata for a config dir (see
/// [`ClaudeCodeAdapter::probe_identity`]).
#[must_use]
pub fn probe_claude_identity(state_dir: &Path) -> juggler_adapter::IdentityProbe {
    use juggler_adapter::IdentityProbe;
    let Ok(raw) = std::fs::read(claude_state_file(state_dir)) else {
        return IdentityProbe::SignedOut;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&raw) else {
        return IdentityProbe::Unverifiable {
            reason: "Claude Code's state file couldn't be read".into(),
        };
    };
    let Some(oauth) = value.get("oauthAccount").filter(|v| v.is_object()) else {
        return IdentityProbe::SignedOut;
    };
    let text = |key: &str| {
        oauth
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    };
    let email = text("emailAddress");
    let Some(subject) = text("accountUuid").or_else(|| email.clone()) else {
        return IdentityProbe::SignedOut;
    };
    let mut label = email.as_deref().map_or_else(
        || "signed in".to_owned(),
        juggler_adapter::identity::mask_email,
    );
    if let Some(org) = text("organizationName") {
        label = format!("{label} · {org}");
    }
    IdentityProbe::SignedIn { subject, label }
}

/// Whether a process is a `claude` CLI session: the one rule for it,
/// shared by everything that looks at running processes (This Mac's
/// tools, live conversations). The binary is `claude`, wherever it is
/// installed (the desktop app runs its Code sessions from
/// `…/claude-code/<version>/…/claude.app/Contents/MacOS/claude`), or the
/// npm package run by node. Claude Code also runs its search helpers as its
/// own executable under their names (`ugrep`, `rg` as the first command
/// word): the same binary, but never a session (QA on #126: five left
/// behind by a finished session read as five tools). So the binary has to
/// run as `claude`: its first command word, or a path to it (which may
/// hold spaces). An unreadable command line leaves it to the name and the
/// executable.
#[must_use]
pub fn is_cli_process(name: &str, exe: Option<&Path>, command_line: &str) -> bool {
    if command_line.contains("@anthropic-ai/claude-code") {
        return true;
    }
    let binary = name == "claude"
        || exe.is_some_and(|exe| exe.file_name().is_some_and(|file| file == "claude"));
    let cmd = command_line.trim_start();
    let runs_as_claude = cmd.split_whitespace().next().is_none_or(|first| {
        Path::new(first)
            .file_name()
            .is_some_and(|file| file == "claude")
            || (cmd.starts_with('/') && (cmd.ends_with("/claude") || cmd.contains("/claude ")))
    });
    binary && runs_as_claude
}
