//! Muse Code adapter (AJ-E1608) — Meta's `muse` CLI (D-74).
//!
//! Built from the research (`docs/research/muse-code.md`, the MIT
//! `meta-models/muse-code-sdk` schema and transcripts, and the installed
//! 1.4.13's own `--help` and strings, read 2026-11-13); not yet run live
//! with a sign-in — that is AJ-E1612's lab check (D-107). Every version is
//! therefore "unverified".
//!
//! **Discovery.** The installer's launcher `~/.local/bin/muse` (a bash
//! script that runs `muse-bin-<version>` beside it, and updates it in the
//! background once an hour), then `muse` on the PATH and Homebrew's
//! (`muse-code` cask). The version is the launcher's own record,
//! `.muse-version` beside it (`1.4.13-R6996.1`), else what
//! `muse --version` prints. Every run AI Juggler makes, probe or session,
//! sets `MUSE_NO_AUTO_UPDATE=1`: the launcher never updates from one of
//! them. A running Muse shows as `muse-bin-<version>` (the launcher `exec`s
//! it), so a session is told by that name.
//!
//! **Isolation per account.** Muse has no home variable of its own: its
//! settings and sign-in record live in `$XDG_CONFIG_HOME/muse`, its sessions
//! and index in `$XDG_DATA_HOME/muse` (`museHome` follows it, checked in a
//! throwaway home), and its session names in `~/Library/Application
//! Support/muse`, which only `HOME` moves. So an account's folder is a home
//! of its own: `HOME=<account>`, `XDG_CONFIG_HOME=<account>/.config`,
//! `XDG_DATA_HOME=<account>/.local/share`, all pinned after Launch-settings
//! overrides. Muse also reads the user's Claude Code and Codex skills
//! (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`): both point at empty folders inside
//! the account, so none of them loads in it. The user's own Muse (the
//! default `~/.config/muse`) runs with the XDG variables unset and `HOME`,
//! `CODEX_HOME` and `CLAUDE_CONFIG_DIR` as they are, as a plain `muse`
//! does. A moved `HOME` means Muse's shell tools in an account see that
//! folder's git identity and SSH keys, not the user's.
//!
//! **Sign-in, one per Mac.** Muse keeps its Meta sign-in in the login
//! Keychain as one generic password (service `ai.meta.dev.credentials`,
//! account `meta`: fixed, not per folder), with `auth.json` in the config
//! folder pointing at it. Two folders therefore share one subscription
//! sign-in, and `muse login` in a second would replace the first's. So:
//!
//! - the user's own Muse is the one subscription account (its sign-in is
//!   told by `auth.json`'s presence, or the Keychain item's, by name only:
//!   [`crate::sign_in`]); its plan usage is read ([`crate::muse_usage`]);
//! - any other account runs on a Meta Model API key, `META_API_KEY` in its
//!   own Launch settings, set for that account's launches only; an
//!   inherited `META_API_KEY` is removed from every launch. Without a key
//!   such an account doesn't start, and its sign-in is refused, each with
//!   the reason ([`ONE_SIGN_IN`]).
//!
//! An undocumented variable, `TBH_CREDENTIAL_BACKEND`, may move the sign-in
//! into the folder's own `auth.json`: the binary records a session's
//! credential backend as `file`, `keychain` or `keychain_fallback_file`
//! (1.4.13's strings), so `file` is the likely value. It is wired
//! ([`MuseAdapter::with_separate_sign_ins`]) and stays off ([`SEPARATE_SIGN_INS`])
//! until the lab shows two folders keep two sign-ins (D-107).
//!
//! **Sessions** come from Muse's own index, `session-index.db`, read-only
//! with a format check ([`crate::muse_sessions`]); a session AI Juggler
//! drives runs over `muse serve` ([`crate::muse_serve`]). **Usage** is the
//! plan's 5-hour and weekly windows Muse reports ([`crate::muse_usage`]).
//!
//! Not offered: a headless run (`muse exec` has no approvals AI Juggler
//! could show, and skipping them is never done); a reasoning effort from
//! Launch settings (AI Juggler has no such setting for any tool yet: each
//! account keeps Muse's own `reasoning_effort` setting).

use crate::common::prelude::*;

use juggler_adapter::inventory::{Applied, Item, SetUp};

use crate::claude_code::{existing, find_on_path};

/// Provider id: `muse-code` (Meta's Muse Code; the bare word is a
/// consumer app's name too).
pub const MUSE_ID: &str = "muse-code";

