//! OpenCode adapter (AJ-E1602) — the `opencode` CLI, lines 1.x and 2.x.
//!
//! Built from OpenCode's source at v1.19.3 and v2.1.2 (research
//! `docs/research/opencode.md`); not yet run live — that is the
//! maintainer's check on a spare account (AJ-E1612, checklist in
//! `docs/MAINTAINER-STEPS.md`). Every version therefore reports
//! "unverified", never a pretended check.
//!
//! Isolation model: OpenCode takes every state path from the XDG base
//! folders (`packages/core/src/global.ts` at v1.19.3; `packages/util/src/
//! global-roots.ts` at v2.1.2). Each managed account gets its own
//!
//! - `XDG_DATA_HOME` = `<account>/data` — sign-ins (`auth.json`, the v2
//!   `credential` table) and sessions (`opencode.db`);
//! - `XDG_CONFIG_HOME` = `<account>/config` — settings, rules, plugins;
//! - `XDG_STATE_HOME` = `<account>/state` — preferences, history, locks;
//! - `OPENCODE_CONFIG_DIR` = `<account>/config/opencode` — v1 loads this
//!   folder after the global one, v2 uses it instead of it; pinned so an
//!   exported value never adds another account's settings.
//!
//! All four are pinned after Launch-settings overrides. The cache
//! (`XDG_CACHE_HOME`: downloaded provider packages and tools) is shared,
//! as rebuildable. `OPENCODE_DB` (moves the database) and
//! `OPENCODE_TEST_HOME` are removed from every launch, as are inherited
//! sign-ins: `OPENCODE_AUTH_CONTENT` (replaces `auth.json` outright) and
//! the provider keys OpenCode reads from the environment — an inherited
//! key would bill every account to it. Keys belong in each account's own
//! OpenCode sign-in (`opencode auth login`).
//!
//! The XDG variables reach the tools the agent runs, too (a `gh` in an
//! OpenCode session looks for its settings under the account's
//! `config/`) — a known trade-off of XDG isolation, on the live
//! checklist.
//!
//! In place (ADR-0018): OpenCode's default data folder,
//! `~/.local/share/opencode`, launches with every pinned variable unset,
//! as a plain `opencode` runs. Any other folder comes in as a copy.
//!
//! Sessions and usage come from the files (`opencode.db`), read-only;
//! `opencode serve` (Req.4) is never started: nothing the app shows needs
//! what only the server has. Rules (`AGENTS.md`) and MCP servers join the
//! sync through [`crate::opencode_inventory`].

use std::path::{Path, PathBuf};

use juggler_adapter::environment::{DiscoveredInstall, ImportOutcome, LoginProbe};
use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, HarnessAdapter, InstallStatus, LaunchMode,
    LaunchSpec, SessionEvent, SessionWatcher, VerificationStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

use crate::claude_code::{
    LoginState, existing, find_on_path, home_dir, is_managed_dir, prepare_managed_dir_for,
};
use crate::opencode_environment::{managed_config_dir, managed_data_dir, managed_state_dir};
use crate::opencode_store::DB_FILE;

/// Provider id: `opencode`.
pub const OPENCODE_ID: &str = "opencode";

/// No version has been run live yet (AJ-E1612): every one is unverified.
const VERIFIED_AGAINST: &[&str] = &[];

/// The versions whose source this adapter was built from.
pub const SOURCE_READ: &[&str] = &["1.19.3", "2.1.2"];

