//! AJ-E1507: a project's rules files, made the same across tools.
//!
//! Each tool reads its own rules file in the project (Claude Code's
//! `CLAUDE.md`, Codex's `AGENTS.md`; the names come from the adapters'
//! [`ToolSpec::rules_files`]). They drift. This compares them with a master
//! copy the user picks and says what applying it would change, line by
//! line. Everything here only reads: the app writes a file only when the
//! user applies it (ADR-0003).

use std::time::Duration;

use juggler_adapter::ToolSpec;
use similar::{ChangeTag, TextDiff};

/// One rules file in a project, as read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesFile {
    /// Its name in the project folder (`CLAUDE.md`).
    pub name: String,
    /// The tools that read it, by id.
    pub tools: Vec<String>,
    /// Its text; `None` when the file isn't there.
    pub text: Option<String>,
    /// When it last changed (ms since the epoch); `None` when missing.
    pub modified_ms: Option<i64>,
    /// The project file it's a link to, when it is one.
    pub link_to: Option<String>,
}

/// How a rules file stands beside the master.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    /// It is the master.
    Master,
    /// The same text (line endings and blank lines at the end aside).
    Same,
    /// It reads the master through its tool's import line, so the tool
    /// sees the master's rules (and anything else the file adds).
    Reads,
    /// It's a link to the master.
    Link,
    /// Its text differs.
    Differs {
        /// Lines applying the master would add to it.
        added: usize,
        /// Lines applying the master would remove from it.
        removed: usize,
    },
    /// It isn't there.
    Missing,
}

impl Standing {
    /// Whether the file's tools see the master's rules as they are.
    #[must_use]
    pub fn in_sync(&self) -> bool {
        matches!(self, Self::Master | Self::Same | Self::Reads | Self::Link)
    }
}

/// The master to compare with when the user hasn't picked one: the file
/// changed most recently, among those with their own text (not a link and
/// not reading another rules file). `None` when there's none.
#[must_use]
pub fn default_master(files: &[RulesFile], specs: &[ToolSpec]) -> Option<String> {
    files
        .iter()
        .filter(|file| can_be_master(file, files, specs))
        .max_by_key(|file| file.modified_ms)
        .map(|file| file.name.clone())
}

/// Whether `file` can be the master: it's there, with its own text.
#[must_use]
pub fn can_be_master(file: &RulesFile, files: &[RulesFile], specs: &[ToolSpec]) -> bool {
    file.text.is_some()
        && file.link_to.is_none()
        && !files
            .iter()
            .any(|other| other.name != file.name && reads(file, &other.name, specs))
}

/// How `file` stands beside the master `master`.
#[must_use]
pub fn standing(file: &RulesFile, master: &RulesFile, specs: &[ToolSpec]) -> Standing {
    if file.name == master.name {
        return Standing::Master;
    }
    if file.link_to.as_deref() == Some(master.name.as_str()) {
        return Standing::Link;
    }
    let Some(text) = &file.text else {
        return Standing::Missing;
    };
    if reads(file, &master.name, specs) {
        return Standing::Reads;
    }
    let master_text = master.text.as_deref().unwrap_or_default();
    if normalized(text) == normalized(master_text) {
        return Standing::Same;
    }
    let (added, removed) = counts(text, master_text);
    Standing::Differs { added, removed }
}

/// Whether every tool reading `file` reads `other` through it.
fn reads(file: &RulesFile, other: &str, specs: &[ToolSpec]) -> bool {
    let Some(text) = &file.text else {
        return false;
    };
    !file.tools.is_empty()
        && file.tools.iter().all(|tool| {
            specs
                .iter()
                .find(|spec| &spec.id == tool)
                .is_some_and(|spec| spec.rules_reads(text, other))
        })
}

/// The rules file `file` reads through its tool's import line that isn't
/// there, if any: its tools then see none of those rules.
#[must_use]
pub fn reads_missing(file: &RulesFile, files: &[RulesFile], specs: &[ToolSpec]) -> Option<String> {
    files
        .iter()
        .find(|other| {
            other.name != file.name && other.text.is_none() && reads(file, &other.name, specs)
        })
        .map(|other| other.name.clone())
}

/// The line that would make `file` read `master` instead of repeating
/// it, when every tool reading `file` has one and they agree.
#[must_use]
pub fn import_line(file: &RulesFile, master: &str, specs: &[ToolSpec]) -> Option<String> {
    let mut lines = file.tools.iter().map(|tool| {
        specs
            .iter()
            .find(|spec| &spec.id == tool)
            .and_then(|spec| spec.rules_import_line(master))
    });
    let first = lines.next()??;
    lines
        .all(|line| line.as_deref() == Some(first.as_str()))
        .then_some(first)
}