/// No version has been run live yet (AJ-E1612): every one is unverified.
const VERIFIED_AGAINST: &[&str] = &[];

/// The version whose help, strings and protocol this adapter was built
/// from.
pub const SOURCE_READ: &[&str] = &["1.4.13"];

/// The home: only it moves `~/Library/Application Support/muse`.
pub const ENV_HOME: &str = "HOME";
/// The config root (`<it>/muse`: settings, the sign-in record, skills).
pub const ENV_CONFIG: &str = "XDG_CONFIG_HOME";
/// The data root (`<it>/muse`: sessions, their index, `museHome`).
pub const ENV_DATA: &str = "XDG_DATA_HOME";
/// Where Muse looks for Codex's skills.
pub const ENV_CODEX: &str = "CODEX_HOME";
/// Where Muse looks for Claude Code's skills.
pub const ENV_CLAUDE: &str = "CLAUDE_CONFIG_DIR";
/// Keeps the launcher from updating Muse during a run.
pub const ENV_NO_UPDATE: &str = "MUSE_NO_AUTO_UPDATE";
/// Muse's undocumented credential backend (see the module docs).
pub const ENV_CREDENTIAL_BACKEND: &str = "TBH_CREDENTIAL_BACKEND";
/// A Meta Model API key: an account's own, from its Launch settings.
pub const ENV_API_KEY: &str = "META_API_KEY";
/// The launcher's own sign-in file for downloads (would point every
/// account at one).
const ENV_LAUNCHER_AUTH: &str = "MUSE_AUTH_PATH";
/// Where Muse keeps project state (`~/.muse/projects` by default: it
/// follows `HOME`, unless an inherited value points every account at one).
const ENV_PROJECTS: &str = "MUSE_PROJECTS_HOME";
/// Optional test/dev override for the binary path.
const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_MUSE_BIN";

/// The credential backend that keeps the sign-in in the folder's own
/// `auth.json` (the likely value; UNCONFIRMED until the lab, D-107).
pub const FILE_BACKEND: &str = "file";

/// Whether accounts other than the user's own Muse may hold a subscription
/// sign-in of their own (through [`FILE_BACKEND`]). Off until AJ-E1612's
/// lab check shows two folders keep two sign-ins (D-107).
pub const SEPARATE_SIGN_INS: bool = false;

/// The Keychain-backed sign-in is one per Mac: why a second account can't
/// have one.
pub const ONE_SIGN_IN: &str = "Muse keeps its Meta sign-in in this computer's system keychain under one fixed name, \
     so only your own Muse can be signed in with your subscription: a second sign-in would replace it. \
     Give this account a Meta Model API key instead (META_API_KEY in its Launch settings)";

const PINNED: &[&str] = &[ENV_HOME, ENV_CONFIG, ENV_DATA, ENV_CODEX, ENV_CLAUDE];
const ISOLATION: &[&str] = &[
    ENV_HOME,
    ENV_CONFIG,
    ENV_DATA,
    ENV_CODEX,
    ENV_CLAUDE,
    ENV_NO_UPDATE,
    ENV_CREDENTIAL_BACKEND,
    ENV_LAUNCHER_AUTH,
    ENV_PROJECTS,
    ENV_BIN_OVERRIDE,
];
/// Removed from every launch (see the module docs).
const UNSET: &[&str] = &[ENV_CREDENTIAL_BACKEND, ENV_LAUNCHER_AUTH, ENV_PROJECTS];
/// The same, while separate sign-ins are on (the backend is then set).
const UNSET_SEPARATE: &[&str] = &[ENV_LAUNCHER_AUTH, ENV_PROJECTS];
/// Set on every run.
pub const FIXED: &[(&str, &str)] = &[(ENV_NO_UPDATE, "1")];

/// The launcher's record of the active version, beside it.
const VERSION_FILE: &str = ".muse-version";
/// The folder an account keeps Muse's config root in.
const CONFIG_ROOT: &str = ".config";
/// The folder an account keeps Muse's data root in.
const DATA_ROOT: &str = ".local/share";
/// Muse's folder inside either root.
const MUSE_DIR: &str = "muse";
/// The empty folders Muse is pointed at for Codex's and Claude's skills.
const NO_CODEX: &str = ".ai-juggler/no-codex";
const NO_CLAUDE: &str = ".ai-juggler/no-claude";

