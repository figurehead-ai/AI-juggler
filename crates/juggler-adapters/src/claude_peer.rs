//! AJ-E3013 (D-116, ADR-0020's D-116 addendum): whether a keyless caller
//! is one of a Claude app profile's Code-tab sessions.
//!
//! The caller's chain comes from the kernel (the app lists it). For the
//! caller's parent, then its grandparent (a hook runs under `sh`):
//!
//! 1. Claude Code's registry entry `<config>/sessions/<pid>.json` (a
//!    regular file of the folder's owner, never a link, at most 16 KiB,
//!    its `pid` equal to its name). Only `pid`, `sessionId`,
//!    `hostSessionId`, `entrypoint` and `procStart` are read; the
//!    `<pid>.<hash>.key` files beside it are never opened.
//! 2. It must say `entrypoint: "claude-desktop"`, and its `procStart` must
//!    be that process's real start time (a second at most apart): a stale
//!    file of a reused pid fails, and names its conversation as stale.
//! 3. Its `hostSessionId` (`local_<id>`) must name the app's own record
//!    `<instance>/claude-code-sessions/<account>/<org>/<id>.json` (a
//!    regular file, never a link), whose `cliSessionId` is the registry's
//!    `sessionId`.
//!
//! Anything short of that is refused with the reason. Nothing here
//! trusts the caller's environment or words.

use std::path::{Path, PathBuf};

use juggler_adapter::live::ctime_utc_secs;
use juggler_adapter::peer::{
    AttestedSession, DesktopPeer, DesktopPlace, MAX_HOPS, PeerVerdict, ProcStep, same_start_strict,
};
use juggler_adapter::safe_fs::{Expect, read_in};

/// The largest registry entry read (they're under 1 KiB).
const ENTRY_MAX: u64 = 16 * 1024;
/// The largest app record read (they're tens of KiB).
const RECORD_MAX: u64 = 2 * 1024 * 1024;
/// The entrypoint the Claude app's Code tab records.
pub const DESKTOP_ENTRYPOINT: &str = "claude-desktop";
/// The longest id taken from a record.
const ID_MAX: usize = 128;

/// A registry entry: the fields attestation reads, nothing else kept.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    pid: Option<u64>,
    session_id: Option<String>,
    host_session_id: Option<String>,
    entrypoint: Option<String>,
    proc_start: Option<String>,
}

/// The app's record: its conversation id, nothing else kept.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    cli_session_id: Option<String>,
}

/// The Claude app's half of the attestation.
pub struct ClaudeAppPeer;

impl DesktopPeer for ClaudeAppPeer {
    fn attest(&self, place: &DesktopPlace<'_>, chain: &[ProcStep]) -> PeerVerdict {
        attest(place, chain)
    }

    fn can_wake(&self) -> bool {
        // D-119: a Code-tab session has its own inbox socket.
        true
    }

    fn still_current(&self, place: &DesktopPlace<'_>, session: &AttestedSession) -> bool {
        still_current(place, session)
    }
}

/// An id as the registry and the app write them: letters, digits, `-`
/// and `_`, not empty, not long. Nothing else is ever joined to a path.
fn plain_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= ID_MAX
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A record attestation may read (L5: the shared walk): `rel` under
/// `root`, never through a link, a regular file of `root`'s owner no one
/// else can write, at most `max` bytes, all checked on the opened file.
/// `Ok(None)`: not there.
fn record(root: &Path, rel: &Path, max: u64) -> Result<Option<Vec<u8>>, ()> {
    read_in(root, rel, Expect::owned(max)).map_err(|_| ())
}

/// The verdict for `chain` (the caller first) in the App at `place`.
#[must_use]
pub fn attest(place: &DesktopPlace<'_>, chain: &[ProcStep]) -> PeerVerdict {
    if chain.is_empty() {
        return PeerVerdict::no("the caller's process isn't known");
    }
    for step in chain.iter().skip(1).take(MAX_HOPS) {
        let rel = PathBuf::from("sessions").join(format!("{}.json", step.pid));
        match record(place.config_dir, &rel, ENTRY_MAX) {
            Ok(None) => {} // not a Claude Code process of this profile
            Ok(Some(bytes)) => return from_entry(place, &bytes, *step),
            Err(()) => {
                return PeerVerdict::no(
                    "Claude Code's session record isn't a plain file of this profile's",
                );
            }
        }
    }
    PeerVerdict::no("the caller wasn't started by a Code-tab session of this Claude app profile")
}

