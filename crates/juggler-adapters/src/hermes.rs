//! Hermes Agent adapter (AJ-E1607) — Nous Research's `hermes` CLI (D-74).
//!
//! Built from the research (`docs/research/hermes.md`) and the MIT source
//! at `NousResearch/hermes-agent@422ad186` (read 2026-11-13); Hermes isn't
//! installed on the Mac this was built on, so nothing has run it yet —
//! that is AJ-E1612's lab check. Every version is "unverified".
//!
//! **Discovery.** The installer's `~/.local/bin/hermes` wrapper, then
//! `hermes` on the PATH and Homebrew's folders. `hermes --version` prints
//! `Hermes Agent v<version> (<release date>)`, then a passive update check
//! that Hermes caches for six hours.
//!
//! **Isolation per account.** `HERMES_HOME` moves all of a home: settings,
//! `SOUL.md`, memories, skills, sign-ins (`auth.json`, `.env`) and the
//! session store (`state.db`). A home outside `~/.hermes` is its own root
//! (`hermes_constants.py`, `_hermes_root_of`), so its shared Nous Portal
//! sign-in (`<root>/shared/nous_auth.json`) and its process ledger are its
//! own too; `HERMES_SHARED_AUTH_DIR`, which would point every account at
//! one, is pinned to it. The gateway's host lock is not under the home
//! (`$HERMES_GATEWAY_LOCK_DIR`, else `~/.local/state/hermes/gateway-locks`),
//! and one gateway per lock folder serves every home that dials it, so each
//! account gets a lock folder of its own ([`crate::hermes_gateway`]).
//! `HERMES_KANBAN_HOME` (one board for several homes) and
//! `HERMES_DATA_DIR_SUFFIX` (moves the default home) are removed from
//! every launch. The user's own `~/.hermes` runs with all of these unset,
//! as a plain `hermes` does.
//!
//! **The guardrail** (Req.3): every run carries Hermes' own policy layer
//! keeping it away from Codex's and Claude's sign-ins and homes
//! ([`crate::hermes_guard`]).
//!
//! **Sessions** come from the home's `state.db`, read-only with a format
//! check ([`crate::hermes_sessions`]). **Usage** is what `hermes usage
//! --json` reports for the account's provider ([`crate::hermes_usage`]).
//!
//! Not offered: a headless run (`hermes -z` bypasses every approval, and
//! skipping them is never done), a message as it resumes, a driven
//! runtime (its ACP and TUI-gateway protocols are found, not yet built).

use crate::common::prelude::*;

use crate::claude_code::{existing, find_on_path};

/// Provider id: `hermes-agent` (Hermes is also a model family and a
/// JavaScript engine).
pub const HERMES_ID: &str = "hermes-agent";

/// No version has been run live yet (AJ-E1612): every one is unverified.
const VERIFIED_AGAINST: &[&str] = &[];

/// The source this adapter was built from.
pub const SOURCE_READ: &[&str] = &["main@422ad186 (2026-11-13)"];

/// Hermes' home.
pub const ENV_HOME: &str = "HERMES_HOME";
/// The gateway's host lock folder.
pub const ENV_LOCKS: &str = "HERMES_GATEWAY_LOCK_DIR";
/// The shared Nous Portal sign-in's folder.
pub const ENV_SHARED_AUTH: &str = "HERMES_SHARED_AUTH_DIR";
/// Hermes' administrator policy folder (the guard's).
pub const ENV_POLICY: &str = "HERMES_MANAGED_DIR";
/// Where Hermes looks for the Codex CLI's home.
pub const ENV_CODEX: &str = "CODEX_HOME";
/// Where Hermes looks for Claude Code's home.
pub const ENV_CLAUDE: &str = "CLAUDE_CONFIG_DIR";
/// One kanban board for several homes.
const ENV_KANBAN: &str = "HERMES_KANBAN_HOME";
/// Moves the default home (`~/.hermes<suffix>`).
const ENV_SUFFIX: &str = "HERMES_DATA_DIR_SUFFIX";
/// Optional test/dev override for the binary path.
const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_HERMES_BIN";

/// The folder an account keeps its shared Nous Portal sign-in in.
const SHARED_DIR: &str = "shared";

