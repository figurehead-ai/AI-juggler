//! 0.4.4: whose a moved conversation's lines are, from the move journal
//! (ADR-0031, D-93). A move renames a conversation's history into the
//! target account's folder, whole, so the lines in it written before the
//! move were written on the account it left. Read where the file sits
//! now, a limit hit among them was credited to the target, which then
//! read as out of room the moment the user moved there to carry on.

use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::moving::MoveStep;
use juggler_handover::moving::{Manifest, Phase, Transaction};

use crate::state::AppState;

/// One file or folder a move renamed from one account's folder into
/// another's.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Hop {
    from: PathBuf,
    to: PathBuf,
    /// The tool whose files these are.
    tool: String,
    /// The account it left, and the one it came to.
    left: Side,
    came: Side,
    /// When the move was made (epoch ms): what was written before, was
    /// written there.
    at_ms: i64,
}

/// One side of a move: the account's tool folder, and its id as the move
/// kept it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Side {
    dir: PathBuf,
    id: Option<String>,
}

impl Side {
    /// This account among `folders` (tool folder and id): by its folder,
    /// else by its id. `None`: it isn't one of them any more.
    fn among(&self, folders: &[(PathBuf, String)]) -> Option<String> {
        folders
            .iter()
            .find(|(dir, _)| *dir == self.dir)
            .or_else(|| {
                folders
                    .iter()
                    .find(|(_, id)| self.id.as_deref() == Some(id.as_str()))
            })
            .map(|(_, id)| id.clone())
    }
}

/// Each account's tool folder and id: the folders a line can be spent on.
#[must_use]
pub fn folders_of(accounts: &[juggler_core::model::Account]) -> Vec<(PathBuf, String)> {
    accounts
        .iter()
        .map(|account| (account.state_dir.clone(), account.id.as_str().to_owned()))
        .collect()
}

impl Hop {
    /// Where `path` (at or under where this hop put something) was before.
    fn before(&self, path: &Path) -> Option<PathBuf> {
        let rest = path.strip_prefix(&self.to).ok()?;
        Some(if rest.as_os_str().is_empty() {
            self.from.clone()
        } else {
            self.from.join(rest)
        })
    }
}

/// What the move journal says files were renamed from, and when.
#[derive(Clone, Debug, Default)]
pub struct Moves {
    hops: Vec<Hop>,
}

impl Moves {
    /// Every move AI Juggler knows of, from its journal.
    #[must_use]
    pub fn read(state: &Arc<AppState>) -> Self {
        Self::read_at(&state.data_dir)
    }

    /// [`Self::read`] from the data folder `data_dir` (at start, before
    /// the app's state is made).
    #[must_use]
    pub fn read_at(data_dir: &Path) -> Self {
        let all = Transaction::all(&data_dir.join(juggler_handover::moving::FOLDER));
        Self::of(all.iter().map(|txn| &txn.manifest))
    }

