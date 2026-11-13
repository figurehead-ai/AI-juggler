//! How a session is named when nothing named it: a git branch only when
//! it says what the work was ("fix/limits"), never one that says nothing
//! ("main"). One rule for every place a name is made (the window's
//! `tellingBranch` holds the same list). A title that is what was asked
//! is named by its first words ([`first_words`], the window's
//! `firstWords`). The window's `shortName` is the same rule, held to it
//! by one file of cases both run (`ui/src/lib/fixtures/session-names.json`).

/// Branches that name no piece of work.
const PLAIN: [&str; 8] = [
    "main",
    "master",
    "trunk",
    "develop",
    "development",
    "dev",
    "head",
    "default",
];

/// `branch` when it says what the work was; `None` for an empty one or
/// one that names no work.
#[must_use]
pub fn telling_branch(branch: Option<&str>) -> Option<&str> {
    branch.filter(|branch| {
        let lower = branch.trim().to_lowercase();
        !lower.is_empty() && !PLAIN.contains(&lower.as_str())
    })
}

/// What an untitled session is told apart by (D-96, persona D2: two
/// sessions in one folder both read "Untitled session · billing"): its
/// folder and when it started, in the user's words ("started 1:14 AM",
/// `juggler_platform::clock::started_words`). Never its App or Account
/// (D-103.8): what shows the name says that beside it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Facts<'a> {
    /// The last part of its working folder (or its project's name).
    pub folder: Option<&'a str>,
    /// When it started, in words.
    pub started: Option<&'a str>,
}

/// An untitled session's name by its facts (the window's `untitledName`):
/// "billing · started 1:14 AM", never its Account (D-103.8, persona R2:
/// "client-b · Client A · client-a · started 4:46 PM"); "Session started
/// 1:14 AM" when only the time is known; "billing" when only the folder
/// is; "Untitled session" when nothing is.
#[must_use]
pub fn untitled_name(facts: &Facts<'_>) -> String {
    fn some(part: Option<&str>) -> Option<&str> {
        part.map(str::trim).filter(|part| !part.is_empty())
    }
    match (some(facts.folder), some(facts.started)) {
        (None, None) => "Untitled session".to_owned(),
        (None, Some(started)) => format!("Session {started}"),
        (Some(folder), None) => folder.to_owned(),
        (Some(folder), Some(started)) => format!("{folder} · {started}"),
    }
}

/// A session's short name, the one rule wherever one is said (a pool's
/// cards and messages, Needs you's notices; the window's `shortName`):
/// its label, else its title's first words, else a branch that says what
/// the work was, else its facts ("billing · started 1:14 AM", D-96,
/// D-103.8) — never the whole first prompt, never "main" (the rc7, rc12 and
/// rc15 walks), never `Untitled session · <folder>` (persona D2).
#[must_use]
pub fn short_name(
    label: Option<&str>,
    title: Option<&str>,
    branch: Option<&str>,
    facts: &Facts<'_>,
) -> String {
    if let Some(label) = label.filter(|label| !label.trim().is_empty()) {
        return label.to_owned();
    }
    if let Some(title) = title.filter(|title| !title.trim().is_empty()) {
        return first_words(title, 8).unwrap_or_else(|| title.to_owned());
    }
    if let Some(branch) = telling_branch(branch) {
        return branch.to_owned();
    }
    untitled_name(facts)
}

/// Whether a session is named by its facts: no label, no title, no
/// branch that says what the work was (its name says when it started).
#[must_use]
pub fn named_by_facts(label: Option<&str>, title: Option<&str>, branch: Option<&str>) -> bool {
    let empty = |part: Option<&str>| part.is_none_or(|part| part.trim().is_empty());
    empty(label) && empty(title) && telling_branch(branch).is_none()
}

/// The most characters [`first_words`] keeps.
const FIRST_WORDS_CHARS: usize = 56;

/// The first few words of what was asked: its first line, cut before a
/// redaction mark, its first clause past a short opener ("Good.", "OK,
/// thanks."), at most `max` words or 56 characters, "…" when cut short;
/// `None` when nothing is left (rc7 walk: pool members and senders were
/// named by the whole first prompt). The window's `firstWords` is the same.
#[must_use]
pub fn first_words(text: &str, max: usize) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    let kept = cut_at_redaction(line);
    let clean: String = kept
        .chars()
        .filter(|c| !matches!(c, '`' | '*' | '_' | '#' | '>'))
        .collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut clauses = clauses_of(&clean);
    while clauses.len() > 1 && clauses[0].split(' ').count() < 3 {
        clauses.remove(0);
    }
    let clause = clauses
        .first()
        .copied()
        .unwrap_or("")
        .trim_end_matches(|c: char| {
            c.is_whitespace() || matches!(c, ',' | ';' | ':' | '–' | '—' | '-')
        });
    if clause.chars().count() < 3 {
        return None;
    }
    let mut out = clause.split(' ').take(max).collect::<Vec<_>>().join(" ");
    if out.chars().count() > FIRST_WORDS_CHARS {
        let head: String = out.chars().take(FIRST_WORDS_CHARS).collect();
        // Back to the last whole word (as `\s+\S*$` cuts it).
        out = match head.rfind(char::is_whitespace) {
            Some(at) => head[..at].trim_end().to_owned(),
            None => head,
        };
    }
    let whole = clause.trim_end_matches(['.', '!', '?']).chars().count();
    let cut = out.chars().count() < whole;
    let out = out.trim_end_matches(['.', ',', ';', ':', '!']);
    Some(if cut {
        format!("{out}…")
    } else {
        out.to_owned()
    })
}