const PINNED: &[&str] = &[
    ENV_HOME,
    ENV_LOCKS,
    ENV_SHARED_AUTH,
    ENV_POLICY,
    ENV_CODEX,
    ENV_CLAUDE,
];
const ISOLATION: &[&str] = &[
    ENV_HOME,
    ENV_LOCKS,
    ENV_SHARED_AUTH,
    ENV_POLICY,
    ENV_CODEX,
    ENV_CLAUDE,
    ENV_KANBAN,
    ENV_SUFFIX,
    ENV_BIN_OVERRIDE,
];
/// Removed from every launch (see the module docs).
const UNSET: &[&str] = &[ENV_KANBAN, ENV_SUFFIX];

/// `hermes --version` prints `Hermes Agent v0.22.3 (2026.9.24)`: the
/// first word that starts with a digit, its `v` dropped.
#[must_use]
pub fn parse_version(stdout: &str) -> Option<String> {
    crate::common::probe::first_version_word(stdout)
}

/// Adapter for Hermes Agent.
#[derive(Debug, Clone, Default)]
pub struct HermesAdapter {
    bin: Option<PathBuf>,
    /// The user's home (tests/fixtures; `None` reads `$HOME`).
    home: Option<PathBuf>,
    /// Where an administrator's policy would be (tests; `None` is
    /// [`crate::hermes_guard::SYSTEM_POLICY`]).
    system_policy: Option<PathBuf>,
}

impl HermesAdapter {
    /// Creates the adapter, resolving the binary at call time.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The adapter pinned to a specific binary (tests/fixtures).
    #[must_use]
    pub fn with_bin(bin: impl Into<PathBuf>) -> Self {
        let mut adapter = Self::new();
        adapter.bin = Some(bin.into());
        adapter
    }