/// Sign-ins and sessions.
pub const ENV_DATA_HOME: &str = "XDG_DATA_HOME";
/// Settings.
pub const ENV_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
/// Preferences, history, locks.
pub const ENV_STATE_HOME: &str = "XDG_STATE_HOME";
/// OpenCode's own settings-folder variable.
pub const ENV_CONFIG_DIR: &str = "OPENCODE_CONFIG_DIR";
/// Would move the database out of the account's data folder.
const ENV_DB: &str = "OPENCODE_DB";
/// Would move the home OpenCode reads `~` paths from.
const ENV_TEST_HOME: &str = "OPENCODE_TEST_HOME";
/// Optional test/dev override for the binary path.
const ENV_BIN_OVERRIDE: &str = "AI_JUGGLER_OPENCODE_BIN";
/// Inline settings, highest precedence — how a Launch-settings model
/// reaches the TUI (v2's TUI has no `--model`).
const ENV_CONFIG_CONTENT: &str = "OPENCODE_CONFIG_CONTENT";

const PINNED: &[&str] = &[
    ENV_DATA_HOME,
    ENV_CONFIG_HOME,
    ENV_STATE_HOME,
    ENV_CONFIG_DIR,
];
const ISOLATION: &[&str] = &[
    ENV_DATA_HOME,
    ENV_CONFIG_HOME,
    ENV_STATE_HOME,
    ENV_CONFIG_DIR,
    ENV_DB,
    ENV_TEST_HOME,
    ENV_BIN_OVERRIDE,
];

/// Removed from every launch: the database and home movers, and the
/// sign-ins an inherited environment would bring (see module docs).
const UNSET: &[&str] = &[
    ENV_DB,
    ENV_TEST_HOME,
    "OPENCODE_AUTH_CONTENT",
    "OPENCODE_API_KEY",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "OPENAI_API_KEY",
    "OPENROUTER_API_KEY",
    "GEMINI_API_KEY",
    "GOOGLE_GENERATIVE_AI_API_KEY",
    "XAI_API_KEY",
    "GROQ_API_KEY",
    "DEEPSEEK_API_KEY",
    "MISTRAL_API_KEY",
];

/// Which OpenCode line a version belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    /// 1.x — `auth.json` sign-ins, `session`/`message` tables.
    V1,
    /// 2.x — sign-ins in the database, `session_v2`/`session_message`.
    V2,
}

/// The line of a version string (`1.19.3` → V1); `None` for others.
#[must_use]
pub fn line_of(version: &str) -> Option<Line> {
    match version.split('.').next()? {
        "1" => Some(Line::V1),
        "2" => Some(Line::V2),
        _ => None,
    }
}

/// `opencode --version` prints the bare version on v1 (`1.19.3`, yargs);
/// v2 may print it after the name. The version is the last token of the
/// first line, a leading `v` dropped.
#[must_use]
pub fn parse_version(stdout: &str) -> Option<String> {
    let token = stdout.lines().next()?.split_whitespace().last()?;
    let token = token.strip_prefix('v').unwrap_or(token);
    token
        .starts_with(|c: char| c.is_ascii_digit())
        .then(|| token.to_owned())
}

/// Adapter for the OpenCode CLI.
#[derive(Debug, Clone, Default)]
pub struct OpenCodeAdapter {
    bin: Option<PathBuf>,
    /// The user's home, for the default data folder (tests/fixtures;
    /// `None` reads `$HOME`).
    home: Option<PathBuf>,
}

impl OpenCodeAdapter {
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

    /// The settings folder of an account folder: `~/.config/opencode` for
    /// the default data folder used in place, else `<account>/config/opencode`.
    #[must_use]
    pub fn config_dir(&self, state_dir: &Path) -> Option<PathBuf> {
        if self.is_default(state_dir) {
            self.user_home()
                .map(|home| home.join(".config").join("opencode"))
        } else {
            Some(managed_config_dir(state_dir))
        }
    }

    /// OpenCode's own default data folder, `~/.local/share/opencode`.
    fn default_data_dir(&self) -> Option<PathBuf> {
        self.user_home()
            .map(|home| home.join(".local").join("share").join("opencode"))
    }

    /// Whether `dir` is the default data folder, by its exact path.
    fn is_default(&self, dir: &Path) -> bool {
        self.default_data_dir().is_some_and(|default| {
            Path::new(dir.to_string_lossy().trim_end_matches('/')) == default
        })
    }

