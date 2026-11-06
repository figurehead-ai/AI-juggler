//! The pipeline tests' shared fixtures: Claude-shaped conversations on
//! disk and a minimal real-filesystem `HandoverSupport`.
#![allow(dead_code)] // each test binary uses its own part

use std::path::{Path, PathBuf};

use juggler_adapter::capabilities::Capabilities;
use juggler_adapter::discovery::DiscoveryReport;
use juggler_adapter::error::AdapterError;
use juggler_adapter::handover::{
    BundleFile, BundleManifest, Fidelity, HandoverSupport, InstallOutcome, SessionBundle, TurnText,
};
use juggler_adapter::launch::{EnvMap, LaunchMode, LaunchSpec};
use juggler_adapter::model::{Account, HarnessId, Profile};
use juggler_adapter::prelude::*;
use juggler_adapter::watcher::SessionWatcher;
use juggler_core::model::{Account as CoreAccount, AccountId, Timestamp, TranscriptRef};

pub fn line(uuid: &str, parent: Option<&str>, session: &str, cwd: &str) -> String {
    serde_json::json!({
        "uuid": uuid, "parentUuid": parent, "sessionId": session,
        "cwd": cwd, "type": "user",
        "message": {"role": "user", "content": "work"},
    })
    .to_string()
}

pub fn seed_tree(config: &Path, cwd: &str, session: &str) -> PathBuf {
    let dir = config.join("projects").join(cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{session}.jsonl")),
        format!(
            "{}\n{}\n",
            line("u1", None, session, cwd),
            line("u2", Some("u1"), session, cwd)
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{session}-agent-1.jsonl")),
        format!("{}\n", line("s1", None, session, cwd)),
    )
    .unwrap();
    dir.join(format!("{session}.jsonl"))
}

pub fn target(state_dir: &Path) -> CoreAccount {
    CoreAccount {
        id: AccountId::new("acc-b"),
        harness: HarnessId::new("claude-code"),
        label: "Personal".into(),
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

pub fn dir_hashes(dir: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| {
            let path = entry.path();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                juggler_adapter::handover::sha256_hex(&std::fs::read(&path).unwrap()),
            )
        })
        .collect();
    out.sort();
    out
}

/// A minimal real-filesystem HandoverSupport (Claude-shaped files,
/// Claude-ish rules — enough for pipeline mechanics; the REAL Claude
/// implementation is proven in juggler-adapters' own suite).
pub struct TreeSupport {
    pub harness: HarnessId,
    /// When set, `install_bundle` writes only the first file then
    /// fails (S-F4 fault injection).
    pub partial_install: bool,
    /// When set, `bundle_session` returns a torn tail (S-F2).
    pub torn_tail: bool,
}

impl ToolIdentity for TreeSupport {
    fn id(&self) -> HarnessId {
        self.harness.clone()
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::empty()
    }
    fn verified_against(&self) -> &'static [&'static str] {
        &["1.0.0"]
    }
}

impl ToolDiscovery for TreeSupport {
    fn discover(&self) -> DiscoveryReport {
        DiscoveryReport {
            adapter: self.harness.clone(),
            install: juggler_adapter::discovery::InstallStatus::NotFound,
            verification: juggler_adapter::discovery::VerificationStatus::Unverified,
            binary: None,
            notes: Vec::new(),
        }
    }
}

impl ToolSignIn for TreeSupport {
    fn prepare_account(&self, _state_dir: &Path) -> Result<EnvMap, AdapterError> {
        Ok(EnvMap::new())
    }
}

impl ToolLaunch for TreeSupport {
    fn launch_spec(
        &self,
        _account: &Account,
        _profile: &Profile,
        cwd: &Path,
        _mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        Ok(LaunchSpec {
            program: PathBuf::from("/usr/bin/true"),
            args: Vec::new(),
            env: EnvMap::new(),
            cwd: cwd.to_path_buf(),
            needs_pty: false,
        })
    }
}

impl ToolConfig for TreeSupport {}

impl ToolInventory for TreeSupport {}

impl ToolTranscripts for TreeSupport {
    fn watcher(&self, _root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        Err(AdapterError::Watch("no watcher".into()))
    }
}

impl ToolQuota for TreeSupport {}

impl ToolLive for TreeSupport {}

impl ToolHandover for TreeSupport {}

impl ToolRuntime for TreeSupport {}

impl HandoverSupport for TreeSupport {
    fn bundle_session(&self, reference: &TranscriptRef) -> Result<SessionBundle, AdapterError> {
        let main = std::fs::read(&reference.path)?;
        if self.torn_tail && !juggler_adapter::handover::tail_parses(&main) {
            return Err(AdapterError::Unsupported(
                "source transcript tail does not parse — stop the session first".into(),
            ));
        }
        let dir = reference.path.parent().unwrap();
        let stem = reference
            .path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut files = Vec::new();
        for sibling in juggler_adapter::handover::sibling_paths(dir, &stem) {
            let mut bytes = std::fs::read(&sibling)?;
            if self.torn_tail
                && sibling.file_name().unwrap().to_string_lossy() == format!("{stem}.jsonl")
            {
                bytes.extend_from_slice(b"{\"torn");
            }
            files.push(BundleFile::inline(
                sibling.file_name().unwrap().to_string_lossy().into_owned(),
                bytes,
            ));
        }
        let cwd = juggler_adapter::handover::embedded_cwd(&main).unwrap_or_default();
        Ok(SessionBundle {
            manifest: BundleManifest {
                schema: 1,
                harness: self.harness.clone(),
                native_id: stem,
                cwd: PathBuf::from(cwd),
                harness_version: None,
                created_at: Timestamp::from_millis(0),
                fidelity: Fidelity {
                    level: "F1".into(),
                    caps_applied: vec![],
                },
                redacted: false,
            },
            files,
            scratch: None,
        })
    }

    fn install_bundle(
        &self,
        bundle: &SessionBundle,
        target: &Account,
    ) -> Result<InstallOutcome, AdapterError> {
        let dir = target
            .state_dir
            .join("projects")
            .join(bundle.manifest.cwd.to_string_lossy().replace('/', "-"));
        std::fs::create_dir_all(&dir).unwrap();
        for (idx, file) in bundle.files.iter().enumerate() {
            if self.partial_install && idx == 1 {
                // S-F4: crash after the first file.
                return Err(AdapterError::AccountDir {
                    path: dir.clone(),
                    message: "injected mid-copy crash".into(),
                });
            }
            std::fs::write(dir.join(&file.name), &file.bytes).unwrap();
        }
        Ok(InstallOutcome {
            reference: TranscriptRef {
                path: dir.join(format!("{}.jsonl", bundle.manifest.native_id)),
                native_id: Some(bundle.manifest.native_id.clone()),
            },
            forked: false,
            target_dir_fallback: false,
            placed: Vec::new(),
        })
    }

    fn prime_prompt(&self, _turns: &[TurnText]) -> Result<String, AdapterError> {
        Ok(String::new())
    }
}
