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
/// M1: Anthropic's Apple developer team, which signs the Claude app and
/// the Claude Code it ships (`codesign -dv`, 2026-11-15: the CLI
/// `Identifier=com.anthropic.claude-code`, the app
/// `Identifier=com.anthropic.claudefordesktop`, its helpers, all
/// `TeamIdentifier=Q6L2SF6YDW`).
pub const ANTHROPIC_TEAM: &str = "Q6L2SF6YDW";
/// M1: the Claude app's bundle identifier, read once from the installed
/// app's `Info.plist` (`CFBundleIdentifier`, 2026-11-15) and pinned.
pub const HOST_BUNDLE_ID: &str = "com.anthropic.claudefordesktop";
/// M1: what the running Claude Code must be.
pub const TOOL_REQUIREMENT: &str = "identifier \"com.anthropic.claude-code\" and anchor apple generic and certificate leaf[subject.OU] = \"Q6L2SF6YDW\"";
/// M1: what the Claude app's own running main process must be.
pub const HOST_REQUIREMENT: &str = "identifier \"com.anthropic.claudefordesktop\" and anchor apple generic and certificate leaf[subject.OU] = \"Q6L2SF6YDW\"";
/// M1 (round 3, Low 2): what the one helper between the Claude app and its
/// Claude Code must be: its `disclaimer` launcher (`codesign -dv`,
/// 2026-11-15: `Identifier=disclaimer`, Anthropic's team), pinned by its
/// identifier, so no other Anthropic-signed program (Claude Code itself,
/// `app-cu-helper`) stands between them.
pub const HELPER_REQUIREMENT: &str = "identifier \"disclaimer\" and anchor apple generic and certificate leaf[subject.OU] = \"Q6L2SF6YDW\"";
/// The third review's Low 1: the lock the Claude app's main (browser)
/// process holds on its instance while it runs it: Chromium's own storage
/// database lock (`fcntl`, checked 2026-11-15: held by the running main
/// process of each of five instances). The app takes no `SingletonLock`
/// (none of those instances has one), so this is the proof.
pub const INSTANCE_LOCK: &str = "Local Storage/leveldb/LOCK";
/// M1: where a Claude app profile keeps its Claude Code, under its own
/// folder: `claude-code/<version>/<build>/…/Contents/MacOS/claude`.
const SHIPPED: &str = "claude-code";

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

/// The app's record: its conversation id and whether the app archived
/// it, nothing else kept.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    cli_session_id: Option<String>,
    is_archived: Option<bool>,
    /// The session's working folder (the aij-test proof, run 2: its
    /// project, from the first call on).
    cwd: Option<String>,
    /// Its title.
    title: Option<String>,
}

/// The longest title taken from a record.
const TITLE_MAX: usize = 200;

/// What the app's record says of the session: its conversation, and
/// (when it names them) its working folder and title.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Told {
    conversation: String,
    cwd: Option<PathBuf>,
    title: Option<String>,
}

impl Told {
    fn of(record: Record) -> Option<Self> {
        Some(Self {
            conversation: record.cli_session_id?,
            // Only a plain absolute folder: never `..`, never relative.
            cwd: record.cwd.map(PathBuf::from).filter(|cwd| {
                cwd.is_absolute()
                    && !cwd
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
            }),
            title: record
                .title
                .map(|title| {
                    title
                        .chars()
                        .map(|c| if c.is_control() { ' ' } else { c })
                        .take(TITLE_MAX)
                        .collect::<String>()
                        .trim()
                        .to_owned()
                })
                .filter(|title| !title.is_empty()),
        })
    }
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

    fn tool_binary(&self, place: &DesktopPlace<'_>, exe: &Path) -> bool {
        tool_binary(place, exe)
    }

    fn tool_requirement(&self) -> Option<&'static str> {
        Some(TOOL_REQUIREMENT)
    }

    fn host_requirement(&self) -> Option<&'static str> {
        Some(HOST_REQUIREMENT)
    }

    fn helper_requirement(&self) -> Option<&'static str> {
        Some(HELPER_REQUIREMENT)
    }

    fn instance_lock(&self) -> Option<&'static str> {
        Some(INSTANCE_LOCK)
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
        AppRecord::Archived => {
            // M1: an archived conversation isn't a live member.
            PeerVerdict::no("the Claude app archived this session")
        }
        AppRecord::Of(told) if told.conversation == conversation => {
            PeerVerdict::Attested(AttestedSession {
                conversation,
                host_session: host,
                tool_pid: step.pid,
                tool_start: step.start_secs,
                cwd: told.cwd,
                title: told.title,
            })
        }
        AppRecord::Of(_) | AppRecord::Unreadable => PeerVerdict::no(
            "the Claude app's record of the session names another conversation, or can't be read",
        ),
        AppRecord::Missing => {
            PeerVerdict::no("this Claude app profile has no record of the session")
        }
    }
}

