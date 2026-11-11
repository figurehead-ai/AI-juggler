//! AJ-E2101 (D-37): what is open now, from the tools' own per-session
//! files, never from a process's environment. Each adapter reads its
//! tool's files ([`crate::ToolLive::live_sessions`],
//! [`crate::ToolLive::app_sessions`]) and describes the apps its
//! sessions run in as data ([`HostApp`], ADR-0019); the app only joins what
//! they return with the process table (pids, start times, parent chains).
//!
//! Reads are tolerant: a file that doesn't parse, or a field of another
//! shape, is left out (never a guess, never a stale value). Only the
//! fields named in each reader's struct are kept; summary text the apps
//! write next to them is never read into memory as a value.

use std::path::{Path, PathBuf};

/// How busy an open session is, as its tool records it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiveStatus {
    /// A turn is running.
    Busy,
    /// Waiting for the user.
    Idle,
    /// AJ-E2107: waiting on the user's answer (a permission, a question),
    /// as the tool records it.
    Waiting,
    /// The file didn't say (or said something new).
    #[default]
    Unknown,
}

/// What a desktop app recorded about a session's last turn: the status
/// word only (D-37), never its summary text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnOutcome {
    /// The session is waiting on the user (a question, a permission).
    Blocked,
    /// It finished something the user should look at.
    ReviewReady,
    /// It finished; nothing to look at.
    Completed,
}

impl TurnOutcome {
    /// The app's word for it; `None` for a word not known yet.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "blocked" => Some(Self::Blocked),
            "review_ready" => Some(Self::ReviewReady),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }
}

/// AJ-E2107: what a transcript's tail says a session waits on, when its
/// last tool call has no result yet. Worked out from the tail, so the UI
/// says "probably".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// A question to the user is open (the tool's own question tool).
    Question,
    /// A tool call waits for its result: a permission, or a long run.
    Pending,
}

/// What a transcript's tail says of a session's last turn, for a tool
/// that records no status word of its own (Codex). Worked out from the
/// tail, so the UI says "probably".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TailTurn {
    /// A turn started and hasn't ended: it is working.
    Running,
    /// The running turn waits on the user's answer (an approval, a
    /// question).
    Waiting,
    /// The last turn ended: waiting for the next thing.
    Ended,
}

/// The most of a session file [`tail`] reads: its last 64 KB (a turn's
/// markers and what it waits on are near the end).
pub const TAIL_BYTES: u64 = 64 * 1024;

/// The last [`TAIL_BYTES`] of `path`, from the first whole line; `None`
/// when it can't be read.
#[must_use]
pub fn tail(path: &Path) -> Option<String> {
    use std::io::{Read as _, Seek as _, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::with_capacity(usize::try_from(len - start).unwrap_or(0));
    file.take(TAIL_BYTES).read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    Some(if start > 0 {
        // Cut mid-line: from the next line on.
        text.split_once('\n')
            .map(|(_, rest)| rest.to_owned())
            .unwrap_or_default()
    } else {
        text.into_owned()
    })
}

/// What a transcript's tail says of a usage limit: the session's last turn
/// stopped at it, as the tool itself recorded (its own error record, never
/// a guess from words on screen), and nothing has answered since. Read by
/// [`crate::ToolQuota::limit_hit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitHit {
    /// When the tool recorded it, epoch ms.
    pub at_ms: i64,
    /// The vendor's reset for it, epoch ms, when the record names one.
    pub resets_at_ms: Option<i64>,
    /// The window that ran out, when the record names it.
    pub window: Option<juggler_core::quota::WindowKind>,
}

/// One open session, as its tool's own per-session file says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LiveSession {
    /// The tool's id for its conversation (what `--resume` takes).
    pub session_id: String,
    /// The process that runs it, when the file names it.
    pub pid: Option<u32>,
    /// When that process started, epoch seconds, as the file recorded it:
    /// a pid counts as this session only when its start time matches.
    pub started_secs: Option<i64>,
    /// Its working folder.
    pub cwd: Option<PathBuf>,
    /// Busy or idle.
    pub status: LiveStatus,
    /// The host app's own id for the session (the Claude app's
    /// `local_<id>`), when the tool records one.
    pub host_session_id: Option<String>,
    /// How it was started (`cli`, `claude-desktop`, …), as recorded.
    pub entrypoint: Option<String>,
    /// The name the user gave it, when any.
    pub name: Option<String>,
    /// Not an interactive session: a headless run (`-p`, `exec`, an SDK).
    pub background: bool,
}

