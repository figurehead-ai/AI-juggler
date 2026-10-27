//! The adapter conformance kit (AJ-E1521 Req.2): the rules every
//! adapter keeps, checked the same way for each one.
//!
//! An adapter joins by implementing [`Subject`] — how to build it inside
//! a sandbox, how its tool writes a session, what an existing setup of
//! it looks like, and how its usage is read — and one line of
//! `conformance!` in `adapter_conformance.rs`. The kit then checks:
//!
//! 1. **Isolation of the home** — each account's pinned variables point
//!    inside that account's folder and nowhere else; preparing a folder
//!    writes nothing outside it.
//! 2. **No credential copies** — bringing a setup in never copies its
//!    sign-in (by name or by content), and never changes the original.
//! 3. **Pinned env after overrides** (in every launch the tool offers: a
//!    tool may refuse a headless run or a first message, saying so) —
//!    Launch-settings overrides apply, but never over a pinned variable,
//!    never bring back a removed credential, and never do either in the
//!    copyable shell line.
//! 4. **Refusing option-like resume ids** — an id that the tool would
//!    read as an option is refused, never passed.
//! 5. **Sessions found and resumed** — a session this account started in
//!    a folder is found, and only that one: not another account's, not
//!    another folder's, not one already linked, never a guess between
//!    two; and it resumes by its own id.
//! 6. **Usage labelled honestly** — usage is claimed only with a reader;
//!    every event is keyed for de-duplication, carries its time, and
//!    never calls a computed or estimated number measured.
//! 7. **The agent bridge, honestly** (AJ-E1704, D-30) — an adapter that
//!    declares `AGENT_BRIDGE` adds the bridge to every launch it offers,
//!    and one that doesn't adds nothing. What it adds keeps the launch's
//!    own arguments in order, names the key's variable but never a value
//!    from the environment, writes files only by plain name into AI
//!    Juggler's launch folder (never the account's), points at the bridge
//!    program, and allows no tool by wildcard.
//! 8. **Both runtimes, honestly** (AJ-E2302, D-40) — `PROTOCOL_SERVER`
//!    is declared exactly when the adapter offers a driven runtime. No
//!    launch of either runtime passes a flag that bypasses the tool's
//!    permission prompts or sign-in (`--bare`, …). A driven launch runs the
//!    terminal's own binary, piped, with the same pinned isolation and
//!    removed credentials after hostile overrides; serves a socket only at
//!    the path AI Juggler gave it; refuses option-like ids; and its
//!    terminal view keeps the same rules. A Beta runtime is never a
//!    tool's default.
//! 9. **The tool's own option rules** (the lab of 2026-10-25) — every
//!    launch, of either runtime and in every mode, keeps the rules the
//!    tool states for its options (Claude Code: `--output-format
//!    stream-json` with `-p` needs `--verbose`), so none is refused at
//!    start.
//! 10. **State reports, honestly** (AJ-E2306) — `STATE_REPORTS` is
//!     declared exactly when the adapter adds them. What it adds keeps the
//!     launch's own arguments in order (it may only fold away an AI
//!     Juggler overlay file it replaces), writes files only by plain name
//!     into the launch's folder, names the report program in every command
//!     it adds, copies no value from the environment, and adds no wildcard.
//! 11. **Subagents, honestly** (AJ-E1520, 0.2) — every adapter answers
//!     whether its tool's subagents can be listed: supported, naming the
//!     records it reads, or not supported, saying why (never the trait's
//!     unanswered default). A supported one lists a session's subagents as
//!     the tool records them — each once, the done, the failed and the
//!     still-running one as such, a nested one under the one that started
//!     it — and a session that started none as none; it changes no byte of
//!     the account's folder while it reads; and a conversation it offers
//!     is a file in that folder. One not supported reads nothing.
//!
//! Fixtures only: every home, credential and transcript is synthetic and
//! lives in a temporary sandbox; nothing real is read.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use juggler_adapter::launch::ENV_UNSET;
use juggler_adapter::{AdapterError, Capabilities, HarnessAdapter, LaunchMode};
use juggler_core::model::{
    Account, AccountId, Confidence, Profile, ProfileId, SettingsOverlay, TokenCounts,
};
use juggler_core::quota::{UsageEvent, UsageSource};
#[cfg(unix)]
#[path = "../../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// What an override tries to repoint a pinned variable at.
pub const HOSTILE: &str = "/hostile/elsewhere";
/// What an override tries to bring back for a removed credential.
pub const LEAKED: &str = "leaked-secret-aj-e1521";
/// An ordinary override, which must reach the launch.
pub const PASSTHROUGH: (&str, &str) = ("AJ_KIT_PASSTHROUGH", "kept");

/// One adapter under the kit.
pub trait Subject {
    /// The adapter, with its binary and its user home in the sandbox.
    fn adapter(&self, sandbox: &Sandbox) -> Box<dyn HarnessAdapter>;

    /// Writes session number `n` as the tool records it, run in `cwd`,
    /// into an account's folder. Returns the tool's own session id.
    fn write_session(&self, state_dir: &Path, cwd: &Path, n: u32) -> String;

    /// Builds an existing setup in the sandbox home as the tool leaves
    /// it — settings, history, and a sign-in holding `canary` — and
    /// returns its folder. `None` when the adapter cannot bring setups in.
    fn seed_source(&self, sandbox: &Sandbox, canary: &str) -> Option<PathBuf>;