    /// The adapter using `home` as the user's home (tests/fixtures).
    #[must_use]
    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }

    /// The adapter looking for an administrator's policy at `path`
    /// (tests/fixtures).
    #[must_use]
    pub fn with_system_policy(mut self, path: impl Into<PathBuf>) -> Self {
        self.system_policy = Some(path.into());
        self
    }

    fn user_home(&self) -> Option<PathBuf> {
        self.home.clone().or_else(home_dir)
    }

    /// The user's own Hermes home, `~/.hermes`.
    fn default_home(&self) -> Option<PathBuf> {
        self.user_home().map(|home| home.join(".hermes"))
    }

    /// Whether `dir` is the user's own home, by its exact path.
    fn is_default(&self, dir: &Path) -> bool {
        self.default_home().is_some_and(|default| {
            Path::new(dir.to_string_lossy().trim_end_matches('/')) == default
        })
    }

    /// The installer's wrapper first, then the PATH and Homebrew's.
    fn resolve_bin(&self) -> Option<PathBuf> {
        crate::common::probe::pinned_bin(self.bin.as_ref(), ENV_BIN_OVERRIDE).or_else(|| {
            self.user_home()
                .map(|home| home.join(".local/bin/hermes"))
                .filter(|path| path.is_file())
                .or_else(|| find_on_path("hermes"))
                .or_else(|| existing(&["/opt/homebrew/bin/hermes", "/usr/local/bin/hermes"]))
        })
    }

    /// The binary, or why a run can't start.
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] when Hermes isn't found.
    pub fn require_bin(&self) -> Result<PathBuf, AdapterError> {
        self.resolve_bin().ok_or_else(|| {
            AdapterError::InvalidLaunch("hermes not found; run discovery for details".into())
        })
    }

    /// Refuses a run on a Mac with an administrator's Hermes policy, then
    /// writes the account's guard (see [`crate::hermes_guard`]).
    fn guard(&self, state_dir: &Path) -> Result<(), AdapterError> {
        let system = self
            .system_policy
            .clone()
            .unwrap_or_else(|| PathBuf::from(crate::hermes_guard::SYSTEM_POLICY));
        if let Some(refused) = crate::hermes_guard::refusal(&system) {
            return Err(refused);
        }
        crate::hermes_guard::ensure(state_dir)
    }

    /// The guard's variables, for the user's own home too (an account's
    /// are among its pinned ones).
    fn guard_env(state_dir: &Path) -> EnvMap {
        let text = |relative: &str| state_dir.join(relative).to_string_lossy().into_owned();
        [
            (ENV_POLICY, text(crate::hermes_guard::POLICY_DIR)),
            (ENV_CODEX, text(crate::hermes_guard::NO_CODEX)),
            (ENV_CLAUDE, text(crate::hermes_guard::NO_CLAUDE)),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
    }

    /// The environment of a Hermes run for the account at `state_dir`: its
    /// isolation, then the guard.
    pub(crate) fn account_env(&self, state_dir: &Path) -> EnvMap {
        let mut env = self.isolation_env(state_dir);
        env.extend(Self::guard_env(state_dir));
        env
    }

    /// One run's environment: Launch-settings overrides first (never an
    /// isolation or guard variable), the account's own LAST (AJ-E309).
    fn run_env(&self, state_dir: &Path, profile: &Profile) -> EnvMap {
        let mut env: EnvMap = profile.settings_overlay.env.clone();
        env.retain(|key, _| !ISOLATION.contains(&key.as_str()));
        env.extend(self.account_env(state_dir));
        env
    }

    /// Starts a gateway for the account that AI Juggler owns
    /// ([`crate::hermes_gateway::OwnedGateway`]), with its isolation and
    /// guard.
    ///
    /// # Errors
    ///
    /// A plain message when Hermes isn't found, the guard can't be
    /// written, or the gateway can't start.
    pub fn start_gateway(
        &self,
        account: &Account,
    ) -> Result<crate::hermes_gateway::OwnedGateway, String> {
        let bin = self.require_bin().map_err(|error| error.to_string())?;
        self.guard(&account.state_dir)
            .map_err(|error| error.to_string())?;
        crate::hermes_gateway::OwnedGateway::start(
            &bin,
            &account.state_dir,
            &self.account_env(&account.state_dir),
        )
    }

    /// Stops whatever gateway serves the account, through Hermes' own
    /// `hermes gateway stop`.
    ///
    /// # Errors
    ///
    /// A plain message when Hermes isn't found or its stop fails.
    pub fn stop_gateway(&self, account: &Account) -> Result<(), String> {
        let bin = self.require_bin().map_err(|error| error.to_string())?;
        crate::hermes_gateway::stop(
            &bin,
            &account.state_dir,
            &self.account_env(&account.state_dir),
        )
    }
}

/// Sign-in state of a Hermes home, from which sign-in files are there
/// (never their contents). A home with keys only in `.env` reads as
/// signed out.
#[must_use]
pub fn hermes_login_state(home: &Path) -> LoginState {
    match crate::sign_in::hermes_signed_in(home) {
        None => LoginState::Unknown,
        Some(true) => LoginState::SignedIn,
        Some(false) => LoginState::LoggedOut,
    }
}

fn scan_probe(home: &Path) -> LoginProbe {
    hermes_login_state(home).probe()
}

/// Where discovery looks for it.
const LOOKED_IN: &str = "looked in ~/.local/bin, on PATH, /opt/homebrew/bin, /usr/local/bin";

/// Options that print and exit.
const HELPER_FLAGS: &[&str] = &["-h", "--help", "-V", "--version"];
/// Options that take the next word as their value (`hermes --help`).
const VALUE_OPTIONS: &[&str] = &[
    "-p",
    "--profile",
    "-m",
    "--model",
    "--provider",
    "--reasoning",
    "-t",
    "--toolsets",
    "-r",
    "--resume",
    "-s",
    "--skills",
    "--usage-file",
    "--in",
    "--cpu-profile",
    "-z",
    "--oneshot",
];

/// Whether a command line is Hermes': its wrapper or binary named
/// `hermes`, or Python running it (`python …/hermes …`,
/// `python -m hermes_cli.main …`). The words after it.
fn hermes_words<'a>(exe: Option<&Path>, args: &'a [String]) -> Option<&'a [String]> {
    let named = |word: &str| {
        Path::new(word)
            .file_name()
            .is_some_and(|name| name == "hermes")
    };
    if exe.is_some_and(|exe| exe.file_name().is_some_and(|name| name == "hermes"))
        || args.first().is_some_and(|first| named(first))
    {
        return args.get(1..);
    }
    let at = args
        .iter()
        .take(4)
        .position(|arg| named(arg) || arg == "hermes_cli.main")?;
    args.get(at + 1..)
}