    /// The data folder of an account folder: the folder itself when it
    /// is the default one used in place, else `<account>/data/opencode`.
    #[must_use]
    pub fn data_dir(&self, state_dir: &Path) -> PathBuf {
        if self.is_default(state_dir) {
            state_dir.to_path_buf()
        } else {
            managed_data_dir(state_dir)
        }
    }

    /// The account's session database.
    #[must_use]
    pub fn db_path(&self, state_dir: &Path) -> PathBuf {
        self.data_dir(state_dir).join(DB_FILE)
    }

    fn resolve_bin(&self) -> Option<PathBuf> {
        if let Some(bin) = &self.bin {
            return Some(bin.clone());
        }
        if let Some(path) = std::env::var_os(ENV_BIN_OVERRIDE).map(PathBuf::from)
            && path.is_file()
        {
            return Some(path);
        }
        find_on_path("opencode").or_else(|| {
            // GUI launches don't inherit the shell PATH: Homebrew, the
            // curl installer (`~/.opencode/bin`), npm and bun globals.
            existing(&["/opt/homebrew/bin/opencode", "/usr/local/bin/opencode"]).or_else(|| {
                let home = self.user_home()?;
                [
                    ".opencode/bin/opencode",
                    ".local/bin/opencode",
                    ".npm-global/bin/opencode",
                    ".bun/bin/opencode",
                ]
                .iter()
                .map(|relative| home.join(relative))
                .find(|path| path.is_file())
            })
        })
    }

    fn require_bin(&self) -> Result<PathBuf, AdapterError> {
        self.resolve_bin().ok_or_else(|| {
            AdapterError::InvalidLaunch(
                "opencode binary not found; run discovery for details".into(),
            )
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

    /// The pinned variables plus every removed one, for a launch or the
    /// sign-in flow.
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

/// The data folder of an account folder, from the account's own record:
/// an in-place account's folder IS OpenCode's data folder (only the
/// default one can be used in place); a managed one keeps it under
/// `data/opencode`.
#[must_use]
pub fn data_dir_of(state_dir: &Path, in_place: bool) -> PathBuf {
    if in_place {
        state_dir.to_path_buf()
    } else {
        managed_data_dir(state_dir)
    }
}

/// Sign-in state of a data folder, from `auth.json`'s shape and the v2
/// `credential` table's row count — never a value (ADR-0003).
#[must_use]
pub fn opencode_login_state(data_dir: &Path) -> LoginState {
    if std::fs::read_dir(data_dir).is_err() {
        return LoginState::Unknown;
    }
    let from_file = match crate::sign_in::opencode_auth(data_dir) {
        crate::sign_in::OpenCodeAuth::Missing => LoginState::LoggedOut,
        crate::sign_in::OpenCodeAuth::Empty => LoginState::Partial,
        crate::sign_in::OpenCodeAuth::Kinds(kinds) => classify_kinds(&kinds),
        crate::sign_in::OpenCodeAuth::Unreadable => LoginState::Unknown,
    };
    if matches!(from_file, LoginState::SignedIn) {
        return from_file;
    }
    // v2 keeps sign-ins in the database: an active row is a sign-in.
    // Only the count is read; the `value` column is never selected.
    let db = data_dir.join(DB_FILE);
    let active = crate::opencode_store::open_read_only(&db)
        .ok()
        .filter(|conn| crate::opencode_store::has_table(conn, "credential"))
        .and_then(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM credential WHERE active IS NULL OR active = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .ok()
        })
        .unwrap_or_default();
    if active > 0 {
        LoginState::SignedIn
    } else {
        from_file
    }
}

/// A sign-in by the kinds of its entries: OAuth or a provider's own
/// well-known sign-in is a sign-in; keys alone are an API-key account.
fn classify_kinds(kinds: &[String]) -> LoginState {
    if kinds
        .iter()
        .any(|kind| matches!(kind.as_str(), "oauth" | "wellknown"))
    {
        LoginState::SignedIn
    } else if kinds.iter().any(|kind| kind == "api") {
        LoginState::ApiKey
    } else {
        LoginState::LoggedOut
    }
}

fn scan_probe(data_dir: &Path) -> LoginProbe {
    match opencode_login_state(data_dir) {
        LoginState::SignedIn | LoginState::ApiKey => LoginProbe::LoggedIn,
        LoginState::Partial
        | LoginState::Expired
        | LoginState::SsoPending
        | LoginState::LoggedOut
        | LoginState::Unknown => LoginProbe::LoggedOut,
    }
}

/// `{"model": "<model>"}` merged into any inline settings the launch
/// already carries.
fn with_model(existing: Option<&String>, model: &str) -> String {
    let mut value = existing
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    value["model"] = serde_json::Value::String(model.to_owned());
    value.to_string()
}

impl HarnessAdapter for OpenCodeAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(OPENCODE_ID)
    }

    fn spec(&self) -> juggler_adapter::ToolSpec {
        opencode_spec()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::HEADLESS
            | Capabilities::RESUME
            | Capabilities::TRANSCRIPT_WATCH
            | Capabilities::USAGE_TOKENS
    }

    fn verified_against(&self) -> &'static [&'static str] {
        VERIFIED_AGAINST
    }

