//! AJ-E2107: "Asks you" for open sessions whose tool doesn't record it,
//! and the state of open sessions whose tool records none at all (the
//! Codex app's threads, its CLI in a terminal), from the tail of their
//! transcript.
//!
//! Each tail is at most its last [`TAIL_BYTES`], read again only when the
//! file changed. The adapter says what the tail waits on
//! ([`juggler_adapter::HarnessAdapter::pending_ask`], for interactive
//! sessions outside a desktop app, which records its own status word) and
//! how its last turn stands ([`juggler_adapter::HarnessAdapter::tail_turn`],
//! for a session with no state yet); nothing of the text is kept. With no
//! turn in the tail, a transcript written in the last [`FRESH_FOR`] is
//! working. Worked out, not recorded: the state says "probably".

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use juggler_adapter::live::{Ask, HostKind, TailTurn};

use super::Conversation;
use super::live::Live;
use super::scan::{LiveEntry, State};

/// The most of a transcript read: its last 64 KB.
pub const TAIL_BYTES: u64 = 64 * 1024;

/// A tool call waiting this long with nothing new written probably waits
/// on the user (a permission); a shorter wait is a tool still running.
pub const PENDING_FOR: Duration = Duration::from_secs(60);

/// A session with no state whose transcript was written this recently is
/// working, when its tail names no turn.
pub const FRESH_FOR: Duration = Duration::from_secs(10);

/// What a tail says: what it waits on, and how its last turn stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Said {
    /// What it waits on ([`juggler_adapter::HarnessAdapter::pending_ask`]).
    pub ask: Option<Ask>,
    /// Its last turn ([`juggler_adapter::HarnessAdapter::tail_turn`]).
    pub turn: Option<TailTurn>,
}

/// A file's tail as last read: its size, its time, and what it said.
type TailRead = (u64, Option<SystemTime>, Said);

/// What each tail last said (one per app; the live refresher's thread
/// uses it).
#[derive(Debug, Default)]
pub struct Tails {
    files: HashMap<PathBuf, TailRead>,
}

