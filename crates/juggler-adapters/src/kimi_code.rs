//! Kimi Code adapter (AJ-E1603) — Moonshot's `kimi` CLI, 2.x.
//!
//! Built from the research (`docs/research/kimi.md`, the vendor's docs and
//! the MIT `MoonshotAI/kimi-code` source at 2.3.0, read 2026-10-21); not
//! yet run live — that is the maintainer's check on a spare account
//! (AJ-E1612, `docs/internal/MAINTAINER-STEPS.md`). Every version is
//! therefore "unverified", never a pretended check.
//!
//! Isolation model: `KIMI_CODE_HOME` moves all of Kimi Code's data —
//! settings, sign-ins, sessions, logs, its global `AGENTS.md`, skills — so
//! each account gets its own home, pinned after Launch-settings overrides.
//! Its default home, `~/.kimi-code`, runs with the variable unset, as a
//! plain `kimi` does. The cross-tool `~/.agents/` (skills, `AGENTS.md`)
//! stays shared by design. Kimi Code reads no provider key from the
//! environment unless its settings name one (`api_key_env`), so no
//! inherited key needs removing. `KIMI_CODE_IDENTITY_*` (the client's
//! identity) is never set: its guidelines forbid altering it.
//!
//! Its terms (AJ-E1603 Req.4): "Kimi Code subscriptions are for
//! interactive use only". An account signed in with its membership (or
//! whose sign-in can't be told) is [`HarnessAdapter::interactive_only`]:
//! its headless runs are refused here, and the app never restarts it
//! without a click. An account on a Kimi Platform API key isn't limited.
//!
//! Sign-in: Kimi Code's own `kimi login` (a device code in the browser),
//! in a terminal. Only which sign-in files are there is read
//! (`sign_in.rs`), never one's contents. Usage isn't read: the only
//! documented quota source is `kimi web`'s local server, reached with a
//! token from the home and answering from Moonshot's servers — outside
//! what AI Juggler reads or sends today (the maintainer's ruling).

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{DiscoveredInstall, ImportOutcome, LoginProbe};
use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, HarnessAdapter, InstallStatus, LaunchMode,
    LaunchSpec, SessionWatcher, VerificationStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

use crate::claude_code::{LoginState, existing, find_on_path, home_dir, prepare_managed_dir_for};

/// Provider id: `kimi-code` (not `kimi`: the archived Python CLI was
/// called that too).
pub const KIMI_CODE_ID: &str = "kimi-code";

/// No version has been run live yet (AJ-E1612): every one is unverified.
const VERIFIED_AGAINST: &[&str] = &[];

/// The version whose docs and source this adapter was built from.
pub const SOURCE_READ: &[&str] = &["2.3.0"];

/// Kimi Code's home.
pub const ENV_HOME: &str = "KIMI_CODE_HOME";
/// Optional test/dev override for the binary path.
const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_KIMI_BIN";

const PINNED: &[&str] = &[ENV_HOME];
const ISOLATION: &[&str] = &[ENV_HOME, ENV_BIN_OVERRIDE];

/// Why a membership account is never run without the user.
const INTERACTIVE_ONLY: &str = "Kimi Code's plans are for interactive use only, so AI Juggler \
     only starts this account when you do";

/// `kimi --version` prints the version, maybe after the name. The version
/// is the last word of the first line, a leading `v` dropped [unverified:
/// the output isn't documented].
#[must_use]
pub fn parse_version(stdout: &str) -> Option<String> {
    let token = stdout.lines().next()?.split_whitespace().last()?;
    let token = token.strip_prefix('v').unwrap_or(token);
    token
        .starts_with(|c: char| c.is_ascii_digit())
        .then(|| token.to_owned())
}

/// Whether a version is Kimi Code (2.x and later); 0.x and 1.x are the
/// archived Python Kimi CLI, which used the same command name.
#[must_use]
pub fn is_kimi_code(version: &str) -> bool {
    version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .is_some_and(|major| major >= 2)
}

/// Adapter for the Kimi Code CLI.
#[derive(Debug, Clone, Default)]
pub struct KimiCodeAdapter {
    bin: Option<PathBuf>,
    /// The user's home, for the default home and the installer's folder
    /// (tests/fixtures; `None` reads `$HOME`).
    home: Option<PathBuf>,
}