/// The verdict from the registry entry `bytes`, for the process `step`.
fn from_entry(place: &DesktopPlace<'_>, bytes: &[u8], step: ProcStep) -> PeerVerdict {
    let Ok(entry) = serde_json::from_slice::<Entry>(bytes) else {
        return PeerVerdict::no("Claude Code's session record couldn't be read");
    };
    if entry.pid != Some(u64::from(step.pid)) {
        return PeerVerdict::no("Claude Code's session record names another process");
    }
    let Some(conversation) = entry.session_id.filter(|id| plain_id(id)) else {
        return PeerVerdict::no("Claude Code's session record has no session id");
    };
    let started = entry.proc_start.as_deref().and_then(ctime_utc_secs);
    if !same_start_strict(started, step.start_secs) {
        // A pid reused since, or a record of another shape: never trusted.
        return PeerVerdict::Unattested {
            reason: "Claude Code's session record doesn't match its process (it ended, or the pid was reused)".to_owned(),
            stale: Some(conversation),
        };
    }
    if entry.entrypoint.as_deref() != Some(DESKTOP_ENTRYPOINT) {
        return PeerVerdict::no("the session isn't one of the Claude app's Code tab");
    }
    let Some(host) = entry
        .host_session_id
        .filter(|id| id.starts_with("local_") && plain_id(id))
    else {
        return PeerVerdict::no("Claude Code's session record names no app session");
    };
    match app_record(place.instance_dir, &host) {
        AppRecord::Of(record) if record == conversation => PeerVerdict::Attested(AttestedSession {
            conversation,
            host_session: host,
            tool_pid: step.pid,
            tool_start: step.start_secs,
        }),
        AppRecord::Of(_) | AppRecord::Unreadable => PeerVerdict::no(
            "the Claude app's record of the session names another conversation, or can't be read",
        ),
        AppRecord::Missing => {
            PeerVerdict::no("this Claude app profile has no record of the session")
        }
    }
}

/// H3: whether the registry still names `session` for its tool process:
/// the entry there, for its pid and conversation, with its start time.
#[must_use]
pub fn still_current(place: &DesktopPlace<'_>, session: &AttestedSession) -> bool {
    let rel = PathBuf::from("sessions").join(format!("{}.json", session.tool_pid));
    let Ok(Some(bytes)) = record(place.config_dir, &rel, ENTRY_MAX) else {
        return false;
    };
    serde_json::from_slice::<Entry>(&bytes).is_ok_and(|entry| {
        entry.pid == Some(u64::from(session.tool_pid))
            && entry.session_id.as_deref() == Some(session.conversation.as_str())
            && same_start_strict(
                entry.proc_start.as_deref().and_then(ctime_utc_secs),
                session.tool_start,
            )
    })
}

/// What the app keeps for one of its sessions.
enum AppRecord {
    /// Its conversation.
    Of(String),
    /// A record that isn't a plain file, or doesn't read.
    Unreadable,
    /// None.
    Missing,
}

/// The app's record of `host`, searched in its two levels of folders
/// only.
fn app_record(instance_dir: &Path, host: &str) -> AppRecord {
    let root = crate::claude_live::app_root(instance_dir);
    let folders = |dir: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default()
    };
    for account in folders(&root) {
        for org in folders(&account) {
            let Ok(rel) = org
                .join(format!("{host}.json"))
                .strip_prefix(instance_dir)
                .map(Path::to_path_buf)
            else {
                continue;
            };
            return match record(instance_dir, &rel, RECORD_MAX) {
                Ok(None) => continue,
                Err(()) => AppRecord::Unreadable,
                Ok(Some(bytes)) => serde_json::from_slice::<Record>(&bytes)
                    .ok()
                    .and_then(|record| record.cli_session_id)
                    .map_or(AppRecord::Unreadable, AppRecord::Of),
            };
        }
    }
    AppRecord::Missing
}

#[cfg(test)]
mod tests {
    use super::*;

    const START_TEXT: &str = "Mon Oct 26 07:33:20 2026";
    const START: i64 = 1_793_000_000; // START_TEXT, checked below

