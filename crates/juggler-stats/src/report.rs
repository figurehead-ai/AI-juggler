//! A closed week turned into one schema-1 report (spec §6): entries built
//! from the store, rounded, clamped to the caps, validated against the
//! schema and kept under the size limit.

use std::collections::BTreeMap;

use crate::generated::{COUNTER_MAX, HISTOGRAM_TOTAL_MAX, METRICS, MetricMeta, SCHEMA_VERSION};
use crate::store::{Store, StoreError};

/// The hard size limit, uncompressed.
pub const MAX_REPORT_BYTES: usize = 8192;

/// Counters reported to two significant figures (token totals and
/// minutes, spec §6.3).
const ROUNDED: &[&str] = &["instance.minutes", "session.tokens"];

/// The release channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Beta builds.
    Beta,
    /// Stable builds.
    Stable,
}

/// The operating system family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsName {
    /// macOS.
    Macos,
    /// Linux.
    Linux,
    /// Windows.
    Windows,
}

/// The chip architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Arch {
    /// Apple silicon and other 64-bit ARM.
    #[serde(rename = "arm64")]
    Arm64,
    /// Intel and AMD 64-bit.
    #[serde(rename = "x86_64")]
    X86_64,
}

/// What every report says about the app and the OS (spec §7.1). The app
/// fills it; nothing here comes from the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// The app version, `major.minor.patch` (from the build).
    pub app_version: String,
    /// The release channel.
    pub channel: Channel,
    /// The OS family.
    pub os: OsName,
    /// The OS major version.
    pub os_major: u32,
    /// The chip architecture.
    pub arch: Arch,
}

/// `app` in a report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct App {
    /// The app version.
    pub version: String,
    /// The release channel.
    pub channel: Channel,
}

/// `os` in a report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Os {
    /// The OS family.
    pub name: OsName,
    /// Its major version.
    pub major: u32,
    /// The chip architecture.
    pub arch: Arch,
}

/// One entry: a counter (`v`), a histogram (`h`) or a gauge (`g`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The metric.
    pub m: String,
    /// Its dimensions (sorted keys).
    pub d: BTreeMap<String, String>,
    /// A counter's value (≥ 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v: Option<u64>,
    /// A histogram's counts per bucket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<Vec<u64>>,
    /// A gauge's bucket label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
}

/// A schema-1 report, exactly as uploaded.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// The schema version.
    pub schema: u32,
    /// A fresh `UUIDv4`, used only to drop duplicate uploads.
    pub report_id: String,
    /// The ISO week (UTC).
    pub week: String,
    /// The sampling rate applied (set just before upload).
    pub sample_rate: f64,
    /// The app.
    pub app: App,
    /// The OS.
    pub os: Os,
    /// How long ago stats were first turned on, as a bucket.
    pub install_age: String,
    /// Days that week the app ran (0–7).
    pub days_active: u32,
    /// The consent version the user answered.
    pub consent_version: u32,
    /// The entries.
    pub entries: Vec<Entry>,
    /// The puzzle nonce, only when the service asks for one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pow: Option<String>,
}

impl Report {
    /// The exact bytes uploaded (compact JSON).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
}

/// A fresh random `UUIDv4`.
#[must_use]
pub fn uuid_v4() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = crate::store::hex(&b);
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// The `install_age` bucket for weeks since stats were first on.
#[must_use]
pub fn install_age(first_week: &str, week: &str) -> &'static str {
    let (Some(first), Some(this)) = (
        crate::week::start_ms(first_week),
        crate::week::start_ms(week),
    ) else {
        return "<1w";
    };
    let weeks = (this - first).max(0) / (7 * 86_400_000);
    match weeks {
        0 => "<1w",
        1..=4 => "1-4w",
        5..=12 => "1-3m",
        13..=51 => "3-12m",
        _ => "1y+",
    }
}

fn meta_of(name: &str) -> Option<&'static MetricMeta> {
    METRICS.iter().find(|m| m.name == name)
}