    /// From the moves' own records: only a move that renamed its files
    /// (applying or past it) and wasn't undone (its files are back).
    pub fn of<'a>(manifests: impl IntoIterator<Item = &'a Manifest>) -> Self {
        let hops = manifests
            .into_iter()
            .filter(|manifest| manifest.phase >= Phase::Applying && manifest.phase != Phase::Undone)
            .flat_map(|manifest| {
                manifest
                    .plan
                    .steps
                    .iter()
                    .filter_map(move |step| match step {
                        MoveStep::Rename { from, to, .. } => Some(Hop {
                            from: from.clone(),
                            to: to.clone(),
                            tool: manifest.tool.clone(),
                            left: Side {
                                dir: manifest.request.source.tool_dir.clone(),
                                id: manifest.accounts.first().cloned(),
                            },
                            came: Side {
                                dir: manifest.request.target.tool_dir.clone(),
                                id: manifest.accounts.get(1).cloned(),
                            },
                            at_ms: manifest.created_ms,
                        }),
                        _ => None,
                    })
            })
            .collect();
        Self { hops }
    }

    /// The moves that brought the file now at `path` from where a line in
    /// it written at `ts` was written, newest first: each move after `ts`
    /// along the file's way here (moved A→B→C, a line from before both
    /// came through both; the last is the one it left where it was
    /// written). Empty: it was written where it is.
    fn trail(&self, path: &Path, ts: i64) -> Vec<&Hop> {
        let mut at = path.to_path_buf();
        let mut before = i64::MAX;
        let mut out = Vec::new();
        // Each step goes back to an earlier move: it ends.
        while let Some((hop, from)) = self
            .hops
            .iter()
            .filter(|hop| hop.at_ms < before)
            .filter_map(|hop| hop.before(&at).map(|from| (hop, from)))
            .max_by_key(|(hop, _)| hop.at_ms)
        {
            if ts >= hop.at_ms {
                break;
            }
            out.push(hop);
            at = from;
            before = hop.at_ms;
        }
        out
    }

    /// The folder (by id, among `folders`: tool folder and id) a line
    /// written at `ts` in the file `path` was spent on, read in folder
    /// `here`: `here`, unless a move brought the file in after it; then the
    /// folder it came from. `None` when that folder isn't one of them any
    /// more: never the wrong one.
    #[must_use]
    pub fn spent_on(
        &self,
        folders: &[(PathBuf, String)],
        here: &str,
        path: &Path,
        ts: i64,
    ) -> Option<String> {
        match self.trail(path, ts).last() {
            None => Some(here.to_owned()),
            Some(hop) => hop.left.among(folders),
        }
    }

    /// The folders (by id, among `folders`) the moves carried a line
    /// written at `ts` in the file `path` into, `here` (where it is read)
    /// among them: where a version before 0.4.4, reading it in each,
    /// credited it. Empty when no move carried it.
    #[must_use]
    pub fn carried_into(
        &self,
        folders: &[(PathBuf, String)],
        here: &str,
        path: &Path,
        ts: i64,
    ) -> Vec<String> {
        let trail = self.trail(path, ts);
        if trail.is_empty() {
            return Vec::new();
        }
        let into: BTreeSet<String> = trail
            .iter()
            .filter_map(|hop| hop.came.among(folders))
            .chain([here.to_owned()])
            .collect();
        into.into_iter().collect()
    }

    /// Where the moves put things (a conversation's history, or its
    /// folder), each with its tool: where a moved conversation's lines
    /// are, or were until a later move took them on.
    #[must_use]
    pub fn brought_in(&self) -> BTreeSet<(&str, &Path)> {
        self.hops
            .iter()
            .map(|hop| (hop.tool.as_str(), hop.to.as_path()))
            .collect()
    }
}

/// [`Moves::spent_on`] for one ingest pass: the journal read once, and only
/// when a line asks (a limit hit is rare).
pub struct SpentOn<'a> {
    /// Where the journal is; `None` reads none (a test's).
    state: Option<&'a Arc<AppState>>,
    folders: Vec<(PathBuf, String)>,
    moves: OnceCell<Moves>,
}

impl<'a> SpentOn<'a> {
    /// For a pass over `folders`.
    #[must_use]
    pub fn new(state: &'a Arc<AppState>, folders: &[juggler_core::model::Account]) -> Self {
        Self {
            state: Some(state),
            folders: folders_of(folders),
            moves: OnceCell::new(),
        }
    }

