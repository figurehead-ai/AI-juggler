//! `ZCode` adapter (AJ-E1605) — Z.ai's `ZCode`, through the engine inside
//! its app.
//!
//! Built from the installed app (desktop 3.15.3, engine 0.16.9; research
//! `docs/research/zcode.md` § C) and the engine's own code; not yet run
//! live with a sign-in — that is the maintainer's check on a spare account
//! (AJ-E1612). Every version is therefore "unverified".
//!
//! Discovery (Req.1): the `ZCode` app (`ZCode.app`, bundle id
//! `dev.zcode.app`) where apps are installed (macOS: `/Applications`, then
//! `~/Applications`; `juggler_adapter::apps`), found by its name and by its
//! engine in its own files: a bundle that holds `ZCode`'s engine where
//! `ZCode` keeps it is taken as `ZCode`'s (the bundle id isn't read). The
//! engine is `Contents/Resources/glm/zcode.cjs`, and it runs on the app's
//! own runtime in Node.js mode, as the desktop app runs it
//! (`ELECTRON_RUN_AS_NODE=1 <app>/Contents/MacOS/ZCode <engine> …`; round 3,
//! checked live 2026-11-13): no Node.js of the user's is needed. Its
//! version is what `version` prints under that runtime.
//! `AI_JUGGLER_ZCODE_ENGINE` names another engine file (tests, dev); the
//! runtime and the provider settings are then the ones of the app bundle
//! it lies in.
//!
//! Isolation without moving `HOME` (Req.2; a moved `HOME` would take the
//! user's git identity and SSH keys away from `ZCode`'s shell tools). An
//! App's tree is `<account>/.zcode`, and each launch pins
//!
//! - `ZCODE_STORAGE_DIR` = the tree (the engine's `storage.dir`: plugins
//!   and its per-session working files);
//! - `ZCODE_DATA_BASE_DIR` = the account folder (the sign-in is then
//!   `<account>/.zcode/v2/credentials.json`);
//! - `ZCODE_SESSION_DB_PATH` = `<tree>/cli/db/db.sqlite` (the engine's
//!   `storage.sessionDbPath`, whose default `~/.zcode/cli/db/db.sqlite`
//!   follows the home folder, not `ZCODE_STORAGE_DIR`: without it every
//!   App's sessions and usage would land in the user's own store).
//!
//! All three are pinned after Launch-settings overrides, and so is
//! `ZCODE_BUILTIN_PROVIDER_CONFIG_FILE`: the app's own built-in provider
//! settings (`Contents/Resources/config/provider/zcode-builtin.json`),
//! without which the engine refuses to start a session (it looks for them
//! beside itself, where the app doesn't keep them). The engine then copies
//! them into the App's tree (`v2/runtime/provider/…`), in a folder named
//! after its own build constant (`0.0.0-dev` in this build: no variable
//! sets it; `ZCODE_APP_VERSION` only labels the engine's telemetry, so AI
//! Juggler leaves it unset rather than pass for the desktop app).
//! `ZCODE_SESSION_DB` (another name for the store's path) and an inherited
//! `ZCODE_PERSONAL_PROVIDER_CONFIG_FILE` (would point every App at one
//! provider file; unset, it is the App's own `v2/provider_config.json`) are
//! removed from every launch. `ZCode`'s own `~/.zcode` runs with the three
//! tree variables unset, as the desktop app does. These variables were
//! found in the engine's own code (0.16.9), not in Z.ai's docs, so a launch
//! check ([`isolation_took`]) confirms the engine wrote inside the App's
//! tree, and the app can refuse an App where it didn't. What the engine
//! reads from the home folder stays shared by every App, whatever these
//! say: its user settings `~/.zcode/cli/config.json` (MCP servers, hooks),
//! its rules `~/.zcode/AGENTS.md` and its skills `~/.zcode/skills/`
//! (`os.homedir()` or `HOME` in its own code; no variable moves them short
//! of `HOME`, research round-2 addendum). Rules, skills and servers are
//! therefore one set for every App ([`crate::zcode_inventory`]).
//!
//! Bringing a tree in (Req.5, ADR-0018): `ZCode`'s own `~/.zcode` where it
//! is, or a copy into the App's tree that never takes its sign-in, store,
//! logs or model-call records ([`crate::zcode_environment`]).
//!
//! Sessions (Req.3) come from the tree's store, read-only
//! ([`crate::zcode_sessions`]). A running engine is recognised by its
//! command line.
//!
//! **No terminal (round 3).** The app ships no terminal UI (`Cannot find
//! package '@zcode/tui'`, checked live): its engine runs only as the
//! desktop app runs it, `app-server --stdio`. So AI Juggler runs `ZCode`
//! sessions driven over that protocol ([`crate::zcode_appserver`]), and
//! every terminal launch refuses with the reason ([`NO_TERMINAL`]).
//!
//! Usage (Req.4): Computed credits from the store's `model_usage`, per
//! session and per App; its limits are the plan's caps the user enters
//! (`LIMITS_FROM_PLAN_CAPS`): the engine records no pool (it never asks
//! Z.ai for one; the desktop app does). A call refused at the plan's limit
//! is a limit hit, with its window and reset ([`crate::zcode_limits`]).
//!
//! Packs (Req.6): a session's chat is read from the store for a pack
//! ([`crate::zcode_sessions::turns`]); a pack comes into `ZCode` through its
//! own import (`session/create` with `importedHistory`, no model call;
//! [`crate::zcode_appserver`]).
//!
//! Not offered (each checked in the engine's code, 0.16.9, round-2
//! addendum): runs without a terminal or AI Juggler (`-p` skips `ZCode`'s
//! permission prompts by default, `--mode yolo`); a model from Launch
//! settings (the model is the App's own choice, kept in its tree's
//! `v2/provider_config.json`, so each App keeps its own); the agent tools.

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{DiscoveredInstall, ImportOutcome, LoginProbe};
use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::prelude::*;
use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, InstallStatus, LaunchMode, LaunchSpec,
    SessionWatcher, VerificationStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

