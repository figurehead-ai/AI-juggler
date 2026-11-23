//! AJ-E3811 (D-131): the one model AI Juggler recommends for this Mac, and
//! why, from this Mac's facts and the offered list ([`super::offered`]).
//!
//! A pure, table-driven function: the same facts and list give the same
//! answer, no model in the loop, no branch on a model id.
//! 1. **Apple's model first** when Apple Intelligence is on and its entry
//!    passes a job: it needs no download.
//! 2. **Apple's model can run here but isn't on yet** (turned off, macOS
//!    still getting it) **or its state couldn't be read** (AJ-E3823: a
//!    probe that failed, got no answer, or was blocked): a model already
//!    downloaded that fits keeps working; otherwise nothing is
//!    recommended, with that reason. Turning it on (or checking again)
//!    costs no download, so none is recommended on a guess.
//! 3. **Otherwise the downloadable model that passes the most jobs** and
//!    fits this Mac: its chip, its memory, and free disk of its size plus
//!    10 GB (D-121 §2.5). Ties go to one already installed, then the
//!    smaller file, then the list's order.
//! 4. **Otherwise nothing,** with the reason. On a Mac without Apple
//!    Intelligence and under 16 GB, that is a correct answer.
//!
//! The words for each reason live in the UI's words file; this returns
//! the reason and the facts it rests on.

use super::{Chip, Engine, ModelEntry};

/// This Mac's chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostChip {
    /// An Intel Mac: no listed model runs on it.
    Intel,
    /// Apple silicon of a generation this build knows.
    Apple(Chip),
    /// Apple silicon newer than this build knows: newer than all.
    AppleNewer,
}

/// Apple Intelligence on this Mac.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppleAi {
    /// On and ready.
    On,
    /// Eligible but turned off (the user can turn it on).
    Off,
    /// On, and the system is still getting its model (it will be ready
    /// by itself).
    Preparing,
    /// Not on this Mac or this macOS (the model or the OS said so).
    Unavailable,
    /// Couldn't check: the helper failed, is missing, or answered with a
    /// word this build doesn't know.
    Unchecked,
    /// Couldn't check: the helper started and said nothing in time (a
    /// system prompt about it may be waiting).
    NoAnswer,
    /// The system refused to run AI Juggler's helper, or stopped it.
    Blocked,
}

impl AppleAi {
    /// This Mac can run it, and it isn't on yet: turned off, or the
    /// system is still getting it.
    #[must_use]
    pub const fn can_turn_on(self) -> bool {
        matches!(self, Self::Off | Self::Preparing)
    }

    /// Its state couldn't be read (never "can't run it").
    #[must_use]
    pub const fn unknown(self) -> bool {
        matches!(self, Self::Unchecked | Self::NoAnswer | Self::Blocked)
    }
}

/// What the recommendation reads of this Mac (through the OS seams).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacFacts {
    /// The chip.
    pub chip: HostChip,
    /// Memory, in GB.
    pub memory_gb: u64,
    /// Free disk, in bytes.
    pub free_disk_bytes: u64,
    /// Apple Intelligence.
    pub apple_ai: AppleAi,
}

/// Why a model is recommended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Apple's model: built in, no download.
    BuiltIn,
    /// Already on this Mac, and it fits.
    Installed,
    /// Fits this Mac's memory and disk; a download of `size_bytes`.
    Fits {
        /// The download's size.
        size_bytes: u64,
    },
}

/// Why nothing is recommended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhyNot {
    /// An Intel Mac.
    NotAppleSilicon,
    /// Apple's model can run here: turn Apple Intelligence on (free,
    /// nothing to download), or wait while the system gets it.
    TurnOnAppleAi {
        /// The system is still getting it; nothing to do but wait.
        preparing: bool,
    },
    /// Apple's model's state couldn't be read: check again, or fix what
    /// blocked the helper. No download is recommended on a guess.
    AppleAiUnknown,
    /// No listed model fits this Mac's memory (Apple Intelligence aside).
    NotEnoughMemory {
        /// Whether turning Apple Intelligence on would give one.
        apple_ai_off: bool,
    },
    /// A model fits the memory, but free disk is short.
    NotEnoughDisk {
        /// The free disk the smallest fitting model needs.
        needed_bytes: u64,
        /// Whether turning Apple Intelligence on would give one.
        apple_ai_off: bool,
    },
    /// Nothing listed passes a job.
    NothingListed,
}

/// The recommendation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recommendation {
    /// This model, and why.
    Model {
        /// Its id in the list.
        id: String,
        /// Why.
        why: Why,
    },
    /// None, and why not.
    Nothing(WhyNot),
}

