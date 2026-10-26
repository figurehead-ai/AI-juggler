//! The `HarnessAdapter` contract (spec §4, ADR-0002).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::capabilities::Capabilities;
use crate::discovery::DiscoveryReport;
use crate::environment::{DiscoveredInstall, ImportOutcome, LoginState, LoginUrl};
use crate::error::AdapterError;
use crate::launch::{DesktopLaunch, EnvMap, LaunchMode, LaunchSpec};
use crate::model::{Account, HarnessId, Profile};
use crate::watcher::SessionWatcher;

/// The only place harness specifics are allowed to live.
///
/// Implementations:
/// - are **read-only** toward harness state except inside
///   [`HarnessAdapter::prepare_account`], the single writer to a managed
///   account directory;
/// - declare `verified_against` versions and capability-probe at runtime —
///   never assume unadvertised capabilities;
/// - never touch vendor credentials' contents.
///
/// The trait is public, semver-protected API (guarded by
/// `cargo-semver-checks` in CI once the first stable release ships).
pub trait HarnessAdapter: Send + Sync {
    /// The harness this adapter integrates, e.g. `claude-code`.
    fn id(&self) -> HarnessId;

    /// What this harness integration supports.
    fn capabilities(&self) -> Capabilities;

    /// ADR-0019 §1: the tool described as data, for every layer that
    /// would otherwise branch on its id. The default claims nothing: the
    /// id as the name and the letter mark.
    fn spec(&self) -> crate::spec::ToolSpec {
        crate::spec::ToolSpec::generic(self.id().as_str())
    }

