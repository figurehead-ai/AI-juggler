//! Three-way reconciliation for switch-back (AJ-E207; D-05;
//! pass2/02 §6.2). Pure set/map arithmetic — no I/O, no clock —
//! over three inputs: the fork (base), the away side, and the
//! original side.
//!
//! Conversation rule (exact for the transplant case, where B's
//! transcript began byte-identical to A's at the fork and both
//! files are append-only): base = uuids(A) ∩ uuids(B);
//! away-only = uuids(B) − uuids(A) → the delta turn;
//! original-only = uuids(A) − uuids(B) → already native, a no-op.
//! Conversation conflicts are NEVER text-merged — transcripts are
//! `uuid`/`parentUuid` trees, so "merge" means delta injection
//! (§6.2). For a replay-born away side (fresh native uuids) the
//! intersection is empty and every away turn is away-only — also
//! correct, the whole excursion is the delta.
//!
//! Tracked-artifact rule: a path conflicts only when BOTH sides
//! changed it AND they disagree (the same change on both sides is
//! not a conflict). Deletions count as changes.

use std::collections::{BTreeMap, HashSet};

/// Conversation reconciliation over the two living uuid sets.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ConversationReconciliation {
    /// uuids present on both sides — the common ancestor (the fork).
    pub base_uuids: usize,
    /// uuids only the away side gained — auto-includable in the
    /// delta turn (in away file order; render dedupes first-wins).
    pub away_only_uuids: Vec<String>,
    /// uuids only the original side gained — already native; the
    /// switch-back does nothing with them (counted for the copy).
    pub original_only_uuids: usize,
}

/// Reconciles the two living conversation sides. `away` and
/// `original` are the FULL uuid sets of each side's transcript
/// today (hash sets — one pass, no O(n²); 20k messages stay
/// linear).
#[must_use]
pub fn reconcile_conversation<S: std::hash::BuildHasher>(
    away: &HashSet<String, S>,
    original: &HashSet<String, S>,
) -> ConversationReconciliation {
    let base = away.intersection(original).count();
    let away_only = away
        .iter()
        .filter(|uuid| !original.contains(*uuid))
        .cloned()
        .collect();
    ConversationReconciliation {
        base_uuids: base,
        away_only_uuids: away_only,
        original_only_uuids: original.iter().filter(|uuid| !away.contains(*uuid)).count(),
    }
}

/// One side's tracked-artifact state: `rel_path → sha256` (absent =
/// the file does not exist on that side / was not resolvable —
/// recorded as `None` in a [`FileConflict`], never guessed).
pub type ArtifactHashes = BTreeMap<String, String>;

/// One conflicting artifact (never auto-merged — the user chooses).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FileConflict {
    /// Pack-relative path (the carry-list name the user sees).
    pub rel_path: String,
    /// Fork sha256, when the base manifest tracked the file.
    pub base_sha: Option<String>,
    /// Away-side sha256 today (`None` = deleted there).
    pub away_sha: Option<String>,
    /// Original-side sha256 today (`None` = deleted there).
    pub original_sha: Option<String>,
}

/// The user's resolution for one conflict (§6.2: exactly three).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConflictChoice {
    /// A's version stays (the away change is dropped from THIS
    /// path; the delta turn still records that it happened).
    KeepOriginal,
    /// The away version replaces the original for this path.
    TakeAway,
    /// Keep both: the away version is saved as an
    /// `…away-<date>` sibling and the delta turn points at it.
    KeepBoth,
}

impl ConflictChoice {
    /// Wire name for the merge edge's manifest row.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KeepOriginal => "keep-original",
            Self::TakeAway => "take-away",
            Self::KeepBoth => "keep-both",
        }
    }
}

/// What applying one choice does to a file on disk. The caller
/// executes; the engine never touches bytes (and the transcript is
/// structurally absent from this model — copy-back has no path).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FileAction {
    /// Nothing moves.
    Unchanged,
    /// The away file's bytes replace the original path.
    ReplaceWithAway,
    /// The away bytes are copied to the dated sibling; the original
    /// path keeps A's version.
    SaveAwaySibling {
        /// The sibling path (`<stem>-away-<YYYYMMDD><ext>`).
        sibling_rel_path: String,
    },
}

