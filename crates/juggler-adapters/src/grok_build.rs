//! Grok Build adapter (AJ-E1604) — xAI's `grok` CLI.
//!
//! Built from the research (`docs/research/grok.md`: xAI's docs and the
//! Apache-2.0 `xai-org/grok-build` source, crate 1.0.45, read 2026-10-21);
//! not yet run live — that is the maintainer's check on a spare account
//! (AJ-E1612). Every version is therefore "unverified".
//!
//! Detection by path and kind (Req.1): the installer's `~/.grok/bin/grok`
//! first; a `grok` elsewhere only when it isn't a Node script — the
//! unofficial `grok-cli` (npm `grok-dev`) is one by the same name.
//!
//! Isolation: `GROK_HOME` per account, pinned after Launch-settings
//! overrides (its default `~/.grok` runs with it unset). `GROK_AUTH_PATH`
//! (would move the sign-in out of the home) and an inherited
//! `XAI_API_KEY` (would bill every account to one key) are removed from
//! every launch.
//!
//! Another tool's folders (Req.2): Grok reads the user's real `~/.claude`
//! and `~/.cursor` (rules, hooks, MCP servers, skills, agents) whatever
//! `GROK_HOME` says. Every launch, and the copyable line, sets
//! `GROK_CLAUDE_<X>_ENABLED=0` and `GROK_CURSOR_<X>_ENABLED=0` for each of
//! them, so a Claude hook never runs inside Grok
//! ([`HarnessAdapter::launch_fixed_env`]). Its Codex compatibility
//! (`[compat.codex]`) has no documented variable; it reads skills and
//! hooks, not a sign-in (on the live checklist).
//!
//! Usage (Req.4): the weekly pool has no sanctioned source, and the
//! per-session `signals.json` fields aren't documented, so no usage is
//! read yet (on the live checklist); its limits say so.

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{DiscoveredInstall, ImportOutcome, LoginProbe};
use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, HarnessAdapter, InstallStatus, LaunchMode,
    LaunchSpec, SessionWatcher, VerificationStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

use crate::claude_code::{LoginState, existing, find_on_path, home_dir, prepare_managed_dir_for};

/// Provider id: `grok-build` (the unofficial CLI is also called `grok`).
pub const GROK_BUILD_ID: &str = "grok-build";

/// No version has been run live yet (AJ-E1612): every one is unverified.
const VERIFIED_AGAINST: &[&str] = &[];

/// Grok Build's home.
pub const ENV_HOME: &str = "GROK_HOME";
/// Would move the sign-in file out of the home.
const ENV_AUTH_PATH: &str = "GROK_AUTH_PATH";
/// Optional test/dev override for the binary path.
const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_GROK_BIN";

/// Every launch sets these to `0`: Grok reads none of the user's Claude
/// or Cursor setup (research §2, `UG/26-config-reference.md`).
const FIXED: &[(&str, &str)] = &[
    ("GROK_CLAUDE_AGENTS_ENABLED", "0"),
    ("GROK_CLAUDE_HOOKS_ENABLED", "0"),
    ("GROK_CLAUDE_MCPS_ENABLED", "0"),
    ("GROK_CLAUDE_RULES_ENABLED", "0"),
    ("GROK_CLAUDE_SKILLS_ENABLED", "0"),
    ("GROK_CURSOR_AGENTS_ENABLED", "0"),
    ("GROK_CURSOR_HOOKS_ENABLED", "0"),
    ("GROK_CURSOR_MCPS_ENABLED", "0"),
    ("GROK_CURSOR_RULES_ENABLED", "0"),
    ("GROK_CURSOR_SKILLS_ENABLED", "0"),
];

const PINNED: &[&str] = &[ENV_HOME];
const ISOLATION: &[&str] = &[
    ENV_HOME,
    ENV_AUTH_PATH,
    ENV_BIN_OVERRIDE,
    "GROK_CLAUDE_AGENTS_ENABLED",
    "GROK_CLAUDE_HOOKS_ENABLED",
    "GROK_CLAUDE_MCPS_ENABLED",
    "GROK_CLAUDE_RULES_ENABLED",
    "GROK_CLAUDE_SKILLS_ENABLED",
    "GROK_CURSOR_AGENTS_ENABLED",
    "GROK_CURSOR_HOOKS_ENABLED",
    "GROK_CURSOR_MCPS_ENABLED",
    "GROK_CURSOR_RULES_ENABLED",
    "GROK_CURSOR_SKILLS_ENABLED",
];
/// Removed from every launch (see the module docs).
const UNSET: &[&str] = &[ENV_AUTH_PATH, "XAI_API_KEY"];

