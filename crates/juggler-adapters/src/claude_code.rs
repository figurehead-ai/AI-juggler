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
        }
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
        match std::process::Command::new(&bin).arg("--version").output() {
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

    fn shell_command(&self) -> Option<&'static str> {
        Some("claude")
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
            account.state_dir.to_string_lossy().into_owned(),
        );
        let (args, needs_pty) = match mode {
            LaunchMode::Interactive => (Vec::new(), true),
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
                // receives the delta as its next user turn (E207).
                let mut args = vec!["--resume".into(), session_id.clone()];
                if let Some(delta) = delta_prompt {
                    args.push(delta.clone());
                }
                (args, true)
            }
        };
        Ok(LaunchSpec {
            program: bin,
            args,
            env,
            cwd: effective_cwd,
            needs_pty,
        })
    }

    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        ClaudeTranscriptWatcher::new(transcript_root)
            .map(|watcher| Box::new(watcher) as Box<dyn SessionWatcher>)
    }
}

/// Login state of a managed account, detected without reading credential
/// contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    /// Credentials are present and structurally usable.
    SignedIn,
    /// A credentials file exists but is EMPTY — an OAuth abandoned
    /// halfway in this dir ("Sign-in didn't finish").
    Partial,
    /// Credentials exist with a PAST expiry claim (the documented
    /// field is parsed; token contents are never read or logged).
    Expired,
    /// An SSO session marker exists without completed credentials —
    /// finish in the browser/IdP window, then return.
    SsoPending,
    /// An API-key account (settings metadata NAMES a key — the value
    /// is never read; subscription rings never apply).
    ApiKey,
    /// No credentials found for this config dir.
    LoggedOut,
    /// The probe itself failed (unreadable dir) — NEVER renders as
    /// [`LoginState::LoggedOut`] (AJ-E308 Req.1).
    Unknown,
}

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
        env.insert(
            ENV_CONFIG_DIR.to_string(),
            state_dir.to_string_lossy().into_owned(),
        );
        Ok(LaunchSpec {
            program: bin,
            args: Vec::new(),
            env,
            cwd: state_dir.to_path_buf(),
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
    create_private_dir(state_dir).map_err(|error| AdapterError::AccountDir {
        path: state_dir.to_path_buf(),
        message: error.to_string(),
    })?;
    // Marker identifies the directory as ours — guardrail primitive for
    // "never touch unmanaged dirs" checks (ADR-0003).
    let marker = state_dir.join(MANAGED_MARKER);
    std::fs::write(&marker, "managed-by: ai-juggler\nharness: claude-code\n")
        .and_then(|()| restrict_to_owner(&marker))
        .map_err(|error| AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: format!("cannot write marker: {error}"),
        })
}

/// notify-based watcher over the account's transcript tree
/// (`<config>/projects/…`). Filesystem events trigger a line-count diff per
/// `.jsonl` file; new files emit [`SessionEvent::Discovered`], growth emits
/// [`SessionEvent::MessageAppended`]. Content is never parsed here — the
/// transcript parser runs downstream on demand (spec §4.1).
struct ClaudeTranscriptWatcher {
    root: PathBuf,
    _watcher: notify::RecommendedWatcher,
    events: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    seen: BTreeMap<PathBuf, usize>,
    pending: std::collections::VecDeque<SessionEvent>,
}

impl ClaudeTranscriptWatcher {
    fn new(root: &Path) -> Result<Self, AdapterError> {
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
            _watcher: watcher,
            events,
            seen: BTreeMap::new(),
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
            let lines = std::fs::read(&path).map_or(0, |bytes| count_lines(&bytes));
            let known = self.seen.entry(path.clone()).or_insert(0);
            if *known == 0 && lines > 0 {
                self.pending.push_back(SessionEvent::Discovered {
                    native_id: file_stem(&path),
                    transcript_path: path.clone(),
                });
            } else if lines > *known {
                self.pending.push_back(SessionEvent::MessageAppended {
                    native_id: file_stem(&path),
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
fn collect_jsonl(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
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

fn count_lines(bytes: &[u8]) -> usize {
    bytes.split(|b| *b == b'\n').count().saturating_sub(1)
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// ---- helpers -------------------------------------------------------------

fn find_on_path(program: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exe = if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_string()
    };
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(&exe))
        .find(|candidate| candidate.is_file())
}

/// The user's home dir (`HOME` / `USERPROFILE`).
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn home_relative(relative: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(relative))
}

fn existing(candidates: &[&str]) -> Option<PathBuf> {
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
    std::process::Command::new("security")
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
