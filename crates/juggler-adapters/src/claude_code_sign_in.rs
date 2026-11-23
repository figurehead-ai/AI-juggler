//! Claude Code's sign-in (split from `claude_code.rs` by feature): the
//! page and code its sign-in shows, the managed folder it signs into, the
//! import and adoption of an existing folder, and whose sign-in a folder
//! holds. Credentials are never read (ADR-0003).

use std::path::Path;

use juggler_adapter::environment::{ImportOutcome, LoginProbe, LoginState, LoginUrl};
use juggler_adapter::prelude::*;
use juggler_adapter::{AdapterError, EnvMap};
use juggler_core::model::Account;

use super::{
    CLAUDE_CODE_ID, ClaudeCodeAdapter, ENV_CONFIG_DIR, claude_state_file, prepare_managed_dir,
    probe_claude_identity,
};
use crate::claude_environment;

/// AJ-E1545 R8: how Claude Code's sign-in output is read: its own page (claude.ai), with a code to paste back when the browser can't call it back.
const SIGN_IN_PAGE: juggler_adapter::PageRule = juggler_adapter::PageRule {
    hosts: &["claude.ai", "claude.com", "anthropic.com"],
    device_codes: false,
    paste_words: &["paste code", "paste the code", "paste your code"],
};

impl ToolSignIn for ClaudeCodeAdapter {
    /// AJ-E1545 R8: the page and code Claude Code's sign-in shows.
    fn sign_in_page(&self, output: &str) -> Option<juggler_adapter::SignInPage> {
        juggler_adapter::sign_in_page::read(output, &SIGN_IN_PAGE)
    }
    /// AJ-E1553: its sign-in file's size and time (never its contents).
    fn sign_in_stamp(&self, state_dir: &Path) -> Option<juggler_adapter::SignInStamp> {
        crate::sign_in::stamp(CLAUDE_CODE_ID, state_dir)
    }
    fn prepare_account(&self, state_dir: &Path) -> Result<EnvMap, AdapterError> {
        prepare_managed_dir(state_dir)?;
        let mut env = EnvMap::new();
        env.insert(
            ENV_CONFIG_DIR.to_string(),
            state_dir.to_string_lossy().into_owned(),
        );
        Ok(env)
    }
    fn import_account(&self, source: &Path, dest: &Path) -> Result<ImportOutcome, AdapterError> {
        let probe = signed_in_probe();
        claude_environment::import_claude_config(source, dest, &probe)
    }
    fn can_use_in_place(&self) -> bool {
        true
    }
    fn adopt_account(&self, source: &Path) -> Result<ImportOutcome, AdapterError> {
        let adapter = ClaudeCodeAdapter::new();
        let probe = move |path: &Path| match adapter.login_state(path) {
            LoginState::SignedIn => LoginProbe::LoggedIn,
            _ => LoginProbe::LoggedOut,
        };
        claude_environment::adopt_claude_config(source, &probe)
    }
    fn inspect_source(
        &self,
        source: &Path,
    ) -> Result<juggler_adapter::environment::SourceInspect, AdapterError> {
        let probe = signed_in_probe();
        claude_environment::inspect_source(source, &probe)
    }
    fn import_account_degraded(
        &self,
        source: &Path,
        dest: &Path,
    ) -> Result<ImportOutcome, AdapterError> {
        let probe = signed_in_probe();
        claude_environment::import_settings_only(source, dest, &probe)
    }
    fn login_url(&self) -> Option<LoginUrl> {
        Some("https://claude.ai/login")
    }
    fn sign_in_vendor(&self) -> String {
        "anthropic".to_owned()
    }
    /// AJ-E1001: reads ONLY the `oauthAccount` object of the state
    /// dir's `.claude.json` — the account id, email and organisation
    /// name Claude Code records there. Credentials live elsewhere (the
    /// Keychain or `.credentials.json`) and are never opened.
    fn probe_identity(&self, account: &Account) -> juggler_adapter::IdentityProbe {
        probe_claude_identity(&account.state_dir)
    }
    /// AJ-E3210 (D-63 as amended): the Claude app's own sign-in in an
    /// instance, by its records' folder names only.
    fn desktop_sign_in(&self, instance_dir: &Path) -> Option<juggler_adapter::SignInNames> {
        crate::claude_sign_ins::desktop_sign_in(instance_dir)
    }
    /// AJ-E3210 (Q2): the kind of the newest run in the folder.
    fn sign_in_writer(&self, state_dir: &Path) -> juggler_adapter::SignInWriter {
        crate::claude_sign_ins::writer(state_dir)
    }
    fn login_state(&self, state_dir: &Path) -> LoginState {
        ClaudeCodeAdapter::login_state(self, state_dir)
    }
    fn complete_first_run(&self, state_dir: &Path) -> Result<(), AdapterError> {
        crate::claude_ready::complete_first_run_in(&claude_state_file(state_dir))
    }
}

/// A folder's sign-in as a scan sees it: binary (the inference's
/// signed-in/not vocabulary); the account-state wire carries the full
/// taxonomy (AJ-E308).
fn signed_in_probe() -> impl Fn(&Path) -> LoginProbe {
    let adapter = ClaudeCodeAdapter::new();
    move |path: &Path| match adapter.login_state(path) {
        LoginState::SignedIn => LoginProbe::LoggedIn,
        LoginState::Partial
        | LoginState::Expired
        | LoginState::SsoPending
        | LoginState::ApiKey
        | LoginState::LoggedOut
        | LoginState::Unknown => LoginProbe::LoggedOut,
    }
}