/// `grok --version` prints the version, maybe after the name; the last
/// word of the first line, a leading `v` dropped (unverified).
#[must_use]
pub fn parse_version(stdout: &str) -> Option<String> {
    let token = stdout.lines().next()?.split_whitespace().last()?;
    let token = token.strip_prefix('v').unwrap_or(token);
    token
        .starts_with(|c: char| c.is_ascii_digit())
        .then(|| token.to_owned())
}

/// Whether `path` is a Node or Bun script (its first line `#!…node` or
/// `#!…bun`): the unofficial `grok-cli` is one (npm `grok-dev`); the
/// official `grok` is a compiled program.
#[must_use]
pub fn is_node_script(path: &Path) -> bool {
    use std::io::Read as _;
    let mut head = Vec::new();
    let read = std::fs::File::open(path).and_then(|file| file.take(128).read_to_end(&mut head));
    let first = String::from_utf8_lossy(&head);
    let first = first.lines().next().unwrap_or_default();
    read.is_ok() && first.starts_with("#!") && (first.contains("node") || first.contains("bun"))
}

/// Adapter for the Grok Build CLI.
#[derive(Debug, Clone, Default)]
pub struct GrokBuildAdapter {
    bin: Option<PathBuf>,
    home: Option<PathBuf>,
}

impl GrokBuildAdapter {
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

    fn default_home(&self) -> Option<PathBuf> {
        self.user_home().map(|home| home.join(".grok"))
    }

    fn is_default(&self, dir: &Path) -> bool {
        self.default_home().is_some_and(|default| {
            Path::new(dir.to_string_lossy().trim_end_matches('/')) == default
        })
    }

    /// The installer's `~/.grok/bin/grok` first, then a `grok` on the
    /// PATH or in Homebrew's folders that isn't a Node script (see the
    /// module docs).
    fn resolve_bin(&self) -> Option<PathBuf> {
        if let Some(bin) = &self.bin {
            return Some(bin.clone());
        }
        if let Some(path) = std::env::var_os(ENV_BIN_OVERRIDE).map(PathBuf::from)
            && path.is_file()
        {
            return Some(path);
        }
        self.user_home()
            .map(|home| home.join(".grok/bin/grok"))
            .filter(|path| path.is_file())
            .or_else(|| find_on_path("grok").filter(|path| !is_node_script(path)))
            .or_else(|| {
                existing(&["/opt/homebrew/bin/grok", "/usr/local/bin/grok"])
                    .filter(|path| !is_node_script(path))
            })
    }

    fn require_bin(&self) -> Result<PathBuf, AdapterError> {
        self.resolve_bin().ok_or_else(|| {
            AdapterError::InvalidLaunch("grok binary not found; run discovery for details".into())
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

    /// The pinned home, the removed variables and the fixed ones.
    fn isolation_env(&self, state_dir: &Path) -> EnvMap {
        let mut env: EnvMap = self
            .pinned_env(state_dir)
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_owned(),
                    value.unwrap_or_else(|| ENV_UNSET.to_owned()),
                )
            })
            .collect();
        for key in UNSET {
            env.insert((*key).to_owned(), ENV_UNSET.to_owned());
        }
        for (key, value) in FIXED {
            env.insert((*key).to_owned(), (*value).to_owned());
        }
        env
    }
}

