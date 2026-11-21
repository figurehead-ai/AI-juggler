//! ADR-0035 §2 (AJ-E2023): feature flags, the `flags` section of the signed
//! remote configuration.
//!
//! **Each feature declares its own flags,** in its own module, as a
//! [`Flag`] const: a name, the compiled default, and what the remote file
//! may do to it ([`RemoteMode`]):
//!
//! | `remote` | What the file may do |
//! |---|---|
//! | `OffOnly` | turn it off (a kill switch); never on. Most flags. |
//! | `OnOff` | turn it on or off. A reviewer sees the declaration. |
//! | `Rollout` | turn it off, or on only for a share of installs (a bucketed rule; the bucket is computed on this Mac, never sent) |
//!
//! The remote-config crate enforces the mode: a value the mode doesn't
//! allow is dropped and the flag keeps its compiled default (or a later
//! kill), so **no feature can be turned on remotely unless it declares
//! `OnOff` or `Rollout`.** Security and privacy settings are never flags.
//!
//! [`DECLARED`] is the one list the app registers (ENGINEERING-GUIDE A5);
//! a feature asks [`is_on`] where it decides. With no file (none fetched,
//! none valid, the check off, no pinned key) every flag is its compiled
//! default.

use std::collections::{BTreeMap, BTreeSet};

use juggler_remote_config::{Effective, RemoteMode, SectionSpec};

/// The section's name in the file.
pub const SECTION: &str = "flags";

/// The section's schema version this build reads.
pub const SCHEMA: u32 = 1;

/// A feature flag, declared in its feature's own module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flag {
    /// Its name in the file, `[a-z0-9_]`, starting with its feature's
    /// name (`notices_on`).
    pub name: &'static str,
    /// The compiled default: what the app does with no file.
    pub default: bool,
    /// What the remote file may do to it.
    pub remote: RemoteMode,
}

/// Every flag the app has, each declared in its own feature's module.
pub const DECLARED: &[&Flag] = &[
    &crate::features::notices::FLAG,
    &crate::features::ai::jobs::FEED_ON,
    &crate::features::ai::jobs::REVIEW_ON,
    &crate::features::ai::jobs::BOT_ON,
    &crate::features::ai::jobs::DIGEST_ON,
    &crate::features::ai::jobs::WATCHES_ON,
];

/// The `flags` section for `flags`, to register with the app's
/// remote-config registry.
///
/// # Errors
///
/// A name declared twice (the registry itself refuses a bad name).
pub fn section(flags: &[&Flag]) -> Result<SectionSpec, String> {
    let mut seen = BTreeSet::new();
    let mut spec = SectionSpec::new(SECTION, SCHEMA);
    for flag in flags {
        if !seen.insert(flag.name) {
            return Err(format!("flag {:?} is declared twice", flag.name));
        }
        spec = spec.switch(flag.name, flag.default, flag.remote);
    }
    Ok(spec)
}

/// Whether `flag` is on in `effective` (its compiled default if the
/// section doesn't hold it).
#[must_use]
pub fn on_in(effective: &Effective, flag: &Flag) -> bool {
    effective.switch(SECTION, flag.name).unwrap_or(flag.default)
}

/// Whether `flag` is on now: from memory, no I/O.
#[must_use]
pub fn is_on(flag: &Flag) -> bool {
    on_in(&crate::features::remote_config::current().effective, flag)
}

/// Every declared flag's state in `effective`, by name.
#[must_use]
pub fn states(effective: &Effective) -> BTreeMap<String, bool> {
    DECLARED
        .iter()
        .map(|flag| (flag.name.to_owned(), on_in(effective, flag)))
        .collect()
}

#[cfg(test)]
mod tests;