/// One session as a desktop app keeps it (its sidebar entry), read field
/// by field from the app's own state file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppSession {
    /// The app's id for it (`local_<id>`).
    pub host_session_id: String,
    /// The tool's conversation id it runs.
    pub session_id: Option<String>,
    /// Its working folder.
    pub cwd: Option<PathBuf>,
    /// Its title in the app's sidebar.
    pub title: Option<String>,
    /// Archived in the app (not in its sidebar).
    pub archived: bool,
    /// Last activity, epoch ms.
    pub last_activity_ms: Option<i64>,
    /// When the user last had it in front, epoch ms (evidence, not proof:
    /// "viewed N min ago").
    pub last_focused_ms: Option<i64>,
    /// What its last turn left, as a status word.
    pub outcome: Option<TurnOutcome>,
    /// The session it was started from, when the app recorded one.
    pub spawned_from: Option<String>,
}

/// What kind of app hosts a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostKind {
    /// A tool's own desktop app.
    Desktop,
    /// A terminal.
    Terminal,
    /// An editor (a session in its integrated terminal).
    Editor,
}

/// A host app as data (ADR-0019): how it is recognised in a process's
/// parent chain, how its instances are told apart, and how Go to it
/// reaches a session in it. Tool apps come from their adapters
/// ([`crate::ToolDiscovery::host_apps`]); terminals and editors from the
/// app's own list.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct HostApp {
    /// Stable id (`terminal`, `cursor`, …).
    pub id: String,
    /// Its name, as the Dock shows it.
    pub name: String,
    /// The app bundle's folder name (`Terminal.app`): a process inside it
    /// is this app.
    pub bundle: String,
    /// Its bundle id, when known.
    #[serde(default)]
    pub bundle_id: Option<String>,
    /// Desktop app, terminal or editor.
    pub kind: HostKind,
    /// The argument whose value tells its instances apart
    /// (`--user-data-dir`), when it runs several.
    #[serde(default)]
    pub instance_arg: Option<String>,
    /// The instance folder used without that argument, relative to home.
    #[serde(default)]
    pub default_instance: Option<String>,
    /// A link that opens one of its sessions, with `{id}`.
    #[serde(default)]
    pub thread_link: Option<String>,
    /// A link that opens a folder in it, with `{folder}`.
    #[serde(default)]
    pub folder_link: Option<String>,
    /// A command that opens a folder in it (`code`).
    #[serde(default)]
    pub folder_command: Option<String>,
    /// An `AppleScript` that selects the tab on `{tty}`; run only when the
    /// user allowed terminal tabs in Settings.
    #[serde(default)]
    pub tab_script: Option<String>,
    /// D-62: its name on screen, when the Dock's would read as a tool's
    /// ("Claude app", beside the tool Claude Code). Words only: matching
    /// and grouping keep [`HostApp::name`].
    #[serde(default)]
    pub shown: Option<String>,
    /// D-62 (AJ-E3203): its instances other than the default one are
    /// kept side by side in one folder (the Claude app's profiles), so the
    /// names of that folder's other folders are its instances that aren't
    /// running ([`HostApp::sibling_instances`]). Off for every app that
    /// doesn't say so.
    #[serde(default)]
    pub siblings: bool,
    /// 0.3.5: the environment variable its instances' sessions read their
    /// tool's folder from (`CLAUDE_CONFIG_DIR`). Starting a profile passes
    /// it with the folder that profile's sessions use, as a launcher that
    /// pairs a profile with its folder does; without it the profile's
    /// sessions would go to the tool's default folder.
    #[serde(default)]
    pub folder_env: Option<String>,
    /// 0.3.5: a link, with `{id}`, that hands one of the tool's sessions
    /// to an instance of this app (it takes the session from its folder).
    /// Sent to that instance's own process, never to the app at large.
    #[serde(default)]
    pub handoff_link: Option<String>,
    /// 0.3.5: the folder, inside an instance folder, where the app
    /// records a session it took: a `local_{id}.json` below it is the
    /// proof the hand-off arrived.
    #[serde(default)]
    pub handoff_record: Option<String>,
    /// 0.4.1: a link, with `{id}` the app's own id for one of its sessions
    /// (the Claude app's `local_<uuid>`), that opens that session in the
    /// instance it runs in. Sent to that instance's own process, never to
    /// the app at large (every profile shares one bundle id).
    #[serde(default)]
    pub session_link: Option<String>,
}

