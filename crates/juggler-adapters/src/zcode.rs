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
//! engine is
//! `Contents/Resources/glm/zcode.cjs`. The app ships no Node.js of its
//! own, so the engine runs on a `node` from the user's PATH or Homebrew's
//! folders; its version is what `node <engine> version` prints.
//! `AI_JUGGLER_ZCODE_ENGINE` names another engine file (tests, dev).
//!
//! Isolation without moving `HOME` (Req.2; a moved `HOME` would take the
//! user's git identity and SSH keys away from `ZCode`'s shell tools). An
//! App's tree is `<account>/.zcode`, and each launch pins
//!
//! - `ZCODE_STORAGE_DIR` = the tree (the engine's `storage.dir`: plugins,
//!   skills, its per-session working files);
//! - `ZCODE_DATA_BASE_DIR` = the account folder (the sign-in is then
//!   `<account>/.zcode/v2/credentials.json`);
//! - `ZCODE_SESSION_DB_PATH` = `<tree>/cli/db/db.sqlite` (the engine's
//!   `storage.sessionDbPath`, whose default `~/.zcode/cli/db/db.sqlite`
//!   follows the home folder, not `ZCODE_STORAGE_DIR`: without it every
//!   App's sessions and usage would land in the user's own store).
//!
//! All three are pinned after Launch-settings overrides; `ZCODE_SESSION_DB`
//! (another name for the store's path) and an inherited
//! `ZCODE_PERSONAL_PROVIDER_CONFIG_FILE` (would point every App at one
//! provider file) are removed from every launch. `ZCode`'s own `~/.zcode`
//! runs with all three unset, as a plain `zcode` does. These variables were
//! found in the engine's own code (0.16.9), not in Z.ai's docs, so a launch
//! check ([`isolation_took`]) confirms the engine wrote inside the App's
//! tree, and the app can refuse an App where it didn't. One file stays
//! shared: the engine reads its user settings from `~/.zcode/cli/config.json`
//! whatever these say (MCP servers, hooks; on the live checklist).
//!
//! Sessions (Req.3) come from the tree's store, read-only
//! ([`crate::zcode_sessions`]); `--resume <sess_…>` reopens one, `-c` the
//! folder's latest. A running engine is recognised by its command line.
//!
//! Not offered yet: runs without a terminal (`-p` skips `ZCode`'s
//! permission prompts by default, `--mode yolo`), a first message (its
//! terminal takes none), a model from Launch settings (its terminal has no
//! model option; `ZCode`'s own choice stands), and the agent tools.

use std::path::{Path, PathBuf};

use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::prelude::*;
use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, InstallStatus, LaunchMode, LaunchSpec,
    SessionWatcher, VerificationStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

use crate::claude_code::{LoginState, existing, find_on_path, home_dir, prepare_managed_dir_for};

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

const PINNED: &[&str] = &[ENV_STORAGE_DIR, ENV_DATA_BASE_DIR, ENV_SESSION_DB_PATH];
const ISOLATION: &[&str] = &[
    ENV_STORAGE_DIR,
    ENV_DATA_BASE_DIR,
    ENV_SESSION_DB_PATH,
    ENV_SESSION_DB,
    ENV_PROVIDER_CONFIG,
    ENV_ENGINE_OVERRIDE,
];
/// Removed from every launch (see the module docs).
const UNSET: &[&str] = &[ENV_SESSION_DB, ENV_PROVIDER_CONFIG];

/// Why AI Juggler doesn't start `ZCode` without a terminal.
const NO_HEADLESS: &str = "ZCode's runs without a terminal (-p) skip its permission prompts unless told otherwise, so AI Juggler doesn't start them until they're checked live";

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

/// Adapter for `ZCode`.
#[derive(Debug, Clone, Default)]
pub struct ZCodeAdapter {
    engine: Option<PathBuf>,
    node: Option<PathBuf>,
    home: Option<PathBuf>,
    app_dirs: Option<Vec<PathBuf>>,
}

impl ZCodeAdapter {
    /// Creates the adapter, resolving the engine and Node.js at call time.
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