fn chip_fits(entry: &ModelEntry, chip: HostChip) -> bool {
    match chip {
        HostChip::Intel => false,
        HostChip::AppleNewer => true,
        HostChip::Apple(chip) => chip >= entry.min_chip,
    }
}

fn passed_jobs(entry: &ModelEntry) -> usize {
    entry
        .verdicts
        .iter()
        .filter(|(job, verdict)| job.counts_for_listing() && verdict.verdict.passed())
        .count()
}

/// The recommendation for `mac` from `offered` (withdrawn models already
/// left out), `installed` being the ids of the models on this Mac.
#[must_use]
pub fn recommend(mac: &MacFacts, offered: &[ModelEntry], installed: &[&str]) -> Recommendation {
    if mac.chip == HostChip::Intel {
        return Recommendation::Nothing(WhyNot::NotAppleSilicon);
    }
    let passing: Vec<&ModelEntry> = offered
        .iter()
        .filter(|entry| entry.passes_any_job() && chip_fits(entry, mac.chip))
        .collect();
    // 1. Apple's model, when it's on and passes.
    if mac.apple_ai == AppleAi::On
        && let Some(apple) = passing.iter().find(|entry| {
            entry.engine == Engine::AppleBuiltin && entry.min_memory_gb <= mac.memory_gb
        })
    {
        return Recommendation::Model {
            id: apple.id.clone(),
            why: Why::BuiltIn,
        };
    }
    let downloads: Vec<&ModelEntry> = passing
        .iter()
        .copied()
        .filter(|entry| entry.engine == Engine::GgufLocal)
        .collect();
    let is_installed = |entry: &ModelEntry| installed.contains(&entry.id.as_str());
    // 2. Apple's model could be the answer but isn't on, or its state is
    //    unknown: one already downloaded keeps working, else no download.
    let apple_here = passing
        .iter()
        .any(|entry| entry.engine == Engine::AppleBuiltin && entry.min_memory_gb <= mac.memory_gb);
    if apple_here && (mac.apple_ai.can_turn_on() || mac.apple_ai.unknown()) {
        if let Some(entry) = downloads
            .iter()
            .filter(|entry| is_installed(entry) && entry.min_memory_gb <= mac.memory_gb)
            .min_by_key(|entry| std::cmp::Reverse(passed_jobs(entry)))
        {
            return Recommendation::Model {
                id: entry.id.clone(),
                why: Why::Installed,
            };
        }
        return Recommendation::Nothing(if mac.apple_ai.unknown() {
            WhyNot::AppleAiUnknown
        } else {
            WhyNot::TurnOnAppleAi {
                preparing: mac.apple_ai == AppleAi::Preparing,
            }
        });
    }
    // 3. The downloadable model that passes the most jobs and fits.
    if downloads.is_empty() {
        return Recommendation::Nothing(WhyNot::NothingListed);
    }
    let in_memory: Vec<&ModelEntry> = downloads
        .iter()
        .copied()
        .filter(|entry| entry.min_memory_gb <= mac.memory_gb)
        .collect();
    // Turning it on gives one only when Apple's model could serve here,
    // and that case was answered above (step 2): kept for the reasons'
    // shape, honest whenever it's read.
    let apple_ai_off = apple_here && mac.apple_ai == AppleAi::Off;
    if in_memory.is_empty() {
        return Recommendation::Nothing(WhyNot::NotEnoughMemory { apple_ai_off });
    }
    let size = |entry: &ModelEntry| entry.download.as_ref().map_or(0, |d| d.size_bytes);
    let best = in_memory
        .iter()
        .enumerate()
        .filter(|(_, entry)| is_installed(entry) || entry.free_disk_needed() <= mac.free_disk_bytes)
        .min_by_key(|(order, entry)| {
            (
                std::cmp::Reverse(passed_jobs(entry)),
                !is_installed(entry),
                size(entry),
                *order,
            )
        })
        .map(|(_, entry)| *entry);
    match best {
        Some(entry) if is_installed(entry) => Recommendation::Model {
            id: entry.id.clone(),
            why: Why::Installed,
        },
        Some(entry) => Recommendation::Model {
            id: entry.id.clone(),
            why: Why::Fits {
                size_bytes: size(entry),
            },
        },
        None => Recommendation::Nothing(WhyNot::NotEnoughDisk {
            needed_bytes: in_memory
                .iter()
                .map(|entry| entry.free_disk_needed())
                .min()
                .unwrap_or(0),
            apple_ai_off,
        }),
    }
}