impl HostApp {
    /// Its name on screen: [`HostApp::shown`], else the Dock's.
    #[must_use]
    pub fn shown_name(&self) -> &str {
        self.shown.as_deref().unwrap_or(&self.name)
    }

    /// The link that opens session `id` in it, when it has one. An id that
    /// isn't plain letters, digits, `-` and `_` gets none (never passed).
    #[must_use]
    pub fn thread_link_for(&self, id: &str) -> Option<String> {
        self.thread_link
            .as_ref()
            .filter(|_| plain_id(id))
            .map(|link| link.replace("{id}", id))
    }

    /// The link that opens the app's own session `id` in an instance
    /// ([`HostApp::session_link`]); never for an id that isn't plain.
    #[must_use]
    pub fn session_link_for(&self, id: &str) -> Option<String> {
        self.session_link
            .as_ref()
            .filter(|_| plain_id(id))
            .map(|link| link.replace("{id}", id))
    }

    /// The link that hands session `id` to an instance, when it takes
    /// one; never for an id that isn't plain (as [`HostApp::thread_link_for`]).
    #[must_use]
    pub fn handoff_link_for(&self, id: &str) -> Option<String> {
        self.handoff_link
            .as_ref()
            .filter(|_| plain_id(id))
            .map(|link| link.replace("{id}", id))
    }
}