/// Text as compared: `\r\n` as `\n`, without blank lines at the end.
fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end().to_owned()
}

/// The longest a diff may search before settling for a coarser one.
const DIFF_DEADLINE: Duration = Duration::from_millis(80);

fn text_diff<'a>(old: &'a str, new: &'a str) -> TextDiff<'a, 'a, str> {
    TextDiff::configure()
        .timeout(DIFF_DEADLINE)
        .diff_lines(old, new)
}

/// Lines added and removed going from `old` to `new`.
fn counts(old: &str, new: &str) -> (usize, usize) {
    let (old, new) = (normalized(old), normalized(new));
    let diff = text_diff(&old, &new);
    diff.iter_all_changes()
        .fold((0, 0), |(added, removed), change| match change.tag() {
            ChangeTag::Insert => (added + 1, removed),
            ChangeTag::Delete => (added, removed + 1),
            ChangeTag::Equal => (added, removed),
        })
}

/// One line of a diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    /// What applying does to it.
    pub change: LineChange,
    /// Its text, without the line ending.
    pub text: String,
    /// Its number in the file now; `None` for an added line.
    pub old_number: Option<usize>,
    /// Its number after applying; `None` for a removed line.
    pub new_number: Option<usize>,
}

/// What applying does to a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineChange {
    /// Kept, shown around a change.
    Kept,
    /// Added.
    Added,
    /// Removed.
    Removed,
}

/// The lines kept around each change.
pub const CONTEXT_LINES: usize = 3;