use crate::claude_code::{LoginState, home_dir, prepare_managed_dir_for};

/// Provider id: `zcode`.
pub const ZCODE_ID: &str = "zcode";

/// No version has been run live yet (AJ-E1612): every one is unverified.
const VERIFIED_AGAINST: &[&str] = &[];

/// The app's name, as the system shows it.
const APP_NAME: &str = "ZCode";
/// The engine inside the app's own files.
const ENGINE_IN_APP: &str = "glm/zcode.cjs";
/// The engine's file name.
pub const ENGINE_FILE: &str = "zcode.cjs";
/// The name of a `ZCode` tree (its storage folder).
pub const TREE_NAME: &str = ".zcode";

/// The engine's storage folder.
pub const ENV_STORAGE_DIR: &str = "ZCODE_STORAGE_DIR";
/// The folder whose `.zcode/v2/` holds the sign-in.
pub const ENV_DATA_BASE_DIR: &str = "ZCODE_DATA_BASE_DIR";
/// The session store's path.
pub const ENV_SESSION_DB_PATH: &str = "ZCODE_SESSION_DB_PATH";
/// The engine's other name for the session store's path.
const ENV_SESSION_DB: &str = "ZCODE_SESSION_DB";
/// Would point every App at one provider settings file.
const ENV_PROVIDER_CONFIG: &str = "ZCODE_PERSONAL_PROVIDER_CONFIG_FILE";
/// Optional test/dev override for the engine's path.
const ENV_ENGINE_OVERRIDE: &str = "AI_JUGGLER_ZCODE_ENGINE";
/// The app's built-in provider settings, which the engine needs to start
/// a session and can't find by itself inside the app.
pub const ENV_BUILTIN_PROVIDER_CONFIG: &str = "ZCODE_BUILTIN_PROVIDER_CONFIG_FILE";
/// Runs the app's own runtime as Node.js (Electron's own variable).
pub const ENV_RUN_AS_NODE: &str = "ELECTRON_RUN_AS_NODE";
/// The app's own program, which runs the engine.
const RUNTIME_NAME: &str = "ZCode";
/// The built-in provider settings inside the app's own files.
const BUILTIN_IN_APP: &str = "config/provider/zcode-builtin.json";

