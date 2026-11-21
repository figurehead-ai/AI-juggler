//! The guardrail that keeps Hermes Agent away from the Codex and Claude
//! homes AI Juggler manages (AJ-E1607 Req.3).
//!
//! What Hermes does with them unless told not to, as its source says
//! (`NousResearch/hermes-agent@422ad186`; research `hermes.md` §3):
//!
//! - It **borrows and refreshes** the Codex CLI's sign-in
//!   (`$CODEX_HOME/auth.json`, else `~/.codex/auth.json`) and Claude
//!   Code's (`$CLAUDE_CONFIG_DIR/.credentials.json`, else
//!   `~/.claude/.credentials.json`, and the login Keychain item
//!   `Claude Code-credentials`), writing a refreshed token back, whenever
//!   it has no login of its own: `auth.adopt_external_logins`, true by
//!   default (`hermes_cli/config_defaults.py`;
//!   `agent/anthropic_credentials.py`, `read_claude_code_credentials`,
//!   `_write_claude_code_credentials`; `hermes_cli/auth_codex.py`). Both
//!   tools' refresh tokens are single-use, so one program's refresh signs
//!   the other out. With the setting false, Hermes "never reads or
//!   refreshes those files" (`agent/credential_sources.py`,
//!   `adopt_external_logins_enabled`), the Keychain item included.
//! - Its Codex runtime writes MCP servers into `$CODEX_HOME/config.toml`
//!   (`hermes_cli/codex_runtime_plugin_migration.py`), and its session
//!   import reads `$CODEX_HOME/sessions` and `$CLAUDE_CONFIG_DIR/projects`
//!   (`hermes_cli/foreign_sessions.py`).
//!
//! So every run AI Juggler starts carries, through Hermes' own
//! administrator layer (`HERMES_MANAGED_DIR`, a launch-only variable no
//! `.env` can move; `hermes_cli/managed_scope.py`, whose values win over
//! the user's own config and `.env`, `hermes_cli/config.py`
//! `_merge_managed_overlay`, `hermes_cli/env_loader.py`):
//!
//! - a `config.yaml` setting `auth.adopt_external_logins: false`;
//! - a `.env` pointing `CODEX_HOME` and `CLAUDE_CONFIG_DIR` at empty
//!   folders of the account's own, so nothing of a Codex or Claude home is
//!   read, imported or edited, whatever the user's own `.env` says.
//!
//! The same two variables are set on the run itself, and inherited by a
//! gateway its client starts (`HERMES_MANAGED_DIR` included: Hermes strips
//! only supervisor markers from a daemon's environment). A Mac with an
//! administrator's own Hermes policy (`/etc/hermes`) is never overridden:
//! AI Juggler won't run Hermes there ([`refusal`]).

use std::path::Path;

use juggler_adapter::AdapterError;

/// AI Juggler's folder inside an account's Hermes home.
pub const OWN_DIR: &str = ".ai-juggler";
/// The policy folder `HERMES_MANAGED_DIR` names.
pub const POLICY_DIR: &str = ".ai-juggler/hermes-policy";
/// The empty folder `CODEX_HOME` names.
pub const NO_CODEX: &str = ".ai-juggler/no-codex";
/// The empty folder `CLAUDE_CONFIG_DIR` names.
pub const NO_CLAUDE: &str = ".ai-juggler/no-claude";
/// The account's own gateway lock folder (`HERMES_GATEWAY_LOCK_DIR`).
pub const LOCKS_DIR: &str = ".ai-juggler/gateway-locks";
/// Where an administrator's Hermes policy lives (`get_managed_dir`'s
/// default on macOS and Linux).
pub const SYSTEM_POLICY: &str = "/etc/hermes";

/// The policy's `config.yaml`: Hermes uses only its own logins.
pub const POLICY_CONFIG: &str = "# Written by AI Juggler: Hermes uses only its own sign-ins here, never\n\
# the Codex CLI's or Claude Code's (AJ-E1607).\n\
auth:\n  adopt_external_logins: false\n";

/// Why no Hermes run starts on a Mac with an administrator's policy.
pub const ADMIN_POLICY: &str = "this computer has an administrator's Hermes policy (/etc/hermes); AI Juggler doesn't \
     replace it, and without its own it can't keep Hermes away from your Codex and Claude sign-ins";