    struct Lab {
        _root: tempfile::TempDir,
        config: PathBuf,
        instance: PathBuf,
    }

    fn lab() -> Lab {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        let instance = root.path().join("instance");
        std::fs::create_dir_all(config.join("sessions")).unwrap();
        std::fs::create_dir_all(instance.join("claude-code-sessions/acct/org")).unwrap();
        Lab {
            _root: root,
            config,
            instance,
        }
    }

    fn entry(lab: &Lab, pid: u32, body: &str) {
        std::fs::write(lab.config.join(format!("sessions/{pid}.json")), body).unwrap();
    }

    fn desktop_entry(lab: &Lab, pid: u32, start: &str) {
        entry(
            lab,
            pid,
            &format!(
                r#"{{"pid":{pid},"sessionId":"conv-1","cwd":"/w","status":"idle","procStart":"{start}","hostSessionId":"local_abc-1","entrypoint":"claude-desktop","kind":"interactive","messagingSocketPath":"/tmp/x"}}"#
            ),
        );
    }

    fn record(lab: &Lab, conversation: &str) {
        std::fs::write(
            lab.instance
                .join("claude-code-sessions/acct/org/local_abc-1.json"),
            format!(r#"{{"sessionId":"local_abc-1","cliSessionId":"{conversation}","title":"t"}}"#),
        )
        .unwrap();
    }

    fn place(lab: &Lab) -> DesktopPlace<'_> {
        DesktopPlace {
            config_dir: &lab.config,
            instance_dir: &lab.instance,
        }
    }

    fn step(pid: u32, start_secs: i64) -> ProcStep {
        ProcStep { pid, start_secs }
    }

    #[test]
    fn the_start_text_is_the_start() {
        assert_eq!(ctime_utc_secs(START_TEXT), Some(START));
    }