/// A running Hermes is a session when it runs its chat: no command, or
/// `chat`. `--resume <id>` (`-r`) names its conversation. A gateway, a
/// server, a sign-in or upkeep isn't one.
#[must_use]
pub fn session_process(
    exe: Option<&Path>,
    args: &[String],
) -> Option<juggler_adapter::conversation::SessionProcess> {
    let words = hermes_words(exe, args)?;
    match crate::common::launch::command_word(words, VALUE_OPTIONS, HELPER_FLAGS) {
        crate::common::launch::Command::Bare | crate::common::launch::Command::Word(_, "chat") => {
            Some(juggler_adapter::conversation::SessionProcess {
                resumes: juggler_adapter::conversation::value_after(words, &["--resume", "-r"]),
            })
        }
        _ => None,
    }
}

/// Whether a running Hermes is its gateway (`hermes gateway run`): the one
/// writer of its home's sessions.
#[must_use]
pub fn is_gateway(exe: Option<&Path>, args: &[String]) -> bool {
    hermes_words(exe, args).is_some_and(|words| {
        matches!(
            crate::common::launch::command_word(words, VALUE_OPTIONS, HELPER_FLAGS),
            crate::common::launch::Command::Word(at, "gateway")
                if words.get(at + 1).is_some_and(|next| next == "run")
        )
    })
}

/// A terminal run's arguments for `mode`, with the Launch settings'
/// `model` (`-m`, an option of `hermes` itself and of `hermes chat`).
fn terminal_args(mode: &LaunchMode, model: Option<&str>) -> Result<Vec<String>, AdapterError> {
    let mut args: Vec<String> = model
        .map(|model| vec!["-m".to_owned(), model.to_owned()])
        .unwrap_or_default();
    match mode {
        LaunchMode::Interactive => {}
        LaunchMode::Resume {
            delta_prompt: Some(_),
            ..
        } => {
            return Err(AdapterError::Unsupported(
                "Hermes Agent can't be given a message as it resumes".into(),
            ));
        }
        LaunchMode::Resume { session_id, .. } => {
            crate::codex::refuse_option_like(session_id)?;
            args.push("--resume".to_owned());
            args.push(session_id.clone());
        }
        // `hermes chat --query` on a terminal seeds an interactive session
        // with its first turn; `--query=<text>` keeps a message that starts
        // with `-` from reading as an option.
        LaunchMode::Primed { prompt, .. } => {
            args.push("chat".to_owned());
            args.push(format!("--query={prompt}"));
        }
        LaunchMode::Headless { .. } => {
            return Err(AdapterError::Unsupported(
                "Hermes Agent runs in its terminal, never headless: its one-shot mode skips every approval".into(),
            ));
        }
    }
    Ok(args)
}

impl ToolIdentity for HermesAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(HERMES_ID)
    }
    fn spec(&self) -> juggler_adapter::ToolSpec {
        hermes_spec()
    }
    /// Sessions from its store; the windows `hermes usage` reports.
    fn capabilities(&self) -> Capabilities {
        Capabilities::RESUME
            | Capabilities::TRANSCRIPT_WATCH
            | Capabilities::USAGE_QUOTA_FEED
            | Capabilities::LIMIT_WINDOWS_REPORTED
    }
    fn verified_against(&self) -> &'static [&'static str] {
        VERIFIED_AGAINST
    }
}

