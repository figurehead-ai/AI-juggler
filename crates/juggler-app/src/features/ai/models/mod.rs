//! ADR-0035 §11 (AJ-E3810, D-131): the `models` section of the signed
//! remote configuration, the list of tested AI models the app may offer.
//!
//! **Data, not code, adds a model.** A record describes one model: its
//! name, the compiled engine that runs it, its one file (a pinned URL on a
//! compiled host, the SHA-256, the size), its licence, the least hardware
//! it needs, and a verdict with one plain line per job. Nothing here
//! branches on a model id: the recommendation, the Models screen, the
//! download and the router read [`ModelEntry`] and nothing else.
//!
//! **What a record can't do:**
//! - add an engine: `engine` is a compiled choice, and an entry naming
//!   any other is dropped whole (§11.3);
//! - point anywhere but [`source`]'s hosts, paths and organisations;
//! - make Apple's engine fetch anything: an `apple_builtin` entry with a
//!   file is dropped whole, and only the first one is read;
//! - turn a job on: each job has its own compiled flag (D-130). A verdict
//!   only says which models may serve a job that is on.
//!
//! **Read only when the app read it** ([`Listing`]): a missing, newer or
//! broken section is [`Listing::Unread`], and the built-in list applies
//! ([`builtin`], Apple's model only), read through the same reader. An
//! installed model keeps working from its installed copy whatever the
//! section says; a withdrawn or no-longer-listed one gets a plain note
//! ([`installed_state`], §11.4).

pub mod recommend;
pub mod source;

#[cfg(test)]
mod ledger;
#[cfg(test)]
mod recommend_tests;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::LazyLock;

use juggler_remote_config::list::{Field, FieldKind, ListSpec};
use juggler_remote_config::{Channel, Facts, InstallId, Item, Lists, Os, Registry, SectionSpec};
use serde_json::Value;

/// The section's name in the file.
pub const SECTION: &str = "models";

/// The section's schema version this build reads.
pub const SCHEMA: u32 = 1;

/// The most entries one file may carry.
pub const MAX_MODELS: u32 = 16;

/// The largest model file, in bytes (8 GiB).
pub const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// D-121 §2.5: free disk must stay at least this far above a model's size.
pub const DISK_MARGIN_BYTES: u64 = 10_000_000_000;

/// The longest display name and licence name, in characters.
pub const MAX_NAME_CHARS: u32 = 40;

/// The longest per-job line, in characters.
pub const MAX_NOTE_CHARS: u32 = 120;

/// The built-in list: Apple's on-device model only, with the verdicts
/// known at build time (§11.6). Read by the same reader as the file.
const BUILTIN: &str = include_str!("builtin.json");

/// The engines this build runs. The file can only pick one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Engine {
    /// A GGUF file run by the local engine (ADR-0033 §4.2).
    GgufLocal,
    /// Apple's built-in model (ADR-0033 §4.1); nothing to download.
    AppleBuiltin,
}

impl Engine {
    /// Every engine, with its name in the file.
    pub const ALL: [(Self, &'static str); 2] = [
        (Self::GgufLocal, "gguf_local"),
        (Self::AppleBuiltin, "apple_builtin"),
    ];

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, known)| *known == name)
            .map(|(engine, _)| *engine)
    }
}

/// The AI jobs a verdict is given for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Job {
    /// The feed line.
    Feed,
    /// The turn review.
    Review,
    /// Attention hints (decided by code from the facts).
    Attention,
    /// Chat answers.
    Chat,
    /// The digest.
    Digest,
    /// Watches drafted from Chat.
    Watches,
}

impl Job {
    /// Every job, with its field name in the file.
    pub const ALL: [(Self, &'static str); 6] = [
        (Self::Feed, "feed"),
        (Self::Review, "review"),
        (Self::Attention, "attention"),
        (Self::Chat, "chat"),
        (Self::Digest, "digest"),
        (Self::Watches, "watches"),
    ];

    /// Its field name in the file.
    #[must_use]
    pub fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(job, _)| *job == self)
            .map_or("", |(_, name)| name)
    }

    /// Whether a pass in this job lets a model be listed (§11.5):
    /// attention is decided by code, so it says nothing about the model.
    #[must_use]
    pub const fn counts_for_listing(self) -> bool {
        !matches!(self, Self::Attention)
    }
}

