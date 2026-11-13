//! How a session is named when nothing named it: a git branch only when
//! it says what the work was ("fix/limits"), never one that says nothing
//! ("main"). One rule for every place a name is made (the window's
//! `tellingBranch` holds the same list). A title that is what was asked
//! (its first prompt, 0.4.5) is named by its first words ([`first_words`],
//! the window's `firstWords`), never a pasted key or a bare path. The
//! window's `shortName` is the same rule, held to it by one file of cases
//! both run (`ui/src/lib/fixtures/session-names.json`).

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
/// is; "Untitled session" when nothing is. Only a session with no first
/// prompt is named so (0.4.5): one that has one is named by its words.
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
/// cards and messages, Needs you's notices, the Floor; the window's
/// `shortName`): its label, else its title's words (the tool's own name
/// for it, else the first words of its first prompt, as `claude --resume`
/// and Codex's picker list it: 0.4.5), else a branch that says what the
/// work was, else its facts ("billing · started 1:14 AM", D-96, D-103.8)
/// — never the whole first prompt, never "main" (the rc7, rc12 and rc15
/// walks), never `Untitled session · <folder>` (persona D2), never a
/// pasted key or a bare path.
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
    if let Some(words) = title.and_then(title_words) {
        return words;
    }
    if let Some(branch) = telling_branch(branch) {
        return branch.to_owned();
    }
    untitled_name(facts)
}

/// Whether a session is named by its facts: no label, no title with words
/// to say, no branch that says what the work was (its name says when it
/// started).
#[must_use]
pub fn named_by_facts(label: Option<&str>, title: Option<&str>, branch: Option<&str>) -> bool {
    let empty = |part: Option<&str>| part.is_none_or(|part| part.trim().is_empty());
    empty(label) && title.and_then(title_words).is_none() && telling_branch(branch).is_none()
}

/// What a title says as a name: its first words ([`first_words`]); a
/// short one ("ok") as it is ([`said_line`]); `None` when it says nothing
/// but a path or a key (a first prompt that was only those: its facts
/// name it).
fn title_words(title: &str) -> Option<String> {
    first_words(title).or_else(|| said_line(title))
}

/// What a prompt says, as one line: its first line that says something
/// (not only paths or pasted keys), cut before a redaction mark or a word
/// that looks like a key, whitespace collapsed; `None` when nothing is
/// left. What a conversation's title is made of when it's what was asked
/// (`juggler_adapter::conversation::prompt_title`), so no list shows a
/// pasted key or a bare path as a title.
#[must_use]
pub fn said_line(text: &str) -> Option<String> {
    let line = spoken_line(text)?;
    let kept = before_secret(cut_at_redaction(line))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!kept.is_empty()).then_some(kept)
}

/// The most characters [`first_words`] keeps (0.4.5: about 60, cut at a
/// word).
const NAME_CHARS: usize = 60;

/// The first words of what was asked, one line: its first line that says
/// something (a line of only paths or pasted keys says nothing:
/// `says_nothing`), whitespace collapsed, cut before a redaction mark or
/// a word that looks like a key (`token_like`), its first clause past a
/// short opener ("Good.", "OK, thanks."), at most 60 characters cut at a
/// word, "…" when cut short; `None` when nothing is left (rc7 walk: pool
/// members and senders were named by the whole first prompt). A colon
/// doesn't end a clause: a tool's own title ("Billing move: rehearsal")
/// reads whole. The window's `firstWords` is the same.
#[must_use]
pub fn first_words(text: &str) -> Option<String> {
    let line = spoken_line(text)?;
    let kept = before_secret(cut_at_redaction(line));
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
    let mut out = clause.to_owned();
    if out.chars().count() > NAME_CHARS {
        let head: String = out.chars().take(NAME_CHARS).collect();
        let ends_a_word = out.chars().nth(NAME_CHARS).is_some_and(char::is_whitespace);
        // Back to the last whole word (as `\s+\S*$` cuts it); a word that
        // ends right at the limit stays.
        out = match head.rfind(char::is_whitespace) {
            _ if ends_a_word => head.trim_end().to_owned(),
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

/// `text`'s first line that says something: not empty, and not only
/// paths or pasted keys ([`says_nothing`]).
fn spoken_line(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !says_nothing(line))
}

/// Whether a line is only paths or pasted keys (an `@"/Users/…/brief.md"`
/// mention, a dragged file, a pasted token): nothing to name a session
/// by, and nothing a name should show. Markdown marks around them
/// (backticks, quotes, `@`) don't count.
fn says_nothing(line: &str) -> bool {
    // A shell's escaped space is part of its path.
    let line = line.replace("\\ ", "_");
    let mut words = line
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| {
                matches!(
                    c,
                    '@' | '"' | '\'' | '`' | '*' | '<' | '>' | '(' | ')' | '[' | ']' | ',' | ';'
                )
            })
        })
        .filter(|word| !word.is_empty())
        .peekable();
    words.peek().is_some() && words.all(|word| path_like(word) || token_like(word))
}

