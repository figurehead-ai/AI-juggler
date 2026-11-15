//! AJ-E3013 (security review M1, round 2): whether a **running** process's
//! code satisfies a code-signing requirement.
//!
//! The kernel's own code-signing host is asked for the guest with that pid
//! (`SecCodeCopyGuestWithAttributes`), and its dynamic validity is checked
//! against an explicit requirement (`SecCodeCheckValidity`): the code
//! actually running, never a file at a path (a genuine program renamed over
//! the path of a running one changes nothing). Through the
//! `security-framework` crate's safe API (already the workspace's, for the
//! Keychain): no `unsafe` here.
//!
//! A verdict is kept per (pid, pid version, requirement), never per path:
//! the caller reads the pid's version before the check and again after it,
//! so a pid reused meanwhile is never taken for the one checked.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

type Key = (u32, u32, String);

fn verdicts() -> &'static Mutex<HashMap<Key, bool>> {
    static VERDICTS: OnceLock<Mutex<HashMap<Key, bool>>> = OnceLock::new();
    VERDICTS.get_or_init(Mutex::default)
}

/// The most verdicts kept (pids come and go).
const KEPT_MAX: usize = 1_024;

/// Whether the code running as `pid` (whose pid version the caller read as
/// `pid_version`) is valid and satisfies `requirement` (a code-signing
/// requirement, `identifier "…" and anchor apple generic and …`). `false`
/// for anything else, a requirement that doesn't parse, a process that's
/// gone, and where the system has no code signing (not macOS).
#[must_use]
pub fn running_satisfies(pid: u32, pid_version: u32, requirement: &str) -> bool {
    let key = (pid, pid_version, requirement.to_owned());
    if let Some(known) = verdicts()
        .lock()
        .ok()
        .and_then(|verdicts| verdicts.get(&key).copied())
    {
        return known;
    }
    let verdict = check(pid, requirement);
    if let Ok(mut verdicts) = verdicts().lock() {
        if verdicts.len() >= KEPT_MAX {
            verdicts.clear();
        }
        verdicts.insert(key, verdict);
    }
    verdict
}

#[cfg(target_os = "macos")]
fn check(pid: u32, requirement: &str) -> bool {
    use security_framework::os::macos::code_signing::{
        Flags, GuestAttributes, SecCode, SecRequirement,
    };
    let (Ok(requirement), Ok(pid)) = (requirement.parse::<SecRequirement>(), libc_pid(pid)) else {
        return false;
    };
    let mut attributes = GuestAttributes::new();
    attributes.set_pid(pid);
    SecCode::copy_guest_with_attribues(None, &attributes, Flags::NONE)
        .and_then(|code| code.check_validity(Flags::NONE, &requirement))
        .is_ok()
}

#[cfg(target_os = "macos")]
fn libc_pid(pid: u32) -> Result<i32, std::num::TryFromIntError> {
    i32::try_from(pid)
}