    fn discover(&self) -> DiscoveryReport {
        let Some(bin) = self.resolve_bin() else {
            return self.report(
                None,
                InstallStatus::NotFound,
                vec!["looked on PATH, /opt/homebrew/bin, /usr/local/bin, ~/.opencode/bin, ~/.local/bin, ~/.npm-global/bin, ~/.bun/bin".into()],
            );
        };
        if !bin.is_file() {
            return self.report(
                None,
                InstallStatus::NotFound,
                vec![format!(
                    "resolved {} but the file does not exist",
                    bin.display()
                )],
            );
        }
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
        let note = match line_of(&version) {
            Some(Line::V1) => "OpenCode 1 — built from the 1.19.3 source; not yet checked live",
            Some(Line::V2) => "OpenCode 2 — built from the 2.1.2 source; not yet checked live",
            None => "a newer OpenCode than this adapter knows (built for 1.x and 2.x)",
        };
        self.report(
            Some(bin),
            InstallStatus::Installed { version },
            vec![note.to_owned()],
        )
    }

    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        prepare_managed_dir_for(state_dir, OPENCODE_ID)?;
        for dir in [
            managed_data_dir(state_dir),
            managed_config_dir(state_dir),
            managed_state_dir(state_dir),
        ] {
            crate::claude_code::create_private_dir(&dir).map_err(|error| {
                AdapterError::AccountDir {
                    path: dir.clone(),
                    message: error.to_string(),
                }
            })?;
        }
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
        let effective_cwd = profile
            .default_cwd
            .clone()
            .unwrap_or_else(|| cwd.to_path_buf());
        if let LaunchMode::Resume { session_id, .. } = mode {
            crate::codex::refuse_option_like(session_id)?;
        }
        // Launch-settings overrides first, isolation LAST (AJ-E309).
        let mut env: EnvMap = profile.settings_overlay.env.clone();
        env.extend(self.isolation_env(&account.state_dir));
        let model = profile.settings_overlay.model.as_deref();
        let mut args: Vec<String> = Vec::new();
        match mode {
            // `opencode run --format json [--model m] -- <message>`: the
            // message after `--`, so text starting with "-" is never an
            // option (v1 joins `args["--"]` into the message).
            LaunchMode::Headless { prompt } => {
                args.extend(["run".into(), "--format".into(), "json".into()]);
                if let Some(model) = model {
                    args.push(format!("--model={model}"));
                }
                args.extend(["--".into(), prompt.clone()]);
            }
            // The TUI: `--session <id>` resumes; `--prompt=<text>` sends
            // the first message (the `=` form keeps a leading "-" text).
            LaunchMode::Interactive | LaunchMode::Primed { .. } | LaunchMode::Resume { .. } => {
                if let LaunchMode::Resume { session_id, .. } = mode {
                    args.extend(["--session".into(), session_id.clone()]);
                }
                let prompt = match mode {
                    LaunchMode::Primed { prompt, .. } => Some(prompt),
                    LaunchMode::Resume {
                        delta_prompt: Some(delta),
                        ..
                    } => Some(delta),
                    _ => None,
                };
                if let Some(prompt) = prompt {
                    args.push(format!("--prompt={prompt}"));
                }
                if let Some(model) = model {
                    let merged = with_model(env.get(ENV_CONFIG_CONTENT), model);
                    env.insert(ENV_CONFIG_CONTENT.to_owned(), merged);
                }
            }
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
        opencode_login_state(&self.data_dir(state_dir))
    }