    /// The sign-in files of a setup, relative to its folder.
    fn credential_files(&self) -> &'static [&'static str];

    /// How the adapter's usage is read; `None` when it has no reader.
    fn usage(&self) -> Option<&dyn UsageSubject>;

    /// Whether the tool's own default folder lives in the sandbox home
    /// (the kit then checks it runs with every pinned variable unset).
    fn sandboxed_default(&self) -> bool;

    /// The tool's own rules for its options, as it states them when one is
    /// broken: `(a, b)` means a launch holding the word `a` must hold the
    /// word `b` too (check 9). Default: none recorded.
    fn flag_rules(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// Records, as the tool does, that the user trusted `cwd` in the
    /// account at `state_dir` (a driven runtime may run only there).
    /// Default: nothing to record.
    fn trust(&self, state_dir: &Path, cwd: &Path) {
        let _ = (state_dir, cwd);
    }

    /// Writes, as the tool records them, a session in `cwd` (in an
    /// account's folder) that started three subagents — one done, one
    /// failed, one still running — and the running one started one more,
    /// done (check 11). Returns the session's record as the conversations
    /// index has it (a file, or a store's key) and its id. Default: none
    /// (an adapter that says it doesn't support subagents).
    fn write_subagents(&self, state_dir: &Path, cwd: &Path) -> Option<(PathBuf, String)> {
        let _ = (state_dir, cwd);
        None
    }

    /// The record of session `id` written by [`Subject::write_session`],
    /// as the conversations index has it (check 11: it started none).
    /// Default: none.
    fn session_record(&self, state_dir: &Path, id: &str) -> Option<PathBuf> {
        let _ = (state_dir, id);
        None
    }

    /// How many subagents a session [`Subject::write_session`] writes
    /// started (OpenCode's has a sub-agent's session beside it). Default:
    /// none.
    fn session_subagents(&self) -> usize {
        0
    }
}

/// How one adapter's usage is read.
pub trait UsageSubject {
    /// Writes a usage fixture into an account's folder: messages with
    /// known counts and times, one of them recorded twice.
    fn seed(&self, state_dir: &Path);
    /// The production reader the usage pipeline runs for this tool.
    fn read(&self, account: &str, state_dir: &Path) -> Vec<UsageEvent>;
    /// Where the events say they came from.
    fn source(&self) -> UsageSource;
    /// How many distinct messages the fixture holds.
    fn distinct_messages(&self) -> usize;
}

/// A throwaway world for one check: a user home, a folder for AI
/// Juggler's accounts, a bin folder and two project folders.
pub struct Sandbox {
    _root: tempfile::TempDir,
    pub home: PathBuf,
    pub accounts: PathBuf,
    pub bin: PathBuf,
    pub project: PathBuf,
    pub other_project: PathBuf,
}

impl Sandbox {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        // Canonical, so `/var` and `/private/var` never differ.
        let base = root.path().canonicalize().unwrap();
        let sandbox = Self {
            home: base.join("home"),
            accounts: base.join("accounts"),
            bin: base.join("bin"),
            project: base.join("work/project"),
            other_project: base.join("work/other"),
            _root: root,
        };
        for dir in [
            &sandbox.home,
            &sandbox.accounts,
            &sandbox.bin,
            &sandbox.project,
            &sandbox.other_project,
        ] {
            std::fs::create_dir_all(dir).unwrap();
        }
        sandbox
    }

    /// A stand-in executable named `name` that prints `version`.
    pub fn stub(&self, name: &str, version: &str) -> PathBuf {
        let path = self.bin.join(name);
        let body = format!("#!/bin/sh\necho {version:?}\n");
        #[cfg(unix)]
        stand_in::write(&path, body);
        #[cfg(not(unix))]
        std::fs::write(&path, body).unwrap();
        path
    }

    /// An account folder (not created).
    pub fn account_dir(&self, name: &str) -> PathBuf {
        self.accounts.join(name)
    }
}