impl ToolDiscovery for HermesAdapter {
    fn shell_command(&self) -> Option<&'static str> {
        Some("hermes")
    }
    fn resolved_binary(&self) -> Option<PathBuf> {
        self.resolve_bin()
    }
    /// AJ-E1610: its own desktop app (`desktop_apps.json`), on the same
    /// default folder as the CLI, so its sessions are this adapter's.
    fn host_apps(&self) -> Vec<juggler_adapter::live::HostApp> {
        crate::desktop_apps::for_tool(HERMES_ID)
    }
    fn vendor_default_dir(&self) -> Option<PathBuf> {
        self.default_home()
    }
    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        crate::common::home::scan_home(self.user_home(), ENV_HOME, |home, env| {
            crate::hermes_environment::scan_hermes_environment(home, env, &scan_probe)
        })
    }
    /// Asks its version (`hermes --version`).
    fn discover(&self) -> DiscoveryReport {
        crate::common::probe::discover_bin(
            self.id(),
            self.resolve_bin(),
            LOOKED_IN,
            |bin| crate::common::probe::version(bin, parse_version),
            "Hermes Agent — built from its MIT source; not yet checked live",
        )
    }
    /// Files only: never runs it.
    fn locate(&self) -> Option<PathBuf> {
        self.resolve_bin().filter(|bin| bin.is_file())
    }
}

impl ToolSignIn for HermesAdapter {
    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        prepare_managed_dir_for(state_dir, HERMES_ID)?;
        crate::hermes_guard::ensure(state_dir)?;
        Ok(self.pinned_set_env(state_dir))
    }
    fn login_state(&self, state_dir: &Path) -> LoginState {
        hermes_login_state(state_dir)
    }
    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::hermes_environment::import_hermes_home(source, dest, &scan_probe, false)
    }
    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        crate::hermes_environment::import_hermes_home(source, dest, &scan_probe, true)
    }
    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        crate::hermes_environment::inspect_hermes_source(source, &scan_probe)
    }
    fn can_use_in_place(&self) -> bool {
        true
    }
    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::hermes_environment::adopt_hermes_home(source, &scan_probe)
    }
}

impl ToolLaunch for HermesAdapter {
    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        let program = self.require_bin()?;
        let args = terminal_args(mode, profile.settings_overlay.model.as_deref())?;
        self.guard(&account.state_dir)?;
        Ok(LaunchSpec {
            needs_pty: true,
            cwd: crate::common::launch::launch_cwd(profile, cwd, mode),
            env: self.run_env(&account.state_dir, profile),
            args,
            program,
        })
    }
    /// Hermes' own `hermes model`: pick a provider and sign in to it (a
    /// code or a page in the browser), in this account's home.
    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        let program = self.require_bin()?;
        self.guard(state_dir)?;
        Ok(LaunchSpec {
            program,
            args: vec!["model".into()],
            env: self.account_env(state_dir),
            cwd: state_dir.to_path_buf(),
            needs_pty: true,
        })
    }
    fn launch_unset_keys(&self) -> &'static [&'static str] {
        UNSET
    }
    fn isolation_env_keys(&self) -> &'static [&'static str] {
        ISOLATION
    }
    fn pinned_env_keys(&self) -> &'static [&'static str] {
        PINNED
    }
    /// An account's home, its own lock folder, shared sign-in folder and
    /// guard; the user's own `~/.hermes` runs with its home, lock and
    /// shared sign-in variables unset (its guard is added to each run).
    fn pinned_env(&self, state_dir: &Path) -> Vec<(&'static str, Option<String>)> {
        if self.is_default(state_dir) {
            return vec![(ENV_HOME, None), (ENV_LOCKS, None), (ENV_SHARED_AUTH, None)];
        }
        let text = |path: PathBuf| Some(path.to_string_lossy().into_owned());
        vec![
            (ENV_HOME, text(state_dir.to_path_buf())),
            (
                ENV_LOCKS,
                text(state_dir.join(crate::hermes_guard::LOCKS_DIR)),
            ),
            (ENV_SHARED_AUTH, text(state_dir.join(SHARED_DIR))),
            (
                ENV_POLICY,
                text(state_dir.join(crate::hermes_guard::POLICY_DIR)),
            ),
            (
                ENV_CODEX,
                text(state_dir.join(crate::hermes_guard::NO_CODEX)),
            ),
            (
                ENV_CLAUDE,
                text(state_dir.join(crate::hermes_guard::NO_CLAUDE)),
            ),
        ]
    }
    fn resume_args(&self, conversation_id: &str) -> Option<Vec<String>> {
        crate::codex::refuse_option_like(conversation_id).ok()?;
        Some(vec!["--resume".into(), conversation_id.to_owned()])
    }
}