    /// The vendor's own `opencode auth login` (pick a provider, then its
    /// browser or key flow) in this account's folders.
    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        Ok(LaunchSpec {
            program: self.require_bin()?,
            args: vec!["auth".into(), "login".into()],
            env: self.isolation_env(state_dir),
            cwd: state_dir.to_path_buf(),
            needs_pty: true,
        })
    }

    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        Ok(Box::new(DbWatcher {
            db: transcript_root.join(DB_FILE),
            seen: std::collections::BTreeMap::new(),
            pending: std::collections::VecDeque::new(),
        }))
    }

    /// The session that ran in `cwd` from `opencode.db` (read-only);
    /// its link is `<db>#<id>` (see `opencode_store`).
    fn locate_transcript(
        &self,
        state_dir: &Path,
        cwd: &Path,
        since_ms: i64,
        taken: &[PathBuf],
    ) -> Option<juggler_core::model::TranscriptRef> {
        crate::opencode_store::locate(&self.db_path(state_dir), cwd, since_ms, taken).map(
            |(path, id)| juggler_core::model::TranscriptRef {
                path,
                native_id: Some(id),
            },
        )
    }

    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        let Some(home) = self.user_home() else {
            return Vec::new();
        };
        let xdg = std::env::var_os(ENV_DATA_HOME)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        crate::opencode_environment::scan_opencode_environment(&home, xdg.as_deref(), &scan_probe)
    }

    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::opencode_environment::import_opencode_data(
            source,
            dest,
            self.settings_of(source).as_deref(),
            &scan_probe,
            false,
        )
    }

    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        crate::opencode_environment::import_opencode_data(
            source,
            dest,
            self.settings_of(source).as_deref(),
            &scan_probe,
            true,
        )
    }

    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        crate::opencode_environment::inspect_opencode_source(source, &scan_probe)
    }

    fn can_use_in_place(&self) -> bool {
        true
    }

    fn vendor_default_dir(&self) -> Option<PathBuf> {
        self.default_data_dir()
    }

    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        crate::opencode_environment::adopt_opencode_data(
            source,
            self.default_data_dir().as_deref(),
            &scan_probe,
        )
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

    /// Each XDG root is its own folder inside the account; the default
    /// data folder runs with all of them unset.
    fn pinned_env(&self, state_dir: &Path) -> Vec<(&'static str, Option<String>)> {
        if self.is_default(state_dir) {
            return PINNED.iter().map(|key| (*key, None)).collect();
        }
        let text = |path: PathBuf| Some(path.to_string_lossy().into_owned());
        vec![
            (ENV_DATA_HOME, text(state_dir.join("data"))),
            (ENV_CONFIG_HOME, text(state_dir.join("config"))),
            (ENV_STATE_HOME, text(state_dir.join("state"))),
            (ENV_CONFIG_DIR, text(managed_config_dir(state_dir))),
        ]
    }

    fn shell_command(&self) -> Option<&'static str> {
        Some("opencode")
    }

    fn resolved_binary(&self) -> Option<PathBuf> {
        self.resolve_bin()
    }

    /// `opencode --session <id>` reopens it in the TUI.
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
        crate::opencode_subagents::SUPPORT
    }

    fn subagent_reader(
        &self,
        scope: juggler_adapter::SubagentScope<'_>,
    ) -> Option<Box<dyn juggler_adapter::SubagentReader>> {
        crate::opencode_subagents::reader(scope)
    }

    fn stored_conversations(
        &self,
        state_dir: &Path,
    ) -> Vec<juggler_adapter::conversation::StoredConversation> {
        crate::opencode_store::stored_conversations(&self.db_path(state_dir))
    }

    /// Completed assistant messages from the account's `opencode.db`:
    /// tokens Measured (the provider's counts, as OpenCode recorded them),
    /// dollars an estimate (OpenCode's price from its model list).
    fn usage_since(
        &self,
        account: &Account,
        since_ms: i64,
    ) -> Option<Result<Vec<juggler_core::quota::UsageEvent>, String>> {
        let db = data_dir_of(&account.state_dir, account.in_place).join(DB_FILE);
        if !db.is_file() {
            return Some(Ok(Vec::new()));
        }
        Some(crate::opencode_store::usage_events(
            account.id.as_str(),
            &db,
            since_ms,
        ))
    }

    fn inventory(
        &self,
        state_dir: &Path,
        _project: Option<&Path>,
    ) -> Vec<juggler_adapter::inventory::Item> {
        self.config_dir(state_dir)
            .map(|config| crate::opencode_inventory::inventory(&config))
            .unwrap_or_default()
    }

    fn can_set_up(
        &self,
        item: &juggler_adapter::inventory::Item,
    ) -> juggler_adapter::inventory::SetUp {
        crate::opencode_inventory::can_set_up(item)
    }

    fn set_up(
        &self,
        state_dir: &Path,
        _project: Option<&Path>,
        item: &juggler_adapter::inventory::Item,
    ) -> Result<juggler_adapter::inventory::Applied, AdapterError> {
        let config = self
            .config_dir(state_dir)
            .ok_or_else(|| AdapterError::Unsupported("no home folder".to_owned()))?;
        crate::opencode_inventory::set_up(&config, item)
    }
}