    #[test]
    fn the_mcp_server_of_a_code_tab_session_is_attested() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        record(&lab, "conv-1");
        let verdict = attest(&place(&lab), &[step(5000, START + 5), step(4242, START)]);
        assert_eq!(
            verdict,
            PeerVerdict::Attested(AttestedSession {
                conversation: "conv-1".into(),
                host_session: "local_abc-1".into(),
                tool_pid: 4242,
                tool_start: START,
            })
        );
    }

    #[test]
    fn a_hook_under_sh_is_attested_and_three_steps_up_is_not() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        record(&lab, "conv-1");
        let hook = [
            step(5001, START + 9),
            step(5000, START + 8),
            step(4242, START),
        ];
        assert!(matches!(
            attest(&place(&lab), &hook),
            PeerVerdict::Attested(_)
        ));
        let far = [
            step(5002, START + 9),
            step(5001, START + 9),
            step(5000, START + 8),
            step(4242, START),
        ];
        assert!(matches!(
            attest(&place(&lab), &far),
            PeerVerdict::Unattested { .. }
        ));
    }

    #[test]
    fn a_process_outside_the_session_is_refused() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        record(&lab, "conv-1");
        // Its parent is the user's shell, not the session.
        let outsider = [step(7000, START + 60), step(6000, START - 600), step(1, 0)];
        assert!(matches!(
            attest(&place(&lab), &outsider),
            PeerVerdict::Unattested { .. }
        ));
        // The session's own pid as the caller itself isn't a descendant.
        assert!(matches!(
            attest(&place(&lab), &[step(4242, START)]),
            PeerVerdict::Unattested { .. }
        ));
    }

    #[test]
    fn a_reused_pid_is_refused_and_named_stale() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        record(&lab, "conv-1");
        // Pid 4242 now runs something that started an hour later.
        let verdict = attest(
            &place(&lab),
            &[step(5000, START + 3700), step(4242, START + 3600)],
        );
        assert_eq!(
            verdict,
            PeerVerdict::Unattested {
                reason: "Claude Code's session record doesn't match its process (it ended, or the pid was reused)".into(),
                stale: Some("conv-1".into()),
            }
        );
        // No procStart at all is never a match.
        entry(
            &lab,
            4242,
            r#"{"pid":4242,"sessionId":"conv-1","hostSessionId":"local_abc-1","entrypoint":"claude-desktop"}"#,
        );
        assert!(matches!(
            attest(&place(&lab), &[step(5000, START + 5), step(4242, START)]),
            PeerVerdict::Unattested { .. }
        ));
    }

    #[test]
    fn the_registry_and_the_app_record_must_agree() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        let chain = [step(5000, START + 5), step(4242, START)];
        assert!(
            matches!(attest(&place(&lab), &chain), PeerVerdict::Unattested { .. }),
            "no app record"
        );
        record(&lab, "conv-other");
        assert!(
            matches!(attest(&place(&lab), &chain), PeerVerdict::Unattested { .. }),
            "a record of another conversation (after /clear the registry is read again)"
        );
        record(&lab, "conv-1");
        assert!(matches!(
            attest(&place(&lab), &chain),
            PeerVerdict::Attested(_)
        ));
        // A terminal session of the same profile isn't the app's.
        entry(
            &lab,
            4242,
            &format!(
                r#"{{"pid":4242,"sessionId":"conv-1","procStart":"{START_TEXT}","hostSessionId":"local_abc-1","entrypoint":"cli"}}"#
            ),
        );
        assert!(matches!(
            attest(&place(&lab), &chain),
            PeerVerdict::Unattested { .. }
        ));
    }

    #[test]
    fn ids_that_could_name_another_path_are_refused() {
        let lab = lab();
        entry(
            &lab,
            4242,
            &format!(
                r#"{{"pid":4242,"sessionId":"conv-1","procStart":"{START_TEXT}","hostSessionId":"local_../../x","entrypoint":"claude-desktop"}}"#
            ),
        );
        let chain = [step(5000, START + 5), step(4242, START)];
        assert!(matches!(
            attest(&place(&lab), &chain),
            PeerVerdict::Unattested { .. }
        ));
        entry(
            &lab,
            4242,
            r#"{"pid":4243,"sessionId":"conv-1","procStart":"x","hostSessionId":"local_abc-1","entrypoint":"claude-desktop"}"#,
        );
        assert!(
            matches!(attest(&place(&lab), &chain), PeerVerdict::Unattested { .. }),
            "a record naming another pid"
        );
    }

    #[cfg(unix)]
    #[test]
    fn links_are_never_followed() {
        let lab = lab();
        let elsewhere = lab.config.join("elsewhere.json");
        std::fs::write(
            &elsewhere,
            format!(
                r#"{{"pid":4242,"sessionId":"conv-1","procStart":"{START_TEXT}","hostSessionId":"local_abc-1","entrypoint":"claude-desktop"}}"#
            ),
        )
        .unwrap();
        std::os::unix::fs::symlink(&elsewhere, lab.config.join("sessions/4242.json")).unwrap();
        record(&lab, "conv-1");
        let chain = [step(5000, START + 5), step(4242, START)];
        assert!(matches!(
            attest(&place(&lab), &chain),
            PeerVerdict::Unattested { .. }
        ));
    }

    #[test]
    fn the_key_files_are_never_read() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        record(&lab, "conv-1");
        // A key file beside the entry that isn't readable at all: had it
        // been opened, the read would fail; the verdict doesn't change.
        let key = lab.config.join("sessions/4242.abcdef.key");
        std::fs::write(&key, "secret").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        let chain = [step(5000, START + 5), step(4242, START)];
        assert!(matches!(
            attest(&place(&lab), &chain),
            PeerVerdict::Attested(_)
        ));
        let source = include_str!("claude_peer.rs");
        let code = source.split("#[cfg(test)]").next().unwrap();
        assert!(
            !code.contains(".key\""),
            "the attestation names no key file"
        );
    }

    #[test]
    fn a_kept_session_stays_current_only_while_its_registry_entry_does() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        record(&lab, "conv-1");
        let PeerVerdict::Attested(session) =
            attest(&place(&lab), &[step(5000, START + 5), step(4242, START)])
        else {
            unreachable!("attested");
        };
        assert!(still_current(&place(&lab), &session));
        // procStart changed: the pid now runs something else.
        desktop_entry(&lab, 4242, "Mon Oct 26 08:33:20 2026");
        assert!(!still_current(&place(&lab), &session));
        // The registry entry gone: the session ended.
        std::fs::remove_file(lab.config.join("sessions/4242.json")).unwrap();
        assert!(!still_current(&place(&lab), &session));
    }
}