pub fn account(adapter: &dyn HarnessAdapter, state_dir: &Path) -> Account {
    Account {
        id: AccountId::new("kit-account"),
        harness: adapter.id(),
        label: "Kit".into(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

pub fn profile(account: &Account, env: BTreeMap<String, String>) -> Profile {
    Profile {
        id: ProfileId::new("kit-profile"),
        name: "kit".into(),
        account: account.id.clone(),
        settings_overlay: SettingsOverlay {
            model: None,
            env,
            restart_on_crash: None,
        },
        default_cwd: None,
    }
}

pub fn now_ms() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

/// Every file under `dir` with its bytes (missing dir: empty).
pub fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                walk(root, &path, out);
            } else if kind.is_file() {
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                out.insert(relative, std::fs::read(&path).unwrap_or_default());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// Every path (files and folders) under `dir`, absolute.
pub fn paths(dir: &Path) -> BTreeSet<PathBuf> {
    fn walk(dir: &Path, out: &mut BTreeSet<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            out.insert(path.clone());
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                walk(&path, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(dir, &mut out);
    out
}

/// The paths that differ between two [`tree`]s (added, removed or
/// changed) — what a failure names.
pub fn changed(
    before: &BTreeMap<PathBuf, Vec<u8>>,
    after: &BTreeMap<PathBuf, Vec<u8>>,
) -> Vec<PathBuf> {
    before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// The value a launch must carry for each pinned variable of `state_dir`.
fn expected_pins(adapter: &dyn HarnessAdapter, state_dir: &Path) -> Vec<(&'static str, String)> {
    adapter
        .pinned_env(state_dir)
        .into_iter()
        .map(|(key, value)| (key, value.unwrap_or_else(|| ENV_UNSET.to_owned())))
        .collect()
}

fn hostile_overrides(adapter: &dyn HarnessAdapter) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    for key in adapter.isolation_env_keys() {
        env.insert((*key).to_owned(), HOSTILE.to_owned());
    }
    for key in adapter.launch_unset_keys() {
        env.insert((*key).to_owned(), LEAKED.to_owned());
    }
    env.insert(PASSTHROUGH.0.to_owned(), PASSTHROUGH.1.to_owned());
    env
}

fn all_modes(resume_id: &str) -> Vec<LaunchMode> {
    vec![
        LaunchMode::Interactive,
        LaunchMode::Headless {
            prompt: "kit prompt".into(),
        },
        LaunchMode::Primed {
            prompt: "kit prompt".into(),
            read_dirs: Vec::new(),
        },
        LaunchMode::Resume {
            session_id: resume_id.into(),
            delta_prompt: None,
        },
    ]
}

// ---------------------------------------------------------------------------
// 1. Isolation of the home

pub fn isolates_each_home(subject: &dyn Subject) {
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let (a, b) = (sandbox.account_dir("a"), sandbox.account_dir("b"));

    let keys = adapter.isolation_env_keys();
    assert!(!keys.is_empty(), "{name}: names no isolation variable");
    let pins_a = adapter.pinned_env(&a);
    assert!(!pins_a.is_empty(), "{name}: pins nothing to an account");
    for (key, _) in &pins_a {
        assert!(
            keys.contains(key),
            "{name}: pinned {key} is not an isolation variable (overrides could set it)"
        );
    }

    let home_before = tree(&sandbox.home);
    let env_a = adapter
        .prepare_account(&a)
        .expect("prepares a fresh folder");
    let env_b = adapter
        .prepare_account(&b)
        .expect("prepares a second folder");
    for path in paths(&sandbox.accounts) {
        assert!(
            path.starts_with(&a) || path.starts_with(&b),
            "{name}: preparing wrote outside the account folders: {}",
            path.display()
        );
    }
    let touched = changed(&home_before, &tree(&sandbox.home));
    assert!(
        touched.is_empty(),
        "{name}: preparing an account touched the user's home: {touched:?}"
    );
    assert_eq!(
        adapter
            .prepare_account(&a)
            .expect("preparing twice is fine"),
        env_a,
        "{name}: preparing is not idempotent"
    );

    let pins_b = adapter.pinned_env(&b);
    for ((key, value_a), (_, value_b)) in pins_a.iter().zip(&pins_b) {
        let value_a = value_a
            .as_deref()
            .unwrap_or_else(|| panic!("{name}: {key} unset for a managed folder"));
        let value_b = value_b
            .as_deref()
            .unwrap_or_else(|| panic!("{name}: {key} unset for a managed folder"));
        assert!(
            Path::new(value_a).starts_with(&a),
            "{name}: {key}={value_a} is outside account a's folder"
        );
        assert!(
            !Path::new(value_a).starts_with(&b) && !Path::new(value_b).starts_with(&a),
            "{name}: {key} crosses accounts"
        );
        assert_ne!(value_a, value_b, "{name}: {key} is shared by two accounts");
        assert_eq!(
            env_a.get(*key).map(String::as_str),
            Some(value_a),
            "{name}: prepare_account and pinned_env disagree on {key}"
        );
        assert_eq!(
            env_b.get(*key).map(String::as_str),
            Some(value_b),
            "{name}: prepare_account and pinned_env disagree on {key}"
        );
    }

    // The tool's own sign-in flow runs in the same isolation.
    if let Ok(login) = adapter.login_spec(&a) {
        for (key, value) in expected_pins(adapter.as_ref(), &a) {
            assert_eq!(
                login.env.get(key),
                Some(&value),
                "{name}: the sign-in flow doesn't pin {key}"
            );
        }
        for key in adapter.launch_unset_keys() {
            assert_eq!(
                login.env.get(*key).map(String::as_str),
                Some(ENV_UNSET),
                "{name}: the sign-in flow inherits {key}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. No credential copies

/// One way of bringing a setup in.
type Import = fn(
    &dyn HarnessAdapter,
    &Path,
    &Path,
) -> Result<juggler_adapter::environment::ImportOutcome, AdapterError>;

pub fn copies_no_credentials(subject: &dyn Subject) {
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let canary = "canary-credential-aj-e1521-7f3c";

    let Some(source) = subject.seed_source(&sandbox, canary) else {
        // No bring-in: every way in refuses, and nothing is created.
        let empty = sandbox.home.join("setup");
        std::fs::create_dir_all(&empty).unwrap();
        let dest = sandbox.account_dir("copy");
        for result in [
            adapter.import_account(&empty, &dest),
            adapter.import_account_degraded(&empty, &dest),
        ] {
            assert!(
                matches!(result, Err(AdapterError::Unsupported(_))),
                "{name}: claims no bring-in but did not refuse"
            );
        }
        assert!(!dest.exists(), "{name}: a refused copy left a folder");
        return;
    };
    for file in subject.credential_files() {
        assert!(
            source.join(file).is_file(),
            "{name}: the fixture setup lacks its sign-in file {file}"
        );
    }
    let before = tree(&source);
    assert!(
        before.values().any(|bytes| contains(bytes, canary)),
        "{name}: the fixture holds no sign-in to protect"
    );

    let _ = adapter.inspect_source(&source);
    let _ = adapter.adopt_account(&source);
    let imports: [(&str, Import); 2] = [
        ("copy", |adapter, source, dest| {
            adapter.import_account(source, dest)
        }),
        ("settings-only copy", |adapter, source, dest| {
            adapter.import_account_degraded(source, dest)
        }),
    ];
    let mut copied_any = false;
    for (index, (kind, import)) in imports.iter().enumerate() {
        let dest = sandbox.account_dir(&format!("copy-{index}"));
        match import(adapter.as_ref(), &source, &dest) {
            Ok(_) => {
                copied_any = true;
                for (path, _) in tree(&dest) {
                    for file in subject.credential_files() {
                        assert!(
                            !path.ends_with(Path::new(file).file_name().unwrap()),
                            "{name}: the {kind} copied the sign-in file {file} to {}",
                            path.display()
                        );
                    }
                }
                for (path, bytes) in tree(&dest) {
                    assert!(
                        !contains(&bytes, canary),
                        "{name}: the {kind} carried the sign-in into {}",
                        path.display()
                    );
                }
            }
            Err(AdapterError::Unsupported(_)) => {
                assert!(!dest.exists(), "{name}: a refused {kind} left a folder");
            }
            Err(error) => panic!("{name}: the {kind} failed on a valid setup: {error}"),
        }
    }
    assert!(
        copied_any,
        "{name}: seeds a setup but brings nothing in (return None instead)"
    );
    let touched = changed(&before, &tree(&source));
    assert!(
        touched.is_empty(),
        "{name}: bringing a setup in changed the original: {touched:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Pinned env after overrides

pub fn pins_env_after_overrides(subject: &dyn Subject) {
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let dir = sandbox.account_dir("a");
    adapter.prepare_account(&dir).unwrap();

    let mut folders = vec![dir.clone()];
    if subject.sandboxed_default() {
        let default = adapter
            .vendor_default_dir()
            .expect("a sandboxed default folder");
        assert!(
            default.starts_with(&sandbox.home),
            "{name}: the default folder is not the sandbox's"
        );
        assert!(
            adapter
                .pinned_env(&default)
                .iter()
                .all(|(_, value)| value.is_none()),
            "{name}: the tool's own folder must run with its variables unset"
        );
        folders.push(default);
    }

    for folder in &folders {
        let account = account(adapter.as_ref(), folder);
        let overrides = hostile_overrides(adapter.as_ref());
        let recipe = profile(&account, overrides.clone());
        let pins = expected_pins(adapter.as_ref(), folder);
        for mode in all_modes("kit-session-1") {
            let spec = match adapter.launch_spec(&account, &recipe, &sandbox.project, &mode) {
                Ok(spec) => spec,
                // A tool may not offer a headless run or a first message
                // (AJ-E1603: Kimi Code's plans are interactive only); it
                // says so, and nothing launches. Every tool starts and
                // resumes.
                Err(AdapterError::Unsupported(_))
                    if matches!(
                        mode,
                        LaunchMode::Headless { .. } | LaunchMode::Primed { .. }
                    ) =>
                {
                    continue;
                }
                Err(error) => panic!("{name}: {mode:?} launch failed: {error}"),
            };
            for (key, value) in &pins {
                assert_eq!(
                    spec.env.get(*key),
                    Some(value),
                    "{name}: an override moved {key} in a {mode:?} launch"
                );
            }
            for key in adapter.launch_unset_keys() {
                assert_eq!(
                    spec.env.get(*key).map(String::as_str),
                    Some(ENV_UNSET),
                    "{name}: an override brought back {key} in a {mode:?} launch"
                );
            }
            for (key, value) in adapter.launch_fixed_env() {
                assert_eq!(
                    spec.env.get(*key).map(String::as_str),
                    Some(*value),
                    "{name}: an override changed the fixed {key} in a {mode:?} launch"
                );
            }
            assert_eq!(
                spec.env.get(PASSTHROUGH.0).map(String::as_str),
                Some(PASSTHROUGH.1),
                "{name}: an ordinary override never reached the {mode:?} launch"
            );
        }

        // The copyable line keeps the same rules.
        if let Some(line) =
            juggler_adapter::adapter::shell_snippet(adapter.as_ref(), folder, &overrides)
        {
            assert!(
                !line.contains(HOSTILE),
                "{name}: the shell line lets an override move a pinned variable: {line}"
            );
            assert!(
                !line.contains(LEAKED),
                "{name}: the shell line brings back a removed credential: {line}"
            );
            for (key, value) in adapter.pinned_env(folder) {
                match value {
                    Some(value) => assert!(
                        line.contains(&format!(
                            "{key}={}",
                            juggler_adapter::adapter::shell_quote(&value)
                        )),
                        "{name}: the shell line doesn't pin {key}: {line}"
                    ),
                    None => assert!(
                        line.contains(&format!("-u {key}")),
                        "{name}: the shell line doesn't unset {key}: {line}"
                    ),
                }
            }
            for key in adapter.launch_unset_keys() {
                assert!(
                    line.contains(&format!("-u {key}")),
                    "{name}: the shell line doesn't remove {key}: {line}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Refusing option-like resume ids

pub fn refuses_option_like_resume_ids(subject: &dyn Subject) {
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let dir = sandbox.account_dir("a");
    adapter.prepare_account(&dir).unwrap();
    let account = account(adapter.as_ref(), &dir);
    let recipe = profile(&account, BTreeMap::new());
    let resume = |id: &str| {
        adapter.launch_spec(
            &account,
            &recipe,
            &sandbox.project,
            &LaunchMode::Resume {
                session_id: id.into(),
                delta_prompt: None,
            },
        )
    };
    for id in [
        "",
        "-",
        "--last",
        "-c",
        "--continue",
        "--session=other",
        "-x",
    ] {
        assert!(
            matches!(resume(id), Err(AdapterError::InvalidLaunch(_))),
            "{name}: resumed the option-like id {id:?}"
        );
    }
    let spec = resume("kit-session-1").expect("an ordinary id resumes");
    assert_eq!(
        spec.args
            .iter()
            .filter(|arg| *arg == "kit-session-1")
            .count(),
        1,
        "{name}: the id is not one argument of its own: {:?}",
        spec.args
    );
}

// ---------------------------------------------------------------------------
// 5. Sessions found and resumed

pub fn finds_and_resumes_sessions(subject: &dyn Subject) {
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    assert!(
        adapter.capabilities().contains(Capabilities::RESUME),
        "{name}: every adapter resumes"
    );
    let (a, b) = (sandbox.account_dir("a"), sandbox.account_dir("b"));
    adapter.prepare_account(&a).unwrap();
    adapter.prepare_account(&b).unwrap();
    let launched = now_ms() - 1_000;

    let first = subject.write_session(&a, &sandbox.project, 1);
    let found = adapter
        .locate_transcript(&a, &sandbox.project, launched, &[])
        .unwrap_or_else(|| panic!("{name}: the session started in the folder isn't found"));
    assert_eq!(found.native_id.as_deref(), Some(first.as_str()), "{name}");

    assert!(
        adapter
            .locate_transcript(&b, &sandbox.project, launched, &[])
            .is_none(),
        "{name}: found account a's session in account b"
    );
    assert!(
        adapter
            .locate_transcript(&a, &sandbox.other_project, launched, &[])
            .is_none(),
        "{name}: found a session of another folder"
    );
    assert!(
        adapter
            .locate_transcript(
                &a,
                &sandbox.project,
                launched,
                std::slice::from_ref(&found.path)
            )
            .is_none(),
        "{name}: found a session already linked to another"
    );
    assert!(
        adapter
            .locate_transcript(&a, &sandbox.project, now_ms() + 60_000, &[])
            .is_none(),
        "{name}: found a session older than the launch"
    );

    let second = subject.write_session(&a, &sandbox.project, 2);
    assert!(
        adapter
            .locate_transcript(&a, &sandbox.project, launched, &[])
            .is_none(),
        "{name}: guessed between two sessions in one folder"
    );
    let next = adapter
        .locate_transcript(
            &a,
            &sandbox.project,
            launched,
            std::slice::from_ref(&found.path),
        )
        .unwrap_or_else(|| panic!("{name}: the remaining session isn't found"));
    assert_eq!(next.native_id.as_deref(), Some(second.as_str()), "{name}");

    let account = account(adapter.as_ref(), &a);
    let spec = adapter
        .launch_spec(
            &account,
            &profile(&account, BTreeMap::new()),
            &sandbox.project,
            &LaunchMode::Resume {
                session_id: first.clone(),
                delta_prompt: None,
            },
        )
        .expect("the found session resumes");
    assert!(
        spec.args.contains(&first),
        "{name}: the resume doesn't name the session: {:?}",
        spec.args
    );
    for (key, value) in expected_pins(adapter.as_ref(), &a) {
        assert_eq!(
            spec.env.get(key),
            Some(&value),
            "{name}: resume moved {key}"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Usage labelled honestly

pub fn labels_usage_honestly(subject: &dyn Subject) {
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let claims = adapter.capabilities().contains(Capabilities::USAGE_TOKENS);
    let Some(usage) = subject.usage() else {
        assert!(!claims, "{name}: claims token usage with no reader");
        return;
    };
    assert!(claims, "{name}: reads usage but doesn't say so");

    let (a, b) = (sandbox.account_dir("a"), sandbox.account_dir("b"));
    adapter.prepare_account(&a).unwrap();
    adapter.prepare_account(&b).unwrap();
    assert!(
        usage.read("acct-b", &b).is_empty(),
        "{name}: invented usage for an account that used nothing"
    );
    usage.seed(&a);
    let before = tree(&a);
    let events = usage.read("acct-a", &a);
    let touched = changed(&before, &tree(&a));
    assert!(
        touched.is_empty(),
        "{name}: reading usage wrote to the folder: {touched:?}"
    );

    let keys = |events: &[UsageEvent]| -> Vec<String> {
        events
            .iter()
            .map(|event| {
                event
                    .msg_uuid
                    .clone()
                    .unwrap_or_else(|| panic!("{name}: an event without a key double-counts"))
            })
            .collect()
    };
    let first = keys(&events);
    let distinct: BTreeSet<&String> = first.iter().collect();
    assert_eq!(
        distinct.len(),
        usage.distinct_messages(),
        "{name}: messages and keys don't match one to one"
    );
    assert_eq!(
        keys(&usage.read("acct-a", &a)),
        first,
        "{name}: reading again keys the same messages differently"
    );
    for event in &events {
        assert_eq!(event.account, "acct-a", "{name}: event of another account");
        assert_eq!(event.source, usage.source(), "{name}: wrong source");
        assert!(event.ts > 0, "{name}: an event without its time");
        assert_ne!(
            event.tokens,
            TokenCounts::default(),
            "{name}: an empty event"
        );
        if event.credits.is_some() {
            assert_eq!(
                event.confidence,
                Confidence::Computed,
                "{name}: credits are computed, never measured"
            );
            assert!(event.model.is_some(), "{name}: credits without a model");
        }
        if event.confidence == Confidence::Measured {
            assert!(
                event.credits.is_none(),
                "{name}: a measured event carries computed credits"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 7. The agent bridge, honestly

/// A value no adapter may copy into what it adds: it is only in the
/// launch's environment, where the key lives.
pub const KEY_CANARY: &str = "aj-e1704-key-canary";

pub fn adds_the_agent_bridge_honestly(subject: &dyn Subject) {
    use juggler_adapter::agent_bridge::BridgeLaunch;
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let dir = sandbox.account_dir("a");
    adapter.prepare_account(&dir).unwrap();
    let account = account(adapter.as_ref(), &dir);
    let mut env = BTreeMap::new();
    env.insert("JUGGLER_AGENT_TOKEN".to_owned(), KEY_CANARY.to_owned());
    let recipe = profile(&account, env);
    let launch_dir = sandbox
        .accounts
        .parent()
        .unwrap()
        .join("overlays/agent/kit-session");
    let bridge = BridgeLaunch {
        program: sandbox.bin.join("juggler-agent"),
        server: "juggler".into(),
        tools: vec!["whoami".into(), "pool_roster".into(), "set_status".into()],
        env_names: vec!["JUGGLER_AGENT_TOKEN".into(), "JUGGLER_AGENT_SOCKET".into()],
        dir: launch_dir.clone(),
    };
    let declares = adapter.capabilities().contains(Capabilities::AGENT_BRIDGE);
    let home = dir.to_string_lossy().into_owned();
    for mode in all_modes("kit-session-1") {
        // A tool may refuse a mode (it says so); the rest must hold.
        let Ok(spec) = adapter.launch_spec(&account, &recipe, &sandbox.project, &mode) else {
            continue;
        };
        let injection = adapter.agent_bridge(&bridge, &spec.args);
        assert_eq!(
            injection.is_some(),
            declares,
            "{name}: AGENT_BRIDGE is declared {declares} but the injection is {injection:?} ({mode:?})"
        );
        let Some(injection) = injection else { continue };
        let applied = injection.applied(&spec.args);
        let mut rest = applied.iter();
        assert!(
            spec.args.iter().all(|arg| rest.any(|seen| seen == arg)),
            "{name}: the launch's own arguments aren't kept in order: {applied:?}"
        );
        let mut added: Vec<String> = injection.args.clone();
        added.extend(injection.files.iter().map(|file| file.contents.clone()));
        let all = added.join("\n");
        assert!(
            !all.contains(KEY_CANARY),
            "{name}: a value from the environment was copied: {all}"
        );
        assert!(
            !all.contains(&home),
            "{name}: it points into the account's folder: {all}"
        );
        assert!(
            !all.contains('*'),
            "{name}: a wildcard allows more than AI Juggler's tools: {all}"
        );
        assert!(
            all.contains(&bridge.program.to_string_lossy().into_owned()),
            "{name}: it doesn't start the bridge program: {all}"
        );
        assert!(
            all.contains("JUGGLER_AGENT_TOKEN"),
            "{name}: the key's variable isn't passed on: {all}"
        );
        for file in &injection.files {
            assert!(
                !file.name.is_empty()
                    && !file.name.contains(['/', '\\'])
                    && !file.name.starts_with('.'),
                "{name}: a file outside the launch's folder: {:?}",
                file.name
            );
        }
        for arg in &injection.args {
            if arg.starts_with('/') && !arg.starts_with(&*bridge.program.to_string_lossy()) {
                assert!(
                    Path::new(arg).starts_with(&launch_dir),
                    "{name}: a path outside AI Juggler's launch folder: {arg}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8. Both runtimes, honestly

pub fn runs_both_runtimes_honestly(subject: &dyn Subject) {
    use juggler_adapter::runtime::{
        DrivenLaunch, Maturity, Runtime, TerminalView, Transport, ViewOf, has_forbidden_arg,
    };
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let dir = sandbox.account_dir("a");
    adapter.prepare_account(&dir).unwrap();
    let account = account(adapter.as_ref(), &dir);
    let recipe = profile(&account, hostile_overrides(adapter.as_ref()));
    let pins = expected_pins(adapter.as_ref(), &dir);
    let keeps_rules = |what: &str, spec: &juggler_adapter::LaunchSpec| {
        assert!(
            !has_forbidden_arg(&spec.args),
            "{name}: {what} bypasses the tool's prompts or sign-in: {:?}",
            spec.args
        );
        for (key, value) in &pins {
            assert_eq!(
                spec.env.get(*key),
                Some(value),
                "{name}: an override moved {key} in {what}"
            );
        }
        for key in adapter.launch_unset_keys() {
            assert_eq!(
                spec.env.get(*key).map(String::as_str),
                Some(ENV_UNSET),
                "{name}: an override brought back {key} in {what}"
            );
        }
        let all = format!("{:?}", spec.args);
        assert!(
            !all.contains(LEAKED),
            "{name}: {what} carries a removed credential"
        );
    };
    for mode in all_modes("kit-session-1") {
        if let Ok(spec) = adapter.launch_spec(&account, &recipe, &sandbox.project, &mode) {
            keeps_rules(&format!("the terminal's {mode:?} launch"), &spec);
        }
    }
    let declares = adapter
        .capabilities()
        .contains(Capabilities::PROTOCOL_SERVER);
    let driven = adapter.driven();
    assert_eq!(
        driven.is_some(),
        declares,
        "{name}: PROTOCOL_SERVER is declared {declares} but driven() is {}",
        driven.is_some()
    );
    let Some(driven) = driven else { return };
    if driven.default_runtime() == Runtime::Driven {
        assert_eq!(
            driven.maturity(),
            Maturity::Stable,
            "{name}: a Beta runtime is never a tool's default"
        );
    }
    subject.trust(&dir, &sandbox.project);
    let runtime_dir = sandbox.accounts.parent().unwrap().join("runtime");
    let socket = runtime_dir.join("kit.sock");
    let terminal = adapter
        .launch_spec(
            &account,
            &recipe,
            &sandbox.project,
            &LaunchMode::Interactive,
        )
        .unwrap();
    for resume in [None, Some("kit-session-1".to_owned())] {
        let launch = DrivenLaunch {
            socket: socket.clone(),
            resume: resume.clone(),
            fresh_id: "0b1c2d3e-0000-4000-8000-0000000000aa".into(),
        };
        let spec = driven
            .driven_spec(&account, &recipe, &sandbox.project, &launch)
            .unwrap_or_else(|error| panic!("{name}: no driven launch ({resume:?}): {error}"));
        let what = format!("the driven launch ({resume:?})");
        keeps_rules(&what, &spec.launch);
        assert_eq!(
            spec.launch.program, terminal.program,
            "{name}: {what} runs another binary than the terminal (never a bundled copy)"
        );
        assert!(!spec.launch.needs_pty, "{name}: {what} asks for a PTY");
        if let Transport::Socket(path) = &spec.transport {
            assert_eq!(path, &socket, "{name}: {what} serves a socket elsewhere");
        }
        for arg in &spec.launch.args {
            if arg.contains(&*runtime_dir.to_string_lossy()) {
                assert!(
                    arg.contains(&*socket.to_string_lossy()),
                    "{name}: {what} names another path in the runtime folder: {arg}"
                );
            }
        }
        if let Some(id) = &resume {
            assert!(
                spec.launch.args.iter().any(|arg| arg.contains(id.as_str()))
                    || spec.native_id.as_deref() == Some(id.as_str()),
                "{name}: {what} doesn't continue {id}"
            );
        }
    }
    for id in ["", "-", "--last", "--session=other"] {
        let launch = DrivenLaunch {
            socket: socket.clone(),
            resume: Some(id.into()),
            fresh_id: "0b1c2d3e-0000-4000-8000-0000000000aa".into(),
        };
        assert!(
            matches!(
                driven.driven_spec(&account, &recipe, &sandbox.project, &launch),
                Err(AdapterError::InvalidLaunch(_))
            ),
            "{name}: a driven launch resumed the option-like id {id:?}"
        );
    }
    let view = driven
        .terminal_view(
            &account,
            &recipe,
            &sandbox.project,
            &ViewOf {
                native_id: "kit-session-1".into(),
                socket: Some(socket.clone()),
            },
        )
        .unwrap_or_else(|error| panic!("{name}: no terminal view: {error}"));
    let (TerminalView::Attach(spec) | TerminalView::AfterRelease(spec)) = &view;
    keeps_rules("the terminal view", spec);
    assert!(spec.needs_pty, "{name}: the terminal view has no terminal");
}

// ---------------------------------------------------------------------------
// 9. The tool's own option rules

pub fn keeps_the_tools_own_option_rules(subject: &dyn Subject) {
    use juggler_adapter::runtime::{DrivenLaunch, TerminalView, ViewOf};
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let dir = sandbox.account_dir("a");
    adapter.prepare_account(&dir).unwrap();
    let account = account(adapter.as_ref(), &dir);
    let recipe = profile(&account, BTreeMap::new());
    subject.trust(&dir, &sandbox.project);
    let mut launches: Vec<(String, Vec<String>)> = Vec::new();
    for mode in all_modes("kit-session-1") {
        if let Ok(spec) = adapter.launch_spec(&account, &recipe, &sandbox.project, &mode) {
            launches.push((format!("{mode:?}"), spec.args));
        }
    }
    if let Some(driven) = adapter.driven() {
        for resume in [None, Some("kit-session-1".to_owned())] {
            let launch = DrivenLaunch {
                socket: sandbox.accounts.join("kit.sock"),
                resume,
                fresh_id: "0b1c2d3e-0000-4000-8000-0000000000aa".into(),
            };
            if let Ok(spec) = driven.driven_spec(&account, &recipe, &sandbox.project, &launch) {
                launches.push(("the driven launch".into(), spec.launch.args));
            }
        }
        let of = ViewOf {
            native_id: "kit-session-1".into(),
            socket: None,
        };
        if let Ok(TerminalView::Attach(spec) | TerminalView::AfterRelease(spec)) =
            driven.terminal_view(&account, &recipe, &sandbox.project, &of)
        {
            launches.push(("the terminal view".into(), spec.args));
        }
    }
    assert!(!launches.is_empty(), "{name}: no launch to check");
    for (what, args) in &launches {
        for (needs, also) in subject.flag_rules() {
            if args.iter().any(|arg| arg == needs) {
                assert!(
                    args.iter().any(|arg| arg == also),
                    "{name}: {what} has {needs} without {also}, which the tool refuses: {args:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 10. State reports, honestly

pub fn adds_state_reports_honestly(subject: &dyn Subject) {
    use juggler_adapter::reports::ReportsLaunch;
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let dir = sandbox.account_dir("a");
    adapter.prepare_account(&dir).unwrap();
    let account = account(adapter.as_ref(), &dir);
    let mut env = BTreeMap::new();
    env.insert("JUGGLER_REPORT_TOKEN".to_owned(), KEY_CANARY.to_owned());
    let recipe = profile(&account, env);
    let overlays = sandbox.accounts.parent().unwrap().join("overlays");
    let launch_dir = overlays.join("runtime/kit-session");
    let launch = ReportsLaunch {
        program: sandbox.bin.join("juggler-agent"),
        dir: launch_dir.clone(),
        overlays: overlays.clone(),
    };
    let declares = adapter.capabilities().contains(Capabilities::STATE_REPORTS);
    let home = dir.to_string_lossy().into_owned();
    for mode in all_modes("kit-session-1") {
        let Ok(spec) = adapter.launch_spec(&account, &recipe, &sandbox.project, &mode) else {
            continue;
        };
        let injection = adapter.state_reports(&launch, &spec.args);
        assert_eq!(
            injection.is_some(),
            declares,
            "{name}: STATE_REPORTS is declared {declares} but the injection is {injection:?}"
        );
        let Some(injection) = injection else { continue };
        let ours = |arg: &str| Path::new(arg).starts_with(&overlays);
        let mut rest = injection.args.iter();
        assert!(
            spec.args
                .iter()
                .filter(|arg| !ours(arg))
                .all(|arg| rest.any(|seen| seen == arg)),
            "{name}: the launch's own arguments aren't kept in order: {:?}",
            injection.args
        );
        // What it adds: the arguments the launch didn't have, and its files.
        let added: Vec<&String> = injection
            .args
            .iter()
            .filter(|arg| !spec.args.contains(arg))
            .collect();
        let all = format!(
            "{added:?}\n{}",
            injection
                .files
                .iter()
                .map(|f| f.contents.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(
            !all.contains(KEY_CANARY),
            "{name}: a value from the environment was copied"
        );
        assert!(
            !all.contains(&home),
            "{name}: it points into the account's folder"
        );
        assert!(!all.contains('*'), "{name}: a wildcard: {all}");
        assert!(
            all.contains(&*launch.program.to_string_lossy()),
            "{name}: no command runs the report program"
        );
        for file in &injection.files {
            assert!(
                !file.name.is_empty()
                    && !file.name.contains(['/', '\\'])
                    && !file.name.starts_with('.'),
                "{name}: a file outside the launch's folder: {:?}",
                file.name
            );
        }
        for arg in &injection.args {
            if ours(arg) {
                assert!(
                    Path::new(arg).starts_with(&launch_dir),
                    "{name}: names an overlay file it didn't write: {arg}"
                );
            }
        }
    }
}

/// Check 11: subagents, honestly (AJ-E1520).
pub fn lists_subagents_honestly(subject: &dyn Subject) {
    use juggler_adapter::subagents::{
        SubagentConversations, SubagentScope, SubagentStatus, SubagentSupport,
    };
    let sandbox = Sandbox::new();
    let adapter = subject.adapter(&sandbox);
    let name = adapter.id();
    let dir = sandbox.account_dir("a");
    adapter.prepare_account(&dir).unwrap();
    let support = adapter.subagent_support();
    match support {
        SubagentSupport::Unanswered => {
            panic!("{name}: it doesn't answer whether its subagents can be listed")
        }
        SubagentSupport::NotSupported(reason) => {
            assert!(
                !reason.trim().is_empty(),
                "{name}: not supported, without saying why"
            );
            assert!(
                subject.write_subagents(&dir, &sandbox.project).is_none(),
                "{name}: a subagents fixture for a tool it says it can't read"
            );
            let id = subject.write_session(&dir, &sandbox.project, 1);
            let record = subject
                .session_record(&dir, &id)
                .unwrap_or_else(|| dir.join("none"));
            assert!(
                adapter
                    .subagent_reader(SubagentScope {
                        path: &record,
                        id: &id
                    })
                    .is_none(),
                "{name}: not supported, yet it offers a reader"
            );
            return;
        }
        SubagentSupport::Supported { route, .. } => {
            assert!(
                !route.trim().is_empty(),
                "{name}: supported, without naming the records it reads"
            );
        }
    }
    let (path, id) = subject
        .write_subagents(&dir, &sandbox.project)
        .unwrap_or_else(|| panic!("{name}: supported, but the kit has no subagents fixture"));
    let before = tree(&dir);
    let mut reader = adapter
        .subagent_reader(SubagentScope {
            path: &path,
            id: &id,
        })
        .unwrap_or_else(|| panic!("{name}: no reader for its own session's record"));
    let mut rounds = 0;
    while !reader
        .refresh()
        .unwrap_or_else(|_| panic!("{name}: its session's record didn't read"))
    {
        rounds += 1;
        assert!(
            rounds < 100,
            "{name}: a small session never finishes reading"
        );
    }
    let list = reader.subagents();
    // A second refresh with nothing new changes nothing.
    reader.refresh().unwrap();
    let again = reader.subagents();
    assert_eq!(
        list.len(),
        again.len(),
        "{name}: a refresh with nothing new changed the list"
    );
    assert_eq!(
        list.len(),
        4,
        "{name}: the fixture's four subagents, each once"
    );
    let ids: BTreeSet<&str> = list.iter().map(|agent| agent.id.as_str()).collect();
    assert_eq!(ids.len(), 4, "{name}: an id listed twice");
    let mut top: Vec<SubagentStatus> = list
        .iter()
        .filter(|agent| agent.depth == 1)
        .map(|agent| agent.status)
        .collect();
    top.sort_by_key(|status| format!("{status:?}"));
    assert_eq!(
        top,
        [
            SubagentStatus::Done,
            SubagentStatus::Failed,
            SubagentStatus::Running
        ],
        "{name}: the session's three don't stand as recorded"
    );
    let running = list
        .iter()
        .find(|agent| agent.depth == 1 && agent.status == SubagentStatus::Running)
        .unwrap();
    let nested: Vec<_> = list.iter().filter(|agent| agent.depth == 2).collect();
    assert_eq!(nested.len(), 1, "{name}: the nested one isn't at depth 2");
    assert_eq!(
        nested[0].parent.as_deref(),
        Some(running.id.as_str()),
        "{name}: the nested one isn't under the one that started it"
    );
    assert_eq!(
        nested[0].status,
        SubagentStatus::Done,
        "{name}: the nested one's end"
    );
    for agent in &list {
        if !agent.status.is_final() {
            assert_eq!(agent.ended_ms, None, "{name}: an end for one not ended");
        }
        if let (Some(start), Some(end)) = (agent.started_ms, agent.ended_ms) {
            assert!(end >= start, "{name}: one ended before it started");
        }
        if agent.depth == 1 {
            assert_eq!(agent.parent, None, "{name}: the session's own has a parent");
        }
    }
    if let SubagentSupport::Supported {
        conversations: SubagentConversations::Read,
        ..
    } = support
    {
        assert!(
            list.iter().any(|agent| agent.transcript.is_some()),
            "{name}: conversations are read, yet none is offered"
        );
    }
    for transcript in list.iter().filter_map(|agent| agent.transcript.as_ref()) {
        assert!(
            transcript.starts_with(&dir) && transcript.is_file(),
            "{name}: a conversation that isn't a file in the account's folder"
        );
    }
    assert!(
        tree(&dir) == before,
        "{name}: reading its subagents changed the account's folder"
    );
    // A plain session lists only what it started.
    let id = subject.write_session(&dir, &sandbox.other_project, 7);
    if let Some(path) = subject.session_record(&dir, &id) {
        let mut reader = adapter
            .subagent_reader(SubagentScope {
                path: &path,
                id: &id,
            })
            .unwrap_or_else(|| panic!("{name}: no reader for a plain session"));
        while !reader.refresh().unwrap() {}
        assert_eq!(
            reader.subagents().len(),
            subject.session_subagents(),
            "{name}: subagents a plain session didn't start"
        );
    }
}

/// Instantiates the whole kit for one subject as a test module.
#[macro_export]
macro_rules! conformance {
    ($module:ident, $subject:expr) => {
        mod $module {
            use super::*;

            #[test]
            fn isolates_each_home() {
                conformance::isolates_each_home(&$subject);
            }

            #[test]
            fn copies_no_credentials() {
                conformance::copies_no_credentials(&$subject);
            }

            #[test]
            fn pins_env_after_overrides() {
                conformance::pins_env_after_overrides(&$subject);
            }

            #[test]
            fn refuses_option_like_resume_ids() {
                conformance::refuses_option_like_resume_ids(&$subject);
            }

            #[test]
            fn finds_and_resumes_sessions() {
                conformance::finds_and_resumes_sessions(&$subject);
            }

            #[test]
            fn labels_usage_honestly() {
                conformance::labels_usage_honestly(&$subject);
            }

            #[test]
            fn adds_the_agent_bridge_honestly() {
                conformance::adds_the_agent_bridge_honestly(&$subject);
            }

            #[test]
            fn runs_both_runtimes_honestly() {
                conformance::runs_both_runtimes_honestly(&$subject);
            }

            #[test]
            fn keeps_the_tools_own_option_rules() {
                conformance::keeps_the_tools_own_option_rules(&$subject);
            }

            #[test]
            fn adds_state_reports_honestly() {
                conformance::adds_state_reports_honestly(&$subject);
            }

            #[test]
            fn lists_subagents_honestly() {
                conformance::lists_subagents_honestly(&$subject);
            }
        }
    };
}