/// `muse --version` prints `Muse Code 1.4.13 (1.4.13-R6996.1)`: the first
/// word that starts with a digit.
#[must_use]
pub fn parse_version(stdout: &str) -> Option<String> {
    crate::common::probe::first_version_word(stdout)
}

/// The launcher's `.muse-version` (`1.4.13-R6996.1`): the version before
/// its build (`1.4.13`), or `None` when it doesn't read as one.
#[must_use]
pub fn parse_version_file(text: &str) -> Option<String> {
    let line = text.lines().next()?.trim();
    let (version, build) = line.split_once("-R")?;
    let numeric = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
    let parts: Vec<&str> = version.split('.').collect();
    (parts.len() == 3 && parts.iter().all(|part| numeric(part)) && !build.is_empty())
        .then(|| version.to_owned())
}

/// Muse's config folder for an account's folder: the folder itself when
/// it is one (`~/.config/muse`, the user's own), else
/// `<account>/.config/muse`.
#[must_use]
pub fn config_of(state_dir: &Path) -> PathBuf {
    if is_config_folder(state_dir) {
        state_dir.to_path_buf()
    } else {
        state_dir.join(CONFIG_ROOT).join(MUSE_DIR)
    }
}

/// Muse's data folder for an account's folder: `~/.local/share/muse` for
/// the user's own (`~/.config/muse`), else `<account>/.local/share/muse`.
#[must_use]
pub fn data_of(state_dir: &Path) -> PathBuf {
    if is_config_folder(state_dir) {
        let home = state_dir
            .parent()
            .and_then(Path::parent)
            .unwrap_or(state_dir);
        home.join(DATA_ROOT).join(MUSE_DIR)
    } else {
        state_dir.join(DATA_ROOT).join(MUSE_DIR)
    }
}

/// Whether `dir` is a Muse config folder itself (`…/.config/muse`).
fn is_config_folder(dir: &Path) -> bool {
    let text = dir.to_string_lossy();
    let dir = Path::new(text.trim_end_matches('/'));
    dir.file_name().is_some_and(|name| name == MUSE_DIR)
        && dir
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == CONFIG_ROOT)
}

/// Adapter for Muse Code.
#[derive(Debug, Clone, Default)]
pub struct MuseAdapter {
    bin: Option<PathBuf>,
    /// The user's home (tests/fixtures; `None` reads `$HOME`).
    home: Option<PathBuf>,
    /// Separate subscription sign-ins per folder ([`SEPARATE_SIGN_INS`]).
    separate: bool,
}

impl MuseAdapter {
    /// Creates the adapter, resolving the binary at call time.
    #[must_use]
    pub fn new() -> Self {
        Self {
            separate: SEPARATE_SIGN_INS,
            ..Self::default()
        }
    }

    /// The adapter pinned to a specific binary (tests/fixtures).
    #[must_use]
    pub fn with_bin(bin: impl Into<PathBuf>) -> Self {
        Self {
            bin: Some(bin.into()),
            ..Self::new()
        }
    }

