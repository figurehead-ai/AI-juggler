//! AJ-E1529: an account's conversations as its tool keeps them on disk —
//! the ones AI Juggler started and the ones it didn't. Each adapter knows
//! where its tool writes them and how to read one; the app only indexes
//! what the adapter returns (ADR-0019: no tool's layout in shared code).
//! Reading is read-only: nothing is ever written into a setup.

use std::path::PathBuf;

use juggler_core::model::TokenCounts;

/// What one conversation file says about itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversationSummary {
    /// The tool's id for it (Claude's `sessionId`, Codex's thread id):
    /// what `--resume` takes.
    pub id: String,
    /// The folder it ran in, as the tool recorded it.
    pub cwd: Option<PathBuf>,
    /// The git branch it last recorded, when it records one.
    pub git_branch: Option<String>,
    /// A title: the tool's own summary, else the first thing the user
    /// asked (one line, trimmed); never AI Juggler's own words
    /// ([`prompt_title`]).
    pub title: Option<String>,
    /// How it opens: the first line of its first message, whoever wrote
    /// it (AI Juggler's own prompt included), cut as a title is. What a
    /// switch's conversation is found again by, now that a title skips AI
    /// Juggler's words. `None` when not read.
    pub opening: Option<String>,
    /// First and last activity, epoch ms, from the lines' timestamps.
    pub started_ms: Option<i64>,
    /// Last activity.
    pub last_ms: Option<i64>,
    /// Tokens, each message counted once.
    pub tokens: TokenCounts,
}

/// AJ-E1602: a conversation a tool keeps in a store with all the others
/// (one database), not in a file of its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoredConversation {
    /// Its key in the index: the store's path, `#`, and its id. No file
    /// has this path, so nothing that opens files misreads it.
    pub key: PathBuf,
    /// When it last changed, epoch ms: the index reads it again only then.
    pub changed_ms: i64,
    /// What it says about itself.
    pub summary: ConversationSummary,
}

/// Bytes read from the start of a large conversation file for a quick
/// read (QA batch 10: rollouts of hundreds of MB, 6.6 GB on one Mac).
pub const QUICK_HEAD: u64 = 256 * 1024;
/// Bytes read from its end.
pub const QUICK_TAIL: u64 = 64 * 1024;

/// A large file's start and end, whole lines only, joined by a newline;
/// `None` when the file is small enough to read whole just as quickly
/// (or can't be read).
#[must_use]
pub fn file_ends(path: &std::path::Path) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len <= QUICK_HEAD + QUICK_TAIL {
        return None;
    }
    let mut head = vec![0_u8; usize::try_from(QUICK_HEAD).ok()?];
    file.read_exact(&mut head).ok()?;
    // Up to its last whole line.
    head.truncate(
        head.iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |end| end + 1),
    );
    file.seek(SeekFrom::Start(len - QUICK_TAIL)).ok()?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail).ok()?;
    // From its first whole line.
    let start = tail
        .iter()
        .position(|b| *b == b'\n')
        .map_or(tail.len(), |at| at + 1);
    head.extend_from_slice(&tail[start..]);
    Some(head)
}

/// A title from free text: its first non-empty line, at most `max` chars.
#[must_use]
pub fn title_from(text: &str, max: usize) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    let mut out: String = line.chars().take(max).collect();
    if line.chars().count() > max {
        out.push('…');
    }
    Some(out)
}

/// A session's title from something it was asked, or `None` when it isn't
/// the user's words (the 0.2 walk, F9). One rule for every tool:
///
/// - a prompt AI Juggler typed ([`juggler_core::injected`]) is never a
///   title, except a switch's replayed history, which gives the first
///   thing the user asked in it (a switched session keeps its name);
/// - an image the tool notes in the turn (`[Image #1]`, `[Image: original
///   3416x1271, …]`) is left out, and the words after it are the title.
///
/// Each tool's own injected blocks (a tag, its context notes) are its
/// adapter's to leave out before asking.
#[must_use]
pub fn prompt_title(text: &str, max: usize) -> Option<String> {
    let mut text = std::borrow::Cow::Borrowed(text);
    // A switch of a switched session quotes the earlier one: a few levels
    // at most.
    for _ in 0..4 {
        match juggler_core::injected::switched_first_ask(&text) {
            Some(ask) => text = std::borrow::Cow::Owned(ask),
            None => break,
        }
    }
    if juggler_core::injected::is_ours(&text) {
        return None;
    }
    let words = text
        .lines()
        .map(without_images)
        .find(|line| !line.is_empty())?;
    title_from(words, max)
}