    /// Harness versions this adapter was validated against (spec §4
    /// version-pinning rule). Discovery compares against this list.
    fn verified_against(&self) -> &'static [&'static str];

    /// Probes the machine: is the harness installed, at what version, and is
    /// that version verified? Never fails — problems are carried in the
    /// report.
    #[must_use]
    fn discover(&self) -> DiscoveryReport;

    /// Prepares (idempotently) an isolated account state directory and
    /// returns the environment overlay that points the harness at it.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::AccountDir`] when the directory cannot be
    /// created or initialized.
    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError>;

    /// Builds the launch specification for one session. The account carries
    /// the managed state directory the adapter itself prepared earlier via
    /// [`HarnessAdapter::prepare_account`].
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::InvalidLaunch`] for unsupported
    /// profile/mode combinations.
    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError>;

    /// Builds a desktop launch for the account (AJ-E413): the
    /// windowed app tracked as an Instance. Providers without a
    /// desktop app keep the default — `Unsupported`, and the core
    /// never offers the launch (capability-gated).
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Unsupported`] when the provider ships no
    /// desktop app, [`AdapterError::InvalidLaunch`] for a missing binary.
    fn desktop_launch(&self, account: &Account) -> Result<DesktopLaunch, AdapterError> {
        let _ = account;
        Err(AdapterError::Unsupported(
            "this provider ships no desktop app".into(),
        ))
    }

    /// The per-Provider desktop isolation claim (AJ-E413 Req.1), shown
    /// verbatim on every desktop Instance of this provider. `None` for
    /// providers without a desktop app; single-account apps carry
    /// `Single-account app — Juggler tracks, cannot isolate` — never an
    /// implied isolation the vendor does not provide.
    fn desktop_isolation_label(&self) -> Option<&'static str> {
        None
    }

    /// AJ-E1001: who this Account's state dir is signed in as —
    /// metadata only (a stable account id and a masked label), never
    /// credential content. Default: `Unverifiable` (the Instance shows
    /// `Identity unverified`, never a pretended check).
    fn probe_identity(&self, account: &Account) -> crate::identity::IdentityProbe {
        let _ = account;
        crate::identity::IdentityProbe::Unverifiable {
            reason: "this Provider exposes no account metadata".into(),
        }
    }

    /// AJ-E308/AJ-E1301: the Account's sign-in state, probed from its
    /// state dir without reading credential contents (ADR-0003). The
    /// default — `LoggedOut` — is what Providers without a probe have
    /// always reported; an adapter that can tell overrides it.
    fn login_state(&self, state_dir: &Path) -> LoginState {
        let _ = state_dir;
        LoginState::LoggedOut
    }

    /// AJ-E1303: finds the transcript of a session this Account started
    /// in `cwd` at or after `since_ms` (epoch ms) — the file the vendor
    /// created for it, read-only. Files in `taken` (linked to other
    /// sessions) are never candidates, and more than one remaining match
    /// is an honest `None`: a guess could hand one session another's
    /// conversation. Default: `None` (the switch then asks).
    fn locate_transcript(
        &self,
        state_dir: &Path,
        cwd: &Path,
        since_ms: i64,
        taken: &[std::path::PathBuf],
    ) -> Option<juggler_core::model::TranscriptRef> {
        let _ = (state_dir, cwd, since_ms, taken);
        None
    }

    /// AJ-E1542: what this account has set up (MCP servers, skills,
    /// commands, subagents, user rules, plugins), read from its files.
    /// Never starts the tool or a server, and never keeps a secret's text
    /// (see [`crate::inventory`]). `project` adds what's set up for that
    /// project only. Default: nothing.
    #[must_use]
    fn inventory(&self, state_dir: &Path, project: Option<&Path>) -> Vec<crate::inventory::Item> {
        let _ = (state_dir, project);
        Vec::new()
    }

    /// AJ-E1542: whether this tool can set up `item`, which another
    /// account or tool has. Default: not yet.
    #[must_use]
    fn can_set_up(&self, item: &crate::inventory::Item) -> crate::inventory::SetUp {
        crate::inventory::SetUp::Impossible(format!("{} can't set up {} yet", self.id(), item.name))
    }

    /// AJ-E1542: sets up `item` in this account (only on the user's
    /// approval, after [`HarnessAdapter::can_set_up`] said it can), and
    /// says how to undo it. Never replaces anything the account has.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] when it can't; the write's error.
    fn set_up(
        &self,
        state_dir: &Path,
        project: Option<&Path>,
        item: &crate::inventory::Item,
    ) -> Result<crate::inventory::Applied, AdapterError> {
        let _ = (state_dir, project);
        Err(AdapterError::Unsupported(format!(
            "{} can't set up {} yet",
            self.id(),
            item.name
        )))
    }

    /// AJ-E1529: the conversation files this account's tool keeps under
    /// `state_dir` (every one, AI Juggler's or not). Lists only; reads
    /// nothing. Default: none (a tool without a readable history).
    #[must_use]
    fn conversation_files(&self, state_dir: &Path) -> crate::conversation::ConversationFiles {
        let _ = state_dir;
        Vec::new()
    }

    /// AJ-E1529: what one of [`HarnessAdapter::conversation_files`] says
    /// about itself. Read-only; `None` when it isn't a conversation (empty,
    /// damaged beyond reading, another tool's file).
    #[must_use]
    fn read_conversation(&self, path: &Path) -> Option<crate::conversation::ConversationSummary> {
        let _ = path;
        None
    }

    /// AJ-E1530 Req.1 (ADR-0018 amendment, opt-in): the one change this
    /// tool's own settings take so that its sessions started outside AI
    /// Juggler report their limits too: a top-level key of a settings
    /// file under the account's folder. `None` when the tool reports its
    /// limits some other way (Codex: in its rollouts), or not at all.
    #[must_use]
    fn limits_capture(&self) -> Option<LimitsCapture> {
        None
    }

    /// The value [`HarnessAdapter::limits_capture`]'s key takes: a call
    /// to AI Juggler's capture (`wrapper`, one shell word) that keeps
    /// what the user's own value (`original`) said, so what they see is
    /// unchanged.
    #[must_use]
    fn limits_capture_value(
        &self,
        wrapper: &str,
        original: Option<&serde_json::Value>,
    ) -> Option<serde_json::Value> {
        let _ = (wrapper, original);
        None
    }

    /// D-39 (needs the maintainer's ruling; AI Juggler keeps it off until
    /// then): a short headless run of the tool's own usage command for the
    /// profile at `state_dir`. The tool asks its vendor with its own
    /// sign-in, which AI Juggler never sees, and keeps the answer where
    /// the tool's limit records are read. No model turn, no transcript.
    /// `None`: the tool has no such command, or isn't installed.
    #[must_use]
    fn limits_probe_spec(&self, state_dir: &Path) -> Option<LaunchSpec> {
        let _ = state_dir;
        None
    }

    /// QA batch 10: a quick read of a large conversation file, from its
    /// start and end only (`conversation::file_ends`): its id, folder,
    /// branch, first and last activity and, when the start has it, its
    /// title, with no tokens (they need the whole file). The index reads
    /// this first, so a running session shows within seconds, and
    /// [`HarnessAdapter::read_conversation`] afterwards. `None`: no quick
    /// read (a small file, or a tool without one), read it whole.
    #[must_use]
    fn read_conversation_quick(
        &self,
        path: &Path,
    ) -> Option<crate::conversation::ConversationSummary> {
        let _ = path;
        None
    }

    /// AJ-E1602: the conversations of a tool that keeps them all in one
    /// store (one database) rather than a file each, read in one pass,
    /// read-only. Each carries a key no file has (`<store>#<id>`) and when
    /// it last changed, so the index reads again only what changed.
    /// Default: none (the tool keeps a file per conversation, listed by
    /// [`HarnessAdapter::conversation_files`]).
    #[must_use]
    fn stored_conversations(
        &self,
        state_dir: &Path,
    ) -> Vec<crate::conversation::StoredConversation> {
        let _ = state_dir;
        Vec::new()
    }

    /// AJ-E1602 (ADR-0019 §2, the quota-source building block): usage from
    /// a tool that keeps it in a store queried by time (a database), not in
    /// files tailed by offset: every message changed at or after `since_ms`
    /// (epoch ms; 0 for all), each keyed so a message read twice counts
    /// once, and labelled as honestly as the store allows. `None`: the tool
    /// has no such store. `Some(Err)`: the store couldn't be read this time
    /// (a plain message, never a secret).
    fn usage_since(
        &self,
        account: &Account,
        since_ms: i64,
    ) -> Option<Result<Vec<juggler_core::quota::UsageEvent>, String>> {
        let _ = (account, since_ms);
        None
    }

    /// AJ-E1603: why the account at `state_dir` may only be used by a
    /// person at the keyboard (its vendor's terms: Kimi Code's plans are
    /// "for interactive use only"), in words; `None` when nothing limits
    /// it. A launch nobody clicked (a restart after a crash) is never made
    /// for it, and the adapter refuses its headless runs.
    #[must_use]
    fn interactive_only(&self, state_dir: &Path) -> Option<String> {
        let _ = state_dir;
        None
    }

    /// AJ-E1529: whether a running process (its executable and its
    /// arguments, the first being the program) is one of this tool's
    /// sessions, and which conversation it resumes. Helpers and a desktop
    /// app's internals are not sessions. Default: never.
    #[must_use]
    fn session_process(
        &self,
        exe: Option<&Path>,
        args: &[String],
    ) -> Option<crate::conversation::SessionProcess> {
        let _ = (exe, args);
        None
    }

    /// AJ-E2101 (D-37): the per-session files in the profile at
    /// `profile_dir` that say which sessions are open (Claude Code's
    /// `sessions/<pid>.json`). Lists only; reads nothing. Default: none.
    #[must_use]
    fn live_session_files(&self, profile_dir: &Path) -> Vec<PathBuf> {
        let _ = profile_dir;
        Vec::new()
    }

    /// AJ-E2101: what one of [`HarnessAdapter::live_session_files`] says:
    /// the session, its pid and start time when recorded. Read-only and
    /// tolerant: `None` for a file of another shape. Default: `None`.
    #[must_use]
    fn read_live_session(&self, path: &Path) -> Option<crate::live::LiveSession> {
        let _ = path;
        None
    }

    /// AJ-E2101 (D-37): the sessions open now in the profile at
    /// `profile_dir`, from the tool's own per-session files (never a
    /// process's environment). Each names its pid and start time when the
    /// tool records them; the app checks both against the process table.
    #[must_use]
    fn live_sessions(&self, profile_dir: &Path) -> Vec<crate::live::LiveSession> {
        self.live_session_files(profile_dir)
            .iter()
            .filter_map(|path| self.read_live_session(path))
            .collect()
    }

    /// AJ-E2101: the files in which this tool's desktop app keeps its
    /// sessions, for the instance whose data folder is `instance_dir`.
    /// Lists only. Default: none.
    #[must_use]
    fn app_session_files(&self, instance_dir: &Path) -> Vec<PathBuf> {
        let _ = instance_dir;
        Vec::new()
    }

    /// AJ-E2101: one of [`HarnessAdapter::app_session_files`], read field
    /// by field: its title, archived, when last in front, and the last
    /// turn's status word, never summary text. Default: `None`.
    #[must_use]
    fn read_app_session(&self, path: &Path) -> Option<crate::live::AppSession> {
        let _ = path;
        None
    }

    /// AJ-E2101: the sessions this tool's desktop app keeps in the
    /// instance at `instance_dir` (its sidebar).
    #[must_use]
    fn app_sessions(&self, instance_dir: &Path) -> Vec<crate::live::AppSession> {
        self.app_session_files(instance_dir)
            .iter()
            .filter_map(|path| self.read_app_session(path))
            .collect()
    }

    /// AJ-E2101: the sessions a running process of this tool holds open,
    /// from the files it has open (`open_files`, as the OS lists them) that
    /// lie in the profile at `profile_dir`. For a tool whose app serves its
    /// sessions from one process (the Codex app's server). Default: none.
    #[must_use]
    fn held_sessions(&self, profile_dir: &Path, open_files: &[PathBuf]) -> Vec<String> {
        let _ = (profile_dir, open_files);
        Vec::new()
    }

    /// AJ-E2101: whether a running process (its executable and arguments)
    /// serves this tool's sessions and holds their files open, so its open
    /// files say which are loaded (the Codex app's server). Default: never.
    #[must_use]
    fn is_session_server(&self, exe: Option<&Path>, args: &[String]) -> bool {
        let _ = (exe, args);
        false
    }

    /// AJ-E2107: what the tail of an open session's transcript says it
    /// waits on (`tail`: at most its last 64 KB, cut at a line start), for
    /// a session whose tool records no such word itself. Default: nothing.
    #[must_use]
    fn pending_ask(&self, tail: &str) -> Option<crate::live::Ask> {
        let _ = tail;
        None
    }

    /// What the tail of an open session's transcript (`tail`: at most its
    /// last 64 KB, cut at a line start) says of its last turn: running,
    /// waiting on the user, or ended. For a session whose tool records no
    /// state for it (the Codex app's threads, its CLI in a terminal).
    /// `None` when the tail doesn't say. Default: nothing.
    #[must_use]
    fn tail_turn(&self, tail: &str) -> Option<crate::live::TailTurn> {
        let _ = tail;
        None
    }

    /// AJ-E2102 (ADR-0019): the tool's own apps, as data: how each is
    /// recognised in a parent chain, its instances, and its links.
    /// Default: none.
    #[must_use]
    fn host_apps(&self) -> Vec<crate::live::HostApp> {
        Vec::new()
    }

    /// Creates a watcher over the account's transcript root.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Watch`] when the root cannot be observed.
    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError>;

    /// Scans the machine for existing installations of this harness
    /// (smart onboarding, spec §12). Metadata-only: presence, size,
    /// timestamps — never file contents. Adapters that cannot scan return
    /// an empty list; scanning never fails.
    #[must_use]
    fn scan_environment(&self) -> Vec<DiscoveredInstall> {
        Vec::new()
    }

    /// Shell rc assignments pointing at this harness's config dirs
    /// (AJ-E302 S5): path values only, source file + line recorded.
    /// Default: none — adapters opt in.
    fn scan_shell_aliases(&self) -> Vec<crate::environment::AliasHint> {
        Vec::new()
    }

    /// Other manager tools holding this harness's setups (AJ-E302
    /// S3/S4): metadata only (existence, counts, names — never
    /// tokens). Default: none — adapters opt in.
    fn scan_managers(&self) -> Vec<crate::environment::ManagerHint> {
        Vec::new()
    }

    /// Copy-imports an existing config dir (found via
    /// [`HarnessAdapter::scan_environment`]) into a fresh managed state
    /// dir. Read-only toward `source`: a byte copy with rebuildable caches
    /// skipped; the source is never written to or marked.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Unsupported`] when this adapter cannot
    /// import, [`AdapterError::AccountDir`] for path/size problems and
    /// copy failures (message carries the plain-language reason).
    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        let _ = (source, dest);
        Err(AdapterError::Unsupported(
            "this adapter cannot import existing configurations".into(),
        ))
    }

    /// ADR-0018: checks an existing config dir can be used where it is
    /// (an in-place account) and reports its sign-in — nothing is copied
    /// or written. Default: unsupported, so the UI offers only a copy.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] by default; otherwise
    /// [`AdapterError::AccountDir`] with the plain-language reason.
    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        let _ = source;
        Err(AdapterError::Unsupported(
            "this tool's setups can only be copied for now".into(),
        ))
    }

    /// ADR-0018: whether [`HarnessAdapter::adopt_account`] is supported —
    /// the UI offers "use it where it is" only for these Providers.
    /// Default: false (the default `adopt_account` refuses).
    fn can_use_in_place(&self) -> bool {
        false
    }

    /// The vendor's own default folder (`~/.claude`, `~/.codex`), which
    /// launches with its isolation variable unset — the one home lookup
    /// the app and the adapter's launch rule share. Default: none.
    fn vendor_default_dir(&self) -> Option<std::path::PathBuf> {
        None
    }

    /// AJ-E307 Req.3: the pre-copy source inspection — metadata-only;
    /// the Confirm-source step's data (sizes, breakdown, evidence).
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] by default; adapters with a copy
    /// import surface their own typed failures.
    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<crate::environment::SourceInspect, AdapterError> {
        let _ = source;
        Err(AdapterError::Unsupported(
            "this adapter cannot inspect configurations".into(),
        ))
    }

    /// AJ-E307 Req.4: the settings-only degraded import — history
    /// dirs skipped, each skip recorded with its size.
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] by default; otherwise the same
    /// conditions as [`HarnessAdapter::import_account`].
    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        let _ = (source, dest);
        Err(AdapterError::Unsupported(
            "this adapter has no degraded import".into(),
        ))
    }

    /// The vendor's own web sign-in page, when one exists (the maximum
    /// login aid: we open the page, the user completes the vendor's flow).
    #[must_use]
    fn login_url(&self) -> Option<LoginUrl> {
        None
    }

    /// Builds the launch spec for the vendor's own interactive login flow
    /// against a config dir — the guided-terminal login primitive (spec
    /// §4.1): AI Juggler spawns it in an embedded terminal; the user
    /// completes the OAuth flow; nothing is automated.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Unsupported`] when the harness has no
    /// terminal login flow, [`AdapterError::InvalidLaunch`] when the
    /// binary cannot be resolved.
    fn login_spec(&self, state_dir: &Path) -> Result<LaunchSpec, AdapterError> {
        let _ = state_dir;
        Err(AdapterError::Unsupported(
            "this harness has no terminal login flow".into(),
        ))
    }

    /// Environment variables every launch removes from what it inherits
    /// (credentials that would override the account's own sign-in).
    /// Default: none.
    fn launch_unset_keys(&self) -> &'static [&'static str] {
        &[]
    }

    /// AJ-E1604: variables every launch sets to a fixed value, whatever
    /// Launch settings say (a guardrail: Grok Build reads the user's
    /// Claude and Cursor setups unless told not to). The copyable shell
    /// line sets them too, and each is one of
    /// [`HarnessAdapter::isolation_env_keys`] so no override reaches it.
    /// Default: none.
    fn launch_fixed_env(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// The value of the pinned isolation keys for `state_dir`, or `None`
    /// when the folder runs with that key unset (ADR-0018: the vendor's
    /// own default folder). Default: the path.
    fn isolation_env_value(&self, state_dir: &Path) -> Option<String> {
        Some(state_dir.to_string_lossy().into_owned())
    }

    /// The environment variables that fence an Account's isolated
    /// state for this Provider (AJ-E309). Launch-settings overrides
    /// may never set them — an override that repointed the config
    /// dir would silently break isolation. The FIRST key is the one
    /// that points the Provider at the state dir (the snippet sets
    /// it); later keys are reserved but never emitted.
    #[must_use]
    fn isolation_env_keys(&self) -> &'static [&'static str] {
        &[]
    }

    /// The isolation keys pinned to the Account's state dir itself —
    /// every one the shell snippet sets and the isolation check
    /// verifies. Default: the first isolation key (one variable points
    /// the Provider at its state); Codex also pins `CODEX_SQLITE_HOME`.
    #[must_use]
    fn pinned_env_keys(&self) -> &'static [&'static str] {
        let keys = self.isolation_env_keys();
        &keys[..keys.len().min(1)]
    }

    /// Every pinned isolation variable with its value for `state_dir`
    /// (AJ-E1521): `Some(path)` points the Provider at the Account's
    /// state, `None` runs with the variable unset (ADR-0018: the
    /// vendor's own default folder). Launches, the copyable shell line
    /// and the isolation check all read this one list. Default: every
    /// [`HarnessAdapter::pinned_env_keys`] entry set to
    /// [`HarnessAdapter::isolation_env_value`]; a Provider whose keys
    /// point at different folders inside the state dir overrides it.
    #[must_use]
    fn pinned_env(&self, state_dir: &Path) -> Vec<(&'static str, Option<String>)> {
        let value = self.isolation_env_value(state_dir);
        self.pinned_env_keys()
            .iter()
            .map(|key| (*key, value.clone()))
            .collect()
    }

    /// The command a user types to start this Provider against
    /// `state_dir` by hand — the Expert-setup snippet and the
    /// external-terminal login fallback (AJ-E309 Req.5). `None` when
    /// the Provider has no terminal entry point.
    #[must_use]
    fn shell_command(&self) -> Option<&'static str> {
        None
    }

    /// The binary this adapter would launch, when one is found (no
    /// process runs). The copyable line names it by its full path when
    /// [`HarnessAdapter::shell_command`] isn't on the user's `PATH`.
    #[must_use]
    fn resolved_binary(&self) -> Option<std::path::PathBuf> {
        None
    }

    /// The arguments that resume conversation `conversation_id` from a
    /// terminal, after [`HarnessAdapter::shell_command`] (AJ-E1537's Copy
    /// resume command). `None` when the tool can't resume from a
    /// terminal, or the id looks like an option (never passed).
    #[must_use]
    fn resume_args(&self, conversation_id: &str) -> Option<Vec<String>> {
        let _ = conversation_id;
        None
    }

    /// AJ-E1601: the protocol this tool speaks when it runs through
    /// another service's published address (D-34); `None` when it can't.
    #[must_use]
    fn endpoint_wire(&self) -> Option<crate::endpoint::WireApi> {
        None
    }

    /// AJ-E1601: points the profile in `state_dir` — a folder AI Juggler
    /// made and marked as its own, never a user's setup — at `endpoint`
    /// with `model`. The key is never written: AI Juggler hands it to each
    /// session at launch, as [`crate::endpoint::ServiceEndpoint::key_env`].
    ///
    /// # Errors
    ///
    /// [`AdapterError::Unsupported`] when the tool can't run through
    /// another service, or `endpoint` speaks another protocol;
    /// [`AdapterError::AccountDir`] when the folder isn't AI Juggler's own
    /// or can't be written.
    fn configure_endpoint(
        &self,
        state_dir: &Path,
        endpoint: &crate::endpoint::ServiceEndpoint,
        model: &str,
    ) -> Result<(), AdapterError> {
        let _ = (state_dir, endpoint, model);
        Err(AdapterError::Unsupported(
            "this tool can't run through another service".into(),
        ))
    }
    /// AJ-E1704 (D-30): how this tool takes AI Juggler's agent tools for
    /// one launch whose arguments are `args`: options to insert and the
    /// files they name (written by the app into `bridge.dir`, AI
    /// Juggler's own folder), with only AI Juggler's tools pre-allowed.
    /// Never a secret (the key is in the environment; only its name is
    /// here) and never an edit to the user's setup. `None` (the default):
    /// the tool can't add an MCP server through its launch options; its
    /// sessions still get the command line through their environment.
    /// An adapter that returns `Some` declares
    /// [`crate::Capabilities::AGENT_BRIDGE`] (the conformance kit holds both).
    #[must_use]
    fn agent_bridge(
        &self,
        bridge: &crate::agent_bridge::BridgeLaunch,
        args: &[String],
    ) -> Option<crate::agent_bridge::BridgeInjection> {
        let _ = (bridge, args);
        None
    }

    /// AJ-E2306 (D-40): how this tool's terminal launch (arguments `args`)
    /// reports its exact state through the tool's own hooks: the whole new
    /// argument list and the files for `launch.dir`, AI Juggler's own
    /// folder. Never an edit to the user's setup, never a hook that decides
    /// anything for the user. `None` (the default): the tool has no hooks
    /// AI Juggler uses. An adapter that returns `Some` declares
    /// [`crate::Capabilities::STATE_REPORTS`] (the conformance kit holds both).
    #[must_use]
    fn state_reports(
        &self,
        launch: &crate::reports::ReportsLaunch,
        args: &[String],
    ) -> Option<crate::reports::ReportsInjection> {
        let _ = (launch, args);
        None
    }

    /// Lab 2 (D-40): the question this tool asks before it works in a
    /// folder, whose answer it would save to the user's own config. AI
    /// Juggler asks it instead and passes the answer in memory
    /// ([`Self::apply_folder_trust`]). `None` (the default): the tool asks
    /// nothing AI Juggler needs to.
    #[must_use]
    fn folder_trust(&self) -> Option<crate::folder_trust::FolderTrust> {
        None
    }

    /// Whether the tool's own setup for `account` already decides `cwd`'s
    /// trust (the user chose in the tool): then AI Juggler passes nothing.
    /// Only reads.
    #[must_use]
    fn folder_trust_decided(&self, account: &crate::model::Account, cwd: &std::path::Path) -> bool {
        let _ = (account, cwd);
        false
    }

    /// Passes the user's trust choice for `cwd` to a launch of this tool
    /// (`args`, the terminal's or the driven one's), in memory only.
    fn apply_folder_trust(&self, args: &mut Vec<String>, cwd: &std::path::Path, trusted: bool) {
        let _ = (args, cwd, trusted);
    }

    /// AJ-E2302 (D-40, ADR-0022): the tool's own protocol, for sessions
    /// AI Juggler drives instead of running its terminal UI. `None` (the
    /// default): the terminal is this tool's only runtime. An adapter that
    /// returns `Some` declares [`crate::Capabilities::PROTOCOL_SERVER`]
    /// (the conformance kit holds both).
    #[must_use]
    fn driven(&self) -> Option<&dyn crate::runtime::DrivenRuntime> {
        None
    }
}