    /// The adapter using `home` as the user's home (tests/fixtures).
    #[must_use]
    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }

    /// The adapter with separate subscription sign-ins on (the lab, D-107;
    /// tests): every other account's sign-in kept in its own `auth.json`
    /// through [`FILE_BACKEND`].
    #[must_use]
    pub fn with_separate_sign_ins(mut self, on: bool) -> Self {
        self.separate = on;
        self
    }

    fn user_home(&self) -> Option<PathBuf> {
        self.home.clone().or_else(home_dir)
    }

    /// The user's own Muse config folder, `~/.config/muse`.
    fn default_config(&self) -> Option<PathBuf> {
        self.user_home()
            .map(|home| home.join(CONFIG_ROOT).join(MUSE_DIR))
    }

    /// Whether `dir` is the user's own Muse, by its exact path.
    pub(crate) fn is_default(&self, dir: &Path) -> bool {
        self.default_config().is_some_and(|default| {
            Path::new(dir.to_string_lossy().trim_end_matches('/')) == default
        })
    }

    /// Whether the folder can hold a subscription sign-in of its own: the
    /// user's own Muse, or any while separate sign-ins are on.
    pub(crate) fn own_sign_in(&self, state_dir: &Path) -> bool {
        self.separate || self.is_default(state_dir)
    }

    /// The installer's launcher first, then the PATH and Homebrew's.
    fn resolve_bin(&self) -> Option<PathBuf> {
        crate::common::probe::pinned_bin(self.bin.as_ref(), ENV_BIN_OVERRIDE).or_else(|| {
            self.user_home()
                .map(|home| home.join(".local/bin/muse"))
                .filter(|path| path.is_file())
                .or_else(|| find_on_path("muse"))
                .or_else(|| existing(&["/opt/homebrew/bin/muse", "/usr/local/bin/muse"]))
        })
    }

    /// The binary, or why a run can't start.
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] when Muse isn't found.
    pub fn require_bin(&self) -> Result<PathBuf, AdapterError> {
        self.resolve_bin().ok_or_else(|| {
            AdapterError::InvalidLaunch("muse not found; run discovery for details".into())
        })
    }

    /// The installed version: the launcher's own record beside it, else
    /// `muse --version` (with updates off).
    fn version(bin: &Path) -> Result<String, InstallStatus> {
        let recorded = bin
            .parent()
            .map(|dir| dir.join(VERSION_FILE))
            .and_then(|file| {
                let small =
                    std::fs::metadata(&file).is_ok_and(|meta| meta.is_file() && meta.len() < 256);
                small
                    .then(|| std::fs::read_to_string(&file).ok())
                    .flatten()
                    .and_then(|text| parse_version_file(&text))
            });
        match recorded {
            Some(version) => Ok(version),
            None => {
                crate::common::probe::version_with_env(bin, ["--version"], FIXED, parse_version)
            }
        }
    }

    /// Refuses a launch of an account that would run on the Mac's one
    /// subscription sign-in without being the user's own Muse ([`ONE_SIGN_IN`]).
    fn refuse_shared_sign_in(
        &self,
        state_dir: &Path,
        profile: &Profile,
    ) -> Result<(), AdapterError> {
        if self.own_sign_in(state_dir) || api_key_of(profile).is_some() {
            Ok(())
        } else {
            Err(AdapterError::Unsupported(ONE_SIGN_IN.into()))
        }
    }

    /// What a run's sign-in takes beyond isolation: the account's own key
    /// (an inherited one removed), and, while separate sign-ins are on, the
    /// file backend for every folder but the user's own.
    fn sign_in_env(&self, state_dir: &Path, key: Option<&str>) -> EnvMap {
        let unset = || juggler_adapter::launch::ENV_UNSET.to_owned();
        let mut env = EnvMap::new();
        env.insert(
            ENV_API_KEY.to_owned(),
            key.map_or_else(unset, str::to_owned),
        );
        if self.separate {
            let backend = if self.is_default(state_dir) {
                unset()
            } else {
                FILE_BACKEND.to_owned()
            };
            env.insert(ENV_CREDENTIAL_BACKEND.to_owned(), backend);
        }
        env
    }

    /// One run's environment: Launch-settings overrides first (never an
    /// isolation variable), isolation LAST (AJ-E309), then the sign-in.
    pub(crate) fn run_env(&self, state_dir: &Path, profile: &Profile) -> EnvMap {
        let mut env: EnvMap = profile.settings_overlay.env.clone();
        env.retain(|key, _| !ISOLATION.contains(&key.as_str()));
        env.extend(self.isolation_env(state_dir));
        env.extend(self.sign_in_env(state_dir, api_key_of(profile)));
        env
    }

    /// The account's own folders Muse is pointed at, made owner-only.
    fn make_folders(state_dir: &Path) -> Result<(), AdapterError> {
        crate::common::home::make_private_dirs([
            config_of(state_dir),
            data_of(state_dir),
            state_dir.join(NO_CODEX),
            state_dir.join(NO_CLAUDE),
        ])
    }
}

/// The account's own Meta Model API key, when its Launch settings set one
/// (passed to its launch only; never read for anything else).
fn api_key_of(profile: &Profile) -> Option<&str> {
    profile
        .settings_overlay
        .env
        .get(ENV_API_KEY)
        .map(String::as_str)
        .filter(|key| !key.is_empty())
}