/// The policy's `.env` for the account folder `state_dir`.
///
/// # Errors
///
/// A folder whose path can't be written as a literal `.env` value (a
/// quote, a `$` or a line break in it).
pub fn policy_env(state_dir: &Path) -> Result<String, AdapterError> {
    let literal = |relative: &str| -> Result<String, AdapterError> {
        let path = state_dir.join(relative).to_string_lossy().into_owned();
        if path.contains(['\'', '$', '\n', '\r']) {
            return Err(AdapterError::AccountDir {
                path: state_dir.to_path_buf(),
                message: "Hermes Agent can't be kept apart in a folder whose path holds a quote, \
                          a $ or a line break"
                    .to_owned(),
            });
        }
        Ok(format!("'{path}'"))
    };
    Ok(format!(
        "# Written by AI Juggler (AJ-E1607): Hermes never reads a Codex or Claude home.\n\
         CODEX_HOME={}\nCLAUDE_CONFIG_DIR={}\n",
        literal(NO_CODEX)?,
        literal(NO_CLAUDE)?
    ))
}

/// The guard's folders and policy for the account folder `state_dir`,
/// made owner-only and written again whenever they differ from what AI
/// Juggler wrote. Nothing else in the folder is touched.
///
/// # Errors
///
/// A folder or file that can't be made.
pub fn ensure(state_dir: &Path) -> Result<(), AdapterError> {
    let env = policy_env(state_dir)?;
    crate::common::home::make_private_dirs([
        state_dir.join(POLICY_DIR),
        state_dir.join(NO_CODEX),
        state_dir.join(NO_CLAUDE),
        state_dir.join(LOCKS_DIR),
    ])?;
    let policy = state_dir.join(POLICY_DIR);
    for (name, text) in [("config.yaml", POLICY_CONFIG), (".env", env.as_str())] {
        let path = policy.join(name);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text) {
            std::fs::write(&path, text)
                .and_then(|()| crate::common::home::restrict_to_owner(&path))
                .map_err(|error| AdapterError::AccountDir {
                    path: path.clone(),
                    message: format!("the Hermes guard couldn't be written: {error}"),
                })?;
        }
    }
    Ok(())
}

/// Why a run can't start here, when an administrator's policy is in
/// place at `system_policy` ([`SYSTEM_POLICY`] on a real Mac).
#[must_use]
pub fn refusal(system_policy: &Path) -> Option<AdapterError> {
    system_policy
        .is_dir()
        .then(|| AdapterError::Unsupported(ADMIN_POLICY.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_policy_names_the_accounts_own_empty_folders_as_literals() {
        let env = policy_env(Path::new(
            "/Users/me/Library/Application Support/AI Juggler/a",
        ))
        .unwrap();
        assert!(env.contains(
            "CODEX_HOME='/Users/me/Library/Application Support/AI Juggler/a/.ai-juggler/no-codex'"
        ));
        assert!(env.contains("CLAUDE_CONFIG_DIR='"));
        assert!(policy_env(Path::new("/odd/it's")).is_err());
        assert!(policy_env(Path::new("/odd/$HOME")).is_err());
    }

    #[test]
    fn the_guard_is_written_once_and_put_back_when_changed() {
        let dir = tempfile::tempdir().unwrap();
        ensure(dir.path()).unwrap();
        let config = dir.path().join(POLICY_DIR).join("config.yaml");
        assert_eq!(std::fs::read_to_string(&config).unwrap(), POLICY_CONFIG);
        assert!(dir.path().join(NO_CODEX).is_dir() && dir.path().join(NO_CLAUDE).is_dir());
        std::fs::write(&config, "auth:\n  adopt_external_logins: true\n").unwrap();
        ensure(dir.path()).unwrap();
        assert_eq!(std::fs::read_to_string(&config).unwrap(), POLICY_CONFIG);
    }

    #[test]
    fn an_administrators_policy_is_never_overridden() {
        let dir = tempfile::tempdir().unwrap();
        assert!(refusal(&dir.path().join("etc-hermes")).is_none());
        std::fs::create_dir(dir.path().join("etc-hermes")).unwrap();
        assert!(refusal(&dir.path().join("etc-hermes")).is_some());
    }
}