/// M1: whether `exe` is the Claude Code this profile ships: under the
/// profile's own folder, in `claude-code/<version>/…`, the bundle's
/// `Contents/MacOS/claude` (the folder's real path: no link on the way).
#[must_use]
pub fn tool_binary(place: &DesktopPlace<'_>, exe: &Path) -> bool {
    let Ok(root) = std::fs::canonicalize(place.instance_dir.join(SHIPPED)) else {
        return false;
    };
    exe.strip_prefix(&root)
        .is_ok_and(|rest| rest.components().count() >= 5)
        && exe.ends_with("Contents/MacOS/claude")
}

/// H3: whether the registry still names `session` for its tool process
/// (the entry there, for its pid and conversation, with its start time),
/// and the app's record still names its conversation, not archived (H3,
/// round 2: archiving a conversation drops its kept facts at once).
#[must_use]
pub fn still_current(place: &DesktopPlace<'_>, session: &AttestedSession) -> bool {
    let rel = PathBuf::from("sessions").join(format!("{}.json", session.tool_pid));
    let Ok(Some(bytes)) = record(place.config_dir, &rel, ENTRY_MAX) else {
        return false;
    };
    let registry = serde_json::from_slice::<Entry>(&bytes).is_ok_and(|entry| {
        entry.pid == Some(u64::from(session.tool_pid))
            && entry.session_id.as_deref() == Some(session.conversation.as_str())
            && same_start_strict(
                entry.proc_start.as_deref().and_then(ctime_utc_secs),
                session.tool_start,
            )
    });
    registry
        && plain_id(&session.host_session)
        && matches!(
            app_record(place.instance_dir, &session.host_session),
            AppRecord::Of(told) if told.conversation == session.conversation
        )
}

/// What the app keeps for one of its sessions.
enum AppRecord {
    /// Its conversation (and folder and title).
    Of(Told),
    /// Archived by the app.
    Archived,
    /// A record that isn't a plain file, or doesn't read.
    Unreadable,
    /// None.
    Missing,
}