    /// The adapter running the engine on `node` (tests/fixtures).
    #[must_use]
    pub fn with_node(mut self, node: impl Into<PathBuf>) -> Self {
        self.node = Some(node.into());
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

    /// The Node.js the engine runs on: the user's PATH, then Homebrew's.
    fn resolve_node(&self) -> Option<PathBuf> {
        if let Some(node) = &self.node {
            return Some(node.clone());
        }
        find_on_path("node")
            .or_else(|| existing(&["/opt/homebrew/bin/node", "/usr/local/bin/node"]))
    }

    /// Node.js and the engine, or why a launch can't start.
    fn require_program(&self) -> Result<(PathBuf, PathBuf), AdapterError> {
        let engine = self.resolve_engine().ok_or_else(|| {
            AdapterError::InvalidLaunch("ZCode not found; run discovery for details".into())
        })?;
        let node = self.resolve_node().ok_or_else(|| {
            AdapterError::InvalidLaunch(
                "ZCode runs on Node.js, which wasn't found; run discovery for details".into(),
            )
        })?;
        Ok((node, engine))
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
        Capabilities::RESUME | Capabilities::TRANSCRIPT_WATCH | Capabilities::USAGE_TOKENS
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
        let Some(node) = self.resolve_node() else {
            return self.report(
                Some(engine),
                InstallStatus::Failed {
                    message: "ZCode runs on Node.js, which its app doesn't include, and none was found on PATH, in /opt/homebrew/bin or in /usr/local/bin".into(),
                },
                Vec::new(),
            );
        };
        let version = match crate::common::probe::version_with(
            &node,
            [engine.as_os_str(), "version".as_ref()],
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
    fn vendor_default_dir(&self) -> Option<PathBuf> {
        self.default_tree()
    }
    /// `ZCode` puts no `zcode` on the PATH: the copyable line names the
    /// engine by its path (it starts itself on the user's `node`).
    fn shell_command(&self) -> Option<&'static str> {
        Some("zcode")
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
}

impl ToolLaunch for ZCodeAdapter {
    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        let (node, engine) = self.require_program()?;
        // A resumed session continues work that lives in `cwd`.
        let effective_cwd = match mode {
            LaunchMode::Resume { .. } => cwd.to_path_buf(),
            _ => profile
                .default_cwd
                .clone()
                .unwrap_or_else(|| cwd.to_path_buf()),
        };
        // Launch-settings overrides first, isolation LAST (AJ-E309).
        let mut env: EnvMap = profile.settings_overlay.env.clone();
        env.extend(self.isolation_env(&account.state_dir));
        let mut args: Vec<String> = vec![engine.to_string_lossy().into_owned()];
        match mode {
            LaunchMode::Interactive => {}
            LaunchMode::Resume {
                session_id,
                delta_prompt,
            } => {
                crate::codex::refuse_option_like(session_id)?;
                if delta_prompt.is_some() {
                    return Err(AdapterError::Unsupported(
                        "ZCode can't be given a message as it resumes yet".into(),
                    ));
                }
                args.extend(["--resume".into(), session_id.clone()]);
            }
            // Its terminal takes no first message.
            LaunchMode::Primed { .. } => {
                return Err(AdapterError::Unsupported(
                    "ZCode can't be started with a first message yet".into(),
                ));
            }
            LaunchMode::Headless { .. } => {
                return Err(AdapterError::Unsupported(NO_HEADLESS.into()));
            }
        }
        Ok(LaunchSpec {
            program: node,
            args,
            env,
            cwd: effective_cwd,
            needs_pty: true,
        })
    }
    /// `ZCode`'s own `zcode login` (Z.ai's sign-in in the browser) in this
    /// App's tree.
    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        let (node, engine) = self.require_program()?;
        Ok(LaunchSpec {
            program: node,
            args: vec![engine.to_string_lossy().into_owned(), "login".into()],
            env: self.isolation_env(state_dir),
            cwd: state_dir.to_path_buf(),
            needs_pty: true,
        })
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
    /// `zcode --resume <sess_…>` reopens it.
    fn resume_args(&self, conversation_id: &str) -> Option<Vec<String>> {
        crate::codex::refuse_option_like(conversation_id).ok()?;
        Some(vec!["--resume".into(), conversation_id.to_owned()])
    }
}

// AJ-E1605 Req.5: rules, skills and MCP servers join later.
impl ToolConfig for ZCodeAdapter {}
impl ToolInventory for ZCodeAdapter {}

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

impl ToolRuntime for ZCodeAdapter {}

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