impl ToolConfig for HermesAdapter {}

impl ToolInventory for HermesAdapter {}

impl ToolTranscripts for HermesAdapter {
    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        Ok(Box::new(crate::common::sqlite::StoreWatcher::new(
            crate::hermes_sessions::store_of(transcript_root),
            crate::hermes_sessions::session_changes,
        )))
    }
    /// The session that ran in `cwd`, from Hermes' store (read-only); its
    /// link is `<store>#<id>`.
    fn locate_transcript(
        &self,
        state_dir: &Path,
        cwd: &Path,
        since_ms: i64,
        taken: &[PathBuf],
    ) -> Option<juggler_core::model::TranscriptRef> {
        crate::hermes_sessions::launched(state_dir, cwd, since_ms, taken)
    }
    fn stored_conversations(
        &self,
        state_dir: &Path,
    ) -> Vec<juggler_adapter::conversation::StoredConversation> {
        crate::hermes_sessions::stored_conversations(&crate::hermes_sessions::store_of(state_dir))
    }
    fn subagent_support(&self) -> juggler_adapter::SubagentSupport {
        juggler_adapter::SubagentSupport::NotSupported(
            "Hermes Agent keeps a delegated task as a session of its own under its parent's; AI Juggler doesn't list them yet.",
        )
    }
}

impl ToolQuota for HermesAdapter {
    /// While the account is signed in (a home with only keys in `.env`
    /// isn't asked).
    fn plan_usage_offered(&self, state_dir: &Path) -> bool {
        hermes_login_state(state_dir) == LoginState::SignedIn
    }
    /// Hermes' own report of its provider's windows
    /// ([`crate::hermes_usage`]).
    fn plan_usage(
        &self,
        account: &Account,
    ) -> Option<Result<juggler_adapter::plan_usage::PlanReading, String>> {
        if !self.plan_usage_offered(&account.state_dir) {
            return None;
        }
        let home = &account.state_dir;
        let read = || -> Result<_, AdapterError> {
            let bin = self.require_bin()?;
            self.guard(home)?;
            Ok(bin)
        };
        Some(
            read()
                .map_err(|error| error.to_string())
                .and_then(|bin| crate::hermes_usage::usage(&bin, home, &self.account_env(home))),
        )
    }
}

impl ToolLive for HermesAdapter {
    fn session_process(
        &self,
        exe: Option<&Path>,
        args: &[String],
    ) -> Option<juggler_adapter::conversation::SessionProcess> {
        session_process(exe, args)
    }
    /// Its gateway holds the home's session store open.
    fn is_session_server(&self, exe: Option<&Path>, args: &[String]) -> bool {
        is_gateway(exe, args)
    }
}

impl ToolHandover for HermesAdapter {
    fn switch_capabilities(&self) -> juggler_adapter::switching::SwitchCapabilities {
        juggler_adapter::switching::fresh_start_only
    }
}

impl ToolRuntime for HermesAdapter {}