/// Single-quotes `value` for a POSIX shell (`'` becomes `'\''`), so a
/// copied snippet survives spaces and quotes in managed paths.
#[must_use]
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Builds the copyable shell line for `adapter` against `state_dir`
/// plus any extra environment (Launch-settings overrides), every pinned
/// isolation key first. `None` when the Provider has no terminal entry point.
#[must_use]
pub fn shell_snippet(
    adapter: &dyn HarnessAdapter,
    state_dir: &Path,
    extra_env: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    let command = adapter.shell_command()?;
    // What AI Juggler's own launches remove, the line removes too: the
    // default folder's key (run as the vendor runs it) and any inherited
    // credentials. Every pinned key points at the folder otherwise.
    let mut unset: Vec<String> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    for (key, value) in adapter.pinned_env(state_dir) {
        match value {
            Some(dir) => parts.push(format!("{key}={}", shell_quote(&dir))),
            None => unset.push(key.to_owned()),
        }
    }
    for (key, value) in adapter.launch_fixed_env() {
        parts.push(format!("{key}={}", shell_quote(value)));
    }
    unset.extend(
        adapter
            .launch_unset_keys()
            .iter()
            .map(|key| (*key).to_owned()),
    );
    // Overrides apply as at launch: never over an isolation variable,
    // and never bringing back a credential the launch removes (`env -u
    // KEY KEY=value` would set it again).
    for (key, value) in extra_env {
        let key = key.as_str();
        if !adapter.isolation_env_keys().contains(&key)
            && !adapter.launch_unset_keys().contains(&key)
        {
            parts.push(format!("{key}={}", shell_quote(value)));
        }
    }
    parts.push(snippet_program(
        command,
        crate::user_path::login_shell_path(),
        || adapter.resolved_binary(),
    ));
    if unset.is_empty() {
        return Some(parts.join(" "));
    }
    let flags: Vec<String> = unset.iter().map(|key| format!("-u {key}")).collect();
    Some(format!("env {} {}", flags.join(" "), parts.join(" ")))
}