const PINNED: &[&str] = &[ENV_STORAGE_DIR, ENV_DATA_BASE_DIR, ENV_SESSION_DB_PATH];
const ISOLATION: &[&str] = &[
    ENV_STORAGE_DIR,
    ENV_DATA_BASE_DIR,
    ENV_SESSION_DB_PATH,
    ENV_SESSION_DB,
    ENV_PROVIDER_CONFIG,
    ENV_ENGINE_OVERRIDE,
    ENV_BUILTIN_PROVIDER_CONFIG,
    ENV_RUN_AS_NODE,
];
/// Removed from every launch (see the module docs).
const UNSET: &[&str] = &[ENV_SESSION_DB, ENV_PROVIDER_CONFIG];
/// Set on every run of the engine: the app's runtime runs it as Node.js.
const FIXED: &[(&str, &str)] = &[(ENV_RUN_AS_NODE, "1")];

/// Why no `ZCode` session starts in a terminal.
pub const NO_TERMINAL: &str = "ZCode's app ships no terminal UI (only the desktop app's server), so its sessions run in AI Juggler without a terminal";

/// `zcode version` prints the bare version (`0.16.9`); `--version` may put
/// the name first. The first word of the first line that starts with a
/// digit, a leading `v` dropped.
#[must_use]
pub fn parse_version(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .next()?
        .split_whitespace()
        .map(|word| word.strip_prefix('v').unwrap_or(word))
        .find(|word| word.starts_with(|c: char| c.is_ascii_digit()))
        .map(str::to_owned)
}

/// The `ZCode` tree of an App's folder: the folder itself when it is a
/// tree (`ZCode`'s own `~/.zcode`), else `<folder>/.zcode`.
#[must_use]
pub fn tree_of(state_dir: &Path) -> PathBuf {
    if state_dir.file_name().is_some_and(|name| name == TREE_NAME) {
        state_dir.to_path_buf()
    } else {
        state_dir.join(TREE_NAME)
    }
}

/// The session store of an App's folder.
#[must_use]
pub fn db_path_of(state_dir: &Path) -> PathBuf {
    crate::zcode_sessions::db_of(&tree_of(state_dir))
}

/// The launch check (Req.2): after a launch, the engine wrote its session
/// store inside the App's tree, so the variables took. `Err` says why the
/// App can't be used, in words.
///
/// # Errors
///
/// The store isn't in the App's tree.
pub fn isolation_took(state_dir: &Path) -> Result<(), String> {
    let db = db_path_of(state_dir);
    if db.is_file() {
        Ok(())
    } else {
        Err(format!(
            "ZCode didn't keep its sessions in this App's folder (no {}): this ZCode may not follow the settings AI Juggler gives it, so the App isn't used",
            db.display()
        ))
    }
}

/// Sign-in state of an App's folder: whether its tree holds `ZCode`'s
/// sign-in file, never its contents.
#[must_use]
pub fn zcode_login_state(state_dir: &Path) -> LoginState {
    match crate::sign_in::zcode_signed_in(state_dir, &tree_of(state_dir)) {
        None => LoginState::Unknown,
        Some(true) => LoginState::SignedIn,
        Some(false) => LoginState::LoggedOut,
    }
}

/// What runs `ZCode`'s engine: the engine, the app's own runtime and the
/// app's built-in provider settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
    /// The engine (`zcode.cjs`).
    pub engine: PathBuf,
    /// The app's own program, run as Node.js.
    pub runtime: PathBuf,
    /// The app's built-in provider settings.
    pub builtin: PathBuf,
}

impl Install {
    /// The variables every run of the engine gets: the runtime as Node.js
    /// and the provider settings.
    #[must_use]
    pub fn env(&self) -> Vec<(&'static str, String)> {
        vec![
            (ENV_RUN_AS_NODE, "1".to_owned()),
            (
                ENV_BUILTIN_PROVIDER_CONFIG,
                self.builtin.to_string_lossy().into_owned(),
            ),
        ]
    }
}

/// Adapter for `ZCode`.
#[derive(Debug, Clone, Default)]
pub struct ZCodeAdapter {
    engine: Option<PathBuf>,
    runtime: Option<PathBuf>,
    home: Option<PathBuf>,
    app_dirs: Option<Vec<PathBuf>>,
}

impl ZCodeAdapter {
    /// Creates the adapter, resolving the app at call time.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The adapter pinned to a specific engine file (tests/fixtures).
    #[must_use]
    pub fn with_engine(engine: impl Into<PathBuf>) -> Self {
        Self {
            engine: Some(engine.into()),
            ..Self::default()
        }
    }

