//! AJ-E2107: "Asks you" for open sessions whose tool doesn't record it,
//! from the tail of their transcript.
//!
//! Only live, interactive sessions outside a desktop app are read (a
//! desktop app records its own status word), each at most its last
//! [`TAIL_BYTES`], and again only when the file changed. The adapter says
//! what the tail waits on ([`juggler_adapter::HarnessAdapter::pending_ask`]);
//! nothing of the text is kept. Worked out, not recorded: the state says
//! "probably".

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use juggler_adapter::live::{Ask, HostKind};

use super::Conversation;
use super::live::Live;
use super::scan::{LiveEntry, State};

/// The most of a transcript read: its last 64 KB.
pub const TAIL_BYTES: u64 = 64 * 1024;

/// A tool call waiting this long with nothing new written probably waits
/// on the user (a permission); a shorter wait is a tool still running.
pub const PENDING_FOR: Duration = Duration::from_secs(60);

/// A file's tail as last read: its size, its time, and what it waited on.
type TailRead = (u64, Option<SystemTime>, Option<Ask>);

/// What each tail last said (one per app; the live refresher's thread
/// uses it).
#[derive(Debug, Default)]
pub struct Tails {
    files: HashMap<PathBuf, TailRead>,
}

impl Tails {
    /// What `path`'s tail waits on, and when the file last changed; read
    /// again only when its size or time changed.
    fn ask(
        &mut self,
        path: &Path,
        read: impl FnOnce(&str) -> Option<Ask>,
    ) -> (Option<Ask>, Option<SystemTime>) {
        let Ok(meta) = std::fs::metadata(path) else {
            self.files.remove(path);
            return (None, None);
        };
        let (len, modified) = (meta.len(), meta.modified().ok());
        if let Some((known_len, known_time, ask)) = self.files.get(path)
            && *known_len == len
            && *known_time == modified
        {
            return (*ask, modified);
        }
        let ask = tail(path).and_then(|text| read(&text));
        self.files.insert(path.to_path_buf(), (len, modified, ask));
        (ask, modified)
    }
}