/// A job's test verdict, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Verdict {
    /// Not tested for this job (also: missing, or a state this build
    /// doesn't know).
    #[default]
    NotTested,
    /// Tested and not good enough yet.
    NotYet,
    /// Passed: offered as an opt-in Beta.
    Beta,
    /// Passed and on by default.
    Ships,
}

impl Verdict {
    /// Every verdict, with its name in the file.
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::Ships, "ships"),
        (Self::Beta, "beta"),
        (Self::NotYet, "not_yet"),
        (Self::NotTested, "not_tested"),
    ];

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, known)| *known == name)
            .map(|(verdict, _)| *verdict)
    }

    /// Whether the model passed this job (it may serve it when the job is
    /// on, `Beta` only for a user who turned that job's Beta on).
    #[must_use]
    pub const fn passed(self) -> bool {
        matches!(self, Self::Beta | Self::Ships)
    }
}

/// The oldest Apple silicon generation a model runs on. A chip newer than
/// this build knows counts as newer than all of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Chip {
    /// M1.
    M1,
    /// M2.
    M2,
    /// M3.
    M3,
    /// M4.
    M4,
    /// M5.
    M5,
}

impl Chip {
    /// Every generation, with its name in the file.
    pub const ALL: [(Self, &'static str); 5] = [
        (Self::M1, "m1"),
        (Self::M2, "m2"),
        (Self::M3, "m3"),
        (Self::M4, "m4"),
        (Self::M5, "m5"),
    ];

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, known)| *known == name)
            .map(|(chip, _)| *chip)
    }
}

/// How fast a model ran on the harness's reference Mac.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Speed {
    /// Quick.
    Quick,
    /// Moderate.
    Moderate,
    /// Slow.
    Slow,
}

impl Speed {
    /// Every speed, with its name in the file.
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::Quick, "quick"),
        (Self::Moderate, "moderate"),
        (Self::Slow, "slow"),
    ];

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, known)| *known == name)
            .map(|(speed, _)| *speed)
    }
}

/// A downloadable model's one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    /// The pinned URL.
    pub url: String,
    /// Where `url` comes from.
    pub source: source::Source,
    /// A second source of the same bytes, tried only if `url` fails.
    pub mirror_url: Option<String>,
    /// The file's SHA-256, lower-case hex.
    pub sha256: String,
    /// The file's exact size.
    pub size_bytes: u64,
    /// The licence's name.
    pub licence_name: String,
    /// The licence's text.
    pub licence_url: String,
}

/// The sampling the harness tested a model with; `None` is the engine's
/// default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sampling {
    /// Temperature × 1000.
    pub temperature_milli: Option<u64>,
    /// Top-p × 1000.
    pub top_p_milli: Option<u64>,
    /// Top-k.
    pub top_k: Option<u64>,
    /// Min-p × 1000.
    pub min_p_milli: Option<u64>,
    /// Presence penalty × 1000.
    pub presence_milli: Option<u64>,
    /// The context it was tested with, in tokens.
    pub context_tokens: Option<u64>,
    /// Whether the facts guard marks quoted data for it.
    pub datamark: Option<bool>,
}

/// A job's verdict and its plain line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JobVerdict {
    /// The verdict.
    pub verdict: Verdict,
    /// One plain line: why.
    pub note: Option<String>,
}

/// One model, as the section describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    /// Its id, unique in the list; a new file is a new id.
    pub id: String,
    /// Its display name.
    pub name: String,
    /// The engine that runs it.
    pub engine: Engine,
    /// Its file; `None` for Apple's built-in model.
    pub download: Option<Download>,
    /// The oldest chip it runs on.
    pub min_chip: Chip,
    /// The least memory it needs, in GB.
    pub min_memory_gb: u64,
    /// Extra free disk to require, in GB (it only tightens).
    pub min_free_gb: Option<u64>,
    /// How fast it ran on the reference Mac.
    pub speed: Option<Speed>,
    /// The sampling it was tested with.
    pub sampling: Sampling,
    /// The verdict per job (every job present).
    pub verdicts: BTreeMap<Job, JobVerdict>,
    /// No longer offered (§11.4).
    pub withdrawn: bool,
}