    /// The adapter running the engine on `runtime` instead of the app's
    /// own (tests/fixtures).
    #[must_use]
    pub fn with_runtime(mut self, runtime: impl Into<PathBuf>) -> Self {
        self.runtime = Some(runtime.into());
        self
    }

    /// The adapter using `home` as the user's home (tests/fixtures).
    #[must_use]
    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }

    /// The adapter looking for the app in `dirs` only (tests/fixtures).
    #[must_use]
    pub fn with_app_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.app_dirs = Some(dirs);
        self
    }

    fn user_home(&self) -> Option<PathBuf> {
        self.home.clone().or_else(home_dir)
    }

    fn default_tree(&self) -> Option<PathBuf> {
        self.user_home().map(|home| home.join(TREE_NAME))
    }

    fn is_default(&self, dir: &Path) -> bool {
        self.default_tree().is_some_and(|default| {
            Path::new(dir.to_string_lossy().trim_end_matches('/')) == default
        })
    }

    /// Where the app is looked for: the system's install folders.
    fn app_dirs(&self) -> Vec<PathBuf> {
        self.app_dirs
            .clone()
            .unwrap_or_else(|| juggler_adapter::apps::install_folders(self.user_home().as_deref()))
    }

    /// The override, else the engine inside the first installed `ZCode`
    /// app that holds it where `ZCode` keeps it.
    fn resolve_engine(&self) -> Option<PathBuf> {
        if let Some(engine) = &self.engine {
            return Some(engine.clone());
        }
        if let Some(path) = std::env::var_os(ENV_ENGINE_OVERRIDE).map(PathBuf::from)
            && path.is_file()
        {
            return Some(path);
        }
        self.app_dirs()
            .into_iter()
            .filter_map(|dir| juggler_adapter::apps::installed_app(&dir, APP_NAME))
            .map(|app| juggler_adapter::apps::resources_of(&app).join(ENGINE_IN_APP))
            .find(|engine| engine.is_file())
    }

    /// The engine, the runtime and the provider settings of the app the
    /// engine lies in (the runtime override first).
    fn resolve(&self) -> Option<Install> {
        let engine = self.resolve_engine()?;
        let app = juggler_adapter::apps::bundle_of(&engine);
        let runtime = self.runtime.clone().or_else(|| {
            app.as_deref()
                .map(|app| juggler_adapter::apps::executable_of(app, RUNTIME_NAME))
        })?;
        let builtin = app
            .as_deref()
            .map(|app| juggler_adapter::apps::resources_of(app).join(BUILTIN_IN_APP))?;
        Some(Install {
            engine,
            runtime,
            builtin,
        })
    }

    /// The app's engine, runtime and settings, or why a run can't start.
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] in words.
    pub fn require_install(&self) -> Result<Install, AdapterError> {
        let install = self
            .resolve()
            .filter(|install| install.engine.is_file())
            .ok_or_else(|| {
                AdapterError::InvalidLaunch("ZCode not found; run discovery for details".into())
            })?;
        if !install.runtime.is_file() {
            return Err(AdapterError::InvalidLaunch(format!(
                "ZCode's app has no runtime at {}; reinstall ZCode",
                install.runtime.display()
            )));
        }
        if !install.builtin.is_file() {
            return Err(AdapterError::InvalidLaunch(format!(
                "ZCode's app has no provider settings at {}; reinstall ZCode",
                install.builtin.display()
            )));
        }
        Ok(install)
    }

    /// One run of the engine with `args`, for the App at `state_dir`: the
    /// app's runtime, Launch-settings overrides first, isolation LAST
    /// (AJ-E309), piped (the driven runtime and its helpers).
    pub(crate) fn engine_spec(
        &self,
        state_dir: &Path,
        overrides: &EnvMap,
        cwd: &Path,
        args: &[&str],
    ) -> Result<LaunchSpec, AdapterError> {
        let install = self.require_install()?;
        let mut env: EnvMap = overrides.clone();
        env.extend(self.isolation_env(state_dir));
        for (key, value) in install.env() {
            env.insert(key.to_owned(), value);
        }
        let mut all = vec![install.engine.to_string_lossy().into_owned()];
        all.extend(args.iter().map(|arg| (*arg).to_owned()));
        Ok(LaunchSpec {
            program: install.runtime,
            args: all,
            env,
            cwd: cwd.to_path_buf(),
            needs_pty: false,
        })
    }

    fn report(
        &self,
        engine: Option<PathBuf>,
        install: InstallStatus,
        notes: Vec<String>,
    ) -> DiscoveryReport {
        DiscoveryReport {
            adapter: self.id(),
            install,
            verification: VerificationStatus::Unverified,
            binary: engine,
            notes,
        }
    }

    /// The pinned variables plus every removed one.
    fn isolation_env(&self, state_dir: &Path) -> EnvMap {
        let mut env = EnvMap::new();
        for (key, value) in self.pinned_env(state_dir) {
            env.insert(
                key.to_owned(),
                value.unwrap_or_else(|| ENV_UNSET.to_owned()),
            );
        }
        for key in UNSET {
            env.insert((*key).to_owned(), ENV_UNSET.to_owned());
        }
        env
    }
}