/// The app's record of `host`, searched in its two levels of folders
/// only.
fn app_record(instance_dir: &Path, host: &str) -> AppRecord {
    let root = crate::claude_live::app_root(instance_dir);
    for account in crate::claude_live::folders_in(&root) {
        for org in crate::claude_live::folders_in(&account) {
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
                Ok(Some(bytes)) => match serde_json::from_slice::<Record>(&bytes) {
                    Ok(Record {
                        is_archived: Some(true),
                        ..
                    }) => AppRecord::Archived,
                    Ok(record) => Told::of(record).map_or(AppRecord::Unreadable, AppRecord::Of),
                    Err(_) => AppRecord::Unreadable,
                },
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

    /// The verdict on the usual chain: the MCP server (5000) under the
    /// session's Claude Code (4242, started at [`START`]).
    fn verdict_of(lab: &Lab) -> PeerVerdict {
        attest(&place(lab), &[step(5000, START + 5), step(4242, START)])
    }

    /// `conv-1`'s Code-tab session, entered, recorded and attested.
    fn attested(lab: &Lab) -> AttestedSession {
        desktop_entry(lab, 4242, START_TEXT);
        record(lab, "conv-1");
        let PeerVerdict::Attested(session) = verdict_of(lab) else {
            unreachable!("attested");
        };
        session
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
        let verdict = verdict_of(&lab);
        assert_eq!(
            verdict,
            PeerVerdict::Attested(AttestedSession {
                conversation: "conv-1".into(),
                host_session: "local_abc-1".into(),
                tool_pid: 4242,
                tool_start: START,
                cwd: None,
                title: Some("t".into()),
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
        assert!(matches!(verdict_of(&lab), PeerVerdict::Unattested { .. }));
    }

    #[test]
    fn the_registry_and_the_app_record_must_agree() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        assert!(
            matches!(verdict_of(&lab), PeerVerdict::Unattested { .. }),
            "no app record"
        );
        record(&lab, "conv-other");
        assert!(
            matches!(verdict_of(&lab), PeerVerdict::Unattested { .. }),
            "a record of another conversation (after /clear the registry is read again)"
        );
        record(&lab, "conv-1");
        assert!(matches!(verdict_of(&lab), PeerVerdict::Attested(_)));
        // A terminal session of the same profile isn't the app's.
        entry(
            &lab,
            4242,
            &format!(
                r#"{{"pid":4242,"sessionId":"conv-1","procStart":"{START_TEXT}","hostSessionId":"local_abc-1","entrypoint":"cli"}}"#
            ),
        );
        assert!(matches!(verdict_of(&lab), PeerVerdict::Unattested { .. }));
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
        assert!(matches!(verdict_of(&lab), PeerVerdict::Unattested { .. }));
        entry(
            &lab,
            4242,
            r#"{"pid":4243,"sessionId":"conv-1","procStart":"x","hostSessionId":"local_abc-1","entrypoint":"claude-desktop"}"#,
        );
        assert!(
            matches!(verdict_of(&lab), PeerVerdict::Unattested { .. }),
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
        assert!(matches!(verdict_of(&lab), PeerVerdict::Unattested { .. }));
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
        assert!(matches!(verdict_of(&lab), PeerVerdict::Attested(_)));
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
        let session = attested(&lab);
        assert!(still_current(&place(&lab), &session));
        // procStart changed: the pid now runs something else.
        desktop_entry(&lab, 4242, "Mon Oct 26 08:33:20 2026");
        assert!(!still_current(&place(&lab), &session));
        // The registry entry gone: the session ended.
        std::fs::remove_file(lab.config.join("sessions/4242.json")).unwrap();
        assert!(!still_current(&place(&lab), &session));
    }

    /// H3 (round 2): a kept session stops being current the moment the app
    /// archives its conversation, or its record names another one or goes.
    #[test]
    fn an_archived_conversation_isnt_current_any_more() {
        let lab = lab();
        let session = attested(&lab);
        assert!(still_current(&place(&lab), &session));
        let file = lab
            .instance
            .join("claude-code-sessions/acct/org/local_abc-1.json");
        std::fs::write(
            &file,
            r#"{"sessionId":"local_abc-1","cliSessionId":"conv-1","isArchived":true}"#,
        )
        .unwrap();
        assert!(!still_current(&place(&lab), &session), "archived");
        record(&lab, "conv-2");
        assert!(
            !still_current(&place(&lab), &session),
            "another conversation"
        );
        std::fs::remove_file(&file).unwrap();
        assert!(!still_current(&place(&lab), &session), "no record");
        record(&lab, "conv-1");
        assert!(still_current(&place(&lab), &session));
    }

    #[test]
    fn only_the_profiles_own_shipped_claude_code_is_its_tool() {
        let lab = lab();
        let shipped = lab
            .instance
            .join("claude-code/2.1.325/485fdac4332e/claude.bundle/Contents/MacOS");
        std::fs::create_dir_all(&shipped).unwrap();
        let root = std::fs::canonicalize(&lab.instance).unwrap();
        let exe = root.join("claude-code/2.1.325/485fdac4332e/claude.bundle/Contents/MacOS/claude");
        assert!(tool_binary(&place(&lab), &exe));
        for not_it in [
            PathBuf::from("/usr/local/bin/claude"),
            root.join("claude-code/claude"),
            root.join("claude-code/2.1.325/485fdac4332e/claude.bundle/Contents/MacOS/sh"),
            lab.config
                .join("claude-code/2.1.325/x/y/Contents/MacOS/claude"),
        ] {
            assert!(!tool_binary(&place(&lab), &not_it), "{}", not_it.display());
        }
        assert_eq!(ClaudeAppPeer.tool_requirement(), Some(TOOL_REQUIREMENT));
    }

    #[test]
    fn an_archived_session_isnt_attested() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        std::fs::write(
            lab.instance
                .join("claude-code-sessions/acct/org/local_abc-1.json"),
            r#"{"sessionId":"local_abc-1","cliSessionId":"conv-1","isArchived":true}"#,
        )
        .unwrap();
        assert!(matches!(verdict_of(&lab), PeerVerdict::Unattested { .. }));
    }

    /// M1: the requirements pin the CLI's identifier, the app's own bundle
    /// identifier (the one its host record names) and Anthropic's team; on
    /// a Mac with the Claude app installed, its Info.plist says the same.
    #[test]
    fn the_requirements_pin_the_identifiers_and_the_team() {
        assert!(TOOL_REQUIREMENT.contains("identifier \"com.anthropic.claude-code\""));
        let host = format!("identifier \"{HOST_BUNDLE_ID}\"");
        assert!(HOST_REQUIREMENT.contains(&host));
        assert!(
            HELPER_REQUIREMENT.starts_with("identifier \"disclaimer\" and "),
            "Low 2: the helper by its identifier, not any of the team's programs"
        );
        for requirement in [TOOL_REQUIREMENT, HOST_REQUIREMENT, HELPER_REQUIREMENT] {
            assert!(requirement.contains("anchor apple generic"));
            assert!(requirement.ends_with(&format!("leaf[subject.OU] = \"{ANTHROPIC_TEAM}\"")));
        }
        assert_eq!(
            crate::claude_live::host_apps()[0].bundle_id.as_deref(),
            Some(HOST_BUNDLE_ID)
        );
        let plist = Path::new("/Applications/Claude.app/Contents/Info.plist");
        if plist.is_file() {
            let read = std::process::Command::new("/usr/libexec/PlistBuddy")
                .args(["-c", "Print :CFBundleIdentifier"])
                .arg(plist)
                .output()
                .unwrap();
            assert_eq!(String::from_utf8_lossy(&read.stdout).trim(), HOST_BUNDLE_ID);
        }
    }

    /// M1 round 2 (the ADR's 4b, "What it rests on"): the installed Claude
    /// app's Electron fuses keep it from running anyone else's code:
    /// `RunAsNode`, `NODE_OPTIONS` and `--inspect` off, asar integrity and
    /// only-from-asar on. Read from its fuse wire (the sentinel, a version,
    /// a count, then one byte a fuse: `0` off, `1` on) on every Mac that
    /// has the app; nothing to read elsewhere. If this fails after a Claude
    /// app update, the host check no longer holds: stop offering the
    /// connector (`not_offered`) until it's rethought.
    #[test]
    fn the_installed_claude_apps_fuses_stay_off() {
        const SENTINEL: &[u8] = b"dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX";
        let framework = Path::new(
            "/Applications/Claude.app/Contents/Frameworks/Electron Framework.framework/Electron Framework",
        );
        let Ok(bytes) = std::fs::read(framework) else {
            return;
        };
        let at = bytes
            .windows(SENTINEL.len())
            .position(|window| window == SENTINEL)
            .expect("the fuse wire");
        let wire = &bytes[at + SENTINEL.len()..];
        assert_eq!(wire[0], 1, "the fuse wire's version");
        let fuses = &wire[2..2 + usize::from(wire[1])];
        let [
            run_as_node,
            _,
            node_options,
            inspect,
            asar_integrity,
            only_asar,
            ..,
        ] = fuses
        else {
            unreachable!("fewer fuses than this check reads: {fuses:?}");
        };
        assert_eq!(
            [*run_as_node, *node_options, *inspect],
            *b"000",
            "RunAsNode, NODE_OPTIONS, --inspect"
        );
        assert_eq!(
            [*asar_integrity, *only_asar],
            *b"11",
            "asar integrity, only from asar"
        );
    }

    /// The aij-test proof, run 2: the app's own record names the session's
    /// working folder and title from its first call, before any index has
    /// read its transcript; a folder that isn't plain and absolute isn't
    /// taken, and a title is one bounded line.
    #[test]
    fn the_apps_record_gives_the_sessions_folder_and_title() {
        let lab = lab();
        desktop_entry(&lab, 4242, START_TEXT);
        let file = lab
            .instance
            .join("claude-code-sessions/acct/org/local_abc-1.json");
        let session = |text: &str| {
            std::fs::write(&file, text).unwrap();
            match verdict_of(&lab) {
                PeerVerdict::Attested(session) => session,
                other => unreachable!("{other:?}"),
            }
        };
        let told = session(
            r#"{"sessionId":"local_abc-1","cliSessionId":"conv-1","cwd":"/Users/you/lab/web","title":"Fix the\nlogin"}"#,
        );
        assert_eq!(told.cwd.as_deref(), Some(Path::new("/Users/you/lab/web")));
        assert_eq!(told.title.as_deref(), Some("Fix the login"));
        for cwd in ["lab/web", "/Users/you/../root"] {
            let told = session(&format!(
                r#"{{"sessionId":"local_abc-1","cliSessionId":"conv-1","cwd":"{cwd}"}}"#
            ));
            assert_eq!(told.cwd, None, "{cwd}");
        }
    }
}