/// What applying `new` over `old` changes, as groups of lines: each change
/// with [`CONTEXT_LINES`] kept lines around it. Empty when nothing changes.
#[must_use]
pub fn diff(old: &str, new: &str) -> Vec<Vec<DiffLine>> {
    let (old, new) = (normalized(old), normalized(new));
    let diff = text_diff(&old, &new);
    diff.grouped_ops(CONTEXT_LINES)
        .iter()
        .map(|group| {
            group
                .iter()
                .flat_map(|op| diff.iter_changes(op))
                .map(|change| DiffLine {
                    change: match change.tag() {
                        ChangeTag::Equal => LineChange::Kept,
                        ChangeTag::Insert => LineChange::Added,
                        ChangeTag::Delete => LineChange::Removed,
                    },
                    text: change.value().trim_end_matches(['\n', '\r']).to_owned(),
                    old_number: change.old_index().map(|index| index + 1),
                    new_number: change.new_index().map(|index| index + 1),
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, files: &[&str], import: Option<&str>) -> ToolSpec {
        let mut spec = ToolSpec::generic(id);
        spec.rules_files = files.iter().map(|file| (*file).to_owned()).collect();
        spec.rules_import = import.map(str::to_owned);
        spec
    }

    fn specs() -> Vec<ToolSpec> {
        vec![
            spec("importer", &["IMPORTER.md"], Some("@{file}")),
            spec("plain", &["PLAIN.md"], None),
        ]
    }

    fn file(name: &str, tool: &str, text: Option<&str>, modified: i64) -> RulesFile {
        RulesFile {
            name: name.to_owned(),
            tools: vec![tool.to_owned()],
            text: text.map(str::to_owned),
            modified_ms: text.map(|_| modified),
            link_to: None,
        }
    }

    #[test]
    fn the_same_text_is_in_sync_whatever_its_line_endings() {
        let master = file("PLAIN.md", "plain", Some("# Rules\n\nUse tabs.\n"), 2);
        let other = file(
            "IMPORTER.md",
            "importer",
            Some("# Rules\r\n\r\nUse tabs.\r\n\r\n"),
            1,
        );
        assert_eq!(standing(&other, &master, &specs()), Standing::Same);
        assert!(standing(&master, &master, &specs()).in_sync());
    }

    #[test]
    fn a_file_that_reads_the_master_is_in_sync_and_cant_be_the_master() {
        let master = file("PLAIN.md", "plain", Some("# Rules\n"), 1);
        let reader = file(
            "IMPORTER.md",
            "importer",
            Some("@PLAIN.md\n\nClaude only: be brief.\n"),
            2,
        );
        let files = vec![master.clone(), reader.clone()];
        assert_eq!(standing(&reader, &master, &specs()), Standing::Reads);
        assert!(!can_be_master(&reader, &files, &specs()));
        assert_eq!(
            default_master(&files, &specs()).as_deref(),
            Some("PLAIN.md"),
            "newer, but it reads the other"
        );
    }

    #[test]
    fn a_tool_without_an_import_line_never_reads_another_file() {
        let master = file("IMPORTER.md", "importer", Some("# Rules\n"), 1);
        let plain = file("PLAIN.md", "plain", Some("@IMPORTER.md\n"), 2);
        assert_eq!(
            standing(&plain, &master, &specs()),
            Standing::Differs {
                added: 1,
                removed: 1
            }
        );
        assert_eq!(import_line(&plain, "IMPORTER.md", &specs()), None);
        let importer = file("IMPORTER.md", "importer", Some("x"), 1);
        assert_eq!(
            import_line(&importer, "PLAIN.md", &specs()).as_deref(),
            Some("@PLAIN.md")
        );
    }

    #[test]
    fn a_link_and_a_missing_file_are_told_apart() {
        let master = file("PLAIN.md", "plain", Some("# Rules\n"), 1);
        let mut link = file("IMPORTER.md", "importer", Some("# Rules\n"), 1);
        link.link_to = Some("PLAIN.md".to_owned());
        assert_eq!(standing(&link, &master, &specs()), Standing::Link);
        let missing = file("IMPORTER.md", "importer", None, 0);
        assert_eq!(standing(&missing, &master, &specs()), Standing::Missing);
        assert!(!Standing::Missing.in_sync());
    }

    #[test]
    fn a_file_reading_one_that_isnt_there_is_told_apart() {
        let files = vec![
            file("IMPORTER.md", "importer", Some("@PLAIN.md\n"), 2),
            file("PLAIN.md", "plain", None, 0),
        ];
        assert_eq!(
            reads_missing(&files[0], &files, &specs()).as_deref(),
            Some("PLAIN.md")
        );
        let there = vec![
            files[0].clone(),
            file("PLAIN.md", "plain", Some("# Rules\n"), 1),
        ];
        assert_eq!(reads_missing(&there[0], &there, &specs()), None);
    }

    #[test]
    fn the_default_master_is_the_newest_file_with_its_own_text() {
        let files = vec![
            file("PLAIN.md", "plain", Some("old"), 1),
            file("IMPORTER.md", "importer", Some("new"), 5),
            file("OTHER.md", "plain", None, 0),
        ];
        assert_eq!(
            default_master(&files, &specs()).as_deref(),
            Some("IMPORTER.md")
        );
        assert_eq!(default_master(&[], &specs()), None);
    }

    #[test]
    fn the_diff_groups_each_change_with_the_lines_around_it() {
        let old: String = (1..=20).map(|n| format!("line {n}\n")).collect();
        let new = old
            .replace("line 3\n", "line three\n")
            .replace("line 18\n", "");
        let groups = diff(&old, &new);
        assert_eq!(groups.len(), 2, "two changes far apart");
        let first = &groups[0];
        assert_eq!(first.first().unwrap().text, "line 1");
        assert!(first.contains(&DiffLine {
            change: LineChange::Removed,
            text: "line 3".into(),
            old_number: Some(3),
            new_number: None,
        }));
        assert!(
            first
                .iter()
                .any(|line| line.change == LineChange::Added && line.text == "line three")
        );
        assert!(
            groups[1]
                .iter()
                .any(|line| line.change == LineChange::Removed && line.text == "line 18")
        );
        assert!(diff(&old, &old).is_empty());
    }

    #[test]
    fn two_thousand_line_files_diff_inside_the_budget() {
        // AJ-E1507's budget: the diff in under 100 ms for 2,000-line files.
        let old: String = (0..2000)
            .map(|n| format!("rule {n}: keep it short\n"))
            .collect();
        let new: String = (0..2000)
            .map(|n| {
                if n % 7 == 0 {
                    format!("rule {n}: changed\n")
                } else {
                    format!("rule {n}: keep it short\n")
                }
            })
            .collect();
        // This thread's CPU time, the best of five: a busy Mac stretches
        // the wall clock, not the work.
        let mut best = Duration::MAX;
        for _ in 0..5 {
            let started = cpu_time::ThreadTime::now();
            assert!(!diff(&old, &new).is_empty());
            best = best.min(started.elapsed());
        }
        assert!(best < Duration::from_millis(100), "{best:?} CPU");
    }
}