/// Whether `word` is a path: it has a `/` or a backslash and isn't a web
/// address (`https://…` names a page, not a file on this Mac).
fn path_like(word: &str) -> bool {
    let separated = word.contains(['/', '\\']) && word.chars().any(char::is_alphanumeric);
    separated && (!word.contains("://") || word.starts_with("file://"))
}

/// The shortest run of letters and digits, mixed, that reads as a key
/// rather than a word.
const TOKEN_RUN: usize = 16;

/// Whether `word` looks like a pasted key or token (`sk-ant-api03-…`, a
/// hash, a base64 blob): a run of at least 16 ASCII letters and digits
/// that has both. Long plain words ("extraordinarily") have no digit.
fn token_like(word: &str) -> bool {
    let (mut run, mut letter, mut digit) = (0, false, false);
    for c in word.chars() {
        if c.is_ascii_alphanumeric() {
            run += 1;
            letter |= c.is_ascii_alphabetic();
            digit |= c.is_ascii_digit();
            if run >= TOKEN_RUN && letter && digit {
                return true;
            }
        } else {
            (run, letter, digit) = (0, false, false);
        }
    }
    false
}

/// `line` up to its first word that looks like a key ([`token_like`]).
fn before_secret(line: &str) -> &str {
    let mut at = 0;
    for word in line.split_inclusive(char::is_whitespace) {
        if token_like(word) {
            return &line[..at];
        }
        at += word.len();
    }
    line
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

/// `text` split after each `.;!?` that a space follows (not `:`: "Billing
/// move: rehearsal" is one clause).
fn clauses_of(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if matches!(c, '.' | ';' | '!' | '?') && chars.peek().is_some_and(|&(_, next)| next == ' ')
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
    use super::{
        Facts, first_words, named_by_facts, said_line, short_name, telling_branch, untitled_name,
    };

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
        // 0.4.5: about 60 characters, cut at a word.
        assert_eq!(
            first_words(
                "Good. Write the release notes for 0.4.1 and check every link in them twice"
            )
            .as_deref(),
            Some("Write the release notes for 0.4.1 and check every link in…")
        );
        assert_eq!(
            first_words("Fix the login test.").as_deref(),
            Some("Fix the login test")
        );
        assert_eq!(
            first_words("\n  **Tidy** the `docs` folder\nmore").as_deref(),
            Some("Tidy the docs folder")
        );
        assert_eq!(
            first_words("Use key [redacted] now").as_deref(),
            Some("Use key")
        );
        assert_eq!(first_words("ok"), None);
        assert_eq!(first_words("   "), None);
        let long = first_words("Investigateeverything thoroughly regardingtheextraordinarilylongwordsthatkeepgoing andgoing").unwrap();
        assert!(long.chars().count() <= 61, "{long}");
        assert!(long.ends_with('…'));
        // A colon doesn't end it: a tool's own title reads whole.
        assert_eq!(
            first_words("Billing move: rehearsal starts (M2)").as_deref(),
            Some("Billing move: rehearsal starts (M2)")
        );
    }

    /// 0.4.5: a name never shows a pasted key or a bare path; a line of
    /// only those says nothing, and the next line speaks.
    #[test]
    fn a_name_never_shows_a_key_or_a_bare_path() {
        let key = "sk-test-0123456789abcdefXYZ";
        for nothing in [
            key,
            "/Users/you/Downloads/brief.md",
            "~/Desktop/Screenshot\\ 2026-11-13\\ at\\ 09.41.png",
            "@\"/Users/you/My Documents/brief.md\"",
            "`src/lib/naming.rs`",
            "C:\\Users\\you\\notes.txt",
            "3f786850e387550fdab836ed7e6dc881de23001b",
        ] {
            assert_eq!(first_words(nothing), None, "{nothing}");
            assert_eq!(said_line(nothing), None, "{nothing}");
            assert_eq!(
                short_name(None, Some(nothing), None, &WEB),
                "web · started 1:14 AM",
                "{nothing}"
            );
            assert!(named_by_facts(None, Some(nothing), None), "{nothing}");
        }
        assert_eq!(
            first_words(&format!("Call the API with {key} and list the models")).as_deref(),
            Some("Call the API with")
        );
        assert_eq!(
            first_words(
                "@\"/Users/you/Downloads/brief.md\"\nOk so we are planning to move the shop"
            )
            .as_deref(),
            Some("Ok so we are planning to move the shop")
        );
        // Not keys: long plain words, ids with dashes, web addresses, versions.
        for words in [
            "Read https://example.com/docs/2026/limits first",
            "Rename 0b1c2d3e-4f5a-4b6c-8d7e-9f0a1b2c3d4e in the store",
            "Ship 0.4.5 to beta",
        ] {
            assert_eq!(first_words(words).as_deref(), Some(words), "{words}");
        }
        assert_eq!(
            said_line("  Fix   the\tlogin\n").as_deref(),
            Some("Fix the login")
        );
        assert_eq!(short_name(None, Some("ok"), None, &WEB), "ok");
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