/// `muse` commands that aren't a session (`muse --help`, 1.4.13): its
/// server, sign-in and upkeep.
const HELPERS: &[&str] = &[
    "serve",
    "config",
    "export",
    "trace",
    "model-profile",
    "skills",
    "voice",
    "plugins",
    "sandbox",
    "schema",
    "session-message",
    "mcp",
    "auth",
    "login",
    "logout",
    "usage",
    "init",
];
/// Options that print and exit.
const HELPER_FLAGS: &[&str] = &["-h", "--help", "-V", "--version"];
/// Options that take the next word as their value.
const VALUE_OPTIONS: &[&str] = &[
    "--agents",
    "--provider",
    "--preset",
    "--model",
    "--contributor-mode",
    "-c",
    "--config",
    "--reasoning-effort",
    "--effort",
    "--base-url",
    "--image",
    "--workspace",
    "--worktree-base",
    "--worktree-existing",
    "--approval-mode",
    "--permission-profile",
    "--approval-judge",
];

/// Whether a program is Muse's: the binary the launcher runs
/// (`muse-bin-<version>`), Homebrew's, or the launcher itself.
fn is_muse(name: &str) -> bool {
    name == "muse" || name.starts_with("muse-bin-")
}

/// A running Muse (`muse-bin-<version> …`, or the launcher before it
/// `exec`s) is a session unless it runs a helper (`serve`, `login`,
/// `usage`, …). `resume <id>` names its conversation.
#[must_use]
pub fn session_process(
    exe: Option<&Path>,
    args: &[String],
) -> Option<juggler_adapter::conversation::SessionProcess> {
    let name_of = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    };
    let named = exe
        .and_then(name_of)
        .or_else(|| args.first().and_then(|first| name_of(Path::new(first))))?;
    if !is_muse(&named) {
        return None;
    }
    let words = args.get(1..).unwrap_or_default();
    let command =
        crate::common::launch::session_command(words, VALUE_OPTIONS, HELPER_FLAGS, HELPERS)?;
    let resumes = match command {
        crate::common::launch::Command::Word(at, "resume") => words[at + 1..]
            .iter()
            .find(|next| !next.starts_with('-'))
            .cloned(),
        _ => None,
    };
    Some(juggler_adapter::conversation::SessionProcess { resumes })
}

/// Sign-in state of an account's folder, from which files are there and
/// the Keychain item's name, never a secret: the user's own Muse is signed
/// in when its `auth.json` is there (or, without it and when `keychain`
/// says this Mac's Keychain is its own, the Keychain item);
/// another folder, while sign-ins are one per Mac, runs on its Launch
/// settings' key, which isn't read here (`Unknown`).
#[must_use]
pub fn muse_login_state(state_dir: &Path, own_sign_in: bool, keychain: bool) -> LoginState {
    if !own_sign_in {
        return LoginState::Unknown;
    }
    match crate::sign_in::muse_signed_in(state_dir, &config_of(state_dir)) {
        None => LoginState::Unknown,
        Some(true) => LoginState::SignedIn,
        Some(false) if keychain && crate::sign_in::muse_keychain_item() => LoginState::SignedIn,
        Some(false) => LoginState::LoggedOut,
    }
}

impl ToolIdentity for MuseAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(MUSE_ID)
    }
    fn spec(&self) -> juggler_adapter::ToolSpec {
        muse_spec()
    }
    /// Sessions from its index, driven over `muse serve`; the plan's
    /// windows as Muse reports them (`usage/read`, `usage/refresh`).
    fn capabilities(&self) -> Capabilities {
        Capabilities::RESUME
            | Capabilities::TRANSCRIPT_WATCH
            | Capabilities::PROTOCOL_SERVER
            | Capabilities::USAGE_QUOTA_FEED
            | Capabilities::LIMIT_WINDOWS_REPORTED
    }
    fn verified_against(&self) -> &'static [&'static str] {
        VERIFIED_AGAINST
    }
}

impl ToolDiscovery for MuseAdapter {
    /// Files only: never runs it (`discover` asks its version).
    fn locate(&self) -> Option<PathBuf> {
        self.resolve_bin().filter(|bin| bin.is_file())
    }
    fn discover(&self) -> DiscoveryReport {
        crate::common::probe::discover_bin(
            self.id(),
            self.resolve_bin(),
            "looked in ~/.local/bin, on PATH, /opt/homebrew/bin, /usr/local/bin",
            Self::version,
            "Muse Code — built from 1.4.13's help, strings and protocol; not yet checked live",
        )
    }
    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        crate::common::home::scan_home(self.user_home(), ENV_CONFIG, |home, env| {
            crate::muse_environment::scan_muse_environment(home, env, &scan_probe)
        })
    }
    fn vendor_default_dir(&self) -> Option<PathBuf> {
        self.default_config()
    }
    fn shell_command(&self) -> Option<&'static str> {
        Some("muse")
    }
    fn resolved_binary(&self) -> Option<PathBuf> {
        self.resolve_bin()
    }
}