impl Tails {
    /// What `path`'s tail says, and when the file last changed; read
    /// again only when its size or time changed.
    fn read(
        &mut self,
        path: &Path,
        read: impl FnOnce(&str) -> Said,
    ) -> (Said, Option<SystemTime>) {
        let Ok(meta) = std::fs::metadata(path) else {
            self.files.remove(path);
            return (Said::default(), None);
        };
        let (len, modified) = (meta.len(), meta.modified().ok());
        if let Some((known_len, known_time, said)) = self.files.get(path)
            && *known_len == len
            && *known_time == modified
        {
            return (*said, modified);
        }
        let said = tail(path).map(|text| read(&text)).unwrap_or_default();
        self.files.insert(path.to_path_buf(), (len, modified, said));
        (said, modified)
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

/// Whether an open session's state is worked out from its tail: an
/// interactive one whose tool recorded none.
fn needs_state(entry: &LiveEntry) -> bool {
    !entry.background && entry.state.is_none()
}

/// The state a tail's last turn says, for a session with none: a running
/// turn works however quiet the file is; with no turn in the tail, a file
/// written within [`FRESH_FOR`] works.
fn state_of_turn(turn: Option<TailTurn>, fresh: bool) -> Option<State> {
    match turn {
        Some(TailTurn::Running) => Some(State::Working),
        Some(TailTurn::Waiting) => Some(State::AsksYou),
        Some(TailTurn::Ended) => Some(State::Idle),
        None => fresh.then_some(State::Working),
    }
}

/// Gives the open sessions with no state the one their tail's last turn
/// says, and marks those whose tail waits on the user as probably asking:
/// an open question at once; another pending call once nothing was
/// written for [`PENDING_FOR`] while it works. `say(tool, tail)` is the
/// tool's adapter. Tails of sessions no longer open are forgotten.
pub(super) fn apply(
    live: &mut HashMap<Live, LiveEntry>,
    conversations: &[Conversation],
    tails: &mut Tails,
    say: &dyn Fn(&str, &str) -> Said,
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
        let (asks_from_tail, state_from_tail) = (reads_tail(entry), needs_state(entry));
        if !asks_from_tail && !state_from_tail {
            continue;
        }
        let Some(path) = paths.get(&(account.as_str(), tool.as_str(), id.as_str())) else {
            continue;
        };
        read.insert(path.to_path_buf());
        let (said, modified) = tails.read(path, |text| say(tool, text));
        let age = modified.and_then(|at| now.duration_since(at).ok());
        if state_from_tail
            && let Some(state) = state_of_turn(said.turn, age.is_some_and(|age| age < FRESH_FOR))
        {
            entry.state = Some(state);
            entry.probably = true;
        }
        if !asks_from_tail {
            continue;
        }
        let quiet = age.is_some_and(|age| age >= PENDING_FOR);
        let asks = match said.ask {
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

    /// The fixture adapter: "Q" in a tail is a question, "P" a pending
    /// call; "R" a running turn, "W" one waiting, "E" an ended one.
    fn ask_of(_tool: &str, tail: &str) -> Said {
        let ask = if tail.contains('Q') {
            Some(Ask::Question)
        } else if tail.contains('P') {
            Some(Ask::Pending)
        } else {
            None
        };
        let turn = [
            ('R', TailTurn::Running),
            ('W', TailTurn::Waiting),
            ('E', TailTurn::Ended),
        ]
        .into_iter()
        .find_map(|(mark, turn)| tail.contains(mark).then_some(turn));
        Said { ask, turn }
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
            Said::default()
        };
        tails.read(&path, count);
        tails.read(&path, count);
        assert_eq!(reads.get(), 1);
    }

    /// The 0.1.5 review: a Codex app thread running a turn showed Idle.
    #[test]
    fn a_session_with_no_state_takes_its_tails_turn_even_in_a_desktop_app() {
        let dir = tempfile::tempdir().unwrap();
        let mut all = Vec::new();
        for (id, text) in [("r", "R\n"), ("w", "W\n"), ("e", "E\n"), ("n", "-\n")] {
            let path = dir.path().join(format!("{id}.jsonl"));
            std::fs::write(&path, text).unwrap();
            all.push(conversation(&path, id));
        }
        let mut live: HashMap<Live, LiveEntry> = ["r", "w", "e", "n"]
            .into_iter()
            .map(|id| (key(id), entry(id, None, Some(HostKind::Desktop))))
            .collect();
        let mut tails = Tails::default();
        // Long after the last write: a running turn still works.
        let later = SystemTime::now() + Duration::from_secs(15 * 60);
        apply(&mut live, &all, &mut tails, &ask_of, later);
        assert_eq!(live[&key("r")].state, Some(State::Working));
        assert_eq!(live[&key("w")].state, Some(State::AsksYou));
        assert_eq!(live[&key("e")].state, Some(State::Idle));
        assert!(["r", "w", "e"].iter().all(|id| live[&key(id)].probably));
        assert_eq!(live[&key("n")].state, None, "a quiet tail with no turn");
        // Written just now, with no turn in its tail: working.
        let mut fresh = HashMap::from([(key("n"), entry("n", None, Some(HostKind::Desktop)))]);
        apply(&mut fresh, &all, &mut tails, &ask_of, SystemTime::now());
        assert_eq!(fresh[&key("n")].state, Some(State::Working));
        assert!(fresh[&key("n")].probably);
    }

    #[test]
    fn a_recorded_state_is_not_replaced_by_the_tails_turn() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("e.jsonl");
        std::fs::write(&path, "E\n").unwrap();
        let all = vec![conversation(&path, "k")];
        let mut live = HashMap::from([(key("k"), entry("k", Some(State::Working), None))]);
        apply(
            &mut live,
            &all,
            &mut Tails::default(),
            &ask_of,
            SystemTime::now(),
        );
        assert_eq!(live[&key("k")].state, Some(State::Working));
        assert!(!live[&key("k")].probably);
    }
}
