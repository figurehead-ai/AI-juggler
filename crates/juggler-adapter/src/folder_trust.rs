//! Lab 2 (D-40, 2026-10-25): a tool that asks whether to trust a folder
//! and saves the answer to the user's own config. AI Juggler never lets that
//! write happen for the user. It asks the question itself, once per tool and
//! folder, in the tool's own words. It keeps the answer in its own data, and
//! passes it to each launch in memory (an argument, never a file).

/// The question a tool asks about a folder, in the tool's own words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderTrust {
    /// The question ("Trust this folder?").
    pub question: &'static str,
    /// The tool's words for trusting it.
    pub trust: &'static str,
    /// The tool's words for not trusting it.
    pub restrict: &'static str,
    /// What the choice does, in one line, in AI Juggler's words.
    pub note: &'static str,
}