impl KimiCodeAdapter {
    /// Creates the adapter, resolving the binary at call time.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The adapter pinned to a specific binary (tests/fixtures).
    #[must_use]
    pub fn with_bin(bin: impl Into<PathBuf>) -> Self {
        Self {
            bin: Some(bin.into()),
            home: None,
        }
    }

    /// The adapter using `home` as the user's home (tests/fixtures).
    #[must_use]
    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }

    fn user_home(&self) -> Option<PathBuf> {
        self.home.clone().or_else(home_dir)
    }

    /// Kimi Code's own default home, `~/.kimi-code`.
    fn default_home(&self) -> Option<PathBuf> {
        self.user_home().map(|home| home.join(".kimi-code"))
    }

    /// Whether `dir` is the default home, by its exact path.
    fn is_default(&self, dir: &Path) -> bool {
        self.default_home().is_some_and(|default| {
            Path::new(dir.to_string_lossy().trim_end_matches('/')) == default
        })
    }

    /// The installer's own folder first (`~/.kimi-code/bin/kimi`), so an
    /// old Python `kimi` earlier on the PATH isn't taken for it; then the
    /// PATH, Homebrew and npm's global folders.
    fn resolve_bin(&self) -> Option<PathBuf> {
        if let Some(bin) = &self.bin {
            return Some(bin.clone());
        }
        if let Some(path) = std::env::var_os(ENV_BIN_OVERRIDE).map(PathBuf::from)
            && path.is_file()
        {
            return Some(path);
        }
        let home = self.user_home();
        home.as_ref()
            .map(|home| home.join(".kimi-code/bin/kimi"))
            .filter(|path| path.is_file())
            .or_else(|| find_on_path("kimi"))
            .or_else(|| existing(&["/opt/homebrew/bin/kimi", "/usr/local/bin/kimi"]))
            .or_else(|| {
                home.map(|home| home.join(".npm-global/bin/kimi"))
                    .filter(|path| path.is_file())
            })
    }

    fn require_bin(&self) -> Result<PathBuf, AdapterError> {
        self.resolve_bin().ok_or_else(|| {
            AdapterError::InvalidLaunch("kimi binary not found; run discovery for details".into())
        })
    }

    fn report(
        &self,
        bin: Option<PathBuf>,
        install: InstallStatus,
        notes: Vec<String>,
    ) -> DiscoveryReport {
        DiscoveryReport {
            adapter: self.id(),
            install,
            verification: VerificationStatus::Unverified,
            binary: bin,
            notes,
        }
    }

    /// The pinned home for a launch or the sign-in flow.
    fn isolation_env(&self, state_dir: &Path) -> EnvMap {
        self.pinned_env(state_dir)
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_owned(),
                    value.unwrap_or_else(|| ENV_UNSET.to_owned()),
                )
            })
            .collect()
    }
}

/// Sign-in state of a Kimi Code home, from which sign-in files are there
/// (never their contents).
#[must_use]
pub fn kimi_login_state(home: &Path) -> LoginState {
    match crate::sign_in::kimi_sign_in(home) {
        None => LoginState::Unknown,
        Some(crate::sign_in::KimiSignIn::Membership) => LoginState::SignedIn,
        Some(crate::sign_in::KimiSignIn::Other) => LoginState::ApiKey,
        Some(crate::sign_in::KimiSignIn::None) => LoginState::LoggedOut,
    }
}

fn scan_probe(home: &Path) -> LoginProbe {
    match kimi_login_state(home) {
        LoginState::SignedIn | LoginState::ApiKey => LoginProbe::LoggedIn,
        LoginState::Partial
        | LoginState::Expired
        | LoginState::SsoPending
        | LoginState::LoggedOut
        | LoginState::Unknown => LoginProbe::LoggedOut,
    }
}

/// `kimi` subcommands that aren't a session: its server, sign-in and
/// upkeep (research §1, §4, §6).
const HELPERS: &[&str] = &[
    "web", "acp", "login", "logout", "upgrade", "update", "doctor", "export", "vis", "app",
    "provider", "migrate", "mcp",
];