/// Engine commands that aren't a session (`zcode --help`, 0.16.9).
const HELPERS: &[&str] = &[
    "app-server",
    "commands",
    "doctor",
    "login",
    "logout",
    "plugins",
    "plugin",
    "skills",
    "version",
];
/// Options that print and exit.
const HELPER_FLAGS: &[&str] = &["-h", "--help", "-v", "--version"];
/// Options that take the next word as their value.
const VALUE_OPTIONS: &[&str] = &[
    "-p",
    "--prompt",
    "--resume",
    "--cwd",
    "--mode",
    "--target",
    "--attach",
    "--locale",
    "--surface",
    "--browser-use",
    "--browser-executable",
    "--workflow-mode",
    "--disallowed-tools",
    "--disallowedTools",
];

/// A running `ZCode` engine (`node …/ZCode.app/Contents/Resources/glm/
/// zcode.cjs …`, or the app's own runtime running it) is a session (its
/// terminal, `-p`, `tui`) unless it runs a helper command (`app-server`,
/// the desktop app's server; `doctor`, `version`, `login`, …).
/// `--resume <id>` names its conversation.
#[must_use]
pub fn session_process(
    exe: Option<&Path>,
    args: &[String],
) -> Option<juggler_adapter::conversation::SessionProcess> {
    let is_engine = |arg: &str| {
        Path::new(arg)
            .file_name()
            .is_some_and(|name| name == ENGINE_FILE)
    };
    let at = args.iter().position(|arg| is_engine(arg))?;
    // The engine runs as a script: on Node.js, on the app's own runtime,
    // or by itself (its `#!` line).
    let runner = exe
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .or_else(|| {
            args.first()
                .and_then(|first| Path::new(first).file_name())
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let runs_it =
        at == 0 || runner.starts_with("node") || runner.contains("ZCode") || is_engine(&runner);
    if !runs_it || at > 1 {
        return None;
    }
    let mut rest = args[at + 1..].iter();
    while let Some(arg) = rest.next() {
        if arg == "--" {
            break;
        }
        if VALUE_OPTIONS.contains(&arg.as_str()) {
            rest.next();
            continue;
        }
        if arg.starts_with('-') {
            if HELPER_FLAGS.contains(&arg.as_str()) {
                return None;
            }
            continue;
        }
        // The first word is its command.
        if HELPERS.contains(&arg.as_str()) {
            return None;
        }
        break;
    }
    Some(juggler_adapter::conversation::SessionProcess {
        resumes: juggler_adapter::conversation::value_after(args, &["--resume"]),
    })
}

impl ToolIdentity for ZCodeAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(ZCODE_ID)
    }
    fn spec(&self) -> juggler_adapter::ToolSpec {
        zcode_spec()
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::RESUME
            | Capabilities::TRANSCRIPT_WATCH
            | Capabilities::USAGE_TOKENS
            | Capabilities::LIMITS_FROM_PLAN_CAPS
            | Capabilities::LIMIT_HITS
            | Capabilities::PROTOCOL_SERVER
    }
    fn verified_against(&self) -> &'static [&'static str] {
        VERIFIED_AGAINST
    }
}