/// Whether a folder found by the scan is signed in (the user's own Muse:
/// its record's presence).
fn scan_probe(dir: &Path) -> LoginProbe {
    muse_login_state(dir, true, false).probe()
}

impl ToolSignIn for MuseAdapter {
    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        prepare_managed_dir_for(state_dir, MUSE_ID)?;
        Self::make_folders(state_dir)?;
        Ok(self.pinned_set_env(state_dir))
    }
    fn login_state(&self, state_dir: &Path) -> LoginState {
        // A home given to the adapter (a fixture, a lab) has no Keychain
        // item of this Mac's: only the real user's own Muse is probed.
        muse_login_state(
            state_dir,
            self.own_sign_in(state_dir),
            self.home.is_none() && self.is_default(state_dir),
        )
    }
    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        crate::muse_environment::inspect_muse_source(source, &scan_probe)
    }
    /// The user's own `~/.config/muse` only: Muse's sign-in is one per
    /// Mac, so no copy is offered (a copy would carry none of it).
    fn can_use_in_place(&self) -> bool {
        true
    }
    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::muse_environment::adopt_muse_config(
            source,
            self.default_config().as_deref(),
            &scan_probe,
        )
    }
}

impl ToolLaunch for MuseAdapter {
    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        let bin = self.require_bin()?;
        if let LaunchMode::Resume { session_id, .. } = mode {
            crate::codex::refuse_option_like(session_id)?;
        }
        self.refuse_shared_sign_in(&account.state_dir, profile)?;
        let mut args: Vec<String> = Vec::new();
        if let Some(model) = &profile.settings_overlay.model {
            args.extend(["--model".into(), model.clone()]);
        }
        match mode {
            LaunchMode::Interactive => {}
            LaunchMode::Resume {
                session_id,
                delta_prompt,
            } => {
                if delta_prompt.is_some() {
                    return Err(AdapterError::Unsupported(
                        "Muse Code can't be given a message as it resumes".into(),
                    ));
                }
                args.extend(["resume".into(), session_id.clone()]);
            }
            // `muse [OPTIONS] [PROMPT]`: the first message, after `--` so
            // one that starts with `-` is never read as an option.
            LaunchMode::Primed { prompt, .. } => {
                args.extend(["--".into(), prompt.clone()]);
            }
            LaunchMode::Headless { .. } => {
                return Err(AdapterError::Unsupported(
                    "Muse Code runs in its terminal or in AI Juggler, never headless: its approvals would have no one to ask".into(),
                ));
            }
        }
        Ok(LaunchSpec {
            program: bin,
            args,
            env: self.run_env(&account.state_dir, profile),
            cwd: crate::common::launch::launch_cwd(profile, cwd, mode),
            needs_pty: true,
        })
    }
    /// Muse's own `muse login` (a code approved in the browser), for the
    /// user's own Muse only while sign-ins are one per Mac ([`ONE_SIGN_IN`]).
    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        if !self.own_sign_in(state_dir) {
            return Err(AdapterError::Unsupported(ONE_SIGN_IN.into()));
        }
        let mut env = self.isolation_env(state_dir);
        env.extend(self.sign_in_env(state_dir, None));
        Ok(LaunchSpec {
            program: self.require_bin()?,
            args: vec!["login".into()],
            env,
            cwd: state_dir.to_path_buf(),
            needs_pty: true,
        })
    }
    /// The backend variable is set while separate sign-ins are on.
    fn launch_unset_keys(&self) -> &'static [&'static str] {
        if self.separate { UNSET_SEPARATE } else { UNSET }
    }
    fn isolation_env_keys(&self) -> &'static [&'static str] {
        ISOLATION
    }
    fn launch_fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        FIXED
    }
    fn pinned_env_keys(&self) -> &'static [&'static str] {
        PINNED
    }
    /// An account's folder is a home of its own; the user's own Muse runs
    /// with the XDG variables unset, and `HOME`, `CODEX_HOME` and
    /// `CLAUDE_CONFIG_DIR` as they are.
    fn pinned_env(&self, state_dir: &Path) -> Vec<(&'static str, Option<String>)> {
        if self.is_default(state_dir) {
            return vec![(ENV_CONFIG, None), (ENV_DATA, None)];
        }
        let text = |path: PathBuf| Some(path.to_string_lossy().into_owned());
        vec![
            (ENV_HOME, text(state_dir.to_path_buf())),
            (ENV_CONFIG, text(state_dir.join(CONFIG_ROOT))),
            (ENV_DATA, text(state_dir.join(DATA_ROOT))),
            (ENV_CODEX, text(state_dir.join(NO_CODEX))),
            (ENV_CLAUDE, text(state_dir.join(NO_CLAUDE))),
        ]
    }
    fn resume_args(&self, conversation_id: &str) -> Option<Vec<String>> {
        crate::codex::refuse_option_like(conversation_id).ok()?;
        Some(vec!["resume".into(), conversation_id.to_owned()])
    }
}