impl ModelEntry {
    /// The verdict for `job`.
    #[must_use]
    pub fn verdict(&self, job: Job) -> Verdict {
        self.verdicts
            .get(&job)
            .map(|verdict| verdict.verdict)
            .unwrap_or_default()
    }

    /// Whether it passed a job that counts for listing (§11.5).
    #[must_use]
    pub fn passes_any_job(&self) -> bool {
        self.verdicts
            .iter()
            .any(|(job, verdict)| job.counts_for_listing() && verdict.verdict.passed())
    }

    /// The free disk a download needs: the larger of its size plus
    /// [`DISK_MARGIN_BYTES`] and the entry's own `min_free_gb`. Zero for
    /// Apple's model.
    #[must_use]
    pub fn free_disk_needed(&self) -> u64 {
        let Some(download) = &self.download else {
            return 0;
        };
        let rule = download.size_bytes.saturating_add(DISK_MARGIN_BYTES);
        let asked = self.min_free_gb.unwrap_or(0).saturating_mul(1_000_000_000);
        rule.max(asked)
    }
}

/// The `models` section, to register with the app's remote-config
/// registry.
#[must_use]
pub fn section() -> SectionSpec {
    let text = |max_chars| FieldKind::Text { max_chars };
    let choice = |names: &[&str]| FieldKind::Choice {
        options: names.iter().map(|name| (*name).to_owned()).collect(),
    };
    let count = |min, max| FieldKind::Count { min, max };
    let link = |hosts: &[&str]| FieldKind::Link {
        hosts: hosts.iter().map(|host| (*host).to_owned()).collect(),
    };
    let mut list = ListSpec::new(MAX_MODELS)
        .field("name", Field::required(text(MAX_NAME_CHARS)))
        .field(
            "engine",
            Field::required(choice(&Engine::ALL.map(|(_, name)| name))),
        )
        .field("url", Field::optional(link(source::DOWNLOAD_HOSTS)))
        .field("mirror_url", Field::optional(link(source::DOWNLOAD_HOSTS)))
        .field("sha256", Field::optional(FieldKind::Sha256))
        .field("size_bytes", Field::optional(count(1, MAX_FILE_BYTES)))
        .field("licence_name", Field::optional(text(MAX_NAME_CHARS)))
        .field("licence_url", Field::optional(link(source::LICENCE_HOSTS)))
        .field(
            "min_chip",
            Field::required(choice(&Chip::ALL.map(|(_, name)| name))),
        )
        .field("min_memory_gb", Field::required(count(8, 256)))
        .field("min_free_gb", Field::optional(count(0, 512)))
        .field(
            "speed",
            Field::optional(choice(&Speed::ALL.map(|(_, name)| name))),
        )
        .field("temperature_milli", Field::optional(count(0, 2_000)))
        .field("top_p_milli", Field::optional(count(1, 1_000)))
        .field("top_k", Field::optional(count(0, 200)))
        .field("min_p_milli", Field::optional(count(0, 1_000)))
        .field("presence_milli", Field::optional(count(0, 2_000)))
        .field("context_tokens", Field::optional(count(1_024, 32_768)))
        .field("datamark", Field::optional(FieldKind::Flag))
        .field("withdrawn", Field::optional(FieldKind::Flag));
    let verdicts = Verdict::ALL.map(|(_, name)| name);
    for (_, job) in Job::ALL {
        list = list.field(job, Field::optional(choice(&verdicts))).field(
            &format!("{job}_note"),
            Field::optional(text(MAX_NOTE_CHARS)),
        );
    }
    SectionSpec::new(SECTION, SCHEMA).list(list)
}

