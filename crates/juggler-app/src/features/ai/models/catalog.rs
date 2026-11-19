//! AJ-E3812: what Settings › AI › Models shows besides the installed
//! models ([`super::dto::ModelsInfo`]): this Mac's facts, the
//! recommendation (AJ-E3811), and every model the screen may show, each
//! with its facts and its verdict per job.
//!
//! **Only what this Mac can use** (D-127): a downloadable model shows
//! when its chip and memory fit; Apple's when Apple Intelligence is on.
//! Free disk is the user's to fix, so a model short of it still shows
//! (its Download says what it needs). An installed model shows whatever
//! the list says, so its jobs can still be picked (ADR-0035 §11.4).
//!
//! Every word is the window's (the words file); these carry keys and
//! numbers only. Pure: the command reads the facts and hands them here.

use super::recommend::{AppleAi, HostChip, MacFacts, Recommendation, Why, WhyNot};
use super::{Chip, Engine, Job, Listing, ModelEntry, Speed, Verdict};

/// A job a verdict is given for (attention too: code decides it, and the
/// screen says so).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum ModelJob {
    /// The feed line.
    Feed,
    /// The turn review.
    Review,
    /// Attention hints.
    Attention,
    /// Bot's answers (D-136; `chat` on the wire, as in the file).
    #[serde(rename = "chat")]
    Bot,
    /// The digest.
    Digest,
    /// Watches drafted from Bot.
    Watches,
}

impl ModelJob {
    const fn of(job: Job) -> Self {
        match job {
            Job::Feed => Self::Feed,
            Job::Review => Self::Review,
            Job::Attention => Self::Attention,
            Job::Bot => Self::Bot,
            Job::Digest => Self::Digest,
            Job::Watches => Self::Watches,
        }
    }
}

/// A job's test verdict (ADR-0035 §11.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum ModelVerdict {
    /// Passed, on by default.
    Ships,
    /// Passed, as an opt-in Beta.
    Beta,
    /// Tested, not good enough yet.
    NotYet,
    /// Not tested.
    NotTested,
}

impl ModelVerdict {
    const fn of(verdict: Verdict) -> Self {
        match verdict {
            Verdict::Ships => Self::Ships,
            Verdict::Beta => Self::Beta,
            Verdict::NotYet => Self::NotYet,
            Verdict::NotTested => Self::NotTested,
        }
    }
}

/// How fast a model ran on the harness's reference Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum ModelSpeed {
    /// Quick.
    Quick,
    /// Moderate.
    Moderate,
    /// Slow.
    Slow,
}

impl ModelSpeed {
    const fn of(speed: Speed) -> Self {
        match speed {
            Speed::Quick => Self::Quick,
            Speed::Moderate => Self::Moderate,
            Speed::Slow => Self::Slow,
        }
    }
}

/// One job's verdict and its plain line.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ModelJobVerdict {
    /// The job.
    pub job: ModelJob,
    /// The verdict.
    pub verdict: ModelVerdict,
    /// The list's one line on why (data, shown as written).
    pub note: Option<String>,
}

/// One model the screen may show.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CatalogModel {
    /// Its id in the list.
    pub id: String,
    /// Its display name.
    pub name: String,
    /// Apple's built-in model: nothing to download.
    pub built_in: bool,
    /// Offered now (Recommended, Other tested models); `false` for an
    /// installed copy the list no longer has.
    pub offered: bool,
    /// Its file's size (`None` for Apple's model).
    #[ts(type = "number | null")]
    pub size_bytes: Option<u64>,
    /// Its licence's name.
    pub licence_name: Option<String>,
    /// Its licence's text.
    pub licence_url: Option<String>,
    /// The oldest Apple chip generation it runs on (M1 is 1).
    pub min_chip: u8,
    /// The least memory it needs, GB (the list's figure).
    #[ts(type = "number")]
    pub min_memory_gb: u64,
    /// The free disk a download needs before it starts: its size plus the
    /// margin (0 for Apple's model).
    #[ts(type = "number")]
    pub free_disk_needed_bytes: u64,
    /// How fast it ran on our reference Mac, when the list says.
    pub speed: Option<ModelSpeed>,
    /// The verdict per job, in the list's job order.
    pub verdicts: Vec<ModelJobVerdict>,
}