/// `opencode` subcommands that aren't a session: servers, sign-in and
/// maintenance (research §4, §6; `opencode --help` at v1.19.3).
const HELPERS: &[&str] = &[
    "serve",
    "web",
    "acp",
    "mcp",
    "auth",
    "providers",
    "upgrade",
    "uninstall",
    "stats",
    "export",
    "import",
    "session",
    "models",
    "debug",
    "github",
    "console",
    "completion",
];

/// An `opencode` CLI (the binary, or the npm package run by node) is a
/// session (the TUI, `run`, `attach`) unless it runs a helper subcommand.
/// `--session <id>` names its conversation.
#[must_use]
pub fn session_process(
    exe: Option<&Path>,
    args: &[String],
) -> Option<juggler_adapter::conversation::SessionProcess> {
    let cli = exe.is_some_and(|exe| exe.file_name().is_some_and(|name| name == "opencode"))
        || args.iter().any(|arg| arg.contains("opencode-ai"));
    let helper = args
        .iter()
        .skip(1)
        .any(|arg| HELPERS.contains(&arg.as_str()));
    (cli && !helper).then(|| juggler_adapter::conversation::SessionProcess {
        resumes: juggler_adapter::conversation::value_after(args, &["--session", "-s"]),
    })
}

/// ADR-0019: OpenCode, described as data.
#[must_use]
pub fn opencode_spec() -> juggler_adapter::ToolSpec {
    use juggler_adapter::{SetupGroup, SignInWords, ToolFlags, ToolKind, ToolSpec, ToolTerms};
    ToolSpec {
        id: OPENCODE_ID.to_owned(),
        display_name: "OpenCode".to_owned(),
        short_name: "OpenCode".to_owned(),
        vendor: "Anomaly".to_owned(),
        // Not yet checked against its owner's guidelines (brand-marks.md):
        // the letter mark until then.
        mark: Some("opencode".to_owned()),
        vendor_mark: None,
        install_url: Some("https://opencode.ai/download".to_owned()),
        install_command: Some("npm install -g opencode-ai".to_owned()),
        sign_in: SignInWords {
            own: "OpenCode's own sign-in".to_owned(),
            short: "OpenCode's".to_owned(),
        },
        same_as: "as when you run OpenCode yourself".to_owned(),
        // OpenCode's desktop app shares the CLI's folder, but can't be
        // started per account (research §2): tracked by AJ-E1610, not here.
        desktop_app: None,
        kinds: vec![ToolKind::Cli],
        rules_files: vec!["AGENTS.md".to_owned()],
        rules_import: None,
        scan_hint: "Your default setup and anything else that looks like one. Nothing is changed."
            .to_owned(),
        setup_groups: vec![SetupGroup {
            id: OPENCODE_ID.to_owned(),
            title: "OpenCode".to_owned(),
            path_contains: None,
            origin: None,
            shows_install: true,
        }],
        default_label: Some("OpenCode".to_owned()),
        flags: ToolFlags {
            first_run: true,
            primary: false,
        },
        // Research `opencode.md` §7 (accessed 2026-10-21), recorded in
        // AJ-E1518's register.
        terms: Some(ToolTerms {
            name: "OpenCode's Terms of Service".to_owned(),
            url: "https://opencode.ai/legal/terms-of-service".to_owned(),
            accounts: "OpenCode's Terms of Service say not to use several accounts to get around \
                       usage limits, access restrictions or billing."
                .to_owned(),
            checked: "2026-10-21".to_owned(),
        }),
        // AJ-E1611: npm's `latest` [the tool's own published package].
        release_feed: Some(juggler_adapter::ReleaseFeed::Npm {
            package: "opencode-ai".to_owned(),
        }),
        update_command: Some("opencode upgrade".to_owned()),
    }
}