/// A `kimi` CLI (the installer's binary, or the npm package run by node)
/// is a session unless it runs a helper. `--session <id>` (`-S`, and the
/// hidden `--resume`/`-r`) names its conversation.
#[must_use]
pub fn session_process(
    exe: Option<&Path>,
    args: &[String],
) -> Option<juggler_adapter::conversation::SessionProcess> {
    let cli = exe.is_some_and(|exe| exe.file_name().is_some_and(|name| name == "kimi"))
        || args
            .iter()
            .any(|arg| arg.contains("@moonshot-ai/kimi-code"));
    let helper = args
        .iter()
        .skip(1)
        .any(|arg| HELPERS.contains(&arg.as_str()));
    (cli && !helper).then(|| juggler_adapter::conversation::SessionProcess {
        resumes: juggler_adapter::conversation::value_after(
            args,
            &["--session", "-S", "--resume", "-r"],
        ),
    })
}

impl HarnessAdapter for KimiCodeAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(KIMI_CODE_ID)
    }

    fn spec(&self) -> juggler_adapter::ToolSpec {
        kimi_code_spec()
    }

    /// Headless runs only for an account on a Kimi Platform API key (see
    /// [`HarnessAdapter::interactive_only`]); no usage reader.
    fn capabilities(&self) -> Capabilities {
        Capabilities::HEADLESS | Capabilities::RESUME | Capabilities::TRANSCRIPT_WATCH
    }

    fn verified_against(&self) -> &'static [&'static str] {
        VERIFIED_AGAINST
    }

    fn discover(&self) -> DiscoveryReport {
        let Some(bin) = self.resolve_bin().filter(|bin| bin.is_file()) else {
            return self.report(
                None,
                InstallStatus::NotFound,
                vec!["looked in ~/.kimi-code/bin, on PATH, /opt/homebrew/bin, /usr/local/bin, ~/.npm-global/bin".into()],
            );
        };
        // With the user's PATH: the npm package runs on `node`.
        let output = match juggler_adapter::user_path::command(&bin)
            .arg("--version")
            .output()
        {
            Ok(output) if output.status.success() => output,
            Ok(output) => {
                return self.report(
                    Some(bin),
                    InstallStatus::Failed {
                        message: format!("--version exited with {}", output.status),
                    },
                    Vec::new(),
                );
            }
            Err(error) => {
                return self.report(
                    Some(bin),
                    InstallStatus::Failed {
                        message: error.to_string(),
                    },
                    Vec::new(),
                );
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let Some(version) = parse_version(&stdout) else {
            return self.report(
                Some(bin),
                InstallStatus::Failed {
                    message: format!("unrecognised --version output: {stdout:?}"),
                },
                Vec::new(),
            );
        };
        if !is_kimi_code(&version) {
            // The archived Python Kimi CLI: Kimi says it will stop working.
            return self.report(
                Some(bin),
                InstallStatus::NotFound,
                vec![format!(
                    "this kimi is the old Kimi CLI ({version}), which Kimi no longer supports; \
                     install Kimi Code"
                )],
            );
        }
        self.report(
            Some(bin),
            InstallStatus::Installed { version },
            vec!["Kimi Code 2 — built from the 2.3.0 docs and source; not yet checked live".into()],
        )
    }

    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        prepare_managed_dir_for(state_dir, KIMI_CODE_ID)?;
        Ok(self
            .pinned_env(state_dir)
            .into_iter()
            .filter_map(|(key, value)| Some((key.to_owned(), value?)))
            .collect())
    }

    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        let bin = self.require_bin()?;
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
        let mut args: Vec<String> = Vec::new();
        match mode {
            LaunchMode::Interactive => {}
            LaunchMode::Resume {
                session_id,
                delta_prompt,
            } => {
                crate::codex::refuse_option_like(session_id)?;
                if delta_prompt.is_some() {
                    return Err(AdapterError::Unsupported(
                        "Kimi Code can't be given a message as it resumes yet".into(),
                    ));
                }
                args.extend(["--session".into(), session_id.clone()]);
            }
            // Its terminal takes no first message that's documented.
            LaunchMode::Primed { .. } => {
                return Err(AdapterError::Unsupported(
                    "Kimi Code can't be started with a first message yet".into(),
                ));
            }
            LaunchMode::Headless { prompt } => {
                if let Some(why) = self.interactive_only(&account.state_dir) {
                    return Err(AdapterError::Unsupported(why));
                }
                args.extend([
                    "-p".into(),
                    prompt.clone(),
                    "--output-format".into(),
                    "stream-json".into(),
                ]);
            }
        }
        if let Some(model) = &profile.settings_overlay.model {
            args.extend(["-m".into(), model.clone()]);
        }
        Ok(LaunchSpec {
            program: bin,
            args,
            env,
            cwd: effective_cwd,
            needs_pty: !matches!(mode, LaunchMode::Headless { .. }),
        })
    }

    fn login_state(&self, state_dir: &Path) -> LoginState {
        kimi_login_state(state_dir)
    }

    /// Anything but an API-key account: a membership (or a sign-in that
    /// can't be told) is for interactive use only.
    fn interactive_only(&self, state_dir: &Path) -> Option<String> {
        (kimi_login_state(state_dir) != LoginState::ApiKey).then(|| INTERACTIVE_ONLY.to_owned())
    }

    /// The vendor's own `kimi login` (a device code in the browser) in
    /// this account's home.
    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        Ok(LaunchSpec {
            program: self.require_bin()?,
            args: vec!["login".into()],
            env: self.isolation_env(state_dir),
            cwd: state_dir.to_path_buf(),
            needs_pty: true,
        })
    }

    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        crate::claude_code::ClaudeTranscriptWatcher::with_ids(
            transcript_root,
            crate::kimi_sessions::session_id_of,
        )
        .map(|watcher| Box::new(watcher) as Box<dyn SessionWatcher>)
    }

    fn locate_transcript(
        &self,
        state_dir: &Path,
        cwd: &Path,
        since_ms: i64,
        taken: &[PathBuf],
    ) -> Option<juggler_core::model::TranscriptRef> {
        crate::kimi_sessions::locate(state_dir, cwd, since_ms, taken).map(|(path, id)| {
            juggler_core::model::TranscriptRef {
                path,
                native_id: Some(id),
            }
        })
    }

    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        let Some(home) = self.user_home() else {
            return Vec::new();
        };
        let env_override = std::env::var_os(ENV_HOME)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        crate::kimi_environment::scan_kimi_environment(&home, env_override.as_deref(), &scan_probe)
    }

    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::kimi_environment::import_kimi_home(source, dest, &scan_probe, false)
    }

    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        crate::kimi_environment::import_kimi_home(source, dest, &scan_probe, true)
    }

    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        crate::kimi_environment::inspect_kimi_source(source, &scan_probe)
    }

    fn can_use_in_place(&self) -> bool {
        true
    }

    fn vendor_default_dir(&self) -> Option<PathBuf> {
        self.default_home()
    }

    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::kimi_environment::adopt_kimi_home(source, &scan_probe)
    }

    fn isolation_env_keys(&self) -> &'static [&'static str] {
        ISOLATION
    }

    fn pinned_env_keys(&self) -> &'static [&'static str] {
        PINNED
    }

    /// `None` for the default `~/.kimi-code`: it runs with
    /// `KIMI_CODE_HOME` unset, as a plain `kimi` does.
    fn isolation_env_value(&self, state_dir: &Path) -> Option<String> {
        (!self.is_default(state_dir)).then(|| state_dir.to_string_lossy().into_owned())
    }

    fn shell_command(&self) -> Option<&'static str> {
        Some("kimi")
    }

    fn resolved_binary(&self) -> Option<PathBuf> {
        self.resolve_bin()
    }

    fn resume_args(&self, conversation_id: &str) -> Option<Vec<String>> {
        crate::codex::refuse_option_like(conversation_id).ok()?;
        Some(vec!["--session".into(), conversation_id.to_owned()])
    }

    fn session_process(
        &self,
        exe: Option<&Path>,
        args: &[String],
    ) -> Option<juggler_adapter::conversation::SessionProcess> {
        session_process(exe, args)
    }

    // AJ-E1520 (0.2): its subagents, from its own records.
    fn subagent_support(&self) -> juggler_adapter::SubagentSupport {
        crate::kimi_subagents::SUPPORT
    }

    fn subagent_reader(
        &self,
        scope: juggler_adapter::SubagentScope<'_>,
    ) -> Option<Box<dyn juggler_adapter::SubagentReader>> {
        crate::kimi_subagents::reader(scope)
    }

    fn conversation_files(&self, state_dir: &Path) -> Vec<PathBuf> {
        crate::kimi_sessions::conversation_files(state_dir)
    }

    fn read_conversation(
        &self,
        path: &Path,
    ) -> Option<juggler_adapter::conversation::ConversationSummary> {
        crate::kimi_sessions::read_conversation(path)
    }

    fn inventory(
        &self,
        state_dir: &Path,
        _project: Option<&Path>,
    ) -> Vec<juggler_adapter::inventory::Item> {
        crate::kimi_inventory::inventory(state_dir)
    }

    fn can_set_up(
        &self,
        item: &juggler_adapter::inventory::Item,
    ) -> juggler_adapter::inventory::SetUp {
        crate::kimi_inventory::can_set_up(item)
    }

    fn set_up(
        &self,
        state_dir: &Path,
        _project: Option<&Path>,
        item: &juggler_adapter::inventory::Item,
    ) -> Result<juggler_adapter::inventory::Applied, AdapterError> {
        crate::kimi_inventory::set_up(state_dir, item)
    }
}