/// The last [`TAIL_BYTES`] of `path`, from the first whole line.
#[must_use]
pub fn tail(path: &Path) -> Option<String> {
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

/// Whether an open session's state is read from its tail: interactive,
/// in no desktop app, and not already known to ask you or have something
/// to review.
fn reads_tail(entry: &LiveEntry) -> bool {
    !entry.background
        && !matches!(entry.state, Some(State::AsksYou | State::DoneReview))
        && entry
            .places
            .iter()
            .all(|place| place.host.as_ref().and_then(|host| host.kind) != Some(HostKind::Desktop))
}

/// Marks the open sessions whose tail waits on the user as probably
/// asking: an open question at once; another pending call once nothing
/// was written for [`PENDING_FOR`] while it works. `ask_of(tool, tail)` is
/// the tool's adapter. Tails of sessions no longer open are forgotten.
pub(super) fn apply(
    live: &mut HashMap<Live, LiveEntry>,
    conversations: &[Conversation],
    tails: &mut Tails,
    ask_of: &dyn Fn(&str, &str) -> Option<Ask>,
    now: SystemTime,
) {
    let paths: HashMap<(&str, &str, &str), &Path> = conversations
        .iter()
        .map(|c| {
            (
                (
                    c.account_id.as_str(),
                    c.tool.as_str(),
                    c.summary.id.as_str(),
                ),
                c.path.as_path(),
            )
        })
        .collect();
    let mut read: HashSet<PathBuf> = HashSet::new();
    for ((account, tool, id), entry) in live.iter_mut() {
        if !reads_tail(entry) {
            continue;
        }
        let Some(path) = paths.get(&(account.as_str(), tool.as_str(), id.as_str())) else {
            continue;
        };
        read.insert(path.to_path_buf());
        let (ask, modified) = tails.ask(path, |text| ask_of(tool, text));
        let quiet = modified
            .and_then(|at| now.duration_since(at).ok())
            .is_some_and(|age| age >= PENDING_FOR);
        let asks = match ask {
            Some(Ask::Question) => true,
            Some(Ask::Pending) => quiet && entry.state != Some(State::Idle),
            None => false,
        };
        if asks {
            entry.state = Some(State::AsksYou);
            entry.probably = true;
        }
    }
    tails.files.retain(|path, _| read.contains(path));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::conversations::places::Host;
    use crate::features::conversations::scan::Place;
    use juggler_adapter::conversation::ConversationSummary;

    fn conversation(path: &Path, id: &str) -> Conversation {
        Conversation {
            summary: ConversationSummary {
                id: id.to_owned(),
                ..ConversationSummary::default()
            },
            account_id: "a".into(),
            tool: "t".into(),
            path: path.to_path_buf(),
            folder: None,
        }
    }

    fn entry(id: &str, state: Option<State>, kind: Option<HostKind>) -> LiveEntry {
        LiveEntry {
            tool: "t".into(),
            account_id: Some("a".into()),
            session_id: id.into(),
            places: vec![Place {
                pid: 1,
                start: None,
                host: kind.map(|kind| Host {
                    app_id: None,
                    name: "App".into(),
                    kind: Some(kind),
                    pid: 2,
                    start: None,
                    instance_dir: None,
                    instance: None,
                }),
                profile: None,
            }],
            state,
            probably: false,
            viewed_ms: None,
            app_title: None,
            background: false,
        }
    }

    /// The fixture adapter: "Q" in a tail is a question, "P" a pending call.
    fn ask_of(_tool: &str, tail: &str) -> Option<Ask> {
        if tail.contains('Q') {
            Some(Ask::Question)
        } else if tail.contains('P') {
            Some(Ask::Pending)
        } else {
            None
        }
    }

    fn key(id: &str) -> Live {
        ("a".into(), "t".into(), id.into())
    }

    #[test]
    fn a_question_asks_at_once_and_a_pending_call_only_after_a_quiet_minute() {
        let dir = tempfile::tempdir().unwrap();
        let question = dir.path().join("q.jsonl");
        let pending = dir.path().join("p.jsonl");
        std::fs::write(&question, "Q\n").unwrap();
        std::fs::write(&pending, "P\n").unwrap();
        let all = vec![conversation(&question, "q"), conversation(&pending, "p")];
        let mut live = HashMap::from([
            (
                key("q"),
                entry("q", Some(State::Working), Some(HostKind::Terminal)),
            ),
            (key("p"), entry("p", Some(State::Working), None)),
        ]);
        let mut tails = Tails::default();
        let now = SystemTime::now();
        apply(&mut live, &all, &mut tails, &ask_of, now);
        assert_eq!(live[&key("q")].state, Some(State::AsksYou));
        assert!(live[&key("q")].probably, "worked out, so \"probably\"");
        assert_eq!(
            live[&key("p")].state,
            Some(State::Working),
            "a tool still running"
        );
        apply(&mut live, &all, &mut tails, &ask_of, now + PENDING_FOR);
        assert_eq!(live[&key("p")].state, Some(State::AsksYou));
        assert!(live[&key("p")].probably);
    }

    #[test]
    fn desktop_apps_background_runs_and_recorded_asks_are_left_as_they_are() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("q.jsonl");
        std::fs::write(&path, "Q\n").unwrap();
        let all = vec![
            conversation(&path, "d"),
            conversation(&path, "b"),
            conversation(&path, "r"),
        ];
        let mut background = entry("b", Some(State::Working), None);
        background.background = true;
        let mut live = HashMap::from([
            (
                key("d"),
                entry("d", Some(State::Idle), Some(HostKind::Desktop)),
            ),
            (key("b"), background),
            (key("r"), entry("r", Some(State::AsksYou), None)),
        ]);
        apply(
            &mut live,
            &all,
            &mut Tails::default(),
            &ask_of,
            SystemTime::now(),
        );
        assert_eq!(live[&key("d")].state, Some(State::Idle));
        assert_eq!(live[&key("b")].state, Some(State::Working));
        assert!(
            !live[&key("r")].probably,
            "the tool said so: not \"probably\""
        );
    }

    #[test]
    fn a_tail_is_read_again_only_when_its_file_changed_and_at_most_64_kb() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.jsonl");
        let mut text = "x".repeat(usize::try_from(TAIL_BYTES).unwrap());
        text.push_str("\nlast line\n");
        std::fs::write(&path, &text).unwrap();
        let read = tail(&path).unwrap();
        assert_eq!(read, "last line\n", "the cut line is dropped");
        let reads = std::cell::Cell::new(0);
        let mut tails = Tails::default();
        let count = |_: &str| {
            reads.set(reads.get() + 1);
            None
        };
        tails.ask(&path, count);
        tails.ask(&path, count);
        assert_eq!(reads.get(), 1);
    }
}