/// What assembly had to change to fit the schema's caps and size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Adjustments {
    /// Counters or histograms clamped to a cap, plus entries dropped for
    /// size (counted in `stats.truncated`).
    pub truncated: u64,
}

/// The week's entries from the store: counters (rounded, clamped),
/// histograms (scaled to their cap) and gauges.
fn entries_of(store: &Store, week: &str) -> Result<(Vec<Entry>, Adjustments), StoreError> {
    let mut adjustments = Adjustments::default();
    let mut counters: BTreeMap<(String, BTreeMap<String, String>), u64> = BTreeMap::new();
    let mut histograms: BTreeMap<(String, BTreeMap<String, String>), Vec<u64>> = BTreeMap::new();
    for (metric, dims, value) in store.counters(week)? {
        let Some(meta) = meta_of(&metric) else {
            continue;
        };
        let mut dims: BTreeMap<String, String> = serde_json::from_str(&dims).unwrap_or_default();
        if meta.kind == "histogram" {
            let bucket = dims.remove("bucket").unwrap_or_default();
            let Some(index) = meta.buckets.iter().position(|b| *b == bucket) else {
                continue;
            };
            let counts = histograms
                .entry((metric, dims))
                .or_insert_with(|| vec![0; meta.buckets.len()]);
            counts[index] = counts[index].saturating_add(value);
        } else {
            let slot = counters.entry((metric, dims)).or_default();
            *slot = slot.saturating_add(value);
        }
    }
    let mut entries = Vec::new();
    for ((metric, dims), mut value) in counters {
        let max = meta_of(&metric).map_or(COUNTER_MAX, |m| m.max);
        if ROUNDED.contains(&metric.as_str()) {
            value = crate::buckets::two_significant(value);
        }
        if value > max {
            value = max;
            adjustments.truncated += 1;
        }
        if value == 0 {
            continue;
        }
        entries.push(Entry {
            m: metric,
            d: dims,
            v: Some(value),
            h: None,
            g: None,
        });
    }
    for ((metric, dims), mut counts) in histograms {
        let max = meta_of(&metric).map_or(HISTOGRAM_TOTAL_MAX, |m| m.max);
        let total: u64 = counts.iter().sum();
        if total > max {
            // Scale down, keeping the shape.
            for c in &mut counts {
                *c = u64::try_from(u128::from(*c) * u128::from(max) / u128::from(total))
                    .unwrap_or(max);
            }
            adjustments.truncated += 1;
        }
        if counts.iter().all(|c| *c == 0) {
            continue;
        }
        entries.push(Entry {
            m: metric,
            d: dims,
            v: None,
            h: Some(counts),
            g: None,
        });
    }
    for (metric, dims, value) in store.gauges(week)? {
        entries.push(Entry {
            m: metric,
            d: serde_json::from_str(&dims).unwrap_or_default(),
            v: None,
            h: None,
            g: Some(value),
        });
    }
    Ok((entries, adjustments))
}

/// Builds `week`'s report from the store (it doesn't delete anything).
///
/// # Errors
///
/// Returns [`StoreError`] on SQLite failures.
pub fn assemble(
    store: &Store,
    week: &str,
    envelope: &Envelope,
    consent_version: u32,
) -> Result<(Report, Adjustments), StoreError> {
    let (mut entries, mut adjustments) = entries_of(store, week)?;
    entries.sort_by(|a, b| (&a.m, &a.d).cmp(&(&b.m, &b.d)));

    let (from, to) = week_days(week);
    let days_active = store.active_days(&from, &to)?.min(7);
    if days_active == 0 {
        entries.retain(|e| e.g.is_some());
    }
    let first_week = store.meta("first_week")?.unwrap_or_else(|| week.to_owned());
    let mut report = Report {
        schema: SCHEMA_VERSION,
        report_id: uuid_v4(),
        week: week.to_owned(),
        sample_rate: 1.0,
        app: App {
            version: envelope.app_version.clone(),
            channel: envelope.channel,
        },
        os: Os {
            name: envelope.os,
            major: envelope.os_major,
            arch: envelope.arch,
        },
        install_age: install_age(&first_week, week).to_owned(),
        days_active,
        consent_version,
        entries,
        pow: None,
    };
    adjustments.truncated += truncate(&mut report, MAX_REPORT_BYTES);
    Ok((report, adjustments))
}

