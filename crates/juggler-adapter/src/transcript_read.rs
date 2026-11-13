//! 0.4.5 (AJ-E805): a conversation file read as it grows: the one reader
//! AI Juggler's conversation list and its usage reading both take from.
//!
//! A tool whose conversations are append-only logs (Claude Code's
//! transcripts, Codex's rollouts) gives a [`TranscriptReader`] per file
//! ([`crate::ToolTranscripts::transcript_reader`]). AI Juggler hands it the
//! lines written since its last read; it says what they hold for usage, and
//! what the file says about itself so far. Before, the conversation list
//! read every changed transcript whole again for its tokens on each pass
//! while the usage reading tailed the same file, so a working session cost
//! more the longer it ran.
//!
//! A conversation's tokens are its usage, counted by the store's own rule
//! ([`TokenTally`]): each response once, the larger figure of each class.

use std::collections::{HashSet, VecDeque};
use std::path::Path;

use juggler_core::model::TokenCounts;
use juggler_core::quota::{QuotaSample, UsageEvent, WindowKind};

use crate::conversation::ConversationSummary;

/// Codex's credits, as its own rollout reports them (no value is read
/// anywhere else; `auth.json` is never opened).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreditsSnapshot {
    /// When Codex wrote it, epoch ms.
    pub ts: i64,
    /// The plan has credits at all.
    pub has_credits: bool,
    /// Credits are unlimited.
    pub unlimited: bool,
    /// The balance as Codex wrote it (a string in its protocol).
    pub balance: Option<String>,
}

/// What lines of a transcript said for usage.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageRead {
    /// Each response's usage, keyed for the store (`msg_uuid`).
    pub events: Vec<UsageEvent>,
    /// The tool's own limit readings among the lines.
    pub samples: Vec<(WindowKind, QuotaSample)>,
    /// The newest credits snapshot among the lines, when any.
    pub credits: Option<CreditsSnapshot>,
    /// Lines that couldn't be read.
    pub parse_errors: u64,
    /// Usage dropped before it could be counted (Codex: compactions past
    /// their waiting cap).
    pub dropped: u64,
    /// Why the file wasn't read, when it wasn't (counted in the source's
    /// health with this note).
    pub unread: Option<&'static str>,
}

impl UsageRead {
    /// `next`, read after these lines, added to them.
    pub fn append(&mut self, next: Self) {
        self.events.extend(next.events);
        self.samples.extend(next.samples);
        if let Some(credits) = next.credits
            && self
                .credits
                .as_ref()
                .is_none_or(|kept| kept.ts <= credits.ts)
        {
            self.credits = Some(credits);
        }
        self.parse_errors += next.parse_errors;
        self.dropped += next.dropped;
        self.unread = self.unread.or(next.unread);
    }

    /// Nothing was read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// One conversation file, read as it grows (see the module docs).
pub trait TranscriptReader: Send {
    /// The lines written since the last call, whole lines only (each
    /// through its newline), in file order: what they hold for usage.
    fn read(&mut self, lines: &[u8]) -> UsageRead;
    /// What the lines so far say about the conversation (its tokens are
    /// [`ConversationFile`]'s to add); `None` while they don't make one (an
    /// empty file, a subagent's own log).
    fn summary(&self, path: &Path) -> Option<ConversationSummary>;
}

/// The responses whose figures a tally keeps for a repeat of one.
const RECENT: usize = 16;

/// A file's tokens by the store's rule (`UsageRepo::store_events`): each
/// response once, by its key, with the larger figure of each class.
///
/// A response's lines come together: Claude Code writes one line per
/// content block, each with the response's usage, and a slice can end
/// between them. The last [`RECENT`] responses keep their figures, so a
/// later line of one raises its count; any other key seen before is a copy
/// (a fork's or a resumed rollout's repeated history) and adds nothing.
/// Measured on one real profile (1,198 transcripts, 16.5 GB): 234,539
/// responses, none came back after another response's line. Memory: a
/// 64-bit hash per response.
#[derive(Debug, Default, Clone)]
pub struct TokenTally {
    total: TokenCounts,
    seen: HashSet<u64>,
    recent: VecDeque<(u64, TokenCounts)>,
}

impl TokenTally {
    /// One response's usage (`key`: its `msg_uuid`; `None` counts it as
    /// it is, there's nothing to know it again by).
    pub fn add(&mut self, key: Option<&str>, tokens: TokenCounts) {
        let Some(key) = key else {
            plus(&mut self.total, &tokens);
            return;
        };
        let hash = {
            use std::hash::{Hash as _, Hasher as _};
            let mut hasher = std::hash::DefaultHasher::new();
            key.hash(&mut hasher);
            hasher.finish()
        };
        if let Some((_, kept)) = self.recent.iter_mut().find(|(seen, _)| *seen == hash) {
            let larger = larger_of(kept, &tokens);
            minus(&mut self.total, kept);
            plus(&mut self.total, &larger);
            *kept = larger;
            return;
        }
        if !self.seen.insert(hash) {
            return;
        }
        plus(&mut self.total, &tokens);
        self.recent.push_back((hash, tokens));
        if self.recent.len() > RECENT {
            self.recent.pop_front();
        }
    }