    /// No moves: every line is the folder's it's read in.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn none() -> Self {
        Self {
            state: None,
            folders: Vec::new(),
            moves: OnceCell::new(),
        }
    }

    /// The folder a line written at `ts` in `path`, read in `here`, was
    /// spent on ([`Moves::spent_on`]).
    #[must_use]
    pub fn folder(&self, here: &str, path: &Path, ts: i64) -> Option<String> {
        self.moves
            .get_or_init(|| self.state.map(Moves::read).unwrap_or_default())
            .spent_on(&self.folders, here, path, ts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::moving::{MovePlan, MoveRequest, MoveSide, NotesChoice};

    fn manifest(from: (&str, &str), to: (&str, &str), at_ms: i64, phase: Phase) -> Manifest {
        let side = |dir: &str| MoveSide {
            tool_dir: PathBuf::from(dir),
            host_dir: None,
            version: None,
        };
        Manifest {
            format: juggler_handover::moving::FORMAT.into(),
            version: juggler_handover::moving::VERSION,
            id: format!("m-{at_ms}"),
            tool: "claude-code".into(),
            created_ms: at_ms,
            request: MoveRequest {
                conversation: "s-1".into(),
                source: side(from.0),
                target: side(to.0),
                notes: NotesChoice::KeepTarget,
            },
            plan: MovePlan {
                steps: vec![
                    MoveStep::Rename {
                        from: format!("{}/projects/p/s-1.jsonl", from.0).into(),
                        to: format!("{}/projects/p/s-1.jsonl", to.0).into(),
                        history: true,
                    },
                    MoveStep::Rename {
                        from: format!("{}/projects/p/s-1", from.0).into(),
                        to: format!("{}/projects/p/s-1", to.0).into(),
                        history: false,
                    },
                ],
                ..MovePlan::default()
            },
            phase,
            entries: Vec::new(),
            previous: None,
            problem: None,
            places: Vec::new(),
            accounts: vec![from.1.to_owned(), to.1.to_owned()],
            preserved: None,
        }
    }

    fn folders() -> Vec<(PathBuf, String)> {
        ["a", "b", "c"]
            .iter()
            .map(|id| (PathBuf::from(format!("/u/{id}")), (*id).to_owned()))
            .collect()
    }

    #[test]
    fn a_line_is_the_folders_it_was_written_in_along_its_moves() {
        // Moved a→b at 100, then b→c at 200; read in c.
        let moves = Moves::of(&[
            manifest(("/u/a", "a"), ("/u/b", "b"), 100, Phase::Committed),
            manifest(("/u/b", "b"), ("/u/c", "c"), 200, Phase::Committed),
        ]);
        let file = Path::new("/u/c/projects/p/s-1.jsonl");
        let on = |ts| moves.spent_on(&folders(), "c", file, ts);
        assert_eq!(on(50).as_deref(), Some("a"));
        assert_eq!(on(150).as_deref(), Some("b"));
        assert_eq!(on(250).as_deref(), Some("c"));
        // A file in a moved folder (a subagent's history) too.
        let nested = Path::new("/u/c/projects/p/s-1/subagents/agent-1.jsonl");
        assert_eq!(
            moves.spent_on(&folders(), "c", nested, 50).as_deref(),
            Some("a")
        );
        // Another conversation is where it's read.
        let other = Path::new("/u/c/projects/p/s-2.jsonl");
        assert_eq!(
            moves.spent_on(&folders(), "c", other, 50).as_deref(),
            Some("c")
        );
    }

    #[test]
    fn a_line_was_credited_in_every_folder_a_move_carried_it_into() {
        // a→b at 100, b→c at 200, back c→a at 300; read in a.
        let moves = Moves::of(&[
            manifest(("/u/a", "a"), ("/u/b", "b"), 100, Phase::Committed),
            manifest(("/u/b", "b"), ("/u/c", "c"), 200, Phase::Committed),
            manifest(("/u/c", "c"), ("/u/a", "a"), 300, Phase::Committed),
        ]);
        let file = Path::new("/u/a/projects/p/s-1.jsonl");
        let into = |ts| moves.carried_into(&folders(), "a", file, ts);
        assert_eq!(into(50), ["a", "b", "c"], "spent on a, read in b and c");
        assert_eq!(into(150), ["a", "c"], "spent on b");
        assert_eq!(into(250), ["a"], "spent on c, read in a");
        assert!(into(350).is_empty(), "written where it is");
        // A folder no longer there isn't one.
        let gone = Moves::of(&[manifest(
            ("/u/a", "a"),
            ("/u/z", "z"),
            100,
            Phase::Committed,
        )]);
        let there = Path::new("/u/z/projects/p/s-1.jsonl");
        assert_eq!(gone.carried_into(&folders(), "b", there, 50), ["b"]);
        assert_eq!(
            gone.brought_in().into_iter().collect::<Vec<_>>(),
            [
                ("claude-code", Path::new("/u/z/projects/p/s-1")),
                ("claude-code", Path::new("/u/z/projects/p/s-1.jsonl"))
            ]
        );
    }

    #[test]
    fn moved_back_its_own_lines_are_its_own_again() {
        // a→b at 100, then back b→a at 200; read in a.
        let moves = Moves::of(&[
            manifest(("/u/a", "a"), ("/u/b", "b"), 100, Phase::Committed),
            manifest(("/u/b", "b"), ("/u/a", "a"), 200, Phase::Committed),
        ]);
        let file = Path::new("/u/a/projects/p/s-1.jsonl");
        let on = |ts| moves.spent_on(&folders(), "a", file, ts);
        assert_eq!(
            (on(50), on(150), on(250)),
            (Some("a".into()), Some("b".into()), Some("a".into()))
        );
    }

    #[test]
    fn only_a_move_that_renamed_and_stands_counts_and_a_gone_folder_gets_nothing() {
        let file = Path::new("/u/b/projects/p/s-1.jsonl");
        for phase in [Phase::Planned, Phase::BackedUp, Phase::Undone] {
            let moves = Moves::of(&[manifest(("/u/a", "a"), ("/u/b", "b"), 100, phase)]);
            assert_eq!(
                moves.spent_on(&folders(), "b", file, 50).as_deref(),
                Some("b"),
                "{phase:?}"
            );
        }
        let moves = Moves::of(&[manifest(("/u/a", "a"), ("/u/b", "b"), 100, Phase::Applied)]);
        assert_eq!(
            moves.spent_on(&folders(), "b", file, 50).as_deref(),
            Some("a")
        );
        // The folder it left was removed from AI Juggler: no one's, not b's.
        let gone = Moves::of(&[manifest(
            ("/u/z", "z"),
            ("/u/b", "b"),
            100,
            Phase::Committed,
        )]);
        assert_eq!(gone.spent_on(&folders(), "b", file, 50), None);
    }
}