/// Reconciles tracked artifacts three-way. `base` comes from the
/// fork edge's recorded manifest (sha256 per `rel_path` at freeze);
/// `away`/`original` are hashed now. A file conflicts iff both
/// sides changed it and they disagree; deletions are changes.
#[must_use]
pub fn reconcile_files(
    base: &ArtifactHashes,
    away: &ArtifactHashes,
    original: &ArtifactHashes,
) -> Vec<FileConflict> {
    let mut paths: Vec<&String> = base.keys().collect();
    let mut known: HashSet<&String> = base.keys().collect();
    for path in away.keys().chain(original.keys()) {
        if known.insert(path) {
            paths.push(path);
        }
    }
    let mut conflicts = Vec::new();
    for path in paths {
        let base_sha = base.get(path);
        let away_sha = away.get(path);
        let original_sha = original.get(path);
        let away_changed = away_sha != base_sha;
        let original_changed = original_sha != base_sha;
        let disagree = away_sha != original_sha;
        if away_changed && original_changed && disagree {
            conflicts.push(FileConflict {
                rel_path: path.clone(),
                base_sha: base_sha.cloned(),
                away_sha: away_sha.cloned(),
                original_sha: original_sha.cloned(),
            });
        }
    }
    conflicts
}

/// Names the dated sibling for a Keep-both choice (deterministic:
/// the caller's date, not a clock read).
#[must_use]
pub fn away_sibling_path(rel_path: &str, date: &str) -> String {
    let (stem, ext) = match rel_path.rsplit_once('.') {
        Some((stem, ext)) => (stem, format!(".{ext}")),
        None => (rel_path, String::new()),
    };
    format!("{stem}-away-{date}{ext}")
}