/// ADR-0019: Kimi Code, described as data.
#[must_use]
pub fn kimi_code_spec() -> juggler_adapter::ToolSpec {
    use juggler_adapter::{SetupGroup, SignInWords, ToolFlags, ToolKind, ToolSpec, ToolTerms};
    ToolSpec {
        id: KIMI_CODE_ID.to_owned(),
        display_name: "Kimi Code".to_owned(),
        short_name: "Kimi".to_owned(),
        vendor: "Moonshot AI".to_owned(),
        // Not yet checked against its owner's guidelines (brand-marks.md):
        // the letter mark until then.
        mark: Some("kimi".to_owned()),
        vendor_mark: None,
        install_url: Some("https://www.kimi.com/code/docs/en/".to_owned()),
        install_command: Some("npm install -g @moonshot-ai/kimi-code".to_owned()),
        sign_in: SignInWords {
            own: "Kimi Code's own sign-in".to_owned(),
            short: "Kimi Code's".to_owned(),
        },
        same_as: "as when you run Kimi Code yourself".to_owned(),
        // Kimi Code Desktop shares the default home but can't be started
        // per account (research §10): tracked by AJ-E1610, not here.
        desktop_app: None,
        kinds: vec![ToolKind::Cli],
        rules_files: vec!["AGENTS.md".to_owned()],
        rules_import: None,
        scan_hint: "Your default setup and anything else that looks like one. Nothing is changed."
            .to_owned(),
        setup_groups: vec![SetupGroup {
            id: KIMI_CODE_ID.to_owned(),
            title: "Kimi Code".to_owned(),
            path_contains: None,
            origin: None,
            shows_install: true,
        }],
        default_label: Some("Kimi Code".to_owned()),
        flags: ToolFlags {
            first_run: true,
            primary: false,
        },
        // Research `kimi.md` §7 (accessed 2026-10-21), recorded in
        // AJ-E1518's register.
        terms: Some(ToolTerms {
            name: "Kimi's Terms of Service".to_owned(),
            url: "https://www.kimi.ai/user/agreement/modelUse".to_owned(),
            accounts: "Kimi's Terms of Service let it freeze accounts registered or used \
                       improperly as several, and its Kimi Code guidelines say plans are for \
                       interactive use only."
                .to_owned(),
            checked: "2026-10-21".to_owned(),
        }),
        // AJ-E1611: npm's `latest` [the tool's own published package].
        release_feed: Some(juggler_adapter::ReleaseFeed::Npm {
            package: "@moonshot-ai/kimi-code".to_owned(),
        }),
        update_command: Some("kimi upgrade".to_owned()),
    }
}