impl ToolConfig for MuseAdapter {}

/// Skills and MCP servers from the account's own config folder
/// ([`crate::muse_inventory`]).
impl ToolInventory for MuseAdapter {
    fn inventory(&self, state_dir: &Path, _project: Option<&Path>) -> Vec<Item> {
        crate::muse_inventory::inventory(&config_of(state_dir))
    }
    fn can_set_up(&self, item: &Item) -> SetUp {
        crate::muse_inventory::can_set_up(item)
    }
    /// Into the account's own config folder.
    fn set_up(
        &self,
        state_dir: &Path,
        _: Option<&Path>,
        item: &Item,
    ) -> Result<Applied, AdapterError> {
        crate::muse_inventory::set_up(&config_of(state_dir), item)
    }
}

impl ToolTranscripts for MuseAdapter {
    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        Ok(Box::new(crate::common::sqlite::StoreWatcher::new(
            crate::muse_sessions::index_of(&data_of(transcript_root)),
            crate::muse_sessions::session_changes,
        )))
    }
    /// The session that ran in `cwd`, from Muse's index (read-only); its
    /// link is `<index>#<id>`.
    fn locate_transcript(
        &self,
        state_dir: &Path,
        cwd: &Path,
        since_ms: i64,
        taken: &[PathBuf],
    ) -> Option<juggler_core::model::TranscriptRef> {
        let index = crate::muse_sessions::index_of(&data_of(state_dir));
        crate::muse_sessions::locate(&index, cwd, since_ms, taken).map(|(path, id)| {
            juggler_core::model::TranscriptRef {
                path,
                native_id: Some(id),
            }
        })
    }
    fn stored_conversations(
        &self,
        state_dir: &Path,
    ) -> Vec<juggler_adapter::conversation::StoredConversation> {
        crate::muse_sessions::stored_conversations(&crate::muse_sessions::index_of(&data_of(
            state_dir,
        )))
    }
    fn subagent_support(&self) -> juggler_adapter::SubagentSupport {
        juggler_adapter::SubagentSupport::NotSupported(
            "Muse Code keeps a sub-agent as a session of its own beside its parent's; AI Juggler doesn't list them yet.",
        )
    }
}

impl ToolQuota for MuseAdapter {
    /// The user's own Muse while it is signed in (only a subscription has
    /// windows; an account on a key has none). Reads which files are there.
    fn plan_usage_offered(&self, state_dir: &Path) -> bool {
        self.own_sign_in(state_dir) && self.login_state(state_dir) == LoginState::SignedIn
    }
    /// Muse's own report of the plan's windows ([`crate::muse_usage`]).
    fn plan_usage(
        &self,
        account: &Account,
    ) -> Option<Result<juggler_adapter::plan_usage::PlanReading, String>> {
        self.plan_usage_offered(&account.state_dir).then(|| {
            let bin = self.require_bin().map_err(|error| error.to_string())?;
            let env = self.isolation_env(&account.state_dir);
            crate::muse_usage::usage(&bin, &account.state_dir, &env)
        })
    }
}

impl ToolLive for MuseAdapter {
    fn session_process(
        &self,
        exe: Option<&Path>,
        args: &[String],
    ) -> Option<juggler_adapter::conversation::SessionProcess> {
        session_process(exe, args)
    }
}

impl ToolHandover for MuseAdapter {
    fn switch_capabilities(&self) -> juggler_adapter::switching::SwitchCapabilities {
        juggler_adapter::switching::fresh_start_only
    }
}

/// Its own `muse serve` ([`crate::muse_serve`]).
impl ToolRuntime for MuseAdapter {
    fn driven(&self) -> Option<&dyn juggler_adapter::runtime::DrivenRuntime> {
        Some(self)
    }
}