impl ToolDiscovery for ZCodeAdapter {
    fn locate(&self) -> Option<PathBuf> {
        // Files only: never runs it (`discover` asks its version).
        self.resolve_engine().filter(|engine| engine.is_file())
    }
    fn discover(&self) -> DiscoveryReport {
        let Some(engine) = self.resolve_engine().filter(|engine| engine.is_file()) else {
            return self.report(
                None,
                InstallStatus::NotFound,
                vec!["looked for the ZCode app where apps are installed".into()],
            );
        };
        let install = match self.require_install() {
            Ok(install) => install,
            Err(error) => {
                return self.report(
                    Some(engine),
                    InstallStatus::Failed {
                        message: error.to_string(),
                    },
                    Vec::new(),
                );
            }
        };
        let version = match crate::common::probe::version_with_env(
            &install.runtime,
            [engine.as_os_str(), "version".as_ref()],
            FIXED,
            parse_version,
        ) {
            Ok(version) => version,
            Err(failed) => return self.report(Some(engine), failed, Vec::new()),
        };
        self.report(
            Some(engine),
            InstallStatus::Installed { version },
            vec!["ZCode — built from engine 0.16.9's own code; not yet checked live".into()],
        )
    }
    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        let Some(home) = self.user_home() else {
            return Vec::new();
        };
        let env_override = std::env::var_os(ENV_STORAGE_DIR)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        crate::zcode_environment::scan_zcode_environment(
            &home,
            env_override.as_deref(),
            &scan_probe,
        )
    }
    fn vendor_default_dir(&self) -> Option<PathBuf> {
        self.default_tree()
    }
    /// None: the app ships no terminal UI to type a line for.
    fn shell_command(&self) -> Option<&'static str> {
        None
    }
    fn resolved_binary(&self) -> Option<PathBuf> {
        self.resolve_engine()
    }
}

impl ToolSignIn for ZCodeAdapter {
    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        prepare_managed_dir_for(state_dir, ZCODE_ID)?;
        let tree = tree_of(state_dir);
        crate::claude_code::create_private_dir(&tree).map_err(|error| {
            AdapterError::AccountDir {
                path: tree.clone(),
                message: error.to_string(),
            }
        })?;
        Ok(self
            .pinned_env(state_dir)
            .into_iter()
            .filter_map(|(key, value)| Some((key.to_owned(), value?)))
            .collect())
    }
    fn login_state(&self, state_dir: &Path) -> LoginState {
        zcode_login_state(state_dir)
    }
    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::zcode_environment::import_zcode_tree(source, dest, &scan_probe, false)
    }
    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        crate::zcode_environment::import_zcode_tree(source, dest, &scan_probe, true)
    }
    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        crate::zcode_environment::inspect_zcode_source(source, &scan_probe)
    }
    /// `ZCode`'s own `~/.zcode` only (ADR-0018; see
    /// [`crate::zcode_environment`]).
    fn can_use_in_place(&self) -> bool {
        true
    }
    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::zcode_environment::adopt_zcode_tree(
            source,
            self.default_tree().as_deref(),
            &scan_probe,
        )
    }
}

/// Whether a tree found by the scan is signed in (its file's presence).
fn scan_probe(dir: &Path) -> LoginProbe {
    match zcode_login_state(dir) {
        LoginState::SignedIn | LoginState::ApiKey => LoginProbe::LoggedIn,
        LoginState::Partial
        | LoginState::Expired
        | LoginState::SsoPending
        | LoginState::LoggedOut
        | LoginState::Unknown => LoginProbe::LoggedOut,
    }
}