/// Sign-in state of a Grok Build home, from each `auth.json` entry's mode
/// (never a value).
#[must_use]
pub fn grok_login_state(home: &Path) -> LoginState {
    if std::fs::read_dir(home).is_err() {
        return LoginState::Unknown;
    }
    match crate::sign_in::grok_auth(home) {
        crate::sign_in::GrokAuth::Missing => LoginState::LoggedOut,
        crate::sign_in::GrokAuth::Empty => LoginState::Partial,
        crate::sign_in::GrokAuth::Unreadable => LoginState::Unknown,
        crate::sign_in::GrokAuth::Modes(modes) => {
            if modes
                .iter()
                .any(|mode| matches!(mode.as_str(), "web_login" | "oidc" | "external"))
            {
                LoginState::SignedIn
            } else if modes.iter().any(|mode| mode == "api_key") {
                LoginState::ApiKey
            } else {
                LoginState::LoggedOut
            }
        }
    }
}

fn scan_probe(home: &Path) -> LoginProbe {
    match grok_login_state(home) {
        LoginState::SignedIn | LoginState::ApiKey => LoginProbe::LoggedIn,
        LoginState::Partial
        | LoginState::Expired
        | LoginState::SsoPending
        | LoginState::LoggedOut
        | LoginState::Unknown => LoginProbe::LoggedOut,
    }
}

/// `grok` subcommands that aren't a session (research §1–§6).
const HELPERS: &[&str] = &[
    "login",
    "logout",
    "update",
    "version",
    "inspect",
    "sessions",
    "export",
    "usage",
    "import",
    "mcp",
    "agent",
    "dashboard",
    "clone",
    "wrap",
];

/// A `grok` program is a session (the terminal, `-p`) unless it runs a
/// helper. `--resume <id>` (`-r`) names its conversation.
#[must_use]
pub fn session_process(
    exe: Option<&Path>,
    args: &[String],
) -> Option<juggler_adapter::conversation::SessionProcess> {
    let cli = exe.is_some_and(|exe| exe.file_name().is_some_and(|name| name == "grok"));
    let helper = args
        .iter()
        .skip(1)
        .any(|arg| HELPERS.contains(&arg.as_str()));
    (cli && !helper).then(|| juggler_adapter::conversation::SessionProcess {
        resumes: juggler_adapter::conversation::value_after(args, &["--resume", "-r"]),
    })
}

impl HarnessAdapter for GrokBuildAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(GROK_BUILD_ID)
    }

    fn spec(&self) -> juggler_adapter::ToolSpec {
        grok_build_spec()
    }

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
                vec!["looked in ~/.grok/bin, on PATH, /opt/homebrew/bin, /usr/local/bin (a Node script named grok is the unofficial grok-cli, not Grok Build)".into()],
            );
        };
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
        self.report(
            Some(bin),
            InstallStatus::Installed { version },
            vec!["Grok Build — built from the 1.0.45 docs and source; not yet checked live".into()],
        )
    }

    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        prepare_managed_dir_for(state_dir, GROK_BUILD_ID)?;
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
                        "Grok can't be given a message as it resumes yet".into(),
                    ));
                }
                args.extend(["--resume".into(), session_id.clone()]);
            }
            LaunchMode::Primed { .. } => {
                return Err(AdapterError::Unsupported(
                    "Grok can't be started with a first message yet".into(),
                ));
            }
            LaunchMode::Headless { prompt } => {
                args.extend([
                    "-p".into(),
                    prompt.clone(),
                    "--output-format".into(),
                    "json".into(),
                ]);
            }
        }
        if let Some(model) = &profile.settings_overlay.model {
            args.extend(["--model".into(), model.clone()]);
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
        grok_login_state(state_dir)
    }

    /// Grok's own `grok login` (the browser) in this account's home.
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
            crate::grok_sessions::session_id_of,
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
        crate::grok_sessions::locate(state_dir, cwd, since_ms, taken).map(|(path, id)| {
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
        crate::grok_environment::scan_grok_environment(&home, env_override.as_deref(), &scan_probe)
    }

    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::grok_environment::import_grok_home(source, dest, &scan_probe, false)
    }

    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        crate::grok_environment::import_grok_home(source, dest, &scan_probe, true)
    }

    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        crate::grok_environment::inspect_grok_source(source, &scan_probe)
    }

    fn can_use_in_place(&self) -> bool {
        true
    }

    fn vendor_default_dir(&self) -> Option<PathBuf> {
        self.default_home()
    }

    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::grok_environment::adopt_grok_home(source, &scan_probe)
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

    fn launch_fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        FIXED
    }

    fn isolation_env_value(&self, state_dir: &Path) -> Option<String> {
        (!self.is_default(state_dir)).then(|| state_dir.to_string_lossy().into_owned())
    }

    fn shell_command(&self) -> Option<&'static str> {
        Some("grok")
    }

    fn resolved_binary(&self) -> Option<PathBuf> {
        self.resolve_bin()
    }

    fn resume_args(&self, conversation_id: &str) -> Option<Vec<String>> {
        crate::codex::refuse_option_like(conversation_id).ok()?;
        Some(vec!["--resume".into(), conversation_id.to_owned()])
    }

    fn session_process(
        &self,
        exe: Option<&Path>,
        args: &[String],
    ) -> Option<juggler_adapter::conversation::SessionProcess> {
        session_process(exe, args)
    }

    fn conversation_files(&self, state_dir: &Path) -> Vec<PathBuf> {
        crate::grok_sessions::conversation_files(state_dir)
    }

    fn read_conversation(
        &self,
        path: &Path,
    ) -> Option<juggler_adapter::conversation::ConversationSummary> {
        crate::grok_sessions::read_conversation(path)
    }

    fn inventory(
        &self,
        state_dir: &Path,
        _project: Option<&Path>,
    ) -> Vec<juggler_adapter::inventory::Item> {
        crate::grok_inventory::inventory(state_dir)
    }

    fn can_set_up(
        &self,
        item: &juggler_adapter::inventory::Item,
    ) -> juggler_adapter::inventory::SetUp {
        crate::grok_inventory::can_set_up(item)
    }

    fn set_up(
        &self,
        state_dir: &Path,
        _project: Option<&Path>,
        item: &juggler_adapter::inventory::Item,
    ) -> Result<juggler_adapter::inventory::Applied, AdapterError> {
        crate::grok_inventory::set_up(state_dir, item)
    }
}

