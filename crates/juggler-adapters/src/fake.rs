//! The fake harness adapter: drives the real `fake-harness` binary through
//! the full [`juggler_adapter::HarnessAdapter`] contract (spec §4).
//!
//! This is the reference implementation adapters can mirror, and the test
//! double the whole system's lifecycle/recovery suites run against.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, HarnessAdapter, InstallStatus, LaunchMode,
    LaunchSpec, SessionEvent, SessionWatcher, VerificationStatus,
};
use juggler_core::model::{Account, HarnessId, Profile};

/// Harness id of the fake adapter.
pub const FAKE_HARNESS_ID: &str = "fake";

/// Adapter over the `fake-harness` binary (see `src/bin/fake_harness.rs`).
#[derive(Debug, Clone)]
pub struct FakeHarnessAdapter {
    bin: PathBuf,
}

impl FakeHarnessAdapter {
    /// Creates an adapter pointing at the fake harness binary.
    #[must_use]
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self { bin: bin.into() }
    }

    /// Creates an adapter from the `AI_JUGGLER_FAKE_HARNESS_BIN` environment
    /// variable, if set.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        std::env::var_os("AI_JUGGLER_FAKE_HARNESS_BIN")
            .map(PathBuf::from)
            .map(Self::new)
    }
}

impl HarnessAdapter for FakeHarnessAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::new(FAKE_HARNESS_ID)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::HEADLESS
            | Capabilities::RESUME
            | Capabilities::TRANSCRIPT_EXPORT
            | Capabilities::TRANSCRIPT_WATCH
            | Capabilities::USAGE_TOKENS
            | Capabilities::FORK
    }

    fn verified_against(&self) -> &'static [&'static str] {
        &["0.1.0"]
    }

    fn discover(&self) -> DiscoveryReport {
        let adapter = self.id();
        match Command::new(&self.bin).arg("--version").output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let version = stdout
                    .split_whitespace()
                    .last()
                    .unwrap_or_default()
                    .to_string();
                let verification = if self.verified_against().contains(&version.as_str()) {
                    VerificationStatus::Verified
                } else {
                    VerificationStatus::Unverified
                };
                DiscoveryReport {
                    adapter,
                    install: InstallStatus::Installed { version },
                    verification,
                    binary: Some(self.bin.clone()),
                    notes: Vec::new(),
                }
            }
            Ok(output) => DiscoveryReport {
                adapter,
                install: InstallStatus::Failed {
                    message: format!("--version exited with {status}", status = output.status),
                },
                verification: VerificationStatus::Unverified,
                binary: Some(self.bin.clone()),
                notes: Vec::new(),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => DiscoveryReport {
                adapter,
                install: InstallStatus::NotFound,
                verification: VerificationStatus::Unverified,
                binary: None,
                notes: Vec::new(),
            },
            Err(error) => DiscoveryReport {
                adapter,
                install: InstallStatus::Failed {
                    message: error.to_string(),
                },
                verification: VerificationStatus::Unverified,
                binary: Some(self.bin.clone()),
                notes: Vec::new(),
            },
        }
    }

    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        let transcripts = state_dir.join("transcripts");
        std::fs::create_dir_all(&transcripts).map_err(|error| AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let mut env = EnvMap::new();
        env.insert(
            "FAKE_HARNESS_HOME".to_string(),
            state_dir.to_string_lossy().into_owned(),
        );
        Ok(env)
    }

    fn launch_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        let effective_cwd = profile
            .default_cwd
            .clone()
            .unwrap_or_else(|| cwd.to_path_buf());
        let transcripts = account.state_dir.join("transcripts");
        let mut args = vec![
            "run".to_string(),
            "--dir".to_string(),
            transcripts.to_string_lossy().into_owned(),
        ];
        let mut env = EnvMap::new();
        env.insert(
            "FAKE_HARNESS_HOME".to_string(),
            account.state_dir.to_string_lossy().into_owned(),
        );
        let spec = match mode {
            LaunchMode::Interactive => juggler_adapter::launch::interactive_spec(
                self.bin.clone(),
                args,
                env,
                effective_cwd,
            ),
            LaunchMode::Headless { prompt } => {
                args.push("--prompt".to_string());
                args.push(prompt.clone());
                juggler_adapter::launch::headless_spec(self.bin.clone(), args, env, effective_cwd)
            }
            LaunchMode::Resume {
                session_id,
                delta_prompt,
            } => {
                args.push("--resume".to_string());
                args.push(session_id.clone());
                if let Some(delta) = delta_prompt {
                    args.push(delta.clone());
                }
                juggler_adapter::launch::interactive_spec(
                    self.bin.clone(),
                    args,
                    env,
                    effective_cwd,
                )
            }
        };
        Ok(spec)
    }

    fn watcher(&self, transcript_root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        Ok(Box::new(FakeWatcher::new(transcript_root)))
    }
}

/// Polling watcher over the fake harness's transcript directory. It counts
/// lines and never parses content, so malformed transcripts cannot crash it.
pub struct FakeWatcher {
    root: PathBuf,
    seen_lines: BTreeMap<PathBuf, usize>,
    reported_ended: BTreeSet<PathBuf>,
    pending: VecDeque<SessionEvent>,
}

impl FakeWatcher {
    #[must_use]
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            seen_lines: BTreeMap::new(),
            reported_ended: BTreeSet::new(),
            pending: VecDeque::new(),
        }
    }

    fn scan(&mut self) {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return;
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        files.sort();
        for path in files {
            let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
                continue;
            };
            let lines = read_line_count(&path);
            let ended = last_line_marks_end(&path);
            let known = self.seen_lines.entry(path.clone()).or_insert(0);
            if *known == 0 {
                self.pending.push_back(SessionEvent::Discovered {
                    native_id: stem.clone(),
                    transcript_path: path.clone(),
                });
                // A first sighting may already contain messages beyond the
                // session-meta line (the run finished before we scanned);
                // replay them as appends.
                if lines > 1 {
                    self.pending.push_back(SessionEvent::MessageAppended {
                        native_id: stem.clone(),
                        transcript_path: path.clone(),
                    });
                }
            } else if lines > *known {
                self.pending.push_back(SessionEvent::MessageAppended {
                    native_id: stem.clone(),
                    transcript_path: path.clone(),
                });
            }
            if ended && !self.reported_ended.contains(&path) {
                self.reported_ended.insert(path.clone());
                self.pending
                    .push_back(SessionEvent::Ended { native_id: stem });
            }
            *known = lines;
        }
    }
}

impl SessionWatcher for FakeWatcher {
    fn poll(&mut self, _timeout: Duration) -> Result<Option<SessionEvent>, AdapterError> {
        if self.pending.is_empty() {
            self.scan();
        }
        Ok(self.pending.pop_front())
    }
}

fn read_line_count(path: &Path) -> usize {
    std::fs::read(path).map_or(0, |bytes| {
        bytes.split(|b| *b == b'\n').count().saturating_sub(1)
    })
}

fn last_line_marks_end(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    content
        .lines()
        .next_back()
        .is_some_and(|line| line.contains(r#""type":"ended""#))
}
