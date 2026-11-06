//! ADR-0028, capability `automation_handover`: what wakes a conversation
//! by itself in its tool, and handing it over in a switch.
//!
//! A tool can keep a conversation working with nobody at the keyboard:
//! the Claude app fixes a pull request's failing checks (Auto-fix) and
//! resumes a session when usage resets; Codex pursues a thread's goal
//! until it's done. After a switch two copies of the conversation exist,
//! and if the original keeps that automation it works on beside the copy,
//! on an account the user left (often because its usage ran out). So a
//! switch hands it over, the way the tools' own moves do:
//!
//! 1. [`HostAutomation::read`] names it before the switch (the confirm
//!    screen says it);
//! 2. [`HostAutomation::adopt`] gives the copy what the original had;
//! 3. [`HostAutomation::quiet`] turns it off on the original, keeping
//!    what's needed to undo that;
//! 4. switching back adopts the copy's into the original, wakes the
//!    original ([`HostAutomation::wake`]) and quiets the copy.
//!
//! Nothing here changes a conversation's own files: only the tool's own
//! marks of what runs by itself, through its API where it has one.
//!
//! Where a tool reads those marks only when it starts (the Claude app's
//! records), [`HostAutomation::home`] says so and AI Juggler applies them
//! while that app is closed; an edit that finds its record not there yet
//! answers [`Applied::NotYet`] and waits.

use std::path::Path;

use crate::AdapterError;

/// Where a tool keeps a conversation's automation, which says when it
/// can be changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomationHome {
    /// A desktop app profile's own records (its instance folder), read
    /// only when the app starts: changed while it's closed.
    AppProfile,
    /// The account's tool folder, changed through the tool's API at once.
    Account,
}

/// What runs by itself for a set of conversations.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Automation {
    /// Each thing, in words for the switch screen ("Auto-fix on pull
    /// request #210"). Empty when nothing runs by itself.
    pub what: Vec<String>,
    /// What runs by itself and stays with the original whatever the
    /// switch does (it belongs to the account), in words.
    pub stays: Vec<String>,
    /// The tool's own description of it, by conversation id, for
    /// [`HostAutomation::adopt`]. Opaque to everyone but the adapter.
    pub carried: serde_json::Value,
}

impl Automation {
    /// Whether anything runs by itself that a switch can hand over.
    #[must_use]
    pub fn any(&self) -> bool {
        !self.what.is_empty()
    }
}

/// Whether an edit took effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    /// Done.
    Done,
    /// Something it needs isn't there yet (the target app hasn't
    /// imported the conversation): try again later.
    NotYet,
}

/// A tool's automation, for the switch ([`crate::ToolRuntime::host_automation`]).
pub trait HostAutomation: Send + Sync {
    /// Where it's kept, and so when it can be changed.
    fn home(&self) -> AutomationHome;

    /// What runs by itself for `conversations` (a conversation and its
    /// family) in `home`: the app profile's instance folder or the
    /// account's tool folder, as [`Self::home`] says. Only reads.
    ///
    /// # Errors
    ///
    /// Something there couldn't be read.
    fn read(&self, home: &Path, conversations: &[String]) -> Result<Automation, AdapterError>;

    /// Gives the conversations in `home` the automation `carried` (from
    /// [`Self::read`] of their originals); `renamed` maps an original's
    /// id to its copy's where they differ.
    ///
    /// # Errors
    ///
    /// A refusal in words (a record of a shape the adapter doesn't
    /// know), or the write that failed.
    fn adopt(
        &self,
        home: &Path,
        carried: &serde_json::Value,
        renamed: &[(String, String)],
    ) -> Result<Applied, AdapterError>;

    /// Turns off what runs by itself for `conversations` in `home`, and
    /// returns what [`Self::wake`] needs to undo it.
    ///
    /// # Errors
    ///
    /// As [`Self::adopt`].
    fn quiet(
        &self,
        home: &Path,
        conversations: &[String],
    ) -> Result<serde_json::Value, AdapterError>;

    /// Undoes [`Self::quiet`] (`restore` is what it returned), except the
    /// automation itself, which a switch back adopts from the copy.
    ///
    /// # Errors
    ///
    /// As [`Self::adopt`].
    fn wake(&self, home: &Path, restore: &serde_json::Value) -> Result<Applied, AdapterError>;
}

/// A conversation id as `renamed` says it is on the copy.
#[must_use]
pub fn renamed_to<'a>(renamed: &'a [(String, String)], id: &'a str) -> &'a str {
    renamed
        .iter()
        .find(|(old, _)| old == id)
        .map_or(id, |(_, new)| new.as_str())
}