    /// The tokens so far.
    #[must_use]
    pub fn total(&self) -> TokenCounts {
        self.total
    }
}

fn plus(total: &mut TokenCounts, add: &TokenCounts) {
    total.input += add.input;
    total.output += add.output;
    total.cache_read += add.cache_read;
    total.cache_write += add.cache_write;
    total.reasoning += add.reasoning;
}

fn minus(total: &mut TokenCounts, less: &TokenCounts) {
    total.input = total.input.saturating_sub(less.input);
    total.output = total.output.saturating_sub(less.output);
    total.cache_read = total.cache_read.saturating_sub(less.cache_read);
    total.cache_write = total.cache_write.saturating_sub(less.cache_write);
    total.reasoning = total.reasoning.saturating_sub(less.reasoning);
}

fn larger_of(a: &TokenCounts, b: &TokenCounts) -> TokenCounts {
    TokenCounts {
        input: a.input.max(b.input),
        output: a.output.max(b.output),
        cache_read: a.cache_read.max(b.cache_read),
        cache_write: a.cache_write.max(b.cache_write),
        reasoning: a.reasoning.max(b.reasoning),
    }
}

/// A conversation file's reader with its tokens: what the file says about
/// itself, its tokens counted from the usage it read.
pub struct ConversationFile {
    reader: Box<dyn TranscriptReader>,
    tally: TokenTally,
}

impl std::fmt::Debug for ConversationFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConversationFile")
            .field("tally", &self.tally)
            .finish_non_exhaustive()
    }
}

impl ConversationFile {
    /// A file not read yet.
    #[must_use]
    pub fn new(reader: Box<dyn TranscriptReader>) -> Self {
        Self {
            reader,
            tally: TokenTally::default(),
        }
    }

    /// The lines written since the last read (see
    /// [`TranscriptReader::read`]); their usage is counted, and returned.
    pub fn read(&mut self, lines: &[u8]) -> UsageRead {
        let read = self.reader.read(lines);
        for event in &read.events {
            self.tally.add(event.msg_uuid.as_deref(), event.tokens);
        }
        read
    }

    /// What the file says about itself so far, with its tokens.
    #[must_use]
    pub fn summary(&self, path: &Path) -> Option<ConversationSummary> {
        let mut summary = self.reader.summary(path)?;
        summary.tokens = self.tally.total();
        Some(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(input: u64, output: u64) -> TokenCounts {
        TokenCounts {
            input,
            output,
            ..TokenCounts::default()
        }
    }

    /// The store's rule: a response once, the larger figure of each class,
    /// however its lines fall across reads; a copy far back adds nothing.
    #[test]
    fn a_tally_counts_each_response_once_at_its_largest() {
        let mut tally = TokenTally::default();
        tally.add(Some("m1"), counts(10, 1));
        tally.add(Some("m1"), counts(10, 7));
        tally.add(Some("m2"), counts(5, 2));
        assert_eq!(tally.total(), counts(15, 9));
        // A smaller figure later never lowers it.
        tally.add(Some("m1"), counts(3, 3));
        assert_eq!(tally.total(), counts(15, 9));
        // Past the window, a repeat is a copy: nothing added.
        for n in 0..RECENT {
            tally.add(Some(&format!("other{n}")), counts(0, 0));
        }
        tally.add(Some("m1"), counts(100, 100));
        assert_eq!(tally.total(), counts(15, 9));
        // Usage with no key counts as it is.
        tally.add(None, counts(1, 1));
        tally.add(None, counts(1, 1));
        assert_eq!(tally.total(), counts(17, 11));
    }
}