/// ADR-0019: Hermes Agent, described as data.
#[must_use]
pub fn hermes_spec() -> juggler_adapter::ToolSpec {
    use juggler_adapter::{SetupGroup, SignInWords, ToolFlags, ToolKind, ToolSpec, ToolTerms};
    ToolSpec {
        id: HERMES_ID.to_owned(),
        display_name: "Hermes Agent".to_owned(),
        short_name: "Hermes".to_owned(),
        vendor: "Nous Research".to_owned(),
        // Not yet checked against its owner's guidelines (brand-marks.md):
        // the letter mark until then.
        mark: Some("hermes".to_owned()),
        vendor_mark: None,
        install_url: Some(
            "https://hermes-agent.nousresearch.com/docs/getting-started/installation".to_owned(),
        ),
        install_command: Some(
            "curl -fsSL https://hermes-agent.nousresearch.com/install.sh | bash".to_owned(),
        ),
        sign_in: SignInWords {
            own: "Hermes Agent's own sign-in".to_owned(),
            short: "Hermes Agent's".to_owned(),
        },
        same_as: "as when you run Hermes Agent yourself".to_owned(),
        // Hermes Desktop (com.nousresearch.hermes) is an App through
        // `host_apps` (AJ-E1610); the tool itself stays a CLI here.
        desktop_app: None,
        app_meter: false,
        kinds: vec![ToolKind::Cli],
        rules_files: vec!["AGENTS.md".to_owned()],
        rules_import: None,
        scan_hint: "Your own Hermes home and its profiles. Nothing is changed.".to_owned(),
        setup_groups: vec![SetupGroup {
            id: HERMES_ID.to_owned(),
            title: "Hermes Agent".to_owned(),
            path_contains: None,
            origin: None,
            shows_install: true,
        }],
        default_label: Some("Hermes".to_owned()),
        flags: ToolFlags {
            first_run: true,
            primary: false,
        },
        // Nous Portal Terms of Service ("Last update: November 10, 2026"),
        // read 2026-11-13, recorded in AJ-E1518's register.
        terms: Some(ToolTerms {
            name: "Nous Portal Terms of Service".to_owned(),
            url: "https://portal.nousresearch.com/terms".to_owned(),
            accounts: "The Nous Portal terms allow one account per person, say a sign-in is never used by \
                       several people, and bar using several accounts to get more credits or capacity or to \
                       get around a usage limit."
                .to_owned(),
            checked: "2026-11-13".to_owned(),
        }),
        // Hermes updates itself (`hermes update`); no package feed.
        release_feed: None,
        update_command: Some("hermes update".to_owned()),
        // ADR-0036 §2.2: inside `ToolObserves::backed_by` its capabilities.
        observes: juggler_adapter::ToolObserves {
            cli: juggler_adapter::Observes::TERMINAL
            .union(juggler_adapter::Observes::LIMIT_HITS)
            .union(juggler_adapter::Observes::RUNWAY),
            desktop: juggler_adapter::Observes::empty(),
        },
        floor: juggler_adapter::FloorFamilies::all("neutral"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn the_version_is_read_from_its_banner() {
        assert_eq!(
            parse_version("Hermes Agent v0.22.3 (2026.9.24)\nInstall directory: /x").as_deref(),
            Some("0.22.3")
        );
        assert_eq!(parse_version("Hermes Agent"), None);
    }

    #[test]
    fn a_running_hermes_is_a_session_only_in_its_chat() {
        let wrapper = Path::new("/u/.local/bin/hermes");
        assert!(session_process(Some(wrapper), &args(&["hermes"])).is_some());
        assert!(session_process(Some(wrapper), &args(&["hermes", "chat", "-q", "hi"])).is_some());
        assert_eq!(
            session_process(
                Some(wrapper),
                &args(&["hermes", "-m", "x", "--resume", "20261113_ab"])
            )
            .and_then(|found| found.resumes)
            .as_deref(),
            Some("20261113_ab")
        );
        // Python running the wrapper's script.
        let python = Path::new("/u/.hermes/hermes-agent/venv/bin/python3");
        assert!(
            session_process(
                Some(python),
                &args(&["python3", "/u/.local/bin/hermes", "chat"])
            )
            .is_some()
        );
        for helper in [
            &["hermes", "gateway", "run", "--quiet"][..],
            &["hermes", "usage", "--json"],
            &["hermes", "--version"],
            &["hermes", "acp"],
            &["hermes", "model"],
        ] {
            assert!(
                session_process(Some(wrapper), &args(helper)).is_none(),
                "{helper:?}"
            );
        }
        assert!(session_process(Some(Path::new("/bin/hermetic")), &args(&["hermetic"])).is_none());
    }

    #[test]
    fn its_gateway_is_told_from_its_other_commands() {
        let wrapper = Path::new("/u/.local/bin/hermes");
        assert!(is_gateway(
            Some(wrapper),
            &args(&["hermes", "gateway", "run", "--quiet"])
        ));
        assert!(is_gateway(
            Some(Path::new("/usr/bin/python3")),
            &args(&[
                "python3",
                "-m",
                "hermes_cli.main",
                "--profile",
                "default",
                "gateway",
                "run"
            ])
        ));
        assert!(!is_gateway(
            Some(wrapper),
            &args(&["hermes", "gateway", "stop"])
        ));
        assert!(!is_gateway(Some(wrapper), &args(&["hermes"])));
    }
}