impl ToolLaunch for ZCodeAdapter {
    /// Refused, every mode: the app ships no terminal UI ([`NO_TERMINAL`]);
    /// its sessions run driven ([`crate::zcode_appserver`]).
    fn launch_spec(
        &self,
        _account: &Account,
        _profile: &Profile,
        _cwd: &Path,
        _mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        Err(AdapterError::Unsupported(NO_TERMINAL.into()))
    }
    /// `ZCode`'s own `login` (Z.ai's sign-in in the browser) in this
    /// App's tree, on the app's runtime. Not yet run (AJ-E1612).
    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        let mut spec = self.engine_spec(state_dir, &EnvMap::new(), state_dir, &["login"])?;
        spec.needs_pty = true;
        Ok(spec)
    }
    fn launch_fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        FIXED
    }
    fn isolation_env_keys(&self) -> &'static [&'static str] {
        ISOLATION
    }
    fn pinned_env_keys(&self) -> &'static [&'static str] {
        PINNED
    }
    fn launch_unset_keys(&self) -> &'static [&'static str] {
        UNSET
    }
    /// The tree, the folder above it (whose `.zcode/v2/` holds the
    /// sign-in) and the session store; `ZCode`'s own `~/.zcode` runs with
    /// all three unset.
    fn pinned_env(&self, state_dir: &Path) -> Vec<(&'static str, Option<String>)> {
        if self.is_default(state_dir) {
            return PINNED.iter().map(|key| (*key, None)).collect();
        }
        let tree = tree_of(state_dir);
        let base = tree.parent().unwrap_or(state_dir).to_path_buf();
        let text = |path: &Path| Some(path.to_string_lossy().into_owned());
        vec![
            (ENV_STORAGE_DIR, text(&tree)),
            (ENV_DATA_BASE_DIR, text(&base)),
            (
                ENV_SESSION_DB_PATH,
                text(&crate::zcode_sessions::db_of(&tree)),
            ),
        ]
    }
}

impl ToolConfig for ZCodeAdapter {}

/// AJ-E1605 Req.5: the engine reads its MCP servers, skills and rules
/// from the user's home folder whatever tree an App runs in, so every App
/// lists, and sets up into, the one set in `~/.zcode`
/// ([`crate::zcode_inventory`]).
impl ToolInventory for ZCodeAdapter {
    fn inventory(
        &self,
        _state_dir: &Path,
        _project: Option<&Path>,
    ) -> Vec<juggler_adapter::inventory::Item> {
        self.default_tree()
            .map(|tree| crate::zcode_inventory::inventory(&tree))
            .unwrap_or_default()
    }
    fn can_set_up(
        &self,
        item: &juggler_adapter::inventory::Item,
    ) -> juggler_adapter::inventory::SetUp {
        crate::zcode_inventory::can_set_up(item)
    }
    fn set_up(
        &self,
        _state_dir: &Path,
        _project: Option<&Path>,
        item: &juggler_adapter::inventory::Item,
    ) -> Result<juggler_adapter::inventory::Applied, AdapterError> {
        let tree = self.default_tree().ok_or_else(|| {
            AdapterError::Unsupported("your home folder isn't known, so nothing was set up".into())
        })?;
        crate::zcode_inventory::set_up(&tree, item)
    }
}

impl ToolTranscripts for ZCodeAdapter {
    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        Ok(Box::new(crate::common::sqlite::StoreWatcher::new(
            db_path_of(transcript_root),
            crate::zcode_sessions::session_changes,
        )))
    }
    /// The session that ran in `cwd`, from the tree's store (read-only);
    /// its link is `<db>#<id>`.
    fn locate_transcript(
        &self,
        state_dir: &Path,
        cwd: &Path,
        since_ms: i64,
        taken: &[PathBuf],
    ) -> Option<juggler_core::model::TranscriptRef> {
        crate::zcode_sessions::locate(&db_path_of(state_dir), cwd, since_ms, taken).map(
            |(path, id)| juggler_core::model::TranscriptRef {
                path,
                native_id: Some(id),
            },
        )
    }
    fn stored_conversations(
        &self,
        state_dir: &Path,
    ) -> Vec<juggler_adapter::conversation::StoredConversation> {
        crate::zcode_sessions::stored_conversations(&db_path_of(state_dir))
    }
    /// A session's turns from the store (`<db>#<id>`), for a handover
    /// (Req.6; [`crate::zcode_sessions::turns`]).
    fn conversation_turns(
        &self,
        transcript: &Path,
    ) -> Option<Vec<juggler_adapter::handover::TurnText>> {
        crate::zcode_sessions::turns_of_ref(transcript).map(|turns| {
            turns
                .into_iter()
                .map(|turn| juggler_adapter::handover::TurnText {
                    uuid: None,
                    role: turn.role,
                    text: turn.text,
                })
                .collect()
        })
    }
    fn subagent_support(&self) -> juggler_adapter::SubagentSupport {
        juggler_adapter::SubagentSupport::NotSupported(
            "ZCode keeps a sub-agent as a session of its own in its store, under its parent's id; AI Juggler doesn't list them yet.",
        )
    }
}