impl OpenCodeAdapter {
    /// The settings folder that goes with a data folder being copied:
    /// `~/.config/opencode` for the default one; unknown for others.
    fn settings_of(&self, source: &Path) -> Option<PathBuf> {
        let canonical = |path: &Path| path.canonicalize().ok();
        let default = self.default_data_dir()?;
        (canonical(&default)? == canonical(source)? && !is_managed_dir(source))
            .then(|| {
                self.user_home()
                    .map(|home| home.join(".config").join("opencode"))
            })
            .flatten()
    }
}

/// Polls `opencode.db` for sessions: a new id is `Discovered`, a newer
/// `time_updated` is `MessageAppended`. Read-only; an unreadable file
/// yields nothing (watching degrades, never crashes).
struct DbWatcher {
    db: PathBuf,
    seen: std::collections::BTreeMap<String, i64>,
    pending: std::collections::VecDeque<SessionEvent>,
}

impl SessionWatcher for DbWatcher {
    fn poll(
        &mut self,
        _timeout: std::time::Duration,
    ) -> Result<Option<SessionEvent>, AdapterError> {
        if self.pending.is_empty()
            && let Ok(sessions) = crate::opencode_store::sessions(&self.db)
        {
            for session in sessions.into_iter().rev() {
                let transcript_path = crate::opencode_store::session_ref(&self.db, &session.id);
                match self.seen.insert(session.id.clone(), session.updated_ms) {
                    None => self.pending.push_back(SessionEvent::Discovered {
                        native_id: session.id,
                        transcript_path,
                    }),
                    Some(before) if before < session.updated_ms => {
                        self.pending.push_back(SessionEvent::MessageAppended {
                            native_id: session.id,
                            transcript_path,
                        });
                    }
                    Some(_) => {}
                }
            }
        }
        Ok(self.pending.pop_front())
    }
}