/// Plain letters, digits, `-` and `_`: an id that may go into a link.
fn plain_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A file's text when it is small enough to be one of these state files
/// (a larger one is not read: these are a few KB).
#[must_use]
pub fn read_small(path: &Path, max: u64) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > max {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// A process start time as `ps -o lstart` prints it in UTC (`Sat Oct 24
/// 08:12:34 2026`, how Claude Code records `procStart`), as epoch seconds.
#[must_use]
pub fn ctime_utc_secs(text: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let words: Vec<&str> = text.split_whitespace().collect();
    let [_, month, day, time, year] = words.as_slice() else {
        return None;
    };
    let month = i64::try_from(MONTHS.iter().position(|m| m == month)?).ok()? + 1;
    let day: i64 = day.parse().ok()?;
    let year: i64 = year.parse().ok()?;
    let mut hms = time.split(':').map(str::parse::<i64>);
    let (Some(Ok(h)), Some(Ok(m)), Some(Ok(s)), None) =
        (hms.next(), hms.next(), hms.next(), hms.next())
    else {
        return None;
    };
    if !(1..=31).contains(&day) || h > 23 || m > 59 || s > 60 {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + h * 3600 + m * 60 + s)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Whether a recorded start time and the process's own are the same
/// process (a second apart at most: one is rounded, the other truncated).
#[must_use]
pub fn same_start(recorded: Option<i64>, actual: Option<i64>) -> bool {
    match (recorded, actual) {
        (Some(recorded), Some(actual)) => (recorded - actual).abs() <= 1,
        // Nothing recorded: the pid alone (a file from a tool that doesn't
        // record it); nothing known of the process: not the same.
        (None, Some(_)) => true,
        (_, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ctime_start_reads_as_epoch_seconds() {
        assert_eq!(ctime_utc_secs("Thu Jan  1 00:00:00 1970"), Some(0));
        assert_eq!(
            ctime_utc_secs("Sat Oct 24 08:12:34 2026"),
            Some(1_792_829_554)
        );
        assert_eq!(
            ctime_utc_secs("Sat Oct  4 08:12:34 2026"),
            Some(1_791_101_554)
        );
        assert_eq!(ctime_utc_secs("2026-10-24T08:12:34Z"), None);
        assert_eq!(ctime_utc_secs("Sat Foo 24 08:12:34 2026"), None);
        assert_eq!(ctime_utc_secs("Sat Oct 24 25:12:34 2026"), None);
        assert_eq!(ctime_utc_secs(""), None);
    }

    #[test]
    fn a_pid_is_the_same_process_only_when_its_start_matches() {
        assert!(same_start(Some(100), Some(100)));
        assert!(same_start(Some(100), Some(101)));
        assert!(!same_start(Some(100), Some(160)), "a reused pid");
        assert!(same_start(None, Some(100)));
        assert!(!same_start(Some(100), None));
    }

    #[test]
    fn a_thread_link_takes_only_a_plain_id() {
        let app = HostApp {
            id: "x".into(),
            name: "X".into(),
            bundle: "X.app".into(),
            bundle_id: None,
            kind: HostKind::Desktop,
            instance_arg: None,
            default_instance: None,
            thread_link: Some("x://threads/{id}".into()),
            folder_link: None,
            folder_command: None,
            tab_script: None,
            shown: None,
            siblings: false,
            folder_env: None,
            handoff_link: None,
            handoff_record: None,
            session_link: None,
        };
        assert_eq!(
            app.thread_link_for("0199-ab_c").as_deref(),
            Some("x://threads/0199-ab_c")
        );
        assert_eq!(app.thread_link_for("a/../b"), None);
        assert_eq!(app.thread_link_for("a b"), None);
        assert_eq!(app.thread_link_for(""), None);
    }

    #[test]
    fn a_large_or_linked_file_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let small = dir.path().join("s.json");
        std::fs::write(&small, "{}").unwrap();
        assert_eq!(read_small(&small, 10).as_deref(), Some("{}"));
        assert_eq!(read_small(&small, 1), None);
        #[cfg(unix)]
        {
            let link = dir.path().join("l.json");
            std::os::unix::fs::symlink(&small, &link).unwrap();
            assert_eq!(read_small(&link, 10), None);
        }
    }
}

/// ADR-0028: a session another started, as a desktop app that keeps that
/// link in its own records says (the Claude app's side sessions).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFamilyMember {
    /// The app's record of it.
    pub record: PathBuf,
    /// The record's id in the app.
    pub local_id: String,
    /// Its conversation id (its transcript's).
    pub conversation: String,
    /// The conversation it was started from.
    pub parent_conversation: String,
    /// Its title, when it has one.
    pub title: Option<String>,
}

/// One conversation a desktop app instance keeps a record of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRecord {
    /// Its tool's id for it.
    pub id: String,
    /// What the app calls it (its sidebar's name), when it has one.
    pub title: Option<String>,
}

/// ADR-0028: a desktop app that keeps who started which session in its own
/// records, outside the conversations' files, so a switch has to carry
/// those records for the family to nest on the target as it did. Its
/// adapter implements it ([`crate::ToolRuntime::host_family`]).
pub trait HostFamily: Send + Sync {
    /// The sessions started from `conversation` in the app instance at
    /// `instance_dir`, and theirs: parents first, each once.
    fn descendants(&self, instance_dir: &Path, conversation: &str) -> Vec<HostFamilyMember>;

    /// Every conversation the app instance at `instance_dir` keeps a
    /// record of, the most recently active last. Read whether the app
    /// runs or not: a closed profile's sessions are its own as much as an
    /// open one's.
    fn conversations(&self, _instance_dir: &Path) -> Vec<HostRecord> {
        Vec::new()
    }

    /// The folder holding the target instance's record of
    /// `conversation`, where the family's records go beside it.
    fn records_folder_of(&self, instance_dir: &Path, conversation: &str) -> Option<PathBuf>;

    /// Places the records of `members` in `folder`, each pointing at its
    /// parent there; `renamed` maps a conversation id the move forked to
    /// its new one. Only while the app isn't running (the caller makes
    /// sure). All or nothing.
    ///
    /// # Errors
    ///
    /// A record that can't be carried, in words; the write that failed.
    fn place(
        &self,
        members: &[HostFamilyMember],
        folder: &Path,
        renamed: &[(String, String)],
    ) -> Result<Vec<PathBuf>, crate::AdapterError>;
}