/// ADR-0019: Muse Code, described as data.
#[must_use]
pub fn muse_spec() -> juggler_adapter::ToolSpec {
    use juggler_adapter::{SetupGroup, SignInWords, ToolFlags, ToolKind, ToolSpec, ToolTerms};
    ToolSpec {
        id: MUSE_ID.to_owned(),
        display_name: "Muse Code".to_owned(),
        short_name: "Muse".to_owned(),
        vendor: "Meta".to_owned(),
        // Not yet checked against its owner's guidelines (brand-marks.md):
        // the letter mark until then.
        mark: Some("muse".to_owned()),
        vendor_mark: None,
        install_url: Some("https://dev.meta.ai/docs/muse-code/".to_owned()),
        install_command: Some("curl -fsSL https://dev.meta.ai/install.sh | sh".to_owned()),
        sign_in: SignInWords {
            own: "Muse Code's own sign-in".to_owned(),
            short: "Muse Code's".to_owned(),
        },
        same_as: "as when you run Muse Code yourself".to_owned(),
        // No Meta desktop app for Muse Code was found (AJ-E1610's pass).
        desktop_app: None,
        app_meter: false,
        kinds: vec![ToolKind::Cli],
        rules_files: vec!["AGENTS.md".to_owned()],
        rules_import: None,
        scan_hint: "Looks for your own Muse Code setup. Nothing is changed.".to_owned(),
        setup_groups: vec![SetupGroup {
            id: MUSE_ID.to_owned(),
            title: "Muse Code".to_owned(),
            path_contains: None,
            origin: None,
            shows_install: true,
        }],
        default_label: Some("Muse Code".to_owned()),
        flags: ToolFlags {
            first_run: true,
            primary: false,
        },
        // Meta Model API Terms of Service (updated 2026-10-09), read
        // 2026-11-13, recorded in AJ-E1518's register.
        terms: Some(ToolTerms {
            name: "Meta Model API Terms of Service".to_owned(),
            url: "https://dev.meta.ai/legal/terms-of-service".to_owned(),
            accounts: "Meta's Model API terms say a subscription's credential is for use only with its coding \
                       harness, an API key is never shared, and rate limits and usage quotas are never evaded."
                .to_owned(),
            checked: "2026-11-13".to_owned(),
        }),
        // Muse updates itself through its launcher (no public package
        // list); its launcher updates at once with MUSE_SYNC_UPDATE=1.
        release_feed: None,
        update_command: Some("MUSE_SYNC_UPDATE=1 muse --version".to_owned()),
        floor: juggler_adapter::FloorFamilies::all("neutral"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_read_from_the_launchers_record_or_its_output() {
        assert_eq!(
            parse_version_file("1.4.13-R6996.1\n").as_deref(),
            Some("1.4.13")
        );
        assert_eq!(parse_version_file("1.4.13").as_deref(), None);
        assert_eq!(parse_version_file("garbage-Rx").as_deref(), None);
        assert_eq!(
            parse_version("Muse Code 1.4.13 (1.4.13-R6996.1)").as_deref(),
            Some("1.4.13")
        );
    }

    #[test]
    fn a_folder_maps_to_its_config_and_data_roots() {
        let own = Path::new("/u/.config/muse");
        assert_eq!(config_of(own), own);
        assert_eq!(data_of(own), Path::new("/u/.local/share/muse"));
        let account = Path::new("/accounts/a");
        assert_eq!(config_of(account), Path::new("/accounts/a/.config/muse"));
        assert_eq!(data_of(account), Path::new("/accounts/a/.local/share/muse"));
    }

    #[test]
    fn a_running_muse_is_told_by_its_binary_and_command() {
        let args = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
        let bin = Path::new("/u/.local/bin/muse-bin-1.4.13-R6996.1");
        assert!(session_process(Some(bin), &args(&["muse-bin", "--model", "m"])).is_some());
        assert_eq!(
            session_process(Some(bin), &args(&["muse-bin", "resume", "0198-id"]))
                .and_then(|found| found.resumes)
                .as_deref(),
            Some("0198-id")
        );
        for helper in [
            &["muse-bin", "serve"][..],
            &["muse-bin", "usage", "--json"],
            &["muse-bin", "--version"],
        ] {
            assert!(
                session_process(Some(bin), &args(helper)).is_none(),
                "{helper:?}"
            );
        }
        assert!(session_process(Some(Path::new("/bin/museum")), &args(&["museum"])).is_none());
    }
}