/// The copyable line that resumes `conversation_id` on the account at
/// `state_dir`, run from `cwd` (the session's folder): the sign-in line's
/// environment and program ([`shell_snippet`]), then the adapter's resume
/// arguments, every word quoted. `None` when the tool can't resume from a
/// terminal.
#[must_use]
pub fn shell_resume_snippet(
    adapter: &dyn HarnessAdapter,
    state_dir: &Path,
    conversation_id: &str,
    cwd: &Path,
) -> Option<String> {
    let args = adapter.resume_args(conversation_id)?;
    let line = shell_snippet(adapter, state_dir, &std::collections::BTreeMap::new())?;
    let args: Vec<String> = args.iter().map(|arg| shell_quote(arg)).collect();
    Some(format!(
        "cd {} && {line} {}",
        shell_quote(&cwd.to_string_lossy()),
        args.join(" ")
    ))
}

/// The program word of the copyable line: `command`, as the user types
/// it, unless their terminal's `PATH` (`shell_path`, when known) doesn't
/// find it — then the binary AI Juggler found, by its full path.
#[must_use]
pub fn snippet_program(
    command: &str,
    shell_path: Option<&std::ffi::OsStr>,
    resolved: impl FnOnce() -> Option<std::path::PathBuf>,
) -> String {
    let typed = shell_path.is_none_or(|path| crate::user_path::find_in(command, path).is_some());
    if typed {
        return command.to_owned();
    }
    resolved().map_or_else(
        || command.to_owned(),
        |bin| shell_quote(&bin.to_string_lossy()),
    )
}

/// Shared adapter handle type used by the registry and supervisor.
pub type SharedAdapter = Arc<dyn HarnessAdapter>;

/// ADR-0008 vocabulary alias: adapters integrate one *Provider*.
pub type ProviderAdapter = dyn HarnessAdapter;

/// AJ-E1530 Req.1: where a tool takes the opt-in limits capture: `key`,
/// top-level in `file` (relative to the account's folder).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitsCapture {
    /// The settings file, relative to the account's folder.
    pub file: &'static str,
    /// The top-level key whose value is set.
    pub key: &'static str,
}