impl ToolQuota for ZCodeAdapter {
    /// Completed `model_usage` rows from the App's store, as credits by
    /// Z.ai's published formula (Computed; [`crate::zcode_usage`]).
    fn usage_since(
        &self,
        account: &Account,
        since_ms: i64,
    ) -> Option<Result<Vec<juggler_core::quota::UsageEvent>, String>> {
        let db = db_path_of(&account.state_dir);
        if !db.is_file() {
            return Some(Ok(Vec::new()));
        }
        Some(crate::zcode_usage::zcode_events_since(
            account.id.as_str(),
            &db,
            since_ms,
        ))
    }
    /// A session's hit is in the App's store, not a transcript's tail
    /// ([`Self::session_limit_hit`]).
    fn limit_hit(&self, _tail: &str) -> Option<juggler_adapter::live::LimitHit> {
        None
    }
    /// From the session's own `model_usage` rows: its last call refused
    /// at the plan's limit, with Z.ai's window and reset
    /// ([`crate::zcode_limits`]).
    fn session_limit_hit(
        &self,
        transcript: &Path,
    ) -> Option<Option<juggler_adapter::live::LimitHit>> {
        crate::zcode_limits::session_limit_hit(transcript)
    }
}

impl ToolLive for ZCodeAdapter {
    fn session_process(
        &self,
        exe: Option<&Path>,
        args: &[String],
    ) -> Option<juggler_adapter::conversation::SessionProcess> {
        session_process(exe, args)
    }
}

impl ToolHandover for ZCodeAdapter {
    fn switch_capabilities(&self) -> juggler_adapter::switching::SwitchCapabilities {
        juggler_adapter::switching::fresh_start_only
    }
}

/// Round 3: the engine's app-server is `ZCode`'s only runtime
/// ([`crate::zcode_appserver`]).
impl ToolRuntime for ZCodeAdapter {
    fn driven(&self) -> Option<&dyn juggler_adapter::runtime::DrivenRuntime> {
        Some(self)
    }
}

/// ADR-0019: `ZCode`, described as data.
#[must_use]
pub fn zcode_spec() -> juggler_adapter::ToolSpec {
    use juggler_adapter::{SetupGroup, SignInWords, ToolFlags, ToolKind, ToolSpec, ToolTerms};
    ToolSpec {
        id: ZCODE_ID.to_owned(),
        display_name: "ZCode".to_owned(),
        short_name: "ZCode".to_owned(),
        vendor: "Z.ai".to_owned(),
        // Not yet checked against its owner's guidelines (brand-marks.md):
        // the letter mark until then.
        mark: Some("zcode".to_owned()),
        vendor_mark: None,
        install_url: Some("https://zcode.z.ai/".to_owned()),
        // Only the desktop app is published; no command installs it.
        install_command: None,
        sign_in: SignInWords {
            own: "ZCode's own sign-in".to_owned(),
            short: "ZCode's".to_owned(),
        },
        same_as: "as when you run ZCode yourself".to_owned(),
        // ZCode's desktop app shares `~/.zcode`, but can't be started per
        // App: not offered here.
        desktop_app: None,
        app_meter: false,
        kinds: vec![ToolKind::Cli],
        rules_files: vec!["AGENTS.md".to_owned()],
        rules_import: None,
        scan_hint: "Looks for ZCode on this computer. Nothing is changed.".to_owned(),
        setup_groups: vec![SetupGroup {
            id: ZCODE_ID.to_owned(),
            title: "ZCode".to_owned(),
            path_contains: None,
            origin: None,
            shows_install: true,
        }],
        default_label: Some("ZCode".to_owned()),
        flags: ToolFlags {
            first_run: true,
            primary: false,
        },
        // The vendor's pages, read 2026-11-13, recorded in AJ-E1518's
        // register.
        terms: Some(ToolTerms {
            name: "ZCode's User Agreement".to_owned(),
            url: "https://zcode.z.ai/cn/terms".to_owned(),
            accounts: "ZCode's User Agreement lets it freeze or close accounts registered or used \
                       improperly as several, and Z.ai's GLM Coding Plan policy says not to share \
                       an account."
                .to_owned(),
            checked: "2026-11-13".to_owned(),
        }),
        // The engine updates with the app (its own updater): no public
        // package list, and no command to run.
        release_feed: None,
        update_command: None,
        floor: juggler_adapter::FloorFamilies::all("neutral"),
    }
}