/// The record check (§11.1): one record as a typed entry, or why the
/// whole entry is dropped.
///
/// # Errors
///
/// The reason, in words: a missing file field on a `gguf_local` entry, a
/// file on an `apple_builtin` one, or a URL that breaks [`source`]'s
/// rules.
pub fn entry(item: &Item) -> Result<ModelEntry, String> {
    let engine = item
        .choice("engine")
        .and_then(Engine::from_name)
        .ok_or("an engine this app doesn't run")?;
    let file_fields = ["url", "mirror_url", "sha256", "size_bytes"];
    let download = match engine {
        Engine::AppleBuiltin => {
            if let Some(field) = file_fields.iter().find(|f| item.fields.contains_key(**f)) {
                return Err(format!("Apple's model has no file ({field})"));
            }
            None
        }
        Engine::GgufLocal => Some(download(item)?),
    };
    let min_chip = item
        .choice("min_chip")
        .and_then(Chip::from_name)
        .ok_or("min_chip: missing")?;
    let min_memory_gb = item
        .count("min_memory_gb")
        .ok_or("min_memory_gb: missing")?;
    let verdicts = Job::ALL
        .iter()
        .map(|(job, name)| {
            let verdict = item
                .choice(name)
                .and_then(Verdict::from_name)
                .unwrap_or_default();
            let note = item.text(&format!("{name}_note")).map(str::to_owned);
            (*job, JobVerdict { verdict, note })
        })
        .collect();
    Ok(ModelEntry {
        id: item.id.clone(),
        name: item.text("name").ok_or("name: missing")?.to_owned(),
        engine,
        download,
        min_chip,
        min_memory_gb,
        min_free_gb: item.count("min_free_gb"),
        speed: item.choice("speed").and_then(Speed::from_name),
        sampling: Sampling {
            temperature_milli: item.count("temperature_milli"),
            top_p_milli: item.count("top_p_milli"),
            top_k: item.count("top_k"),
            min_p_milli: item.count("min_p_milli"),
            presence_milli: item.count("presence_milli"),
            context_tokens: item.count("context_tokens"),
            datamark: item.flag("datamark"),
        },
        verdicts,
        withdrawn: item.flag("withdrawn").unwrap_or(false),
    })
}

/// A `gguf_local` entry's file, or why the entry is dropped.
fn download(item: &Item) -> Result<Download, String> {
    let missing = |field: &str| format!("{field}: missing or unreadable");
    let sha256 = item.sha256("sha256").ok_or_else(|| missing("sha256"))?;
    let size_bytes = item
        .count("size_bytes")
        .ok_or_else(|| missing("size_bytes"))?;
    let url = item.link("url").ok_or_else(|| missing("url"))?;
    let licence_name = item
        .text("licence_name")
        .ok_or_else(|| missing("licence_name"))?;
    let licence_url = item
        .link("licence_url")
        .ok_or_else(|| missing("licence_url"))?;
    let source = source::pinned(url, sha256).map_err(|why| format!("url: {why}"))?;
    let mirror_url = item.link("mirror_url");
    if let Some(mirror) = mirror_url {
        source::pinned(mirror, sha256).map_err(|why| format!("mirror_url: {why}"))?;
    }
    if !source::licence_link_ok(licence_url) {
        return Err("licence_url: not a plain link to an allowed host".to_owned());
    }
    Ok(Download {
        url: url.to_owned(),
        source,
        mirror_url: mirror_url.map(str::to_owned),
        sha256: sha256.to_owned(),
        size_bytes,
        licence_name: licence_name.to_owned(),
        licence_url: licence_url.to_owned(),
    })
}

/// The entries of read records, in the file's order: a record failing the
/// record check is dropped whole (and logged), and only the first
/// `apple_builtin` entry is kept.
#[must_use]
pub fn entries(items: &[Item]) -> Vec<ModelEntry> {
    let mut out: Vec<ModelEntry> = Vec::new();
    for item in items {
        match entry(item) {
            Ok(entry)
                if entry.engine == Engine::AppleBuiltin
                    && out.iter().any(|e| e.engine == Engine::AppleBuiltin) =>
            {
                tracing::warn!(id = %entry.id, "models: a second Apple entry was dropped");
            }
            Ok(entry) => out.push(entry),
            Err(reason) => {
                tracing::warn!(id = %item.id, %reason, "models: an entry was dropped");
            }
        }
    }
    out
}