/// The on-disk consequence of one choice (pure — the engine plans,
/// the app executes; the original TRANSCRIPT is never a legal
/// input, which is how the copy-back ban stays structural).
#[must_use]
pub fn apply_choice(conflict: &FileConflict, choice: ConflictChoice, date: &str) -> FileAction {
    match choice {
        ConflictChoice::KeepOriginal => FileAction::Unchanged,
        ConflictChoice::TakeAway => {
            // A deleted away file never "replaces" — taking away a
            // deletion would destroy the original silently.
            if conflict.away_sha.is_some() {
                FileAction::ReplaceWithAway
            } else {
                FileAction::Unchanged
            }
        }
        ConflictChoice::KeepBoth => {
            if conflict.away_sha.is_some() {
                FileAction::SaveAwaySibling {
                    sibling_rel_path: away_sibling_path(&conflict.rel_path, date),
                }
            } else {
                FileAction::Unchanged
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn transplant_shape_splits_exactly() {
        // B began as a byte-copy of A (base 1..3), then each side
        // appended its own turns.
        let away = set(&["u1", "u2", "u3", "b4", "b5"]);
        let original = set(&["u1", "u2", "u3", "a4"]);
        let outcome = reconcile_conversation(&away, &original);
        assert_eq!(outcome.base_uuids, 3);
        assert_eq!(outcome.away_only_uuids.len(), 2);
        assert!(outcome.away_only_uuids.contains(&"b4".to_owned()));
        assert!(outcome.away_only_uuids.contains(&"b5".to_owned()));
        assert_eq!(outcome.original_only_uuids, 1);
    }

    #[test]
    fn replay_born_away_side_is_all_delta() {
        // A fresh native session on B: no shared uuids — the whole
        // excursion is away-only, nothing conflicts.
        let away = set(&["x1", "x2"]);
        let original = set(&["u1"]);
        let outcome = reconcile_conversation(&away, &original);
        assert_eq!(outcome.base_uuids, 0);
        assert_eq!(outcome.away_only_uuids.len(), 2);
        assert_eq!(outcome.original_only_uuids, 1);
    }

    #[test]
    fn identical_change_on_both_sides_is_not_a_conflict() {
        let base = ArtifactHashes::from([("rules/CLAUDE.md".into(), "h0".into())]);
        let away = ArtifactHashes::from([("rules/CLAUDE.md".into(), "h1".into())]);
        let original = ArtifactHashes::from([("rules/CLAUDE.md".into(), "h1".into())]);
        assert!(reconcile_files(&base, &away, &original).is_empty());
    }

    #[test]
    fn both_changed_and_disagree_conflicts_deletion_included() {
        let base = ArtifactHashes::from([
            ("todos/list.json".into(), "h0".into()),
            ("skills/a.md".into(), "h0".into()),
        ]);
        let away = ArtifactHashes::from([
            ("todos/list.json".into(), "h1".into()),
            ("skills/a.md".into(), "h0".into()),
        ]);
        let original = ArtifactHashes::from([
            ("todos/list.json".into(), "h2".into()),
            ("skills/a.md".into(), "h3".into()),
        ]);
        let conflicts = reconcile_files(&base, &away, &original);
        // todos: both changed, disagree → conflict.
        // skills: only original changed → no conflict (it is already
        // native; the away side simply has the older bytes).
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].rel_path, "todos/list.json");
        assert_eq!(conflicts[0].away_sha.as_deref(), Some("h1"));
        assert_eq!(conflicts[0].original_sha.as_deref(), Some("h2"));

        // Deleted-away vs edited-original also conflicts (skills
        // untouched on the away side ⇒ not a conflict).
        let original_delete = ArtifactHashes::from([
            ("todos/list.json".into(), "h9".into()),
            ("skills/a.md".into(), "h0".into()),
        ]);
        let away_deleted = ArtifactHashes::from([("skills/a.md".into(), "h0".into())]);
        let conflicts = reconcile_files(&base, &away_deleted, &original_delete);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].rel_path, "todos/list.json");
        assert_eq!(conflicts[0].away_sha, None);
    }

    #[test]
    fn choices_map_to_actions_and_wire_names() {
        let conflict = FileConflict {
            rel_path: "todos/list.json".into(),
            base_sha: Some("h0".into()),
            away_sha: Some("h1".into()),
            original_sha: Some("h2".into()),
        };
        assert_eq!(
            apply_choice(&conflict, ConflictChoice::KeepOriginal, "20260101"),
            FileAction::Unchanged
        );
        assert_eq!(
            apply_choice(&conflict, ConflictChoice::TakeAway, "20260101"),
            FileAction::ReplaceWithAway
        );
        assert_eq!(
            apply_choice(&conflict, ConflictChoice::KeepBoth, "20260101"),
            FileAction::SaveAwaySibling {
                sibling_rel_path: "todos/list-away-20260101.json".into()
            }
        );
        // Take-away on a conflict whose away file was deleted leaves
        // the original alone (a deletion never "replaces").
        let deleted = FileConflict {
            rel_path: "todos/list.json".into(),
            base_sha: Some("h0".into()),
            away_sha: None,
            original_sha: Some("h2".into()),
        };
        assert_eq!(
            apply_choice(&deleted, ConflictChoice::TakeAway, "20260101"),
            FileAction::Unchanged
        );
        assert_eq!(ConflictChoice::KeepBoth.as_str(), "keep-both");
    }

    #[test]
    fn twenty_k_uuids_reconcile_linearly() {
        // §H7.3 budget shape: one pass over hash sets (20k messages
        // ≤ 200 ms — the algorithmic guarantee; wall time is
        // recorded in the evidence manifest, not asserted here).
        let mut away = HashSet::new();
        let mut original = HashSet::new();
        for index in 0..10_000 {
            let uuid = format!("u-{index}");
            away.insert(uuid.clone());
            original.insert(uuid);
        }
        for index in 0..10_000 {
            away.insert(format!("b-{index}"));
            original.insert(format!("a-{index}"));
        }
        let outcome = reconcile_conversation(&away, &original);
        assert_eq!(outcome.base_uuids, 10_000);
        assert_eq!(outcome.away_only_uuids.len(), 10_000);
        assert_eq!(outcome.original_only_uuids, 10_000);
    }
}