impl CatalogModel {
    fn of(entry: &ModelEntry, offered: bool) -> Self {
        let generation = Chip::ALL
            .iter()
            .position(|(chip, _)| *chip == entry.min_chip)
            .map_or(1, |index| u8::try_from(index + 1).unwrap_or(u8::MAX));
        Self {
            id: entry.id.clone(),
            name: entry.name.clone(),
            built_in: entry.engine == Engine::AppleBuiltin,
            offered,
            size_bytes: entry.download.as_ref().map(|d| d.size_bytes),
            licence_name: entry.download.as_ref().map(|d| d.licence_name.clone()),
            licence_url: entry.download.as_ref().map(|d| d.licence_url.clone()),
            min_chip: generation,
            min_memory_gb: entry.min_memory_gb,
            free_disk_needed_bytes: entry.free_disk_needed(),
            speed: entry.speed.map(ModelSpeed::of),
            verdicts: Job::ALL
                .iter()
                .map(|(job, _)| ModelJobVerdict {
                    job: ModelJob::of(*job),
                    verdict: ModelVerdict::of(entry.verdict(*job)),
                    note: entry.verdicts.get(job).and_then(|v| v.note.clone()),
                })
                .collect(),
        }
    }
}

/// Apple Intelligence on this Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum AppleAiState {
    /// On and ready.
    On,
    /// Eligible but off: the user can turn it on in System Settings.
    Off,
    /// Not on this Mac or this macOS.
    Unavailable,
}

/// This Mac, as the recommendation read it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MacInfo {
    /// Apple silicon (an Intel Mac runs no listed model).
    pub apple_silicon: bool,
    /// Its chip's generation (M1 is 1); `None` when newer than this build
    /// knows, or not Apple silicon.
    pub chip: Option<u8>,
    /// Its memory as the system reports it, bytes (`None`: unknown).
    #[ts(type = "number | null")]
    pub memory_bytes: Option<u64>,
    /// Its memory as the recommendation compares it, whole GB.
    #[ts(type = "number")]
    pub memory_gb: u64,
    /// Free disk where models live, bytes.
    #[ts(type = "number")]
    pub free_disk_bytes: u64,
    /// Apple Intelligence.
    pub apple_ai: AppleAiState,
}

impl MacInfo {
    fn of(mac: &MacFacts, memory_bytes: Option<u64>) -> Self {
        let (apple_silicon, chip) = match mac.chip {
            HostChip::Intel => (false, None),
            HostChip::AppleNewer => (true, None),
            HostChip::Apple(chip) => (
                true,
                Chip::ALL
                    .iter()
                    .position(|(known, _)| *known == chip)
                    .and_then(|index| u8::try_from(index + 1).ok()),
            ),
        };
        Self {
            apple_silicon,
            chip,
            memory_bytes,
            memory_gb: mac.memory_gb,
            free_disk_bytes: mac.free_disk_bytes,
            apple_ai: match mac.apple_ai {
                AppleAi::On => AppleAiState::On,
                AppleAi::Off => AppleAiState::Off,
                AppleAi::Unavailable => AppleAiState::Unavailable,
            },
        }
    }
}

/// Why the recommended model fits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum RecommendedWhy {
    /// Apple's model: built in, no download.
    BuiltIn,
    /// Already on this Mac.
    Installed,
    /// Fits this Mac's chip, memory and disk; a download.
    Fits,
}

/// Why nothing is recommended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum NothingWhy {
    /// An Intel Mac.
    NotAppleSilicon,
    /// No listed model fits this Mac's memory.
    NotEnoughMemory,
    /// A model fits the memory, but free disk is short (`needed_bytes`).
    NotEnoughDisk,
    /// Nothing listed passes a job (or the list can't be read).
    NothingListed,
}