/// What the app knows of the section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listing {
    /// No file, or a file whose section is missing, newer or unreadable:
    /// the app can't tell what is listed.
    Unread,
    /// The section as read: its entries that passed the record check.
    Read(Vec<ModelEntry>),
}

/// The section in `lists` (a snapshot's): read only if the app read it.
#[must_use]
pub fn listing(lists: &Lists) -> Listing {
    lists
        .get(SECTION)
        .map_or(Listing::Unread, |items| Listing::Read(entries(items)))
}

/// A registry with this section alone, to read the built-in list.
static OWN_REGISTRY: LazyLock<Option<Registry>> = LazyLock::new(|| {
    let mut registry = Registry::new();
    registry.register(section()).ok()?;
    Some(registry)
});

/// The records of a raw `models` section value, read by the same reader
/// as the file; `None` if it can't be read. `facts` decide targeting.
#[must_use]
pub fn read_raw(raw: &Value, facts: &Facts) -> Option<Vec<ModelEntry>> {
    let registry = OWN_REGISTRY.as_ref()?;
    let sections = BTreeMap::from([(SECTION.to_owned(), raw.clone())]);
    let (mut lists, _) = registry.resolve_lists(&sections, facts);
    lists.remove(SECTION).map(|items| entries(&items))
}

/// The built-in list (§11.6): Apple's model only. It carries no
/// targeting, so any facts read it the same.
#[must_use]
pub fn builtin() -> Vec<ModelEntry> {
    let facts = Facts {
        app_version: juggler_remote_config::Version::new(0, 0, 0),
        channel: Channel::Stable,
        os: Os::Macos,
        os_version: None,
        tools: BTreeMap::new(),
        install: InstallId::from_bytes([0; 16]),
    };
    serde_json::from_str::<Value>(BUILTIN)
        .ok()
        .and_then(|raw| read_raw(&raw, &facts))
        .unwrap_or_default()
}

/// The models the app may offer now (Recommended and Other tested
/// models): the section's entries that aren't withdrawn, plus the
/// built-in Apple entry unless the section has its own; with the section
/// unread, the built-in list.
#[must_use]
pub fn offered(listing: &Listing) -> Vec<ModelEntry> {
    match listing {
        Listing::Unread => builtin(),
        Listing::Read(entries) => {
            let mut out: Vec<ModelEntry> = entries
                .iter()
                .filter(|entry| !entry.withdrawn)
                .cloned()
                .collect();
            if !entries.iter().any(|e| e.engine == Engine::AppleBuiltin) {
                out.splice(0..0, builtin());
            }
            out
        }
    }
}

/// An installed model's standing against the section (§11.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstalledState {
    /// Listed: its copy refreshed from the entry (verdicts may have
    /// narrowed or widened).
    Listed(ModelEntry),
    /// The section can't be read: the installed copy stands, with no note.
    Unknown(ModelEntry),
    /// Withdrawn or no longer listed: still works from its copy, with the
    /// plain note, until the user removes it.
    Withdrawn(ModelEntry),
}

/// Where the installed `copy` stands in `listing`. An entry with the same
/// id but another file is ignored (the copy stands and nothing is
/// fetched); a new file is a new id.
#[must_use]
pub fn installed_state(copy: &ModelEntry, listing: &Listing) -> InstalledState {
    let Listing::Read(entries) = listing else {
        return InstalledState::Unknown(copy.clone());
    };
    let same_file = |entry: &ModelEntry| {
        entry.engine == copy.engine
            && entry.download.as_ref().map(|d| (&d.sha256, d.size_bytes))
                == copy.download.as_ref().map(|d| (&d.sha256, d.size_bytes))
    };
    match entries.iter().find(|entry| entry.id == copy.id) {
        Some(entry) if !same_file(entry) => {
            tracing::warn!(id = %copy.id, "models: an entry changed its file; ignored here");
            InstalledState::Unknown(copy.clone())
        }
        Some(entry) if entry.withdrawn => InstalledState::Withdrawn(entry.clone()),
        Some(entry) => InstalledState::Listed(entry.clone()),
        None => InstalledState::Withdrawn(copy.clone()),
    }
}