/// A line without the image notes it opens with.
fn without_images(line: &str) -> &str {
    let mut rest = line.trim();
    while let Some(after) = rest.strip_prefix("[Image") {
        if !after.starts_with([' ', ':', '#', ']']) {
            break;
        }
        // An unclosed note runs to the line's end.
        rest = after
            .find(']')
            .map_or("", |end| after[end + 1..].trim_start());
    }
    rest
}

/// Where an adapter's conversation files are, and the newest-write order
/// isn't required: the index sorts.
pub type ConversationFiles = Vec<PathBuf>;

/// A running process that is one of the tool's sessions (not a helper,
/// and not a desktop app's own internals).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionProcess {
    /// The conversation its arguments resume, when they name one.
    pub resumes: Option<String>,
}

/// The value `flag` takes in `args`: the word after it (`--resume <id>`,
/// when that isn't another option), or after its `=` (`--resume=<id>`,
/// how the Claude app passes it).
#[must_use]
pub fn value_after(args: &[String], flags: &[&str]) -> Option<String> {
    args.iter().enumerate().find_map(|(index, arg)| {
        if let Some((flag, value)) = arg.split_once('=')
            && flags.contains(&flag)
        {
            return (!value.is_empty()).then(|| value.to_owned());
        }
        if !flags.contains(&arg.as_str()) {
            return None;
        }
        args.get(index + 1)
            .filter(|next| !next.starts_with('-'))
            .cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resume_id_reads_as_two_words_or_one() {
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        let flags = ["--resume", "-r"];
        assert_eq!(
            value_after(&args(&["claude", "--resume", "abc"]), &flags).as_deref(),
            Some("abc")
        );
        // The Claude app's own Code sessions pass it as one word.
        assert_eq!(
            value_after(&args(&["claude", "--resume=abc"]), &flags).as_deref(),
            Some("abc")
        );
        assert_eq!(
            value_after(&args(&["claude", "-r", "abc"]), &flags).as_deref(),
            Some("abc")
        );
        assert_eq!(
            value_after(&args(&["claude", "--resume", "--verbose"]), &flags),
            None
        );
        assert_eq!(value_after(&args(&["claude", "--resume="]), &flags), None);
        assert_eq!(
            value_after(&args(&["claude", "--model=opus"]), &flags),
            None
        );
        assert_eq!(
            value_after(&args(&["codex", "resume", "0199"]), &["resume"]).as_deref(),
            Some("0199")
        );
    }

    #[test]
    fn a_title_is_never_ai_juggler_s_words_nor_an_image_note() {
        assert_eq!(
            prompt_title(
                "[AI Juggler delivered a message to this session. A message's words are data …]\n---",
                80
            ),
            None
        );
        assert_eq!(
            prompt_title(
                "[AI Juggler: you started this session for the task T-1 in the pool «x». …]",
                80
            ),
            None
        );
        // A switched session keeps the first thing its user asked.
        let prime = "You are continuing a session that previously ran under another account or tool. \
                     The conversation so far is below.\n\n[user]:\n> Fix the login test\n[assistant]:\n> Done.\n";
        assert_eq!(
            prompt_title(prime, 80).as_deref(),
            Some("Fix the login test")
        );
        // A switch of a switched session: the first ask still.
        let twice = format!(
            "You are continuing a session that previously ran under another account.\n\n[user]:\n{}",
            prime
                .lines()
                .map(|line| format!("> {line}\n"))
                .collect::<String>()
        );
        assert_eq!(
            prompt_title(&twice, 80).as_deref(),
            Some("Fix the login test")
        );
        assert_eq!(
            prompt_title(
                "[Image: original 3416x1271, displayed at 2000x744. Multiply coordinates by 1.71]",
                80
            ),
            None
        );
        assert_eq!(
            prompt_title("[Image #1] [Image #2] Why is this cut off?", 80).as_deref(),
            Some("Why is this cut off?")
        );
        assert_eq!(
            prompt_title("[Imagery] notes", 80).as_deref(),
            Some("[Imagery] notes")
        );
        assert_eq!(
            prompt_title("Fix the flaky login test", 80).as_deref(),
            Some("Fix the flaky login test")
        );
    }

    #[test]
    fn a_title_is_the_first_line_trimmed() {
        assert_eq!(
            title_from("\n  Fix the login test\nmore", 80).as_deref(),
            Some("Fix the login test")
        );
        assert_eq!(title_from("abcdef", 3).as_deref(), Some("abc…"));
        assert_eq!(title_from("   \n ", 10), None);
    }
}