/// The recommendation (AJ-E3811): a model and why, or nothing and why not.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RecommendationInfo {
    /// The recommended model's id.
    pub model_id: Option<String>,
    /// Why it fits.
    pub why: Option<RecommendedWhy>,
    /// Why nothing is recommended.
    pub nothing: Option<NothingWhy>,
    /// The free disk the smallest fitting model needs (not enough disk).
    #[ts(type = "number | null")]
    pub needed_bytes: Option<u64>,
    /// Turning Apple Intelligence on would give a recommendation.
    pub apple_ai_off: bool,
}

impl RecommendationInfo {
    fn of(recommendation: &Recommendation) -> Self {
        let mut info = Self {
            model_id: None,
            why: None,
            nothing: None,
            needed_bytes: None,
            apple_ai_off: false,
        };
        match recommendation {
            Recommendation::Model { id, why } => {
                info.model_id = Some(id.clone());
                info.why = Some(match why {
                    Why::BuiltIn => RecommendedWhy::BuiltIn,
                    Why::Installed => RecommendedWhy::Installed,
                    Why::Fits { .. } => RecommendedWhy::Fits,
                });
            }
            Recommendation::Nothing(why_not) => {
                info.nothing = Some(match *why_not {
                    WhyNot::NotAppleSilicon => NothingWhy::NotAppleSilicon,
                    WhyNot::NotEnoughMemory { apple_ai_off } => {
                        info.apple_ai_off = apple_ai_off;
                        NothingWhy::NotEnoughMemory
                    }
                    WhyNot::NotEnoughDisk {
                        needed_bytes,
                        apple_ai_off,
                    } => {
                        info.apple_ai_off = apple_ai_off;
                        info.needed_bytes = Some(needed_bytes);
                        NothingWhy::NotEnoughDisk
                    }
                    WhyNot::NothingListed => NothingWhy::NothingListed,
                });
            }
        }
        info
    }
}

/// Everything Settings › AI › Models shows besides the installed rows.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ModelsCatalog {
    /// The list of tested models was read; `false`: only the built-in
    /// list (Apple's model) is known.
    pub listed: bool,
    /// This Mac (`None` until it could be read).
    pub mac: Option<MacInfo>,
    /// The recommendation (`None` until this Mac could be read).
    pub recommendation: Option<RecommendationInfo>,
    /// The models the screen may show: the offered ones this Mac can use,
    /// then installed copies the list no longer has.
    pub models: Vec<CatalogModel>,
}

/// Whether this Mac can use `entry` at all (D-127): its chip and memory
/// fit; Apple's model needs Apple Intelligence on. Free disk is left out:
/// the user can fix it.
fn usable_here(entry: &ModelEntry, mac: &MacFacts) -> bool {
    let chip = match mac.chip {
        HostChip::Intel => false,
        HostChip::AppleNewer => true,
        HostChip::Apple(chip) => chip >= entry.min_chip,
    };
    let built_in_on = entry.engine != Engine::AppleBuiltin || mac.apple_ai == AppleAi::On;
    chip && built_in_on && entry.min_memory_gb <= mac.memory_gb
}

/// The screen's list for `mac` (`None`: not read yet, nothing offered is
/// shown), from `listing`'s `offered` models and the `installed` copies,
/// with the `recommendation` the router uses too.
#[must_use]
pub fn catalog(
    mac: Option<&MacFacts>,
    memory_bytes: Option<u64>,
    listing: &Listing,
    offered: &[ModelEntry],
    installed: &[ModelEntry],
    recommendation: Option<&Recommendation>,
) -> ModelsCatalog {
    let mut models: Vec<CatalogModel> = offered
        .iter()
        .filter(|entry| entry.passes_any_job() && mac.is_some_and(|mac| usable_here(entry, mac)))
        .map(|entry| CatalogModel::of(entry, true))
        .collect();
    for entry in installed {
        if !models.iter().any(|known| known.id == entry.id) {
            let offered_too = offered.iter().any(|known| known.id == entry.id);
            models.push(CatalogModel::of(entry, offered_too));
        }
    }
    ModelsCatalog {
        listed: matches!(listing, Listing::Read(_)),
        mac: mac.map(|mac| MacInfo::of(mac, memory_bytes)),
        recommendation: recommendation.map(RecommendationInfo::of),
        models,
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