/// The first and last UTC dates of `week`.
fn week_days(week: &str) -> (String, String) {
    let start = crate::week::start_ms(week).unwrap_or(0);
    let day = |ms: i64| {
        let (y, m, d) = crate::week::civil(ms.div_euclid(86_400_000));
        format!("{y:04}-{m:02}-{d:02}")
    };
    (day(start), day(start + 6 * 86_400_000))
}

/// Drops entries until the report fits `limit` bytes: `ui.*` first, then
/// `perf.*`, then the rest (largest metric names last). Returns how many
/// were dropped.
pub fn truncate(report: &mut Report, limit: usize) -> u64 {
    let mut dropped = 0;
    for tier in [Some("ui."), Some("perf."), None] {
        while report.to_bytes().len() > limit {
            let position = report
                .entries
                .iter()
                .rposition(|e| tier.is_none_or(|prefix| e.m.starts_with(prefix)));
            let Some(position) = position else { break };
            report.entries.remove(position);
            dropped += 1;
        }
    }
    dropped
}

/// Checks a report against the schema, as strictly as the service does.
///
/// # Errors
///
/// Returns the first offending field.
pub fn validate(report: &Report) -> Result<(), String> {
    if report.schema != SCHEMA_VERSION {
        return Err("schema".into());
    }
    if crate::week::start_ms(&report.week).is_none() {
        return Err("week".into());
    }
    if !(report.sample_rate > 0.0 && report.sample_rate <= 1.0) {
        return Err("sample_rate".into());
    }
    if report.days_active > 7 {
        return Err("days_active".into());
    }
    if !["<1w", "1-4w", "1-3m", "3-12m", "1y+"].contains(&report.install_age.as_str()) {
        return Err("install_age".into());
    }
    let semver = |v: &str| {
        let parts: Vec<&str> = v.split('.').collect();
        parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.len() <= 6 && p.bytes().all(|b| b.is_ascii_digit()))
    };
    let version = report
        .app
        .version
        .split(['-', '+'])
        .next()
        .unwrap_or_default();
    if !semver(version) {
        return Err("app.version".into());
    }
    if let Some(pow) = &report.pow
        && (pow.is_empty()
            || pow.len() > 32
            || !pow.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
    {
        return Err("pow".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in &report.entries {
        let field = |what: &str| format!("entries.{}.{what}", entry.m);
        let meta = meta_of(&entry.m).ok_or_else(|| field("m"))?;
        if !seen.insert((&entry.m, &entry.d)) {
            return Err(field("duplicate"));
        }
        if entry.d.len() != meta.dims.len() {
            return Err(field("d"));
        }
        for (name, allowed) in meta.dims {
            let value = entry.d.get(*name).ok_or_else(|| field("d"))?;
            if !allowed.contains(&value.as_str()) {
                return Err(field("d"));
            }
        }
        match (meta.kind, &entry.v, &entry.h, &entry.g) {
            ("counter", Some(v), None, None) if (1..=meta.max).contains(v) => {}
            ("histogram", None, Some(h), None)
                if h.len() == meta.buckets.len() && h.iter().sum::<u64>() <= meta.max => {}
            ("gauge", None, None, Some(g)) => {
                let ok = if meta.values.is_empty() {
                    crate::CliVersion::parse(g).wire() == *g
                } else {
                    meta.values.contains(&g.as_str())
                };
                if !ok {
                    return Err(field("g"));
                }
            }
            _ => return Err(field("value")),
        }
        if report.days_active == 0 && entry.g.is_none() {
            return Err("days_active".into());
        }
    }
    if report.to_bytes().len() > MAX_REPORT_BYTES {
        return Err("size".into());
    }
    Ok(())
}