/// `line` up to a redaction mark (`[redacted]`, or two or more `•`).
fn cut_at_redaction(line: &str) -> &str {
    let lower = line.to_lowercase();
    let mut end = line.len();
    if lower.len() == line.len()
        && let Some(at) = lower.find("[redacted]")
    {
        end = end.min(at);
    }
    if let Some(at) = line.find("••") {
        end = end.min(at);
    }
    &line[..end]
}

/// `text` split after each `.;:!?` that a space follows.
fn clauses_of(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if matches!(c, '.' | ';' | ':' | '!' | '?')
            && chars.peek().is_some_and(|&(_, next)| next == ' ')
        {
            let end = at + c.len_utf8();
            out.push(&text[start..end]);
            start = end + 1;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out.into_iter().filter(|part| !part.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::{Facts, first_words, named_by_facts, short_name, telling_branch, untitled_name};

    const WEB: Facts<'static> = Facts {
        folder: Some("web"),
        started: Some("started 1:14 AM"),
    };

    /// One case of `ui/src/lib/fixtures/session-names.json`.
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        label: Option<String>,
        title: Option<String>,
        branch: Option<String>,
        folder: Option<String>,
        started: bool,
        name: String,
        by_facts: bool,
    }

    #[derive(serde::Deserialize)]
    struct Cases {
        cases: Vec<Case>,
    }

    /// D-103.8: the window names a session by the same rule, from the same
    /// cases (`session-titles.test.ts`).
    #[test]
    fn the_backend_and_the_window_name_a_session_the_same_way() {
        let cases: Cases = serde_json::from_str(include_str!(
            "../../../ui/src/lib/fixtures/session-names.json"
        ))
        .expect("the shared naming cases parse");
        assert!(cases.cases.len() >= 10);
        for case in &cases.cases {
            let facts = Facts {
                folder: case.folder.as_deref(),
                started: case.started.then_some("started 1:14 AM"),
            };
            let (label, title, branch) = (
                case.label.as_deref(),
                case.title.as_deref(),
                case.branch.as_deref(),
            );
            assert_eq!(
                short_name(label, title, branch, &facts),
                case.name.replace("{started}", "started 1:14 AM"),
                "{label:?} {title:?} {branch:?} {:?}",
                case.folder
            );
            assert_eq!(named_by_facts(label, title, branch), case.by_facts);
        }
    }

    #[test]
    fn a_session_is_named_by_its_label_its_first_words_a_telling_branch_or_its_facts() {
        assert_eq!(
            short_name(Some("Writer"), Some("x"), Some("main"), &WEB),
            "Writer"
        );
        assert_eq!(
            short_name(None, None, Some("fix/limits"), &WEB),
            "fix/limits"
        );
        // D-96: by its facts, never "Untitled session · web"; D-103.8:
        // never its Account.
        assert_eq!(
            short_name(None, None, Some("main"), &WEB),
            "web · started 1:14 AM"
        );
        assert_eq!(
            short_name(None, Some("  "), None, &Facts::default()),
            "Untitled session"
        );
    }

    #[test]
    fn an_untitled_session_says_its_folder_and_when_it_started() {
        let two = Facts {
            started: Some("started 1:15 AM"),
            ..WEB
        };
        // Two in one folder read apart by their minute.
        assert_ne!(untitled_name(&WEB), untitled_name(&two));
        assert_eq!(untitled_name(&WEB), "web · started 1:14 AM");
        assert_eq!(
            untitled_name(&Facts {
                folder: Some(" "),
                ..WEB
            }),
            "Session started 1:14 AM"
        );
        assert_eq!(
            untitled_name(&Facts {
                started: None,
                ..WEB
            }),
            "web"
        );
        assert!(named_by_facts(None, Some(" "), Some("main")));
        assert!(!named_by_facts(None, None, Some("fix/limits")));
        assert!(!named_by_facts(Some("Writer"), None, None));
    }

    #[test]
    fn a_long_first_prompt_names_a_session_by_its_first_words() {
        assert_eq!(
            first_words(
                "Good. Write the release notes for 0.4.1 and check every link in them twice",
                8
            )
            .as_deref(),
            Some("Write the release notes for 0.4.1 and check…")
        );
        assert_eq!(
            first_words("Fix the login test.", 8).as_deref(),
            Some("Fix the login test")
        );
        assert_eq!(
            first_words("\n  **Tidy** the `docs` folder\nmore", 8).as_deref(),
            Some("Tidy the docs folder")
        );
        assert_eq!(
            first_words("Use key [redacted] now", 8).as_deref(),
            Some("Use key")
        );
        assert_eq!(first_words("ok", 8), None);
        assert_eq!(first_words("   ", 8), None);
        let long = first_words("Investigateeverything thoroughly regardingtheextraordinarilylongwordsthatkeepgoing andgoing", 8).unwrap();
        assert!(long.chars().count() <= 57, "{long}");
        assert!(long.ends_with('…'));
    }

    #[test]
    fn a_branch_names_a_session_only_when_it_says_what_the_work_was() {
        assert_eq!(telling_branch(Some("fix/limits")), Some("fix/limits"));
        for plain in [
            "main",
            "Master",
            "trunk",
            "develop",
            "development",
            "dev",
            "HEAD",
            "default",
            "",
            "  ",
        ] {
            assert_eq!(telling_branch(Some(plain)), None, "{plain}");
        }
        assert_eq!(telling_branch(None), None);
    }
}