/// ADR-0019: Grok Build, described as data.
#[must_use]
pub fn grok_build_spec() -> juggler_adapter::ToolSpec {
    use juggler_adapter::{SetupGroup, SignInWords, ToolFlags, ToolKind, ToolSpec, ToolTerms};
    ToolSpec {
        id: GROK_BUILD_ID.to_owned(),
        display_name: "Grok Build".to_owned(),
        short_name: "Grok".to_owned(),
        vendor: "xAI".to_owned(),
        // Not yet checked against its owner's guidelines (brand-marks.md).
        mark: Some("grok".to_owned()),
        vendor_mark: None,
        install_url: Some("https://docs.x.ai/build/overview".to_owned()),
        install_command: Some("curl -fsSL https://x.ai/cli/install.sh | bash".to_owned()),
        sign_in: SignInWords {
            own: "Grok's own sign-in".to_owned(),
            short: "Grok's".to_owned(),
        },
        same_as: "as when you run Grok yourself".to_owned(),
        desktop_app: None,
        app_meter: false,
        kinds: vec![ToolKind::Cli],
        rules_files: vec!["AGENTS.md".to_owned()],
        rules_import: None,
        scan_hint: "Your default setup and anything else that looks like one. Nothing is changed."
            .to_owned(),
        setup_groups: vec![SetupGroup {
            id: GROK_BUILD_ID.to_owned(),
            title: "Grok Build".to_owned(),
            path_contains: None,
            origin: None,
            shows_install: true,
        }],
        default_label: Some("Grok".to_owned()),
        flags: ToolFlags {
            first_run: true,
            primary: false,
        },
        // Research `grok.md` §7 (the vendor's pages, read 2026-10-21),
        // recorded in AJ-E1518's register.
        terms: Some(ToolTerms {
            name: "xAI's Terms of Service".to_owned(),
            url: "https://x.ai/legal/terms-of-service".to_owned(),
            accounts: "xAI's Terms of Service say not to share an account, and its usage pool \
                       is shared with Cursor and Grok on the X Subscription."
                .to_owned(),
            checked: "2026-10-21".to_owned(),
        }),
        // Grok Build updates itself (`grok update`); no public package
        // list is read for it.
        release_feed: None,
        update_command: Some("grok update".to_owned()),
    }
}