#[cfg(not(target_os = "macos"))]
fn check(_pid: u32, _requirement: &str) -> bool {
    false
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    /// Not a test: the program the probes run (this test binary, copied
    /// and signed again), waiting to be checked.
    #[test]
    #[ignore = "run by the probes as their program"]
    fn sleeper() {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    /// A running copy, killed on drop.
    struct Running(std::process::Child);

    impl Drop for Running {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// This test program copied to `at`, signed ad hoc as `identifier`.
    fn program(at: &Path, identifier: &str) -> PathBuf {
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), at).unwrap();
        let signed = Command::new("/usr/bin/codesign")
            .args(["-s", "-", "-f", "-i", identifier])
            .arg(at)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(signed.success(), "signed {identifier}");
        at.to_path_buf()
    }

    fn run(at: &Path) -> Running {
        let child = Command::new(at)
            .args(["code_sign::tests::sleeper", "--exact", "--ignored", "-q"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        Running(child)
    }

    const GENUINE: &str = "identifier \"ai.juggler.test.genuine\"";

    #[test]
    fn the_running_code_is_checked_against_the_requirement() {
        let root = tempfile::tempdir().unwrap();
        let genuine = run(&program(
            &root.path().join("genuine"),
            "ai.juggler.test.genuine",
        ));
        assert!(
            running_satisfies(genuine.0.id(), 1, GENUINE),
            "its own identity"
        );
        assert!(!running_satisfies(
            genuine.0.id(),
            1,
            "identifier \"com.anthropic.claude-code\" and anchor apple generic"
        ));
        assert!(!running_satisfies(genuine.0.id(), 1, "not a requirement"));
        assert!(!running_satisfies(u32::MAX, 1, GENUINE), "no such process");
    }

    /// The review's probe (b): an ad-hoc program runs at a path, then a
    /// program that does satisfy the requirement is renamed over that path.
    /// The path would pass (the file there is genuine); the running code
    /// doesn't.
    #[test]
    fn a_genuine_program_renamed_over_a_running_one_changes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let path = program(&root.path().join("tool"), "ai.juggler.test.impostor");
        let running = run(&path);
        let genuine = program(&root.path().join("genuine"), "ai.juggler.test.genuine");
        std::fs::rename(&genuine, &path).unwrap();
        let on_disk = Command::new("/usr/bin/codesign")
            .args(["--verify", &format!("-R={GENUINE}")])
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(on_disk.success(), "the file at the path is genuine now");
        assert!(
            !running_satisfies(running.0.id(), 2, GENUINE),
            "the running code isn't"
        );
    }

    /// The review's probe (a): a program at `…/x/Claude.app/Contents/MacOS/
    /// Claude` is no host by its name: the Claude app's own requirement is
    /// met only by the Claude app's code.
    #[test]
    fn a_program_named_like_the_host_isnt_the_host() {
        let root = tempfile::tempdir().unwrap();
        let fake = program(
            &root.path().join("x/Claude.app/Contents/MacOS/Claude"),
            "com.anthropic.claudefordesktop",
        );
        let running = run(&fake);
        let host = "identifier \"com.anthropic.claudefordesktop\" and anchor apple generic \
                    and certificate leaf[subject.OU] = \"Q6L2SF6YDW\"";
        assert!(
            !running_satisfies(running.0.id(), 3, host),
            "even signed with the host's own identifier, not by its team"
        );
    }

    /// A manual check on a Mac running the Claude app (not run by default):
    /// its main process and a Code-tab Claude Code meet the requirements
    /// the attestation pins.
    #[test]
    #[ignore = "needs the Claude app and a Code-tab session running"]
    fn the_installed_claude_app_and_its_claude_code_pass() {
        let pids = |name: &str| -> Vec<u32> {
            let out = Command::new("/usr/bin/pgrep")
                .args(["-x", name])
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect()
        };
        let host = "identifier \"com.anthropic.claudefordesktop\" and anchor apple generic \
                    and certificate leaf[subject.OU] = \"Q6L2SF6YDW\"";
        let tool = "identifier \"com.anthropic.claude-code\" and anchor apple generic \
                    and certificate leaf[subject.OU] = \"Q6L2SF6YDW\"";
        assert!(
            pids("Claude")
                .iter()
                .any(|pid| running_satisfies(*pid, 9, host))
        );
        // Low 2: the helper hop is the app's `disclaimer` only; Claude
        // Code itself, signed by the same team, isn't one.
        let helper = "identifier \"disclaimer\" and anchor apple generic \
                      and certificate leaf[subject.OU] = \"Q6L2SF6YDW\"";
        assert!(
            pids("disclaimer")
                .iter()
                .any(|pid| running_satisfies(*pid, 9, helper))
        );
        assert!(
            !pids("claude")
                .iter()
                .any(|pid| running_satisfies(*pid, 9, helper))
        );
        assert!(
            pids("claude")
                .iter()
                .any(|pid| running_satisfies(*pid, 9, tool))
        );
    }
}
